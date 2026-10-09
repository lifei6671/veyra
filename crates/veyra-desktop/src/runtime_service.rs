//! 单一串行 worker 同时拥有应用服务和真实 Port；GPUI 只投递封闭命令。
use crate::{platform::manual_sidecar::ManualSidecarPort, state_bridge::AppEvent};
use std::{
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};
use veyra_core::application::{
    manual_runtime::{
        ManualRuntime, ManualRuntimeSnapshot, ManualSelectionRequest, RuntimeCommand, RuntimeError,
        RuntimeResult, SelectionChoice, SelectionError,
    },
    state_service::SnapshotService,
};
#[derive(Clone, Debug)]
pub struct RuntimeEvent {
    pub request: u64,
    pub sequence: u64,
    pub snapshot: Result<ManualRuntimeSnapshot, RuntimeError>,
    pub result: Option<Result<RuntimeResult, RuntimeError>>,
}
enum Message {
    Operation(u64, RuntimeCommand),
    Selection(
        ManualSelectionRequest<SelectionChoice>,
        mpsc::Sender<Result<veyra_core::domain::SelectionVersion, SelectionError>>,
    ),
    #[cfg(target_os = "macos")]
    Helper(
        Box<veyra_core::application::helper_protocol::Command>,
        mpsc::Sender<
            Result<
                veyra_core::application::helper_protocol::Response,
                veyra_core::application::helper_protocol::Error,
            >,
        >,
    ),
    #[cfg(target_os = "macos")]
    Transfer(
        veyra_core::domain::StateEpoch,
        veyra_core::domain::SnapshotVersion,
        veyra_core::application::helper_transfer::TransferDirection,
        mpsc::Sender<
            Result<
                veyra_core::application::runtime_recovery::OwnerTransfer,
                veyra_core::application::helper_protocol::Error,
            >,
        >,
    ),
    #[cfg(target_os = "macos")]
    BootstrapCommit(
        veyra_core::domain::SnapshotVersion,
        mpsc::Sender<
            Result<
                veyra_core::application::helper_protocol::BootstrapReply,
                veyra_core::application::helper_protocol::Error,
            >,
        >,
    ),
    #[cfg(target_os = "macos")]
    RebindPrepare(
        veyra_core::domain::StateEpoch,
        veyra_core::domain::SnapshotVersion,
        mpsc::Sender<
            Result<
                veyra_core::application::helper_protocol::BootstrapReply,
                veyra_core::application::helper_protocol::Error,
            >,
        >,
    ),
    #[cfg(target_os = "macos")]
    RebindCommit(
        veyra_core::domain::SnapshotVersion,
        mpsc::Sender<
            Result<
                veyra_core::application::helper_protocol::BootstrapReply,
                veyra_core::application::helper_protocol::Error,
            >,
        >,
    ),

