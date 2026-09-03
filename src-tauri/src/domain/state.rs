use std::collections::HashSet;
use std::fmt;

use serde::{Deserialize, Serialize};

pub const CURRENT_SCHEMA_VERSION: u32 = 2;

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
        let nodes = unique_ids(self.nodes.iter().map(|value| &value.id))?;
        let pools = unique_ids(self.pools.iter().map(|value| &value.id))?;
        unique_ids(self.routes.iter().map(|value| &value.id))?;

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
        }

        for pool in &self.pools {
            if pool.sources.is_empty() {
                return Err(StateValidationError::EmptyPoolSources);
            }
            for source in &pool.sources {
                if !providers.contains(&source.provider_id) {
                    return Err(StateValidationError::MissingProvider);
                }
                source.filter.validate(&nodes)?;
            }
            if let SelectionPolicy::Manual {
                selected_node_id: Some(node_id),
            } = &pool.selection
            {
                if !self.resolve_pool_members(pool).contains(node_id) {
                    return Err(StateValidationError::InvalidSelection);
                }
            }
            if let SelectionPolicy::UrlTest {
                probe_url,
                interval_secs,
                ..
            } = &pool.selection
            {
                if probe_url.trim().is_empty() || *interval_secs == 0 {
                    return Err(StateValidationError::InvalidSelection);
                }
            }
        }

        for route in &self.routes {
            if !route.matcher.is_valid() {
                return Err(StateValidationError::InvalidRoute);
            }
            if let RouteTarget::Pool(pool_id) = &route.target {
                if !pools.contains(pool_id) {
                    return Err(StateValidationError::MissingPool);
                }
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Subscription {
    pub id: SubscriptionId,
    pub name: String,
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
    pub credentials: NodeCredentials,
    pub transport: Option<Transport>,
    pub tls: Option<TlsOptions>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyProtocol {
    Shadowsocks,
    Vmess,
    Vless,
    Trojan,
    Hysteria2,
    Tuic,
    Socks5,
    Http,
    Https,
    AnyTls,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeCredentials {
    None,
    Password {
        username: Option<String>,
        password: String,
        cipher: Option<String>,
    },
    Uuid {
        uuid: String,
        flow: Option<String>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Tcp,
    Websocket { path: String, host: Option<String> },
    Grpc { service_name: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TlsOptions {
    pub server_name: Option<String>,
    pub allow_insecure: bool,
    pub reality_public_key: Option<String>,
    pub reality_short_id: Option<String>,
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
    fn validate(&self, nodes: &HashSet<&NodeId>) -> Result<(), StateValidationError> {
        if self
            .include_node_ids
            .iter()
            .chain(self.exclude_node_ids.iter())
            .any(|id| !nodes.contains(id))
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
    Pool(PoolId),
    Direct,
    Block,
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
    EmptyPoolMembership,
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
            Self::EmptyPoolMembership => "enabled pool resolves to no nodes",
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

    fn valid_state() -> AppState {
        AppState {
            schema_version: CURRENT_SCHEMA_VERSION,
            subscriptions: vec![Subscription {
                id: SubscriptionId(id("subscription-a")),
                name: id("A"),
            }],
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
                credentials: NodeCredentials::Uuid {
                    uuid: id("secret-not-in-errors"),
                    flow: None,
                },
                transport: Some(Transport::Websocket {
                    path: id("/ws"),
                    host: None,
                }),
                tls: None,
            }],
            pools: Vec::new(),
            routes: Vec::new(),
        }
    }

    #[test]
    fn validates_a_multi_subscription_state() {
        let mut state = valid_state();
        state.subscriptions.push(Subscription {
            id: SubscriptionId(id("subscription-b")),
            name: id("B"),
        });
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
}
