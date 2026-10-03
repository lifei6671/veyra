//! Windows-only platform semantics. No Win32 details cross this module boundary.

#[cfg(windows)]
pub(crate) mod managed_sidecar_port;
#[cfg(windows)]
pub(crate) mod private_runtime;
pub(crate) mod recovery;
pub(crate) mod system_proxy;
