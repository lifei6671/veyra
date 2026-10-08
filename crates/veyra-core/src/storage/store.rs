#[path = "remote_selection.rs"]
mod remote_selection;
pub use remote_selection::{RemoteSelection, SelectionFence};
use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::{
    AppState, CURRENT_SCHEMA_VERSION, NodePool, Provider, ProxyNode, RoutePolicy, SelectionPolicy,
    StateValidationError, Subscription,
};

use super::migration::migrate_to_current;
use super::snapshot::{
    atomic_replace, backup_path, copy_for_backup, pre_migration_backup_path, preserve_corrupt_copy,
    read_snapshot,
};
use super::validation::validate_state;

pub trait StateStore {
    fn load(&self) -> Result<AppState, StateStoreError>;
    /// Physical write primitive for an already validated transaction (or an initial fixture).
    fn save(&self, state: &AppState) -> Result<(), StateStoreError>;
    fn commit(&self, candidate: &AppState) -> Result<AppState, StateStoreError> {
        let previous = self.load()?;
        let next = candidate
            .revised_from(&previous)
            .map_err(|_| StateStoreError::RevisionConflict)?;
        self.save(&next)?;
        Ok(next)
    }
}

#[derive(Clone, Debug)]
pub struct JsonStateStore {
    state_file: PathBuf,
}

impl JsonStateStore {
    /// 预览只读版本，不触发迁移、恢复、备份或新 epoch；恢复仍由正常加载入口负责。
    pub fn read_current_version(
        &self,
    ) -> Result<Option<crate::domain::ConfigVersion>, StateStoreError> {
        if !self.has_snapshot_or_backup()? {
            return Ok(None);
        }
        let (state, migrated) = self.decode(&read_snapshot(&self.state_file)?)?;
        if migrated {
            return Err(StateStoreError::InvalidStoredState);
        }
        Ok(Some(state.config_version()))
    }

    pub fn new(state_file: PathBuf) -> Result<Self, StateStoreError> {
        if state_file.file_name().is_none() {
            return Err(StateStoreError::InvalidStatePath);
        }
        Ok(Self { state_file })
    }

    #[cfg(test)]
    fn state_file(&self) -> &std::path::Path {
        &self.state_file
    }

    fn decode(&self, contents: &[u8]) -> Result<(AppState, bool), StateStoreError> {
        let document =
            serde_json::from_slice(contents).map_err(|_| StateStoreError::InvalidJson)?;
        let (migrated, was_migrated) = migrate_to_current(document)?;
        let stored = serde_json::from_value::<StoredStateV9>(migrated)
            .map_err(|_| StateStoreError::InvalidStoredState)?;
        let state = validate_state(AppState::try_from(stored)?)?;
        Ok((state, was_migrated))
    }

    fn load_backup(&self) -> Result<AppState, StateStoreError> {
        let contents = read_snapshot(&backup_path(&self.state_file))?;
        let (mut state, _) = self.decode(&contents)?;
        // A full rollback/recovery is a replacement, never a reusable old concurrency token.
        // replacement 的旧 pending 不得跨越 epoch 恢复为新业务意图。
        for pool in &mut state.pools {
            if let SelectionPolicy::Manual {
                pending_node_id, ..
            } = &mut pool.selection
            {
                *pending_node_id = None;
            }
        }
        state.state_epoch =
            crate::domain::StateEpoch::fresh().map_err(|_| StateStoreError::WriteFailed)?;
        state.config_revision = 0;
        state.selection_revision = 0;
        Ok(state)
    }

    fn write_current_without_backup(&self, state: &AppState) -> Result<(), StateStoreError> {
        let _writer = self.writer_lock()?;
        self.writable()?;
        validate_state(state.clone())?;
        let contents = serde_json::to_vec_pretty(&StoredStateV9::from(state))
            .map_err(|_| StateStoreError::SerializationFailed)?;
        atomic_replace(&self.state_file, &contents)
    }

    /// Caller holds the shared StateAccessGate. Raw save is only the physical snapshot primitive.
    /// Legacy consumers use this entrypoint too, so a delayed candidate cannot overwrite a new epoch.
    pub fn commit(&self, candidate: &AppState) -> Result<AppState, StateStoreError> {
        // 自动迁移/备份恢复先按原契约执行；取得writer锁后再读取当前版本做CAS。
        if self.has_snapshot_or_backup()? {
            self.load()?;
        }
        let _writer = self.lock_unfenced_writer()?;
        let next = if self.has_snapshot_or_backup()? {
            let previous = self.strict_state()?;
            let revised = candidate.revised_from(&previous).map_err(|error| {
                if error.code() == crate::domain::AppErrorCode::RevisionConflict {
                    StateStoreError::RevisionConflict
                } else {
                    StateStoreError::WriteFailed
                }
            })?;
            if revised == previous {
                return Ok(previous);
            }
            revised
        } else {
            let mut initial = AppState::try_empty().map_err(|_| StateStoreError::WriteFailed)?;
            initial.state_epoch = candidate.state_epoch.clone();
            candidate
                .revised_from(&initial)
                .map_err(|_| StateStoreError::RevisionConflict)?
        };
        self.save_unfenced(&next)?;
        Ok(next)
    }

