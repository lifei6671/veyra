//! Shared application use cases; desktop orchestration stays in the entrypoint.

pub mod observability;
pub mod provider_replacement;
pub mod runtime;
pub mod selected_subscription;
pub mod state_access;
pub mod subscription_management;
pub mod subscription_scheduler;
pub mod system_proxy;

pub mod error;
pub mod runtime_snapshot;
pub mod state_service;

pub mod outbound_query;

pub mod manual_runtime;

pub mod runtime_recovery;
