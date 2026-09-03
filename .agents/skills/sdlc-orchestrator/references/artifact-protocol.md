# Artifact Protocol

## 默认工作集与懒创建

```text
.sdlc/
├── state.yaml
├── tasks.yaml
├── tasks/
│   └── TASK-xxx.md
└── memory/HANDOFF.md
```

前三个治理文件是初始化时的最小工作集；`tasks/` 与当前窗口的 Markdown Task 只在首次
Task Breakdown 时创建。`author` 的 Baseline Candidate 实际形成时，再懒创建
`.sdlc/REQUIREMENT.md`；只有被接受并绑定 Source identity 后它才是业务事实源。`ingest` 始终引用已有需求文档。根据实际需要才创建
`decisions/ADR-xxx.md`、`changes/DCR-xxx.md`、`design/foundation.md`、其它模块化 Design、`qa/`、
`release/`、`experiment/` 或 `evidence/`。不要创建空目录、空模板或 `NOT_APPLICABLE` 文件。

不要维护第二份正式 Decision 日志。需要 ADR 时，`.sdlc/decisions/ADR-xxx.md` 是工程决策
事实源；Memory 只引用 ADR。

## 权威关系

- `state.yaml`：Anchor、流程位置、Focus、Gate、Blocker 和 `next`。
- `tasks.yaml`：里程碑与滚动窗口索引；当前 Task 只保存 `id` 与 `task_ref`，后续 Task 只保存
  轻量 planning stub。两者都不复制当前 Task 的完整 Scope、子功能或 Verification。
- `tasks/TASK-xxx.md`：每个已展开 Task 的唯一事实源。YAML front matter 保存 Task 动态元数据，
  Markdown 正文保存本次需求、Scope、子功能契约、Task 独立验收和 Verification。
- Requirement Source：`ingest` 使用用户已有文档；`author` 可使用实际形成时创建的
  `.sdlc/REQUIREMENT.md` Candidate。只有接受的 Source 才回读并记录到 State 当前 identity。
- Design、ADR / DCR、QA、Release、Experiment 与 Evidence：仅在实际适用时创建，各自是该
  concern 的事实源。
- `memory/HANDOFF.md`：可重建的恢复摘要，不是流程事实源。仅在 ingest 的 Material clarification
  Blocker 存在时，可临时保存 source-bound derived Intake Snapshot：`source`、`source_identity`、
  `readiness`、已解析的 Goal/Scope/Non-goals/Constraints/Acceptance refs 摘要和 `open_questions`。
  Snapshot 只能在 identity 匹配时与用户回答合并；它缺失或不匹配时允许重新完整读取 canonical
  Requirement Source。Anchor 建立后删除该 Snapshot，且它永远不能覆盖 Requirement Source 或 Anchor。

Delivery Unit 不创建新的事实源或缓存目录。跨会话所需的原始 baseline、delivery-owned
inventory/provenance、checkpoint、验证摘要和未解决 Finding，由 Orchestrator 校验后压缩到
`memory/HANDOFF.md`；只有观察需要跨 Task、长期审计或被 Completed Task 的 `evidence_refs` 引用时，
才追加到 `.sdlc/evidence/`。详细内容仍由 `tasks.yaml`、Git/宿主编辑记录和不可变 Evidence 提供，
摘要不能覆盖它们。

未解决 Finding 少时，HANDOFF 从当前 Review Result/Evidence 派生复制最小
`finding_id/fingerprint/severity/status/target_identity/evidence_ref`；超过 300–800 token 的 L0
预算时，把 compact review Evidence 持久化到 `.sdlc/evidence/TASK-xxx/`，HANDOFF 只保存引用。
写 HANDOFF 前必须确认每个 `evidence_ref` 可解析到当前 Gate/Task 的有界 compact record、宿主
持久 Review Result 或 `.sdlc/evidence/`；若当前结果在会话结束后不可解析，则无论 Finding 数量
多少都先持久化 compact review Evidence。HANDOFF 始终不是 Finding 的独立事实源。

## 最终表面与事实边界

- **会话级弃案（session-only rejected alternatives）**：只在讨论或草稿中出现、未进入已接受
  canonical artifact，且对后续范围、兼容、迁移、安全、审计或诊断没有持续价值的方案与
  措辞。它们只用于控制本轮生成，不得
  通过直接提及、同义转述或括号说明进入最终表面。
