//! Typed configuration aggregates and their invariants.

mod state;

pub use state::{
    AppState, CURRENT_SCHEMA_VERSION, DnsPolicy, NetworkProtocol, NodeFilter, NodeId, NodePool,
    PoolId, PoolKind, PoolSource, ProtocolOptions, Provider, ProviderId, ProxyNode, ProxyProtocol,
    RoutePolicy, RoutePolicyId, RouteTarget, RuntimeIntent, RuntimePool, SelectionPolicy,
    StateValidationError, Subscription, SubscriptionId, TlsOptions, TrafficMatcher, Transport,
};
