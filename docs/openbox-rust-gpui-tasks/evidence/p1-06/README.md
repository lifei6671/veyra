# OBG-P1-06 托盘与窗口关闭流程

2026-10-04 · Codex Desktop · GPUI / Runtime-Platform · **DONE**。

起始 HEAD `f9adda4ebe8a836a2f1113747cb672ea8b834c15`。P1-05 与 Host Review 已有修改全部保留；[继承文件哈希](inherited-files.json)、[范围外文件保持不变](preserved-inherited-files.json)。没有 commit/push；P1-07/P2 未启动。当前 Host 会话的 User/Host manual acceptance 已补足真实托盘菜单、连续三轮恢复与托盘 Quit，见[人工验收证据](host-manual-acceptance.json)；Human Visual / Tray interaction PASS，三项任务验收全部完成。

## 实现

`gpui_kit::application().run` 是唯一 UI loop。`AppView → DesktopTray → Option<TrayIcon>` 长期持有 macOS 主线程原生对象。使用既定 `tray-icon 0.24.2`；不依赖 P0 prototype、不新增 tao/winit loop 或直接 NSStatusItem adapter。自绘 18px V template 无外部图标下载。

菜单 callback 仅按 MenuId 转为固定 `Intent::Show / Quit`，经 Tokio unbounded channel 唤醒 GPUI foreground `recv()`。不轮询、不把 Entity/Context 移入 Send 线程。P1-05 Activate receiver 和托盘 receiver 都只更新已有 WindowHandle。

`Close → Hide`：关闭回调返回 false；`cx.hide()` 隐藏应用窗口，不销毁 AppView/页面/草稿/服务/runtime/单实例 owner。`Show / Activate → RevealExisting`：activate app + activate 原窗口，不构造窗口、服务或 writer。`Quit → cx.quit()` 是唯一显式退出入口，窗口按钮和 tray Quit 共用。`on_app_quit` 执行 PrimaryInstance::prepare_quit 停 listener/删 socket；composition root Arc 留住 flock FD 直到实际退出。AppView quit subscription 显式释放 TrayIcon，适配 NSApplication terminate 不展开 main stack 的行为。

菜单只投影现有 Core AppState / optional RuntimeSnapshot，不保存 Runtime bool。当前 composition root 没有 P2 Runtime owner，因此传 None，显示 **“内核：未接入（当前不可用）”**，启动/停止为 disabled，并注明 Runtime 未接入。配置保存或历史成功记录不能推断 Running。后续真实 Runtime owner 接入仍属于 P2，本卡不执行内核/代理/TUN。

## 自动验证

[validation.json](validation.json)、[检查日志](checks/)、[静态自查](static-review.json)。41 desktop（原有36 + 新增5），294 core，全部通过；零 ignored/filtered。新增测试保护关闭与退出语义、显示/激活仅恢复已有窗口、实际 flock/socket/StateStore writer 保留到退出、缺 Runtime owner 不显示 Running、Core 状态/历史记录投影。

| 验证 | 结果 |
| --- | --- |
| cargo test -p veyra-desktop | 41 PASS |
| cargo test -p veyra-core --lib -- --test-threads=1 | 294 PASS |
| cargo check -p veyra-desktop / veyra-core | PASS |
| cargo build -p veyra-desktop | PASS |
| cargo clippy -p veyra-desktop --all-targets -- -D warnings | PASS |
| cargo clippy -p veyra-core --lib -- -D warnings | PASS |
| cargo fmt --all -- --check | PASS |
| cargo check --workspace --locked | PASS，既定 TAURI_CONFIG resource override |
| cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings | PASS，同一 override |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | PASS |
| cargo tree -p veyra-desktop | 单一 gpui-pre 0.3.7 / gpui-kit 0.7.0 / tray-icon 0.24.2；无 Tauri/helper/prototype/tao/winit 依赖边 |
| git diff --check | PASS |

命令使用 `MACOSX_DEPLOYMENT_TARGET=15.0`、`CARGO_NET_OFFLINE=true`。资源覆盖只证明 compile check，不证明旧 Tauri package resources 完整。首次 Option<&Box<AppState>> 编译错误与 debug sampling 的 clippy let_unit_value 失败均保留日志，修复后检查 PASS。上游 `block 0.1.6` future incompatibility warning 保留。

用户确认 Host 在本轮此前独立复核实现并复跑自动验证：41 Desktop tests、294 Core tests，以及 desktop/core check、clippy、fmt、resource override workspace locked check、旧 Tauri lib clippy、git diff --check 全部 exit 0。该来源登记在人工验收证据中；本次仅文档收口，未重复运行产品测试或构建。

## 真实 macOS 操作

macOS 27.0 (26A428), arm64, Rust 1.99.0；临时 evidence app 注入独立 state root，不访问正式/旧应用目录。GUI 三轮构建阶段分别用于业务操作与可见性诊断，详见 [interaction-results.json](interaction-results.json)、[真实日志](interaction.log)、[single-instance.json](single-instance.json)。

