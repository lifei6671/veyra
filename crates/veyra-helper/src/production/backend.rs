//! 真正执行的 Runtime Backend。复用 P2-04 生命周期与恢复记录，不自建 Ready 状态机。
use super::{assets::Assets, host::Backend, process::ProcessPort, *};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use veyra_core::{
    application::{
        manual_runtime::{ManualRuntime, RuntimeCommand as LocalCommand},
        runtime_snapshot::RuntimeStatus,
        state_access::StateAccessGate,
        state_service::SnapshotService,
    },
    domain::SnapshotVersion,
    storage::{JsonStateStore, StateStore},
};

pub(super) struct InstalledBackend {
    bootstrap: Option<super::bootstrap::Prepared>,
    session: Option<super::transport::Peer>,
    runtime: Option<ManualRuntime<ProcessPort>>,
    store: Option<JsonStateStore>,
    assets: Option<Assets>,
    unavailable: Option<Error>,
    deadline: Arc<Mutex<Instant>>,
    wire_instance: Option<(String, u64)>,
    status: Status,
    restore_transferred: bool,
    root: Option<std::path::PathBuf>,
    selection: Option<RemoteSelectionReply>,
    admin_root: Option<std::path::PathBuf>,
    draining: bool,
    requires_source: bool,
    source_failed: bool,
    source: Option<super::source::Witness>,
    accepted: Option<veyra_core::application::runtime_recovery::TransferTicket>,
    #[cfg(test)]
    source_root: Option<std::path::PathBuf>,
    #[cfg(test)]
    source_exclude: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
}
impl InstalledBackend {
    pub fn new(uid: u32, gid: u32) -> Self {
        let assets = Assets::installed(uid, gid);
        let mut backend = Self::from_assets(assets);
        backend.requires_source = true;
        backend.admin_root = Some(super::transport::ROOT.into());
        backend
    }
    pub(super) fn from_assets(assets: Result<Assets, Error>) -> Self {
        let (assets, unavailable) = match assets {
            Ok(a) => (Some(a), None),
            Err(e) => (None, Some(e)),
        };
        let root = assets.as_ref().map(|a| a.root.clone());
        let status = Status {
            // 不完整/旧安装身份也属于未知归属，不能因尚无Runtime槽就宣称干净Stopped。
            recovery_required: matches!(
                unavailable,
                Some(Error::HandoffRequired | Error::UnsafeEndpoint)
            ) || assets.as_ref().is_some_and(|a| a.pristine_root().is_err()),
            ..Status::default()
        };
        Self {
            bootstrap: None,
            root,
            admin_root: None,
            draining: false,
            requires_source: false,
            source_failed: false,
            source: None,
            accepted: None,
            #[cfg(test)]
            source_root: None,
            #[cfg(test)]
            source_exclude: assets
                .as_ref()
                .map(|a| a.spawned.clone())
                .unwrap_or_default(),
            selection: None,
            session: None,
            runtime: None,
            store: None,
            assets,
            unavailable,
            deadline: Arc::new(Mutex::new(Instant::now())),
            wire_instance: None,
            status,
            restore_transferred: false,
        }
    }
    #[cfg(test)]
    pub(super) fn take_test_runtime(&mut self) -> ManualRuntime<ProcessPort> {
        self.runtime.take().expect("test initialized")
    }
    #[cfg(test)]
    pub(super) fn test_admin_root(&mut self, root: std::path::PathBuf) {
        self.admin_root = Some(root);
    }
    /// 同一Runtime worker执行管理员停止；不借用root作为业务Peer，不使用外来PID。
    fn admin_barrier(&mut self) -> Result<(), Error> {
        let Some(root) = self.admin_root.clone() else {
            return Ok(());
        };
        if !self.draining && !super::admin::requested(&root)? {
            return Ok(());
        }
        self.draining = true;
        *self.deadline.lock().expect("operation deadline") =
            Instant::now() + std::time::Duration::from_secs(30);
        if super::admin::acknowledged(&root)? {
            return Err(Error::HandoffRequired);
        }
        if let Some(runtime) = self.runtime.as_mut() {
            // 未确认选择可能已PUT，必须先停止自有child，但不能凭Stop清pending/签发ACK。
            if self
                .selection
                .as_ref()
                .is_some_and(|slot| slot.confirmed.is_none())
            {
                runtime
                    .execute(LocalCommand::Stop, |_| {})
                    .map_err(|_| Error::Failed)?;
                return Err(Error::HandoffRequired);
            }
            let state = runtime.handoff_state().map_err(|_| Error::Unavailable)?;
            let id = match runtime.owner_transfer().map_err(|_| Error::Unavailable)? {
                Some(veyra_core::application::runtime_recovery::OwnerTransfer::Closed {
                    ticket,
                }) => ticket.id,
                None
                | Some(veyra_core::application::runtime_recovery::OwnerTransfer::Local {
                    ..
                }) => veyra_core::domain::StateEpoch::fresh().map_err(|_| Error::Unavailable)?,
                _ => return Err(Error::HandoffRequired),
            };
            // 复用P2-04真实Stop/reap、关闭cache摘要和manifest一致性，不新建清理语义。
            runtime
                .prepare_handoff(id, state.version())
                .map_err(|_| Error::HandoffRequired)?;
        } else {
            // 没有Child handle绝不等于旧writer死亡；任何历史runtime材料拒绝卸载。
            super::admin::no_runtime(&root)?;
        }
        super::admin::acknowledge(&root)?;
        Err(Error::HandoffRequired)
    }
    #[cfg(test)]
    pub(super) fn require_test_source(&mut self, root: std::path::PathBuf) {
        self.requires_source = true;
        self.source_root = Some(root);
    }
    fn bootstrap_source(
        &self,
        peer: super::transport::Peer,
        ticket: &BootstrapTicket,
    ) -> Result<super::source::BootstrapSource, Error> {
        if let Some(slot) = &self.bootstrap
            && slot.is_rebound()
        {
            return slot.rebind_source(
                peer,
                #[cfg(test)]
                self.source_root.as_deref(),
            );
        }
        #[cfg(test)]
        if let Some(root) = &self.source_root {
            return super::source::test_bootstrap(peer, ticket, root);
        }
        super::source::bootstrap(peer, ticket)
    }
    fn authorized_source(&mut self, peer: super::transport::Peer) -> Result<(), Error> {
        if let Some(slot) = &self.bootstrap {
            slot.verify_committed(peer)?;
            if self.source_failed {
                return Err(Error::HandoffRequired);
            }
            let proof = self.bootstrap_source(peer, slot.ticket())?;
            return slot.verify_source(&proof);
        }
        if !self.requires_source {
            return Ok(());
        }
        let result = (|| {
            let ticket = self.accepted.as_ref().ok_or(Error::HandoffRequired)?;
            if self.session != Some(peer)
                || self
                    .runtime
                    .as_ref()
                    .ok_or(Error::HandoffRequired)?
                    .owner_transfer()
                    .map_err(|_| Error::Unavailable)?
                    != Some(
                        veyra_core::application::runtime_recovery::OwnerTransfer::Local {
                            ticket: ticket.clone(),
                        },
                    )
            {
                return Err(Error::HandoffRequired);
            }
            self.source
                .as_mut()
                .ok_or(Error::HandoffRequired)?
                .verify(peer, ticket, true)
        })();
        self.source_failed = result.is_err();
        result
    }

