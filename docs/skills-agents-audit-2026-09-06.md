# 项目 Skills / AGENTS.md 审计

日期：2026-09-06。性质：规则与上下文审计，不是 TASK-011 Delivery Review，不修改冻结契约或 Gate。

## 结论

建议针对触发描述、条件上下文和停止条件做减法，保留现有 SDLC 的授权、证据与独立验收边界。没有依据删除整套流程、降低项目 Profile、替换模型或取消人工验收。

发现 3 项明确的上下文契约不一致，以及 4 项可能增加误触发、重复读取或过早停工的指令问题。下述优先级用于规则维护排序，不表示产品安全漏洞或当前 Task Gate 失败；性能影响尚未做运行对照。

## 文章依据

已通过浏览器读取 Eric Provencher 的原文 [Rethinking skills and prompts for GPT-6 Astra](https://x.com/pvncher/status/2095991462416490862)，页面显示发表于 2026-09-05。网页抓取工具返回 403，随后浏览器成功读取正文；结论不依赖搜索结果中的二手摘要。

文章建议：技能描述只保留清楚、简短的触发条件；多模式技能以最小入口按需加载细则；重新审视旧模型需要的逐步配方和每次修改前的大量阅读；准确表达可自主完成的工作与真正需要审批的边界；完成条件应覆盖运行、检查和修复。文章也提醒仓库技能可能供不同模型使用，因此不能把当前主模型的能力当作所有执行者的共同前提。

这些是审计方法，不构成变更 Veyra 已批准需求或人工 Gate 的授权。

## 清点与覆盖

- 项目自有文件：1 个 `AGENTS.md`，11 个 `SKILL.md`；未发现项目自有的嵌套 `AGENTS.md` 或 `AGENTS.override.md`。
- `.agents/skills/` 共 70 个文件：51 个 Markdown、14 个 YAML、5 个 TOML。已全文审阅全部 11 个技能入口及全部 11 个 `agents/openai.yaml`；支持文档按发现追踪阅读，不声称所有支持文件均完成逐行语义审阅。
- 检查了项目实际启用的路由 Profile 与 4 个角色 TOML，以及恢复、上下文、Change Control、实现、QA、Delivery Unit、Review、复审和验证规则。
- 对技能包全部 Markdown 中的静态相对链接做路径检查：74 个引用，未发现缺失文件。此检查不验证锚点、外部 URL 或运行行为。
- 文件清点包含隐藏与忽略路径，排除 `.git`、`node_modules`、Rust `target` 生成目录；用户全局技能不属于此次项目文件审计。会话提供的全局规则仅用于检查有效指令冲突。
- 工作树原有 `.sdlc/tasks.yaml` 修改与已暂存 Settings Contract 均保留。本轮仅新增本报告。

## 发现

### F1 / P2：派生 HANDOFF 被提升为不可缺失的执行前提

证据：`implementation/SKILL.md:15`、`requirement-review/SKILL.md:19`、`task-breakdown/SKILL.md:16`、`technical-design/SKILL.md:17` 均强制要求 HANDOFF；`sdlc-orchestrator/references/context-protocol.md:53` 规定 Required 缺失返回 Blocker。与此同时，`sdlc-orchestrator/SKILL.md:19` 明确 HANDOFF 是可重建摘要。

触发：摘要文件缺失，但 State、Task、Design 和 Evidence 完整。当前组合可能让 Specialist 因丢失缓存而停止，而不是恢复有效事实。

建议：权威 Task、批准和目标身份仍必需；HANDOFF 改为存在且新鲜时复用，缺失时由 Orchestrator 从权威引用重建。缺少真正的交付基线或批准证据时仍应停止相关动作。

实际上下文负担：当前 HANDOFF 为 22,250 bytes / 46 行，多行压入大量历史，包含连续的“优先于历史”记录；Task 正文为 37,397 bytes / 403 行。行数不能代表上下文很小。建议后续只在 HANDOFF 保留当前有效结果与历史 Evidence 引用，不删除历史证据，也不在此次审计改写 Task 或冻结设计。

### F2 / P2：QA 的 Show Case 必需条件前后不一致

证据：`qa-review/SKILL.md:20-21` 无条件把 Show Case Evidence 列为 verification 的 Required Context；同文件 `:54-56` 与 `sdlc-orchestrator/references/phases/qa-release-and-observation.md:23-34` 仅在 QA 级别或项目规则适用时要求。

触发：light/standard QA 没有声明 Show Case 时，入口仍可能错误阻塞或要求补造演示证据。

建议：将 Show Case 改为 Conditional Required，条件直接引用当前 QA 深度或项目要求。Veyra 当前已声明的真实 UI 验证和人工验收不因此减少；本项主要是技能通用分支的契约缺陷。

### F3 / P2：Implementation 的设计上下文被标为 Optional

证据：`implementation/SKILL.md:26-28` 将 Task 引用的 Design/ADR/DCR 列为 Optional；`:38-45` 又要求设计授权满足且引用新鲜；共享 `context-protocol.md:64` 已将 referenced Foundation/Design/ADR 列为 Conditional Required。

触发：Task 有冻结设计时，执行者可能依据 Optional 跳过必要设计约束，或者为解决矛盾反复查找。

建议：入口与共享协议统一：仅 Task 或当前 concern 实际引用的设计小节必读；不要求读取全部 Design。TASK-011 已批准的 Design 和 Acceptance 必须继续作为实现依据。

### F4 / P2：触发描述过长，并混入职责、流程与禁令

证据：11 个 description 去掉外围引号与换行后合计 3,782 字符；最长的 Orchestrator 原始字段约 568 字符。本会话的可用技能目录已可见多个描述被截断，部分尾部排除条件不可见。不能仅凭此归因截断全部来自本项目，但可以确认这些描述在实际目录中没有完整呈现。

`sdlc-orchestrator/SKILL.md:3` 同时包含“仓库存在 state.yaml 即触发”和“只读解释不触发”；`technical-design/SKILL.md:3` 枚举 persistence/concurrency 等宽泛对象；`qa-review/SKILL.md:3` 包含宽泛 regression 触发。尾部限制被截断时，前部宽触发更容易抢占普通修复或测试任务。

建议：description 只回答何时选择该技能；把状态枚举、输出要求和禁止事项留在正文。下表给出逐技能候选，不自动应用。

### F5 / P2：每次编辑对账与单步路由措辞容易造成重复工作、提前结束

证据：`implementation/SKILL.md:65` 要求每次修改前对账 Anchor/Scope/Acceptance/Design；实现 phase 文档 `:55-64` 再次列出相同步骤；`sdlc-orchestrator/SKILL.md:91-106` 将一个 bounded action 后报告状态写成循环。

这些文本不等于明确要求每次重新读文件，但缺少“已加载且未变化时复用”和“内部路由结束不等于本轮工作结束”的明确区分。

建议：进入 Task、收到新要求、边界变化或上下文失效时重新对账；同一已核验范围内持续实现、运行必要验证、修复并进入独立 Review。当前 Task 的人工验收是有效停止点，首次实现或单次工具返回不是。

### F6 / P2：局部边界措辞比统一授权规则更绝对

证据：`AGENTS.md:26` 将依赖、主机网络和权限等与 commit/push 并列禁止，除非“明确要求”；`.codex/agents/sdlc-implementer.toml:8` 对需求、权限或架构不确定统一要求返回问题；Orchestrator 的默认规则则允许推断和已批准范围内的 L0/L1 决策。

风险：已批准设计包含的必要依赖或平台操作，可能被重新当作未授权动作；可以从代码确定的普通实现细节也可能被上报阻塞。本轮没有观察到由这句话单独导致的真实停工，属于可执行措辞风险。

建议：明确“当前请求或已批准 Task/Design 已覆盖的动作无需重复确认；真正超出授权的依赖、网络、权限、生产或破坏性变更才停下”。角色约束同步采用相同的实质不确定性标准。保留 commit/push/release 的现有授权边界。

### F7 / P3：三轮修复上限缺少到达上限后的持续推进条件

证据：`code-delivery-review/SKILL.md:115`、`references/review/incremental-rereview.md:3-5` 和实现 phase 文档 `:99` 均规定默认最多三轮。

收益：防止同一失败机械循环和无限扩大 Scope。风险：仍在批准范围、持续取得进展的修复也可能仅因计数停止。

建议：将次数上限的含义明确为重新诊断根因与策略的检查点；是否继续由进展、剩余失败和授权边界决定。独立复审、证据新鲜度与人工 Gate 保持不变。该项涉及流程政策，仅作为建议，当前上限未变更。

## 逐技能结论与 description 候选

路径均相对于 `.agents/skills/`；候选仅用于展示可以缩短到何种粒度，不是替换后的正式规则。

| 技能 | 当前入口行数 | 审计结论 | description 候选 |
| --- | ---: | --- | --- |
| sdlc-orchestrator | 185 | 保留治理职责；收窄触发，复用已验证上下文；初始化和旧版本迁移细节进一步按需加载 | Coordinate an adopted SDLC project's current task and gates, or initialize SDLC when explicitly requested. |
| implementation | 110 | 修正 Required/Conditional；普通实现与补证据模式分开按需读取；明确持续推进终点 | Implement, fix, or verify the current approved SDLC task within its frozen scope and design. |
| task-breakdown | 90 | 保留 current/future 分离；HANDOFF 改为可恢复；不因精简提前物化后续 Task | Plan the current SDLC task or materialize the orchestrator-selected future task. |
| requirement-review | 83 | refinement/formal-review 可按模式加载；保留已给定需求不默认重审 | Refine an SDLC requirement baseline, or independently review it when explicitly requested. |
| technical-design | 90 | 将触发从技术对象改为尚未解决的重大决策；保留首次 Foundation 与变更的区别 | Design an SDLC foundation or resolve a task's material engineering decision before implementation. |
| technical-design-review | 81 | 职责清晰；主要缩短描述，保留独立性和目标身份 | Independently review an SDLC design candidate before freezing or refreezing it. |
| code-delivery-review | 144 | 已有按 Lane/语言懒加载；入口重复了较多协议细节，可引用收敛；保留完整覆盖 | Independently review a frozen SDLC task delivery against acceptance and verification evidence. |
| qa-review | 81 | 修正 Show Case 条件；三种模式用短路由分流；避免普通测试误触发 | Design SDLC QA cases, independently review cases, or verify an acceptance target against approved cases. |
| release-review | 64 | 已按 concern 加载，无需扩大删改；精简描述即可 | Independently assess a specific SDLC release target's readiness before approval. |
| post-release-review | 77 | 指标按适用性要求是合理边界；保留观察窗口和证据不足语义 | Evaluate a deployed SDLC release after its defined observation window. |
| sdlc-codex-setup | 99 | 显式调用且 allow_implicit_invocation=false，保留；安装/更新/停用可按模式加载 | Install, update, or disable project-local SDLC Codex routing only on explicit request. |

当前路由使用 Sol/Terra 角色。文章指出不同模型可能需要不同指导，因此建议精简冗余和歧义，而非删除可验证契约或一键改成 Astra。未执行路由安装、模型切换或收益测试。

## AGENTS.md 结论

项目文件只有 35 行，内容以项目特有事实为主，不属于需要大规模重写的通用指令墙。

保留：Clash Verge Rev 的明确 UI 映射、字体资产和许可、浅深主题要求；Veyra 与 sing-box 的验证责任；DCR-017 的取代关系；用户改动保护；真实 child/外部资源检查；零测试不能记 PASS；实际 UI 操作不能由构建替代。

建议仅微调：

1. `:22` 的恢复入口改为“恢复 SDLC 实现时读取 State 和当前 Task，Requirement 只读所需小节；身份变化时追踪受影响内容”。当前需求正文 5,126 行 / 75,508 bytes，不应把每次只读审计、拼写修正或 UI 小改都解释成完整重读需求。
2. `:26` 补上已有授权例外，见 F6。
3. `:30-35` 保留仓库命令与真实资源边界，将命令表明确为按改动选择；验证通过且没有新变更、失败或疑点时结束。不要因文章示例而宣称本仓库所有本地测试都是无外部资源的 disposable fixtures。

会话全局 CodeGraph 指令中的“信任索引”也应与当前目标新鲜度共同理解：工具不可用或索引明确过期时需要可用的文件读取路径。项目本地 `AGENTS.md` 未包含该段，本轮不修改全局文件，也不把它计作第二个项目文件。

## 建议实施顺序与边界

先修 F1–F3 的入口契约不一致，再缩短 11 个 description，最后按模式提取重复细节并澄清 F5–F7。无需新增 Skill、Runtime、缓存或状态根。

若以后实施规则修改，验证应覆盖：只读审计不触发实现；批准后的 TASK-011 不重复请求批准；已引用设计必读；light QA 不要求未声明 Show Case；HANDOFF 缺失可恢复；测试失败能在原 Scope 内修复；独立 Review 和 Human Gate 仍有效；TASK-012～TASK-017 不会提前启动。这些是建议验证场景，本轮未运行行为评测。

本轮静态相对引用检查通过；`git diff --check` 通过（已有 tasks.yaml 的 LF/CRLF 提示不作为失败）。未运行产品构建、内核、主机网络操作或测试；此次审计不产生这些 Gate 的 PASS。

当前 `focus: TASK-011`、Settings Contract、future plan、已批准 Design/Acceptance 与 Delivery Gate 均未修改。TASK-011 的产品完成状态不由本报告判定。
