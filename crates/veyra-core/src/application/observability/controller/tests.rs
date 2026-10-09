use super::*;
use crate::singbox::{GeneratedConfig, secret::generate_api_secret};
use futures_util::SinkExt;
use tokio::{net::TcpListener, sync::mpsc, time::timeout};
use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::handshake::server::{Request, Response},
};

fn state(id: Identity) -> State {
    let (events, _) = broadcast::channel(EVENT_CAPACITY);
    let mut snapshot = Snapshot::empty(Some(id.clone()), 0);
    snapshot.stream_generation = [1; 4];
    State {
        snapshot,
        events,
        writer: None,
        traffic_status: TrafficStorageStatus::default(),
    }
}
// 保护区间增量不差分/不重复累计，counter 回退不产生负数，Unknown 不伪零，以及日志脱敏。
#[test]
fn intervals_totals_reset_unknown_and_redaction() {
    let id = Identity {
        instance_id: InstanceId("one".into()),
        generation: 1,
    };
    let mut state = state(id.clone());
    assert_eq!(state.snapshot.memory_bytes, None);
    apply(
        &mut state,
        id.clone(),
        Stream::Memory,
        1,
        1,
        r#"{"inuse":0}"#,
        1000,
    )
    .unwrap();
    assert_eq!(state.snapshot.memory_bytes, None);
    state.snapshot.stream_generation[1] = 2;
    state.snapshot.sequence[1] = 0;
    apply(
        &mut state,
        id.clone(),
        Stream::Memory,
        2,
        1,
        r#"{"inuse":0}"#,
        1000,
    )
    .unwrap();
    assert_eq!(state.snapshot.memory_bytes, None);
    state.snapshot.stream_generation[1] = 1;
    state.snapshot.sequence[1] = 0;

    for seq in 1..=2 {
        apply(
            &mut state,
            id.clone(),
            Stream::Traffic,
            1,
            seq,
            r#"{"up":100,"down":200}"#,
            1000,
        )
        .unwrap();
    }
    let t = state.snapshot.traffic.as_ref().unwrap();
    assert_eq!(t.upload_bytes_per_second, 100);
    assert_eq!(t.session_upload_bytes, 200);
    assert!(
        apply(
            &mut state,
            id.clone(),
            Stream::Traffic,
            1,
            2,
            r#"{"up":100,"down":200}"#,
            1000
        )
        .is_err()
    );
    apply(
        &mut state,
        id.clone(),
        Stream::Traffic,
        1,
        3,
        r#"{"up":100,"down":200}"#,
        500,
    )
    .unwrap();
    assert_eq!(
        state
            .snapshot
            .traffic
            .as_ref()
            .unwrap()
            .upload_bytes_per_second,
        200
    );
    for (seq, total) in [(1, 2000), (2, 100)] {
        apply(
            &mut state,
            id.clone(),
            Stream::Connections,
            1,
            seq,
            &format!("{{\"uploadTotal\":{total},\"downloadTotal\":{total},\"connections\":null}}"),
            1000,
        )
        .unwrap();
    }
    assert_eq!(state.snapshot.counter_resets, 1);
    assert_eq!(
        state
            .snapshot
            .traffic
            .as_ref()
            .unwrap()
            .session_upload_bytes,
        300
    );
    for seq in 1..=1100 {
        apply(
            &mut state,
            id.clone(),
            Stream::Logs,
            1,
            seq,
            r#"{"type":"error","payload":"token=private password=private /Users/private"}"#,
            1000,
        )
        .unwrap();
    }
    assert_eq!(state.snapshot.logs.len(), LOG_CAPACITY);
    assert!(!format!("{:?}", state.snapshot).contains("private"));
    apply(
        &mut state,
        id.clone(),
        Stream::Memory,
        1,
        1,
        r#"{"inuse":128}"#,
        1000,
    )
    .unwrap();
    state.disconnected(id.clone(), Stream::Memory);
    assert_eq!(state.snapshot.memory_bytes, None);
    state.snapshot.stream_generation[0] = 2;
    state.snapshot.sequence[0] = 0;
    assert!(
        apply(
            &mut state,
            id.clone(),
            Stream::Traffic,
            1,
            4,
            r#"{"up":100,"down":200}"#,
            1000
        )
        .is_err()
    );
    apply(
        &mut state,
        id.clone(),
        Stream::Traffic,
        2,
        1,
        r#"{"up":10,"down":20}"#,
        1000,
    )
    .unwrap();
    assert_eq!(
        state
            .snapshot
            .traffic
            .as_ref()
            .unwrap()
            .session_upload_bytes,
        310
    );
    state.snapshot = Snapshot::empty(
        Some(Identity {
            instance_id: InstanceId("two".into()),
            generation: 2,
        }),
        state.snapshot.revision,
    );
    assert!(
        apply(
            &mut state,
            id.clone(),
            Stream::Memory,
            1,
            2,
            r#"{"inuse":42}"#,
            1000
        )
        .is_err()
    );
}