    pub fn has_snapshot_or_backup(&self) -> Result<bool, StateStoreError> {
        let current = self
            .state_file
            .try_exists()
            .map_err(|_| StateStoreError::ReadFailed)?;
        let backup = backup_path(&self.state_file)
            .try_exists()
            .map_err(|_| StateStoreError::ReadFailed)?;
        Ok(current || backup)
    }
}

impl StateStore for JsonStateStore {
    fn commit(&self, candidate: &AppState) -> Result<AppState, StateStoreError> {
        JsonStateStore::commit(self, candidate)
    }
    fn load(&self) -> Result<AppState, StateStoreError> {
        if !self
            .state_file
            .try_exists()
            .map_err(|_| StateStoreError::ReadFailed)?
        {
            if !backup_path(&self.state_file)
                .try_exists()
                .map_err(|_| StateStoreError::ReadFailed)?
            {
                return Err(StateStoreError::ReadFailed);
            }
            let recovered = self
                .load_backup()
                .map_err(|_| StateStoreError::NoValidBackup)?;
            self.write_current_without_backup(&recovered)?;
            return Ok(recovered);
        }
        let contents = read_snapshot(&self.state_file)?;
        match self.decode(&contents) {
            Ok((state, false)) => Ok(state),
            Ok((state, true)) => {
                copy_for_backup(
                    &self.state_file,
                    &pre_migration_backup_path(&self.state_file),
                )?;
                self.save(&state)?;
                Ok(state)
            }
            Err(error) if error.is_recoverable() => {
                self.writable()?;
                preserve_corrupt_copy(&self.state_file)?;
                let recovered = self
                    .load_backup()
                    .map_err(|_| StateStoreError::NoValidBackup)?;
                self.write_current_without_backup(&recovered)?;
                Ok(recovered)
            }
            Err(error) => Err(error),
        }
    }

    fn save(&self, state: &AppState) -> Result<(), StateStoreError> {
        let _writer = self.writer_lock()?;
        self.writable()?;
        self.save_unfenced(state)
    }
}
impl JsonStateStore {
    fn save_unfenced(&self, state: &AppState) -> Result<(), StateStoreError> {
        validate_state(state.clone())?;
        let stored = StoredStateV9::from(state);
        let contents =
            serde_json::to_vec_pretty(&stored).map_err(|_| StateStoreError::SerializationFailed)?;

        if self.state_file.exists() {
            copy_for_backup(&self.state_file, &backup_path(&self.state_file))?;
        }
        atomic_replace(&self.state_file, &contents)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredStateV9 {
    schema_version: u32,
    state_epoch: crate::domain::StateEpoch,
    config_revision: u64,
    selection_revision: u64,
    profile: crate::domain::Profile,
    app_config: crate::domain::AppConfig,
    default_target: crate::domain::RouteTarget,
    active_subscription_id: Option<crate::domain::SubscriptionId>,
    active_configuration_generation: u64,
    subscriptions: Vec<Subscription>,
    providers: Vec<Provider>,
    nodes: Vec<ProxyNode>,
    pools: Vec<NodePool>,
    #[serde(default)]
    groups: Vec<crate::domain::NodeGroup>,
    routes: Vec<RoutePolicy>,
}

impl From<&AppState> for StoredStateV9 {
    fn from(state: &AppState) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            state_epoch: state.state_epoch.clone(),
            config_revision: state.config_revision,
            selection_revision: state.selection_revision,
            profile: state.profile.clone(),
            app_config: state.app_config.clone(),
            default_target: state.default_target.clone(),
            active_subscription_id: state.active_subscription_id.clone(),
            active_configuration_generation: state.active_configuration_generation,
            subscriptions: state.subscriptions.clone(),
            providers: state.providers.clone(),
            nodes: state.nodes.clone(),
            pools: state.pools.clone(),
            groups: state.groups.clone(),
            routes: state.routes.clone(),
        }
    }
}

impl TryFrom<StoredStateV9> for AppState {
    type Error = StateStoreError;

