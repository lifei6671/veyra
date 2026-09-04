# Finding 与结果映射

本参考把通用 Delivery Review 术语适配到 Agent-native SDLC 的闭合协议。外部 Verdict
不得进入 canonical State，不得直接写入 `.sdlc/state.yaml` 或替代 Orchestrator Gate。

## Finding 管线

所有 Reviewer Lane 只产生 Candidate；Candidate 不是 Finding，也不能直接影响 `review_result`。
最终 Finding 必须依次通过：

```text
Candidate
  -> Universal NOT-Flag
  -> Security Validation (only when applicable)
  -> Finding Admission
  -> Dedup / Root-cause Merge
  -> Judge
  -> Severity
  -> Emit or Suppress
```

### Universal NOT-Flag

先应用 [Universal NOT-Flag](review/universal-not-flag.md)；不要在本文件复制第二份易漂移的清单。
Hard Suppress 命中时 Candidate 直接抑制。`TIER_3_DEEP + security + potential P0/P1` 的 Needs
Validation Candidate 先做有界 Security Validation Pass：`CONFIRMED` 回到 Finding Admission，
`DISPROVEN` 不输出 Finding。若新证据证明已抑制 Candidate 与当前变更存在可触发、可观察的
material interaction，可重新进入完整管线。

`NEEDS_VALIDATION` 不能成为 Review 的终态。若 Required Context 不足，Coverage 标记
`validation: BLOCKED` 并返回 `outcome: BLOCKED`、`review_result: REWORK`；若工具、权限、依赖或
环境不可用，标记 `validation: UNAVAILABLE` 并返回 `outcome: UNAVAILABLE`、`review_result: null`。
两种情况都不得组合 `remaining_risks + PASS`。

### Finding Admission

未被抑制的 Candidate 必须同时满足：

- Concrete：指出当前变更引入或触发的具体问题；
- Actionable：给出最小可行修复方向；
- Triggerable：说明输入、状态、事件或调用路径；
- Relevant：由当前 Delivery Unit 导致或与其产生 material interaction；
- Evidence-backed：引用代码、Contract、Verification 或可观察后果；
- High confidence：检查反证后，作者大概率会据此修复。

无法确认的 Candidate 继续取证或放入 `remaining_risks`；不要用严重假设抬高置信度。唯一例外是
`TIER_3_DEEP + security + potential P0/P1`：按 Universal NOT-Flag 的 Needs Validation 先确认或反证，
不得因 Trigger Path 尚未闭合而直接抑制。

### Dedup 与 Judge

先按共同根因合并跨 Lane、跨文件表现，再交给 Judge。合并后保留能证明触发路径和影响的
最小证据集，不得因去重隐藏任何可信 P0/P1。

Judge 只接收：

- 通过 Admission 的 Candidates；
- Candidates 对应的 Evidence；
- 当前 Acceptance；
- 与 Candidates 相关的 Diff；
- Previous Findings 及其原 `finding_id`、`fingerprint`、`status` 和 `target_identity`；
- 当前冻结的 `target_identity`。

Judge 负责再次应用 NOT-Flag、挑战证据、校验根因合并、重新分类问题类别，并在最后统一
确定 Severity。Judge 不从头执行完整 Review，不扩展到无关代码，不替代 Reviewer Coverage，
也不读取或修改 canonical State、Task 状态或 SDLC Gate。Judge 的唯一协议输出是最终 Finding
集合及 `review_result` 建议；最终 Gate 仍由 Orchestrator 独占判断。

## 初次审查 Finding 身份

每个最终 Finding 至少包含：

```yaml
finding_id: FINDING-003
fingerprint: auth/service.go:UpdateUser:ownership-check
status: NEW
target_identity: <current reviewed target>
severity: P0 | P1 | P2 | P3
disposition_recommendation: ADVISORY | FOLLOW_UP # P2 only; Reviewer output
location: path:line
confidence: high
problem: ...
evidence_and_impact: ...
suggested_fix: ...
```

- `finding_id` 在 Finding 首次准入时生成；后续复审身份规则仅在按需加载
  [Incremental Re-review](review/incremental-rereview.md) 时适用；
- 首次准入状态为 `NEW`；协议不存在未定义的 `OPEN` 状态；
- `fingerprint` 表达稳定的语义身份，优先由受影响边界、符号和根因组成，不包含行号或
  `target_identity`；
- `target_identity` 随代码变化而更新，但同一根因的 `finding_id` / `fingerprint` 跨 Target 延续；
- `location`、Evidence、Impact 和 Severity 必须基于当前 Target 刷新，不得复用 stale 行号。

P2 必须带一个轻量 `disposition_recommendation`，它只是 Reviewer 建议：

- `ADVISORY`：只保留在 Review Result，不创建 Task；
- `FOLLOW_UP`：建议在当前交付后继续处理，但不自动成为 active Task。

Reviewer 无权输出“风险已经被接受”。只有有权 Owner 明确决定后，Orchestrator 才能在 Review
Evidence 中追加：

```yaml
resolution: ACCEPTED_RISK
accepted_by: <owner identity>
accepted_at: <timestamp>
evidence_ref: <owner decision reference>
```

若 `FOLLOW_UP` 建议被采纳，Orchestrator 在 `tasks.yaml` 对应 milestone 的按需 `follow_ups` 中写入
简短记录，不立即创建 `tasks/TASK-xxx.md`。Recommendation 或 Resolution 都不写 canonical State，
也不改变当前 Delivery Gate 的非阻断语义。

## Severity 与结果

