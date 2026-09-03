# `context-docs` Lane

**Activate**

- High materiality：Package Manager、Test Framework、Build System、Directory Layout、Required
  Environment、CI Command 或 Language/Toolchain 发生变化；
- Medium signal：Major Dependency、Lint Rule、API Client、State/Data Access Convention 发生 material
  变化，且存在相关开发文档或仓库指令可能失真的具体信号；
- 普通 Feature、Bug Fix、CSS 或沿用既有约定的改动不触发该 Lane。

**Inspect**

- 适用的 `AGENTS.md`、README Developer Section、Build/Test Commands 和 Repository Conventions；
- 指令、路径、命令、工具版本及必需环境是否仍与当前仓库事实一致；
- 文档承诺是否覆盖已改变的开发者操作边界，且没有要求无关 Artifact。

**Do NOT flag**

- 与当前 Change Signal 无关的文档历史债务、措辞偏好或格式问题；
- Low-materiality 改动中的 AGENTS/README 全量新鲜度审查；
- 仅因代码变更存在而要求同步文档，或要求记录内部实现细节。

**Escalation signals**

- 现有指令会让 Agent/开发者运行错误命令、使用错误工具链或遗漏 Required Environment；
- 目录、构建、测试或 CI 现实已改变，而权威上下文仍明确给出相反要求；
- 文档漂移会直接导致 Verification 不可信、交付失败或后续自动化误操作。