    #[cfg(target_os = "macos")]
    BootstrapPrepare(
        veyra_core::domain::StateEpoch,
        veyra_core::domain::SnapshotVersion,
        mpsc::Sender<
            Result<
                veyra_core::application::helper_protocol::BootstrapReply,
                veyra_core::application::helper_protocol::Error,
            >,
        >,
    ),
    FirstFreeze(
        veyra_core::domain::StateEpoch,
        veyra_core::domain::StateEpoch,
        veyra_core::domain::SnapshotVersion,
        mpsc::Sender<
            Result<veyra_core::application::runtime_recovery::OwnerTransfer, RuntimeError>,
        >,
    ),
    OwnerQuery(
        mpsc::Sender<
            Result<Option<veyra_core::application::runtime_recovery::OwnerTransfer>, RuntimeError>,
        >,
    ),
    Reconcile(
        veyra_core::application::runtime_snapshot::InstanceId,
        veyra_core::domain::SnapshotVersion,
        veyra_core::domain::PoolId,
        mpsc::Sender<Result<veyra_core::domain::SelectionVersion, SelectionError>>,
    ),
    Quit(mpsc::Sender<Result<(), RuntimeError>>),
}
pub struct RuntimeService {
    sender: mpsc::Sender<Message>,
    worker: Option<thread::JoinHandle<()>>,
    observation: Arc<
        std::sync::Mutex<
            Option<veyra_core::application::observability::controller::ObservationReader>,
        >,
    >,
}
impl RuntimeService {
    pub fn observation_reader(
        &self,
    ) -> Option<veyra_core::application::observability::controller::ObservationReader> {
        self.observation.lock().expect("observation reader").clone()
    }
    pub fn new(
        root: std::path::PathBuf,
        snapshots: Arc<SnapshotService>,
        events: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let observation = Arc::new(std::sync::Mutex::new(None));
        let reader_slot = observation.clone();
        let worker = thread::spawn(move || {
            let port = ManualSidecarPort::new(root.clone());
            let available = port.kernel_available();
            let traffic_directory = root.join("traffic");
            let mut owner = ManualRuntime::new(port, snapshots.clone(), root, available);
            owner.enable_traffic_storage(traffic_directory);
            // P2-06 owner确认：只交付一次 Weak reader，不交出Runtime/统计写权。
            *reader_slot.lock().expect("observation reader") = Some(owner.observation().reader());
            #[cfg(target_os = "macos")]
            let helper = veyra_helper::production::Client::default();
            #[cfg(not(target_os = "macos"))]
            let helper = ();
            #[cfg(target_os = "macos")]
            let mut bootstrap_touched = false;
            #[cfg(target_os = "macos")]
            let mut bootstrap_committed = false;
            #[cfg(not(target_os = "macos"))]
            let bootstrap_touched = false;
            let (mut request, mut sequence) = (0, 0);
            let mut previous = None;
            let clock = std::time::Instant::now();
            loop {
                let message = receiver.recv_timeout(Duration::from_millis(250));
                let mut emit = |snapshot, result| {
                    sequence += 1;
                    let _ = events.send(AppEvent::Runtime(RuntimeEvent {
                        request,
                        sequence,
                        snapshot,
                        result,
                    }));
                };
                match message {
                    #[cfg(target_os = "macos")]
                    Ok(Message::Helper(command, reply)) => {
                        if matches!(
                            *command,
                            veyra_core::application::helper_protocol::Command::Start { .. }
                                | veyra_core::application::helper_protocol::Command::Apply { .. }
                        ) && let Err(error) = helper_group_admission(&snapshots)
                        {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        // 本worker从未确认Commit时，不以磁盘Frozen或空child槽推导启动能力。
                        if !bootstrap_committed
                            && matches!(*command, veyra_core::application::helper_protocol::Command::Start { .. } | veyra_core::application::helper_protocol::Command::Apply { .. })
                            && matches!(owner.owner_transfer(), Ok(Some(veyra_core::application::runtime_recovery::OwnerTransfer::BootstrapFrozen { .. }))) {
                            let _ = reply.send(Err(veyra_core::application::helper_protocol::Error::HandoffRequired));
                            continue;
                        }

                        if let veyra_core::application::helper_protocol::Command::RuntimeCommand {
                            request_id,
                            instance,
                            expected,
                            command:
                                veyra_core::application::helper_protocol::RuntimeCommand::Select {
                                    pool,
                                    node,
                                },
                        } = &*command
                        {
                            let selection = veyra_core::storage::RemoteSelection {
                                request_id: *request_id,
                                instance: *instance,
                                expected: expected.clone(),
                                pool: pool.clone(),
                                node: node.clone(),
                            };
                            use veyra_core::application::{
                                helper_protocol::{Command, Error, Response},
                                runtime_recovery::OwnerTransfer,
                            };
                            let result = (|| {
                                if !matches!(
                                    owner.owner_transfer().map_err(|_| Error::Unavailable)?,
                                    Some(OwnerTransfer::Released { .. })
                                ) || owner
                                    .snapshot()
                                    .map_err(|_| Error::Unavailable)?
                                    .runtime
                                    .instance_id
                                    .is_some()
                                {
                                    return Err(Error::HandoffRequired);
                                }
                                // 首次请求先确认helper实际owner；重试靠持久fence查询，不能把UNKNOWN当失败解除。
                                if snapshots
                                    .remote_selection_fence()
                                    .map_err(|_| Error::Unavailable)?
                                    .is_none()
                                {
                                    match helper.request(Command::Status)? {
                                        Response::Status(s)
                                            if s.instance == Some(selection.instance)
                                                && s.applied.as_ref()
                                                    == Some(&selection.expected)
                                                && !s.recovery_required => {}
                                        _ => return Err(Error::VersionConflict),
                                    }
                                }
                                veyra_core::application::helper_transfer::select_remote(
                                    &snapshots,
                                    selection,
                                    |action| match helper.request(Command::Selection {
                                        action: Box::new(action),
                                    })? {
                                        Response::Selection(result) => result,
                                        Response::Rejected(e) => Err(e),
                                        _ => Err(Error::InvalidRequest),
                                    },
                                )?;
                                helper.request(Command::Status)
                            })();
                            let _ = reply.send(result);
                            continue;
                        }
                        let result = owner
                            .snapshot()
                            .map_err(|_| {
                                veyra_core::application::helper_protocol::Error::Unavailable
                            })
                            .and_then(|manual| {
                                crate::platform::helper::request(
                                    &helper,
                                    &snapshots,
                                    &manual,
                                    helper_source_owner(&owner, bootstrap_committed).map_err(|_| {
                                        veyra_core::application::helper_protocol::Error::HandoffRequired
                                    })?,
                                    *command,
                                )
                            });
                        let _ = reply.send(result);
                    }
                    #[cfg(target_os = "macos")]
                    Ok(Message::Transfer(id, expected, direction, reply)) => {
                        if let Err(error) = helper_group_admission(&snapshots) {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        use veyra_core::application::helper_protocol::{Command, Error, Response};
                        let result = veyra_core::application::helper_transfer::transfer(
                            &mut owner,
                            id,
                            expected,
                            direction,
                            |action| match helper.request(Command::Handoff {
                                action: Box::new(action),
                            })? {
                                Response::Handoff(result) => result,
                                Response::Rejected(e) => Err(e),
                                _ => Err(Error::InvalidRequest),
                            },
                        );
                        let _ = reply.send(result);
                    }
                    #[cfg(target_os = "macos")]
                    Ok(Message::BootstrapCommit(expected, reply)) => {
                        if let Err(error) = helper_group_admission(&snapshots) {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        bootstrap_touched = true;
                        use veyra_core::application::helper_protocol::{Command, Error, Response};
                        let result = veyra_core::application::helper_transfer::commit_bootstrap(
                            &mut owner,
                            expected,
                            |action| match helper.request(Command::Bootstrap {
                                action: Box::new(action),
                            })? {
                                Response::Bootstrap(result) => result,
                                Response::Rejected(e) => Err(e),
                                _ => Err(Error::InvalidRequest),
                            },
                        );
                        bootstrap_committed |= result.is_ok();
                        let _ = reply.send(result);
                    }
                    #[cfg(target_os = "macos")]
                    Ok(Message::RebindCommit(expected, reply)) => {
                        if let Err(error) = helper_group_admission(&snapshots) {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        bootstrap_touched = true;
                        use veyra_core::application::helper_protocol::{Command, Error, Response};
                        let result = veyra_core::application::helper_transfer::commit_rebind(
                            &mut owner,
                            expected,
                            |action| match helper.request(Command::Bootstrap {
                                action: Box::new(action),
                            })? {
                                Response::Bootstrap(result) => result,
                                Response::Rejected(e) => Err(e),
                                _ => Err(Error::InvalidRequest),
                            },
                        );
                        bootstrap_committed |= result.is_ok();
                        let _ = reply.send(result);
                    }
                    #[cfg(target_os = "macos")]
                    Ok(Message::RebindPrepare(id, expected, reply)) => {
                        if let Err(error) = helper_group_admission(&snapshots) {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        bootstrap_touched = true;
                        use veyra_core::application::helper_protocol::{Command, Error, Response};
                        let result = veyra_core::application::helper_transfer::prepare_rebind(
                            &mut owner,
                            id,
                            expected,
                            |action| match helper.request(Command::Bootstrap {
                                action: Box::new(action),
                            })? {
                                Response::Bootstrap(result) => result,
                                Response::Rejected(e) => Err(e),
                                _ => Err(Error::InvalidRequest),
                            },
                        );
                        let _ = reply.send(result);
                    }
                    #[cfg(target_os = "macos")]
                    Ok(Message::BootstrapPrepare(id, expected, reply)) => {
                        if let Err(error) = helper_group_admission(&snapshots) {
                            let _ = reply.send(Err(error));
                            continue;
                        }
                        bootstrap_touched = true;
                        use veyra_core::application::helper_protocol::{Command, Error, Response};
                        let result = veyra_core::application::helper_transfer::prepare_bootstrap(
                            &mut owner,
                            id,
                            expected,
                            |action| match helper.request(Command::Bootstrap {
                                action: Box::new(action),
                            })? {
                                Response::Bootstrap(result) => result,
                                Response::Rejected(e) => Err(e),
                                _ => Err(Error::InvalidRequest),
                            },
                        );
                        let _ = reply.send(result);
                    }
                    Ok(Message::FirstFreeze(id, installation, expected, reply)) => {
                        #[cfg(target_os = "macos")]
                        if helper_group_admission(&snapshots).is_err() {
                            let _ = reply.send(Err(RuntimeError::SelectionPending));
                            continue;
                        }
                        // 与Manual Start/Select同一worker。丢失reply不撤销已经落盘的冻结。
                        let _ =
                            reply.send(owner.freeze_first_bootstrap(id, installation, expected));
                    }
                    Ok(Message::OwnerQuery(reply)) => {
                        let _ = reply.send(owner.owner_transfer());
                    }
                    Ok(Message::Quit(done)) => {
                        let result = shutdown_owner(&mut owner, &helper, bootstrap_touched);
                        // 远端失败不能拿本地空Port快照冒充Stopped；持久Released/fence原样保留。
                        // 成功退出不读取业务snapshot，避免尚未初始化的state因Quit而落盘。
                        if let Err(error) = result {
                            emit(
                                owner.snapshot().map(|mut snapshot| {
                                    snapshot.runtime.status = veyra_core::application::runtime_snapshot::RuntimeStatus::Recovering;
                                    snapshot.runtime.applied_version = None;
                                    snapshot.confirmed_selection_version = None;
                                    snapshot
                                }),
                                Some(Err(error)),
                            );
                        }
                        let _ = done.send(result);
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let result = shutdown_owner(&mut owner, &helper, bootstrap_touched);
                        // 成功退出不读取业务snapshot，避免尚未初始化的state因Quit而落盘。
                        if let Err(error) = result {
                            emit(
                                owner.snapshot().map(|mut snapshot| {
                                    snapshot.runtime.status = veyra_core::application::runtime_snapshot::RuntimeStatus::Recovering;
                                    snapshot.runtime.applied_version = None;
                                    snapshot.confirmed_selection_version = None;
                                    snapshot
                                }),
                                Some(Err(error)),
                            );
                        }
                        break;
                    }
                    Ok(Message::Reconcile(instance, expected, group, reply)) => {
                        let result = owner.reconcile_selection(instance, expected, group);
                        let _ = reply.send(result);
                        let snapshot = owner.snapshot();
                        previous = Some(snapshot.clone());
                        emit(snapshot, None);
                    }
                    Ok(Message::Selection(selection, reply)) => {
                        let result = owner.select_manual(selection);
                        let _ = reply.send(result);
                        let snapshot = owner.snapshot();
                        previous = Some(snapshot.clone());
                        emit(snapshot, None);
                    }
                    Ok(Message::Operation(id, command)) => {
                        request = id;
                        let result = owner.execute(command, |snapshot| {
                            sequence += 1;
                            let _ = events.send(AppEvent::Runtime(RuntimeEvent {
                                request,
                                sequence,
                                snapshot: Ok(snapshot),
                                result: None,
                            }));
                        });
                        let snapshot = owner.snapshot();
                        previous = Some(snapshot.clone());
                        sequence += 1;
                        eprintln!("manual-runtime operation request={request} result={result:?}");
                        let _ = events.send(AppEvent::Runtime(RuntimeEvent {
                            request,
                            sequence,
                            snapshot,
                            result: Some(result),
                        }));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) if request != 0 => {
                        let result = owner.execute(RuntimeCommand::Refresh, |_| {});
                        owner.poll_failover(clock.elapsed().as_millis() as u64);
                        let snapshot = owner.snapshot();
                        if previous.as_ref() != Some(&snapshot) {
                            previous = Some(snapshot.clone());
                            emit(snapshot, result.err().map(Err));
                        }
                    }
                    _ => {}
                }
            }
        });
        Self {
            sender,
            worker: Some(worker),
            observation,
        }
    }
    pub fn reconcile_selection(
        &self,
        instance: veyra_core::application::runtime_snapshot::InstanceId,
        expected: veyra_core::domain::SnapshotVersion,
        group: veyra_core::domain::PoolId,
    ) -> mpsc::Receiver<Result<veyra_core::domain::SelectionVersion, SelectionError>> {
        let (tx, rx) = mpsc::channel();
        let _ = self
            .sender
            .send(Message::Reconcile(instance, expected, group, tx));
        rx
    }
    pub fn submit(&self, request: u64, command: RuntimeCommand) {
        let _ = self.sender.send(Message::Operation(request, command));
    }
    /// 后续 Proxy UI 消费的业务 ID 入口，与应用命令共用一个 worker，无法传 endpoint/tag。
    pub fn select_manual<T: Into<SelectionChoice>>(
        &self,
        selection: ManualSelectionRequest<T>,
    ) -> mpsc::Receiver<Result<veyra_core::domain::SelectionVersion, SelectionError>> {
        let (tx, rx) = mpsc::channel();
        let selection = ManualSelectionRequest {
            instance: selection.instance,
            config: selection.config,
            expected: selection.expected,
            pool: selection.pool,
            node: selection.node.into(),
        };
        let _ = self.sender.send(Message::Selection(selection, tx));
        rx
    }
    /// P2-06 Runtime 生产入口，与手动 owner 共用串行 worker；不开放 GUI 模式开关。
    #[cfg(target_os = "macos")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "P2-06 local contract; Native/UI consumer pending")
    )]
    pub fn helper_request(
        &self,
        command: veyra_core::application::helper_protocol::Command,
    ) -> mpsc::Receiver<
        Result<
            veyra_core::application::helper_protocol::Response,
            veyra_core::application::helper_protocol::Error,
        >,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::Helper(Box::new(command), tx));
        rx
    }
    /// 同一worker内完成真实Stop/reap和双向握手；超时保留持久fence。
    #[cfg(target_os = "macos")]
    #[cfg_attr(not(test), expect(dead_code, reason = "P2-06 native consumer pending"))]
    pub fn transfer_owner(
        &self,
        id: veyra_core::domain::StateEpoch,
        expected: veyra_core::domain::SnapshotVersion,
        direction: veyra_core::application::helper_transfer::TransferDirection,
    ) -> mpsc::Receiver<
        Result<
            veyra_core::application::runtime_recovery::OwnerTransfer,
            veyra_core::application::helper_protocol::Error,
        >,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self
            .sender
            .send(Message::Transfer(id, expected, direction, tx));
        rx
    }
    /// Commit仅提交owner，调用方随后以同一SnapshotVersion显式Start并查询Operation。
    #[cfg(target_os = "macos")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "P2-06 native UI consumer pending")
    )]
    pub fn commit_bootstrap(
        &self,
        expected: veyra_core::domain::SnapshotVersion,
    ) -> mpsc::Receiver<
        Result<
            veyra_core::application::helper_protocol::BootstrapReply,
            veyra_core::application::helper_protocol::Error,
        >,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::BootstrapCommit(expected, tx));
        rx
    }
    /// 只到root Prepared；固定端点预检失败时不会冻结Desktop，也不自动Start。
    #[cfg(target_os = "macos")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "P2-06 rebind Commit/UI pending")
    )]
    pub fn prepare_rebind(
        &self,
        id: veyra_core::domain::StateEpoch,
        expected: veyra_core::domain::SnapshotVersion,
    ) -> mpsc::Receiver<
        Result<
            veyra_core::application::helper_protocol::BootstrapReply,
            veyra_core::application::helper_protocol::Error,
        >,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::RebindPrepare(id, expected, tx));
        rx
    }
    #[cfg(target_os = "macos")]
    #[cfg_attr(not(test), expect(dead_code, reason = "P2-06 GUI admission pending"))]
    pub fn commit_rebind(
        &self,
        expected: veyra_core::domain::SnapshotVersion,
    ) -> mpsc::Receiver<
        Result<
            veyra_core::application::helper_protocol::BootstrapReply,
            veyra_core::application::helper_protocol::Error,
        >,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::RebindCommit(expected, tx));
        rx
    }
    /// 只到root Prepared；固定端点预检失败时不会冻结Desktop，也不自动Start。
    #[cfg(target_os = "macos")]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "P2-06 Commit/UI admission pending")
    )]
    pub fn prepare_bootstrap(
        &self,
        id: veyra_core::domain::StateEpoch,
        expected: veyra_core::domain::SnapshotVersion,
    ) -> mpsc::Receiver<
        Result<
            veyra_core::application::helper_protocol::BootstrapReply,
            veyra_core::application::helper_protocol::Error,
        >,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self
            .sender
            .send(Message::BootstrapPrepare(id, expected, tx));
        rx
    }
    /// 首次合作让权的本地阶段；调用方须先完成安装预检，当前不会调用helper Start。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "P2-06 bootstrap root handshake pending")
    )]
    pub fn freeze_first_bootstrap(
        &self,
        id: veyra_core::domain::StateEpoch,
        installation: veyra_core::domain::StateEpoch,
        expected: veyra_core::domain::SnapshotVersion,
    ) -> mpsc::Receiver<
        Result<veyra_core::application::runtime_recovery::OwnerTransfer, RuntimeError>,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self
            .sender
            .send(Message::FirstFreeze(id, installation, expected, tx));
        rx
    }
    /// 查询不会清除跨进程遗留freeze，也不会凭空Child槽声明旧writer已退出。
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "P2-06 bootstrap root handshake pending")
    )]
    pub fn owner_transfer(
        &self,
    ) -> mpsc::Receiver<
        Result<Option<veyra_core::application::runtime_recovery::OwnerTransfer>, RuntimeError>,
    > {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::OwnerQuery(tx));
        rx
    }
    /// on_app_quit 在后台 await 清理完成；窗口隐藏不调用此方法。
    pub fn shutdown(&self) -> mpsc::Receiver<Result<(), RuntimeError>> {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::Quit(tx));
        rx
    }
}
/// 准入只来自本worker确认的提交。磁盘旧Frozen仍是本地writer永久fence。
#[cfg(target_os = "macos")]
fn helper_source_owner(
    owner: &ManualRuntime<ManualSidecarPort>,
    committed: bool,
) -> Result<Option<veyra_core::application::runtime_recovery::OwnerTransfer>, RuntimeError> {
    if committed && let Some(ticket) = owner.rebind_ticket()? {
        return Ok(Some(
            veyra_core::application::runtime_recovery::OwnerTransfer::BootstrapFrozen {
                id: ticket.id,
                installation: ticket.installation,
                version: ticket.version,
            },
        ));
    }
    owner.owner_transfer()
}
/// Quit和sender断开共用同一worker清理；隐藏窗口没有这个消息。
#[cfg(target_os = "macos")]
fn shutdown_owner(
    owner: &mut ManualRuntime<ManualSidecarPort>,
    helper: &veyra_helper::production::Client,
    bootstrap_touched: bool,
) -> Result<(), RuntimeError> {
    use veyra_core::application::runtime_recovery::OwnerTransfer;
    match helper_source_owner(owner, bootstrap_touched)? {
        Some(OwnerTransfer::Released { ticket }) => helper
            .stop_released(&ticket)
            .map_err(|_| RuntimeError::StopFailed),
        Some(OwnerTransfer::BootstrapFrozen { .. }) if !bootstrap_touched => {
            Err(RuntimeError::StopFailed)
        }
        Some(OwnerTransfer::BootstrapFrozen {
            id,
            installation,
            version,
        }) => helper
            .stop_bootstrap(&veyra_core::application::helper_protocol::BootstrapTicket {
                id,
                installation,
                version,
            })
            .map_err(|_| RuntimeError::StopFailed),
        Some(
            OwnerTransfer::Freezing { .. }
            | OwnerTransfer::Closed { .. }
            | OwnerTransfer::Prepared { .. },
        ) => {
            // 半交接不能从缺少本地handle推断全局停止，也不重新启用手动writer。
            Err(RuntimeError::StopFailed)
        }
        None | Some(OwnerTransfer::Local { .. }) => {
            owner.execute(RuntimeCommand::Stop, |_| {}).map(|_| ())
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn shutdown_owner(
    owner: &mut ManualRuntime<ManualSidecarPort>,
    _: &(),
    _: bool,
) -> Result<(), RuntimeError> {
    owner.execute(RuntimeCommand::Stop, |_| {}).map(|_| ())
}

impl Drop for RuntimeService {
    fn drop(&mut self) {
        let _ = self.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
mod helper_tests {
    use super::*;
    use veyra_core::{
        application::{
            helper_protocol::{Command, Configuration, Error},
            state_access::StateAccessGate,
        },
        domain::AppState,
        storage::JsonStateStore,
    };

    #[test]
    fn bootstrap_worker_stale_version_rejects_before_endpoint_or_freeze() {
        // 真worker先做本地CAS；测试不会访问正式helper endpoint。
        let root = std::env::temp_dir().join(format!(
            "v206-bootstrap-worker-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let state = AppState::empty();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let (events, _rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots, events);
        let mut stale = state.version();
        stale.config.0.revision += 1;
        assert_eq!(
            service
                .prepare_bootstrap(
                    veyra_core::domain::StateEpoch::fresh().unwrap(),
                    stale.clone()
                )
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .owner_transfer()
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap(),
            None
        );
        assert_eq!(
            service
                .commit_bootstrap(stale)
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .prepare_rebind(
                    veyra_core::domain::StateEpoch::fresh().unwrap(),
                    state.version()
                )
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .commit_rebind(state.version())
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        // 无本地Frozen票据的Query必须在访问正式endpoint之前拒绝。
        assert_eq!(
            service
                .helper_request(Command::Bootstrap {
                    action: Box::new(
                        veyra_core::application::helper_protocol::BootstrapAction::Query {
                            ticket: veyra_core::application::helper_protocol::BootstrapTicket {
                                id: veyra_core::domain::StateEpoch::fresh().unwrap(),
                                installation: veyra_core::domain::StateEpoch::fresh().unwrap(),
                                version: state.version()
                            }
                        }
                    )
                })
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn first_freeze_worker_reply_loss_and_restart_preserve_fence() {
        // 真worker/磁盘集成；不连接正式helper，不启动任何内核。
        let root = std::env::temp_dir().join(format!(
            "v206-first-worker-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let state = AppState::empty();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let (events, _rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots.clone(), events.clone());
        let id = veyra_core::domain::StateEpoch::fresh().unwrap();
        let installation = veyra_core::domain::StateEpoch::fresh().unwrap();
        drop(service.freeze_first_bootstrap(id.clone(), installation.clone(), state.version()));
        let record = service
            .owner_transfer()
            .recv_timeout(Duration::from_secs(2))
            .unwrap()
            .unwrap();
        assert!(matches!(
            record,
            Some(veyra_core::application::runtime_recovery::OwnerTransfer::BootstrapFrozen { .. })
        ));
        assert_eq!(
            service
                .freeze_first_bootstrap(id.clone(), installation.clone(), state.version())
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap(),
            record.clone().unwrap()
        );
        assert_eq!(
            service
                .helper_request(Command::Start {
                    request_id: 101,
                    expected: state.version(),
                    config: Configuration {
                        state: Box::new(state.clone())
                    }
                })
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        let bytes = std::fs::read(root.join("runtime/owner-transfer.json")).unwrap();
        assert!(
            service
                .shutdown()
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .is_err()
        );
        drop(service);
        let service = RuntimeService::new(root.clone(), snapshots, events);
        assert_eq!(
            service
                .owner_transfer()
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap(),
            record
        );
        assert!(
            service
                .freeze_first_bootstrap(id, installation, state.version())
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .is_err()
        );
        service.submit(99, RuntimeCommand::Start);
        // 同worker查询是队列屏障，Start已执行并拒绝，未连接helper或创建第二writer。
        assert_eq!(
            service
                .owner_transfer()
                .recv_timeout(Duration::from_secs(2))
                .unwrap()
                .unwrap(),
            record
        );
        drop(service);
        assert_eq!(
            std::fs::read(root.join("runtime/owner-transfer.json")).unwrap(),
            bytes
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn helper_uses_real_runtime_worker_and_rejects_stale_business_version() {
        // 保护 helper 通过正式 Runtime composition 读取业务事实，旧请求在连接前被拒绝。
        // 自有临时 root，无生产 socket/真实 child/GUI 操作。
        let root = std::env::temp_dir().join(format!(
            "v206-desktop-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let state = AppState::empty();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let (events, _rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots, events);
        let mut expected = state.version();
        expected.config.0.revision += 1;
        let result = service
            .helper_request(Command::Start {
                request_id: 1,
                expected,
                config: Configuration {
                    state: Box::new(state),
                },
            })
            .recv_timeout(Duration::from_secs(2))
            .unwrap();
        assert_eq!(result, Err(Error::VersionConflict));
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, target_os = "macos"))]
mod transfer_tests {
    #[test]
    fn clean_manual_owner_without_manifest_is_not_frozen_or_sent_to_helper() {
        // 保护干净首次启动：尚无cold-start合同就拒绝，但不能留下阻止手动代理的交接fence。
        use super::*;
        use veyra_core::{
            application::{
                helper_protocol::Error, helper_transfer::TransferDirection,
                state_access::StateAccessGate,
            },
            domain::{AppState, StateEpoch},
            storage::JsonStateStore,
        };
        let root = std::env::temp_dir().join(format!(
            "p206-empty-transfer-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let state = AppState::empty();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let (events, _rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots, events);
        assert_eq!(
            service
                .transfer_owner(
                    StateEpoch::fresh().unwrap(),
                    state.version(),
                    TransferDirection::ToHelper
                )
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert!(!root.join("runtime/owner-transfer.json").exists());
        assert!(!root.join("runtime/owner-incarnation.json").exists());
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(all(test, target_os = "macos"))]
mod quit_tests {
    // 保护实际 Desktop worker 的 Ready→统计 writer→Quit 完成回告后的 DB/端口释放。
    #[test]
    #[ignore = "requires pinned VEYRA_SING_BOX_PATH; real RuntimeService Quit cleanup"]
    fn traffic_wiring_real_worker_quit_releases_statistics() {
        use super::*;
        use veyra_core::{
            application::state_access::StateAccessGate,
            domain::AppState,
            storage::{JsonStateStore, traffic::TrafficWriter},
        };
        let root = std::env::temp_dir().join(format!(
            "veyra-stat-quit-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../veyra-core/tests/fixtures/compiler/p2-02b.json"
        ))
        .unwrap();
        for (key, entry) in fixture.as_object().unwrap() {
            value[key] = entry.clone();
        }
        let mut state: AppState = serde_json::from_value(value).unwrap();
        state.active_subscription_id =
            Some(veyra_core::domain::SubscriptionId("subscription".into()));
        state.default_target = veyra_core::domain::RouteTarget::Direct;
        state.routes.clear();
        state.pools.retain(|pool| pool.id.0 == "manual");
        state.validate().unwrap();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let (events, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots, events);
        assert!(!root.join("traffic").exists());
        service.submit(1, RuntimeCommand::Start);
        let until = std::time::Instant::now() + Duration::from_secs(20);
        let endpoints = loop {
            if let Ok(AppEvent::Runtime(event)) = rx.try_recv() {
                if let Some(Err(error)) = event.result {
                    panic!("start failed: {error:?}");
                }
                if let Ok(snapshot) = event.snapshot
                    && snapshot.runtime.status
                        == veyra_core::application::runtime_snapshot::RuntimeStatus::Ready
                {
                    break snapshot.endpoints.unwrap();
                }
            }
            assert!(std::time::Instant::now() < until, "worker Ready timeout");
            thread::sleep(Duration::from_millis(30));
        };
        assert!(root.join("traffic/traffic.sqlite3").exists());
        assert!(
            TrafficWriter::open(&root.join("traffic"), "UTC".parse().unwrap()).is_err(),
            "one active statistics writer"
        );
        service
            .shutdown()
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .unwrap();
        drop(service);
        assert!(std::net::TcpStream::connect(endpoints.mixed).is_err());
        assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
        TrafficWriter::open(&root.join("traffic"), "UTC".parse().unwrap())
            .unwrap()
            .close()
            .unwrap();
        eprintln!(
            "P3_04_REAL_QUIT PASS RuntimeService.shutdown completed; real child ports and SQLite lease released"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn incomplete_owner_quit_reports_unknown_and_manual_start_cannot_create_writer() {
        // 私有root模拟持久半交接；worker不能用空本地Port发布Stopped或绕过fence启动。
        use super::*;
        use veyra_core::{
            application::{runtime_recovery::*, state_access::StateAccessGate},
            domain::{AppState, StateEpoch},
            storage::JsonStateStore,
        };
        let root =
            std::env::temp_dir().join(format!("p206-quit-{:?}", StateEpoch::fresh().unwrap()));
        std::fs::create_dir(&root).unwrap();
        let state = AppState::empty();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        std::fs::create_dir(root.join("runtime")).unwrap();
        let record = serde_json::to_vec(&OwnerTransfer::Closed {
            ticket: TransferTicket {
                id: StateEpoch::fresh().unwrap(),
                version: state.version(),
                digest: "0".repeat(64),
            },
        })
        .unwrap();
        let path = root.join("runtime/owner-transfer.json");
        std::fs::write(&path, &record).unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        let (events, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots, events);
        service.submit(1, RuntimeCommand::Start);
        assert_eq!(
            service
                .shutdown()
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(RuntimeError::StopFailed)
        );
        drop(service);
        let mut errors = 0;
        let mut last = None;
        while let Ok(AppEvent::Runtime(event)) = rx.try_recv() {
            last = Some(event.snapshot.clone());
            if matches!(event.result, Some(Err(_))) {
                errors += 1;
            }
            assert!(!matches!(
                event.result,
                Some(Ok(RuntimeResult::Started | RuntimeResult::Stopped))
            ));
        }
        assert!(errors >= 2);
        assert_eq!(
            last.unwrap().unwrap().runtime.status,
            veyra_core::application::runtime_snapshot::RuntimeStatus::Recovering
        );
        assert_eq!(std::fs::read(&path).unwrap(), record);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod logs_native_tests {
    use super::*;
    use std::io::{Read, Write};
    use veyra_core::{
        application::{
            observability::{
                controller::{LogDelivery, ObservationReader},
                logs::Buffer,
            },
            runtime_snapshot::RuntimeStatus,
        },
        domain::{AppState, RouteTarget, SubscriptionId},
        storage::JsonStateStore,
    };
    fn fixture_state() -> AppState {
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../veyra-core/tests/fixtures/compiler/p2-02b.json"
        ))
        .unwrap();
        for (k, v) in fixture.as_object().unwrap() {
            value[k] = v.clone();
        }
        let mut state: AppState = serde_json::from_value(value).unwrap();
        state.active_subscription_id = Some(SubscriptionId("subscription".into()));
        state.default_target = RouteTarget::Direct;
        state.routes.clear();
        state.pools.retain(|p| p.id.0 == "manual");
        state.validate().unwrap();
        state.app_config.visual.theme_mode = veyra_core::domain::DesktopThemeMode::Light;
        state
    }
    /// 只准备自己指定的隔离GUI目录；真实操作仍由产品GPUI发起。
    #[test]
    #[ignore = "local GUI fixture setup only; not acceptance evidence"]
    fn logs_prepare_owned_gui_fixture() {
        let root = std::path::PathBuf::from(
            std::env::var_os("VEYRA_P303_GUI_ROOT").expect("owned GUI root"),
        );
        assert!(root.is_absolute() && !root.join("state.json").exists());
        JsonStateStore::new(root.join("state.json"))
            .unwrap()
            .commit(&fixture_state())
            .unwrap();
    }
    fn wait(
        reader: &ObservationReader,
        predicate: impl Fn(&veyra_core::application::observability::controller::LogStatus) -> bool,
    ) {
        let until = std::time::Instant::now() + Duration::from_secs(4);
        loop {
            if reader.log_status().as_ref().is_some_and(&predicate) {
                return;
            }
            assert!(
                std::time::Instant::now() < until,
                "real log delivery deadline"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
    // 保护正式AppServices→Runtime唯一/logs→页面Buffer→真实文件导出与Replace/Quit清理。
    #[test]
    #[ignore = "requires pinned sing-box 1.14.0; real local managed logs"]
    fn logs_real_services_pause_export_replace_and_quit() {
        let root = std::env::temp_dir().join(format!(
            "veyra-logs-native-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let state = fixture_state();
        JsonStateStore::new(root.join("state.json"))
            .unwrap()
            .commit(&state)
            .unwrap();
        let (services, mut events) = crate::services::AppServices::new(root.clone()).unwrap();
        services.manual_runtime.submit(1, RuntimeCommand::Start);
        fn ready(
            events: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
        ) -> veyra_core::singbox::runtime::ManagedRuntimeEndpoints {
            let until = std::time::Instant::now() + Duration::from_secs(20);
            loop {
                if let Ok(AppEvent::Runtime(event)) = events.try_recv() {
                    if let Some(Err(_)) = event.result {
                        panic!("owned runtime command failed")
                    }
                    if let Ok(snapshot) = event.snapshot
                        && snapshot.runtime.status == RuntimeStatus::Ready
                    {
                        return snapshot.endpoints.unwrap();
                    }
                }
                assert!(std::time::Instant::now() < until, "owned Ready deadline");
                thread::sleep(Duration::from_millis(20));
            }
        }
        let first = ready(&mut events);
        let reader = services.manual_runtime.observation_reader().unwrap();
        wait(&reader, |s| s.available);
        let mut subscription = reader.subscribe_logs().unwrap();
        let mut buffer = Buffer::default();
        buffer.set_identity(reader.log_status().unwrap().identity);
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let target = listener.local_addr().unwrap();
        let peer = thread::spawn(move || {
            for _ in 0..3 {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(4)))
                    .unwrap();
                let mut request = [0; 4096];
                assert!(socket.read(&mut request).unwrap() > 0);
                socket.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
            }
        });
        let request = || {
            let before = reader.log_status().unwrap().sequence;
            let mut c = std::net::TcpStream::connect(first.mixed).unwrap();
            c.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
            write!(
                c,
                "GET http://{target}/logs HTTP/1.1\r\nHost: {target}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut response = Vec::new();
            c.read_to_end(&mut response).unwrap();
            assert!(response.starts_with(b"HTTP/1.1 204"));
            wait(&reader, |s| s.sequence > before);
        };
        request();
        while let Some(message) = subscription.try_recv() {
            if let LogDelivery::Record(record) = message {
                buffer.push(record);
            }
        }
        assert!(!buffer.is_empty());
        assert!(buffer.export().contains("127.0.0.1"));
        let initial = buffer.len();
        buffer.paused = true;
        request();
        subscription.discard_pending();
        buffer.paused = false;
        assert_eq!(buffer.len(), initial);
        request();
        while let Some(message) = subscription.try_recv() {
            if let LogDelivery::Record(record) = message {
                buffer.push(record);
            }
        }
        assert!(buffer.len() > initial);
        peer.join().unwrap();
        buffer.query("outbound.*127\\.0\\.0\\.1");
        let export = buffer.export();
        assert!(!export.is_empty());
        let path = root.join("filtered.log");
        assert_eq!(
            services
                .save_log_export(
                    Ok(crate::platform::macos::DialogResult::Selected(path.clone())),
                    export.as_bytes().to_vec()
                )
                .blocking_recv()
                .unwrap(),
            Ok(true)
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), export);
        buffer.clear();
        assert!(buffer.is_empty());
        assert!(reader.log_status().unwrap().received);
        let old = reader.log_status().unwrap().identity;
        services.manual_runtime.submit(2, RuntimeCommand::Restart);
        let second = ready(&mut events);
        wait(&reader, |s| s.available && s.identity != old);
        assert_ne!(reader.log_status().unwrap().identity, old);
        buffer.set_identity(reader.log_status().unwrap().identity);
        while let Some(message) = subscription.try_recv() {
            if let LogDelivery::Record(record) = message {
                assert!(
                    !buffer.push(record.clone())
                        || Some(record.identity.clone()) == buffer.identity
                );
            }
        }
        services
            .manual_runtime
            .shutdown()
            .recv_timeout(Duration::from_secs(15))
            .unwrap()
            .unwrap();
        assert_eq!(reader.log_status().unwrap().identity, None);
        for address in [
            first.mixed,
            first.controller,
            second.mixed,
            second.controller,
        ] {
            assert!(std::net::TcpStream::connect(address).is_err());
        }
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
        eprintln!(
            "P3_03_REAL_LOGS PASS: current managed /logs, page pause/export, replacement fence, Quit ports/root cleanup"
        );
    }
}

/// P2-06 原 owner 确认：Helper 尚无 Group 选择/编排协议，桌面正式变更链明确拒绝。
#[cfg(target_os = "macos")]
fn helper_group_admission(
    snapshots: &SnapshotService,
) -> Result<(), veyra_core::application::helper_protocol::Error> {
    use veyra_core::{application::helper_protocol::Error, domain::GroupRule};
    let state = snapshots.snapshot().map_err(|_| Error::Unavailable)?;
    if !state.group_selections.is_empty()
        || state
            .groups
            .iter()
            .any(|g| g.rule == GroupRule::Failover && g.enabled)
    {
        Err(Error::HandoffRequired)
    } else {
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod p403_helper_admission_tests {
    use super::*;
    use veyra_core::{
        application::{
            helper_protocol::{Command, Configuration, Error},
            helper_transfer::TransferDirection,
            state_access::StateAccessGate,
        },
        domain::*,
        storage::{JsonStateStore, StateStore},
    };
    #[test]
    fn p403_helper_rejects_every_mutating_admission_before_freeze_or_rpc() {
        // 保护主备事实不进入尚未实现 Group 事务的 Helper；拒绝不能停 child 或落盘 fence。
        let root = std::env::temp_dir().join(format!(
            "p403-helper-admission-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        let mut state = AppState::empty();
        let mut group = NodeGroup::fresh().unwrap();
        group.rule = GroupRule::Failover;
        group.name = "主备".into();
        group.mode = GroupMode::Static;
        group.members.clear();
        group.lanes = vec![FailoverLane {
            id: "primary".into(),
            name: String::new(),
            icon: String::new(),
            members: vec![OutboundId::Direct],
            manual: true,
        }];
        group.failover = Some(FailoverSettings::default());
        state.groups.push(group);
        state.validate().unwrap();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let (events, _rx) = tokio::sync::mpsc::unbounded_channel();
        let service = RuntimeService::new(root.clone(), snapshots.clone(), events);
        let version = state.version();
        for command in [
            Command::Start {
                request_id: 1,
                expected: version.clone(),
                config: Configuration {
                    state: Box::new(state.clone()),
                },
            },
            Command::Apply {
                request_id: 2,
                instance: 1,
                expected: version.clone(),
                config: Configuration {
                    state: Box::new(state.clone()),
                },
            },
        ] {
            assert_eq!(
                service
                    .helper_request(command)
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap(),
                Err(Error::HandoffRequired)
            );
        }
        for direction in [TransferDirection::ToHelper, TransferDirection::ToDesktop] {
            assert_eq!(
                service
                    .transfer_owner(StateEpoch::fresh().unwrap(), version.clone(), direction)
                    .recv_timeout(Duration::from_secs(2))
                    .unwrap(),
                Err(Error::HandoffRequired)
            );
        }
        assert_eq!(
            service
                .prepare_bootstrap(StateEpoch::fresh().unwrap(), version.clone())
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .commit_bootstrap(version.clone())
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .prepare_rebind(StateEpoch::fresh().unwrap(), version.clone())
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .commit_rebind(version.clone())
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            service
                .freeze_first_bootstrap(
                    StateEpoch::fresh().unwrap(),
                    StateEpoch::fresh().unwrap(),
                    version
                )
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Err(RuntimeError::SelectionPending)
        );
        assert_eq!(
            service
                .owner_transfer()
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Ok(None)
        );
        assert_eq!(store.load().unwrap(), state);
        assert!(!root.join("runtime/owner-transfer.json").exists());
        assert!(!root.join("runtime/owner-incarnation.json").exists());
        // 不改变旧普通池 admission；只读 owner 查询、正常本地 Stop/退出仍可运行。
        let ordinary = AppState::empty();
        snapshots.replace(state.version(), ordinary).unwrap();
        assert!(helper_group_admission(&snapshots).is_ok());
        assert_eq!(
            service
                .shutdown()
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            Ok(())
        );
        drop(service);
        std::fs::remove_dir_all(root).unwrap();
    }
}
