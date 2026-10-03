//! Application use cases are introduced by their dedicated delivery tasks.

pub(crate) mod managed_observation_runtime;
pub use veyra_core::application::observability;
pub use veyra_core::application::provider_replacement;
pub(crate) mod proxy_routing;
pub use veyra_core::application::runtime;
pub use veyra_core::application::selected_subscription;
pub use veyra_core::application::state_access;
pub use veyra_core::application::subscription_management;
pub use veyra_core::application::subscription_scheduler;

#[cfg(test)]
mod runtime_adapter_tests;