- **canonical / audit facts**：已接受或后续工作必须知道的事实，包括 `non_goals`、
  `scope.deny`、ADR Alternatives、ADR/DCR、Finding、真实删除或迁移、兼容性、安全与诊断
  事实、Blocker、Evidence，以及交付范围外的用户已有改动及其 provenance。这些事实必须按
  对应事实源保留，不得因为采用否定表述而被静默删除或改写。

最终表面包括正文、标题、文件名与路径、标签和 metadata、代码注释与测试名称、commit、
PR、报告、`memory/HANDOFF.md` 和最终回复。生成这些表面时，以已接受的
canonical state 和实际交付结果为起点；不得让会话级弃案成为产物身份或叙事中心，也不得把
交付范围外的用户已有改动吸收到本次交付叙事。该边界不新增字段或 Schema；无法确定归类时，
按事实源、后续依赖和审计需求判断，而不是按句子是否包含否定词判断。

## `tasks.yaml`、Markdown Task 与滚动窗口

`tasks.yaml` 维护粗粒度里程碑和一个 JIT 滚动窗口。只为 **当前** Task 创建 Markdown 文件；后续
2–5 个 Task 保留为轻量 stub，直到被提升为当前 Task。当前项只包含稳定 `id` 与可解析 `task_ref`；
stub 只包含 `id`、`milestone_ref`、`objective`、`dependencies`、`risk` 与 `acceptance_summary`，不得提前
拥有完整 Scope、SF、Task Acceptance 或 Verification。旧 V1/V2 转换而来的 stub 可额外保存
`migration_ref`，只用于保留既有 Markdown 的约束输入。Task 文件路径固定为 `.sdlc/tasks/TASK-xxx.md`。当前
Task `DONE` 后，Orchestrator 只记录完成、选择下一个 stub 并路由 `task-breakdown: materialize-current`；
Planning Producer 原子地把 stub 替换为 current `id/task_ref` 并创建完整 Markdown。Orchestrator 回读
该产物并通过 Task Readiness Check 后，才更新 `focus.task` 和 `READY`；不得编写 Task 正文。完成的 Task
不保留完整窗口定义：先把 20–50 token
的摘要写入所属 milestone，确认 Git、ADR/DCR 和长期 Evidence 引用可定位后，再从 `window` 移除
引用；已被 Evidence/Decision 引用的 Task Markdown 不得删除。

```yaml
version: 3
milestones:
  - id: M1
    objective: Replace with a goal-aligned milestone.
    status: PLANNED
    completed: # only after a task in this milestone is DONE
      - id: TASK-001
        result: Parser bootstrap delivered.
        target_identity: git:abc123
        decision_refs: []
        evidence_refs: []
    follow_ups: # only after an authorized Owner adopts a review follow-up
      - id: FU-001
        finding_ref: FINDING-017
        summary: Add the remaining boundary assertion.
        status: OPEN
        evidence_ref: .sdlc/evidence/TASK-017/review.yaml
window:
  - id: TASK-001
    task_ref: .sdlc/tasks/TASK-001.md
  - id: TASK-002
    milestone_ref: M1
    objective: Add the bounded parser integration.
    dependencies: [TASK-001]
    risk: low
    acceptance_summary: The parser result is available through the existing boundary.
```

每个 Markdown Task 必须包含：

1. YAML front matter：`id`、`milestone_ref`、`dependencies`、`risk`、`status`，以及存在时的
   `design_refs`、`decision_refs`、`approval_refs`；
2. `本次需求`：明确本次要实现的 Objective、Requirement/Anchor 引用和不做什么；
3. `Scope`：明确 `allow` 与必要的 `deny`；
4. 一个或多个 `SF-xxx` 子功能；每项分别保存非空需求、可观察 Acceptance、可执行或客观可观察
   Verification、`implementation_status`、`acceptance_status` 和按需 `evidence_refs`；
5. `Task 独立验收`：定义跨子功能的整体验收条件、验证方法、`acceptance_status` 与按需
   `evidence_refs`。所有子功能分别通过仍不能替代 Task 级集成验收。

子功能实现状态使用 `DRAFT | READY | IN_PROGRESS | IMPLEMENTED | VERIFYING | ACCEPTED | BLOCKED |
CANCELLED`；子功能和 Task 验收状态使用 `PENDING | PASSED | FAILED | STALE`。这些状态只由
Orchestrator 在校验 Specialist Result 和 Evidence 后更新，Implementation 不得自批。

Task Readiness Check 是每次进入 Implementation 前的自动契约检查，不是 Human Gate。它只校验
current `task_ref` 指向的完整 Markdown，不要求 future stub 预先展开。Orchestrator 必须回读
`tasks.yaml` 当前引用和完整 Markdown Task，并在一个检查中确认：

