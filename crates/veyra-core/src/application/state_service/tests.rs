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
                pending_node_id: None,
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
        icon: String::new(),
        icon_scale: 0,
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
        pending_node_id: None,
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

// 保护旧 schema9 无 pending 的逐字兼容、重启读取与两阶段原子版本语义。
#[test]
fn manual_pending_schema9_and_staged_versions() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let bytes = f.bytes();
    assert!(!String::from_utf8_lossy(&bytes).contains("pending_node_id"));
    let reopened = JsonStateStore::new(f.root.join("state.json")).unwrap();
    assert_eq!(reopened.load().unwrap(), old);
    assert_eq!(f.bytes(), bytes, "旧 v9 不应触发迁移写入");
    let service = f.selection();
    let pending = service
        .begin_manual_pending(
            old.selection_version(),
            PoolId("pool".into()),
            NodeId("n".into()),
        )
        .unwrap();
    let state = reopened.load().unwrap();
    assert_eq!(state.config_version(), old.config_version());
    assert_eq!(pending.version.0.revision, old.selection_revision + 1);
    assert_eq!(
        state.pools[0].selection,
        SelectionPolicy::Manual {
            selected_node_id: None,
            pending_node_id: Some(NodeId("n".into()))
        }
    );
    assert!(
        service
            .begin_manual_pending(
                pending.version.clone(),
                PoolId("pool".into()),
                NodeId("n".into())
            )
            .is_err()
    );
    for operation in [
        SelectionService::confirm_manual_pending,
        SelectionService::clear_manual_pending,
    ] {
        assert_eq!(
            operation(
                &service,
                old.selection_version(),
                PoolId("pool".into()),
                NodeId("n".into())
            )
            .unwrap_err()
            .code(),
            AppErrorCode::RevisionConflict
        );
        assert!(
            operation(
                &service,
                pending.version.clone(),
                PoolId("pool".into()),
                NodeId("other".into())
            )
            .is_err()
        );
        assert_eq!(reopened.load().unwrap(), state);
    }
    assert_eq!(
        service
            .begin_manual_pending(
                old.selection_version(),
                PoolId("pool".into()),
                NodeId("n".into())
            )
            .unwrap_err()
            .code(),
        AppErrorCode::RevisionConflict
    );
    let confirmed = service
        .confirm_manual_pending(
            pending.version.clone(),
            PoolId("pool".into()),
            NodeId("n".into()),
        )
        .unwrap();
    assert_eq!(confirmed.version.0.revision, old.selection_revision + 2);
    let state = reopened.load().unwrap();
    assert_eq!(state.config_version(), old.config_version());
    assert_eq!(
        state.pools[0].selection,
        SelectionPolicy::Manual {
            selected_node_id: Some(NodeId("n".into())),
            pending_node_id: None
        }
    );
    assert!(!String::from_utf8_lossy(&f.bytes()).contains("pending_node_id"));
    assert!(
        service
            .clear_manual_pending(pending.version, PoolId("pool".into()), NodeId("n".into()))
            .is_err()
    );
}

