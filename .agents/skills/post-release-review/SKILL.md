---
name: post-release-review
description: "Evaluate post-release technical health, business outcomes, and experiment results for a specific deployed identity and observation window. Use after rollout when deciding keep, iterate, rollback, or sunset; do not mutate production, close lifecycle state, or infer results without evidence."
---

# Post Release Review

在发布后观察窗口结束时，结合技术健康、业务结果与 Experiment Evidence 评估是否产生价值。
该 Skill 是 Post-release Reviewer；Release 不等于 Done，Review 结论也不直接关闭 Story/Epic。

## Context Contract

### Required Context

- 已部署 target identity、Release/rollout 记录和观察窗口；
- 已声明适用的指标定义、基线和当前数据。

### Conditional Required Context

- 技术指标：Error、Latency、Resource、DB/Queue/Cache、Logs、Alerts；
- 业务指标：Conversion、Retention、Usage、Revenue 或 Story Success Metrics；
- Experiment 的 Assignment、Exposure、Metrics、判定标准与 Data Quality Evidence；
- Incident、Rollback、Customer Impact 和未关闭 Finding。

只要求观察目标已声明适用的指标或 concern；无业务观察、Experiment 或 Incident 时，不得把其
缺失当成 Blocker。

### Optional Context

- Relevant Story Acceptance、Release Plan、Design/ADR；
- 分群、版本、区域和时间窗口切片；
- 已批准的后续观察或迭代提案。

### Forbidden Default Context

- Entire Repo、全部历史发布、Story、Task、ADR 或原始无界日志；
- 未脱敏用户级数据、生产凭据和无授权生产查询；
- 与部署身份或观察窗口不匹配的旧指标和实验结果。

## 初次审查：只读

1. 绑定 deployed target identity、观察窗口、指标版本和数据来源；缺失则返回 Blocker。
2. 对已声明适用的技术健康、业务价值和 Guardrail 分别比较基线、目标和当前结果，不以相关性替代因果性。
3. Experiment Evidence 有效且可解释时，按判定标准输出 `WIN`、`LOSS` 或 `NEUTRAL`；
   已有当前目标 Evidence 足以证明 Assignment、Exposure、Sample 或 Data Quality 使实验
   不可解释时，输出 `INVALID`，记录无效原因并建议重新设计。只有观察尚未完成、关键数据
   根本未采集或环境/权限不可用，无法判断有效性时，才返回 `NOT_RUN`、`UNAVAILABLE` 或
   `BLOCKED`，不得提前形成实验结论。
4. 只有所有已声明适用的技术、业务与 Experiment Evidence 支持时，才综合给出 `SUCCESS/KEEP`、
   `PARTIAL_SUCCESS/ITERATE`、`FAILED/ROLLBACK` 或 `NO_VALUE/SUNSET`；否则
   `post_release_result: null`，不得提前形成最终业务结论。
5. 首次审查不修改生产、实验、Release、Story/Epic 或被审数据；需要继续观察时明确时间窗和 Owner。

## 权限边界

- 不修改 `.sdlc/state.yaml`，不关闭 Story/Epic，不批准 Gate。
- 不执行回滚、扩量、实验关闭、配置变更或数据修复；这些动作需要独立授权和适用 Gate。
- 不把缺失、延迟或污染数据写成成功：可由当前 Evidence 判定实验不可解释时写 `INVALID`；
  尚无足够观察或环境不可用时使用 `NOT_RUN`、`UNAVAILABLE` 或 `BLOCKED`。

遵循 [Evidence 与 SkillResult 协议](../sdlc-orchestrator/references/evidence-and-results.md)。

## 返回

返回 Sparse `skill_result`：始终包含 `skill`、`target`、`outcome`、`next_action_hint`；只在非默认时
返回 artifacts、Evidence、issues、blockers、architecture/scope flags。Reviewer 还必须返回 `review_mode`、`reviewed_target_identity`、
`coverage`、`remaining_risks`、`experiment_result`、`post_release_result` 和
`review_result`。`experiment_result` 使用 `WIN`、`LOSS`、`NEUTRAL`、`INVALID` 或
`null`（不适用/尚不能判断）；`post_release_result` 使用 `SUCCESS/KEEP`、
`PARTIAL_SUCCESS/ITERATE`、`FAILED/ROLLBACK`、`NO_VALUE/SUNSET` 或 `null`。
观察窗口未完成时固定为 `outcome: BLOCKED`、Evidence `NOT_RUN`、两个结果 `null`；环境/
权限不可用时固定为 `outcome: UNAVAILABLE`、Evidence `UNAVAILABLE`、两个结果和
`review_result` 均为 `null`。`NOT_RUN` 只属于 Evidence，不是 `skill_result.outcome`。
`review_result` 使用 `PASS`、
`PASS_WITH_CONDITIONS` 或 `REWORK`，环境不可用时为 `null`；结果建议与
`next_action_hint` 均非权威；
只有 Orchestrator 能依据 Gate 与授权更新生命周期。
