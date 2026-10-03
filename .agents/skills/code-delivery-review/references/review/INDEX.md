# 审查关注面

根据当前改动选择相关内容，先读 `universal-not-flag.md`，再按需加载下表文件。关注面只帮助发现问题，不要求固定角色、审查轮次或流程工件。

| Lane | 文件 | 激活信号 |
| --- | --- | --- |
| `correctness` | [correctness.md](correctness.md) | 行为、逻辑、错误处理、状态或副作用 |
| `security` | [security.md](security.md) | Auth、Permission、Trust Boundary、Secret、Crypto、不可信输入 |
| `contract-data` | [contract-data.md](contract-data.md) | API、Protocol、Schema、Persistence、Migration、Material Config |
| `concurrency-performance` | [concurrency-performance.md](concurrency-performance.md) | 并发、锁、生命周期、资源所有权、真实 Hot Path |
| `verification` | [verification.md](verification.md) | 所有实现变更的 Acceptance Coverage、失败/边界路径和测试质量 |
| `context-docs` | [context-docs.md](context-docs.md) | Toolchain、构建/测试框架、包管理、目录布局、必需环境或 CI 命令变化 |
| `release` | [release.md](release.md) | Runtime、Deployment、Production Config 或 Migration 的代码可发布性变化 |

审查深度与实际风险相称；只在发现有关联的新风险时补读对应内容。是否委派、是否需要独立审查，以当前用户要求和仓库规则为准。
