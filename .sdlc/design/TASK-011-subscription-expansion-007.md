# TASK-011 订阅扩展 U2 切换设计定向修订

> 状态：`CANDIDATE`
>
> 模式：`technical-design / remediation`
>
> 目标身份：`TASK-011-subscription-expansion-007`
>
> 本文件不修改旧候选、不批准 Technical Design Gate。

## 1. 修订依据与适用关系

本候选定向修复 `.sdlc/evidence/TASK-011/expansion-design-review-006.json`
（`sha256:ad1cd971e99a7f9ac556226dbe08853b0fdcc7167ac3fa97841b4d88978516ac`）的
`EXP006-DESIGN-001` 与 `EXP006-DESIGN-002`。

它以 `.sdlc/design/TASK-011-subscription-expansion-006.md`
（`sha256:34efae0995185f8117c1cea289813ba3cb54e29dddaaab608e6ed5f8158c1d59`）为基线：

- 本文件第 2 节替换 006 第 3 节中投影返回值与 default target 的描述；
- 本文件第 3–5 节封闭 006 第 5、7、9、11、12 节涉及的 IPC、Start 结果、观测状态与验收；
- 006 的其余 V5、事务提交点、DCR-004 失败语义、并发、退出、表单/菜单引用和待批准边界保持不变。

Requirement Source 仍为 `docs/veyra.md`
（`sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4`），
U2 决策仍为 `docs/decisions/DCR-018-subscription-use.md`
（`sha256:f4950c946aeeeb4669942980a849d689c1e4c1db47a0985cab8e6df550e4ce7c`）。
表单、菜单、请求策略、代理、调度与二维码继续引用
`.sdlc/evidence/TASK-011/subscription-expansion-proposal-005.md`
（`sha256:15f7676dc4ab386ed232ae30ac42faa170d854af7770a135512363d8e97e2071`），
仅由第 3–4 节覆盖其通用双分支响应中 Activate 的特例。

## 2. 强类型选择投影

真实 `RuntimeIntent` 只携带 nodes、pools、routes；default target 属于编译输入的独立字段。
选择投影因此必须返回不可拆开的强类型结果：

```rust
struct SelectedRuntimeProjection {
    runtime_intent: RuntimeIntent,
    projected_default_target: RouteTarget,
    selected_subscription_id: SubscriptionId,
    selected_generation: u64,
}
```

唯一构造入口 `project_selected_runtime(&AppState) -> Result<SelectedRuntimeProjection,
SelectionProjectionError>` 同时执行 006 第 3 节的订阅过滤、隐式 Pool 生成和硬引用检查：

1. 持久 `default_target == Unconfigured` 时，`projected_default_target` 必须指向本次
   `runtime_intent` 中运行时专用的当前订阅隐式 Pool。
2. 持久 default target 为 Direct/Block 或过滤后仍存在的用户 Pool 时，投影保持该目标。
3. 显式 default target、Route 或 Pool 手动节点在过滤后失效时，整体返回
   `selectionConflict`；不得回退隐式 Pool，也不得返回半成品。
4. 构造完成前断言：Pool 型 projected default 必须存在于同一 `runtime_intent` 且非空；
   selected ID/generation 必须等于本次输入状态的持久元组；generation 不超过
   JavaScript safe integer 上限。断言失败转换为封闭 `configurationFailed`，不 panic、不保存。

`ObservationCompilationInput` 只接受 `SelectedRuntimeProjection` 的整体转换，字段保持私有；
不得再以 `RuntimeIntent::from_state(state)` 加持久 `state.default_target` 分别组装选择配置。

`activate_subscription` 的 preflight 与零参数 `start_managed_observation_runtime` 的 load 路径
必须调用同一 `project_selected_runtime`。Use 使用候选 active ID/generation；Stopped 后 Start
使用已持久 active ID/generation。两条路径把同一对象中的 runtime/default 成对交给 compiler，
Ready 后发布该对象携带的 selected 元组。active 为空的 Start 返回下节固定结果，绝不恢复全量投影。

## 3. 封闭命令响应

### 3.1 Start

零参数 Start 延续当前 camelCase 字符串枚举，只新增一个结果：

```text
"started"
| "alreadyRunning"
| "subscriptionSelectionRequired"
| "stateUnavailable"
| "configurationFailed"
| "startFailed"
| "busy"
```

`subscriptionSelectionRequired` 只表示 V5 `active_subscription_id == null`。投影冲突、隐式 Pool
一致性失败和 compiler/check 失败均为 `configurationFailed`；响应不附底层正文。

### 3.2 Activate

`activate_subscription` 请求固定为 `{id: string}`，Rust 请求 DTO 使用
`deny_unknown_fields`。Tauri 在进入 command 前拒绝的反序列化错误不构造业务响应，前端将该 invoke
拒绝固定映射为 `invalidInput`，不得展示框架原文。成功进入 command 后，响应是唯一三分支 tagged union：

```ts
type ActivateSubscriptionResponse =
  | {
      status: "ok";
      outcome: "activated" | "alreadyCurrent";
      operationId: string;
      subscription: SubscriptionSummary;
      activeConfigurationGeneration: number;
    }
  | { status: "pending"; operationId: string }
  | {
      status: "error";
      operationId: string | null;
      error: ActivateSubscriptionErrorCode;
    };

type ActivateSubscriptionErrorCode =
  | "invalidInput"
  | "identityFailed"
  | "busy"
  | "notFound"
  | "stateUnavailable"
  | "selectionConflict"
  | "configurationFailed"
  | "saveFailed"
  | "stopFailed"
  | "startFailed"
  | "recoveryRequired";
```

精确语义：

- `ok/activated` 只在目标 generation 已 Ready 后返回；`ok/alreadyCurrent` 只在当前 Ready 的
  applied `(id,generation)` 与持久元组完全相同后返回。
