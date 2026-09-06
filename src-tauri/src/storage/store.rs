use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::domain::{
    AppState, CURRENT_SCHEMA_VERSION, NodePool, Provider, ProxyNode, RoutePolicy,
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
    fn save(&self, state: &AppState) -> Result<(), StateStoreError>;
}

#[derive(Clone, Debug)]
pub struct JsonStateStore {
    state_file: PathBuf,
}

impl JsonStateStore {
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
        let stored = serde_json::from_value::<StoredStateV6>(migrated)
            .map_err(|_| StateStoreError::InvalidStoredState)?;
        let state = validate_state(AppState::try_from(stored)?)?;
        Ok((state, was_migrated))
    }

    fn load_backup(&self) -> Result<AppState, StateStoreError> {
        let contents = read_snapshot(&backup_path(&self.state_file))?;
        self.decode(&contents).map(|(state, _)| state)
    }

    fn write_current_without_backup(&self, state: &AppState) -> Result<(), StateStoreError> {
        validate_state(state.clone())?;
        let contents = serde_json::to_vec_pretty(&StoredStateV6::from(state))
            .map_err(|_| StateStoreError::SerializationFailed)?;
        atomic_replace(&self.state_file, &contents)
    }

    pub(crate) fn has_snapshot_or_backup(&self) -> Result<bool, StateStoreError> {
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
        validate_state(state.clone())?;
        let stored = StoredStateV6::from(state);
        let contents =
            serde_json::to_vec_pretty(&stored).map_err(|_| StateStoreError::SerializationFailed)?;

        if self.state_file.exists() {
            copy_for_backup(&self.state_file, &backup_path(&self.state_file))?;
        }
        atomic_replace(&self.state_file, &contents)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredStateV6 {
    schema_version: u32,
    default_target: crate::domain::RouteTarget,
    active_subscription_id: Option<crate::domain::SubscriptionId>,
    active_configuration_generation: u64,
    subscriptions: Vec<Subscription>,
    providers: Vec<Provider>,
    nodes: Vec<ProxyNode>,
    pools: Vec<NodePool>,
    routes: Vec<RoutePolicy>,
}

impl From<&AppState> for StoredStateV6 {
    fn from(state: &AppState) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            default_target: state.default_target.clone(),
            active_subscription_id: state.active_subscription_id.clone(),
            active_configuration_generation: state.active_configuration_generation,
            subscriptions: state.subscriptions.clone(),
            providers: state.providers.clone(),
            nodes: state.nodes.clone(),
            pools: state.pools.clone(),
            routes: state.routes.clone(),
        }
    }
}

impl TryFrom<StoredStateV6> for AppState {
    type Error = StateStoreError;

    fn try_from(stored: StoredStateV6) -> Result<Self, Self::Error> {
        if stored.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(StateStoreError::UnsupportedSchemaVersion);
        }
        Ok(Self {
            schema_version: stored.schema_version,
            default_target: stored.default_target,
            active_subscription_id: stored.active_subscription_id,
            active_configuration_generation: stored.active_configuration_generation,
            subscriptions: stored.subscriptions,
            providers: stored.providers,
            nodes: stored.nodes,
            pools: stored.pools,
            routes: stored.routes,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateStoreError {
    InvalidState(StateValidationError),
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
            Self::InvalidState(error) => return write!(formatter, "invalid state: {error}"),
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

        assert_eq!(store.load().expect("migrate v1"), state);
        assert!(pre_migration_backup_path(store.state_file()).exists());
        let migrated_bytes = fs::read(store.state_file()).expect("read migrated state");
        assert_eq!(store.load().expect("reload v1"), state);
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
            SelectionPolicy::Manual { selected_node_id: Some(NodeId(ref id)) } if id == "node-preserved"
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
        let mut v3 = serde_json::to_value(StoredStateV6::from(&valid_state_with_pool_and_route()))
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

        assert_eq!(state.schema_version, 6);
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
        let mut v4 = serde_json::to_value(StoredStateV6::from(&valid_state_with_pool_and_route()))
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

        assert_eq!(state.schema_version, 6);
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
            serde_json::to_value(StoredStateV6::from(&original)).expect("encode current fixture");
        v5["schema_version"] = serde_json::json!(5);
        v5["subscriptions"][0]
            .as_object_mut()
            .expect("subscription object")
            .remove("document");
        let bytes = serde_json::to_vec(&v5).expect("encode v5 fixture");
        atomic_replace(store.state_file(), &bytes).expect("write v5 fixture");

        let migrated = store.load().expect("migrate v5");

        assert_eq!(migrated.schema_version, 6);
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
        let mut candidate = serde_json::to_value(StoredStateV6::from(&state))
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

        assert_eq!(store.load().expect("recover backup"), state);
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

        assert_eq!(store.load().expect("recover missing current"), state);
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

        assert_eq!(store.load().expect("recover backup"), state);
        remove_test_files(&store);
    }

    #[test]
    fn rejects_an_unsupported_schema_without_replacing_the_snapshot() {
        let store = unique_test_store();
        let mut document = serde_json::to_value(StoredStateV6::from(&valid_state()))
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
