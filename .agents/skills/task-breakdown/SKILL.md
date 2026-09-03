---
name: task-breakdown
description: "Maintain a rolling Markdown Task plan with scoped subfeatures and separate Task acceptance. Use to plan the current task, or materialize an Orchestrator-selected future stub as the next current Task; do not use to change requirements or design, write implementation code, or approve the Planning Gate."
---

# Task Breakdown

把 Anchor 与适用的 Requirement/Design 拆成可独立交付、独立验收的滚动 Markdown Task 窗口。
一个 Task 可以包含多个可分别实现和验收的子功能，但仍必须定义 Task 级整体验收。该 Skill 是
Planning Producer，不是状态迁移器。

## Context Contract

### Required Context

- `.sdlc/state.yaml`、`.sdlc/memory/HANDOFF.md`、`.sdlc/tasks.yaml`；
- Anchor、当前 Requirement Source 与 Acceptance；
- 项目验证命令、目录边界和并行工作约束。

### Conditional Required Context

- 已存在的窗口 Task Markdown（若有）；
- Orchestrator 已选择、但尚未展开的 future stub（仅 `materialize-current` 模式）；
- 当前 Task 或 concern 实际适用的 Foundation、Design、ADR/DCR。

普通 Task 没有适用 Design 或 Decision 时可直接拆解；不得因为缺少这些按需工件阻塞 Planning，
也不得为了填满上下文而创建它们。

### Optional Context

- 相关源码结构、测试布局和现有模块依赖；
- QA Strategy、Release/Migration 约束；
- 影响工作量或顺序的风险与历史 Evidence。

### Forbidden Default Context

- 全部 Epic、Story、Task、Design、ADR 或 Evidence；
- 与当前 Task 无关的完整仓库实现；
- 未冻结的替代设计和未获批准的范围扩展。

## 执行

1. 在 `.sdlc/tasks.yaml` 保留粗粒度里程碑、当前 Task 的 `id/task_ref` 与随后 2–5 个 Task 的
   轻量 stub（`id/milestone_ref/objective/dependencies/risk/acceptance_summary`）。只创建或更新当前
   `.sdlc/tasks/TASK-xxx.md`；future stub 不得提前展开为完整 Task 文档。
2. 每个 Task 只包含一个清晰的独立交付 Objective，可以在其内部拆成多个 `SF-xxx` 子功能；
   Task 以 `milestone_ref` 绑定所属里程碑，并引用 Anchor、Requirement 和实际适用的 Design/ADR/DCR。
3. 为 Task 定义 `scope.allow` 和必要的 `scope.deny`；Scope 使用路径、符号或明确行为，
   不用“相关代码”等开放表达。
4. 每个子功能分别定义需求、可观察 Acceptance、真实可运行或客观可观察 Verification、实现状态、
   验收状态和按需 Evidence；另定义跨子功能的 `Task 独立验收`，不能用子功能全部通过代替。
5. 为当前 Task 定义完整 Dependencies、Risk 和状态初值；为 future stub 定义足以排队的依赖与
   风险摘要。检查已知依赖的循环、缺失引用、隐式前置条件和当前 Task 内同文件写冲突。
6. future stub 不是 `DRAFT` Task，不能实现。当前 Task 独立验收 `PASSED` 且 Task 为 `DONE` 后，
   Orchestrator 只选择下一 stub 并路由 `materialize-current`；Task Breakdown 使用最新 Anchor、适用
   Foundation/Design、仓库事实和按需 `migration_ref` 物化完整 Markdown，并原子更新 `tasks.yaml` 为
   current `id/task_ref`。Orchestrator 回读后才设置 `focus.task` 与 `READY`；不得由 Orchestrator
   编写 Task 正文。跨 Task 不并行实现。
   当前 Task 内无前置依赖且无写冲突的子功能可以并行，不新增 Task `owner` 或 `parallel` 字段。
7. 超大、不可验收、缺 Verification、Task 独立验收缺失或需要 Material Decision 的 Task返回
   `issues`/`blockers`，
   不用占位 Task 掩盖问题。
8. 写回后回读索引和当前 Task Markdown，执行 Task Readiness Check。缺本次需求、Scope、任何
   子功能 Acceptance/Verification、Task 独立验收、合法依赖或 Risk 时保持 `DRAFT` 并自行修复；
   只有根因是产品语义缺失时才返回需求澄清 Blocker。
9. 保留有权 Owner 已明确采纳、由 Orchestrator 记录的 milestone `follow_ups`；它们不自动进入滚动 `window`，
   也不立即展开为 Markdown Task。排入近期交付时，按正常 Scope、Acceptance、Dependencies
   和 Verification 契约形成 Task，并原子把原记录改为 `status: PROMOTED`、写入 `task_ref`；
   新 Task 保留原 `finding_ref/evidence_ref`，不得留下同源 OPEN Follow-up。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不把 Task 设为 `READY`/`DONE`，不批准 Planning Gate。
- `materialize-current` 只能写 selected stub 对应的 Task Markdown 和 `tasks.yaml` current 引用；它不写
  `focus.task`、Task 状态或任何 Gate。
- 不改变 Requirement、Frozen Design 或 ADR；冲突时返回 DCR/需求变更提示。
- 不实现代码、不安装依赖、不执行发布或迁移。若 Task 需要 Foundation/Material dependency，只记录
  精确候选及影响并请求用户确认；低影响 dev/test dependency 只记录 Assumption 与验证，除非项目规则
  要求 `dependency_policy: confirm_all`。未确认的 Material dependency 不得改 manifest/lockfile 或运行安装命令。

遵循 [工作流协议](../sdlc-orchestrator/references/workflow-protocol.md)和
[变化控制协议](../sdlc-orchestrator/references/change-control.md)，并按
[Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)
生成和复检最终工件表面。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。`next_action_hint` 只是非权威路由建议；Planning Gate
及 Task 状态均由 Orchestrator 决定。