    fn try_from(stored: StoredStateV9) -> Result<Self, Self::Error> {
        if stored.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(StateStoreError::UnsupportedSchemaVersion);
        }
        Ok(Self {
            schema_version: stored.schema_version,
            state_epoch: stored.state_epoch,
            config_revision: stored.config_revision,
            selection_revision: stored.selection_revision,
            profile: stored.profile,
            app_config: stored.app_config,
            default_target: stored.default_target,
            active_subscription_id: stored.active_subscription_id,
            active_configuration_generation: stored.active_configuration_generation,
            subscriptions: stored.subscriptions,
            providers: stored.providers,
            nodes: stored.nodes,
            pools: stored.pools,
            groups: stored.groups,
            routes: stored.routes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateStoreError {
    WriteFenced,
    InvalidState(StateValidationError),
    RevisionConflict,
    InvalidStatePath,
    ReadFailed,
    WriteFailed,
    ReplaceFailed,
    InvalidJson,
    MissingSchemaVersion,
    MigrationFailed,
    InvalidStoredState,
    UnsupportedSchemaVersion,
    NoValidBackup,
    SerializationFailed,
}

impl fmt::Display for StateStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::WriteFenced => "state writes await remote selection reconciliation",
            Self::InvalidState(error) => return write!(formatter, "invalid state: {error}"),
            Self::RevisionConflict => "state snapshot version changed",
            Self::InvalidStatePath => "state storage path is invalid",
            Self::ReadFailed => "state snapshot could not be read",
            Self::WriteFailed => "state snapshot could not be written",
            Self::ReplaceFailed => "state snapshot could not be atomically replaced",
            Self::InvalidJson => "state snapshot is not valid JSON",
            Self::MissingSchemaVersion => "state snapshot has no schema version",
            Self::MigrationFailed => "state snapshot could not be migrated",
            Self::InvalidStoredState => "state snapshot has an invalid structure",
            Self::UnsupportedSchemaVersion => "state snapshot schema version is unsupported",
            Self::NoValidBackup => "state snapshot and backup are not valid",
            Self::SerializationFailed => "state snapshot could not be serialized",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for StateStoreError {}

impl StateStoreError {
    fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::InvalidState(_)
                | Self::InvalidJson
                | Self::MissingSchemaVersion
                | Self::InvalidStoredState
        )
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::domain::{
        NodeFilter, NodeId, NodePool, PoolId, PoolKind, PoolSource, ProtocolOptions, ProviderId,
        ProxyProtocol, RoutePolicy, RoutePolicyId, RouteTarget, SelectionPolicy, SubscriptionId,
        TrafficMatcher, Transport,
    };
    use crate::storage::snapshot::{backup_path, corrupt_copy_path, pre_migration_backup_path};

    fn unique_test_store() -> JsonStateStore {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir()
            .join(format!("veyra-state-store-{}-{nanos}", std::process::id()))
            .join("state.json");
        JsonStateStore::new(path).expect("valid test state file")
    }

    fn valid_state() -> AppState {
        AppState {
            schema_version: CURRENT_SCHEMA_VERSION,
            state_epoch: crate::domain::StateEpoch::fresh().expect("test epoch"),
            config_revision: 0,
            selection_revision: 0,
            profile: crate::domain::Profile::default(),
            app_config: crate::domain::AppConfig::default(),
            default_target: RouteTarget::Unconfigured,
            active_subscription_id: None,
            active_configuration_generation: 0,
            subscriptions: vec![Subscription {
                skipped_unsupported_nodes: 0,
                id: SubscriptionId("subscription".to_owned()),
                name: "Test".to_owned(),
                description: String::new(),
                source: crate::domain::SubscriptionSource::Manual,
                last_success_at_ms: None,
                last_attempt_at_ms: None,
                http_metadata: None,
                remote_request: None,
                update_policy: crate::domain::SubscriptionUpdatePolicy::manual(),
                document: None,
            }],
            providers: vec![Provider {
                id: ProviderId("provider".to_owned()),
                subscription_id: SubscriptionId("subscription".to_owned()),
                name: "Default".to_owned(),
            }],
            nodes: vec![ProxyNode {
                id: NodeId("node".to_owned()),
                provider_id: ProviderId("provider".to_owned()),
                name: "Test node".to_owned(),
                protocol: ProxyProtocol::Shadowsocks,
                server: "example.invalid".to_owned(),
                port: 443,
                options: ProtocolOptions::Shadowsocks {
                    method: "aes-128-gcm".to_owned(),
                    password: "test-secret".to_owned(),
                },
                transport: Some(Transport::Tcp),
                tls: None,
            }],
            pools: Vec::new(),
            groups: Vec::new(),
            routes: Vec::new(),
        }
    }

    fn valid_state_with_pool_and_route() -> AppState {
        let mut state = valid_state();
        state.pools.push(NodePool {
            id: PoolId("pool".to_owned()),
            name: "Default".to_owned(),
            kind: PoolKind::ImplicitProvider,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider".to_owned()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(NodeId("node".to_owned())),
                pending_node_id: None,
            },
            enabled: true,
        });
        state.routes.push(RoutePolicy {
            id: RoutePolicyId("route".to_owned()),
            name: "Example".to_owned(),
            enabled: true,
            priority: 0,
            matcher: TrafficMatcher::DomainSuffix(vec!["example.com".to_owned()]),
            target: RouteTarget::Pool(PoolId("pool".to_owned())),
        });
        state
    }

    fn remove_test_files(store: &JsonStateStore) {
        let directory = store.state_file().parent().expect("parent directory");
        if directory.exists() {
            fs::remove_dir_all(directory).expect("remove test directory");
        }
    }

    #[test]
    #[ignore = "internal child fixture, invoked only by remote_fence_cross_process"]
    fn remote_fence_os_child() {
        // 只由父测试提供自有临时state路径；不是生产入口。
        let path =
            PathBuf::from(std::env::var_os("VEYRA_TEST_FENCED_STATE").expect("parent fixture"));
        let store = JsonStateStore::new(path).unwrap();
        let state = store.load().unwrap();
        assert_eq!(store.save(&state), Err(StateStoreError::WriteFenced));
    }
    #[test]
    fn remote_fence_cross_process() {
        // 保护新进程/独立StateAccessGate：OS互斥及持久fence都不能因进程内锁消失而绕过。
        let store = unique_test_store();
        let state = valid_state_with_pool_and_route();
        store.save(&state).unwrap();
        let child = || {
            let mut process = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "storage::store::tests::remote_fence_os_child",
                    "--ignored",
                ])
                .env("VEYRA_TEST_FENCED_STATE", &store.state_file)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap();
            let until = std::time::Instant::now() + std::time::Duration::from_secs(5);
            loop {
                if let Some(status) = process.try_wait().unwrap() {
                    assert!(status.success());
                    break;
                }
                if std::time::Instant::now() >= until {
                    process.kill().unwrap();
                    process.wait().unwrap();
                    panic!("isolated child timeout");
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        };
        let guard = store.writer_lock().unwrap();
        child();
        drop(guard);
        store
            .begin_remote(&RemoteSelection {
                request_id: 1,
                instance: 1,
                expected: state.version(),
                pool: PoolId("pool".into()),
                node: NodeId("node".into()),
            })
            .unwrap();
        child();
        assert!(store.selection_fence().unwrap().is_some());
        remove_test_files(&store);
    }

    #[test]
    fn remote_fence_blocks_all_json_writers_and_survives_reopen() {
        // 保护RPC超时/重启后的pending：整体替换、订阅更新、旧cache恢复不能越过持久fence。
        use crate::application::{
            state_access::StateAccessGate,
            state_service::{SelectionService, SnapshotService},
            subscription_management::{
                ImportRequestSource, SubscriptionImport, SubscriptionManager,
            },
        };
        let store = unique_test_store();
        let mut state = valid_state_with_pool_and_route();
        let mut next_node = state.nodes[0].clone();
        next_node.id = NodeId("next".into());
        state.nodes.push(next_node);
        store.save(&state).unwrap();
        let request = RemoteSelection {
            request_id: 9,
            instance: 11,
            expected: state.version(),
            pool: PoolId("pool".into()),
            node: NodeId("next".into()),
        };
        let fence = store.begin_remote(&request).unwrap();
        let pending = store.load().unwrap();
        assert_eq!(pending.selection_revision, 1);
        let reopened = JsonStateStore::new(store.state_file.clone()).unwrap();
        assert_eq!(reopened.selection_fence().unwrap(), Some(fence.clone()));
        assert_eq!(reopened.save(&state), Err(StateStoreError::WriteFenced));
        let snapshots = SnapshotService::new(reopened.clone(), StateAccessGate::default());
        assert!(snapshots.replace(pending.version(), state.clone()).is_err());
        assert!(
            SelectionService::new(snapshots.clone())
                .begin_manual_pending(
                    pending.selection_version(),
                    request.pool.clone(),
                    NodeId("node".into())
                )
                .is_err()
        );
        // 正式SubscriptionManager用独立gate，也必须由底层writer边界拒绝。
        let manager =
            SubscriptionManager::new(store.state_file.clone(), StateAccessGate::default()).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        assert!(
            runtime
                .block_on(manager.import_with_options(SubscriptionImport {
                    name: "blocked".into(),
                    description: String::new(),
                    source: ImportRequestSource::Manual {
                        content:
                            "ss://YWVzLTEyOC1nY206cGFzc3dvcmQ=@example.invalid:443#fixture".into()
                    }
                }))
                .is_err()
        );
        // 即使ProviderReplacement持有自己的mutex而非SnapshotService gate，最终commit也被拒绝。
        let mut provider_state = pending.clone();
        provider_state.pools.clear();
        provider_state.routes.clear();
        let provider = crate::application::provider_replacement::ProviderReplacementService::new(
            reopened.clone(),
            provider_state,
        )
        .unwrap();
        let parsed = crate::subscription::parse_subscription(
            "ss://YWVzLTEyOC1nY206cGFzc3dvcmQ=@example.invalid:443#fixture",
        )
        .unwrap();
        assert!(matches!(
            provider.replace(ProviderId("provider".into()), parsed),
            Err(
                crate::application::provider_replacement::ProviderReplacementError::Store(
                    StateStoreError::WriteFenced
                )
            )
        ));
        assert_eq!(reopened.load().unwrap(), pending);
        assert!(
            reopened
                .confirm_remote(&fence, &NodeId("third".into()))
                .is_err()
        );
        let confirmed = reopened.confirm_remote(&fence, &request.node).unwrap();
        assert_eq!(confirmed.selection_revision, 2);
        assert_eq!(reopened.save(&confirmed), Err(StateStoreError::WriteFenced));
        assert_eq!(
            reopened.confirm_remote(&fence, &request.node).unwrap(),
            confirmed
        );
        reopened
            .finish_remote(&fence, &confirmed.version())
            .unwrap();
        assert!(reopened.selection_fence().unwrap().is_none());
        reopened.save(&confirmed).unwrap();
        // 原快照损坏时不能从旧backup清pending/fence并换epoch。
        let mut request = request;
        request.expected = confirmed.version();
        request.request_id += 1;
        reopened.begin_remote(&request).unwrap();
        fs::write(&reopened.state_file, b"corrupt").unwrap();
        assert_eq!(reopened.load(), Err(StateStoreError::WriteFenced));
        assert!(reopened.selection_fence().unwrap().is_some());
        remove_test_files(&store);
    }

    #[test]
    fn saves_and_loads_a_complete_valid_state() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save state");

        assert_eq!(store.load().expect("load state"), state);
        remove_test_files(&store);
    }

