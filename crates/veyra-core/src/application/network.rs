//! 下载之外的正式测速/IP 客户端。调用方必须显式选择路径；没有自动探测或 fallback。
use crate::{
    domain::NodeId,
    singbox::clash_api::{ClashApiClient, ClashApiError},
    subscription::{
        FetchClientOptions, FetchResult,
        outbound::{OutboundClient, OutboundError, ProxySource, RoutePolicy, RunningProxy},
    },
};
use serde::Serialize;
use std::{
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RequestedPath {
    Direct,
    ViaRunningProxy,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum UsedPath {
    Direct,
    ViaRunningProxy {
        instance_id: String,
    },
    Node {
        instance_id: String,
        node_id: NodeId,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NetworkError {
    Closed,
    InvalidRequest,
    ProxyUnavailable,
    NodeNotApplied,
    UnsupportedNodeTestUrl,
    Request(OutboundError),
    Controller(ClashApiError),
    InvalidResponse,
    ProviderRejected,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Latency {
    pub milliseconds: Option<u64>,
    pub path: UsedPath,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum GeoIpProvider {
    IpSb,
    IpWhoIs,
    IpApiIs,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GeoIpInfo {
    pub provider: GeoIpProvider,
    pub ip: IpAddr,
    pub asn: Option<u64>,
    pub country_code: Option<String>,
    pub country: Option<String>,
    pub organization: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct IpResult {
    pub info: GeoIpInfo,
    pub path: UsedPath,
}

pub struct NetworkService {
    source: ProxySource,
    closing: Arc<tokio::sync::watch::Sender<bool>>,
}
impl NetworkService {
    pub fn new(source: ProxySource) -> Self {
        let (closing, _) = tokio::sync::watch::channel(false);
        Self {
            source,
            closing: Arc::new(closing),
        }
    }
    pub fn shutdown(&self) {
        self.closing.send_replace(true);
    }
    fn proxy(&self) -> Result<RunningProxy, NetworkError> {
        self.source
            .current()
            .filter(|p| p.is_valid())
            .ok_or(NetworkError::ProxyUnavailable)
    }
    fn route(&self, path: RequestedPath) -> Result<(RoutePolicy, UsedPath), NetworkError> {
        match path {
            RequestedPath::Direct => Ok((RoutePolicy::Direct, UsedPath::Direct)),
            RequestedPath::ViaRunningProxy => {
                let p = self.proxy()?;
                let used = UsedPath::ViaRunningProxy {
                    instance_id: p.instance_id.clone(),
                };
                Ok((RoutePolicy::ViaRunningProxy(p), used))
            }
        }
    }
    async fn open<T>(
        &self,
        future: impl std::future::Future<Output = Result<T, NetworkError>>,
    ) -> Result<T, NetworkError> {
        let mut closed = self.closing.subscribe();
        if *closed.borrow() {
            return Err(NetworkError::Closed);
        }
        tokio::select! { biased; _ = closed.wait_for(|v| *v) => Err(NetworkError::Closed), result = future => result }
    }
    pub async fn site_latency(
        &self,
        target: &str,
        timeout: Duration,
        path: RequestedPath,
    ) -> Result<Latency, NetworkError> {
        validate_test(target, timeout)?;
        let (route, path) = self.route(path)?;
        self.open(async {
            let client =
                OutboundClient::new(route, &options(timeout)).map_err(NetworkError::Request)?;
            let start = Instant::now();
            client
                .resource(target)
                .await
                .map_err(NetworkError::Request)?;
            Ok(Latency {
                milliseconds: Some(start.elapsed().as_millis() as u64),
                path,
            })
        })
        .await
    }
    pub async fn node_latency(
        &self,
        node: &NodeId,
        target: &str,
        timeout: Duration,
    ) -> Result<Latency, NetworkError> {
        validate_test(target, timeout)?;
        // 固定 1.14.0 controller 会丢弃 http URL 并改测默认站点；
        // 明确拒绝，不能把另一目标的结果归给用户指定目标。
        if !target.starts_with("https://") {
            return Err(NetworkError::UnsupportedNodeTestUrl);
        }
        let proxy = self.proxy()?;
        let tag = proxy.nodes.get(node).ok_or(NetworkError::NodeNotApplied)?;
        let endpoint = proxy
            .controller
            .as_ref()
            .ok_or(NetworkError::ProxyUnavailable)?;
        self.open(async {
            let result = proxy
                .while_valid(
                    ClashApiClient::managed(endpoint)
                        .map_err(NetworkError::Controller)?
                        .node_delay(tag, target, timeout),
                )
                .await;
            let ms = result
                .map_err(NetworkError::Request)?
                .map_err(NetworkError::Controller)?;
            if !proxy.is_valid() {
                return Err(NetworkError::ProxyUnavailable);
            }
            Ok(Latency {
                milliseconds: ms,
                path: UsedPath::Node {
                    instance_id: proxy.instance_id.clone(),
                    node_id: node.clone(),
                },
            })
        })
        .await
    }
    pub async fn geo_ip(
        &self,
        provider: GeoIpProvider,
        ip: IpAddr,
        path: RequestedPath,
    ) -> Result<IpResult, NetworkError> {
        self.ip_request(provider, Some(ip), path).await
    }
    pub async fn exit_ip(
        &self,
        provider: GeoIpProvider,
        path: RequestedPath,
    ) -> Result<IpResult, NetworkError> {
        self.ip_request(provider, None, path).await
    }
    async fn ip_request(
        &self,
        provider: GeoIpProvider,
        ip: Option<IpAddr>,
        path: RequestedPath,
    ) -> Result<IpResult, NetworkError> {
        let (route, path) = self.route(path)?;
        self.open(async {
            let client = OutboundClient::new(route, &options(Duration::from_secs(10)))
                .map_err(NetworkError::Request)?;
            let response = client
                .resource(&provider.url(ip))
                .await
                .map_err(NetworkError::Request)?;
            let FetchResult::Modified { body, .. } = response else {
                return Err(NetworkError::InvalidResponse);
            };
            Ok(IpResult {
                info: normalize_geo_ip(provider, &body, ip)?,
                path,
            })
        })
        .await
    }
}
impl Drop for NetworkService {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn options(timeout: Duration) -> FetchClientOptions {
    FetchClientOptions {
        timeout,
        user_agent: Some("Veyra/0.1".into()),
        ..Default::default()
    }
}
fn validate_test(url: &str, timeout: Duration) -> Result<(), NetworkError> {
    if timeout.is_zero()
        || timeout > Duration::from_secs(30)
        || reqwest::Url::parse(url).ok().is_none_or(|url| {
            crate::subscription::fetch::validate_resource_url(&url, None).is_err()
        })
    {
        return Err(NetworkError::InvalidRequest);
    }
    Ok(())
}
impl GeoIpProvider {
    fn url(self, ip: Option<IpAddr>) -> String {
        let ip = ip.map(|v| v.to_string()).unwrap_or_default();
        match self {
            Self::IpSb => format!("https://api.ip.sb/geoip/{ip}"),
            Self::IpWhoIs => format!("https://ipwho.is/{ip}"),
            Self::IpApiIs => {
                if ip.is_empty() {
                    "https://api.ipapi.is/".into()
                } else {
                    format!("https://api.ipapi.is/?q={ip}")
                }
            }
        }
    }
}
/// 按 React 已有字段归一化，未知保留 None；拒绝 provider 的业务错误及错目标结果。
pub fn normalize_geo_ip(
    provider: GeoIpProvider,
    body: &str,
    requested: Option<IpAddr>,
) -> Result<GeoIpInfo, NetworkError> {
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| NetworkError::InvalidResponse)?;
    if !value.is_object() {
        return Err(NetworkError::InvalidResponse);
    }
    if value.get("success").and_then(|v| v.as_bool()) == Some(false)
        || value.get("error").is_some_and(|v| !v.is_null())
    {
        return Err(NetworkError::ProviderRejected);
    }
    let ip = match value.get("ip") {
        Some(v) => v
            .as_str()
            .and_then(|v| v.parse::<IpAddr>().ok())
            .ok_or(NetworkError::InvalidResponse)?,
        None => requested.ok_or(NetworkError::InvalidResponse)?,
    };
    if requested.is_some_and(|target| target != ip) {
        return Err(NetworkError::InvalidResponse);
    }
    let string = |paths: &[&str]| {
        paths.iter().find_map(|path| {
            value
                .pointer(path)
                .and_then(|v| v.as_str())
                .filter(|v| !v.is_empty())
                .map(str::to_owned)
        })
    };
    let mut asn = value
        .pointer(match provider {
            GeoIpProvider::IpSb => "/asn",
            GeoIpProvider::IpWhoIs => "/connection/asn",
            GeoIpProvider::IpApiIs => "/asn/asn",
        })
        .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))
        .filter(|v| *v > 0 && *v <= crate::domain::MAX_SAFE_INTEGER);
    let mut organization = string(match provider {
        GeoIpProvider::IpSb => &["/organization", "/asn_organization", "/isp"],
        GeoIpProvider::IpWhoIs => &["/connection/org", "/connection/isp"],
        GeoIpProvider::IpApiIs => &["/asn/org", "/asn/organization"],
    });
    // 沿用 ChainProxySettings.helpers 的 ipapi.is 无 location 响应语义；
    // 当前真实 free-tier 返回 "AS15169 Google LLC" 和 company 字符串。
    if provider == GeoIpProvider::IpApiIs && !value.get("location").is_some_and(|v| v.is_object()) {
        if let Some(value) = value
            .get("asn")
            .and_then(|v| v.as_str())
            .and_then(|v| v.strip_prefix("AS"))
        {
            let (number, name) = value.split_once(char::is_whitespace).unwrap_or((value, ""));
            asn = number
                .parse::<u64>()
                .ok()
                .filter(|v| *v > 0 && *v <= crate::domain::MAX_SAFE_INTEGER);
            if asn.is_some() && !name.trim().is_empty() {
                organization = Some(name.trim().to_owned());
            }
        }
        organization = organization.or_else(|| string(&["/company"]));
    }
    Ok(GeoIpInfo {
        provider,
        ip,
        asn,
        country_code: string(&["/country_code", "/location/country_code"])
            .map(|v| v.to_uppercase()),
        country: string(&["/country", "/location/country"]),
        organization,
        region: string(&["/location/state", "/region"]),
        city: string(&["/location/city", "/city"]),
    })
}
#[cfg(test)]
mod tests;
