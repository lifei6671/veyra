# `correctness` Lane

**Activate**

- 默认适用于所有改变可执行行为、状态或错误语义的 Delivery Unit；
- 纯机械生成结果或只提供上下文的文件，不单独触发该 Lane。

**Inspect**

- 正常、边界、空/零/极值和非法输入路径；
- 状态迁移、Caller 语义、错误传播、Timeout、Retry、Rollback 和 Partial Failure；
- 数据一致性、重复副作用、清理失败、失败伪装成功和静默降级。

**Do NOT flag**

- 按构造不可达且框架或类型系统已经保证的内部状态；
- 不改变可观察行为的个人实现偏好或等价重写；
- 缺少具体失败输入、状态或调用路径的泛化担忧。

**Escalation signals**

- 可复现的错误结果、非法状态迁移、数据损坏或重复外部副作用；
- 失败被吞掉、成功被错误报告，或恢复/回滚会留下不一致状态；
- 缺陷跨越模块边界，说明需联动 `contract-data`、`security` 或 `concurrency-performance`。
