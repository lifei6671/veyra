use super::*;
use crate::{
    application::{
        outbound_query::OutboundQueryService,
        runtime_recovery::{RecoveryAvailability, RecoveryError},
        runtime_snapshot::{InstanceId, RuntimeSnapshot},
        state_access::StateAccessGate,
    },
    domain::{SelectionPolicy, StateEpoch},
    singbox::clash_api::ManagedControllerEndpoint,
    storage::{JsonStateStore, StateStore},
};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture {
    root: PathBuf,
    state: AppState,
    store: JsonStateStore,
    snapshots: SnapshotService,
    source: ProxySource,
    runtime: ManualRuntimeSnapshot,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "veyra-p208-service-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir(&root).unwrap();
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/compiler/p2-02b.json"))
                .unwrap();
        for (k, v) in fixture.as_object().unwrap() {
            value[k] = v.clone();
        }
        let mut state: AppState = serde_json::from_value(value).unwrap();
        state.app_config.behavior.latency.test_url.0 =
            "https://target.example.invalid/custom?q=1".into();
        state.app_config.behavior.latency.timeout_ms = 7000;
        state.validate().unwrap();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = SnapshotService::new(store.clone(), StateAccessGate::default());
        let runtime = ManualRuntimeSnapshot {
            runtime: RuntimeSnapshot {
                status: RuntimeStatus::Ready,
                instance_id: Some(InstanceId("owned-p208".into())),
                saved_version: state.config_version(),
                applied_version: Some(state.config_version()),
                last_successful_version: None,
                last_successful_selection_version: None,
            },
            endpoints: None,
            uptime_seconds: None,
            kernel_available: true,
            recovery: RecoveryAvailability::Unavailable(RecoveryError::Missing),
            saved_selection_version: state.selection_version(),
            confirmed_selection_version: Some(state.selection_version()),
            pending_selection: None,
            pending_selections: vec![],
            selection_fallbacks: vec![],
            confirmed_manual_selections: BTreeMap::from([(
                PoolId("manual".into()),
                NodeId("a".into()),
            )]),
            group_selections: Default::default(),
            confirmed_group_selections: Default::default(),
            failover_status: Default::default(),
        };
        Self {
            root,
            state,
            store,
            snapshots,
            source: Default::default(),
            runtime,
        }
    }
    fn service(&self) -> ProxyService {
        ProxyService::open(
            self.snapshots.clone(),
            self.source.clone(),
            &self.root.join("latency.sqlite3"),
        )
        .unwrap()
    }
    fn publish(&self, address: std::net::SocketAddr) {
        // 用正式 Compiler 生成 owner 索引，fixture 不从 Saved 目录手工伪造应用边。
        use crate::singbox::{
            LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
            SingBoxCompiler,
        };
        let intent = crate::domain::RuntimeIntent::from_state(&self.state).unwrap();
        let resources = ProductRuntimeResources {
            mixed: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
            controller: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
            cache: ManagedCacheFile::new(
                &self.root,
                self.root.join("cache.db"),
                "p208-fixture".into(),
                false,
                false,
            )
            .unwrap(),
        };
        let target = OutboundId::from_route_target(&self.state.default_target).unwrap();
        let plan = SingBoxCompiler
            .compile_product(ProductCompileRequest {
                state: &self.state,
                runtime_intent: &intent,
                default_outbound: &target,
                resources: &resources,
            })
            .unwrap();
        let index = plan.artifact_index().unwrap().clone();
        let config = plan
            .finalize(&crate::singbox::secret::test_api_secret())
            .unwrap();
        let mut proxy = RunningProxy::ready("owned-p208".into(), address).unwrap();
        proxy.controller = Some(Arc::new(
            ManagedControllerEndpoint::from_owned_child(address, &config).unwrap(),
        ));
        proxy.nodes = index.nodes.clone();
        proxy.applied_index = Some(Arc::new(index));
        self.source.publish(proxy);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.source.revoke();
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn executor() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

struct Probe {
    requests: Mutex<Vec<String>>,
    replies: Mutex<BTreeMap<String, (u16, String)>>,
    started: Semaphore,
    release: Semaphore,
    active: AtomicUsize,
    maximum: AtomicUsize,
}
// 真loopback HTTP由正式Service/NetworkService发起；此server模拟Controller协议，不冒充内核。
async fn controller(f: &Fixture, release: usize) -> (Arc<Probe>, tokio::task::JoinHandle<()>) {
    let probe = Arc::new(Probe {
        release: Semaphore::new(release),
        started: Semaphore::new(0),
        requests: Default::default(),
        replies: Default::default(),
        active: AtomicUsize::new(0),
        maximum: AtomicUsize::new(0),
    });
    let app = axum::Router::new()
        .route(
            "/proxies/{tag}/delay",
            axum::routing::get(
                |axum::extract::State(p): axum::extract::State<Arc<Probe>>,
                 req: axum::extract::Request| async move {
                    assert_eq!(
                        req.headers()["authorization"],
                        format!("Bearer {}", "a".repeat(64))
                    );
                    p.requests.lock().unwrap().push(req.uri().to_string());
                    let active = p.active.fetch_add(1, Ordering::SeqCst) + 1;
                    p.maximum.fetch_max(active, Ordering::SeqCst);
                    p.started.add_permits(1);
                    p.release.acquire().await.unwrap().forget();
                    p.active.fetch_sub(1, Ordering::SeqCst);
                    let (code, body) = p
                        .replies
                        .lock()
                        .unwrap()
                        .get(req.uri().path())
                        .cloned()
                        .unwrap_or((200, "{\"delay\":25}".into()));
                    (axum::http::StatusCode::from_u16(code).unwrap(), body)
                },
            ),
        )
        .route(
            "/proxies/{tag}",
            axum::routing::get(
                |axum::extract::State(p): axum::extract::State<Arc<Probe>>,
                 req: axum::extract::Request| async move {
                    assert_eq!(
                        req.headers()["authorization"],
                        format!("Bearer {}", "a".repeat(64))
                    );
                    p.requests.lock().unwrap().push(req.uri().to_string());
                    p.started.add_permits(1);
                    p.release.acquire().await.unwrap().forget();
                    let (code, body) = p
                        .replies
                        .lock()
                        .unwrap()
                        .get(req.uri().path())
                        .cloned()
                        .unwrap_or((200, "{\"now\":\"node-a\"}".into()));
                    (axum::http::StatusCode::from_u16(code).unwrap(), body)
                },
            ),
        )
        .with_state(probe.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    f.publish(listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (probe, task)
}

// 保护同名节点的稳定身份、未应用与未知选择，查询不改业务磁盘/health配置。
#[test]
fn snapshot_and_selection_adaptor_use_only_ids_and_owner_facts() {
    let f = Fixture::new();
    let service = f.service();
    let bytes = std::fs::read(f.root.join("state.json")).unwrap();
    let snapshot = service.snapshot(&f.runtime).unwrap();
    assert_eq!(
        snapshot.entries.len(),
        OutboundQueryService::new(f.snapshots.clone())
            .snapshot()
            .unwrap()
            .list()
            .len()
    );
    assert_eq!(
        snapshot
            .entries
            .iter()
            .filter(|e| e.outbound.kind == OutboundKind::Node)
            .count(),
        3
    );
    assert!(snapshot.entries.iter().all(|e| e.current.is_none()));
    assert!(
        snapshot
            .entries
            .iter()
            .filter(|e| e.outbound.kind == OutboundKind::Node)
            .all(|e| e.applied == Some(false))
    );
    assert!(matches!(
        service.selection_request(
            &f.runtime,
            PoolId("manual".into()),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin
        ),
        Err(ProxyError::NotReady)
    ));
    f.publish("127.0.0.1:9".parse().unwrap());
    let request = service
        .selection_request(
            &f.runtime,
            PoolId("manual".into()),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin,
        )
        .unwrap();
    assert_eq!(request.instance, InstanceId("owned-p208".into()));
    assert_eq!(request.expected, f.state.selection_version());
    assert_eq!(request.config, f.state.config_version());
    assert_eq!(request.node, SelectionChoice::Node(NodeId("b".into())));
    assert!(matches!(
        service.selection_request(
            &f.runtime,
            PoolId("auto".into()),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin
        ),
        Err(ProxyError::InvalidSelection)
    ));
    assert_eq!(
        service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("auto".into())))
            .unwrap()
            .nodes()
            .len(),
        3
    );
    assert!(matches!(
        service.begin_test(&f.runtime, OutboundId::Direct),
        Err(ProxyError::UnsupportedTarget)
    ));
    assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
}

// 保护正式服务实际消费UI目标/超时/阈值，结果真实落盘并重建；不覆盖组选择/health URL。
#[test]
fn group_loopback_preferences_zero_failure_and_service_reopen() {
    executor().block_on(async {
        let f = Fixture::new();
        let (probe, server) = controller(&f, 20).await;
        probe.replies.lock().unwrap().extend([
            (
                "/proxies/node-a/delay".into(),
                (200, "{\"delay\":25}".into()),
            ),
            (
                "/proxies/node-b/delay".into(),
                (200, "{\"delay\":0}".into()),
            ),
            ("/proxies/node-c/delay".into(), (503, "{}".into())),
        ]);
        let service = f.service();
        let bytes = std::fs::read(f.root.join("state.json")).unwrap();
        let request = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        assert_eq!(request.nodes().len(), 3);
        let results = service.run_test(request).await;
        assert_eq!(results.len(), 3);
        assert!(
            results
                .iter()
                .all(|r| matches!(r.disposition, TestDisposition::Recorded))
        );
        for req in probe.requests.lock().unwrap().iter() {
            assert!(req.contains(
                "url=https%3A%2F%2Ftarget.example.invalid%2Fcustom%3Fq%3D1&timeout=7000"
            ));
        }
        let snapshot = service.snapshot(&f.runtime).unwrap();
        let get = |id: &str| {
            snapshot
                .entries
                .iter()
                .find(|e| e.outbound.id == OutboundId::Node(NodeId(id.into())))
                .unwrap()
        };
        assert_eq!(get("a").history[0].outcome, LatencyOutcome::Success(25));
        assert_eq!(get("a").band, Some(LatencyBand::Low));
        assert_eq!(get("b").history[0].outcome, LatencyOutcome::Zero);
        assert_eq!(get("c").history[0].outcome, LatencyOutcome::Failed);
        assert!(get("a").history[0].tested_at_ms > 0);
        assert_eq!(
            snapshot
                .entries
                .iter()
                .find(|e| e.outbound.id == OutboundId::Pool(PoolId("manual".into())))
                .unwrap()
                .current,
            Some(OutboundId::Node(NodeId("a".into())))
        );
        drop(service);
        let rebuilt = f.service();
        assert_eq!(
            rebuilt
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .find(|e| e.outbound.id == OutboundId::Node(NodeId("a".into())))
                .unwrap()
                .history,
            get("a").history
        );
        assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
        server.abort();
    });
}

// 保护新单节点请求不被旧组请求覆盖；选择版本独立推进，测速不会写选择。
#[test]
fn newer_node_generation_wins_and_selection_change_is_preserved() {
    executor().block_on(async {
        let mut f = Fixture::new();
        let (probe, server) = controller(&f, 0).await;
        let service = f.service();
        let old = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        let newer = service
            .begin_test(&f.runtime, OutboundId::Node(NodeId("a".into())))
            .unwrap();
        f.state.pools[0].selection = SelectionPolicy::Manual {
            selected_node_id: Some(NodeId("b".into())),
            pending_node_id: None,
        };
        let saved = f.store.commit(&f.state).unwrap();
        let bytes = std::fs::read(f.root.join("state.json")).unwrap();
        probe.release.add_permits(10);
        let old = service.run_test(old).await;
        assert!(matches!(
            old.iter().find(|r| r.node.0 == "a").unwrap().disposition,
            TestDisposition::Superseded
        ));
        assert!(matches!(
            service.run_test(newer).await[0].disposition,
            TestDisposition::Recorded
        ));
        assert_eq!(
            f.store.load().unwrap().selection_version(),
            saved.selection_version()
        );
        assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
        server.abort();
    });
}

// 保护在途取消/超时明确分开，取消不写历史，实际超时写历史；未知节点不伪成功。
#[test]
fn timeout_cancel_and_unapplied_node_are_distinct() {
    executor().block_on(async {
        let mut f = Fixture::new();
        f.state.app_config.behavior.latency.timeout_ms = 30;
        f.state = f.store.commit(&f.state).unwrap();
        f.runtime.runtime.saved_version = f.state.config_version();
        f.runtime.runtime.applied_version = Some(f.state.config_version());
        let (probe, server) = controller(&f, 0).await;
        let service = f.service();
        let request = service
            .begin_test(&f.runtime, OutboundId::Node(NodeId("a".into())))
            .unwrap();
        let results = service.run_test(request).await;
        assert!(matches!(results[0].disposition, TestDisposition::Recorded));
        assert_eq!(
            LatencyOutcome::from_result(&results[0].result),
            LatencyOutcome::Timeout
        );
        let request = service
            .begin_test(&f.runtime, OutboundId::Node(NodeId("b".into())))
            .unwrap();
        let cancel = request.cancellation();
        let (results, ()) = tokio::join!(service.run_test(request), async {
            probe.started.acquire_many(2).await.unwrap().forget();
            cancel.cancel();
        });
        assert!(matches!(results[0].disposition, TestDisposition::Cancelled));
        let old = f.source.current().unwrap();
        let mut proxy = RunningProxy::ready("owned-p208".into(), old.address()).unwrap();
        proxy.controller = old.controller.clone();
        proxy.nodes = old.nodes.clone();
        proxy.applied_index = old.applied_index.clone();
        proxy.nodes.remove(&NodeId("c".into()));
        f.source.publish(proxy);
        assert!(matches!(
            service.begin_test(&f.runtime, OutboundId::Node(NodeId("c".into()))),
            Err(ProxyError::UnsupportedTarget)
        ));
        // 既有 Network 未应用节点仍为 Unknown；Proxy 批次在发送网络请求前拒绝。
        let result = service
            .network
            .node_latency(
                &NodeId("c".into()),
                &f.state.app_config.behavior.latency.test_url.0,
                Duration::from_millis(30),
            )
            .await;
        assert_eq!(result, Err(NetworkError::NodeNotApplied));
        assert_eq!(
            LatencyOutcome::from_result(&result),
            LatencyOutcome::Unknown
        );
        assert!(
            service
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .find(|e| e.outbound.id == OutboundId::Node(NodeId("b".into())))
                .unwrap()
                .history
                .is_empty()
        );
        server.abort();
    });
}

// 保护跨组/单节点批次共享四个网络槽，取消排队请求不会泄漏permit或写历史。
#[test]
fn concurrent_batches_share_limit_and_cancel_waiters() {
    executor().block_on(async {
        let f = Fixture::new();
        let (probe, server) = controller(&f, 0).await;
        let service = f.service();
        let first = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        let second = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        let cancel1 = first.cancellation();
        let cancel2 = second.cancellation();
        let (a, b, ()) = tokio::join!(service.run_test(first), service.run_test(second), async {
            probe.started.acquire_many(4).await.unwrap().forget();
            assert_eq!(probe.requests.lock().unwrap().len(), 4);
            assert_eq!(probe.maximum.load(Ordering::SeqCst), MAX_CONCURRENT_TESTS);
            cancel1.cancel();
            cancel2.cancel();
        });
        assert_eq!(a.len() + b.len(), 6);
        assert!(
            a.iter()
                .chain(&b)
                .all(|r| matches!(r.disposition, TestDisposition::Cancelled))
        );
        assert_eq!(service.permits.available_permits(), MAX_CONCURRENT_TESTS);
        assert!(
            service
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .all(|e| e.history.is_empty())
        );
        server.abort();
    });
}

// 保护新组成员/新实例出现时迟到实测不回写；下一批重新从唯一目录读取真实成员。
#[test]
fn group_edit_and_instance_replacement_discard_late_results() {
    executor().block_on(async {
        let mut f = Fixture::new();
        let (probe, server) = controller(&f, 0).await;
        let service = f.service();
        let old = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        let (results, ()) = tokio::join!(service.run_test(old), async {
            probe.started.acquire_many(3).await.unwrap().forget();
            f.state.pools[0].sources[0].filter.exclude_node_ids = vec![NodeId("c".into())];
            f.state = f.store.commit(&f.state).unwrap();
            probe.release.add_permits(3);
        });
        assert!(
            results
                .iter()
                .all(|r| matches!(r.disposition, TestDisposition::StateChanged))
        );
        assert!(matches!(
            service.begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into()))),
            Err(ProxyError::ConfigNotApplied)
        ));
        let pending_snapshot = service.snapshot(&f.runtime).unwrap();
        let group = pending_snapshot
            .entries
            .iter()
            .find(|entry| entry.outbound.id == OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        assert_eq!(
            group.outbound.references,
            [
                OutboundId::Node(NodeId("a".into())),
                OutboundId::Node(NodeId("b".into()))
            ]
        );
        assert_eq!(
            group.applied_members,
            Some(vec![
                OutboundId::Node(NodeId("a".into())),
                OutboundId::Node(NodeId("b".into())),
                OutboundId::Node(NodeId("c".into()))
            ])
        );
        f.runtime.runtime.saved_version = f.state.config_version();
        f.runtime.runtime.applied_version = Some(f.state.config_version());
        f.publish(f.source.current().unwrap().address());
        let request = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        assert_eq!(request.nodes(), [NodeId("a".into()), NodeId("b".into())]);
        f.source.revoke();
        let results = service.run_test(request).await;
        assert!(
            results
                .iter()
                .all(|r| matches!(r.disposition, TestDisposition::InstanceChanged))
        );
        assert!(
            service
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .all(|e| e.history.is_empty())
        );
        server.abort();
    });
}