Severity 必须综合可观察 impact、实际可利用性、exposure/reachability、攻击者能力和既有
compensating controls；Impact 单独不得决定 P0/P1。先完成 Finding Admission 与适用的 Security
Validation，再按以下后果分级。

- `P0`：安全绕过、Secret 泄露、数据损坏、不可恢复状态、确定性死锁、重大 Contract 破坏、
  大范围不可用等阻断问题，映射为 `REWORK`；
- `P1`：已证明的正确性缺陷、关键失败路径缺失、资源泄漏、非法状态迁移、关键测试缺口或
  明确兼容回归，映射为 `REWORK`；
- `P2`：非阻断但有明确价值的可维护性、可观测性、局部性能、测试质量或设计改进，映射为
  `PASS_WITH_CONDITIONS`，并带 `ADVISORY` 或 `FOLLOW_UP` recommendation；Owner 可另行产生
  `ACCEPTED_RISK` resolution；
- `P3`：Naming、Comment、Formatting 等 Nit，默认抑制；仅在项目规则明确要求时输出，且不得
  单独改变 `PASS`。

多个 P2 不得仅因数量累积为 P1。只有 Judge 证明它们来自同一根因，且组合后形成可触发、
可观察的正确性、安全或 Contract 风险时，才可把合并后的单一 Finding 重新分类为 P1。

## 通用结果到当前协议的映射

| 通用审查状态 | `skill_result.outcome` | `review_result` | 约束 |
| --- | --- | --- | --- |
| `PASSED`，无 P0/P1/P2 | `PASS` | `PASS` | Coverage `COMPLETE`、Freshness `FRESH`；规则要求输出的 P3 不改变结果 |
| `PASSED`，有 P2 条件 | `PASS` | `PASS_WITH_CONDITIONS` | 条件明确、可追踪且不阻断当前 Gate |
| `BLOCKED`，存在 P0/P1 | `FAIL` | `REWORK` | 交回 Producer 最小修复并重新验证/复审 |
| 缺身份、Context、Ownership 或 Coverage | `BLOCKED` | `REWORK` | 缺 Required Context，不得伪装能力故障 |
| 工具、权限、依赖、环境或独立 Reviewer 不可用 | `UNAVAILABLE` | `null` | 保留具体 blocker，不伪造审查结论 |
| Target/Context stale | `BLOCKED` | `REWORK` | 刷新受影响验证与 Coverage 后再审 |
| 用户请求豁免 | `BLOCKED` | `null` | 只报告请求；由 Orchestrator 判断 Gate 是否可 `WAIVED` |

`PASS_WITH_CONDITIONS` 不能包含 P0/P1 或 material Coverage gap。默认 `SELF_REVIEW` 只形成
非权威诊断；仅适用的 `prototype` 或已持久化 `standard/best_effort_self_review` 政策可把其作为
受限 Delivery Evidence。此时结果必须保留 `SELF_REVIEW`、政策来源和非独立性，且不得用于
critical、项目规则要求独立审查或 Material Contract/Security/Production 边界。

## Coverage 返回形状

```yaml
coverage:
  strategy: FULL_SCOPE | PARTITIONED_PLUS_INTEGRATION
  status: COMPLETE | INCOMPLETE
  applicable_lanes:
    - correctness
    - security
    - verification
  lanes:
    correctness:
      result: PASS | PASS_WITH_CONDITIONS | REWORK | UNAVAILABLE
      validation: NOT_APPLICABLE
      freshness: FRESH | STALE
    security:
      result: PASS | PASS_WITH_CONDITIONS | REWORK | UNAVAILABLE
      validation: NOT_APPLICABLE | COMPLETE | BLOCKED | UNAVAILABLE
      freshness: FRESH | STALE
  partitions:
    - id: backend
      paths_or_responsibility: []
      applicable_lanes: [correctness, security, verification]
      lanes:
        correctness:
          result: PASS | PASS_WITH_CONDITIONS | REWORK | UNAVAILABLE
          validation: NOT_APPLICABLE
          freshness: FRESH | STALE
        security:
          result: PASS | PASS_WITH_CONDITIONS | REWORK | UNAVAILABLE
          validation: NOT_APPLICABLE | COMPLETE | BLOCKED | UNAVAILABLE
          freshness: FRESH | STALE
        verification:
          result: PASS | PASS_WITH_CONDITIONS | REWORK | UNAVAILABLE
          validation: NOT_APPLICABLE
          freshness: FRESH | STALE
      target_identity: <identity>
      review_mode: NATIVE_ISOLATED | CHILD_AGENT | SELF_REVIEW
      result: PASS | PASS_WITH_CONDITIONS | REWORK | UNAVAILABLE
      freshness: FRESH | STALE
  integration_result: PASS | PASS_WITH_CONDITIONS | REWORK | NOT_APPLICABLE
  freshness: FRESH | STALE
```

`applicable_lanes` 必须与 Review Plan 的最终 `selected_lanes` 一致，并与 `lanes` 的键完全相等；
不能只从已返回的 Lane 反推应运行的 Lane。Partitioned Review 还必须证明每个 Lane 覆盖它被分配的
所有适用分区：每个 `partitions[].applicable_lanes` 必须与同一分区 `lanes` 的键完全相等，并为
每个 Lane 返回终端结果。`status: COMPLETE` 仅在所有 material paths、适用 Lanes、必需 Validation
和必要 Integration 都完成且新鲜时成立。

Reviewer 返回该结构和 Finding；Orchestrator 再结合自动 Evidence、Human Gate、Scope/Architecture
flags 和当前 State 判定 Delivery Gate，不直接采用任何外部 `APPROVED` 词汇。
