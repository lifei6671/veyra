---
name: qa-review
description: "Design QA cases, independently review a frozen case set, or verify an acceptance target against approved cases using mutually exclusive modes. Use for QA case design/review, QA entry assessment, regression, or experiment verification; do not use to silently fix product code or approve release."
---

# QA Review

设计可执行 QA Case、独立审查冻结 Case 集，或依据已批准 Case 验证当前验收目标。每次执行
必须在 `case-design`、`case-review`、`verification` 三种模式中选择一种，不得在同一执行
中修改 Case 后自行批准。QA 角色报告 Finding 与回归结果，不修业务代码再自行宣布通过。

## Context Contract

### Required Context

- 所有模式：当前 Acceptance target、Task、风险、适用 Experiment 与明确模式；
- `case-design`：Task/Scope、Test Strategy、环境能力，以及存在时的适用 Frozen Design；不要求
  既有 Case 或 Show Case；
- `case-review`：冻结 Test Case target identity、审查标准和 Producer 身份；
- `verification`：已批准 Case、实现 target identity、delivery-owned Diff 摘要，以及
  Build/Test/Delivery Review/Show Case Evidence。

### Optional Context

- 存在时的 Story、Relevant Design/ADR、接口契约和风险清单；
- Scope 内实现或测试源码，用于定位具体失败；
- 历史关联 Bug、回归范围和兼容性基线。

### Forbidden Default Context

- Entire Repo、全部 Epic/Story/Task/ADR/Evidence；
- 与当前验收目标无关的历史 QA 日志；
- 生产凭据、真实用户敏感数据和未授权生产操作。

## `case-design` 模式：Producer

1. 为每个关键 Acceptance 设计 Functional/Boundary/Failure/Permission/Integration/
   Regression/Security/Performance/Compatibility/Experiment/Observability 中适用的 Case。
2. 每个 Case 写明前置条件、步骤、预期结果、环境与所需 Evidence。
3. 只写 QA Case 工件，不给 `review_result`。`critical` QA 冻结后交给独立 `case-review`；
   `standard` 仅在风险或项目规则要求独立 Case Review 时交给该模式；`light` QA 直接交给
   独立 `verification`，不为流程补造 Case Reviewer。

## `case-review` 模式：独立只读 Reviewer

1. 绑定冻结 Case target identity 与 Producer；缺失时返回 `BLOCKED`。
2. 检查 Acceptance 覆盖、边界/失败路径、环境可执行性、Experiment/Exposure 和 Evidence
   需求；首次审查不修改 Case。
3. Reviewer 不得是该 Case 集 Producer；输出 P0–P3、Coverage、Remaining Risks 和
   `review_result`。

## `verification` 模式：独立执行与审查

1. QA Entry 按级别评估：`light` 需要 Case、Implementation/Dependency、适用验证和 Delivery Review；
   `standard` 额外需要独立 Verification；`critical` 还需要独立 QA Case Review 与 RD Show Case。
   缺少当前级别必需项不能用“代码完成”替代。
2. 在授权的非生产环境执行批准的 Case，记录真实 target identity、命令/方法、结果和有限诊断。
3. 失败以关联 Acceptance target / Task 的 `issues` 返回；修复必须回到 Implementation Producer，再由 QA 回归。
4. 首次审查不修改业务实现、Design、Task、Acceptance 或 Test Case；当前实现变化使相关
   QA Evidence stale。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不批准 QA Entry、QA Gate 或 Release Gate。
- `case-design` 可生成 QA Case；Reviewer 模式只能在独立输出路径写 Review/Evidence/报告，
  不得修改被审 Case 或业务目标。
- 不访问生产、不伪造无法运行的 Case；分别记录 `NOT_RUN` 或 `UNAVAILABLE`。
- Test Case 变化会使 QA Case Review、受影响 QA Result 和 Release Readiness Evidence `STALE`，
  必须重新审查再验证。

遵循 [Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。`case-review` 和 `verification` 还必须返回
`review_mode`、`reviewed_target_identity`、`coverage`、`remaining_risks` 和
`review_result`；`review_result` 使用 `PASS`、`PASS_WITH_CONDITIONS` 或 `REWORK`，
环境不可用时为 `null`。`case-design` 的 Reviewer 字段为 `null`/空集合且不得批准；
`issues` 按 P0–P3 分级；`next_action_hint` 非权威，
QA 通过不自动授权发布。
