//! Typed configuration aggregates and their invariants.

mod state;

pub use state::{
    AppState, NetworkProtocol, NodeCredentials, NodeFilter, NodeId, NodePool, PoolId, PoolKind,
    PoolSource, Provider, ProviderId, ProxyNode, ProxyProtocol, RoutePolicy, RoutePolicyId,
    RouteTarget, RuntimeIntent, RuntimePool, SelectionPolicy, StateValidationError, Subscription,
    SubscriptionId, TlsOptions, TrafficMatcher, Transport, CURRENT_SCHEMA_VERSION,
};
