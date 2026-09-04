---
name: sdlc-orchestrator
description: Govern long-running software delivery from a new idea or an existing requirement baseline, using a repository-owned intent anchor, rolling tasks, risk-based escalation, and evidence-backed delivery. Use when the user starts or resumes a multi-stage software project, explicitly adopts this workflow, works in a repository already containing .sdlc/state.yaml, or asks to coordinate sustained delivery. Do not use for one-off code fixes in an unadopted repository, standalone code review, read-only explanation, or prose-only planning with no request to govern delivery.
---

# SDLC Orchestrator

把 Agent 的长期交付变成 **Intent-driven、风险升级、证据交付** 的可恢复协议。V0.2 是
**Soft Enforcement**：依靠仓库协议、Git/Test 等现有工具、Evidence 和独立 Reviewer 约束
Agent；不提供自研 CLI、后台 Runtime 或绝对强制能力。

## 不可变边界

1. `anchor` 是最高优先级的稳定意图：Goal、Must-have、Non-goals、Constraints、Critical
   Invariants、Acceptance References 和 Requirement Source。每个 Task 开始时静默做 Drift
   Check；仅在实现偏离 Anchor 时停止并报告。
2. `.sdlc/state.yaml` 是唯一流程事实源；`.sdlc/tasks.yaml` 保存滚动索引，只有当前
   `.sdlc/tasks/TASK-xxx.md` 保存完整 Task 正文，future stub 只保存轻量规划信息；
   `memory/HANDOFF.md` 是可重建的恢复摘要。仅在 ingest 的 Material clarification Blocker 期间，
   它可保存 source-bound derived Intake Snapshot；该快照不是真实源，缺失时允许回读 Requirement。
   聊天或 Specialist 输出都不能覆盖这些事实源。
3. Orchestrator 独占状态迁移权。Specialist 只产出工件、Evidence、Blocker 和 `skill_result`，不能把 Task、Story、Gate 或 Phase 标为完成。
4. **默认行动，不默认澄清**：L0 局部实现决策自主完成；L1 可逆工程决策按需求和仓库
   约定选择最小风险方案并记录假设；只有 L2 产品语义、公共契约、权限/安全、不可逆数据、
   生产/破坏性或外部成本决策才请求确认。
5. `Unknown != Blocker`：依次 infer → assume + record → proceed；只有无法推断、会实质
   改变产品语义或风险、且不低成本可逆的不确定性才设为 `BLOCKED`。
6. Gate 必须由当前且匹配目标的 Evidence 支持。未运行或无法运行的检查分别记录为 `NOT_RUN`、`UNAVAILABLE`，不得写成 PASS。
7. 当前 Task 的 Scope 是实现边界。任何已接受 Requirement/Acceptance 变化、或新增未确认的
   Foundation/Material dependency，都必须先停止并请求用户明确确认；标准库、仓库已有已批准依赖、
   低影响 dev/test dependency 与可逆开发工具按 L1 规则处理，项目可显式要求 `confirm_all`。
   其他工程事项不要按“配置、数据库”等对象类型机械询问；仅当
   变化跨 Scope、改变公共/持久化/运行/运维契约、触及权限安全、生产/破坏性/外部成本，或
   违反适用项目规则与 Frozen Design 时停止并请求 Change Control。Frozen Design 冲突还必须创建 DCR。
8. `继续`、`继续下一步`、`开始吧`只授权推进当前已批准范围。若当前唯一待决项是刚展示的
   Requirement Baseline，用户的明确同意可确认该 Baseline。若当前唯一 Human pending 是本轮刚展示、
   已有当前独立 Review、identity 未变化的 Foundation，同样可确认该 Foundation；它绝不代表 Release、
   生产、破坏性 Migration 或未列出的 Material dependency 等其他高风险授权。
9. Release 不等于自动 Done。只有已定义发布后窗口、技术/业务指标或实验结论时，才必须完成
   Observation；没有观察要求时，以发布 Evidence 和其他适用子流程闭合当前交付目标。
10. Context is a budget。默认只读完成当前决策所需的最小上下文；引用优于复制。
11. 用户用于否决或纠正方案的会话措辞、本轮未采用方案和临时草稿只作为生成控制信息，
   不自动成为最终工件的身份或叙事中心；纠正后确认的目标必须进入 canonical accepted
   state。终稿从该状态和实际读回结果生成，真实边界、风险、失败、变更与审计事实仍须保留。

## 选择运行路径

先定位项目根目录并检查 `.sdlc/state.yaml`：

