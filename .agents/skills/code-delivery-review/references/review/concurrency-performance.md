# `concurrency-performance` Lane

**Activate**

- Lock、Thread、Goroutine、Task、Stream、Channel、Transaction 或共享可变状态发生变化；
- File、Socket、Connection 等资源的 Ownership、Cancellation 或 Shutdown 发生变化；
- 真实 Hot Path、复杂度、I/O、内存或连接使用存在 material 变化。

**Inspect**

- 资源 Owner、创建、释放、Failure Cleanup、Cancellation 和 Shutdown；
- Race、Deadlock、Ordering、Backpressure、重复执行和被破坏的不变量；
- Hot Path 上的复杂度、重复 I/O、N+1、无界内存、大复制、阻塞、锁竞争和连接占用。

**Do NOT flag**

- 没有可触发 Interleaving 或受损不变量的并发猜测；
- 非 Hot Path 上没有测量依据或可观察影响的理论优化；
- 仅为未来规模提出 Pool、Cache、并行化、批处理或更复杂同步方案。

**Escalation signals**

- 可构造的 Race/Deadlock/Leak，或 Cancellation/Shutdown 无法有界完成；
- 资源在失败路径失去 Owner，或 Transaction/Lock 生命周期越过预期边界；
- 复杂度或无界资源增长会在已知输入规模下导致明确延迟、内存或可用性问题。