    fn initialize(
        &mut self,
        config: &Configuration,
        peer: super::transport::Peer,
    ) -> Result<(), Error> {
        if self.runtime.is_some() {
            return Ok(());
        }
        let a = self
            .assets
            .as_ref()
            .ok_or(self.unavailable.unwrap_or(Error::Unavailable))?;
        a.verify()?;
        let root = a.root.clone();
        let port = if let Some(slot) = &self.bootstrap {
            // 同一lease open-file-description，绝不重新flock或在转入Port前释放。
            let lease = slot.lease_for_start(peer, config)?;
            ProcessPort::with_writer_lease(
                self.assets.take().expect("verified bootstrap assets"),
                self.deadline.clone(),
                Ok(lease),
            )
        } else {
            // 旧手动handoff仍按原pristine准入，不放宽旧owner路径。
            a.pristine_root()?;
            a.prepare_root()?;
            self.acquire_port()?
        };
        // 从此开始可能已有持久写入；后续失败不能把空Runtime槽报告成干净Stopped。
        self.status.recovery_required = true;
        self.unavailable = Some(Error::HandoffRequired);
        // 只写内核认证的peer，不接受payload自报session；文件在同一固定保护root。
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let binding = root.join("owner-session.json");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(binding)
            .map_err(|_| Error::UnsafeEndpoint)?;
        file.write_all(&serde_json::to_vec(&peer).map_err(|_| Error::Unavailable)?)
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Unavailable)?;
        std::fs::File::open(&root)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Unavailable)?;
        let store = JsonStateStore::new(root.join("received-state.json"))
            .map_err(|_| Error::Unavailable)?;
        store.save(&config.state).map_err(|_| Error::Unavailable)?;
        let snapshots = Arc::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let runtime = ManualRuntime::new(port, snapshots, root, true);
        self.runtime = Some(runtime);
        self.store = Some(store);
        self.unavailable = None;
        Ok(())
    }
    /// 在借用安装资产期间取得独占；竞争失败不消耗资产、不写owner/业务投影。
    fn acquire_port(&mut self) -> Result<ProcessPort, Error> {
        let assets = self.assets.as_ref().ok_or(Error::Unavailable)?;
        let lease = ProcessPort::acquire_writer_lease(assets)?;
        lease.verify()?;
        Ok(ProcessPort::with_writer_lease(
            self.assets
                .take()
                .expect("lease acquired with borrowed assets"),
            self.deadline.clone(),
            Ok(lease),
        ))
    }
    fn snapshot(&mut self) -> Result<Status, Error> {
        let Some(runtime) = &self.runtime else {
            return Ok(self.status.clone());
        };
        let s = runtime.snapshot().map_err(|_| Error::Unavailable)?;
        let id = if let Some(id) = s.runtime.instance_id {
            if self
                .wire_instance
                .as_ref()
                .is_none_or(|(known, _)| *known != id.0)
            {
                use sha2::Digest;
                let hash = sha2::Sha256::digest(id.0.as_bytes());
                self.wire_instance = Some((
                    id.0,
                    u64::from_be_bytes(hash[..8].try_into().expect("8 bytes")),
                ));
            }
            self.wire_instance.as_ref().map(|(_, id)| *id)
        } else if s.runtime.status == RuntimeStatus::Recovering {
            self.wire_instance.as_ref().map(|(_, id)| *id)
        } else {
            self.wire_instance = None;
            None
        };
        let applied = s
            .runtime
            .applied_version
            .zip(s.confirmed_selection_version)
            .map(|(config, selection)| SnapshotVersion { config, selection });
        let last_successful = s
            .runtime
            .last_successful_version
            .zip(s.runtime.last_successful_selection_version)
            .map(|(config, selection)| SnapshotVersion { config, selection });
        if self.bootstrap.is_some()
            && matches!(
                s.runtime.status,
                RuntimeStatus::Failed | RuntimeStatus::Recovering
            )
        {
            self.source_failed = true;
        }
        self.status = Status {
            instance: id,
            applied,
            last_successful,
            recovery_required: self.source_failed
                || self.bootstrap.as_ref().is_some_and(|s| !s.is_committed())
                || self.draining
                || s.runtime.status == RuntimeStatus::Recovering
                || self
                    .selection
                    .as_ref()
                    .is_some_and(|s| s.confirmed.is_none()),
        };
        Ok(self.status.clone())
    }
}
impl Backend for InstalledBackend {
    fn bootstrap(
        &mut self,
        peer: super::transport::Peer,
        action: BootstrapAction,
    ) -> (Status, Result<BootstrapReply, Error>) {
        let result = (|| {
            // 所有bootstrap读写均不触发drain操作；只读拒绝管理员正在撤销的安装。
            if self.draining
                || self
                    .admin_root
                    .as_ref()
                    .map(|r| super::admin::requested(r))
                    .transpose()?
                    .unwrap_or(false)
            {
                return Err(Error::HandoffRequired);
            }
            if matches!(
                action,
                BootstrapAction::RebindPreflight { .. }
                    | BootstrapAction::RebindPrepare { .. }
                    | BootstrapAction::RebindQuery { .. }
                    | BootstrapAction::RebindCommit { .. }
            ) {
                if self.source_failed || self.selection.is_some() || self.accepted.is_some() {
                    return Err(Error::HandoffRequired);
                }
                let slot = self.bootstrap.as_mut().ok_or(Error::HandoffRequired)?;
                let runtime = self.runtime.as_ref().ok_or(Error::HandoffRequired)?;
                let bundle = if slot.is_rebound() {
                    None
                } else {
                    Some(
                        runtime
                            .clean_stop_material(&slot.ticket().version)
                            .map_err(|_| Error::HandoffRequired)?,
                    )
                };
                let reply = slot.rebind(
                    peer,
                    action,
                    bundle.as_ref(),
                    #[cfg(test)]
                    self.source_root.as_deref(),
                )?;
                if slot.is_rebound() {
                    self.session = Some(peer);
                }
                return Ok(reply);
            }
            if let BootstrapAction::Commit { ticket, config } = &action
                && let Some(slot) = &self.bootstrap
                && slot.is_committed()
            {
                if slot.ticket() != ticket {
                    return Err(Error::RequestConflict);
                }
                config.validate()?;
                slot.lease_for_start(peer, config)?;
                slot.verify_source(&self.bootstrap_source(peer, ticket)?)?;
                return Ok(BootstrapReply {
                    ticket: ticket.clone(),
                    phase: BootstrapPhase::Committed,
                });
            }
            if let BootstrapAction::Query { ticket } = &action
                && let Some(slot) = &self.bootstrap
                && slot.is_committed()
            {
                if slot.ticket() != ticket {
                    return Err(Error::RequestConflict);
                }
                // 只读记录查询允许退出时primary listener已关闭；Start仍独立重新核验primary。
                slot.verify_committed(peer)?;
                return Ok(BootstrapReply {
                    ticket: ticket.clone(),
                    phase: BootstrapPhase::Committed,
                });
            }
            if self.session.is_some()
                || self.runtime.is_some()
                || self.selection.is_some()
                || self.accepted.is_some()
            {
                return Err(Error::HandoffRequired);
            }
            let assets = self
                .assets
                .as_ref()
                .ok_or(self.unavailable.unwrap_or(Error::HandoffRequired))?;
            #[cfg(test)]
            let source_root = self.source_root.clone();
            super::bootstrap::handle(assets, &mut self.bootstrap, peer, action, |ticket| {
                #[cfg(test)]
                if let Some(root) = &source_root {
                    return super::source::test_bootstrap(peer, ticket, root);
                }
                super::source::bootstrap(peer, ticket)
            })
        })();
        if let Some(slot) = &self.bootstrap {
            self.status.recovery_required = !slot.is_committed() || self.source_failed;
        }
        (self.status.clone(), result)
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            protocol: PROTOCOL,
            host_version: env!("CARGO_PKG_VERSION").into(),
            kernel_version: self
                .unavailable
                .is_none()
                .then(|| veyra_core::application::runtime_recovery::KERNEL_VERSION.into()),
            required_kernel_version: veyra_core::application::runtime_recovery::KERNEL_VERSION
                .into(),
            runtime: self.unavailable.map_or_else(
                || {
                    if self.requires_source
                        && self.accepted.is_none()
                        && !self.bootstrap.as_ref().is_some_and(|s| s.is_committed())
                    {
                        Err(Error::HandoffRequired)
                    } else {
                        Ok(())
                    }
                },
                Err,
            ),
            handoff: self.unavailable.map_or(Ok(()), Err),
            system_proxy: false,
            resource_bytes: RESOURCE_BYTES,
        }
    }
    fn poll(&mut self) -> Option<Status> {
        let _ = self.admin_barrier();
        let Some(runtime) = self.runtime.as_mut() else {
            return Some(self.status.clone());
        };
        // 明确NOTE_EXIT才停止已有Child；socket断线/锁消失/查询错误都不是kill证据。
        if let Some(slot) = self.bootstrap.as_mut() {
            match slot.peer_exited() {
                Ok(true) if self.status.instance.is_some() => {
                    *self.deadline.lock().expect("operation deadline") =
                        Instant::now() + std::time::Duration::from_secs(30);
                    self.source_failed = true; // 异常owner退出不签发CleanStop，不自动重新接管。
                    let _ = runtime.execute(LocalCommand::Stop, |_| {});
                }
                Err(_) => self.source_failed = true,
                _ => {}
            }
        }
        // 只在同一串行worker上核验自有child；不把断线当作owner死亡。
        let _ = runtime.execute(LocalCommand::Refresh, |_| {});
        let mut status = self.snapshot().ok()?;
        if self.requires_source
            && self.accepted.is_some()
            && self
                .session
                .is_some_and(|peer| self.authorized_source(peer).is_err())
        {
            status.recovery_required = true;
            self.status = status.clone();
        }
        Some(status)
    }
    fn selection(
        &mut self,
        peer: super::transport::Peer,
        action: RemoteSelectionAction,
        deadline: Instant,
    ) -> (Status, Result<RemoteSelectionReply, Error>) {
        *self.deadline.lock().expect("operation deadline") = deadline;
        let result = (|| {
            if self.bootstrap.is_some() {
                return Err(Error::Unsupported);
            }
            self.admin_barrier()?;
            if let Some(e) = self.unavailable {
                return Err(e);
            }
            if self.session != Some(peer) {
                return Err(Error::Unauthorized);
            }
            self.authorized_source(peer)?;
            let request = action.request().clone();
            if request.request_id == 0 {
                return Err(Error::InvalidRequest);
            }
            let current = self.poll().ok_or(Error::StaleInstance)?;
            if current.instance != Some(request.instance) {
                return Err(Error::StaleInstance);
            }
            let same = self
                .selection
                .as_ref()
                .is_some_and(|s| s.request == request);
            if !same
                && self
                    .selection
                    .as_ref()
                    .is_some_and(|s| s.request.request_id == request.request_id)
            {
                return Err(Error::RequestConflict);
            }
            if !same
                && self
                    .selection
                    .as_ref()
                    .is_some_and(|s| s.confirmed.is_none())
            {
                return Err(Error::Busy);
            }
            if !same {
                if !matches!(action, RemoteSelectionAction::Execute { .. }) {
                    return Err(Error::HandoffRequired);
                }
                self.runtime
                    .as_mut()
                    .unwrap()
                    .remote_selection_preflight(&request)
                    .map_err(|_| Error::VersionConflict)?;
                // 在controller PUT之前占有slot；断线/GET失败都不释放或重新PUT。
                self.selection = Some(RemoteSelectionReply {
                    request: request.clone(),
                    actual: None,
                    confirmed: None,
                });
                let actual = self
                    .runtime
                    .as_mut()
                    .unwrap()
                    .remote_selection_controller(&request, true)
                    .ok();
                self.selection.as_mut().unwrap().actual = actual;
            } else if matches!(action, RemoteSelectionAction::Query { .. })
                && self.selection.as_ref().unwrap().confirmed.is_none()
            {
                self.selection.as_mut().unwrap().actual = self
                    .runtime
                    .as_mut()
                    .unwrap()
                    .remote_selection_controller(&request, false)
                    .ok();
            }
            if let RemoteSelectionAction::Confirm { config, .. } = action {
                config.validate()?;
                let slot = self.selection.as_ref().unwrap();
                if let Some(version) = &slot.confirmed {
                    if *version != config.version()
                        || self
                            .store
                            .as_ref()
                            .unwrap()
                            .load()
                            .map_err(|_| Error::Unavailable)?
                            != *config.state
                    {
                        return Err(Error::VersionConflict);
                    }
                    return Ok(slot.clone());
                }
                let actual = slot.actual.clone().ok_or(Error::HandoffRequired)?;
                // 输入必须是旧投影的唯一本池确认变化；helper不自行生成第二份业务事实。
                let mut expected = self
                    .store
                    .as_ref()
                    .unwrap()
                    .load()
                    .map_err(|_| Error::Unavailable)?;
                if expected.version() != request.expected {
                    return Err(Error::VersionConflict);
                }
                let pool = expected
                    .pools
                    .iter_mut()
                    .find(|p| p.id == request.pool)
                    .ok_or(Error::InvalidRequest)?;
                let veyra_core::domain::SelectionPolicy::Manual {
                    selected_node_id,
                    pending_node_id,
                } = &mut pool.selection
                else {
                    return Err(Error::InvalidRequest);
                };
                if actual != request.node && selected_node_id.as_ref() != Some(&actual) {
                    return Err(Error::HandoffRequired);
                }
                *selected_node_id = Some(actual.clone());
                *pending_node_id = None;
                expected.selection_revision = expected
                    .selection_revision
                    .checked_add(2)
                    .ok_or(Error::VersionConflict)?;
                if expected != *config.state {
                    return Err(Error::VersionConflict);
                }
                self.runtime
                    .as_mut()
                    .unwrap()
                    .acknowledge_remote_selection(&request, actual, config.version().selection)
                    .map_err(|_| Error::Failed)?;
                // 仅保存桌面已CAS确认并严格校验的输入投影；失败slot仍未完成并阻止其他操作。
                self.store
                    .as_ref()
                    .unwrap()
                    .save(&config.state)
                    .map_err(|_| Error::Unavailable)?;
                self.selection.as_mut().unwrap().confirmed = Some(config.version());
            }
            Ok(self.selection.as_ref().unwrap().clone())
        })();
        (
            self.snapshot().unwrap_or_else(|_| Status {
                recovery_required: true,
                ..self.status.clone()
            }),
            result,
        )
    }
    fn handoff(
        &mut self,
        peer: super::transport::Peer,
        action: HandoffAction,
        deadline: Instant,
    ) -> (Status, Result<HandoffReply, Error>) {
        *self.deadline.lock().expect("operation deadline") = deadline;
        let result = (|| {
            if self.bootstrap.is_some() {
                return Err(Error::HandoffRequired);
            }
            // 诊断Query不得触发Stop/封存/ACK；管理员工作由poll或显式变更路径处理。
            // Preflight只检查drain拒绝状态，不能因读查询停止旧writer。
            if matches!(action, HandoffAction::Preflight { .. }) {
                if self.draining
                    || self
                        .admin_root
                        .as_ref()
                        .map(|root| super::admin::requested(root))
                        .transpose()?
                        .unwrap_or(false)
                {
                    return Err(Error::HandoffRequired);
                }
            } else if !matches!(action, HandoffAction::Query) {
                self.admin_barrier()?;
            }
            if self
                .unavailable
                .is_some_and(|e| e != Error::HandoffRequired)
            {
                return Err(self.unavailable.unwrap());
            }
            if self.session.is_some_and(|owner| owner != peer) {
                return Err(Error::Unauthorized);
            }
            // Query不写业务事实；未确认选择也要允许退出核验owner，再Stop而不清pending。
            if !matches!(action, HandoffAction::Query)
                && self
                    .selection
                    .as_ref()
                    .is_some_and(|s| s.confirmed.is_none())
            {
                return Err(Error::HandoffRequired);
            }
            if let HandoffAction::Preflight { id, config } = &action {
                // 未完成的正式总gate也属于不可接入，不可用一次成功握手诱使桌面停止。
                if let Some(error) = self.unavailable {
                    return Err(error);
                }
                config.validate()?;
                let owner = if let Some(runtime) = &self.runtime {
                    let owner = runtime.owner_transfer().map_err(|_| Error::Unavailable)?;
                    use veyra_core::application::runtime_recovery::OwnerTransfer;
                    let compatible = match &owner {
                        Some(
                            OwnerTransfer::Prepared { ticket } | OwnerTransfer::Local { ticket },
                        ) => ticket.id == *id && ticket.version == config.version(),
                        Some(OwnerTransfer::Released { ticket }) => {
                            ticket.id != *id
                                && ticket.version.config.0.epoch == config.version().config.0.epoch
                                && ticket.version.config.0.revision
                                    <= config.version().config.0.revision
                                && ticket.version.selection.0.revision
                                    <= config.version().selection.0.revision
                        }
                        _ => false,
                    };
                    if !compatible
                        || runtime
                            .snapshot()
                            .map_err(|_| Error::Unavailable)?
                            .runtime
                            .instance_id
                            .is_some()
                    {
                        return Err(Error::HandoffRequired);
                    }
                    owner
                } else {
                    let assets = self.assets.as_ref().ok_or(Error::NotInstalled)?;
                    assets.verify()?;
                    assets.pristine_root()?;
                    None
                };
                if self.requires_source
                    && !self
                        .source
                        .as_ref()
                        .is_some_and(|s| s.matches(peer, id, &config.version()))
                {
                    self.source = Some(super::source::Witness::capture(
                        peer,
                        id.clone(),
                        config.version(),
                        #[cfg(test)]
                        self.source_root.clone(),
                        #[cfg(test)]
                        self.source_exclude.clone(),
                    )?);
                }
                return Ok(HandoffReply {
                    owner,
                    bundle: None,
                });
            }
            if let HandoffAction::Prepare {
                config,
                ticket,
                bundle,
            } = &action
            {
                config.validate()?;
                if config.version() != ticket.version
                    || bundle
                        .ticket(ticket.id.clone())
                        .map_err(|_| Error::InvalidConfiguration)?
                        != *ticket
                {
                    return Err(Error::VersionConflict);
                }
                if self.requires_source {
                    self.source
                        .as_mut()
                        .ok_or(Error::HandoffRequired)?
                        .verify(peer, ticket, false)?;
                }
                self.initialize(config, peer)?;
                self.session = Some(peer);
            }
            if let HandoffAction::Commit { release } = &action
                && self.requires_source
            {
                let veyra_core::application::runtime_recovery::OwnerTransfer::Released { ticket } =
                    release
                else {
                    return Err(Error::InvalidRequest);
                };
                self.source
                    .as_mut()
                    .ok_or(Error::HandoffRequired)?
                    .verify(peer, ticket, true)?;
            }
            if matches!(action, HandoffAction::Export { .. }) {
                self.authorized_source(peer)?;
            }
            let Some(runtime) = self.runtime.as_mut() else {
                // 重启后没有child所有权证明，不初始化/恢复任何旧材料。
                if !matches!(action, HandoffAction::Query) {
                    return Err(Error::HandoffRequired);
                }
                let root = self.root.as_ref().ok_or(Error::NotInstalled)?;
                use std::io::Read;
                use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
                let file = std::fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                    .open(root.join("owner-session.json"))
                    .map_err(|_| Error::HandoffRequired)?;
                let m = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
                if !m.is_file()
                    || m.uid() != unsafe { libc::geteuid() }
                    || m.mode() & 0o7777 != 0o600
                    || m.nlink() != 1
                    || m.len() > 4096
                {
                    return Err(Error::UnsafeEndpoint);
                }
                let mut bytes = vec![];
                file.take(4097)
                    .read_to_end(&mut bytes)
                    .map_err(|_| Error::Unavailable)?;
                let bound: super::transport::Peer =
                    serde_json::from_slice(&bytes).map_err(|_| Error::Unauthorized)?;
                if bound != peer {
                    return Err(Error::Unauthorized);
                }
                self.status.recovery_required = true;
                return Ok(HandoffReply{owner:veyra_core::application::runtime_recovery::RecoveryStore::inspect_owner_transfer(root).map_err(|_|Error::UnsafeEndpoint)?,bundle:None});
            };
            let mut bundle = None;
            match action {
                HandoffAction::Query => {}
                HandoffAction::Preflight { .. } => unreachable!("handled without mutations"),
                HandoffAction::Prepare {
                    ticket, bundle: b, ..
                } => runtime
                    .receive_handoff(&ticket, &b)
                    .map_err(|_| Error::HandoffRequired)?,
                HandoffAction::Commit { release } => {
                    runtime
                        .commit_handoff(&release)
                        .map_err(|_| Error::HandoffRequired)?;
                    self.restore_transferred = true;
                    self.accepted = release.ticket().cloned();
                    self.source_failed = false;
                }
                HandoffAction::Export { id, expected } => {
                    bundle = Some(Box::new(
                        runtime
                            .prepare_handoff(id, expected)
                            .map_err(|_| Error::HandoffRequired)?
                            .1,
                    ));
                }
                HandoffAction::Release { ticket } => {
                    runtime
                        .release_handoff(&ticket)
                        .map_err(|_| Error::HandoffRequired)?;
                    self.accepted = None;
                    self.source = None;
                }
            }
            Ok(HandoffReply {
                owner: runtime.owner_transfer().map_err(|_| Error::Unavailable)?,
                bundle,
            })
        })();
        (
            self.snapshot().unwrap_or_else(|_| Status {
                recovery_required: true,
                ..Status::default()
            }),
            result,
        )
    }
    fn execute(
        &mut self,
        peer: super::transport::Peer,
        command: Command,
        deadline: Instant,
    ) -> (Status, Result<Status, Error>) {
        *self.deadline.lock().expect("operation deadline") = deadline;
        let result = (|| {
            if let Some(slot) = &self.bootstrap {
                if !slot.is_committed() {
                    return Err(Error::HandoffRequired);
                }
                if matches!(command, Command::Apply { .. }) {
                    return Err(Error::Unsupported);
                }
                if let Command::Start { config, .. } = &command {
                    slot.lease_for_start(peer, config)?;
                }
            }
            self.admin_barrier()?;
            if let Some(error) = self.unavailable {
                return Err(error);
            }
            if self.session.is_some_and(|session| session != peer) {
                return Err(Error::HandoffRequired);
            }
            if self
                .selection
                .as_ref()
                .is_some_and(|s| s.confirmed.is_none())
                && !matches!(
                    command,
                    Command::Stop { .. }
                        | Command::RuntimeCommand {
                            command: RuntimeCommand::Observe,
                            ..
                        }
                )
            {
                return Err(Error::HandoffRequired);
            }
            if !matches!(
                command,
                Command::Stop { .. }
                    | Command::RuntimeCommand {
                        command: RuntimeCommand::Observe,
                        ..
                    }
            ) {
                self.authorized_source(peer)?;
            }
            // IPC入队与执行之间child可能退出；在真正副作用之前重新核验当前实例/CAS。
            if let Some(runtime) = self.runtime.as_mut() {
                let _ = runtime.execute(LocalCommand::Refresh, |_| {});
            }
            let current = self.snapshot()?;
            // Refresh可能刚发现异常退出；不能沿用进入本次调用前的授权继续重启。
            if self.bootstrap.is_some()
                && self.source_failed
                && matches!(command, Command::Start { .. } | Command::Apply { .. })
            {
                return Err(Error::HandoffRequired);
            }
            if command
                .instance()
                .is_some_and(|id| current.instance != Some(id))
            {
                return Err(Error::StaleInstance);
            }
            match &command {
                Command::Apply { expected, .. } | Command::RuntimeCommand { expected, .. }
                    if current.applied.as_ref() != Some(expected) =>
                {
                    return Err(Error::VersionConflict);
                }
                Command::Start {
                    expected, config, ..
                } if expected != &config.version() => return Err(Error::VersionConflict),
                _ => {}
            }
            if self.requires_source && self.bootstrap.is_none() {
                if let Command::Start { expected, .. } = &command
                    && current
                        .applied
                        .as_ref()
                        .or(current.last_successful.as_ref())
                        != Some(expected)
                {
                    return Err(Error::VersionConflict);
                }
                if let Some(config) = command.configuration()
                    && self
                        .accepted
                        .as_ref()
                        .is_none_or(|t| t.version.config.0.epoch != config.version().config.0.epoch)
                {
                    return Err(Error::VersionConflict);
                }
            }
            let local = match &command {
                Command::Start { config, .. } | Command::Apply { config, .. } => {
                    config.validate()?;
                    if current.instance.is_none()
                        && let Some(slot) = self.bootstrap.as_mut()
                        && let Err(e) = slot.begin_run()
                    {
                        self.source_failed = true;
                        return Err(e);
                    }
                    self.initialize(config, peer)?;
                    self.session = Some(peer);
                    if self.wire_instance.is_none() {
                        use sha2::Digest;
                        let identity = format!(
                            "cleanup:{:?}",
                            veyra_core::domain::StateEpoch::fresh()
                                .map_err(|_| Error::Unavailable)?
                        );
                        let hash = sha2::Sha256::digest(identity.as_bytes());
                        self.wire_instance = Some((
                            identity,
                            u64::from_be_bytes(hash[..8].try_into().expect("8 bytes")),
                        ));
                    }
                    // 这是已接收的不可变编译输入副本，不是第二份可编辑业务权威；禁止在此保存选择。
                    self.store
                        .as_ref()
                        .expect("initialized input")
                        .save(&config.state)
                        .map_err(|_| Error::Unavailable)?;
                    if matches!(command, Command::Start { .. }) {
                        if self.restore_transferred {
                            LocalCommand::RestoreLastSuccessful
                        } else {
                            LocalCommand::Start
                        }
                    } else {
                        LocalCommand::ApplySaved
                    }
                }
                Command::Stop { .. } => LocalCommand::Stop,
                Command::RuntimeCommand {
                    command: RuntimeCommand::Observe,
                    ..
                } => LocalCommand::Refresh,
                Command::RuntimeCommand {
                    command: RuntimeCommand::Select { .. },
                    ..
                } => return Err(Error::HandoffRequired),
                _ => return Err(Error::Unsupported),
            };
            self.runtime
                .as_mut()
                .ok_or(Error::StaleInstance)?
                .execute(local, |_| {})
                .map_err(|_| Error::Failed)?;
            if matches!(command, Command::Stop { .. })
                && !self.source_failed
                && let Some(slot) = &self.bootstrap
            {
                let bundle = self
                    .runtime
                    .as_ref()
                    .ok_or(Error::HandoffRequired)?
                    .clean_stop_material(&slot.ticket().version)
                    .map_err(|_| Error::HandoffRequired);
                let result = bundle.and_then(|bundle| slot.clean_stop(&bundle));
                if let Err(e) = result {
                    self.source_failed = true;
                    return Err(e);
                }
            }
            self.restore_transferred = false;
            self.snapshot()
        })();
        let status = self.snapshot().unwrap_or_else(|_| {
            let mut s = self.status.clone();
            s.recovery_required = true;
            s
        });
        (status, result)
    }
}