// 保护新偏好在下一快照真实生效；阈值边界、未知排末尾，隐藏只消费显式不可用。
#[test]
fn live_view_preferences_sort_thresholds_and_hide_only_explicit_unavailability() {
    let mut f = Fixture::new();
    let service = f.service();
    let mut record = LatencyRecord {
        epoch: f.state.state_epoch.clone(),
        node: NodeId("a".into()),
        tested_at_ms: 10,
        instance_id: "owned-p208".into(),
        generation: 1,
        outcome: LatencyOutcome::Success(200),
    };
    service
        .history
        .lock()
        .unwrap()
        .append(None, &record)
        .unwrap();
    record.node = NodeId("b".into());
    record.outcome = LatencyOutcome::Zero;
    service
        .history
        .lock()
        .unwrap()
        .append(None, &record)
        .unwrap();
    f.state.nodes[0].name = "Zulu".into();
    f.state.nodes[1].name = "Alpha".into();
    f.state.app_config.behavior.latency.low_ms = 100;
    f.state.app_config.behavior.latency.medium_ms = 200;
    f.state.app_config.behavior.proxy_view.node_sort = NodeSort::NameAsc;
    f.state.app_config.behavior.proxy_view.hide_unavailable = true;
    f.state = f.store.commit(&f.state).unwrap();
    let mut snapshot = service.snapshot(&f.runtime).unwrap();
    assert_eq!(
        snapshot.visible_nodes()[0].outbound.id,
        OutboundId::Node(NodeId("b".into()))
    );
    assert_eq!(
        snapshot
            .entries
            .iter()
            .find(|e| e.outbound.id == OutboundId::Node(NodeId("a".into())))
            .unwrap()
            .band,
        Some(LatencyBand::Medium)
    );
    snapshot.view_preferences.node_sort = NodeSort::LatencyDesc;
    assert_eq!(
        snapshot.visible_nodes()[0].outbound.id,
        OutboundId::Node(NodeId("a".into()))
    );
    assert_eq!(snapshot.visible_nodes().len(), 3); // Zero不等于Controller alive=false。
    snapshot
        .entries
        .iter_mut()
        .find(|e| e.outbound.id == OutboundId::Node(NodeId("a".into())))
        .unwrap()
        .outbound
        .availability =
        OutboundAvailability::Unavailable(crate::domain::OutboundUnavailableReason::Disabled);
    assert_eq!(snapshot.visible_nodes().len(), 2);
}

