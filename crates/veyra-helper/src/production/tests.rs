//! 全部生命周期是 MOCK；只有 Unix socket/getpeereid/proc_pidinfo 是本机非特权 OS 验证。
//! 不启动 sing-box，不调用安装/网络设置，不读取真实用户业务数据。
use super::{
    host::{Backend, Host, compile_configuration},
    transport::{self, Peer},
    *,
};
use std::{
    fs,
    io::Write,
    os::unix::{
        fs::PermissionsExt,
        net::{UnixListener, UnixStream},
    },
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};
use veyra_core::domain::*;

pub(super) fn config() -> Configuration {
    let mut state = AppState::empty();
    state.active_subscription_id = Some(SubscriptionId("subscription".into()));
    state.subscriptions.push(Subscription {
        id: SubscriptionId("subscription".into()),
        name: "fixture".into(),
        description: String::new(),
        source: SubscriptionSource::Manual,
        document: None,
        skipped_unsupported_nodes: 0,
        last_success_at_ms: None,
        last_attempt_at_ms: None,
        http_metadata: None,
        remote_request: None,
        update_policy: SubscriptionUpdatePolicy::manual(),
    });
    state.providers.push(Provider {
        id: ProviderId("provider".into()),
        subscription_id: SubscriptionId("subscription".into()),
        name: "fixture".into(),
    });
    state.nodes.push(ProxyNode {
        id: NodeId("node".into()),
        provider_id: ProviderId("provider".into()),
        name: "fixture".into(),
        protocol: ProxyProtocol::Shadowsocks,
        server: "example.invalid".into(),
        port: 443,
        options: ProtocolOptions::Shadowsocks {
            method: "aes-128-gcm".into(),
            password: "fixture-sensitive".into(),
        },
        transport: Some(Transport::Tcp),
        tls: None,
    });
    Configuration {
        state: Box::new(state),
    }
}
fn identity() -> Peer {
    let (a, _b) = UnixStream::pair().unwrap();
    transport::peer(&a).unwrap()
}
fn request(command: Command) -> Request {
    Request {
        protocol: PROTOCOL,
        command,
    }
}
fn start(c: &Configuration, id: u64) -> Command {
    Command::Start {
        request_id: id,
        expected: c.version(),
        config: c.clone(),
    }
}
#[derive(Default)]
struct Trace {
    runs: usize,
    stops: usize,
    active: bool,
    fail_ready: bool,
    fail_stop: bool,
    calls: usize,
}
struct Mock {
    trace: Arc<Mutex<Trace>>,
    status: Status,
    release: Option<mpsc::Receiver<()>>,
}
impl Backend for Mock {
    fn poll(&mut self) -> Option<Status> {
        if !self.trace.lock().unwrap().active {
            self.status.instance = None;
            self.status.applied = None;
        }
        Some(self.status.clone())
    }
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            protocol: PROTOCOL,
            host_version: "mock".into(),
            kernel_version: Some("mock".into()),
            required_kernel_version: "mock".into(),
            runtime: Ok(()),
            handoff: Ok(()),
            system_proxy: false,
            resource_bytes: 0,
        }
    }
    fn execute(
        &mut self,
        _peer: Peer,
        command: Command,
        deadline: Instant,
    ) -> (Status, Result<Status, Error>) {
        if let Some(release) = self.release.take() {
            release
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .unwrap();
        }
        let mut trace = self.trace.lock().unwrap();
        trace.calls += 1;
        let error = match command {
            Command::Start { config, .. } | Command::Apply { config, .. } => {
                // 使用真实产品 compiler，Mock 只替代 OS child/Ready/cleanup 这一边界。
                if let Err(error) = compile_configuration(&config, unsafe { libc::geteuid() }) {
                    return (self.status.clone(), Err(error));
                }
                if trace.active {
                    trace.stops += 1;
                    trace.active = false;
                }
                trace.runs += 1;
                if trace.fail_ready {
                    trace.stops += 1;
                    self.status.instance = None;
                    self.status.applied = None;
                    Some(Error::Failed)
                } else {
                    trace.active = true;
                    self.status.instance = Some(trace.runs as u64);
                    self.status.applied = Some(config.version());
                    self.status.last_successful = self.status.applied.clone();
                    None
                }
            }
            Command::Stop { .. } => {
                if trace.fail_stop {
                    self.status.recovery_required = true;
                    Some(Error::Failed)
                } else {
                    trace.active = false;
                    trace.stops += 1;
                    self.status.instance = None;
                    self.status.applied = None;
                    self.status.recovery_required = false;
                    None
                }
            }
            Command::RuntimeCommand {
                command: RuntimeCommand::Observe,
                ..
            } => None,
            _ => Some(Error::Unsupported),
        };
        (
            self.status.clone(),
            error.map_or_else(|| Ok(self.status.clone()), Err),
        )
    }
}
fn host() -> (Host, Arc<Mutex<Trace>>) {
    let trace = Arc::new(Mutex::new(Trace::default()));
    (
        Host::new(
            identity().uid,
            Mock {
                trace: trace.clone(),
                status: Status::default(),
                release: None,
            },
        ),
        trace,
    )
}
fn completed(host: &Host, peer: Peer, id: u64) -> Result<Status, Error> {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Response::Operation(Operation::Completed(result)) =
            host.dispatch(peer, request(Command::Operation { request_id: id }))
        {
            return result;
        }
        assert!(Instant::now() < deadline, "operation not completed");
        thread::sleep(Duration::from_millis(1));
    }
}
fn started(host: &Host, c: &Configuration) -> Status {
    assert_eq!(
        host.dispatch(identity(), request(start(c, 1))),
        Response::Operation(Operation::Inflight)
    );
    completed(host, identity(), 1).unwrap()
}

