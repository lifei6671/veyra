//! P0-08 独立 CLI 原型：只检查/下载，不安装、不启动 helper 或内核。
//! 使用 --features p0-06-prototype；不向产品公共 DTO 注册原型类型。
#[path = "p0_08/update.rs"]
mod update;

use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::watch;
use update::{Manifest, Release, UpdateClient, Version};
use veyra_core::subscription::outbound::{BootstrapResolver, RoutePolicy, RunningProxy};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let usage = "check CURRENT [direct | via INSTANCE_ID LOOPBACK] | download CURRENT OUTPUT_DIR [direct | via INSTANCE_ID LOOPBACK] | inspect CURRENT RELEASE_JSON MANIFEST_JSON";
    let action = args.first().ok_or(usage)?.as_str();
    let current = Version::parse(args.get(1).ok_or(usage)?)?;
    // 离线检查同样走生产清单校验，便于 Host 审查固定来源与版本绑定。
    if action == "inspect" && args.len() == 4 {
        let release: Release = serde_json::from_slice(&std::fs::read(&args[2])?)?;
        let manifest: Manifest = serde_json::from_slice(&std::fs::read(&args[3])?)?;
        manifest.validate(&release)?;
        println!(
            "{}",
            serde_json::json!({"update_available": manifest.version()? > current, "release": release, "manifest": manifest})
        );
        return Ok(());
    }
    let offset = match action {
        "check" => 2,
        "download" => 3,
        _ => return Err(usage.into()),
    };
    let policy = match args.get(offset).map(String::as_str) {
        None | Some("direct") if args.len() <= offset + 1 => RoutePolicy::Direct,
        Some("via") if args.len() == offset + 3 => RoutePolicy::ViaRunningProxy(
            RunningProxy::from_current(Some((args[offset + 1].clone(), args[offset + 2].parse()?)))
                .map_err(|e| format!("当前有效实例参数错误：{e:?}"))?,
        ),
        _ => return Err(usage.into()),
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let client = UpdateClient::new(&policy, Arc::new(BootstrapResolver::new(Duration::from_secs(5)).map_err(|e| format!("DNS 初始化失败：{e:?}"))?))?;
        let (cancel, mut cancelled) = watch::channel(false);
        // Enter 取消本次网络操作；EOF 不算取消。没有新后台服务或持久 updater 状态。
        std::thread::spawn(move || {
            let mut line = String::new();
            if std::io::stdin().read_line(&mut line).is_ok_and(|n| n > 0) {
                let _ = cancel.send(true);
            }
        });
        let (release, manifest) = client.check(&mut cancelled).await?;
        println!("{}", serde_json::json!({"update_available": manifest.version()? > current, "release": release, "manifest": manifest}));
        if action == "download" && manifest.version()? > current {
            let path = Path::new(args.get(2).ok_or(usage)?).join(&manifest.asset_name);
            client.download(&manifest, &path, &mut cancelled).await?;
            println!("{}", serde_json::json!({"result":"VERIFIED_MANUAL_INSTALL_REQUIRED", "path":path, "installed":false}));
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}
