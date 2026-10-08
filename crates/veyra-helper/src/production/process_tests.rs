//! OS隔离集成：真正运行当前测试可执行文件，模拟sing-box协议而不是Mock Backend。
use super::{assets::Assets, backend::InstalledBackend, host::Backend, transport::Peer, *};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    os::fd::FromRawFd,
    time::{Duration, Instant},
};

#[test]
#[ignore = "only spawned by isolated production backend tests with FD3"]
fn kernel_fixture() {
    let action = std::env::var("VEYRA_FIXTURE_ACTION").expect("isolated child only");
    // 正式ProcessPort的check/run都必须真正继承FD4；不是fixture临时自行获取租约。
    assert_eq!(unsafe { libc::fcntl(4, libc::F_GETFD) }, 0);
    let mut input = unsafe { std::fs::File::from_raw_fd(3) };
    let mut bytes = Vec::new();
    input.read_to_end(&mut bytes).unwrap();
    let config: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    if action == "check" {
        std::thread::sleep(Duration::from_millis(80));
        return;
    }
    assert_ne!(unsafe { libc::geteuid() }, 0);
    if config["outbounds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["server"] == "exit.invalid")
    {
        std::process::exit(2);
    }
    // 测试内核持有真正的cache writer FD直到进程退出；只写固定fixture字节，不解析/模拟DB算法。
    let cache_path = config["experimental"]["cache_file"]["path"]
        .as_str()
        .unwrap();
    let mut cache_writer = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(cache_path)
        .unwrap();
    cache_writer
        .write_all(b"veyra-p206-controlled-cache\n")
        .unwrap();
    cache_writer.sync_all().unwrap();
    let mixed = TcpListener::bind("127.0.0.1:0").unwrap();
    let controller = TcpListener::bind("127.0.0.1:0").unwrap();
    println!(
        "inbound/mixed[mixed]: tcp server started at {}",
        mixed.local_addr().unwrap()
    );
    println!(
        "clash-api: restful api listening at {}",
        controller.local_addr().unwrap()
    );
    std::io::stdout().flush().unwrap();
    let secret = config["experimental"]["clash_api"]["secret"]
        .as_str()
        .unwrap();
    let bad_auth = config["outbounds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["server"] == "auth.invalid");
    if config["outbounds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["server"] == "later-exit.invalid")
    {
        std::thread::spawn(|| {
            std::thread::sleep(Duration::from_millis(600));
            std::process::exit(3);
        });
    }
    let mut selectors = std::collections::BTreeMap::new();
    for out in config["outbounds"].as_array().unwrap() {
        if out["type"] == "selector" {
            selectors.insert(
                out["tag"].as_str().unwrap().to_string(),
                out["outbounds"][0].as_str().unwrap().to_string(),
            );
        }
    }
    for stream in controller.incoming() {
        let mut stream = stream.unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut first = String::new();
        reader.read_line(&mut first).unwrap();
        let mut authorized = false;
        let mut len = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("authorization:") {
                authorized = line.trim().ends_with(&format!("Bearer {secret}"));
            }
            if lower.starts_with("content-length:") {
                len = line
                    .split_once(':')
                    .unwrap()
                    .1
                    .trim()
                    .parse::<usize>()
                    .unwrap();
            }
        }
        let mut body = vec![0; len];
        reader.read_exact(&mut body).unwrap();
        let path = first.split_whitespace().nth(1).unwrap();
        let (status, body) = if !authorized || bad_auth {
            ("401 Unauthorized", serde_json::json!({}))
        } else if path == "/version" {
            ("200 OK", serde_json::json!({"version":"sing-box 1.14.0"}))
        } else if let Some(tag) = path.strip_prefix("/proxies/") {
            if first.starts_with("PUT ") {
                let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
                let dir = std::path::Path::new(cache_path).parent().unwrap();
                if dir.join("fixture-third-put").exists() {
                    let third = config["outbounds"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|o| o["server"] == "third-fixture.invalid")
                        .unwrap()["tag"]
                        .as_str()
                        .unwrap();
                    selectors.insert(tag.into(), third.into());
                } else if dir.join("fixture-unknown-put").exists() {
                    selectors.insert(tag.into(), "unmapped-fixture-tag".into());
                } else if !dir.join("fixture-ignore-put").exists() {
                    selectors.insert(tag.into(), v["name"].as_str().unwrap().into());
                }
                // 仅fixture记录本child实际收到的PUT数量，便于证明重连不重发旧PUT。
                let count = std::path::Path::new(cache_path)
                    .parent()
                    .unwrap()
                    .join("fixture-put-count");
                let previous = std::fs::read_to_string(&count)
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);
                std::fs::write(count, (previous + 1).to_string()).unwrap();
                ("204 No Content", serde_json::json!({}))
            } else {
                // 仅fixture的确定性GET屏障；正式process没有此路径/开关。
                let dir = std::path::Path::new(cache_path).parent().unwrap();
                if dir.join("fixture-pause-get").exists() {
                    std::fs::write(dir.join("fixture-get-entered"), b"entered").unwrap();
                    let until = Instant::now() + Duration::from_secs(4);
                    while dir.join("fixture-pause-get").exists() {
                        assert!(Instant::now() < until);
                        std::thread::sleep(Duration::from_millis(2));
                    }
                }
                (
                    "200 OK",
                    serde_json::json!({"type":"Selector","now":selectors.get(tag),"all":selectors.values().collect::<Vec<_>>()}),
                )
            }
        } else {
            ("200 OK", serde_json::json!({"hello":"clash"}))
        };
        let text = if status.starts_with("204") {
            String::new()
        } else {
            body.to_string()
        };
        write!(stream,"HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}",text.len()).unwrap();
    }
}
struct Fixture {
    root: std::path::PathBuf,
    peer: Peer,
    backend: InstalledBackend,
    spawned: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "v206-real-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let (a, _b) = std::os::unix::net::UnixStream::pair().unwrap();
        let peer = transport::peer(&a).unwrap();
        let spawned: std::sync::Arc<std::sync::Mutex<Vec<u32>>> = Default::default();
        let backend = InstalledBackend::from_assets(Ok(Assets {
            installation: None,
            root: root.clone(),
            kernel: std::env::current_exe().unwrap(),
            uid: peer.uid,
            gid: peer.gid,
            fixture: true,
            spawned: std::sync::Arc::clone(&spawned),
        }));
        Self {
            root,
            peer,
            backend,
            spawned,
        }
    }
    fn execute(&mut self, c: Command) -> (Status, Result<Status, Error>) {
        self.backend
            .execute(self.peer, c, Instant::now() + Duration::from_secs(15))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let replacement = InstalledBackend::from_assets(Err(Error::Unavailable));
        drop(std::mem::replace(&mut self.backend, replacement));
        std::fs::remove_dir_all(&self.root).unwrap();
        // 只读核验本测试记录的自有child已wait/reap，不探测或终止其他进程。
        for pid in self.spawned.lock().unwrap().drain(..) {
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
                -1
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ECHILD)
            );
        }
    }
}
fn start(c: &Configuration, id: u64) -> Command {
    Command::Start {
        request_id: id,
        expected: c.version(),
        config: c.clone(),
    }
}
#[test]
fn production_backend_check_run_authenticated_ready_apply_stop_reap() {
    // 保护真实执行/FD配置/鉴权Ready/回收及P2-04 manifest；不是Mock Backend结果。
    let mut f = Fixture::new();
    let mut config = super::tests::config();
    let (s, r) = f.execute(start(&config, 1));
    assert!(r.is_ok(), "{s:?} {r:?}");
    assert!(s.applied.is_some());
    assert!(s.last_successful.is_some());
    let first = s.instance.unwrap();
    let expected = config.version();
    config.state.config_revision += 1;
    let (s, r) = f.execute(Command::Apply {
        request_id: 2,
        instance: first,
        expected,
        config: config.clone(),
    });
    assert!(r.is_ok(), "{s:?} {r:?}");
    assert_ne!(s.instance, Some(first));
    let (s, r) = f.execute(Command::Stop {
        request_id: 3,
        instance: s.instance.unwrap(),
    });
    assert!(r.is_ok(), "{s:?} {r:?}");
    assert!(s.instance.is_none());
    assert!(s.last_successful.is_some());
    let (s, r) = f.execute(start(&config, 4));
    assert!(r.is_ok(), "{s:?} {r:?}");
    assert_ne!(s.instance, Some(first));
    assert!(
        f.execute(Command::Stop {
            request_id: 5,
            instance: s.instance.unwrap()
        })
        .1
        .is_ok()
    );
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    assert!(!String::from_utf8_lossy(&manifest).contains("secret"));
}
#[test]
fn production_backend_child_failure_and_bad_auth_never_ready() {
    // 保护child提前退出/鉴权失败：真实进程清理，不能发布applied/last-successful。
    for server in ["exit.invalid", "auth.invalid"] {
        let mut f = Fixture::new();
        let mut c = super::tests::config();
        c.state.nodes[0].server = server.into();
        let (s, r) = f.execute(start(&c, 1));
        assert!(r.is_err());
        assert!(s.instance.is_none());
        assert!(s.applied.is_none());
        assert!(s.last_successful.is_none());
    }
}
#[test]
fn production_backend_new_session_and_reopened_owner_refuse_existing_material() {
    // 保护同UID另一进程/重启不会自动用旧cache/manifest开启第二writer。
    let mut f = Fixture::new();
    let c = super::tests::config();
    let (s, r) = f.execute(start(&c, 1));
    assert!(r.is_ok());
    assert!(
        f.execute(Command::Stop {
            request_id: 2,
            instance: s.instance.unwrap(),
        })
        .1
        .is_ok()
    );
    let other = Peer {
        pid: f.peer.pid + 1,
        ..f.peer
    };
    assert_eq!(
        f.backend
            .execute(
                other,
                start(&c, 3),
                Instant::now() + Duration::from_secs(15)
            )
            .1,
        Err(Error::HandoffRequired)
    );
    let mut reopened = InstalledBackend::from_assets(Ok(Assets {
        installation: None,
        root: f.root.clone(),
        kernel: std::env::current_exe().unwrap(),
        uid: f.peer.uid,
        gid: f.peer.gid,
        fixture: true,
        spawned: f.spawned.clone(),
    }));
    assert_eq!(
        reopened
            .execute(
                f.peer,
                start(&c, 4),
                Instant::now() + Duration::from_secs(15)
            )
            .1,
        Err(Error::HandoffRequired)
    );
}

