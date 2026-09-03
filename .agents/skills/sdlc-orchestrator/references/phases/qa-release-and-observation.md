# QA, Release, and Observation

## QA Case Design

Acceptance target 与 Task 稳定、适用的 Material Design 已冻结后，即可与 Implementation 并行
设计 QA Case。根据 Test Strategy 覆盖实际适用的：Functional、Boundary、Failure、Permission、
Integration、Regression、Security、Performance、Compatibility、Experiment 和 Observability。

每个关键 Acceptance 至少映射一个可执行或可观察 Case，并明确前置条件、步骤、预期结果、所需 Evidence 和环境。QA Case Design 不修改业务实现，也不自行批准 QA Entry。

`qa-review` 必须用互斥模式工作。按项目风险选择 QA 深度：

- `light`：Case + QA Verification；
- `standard`：Case + independent Verification；仅项目规则或实际风险要求时追加 Case Review；
- `critical`：Case Design -> independent Case Review -> RD Show Case -> Verification。

`case-design` 只生产 Case；`case-review` 首次只读且不得由 Producer 自审；Case 改变会使已存在的
Case Review、受影响 QA Result 和 Release Readiness Evidence stale。没有适用的独立 Case Review
不得阻止 light/standard QA 进入 Verification。

## Show Case 与 QA Entry

仅当当前 QA depth 或项目规则要求 RD Show Case 时，才在进入 QA 前执行它，以证明核心路径可运行、
环境可用、关键 Acceptance 基本满足；存在 Experiment 时同时演示 Control/Treatment、Exposure 和关键埋点。

当前 QA depth 或项目规则要求、但缺 RD Show Case Evidence 时，路由到 `implementation` 的
`show-case/evidence-only` 模式。
该模式只运行批准步骤并记录当前目标 Evidence；若需要改代码或测试，转入 remediation、
生成新 target identity 并重新 Delivery Review。

QA Entry 共同需要 Implementation / Dependencies、相关 Unit/Integration/Static Evidence、
Delivery Review 和 P0/P1 为 0。`critical` 额外需要 QA Case Review 与 Show Case；`light` 与
`standard` 只要求其已声明适用的项。缺少当前 QA 级别的必需项才保持 `PENDING` 或 `FAILED`，
不得以“代码已完成”进入 QA。

## QA Verification 与 Bug

按策略执行 Smoke、White/Black Box、Integration、Regression 和 Experiment Verification。失败
建立关联 Acceptance target / Task 的 Finding；QA 负责报告与回归，不应在 QA 角色中直接改代码再宣布通过。

修复流程：QA Finding -> Implementation Fix -> RD Verification -> QA Regression。实现变化会使相关 Review、Test、QA 和 Release Evidence stale。

## Release Planning

Release Planning 可以与 QA 并行，但 Readiness 必须等待所需 Evidence。计划按适用性覆盖：Dependencies、DB/Cache/MQ、Config/Secrets、Infrastructure/Capacity、Order、Compatibility、Migration、Rollback、Feature Flag、Experiment、Observability、Alerting、Runbook。

`release-planning` 是 Orchestrator 的内部 Producer 动作，不是第十个 Specialist。它在
Release Plan 缺失、被 Reviewer 退回或因上游变化失效时创建/修订计划；只允许写 Release
Plan 及其派生上下文，不得给出 Readiness 结论，也不得执行生产操作。计划完成后必须路由
到独立的 `release-review`，避免同一角色既生产又批准。

Release Reviewer 默认只读 Release Plan、Deployment/Migration/Rollback Design、依赖、QA 摘要和 Observability；除非具体问题需要，不读取业务源码。

生产发布、危险迁移、大流量切换、生产回滚和实验关闭是默认 Human Gate。用户普通的“继续”不能授权这些动作。

## Rollout

遵循 Release Plan 声明的 rollout sequence。每个适用步骤都必须有进入条件、停止条件、验证方法或
观察指标，以及 Rollback 触发器；未声明的 Migration、Canary、渐进发布或全量发布不是固定步骤。
不要在生产上临时试验或跳过 Human Gate。

## Observation

只有定义了发布后观察窗口、业务指标或实验结论时，才进入 Observation，不直接 Done：

- RD：Error、Latency、CPU/Memory、DB/Queue/Cache、Logs、Alerts；
- PM/Product：Conversion、Retention、Usage、Revenue、Experiment Metrics。

观察窗口未结束或必需数据仍在采集时，使用 Orchestrator 内部 `observation` 动作记录当前
状态或等待，不调用最终 Reviewer、不形成结果。窗口和必需 Evidence 完整后才路由
`post-release-review`；若 Reviewer 发现窗口其实未完成，固定返回 Blocker 与 `NOT_RUN`
Evidence，两个最终结果保持 `null`。

Experiment 结论必须引用当前实验身份的 Exposure/Data Collection Evidence：

- `WIN` -> 推广 treatment；
- `LOSS` -> 关闭或回滚；
- `NEUTRAL` -> 请求产品决策；
- `INVALID` -> 重新设计实验。

生产关停/回滚仍需 Human Gate。Post Release Review 结论：`SUCCESS/KEEP`、
`PARTIAL_SUCCESS/ITERATE`、`FAILED/ROLLBACK`、`NO_VALUE/SUNSET`。

Observation 适用时，只有技术验证、业务评估、所需观察窗口和后续决定都有 Evidence，才能关闭
该子流程。Observation 不适用时，不创建指标或 Post-release Review；发布证据和其他适用子流程
闭合后即可完成当前交付目标。

## Context 契约

QA 加载当前 Acceptance target、Task、Test Cases、Diff 摘要、Evidence 和适用 Experiment；
Release 加载 Release Plan、受影响 Design、Migration/Rollback、依赖、Observability 和 QA 摘要；
Observation 加载指标定义、基线、当前数据与发布/实验身份。均不默认读取整个仓库。