    #[test]
    fn round_trips_v2_state_with_pool_and_route() {
        let store = unique_test_store();
        let state = valid_state_with_pool_and_route();
        store.save(&state).expect("save v2 state");

        assert_eq!(store.load().expect("load v2 state"), state);
        remove_test_files(&store);
    }

    #[test]
    fn keeps_a_backup_before_replacing_the_current_state() {
        let store = unique_test_store();
        let original = valid_state();
        store.save(&original).expect("save original");

        let mut updated = original.clone();
        updated.nodes[0].name = "Updated node".to_owned();
        store.save(&updated).expect("save updated");

        let backup = fs::read_to_string(backup_path(store.state_file())).expect("read backup");
        assert!(backup.contains("Test node"));
        assert_eq!(store.load().expect("load updated"), updated);
        remove_test_files(&store);
    }

    #[test]
    fn migrates_v1_once_and_keeps_the_pre_migration_snapshot() {
        let store = unique_test_store();
        let state = valid_state();
        atomic_replace(
            store.state_file(),
            include_bytes!("../../tests/fixtures/state/v1-valid.json"),
        )
        .expect("write v1 fixture");

        let migrated = store.load().expect("migrate v1");
        let mut expected = state.clone();
        expected.state_epoch = migrated.state_epoch.clone();
        assert_eq!(migrated, expected);
        assert!(pre_migration_backup_path(store.state_file()).exists());
        let migrated_bytes = fs::read(store.state_file()).expect("read migrated state");
        assert_eq!(store.load().expect("reload v1"), migrated);
        assert_eq!(
            fs::read(store.state_file()).expect("read reloaded state"),
            migrated_bytes
        );
        remove_test_files(&store);
    }

