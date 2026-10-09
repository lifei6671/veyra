//! Controlled sing-box configuration, controller API and sidecar contracts.

pub mod clash_api;
mod compiler;
pub mod runtime;
pub mod secret;

pub use compiler::{
    AppliedArtifactIndex, AppliedPool, CompileError, CompiledSharedInbounds, ConfigCompiler,
    GeneratedConfig, LoopbackListener, ManagedCacheFile, ProductCompileRequest,
    ProductRuntimeResources, RECOVERY_FORMAT, RuntimeHealthPlan, RuntimeProfile, SingBoxCompiler,
    SingBoxPlan, UnsupportedProductOption, subscription_document,
};

#[cfg(any(test, feature = "legacy-test-support"))]
pub mod test_support {
    use std::sync::Mutex;
    pub static FIXED_CLASH_API_TEST_LOCK: Mutex<()> = Mutex::new(());
}
