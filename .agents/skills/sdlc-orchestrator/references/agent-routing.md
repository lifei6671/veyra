# 可选 Agent 路由

本协议只在用户安装并启用 Codex 路由配置后加载。目标是减少完成并验收当前 Task 的总用量和
总耗时；它是 Soft Enforcement，不是新的调度 Runtime、权限系统或生命周期状态源。

## 入口与配置

读取项目 `.codex/sdlc-agent-routing.toml`：`version = 1`、`enabled` 为布尔值，
`max_parallel` 为正整数；`roles` 必须提供 `routine`、`implementer`、`expert`、`reviewer`。
角色值对应项目 `.codex/agents/*.toml` 中唯一的 `name`。只加载本次候选角色文件；它必须有
`name`、`description`、`developer_instructions`、明确的 `model` 与 `model_reasoning_effort`。
模型与推理参数唯一来源是这些角色文件；本协议不维护第二份映射。

缺少配置或 `enabled = false`：继续普通工作流，不要求用户安装。配置格式不合法、角色缺失或
重复、模型/推理不受当前宿主支持：报告具体问题并使用普通 Skill 路径，不自动补写配置，
不静默替换模型。普通路径仍必须满足原有 Review/Gate；能力不足按既有规则报告。
不读取其他项目、用户全局或非 Codex 宿主的同名文件来推断本项目已启用。

## 先选工作，再判断能力

每个有边界的动作分别判断，不能仅用整个 Task 的大小或 Skill 名称分级：

| 执行要求 | 选择 |
|---|---|
| 一次工具调用、已知文件读取、既定命令执行、很小的局部修改，交接成本高于收益 | 当前 Agent 直接完成 |
| 契约完整、影响局部、低风险、验证明确，且有足够工作量值得交接 | `routine` |
| 需要业务理解或跨文件实现，但架构和验收已明确 | `implementer` |
| 并发、权限、安全、持久化、公共协议、生命周期或复杂推理 | `expert`，或已有足够能力的主 Agent |
| 正式独立审阅 | 未参与实现的 `reviewer`，执行现有审阅 Skill |

“补测试、DTO、日志、CRUD、搜索”不是低风险证明。测试设计和探索同样按契约明确度、影响范围、
错误后果、验证充分性判断；简单查询优先已有索引/工具。需求歧义和授权缺口先按现有协议处理，
不能通过派给 expert 取得授权。显然复杂的工作直接选足够能力，不逐档试遍。
主 Agent 当前模型由宿主控制；Skill 不尝试自行换主模型。

角色描述执行能力，不替代 Specialist：例如 routine 和 expert 都可能执行 `implementation`，
审查必须运行对应 Review Skill 和适用 Lane。保持现有 Context Contract 和 `skill_result`。

## 派发与并行

只有当前 Task 内无前置依赖、无写冲突、可以清晰验收的工作才并行。使用
`max_parallel`、宿主可用槽位和项目限制中的最小值；模板默认两项，按上限使用并不意味着必须填满。
每条写入路径只有一个执行者；等相关工作返回并整合后再冻结目标、执行独立 Review。
子 Agent 不再自行扇出，Orchestrator 统一分配槽位。不要为运行已有测试命令再启动一个 Agent。

按本会话真实工具 schema 选择派发方法，不猜参数：

1. 宿主提供 named role 选择器：使用配置指定的角色，并确认宿主加载的是项目角色文件。
2. 没有角色选择器，但工具支持显式 `model` / `reasoning_effort`：读取角色 TOML，将对应值
   显式传给工具，并把角色的 `developer_instructions` 作为任务约束一并交接。
   `task_name` 只是任务名称，不等于选择了 TOML 角色。不得谎称 prompt 具有 developer 层优先级。
3. 若完整历史 fork 不允许覆盖模型，选工具支持的无历史或有限历史模式，并明确交接所需 Context。
   不为继承无关历史而放弃已选择的模型，也不传入工具不支持的参数。
4. 两种方法均不可用：报告本会话无法执行可选路由，继续普通 Skill 路径；不得伪造角色生效。

派发包只包含：代理名称、任务定义、执行动作、允许/禁止范围、相关规则及文件引用、当前目标身份、
Acceptance/Verification、预期结果。必须携带 Specialist 的 Required/Conditional Context；
不要复制整段聊天、整个仓库或所有 Skill。主 Agent 已完成的探索直接交接定位与证据，
子 Agent 只补当前缺口。独立 Reviewer 用未参与实现的上下文，从源目标核验而非只相信作者摘要。

## 失败与升级

- 任务清楚但实现未满足既定验收：保留 Diff、失败证据、已尝试动作与剩余问题，重新判断能力。
  同一有边界动作最多升级一次；仍失败则回到 Orchestrator 分析具体阻塞，不循环换模型。
- 环境、工具、权限或依赖不可用：使用现有 `UNAVAILABLE` / `BLOCKED` 语义，不按能力不足升级。
- Requirement、Scope 或 Frozen Design 冲突：走现有澄清/Change Control，不能换模型继续越界。
- Reviewer 的有效负面结论：修复 Finding 后按原协议复审，不能更换 Reviewer 寻求通过。

升级只影响执行能力，不清空已有 Findings，不重置交付单元，也不重置既有修复次数上限。
整合、验证与 Review 保持必要覆盖；主 Agent 依据实际 Diff 和 Evidence 核验交付，避免重做已完成
且可追溯的整段探索。作者不能充当独立 Reviewer；更强模型不能代替人工验收或直接写 `DONE`。

## 验证收益

安装完成、请求了某模型、宿主实际使用某模型是三件事。只在实际委派时，把以下简短信息附在
现有 Evidence/HANDOFF 的执行摘要中，不改 `skill_result`、Gate 枚举或新建流程状态根：

- 为什么委派/选择该角色、角色配置身份、请求的模型与推理强度；
- 宿主可观察到的实际模型/推理及证据来源；不可观察时写 `UNAVAILABLE`，不采用模型自述；
- 可获得的总用量、总耗时、返工/升级次数，以及该 Task 的验收结果。

总用量要包含主 Agent、子 Agent、Review 和返工；订阅额度和 token 数分开记录，不能用 API
标价推算订阅额度。没有可比基线或宿主计量时，不声称节省比例。先用少量代表性任务比较普通路径
与可选路由；质量和验收标准保持一致。配置存在或测试语法通过都不证明宿主路由与节省效果已验证。