#[test]
fn hello_without_instance_and_rejects_version_uid_before_effects() {
    // 保护未运行时检查安装能力，以及旧客户端/其他用户不能触发启动。
    let (host, trace) = host();
    let peer = identity();
    assert!(matches!(
        host.dispatch(peer, request(Command::Hello)),
        Response::Hello(_)
    ));
    let mut wrong = request(start(&config(), 1));
    wrong.protocol += 1;
    assert_eq!(
        host.dispatch(peer, wrong),
        Response::Rejected(Error::IncompatibleVersion)
    );
    for uid in [0, peer.uid + 1] {
        assert_eq!(
            host.dispatch(Peer { uid, ..peer }, request(Command::Hello)),
            Response::Rejected(Error::Unauthorized)
        );
    }
    assert_eq!(trace.lock().unwrap().calls, 0);
}
#[test]
fn duplicate_start_one_child_and_payload_conflict() {
    // 保护重复点击、请求重传不会制造第二个 writer。
    let (host, trace) = host();
    let c = config();
    let active = started(&host, &c);
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 1))),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(completed(&host, identity(), 1), Ok(active.clone()));
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 2))),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(completed(&host, identity(), 2), Ok(active));
    let mut same_version = c.clone();
    same_version.state.nodes[0].name = "changed".into();
    assert_eq!(
        host.dispatch(identity(), request(start(&same_version, 3))),
        Response::Rejected(Error::VersionConflict)
    );
    let mut changed = c.clone();
    changed.state.config_revision += 1;
    assert_eq!(
        host.dispatch(identity(), request(start(&changed, 1))),
        Response::Rejected(Error::RequestConflict)
    );
    assert_eq!(trace.lock().unwrap().runs, 1);
}
#[test]
fn process_session_cannot_stop_or_query_another_session_operation() {
    // 保护另一进程、PID 重用者不能停止现有实例或取用去重结果。
    let (host, trace) = host();
    let active = started(&host, &config());
    let peer = identity();
    for other in [
        Peer {
            pid: peer.pid + 1,
            ..peer
        },
        Peer {
            start: (0, 0),
            ..peer
        },
    ] {
        assert_eq!(
            host.dispatch(
                other,
                request(Command::Stop {
                    request_id: 2,
                    instance: active.instance.unwrap()
                })
            ),
            Response::Rejected(Error::StaleInstance)
        );
        assert_eq!(
            host.dispatch(other, request(Command::Operation { request_id: 1 })),
            Response::Operation(Operation::Unknown)
        );
    }
    assert_eq!(trace.lock().unwrap().stops, 0);
}
#[test]
fn complete_epoch_config_selection_cas_guards_apply_and_controller() {
    // 保护陈旧页面和旧 epoch 无法覆盖当前运行状态或写控制器。
    let (host, trace) = host();
    let c = config();
    let active = started(&host, &c);
    for part in 0..3 {
        let mut expected = c.version();
        match part {
            0 => expected.config.0.epoch = StateEpoch::fresh().unwrap(),
            1 => expected.config.0.revision += 1,
            _ => expected.selection.0.revision += 1,
        }
        let command = Command::RuntimeCommand {
            request_id: 10 + part,
            instance: active.instance.unwrap(),
            expected,
            command: RuntimeCommand::Observe,
        };
        assert_eq!(
            host.dispatch(identity(), request(command)),
            Response::Rejected(Error::VersionConflict)
        );
    }
    let mut newer = c.clone();
    newer.state.config_revision += 1;
    assert_eq!(
        host.dispatch(
            identity(),
            request(Command::Apply {
                request_id: 5,
                instance: active.instance.unwrap(),
                expected: c.version(),
                config: newer.clone()
            })
        ),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(
        completed(&host, identity(), 5).unwrap().applied,
        Some(newer.version())
    );
    assert_eq!(trace.lock().unwrap().runs, 2);
}
#[test]
fn failed_ready_and_stop_report_actual_cleanup_not_success() {
    // 保护失败后 UI 不出现假 Ready；停止失败保留实际实例与恢复要求。
    let (host, trace) = host();
    let c = config();
    trace.lock().unwrap().fail_ready = true;
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 1))),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(completed(&host, identity(), 1), Err(Error::Failed));
    assert_eq!(
        host.dispatch(identity(), request(Command::Status)),
        Response::Status(Status::default())
    );
    assert!(!trace.lock().unwrap().active);
    trace.lock().unwrap().fail_ready = false;
    host.dispatch(identity(), request(start(&c, 2)));
    completed(&host, identity(), 2).unwrap();
    let active = completed(&host, identity(), 2).unwrap();
    trace.lock().unwrap().fail_stop = true;
    host.dispatch(
        identity(),
        request(Command::Stop {
            request_id: 3,
            instance: active.instance.unwrap(),
        }),
    );
    assert_eq!(completed(&host, identity(), 3), Err(Error::Failed));
    assert!(trace.lock().unwrap().active);
    trace.lock().unwrap().fail_stop = false;
    host.dispatch(
        identity(),
        request(Command::Stop {
            request_id: 4,
            instance: active.instance.unwrap(),
        }),
    );
    assert!(completed(&host, identity(), 4).unwrap().instance.is_none());
    assert!(!trace.lock().unwrap().active);
}
#[test]
fn cache_expiry_and_lru_unknown_require_status() {
    // 保护有界内存；过期不是“操作失败”，实际 child 仍应能从 Status 查到。
    let (mut host, _) = host();
    host.capacity = 2;
    let c = config();
    let active = started(&host, &c);
    for id in 2..=3 {
        host.dispatch(
            identity(),
            request(Command::RuntimeCommand {
                request_id: id,
                instance: active.instance.unwrap(),
                expected: c.version(),
                command: RuntimeCommand::Observe,
            }),
        );
        completed(&host, identity(), id).unwrap();
    }
    assert_eq!(
        host.dispatch(identity(), request(Command::Operation { request_id: 1 })),
        Response::Operation(Operation::Unknown)
    );
    host.retain = Duration::ZERO;
    assert_eq!(
        host.dispatch(identity(), request(Command::Operation { request_id: 3 })),
        Response::Operation(Operation::Unknown)
    );
    assert_eq!(
        host.dispatch(identity(), request(Command::Status)),
        Response::Status(active)
    );
}
#[test]
fn events_bounded_and_proxy_interface_never_writes_os() {
    // 保护慢观察者不能无限积累事件；P2-07 之前系统代理始终不支持。
    let (host, _) = host();
    let c = config();
    let active = started(&host, &c);
    let instance = active.instance.unwrap();
    assert_eq!(
        host.dispatch(
            identity(),
            request(Command::RuntimeEvent {
                instance,
                after: 0,
                limit: EVENT_BATCH + 1
            })
        ),
        Response::Rejected(Error::TooLarge)
    );
    for id in 2..=EVENT_CAPACITY as u64 + 2 {
        host.dispatch(
            identity(),
            request(Command::RuntimeCommand {
                request_id: id,
                instance,
                expected: c.version(),
                command: RuntimeCommand::Observe,
            }),
        );
        completed(&host, identity(), id).unwrap();
    }
    let response = host.dispatch(
        identity(),
        request(Command::RuntimeEvent {
            instance,
            after: 0,
            limit: EVENT_BATCH,
        }),
    );
    let Response::Events { events, gap } = &response else {
        panic!("events required")
    };
    assert!(gap);
    assert_eq!(events.len(), EVENT_BATCH);
    assert!(serde_json::to_vec(&response).unwrap().len() < CONTROL_BYTES);
    for command in [
        Command::ApplySystemProxy {
            request_id: 500,
            instance,
        },
        Command::RestoreSystemProxy {
            request_id: 501,
            instance,
        },
    ] {
        assert_eq!(
            host.dispatch(identity(), request(command)),
            Response::Rejected(Error::Unsupported)
        );
    }
}

