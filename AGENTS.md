# Veyra 开发规范

## 产品定位与 UI 参考

- Veyra 为 sing-box 提供桌面 UI，以及实现 UI 所必需的订阅、状态持久化、配置转换、进程管理和平台适配。
- 当前迁移路线为 Rust 本地服务与 GPUI，依据 `docs/openbox-rust-gpui-implementation-plan.md`。UI 以现有 `src/openbox` 页面、对应接口和用户指定的参考环境为准，记录必要的平台差异。
- 保留浅色/深色、布局、交互、字体/图标来源和许可；GPUI 使用其文本系统支持的字体资源，不直接照搬网页字体加载方式。
- 保留领域模型、Compiler 和平台适配边界；sing-box 负责数据转发，不扩展为通用代理平台或多内核框架。旧 Tauri/React 入口在迁移期间保留对照。
- 旧 SDLC 状态、UI Contract 强制绑定、Compliance/Human Visual Approval 门禁已于 2026-10-03 按用户要求停用。`docs/ui/veyra-ui-spec.md`、旧设计/DCR 和审计只作历史参考，不触发恢复旧流程、补办批准或等待逐 Task 人工放行。
- 分发使用 GitHub Releases，不要求开发者账户、Developer ID/Authenticode 发布签名或 Apple 公证；Apple Silicon 的最低 ad-hoc 运行要求按技术方案单独说明。第三方驱动的系统签名要求仍适用。

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
- Legacy 采用“完成一片，退役一片”：`src/openbox`/`src-tauri` 对应职责在新实现真实接管、验收通过且无剩余引用后立即随切片删除；未迁移部分继续作为参考，禁止开发前整体清空旧 React/Tauri。生成物和本机状态可独立清理。
- 保留无关工作区变更；未经明确授权不 commit/push 或发布。当前请求或已批准 Task/Design 已覆盖的依赖、主机网络、权限与环境操作无需重复确认；超出已有授权的实质变更须先取得用户决定。
- 在当前授权范围内持续完成实现、必要验证与修复，按风险进行审查。记录结果和剩余问题后更新任务状态；不因缺少旧门禁文件或逐 Task 人工签字中断。用户明确要求的人工确认、发布授权和真实平台验证仍按实际任务执行。

## 验证命令

- 前端按改动需要运行 `pnpm lint`、`pnpm test`、`pnpm build`。
- Rust 改动按当前 Task 指定的测试名/target 验证，并运行 `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`；不因无关文档或前端改动重复运行 Rust 检查。
- 先确认测试会否启动真实 child 或外部资源；不能把全量库测试当成纯单测，也不能把过滤后零测试记为通过。
- 长命令必须有合理外层超时；使用仓库已有脚本/约定，不凭空推断命令。文档规划变更只校验引用、身份、结构和 `git diff --check`，不启动内核或网络。
- 在已授权的环境中修复本次改动引起的失败并重跑受影响验证，无需逐步确认。适用验证通过且没有新改动、失败或未解决的明确疑点后停止扩大验证，按当前任务验收项记录完成情况。