#[test]
fn production_socket_duplicate_reconnect_cas_and_unexpected_exit() {
    // 保护用户重复点启动/IPC断线后查询，以及内核崩溃不能继续显示已应用。
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let mut f = Fixture::new();
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = Arc::new(super::host::Host::new(f.peer.uid, backend));
    let socket = super::tests::SocketFixture::new();
    let listener = socket.listener.try_clone().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stopping = Arc::new(AtomicBool::new(false));
    let done = stopping.clone();
    let server = host.clone();
    // 断言失败也先关闭本测试server并释放Host，让Backend持有的child真实reap。
    struct Server {
        stopping: Arc<AtomicBool>,
        worker: Option<std::thread::JoinHandle<()>>,
    }
    impl Drop for Server {
        fn drop(&mut self) {
            self.stopping.store(true, Ordering::Release);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
    let worker = std::thread::spawn(move || {
        while !done.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = server.connection(stream);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("isolated listener: {e}"),
            }
        }
    });
    let worker = Server {
        stopping: stopping.clone(),
        worker: Some(worker),
    };
    let client = socket.client();
    assert!(matches!(
        client.request(Command::Hello).unwrap(),
        Response::Hello(_)
    ));
    let mut c = super::tests::config();
    c.state.nodes[0].server = "later-exit.invalid".into();
    let request = start(&c, 31);
    assert_eq!(
        client.request(request.clone()).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(
        client.request(request.clone()).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    let until = Instant::now() + Duration::from_secs(5);
    let active = loop {
        if let Response::Operation(Operation::Completed(Ok(s))) = client
            .request(Command::Operation { request_id: 31 })
            .unwrap()
        {
            break s;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(active.applied.is_some());
    assert_eq!(
        client.request(request.clone()).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    loop {
        if let Response::Operation(Operation::Completed(result)) = client
            .request(Command::Operation { request_id: 31 })
            .unwrap()
        {
            assert_eq!(result, Ok(active.clone()));
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        client.request(start(&c, 32)).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    loop {
        if let Response::Operation(Operation::Completed(result)) = client
            .request(Command::Operation { request_id: 32 })
            .unwrap()
        {
            assert_eq!(result, Ok(active.clone()));
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    let mut stale = c.version();
    stale.config.0.revision += 1;
    assert_eq!(
        client
            .request(Command::Apply {
                request_id: 33,
                instance: active.instance.unwrap(),
                expected: stale,
                config: c.clone()
            })
            .unwrap(),
        Response::Rejected(Error::VersionConflict)
    );
    let other = Peer {
        pid: f.peer.pid + 1,
        ..f.peer
    };
    assert_eq!(
        host.dispatch(
            other,
            Request {
                protocol: PROTOCOL,
                command: Command::Stop {
                    request_id: 34,
                    instance: active.instance.unwrap()
                }
            }
        ),
        Response::Rejected(Error::StaleInstance)
    );
    loop {
        if let Response::Status(s) = client.request(Command::Status).unwrap()
            && s.applied.is_none()
        {
            assert!(s.instance.is_none());
            assert!(s.last_successful.is_some());
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        f.spawned.lock().unwrap().len(),
        2,
        "one check plus one child; duplicate Start must not spawn"
    );
    drop(worker);
    drop(host); // Backend Drop terminates/reaps before Fixture verifies ECHILD.
}

#[test]
fn production_bidirectional_closed_cache_handoff_over_socket() {
    // 保护desktop→helper→desktop：实际旧child先reap，关闭cache/plan/manifest跨私有root，两个owner不能同时Start。
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    use veyra_core::application::{
        helper_transfer::{TransferDirection, transfer},
        manual_runtime::RuntimeCommand as Local,
        runtime_recovery::OwnerTransfer,
    };
    let mut source = Fixture::new();
    let c = super::tests::config();
    assert!(source.execute(start(&c, 1)).1.is_ok());
    let mut local = source.backend.take_test_runtime();
    let mut target = Fixture::new();
    let backend = std::mem::replace(
        &mut target.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = Arc::new(super::host::Host::new(target.peer.uid, backend));
    let socket = super::tests::SocketFixture::new();
    let listener = socket.listener.try_clone().unwrap();
    listener.set_nonblocking(true).unwrap();
    let done = Arc::new(AtomicBool::new(false));
    let stop = done.clone();
    let server = host.clone();
    let worker = std::thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = server.connection(stream);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    struct ServerGuard(
        std::sync::Arc<std::sync::atomic::AtomicBool>,
        Option<std::thread::JoinHandle<()>>,
    );
    impl Drop for ServerGuard {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Release);
            if let Some(worker) = self.1.take() {
                worker.join().unwrap();
            }
        }
    }
    let server_guard = ServerGuard(done.clone(), Some(worker));
    let client = socket.client();
    let mut exchange = |action| match client.request(Command::Handoff {
        action: Box::new(action),
    })? {
        Response::Handoff(result) => result,
        Response::Rejected(e) => Err(e),
        _ => Err(Error::InvalidRequest),
    };
    let id = veyra_core::domain::StateEpoch::fresh().unwrap();
    let released = transfer(
        &mut local,
        id.clone(),
        c.version(),
        TransferDirection::ToHelper,
        &mut exchange,
    )
    .unwrap();
    assert!(matches!(released, OwnerTransfer::Released { .. }));
    assert!(local.execute(Local::Start, |_| {}).is_err());
    // 同id重试仅复用持久票据，不能再次复制活cache或启动child。
    assert_eq!(
        transfer(
            &mut local,
            id,
            c.version(),
            TransferDirection::ToHelper,
            &mut exchange
        )
        .unwrap(),
        released
    );
    assert_eq!(
        client.request(start(&c, 10)).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Response::Operation(Operation::Completed(r)) = client
            .request(Command::Operation { request_id: 10 })
            .unwrap()
        {
            assert!(r.is_ok(), "{r:?}");
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    let back = transfer(
        &mut local,
        veyra_core::domain::StateEpoch::fresh().unwrap(),
        c.version(),
        TransferDirection::ToDesktop,
        &mut exchange,
    )
    .unwrap();
    assert!(matches!(back, OwnerTransfer::Local { .. }));
    assert!(local.execute(Local::RestoreLastSuccessful, |_| {}).is_ok());
    assert_eq!(
        client.request(start(&c, 11)).unwrap(),
        Response::Operation(Operation::Inflight)
    );
    loop {
        if let Response::Operation(Operation::Completed(r)) = client
            .request(Command::Operation { request_id: 11 })
            .unwrap()
        {
            assert!(r.is_err());
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    local.execute(Local::Stop, |_| {}).unwrap();
    drop(local);
    drop(server_guard);
    drop(host);
}

#[test]
fn production_transfer_partial_commit_retry_and_restart_query() {
    // 保护断线/材料写入失败：Prepared不能启动，重试同票据修复材料，重启只查询而不假设旧child已消失。
    use veyra_core::application::{
        manual_runtime::RuntimeCommand as Local, runtime_recovery::OwnerTransfer,
    };
    let mut source = Fixture::new();
    let c = super::tests::config();
    assert!(source.execute(start(&c, 1)).1.is_ok());
    let mut local = source.backend.take_test_runtime();
    let (ticket, bundle) = local
        .prepare_handoff(
            veyra_core::domain::StateEpoch::fresh().unwrap(),
            c.version(),
        )
        .unwrap();
    let mut target = Fixture::new();
    let prepare = HandoffAction::Prepare {
        config: c.clone(),
        ticket: ticket.clone(),
        bundle: Box::new(bundle),
    };
    let deadline = || Instant::now() + Duration::from_secs(10);
    assert_eq!(
        target
            .backend
            .handoff(target.peer, prepare.clone(), deadline())
            .1
            .unwrap()
            .owner,
        Some(OwnerTransfer::Prepared {
            ticket: ticket.clone()
        })
    );
    assert!(target.execute(start(&c, 2)).1.is_err());
    assert!(target.spawned.lock().unwrap().is_empty());
    std::fs::remove_file(target.root.join("runtime/owner-bundle.json")).unwrap(); // 限定文件故障，模拟阶段写入未完成。
    let release = local.release_handoff(&ticket).unwrap();
    assert!(
        target
            .backend
            .handoff(
                target.peer,
                HandoffAction::Commit {
                    release: release.clone()
                },
                deadline()
            )
            .1
            .is_err()
    );
    assert_eq!(
        target
            .backend
            .handoff(target.peer, HandoffAction::Query, deadline())
            .1
            .unwrap()
            .owner,
        Some(OwnerTransfer::Prepared {
            ticket: ticket.clone()
        })
    );
    target
        .backend
        .handoff(target.peer, prepare, deadline())
        .1
        .unwrap();
    target
        .backend
        .handoff(target.peer, HandoffAction::Commit { release }, deadline())
        .1
        .unwrap();
    let imported: serde_json::Value = serde_json::from_slice(
        &std::fs::read(target.root.join("runtime/last-applied.json")).unwrap(),
    )
    .unwrap();
    let cache = imported["cache"]["path"].as_str().unwrap();
    assert_eq!(
        std::fs::read(target.root.join(cache)).unwrap(),
        b"veyra-p206-controlled-cache\n"
    );
    assert!(
        !target
            .root
            .join("kernel-cache")
            .join(format!(
                "{}.db",
                imported["cache_generation"].as_str().unwrap()
            ))
            .exists()
    );
    assert!(local.execute(Local::Start, |_| {}).is_err());
    assert!(target.spawned.lock().unwrap().is_empty());
    let mut reopened = InstalledBackend::from_assets(Ok(Assets {
        installation: None,
        root: target.root.clone(),
        kernel: std::env::current_exe().unwrap(),
        uid: target.peer.uid,
        gid: target.peer.gid,
        fixture: true,
        spawned: target.spawned.clone(),
    }));
    let (status, result) = reopened.handoff(target.peer, HandoffAction::Query, deadline());
    assert_eq!(result.unwrap().owner, Some(OwnerTransfer::Local { ticket }));
    assert!(status.recovery_required);
    let other = Peer {
        pid: target.peer.pid + 1,
        ..target.peer
    };
    assert_eq!(
        reopened.handoff(other, HandoffAction::Query, deadline()).1,
        Err(Error::Unauthorized)
    );
    assert!(
        reopened
            .execute(target.peer, start(&c, 3), deadline())
            .1
            .is_err()
    );
    // Desktop进程重建时即使能读到票据，也没有旧Child句柄；新Runtime nonce不能释放旧writer。
    let state_store =
        veyra_core::storage::JsonStateStore::new(source.root.join("received-state.json")).unwrap();
    let snapshots = std::sync::Arc::new(
        veyra_core::application::state_service::SnapshotService::new(
            state_store,
            veyra_core::application::state_access::StateAccessGate::default(),
        ),
    );
    let port = super::process::ProcessPort::new(
        Assets {
            installation: None,
            root: source.root.clone(),
            kernel: std::env::current_exe().unwrap(),
            uid: source.peer.uid,
            gid: source.peer.gid,
            fixture: true,
            spawned: source.spawned.clone(),
        },
        std::sync::Arc::new(std::sync::Mutex::new(deadline())),
    );
    let mut new_runtime = veyra_core::application::manual_runtime::ManualRuntime::new(
        port,
        snapshots,
        source.root.clone(),
        true,
    );
    assert!(new_runtime.execute(Local::Start, |_| {}).is_err());
    let old_ticket = local
        .owner_transfer()
        .unwrap()
        .unwrap()
        .ticket()
        .unwrap()
        .clone();
    assert!(
        new_runtime
            .prepare_handoff(old_ticket.id.clone(), old_ticket.version.clone())
            .is_err()
    );
    assert!(new_runtime.release_handoff(&old_ticket).is_err());
    assert_eq!(
        new_runtime.owner_transfer().unwrap(),
        local.owner_transfer().unwrap()
    );
    drop(new_runtime);
    drop(local);
}

#[test]
fn production_transfer_rejects_bad_material_and_pending_without_clearing_it() {
    // 保护SHA/大小/旧epoch先拒绝；已有pending不被交接的旧cache/manifest消除，活child不为失败预检停止。
    use veyra_core::{
        application::runtime_recovery::{ClosedBundle, HANDOFF_CACHE_BYTES},
        domain::*,
        storage::{JsonStateStore, StateStore},
    };
    let mut source = Fixture::new();
    let mut c = super::tests::config();
    assert!(source.execute(start(&c, 1)).1.is_ok());
    let mut local = source.backend.take_test_runtime();
    let before = std::fs::read(source.root.join("runtime/last-applied.json")).unwrap();
    c.state.pools.push(NodePool {
        id: PoolId("pending-pool".into()),
        name: "pending".into(),
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
    c.state.selection_revision += 1;
    JsonStateStore::new(source.root.join("received-state.json"))
        .unwrap()
        .save(&c.state)
        .unwrap();
    assert!(
        local
            .prepare_handoff(StateEpoch::fresh().unwrap(), c.version())
            .is_err()
    );
    assert!(local.snapshot().unwrap().runtime.instance_id.is_some());
    assert_eq!(
        std::fs::read(source.root.join("runtime/last-applied.json")).unwrap(),
        before
    );
    assert!(local.owner_transfer().unwrap().is_none());
    assert!(matches!(
        local.handoff_state().unwrap().pools[0].selection,
        SelectionPolicy::Manual {
            pending_node_id: Some(_),
            ..
        }
    ));
    // 新的隔离source保持同一份可信材料；不清除上面fixture的pending来制造PASS。
    let mut clean = Fixture::new();
    let config = super::tests::config();
    assert!(clean.execute(start(&config, 1)).1.is_ok());
    let mut clean_runtime = clean.backend.take_test_runtime();
    let (mut ticket, bundle) = clean_runtime
        .prepare_handoff(StateEpoch::fresh().unwrap(), config.version())
        .unwrap();
    let mut target = Fixture::new();
    ticket.digest = "wrong".into();
    assert!(
        target
            .backend
            .handoff(
                target.peer,
                HandoffAction::Prepare {
                    config: config.clone(),
                    ticket,
                    bundle: Box::new(bundle.clone())
                },
                Instant::now() + Duration::from_secs(5)
            )
            .1
            .is_err()
    );
    assert_eq!(std::fs::read_dir(&target.root).unwrap().count(), 0);
    let mut raw = serde_json::to_value(&bundle).unwrap();
    raw["cache"] = serde_json::json!(vec![0u8; HANDOFF_CACHE_BYTES + 1]);
    let oversized: ClosedBundle = serde_json::from_value(raw).unwrap();
    assert!(oversized.ticket(StateEpoch::fresh().unwrap()).is_err());
    let mut wrong = config.version();
    wrong.config.0.epoch = StateEpoch::fresh().unwrap();
    assert!(
        clean_runtime
            .prepare_handoff(StateEpoch::fresh().unwrap(), wrong)
            .is_err()
    );
    drop(clean_runtime);
    drop(local);
}

#[test]
fn production_handoff_changed_business_epoch_preserves_running_source() {
    // 保护整体替换state后的旧owner：epoch不符必须在freeze/Stop前拒绝，不用旧manifest接管新业务。
    use veyra_core::storage::{JsonStateStore, StateStore};
    let mut f = Fixture::new();
    let mut c = super::tests::config();
    assert!(f.execute(start(&c, 1)).1.is_ok());
    let mut local = f.backend.take_test_runtime();
    let before = local.snapshot().unwrap();
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    c.state.state_epoch = veyra_core::domain::StateEpoch::fresh().unwrap();
    JsonStateStore::new(f.root.join("received-state.json"))
        .unwrap()
        .save(&c.state)
        .unwrap();
    assert!(
        local
            .prepare_handoff(
                veyra_core::domain::StateEpoch::fresh().unwrap(),
                c.version()
            )
            .is_err()
    );
    assert_eq!(
        local.snapshot().unwrap().runtime.instance_id,
        before.runtime.instance_id
    );
    assert!(local.owner_transfer().unwrap().is_none());
    assert!(!f.root.join("runtime/owner-incarnation.json").exists());
    assert_eq!(
        std::fs::read(f.root.join("runtime/last-applied.json")).unwrap(),
        manifest
    );
    assert_eq!(
        local.handoff_state().unwrap().state_epoch,
        c.state.state_epoch
    );
    drop(local);
}

#[test]
fn production_handoff_preflight_failures_preserve_live_writer() {
    // 保护“helper未安装/拒绝/版本错误仍可手动代理”：源是实际fixture child，不用Mock Ready证明存活。
    use veyra_core::{
        application::{
            helper_transfer::{TransferDirection, transfer},
            manual_runtime::RuntimeCommand as Local,
        },
        domain::StateEpoch,
        storage::{JsonStateStore, StateStore},
    };
    let mut source = Fixture::new();
    let c = super::tests::config();
    assert!(source.execute(start(&c, 1)).1.is_ok());
    let mut local = source.backend.take_test_runtime();
    let before = local.snapshot().unwrap().runtime.instance_id;
    let manifest = std::fs::read(source.root.join("runtime/last-applied.json")).unwrap();
    let m: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    let cache = source
        .root
        .join("kernel-cache")
        .join(format!("{}.db", m["cache_generation"].as_str().unwrap()));
    let cache_before = std::fs::read(&cache).unwrap();
    for scenario in 0..6 {
        let mut expected = c.version();
        if scenario == 3 {
            expected.config.0.epoch = StateEpoch::fresh().unwrap();
        }
        if scenario == 4 {
            expected.selection.0.revision += 1;
        }
        if scenario == 5 {
            let mut large = c.clone();
            for n in 0..3000 {
                let mut subscription = c.state.subscriptions[0].clone();
                subscription.id = veyra_core::domain::SubscriptionId(format!("large-{n}"));
                subscription.description = "x".repeat(280);
                large.state.subscriptions.push(subscription);
            }
            JsonStateStore::new(source.root.join("received-state.json"))
                .unwrap()
                .save(&large.state)
                .unwrap();
        }
        let socket = super::tests::SocketFixture::new();
        let client = if scenario == 0 {
            transport::Client::isolated(source.root.join("absent.sock"))
        } else {
            socket.client()
        };
        let worker = if scenario == 1 || scenario == 2 {
            let listener = socket.listener.try_clone().unwrap();
            Some(std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                assert!(matches!(
                    transport::read_request(&mut stream).unwrap().command,
                    Command::Hello
                ));
                if scenario == 1 {
                    transport::write_response(
                        &mut stream,
                        &Response::Rejected(Error::IncompatibleVersion),
                    )
                    .unwrap();
                } else {
                    let backend = InstalledBackend::from_assets(Err(Error::HandoffRequired));
                    transport::write_response(
                        &mut stream,
                        &Response::Hello(backend.capabilities()),
                    )
                    .unwrap();
                }
            }))
        } else {
            None
        };
        let result = transfer(
            &mut local,
            StateEpoch::fresh().unwrap(),
            expected,
            TransferDirection::ToHelper,
            |action| match client.request(Command::Handoff {
                action: Box::new(action),
            })? {
                Response::Handoff(r) => r,
                Response::Rejected(e) => Err(e),
                _ => Err(Error::InvalidRequest),
            },
        );
        assert!(result.is_err(), "scenario {scenario}");
        if let Some(worker) = worker {
            worker.join().unwrap();
        }
        local.execute(Local::Refresh, |_| {}).unwrap();
        assert_eq!(local.snapshot().unwrap().runtime.instance_id, before);
        assert!(local.owner_transfer().unwrap().is_none());
        assert!(!source.root.join("runtime/owner-incarnation.json").exists());
        assert_eq!(
            std::fs::read(source.root.join("runtime/last-applied.json")).unwrap(),
            manifest
        );
        assert_eq!(std::fs::read(&cache).unwrap(), cache_before);
        // waitpid对本测试的真实child返回0证明仍存活，不能仅依赖Runtime缓存。
        let mut alive = 0;
        for pid in source.spawned.lock().unwrap().iter() {
            let mut status = 0;
            // check子进程已reap，只有run仍活；通过instance + refresh及下方run计数证明。
            let result = unsafe { libc::waitpid(*pid as i32, &mut status, libc::WNOHANG) };
            if result == 0 {
                alive += 1;
            }
            assert!(
                result == 0
                    || (result == -1
                        && std::io::Error::last_os_error().raw_os_error() == Some(libc::ECHILD))
            );
        }
        assert_eq!(alive, 1);
    }
    local.execute(Local::Stop, |_| {}).unwrap();
    drop(local);
}

#[test]
fn production_remote_selection_ipc_pending_readback_cas_manifest_and_retry() {
    // 保护用户选择节点：真实fixture controller PUT/GET、桌面持久pending/CAS、helper manifest回告。
    use veyra_core::{
        application::{
            helper_transfer::select_remote, state_access::StateAccessGate,
            state_service::SnapshotService,
        },
        domain::*,
        storage::{JsonStateStore, RemoteSelection, StateStore},
    };
    let mut f = Fixture::new();
    let c = manual_selection_config();
    let started = f.execute(start(&c, 1)).1.unwrap();
    let count = f.root.join("kernel-cache").join("fixture-put-count");
    let initial_puts = std::fs::read_to_string(&count)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let store = JsonStateStore::new(f.root.join("desktop-state.json")).unwrap();
    store.save(&c.state).unwrap();
    let snapshots = SnapshotService::new(store.clone(), StateAccessGate::default());
    let request = RemoteSelection {
        request_id: 90,
        instance: started.instance.unwrap(),
        expected: c.version(),
        pool: PoolId("pool".into()),
        node: NodeId("next".into()),
    };
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let socket = super::tests::SocketFixture::new();
    let host = std::sync::Arc::new(super::host::Host::new(f.peer.uid, backend));
    let listener = socket.listener.try_clone().unwrap();
    listener.set_nonblocking(true).unwrap();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop = done.clone();
    let server = host.clone();
    let worker = std::thread::spawn(move || {
        while !stop.load(std::sync::atomic::Ordering::Acquire) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = server.connection(stream);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(e) => panic!("{e}"),
            }
        }
    });
    struct Guard(
        std::sync::Arc<std::sync::atomic::AtomicBool>,
        Option<std::thread::JoinHandle<()>>,
    );
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, std::sync::atomic::Ordering::Release);
            self.1.take().unwrap().join().unwrap();
        }
    }
    let guard = Guard(done, Some(worker));
    let client = socket.client();
    let mut rpc = |action| match client.request(Command::Selection {
        action: Box::new(action),
    })? {
        Response::Selection(r) => r,
        Response::Rejected(e) => Err(e),
        _ => Err(Error::InvalidRequest),
    };
    let manifest_before = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    // 实际执行完成后丢弃响应，模拟断线；业务状态必须保留pending，其他writer不能插入新意图。
    let error = select_remote(&snapshots, request.clone(), |action| {
        let result = rpc(action)?;
        assert_eq!(result.actual, Some(request.node.clone()));
        Err(Error::Timeout)
    });
    assert_eq!(error, Err(Error::Timeout));
    assert!(store.selection_fence().unwrap().is_some());
    let pending = store.load().unwrap();
    assert_eq!(pending.selection_revision, 1);
    assert!(
        snapshots
            .replace(pending.version(), (*c.state).clone())
            .is_err()
    );
    assert_eq!(
        std::fs::read(f.root.join("runtime/last-applied.json")).unwrap(),
        manifest_before
    );
    let status = client.request(Command::Status).unwrap();
    assert!(matches!(
        status,
        Response::Status(Status {
            recovery_required: true,
            ..
        })
    ));
    // 新SnapshotService代表桌面事务状态重读，不能靠内存锁恢复写权。
    let reopened = SnapshotService::new(
        JsonStateStore::new(f.root.join("desktop-state.json")).unwrap(),
        StateAccessGate::default(),
    );
    let confirmed = select_remote(&reopened, request.clone(), &mut rpc).unwrap();
    assert_eq!(confirmed.selection.0.revision, 2);
    assert!(store.selection_fence().unwrap().is_none());
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.root.join("runtime/last-applied.json")).unwrap())
            .unwrap();
    assert_ne!(
        std::fs::read(f.root.join("runtime/last-applied.json")).unwrap(),
        manifest_before
    );
    let count = f.root.join("kernel-cache").join("fixture-put-count");
    assert_eq!(
        std::fs::read_to_string(&count)
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        initial_puts + 1
    );
    assert!(manifest["confirmed_selection"].to_string().contains("next"));
    assert!(matches!(
        client
            .request(Command::Operation {
                request_id: request.request_id
            })
            .unwrap(),
        Response::Operation(Operation::Completed(Ok(_)))
    ));
    let reused = RemoteSelection {
        expected: confirmed.clone(),
        ..request.clone()
    };
    assert_eq!(
        rpc(RemoteSelectionAction::Execute { request: reused }),
        Err(Error::RequestConflict)
    );
    // 同请求再Execute返回历史确认，不做PUT；旧expected新请求必须失败。
    assert_eq!(
        rpc(RemoteSelectionAction::Execute {
            request: request.clone()
        })
        .unwrap()
        .confirmed,
        Some(confirmed.clone())
    );
    let mut stale = request;
    stale.request_id += 1;
    assert!(rpc(RemoteSelectionAction::Execute { request: stale }).is_err());
    assert_eq!(store.load().unwrap().version(), confirmed);
    assert_eq!(
        std::fs::read_to_string(&count)
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        initial_puts + 1
    );
    // 真正关闭发送端socket：完整请求已写入，但不读取结果；重连只能Query，不重发PUT。
    let disconnected = RemoteSelection {
        request_id: 100,
        instance: started.instance.unwrap(),
        expected: confirmed,
        pool: PoolId("pool".into()),
        node: NodeId("node".into()),
    };
    reopened.begin_remote_selection(&disconnected).unwrap();
    let mut stream = std::os::unix::net::UnixStream::connect(
        socket.listener.local_addr().unwrap().as_pathname().unwrap(),
    )
    .unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let write = |stream: &mut std::os::unix::net::UnixStream, command| {
        let bytes = serde_json::to_vec(&Request {
            protocol: PROTOCOL,
            command,
        })
        .unwrap();
        stream.write_all(&[1]).unwrap();
        stream
            .write_all(&(bytes.len() as u32).to_be_bytes())
            .unwrap();
        stream.write_all(&bytes).unwrap();
    };
    write(&mut stream, Command::Hello);
    let mut header = [0; 5];
    stream.read_exact(&mut header).unwrap();
    let size = u32::from_be_bytes(header[1..].try_into().unwrap()) as usize;
    assert!(size <= CONTROL_BYTES);
    stream.read_exact(&mut vec![0; size]).unwrap();
    write(
        &mut stream,
        Command::Selection {
            action: Box::new(RemoteSelectionAction::Execute {
                request: disconnected.clone(),
            }),
        },
    );
    stream.shutdown(std::net::Shutdown::Both).unwrap();
    drop(stream);
    let until = Instant::now() + Duration::from_secs(5);
    let version = loop {
        match select_remote(&reopened, disconnected.clone(), &mut rpc) {
            Ok(v) => break v,
            Err(Error::Busy) => {
                assert!(Instant::now() < until);
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => panic!("disconnected selection: {e:?}"),
        }
    };
    assert_eq!(version.selection.0.revision, 4);
    assert_eq!(
        std::fs::read_to_string(&count)
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        initial_puts + 2
    );
    // 同池GET尚在进行时，新pending/整体换epoch/原始JSON写都必须被持久fence拒绝。
    let interleaved = RemoteSelection {
        request_id: 105,
        expected: version.clone(),
        node: NodeId("next".into()),
        ..disconnected.clone()
    };
    let pause = f.root.join("kernel-cache/fixture-pause-get");
    let entered = f.root.join("kernel-cache/fixture-get-entered");
    std::fs::write(&pause, b"pause").unwrap();
    let thread_client = client.clone();
    let thread_snapshots = reopened.clone();
    let thread_request = interleaved.clone();
    let selecting = std::thread::spawn(move || {
        select_remote(
            &thread_snapshots,
            thread_request,
            |action| match thread_client.request(Command::Selection {
                action: Box::new(action),
            })? {
                Response::Selection(r) => r,
                Response::Rejected(e) => Err(e),
                _ => Err(Error::InvalidRequest),
            },
        )
    });
    let until = Instant::now() + Duration::from_secs(2);
    while !entered.exists() {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(2));
    }
    let pending = store.load().unwrap();
    assert_eq!(pending.selection_revision, 5);
    assert!(
        veyra_core::application::state_service::SelectionService::new(reopened.clone())
            .begin_manual_pending(
                pending.selection_version(),
                interleaved.pool.clone(),
                NodeId("node".into())
            )
            .is_err()
    );
    assert!(
        reopened
            .replace(pending.version(), (*c.state).clone())
            .is_err()
    );
    assert!(store.save(&c.state).is_err());
    std::fs::remove_file(pause).unwrap();
    let version = selecting.join().unwrap().unwrap();
    assert_eq!(version.selection.0.revision, 6);
    // 桌面CAS成功后helper manifest失败：fence继续阻塞，重试不重新PUT也不回退旧cache。
    let partial = RemoteSelection {
        request_id: 106,
        expected: version,
        node: NodeId("node".into()),
        ..disconnected.clone()
    };
    let manifest_path = f.root.join("runtime/last-applied.json");
    let saved_path = f.root.join("runtime/test-last-applied.saved");
    let result = select_remote(&reopened, partial.clone(), |action| {
        if matches!(action, RemoteSelectionAction::Confirm { .. }) {
            std::fs::rename(&manifest_path, &saved_path).unwrap();
            std::fs::create_dir(&manifest_path).unwrap();
            let result = rpc(action);
            std::fs::remove_dir(&manifest_path).unwrap();
            std::fs::rename(&saved_path, &manifest_path).unwrap();
            result
        } else {
            rpc(action)
        }
    });
    assert!(result.is_err());
    assert_eq!(store.load().unwrap().selection_revision, 8);
    assert!(store.selection_fence().unwrap().is_some());
    let version = select_remote(&reopened, partial, &mut rpc).unwrap();
    assert_eq!(version.selection.0.revision, 8);
    assert_eq!(
        std::fs::read_to_string(&count)
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        initial_puts + 4
    );
    // UNKNOWN绝不清pending或恢复旧cache。未发送的请求只有fence，Query无slot必须保留。
    let unknown = RemoteSelection {
        request_id: 110,
        expected: version,
        ..disconnected
    };
    reopened.begin_remote_selection(&unknown).unwrap();
    assert!(select_remote(&reopened, unknown, &mut rpc).is_err());
    assert!(store.selection_fence().unwrap().is_some());
    drop(guard);
    drop(host);
}

fn manual_selection_config() -> Configuration {
    use veyra_core::domain::*;
    let mut c = super::tests::config();
    let mut node = c.state.nodes[0].clone();
    node.id = NodeId("next".into());
    node.name = "next".into();
    c.state.nodes.push(node);
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
            selected_node_id: Some(NodeId("node".into())),
            pending_node_id: None,
        },
    });
    // 显式route使该业务pool进入当前运行闭包。
    c.state.default_target = RouteTarget::Pool(PoolId("pool".into()));
    let mut third = c.state.nodes[0].clone();
    third.id = NodeId("third".into());
    third.name = "third".into();
    third.server = "third-fixture.invalid".into();
    c.state.nodes.push(third);
    c
}

#[test]
fn production_remote_selection_old_third_unknown_never_guess_confirmation() {
    // 实际controller GET=old才clear；third/unknown保留pending/fence/旧manifest，不拿cache猜actual。
    use veyra_core::{
        application::{
            helper_transfer::select_remote, state_access::StateAccessGate,
            state_service::SnapshotService,
        },
        domain::*,
        storage::{JsonStateStore, RemoteSelection, StateStore},
    };
    for mode in ["ignore", "third", "unknown"] {
        let mut f = Fixture::new();
        let c = manual_selection_config();
        let status = f.execute(start(&c, 1)).1.unwrap();
        let before = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
        let store = JsonStateStore::new(f.root.join("desktop-state.json")).unwrap();
        store.save(&c.state).unwrap();
        let snapshots = SnapshotService::new(store.clone(), StateAccessGate::default());
        std::fs::write(
            f.root.join(format!("kernel-cache/fixture-{mode}-put")),
            b"fault",
        )
        .unwrap();
        let request = RemoteSelection {
            request_id: 20,
            instance: status.instance.unwrap(),
            expected: c.version(),
            pool: PoolId("pool".into()),
            node: NodeId("next".into()),
        };
        let result = select_remote(&snapshots, request.clone(), |action| {
            f.backend
                .selection(f.peer, action, Instant::now() + Duration::from_secs(5))
                .1
        });
        if mode == "ignore" {
            assert_eq!(result, Err(Error::Failed));
            assert!(store.selection_fence().unwrap().is_none());
            assert_eq!(store.load().unwrap().selection_revision, 2);
            assert!(
                matches!(&store.load().unwrap().pools[0].selection, SelectionPolicy::Manual { selected_node_id: Some(n), pending_node_id: None } if n.0 == "node")
            );
        } else {
            assert!(result.is_err());
            assert!(store.selection_fence().unwrap().is_some());
            assert_eq!(store.load().unwrap().selection_revision, 1);
            assert_eq!(
                std::fs::read(f.root.join("runtime/last-applied.json")).unwrap(),
                before
            );
            let (status, query) = f.backend.selection(
                f.peer,
                RemoteSelectionAction::Query { request },
                Instant::now() + Duration::from_secs(5),
            );
            assert!(status.recovery_required);
            assert_eq!(
                query.unwrap().actual,
                (mode == "third").then(|| NodeId("third".into()))
            );
        }
    }
}

#[test]
fn production_admin_drain_stops_reaps_and_seals_before_ack() {
    // 保护卸载不会先bootout留下writer；只运行本测试exe，管理员文件位于私有fixture。
    let mut f = Fixture::new();
    let c = super::tests::config();
    f.execute(start(&c, 1)).1.unwrap();
    f.backend.test_admin_root(f.root.clone());
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    admin::request(&f.root).unwrap();
    let status = f.backend.poll().unwrap();
    assert!(status.instance.is_none());
    assert!(admin::acknowledged(&f.root).unwrap());
    let owner =
        veyra_core::application::runtime_recovery::RecoveryStore::inspect_owner_transfer(&f.root)
            .unwrap();
    assert!(matches!(
        owner,
        Some(veyra_core::application::runtime_recovery::OwnerTransfer::Closed { .. })
    ));
    let before: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.root.join("runtime/last-applied.json")).unwrap())
            .unwrap();
    for field in ["config", "confirmed_selection", "plan", "cache_generation"] {
        assert_eq!(before[field], after[field], "{field}");
    }
    // Stop后只允许按真实关闭cache更新快照引用；不能改变已确认业务版本。

    assert_eq!(f.execute(start(&c, 2)).1, Err(Error::HandoffRequired));
    // 重复请求不再spawn；Drop对所有真正创建的check/run child逐一核验ECHILD。
    assert_eq!(f.spawned.lock().unwrap().len(), 2);
}

#[test]
fn production_admin_drain_uncertain_selection_preserves_fence_and_refuses_ack() {
    // GET未知时卸载只能Stop自有child，不能清pending、签发成功或丢失恢复资料。
    use veyra_core::{
        application::{
            helper_transfer::select_remote, state_access::StateAccessGate,
            state_service::SnapshotService,
        },
        domain::*,
        storage::{JsonStateStore, RemoteSelection, StateStore},
    };
    let mut f = Fixture::new();
    let c = manual_selection_config();
    let status = f.execute(start(&c, 1)).1.unwrap();
    let store = JsonStateStore::new(f.root.join("desktop-state.json")).unwrap();
    store.save(&c.state).unwrap();
    let snapshots = SnapshotService::new(store.clone(), StateAccessGate::default());
    std::fs::write(f.root.join("kernel-cache/fixture-unknown-put"), b"fault").unwrap();
    let request = RemoteSelection {
        request_id: 20,
        instance: status.instance.unwrap(),
        expected: c.version(),
        pool: PoolId("pool".into()),
        node: NodeId("next".into()),
    };
    assert!(
        select_remote(&snapshots, request, |action| f
            .backend
            .selection(f.peer, action, Instant::now() + Duration::from_secs(5))
            .1)
        .is_err()
    );
    let before = std::fs::read(f.root.join("desktop-state.json")).unwrap();
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    f.backend.test_admin_root(f.root.clone());
    admin::request(&f.root).unwrap();
    let status = f.backend.poll().unwrap();
    assert!(status.instance.is_none());
    assert!(status.recovery_required);
    assert!(!admin::acknowledged(&f.root).unwrap());
    assert!(store.selection_fence().unwrap().is_some());
    assert_eq!(
        std::fs::read(f.root.join("desktop-state.json")).unwrap(),
        before
    );
    let before: serde_json::Value = serde_json::from_slice(&manifest).unwrap();
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(f.root.join("runtime/last-applied.json")).unwrap())
            .unwrap();
    for field in ["config", "confirmed_selection", "plan", "cache_generation"] {
        assert_eq!(before[field], after[field], "{field}");
    }
    // Stop后只允许按真实关闭cache更新快照引用；不能改变已确认业务版本。

    assert_eq!(f.execute(start(&c, 21)).1, Err(Error::HandoffRequired));
}

