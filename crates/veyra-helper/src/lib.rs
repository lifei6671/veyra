//! 正式 IPC 不链接 P0-05 的 SystemConfiguration/native harness。
#[cfg(all(feature = "production", target_os = "macos"))]
pub mod production;
