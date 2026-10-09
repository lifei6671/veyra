//! 自有 loopback 返回器只模拟健康/断开；实际 check/run/API/选择均使用固定原生内核。
use super::{real_tests::RecordingSidecarPort, *};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Instant,
};
use veyra_core::{
    application::{
        manual_runtime::*, state_access::StateAccessGate, state_service::SnapshotService,
    },
    domain::*,
    storage::{JsonStateStore, StateStore},
};
struct HealthPeer {
    address: SocketAddr,
    healthy: Arc<AtomicBool>,
    hits: Arc<AtomicUsize>,
    closing: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl HealthPeer {
    fn new() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let healthy = Arc::new(AtomicBool::new(true));
        let hits = Arc::new(AtomicUsize::new(0));
        let seen = hits.clone();
        let closing = Arc::new(AtomicBool::new(false));
        let ok = healthy.clone();
        let stop = closing.clone();
        let worker = thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        // macOS accept 继承 listener 非阻塞标志；健康返回器必须完整等待请求头。
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(1)))
                            .unwrap();
                        let mut reader = BufReader::new(stream);
                        let Some(request) = read_head(&mut reader) else {
                            continue;
                        };
                        if !ok.load(Ordering::SeqCst) {
                            continue;
                        }
                        // 先完整消费 CONNECT 头，再处理同一受控连接的 HEAD；不向外转发。
                        if request.starts_with("CONNECT 127.0.0.1:9 ") {
                            if reader.get_mut().write_all(b"HTTP/1.1 200 Connection Established\r\nContent-Length: 0\r\n\r\n").is_err() {continue;}
                            let Some(head) = read_head(&mut reader) else {
                                continue;
                            };
                            if !head.starts_with("HEAD /controlled-health HTTP/1.1\r\n") {
                                continue;
                            }
                            seen.fetch_add(1, Ordering::SeqCst);
                        } else {
                            continue;
                        }
                        thread::sleep(Duration::from_millis(3));
                        let _=reader.get_mut().write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            address,
            healthy,
            hits,
            closing,
            worker: Some(worker),
        }
    }
}
fn read_head(reader: &mut BufReader<std::net::TcpStream>) -> Option<String> {
    let mut header = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        if header.len() + line.len() > 8192 {
            return None;
        }
        if line == "\r\n" {
            return Some(header);
        }
        header.push_str(&line);
    }
}
impl Drop for HealthPeer {
    fn drop(&mut self) {
        self.closing.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}
fn request(
    owner: &ManualRuntime<RecordingSidecarPort>,
    group: &NodeGroup,
    member: OutboundId,
    mode: GroupSelectionMode,
) -> ManualSelectionRequest<GroupChoice> {
    let s = owner.snapshot().unwrap();
    ManualSelectionRequest {
        instance: s.runtime.instance_id.unwrap(),
        config: s.runtime.applied_version.unwrap(),
        expected: s.confirmed_selection_version.unwrap(),
        pool: group.id.clone(),
        node: GroupChoice {
            group: group.id.clone(),
            member,
            mode,
        },
    }
}
fn wait_for(
    owner: &mut ManualRuntime<RecordingSidecarPort>,
    clock: &Instant,
    condition: impl Fn(&ManualRuntimeSnapshot) -> bool,
) {
    let deadline = Instant::now() + Duration::from_secs(25);
    loop {
        owner.poll_failover(clock.elapsed().as_millis() as u64);
        let s = owner.snapshot().unwrap();
        if condition(&s) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "native timeout: {:?}",
            s.failover_status
        );
        thread::sleep(Duration::from_millis(25));
    }
}
#[test]
#[ignore = "requires explicitly verified v1.14.0 VEYRA_SING_BOX_PATH; self-owned loopback only"]
fn p403_native_failover_pin_pending_restart_and_cleanup() {
    let primary = HealthPeer::new();
    let backup = HealthPeer::new();
    let mut value = serde_json::to_value(AppState::empty()).unwrap();
    let parts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../veyra-core/tests/fixtures/compiler/p2-02b.json"
    ))
    .unwrap();
    for (k, v) in parts.as_object().unwrap() {
        value[k] = v.clone();
    }
    let mut state: AppState = serde_json::from_value(value).unwrap();
    state.active_subscription_id = Some(SubscriptionId("subscription".into()));
    state.routes.clear();
    state
        .pools
        .retain(|p| matches!(p.selection, SelectionPolicy::Manual { .. }));
    for node in &mut state.nodes {
        node.protocol = ProxyProtocol::Http;
        node.server = "127.0.0.1".into();
        node.port = if node.id.0 == "c" {
            backup.address.port()
        } else {
            primary.address.port()
        };
        node.options = ProtocolOptions::Http {
            username: None,
            password: None,
            tls: false,
        };
        node.tls = None;
        node.transport = None;
    }
    let mut group = NodeGroup::fresh().unwrap();
    group.id = PoolId("p403-native".into());
    group.name = "主备".into();
    group.rule = GroupRule::Failover;
    group.members.clear();
    group.mode = GroupMode::Static;
    group.interval_secs = 5;
    group.test_url = "http://127.0.0.1:9/controlled-health".into();
    group.failover = Some(FailoverSettings {
        timeout_ms: 1000,
        failure_threshold: 2,
        restore_primary: true,
        recovery_hold_ms: 100,
    });
    group.lanes = vec![
        FailoverLane {
            id: "primary".into(),
            name: String::new(),
            icon: String::new(),
            members: vec![
                OutboundId::Node(NodeId("a".into())),
                OutboundId::Node(NodeId("b".into())),
            ],
            manual: true,
        },
        FailoverLane {
            id: "backup".into(),
            name: String::new(),
            icon: String::new(),
            members: vec![
                OutboundId::Node(NodeId("c".into())),
                OutboundId::Node(NodeId("a".into())),
            ],
            manual: false,
        },
    ];
    state.groups = vec![group.clone()];
    state.default_target = RouteTarget::Pool(group.id.clone());
    state.config_revision = 11;
    let root =
        std::env::temp_dir().join(format!("veyra-p403-runtime-native-{:?}", state.state_epoch));
    let store = JsonStateStore::new(root.join("state.json")).unwrap();
    store.save(&state).unwrap();
    let snapshots = Arc::new(SnapshotService::new(
        store.clone(),
        StateAccessGate::default(),
    ));
    println!("P403_NATIVE_ROOT {}", root.display());
    let port = RecordingSidecarPort::new(&root);
    let mut owner = ManualRuntime::new(port.clone(), snapshots.clone(), root.clone(), true);
    let clock = Instant::now();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let first = owner.snapshot().unwrap();

    let endpoints = first.endpoints.unwrap();
    let primary_id = OutboundId::Pool(group.lanes[0].pool_id(&group.id));
    let backup_id = OutboundId::Pool(group.lanes[1].pool_id(&group.id));
    assert_eq!(first.confirmed_group_selections[&group.id], primary_id);
    wait_for(&mut owner, &clock, |s| {
        s.failover_status.get(&group.id) == Some(&FailoverStatus::Healthy)
    });
    // 同一健康 companion 两个并发批次均完成；不能因原 URLTest checking 锁得到伪失败。
    let endpoint = port
        .inner
        .lock()
        .unwrap()
        .owned
        .as_ref()
        .unwrap()
        .controller
        .clone()
        .unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let tag = format!(
        "pool-failover-health-{}",
        group.lanes[0].pool_id(&group.id).0
    );
    executor.block_on(async {
        let client = ClashApiClient::managed(&endpoint).unwrap();
        let (a, b) = tokio::join!(
            client.test_group(&tag, 1000, &group.test_url),
            client.test_group(&tag, 1000, &group.test_url)
        );
        assert!(a.unwrap().contains_key("node-a"));
        assert!(b.unwrap().contains_key("node-a"));
    });
    assert!(primary.hits.load(Ordering::SeqCst) > 0 && backup.hits.load(Ordering::SeqCst) > 0);
    primary.healthy.store(false, Ordering::SeqCst);
    wait_for(&mut owner, &clock, |s| {
        s.confirmed_group_selections.get(&group.id) == Some(&backup_id)
    });
    primary.healthy.store(true, Ordering::SeqCst);
    wait_for(&mut owner, &clock, |s| {
        s.confirmed_group_selections.get(&group.id) == Some(&primary_id)
    });
    owner
        .select_manual(request(
            &owner,
            &group,
            backup_id.clone(),
            GroupSelectionMode::ManualPin,
        ))
        .unwrap();
    let pinned = store.load().unwrap();
    assert_eq!(
        pinned.group_selections[&group.id].mode,
        GroupSelectionMode::ManualPin
    );
    // 完整健康批次不能覆盖人工固定；真实持久事实及 Controller GET 都保留备用。
    wait_for(&mut owner, &clock, |s| {
        s.failover_status.get(&group.id) == Some(&FailoverStatus::Healthy)
    });
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&group.id],
        backup_id
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
    drop(owner);
    let mut owner = ManualRuntime::new(port.clone(), snapshots, root.clone(), true);
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    assert_ne!(
        owner.snapshot().unwrap().runtime.instance_id,
        first.runtime.instance_id
    );
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&group.id],
        backup_id
    );
    assert_eq!(
        store.load().unwrap().group_selections[&group.id].mode,
        GroupSelectionMode::ManualPin
    );
    // 仅丢弃一次真实成功 GET 的返回，不改变内核；不确定状态后只读核对。
    port.rework.lock().unwrap().lose_get_result = true;
    assert_eq!(
        owner.select_manual(request(
            &owner,
            &group,
            primary_id.clone(),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerReadBack)
    );
    assert!(
        store.load().unwrap().group_selections[&group.id]
            .pending
            .is_some()
    );
    let offset = port.events().len();
    owner
        .execute(RuntimeCommand::ReconcileSelection, |_| {})
        .unwrap();
    assert!(!port.events()[offset..].iter().any(|e| e["op"] == "PUT"));
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&group.id],
        primary_id
    );
    // pending 落盘后重建 Store/Service/owner：新实例只读实际缓存，不重发旧 PUT。
    port.rework.lock().unwrap().lose_get_result = true;
    assert_eq!(
        owner.select_manual(request(
            &owner,
            &group,
            backup_id.clone(),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerReadBack)
    );
    assert!(
        store.load().unwrap().group_selections[&group.id]
            .pending
            .is_some()
    );
    let pending_instance = owner.snapshot().unwrap().runtime.instance_id;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    drop(owner);
    let restored_store = JsonStateStore::new(root.join("state.json")).unwrap();
    let restored_snapshots = Arc::new(SnapshotService::new(
        restored_store.clone(),
        StateAccessGate::default(),
    ));
    let offset = port.events().len();
    let mut owner = ManualRuntime::new(port.clone(), restored_snapshots, root.clone(), true);
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    assert_ne!(
        owner.snapshot().unwrap().runtime.instance_id,
        pending_instance
    );
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&group.id],
        backup_id
    );
    let restored = restored_store.load().unwrap();
    assert!(restored.group_selections[&group.id].pending.is_none());
    assert_eq!(
        restored.group_selections[&group.id].mode,
        GroupSelectionMode::ManualPin
    );
    assert!(!port.events()[offset..].iter().any(|e| e["op"] == "PUT"));
    owner
        .select_manual(request(
            &owner,
            &group,
            primary_id.clone(),
            GroupSelectionMode::Auto,
        ))
        .unwrap();
    assert_eq!(
        store.load().unwrap().group_selections[&group.id].mode,
        GroupSelectionMode::Auto
    );
    primary.healthy.store(false, Ordering::SeqCst);
    backup.healthy.store(false, Ordering::SeqCst);
    wait_for(&mut owner, &clock, |s| {
        s.failover_status.get(&group.id) == Some(&FailoverStatus::AllFailed)
    });
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&group.id],
        primary_id
    );
    assert_eq!(store.load().unwrap().config_revision, 11);
    let final_endpoints = owner.snapshot().unwrap().endpoints.unwrap();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    assert!(std::net::TcpStream::connect(final_endpoints.mixed).is_err());
    assert!(std::net::TcpStream::connect(final_endpoints.controller).is_err());
    assert!(port.inner.lock().unwrap().owned.is_none());
    assert!(port.inner.lock().unwrap().pending.is_none());
    let evidence = serde_json::json!({"status":"PASS","kernel":"1.14.0","kernel_sha256":KERNEL_SHA,"root":root,"primary_peer":primary.address,"backup_peer":backup.address,"config_revision":store.load().unwrap().config_revision,"selection_revision":store.load().unwrap().selection_revision,"events":port.events(),"health_same_tag_concurrent_batches":true,"http_url_actual_target_confirmed":true,"primary_health_hits":primary.hits.load(Ordering::SeqCst),"backup_health_hits":backup.hits.load(Ordering::SeqCst),"auto_backup_urltest":true,"all_failed_preserved_confirmed":true,"pending_reconcile_puts":0,"pending_restart_store_service_rebuilt":true,"pending_restart_puts":0,"cleanup":true});
    println!("P403_NATIVE {}", evidence);
    fs::write(
        root.join("evidence.json"),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
}
