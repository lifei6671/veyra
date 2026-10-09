//! Versioned, whole-state persistence.

mod migration;
mod snapshot;
mod store;
mod validation;

pub use store::{JsonStateStore, RemoteSelection, SelectionFence, StateStore, StateStoreError};

pub mod traffic;
