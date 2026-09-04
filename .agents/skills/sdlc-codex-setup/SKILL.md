---
name: sdlc-codex-setup
description: "Install, update, or disable the optional project-scoped Codex model routing for SDLC only when the user explicitly requests setup. Ordinary SDLC use or copying the skill collection does not activate this skill or install host configuration."
---

# SDLC Codex Setup

仅在用户主动要求安装、更新或停用 SDLC 的 Codex 模型路由时使用。首版只提供 Codex
适配；其他宿主继续按普通 Skill 工作流运行，不写 Codex 配置。不要据此断言其他宿主
没有子 Agent 能力。复制本 Skill 或整套 `skills/` 只使安装入口可用，不代表启用路由。

安装是 Agent 引导的项目文件复制，不引入 Runtime、CLI、依赖或全局默认值。用户明确
要求首次安装时，即授权使用下列默认映射；已有具体授权不重复确认。

## 安装内容

读取 [共享路由规则](../sdlc-orchestrator/references/agent-routing.md)，再按以下映射准备
目标项目的五个文件。源路径均相对于本 Skill，目标路径相对于用户指定的项目根目录。

| 源 | 目标 |
|---|---|
| `assets/codex/sdlc-agent-routing.toml` | `.codex/sdlc-agent-routing.toml` |
| `assets/codex/agents/sdlc-routine.toml` | `.codex/agents/sdlc-routine.toml` |
| `assets/codex/agents/sdlc-implementer.toml` | `.codex/agents/sdlc-implementer.toml` |
| `assets/codex/agents/sdlc-expert.toml` | `.codex/agents/sdlc-expert.toml` |
| `assets/codex/agents/sdlc-reviewer.toml` | `.codex/agents/sdlc-reviewer.toml` |

默认模型映射以角色 TOML 为唯一来源，不在工作流 Skill 内复制模型名。Profile 是本工作流
读取的可选设置，不是 Codex 原生配置：`version = 1`、`enabled = true`、`max_parallel = 2`，
`[roles]` 将 `routine / implementer / expert / reviewer` 映射到对应 Agent `name`。
`max_parallel` 由 Orchestrator 在宿主上限以内执行，不会修改宿主并发设置。

## 完整预检

首次安装、更新和重新启用都先完成预检，预检失败不写任何目标文件：

1. 确认用户目标项目，读取该项目的规则、现有路由 Profile 和上述四个目标角色文件。
   项目不明确且无法从当前工作区确定时，只询问目标路径。解析目标绝对路径；遇到链接或
   重解析点时确认解析后仍在所选项目内，不能借安装写入其他项目或用户全局目录。
2. 确认宿主是 Codex，目标项目可使用当前 SDLC Skill 集合及共享路由规则。配置安装本身
   不安装整套 Skill，也不初始化 `.sdlc/`。缺少 Skill 时给出缺项和已有安装入口。
