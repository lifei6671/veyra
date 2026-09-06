---
id: TASK-010
milestone_ref: M7
dependencies: [TASK-009]
risk: HIGH
status: DONE
requirement_ref: docs/veyra.md
requirement_identity: sha256:ddd1d20999348e6a1f8f2091539e27bc6729597f59e35357ed2cb2c7c29cad74
design_refs:
  - docs/decisions/DCR-017-ui-integration-boundary.md
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-005-traffic-observation.md
  - .sdlc/design/DCR-006-memory-traffic-trend.md
  - .sdlc/design/DCR-007-dual-traffic-windows.md
approval_refs:
  - .sdlc/evidence/TASK-010/change-control-001.json
  - .sdlc/evidence/TASK-009/acceptance-001.json#EVIDENCE-TASK-009-ACCEPTANCE-001
---

# TASK-010：桌面导航、首页连续性与原生窗口/Tray 联合验收

## 本次需求

参考本地 Clash Verge Rev 的侧栏、页面布局和桌面交互，为 Veyra 增加可操作的一级导航与
尚未交付页面的诚实页壳，并在真实 Windows WebView 中完成首页、窗口隐藏和 Tray 恢复的
联合验收。现有首页观测 owner 保持在应用根部挂载；切页、隐藏和恢复不得重新订阅、重建
React Root、重启受管内核或丢失当前运行与趋势状态。

需求依据为 `docs/veyra.md` §1.2、§100–103、§107、§143.1 与 Phase 7，职责边界沿用
DCR-017。参考映射为 Clash Verge Rev `src/pages/_navigation.tsx` 的导航元数据、
`src/pages/_layout.tsx` 的 `navMenuItems`/`the-menu` 布局及其现有侧栏结构。Veyra 使用
“首页 / 出口组 / 订阅 / 连接 / 分流 / 日志 / 设置”；不引入参考项目的 Unlock，
不复制 Mihomo API、配置、脚本、依赖或未批准功能。

## Scope

### allow

- `src-tauri/tauri.conf.json`：用户本轮明确要求限制窗口缩小，仅为 main 设置 minWidth/minHeight=520，沿用 Clash Verge 的逻辑尺寸下限。

- `src-tauri/capabilities/default.json`：USER:lifei CHANGE001 明确批准仅为 main 窗口补充
  `core:event:allow-listen`、`core:event:allow-unlisten`，恢复既有观测订阅；不增加发送事件权限。

- `src/App.tsx`、`src/styles.css`：只实现一级导航、选中/悬停/键盘状态、诚实页壳、响应式布局，
  并确保现有首页观测逻辑在应用根部持续挂载。
- `src-tauri/src/lib.rs`、`src-tauri/src/commands.rs` 及文件内现有测试：只在真实窗口/Tray 联合
  验证发现本 Task 的具体缺陷时，局部修复既有 CloseRequested→hide、Tray show/focus、可见性
  门控、当前安全 Snapshot 通知或 Quit→controller shutdown 顺序。
- `src/**/*.test.ts` 中与上述导航、根挂载和首页连续性直接相关的最小测试，以及
  `.sdlc/evidence/TASK-010/**` 下不含 secret、私有配置原文或敏感载荷的脚本、日志和截图。

### deny

- 不实现 TASK-011 的订阅导入/更新，TASK-012 的出口组选择/分流编辑，或 TASK-013 的连接、
  日志、设置业务能力；对应入口只能展示明确标题和诚实空态，不得伪造数据、操作或成功状态。
- 不新增 Router/状态库、第三方依赖、锁文件、公共 IPC、CHANGE001之外的Tauri capability/权限、listener、
  持久化导航 schema、Deep Link、Auto Start、Updater、通知或平台适配层。
- 不修改订阅/Provider、Domain、Compiler、数据库/migration、System Proxy、TUN、UAC、WFP、
  Service、主机网络或生产配置；不新增 sing-box 协议、地址族、DNS/TLS 或包级验证。
- 不销毁/重建 WebView 来模拟页面切换或恢复，不通过刷新页面、重启应用或重启内核掩盖状态问题。

## SF-001：一级导航与诚实页面壳

**需求：** 在既有侧栏内加入“首页 / 出口组 / 订阅 / 连接 / 分流 / 日志 / 设置”入口。
首页对应需求中的“概览”；其它入口只建立后续 Task 可接入的页面壳。使用应用内存中的当前页状态，
不引入 Router 依赖、URL/Deep Link 契约或持久化字段。

**验收：** 鼠标与键盘均可切换入口；当前入口有唯一 `aria-current` 和与参考主题一致的选中、悬停、
焦点状态。每个页壳只显示 Veyra 产品术语、页面标题和“功能尚未配置/将在对应功能交付后可用”一类
诚实空态，不显示假列表、假计数、无效按钮或 Mihomo/sing-box 配置术语。桌面及 最小 520×520 窗口 无横向
溢出、遮挡或不可达入口，浅色/深色下文字、焦点和边界可读。

