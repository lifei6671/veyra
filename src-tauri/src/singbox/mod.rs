//! Typed, in-memory configuration snapshot compilation.

mod compiler;
pub(crate) mod managed_sidecar;
pub(crate) mod runtime;

pub use compiler::{ConfigCompiler, GeneratedConfig, SingBoxCompiler};
