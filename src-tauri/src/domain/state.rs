use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 6;
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
pub const MAX_SUBSCRIPTION_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;

fn is_zero_u32(value: &u32) -> bool {
    *value == 0
}

fn is_false(value: &bool) -> bool {
    !*value
}

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(pub String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, StateValidationError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(StateValidationError::InvalidIdentifier);
                }
                Ok(Self(value))
            }

            fn is_valid(&self) -> bool {
                !self.0.trim().is_empty()
            }
        }
    };
}

stable_id!(SubscriptionId);
stable_id!(ProviderId);
stable_id!(NodeId);
stable_id!(PoolId);
stable_id!(RoutePolicyId);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AppState {
    pub schema_version: u32,
    pub default_target: RouteTarget,
    pub active_subscription_id: Option<SubscriptionId>,
    pub active_configuration_generation: u64,
    pub subscriptions: Vec<Subscription>,
    pub providers: Vec<Provider>,
    pub nodes: Vec<ProxyNode>,
    pub pools: Vec<NodePool>,
    pub routes: Vec<RoutePolicy>,
}

impl AppState {
    pub fn empty() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            default_target: RouteTarget::Unconfigured,
            active_subscription_id: None,
            active_configuration_generation: 0,
            subscriptions: Vec::new(),
            providers: Vec::new(),
            nodes: Vec::new(),
            pools: Vec::new(),
            routes: Vec::new(),
        }
    }

    pub fn validate(&self) -> Result<(), StateValidationError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(StateValidationError::UnsupportedSchemaVersion);
        }

        let subscriptions = unique_ids(self.subscriptions.iter().map(|value| &value.id))?;
        let providers = unique_ids(self.providers.iter().map(|value| &value.id))?;
        unique_ids(self.nodes.iter().map(|value| &value.id))?;
        let pools = unique_ids(self.pools.iter().map(|value| &value.id))?;
        unique_ids(self.routes.iter().map(|value| &value.id))?;

        self.default_target.validate(&pools)?;
        if self.active_configuration_generation > MAX_SAFE_INTEGER
            || self
                .active_subscription_id
                .as_ref()
                .is_some_and(|id| !subscriptions.contains(id))
        {
            return Err(StateValidationError::InvalidSubscription);
        }

        for subscription in &self.subscriptions {
            subscription.validate()?;
        }

        for provider in &self.providers {
            if !provider.subscription_id.is_valid()
                || !subscriptions.contains(&provider.subscription_id)
            {
                return Err(StateValidationError::MissingSubscription);
            }
        }

        for node in &self.nodes {
            if !node.provider_id.is_valid() || !providers.contains(&node.provider_id) {
                return Err(StateValidationError::MissingProvider);
            }
            if !node.options.is_compatible_with(node.protocol)
                || node.tls.as_ref().is_some_and(|tls| !tls.is_valid())
                || node
                    .transport
                    .as_ref()
                    .is_some_and(|transport| !transport.has_valid_early_data())
            {
                return Err(StateValidationError::InvalidProtocolOptions);
            }
        }

        for pool in &self.pools {
            if pool.sources.is_empty() {
                return Err(StateValidationError::EmptyPoolSources);
            }
            for source in &pool.sources {
                if !providers.contains(&source.provider_id) {
                    return Err(StateValidationError::MissingProvider);
                }
                source.filter.validate(&source.provider_id, &self.nodes)?;
            }
            if let SelectionPolicy::Manual {
                selected_node_id: Some(node_id),
            } = &pool.selection
                && !self.resolve_pool_members(pool).contains(node_id)
            {
                return Err(StateValidationError::InvalidSelection);
            }
            if let SelectionPolicy::UrlTest {
                probe_url,
                interval_secs,
                ..
            } = &pool.selection
                && (probe_url.trim().is_empty() || *interval_secs == 0)
            {
                return Err(StateValidationError::InvalidSelection);
            }
        }

        for route in &self.routes {
            if !route.matcher.is_valid() {
                return Err(StateValidationError::InvalidRoute);
            }
            if let RouteTarget::Pool(pool_id) = &route.target
                && !pools.contains(pool_id)
            {
                return Err(StateValidationError::MissingPool);
            }
        }

        Ok(())
    }

    pub fn resolve_pool_members(&self, pool: &NodePool) -> Vec<NodeId> {
        let mut members = pool
            .sources
            .iter()
            .flat_map(|source| {
                self.nodes
                    .iter()
                    .filter(move |node| {
                        node.provider_id == source.provider_id && source.filter.matches(node)
                    })
                    .map(|node| node.id.clone())
            })
            .collect::<Vec<_>>();
        members.sort();
        members.dedup();
        members
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::empty()
    }
}

/// 仅供当前后端编译输入使用；不进入 AppState 序列化或状态迁移。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DnsPolicy {
    System,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeIntent {
    pub nodes: Vec<ProxyNode>,
    pub pools: Vec<RuntimePool>,
    pub routes: Vec<RoutePolicy>,
}