**Verification：** 组件/浏览器交互验证入口顺序、鼠标/键盘切换、唯一选中态、页标题、空态、
无额外 IPC 和 最小 520×520 窗口右侧重排；分别回读浅色/深色桌面及窄屏截图。实际源码与参考
`_navigation.tsx`/`_layout.tsx` 的映射写入 Evidence，不以截图相似代替交互断言。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## SF-002：首页根挂载与切页连续性

**需求：** 保留 TASK-009 已验收的首页 Start/Stop、Toast、观测和双时间窗实现。页面切换只改变
展示面；现有 observation subscription、显示时钟、动作状态和趋势数据继续由同一个应用根实例持有。

**验收：** 从有数据的首页切到任一页壳再返回，首页 DOM/状态 owner 未被重新创建，运行状态、
最近 10 分钟/60 秒趋势、当前速率、累计量和未过期操作结果保持；切页期间不新增
`runtime_observation_snapshot`、Delta subscription、Start/Stop 或网络请求。切页时不可见首页不覆盖
当前页、不可获得键盘焦点，返回后无重复事件处理、重复 Toast 或计时器叠加。加载、空、忙碌、
错误和已停止状态在导航布局中仍可读。

**Verification：** 使用当前 React 页面和官方 Tauri mockIPC 记录命令/订阅计数、页面节点身份与
切换前后状态；定向验证首页加载/空/忙碌/错误/Stop 及 Toast 没有回归。Browser Evidence 只证明
前端交互，不能替代 SF-003 的真实 Windows WebView/Tray 结果。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## SF-003：真实 Windows WebView、窗口隐藏与 Tray 恢复

**当前验收修订（USER:lifei / closure-012）：** 用户已明确确认按已验证的界面与托盘行为验收；以下原生精确观测保留为 `NOT_RUN`、移至 M7 发布前补验：隐藏期采样修订与 Delta 抑制/无积压、原生 IPC/订阅精确计数、人工恢复的同 HWND/React Root 身份与单次安全 Snapshot 时序、退出前 controller shutdown 顺序、真实 WebView 受控封闭错误与 Toast，以及完整同进程时序。下列描述继续作为产品行为要求及补验目标，但上述观测不再阻塞本 Task 的交付验收，不得声称其已实测通过。批准记录见 `closure-012-acceptance.json`，原候选为 `proposed-closure-012.json`。

**需求：** 使用现有 Tauri 主窗口、CloseRequested→hide、Tray“显示 Veyra”→show/focus、
`MainWindowVisibility` Delta 门控和 Quit→controller shutdown 接线，验证真实桌面生命周期。
窗口隐藏时 Rust backend 与当前受管 sing-box 继续运行；恢复使用同一窗口和 React Root。

**验收：** 在隔离测试数据目录启动真实 Windows 应用，记录应用 PID、主窗口身份、受管 child 身份
及当前页。点击窗口关闭后窗口隐藏但进程和自有 child 未退出，后端继续采样；隐藏期间不向不可见
WebView 积压 Delta，不新增前端 Snapshot/订阅或业务 IPC。通过真实 Tray 菜单恢复后，同一窗口被
show/focus，当前导航页、首页状态和隐藏期间的最新趋势连续；只允许既有恢复路径发送一次当前安全
Snapshot Delta，不重读 state、重建 React Root 或重启 child。Tray“退出”仅在 controller shutdown
确认后退出应用并清理自有 child，不终止无关进程。

真实 WebView 还须实际操作首页 Start/Stop，并观察加载、空、操作 pending/Busy、封闭错误、Toast
和恢复后的状态；不能以 Mock、构建成功或进程存活代替。证据不得包含 PID 之外的私有路径、secret、
完整配置或原始日志载荷。若原生自动化或 Tray 控件在实际运行时不可访问，结果必须记为
`UNAVAILABLE` 并保留验收缺口，不能改记 PASS。

**Verification：** 使用当前已可用的 Windows 原生自动化能力驱动真实 Veyra WebView、窗口关闭和
Tray 菜单，记录带时间顺序的窗口可见性、焦点、应用/child identity、前端命令/订阅计数与恢复后
安全状态，并截图回读首页、页壳、隐藏前后恢复及 最小 520×520 窗口。按受影响范围运行 `pnpm test`、
`pnpm lint`、`pnpm build`；若修改 Rust，则运行精确非零窗口/Tray 测试、
`cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` 与
`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`。所有长命令使用合理外层超时。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## Task 独立验收

**当前验收口径：** 按 closure-012 用户确认，将 SF-003 列出的未观测项保留 `NOT_RUN` 并转入 M7 发布前 follow-up。本次整体证据由当前字体/布局与浏览器交互、未改行为的原生 Start/Stop/导航、原生最小尺寸与运行中隐藏、用户人工 Tray 恢复/退出及进程读回组成。分别标注证据适用范围，不将多会话拼接为单一进程时序，不以本 Task 验收替代发布前补验。

