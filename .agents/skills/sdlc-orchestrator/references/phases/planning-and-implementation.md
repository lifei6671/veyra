# Planning and Implementation

## Task Breakdown

Task 是可独立交付并在完成后独立验收的顺序单元。`tasks.yaml` 保持里程碑、current `id/task_ref` 与
future lightweight stub；只有当前 Task 使用 `.sdlc/tasks/TASK-xxx.md`。一个 Task 可以包含多个可分别
标记和验收的子功能，但还必须有 Task 级整体验收。每个已展开 Task 必须：

- 只承载一个清晰的独立交付 Objective；
- 有 `scope.allow` 和必要的 `scope.deny`；
- 每个子功能都有非空需求、可观察 Acceptance、Verification 和独立状态；
- 有跨子功能的 Task 独立验收与 Task Verification；
- 有显式 Dependencies；
- 绑定相关 Design/ADR；
- 定义真实可运行的 Verification；
- 标注 Risk；任何已接受需求变更或 Foundation/Material dependency 都有用户确认引用；L1 dev/test
  dependency 记录 Assumption 与验证。

使用 current 与 future stub 的已知依赖校验顺序，不只按 P0/P1/P2。后续 Task 可以预先规划但只是
stub；当前 Task 独立验收通过并 `DONE` 后，Orchestrator 只选择 stub 并路由
`task-breakdown: materialize-current`。Planning Producer 物化后，Orchestrator 回读并通过 Readiness
Check 才能更新 Focus/`READY`；不得由 Orchestrator 直接展开正文或开始实现。当前 Task 内
无写冲突、无前置依赖的子功能可以并行。循环依赖、超大 Task、不可验收 Task 或缺 Verification
必须在当前窗口内修复；远期任务保持里程碑。

Task Breakdown 后，Orchestrator 必须回读索引和 Task Markdown 执行 Task Readiness Check；
不满足契约时保持 `DRAFT` 并路由 Planning 修复，不进入 Implementation。

## Implementation 前置检查

1. 当前 Phase 允许实现；
2. Anchor 已建立，且实际适用的 Requirement、Design、Planning Gate 或风险授权已经满足；
3. `task_ref` 可解析、ID 与 Markdown front matter 匹配，Task Readiness Check 已通过；正常实现时
   Task 为 `READY`/`IN_PROGRESS`；或者存在明确的 Delivery Review、RD Show Case、
   QA Finding/缺失 Evidence，Router 已选择 `remediation`、`verification-only` 或
   `show-case/evidence-only`，且 Task 为 `IMPLEMENTED`/`VERIFYING`/`QA_READY`/`QA`；依赖已完成；
4. 适用的 Design version 和引用未 stale；
5. 所有前序 Task 都已有当前独立验收 Evidence 并为 `DONE`，当前 Task 是唯一可开发 Task；
6. Git 基线、预存改动和 delivery-owned 范围已记录；
7. 没有未批准的需求变更、Foundation/Material dependency、Material Scope/Contract、持久数据语义、重要运行依赖
   或权限安全变化。

不满足则停止并返回 Blocker，不创建业务代码。`verification-only` 与
`show-case/evidence-only` 禁止修改目标；`remediation` 发生任何目标变化时必须生成新身份、
失效旧 Delivery/Show Case/QA/Release Evidence，并重新 Delivery Review。

Delivery Unit 从当前 Task 首次获准实现时开始，跨越会话、暂停、用户澄清、子代理和上下文
压缩，直到该 Task 形成一次完整交付结果。Orchestrator 必须保留原始 baseline 和经过校验的
inventory/provenance；恢复任务时不得重置。详细字段遵循
[Delivery Unit 与 Freshness](../../../code-delivery-review/references/delivery-unit.md)。这些信息
只压缩写入 HANDOFF 和必要 Evidence，不增加第二套状态根。

## 实现循环

每次修改前检查：

```text
Is this required by the current Task?
Is the path inside allowed scope?
Does it satisfy Acceptance?
Does it violate frozen Design or ADR?
Does it introduce a material dependency or public/persistent/runtime/operational contract?
Does it change an accepted requirement or introduce an unapproved third-party dependency?
```

选择最小、正确、可验证的改动。不要为假想未来需求增加抽象、fallback、兼容层、Feature Flag、重试或配置项。系统边界做必要校验，可信内部状态异常时快速失败。

按仓库约定补充有意义的测试。Agent 擅长生成结构和明显参数错误用例，但必须主动检查业务边界、状态副作用、并发/幂等、失败与回滚断言，不能用浅断言冒充质量。

## 软约束检查

实现完成后至少对账：

- `git status --short --untracked-files=all`
- staged / unstaged / untracked / deleted 文件；
- `git diff --name-status` 与 Task Scope；
- delivery-owned Diff 与 Acceptance；
- 实际 Build/Test/Lint/Scan；
- Design/ADR 引用和新决策；
- Verification 是否改变了文件。

这些检查提供 Evidence，但不是机械拦截。发现越界时按 Change Control 停止，不得隐藏或回滚用户改动。

## Delivery Review

实现与验证完成后冻结目标。使用专门的代码交付审查 Skill，或由未参与实现的只读 Reviewer
执行等价完整范围审查。ReviewContext 仅包含当前 Task、Acceptance、相关 Design/ADR、完整
delivery-owned inventory/Diff、coverage manifest、冻结身份和实际 Evidence。

小型/中型变更可执行一次全范围审查；大型/多模块变更必须每个分区都有新鲜结果并追加
Integration Review。Implementation 记录的 checkpoint 只有在目标和 material interaction
未变化时才能复用，且不能单独批准 Delivery Gate。

正式 Review 必须覆盖所有 delivery-owned 内容、目标和重要 Context 新鲜、无 P0/P1，才能让
Delivery Gate 通过。Delivery Review 通过仍不等于 Task 已独立验收：Orchestrator 还必须逐项核验
子功能 Acceptance Evidence，并执行 Task Markdown 声明的 Task 级整体/集成验收。Producer 不能以
`SELF_REVIEW` 替代独立验收。修复 Finding 后由
Orchestrator 路由 `implementation(remediation)`，重新运行受影响验证、生成新身份并复审；
默认最多三轮自动修复，不循环扩大 Scope。

## Context 契约

默认只读 State、`memory/HANDOFF.md`、`tasks.yaml` 和解析后的 focus Task Markdown。按需加载 Task 引用的
Requirement/Story Acceptance、Design/ADR、相关源码与测试。禁止默认读取全部 Epic、Story、
Task、Design、ADR 或历史 Evidence。
