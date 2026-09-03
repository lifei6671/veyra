# Workflow Protocol

## 三个平面

- **Control Plane**：Orchestrator、State、Router、适用 Gate。
- **Intelligence Plane**：Requirement、Design、Planning、Implementation、QA、Reviewer。
- **Evidence Plane**：Git、Build、Test、Lint、Scan、Show Case、Review 记录。

V0.2 是 structured soft enforcement，没有自研 Enforcement Runtime。它以 Intent Anchor 和
滚动交付为主线；Phase 和 Gate 只描述实际会阻止主线的状态，不要求 Agent 表演完整 SDLC。

## Sparse Canonical State

`.sdlc/state.yaml` 只保存稳定意图、当前位置、焦点、实际进入的 Gate、Blocker 和下一个路由。
新项目使用以下最小形状：

```yaml
schema_version: 2
workflow_version: "0.2"
mode: ingest

project:
  id: PROJECT-001
  profile: standard
  engineering_context: established
  # review_policy is absent unless an explicit fallback is accepted.

source:
  requirement: docs/requirements.md
  identity: sha256:replace-with-current-source-identity

anchor:
  goal: Replace with the accepted goal.
  must_have: []
  non_goals: []
  constraints: []
  critical_invariants: []
  acceptance_refs: []

assumptions: []
phase: EXECUTING

focus:
  task: TASK-003

gates:
  delivery:
    status: PENDING
    target_identity: null
    evidence: []
    reviewed_by: null
    approved_by: null
    evaluated_at: null

blocked: null

next:
  skill: implementation
  target: TASK-003
  reason: Implement the current bounded task.
```

`source` 固定 Requirement Baseline 的位置和身份；`anchor` 保存每个 Task 都必须尊重的稳定
意图，但不复制完整 Requirement Source。可逆选择写入 `assumptions`，并包含依据、验证方式和
失效条件；未确认的 Material Decision 不得伪装成 Accepted Decision。

State 保持稀疏：

- `gates` 只在实际进入 Gate 时增加该键；不预建 `NOT_EVALUATED` 列表。
- `author` 的 Candidate 已形成但尚未接受时，允许唯一的 pending-baseline 状态：
  `phase: PROJECT_INIT`、`source.requirement: null`、`source.identity: null`、未形成 Anchor，且
  `focus.task: null`、`gates.requirement.status: PENDING`。该 Gate 的 `target_identity` 是
  `.sdlc/REQUIREMENT.md` 当前 Candidate identity；它标识待批准草案，不把该文件变成
  Accepted Source。恢复时必须重算该 Candidate identity；不同时，这不是可直接恢复的旧
  pending-baseline，Orchestrator 清除该 Candidate 的 Gate Evidence、以当前 identity 重建同一
  `PENDING` Gate，并请求 Human approval，不进入 Accepted Source Change Control。
- `design`、`workstreams`、`experiment` 等子流程状态只在确实需要持久化当前动作时出现。
- `project.engineering_context` 在首次 intake 后必须为 `greenfield` 或 `established`；前者需要
  Foundation，后者沿用现有架构。它与 `author/ingest` 正交。
- 不保存 `implementation_ready` 等可由 Task、Blocker 与适用 Gate 推导的字段。
- `project.review_policy` 默认缺失并视为 `independent_required`；只有用户已明确接受一次
  `standard/best_effort_self_review` 降级时才持久化，并同时写入 `project.review_policy_source` 的
  `approved_by`、`accepted_at` 和 `evidence_ref`。
- `focus` 默认只保存当前 Task；Story/Epic 是可选业务工件，不是执行前提。
- 无 Blocker 时使用 `blocked: null`。有 Blocker 时记录 `origin_phase`、`reason`、`owner`、
  `scope`、`unlock_condition` 和 `next_check`。
- 仅当 ingest 因 Material clarification 被阻塞时，`memory/HANDOFF.md` 可保存与 Requirement
  Source identity 绑定的派生 Intake Snapshot；它不是 State 字段或新的事实源，identity 不匹配或
  快照缺失时必须回读 canonical Requirement Source。

