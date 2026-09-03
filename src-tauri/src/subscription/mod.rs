//! Subscription parsing and normalization into typed domain nodes.

mod normalize;
mod parser;

pub use normalize::{NormalizationError, ProxyNodeDraft, normalize_nodes};
pub use parser::{ParseError, ParseResult, SkippedNode, SubscriptionFormat, parse_subscription};