// 保护新 epoch、删除/过滤的意图撤销和保存失败不发布 pending。
#[test]
fn pending_clear_validation_epoch_and_atomic_failure() {
    let f = Fixture::new();
    let old = f.seed_pool();
    let service = f.selection();
    assert!(
        service
            .begin_manual_pending(
                old.selection_version(),
                PoolId("pool".into()),
                NodeId("missing".into())
            )
            .is_err()
    );
    assert_eq!(f.state(), old);
    f.fail_writes();
    assert!(
        service
            .begin_manual_pending(
                old.selection_version(),
                PoolId("pool".into()),
                NodeId("n".into())
            )
            .is_err()
    );
    assert_eq!(f.state(), old);
    fs::remove_dir(f.root.join("state.tmp")).unwrap();
    let pending = service
        .begin_manual_pending(
            old.selection_version(),
            PoolId("pool".into()),
            NodeId("n".into()),
        )
        .unwrap();
    let mut dangling = f.state();
    if let SelectionPolicy::Manual {
        pending_node_id, ..
    } = &mut dangling.pools[0].selection
    {
        *pending_node_id = Some(NodeId("missing".into()));
    }
    assert_eq!(
        dangling.validate(),
        Err(StateValidationError::InvalidSelection)
    );
    let cleared = service
        .clear_manual_pending(
            pending.version.clone(),
            PoolId("pool".into()),
            NodeId("n".into()),
        )
        .unwrap();
    assert_eq!(cleared.version.0.revision, old.selection_revision + 2);
    assert_eq!(f.state().config_version(), old.config_version());
    service
        .begin_manual_pending(cleared.version, PoolId("pool".into()), NodeId("n".into()))
        .unwrap();
    let before = f.state();
    let replacement = f
        .snapshots
        .replace(before.version(), before.clone())
        .unwrap()
        .value;
    assert_ne!(replacement.state_epoch, before.state_epoch);
    assert_eq!(replacement.selection_revision, 0);
    assert!(matches!(
        replacement.pools[0].selection,
        SelectionPolicy::Manual {
            pending_node_id: None,
            ..
        }
    ));
    assert!(
        service
            .confirm_manual_pending(
                before.selection_version(),
                PoolId("pool".into()),
                NodeId("n".into())
            )
            .is_err()
    );
    assert!(
        service
            .clear_manual_pending(
                before.selection_version(),
                PoolId("pool".into()),
                NodeId("n".into())
            )
            .is_err()
    );
}

// 保护 pending 不能出现在 UrlTest，避免不可信 JSON 把它静默丢弃。
#[test]
fn non_manual_policy_rejects_pending_input() {
    let value = serde_json::json!({"kind":"url_test", "probe_url":"https://example.invalid", "interval_secs":120, "tolerance_ms":80, "pending_node_id":"n"});
    assert!(serde_json::from_value::<SelectionPolicy>(value).is_err());
}