旧版或过渡版状态不得由本文件的 Router 直接执行。识别后读取
[legacy-v01-migration.md](legacy-v01-migration.md)，先归一化为上述 V0.2 形状，再继续路由。

## 闭合词汇

- `mode`：`author`、`ingest`。
- `phase`：`PROJECT_INIT`、`ANCHORED`、`PLAN`、`EXECUTING`、`REVIEW`、`REMEDIATION`、`DONE`。
- `project.profile`：`prototype`、`standard`、`critical`。
- `project.engineering_context`：`greenfield`、`established`。`established` 仅表示当前交付目标可
  可靠沿用已有 stack、布局/模块约定、build/test 约定和架构边界；仓库仅有 README、manifest 或
  需求文档时仍为 `greenfield`。成熟 monorepo 中的新 service 也可以是 `greenfield`。
- `task.status`：`DRAFT`、`BLOCKED`、`READY`、`IN_PROGRESS`、`IMPLEMENTED`、`VERIFYING`、`QA_READY`、`QA`、`DONE`、`CANCELLED`。
- `gate.status`：`PENDING`、`PASSED`、`FAILED`、`STALE`、`WAIVED`。
- `skill_result.outcome`：`PASS`、`FAIL`、`BLOCKED`、`UNAVAILABLE`。
- Reviewer：`PASS`、`PASS_WITH_CONDITIONS`、`REWORK`。
- Evidence：`PASS`、`FAIL`、`NOT_RUN`、`UNAVAILABLE`、`STALE`。

Task 的共同迁移为 `DRAFT -> READY -> IN_PROGRESS -> IMPLEMENTED -> VERIFYING`。随后按适用性
分支：独立 QA 适用时走 `VERIFYING -> QA_READY -> QA -> DONE`；独立 QA 不适用时仍可走
`VERIFYING -> DONE`，但两条路径都必须先通过独立 Task Acceptance：所有子功能分别有当前
验收 Evidence、Task 级整体/集成验收通过、Delivery Review 与 Task Verification 新鲜。Task 未
`DONE` 前不得开始后续 Task。`BLOCKED` 必须保存恢复点，`CANCELLED` 是终止状态；Task Acceptance
不等于强制创建 QA Gate；不得为了满足状态图而强行引入 QA。

## 主生命周期

```text
author -> ANCHORED ─┐
                    ├-> PLAN -> EXECUTING -> REVIEW -> DONE
ingest -> ANCHORED ─┘               │
                                    └-> REMEDIATION
```

Design、QA、Release 和 Observation 是按需子流程，不是每个项目的必经 Phase。

入口由两个正交维度决定：

```text
ingest + established: Full Read -> Readiness -> Anchor -> Tasks -> Contract Check
ingest + greenfield:  Full Read -> Readiness -> Anchor -> Foundation -> applicable Review -> Human approval
                      -> Tasks -> Contract Check
author + established: Requirement approval -> Anchor -> existing architecture -> Tasks
author + greenfield:  Requirement approval -> Anchor -> Foundation -> applicable Review -> Human approval -> Tasks
```

## Impact / Reversibility / Contract

不要根据“依赖、配置、数据库”这些对象类型机械询问。先评估影响、可逆性和契约边界：

- **自主执行**：当前 Task Scope 内、内部或局部、低影响、低成本可逆，且不改变公共、持久化、
  运行或运维契约。标准库、仓库已有且已批准的第三方依赖、内部 config struct 和内部 package
  通常属于此类，但更近的项目规则可以要求确认。
- **Assumption 后执行**：仓库没有既定答案，但选择仍低风险、可逆，并有明确失效条件与验证方式。
- **Foundation Decision**：Greenfield 中技术上可逆、但会在当前规划窗口后被大量代码放大迁移
  成本的 Framework、项目布局、Module Boundary、Data Access、API/Transport、Migration、测试
  基础设施和核心依赖策略；在首次实现前一次集中确认。
