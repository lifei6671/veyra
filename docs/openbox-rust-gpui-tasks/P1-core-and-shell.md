# P1 核心库与桌面壳

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：目标是可独立启动、保存配置并通过托盘恢复的桌面壳；视觉与跨页面行为偏好最小拆分，不用占位页面冒充业务完成。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

<a id="obg-p1-01"></a>
## OBG-P1-01 抽取无 Tauri 依赖的核心库

**类型**：核心重构；**依赖**：OBG-P0-01、OBG-P0-02。**依据/范围**：方案 §4–5；`src-tauri/src/{application,domain,subscription,singbox,storage}/` → 新 `crates/veyra-core/` 与根 workspace。

**泳道 / 写范围**：Core/Config；根 Cargo workspace/锁文件、core 抽取与旧入口接线；workspace 单一 owner。

**执行与交付**：只移动本阶段需要的核心与有效测试；调度改用 Tokio runtime/Handle，抽离现有 SidecarPort/SystemProxyController；保留旧入口对共享核心的调用及 Windows 条件编译。

**验收**：

- [x] core 在 macOS 独立构建，依赖树不含 Tauri/GPUI，公共路径不引用具体 Windows 实现。
- [x] 相关纯单测实际执行且有测试数量；旧入口受影响的回归通过。
- [x] 平台装配留在入口，不新增通用 Executor/平台工厂；记录新 workspace 的真实验证命令。