impl RuntimeIntent {
    pub fn from_state(state: &AppState) -> Result<Self, StateValidationError> {
        state.validate()?;
        let mut pools = state
            .pools
            .iter()
            .filter(|pool| pool.enabled)
            .map(|pool| RuntimePool {
                id: pool.id.clone(),
                members: state.resolve_pool_members(pool),
                selection: pool.selection.clone(),
            })
            .collect::<Vec<_>>();
        if pools.iter().any(|pool| pool.members.is_empty()) {
            return Err(StateValidationError::EmptyPoolMembership);
        }
        pools.sort_by(|left, right| left.id.cmp(&right.id));
        let mut nodes = state.nodes.clone();
        nodes.sort_by(|left, right| left.id.cmp(&right.id));
        let mut routes = state
            .routes
            .iter()
            .filter(|route| route.enabled)
            .cloned()
            .collect::<Vec<_>>();
        let active_pool_ids = pools.iter().map(|pool| &pool.id).collect::<HashSet<_>>();
        if routes.iter().any(|route| {
            matches!(
                &route.target,
                RouteTarget::Pool(pool_id) if !active_pool_ids.contains(pool_id)
            )
        }) {
            return Err(StateValidationError::InactivePoolTarget);
        }
        routes.sort_by_key(|route| (route.priority, route.id.clone()));
        Ok(Self {
            nodes,
            pools,
            routes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimePool {
    pub id: PoolId,
    pub members: Vec<NodeId>,
    pub selection: SelectionPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionDocumentFormat {
    Json,
    Yaml,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubscriptionDocument {
    pub format: SubscriptionDocumentFormat,
    pub content: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub local_override: bool,
}

impl fmt::Debug for SubscriptionDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubscriptionDocument")
            .field("format", &self.format)
            .field("content", &"[redacted]")
            .field("local_override", &self.local_override)
            .finish()
    }
}

impl SubscriptionDocument {
    fn is_valid(&self) -> bool {
        !self.content.is_empty() && self.content.len() <= MAX_SUBSCRIPTION_DOCUMENT_BYTES
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub name: String,
    pub description: String,
    pub source: SubscriptionSource,
    pub last_success_at_ms: Option<u64>,
    pub last_attempt_at_ms: Option<u64>,
    pub http_metadata: Option<SubscriptionHttpMetadata>,
    pub remote_request: Option<RemoteRequestOptions>,
    pub update_policy: SubscriptionUpdatePolicy,
    #[serde(default, skip_serializing_if = "is_zero_u32")]
    pub skipped_unsupported_nodes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<SubscriptionDocument>,
}

impl Subscription {
    fn validate(&self) -> Result<(), StateValidationError> {
        if self
            .last_success_at_ms
            .into_iter()
            .chain(self.last_attempt_at_ms)
            .any(|value| value > MAX_SAFE_INTEGER)
            || self.description != self.description.trim()
            || self.description.chars().count() > 280
            || self.description.chars().any(char::is_control)
            || self
                .document
                .as_ref()
                .is_some_and(|document| !document.is_valid())
        {
            return Err(StateValidationError::InvalidSubscription);
        }
        match &self.source {
            SubscriptionSource::Remote { url }
                if url.trim().is_empty()
                    || url.len() > 8_192
                    || self
                        .remote_request
                        .as_ref()
                        .is_none_or(|value| !value.is_valid()) =>
            {
                Err(StateValidationError::InvalidSubscription)
            }
            SubscriptionSource::Manual
                if self.http_metadata.is_some()
                    || self.remote_request.is_some()
                    || self.update_policy.allow_auto_update
                    || self.update_policy.interval_minutes.is_some()
                    || self
                        .document
                        .as_ref()
                        .is_some_and(|document| document.local_override) =>
            {
                Err(StateValidationError::InvalidSubscription)
            }
            _ if self
                .http_metadata
                .as_ref()
                .is_some_and(|metadata| !metadata.is_valid()) =>
            {
                Err(StateValidationError::InvalidSubscription)
            }
            _ if !self.update_policy.is_valid() => Err(StateValidationError::InvalidSubscription),
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RemoteRequestOptions {
    pub user_agent: Option<String>,
    pub timeout_seconds: u16,
    pub proxy_mode: SubscriptionProxyMode,
    pub verify_tls: bool,
}

impl RemoteRequestOptions {
    pub fn default_remote() -> Self {
        Self {
            user_agent: None,
            timeout_seconds: 30,
            proxy_mode: SubscriptionProxyMode::Direct,
            verify_tls: true,
        }
    }

    fn is_valid(&self) -> bool {
        (5..=120).contains(&self.timeout_seconds)
            && self.user_agent.as_ref().is_none_or(|value| {
                (1..=256).contains(&value.len())
                    && value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
            })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionProxyMode {
    Direct,
    System,
    ManagedCore,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubscriptionUpdatePolicy {
    pub allow_auto_update: bool,
    pub interval_minutes: Option<u32>,
}

impl SubscriptionUpdatePolicy {
    pub fn default_remote() -> Self {
        Self {
            allow_auto_update: true,
            interval_minutes: None,
        }
    }

    pub fn manual() -> Self {
        Self {
            allow_auto_update: false,
            interval_minutes: None,
        }
    }

    fn is_valid(&self) -> bool {
        self.interval_minutes.is_none_or(|minutes| minutes >= 1_440)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionSource {
    Remote { url: String },
    Manual,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubscriptionHttpMetadata {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub subscription_userinfo: Option<SubscriptionTraffic>,
    pub content_disposition: Option<String>,
}

impl SubscriptionHttpMetadata {
    fn is_valid(&self) -> bool {
        bounded_visible(&self.etag, 1_024)
            && bounded_visible(&self.last_modified, 128)
            && bounded_visible(&self.content_disposition, 255)
            && self
                .subscription_userinfo
                .as_ref()
                .is_none_or(SubscriptionTraffic::is_valid)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct SubscriptionTraffic {
    pub upload: Option<u64>,
    pub download: Option<u64>,
    pub total: Option<u64>,
    pub expire_at_ms: Option<u64>,
}

impl SubscriptionTraffic {
    fn is_valid(&self) -> bool {
        [self.upload, self.download, self.total, self.expire_at_ms]
            .into_iter()
            .flatten()
            .all(|value| value <= 9_007_199_254_740_991)
    }
}

fn bounded_visible(value: &Option<String>, max_bytes: usize) -> bool {
    value.as_ref().is_none_or(|value| {
        !value.is_empty()
            && value.len() <= max_bytes
            && value.chars().all(|character| !character.is_control())
    })
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Provider {
    pub id: ProviderId,
    pub subscription_id: SubscriptionId,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyNode {
    pub id: NodeId,
    pub provider_id: ProviderId,
    pub name: String,
    pub protocol: ProxyProtocol,
    pub server: String,
    pub port: u16,
    pub options: ProtocolOptions,
    pub transport: Option<Transport>,
    pub tls: Option<TlsOptions>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyProtocol {
    Socks,
    Http,
    Shadowsocks,
    Vmess,
    Vless,
    Trojan,
    WireGuard,
    Hysteria,
    Hysteria2,
    Tuic,
    ShadowTls,
    Ssh,
    Naive,
    AnyTls,
    Snell,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProtocolOptions {
    Socks {
        version: u8,
        username: Option<String>,
        password: Option<String>,
    },
    Http {
        username: Option<String>,
        password: Option<String>,
        tls: bool,
    },
    Shadowsocks {
        method: String,
        password: String,
    },
    Vmess {
        uuid: String,
        alter_id: Option<u32>,
        security: Option<String>,
    },
    Vless {
        uuid: String,
        flow: Option<String>,
    },
    Trojan {
        password: String,
    },
    WireGuard {
        private_key: String,
        peer_public_key: String,
        pre_shared_key: Option<String>,
        local_addresses: Vec<String>,
        mtu: Option<u32>,
        reserved: Option<[u8; 3]>,
    },
    Hysteria {
        auth: String,
        obfs: Option<String>,
        up_mbps: Option<u32>,
        down_mbps: Option<u32>,
    },
    Hysteria2 {
        password: String,
        obfs: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        up_mbps: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        down_mbps: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        disable_path_mtu_discovery: Option<bool>,
    },
    Tuic {
        uuid: String,
        password: String,
        congestion_control: Option<String>,
        udp_relay_mode: Option<String>,
        zero_rtt: bool,
    },
    ShadowTls {
        version: u8,
        password: String,
    },
    Ssh {
        user: String,
        password: Option<String>,
        private_key: Option<String>,
        private_key_passphrase: Option<String>,
        host_key: Option<String>,
    },
    Naive {
        username: String,
        password: String,
    },
    AnyTls {
        password: String,
    },
    Snell {
        psk: String,
        version: u8,
    },
}

impl ProtocolOptions {
    pub fn is_compatible_with(&self, protocol: ProxyProtocol) -> bool {
        matches!(
            (protocol, self),
            (ProxyProtocol::Socks, Self::Socks { version: 4 | 5, .. })
                | (ProxyProtocol::Http, Self::Http { .. })
                | (ProxyProtocol::Shadowsocks, Self::Shadowsocks { .. })
                | (ProxyProtocol::Vmess, Self::Vmess { .. })
                | (ProxyProtocol::Vless, Self::Vless { .. })
                | (ProxyProtocol::Trojan, Self::Trojan { .. })
                | (ProxyProtocol::WireGuard, Self::WireGuard { .. })
                | (ProxyProtocol::Hysteria, Self::Hysteria { .. })
                | (ProxyProtocol::Hysteria2, Self::Hysteria2 { .. })
                | (ProxyProtocol::Tuic, Self::Tuic { .. })
                | (ProxyProtocol::ShadowTls, Self::ShadowTls { .. })
                | (ProxyProtocol::Ssh, Self::Ssh { .. })
                | (ProxyProtocol::Naive, Self::Naive { .. })
                | (ProxyProtocol::AnyTls, Self::AnyTls { .. })
                | (ProxyProtocol::Snell, Self::Snell { .. })
        ) && self.has_required_values()
    }

    fn has_required_values(&self) -> bool {
        let present = |value: &str| !value.trim().is_empty();
        match self {
            Self::Socks {
                username, password, ..
            }
            | Self::Http {
                username, password, ..
            } => {
                username.as_deref().is_none_or(present)
                    && password.as_deref().is_none_or(present)
                    && username.is_some() == password.is_some()
            }
            Self::Shadowsocks { method, password } => present(method) && present(password),
            Self::Vmess { uuid, .. } | Self::Vless { uuid, .. } => present(uuid),
            Self::Trojan { password }
            | Self::AnyTls { password }
            | Self::ShadowTls { password, .. } => present(password),
            Self::Hysteria2 {
                password,
                obfs,
                up_mbps,
                down_mbps,
                ..
            } => {
                present(password)
                    && obfs.as_deref().is_none_or(present)
                    && up_mbps.is_none_or(|value| value > 0)
                    && down_mbps.is_none_or(|value| value > 0)
            }
            Self::WireGuard {
                private_key,
                peer_public_key,
                local_addresses,
                ..
            } => {
                present(private_key)
                    && present(peer_public_key)
                    && !local_addresses.is_empty()
                    && local_addresses.iter().all(|address| present(address))
            }
            Self::Hysteria { auth, .. } => present(auth),
            Self::Tuic { uuid, password, .. } => present(uuid) && present(password),
            Self::Ssh {
                user,
                password,
                private_key,
                ..
            } => {
                present(user)
                    && (password.as_deref().is_some_and(present)
                        || private_key.as_deref().is_some_and(present))
            }
            Self::Naive { username, password } => present(username) && present(password),
            Self::Snell { psk, version } => present(psk) && (1..=5).contains(version),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Tcp,
    Websocket {
        path: String,
        host: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max_early_data: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        early_data_header_name: Option<String>,
    },
    Grpc {
        service_name: String,
    },
}

impl Transport {
    /// 保存和配置生成共用提前数据校验，避免握手参数在边界间丢失。
    pub(crate) fn has_valid_early_data(&self) -> bool {
        let Self::Websocket {
            max_early_data,
            early_data_header_name,
            ..
        } = self
        else {
            return true;
        };
        max_early_data.is_none_or(|value| value > 0)
            && early_data_header_name.as_ref().is_none_or(|name| {
                max_early_data.is_some_and(|value| value > 0)
                    && !name.is_empty()
                    && name.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
                    })
            })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TlsOptions {
    pub server_name: Option<String>,
    pub allow_insecure: bool,
    pub reality_public_key: Option<String>,
    pub reality_short_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alpn: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utls_fingerprint: Option<String>,
}

impl TlsOptions {
    pub(crate) fn is_valid(&self) -> bool {
        self.alpn.len() <= 16
            && self.alpn.iter().all(|protocol| {
                (1..=255).contains(&protocol.chars().count())
                    && protocol.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
            })
            && self.alpn.iter().collect::<HashSet<_>>().len() == self.alpn.len()
            && self.utls_fingerprint.as_deref().is_none_or(|fingerprint| {
                matches!(
                    fingerprint,
                    "chrome"
                        | "firefox"
                        | "edge"
                        | "safari"
                        | "360"
                        | "qq"
                        | "ios"
                        | "android"
                        | "random"
                        | "randomized"
                )
            })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodePool {
    pub id: PoolId,
    pub name: String,
    pub kind: PoolKind,
    pub sources: Vec<PoolSource>,
    pub selection: SelectionPolicy,
    pub enabled: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolKind {
    ImplicitProvider,
    Custom,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PoolSource {
    pub provider_id: ProviderId,
    pub filter: NodeFilter,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct NodeFilter {
    pub regions: Vec<String>,
    pub protocols: Vec<ProxyProtocol>,
    pub include_keywords: Vec<String>,
    pub exclude_keywords: Vec<String>,
    pub include_node_ids: Vec<NodeId>,
    pub exclude_node_ids: Vec<NodeId>,
}

impl NodeFilter {
    fn validate(
        &self,
        provider_id: &ProviderId,
        nodes: &[ProxyNode],
    ) -> Result<(), StateValidationError> {
        if self
            .include_node_ids
            .iter()
            .chain(self.exclude_node_ids.iter())
            .any(|id| {
                nodes
                    .iter()
                    .find(|node| node.id == *id)
                    .is_none_or(|node| node.provider_id != *provider_id)
            })
            || self
                .include_node_ids
                .iter()
                .any(|id| self.exclude_node_ids.contains(id))
        {
            return Err(StateValidationError::InvalidFilter);
        }
        Ok(())
    }

    fn matches(&self, node: &ProxyNode) -> bool {
        let name = node.name.to_ascii_lowercase();
        (self.regions.is_empty()
            || self
                .regions
                .iter()
                .any(|region| name.contains(&region.to_ascii_lowercase())))
            && (self.protocols.is_empty() || self.protocols.contains(&node.protocol))
            && (self.include_keywords.is_empty()
                || self
                    .include_keywords
                    .iter()
                    .all(|keyword| name.contains(&keyword.to_ascii_lowercase())))
            && !self
                .exclude_keywords
                .iter()
                .any(|keyword| name.contains(&keyword.to_ascii_lowercase()))
            && (self.include_node_ids.is_empty() || self.include_node_ids.contains(&node.id))
            && !self.exclude_node_ids.contains(&node.id)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SelectionPolicy {
    Manual {
        selected_node_id: Option<NodeId>,
    },
    UrlTest {
        probe_url: String,
        interval_secs: u64,
        tolerance_ms: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RoutePolicy {
    pub id: RoutePolicyId,
    pub name: String,
    pub enabled: bool,
    pub priority: i32,
    pub matcher: TrafficMatcher,
    pub target: RouteTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "values", rename_all = "snake_case")]
pub enum TrafficMatcher {
    Domain(Vec<String>),
    DomainSuffix(Vec<String>),
    Application(Vec<String>),
    IpCidr(Vec<String>),
    Port(Vec<u16>),
    Protocol(Vec<NetworkProtocol>),
}

impl TrafficMatcher {
    fn is_valid(&self) -> bool {
        match self {
            Self::Domain(values)
            | Self::DomainSuffix(values)
            | Self::Application(values)
            | Self::IpCidr(values) => {
                values.iter().all(|value| !value.trim().is_empty()) && !values.is_empty()
            }
            Self::Port(values) => values.iter().all(|value| *value > 0) && !values.is_empty(),
            Self::Protocol(values) => !values.is_empty(),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkProtocol {
    Tcp,
    Udp,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "pool_id", rename_all = "snake_case")]
pub enum RouteTarget {
    Unconfigured,
    Pool(PoolId),
    Direct,
    Block,
}

impl RouteTarget {
    fn validate(&self, pools: &HashSet<&PoolId>) -> Result<(), StateValidationError> {
        if let Self::Pool(pool_id) = self
            && !pools.contains(pool_id)
        {
            return Err(StateValidationError::MissingPool);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateValidationError {
    InvalidIdentifier,
    DuplicateIdentifier,
    MissingSubscription,
    MissingProvider,
    MissingPool,
    EmptyPoolSources,
    InvalidFilter,
    InvalidSelection,
    InvalidRoute,
    InvalidProtocolOptions,
    InvalidSubscription,
    EmptyPoolMembership,
    InactivePoolTarget,
    UnsupportedSchemaVersion,
}

impl fmt::Display for StateValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidIdentifier => "state contains an invalid stable identifier",
            Self::DuplicateIdentifier => "state contains a duplicate stable identifier",
            Self::MissingSubscription => "provider references a missing subscription",
            Self::MissingProvider => "node references a missing provider",
            Self::MissingPool => "route references a missing pool",
            Self::EmptyPoolSources => "pool requires at least one provider source",
            Self::InvalidFilter => "pool contains an invalid node filter",
            Self::InvalidSelection => "pool contains an invalid selection policy",
            Self::InvalidRoute => "route contains an invalid matcher",
            Self::InvalidProtocolOptions => "node options do not match the selected protocol",
            Self::InvalidSubscription => "subscription metadata is invalid",
            Self::EmptyPoolMembership => "enabled pool resolves to no nodes",
            Self::InactivePoolTarget => "enabled route references an inactive pool",
            Self::UnsupportedSchemaVersion => "state schema version is unsupported",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for StateValidationError {}

fn unique_ids<'a, T>(
    ids: impl Iterator<Item = &'a T>,
) -> Result<HashSet<&'a T>, StateValidationError>
where
    T: Eq + std::hash::Hash + StableIdentifier,
{
    let mut unique = HashSet::new();
    for id in ids {
        if !id.is_valid_identifier() {
            return Err(StateValidationError::InvalidIdentifier);
        }
        if !unique.insert(id) {
            return Err(StateValidationError::DuplicateIdentifier);
        }
    }
    Ok(unique)
}

trait StableIdentifier {
    fn is_valid_identifier(&self) -> bool;
}

impl StableIdentifier for SubscriptionId {
    fn is_valid_identifier(&self) -> bool {
        self.is_valid()
    }
}

impl StableIdentifier for ProviderId {
    fn is_valid_identifier(&self) -> bool {
        self.is_valid()
    }
}

impl StableIdentifier for NodeId {
    fn is_valid_identifier(&self) -> bool {
        self.is_valid()
    }
}

impl StableIdentifier for PoolId {
    fn is_valid_identifier(&self) -> bool {
        self.is_valid()
    }
}

impl StableIdentifier for RoutePolicyId {
    fn is_valid_identifier(&self) -> bool {
        self.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> String {
        value.to_owned()
    }

    fn manual_subscription(subscription_id: &str, name: &str) -> Subscription {
        Subscription {
            id: SubscriptionId(id(subscription_id)),
            name: id(name),
            description: String::new(),
            source: SubscriptionSource::Manual,
            last_success_at_ms: None,
            last_attempt_at_ms: None,
            http_metadata: None,
            remote_request: None,
            update_policy: SubscriptionUpdatePolicy::manual(),
            skipped_unsupported_nodes: 0,
            document: None,
        }
    }

    fn valid_state() -> AppState {
        AppState {
            schema_version: CURRENT_SCHEMA_VERSION,
            default_target: RouteTarget::Unconfigured,
            active_subscription_id: None,
            active_configuration_generation: 0,
            subscriptions: vec![manual_subscription("subscription-a", "A")],
            providers: vec![Provider {
                id: ProviderId(id("provider-a")),
                subscription_id: SubscriptionId(id("subscription-a")),
                name: id("A default"),
            }],
            nodes: vec![ProxyNode {
                id: NodeId(id("node-a")),
                provider_id: ProviderId(id("provider-a")),
                name: id("A Hong Kong"),
                protocol: ProxyProtocol::Vless,
                server: id("example.invalid"),
                port: 443,
                options: ProtocolOptions::Vless {
                    uuid: id("secret-not-in-errors"),
                    flow: None,
                },
                transport: Some(Transport::Websocket {
                    path: id("/ws"),
                    host: None,
                    max_early_data: None,
                    early_data_header_name: None,
                }),
                tls: None,
            }],
            pools: Vec::new(),
            routes: Vec::new(),
        }
    }

    #[test]
    fn subscription_document_debug_redacts_content_and_validates_source_boundary() {
        let mut subscription = manual_subscription("subscription-a", "A");
        subscription.document = Some(SubscriptionDocument {
            format: SubscriptionDocumentFormat::Yaml,
            content: "password: fixture-secret".to_owned(),
            local_override: false,
        });
        let rendered = format!("{subscription:?}");
        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("fixture-secret"));
        assert_eq!(subscription.validate(), Ok(()));

        subscription
            .document
            .as_mut()
            .expect("document")
            .local_override = true;
        assert_eq!(
            subscription.validate(),
            Err(StateValidationError::InvalidSubscription)
        );

        subscription
            .document
            .as_mut()
            .expect("document")
            .local_override = false;
        subscription.document.as_mut().expect("document").content =
            "x".repeat(MAX_SUBSCRIPTION_DOCUMENT_BYTES + 1);
        assert_eq!(
            subscription.validate(),
            Err(StateValidationError::InvalidSubscription)
        );
    }

    #[test]
    fn validates_a_multi_subscription_state() {
        let mut state = valid_state();
        state
            .subscriptions
            .push(manual_subscription("subscription-b", "B"));
        state.providers.push(Provider {
            id: ProviderId(id("provider-b")),
            subscription_id: SubscriptionId(id("subscription-b")),
            name: id("B default"),
        });
        state.nodes.push(ProxyNode {
            id: NodeId(id("node-b")),
            provider_id: ProviderId(id("provider-b")),
            ..state.nodes[0].clone()
        });

        assert_eq!(state.validate(), Ok(()));
    }

    #[test]
    fn rejects_duplicate_stable_ids_without_exposing_credentials() {
        let mut state = valid_state();
        state.nodes.push(ProxyNode {
            id: NodeId(id("node-a")),
            ..state.nodes[0].clone()
        });

        let error = state.validate().expect_err("duplicate node id must fail");
        assert_eq!(error, StateValidationError::DuplicateIdentifier);
        assert!(!error.to_string().contains("secret-not-in-errors"));
    }

    #[test]
    fn rejects_options_that_do_not_match_the_protocol() {
        let mut state = valid_state();
        state.nodes[0].options = ProtocolOptions::Trojan {
            password: id("secret-not-in-errors"),
        };

        assert_eq!(
            state.validate(),
            Err(StateValidationError::InvalidProtocolOptions)
        );
    }

    #[test]
    fn v5_absent_compatibility_fields_use_defaults_without_rewriting_them() {
        let mut state = valid_state();
        state.nodes[0].tls = Some(TlsOptions {
            server_name: Some(id("example.invalid")),
            allow_insecure: false,
            reality_public_key: None,
            reality_short_id: None,
            alpn: Vec::new(),
            utls_fingerprint: None,
        });
        let encoded = serde_json::to_value(&state).expect("serialize V5 state");
        assert!(
            encoded["subscriptions"][0]
                .get("skipped_unsupported_nodes")
                .is_none()
        );
        assert!(encoded["nodes"][0]["tls"].get("alpn").is_none());
        assert!(encoded["nodes"][0]["tls"].get("utls_fingerprint").is_none());

        let decoded: AppState = serde_json::from_value(encoded.clone()).expect("read older V5");
        assert_eq!(decoded.subscriptions[0].skipped_unsupported_nodes, 0);
        assert!(decoded.nodes[0].tls.as_ref().expect("TLS").alpn.is_empty());
        assert!(
            decoded.nodes[0]
                .tls
                .as_ref()
                .expect("TLS")
                .utls_fingerprint
                .is_none()
        );
        assert_eq!(
            serde_json::to_value(decoded).expect("serialize defaults"),
            encoded
        );
    }

    #[test]
    fn websocket_early_data_preserves_legacy_state_and_rejects_invalid_parameters() {
        let legacy = serde_json::to_value(valid_state()).unwrap();
        assert!(
            legacy["nodes"][0]["transport"]["websocket"]
                .get("max_early_data")
                .is_none()
        );
        let restored: AppState = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), legacy);
        let mut modern = legacy;
        modern["nodes"][0]["transport"]["websocket"]["max_early_data"] = serde_json::json!(2048);
        modern["nodes"][0]["transport"]["websocket"]["early_data_header_name"] =
            serde_json::json!("Sec-WebSocket-Protocol");
        let restored: AppState = serde_json::from_value(modern.clone()).unwrap();
        assert_eq!(restored.validate(), Ok(()));
        assert_eq!(serde_json::to_value(restored).unwrap(), modern);
        for (field, value) in [
            ("max_early_data", serde_json::json!(0)),
            ("early_data_header_name", serde_json::json!("bad header")),
            ("early_data_header_name", serde_json::json!("")),
        ] {
            let mut invalid = modern.clone();
            invalid["nodes"][0]["transport"]["websocket"][field] = value;
            let restored: AppState = serde_json::from_value(invalid).unwrap();
            assert_eq!(
                restored.validate(),
                Err(StateValidationError::InvalidProtocolOptions)
            );
        }
    }

    #[test]
    fn compatibility_options_and_skipped_count_roundtrip() {
        let mut state = valid_state();
        state.subscriptions[0].skipped_unsupported_nodes = 1;
        state.nodes[0].protocol = ProxyProtocol::Hysteria2;
        state.nodes[0].options = ProtocolOptions::Hysteria2 {
            password: id("synthetic-password"),
            obfs: Some(id("synthetic-obfs")),
            up_mbps: Some(25),
            down_mbps: Some(100),
            disable_path_mtu_discovery: Some(true),
        };
        state.nodes[0].transport = None;
        state.nodes[0].tls = Some(TlsOptions {
            server_name: Some(id("example.invalid")),
            allow_insecure: false,
            reality_public_key: None,
            reality_short_id: None,
            alpn: vec![id("h3"), id("h2")],
            utls_fingerprint: Some(id("chrome")),
        });

        assert_eq!(state.validate(), Ok(()));
        let encoded = serde_json::to_vec(&state).expect("serialize compatibility state");
        let decoded: AppState = serde_json::from_slice(&encoded).expect("read compatibility state");
        assert_eq!(decoded, state);
        assert_eq!(decoded.validate(), Ok(()));
    }

    #[test]
    fn rejects_invalid_alpn_fingerprint_and_hysteria2_rates() {
        let mut state = valid_state();
        state.nodes[0].tls = Some(TlsOptions {
            server_name: None,
            allow_insecure: false,
            reality_public_key: None,
            reality_short_id: None,
            alpn: vec![id("h2"), id("h2")],
            utls_fingerprint: None,
        });
        assert_eq!(
            state.validate(),
            Err(StateValidationError::InvalidProtocolOptions)
        );

        for invalid_alpn in [
            vec![String::new()],
            vec![id("h2"); 17],
            vec!["a".repeat(256)],
            vec![id("非ASCII")],
        ] {
            state.nodes[0].tls.as_mut().expect("TLS").alpn = invalid_alpn;
            assert_eq!(
                state.validate(),
                Err(StateValidationError::InvalidProtocolOptions)
            );
        }

        state.nodes[0].tls.as_mut().expect("TLS").alpn = vec![id("h2")];
        state.nodes[0].tls.as_mut().expect("TLS").utls_fingerprint = Some(id("unknown"));
        assert_eq!(
            state.validate(),
            Err(StateValidationError::InvalidProtocolOptions)
        );

        state.nodes[0].tls = None;
        state.nodes[0].protocol = ProxyProtocol::Hysteria2;
        state.nodes[0].options = ProtocolOptions::Hysteria2 {
            password: id("synthetic-password"),
            obfs: None,
            up_mbps: Some(0),
            down_mbps: Some(1),
            disable_path_mtu_discovery: None,
        };
        assert_eq!(
            state.validate(),
            Err(StateValidationError::InvalidProtocolOptions)
        );
    }

    #[test]
    fn keeps_an_unconfigured_default_target_valid_but_distinct() {
        let state = valid_state();

        assert_eq!(state.default_target, RouteTarget::Unconfigured);
        assert_eq!(state.validate(), Ok(()));
    }

    #[test]
    fn rejects_provider_with_missing_subscription() {
        let mut state = valid_state();
        state.providers[0].subscription_id = SubscriptionId(id("missing"));

        assert_eq!(
            state.validate(),
            Err(StateValidationError::MissingSubscription)
        );
    }

    #[test]
    fn rejects_node_with_missing_provider() {
        let mut state = valid_state();
        state.nodes[0].provider_id = ProviderId(id("missing"));

        assert_eq!(state.validate(), Err(StateValidationError::MissingProvider));
    }

    fn pool(selection: SelectionPolicy) -> NodePool {
        NodePool {
            id: PoolId(id("pool-a")),
            name: id("Default"),
            kind: PoolKind::ImplicitProvider,
            sources: vec![PoolSource {
                provider_id: ProviderId(id("provider-a")),
                filter: NodeFilter::default(),
            }],
            selection,
            enabled: true,
        }
    }

    #[test]
    fn validates_pool_members_and_pool_route_targets() {
        let mut state = valid_state();
        state.pools.push(pool(SelectionPolicy::Manual {
            selected_node_id: Some(NodeId(id("node-a"))),
        }));
        state.routes.push(RoutePolicy {
            id: RoutePolicyId(id("route-a")),
            name: id("Domains"),
            enabled: true,
            priority: 0,
            matcher: TrafficMatcher::DomainSuffix(vec![id("example.com")]),
            target: RouteTarget::Pool(PoolId(id("pool-a"))),
        });

        assert_eq!(state.validate(), Ok(()));
        assert_eq!(
            state.resolve_pool_members(&state.pools[0]),
            vec![NodeId(id("node-a"))]
        );
    }

    #[test]
    fn resolves_a_filtered_pool_across_multiple_providers_by_stable_node_id() {
        let mut state = valid_state();
        state
            .subscriptions
            .push(manual_subscription("subscription-b", "B"));
        state.providers.push(Provider {
            id: ProviderId(id("provider-b")),
            subscription_id: SubscriptionId(id("subscription-b")),
            name: id("B default"),
        });
        state.nodes.push(ProxyNode {
            id: NodeId(id("node-b")),
            provider_id: ProviderId(id("provider-b")),
            name: id("B Japan"),
            ..state.nodes[0].clone()
        });
        state.pools.push(NodePool {
            id: PoolId(id("pool-multi")),
            name: id("Multi provider"),
            kind: PoolKind::Custom,
            sources: vec![
                PoolSource {
                    provider_id: ProviderId(id("provider-b")),
                    filter: NodeFilter {
                        regions: vec![id("japan")],
                        ..NodeFilter::default()
                    },
                },
                PoolSource {
                    provider_id: ProviderId(id("provider-a")),
                    filter: NodeFilter {
                        include_node_ids: vec![NodeId(id("node-a"))],
                        ..NodeFilter::default()
                    },
                },
            ],
            selection: SelectionPolicy::UrlTest {
                probe_url: id("https://example.invalid/probe"),
                interval_secs: 60,
                tolerance_ms: 50,
            },
            enabled: true,
        });

        assert_eq!(state.validate(), Ok(()));
        assert_eq!(
            state.resolve_pool_members(&state.pools[0]),
            vec![NodeId(id("node-a")), NodeId(id("node-b"))]
        );
    }

    #[test]
    fn rejects_a_pool_filter_that_selects_another_providers_node() {
        let mut state = valid_state();
        state
            .subscriptions
            .push(manual_subscription("subscription-b", "B"));
        state.providers.push(Provider {
            id: ProviderId(id("provider-b")),
            subscription_id: SubscriptionId(id("subscription-b")),
            name: id("B default"),
        });
        state.nodes.push(ProxyNode {
            id: NodeId(id("node-b")),
            provider_id: ProviderId(id("provider-b")),
            ..state.nodes[0].clone()
        });
        let mut filter = NodeFilter::default();
        filter.include_node_ids.push(NodeId(id("node-b")));
        state.pools.push(NodePool {
            sources: vec![PoolSource {
                provider_id: ProviderId(id("provider-a")),
                filter,
            }],
            ..pool(SelectionPolicy::Manual {
                selected_node_id: None,
            })
        });

        assert_eq!(state.validate(), Err(StateValidationError::InvalidFilter));
    }

    #[test]
    fn rejects_an_enabled_route_to_an_inactive_pool() {
        let mut state = valid_state();
        state.pools.push(NodePool {
            enabled: false,
            ..pool(SelectionPolicy::Manual {
                selected_node_id: None,
            })
        });
        state.routes.push(RoutePolicy {
            id: RoutePolicyId(id("route-a")),
            name: id("Inactive target"),
            enabled: true,
            priority: 0,
            matcher: TrafficMatcher::Domain(vec![id("example.com")]),
            target: RouteTarget::Pool(PoolId(id("pool-a"))),
        });

        assert_eq!(
            RuntimeIntent::from_state(&state),
            Err(StateValidationError::InactivePoolTarget)
        );
    }

    #[test]
    fn rejects_invalid_pool_and_route_references() {
        let mut state = valid_state();
        state.pools.push(NodePool {
            sources: vec![PoolSource {
                provider_id: ProviderId(id("missing")),
                filter: NodeFilter::default(),
            }],
            ..pool(SelectionPolicy::Manual {
                selected_node_id: None,
            })
        });
        assert_eq!(state.validate(), Err(StateValidationError::MissingProvider));

        state.pools.clear();
        state.routes.push(RoutePolicy {
            id: RoutePolicyId(id("route-a")),
            name: id("Missing pool"),
            enabled: true,
            priority: 0,
            matcher: TrafficMatcher::Domain(vec![id("example.com")]),
            target: RouteTarget::Pool(PoolId(id("missing"))),
        });
        assert_eq!(state.validate(), Err(StateValidationError::MissingPool));
    }

    #[test]
    fn rejects_invalid_filters_and_selection_policies() {
        let mut state = valid_state();
        let mut invalid_filter = NodeFilter::default();
        invalid_filter.include_node_ids.push(NodeId(id("missing")));
        state.pools.push(NodePool {
            sources: vec![PoolSource {
                provider_id: ProviderId(id("provider-a")),
                filter: invalid_filter,
            }],
            ..pool(SelectionPolicy::Manual {
                selected_node_id: None,
            })
        });
        assert_eq!(state.validate(), Err(StateValidationError::InvalidFilter));

        state.pools[0] = pool(SelectionPolicy::UrlTest {
            probe_url: String::new(),
            interval_secs: 0,
            tolerance_ms: 50,
        });
        assert_eq!(
            state.validate(),
            Err(StateValidationError::InvalidSelection)
        );
    }

    #[test]
    fn rejects_subscription_numeric_values_that_cannot_cross_the_ipc_contract() {
        let mut state = valid_state();
        state.subscriptions[0].source = SubscriptionSource::Remote {
            url: "https://example.invalid/sub".to_owned(),
        };
        state.subscriptions[0].http_metadata = Some(SubscriptionHttpMetadata {
            subscription_userinfo: Some(SubscriptionTraffic {
                total: Some(9_007_199_254_740_992),
                ..SubscriptionTraffic::default()
            }),
            ..SubscriptionHttpMetadata::default()
        });
        assert_eq!(
            state.validate(),
            Err(StateValidationError::InvalidSubscription)
        );
    }
}