- **请求确认 / Change Control**：Scope 扩张；公共 API/协议/外部调用方契约变化；持久数据语义
  或 Migration；重要运行时/运维依赖；权限、鉴权或安全模型；生产配置协议、部署边界、明显外部
  成本、生产/破坏性动作；违反 Frozen Design/ADR；或适用项目规则明确要求确认。

已接受 Requirement/Acceptance 变化必须先做 Impact Analysis 并获得用户明确确认。依赖按风险
分级：Foundation 或 Material runtime/operational dependency（框架、ORM、迁移、鉴权、MQ、外部
服务/大型 SDK、runtime plugin）必须确认；获批准 Foundation 中精确列出的依赖可正常使用；仅用于
开发/测试的低影响工具和其它低风险可逆依赖可按 L1 Assumption 自主引入并记录验证。项目规则可
显式采用 `dependency_policy: confirm_all` 覆盖本默认。未确认的 Material dependency 不得修改
manifest、lockfile/checksum 或运行安装命令。用户要求“按该需求开始开发”不覆盖这些边界。

## 不确定性与 V0.2 Router

先按 `infer -> assume + record -> proceed -> ask` 处理不确定性。只有会改变产品语义或上述
Material Contract、且不能低成本逆转的 Material Uncertainty 才阻塞；随后按顺序命中第一个谓词：

| Predicate | Route |
| --- | --- |
| `blocked != null` 且 unlock condition 未满足 | 路由 blocker owner/action，不迁移 |
| `author` 且 Candidate 存在、`gates.requirement.status = PENDING` | 请求 Human approval；不得再次调用 `requirement-review` 生成或改写 Candidate |
| `author` 且 Material Product Uncertainty 需要研究或有界 POC | `discovery`；只解决该未知 |
| `author` 且 canonical Baseline/Anchor 未形成，且不存在 pending Candidate | `requirement-review` 的 `refinement`；创建或更新 `.sdlc/REQUIREMENT.md` |
| `ingest` 且是首次 intake、Readiness 未完成 | 完整读取 Requirement Source，运行 Ingest Readiness Pass；不默认 Formal Review |
| `ingest` Readiness = `NEEDS_CLARIFICATION` | 写入 `blocked`（`origin_phase: PROJECT_INIT`、`owner: user`、`reason: material_requirement_clarification`、问题 scope、unlock condition）及 source-bound Intake Snapshot，集中请求 clarification；不得建立 Anchor 或实现 |
| Requirement Ready 且 Anchor 未形成 | 建立 Anchor 并记录 `project.engineering_context`；不默认 Formal Review |
| `greenfield` 且 Foundation 不存在 | `technical-design` 的 `foundation`；形成单文件和同一 `technical_design` PENDING Gate |
| `greenfield` 且 Foundation identity 匹配、适用独立 Review 缺失或 stale | `technical-design-review`；standard/critical 默认适用，prototype 仅在 Material concern 时适用 |
| Foundation Review = `REWORK` | `technical-design` 的 `remediation`；只修复 Foundation Finding，并生成新 identity |
| `greenfield` 且 Foundation Review 当前、Human approval 缺失 | 只请求 Human approval；不得再次调用 `technical-design` 或跳过 Review |
| `greenfield` 且 Foundation Review 与 Human approval 均匹配当前 identity | 标记 Foundation frozen、`technical_design` 为 `PASSED`，再进入 Task Breakdown |
| Anchor/适用 Foundation 已建立但滚动窗口没有当前 Task | `task-breakdown` 的 rolling 模式，创建 Markdown Task 引用 |
| 当前 Task Readiness Check 失败 | Task 保持 `DRAFT`，路由 `task-breakdown` 修复；只有产品语义缺失才回需求澄清 |
| 当前 Task 触及 Material Decision，或 Design Review = `REWORK` | `technical-design`；只覆盖受影响 concern |
| 非 Foundation 的 Material Design 候选已冻结且独立审查未完成 | `technical-design-review` |
| 当前 Task 为 `READY`/`IN_PROGRESS`、Markdown Contract 有效、所有前序 Task 已 `DONE` | `implementation` |
| 当前 Task 为 `IMPLEMENTED`/`VERIFYING` 且 Finding 要求修改目标 | `implementation` 的 `remediation`；生成新身份并失效受影响 Evidence |
| 当前 Task 只缺声明的实现验证 Evidence | `implementation` 的 `verification-only`；不得修改目标 |
| Delivery Unit 已冻结且 Delivery Review 未通过 | `code-delivery-review` |
| 独立 QA 适用且 QA Case 缺失、`STALE` 或 Review = `REWORK` | `qa-review` 的 `case-design` |
| 独立 QA Case Review 适用且 QA Case 已冻结且独立 Case Review 未完成 | `qa-review` 的 `case-review` |
| Delivery Review 通过但批准的 RD Show Case Evidence 缺失 | `implementation` 的 `show-case/evidence-only` |
| QA Finding 要求修改实现或测试 | `implementation` 的 `remediation`，重走 Delivery Review |
| 独立 QA 适用且目标进入 `QA_READY`/`QA` | `qa-review` 的 `verification` |
| Delivery Review、适用 QA 与子功能验收已闭合，但 Task Markdown 的 Task 独立验收 Evidence 缺失或 stale | `task-acceptance`（Orchestrator 内部动作）；按当前 Task 的整体验证更新该 Markdown，不能创建 Gate 或新 Skill |
| Release 子流程适用，且 Plan 缺失、`STALE` 或 Review = `REWORK` | `release-planning`（Orchestrator 内部 Producer 动作） |
| Release Plan 完整且 Readiness Review 未通过 | `release-review` |
| 生产动作等待 Human Gate | 请求明确 approval，不调 Specialist |
| 已批准 Rollout 存在下一个有界步骤 | 执行该步骤并记录 Evidence |
| Observation 适用但窗口或必需数据未完成 | `observation`（收集/等待，不形成最终结论） |
| Observation 窗口与必需 Evidence 完整 | `post-release-review` |
| Task Completion Predicate 成立 | 迁移当前 Task 为 `DONE`、选择下一 stub，并路由 `task-breakdown` 的 `materialize-current`；Orchestrator 不编写 Task 正文 |

