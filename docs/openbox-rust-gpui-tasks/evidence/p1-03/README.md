# OBG-P1-03 GPUI 壳层与异步状态桥

**状态：DONE。owner：Codex Desktop；泳道：GPUI；日期：2026-10-04。**

基线 `f460e767957569e9f4a035132e26cc216277e98f`（`test(platform): add macOS helper prototype`），开工工作树干净；本轮未 commit/push。正式实现是 `crates/veyra-desktop/`，不依赖或删除 P0 prototype。Core DTO/存储契约未修改。

实现与规定构建检查已完成。实际 GPUI 已完成六页、九分类、filter/draft、浅深色、Busy、迟到请求、synthetic instance、真实坏 JSON 启动及 Retry，共 11 张截图；之后为满足离线纯测试要求，将同一六槽页面持有表参数化（生产仍是 Entity<PageView>），新增两项纯状态保留测试。最终版本重新启动及 Core 读取成功，但复跑原生 Entity 自动断言时 CUA 明确报告 Mac locked，自动解锁失败；已请求手动解锁，不能把该复核记为 PASS。用户随后回复“已解锁”，重新启动最终构建，三组原生 Entity 断言及实际 filter/DNS category/draft 往返全部 PASS，新增 final-settings-retained.png 后正常退出。锁屏失败作为历史保留，最终四项验收均 PASS。

## 四项验收

| 条件 | 当前证据 | 最终状态 |
| --- | --- | --- |
| 无配置进入本地 UI，无 HTTP/login | 两个正常隔离启动，首次截图和最后启动日志；真实 core 初始化空快照 | PASS |
| 页面状态/草稿保留，迟到/旧实例隔离 | 11 Cargo tests；先前原生 Entity 三组断言与人工式 CUA 输入/切换；最终页面持有表改动后原生复核已在解锁后通过 | PASS |
| render/click 不阻塞，Send 边界正确 | 单 Runtime、一个阻塞 worker；typed channel → GPUI foreground weak Entity update → notify；static-review.json | PASS |
| 实际壳/导航/浅深色/尺寸 | 12 真实 PNG，1280×720 logical、2x、2560×1506 physical、含 66 physical px titlebar；无裁图 | PASS（截图对应初次验收构建，构建身份分开记录） |

## 实现

- `main.rs`：Kit composition root、1280×720 窗口、P1-03 隔离 root；没有正式用户目录。默认 PID 临时目录退出后删除，显式 evidence root 由验收者清理。关闭窗口/退出结束本轮程序，不实现 P1-06 托盘。
- `app.rs`：AppView route/session theme、六页 Entity、全局 Core 只读快照和 channel 接收 Task。页面离开使 generation 失效，保留页面/输入实体；无快照时导航仍可继续加载。
- `services.rs`：Arc SnapshotService/ProfileService/SelectionService，共用 JsonStateStore/StateAccessGate；只执行本卡 snapshot 读取。一个 Tokio runtime（2 worker、max_blocking_threads=1），真实同步读在阻塞 worker；不走 HTTP。
- `state_bridge.rs`：请求 ID、页面 generation、捕获 state epoch、synthetic InstanceId + generation/epoch 检查。合法最新权威读取可采用新 epoch，旧请求无法覆盖。AppError 保留 Core 稳定错误模型，无任意 JSON 命令。
- `navigation.rs`：六页顺序 Overview/Proxies/Connections/Logs/Rules/Settings；九分类严格来自 SettingsPage.tsx：面板设置、订阅管理、出站节点、目标分流、终端分流、链式代理、共享网络、DNS 设置、后端设置。
- `ui/pages/mod.rs`：六个独立持久 PageView Entity，共享最小 placeholder 渲染写法；每页独立 InputState、设置 category、sort/selection/level 会话标记。Pages 泛型仅为纯持有表测试注入 Rc<RefCell<Session>>，生产固定 Entity<PageView>；原生 Entity 测试另行验证，纯模型不冒充 GPUI。
- `ui/shell.rs`：256 logical px sidebar、绿色 active route、圆角主面板、系统字体与会话浅深色。无模糊、背景图、正式 Preference 或 MiSans。

## 真实交互与异步结果

详细动作见 [interaction-results.json](interaction-results.json)、[gui.log](gui.log)、[gui-error.log](gui-error.log)、[async-bridge.json](async-bridge.json)。

- `p1-03-filter` 在 Proxies → Overview → Proxies 保留。
- Chain 分类与 `P1-03 draft retained` 在 Settings → Overview → Settings 保留，不写入 Profile。
- A request 2 / generation 16 / delay 500ms；B request 3 / generation 16 / delay 50ms。B Accepted，A StaleRequest，页面保持 accepted request 3。延迟位于真实 snapshot 读取之后，仅 debug。
- Busy 3s 时切到 Proxies，窗口立即响应；request 4 离页后 StaleGeneration。
- **synthetic bridge evidence**：current B generation 2；A generation 1 OldInstance，B Accepted。单测另覆盖同 ID 旧 generation 和旧 epoch。不是 kernel event，不表示 Runtime owner 已接入。
- 独立 root 写 `{malformed` 后真实启动显示 StorageFailed/Retry；删除测试 state.json 后实际点 Retry 恢复空状态。Core 自动保留的 corrupt 文件随该测试 root 一起清除。

