//! P2-03 本机隔离验收 fixture：只写应用 root，不执行内核。
use veyra_core::{
    domain::{AppState, DesktopThemeMode},
    storage::{JsonStateStore, StateStore},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().ok_or("root")?);
    let port: u16 = args.next().ok_or("port")?.parse()?;
    let mut value = serde_json::to_value(AppState::try_empty()?)?;
    let mut fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../tests/fixtures/compiler/p2-02b-selected.json"
    ))?;
    for field in ["subscriptions", "providers", "nodes"] {
        fixture[field].as_array_mut().unwrap().truncate(1);
    }
    fixture["pools"] = serde_json::json!([]);
    fixture["routes"] = serde_json::json!([]);
    fixture["nodes"][0]["port"] = port.into();
    fixture["nodes"][0]["name"] = "P2-03 Loopback SOCKS".into();
    fixture["subscriptions"][0]["name"] = "P2-03 Local Fixture".into();
    for (k, v) in fixture.as_object().unwrap() {
        value[k] = v.clone();
    }
    let mut state: AppState = serde_json::from_value(value)?;
    state.profile.direct_for_nodes = false;
    state.app_config.visual.theme_mode = DesktopThemeMode::Light;
    JsonStateStore::new(root.join("state.json"))?.save(&state)?;
    Ok(())
}
