//! 正式显式出站客户端：复用订阅 fetch 与 P0 引导解析，不修改系统代理/TUN。
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

/// Runtime Ready 发布的只读能力。完整实例 ID 是生命周期身份，不解析其内部格式。
#[derive(Clone)]
pub struct RunningProxy {
    pub(crate) instance_id: String,
    address: SocketAddr,
    valid: Arc<tokio::sync::watch::Sender<bool>>,
    pub(crate) controller: Option<Arc<crate::singbox::clash_api::ManagedControllerEndpoint>>,
    pub(crate) nodes: std::collections::BTreeMap<crate::domain::NodeId, String>,
}
impl std::fmt::Debug for RunningProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RunningProxy")
            .field("instance_id", &self.instance_id)
            .field("valid", &self.is_valid())
            .finish_non_exhaustive()
    }
}
/// 由同一个 Runtime owner 发布/撤销；下载不能用历史端口重新构造身份。
#[derive(Clone, Default)]
pub struct ProxySource(Arc<std::sync::RwLock<Option<RunningProxy>>>);
impl ProxySource {
    pub fn current(&self) -> Option<RunningProxy> {
        self.0.read().expect("outbound source").clone()
    }
    pub(crate) fn publish(&self, proxy: RunningProxy) {
        let mut current = self.0.write().expect("outbound source");
        if let Some(old) = current.replace(proxy) {
            old.invalidate();
        }
    }
    pub(crate) fn revoke(&self) {
        if let Some(proxy) = self.0.write().expect("outbound source").take() {
            proxy.invalidate();
        }
    }
}
impl RunningProxy {
    pub(crate) fn ready(instance_id: String, address: SocketAddr) -> Result<Self, OutboundError> {
        if instance_id.is_empty() || !address.ip().is_loopback() || address.port() == 0 {
            return Err(OutboundError::InvalidProxy);
        }
        let (valid, _) = tokio::sync::watch::channel(true);
        Ok(Self {
            instance_id,
            address,
            valid: Arc::new(valid),
            controller: None,
            nodes: Default::default(),
        })
    }
    // 历史 P0 harness 和本地 fixture 专用；正式调用方只能消费 owner 的能力。
    #[cfg(any(test, feature = "p0-06-prototype"))]
    pub fn from_current(instance: Option<(String, SocketAddr)>) -> Result<Self, OutboundError> {
        let (id, address) = instance.ok_or(OutboundError::ProxyUnavailable)?;
        Self::ready(id, address)
    }
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn is_valid(&self) -> bool {
        *self.valid.borrow()
    }
    pub(crate) fn invalidate(&self) {
        self.valid.send_replace(false);
    }
    pub(crate) async fn while_valid<T>(
        &self,
        request: impl std::future::Future<Output = T>,
    ) -> Result<T, OutboundError> {
        let mut valid = self.valid.subscribe();
        if !*valid.borrow() {
            return Err(OutboundError::ProxyUnavailable);
        }
        tokio::select! {
            biased;
            _ = valid.wait_for(|v| !*v) => Err(OutboundError::ProxyUnavailable),
            value = request => if self.is_valid() { Ok(value) } else { Err(OutboundError::ProxyUnavailable) },
        }
    }
}

