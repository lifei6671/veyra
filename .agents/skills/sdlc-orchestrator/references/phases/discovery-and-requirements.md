# Discovery and Requirements

## Discovery

`ingest` 路径不默认进入 Discovery 或 Formal Requirement Review，但首次 ingest 必须先完整读取
Requirement Source 并执行轻量 Ingest Readiness Pass，不能直接基于局部片段建立 Anchor。
`author` 路径的目标是形成“足够执行”的 Requirement Baseline，不是消灭所有未知或开始编码。

输入：用户目标、已知约束、相关产品/仓库事实。

输出：

- Anchor：用户问题、业务目标、Must-have、范围、非目标、关键约束与验收引用；
- 仅真正影响产品语义的 Open Questions；
- 明示的可逆 Assumption、风险和必要 POC/Spike 的退出条件。

Discovery 不拥有 Gate，也不得创建或迁移 Gate。它只返回一个内部 `discovery_result`：

- `RESOLVED`：把已解决事实带回 Requirement / Anchor；
- `NEEDS_POC`：执行有退出条件的 POC，只验证该未知，不得悄悄变成生产实现；
- `NOT_FEASIBLE`：报告约束和不可行原因；
- `NEEDS_PRODUCT_DECISION`：把无法推断的 Material Product Uncertainty 集中交给用户。

Discovery 结果不是审批、失败 Gate 或等待 Gate 的替代物；后续路由只由该结果和实际风险边界决定。

## Ingest Readiness Pass

首次 ingest 内部检查只做四件事：完整读取 Requirement Source；提取 Goal、Scope、Non-goals 和
Acceptance；识别会改变产品语义或验收的矛盾/缺口；返回 `READY | NEEDS_CLARIFICATION`。
大文档可以分块读取，但必须覆盖全部需求章节后才能建立 Anchor、判断工程上下文或拆 Task。

`NEEDS_CLARIFICATION` 只包含无法从需求和仓库事实推断、会实质改变产品行为或验收的问题，并在
一轮中集中询问。分页默认值等可由 established 仓库约定推断的事项记录 Assumption 后继续。
它不是会话瞬时结论：必须使用既有顶层 `blocked` 记录
`origin_phase: PROJECT_INIT`、`owner: user`、`reason: material_requirement_clarification`、问题
`scope` 与 `unlock_condition`，并在 HANDOFF 写入 source-bound derived Intake Snapshot：source、identity、
readiness、已解析的 Goal/Scope/Non-goals/Constraints/Acceptance refs 摘要与 open questions。用户回答后，
snapshot identity 匹配时合并回答、清空 `blocked` 并从 `origin_phase` 恢复，不得冗余完整读取；snapshot
缺失或 identity 不匹配时，重新完整读取 canonical Requirement Source 是正确 fallback，不得丢失已完成的
Readiness 工作或把派生 Snapshot 当作事实源。

## Requirement Review

仅在 `author` 或用户明确要求评审既有需求时使用 Requirement Review。不要为了流程强制创建
Story；Requirement Baseline 至少应回答：

- 业务 Goal 和 User Story；
- 对产品语义有影响的 User Flow 与 Edge Cases；
- Scope / Non-goals；
- 可观察、可验收的 Acceptance；
- Dependencies、Risks；
- Metrics 或 Experiment（仅适用时）。

Acceptance 优先使用行为形式：

```text
Given <precondition>
When <event>
Then <observable result>
And <side effect/evidence>
```

不要写“优化体验”“重构模块”“支持管理”等无法独立验收的句子。

## Experiment 是一等需求

`experiment.enabled = true` 时 Requirement 或 Current Task 必须用 `experiment.ref` 引用
`.sdlc/experiment/EXP-xxx.yaml`，该 Experiment Artifact 必须定义：

- hypothesis；
- control / treatment；
- assignment key 与 allocation；
- exposure event；
- primary / secondary / guardrail metrics；
- success/failure criteria；
- kill switch 与 fallback；
- data collection owner。

PM 负责 Hypothesis、Metrics、Success Criteria 和 Traffic Strategy；RD 负责 Assignment、
Feature Flag、Control/Treatment、Exposure、Tracking、Fallback 和 Kill Switch；QA 负责
Control/Treatment、分流、Exposure、埋点和 Kill Switch 验证。Allocation 总和必须为 100。

实验实际启用时，缺 Exposure、Guardrail、Kill Switch 或 Data Collection 才是 Blocker；普通
功能不能因尚未设计实验而阻塞。

## Context 契约

默认加载 Anchor、Requirement Source、当前滚动 Task 与项目约束。不要读取全部历史 Task、QA
Evidence、ADR 或源码。只有为了验证现有能力或约束时，定向读取相关仓库工件。

## 输出

`refinement` Producer 返回更新的 Baseline/Anchor 输入、未决问题、Risk、Evidence 和非权威
建议；仅用户要求独立需求评审或风险确有必要时，再交给未参与生产的 `formal-review` Reviewer。
Reviewer 首次只读并返回目标身份、Coverage、Remaining Risks 与 `review_result`，不得修订后
自行批准。Specialist 不得自行批准 Gate；`author` 中改变产品语义的 Baseline 等待用户明确确认。
