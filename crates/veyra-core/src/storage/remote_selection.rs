//! 一份尚未解决的远程选择fence；不是历史请求journal。普通JSON writer无法跨越它。
use super::*;
use crate::domain::{NodeId, PoolId, SnapshotVersion};
use std::{
    fs::{File, OpenOptions},
    io::Read,
};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteSelection {
    pub request_id: u64,
    pub instance: u64,
    pub expected: SnapshotVersion,
    pub pool: PoolId,
    pub node: NodeId,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectionFence {
    pub request: RemoteSelection,
    pub previous: NodeId,
}
impl JsonStateStore {
    fn fence_path(&self) -> PathBuf {
        self.state_file.with_extension("selection-fence")
    }
    /// OS文件锁使“检查fence→写快照”不可与另一个进程开始远程选择交错。
    pub(super) fn writer_lock(&self) -> Result<File, StateStoreError> {
        let parent = self
            .state_file
            .parent()
            .ok_or(StateStoreError::InvalidStatePath)?;
        std::fs::create_dir_all(parent).map_err(|_| StateStoreError::WriteFailed)?;
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options
            .open(self.state_file.with_extension("writer-lock"))
            .map_err(|_| StateStoreError::WriteFenced)?;
        Self::private_file(&file)?;
        file.try_lock().map_err(|_| StateStoreError::WriteFenced)?;
        Ok(file)
    }
    fn private_file(file: &File) -> Result<(), StateStoreError> {
        let m = file.metadata().map_err(|_| StateStoreError::ReadFailed)?;
        if !m.is_file() {
            return Err(StateStoreError::WriteFenced);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o7777 != 0o600 || m.nlink() != 1
            {
                return Err(StateStoreError::WriteFenced);
            }
        }
        Ok(())
    }
    pub fn selection_fence(&self) -> Result<Option<SelectionFence>, StateStoreError> {
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = match options.open(self.fence_path()) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(StateStoreError::WriteFenced),
        };
        Self::private_file(&file)?;
        let mut bytes = Vec::new();
        file.take(16385)
            .read_to_end(&mut bytes)
            .map_err(|_| StateStoreError::ReadFailed)?;
        if bytes.len() > 16384 {
            return Err(StateStoreError::WriteFenced);
        }
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|_| StateStoreError::WriteFenced)
    }
    pub(super) fn writable(&self) -> Result<(), StateStoreError> {
        if self.selection_fence()?.is_some() {
            Err(StateStoreError::WriteFenced)
        } else {
            Ok(())
        }
    }
    pub(crate) fn lock_unfenced_writer(&self) -> Result<File, StateStoreError> {
        let lock = self.writer_lock()?;
        self.writable()?;
        Ok(lock)
    }
    fn sync_parent(&self) -> Result<(), StateStoreError> {
        File::open(
            self.state_file
                .parent()
                .ok_or(StateStoreError::InvalidStatePath)?,
        )
        .and_then(|f| f.sync_all())
        .map_err(|_| StateStoreError::WriteFailed)
    }
    pub(super) fn strict_state(&self) -> Result<AppState, StateStoreError> {
        let (state, migrated) = self.decode(&read_snapshot(&self.state_file)?)?;
        if migrated {
            return Err(StateStoreError::WriteFenced);
        }
        Ok(state)
    }
    /// 整体替换也在同一个OS writer锁下重新核对完整版本，防止等待中的旧replacement晚到。
    pub(crate) fn replace_at(
        &self,
        expected: &SnapshotVersion,
        replacement: &AppState,
    ) -> Result<(), StateStoreError> {
        let _writer = self.lock_unfenced_writer()?;
        if self.strict_state()?.version() != *expected {
            return Err(StateStoreError::RevisionConflict);
        }
        self.save_unfenced(replacement)
    }
    /// fence先持久化，才保存pending；中间崩溃后只能重试同一请求，不放行业务writer。
    pub(crate) fn begin_remote(
        &self,
        request: &RemoteSelection,
    ) -> Result<SelectionFence, StateStoreError> {
        if request.request_id == 0 {
            return Err(StateStoreError::RevisionConflict);
        }
        let _writer = self.writer_lock()?;
        let mut state = self.strict_state()?;
        let existing = self.selection_fence()?;
        if let Some(fence) = &existing {
            if fence.request != *request {
                return Err(StateStoreError::WriteFenced);
            }
            if state.version() != request.expected {
                return Ok(fence.clone());
            }
        }
        if state.version() != request.expected
            || state.pools.iter().any(|p| {
                matches!(
                    p.selection,
                    SelectionPolicy::Manual {
                        pending_node_id: Some(_),
                        ..
                    }
                )
            })
        {
            return Err(StateStoreError::RevisionConflict);
        }
        let previous = state.clone();
        let pool = state
            .pools
            .iter_mut()
            .find(|p| p.id == request.pool)
            .ok_or(StateStoreError::InvalidStoredState)?;
        let SelectionPolicy::Manual {
            selected_node_id: Some(old),
            pending_node_id,
        } = &mut pool.selection
        else {
            return Err(StateStoreError::InvalidStoredState);
        };
        let fence = SelectionFence {
            request: request.clone(),
            previous: old.clone(),
        };
        *pending_node_id = Some(request.node.clone());
        let state = state
            .revised_from(&previous)
            .map_err(|_| StateStoreError::RevisionConflict)?;
        validate_state(state.clone())?;
        if existing.is_none() {
            let bytes =
                serde_json::to_vec(&fence).map_err(|_| StateStoreError::SerializationFailed)?;
            if bytes.len() > 16384 {
                return Err(StateStoreError::WriteFenced);
            }
            atomic_replace(&self.fence_path(), &bytes)?;
            self.sync_parent()?;
        }
        self.save_unfenced(&state)?;
        self.sync_parent()?;
        Ok(fence)
    }
    /// 仅实际GET得到requested或old才可精确CAS。third/unknown保持pending和fence。
    pub(crate) fn confirm_remote(
        &self,
        fence: &SelectionFence,
        actual: &NodeId,
    ) -> Result<AppState, StateStoreError> {
        let _writer = self.writer_lock()?;
        if self.selection_fence()?.as_ref() != Some(fence) {
            return Err(StateStoreError::WriteFenced);
        }
        if actual != &fence.previous && actual != &fence.request.node {
            return Err(StateStoreError::WriteFenced);
        }
        let mut state = self.strict_state()?;
        let mut pending = fence.request.expected.clone();
        pending.selection.0.revision = pending
            .selection
            .0
            .revision
            .checked_add(1)
            .ok_or(StateStoreError::RevisionConflict)?;
        let mut confirmed = pending.clone();
        confirmed.selection.0.revision = confirmed
            .selection
            .0
            .revision
            .checked_add(1)
            .ok_or(StateStoreError::RevisionConflict)?;
        let previous = state.clone();
        let pool = state
            .pools
            .iter_mut()
            .find(|p| p.id == fence.request.pool)
            .ok_or(StateStoreError::InvalidStoredState)?;
        let SelectionPolicy::Manual {
            selected_node_id,
            pending_node_id,
        } = &mut pool.selection
        else {
            return Err(StateStoreError::InvalidStoredState);
        };
        if previous.version() == confirmed
            && pending_node_id.is_none()
            && selected_node_id.as_ref() == Some(actual)
        {
            return Ok(previous);
        }
        if previous.version() != pending
            || pending_node_id.as_ref() != Some(&fence.request.node)
            || selected_node_id.as_ref() != Some(&fence.previous)
        {
            return Err(StateStoreError::RevisionConflict);
        }
        *selected_node_id = Some(actual.clone());
        *pending_node_id = None;
        let state = state
            .revised_from(&previous)
            .map_err(|_| StateStoreError::RevisionConflict)?;
        self.save_unfenced(&state)?;
        self.sync_parent()?;
        Ok(state)
    }
    /// helper已确认其manifest提交后才允许后续业务写；失败保留fence可重复确认。
    pub(crate) fn finish_remote(
        &self,
        fence: &SelectionFence,
        confirmed: &SnapshotVersion,
    ) -> Result<(), StateStoreError> {
        let _writer = self.writer_lock()?;
        if self.selection_fence()?.as_ref() != Some(fence)
            || self.strict_state()?.version() != *confirmed
        {
            return Err(StateStoreError::WriteFenced);
        }
        std::fs::remove_file(self.fence_path()).map_err(|_| StateStoreError::WriteFailed)?;
        self.sync_parent()
    }
}