// 保护主备线路实际选择请求保留selector ID和父group ID，沿用owner要求的Auto lane模式。
#[test]
fn failover_lane_selection_adaptor_keeps_parent_group_and_selector_ids() {
    use crate::domain::{FailoverLane, FailoverSettings, GroupRule, NodeGroup};
    let mut f = Fixture::new();
    let mut group = NodeGroup::new(PoolId("failover".into()));
    group.name = "failover".into();
    group.rule = GroupRule::Failover;
    group.failover = Some(FailoverSettings::default());
    group.lanes = vec![
        FailoverLane {
            id: "primary".into(),
            name: String::new(),
            icon: String::new(),
            manual: true,
            members: vec![
                OutboundId::Node(NodeId("a".into())),
                OutboundId::Node(NodeId("b".into())),
            ],
        },
        FailoverLane {
            id: "backup".into(),
            name: String::new(),
            icon: String::new(),
            manual: false,
            members: vec![OutboundId::Node(NodeId("c".into()))],
        },
    ];
    let selector = group.lanes[0].pool_id(&group.id);
    f.state.groups.push(group);
    f.state = f.store.commit(&f.state).unwrap();
    f.runtime.runtime.saved_version = f.state.config_version();
    f.runtime.saved_selection_version = f.state.selection_version();
    f.publish("127.0.0.1:9".parse().unwrap());
    let service = f.service();
    let request = service
        .selection_request(
            &f.runtime,
            selector.clone(),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::Auto,
        )
        .unwrap();
    assert_eq!(request.pool, selector);
    assert_eq!(
        request.node,
        SelectionChoice::Group(GroupChoice {
            group: PoolId("failover".into()),
            member: OutboundId::Node(NodeId("b".into())),
            mode: GroupSelectionMode::Auto
        })
    );
    assert!(matches!(
        service.selection_request(
            &f.runtime,
            selector,
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin
        ),
        Err(ProxyError::InvalidSelection)
    ));
}

