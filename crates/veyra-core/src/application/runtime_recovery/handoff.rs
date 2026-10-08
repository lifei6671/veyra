//! P2-06 单份持久owner凭据与关闭缓存运输；不是请求journal，不接受调用方路径。
use super::*;
use crate::domain::SnapshotVersion;
use crate::singbox::{ProductRuntimeResources, SingBoxPlan};

pub const HANDOFF_PLAN_BYTES: usize = 1024 * 1024;
pub const HANDOFF_CACHE_BYTES: usize = 2 * 1024 * 1024;
pub const HANDOFF_WIRE_BYTES: usize = 13 * 1024 * 1024;
const RECORD: &str = "runtime/owner-transfer.json";
const BUNDLE: &str = "runtime/owner-bundle.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransferTicket {
    pub id: StateEpoch,
    pub version: SnapshotVersion,
    pub digest: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "phase", deny_unknown_fields)]
pub enum OwnerTransfer {
    /// 写入后即冻结本地Start/Select；失败也不自动取消。
    Freezing {
        id: StateEpoch,
        version: SnapshotVersion,
    },
    /// 首次合作让权的本地冻结，仅限制本地writer；不构成helper启动凭据。
    BootstrapFrozen {
        id: StateEpoch,
        installation: StateEpoch,
        version: SnapshotVersion,
    },
    Closed {
        ticket: TransferTicket,
    },
    Released {
        ticket: TransferTicket,
    },
    Prepared {
        ticket: TransferTicket,
    },
    /// 只有目标收到source Released后才可提交；启动仍需独立的check/Ready。
    Local {
        ticket: TransferTicket,
    },
}
impl OwnerTransfer {
    pub fn ticket(&self) -> Option<&TransferTicket> {
        match self {
            Self::Freezing { .. } | Self::BootstrapFrozen { .. } => None,
            Self::Closed { ticket }
            | Self::Released { ticket }
            | Self::Prepared { ticket }
            | Self::Local { ticket } => Some(ticket),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClosedBundle {
    version: SnapshotVersion,
    manifest: LastAppliedManifest,
    plan: Vec<u8>,
    cache: Vec<u8>,
}
impl std::fmt::Debug for ClosedBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ClosedBundle([redacted])")
    }
}
impl ClosedBundle {
    pub fn source_manifest(&self) -> &LastAppliedManifest {
        &self.manifest
    }
    pub fn version(&self) -> &SnapshotVersion {
        &self.version
    }
    pub fn ticket(&self, id: StateEpoch) -> Result<TransferTicket, RecoveryError> {
        if self.plan.len() > HANDOFF_PLAN_BYTES || self.cache.len() > HANDOFF_CACHE_BYTES {
            return Err(RecoveryError::Corrupt);
        }
        let bytes = serde_json::to_vec(self).map_err(|_| RecoveryError::Corrupt)?;
        if bytes.len() > HANDOFF_WIRE_BYTES {
            return Err(RecoveryError::Corrupt);
        }
        Ok(TransferTicket {
            id,
            version: self.version.clone(),
            digest: digest(&bytes),
        })
    }
    /// 运输只允许已确认、无pending的同epoch资料；所有引用都重新推导，绝不按payload路径写文件。
    pub fn validate(&self, resources: &ProductRuntimeResources) -> Result<(), RecoveryError> {
        let m = &self.manifest;
        if self.plan.len() > HANDOFF_PLAN_BYTES
            || self.cache.len() > HANDOFF_CACHE_BYTES
            || m.schema != 1
            || m.compiler_format != crate::singbox::RECOVERY_FORMAT
            || m.recovery_format != crate::singbox::RECOVERY_FORMAT
            || m.kernel_version != KERNEL_VERSION
            || m.kernel_digest != KERNEL_DIGEST
            || m.state_epoch != self.version.config.0.epoch
            || m.state_epoch != self.version.selection.0.epoch
            || m.config.0.epoch != m.state_epoch
            || m.config.0.revision > self.version.config.0.revision
            || m.confirmed_selection.version != self.version.selection
            || m.selection_at_apply.0.epoch != m.state_epoch
            || m.plan_selection.0.epoch != m.state_epoch
            || m.plan_selection.0.revision > m.selection_at_apply.0.revision
            || m.selection_at_apply.0.revision > m.confirmed_selection.version.0.revision
            || m.cache_generation != cache_generation(&m.state_epoch)
            || !m.resources.is_empty()
            || m.plan.path != format!("runtime/plan-{}.json", digest(&self.plan))
            || m.plan.digest != digest(&self.plan)
            || m.cache.as_ref()
                != Some(&ArtifactRef {
                    path: format!("kernel-cache/rollback-{}.db", digest(&self.cache)),
                    digest: digest(&self.cache),
                })
        {
            return Err(RecoveryError::Corrupt);
        }
        let plan =
            SingBoxPlan::recover(&self.plan, resources).map_err(|_| RecoveryError::Corrupt)?;
        let index = plan.artifact_index().ok_or(RecoveryError::Corrupt)?;
        if index.config != m.config
            || index.selection != m.plan_selection
            || index.pools.len() != m.confirmed_selection.nodes.len()
            || m.confirmed_selection.nodes.iter().any(|(id, node)| {
                index
                    .pools
                    .get(id)
                    .is_none_or(|p| !p.members.contains_key(node))
            })
        {
            return Err(RecoveryError::Corrupt);
        }
        Ok(())
    }
}
impl RecoveryStore {
    /// 崩溃后只读查询，不chmod目录、不创建文件、不推断child已退出。
    pub fn inspect_owner_transfer(
        root: &std::path::Path,
    ) -> Result<Option<OwnerTransfer>, RecoveryError> {
        let m = std::fs::symlink_metadata(root).map_err(|_| RecoveryError::UnsafePath)?;
        if !m.is_dir() || m.file_type().is_symlink() {
            return Err(RecoveryError::UnsafePath);
        }
        #[cfg(unix)]
        if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
            return Err(RecoveryError::UnsafePath);
        }
        let store = Self {
            root: root.to_path_buf(),
            #[cfg(test)]
            fail_manifest: false,
            #[cfg(test)]
            fail_copy: false,
            #[cfg(test)]
            fail_manifest_at: None,
            #[cfg(test)]
            commits: std::cell::Cell::new(0),
        };
        store.owner_transfer()
    }
    fn transfer_read(&self, relative: &str, max: usize) -> Result<Vec<u8>, RecoveryError> {
        let path = self.path(relative)?;
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        let file = options.open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RecoveryError::Missing
            } else {
                RecoveryError::UnsafePath
            }
        })?;
        let m = file.metadata().map_err(|_| RecoveryError::UnsafePath)?;
        if !m.is_file() || m.len() > max as u64 {
            return Err(RecoveryError::UnsafePath);
        }
        #[cfg(unix)]
        if m.uid() != unsafe { libc::geteuid() } || m.nlink() != 1 || m.mode() & 0o7777 != 0o600 {
            return Err(RecoveryError::UnsafePath);
        }
        let mut bytes = vec![];
        file.take(max as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| RecoveryError::Corrupt)?;
        if bytes.len() > max {
            return Err(RecoveryError::Corrupt);
        }
        Ok(bytes)
    }
    pub fn owner_transfer(&self) -> Result<Option<OwnerTransfer>, RecoveryError> {
        match self.transfer_read(RECORD, 4096) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| RecoveryError::Corrupt),
            Err(RecoveryError::Missing) => Ok(None),
            Err(e) => Err(e),
        }
    }
    pub(crate) fn verify_owner_incarnation(&self, incarnation: &str) -> Result<(), RecoveryError> {
        if self.transfer_read("runtime/owner-incarnation.json", 256)? != incarnation.as_bytes() {
            return Err(RecoveryError::UnsafePath);
        }
        Ok(())
    }
    pub(crate) fn bind_owner_incarnation(&self, incarnation: &str) -> Result<(), RecoveryError> {
        match self.transfer_read("runtime/owner-incarnation.json", 256) {
            Ok(bytes) if bytes == incarnation.as_bytes() => Ok(()),
            Ok(_) => Err(RecoveryError::UnsafePath),
            Err(RecoveryError::Missing) if self.owner_transfer()?.is_none() => {
                self.write("runtime/owner-incarnation.json", incarnation.as_bytes())
            }
            Err(e) => Err(e),
        }
    }
    pub(crate) fn local_owner_available(&self, incarnation: &str) -> Result<bool, RecoveryError> {
        let record = self.owner_transfer()?;
        match self.transfer_read("runtime/owner-incarnation.json", 256) {
            Err(RecoveryError::Missing) if record.is_none() => Ok(true),
            Ok(bytes) if bytes == incarnation.as_bytes() => {
                Ok(matches!(record, Some(OwnerTransfer::Local { .. })))
            }
            Ok(_) => Ok(false),
            Err(e) => Err(e),
        }
    }
    fn transfer_write(&self, record: &OwnerTransfer) -> Result<(), RecoveryError> {
        self.write(
            RECORD,
            &serde_json::to_vec(record).map_err(|_| RecoveryError::Corrupt)?,
        )
    }
    pub(crate) fn freeze_owner(
        &self,
        id: StateEpoch,
        version: SnapshotVersion,
    ) -> Result<(), RecoveryError> {
        match self.owner_transfer()? {
            None | Some(OwnerTransfer::Local { .. }) => {
                self.transfer_write(&OwnerTransfer::Freezing { id, version })
            }
            Some(OwnerTransfer::Freezing {
                id: old,
                version: v,
            }) if old == id && v == version => Ok(()),
            _ => Err(RecoveryError::UnsafePath),
        }
    }
    /// 只建立本地拒写事实。目录为空只用于排除已知历史，不证明机器没有旧writer。
    pub(crate) fn freeze_bootstrap(
        &self,
        incarnation: &str,
        record: &OwnerTransfer,
    ) -> Result<(), RecoveryError> {
        if !matches!(record, OwnerTransfer::BootstrapFrozen { .. }) {
            return Err(RecoveryError::UnsafePath);
        }
        if let Some(old) = self.owner_transfer()? {
            self.verify_owner_incarnation(incarnation)?;
            return if old == *record {
                Ok(())
            } else {
                Err(RecoveryError::UnsafePath)
            };
        }
        // 任何残留资料都不并入首次路径；不删除、不尝试恢复旧cache。
        for relative in ["runtime", "kernel-cache"] {
            let path = self.root.join(relative);
            match std::fs::symlink_metadata(&path) {
                Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {
                    for entry in std::fs::read_dir(path).map_err(|_| RecoveryError::UnsafePath)? {
                        let entry = entry.map_err(|_| RecoveryError::UnsafePath)?;
                        // primary listener先于业务writer创建；这里只允许固定socket形态，helper另做OS验证。
                        #[cfg(unix)]
                        if relative == "runtime" && entry.file_name() == "desktop.sock" {
                            use std::os::unix::fs::FileTypeExt;
                            let m = std::fs::symlink_metadata(entry.path())
                                .map_err(|_| RecoveryError::UnsafePath)?;
                            if !m.file_type().is_socket()
                                || m.uid() != unsafe { libc::geteuid() }
                                || m.mode() & 0o7777 != 0o600
                            {
                                return Err(RecoveryError::UnsafePath);
                            }
                            continue;
                        }
                        // RecoveryStore初始化创建的两个空目录不是历史运行材料。
                        if relative != "runtime"
                            || !["configs", "resources"]
                                .iter()
                                .any(|name| entry.file_name() == *name)
                        {
                            return Err(RecoveryError::UnsafePath);
                        }
                        let meta = std::fs::symlink_metadata(entry.path())
                            .map_err(|_| RecoveryError::UnsafePath)?;
                        if !meta.is_dir()
                            || meta.file_type().is_symlink()
                            || std::fs::read_dir(entry.path())
                                .map_err(|_| RecoveryError::UnsafePath)?
                                .next()
                                .is_some()
                        {
                            return Err(RecoveryError::UnsafePath);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(RecoveryError::UnsafePath),
            }
        }
        // 若后一写入失败，incarnation依旧阻止本地Start，不能自动清掉再试。
        self.bind_owner_incarnation(incarnation)?;
        self.transfer_write(record)
    }
    /// 仅ManualRuntime在成功Stop/reap且版本/pending重新核验后调用。
    pub(crate) fn close_transfer(
        &self,
        id: StateEpoch,
        version: SnapshotVersion,
        resources: &ProductRuntimeResources,
    ) -> Result<(TransferTicket, ClosedBundle), RecoveryError> {
        if self.owner_transfer()?
            != Some(OwnerTransfer::Freezing {
                id: id.clone(),
                version: version.clone(),
            })
        {
            return Err(RecoveryError::UnsafePath);
        }
        let bundle = self.closed_material(version, resources)?;
        let ticket = bundle.ticket(id)?;
        self.write(
            BUNDLE,
            &serde_json::to_vec(&bundle).map_err(|_| RecoveryError::Corrupt)?,
        )?;
        self.transfer_write(&OwnerTransfer::Closed {
            ticket: ticket.clone(),
        })?;
        Ok((ticket, bundle))
    }
    /// 只读关闭材料；调用方仍须持有真实Runtime并确认Stop/reap，不能作为无writer证明。
    pub(crate) fn closed_material(
        &self,
        version: SnapshotVersion,
        resources: &ProductRuntimeResources,
    ) -> Result<ClosedBundle, RecoveryError> {
        let manifest: LastAppliedManifest = serde_json::from_slice(
            &self.transfer_read("runtime/last-applied.json", HANDOFF_PLAN_BYTES)?,
        )
        .map_err(|_| RecoveryError::Corrupt)?;
        let plan = self.transfer_read(&manifest.plan.path, HANDOFF_PLAN_BYTES)?;
        let cache = self.transfer_read(
            &manifest
                .cache
                .as_ref()
                .ok_or(RecoveryError::CacheCopyFailed)?
                .path,
            HANDOFF_CACHE_BYTES,
        )?;
        // 最后一次关闭快照必须与当前live inode内容一致，不能发送上一次成功的旧cache。
        let live = self.transfer_read(
            &format!("kernel-cache/{}.db", manifest.cache_generation),
            HANDOFF_CACHE_BYTES,
        )?;
        if live != cache {
            return Err(RecoveryError::DigestMismatch);
        }
        let bundle = ClosedBundle {
            version,
            manifest,
            plan,
            cache,
        };
        bundle.validate(resources)?;
        Ok(bundle)
    }
    pub(crate) fn closed_transfer(
        &self,
        ticket: &TransferTicket,
    ) -> Result<ClosedBundle, RecoveryError> {
        if self
            .owner_transfer()?
            .as_ref()
            .and_then(OwnerTransfer::ticket)
            != Some(ticket)
        {
            return Err(RecoveryError::UnsafePath);
        }
        let bundle: ClosedBundle =
            serde_json::from_slice(&self.transfer_read(BUNDLE, HANDOFF_WIRE_BYTES)?)
                .map_err(|_| RecoveryError::Corrupt)?;
        if bundle.ticket(ticket.id.clone())? != *ticket {
            return Err(RecoveryError::DigestMismatch);
        }
        Ok(bundle)
    }
    pub(crate) fn release_owner(
        &self,
        ticket: &TransferTicket,
    ) -> Result<OwnerTransfer, RecoveryError> {
        match self.owner_transfer()? {
            Some(OwnerTransfer::Closed { ticket: t } | OwnerTransfer::Released { ticket: t })
                if t == *ticket =>
            {
                self.closed_transfer(ticket)?;
                let record = OwnerTransfer::Released {
                    ticket: ticket.clone(),
                };
                self.transfer_write(&record)?;
                Ok(record)
            }
            _ => Err(RecoveryError::UnsafePath),
        }
    }
    /// 目标仅暂存并冻结；重试同ticket幂等，不触及旧manifest/cache，不创建child。
    pub(crate) fn prepare_receive(
        &self,
        ticket: &TransferTicket,
        bundle: &ClosedBundle,
        resources: &ProductRuntimeResources,
    ) -> Result<(), RecoveryError> {
        bundle.validate(resources)?;
        if bundle.ticket(ticket.id.clone())? != *ticket {
            return Err(RecoveryError::DigestMismatch);
        }
        match self.owner_transfer()? {
            Some(OwnerTransfer::Local { ticket: t }) if t == *ticket => return Ok(()),
            Some(OwnerTransfer::Prepared { ticket: t }) if t == *ticket => {}
            None if matches!(self.read_manifest(), Err(RecoveryError::Missing)) => {}
            Some(OwnerTransfer::Released { ticket: old })
                if old.id != ticket.id
                    && old.version.config.0.epoch == ticket.version.config.0.epoch
                    && old.version.config.0.revision <= ticket.version.config.0.revision
                    && old.version.selection.0.revision <= ticket.version.selection.0.revision => {}
            _ => return Err(RecoveryError::UnsafePath),
        }
        // 先写冻结标志，断电/写bundle失败也不能恢复旧owner。
        self.transfer_write(&OwnerTransfer::Prepared {
            ticket: ticket.clone(),
        })?;
        self.write(
            BUNDLE,
            &serde_json::to_vec(bundle).map_err(|_| RecoveryError::Corrupt)?,
        )
    }
    /// source Released是经已验证OS session收到的封闭凭据；调用方还须禁止并行writer。
    pub(crate) fn commit_receive(
        &self,
        release: &OwnerTransfer,
        resources: &ProductRuntimeResources,
    ) -> Result<(), RecoveryError> {
        let OwnerTransfer::Released { ticket } = release else {
            return Err(RecoveryError::UnsafePath);
        };
        match self.owner_transfer()? {
            Some(OwnerTransfer::Local { ticket: t }) if t == *ticket => return Ok(()),
            Some(OwnerTransfer::Prepared { ticket: t }) if t == *ticket => {}
            _ => return Err(RecoveryError::UnsafePath),
        }
        let bundle = self.closed_transfer(ticket)?;
        bundle.validate(resources)?;
        self.save_plan(&bundle.plan)?;
        let cache = bundle
            .manifest
            .cache
            .as_ref()
            .ok_or(RecoveryError::Corrupt)?;
        self.write(&cache.path, &bundle.cache)?;
        self.commit(&bundle.manifest)?;
        // 不写live cache，更不自动启动；RestoreLastSuccessful必须重新check/Ready。
        self.transfer_write(&OwnerTransfer::Local {
            ticket: ticket.clone(),
        })
    }
}

/// 文件记录只绑定当前OS会话与Runtime nonce，本身不构成OS退出证明。
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SourceSession {
    pub incarnation: String,
    pub uid: u32,
    pub pid: i32,
    pub start: (u64, u64),
}
impl RecoveryStore {
    pub(crate) fn source_session(&self, bytes: &[u8]) -> Result<(), RecoveryError> {
        self.write("runtime/source-session.json", bytes)
    }
}

impl RecoveryStore {
    pub(crate) fn rebind_pending(
        &self,
        proposal: &crate::application::helper_protocol::RebindPending,
    ) -> Result<(), RecoveryError> {
        let path = "runtime/bootstrap-rebind.json";
        match self.transfer_read(path, 8192) {
            Ok(bytes) => {
                let old: crate::application::helper_protocol::RebindPending =
                    serde_json::from_slice(&bytes).map_err(|_| RecoveryError::Corrupt)?;
                return if old == *proposal {
                    Ok(())
                } else {
                    Err(RecoveryError::UnsafePath)
                };
            }
            Err(RecoveryError::Missing) => {}
            Err(e) => return Err(e),
        }
        // 失败保留原Frozen；此申请永不覆盖旧owner/session/incarnation。
        self.write(
            path,
            &serde_json::to_vec(proposal).map_err(|_| RecoveryError::Corrupt)?,
        )
    }
}

impl RecoveryStore {
    pub(crate) fn read_rebind_pending(
        &self,
    ) -> Result<Option<crate::application::helper_protocol::RebindPending>, RecoveryError> {
        match self.transfer_read("runtime/bootstrap-rebind.json", 8192) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| RecoveryError::Corrupt),
            Err(RecoveryError::Missing) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
