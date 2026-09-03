//! Closed Windows System Proxy adapter, verified with Mock Ports in TASK-004.
//!
//! A real application construction path is deliberately out of this Task's scope, so these
//! internal concrete operations remain test-covered but unconstructed in the production crate.
#![allow(dead_code)]

use std::fmt;
use std::num::NonZeroU16;
use std::sync::Mutex;

use super::recovery::{ProxyRecoveryRecord, ProxyRecoveryStore};

#[cfg(windows)]
use std::mem::size_of;

#[cfg(windows)]
use windows::Win32::Foundation::HGLOBAL;
#[cfg(windows)]
use windows::Win32::Networking::WinInet::{
    INTERNET_OPTION_PER_CONNECTION_OPTION, INTERNET_OPTION_PROXY_SETTINGS_CHANGED,
    INTERNET_OPTION_REFRESH, INTERNET_OPTION_SETTINGS_CHANGED, INTERNET_PER_CONN,
    INTERNET_PER_CONN_AUTOCONFIG_URL, INTERNET_PER_CONN_FLAGS, INTERNET_PER_CONN_FLAGS_UI,
    INTERNET_PER_CONN_OPTION_LISTW, INTERNET_PER_CONN_OPTIONW, INTERNET_PER_CONN_OPTIONW_0,
    INTERNET_PER_CONN_PROXY_BYPASS, INTERNET_PER_CONN_PROXY_SERVER, InternetQueryOptionW,
    InternetSetOptionW, PROXY_TYPE_AUTO_DETECT, PROXY_TYPE_AUTO_PROXY_URL, PROXY_TYPE_DIRECT,
    PROXY_TYPE_PROXY,
};
#[cfg(windows)]
use windows::core::{Free, PWSTR};

const LOOPBACK_HOST: &str = "127.0.0.1";
const LOOPBACK_BYPASS: &str = "localhost;127.0.0.1;::1";

/// The complete proxy-related state for the current user's *default* WinINet connection.
/// Named connections are deliberately absent from this type and cannot be addressed through it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProxySnapshot {
    pub(crate) direct: bool,
    pub(crate) proxy_enabled: bool,
    pub(crate) proxy_server: Option<String>,
    pub(crate) proxy_bypass: Option<String>,
    pub(crate) auto_config_url: Option<String>,
    pub(crate) auto_config_enabled: bool,
    pub(crate) auto_detect: bool,
}

impl ProxySnapshot {
    fn semantically_equals(&self, other: &Self) -> bool {
        self.direct == other.direct
            && self.proxy_enabled == other.proxy_enabled
            && self.auto_config_enabled == other.auto_config_enabled
            && self.auto_detect == other.auto_detect
            && normalized_optional(&self.proxy_server) == normalized_optional(&other.proxy_server)
            && normalized_bypass(&self.proxy_bypass) == normalized_bypass(&other.proxy_bypass)
            && normalized_optional(&self.auto_config_url)
                == normalized_optional(&other.auto_config_url)
    }
}

/// The only system-proxy value this application can apply: a loopback mixed port and fixed
/// bypass list with PAC/WPAD disabled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedProxyState {
    snapshot: ProxySnapshot,
}

impl ManagedProxyState {
    pub(crate) fn loopback(mixed_port: NonZeroU16) -> Self {
        Self {
            snapshot: ProxySnapshot {
                direct: true,
                proxy_enabled: true,
                proxy_server: Some(format!("{LOOPBACK_HOST}:{}", mixed_port.get())),
                proxy_bypass: Some(LOOPBACK_BYPASS.to_owned()),
                auto_config_url: None,
                auto_config_enabled: false,
                auto_detect: false,
            },
        }
    }

    fn matches(&self, observed: &ProxySnapshot) -> bool {
        self.snapshot.semantically_equals(observed)
    }
}

/// A state observed from a fresh WinINet readback. It is intentionally separate from both a
/// pre-change snapshot and the application's intended managed state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ObservedProxyState {
    snapshot: ProxySnapshot,
}