#[test]
fn production_admin_drain_bad_cache_keeps_evidence_and_never_acknowledges() {
    // 封存实际cache失败不伪造清理成功；服务仍活着供诊断，卸载不能bootout。
    let mut f = Fixture::new();
    let c = super::tests::config();
    f.execute(start(&c, 1)).1.unwrap();
    f.backend.test_admin_root(f.root.clone());
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    let manifest_path = f.root.join("runtime/last-applied.json");
    let saved = f.root.join("runtime/fault-saved-manifest.json");
    std::fs::rename(&manifest_path, &saved).unwrap();
    std::fs::create_dir(&manifest_path).unwrap();
    admin::request(&f.root).unwrap();
    f.backend.poll();
    assert!(!admin::acknowledged(&f.root).unwrap());
    assert_eq!(std::fs::read(saved).unwrap(), manifest);
    assert!(manifest_path.is_dir());
    assert!(admin::requested(&f.root).unwrap());
}

#[test]
#[ignore = "only spawned by admin crash isolation test"]
fn admin_crash_fixture() {
    let root = std::path::PathBuf::from(std::env::var_os("VEYRA_ADMIN_FIXTURE_ROOT").unwrap());
    let role = std::env::var("VEYRA_ADMIN_FIXTURE_ROLE").unwrap();
    let _held = if role == "service" {
        admin::lock(&root, "lifecycle-lock").unwrap()
    } else {
        // 这是独立的实际FD writer，不是sing-box；父测试始终持有其Child句柄。
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(root.join("runtime/held-cache"))
            .unwrap()
    };
    std::fs::write(root.join(format!("{role}-ready")), b"ready").unwrap();
    std::thread::sleep(Duration::from_secs(10));
}
#[test]
fn production_admin_service_crash_does_not_prove_old_writer_dead() {
    // helper-like进程被kill后锁会释放，但独立旧writer仍持FD：必须保留现场拒绝ACK。
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let f = Fixture::new();
    std::fs::create_dir(f.root.join("runtime")).unwrap();
    std::fs::write(f.root.join("runtime/held-cache"), b"old-cache").unwrap();
    let spawn = |role: &str| {
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "production::process_tests::admin_crash_fixture",
                "--ignored",
            ])
            .env("VEYRA_ADMIN_FIXTURE_ROOT", &f.root)
            .env("VEYRA_ADMIN_FIXTURE_ROLE", role)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let child = Owned(child);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !f.root.join(format!("{role}-ready")).exists() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        child
    };
    let mut service = spawn("service");
    let mut writer = spawn("writer");
    assert!(matches!(
        admin::lock(&f.root, "lifecycle-lock"),
        Err(Error::Busy)
    ));
    admin::request(&f.root).unwrap();
    service.0.kill().unwrap();
    service.0.wait().unwrap();
    let _offline = admin::lock(&f.root, "lifecycle-lock").unwrap();
    assert!(writer.0.try_wait().unwrap().is_none());
    assert_eq!(admin::no_runtime(&f.root), Err(Error::HandoffRequired));
    assert!(!admin::acknowledged(&f.root).unwrap());
    assert_eq!(
        std::fs::read(f.root.join("runtime/held-cache")).unwrap(),
        b"old-cache"
    );
    // 有界关闭仅父测试持有的fixture Child；并不作为生产恢复授权。
    drop(writer);
    drop(service);
}