// 保护组卡片当前延迟沿已确认稳定ID链取值，不取最后完成的任意组成员，Direct保持未知。
#[test]
fn selected_group_latency_uses_current_node_not_latest_group_sample() {
    let f = Fixture::new();
    f.publish("127.0.0.1:9".parse().unwrap());
    let service = f.service();
    for (node, ms, time) in [("a", 25, 1), ("b", 1000, 2)] {
        service
            .history
            .lock()
            .unwrap()
            .append(
                Some(&OutboundId::Pool(PoolId("manual".into()))),
                &LatencyRecord {
                    epoch: f.state.state_epoch.clone(),
                    node: NodeId(node.into()),
                    tested_at_ms: time,
                    instance_id: "owned-p208".into(),
                    generation: 1,
                    outcome: LatencyOutcome::Success(ms),
                },
            )
            .unwrap();
    }
    let snapshot = service.snapshot(&f.runtime).unwrap();
    let id = OutboundId::Pool(PoolId("manual".into()));
    let group = snapshot
        .entries
        .iter()
        .find(|e| e.outbound.id == id)
        .unwrap();
    assert_eq!(group.history[0].outcome, LatencyOutcome::Success(1000));
    assert_eq!(group.band, None);
    assert_eq!(snapshot.selected_latency(&id), Some(25));
    assert_eq!(snapshot.selected_latency(&OutboundId::Direct), None);
}

// 保护退出取消正在请求和排队请求；用户超过30秒的合法偏好保持原值，不被截断。
#[test]
fn shutdown_cancels_inflight_and_preserves_long_timeout_preference() {
    executor().block_on(async {
        let mut f = Fixture::new();
        f.state.app_config.behavior.latency.timeout_ms = 60_000;
        f.state = f.store.commit(&f.state).unwrap();
        f.runtime.runtime.saved_version = f.state.config_version();
        f.runtime.runtime.applied_version = Some(f.state.config_version());
        let (probe, server) = controller(&f, 0).await;
        let service = f.service();
        let request = service
            .begin_test(&f.runtime, OutboundId::Pool(PoolId("manual".into())))
            .unwrap();
        let (results, ()) = tokio::join!(service.run_test(request), async {
            probe.started.acquire_many(3).await.unwrap().forget();
            assert!(
                probe
                    .requests
                    .lock()
                    .unwrap()
                    .iter()
                    .all(|r| r.ends_with("timeout=60000"))
            );
            service.shutdown();
        });
        assert!(
            results
                .iter()
                .all(|r| matches!(r.disposition, TestDisposition::Cancelled))
        );
        assert!(
            service
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .all(|e| e.history.is_empty())
        );
        assert!(matches!(
            service.begin_test(&f.runtime, OutboundId::Node(NodeId("a".into()))),
            Err(ProxyError::NotReady)
        ));
        server.abort();
    });
}

// 最近失败仍展示失败，但排序/当前组的最近有效延迟不能丢失。
#[test]
fn recent_failure_retains_last_success_for_sort_and_selected_latency() {
    let f = Fixture::new();
    let mut snapshot = f.service().snapshot(&f.runtime).unwrap();
    let node = snapshot
        .entries
        .iter_mut()
        .find(|e| e.outbound.id == OutboundId::Node(NodeId("a".into())))
        .unwrap();
    node.history = [
        LatencyOutcome::Timeout,
        LatencyOutcome::Zero,
        LatencyOutcome::Success(25),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, outcome)| LatencyRecord {
        epoch: f.state.state_epoch.clone(),
        node: NodeId("a".into()),
        tested_at_ms: 10 - i as i64,
        instance_id: "owned-p208".into(),
        generation: i as u64,
        outcome,
    })
    .collect();
    assert_eq!(node.history[0].outcome, LatencyOutcome::Timeout);
    assert_eq!(
        snapshot.selected_latency(&OutboundId::Node(NodeId("a".into()))),
        Some(25)
    );
    snapshot.view_preferences.node_sort = NodeSort::LatencyAsc;
    assert_eq!(
        snapshot.visible_nodes()[0].outbound.id,
        OutboundId::Node(NodeId("a".into()))
    );
}

