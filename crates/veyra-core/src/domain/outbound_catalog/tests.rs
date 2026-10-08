use super::*;
use crate::domain::*;

// 保护出口身份、订阅刷新后的即时目录及 UI 查询可观察状态；不启动内核或网络。
fn state() -> AppState {
    let mut s = AppState::empty();
    s.subscriptions.push(Subscription {
        id: SubscriptionId("subscription".into()),
        name: "订阅".into(),
        description: String::new(),
        source: SubscriptionSource::Manual,
        last_success_at_ms: None,
        last_attempt_at_ms: None,
        http_metadata: None,
        remote_request: None,
        update_policy: SubscriptionUpdatePolicy::manual(),
        skipped_unsupported_nodes: 0,
        document: None,
    });
    s.providers.push(Provider {
        id: ProviderId("provider".into()),
        subscription_id: s.subscriptions[0].id.clone(),
        name: "Provider".into(),
    });
    for id in ["a", "b"] {
        s.nodes.push(ProxyNode {
            id: NodeId(id.into()),
            provider_id: s.providers[0].id.clone(),
            name: "同名".into(),
            protocol: ProxyProtocol::Shadowsocks,
            server: "example.invalid".into(),
            port: 443,
            options: ProtocolOptions::Shadowsocks {
                method: "aes-128-gcm".into(),
                password: "fixture-secret".into(),
            },
            tls: None,
            transport: None,
        });
    }
    for (id, selection) in [
        (
            "manual",
            SelectionPolicy::Manual {
                selected_node_id: Some(s.nodes[0].id.clone()),
            },
        ),
        (
            "auto",
            SelectionPolicy::UrlTest {
                probe_url: "https://example.invalid/probe".into(),
                interval_secs: 120,
                tolerance_ms: 80,
            },
        ),
    ] {
        s.pools.push(NodePool {
            id: PoolId(id.into()),
            name: "同名组".into(),
            kind: PoolKind::ImplicitProvider,
            sources: vec![PoolSource {
                provider_id: s.providers[0].id.clone(),
                filter: NodeFilter::default(),
            }],
            selection,
            enabled: true,
        });
    }
    s
}
fn pool_id(id: &str) -> OutboundId {
    OutboundId::Pool(PoolId(id.into()))
}
fn node_id(id: &str) -> OutboundId {
    OutboundId::Node(NodeId(id.into()))
}
fn entry(s: &AppState, id: &str) -> OutboundEntry {
    OutboundCatalog::from_state(s)
        .get(&pool_id(id))
        .unwrap()
        .clone()
}

#[test]
fn base_entries_lookup_and_route_target_resolution_are_explicit() {
    let s = state();
    s.validate().unwrap();
    let catalog = OutboundCatalog::from_state(&s);
    assert_eq!(catalog.list().len(), 6);
    for (id, kind) in [
        (node_id("a"), OutboundKind::Node),
        (node_id("b"), OutboundKind::Node),
        (pool_id("manual"), OutboundKind::Selector),
        (pool_id("auto"), OutboundKind::UrlTest),
        (OutboundId::Direct, OutboundKind::Direct),
        (OutboundId::Block, OutboundKind::Block),
    ] {
        let e = catalog.require_available(&id).unwrap();
        assert_eq!(e.kind, kind);
        assert_eq!(e.id, id);
    }
    for target in [
        RouteTarget::Pool(PoolId("manual".into())),
        RouteTarget::Direct,
        RouteTarget::Block,
    ] {
        assert_eq!(
            catalog.resolve(&target),
            OutboundId::from_route_target(&target)
        );
    }
    assert_eq!(
        catalog.resolve(&RouteTarget::Unconfigured),
        Err(OutboundResolveError::Unconfigured)
    );
    assert_eq!(
        catalog.resolve(&RouteTarget::Pool(PoolId("unknown".into()))),
        Err(OutboundResolveError::Missing(pool_id("unknown")))
    );
    assert_eq!(
        catalog.get(&node_id("unknown")),
        Err(OutboundResolveError::Missing(node_id("unknown")))
    );
    let manual = entry(&s, "manual");
    assert_eq!(manual.pool_kind, Some(PoolKind::ImplicitProvider));
    assert_eq!(manual.provider_ids, vec![s.providers[0].id.clone()]);
    assert_eq!(manual.subscription_ids, vec![s.subscriptions[0].id.clone()]);
    assert_eq!(manual.references, vec![node_id("a"), node_id("b")]);
    assert_eq!(manual.selection, Some(s.pools[0].selection.clone()));
    assert_eq!(
        entry(&s, "auto").selection,
        Some(s.pools[1].selection.clone())
    );
    assert!(!format!("{catalog:?}").contains("fixture-secret"));
}

