//! P2-06 生产 IPC：固定安装端点、OS 会话身份与可查询操作。Native管理员安装与降权仍待实机验收。
pub use veyra_core::application::helper_protocol::*;
mod host;
mod transport;
pub use host::serve_installed;
pub use transport::Client;
#[cfg(test)]
mod tests;

mod assets;
mod backend;
mod process;
#[cfg(test)]
mod process_tests;

mod install;
pub use install::{install, installation_manifest, installation_plist, uninstall};

mod admin;

mod archive;

mod source;

mod writer_lease;

mod bootstrap;

#[cfg(test)]
mod bootstrap_tests;
