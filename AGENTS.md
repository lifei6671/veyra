# Veyra 开发规范

## 开发规范

- 必须有足够的中文注释
- 禁止过度设计，比如极端边界情况或极端安全边界等

## 产品定位与 UI 参考

- Veyra 为 sing-box 提供桌面 UI，以及实现 UI 所必需的订阅、状态持久化、配置转换、进程管理和平台适配。
- 当前迁移路线为 Rust 本地服务与 GPUI，依据 `docs/openbox-rust-gpui-implementation-plan.md`。行为以现有页面及对应接口为参考；视觉事实来源必须遵守下述强制规范，记录必要的平台差异。
- 保留浅色/深色、布局、交互、字体/图标来源和许可；GPUI 使用其文本系统支持的字体资源，不直接照搬网页字体加载方式。
- 保留领域模型、Compiler 和平台适配边界；sing-box 负责数据转发，不扩展为通用代理平台或多内核框架。旧 Tauri/React 入口在迁移期间保留对照。
- 旧 SDLC 状态、UI Contract 强制绑定、Compliance/Human Visual Approval 门禁已于 2026-10-03 按用户要求停用。`docs/ui/veyra-ui-spec.md`、旧设计/DCR 和审计只作历史参考，不触发恢复旧流程、补办批准或等待逐 Task 人工放行。
- 分发使用 GitHub Releases，不要求开发者账户、Developer ID/Authenticode 发布签名或 Apple 公证；Apple Silicon 的最低 ad-hoc 运行要求按技术方案单独说明。第三方驱动的系统签名要求仍适用。

## UI 视觉迁移与组件复用强制规范

本节是后续 Rust/GPUI UI 开发的项目最高层持续规则（MUST）。方案、任务卡和验收记录必须遵守；仅用户明确批准的变更可改变视觉基线。

- **唯一视觉事实来源（MUST）**：除 macOS 必须使用原生 UI 的系统区域外，现有 `src/openbox/**` React 实现及 `src/openbox/openbox.css` 是 GPUI 的唯一视觉事实来源，必须包含 CSS 级联、页面局部覆盖、浅色/深色与交互状态。GPUI 是视觉迁移，禁止未经用户明确批准重新设计。
- **精确迁移（MUST）**：禁止主动改变菜单宽高、背景色、字体、字号、字重、行高、间距、边框、圆角、卡片尺寸、图标、图标大小、stroke/几何，以及 hover/focus/active/disabled/loading/selected 等视觉结构或状态。React/CSS 有明确尺寸、颜色或数值时，必须按相同逻辑像素、颜色、数值及其生效逻辑迁移，禁止以“近似”替代。
- **一致性目标（MUST）**：尽最大可能保持一致，总体视觉至少 95% 一致。95% 是最低工程目标，不是允许任意近似的额度，也不以单一全图像素相似度作为唯一 Gate。每个已实现页面和核心状态必须独立逐项验收，禁止用全局平均分掩盖局部偏差；肉眼可见的明显尺寸、颜色、图标或布局差异是 blocker，必须修复。仅 GPUI/macOS 技术上确实无法等价的差异可保留，必须记录具体技术原因、影响区域和对应视觉证据，不能声称完全一致。
- **原生边界（MUST）**：macOS 原生标题栏、NSOpenPanel/NSSavePanel、托盘菜单等由 OS 绘制且 React 不拥有的系统区域不参与 React 像素复刻；应用自绘区域全部受本节约束，禁止借“原生风格”改变其外观。
- **GPUI Kit 使用（MUST）**：仅复用行为、焦点、输入等基础能力，外观必须包装/覆写至 React 基线；禁止直接以 GPUI Kit 默认视觉替代 React 视觉。
- **设计层级（MUST）**：采用 `Design Tokens/Theme → 基础组件 → 跨页面组合组件 → 页面`。重复的颜色、固定尺寸、控件高度、圆角、间距、字体、字重、图标尺寸、边框、状态色等必须集中定义并可追溯到 React/CSS，禁止散落 magic numbers；集中管理不得抹平原有页面或状态差异。
- **复用触发条件（MUST）**：React 中复用的模式，或 GPUI 两处及以上真实出现的相同/近似视觉交互，必须抽成共享组件。页面必须以组合共享组件为主，禁止同类按钮、设置行、卡片、工具栏、状态标签等在不同页面各写一套视觉逻辑。优先共享 Button/IconButton/Input/Select/Switch/Segmented/Modal/Tooltip/Toast/Surface/Card/FormRow/PageToolbar/EmptyState/Loading/Error/NavigationItem/StatusBadge 等语义；名称可随现有结构调整。禁止为未来假设过度抽象，真正一次性的布局可保留页面局部实现。
- **图标与字体（MUST）**：图标优先复用 React 已使用的同源 SVG/资源或等价几何，禁止用“差不多”的 glyph、emoji 或其他图标替代；必须保留资源来源和许可。字体以 React 基线为准，优先 MiSans/NotoEmoji 等已确认来源，并使用 GPUI 文本系统支持的资源格式；无法使用相同字体时必须记录具体技术差异及视觉证据，禁止称完全一致。
- **视觉验证（MUST）**：React 与 GPUI 必须在相同窗口尺寸、缩放、主题、页面状态及尽可能相同数据下做对应截图，禁止比较不同状态截图得出整体相似度。逐页/逐核心状态对照几何、颜色、排版、图标和交互状态，保留截图、叠图或测量及差异处理结果；功能测试、构建成功不能代替视觉验收。
- **Legacy 视觉参考（MUST）**：对应 GPUI 页面完成视觉对齐并留存可追溯视觉基线/证据前，禁止因“功能已迁移”删除 Legacy React 视觉参考源码或资源；仍被其他未对齐页面使用的共享 CSS、字体、图标必须保留。

