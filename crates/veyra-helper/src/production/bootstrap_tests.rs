//! 非特权root角色fixture：真实Unix peer/文件权限/lease；不把Mock Port当真实内核。
use super::{
    assets::Assets,
    backend::InstalledBackend,
    host::{Backend, Host},
    transport::Peer,
    *,
};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, sync::Arc};
use veyra_core::{
    application::{
        manual_runtime::ManualRuntime, state_access::StateAccessGate,
        state_service::SnapshotService,
    },
    domain::StateEpoch,
    singbox::{
        GeneratedConfig,
        runtime::{ManagedSidecar, SidecarPort, SidecarPortError},
    },
    storage::{JsonStateStore, StateStore},
};
struct NoChild;
impl SidecarPort for NoChild {
    fn check(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
        panic!("bootstrap must not start")
    }
    fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
        panic!("bootstrap must not prepare kernel")
    }
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        panic!("bootstrap must not run")
    }
    fn ready(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
        panic!("no child")
    }
    fn stop(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
        panic!("no child")
    }
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        Ok(())
    }
    fn has_pending_cleanup(&self) -> bool {
        false
    }
}
struct Fixture {
    root: PathBuf,
    peer: Peer,
    config: Configuration,
    source: ManualRuntime<NoChild>,
    backend: InstalledBackend,
    primary: Option<PrimaryFixture>,
}
impl Fixture {
    fn new() -> Self {
        let nonce = veyra_core::application::runtime_recovery::digest(
            &serde_json::to_vec(&StateEpoch::fresh().unwrap()).unwrap(),
        );
        let root = PathBuf::from("/private/tmp").join(format!("v16-{}", &nonce[..16]));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let install = root.join("install");
        std::fs::create_dir(&install).unwrap();
        std::fs::set_permissions(&install, std::fs::Permissions::from_mode(0o700)).unwrap();
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let peer = transport::peer(&a).unwrap();
        super::install::record_installation(&install, peer.uid, peer.gid).unwrap();
        let config = super::tests::config();
        let store = JsonStateStore::new(root.join("desktop/state.json")).unwrap();
        store.save(&config.state).unwrap();
        let source = ManualRuntime::new(
            NoChild,
            Arc::new(SnapshotService::new(store, StateAccessGate::default())),
            root.join("desktop"),
            true,
        );
        let primary = Some(PrimaryFixture::new(&root.join("desktop")));
        let mut result = Self {
            primary,
            root,
            peer,
            config,
            source,
            backend: InstalledBackend::from_assets(Err(Error::Unavailable)),
        };
        result.backend = result.backend();
        result
    }
    fn backend(&self) -> InstalledBackend {
        let install = self.root.join("install");
        let mut b = InstalledBackend::from_assets(Ok(Assets {
            kernel: if install.join("veyra-sing-box").exists() {
                install.join("veyra-sing-box")
            } else {
                std::env::current_exe().unwrap()
            },
            root: install.join("runtime"),
            uid: self.peer.uid,
            gid: self.peer.gid,
            installation: Some(
                super::install::installed_identity(&install, self.peer.uid, self.peer.gid).unwrap(),
            ),
            fixture: true,
            spawned: Default::default(),
        }));
        b.require_test_source(self.root.join("desktop"));
        b.test_admin_root(install);
        b
    }
    fn preflight(&mut self) -> BootstrapTicket {
        self.backend
            .bootstrap(
                self.peer,
                BootstrapAction::Preflight {
                    id: StateEpoch::fresh().unwrap(),
                    expected: self.config.version(),
                    config: self.config.clone(),
                },
            )
            .1
            .unwrap()
            .ticket
    }
    fn freeze(&mut self, t: &BootstrapTicket) {
        self.source
            .freeze_first_bootstrap(t.id.clone(), t.installation.clone(), t.version.clone())
            .unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.backend = InstalledBackend::from_assets(Err(Error::Unavailable));
        drop(self.primary.take());
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn rpc(host: &Host, socket: &super::tests::SocketFixture, command: Command) -> Response {
    std::thread::scope(|scope| {
        let connection = scope.spawn(|| {
            let (stream, _) = socket.listener.accept().unwrap();
            host.connection(stream)
        });
        let result = socket.client().request(command).unwrap();
        connection.join().unwrap().unwrap();
        result
    })
}
#[test]
fn bootstrap_preflight_is_read_only_and_checks_generation_history_and_version() {
    // 用户目标不可接入时不freeze Desktop、不创建root运行资料或锁。
    let mut f = Fixture::new();
    let first = f.preflight();
    let next = f.preflight();
    assert_eq!(first.installation, next.installation);
    assert!(!f.root.join("install/runtime").exists());
    assert_eq!(f.source.owner_transfer().unwrap(), None);
    let mut stale = f.config.version();
    stale.selection.0.revision += 1;
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Preflight {
                    id: first.id.clone(),
                    expected: stale,
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::VersionConflict)
    );
    std::fs::create_dir(f.root.join("install/runtime")).unwrap();
    std::fs::set_permissions(
        f.root.join("install/runtime"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    std::fs::write(f.root.join("install/runtime/unknown-owner"), b"retain").unwrap();
    assert!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Preflight {
                    id: first.id,
                    expected: f.config.version(),
                    config: f.config.clone()
                }
            )
            .1
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.root.join("install/runtime/unknown-owner")).unwrap(),
        b"retain"
    );
}
#[test]
fn bootstrap_real_socket_core_freeze_prepare_retry_query_never_starts() {
    // 真正Client→Unix peer→Host worker→Backend→固定source读取；Core真实持久freeze，Port为禁止运行Mock。
    let mut f = Fixture::new();
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = Host::new(f.peer.uid, backend);
    let socket = super::tests::SocketFixture::new();
    let id = StateEpoch::fresh().unwrap();
    let expected = f.config.version();
    // root完成写入后，模拟调用方丢失Prepare ACK；下一请求仍走真实socket查询/重试。
    let result = veyra_core::application::helper_transfer::prepare_bootstrap(
        &mut f.source,
        id.clone(),
        expected.clone(),
        |action| {
            let prepare = matches!(action, BootstrapAction::Prepare { .. });
            match rpc(
                &host,
                &socket,
                Command::Bootstrap {
                    action: Box::new(action),
                },
            ) {
                Response::Bootstrap(Ok(_)) if prepare => Err(Error::Timeout),
                Response::Bootstrap(r) => r,
                other => panic!("{other:?}"),
            }
        },
    );
    assert_eq!(result, Err(Error::Timeout));
    assert!(matches!(
        f.source.owner_transfer().unwrap(),
        Some(veyra_core::application::runtime_recovery::OwnerTransfer::BootstrapFrozen { .. })
    ));
    let mut exchange = |action| match rpc(
        &host,
        &socket,
        Command::Bootstrap {
            action: Box::new(action),
        },
    ) {
        Response::Bootstrap(r) => r,
        other => panic!("{other:?}"),
    };
    let reply = veyra_core::application::helper_transfer::prepare_bootstrap(
        &mut f.source,
        id.clone(),
        expected.clone(),
        &mut exchange,
    )
    .unwrap();
    assert_eq!(reply.phase, BootstrapPhase::Prepared);
    let bytes = std::fs::read(f.root.join("install/runtime/bootstrap-prepared.json")).unwrap();
    assert_eq!(
        veyra_core::application::helper_transfer::prepare_bootstrap(
            &mut f.source,
            id,
            expected,
            &mut exchange
        )
        .unwrap(),
        reply
    );
    assert_eq!(
        exchange(BootstrapAction::Query {
            ticket: reply.ticket.clone()
        })
        .unwrap(),
        reply
    );
    assert_eq!(
        std::fs::read(f.root.join("install/runtime/bootstrap-prepared.json")).unwrap(),
        bytes
    );
    let mut changed_config = f.config.clone();
    changed_config.state.nodes[0].name.push_str(" changed");
    assert_eq!(
        exchange(BootstrapAction::Prepare {
            ticket: reply.ticket.clone(),
            config: changed_config
        }),
        Err(Error::RequestConflict)
    );
    assert!(!f.root.join("install/runtime/received-state.json").exists());
    assert_eq!(
        super::writer_lease::WriterLease::preflight(&f.root.join("install/runtime")),
        Err(Error::Busy)
    );
    let start = host.dispatch(
        f.peer,
        Request {
            protocol: PROTOCOL,
            command: Command::Start {
                request_id: 42,
                expected: f.config.version(),
                config: f.config.clone(),
            },
        },
    );
    assert!(matches!(
        start,
        Response::Rejected(Error::HandoffRequired) | Response::Operation(_)
    ));
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        match host.dispatch(
            f.peer,
            Request {
                protocol: PROTOCOL,
                command: Command::Operation { request_id: 42 },
            },
        ) {
            Response::Operation(Operation::Completed(Err(Error::HandoffRequired))) => break,
            Response::Operation(Operation::Inflight) => {
                assert!(std::time::Instant::now() < until);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            other => panic!("unexpected bootstrap Start result: {other:?}"),
        }
    }
    drop(host); // 模拟宿主丢slot；持久记录不变，重开只可诊断
    f.backend = f.backend();
    let query = f
        .backend
        .bootstrap(
            f.peer,
            BootstrapAction::Query {
                ticket: reply.ticket.clone(),
            },
        )
        .1
        .unwrap();
    assert_eq!(query.phase, BootstrapPhase::RecoveryRequired);
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Prepare {
                    ticket: reply.ticket,
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    assert_eq!(
        std::fs::read(f.root.join("install/runtime/bootstrap-prepared.json")).unwrap(),
        bytes
    );
}
#[test]
fn bootstrap_rejects_partial_freeze_forged_session_and_changed_ticket() {
    // Host finding：只有incarnation而无owner-transfer，不能被Prepare补成授权。
    let mut f = Fixture::new();
    let ticket = f.preflight();
    f.freeze(&ticket);
    let path = f.root.join("desktop/runtime/owner-transfer.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Prepare {
                    ticket: ticket.clone(),
                    config: f.config.clone()
                }
            )
            .1
            .is_err()
    );
    assert!(!f.root.join("install/runtime").exists());
    assert!(
        f.root
            .join("desktop/runtime/owner-incarnation.json")
            .exists()
    );
    std::fs::write(&path, &bytes).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
    let session_path = f.root.join("desktop/runtime/source-session.json");
    let mut session: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&session_path).unwrap()).unwrap();
    session["pid"] = serde_json::json!(0);
    std::fs::write(&session_path, serde_json::to_vec(&session).unwrap()).unwrap();
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Prepare {
                    ticket: ticket.clone(),
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::Unauthorized)
    );
    f.freeze(&ticket); // 同incarnation重试重新发布OS来源绑定
    let mut wrong = ticket.clone();
    wrong.installation = StateEpoch::fresh().unwrap();
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Prepare {
                    ticket: wrong,
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::VersionConflict)
    );
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Prepare {
                ticket: ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    let mut wrong_peer = f.peer;
    wrong_peer.start.1 += 1;
    assert!(
        f.backend
            .bootstrap(wrong_peer, BootstrapAction::Query { ticket })
            .1
            .is_err()
    );
}

