# Evidence and Skill Results

## Evidence 是观察，不是自述

每条 Evidence record 必须关联自己的 `target` 和 `target_identity`，例如 Git HEAD + delivery-owned diff、Design version/hash、Artifact revision 或部署批次。Record 一旦写入不得通过改写身份重新归属；目标变化时追加新 record，并让 Gate 通过身份比较把旧 record 视为 stale。命令文本和“PASS”字符串本身不证明执行过。

结果词汇固定：

- `PASS`：检查真实运行完成，相关断言成功。
- `FAIL`：检查真实运行并失败。
- `NOT_RUN`：已识别但未执行，记录原因。
- `UNAVAILABLE`：因工具、服务、权限、依赖或环境不可用而无法执行。
- `STALE`：Evidence 的目标或重要前提已经改变。

不要把 `NOT_RUN`、`UNAVAILABLE`、`STALE` 翻译成“应该没问题”。

`UNABLE_TO_VERIFY` 仅是最终用户表面的解释词，不是机读状态：不得写入 `Evidence.result`、
`skill_result.outcome` 或 `gate.status`。无法运行检查时 Evidence 使用 `UNAVAILABLE`，已识别但未执行时
使用 `NOT_RUN`，State/Gate 输入冲突时使用 `STALE` 或 `BLOCKED`；用户报告可同时说明
`UNABLE_TO_VERIFY` 及其对应机读状态。

## 选择验证命令

按权威顺序发现：

1. 当前路径适用的 `AGENTS.md` / `AGENTS.override.md`；
2. CI、Makefile、Taskfile、package scripts、语言清单和项目文档；
3. 没有约定时，才选择该技术栈最小必要检查。

运行前检查陌生命令及副作用。不要为通过 Gate 擅自安装依赖、修改锁文件、运行真实迁移、访问生产或写外部系统。

## Evidence 压缩

成功时保存：

```yaml
id: EVIDENCE-017
type: test
target: TASK-017
target_identity: <current target>
produced_by_phase: DELIVERY
command_or_method: go test ./internal/agent/...
exit_code: 0
result: PASS
timestamp: <ISO-8601 timestamp>
summary: 4 packages passed
observer: agent-or-ci-identity
```

失败时可以额外保存有限 `diagnostic_tail`；成功记录不需要该可选字段。完整原始日志应留在宿主、CI 或外部 Artifact 中并只保存引用；不要把几千行 stdout 放进项目 Context。

## Evidence 存储策略

当前 Gate 的短生命周期 Evidence 可保存在对应 canonical Gate/Task 的有界 compact record 或引用中；
只有需要跨 Task、长期审计或被 Completed Task 的 `evidence_refs` 引用时，才懒创建并持久化到
`.sdlc/evidence/`。不得为每次命令或每个 Task 预建 Evidence 目录；持久化后仍只保存摘要和外部
日志引用。

## SkillResult

Specialist 使用统一结果：

```yaml
skill_result:
  skill: implementation
  target: TASK-017
  outcome: PASS
  signals:
    implementation_complete: true
  next_action_hint: code-delivery-review
```

约束：

- `outcome` 只描述当前专业任务，不代表 Gate 或生命周期通过。
- 不提供 `recommended_transition`；`next_action_hint` 非权威。
- `architecture_change` 只表示改变已接受/Frozen Architecture，`scope_change` 只表示改变已接受 Scope；
  两者为真时必须进入 Change Control，不得继续迁移。首次 Foundation/首份未冻结 Design 不是
  `architecture_change`，可用非权威 `signals.design_kind: foundation` 标识。
- `issues` 是 Specialist 发现的非阻断问题；`blockers` 只放阻止当前专业任务的项。
- `artifacts_changed` 和 Evidence 必须能与 Git/文件系统事实对账。
- 结果使用 Sparse 形状：`artifacts_changed`、`evidence`、`issues`、`blockers` 为空时省略；
  `architecture_change`、`scope_change` 为假时省略。缺失 collection 等于 `[]`，缺失 bool 等于
  `false`。`skill`、`target`、`outcome`、`next_action_hint` 始终必填。

## Reviewer

Producer 与 Reviewer 分离。Review 第一次只读，不直接修被审对象。优先选择未参与实现且能访问冻结目标、规则、Acceptance、Diff 和 Evidence 的 Reviewer。

所有正式 Reviewer 结果在基础 `skill_result` 之外固定包含：`review_mode`、
`reviewed_target_identity`、`coverage`、`remaining_risks`、`review_result`。身份缺失是 Required
Context 缺失，必须返回 `outcome: BLOCKED`，不能用 `UNAVAILABLE` 代替；`UNAVAILABLE` 只用于
工具、权限、依赖或环境不可用。

最小 ReviewContext：

```text
Task
+ Acceptance
+ Design/ADR/DCR when referenced/applicable
+ delivery-unit baseline and complete inventory/provenance
+ delivery-owned Diff and untracked/generated artifacts
+ coverage manifest and frozen target identity
+ actual verification Evidence
```

Reviewer 必须返回：

- review mode；
- reviewed target identity；
- covered paths/responsibilities；
- review strategy、applicable Lane results、required Validation、partition results、integration result 和 freshness；
- evidence-backed findings（P0/P1/P2/P3）；
- `review_result`：`PASS`、`PASS_WITH_CONDITIONS` 或 `REWORK`；
- remaining verification/coverage risk。

Reviewer 同时遵循统一 `skill_result.outcome`。审查已完成时 `outcome=PASS/FAIL`
并携带 `review_result`；目标、上下文或覆盖不足时使用 `outcome=BLOCKED` 且
`review_result=REWORK`；工具或环境不可用时使用 `outcome=UNAVAILABLE`，此时
`review_result` 为 `null`，不得伪造审查结论。

