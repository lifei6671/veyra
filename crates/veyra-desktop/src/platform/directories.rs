use super::{PlatformError, files::private_directory};
use objc2_foundation::{
    NSSearchPathDirectory as Directory, NSSearchPathDomainMask as Domain,
    NSSearchPathForDirectoriesInDomains,
};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
pub const PRODUCT_IDENTIFIER: &str = "me.disign.veyra";
pub const PREVIEW_NAMESPACE: &str = "me.disign.veyra.gpui-preview";
#[derive(Clone, Debug)]
pub struct AppDirectories {
    pub application_support: PathBuf,
    pub caches: PathBuf,
    pub logs: PathBuf,
}
fn search(directory: Directory) -> Result<PathBuf, PlatformError> {
    let paths = NSSearchPathForDirectoriesInDomains(directory, Domain::UserDomainMask, true);
    paths
        .firstObject()
        .map(|s| PathBuf::from(s.to_string()))
        .ok_or(PlatformError::FileReadFailed)
}
impl AppDirectories {
    pub fn resolve() -> Result<Self, PlatformError> {
        if let Some(root) = std::env::var_os("VEYRA_DESKTOP_ROOT_OVERRIDE")
            .or_else(|| std::env::var_os("VEYRA_DESKTOP_STATE_ROOT"))
        {
            return Ok(Self::injected(PathBuf::from(root)));
        }
        Self::from_roots(
            search(Directory::ApplicationSupportDirectory)?,
            search(Directory::CachesDirectory)?,
            search(Directory::LibraryDirectory)?.join("Logs"),
        )
    }
    fn from_roots(support: PathBuf, cache: PathBuf, logs: PathBuf) -> Result<Self, PlatformError> {
        Ok(Self {
            application_support: support.join(PREVIEW_NAMESPACE),
            caches: cache.join(PREVIEW_NAMESPACE),
            logs: logs.join(PREVIEW_NAMESPACE),
        })
    }
    pub fn injected(root: PathBuf) -> Self {
        Self {
            caches: root.join("caches"),
            logs: root.join("logs"),
            application_support: root,
        }
    }
    pub fn state(&self) -> PathBuf {
        self.application_support.join("state.json")
    }
    pub fn assets(&self) -> PathBuf {
        self.application_support.join("assets")
    }
    pub fn socket(&self) -> PathBuf {
        self.application_support.join("runtime/desktop.sock")
    }
    pub fn lock(&self) -> PathBuf {
        self.application_support.join("locks/desktop.lock")
    }
    pub fn prepare_ownership(&self) -> Result<(), PlatformError> {
        private_directory(&self.application_support)?;
        private_directory(&self.application_support.join("locks"))?;
        private_directory(&self.application_support.join("runtime"))
    }
    /// Only the primary calls this. Empty reserved directories, no Runtime manifests invented.
    pub fn prepare_primary(&self) -> Result<(), PlatformError> {
        for p in [
            self.assets(),
            self.caches.clone(),
            self.logs.clone(),
            self.application_support.join("rulesets"),
            self.application_support.join("runtime/configs"),
            self.application_support.join("runtime/resources"),
            self.application_support.join("kernel-cache"),
        ] {
            private_directory(&p)?;
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn support_path(&self, relative: &str) -> PathBuf {
        self.application_support.join(relative)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn foundation_resolves_standard_roots_without_reading_legacy_data() {
        let support = search(Directory::ApplicationSupportDirectory).unwrap();
        let caches = search(Directory::CachesDirectory).unwrap();
        let logs = search(Directory::LibraryDirectory).unwrap().join("Logs");
        assert!(support.ends_with("Library/Application Support"));
        assert!(caches.ends_with("Library/Caches"));
        assert!(logs.ends_with("Library/Logs"));
        let d = AppDirectories::from_roots(support, caches, logs).unwrap();
        assert!(d.application_support.ends_with(PREVIEW_NAMESPACE));
        // Resolution is path-only; neither legacy nor preview contents are opened here.
    }
    #[test]
    fn namespace_and_state_layout_are_separate_from_legacy_and_evictable_cache() {
        // 新产品身份不复用旧用户namespace；此测试只解析路径，不迁移/读取任何用户数据。
        assert_eq!(PRODUCT_IDENTIFIER, "me.disign.veyra");
        assert_eq!(PREVIEW_NAMESPACE, "me.disign.veyra.gpui-preview");
        let d = AppDirectories::from_roots(
            Path::new("/test/Library/Application Support").into(),
            Path::new("/test/Library/Caches").into(),
            Path::new("/test/Library/Logs").into(),
        )
        .unwrap();
        assert_ne!(PRODUCT_IDENTIFIER, PREVIEW_NAMESPACE);
        assert!(PREVIEW_NAMESPACE.starts_with(PRODUCT_IDENTIFIER));
        assert_ne!(
            d.application_support,
            Path::new("/test/Library/Application Support").join(PRODUCT_IDENTIFIER)
        );
        assert_eq!(
            d.caches,
            Path::new("/test/Library/Caches").join(PREVIEW_NAMESPACE)
        );
        assert_eq!(
            d.logs,
            Path::new("/test/Library/Logs").join(PREVIEW_NAMESPACE)
        );
        assert!(!d.support_path("kernel-cache").starts_with(&d.caches));
        assert!(
            d.support_path("runtime/last-applied.json")
                .starts_with(&d.application_support)
        );
    }
}
