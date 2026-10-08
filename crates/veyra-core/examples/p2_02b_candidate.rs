//! 从代表性产品 fixture 生成候选；只生成文件，不 check/run、不访问网络。
use std::{fs, io::Write, path::PathBuf};
use veyra_core::{
    application::selected_subscription::project_selected_runtime,
    domain::{AppState, OutboundId, ProfilePatch, RuntimeIntent},
    singbox::{
        LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
        SingBoxCompiler, secret::generate_api_secret,
    },
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = PathBuf::from(args.next().ok_or("需要隔离的绝对 runtime root")?);
    let mode = args
        .next()
        .ok_or("需要 base/options/direct/block/selected 模式")?;
    let mut value = serde_json::to_value(AppState::try_empty()?)?;
    let fixture: serde_json::Value = serde_json::from_str(if mode == "selected" {
        include_str!("../tests/fixtures/compiler/p2-02b-selected.json")
    } else {
        include_str!("../tests/fixtures/compiler/p2-02b.json")
    })?;
    for (key, part) in fixture.as_object().ok_or("fixture object")? {
        value[key] = part.clone();
    }
    let mut state: AppState = serde_json::from_value(value)?;
    // 只用 loopback DNS/health；check 不需要远程依赖。节点域名仅用于静态规则覆盖。
    let patch: ProfilePatch = serde_json::from_value(
        serde_json::json!({"testUrl":"http://127.0.0.1:20002/runtime","directTestUrl":"http://127.0.0.1:20003/direct","dns":{"direct":"127.0.0.1","directPort":20004,"proxy":"::1","proxyPort":20005}}),
    )?;
    state.profile = state.profile.patched(patch)?;
    // 首次导入必须走真实 application 投影；完整已配置路径由调用方显式构造 intent。
    let (intent, mut target) = if mode == "selected" {
        let projection = project_selected_runtime(&state).map_err(|_| "selected projection")?;
        (
            projection.runtime_intent,
            OutboundId::from_route_target(&projection.projected_default_target)
                .map_err(|_| "projected target")?,
        )
    } else {
        (
            RuntimeIntent::from_state(&state)?,
            OutboundId::from_route_target(&state.default_target).map_err(|_| "fixture target")?,
        )
    };
    match mode.as_str() {
        "base" | "selected" => {}
        "options" => {
            state.profile.reject_quic = true;
            state.profile.dns.split = true;
            state.profile.ipv6 = true;
        }
        "direct" => {
            target = OutboundId::Direct;
            state.profile.direct_for_nodes = false;
            state.profile.reject_quic = true;
        }
        "block" => target = OutboundId::Block,
        _ => return Err("未知模式".into()),
    }
    let resources = ProductRuntimeResources {
        mixed: LoopbackListener::new("127.0.0.1:0".parse()?)?,
        controller: LoopbackListener::new("127.0.0.1:0".parse()?)?,
        cache: ManagedCacheFile::new(
            &root,
            root.join("profile-v114/cache.db"),
            "p202b-profile-v114".into(),
            false,
            false,
        )?,
    };
    let plan = SingBoxCompiler.compile_product(ProductCompileRequest {
        state: &state,
        runtime_intent: &intent,
        default_outbound: &target,
        resources: &resources,
    })?;
    let config = plan.finalize(&generate_api_secret()?)?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(root.join(format!("{mode}.json")))?;
    file.write_all(config.as_bytes())?;
    file.sync_all()?;
    Ok(())
}
