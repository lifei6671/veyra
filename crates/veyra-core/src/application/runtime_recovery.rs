//! Runtime owner 的私有恢复材料。业务 state、内核 cache 和恢复记录各自保持归属。
use crate::{
    domain::{ConfigVersion, NodeId, PoolId, SelectionVersion, StateEpoch},
    singbox::RECOVERY_FORMAT,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

/// 本地受控可执行文件名；上游版本/协议及原始字节摘要保持不变。
pub const KERNEL_EXECUTABLE: &str = "veyra-sing-box";
pub const KERNEL_VERSION: &str = "1.14.0";
pub const KERNEL_DIGEST: &str = "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4";
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryError {
    Missing,
    Corrupt,
    EpochMismatch,
    KernelMismatch,
    CacheGenerationMismatch,
    ResourceMissing,
    DigestMismatch,
    UnsafePath,
    WriteFailed,
    CacheCopyFailed,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecoveryAvailability {
    Available,
    Unavailable(RecoveryError),
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub path: String,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ConfirmedSelection {
    pub version: SelectionVersion,
    pub nodes: BTreeMap<PoolId, NodeId>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub groups: BTreeMap<PoolId, crate::domain::OutboundId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LastAppliedManifest {
    pub schema: u32,
    pub state_epoch: StateEpoch,
    pub config: ConfigVersion,
    /// 配置应用时的版本固定；选择记录可原子轻量前进，不重写 plan。
    pub selection_at_apply: SelectionVersion,
    pub plan_selection: SelectionVersion,
    pub confirmed_selection: ConfirmedSelection,
    pub kernel_version: String,
    pub kernel_digest: String,
    pub compiler_format: u32,
    pub recovery_format: u32,
    pub plan: ArtifactRef,
    pub config_digest: String,
    pub cache_generation: String,
    /// None 明确表示无缓存的 cold-start 基线；有引用则缺失/损坏必须拒绝恢复。
    pub cache: Option<ArtifactRef>,
    pub resources: Vec<ArtifactRef>,
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn cache_generation(epoch: &StateEpoch) -> String {
    format!(
        "v{KERNEL_VERSION}-{}",
        digest(&serde_json::to_vec(&(KERNEL_DIGEST, epoch)).expect("typed epoch"))
    )
}
/// 所有路径由 Runtime composition root 和固定的 adapter 命名生成，UI 无路径入口。
pub struct RecoveryStore {
    root: PathBuf,
    #[cfg(test)]
    pub fail_manifest: bool,
    #[cfg(test)]
    pub fail_copy: bool,
    #[cfg(test)]
    pub fail_manifest_at: Option<usize>,
    #[cfg(test)]
    pub commits: std::cell::Cell<usize>,
}
impl RecoveryStore {
    pub fn new(root: &Path) -> Result<Self, RecoveryError> {
        if !root.is_absolute()
            || root
                .components()
                .any(|v| matches!(v, std::path::Component::ParentDir))
        {
            return Err(RecoveryError::UnsafePath);
        }
        if fs::symlink_metadata(root).is_ok_and(|m| m.file_type().is_symlink()) {
            return Err(RecoveryError::UnsafePath);
        }
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        builder.mode(0o700);
        builder
            .create(root)
            .map_err(|_| RecoveryError::WriteFailed)?;
        let root = root.canonicalize().map_err(|_| RecoveryError::UnsafePath)?;
        #[cfg(unix)]
        {
            if fs::metadata(&root)
                .map_err(|_| RecoveryError::UnsafePath)?
                .uid()
                != unsafe { libc::geteuid() }
            {
                return Err(RecoveryError::UnsafePath);
            }
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
                .map_err(|_| RecoveryError::WriteFailed)?;
        }
        let result = Self {
            root,
            #[cfg(test)]
            fail_manifest: false,
            #[cfg(test)]
            fail_copy: false,
            #[cfg(test)]
            fail_manifest_at: None,
            #[cfg(test)]
            commits: std::cell::Cell::new(0),
        };
        for directory in [
            "runtime",
            "runtime/configs",
            "runtime/resources",
            "kernel-cache",
        ] {
            result.directory(directory)?;
        }
        Ok(result)
    }
    fn directory(&self, relative: &str) -> Result<(), RecoveryError> {
        let path = self.root.join(relative);
        match fs::symlink_metadata(&path) {
            Ok(m) if !m.is_dir() || m.file_type().is_symlink() => {
                return Err(RecoveryError::UnsafePath);
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut builder = fs::DirBuilder::new();
                #[cfg(unix)]
                builder.mode(0o700);
                builder
                    .create(&path)
                    .map_err(|_| RecoveryError::WriteFailed)?;
            }
            Err(_) => return Err(RecoveryError::UnsafePath),
        }
        #[cfg(unix)]
        {
            if fs::metadata(&path)
                .map_err(|_| RecoveryError::UnsafePath)?
                .uid()
                != unsafe { libc::geteuid() }
            {
                return Err(RecoveryError::UnsafePath);
            }
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .map_err(|_| RecoveryError::WriteFailed)?;
        }
        Ok(())
    }
    fn path(&self, relative: &str) -> Result<PathBuf, RecoveryError> {
        let parts: Vec<_> = Path::new(relative).components().collect();
        if parts.len() != 2
            || !parts
                .iter()
                .all(|p| matches!(p, std::path::Component::Normal(_)))
            || !matches!(
                parts[0].as_os_str().to_str(),
                Some("runtime" | "kernel-cache")
            )
        {
            return Err(RecoveryError::UnsafePath);
        }
        let path = self.root.join(relative);
        let parent =
            fs::symlink_metadata(path.parent().unwrap()).map_err(|_| RecoveryError::UnsafePath)?;
        if !parent.is_dir() || parent.file_type().is_symlink() {
            return Err(RecoveryError::UnsafePath);
        }
        if let Ok(m) = fs::symlink_metadata(&path) {
            if !m.is_file() || m.file_type().is_symlink() {
                return Err(RecoveryError::UnsafePath);
            }
            #[cfg(unix)]
            if m.permissions().mode() & 0o777 != 0o600 {
                return Err(RecoveryError::UnsafePath);
            }
        }
        Ok(path)
    }
    fn read(&self, relative: &str) -> Result<Vec<u8>, RecoveryError> {
        let path = self.path(relative)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        let mut file = options.open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RecoveryError::ResourceMissing
            } else {
                RecoveryError::UnsafePath
            }
        })?;
        let metadata = file.metadata().map_err(|_| RecoveryError::Corrupt)?;
        if !metadata.is_file() {
            return Err(RecoveryError::UnsafePath);
        }
        #[cfg(unix)]
        if metadata.permissions().mode() & 0o777 != 0o600
            || metadata.uid() != unsafe { libc::geteuid() }
        {
            return Err(RecoveryError::UnsafePath);
        }
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)
            .map_err(|_| RecoveryError::Corrupt)?;
        Ok(bytes)
    }
    fn write(&self, relative: &str, bytes: &[u8]) -> Result<(), RecoveryError> {
        let path = self.path(relative)?;
        let mut nonce = [0u8; 16];
        getrandom::fill(&mut nonce).map_err(|_| RecoveryError::WriteFailed)?;
        let temp = path.with_extension(format!("tmp-{}", digest(&nonce)));
        let previous = path.with_extension(format!("previous-{}", digest(&nonce)));
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options
                .open(&temp)
                .map_err(|_| RecoveryError::WriteFailed)?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| RecoveryError::WriteFailed)?;
            // 替换前确认父目录可 fsync，旧记录一直保留到完整 temp 已持久化。
            let directory =
                fs::File::open(path.parent().unwrap()).map_err(|_| RecoveryError::WriteFailed)?;
            directory
                .sync_all()
                .map_err(|_| RecoveryError::WriteFailed)?;
            #[cfg(test)]
            if relative == "runtime/last-applied.json"
                && (self.fail_manifest || self.fail_manifest_at == Some(self.commits.get()))
            {
                return Err(RecoveryError::WriteFailed);
            }
            let existed = path.exists();
            if existed {
                fs::hard_link(&path, &previous).map_err(|_| RecoveryError::WriteFailed)?;
            }
            fs::rename(&temp, &path).map_err(|_| RecoveryError::WriteFailed)?;
            if directory.sync_all().is_err() {
                // rename 后 fsync 失败也恢复旧记录；证据留在 previous，不能冒称提交成功。
                if existed {
                    let _ = fs::rename(&previous, &path);
                } else {
                    let _ = fs::remove_file(&path);
                }
                let _ = directory.sync_all();
                return Err(RecoveryError::WriteFailed);
            }
            if existed {
                let _ = fs::remove_file(&previous);
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(temp);
        }
        result
    }
    /// 只清理摘要仍匹配文件名的旧受管工件；损坏材料留给诊断，不能当作无记录删除。
    pub fn prune_after_success(&self, manifest: &LastAppliedManifest) {
        for (folder, prefix, extension) in [
            ("runtime", "plan-", "json"),
            ("kernel-cache", "rollback-", "db"),
        ] {
            let Ok(entries) = fs::read_dir(self.root.join(folder)) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                let Some(hash) = name
                    .strip_prefix(prefix)
                    .and_then(|v| v.strip_suffix(&format!(".{extension}")))
                else {
                    continue;
                };
                if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
                    continue;
                }
                let relative = format!("{folder}/{name}");
                if manifest.plan.path == relative
                    || manifest.cache.as_ref().is_some_and(|r| r.path == relative)
                {
                    continue;
                }
                if self
                    .read(&relative)
                    .is_ok_and(|bytes| digest(&bytes) == hash)
                {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
    pub fn cache_path(&self, generation: &str) -> Result<PathBuf, RecoveryError> {
        if generation.is_empty()
            || !generation
                .bytes()
                .all(|v| v.is_ascii_alphanumeric() || b"-_.".contains(&v))
        {
            return Err(RecoveryError::UnsafePath);
        }
        self.path(&format!("kernel-cache/{generation}.db"))
    }
    /// 在内核首次打开前创建0600空文件；不修改已经打开的cache内容或权限。
    pub fn ensure_cache(&self, generation: &str) -> Result<(), RecoveryError> {
        let path = self.cache_path(generation)?;
        if path.exists() {
            return Ok(());
        }
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        let file = options
            .open(&path)
            .map_err(|_| RecoveryError::WriteFailed)?;
        file.sync_all().map_err(|_| RecoveryError::WriteFailed)?;
        fs::File::open(path.parent().unwrap())
            .and_then(|p| p.sync_all())
            .map_err(|_| RecoveryError::WriteFailed)
    }
    /// 显式ApplySaved修复损坏记录前保留原字节；只读取已经受管的引用，不跟随symlink。
    pub fn preserve_unavailable_record(&self) -> Result<(), RecoveryError> {
        if let Ok(bytes) = self.read("runtime/last-applied.json") {
            self.write(
                &format!("runtime/corrupt-manifest-{}.json", digest(&bytes)),
                &bytes,
            )?;
        }
        if let Ok(m) = self.read_manifest() {
            if m.plan.path.starts_with("runtime/plan-")
                && let Ok(bytes) = self.read(&m.plan.path)
            {
                self.write(
                    &format!("runtime/corrupt-plan-{}.json", digest(&bytes)),
                    &bytes,
                )?;
            }
            if let Some(c) = m.cache
                && c.path.starts_with("kernel-cache/rollback-")
                && let Ok(bytes) = self.read(&c.path)
            {
                self.write(
                    &format!("kernel-cache/corrupt-cache-{}.db", digest(&bytes)),
                    &bytes,
                )?;
            }
        }
        Ok(())
    }
    pub fn save_plan(&self, bytes: &[u8]) -> Result<ArtifactRef, RecoveryError> {
        let reference = ArtifactRef {
            path: format!("runtime/plan-{}.json", digest(bytes)),
            digest: digest(bytes),
        };
        self.write(&reference.path, bytes)?;
        Ok(reference)
    }
    pub fn commit(&self, manifest: &LastAppliedManifest) -> Result<(), RecoveryError> {
        #[cfg(test)]
        self.commits.set(self.commits.get() + 1);
        if self
            .read_manifest()
            .is_err_and(|e| e == RecoveryError::Corrupt)
        {
            let bytes = self.read("runtime/last-applied.json")?;
            self.write(
                &format!("runtime/corrupt-manifest-{}.json", digest(&bytes)),
                &bytes,
            )?;
        }
        self.write(
            "runtime/last-applied.json",
            &serde_json::to_vec(manifest).map_err(|_| RecoveryError::Corrupt)?,
        )
    }
    pub fn read_manifest(&self) -> Result<LastAppliedManifest, RecoveryError> {
        let bytes = self.read("runtime/last-applied.json").map_err(|e| {
            if e == RecoveryError::ResourceMissing {
                RecoveryError::Missing
            } else {
                e
            }
        })?;
        let m: LastAppliedManifest =
            serde_json::from_slice(&bytes).map_err(|_| RecoveryError::Corrupt)?;
        let raw: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| RecoveryError::Corrupt)?;
        if raw != serde_json::to_value(&m).map_err(|_| RecoveryError::Corrupt)? {
            return Err(RecoveryError::Corrupt);
        }
        Ok(m)
    }
    pub fn artifact(&self, reference: &ArtifactRef) -> Result<Vec<u8>, RecoveryError> {
        let bytes = self.read(&reference.path)?;
        if digest(&bytes) != reference.digest {
            return Err(RecoveryError::DigestMismatch);
        }
        Ok(bytes)
    }
    pub fn load(
        &self,
        epoch: &StateEpoch,
    ) -> Result<(LastAppliedManifest, Vec<u8>), RecoveryError> {
        let m = self.read_manifest()?;
        if m.schema != 1
            || m.compiler_format != RECOVERY_FORMAT
            || m.recovery_format != RECOVERY_FORMAT
            || m.config.0.epoch != m.state_epoch
            || m.selection_at_apply.0.epoch != m.state_epoch
            || m.plan_selection.0.epoch != m.state_epoch
            || m.plan_selection.0.revision > m.selection_at_apply.0.revision
            || m.confirmed_selection.version.0.epoch != m.state_epoch
            || m.confirmed_selection.version.0.revision < m.selection_at_apply.0.revision
        {
            return Err(RecoveryError::Corrupt);
        }
        if m.state_epoch != *epoch {
            return Err(RecoveryError::EpochMismatch);
        }
        if m.kernel_version != KERNEL_VERSION || m.kernel_digest != KERNEL_DIGEST {
            return Err(RecoveryError::KernelMismatch);
        }
        if m.cache_generation != cache_generation(epoch) {
            return Err(RecoveryError::CacheGenerationMismatch);
        }
        if !m.plan.path.starts_with("runtime/plan-") {
            return Err(RecoveryError::UnsafePath);
        }
        let plan = self.artifact(&m.plan)?;
        if let Some(cache) = &m.cache {
            if !cache.path.starts_with("kernel-cache/rollback-") {
                return Err(RecoveryError::UnsafePath);
            }
            self.artifact(cache)?;
        }
        // 当前 P2 产品不产生外部资源，禁止注入未来未支持的磁盘引用。
        if !m.resources.is_empty() {
            return Err(RecoveryError::ResourceMissing);
        }
        Ok((m, plan))
    }
    /// 只能在 Sidecar stop/reap 成功后调用；不解析数据库内部格式。
    pub fn snapshot_cache(&self, generation: &str) -> Result<ArtifactRef, RecoveryError> {
        #[cfg(test)]
        if self.fail_copy {
            return Err(RecoveryError::CacheCopyFailed);
        }
        let bytes = self
            .read(&format!("kernel-cache/{generation}.db"))
            .map_err(|_| RecoveryError::CacheCopyFailed)?;
        let reference = ArtifactRef {
            path: format!("kernel-cache/rollback-{}.db", digest(&bytes)),
            digest: digest(&bytes),
        };
        self.write(&reference.path, &bytes)
            .map_err(|_| RecoveryError::CacheCopyFailed)?;
        Ok(reference)
    }
    /// writer 已停止才允许恢复快照；candidate 无法改写只读身份的 rollback 文件。
    pub fn restore_cache(&self, manifest: &LastAppliedManifest) -> Result<(), RecoveryError> {
        let live = format!("kernel-cache/{}.db", manifest.cache_generation);
        if let Some(reference) = &manifest.cache {
            self.write(&live, &self.artifact(reference)?)
        } else {
            match fs::remove_file(self.path(&live)?) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(_) => Err(RecoveryError::CacheCopyFailed),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (RecoveryStore, LastAppliedManifest, PathBuf) {
        let state = crate::domain::AppState::empty();
        let root = std::env::temp_dir().join(format!("veyra-p204-record-{:?}", state.state_epoch));
        let store = RecoveryStore::new(&root).unwrap();
        let plan = store.save_plan(b"opaque-closed-plan-test").unwrap();
        let m = LastAppliedManifest {
            schema: 1,
            state_epoch: state.state_epoch.clone(),
            config: state.config_version(),
            selection_at_apply: state.selection_version(),
            plan_selection: state.selection_version(),
            confirmed_selection: ConfirmedSelection {
                version: state.selection_version(),
                nodes: BTreeMap::new(),
                groups: BTreeMap::new(),
            },
            kernel_version: KERNEL_VERSION.into(),
            kernel_digest: KERNEL_DIGEST.into(),
            compiler_format: RECOVERY_FORMAT,
            recovery_format: RECOVERY_FORMAT,
            plan,
            config_digest: digest(b"bound-config"),
            cache_generation: cache_generation(&state.state_epoch),
            cache: None,
            resources: vec![],
        };
        (store, m, root)
    }
    // 保护首次、二次原子发布，以及完整temp写好但rename失败时旧manifest仍可读。
    #[test]
    fn atomic_manifest_replacement_failure_keeps_old_file() {
        let (mut store, mut m, root) = fixture();
        store.commit(&m).unwrap();
        let old = std::fs::read(root.join("runtime/last-applied.json")).unwrap();
        m.config.0.revision = 12;
        store.fail_manifest = true;
        assert_eq!(store.commit(&m), Err(RecoveryError::WriteFailed));
        assert_eq!(
            std::fs::read(root.join("runtime/last-applied.json")).unwrap(),
            old
        );
        assert!(
            !std::fs::read_dir(root.join("runtime")).unwrap().any(|p| p
                .unwrap()
                .path()
                .to_string_lossy()
                .contains("tmp-"))
        );
        store.fail_manifest = false;
        store.commit(&m).unwrap();
        assert_eq!(store.read_manifest().unwrap().config.0.revision, 12);
        std::fs::remove_dir_all(root).unwrap();
    }
    // 保护密钥恢复资料的权限与受控相对引用，拒绝世界可读文件/越界路径。
    #[cfg(unix)]
    #[test]
    fn private_permissions_traversal_and_symlink_rejection() {
        let (store, mut m, root) = fixture();
        store.commit(&m).unwrap();
        assert_eq!(
            std::fs::metadata(root.join("runtime"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(root.join("runtime/last-applied.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(root.join(&m.plan.path))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        std::fs::set_permissions(
            root.join(&m.plan.path),
            std::fs::Permissions::from_mode(0o644),
        )
        .unwrap();
        assert_eq!(store.artifact(&m.plan), Err(RecoveryError::UnsafePath));
        m.plan.path = "runtime/../state.json".into();
        assert_eq!(store.artifact(&m.plan), Err(RecoveryError::UnsafePath));
        let external = root.join("external");
        std::fs::write(&external, b"untouched").unwrap();
        std::fs::remove_file(root.join("runtime/last-applied.json")).unwrap();
        std::os::unix::fs::symlink(&external, root.join("runtime/last-applied.json")).unwrap();
        assert_eq!(store.commit(&m), Err(RecoveryError::UnsafePath));
        assert_eq!(std::fs::read(&external).unwrap(), b"untouched");
        assert_eq!(
            store.read_manifest().unwrap_err(),
            RecoveryError::UnsafePath
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    // 保护cache关闭快照不可被candidate覆盖，以及缺失时不能假装冷恢复。
    #[test]
    fn cache_snapshot_digest_and_missing_material() {
        let (store, mut m, root) = fixture();
        let path = store.cache_path(&m.cache_generation).unwrap();
        store
            .write(
                &format!("kernel-cache/{}.db", m.cache_generation),
                b"old-opaque",
            )
            .unwrap();
        m.cache = Some(store.snapshot_cache(&m.cache_generation).unwrap());
        store.commit(&m).unwrap();
        std::fs::write(&path, b"candidate-opaque").unwrap();
        assert_eq!(
            store.artifact(m.cache.as_ref().unwrap()).unwrap(),
            b"old-opaque"
        );
        store.restore_cache(&m).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"old-opaque");
        std::fs::remove_file(root.join(&m.cache.unwrap().path)).unwrap();
        assert_eq!(
            store.load(&m.state_epoch).unwrap_err(),
            RecoveryError::ResourceMissing
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    // 保护损坏文件不会被自动删除，历史PID/ports字段不进入恢复模型。
    #[test]
    fn corrupt_and_old_process_fields_remain_diagnostic() {
        let (store, m, root) = fixture();
        store.commit(&m).unwrap();
        let path = root.join("runtime/last-applied.json");
        for extra in ["pid", "secret", "mixed_port", "controller_port", "ready"] {
            let mut v = serde_json::to_value(&m).unwrap();
            v[extra] = true.into();
            std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
            assert_eq!(store.read_manifest().unwrap_err(), RecoveryError::Corrupt);
            assert!(path.exists());
        }
        std::fs::write(&path, b"{broken").unwrap();
        assert_eq!(store.read_manifest().unwrap_err(), RecoveryError::Corrupt);
        assert_eq!(std::fs::read(path).unwrap(), b"{broken");
        std::fs::remove_dir_all(root).unwrap();
    }
}

mod handoff;
pub use handoff::*;