    #[test]
    fn migrates_v2_nodes_without_changing_ids_or_pool_and_route_references() {
        let store = unique_test_store();
        let v2 = serde_json::json!({
            "schema_version": 2,
            "subscriptions": [{"id":"subscription","name":"Test"}],
            "providers": [{"id":"provider","subscription_id":"subscription","name":"Default"}],
            "nodes": [{
                "id":"node-preserved","provider_id":"provider","name":"Node","protocol":"vless",
                "server":"example.invalid","port":443,
                "credentials":{"kind":"uuid","uuid":"fixture-uuid","flow":"xtls-rprx-vision"},
                "transport":"tcp","tls":null
            }],
            "pools": [{
                "id":"pool-preserved","name":"Pool","kind":"custom","enabled":true,
                "sources":[{"provider_id":"provider","filter":{"regions":[],"protocols":[],"include_keywords":[],"exclude_keywords":[],"include_node_ids":[],"exclude_node_ids":[]}}],
                "selection":{"kind":"manual","selected_node_id":"node-preserved"}
            }],
            "routes": [{
                "id":"route-preserved","name":"Route","enabled":true,"priority":0,
                "matcher":{"kind":"domain","values":["example.com"]},
                "target":{"kind":"pool","pool_id":"pool-preserved"}
            }]
        });
        atomic_replace(
            store.state_file(),
            &serde_json::to_vec(&v2).expect("encode v2 fixture"),
        )
        .expect("write v2 fixture");

        let state = store.load().expect("migrate v2");

        assert_eq!(state.default_target, RouteTarget::Unconfigured);
        assert_eq!(state.nodes[0].id, NodeId("node-preserved".to_owned()));
        assert_eq!(state.pools[0].id, PoolId("pool-preserved".to_owned()));
        assert!(matches!(
            state.pools[0].selection,
            SelectionPolicy::Manual { selected_node_id: Some(NodeId(ref id)), .. } if id == "node-preserved"
        ));
        assert!(matches!(
            state.routes[0].target,
            RouteTarget::Pool(ref id) if id.0 == "pool-preserved"
        ));
        assert!(pre_migration_backup_path(store.state_file()).exists());
        remove_test_files(&store);
    }

