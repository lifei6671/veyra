# Delivery Review Protocol

从 [Delivery Unit](delivery-unit.md) 的 inventory、规则、change map、coverage manifest、
Verification、target identity 和 Review Context 开始。审查当前 Task 的 delivery-owned 内容，
不是整个仓库或任意 Git 对象。

```text
Delivery Unit
  -> Review Planner(Tier + Signals + Diff Map + Previous Findings)
  -> Applicable Reviewer Lanes
  -> Judge Pass
  -> review_result
  -> SDLC Orchestrator evaluates Gate
```

`Review Planner`、Reviewer Lanes 和 Judge 都是 `code-delivery-review` 内部角色。它们不新增
Skill、Runtime、状态、缓存或默认文件，也不能修改 canonical State 或批准 Delivery Gate。

## Review Planner

Planner 只产生本轮 Review Plan：确定风险深度、Diff 读取方式、适用 Lanes、Coverage 策略、
执行方式和复审范围。它不对未冻结目标开始审查，也不把调度决定写成新的流程事实源。

## 风险深度

- `TIER_1_FOCUSED`：小范围内部实现，无 Material Contract，且没有 Security、Persistence、
  Concurrency 或 Production Signal。默认选择 `correctness` 与 `verification`；极小变更可由一个
  general review 同时覆盖两者，但 Coverage 不能隐式抽样。
- `TIER_2_STANDARD`：普通功能的默认层级。选择 `correctness`、`verification`，再选择所有
  material 的 Domain Lane；通常为 0–2 个。若需要超过两个独立 Domain Lane 或跨越多个系统
  边界，Planner 应考虑 `TIER_3_DEEP`。
- `TIER_3_DEEP`：存在认证授权、安全、加密、公共 API/协议、持久数据、Migration、并发、
  Production Config/Deployment 或大型跨模块变化等 Material Signal。执行所有适用 Lanes，
  上钻调用、失败、生命周期、兼容、恢复和跨模块影响，但不机械开启无关 Lane。

风险深度决定证据深度和投入；Change Signals 决定 Lanes。两者都不自动改变 Profile、Reviewer
隔离要求、Human Gate 或权限，也不能仅凭 Diff 行数把 Security/Contract 变化降级。

## Diff 分类

Planner 对 inventory 中的每个路径或 hunk 分类；所有类别仍保留 provenance 和 Coverage：

- `PRIMARY`：实现、测试、Schema、Migration、行为配置、API/Protocol 等主要审查对象；
- `CONTEXT_ONLY`：Lockfile、生成 Manifest 等语义证据；用于确认依赖身份和影响，不逐行制造
  Style Finding；
- `GENERATED`：Protobuf、ORM、Codegen 等生成输出；默认检查权威源、生成一致性和消费者影响，
  不对生成代码本身提出手工质量偏好；
- `EXCLUDED`：Vendor、Minified、Source Map、Binary 等默认不逐行审查的内容。只有 Task 明确
  修改其生成/供应机制或它产生 material interaction 时才升级分类。

Migration 即使由工具生成，也因持久化与 Rollout 语义归入 `PRIMARY`。Dependency Change 的
Lockfile 可为 `CONTEXT_ONLY`，但权威依赖声明、解析身份和平台影响仍必须审查。`EXCLUDED` 必须
有理由；分类不得掩盖 material Coverage gap。

## Applicable Reviewer Lanes

Lane 是内部关注面，不是固定 Reviewer Skill。Planner 使用 `Tier + Change Signals ->
Applicable Lanes`：

| Lane | 激活信号 |
| --- | --- |
| `correctness` | 所有行为变更；逻辑、错误处理、状态和副作用 |
| `security` | Auth、Permission、Trust Boundary、Secret、Crypto、不可信输入 |
| `contract-data` | API、Protocol、Schema、Persistence、Migration、Material Config |
| `concurrency-performance` | 并发、锁、生命周期、资源所有权、真实 Hot Path |
| `verification` | 所有实现变更；Acceptance Coverage、失败/边界路径和测试质量 |
| `context-docs` | Toolchain、构建/测试框架、包管理、目录布局、必需环境或 CI 命令变化 |
| `release` | Runtime、Deployment、Production Config 或 Migration 执行/恢复语义变化 |

