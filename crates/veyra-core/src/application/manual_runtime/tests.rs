use super::*;
use crate::{
    application::state_access::StateAccessGate,
    domain::{AppState, RouteTarget, StateEpoch},
    singbox::{
        GeneratedConfig,
        runtime::{ManagedSidecar, SidecarPortError},
    },
    storage::{JsonStateStore, StateStore},
};
use std::sync::Mutex;
#[derive(Default)]
struct Trace {
    observation_address: Option<std::net::SocketAddr>,
    observation_endpoint: Option<Arc<crate::singbox::clash_api::ManagedControllerEndpoint>>,
    next: u64,
    active: Option<u64>,
    checks: usize,
    stops: usize,
    fail_check: bool,
    fail_prepare: bool,
    fail_ready: bool,
    fail_stop: bool,
    crash: bool,
    config: Option<serde_json::Value>,
    selectors: BTreeMap<String, String>,
    writes: usize,
    reads: usize,
    write_failures: std::collections::VecDeque<bool>,
    read_failures: std::collections::VecDeque<bool>,
    mismatch_reads: std::collections::VecDeque<bool>,
    run_failures: usize,
    ready_failures: usize,
    save_obstacle: Option<PathBuf>,
    cache_choice: Option<String>,
    cache_choices: BTreeMap<String, String>,
    // 仅此 Mock 模式把单个 selector 写为 opaque cache；让回退来源错误可确定复现。
    selector_cache: Option<String>,
    events: Vec<String>,
    active_cache: Option<PathBuf>,
    cache_on_stop: Option<Vec<u8>>,
    business_file: Option<PathBuf>,
    business_at_write: Vec<AppState>,
    business_on_read: Option<AppState>,
    business_on_prepare: Option<AppState>,
    pending_pool_on_read: Option<(String, usize)>,
    selection_on_write: Option<SnapshotService>,
    selection_write_result: Option<Result<SelectionVersion, crate::domain::AppError>>,
    replacement_on_read: Option<(SnapshotService, AppState)>,
}
struct Mock(Arc<Mutex<Trace>>);
impl SidecarPort for Mock {
    fn observation_endpoint(
        &self,
        instance: &ManagedSidecar,
    ) -> Option<Arc<crate::singbox::clash_api::ManagedControllerEndpoint>> {
        let t = self.0.lock().unwrap();
        (t.active == Some(instance.identity()))
            .then(|| t.observation_endpoint.clone())
            .flatten()
    }
    fn write_selector(
        &mut self,
        i: &ManagedSidecar,
        pool: &str,
        node: &str,
    ) -> Result<(), SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        assert_eq!(t.active, Some(i.identity()));
        if let Some(path) = &t.business_file {
            let state = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
            t.business_at_write.push(state);
        }
        t.writes += 1;
        t.events.push("write".into());
        if let Some(snapshots) = t.selection_on_write.take() {
            let expected = t.business_at_write.last().unwrap().selection_version();
            t.selection_write_result = Some(
                SelectionService::new(snapshots)
                    .begin_manual_pending(expected, PoolId("manual".into()), NodeId("c".into()))
                    .map(|saved| saved.version),
            );
        }
        if t.write_failures.pop_front().unwrap_or(false) {
            return Err(SidecarPortError);
        }
        t.selectors.insert(pool.into(), node.into());
        if t.selector_cache.as_deref() == Some(pool) {
            std::fs::write(t.active_cache.as_ref().unwrap(), node).unwrap();
        }
        Ok(())
    }
    fn read_selector(&mut self, i: &ManagedSidecar, tag: &str) -> Result<String, SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        assert_eq!(t.active, Some(i.identity()));
        t.reads += 1;
        t.events.push("read".into());
        if t.read_failures.pop_front().unwrap_or(false) {
            return Err(SidecarPortError);
        }
        if t.mismatch_reads.pop_front().unwrap_or(false) {
            return Ok("different-tag".into());
        }
        if let Some(obstacle) = t.save_obstacle.take() {
            std::fs::create_dir(obstacle).unwrap();
        }
        if t.pending_pool_on_read
            .as_ref()
            .is_some_and(|(_, reads)| t.reads >= *reads)
        {
            let (id, _) = t.pending_pool_on_read.take().unwrap();
            let store = JsonStateStore::new(t.business_file.as_ref().unwrap().clone()).unwrap();
            add_pending_pool(&store, &id);
        }
        if let Some(candidate) = t.business_on_read.take() {
            JsonStateStore::new(t.business_file.as_ref().unwrap().clone())
                .unwrap()
                .commit(&candidate)
                .unwrap();
        }
        if let Some((snapshots, replacement)) = t.replacement_on_read.take() {
            let expected = snapshots.snapshot().unwrap().version();
            snapshots.replace(expected, replacement).unwrap();
        }
        t.selectors.get(tag).cloned().ok_or(SidecarPortError)
    }
    fn check(&mut self, c: &GeneratedConfig) -> Result<(), SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        if let Some(address) = t.observation_address {
            t.observation_endpoint = Some(Arc::new(
                crate::singbox::clash_api::ManagedControllerEndpoint::from_owned_child(address, c)
                    .unwrap(),
            ));
        }
        t.checks += 1;
        t.events.push("check".into());
        t.config = Some(serde_json::from_slice(c.as_bytes()).unwrap());
        if t.fail_check {
            Err(SidecarPortError)
        } else {
            Ok(())
        }
    }
    fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        if let Some(candidate) = t.business_on_prepare.take() {
            JsonStateStore::new(t.business_file.as_ref().unwrap().clone())
                .unwrap()
                .commit(&candidate)
                .unwrap();
        }
        if t.fail_prepare {
            Err(SidecarPortError)
        } else {
            Ok(())
        }
    }
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        assert!(t.active.is_none());
        if t.next > 0
            && let Some(expected) = &t.cache_on_stop
        {
            let parent = t.active_cache.as_ref().unwrap().parent().unwrap();
            let snapshot = parent.join(format!("rollback-{}.db", digest(expected)));
            assert_eq!(
                std::fs::read(snapshot).unwrap(),
                *expected,
                "copy must observe closed writer before new run"
            );
            t.events.push("snapshot-confirmed-before-run".into());
        }
        t.events.push("run".into());
        if t.run_failures > 0 {
            t.run_failures -= 1;
            return Err(SidecarPortError);
        }
        let config = t.config.clone().unwrap();
        let cache = PathBuf::from(
            config["experimental"]["cache_file"]["path"]
                .as_str()
                .unwrap(),
        );
        for pool in config["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["type"] == "selector")
        {
            let actual = t
                .cache_choices
                .get(pool["tag"].as_str().unwrap())
                .cloned()
                .or_else(|| t.cache_choice.clone())
                .or_else(|| {
                    (t.selector_cache.as_deref() == pool["tag"].as_str())
                        .then(|| std::fs::read_to_string(&cache).unwrap())
                        .filter(|cached| {
                            pool["outbounds"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|v| v == cached)
                        })
                })
                .unwrap_or_else(|| {
                    pool["default"]
                        .as_str()
                        .unwrap_or(pool["outbounds"][0].as_str().unwrap())
                        .into()
                });
            t.selectors
                .insert(pool["tag"].as_str().unwrap().into(), actual);
        }
        // Mock 的 opaque writer 字节只用于 handoff 测试，不解析真实 cache.db。
        let bytes = t.selector_cache.as_ref().map_or_else(
            || format!("writer-{}", t.next + 1),
            |tag| t.selectors[tag].clone(),
        );
        std::fs::write(&cache, bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        t.active_cache = Some(cache);
        t.next += 1;
        t.active = Some(t.next);
        Ok(ManagedSidecar::from_port_identity(t.next))
    }
    fn ready(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        let fail = t.fail_ready || t.ready_failures > 0;
        t.ready_failures = t.ready_failures.saturating_sub(1);
        if fail { Err(SidecarPortError) } else { Ok(()) }
    }
    fn endpoints(&self, i: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
        let t = self.0.lock().unwrap();
        (t.active == Some(i.identity())).then_some(ManagedRuntimeEndpoints {
            mixed: ([127, 0, 0, 1], 12345).into(),
            controller: ([127, 0, 0, 1], 54321).into(),
        })
    }
    fn is_alive(&mut self, i: &ManagedSidecar) -> Result<bool, SidecarPortError> {
        let t = self.0.lock().unwrap();
        Ok(t.active == Some(i.identity()) && !t.crash)
    }
    fn stop(&mut self, i: &ManagedSidecar) -> Result<(), SidecarPortError> {
        let mut t = self.0.lock().unwrap();
        if t.fail_stop {
            return Err(SidecarPortError);
        }
        assert_eq!(t.active, Some(i.identity()));
        if let Some(bytes) = &t.cache_on_stop {
            std::fs::write(t.active_cache.as_ref().unwrap(), bytes).unwrap();
        }
        t.active = None;
        t.stops += 1;
        t.events.push("reap".into());
        Ok(())
    }
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        Ok(())
    }
    fn has_pending_cleanup(&self) -> bool {
        false
    }
}
fn fixture() -> (
    ManualRuntime<Mock>,
    Arc<Mutex<Trace>>,
    JsonStateStore,
    PathBuf,
) {
    let mut value = serde_json::to_value(AppState::empty()).unwrap();
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/compiler/p2-02b-selected.json"
    ))
    .unwrap();
    for (k, v) in fixture.as_object().unwrap() {
        value[k] = v.clone();
    }
    let state: AppState = serde_json::from_value(value).unwrap();
    assert_eq!(state.default_target, RouteTarget::Unconfigured);
    let root = std::env::temp_dir().join(format!("veyra-p203-core-{:?}", state.state_epoch));
    let store = JsonStateStore::new(root.join("state.json")).unwrap();
    store.save(&state).unwrap();
    let snapshots = Arc::new(SnapshotService::new(
        store.clone(),
        StateAccessGate::default(),
    ));
    let trace = Arc::new(Mutex::new(Trace {
        business_file: Some(root.join("state.json")),
        ..Trace::default()
    }));
    (
        ManualRuntime::new(Mock(trace.clone()), snapshots, root.clone(), true),
        trace,
        store,
        root,
    )
}
// 保护首次导入 Unconfigured 真实正式服务路径、版本区别与失败保留旧进程。
#[test]
fn first_import_product_duplicate_check_failure_and_saved_applied() {
    let (mut owner, trace, store, root) = fixture();
    assert_eq!(
        owner.execute(RuntimeCommand::Start, |s| assert_eq!(
            s.runtime.status,
            super::super::runtime_snapshot::RuntimeStatus::Starting
        )),
        Ok(RuntimeResult::Started)
    );
    let before = owner.snapshot().unwrap();
    let config = trace.lock().unwrap().config.clone().unwrap();
    assert_eq!(config["inbounds"][0]["listen_port"], 0);
    assert_eq!(
        config["experimental"]["clash_api"]["external_controller"],
        "127.0.0.1:0"
    );
    assert!(
        config["route"]["final"]
            .as_str()
            .unwrap()
            .contains("runtime-active")
    );
    assert_eq!(
        owner.execute(RuntimeCommand::Start, |_| {}),
        Ok(RuntimeResult::AlreadyRunning)
    );
    assert_eq!(trace.lock().unwrap().checks, 1);
    let mut state = store.load().unwrap();
    state.profile.reject_quic = true;
    store.commit(&state).unwrap();
    trace.lock().unwrap().fail_check = true;
    assert_eq!(
        owner.execute(RuntimeCommand::Restart, |_| {}),
        Err(RuntimeError::CandidateFailed)
    );
    let after = owner.snapshot().unwrap();
    assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
    assert_eq!(
        after.runtime.applied_version,
        before.runtime.applied_version
    );
    assert_ne!(after.runtime.saved_version, before.runtime.saved_version);
    assert_eq!(trace.lock().unwrap().stops, 0);
    trace.lock().unwrap().fail_check = false;
    trace.lock().unwrap().fail_prepare = true;
    assert_eq!(
        owner.execute(RuntimeCommand::Restart, |_| {}),
        Err(RuntimeError::CandidateFailed)
    );
    assert_eq!(
        owner.snapshot().unwrap().runtime.instance_id,
        before.runtime.instance_id
    );
    assert_eq!(trace.lock().unwrap().stops, 0);
    trace.lock().unwrap().fail_prepare = false;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let stopped = owner.snapshot().unwrap();
    assert!(stopped.runtime.applied_version.is_none());
    assert!(stopped.endpoints.is_none());
    std::fs::remove_dir_all(root).unwrap();
}
// 保护 ready 失败清理、异常退出撤销 endpoint 及新实例恢复。
#[test]
fn timeout_cleanup_crash_refresh_and_restart_identity() {
    let (mut owner, trace, _, root) = fixture();
    trace.lock().unwrap().fail_ready = true;
    assert_eq!(
        owner.execute(RuntimeCommand::Start, |_| {}),
        Err(RuntimeError::CandidateFailed)
    );
    assert!(trace.lock().unwrap().active.is_none());
    assert!(owner.snapshot().unwrap().endpoints.is_none());
    trace.lock().unwrap().fail_ready = false;
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let before = owner.snapshot().unwrap();
    trace.lock().unwrap().crash = true;
    assert_eq!(
        owner.execute(RuntimeCommand::Refresh, |_| {}),
        Err(RuntimeError::UnexpectedExit)
    );
    let failed = owner.snapshot().unwrap();
    assert!(failed.endpoints.is_none());
    assert!(failed.runtime.applied_version.is_none());
    trace.lock().unwrap().crash = false;
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    assert_ne!(
        owner.snapshot().unwrap().runtime.instance_id,
        before.runtime.instance_id
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn stop_and_crash_cleanup_failure_never_retains_ready() {
    let (mut owner, trace, _, root) = fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    trace.lock().unwrap().fail_stop = true;
    assert_eq!(
        owner.execute(RuntimeCommand::Stop, |_| {}),
        Err(RuntimeError::StopFailed)
    );
    let s = owner.snapshot().unwrap();
    assert_eq!(
        s.runtime.status,
        super::super::runtime_snapshot::RuntimeStatus::Recovering
    );
    assert!(s.runtime.applied_version.is_none());
    assert!(s.uptime_seconds.is_none());
    assert!(s.endpoints.is_none());
    trace.lock().unwrap().fail_stop = false;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    {
        let mut t = trace.lock().unwrap();
        t.crash = true;
        t.fail_stop = true;
    }
    assert_eq!(
        owner.execute(RuntimeCommand::Refresh, |_| {}),
        Err(RuntimeError::StopFailed)
    );
    assert!(owner.snapshot().unwrap().runtime.applied_version.is_none());
    trace.lock().unwrap().fail_stop = false;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
fn manual_fixture() -> (
    ManualRuntime<Mock>,
    Arc<Mutex<Trace>>,
    JsonStateStore,
    PathBuf,
) {
    let (owner, trace, store, root) = fixture();
    let mut state = store.load().unwrap();
    let mut value = serde_json::to_value(&state).unwrap();
    let parts: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/compiler/p2-02b.json")).unwrap();
    for (k, v) in parts.as_object().unwrap() {
        value[k] = v.clone();
    }
    state = serde_json::from_value(value).unwrap();
    state.config_revision = 11;
    store.save(&state).unwrap();
    (owner, trace, store, root)
}
fn request(owner: &ManualRuntime<Mock>, node: &str) -> ManualSelectionRequest {
    let s = owner.snapshot().unwrap();
    ManualSelectionRequest {
        config: s.runtime.applied_version.clone().unwrap(),
        instance: s.runtime.instance_id.unwrap(),
        expected: s.confirmed_selection_version.unwrap(),
        pool: PoolId("manual".into()),
        node: NodeId(node.into()),
    }
}
// 保护 write/readback/save 的顺序、选择版本独立、旧实例与旧版本不能写 controller。
#[test]
fn selection_transaction_versions_and_stale_identity() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
    let before = owner.snapshot().unwrap();
    let plan_before = owner.store().unwrap().read_manifest().unwrap().plan;
    let stale = request(&owner, "a");
    let selected =
        if owner.active.as_ref().unwrap().selection.nodes[&PoolId("manual".into())].0 == "a" {
            "b"
        } else {
            "a"
        };
    let next = owner.select_manual(request(&owner, selected)).unwrap();
    let disk_before_put = trace
        .lock()
        .unwrap()
        .business_at_write
        .last()
        .unwrap()
        .clone();
    assert_eq!(
        disk_before_put.selection_revision,
        before
            .confirmed_selection_version
            .as_ref()
            .unwrap()
            .0
            .revision
            + 1
    );
    assert!(
        matches!(&disk_before_put.pools.iter().find(|p| p.id.0 == "manual").unwrap().selection,
        SelectionPolicy::Manual { selected_node_id, pending_node_id: Some(pending) } if pending.0 == selected && selected_node_id.as_ref() != Some(pending))
    );
    assert_eq!(
        next.0.revision,
        before.confirmed_selection_version.unwrap().0.revision + 2
    );
    assert_eq!(
        store.load().unwrap().config_version(),
        before.runtime.saved_version
    );
    assert_eq!(
        owner.snapshot().unwrap().runtime.applied_version,
        before.runtime.applied_version
    );
    assert_eq!(
        owner
            .snapshot()
            .unwrap()
            .runtime
            .last_successful_selection_version,
        Some(next.clone())
    );
    let m = owner.store().unwrap().read_manifest().unwrap();
    assert_ne!(m.selection_at_apply, m.confirmed_selection.version);
    assert_eq!(m.plan.digest, plan_before.digest);
    let writes = trace.lock().unwrap().writes;
    assert_eq!(
        owner.select_manual(stale),
        Err(SelectionError::StaleVersion)
    );
    let mut stale = request(&owner, "a");
    stale.instance = InstanceId("old".into());
    assert_eq!(
        owner.select_manual(stale),
        Err(SelectionError::StaleInstance)
    );
    assert_eq!(trace.lock().unwrap().writes, writes);
    let events = trace.lock().unwrap().events.clone();
    assert_eq!(&events[events.len() - 2..], ["write", "read"]);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let mut restarted = ManualRuntime::new(
        Mock(trace.clone()),
        owner.snapshots.clone(),
        root.clone(),
        true,
    );
    assert_eq!(restarted.snapshot().unwrap().runtime.applied_version, None);
    restarted
        .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
        .unwrap();
    assert_eq!(restarted.active.as_ref().unwrap().selection.version, next);
    assert_eq!(
        restarted.active.as_ref().unwrap().selection.nodes[&PoolId("manual".into())].0,
        selected
    );
    assert_ne!(
        restarted.snapshot().unwrap().runtime.instance_id,
        before.runtime.instance_id
    );
    restarted.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
// 保护 write 明确失败读回旧值才清 pending；read-back 不确定落盘且重建 owner 不丢失。
#[test]
fn selection_controller_failures_are_distinct() {
    for kind in [
        "write",
        "mismatch",
        "read",
        "write_unknown",
        "write_clear_fail",
    ] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let before = store.load().unwrap().selection_version();
        let req = request(&owner, "b");
        if kind == "mismatch" {
            trace.lock().unwrap().mismatch_reads.push_back(true);
        } else if kind == "read" {
            trace.lock().unwrap().read_failures.push_back(true);
        } else {
            trace.lock().unwrap().write_failures.push_back(true);
            if kind == "write_clear_fail" {
                trace.lock().unwrap().save_obstacle = Some(root.join("state.tmp"));
            }
            if kind == "write_unknown" {
                trace.lock().unwrap().read_failures.push_back(true);
            }
        }
        assert_eq!(
            owner.select_manual(req.clone()),
            Err(if kind == "write_clear_fail" {
                SelectionError::PendingClearFailed
            } else if kind.starts_with("write") {
                SelectionError::ControllerWrite
            } else {
                SelectionError::ControllerReadBack
            })
        );
        let state = store.load().unwrap();
        let uncertain = kind != "write";
        assert_eq!(
            state.selection_revision,
            before.0.revision + if uncertain { 1 } else { 2 }
        );
        assert_eq!(
            owner.snapshot().unwrap().pending_selection.is_some(),
            uncertain
        );
        assert_eq!(trace.lock().unwrap().writes, 1, "不默认补偿或重发");
        if uncertain {
            assert!(
                owner
                    .snapshot()
                    .unwrap()
                    .confirmed_manual_selections
                    .is_empty()
            );
            assert_eq!(owner.select_manual(req), Err(SelectionError::Pending));
        }
        if kind == "write_clear_fail" {
            std::fs::remove_dir(root.join("state.tmp")).unwrap();
        }
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        let restarted = ManualRuntime::new(
            Mock(trace.clone()),
            owner.snapshots.clone(),
            root.clone(),
            true,
        );
        assert_eq!(
            restarted.snapshot().unwrap().pending_selection.is_some(),
            uncertain
        );
        assert_eq!(restarted.snapshot().unwrap().runtime.applied_version, None);
        std::fs::remove_dir_all(root).unwrap();
    }
}
// 原补偿测试由已批准的持久 pending 契约替换：已切换而确认保存失败不伪装旧值、不自动补偿。
#[test]
fn selection_save_failure_persists_pending_without_compensation() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let before = store.load().unwrap();
    let req = request(&owner, "b");
    trace.lock().unwrap().save_obstacle = Some(root.join("state.tmp"));
    assert_eq!(
        owner.select_manual(req.clone()),
        Err(SelectionError::ConfirmationSaveFailed)
    );
    let after = store.load().unwrap();
    assert_eq!(after.selection_revision, before.selection_revision + 1);
    assert_eq!(after.config_version(), before.config_version());
    assert!(
        matches!(&after.pools.iter().find(|p| p.id.0 == "manual").unwrap().selection, SelectionPolicy::Manual { pending_node_id: Some(node), selected_node_id, } if node.0 == "b" && *selected_node_id != Some(node.clone()))
    );
    assert_eq!(trace.lock().unwrap().writes, 1);
    assert!(owner.snapshot().unwrap().pending_selection.is_some());
    assert_eq!(owner.snapshot().unwrap().confirmed_selection_version, None);
    assert_eq!(owner.select_manual(req), Err(SelectionError::Pending));
    std::fs::remove_dir(root.join("state.tmp")).unwrap();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let restarted = ManualRuntime::new(Mock(trace), owner.snapshots.clone(), root.clone(), true);
    assert_eq!(
        restarted
            .snapshot()
            .unwrap()
            .pending_selection
            .unwrap()
            .requested
            .0,
        "b"
    );
    std::fs::remove_dir_all(root).unwrap();
}
// 保护恢复记录失败不撤销已经确认的 controller/business 选择。
#[test]
fn selection_record_failure_keeps_business_confirmation() {
    let (mut owner, _, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let old = owner.snapshot().unwrap();
    owner.records.as_mut().unwrap().fail_manifest = true;
    assert_eq!(
        owner.select_manual(request(&owner, "b")),
        Err(SelectionError::RecoveryRecordFailed)
    );
    assert_eq!(
        owner.snapshot().unwrap().confirmed_selection_version,
        Some(store.load().unwrap().selection_version())
    );
    assert_eq!(
        owner
            .snapshot()
            .unwrap()
            .runtime
            .last_successful_selection_version,
        old.runtime.last_successful_selection_version
    );
    owner.records.as_mut().unwrap().fail_manifest = false;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let mut restart = ManualRuntime::new(
        Mock(Arc::new(Mutex::new(Trace::default()))),
        owner.snapshots.clone(),
        root.clone(),
        true,
    );
    restart
        .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
        .unwrap();
    assert_eq!(
        restart.active.as_ref().unwrap().selection.nodes[&PoolId("manual".into())],
        NodeId("b".into())
    );
    restart.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
// 保护 cache 不能决定业务选择；Ready 只在读回/必要写回完成后发布。
#[test]
fn startup_reconcile_matching_different_and_invalid_target() {
    for actual in ["node-b", "node-a", "deleted-tag"] {
        let (mut owner, trace, store, root) = manual_fixture();
        trace.lock().unwrap().cache_choice = Some(actual.into());
        owner
            .execute(RuntimeCommand::Start, |s| assert!(s.endpoints.is_none()))
            .unwrap();
        let state = store.load().unwrap();
        let index = owner
            .active
            .as_ref()
            .unwrap()
            .plan
            .artifact_index()
            .unwrap();
        let pool = &index.pools[&PoolId("manual".into())];
        let target = &pool.members
            [&owner.active.as_ref().unwrap().selection.nodes[&PoolId("manual".into())]];
        assert_eq!(trace.lock().unwrap().selectors[&pool.runtime_tag], *target);
        assert_eq!(
            owner.snapshot().unwrap().confirmed_selection_version,
            Some(state.selection_version())
        );
        let writes = trace.lock().unwrap().writes;
        assert_eq!(
            writes,
            index
                .pools
                .iter()
                .filter(|(id, p)| actual
                    != p.members[&owner.active.as_ref().unwrap().selection.nodes[*id]])
                .count()
        );
        let mut invalid = ConfirmedSelection {
            version: state.selection_version(),
            nodes: BTreeMap::from([(PoolId("manual".into()), NodeId("deleted".into()))]),
            groups: BTreeMap::new(),
        };
        invalid.version.0.revision += 1;
        let (fallback, warnings) = ManualRuntime::<Mock>::selection_for(
            &owner.active.as_ref().unwrap().plan,
            &state,
            Some(&invalid),
        );
        assert_eq!(fallback.nodes[&PoolId("manual".into())], pool.default_node);
        assert_eq!(warnings, vec![PoolId("manual".into())]);
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
// 保护 saved12 / last11：重开不自动启动，两个命令确实使用不同的不可变计划。
#[test]
fn stopped_saved12_last11_two_explicit_entries() {
    for restore in [false, true] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        let mut state = store.load().unwrap();
        state.profile.reject_quic = !state.profile.reject_quic;
        let state = store.commit(&state).unwrap();
        assert_eq!(state.config_revision, 12);
        let mut reopened = ManualRuntime::new(
            Mock(trace.clone()),
            owner.snapshots.clone(),
            root.clone(),
            true,
        );
        let s = reopened.snapshot().unwrap();
        assert_eq!(s.runtime.saved_version.0.revision, 12);
        assert_eq!(s.runtime.last_successful_version.unwrap().0.revision, 11);
        assert_eq!(s.runtime.applied_version, None);
        let runs = trace.lock().unwrap().next;
        reopened
            .execute(
                if restore {
                    RuntimeCommand::RestoreLastSuccessful
                } else {
                    RuntimeCommand::ApplySaved
                },
                |_| {},
            )
            .unwrap();
        assert_eq!(
            reopened
                .snapshot()
                .unwrap()
                .runtime
                .applied_version
                .unwrap()
                .0
                .revision,
            if restore { 11 } else { 12 }
        );
        assert_eq!(trace.lock().unwrap().next, runs + 1);
        reopened.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
// 保护候选run/Ready/reconcile失败只回退一次；失败旧check不停止。
#[test]
fn candidate_failures_rollback_once_and_failed_rollback() {
    for stage in ["run", "ready", "reconcile", "both"] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let old = owner.snapshot().unwrap().runtime.applied_version;
        let mut state = store.load().unwrap();
        state.profile.reject_quic = !state.profile.reject_quic;
        store.commit(&state).unwrap();
        {
            let mut t = trace.lock().unwrap();
            match stage {
                "run" => t.run_failures = 1,
                "ready" => t.ready_failures = 1,
                "reconcile" => t.read_failures.push_back(true),
                "both" => t.ready_failures = 2,
                _ => unreachable!(),
            }
        }
        let failure = if stage == "reconcile" {
            CandidateFailure::SelectionReconcile
        } else {
            CandidateFailure::RunOrReady
        };
        assert_eq!(
            owner.execute(RuntimeCommand::ApplySaved, |_| {}),
            Err(if stage == "both" {
                RuntimeError::RollbackFailed(failure)
            } else {
                RuntimeError::CandidateRolledBack(failure)
            })
        );
        if stage == "both" {
            assert!(owner.snapshot().unwrap().runtime.applied_version.is_none());
        } else {
            assert_eq!(owner.snapshot().unwrap().runtime.applied_version, old);
        }
        assert_eq!(trace.lock().unwrap().checks, 3);
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
// 保护 manifest 最终提交失败时真实 Ready/new applied 与 old last_successful 同时可见。
#[test]
fn ready_candidate_manifest_failure_preserves_old_recovery() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let old = owner.store().unwrap().read_manifest().unwrap();
    let mut state = store.load().unwrap();
    state.profile.reject_quic = !state.profile.reject_quic;
    let new = store.commit(&state).unwrap();
    owner.records.as_mut().unwrap().fail_manifest_at = Some(3);
    assert_eq!(
        owner.execute(RuntimeCommand::ApplySaved, |_| {}),
        Err(RuntimeError::RecoveryRecordFailed)
    );
    let s = owner.snapshot().unwrap();
    assert_eq!(
        s.runtime.status,
        super::super::runtime_snapshot::RuntimeStatus::Ready
    );
    assert_eq!(s.runtime.applied_version, Some(new.config_version()));
    assert_eq!(s.runtime.last_successful_version, Some(old.config.clone()));
    assert_eq!(
        owner.store().unwrap().read_manifest().unwrap().plan,
        old.plan
    );
    assert!(trace.lock().unwrap().active.is_some());
    assert!(owner.load_recovery(&new).is_ok());
    owner.records.as_mut().unwrap().fail_manifest_at = None;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
// 保护复制确实在stop/reap之后run之前，candidate写入不改变immutable rollback。
#[test]
fn cache_handoff_snapshot_and_copy_failure() {
    for fail in [false, true] {
        let (mut owner, trace, _, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let cache = owner
            .resources(&owner.snapshots.snapshot().unwrap().state_epoch)
            .unwrap()
            .cache
            .path()
            .to_owned();
        std::fs::write(&cache, b"open-writer").unwrap();
        trace.lock().unwrap().cache_on_stop = Some(b"closed-old-writer".to_vec());
        owner.records.as_mut().unwrap().fail_copy = fail;
        let result = owner.execute(RuntimeCommand::Restart, |_| {});
        if fail {
            assert_eq!(result, Err(RuntimeError::CacheSnapshotFailed));
            assert_eq!(trace.lock().unwrap().next, 1);
            assert!(trace.lock().unwrap().active.is_none());
        } else {
            result.unwrap();
            let m = owner.store().unwrap().read_manifest().unwrap();
            let snapshot = m.cache.unwrap();
            assert_eq!(
                owner.store().unwrap().artifact(&snapshot).unwrap(),
                b"closed-old-writer"
            );
            assert_ne!(std::fs::read(cache).unwrap(), b"closed-old-writer");
            let events = trace.lock().unwrap().events.clone();
            let reap = events.iter().position(|v| v == "reap").unwrap();
            let run = events.iter().rposition(|v| v == "run").unwrap();
            let copy = events
                .iter()
                .position(|v| v == "snapshot-confirmed-before-run")
                .unwrap();
            assert!(reap < copy && copy < run);
        }
        owner.records.as_mut().unwrap().fail_copy = false;
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
// 保护 corrupt / epoch / kernel / generation / resource 缺失拒绝恢复，且保留磁盘事实。
#[test]
fn incompatible_or_incomplete_recovery_is_unavailable() {
    for kind in [
        "epoch",
        "kernel",
        "generation",
        "resource",
        "cache",
        "digest",
        "unknown",
        "unknown-version",
        "plan-unknown",
    ] {
        let (mut owner, trace, _, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        let path = root.join("runtime/last-applied.json");
        let mut m = owner.store().unwrap().read_manifest().unwrap();
        let reason = match kind {
            "epoch" => {
                m.state_epoch = crate::domain::StateEpoch::fresh().unwrap();
                m.config.0.epoch = m.state_epoch.clone();
                m.selection_at_apply.0.epoch = m.state_epoch.clone();
                m.plan_selection.0.epoch = m.state_epoch.clone();
                m.confirmed_selection.version.0.epoch = m.state_epoch.clone();
                RecoveryError::EpochMismatch
            }
            "kernel" => {
                m.kernel_digest = "wrong".into();
                RecoveryError::KernelMismatch
            }
            "generation" => {
                m.cache_generation = "other".into();
                RecoveryError::CacheGenerationMismatch
            }
            "resource" => {
                m.resources.push(ArtifactRef {
                    path: "runtime/missing".into(),
                    digest: "wrong".into(),
                });
                RecoveryError::ResourceMissing
            }
            "cache" => {
                std::fs::remove_file(root.join(&m.cache.as_ref().unwrap().path)).unwrap();
                RecoveryError::ResourceMissing
            }
            "digest" => {
                std::fs::write(root.join(&m.plan.path), b"tampered").unwrap();
                RecoveryError::DigestMismatch
            }
            "plan-unknown" => {
                let mut v: serde_json::Value =
                    serde_json::from_slice(&owner.store().unwrap().artifact(&m.plan).unwrap())
                        .unwrap();
                v["arbitrary"] = "field".into();
                m.plan = owner
                    .store()
                    .unwrap()
                    .save_plan(&serde_json::to_vec(&v).unwrap())
                    .unwrap();
                RecoveryError::Corrupt
            }
            _ => RecoveryError::Corrupt,
        };
        owner.store().unwrap().commit(&m).unwrap();
        if kind.starts_with("unknown") {
            let mut v = serde_json::to_value(m).unwrap();
            if kind == "unknown" {
                v["extra"] = true.into();
            } else {
                v["config"]["extra"] = true.into();
            }
            std::fs::write(&path, serde_json::to_vec(&v).unwrap()).unwrap();
        }
        assert_eq!(
            owner.execute(RuntimeCommand::RestoreLastSuccessful, |_| {}),
            Err(RuntimeError::RecoveryUnavailable(reason)),
            "{kind}"
        );
        assert!(path.exists());
        assert_eq!(trace.lock().unwrap().next, 1);
        std::fs::remove_dir_all(root).unwrap();
    }
}
// P4-03 新契约：配置变化后旧选择请求拒绝，不能覆盖新配置的成员或选择事实。
#[test]
fn selection_rejects_unapplied_config_without_controller_write() {
    let (mut owner, _, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let before = owner.snapshot().unwrap();
    let mut state = store.load().unwrap();
    state.profile.reject_quic = !state.profile.reject_quic;
    let saved = store.commit(&state).unwrap();
    assert_eq!(
        owner.select_manual(request(&owner, "b")),
        Err(SelectionError::ConfigChanged)
    );
    assert_eq!(
        store.load().unwrap().selection_revision,
        state.selection_revision
    );
    let after = owner.snapshot().unwrap();
    assert_eq!(
        after.runtime.applied_version,
        before.runtime.applied_version
    );
    assert_eq!(after.runtime.saved_version, saved.config_version());
    assert_eq!(
        after.runtime.last_successful_version,
        before.runtime.last_successful_version
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
// 保护显式ApplySaved修复tampered旧plan前留存原证据，而不是覆盖后宣称没有旧记录。
#[test]
fn apply_saved_preserves_corrupt_recovery_evidence() {
    let (mut owner, _, _, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let old = owner.store().unwrap().read_manifest().unwrap();
    std::fs::write(root.join(&old.plan.path), b"corrupt-plan-bytes").unwrap();
    assert_eq!(
        owner.snapshot().unwrap().recovery,
        RecoveryAvailability::Unavailable(RecoveryError::DigestMismatch)
    );
    owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
    assert_eq!(
        owner.snapshot().unwrap().recovery,
        RecoveryAvailability::Available
    );
    assert_eq!(
        std::fs::read(root.join(format!(
            "runtime/corrupt-plan-{}.json",
            digest(b"corrupt-plan-bytes")
        )))
        .unwrap(),
        b"corrupt-plan-bytes"
    );
    assert!(std::fs::read_dir(root.join("runtime")).unwrap().any(|p| {
        p.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("corrupt-manifest-")
    }));
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 保护跨 owner 恢复磁盘 pending：只 GET，actual=requested/old 才解决；third/unknown 不 Ready 不 PUT。
#[test]
fn restart_pending_reconciles_by_actual_for_both_recovery_entries() {
    for command in [
        RuntimeCommand::ApplySaved,
        RuntimeCommand::RestoreLastSuccessful,
    ] {
        for actual in ["pending", "old", "third", "unknown", "timeout"] {
            let (mut owner, trace, store, root) = manual_fixture();
            owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
            let pool = owner
                .active
                .as_ref()
                .unwrap()
                .plan
                .artifact_index()
                .unwrap()
                .pools[&PoolId("manual".into())]
                .clone();
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            let state = store.load().unwrap();
            let pending = SelectionService::new((*owner.snapshots).clone())
                .begin_manual_pending(
                    state.selection_version(),
                    PoolId("manual".into()),
                    NodeId("b".into()),
                )
                .unwrap();
            let mut restarted = ManualRuntime::new(
                Mock(trace.clone()),
                owner.snapshots.clone(),
                root.clone(),
                true,
            );
            drop(owner);
            let snapshot = restarted.snapshot().unwrap();
            assert_eq!(snapshot.runtime.applied_version, None);
            assert_eq!(
                snapshot.pending_selections,
                vec![PendingSelection {
                    pool: PoolId("manual".into()),
                    requested: NodeId("b".into())
                }]
            );
            let writes = trace.lock().unwrap().writes;
            let actual_tag = match actual {
                "pending" => pool.members[&NodeId("b".into())].clone(),
                "old" | "timeout" => pool.members[&NodeId("a".into())].clone(),
                "third" => pool.members[&NodeId("c".into())].clone(),
                _ => "unknown-runtime-tag".into(),
            };
            trace
                .lock()
                .unwrap()
                .cache_choices
                .insert(pool.runtime_tag.clone(), actual_tag);
            if actual == "timeout" {
                trace.lock().unwrap().read_failures.push_back(true);
            }
            let result = restarted.execute(command, |s| {
                assert_ne!(
                    s.runtime.status,
                    super::super::runtime_snapshot::RuntimeStatus::Ready
                );
                assert!(s.pending_selection.is_some());
            });
            assert_eq!(
                trace.lock().unwrap().writes,
                writes,
                "pending 不得重发或强制旧值"
            );
            assert_eq!(
                trace.lock().unwrap().next,
                2,
                "未解释 pending 不自动回退另一个实例"
            );
            let after = store.load().unwrap();
            assert_eq!(after.config_version(), state.config_version());
            if matches!(actual, "pending" | "old") {
                assert_eq!(result, Ok(RuntimeResult::Started));
                assert_eq!(after.selection_revision, pending.version.0.revision + 1);
                assert!(restarted.snapshot().unwrap().pending_selection.is_none());
                assert_eq!(
                    restarted.snapshot().unwrap().confirmed_selection_version,
                    Some(after.selection_version())
                );
                let expected = if actual == "pending" { "b" } else { "a" };
                let manifest = restarted.store().unwrap().read_manifest().unwrap();
                assert_eq!(
                    manifest.confirmed_selection.version,
                    after.selection_version()
                );
                assert_eq!(
                    manifest.confirmed_selection.nodes[&PoolId("manual".into())].0,
                    expected
                );
                assert_eq!(
                    restarted.snapshot().unwrap().runtime.applied_version,
                    Some(state.config_version())
                );
                restarted.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            } else {
                assert_eq!(result, Err(RuntimeError::SelectionPending));
                assert_eq!(after.selection_version(), pending.version);
                let snapshot = restarted.snapshot().unwrap();
                assert!(snapshot.pending_selection.is_some());
                assert_eq!(snapshot.runtime.applied_version, None);
                assert!(trace.lock().unwrap().active.is_none());
                assert_eq!(
                    restarted
                        .store()
                        .unwrap()
                        .read_manifest()
                        .unwrap()
                        .confirmed_selection
                        .version,
                    state.selection_version()
                );
            }
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}

// 保护 pending 原子保存失败不碰 controller，显式重连解决后才重新允许选择。
#[test]
fn selection_pending_begin_failure_and_explicit_reconcile() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let before = owner.snapshot().unwrap();
    let req = request(&owner, "b");
    std::fs::create_dir(root.join("state.tmp")).unwrap();
    let writes = trace.lock().unwrap().writes;
    assert_eq!(
        owner.select_manual(req.clone()),
        Err(SelectionError::PendingSaveFailed)
    );
    assert_eq!(trace.lock().unwrap().writes, writes);
    assert!(owner.snapshot().unwrap().pending_selection.is_none());
    std::fs::remove_dir(root.join("state.tmp")).unwrap();
    trace.lock().unwrap().mismatch_reads.push_back(true);
    assert_eq!(
        owner.select_manual(req),
        Err(SelectionError::ControllerReadBack)
    );
    assert_eq!(store.load().unwrap().selection_revision, 1);
    let writes = trace.lock().unwrap().writes;
    assert_eq!(
        owner.execute(RuntimeCommand::ReconcileSelection, |_| {}),
        Ok(RuntimeResult::Refreshed)
    );
    assert_eq!(trace.lock().unwrap().writes, writes);
    let resolved = owner.snapshot().unwrap();
    assert!(resolved.pending_selection.is_none());
    assert_eq!(resolved.confirmed_selection_version.unwrap().0.revision, 2);
    assert_eq!(
        resolved.runtime.applied_version,
        before.runtime.applied_version
    );
    owner.select_manual(request(&owner, "a")).unwrap();
    assert_eq!(store.load().unwrap().selection_revision, 4);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 保护恢复旧plan时当前confirmed不在旧artifact：default不能冒充旧值清pending，actual=requested仍可确认。
#[test]
fn restore_pending_never_guesses_old_confirmed_from_default() {
    for actual_requested in [false, true] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let old_pool = owner
            .active
            .as_ref()
            .unwrap()
            .plan
            .artifact_index()
            .unwrap()
            .pools[&PoolId("manual".into())]
            .clone();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        let mut next = store.load().unwrap();
        let mut added = next.nodes[0].clone();
        added.id = NodeId("new-node".into());
        next.nodes.push(added);
        if let SelectionPolicy::Manual {
            selected_node_id, ..
        } = &mut next
            .pools
            .iter_mut()
            .find(|p| p.id.0 == "manual")
            .unwrap()
            .selection
        {
            *selected_node_id = Some(NodeId("new-node".into()));
        }
        let saved = store.commit(&next).unwrap();
        let pending = SelectionService::new((*owner.snapshots).clone())
            .begin_manual_pending(
                saved.selection_version(),
                PoolId("manual".into()),
                NodeId("b".into()),
            )
            .unwrap();
        trace.lock().unwrap().cache_choices.insert(
            old_pool.runtime_tag.clone(),
            old_pool.members[&NodeId(if actual_requested { "b" } else { "a" }.into())].clone(),
        );
        let writes = trace.lock().unwrap().writes;
        let mut restarted = ManualRuntime::new(
            Mock(trace.clone()),
            owner.snapshots.clone(),
            root.clone(),
            true,
        );
        let result = restarted.execute(RuntimeCommand::RestoreLastSuccessful, |_| {});
        assert_eq!(trace.lock().unwrap().writes, writes);
        assert_eq!(
            store.load().unwrap().config_version(),
            saved.config_version()
        );
        if actual_requested {
            assert_eq!(result, Ok(RuntimeResult::Started));
            assert!(restarted.snapshot().unwrap().pending_selection.is_none());
            assert_eq!(
                store.load().unwrap().selection_revision,
                pending.version.0.revision + 1
            );
            assert_eq!(
                restarted
                    .store()
                    .unwrap()
                    .read_manifest()
                    .unwrap()
                    .confirmed_selection
                    .version,
                store.load().unwrap().selection_version()
            );
            restarted.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        } else {
            assert_eq!(result, Err(RuntimeError::SelectionPending));
            assert_eq!(store.load().unwrap().selection_version(), pending.version);
            assert!(restarted.snapshot().unwrap().pending_selection.is_some());
            assert_eq!(restarted.snapshot().unwrap().runtime.applied_version, None);
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

fn add_pending_pool(store: &JsonStateStore, id: &str) -> AppState {
    let mut state = store.load().unwrap();
    let mut pool = state
        .pools
        .iter()
        .find(|p| p.id.0 == "manual")
        .unwrap()
        .clone();
    pool.id = PoolId(id.into());
    pool.name = id.into();
    pool.selection = SelectionPolicy::Manual {
        selected_node_id: Some(NodeId("a".into())),
        pending_node_id: None,
    };
    state.pools.push(pool);
    let saved = store.commit(&state).unwrap();
    let service = SelectionService::new(SnapshotService::new(
        store.clone(),
        StateAccessGate::default(),
    ));
    service
        .begin_manual_pending(
            saved.selection_version(),
            PoolId(id.into()),
            NodeId("b".into()),
        )
        .unwrap();
    store.load().unwrap()
}

// 保护 Host 可达路径：v12 Ready但manifest失败，新pool不确定；重建owner恢复v11不能忽略pending。
#[test]
fn pending_uncovered_restore_rejects_manifest_failure_new_pool_before_prepare() {
    for crash in [false, true] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let mut next = store.load().unwrap();
        let mut new_pool = next
            .pools
            .iter()
            .find(|p| p.id.0 == "manual")
            .unwrap()
            .clone();
        new_pool.id = PoolId("new-manual".into());
        next.pools.push(new_pool);
        let saved = store.commit(&next).unwrap();
        assert_eq!(saved.config_revision, 12);
        let records = owner.records.as_mut().unwrap();
        records.fail_manifest_at = Some(records.commits.get() + 2);
        assert_eq!(
            owner.execute(RuntimeCommand::ApplySaved, |_| {}),
            Err(RuntimeError::RecoveryRecordFailed)
        );
        let ready = owner.snapshot().unwrap();
        assert_eq!(
            ready.runtime.applied_version.as_ref().unwrap().0.revision,
            12
        );
        assert_eq!(
            ready
                .runtime
                .last_successful_version
                .as_ref()
                .unwrap()
                .0
                .revision,
            11
        );
        trace.lock().unwrap().read_failures.push_back(true);
        assert_eq!(
            owner.select_manual(ManualSelectionRequest {
                config: ready.runtime.applied_version.clone().unwrap(),
                instance: ready.runtime.instance_id.unwrap(),
                expected: ready.confirmed_selection_version.unwrap(),
                pool: PoolId("new-manual".into()),
                node: NodeId("b".into()),
            }),
            Err(SelectionError::ControllerReadBack)
        );
        let pending_state = store.load().unwrap();
        assert!(owner.snapshot().unwrap().pending_selection.is_some());
        if crash {
            // 只模拟进程结束；无真实child/OS资源操作，重建owner从同一磁盘state读取。
            trace.lock().unwrap().active = None;
            owner = ManualRuntime::new(
                Mock(trace.clone()),
                owner.snapshots.clone(),
                root.clone(),
                true,
            );
        }
        let before = owner.snapshot().unwrap();
        let counters = {
            let t = trace.lock().unwrap();
            (t.checks, t.stops, t.next, t.writes, t.reads, t.active)
        };
        assert_eq!(
            owner.execute(RuntimeCommand::RestoreLastSuccessful, |_| panic!(
                "preflight不能发布candidate"
            )),
            Err(RuntimeError::SelectionPending)
        );
        assert_eq!(store.load().unwrap(), pending_state);
        let after = owner.snapshot().unwrap();
        assert_eq!(after.pending_selections, before.pending_selections);
        assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
        assert_eq!(
            after.runtime.applied_version,
            before.runtime.applied_version
        );
        assert_eq!(after.endpoints, before.endpoints);
        if crash {
            assert_ne!(
                after.runtime.status,
                super::super::runtime_snapshot::RuntimeStatus::Ready
            );
        }
        let t = trace.lock().unwrap();
        assert_eq!(
            (t.checks, t.stops, t.next, t.writes, t.reads, t.active),
            counters,
            "未prepare/check/stop/run/GET/PUT旧或新child"
        );
        drop(t);
        assert_eq!(
            owner
                .store()
                .unwrap()
                .read_manifest()
                .unwrap()
                .config
                .0
                .revision,
            11
        );
        if !crash {
            owner.records.as_mut().unwrap().fail_manifest_at = None;
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

// 保护全量覆盖与显式重连：部分覆盖不得先CAS一个pending而忽略另一个；ApplySaved可用完整新plan解决。
#[test]
fn pending_uncovered_partial_coverage_blocks_reconcile_and_restore_but_apply_saved_resolves_all() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let old_pool = owner
        .active
        .as_ref()
        .unwrap()
        .plan
        .artifact_index()
        .unwrap()
        .pools[&PoolId("manual".into())]
        .clone();
    let saved = add_pending_pool(&store, "new-manual");
    SelectionService::new((*owner.snapshots).clone())
        .begin_manual_pending(
            saved.selection_version(),
            PoolId("manual".into()),
            NodeId("b".into()),
        )
        .unwrap();
    trace.lock().unwrap().selectors.insert(
        old_pool.runtime_tag.clone(),
        old_pool.members[&NodeId("b".into())].clone(),
    );
    let pending_state = store.load().unwrap();
    let before = owner.snapshot().unwrap();
    assert_eq!(before.pending_selections.len(), 2);
    let counters = {
        let t = trace.lock().unwrap();
        (t.checks, t.stops, t.next, t.writes, t.reads)
    };
    for command in [
        RuntimeCommand::ReconcileSelection,
        RuntimeCommand::RestoreLastSuccessful,
    ] {
        assert_eq!(
            owner.execute(command, |_| {}),
            Err(RuntimeError::SelectionPending)
        );
        assert_eq!(
            store.load().unwrap(),
            pending_state,
            "覆盖不完整时两个pending均不CAS/clear"
        );
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.pending_selections.len(), 2);
        assert_eq!(snapshot.runtime.instance_id, before.runtime.instance_id);
        assert_eq!(snapshot.endpoints, before.endpoints);
        let t = trace.lock().unwrap();
        assert_eq!((t.checks, t.stops, t.next, t.writes, t.reads), counters);
    }
    // 新plan包含两个业务pool，其actual按stable artifact tag对应b；不依赖显示名或manifest默认。
    trace.lock().unwrap().cache_choices.insert(
        old_pool.runtime_tag.clone(),
        old_pool.members[&NodeId("b".into())].clone(),
    );
    let projection = project_selected_runtime(&pending_state).unwrap();
    let default = OutboundId::from_route_target(&projection.projected_default_target).unwrap();
    let plan = SingBoxCompiler
        .compile_product(ProductCompileRequest {
            state: &pending_state,
            runtime_intent: &projection.runtime_intent,
            default_outbound: &default,
            resources: &owner.resources(&pending_state.state_epoch).unwrap(),
        })
        .unwrap();
    let new_pool = &plan.artifact_index().unwrap().pools[&PoolId("new-manual".into())];
    trace.lock().unwrap().cache_choices.insert(
        new_pool.runtime_tag.clone(),
        new_pool.members[&NodeId("b".into())].clone(),
    );
    let writes = trace.lock().unwrap().writes;
    owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
    let resolved = store.load().unwrap();
    assert_eq!(
        resolved.selection_revision,
        pending_state.selection_revision + 2
    );
    assert_eq!(resolved.config_version(), pending_state.config_version());
    assert!(owner.snapshot().unwrap().pending_selections.is_empty());
    assert_eq!(trace.lock().unwrap().writes, writes);
    assert_eq!(
        owner.snapshot().unwrap().confirmed_selection_version,
        Some(resolved.selection_version())
    );
    for id in ["manual", "new-manual"] {
        assert_eq!(
            owner.active.as_ref().unwrap().selection.nodes[&PoolId(id.into())],
            NodeId("b".into())
        );
    }
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 保护Restart裁剪掉pending pool时提前拒绝，以及循环期间新出现pending的最终gate。
#[test]
fn pending_uncovered_restart_preflight_and_latest_gate_preserve_business_intent() {
    for during_read in [false, true] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let before = owner.snapshot().unwrap();
        let mut candidate = store.load().unwrap();
        let mut pool = candidate
            .pools
            .iter()
            .find(|p| p.id.0 == "manual")
            .unwrap()
            .clone();
        pool.id = PoolId("late-pending".into());
        pool.enabled = false;
        pool.selection = SelectionPolicy::Manual {
            selected_node_id: Some(NodeId("a".into())),
            pending_node_id: Some(NodeId("b".into())),
        };
        candidate.pools.push(pool);
        let (command, expected_reads) = if during_read {
            trace.lock().unwrap().business_on_read = Some(candidate);
            (RuntimeCommand::ReconcileSelection, None)
        } else {
            store.commit(&candidate).unwrap();
            (RuntimeCommand::Restart, Some(trace.lock().unwrap().reads))
        };
        let counters = {
            let t = trace.lock().unwrap();
            (t.checks, t.stops, t.next, t.writes, t.active)
        };
        assert_eq!(
            owner.execute(command, |_| panic!("不发布新Ready")),
            Err(RuntimeError::SelectionPending)
        );
        let after = owner.snapshot().unwrap();
        assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
        assert_eq!(after.endpoints, before.endpoints);
        assert_eq!(
            after.runtime.applied_version,
            before.runtime.applied_version
        );
        assert_eq!(
            after.pending_selections,
            vec![PendingSelection {
                pool: PoolId("late-pending".into()),
                requested: NodeId("b".into())
            }]
        );
        assert_eq!(store.load().unwrap().selection_revision, 1);
        let t = trace.lock().unwrap();
        assert_eq!((t.checks, t.stops, t.next, t.writes, t.active), counters);
        if let Some(reads) = expected_reads {
            assert_eq!(t.reads, reads);
        }
        drop(t);
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

// 保护prepare期间业务出现未覆盖pending：在stop/reap之前取消候选，已有child仍由owner持有。
#[test]
fn pending_uncovered_created_during_prepare_cancels_candidate_before_old_stop() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let before = owner.snapshot().unwrap();
    let mut candidate = store.load().unwrap();
    let mut pool = candidate
        .pools
        .iter()
        .find(|p| p.id.0 == "manual")
        .unwrap()
        .clone();
    pool.id = PoolId("during-prepare".into());
    pool.selection = SelectionPolicy::Manual {
        selected_node_id: Some(NodeId("a".into())),
        pending_node_id: Some(NodeId("b".into())),
    };
    candidate.pools.push(pool);
    trace.lock().unwrap().business_on_prepare = Some(candidate);
    let stops = trace.lock().unwrap().stops;
    let writes = trace.lock().unwrap().writes;
    assert_eq!(
        owner.execute(RuntimeCommand::Restart, |_| panic!("不能停止旧writer")),
        Err(RuntimeError::SelectionPending)
    );
    let snapshot = owner.snapshot().unwrap();
    assert_eq!(snapshot.runtime.instance_id, before.runtime.instance_id);
    assert_eq!(
        snapshot.runtime.applied_version,
        before.runtime.applied_version
    );
    assert_eq!(snapshot.endpoints, before.endpoints);
    assert_eq!(snapshot.pending_selections.len(), 1);
    assert_eq!(store.load().unwrap().selection_revision, 1);
    assert_eq!(trace.lock().unwrap().stops, stops);
    assert_eq!(trace.lock().unwrap().writes, writes);
    assert_eq!(trace.lock().unwrap().next, 1);
    assert!(!owner.sidecar.snapshot().has_candidate_config);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 保护最终gate与已发生CAS：先确认一个pending后出现未覆盖pending，不能Ready/回退/丢失任一业务结果。
#[test]
fn pending_uncovered_after_confirm_cas_blocks_candidate_ready_without_losing_other_intent() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let pool = owner
        .active
        .as_ref()
        .unwrap()
        .plan
        .artifact_index()
        .unwrap()
        .pools[&PoolId("manual".into())]
        .clone();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let state = store.load().unwrap();
    SelectionService::new((*owner.snapshots).clone())
        .begin_manual_pending(
            state.selection_version(),
            PoolId("manual".into()),
            NodeId("b".into()),
        )
        .unwrap();
    {
        let mut t = trace.lock().unwrap();
        t.cache_choices.insert(
            pool.runtime_tag.clone(),
            pool.members[&NodeId("b".into())].clone(),
        );
        t.pending_pool_on_read = Some(("after-confirm".into(), t.reads + 2));
    }
    assert_eq!(
        owner.execute(RuntimeCommand::ApplySaved, |s| assert_ne!(
            s.runtime.status,
            super::super::runtime_snapshot::RuntimeStatus::Ready
        )),
        Err(RuntimeError::SelectionPending)
    );
    let latest = store.load().unwrap();
    // begin + confirm + 新pool的confirmed事实 + 新pool pending，四次业务选择变更。
    assert_eq!(latest.selection_revision, state.selection_revision + 4);
    assert!(
        matches!(&latest.pools.iter().find(|p| p.id.0 == "manual").unwrap().selection,
        SelectionPolicy::Manual { selected_node_id: Some(node), pending_node_id: None } if node.0 == "b")
    );
    assert_eq!(
        owner.snapshot().unwrap().pending_selections,
        vec![PendingSelection {
            pool: PoolId("after-confirm".into()),
            requested: NodeId("b".into())
        }]
    );
    assert_eq!(owner.snapshot().unwrap().runtime.applied_version, None);
    assert!(trace.lock().unwrap().active.is_none());
    assert_eq!(trace.lock().unwrap().next, 2, "不回退另一实例来消除pending");
    assert_eq!(trace.lock().unwrap().writes, 0);
    assert_eq!(
        owner
            .store()
            .unwrap()
            .read_manifest()
            .unwrap()
            .confirmed_selection
            .version,
        state.selection_version()
    );
    std::fs::remove_dir_all(root).unwrap();
}

// 构造 Host P1：成功版11/cache=a；实际12 Ready但记录失败；PUT b成功、GET失败后pending持久化。
fn newer_child_with_unconfirmed_switch() -> (
    ManualRuntime<Mock>,
    Arc<Mutex<Trace>>,
    JsonStateStore,
    PathBuf,
) {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let pool = owner
        .active
        .as_ref()
        .unwrap()
        .plan
        .artifact_index()
        .unwrap()
        .pools[&PoolId("manual".into())]
        .clone();
    {
        let mut t = trace.lock().unwrap();
        t.selector_cache = Some(pool.runtime_tag.clone());
        std::fs::write(
            t.active_cache.as_ref().unwrap(),
            &pool.members[&NodeId("a".into())],
        )
        .unwrap();
    }
    let mut next = store.load().unwrap();
    next.profile.reject_quic = !next.profile.reject_quic;
    let saved = store.commit(&next).unwrap();
    assert_eq!(saved.config_revision, 12);
    let records = owner.records.as_mut().unwrap();
    records.fail_manifest_at = Some(records.commits.get() + 2);
    assert_eq!(
        owner.execute(RuntimeCommand::ApplySaved, |_| {}),
        Err(RuntimeError::RecoveryRecordFailed)
    );
    let before = owner.snapshot().unwrap();
    assert_eq!(before.runtime.applied_version.unwrap().0.revision, 12);
    assert_eq!(
        before.runtime.last_successful_version.unwrap().0.revision,
        11
    );
    trace.lock().unwrap().read_failures.push_back(true);
    assert_eq!(
        owner.select_manual(request(&owner, "b")),
        Err(SelectionError::ControllerReadBack)
    );
    let state = store.load().unwrap();
    assert_eq!((state.config_revision, state.selection_revision), (12, 1));
    let m = owner.store().unwrap().read_manifest().unwrap();
    assert_eq!(
        (
            m.config.0.revision,
            m.confirmed_selection.version.0.revision
        ),
        (11, 0)
    );
    let t = trace.lock().unwrap();
    assert_eq!(
        t.selectors[&pool.runtime_tag],
        pool.members[&NodeId("b".into())]
    );
    assert_eq!(
        std::fs::read_to_string(t.active_cache.as_ref().unwrap()).unwrap(),
        pool.members[&NodeId("b".into())]
    );
    assert_eq!(
        m.cache.as_ref().unwrap().digest,
        digest(pool.members[&NodeId("a".into())].as_bytes())
    );
    drop(t);
    (owner, trace, store, root)
}

// Run/Ready失败不能借另一plan/cache的GET=a消除已实际PUT=b的意图。
#[test]
fn host_rework_fallback_preserves_pending_after_newer_child_manifest_failure() {
    for ready_failure in [false, true] {
        let (mut owner, trace, store, root) = newer_child_with_unconfirmed_switch();
        let pending = store.load().unwrap();
        let manifest =
            serde_json::to_value(owner.store().unwrap().read_manifest().unwrap()).unwrap();
        let (runs, writes, cache, bytes) = {
            let mut t = trace.lock().unwrap();
            if ready_failure {
                t.ready_failures = 1;
            } else {
                t.run_failures = 1;
            }
            let cache = t.active_cache.clone().unwrap();
            (
                t.next,
                t.writes,
                cache.clone(),
                std::fs::read(cache).unwrap(),
            )
        };
        assert_eq!(
            owner.execute(RuntimeCommand::ApplySaved, |s| assert_ne!(
                s.runtime.status,
                super::super::runtime_snapshot::RuntimeStatus::Ready
            )),
            Err(RuntimeError::SelectionPending)
        );
        assert_eq!(store.load().unwrap(), pending);
        assert_eq!(
            serde_json::to_value(owner.store().unwrap().read_manifest().unwrap()).unwrap(),
            manifest
        );
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.runtime.applied_version, None);
        assert_eq!(snapshot.confirmed_selection_version, None);
        assert_eq!(snapshot.pending_selections.len(), 1);
        assert!(snapshot.endpoints.is_none());
        let t = trace.lock().unwrap();
        assert_eq!(
            t.next,
            runs + u64::from(ready_failure),
            "不启动旧cache rollback child"
        );
        assert_eq!(t.writes, writes);
        assert!(t.active.is_none());
        assert!(!owner.sidecar.snapshot().has_candidate_config);
        assert_eq!(
            std::fs::read(cache).unwrap(),
            bytes,
            "不能用旧cache a覆盖当前cache b"
        );
        drop(t);
        std::fs::remove_dir_all(root).unwrap();
    }
}

// 显式Restore也不能用旧记录覆盖来源不同的当前/关闭cache；先核对可信当前child或ApplySaved。
#[test]
fn host_rework_explicit_restore_checks_pending_cache_and_plan_provenance() {
    for (restarted, unchanged_cache) in [(false, false), (false, true), (true, false)] {
        let (mut owner, trace, store, root) = newer_child_with_unconfirmed_switch();
        if unchanged_cache {
            // opaque cache尚未反映PUT：即使字节等于旧manifest，也必须拒绝不同active plan。
            let t = trace.lock().unwrap();
            let pool = &owner
                .active
                .as_ref()
                .unwrap()
                .plan
                .artifact_index()
                .unwrap()
                .pools[&PoolId("manual".into())];
            std::fs::write(
                t.active_cache.as_ref().unwrap(),
                &pool.members[&NodeId("a".into())],
            )
            .unwrap();
        }
        if restarted {
            // Mock owner重建，无真实进程/主机操作；live cache仍为b，manifest cache为a。
            trace.lock().unwrap().active = None;
            owner = ManualRuntime::new(
                Mock(trace.clone()),
                owner.snapshots.clone(),
                root.clone(),
                true,
            );
        }
        let before = owner.snapshot().unwrap();
        let pending = store.load().unwrap();
        let manifest =
            serde_json::to_value(owner.store().unwrap().read_manifest().unwrap()).unwrap();
        let (counters, cache, bytes) = {
            let t = trace.lock().unwrap();
            let path = t.active_cache.clone().unwrap();
            (
                (t.checks, t.stops, t.next, t.writes, t.reads, t.active),
                path.clone(),
                std::fs::read(path).unwrap(),
            )
        };
        assert_eq!(
            owner.execute(RuntimeCommand::RestoreLastSuccessful, |_| panic!(
                "不得prepare/stop旧child"
            )),
            Err(RuntimeError::SelectionPending)
        );
        assert_eq!(store.load().unwrap(), pending);
        assert_eq!(
            serde_json::to_value(owner.store().unwrap().read_manifest().unwrap()).unwrap(),
            manifest
        );
        let after = owner.snapshot().unwrap();
        assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
        assert_eq!(
            after.runtime.applied_version,
            before.runtime.applied_version
        );
        assert_eq!(after.endpoints, before.endpoints);
        assert_eq!(after.pending_selections, before.pending_selections);
        let t = trace.lock().unwrap();
        assert_eq!(
            (t.checks, t.stops, t.next, t.writes, t.reads, t.active),
            counters
        );
        assert_eq!(std::fs::read(cache).unwrap(), bytes);
        drop(t);
        if restarted {
            owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
        } else {
            // 核对12 actual=b可以持久确认，但11 manifest不能冒充12的记录成功。
            assert_eq!(
                owner.execute(RuntimeCommand::ReconcileSelection, |_| {}),
                Err(RuntimeError::RecoveryRecordFailed)
            );
        }
        let confirmed = store.load().unwrap();
        assert_eq!(
            (confirmed.config_revision, confirmed.selection_revision),
            (12, 2)
        );
        assert!(owner.snapshot().unwrap().pending_selections.is_empty());
        // 已明确确认b后，Restore允许按该业务事实PUT=b；不再消除未核对意图。
        owner
            .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
            .unwrap();
        assert_eq!(store.load().unwrap(), confirmed);
        assert_eq!(
            owner
                .snapshot()
                .unwrap()
                .runtime
                .applied_version
                .unwrap()
                .0
                .revision,
            if restarted { 12 } else { 11 }
        );
        assert_eq!(
            owner.snapshot().unwrap().confirmed_selection_version,
            Some(confirmed.selection_version())
        );
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

// GET期间同一pool新pending=b与actual=b必须阻止旧confirmed=a的PUT；保留当前child供显式重连。
#[test]
fn host_rework_same_pool_pending_during_get_prevents_stale_put() {
    for command in [
        RuntimeCommand::ReconcileSelection,
        RuntimeCommand::ApplySaved,
        RuntimeCommand::RestoreLastSuccessful,
    ] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let before = owner.snapshot().unwrap();
        let pool = owner
            .active
            .as_ref()
            .unwrap()
            .plan
            .artifact_index()
            .unwrap()
            .pools[&PoolId("manual".into())]
            .clone();
        let mut next = store.load().unwrap();
        if let SelectionPolicy::Manual {
            pending_node_id, ..
        } = &mut next
            .pools
            .iter_mut()
            .find(|p| p.id.0 == "manual")
            .unwrap()
            .selection
        {
            *pending_node_id = Some(NodeId("b".into()));
        }
        let writes = trace.lock().unwrap().writes;
        {
            let mut t = trace.lock().unwrap();
            let tag = pool.members[&NodeId("b".into())].clone();
            t.selectors.insert(pool.runtime_tag.clone(), tag.clone());
            t.cache_choices.insert(pool.runtime_tag.clone(), tag);
            t.business_on_read = Some(next);
        }
        assert_eq!(
            owner.execute(command, |s| assert_ne!(
                s.runtime.status,
                super::super::runtime_snapshot::RuntimeStatus::Ready
            )),
            Err(RuntimeError::SelectionPending)
        );
        let pending = store.load().unwrap();
        assert_eq!(
            (pending.config_revision, pending.selection_revision),
            (11, 1)
        );
        assert_eq!(
            trace.lock().unwrap().writes,
            writes,
            "不能先PUT=a再靠final gate报pending"
        );
        assert_eq!(
            trace.lock().unwrap().selectors[&pool.runtime_tag],
            pool.members[&NodeId("b".into())]
        );
        let after = owner.snapshot().unwrap();
        assert_eq!(
            after.pending_selections,
            vec![PendingSelection {
                pool: PoolId("manual".into()),
                requested: NodeId("b".into())
            }]
        );
        assert!(
            matches!(&pending.pools.iter().find(|p| p.id.0 == "manual").unwrap().selection,
            SelectionPolicy::Manual { selected_node_id: Some(old), pending_node_id: Some(requested) }
                if old.0 == "a" && requested.0 == "b")
        );
        assert_eq!(after.confirmed_selection_version, None);
        assert_eq!(
            owner
                .store()
                .unwrap()
                .read_manifest()
                .unwrap()
                .confirmed_selection
                .version
                .0
                .revision,
            0
        );
        if command == RuntimeCommand::ReconcileSelection {
            assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
            assert_eq!(after.endpoints, before.endpoints);
            owner
                .execute(RuntimeCommand::ReconcileSelection, |_| {})
                .unwrap();
            assert_eq!(store.load().unwrap().selection_revision, 2);
            assert_eq!(trace.lock().unwrap().writes, writes);
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        } else {
            assert_eq!(after.runtime.applied_version, None);
            assert!(trace.lock().unwrap().active.is_none());
            assert_eq!(trace.lock().unwrap().next, 2, "没有rollback误确认");
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

// 校验到PUT之间必须原子排除同gate业务写，并检查epoch替换；不能以复读快照替代锁。
#[test]
fn host_rework_controller_write_gate_excludes_business_mutation_and_checks_epoch() {
    for (epoch_change, manual_write) in [(false, false), (false, true), (true, false)] {
        let (mut owner, trace, store, root) = manual_fixture();
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let pool = owner
            .active
            .as_ref()
            .unwrap()
            .plan
            .artifact_index()
            .unwrap()
            .pools[&PoolId("manual".into())]
            .clone();
        let initial = store.load().unwrap();
        let before = owner.snapshot().unwrap();
        let writes = trace.lock().unwrap().writes;
        {
            let mut t = trace.lock().unwrap();
            t.selectors.insert(
                pool.runtime_tag.clone(),
                pool.members[&NodeId("b".into())].clone(),
            );
            if epoch_change {
                t.replacement_on_read = Some(((*owner.snapshots).clone(), initial.clone()));
            } else {
                t.selection_on_write = Some((*owner.snapshots).clone());
            }
        }
        let result = if manual_write {
            let version = owner.select_manual(request(&owner, "b")).unwrap();
            assert_eq!(version.0.revision, initial.selection_revision + 2);
            Ok(RuntimeResult::Refreshed)
        } else {
            owner.execute(RuntimeCommand::ReconcileSelection, |_| {})
        };
        if epoch_change {
            assert_eq!(result, Err(RuntimeError::SelectionPending));
            assert_ne!(store.load().unwrap().state_epoch, initial.state_epoch);
            assert_eq!(trace.lock().unwrap().writes, writes);
            assert_eq!(
                owner.snapshot().unwrap().runtime.instance_id,
                before.runtime.instance_id
            );
        } else {
            assert_eq!(result, Ok(RuntimeResult::Refreshed));
            assert_eq!(
                trace.lock().unwrap().selection_write_result,
                Some(Err(crate::domain::AppError::new(
                    crate::domain::AppErrorCode::StorageFailed
                )
                .with_detail(crate::domain::ErrorDetail::Busy)))
            );
            let latest = store.load().unwrap();
            if manual_write {
                assert_eq!(latest.config_version(), initial.config_version());
                assert_eq!(latest.selection_revision, initial.selection_revision + 2);
                assert!(
                    matches!(&latest.pools.iter().find(|p| p.id.0 == "manual").unwrap().selection,
                    SelectionPolicy::Manual { selected_node_id: Some(node), pending_node_id: None } if node.0 == "b")
                );
            } else {
                assert_eq!(latest, initial);
            }
            assert_eq!(trace.lock().unwrap().writes, writes + 1);
            // 临界区结束即释放gate，后续正常业务意图可保存。
            SelectionService::new((*owner.snapshots).clone())
                .begin_manual_pending(
                    latest.selection_version(),
                    PoolId("manual".into()),
                    NodeId("c".into()),
                )
                .unwrap();
        }
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

// 第一组确认CAS已提交，第二组GET期间并发版本变化：保留第一组结果及后两组pending，不Ready/回退。
#[test]
fn host_rework_partial_confirm_cas_conflict_retains_committed_result_and_pending() {
    let (mut owner, trace, store, root) = manual_fixture();
    let mut state = store.load().unwrap();
    let mut second = state
        .pools
        .iter()
        .find(|p| p.id.0 == "manual")
        .unwrap()
        .clone();
    second.id = PoolId("second-manual".into());
    state.pools.push(second);
    let saved = store.commit(&state).unwrap();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let index = owner
        .active
        .as_ref()
        .unwrap()
        .plan
        .artifact_index()
        .unwrap()
        .clone();
    let service = SelectionService::new((*owner.snapshots).clone());
    let first = service
        .begin_manual_pending(
            saved.selection_version(),
            PoolId("manual".into()),
            NodeId("b".into()),
        )
        .unwrap();
    service
        .begin_manual_pending(
            first.version,
            PoolId("second-manual".into()),
            NodeId("b".into()),
        )
        .unwrap();
    {
        let mut t = trace.lock().unwrap();
        for id in ["manual", "second-manual"] {
            let pool = &index.pools[&PoolId(id.into())];
            t.cache_choices.insert(
                pool.runtime_tag.clone(),
                pool.members[&NodeId("b".into())].clone(),
            );
        }
        let position = index
            .pools
            .keys()
            .position(|id| id.0 == "second-manual")
            .unwrap()
            + 1;
        t.pending_pool_on_read = Some(("concurrent-manual".into(), t.reads + position));
    }
    let manifest_version = owner
        .store()
        .unwrap()
        .read_manifest()
        .unwrap()
        .confirmed_selection
        .version;
    assert_eq!(
        owner.execute(RuntimeCommand::ApplySaved, |s| assert_ne!(
            s.runtime.status,
            super::super::runtime_snapshot::RuntimeStatus::Ready
        )),
        Err(RuntimeError::SelectionPending)
    );
    let latest = store.load().unwrap();
    assert_eq!(latest.state_epoch, saved.state_epoch);
    assert_eq!(latest.config_revision, saved.config_revision + 1);
    assert_eq!(latest.selection_revision, saved.selection_revision + 5);
    assert!(
        matches!(&latest.pools.iter().find(|p| p.id.0 == "manual").unwrap().selection,
        SelectionPolicy::Manual { selected_node_id: Some(node), pending_node_id: None } if node.0 == "b")
    );
    let pending = owner.snapshot().unwrap().pending_selections;
    assert_eq!(pending.len(), 2);
    assert_eq!(
        pending
            .into_iter()
            .map(|p| (p.pool, p.requested))
            .collect::<BTreeMap<_, _>>(),
        BTreeMap::from([
            (PoolId("concurrent-manual".into()), NodeId("b".into())),
            (PoolId("second-manual".into()), NodeId("b".into()))
        ])
    );
    assert_eq!(owner.snapshot().unwrap().runtime.applied_version, None);
    assert_eq!(
        owner
            .store()
            .unwrap()
            .read_manifest()
            .unwrap()
            .confirmed_selection
            .version,
        manifest_version
    );
    assert_eq!(trace.lock().unwrap().writes, 0);
    assert_eq!(trace.lock().unwrap().next, 2);
    assert!(trace.lock().unwrap().active.is_none());
    assert!(!owner.sidecar.snapshot().has_candidate_config);
    std::fs::remove_dir_all(root).unwrap();
}

// 首次freeze只封住Desktop writer，不能被当成helper授权；重启和丢回复均不得解冻。
#[test]
fn first_bootstrap_freeze_is_durable_idempotent_and_incarnation_bound() {
    let (mut owner, trace, store, root) = fixture();
    let version = owner.handoff_state().unwrap().version();
    let id = StateEpoch::fresh().unwrap();
    let installation = StateEpoch::fresh().unwrap();
    let record = owner
        .freeze_first_bootstrap(id.clone(), installation.clone(), version.clone())
        .unwrap();
    assert_eq!(
        owner
            .freeze_first_bootstrap(id.clone(), installation.clone(), version.clone())
            .unwrap(),
        record
    );
    assert!(owner.execute(RuntimeCommand::Start, |_| {}).is_err());
    assert_eq!(
        owner.select_manual(ManualSelectionRequest {
            config: version.config.clone(),
            instance: InstanceId("unstarted".into()),
            expected: version.selection.clone(),
            pool: PoolId("manual".into()),
            node: NodeId("a".into()),
        }),
        Err(SelectionError::Pending)
    );
    assert_eq!(trace.lock().unwrap().checks, 0);
    assert!(
        owner
            .freeze_first_bootstrap(
                StateEpoch::fresh().unwrap(),
                installation.clone(),
                version.clone()
            )
            .is_err()
    );
    let bytes = std::fs::read(root.join("runtime/owner-transfer.json")).unwrap();
    drop(owner);
    let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
    let mut restarted = ManualRuntime::new(Mock(trace.clone()), snapshots, root.clone(), true);
    assert_eq!(restarted.owner_transfer().unwrap(), Some(record));
    assert!(
        restarted
            .freeze_first_bootstrap(id, installation, version)
            .is_err()
    );
    assert!(restarted.execute(RuntimeCommand::Start, |_| {}).is_err());
    assert_eq!(
        std::fs::read(root.join("runtime/owner-transfer.json")).unwrap(),
        bytes
    );
    assert_eq!(trace.lock().unwrap().checks, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_bootstrap_rejects_stale_version_and_known_history_without_freezing() {
    let (mut owner, trace, _store, root) = fixture();
    let version = owner.handoff_state().unwrap().version();
    let mut stale = version.clone();
    stale.selection.0.revision += 1;
    assert!(
        owner
            .freeze_first_bootstrap(
                StateEpoch::fresh().unwrap(),
                StateEpoch::fresh().unwrap(),
                stale
            )
            .is_err()
    );
    assert_eq!(owner.owner_transfer().unwrap(), None);
    let history = root.join("kernel-cache/old-writer.db");
    std::fs::write(&history, b"preserve unknown writer").unwrap();
    assert!(
        owner
            .freeze_first_bootstrap(
                StateEpoch::fresh().unwrap(),
                StateEpoch::fresh().unwrap(),
                version
            )
            .is_err()
    );
    assert_eq!(owner.owner_transfer().unwrap(), None);
    assert_eq!(std::fs::read(history).unwrap(), b"preserve unknown writer");
    assert_eq!(trace.lock().unwrap().checks, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_bootstrap_does_not_accept_symlink_or_old_incarnation_as_empty_history() {
    // 不把同UID可改的空目录当root可信证明；已知危险路径/旧归属在freeze前拒绝。
    for symlink in [true, false] {
        let (mut owner, _trace, _store, root) = fixture();
        let version = owner.handoff_state().unwrap().version();
        if symlink {
            std::fs::remove_dir(root.join("runtime/configs")).unwrap();
            std::os::unix::fs::symlink(root.join("kernel-cache"), root.join("runtime/configs"))
                .unwrap();
        } else {
            std::fs::write(root.join("runtime/owner-incarnation.json"), b"old owner").unwrap();
        }
        assert!(
            owner
                .freeze_first_bootstrap(
                    StateEpoch::fresh().unwrap(),
                    StateEpoch::fresh().unwrap(),
                    version
                )
                .is_err()
        );
        assert_eq!(owner.owner_transfer().unwrap(), None);
        if symlink {
            assert!(
                std::fs::symlink_metadata(root.join("runtime/configs"))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        } else {
            assert_eq!(
                std::fs::read(root.join("runtime/owner-incarnation.json")).unwrap(),
                b"old owner"
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn bootstrap_target_preflight_failure_never_freezes_desktop() {
    // 未安装/版本不兼容时用户仍可手动启动；首次预检必须在freeze及SourceSession写入前。
    use crate::application::{helper_protocol::Error, helper_transfer::prepare_bootstrap};
    for error in [
        Error::NotInstalled,
        Error::IncompatibleVersion,
        Error::HandoffRequired,
    ] {
        let (mut owner, trace, _store, root) = fixture();
        let version = owner.handoff_state().unwrap().version();
        assert_eq!(
            prepare_bootstrap(&mut owner, StateEpoch::fresh().unwrap(), version, |_| Err(
                error
            )),
            Err(error)
        );
        assert_eq!(owner.owner_transfer().unwrap(), None);
        assert!(!root.join("runtime/owner-incarnation.json").exists());
        assert!(!root.join("runtime/source-session.json").exists());
        assert_eq!(trace.lock().unwrap().checks, 0);
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}

// 保护重复 Start 不重置观测、候选预检失败保留旧观测、替换与未知状态清除旧来源。
#[test]
fn observation_follows_ready_owner_and_never_failed_candidate() {
    let (mut owner, trace, _store, root) = fixture();
    owner.enable_traffic_storage(root.join("traffic"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    trace.lock().unwrap().observation_address = Some(listener.local_addr().unwrap());
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let first = owner.observation().snapshot().identity.unwrap();
    assert!(owner.observation().traffic_status().active);
    assert_eq!(
        Some(first.instance_id.clone()),
        owner.snapshot().unwrap().runtime.instance_id
    );
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    owner.execute(RuntimeCommand::Refresh, |_| {}).unwrap();
    assert_eq!(owner.observation().snapshot().identity, Some(first.clone()));
    trace.lock().unwrap().fail_check = true;
    assert!(owner.execute(RuntimeCommand::Restart, |_| {}).is_err());
    assert_eq!(owner.observation().snapshot().identity, Some(first.clone()));
    assert!(owner.observation().traffic_status().active);
    trace.lock().unwrap().fail_check = false;
    owner.execute(RuntimeCommand::Restart, |_| {}).unwrap();
    assert_ne!(owner.observation().snapshot().identity, Some(first));
    trace.lock().unwrap().crash = true;
    assert!(owner.execute(RuntimeCommand::Refresh, |_| {}).is_err());
    assert!(owner.observation().snapshot().identity.is_none());
    assert!(!owner.observation().traffic_status().active);
    crate::storage::traffic::TrafficWriter::open(&root.join("traffic"), chrono_tz::UTC)
        .unwrap()
        .close()
        .unwrap();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    drop(owner);
    std::fs::remove_dir_all(root).unwrap();
}

// 停止失败时不能继续把仍存活但归属不明的 child 观测当作 Ready 数据。
#[test]
fn observation_stop_failure_clears_source_before_recovery() {
    let (mut owner, trace, _store, root) = fixture();
    owner.enable_traffic_storage(root.join("traffic"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    trace.lock().unwrap().observation_address = Some(listener.local_addr().unwrap());
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    assert!(owner.observation().snapshot().identity.is_some());
    trace.lock().unwrap().fail_stop = true;
    assert!(owner.execute(RuntimeCommand::Stop, |_| {}).is_err());
    assert!(owner.observation().snapshot().identity.is_none());
    assert!(!owner.observation().traffic_status().active);
    crate::storage::traffic::TrafficWriter::open(&root.join("traffic"), chrono_tz::UTC)
        .unwrap()
        .close()
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap().runtime.status,
        crate::application::runtime_snapshot::RuntimeStatus::Recovering
    );
    trace.lock().unwrap().fail_stop = false;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    drop(owner);
    std::fs::remove_dir_all(root).unwrap();
}

// 保护正式 handoff 在冻结后、封存/release 前通过现有 Stop 关闭统计，不新增 Helper/IPC 路径。
#[test]
fn traffic_storage_handoff_closes_before_release() {
    let (mut owner, trace, _store, root) = fixture();
    owner.enable_traffic_storage(root.join("traffic"));
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    trace.lock().unwrap().observation_address = Some(listener.local_addr().unwrap());
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    assert!(owner.observation().traffic_status().active);
    let version = owner.handoff_state().unwrap().version();
    let (ticket, _closed) = owner
        .prepare_handoff(StateEpoch::fresh().unwrap(), version)
        .unwrap();
    assert!(!owner.observation().traffic_status().active);
    assert!(owner.observation().traffic_status().failure.is_none());
    assert!(owner.observation().snapshot().identity.is_none());
    crate::storage::traffic::TrafficWriter::open(&root.join("traffic"), chrono_tz::UTC)
        .unwrap()
        .close()
        .unwrap();
    assert!(matches!(
        owner.release_handoff(&ticket).unwrap(),
        OwnerTransfer::Released { .. }
    ));
    drop(owner);
    std::fs::remove_dir_all(root).unwrap();
}

// P4-03：保护稳定 Group/线路身份、真实事务顺序及选择与配置版本分离。
fn p403_fixture() -> (
    ManualRuntime<Mock>,
    Arc<Mutex<Trace>>,
    JsonStateStore,
    PathBuf,
) {
    use crate::domain::*;
    let (owner, trace, store, root) = manual_fixture();
    let mut state = store.load().unwrap();
    let mut group = NodeGroup::fresh().unwrap();
    group.id = PoolId("failover".into());
    group.name = "主备".into();
    group.rule = GroupRule::Failover;
    group.members.clear();
    group.mode = GroupMode::Static;
    group.failover = Some(FailoverSettings::default());
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
            members: vec![OutboundId::Node(NodeId("c".into()))],
            manual: false,
        },
    ];
    state.groups = vec![group];
    state.default_target = RouteTarget::Pool(PoolId("failover".into()));
    state.active_subscription_id = Some(SubscriptionId("subscription".into()));
    state.routes.clear();
    store.save(&state).unwrap();
    (owner, trace, store, root)
}
fn p403_request(
    owner: &ManualRuntime<Mock>,
    selector: PoolId,
    member: OutboundId,
    mode: crate::domain::GroupSelectionMode,
) -> ManualSelectionRequest<GroupChoice> {
    let s = owner.snapshot().unwrap();
    ManualSelectionRequest {
        instance: s.runtime.instance_id.unwrap(),
        config: s.runtime.applied_version.unwrap(),
        expected: s.confirmed_selection_version.unwrap(),
        pool: selector,
        node: GroupChoice {
            group: PoolId("failover".into()),
            member,
            mode,
        },
    }
}
#[test]
fn p403_group_pin_lane_manual_resume_and_restart_are_selection_only() {
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let state = store.load().unwrap();
    let group = &state.groups[0];
    let outer = group.id.clone();
    let primary = group.lanes[0].pool_id(&outer);
    let backup = group.lanes[1].pool_id(&outer);
    let request = p403_request(
        &owner,
        outer.clone(),
        OutboundId::Pool(backup.clone()),
        GroupSelectionMode::ManualPin,
    );
    owner.select_manual(request).unwrap();
    let pinned = store.load().unwrap();
    assert_eq!(pinned.config_revision, state.config_revision);
    assert_eq!(pinned.selection_revision, state.selection_revision + 2);
    assert_eq!(
        pinned.group_selections[&outer].mode,
        GroupSelectionMode::ManualPin
    );
    owner
        .select_manual(p403_request(
            &owner,
            primary.clone(),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::Auto,
        ))
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&outer],
        OutboundId::Pool(backup)
    );
    assert_eq!(
        store.load().unwrap().group_selections[&outer].mode,
        GroupSelectionMode::ManualPin
    );
    owner
        .select_manual(p403_request(
            &owner,
            outer.clone(),
            OutboundId::Pool(primary.clone()),
            GroupSelectionMode::Auto,
        ))
        .unwrap();
    assert_eq!(
        store.load().unwrap().group_selections[&outer].mode,
        GroupSelectionMode::Auto
    );
    let old = owner.snapshot().unwrap().runtime.instance_id;
    owner.execute(RuntimeCommand::Restart, |_| {}).unwrap();
    assert_ne!(owner.snapshot().unwrap().runtime.instance_id, old);
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&primary],
        OutboundId::Node(NodeId("b".into()))
    );
    assert!(trace.lock().unwrap().writes >= 3);
    assert_eq!(store.load().unwrap().config_revision, state.config_revision);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn p403_group_failure_retains_mode_and_pending_reads_actual_without_retry() {
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let group = store.load().unwrap().groups[0].clone();
    let backup = OutboundId::Pool(group.lanes[1].pool_id(&group.id));
    trace.lock().unwrap().write_failures.push_back(true);
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.id.clone(),
            backup.clone(),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerWrite)
    );
    assert_eq!(
        store.load().unwrap().group_selections[&group.id].mode,
        GroupSelectionMode::Auto
    );
    assert!(
        store.load().unwrap().group_selections[&group.id]
            .pending
            .is_none()
    );
    trace.lock().unwrap().read_failures.push_back(true);
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.id.clone(),
            backup.clone(),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerReadBack)
    );
    assert_eq!(
        store.load().unwrap().group_selections[&group.id].mode,
        GroupSelectionMode::Auto
    );
    assert!(
        store.load().unwrap().group_selections[&group.id]
            .pending
            .is_some()
    );
    let snapshot = owner.snapshot().unwrap();
    assert_eq!(
        snapshot.saved_selection_version,
        store.load().unwrap().selection_version()
    );
    assert!(snapshot.confirmed_selection_version.is_none());
    let writes = trace.lock().unwrap().writes;
    owner
        .execute(RuntimeCommand::ReconcileSelection, |_| {})
        .unwrap();
    assert_eq!(trace.lock().unwrap().writes, writes);
    assert_eq!(
        store.load().unwrap().group_selections[&group.id].mode,
        GroupSelectionMode::ManualPin
    );
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections[&group.id],
        backup
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn p403_group_confirmation_disk_failure_and_config_conflict_keep_truth() {
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let group = store.load().unwrap().groups[0].clone();
    let backup = OutboundId::Pool(group.lanes[1].pool_id(&group.id));
    trace.lock().unwrap().save_obstacle = Some(root.join("state.tmp"));
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.id.clone(),
            backup.clone(),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ConfirmationSaveFailed)
    );
    assert!(
        store.load().unwrap().group_selections[&group.id]
            .pending
            .is_some()
    );
    std::fs::remove_dir(root.join("state.tmp")).unwrap();
    let writes = trace.lock().unwrap().writes;
    owner
        .execute(RuntimeCommand::ReconcileSelection, |_| {})
        .unwrap();
    assert_eq!(trace.lock().unwrap().writes, writes);
    let request = p403_request(&owner, group.id.clone(), backup, GroupSelectionMode::Auto);
    let mut state = store.load().unwrap();
    state.groups[0].lanes[0].members.reverse();
    store.commit(&state).unwrap();
    assert_eq!(
        owner.select_manual(request),
        Err(SelectionError::ConfigChanged)
    );
    assert_eq!(trace.lock().unwrap().writes, writes);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 保护配置编辑不会破坏未知 pending，也不会把无效 pin 偷偷固定到默认主用。
#[test]
fn p403_edit_pending_guard_and_invalid_pin_returns_auto() {
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let group = store.load().unwrap().groups[0].clone();
    let backup = OutboundId::Pool(group.lanes[1].pool_id(&group.id));
    trace.lock().unwrap().read_failures.push_back(true);
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.id.clone(),
            backup,
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerReadBack)
    );
    let before = store.load().unwrap();
    let mut groups = before.groups.clone();
    groups[0].lanes.pop();
    assert!(
        owner
            .snapshots
            .save_groups(before.config_version(), groups.clone())
            .is_err()
    );
    assert_eq!(store.load().unwrap(), before);
    // requested 仍存在也不能移除旧确认（此处为编译默认主用），或改变默认顺序。
    let mut remove_old = before.groups.clone();
    remove_old[0].lanes.remove(0);
    assert!(
        owner
            .snapshots
            .save_groups(before.config_version(), remove_old)
            .is_err()
    );
    let mut reorder = before.groups.clone();
    reorder[0].lanes.reverse();
    assert!(
        owner
            .snapshots
            .save_groups(before.config_version(), reorder)
            .is_err()
    );
    assert_eq!(store.load().unwrap(), before);
    owner
        .execute(RuntimeCommand::ReconcileSelection, |_| {})
        .unwrap();
    let before = store.load().unwrap();
    let saved = owner
        .snapshots
        .save_groups(before.config_version(), groups)
        .unwrap()
        .value;
    assert_eq!(
        saved.group_selections[&group.id].mode,
        GroupSelectionMode::Auto
    );
    assert!(saved.group_selections[&group.id].selected.is_none());
    assert_eq!(saved.selection_revision, before.selection_revision + 1);
    assert_eq!(saved.config_revision, before.config_revision + 1);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
// 保护旧 Group manifest 升级后仍能恢复；明确默认值随后通过当前 child GET 核对。
#[test]
fn p403_old_group_manifest_without_group_section_is_readable() {
    let (mut owner, _, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let path = root.join("runtime/last-applied.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    manifest["confirmed_selection"]
        .as_object_mut()
        .unwrap()
        .remove("groups");
    std::fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(owner.load_recovery(&store.load().unwrap()).is_ok());
    owner
        .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
        .unwrap();
    assert_eq!(
        owner.snapshot().unwrap().confirmed_group_selections.len(),
        3
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn p403_plain_selector_rejects_pin_before_pending_or_controller() {
    // 普通组只能选择成员；非法 pin 必须在 Controller 前拒绝，而不是写后卡 pending。
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    let mut state = store.load().unwrap();
    state.groups[0].rule = GroupRule::Selector;
    state.groups[0].members = vec![
        OutboundId::Node(NodeId("a".into())),
        OutboundId::Node(NodeId("b".into())),
    ];
    state.groups[0].lanes.clear();
    state.groups[0].failover = None;
    store.commit(&state).unwrap();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let state = store.load().unwrap();
    let before = trace.lock().unwrap().writes;
    let group = state.groups[0].id.clone();
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.clone(),
            OutboundId::Node(NodeId("b".into())),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::InvalidMember)
    );
    assert_eq!(trace.lock().unwrap().writes, before);
    assert_eq!(store.load().unwrap(), state);
    assert!(
        SelectionService::new(owner.snapshots.as_ref().clone())
            .stage_group(
                state.config_version(),
                state.selection_version(),
                group,
                GroupSelectionIntent {
                    member: OutboundId::Node(NodeId("b".into())),
                    mode: GroupSelectionMode::ManualPin
                },
                None
            )
            .is_err()
    );
    assert_eq!(store.load().unwrap(), state);
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 核对使用原快照；读取 Controller 期间的配置保存不得被重新绑定为已确认选择。
#[test]
fn p403_reconcile_keeps_request_fence_during_concurrent_save() {
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let group = store.load().unwrap().groups[0].clone();
    let backup = OutboundId::Pool(group.lanes[1].pool_id(&group.id));
    trace.lock().unwrap().read_failures.push_back(true);
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.id.clone(),
            backup,
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerReadBack)
    );
    let snapshot = owner.snapshot().unwrap();
    let instance = snapshot.runtime.instance_id.unwrap();
    let expected = store.load().unwrap().version();
    let reads = trace.lock().unwrap().reads;
    let mut stale = expected.clone();
    stale.selection.0.revision -= 1;
    assert_eq!(
        owner.reconcile_selection(instance.clone(), stale, group.id.clone()),
        Err(SelectionError::StaleVersion)
    );
    assert_eq!(trace.lock().unwrap().reads, reads);
    let mut changed = store.load().unwrap();
    changed.groups[0].name = "保存后的名字".into();
    trace.lock().unwrap().business_on_read = Some(changed);
    let writes = trace.lock().unwrap().writes;
    assert_eq!(
        owner.reconcile_selection(instance.clone(), expected.clone(), group.id.clone()),
        Err(SelectionError::Pending)
    );
    assert_eq!(trace.lock().unwrap().writes, writes);
    assert!(
        store.load().unwrap().group_selections[&group.id]
            .pending
            .is_some()
    );
    assert_eq!(
        owner.reconcile_selection(instance, expected, group.id),
        Err(SelectionError::ConfigChanged)
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// candidate 已确认 pending 后，后续读回失败仍可按最新已确认快照回退成功版。
#[test]
fn p403_pending_confirm_then_read_failure_rolls_back_with_latest_version() {
    let (mut owner, trace, store, root) = manual_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    let state = store.load().unwrap();
    let pending = SelectionService::new((*owner.snapshots).clone())
        .begin_manual_pending(
            state.selection_version(),
            PoolId("manual".into()),
            NodeId("b".into()),
        )
        .unwrap();
    {
        let mut t = trace.lock().unwrap();
        t.cache_choices
            .insert("pool-manual".into(), "node-b".into());
        t.read_failures.extend([false, true]);
    }
    assert_eq!(
        owner.execute(RuntimeCommand::ApplySaved, |_| {}),
        Err(RuntimeError::CandidateRolledBack(
            CandidateFailure::SelectionReconcile
        ))
    );
    let saved = store.load().unwrap();
    assert_eq!(saved.selection_revision, pending.version.0.revision + 1);
    assert!(!ManualRuntime::<Mock>::has_pending(&saved));
    assert_eq!(
        owner.snapshot().unwrap().confirmed_selection_version,
        Some(saved.selection_version())
    );
    assert_eq!(
        owner.snapshot().unwrap().runtime.status,
        crate::application::runtime_snapshot::RuntimeStatus::Ready
    );
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

// 核对时 child 退出/清理失败，不得发布旧 Ready，也不得清除不确定 pending。
fn p403_reconcile_inactive_child_case(fail_stop: bool) {
    use crate::application::runtime_snapshot::RuntimeStatus;
    use crate::domain::*;
    let (mut owner, trace, store, root) = p403_fixture();
    owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
    let group = store.load().unwrap().groups[0].clone();
    trace.lock().unwrap().read_failures.push_back(true);
    assert_eq!(
        owner.select_manual(p403_request(
            &owner,
            group.id.clone(),
            OutboundId::Pool(group.lanes[1].pool_id(&group.id)),
            GroupSelectionMode::ManualPin
        )),
        Err(SelectionError::ControllerReadBack)
    );
    let before = store.load().unwrap();
    let instance = owner.snapshot().unwrap().runtime.instance_id.unwrap();
    let (reads, writes) = {
        let mut t = trace.lock().unwrap();
        t.crash = true;
        t.fail_stop = fail_stop;
        (t.reads, t.writes)
    };
    assert_eq!(
        owner.reconcile_selection(instance, before.version(), group.id.clone()),
        Err(SelectionError::StaleInstance)
    );
    let after = owner.snapshot().unwrap();
    assert_eq!(
        after.runtime.status,
        if fail_stop {
            RuntimeStatus::Recovering
        } else {
            RuntimeStatus::Failed
        }
    );
    assert!(after.runtime.applied_version.is_none());
    assert!(after.runtime.instance_id.is_none());
    assert!(after.endpoints.is_none());
    assert!(after.uptime_seconds.is_none());
    assert!(owner.failover.is_empty());
    assert!(after.failover_status.is_empty());
    assert_eq!(store.load().unwrap(), before);
    {
        let t = trace.lock().unwrap();
        assert_eq!((t.reads, t.writes), (reads, writes));
    }
    trace.lock().unwrap().fail_stop = false;
    owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn p403_reconcile_exited_child_clears_ready_and_keeps_pending() {
    p403_reconcile_inactive_child_case(false);
}
#[test]
fn p403_reconcile_cleanup_failure_clears_ready_and_keeps_pending() {
    p403_reconcile_inactive_child_case(true);
}
