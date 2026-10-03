# `verification` Lane

**Activate**

- 默认适用于所有改变可执行行为或 executable contract 的 Delivery Unit；
- 变更只涉及不可执行普通文档且 Review 本身不适用时，不单独启动。

**Inspect**

- Acceptance、风险和 Change Map 是否有对应 Verification Evidence；
- 成功、失败、边界、状态、副作用及适用的并发终止是否被有效断言；
- 命令是否真实执行、结果是否新鲜、环境是否匹配，Evidence 是否足以证明目标行为；
- 测试是否会在目标回归时失败，而非只断言 `NoError`、`NotNil` 或文件存在。

**Do NOT flag**

- 没有 Acceptance、风险或回归路径依据的泛化“增加测试”建议；
- 仅追求覆盖率数字、测试数量或某种测试层级的偏好；
- 项目未要求且不能提高当前结论置信度的重复验证。

**Escalation signals**

- material Requirement 没有可追踪 Evidence，或关键失败路径无法被现有验证捕获；
- Evidence 未运行、失败、过期、目标不匹配，或只证明编译而未证明行为；
- 验证不足时说明哪些问题尚无法确认；工具、权限或环境不可用时准确报告原因和影响，不要求流程状态或结果适配文件。
