---
name: technical-design
description: "Produce a greenfield foundation or remediate traceable designs for material engineering boundaries. Use before the first task in a greenfield project, or when a task changes architecture, public API, persistence, permissions, security, concurrency, deployment, or another material decision; do not use for design approval, code implementation, QA, or release authorization."
---

# Technical Design

基于 Anchor 和 Requirement Source 为真正的工程边界形成可实现、可评审、可冻结的 Technical Design。
支持互斥模式 `foundation`、`task-boundary`、`remediation`。该 Skill 是 Design Producer；它可以
提出和记录决策，但不能批准 Technical Design Gate。

## Context Contract

### Required Context

- 当前 Anchor、Requirement Source、Acceptance 与约束；
- `.sdlc/state.yaml`、`.sdlc/memory/HANDOFF.md`；
- 明确适用的兼容性、安全和运行约束。

### Conditional Required Context

- `foundation`：已完整读取的 Requirement Source、Repository Snapshot、Project Constraints；不要求
  `tasks.yaml` 或 Current Task；
- `task-boundary` / `remediation`：`.sdlc/tasks.yaml`、Current Task Markdown、相关现有架构；
- `remediation`：需修复的 Design Review Finding 或 Frozen Design/DCR 上下文。

### Optional Context

- 已存在的 Story、`.sdlc/design/INDEX.md`、Existing ADR/DCR 和相关源码符号；
- 竞品或技术调研、POC Evidence、部署和容量事实；
- 实验、数据采集和可观测性要求。

### Forbidden Default Context

- 全部历史 Task、QA、Release、Evidence 或完整源码树；
- 与当前设计无关的所有 ADR、Story 和外部资料；
- 未经授权的生产配置、凭据或真实生产操作。

## 执行

1. `foundation` 只用于 `greenfield` 且尚无技术基线的项目，在第一行实现前形成唯一
   `.sdlc/design/foundation.md`；`established` 项目沿用现有约定，不创建 Foundation。
2. Foundation 只包含 Stack、Architecture、Project Layout、Data Access、API/Transport、Key Decisions、
   Test Strategy、Verification Commands、Core Dependencies，控制为一个短文件；集中列出路径依赖决策和
   拟新增 Material dependency。
3. `task-boundary` / `remediation` 只处理架构、公共 API、持久化、权限/安全、并发、部署、
   不可逆数据或其它 Material Engineering Decision；局部实现决策留给 Implementation。
4. 仅将受影响 concern 写入 `.sdlc/design/` 或 ADR；可覆盖 Architecture、Module、Data/API/Protocol、
    Transaction/Concurrency/Error、Security、Observability、Deployment/Migration/Rollback、
    Experiment 和 Testing 路由到适用的模块文档。
5. 不为不适用领域创建文件或 `not_applicable` 记录。
6. 对非 Requirement 明示的 Security、Reliability、Compatibility、Performance 控制，先按
   `threat/failure -> attacker capability -> reachability/trigger -> existing controls -> credible residual
   path -> impact -> minimal sufficient mitigation -> complexity/disclosure` 评估。风险理论存在不等于
   可以把控制纳入 Design；既有控制已关闭路径或剩余路径未证明时，记录为 risk/advisory，不创建
   Material Control、Task 或独立机制。
7. 不得仅为该类风险新增模块、抽象层、后台机制、持久状态、配置、Runtime dependency、兼容层或
   底层替换。若最小可行缓解仍具有 Material complexity，冻结候选必须披露最小已知实现、工程影响、
   替代方案及其已证明的剩余风险，并等待所需 Owner approval；不得把复杂缓解作为局部实现决策。
8. 为重要决策准备 `.sdlc/decisions/ADR-xxx.md`，记录 Options、Decision、Rationale、
   Consequences 和 References。
9. 技术上可逆但会在当前规划窗口被大量代码放大迁移成本的 Framework、项目布局、模块边界、
   Data Access、API/Transport、Migration、测试基础设施和核心依赖策略视为 Foundation Decision，
   在首次实现前一次集中确认。Foundation/Material dependency 必须列出 package、version/range、
   用途、影响和替代方案并等待用户确认；低影响 dev/test dependency 与可逆开发工具按 L1 Assumption
   处理，除非项目规则要求 `dependency_policy: confirm_all`。
10. 维护 Requirement → Design → ADR 的追踪关系，返回待审冻结目标身份。
11. 首次 Foundation 形成时，返回 `signals.design_kind: foundation`，并省略 `architecture_change`
   （或明确为 `false`）；只有改写已接受/Frozen Design 或 accepted ADR 时才置
   `architecture_change: true` 并进入 DCR。
12. Technical Design Review 为 `REWORK` 时，只修复其 Finding 与受影响范围；生成新目标身份，
   使旧 Review/Evidence stale，再交给独立 Reviewer。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不设置 `status: FROZEN`，不通过 Gate。
- 不创建业务实现或测试代码；POC 必须有独立 Scope 和退出条件。
- 发现已冻结设计冲突时，创建/建议 DCR，而不是直接改写冻结设计。

遵循 [工作流协议](../sdlc-orchestrator/references/workflow-protocol.md)和
[变化控制协议](../sdlc-orchestrator/references/change-control.md)，并按
[Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)
生成和复检最终工件表面。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。`architecture_change` 只表示变更
已接受/Frozen Architecture；首次 Foundation 或首份未冻结 Design 不是 change。`next_action_hint` 非权威，
通常指向 `technical-design-review`。