#[test]
fn production_admin_markers_reject_untrusted_files_without_runtime_changes() {
    // 非管理员可写权限、symlink、畸形marker不能触发Stop，也不能签发ACK。
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let path = f.root.join("uninstall-request");
    std::fs::write(&path, b"veyra-uninstall-v1\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o666)).unwrap();
    assert_eq!(admin::requested(&f.root), Err(Error::UnsafeEndpoint));
    std::fs::remove_file(&path).unwrap();
    let target = f.root.join("untrusted");
    std::fs::write(&target, b"do not touch").unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert_eq!(admin::request(&f.root), Err(Error::UnsafeEndpoint));
    assert_eq!(std::fs::read(target).unwrap(), b"do not touch");
    assert!(!f.root.join("runtime").exists());
}

#[test]
fn production_missing_manifest_is_not_proof_of_clean_owner() {
    // 崩溃只留下cache/owner碎片时，缺少Child/manifest不能授权新writer，也不能隐瞒恢复状态。
    let mut f = Fixture::new();
    std::fs::create_dir(f.root.join("kernel-cache")).unwrap();
    std::fs::write(f.root.join("kernel-cache/old-cache"), b"unknown-writer").unwrap();
    let reopened = InstalledBackend::from_assets(Ok(Assets {
        installation: None,
        root: f.root.clone(),
        kernel: std::env::current_exe().unwrap(),
        uid: f.peer.uid,
        gid: f.peer.gid,
        fixture: true,
        spawned: f.spawned.clone(),
    }));
    f.backend = reopened;
    assert!(f.backend.poll().unwrap().recovery_required);
    assert_eq!(
        f.execute(start(&super::tests::config(), 1)).1,
        Err(Error::HandoffRequired)
    );
    assert!(f.spawned.lock().unwrap().is_empty());
    assert!(!f.root.join("owner-session.json").exists());
    assert_eq!(
        std::fs::read(f.root.join("kernel-cache/old-cache")).unwrap(),
        b"unknown-writer"
    );
}

#[test]
fn production_verified_source_exit_released_enables_normal_runtime_only() {
    // 真OS子进程退出/kqueue/reap + 固定文件读取；cfg(test)仅指定自有source root/test exe。
    // 保护伪造Released、存活writer、跨版本/nonce不能开启helper，成功路径复用正式执行器。
    use std::os::unix::fs::PermissionsExt;
    use veyra_core::{
        application::{manual_runtime::RuntimeCommand as Local, runtime_recovery::*},
        domain::StateEpoch,
    };
    let mut source = Fixture::new();
    let c = super::tests::config();
    source.execute(start(&c, 1)).1.unwrap();
    let mut local = source.backend.take_test_runtime();
    for path in [&source.root, &source.root.join("kernel-cache")] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    local.publish_source_session().unwrap();
    let session_path = source.root.join("runtime/source-session.json");
    let original_session = std::fs::read(&session_path).unwrap();
    let mut target = Fixture::new();
    target.backend.require_test_source(source.root.clone());
    assert_eq!(
        target.backend.capabilities().runtime,
        Err(Error::HandoffRequired)
    );
    assert_eq!(target.backend.capabilities().handoff, Ok(()));
    assert_eq!(target.execute(start(&c, 1)).1, Err(Error::HandoffRequired));
    let id = StateEpoch::fresh().unwrap();
    let call = |b: &mut InstalledBackend, action| {
        b.handoff(
            target.peer,
            action,
            Instant::now() + Duration::from_secs(10),
        )
        .1
    };
    let mut forged: SourceSession = serde_json::from_slice(&original_session).unwrap();
    forged.pid += 1;
    std::fs::write(&session_path, serde_json::to_vec(&forged).unwrap()).unwrap();
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Preflight {
                id: id.clone(),
                config: c.clone()
            }
        )
        .is_err()
    );
    std::fs::write(&session_path, &original_session).unwrap();
    call(
        &mut target.backend,
        HandoffAction::Preflight {
            id: id.clone(),
            config: c.clone(),
        },
    )
    .unwrap();
    let fake = TransferTicket {
        id: id.clone(),
        version: c.version(),
        digest: "0".repeat(64),
    };
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Commit {
                release: OwnerTransfer::Released { ticket: fake }
            }
        )
        .is_err()
    );
    assert!(local.snapshot().unwrap().runtime.instance_id.is_some());
    assert!(target.spawned.lock().unwrap().is_empty());
    let (ticket, bundle) = local.prepare_handoff(id, c.version()).unwrap();
    let source_manifest = std::fs::read(source.root.join("runtime/last-applied.json")).unwrap();
    let incarnation = source.root.join("runtime/owner-incarnation.json");
    let original = std::fs::read(&incarnation).unwrap();
    std::fs::write(&incarnation, b"forged-incarnation").unwrap();
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Prepare {
                config: c.clone(),
                ticket: ticket.clone(),
                bundle: Box::new(bundle.clone())
            }
        )
        .is_err()
    );
    std::fs::write(&incarnation, &original).unwrap();
    call(
        &mut target.backend,
        HandoffAction::Prepare {
            config: c.clone(),
            ticket: ticket.clone(),
            bundle: Box::new(bundle.clone()),
        },
    )
    .unwrap();
    // Prepare不是Released；IPC DTO自称release仍不够。
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Commit {
                release: OwnerTransfer::Released {
                    ticket: ticket.clone()
                }
            }
        )
        .is_err()
    );
    let release = local.release_handoff(&ticket).unwrap();
    let mut wrong = ticket.clone();
    wrong.version.selection.0.revision += 1;
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Commit {
                release: OwnerTransfer::Released { ticket: wrong }
            }
        )
        .is_err()
    );
    let source_record = source.root.join("runtime/owner-transfer.json");
    let record_bytes = std::fs::read(&source_record).unwrap();
    let mut wrong_epoch = ticket.clone();
    wrong_epoch.version.config.0.epoch = StateEpoch::fresh().unwrap();
    std::fs::write(
        &source_record,
        serde_json::to_vec(&OwnerTransfer::Released {
            ticket: wrong_epoch,
        })
        .unwrap(),
    )
    .unwrap();
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Commit {
                release: release.clone()
            }
        )
        .is_err()
    );
    std::fs::write(&source_record, &record_bytes).unwrap();
    let mut different_peer = target.peer;
    different_peer.start.1 += 1;
    assert!(
        target
            .backend
            .handoff(
                different_peer,
                HandoffAction::Commit {
                    release: release.clone()
                },
                Instant::now() + Duration::from_secs(5)
            )
            .1
            .is_err()
    );
    let sealed = source.root.join("runtime/owner-bundle.json");
    let sealed_bytes = std::fs::read(&sealed).unwrap();
    std::fs::write(&sealed, b"{}").unwrap();
    assert!(
        call(
            &mut target.backend,
            HandoffAction::Commit {
                release: release.clone()
            }
        )
        .is_err()
    );
    std::fs::write(&sealed, &sealed_bytes).unwrap();
    call(
        &mut target.backend,
        HandoffAction::Commit {
            release: release.clone(),
        },
    )
    .unwrap();
    call(&mut target.backend, HandoffAction::Commit { release }).unwrap(); // 同票据半响应重试
    assert_eq!(target.backend.capabilities().runtime, Ok(()));
    assert!(local.execute(Local::Start, |_| {}).is_err());
    let backend = std::mem::replace(
        &mut target.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = std::sync::Arc::new(super::host::Host::new(target.peer.uid, backend));
    let socket = super::tests::SocketFixture::new();
    let exchange = |command| {
        let listener = socket.listener.try_clone().unwrap();
        let host = host.clone();
        let worker = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            host.connection(stream).unwrap();
        });
        let reply = socket.client().request(command).unwrap();
        worker.join().unwrap();
        reply
    };
    let operation = |id| {
        let until = Instant::now() + Duration::from_secs(5);
        loop {
            if let Response::Operation(Operation::Completed(result)) =
                exchange(Command::Operation { request_id: id })
            {
                break result;
            }
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    assert_eq!(
        exchange(start(&c, 10)),
        Response::Operation(Operation::Inflight)
    );
    let status = operation(10).unwrap();
    assert!(status.applied.is_some());
    assert_eq!(
        exchange(start(&c, 11)),
        Response::Operation(Operation::Inflight)
    );
    assert_eq!(operation(11).unwrap().instance, status.instance);
    assert_eq!(target.spawned.lock().unwrap().len(), 2);
    let mut next = c.clone();
    next.state.config_revision += 1;
    std::fs::write(&incarnation, b"changed-after-commit").unwrap();
    assert_eq!(
        exchange(Command::Apply {
            request_id: 13,
            instance: status.instance.unwrap(),
            expected: c.version(),
            config: next.clone()
        }),
        Response::Operation(Operation::Inflight)
    );
    assert!(operation(13).is_err());
    assert!(matches!(exchange(Command::Status),Response::Status(s) if s.recovery_required));
    assert_eq!(target.spawned.lock().unwrap().len(), 2);
    std::fs::write(&incarnation, &original).unwrap();
    assert_eq!(
        exchange(Command::Apply {
            request_id: 14,
            instance: status.instance.unwrap(),
            expected: c.version(),
            config: next.clone()
        }),
        Response::Operation(Operation::Inflight)
    );
    let status = operation(14).unwrap();
    assert_eq!(status.applied, Some(next.version()));
    assert_eq!(target.spawned.lock().unwrap().len(), 4);
    // Quit复用真实Client清理合同：错误票据不能Stop；正确票据查询实际结果与Status。
    socket.listener.set_nonblocking(true).unwrap();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let finished = done.clone();
    let listener = socket.listener.try_clone().unwrap();
    let server = host.clone();
    let serving = std::thread::spawn(move || {
        while !finished.load(std::sync::atomic::Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let _ = server.connection(stream);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(e) => panic!("fixture accept: {e}"),
            }
        }
    });
    let mut wrong = ticket.clone();
    wrong.digest = "0".repeat(64);
    let client = socket.client();
    let denied = client.stop_released(&wrong);
    let still = client.request(Command::Status);
    // 模拟Stop已送达但其响应丢失；实际socket请求已完成，客户端只能查询同一个operation。
    let mut dropped = false;
    let stopped = super::transport::stop_released(&ticket, |command| {
        let stopping = matches!(command, Command::Stop { .. });
        let result = client.request(command);
        if stopping {
            dropped = true;
            Err(Error::Timeout)
        } else {
            result
        }
    });
    let repeated = client.stop_released(&ticket);
    done.store(true, std::sync::atomic::Ordering::SeqCst);
    serving.join().unwrap();
    assert_eq!(denied, Err(Error::HandoffRequired));
    assert!(matches!(still, Ok(Response::Status(s)) if s.instance == status.instance));
    assert!(dropped);
    assert_eq!(stopped, Ok(()));
    assert_eq!(repeated, Ok(()));
    drop(host);
    assert_eq!(
        std::fs::read(source.root.join("runtime/last-applied.json")).unwrap(),
        source_manifest
    );
    assert_eq!(std::fs::read(&incarnation).unwrap(), original);
    assert_eq!(std::fs::read(session_path).unwrap(), original_session);
    drop(local);
}

