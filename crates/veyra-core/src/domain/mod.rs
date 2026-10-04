//! Typed configuration aggregates and their invariants.

mod error;
mod profile;
mod state;
mod version;

pub use error::{AppError, AppErrorCode, ErrorDetail, FieldPath};
pub use profile::*;
pub use version::{ConfigVersion, SelectionVersion, SnapshotVersion, StateEpoch, StateVersion};

pub use state::{
    AppConfig, AppState, CURRENT_SCHEMA_VERSION, DnsPolicy, MAX_SAFE_INTEGER,
    MAX_SUBSCRIPTION_DOCUMENT_BYTES, NetworkProtocol, NodeFilter, NodeId, NodePool, PoolId,
    PoolKind, PoolSource, ProtocolOptions, Provider, ProviderId, ProxyNode, ProxyProtocol,
    RemoteRequestOptions, RoutePolicy, RoutePolicyId, RouteTarget, RuntimeIntent, RuntimePool,
    SelectionPolicy, StateValidationError, Subscription, SubscriptionDocument,
    SubscriptionDocumentFormat, SubscriptionHttpMetadata, SubscriptionId, SubscriptionProxyMode,
    SubscriptionSource, SubscriptionTraffic, SubscriptionUpdatePolicy, TlsOptions, TrafficMatcher,
    Transport,
};

mod desktop_visual;
pub use desktop_visual::*;

mod desktop_behavior;
pub use desktop_behavior::*;
