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
    State { snapshot, events }
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