pub(super) struct SocketFixture {
    path: std::path::PathBuf,
    pub(super) listener: UnixListener,
}
impl SocketFixture {
    pub(super) fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "v206-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        let listener = UnixListener::bind(path.join("control.sock")).unwrap();
        fs::set_permissions(path.join("control.sock"), fs::Permissions::from_mode(0o660)).unwrap();
        Self { path, listener }
    }
    pub(super) fn client(&self) -> Client {
        Client::isolated(self.path.join("control.sock"))
    }
}
impl Drop for SocketFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).unwrap();
    }
}
#[test]
fn os_peer_real_uid_pid_and_protected_socket_validation() {
    // OS隔离：真实 getpeereid/LOCAL_PEERPID/启动时间，不将注入的 Peer 说成认证证据。
    let peer = identity();
    assert_eq!(peer.uid, unsafe { libc::geteuid() });
    assert_eq!(peer.pid, std::process::id() as i32);
    assert!(peer.start.0 > 0);
    let fixture = SocketFixture::new();
    let path = fixture.path.join("control.sock");
    assert!(transport::socket_metadata(&path, peer.uid, peer.gid, 0o660).is_ok());
    assert_eq!(
        transport::socket_metadata(&path, peer.uid + 1, peer.gid, 0o660),
        Err(Error::UnsafeEndpoint)
    );
    fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
    assert_eq!(
        fixture.client().request(Command::Hello),
        Err(Error::UnsafeEndpoint)
    );
    assert_eq!(
        transport::protected_directories(&fixture.path),
        Err(Error::UnsafeEndpoint)
    );
    let link = fixture.path.join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert_eq!(
        transport::socket_metadata(&link, peer.uid, peer.gid, 0o666),
        Err(Error::UnsafeEndpoint)
    );
    let file = fixture.path.join("file");
    fs::write(&file, []).unwrap();
    assert_eq!(
        transport::socket_metadata(&file, peer.uid, peer.gid, 0o660),
        Err(Error::UnsafeEndpoint)
    );
}
#[test]
fn unix_disconnect_reconnect_operation_retains_inflight_and_completed() {
    // OS隔离+MOCK：客户端断开后业务继续；新连接的真实同进程身份查询同一操作。
    let fixture = SocketFixture::new();
    let (tx, rx) = mpsc::channel();
    let trace = Arc::new(Mutex::new(Trace::default()));
    let host = Arc::new(Host::new(
        identity().uid,
        Mock {
            trace: trace.clone(),
            status: Status::default(),
            release: Some(rx),
        },
    ));
    let listener = fixture.listener.try_clone().unwrap();
    let server = host.clone();
    let thread = thread::spawn(move || {
        for stream in listener.incoming().take(4) {
            let _ = server.connection(stream.unwrap());
        }
    });
    let client = fixture.client();
    assert_eq!(
        client.request(start(&config(), 1)).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(
        client
            .request(Command::Operation { request_id: 1 })
            .unwrap(),
        Response::Operation(Operation::Inflight)
    );
    tx.send(()).unwrap();
    let active = completed(&host, identity(), 1).unwrap();
    assert_eq!(
        client
            .request(Command::Operation { request_id: 1 })
            .unwrap(),
        Response::Operation(Operation::Completed(Ok(active.clone())))
    );
    assert_eq!(
        client.request(Command::Status).unwrap(),
        Response::Status(active)
    );
    thread.join().unwrap();
    assert_eq!(trace.lock().unwrap().runs, 1);
}
fn decode(bytes: &[u8], kind: u8) -> Result<Request, Error> {
    let (mut sender, mut receiver) = UnixStream::pair().unwrap();
    sender.write_all(&[kind]).unwrap();
    sender
        .write_all(&(bytes.len() as u32).to_be_bytes())
        .unwrap();
    sender.write_all(bytes).unwrap();
    drop(sender);
    transport::read_request(&mut receiver)
}
#[test]
fn closed_frames_reject_path_pid_command_unknown_fields_and_oversize() {
    // 保护高权限入口：不能通过顶层或嵌套字段夹带不受控参数，超限头不分配/执行。
    let good = serde_json::to_value(request(start(&config(), 1))).unwrap();
    for field in ["path", "pid", "executable", "command", "owner", "timeout"] {
        let mut value = good.clone();
        value["command"][field] = serde_json::json!("arbitrary");
        assert!(decode(&serde_json::to_vec(&value).unwrap(), 2).is_err());
    }
    let mut nested = good.clone();
    nested["command"]["config"]["state"]["path"] = serde_json::json!("/etc/passwd");
    assert!(decode(&serde_json::to_vec(&nested).unwrap(), 2).is_err());
    assert!(decode(br#"{"protocol":1,"command":{"kind":"RunShell"}}"#, 1).is_err());
    assert!(decode(b"not json", 1).is_err());
    assert!(decode(&serde_json::to_vec(&good).unwrap(), 1).is_err());
    assert!(decode(&serde_json::to_vec(&good).unwrap(), 2).is_ok());
    let (mut sender, mut receiver) = UnixStream::pair().unwrap();
    sender.write_all(&[1]).unwrap();
    sender
        .write_all(&((CONTROL_BYTES + 1) as u32).to_be_bytes())
        .unwrap();
    assert!(matches!(
        transport::read_request(&mut receiver),
        Err(Error::TooLarge)
    ));
}
#[test]
fn config_limits_redaction_and_product_compiler_are_real() {
    // 保护业务配置不会成为 opaque sing-box/path 通道，也不泄露节点凭据。
    let c = config();
    let plan = compile_configuration(&c, identity().uid).unwrap();
    assert!(plan.artifact_index().is_some());
    assert!(!format!("{:?}", request(start(&c, 1))).contains("fixture-sensitive"));
    let mut huge = c.clone();
    huge.state.nodes[0].name = "x".repeat(CONFIG_BYTES);
    assert_eq!(huge.validate(), Err(Error::TooLarge));
    let mut invalid = c;
    invalid.state.nodes[0].port = 0;
    assert!(matches!(
        compile_configuration(&invalid, identity().uid),
        Err(Error::InvalidConfiguration)
    ));
}

#[test]
fn repeat_noop_start_retained_after_stop() {
    // 保护后续Stop后旧Start重传不重新开child，也不重放历史Ready；Operation先保留历史。
    let (mut host, trace) = host();
    host.capacity = 3;
    let c = config();
    let active = started(&host, &c);
    host.dispatch(identity(), request(start(&c, 2)));
    completed(&host, identity(), 2).unwrap();
    host.dispatch(
        identity(),
        request(Command::Stop {
            request_id: 3,
            instance: active.instance.unwrap(),
        }),
    );
    completed(&host, identity(), 3).unwrap();
    assert_eq!(completed(&host, identity(), 2), Ok(active));
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 2))),
        Response::Operation(Operation::Completed(Err(Error::StaleInstance)))
    );
    assert_eq!(trace.lock().unwrap().runs, 1);
}
#[test]
fn partial_frame_times_out_and_closed_peer_buffer_is_drained() {
    // 保护慢/中断连接不会永久占用 worker；保留 P0-05 HUP 缓冲读取修复。
    let bytes = serde_json::to_vec(&request(Command::Hello)).unwrap();
    assert!(decode(&bytes, 1).is_ok());
    let (mut sender, mut receiver) = UnixStream::pair().unwrap();
    sender.write_all(&[1, 0]).unwrap();
    let at = Instant::now();
    assert!(matches!(
        transport::read_request(&mut receiver),
        Err(Error::Timeout)
    ));
    assert!(at.elapsed() < Duration::from_secs(3));
}
#[test]
fn pending_selection_fails_before_worker_or_writer() {
    // 保护 P2-04 pending 意图：未对账不能被跨 owner 新启动清空。
    let (host, trace) = host();
    let mut c = config();
    c.state.pools.push(NodePool {
        id: PoolId("pool".into()),
        name: "pool".into(),
        enabled: true,
        kind: PoolKind::Custom,
        sources: vec![PoolSource {
            provider_id: ProviderId("provider".into()),
            filter: NodeFilter::default(),
        }],
        selection: SelectionPolicy::Manual {
            selected_node_id: None,
            pending_node_id: Some(NodeId("node".into())),
        },
    });
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 1))),
        Response::Rejected(Error::HandoffRequired)
    );
    assert_eq!(trace.lock().unwrap().calls, 0);
}

