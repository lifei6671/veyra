---
name: implementation
description: "Implement, remediate, re-verify, or run an approved RD show case for one SDLC Task within its explicit scope and frozen design, producing honest evidence. Use when the Task is READY/IN_PROGRESS, or when an IMPLEMENTED/VERIFYING Task needs bounded remediation, missing verification, or Show Case evidence; do not use for unapproved scope, architecture redesign, standalone review, QA sign-off, or release."
---

# Implementation

在一个已批准 Task 的 Scope 内完成最小、正确、可验证的代码交付。该 Skill 是
Implementation Producer；它可以报告实现完成，但不能把 Task、Gate 或 Phase 迁移为完成。

## Context Contract

### Required Context

- `.sdlc/state.yaml`、`.sdlc/memory/HANDOFF.md`、`.sdlc/tasks.yaml`；
- 从 `.sdlc/tasks.yaml.task_ref` 解析的当前 `.sdlc/tasks/TASK-xxx.md` 完整正文，包括本次需求、
  Scope、各子功能 Acceptance/Verification、Task 独立验收、Dependencies、Risk、批准引用以及
  适用的 Design version；
- Git 基线、预存用户改动与 delivery-owned 范围。

若为恢复中的 Task，还必须恢复原始 Delivery Unit 的起始身份、inventory/provenance、已冻结
分区、验证结果和未解决 Finding，不能把当前会话或 Skill 首次介入时的工作树当成新基线。

### Optional Context

- 当前 Story 的相关 Acceptance；
- Task 引用的 Design 小节、ADR/DCR；
- Scope 内相关源码、测试、仓库约定和验证配置。

### Forbidden Default Context

- 全部 Epic、Story、Task、Design、ADR 和历史 Evidence；
- Scope 外源码或与当前 Task 无关的工作树改动；
- 生产环境、凭据、未授权外部系统和未批准的新依赖。

## 前置检查

开始写代码前必须确认：当前 Task 通过 Anchor Drift Check 和 Task Readiness Check；`task_ref`
存在、ID 匹配且 Task Markdown identity 当前；其本次需求、Scope、每项子功能
Acceptance/Verification、Task 独立验收完整；所有适用的 Requirement、Foundation/Design、Planning
或高风险授权已满足；Task 为
`READY`/`IN_PROGRESS`，或为已被 Delivery Review、RD Show Case、QA Finding
明确退回并需要有界修复/补充验证的 `IMPLEMENTED`/`VERIFYING`/`QA_READY`/`QA`；依赖完成；
Design 引用未 stale；所有前序 Task 均已独立验收并 `DONE`；当前 Task 是唯一允许开发的 Task；
Scope 和 Git 基线可识别。任一项不满足，返回 Blocker，不创建业务实现。

`IMPLEMENTED`/`VERIFYING` 的 verification-only 模式只能执行 Task 已声明的验证并追加
当前 target identity 的 Evidence，禁止修改实现、测试、配置或其他目标文件。若检查失败
需要修改，退出 verification-only 并进入 remediation。

remediation 模式只修复 Delivery Review Finding、RD Show Case `FAIL` 或 QA Finding 已指出
且仍在原 Scope/Design 内的问题。任何代码、测试或目标工件变化都必须生成新的 target
identity，使旧 Delivery Review、Show Case、Test、QA 与 Release Evidence 标为 `STALE`，
完成后重新冻结并交给独立 `code-delivery-review`。

show-case/evidence-only 模式用于 Delivery Review 通过但 RD Show Case Evidence 缺失时，
仅按批准的 Show Case 步骤运行当前目标并记录 Evidence，禁止改目标。若演示暴露缺陷，
转入 remediation；该模式不批准 QA Entry 或 QA Gate。

## 实现

1. Task 开始时记录起始 HEAD/快照、完整工作树状态和可用宿主编辑记录；恢复时沿用原始
   baseline。按 [Delivery Unit 协议](../code-delivery-review/references/delivery-unit.md) 持续维护
   path/hunk provenance、exclusion、ambiguity 和 review partition。