#[test]
fn bootstrap_drain_missing_installation_lease_and_invalid_frame_reject_without_freeze() {
    // 安装撤销、安装记录缺失、旧writer与协议输入错误都不能诱使Desktop放弃本地owner。
    let mut f = Fixture::new();
    super::admin::request(&f.root.join("install")).unwrap();
    let action = BootstrapAction::Preflight {
        id: StateEpoch::fresh().unwrap(),
        expected: f.config.version(),
        config: f.config.clone(),
    };
    assert_eq!(
        f.backend.bootstrap(f.peer, action.clone()).1,
        Err(Error::HandoffRequired)
    );
    assert!(!f.root.join("install/uninstall-ack").exists());
    assert!(!f.root.join("install/runtime").exists());
    std::fs::remove_file(f.root.join("install/uninstall-request")).unwrap(); // 仅移除测试自己创建的marker
    let bytes = std::fs::read(f.root.join("install/installation.json")).unwrap();
    std::fs::remove_file(f.root.join("install/installation.json")).unwrap();
    assert_eq!(
        f.backend.bootstrap(f.peer, action.clone()).1,
        Err(Error::HandoffRequired)
    );
    std::fs::write(f.root.join("install/installation.json"), bytes).unwrap();
    std::fs::set_permissions(
        f.root.join("install/installation.json"),
        std::fs::Permissions::from_mode(0o600),
    )
    .unwrap();
    std::fs::create_dir(f.root.join("install/runtime")).unwrap();
    std::fs::set_permissions(
        f.root.join("install/runtime"),
        std::fs::Permissions::from_mode(0o700),
    )
    .unwrap();
    let lease = super::writer_lease::WriterLease::acquire(&f.root.join("install/runtime")).unwrap();
    assert_eq!(
        f.backend.bootstrap(f.peer, action.clone()).1,
        Err(Error::HandoffRequired)
    );
    drop(lease);
    let request = Request {
        protocol: PROTOCOL,
        command: Command::Bootstrap {
            action: Box::new(action),
        },
    };
    let mut value = serde_json::to_value(&request).unwrap();
    value["command"]["action"]["path"] = serde_json::json!("/forbidden");
    assert!(serde_json::from_value::<Request>(value).is_err());
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = Host::new(f.peer.uid, backend);
    let mut bad = request.clone();
    bad.protocol += 1;
    assert_eq!(
        host.dispatch(f.peer, bad),
        Response::Rejected(Error::IncompatibleVersion)
    );
    let mut huge = f.config.clone();
    huge.state.subscriptions[0].name = "x".repeat(CONFIG_BYTES + 1);
    assert_eq!(
        host.dispatch(
            f.peer,
            Request {
                protocol: PROTOCOL,
                command: Command::Bootstrap {
                    action: Box::new(BootstrapAction::Preflight {
                        id: StateEpoch::fresh().unwrap(),
                        expected: huge.version(),
                        config: huge
                    })
                }
            }
        ),
        Response::Rejected(Error::TooLarge)
    );
    assert_eq!(f.source.owner_transfer().unwrap(), None);
    assert!(
        !f.root
            .join("install/runtime/bootstrap-prepared.json")
            .exists()
    );
}