## 验收责任边界

- 测试 Veyra 自己负责的行为：UI 输入与反馈、订阅归一化、保存和恢复、领域到配置的映射、最终配置 `sing-box check`、同一配置的启动/停止、错误传播、观测映射与实例隔离、桌面/平台资源归属。
- 对真实 sing-box 只做与当前功能直接相关的最小集成检查：生成配置被加载、固定鉴权 API 可用、必要时一个已有受控连通场景、停止清理。单测和 Mock 覆盖 Veyra 的失败/并发/UI 分支；Mock 不冒充真实桌面结果。
- Veyra 自己生成或管理的安全策略仍需验证，例如置顶拒绝规则、防止任意配置透传、私有配置权限、secret 脱敏和只停止自有 child。优先静态正反配置断言及已有代表性集成证据，不扩展成全协议/全地址族矩阵。
- 不重新验证 sing-box 的 TCP/UDP、WireGuard、IPv4/IPv6、TLS、DNS、URLTest 算法、校验和、重传或包级 ACK 实现；不为这些项目建立新的 peer、用户态网络栈、报文解析器、抓包框架或测试入口。
- 只有出现可复现、影响具体 Veyra 用户操作的缺陷，才围绕最小复现诊断内核边界；诊断不自动变为后续功能的通用验收前置条件。
- 每项新测试先说明它保护哪一项 Veyra 用户行为/自有契约。无法对应时不新增测试。兼容性升级检查只针对 Veyra 使用的配置/API/进程契约，不重新认证内核。
- 界面变更必须验证实际操作、加载/空/错误/忙碌状态和必要截图；解析单测、构建成功、内核启动成功都不能代替 UI 验收。

## 任务恢复与变更

- 后续 Veyra Rust/GPUI 开发默认遵守 [长期开发总规范](docs/openbox-rust-gpui-tasks/DEVELOPMENT_WORKFLOW.md)。恢复时读取该规范、`docs/openbox-rust-gpui-tasks/SESSION.md`、任务总表和相关任务卡，只加载相关方案小节；任务文档记录进度，不依赖隐藏状态文件或流程引擎。
- P0–P7/Windows 仅作里程碑分组及组合验收，不作统一串行 Gate；READY 由每卡显式依赖全部 DONE 决定。允许多个 READY 且写范围不冲突的任务并行 DOING，SESSION 使用 Active Tasks / Ready Queue / Blocked。
- 公共领域模型、共享 DTO、Cargo workspace 和 Runtime 公共契约各设单一 owner；并行前登记写范围/资源冲突，公共契约先由 owner 整合。UI fixture 可以先做，但真实服务/Runtime/Observation 集成及实际 UI 验收完成后才 DONE。
- 配置事实→派生目录→Runtime→Observation→UI 保持单向事实归属；Unified OutboundCatalog = Base OutboundCatalog + Group/Failover + ChainProxy，统一负责 self/cycle/dangling 校验，Chain 保存后注册为 Outbound 并先于 Routing/Client Routing 完成。
- 保存成功与已应用分别反馈；同一 state_epoch 下 config_revision 与 selection_revision 独立推进。UI latency preference test URL、Runtime health test URL、group health URL 三种语义不合并，UI diagnostics ipv6-test 与 Runtime profile ipv6 不合并。
- 历史测试、失败和证据不改写为通过。已经明确退役的流程脚本及其专用测试可以随流程清理，业务测试继续保留。
- 已存在的测试不自动删除/禁用。代码改动影响到的 Veyra 自有逻辑仍做定向回归，避免仅因历史庞大测试设施存在就扩建它。
- 当前 Task 验收与后续 Task 保持独立；修改验收范围不等于实现完成或用户验收通过。
- Legacy 采用“完成一片，退役一片”：`src/openbox`/`src-tauri` 对应职责在新实现真实接管、验收通过且无剩余引用后立即随切片删除；涉及 React 视觉参考时还必须满足本节视觉对齐及基线/证据留存条件。未迁移部分继续作为参考，禁止开发前整体清空旧 React/Tauri。生成物和本机状态可独立清理。
- 保留无关工作区变更；未经明确授权不 commit/push 或发布。当前请求或已批准 Task/Design 已覆盖的依赖、主机网络、权限与环境操作无需重复确认；超出已有授权的实质变更须先取得用户决定。
- 在当前授权范围内持续完成实现、必要验证与修复，按风险进行审查。记录结果和剩余问题后更新任务状态；不因缺少旧门禁文件或逐 Task 人工签字中断。用户明确要求的人工确认、发布授权和真实平台验证仍按实际任务执行。

## 验证命令

- 前端按改动需要运行 `pnpm lint`、`pnpm test`、`pnpm build`。
- Rust 改动按当前 Task 指定的测试名/target 验证，并运行 `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`；不因无关文档或前端改动重复运行 Rust 检查。
- 先确认测试会否启动真实 child 或外部资源；不能把全量库测试当成纯单测，也不能把过滤后零测试记为通过。
- 长命令必须有合理外层超时；使用仓库已有脚本/约定，不凭空推断命令。文档规划变更只校验引用、身份、结构和 `git diff --check`，不启动内核或网络。
- 在已授权的环境中修复本次改动引起的失败并重跑受影响验证，无需逐步确认。适用验证通过且没有新改动、失败或未解决的明确疑点后停止扩大验证，按当前任务验收项记录完成情况。