#[test]
fn production_handoff_preflight_capability_is_independent_of_start_authorization() {
    // 真Unix Hello/Preflight能在Start仍未授权时成功；预检不freeze/Stop活源child。
    use std::os::unix::fs::PermissionsExt;
    let mut source = Fixture::new();
    let c = super::tests::config();
    source.execute(start(&c, 1)).1.unwrap();
    let local = source.backend.take_test_runtime();
    local.publish_source_session().unwrap();
    std::fs::set_permissions(&source.root, std::fs::Permissions::from_mode(0o700)).unwrap();
    let before = std::fs::read(source.root.join("runtime/last-applied.json")).unwrap();
    let mut target = Fixture::new();
    target.backend.require_test_source(source.root.clone());
    let backend = std::mem::replace(
        &mut target.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = std::sync::Arc::new(super::host::Host::new(target.peer.uid, backend));
    let socket = super::tests::SocketFixture::new();
    let exchange = |command| {
        let listener = socket.listener.try_clone().unwrap();
        let host = host.clone();
        let worker = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let _ = host.connection(stream);
        });
        let reply = socket.client().request(command);
        worker.join().unwrap();
        reply.unwrap()
    };
    assert!(
        matches!(exchange(Command::Hello),Response::Hello(cap) if cap.handoff.is_ok() && cap.runtime==Err(Error::HandoffRequired))
    );
    let preflight = || Command::Handoff {
        action: Box::new(HandoffAction::Preflight {
            id: veyra_core::domain::StateEpoch::fresh().unwrap(),
            config: c.clone(),
        }),
    };
    std::fs::set_permissions(&source.root, std::fs::Permissions::from_mode(0o777)).unwrap();
    assert!(matches!(
        exchange(preflight()),
        Response::Handoff(Err(Error::UnsafeEndpoint))
    ));
    std::fs::set_permissions(&source.root, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(exchange(preflight()), Response::Handoff(Ok(_))));
    assert!(local.snapshot().unwrap().runtime.instance_id.is_some());
    assert!(!source.root.join("runtime/owner-transfer.json").exists());
    assert!(!source.root.join("runtime/owner-incarnation.json").exists());
    assert_eq!(
        std::fs::read(source.root.join("runtime/last-applied.json")).unwrap(),
        before
    );
    assert!(target.spawned.lock().unwrap().is_empty());
    drop(host);
    drop(local);
}

