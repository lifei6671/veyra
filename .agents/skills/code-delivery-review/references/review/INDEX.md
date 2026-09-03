# Review Lane Index

Review Planner 先只读取本索引，根据 `Tier + Change Signals` 选择 Lanes；随后只加载
`universal-not-flag.md` 与实际选中的 Lane 文件。未选中的 Lane 不进入 Review Context。

| Lane | 文件 | 激活信号 |
| --- | --- | --- |
| `correctness` | [correctness.md](correctness.md) | 行为、逻辑、错误处理、状态或副作用 |
| `security` | [security.md](security.md) | Auth、Permission、Trust Boundary、Secret、Crypto、不可信输入 |
| `contract-data` | [contract-data.md](contract-data.md) | API、Protocol、Schema、Persistence、Migration、Material Config |
| `concurrency-performance` | [concurrency-performance.md](concurrency-performance.md) | 并发、锁、生命周期、资源所有权、真实 Hot Path |
| `verification` | [verification.md](verification.md) | 所有实现变更的 Acceptance Coverage、失败/边界路径和测试质量 |
| `context-docs` | [context-docs.md](context-docs.md) | Toolchain、构建/测试框架、包管理、目录布局、必需环境或 CI 命令变化 |
| `release` | [release.md](release.md) | Runtime、Deployment、Production Config 或 Migration 的代码可发布性变化 |

风险深度决定阅读深度和上下文投入，不决定固定 Lane 数量：

- `TIER_1_FOCUSED`：通常选择 `correctness` 和 `verification`；
- `TIER_2_STANDARD`：选择 `correctness`、`verification`，再选择所有 material 的 Domain Lane；通常为 0–2 个。
  若需要超过两个独立 Domain Lane 或跨越多个系统边界，Planner 应考虑 `TIER_3_DEEP`；
- `TIER_3_DEEP`：深入运行所有**适用** Lane，但不得因为 Tier 高而加载无关 Lane。

Lane 是内部关注面，不是固定 Skill、Agent 或并发要求。支持独立子代理时互不依赖的 Lane 可以并发，
否则串行执行；这不改变 Producer/Reviewer 的独立性要求。

Lane 发现新的 material signal 时只返回 Review Planner；由 Planner 扩展 `selected_lanes`、加载新增
Lane 并刷新 Coverage，Lane 不得自行越界审查未选 concern。