并行仅用于当前 Task 内无前置依赖、无写冲突的子功能或辅助动作，不并行开发多个 Task。QA Case Design 或 Release Planning 可以在条件满足时
作为辅助动作，但不得使 Readiness 提前通过。

## Gate 权限与稀疏记录

| Gate | 何时适用 | Agent / Automatic Evidence | Human authority |
| --- | --- | --- | --- |
| Requirement | `author` Baseline 改变产品语义，或用户明确要求评审 | Schema、追踪、独立需求审查 | 接受 Material Product Semantics |
| Technical Design | Material Engineering Decision | Scoped Design Review、风险证据 | 公共/持久化/运行/安全等契约决定 |
| Planning | 当前窗口需要独立计划审查 | DAG、Scope、Acceptance、Verify | Scope 或责任边界扩张 |
| Delivery | 每个实现 Delivery Unit | Build/Test/Lint/Scan、独立 Delivery Review；或已批准 policy 下披露非独立性的受限 SELF_REVIEW Evidence | Material Scope/Contract 变化 |
| QA | 独立 QA 或项目规则要求 | Case Review、QA Evidence | 项目规定的高风险验收 |
| Release Readiness | 存在发布动作 | Plan Review、Readiness Evidence | 生产发布、危险迁移、切流、生产回滚 |
| Observation | 需要发布后技术/业务结论 | 窗口、指标、Post-release Review | 实验关停和业务去向 |

Gate 只在进入时创建，至少包含 `status`、`target_identity`、`evidence`、`reviewed_by`、
`approved_by`、`evaluated_at`。Automatic 只能记录观察，Reviewer 只能给审查结论，Human 只能
批准自己有权的边界；三者不能互相替代。

## Gate 最小条件