#[test]
fn recovery_queries_do_not_drain_live_writer_or_acknowledge_uninstall() {
    // 用户查看恢复状态/预检不能成为停止代理的操作；真实自有child及cache保持活跃。
    let mut f = Fixture::new();
    let c = super::tests::config();
    let live = f.execute(start(&c, 1)).1.unwrap();
    f.backend.test_admin_root(f.root.clone());
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    admin::request(&f.root).unwrap();
    let query = f.backend.handoff(
        f.peer,
        HandoffAction::Query,
        Instant::now() + Duration::from_secs(5),
    );
    assert!(query.1.is_ok());
    assert_eq!(query.0.instance, live.instance);
    let preflight = f.backend.handoff(
        f.peer,
        HandoffAction::Preflight {
            id: veyra_core::domain::StateEpoch::fresh().unwrap(),
            config: c,
        },
        Instant::now() + Duration::from_secs(5),
    );
    assert_eq!(preflight.1, Err(Error::HandoffRequired));
    assert_eq!(preflight.0.instance, live.instance);
    assert!(!admin::acknowledged(&f.root).unwrap());
    assert!(!f.root.join("runtime/owner-transfer.json").exists());
    assert_eq!(
        std::fs::read(f.root.join("runtime/last-applied.json")).unwrap(),
        manifest
    );
    // 真正管理员poll仍按原合同Stop/reap/封存，不能因只读修复而禁用它。
    assert!(f.backend.poll().unwrap().instance.is_none());
    assert!(admin::acknowledged(&f.root).unwrap());
}

