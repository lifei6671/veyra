---
name: code-delivery-review
description: "Independently review one frozen SDLC Task delivery unit against its acceptance, frozen design, complete delivery-owned inventory, verification evidence, coverage, and freshness. Use only after implementation freezes a candidate target; do not use to implement the task, control lifecycle state, or review arbitrary branches, commits, pull requests, or patches."
---

# Code Delivery Review

对一个 SDLC Task 的冻结交付单元执行独立、只读、缺陷优先审查。本 Skill 吸收完整
Delivery Unit、Coverage Gate、Target Freshness、Reviewer Capability Ladder 和轻量 Review
Orchestration Protocol，但只承担
**Reviewer** 职责：Implementation 负责生产和修复，Orchestrator 独占 Gate 与状态迁移。
`Review Planner`、Reviewer Lanes 与 `Judge Pass` 都是本 Skill 内部角色，不是新 Skill、Runtime
或生命周期状态。

```text
Delivery Gate candidate =
  Implementation complete
  AND Required verification satisfied
  AND Delivery unit frozen
  AND Review coverage complete
  AND Reviewed state fresh
  AND No P0/P1 findings
```

这个公式只是 Reviewer 的判定输入，不是生命周期状态机。最终是否通过 Delivery Gate，
仍由 Orchestrator 结合 Human Gate 和其他 Evidence 独立判断。

## Context Contract

### Required Context

- 从 `tasks.yaml.task_ref` 解析的当前 Task Markdown：本次需求、Scope、每项子功能
  Acceptance/Verification、Task 独立验收和 Risk；
- Delivery Unit 起始身份、delivery-owned inventory、provenance、exclusions 和 ambiguities；
- 冻结的 target identity、实际 Build/Test/Lint/Scan Evidence 和 coverage manifest。

### Conditional Required Context

- Task 实际引用的 Frozen Design；
- Task 引用的 ADR，或本次交付变更的是已冻结 Design / accepted ADR 时的 DCR。

这些工件只在 Task 或受影响 concern 已引用时成为必需项。普通实现 Task 不得因缺少
Design、ADR 或 DCR 被阻塞，更不得为了审查补造工件。

### Optional Context

- Scope 内调用方/被调用方、相关测试及 material neighboring context；
- 仓库适用的编码、安全、验证规则和机器可读 Contract；
- 已冻结 checkpoint、上轮 Finding、修复 Diff 与受影响验证。

### Forbidden Default Context

- Entire Repo、Entire PRD、全部 Story/Task/ADR/Evidence；
- 与当前 delivery-owned 范围无关的用户改动、并发改动或历史缺陷；
- Producer 的自我评价、未执行验证、未经冻结目标；
- 任意 Branch、Commit、Commit Range、Pull Request 或外部 Patch。

## 先验证交付包

1. 按 [Delivery Unit 与 Freshness](references/delivery-unit.md) 重建完整 inventory。
2. 对账 staged、unstaged、untracked、deleted、task-time commits、generated 和
   verification-created 内容；保留来源，不把用户贡献冒充 Agent 产出。
3. 确认 target identity 能识别内容而不只是文件名；身份、Scope 或 material ownership
   不清时返回 `outcome: BLOCKED`，不得猜测。
4. 将实际 Verification 与当前目标绑定。测试通过不代替 Review，Review 也不证明测试运行。

## 选择审查策略

读取 [审查协议](references/review-protocol.md)：

- 先由 `Review Planner` 从 Tier、Change Signals、Diff 分类和 Previous Findings 生成适用
  Review Plan；Tier 决定深度和投入，Change Signals 决定实际 Lanes，不能把 Deep 等同于
  全量开启所有 Lane。
- 小型/中型变更使用 `FULL_SCOPE`；大型或多模块变更使用
  `PARTITIONED_PLUS_INTEGRATION`，每个分区都必须有结果，最后检查跨模块边界。
- 风险深度使用 `TIER_1_FOCUSED`、`TIER_2_STANDARD`、`TIER_3_DEEP`；风险深度不能
  自动扩大权限或取消 Human Gate。
- Diff 必须分类为 `PRIMARY`、`CONTEXT_ONLY`、`GENERATED` 或 `EXCLUDED`。分类只调整读取与
  Finding 深度，不得从 inventory 静默删除 material 文件；Migration 即使由工具生成也按其
  运行语义审查。
- Reviewer capability 顺序为 `NATIVE_ISOLATED -> CHILD_AGENT -> SELF_REVIEW`。
  在本 SDLC 中，正式 Delivery Review 必须保持 Producer/Reviewer 分离，因此
  `SELF_REVIEW` 默认只能形成非权威诊断，不能让 Delivery Gate 通过；仅当 Orchestrator 已记录
  适用的 `prototype` 或 `standard/best_effort_self_review` 项目级政策时，才可作为受限 Delivery
  Evidence，且必须披露其非独立性。