| 操作 | 实际结果 |
| --- | --- |
| 点击原生关闭按钮 | PASS：最终构建异步采样 native hidden=true / active_key=false；精确 executable 仍存活 |
| 隐藏状态下 secondary 5 次 | PASS：全部 exit0、writer_created_count=0、始终1 primary、state bytes 不变 |
| 将计算器置于前台再 secondary | PASS：原窗口 active/key，日志5次 primary_front_key=true |
| 原窗口/页面/筛选保留 | PASS：同一 WindowId(1v1)，AppView/六页每 primary 仅创建一次；Proxies EntityId(5v1) 和 p1-06-retained 保留 |
| 最终构建关闭→secondary Activate | PASS：hidden=true→false、active_key=false→true、原窗口恢复 |
| 窗口内 Quit | PASS：统一 Quit intent、listener/socket/tray cleanup，process0，flock 可重获 |
| Tray 显示窗口至少3轮 | **PASS（人工）**：用户连续三轮“关闭窗口 → 托盘‘显示窗口’恢复”，均正常，无异常 |
| 打开 tray menu / 观察准确状态及 disabled 启停 | **PASS（人工）**：用户看到 macOS 菜单栏 Veyra “V” 图标并真实展开菜单；Host 已查看用户菜单截图，状态与灰色禁用启停准确 |
| Tray Quit | **PASS（人工）**：用户最终通过托盘菜单“退出”，正常，无异常；未做新的程序化退出后检查 |

工具可捕获隐藏窗口的离屏图像，不能据此判断屏幕可见性。最初同步 `isHidden=false` 是 AppKit 切换中的瞬态，未当作结论；最终在100ms单次采样中读到 hidden=true。该采样只在 debug build，非菜单事件轮询。NSApplication 的 window_count=3 包含框架辅助窗口，不能将其解释成3个主窗口；GPUI 装配只有一个主 WindowId。SystemUIServer 读取超时 -10005；Ctrl+F8 未产生可观察变化。此前 AX 未提供 status item，两次请求人工展开菜单时尚无响应，因此当时三项托盘操作为 NOT_RUN；这些工具限制与历史结果保留于 [interaction-results.json](interaction-results.json)。本次新增的是当前 Host 会话用户真实操作结果，不复用 P0 人工结果，也不改写为自动通过。

用户实际展开的菜单显示“显示窗口”、“内核：未接入（当前不可用）”、灰色禁用的“启动（Runtime 未接入）”与“停止（Runtime 未接入）”、以及“退出”。人工证据只记录观察与已确认操作，不声称点击禁用启停或启动 Runtime。

## 截图

[visual-manifest.json](visual-manifest.json) 记录 SHA256。1280×720 logical content，DPR2，完整截图2560×1506，包含titlebar；仅将原生工具JPEG转换PNG，不裁切/缩放/改图。

- [main-window.png](visual/main-window.png)：第一轮正常主窗口。
- [secondary-restored.png](visual/secondary-restored.png)：secondary 恢复后的原 Proxies 窗口/筛选；**不是托盘恢复截图**。
- [final-main-window.png](visual/final-main-window.png)：最终构建正常主窗口。

用户在当前 Host 会话上传两张托盘截图，Host 会话已查看用户截图，未复制入仓库；以用户确认作为 Human Acceptance Evidence，详见 [host-manual-acceptance.json](host-manual-acceptance.json)。不生成本地图片、路径或 SHA，不向既有 visual manifest 添加不存在的文件。此前本机菜单截图缺失的工具限制保留；未扩展深色/缩放矩阵。

## 清理与后续

[cleanup.json](cleanup.json) 原样保留此前窗口显式 Quit 的程序化清理结果：desktop/test child/sing-box 0，socket 无残留、flock 可重获，TrayIcon 主线程释放；当时 state root、staging、临时 app/图片删除。其 `tray Quit NOT_RUN` 描述属于当时历史阶段。本次用户通过托盘退出正常、无异常，单独记为人工 PASS；没有新的程序化 post-check，不据此追加进程数、socket 或 flock 断言。本次只更新证据与任务文档，无公网、sing-box、System Proxy 或 TUN 操作。

[68卡 DAG](dag.json)：DONE12 / ACCEPTANCE1 / READY1 / TODO47 / Windows DEFERRED7。P1-07 的七项显式依赖均 DONE，成为唯一 READY，但未领取或启动；P0-05 仍 ACCEPTANCE，P0-06/P0-08 仍 TODO，P2 未启动。此前静态自查仍为实现者自查；本轮此前 Host 独立复核与自动重跑按用户确认单独记录。本次收口未改产品功能源码、AGENTS.md 或既有 P1-05 实现，未 commit/push。
