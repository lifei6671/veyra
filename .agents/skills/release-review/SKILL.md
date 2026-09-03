---
name: release-review
description: "Review release readiness, rollout, migration, rollback, dependency, observability, and QA evidence for a specific release target. Use before a release Human Gate or after a material release-plan change; do not deploy, migrate, roll back, or alter production on the first review pass."
---

# Release Review

对具体 Release target 的计划与 Readiness 执行独立只读审查，确保发布、停止和回滚条件
可执行且由当前 Evidence 支持。该 Skill 是 Release Reviewer，不是发布执行器或批准者。

## Context Contract

### Required Context

- 当前 Release Plan 与明确 target identity；
- 已声明的适用 concern 与 Release 顺序。

### Conditional Required Context

- Deployment、Migration、Rollback、Dependency、Compatibility、Config/Secrets、Infrastructure/Capacity；
- QA Summary、未关闭 P0/P1、Monitoring/Alerting、Runbook；
- Feature Flag、Experiment、Data Collection 与 Owner。

以上项只在 Release Plan 声明 concern 适用时必需；未声明的 concern 不得伪造成 Blocker。

### Optional Context

- Relevant Design/ADR/DCR 和演练 Evidence；
- 变更摘要、依赖清单、容量基线和已批准变更单；
- Canary/Progressive Rollout 指标与历史发布基线。

### Forbidden Default Context

- Entire Repo、无关业务源码、全部 Story/Task/ADR/Evidence；
- 生产凭据、未脱敏用户数据和未授权外部系统；
- 与当前发布身份不匹配的旧 QA、演练或监控 Evidence。

## 初次审查：只读

1. 确认 Release target、变更范围、Owner、窗口、前置依赖和批准链。
2. 只对已声明适用的 DB/Cache/MQ、Config/Secrets、Infrastructure/Capacity、Order、
   Compatibility、Migration、Rollback、Feature Flag、Experiment、Observability、Alerting、Runbook。
3. 验证 Release Plan 声明的 rollout sequence：每个适用步骤都有进入条件、停止条件、验证
   方法/观察指标和 Rollback 触发器；未声明的 Migration、Canary 或渐进发布不得被当作固定步骤。
4. 核对 QA Summary、P0/P1、演练和监控 Evidence 的 target identity 与 freshness。
5. 以 P0–P3 输出 `issues`、阻塞项、覆盖范围和剩余风险；首次审查不修改计划或被审工件。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不批准 Release Gate 或 Human Gate。
- 不部署、迁移、切流、回滚、关闭实验或修改生产配置；用户普通的“继续”不构成授权。
- 可在 Orchestrator 指定的独立输出路径写 Review 记录，但不得直接修 Release Plan。

遵循 [工作流协议](../sdlc-orchestrator/references/workflow-protocol.md)和
[Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。Reviewer 还必须返回 `review_mode`、`reviewed_target_identity`、
`coverage`、`remaining_risks` 和 `review_result`。`review_result` 使用 `PASS`、
`PASS_WITH_CONDITIONS` 或 `REWORK`，环境不可用时为 `null`；Readiness 结论只是
专业审查结果；
`next_action_hint` 非权威，生产操作仍需适用 Human Gate。