- Requirement：仅 `author` 或用户明确要求时。`author` Candidate 形成后先写入稀疏
  `PENDING` Gate 和 Candidate identity，等待 Human approval；不建立 Source 或 Anchor。用户明确
  接受当前 Candidate 后，回读并绑定 `source.requirement` / `source.identity`，建立 Anchor，随后将
  同一 Requirement Gate 更新为 `PASSED`。
- Technical Design：仅 Material Engineering Decision；只审受影响 concern。Greenfield Foundation
  首次生成时创建该 Gate 的 `PENDING` record，`target_identity` 是 foundation 当前 identity。对适用
  独立 Review 的 Foundation，`reviewed_by: null` 表示等待 Review；`reviewed_by` 与当前 review
  Evidence 均匹配、`approved_by: null` 表示只等待 Human approval；两者均匹配当前 identity 后才可
  标记 `PASSED` 并冻结。identity 改变时清空 review/approval/evidence、保持 `PENDING`，重新进入
  Review，再请求批准，不重跑同一 Producer。
- Planning：`tasks.yaml` 的 current `task_ref` 可解析；依赖无环；当前 Task 的本次需求、Scope、
  每项子功能 Acceptance/Verify、Task 独立验收、Risk 完整；future stub 只有轻量规划字段；未确认
  需求变更/Material dependency 为零。
- Delivery：实现匹配 Task，Scope 对账，必需验证与适用 Review Policy 的新鲜审查当前，P0/P1 为 0。
- QA：仅独立 QA 适用时；批准 Case、目标与 Evidence 新鲜。
- Release Readiness：仅有发布动作时；受影响依赖、Migration/Rollback、Observability 和 Runbook 就绪。
- Observation：仅定义了发布后窗口或业务/实验结论时；窗口与必需数据完整。

## Profile

- `prototype`：Anchor、Task Verification、Task 独立验收、Show Case、退出/清理决定；
  高风险边界自动升级。Foundation Review 默认可省略，但触及 Material concern 时仍适用。
- `standard`：Anchor、滚动计划、Delivery Review、Task 独立验收和适用验证；
  其他子流程按风险进入。
- `critical`：Standard + 实际受影响的安全/性能/迁移/回滚审查、演练和额外 Human Gate。

Profile 调整深度，不改变适用项目规则或 Material 边界的授权要求。

## Reviewer Capability 降级

默认 `review_policy` 为 `independent_required`，只在独立 Reviewer 能力不可用时才评估降级：

- `critical`：保持 `BLOCKED`，不得降级；
- `prototype`：可用披露的 `SELF_REVIEW` 加 Task Verification 闭合低风险交付；
- `standard`：首次遇到能力不足时，只询问一次项目级选择。用户明确允许后，持久化
  `project.review_policy: best_effort_self_review` 与可追溯 `project.review_policy_source`；该项目后续
  低风险 Delivery Unit 可由披露的 `SELF_REVIEW` + 新鲜 Verification 形成受限 Delivery Evidence。
  Gate 记录必须保留 `reviewed_by: SELF_REVIEW`、`review_policy` 和 `review_policy_source`，且只在
  其他 Delivery 条件都满足时才可 `PASSED`。未获明确许可则保持
  `independent_required` 并返回 `UNAVAILABLE`。

`best_effort_self_review` 不是独立 Review，也不适用于 `critical`、项目规则要求独立审查、或
Material Contract/Security/Production 边界。它只记录为项目级降级政策，不得伪装为 Reviewer
独立性或通用 `gate.status: WAIVED`。

## 迁移和特殊状态

迁移前必须确认状态边合法、适用 Gate 通过或合法豁免、Evidence 匹配当前目标、无 Blocker、
Human Gate 已批准且失效副作用已记录。Specialist 的 `next_action_hint` 不是迁移权。

- `BLOCKED`：写入非空 `blocked` 对象；解除后回到 `origin_phase` 并设回 `blocked: null`。
- `REMEDIATION`：记录 originating Gate、需修复 Artifact 和失效范围，完成后重过受影响 Gate。
- 暂停、取消、拒绝或回滚等非常态动作只在实际发生时记录到 Task、Gate、Release Artifact 或
  HANDOFF；不要为它们预建空字段。
