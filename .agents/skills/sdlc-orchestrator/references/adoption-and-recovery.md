# Adoption and Recovery

## 首次采用

用户显式选择 `$sdlc-orchestrator` 或明确要求初始化本流程时，该调用授权创建最小 `.sdlc/`
工作集；不必先展示目录清单再等待一次仪式性确认。语义自动触发只授权只读发现和采用预览；
只有用户确认预览后，才允许首次创建 `.sdlc/`。

先读取并完成：

1. 定位项目根和适用的 `AGENTS.md` / `AGENTS.override.md`。
2. 读取 README、当前设计/计划、Git 状态和与目标直接相关的代码或测试。
3. 区分：用户已确认、仓库事实、推断、未知。
4. 确认入口：从想法开始使用 `author`；用户提供需求文档并要求按其开发使用 `ingest`。首次
   ingest 完整读取 Requirement Source 并完成 `READY | NEEDS_CLARIFICATION` Readiness Pass。
5. 判断 `project.engineering_context`：只有当前 Scope 可直接沿用已有 stack、布局/模块约定、build/test
   约定和架构边界时为 `established`；仅有 README、manifest 或需求文档，以及成熟 monorepo 中的新 service，
   均为 `greenfield`。
6. 提取或拟定 Intent Anchor：Goal、Must-have、Non-goals、Constraints、Critical
   Invariants、Acceptance References 和 Requirement Source。
7. 只将 Material Product Uncertainty 放入一个集中澄清；其余可推断或可逆未知记录为 Assumption 后继续。
8. Greenfield 在 Task Breakdown 前创建唯一 `.sdlc/design/foundation.md`，集中确认路径依赖技术
   基线和拟新增第三方依赖；Established 沿用仓库约定，不创建 Foundation。

单纯“采用/初始化流程”只授权治理工作集；用户同时明确“按需求开始开发”时，授权当前 Scope
内必要的内部代码初始化和低影响可逆 L0/L1 工程动作。以下边界仍需单独授权：

- Foundation/Material dependency；Foundation 批准只覆盖其中精确列出的 package、version/range、用途和影响；
  低影响 dev/test dependency 和可逆开发工具可按 L1 处理，项目规则可要求 `dependency_policy: confirm_all`；
- 已接受 Requirement、功能清单、产品语义或 Acceptance 的任何变化；
- Material Runtime/Operational Dependency 或公共/运行/运维契约变化；
- 持久数据语义或 Migration、权限/安全、生产/破坏性动作、明显外部成本；
- 将推断写成 Accepted Decision，或宣称 Requirement、Design、Plan/Gate 已通过。

更近层级的项目规则可以要求更严格的确认；不得用本协议绕过它。

`author` 可把候选 Baseline 暂存于唯一 `.sdlc/REQUIREMENT.md`，但文件存在不等于已经接受。只有
用户明确确认改变产品语义的 Baseline 后，才回读该文件、写入 `source.requirement` 和
`source.identity`，并建立 Anchor。Candidate 形成但尚未确认时，原子地保留
`mode: author`、`phase: PROJECT_INIT`、空 `source`、空 Anchor，并写入
`gates.requirement.status: PENDING` 与该 Candidate identity。`ingest` 直接引用用户已有文档，不创建 `.sdlc/REQUIREMENT.md`。
初始化默认只创建
`state.yaml`、`tasks.yaml` 与 `memory/HANDOFF.md`；不要复制整套空工件，也不要为不适用领域
创建 `NOT_APPLICABLE` 文件。首次 Task Breakdown 才创建 `tasks/TASK-xxx.md`；Greenfield 才创建
`design/foundation.md`；其它 Design、ADR/DCR、QA、Release 与 Evidence 只在确有适用性或长期
审计价值时懒创建。

## Profile 选择

根据规模和风险提出建议，用户可覆盖：

- `prototype`：短期 POC、个人工具、低风险内部验证。保留 Scope、Light Design、Verification 和明确退出条件。
- `standard`：默认。完整 Scope 对账、Delivery Review 和适用验证；需求、设计、QA、发布和观察按风险与产品需要进入。
- `critical`：安全、权限、资金、关键基础设施、持久数据或高风险发布。增加独立安全/性能/迁移 Review、演练与更多 Human Gate。

Profile 只能调整 Gate 深度，不能抹去安全、权限、生产、Material Runtime/Operational
Dependency、公共契约或持久数据语义变化所需的独立授权。

## 已采用项目的恢复

先读取 L0，并在加载 L1 前重算 Requirement Source identity：

1. `.sdlc/state.yaml`
2. `.sdlc/memory/HANDOFF.md`

校验至少包括：

- `schema_version` 和 `workflow_version` 可识别；若是 V0.1 或过渡版，先读取
  [legacy-v01-migration.md](legacy-v01-migration.md) 归一化；
- `schema_version: 2` 但缺少 `project.engineering_context` 或其值仍为 `null` 也属于过渡版，
  必须先按仓库事实归一化并持久化该字段，不能因字段缺失阻塞恢复；
