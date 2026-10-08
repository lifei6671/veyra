//! P0-06显式出站原型。复用订阅fetch，不接管Runtime、不修改系统代理/TUN。
use std::{
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::Duration,
};

use reqwest::{
    Client, ClientBuilder,
    dns::{Addrs, Name, Resolve, Resolving},
    header::HeaderMap,
};
use serde::{Deserialize, Serialize};

use super::{
    ConditionalHeaders, FetchClientOptions, FetchError, FetchResult,
    fetch::{client_builder_with_options, fetch_subscription_on_client},
};

/// 只接受调用方已确认的当前最后有效实例；没有Auto或fallback。
#[derive(Clone, Debug)]
pub struct RunningProxy {
    pub instance_id: String,
    address: SocketAddr,
}
impl RunningProxy {
    pub fn from_current(instance: Option<(String, SocketAddr)>) -> Result<Self, OutboundError> {
        let (instance_id, address) = instance.ok_or(OutboundError::ProxyUnavailable)?;
        if instance_id.is_empty() || !address.ip().is_loopback() || address.port() == 0 {
            return Err(OutboundError::InvalidProxy);
        }
        Ok(Self {
            instance_id,
            address,
        })
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}

#[derive(Clone, Debug)]
pub enum RoutePolicy {
    Direct,
    ViaRunningProxy(RunningProxy),
}

#[derive(Debug, PartialEq, Eq)]
pub enum OutboundError {
    ProxyUnavailable,
    InvalidProxy,
    Fetch(FetchError),
}

#[derive(Debug, Serialize)]
pub struct Hop {
    pub host: String,
    pub port: Option<u16>,
    pub peer: Option<SocketAddr>,
}
#[derive(Debug)]
pub struct OutboundResponse {
    pub result: FetchResult,
    pub hops: Vec<Hop>,
}

/// 一次构建固定route和resolver；redirect复用同一client，失效proxy只返回错误。
pub fn client_builder<R: Resolve + 'static>(
    policy: &RoutePolicy,
    resolver: Arc<R>,
    timeout: Duration,
) -> Result<ClientBuilder, OutboundError> {
    let options = FetchClientOptions {
        timeout,
        proxy_url: match policy {
            RoutePolicy::Direct => None,
            RoutePolicy::ViaRunningProxy(proxy) => Some(format!("http://{}", proxy.address)),
        },
        ..FetchClientOptions::default()
    };
    let builder = client_builder_with_options(&options).map_err(OutboundError::Fetch)?;
    Ok(match policy {
        RoutePolicy::Direct => builder.dns_resolver(resolver),
        RoutePolicy::ViaRunningProxy(_) => builder,
    })
}

pub async fn fetch_on_client(
    client: &Client,
    source: &str,
    credentials: HeaderMap,
    timeout: Duration,
) -> Result<OutboundResponse, OutboundError> {
    let mut hops = Vec::new();
    let result = fetch_subscription_on_client(
        client,
        source,
        ConditionalHeaders::default(),
        timeout,
        credentials,
        |url, peer| {
            hops.push(Hop {
                host: url.host_str().unwrap_or_default().to_owned(),
                port: url.port_or_known_default(),
                peer,
            })
        },
    )
    .await
    .map_err(OutboundError::Fetch)?;
    Ok(OutboundResponse { result, hops })
}

#[derive(Clone, Debug, Serialize)]
pub struct DnsObservation {
    pub hostname: String,
    pub upstream_ip: IpAddr,
    pub upstream_peer: Option<SocketAddr>,
    pub outcome: String,
    pub addresses: Vec<IpAddr>,
    pub tls_verified: bool,
}