#[test]
fn rename_same_name_and_refresh_do_not_change_identity() {
    let mut s = state();
    let old = OutboundCatalog::from_state(&s);
    s.nodes[0].name = "重命名".into();
    s.pools[0].name = "新组名".into();
    let renamed = OutboundCatalog::from_state(&s);
    assert_eq!(
        old.list().iter().map(|e| &e.id).collect::<Vec<_>>(),
        renamed.list().iter().map(|e| &e.id).collect::<Vec<_>>()
    );
    assert_eq!(
        renamed.get(&node_id("a")).unwrap().display_name.as_deref(),
        Some("重命名")
    );
    assert_eq!(
        renamed.get(&node_id("b")).unwrap().display_name.as_deref(),
        Some("同名")
    );
    s.nodes[1].id = NodeId("replacement".into());
    let refreshed = OutboundCatalog::from_state(&s);
    assert!(matches!(
        refreshed.get(&node_id("b")),
        Err(OutboundResolveError::Missing(_))
    ));
    assert_eq!(
        refreshed.get(&pool_id("manual")).unwrap().references,
        vec![node_id("a"), node_id("replacement")]
    );
    assert_eq!(
        refreshed.resolve(&RouteTarget::Pool(PoolId("manual".into()))),
        Ok(pool_id("manual"))
    );
}

#[test]
fn disabled_empty_deleted_and_missing_owners_are_reported() {
    let mut s = state();
    s.pools[0].enabled = false;
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::Disabled)
    );
    assert!(matches!(
        OutboundCatalog::from_state(&s).resolve(&RouteTarget::Pool(PoolId("manual".into()))),
        Err(OutboundResolveError::Unavailable { .. })
    ));
    s = state();
    s.nodes.clear();
    s.pools[0].selection = SelectionPolicy::Manual {
        selected_node_id: None,
    };
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::EmptyMembers)
    );
    s = state();
    s.providers.clear();
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::MissingProvider(ProviderId(
            "provider".into()
        )))
    );
    s = state();
    s.subscriptions.clear();
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::MissingSubscription(
            SubscriptionId("subscription".into())
        ))
    );
    assert!(matches!(
        OutboundCatalog::from_state(&s)
            .get(&node_id("a"))
            .unwrap()
            .availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::MissingSubscription(_))
    ));
    s = state();
    s.pools.clear();
    assert!(matches!(
        OutboundCatalog::from_state(&s).get(&pool_id("manual")),
        Err(OutboundResolveError::Missing(_))
    ));
}

#[test]
fn selected_node_must_be_a_member_and_deleted_filter_ids_are_not_silently_empty() {
    let mut s = state();
    s.pools[0].sources[0].filter.include_node_ids = vec![NodeId("b".into())];
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::SelectedNodeNotMember(
            NodeId("a".into())
        ))
    );
    assert_eq!(s.validate(), Err(StateValidationError::InvalidSelection));
    s.pools[0].sources[0].filter.include_node_ids = vec![NodeId("deleted".into())];
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::MissingNode(NodeId(
            "deleted".into()
        )))
    );
    assert_eq!(s.validate(), Err(StateValidationError::InvalidFilter));
    s = state();
    s.nodes.remove(0);
    assert_eq!(
        entry(&s, "manual").availability,
        OutboundAvailability::Unavailable(OutboundUnavailableReason::SelectedNodeNotMember(
            NodeId("a".into())
        ))
    );
}

#[test]
fn runtime_intent_uses_the_same_member_selection_and_graph_contract() {
    let s = state();
    let mut intent = RuntimeIntent::from_state(&s).unwrap();
    let catalog = OutboundCatalog::from_runtime_intent(&intent);
    assert_eq!(catalog.validate_graph(), Ok(()));
    assert_eq!(
        catalog.get(&pool_id("auto")).unwrap().selection,
        Some(s.pools[1].selection.clone())
    );
    intent.pools[0].members.push(NodeId("deleted".into()));
    let catalog = OutboundCatalog::from_runtime_intent(&intent);
    assert!(matches!(
        catalog.validate_graph(),
        Err(OutboundGraphError::Dangling { .. })
    ));
    assert!(matches!(
        catalog.resolve(&RouteTarget::Pool(intent.pools[0].id.clone())),
        Err(OutboundResolveError::Unavailable { .. })
    ));
}

