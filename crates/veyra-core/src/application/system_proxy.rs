//! Platform-independent system proxy lifecycle contract.

use std::{fmt, num::NonZeroU16};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProxyState {
    NotManaged,
    Managed,
    UserModified,
}

pub trait SystemProxyController: Send + Sync {
    fn enable_loopback_proxy(
        &self,
        mixed_port: NonZeroU16,
    ) -> Result<SystemProxyEnableOutcome, SystemProxyEnableError>;
    fn restore_proxy(&self) -> Result<SystemProxyRestoreOutcome, SystemProxyError>;
    fn state(&self) -> Result<ProxyState, SystemProxyError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemProxyEnableOutcome {
    Enabled,
}

/// 启用失败的补偿确定性。只有 `SafelyUnapplied` 允许调用方停止 sidecar；任何
/// 写入、回读、回滚或恢复记录状态未能证明时，都必须保留 sidecar 进入恢复状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemProxyEnableError {
    SafelyUnapplied(SystemProxyError),
    StateUncertain(SystemProxyError),
}

impl fmt::Display for SystemProxyEnableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SafelyUnapplied(_) => formatter.write_str("system proxy was not applied"),
            Self::StateUncertain(_) => {
                formatter.write_str("system proxy state is uncertain and requires recovery")
            }
        }
    }
}

impl std::error::Error for SystemProxyEnableError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemProxyRestoreOutcome {
    Restored,
    NotManaged,
    UserModified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemProxyError {
    Read,
    Write,
    Notify,
    RecoveryStore,
    Verification,
}

impl fmt::Display for SystemProxyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Read => "unable to read the system proxy state",
            Self::Write => "unable to write the system proxy state",
            Self::Notify => "unable to notify the OS about the system proxy state",
            Self::RecoveryStore => "unable to update the private proxy recovery state",
            Self::Verification => "system proxy state did not verify after the operation",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SystemProxyError {}