// 保护目标策略真实磁盘 CRUD/命中排序、旧schema恢复及失败保旧，不启动内核。
#[test]
fn policy33_routing_disk_roundtrip_order_validation_and_failed_write_keep_truth() {
    let fixture = Fixture::new();
    let initial = fixture.seed_pool();
    let mut a = TargetRoutingPolicy::new(PoolId("routing-a".into()));
    a.name = "策略甲".into();
    a.icon = "misc:pin".into();
    a.icon_scale = 2;
    a.matchers = vec![TrafficMatcher::DomainSuffix(vec!["example.invalid".into()])];
    let mut b = a.clone();
    b.id = PoolId("routing-b".into());
    b.name = "策略乙".into();
    b.matchers = vec![TrafficMatcher::Port(vec![443])];
    let mut fallback = TargetRoutingPolicy::new(PoolId("routing-fallback".into()));
    fallback.name = "未匹配".into();
    fallback.icon = "misc:dart".into();
    let mut routing = initial.profile.routing.clone();
    routing.policies = vec![a.clone(), b.clone()];
    routing.fallback = Some(fallback);
    let saved = fixture
        .snapshots
        .save_routing(initial.config_version(), routing.clone())
        .unwrap();
    assert_eq!(saved.effect, ApplyEffect::SavedOnly);
    assert_eq!(saved.version.0.revision, initial.config_revision + 1);
    // UI 必须直接收到同一事务提交的快照/版本，不能依赖保存之后的第二次 gate。
    assert_eq!(saved.value.profile.routing, routing);
    assert_eq!(saved.value.config_version(), saved.version);
    assert_eq!(saved.value, fixture.state());
    let busy = fixture.snapshots.gate.try_lock().unwrap();
    assert!(fixture.snapshots.snapshot().is_err());
    assert_eq!(saved.value.profile.routing, routing);
    assert_eq!(saved.value.config_version(), saved.version);
    drop(busy);
    assert_eq!(
        fixture.state().selection_version(),
        initial.selection_version()
    );
    let reopened = SnapshotService::new(
        JsonStateStore::new(fixture.root.join("state.json")).unwrap(),
        StateAccessGate::default(),
    );
    assert_eq!(reopened.snapshot().unwrap().profile.routing, routing);
    // 策略身份、名称、匹配条件与兜底约束在提交前失败；真实出口图由组配置继续校验。
    let before = fixture.bytes();
    for case in 0..5 {
        let mut invalid = routing.clone();
        match case {
            0 => invalid.policies[0].name.clear(),
            1 => invalid.policies[0].matchers.clear(),
            2 => invalid.policies[1].name = invalid.policies[0].name.clone(),
            3 => invalid.policies[1].id = a.id.clone(),
            _ => invalid.fallback.as_mut().unwrap().enabled = false,
        }
        assert!(
            fixture
                .snapshots
                .save_routing(saved.version.clone(), invalid)
                .is_err()
        );
        assert_eq!(fixture.bytes(), before);
    }
    routing.policies.reverse();
    routing.policies[0].name = "策略乙更新".into();
    routing.policies[0].enabled = false;
    let updated = fixture
        .snapshots
        .save_routing(saved.version.clone(), routing.clone())
        .unwrap();
    assert_eq!(reopened.snapshot().unwrap().profile.routing, routing);
    assert!(
        fixture
            .snapshots
            .save_routing(saved.version, routing.clone())
            .is_err()
    );
    routing.policies.pop();
    let deleted = fixture
        .snapshots
        .save_routing(updated.version, routing.clone())
        .unwrap();
    assert_eq!(
        reopened.snapshot().unwrap().profile.routing.policies.len(),
        1
    );
    let before = fixture.bytes();
    fixture.fail_writes();
    routing.policies.clear();
    assert!(
        fixture
            .snapshots
            .save_routing(deleted.version, routing)
            .is_err()
    );
    assert_eq!(fixture.bytes(), before);
    // 真旧字段缺失读取默认空/None，不生成假策略，也不改变旧默认路由。
    let mut old = serde_json::to_value(&initial).unwrap();
    old["profile"]["routing"]
        .as_object_mut()
        .unwrap()
        .remove("policies");
    old["profile"]["routing"]
        .as_object_mut()
        .unwrap()
        .remove("fallback");
    let restored: AppState = serde_json::from_value(old).unwrap();
    assert!(restored.profile.routing.policies.is_empty());
    assert!(restored.profile.routing.fallback.is_none());
    assert_eq!(restored.default_target, initial.default_target);
}
// 保护真实未决选择：改名保留 pending，停用策略不能清理或隐藏未决出口。
#[test]
fn policy33_routing_edit_preserves_pending_or_rejects_disabled_selector() {
    let fixture = Fixture::new();
    let initial = fixture.seed_pool();
    let mut policy = TargetRoutingPolicy::new(PoolId("routing-pending".into()));
    policy.name = "待确认策略".into();
    policy.matchers = vec![TrafficMatcher::Domain(vec!["test.invalid".into()])];
    let mut routing = initial.profile.routing.clone();
    routing.policies.push(policy.clone());
    let saved = fixture
        .snapshots
        .save_routing(initial.config_version(), routing.clone())
        .unwrap();
    fixture
        .selection()
        .stage_group(
            saved.version.clone(),
            fixture.state().selection_version(),
            policy.id.clone(),
            GroupSelectionIntent {
                member: OutboundId::Direct,
                mode: GroupSelectionMode::Auto,
            },
            None,
        )
        .unwrap();
    let before = fixture.bytes();
    let policy_index = routing
        .policies
        .iter()
        .position(|p| p.id == policy.id)
        .unwrap();
    routing.policies[policy_index].enabled = false;
    assert!(
        fixture
            .snapshots
            .save_routing(saved.version.clone(), routing.clone())
            .is_err()
    );
    assert_eq!(fixture.bytes(), before);
    routing.policies[policy_index].enabled = true;
    routing.policies[policy_index].name = "改名但保持未决".into();
    fixture
        .snapshots
        .save_routing(saved.version, routing)
        .unwrap();
    assert_eq!(
        fixture.state().group_selections[&policy.id]
            .pending
            .as_ref()
            .unwrap()
            .member,
        OutboundId::Direct
    );
}