    #[test]
    fn migrates_v3_subscription_fields_without_changing_existing_identity() {
        let store = unique_test_store();
        let mut v3 = serde_json::to_value(StoredStateV9::from(&valid_state_with_pool_and_route()))
            .expect("encode current fixture");
        v3["schema_version"] = serde_json::json!(3);
        let object = v3.as_object_mut().expect("state object");
        object.remove("active_subscription_id");
        object.remove("active_configuration_generation");
        let subscription = v3["subscriptions"][0]
            .as_object_mut()
            .expect("subscription object");
        subscription.remove("source");
        subscription.remove("last_success_at_ms");
        subscription.remove("http_metadata");
        subscription.remove("description");
        subscription.remove("last_attempt_at_ms");
        subscription.remove("remote_request");
        subscription.remove("update_policy");
        atomic_replace(
            store.state_file(),
            &serde_json::to_vec(&v3).expect("encode v3 fixture"),
        )
        .expect("write v3 fixture");

        let state = store.load().expect("migrate v3");

        assert_eq!(state.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(state.subscriptions[0].id.0, "subscription");
        assert_eq!(state.providers[0].id.0, "provider");
        assert_eq!(state.nodes[0].id.0, "node");
        assert!(matches!(
            state.subscriptions[0].source,
            crate::domain::SubscriptionSource::Manual
        ));
        assert_eq!(state.subscriptions[0].last_success_at_ms, None);
        assert_eq!(state.subscriptions[0].http_metadata, None);
        assert_eq!(state.subscriptions[0].description, "");
        assert_eq!(state.subscriptions[0].last_attempt_at_ms, None);
        assert_eq!(state.subscriptions[0].remote_request, None);
        assert_eq!(
            state.subscriptions[0].update_policy,
            crate::domain::SubscriptionUpdatePolicy::manual()
        );
        assert_eq!(state.active_subscription_id, None);
        assert_eq!(state.active_configuration_generation, 0);
        assert!(pre_migration_backup_path(store.state_file()).exists());
        remove_test_files(&store);
    }

    #[test]
    fn migrates_v4_remote_subscription_to_current_without_changing_identity_or_references() {
        let store = unique_test_store();
        let mut v4 = serde_json::to_value(StoredStateV9::from(&valid_state_with_pool_and_route()))
            .expect("encode current fixture");
        v4["schema_version"] = serde_json::json!(4);
        let object = v4.as_object_mut().expect("state object");
        object.remove("active_subscription_id");
        object.remove("active_configuration_generation");
        let subscription = v4["subscriptions"][0]
            .as_object_mut()
            .expect("subscription object");
        subscription.insert(
            "source".to_owned(),
            serde_json::json!({"kind":"remote","url":"https://example.invalid/sub"}),
        );
        subscription.remove("description");
        subscription.remove("last_attempt_at_ms");
        subscription.remove("remote_request");
        subscription.remove("update_policy");
        atomic_replace(
            store.state_file(),
            &serde_json::to_vec(&v4).expect("encode v4 fixture"),
        )
        .expect("write v4 fixture");

        let state = store.load().expect("migrate v4");

        assert_eq!(state.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(state.subscriptions[0].id.0, "subscription");
        assert_eq!(state.providers[0].subscription_id.0, "subscription");
        assert_eq!(state.nodes[0].provider_id.0, "provider");
        assert_eq!(state.pools[0].sources[0].provider_id.0, "provider");
        assert_eq!(state.active_subscription_id, None);
        assert_eq!(state.active_configuration_generation, 0);
        assert_eq!(
            state.subscriptions[0].remote_request,
            Some(crate::domain::RemoteRequestOptions::default_remote())
        );
        assert_eq!(
            state.subscriptions[0].update_policy,
            crate::domain::SubscriptionUpdatePolicy::default_remote()
        );
        assert!(pre_migration_backup_path(store.state_file()).exists());
        let migrated = fs::read(store.state_file()).expect("read migrated state");
        assert_eq!(store.load().expect("reload migrated state"), state);
        assert_eq!(
            fs::read(store.state_file()).expect("read stable state"),
            migrated
        );
        remove_test_files(&store);
    }

    #[test]
    fn v6_round_trips_exact_subscription_document_without_debug_disclosure() {
        let store = unique_test_store();
        let mut state = valid_state();
        state.subscriptions[0].document = Some(crate::domain::SubscriptionDocument {
            format: crate::domain::SubscriptionDocumentFormat::Yaml,
            content: "# retained\nproxies: []\nsecret: fixture-only".to_owned(),
            local_override: false,
        });

        store.save(&state).expect("save v6 document");
        assert_eq!(store.load().expect("reload v6 document"), state);
        let rendered = format!("{:?}", store.load().expect("load for debug"));
        assert!(rendered.contains("[redacted]"));
        assert!(!rendered.contains("fixture-only"));
        remove_test_files(&store);
    }

    #[test]
    fn migrates_v5_to_v6_with_unavailable_document_and_preserved_identity() {
        let store = unique_test_store();
        let mut original = valid_state_with_pool_and_route();
        original.active_subscription_id = Some(original.subscriptions[0].id.clone());
        original.active_configuration_generation = 7;
        let mut v5 =
            serde_json::to_value(StoredStateV9::from(&original)).expect("encode current fixture");
        v5["schema_version"] = serde_json::json!(5);
        v5["subscriptions"][0]
            .as_object_mut()
            .expect("subscription object")
            .remove("document");
        let bytes = serde_json::to_vec(&v5).expect("encode v5 fixture");
        atomic_replace(store.state_file(), &bytes).expect("write v5 fixture");

        let migrated = store.load().expect("migrate v5");

        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(migrated.subscriptions[0].id, original.subscriptions[0].id);
        assert_eq!(migrated.providers, original.providers);
        assert_eq!(migrated.nodes, original.nodes);
        assert_eq!(migrated.pools, original.pools);
        assert_eq!(migrated.routes, original.routes);
        assert_eq!(
            migrated.active_subscription_id,
            original.active_subscription_id
        );
        assert_eq!(migrated.active_configuration_generation, 7);
        assert_eq!(migrated.subscriptions[0].document, None);
        assert!(pre_migration_backup_path(store.state_file()).exists());
        assert_eq!(store.load().expect("reload v6"), migrated);
        remove_test_files(&store);
    }

    #[test]
    fn rejects_an_invalid_v1_migration_candidate_without_writing_it() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save current state");
        let before = fs::read(store.state_file()).expect("read current state");

        assert!(matches!(
            store.decode(include_bytes!(
                "../../tests/fixtures/state/v1-invalid-reference.json"
            )),
            Err(StateStoreError::MigrationFailed)
        ));
        assert_eq!(
            fs::read(store.state_file()).expect("read current state"),
            before
        );
        remove_test_files(&store);
    }

    #[test]
    fn preserves_an_unmappable_v2_snapshot_even_when_a_backup_exists() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save initial state");
        let mut updated = state.clone();
        updated.nodes[0].name = "Updated".to_owned();
        store.save(&updated).expect("create valid backup");
        let unmappable_v2 = serde_json::json!({
            "schema_version": 2,
            "subscriptions": [{"id":"subscription","name":"Test"}],
            "providers": [{"id":"provider","subscription_id":"subscription","name":"Default"}],
            "nodes": [{
                "id":"node","provider_id":"provider","name":"Node","protocol":"unsupported",
                "server":"example.invalid","port":443,
                "credentials":{"kind":"password","username":null,"password":"secret","cipher":null},
                "transport":"tcp","tls":null
            }],
            "pools": [],
            "routes": []
        });
        let bytes = serde_json::to_vec(&unmappable_v2).expect("encode v2 state");
        atomic_replace(store.state_file(), &bytes).expect("write unmappable v2 state");

        assert_eq!(store.load(), Err(StateStoreError::MigrationFailed));
        assert_eq!(
            fs::read(store.state_file()).expect("read preserved v2 state"),
            bytes
        );
        assert!(!corrupt_copy_path(store.state_file()).exists());
        remove_test_files(&store);
    }