#[derive(Clone, Debug)]
pub enum RoutePolicy {
    Direct,
    ViaRunningProxy(RunningProxy),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    if let RoutePolicy::ViaRunningProxy(proxy) = policy
        && !proxy.is_valid()
    {
        return Err(OutboundError::ProxyUnavailable);
    }
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
    observations: Arc<Mutex<std::collections::VecDeque<DnsObservation>>>,
    timeout: Duration,
    endpoint: String,
    upstream_ip: IpAddr,
    #[cfg(test)]
    fixture_skip_route_validation: bool,
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
            observations: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            timeout,
            endpoint: "https://cloudflare-dns.com/dns-query".into(),
            upstream_ip: [1, 1, 1, 1].into(),
            #[cfg(test)]
            fixture_skip_route_validation: false,
        })
    }
    pub fn observations(&self) -> Vec<DnsObservation> {
        self.observations
            .lock()
            .expect("DNS observation mutex")
            .iter()
            .cloned()
            .collect()
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
fn real_addresses(value: DohResponse) -> Result<Vec<IpAddr>, FetchError> {
    if value.status != 0 {
        return Err(FetchError::DnsStatus(value.status));
    }
    let mut addresses = Vec::new();
    for answer in value.answers.into_iter().filter(|a| a.kind == 1) {
        let ip: IpAddr = answer
            .data
            .parse()
            .map_err(|_| FetchError::InvalidDnsResponse)?;
        if !ip.is_ipv4() || ip.is_unspecified() || ip.is_loopback() || ip.is_multicast() {
            return Err(FetchError::InvalidDnsResponse);
        }
        if is_fake_ip(ip) {
            return Err(FetchError::FakeIpAddress);
        }
        addresses.push(ip);
    }
    if addresses.is_empty() {
        return Err(FetchError::NoDnsAddress);
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
                upstream_ip: this.upstream_ip,
                upstream_peer: None,
                outcome: "FAIL".into(),
                addresses: Vec::new(),
                tls_verified: false,
            };
            let result = tokio::time::timeout(this.timeout, async {
                ensure_direct_address(this.upstream_ip).await?;
                let mut response = this
                    .client
                    .get(&this.endpoint)
                    .query(&[("name", hostname.as_str()), ("type", "A")])
                    .header("accept", "application/dns-json")
                    .send()
                    .await
                    .map_err(super::fetch::request_error)?;
                observation.upstream_peer = response.remote_addr();
                observation.tls_verified = true;
                response = response
                    .error_for_status()
                    .map_err(super::fetch::request_error)?;
                // DoH边界同样有body budget，不能对不可信JSON无限读。
                let mut bytes = Vec::new();
                while let Some(chunk) = response
                    .chunk()
                    .await
                    .map_err(super::fetch::request_error)?
                {
                    if bytes.len() + chunk.len() > 65536 {
                        return Err(FetchError::BodyTooLarge.into());
                    }
                    bytes.extend_from_slice(&chunk);
                }
                let value: DohResponse =
                    serde_json::from_slice(&bytes).map_err(|_| FetchError::InvalidDnsResponse)?;
                let addresses = real_addresses(value)?;
                #[cfg(test)]
                let validate_routes = !this.fixture_skip_route_validation;
                #[cfg(not(test))]
                let validate_routes = true;
                if validate_routes {
                    for address in &addresses {
                        ensure_direct_address(*address).await?;
                    }
                }
                Ok::<_, Box<dyn std::error::Error + Send + Sync>>(addresses)
            })
            .await;
            let result: Result<Vec<IpAddr>, Box<dyn std::error::Error + Send + Sync>> = match result
            {
                Ok(result) => result,
                Err(_) => Err(FetchError::Timeout.into()),
            };
            if let Ok(addresses) = &result {
                observation.addresses = addresses.clone();
                observation.outcome = "REAL_A_ANSWER".into();
            } else if let Err(error) = &result {
                observation.outcome = format!("FAIL:{error}");
            }
            let mut observations = this.observations.lock().expect("DNS observation mutex");
            if observations.len() == 64 {
                observations.pop_front();
            }
            observations.push_back(observation);
            drop(observations);
            let addresses = result
                .map_err(|error| {
                    if error.downcast_ref::<super::FetchError>().is_some() {
                        error
                    } else {
                        Box::new(BootstrapDnsError) as Box<dyn std::error::Error + Send + Sync>
                    }
                })?
                .into_iter()
                .map(|ip| SocketAddr::new(ip, 0))
                .collect::<Vec<_>>();
            Ok(Box::new(addresses.into_iter()) as Addrs)
        })
    }
}