struct Fixture {
    endpoint: Arc<ManagedControllerEndpoint>,
    accepted: mpsc::Receiver<(String, mpsc::Sender<Option<String>>)>,
    task: JoinHandle<()>,
}
#[expect(
    clippy::result_large_err,
    reason = "upstream handshake callback fixed error type"
)]
async fn fixture() -> Fixture {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let secret = generate_api_secret().unwrap();
    let mut json: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/compiler/p2-02b-base.expected.json"
    ))
    .unwrap();
    json["experimental"]["clash_api"]["secret"] = secret.as_str().into();
    let config = GeneratedConfig::from_bytes(serde_json::to_vec(&json).unwrap());
    let endpoint = Arc::new(ManagedControllerEndpoint::from_owned_child(address, &config).unwrap());
    let expected = format!("Bearer {}", secret.as_str());
    let (tx, accepted) = mpsc::channel(16);
    let task = tokio::spawn(async move {
        let mut sockets = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                next=listener.accept()=> {
                    let (stream,_)=next.unwrap();let expected=expected.clone();let tx=tx.clone();
                    sockets.spawn(async move {
                        let mut path=String::new();
                        let mut socket=accept_hdr_async(stream,|request:&Request,response:Response| {assert_eq!(request.headers()["authorization"],expected);path=request.uri().path().into();Ok(response)}).await.unwrap();
                        let (send,mut receive)=mpsc::channel::<Option<String>>(8);tx.send((path,send)).await.unwrap();
                        loop {tokio::select! {
                            frame=receive.recv()=>match frame {Some(Some(text))=>{if socket.send(Message::Text(text.into())).await.is_err(){break;}},_=>{let _=socket.close(None).await;break;}},
                            next=socket.next()=>match next {None|Some(Err(_))|Some(Ok(Message::Close(_)))=>break,_=>{}}
                        }}
                    });
                },
                _=sockets.join_next(),if !sockets.is_empty()=>{}
            }
        }
    });
    Fixture {
        endpoint,
        accepted,
        task,
    }
}
async fn wait_for(service: &ObservationService, predicate: impl Fn(&Snapshot) -> bool) {
    timeout(Duration::from_secs(4), async {
        loop {
            if predicate(&service.snapshot()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
}
// 生产 Service/Client 真正建四条 authenticated WS；多订阅复用，落后报告 gap，换代/断流/恢复清理。
#[tokio::test]
async fn production_four_ws_reuse_gap_replacement_reconnect_and_clients() {
    let mut f = fixture().await;
    let mut service = ObservationService::new();
    let mut slow = service.subscribe();
    let _other = service.subscribe();
    let old_id = service.bind(InstanceId("seven".into()), f.endpoint.clone());
    let mut senders = std::collections::HashMap::new();
    for _ in 0..4 {
        let (path, sender) = timeout(Duration::from_secs(3), f.accepted.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(senders.insert(path, sender).is_none());
    }
    senders["/traffic"]
        .send(Some(r#"{"up":17,"down":23}"#.into()))
        .await
        .unwrap();
    senders["/memory"]
        .send(Some(r#"{"inuse":123}"#.into()))
        .await
        .unwrap();
    senders["/connections"].send(Some(r#"{"uploadTotal":31,"downloadTotal":47,"connections":[{"metadata":{"sourceIP":"127.0.0.9","inboundName":"owned-mixed","processPath":"secret"}},{"metadata":{}}]}"#.into())).await.unwrap();
    senders["/logs"]
        .send(Some(r#"{"type":"info","payload":"private token"}"#.into()))
        .await
        .unwrap();
    wait_for(&service, |s| s.available == [true; 4]).await;
    assert_eq!(
        service.observed().unwrap(),
        vec![ObservedClient {
            source_ip: Some("127.0.0.9".parse().unwrap()),
            inbound_name: Some("owned-mixed".into())
        }]
    );
    assert_eq!(service.snapshot().traffic.unwrap().session_upload_bytes, 17);
    assert_eq!(service.snapshot().lifetime_totals, Some((31, 47)));
    senders["/connections"]
        .send(Some(
            r#"{"uploadTotal":31,"downloadTotal":47,"connections":null}"#.into(),
        ))
        .await
        .unwrap();
    wait_for(&service, |s| s.connection_count == Some(0)).await;
    assert_eq!(service.observed(), Some(vec![]));
    let large_frame = serde_json::json!({"uploadTotal":31,"downloadTotal":47,"connections":[{"metadata":{"sourceIP":"127.0.0.10","processPath":"x".repeat(20000)}}]}).to_string();
    assert!(large_frame.len() > 16 * 1024);
    senders["/connections"]
        .send(Some(large_frame))
        .await
        .unwrap();
    wait_for(&service, |s| s.connection_count == Some(1)).await;
    assert_eq!(
        service.observed().unwrap()[0].source_ip,
        Some("127.0.0.10".parse().unwrap())
    );

    assert!(f.accepted.try_recv().is_err());
    for _ in 0..100 {
        senders["/logs"]
            .send(Some(r#"{"type":"warn","payload":"private"}"#.into()))
            .await
            .unwrap();
    }
    wait_for(&service, |s| s.logs.len() == 101).await;
    assert!(matches!(slow.recv().await,Delivery::Gap{dropped} if dropped>0));
    senders["/memory"].send(None).await.unwrap();
    wait_for(&service, |s| s.gaps[1] > 0).await;
    assert_eq!(service.snapshot().memory_bytes, None);
    let (path, sender) = timeout(Duration::from_secs(3), f.accepted.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(path, "/memory");
    sender.send(Some(r#"{"inuse":0}"#.into())).await.unwrap();
    wait_for(&service, |s| {
        s.sequence[1] == 1 && s.stream_generation[1] == 2
    })
    .await;
    assert_eq!(service.snapshot().memory_bytes, None);

    sender.send(Some(r#"{"inuse":456}"#.into())).await.unwrap();
    wait_for(&service, |s| s.memory_bytes == Some(456)).await;
    let mut replacement = fixture().await;
    let new_id = service.bind(InstanceId("eight".into()), replacement.endpoint.clone());
    assert_ne!(old_id, new_id);
    assert_eq!(service.observed(), None);
    // 直接模拟旧任务已读取后迟到发布，验证 instance fence；真实旧 socket 同时被 abort。
    assert!(
        apply(
            &mut service.state.lock().unwrap(),
            old_id,
            Stream::Memory,
            1,
            10,
            r#"{"inuse":999}"#,
            1000
        )
        .is_err()
    );
    let mut replacement_senders = Vec::new();
    for _ in 0..4 {
        replacement_senders.push(
            timeout(Duration::from_secs(3), replacement.accepted.recv())
                .await
                .unwrap()
                .unwrap(),
        );
    }
    replacement.endpoint.invalidate();
    wait_for(&service, |s| s.gaps.iter().all(|g| *g > 0)).await;
    service.stop();
    assert_eq!(service.snapshot().identity, None);
    assert_eq!(service.observed(), None);
    wait_for(&service, |_| {
        senders.values().all(|s| s.is_closed())
            && replacement_senders.iter().all(|(_, s)| s.is_closed())
    })
    .await;
    f.task.abort();
    replacement.task.abort();
    let _ = f.task.await;
    let _ = replacement.task.await;
}

// P3-01-HOST-001：只读 snapshot/observed 消费者也必须持续获得观测；没有事件订阅时断流仍重连。
#[tokio::test]
async fn production_reconnects_without_event_subscribers_and_stop_releases_sockets() {
    let mut f = fixture().await;
    let mut service = ObservationService::new();
    let identity = service.bind(InstanceId("snapshot-only".into()), f.endpoint.clone());
    let mut senders = std::collections::HashMap::new();
    for _ in 0..4 {
        let (path, sender) = timeout(Duration::from_secs(3), f.accepted.recv())
            .await
            .expect("four authenticated WS accepts are bounded")
            .expect("fixture remains available");
        assert!(senders.insert(path, sender).is_none());
    }
    assert_eq!(senders.len(), 4);
    senders["/memory"]
        .send(Some(r#"{"inuse":64}"#.into()))
        .await
        .unwrap();
    wait_for(&service, |s| s.memory_bytes == Some(64)).await;
    senders["/memory"].send(None).await.unwrap();
    wait_for(&service, |s| s.gaps[Stream::Memory.index()] == 1).await;
    assert_eq!(service.snapshot().memory_bytes, None);

    // 此测试全程不调用 subscribe；新握手仍由 fixture 检查同一受管 Bearer 身份。
    let (path, restored) = timeout(Duration::from_secs(3), f.accepted.recv())
        .await
        .expect("snapshot-only consumer must reconnect without a broadcast receiver")
        .expect("fixture accepts reconnected WS");
    assert_eq!(path, "/memory");
    restored
        .send(Some(r#"{"inuse":128}"#.into()))
        .await
        .unwrap();
    wait_for(&service, |s| s.memory_bytes == Some(128)).await;
    let snapshot = service.snapshot();
    assert_eq!(snapshot.identity, Some(identity));
    assert!(snapshot.available[Stream::Memory.index()]);
    assert_eq!(snapshot.stream_generation[Stream::Memory.index()], 2);
    assert_eq!(snapshot.sequence[Stream::Memory.index()], 1);
    assert!(f.accepted.try_recv().is_err(), "other streams stay shared");

    service.stop();
    assert_eq!(service.snapshot().identity, None);
    assert_eq!(service.observed(), None);
    wait_for(&service, |_| {
        senders.values().all(|sender| sender.is_closed()) && restored.is_closed()
    })
    .await;
    assert!(
        timeout(Duration::from_millis(300), f.accepted.recv())
            .await
            .is_err(),
        "stop does not reconnect"
    );
    f.task.abort();
    let _ = f.task.await;
    println!(
        "P3-01-HOST-001: four authenticated WS; no subscribe; memory reconnect generation 2/frame 128; stop released sockets and did not reconnect"
    );
}

// 保护真实记录从同一帧进入 SQLite：重复/重连不重加、未知不猜测、停止前排空并回读。
#[tokio::test]
async fn connection_records_write_sqlite_and_release_on_stop() {
    let root = std::env::temp_dir().join(format!(
        "veyra-stat-seam-{:?}",
        crate::domain::StateEpoch::fresh().unwrap()
    ));
    let mut f = fixture().await;
    let mut service = ObservationService::new();
    service.enable_traffic_storage(root.clone(), chrono_tz::UTC);
    assert!(!root.exists());
    let identity = service.bind(InstanceId("statistics-owned".into()), f.endpoint.clone());
    let mut senders = std::collections::HashMap::new();
    for _ in 0..4 {
        let (path, sender) = timeout(Duration::from_secs(3), f.accepted.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(senders.insert(path, sender).is_none());
    }
    let frame = |up, down| {
        serde_json::json!({"uploadTotal":up,"downloadTotal":down,"connections":[
        {"id":"stable-id","upload":up,"download":down,"chains":["actually-observed","group"],"metadata":{"sourceIP":"127.0.0.7","host":"local.test","processPath":"never-persist-process"}},
        {"metadata":{}}
    ]}).to_string()
    };
    senders["/connections"]
        .send(Some(frame(10, 20)))
        .await
        .unwrap();
    wait_for(&service, |s| s.sequence[2] == 1).await;
    let batch = service.snapshot().connection_records.unwrap();
    assert_eq!(batch.identity, identity.clone());
    assert!(batch.sampled_at_ms > 0);
    assert_eq!(
        batch.records[0].dimensions.node.as_deref(),
        Some("actually-observed")
    );
    assert_eq!(batch.records[0].dimensions.direct, None);
    assert_eq!(batch.records[1].id, None);
    tokio::time::sleep(Duration::from_millis(10)).await;
    senders["/connections"]
        .send(Some(frame(30, 60)))
        .await
        .unwrap();
    wait_for(&service, |s| s.sequence[2] == 2).await;
    // 同一采集代次/序号重放在 Observation fence 拒绝，不能再次提交。
    assert!(
        apply(
            &mut service.state.lock().unwrap(),
            identity.clone(),
            Stream::Connections,
            1,
            2,
            &frame(30, 60),
            1000
        )
        .is_err()
    );
    // 真实断开后仍只有该单路重连；累计不变时 SQLite 增量为零。
    senders["/connections"].send(None).await.unwrap();
    wait_for(&service, |s| s.gaps[2] > 0).await;
    assert!(service.snapshot().connection_records.is_none());
    let (path, reconnected) = timeout(Duration::from_secs(3), f.accepted.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(path, "/connections");
    reconnected.send(Some(frame(30, 60))).await.unwrap();
    wait_for(&service, |s| {
        s.sequence[2] == 1 && s.stream_generation[2] == 2
    })
    .await;
    tokio::time::sleep(Duration::from_millis(10)).await;
    reconnected.send(Some(frame(50, 100))).await.unwrap();
    wait_for(&service, |s| s.sequence[2] == 2).await;
    assert!(f.accepted.try_recv().is_err());
    service.stop();
    assert!(!service.traffic_status().active);
    assert!(service.traffic_status().failure.is_none());
    assert_eq!(service.traffic_status().incomplete_records, 4);
    let reopened = TrafficWriter::open(&root, chrono_tz::UTC).unwrap();
    let db = rusqlite::Connection::open(root.join("traffic.sqlite3")).unwrap();
    let total: (i64, i64) = db
        .query_row(
            "SELECT SUM(up),SUM(down) FROM detail WHERE kind='partial'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(total, (40, 80));
    let unknown: i64 = db
        .query_row(
            "SELECT COUNT(*) FROM detail WHERE direct IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(unknown, 0);
    let sources: i64 = db
        .query_row("SELECT COUNT(DISTINCT instance) FROM detail", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(sources, 1);
    drop(db);
    reopened.close().unwrap();
    service.stop();
    f.task.abort();
    let _ = f.task.await;
    std::fs::remove_dir_all(root).unwrap();
}

// 保护落盘失败不伪称统计成功，也不阻止必要的观测停止与数据库资源释放。
#[tokio::test]
async fn statistics_open_failure_is_explicit_and_does_not_prevent_stop() {
    let root = std::env::temp_dir().join(format!(
        "veyra-stat-failure-{:?}",
        crate::domain::StateEpoch::fresh().unwrap()
    ));
    std::fs::write(&root, b"file blocks statistics directory").unwrap();
    let f = fixture().await;
    let mut service = ObservationService::new();
    service.enable_traffic_storage(root.clone(), chrono_tz::UTC);
    service.bind(
        InstanceId("ready-with-storage-failure".into()),
        f.endpoint.clone(),
    );
    assert_eq!(
        service.traffic_status().failure,
        Some(TrafficStorageFailure::Open)
    );
    assert!(!service.traffic_status().active);
    service.stop();
    assert!(service.snapshot().identity.is_none());
    assert_eq!(
        service.traffic_status().failure,
        Some(TrafficStorageFailure::Open)
    );
    f.task.abort();
    let _ = f.task.await;
    std::fs::remove_file(root).unwrap();
}

// 保护消费 seam 的最终提交错误：Stop 必须报告损失并 join 释放库，而不是伪称落盘成功。
#[tokio::test]
async fn statistics_commit_failure_retains_status_and_releases_database() {
    let root = std::env::temp_dir().join(format!(
        "veyra-stat-commit-failure-{:?}",
        crate::domain::StateEpoch::fresh().unwrap()
    ));
    let mut f = fixture().await;
    let mut service = ObservationService::new();
    service.enable_traffic_storage(root.clone(), chrono_tz::UTC);
    service.bind(
        InstanceId("failed-statistics-owned".into()),
        f.endpoint.clone(),
    );
    let db = rusqlite::Connection::open(root.join("traffic.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER test_failure BEFORE INSERT ON detail BEGIN SELECT RAISE(ABORT,'controlled write failure'); END;").unwrap();
    let mut senders = std::collections::HashMap::new();
    for _ in 0..4 {
        let (path, sender) = timeout(Duration::from_secs(3), f.accepted.recv())
            .await
            .unwrap()
            .unwrap();
        senders.insert(path, sender);
    }
    senders["/traffic"]
        .send(Some(r#"{"up":11,"down":22}"#.into()))
        .await
        .unwrap();
    wait_for(&service, |s| s.sequence[0] == 1).await;
    service.stop();
    let status = service.traffic_status();
    assert!(!status.active);
    assert_eq!(status.failure, Some(TrafficStorageFailure::Commit));
    assert!(status.writer.failed);
    assert_eq!(status.writer.failed_batch, 1);
    assert_eq!(status.writer.uncommitted_lost, 1);
    let count: i64 = db
        .query_row("SELECT COUNT(*) FROM detail", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0, "failed transaction rolled back");
    db.execute_batch("DROP TRIGGER test_failure").unwrap();
    drop(db);
    TrafficWriter::open(&root, chrono_tz::UTC)
        .unwrap()
        .close()
        .unwrap();
    f.task.abort();
    let _ = f.task.await;
    std::fs::remove_dir_all(root).unwrap();
}