2. 每次修改前对账 Anchor、Task Scope、Acceptance、适用的 Frozen Design/ADR 和授权边界。
3. 只修改 `scope.allow` 内的实现、测试和必要文档；`scope.deny` 永远禁止。
4. 选择最小实现，不为假想需求增加抽象、Fallback、Feature Flag、重试、配置或兼容层。
   自主安全加固仅在以下条件同时成立时允许：它关闭已有控制之后仍存在的可信触发路径；保持在
   当前 Task Scope；不改变已接受的产品语义或 Security Model；不引入 Material dependency、抽象、
   配置、持久状态、运行组件、后台机制、兼容层或运维负担；且它是最小局部修复。否则立即停止，
   返回风险、现有控制、剩余路径、最小候选方案和所需授权：尚无覆盖问题的 Frozen Design/ADR 时
   路由 `technical-design`，只有需要改写既有 Frozen Design/ADR 时才通过 Change Control/DCR；
   不得推测性实现较大的缓解方案。
5. 发现已接受 Requirement/Acceptance 需要变化，或需要新增未确认的 Foundation/Material dependency 时
   立即停止，返回 Impact、拟变更内容与所需用户确认；未确认前不得改 canonical Requirement/Task、
   manifest/lockfile/checksum，不得运行安装命令。低影响 dev/test dependency 与可逆开发工具按 L1
   Assumption 处理，除非项目规则要求 `dependency_policy: confirm_all`。Scope 扩张、Material Runtime/Operational
   Dependency、公共 API/协议、持久数据语义/Migration、生产配置契约、权限安全、架构或 Frozen
   Design 变化同样进入 Change Control。标准库、仓库已存在且已批准的依赖可正常使用。
6. 按仓库权威命令执行格式化、构建、测试、Lint/Scan；真实记录 PASS、FAIL、NOT_RUN
   或 UNAVAILABLE，不把自述当 Evidence。
7. 完成逻辑分区或高风险边界时可以记录稳定 checkpoint identity 和定向验证，供后续独立
   Reviewer 复用；checkpoint 不迁移 Task/Gate，也不替代最终完整 Review。
8. 对账 staged、unstaged、untracked、deleted、task-time commits、generated、
   `git diff --name-status`、Task Scope 与 Verification 产生的文件；保留用户已有改动。
9. 可以按子功能报告实现和验证 Evidence，但不得把自己的结果写成子功能或 Task 已验收；
   Orchestrator 依据独立验收 Evidence 更新 `acceptance_status`。所有子功能通过后仍要执行 Task 级验收。
10. 代码命名、测试名、注释、文档、标题、文件名和交付摘要从 Task、Frozen Design、
   delivery-owned Diff 与实际读回状态生成。用户纠正或本轮被弃方案不得成为这些表面的
   叙事中心；真实 Scope、Finding、删除、迁移、兼容、诊断、Evidence 和用户已有改动不得
   因此被隐藏。
11. 若获授权的 commit、PR、Hook 或其他工具创建、包装或改写用户可见结果，必须读回实际
    内容与元数据，重新检查完整交付表面；检查后的任何变化都会使先前检查失效。

## 权限边界

- 不修改 `.sdlc/state.yaml`、Gate、Story/Task 状态或 Human Approval。
- 不提交、推送、部署、迁移或访问生产，除非用户另有明确授权且上游 Gate 允许。
- 不自行执行 Delivery Review；冻结候选目标后交给独立 Reviewer。

遵循 [变化控制协议](../sdlc-orchestrator/references/change-control.md)和
[Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。`signals.delivery_unit` 还应返回起始身份、inventory、
provenance/exclusions/ambiguities、checkpoint、候选 target identity 和 verification summary，
由 Orchestrator 校验后写入既有 Context/HANDOFF/Evidence。`next_action_hint` 非权威，通常
指向 `code-delivery-review`；实现成功不等于 Delivery Gate 通过。