/// macOS 对实际目标只读查路由。外部 TUN 未证明绕行时 fail closed；不修改网络。
/// 这只判断可用性，不将非 utun 路由宣称为物理出口已验证。
pub(crate) async fn ensure_direct_address(address: IpAddr) -> Result<(), super::FetchError> {
    if is_fake_ip(address) {
        return Err(super::FetchError::FakeIpAddress);
    }
    if address.is_loopback() {
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let mut command = tokio::process::Command::new("/sbin/route");
        command
            .args(["-n", "get", &address.to_string()])
            .kill_on_drop(true);
        let output = tokio::time::timeout(Duration::from_secs(2), command.output())
            .await
            .map_err(|_| super::FetchError::DirectPathUnverified)?
            .map_err(|_| super::FetchError::DirectPathUnverified)?;
        let text = String::from_utf8_lossy(&output.stdout);
        let interface = text
            .lines()
            .find_map(|line| line.trim().strip_prefix("interface:").map(str::trim));
        if !output.status.success() || interface.is_none_or(|value| value.starts_with("utun")) {
            return Err(super::FetchError::DirectPathUnverified);
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(super::FetchError::DirectPathUnverified)
    }
}

/// DNS 错误只保留失败事实，不让上游 URL/查询内容进入普通日志。
#[derive(Debug, thiserror::Error)]
#[error("independent bootstrap DNS failed")]
pub(crate) struct BootstrapDnsError;

/// 一次操作固定网络路径。撤销取消整条请求链；不存在 Direct fallback。
pub struct OutboundClient {
    client: Client,
    policy: RoutePolicy,
    timeout: Duration,
}
impl OutboundClient {
    pub fn new(policy: RoutePolicy, options: &FetchClientOptions) -> Result<Self, OutboundError> {
        let resolver = Arc::new(BootstrapResolver::new(options.timeout)?);
        Self::with_resolver(policy, options, resolver)
    }
    pub(crate) fn with_resolver<R: Resolve + 'static>(
        policy: RoutePolicy,
        options: &FetchClientOptions,
        resolver: Arc<R>,
    ) -> Result<Self, OutboundError> {
        // 正式路径一律校验 TLS；拒绝由旧选项绕过证书验证。
        if !options.verify_tls {
            return Err(OutboundError::Fetch(FetchError::InvalidTlsPolicy));
        }
        let client = client_builder(&policy, resolver, options.timeout)?
            .user_agent(
                options
                    .user_agent
                    .clone()
                    .unwrap_or_else(super::fetch::default_user_agent),
            )
            .build()
            .map_err(|e| OutboundError::Fetch(super::fetch::request_error(e)))?;
        Ok(Self {
            client,
            policy,
            timeout: options.timeout,
        })
    }
    pub async fn fetch(
        &self,
        source: &str,
        conditional: ConditionalHeaders,
    ) -> Result<FetchResult, OutboundError> {
        self.run(super::fetch::fetch_routed_on_client(
            &self.client,
            source,
            conditional,
            self.timeout,
            matches!(self.policy, RoutePolicy::Direct),
            false,
        ))
        .await
    }
    pub async fn resource(&self, url: &str) -> Result<FetchResult, OutboundError> {
        self.run(super::fetch::fetch_routed_on_client(
            &self.client,
            url,
            ConditionalHeaders::default(),
            self.timeout,
            matches!(self.policy, RoutePolicy::Direct),
            true,
        ))
        .await
    }
    async fn run<T>(
        &self,
        request: impl std::future::Future<Output = Result<T, FetchError>>,
    ) -> Result<T, OutboundError> {
        match &self.policy {
            RoutePolicy::Direct => request.await.map_err(OutboundError::Fetch),
            RoutePolicy::ViaRunningProxy(proxy) => proxy
                .while_valid(request)
                .await?
                .map_err(OutboundError::Fetch),
        }
    }
}

#[cfg(test)]
mod tests;