#[test]
fn recovery_query_after_backend_restart_is_read_only_and_peer_bound() {
    // 重启无handle只读展示历史Closed；不能升级为可Start，不得接受另一个OS会话。
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new();
    let c = super::tests::config();
    f.execute(start(&c, 1)).1.unwrap();
    let mut runtime = f.backend.take_test_runtime();
    let (ticket, _) = runtime
        .prepare_handoff(
            veyra_core::domain::StateEpoch::fresh().unwrap(),
            c.version(),
        )
        .unwrap();
    drop(runtime);
    f.backend = InstalledBackend::from_assets(Ok(Assets {
        installation: None,
        root: f.root.clone(),
        kernel: std::env::current_exe().unwrap(),
        uid: f.peer.uid,
        gid: f.peer.gid,
        fixture: true,
        spawned: f.spawned.clone(),
    }));
    f.backend.require_test_source(f.root.clone());
    f.backend.test_admin_root(f.root.clone());
    admin::request(&f.root).unwrap();
    let record = std::fs::read(f.root.join("runtime/owner-transfer.json")).unwrap();
    let session = f.root.join("owner-session.json");
    let query = |f: &mut Fixture, peer| {
        f.backend.handoff(
            peer,
            HandoffAction::Query,
            Instant::now() + Duration::from_secs(5),
        )
    };
    let peer = f.peer;
    let (status, result) = query(&mut f, peer);
    assert!(status.recovery_required);
    assert_eq!(
        result.unwrap().owner,
        Some(veyra_core::application::runtime_recovery::OwnerTransfer::Closed { ticket })
    );
    let mut other = f.peer;
    other.start.1 += 1;
    assert_eq!(query(&mut f, other).1, Err(Error::Unauthorized));
    std::fs::set_permissions(&session, std::fs::Permissions::from_mode(0o666)).unwrap();
    assert_eq!(query(&mut f, peer).1, Err(Error::UnsafeEndpoint));
    std::fs::set_permissions(&session, std::fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(f.execute(start(&c, 2)).1, Err(Error::HandoffRequired));
    assert_eq!(
        std::fs::read(f.root.join("runtime/owner-transfer.json")).unwrap(),
        record
    );
    assert!(!admin::acknowledged(&f.root).unwrap());
    assert_eq!(f.spawned.lock().unwrap().len(), 2);
}

#[test]
fn empty_root_with_unlinked_live_writer_does_not_authorize_cold_start() {
    // 真实FD已unlink且目录为空，旧writer仍活；空目录只能描述文件，不能证明从未启动。
    struct Owned(std::process::Child);
    impl Drop for Owned {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    let mut f = Fixture::new();
    std::fs::create_dir(f.root.join("runtime")).unwrap();
    std::fs::write(f.root.join("runtime/held-cache"), b"owned fixture").unwrap();
    let mut child = Owned(
        std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "production::process_tests::admin_crash_fixture",
                "--ignored",
            ])
            .env("VEYRA_ADMIN_FIXTURE_ROOT", &f.root)
            .env("VEYRA_ADMIN_FIXTURE_ROLE", "writer")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap(),
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !f.root.join("writer-ready").exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::remove_file(f.root.join("runtime/held-cache")).unwrap();
    std::fs::remove_dir(f.root.join("runtime")).unwrap();
    std::fs::remove_file(f.root.join("writer-ready")).unwrap();
    assert!(child.0.try_wait().unwrap().is_none());
    let assets = Assets {
        installation: None,
        root: f.root.clone(),
        kernel: std::env::current_exe().unwrap(),
        uid: f.peer.uid,
        gid: f.peer.gid,
        fixture: true,
        spawned: f.spawned.clone(),
    };
    assert_eq!(assets.pristine_root(), Ok(()));
    f.backend = InstalledBackend::from_assets(Ok(assets));
    f.backend.require_test_source(f.root.clone()); // 正式requires_source=true合同，不绕过准入。
    assert_eq!(
        f.execute(start(&super::tests::config(), 1)).1,
        Err(Error::HandoffRequired)
    );
    assert!(f.spawned.lock().unwrap().is_empty());
    assert_eq!(std::fs::read_dir(&f.root).unwrap().count(), 0);
    assert!(child.0.try_wait().unwrap().is_none());
}

#[test]
fn owned_kernel_exec_has_real_product_basename() {
    // 真正exec私有目录里的测试exe字节副本，不用argv[0]/exec-a；不执行正式sing-box。
    use std::os::unix::fs::PermissionsExt;
    let resources = Fixture::new();
    let binary = resources
        .root
        .join(veyra_core::application::runtime_recovery::KERNEL_EXECUTABLE);
    std::fs::copy(std::env::current_exe().unwrap(), &binary).unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
    let mut f = Fixture::new();
    f.backend = InstalledBackend::from_assets(Ok(Assets {
        installation: None,
        root: f.root.clone(),
        kernel: binary.clone(),
        uid: f.peer.uid,
        gid: f.peer.gid,
        fixture: true,
        spawned: f.spawned.clone(),
    }));
    let status = f.execute(start(&super::tests::config(), 1)).1.unwrap();
    let pid = *f.spawned.lock().unwrap().last().unwrap() as i32;
    let mut path = [0u8; 4096];
    assert!(unsafe { libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) } > 0);
    let actual = std::path::Path::new(
        std::ffi::CStr::from_bytes_until_nul(&path)
            .unwrap()
            .to_str()
            .unwrap(),
    );
    assert_eq!(actual.file_name().unwrap(), "veyra-sing-box");
    assert_eq!(actual, binary.canonicalize().unwrap());
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    assert_eq!(
        unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut info as *mut libc::proc_bsdinfo).cast(),
                size,
            )
        },
        size
    );
    let name = unsafe { std::ffi::CStr::from_ptr(info.pbi_comm.as_ptr()) };
    assert_eq!(name.to_str().unwrap(), "veyra-sing-box");
    println!(
        "OS fixture: executable basename=veyra-sing-box; proc_bsdinfo comm=veyra-sing-box; real exec, no argv alias"
    );
    f.execute(Command::Stop {
        request_id: 2,
        instance: status.instance.unwrap(),
    })
    .1
    .unwrap();
    assert!(f.backend.poll().unwrap().instance.is_none());
}

