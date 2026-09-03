//! Private recovery facts for the System Proxy adapter.
//!
//! TASK-004 deliberately does not wire a production system-proxy operation; these recovery
//! types are verified through the adapter's Mock tests until that later authorized integration.
#![allow(dead_code)]

use super::system_proxy::{ManagedProxyState, ProxySnapshot};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryPhase {
    Transitioning,
    Stable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProxyRecoveryRecord {
    pub(crate) phase: RecoveryPhase,
    pub(crate) snapshot: ProxySnapshot,
    pub(crate) managed: ManagedProxyState,
}

impl ProxyRecoveryRecord {
    pub(crate) fn transitioning(snapshot: ProxySnapshot, managed: ManagedProxyState) -> Self {
        Self {
            phase: RecoveryPhase::Transitioning,
            snapshot,
            managed,
        }
    }

    pub(crate) fn mark_stable(&mut self) {
        self.phase = RecoveryPhase::Stable;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RecoveryStoreError {
    Unavailable,
}

/// Private persistence for recovery facts. Implementations must keep these facts outside
/// `AppState` and only expose this closed record shape.
pub(crate) trait ProxyRecoveryStore: Send {
    fn load(&mut self) -> Result<Option<ProxyRecoveryRecord>, RecoveryStoreError>;
    fn save(&mut self, record: &ProxyRecoveryRecord) -> Result<(), RecoveryStoreError>;
    fn clear(&mut self) -> Result<(), RecoveryStoreError>;
}
