# P2-08 Desktop 接续与实机检查点

2026-10-10，**DOING；五项原验收尚未全部满足，不建议 DONE**。本轮仅写 `.worktrees/p2-08-proxy-latency`，分支 `dev/p2-08-proxy-latency`、HEAD `6857b2e39633ca0564cac66b40ee0f9129cf275a`，未 stage/commit/merge/push。Core Execution `execution-23397-1791604296826942-271` 已 COMPLETED 后接管；原业务服务保留，P2-06 owner 不变。

本记录追加 Desktop/Native 证据，不改写[Core 阶段检查点](P2-08-backend-checkpoint.md)的历史 NOT_RUN。原五个 checkbox 不勾选。总表仍 DONE31 / READY4 / DOING2 / TODO24 / DEFERRED7，READY 的 P3-05/P4-01/P4-04/P5-01 未启动。

## 当前候选与运行交接

- 当前已实测旧候选：`target/p208-desktop/VeyraP208Acceptance.app`，signed Mach-O SHA256 `4fdecfd322fd401b8838d08892a1065dea2545409d1fa4885651261531861472`，PID **83390**；自有主内核 PID **83493**，mixed/controller **60746/60747**。正常托盘 Quit 等待用户回复，未冒充退出完成。
- 最新源码候选：`target/p208-desktop-next/VeyraP208Acceptance.app`，signed Mach-O SHA256 `9b0a8112786d348e984f01c89e807fc70ffb99a7bc660e216d8866efbc604570`。构建与 `codesign --verify --deep --strict` exit0；**尚未启动，GUI NOT_RUN**。没有覆盖正在运行的旧 App，也没有把旧截图绑定新 SHA。
- 自有数据 root `/private/tmp/veyra-p208-1010` 保留用于后续重启恢复；草稿 root 已清理。仅普通用户 child、loopback；未安装/运行管理员 helper。
- 真实内核固定 sing-box 1.14.0，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。当前 Apple M4 Pro、macOS 27.0.1，固定内核/候选 minOS 26；不宣称 macOS15 已验收。
- 保留当前默认显示配置 1512×982、Retina scale 2、浅色；GPUI 内容 1280×720，原生标题栏单列。React 局部参考 viewport 1280×720、DPR2；临时浏览器覆盖已恢复，不修改系统显示配置。

## 实现与修复

唯一出口目录继续使用 OutboundQueryService；NodeId/PoolId 选择只经原 RuntimeService::select_manual，卡片 selected 来自 owner 确认快照。测速只写 SQLite 延迟历史，取消/实例替换/请求代次失效结果不提交，UI preference URL 与 Runtime/group health URL 分离。

GPUI 增加最小策略/节点页、目录与未应用/未知反馈、名称及协议筛选、独立选择/测速兄弟点击区、单节点/已确认手动组/全部节点进度、部分失败和历史 Tooltip；直接消费已有偏好、阈值与布局选项。三语言接入统一 i18n；不猜测 UDP 或自动组事实。未抢跑 P3-07 完整订阅/穿透/规则视图。

修复真实问题：保留最近正值成功历史；macOS zh-CN 名称排序；草稿 worker 持有 session 至 cleanup、关闭/离页/Quit 取消并等待清理；陈旧 UI 代次仍报告 cleanup 错误；旧快照及仅 uptime 投影不抢读 Store gate，解决代理页首次 Start 的 StateUnavailable 时序；协议/别名筛选；卡片/搜索/工具 surface 按 React/CSS 白75%及暗94%、工具 gap4；显式取消反馈。未改变公共 Runtime/Helper/IPC DTO、Cargo.lock 或 P2-06 owner 文件。

## 真实结果与界限