- `task_ref` 存在且位于 `.sdlc/tasks/`，索引 ID、文件名和 front matter ID 一致；
- `milestone_ref` 能解析到 `tasks.yaml` 中的现有 milestone；front matter 字段和值合法；
- `本次需求` 非空，明确本次交付行为、Requirement/Anchor 引用和非目标；
- `scope.allow` 非空，必要的 `scope.deny` 已声明；至少有一个子功能；
- 每项子功能的需求非空、Acceptance 可观察、Verification 可执行或可客观观察；
- Task 独立验收具有可观察 Acceptance 和可执行或可客观观察的 Verification；
- Dependencies 存在、无环，所有前序 Task 已独立验收并 `DONE`；Risk 合法；
- 不存在 `TBD`、占位命令、空标题或等待中的需求变更/第三方依赖确认。

任何一项不满足时保持 `DRAFT`，路由 `task-breakdown` 修复；只有缺失内容源于真实产品语义
不确定性时才集中请求需求澄清。

读取旧 `tasks.yaml version: 1` 且 `window` 内联完整 Task 时，先逐项无损迁移：为 focus Task 创建
对应 Markdown；每个 future inline Task 也物化为只读迁移来源 `.sdlc/tasks/TASK-xxx.md`，并在对应 JIT
stub 写入该路径的 `migration_ref`。它不是 current `task_ref`，不得实现或作为已展开 Task 使用；
`task-breakdown: materialize-current` 提升时必须回读它以保留 Scope/deny/SF Acceptance/Verification 输入，
再形成新的 current Task Contract。保留原
ID、`milestone_ref`、依赖、Risk 和足以追踪的 Objective/Acceptance 摘要。`version: 2` 的多个 `task_ref`
仍可恢复：focus 引用保持 current，后续引用写为 stub 的 `migration_ref`，在提升时回读以保留
Scope/deny/Acceptance 输入；不自动删除旧 Markdown，只有用户明确授权清理且不存在 Evidence/Decision
引用时才可删除。回读后将
索引升级为 `version: 3`。迁移不得把旧 `READY/IN_PROGRESS` 静默解释为已通过新的 Task Readiness
Check；补齐子功能与 Task 独立验收契约后才能继续实现。

## ID 与状态

- ID 稳定且不依赖 Conversation：`EPIC-001`、`STORY-001`、`TASK-001`、`ADR-001`、`DCR-001`、`EXP-001`。
- 不删除已引用的 Accepted ADR；变更时创建新 ADR 并记录 supersedes 关系。
- Task 只有在依赖已 `DONE` 且 Task Readiness Check 通过时才能 `READY`。
- 当前 Agent 只维护一个 `focus.task`。当前 Task 的独立验收未 `PASSED`、状态未 `DONE` 前，
  不得开始窗口中后续 Task 的实现。
- 一个 Task 内无前置依赖、无写冲突的子功能可以并行；只有当前 Task Markdown 明确记录子功能间
  依赖，且从各自实现计划涉及的路径/符号可证明 Scope 不重叠时才并行，信息不足时保持串行。
  不为 Task 增加 `owner` 或 `parallel` 字段。

## 最小字段

### Optional Epic / Story

`id`、`title`、`business_goal`、`scope`、`non_goals`、`success_metrics`、`stories`、`owner`、`status`。

### Current Task Index Entry

`id`、`task_ref`。

### Future Task Stub

`id`、`milestone_ref`、`objective`、`dependencies`、`risk`、`acceptance_summary`。Stub 是滚动规划提示，
不是可实现 Task Contract；不得含 `task_ref`、完整 Scope、SF、Task 独立验收或 Verification。可选
`migration_ref` 仅用于遗留 V1/V2 迁移：提升时必须回读该只读迁移来源，把仍适用的
Scope/deny/Acceptance/Verification 作为
输入重新形成 current Task Contract；它不是 future Task 的第二个正式事实源。

### Markdown Task

front matter：`id`、`milestone_ref`、`dependencies`、`risk`、`status`；`design_refs`、
`decision_refs`、`approval_refs` 按需存在。正文：`本次需求`、`Scope`、至少一个带独立状态与验收的
`SF-xxx`、`Task 独立验收`。Foundation/Material dependency 的批准引用必须精确关联 package、
version/range、用途和影响；L1 dev/test dependency 记录 Assumption 与验证，除非项目规则要求
`dependency_policy: confirm_all`。已接受 Requirement 的变更批准引用必须关联 Impact Analysis 与用户确认。

