//! 只验证Veyra的选择/测速/草稿隔离，不重新认证内核协议栈。
use crate::{services::AppServices, state_bridge::AppEvent};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use veyra_core::{
    application::{
        draft_latency::DraftLatencyError,
        manual_runtime::{ManualRuntimeSnapshot, RuntimeCommand},
        network::NetworkError,
        proxies::TestDisposition,
        runtime_snapshot::RuntimeStatus,
    },
    domain::{
        AppState, GroupSelectionMode, NodeId, OutboundId, PoolId, ProtocolOptions, ProxyProtocol,
    },
    storage::JsonStateStore,
    subscription::outbound::{OutboundClient, RoutePolicy},
};

struct NodeFixture {
    address: SocketAddr,
    stop: Arc<AtomicBool>,
    listener: Option<thread::JoinHandle<()>>,
    accepted: mpsc::Receiver<()>,
}
impl NodeFixture {
    fn new(name: &'static str, mode: u8, upstream: Arc<Mutex<Option<SocketAddr>>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let ending = stop.clone();
        let (send, accepted) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut connections = vec![];
            while !ending.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut socket, _)) => {
                        socket.set_nonblocking(false).unwrap();
                        socket
                            .set_read_timeout(Some(Duration::from_secs(8)))
                            .unwrap();
                        socket
                            .set_write_timeout(Some(Duration::from_secs(8)))
                            .unwrap();
                        let _ = send.send(());
                        let upstream = upstream.clone();
                        let ending = ending.clone();
                        connections.push(thread::spawn(move||{
                            let Ok(headers)=headers(&mut socket) else {return;};
                            if headers.starts_with("CONNECT 192.0.2.1:9 ") {
                                // 文档保留地址只作为隧道内marker，由自有节点应答，不连接该地址。
                                socket.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").unwrap();
                                let request=self::headers(&mut socket).unwrap();
                                assert!(request.contains("/owned-selection-marker"));
                                let reply=format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{name}",name.len());
                                socket.write_all(reply.as_bytes()).unwrap();
                            } else if headers.starts_with("CONNECT ") {
                                assert!(headers.starts_with("CONNECT www.gstatic.com:443 "),"unexpected TLS target");
                                if mode==1 {let _=socket.write_all(b"HTTP/1.1 503 Fixture\r\nContent-Length: 0\r\n\r\n");return;}
                                if mode==2 {while !ending.load(Ordering::SeqCst) {thread::park_timeout(Duration::from_millis(20));}return;}
                                let mut target=TcpStream::connect(upstream.lock().unwrap().expect("owned mixed")).unwrap();
                                target.set_read_timeout(Some(Duration::from_secs(8))).unwrap();target.set_write_timeout(Some(Duration::from_secs(8))).unwrap();
                                target.write_all(b"CONNECT www.gstatic.com:443 HTTP/1.1\r\nHost: www.gstatic.com:443\r\n\r\n").unwrap();
                                assert!(self::headers(&mut target).unwrap().starts_with("HTTP/1.1 200"));
                                socket.write_all(b"HTTP/1.1 200 Connection established\r\n\r\n").unwrap();
                                let input=socket.try_clone().unwrap();let output=target.try_clone().unwrap();
                                let relay=thread::spawn(move||transfer(input,output));let down=transfer(target,socket);let up=relay.join().unwrap();
                                eprintln!("P208_TLS node={name} up={up} down={down}");
                            }else{
                                // 自有HTTP代理的marker证明每条新连接实际经过所选NodeId。
                                assert!(headers.contains("127.0.0.1:9"));
                                let reply=format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{name}",name.len());socket.write_all(reply.as_bytes()).unwrap();
                            }
                        }));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::park_timeout(Duration::from_millis(2))
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            }
            for connection in connections {
                connection.join().unwrap();
            }
        });
        Self {
            address,
            stop,
            listener: Some(handle),
            accepted,
        }
    }
}
impl Drop for NodeFixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.listener.take().unwrap().join().unwrap();
    }
}
fn headers(socket: &mut TcpStream) -> std::io::Result<String> {
    let mut bytes = vec![];
    while !bytes.ends_with(b"\r\n\r\n") && bytes.len() < 8192 {
        let mut b = [0];
        socket.read_exact(&mut b)?;
        bytes.push(b[0]);
    }
    Ok(String::from_utf8(bytes).unwrap())
}
fn transfer(mut input: TcpStream, mut output: TcpStream) -> usize {
    let mut bytes = [0; 4096];
    let mut count = 0;
    loop {
        match input.read(&mut bytes) {
            Ok(0) => break,
            Ok(n) => {
                if output.write_all(&bytes[..n]).is_err() {
                    break;
                }
                count += n;
            }
            Err(_) => break,
        }
    }
    let _ = output.shutdown(std::net::Shutdown::Write);
    count
}
fn ready(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    different: Option<&str>,
) -> ManualRuntimeSnapshot {
    let until = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(AppEvent::Runtime(event)) = receiver.try_recv() {
            if let Some(Err(error)) = event.result {
                panic!("runtime: {error:?}");
            }
            if let Ok(snapshot) = event.snapshot
                && snapshot.runtime.status == RuntimeStatus::Ready
                && snapshot
                    .runtime
                    .instance_id
                    .as_ref()
                    .is_some_and(|id| Some(id.0.as_str()) != different)
            {
                return snapshot;
            }
        }
        assert!(Instant::now() < until, "Ready timeout");
        thread::yield_now();
    }
}
fn selected(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>,
    node: &str,
) -> ManualRuntimeSnapshot {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(AppEvent::Runtime(event)) = receiver.try_recv()
            && let Ok(snapshot) = event.snapshot
            && snapshot
                .confirmed_manual_selections
                .get(&PoolId("manual".into()))
                == Some(&NodeId(node.into()))
        {
            return snapshot;
        }
        assert!(Instant::now() < until, "confirmed selection timeout");
        thread::yield_now();
    }
}
/// 保护实际选择新连接、测速不改选择、真实child草稿清理及SQLite重建恢复。
#[test]
#[ignore = "requires pinned kernel and authorized fixed HTTPS target; no OS network changes"]
fn real_selection_latency_draft_cleanup() {
    let kernel = std::env::var("VEYRA_SING_BOX_PATH").expect("pinned kernel");
    assert!(!kernel.is_empty());
    let root = std::path::PathBuf::from("/private/tmp").join(format!(
        "veyra-p208-native-{:?}",
        veyra_core::domain::StateEpoch::fresh().unwrap()
    ));
    let upstream = Arc::new(Mutex::new(None));
    let a = NodeFixture::new("node-a", 0, upstream.clone());
    let b = NodeFixture::new("node-b", 1, upstream.clone());
    let mut c = NodeFixture::new("node-c", 2, upstream.clone());
    let mut value = serde_json::to_value(AppState::empty()).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../veyra-core/tests/fixtures/compiler/p2-02b.json"
    ))
    .unwrap();
    for (key, entry) in fixture.as_object().unwrap() {
        value[key] = entry.clone();
    }
    value["routes"] = serde_json::json!([{"id":"tls-target","name":"owned relay route","enabled":true,"priority":0,"matcher":{"kind":"domain","values":["www.gstatic.com"]},"target":{"kind":"direct"}}]);
    let mut state: AppState = serde_json::from_value(value).unwrap();
    state.pools.retain(|p| p.id.0 == "manual");
    state.pools[0].name = "所有-手动".into();
    state.active_subscription_id = Some(veyra_core::domain::SubscriptionId("subscription".into()));
    for (node, fixture) in state.nodes.iter_mut().zip([&a, &b, &c]) {
        node.protocol = ProxyProtocol::Http;
        node.server = "127.0.0.1".into();
        node.port = fixture.address.port();
        node.options = ProtocolOptions::Http {
            username: None,
            password: None,
            tls: false,
        };
        node.tls = None;
        node.name = node.id.0.clone();
    }
    state.app_config.behavior.latency.test_url.0 = "https://www.gstatic.com/generate_204".into();
    state.app_config.behavior.latency.timeout_ms = 3000;
    state.validate().unwrap();
    JsonStateStore::new(root.join("state.json"))
        .unwrap()
        .commit(&state)
        .unwrap();
    let (services, mut events) = AppServices::new(root.clone()).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    services.manual_runtime.submit(1, RuntimeCommand::Start);
    let first = ready(&mut events, None);
    let old = services
        .manual_runtime
        .outbound_proxy_source()
        .current()
        .unwrap();
    *upstream.lock().unwrap() = Some(old.address());
    eprintln!(
        "P208_READY root={} instance={} mixed={} controller={}",
        root.display(),
        old.instance_id(),
        old.address(),
        first.endpoints.as_ref().unwrap().controller
    );
    let request = services
        .proxies
        .selection_request(
            &first,
            PoolId("manual".into()),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin,
        )
        .unwrap();
    services
        .manual_runtime
        .select_manual(request)
        .recv_timeout(Duration::from_secs(20))
        .unwrap()
        .unwrap();
    let confirmed = selected(&mut events, "b");
    let client = OutboundClient::new(
        RoutePolicy::ViaRunningProxy(old.clone()),
        &Default::default(),
    )
    .unwrap();
    let marker = rt.block_on(client.resource("http://192.0.2.1:9/owned-selection-marker"));
    eprintln!("P208_MARKER {marker:?}");
    assert!(
        matches!(marker,Ok(veyra_core::subscription::FetchResult::Modified{ref body,..})if body=="node-b")
    );
    eprintln!("P208_SELECTION_PASS Controller confirmed NodeId=b; new connection marker=node-b");
    let request = services
        .proxies
        .begin_test(&confirmed, OutboundId::Pool(PoolId("manual".into())))
        .unwrap();
    let mut completed = vec![];
    rt.block_on(services.proxies.run_test_with_progress(request, |result| {
        eprintln!(
            "P208_PROGRESS node={} result={:?} disposition={:?}",
            result.node.0, result.result, result.disposition
        );
        completed.push(result);
    }));
    let a_success = completed.iter().any(|r| {
        r.node.0 == "a"
            && r.result
                .as_ref()
                .is_ok_and(|latency| latency.milliseconds.is_some_and(|ms| ms > 0))
            && matches!(r.disposition, TestDisposition::Recorded)
    });
    assert!(a_success, "real NodeId=a HTTPS latency must succeed");
    assert_eq!(completed.len(), 3);
    assert!(
        completed
            .iter()
            .any(|r| r.node.0 == "b" && r.result.is_err())
    );
    assert!(
        completed
            .iter()
            .any(|r| r.node.0 == "c" && r.result.is_err())
    );
    let after = services.proxies.snapshot(&confirmed).unwrap();
    assert_eq!(
        after
            .entries
            .iter()
            .find(|e| e.outbound.id == OutboundId::Pool(PoolId("manual".into())))
            .unwrap()
            .current,
        Some(OutboundId::Node(NodeId("b".into())))
    );
    let saved = services.snapshots.snapshot().unwrap();
    let draft = veyra_core::subscription::ProxyNodeDraft {
        name: "owned draft".into(),
        protocol: ProxyProtocol::Http,
        server: "127.0.0.1".into(),
        port: a.address.port(),
        options: ProtocolOptions::Http {
            username: None,
            password: None,
            tls: false,
        },
        transport: None,
        tls: None,
    };
    let main_bytes = std::fs::read(root.join("state.json")).unwrap();
    let mut session = services.draft_latency_session().unwrap();
    let draft_root = session.root().to_path_buf();
    let result = session.test(
        draft.clone(),
        &saved.app_config.behavior.latency,
        &Default::default(),
    );
    assert!(
        result
            .result
            .as_ref()
            .is_ok_and(|r| r.milliseconds.is_some_and(|ms| ms > 0))
    );
    assert!(result.cleanup.is_ok());
    assert!(!draft_root.exists());
    assert!(old.is_valid());
    assert_eq!(main_bytes, std::fs::read(root.join("state.json")).unwrap());
    eprintln!("P208_DRAFT_SUCCESS main instance unchanged; draft root removed");
    let mut delayed = draft;
    delayed.port = c.address.port();
    let mut prefs = saved.app_config.behavior.latency.clone();
    prefs.timeout_ms = 200;
    let mut session = services.draft_latency_session().unwrap();
    let timeout_root = session.root().to_path_buf();
    let result = session.test(delayed.clone(), &prefs, &Default::default());
    assert!(
        matches!(
            &result.result,
            Err(DraftLatencyError::Network(NetworkError::Controller(
                veyra_core::singbox::clash_api::ClashApiError::Request(
                    veyra_core::subscription::FetchError::Timeout
                )
            )))
        ),
        "typed timeout: {:?}",
        result.result
    );
    eprintln!("P208_DRAFT_TIMEOUT typed={:?}", result.result);
    assert!(result.cleanup.is_ok());
    assert!(!timeout_root.exists());
    for _ in c.accepted.try_iter() {}
    let mut session = services.draft_latency_session().unwrap();
    let cancel_root = session.root().to_path_buf();
    let cancel = veyra_core::application::proxies::TestCancellation::new();
    let watcher = thread::scope(|scope| {
        let token = cancel.clone();
        let signal = &mut c.accepted;
        scope.spawn(move || {
            signal.recv_timeout(Duration::from_secs(20)).unwrap();
            token.cancel();
        });
        session.test(delayed.clone(), &saved.app_config.behavior.latency, &cancel)
    });
    assert!(matches!(watcher.result, Err(DraftLatencyError::Cancelled)));
    eprintln!("P208_DRAFT_CANCEL typed={:?}", watcher.result);
    assert!(watcher.cleanup.is_ok());
    assert!(!cancel_root.exists());
    assert!(old.is_valid());
    eprintln!("P208_DRAFT_TIMEOUT_CANCEL roots removed; main instance remains valid");
    // 关闭编辑器使用同一公开取消句柄；必须等正式 worker 清理后才能开始下一次草稿。
    for _ in c.accepted.try_iter() {}
    let (cancel, receiver) = services
        .test_draft(delayed.clone(), saved.app_config.behavior.latency.clone())
        .unwrap();
    c.accepted.recv_timeout(Duration::from_secs(20)).unwrap();
    cancel.cancel();
    let closed = receiver.blocking_recv().unwrap();
    assert!(matches!(closed.result, Err(DraftLatencyError::Cancelled)));
    assert!(closed.cleanup.is_ok());
    assert_eq!(
        std::fs::read_dir(root.join("draft-latency"))
            .unwrap()
            .count(),
        0
    );
    assert!(old.is_valid());
    assert_eq!(main_bytes, std::fs::read(root.join("state.json")).unwrap());
    eprintln!(
        "P208_WORKER_CLOSE typed={:?}; roots removed; main unchanged",
        closed.result
    );
    services.manual_runtime.submit(3, RuntimeCommand::Restart);
    let second = ready(&mut events, Some(old.instance_id()));
    assert!(!old.is_valid());
    let source = services.manual_runtime.outbound_proxy_source();
    let active = source.current().unwrap();
    let endpoints = second.endpoints.unwrap();
    for _ in c.accepted.try_iter() {}
    let (_, receiver) = services
        .test_draft(delayed, saved.app_config.behavior.latency.clone())
        .unwrap();
    c.accepted.recv_timeout(Duration::from_secs(20)).unwrap();
    assert_eq!(
        std::fs::read_dir(root.join("draft-latency"))
            .unwrap()
            .count(),
        1
    );
    services.request_closing();
    let draft_done = services
        .draft_quit_cleanup()
        .expect("in-flight draft worker");
    assert!(draft_done.recv_timeout(Duration::from_secs(30)).unwrap());
    let quitting = receiver.blocking_recv().unwrap();
    assert!(matches!(quitting.result, Err(DraftLatencyError::Cancelled)));
    assert!(quitting.cleanup.is_ok());
    assert_eq!(
        std::fs::read_dir(root.join("draft-latency"))
            .unwrap()
            .count(),
        0
    );
    eprintln!(
        "P208_WORKER_QUIT typed={:?}; cleanup=true; draft root removed",
        quitting.result
    );
    services
        .manual_runtime
        .shutdown()
        .recv_timeout(Duration::from_secs(30))
        .unwrap()
        .unwrap();
    assert!(source.current().is_none());
    assert!(!active.is_valid());
    drop(session);
    drop(services);
    TcpListener::bind(endpoints.mixed).unwrap();
    TcpListener::bind(endpoints.controller).unwrap();
    let (restored, _) = AppServices::new(root.clone()).unwrap();
    let restored_snapshot = restored.proxies.snapshot(&second).unwrap();
    assert!(
        restored_snapshot
            .entries
            .iter()
            .find(|e| e.outbound.id == OutboundId::Node(NodeId("a".into())))
            .unwrap()
            .history
            .iter()
            .any(|r| r.outcome.milliseconds().is_some())
    );
    drop(restored);
    drop(a);
    drop(b);
    drop(c);
    std::fs::remove_dir_all(&root).unwrap();
    assert!(!root.exists());
    eprintln!(
        "P208_QUIT_REOPEN_PASS replaced ability revoked; owned ports released, history reconstructed, fixture threads joined, root removed"
    );
}