| 项目 | 实际结果 / 证据 |
| --- | --- |
| 真目录与未应用 | PASS，真实 Store 的 a/b/c、所有-手动；`gpui-proxies-unapplied.png`。 |
| 未启动选择失败 | PASS，NotReady 不冒充成功；`gpui-selection-not-ready.png`。 |
| Ready 与正式 NodeId 选择 | PASS，固定内核 Controller 200 确认 node-a；`gui-selection-a.json`。修复候选 PID83390 request2 一次 Started，`gui-frozen-start.json`；历史首次失败保留。 |
| 选择保存失败 | PASS，精确自有备份路径冲突时显示“选择保存失败，尚未切换”，状态 bytes 不变、Controller 仍 node-a；备份已恢复，`gui-selection-savefail-before/after.json` 与截图。 |
| 测速不误触选择 | PASS，点击 b 测速失败仍选 a；组测 a854ms/b503/cTimeout，实际进度3/3、失败2；截图 `gpui-test-b-failed-preserves-a.png`、`gpui-group-busy/complete.png`。 |
| 在途测速后再选择 | 实际 c 测速中选择 a 已确认，`gpui-test-select-interleaved.png`；随后离页取消，不能声称该次迟到完成已实测；Core 定向测试覆盖。 |
| 空/忙碌/失败 | PASS，真实筛选空态、组进度、草稿 busy、503/Timeout；Zero/Unknown 的 typed 分支仅自动化/源码证据，实际 GPUI 注入 NOT_RUN。 |
| SQLite 重启恢复 | PASS，正常托盘退出后同一隔离目录重开，a854ms/b失败/c超时历史恢复，`gpui-visual-restart-history.png`、`gui-restored-history.json`。 |
| 语言 | 实际简中/English/繁中页面显示 PASS，业务名称不翻译；`gpui-proxies-english.png`、`gpui-proxies-zh-tw.png`。最新语言重启与切换时草稿保持仍待本候选复验。 |
| 草稿成功/失败 | 实际独立 child HTTPS 成功717ms、cleanup=true，`gpui-draft-fixed-fixture-result.png`；旧 relay 递归失败及错误截图保留。失败不假成功。 |
| 草稿弹窗在途关闭 | PASS，PID83390 的 draft-c 在 busy 后关闭，日志 `cancelled=true cleanup=true`，自有 child/root 清理、主83493存活；`gpui-draft-close-inflight.png`、`gui-frozen-draft-close-completed.json`。 |
| 草稿在途 GPUI Quit | **NOT_RUN**。截图和退出前进程证明当时 child85367在途，但等待用户托盘操作期间实际30000ms超时，最后 cleanup=true/cancelled=false；不能算 Quit 取消证据。`gui-inflight-quit-deadline.json`。正式 AppServices worker Quit 的 Native 测试已 PASS，不能替代这个 GUI hook 的实测。 |
| 隐藏/托盘恢复/退出 | 较早候选 PID75548 同 PID 恢复、用户托盘 Quit 后主76538退出与端口清理 PASS；第二候选 PID80794 用户 Quit 后主81425退出 PASS；`gui-first-lifecycle.json`、`gui-visual-lifecycle.json`。最新 PID83390 仍等待正常 Quit，不重绑较早身份。 |
| P4-03 / P3 回归 | 原10个 Group selection 测试 PASS；实际日志页、出站页和主备编辑器显示/取消未保存 smoke 有截图，不声称完整主备切换 E2E。本轮所有47个保护 owner文件 SHA 未变化。 |

Native ignored `ui::proxies::native_tests::real_selection_latency_draft_cleanup` **1 PASS / exit0**，600秒外层超时，`native-real-worker.json/log`。生产 Runtime/Controller 选择 b 并有新连接出口 marker；HTTPS a成功、b503、c超时；accepted socket blocking、TLS双向 bytes>0；正式 worker close/cancel 和 Quit 清理；Replace 撤销旧能力，原主快照保持，停止后端口重绑、线程 joined、SQLite 重建、临时 root 删除。Loopback fixture 的 marker 不冒充物理 Direct 路由验证。

## 命令与计数

