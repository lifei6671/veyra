# Context Protocol

## 原则

Context is a budget。每次加载都必须服务于一个具体决策、实现或验证问题；事实优先引用，
不要在 Requirement、Task、HANDOFF 和 Evidence 中复制同一段说明。

## 四级上下文

### L0 — Project Snapshot（目标不超过 1K tokens）

- `.sdlc/state.yaml`
- `.sdlc/memory/HANDOFF.md`

用于回答当前 Focus、Blocker 与 Next。HANDOFF 目标 300–800 tokens，只保存 Current、
Completed、Remaining、Blocker、Next 和少量 canonical 引用，不保存工作日志或完整聊天。仅在 ingest
因 Material clarification 被阻塞时，允许一个临时 source-bound derived Intake Snapshot：Requirement
Source/identity、Readiness、已解析 Goal/Scope/Non-goals/Constraints/Acceptance refs 摘要和 open
questions。它是可丢失的派生缓存，不是事实源：identity 匹配才可复用，缺失或不匹配时上钻重读
canonical Requirement，Anchor 建立后立即删除。
未解决 Finding 少时，从当前 Review Result/Evidence 派生保存
`finding_id/fingerprint/severity/status/target_identity/evidence_ref`；超过预算时只保存 compact
review Evidence 引用。写入前必须确认引用跨会话可解析；否则先持久化 compact review Evidence。
HANDOFF 不得成为 Finding 的独立事实源。

### L1 — Current Working Set（目标不超过 4K tokens）

- `.sdlc/tasks.yaml`
- 当前 focus Task
- 当前 Task 所需的命令、Scope、Acceptance 与 Verification

用于开始一个明确的 Task 或 Gate 工作。不要为恢复上下文创建额外缓存文件；上述 Intake Snapshot
是既有 HANDOFF 的受限例外，不创建新文件或 State 字段。

### L2 — Specialist Context（目标不超过 8K tokens）

只加载当前专业工作需要的 Requirement 小节、Task 引用的 Design/ADR/DCR、QA/Release/Experiment
定义、当前 Diff、Evidence 摘要和必要失败诊断。

### L3 — Source Artifacts（按需）

只有 L0–L2 无法回答具体问题时才读取完整 Requirement、更多历史 Evidence 或更大源码范围。
说明上钻原因；问题解决后不要把全部 L3 内容复制进 HANDOFF。

### Initial Ingest Exception

首次 `ingest` 时 Requirement Source 不是 L3 Optional Context，而是 Baseline Required Context。
允许分块读取，但必须覆盖全部需求章节后才能建立 Anchor、完成 Readiness Pass、判断
`greenfield/established` 或拆 Task。该例外只用于首次 intake；恢复仍按 L0→L3 渐进加载。

## Specialist Context Contract

每次路由只加载一个 Specialist 的契约。`Required` 缺失时返回 Blocker；`Conditional Required`
只在 concern 已声明适用时成为必需项；`Optional` 只在有具体问题时加载；`Forbidden Default`
不是永久禁止，但必须先说明上钻理由。

| Specialist | Required Context | Conditional Required Context | Optional Context | Forbidden Default Context |
| --- | --- | --- | --- | --- |
| discovery | User goal, material uncertainty, project constraints, repository snapshot | none | focused market/technical facts, selected source files | all code, all history, all evidence |
| requirement-review | selected mode, Anchor draft, Requirement Source or author input, constraints; formal-review also needs frozen identity/producer | Experiment draft if enabled | relevant domain facts, Experiment draft | historical Tasks, all ADR, QA history, unrelated code |
| technical-design | selected mode, Anchor, Requirement Source, constraints | foundation: full Requirement + repository snapshot; task-boundary/remediation: current Task + relevant architecture | existing Story, Design Index, relevant ADR/DCR, focused research | unrelated QA/history, all Tasks, all Evidence |
| technical-design-review | frozen affected Design target, Requirement/Acceptance trace, affected concerns | ADR/DCR if the concern has one | focused repository facts, risk-specific evidence, Design Index when multi-file | entire repo, unrelated concerns/Story, producer chat |
| task-breakdown | Anchor, Requirement Source, tasks.yaml | existing window Task Markdown; applicable Foundation and applicable Design/ADR/DCR | risk facts, existing test commands | unrelated Epics/Stories, full Evidence history |
| implementation | State, HANDOFF, tasks.yaml, resolved focus Task Markdown, selected execution mode | referenced Foundation/Design/ADR and approved Show Case steps when applicable | relevant code/tests, existing Story acceptance | all Epic/Story/Task/ADR/Evidence |
| code-delivery-review | Task and Acceptance, Delivery Unit baseline/inventory/provenance, frozen identity, coverage manifest, delivery-owned content, actual Evidence | relevant Design/ADR/DCR when present | related Requirement/Story acceptance, risk-specific source/tests, material neighboring context, fresh checkpoints | entire repo, full PRD, arbitrary Git targets, unrelated changes/history |
| qa-review | selected mode, current acceptance target, Task, applicable risk | mode-specific Cases/identity/Evidence | existing Story, focused Design/ADR, failure diagnostics | unrelated business background, all source/history |
| release-planning | release target/scope, affected operational concerns | current QA status, ADR/DCR, capacity facts, runbook conventions when applicable | none | production credentials, unrelated source code, unrelated Stories/history |
| release-review | Release Plan and target identity | Deployment, Migration, Rollback, dependency, compatibility, config, capacity, QA, monitoring, alerting, runbook, flag/experiment evidence only for declared applicable concerns | focused runbook/config schema/evidence | business source code, unrelated Stories/ADR |
| observation | release identity, observation window/status, metric definitions, approved data sources | Experiment identity/definition/data-quality only when enabled | current metric/evidence collection status | implementation internals, unrelated history, unapproved production mutation |
| post-release-review | release identity, completed observation window, declared applicable metric definitions/baselines/current data | business metrics, Experiment evidence, Incident/Rollback/Customer impact only when applicable | focused incident/evidence records | implementation internals, unrelated history |

## 摘要策略

- 成功命令：保存命令、退出码、结果、目标身份和计数摘要。
- 失败命令：额外保存足以定位问题的尾部诊断；不要保存无界 stdout。
- ADR 与 Design：引用当前 Task 实际需要的文件，不复制完整 rationale 或整个 Design。
- HANDOFF 只传播已接受状态、有效约束、真实结果及其引用；唯一例外是 clarification Blocker 期间的
  source-bound derived Intake Snapshot，且它不能覆盖 Requirement Source、Anchor 或其他 canonical artifact。
- 未进入 canonical artifact 且对后续无持续价值的会话级弃案不得进入跨会话摘要；已成为
  `non_goals`、`scope.deny`、ADR/DCR、Finding、迁移、兼容性、诊断、Evidence、Blocker 或
  provenance 的必要事实必须保留引用，不得静默删除。

`current.md`、`DAILY.md`、`LESSONS.md` 不属于 V0.2 工作集。旧项目中它们只可按
[legacy-v01-migration.md](legacy-v01-migration.md) 作为一次性低权威迁移输入，归一化后停止读写。

预算是目标，不是 tokenizer Gate。正确性、安全或契约判断需要更多上下文时可以上钻，但必须
有明确理由并保持范围聚焦。