#[test]
fn new_start_id_checks_owner_liveness_instead_of_cached_ready() {
    // 保护250ms轮询窗口：旧Ready尚在缓存，但owner同步核验已退出，不能回报AlreadyRunning或开第二child。
    let (host, trace) = host();
    // 只暂停测试的空闲poll，使窗口确定存在；请求路径的同步核验不受此开关影响。
    host.idle_poll
        .store(false, std::sync::atomic::Ordering::Release);
    let c = config();
    let old = started(&host, &c);
    trace.lock().unwrap().active = false;
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 99))),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(completed(&host, identity(), 99), Err(Error::StaleInstance));
    assert_eq!(trace.lock().unwrap().runs, 1);
    // Operation只读历史；Start重传必须拒绝已确认退出的实例。
    assert_eq!(completed(&host, identity(), 1), Ok(old));
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 1))),
        Response::Operation(Operation::Completed(Err(Error::StaleInstance)))
    );
}

#[test]
fn repeated_start_verification_does_not_extend_result_ttl() {
    // 保护断线重试不能把有界操作缓存变成永久journal；过期必须UNKNOWN并核对Status。
    let (mut host, trace) = host();
    host.retain = Duration::from_millis(250);
    let c = config();
    let active = started(&host, &c);
    thread::sleep(Duration::from_millis(160));
    assert_eq!(
        host.dispatch(identity(), request(start(&c, 1))),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(completed(&host, identity(), 1), Ok(active));
    thread::sleep(Duration::from_millis(120));
    assert_eq!(
        host.dispatch(identity(), request(Command::Operation { request_id: 1 })),
        Response::Operation(Operation::Unknown)
    );
    assert!(matches!(
        host.dispatch(identity(), request(Command::Status)),
        Response::Status(Status {
            instance: Some(_),
            ..
        })
    ));
    assert_eq!(trace.lock().unwrap().runs, 1);
}
