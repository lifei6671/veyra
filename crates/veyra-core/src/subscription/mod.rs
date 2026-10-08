//! Subscription parsing and normalization into typed domain nodes.

mod document;
mod fetch;
mod normalize;
mod parser;

#[cfg(feature = "p0-06-prototype")]
pub mod outbound;

pub use document::{
    DocumentError, DocumentErrorCode, DocumentErrorLocation, format_document, parse_exact_document,
};
pub use fetch::{
    ConditionalHeaders, FetchClientOptions, FetchError, FetchResult,
    fetch_subscription_with_options, validate_source_url,
};
pub use normalize::{NormalizationError, ProxyNodeDraft, normalize_nodes};
pub use parser::{ParseError, ParseResult, SkippedNode, SubscriptionFormat, parse_subscription};