全部实际 cwd 为本树；长 Cargo 命令外层600秒、`--offline --locked`、MACOSX_DEPLOYMENT_TARGET=15.0。唯一定向 **55 PASS = 原50 + Core新增历史回归1 + Desktop UI3 + Native1**；重跑不累计。

| 收据 / 命令范围 | exit / 用例 |
| --- | --- |
| `core-proxy-final`：Core proxies | 0 / 11，含原10与新增1 |
| `desktop-final-3`：`cargo test -p veyra-desktop ui::proxies::tests` | 0 / 3 |
| `native-real-worker`：指定 ignored Native，单线程/nocapture | 0 / 1 |
| `rust-clippy-current`：Core/Desktop all-targets `-D warnings` | 0 |
| `gui-build-current`：Desktop build | 0 |
| `rust-fmt-current` / `legacy-fmt-final` | 0 / 0 |
| `legacy-clippy-final` | **101 FAIL**，缺 Windows sing-box LICENSE；不绕过，不改资产 |
| `diff-current` | 0 |

原 Core/backend 的其余43个用例日志继续保留；不是当前 Desktop3 或 Native1 的替代。编译首轮、Native marker、fixture relay、Clippy collapsible_if/clone_on_copy、E0599、搜索私有模块 E0603 的 FAIL101 均保留并有修复后收据；不删历史失败。

## 尚未收口的最小范围

1. **Runtime owner 只读事实**：按 InstanceId + 完整 ConfigVersion 提供已应用 `PoolId → Vec<OutboundId>` 成员边和可确认 `PoolId → Option<OutboundId>` 当前出口。当前自动组拒绝 GroupApplicationUnknown/展示未知，不能算核心支持完成；本卡不越权另建目录或修改 owner DTO。
2. **Host 产品默认 URL 决定**：当前默认 HTTP 保留。正式节点测速只支持 HTTPS，精确提示“节点测速仅支持 HTTPS；保留原 URL，未执行测速”；本次测试明确授权的 HTTPS，不替换默认、不冒充 HTTP 成功。支持 HTTP 或调整默认须单列 Host 审定。
3. **最新候选实际 UI 与视觉**：新 surface/工具间距/协议检索/取消反馈源码已构建，最新 App 尚未启动。旧截图与同源 React measurements 仅是逐项依据，未达到95%整页/逐态结论；设置按钮当前进入 Panel Settings，与原局部 Proxy Settings modal 存在差异；hover/selected/loading/历史与颜色、浅深色、Zero/Unknown、草稿语言、最新托盘和在途退出仍需继续验收。字体使用已记录 GPUI系统fallback，局部blur/动画差异不以“不透明替代”掩盖。
4. **资源交接**：自有 loopback fixture/React server 正常 SIGTERM、监听重绑见 `gui-servers-cleanup.json`；临时浏览器 viewport/CDP恢复、缓存删除。PID83390/83493 与隔离数据root保留待用户正常 Quit及后续重启；不得写“全部资源清理完成”。外部 Karing/Clash helper、TUN、DNS、路由和系统代理保持，退出后仍需只读对照收据。

SystemProxy/TUN、物理 Direct 与完整平台组合继续按 P2-07/P2-09/P6-01/P6-05，不将组合延期当作以上本卡功能缺口的免责。独立 Reviewer 的[本轮报告](evidence/p2-08/desktop-20261010/REVIEW.md)只支持其实际审查边界。

原始证据均在 `evidence/p2-08/desktop-20261010/`，受仓库现有 ignore 影响，未强制 stage；Host 应保留本地证据。新包源码SHA在 `gui-bundle-next.json`，旧包SHA在 `gui-bundle-frozen.json`。本轮没有完成态提交或合并SHA。

## 最新候选追加实测（以上检查点保留为历史）

用户确认托盘退出后，旧 PID83390/main83493/草稿85367 均消失，60746/60747 重绑、draft roots 空；`gui-frozen-lifecycle.json` PASS。旧在途Quit仍NOT_RUN，原因不改写。