// 与生产primary固定协议相同的OS fixture；不运行GPUI，不调用ACTIVATE。
struct PrimaryFixture {
    lock: Option<std::fs::File>,
    done: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    socket: PathBuf,
}
impl PrimaryFixture {
    fn new(root: &std::path::Path) -> Self {
        use std::os::unix::fs::OpenOptionsExt;
        std::fs::create_dir_all(root.join("locks")).unwrap();
        std::fs::set_permissions(root.join("locks"), std::fs::Permissions::from_mode(0o700))
            .unwrap();
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(root.join("locks/desktop.lock"))
            .unwrap();
        assert_eq!(
            unsafe {
                libc::flock(
                    std::os::fd::AsRawFd::as_raw_fd(&lock),
                    libc::LOCK_EX | libc::LOCK_NB,
                )
            },
            0
        );
        let socket = root.join("runtime/desktop.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let finished = done.clone();
        let thread = std::thread::spawn(move || {
            use std::io::{Read, Write};
            while !finished.load(std::sync::atomic::Ordering::Acquire) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        let _ =
                            stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
                        let mut bytes = [0; 2];
                        if stream.read_exact(&mut bytes).is_ok() && bytes == OWNER_PROBE {
                            let _ = stream.write_all(&OWNER_ACK);
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(std::time::Duration::from_millis(2))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        Self {
            lock: Some(lock),
            done,
            thread: Some(thread),
            socket,
        }
    }
}
impl Drop for PrimaryFixture {
    fn drop(&mut self) {
        self.done.store(true, std::sync::atomic::Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
        let _ = std::fs::remove_file(&self.socket);
        drop(self.lock.take());
    }
}

fn completed(host: &Host, peer: Peer, id: u64) -> Result<Status, Error> {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        match host.dispatch(
            peer,
            Request {
                protocol: PROTOCOL,
                command: Command::Operation { request_id: id },
            },
        ) {
            Response::Operation(Operation::Completed(result)) => return result,
            Response::Operation(Operation::Inflight) => {
                assert!(std::time::Instant::now() < until);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            other => panic!("{other:?}"),
        }
    }
}
#[test]
fn bootstrap_commit_independent_start_ready_duplicate_stop_restart_over_socket() {
    // 首次无需手动child：Core NoChild源 + 真实socket/OS primary + 正式Backend/ProcessPort测试内核。
    let mut f = Fixture::new();
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = Host::new(f.peer.uid, backend);
    let socket = super::tests::SocketFixture::new();
    let expected = f.config.version();
    let mut exchange = |action| match rpc(
        &host,
        &socket,
        Command::Bootstrap {
            action: Box::new(action),
        },
    ) {
        Response::Bootstrap(r) => r,
        other => panic!("{other:?}"),
    };
    let prepared = veyra_core::application::helper_transfer::prepare_bootstrap(
        &mut f.source,
        StateEpoch::fresh().unwrap(),
        expected.clone(),
        &mut exchange,
    )
    .unwrap();
    let commit = veyra_core::application::helper_transfer::commit_bootstrap(
        &mut f.source,
        expected.clone(),
        &mut exchange,
    )
    .unwrap();
    assert_eq!(commit.phase, BootstrapPhase::Committed);
    assert_eq!(commit.ticket, prepared.ticket);
    assert_eq!(
        veyra_core::application::helper_transfer::commit_bootstrap(
            &mut f.source,
            expected.clone(),
            &mut exchange
        )
        .unwrap(),
        commit
    );
    assert!(
        !f.root
            .join("install/runtime/runtime/last-applied.json")
            .exists()
    );
    let start = Command::Start {
        request_id: 800,
        expected: expected.clone(),
        config: f.config.clone(),
    };
    assert!(matches!(
        rpc(&host, &socket, start.clone()),
        Response::Operation(Operation::Inflight)
    ));
    let ready = completed(&host, f.peer, 800).unwrap();
    assert_eq!(ready.applied, Some(expected.clone()));
    assert!(!ready.recovery_required);
    assert_eq!(
        exchange(BootstrapAction::Commit {
            ticket: commit.ticket.clone(),
            config: f.config.clone()
        })
        .unwrap()
        .phase,
        BootstrapPhase::Committed
    );

    assert!(matches!(
        rpc(&host, &socket, start),
        Response::Operation(Operation::Inflight)
    ));
    assert_eq!(
        completed(&host, f.peer, 800).unwrap().instance,
        ready.instance
    );
    assert!(matches!(
        rpc(
            &host,
            &socket,
            Command::Stop {
                request_id: 801,
                instance: ready.instance.unwrap()
            }
        ),
        Response::Operation(Operation::Inflight)
    ));
    assert!(completed(&host, f.peer, 801).unwrap().instance.is_none());
    let clean: serde_json::Value = serde_json::from_slice(
        &std::fs::read(f.root.join("install/runtime/bootstrap-lifecycle.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(clean["cycle"], 1);
    assert_eq!(clean["closed"].as_str().unwrap().len(), 64);

    assert!(matches!(
        rpc(
            &host,
            &socket,
            Command::Start {
                request_id: 802,
                expected,
                config: f.config.clone()
            }
        ),
        Response::Operation(Operation::Inflight)
    ));
    let again = completed(&host, f.peer, 802).unwrap();
    assert_ne!(again.instance, ready.instance);
    let running: serde_json::Value = serde_json::from_slice(
        &std::fs::read(f.root.join("install/runtime/bootstrap-lifecycle.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(running["cycle"], 2);
    assert!(running["closed"].is_null());

    // 退出可以先关闭primary listener；同peer只读owner查询和Stop仍能清理，但新Start不得放行。
    drop(f.primary.take());
    assert_eq!(
        exchange(BootstrapAction::Query {
            ticket: commit.ticket
        })
        .unwrap()
        .phase,
        BootstrapPhase::Committed
    );
    assert!(matches!(
        rpc(
            &host,
            &socket,
            Command::Stop {
                request_id: 803,
                instance: again.instance.unwrap()
            }
        ),
        Response::Operation(Operation::Inflight)
    ));
    let stopped = completed(&host, f.peer, 803).unwrap();
    assert!(stopped.instance.is_none());
    assert!(!stopped.recovery_required);
    drop(host);
}
#[test]
fn bootstrap_primary_lock_required_and_commit_rechecks_it() {
    let mut f = Fixture::new();
    let ticket = f.preflight();
    f.freeze(&ticket);
    let lock = f.primary.as_mut().unwrap().lock.take().unwrap();
    drop(lock);
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Prepare {
                    ticket: ticket.clone(),
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    assert!(!f.root.join("install/runtime").exists());
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.root.join("desktop/locks/desktop.lock"))
        .unwrap();
    assert_eq!(
        unsafe {
            libc::flock(
                std::os::fd::AsRawFd::as_raw_fd(&lock),
                libc::LOCK_EX | libc::LOCK_NB,
            )
        },
        0
    );
    f.primary.as_mut().unwrap().lock = Some(lock);
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Prepare {
                ticket: ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    drop(f.primary.as_mut().unwrap().lock.take());
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Commit {
                    ticket,
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    assert!(
        !f.root
            .join("install/runtime/bootstrap-committed.json")
            .exists()
    );
}

#[test]
fn bootstrap_partial_commit_and_lost_slot_preserve_frozen_owner() {
    // 保护半写Commit/服务重启不能把旧Frozen当首次空历史，或创建第二writer。
    let mut f = Fixture::new();
    let ticket = f.preflight();
    f.freeze(&ticket);
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Prepare {
                ticket: ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    let marker = f.root.join("install/runtime/bootstrap-committed.json");
    std::fs::write(&marker, b"partial").unwrap();
    std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Commit {
                    ticket: ticket.clone(),
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    assert_eq!(std::fs::read(&marker).unwrap(), b"partial");
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Query {
                    ticket: ticket.clone()
                }
            )
            .1
            .unwrap()
            .phase,
        BootstrapPhase::RecoveryRequired
    );

    let source = std::fs::read(f.root.join("desktop/runtime/owner-transfer.json")).unwrap();
    f.backend = f.backend();
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Query {
                    ticket: ticket.clone()
                }
            )
            .1
            .unwrap()
            .phase,
        BootstrapPhase::RecoveryRequired
    );
    assert!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Commit {
                    ticket,
                    config: f.config.clone()
                }
            )
            .1
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.root.join("desktop/runtime/owner-transfer.json")).unwrap(),
        source
    );
    assert!(!f.root.join("install/runtime/owner-session.json").exists());
}

#[test]
#[ignore = "only spawned by bootstrap nonprimary peer test"]
fn primary_peer_fixture() {
    // 子进程只接受父测试创建的路径；生产没有此入口。
    use std::io::{Read, Write};
    let root = PathBuf::from(std::env::var("VEYRA_PRIMARY_FIXTURE").unwrap());
    let mut primary = Some(PrimaryFixture::new(&root));
    if std::env::var_os("VEYRA_PRIMARY_FREEZE").is_some() {
        let ticket: BootstrapTicket = serde_json::from_slice(
            &std::fs::read(root.join("bootstrap-fixture-ticket.json")).unwrap(),
        )
        .unwrap();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        let mut runtime = ManualRuntime::new(
            NoChild,
            Arc::new(SnapshotService::new(store, StateAccessGate::default())),
            root.clone(),
            true,
        );
        runtime
            .freeze_first_bootstrap(ticket.id, ticket.installation, ticket.version)
            .unwrap();
    }
    println!("primary-ready");
    std::io::stdout().flush().unwrap();
    let mut byte = [0];
    std::io::stdin().read_exact(&mut byte).unwrap();
    if std::env::var_os("VEYRA_PRIMARY_REBIND").is_some() {
        let (previous, ticket): (BootstrapTicket, BootstrapTicket) = serde_json::from_slice(
            &std::fs::read(root.join("bootstrap-fixture-next.json")).unwrap(),
        )
        .unwrap();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        let mut runtime = ManualRuntime::new(
            NoChild,
            Arc::new(SnapshotService::new(store, StateAccessGate::default())),
            root.clone(),
            true,
        );
        runtime.prepare_rebind(&previous, &ticket).unwrap();
        println!("rebind-ready");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_exact(&mut byte).unwrap();
        if let Some(path) = std::env::var_os("VEYRA_REBIND_RPC") {
            let client = super::transport::Client::isolated(PathBuf::from(path));
            assert!(matches!(
                client
                    .request(Command::Bootstrap {
                        action: Box::new(BootstrapAction::RebindQuery {
                            ticket: ticket.clone()
                        })
                    })
                    .unwrap(),
                Response::Bootstrap(Ok(BootstrapReply {
                    phase: BootstrapPhase::Prepared,
                    ..
                }))
            ));
            if std::env::var_os("VEYRA_REBIND_COMMIT").is_some() {
                let config = Configuration {
                    state: Box::new(runtime.handoff_state().unwrap()),
                };
                let version = config.version();
                let mut exchange = |action| match client.request(Command::Bootstrap {
                    action: Box::new(action),
                })? {
                    Response::Bootstrap(r) => r,
                    _ => Err(Error::InvalidRequest),
                };
                // 实际Core编排经IPC Query/Commit/Query；再次调用模拟首个提交回复丢失。
                let lost = veyra_core::application::helper_transfer::commit_rebind(
                    &mut runtime,
                    version.clone(),
                    |action| {
                        let commit = matches!(action, BootstrapAction::RebindCommit { .. });
                        let actual = exchange(action)?;
                        if commit {
                            Err(Error::Timeout)
                        } else {
                            Ok(actual)
                        }
                    },
                );
                assert_eq!(lost, Err(Error::Timeout)); // root已提交，模拟仅丢首个Commit回复。
                let committed = veyra_core::application::helper_transfer::commit_rebind(
                    &mut runtime,
                    version.clone(),
                    |action| {
                        assert!(matches!(action, BootstrapAction::RebindQuery { .. }));
                        exchange(action)
                    },
                )
                .unwrap();
                assert_eq!(committed.phase, BootstrapPhase::Committed);
                assert_eq!(
                    veyra_core::application::helper_transfer::commit_rebind(
                        &mut runtime,
                        version.clone(),
                        &mut exchange
                    )
                    .unwrap(),
                    committed
                );
                fn complete(client: &super::transport::Client, request_id: u64) -> Status {
                    let end = std::time::Instant::now() + std::time::Duration::from_secs(20);
                    loop {
                        match client.request(Command::Operation { request_id }).unwrap() {
                            Response::Operation(Operation::Completed(result)) => {
                                return result.unwrap();
                            }
                            Response::Operation(Operation::Inflight) => {}
                            other => panic!("unexpected operation {other:?}"),
                        }
                        assert!(std::time::Instant::now() < end);
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
                let start = Command::Start {
                    request_id: 1100,
                    expected: version.clone(),
                    config: config.clone(),
                };
                assert!(matches!(
                    client.request(start.clone()).unwrap(),
                    Response::Operation(Operation::Inflight)
                ));
                let ready = complete(&client, 1100);
                assert!(ready.instance.is_some());
                assert_eq!(ready.applied, Some(version.clone()));
                assert!(!ready.recovery_required);
                assert!(matches!(
                    client.request(start).unwrap(),
                    Response::Operation(Operation::Inflight)
                ));
                assert_eq!(complete(&client, 1100).instance, ready.instance);
                client
                    .request(Command::Start {
                        request_id: 1101,
                        expected: version.clone(),
                        config: config.clone(),
                    })
                    .unwrap();
                assert_eq!(complete(&client, 1101).instance, ready.instance);
                assert_eq!(
                    exchange(BootstrapAction::RebindCommit {
                        ticket: ticket.clone(),
                        config
                    })
                    .unwrap(),
                    committed
                );
                assert_eq!(
                    exchange(BootstrapAction::RebindQuery {
                        ticket: ticket.clone()
                    })
                    .unwrap(),
                    committed
                );
                // 正常Quit先关闭listener但仍持有运行时业务身份；Stop不要求再probe窗口。
                drop(primary.take());
                client
                    .request(Command::Stop {
                        request_id: 1102,
                        instance: ready.instance.unwrap(),
                    })
                    .unwrap();
                let stopped = complete(&client, 1102);
                assert!(stopped.instance.is_none());
                assert!(!stopped.recovery_required);
                assert_eq!(
                    exchange(BootstrapAction::RebindQuery { ticket }).unwrap(),
                    committed
                );
            }
            println!("rpc-ready");
            std::io::stdout().flush().unwrap();
            std::io::stdin().read_exact(&mut byte).unwrap();
        }
    }
    drop(primary);
}
#[test]
fn bootstrap_other_process_primary_cannot_authorize_same_uid_rpc() {
    // 同UID的另一进程持锁/listen也不能替当前RPC peer提供primary证明。
    use std::io::{BufRead, BufReader};
    let mut f = Fixture::new();
    let ticket = f.preflight();
    f.freeze(&ticket);
    drop(f.primary.take());
    std::fs::remove_file(f.root.join("desktop/locks/desktop.lock")).unwrap();
    std::fs::remove_dir(f.root.join("desktop/locks")).unwrap();
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut child = Owned(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "production::bootstrap_tests::primary_peer_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("VEYRA_PRIMARY_FIXTURE", f.root.join("desktop"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut reader = BufReader::new(child.0.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if line.trim() == "primary-ready" {
            break;
        }
    }
    assert_eq!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::Prepare {
                    ticket,
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::Unauthorized)
    );
    assert!(!f.root.join("install/runtime").exists());
    let stream =
        std::os::unix::net::UnixStream::connect(f.root.join("desktop/runtime/desktop.sock"))
            .unwrap();
    let child_peer = transport::peer(&stream).unwrap();
    let mut watch = super::source::PeerExit::capture(child_peer).unwrap();
    assert!(!watch.exited().unwrap());
    drop(stream); // 断线不能当作owner退出。
    assert!(!watch.exited().unwrap());
    child.0.kill().unwrap();
    child.0.wait().unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while !watch.exited().unwrap() {
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(watch.exited().unwrap());
    drop(child);
}

#[test]
fn bootstrap_failed_child_cannot_reuse_committed_admission() {
    // 用户看到启动失败后，旧Committed不能把未知现场解释成可自动重启。
    let mut f = Fixture::new();
    f.config.state.nodes[0].server = "exit.invalid".into();
    let ticket = f.preflight();
    f.freeze(&ticket);
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Prepare {
                ticket: ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Commit {
                ticket,
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    let start = |id| Command::Start {
        request_id: id,
        expected: f.config.version(),
        config: f.config.clone(),
    };
    let (_, first) = f.backend.execute(
        f.peer,
        start(901),
        std::time::Instant::now() + std::time::Duration::from_secs(15),
    );
    assert_eq!(first, Err(Error::Failed));
    assert!(f.backend.poll().unwrap().recovery_required);
    let (_, retry) = f.backend.execute(
        f.peer,
        start(902),
        std::time::Instant::now() + std::time::Duration::from_secs(15),
    );
    assert_eq!(retry, Err(Error::HandoffRequired));
    assert!(
        f.root
            .join("install/runtime/bootstrap-committed.json")
            .exists()
    );
}

#[test]
fn clean_stop_partial_write_preserves_material_and_refuses_next_start() {
    // 关闭凭据写入失败不能冒充可恢复；Stop/reap仍完成，旧cache/manifest不删除。
    let mut f = Fixture::new();
    let ticket = f.preflight();
    f.freeze(&ticket);
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Prepare {
                ticket: ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    f.backend
        .bootstrap(
            f.peer,
            BootstrapAction::Commit {
                ticket,
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    let start = |id| Command::Start {
        request_id: id,
        expected: f.config.version(),
        config: f.config.clone(),
    };
    let ready = f
        .backend
        .execute(
            f.peer,
            start(950),
            std::time::Instant::now() + std::time::Duration::from_secs(15),
        )
        .1
        .unwrap();
    std::fs::write(
        f.root.join("install/runtime/bootstrap-lifecycle.pending"),
        b"partial",
    )
    .unwrap();
    let (status, result) = f.backend.execute(
        f.peer,
        Command::Stop {
            request_id: 951,
            instance: ready.instance.unwrap(),
        },
        std::time::Instant::now() + std::time::Duration::from_secs(15),
    );
    assert_eq!(result, Err(Error::HandoffRequired));
    assert!(status.instance.is_none());
    assert!(status.recovery_required);
    assert!(
        f.root
            .join("install/runtime/runtime/last-applied.json")
            .exists()
    );
    assert_eq!(
        std::fs::read(f.root.join("install/runtime/bootstrap-lifecycle.pending")).unwrap(),
        b"partial"
    );
    assert_eq!(
        f.backend
            .execute(
                f.peer,
                start(952),
                std::time::Instant::now() + std::time::Duration::from_secs(15)
            )
            .1,
        Err(Error::HandoffRequired)
    );
}

#[test]
fn bootstrap_primary_process_exit_reaps_only_owned_kernel_and_keeps_unknown_record() {
    // 真正OS Primary退出事件触发正式Backend清理自己的kernel Child；不以断线/PID缺失猜测。
    use std::io::{BufRead, BufReader};
    let mut f = Fixture::new();
    let ticket = f.preflight();
    drop(f.primary.take());
    std::fs::remove_file(f.root.join("desktop/locks/desktop.lock")).unwrap();
    std::fs::remove_dir(f.root.join("desktop/locks")).unwrap();
    std::fs::write(
        f.root.join("desktop/bootstrap-fixture-ticket.json"),
        serde_json::to_vec(&ticket).unwrap(),
    )
    .unwrap();
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut primary = Owned(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "production::bootstrap_tests::primary_peer_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("VEYRA_PRIMARY_FIXTURE", f.root.join("desktop"))
            .env("VEYRA_PRIMARY_FREEZE", "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let mut reader = BufReader::new(primary.0.stdout.take().unwrap());
    let mut line = String::new();
    loop {
        line.clear();
        assert_ne!(reader.read_line(&mut line).unwrap(), 0);
        if line.trim() == "primary-ready" {
            break;
        }
    }
    let stream =
        std::os::unix::net::UnixStream::connect(f.root.join("desktop/runtime/desktop.sock"))
            .unwrap();
    let peer = transport::peer(&stream).unwrap();
    drop(stream);
    f.backend
        .bootstrap(
            peer,
            BootstrapAction::Prepare {
                ticket: ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    f.backend
        .bootstrap(
            peer,
            BootstrapAction::Commit {
                ticket,
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    let ready = f
        .backend
        .execute(
            peer,
            Command::Start {
                request_id: 970,
                expected: f.config.version(),
                config: f.config.clone(),
            },
            std::time::Instant::now() + std::time::Duration::from_secs(15),
        )
        .1
        .unwrap();
    assert!(ready.instance.is_some());
    assert!(!f.backend.poll().unwrap().recovery_required); // 仅probe断线仍活。
    primary.0.kill().unwrap();
    primary.0.wait().unwrap();
    let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        let status = f.backend.poll().unwrap();
        if status.instance.is_none() {
            assert!(status.recovery_required);
            assert!(status.applied.is_none());
            break;
        }
        assert!(std::time::Instant::now() < until);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(f.root.join("install/runtime/bootstrap-lifecycle.json")).unwrap(),
    )
    .unwrap();
    assert!(
        record["closed"].is_null(),
        "abnormal owner exit must not certify clean rebind"
    );
    assert!(f.root.join("desktop/runtime/owner-transfer.json").exists());
    assert_eq!(
        f.backend
            .execute(
                peer,
                Command::Start {
                    request_id: 971,
                    expected: f.config.version(),
                    config: f.config.clone()
                },
                std::time::Instant::now() + std::time::Duration::from_secs(15)
            )
            .1,
        Err(Error::HandoffRequired)
    );
}

#[test]
fn rebind_clean_stop_new_os_primary_preflight_prepare_query_preserves_old_owner() {
    rebind_os_scenario(0);
}
#[test]
fn rebind_commit_new_primary_ipc_start_ready_stop_preserves_history() {
    // 保护用户正常退出再打开：真实双Primary/IPC/FD4内核fixture，不能只测DTO提交。
    rebind_os_scenario(1);
}
#[test]
fn rebind_partial_commit_preserves_material_and_blocks_both_owners() {
    // 保护提交半写不能靠文件存在或新Peer绕过，旧凭据不可被覆盖。
    rebind_os_scenario(2);
}
fn rebind_os_scenario(mode: u8) {
    // 保护正常重开准备：两代真实Primary进程，不以用户文件摘要独立授权新writer。
    use std::io::{BufRead, BufReader, Write};
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn spawn(
        root: &std::path::Path,
        mode: &str,
        commit: bool,
    ) -> (Owned, BufReader<std::process::ChildStdout>) {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "production::bootstrap_tests::primary_peer_fixture",
                "--ignored",
                "--nocapture",
            ])
            .env("VEYRA_PRIMARY_FIXTURE", root)
            .env(mode, "1")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        if mode == "VEYRA_PRIMARY_REBIND" {
            command.env(
                "VEYRA_REBIND_RPC",
                root.parent().unwrap().join("control.sock"),
            );
        }
        if commit {
            command.env("VEYRA_REBIND_COMMIT", "1");
        }
        let mut c = Owned(command.spawn().unwrap());
        let reader = BufReader::new(c.0.stdout.take().unwrap());
        (c, reader)
    }
    fn wait(reader: &mut BufReader<std::process::ChildStdout>, expected: &str) {
        let mut line = String::new();
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line.trim() == expected {
                break;
            }
        }
    }
    fn peer(root: &std::path::Path) -> Peer {
        let stream =
            std::os::unix::net::UnixStream::connect(root.join("runtime/desktop.sock")).unwrap();
        transport::peer(&stream).unwrap()
    }
    let mut f = Fixture::new();
    if mode == 1 {
        // 仅复制本次测试程序；实际exec文件名与生产一致，不接触真实内核或安装。
        std::fs::copy(
            std::env::current_exe().unwrap(),
            f.root.join("install/veyra-sing-box"),
        )
        .unwrap();
        f.backend = f.backend();
    }
    let previous = f.preflight();
    drop(f.primary.take());
    let source = f.root.join("desktop");
    std::fs::write(
        source.join("bootstrap-fixture-ticket.json"),
        serde_json::to_vec(&previous).unwrap(),
    )
    .unwrap();
    let (mut old, mut reader) = spawn(&source, "VEYRA_PRIMARY_FREEZE", false);
    wait(&mut reader, "primary-ready");
    let old_peer = peer(&source);
    f.backend
        .bootstrap(
            old_peer,
            BootstrapAction::Prepare {
                ticket: previous.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    f.backend
        .bootstrap(
            old_peer,
            BootstrapAction::Commit {
                ticket: previous.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    let ready = f
        .backend
        .execute(
            old_peer,
            Command::Start {
                request_id: 1001,
                expected: f.config.version(),
                config: f.config.clone(),
            },
            std::time::Instant::now() + std::time::Duration::from_secs(15),
        )
        .1
        .unwrap();
    f.backend
        .execute(
            old_peer,
            Command::Stop {
                request_id: 1002,
                instance: ready.instance.unwrap(),
            },
            std::time::Instant::now() + std::time::Duration::from_secs(15),
        )
        .1
        .unwrap();
    let next_id = StateEpoch::fresh().unwrap();
    assert!(
        f.backend
            .bootstrap(
                f.peer,
                BootstrapAction::RebindPreflight {
                    id: next_id.clone(),
                    previous: previous.clone(),
                    config: f.config.clone()
                }
            )
            .1
            .is_err(),
        "old peer still alive"
    );
    old.0.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
    assert!(old.0.wait().unwrap().success());
    let old_source = std::fs::read(source.join("runtime/source-session.json")).unwrap();
    let old_frozen = std::fs::read(source.join("runtime/owner-transfer.json")).unwrap();
    let (mut next, mut reader) = spawn(&source, "VEYRA_PRIMARY_REBIND", mode == 1);
    wait(&mut reader, "primary-ready");
    let new_peer = peer(&source);
    assert_ne!(new_peer, old_peer);
    let eligible = f
        .backend
        .bootstrap(
            new_peer,
            BootstrapAction::RebindPreflight {
                id: next_id.clone(),
                previous: previous.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    assert_eq!(eligible.phase, BootstrapPhase::Eligible);
    assert!(
        !f.root
            .join("install/runtime/bootstrap-rebind-prepared.json")
            .exists()
    );
    // 污染本测试的live cache后，不能只信CleanStop.closed字段；恢复fixture原字节后才继续。
    let root = f.root.join("install/runtime");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("runtime/last-applied.json")).unwrap())
            .unwrap();
    let cache = root.join(format!(
        "kernel-cache/{}.db",
        manifest["cache_generation"].as_str().unwrap()
    ));
    let bytes = std::fs::read(&cache).unwrap();
    std::fs::write(&cache, b"dirty").unwrap();
    assert_eq!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindPreflight {
                    id: next_id,
                    previous: previous.clone(),
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    std::fs::write(&cache, &bytes).unwrap();
    std::fs::write(
        source.join("bootstrap-fixture-next.json"),
        serde_json::to_vec(&(previous.clone(), eligible.ticket.clone())).unwrap(),
    )
    .unwrap();
    next.0.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
    wait(&mut reader, "rebind-ready");
    let prepared = f
        .backend
        .bootstrap(
            new_peer,
            BootstrapAction::RebindPrepare {
                previous: previous.clone(),
                ticket: eligible.ticket.clone(),
                config: f.config.clone(),
            },
        )
        .1
        .unwrap();
    assert_eq!(prepared.phase, BootstrapPhase::Prepared);
    assert_eq!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindPrepare {
                    previous,
                    ticket: eligible.ticket.clone(),
                    config: f.config.clone()
                }
            )
            .1
            .unwrap(),
        prepared
    );
    assert_eq!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindQuery {
                    ticket: eligible.ticket.clone()
                }
            )
            .1
            .unwrap(),
        prepared
    );
    assert_eq!(
        std::fs::read(source.join("runtime/source-session.json")).unwrap(),
        old_source
    );
    assert_eq!(
        std::fs::read(source.join("runtime/owner-transfer.json")).unwrap(),
        old_frozen
    );
    assert_eq!(
        f.backend
            .execute(
                new_peer,
                Command::Start {
                    request_id: 1003,
                    expected: f.config.version(),
                    config: f.config.clone()
                },
                std::time::Instant::now() + std::time::Duration::from_secs(15)
            )
            .1,
        Err(Error::HandoffRequired)
    );
    let audit_names = [
        "bootstrap-prepared.json",
        "bootstrap-committed.json",
        "owner-session.json",
        "bootstrap-rebind-prepared.json",
    ];
    let audit: Vec<_> = audit_names
        .iter()
        .map(|n| std::fs::read(root.join(n)).unwrap())
        .collect();
    let mut bad_config = f.config.clone();
    bad_config.state.config_revision += 1;
    assert_eq!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindCommit {
                    ticket: eligible.ticket.clone(),
                    config: bad_config
                }
            )
            .1,
        Err(Error::VersionConflict)
    );
    let mut bad_ticket = eligible.ticket.clone();
    bad_ticket.id = StateEpoch::fresh().unwrap();
    assert!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindCommit {
                    ticket: bad_ticket,
                    config: f.config.clone()
                }
            )
            .1
            .is_err()
    );
    std::fs::write(&cache, b"changed-after-prepare").unwrap();
    assert_eq!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindCommit {
                    ticket: eligible.ticket.clone(),
                    config: f.config.clone()
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    assert!(!root.join("bootstrap-rebind-committed.json").exists());
    std::fs::write(&cache, &bytes).unwrap();
    assert!(matches!(
        super::writer_lease::WriterLease::acquire(&root),
        Err(Error::Busy)
    ));
    if mode == 2 {
        let marker = root.join("bootstrap-rebind-committed.json");
        std::fs::write(&marker, b"partial").unwrap();
        std::fs::set_permissions(&marker, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            f.backend
                .bootstrap(
                    new_peer,
                    BootstrapAction::RebindCommit {
                        ticket: eligible.ticket.clone(),
                        config: f.config.clone()
                    }
                )
                .1
                .is_err()
        );
        assert_eq!(std::fs::read(&marker).unwrap(), b"partial");
        assert_eq!(
            f.backend
                .bootstrap(
                    new_peer,
                    BootstrapAction::RebindQuery {
                        ticket: eligible.ticket.clone()
                    }
                )
                .1
                .unwrap()
                .phase,
            BootstrapPhase::RecoveryRequired
        );
        for peer in [new_peer, old_peer] {
            assert!(
                f.backend
                    .execute(
                        peer,
                        Command::Start {
                            request_id: 1190,
                            expected: f.config.version(),
                            config: f.config.clone()
                        },
                        std::time::Instant::now() + std::time::Duration::from_secs(10)
                    )
                    .1
                    .is_err()
            );
        }
        for (name, bytes) in audit_names.iter().zip(&audit) {
            assert_eq!(&std::fs::read(root.join(name)).unwrap(), bytes);
        }
        assert_eq!(
            std::fs::read(source.join("runtime/source-session.json")).unwrap(),
            old_source
        );
        assert!(f.backend.poll().unwrap().recovery_required);
        return;
    }
    let listener = std::os::unix::net::UnixListener::bind(f.root.join("control.sock")).unwrap();
    std::os::unix::fs::chown(f.root.join("control.sock"), None, Some(f.peer.gid)).unwrap();
    listener.set_nonblocking(true).unwrap();
    std::fs::set_permissions(
        f.root.join("control.sock"),
        std::fs::Permissions::from_mode(0o660),
    )
    .unwrap();
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = Host::new(f.peer.uid, backend);
    let done = std::sync::atomic::AtomicBool::new(false);
    std::thread::scope(|scope| {
        let serving = scope.spawn(|| {
            let until = std::time::Instant::now() + std::time::Duration::from_secs(45);
            while !done.load(std::sync::atomic::Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        host.connection(stream).unwrap();
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(std::time::Instant::now() < until);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(e) => panic!("{e}"),
                }
            }
        });
        next.0.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
        wait(&mut reader, "rpc-ready");
        done.store(true, std::sync::atomic::Ordering::SeqCst);
        serving.join().unwrap();
    });
    if mode == 1 {
        let lifecycle: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("bootstrap-lifecycle.json")).unwrap())
                .unwrap();
        assert_eq!(lifecycle["cycle"], 2);
        assert!(lifecycle["closed"].is_string());
        let new_owner: Peer = serde_json::from_slice(
            &std::fs::read(root.join("bootstrap-current-owner.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(new_owner, new_peer);
        // 旧Peer重传不能启动；真实子进程已经退出，此处只测试服务端身份拒绝。
        host.dispatch(
            old_peer,
            Request {
                protocol: PROTOCOL,
                command: Command::Start {
                    request_id: 1191,
                    expected: f.config.version(),
                    config: f.config.clone(),
                },
            },
        );
        assert!(completed(&host, old_peer, 1191).is_err());
    }
    for (name, bytes) in audit_names.iter().zip(&audit) {
        assert_eq!(&std::fs::read(root.join(name)).unwrap(), bytes);
    }
    assert_eq!(
        std::fs::read(source.join("runtime/source-session.json")).unwrap(),
        old_source
    );
    assert_eq!(
        std::fs::read(source.join("runtime/owner-transfer.json")).unwrap(),
        old_frozen
    );
    assert!(matches!(
        super::writer_lease::WriterLease::acquire(&root),
        Err(Error::Busy)
    ));
    drop(host);
    f.backend = f.backend(); // 新helper没有旧Runtime/lease/NOTE_EXIT事实，不能重建准入。
    assert_eq!(
        f.backend
            .bootstrap(
                new_peer,
                BootstrapAction::RebindQuery {
                    ticket: eligible.ticket
                }
            )
            .1,
        Err(Error::HandoffRequired)
    );
    next.0.stdin.as_mut().unwrap().write_all(&[1]).unwrap();
    assert!(next.0.wait().unwrap().success());
}
