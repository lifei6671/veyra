//! Versioned, whole-state persistence.

mod migration;
mod snapshot;
mod store;
mod validation;

pub use store::{JsonStateStore, StateStore, StateStoreError};
