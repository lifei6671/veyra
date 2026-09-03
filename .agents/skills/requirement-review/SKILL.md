---
name: requirement-review
description: "Turn a new product idea into an executable requirement baseline, or independently review a requirement when the user explicitly requests review. Use for author-mode requirement work and material product ambiguity; do not use to re-review an existing requirement the user already asked to implement, nor for architecture design, implementation, QA execution, or release approval."
---

# Requirement Review

把新产品想法转成可执行的 Requirement Baseline，或独立审查用户明确要求审查的冻结需求
目标。`ingest` 路径中，用户指定“按此需求开发”的文档是 Baseline，不默认重新 refinement 或
formal-review。每次执行必须在 `refinement` Producer 与 `formal-review` Reviewer 中二选一，不得在
同一执行中修改目标后自行审查。Requirement Gate 是否通过只由 Orchestrator 结合 Evidence
与所需 Human Gate 决定。

## Context Contract

### Required Context

- `.sdlc/state.yaml` 中当前 mode、phase、focus 与 profile；
- `.sdlc/memory/HANDOFF.md` 和 `.sdlc/tasks.yaml`；
- `mode` 与 Project Constraints；`ingest` 需要 Anchor/Requirement Source，`author` 需要用户输入
  或当前 Baseline 草稿；
- 当前 Baseline 或用户明确指定的需求目标；首次 `author` 可由用户输入形成。
- `formal-review` 还必须有冻结的 reviewed target identity、审查标准和 Producer 身份。

### Optional Context

- 与当前需求直接相关的仓库能力、竞品事实或现有接口；
- 相关历史决策、用户研究和指标基线；
- Experiment 约束与数据采集能力。

### Forbidden Default Context

- 全部历史 Epic、Story、Task、ADR、QA 或 Evidence；
- 与当前需求无关的源码、完整仓库或生产数据；
- 未经授权的外部系统与敏感数据。

## `refinement` 模式：Producer

1. 在 `author` 路径提取目标、Must-have、Scope、Non-goals、依赖、风险和真正阻塞的问题；
   最多一轮集中澄清，不为低风险细节收集问题清单。
2. 将 Acceptance 写成可观察的行为与副作用；不得用“优化”“支持”等模糊表述代替验收。
3. 建立 Goal → Anchor → Acceptance → Metric（若适用）的追踪关系；不要为追踪而强制创建
   Epic 或 Story。
4. 若 `experiment.enabled = true`，验证 Hypothesis、Control/Treatment、Assignment、
   Exposure、Primary/Secondary/Guardrail Metrics、判定标准、Kill Switch、Fallback 和
   Data Owner 均存在。
5. `author` 模式可把 Candidate Baseline 暂存于唯一 `.sdlc/REQUIREMENT.md`；只有用户确认后
   Orchestrator 才能把它绑定为 `source.requirement` / `source.identity` 并建立 Anchor。`ingest`
   只更新用户明确授权的 Existing Requirement Source。回读目标并把 candidate identity 与 Anchor
   输入返回给 Orchestrator；不修改 `state.yaml`，也不要为不适用领域创建文件。发现 Material Scope
   或架构变化时停止并标记。
6. 返回问题、Blocker、Evidence 和非权威 Gate 建议，不返回正式 `review_result`。
7. 完成后冻结目标身份；仅用户明确要求需求评审或风险使独立审查有实际价值时，才交给未参与
   该目标生产的 `formal-review` 执行者。

## `formal-review` 模式：独立只读

1. 确认 `reviewed_target_identity`、Producer 身份与目标冻结状态；缺失时返回 `BLOCKED`。
2. 检查 Goal → Anchor → User Flow / Edge Cases（适用时）→ Acceptance → Metric（适用时）的
   完整性、可观察性、边界、依赖、风险和歧义。Epic/Story 存在时验证其追踪，不存在不能阻塞。
3. 启用 Experiment 时检查完整 EXP Artifact 与 Requirement 的追踪关系。
4. 首次审查只读，不修改 Epic、Story、Requirement 或 Experiment；以 P0–P3 返回问题、
   Coverage、Remaining Risks 和正式 `review_result`。
5. Reviewer 不得是该冻结目标的 Producer；修订后目标身份变化，旧 Review 立即 stale。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不把 Story、Gate 或 Phase 标为完成。
- 不设计架构、不创建实施代码、不执行 QA 或发布。
- 对 `standard` / `critical` Profile，不把用户的“继续”解释为 Requirement Human Gate 批准。

遵循 [Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)
及 [变化控制协议](../sdlc-orchestrator/references/change-control.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。`formal-review` 还必须返回 `review_mode`、
`reviewed_target_identity`、`coverage`、`remaining_risks` 和 `review_result`；
`review_result` 使用 `PASS`、`PASS_WITH_CONDITIONS` 或 `REWORK`。`refinement` 模式的这些
Reviewer 字段为 `null`/空集合，且不得给出 Gate 审查结论。`outcome` 仅描述本次专业工作
是否完成；
`next_action_hint` 只是提示，Orchestrator 必须独立决定下一步。