测试通过不代替 Review。Review `PASS` 也不代替 Build/Test/QA Evidence。
`PASS_WITH_CONDITIONS` 只能携带明确、可追踪且不阻断当前 Gate 的条件；任一 P0/P1
或覆盖不完整都必须 `REWORK`。

Code Delivery Review 的 P2 条件还必须由 Reviewer 声明
`disposition_recommendation: ADVISORY | FOLLOW_UP`。Reviewer 无权声明风险已经被接受。
只有有权 Owner 明确决定后，Orchestrator 才能在 Review Evidence 中写入
`resolution: ACCEPTED_RISK`，并同时记录 `accepted_by`、`accepted_at` 和 `evidence_ref`。
采纳 `FOLLOW_UP` 时只在 `tasks.yaml` 对应 milestone 增加按需 `follow_ups`，不自动创建 active
Task 或 `tasks/TASK-xxx.md`。

`review_mode` 使用 `NATIVE_ISOLATED`、`CHILD_AGENT`、`SELF_REVIEW` 或 `MIXED`。
默认正式 Delivery Gate 要求 Producer/Reviewer 分离；`SELF_REVIEW` 只能作为披露的非权威诊断。
仅当 Orchestrator 已记录 `prototype` 或 `standard/best_effort_self_review` 的适用项目级政策时，
才可作为受限 Delivery Evidence，且必须披露其非独立性；不得用于 critical、项目规则要求独立
审查或 Material Contract/Security/Production 边界。大型/多模块 Delivery Unit 的
`coverage.strategy` 必须为 `PARTITIONED_PLUS_INTEGRATION`，且每个适用 Lane、分区和
Integration 都有匹配身份的新鲜结果；任何必需 Validation 未完成都不得 PASS。

通用 Delivery Review 词汇必须适配到本协议，不得进入 canonical State：

- `PASSED/APPROVED` -> `outcome: PASS` + `review_result: PASS`；
- 带非阻断 Follow-up -> `outcome: PASS` + `review_result: PASS_WITH_CONDITIONS`；
- P0/P1 -> `outcome: FAIL` + `review_result: REWORK`；
- 缺身份、Context、Ownership、Coverage 或 Freshness -> `outcome: BLOCKED` +
  `review_result: REWORK`；
- 工具、权限、依赖、环境或独立 Reviewer 能力不可用 -> `outcome: UNAVAILABLE` +
  `review_result: null`，除非适用的项目级 review policy 已明确允许受限 SELF_REVIEW。

用户请求跳过 Review 不是 Reviewer PASS。只有 Orchestrator/Owner 能按 Gate 权限判断是否记录
`gate.status: WAIVED`，且不能覆盖 P0/P1、CI、平台、安全或 Human Gate。

## 最终表面生成与复检

1. 从已接受的 canonical state、Task Scope 和实际交付结果重新生成最终表面；不要围绕会话级
   弃案逐词替换或继续解释。
2. 检查正文和包装层是否仍有弃案的直接提及、同义转述、括号说明或身份化命名；同时确认
   `non_goals`、`scope.deny`、ADR/DCR、Finding、真实删除与迁移、兼容性、安全与诊断事实、
   Blocker、Evidence 和用户已有改动的 provenance 没有被误删或吸收到交付范围。
3. commit、PR、报告或其他最终表面经外部工具、Hook 或平台创建或改写后，必须读回实际标题、
   正文和 metadata，再对实际结果复检；操作前的草稿不能作为完成证据。
4. 精确词扫描只用于辅助发现残留，不能证明语义无残留，也不能替代事实边界检查和实际读回。

复检发现问题时，只在既有授权内修正对应表面并重新读回；不得借此扩大外部操作权限，或
改写 canonical artifact、Evidence、Gate 或状态机来制造一致性。

## Gate 评估

Orchestrator 检查：

1. 每项必需 Evidence 都存在且非 stale；
2. Evidence 的 target identity 与当前状态匹配；
3. 所有 material paths 和分区覆盖完整；
4. `coverage.applicable_lanes` 与实际 Lane 结果完全对账，且每个分区都有完整 Lane×Partition 结果；
5. 所有 Required Validation 已完成；
6. 大型/多模块目标的 Integration Review 已通过；
7. Target 与 material Review Context 为 `FRESH`；
8. 无 P0/P1；
9. Human Gate 已批准；
10. 下游 Gate 未因新变化失效。

任一条件不满足，Gate 不得 `PASSED`。

## 内部状态与用户可见结果

Machine/Internal Result 可保留完整状态，供 `skill_result`、Evidence、HANDOFF 与 Orchestrator
迁移判断使用：

```text
Implementation: COMPLETE | IN_PROGRESS | BLOCKED
Verification: PASS | FAIL | NOT_RUN | UNAVAILABLE | STALE
Review: PASS | PASS_WITH_CONDITIONS | REWORK | NOT_RUN
Gate: PASSED | FAILED | PENDING | STALE | WAIVED
Transition: APPLIED | NOT_AUTHORIZED | NOT_READY
```

User-facing Result 默认只输出：

```text
Focus:
Result:
Next:
```

只有对应事件实际发生时，才追加 `Verification:`、`Review:`、`Blocker:`、`Approval required:` 或
`Risk:`。`target_identity`、完整 Evidence、命令、Gate transition 默认不展示；仅在用户要求、
发生失败/阻塞，或当前是高风险 Gate 时展示最小必要部分。内部字段不得机械泄露成每次对话的
CI 报告。
