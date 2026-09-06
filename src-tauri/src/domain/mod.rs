//! Typed configuration aggregates and their invariants.

mod state;

pub use state::{
    AppState, CURRENT_SCHEMA_VERSION, DnsPolicy, MAX_SAFE_INTEGER, MAX_SUBSCRIPTION_DOCUMENT_BYTES,
    NetworkProtocol, NodeFilter, NodeId, NodePool, PoolId, PoolKind, PoolSource, ProtocolOptions,
    Provider, ProviderId, ProxyNode, ProxyProtocol, RemoteRequestOptions, RoutePolicy,
    RoutePolicyId, RouteTarget, RuntimeIntent, RuntimePool, SelectionPolicy, StateValidationError,
    Subscription, SubscriptionDocument, SubscriptionDocumentFormat, SubscriptionHttpMetadata,
    SubscriptionId, SubscriptionProxyMode, SubscriptionSource, SubscriptionTraffic,
    SubscriptionUpdatePolicy, TlsOptions, TrafficMatcher, Transport,
};
