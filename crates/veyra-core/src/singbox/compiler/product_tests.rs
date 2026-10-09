use super::*;
use crate::domain::*;
use serde_json::{Value, json};

// 代表性输入来自产品 AppState，真实 Node/selector/urltest；不复制 P0 direct-only 模型。
fn state() -> AppState {
    let mut value = serde_json::to_value(AppState::empty()).unwrap();
    let fixture: Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/compiler/p2-02b.json")).unwrap();
    for (key, value_part) in fixture.as_object().unwrap() {
        value[key] = value_part.clone();
    }
    let mut s: AppState = serde_json::from_value(value).unwrap();
    s.profile.test_url = RuntimeHealthUrl::new("http://127.0.0.1:20002/runtime".into()).unwrap();
    s.profile.direct_test_url =
        RuntimeHealthUrl::new("http://127.0.0.1:20003/direct".into()).unwrap();
    s
}
fn resources() -> ProductRuntimeResources {
    ProductRuntimeResources {
        mixed: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
        controller: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
        cache: ManagedCacheFile::new(
            Path::new("/tmp/veyra-product"),
            "/tmp/veyra-product/profile-v114/cache.db".into(),
            "profile-v114".into(),
            false,
            false,
        )
        .unwrap(),
    }
}
fn plan(
    s: &AppState,
    target: &OutboundId,
    r: &ProductRuntimeResources,
) -> Result<SingBoxPlan, CompileError> {
    // 完整已配置状态的旧路径：由调用方明确构造 intent，产品入口不会隐式重建。
    let intent = RuntimeIntent::from_state(s).map_err(|_| CompileError::InvalidProfile)?;
    SingBoxCompiler.compile_product(ProductCompileRequest {
        state: s,
        runtime_intent: &intent,
        default_outbound: target,
        resources: r,
    })
}
fn target() -> OutboundId {
    OutboundId::Pool(PoolId("manual".into()))
}
fn document(s: &AppState) -> Value {
    let generated = plan(s, &target(), &resources())
        .unwrap()
        .finalize(&crate::singbox::secret::test_api_secret())
        .unwrap();
    serde_json::from_slice(generated.as_bytes()).unwrap()
}

// P2-06 合入共享入站前，正式路径必须明确拒绝，不能把已保存/启用配置静默丢弃。
#[test]
fn p504_product_rejects_unintegrated_enabled_shared_inbounds() {
    let mut s = state();
    s.app_config.shared_servers.push(SharedServer {
        id: "home".into(),
        name: "home".into(),
        enabled: true,
        address: "example.invalid".into(),
        listen: "127.0.0.1".parse().unwrap(),
        port: 24008,
        protocol: SharedProtocol::Mixed {
            username: "".into(),
            password: "".into(),
        },
    });
    assert!(matches!(
        plan(&s, &target(), &resources()),
        Err(CompileError::UnsupportedOption(
            UnsupportedProductOption::SharedInbounds
        ))
    ));
    s.app_config.shared_servers[0].enabled = false;
    assert!(plan(&s, &target(), &resources()).is_ok());
}

