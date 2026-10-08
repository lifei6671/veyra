use super::*;
use crate::{
    application::{
        selected_subscription::project_selected_runtime,
        state_access::StateAccessGate,
        state_service::{GroupSaveError, SnapshotService},
    },
    singbox::{
        LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
        SingBoxCompiler,
    },
    storage::{JsonStateStore, StateStore},
};
use serde_json::{Value, json};
use std::path::Path;
fn state() -> AppState {
    let mut value = serde_json::to_value(AppState::empty()).unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/compiler/p2-02b.json")).unwrap();
    for (key, part) in fixture.as_object().unwrap() {
        value[key] = part.clone();
    }
    let mut state: AppState = serde_json::from_value(value).unwrap();
    state.groups.clear();
    state.active_subscription_id = Some(state.subscriptions[0].id.clone());
    state
}
fn group(id: &str, members: Vec<OutboundId>) -> NodeGroup {
    NodeGroup {
        name: id.into(),
        rule: GroupRule::Selector,
        members,
        ..NodeGroup::new(PoolId(id.into()))
    }
}
fn runtime_document(state: &AppState) -> Value {
    let projection = project_selected_runtime(state).unwrap();
    let resources = ProductRuntimeResources {
        mixed: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
        controller: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
        cache: ManagedCacheFile::new(
            Path::new("/tmp/p402-unit"),
            "/tmp/p402-unit/cache.db".into(),
            "p402".into(),
            false,
            false,
        )
        .unwrap(),
    };
    let target = OutboundId::from_route_target(&projection.projected_default_target).unwrap();
    let generated = SingBoxCompiler
        .compile_product(ProductCompileRequest {
            state,
            runtime_intent: &projection.runtime_intent,
            default_outbound: &target,
            resources: &resources,
        })
        .unwrap();
    // 保存的产品配置必须可恢复；包含嵌套组和 Direct 也不能破坏既有恢复索引。
    let bytes = generated.recovery_bytes().unwrap();
    let recovered = crate::singbox::SingBoxPlan::recover(&bytes, &resources).unwrap();
    assert_eq!(recovered.artifact_index(), generated.artifact_index());
    let generated = recovered
        .finalize(&crate::singbox::secret::test_api_secret())
        .unwrap();
    serde_json::from_slice(generated.as_bytes()).unwrap()
}
#[test]
fn p402_keywords_match_flags_short_codes_or_and_cn2() {
    // 保护国家自动分组：US 不误选 Russia，CN 不误选 CN2；关键词是 OR。
    for name in ["🇺🇸 01", "US-02", "🇺🇸US-03"] {
        assert!(group_keyword_matches(name, "us"));
    }
    for name in ["Russia", "Australia", "RUS-1"] {
        assert!(!group_keyword_matches(name, "US"));
    }
    assert!(!group_keyword_matches("CN2 GIA", "cn"));
    assert!(group_keyword_matches("CN20", "cn"));
    assert!(group_keyword_matches("香港-01", "香港"));
    assert!(!group_keyword_matches("node", "  "));
}
#[test]
fn p402_dynamic_members_follow_real_nodes_and_disabled_sources() {
    // 保护动态预览/编译一致；刷新增加真实节点后自动加入，停用来源后移除。
    let mut s = state();
    let mut g = group("dynamic", vec![]);
    g.mode = GroupMode::Dynamic;
    let provider = s.nodes[0].provider_id.clone();
    s.nodes[0].name = "HK-1".into();
    g.keywords = vec!["hk".into(), "日本".into()];
    assert_eq!(
        s.group_members(&g),
        vec![OutboundId::Node(s.nodes[0].id.clone())]
    );
    let mut node = s.nodes[0].clone();
    node.id = NodeId("new-jp".into());
    node.name = "日本-2".into();
    s.nodes.push(node.clone());
    assert_eq!(
        s.group_members(&g),
        vec![
            OutboundId::Node(s.nodes[0].id.clone()),
            OutboundId::Node(node.id)
        ]
    );
    s.pools.push(NodePool {
        id: PoolId("disabled-provider".into()),
        name: "disabled".into(),
        kind: PoolKind::ImplicitProvider,
        sources: vec![PoolSource {
            provider_id: provider,
            filter: NodeFilter::default(),
        }],
        selection: SelectionPolicy::Manual {
            selected_node_id: None,
            pending_node_id: None,
        },
        enabled: false,
    });
    assert!(s.group_members(&g).is_empty());
}
#[test]
fn p402_static_order_nested_groups_and_health_reach_product_compiler() {
    // 保护编辑器保存的成员顺序、嵌套 selector/urltest 以及组独立健康地址。
    let mut s = state();
    let a = OutboundId::Node(s.nodes[0].id.clone());
    let b = OutboundId::Node(s.nodes[1].id.clone());
    let child = group("child", vec![b.clone(), a.clone(), OutboundId::Direct]);
    let mut parent = group("parent", vec![child.outbound_id()]);
    parent.rule = GroupRule::UrlTest;
    parent.test_url = "https://group.example.invalid/health".into();
    parent.interval_secs = 17;
    parent.tolerance_ms = 42;
    s.groups = vec![parent, child];
    s.default_target = RouteTarget::Pool(PoolId("parent".into()));
    let doc = runtime_document(&s);
    let outs = doc["outbounds"].as_array().unwrap();
    let child = outs.iter().find(|o| o["tag"] == "pool-child").unwrap();
    // tags 经过既有 Compiler 映射；本 fixture ID 为可读 ASCII。
    assert_eq!(
        child["outbounds"],
        json!([
            format!("node-{}", s.nodes[1].id.0),
            format!("node-{}", s.nodes[0].id.0),
            "direct"
        ])
    );
    let parent = outs.iter().find(|o| o["tag"] == "pool-parent").unwrap();
    assert_eq!(parent["url"], "https://group.example.invalid/health");
    assert_eq!(parent["interval"], "17s");
    assert_eq!(parent["tolerance"], 42);
    s.groups[0].test_url.clear();
    s.profile.test_url =
        RuntimeHealthUrl::new("https://runtime.example.invalid/health".into()).unwrap();
    let doc = runtime_document(&s);
    assert_eq!(
        doc["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["tag"] == "pool-parent")
            .unwrap()["url"],
        "https://runtime.example.invalid/health"
    );
}
#[test]
fn p402_cycles_self_dangling_duplicate_names_and_block_are_rejected() {
    // 保护保存/编译共同的出口图，不丢成员、不将非法目标猜成 Direct。
    let mut s = state();
    let g = group("a", vec![OutboundId::Pool(PoolId("a".into()))]);
    s.groups = vec![g];
    assert!(matches!(
        s.validate_groups(),
        Err(GroupIssue::Graph(OutboundGraphError::SelfReference(_)))
    ));
    s.groups[0].members = vec![OutboundId::Pool(PoolId("b".into()))];
    s.groups.push(group("b", vec![s.groups[0].outbound_id()]));
    assert!(matches!(
        s.validate_groups(),
        Err(GroupIssue::Graph(OutboundGraphError::Cycle(_)))
    ));
    s.groups.pop();
    assert!(matches!(
        s.validate_groups(),
        Err(GroupIssue::Graph(OutboundGraphError::Dangling { .. }))
    ));
    s.groups[0].members = vec![OutboundId::Block];
    assert!(matches!(
        s.validate_groups(),
        Err(GroupIssue::BlockMember(_))
    ));
    s.groups[0].members = vec![OutboundId::Direct];
    s.groups.push(group("b", vec![OutboundId::Direct]));
    s.groups[1].name = "a".into();
    assert_eq!(
        s.validate_groups(),
        Err(GroupIssue::DuplicateName("a".into()))
    );
}
#[test]
fn p402_empty_dynamic_is_saved_but_unavailable_and_not_compiled() {
    // 新增空关键词结果不伪造出站，且被引用时必须显式失败。
    let mut s = state();
    let mut g = group("empty", vec![]);
    g.mode = GroupMode::Dynamic;
    g.keywords = vec!["no-such-node".into()];
    s.groups.push(g.clone());
    s.validate_groups().unwrap();
    assert!(
        OutboundCatalog::from_state(&s)
            .require_available(&g.outbound_id())
            .is_err()
    );
    assert!(s.runtime_groups().is_empty());
    assert!(
        !runtime_document(&s)["outbounds"]
            .as_array()
            .unwrap()
            .iter()
            .any(|o| o["tag"] == "pool-empty")
    );
    s.groups.push(group("parent", vec![g.outbound_id()]));
    assert!(matches!(
        s.validate_groups(),
        Err(GroupIssue::Unavailable { .. })
    ));
}
#[test]
fn p402_save_reload_cas_and_invalid_edit_preserve_snapshot() {
    // 保护保存/重启恢复、配置与选择版本分离、失败不覆盖磁盘、并发编辑拒绝。
    let root = std::env::temp_dir().join(NodeGroup::fresh().unwrap().id.0);
    std::fs::create_dir(&root).unwrap();
    let store = JsonStateStore::new(root.join("state.json")).unwrap();
    let s = state();
    store.save(&s).unwrap();
    let service = SnapshotService::new(store.clone(), StateAccessGate::default());
    let expected = s.config_version();
    let groups = vec![group(
        "ordered",
        vec![
            OutboundId::Node(s.nodes[1].id.clone()),
            OutboundId::Node(s.nodes[0].id.clone()),
        ],
    )];
    let out = service
        .save_groups(expected.clone(), groups.clone())
        .unwrap();
    assert_eq!(out.value.groups, groups);
    assert_eq!(out.value.selection_revision, s.selection_revision);
    assert_eq!(out.value.config_revision, s.config_revision + 1);
    assert_eq!(store.load().unwrap().groups, groups);
    assert!(matches!(
        service.save_groups(expected, groups.clone()),
        Err(GroupSaveError::Storage(_))
    ));
    let before = std::fs::read(root.join("state.json")).unwrap();
    let mut invalid = groups;
    invalid[0].members = vec![invalid[0].outbound_id()];
    assert!(
        service
            .save_groups(out.value.config_version(), invalid)
            .is_err()
    );
    assert_eq!(before, std::fs::read(root.join("state.json")).unwrap());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn p402_production_disk_crud_rebuild_and_write_failure() {
    // 保护真实编辑闭环：Service/Store 销毁重建后字段、顺序和版本仍来自磁盘。
    let root = std::env::temp_dir().join(NodeGroup::fresh().unwrap().id.0);
    std::fs::create_dir(&root).unwrap();
    let path = root.join("state.json");
    let initial = state();
    JsonStateStore::new(path.clone())
        .unwrap()
        .save(&initial)
        .unwrap();
    let reopen = || {
        SnapshotService::new(
            JsonStateStore::new(path.clone()).unwrap(),
            StateAccessGate::default(),
        )
    };
    let mut manual = group(
        "p402-manual",
        vec![
            OutboundId::Node(initial.nodes[1].id.clone()),
            OutboundId::Direct,
        ],
    );
    manual.icon = "US".into();
    manual.icon_scale = 4;
    let mut dynamic = group("dynamic", vec![]);
    dynamic.mode = GroupMode::Dynamic;
    dynamic.rule = GroupRule::UrlTest;
    dynamic.keywords = vec![initial.nodes[0].name.clone()];
    dynamic.interval_secs = 60;
    dynamic.tolerance_ms = 25;
    dynamic.test_url = "http://127.0.0.1:20002/group-health".into();
    let parent = group("parent", vec![manual.outbound_id(), dynamic.outbound_id()]);
    let created = {
        let service = reopen();
        service
            .save_groups(
                initial.config_version(),
                vec![manual.clone(), dynamic.clone(), parent.clone()],
            )
            .unwrap()
            .value
    };
    assert_eq!(reopen().snapshot().unwrap(), created);
    manual.name = "重命名后 ID 不变".into();
    manual.members.reverse();
    dynamic.enabled = false;
    // 父组不再引用停用成员；组顺序与成员顺序分别保存。
    let mut parent = parent;
    parent.members = vec![manual.outbound_id()];
    let updated = {
        let service = reopen();
        service
            .save_groups(
                created.config_version(),
                vec![parent.clone(), dynamic.clone(), manual.clone()],
            )
            .unwrap()
            .value
    };
    assert_eq!(reopen().snapshot().unwrap(), updated);
    assert_eq!(updated.config_revision, initial.config_revision + 2);
    assert_eq!(updated.selection_revision, initial.selection_revision);
    let bytes = std::fs::read(&path).unwrap();
    let stored: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(stored["schema_version"], CURRENT_SCHEMA_VERSION);
    assert_eq!(
        stored["groups"],
        serde_json::to_value(&updated.groups).unwrap()
    );
    // 删除仍被引用的组应返回可定位 dangling，文件逐字节不变。
    let error = reopen()
        .save_groups(updated.config_version(), vec![parent, dynamic.clone()])
        .unwrap_err();
    assert!(matches!(
        error,
        GroupSaveError::Invalid(GroupIssue::Graph(OutboundGraphError::Dangling { .. }))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // 只占用自有原子替换临时路径，制造真实 IO 失败，不依赖 chmod/root 行为。
    std::fs::create_dir(path.with_extension("tmp")).unwrap();
    assert!(matches!(
        reopen().save_groups(updated.config_version(), vec![manual.clone()]),
        Err(GroupSaveError::Storage(_))
    ));
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    std::fs::remove_dir(path.with_extension("tmp")).unwrap();
    assert_eq!(reopen().snapshot().unwrap(), updated);
    let deleted = reopen()
        .save_groups(updated.config_version(), vec![manual])
        .unwrap()
        .value;
    assert_eq!(reopen().snapshot().unwrap(), deleted);
    let defaults = reopen()
        .save_groups(deleted.config_version(), default_groups())
        .unwrap();
    assert_eq!(
        defaults.effect,
        crate::application::state_service::ApplyEffect::SavedOnly
    );
    assert_eq!(reopen().snapshot().unwrap().groups, default_groups());
    let mut empty = dynamic;
    empty.enabled = true;
    empty.keywords = vec!["missing-country".into()];
    let dropped = reopen()
        .save_groups(defaults.value.config_version(), vec![empty.clone()])
        .unwrap();
    assert_eq!(dropped.dropped, vec![empty.id]);
    assert_eq!(
        reopen().snapshot().unwrap().dropped_groups(),
        dropped.dropped
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn p402_old_v8_and_v9_disk_snapshots_accept_groups_without_losing_facts() {
    // 保护升级：旧 v8 迁移和无 groups 的既有 v9 均保留身份/版本，首次组保存可重建回读。
    for version in [8, 9] {
        let root = std::env::temp_dir().join(NodeGroup::fresh().unwrap().id.0);
        std::fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        let original = state();
        let mut old = serde_json::to_value(&original).unwrap();
        old["schema_version"] = json!(version);
        old.as_object_mut().unwrap().remove("groups");
        if version == 8 {
            old["app_config"]
                .as_object_mut()
                .unwrap()
                .remove("behavior");
        }
        std::fs::write(&path, serde_json::to_vec_pretty(&old).unwrap()).unwrap();
        let restored = {
            let service = SnapshotService::new(
                JsonStateStore::new(path.clone()).unwrap(),
                StateAccessGate::default(),
            );
            assert_eq!(service.snapshot().unwrap(), original);
            service
                .save_groups(
                    original.config_version(),
                    vec![group("new", vec![OutboundId::Direct])],
                )
                .unwrap()
                .value
        };
        let fresh = SnapshotService::new(
            JsonStateStore::new(path).unwrap(),
            StateAccessGate::default(),
        );
        assert_eq!(fresh.snapshot().unwrap(), restored);
        drop(fresh);
        std::fs::remove_dir_all(root).unwrap();
    }
}
#[test]
fn p402_default_and_auto_groups_preserve_user_groups_and_country_order() {
    // 保护恢复默认与国家生成；同名跳过、组顺序稳定、手工组不删除。
    let defaults = default_groups();
    assert_eq!(defaults.len(), 4);
    assert_eq!(defaults[0].rule, GroupRule::Direct);
    assert_eq!(defaults[3].rule, GroupRule::Block);
    let groups = auto_groups(
        &defaults,
        &["US".into(), "HK".into()],
        &[GroupRule::UrlTest, GroupRule::Selector],
    );
    assert_eq!(groups.len(), 8);
    assert_eq!(groups[4].name, "美国-自动");
    assert_eq!(groups[5].name, "美国-手动");
    assert_eq!(
        auto_groups(
            &groups,
            &["US".into()],
            &[GroupRule::UrlTest, GroupRule::Selector]
        ),
        groups
    );
}
#[test]
fn p402_cross_subscription_pool_members_survive_selected_projection() {
    // 当前订阅 A 不得滤掉静态组引用的 B 池；嵌套组仍能生成并恢复产品配置。
    let mut s = state();
    let mut subscription = s.subscriptions[0].clone();
    subscription.id = SubscriptionId("other".into());
    s.subscriptions.push(subscription);
    let mut provider = s.providers[0].clone();
    provider.id = ProviderId("other-provider".into());
    provider.subscription_id = SubscriptionId("other".into());
    s.providers.push(provider.clone());
    let mut node = s.nodes[0].clone();
    node.id = NodeId("other-node".into());
    node.provider_id = provider.id.clone();
    s.nodes.push(node.clone());
    let pool = NodePool {
        id: PoolId("other-pool".into()),
        name: "Other".into(),
        kind: PoolKind::ImplicitProvider,
        sources: vec![PoolSource {
            provider_id: provider.id,
            filter: NodeFilter::default(),
        }],
        selection: SelectionPolicy::Manual {
            selected_node_id: None,
            pending_node_id: None,
        },
        enabled: true,
    };
    s.groups = vec![
        group("inner", vec![OutboundId::Pool(pool.id.clone())]),
        group("outer", vec![OutboundId::Pool(PoolId("inner".into()))]),
    ];
    s.pools.push(pool);
    let projection = project_selected_runtime(&s).unwrap();
    assert!(
        projection
            .runtime_intent
            .nodes
            .iter()
            .any(|n| n.id == node.id)
    );
    let doc = runtime_document(&s);
    let outs = doc["outbounds"].as_array().unwrap();
    assert!(outs.iter().any(|o| o["tag"] == "pool-other-pool"));
}

#[test]
fn p402_deleted_route_and_default_group_report_reference_and_preserve_disk() {
    // 删除与恢复默认都必须指出仍被引用的组，且不能覆盖旧磁盘快照。
    for default_reference in [true, false] {
        let root = std::env::temp_dir().join(NodeGroup::fresh().unwrap().id.0);
        std::fs::create_dir(&root).unwrap();
        let path = root.join("state.json");
        let mut state = state();
        let group = group("route-group", vec![OutboundId::Direct]);
        let target = RouteTarget::Pool(group.id.clone());
        if default_reference {
            state.default_target = target.clone();
        } else {
            state.routes[0].enabled = true;
            state.routes[0].target = target.clone();
        }
        state.groups = vec![group.clone()];
        let store = JsonStateStore::new(path.clone()).unwrap();
        store.save(&state).unwrap();
        let service = SnapshotService::new(store, StateAccessGate::default());
        let before = std::fs::read(&path).unwrap();
        for replacement in [vec![], default_groups()] {
            assert!(
                matches!(service.save_groups(state.config_version(), replacement),
                Err(GroupSaveError::Invalid(GroupIssue::Referenced(id))) if id == group.outbound_id())
            );
            assert_eq!(std::fs::read(&path).unwrap(), before);
        }
        drop(service);
        assert_eq!(JsonStateStore::new(path).unwrap().load().unwrap(), state);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn p402_existing_unavailable_base_pool_keeps_snapshot_contract() {
    // P4 组校验不能把既有空/停用 Base pool 快照变成损坏数据。
    let mut state = state();
    state.groups.clear();
    let pool = &mut state.pools[0];
    pool.enabled = false;
    let id = pool.id.clone();
    state.default_target = RouteTarget::Pool(id.clone());
    state.routes[0].target = RouteTarget::Pool(id);
    assert!(state.validate().is_ok());
}
