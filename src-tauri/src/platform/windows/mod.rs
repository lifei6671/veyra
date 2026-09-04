//! Windows-only platform semantics. No Win32 details cross this module boundary.

pub(crate) mod managed_sidecar_port;
pub(crate) mod private_runtime;
pub(crate) mod recovery;
pub(crate) mod system_proxy;

pub(crate) use system_proxy::{
    SystemProxyController, SystemProxyEnableError, SystemProxyRestoreOutcome,
};