- Checkpoint 可减少后续重复阅读，但不能代替最终完整 Coverage、Integration 和 Freshness。

Planner 为所选 Lane 构造临时 Shared Review Context Packet，包含目标、Acceptance、身份、
Change/Diff Map、相关 Design/ADR、Verification 摘要和 Previous Findings。Packet 只存在于本次
调度上下文，不写 `.sdlc`、不形成 Review Cache，也不要求新增默认文件。

适用 Lanes 可在宿主支持时并发，也可由同一独立 Reviewer 串行执行。Lane 并发或 Lane 之间
使用不同 Agent 都不是正式审查独立性的前提；独立性仍只要求 Implementation Producer 与
Code Delivery Reviewer 分离。

## 初次审查：只读

1. 执行 Review Plan，只把 Shared Review Context、分配路径和 Lane-specific 规则交给适用
   Lane。先读 [Review Lane Index](references/review/INDEX.md)，随后只加载
   [Universal NOT-Flag](references/review/universal-not-flag.md) 和被选中的 Lane 文件；未选
   Lane 不进入 Context。按 [Language Profile Routing](languages/INDEX.md) 只加载变更涉及的
   语言；项目规则和冻结 Design 始终优先，不得为了审查新增工具、依赖或 Style Policy。
2. 只审 Review Plan 中 `selected_lanes` 负责的 concern，不得自行扩展到未选 Lane。Lane 发现
   新的 material signal 时，返回 Review Planner 扩展 Review Plan、加载新增 Lane 并刷新 Coverage，
   再继续对应 concern 的审查。
3. 汇总 Lane 候选后执行 `Judge Pass`。按
   [Finding 与结果映射](references/result-adapter.md) 过滤非问题、合并同根因、挑战证据、
   归一严重度并对账 Previous Findings。每项最终 Finding 必须给出位置、触发路径、影响、
   证据和最小修复方向；没有可信缺陷时允许 `No findings.`。
4. 首次审查不修改实现、测试、Task、Design 或被审 Evidence。Producer 修复后必须重跑
   受影响验证、生成新 target identity，再按需加载
   [Incremental Re-review](references/review/incremental-rereview.md)。Planner 使用 Previous
   Findings 与修复 Diff 只调度受影响 Lane、分区和 Integration 边界；广泛修复、Contract/Scope
   变化、Coverage gap 或 material interaction 变化时扩大为必要的完整复审。
5. Orchestrator 默认最多调度三轮自动修复闭环；Reviewer 只返回 Finding，不自行修复。
   修复需要扩大 Scope、改变公共/持久化/运行/运维契约、重要运行依赖、权限安全或架构时
   立即停止并交回 Change Control，不得为了通过审查扩大范围。
6. 覆盖每项子功能和 Task 级整体 Acceptance，但 Review Result 只提供独立审查 Evidence；
   Reviewer 不得自行把子功能标为 `ACCEPTED`、把 Task Acceptance 标为 `PASSED` 或把 Task
   迁移 `DONE`，这些由 Orchestrator 回读当前 Evidence 后决定。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不把 Task 设为 Done，不批准 Delivery Gate。
- 可在 Orchestrator 明确指定独立输出路径时写 Review 记录，但不得修改被审对象。
- 不审查或修复无关历史问题，不提交、推送、发布或写外部系统。
- 用户明确豁免 Review 时只报告豁免请求和仍适用 Gate；Reviewer 不伪造 `PASS`。

遵循 [Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。Reviewer 还必须返回 `review_mode`、`reviewed_target_identity`、
`coverage`、`remaining_risks` 和 `review_result`。

最终 Finding 按 [Finding 与结果映射](references/result-adapter.md) 返回稳定的 `finding_id`、
`fingerprint`、本轮 `status` 和当前 `target_identity`；这些字段支持复审对账，不是生命周期状态。

`coverage` 至少包含 `strategy`、`status`、`applicable_lanes`、`lanes`、`partitions`、
`integration_result` 和 `freshness`；
`review_mode` 使用 `NATIVE_ISOLATED`、`CHILD_AGENT`、`SELF_REVIEW` 或 `MIXED`。
`review_result` 只使用 `PASS`、`PASS_WITH_CONDITIONS` 或 `REWORK`，环境/能力不可用时为
`null`。`next_action_hint` 非权威，Orchestrator 独立判断修复、复审、豁免或 Gate 迁移。