- **已采用项目**：先执行“恢复”，然后从 Anchor、Focus、风险和 Evidence 决定下一步。
- **用户显式调用 `$sdlc-orchestrator` 或明确采用/初始化**：该调用授权创建最小 `.sdlc/`
  工作集；若用户同时明确“按需求开始开发”，才授权当前 Scope 内必要的 L0/L1 工程动作。
  两者都不扩大为 Material Contract、权限安全、生产、破坏性或明显外部成本操作的授权。
  - 从想法开始走 `author`：最多一轮集中澄清后可将 Candidate Requirement Baseline 暂存为
    `.sdlc/REQUIREMENT.md`；只有用户确认后才把它绑定为 canonical Source 并建立 Anchor。
  - 用户提供需求文档并要求按其开发走 `ingest`：首次必须完整读取该文档并完成轻量 Readiness
    Pass；只集中询问会改变产品语义或验收的实质缺口，不默认重做 Formal Requirement Review。
  - 判断工程上下文是 `greenfield` 还是 `established`：只有当前 Scope 可复用已有 stack、布局/模块、
    build/test 约定和架构边界时才是 established。Greenfield 在 Task Breakdown 前形成唯一
    `.sdlc/design/foundation.md` 并请求一次技术基线确认；Established 沿用可验证的现有架构。
- **语义自动触发**：可以进入只读发现和采用预览，但不能跳过写入确认。
- **未采用的一锤子任务**：不得静默创建 `.sdlc/` 或改变项目流程，按普通交付流程处理。
- **一锤子修复或独立 Code Review**：不接管完整生命周期。使用项目已有的实现/审查流程。

初始化、恢复或修复状态时，读取 [adoption-and-recovery.md](references/adoption-and-recovery.md)。判断 Mode、Anchor、Phase、Profile、Workstream、Gate 或迁移时，读取 [workflow-protocol.md](references/workflow-protocol.md)。

## 恢复：只加载最小工作集

按以下顺序读取，足够完成当前决策就停止：

1. L0：`.sdlc/state.yaml` 与 `.sdlc/memory/HANDOFF.md`。
2. 在读取 L1 前重算并比较 Requirement Source identity；合法的 author pending-baseline 状态
   （`PROJECT_INIT`、空 Accepted Source、Requirement Gate `PENDING`）跳过 Accepted Source
   比较，但必须重算 `.sdlc/REQUIREMENT.md` 的 Candidate identity。Candidate 变化时清空该 Gate
   Evidence、保持 `PENDING` 并以当前 identity 请求 Human approval；ingest clarification Blocker
   则先比较 HANDOFF Intake Snapshot 的 source identity：匹配时复用并合并用户回答，缺失或不匹配时
   允许重新完整读取 canonical Requirement Source；其他不匹配先进入 Change Control。
3. L1：`.sdlc/tasks.yaml` 与 current `task_ref` 指向的 focus Task Markdown；future stub 不上钻为
   完整 Task Context。
4. L2：当前阶段允许的 Story 验收、Design 小节、ADR、QA/Release 工件。
5. L3：只有具体问题无法从索引解决时，才读取完整源工件或更大代码范围。

构建或压缩工作集时读取 [context-protocol.md](references/context-protocol.md)。创建、校验或更新工件时读取 [artifact-protocol.md](references/artifact-protocol.md)。

若 HANDOFF 或聊天与 canonical artifact 冲突，以 `state.yaml` 和被引用的源工件为准，重建
HANDOFF，不得反向篡改事实源以迁就摘要。检测到 V0.1 或过渡版 State 时，先读取
[legacy-v01-migration.md](references/legacy-v01-migration.md) 完成一次性归一化；主 Router 不直接执行旧字段。

## Orchestrator 循环

每次只完成一个清晰的路由或状态决策：

```text
Inspect state and minimal context
  -> Validate state and references
  -> Reconcile Git/artifacts/evidence when needed
  -> Drift-check the focus task against the Anchor
  -> Run Task Readiness Check before implementation
  -> Evaluate only applicable Gate or risk boundary
  -> Route one bounded action
  -> Validate SkillResult and observed Evidence
  -> Invalidate stale downstream Gates if inputs changed
  -> Request Human Gate when required
  -> Transition state only when authorized
  -> Rebuild memory/HANDOFF
  -> Report status and next action
```

不要用一个长 Prompt 同时模拟所有角色。Design、QA、Release 和 Observation 是按适用性进入
的子流程，不是每个项目的必经阶段。根据当前动作只读取一个专业 Playbook：

- Discovery / Requirement Review：读取 [discovery-and-requirements.md](references/phases/discovery-and-requirements.md)。
- Technical Design / Design Review：读取 [technical-design.md](references/phases/technical-design.md)。
- Delivery Planning / Implementation：读取 [planning-and-implementation.md](references/phases/planning-and-implementation.md)。
- QA / Release / Observation：读取 [qa-release-and-observation.md](references/phases/qa-release-and-observation.md)。