- `tasks.yaml version: 1` 的内联窗口或 `version: 2` 的多 Markdown 引用先按 Artifact Protocol 无损迁移为
  `version: 3` JIT current Task + future stub；future stub 保留 `milestone_ref`，V1 future inline Task
  先物化为只读迁移来源、V2 的既有 Markdown 路径均保留为 `migration_ref`，提升时回读后再执行
  Task Readiness Check；
- `mode`、`anchor`、`phase`、`project.profile`、Focus Task 和实际存在的 Gate 值属于协议允许集合；
  `project.engineering_context` 在新项目尚未完成首次仓库检查时可为 `null`，恢复已采用项目时必须
  归一化为 `greenfield` 或 `established`；
- Focus Task 能在 `tasks.yaml` 定位，`task_ref` 指向的 Markdown 存在且 ID 一致；
- 合法 pending-baseline 状态为 `mode: author`、`phase: PROJECT_INIT`、空
  `source.requirement` / `source.identity`、未形成 Anchor、`focus.task: null`，且
  `gates.requirement` 为带有 Candidate identity 的 `PENDING`。此状态跳过 Accepted Source identity
  校验，但必须回读 `.sdlc/REQUIREMENT.md` 并重算 Candidate identity。若与
  `gates.requirement.target_identity` 不同，不得把旧 pending 直接恢复：清空该 Gate 的 Candidate
  Evidence、以当前 Candidate identity 保持 `PENDING`，再请求 Human approval；这不是 Accepted
  Source Changed，不进入 Change Control；
- ingest Readiness 的合法 clarification 状态使用既有 `blocked`：`origin_phase: PROJECT_INIT`、
  `owner: user`、`reason: material_requirement_clarification`、非空问题 scope 和 unlock condition。
  Blocker 存在时，HANDOFF 必须保存 source-bound derived Intake Snapshot（source、source identity、
  readiness、已解析的 Goal/Scope/Non-goals/Constraints/Acceptance refs 摘要和 open questions）。用户
  回答后，snapshot identity 匹配时合并回答、清空 `blocked` 并回到 origin phase，不得冗余完整 ingest；
  snapshot 缺失或 identity 不匹配时允许重新完整读取 canonical Requirement Source，再建立 Anchor；
- Foundation 的合法 pending 状态是文件存在且 `gates.technical_design.status: PENDING`，其
  `target_identity` 匹配 foundation 当前 identity。适用独立 Review 时，`reviewed_by: null` 表示先
  路由 `technical-design-review`；Review 当前而 `approved_by: null` 才请求 Human approval。identity
  不同则清空旧 review/approval/evidence、保持 `PENDING` 并重新 Review；不得再次调用
  `technical-design` 生成同一 Foundation；
- 除上述 pending-baseline 状态外，`source.requirement` 必须可读，且重算后的 Requirement Source
  identity 必须与 `source.identity` 一致；
- 当前 Task 引用和按需创建的 Design/ADR 引用存在；
- HANDOFF 指向同一 Project/Task/Phase；
- 非空 Blocker 有 owner、reason、next_check 或解除条件；
- 通过的 Gate 能找到匹配 Evidence。

若 observed identity 与 `state.source.identity` 不同，不得静默重建 Anchor 或直接恢复实现：记录
Requirement Source Changed，先进入 Change Control，分析受影响 Anchor、Task、Design、Evidence
和 Gate，把受影响下游项标为 `STALE`；只有分析完成后才可更新 source identity、Anchor 或路由。

状态有效时，用一句话说明恢复到的 Phase、Focus 和 Next Action，然后继续当前请求。不要让用户重复描述已被事实源可靠记录的内容。

## 对账

以下情况触发 Reconciliation：

- HANDOFF、State 或源工件相互冲突；
- Git HEAD、Diff 或关键文件与 Evidence 目标身份不匹配；
- observed Requirement Source identity 与 State 保存的 identity 不匹配；
- Design 版本、Task 引用或 Gate Evidence 缺失；
- 用户说“继续”，但 `next` 已不可执行；
- 新会话发现并发工作或未归属改动。

对账顺序：

```text
User current instruction
  > repository rules and canonical artifacts
  > .sdlc/state.yaml for workflow state
  > observed Git/Test evidence
  > derived HANDOFF
  > chat recollection or model assumption
```

若 `state.yaml` 本身与可验证仓库事实冲突，不要静默选一边。报告冲突、保留原文件、提出最小修复，并在
修复前将相关 Gate 标为 `STALE`；若无法继续验证，在用户表面报告 `UNABLE_TO_VERIFY`，同时以闭合的
`STALE`、`UNAVAILABLE` 或 `BLOCKED` 机读状态记录原因。

## 恢复后的写权限

恢复不等于扩大授权。用户的“继续”只允许执行 `next` 对应的已批准范围；只有本轮刚展示、identity
未变化、独立 Review 当前且唯一 Human pending 的 Foundation 可被解释为该 Foundation 的批准。遇到其它 Human Gate、
已接受需求变化、Foundation/Material dependency、Scope 扩张、公共/持久化/运行/运维契约变化、权限安全、
批量重构、生产发布或破坏性操作时仍需即时确认；低影响可逆的内部选择不因对象类型而自动升级。