signed SHA `9b0a8112786d348e984f01c89e807fc70ffb99a7bc660e216d8866efbc604570` 已实际启动为 PID87937，自有 main88065，mixed63635/controller63636；当前 child Ready 日志与一次Start request2 Started见 `gui-next-ready.json`。首次 `gui-next-start.json` 记录的是candidate配置端口0，不能当监听端口，后续明确按owned Ready修正，不扫描外部端口。

最新候选真实繁体语言重启、历史恢复、HTTP协议筛选、在途c取消反馈、a HTTPS测速704ms、浅/暗色页面显示已有截图：`gpui-next-history-restored.png`、`gpui-next-protocol-filter.png`、`gpui-next-cancel-feedback.png`、`gpui-next-light-result.png`、`gpui-next-dark-result.png`。fixture记录 accepted_blocking=true、TLS up1707/down4081、owned relay88065；测速后当前a保持。恢复浅色后继续退出检查。

这些截图关闭“下一候选完全未启动”的旧边界，但不判整体95%或全部视觉/交互完成；局部ProxySettings modal、Zero/Unknown GUI、同态全对照、最新同PID托盘恢复和真实在途Quit仍待验。当前 app87937/main88065、重开的fixture87918与隔离root保留。此前服务清理收据对应旧82220/76850，不能写新fixture已退出。

## 最终退出与清理（2026-10-10）

用户确认 next 托盘 Quit 后，`gui-next-lifecycle.json` 核实 App87937/main88065/draft89403 全部消失、63635/63636重绑、draft roots空。日志顺序仍为草稿 `cleanup=true cancelled=false` 后才 `Quit`；**这次正常退出 PASS，真实在途 GPUI Quit 仍 NOT_RUN**，不再机械重试。正式 Native worker Quit PASS不改写这个边界。

新fixture87918已精确SIGTERM正常退出，四线程joined、65450/65451/65452/65453及主63635/63636均重绑。`gui-next-final-cleanup.json`、`final-owned-data-cleanup.json` 保存收据。已关闭SQLite、归档自有state/8条历史独立数据库及history JSON到 `final-owned-data/`，删除只读核验生成的空WAL/SHM；未归档Controller secret，整个 `/private/tmp/veyra-p208-1010` 已删除。无本次测试应用、内核、草稿、fixture、React server或临时root残留；bundle与必要原始证据保留。

`network-final-after.json` 核实系统proxy/DNS/default IPv4和IPv6与before逐字一致，外部822/1267/1356保持，未作为清理对象。当前本卡仍 **DOING**：auto owner只读事实、默认HTTP决定、实际Zero/Unknown、完整逐态视觉、最新同PID托盘恢复与在途GUIQuit仍未全部满足。五项不勾选，owner保留，55唯一PASS计数不变；没有提交/合并/push或下游启动。


## 2026-10-10 owner 只读事实消费者追加（此前记录保留）

原 WorkRun 同隔离树/HEAD；已原样引入 P2-06 owner 的6个Core文件改动（mod只合并导出），接通 ProxyService 同步 Applied 成员、实际应用图组测速与异步鉴权 Now 入口。Core自动组事实消费缺口已关闭；同步无HTTP，Saved/Applied分开，实例/版本/能力替换拒绝旧结果，无选择writer或新缓存。详见[P2-08 owner事实集成](P2-08-owner-facts-integration.md)。

本轮 Core170 + Desktop10 = 180唯一定向PASS/exit0，含新增消费者6例；原55覆盖为历史，不重复累计，4个Native opt-in ignored，不执行新GUI/Native。check/Clippy/build/fmt/diff exit0；Legacy Windows LICENSE Clippy FAIL101及01/02初始测试FAIL保留。原UI、Native bundle/截图/证据与owner树SHA保持，临时测试资源已清理。