## 测试与构建

[commands.json](commands.json)记录实际命令和历史失败。所有 Cargo 命令 `CARGO_NET_OFFLINE=true`，适用构建环境 `MACOSX_DEPLOYMENT_TARGET=15.0`，外层 Python subprocess timeout=900 秒；没有 child/kernel/network 测试。

| 检查 | 结果 |
| --- | --- |
| cargo check -p veyra-desktop | PASS |
| cargo build -p veyra-desktop | PASS |
| cargo test -p veyra-desktop | **11 passed / 0 failed / 0 ignored / 0 filtered**；10 纯逻辑/内存测试 + 1 实际隔离 StateStore/channel 测试 |
| cargo clippy -p veyra-desktop --all-targets -- -D warnings | PASS |
| cargo fmt --all -- --check | PASS |
| cargo tree -p veyra-desktop；cargo tree -p veyra-core | PASS |
| TAURI_CONFIG='{"bundle":{"resources":[]}}' cargo check --workspace --locked | PASS；仅检查资源覆盖，不证明旧 Tauri 打包资源完整 |
| 同覆盖的 cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings | PASS |
| 真实 GPUI Entity 三组自动断言 | 初次实际点击 PASS；最终持有表改动后解锁复跑 PASS，见 gui-final-verified.log |

依赖闭包仅一套 Kit 0.7.0 / gpui-pre 0.3.7；desktop → core、core 无 UI 反向依赖。无新增 tao/winit、tray、helper 或 prototype 依赖。根 default-members 仍仅 core。Cargo.lock 只新增本地 desktop package，第三方包版本未变。[dependencies.json](dependencies.json)

尝试历史：初始 InstanceId import 路径错误已修为既有 application::runtime_snapshot；GPUI test-support 解析缺离线 proptest，已撤回该依赖且没有公网下载；测试默认值初始化 clippy 失败已修；新增纯测试的 glob import 误引 GPUI test macro，已限定 import 后通过。失败日志保留，不改写为初次通过。既有 block 0.1.6 future-incompat warning 未扩大处理。

## 视觉与边界

[visual-manifest.json](visual-manifest.json)记录截图、原始 JPEG 与最终 PNG SHA256、大小、DPR、转码。CUA 原生返回 JPEG，使用 sips **只转 PNG 编码**，未裁剪、重绘、拼接或调整主题。截图标题栏的紫色指示来自捕获环境。

截图：light-overview、dark-overview、light-settings、dark-settings、loading、error、proxies-filter-retained、settings-draft-retained、late-request、synthetic-instance、retry-recovered、final-settings-retained。

相对 OpenBox 基线：保留侧栏/内容区/绿色强调/大圆角方向；采用纯色本地 shell、系统字体、文字导航与 Kit 基础输入/按钮，去除 Web 广告、登录/后端连接以及所有业务卡片。分类可换行，未实现后续水平滚动/组件偏好系统。深色使用可读的深底浅字，未复刻参考截图中的浅卡低对比缺陷。本卡不声称像素级或完整业务 UI 完成。

六页业务与九设置业务均是有后续 Task 的 placeholder。未实现 P1-04A/B、P1-05、P1-06、P2 Runtime/订阅/内核、P3 Observation、P4 routing/chain、P5 DNS/sharing、helper、updater。

网络边界：本轮无 HTTP server/站点访问/登录/sing-box/代理/TUN 修改；桌面源码没有相应执行入口，所有 Cargo 解析离线。运行 PID 的 lsof 无 IP socket，pgrep 无 child；此为局部进程观测，不是对用户 TUN/网络健康的结论。[process-observations.json](process-observations.json)

## 审查与后续

使用 code-delivery-review 做实现者自查，不称独立 Host Review。检查请求/epoch/实例隔离、持久 Entity、仅会话草稿、Send 边界、单一 Runtime/Core 真相和静态禁止项。无已知未修复业务逻辑错误；最终原生实体复核已在用户解锁后通过；无本卡剩余验收缺口。

68 卡 DAG 已核实唯一/依赖存在/无环。当前 P1-03 DONE，重算后只有 P1-04A/P1-04B/P1-05 READY，下游未启动；P1-06 仍缺 P1-05，P2 仍缺各卡显式依赖，P0-05 ACCEPTANCE 不释放 P0-06/P0-08。[dag.json](dag.json)

现场已清理：[cleanup.json](cleanup.json)。初次和错误验收通过 UI 退出；锁屏后的最终复核进程按已核对 executable/PID 精确 SIGTERM 退出，无 child/desktop 残留。四个临时 state roots、坏 JSON/corrupt 副本、临时 app bundle/staging 均删除。用户 TUN/系统代理没有任何修改动作。用户解锁后最后一项已补验并正常 UI 退出，临时目录与 app 再次清除。
