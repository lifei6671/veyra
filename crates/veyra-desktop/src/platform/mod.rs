//! macOS desktop ownership and user-selected file boundaries; never a Core dependency.
pub mod directories;
pub mod files;
pub mod macos;
pub mod manual_sidecar;
pub mod single_instance;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformError {
    InstanceBusyUnconfirmed,
    PermissionDenied,
    InvalidFile,
    FileTooLarge,
    FileReadFailed,
    FileWriteFailed,
    FileDialogUnavailable,
    ExternalUrlRejected,
    ClipboardUnavailable,
}
impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}") // Fixed codes only: no home/path/clipboard contents.
    }
}
impl std::error::Error for PlatformError {}