若平台支持子代理，只在当前 Task 内把无前置依赖、无写冲突的子功能或验证并行化。不得并行开发
后续 Task；当前 Task 必须独立验收并 `DONE` 后才能推进下一 Task。给每个子任务明确代理名称、
任务定义、执行动作、边界和预期结果；所有结果返回后由 Orchestrator 统一校验，子代理不得直接改状态。

模型路由是可选安装能力。普通使用、复制 Skill 或初始化 `.sdlc/` 都不安装 Agent 配置。
仅在 Codex 宿主且项目存在 `.codex/sdlc-agent-routing.toml` 时，读取
[agent-routing.md](references/agent-routing.md) 检查是否启用，再决定当前动作是否值得委派。
缺少配置或 `enabled = false` 时沿用普通 Skill 路径，不要求 named Agent 或特定模型；
既有项目指令要求的并行、独立 Review 和人工验收仍然有效。只有用户主动请求安装/更新/停用时，
才调用可选的 `sdlc-codex-setup` Skill；不得在恢复或任务执行时自动安装、修复或升级配置。

## Gate 与 Evidence

进入任何 Gate 前，先定义所需证据，再执行验证。读取 [evidence-and-results.md](references/evidence-and-results.md) 处理 Evidence、Reviewer、SkillResult 和最终报告。

- 自动检查必须记录真实命令、退出码、目标身份和结果摘要。
- 成功只保留结论；失败保留足够诊断的末尾输出，不保存无界完整日志。
- 测试通过不能替代语义 Review；Review 通过也不能证明测试运行过。
- Reviewer 默认读取 `Task + Acceptance + Diff + Evidence`，并只在 concern 实际适用时加载
  `Design/ADR/DCR`，不得无目的读取整个仓库。
- 有条件时优先使用未参与实现的只读 Reviewer，并要求它返回覆盖范围、目标身份、Finding 和 Gate 结果。

## 变化、越界与阻塞

需求、设计、Task Scope、实现或发布前提变化时，读取 [change-control.md](references/change-control.md)：

- 先做 Impact Analysis；已接受需求变更和 Foundation/Material dependency 必须获得用户明确确认，
  再更新事实源；Verification strengthening/equivalent 与低影响 dev/test dependency 按 L1 处理。
- 根据变化类型失效下游 Gate；不得只改文档然后继续使用旧批准。
- 冻结设计发生冲突时停止实现，创建 DCR，重新评审并冻结后再恢复。
- Scope 外修改必须保持未执行，除非用户明确扩展当前 Delivery Unit 或创建新 Task。
- Blocker 解除后回到 `blocked.origin_phase`，并设回 `blocked: null`；不得猜测下一阶段。

## 写回顺序

状态变化后按以下顺序写回，避免派生文件先于事实源：

1. 更新相关 canonical artifact（通常是当前 Task Markdown；窗口或里程碑变化时才更新
   `tasks.yaml`；按需才是 Requirement、Foundation/Design、ADR/DCR、QA、Release 或 Evidence）。
2. 评估并记录 Gate 失效/通过结果。
3. 仅由 Orchestrator 更新 `.sdlc/state.yaml`。
4. 最后从事实源压缩更新 `.sdlc/memory/HANDOFF.md`。

Reviewer 的 P2 输出只能是 `ADVISORY/FOLLOW_UP` recommendation。只有有权 Owner 明确决定后，
Orchestrator 才能在 Review Evidence 中记录带 `accepted_by/accepted_at/evidence_ref` 的
`ACCEPTED_RISK` resolution，或在对应 milestone 写入按需 `follow_ups`；两者都不自动创建 active
Task。

写回必须由事件触发：Phase/Task/Scope/Blocker/Gate/Decision/Evidence/Next Action 发生实质变化时才更新。不要把每轮聊天、思考过程或完整命令输出写入仓库。

## 状态报告

默认只报告用户继续推进所需的三项：

```text
Focus:
Result:
Next:
```

仅在实际发生验证时增加 `Verification:`；仅在存在时增加 `Blocker:` 和
`Approval required:`；仅在风险发生变化时增加 `Risk:`。只报告实际结果，没有 Evidence 时写
`NOT_RUN`；环境无法运行时持久化 `UNAVAILABLE`。`UNABLE_TO_VERIFY` 仅可作为最终用户表面说明，
不得写入 Evidence、SkillResult 或 Gate；存在未满足 Gate 时，不得用“完成”“已通过”“可发布”掩盖。

最终回复和交付包装应聚焦已接受结果、实际验证状态、Blocker 与下一步，不为无会话历史的
读者解释无关的被弃方案，也不附加“已避免某项”之类自证声明。生成或更新标题、文件名、
元数据、commit、PR、报告与 HANDOFF 时，遵循
[Evidence 与 SkillResult 协议](references/evidence-and-results.md)的终稿表面规则。