impl ObservedProxyState {
    fn new(snapshot: ProxySnapshot) -> Self {
        Self { snapshot }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProxyState {
    NotManaged,
    Managed,
    UserModified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SystemProxyPortError {
    Unavailable,
    WriteRejected,
    NotificationRejected,
}

/// Fixed operations over only the current user's default WinINet connection.
///
/// There is no registry path, connection name, command, process, or arbitrary native argument
/// in this interface. Production code supplies the WinINet implementation; tests use a mock.
pub(crate) trait SystemProxyPort: Send {
    fn read_default_connection(&mut self) -> Result<ProxySnapshot, SystemProxyPortError>;
    fn write_default_connection(
        &mut self,
        state: &ProxySnapshot,
    ) -> Result<(), SystemProxyPortError>;
    fn notify_settings_changed(&mut self) -> Result<(), SystemProxyPortError>;
    fn refresh_settings(&mut self) -> Result<(), SystemProxyPortError>;
    fn notify_proxy_settings_changed(&mut self) -> Result<(), SystemProxyPortError>;
}

/// Production WinINet port. It is inert until application wiring explicitly constructs it and
/// invokes a `SystemProxyController` operation; module loading and tests never call WinINet.
#[cfg(windows)]
#[derive(Default)]
pub(crate) struct WinInetSystemProxyPort;

#[cfg(windows)]
impl WinInetSystemProxyPort {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[cfg(windows)]
impl SystemProxyPort for WinInetSystemProxyPort {
    fn read_default_connection(&mut self) -> Result<ProxySnapshot, SystemProxyPortError> {
        let mut options = [
            query_option(INTERNET_PER_CONN(INTERNET_PER_CONN_FLAGS_UI)),
            query_option(INTERNET_PER_CONN_PROXY_SERVER),
            query_option(INTERNET_PER_CONN_PROXY_BYPASS),
            query_option(INTERNET_PER_CONN_AUTOCONFIG_URL),
        ];
        let mut list = INTERNET_PER_CONN_OPTION_LISTW {
            dwSize: size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
            pszConnection: PWSTR::null(),
            dwOptionCount: options.len() as u32,
            dwOptionError: 0,
            pOptions: options.as_mut_ptr(),
        };
        let mut list_size = size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32;

        // SAFETY: `list` and its fixed option array are valid writable buffers for the duration
        // of this call; a null connection pointer selects only the current user's default/LAN
        // connection as required by the WinINet contract.
        unsafe {
            InternetQueryOptionW(
                None,
                INTERNET_OPTION_PER_CONNECTION_OPTION,
                Some((&mut list as *mut INTERNET_PER_CONN_OPTION_LISTW).cast()),
                &mut list_size,
            )
        }
        .map_err(|_| SystemProxyPortError::Unavailable)?;

        let flags = unsafe { options[0].Value.dwValue };
        // Consume every WinINet allocation before propagating a malformed-string failure.
        let proxy_server = unsafe { copy_and_free_global_string(options[1].Value.pszValue) };
        let proxy_bypass = unsafe { copy_and_free_global_string(options[2].Value.pszValue) };
        let auto_config_url = unsafe { copy_and_free_global_string(options[3].Value.pszValue) };

        Ok(ProxySnapshot {
            direct: flags & PROXY_TYPE_DIRECT != 0,
            proxy_enabled: flags & PROXY_TYPE_PROXY != 0,
            proxy_server: proxy_server?,
            proxy_bypass: proxy_bypass?,
            auto_config_url: auto_config_url?,
            auto_config_enabled: flags & PROXY_TYPE_AUTO_PROXY_URL != 0,
            auto_detect: flags & PROXY_TYPE_AUTO_DETECT != 0,
        })
    }

    fn write_default_connection(
        &mut self,
        state: &ProxySnapshot,
    ) -> Result<(), SystemProxyPortError> {
        let mut proxy_server = wide_string(state.proxy_server.as_deref());
        let mut proxy_bypass = wide_string(state.proxy_bypass.as_deref());
        let mut auto_config_url = wide_string(state.auto_config_url.as_deref());
        let flags = proxy_flags(state);
        let mut options = [
            value_option(INTERNET_PER_CONN_FLAGS, flags),
            string_option(INTERNET_PER_CONN_PROXY_SERVER, &mut proxy_server),
            string_option(INTERNET_PER_CONN_PROXY_BYPASS, &mut proxy_bypass),
            string_option(INTERNET_PER_CONN_AUTOCONFIG_URL, &mut auto_config_url),
        ];
        let list = INTERNET_PER_CONN_OPTION_LISTW {
            dwSize: size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
            pszConnection: PWSTR::null(),
            dwOptionCount: options.len() as u32,
            dwOptionError: 0,
            pOptions: options.as_mut_ptr(),
        };

        // SAFETY: all option and UTF-16 buffers remain alive for the call; a null connection
        // pointer is deliberately the current user's default/LAN WinINet connection.
        unsafe {
            InternetSetOptionW(
                None,
                INTERNET_OPTION_PER_CONNECTION_OPTION,
                Some((&list as *const INTERNET_PER_CONN_OPTION_LISTW).cast()),
                size_of::<INTERNET_PER_CONN_OPTION_LISTW>() as u32,
            )
        }
        .map_err(|_| SystemProxyPortError::WriteRejected)
    }

    fn notify_settings_changed(&mut self) -> Result<(), SystemProxyPortError> {
        set_empty_wininet_option(INTERNET_OPTION_SETTINGS_CHANGED)
    }

    fn refresh_settings(&mut self) -> Result<(), SystemProxyPortError> {
        set_empty_wininet_option(INTERNET_OPTION_REFRESH)
    }

    fn notify_proxy_settings_changed(&mut self) -> Result<(), SystemProxyPortError> {
        set_empty_wininet_option(INTERNET_OPTION_PROXY_SETTINGS_CHANGED)
    }
}

#[cfg(windows)]
fn query_option(option: INTERNET_PER_CONN) -> INTERNET_PER_CONN_OPTIONW {
    INTERNET_PER_CONN_OPTIONW {
        dwOption: option,
        Value: INTERNET_PER_CONN_OPTIONW_0 { dwValue: 0 },
    }
}

#[cfg(windows)]
fn value_option(option: INTERNET_PER_CONN, value: u32) -> INTERNET_PER_CONN_OPTIONW {
    INTERNET_PER_CONN_OPTIONW {
        dwOption: option,
        Value: INTERNET_PER_CONN_OPTIONW_0 { dwValue: value },
    }
}

#[cfg(windows)]
fn string_option(
    option: INTERNET_PER_CONN,
    value: &mut Option<Vec<u16>>,
) -> INTERNET_PER_CONN_OPTIONW {
    INTERNET_PER_CONN_OPTIONW {
        dwOption: option,
        Value: INTERNET_PER_CONN_OPTIONW_0 {
            pszValue: value
                .as_mut()
                .map_or(PWSTR::null(), |value| PWSTR::from_raw(value.as_mut_ptr())),
        },
    }
}

#[cfg(windows)]
fn wide_string(value: Option<&str>) -> Option<Vec<u16>> {
    value.map(|value| value.encode_utf16().chain(Some(0)).collect())
}

#[cfg(windows)]
fn proxy_flags(state: &ProxySnapshot) -> u32 {
    (state.direct as u32 * PROXY_TYPE_DIRECT)
        | (state.proxy_enabled as u32 * PROXY_TYPE_PROXY)
        | (state.auto_config_enabled as u32 * PROXY_TYPE_AUTO_PROXY_URL)
        | (state.auto_detect as u32 * PROXY_TYPE_AUTO_DETECT)
}

#[cfg(windows)]
fn set_empty_wininet_option(option: u32) -> Result<(), SystemProxyPortError> {
    // SAFETY: this notification accepts a null WinINet handle and no buffer by API contract.
    unsafe { InternetSetOptionW(None, option, None, 0) }
        .map_err(|_| SystemProxyPortError::NotificationRejected)
}

#[cfg(windows)]
unsafe fn copy_and_free_global_string(
    value: PWSTR,
) -> Result<Option<String>, SystemProxyPortError> {
    if value.is_null() {
        return Ok(None);
    }

    // SAFETY: WinINet returned this global allocation as a valid NUL-terminated UTF-16 string.
    let copied = unsafe { value.to_string() }.map_err(|_| SystemProxyPortError::Unavailable);
    // SAFETY: Microsoft documents that each string returned for these query options must be
    // released with GlobalFree exactly once after consumption.
    let mut allocation = HGLOBAL(value.as_ptr().cast());
    // SAFETY: the query API transferred ownership of this global allocation to this caller.
    unsafe { allocation.free() };
    copied
        .map(Some)
        .map_err(|_| SystemProxyPortError::Unavailable)
}

pub(crate) trait SystemProxyController: Send + Sync {
    fn enable_loopback_proxy(
        &self,
        mixed_port: NonZeroU16,
    ) -> Result<SystemProxyEnableOutcome, SystemProxyEnableError>;
    fn restore_proxy(&self) -> Result<SystemProxyRestoreOutcome, SystemProxyError>;
    fn state(&self) -> Result<ProxyState, SystemProxyError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SystemProxyEnableOutcome {
    Enabled,
}

/// 启用失败的补偿确定性。只有 `SafelyUnapplied` 允许调用方停止 sidecar；任何
/// 写入、回读、回滚或恢复记录状态未能证明时，都必须保留 sidecar 进入恢复状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SystemProxyEnableError {
    SafelyUnapplied(SystemProxyError),
    StateUncertain(SystemProxyError),
}

impl fmt::Display for SystemProxyEnableError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SafelyUnapplied(_) => formatter.write_str("Windows system proxy was not applied"),
            Self::StateUncertain(_) => {
                formatter.write_str("Windows system proxy state is uncertain and requires recovery")
            }
        }
    }
}

impl std::error::Error for SystemProxyEnableError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SystemProxyRestoreOutcome {
    Restored,
    NotManaged,
    UserModified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SystemProxyError {
    Read,
    Write,
    Notify,
    RecoveryStore,
    Verification,
}

impl fmt::Display for SystemProxyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Read => "unable to read the Windows system proxy state",
            Self::Write => "unable to write the Windows system proxy state",
            Self::Notify => "unable to notify Windows about the system proxy state",
            Self::RecoveryStore => "unable to update the private proxy recovery state",
            Self::Verification => "Windows system proxy state did not verify after the operation",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SystemProxyError {}

/// Serializes capture, apply, verification, and recovery. It never changes the real system
/// unless a production `SystemProxyPort` is explicitly constructed and supplied by application
/// wiring; unit tests only inject in-memory ports.
pub(crate) struct WindowsSystemProxy<P, R> {
    inner: Mutex<Inner<P, R>>,
}

struct Inner<P, R> {
    port: P,
    recovery: R,
}

impl<P, R> WindowsSystemProxy<P, R>
where
    P: SystemProxyPort,
    R: ProxyRecoveryStore,
{
    pub(crate) fn new(port: P, recovery: R) -> Self {
        Self {
            inner: Mutex::new(Inner { port, recovery }),
        }
    }

    fn read(inner: &mut Inner<P, R>) -> Result<ObservedProxyState, SystemProxyError> {
        inner
            .port
            .read_default_connection()
            .map(ObservedProxyState::new)
            .map_err(|_| SystemProxyError::Read)
    }

    fn write_and_notify(
        inner: &mut Inner<P, R>,
        state: &ProxySnapshot,
    ) -> Result<(), SystemProxyError> {
        inner
            .port
            .write_default_connection(state)
            .map_err(|_| SystemProxyError::Write)?;
        inner
            .port
            .notify_settings_changed()
            .map_err(|_| SystemProxyError::Notify)?;
        inner
            .port
            .refresh_settings()
            .map_err(|_| SystemProxyError::Notify)?;
        inner
            .port
            .notify_proxy_settings_changed()
            .map_err(|_| SystemProxyError::Notify)
    }

    fn rollback_if_still_managed(
        inner: &mut Inner<P, R>,
        record: &ProxyRecoveryRecord,
    ) -> Result<SystemProxyRestoreOutcome, SystemProxyError> {
        let observed = Self::read(inner)?;
        if !record.managed.matches(&observed.snapshot) {
            return Ok(SystemProxyRestoreOutcome::UserModified);
        }

        Self::write_and_notify(inner, &record.snapshot)?;
        let verified = Self::read(inner)?;
        if !verified.snapshot.semantically_equals(&record.snapshot) {
            return Err(SystemProxyError::Verification);
        }
        inner
            .recovery
            .clear()
            .map_err(|_| SystemProxyError::RecoveryStore)?;
        Ok(SystemProxyRestoreOutcome::Restored)
    }

    /// Best-effort compensation for a failed enable. This operation is intentionally stricter
    /// than normal restore: only a verified original snapshot plus a cleared recovery record
    /// proves the sidecar may be stopped. A user change, failed readback, or any failed rollback
    /// is an uncertain state rather than evidence that the loopback endpoint is unused.
    fn prove_enable_unapplied(
        inner: &mut Inner<P, R>,
        record: &ProxyRecoveryRecord,
    ) -> Result<(), SystemProxyEnableError> {
        let observed = Self::read(inner).map_err(SystemProxyEnableError::StateUncertain)?;
        if observed.snapshot.semantically_equals(&record.snapshot) {
            return inner.recovery.clear().map_err(|_| {
                SystemProxyEnableError::StateUncertain(SystemProxyError::RecoveryStore)
            });
        }
        if !record.managed.matches(&observed.snapshot) {
            return Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Verification,
            ));
        }