- 同步等待超时只返回 `pending`。它不是错误，也不说明操作最终成功；UI 以 operation ID 等待观测终态。
- operation ID 成功生成后，所有 error 都回传该 ID；只有 ID 熵源失败使用
  `{status:"error", operationId:null, error:"identityFailed"}`。
- `selectionConflict` 专用于第 2 节引用冲突；prepare/check 映射 `configurationFailed`；
  持久提交失败映射 `saveFailed`；旧实例停止无法确认映射 `stopFailed`；候选 spawn/Ready
  失败映射 `startFailed`，清理或所有权不确定映射 `recoveryRequired`。
- preflight 的输入、busy、not-found 或 state-load 错误分别使用表中对应固定码。

proposal005 的 `SubscriptionSummary.active` 明确定义为“持久选择”，不表示 Ready；增加
`activeConfigurationGeneration: number | null`，仅 active 项非 null。其它四个新增 command 及
既有 list/import/update 继续使用 proposal005 的 `{status:"ok"}|{status:"error"}`，不增加
`pending`。proposal005 的固定错误码继续适用于它们；Activate 只使用上面的专用闭集。

`subscription-state-changed` 保持 proposal005 的安全事件形状。选择持久提交后发送
`{id, change:"activated"}`；alreadyCurrent、提交前失败和取消不发送。运行切换进度只走下节观测，
事件不携带 operation ID、URL 或错误正文。

## 4. 封闭观测状态与错误映射

`RuntimeObservationResponse.subscriptionSwitch` 固定为：

```ts
type RuntimeSubscriptionObservation = {
  appliedSubscriptionId: string | null;
  appliedConfigurationGeneration: number | null;
  subscriptionSwitch: SubscriptionSwitch | null;
};

type SubscriptionSwitchStatus =
  | "queued"
  | "checking"
  | "prepared"
  | "persisted"
  | "applying"
  | "ready"
  | "cancelled"
  | "failed";

type SubscriptionSwitchErrorCode =
  | "cancelled"
  | "stateUnavailable"
  | "configurationFailed"
  | "saveFailed"
  | "stopFailed"
  | "startFailed"
  | "recoveryRequired";

type SubscriptionSwitch = {
  operationId: string;
  status: SubscriptionSwitchStatus;
  errorCode: SubscriptionSwitchErrorCode | null;
};
```

状态约束：

- 只有 projection/compiler preflight 成功、guard 已随请求交给 worker 的 operation 才发布
  `queued`；`invalidInput/identityFailed/busy/notFound/selectionConflict` 以及 preflight 的
  `stateUnavailable/configurationFailed` 只走同步 error，不创建 switch 状态。
- `queued/checking/prepared/persisted/applying/ready` 的 `errorCode` 必须为 null；
- `cancelled` 的 `errorCode` 必须为 `cancelled`；
- `failed` 必须携带除 `cancelled` 外的一个固定错误码；
- worker check/prepare 失败为 `configurationFailed`；提交 reload/save、旧 stop、候选
  spawn/Ready、清理/所有权不确定分别映射 `stateUnavailable/saveFailed/stopFailed/startFailed/
  recoveryRequired`；
- `ready` 必须与同一快照中的 Ready lifecycle、`appliedSubscriptionId` 和
  `appliedConfigurationGeneration` 一致；其它终态不得伪造 Ready；
- 新 operation 发布后，旧 operation 的迟到终态可以保留在有界后端诊断记录，但不得覆盖当前
  `subscriptionSwitch`。前端只接收并处理它正在跟踪或更新 revision 中权威的 operation ID。

Rust 只序列化上述字段并对 exact-key-set 做测试。所有请求 DTO 拒绝未知字段；前端 IPC decoder 对
额外字段、未知 `status`、`outcome`、switch status/errorCode 或不合法字段组合 fail closed，并显示
固定通用错误，不回显原始 payload。
响应、事件和日志不得包含 URL/query/fragment、User-Agent、HTTP header、节点凭据、生成配置、
PID、文件路径或底层错误正文。

## 5. 对应最小验收

除 006 第 12 节既有验收外，只增加以下定向断言：

1. `Unconfigured` 的 Use 与 Stop→Start 都从同一投影得到“隐式 Pool + 指向该 Pool 的
   projected default”，compiler 从未收到持久 `Unconfigured`。
2. 显式 Direct/Block、有效用户 Pool 和失效跨订阅引用分别验证 runtime/default 成对结果；
   任一不一致不保存、不停止旧实例。
3. Start 七个字符串结果做精确序列化测试；迁移 active 为空命中
   `subscriptionSelectionRequired`，选择 B 后 Stop→Start 使用 B 最新 generation。
4. Activate 的 ok/pending/error、outcome 和全部错误码做穷举序列化/decoder 测试；未知请求字段、
   未知 discriminant、未知错误码及非法 status/errorCode 组合被拒绝。
5. 同步超时后分别覆盖提交前 `cancelled` 和提交后迟到 `ready/failed`；旧 operation 终态不得覆盖
   新 operation，UI 不从 pending 猜测成功。
6. 对所有新响应、观测、事件和错误路径做敏感值反向断言；不以构建通过替代真实 WebView 的
   pending、迟到终态和 selected/applied 不一致展示。

## 6. 待批准边界

用户已批准 U2 产品语义。本修订不改变 006 第 13 节的最小批准清单：V5 字段与迁移、DTO 与五个
IPC/main capability、RuntimeObservation/worker/guard/应用内调度，以及 `qrcode.react ^4.2.0`
和工具生成锁文件，仍须独立复审后一次明确批准。本候选没有修改权限、依赖、生产代码或 Gate。
