# `release` Lane

本 Lane 只审 Implementation Releasability：当前 Delivery Unit 的代码、迁移、配置或产物是否自带
可证实的发布风险。完整 Rollout Plan、值班、发布窗口、Canary/Progressive 策略与 Runbook 属于
`release-planning` / `release-review`。

**Activate**

- Delivery Unit 真正改变 Runtime、Deployment、Production Configuration 或 Migration；
- Artifact Identity/Permission、运行版本共存或启动/恢复语义发生 material 变化；
- 普通内部实现、测试、UI/CSS 或不影响发布面的文档变更不触发该 Lane。

**Inspect**

- Build/CI Trigger、Permission、Identity、Toolchain、Cache 和 Artifact Provenance；
- 当前实现的 Migration/Schema 兼容、启动失败传播、恢复语义及新旧版本共存；
- Production Configuration Default、Secret、环境差异和运行时可观测失败信号。

**Do NOT flag**

- 缺少 Release Plan、Runbook、Canary、Progressive Rollout、完整发布窗口或值班 Owner，除非当前
  Task 明确拥有该 Artifact；
- 没有发布面变化时的通用运维、Telemetry 或 Rollback 建议；
- 项目规模不需要的 Control Plane、Circuit Breaker、Review Runtime 或模型路由基础设施；
- 与当前交付无关的 CI/Deployment 历史问题或“最佳实践”清单。

**Escalation signals**

- 当前代码会造成不可恢复 Migration、错误权限/Secret 暴露、启动失败或新旧版本不可共存；
- 当前配置/产物身份会使部署后无法安全恢复程序状态；
- 修复本身需要修改生产配置、权限、依赖、部署契约或执行发布动作，但当前任务未获相应授权。