/// 真实 State gate 忙时，不能声称配置变化，也不能覆盖已有成功历史。
#[test]
fn result_state_busy_is_storage_failure_and_retains_history() {
    let mut f = Fixture::new();
    let gate = StateAccessGate::default();
    f.snapshots = SnapshotService::new(f.store.clone(), gate.clone());
    f.publish("127.0.0.1:1".parse().unwrap());
    let service = f.service();
    let node = NodeId("a".into());
    let request = service
        .begin_test(&f.runtime, OutboundId::Node(node.clone()))
        .unwrap();
    let result = Ok(super::super::network::Latency {
        milliseconds: Some(25),
        path: super::super::network::UsedPath::Node {
            instance_id: "owned-p208".into(),
            node_id: node.clone(),
        },
    });
    assert!(matches!(
        service.record_result(&request, &node, &result),
        TestDisposition::Recorded
    ));
    let before = service.snapshot(&f.runtime).unwrap();
    let request = service
        .begin_test(&f.runtime, OutboundId::Node(node.clone()))
        .unwrap();
    let guard = gate.try_lock().unwrap();
    let disposition = service.record_result(&request, &node, &result);
    assert!(
        matches!(disposition, TestDisposition::StateFailed(ProxyError::State(error))
        if error == AppError::new(crate::domain::AppErrorCode::StorageFailed)
            .with_detail(crate::domain::ErrorDetail::Busy))
    );
    drop(guard);
    let after = service.snapshot(&f.runtime).unwrap();
    let history = |snapshot: &ProxySnapshot| {
        snapshot
            .entries
            .iter()
            .find(|entry| entry.outbound.id == OutboundId::Node(node.clone()))
            .unwrap()
            .history
            .clone()
    };
    assert_eq!(history(&before), history(&after));
    assert_eq!(history(&after)[0].outcome, LatencyOutcome::Success(25));
    assert!(matches!(
        service.record_result(&request, &node, &result),
        TestDisposition::Recorded
    ));
}

// 自动组真正应用后可测；同步快照不请求 HTTP，当前出口只能由鉴权 GET 取得。
#[test]
fn applied_auto_pool_snapshot_latency_and_authenticated_now() {
    executor().block_on(async {
        let f = Fixture::new();
        let (probe, server) = controller(&f, 20).await;
        let service = f.service();
        let bytes = std::fs::read(f.root.join("state.json")).unwrap();
        let auto = PoolId("auto".into());
        let snapshot = service.snapshot(&f.runtime).unwrap();
        let entry = snapshot
            .entries
            .iter()
            .find(|e| e.outbound.id == OutboundId::Pool(auto.clone()))
            .unwrap();
        assert_eq!(entry.applied, Some(true));
        assert_eq!(
            entry.applied_members,
            Some(vec![
                OutboundId::Node(NodeId("a".into())),
                OutboundId::Node(NodeId("b".into())),
                OutboundId::Node(NodeId("c".into()))
            ])
        );
        assert_eq!(entry.current, None); // Compiler 默认与 Saved 不是观测。
        assert!(probe.requests.lock().unwrap().is_empty());
        let request = service
            .begin_test(&f.runtime, OutboundId::Pool(auto.clone()))
            .unwrap();
        assert_eq!(
            request.nodes(),
            [NodeId("a".into()), NodeId("b".into()), NodeId("c".into())]
        );
        let results = service.run_test(request).await;
        assert!(
            results
                .iter()
                .all(|r| matches!(r.disposition, TestDisposition::Recorded))
        );
        probe.replies.lock().unwrap().insert(
            "/proxies/pool-auto".into(),
            (200, "{\"now\":\"node-b\"}".into()),
        );
        let observed = service
            .observe_applied_pool(&f.runtime, &auto)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observed.current, Some(OutboundId::Node(NodeId("b".into()))));
        assert_eq!(
            observed.instance_id,
            f.runtime.runtime.instance_id.clone().unwrap()
        );
        assert_eq!(observed.config, f.state.config_version());
        assert_eq!(observed.pool_id, auto);
        assert!(
            probe
                .requests
                .lock()
                .unwrap()
                .iter()
                .any(|uri| uri == "/proxies/pool-auto")
        );
        // 观测是返回值，不写新的选择/缓存事实；同步快照仍不猜 auto 的当前出口。
        assert_eq!(
            service
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .find(|e| e.outbound.id == OutboundId::Pool(auto.clone()))
                .unwrap()
                .current,
            None
        );
        assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
        drop(service);
        assert_eq!(
            f.service()
                .snapshot(&f.runtime)
                .unwrap()
                .entries
                .iter()
                .find(|e| e.outbound.id == OutboundId::Pool(auto.clone()))
                .unwrap()
                .history
                .len(),
            3
        );
        server.abort();
    });
}

