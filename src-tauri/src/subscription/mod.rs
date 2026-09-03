//! Subscription parsing and normalization into typed domain nodes.

mod normalize;
mod parser;

pub use normalize::{normalize_nodes, NormalizationError, ProxyNodeDraft};
pub use parser::{parse_subscription, ParseError, ParseResult, SkippedNode, SubscriptionFormat};