#[test]
fn repeated_start_id_rechecks_exited_child_without_idle_poll_or_respawn() {
    // 保护断线后重试旧Start不能把已退出内核报告为Ready；真实OS child/socket，禁用后台poll稳定复现窗口。
    use std::sync::atomic::Ordering;
    let mut f = Fixture::new();
    let backend = std::mem::replace(
        &mut f.backend,
        InstalledBackend::from_assets(Err(Error::Unavailable)),
    );
    let host = super::host::Host::new(f.peer.uid, backend);
    host.idle_poll.store(false, Ordering::Release);
    let socket = super::tests::SocketFixture::new();
    let call = |command| {
        std::thread::scope(|scope| {
            let connection = scope.spawn(|| {
                let (stream, _) = socket.listener.accept().unwrap();
                host.connection(stream).unwrap();
            });
            let result = socket.client().request(command).unwrap();
            connection.join().unwrap();
            result
        })
    };
    let mut c = super::tests::config();
    c.state.nodes[0].server = "later-exit.invalid".into();
    let command = start(&c, 500);
    assert_eq!(
        call(command.clone()),
        Response::Operation(Operation::Inflight)
    );
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        if let Response::Operation(Operation::Completed(Ok(_))) =
            call(Command::Operation { request_id: 500 })
        {
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    let pid = *f.spawned.lock().unwrap().last().unwrap();
    // 只观察本测试持有的child NOTE_EXIT；不reap、不把proc查询缺失当作退出证明。
    use std::os::fd::{AsRawFd, OwnedFd};
    let fd = unsafe { libc::kqueue() };
    assert!(fd >= 0);
    let queue = unsafe { OwnedFd::from_raw_fd(fd) };
    let change = libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ENABLE,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    assert_eq!(
        unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                &change,
                1,
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        },
        0
    );
    let mut event: libc::kevent = unsafe { std::mem::zeroed() };
    let timeout = libc::timespec {
        tv_sec: 3,
        tv_nsec: 0,
    };
    assert_eq!(
        unsafe {
            libc::kevent(
                queue.as_raw_fd(),
                std::ptr::null(),
                0,
                &mut event,
                1,
                &timeout,
            )
        },
        1
    );
    let exited_pid = event.ident;
    assert_eq!(exited_pid, pid as usize);
    assert_ne!(event.fflags & libc::NOTE_EXIT, 0);
    assert!(
        matches!(
            call(Command::Status),
            Response::Status(Status {
                applied: Some(_),
                ..
            })
        ),
        "deliberately stale poll cache"
    );
    assert_eq!(call(command), Response::Operation(Operation::Inflight));
    loop {
        if let Response::Operation(Operation::Completed(result)) =
            call(Command::Operation { request_id: 500 })
        {
            assert_eq!(result, Err(Error::StaleInstance));
            break;
        }
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(
        call(Command::Status),
        Response::Status(Status {
            instance: None,
            applied: None,
            ..
        })
    ));
    assert_eq!(
        f.spawned.lock().unwrap().len(),
        2,
        "one check and one run, never respawn"
    );
    drop(host);
}

#[test]
fn missing_installation_identity_never_reports_clean_stopped() {
    // 保护升级/半安装诊断：未知安装不能显示可安全重启，真正NotInstalled仍能准确呈现。
    for error in [Error::HandoffRequired, Error::UnsafeEndpoint] {
        let mut backend = InstalledBackend::from_assets(Err(error));
        assert!(backend.poll().unwrap().recovery_required);
        assert_eq!(backend.capabilities().runtime, Err(error));
        let peer = Fixture::new().peer;
        let (_, result) = backend.execute(
            peer,
            start(&super::tests::config(), 1),
            Instant::now() + Duration::from_secs(1),
        );
        assert_eq!(result, Err(error));
    }
    let mut absent = InstalledBackend::from_assets(Err(Error::NotInstalled));
    assert!(!absent.poll().unwrap().recovery_required);
    assert_eq!(absent.capabilities().runtime, Err(Error::NotInstalled));
}

#[test]
fn first_bootstrap_cannot_freeze_a_live_fixture_writer() {
    // 保护正在运行的手动/受管writer不被归入首次空历史：真实Child、FD4和cache不变。
    use veyra_core::application::manual_runtime::RuntimeCommand as LocalCommand;
    let mut f = Fixture::new();
    let config = super::tests::config();
    let (_, result) = f.execute(start(&config, 1));
    result.unwrap();
    let mut runtime = f.backend.take_test_runtime();
    let before = runtime.snapshot().unwrap();
    let manifest = std::fs::read(f.root.join("runtime/last-applied.json")).unwrap();
    let pid = *f.spawned.lock().unwrap().last().unwrap();
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, 0);
    assert!(
        runtime
            .freeze_first_bootstrap(
                veyra_core::domain::StateEpoch::fresh().unwrap(),
                veyra_core::domain::StateEpoch::fresh().unwrap(),
                config.version(),
            )
            .is_err()
    );
    assert_eq!(runtime.owner_transfer().unwrap(), None);
    assert_eq!(
        runtime.snapshot().unwrap().runtime.instance_id,
        before.runtime.instance_id
    );
    assert_eq!(unsafe { libc::kill(pid as i32, 0) }, 0);
    assert_eq!(
        std::fs::read(f.root.join("runtime/last-applied.json")).unwrap(),
        manifest
    );
    runtime.execute(LocalCommand::Stop, |_| {}).unwrap();
    drop(runtime);
}
