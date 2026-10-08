//! 在调用方的临时目录创建生产快照与组候选；只写文件，不启动内核或联网。
use std::{fs, io::Write, path::PathBuf};
use veyra_core::{
    application::{
        selected_subscription::project_selected_runtime, state_access::StateAccessGate,
        state_service::SnapshotService,
    },
    domain::*,
    singbox::{
        LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
        SingBoxCompiler, secret::generate_api_secret,
    },
    storage::{JsonStateStore, StateStore},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("需要自有临时目录")?);
    fs::create_dir_all(&root)?;
    let mut value = serde_json::to_value(AppState::try_empty()?)?;
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/compiler/p2-02b.json"))?;
    for (key, part) in fixture.as_object().ok_or("fixture")? {
        value[key] = part.clone();
    }
    let mut state: AppState = serde_json::from_value(value)?;
    state.pools.clear();
    state.routes.clear();
    state.default_target = RouteTarget::Direct;
    state.active_subscription_id = Some(state.subscriptions[0].id.clone());
    state.subscriptions[0].name = "验收订阅".into();
    for (index, node) in state.nodes.iter_mut().enumerate() {
        node.name = format!(
            "{} 验收节点 {}",
            if index % 2 == 0 { "HK" } else { "US" },
            index + 1
        );
    }
    state.app_config.visual.theme_mode = DesktopThemeMode::Light;
    state.profile = state.profile.patched(serde_json::from_value(serde_json::json!({
        "testUrl":"http://127.0.0.1:20002/runtime", "directTestUrl":"http://127.0.0.1:20003/direct",
        "dns":{"direct":"127.0.0.1","directPort":20004,"proxy":"127.0.0.1","proxyPort":20005}
    }))?)?;
    let store = JsonStateStore::new(root.join("state.json"))?;
    store.save(&state)?;
    let mut groups = default_groups();
    let mut manual = NodeGroup::new(PoolId("p402-static".into()));
    manual.name = "静态验收".into();
    manual.rule = GroupRule::Selector;
    manual.members = vec![
        OutboundId::Node(state.nodes[1].id.clone()),
        OutboundId::Direct,
    ];
    let mut outer = NodeGroup::new(PoolId("p402-nested".into()));
    outer.name = "嵌套验收".into();
    outer.members = vec![manual.outbound_id(), groups[1].outbound_id()];
    outer.test_url = "http://127.0.0.1:20006/group".into();
    groups.extend([manual, outer]);
    {
        let service = SnapshotService::new(store, StateAccessGate::default());
        service
            .save_groups(state.config_version(), groups)
            .map_err(|e| format!("{e:?}"))?;
    }
    // 销毁后从正式磁盘重新载入；Compiler 只消费这份恢复的事实。
    let state = SnapshotService::new(
        JsonStateStore::new(root.join("state.json"))?,
        StateAccessGate::default(),
    )
    .snapshot()?;
    let projection = project_selected_runtime(&state).map_err(|e| format!("{e:?}"))?;
    let resources = ProductRuntimeResources {
        mixed: LoopbackListener::new("127.0.0.1:0".parse()?)?,
        controller: LoopbackListener::new("127.0.0.1:0".parse()?)?,
        cache: ManagedCacheFile::new(&root, root.join("cache.db"), "p402".into(), false, false)?,
    };
    let target = OutboundId::Pool(PoolId("p402-nested".into()));
    let config = SingBoxCompiler
        .compile_product(ProductCompileRequest {
            state: &state,
            runtime_intent: &projection.runtime_intent,
            default_outbound: &target,
            resources: &resources,
        })?
        .finalize(&generate_api_secret()?)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(root.join("groups.json"))?;
    file.write_all(config.as_bytes())?;
    file.sync_all()?;
    println!(
        "groups={} nodes={} schema={} revision={} effect=SavedOnly",
        state.groups.len(),
        state.nodes.len(),
        state.schema_version,
        state.config_revision
    );
    Ok(())
}