#[test]
fn p202b_catalog_nodes_selector_urltest_direct_block_and_exact_resources() {
    let s = state();
    s.validate().unwrap();
    let doc = document(&s);
    assert_eq!(
        doc["inbounds"],
        json!([{"type":"mixed","tag":"mixed","listen":"127.0.0.1","listen_port":0}])
    );
    assert_eq!(
        doc["experimental"]["cache_file"],
        json!({"enabled":true,"path":"/tmp/veyra-product/profile-v114/cache.db","cache_id":"profile-v114","store_fakeip":false,"store_dns":false})
    );
    assert_eq!(
        doc["experimental"]["clash_api"]["external_controller"],
        "127.0.0.1:0"
    );
    assert!(
        doc["experimental"]["clash_api"]
            .get("store_selected")
            .is_none()
    );
    let out = doc["outbounds"].as_array().unwrap();
    for (tag, kind) in [
        ("node-a", "socks"),
        ("pool-manual", "selector"),
        ("pool-auto", "urltest"),
        ("direct", "direct"),
    ] {
        assert!(out.iter().any(|o| o["tag"] == tag && o["type"] == kind));
    }
    assert!(!out.iter().any(|o| o["type"] == "block"));
    assert_eq!(
        out.iter().find(|o| o["tag"] == "pool-auto").unwrap()["url"],
        "http://127.0.0.1:20001/pool"
    );
    assert!(
        doc["route"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["domain_suffix"] == json!(["block.example.invalid"])
                && r["action"] == "reject"
                && r.get("outbound").is_none())
    );
    for id in [
        OutboundId::Node(NodeId("a".into())),
        target(),
        OutboundId::Pool(PoolId("auto".into())),
        OutboundId::Direct,
        OutboundId::Block,
    ] {
        let p = plan(&s, &id, &resources()).unwrap();
        let d: Value = serde_json::from_slice(
            p.finalize(&crate::singbox::secret::test_api_secret())
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        if id == OutboundId::Block {
            assert_eq!(
                d["route"]["rules"].as_array().unwrap().last().unwrap(),
                &json!({"action":"reject"})
            );
        } else {
            assert_eq!(d["route"]["final"], outbound_tag(&id));
        }
    }
}
#[test]
fn p202b_runtime_resources_and_closed_final_parser_reject_invalid_inputs() {
    for address in ["0.0.0.0:0", "192.0.2.1:12000", "[::]:0"] {
        assert!(LoopbackListener::new(address.parse().unwrap()).is_err());
    }
    for path in [
        "cache.db",
        "/other/cache.db",
        "/tmp/veyra-product/../outside/cache.db",
        "/tmp/veyra-product",
    ] {
        assert!(
            ManagedCacheFile::new(
                Path::new("/tmp/veyra-product"),
                path.into(),
                "stable".into(),
                false,
                false
            )
            .is_err()
        );
    }
    for (id, fake, dns) in [
        ("", false, false),
        ("bad/id", false, false),
        ("stable", true, false),
        ("stable", false, true),
    ] {
        assert!(
            ManagedCacheFile::new(
                Path::new("/tmp/veyra-product"),
                "/tmp/veyra-product/cache.db".into(),
                id.into(),
                fake,
                dns
            )
            .is_err()
        );
    }
    let mut r = resources();
    r.mixed = LoopbackListener::new("127.0.0.1:23456".parse().unwrap()).unwrap();
    r.controller = r.mixed;
    assert_eq!(
        plan(&state(), &target(), &r).unwrap_err(),
        CompileError::InvalidRuntimeResources
    );
    let original = document(&state());
    for (pointer, value) in [
        ("/inbounds/0/listen", json!("0.0.0.0")),
        ("/experimental/cache_file/store_fakeip", json!(true)),
        ("/experimental/cache_file/path", json!("relative")),
        ("/experimental/cache_file/enabled", json!(false)),
        (
            "/experimental/clash_api/external_controller",
            json!("192.0.2.1:9"),
        ),
        ("/dns/servers/0/type", json!("legacy")),
        ("/route/rules/0/action", json!("execute")),
    ] {
        let mut d = original.clone();
        *d.pointer_mut(pointer).unwrap() = value;
        assert!(
            GeneratedConfig::from_bytes(serde_json::to_vec(&d).unwrap())
                .validate_final()
                .is_err(),
            "{pointer}"
        );
    }
    for pointer in [
        "/experimental/clash_api",
        "/experimental/cache_file",
        "/inbounds/0",
        "/dns/servers/0",
        "/route/rules/0",
    ] {
        let mut d = original.clone();
        d.pointer_mut(pointer).unwrap()["unknown"] = json!(true);
        assert!(
            GeneratedConfig::from_bytes(serde_json::to_vec(&d).unwrap())
                .validate_final()
                .is_err()
        );
    }
}
#[test]
fn p202b_tag_cache_health_identity_survives_rename_revision_resource_changes() {
    let mut s = state();
    let before = document(&s);
    let health = plan(&s, &target(), &resources())
        .unwrap()
        .runtime_health()
        .unwrap()
        .clone();
    s.nodes[0].name = "renamed".into();
    s.pools[0].name = "renamed".into();
    s.config_revision += 1;
    s.app_config.behavior.latency.test_url =
        UiLatencyUrl("https://ui.example.invalid/changed".into());
    let mut r = resources();
    r.mixed = LoopbackListener::new("[::1]:24567".parse().unwrap()).unwrap();
    r.controller = LoopbackListener::new("127.0.0.1:24568".parse().unwrap()).unwrap();
    let p = plan(&s, &target(), &r).unwrap();
    let after: Value = serde_json::from_slice(
        p.finalize(&crate::singbox::secret::test_api_secret())
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(before["outbounds"], after["outbounds"]);
    assert_eq!(
        before["experimental"]["cache_file"],
        after["experimental"]["cache_file"]
    );
    assert_eq!(p.runtime_health(), Some(&health));
    assert_eq!(health.proxy_url.as_str(), "http://127.0.0.1:20002/runtime");
    assert_eq!(health.direct_url.as_str(), "http://127.0.0.1:20003/direct");
    assert!(!format!("{p:?}").contains("runtime"));
}
#[test]
fn p202b_direct_for_nodes_precedes_sites_and_handles_host_ipv4_ipv6() {
    let mut s = state();
    let d = document(&s);
    let rules = d["route"]["rules"].as_array().unwrap();
    assert_eq!(
        rules[1],
        json!({"outbound":"direct","domain":["node.example.invalid","subscription.example.invalid"]})
    );
    assert_eq!(
        rules[2],
        json!({"outbound":"direct","ip_cidr":["127.0.0.1/32","::1/128"]})
    );
    assert_eq!(rules[3]["domain_suffix"], json!(["direct.example.invalid"]));
    assert!(!serde_json::to_string(&d).unwrap().contains("synthetic"));
    s.profile.direct_for_nodes = false;
    let off = document(&s);
    assert_eq!(
        off["route"]["rules"][1]["domain_suffix"],
        json!(["direct.example.invalid"])
    );
    assert!(
        !off["route"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r.get("ip_cidr").is_some())
    );
}
// 以生成规则的 first-match 可观察结果保护 Direct 例外，不仅数 reject 数量。
fn route_udp443(doc: &Value, host: &str) -> String {
    for r in doc["route"]["rules"].as_array().unwrap() {
        if r.get("ip_version").is_some() || r.get("ip_cidr").is_some() || r.get("inbound").is_some()
        {
            continue;
        }
        if r.get("domain")
            .is_some_and(|d| !d.as_array().unwrap().iter().any(|v| v == host))
        {
            continue;
        }
        if r.get("domain_suffix").is_some_and(|d| {
            !d.as_array()
                .unwrap()
                .iter()
                .any(|v| host.ends_with(v.as_str().unwrap()))
        }) {
            continue;
        }
        if r.get("port")
            .is_some_and(|p| !p.as_array().unwrap().contains(&json!(443)))
        {
            continue;
        }
        if r.get("network")
            .is_some_and(|n| !n.as_array().unwrap().contains(&json!("udp")))
        {
            continue;
        }
        if r["action"] == "reject" {
            return "reject".into();
        }
        if let Some(t) = r["outbound"].as_str() {
            return t.into();
        }
    }
    doc["route"]["final"].as_str().unwrap().into()
}
#[test]
fn p202b_reject_quic_only_proxy_path_preserves_direct_and_priority() {
    let mut s = state();
    let off = document(&s);
    assert_eq!(route_udp443(&off, "proxy.example.invalid"), "pool-auto");
    assert_eq!(route_udp443(&off, "elsewhere.invalid"), "pool-manual");
    s.profile.reject_quic = true;
    let on = document(&s);
    for h in ["proxy.example.invalid", "elsewhere.invalid"] {
        assert_eq!(route_udp443(&on, h), "reject");
    }
    for h in [
        "direct.example.invalid",
        "node.example.invalid",
        "subscription.example.invalid",
    ] {
        assert_eq!(route_udp443(&on, h), "direct");
    }
    let direct_default: Value = serde_json::from_slice(
        plan(&s, &OutboundId::Direct, &resources())
            .unwrap()
            .finalize(&crate::singbox::secret::test_api_secret())
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(route_udp443(&direct_default, "elsewhere.invalid"), "direct");
    assert_eq!(
        route_udp443(&direct_default, "proxy.example.invalid"),
        "reject"
    );
    assert!(
        proxy_udp443(&CoreRule {
            network: Some(vec![NetworkProtocol::Tcp]),
            ..CoreRule::default()
        })
        .is_none()
    );
    assert!(
        proxy_udp443(&CoreRule {
            port: Some(vec![80]),
            ..CoreRule::default()
        })
        .is_none()
    );
}
#[test]
fn p202b_ipv6_dns_route_and_unsupported_options_are_explicit() {
    let mut s = state();
    let off = document(&s);
    assert_eq!(off["dns"]["strategy"], "ipv4_only");
    assert_eq!(
        off["route"]["rules"][0],
        json!({"ip_version":6,"action":"reject"})
    );
    s.profile.ipv6 = true;
    let on = document(&s);
    assert_eq!(on["dns"]["strategy"], "prefer_ipv4");
    assert!(
        !on["route"]["rules"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r.get("ip_version").is_some())
    );
    for mode in [Ipv6Proxy::Ipv4, Ipv6Proxy::Bypass] {
        s.profile.ipv6_proxy = mode;
        assert_eq!(
            plan(&s, &target(), &resources()).unwrap_err(),
            CompileError::UnsupportedOption(UnsupportedProductOption::Ipv6Proxy)
        );
    }
    s.profile.ipv6_proxy = Ipv6Proxy::Node;
    s.profile.tun.tcp_mss = 1200;
    assert_eq!(
        plan(&s, &target(), &resources()).unwrap_err(),
        CompileError::UnsupportedOption(UnsupportedProductOption::Tun)
    );
    s.profile.tun = ProfileTun::default();
    s.profile.dns.direct = "dns.example.invalid".into();
    assert_eq!(
        plan(&s, &target(), &resources()).unwrap_err(),
        CompileError::UnsupportedOption(UnsupportedProductOption::DnsUpstream)
    );
}
#[test]
fn p202b_split_dns_uses_profile_paths_bootstrap_and_site_order() {
    let mut s = state();
    s.profile.dns.split = true;
    s.profile.dns.direct = "127.0.0.1".into();
    s.profile.dns.proxy = "::1".into();
    let d = document(&s);
    assert_eq!(
        d["dns"]["servers"],
        json!([{"type":"udp","tag":"dns-direct","server":"127.0.0.1","server_port":53},{"type":"udp","tag":"dns-proxy-pool-auto","server":"::1","server_port":53,"detour":"pool-auto"},{"type":"udp","tag":"dns-proxy","server":"::1","server_port":53,"detour":"pool-manual"}])
    );
    assert_eq!(d["dns"]["rules"][0]["server"], "dns-direct");
    assert_eq!(d["dns"]["rules"][2]["server"], "dns-proxy-pool-auto");
    assert_eq!(d["dns"]["rules"][3]["action"], "reject");
    assert_eq!(d["route"]["default_domain_resolver"], "dns-direct");
    assert!(d["dns"].get("fakeip").is_none());
    assert!(d["dns"]["servers"][0].get("address").is_none());
}
#[test]
fn p202b_unknown_disabled_dangling_unconfigured_and_graph_contract_reject() {
    let mut s = state();
    assert!(
        plan(
            &s,
            &OutboundId::Node(NodeId("missing".into())),
            &resources()
        )
        .is_err()
    );
    s.pools[0].enabled = false;
    assert!(plan(&s, &target(), &resources()).is_err());
    s.pools[0].enabled = true;
    s.routes[0].target = RouteTarget::Unconfigured;
    assert!(plan(&s, &target(), &resources()).is_err());
    s.routes[0].target = RouteTarget::Pool(PoolId("missing".into()));
    assert!(plan(&s, &target(), &resources()).is_err());
    for edges in [
        vec![(target(), vec![target()])],
        vec![
            (target(), vec![OutboundId::Direct]),
            (OutboundId::Direct, vec![target()]),
        ],
        vec![(target(), vec![OutboundId::Node(NodeId("missing".into()))])],
    ] {
        assert!(validate_outbound_graph(edges).is_err());
    }
}

#[test]
fn p202b_profile_defaults_patch_validation_and_saved_only_without_runtime() {
    use crate::application::{
        state_access::StateAccessGate,
        state_service::{ApplyEffect, ProfileService, SnapshotService},
    };
    use crate::storage::{JsonStateStore, StateStore};
    let p = Profile::default();
    assert_eq!(p.ipv6_proxy, Ipv6Proxy::Node);
    assert!(p.direct_for_nodes);
    assert!(!p.reject_quic);
    let patch:ProfilePatch=serde_json::from_value(json!({"rejectQuic":true,"directForNodes":false,"ipv6Proxy":"ipv4","testUrl":"https://runtime.example.invalid","directTestUrl":"http://direct.example.invalid"})).unwrap();
    let next = p.patched(patch).unwrap();
    assert!(next.reject_quic);
    assert!(!next.direct_for_nodes);
    assert_eq!(next.ipv6_proxy, Ipv6Proxy::Ipv4);
    assert_eq!(
        next.direct_test_url.as_str(),
        "http://direct.example.invalid"
    );
    for value in [
        json!({"ipv6Proxy":"unknown"}),
        json!({"testUrl":"ftp://example.invalid"}),
        json!({"directTestUrl":"https://secret@example.invalid"}),
        json!({"directBypass":true}),
        json!({"testUrl":null}),
    ] {
        let result = serde_json::from_value::<ProfilePatch>(value);
        assert!(result.is_err() || p.patched(result.unwrap()).is_err());
    }
    for field in ["ipv6Proxy", "rejectQuic", "directForNodes", "directTestUrl"] {
        let patch: ProfilePatch = serde_json::from_value(json!({field:null})).unwrap();
        assert!(p.patched(patch).is_err());
    }
    let s = state();
    let root = std::env::temp_dir().join(format!("veyra-p202b-save-{:?}", s.state_epoch));
    let store = JsonStateStore::new(root.join("state.json")).unwrap();
    store.save(&s).unwrap();
    let service = ProfileService::new(SnapshotService::new(
        store.clone(),
        StateAccessGate::default(),
    ));
    // 保存一个当前不能应用的已知选项仍为 SavedOnly；service 无 Compiler/SidecarPort/child 依赖。
    let outcome = service
        .patch(
            s.config_version(),
            serde_json::from_value(json!({"ipv6Proxy":"bypass","rejectQuic":true})).unwrap(),
        )
        .unwrap();
    assert_eq!(outcome.effect, ApplyEffect::SavedOnly);
    let saved = store.load().unwrap();
    assert_eq!(saved.profile.ipv6_proxy, Ipv6Proxy::Bypass);
    assert_eq!(saved.selection_revision, s.selection_revision);
    assert_eq!(
        plan(&saved, &target(), &resources()).unwrap_err(),
        CompileError::UnsupportedOption(UnsupportedProductOption::Ipv6Proxy)
    );
    assert!(!root.join("cache.db").exists());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn p202b_old_v9_and_v8_store_load_preserves_facts_versions_and_second_load() {
    use crate::storage::{JsonStateStore, StateStore};
    for version in [8, 9] {
        let mut original = state();
        original.config_revision = 23;
        original.selection_revision = 4;
        original.profile = Profile::default();
        original.profile.ipv6 = true;
        let mut old = serde_json::to_value(&original).unwrap();
        old["schema_version"] = json!(version);
        for key in [
            "ipv6Proxy",
            "directForNodes",
            "rejectQuic",
            "testUrl",
            "directTestUrl",
        ] {
            old["profile"].as_object_mut().unwrap().remove(key);
        }
        if version == 8 {
            old["app_config"]
                .as_object_mut()
                .unwrap()
                .remove("behavior");
        }
        let root =
            std::env::temp_dir().join(format!("veyra-p202b-v{version}-{:?}", original.state_epoch));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("state.json");
        std::fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let bytes_before = std::fs::read(&path).unwrap();
        let store = JsonStateStore::new(path.clone()).unwrap();
        let first = store.load().unwrap();
        assert_eq!(first, original);
        assert_eq!(CURRENT_SCHEMA_VERSION, 9);
        let after = std::fs::read(&path).unwrap();
        let second = store.load().unwrap();
        assert_eq!(first, second);
        assert_eq!(after, std::fs::read(&path).unwrap());
        if version == 9 {
            assert_eq!(after, bytes_before);
        } else {
            assert!(root.join("state.json.pre-migration").exists());
        }
        store.save(&first).unwrap();
        assert_eq!(store.load().unwrap(), first);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn p202b_split_dns_default_direct_and_non_domain_sites_do_not_guess() {
    let mut s = state();
    s.profile.dns.split = true;
    let p = plan(&s, &OutboundId::Direct, &resources()).unwrap();
    let d: Value = serde_json::from_slice(
        p.finalize(&crate::singbox::secret::test_api_secret())
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(d["dns"]["final"], "dns-direct");
    assert!(
        d["dns"]["servers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["detour"] == "pool-auto")
    );
    for matcher in [
        TrafficMatcher::IpCidr(vec!["192.0.2.0/24".into()]),
        TrafficMatcher::Port(vec![443]),
        TrafficMatcher::Application(vec!["Safari".into()]),
        TrafficMatcher::Protocol(vec![NetworkProtocol::Udp]),
    ] {
        s.routes[0].matcher = matcher;
        assert_eq!(
            plan(&s, &target(), &resources()).unwrap_err(),
            CompileError::UnsupportedOption(UnsupportedProductOption::SplitDnsRoute)
        );
    }
}
#[test]
fn p202b_subscription_ip_bootstrap_and_custom_profile_routes_are_mapped() {
    let mut s = state();
    s.profile.ipv6 = true;
    for url in [
        "https://192.0.2.10/feed?token=private",
        "https://[2001:db8::1]/feed?token=private",
    ] {
        s.subscriptions[0].source = SubscriptionSource::Remote { url: url.into() };
        let d = document(&s);
        let ip = if url.contains("db8") {
            "2001:db8::1/128"
        } else {
            "192.0.2.10/32"
        };
        assert!(
            d["route"]["rules"][1]["ip_cidr"]
                .as_array()
                .unwrap()
                .contains(&json!(ip))
        );
        assert!(!serde_json::to_string(&d).unwrap().contains("private"));
    }
    s.profile.routing.custom = Some(CustomRouting {
        name: None,
        enabled: true,
        rules: vec![CustomRoutingRule {
            matcher: DomainMatcher::Domain,
            value: "custom.example.invalid".into(),
            target: RouteTarget::Block,
        }],
    });
    let d = document(&s);
    assert_eq!(
        d["route"]["rules"][2],
        json!({"action":"reject","domain":["custom.example.invalid"]})
    );
    s.profile.dns.direct_extras.push(DnsUpstream {
        server: "1.0.0.1".into(),
        protocol: DnsTransport::Udp,
        port: 53,
    });
    assert_eq!(
        plan(&s, &target(), &resources()).unwrap_err(),
        CompileError::UnsupportedOption(UnsupportedProductOption::DnsExtras)
    );
}

#[test]
fn p202b_representative_fixtures_match_complete_expected_product_documents() {
    let mut s = state();
    let mut base = document(&s);
    base["experimental"]["clash_api"]["secret"] = json!("<runtime-secret>");
    let expected: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/compiler/p2-02b-base.expected.json"
    ))
    .unwrap();
    assert_eq!(base, expected);
    s.profile.ipv6 = true;
    s.profile.reject_quic = true;
    s.profile.dns.split = true;
    s.profile.dns.direct = "127.0.0.1".into();
    s.profile.dns.proxy = "::1".into();
    let mut options = document(&s);
    options["experimental"]["clash_api"]["secret"] = json!("<runtime-secret>");
    let expected: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/compiler/p2-02b-options.expected.json"
    ))
    .unwrap();
    assert_eq!(options, expected);
}

fn selected_state() -> AppState {
    let mut value = serde_json::to_value(AppState::empty()).unwrap();
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/compiler/p2-02b-selected.json"
    ))
    .unwrap();
    for (key, part) in fixture.as_object().unwrap() {
        value[key] = part.clone();
    }
    serde_json::from_value(value).unwrap()
}
fn projected_plan(
    s: &AppState,
    intent: &RuntimeIntent,
    target: &OutboundId,
) -> Result<SingBoxPlan, CompileError> {
    SingBoxCompiler.compile_product(ProductCompileRequest {
        state: s,
        runtime_intent: intent,
        default_outbound: target,
        resources: &resources(),
    })
}
fn plan_document(p: SingBoxPlan) -> Value {
    serde_json::from_slice(
        p.finalize(&crate::singbox::secret::test_api_secret())
            .unwrap()
            .as_bytes(),
    )
    .unwrap()
}
fn selected_projection(
    s: &AppState,
) -> crate::application::selected_subscription::SelectedRuntimeProjection {
    crate::application::selected_subscription::project_selected_runtime(s).unwrap()
}

// 保护 P2-01 首次导入→真实 selected projection→产品配置，且编译不得持久化 synthetic pool。
#[test]
fn p202b_projection_first_import_unconfigured_compiles_selected_selector_and_final() {
    let s = selected_state();
    s.validate().unwrap();
    assert_eq!(s.default_target, RouteTarget::Unconfigured);
    assert_eq!(s.subscriptions.len(), 3);
    let before = serde_json::to_vec(&s).unwrap();
    let projection = selected_projection(&s);
    let target = OutboundId::from_route_target(&projection.projected_default_target).unwrap();
    assert_eq!(
        target,
        OutboundId::Pool(PoolId("runtime-active-subscription".into()))
    );
    assert!(OutboundCatalog::from_state(&s).get(&target).is_err());
    let d = plan_document(projected_plan(&s, &projection.runtime_intent, &target).unwrap());
    assert_eq!(d["route"]["final"], "pool-runtime-active-subscription");
    let out = d["outbounds"].as_array().unwrap();
    let selector = out
        .iter()
        .find(|o| o["tag"] == "pool-runtime-active-subscription")
        .unwrap();
    assert_eq!(selector["type"], "selector");
    assert_eq!(selector["outbounds"], json!(["node-a", "node-b", "node-c"]));
    let servers = out
        .iter()
        .filter_map(|o| o.get("server").cloned())
        .collect::<Vec<_>>();
    assert_eq!(
        servers,
        vec![
            json!("127.0.0.1"),
            json!("node.example.invalid"),
            json!("::1")
        ]
    );
    let rules = &d["route"]["rules"];
    assert_eq!(
        rules[1],
        json!({"outbound":"direct","domain":["node.example.invalid","subscription.example.invalid"]})
    );
    assert_eq!(
        rules[2],
        json!({"outbound":"direct","ip_cidr":["127.0.0.1/32","::1/128"]})
    );
    let serialized = serde_json::to_string(&d).unwrap();
    for absent in ["unrelated", "cross-", "outside", "synthetic", "hidden-"] {
        assert!(!serialized.contains(absent), "{absent}");
    }
    assert_eq!(serde_json::to_vec(&s).unwrap(), before);
}

// 保护明确跨订阅 custom pool 的运行闭包；同 provider 未匹配的 sibling 也必须排除。
#[test]
fn p202b_projection_cross_subscription_pool_adds_only_matching_nodes_and_provider_hosts() {
    let mut s = selected_state();
    s.pools.push(NodePool {
        id: PoolId("cross-pool".into()),
        name: "cross pool".into(),
        kind: PoolKind::Custom,
        sources: vec![PoolSource {
            provider_id: ProviderId("provider-cross".into()),
            filter: NodeFilter {
                include_node_ids: vec![NodeId("cross".into())],
                ..NodeFilter::default()
            },
        }],
        selection: SelectionPolicy::Manual {
            selected_node_id: None,
            pending_node_id: None,
        },
        enabled: true,
    });
    s.profile.routing.custom = Some(CustomRouting {
        name: None,
        enabled: true,
        rules: vec![CustomRoutingRule {
            matcher: DomainMatcher::Domain,
            value: "custom.example.invalid".into(),
            target: RouteTarget::Pool(PoolId("cross-pool".into())),
        }],
    });
    let mut projection = selected_projection(&s);
    // runtime routes 可指向只存在于实际投影的 synthetic pool，不能从 state routes 重建。
    projection.runtime_intent.routes.push(RoutePolicy {
        id: RoutePolicyId("runtime-only".into()),
        name: "runtime only".into(),
        enabled: true,
        priority: 3,
        matcher: TrafficMatcher::Domain(vec!["runtime.example.invalid".into()]),
        target: projection.projected_default_target.clone(),
    });
    let target = OutboundId::from_route_target(&projection.projected_default_target).unwrap();
    let d = plan_document(projected_plan(&s, &projection.runtime_intent, &target).unwrap());
    let out = d["outbounds"].as_array().unwrap();
    assert_eq!(out.iter().filter(|o| o.get("server").is_some()).count(), 4);
    assert!(
        out.iter()
            .any(|o| o["tag"] == "node-cross" && o["server"] == "cross-node.example.invalid")
    );
    assert_eq!(
        out.iter().find(|o| o["tag"] == "pool-cross-pool").unwrap()["outbounds"],
        json!(["node-cross"])
    );
    assert_eq!(
        d["route"]["rules"][1]["domain"],
        json!([
            "cross-node.example.invalid",
            "cross-subscription.example.invalid",
            "node.example.invalid",
            "subscription.example.invalid"
        ])
    );
    for (domain, tag) in [
        ("custom.example.invalid", "pool-cross-pool"),
        (
            "runtime.example.invalid",
            "pool-runtime-active-subscription",
        ),
    ] {
        assert!(
            d["route"]["rules"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r["domain"] == json!([domain]) && r["outbound"] == tag)
        );
    }
    let serialized = serde_json::to_string(&d).unwrap();
    for absent in ["unrelated", "cross-sibling", "hidden-", "outside"] {
        assert!(!serialized.contains(absent), "{absent}");
    }
}

// 保护显式 intent 的节点事实；即使与保存快照不同也不得重新从快照生成运行节点。
#[test]
fn p202b_projection_uses_actual_runtime_node_hosts_and_exact_remote_ip_hosts() {
    let mut s = selected_state();
    for (url, expected) in [
        ("https://192.0.2.11/feed?token=hidden", "192.0.2.11/32"),
        (
            "https://[2001:db8::11]/feed?token=hidden",
            "2001:db8::11/128",
        ),
    ] {
        s.subscriptions[0].source = SubscriptionSource::Remote { url: url.into() };
        let mut p = selected_projection(&s);
        p.runtime_intent
            .nodes
            .iter_mut()
            .find(|n| n.id.0 == "b")
            .unwrap()
            .server = "actual-runtime.example.invalid".into();
        let target = OutboundId::from_route_target(&p.projected_default_target).unwrap();
        let d = plan_document(projected_plan(&s, &p.runtime_intent, &target).unwrap());
        let serialized = serde_json::to_string(&d).unwrap();
        assert!(serialized.contains("actual-runtime.example.invalid"));
        assert!(!serialized.contains("node.example.invalid"));
        assert!(!serialized.contains("hidden"));
        assert!(d["route"]["rules"].as_array().unwrap().iter().any(|r| {
            r["ip_cidr"]
                .as_array()
                .is_some_and(|ips| ips.contains(&json!(expected)))
        }));
    }
}

// 闭包外已保存 pool 不能当默认出口；unknown/unavailable/unconfigured 不猜 Direct。
#[test]
fn p202b_projection_rejects_missing_default_unavailable_members_and_unconfigured_routes() {
    let s = selected_state();
    let mut p = selected_projection(&s);
    for id in ["outside", "missing"] {
        assert_eq!(
            projected_plan(&s, &p.runtime_intent, &OutboundId::Pool(PoolId(id.into())))
                .unwrap_err(),
            CompileError::InvalidRouteTarget
        );
    }
    assert!(OutboundId::from_route_target(&RouteTarget::Unconfigured).is_err());
    let target = OutboundId::from_route_target(&p.projected_default_target).unwrap();
    let synthetic = p
        .runtime_intent
        .pools
        .iter_mut()
        .find(|pool| pool.id.0.starts_with("runtime-active-"))
        .unwrap();
    let members = std::mem::take(&mut synthetic.members);
    assert!(projected_plan(&s, &p.runtime_intent, &target).is_err());
    p.runtime_intent
        .pools
        .iter_mut()
        .find(|pool| pool.id.0.starts_with("runtime-active-"))
        .unwrap()
        .members = vec![NodeId("missing".into())];
    assert!(projected_plan(&s, &p.runtime_intent, &target).is_err());
    p.runtime_intent
        .pools
        .iter_mut()
        .find(|pool| pool.id.0.starts_with("runtime-active-"))
        .unwrap()
        .members = members;
    p.runtime_intent.routes[0].target = RouteTarget::Unconfigured;
    assert_eq!(
        projected_plan(&s, &p.runtime_intent, &target).unwrap_err(),
        CompileError::InvalidRouteTarget
    );
    p.runtime_intent.routes[0].target = RouteTarget::Pool(PoolId("outside".into()));
    assert_eq!(
        projected_plan(&s, &p.runtime_intent, &target).unwrap_err(),
        CompileError::InvalidRouteTarget
    );
}

// 保存 Profile rule 合法不代表它能在本次投影运行；闭包外规则必须明确拒绝。
#[test]
fn p202b_projection_profile_custom_rule_outside_runtime_closure_is_rejected() {
    let mut s = selected_state();
    s.profile.routing.custom = Some(CustomRouting {
        name: None,
        enabled: true,
        rules: vec![CustomRoutingRule {
            matcher: DomainMatcher::DomainSuffix,
            value: "custom.example.invalid".into(),
            target: RouteTarget::Pool(PoolId("outside".into())),
        }],
    });
    s.validate().unwrap();
    let p = selected_projection(&s);
    let target = OutboundId::from_route_target(&p.projected_default_target).unwrap();
    assert!(
        OutboundCatalog::from_state(&s)
            .resolve(&s.profile.routing.custom.as_ref().unwrap().rules[0].target)
            .is_ok()
    );
    assert_eq!(
        projected_plan(&s, &p.runtime_intent, &target).unwrap_err(),
        CompileError::InvalidRouteTarget
    );
}
