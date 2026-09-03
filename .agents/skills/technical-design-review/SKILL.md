---
name: technical-design-review
description: "Independently review a frozen technical-design candidate for traceability, decision completeness, feasibility, risk coverage, and unauthorized changes. Use when a design needs a pre-freeze or re-freeze gate review; do not use to author the design or implement fixes on the first review pass."
---

# Technical Design Review

对 Technical Design 冻结候选执行独立、缺陷优先的只读审查。该 Skill 是 Reviewer，
不得兼任被审设计的 Producer；Review 结论不等于 Orchestrator Gate 或 Human Gate。

## Context Contract

### Required Context

- 当前 Requirement、Acceptance 和 Project Constraints；
- 待审 Design 目标身份与实际受影响的 Design Artifact；
- 受影响 concern 的审查标准与约束。

### Conditional Required Context

- `foundation` 目标：完整 Requirement Source、Repository Snapshot 与 Project Constraints；不要求 Current Task；
- 受影响 concern 已有 accepted ADR 时的该 ADR；
- 仅在本次审查修改 Frozen Design 或 accepted ADR 时需要的 DCR。

首次 Design 可以没有 ADR；DCR 不用于首次设计。不得因不适用的 ADR/DCR 缺失阻塞审查或
要求 Producer 额外创建工件。

### Optional Context

- 多文件设计存在时的 `.sdlc/design/INDEX.md`、当前架构的定向源码或符号证据；
- POC、容量、兼容性和 Experiment/Data Collection Evidence；
- 上一轮 Review Finding 与修复 Diff。

### Forbidden Default Context

- Entire Repo、Entire PRD、全部 Story/Task/ADR/Evidence；
- 与冻结候选无关的历史 Review；
- Producer 的推理过程或未经证实的完成声明。

## 初次审查：只读

1. 冻结并回报实际 reviewed target identity；目标不明确时返回 `BLOCKED`。
2. Foundation 是合法冻结目标；检查其路径依赖技术基线、拟新增第三方依赖和 Requirement 追踪，
   不因尚无 Current Task 阻塞。其它目标检查 Requirement → Design 的追踪性及 L1/L2 Decision 完整性；ADR/DCR 存在且适用时，
   再检查其引用和变更边界。
3. 检查与当前仓库和约束的兼容性，只审实际受影响 concern 的 Failure、Security、Migration、
   Rollback、Observability、Testing 或 Experiment 闭环；未受影响 concern 的缺失不构成 Finding。
4. 标记未授权的 Material Scope/Contract、持久数据、运行依赖、权限安全或架构变化。
5. 以 P0/P1/P2/P3 给出可定位、可执行、证据支持的 `issues`，并说明覆盖范围与残余风险。
6. 首次审查不修改 Design、ADR、源码或测试；Producer 修复后，对新目标重新审查受影响范围。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不设置 Design `FROZEN`，不批准任何 Gate。
- 可在 Orchestrator 明确指定独立输出路径时写 Review 记录，但不得修改被审对象。
- 不以测试或静态检查替代语义 Design Review。

遵循 [Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。Reviewer 还必须返回 `review_mode`、`reviewed_target_identity`、
`coverage`、`remaining_risks` 和 `review_result`。`outcome` 使用 `PASS`、
`FAIL`、`BLOCKED` 或 `UNAVAILABLE`；`review_result` 使用 `PASS`、
`PASS_WITH_CONDITIONS`、`REWORK`，环境不可用时为 `null`；`next_action_hint`
非权威，不得声明状态迁移。