/// 独立IPv4 bootstrap：DoH以显式1.1.1.1建立TLS，绝不解析upstream hostname。
/// 固定HTTPS JSON协议复用reqwest，避免另建DNS报文栈/系统resolver/fallback。
#[derive(Clone)]
pub struct BootstrapResolver {
    client: Client,
    observations: Arc<Mutex<Vec<DnsObservation>>>,
    timeout: Duration,
}
impl BootstrapResolver {
    pub fn new(timeout: Duration) -> Result<Self, OutboundError> {
        let options = FetchClientOptions {
            timeout,
            ..FetchClientOptions::default()
        };
        let client = client_builder_with_options(&options)
            .map_err(OutboundError::Fetch)?
            .resolve("cloudflare-dns.com", SocketAddr::from(([1, 1, 1, 1], 0)))
            .build()
            .map_err(|_| OutboundError::Fetch(FetchError::RequestFailed))?;
        Ok(Self {
            client,
            observations: Arc::new(Mutex::new(Vec::new())),
            timeout,
        })
    }
    pub fn observations(&self) -> Vec<DnsObservation> {
        self.observations
            .lock()
            .expect("DNS observation mutex")
            .clone()
    }
}

#[derive(Deserialize)]
struct DohResponse {
    #[serde(rename = "Status")]
    status: u32,
    #[serde(rename = "Answer", default)]
    answers: Vec<DohAnswer>,
}
#[derive(Deserialize)]
struct DohAnswer {
    #[serde(rename = "type")]
    kind: u16,
    data: String,
}

pub fn is_fake_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => (u32::from(ip) & 0xfffe0000) == 0xc6120000, // 198.18.0.0/15
        IpAddr::V6(ip) => (u128::from(ip) >> 110) == (0xfc00_u128 << 112 >> 110), // fc00::/18
    }
}
fn real_addresses(value: DohResponse) -> Result<Vec<IpAddr>, &'static str> {
    if value.status != 0 {
        return Err("DNS upstream error");
    }
    let mut addresses = Vec::new();
    for answer in value.answers.into_iter().filter(|a| a.kind == 1) {
        let ip: IpAddr = answer.data.parse().map_err(|_| "invalid A answer")?;
        if !ip.is_ipv4()
            || is_fake_ip(ip)
            || ip.is_unspecified()
            || ip.is_loopback()
            || ip.is_multicast()
        {
            return Err("non-real bootstrap address");
        }
        addresses.push(ip);
    }
    if addresses.is_empty() {
        return Err("no A answer; IPv4 bootstrap only");
    }
    Ok(addresses)
}

impl Resolve for BootstrapResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let this = self.clone();
        let hostname = name.as_str().to_owned();
        Box::pin(async move {
            let mut observation = DnsObservation {
                hostname: hostname.clone(),
                upstream_ip: [1, 1, 1, 1].into(),
                upstream_peer: None,
                outcome: "FAIL".into(),
                addresses: Vec::new(),
                tls_verified: false,
            };
            let result = tokio::time::timeout(this.timeout, async {
                let mut response = this
                    .client
                    .get("https://cloudflare-dns.com/dns-query")
                    .query(&[("name", hostname.as_str()), ("type", "A")])
                    .header("accept", "application/dns-json")
                    .send()
                    .await?
                    .error_for_status()?;
                observation.upstream_peer = response.remote_addr();
                observation.tls_verified = true;
                // DoH边界同样有body budget，不能对不可信JSON无限读。
                let mut bytes = Vec::new();
                while let Some(chunk) = response.chunk().await? {
                    if bytes.len() + chunk.len() > 65536 {
                        return Err("DNS body too large".into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                let value: DohResponse = serde_json::from_slice(&bytes)?;
                real_addresses(value).map_err(Into::into)
            })
            .await;
            let result: Result<Vec<IpAddr>, Box<dyn std::error::Error + Send + Sync>> = match result
            {
                Ok(result) => result,
                Err(_) => Err("DNS total timeout".into()),
            };
            if let Ok(addresses) = &result {
                observation.addresses = addresses.clone();
                observation.outcome = "REAL_A_ANSWER".into();
            }
            this.observations
                .lock()
                .expect("DNS observation mutex")
                .push(observation);
            let addresses = result?
                .into_iter()
                .map(|ip| SocketAddr::new(ip, 0))
                .collect::<Vec<_>>();
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

#[cfg(test)]
mod tests;
