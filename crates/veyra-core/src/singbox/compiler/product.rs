//! P2 产品编译入口。运行期资源由 owner 显式提供，不进入业务快照。
use super::*;
use crate::domain::{
    AppState, DomainMatcher, Ipv6Proxy, ProfileTun, RuntimeHealthUrl, SubscriptionSource,
};
use std::{
    net::SocketAddr,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnsupportedProductOption {
    Ipv6Proxy,
    Tun,
    DnsExtras,
    DnsUpstream,
    SplitDnsRoute,
}

/// 私有字段保证任何正式 listener 都只能是 loopback；0 表示待内核分配。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoopbackListener(SocketAddr);
impl LoopbackListener {
    pub fn new(address: SocketAddr) -> Result<Self, CompileError> {
        if !address.ip().is_loopback() {
            return Err(CompileError::InvalidRuntimeResources);
        }
        Ok(Self(address))
    }
    pub fn address(self) -> SocketAddr {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManagedCacheFile {
    path: PathBuf,
    cache_id: String,
    store_fakeip: bool,
    store_dns: bool,
}
impl ManagedCacheFile {
    /// owner 给出按用户/profile/内核代际隔离的 root 和稳定 ID；不能以 instance/revision 派生。
    /// 当前产品没有 FakeIP，也不恢复 DNS 缓存，两标志显式 false。
    pub fn new(
        root: &Path,
        path: PathBuf,
        cache_id: String,
        store_fakeip: bool,
        store_dns: bool,
    ) -> Result<Self, CompileError> {
        if !managed_absolute(root)
            || !managed_absolute(&path)
            || !path.starts_with(root)
            || path == root
            || !valid_cache_id(&cache_id)
            || store_fakeip
            || store_dns
        {
            return Err(CompileError::InvalidRuntimeResources);
        }
        Ok(Self {
            path,
            cache_id,
            store_fakeip,
            store_dns,
        })
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn cache_id(&self) -> &str {
        &self.cache_id
    }
}
fn managed_absolute(path: &Path) -> bool {
    path.is_absolute()
        && path.to_str().is_some()
        && path.components().all(|c| {
            matches!(
                c,
                Component::RootDir | Component::Prefix(_) | Component::Normal(_)
            )
        })
}
fn valid_cache_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductRuntimeResources {
    pub mixed: LoopbackListener,
    pub controller: LoopbackListener,
    pub cache: ManagedCacheFile,
}
/// state 提供持久设置/订阅元数据；运行出口事实必须来自调用方的实际投影，不能重建或猜测。
/// RuntimeIntent 可包含 runtime-only pool，默认出口使用领域 ID，tag 仅在本 adapter 内转换。
pub struct ProductCompileRequest<'a> {
    pub state: &'a AppState,
    pub runtime_intent: &'a RuntimeIntent,
    pub default_outbound: &'a OutboundId,
    pub resources: &'a ProductRuntimeResources,
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RuntimeHealthPlan {
    pub proxy_url: RuntimeHealthUrl,
    pub direct_url: RuntimeHealthUrl,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CacheFile {
    enabled: bool,
    pub(super) path: PathBuf,
    pub(super) cache_id: String,
    store_fakeip: bool,
    store_dns: bool,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(super) enum CoreInbound {
    Mixed(MixedInbound),
    #[cfg(any(test, feature = "legacy-test-support"))]
    Test(TestInbound),
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MixedInbound {
    #[serde(rename = "type")]
    kind: String,
    tag: String,
    listen: IpAddr,
    pub(super) listen_port: u16,
}

impl SingBoxCompiler {
    pub fn compile_product(
        &self,
        request: ProductCompileRequest<'_>,
    ) -> Result<SingBoxPlan, CompileError> {
        let state = request.state;
        state.validate().map_err(|_| CompileError::InvalidProfile)?;
        let profile = &state.profile;
        if profile.ipv6_proxy != Ipv6Proxy::Node {
            return Err(CompileError::UnsupportedOption(
                UnsupportedProductOption::Ipv6Proxy,
            ));
        }
        if profile.tun != ProfileTun::default() {
            return Err(CompileError::UnsupportedOption(
                UnsupportedProductOption::Tun,
            ));
        }
        if !profile.dns.direct_extras.is_empty() || !profile.dns.proxy_extras.is_empty() {
            return Err(CompileError::UnsupportedOption(
                UnsupportedProductOption::DnsExtras,
            ));
        }
        let resources = request.resources;
        if resources.mixed.0.port() != 0 && resources.mixed == resources.controller {
            return Err(CompileError::InvalidRuntimeResources);
        }
        let intent = request.runtime_intent;
        let catalog = OutboundCatalog::from_runtime_intent(intent);
        catalog
            .validate_graph()
            .map_err(|_| CompileError::InvalidRouteTarget)?;
        catalog
            .require_available(request.default_outbound)
            .map_err(catalog_error)?;
        // 保留既有协议、成员、tag 与封闭校验；只扩展产品资源及 P2 行为。
        let mut plan = self.compile(
            intent,
            &RouteTarget::Direct,
            DnsPolicy::System,
            RuntimeProfile::ObservationOnly,
        )?;
        let document = &mut plan.document;
        // 动态地址只从当前 child 的受管 stderr 发现，Platform 丢弃非监听行。
        document.log = LogConfig {
            disabled: false,
            level: Some("info".into()),
            output: None,
        };
        document.experimental.clash_api.external_controller = resources.controller.0.to_string();
        document.experimental.cache_file = Some(CacheFile {
            enabled: true,
            path: resources.cache.path.clone(),
            cache_id: resources.cache.cache_id.clone(),
            store_fakeip: resources.cache.store_fakeip,
            store_dns: resources.cache.store_dns,
        });
        document.inbounds = vec![CoreInbound::Mixed(MixedInbound {
            kind: "mixed".into(),
            tag: "mixed".into(),
            listen: resources.mixed.0.ip(),
            listen_port: resources.mixed.0.port(),
        })];
        // v1.14 已移除 Block outbound；domain Block 仅在适配边界转换为 reject action。
        document
            .outbounds
            .retain(|o| !matches!(o, CoreOutbound::Block(_)));
        let default_tag = outbound_tag(request.default_outbound);
        document.route.final_outbound = if *request.default_outbound == OutboundId::Block {
            "direct".into()
        } else {
            default_tag.clone()
        };
        document.route.default_domain_resolver = "dns-direct".into();
        let mut rules = document
            .route
            .rules
            .iter()
            .filter(|r| r.inbound.is_some())
            .cloned()
            .collect::<Vec<_>>();
        if !profile.ipv6 {
            rules.push(CoreRule {
                ip_version: Some(6),
                action: Some("reject".into()),
                ..CoreRule::default()
            });
        }
        let mut bootstrap = CoreRule {
            outbound: Some("direct".into()),
            ..CoreRule::default()
        };
        let mut domains = BTreeSet::new();
        let mut ips = BTreeSet::new();
        if profile.direct_for_nodes {
            let provider_ids = intent
                .nodes
                .iter()
                .map(|node| &node.provider_id)
                .collect::<BTreeSet<_>>();
            for node in &intent.nodes {
                add_host(&node.server, &mut domains, &mut ips)?;
            }
            // 只沿实际运行节点的 provider 取订阅元数据，保存但未运行的订阅不进入引导规则。
            let mut subscription_ids = BTreeSet::new();
            for id in provider_ids {
                let provider = state
                    .providers
                    .iter()
                    .find(|provider| &provider.id == id)
                    .ok_or(CompileError::InvalidProfile)?;
                subscription_ids.insert(&provider.subscription_id);
            }
            for sub in state
                .subscriptions
                .iter()
                .filter(|sub| subscription_ids.contains(&sub.id))
            {
                if let SubscriptionSource::Remote { url } = &sub.source {
                    let url = reqwest::Url::parse(url).map_err(|_| CompileError::InvalidProfile)?;
                    add_host(
                        url.host_str().ok_or(CompileError::InvalidProfile)?,
                        &mut domains,
                        &mut ips,
                    )?;
                }
            }
            if !domains.is_empty() {
                bootstrap.domain = Some(domains.into_iter().collect());
                rules.push(bootstrap.clone());
            }
            if !ips.is_empty() {
                rules.push(CoreRule {
                    ip_cidr: Some(ips.into_iter().collect()),
                    outbound: Some("direct".into()),
                    ..CoreRule::default()
                });
            }
        }
        let mut sites = Vec::new();
        if let Some(custom) = &profile.routing.custom
            && custom.enabled
        {
            for rule in &custom.rules {
                let mut compiled = CoreRule {
                    outbound: Some(outbound_tag(
                        &catalog.resolve(&rule.target).map_err(catalog_error)?,
                    )),
                    ..CoreRule::default()
                };
                match rule.matcher {
                    DomainMatcher::Domain => compiled.domain = Some(vec![rule.value.clone()]),
                    DomainMatcher::DomainSuffix => {
                        compiled.domain_suffix = Some(vec![rule.value.clone()])
                    }
                }
                sites.push(compiled);
            }
        }
        sites.extend(
            document
                .route
                .rules
                .iter()
                .filter(|r| r.inbound.is_none())
                .cloned(),
        );
        // P2 只做域名 DNS 分流；IP/端口/process 的 DNS 对应语义留 P5，不能猜测。
        if profile.dns.split
            && sites
                .iter()
                .any(|r| r.domain.is_none() && r.domain_suffix.is_none())
        {
            return Err(CompileError::UnsupportedOption(
                UnsupportedProductOption::SplitDnsRoute,
            ));
        }
        for mut rule in sites {
            if profile.reject_quic
                && is_proxy(rule.outbound.as_deref())
                && let Some(reject) = proxy_udp443(&rule)
            {
                rules.push(reject);
            }
            if rule.outbound.as_deref() == Some("block") {
                rule.outbound = None;
                rule.action = Some("reject".into());
            }
            rules.push(rule);
        }
        if profile.reject_quic && is_proxy(Some(&default_tag)) {
            rules.push(proxy_udp443(&CoreRule::default()).expect("无约束的 proxy 默认路径"));
        }
        if *request.default_outbound == OutboundId::Block {
            rules.push(CoreRule {
                action: Some("reject".into()),
                ..CoreRule::default()
            });
        }
        // DNS 与站点路由使用相同顺序/域名匹配；bootstrap 永远在站点前。
        let mut dns_rules = document.dns.rules.clone();
        let mut proxy_dns_targets = BTreeSet::new();
        for rule in &rules {
            if !profile.dns.split
                || rule.inbound.is_some()
                || rule.ip_version.is_some()
                || rule.network.is_some()
                || (rule.domain.is_none() && rule.domain_suffix.is_none())
            {
                continue;
            }
            let server = if rule.action.as_deref() == Some("reject") {
                None
            } else if let Some(target) = rule.outbound.as_deref().filter(|t| is_proxy(Some(t))) {
                proxy_dns_targets.insert(target.to_owned());
                Some(proxy_dns_tag(target, &default_tag))
            } else {
                Some("dns-direct".into())
            };
            dns_rules.push(CoreRule {
                domain: rule.domain.clone(),
                domain_suffix: rule.domain_suffix.clone(),
                action: Some(if server.is_none() { "reject" } else { "route" }.into()),
                server,
                ..CoreRule::default()
            });
        }
        let mut servers = vec![udp_dns(
            "dns-direct",
            &profile.dns.direct,
            profile.dns.direct_port,
            "direct",
        )?];
        if profile.dns.split {
            if *request.default_outbound == OutboundId::Block {
                return Err(CompileError::UnsupportedOption(
                    UnsupportedProductOption::SplitDnsRoute,
                ));
            }
            if is_proxy(Some(&default_tag)) {
                proxy_dns_targets.insert(default_tag.clone());
            }
            // 同一 Profile proxy upstream 按各真实出口派生 detour；不把另一 pool 的 DNS 发到默认 pool。
            for target in proxy_dns_targets {
                servers.push(udp_dns(
                    &proxy_dns_tag(&target, &default_tag),
                    &profile.dns.proxy,
                    profile.dns.proxy_port,
                    &target,
                )?);
            }
        }
        document.dns = DnsConfig {
            servers,
            final_server: if profile.dns.split && is_proxy(Some(&default_tag)) {
                "dns-proxy"
            } else {
                "dns-direct"
            }
            .into(),
            strategy: Some(
                if profile.ipv6 {
                    "prefer_ipv4"
                } else {
                    "ipv4_only"
                }
                .into(),
            ),
            rules: dns_rules,
        };
        document.route.rules = rules;
        plan.health = Some(RuntimeHealthPlan {
            proxy_url: profile.test_url.clone(),
            direct_url: profile.direct_test_url.clone(),
        });
        document.validate(false)?;
        plan.index = Some(AppliedArtifactIndex::compile(state, intent));
        Ok(plan)
    }
}
fn proxy_dns_tag(target: &str, default: &str) -> String {
    if target == default {
        "dns-proxy".into()
    } else {
        format!("dns-proxy-{target}")
    }
}
fn udp_dns(tag: &str, server: &str, port: u16, detour: &str) -> Result<DnsServer, CompileError> {
    server
        .parse::<IpAddr>()
        .map_err(|_| CompileError::UnsupportedOption(UnsupportedProductOption::DnsUpstream))?;
    Ok(DnsServer {
        kind: "udp".into(),
        tag: tag.into(),
        server: Some(server.into()),
        server_port: Some(port),
        // v1.14 默认拨号即直连；空 direct outbound 不允许作为 DNS detour。
        detour: (detour != "direct").then(|| detour.into()),
    })
}
fn add_host(
    host: &str,
    domains: &mut BTreeSet<String>,
    ips: &mut BTreeSet<String>,
) -> Result<(), CompileError> {
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        ips.insert(format!("{ip}/{}", if ip.is_ipv4() { 32 } else { 128 }));
    } else {
        // host 来自节点或 URL，仅提取 hostname，不复制订阅 query/userinfo/节点凭据。
        if host.is_empty() || host.contains(['/', ':', '@', '?', '#', ' ']) {
            return Err(CompileError::InvalidNodeConfiguration);
        }
        domains.insert(host.to_ascii_lowercase());
    }
    Ok(())
}
fn is_proxy(tag: Option<&str>) -> bool {
    tag.is_some_and(|t| t.starts_with("pool-") || t.starts_with("node-"))
}
/// 保留原规则条件，取 UDP/443 的交集；顺序保证更早的 Direct 不受影响。
fn proxy_udp443(rule: &CoreRule) -> Option<CoreRule> {
    if rule.port.as_ref().is_some_and(|p| !p.contains(&443))
        || rule
            .network
            .as_ref()
            .is_some_and(|n| !n.contains(&NetworkProtocol::Udp))
    {
        return None;
    }
    let mut reject = rule.clone();
    reject.outbound = None;
    reject.action = Some("reject".into());
    reject.port = Some(vec![443]);
    reject.network = Some(vec![NetworkProtocol::Udp]);
    Some(reject)
}

impl Document {
    pub(super) fn validate_product(&self, bound: bool) -> Result<(), CompileError> {
        let rejected = CompileError::InvalidFinalConfiguration;
        let cache = self.experimental.cache_file.as_ref().ok_or(rejected)?;
        if !cache.enabled
            || !managed_absolute(&cache.path)
            || !valid_cache_id(&cache.cache_id)
            || cache.store_fakeip
            || cache.store_dns
        {
            return Err(rejected);
        }
        let controller: SocketAddr = self
            .experimental
            .clash_api
            .external_controller
            .parse()
            .map_err(|_| rejected)?;
        let [CoreInbound::Mixed(mixed)] = self.inbounds.as_slice() else {
            return Err(rejected);
        };
        if mixed.kind != "mixed"
            || mixed.tag != "mixed"
            || !mixed.listen.is_loopback()
            || !controller.ip().is_loopback()
            || (mixed.listen_port != 0
                && controller == SocketAddr::new(mixed.listen, mixed.listen_port))
        {
            return Err(rejected);
        }
        let tags = self
            .outbounds
            .iter()
            .map(|o| o.tag().to_owned())
            .chain(self.endpoints.iter().map(|e| e.tag.clone()))
            .collect::<BTreeSet<_>>();
        if self
            .outbounds
            .iter()
            .any(|o| matches!(o, CoreOutbound::Block(_)))
            || !tags.contains(&self.route.final_outbound)
            || self.route.default_domain_resolver != "dns-direct"
            || !matches!(
                self.dns.strategy.as_deref(),
                Some("ipv4_only" | "prefer_ipv4")
            )
        {
            return Err(rejected);
        }
        let dns_tags = self
            .dns
            .servers
            .iter()
            .map(|s| s.tag.clone())
            .collect::<BTreeSet<_>>();
        if self.dns.servers.is_empty()
            || self.dns.servers.len() > tags.len() + 1
            || dns_tags.len() != self.dns.servers.len()
            || !dns_tags.contains("dns-direct")
            || !dns_tags.contains(&self.dns.final_server)
        {
            return Err(rejected);
        }
        for server in &self.dns.servers {
            if server.kind != "udp"
                || !(if server.tag == "dns-direct" {
                    server.detour.is_none()
                } else {
                    server.detour.as_ref().is_some_and(|target| {
                        is_proxy(Some(target))
                            && tags.contains(target)
                            && server.tag == proxy_dns_tag(target, &self.route.final_outbound)
                    })
                })
                || !server
                    .server
                    .as_ref()
                    .is_some_and(|s| s.parse::<IpAddr>().is_ok())
                || !server.server_port.is_some_and(|p| p > 0)
            {
                return Err(rejected);
            }
        }
        let ingress = self
            .endpoints
            .iter()
            .map(|e| e.tag.clone())
            .collect::<Vec<_>>();
        if !ingress.is_empty()
            && (!self
                .route
                .rules
                .first()
                .is_some_and(|r| r.is_ingress_reject(&ingress))
                || !self
                    .dns
                    .rules
                    .first()
                    .is_some_and(|r| r.is_ingress_reject(&ingress)))
        {
            return Err(rejected);
        }
        for rule in &self.route.rules {
            validate_rule_values(rule)?;
            if rule.server.is_some()
                || (rule.inbound.is_some() && !rule.is_ingress_reject(&ingress))
                || !match rule.action.as_deref() {
                    None | Some("route") => {
                        rule.outbound.as_ref().is_some_and(|t| tags.contains(t))
                    }
                    Some("reject") => rule.outbound.is_none(),
                    _ => false,
                }
            {
                return Err(rejected);
            }
        }
        for rule in &self.dns.rules {
            validate_rule_values(rule)?;
            if rule.outbound.is_some()
                || rule.process_name.is_some()
                || rule.ip_cidr.is_some()
                || rule.port.is_some()
                || rule.network.is_some()
                || rule.ip_version.is_some()
                || (rule.inbound.is_some() && !rule.is_ingress_reject(&ingress))
                || !match rule.action.as_deref() {
                    Some("route") => {
                        rule.server.as_ref().is_some_and(|t| dns_tags.contains(t))
                            && rule.matcher_count() > 0
                    }
                    Some("reject") => rule.server.is_none(),
                    _ => false,
                }
            {
                return Err(rejected);
            }
        }
        // 同一封闭 Document 复用原有节点/endpoint/selector/urltest 验证，不建立第二套出口模型。
        if self.log.disabled
            || self.log.level.as_deref() != Some("info")
            || self.log.output.is_some()
        {
            return Err(CompileError::InvalidFinalConfiguration);
        }
        let mut ordinary = self.clone();
        ordinary.log = LogConfig {
            disabled: true,
            level: None,
            output: None,
        };
        ordinary.inbounds.clear();
        ordinary.experimental.cache_file = None;
        ordinary.experimental.clash_api.external_controller = API_ADDRESS.into();
        ordinary.outbounds.push(CoreOutbound::Block(Terminal {
            tag: "block".into(),
        }));
        ordinary.route.final_outbound = "direct".into();
        ordinary.route.default_domain_resolver = DNS_TAG.into();
        ordinary.route.rules = if ingress.is_empty() {
            vec![]
        } else {
            vec![CoreRule::reject(ingress.clone())]
        };
        ordinary.dns = DnsConfig {
            strategy: None,
            servers: vec![DnsServer {
                kind: "local".into(),
                tag: DNS_TAG.into(),
                server: None,
                server_port: None,
                detour: None,
            }],
            final_server: DNS_TAG.into(),
            rules: if ingress.is_empty() {
                vec![]
            } else {
                vec![CoreRule::reject(ingress)]
            },
        };
        ordinary.validate_standard(bound)
    }
}
fn validate_rule_values(rule: &CoreRule) -> Result<(), CompileError> {
    let bad = CompileError::InvalidFinalConfiguration;
    if rule.ip_version.is_some_and(|v| ![4, 6].contains(&v))
        || rule
            .port
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.contains(&0))
        || rule.network.as_ref().is_some_and(Vec::is_empty)
        || rule
            .ip_cidr
            .as_ref()
            .is_some_and(|v| v.is_empty() || v.iter().any(|s| !valid_prefix(s)))
    {
        return Err(bad);
    }
    for list in [&rule.domain, &rule.domain_suffix, &rule.process_name]
        .into_iter()
        .flatten()
    {
        if list.is_empty() || list.iter().any(|s| !present(s)) {
            return Err(bad);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "product_tests.rs"]
mod tests;