普通功能、Bug Fix、CSS 或沿用既有约定的低物质性变更不激活 `context-docs`；仅因领域存在也不
激活对应 Lane。`release` 只审当前交付的代码可发布性，不替代 `release-planning` 或
`release-review` Skill。
Planner 先读取 [Review Lane Index](review/INDEX.md)，再只加载 Universal NOT-Flag 和选中的
Lane 文件；未选 Lane 不进入 Context。必要时一个 Lane 可覆盖多个相邻 concern，但最终 Coverage
必须说明实际责任与路径。

Lane 若发现新的 material signal，不得自行加载或执行未选 Lane。它必须把 signal、相关路径和
证据返回 Review Planner；Planner 更新 `selected_lanes` 与 assigned paths，加载新增 Lane，刷新 Coverage
后再继续审查该 concern。

## Shared Review Context Packet

Planner 为所有 Lane 构造一次临时 Packet，再为每个 Lane 附加 assigned paths 和 lane-specific
rules，避免重复加载整个 Task：

```yaml
review_context:
  task: TASK-017
  objective: <objective>
  acceptance_refs: []
  target_identity: <frozen identity>
  tier: TIER_2_STANDARD
  signals: []
  diff_map: []
  relevant_design_refs: []
  verification_summary: []
  previous_findings: []
```

Packet 只存在于当前 Agent 调度上下文，审查结束即丢弃；不得写入 `.sdlc`、创建 Review Cache、
新增状态或要求默认 Artifact。需要跨会话保留的最终 Review Evidence 仍遵循现有 Evidence 策略，
不持久化这个调度 Packet。

## Change Map 与 Checkpoint

每个变更组记录责任、预期行为、适用规则/Contract、风险点和预期验证。Tier 2/3 对 material
Requirement 建立 `requirement -> implementation -> verification evidence` 追踪；公共行为、
Default、Registration、Serialization、Persistence 或 Generation 做有界影响闭包。

Implementation 可以在逻辑子任务、高风险边界或切换模块前冻结 checkpoint。Checkpoint 前
只运行与该分区直接相关、成本合理且已授权的验证。Checkpoint 是可复用 Coverage Evidence，
不是最终批准；后续实质交互变化只失效受影响结论。

## Reviewer Capability

Review capability 只有同时满足以下条件才 eligible：

- 能访问冻结 target identity、完整 assigned partition 和适用规则/Contract；
- 能访问 Acceptance、适用的 Design/ADR/DCR、Coverage 状态和真实 Verification Evidence；
- 审查过程只读，覆盖不是隐式抽样；
- 返回 evidence-backed findings、明确 Coverage、终端结果和 reviewed identity。

能力顺序：

1. `NATIVE_ISOLATED`：宿主原生隔离审查，Reviewer 未参与目标生产或修改；
2. `CHILD_AGENT`：未参与实现的独立只读子代理；
3. `SELF_REVIEW`：Producer 冻结目标后的专用只读检查，只是能力降级诊断。

默认正式 Delivery Gate 保留 Producer/Reviewer 分离：`SELF_REVIEW` 不具备独立 Reviewer
authority，不能把自己描述成独立审查。独立能力不可用时，`critical` 和
`independent_required` 返回 `outcome: UNAVAILABLE`。只有 Orchestrator 已记录的 `prototype` 或
`standard/best_effort_self_review` 政策才允许自审形成受限 Evidence；结果必须明确
`review_mode: SELF_REVIEW`、非独立性和政策来源，且不得覆盖项目规则、Material Contract、
Security 或 Production 边界。

适用 Lanes 在宿主支持且分配清晰时可以并发；否则由同一个 eligible Reviewer 串行执行。
Lane concurrency 不等于 Delivery Review independence，Lane 之间使用不同 Agent 也不是独立性
前提。正式独立性始终判断 Implementation Producer 与 Code Delivery Reviewer 是否分离。