    #[test]
    fn rejects_an_unapproved_v0_schema_without_writing_it() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save current state");
        let before = fs::read(store.state_file()).expect("read current state");
        let mut candidate = serde_json::to_value(StoredStateV9::from(&state))
            .expect("serialize unsupported candidate");
        candidate["schema_version"] = serde_json::json!(0);

        assert_eq!(
            store.decode(&serde_json::to_vec(&candidate).expect("encode candidate")),
            Err(StateStoreError::UnsupportedSchemaVersion)
        );
        assert_eq!(
            fs::read(store.state_file()).expect("read current state"),
            before
        );
        remove_test_files(&store);
    }

    #[test]
    fn recovers_from_a_corrupt_current_snapshot_using_a_valid_backup() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save state");
        let mut replacement = state.clone();
        replacement.nodes[0].name = "Replacement".to_owned();
        store.save(&replacement).expect("create backup");
        fs::write(store.state_file(), b"not json").expect("corrupt current snapshot");

        let recovered = store.load().expect("recover backup");
        assert_ne!(recovered.state_epoch, state.state_epoch);
        let mut expected = state.clone();
        expected.state_epoch = recovered.state_epoch.clone();
        assert_eq!(recovered, expected);
        assert_eq!(store.load().expect("stable recovery epoch"), recovered);
        assert!(corrupt_copy_path(store.state_file()).exists());
        assert!(
            fs::read_to_string(store.state_file())
                .expect("read restored state")
                .contains("Test node")
        );
        remove_test_files(&store);
    }

    #[test]
    fn recovers_a_valid_backup_when_the_current_snapshot_is_missing() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save state");
        fs::rename(store.state_file(), backup_path(store.state_file())).expect("leave only backup");

        let recovered = store.load().expect("recover missing current");
        assert_ne!(recovered.state_epoch, state.state_epoch);
        let mut expected = state.clone();
        expected.state_epoch = recovered.state_epoch.clone();
        assert_eq!(recovered, expected);
        assert_eq!(store.load().expect("stable recovery epoch"), recovered);
        assert!(store.state_file().exists());
        remove_test_files(&store);
    }

    #[test]
    fn rejects_corrupt_state_when_no_valid_backup_exists() {
        let store = unique_test_store();
        atomic_replace(store.state_file(), b"not json").expect("write corrupt state");
        atomic_replace(&backup_path(store.state_file()), b"also not json")
            .expect("write corrupt backup");

        assert_eq!(store.load(), Err(StateStoreError::NoValidBackup));
        remove_test_files(&store);
    }

    #[test]
    fn recovers_from_a_missing_schema_version_using_a_valid_backup() {
        let store = unique_test_store();
        let state = valid_state();
        store.save(&state).expect("save original");
        let mut replacement = state.clone();
        replacement.nodes[0].name = "Replacement".to_owned();
        store.save(&replacement).expect("create backup");
        atomic_replace(store.state_file(), br#"{"subscriptions":[]}"#)
            .expect("write malformed state");

        let recovered = store.load().expect("recover backup");
        assert_ne!(recovered.state_epoch, state.state_epoch);
        let mut expected = state.clone();
        expected.state_epoch = recovered.state_epoch.clone();
        assert_eq!(recovered, expected);
        assert_eq!(store.load().expect("stable recovery epoch"), recovered);
        remove_test_files(&store);
    }

    #[test]
    fn rejects_an_unsupported_schema_without_replacing_the_snapshot() {
        let store = unique_test_store();
        let mut document = serde_json::to_value(StoredStateV9::from(&valid_state()))
            .expect("serialize future schema");
        document["schema_version"] = serde_json::json!(99);
        let bytes = serde_json::to_vec(&document).expect("encode future schema");
        atomic_replace(store.state_file(), &bytes).expect("write future schema");

        assert_eq!(store.load(), Err(StateStoreError::UnsupportedSchemaVersion));
        assert_eq!(fs::read(store.state_file()).expect("read original"), bytes);
        remove_test_files(&store);
    }

    #[test]
    fn rejects_invalid_references_before_writing() {
        let store = unique_test_store();
        let mut state = valid_state();
        state.nodes[0].provider_id = ProviderId("missing".to_owned());

        assert_eq!(
            store.save(&state),
            Err(StateStoreError::InvalidState(
                StateValidationError::MissingProvider
            ))
        );
        assert!(!store.state_file().exists());
    }
}