        Self::write_and_notify(inner, &record.snapshot)
            .map_err(SystemProxyEnableError::StateUncertain)?;
        let verified = Self::read(inner).map_err(SystemProxyEnableError::StateUncertain)?;
        if !verified.snapshot.semantically_equals(&record.snapshot) {
            return Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Verification,
            ));
        }
        inner
            .recovery
            .clear()
            .map_err(|_| SystemProxyEnableError::StateUncertain(SystemProxyError::RecoveryStore))
    }

    fn enable(
        inner: &mut Inner<P, R>,
        managed: ManagedProxyState,
    ) -> Result<SystemProxyEnableOutcome, SystemProxyEnableError> {
        let snapshot = Self::read(inner)
            .map_err(SystemProxyEnableError::SafelyUnapplied)?
            .snapshot;
        let mut record = ProxyRecoveryRecord::transitioning(snapshot, managed);
        inner.recovery.save(&record).map_err(|_| {
            SystemProxyEnableError::SafelyUnapplied(SystemProxyError::RecoveryStore)
        })?;

        if let Err(error) = Self::write_and_notify(inner, &record.managed.snapshot) {
            return match Self::prove_enable_unapplied(inner, &record) {
                Ok(()) => Err(SystemProxyEnableError::SafelyUnapplied(error)),
                Err(_) => Err(SystemProxyEnableError::StateUncertain(error)),
            };
        }

        let observed = Self::read(inner).map_err(SystemProxyEnableError::StateUncertain)?;
        if !record.managed.matches(&observed.snapshot) {
            return match Self::prove_enable_unapplied(inner, &record) {
                Ok(()) => Err(SystemProxyEnableError::SafelyUnapplied(
                    SystemProxyError::Verification,
                )),
                Err(_) => Err(SystemProxyEnableError::StateUncertain(
                    SystemProxyError::Verification,
                )),
            };
        }

        record.mark_stable();
        inner
            .recovery
            .save(&record)
            .map_err(|_| SystemProxyEnableError::StateUncertain(SystemProxyError::RecoveryStore))?;
        Ok(SystemProxyEnableOutcome::Enabled)
    }

    fn restore(inner: &mut Inner<P, R>) -> Result<SystemProxyRestoreOutcome, SystemProxyError> {
        let record = inner
            .recovery
            .load()
            .map_err(|_| SystemProxyError::RecoveryStore)?;
        match record {
            Some(record) => Self::rollback_if_still_managed(inner, &record),
            None => Ok(SystemProxyRestoreOutcome::NotManaged),
        }
    }

    fn current_state(inner: &mut Inner<P, R>) -> Result<ProxyState, SystemProxyError> {
        let record = inner
            .recovery
            .load()
            .map_err(|_| SystemProxyError::RecoveryStore)?;
        let Some(record) = record else {
            return Ok(ProxyState::NotManaged);
        };
        let observed = Self::read(inner)?;
        if record.managed.matches(&observed.snapshot) {
            Ok(ProxyState::Managed)
        } else {
            Ok(ProxyState::UserModified)
        }
    }
}