**当前交付**：DONE；详见[抽取记录](#p1-01-delivery)。

<a id="obg-p1-02"></a>
## OBG-P1-02 类型化服务、原子快照与版本

**类型**：状态与持久化；**依赖**：OBG-P1-01。**依据/范围**：方案 §6、§12；core 的 domain/application/storage。

**泳道 / 写范围**：Core/Config；domain/application/storage 公共领域模型、共享 DTO、StateVersion/Preferences；单一 owner。

**执行与交付**：定义稳定 ID、ProfilePatch、ApplyEffect、AppError 和 RuntimeSnapshot；单文件原子业务快照、共享 state_epoch、配置/选择计数与整体替换入口。若桌面 update preference 属于版本化 AppConfig，其字段、默认值与迁移在此定义，不依赖 P6 Updater 行为先实现。

**验收**：

- [x] Missing/Null/Value 与数组替换行为明确；保存局部字段不丢失未编辑的 DNS/routing/TUN 字段。
- [x] 写入失败保留有效旧快照；整体替换后旧表单/任务不能跨 epoch 写回，append 正确推进当前版本。
- [x] 保存、当前运行、最后成功版本分别表达；没有运行实例时不凭历史记录显示 Ready。
- [x] 服务错误保留稳定码/字段位置/脱敏详情；内部类型不退回任意 JSON 命令。

**当前交付**：DONE；四项验收 PASS，详见[类型/快照记录](#p1-02-delivery)。

<a id="obg-p1-03"></a>
## OBG-P1-03 GPUI 壳层与异步状态桥

**类型**：UI；**依赖**：OBG-P1-02、OBG-P0-03。**依据/范围**：方案 §9.1–9.3；新 desktop `app.rs`、`ui/pages/`；参考 `OpenBoxApp.tsx`、`SettingsPage.tsx`。

**泳道 / 写范围**：GPUI；desktop app/navigation/state bridge；公共 DTO 变更交对应 owner。

**执行与交付**：建立六主页面/九分类导航和页面实体；AppServices 通道与 GPUI 更新桥；启动引导、错误/忙碌状态及草稿生命周期。

**验收**：

- [x] 无配置可进入本地界面；无需启动 OpenBox HTTP 服务或浏览器登录流程。
- [x] 页面切换保留适用的筛选/草稿；迟到回调、旧实例事件不污染新页面。
- [x] render/点击回调不阻塞等待网络，GPUI 实体不被错误传入 Send 任务。
- [x] 壳层与导航完成实际操作、浅深色及适用尺寸的 UI 验收；未实现业务明确标记。

**当前交付**：DONE；11 Cargo tests、最终原生 Entity 三组断言及真实六页/九分类/浅深色/异步/错误重试均通过。最终复核曾因 Mac 锁屏未运行，用户解锁后已完成；12 图及完整构建身份见 [P1-03 交付](evidence/p1-03/README.md)。

<a id="obg-p1-04a"></a>
## OBG-P1-04A 视觉桌面偏好、主题与基础组件

**类型**：UI 与偏好；**依赖**：OBG-P1-02、OBG-P1-03。**依据/范围**：方案 §9.3–9.4；`PanelSettings.tsx`、`openbox.css` → desktop theme/components/settings 与 core preferences/assets。

**泳道 / 写范围**：GPUI；theme/components、视觉偏好 UI/assets；Preferences DTO 交单一 owner。

**执行与交付**：建立输入、选择、弹层、通知、拖拽基础；迁移语言、主题、背景、透明度、模糊、圆角、侧栏折叠/展开（sidebar）及布局（layout）类纯视觉桌面偏好；打包许可明确的字体与图标。proxy columns、hide unavailable 等跨页面行为偏好由 OBG-P1-04B 负责，不并入纯视觉布局。

**验收**：

- [x] 视觉偏好（含侧栏折叠/布局）保存并重启恢复；滑块合并保存、失败保留草稿，背景引用原子提交。
- [x] Tab/Enter/Space/Escape、输入法、焦点归还正确；顶层弹层关闭不误关下层。
- [x] 浅色/深色和局部覆盖可追溯到批准基线，字体/Emoji/背景差异有明确验收。

**当前交付**：DONE；schema v8 显式迁移、类型化视觉偏好与受管图片原子提交；280 core / 17 desktop tests、真实 GPUI 浅深色/重启/失败重试/嵌套 Escape/键盘，以及用户中文候选与连续 drag 确认通过。Input 裁切、侧栏小 icon、底栏高度/对比已修复并真实复测；27 图及能力差异见 [P1-04A 交付](evidence/p1-04a/README.md)。仅作者自查，未 commit/push，后续 Host Review 决定提交。

<a id="obg-p1-04b"></a>
## OBG-P1-04B 跨页面行为偏好与消费契约

**类型**：偏好与 UI；**依赖**：OBG-P1-02、OBG-P1-03。**依据/范围**：方案 §6、§9.4；core preferences、面板行为字段及各页面读取契约。

**泳道 / 写范围**：Core/Config + GPUI；行为 Preferences/消费通知与面板行为 UI；与 A 预约共享设置组件/DTO。

**执行与交付**：迁移测速站点/阈值、IP 信息源、proxy columns、hide unavailable、代理显示/排序默认值及 UI diagnostics `config/ipv6-test`；定义类型化读取与变更通知，供后续代理、概览、连接和诊断消费。共享 Preferences DTO 由单一 owner 修改，A/B 不同时写同一设置组件。

**验收**：

- [x] 行为偏好可保存/恢复，非法输入可定位；消费者收到新值而不丢失未编辑字段。
- [x] UI latency preference test URL、Runtime health test URL、group health URL 分别建模；group 的空值按现有明确规则使用 Runtime 全局地址，不回落到 UI 测速偏好。
- [x] UI diagnostics ipv6-test 与 Runtime profile ipv6 分别保存/消费；修改诊断偏好不改变 Runtime IPv6 配置。
- [x] 面板行为字段的实际操作与失败反馈通过；后续页面消费的真实集成分别在 P2-08、P3-02/05/07、P4-05C、P5-03 验收，不在本任务冒充完成。

**当前交付**：DONE；schema9、typed partial patch/CAS/通知与 Panel 行为字段实际验收通过；[完整证据](evidence/p1-04b/README.md)。共享 owner 已释放，后续真实页面/Runtime 消费未实现。

<a id="obg-p1-05"></a>
## OBG-P1-05 平台目录、单实例与原生文件操作

**类型**：桌面适配；**依赖**：OBG-P1-02、OBG-P1-03。**依据/范围**：方案 §9.3、§12–13；core platform/macos、desktop 原生动作。

**泳道 / 写范围**：Runtime/Platform + GPUI；platform/macos 目录/锁/文件桥及原生动作。

**执行与交付**：用户数据/日志路径与受管文件权限、OS 文件锁；文件打开/保存/拖放、剪贴板和外部链接。新路线试运行使用独立目录。

**验收**：

- [x] 第二次启动聚焦已有窗口，不能出现第二个业务 writer；占用无法确认时明确报告。
- [x] 取消文件对话框无副作用；非法/超大输入在边界失败，含凭据文件权限符合方案。
- [x] 不扫描浏览器 profile，不覆盖旧应用目录；文件复制与外部链接真实可操作。

<a id="p1-05-delivery"></a>
### P1-05 平台交付（2026-10-04）

正式 desktop/platform 使用 Foundation preview 目录、0700/0600 权限、flock + 同 UID 固定 Activate IPC、异步 NSOpenPanel/NSSavePanel、可物化 item/type payload 的内存 pasteboard snapshot/restore（捕获失败不覆盖 pasteboard） 和仅 http/https 的 NSWorkspace handoff。Core 仅修正 portable atomic snapshot 权限，不引入 GPUI/AppKit，schema v9 未变。

真实五次 secondary/active-key、占用不明、SIGKILL/stale socket、native Cancel/Accept、合法/非法/超大图片、managed hash/source 不变、保存文件逐字读回、Cmd+V/恢复、loopback URL OS handoff 通过。正常退出的 socket 残留已由 GPUI on_app_quit 修复，锁 FD 保留到进程结束。首次交付 294 Core / 33 Desktop（新增 1 + 9，含 child harness）、规定构建/检查与脱敏截图通过；旧资源 clippy 原始失败及检查覆盖、TOCTOU 和自动化滚动限制明示保留。

Host Review 修订：Core 恢复 rename 成功即保存成功；preview stale temp 与复用 final cache 显式 0600；无 Veyra flock 的活 socket 不删除/接管。新增 3 Desktop 回归，修复前均复现失败，修复后 294 Core / 36 Desktop、规定 test/check/clippy/fmt/workspace check/diff 均通过；未重跑原生 GUI，历史身份/限制保留。

[完整证据](evidence/p1-05/README.md)、[验证](evidence/p1-05/validation.json)、[交互](evidence/p1-05/interaction-results.json)、[清理](evidence/p1-05/cleanup.json)。本轮不提交；无正式旧目录/browser profile 导入、无 sing-box/系统代理/TUN/公网验证；P1-06 托盘未开始。

<a id="obg-p1-06"></a>
## OBG-P1-06 托盘与窗口关闭流程

**类型**：原生 UI；**依赖**：OBG-P1-03、OBG-P1-05。**依据/范围**：方案 §9.5；desktop 装配/托盘/退出流程。

**泳道 / 写范围**：GPUI；托盘/窗口/退出装配；公共 Runtime 接口交 owner。

**执行与交付**：把 P0 托盘原型接入统一业务状态；实现显示、状态、启停入口及退出。此时内核尚未接通的动作显示准确可用性。

**验收**：

- [x] 关闭最后窗口后可从托盘恢复、聚焦；重复操作不新建额外应用实例。
- [x] 菜单状态来自 core，不维护另一套运行真相；无内核时不显示已启动。
- [x] 明确退出走统一清理入口，关闭窗口只隐藏；真实 macOS 操作和视觉验收通过。

**本轮交付：DONE（2026-10-04）**。三项验收全部 PASS；Human Visual / Tray interaction 已由当前 Host 会话用户真实操作补足。本次仅更新证据/状态文档，保留基线 f9adda4 上所有 P1-05/P1-06/Host Review 与 AGENTS.md 既有修改，不启动 P1-07/P2。

- AppView 长期持有主线程 TrayIcon；固定 Intent channel → GPUI foreground recv，无 40ms 轮询/第二 UI loop。关闭返回 false 并 hide；Show/Activate 指向已有窗口；窗口 Quit 与 tray Quit 共用唯一 `cx.quit()` 入口。
- 统一 Core snapshot / optional RuntimeSnapshot 的只读投影；P2 owner 未绑定时显示“内核：未接入（当前不可用）”，Start/Stop disabled。没有第二份运行 bool 或假的内核启停。
- 41 desktop（原有36+新增5）、294 core 与规定 check/build/clippy/fmt/workspace override/旧 Tauri lib clippy PASS。实际 close/native hidden=true、五次 secondary existing window active/key、writer0、筛选保留与窗口 Quit/socket/flock/tray cleanup PASS。
- 当前 User/Host manual acceptance：用户看到 macOS 菜单栏 Veyra “V” 图标并展开菜单；菜单显示“显示窗口”、“内核：未接入（当前不可用）”、灰色禁用的“启动（Runtime 未接入）”/“停止（Runtime 未接入）”及“退出”。连续三轮“关闭窗口 → 托盘‘显示窗口’恢复”均正常，最终托盘“退出”正常、无异常。Host 会话已查看用户上传的两张托盘截图，未复制入仓库，不伪造图片/SHA，见[人工证据](evidence/p1-06/host-manual-acceptance.json)。
- 历史 AX 无托盘入口、SystemUIServer 超时、Ctrl+F8 无变化及当时 NOT_RUN 保留；本次人工 PASS 不改写为自动通过。此前窗口 Quit 的程序化 socket/flock/process 清理与本次托盘 Quit 用户结果分开，后者没有新增程序化 post-check。
- 用户确认 Host 在本轮此前独立复核并复跑 41 Desktop / 294 Core tests、desktop/core check、clippy、fmt、resource override workspace locked check、旧 Tauri lib clippy、git diff --check，全部 exit 0；本次不重复运行大测试。
- [证据、检查与截图](evidence/p1-06/README.md)。68 卡重算：DONE12 / ACCEPTANCE1 / READY1 / TODO47 / Windows DEFERRED7；P1-07 七项依赖均 DONE，为唯一 READY，未领取/启动。P0-05 仍 ACCEPTANCE，P0-06/P0-08 仍 TODO，未 commit/push。

<a id="obg-p1-07"></a>
## OBG-P1-07 桌面壳阶段验收

**类型**：阶段验收；**依赖**：OBG-P1-01、OBG-P1-02、OBG-P1-03、OBG-P1-04A、OBG-P1-04B、OBG-P1-05、OBG-P1-06。

**泳道 / 写范围**：组合验收；所列依赖的证据/页面组合与本卡验收记录；修复先预约相关 owner 写范围。

**执行与交付**：先按 [UI 强制规范](../../AGENTS.md#ui-视觉迁移与组件复用强制规范)及[长期视觉/组件流程](DEVELOPMENT_WORKFLOW.md#31-长期-ui-视觉迁移与组件抽象流程)，对当前已实现的 App Shell + Settings（含 Sidebar Settings / Panel Settings）及基础组件做 React 视觉对照与组件复用审查；固定可复测构建，验证首次启动→修改偏好→隐藏/恢复→退出→重新打开，留存必要审查与实际操作记录。

**范围边界**：Proxies/Connections/Logs/Rules 业务页面尚未真正实现，不纳入 P1-07 业务完成验收；导航、占位或 fixture 不能冒充页面完成。其各自后续任务必须遵守全局视觉规范并独立留下页面/核心状态证据，不以 P1-07 壳层通过代替。

**当前状态**：DONE（2026-10-05），owner 已释放。最终 binary SHA `8bd7a366d24fa2eff8bf78e22604f461030cedae9d1184c11a28ca8eb4f5c28d`；视觉 PASS_WITH_TECHNICAL_DIFFERENCES，组合验收 PASS。实现起始基线 `b982e8d`；本轮只收口文档/evidence。[最终证据](evidence/p1-07/README.md)。

**验收**：

- [x] core 独立验证、桌面构建、单实例及偏好恢复全部通过，命令/机器/构建身份有记录。
- [x] 当前 App Shell + Settings/基础组件与 `src/openbox/**`、`openbox.css` 在相同窗口尺寸、缩放、主题、页面状态和尽可能相同数据下逐项截图对照，真实交互及适用加载/空/错误/忙碌、hover/focus/active/disabled/loading/selected 状态留证；禁止用不同状态图或全局平均分代替页面/核心状态独立验收。
- [x] 固定几何、颜色、字体、图标尺寸/stroke/几何及其生效逻辑精确迁移；95% 为最低视觉目标，不以单一全图像素分数作为唯一 Gate。明显尺寸/颜色/图标/布局差异全部修复；仅确实不可等价的 GPUI/macOS 差异记录技术原因和对应视觉证据，OS 自绘系统区域与应用自绘区域明确分界。
- [x] 组件体系审查确认 `Design Tokens/Theme → 基础组件 → 跨页面组合组件 → 页面`、重复值集中及来源可追溯；React 复用模式或 GPUI 两处及以上真实重复交互使用共享组件，同类按钮/设置行/卡片/工具栏/状态标签不各页重写；GPUI Kit 外观已覆写为 React 基线，无近似图标替代，字体/资源来源许可和技术差异有记录。
- [x] 未真正实现的 Proxies/Connections/Logs/Rules 未计业务完成；对应页面视觉对齐及基线/证据留存前，Legacy React 视觉参考源码/资源保留，不能以功能已迁移提前退役。
- [x] 无本阶段未解决 blocker，历史失败保留且复测无稳定回归证据，任务总表与 SESSION 更新；P2 各任务按自身依赖就绪，不等待本组合验收统一放行。

<a id="p1-01-delivery"></a>
## OBG-P1-01 交付记录（2026-10-03）

**负责人 / 泳道**：Codex /root / Core/Config，workspace、锁文件与 core/现有公共契约单一 owner。**状态**：DONE，三项卡验收 PASS。源码 HEAD `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`，分支 `codex/dist-react-restore`；本次源码/证据未提交。保留开始前 `.gitignore`、AGENTS、长期规范和全部 P0-01 证据/状态，无 commit/push/publish。环境 macOS 27.0 (26A428)、arm64，Rust/Cargo 1.99.0；GPUI 与真实内核运行 N/A（纯本地 Rust 抽取）。

### 迁移与边界

- 根 `Cargo.toml`：成员 `crates/veyra-core`、`src-tauri`，resolver 3，default-members 仅 core。原 `src-tauri/Cargo.lock` 移为根锁文件；新增唯一的本地 core 包，已有第三方包版本全部保持。输出统一为根 `target/`。
- core：完整 domain/subscription/storage；singbox 的 compiler/clash_api/runtime，以及从 managed_sidecar 抽出的 secret（ApiSecret、既有错误与生成/提取）；application 的 observability/provider_replacement/runtime/selected_subscription/state_access/subscription_management/subscription_scheduler/system_proxy。保持当前 singbox 目录命名，没有顺带重命名整个 Runtime/Compiler。
- 有效纯测试随代码迁移；12 个 state/subscription fixtures 移至 core，逐个对照 HEAD 字节一致。两个 secret 测试迁入 core；三个具体 Windows 代理与 RuntimeSupervisor 组合 Mock 回归/夹具留旧入口 `runtime_adapter_tests.rs`，没有删除或弱化断言。
- 旧 lib/application/singbox 重导出真实共享模块，commands 消费同一 core 类型/服务。IPC DTO、命令名、事件、窗口行为保留。已接管旧文件与 fixture 已移走，没有复制两套实现。
- **未迁**：旧 managed_observation_runtime/proxy_routing 编排及其测试；Windows managed sidecar/private runtime/recovery/WinINet、managed_sidecar 的进程/资产/DNS 测试；Tauri commands/lib/main/tray/event、React 和原打包资源配置。继续消费 core；不一次迁移整仓、不实现 P1-02 新领域契约或 P0/GPUI 工作。

### 真实耦合处理

1. SubscriptionScheduler 改为显式 Tokio Handle::spawn 与 tokio::task::JoinHandle。旧入口借用 Tauri runtime 的底层 Tokio Handle；core 不创建全局 runtime。测试自己持有 Tokio runtime，写屏障/取消/shutdown 语义保留。
2. managed_observation_runtime 生产路径改为注入 Handle 的 block_on；worker 对既有 SidecarPort/RuntimeObservationSidecarPort 参数化。三处 Windows 构造改为入口传入的延迟构造闭包，资源校验仍在 worker 首次使用时发生；没有 Executor、平台工厂类/注册表/平台选择服务。旧具体 Windows Port 只留装配/旧测试。
3. SystemProxyController、ProxyState、既有结果/错误移至 core application/system_proxy；RuntimeSupervisor 不再反向引用 Windows，旧 adapter 实现共享 trait。SidecarPort、opaque ManagedSidecar identity、观测接口/错误在 core singbox/runtime；具体 child/路径/Win32 仍留旧平台层。
4. 额外实际耦合：SubscriptionManager 的 System-mode Windows 代理读取改为入口注入函数。每次仍读当时系统代理，错误原样传播，无缓存/fallback。新增纯单测保护注入/不可用传播，证明失败前不会发 HTTP 请求。
5. Windows managed_sidecar/private_runtime 用 cfg(windows) 保留，WinINet 原 cfg 不变。macOS 旧入口无打包 observation sidecar，非 Windows 接线只返回既有 SidecarPortError，不伪装可运行内核。core 无具体 Windows/UI 引用，也无 cfg(windows) 业务分支；条件边界均在旧平台/入口。

跨 crate 后旧 Windows 测试需要原 cfg(test) Compiler fixture/流探针；core `legacy-test-support` **仅由旧 crate dev-dependency 启用**，复用原测试白名单、GeneratedConfig 夹具与探针，不是产品 feature 或新能力开关。普通生产 feature tree 未启用它，原生产配置白名单保持。旧入口无消费者的 serde_yaml_ng/futures-util/tokio-tungstenite 依赖移除。

### 实际验证与重跑

长命令均用 Python subprocess.run 的外层 timeout=600；旧 build/check/clippy 为 900 秒。以下均从根目录执行。旧入口命令必须前置 `TAURI_CONFIG='{"bundle":{"resources":[]}}'`，仅排除当前缺失的 Windows bundle 资源；原 tauri.conf.json 不变。这不是打包/真实运行验收。

| 实际命令/检查 | 结果/保护契约 |
| --- | --- |
| `cargo check -p veyra-core`；`cargo build -p veyra-core --lib` | PASS；macOS 独立核心构建，无 UI/内核资源 |
| `cargo tree --locked --offline -p veyra-core --target aarch64-apple-darwin -e normal,build,dev` | PASS，无 Tauri/GPUI；另查根锁文件全目标传递闭包，159 包中无 UI 包 |
| `rg -n -e platform::windows -e WindowsManagedSidecarPort -e WindowsSystemProxy -e tauri:: -e gpui:: -e windows:: crates/veyra-core/src` | 无匹配（rg exit 1 表示零匹配），core 无具体 Windows/UI 实现依赖 |
| `cargo test -p veyra-core --lib -- --test-threads=1` | **243 passed / 0 failed / 0 ignored / 0 filtered**；其中 **178 项明确纯单测**，其余为纯逻辑与本机 loopback fixture 混合集合；不启动真实 child/不改系统代理 |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | PASS；旧 main/lib/测试目标接线编译 |
| `cargo clippy -p veyra-core --lib -- -D warnings` | PASS；core 生产库 lint |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | PASS；旧入口生产库 lint |
| `cargo fmt --all -- --check`；`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | PASS；workspace/旧入口格式 |
| `git diff --check`；新增源码/文档 whitespace、引用/Task/DAG/状态检查 | PASS；无无意修改，状态/证据一致 |

旧入口定向命令：`cargo test --manifest-path src-tauri/Cargo.toml --lib FILTER -- --test-threads=1`（保留上述环境覆盖）。总计 **79 passed**，各组均 0 failed / 0 ignored：

| FILTER / 附加参数 | passed | 保护行为 |
| --- | --- | --- |
| `application::managed_observation_runtime::tests::`，追加 `--skip real_worker_` | 34 | 配置/订阅切换、取消/忙碌、保存/应用分离、shutdown、Mock child 归属；明确不执行四个真实 Windows worker 用例 |
| `application::proxy_routing::tests::` | 16 | 旧 routing 保存/CAS/选择与应用结果映射 |
| `application::runtime_adapter_tests::` | 3 | 共享 Runtime 与旧具体代理 Mock 组合恢复语义 |
| `commands::tests::` | 11 | 旧 IPC DTO/错误/请求与命令权限 |
| `platform::windows::system_proxy::tests::` | 14 | 原代理算法和共享 trait；内存 Mock，不调用 WinINet |
| `tests::hidden_window_drops_deltas_and_visible_window_resumes_without_backlog`，追加 `--exact` | 1 | 隐藏/恢复事件桥消费共享 Observation |

带超时的实际重跑形式（其他命令替换参数列表，旧入口保留 env）：

```sh
python3 - <<'PY'
import subprocess
subprocess.run(['cargo', 'test', '-p', 'veyra-core', '--lib', '--', '--test-threads=1'], timeout=600, check=True)
PY
TAURI_CONFIG='{"bundle":{"resources":[]}}' python3 - <<'PY'
import subprocess
subprocess.run(['cargo', 'check', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets'], timeout=900, check=True)
PY
```

[验证摘要](evidence/p1-01/validation.json)、[全目标锁闭包](evidence/p1-01/lock-closure.json)、[core 测试](evidence/p1-01/core-tests.log)、[依赖树](evidence/p1-01/core-tree-macos.log)、[生产 feature 树](evidence/p1-01/legacy-features.log)及全部最终命令日志见 evidence/p1-01/。

### 尝试历史、自查与限制

- 初次 core 测试编译有搬迁后的测试 import/字段初始化遗漏，已修复；最初旧检查缺 Windows 资源为 FAIL，使用检查专用覆盖后 PASS；后续跨 crate 探针/Handle 参数遗漏均修复。未伪造资源或改打包配置。
- `cargo tree -p veyra-core --target all -e normal,build,dev` 尝试因不可写 Cargo cache 中缺失 wasm crate 失败，不计 PASS；macOS tree + 根锁文件全目标闭包为等价依赖证明，未把空输出当成功。
- 额外 `cargo clippy -p veyra-core --all-targets -- -D warnings` **FAIL**：5 项继承测试 lint（Observation 借用 1、第三方 WS 回调固定大 Err 类型 3、WireGuard fixture 八参数 1）。表达式/签名来自 HEAD 旧测试；未删除测试、加全局 allow 或弱化断言。项目规定的两条生产 --lib clippy 已 PASS，此额外测试 lint 不改变本卡纯单测/生产库验收。[失败日志](evidence/p1-01/core-clippy.log)保留。
- 采用 code-delivery-review 做**实现者自查**：公共类型/secret/配置白名单、Tokio runtime 生命周期、worker 延迟资源构造/shutdown、旧 commands 接线、Windows cfg/有效测试、单锁文件/依赖版本与剩余引用。未发现未处理的本卡行为缺陷；不声称独立审查。
- Windows 真实运行/打包、macOS 可用 sidecar、GPUI/browser/截图/视觉均 NOT_RUN 或 N/A（不属本卡），不以 Mock/编译冒充。原 P0-01 历史含义不变。
- DAG 重算：P1-02 READY；P0-03/P0-04 仍 READY；未开始下游。当前公共 owner 释放，下一领取任务登记新 owner。

<a id="p1-02-delivery"></a>
## OBG-P1-02 交付记录（2026-10-03）

**负责人 / 泳道**：Codex /root / Core/Config，domain/application/storage 共享 DTO、版本和业务快照单一 owner；完成后释放。**状态**：DONE，四项卡验收 PASS。分支 `codex/dist-react-restore`，HEAD `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`；所有交付未提交，保留 P0/P1-01 dirty 基线。macOS arm64，沿用 P1-01 workspace/单一锁文件，没有新依赖；禁止的 `.gitignore`、AGENTS、长期规范字节保持不变。纯本地 Rust，本轮没有访问 openbox.disign.me、启动浏览器/视觉对齐、真实 sing-box、系统代理/helper 或 commit/push/publish。

### 类型、schema 与服务

- `domain/version.rs`：StateEpoch（OS 随机 128-bit 身份）、StateVersion（epoch + revision）、ConfigVersion、SelectionVersion、SnapshotVersion。版本不实现跨 epoch 数字排序；compare/require 对 epoch 或 revision 失配返回稳定 RevisionConflict。计数保持既有 MAX_SAFE_INTEGER 范围，耗尽拒绝写入；`try_empty` 将随机源失败传播为错误，保留旧 empty/Default 构造兼容。
- `AppState` / `StoredStateV7`：在原单一业务快照增加 state_epoch/config_revision/selection_revision、Profile 与 AppConfig；原 SubscriptionId/ProviderId/NodeId/PoolId/RoutePolicyId、引用和旧 generation 保留。pool 中手动选择是本卡单一选择事实，没有再复制 selection-state 文件或第二份可编辑订阅/路由真相。
- `domain/profile.rs`：只建立本卡代表字段。IPv6；DNS split/direct/proxy/port/extras；TUN autoRedirect/stack/mtu/tcpMss；routing custom name/enabled/domain/domainSuffix rules，目标继续使用已有 RouteTarget/PoolId。Profile custom 部分是后续 OpenBox 字段的基础，旧 routes 仍是旧 routing 事实；本卡不在两者间做导入同步，不实现完整 P4/P5 或新 Compiler 映射。
- ProfilePatch/Patch<T>：Missing 保留，Null 仅清空 routing.custom/name；必填对象、标量、数组的 Null 返回固定字段位置。嵌套对象只合并批准字段，DNS extras/custom rules 数组整体替换（空数组清空）。未知字段拒绝；端口/TUN 参数、非空值与已有 Pool 引用在保存前校验，内部没有任意 JSON merge/command。
- `AppConfig.check_updates_on_start` 默认 true，v6 migration 明确填入，仅持久化桌面版本检查偏好。登录项、注册状态、updater 运行进度及自动安装不进入业务快照/本卡实现。
- `application/state_service.rs`：ProfileService.patch、SelectionService.select_manual、SnapshotService.snapshot/append/replace，共享既有 StateAccessGate 与 JsonStateStore。写时加载最新完整快照、检查适用预期版本，构造候选并在持久化成功后返回版本/事实；没有内存先推进。StateStore.save 仍是已验证事务的物理写入原语，业务修改使用 commit 或明确 replace，未来消费者须共享同一 gate。

### 版本推进与原子性

| 操作 | epoch / 版本规则 | 保存/应用事实 |
| --- | --- | --- |
| 首次空快照 / v6 首次升级 | 新建一次 epoch，config=0、selection=0；稳定重载不重新生成 | SavedOnly |
| 普通 profile/配置/订阅事实变化、append | 当前 epoch；config +1，保留 selection；实际无变化不推进/不重复写 | SavedOnly |
| pool 手动节点选择 | 当前 epoch；selection +1，保留 config；相同选择 no-op | SavedOnly，不声称控制器已确认 |
| 结构变更同时移除/改变已有选择 | 两个实际受影响计数分别推进 | SavedOnly |
| 完整 replace/恢复/整份 backup 回退 | 新 epoch；丢弃导入 token，config=0、selection=0；旧表单/task 失效 | SavedOnly |
| v7 稳定读取 / 普通 schema 升级 | 已有 epoch 不无意义更换；不因读操作推进计数 | 不制造应用结果 |

配置 patch 只检查 ConfigVersion，因此并发选择不使其失效；选择只检查 SelectionVersion，但在最新配置上验证成员，所以不覆盖配置变更。完整 replace 必须同时匹配配置/选择版本。append 仅追加已经规范化 subscription/provider/node 批次，重复 ID/非法引用拒绝整批；完整 OpenBox/备份未知字段报告与冲突 ID 重映射仍由 P7-01 负责。

物理写入继续使用已有 temp/write/fsync/rename/backup 流程。模拟当前临时文件写入失败和备份 rename 失败，分别验证原 bytes、epoch、两个版本与全部状态保留；replace/append/selection 的失败也不返回新状态。整体 backup 恢复先产生候选新 epoch，成功原子写入才发布；旧备份不被当作可复用并发凭证。

### 错误、Runtime 与旧入口

- AppError/AppErrorCode/FieldPath/ErrorDetail：稳定 code（Validation、RevisionConflict、UnavailableOnPlatform、PermissionDenied、PortInUse、KernelUnavailable、Timeout、Cancelled、DownloadFailed、StorageFailed、ApplyFailed）、固定中文用户 message、可选批准字段路径、可选诊断类别。detail 不接受配置、secret、任意路径或原始响应；Debug/Display/JSON 经 secret fixture 与存储路径反向断言。旧 StateStoreError 映射保留 RevisionConflict 语义。
- ApplyEffect 明确定义 SavedOnly / Applied / RestartRequired。本卡没有 Runtime 应用证据，所有服务返回 SavedOnly；不根据是否历史 Ready 推测 RestartRequired，不返回 Applied。后续实际 Runtime owner 才能报告两者。
- RuntimeFacts/RuntimeState、InstanceId、AppliedInstance、LastSuccessfulVersion、RuntimeSnapshot：当前实例/status、业务 saved、当前 applied、历史 last_successful（含应用时选择版本）分别表达。Stopped 永远没有 instance/applied；Ready 必须由 owner 提供当前 ID/版本；Recovering 的 AppliedInstance 成对绑定 ID/版本；新业务 epoch 可与仍需清理的旧实例版本并存。
- 本卡不创建 `runtime/last-applied.json`，不伪造 applied/last-success 持久化；LastSuccessfulVersion 是可序列化的后续 owner 读写边界。测试覆盖 stopped+history、ready+旧 epoch、恢复/启动/失败、manifest 失败时 live 与 history 分离、selection 不制造配置待应用。
- v1→…→v6→v7 使用明确 migration；保留 pre-migration 和旧节点凭据/稳定 ID/引用，不把旧 v6 冒充 OpenBox Profile。v7 缺必需版本字段视为无效快照，沿用既有恢复策略，不默默生成默认并发 token。
- 旧 SubscriptionManager/ProviderReplacement/Runtime activation/routing 写入接同一 commit；返回的新版本仅在成功保存后用于内存/消费者。订阅 UpdateSnapshot 带 ConfigVersion；旧 routing 兼容 numeric CAS 的 material 包含 epoch，保留既有 IPC DTO/错误 envelope；同进程整体替换使旧 request Conflict。旧 missing-state routing 读取先持久化初始 epoch，避免每次空读取生成新 token。旧 IPC 不暴露 AppState 原始模型，因此升级字段不破坏原请求/响应 shape。

### 实际验证与验收

所有 Cargo 长命令使用既有 Python subprocess.run 外层超时：core/fmt/tree 600 秒，旧入口 900 秒。旧入口保留 `TAURI_CONFIG='{"bundle":{"resources":[]}}'` 检查专用覆盖，tauri.conf.json 未改；这不代表资源打包或真实设备运行通过。完整命令/日志见[命令记录](evidence/p1-02/commands.json)。

| 命令/检查 | 最终实际结果 |
| --- | --- |
| `cargo check -p veyra-core`；`cargo build -p veyra-core --lib` | PASS |
| `cargo test -p veyra-core --lib -- --test-threads=1` | **275 passed / 0 failed / 0 ignored / 0 filtered**；新增 32 项，本机文件事务/纯逻辑；继承集合还含 loopback HTTP/WS fixture，无真实 child/外站 |
| `cargo clippy -p veyra-core --lib -- -D warnings` | PASS |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | PASS |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | PASS |
| 旧 lib 定向测试：routing / observation（`--skip real_worker_`）/ runtime_adapter / commands | **17 / 34 / 3 / 11 passed，共 65**；均 0 failed / 0 ignored，非零过滤数明列在各日志；Mock/local，未执行真实 worker |
| `cargo fmt --all -- --check` | PASS |
| `cargo tree --locked --offline -p veyra-core/veyra --target aarch64-apple-darwin -e normal,build,features` | PASS；两棵生产树均未启用 legacy-test-support；core 无 Tauri/GPUI |
| core 源码具体 Windows/UI 反依赖检查；受保护文件 SHA-256；HEAD/工作树核验 | PASS；原依赖/锁文件无新增，本轮未修改受保护文件 |
| `git diff --check`；任务引用/身份/计数/DAG 与新增文件 whitespace | PASS，见最终验证摘要 |

| 用户验收 | 对应实际保护契约 | 结果 |
| --- | --- | --- |
| A | Missing/Null/Value、局部 DNS/routing/TUN 保留、数组整替、字段定位/引用校验 | PASS |
| B | profile/replace/append/selection 保存失败旧完整 snapshot 保留、备份替换失败 | PASS |
| C | 跨 epoch 及 stale revision 拒绝；旧 routing form/订阅任务迟到写回被拒绝 | PASS |
| D / E | append 同 epoch config +1；手动 selection 单独 +1；replace 新 epoch 0/0 | PASS |
| F | saved/current applied/history 分离；Stopped/Starting/Recovery/Failed 不沿用历史 Ready | PASS |
| G | 11 稳定码、固定字段/detail、Debug/Display/JSON 无 secret fixture/存储路径 | PASS |
| H | 原 243 core 继续执行；旧入口 check 与 64 原定向回归 +1 新 epoch 回归 | PASS |

**必要审查**：按 code-delivery-review Skill 做实现者自查，未声称独立审查；关注原子事务/旧 CAS、schema 与稳定引用、选择归属、AppError 输出、Runtime owner 边界和生产 feature。没有本卡未解决缺陷。GPUI/UI/浏览器/视觉、真实 Runtime/manifest、helper/系统网络/Windows 设备验收均 N/A 或 NOT_RUN（当前用户明确纯本地 Rust，后续按卡执行）。

**尝试历史保留**：初次测试编译遗漏新测试模块/测试构造字段及借用问题，修复；首轮 core **263 PASS / 8 FAIL**，为旧 schema 字段清单/迁移 epoch/backup epoch 的旧预期；第二轮 **274 PASS / 1 FAIL**，为字段数量断言仍为 9，改为 schema v7 的明确 14 字段；最终 275 PASS。旧 observation 初次 **33 PASS / 1 FAIL**（断言候选未带提交后 revision），更新为 revised_from 的精确预期后 34 PASS。新增旧 routing 测试曾误用旧 application namespace 导致 check FAIL，改用实际 core namespace 后 check/17 routing PASS。失败日志/记录保留，不将旧 FAIL 改写为当时 PASS。

**后续边界**：Profile 是批准代表字段基础，完整 P4/P5 能力、OpenBox importer/ID remap、Runtime manifest 和 controller 确认不在本卡；后续写入使用同一 gate 与版本服务，save 只是物理原语。新 v7 数据不承诺旧二进制直读，回退须使用保留的旧 schema 快照。本卡没有待修复阻塞。

**DAG / 下一动作**：P1-02 DONE 后重新计算 68 卡：macOS 4/61 DONE、2 READY、55 TODO；Windows 7 DEFERRED。READY 仅 P0-03/P0-04，均未启动；P1-03 仍等待 P0-03，保持 TODO。共享 owner 已释放；不启动后续 Task，不 commit/push。

## OBG-P1-07 Host Review 修订记录

三项指定blocker逐项复核：Shell错误主题evidence重新采集；默认背景CPU派生移出UI Render，3条回归；相同实际Mach-O bundle下b982e8d/current真人Tray A/B可见，三轮Close→Tray Show及显式Tray Quit/重开值恢复/再次Tray Quit通过。45 Desktop/294 Core与完整自动检查PASS，组件焦点/Tooltip定位最后补验身份分开记录。历史FAIL/INCOMPLETE保留。整卡视觉仍有字体/局部blur技术差异及细节未完成，保持**DOING、owner GPUI**，其他Task状态不变；不commit/push、不启动P2。见[修订证据](evidence/p1-07/README.md)。

## OBG-P1-07 2026-10-05 最终视觉收口续录

保持 **DOING、owner GPUI**。在627728a2同一构建重新采集Shell/Panel/基础组件Light/Dark状态，补sidebar Tooltip箭头、UA spinner/focus颜色/品牌局部级联，45 Desktop/294 Core及完整检查通过；真人三轮Close→Tray Show与Tray Quit、重启亮色/6600ms/IPv6on值恢复已执行。随后补查现有IconPicker展开态发现tab均分与局部input focus差异，做共享Input局部focus覆写及picker直接flex child/space-around的最小修正。新构建8bd7a366完整11项自动验证PASS，但尚未采集新构建GUI；627728a2所有图/操作已归档为中途证据，不计最后源码PASS。当前等待用户完成627728a2重启窗口已请求的Tray Quit，再换入新build重新截图和组合，不能建议DONE。此前FAIL/INCOMPLETE/PENDING保留，字体/局部backdrop blur继续明确技术差异。其他Task状态不变，无commit/push/P2/业务/公网/内核运行。[最新证据状态](evidence/p1-07/README.md)。

2026-10-05 最后构建8bd7a366视觉复核续录：Light/Dark显式1280×720 Shell展开/折叠、Panel及当前组件状态已重采；Tooltip箭头/定位、spinner、IconPicker分类均分/局部focus/trigger已对照React级联和叠图复核，技术差异单列字体fallback/element backdrop blur/UA栅格。最后源码完整11项检查PASS（45 Desktop/294 Core），无新源码改动。实际bundle保存Light/6600ms/IPv6on，用户回报完成三轮与退出；日志仅Show2/Close3，明确不计3次日志PASS。Tray Quit后进程/socket清理、flock可重新获得，重启实际Settings值恢复PASS；已重做视觉/行为保存并隐藏，等待真人明确3Show→Tray Quit补齐。P1-07保持DOING、owner GPUI，其他Task/DAG不变；无commit/push/P2/公网/内核。见[evidence](evidence/p1-07/README.md)。

<a id="p1-07-final-closeout"></a>
## OBG-P1-07 最终 Host 验收收口（2026-10-05）

**状态：DONE，六项验收全部勾选，owner 已释放。** 最终 `8bd7a366` 的截图、AX、comparison 均在 `evidence/p1-07/final-20261005/`；Host 已查看 Shell、Panel、Select、Tooltip、number spinner、Dark Panel、Modal 代表图。视觉 **PASS_WITH_TECHNICAL_DIFFERENCES**，没有新的可修视觉 blocker；MiSans/NotoEmoji fallback、element-level Card/Modal backdrop blur、少量 UA 栅格差异继续记录技术原因/影响/证据，不冒称完全一致。组件层级保持 Tokens/Theme → 基础组件 → NavigationItem/CompactSetting/Section → 页面。

三项 Host blocker 的最终证据齐备：显式 Light/Dark 同 1280×720、最终 build 截图；默认背景 blocking task + key/generation + ready cache 移出 Render，相关测试保留；Tray baseline/current shell wrapper 均不可见，真实 Mach-O bundle 均真人可见，旧 68×0 不计本卡回归，Tray 架构未重写。

[最终三轮人工操作](evidence/p1-07/final-20261005/human-host-final-three-rounds.json) 为 Show #1 → Close → Show #2 → Close → Show #3 → Close → Tray Quit，Host 独立 process/socket/flock 清理 PASS。历史 first-launch log 只有 Show2/Close3，原样保留，不能写为 3Show。

[最终重启与 Tray Quit](evidence/p1-07/final-20261005/human-host-final-restart-quit.json)：同 binary、真实 Mach-O bundle、同 root 重启 PID94362，snapshot_loaded=true，Settings → Panel Settings 为 Light/6600ms/IPv6on，primary_count=1、writer_created_count=1、secondary_writer_count=0，state bytes 前后 hash 不变。用户随后回复“最终退出完成”，通过托盘 Tray Quit；Host 独立确认 PID94362 gone、socket absent、Python flock free，无自有验收进程残留。瞬态 pgrep PID96193 后续 ps absent，不计残留。原重启等待人工退出的 status/PENDING 字段保留，新增 final_quit/closeout 收口 PASS。

[Host 自动复核](evidence/p1-07/final-20261005/host-final-review.json) 原样记录首跑 Desktop **44 PASS / 1 FAIL**：`platform::single_instance::tests::live_socket_without_veyra_lock_is_busy_and_remains_connectable`，断言 `locked_file(&d).unwrap().is_some()`；随后定向连续 **5/5 PASS**、单线程 **45/45 PASS**、默认并行 **45/45 PASS**。结论仅为“未复现的瞬态测试环境/flock 时序失败，目前无稳定回归证据”，根因未证明。Core **294/294** 及 Host 指定 check/build/clippy/fmt/workspace override/旧 Tauri lib clippy/diff 全部 PASS；Codex 原日志不篡改，override 不代表完整打包验收。本轮没有重跑产品测试/构建/GUI。

**边界与 DAG**：Proxies/Connections/Logs/Rules 业务页面仍未实现，不计 P1-07 业务完成、不改各自任务；Legacy React/CSS/视觉资源保留。完整 [68 卡显式 DAG](evidence/p1-07/final-20261005/dag.json) 为 DONE13 / ACCEPTANCE1 / READY0 / DOING0 / TODO47 / Windows DEFERRED7，无环、READY 为空；P2-09/P7-04 仍缺其他依赖。P0-05 保持 ACCEPTANCE，P0-06/P0-08 仍 TODO，Windows 不变。本轮源码/Cargo/assets/测试字节不变，不改 AGENTS.md/DEVELOPMENT_WORKFLOW.md、不清理 Host 复核文件、不 commit/push、不启动 P2/公网/sing-box/System Proxy/TUN。
