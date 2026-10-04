//! A–H: tests protect local saves, delayed form/task writes and truthful runtime feedback.
use super::*;
use crate::application::runtime_snapshot::*;
use crate::domain::*;
use std::{fs, path::PathBuf};

struct Fixture {
    root: PathBuf,
    store: JsonStateStore,
    snapshots: SnapshotService,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("veyra-p1-02-{:?}", StateEpoch::fresh().unwrap()));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        let snapshots = SnapshotService::new(store.clone(), StateAccessGate::default());
        snapshots.snapshot().unwrap();
        Self {
            root,
            store,
            snapshots,
        }
    }
    fn bytes(&self) -> Vec<u8> {
        fs::read(self.root.join("state.json")).unwrap()
    }
    fn state(&self) -> AppState {
        self.snapshots.snapshot().unwrap()
    }
    fn profile(&self) -> ProfileService {
        ProfileService::new(self.snapshots.clone())
    }
    fn selection(&self) -> SelectionService {
        SelectionService::new(self.snapshots.clone())
    }
    fn fail_writes(&self) {
        fs::create_dir(self.root.join("state.tmp")).unwrap();
    }
    fn seed_pool(&self) -> AppState {
        let mut state = self.state();
        state.subscriptions.push(subscription("s"));
        state.providers.push(Provider {
            id: ProviderId("p".into()),
            subscription_id: SubscriptionId("s".into()),
            name: "P".into(),
        });
        state.nodes.push(ProxyNode {
            id: NodeId("n".into()),
            provider_id: ProviderId("p".into()),
            name: "N".into(),
            protocol: ProxyProtocol::Shadowsocks,
            server: "example.invalid".into(),
            port: 443,
            options: ProtocolOptions::Shadowsocks {
                method: "aes-128-gcm".into(),
                password: "secret-fixture-do-not-leak".into(),
            },
            transport: None,
            tls: None,
        });
        state.pools.push(NodePool {
            id: PoolId("pool".into()),
            name: "Pool".into(),
            kind: PoolKind::Custom,
            enabled: true,
            sources: vec![PoolSource {
                provider_id: ProviderId("p".into()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
            },
        });
        self.store.commit(&state).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
fn subscription(id: &str) -> Subscription {
    Subscription {
        id: SubscriptionId(id.into()),
        name: "Fixture".into(),
        description: String::new(),
        source: SubscriptionSource::Manual,
        skipped_unsupported_nodes: 0,
        last_success_at_ms: None,
        last_attempt_at_ms: None,
        http_metadata: None,
        remote_request: None,
        update_policy: SubscriptionUpdatePolicy::manual(),
        document: None,
    }
}
fn patch(json: &str) -> ProfilePatch {
    serde_json::from_str(json).unwrap()
}
fn representative() -> Profile {
    let mut p = Profile::default();
    p.dns.split = true;
    p.dns.proxy = "9.9.9.9".into();
    p.dns.direct_extras = vec![DnsUpstream {
        server: "8.8.8.8".into(),
        protocol: DnsTransport::Tcp,
        port: 53,
    }];
    p.routing.custom = Some(CustomRouting {
        name: Some("User rules".into()),
        enabled: true,
        rules: vec![CustomRoutingRule {
            matcher: DomainMatcher::DomainSuffix,
            value: "example.invalid".into(),
            target: RouteTarget::Direct,
        }],
    });
    p.tun.stack = TunStack::Gvisor;
    p.tun.mtu = 1400;
    p
}

#[test]
fn missing_fields_preserve_unedited_dns_routing_and_tun() {
    let before = representative();
    let next = before
        .patched(patch(
            r#"{"dns":{"direct":"4.4.4.4"},"tun":{"autoRedirect":true}}"#,
        ))
        .unwrap();
    assert_eq!(next.dns.proxy, before.dns.proxy);
    assert_eq!(next.dns.direct_extras, before.dns.direct_extras);
    assert_eq!(next.routing, before.routing);
    assert_eq!(next.tun.stack, before.tun.stack);
    assert_eq!(next.tun.mtu, before.tun.mtu);
    assert!(next.tun.auto_redirect);
    assert_eq!(next.dns.direct, "4.4.4.4");
}
#[test]
fn nullable_fields_clear_and_nested_values_merge() {
    let before = representative();
    let next = before
        .patched(patch(r#"{"routing":{"custom":{"name":null}}}"#))
        .unwrap();
    assert_eq!(next.routing.custom.as_ref().unwrap().name, None);
    assert_eq!(
        next.routing.custom.as_ref().unwrap().rules,
        before.routing.custom.as_ref().unwrap().rules
    );
    assert!(next.routing.custom.as_ref().unwrap().enabled);
    assert_eq!(
        next.patched(patch(r#"{"routing":{"custom":null}}"#))
            .unwrap()
            .routing
            .custom,
        None
    );
    assert_eq!(
        Profile::default()
            .patched(patch(r#"{"routing":{"custom":{"name":"new"}}}"#))
            .unwrap()
            .routing
            .custom
            .unwrap()
            .name
            .as_deref(),
        Some("new")
    );
}
#[test]
fn illegal_nulls_have_exact_field_paths_and_leave_candidate_untouched() {
    for (json, field) in [
        (r#"{"ipv6":null}"#, FieldPath::Ipv6),
        (r#"{"dns":null}"#, FieldPath::Dns),
        (r#"{"dns":{"split":null}}"#, FieldPath::DnsSplit),
        (
            r#"{"dns":{"directExtras":null}}"#,
            FieldPath::DnsDirectExtras,
        ),
        (r#"{"routing":null}"#, FieldPath::Routing),
        (
            r#"{"routing":{"custom":{"rules":null}}}"#,
            FieldPath::RoutingCustomRules,
        ),
        (r#"{"tun":null}"#, FieldPath::Tun),
        (r#"{"tun":{"mtu":null}}"#, FieldPath::TunMtu),
    ] {
        let before = representative();
        let error = before.patched(patch(json)).unwrap_err();
        assert_eq!(error.code(), AppErrorCode::Validation);
        assert_eq!(error.field(), Some(field));
        assert_eq!(before, representative());
    }
}
#[test]
fn arrays_replace_including_empty_arrays_without_merging_old_entries() {
    let before = representative();
    let next = before.patched(patch(r#"{"dns":{"directExtras":[{"server":"1.0.0.1","protocol":"udp","port":5353}]},"routing":{"custom":{"rules":[]}}}"#)).unwrap();
    assert_eq!(next.dns.direct_extras.len(), 1);
    assert_eq!(next.dns.direct_extras[0].server, "1.0.0.1");
    assert!(next.routing.custom.as_ref().unwrap().rules.is_empty());
    assert!(
        next.patched(patch(r#"{"dns":{"directExtras":[]}}"#))
            .unwrap()
            .dns
            .direct_extras
            .is_empty()
    );
}
#[test]
fn unknown_patch_fields_are_rejected_at_each_object_boundary() {
    for json in [
        r#"{"secret":"fixture"}"#,
        r#"{"dns":{"secret":"fixture"}}"#,
        r#"{"tun":{"secret":"fixture"}}"#,
        r#"{"routing":{"custom":{"secret":"fixture"}}}"#,
    ] {
        assert!(serde_json::from_str::<ProfilePatch>(json).is_err());
    }
}
#[test]
fn invalid_profile_values_fail_without_saving_or_advancing_versions() {
    let f = Fixture::new();
    let old = f.state();
    let bytes = f.bytes();
    for (json, field) in [
        (r#"{"dns":{"directPort":0}}"#, FieldPath::DnsDirectPort),
        (r#"{"tun":{"mtu":1}}"#, FieldPath::TunMtu),
        (r#"{"tun":{"tcpMss":1500}}"#, FieldPath::TunTcpMss),
    ] {
        assert_eq!(
            f.profile()
                .patch(old.config_version(), patch(json))
                .unwrap_err()
                .field(),
            Some(field)
        );
        assert_eq!(f.bytes(), bytes);
        assert_eq!(f.state(), old);
    }
}
#[test]
fn profile_saves_only_config_revision_and_returns_saved_only() {
    let f = Fixture::new();
    let old = f.state();
    let outcome = f
        .profile()
        .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
        .unwrap();
    let saved = f.state();
    assert_eq!(saved.state_epoch, old.state_epoch);
    assert_eq!(saved.config_revision, 1);
    assert_eq!(saved.selection_revision, 0);
    assert_eq!(outcome.version, saved.config_version());
    assert_eq!(outcome.effect, ApplyEffect::SavedOnly);
    assert!(saved.profile.ipv6);
    assert_eq!(
        f.profile()
            .patch(saved.config_version(), patch("{}"))
            .unwrap()
            .version,
        saved.config_version()
    );
}
#[test]
fn failed_profile_write_preserves_old_bytes_epoch_versions_and_facts() {
    let f = Fixture::new();
    let old = f.state();
    let bytes = f.bytes();
    f.fail_writes();
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.state(), old);
}
#[test]
fn failed_replace_does_not_invalidate_existing_forms() {
    let f = Fixture::new();
    let old = f.state();
    let bytes = f.bytes();
    f.fail_writes();
    assert_eq!(
        f.snapshots
            .replace(old.version(), AppState::empty())
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.state(), old);
}
#[test]
fn failed_append_is_atomic_and_leaves_no_partial_subscriptions() {
    let f = Fixture::new();
    let old = f.state();
    let bytes = f.bytes();
    f.fail_writes();
    assert_eq!(
        f.snapshots
            .append(
                old.config_version(),
                SubscriptionAppend {
                    subscriptions: vec![subscription("new")],
                    ..Default::default()
                }
            )
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.state(), old);
}
#[test]
fn failed_selection_write_preserves_confirmed_choice_and_both_versions() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let bytes = f.bytes();
    f.fail_writes();
    assert_eq!(
        f.selection()
            .select_manual(
                old.selection_version(),
                PoolId("pool".into()),
                Some(NodeId("n".into()))
            )
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.state(), old);
}
#[test]
fn backup_replace_failure_leaves_current_valid_snapshot() {
    let f = Fixture::new();
    let old = f.state();
    let bytes = f.bytes();
    fs::create_dir(f.root.join("state.json.bak")).unwrap();
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.state(), old);
}
#[test]
fn replace_discards_import_versions_and_resets_both_counters_to_zero() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let mut imported = old.clone();
    imported.config_revision = u64::MAX;
    imported.selection_revision = u64::MAX;
    let outcome = f.snapshots.replace(old.version(), imported).unwrap();
    assert_ne!(outcome.value.state_epoch, old.state_epoch);
    assert_eq!(outcome.value.config_revision, 0);
    assert_eq!(outcome.value.selection_revision, 0);
    assert_eq!(f.state(), outcome.value);
    assert_eq!(outcome.effect, ApplyEffect::SavedOnly);
}
#[test]
fn old_forms_tasks_and_legacy_candidates_cannot_write_across_epoch() {
    let f = Fixture::new();
    let old = f.seed_pool();
    f.snapshots.replace(old.version(), old.clone()).unwrap();
    let bytes = f.bytes();
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
    assert_eq!(
        f.selection()
            .select_manual(
                old.selection_version(),
                PoolId("pool".into()),
                Some(NodeId("n".into()))
            )
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
    assert_eq!(
        f.snapshots
            .append(old.config_version(), SubscriptionAppend::default())
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
    assert_eq!(
        f.store.commit(&old).unwrap_err(),
        crate::storage::StateStoreError::RevisionConflict
    );
    assert_eq!(f.bytes(), bytes);
}
#[test]
fn old_revision_and_same_number_in_different_epoch_are_both_conflicts() {
    let f = Fixture::new();
    let old = f.state();
    f.profile()
        .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
        .unwrap();
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch("{}"))
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
    let a = old.config_version().0;
    let b = StateVersion {
        epoch: StateEpoch::fresh().unwrap(),
        revision: a.revision,
    };
    assert_eq!(
        a.compare(&b).unwrap_err().code(),
        AppErrorCode::RevisionConflict
    );
    assert_eq!(a.compare(&a).unwrap(), std::cmp::Ordering::Equal);
}
#[test]
fn append_preserves_epoch_and_increments_config_only() {
    let f = Fixture::new();
    let old = f.state();
    let outcome = f
        .snapshots
        .append(
            old.config_version(),
            SubscriptionAppend {
                subscriptions: vec![subscription("new")],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(outcome.value.state_epoch, old.state_epoch);
    assert_eq!(outcome.value.config_revision, 1);
    assert_eq!(outcome.value.selection_revision, 0);
    assert_eq!(outcome.value.subscriptions[0].id.0, "new");
    assert_eq!(
        f.snapshots
            .append(
                outcome.value.config_version(),
                SubscriptionAppend {
                    subscriptions: vec![subscription("new")],
                    ..Default::default()
                }
            )
            .unwrap_err()
            .code(),
        AppErrorCode::Validation
    );
    assert_eq!(f.state(), outcome.value);
}
#[test]
fn selection_and_config_expected_versions_are_independent() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let chosen = f
        .selection()
        .select_manual(
            old.selection_version(),
            PoolId("pool".into()),
            Some(NodeId("n".into())),
        )
        .unwrap();
    assert_eq!(chosen.version.0.revision, 1);
    assert_eq!(f.state().config_version(), old.config_version());
    f.profile()
        .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
        .unwrap();
    f.selection()
        .select_manual(chosen.version, PoolId("pool".into()), None)
        .unwrap();
    assert_eq!(f.state().config_revision, old.config_revision + 1);
    assert_eq!(f.state().selection_revision, 2);
    assert_eq!(
        f.selection()
            .select_manual(old.selection_version(), PoolId("pool".into()), None)
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
}
#[test]
fn replace_checks_both_expected_versions_before_overwriting_choices() {
    let f = Fixture::new();
    let old = f.seed_pool();
    f.selection()
        .select_manual(
            old.selection_version(),
            PoolId("pool".into()),
            Some(NodeId("n".into())),
        )
        .unwrap();
    assert_eq!(
        f.snapshots
            .replace(old.version(), AppState::empty())
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
}
#[test]
fn legacy_commit_infers_selection_only_and_rejects_stale_revision() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let mut next = old.clone();
    next.pools[0].selection = SelectionPolicy::Manual {
        selected_node_id: Some(NodeId("n".into())),
    };
    let saved = f.store.commit(&next).unwrap();
    assert_eq!(saved.config_version(), old.config_version());
    assert_eq!(saved.selection_revision, 1);
    assert_eq!(
        f.store.commit(&next).unwrap_err(),
        crate::storage::StateStoreError::RevisionConflict
    );
}
#[test]
fn v6_migration_preserves_legacy_identity_and_persists_new_epoch_once() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let mut doc = serde_json::to_value(&old).unwrap();
    doc["schema_version"] = 6.into();
    let object = doc.as_object_mut().unwrap();
    for key in [
        "state_epoch",
        "config_revision",
        "selection_revision",
        "profile",
        "app_config",
    ] {
        object.remove(key);
    }
    let bytes = serde_json::to_vec(&doc).unwrap();
    fs::write(f.root.join("state.json"), &bytes).unwrap();
    let migrated = f.store.load().unwrap();
    let persisted = f.bytes();
    assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(migrated.config_revision, 0);
    assert_eq!(migrated.selection_revision, 0);
    assert_eq!(migrated.subscriptions, old.subscriptions);
    assert_eq!(migrated.nodes, old.nodes);
    assert_eq!(migrated.pools, old.pools);
    assert_eq!(migrated.profile, Profile::default());
    assert!(migrated.app_config.check_updates_on_start);
    assert_eq!(
        fs::read(f.root.join("state.json.pre-migration")).unwrap(),
        bytes
    );
    assert_eq!(f.store.load().unwrap(), migrated);
    assert_eq!(f.bytes(), persisted);
}
#[test]
fn recovery_from_backup_changes_epoch_and_rejects_pre_recovery_task() {
    let f = Fixture::new();
    let old = f.seed_pool();
    f.profile()
        .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
        .unwrap();
    fs::write(f.root.join("state.json"), b"corrupt").unwrap();
    let recovered = f.state();
    assert_ne!(recovered.state_epoch, old.state_epoch);
    assert_eq!(recovered.config_revision, 0);
    assert_eq!(recovered.selection_revision, 0);
    assert_eq!(recovered.nodes, old.nodes);
    assert_eq!(recovered.profile, old.profile);
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch("{}"))
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
}
#[test]
fn busy_shared_legacy_gate_prevents_parallel_snapshot_writes() {
    let f = Fixture::new();
    let old = f.state();
    let _guard = f.snapshots.gate.try_lock().unwrap();
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch("{}"))
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
}
fn history(state: &AppState) -> LastSuccessfulVersion {
    LastSuccessfulVersion {
        config: state.config_version(),
        selection: state.selection_version(),
    }
}
#[test]
fn stopped_with_last_success_never_reports_ready_or_applied() {
    let f = Fixture::new();
    let old = f.state();
    let snapshot = RuntimeSnapshot::from_owner(
        old.config_version(),
        &RuntimeFacts {
            current: RuntimeState::Stopped,
            last_successful: Some(history(&old)),
        },
    );
    assert_eq!(snapshot.status, RuntimeStatus::Stopped);
    assert_eq!(snapshot.instance_id, None);
    assert_eq!(snapshot.applied_version, None);
    assert_eq!(snapshot.last_successful_version, Some(old.config_version()));
    assert!(snapshot.has_unapplied_config());
}
#[test]
fn ready_old_instance_remains_visible_after_business_replacement() {
    let f = Fixture::new();
    let old = f.state();
    let new = f
        .snapshots
        .replace(old.version(), old.clone())
        .unwrap()
        .value;
    let snapshot = RuntimeSnapshot::from_owner(
        new.config_version(),
        &RuntimeFacts {
            current: RuntimeState::Ready {
                instance_id: InstanceId("owned-instance".into()),
                applied_version: old.config_version(),
            },
            last_successful: Some(history(&old)),
        },
    );
    assert_eq!(snapshot.status, RuntimeStatus::Ready);
    assert_eq!(
        snapshot.instance_id,
        Some(InstanceId("owned-instance".into()))
    );
    assert_eq!(snapshot.applied_version, Some(old.config_version()));
    assert!(snapshot.has_unapplied_config());
}
#[test]
fn manifest_failure_keeps_live_version_and_previous_last_success_distinct() {
    let f = Fixture::new();
    let old = f.state();
    f.profile()
        .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
        .unwrap();
    let new = f.state();
    let snapshot = RuntimeSnapshot::from_owner(
        new.config_version(),
        &RuntimeFacts {
            current: RuntimeState::Ready {
                instance_id: InstanceId("new-instance".into()),
                applied_version: new.config_version(),
            },
            last_successful: Some(history(&old)),
        },
    );
    assert_eq!(snapshot.applied_version, Some(new.config_version()));
    assert_eq!(snapshot.last_successful_version, Some(old.config_version()));
    assert!(!snapshot.has_unapplied_config());
}
#[test]
fn recovery_starting_and_failed_states_do_not_inherit_history_ready() {
    let f = Fixture::new();
    let old = f.state();
    for (current, status) in [
        (
            RuntimeState::Starting {
                instance_id: InstanceId("start".into()),
            },
            RuntimeStatus::Starting,
        ),
        (
            RuntimeState::Recovering { instance: None },
            RuntimeStatus::Recovering,
        ),
        (
            RuntimeState::Failed { instance_id: None },
            RuntimeStatus::Failed,
        ),
    ] {
        let snapshot = RuntimeSnapshot::from_owner(
            old.config_version(),
            &RuntimeFacts {
                current,
                last_successful: Some(history(&old)),
            },
        );
        assert_eq!(snapshot.status, status);
        assert_eq!(snapshot.applied_version, None);
    }
}
#[test]
fn selection_changes_do_not_mark_applied_config_stale() {
    let f = Fixture::new();
    let old = f.seed_pool();
    f.selection()
        .select_manual(
            old.selection_version(),
            PoolId("pool".into()),
            Some(NodeId("n".into())),
        )
        .unwrap();
    let snapshot = RuntimeSnapshot::from_owner(
        f.state().config_version(),
        &RuntimeFacts {
            current: RuntimeState::Ready {
                instance_id: InstanceId("instance".into()),
                applied_version: old.config_version(),
            },
            last_successful: Some(history(&old)),
        },
    );
    assert!(!snapshot.has_unapplied_config());
}
#[test]
fn error_codes_messages_fields_and_details_are_stable_and_secret_free() {
    for code in [
        AppErrorCode::Validation,
        AppErrorCode::RevisionConflict,
        AppErrorCode::UnavailableOnPlatform,
        AppErrorCode::PermissionDenied,
        AppErrorCode::PortInUse,
        AppErrorCode::KernelUnavailable,
        AppErrorCode::Timeout,
        AppErrorCode::Cancelled,
        AppErrorCode::DownloadFailed,
        AppErrorCode::StorageFailed,
        AppErrorCode::ApplyFailed,
    ] {
        let error = AppError::new(code).with_detail(ErrorDetail::Write);
        let serialized = serde_json::to_value(&error).unwrap();
        assert_eq!(serialized["code"], format!("{code:?}"));
        assert_eq!(serialized["message"], code.message());
        assert_eq!(serialized["detail"], "Write");
        assert!(serialized.get("field").is_none());
    }
    let f = Fixture::new();
    let old = f.seed_pool();
    f.fail_writes();
    let error = f
        .profile()
        .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
        .unwrap_err();
    let rendered = format!(
        "{error} {error:?} {}",
        serde_json::to_string(&error).unwrap()
    );
    assert!(!rendered.contains("secret-fixture-do-not-leak"));
    assert!(!rendered.contains(&f.root.to_string_lossy().to_string()));
    let validation = representative()
        .patched(patch(r#"{"dns":{"direct":null}}"#))
        .unwrap_err();
    assert_eq!(
        serde_json::to_value(validation).unwrap()["field"],
        "profile.dns.direct"
    );
}

#[test]
fn exhausted_config_revision_fails_without_wrapping_or_writing() {
    let f = Fixture::new();
    let mut old = f.state();
    old.config_revision = MAX_SAFE_INTEGER;
    f.store.save(&old).unwrap();
    let bytes = f.bytes();
    assert_eq!(
        f.profile()
            .patch(old.config_version(), patch(r#"{"ipv6":true}"#))
            .unwrap_err()
            .code(),
        AppErrorCode::StorageFailed
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(f.state(), old);
}
#[test]
fn dangling_profile_rule_is_rejected_before_persistence() {
    let f = Fixture::new();
    let old = f.state();
    let bytes = f.bytes();
    let error = f.profile().patch(old.config_version(), patch(r#"{"routing":{"custom":{"rules":[{"matcher":"domain","value":"example.invalid","target":{"kind":"pool","pool_id":"missing"}}]}}}"#)).unwrap_err();
    assert_eq!(error.field(), Some(FieldPath::RoutingCustomRules));
    assert_eq!(f.bytes(), bytes);
}
#[test]
fn recovering_live_instance_retains_its_applied_identity_without_claiming_ready() {
    let f = Fixture::new();
    let old = f.state();
    let snapshot = RuntimeSnapshot::from_owner(
        old.config_version(),
        &RuntimeFacts {
            current: RuntimeState::Recovering {
                instance: Some(AppliedInstance {
                    instance_id: InstanceId("cleanup-owner".into()),
                    applied_version: old.config_version(),
                }),
            },
            last_successful: None,
        },
    );
    assert_eq!(snapshot.status, RuntimeStatus::Recovering);
    assert_eq!(
        snapshot.instance_id,
        Some(InstanceId("cleanup-owner".into()))
    );
    assert_eq!(snapshot.applied_version, Some(old.config_version()));
    assert_eq!(snapshot.last_successful_version, None);
}

// Protect visual persistence independently of Runtime/selection facts.
#[test]
fn visual_save_reload_conflict_noop_and_versions() {
    let fixture = Fixture::new();
    let before = fixture.snapshots.snapshot().unwrap();
    let service = super::DesktopPreferencesService::new(fixture.snapshots.clone());
    let visual = crate::domain::DesktopVisualPreferences {
        theme_mode: crate::domain::DesktopThemeMode::Dark,
        language: crate::domain::DesktopLanguage::TraditionalChinese,
        sidebar_collapsed: true,
        global_radius: 8,
        background_opacity: 40,
        background_blur: 20,
        ..Default::default()
    };
    let saved = service
        .save(before.config_version(), visual.clone())
        .unwrap();
    assert_eq!(saved.effect, ApplyEffect::SavedOnly);
    assert_eq!(saved.value.config_revision, before.config_revision + 1);
    assert_eq!(saved.value.selection_version(), before.selection_version());
    assert_eq!(saved.value.state_epoch, before.state_epoch);
    assert_eq!(
        fixture.snapshots.snapshot().unwrap().app_config.visual,
        visual
    );
    assert_eq!(
        service
            .save(before.config_version(), visual.clone())
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
    let noop = service.save(saved.version.clone(), visual).unwrap();
    assert_eq!(noop.version, saved.version);
}

// Protect real v7 file migration without inventing an edit or replacing snapshot identity.
#[test]
fn v7_visual_migration_persists_defaults_without_advancing_versions() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let mut doc = serde_json::to_value(&old).unwrap();
    doc["schema_version"] = 7.into();
    doc["app_config"].as_object_mut().unwrap().remove("visual");
    let bytes = serde_json::to_vec(&doc).unwrap();
    fs::write(f.root.join("state.json"), &bytes).unwrap();
    let migrated = f.store.load().unwrap();
    assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
    assert_eq!(migrated.state_epoch, old.state_epoch);
    assert_eq!(migrated.config_version(), old.config_version());
    assert_eq!(migrated.selection_version(), old.selection_version());
    assert_eq!(migrated.pools, old.pools);
    assert_eq!(
        migrated.app_config.visual,
        DesktopVisualPreferences::default()
    );
    assert_eq!(
        fs::read(f.root.join("state.json.pre-migration")).unwrap(),
        bytes
    );
    let persisted = f.bytes();
    assert_eq!(f.store.load().unwrap(), migrated);
    assert_eq!(f.bytes(), persisted);
}