#[cfg(test)]
mod lease_race_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn lease_race_retains_assets_until_exclusive_acquisition() {
        // 保护第一次启动预检后发生竞争：失败不能吞掉安装资产或写owner/投影。
        // 真实OS flock + 独立线程持锁；此处直接测试initialize的不可逆边界。
        let root = std::env::temp_dir().join(format!(
            "v206-lease-race-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let assets = Assets {
            root: root.clone(),
            kernel: std::env::current_exe().unwrap(),
            uid: unsafe { libc::geteuid() },
            gid: unsafe { libc::getegid() },
            installation: None,
            fixture: true,
            spawned: Default::default(),
        };
        let mut backend = InstalledBackend::from_assets(Ok(assets));
        backend.assets.as_ref().unwrap().pristine_root().unwrap();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let other_root = root.clone();
        let holder = std::thread::spawn(move || {
            let lease = super::super::writer_lease::WriterLease::acquire(&other_root).unwrap();
            ready_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            drop(lease);
        });
        ready_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(matches!(backend.acquire_port(), Err(Error::Busy)));
        assert!(backend.assets.is_some());
        assert!(backend.runtime.is_none());
        assert!(!root.join("owner-session.json").exists());
        assert!(!root.join("received-state.json").exists());
        release_tx.send(()).unwrap();
        holder.join().unwrap();
        // 不删除lock文件；释放后同backend仍可取得租约，安装事实没有变成NotInstalled。
        let port = backend.acquire_port().unwrap();
        assert!(!root.join("owner-session.json").exists());
        drop(port);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn initialize_failure_after_owner_write_keeps_recovery_required() {
        // 模拟受控输入在持久边界失败；已写owner之后绝不能套用“未修改可重试”分支。
        let root = std::env::temp_dir().join(format!(
            "v206-partial-init-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let (socket, _other) = std::os::unix::net::UnixStream::pair().unwrap();
        let peer = super::super::transport::peer(&socket).unwrap();
        let mut backend = InstalledBackend::from_assets(Ok(Assets {
            root: root.clone(),
            kernel: std::env::current_exe().unwrap(),
            uid: peer.uid,
            gid: peer.gid,
            installation: None,
            fixture: true,
            spawned: Default::default(),
        }));
        let mut bad = super::super::tests::config();
        bad.state.providers.clear(); // dangling节点使store.save在owner-session之后明确失败
        assert_eq!(backend.initialize(&bad, peer), Err(Error::Unavailable));
        let owner = std::fs::read(root.join("owner-session.json")).unwrap();
        assert!(backend.poll().unwrap().recovery_required);
        assert_eq!(backend.capabilities().handoff, Err(Error::HandoffRequired));
        assert_eq!(
            backend.initialize(&super::super::tests::config(), peer),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            std::fs::read(root.join("owner-session.json")).unwrap(),
            owner
        );
        assert!(backend.runtime.is_none());
        std::fs::remove_dir_all(root).unwrap();
    }
}