// 保护已存在策略在正式新增/停用/删除出站后更新同一目录与 RuntimeIntent，磁盘无第二份成员真相。
#[test]
fn policy72_routing_members_follow_group_save_and_disk_reopen() {
    let fixture = Fixture::new();
    let initial = fixture.seed_pool();
    let mut policy = TargetRoutingPolicy::new(PoolId("routing-live".into()));
    policy.name = "实时出站策略".into();
    policy.matchers = vec![TrafficMatcher::Domain(vec!["live.invalid".into()])];
    let mut routing = initial.profile.routing.clone();
    routing.policies.push(policy.clone());
    let saved = fixture
        .snapshots
        .save_routing(initial.config_version(), routing)
        .unwrap()
        .value;
    let mut group = NodeGroup::new(PoolId("new-outbound".into()));
    group.name = "新增出站".into();
    group.rule = GroupRule::Selector;
    group.members = vec![OutboundId::Node(NodeId("n".into()))];
    let added = fixture
        .snapshots
        .save_groups(saved.config_version(), vec![group.clone()])
        .unwrap();
    assert_eq!(added.effect, ApplyEffect::SavedOnly);
    let reopen = || {
        SnapshotService::new(
            JsonStateStore::new(fixture.root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        )
        .snapshot()
        .unwrap()
    };
    let added = reopen();
    let derived = added.target_routing_members();
    assert!(derived.contains(&group.outbound_id()));
    assert_eq!(
        OutboundCatalog::from_state(&added)
            .get(&OutboundId::Pool(policy.id.clone()))
            .unwrap()
            .references,
        derived
    );
    assert_eq!(
        RuntimeIntent::from_state(&added)
            .unwrap()
            .groups
            .iter()
            .find(|g| g.id == policy.id)
            .unwrap()
            .members,
        derived
    );
    assert!(
        !serde_json::to_value(&added.profile.routing.policies[0])
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("members")
    );
    // 兼容早期冻结名单，丢弃旧缓存但不接受其它未知字段。
    let mut old = serde_json::to_value(&added).unwrap();
    old["profile"]["routing"]["policies"][0]["members"] =
        serde_json::json!([{"pool":"missing-old-cache"}]);
    let restored: AppState = serde_json::from_value(old.clone()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.target_routing_members(), derived);
    old["profile"]["routing"]["policies"][0]["unknown"] = true.into();
    assert!(serde_json::from_value::<AppState>(old).is_err());
    // pending 时禁止改变派生成员；不是静默移除 pending 或自动 Apply。
    fixture
        .selection()
        .stage_group(
            added.config_version(),
            added.selection_version(),
            policy.id.clone(),
            GroupSelectionIntent {
                member: group.outbound_id(),
                mode: GroupSelectionMode::Auto,
            },
            None,
        )
        .unwrap();
    let before = fixture.bytes();
    group.enabled = false;
    assert!(
        fixture
            .snapshots
            .save_groups(added.config_version(), vec![group.clone()])
            .is_err()
    );
    assert_eq!(fixture.bytes(), before);
    let pending = fixture.state();
    fixture
        .selection()
        .stage_group(
            pending.config_version(),
            pending.selection_version(),
            policy.id.clone(),
            GroupSelectionIntent {
                member: group.outbound_id(),
                mode: GroupSelectionMode::Auto,
            },
            Some(true),
        )
        .unwrap();
    let disabled = fixture
        .snapshots
        .save_groups(added.config_version(), vec![group.clone()])
        .unwrap()
        .value;
    assert!(
        !disabled
            .target_routing_members()
            .contains(&group.outbound_id())
    );
    assert!(disabled.group_selections[&policy.id].selected.is_none());
    assert_eq!(
        disabled.group_selections[&policy.id].mode,
        GroupSelectionMode::Auto
    );
    assert_eq!(reopen(), disabled);
    let deleted = fixture
        .snapshots
        .save_groups(disabled.config_version(), vec![])
        .unwrap()
        .value;
    assert!(
        !deleted
            .target_routing_members()
            .contains(&group.outbound_id())
    );
    assert_eq!(reopen(), deleted);
    // 仍拒绝真实组配置的悬空/self/cycle，不把失效引用解释为Direct。
    group.enabled = true;
    group.members = vec![OutboundId::Pool(policy.id.clone())];
    let before = fixture.bytes();
    assert!(
        fixture
            .snapshots
            .save_groups(deleted.config_version(), vec![group.clone()])
            .is_err()
    );
    assert_eq!(fixture.bytes(), before);
    group.members = vec![OutboundId::Pool(PoolId("missing-real-outbound".into()))];
    assert!(
        fixture
            .snapshots
            .save_groups(deleted.config_version(), vec![group])
            .is_err()
    );
    assert_eq!(fixture.bytes(), before);
}

// 保护新安装使用同一份内置数据，已有快照不自动迁移；两类恢复各自只改所负责事实。
#[test]
fn policy87_builtin_initialization_restore_isolation_and_disk_reopen() {
    let f = Fixture::new();
    let initial = f.state();
    assert_eq!(initial.groups, builtin_defaults::default_outbound_groups());
    assert_eq!(
        initial.profile.routing,
        builtin_defaults::default_target_routing()
    );
    assert_eq!(
        initial.app_config.behavior.proxy_view.strategy_order,
        builtin_defaults::default_strategy_order()
    );
    initial.validate().unwrap();
    let mut old = f.seed_pool();
    old.profile.routing = ProfileRouting::default();
    old.app_config.behavior.proxy_view.node_card_min_width = 3030;
    let mut custom = NodeGroup::new(PoolId("user-group".into()));
    custom.name = "用户组".into();
    custom.mode = GroupMode::Dynamic;
    old.groups.push(custom);
    let old = f.store.commit(&old).unwrap();
    assert_eq!(f.state(), old); // 已有空routing不被初始化静默替换。
    let saved = f
        .snapshots
        .save_groups(
            old.config_version(),
            builtin_defaults::default_groups_for(&old),
        )
        .unwrap()
        .value;
    assert_eq!(saved.profile, old.profile);
    assert_eq!(saved.nodes, old.nodes);
    assert_eq!(saved.subscriptions, old.subscriptions);
    assert_eq!(saved.app_config, old.app_config);
    assert!(
        saved
            .groups
            .iter()
            .find(|g| g.rule == GroupRule::Failover)
            .unwrap()
            .enabled
    );
    let restored = f
        .snapshots
        .restore_target_routing(saved.config_version())
        .unwrap()
        .value;
    assert_eq!(restored.groups, saved.groups);
    assert_eq!(restored.subscriptions, saved.subscriptions);
    assert_eq!(restored.nodes, saved.nodes);
    assert_eq!(
        restored.profile.routing,
        builtin_defaults::default_target_routing()
    );
    assert_eq!(
        restored.app_config.behavior.proxy_view.node_card_min_width,
        3030
    );
    let reopened = JsonStateStore::new(f.root.join("state.json"))
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(restored, reopened);
    let bytes = f.bytes();
    assert!(
        f.snapshots
            .restore_target_routing(saved.config_version())
            .is_err()
    );
    assert_eq!(f.bytes(), bytes);
    let mut changed = restored.profile.routing.clone();
    changed.policies[0].name = "恢复前修改".into();
    let changed = f
        .snapshots
        .save_routing(restored.config_version(), changed)
        .unwrap()
        .value;
    let bytes = f.bytes();
    f.fail_writes();
    assert!(
        f.snapshots
            .restore_target_routing(changed.config_version())
            .is_err()
    );
    assert_eq!(f.bytes(), bytes);
}

// 重复恢复默认也必须保留未决选择，不能把尚未确认的真实结果当作已撤销。
#[test]
fn policy87_repeated_restore_rejects_pending_without_changing_disk() {
    let f = Fixture::new();
    let state = f.seed_pool();
    let policy = state.profile.routing.policies[0].id.clone();
    f.selection()
        .stage_group(
            state.config_version(),
            state.selection_version(),
            policy.clone(),
            GroupSelectionIntent {
                member: OutboundId::Direct,
                mode: GroupSelectionMode::Auto,
            },
            None,
        )
        .unwrap();
    let before = f.state();
    let bytes = f.bytes();
    assert!(
        f.snapshots
            .restore_target_routing(before.config_version())
            .is_err()
    );
    assert_eq!(f.bytes(), bytes);
    assert_eq!(
        f.state().group_selections[&policy],
        before.group_selections[&policy]
    );
}

// 保护站点集备注真实保存/重启恢复、旧快照默认空与失败保旧；不启停内核或改变选择。
#[test]
fn policy104_routing_notes_disk_reopen_old_state_and_failed_write_keep_truth() {
    let fixture = Fixture::new();
    let initial = fixture.seed_pool();
    let mut policy = TargetRoutingPolicy::new(PoolId("routing-notes".into()));
    policy.name = "带备注站点集".into();
    policy.matchers = vec![TrafficMatcher::DomainSuffix(vec!["example.invalid".into()])];
    assert!(policy.notes.is_empty());
    let mut old = serde_json::to_value(&policy).unwrap();
    assert!(!old.as_object().unwrap().contains_key("notes"));
    // 真旧成员缓存的读取兼容也不能因新增 notes 字段失效。
    old["members"] = serde_json::json!(["direct"]);
    let restored: TargetRoutingPolicy = serde_json::from_value(old).unwrap();
    assert_eq!(restored, policy);
    policy.notes.insert(
        "domainSuffix:example.invalid".into(),
        "保留这条域名后缀的说明".into(),
    );
    policy.notes.insert(
        "ruleset:geosite-youtube".into(),
        "域名集备注元数据，不自动添加匹配能力".into(),
    );
    let mut routing = initial.profile.routing.clone();
    routing.policies.push(policy.clone());
    let saved = fixture
        .snapshots
        .save_routing(initial.config_version(), routing.clone())
        .unwrap();
    assert_eq!(saved.effect, ApplyEffect::SavedOnly);
    assert_eq!(saved.value.selection_version(), initial.selection_version());
    assert_eq!(saved.version.0.revision, initial.config_revision + 1);
    let reopen = || {
        SnapshotService::new(
            JsonStateStore::new(fixture.root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        )
        .snapshot()
        .unwrap()
    };
    assert_eq!(
        reopen().profile.routing.selector(&policy.id).unwrap().notes,
        policy.notes
    );
    let bytes = fixture.bytes();
    fixture.fail_writes();
    routing
        .policies
        .iter_mut()
        .find(|p| p.id == policy.id)
        .unwrap()
        .notes
        .clear();
    assert!(
        fixture
            .snapshots
            .save_routing(saved.version, routing)
            .is_err()
    );
    assert_eq!(fixture.bytes(), bytes);
    assert_eq!(
        reopen().profile.routing.selector(&policy.id).unwrap().notes,
        policy.notes
    );
}