## Full Scope 与 Partitioned Review

小型/中型变更使用一次 `FULL_SCOPE`。大型/多模块变更：

1. 按模块、包、责任或风险边界分区；
2. 每个分区都有 target identity、Reviewer mode、Coverage 和结果；
3. 合并根因相同 Finding，但不得隐藏 P0/P1；
4. 执行 Integration Review，覆盖 Contract、调用关系、状态传播、Schema/API 对齐、兼容、
   Finding 解决状态和 Coverage gap。

Integration Reviewer 不必逐行重读已被新鲜分区审查覆盖的 Diff，但必须读取足够底层代码和
Evidence 验证边界。不能从样本或摘要推导完整 Coverage。

```text
ReviewCoverageComplete =
  AllMaterialPathsCovered
  AND AllApplicableLanesCovered
  AND AllRequiredValidationCompleted
  AND RequiredIntegrationReviewPassed

DeliveryReviewPass =
  ReviewCoverageComplete
  AND Freshness = FRESH
  AND No P0/P1
```

`AllApplicableLanesCovered` 要求 `coverage.applicable_lanes` 与 `coverage.lanes` 的键完全一致，
每个 Lane 都有终端结果；每个 `partitions[].applicable_lanes` 也必须与该分区 `lanes` 的键完全
一致，使 Lane×Partition 结果可机械对账。任何必需 Validation 为 `BLOCKED` 或 `UNAVAILABLE` 时，
`AllRequiredValidationCompleted` 为 false，Review 不得返回 PASS。

## Judge Pass

所有适用 Lane 完成后，Judge 只读取 Finding candidates、相关 Evidence、Acceptance、Relevant
Diff 和 Previous Findings，不重新执行一次完整 Review。它按顺序：

1. 应用通用 NOT-Flag 与 Finding 准入规则，先 Hard Suppress 无 material interaction 的旧问题、
   充分主防御上的 defense-in-depth、风格偏好、库替换、假想未来抽象、无 Acceptance/风险依据
   的补测建议和无证据的猜测；`TIER_3_DEEP + security + potential P0/P1` 的未闭合安全候选先走
   Security Validation Pass，其他无真实触发路径的理论风险仍抑制；
2. 去重并按根因合并跨 Lane 候选，保留最能说明可观察后果的证据；
3. 对不确定候选读取必要代码或 Contract 挑战证据，无法高置信确认时不升级为 Finding；
4. 重新分类并归一严重度，再与 Previous Findings 对账；
5. 输出最终 Findings、Coverage、remaining risks 和 `review_result`。

Judge 偏向按证据通过：P0/P1 导致 `REWORK`，P2 导致 `PASS_WITH_CONDITIONS`，P3 默认省略。
多个 P2 不能仅因数量升级为 P1；只有证明它们属于同一根因且组合后形成可观察的正确性、
安全或 Contract 风险时才能重新分类。Judge 只裁定 `review_result`，不是 SDLC Judge，不能把
Task 设为 Done 或批准 Delivery Gate。

## Finding 与按需复审

缺陷候选必须结合权威 Context、当前变更因果、触发路径、可观察后果、调用/测试和反证验证。
没有 Finding 配额，`No findings.` 是合法结果。首次审查到此为止；仅在 Producer 修复后按需读取
[Incremental Re-review](review/incremental-rereview.md)，不得把 `FIXED`、`UNFIXED`、`DISPUTED`
等复审规则默认带入首次 Review Context。

## Review Waiver

用户明确要求跳过当前 Delivery Unit 的默认 Review 时，Reviewer 只记录请求范围、实际
Verification、已知 Blocker、剩余风险和仍适用 Gate。豁免不能伪造 `review_result: PASS`，
也不能覆盖独立适用的仓库、CI、平台、安全或 Human Gate；只有 Orchestrator/Owner 按 Gate
权限矩阵决定是否允许 `gate.status: WAIVED`。