P2-08与P2-06继续DOING、原owner/68卡状态不变，未stage/commit/merge/push。GUI须接异步typed观测并核验instance/applied config/页面generation，Unknown清旧显示；默认HTTP产品决定、完整视觉/Zero-Unknown GUI、托盘恢复/真实在途GUIQuit等原缺口保持，不以Core集成覆盖历史NOT_RUN或重绑旧截图到新构建。


## 2026-10-10 用户已批准默认HTTPS（此前记录保留）

用户正式批准仅新安装/新建AppState的 UI 测速默认改为 `https://www.gstatic.com/generate_204`；这是相对 React 原 HTTP 默认的授权差异，timeout/阈值、Runtime/group health、owner事实、UI布局/行为不变。已真实保存的HTTP不迁移，读取、无关偏好patch、语言保存与重开继续保留。旧HTTP测速未发Controller请求，正式节点/草稿统一三语言提示手动到设置修改网址，简中“节点测速仅支持 HTTPS，请到设置中修改网址；原设置未改”。旧“HTTP待决定”记录保留为历史，批准现已落实。

本轮21唯一定向PASS（Core14/Desktop7，新增3，未累计原55/上轮180与重跑），Core/Desktop all-targets check/clippy -D warnings/build、fmt/diff exit0；初始store测试FAIL101 5PASS/1FAIL（空AppState组fixture）及修复后6PASS收据均保留。Legacy资源FAIL101未改写、未复跑。自查及独立只读Review无剩余源码Finding；完整可重跑命令、行为绑定、原文件SHA保护及证据见 [默认HTTPS追加](P2-08-owner-facts-integration.md#2026-10-10-用户已批准默认https此前记录保留) 与 [本轮收据](evidence/p2-08/default-https-20261010/)。原signed bundle/截图/Native身份不变，没有新GUI/OS/外部网络执行，不将旧图重绑定到新构建。

Desktop最短接续：用隔离新root复验HTTPS默认；用另一个显式保存HTTP的隔离root验证重启/切语言保原值、测速拒绝和三语言新提示，然后用户手动改HTTPS后测速。新默认/新文案实际GUI、自动组typed观测GUI接线、完整逐态视觉/实际Zero-Unknown、最新同PID托盘恢复和真实在途GUI Quit继续待验/NOT_RUN。P2-08/P2-06 DOING、owner/68卡/READY不变，无stage/commit/merge/push。


## 2026-10-10 默认HTTPS专项真实GUI追加

P2-08继续DOING，原五项不勾选、不收口。最新源码无本轮产品修改，`cargo build --offline --locked -p veyra-desktop` exit0/600秒外层超时；unsigned SHA `d28499416008b721c3d4d904a628b19a20572dbc4febd1768e00f8da90c3dfe9`。新空目录signed SHA `d08442518ec37e4bff6b71225514e0a502740af534b8a20ce1b0fada9b966816`；更换自有root/log后重签的HTTP/重启signed SHA `3af4ae4bb63592739eed399d17c828518ee72b519ae19e940257f359d3cf1a23`。源码身份为HEAD6857b2e及本轮source-before.json清单，不把HEAD单独冒充脏树完整源码身份。

用户专项第1/2/3/5项PASS：空目录真实AppState写HTTPS默认；隔离保存HTTP目录实际切三语言、正常Quit/重启后HTTP及繁中保留，state.json bytes未变；Ready正式节点/有效草稿6张三语言拒绝图、无fixture目标请求/无草稿child；正式Core Controller请求计数守卫另实跑1PASS/exit0。English保存后的第一次测试因Saved≠Applied被准确拒绝，原图保留，产品Stop/Start自有实例后才取得正确HTTPS提示。

**第4项BLOCKED/NOT_RUN，Finding P2-08-UI-LATENCY-URL-001 OPEN**：实际正常面板设置无UI latency.test_url输入，策略设置只跳面板。InputKey::Url只在evidence render存在（behavior_panel.rs:457），不能作为正常用户手动修改入口；Backend的Runtime health与四站点URL均不替代它。未后台改URL冒充手动，未执行远端HTTPS成功测试。后续最小范围是依据原React/CSS提供可达的UI URL编辑/显式保存、复用BehaviorCoordinator/CAS，再实际手动保存HTTPS测节点/草稿；其余视觉/生命周期缺口保持，本轮不扩建完整代理设置modal。

正常用户托盘Quit后App10721/11729/13574及内核12600/13019/13203gone，6主端口重绑；fixture11666正常SIGTERM exit0/4线程joined/4端口重绑。归档state/关闭后SQLite，不归档Controller secret；`/private/tmp/veyra-p208-https-gui-1010`已删除。proxy/DNS/IPv4/IPv6默认路由与before逐字一致，外部代理822/1267/1356保持。构建、截图、操作及清理证据见 [专项实测](evidence/p2-08/default-https-gui-20261010/README.md)。历史FAIL/NOT_RUN/旧候选保留；无stage/commit/merge/push，READY/owner不变。

独立Reviewer `/root/review_p208` 只读核545个产品文件/两signed身份/六张实际图/HTTP归档重启bytes/guard日志与cleanup网络收据，确认专项1/2/3/5 PASS，4 BLOCKED，保留P2 Finding UI-LATENCY-URL-001；[Review](evidence/p2-08/default-https-gui-20261010/REVIEW.md)。本轮只验证未修复这个正常产品UI入口缺口，继续DOING。


## 测速地址正常 UI 最小修复收口（2026-10-10）

**P2-08-UI-LATENCY-URL-001 CLOSED；P2-08继续DOING。** 此为旧 default-https-gui 证据中缺失入口的后续实际修复，不改写旧 OPEN/BLOCKED/NOT_RUN。

唯一工作树/分支/HEAD保持本记录输入身份；新增产品变更仅 behavior_panel.rs 的延迟首项“测速地址”及i18n说明“用于节点测速”。稳定InputKey::Url、原draft、Edit/Save及ConfigVersion CAS复用；不新增配置/接口，不改Core、Runtime/group health/test_sites、P2-06、节点选择测速控制或React参考。843项进入时SHA与最终构建SHA保留。

实际最终App SHA：new-data `fcda2615577135243899166c5849273df01eeded3cef3bc24e7aa0afd2384d16`，saved-http `359e59ed6f348819e4f643c5a51647ddcdea6ce656f7ee7f08fa04ae53a3660c`。新目录HTTPS默认PASS；旧HTTP保持PASS。真实中部编辑焦点/光标、未保存保旧、非法URL LatencyUrl拒绝、StorageFailed/Replace保HTTPS草稿和旧HTTP均PASS；只从UI手动保存HTTPS成功。简中/繁中/English及浅暗色显示正常。

全部保存完成后通过正式UI启动固定sing-box1.14.0、当前实例Ready；正式节点真实Controller/NetworkService **768ms PASS**，独立草稿节点 **791ms PASS** / cleanup=true。自有loopback CONNECT/TLS字节、SQLite正式结果、Controller选择前后相等及profile/groups/test_sites保留已核。公网gstatic实际HTTPS经自有relay，不声称自有gstatic或物理Direct。未改运行配置文件绕过版本应用。随后用户正常托盘退出，同最终App重启显示HTTPS/繁中，state逐字节不变。

本轮复跑 **37唯一定向PASS（Core25/Desktop12）**，check/Clippy all-targets/build/fmt/diff PASS。首轮fmt格式FAIL1修正并重build；Legacy Clippy仍FAIL101（现有Windows sing-box.exe资源缺失），不绕过、不改无关资产。独立Reviewer已核源码、实际图/日志、App SHA、state/SQLite及清理收据，无本轮可信剩余Finding。

React原5项双列变6项仍3行，新增URL置左1、其余顺序移位；共享Grid/输入高度边距圆角字体颜色沿用，单列理论增44px。记录为本次用户批准的最小功能差异；参考harness语言/外壳/DPR边界明确，**不能据此宣称完整页面95%视觉通过**。1280×720保存按钮当前页下方不可见，真实保存通过原生zoom扩大窗口后完成，再恢复1280×720，不扩展重设计。

所有本轮App/main/draft已正常退出、自有fixture clean_exit/线程joined，9端口重绑、socket无残留；归档后删除隔离root与Vite cache，浏览器viewport恢复。系统proxy/DNS/IPv4/6默认路由与开始前逐字一致。App与可审查证据保留，未stage/commit/merge/push/发布。

完整记录、截图与源码/运行/清理身份见 [本轮README](evidence/p2-08/latency-url-fix-20261010/README.md) 与 [独立Review](evidence/p2-08/latency-url-fix-20261010/REVIEW.md)。原五项验收checkbox、owner及READY不改变；自动组GUI、Zero/Unknown、完整视觉、真实在途Quit各以独立证据后续收口。


## 2026-10-10 最终 GUI/当前浅色验收追加（P2-08 DOING）

唯一工作树/分支及HEAD不变。本轮最小异步接线消费生产 `observe_applied_pool`，完整InstanceId/ConfigVersion/页面代次核验，自动组current保Node/Pool/Direct、不猜叶子、不增加选择writer；版本变化清旧显示为Unknown。正常面板设置1280×720可滚动44px到保存，初轮工具自然滚动方向误判已更正，Settings源码原字节恢复，不声称修了布局bug。

最终 signed Mach-O SHA256 `d8e0d50661487fafccb213e0af6ef879904dabdcfadedd0fec872bd838c1e585`；产品源码manifest SHA256 `59e4acac72560147db969be5aed2195e2fe72bf2f76ed5cbf9876e4611b38ba7`。最新App45385正式694ms落SQLite、草稿796ms真实TLS并清理；保存rev4但仍应用rev3时Unknown，通过正式Stop/Start应用后自动a/Direct，当前选择b；同PID Hide→用户真实TrayShow→原window_count4恢复通过。节点/组/Runtime profile/test sites未变。所有自有App/main/draft/fixture正常退出、27端口重绑、socket/locks清理、隔离root归档删除，浏览器视口恢复；用户代理/DNS/IPv4/6默认路由一致。

Core读取/事务错误误标StateChanged已最小修为typed StateFailed（仅真实SnapshotChanged/RevisionConflict继续过期分类），真实Gate Busy保Success25旧历史及释放后Recorded测试PASS；不自动重试，不改P2-06 Runtime读循环。最终改动后Core18/Desktop7/i18n3共28唯一PASS，check/Clippy/build/fmt/diff PASS；本轮早期未受后续修改影响15项另列，跨构建去重43不冒充全部最终包新跑。Legacy Windows LICENSE Clippy FAIL101保留。

**未通过项不勾选**：`P2-08-GUI-HISTORY-WRITE-001 OPEN`，较早63320fef包真实733/734ms历史丢失FAIL，分类修正和一次诊断675ms/最终694ms成功不能证明竞争消除；固定1.14.0将0转503，真实Zero GUI NOT_RUN；唯一草稿TrayQuit在30秒Timeout后才Quit，取消回执NOT_RUN且不重试；特意命中旧自动观测在途换实例GUI窗口NOT_RUN；完整浅色逐态视觉未通过（订阅Tab范围、三组/一组、字体fallback、hover/history/loading/error对应证据等差异）。其它缩放/深色矩阵待全部功能完成。测速地址Finding继续前轮CLOSED，局部URL差异记录保留，不重做已验收业务、不宣称95%。P2-08继续DOING、P2-06原owner、状态计数/READY/五项checkbox不变；没有stage/commit/merge/push/发布，不交Host DONE。

完整身份、逐态截图/差异、历史失败、命令、退出时刻边界、清理与独立审查见 [本轮证据](evidence/p2-08/final-gui-20261010/README.md) 和 [REVIEW](evidence/p2-08/final-gui-20261010/REVIEW.md)。