impl<P, R> SystemProxyController for WindowsSystemProxy<P, R>
where
    P: SystemProxyPort,
    R: ProxyRecoveryStore,
{
    fn enable_loopback_proxy(
        &self,
        mixed_port: NonZeroU16,
    ) -> Result<SystemProxyEnableOutcome, SystemProxyEnableError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| SystemProxyEnableError::StateUncertain(SystemProxyError::RecoveryStore))?;
        Self::enable(&mut inner, ManagedProxyState::loopback(mixed_port))
    }

    fn restore_proxy(&self) -> Result<SystemProxyRestoreOutcome, SystemProxyError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| SystemProxyError::RecoveryStore)?;
        Self::restore(&mut inner)
    }

    fn state(&self) -> Result<ProxyState, SystemProxyError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| SystemProxyError::RecoveryStore)?;
        Self::current_state(&mut inner)
    }
}

fn normalized_optional(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

fn normalized_bypass(value: &Option<String>) -> Vec<String> {
    let mut parts = value
        .as_deref()
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    parts.sort();
    parts.dedup();
    parts
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::platform::windows::recovery::{RecoveryPhase, RecoveryStoreError};

    #[derive(Clone, Debug)]
    struct MockPort {
        current: ProxySnapshot,
        reads: VecDeque<Result<ProxySnapshot, SystemProxyPortError>>,
        fail_write: bool,
        fail_notify: bool,
        fail_refresh: bool,
        fail_proxy_settings_notification: bool,
        writes: Vec<ProxySnapshot>,
        notifications: usize,
        refreshes: usize,
        proxy_settings_notifications: usize,
    }

    impl MockPort {
        fn new(current: ProxySnapshot) -> Self {
            Self {
                current,
                reads: VecDeque::new(),
                fail_write: false,
                fail_notify: false,
                fail_refresh: false,
                fail_proxy_settings_notification: false,
                writes: Vec::new(),
                notifications: 0,
                refreshes: 0,
                proxy_settings_notifications: 0,
            }
        }

        fn next_read(mut self, state: ProxySnapshot) -> Self {
            self.reads.push_back(Ok(state));
            self
        }

        fn failing_read(mut self) -> Self {
            self.reads.push_back(Err(SystemProxyPortError::Unavailable));
            self
        }
    }

    impl SystemProxyPort for MockPort {
        fn read_default_connection(&mut self) -> Result<ProxySnapshot, SystemProxyPortError> {
            if let Some(next) = self.reads.pop_front() {
                let next = next?;
                self.current = next;
            }
            Ok(self.current.clone())
        }

        fn write_default_connection(
            &mut self,
            state: &ProxySnapshot,
        ) -> Result<(), SystemProxyPortError> {
            if self.fail_write {
                return Err(SystemProxyPortError::WriteRejected);
            }
            self.current = state.clone();
            self.writes.push(state.clone());
            Ok(())
        }

        fn notify_settings_changed(&mut self) -> Result<(), SystemProxyPortError> {
            self.notifications += 1;
            if self.fail_notify {
                return Err(SystemProxyPortError::NotificationRejected);
            }
            Ok(())
        }

        fn refresh_settings(&mut self) -> Result<(), SystemProxyPortError> {
            self.refreshes += 1;
            if self.fail_refresh {
                return Err(SystemProxyPortError::NotificationRejected);
            }
            Ok(())
        }

        fn notify_proxy_settings_changed(&mut self) -> Result<(), SystemProxyPortError> {
            self.proxy_settings_notifications += 1;
            if self.fail_proxy_settings_notification {
                return Err(SystemProxyPortError::NotificationRejected);
            }
            Ok(())
        }
    }

    #[derive(Clone, Debug, Default)]
    struct MemoryRecoveryStore {
        record: Option<ProxyRecoveryRecord>,
        fail_save: bool,
        fail_second_save: bool,
        save_count: usize,
        fail_load: bool,
        fail_clear: bool,
    }

    impl ProxyRecoveryStore for MemoryRecoveryStore {
        fn load(&mut self) -> Result<Option<ProxyRecoveryRecord>, RecoveryStoreError> {
            if self.fail_load {
                return Err(RecoveryStoreError::Unavailable);
            }
            Ok(self.record.clone())
        }

        fn save(&mut self, record: &ProxyRecoveryRecord) -> Result<(), RecoveryStoreError> {
            self.save_count += 1;
            if self.fail_save || self.fail_second_save && self.save_count == 2 {
                return Err(RecoveryStoreError::Unavailable);
            }
            self.record = Some(record.clone());
            Ok(())
        }

        fn clear(&mut self) -> Result<(), RecoveryStoreError> {
            if self.fail_clear {
                return Err(RecoveryStoreError::Unavailable);
            }
            self.record = None;
            Ok(())
        }
    }

    fn original() -> ProxySnapshot {
        ProxySnapshot {
            direct: true,
            proxy_enabled: false,
            proxy_server: None,
            proxy_bypass: None,
            auto_config_url: Some("https://pac.example.invalid/proxy.pac".to_owned()),
            auto_config_enabled: true,
            auto_detect: true,
        }
    }

    fn user_modified() -> ProxySnapshot {
        ProxySnapshot {
            direct: true,
            proxy_enabled: true,
            proxy_server: Some("user-proxy.example.invalid:8080".to_owned()),
            proxy_bypass: Some("localhost".to_owned()),
            auto_config_url: Some("https://user.example.invalid/proxy.pac".to_owned()),
            auto_config_enabled: true,
            auto_detect: true,
        }
    }

    fn adapter(
        port: MockPort,
        store: MemoryRecoveryStore,
    ) -> WindowsSystemProxy<MockPort, MemoryRecoveryStore> {
        WindowsSystemProxy::new(port, store)
    }

    fn port() -> NonZeroU16 {
        NonZeroU16::new(2080).expect("non-zero loopback port")
    }

    #[test]
    fn enables_and_restores_complete_pac_wpad_snapshot() {
        let proxy = adapter(MockPort::new(original()), MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Ok(SystemProxyEnableOutcome::Enabled)
        );
        assert_eq!(proxy.state(), Ok(ProxyState::Managed));
        {
            let inner = proxy.inner.lock().expect("test lock");
            assert_eq!(
                inner.recovery.record.as_ref().map(|record| record.phase),
                Some(RecoveryPhase::Stable)
            );
            assert_eq!(inner.port.notifications, 1);
            assert_eq!(inner.port.refreshes, 1);
            assert_eq!(inner.port.proxy_settings_notifications, 1);
        }
        assert_eq!(
            proxy.restore_proxy(),
            Ok(SystemProxyRestoreOutcome::Restored)
        );
        assert_eq!(proxy.state(), Ok(ProxyState::NotManaged));
    }

    #[test]
    fn write_failure_proves_original_system_state_before_allowing_sidecar_stop() {
        let mut mock_port = MockPort::new(original());
        mock_port.fail_write = true;
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::SafelyUnapplied(
                SystemProxyError::Write
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.current, original());
        assert!(inner.port.writes.is_empty());
        assert_eq!(inner.recovery.record, None);
    }

    #[test]
    fn notification_failure_restores_only_the_just_applied_managed_state() {
        let mut mock_port = MockPort::new(original());
        mock_port.fail_notify = true;
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Notify
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.current, original());
        assert_eq!(inner.port.writes.len(), 2);
        assert_eq!(
            inner.recovery.record.as_ref().map(|record| record.phase),
            Some(RecoveryPhase::Transitioning)
        );
    }

    #[test]
    fn notification_failure_never_overwrites_a_concurrent_user_change() {
        let mut mock_port = MockPort::new(original())
            .next_read(original())
            .next_read(user_modified());
        mock_port.fail_notify = true;
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Notify
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.current, user_modified());
        assert_eq!(inner.port.writes.len(), 1);
        assert_eq!(
            inner.recovery.record.as_ref().map(|record| record.phase),
            Some(RecoveryPhase::Transitioning)
        );
    }

    #[test]
    fn proxy_settings_notification_failure_enters_the_existing_uncertain_state() {
        let mut mock_port = MockPort::new(original());
        mock_port.fail_proxy_settings_notification = true;
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Notify
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.writes.len(), 2);
        assert_eq!(inner.port.proxy_settings_notifications, 2);
        assert_eq!(
            inner.recovery.record.as_ref().map(|record| record.phase),
            Some(RecoveryPhase::Transitioning)
        );
    }

    #[test]
    fn readback_mismatch_preserves_user_modified_observed_state() {
        let mock_port = MockPort::new(original())
            .next_read(original())
            .next_read(user_modified())
            .next_read(user_modified());
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Verification
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.current, user_modified());
        assert_eq!(inner.port.writes.len(), 1);
        assert!(inner.recovery.record.is_some());
    }

    #[test]
    fn readback_failure_keeps_the_transitioning_record_without_recursing() {
        let mock_port = MockPort::new(original())
            .next_read(original())
            .failing_read();
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::Read
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(
            inner.recovery.record.as_ref().map(|record| record.phase),
            Some(RecoveryPhase::Transitioning)
        );
        assert_eq!(inner.port.writes.len(), 1);
    }

    #[test]
    fn user_manual_change_blocks_restore_without_an_overwrite() {
        let proxy = adapter(MockPort::new(original()), MemoryRecoveryStore::default());
        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Ok(SystemProxyEnableOutcome::Enabled)
        );
        {
            let mut inner = proxy.inner.lock().expect("test lock");
            inner.port.current = user_modified();
        }

        assert_eq!(
            proxy.restore_proxy(),
            Ok(SystemProxyRestoreOutcome::UserModified)
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.current, user_modified());
        assert_eq!(inner.port.writes.len(), 1);
        assert!(inner.recovery.record.is_some());
    }

    #[test]
    fn transitioning_crash_record_is_only_restored_when_managed_state_is_still_observed() {
        let managed = ManagedProxyState::loopback(port());
        let record = ProxyRecoveryRecord::transitioning(original(), managed.clone());
        let proxy = adapter(
            MockPort::new(managed.snapshot.clone()),
            MemoryRecoveryStore {
                record: Some(record),
                ..MemoryRecoveryStore::default()
            },
        );

        assert_eq!(
            proxy.restore_proxy(),
            Ok(SystemProxyRestoreOutcome::Restored)
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(inner.port.current, original());
        assert!(inner.recovery.record.is_none());
    }

    #[test]
    fn transition_record_is_never_marked_stable_before_readback_verifies() {
        let mock_port = MockPort::new(original())
            .next_read(original())
            .next_read(user_modified())
            .next_read(user_modified());
        let proxy = adapter(mock_port, MemoryRecoveryStore::default());

        let _ = proxy.enable_loopback_proxy(port());
        let inner = proxy.inner.lock().expect("test lock");
        assert_eq!(
            inner.recovery.record.as_ref().map(|record| record.phase),
            Some(RecoveryPhase::Transitioning)
        );
    }

    #[test]
    fn stable_record_failure_reports_uncertain_state_after_verified_proxy_apply() {
        let proxy = adapter(
            MockPort::new(original()),
            MemoryRecoveryStore {
                fail_second_save: true,
                ..MemoryRecoveryStore::default()
            },
        );

        assert_eq!(
            proxy.enable_loopback_proxy(port()),
            Err(SystemProxyEnableError::StateUncertain(
                SystemProxyError::RecoveryStore
            ))
        );
        let inner = proxy.inner.lock().expect("test lock");
        assert!(inner.port.current.proxy_enabled);
        assert_eq!(
            inner.recovery.record.as_ref().map(|record| record.phase),
            Some(RecoveryPhase::Transitioning)
        );
    }

    #[test]
    fn managed_semantic_comparison_accepts_equivalent_bypass_order_only() {
        let managed = ManagedProxyState::loopback(port());
        let mut observed = managed.snapshot.clone();
        observed.proxy_bypass = Some("::1; localhost;127.0.0.1;localhost".to_owned());
        assert!(managed.matches(&observed));

        observed.auto_detect = true;
        assert!(!managed.matches(&observed));
    }
}
