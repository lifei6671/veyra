//! Windows-only platform semantics. No Win32 details cross this module boundary.

pub(crate) mod recovery;
pub(crate) mod system_proxy;

pub(crate) use system_proxy::{
    SystemProxyController, SystemProxyEnableError, SystemProxyRestoreOutcome,
};