3. 优先检查当前宿主提供的工具签名和本地文档；不足时核对
   [Codex 官方子 Agent 文档](https://learn.chatgpt.com/docs/agent-configuration/subagents)。
   检查当前支持的模型、推理强度、角色选择和上下文继承方式。模板是初始配置，不能保证
   每个账号或客户端都支持；任一所选模型或推理强度明确不可用时，报告具体不支持项，
   不静默换模型。无法确认可用性时记录 `UNAVAILABLE`，不启用未经确认的映射。
4. 必须确认至少一种实际派发路径可用：
   - **Named role：**宿主真正支持选择自定义 Agent 时，按 TOML 的 `name` 选择角色。
     工具中的 `task_name` 只是任务标签时，不能把它当作角色选择参数。
   - **显式参数：**宿主支持 `model` 与 `reasoning_effort` 覆盖时，由 Orchestrator 读取
     角色 TOML，将 `model_reasoning_effort` 映射为工具的 `reasoning_effort`，并在交接
     提示中传入该角色的 `developer_instructions`。没有角色选择器不妨碍采用此路径。
   - 若完整历史 fork 不允许覆盖模型或推理强度，使用工具实际支持的 `fork_turns = "none"`
     或有限历史模式，并明确传入必要 Context；不能将不合法覆盖和完整 fork 一起提交。
   没有适用派发能力时保留普通工作流，说明安装未执行。不要通过新建用户任务、自研
   调度器或更改权限来绕过宿主限制。
5. 解析所有源和目标 TOML，检查 Profile 版本、布尔开关、正整数并发值、完整角色映射、
   唯一的 Agent `name`、所需 `description / developer_instructions / model /
   model_reasoning_effort` 字段。检查宿主可见的项目/个人 Agent 是否已有同名定义，防止
   角色遮蔽；只读取相关定义，不复制或泄露其他配置。
6. 一次比较全部五个目标文件。内容相同视为 no-op，缺失文件可安装；任一内容不同、
   无法解析、同名角色冲突或未知版本，先保留全部目标文件，列出具体路径、字段差异和
   建议修改。仅当已有授权覆盖该差异，或用户批准该具体修改后，才继续写入。

## 写入与验证

- 预检通过后，创建必需的 `.codex/agents/` 目录。只写五个明确目标文件，不写
  `.codex/config.toml`、全局配置、权限、安全模式、环境变量或 `.sdlc/` 状态。
- 首次安装先写四个角色，逐一解析、读回并确认内容，再最后写入启用的 Profile。
  中途失败时不写启用 Profile，报告已写/未写文件；下次安装按同一预检流程处理残留。
- 更新已启用配置时，在获授权的更新开始前先将现有 Profile 的 `enabled` 设为 `false`；
  然后更新并读回全部角色，最后写入完整 Profile。任何一步失败都保持停用，禁止继续
  使用半套新旧映射。不要在尚未获得差异授权时提前停用。
- 全部文件完全一致时不重写文件。已停用 Profile 不因重复复制自动重新启用，只有用户
  明确要求启用/重新安装时才把 `enabled` 改回 `true`。
- 若宿主要求重新加载项目或开启新会话才能发现角色，报告该必要步骤；当前会话未发现
  角色时可以使用已确认的显式参数路径，否则继续普通工作流。
- 用户要求停用时，仅将已知版本 Profile 的 `enabled` 改为 `false`，保留角色文件及其他
  字段；已停用或 Profile 不存在为 no-op。停用不需要模型/派发能力可用，未知版本或不可
  解析文件先给出具体修复差异。停用控制 SDLC 自动路由，不删除用户可手动调用的 Agent。

检查安装结果分三层报告，不能用文件存在替代运行证据：

1. **配置验证：**五个文件完整、解析成功、角色引用唯一且读回一致。
2. **派发能力：**实际工具签名支持的路径、模型/推理组合与 fork 限制。仅声明派发参数
   可用，不宣称尚未运行的 Agent 已采用这些参数。
3. **运行验证：**在用户本次安装授权范围内，可使用最小只读派发检查选定路径，记录
   requested role/model/effort 和宿主实际返回的配置或 telemetry。子 Agent 自述不是
   模型生效证据；宿主无法暴露实际模型或推理强度时，此项标记 `UNAVAILABLE`。
   未执行为 `NOT_RUN`，派发失败为 `FAIL`；已知失败时保持/改为停用，不假装安装生效。

只读检查成功但 telemetry 不可观察时，可以报告配置已安装、运行配置验证 `UNAVAILABLE`，
不得写成路由已验证或额度已节省。用量、耗时与返工收益由后续真实任务对照评估。

## 返回

简要报告目标项目、创建/更新/no-op 文件、启用状态、派发路径，以及配置验证和运行验证
各自的结果与证据。失败时列出原文件是否保留、已写文件、缺失条件和最小下一步。
本 Skill 不参与 Phase/Gate 路由，不迁移 Task、Gate、Phase 或 Human Approval。