// 保护统一注册机制，不创建或持久化假高级组；用现有 PoolId 构造抽象引用图。
#[test]
fn graph_rejects_dangling_self_cycle_and_duplicate_and_accepts_shared_dag() {
    assert_eq!(
        validate_outbound_graph(vec![(pool_id("a"), vec![node_id("missing")])]),
        Err(OutboundGraphError::Dangling {
            source: pool_id("a"),
            target: node_id("missing")
        })
    );
    assert_eq!(
        validate_outbound_graph(vec![(pool_id("a"), vec![pool_id("a")])]),
        Err(OutboundGraphError::SelfReference(pool_id("a")))
    );
    assert_eq!(
        validate_outbound_graph(vec![
            (pool_id("a"), vec![pool_id("b")]),
            (pool_id("b"), vec![pool_id("c")]),
            (pool_id("c"), vec![pool_id("a")])
        ]),
        Err(OutboundGraphError::Cycle(vec![
            pool_id("a"),
            pool_id("b"),
            pool_id("c"),
            pool_id("a")
        ]))
    );
    assert_eq!(
        validate_outbound_graph(vec![(pool_id("a"), vec![]), (pool_id("a"), vec![])]),
        Err(OutboundGraphError::Duplicate(pool_id("a")))
    );
    assert_eq!(
        validate_outbound_graph(vec![
            (pool_id("a"), vec![node_id("n")]),
            (pool_id("b"), vec![node_id("n")]),
            (node_id("n"), vec![])
        ]),
        Ok(())
    );
}

// 保护真实 provider replacement 入口：重命名保留身份，成员增删即时反映到同一隐式组。
#[test]
fn provider_refresh_uses_normalized_identity_and_updates_implicit_members() {
    use crate::application::provider_replacement::apply_provider_replacement;
    use crate::subscription::{ParseResult, ProxyNodeDraft, SubscriptionFormat, normalize_nodes};
    let mut s = state();
    let draft = |name: &str, server: &str| ProxyNodeDraft {
        name: name.into(),
        protocol: ProxyProtocol::Shadowsocks,
        server: server.into(),
        port: 443,
        options: s.nodes[0].options.clone(),
        transport: None,
        tls: None,
    };
    let first = draft("同名", "first.example.invalid");
    let removed = draft("同名", "removed.example.invalid");
    let added = draft("同名", "added.example.invalid");
    let provider = s.providers[0].id.clone();
    s.nodes = normalize_nodes(provider.clone(), vec![first.clone(), removed]).unwrap();
    let stable = s.nodes[0].id.clone();
    let deleted = s.nodes[1].id.clone();
    s.pools[0].selection = SelectionPolicy::Manual {
        selected_node_id: Some(stable.clone()),
    };
    let before = entry(&s, "manual");
    let mut renamed = first;
    renamed.name = "重命名".into();
    apply_provider_replacement(
        &mut s,
        provider,
        ParseResult {
            format: SubscriptionFormat::ClashYaml,
            nodes: vec![renamed, added],
            skipped: Vec::new(),
        },
    )
    .unwrap();
    let catalog = OutboundCatalog::from_state(&s);
    assert_eq!(
        catalog
            .require_available(&OutboundId::Node(stable.clone()))
            .unwrap()
            .display_name
            .as_deref(),
        Some("重命名")
    );
    assert!(matches!(
        catalog.get(&OutboundId::Node(deleted)),
        Err(OutboundResolveError::Missing(_))
    ));
    let after = entry(&s, "manual");
    assert_eq!(before.id, after.id);
    assert_ne!(before.references, after.references);
    assert_eq!(
        after.selection,
        Some(SelectionPolicy::Manual {
            selected_node_id: Some(stable)
        })
    );
    assert_eq!(after.availability, OutboundAvailability::Available);
    assert_eq!(catalog.list().len(), s.nodes.len() + s.pools.len() + 2);
}