// 嵌套自动/手动组和主备线路按实际编译成员顺序展开；重复引用去重，不把 Direct 当成功。
#[test]
fn applied_nested_groups_and_lanes_keep_member_order() {
    use crate::domain::{FailoverLane, FailoverSettings, GroupRule, NodeGroup};
    let mut f = Fixture::new();
    let auto = PoolId("nested-auto".into());
    let group = PoolId("nested-selector".into());
    let mut a = NodeGroup::new(auto.clone());
    a.name = "auto".into();
    a.members = vec![
        OutboundId::Node(NodeId("b".into())),
        OutboundId::Node(NodeId("a".into())),
    ];
    let mut g = NodeGroup::new(group.clone());
    g.name = "selector".into();
    g.rule = GroupRule::Selector;
    g.members = vec![
        OutboundId::Pool(auto.clone()),
        OutboundId::Node(NodeId("c".into())),
        OutboundId::Node(NodeId("a".into())),
        OutboundId::Direct,
    ];
    let mut failover = NodeGroup::new(PoolId("ordered-lanes".into()));
    failover.name = "failover".into();
    failover.rule = GroupRule::Failover;
    failover.failover = Some(FailoverSettings::default());
    failover.lanes = vec![
        FailoverLane {
            id: "primary".into(),
            name: String::new(),
            icon: String::new(),
            manual: false,
            members: vec![OutboundId::Pool(group.clone())],
        },
        FailoverLane {
            id: "backup".into(),
            name: String::new(),
            icon: String::new(),
            manual: true,
            members: vec![OutboundId::Node(NodeId("c".into()))],
        },
    ];
    let lane = failover.lanes[0].pool_id(&failover.id);
    f.state.groups = vec![a, g, failover];
    f.state = f.store.commit(&f.state).unwrap();
    f.runtime.runtime.applied_version = Some(f.state.config_version());
    f.publish("127.0.0.1:9".parse().unwrap());
    let service = f.service();
    let request = service
        .begin_test(&f.runtime, OutboundId::Pool(group))
        .unwrap();
    assert_eq!(
        request.nodes(),
        [NodeId("b".into()), NodeId("a".into()), NodeId("c".into())]
    );
    assert_eq!(request.untestable, [OutboundId::Direct]);
    let request = service
        .begin_test(&f.runtime, OutboundId::Pool(lane))
        .unwrap();
    assert_eq!(
        request.nodes(),
        [NodeId("b".into()), NodeId("a".into()), NodeId("c".into())]
    );
    assert_eq!(request.untestable, [OutboundId::Direct]);
}

// 索引缺失、错误实例/应用配置与缺失 Pool 必须拒绝；确认选择不能替代已应用边。
#[test]
fn missing_stale_and_unapplied_facts_reject_saved_fallback() {
    let mut f = Fixture::new();
    let service = f.service();
    let auto = OutboundId::Pool(PoolId("auto".into()));
    assert!(matches!(
        service.begin_test(&f.runtime, auto.clone()),
        Err(ProxyError::NotReady)
    ));
    f.publish("127.0.0.1:9".parse().unwrap());
    let mut proxy = f.source.current().unwrap();
    // 同实例的新能力没有完整索引（旧恢复格式），不能拿原 Saved 目录补齐。
    let mut legacy = RunningProxy::ready(proxy.instance_id().into(), proxy.address()).unwrap();
    legacy.nodes = proxy.nodes.clone();
    f.source.publish(legacy);
    assert!(matches!(
        service.begin_test(&f.runtime, auto.clone()),
        Err(ProxyError::GroupApplicationUnknown)
    ));
    assert!(
        service
            .snapshot(&f.runtime)
            .unwrap()
            .entries
            .iter()
            .all(|e| e.applied.is_none() && e.applied_members.is_none() && e.current.is_none())
    );
    f.publish("127.0.0.1:9".parse().unwrap());
    proxy = f.source.current().unwrap();
    let mut missing = RunningProxy::ready(proxy.instance_id().into(), proxy.address()).unwrap();
    missing.nodes = proxy.nodes.clone();
    let mut index = (*proxy.applied_index.clone().unwrap()).clone();
    index
        .applied_members
        .as_mut()
        .unwrap()
        .remove(&PoolId("auto".into()));
    missing.applied_index = Some(Arc::new(index));
    f.source.publish(missing);
    assert!(matches!(
        service.begin_test(&f.runtime, auto.clone()),
        Err(ProxyError::GroupApplicationUnknown)
    ));
    let snap = service.snapshot(&f.runtime).unwrap();
    assert_eq!(
        snap.entries
            .iter()
            .find(|e| e.outbound.id == auto)
            .unwrap()
            .applied,
        Some(false)
    );
    f.publish("127.0.0.1:9".parse().unwrap());
    f.runtime
        .runtime
        .applied_version
        .as_mut()
        .unwrap()
        .0
        .revision += 1;
    assert!(
        service
            .snapshot(&f.runtime)
            .unwrap()
            .entries
            .iter()
            .all(|e| e.applied.is_none())
    );
    assert!(matches!(
        service.begin_test(&f.runtime, auto.clone()),
        Err(ProxyError::ConfigNotApplied)
    ));
    f.runtime.runtime.applied_version = Some(f.state.config_version());
    f.runtime.runtime.instance_id = Some(InstanceId("other-instance".into()));
    assert!(matches!(
        service.begin_test(&f.runtime, auto),
        Err(ProxyError::NotReady)
    ));
}

// GET 返回 Pool/Direct 时保持原领域类型；断连、非法 now 为 Unknown，不写选择或递归猜叶子。
#[test]
fn applied_pool_observation_keeps_pool_direct_and_unknown() {
    use crate::domain::{GroupRule, NodeGroup};
    executor().block_on(async {
        let mut f = Fixture::new();
        let group = PoolId("outer".into());
        let mut g = NodeGroup::new(group.clone());
        g.name = "outer".into();
        g.rule = GroupRule::Selector;
        g.members = vec![OutboundId::Pool(PoolId("auto".into())), OutboundId::Direct];
        f.state.groups.push(g);
        f.state = f.store.commit(&f.state).unwrap();
        f.runtime.runtime.applied_version = Some(f.state.config_version());
        let (probe, server) = controller(&f, 20).await;
        let service = f.service();
        let bytes = std::fs::read(f.root.join("state.json")).unwrap();
        for (now, expected) in [
            ("pool-auto", Some(OutboundId::Pool(PoolId("auto".into())))),
            ("direct", Some(OutboundId::Direct)),
            ("block", None),
            ("foreign-node", None),
        ] {
            probe.replies.lock().unwrap().insert(
                "/proxies/pool-outer".into(),
                (200, serde_json::json!({"now":now}).to_string()),
            );
            assert_eq!(
                service
                    .observe_applied_pool(&f.runtime, &group)
                    .await
                    .unwrap()
                    .unwrap()
                    .current,
                expected
            );
        }
        assert_eq!(probe.requests.lock().unwrap().len(), 4); // 无隐式 nested GET。
        probe
            .replies
            .lock()
            .unwrap()
            .insert("/proxies/pool-outer".into(), (503, "{}".into()));
        assert_eq!(
            service
                .observe_applied_pool(&f.runtime, &group)
                .await
                .unwrap()
                .unwrap()
                .current,
            None
        );
        assert!(
            service
                .observe_applied_pool(&f.runtime, &PoolId("missing".into()))
                .await
                .unwrap()
                .is_none()
        );
        server.abort();
        let _ = server.await;
        assert_eq!(
            service
                .observe_applied_pool(&f.runtime, &group)
                .await
                .unwrap()
                .unwrap()
                .current,
            None
        );
        assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
    });
}