在同一个真实 Veyra 应用进程中，从首页启动或观察当前受管内核，切换全部七个一级入口并返回，
证明页壳诚实、首页根实例和观测连续；随后关闭主窗口、确认 backend/child 继续运行，通过真实 Tray
恢复同一窗口并保持当前页和隐藏期间最新趋势，最后经 Tray 退出确认 controller shutdown 与自有
child 清理。实际 Windows WebView 同时覆盖首页 Start/Stop、加载/空/忙碌/错误/Toast；桌面与
最小 520×520 窗口、当前宿主主题的截图可读，导航样式的浅色/深色 Browser 回归通过。无重复订阅/IPC、WebView
重建、内核重启、敏感信息泄露、无关进程终止或系统配置变化。三个 SF 须分别有当前身份 Evidence，
再冻结完整 Delivery Unit 交独立 Review；Task 与 SF 验收仍需 USER:lifei 明确确认。

**acceptance_status：** PASSED

## Dependencies、Risk 与 Readiness

- TASK-009 已由 USER:lifei 对三个 SF 与 Task 明确验收，身份
  `sha256:43f29eef134da2f2266c41167d8650d2e0a36edca10417cc2dccead587e12892`；
  本 Task 不重做其 sing-box 协议能力验证。
- 主要风险是导航切换或窗口恢复重建首页 owner、重复订阅/IPC、丢失隐藏期间状态，或把
  后续业务页壳误呈现为已实现功能；验证必须绑定同一应用/window/child 身份。
- 当前宿主已发现可操作真实 Windows 窗口的原生自动化能力，无新增 driver 依赖；该可用性仍须在
  Implementation 运行时实测。工具失败只影响 Evidence 状态，不授权改依赖、权限或系统配置。
- 低风险假设：当前页只保存在 React 内存，关闭/隐藏时自然保留，应用退出后回到首页；若后续要求
  URL 历史、Deep Link 或跨重启页面恢复，该假设失效并需独立需求/设计，不在本 Task 内推断实现。

## 当前验证进度

2026-09-05 USER:lifei 授权按 Clash Verge Rev 源码和所附设置页截图整改当前 UI。
本次视觉整改覆盖 App.tsx/styles.css 的侧栏、SVG 导航图标、独立页头与内容滚动、卡片排版、
字号间距和底部流量展示；延续参考浅深主题、字体与用户 Logo。以参考源码的 CSS 像素为准，
不将截图的显示缩放比例直接作为布局尺寸。不新增设置业务、依赖、后端或权限。
验收补充：960×640 默认窗口下导航与侧栏速率可见，页头/侧栏不随内容滚动；
1280×860 和 最小 520×520 窗口 下浅深主题无横向溢出；原首页状态与切页连续性保持。
旧视觉证据及 delivery-review-001 对变化部分失效，待 visual-003 当前证据和独立复审。

SF-001/SF-002：frontend-002.json 与 browser-002-verification.json；SF-003：native-verification-002.json 为 PARTIAL。原生 Start/Stop、实时 Delta、切页及停止后关闭隐藏已观察；真实 Tray 恢复/退出和运行中隐藏连续性仍未验证，Task 与 SF 验收保留 PENDING。

## 窗口缩放修订（USER:lifei / resize-008）

用户明确要求窗口缩小后菜单始终位于左侧，只让右侧内容调整宽度，且限制最小尺寸。
参考 `Clash Verge Rev/src-tauri/src/utils/resolve/window.rs:19-20,89`，主窗口采用
520×520 逻辑像素下限，侧栏固定200px；取消菜单顶部两列重排。默认960×640不变。
原320px移动式顶置验收由本修订替代，不再作为桌面支持尺寸；参考截图的物理像素不作为逻辑尺寸。
在520×520、680×640、960×640浅深主题验证左栏固定、右侧正常换行/独立滚动、
导航与底部速率可达、窗口缩放不新增IPC或重建首页。原生最小尺寸配置与浏览器布局证据分别记录，
未实测原生拖拽不得记为PASS。此前视觉审阅仅未改分区可复用，变化分区重新验证。

## 菜单排字修订（USER:lifei / font-011）

用户再次要求按所附Clash Verge菜单截图调整字体观感。本次仅调整菜单排字：
16px字号、400常规字重、24px行高，中文标签4px字距并保持居中。
字体栈沿用参考_theme.tsx；字间疏排参考zh/layout.json的“首 页”。
本地layout-item.tsx写700字重，与用户提供的常规字重截图有差异，本次以用户截图为准，
不宣称400来自该源码。保留标题/品牌字重、颜色、Logo、520最小尺寸和200px左栏。
在520/960浅深截图核对真实字体、标签无截断与导航可达；不改业务/IPC/字体依赖。

## 最终验收记录（closure-012）

USER:lifei 明确确认已呈现的验收范围修订与继续 TASK-011。三个 SF 与 Task 按当前修订口径验收 PASSED，Task DONE。当前实现身份仍为 sha256:d7082fb55932a8a25591ee6ceff7e43fadb1286bd37a13350e00f089e957b3f8；closure-012-review.json 独立整合 PASS、无开放 Finding。原生未观测项由 M7 FOLLOWUP-TASK010-NATIVE-012 保留 NOT_RUN，发布状态未评估。上方历史进度仅表示当时状态。

