//! 双向交接的唯一Runtime入口：先持久冻结，再真实Stop/reap，然后在业务gate内封存。
use super::*;
use crate::domain::{SnapshotVersion, StateEpoch};
impl<P: SidecarPort> ManualRuntime<P> {
    pub fn handoff_state(&self) -> Result<AppState, RuntimeError> {
        self.snapshots
            .snapshot()
            .map_err(|_| RuntimeError::StateUnavailable)
    }
    /// 交接前的双向业务预检；只读查询仍可查看被保留的pending/fence。
    pub fn preflight_transfer(&self, expected: &SnapshotVersion) -> Result<AppState, RuntimeError> {
        if self
            .snapshots
            .remote_selection_fence()
            .map_err(|_| RuntimeError::StateUnavailable)?
            .is_some()
        {
            return Err(RuntimeError::SelectionPending);
        }
        let state = self.handoff_state()?;
        if state.version() != *expected || !Self::pending_in(&state).is_empty() {
            return Err(RuntimeError::SelectionPending);
        }
        Ok(state)
    }

    pub fn owner_transfer(&self) -> Result<Option<OwnerTransfer>, RuntimeError> {
        self.store()?
            .owner_transfer()
            .map_err(RuntimeError::RecoveryUnavailable)
    }
    /// 首次bootstrap的Desktop半边：持久冻结并返回可重查记录，不授予helper启动权。
    /// installation仅绑定候选安装世代；helper仍必须独立从root记录核验，不能当授权token。
    pub fn freeze_first_bootstrap(
        &mut self,
        id: StateEpoch,
        installation: StateEpoch,
        expected: SnapshotVersion,
    ) -> Result<OwnerTransfer, RuntimeError> {
        let state = self.preflight_transfer(&expected)?;
        crate::application::helper_protocol::Configuration {
            state: Box::new(state),
        }
        .validate()
        .map_err(|_| RuntimeError::CompileFailed)?;
        if self.sidecar.active_identity().is_some()
            || self.sidecar.snapshot().lifecycle != SidecarLifecycle::Stopped
            || self.facts.last_successful.is_some()
        {
            return Err(RuntimeError::RecoveryPreparationFailed);
        }
        let record = OwnerTransfer::BootstrapFrozen {
            id,
            installation,
            version: expected.clone(),
        };
        self.snapshots
            .with_runtime_version(&expected, || {
                self.store()?
                    .freeze_bootstrap(&self.nonce, &record)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)??;
        // freeze已落盘后发布OS会话绑定；失败保持冻结，同请求可补写，绝不解除。
        #[cfg(target_os = "macos")]
        self.publish_source_session()?;
        Ok(record)
    }
    /// 只读业务预检；必须在连接目标和任何freeze/Stop之前执行，执行阶段再次检查。
    pub fn preflight_handoff(&self, expected: &SnapshotVersion) -> Result<(), RuntimeError> {
        self.preflight_transfer(expected)?;
        // 持久绑定本Runtime生命周期；崩溃后的空Port不能替代旧writer的wait/reap证据。
        // 无历史成功材料先拒绝，干净首次启动不会被无谓冻结。
        let before = self.snapshot()?;
        if before.runtime.last_successful_version.is_none() {
            return Err(RuntimeError::RecoveryUnavailable(RecoveryError::Missing));
        }
        if before
            .runtime
            .last_successful_version
            .as_ref()
            .is_none_or(|v| v.0.epoch != expected.config.0.epoch)
            || before.runtime.last_successful_selection_version.as_ref()
                != Some(&expected.selection)
        {
            return Err(RuntimeError::SelectionPending);
        }
        if before.runtime.applied_version.is_some()
            && (before.runtime.applied_version != before.runtime.last_successful_version
                || before.confirmed_selection_version
                    != before.runtime.last_successful_selection_version)
        {
            return Err(RuntimeError::RecoveryRecordFailed);
        }
        Ok(())
    }
    /// 在远端只读预检之前发布当前进程绑定，不冻结owner、不Stop或改业务状态。
    #[cfg(target_os = "macos")]
    pub fn publish_source_session(&self) -> Result<(), RuntimeError> {
        use crate::application::runtime_recovery::SourceSession;
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let pid = unsafe { libc::getpid() };
        let size = std::mem::size_of_val(&info) as i32;
        if unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut info as *mut libc::proc_bsdinfo).cast(),
                size,
            )
        } != size
        {
            return Err(RuntimeError::StateUnavailable);
        }
        let session = SourceSession {
            incarnation: self.nonce.clone(),
            uid: unsafe { libc::geteuid() },
            pid,
            start: (info.pbi_start_tvsec, info.pbi_start_tvusec),
        };
        self.store()?
            .source_session(
                &serde_json::to_vec(&session).map_err(|_| RuntimeError::StateUnavailable)?,
            )
            .map_err(RuntimeError::RecoveryUnavailable)
    }
    pub fn prepare_handoff(
        &mut self,
        id: StateEpoch,
        expected: SnapshotVersion,
    ) -> Result<(TransferTicket, ClosedBundle), RuntimeError> {
        self.preflight_handoff(&expected)?;
        self.store()?
            .bind_owner_incarnation(&self.nonce)
            .map_err(RuntimeError::RecoveryUnavailable)?;
        if let Some(OwnerTransfer::Closed { ticket } | OwnerTransfer::Released { ticket }) =
            self.owner_transfer()?
        {
            if ticket.id != id || ticket.version != expected {
                return Err(RuntimeError::RecoveryPreparationFailed);
            }
            let bundle = self
                .store()?
                .closed_transfer(&ticket)
                .map_err(RuntimeError::RecoveryUnavailable)?;
            return Ok((ticket, bundle));
        }
        self.store()?
            .freeze_owner(id.clone(), expected.clone())
            .map_err(RuntimeError::RecoveryUnavailable)?;
        self.execute(RuntimeCommand::Stop, |_| {})?;
        if self.sidecar.snapshot().lifecycle != SidecarLifecycle::Stopped
            || self.sidecar.active_identity().is_some()
        {
            return Err(RuntimeError::StopFailed);
        }
        let resources = self.resources(&expected.config.0.epoch)?;
        self.snapshots
            .with_runtime_version(&expected, || {
                self.store()?
                    .close_transfer(id, expected.clone(), &resources)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)?
    }
    /// 仅当前Runtime的成功Stop之后封存；不修改owner-transfer，也不授权另一个owner。
    pub fn clean_stop_material(
        &self,
        expected: &SnapshotVersion,
    ) -> Result<ClosedBundle, RuntimeError> {
        self.preflight_transfer(expected)?;
        if self.sidecar.snapshot().lifecycle != SidecarLifecycle::Stopped
            || self.sidecar.active_identity().is_some()
            || self
                .facts
                .last_successful
                .as_ref()
                .is_none_or(|f| f.config != expected.config || f.selection != expected.selection)
        {
            return Err(RuntimeError::StopFailed);
        }
        let resources = self.resources(&expected.config.0.epoch)?;
        self.snapshots
            .with_runtime_version(expected, || {
                self.store()?
                    .closed_material(expected.clone(), &resources)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)?
    }
    pub fn release_handoff(&self, ticket: &TransferTicket) -> Result<OwnerTransfer, RuntimeError> {
        self.store()?
            .verify_owner_incarnation(&self.nonce)
            .map_err(RuntimeError::RecoveryUnavailable)?;
        self.snapshots
            .with_runtime_version(&ticket.version, || {
                self.store()?
                    .release_owner(ticket)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)?
    }
    pub fn receive_handoff(
        &mut self,
        ticket: &TransferTicket,
        bundle: &ClosedBundle,
    ) -> Result<(), RuntimeError> {
        if self.owner_transfer()?
            == Some(OwnerTransfer::Local {
                ticket: ticket.clone(),
            })
        {
            self.store()?
                .verify_owner_incarnation(&self.nonce)
                .map_err(RuntimeError::RecoveryUnavailable)?;
            if bundle
                .ticket(ticket.id.clone())
                .map_err(RuntimeError::RecoveryUnavailable)?
                != *ticket
            {
                return Err(RuntimeError::RecoveryUnavailable(
                    RecoveryError::DigestMismatch,
                ));
            }
            return Ok(());
        }
        if self.sidecar.active_identity().is_some()
            || self.sidecar.snapshot().lifecycle != SidecarLifecycle::Stopped
        {
            return Err(RuntimeError::StopFailed);
        }
        let resources = self.resources(&ticket.version.config.0.epoch)?;
        self.snapshots
            .with_runtime_version(&ticket.version, || {
                bundle
                    .validate(&resources)
                    .map_err(RuntimeError::RecoveryUnavailable)?;
                if bundle
                    .ticket(ticket.id.clone())
                    .map_err(RuntimeError::RecoveryUnavailable)?
                    != *ticket
                {
                    return Err(RuntimeError::RecoveryUnavailable(
                        RecoveryError::DigestMismatch,
                    ));
                }
                self.store()?
                    .bind_owner_incarnation(&self.nonce)
                    .map_err(RuntimeError::RecoveryUnavailable)?;
                self.store()?
                    .prepare_receive(ticket, bundle, &resources)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)?
    }
    pub fn commit_handoff(&mut self, release: &OwnerTransfer) -> Result<(), RuntimeError> {
        self.store()?
            .verify_owner_incarnation(&self.nonce)
            .map_err(RuntimeError::RecoveryUnavailable)?;
        let OwnerTransfer::Released { ticket } = release else {
            return Err(RuntimeError::RecoveryPreparationFailed);
        };
        if self.owner_transfer()?
            == Some(OwnerTransfer::Local {
                ticket: ticket.clone(),
            })
        {
            return Ok(());
        }
        if self.sidecar.active_identity().is_some()
            || self.sidecar.snapshot().lifecycle != SidecarLifecycle::Stopped
        {
            return Err(RuntimeError::StopFailed);
        }
        let resources = self.resources(&ticket.version.config.0.epoch)?;
        self.snapshots
            .with_runtime_version(&ticket.version, || {
                self.store()?
                    .commit_receive(release, &resources)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)??;
        let m = self
            .store()?
            .read_manifest()
            .map_err(RuntimeError::RecoveryUnavailable)?;
        self.facts.last_successful = Some(LastSuccessfulVersion {
            config: m.config,
            selection: m.confirmed_selection.version,
        });
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl<P: SidecarPort> ManualRuntime<P> {
    pub fn prepare_rebind(
        &self,
        previous: &crate::application::helper_protocol::BootstrapTicket,
        ticket: &crate::application::helper_protocol::BootstrapTicket,
    ) -> Result<(), RuntimeError> {
        self.preflight_transfer(&previous.version)?;
        if ticket.version != previous.version
            || ticket.installation != previous.installation
            || ticket.id == previous.id
            || self.sidecar.active_identity().is_some()
            || self.sidecar.snapshot().lifecycle != SidecarLifecycle::Stopped
            || self.owner_transfer()?
                != Some(OwnerTransfer::BootstrapFrozen {
                    id: previous.id.clone(),
                    installation: previous.installation.clone(),
                    version: previous.version.clone(),
                })
        {
            return Err(RuntimeError::RecoveryPreparationFailed);
        }
        // 新进程nonce不可覆盖旧nonce。原Frozen持续阻止Manual Start/Select。
        if self.store()?.verify_owner_incarnation(&self.nonce).is_ok() {
            return Err(RuntimeError::RecoveryPreparationFailed);
        }
        let pid = unsafe { libc::getpid() };
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of_val(&info) as i32;
        if unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut info as *mut libc::proc_bsdinfo).cast(),
                size,
            )
        } != size
        {
            return Err(RuntimeError::StateUnavailable);
        }
        let proposal = crate::application::helper_protocol::RebindPending {
            previous: previous.clone(),
            ticket: ticket.clone(),
            session: SourceSession {
                incarnation: self.nonce.clone(),
                uid: unsafe { libc::geteuid() },
                pid,
                start: (info.pbi_start_tvsec, info.pbi_start_tvusec),
            },
        };
        self.snapshots
            .with_runtime_version(&previous.version, || {
                self.store()?
                    .rebind_pending(&proposal)
                    .map_err(RuntimeError::RecoveryUnavailable)
            })
            .map_err(|_| RuntimeError::StateUnavailable)?
    }
}

#[cfg(target_os = "macos")]
impl<P: SidecarPort> ManualRuntime<P> {
    /// 只返回本worker创建的持久申请；新进程不能借旧申请nonce取得准入。
    pub fn rebind_ticket(
        &self,
    ) -> Result<Option<crate::application::helper_protocol::BootstrapTicket>, RuntimeError> {
        let Some(proposal) = self
            .store()?
            .read_rebind_pending()
            .map_err(RuntimeError::RecoveryUnavailable)?
        else {
            return Ok(None);
        };
        if proposal.session.incarnation != self.nonce
            || proposal.session.pid != unsafe { libc::getpid() }
            || proposal.session.uid != unsafe { libc::geteuid() }
            || self.owner_transfer()?
                != Some(OwnerTransfer::BootstrapFrozen {
                    id: proposal.previous.id,
                    installation: proposal.previous.installation,
                    version: proposal.previous.version,
                })
        {
            return Err(RuntimeError::RecoveryPreparationFailed);
        }
        // 这里只识别本worker凭据，允许业务版本变化后仍Stop自有child。
        // Commit与Start各自重新检查完整业务版本；不能让失败CAS阻止退出清理。
        Ok(Some(proposal.ticket))
    }
}