// 在途 GET 的实例替换/保存变更丢弃旧结果；真实两秒读超时只报 Unknown。
#[test]
fn applied_pool_observation_discards_replacement_and_config_change() {
    executor().block_on(async {
        let mut f = Fixture::new();
        let (probe, server) = controller(&f, 0).await;
        let service = f.service();
        let auto = PoolId("auto".into());
        let (observed, ()) = tokio::join!(service.observe_applied_pool(&f.runtime, &auto), async {
            probe.started.acquire().await.unwrap().forget();
            f.publish(f.source.current().unwrap().address()); // 相同文本 InstanceId 也撤销旧能力。
            probe.release.add_permits(1);
        });
        assert!(observed.unwrap().is_none());
        let (observed, ()) = tokio::join!(service.observe_applied_pool(&f.runtime, &auto), async {
            probe.started.acquire().await.unwrap().forget();
            f.state.pools[0].name.push_str(" changed");
            f.state = f.store.commit(&f.state).unwrap();
            probe.release.add_permits(1);
        });
        assert!(observed.unwrap().is_none());
        assert!(matches!(
            service.begin_test(&f.runtime, OutboundId::Pool(auto.clone())),
            Err(ProxyError::ConfigNotApplied)
        ));
        // 已保存新配置不冒充已应用；只读观测可明确查询仍运行的旧应用版本。
        let observed = service
            .observe_applied_pool(&f.runtime, &auto)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(observed.current, None);
        assert_eq!(
            observed.config,
            f.runtime.runtime.applied_version.clone().unwrap()
        );
        server.abort();
    });
}

// 已保存旧 HTTP 目标原样保留并返回不支持，Controller 不收到替换 URL 或伪执行请求。
#[test]
fn applied_auto_pool_preserves_unsupported_http_preference() {
    executor().block_on(async {
        let mut f = Fixture::new();
        f.state.app_config.behavior.latency.test_url =
            crate::domain::UiLatencyUrl("http://www.gstatic.com/generate_204".into());
        f.state = f.store.commit(&f.state).unwrap();
        f.runtime.runtime.applied_version = Some(f.state.config_version());
        let (probe, server) = controller(&f, 10).await;
        let service = f.service();
        let bytes = std::fs::read(f.root.join("state.json")).unwrap();
        let results = service
            .run_test(
                service
                    .begin_test(&f.runtime, OutboundId::Pool(PoolId("auto".into())))
                    .unwrap(),
            )
            .await;
        assert_eq!(results.len(), 3);
        assert!(
            results
                .iter()
                .all(|r| r.result == Err(NetworkError::UnsupportedNodeTestUrl))
        );
        assert!(
            results
                .iter()
                .all(|r| LatencyOutcome::from_result(&r.result) == LatencyOutcome::Unknown)
        );
        assert!(probe.requests.lock().unwrap().is_empty());
        assert!(
            service
                .snapshot(&f.runtime)
                .unwrap()
                .latency_preferences
                .test_url
                .0
                .starts_with("http://")
        );
        assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
        server.abort();
    });
}

// 保护普通 Selector 的真实点击契约：ManualPin 仅用于主备外层，普通组沿用 Auto。
#[test]
fn ordinary_selector_snapshot_adaptor_uses_owner_auto_mode() {
    use crate::domain::{GroupRule, NodeGroup};
    let mut f = Fixture::new();
    let mut g = NodeGroup::new(PoolId("selector-ui".into()));
    g.name = "selector-ui".into();
    g.rule = GroupRule::Selector;
    g.members = vec![OutboundId::Node(NodeId("a".into())), OutboundId::Direct];
    f.state.groups = vec![g.clone()];
    f.state = f.store.commit(&f.state).unwrap();
    f.runtime.runtime.saved_version = f.state.config_version();
    f.runtime.runtime.applied_version = Some(f.state.config_version());
    f.runtime.saved_selection_version = f.state.selection_version();
    f.publish("127.0.0.1:9".parse().unwrap());
    let service = f.service();
    let snapshot = service.snapshot(&f.runtime).unwrap();
    let entry = snapshot
        .entries
        .iter()
        .find(|e| e.outbound.id == g.outbound_id())
        .unwrap();
    assert_eq!(entry.selection_mode, GroupSelectionMode::Auto);
    let request = service
        .selection_request(
            &f.runtime,
            g.id.clone(),
            OutboundId::Direct,
            entry.selection_mode,
        )
        .unwrap();
    assert_eq!(
        request.node,
        SelectionChoice::Group(GroupChoice {
            group: g.id,
            member: OutboundId::Direct,
            mode: GroupSelectionMode::Auto
        })
    );
    assert_eq!(
        snapshot
            .entries
            .iter()
            .find(|e| e.outbound.id == OutboundId::Pool(PoolId("manual".into())))
            .unwrap()
            .selection_mode,
        GroupSelectionMode::ManualPin
    );
}

// 保护策略/节点组读模型按稳定身份连接真实元数据，而非同名或图标名称启发式。
#[test]
fn policy33_proxy_metadata_distinguishes_routing_and_configured_group() {
    let mut fixture = Fixture::new();
    let mut policy = crate::domain::TargetRoutingPolicy::new(PoolId("routing-meta".into()));
    policy.name = "同名合法".into();
    policy.icon = "misc:pin".into();
    policy.icon_scale = 3;
    policy.matchers = vec![crate::domain::TrafficMatcher::Domain(vec![
        "example.invalid".into(),
    ])];
    let mut group = crate::domain::NodeGroup::new(PoolId("node-group-meta".into()));
    group.name = policy.name.clone();
    group.icon = "globe:earth-asia".into();
    group.icon_scale = -3;
    group.rule = crate::domain::GroupRule::Selector;
    group.members = vec![OutboundId::Direct];
    fixture.state.profile.routing.policies.push(policy.clone());
    fixture.state.groups.push(group.clone());
    fixture.store.save(&fixture.state).unwrap();
    fixture.publish("127.0.0.1:9".parse().unwrap());
    let service = fixture.service();
    let snapshot = service.snapshot(&fixture.runtime).unwrap();
    let entry = snapshot
        .entries
        .iter()
        .find(|e| e.outbound.id == OutboundId::Pool(policy.id.clone()))
        .unwrap();
    assert_eq!(entry.routing_policy, Some(policy.clone()));
    assert!(!entry.routing_fallback);
    assert!(entry.configured_group.is_none());
    assert_eq!(entry.selection_mode, GroupSelectionMode::Auto);
    let node_group = snapshot
        .entries
        .iter()
        .find(|e| e.outbound.id == OutboundId::Pool(group.id.clone()))
        .unwrap();
    assert!(node_group.routing_policy.is_none());
    assert_eq!(node_group.configured_group, Some(group));
    let request = service
        .selection_request(
            &fixture.runtime,
            policy.id.clone(),
            OutboundId::Direct,
            GroupSelectionMode::Auto,
        )
        .unwrap();
    assert_eq!(
        request.node,
        SelectionChoice::Group(GroupChoice {
            group: policy.id,
            member: OutboundId::Direct,
            mode: GroupSelectionMode::Auto
        })
    );
    assert_eq!(request.config, fixture.state.config_version());
    assert_eq!(request.expected, fixture.state.selection_version());
    assert_eq!(fixture.store.load().unwrap(), fixture.state);
}

