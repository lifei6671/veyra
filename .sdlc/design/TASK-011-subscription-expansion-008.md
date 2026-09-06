# TASK-011 订阅扩展 U2 投影与终态定向修订

> 状态：`CANDIDATE`
>
> 模式：`technical-design / remediation`
>
> 目标身份：`TASK-011-subscription-expansion-008`
>
> 本文件不修改旧候选、不批准 Technical Design Gate。

## 1. 修订依据与继承关系

本候选只修复 `.sdlc/evidence/TASK-011/expansion-design-review-007.json`
（`sha256:329d681cc0ddd628efdcc27ec33835ccab3e87b94506ef0c940d866984e95652`）中的：

- `EXP007-DESIGN-003`：projected default target 超出当前 Compiler 支持范围；
- `EXP007-DESIGN-004`：worker reload 失配缺少封闭的 `busy` 观测终态。

它以 `.sdlc/design/TASK-011-subscription-expansion-007.md`
（`sha256:187772921ebce7c7df6a0c9780bd0491421958287fa350a46747d534f376d5ba`）为基线。
本文件第 2 节替换 007 第 2 节第 2 项及相关 Direct/Block 措辞；第 3 节补充 007 第 4 节的错误码与映射；
第 4 节替换对应验收。007 的其它修订及 006 的继承内容保持不变。

真实代码边界为 `src-tauri/src/singbox/compiler.rs:289-302`
（`sha256:6b7fa49e84d0eb2dce8a3a9f47d2a18c20e62b9e04ee1e96129b1720454a1208`）：
`SingBoxCompiler::compile` 的 default target 只接受 `RouteTarget::Pool`，且该 Pool 必须存在于同一
`RuntimeIntent`。本修订沿用该能力限制，不扩展 Compiler。

## 2. Projected default target 仅允许 Pool

`SelectedRuntimeProjection` 仍不可拆分地携带 `runtime_intent`、`projected_default_target`、
selected ID 与 generation；Use 和零参数 Start 仍只消费同一投影结果。其 default 规则收窄为：

1. 持久 `default_target == Unconfigured` 时，投影生成当前订阅的运行时隐式 Pool，并令
   `projected_default_target = RouteTarget::Pool(implicit_pool_id)`。
2. 持久 default target 为显式 Pool，且该 Pool 经当前订阅过滤后仍存在并非空时，保持该 Pool。
3. 持久 default target 为显式 Pool 但过滤后不存在或为空时，preflight 返回
   `selectionConflict`。
4. 持久 default target 为 Direct 或 Block 时，preflight 固定返回 `selectionConflict`。
   不把它传给 Compiler，不自动改为隐式 Pool，也不写回持久 default。
5. 上述 3/4 发生在保存选择和停止旧 child 之前；旧状态、旧 child 与观测身份保持不变。

这里仅限制 **default target**。Route 中现有 Direct/Block 语义继续由既有 Compiler 契约处理，
不因本修订被扩大或改写。

强类型构造完成条件同步收窄：`projected_default_target` 必须匹配
`RouteTarget::Pool(pool_id)`，且 `pool_id` 在同一 `runtime_intent.pools` 中存在并非空。无法满足时返回
`selectionConflict`；内部成对构造不一致仍按 007 的 `configurationFailed` 处理。

## 3. Reload 失配的封闭终态

007 的 `SubscriptionSwitchErrorCode` 增加唯一缺项：

```ts
type SubscriptionSwitchErrorCode =
  | "cancelled"
  | "busy"
  | "stateUnavailable"
  | "configurationFailed"
  | "saveFailed"
  | "stopFailed"
  | "startFailed"
  | "recoveryRequired";
```

worker 在已经发布 `queued` 后，于持久提交前重新取得短 state gate：

- reload I/O 或迁移读取失败：发布
  `{status:"failed", errorCode:"stateUnavailable"}`；
- reload 成功但实际完整状态不等于 request 携带的 expected state：取消 prepared candidate，发布
  `{status:"failed", errorCode:"busy"}`；
- 上述失配的候选清理失败：按既有所有权语义发布
  `{status:"failed", errorCode:"recoveryRequired"}`，不伪造 `busy` 已安全结束。

若同步等待尚未超时，expected-state 失配同时返回
`{status:"error", operationId, error:"busy"}`。若调用方已经收到
`{status:"pending", operationId}`，则只由同一 operation ID 的迟到
`failed/busy` 观测收敛；不得把它改写成超时失败、取消或成功。

`busy` 仅允许和 switch `status:"failed"` 组合。它不得出现在 queued/checking/prepared/persisted/
applying/ready/cancelled；decoder 遇到这些非法组合必须 fail closed。

最终 `SubscriptionSummary` wire 字段统一为 `active: boolean`，只表示持久选择；
`activeConfigurationGeneration` 仅 active 项非 null。006 中 `selected` 一词只描述产品概念，不是第二个
wire 字段；不得同时输出 `active`、`selected` 或 `enabled`。

## 4. 最小验收修订

1. `Unconfigured` 投影得到隐式 Pool default；有效显式 Pool 保持；失效/空显式 Pool、Direct、Block
   均在 Use 和 Stop→Start preflight 返回 `selectionConflict`，Compiler 未被调用，状态与旧 child 不变。
2. 成对投影测试断言传给 Compiler 的 default 必为同一 `RuntimeIntent` 中非空 Pool。
3. operation 已 queued 后，reload I/O 失败精确为 `failed/stateUnavailable`；expected-state 失配精确为
   `failed/busy`；清理失败精确为 `failed/recoveryRequired`。
4. expected-state 失配分别覆盖同步 `error/busy` 与 pending 后迟到 `failed/busy`；两者 operation ID
   相同，旧 operation 终态不得覆盖新 operation。
5. Rust 序列化与前端 decoder 穷举新增 `busy`，并拒绝 `busy` 与非 failed 状态的组合。
6. Summary exact-key-set 只允许 `active` 与 `activeConfigurationGeneration`，禁止同时出现
   `selected` 或 `enabled`。

## 5. 待批准边界

本修订没有增加机制、字段、IPC、权限、依赖或批准项。007/006 所列 V5、DTO 与五个 IPC/main
capability、RuntimeObservation/worker/guard/应用内调度、`qrcode.react ^4.2.0` 和工具生成锁文件仍待
独立复审后一次明确批准。本候选不修改生产代码、canonical Task/state、旧设计或 Gate。