### Completed Task Summary

`id`、`result`、`target_identity`；`decision_refs` 和 `evidence_refs` 仅在存在长期审计价值时写入。
摘要用于 Requirement Change 的影响定位；详细历史仍由 Git、ADR/DCR 和 Evidence 承担。

### Milestone Follow-up

`follow_ups` 是 milestone 的按需字段；每项包含 `id`、`finding_ref`、`summary`、`status` 和
`evidence_ref`。它只在有权 Owner 明确采纳 Reviewer 的 `FOLLOW_UP` recommendation 后，由
Orchestrator 写入，
不自动进入 `window`，也不立即创建 Markdown Task。`status` 使用 `OPEN | PROMOTED | CLOSED`：提升为
Task 时原子写入 `status: PROMOTED` 与 `task_ref`，新 Task 同时保留 `finding_ref/evidence_ref`；关闭但
不转 Task 时写 `status: CLOSED` 与 `resolution_ref`。不得保留 OPEN Follow-up 与同源 active Task
双重待办，也不得删除已引用 Follow-up。为空时省略整个字段。

### Experiment

`id`、`story`、`hypothesis`、`control.name/allocation`、`treatment.name/allocation`、
`assignment.key`、`exposure.event`、`metrics.primary/secondary/guardrail`、
`success_criteria`、`failure_criteria`、`kill_switch.required/method/owner`、
`fallback`、`data_collection.owner`、`status`、`result`、`result_evidence`、`result_action`。

Allocation 必须和为 100；Assignment 稳定且可重现；Exposure 必须可观测；Primary 和
Guardrail 不得为空。`result` 为 `WIN/LOSS/NEUTRAL/INVALID` 时，`result_evidence`
必须引用当前实验目标身份。对应动作固定为：WIN 推广 treatment；LOSS 关闭/回滚；
NEUTRAL 请求产品决策；INVALID 重新设计实验。任何生产关停/回滚仍需 Human Gate。

### ADR

`Status`、`Context`、`Decision`、`Rationale`、`Alternatives`、`Consequences`、`Reconsider When`、`Supersedes/Superseded By`。

### DCR

`Problem`、`Current Frozen Design`、`Proposed Change`、`Reason`、`Affected Story/Task/Gates`、`Migration`、`Compatibility`、`Risk`、`Verification`、`Decision`。

### Evidence

每条 record 必须包含 `id`、`type`、`target`、`target_identity`、`produced_by_phase`、
`command_or_method`、`result`、`timestamp`、`summary`、`observer`。命令型 Evidence 还必须有
`exit_code`；`diagnostic_tail` 仅在失败诊断需要时存在，并保持有界。`target_identity` 不得只放
在可变文件顶层；record 写入后身份不可改，目标变化时追加新 record。

## 模板使用

`assets/project-template/` 物理上只包含初始化时复制的三个治理文件。
`assets/examples/` 是按需参考的可选工件，永远不能整体复制到项目中。复制默认模板后必须：

按需示例：[ADR](../assets/examples/ADR.example.md)、[DCR](../assets/examples/DCR.example.md)、
[Task](../assets/examples/TASK.example.md)、[Experiment](../assets/examples/EXPERIMENT.example.yaml)、
[QA](../assets/examples/QA.example.md)、[Release](../assets/examples/RELEASE.example.md)。

1. 根据入口设置 mode；`author` 形成 Candidate 时可创建 `.sdlc/REQUIREMENT.md`，用户接受后才绑定
   Source identity 与 Anchor；`ingest` 记录已有文档路径；
2. 仅在 concern 适用且需要可追溯性时创建工件；可选字段用 `null/none` 或删除，不得留下
   误导性示例值；
3. 校验所有相对路径和 ID 引用；
4. 先写并回读 Requirement Source，再更新 Anchor 和 `tasks.yaml`，最后写
   State/`memory/HANDOFF.md`；
5. 回读默认工作集，确认不存在半初始化状态。

## 防止重复与漂移

- Anchor 不复制 Requirement Source；只保存稳定意图和引用。
- 已展开 Task 使用短 Markdown，聚焦本次需求、Scope、子功能 Acceptance/Verify、Task 独立验收和引用；
  不复制 Requirement 或 Design 正文。
- Design 必须模块化，禁止让每个执行者默认读取一个数千行单文件。
- Decision 原因只在 ADR 保存一次。
- Evidence 成功只保存摘要；失败保存有限诊断。
- `memory/HANDOFF.md` 可以随时从事实源重建。