// 保护页面目录、节点名称及偏好来自同次正式状态；已有 Service 也立即读到后续保存，不缓存或写盘。
#[test]
fn policy49_snapshot_projects_live_saved_directory_and_preferences() {
    let f = Fixture::new();
    let service = f.service();
    let mut state = f.store.load().unwrap();
    for (name, width) in [("正式第一次改名", 230), ("正式第二次改名", 145)] {
        state.nodes[0].name = name.into();
        state.app_config.behavior.proxy_view.node_card_min_width = width;
        let saved = f.store.commit(&state).unwrap();
        let bytes = std::fs::read(f.root.join("state.json")).unwrap();
        let snapshot = service.snapshot(&f.runtime).unwrap();
        assert_eq!(snapshot.version, saved.version());
        assert_eq!(
            snapshot.view_preferences,
            saved.app_config.behavior.proxy_view
        );
        assert_eq!(
            snapshot.latency_preferences,
            saved.app_config.behavior.latency
        );
        let expected = OutboundQuerySnapshot::from_state(&saved);
        assert_eq!(
            snapshot
                .entries
                .iter()
                .map(|entry| &entry.outbound)
                .collect::<Vec<_>>(),
            expected.list().iter().collect::<Vec<_>>()
        );
        assert_eq!(
            snapshot
                .entries
                .iter()
                .find(|entry| entry.outbound.id == OutboundId::Node(saved.nodes[0].id.clone()))
                .unwrap()
                .outbound
                .display_name
                .as_deref(),
            Some(name)
        );
        assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
        state = saved;
    }
}

// 保护首读 Busy 不吞掉或重试；目录同态化不放宽选择 Saved/selection fence、测速配置/写回检查。
#[test]
fn policy49_first_busy_and_changed_versions_remain_explicit() {
    let mut f = Fixture::new();
    let gate = StateAccessGate::default();
    f.snapshots = SnapshotService::new(f.store.clone(), gate.clone());
    // 仅发布正式 Compiler 生成的 fixture 索引；本测试不发 Controller 请求、不启动 child。
    f.publish("127.0.0.1:1".parse().unwrap());
    let service = f.service();
    let node = NodeId("a".into());
    let request = service
        .begin_test(&f.runtime, OutboundId::Node(node.clone()))
        .unwrap();
    let result = Ok(super::super::network::Latency {
        milliseconds: Some(25),
        path: super::super::network::UsedPath::Node {
            instance_id: "owned-p208".into(),
            node_id: node.clone(),
        },
    });
    let selection = |runtime: &ManualRuntimeSnapshot| {
        service.selection_request(
            runtime,
            PoolId("manual".into()),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin,
        )
    };
    let bytes = std::fs::read(f.root.join("state.json")).unwrap();
    let generation = service.requests.lock().unwrap().next;
    let guard = gate.try_lock().unwrap();
    let busy = |error: &ProxyError| {
        matches!(error, ProxyError::State(error)
        if *error == AppError::new(crate::domain::AppErrorCode::StorageFailed)
            .with_detail(crate::domain::ErrorDetail::Busy))
    };
    assert!(busy(&service.snapshot(&f.runtime).unwrap_err()));
    assert!(busy(&selection(&f.runtime).unwrap_err()));
    assert!(busy(
        &service
            .begin_test(&f.runtime, OutboundId::Node(node.clone()))
            .err()
            .unwrap()
    ));
    assert!(matches!(service.record_result(&request, &node, &result),
        TestDisposition::StateFailed(error) if busy(&error)));
    assert_eq!(service.requests.lock().unwrap().next, generation);
    assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), bytes);
    drop(guard);
    assert!(selection(&f.runtime).is_ok());
    let before_history = service
        .snapshot(&f.runtime)
        .unwrap()
        .entries
        .iter()
        .find(|e| e.outbound.id == OutboundId::Node(node.clone()))
        .unwrap()
        .history
        .clone();
    let mut state = f.store.load().unwrap();
    state
        .pools
        .iter_mut()
        .find(|pool| pool.id.0 == "manual")
        .unwrap()
        .selection = SelectionPolicy::Manual {
        selected_node_id: Some(NodeId("b".into())),
        pending_node_id: None,
    };
    let saved = f.store.commit(&state).unwrap();
    assert_eq!(saved.config_version(), f.runtime.runtime.saved_version);
    assert_ne!(saved.selection_version(), f.runtime.saved_selection_version);
    assert!(matches!(
        selection(&f.runtime),
        Err(ProxyError::SnapshotChanged)
    ));
    f.runtime.saved_selection_version = saved.selection_version();
    let current = selection(&f.runtime).unwrap();
    assert_eq!(current.expected, saved.selection_version());
    assert_eq!(current.config, saved.config_version());
    state = saved;
    state.nodes[0].port += 1;
    let changed = f.store.commit(&state).unwrap();
    assert!(matches!(
        selection(&f.runtime),
        Err(ProxyError::SnapshotChanged)
    ));
    f.runtime.runtime.saved_version = changed.config_version();
    assert!(matches!(
        service.begin_test(&f.runtime, OutboundId::Node(node.clone())),
        Err(ProxyError::ConfigNotApplied)
    ));
    assert!(matches!(
        service.record_result(&request, &node, &result),
        TestDisposition::StateChanged
    ));
    assert_eq!(
        service
            .snapshot(&f.runtime)
            .unwrap()
            .entries
            .iter()
            .find(|e| e.outbound.id == OutboundId::Node(node.clone()))
            .unwrap()
            .history,
        before_history
    );
    assert_eq!(f.store.load().unwrap(), changed);
}
