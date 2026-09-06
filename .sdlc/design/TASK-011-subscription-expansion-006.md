# TASK-011 订阅扩展与“使用”即时切换增量设计

> 状态：`CANDIDATE`
>
> 模式：`technical-design / task-boundary`
>
> 目标身份：`TASK-011-subscription-expansion-006`
>
> 本文件不修改既有冻结设计，也不批准 Technical Design Gate。

## 1. 权威与边界

本候选落实用户已确认的 U2 语义：点击订阅“使用”后，立即以该订阅的最新完整快照生成、检查并切换运行配置。产品决定由以下权威约束：

- Requirement Source：`docs/veyra.md`，`sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4`；
- 决策：`docs/decisions/DCR-018-subscription-use.md`，`sha256:f4950c946aeeeb4669942980a849d689c1e4c1db47a0985cab8e6df550e4ce7c`；
- 用户决定证据：`.sdlc/evidence/TASK-011/use-decision-006.json`，`sha256:99719503ab93a2c6fc22f52ff3cb8aaca9662a99b438d71821e1087640ec59c7`；
- 验收映射：`.sdlc/evidence/TASK-011/use-acceptance-006.md`，`sha256:da7524ed0cadbca4ea1cac93d1fe8cf2fbe9381857a8209d8d8250a3cedf2c60`；
- 运行失败权威：`.sdlc/design/DCR-004-runtime-update-failures.md`，`sha256:8f06957ff73919f9529dd1740b72f4dd4ccf73745d6481b124133f3ca8d8d944`。

表单、右键菜单、请求选项、应用内调度、代理路径和二维码的完整候选沿用 `.sdlc/evidence/TASK-011/subscription-expansion-proposal-005.md`（`sha256:15f7676dc4ab386ed232ae30ac42faa170d854af7770a135512363d8e97e2071`）第 2A、2B、2C、3–11 节。本文件只以 U2 替换该提案中 U1/U2 待决部分，并补足切换事务、并发、生命周期和观测设计，不重述其余方案。

兼容性修复仍是独立候选 `.sdlc/evidence/TASK-011/compatibility-scope-005.md`（`sha256:c6d9474791357da3525302d5d5d7202394bf25d1b178dab6c1f4f27666b479ef`）。默认 User-Agent、BOM、YAML 写法及 VMess 字段在其独立 Review 通过前不得被本设计声称为已交付。

实现锚点：现有 worker 请求与 Start 分支在 `src-tauri/src/application/managed_observation_runtime.rs:73,279,318`；last-known-good 替换语义在 `src-tauri/src/singbox/runtime.rs:162`；状态与全量投影入口在 `src-tauri/src/domain/state.rs:37,168-175`；共享 state gate 和命令注册在 `src-tauri/src/lib.rs:65-88`。Clash Verge 的使用队列与 current 切换仅作为交互参照，位于 `E:/wx_lifeilin/github.com/clash-verge-rev/src/pages/profiles.tsx:341-462`；Veyra 不复制其前端排队或 Mihomo 调用链。

## 2. 持久模型与迁移

在现有 V4 状态上形成 V5：

```rust
pub struct AppStateV5 {
    // 既有字段省略
    pub active_subscription_id: Option<SubscriptionId>,
    pub active_configuration_generation: u64,
}
```

`active_subscription_id` 表示用户已持久选择的订阅，不表示某个 child 当前确实 Ready。`active_configuration_generation` 是所选订阅运行投影的单调版本，只用于比较“持久选择快照”和“实际 Ready 快照”，不是数据库修订号或进程身份。

V4→V5 迁移规则：

1. `active_subscription_id = None`，`active_configuration_generation = 0`。V4 曾把全部节点投影到运行配置，无法可靠推断唯一订阅，迁移不得擅自选中第一项。
2. 保留全部既有订阅、Provider、Node、Pool、Route 的 ID、名称和引用；不在迁移时联网、构建或启动 core。
3. 同一步迁移纳入本设计的两个 U2 选择字段，以及 proposal005 已列出的订阅描述、请求选项和更新策略字段及其默认值；除此之外不增加持久字段。
4. `AppState::validate` 在 `Some(id)` 时要求订阅存在；`None` 合法。生成计数不得回绕，溢出时当前写操作失败且旧状态不变。

生成变化规则：

- 选择不同订阅：持久 ID 改为目标 ID，并将 generation 加一。
- 当前已选订阅的 Provider/Node 完整快照发生运行语义变化：更新成功时 generation 加一，但不自动 Apply。
- 当前已选订阅仅改名称、描述或不影响已缓存节点的抓取选项：generation 不变。
- 未选订阅的普通导入、更新或编辑：generation 不变。
- 后续若开放 Pool/Route 编辑，凡影响当前订阅运行投影者必须加一；本候选不扩大这些编辑入口。
- 再次“使用”同一订阅时不因 ID 相同而提前返回。仅当 worker 观测到的实际 Ready 元组 `(id, generation)` 与持久元组完全一致，才返回 `alreadyCurrent`。

普通导入、编辑、手动更新和应用内自动更新只保存管理数据。即使它们使当前订阅 generation 变化，也不得隐式调用 Apply。

## 3. 当前订阅到 RuntimeIntent 的投影

`RuntimeIntent::from_state` 增加面向选择的投影入口；完整 `AppState` 仍先按现有规则验证，随后按目标订阅构建独立快照：

1. 只纳入 `Provider.subscription_id == active_subscription_id` 的 Provider 与其 Node。
2. 保留其它订阅及其全部数据在持久状态中，但它们的 Node 不进入此次 `RuntimeIntent`。
3. 为目标订阅生成一个仅存在于 `RuntimeIntent` 的隐式 Pool，成员是该订阅完整快照中的全部可用 Node，ID 使用持久层不会生成的编译器保留命名空间加订阅 ID 确定性生成。它不写入 `AppState`，也不与用户 Pool 混用身份；若旧数据异常占用该保留 ID，投影返回 `selectionConflict`，不覆盖该 Pool。
4. 当持久 `default_target == Unconfigured` 时，以该隐式 Pool 作为本次投影的默认出口。这使新导入订阅可以直接“使用”，同时不静默创建或改写持久 Pool。
5. 对用户启用的 Pool，仅保留属于目标订阅的 Provider source。过滤后仍有成员的 Pool 可进入投影；空 Pool 从候选投影中移除，不修改持久 Pool。用户明确配置的 default target 若指向过滤后仍有效的 Pool、Direct 或 Block，则保持该选择，不改用隐式 Pool。
6. Direct/Block 等不依赖订阅 Node 的合法目标可保留。
7. 任一用户明确配置的 Route、default target 或 Pool 手动选中节点，在过滤后指向不存在的 Pool/Node 时，返回结构化 `selectionConflict`。不得用隐式 Pool 掩盖硬引用冲突，不得静默改写持久选择，也不得保存新的 active ID。
8. 目标订阅不存在、没有 Provider、没有可用 Node 或完整快照未通过现有 normalize/validate 时，构建失败；旧状态和旧实例保持不变。

该规则允许一个 Pool 的多订阅 source 在选择时变为目标订阅子集，同时阻止跨订阅硬引用悄然指向另一份配置。

## 4. 最小运行时改造

现有 `ManagedObservationRuntimeController` worker 继续作为唯一 core 生命周期所有者；不在前端拼接 Stop→Start，也不另建通用事务框架。

现有 `SidecarRuntime::start_or_replace` 最小拆为三个内部阶段，并由原方法顺序组合以保留旧调用行为：

- `prepare_replacement(candidate)`：执行 `sing-box check` 与候选准备，旧 child 全程继续运行；
- `commit_prepared()`：停止旧 child，启动候选并等待 Ready；
- `cancel_prepared()`：在破坏旧实例前清理候选临时资源。

新 worker 请求：

```rust
WorkerRequest::ActivateSubscription {
    operation_id,
    expected_state,
    candidate_state,
    candidate_config,
    selected_subscription_id,
    selected_generation,
    cancel_state,
    owned_guards,
    response,
}
```

`activate_subscription` 命令只负责边界校验、生成后端 operation ID、取得并移交 guard、准备请求并等待有界响应。worker 负责所有会改变 child 所有权的阶段。

零参数 `start_managed_runtime` 仍保留原 IPC，但其 load 路径改为读取 V5 选择投影：`active_subscription_id = Some(B)` 时始终按 B 的最新 generation 构建并启动，Ready 后发布同一 applied 元组；因此 Stop→Start 会继续使用 B，而不会重新调用全量 `RuntimeIntent::from_state`。`active_subscription_id = None` 时返回 `subscriptionSelectionRequired`，要求先在订阅页执行“使用”；迁移后不得用全量订阅隐式恢复旧行为。Ready 状态下原有 `AlreadyRunning` 保持不变，Start 不承担热切换。

## 5. “使用”提交序列

统一流程同时覆盖“已运行 A→使用 B”和“已停止→使用 B”：

1. **占有操作**：先取得 runtime operation guard，再取得 subscription mutation guard；任一已占用即返回 `busy`，不排队覆盖旧请求。
2. **预取快照**：在短 state gate 内 load 最新状态，计算候选 active ID/generation，并对目标订阅完整快照生成 `RuntimeIntent`、编译和 finalize；释放 state gate。网络获取不属于“使用”，此流程不联网。
3. **进入 worker**：携带 expected/candidate state、完整生成配置、元组和 owned guards 发送 `ActivateSubscription`。guard 随请求存活，调用方超时或断开不会释放并发保护。
4. **无破坏检查**：worker 调用 `prepare_replacement`。check/prepare 失败时清理候选，旧 A 的 child、观测和持久选择均不变。
5. **提交前取消点**：若收到取消或退出请求，调用 `cancel_prepared` 并结束；旧实例和持久状态不变。
6. **持久提交**：重新取得短 state gate，reload 并与 expected state 做完整一致性比较。失配返回 `busy` 并取消候选。匹配时：
   - active ID/generation 变化则原子保存 candidate state；保存失败即取消候选；
   - 同一已选 B 因 generation 已在先前 update 中持久增加时，不重写相同状态；成功的精确 reload 即确认该持久快照仍是提交目标。
7. **破坏性应用**：持久提交点之后不再接受取消。worker 调用 `commit_prepared`，并遵循 DCR-004。
8. **发布 Ready**：只有候选 Ready 后，才把实际 applied 元组 `(subscription_id, generation)` 与 Ready 生命周期在同一 worker 更新中发布。此时 UI 才能显示“正在使用”。

同一已选 B 的节点更新成功会提高持久 generation。再次点击“使用”时，applied generation 仍旧，因此必须用最新完整 B 快照重新走上述流程，不能返回 ID 相同或 `AlreadyRunning`。

## 6. 准确失败状态

- **投影、编译、finalize、check、prepare 失败**：持久选择不变；旧 A 继续 Ready；B 不显示正在使用。
- **提交前 state 失配**：候选清理，返回 `busy`；旧状态与旧实例不变。
- **新选择保存失败**：不得停止 A，不得宣称 B 已选或已应用。若候选清理本身失败，进入现有 `RecoveryRequired` 并保留所有权，仍不得宣称 B Ready。
- **旧 child 停止无法确认**：持久选择已经是 B，但不得启动 B；保留旧 child 所有权并显示 `RecoveryRequired`。不伪造对持久选择的回滚。
- **候选 spawn/Ready 失败**：清理候选；实际为 `Stopped`，清理失败则 `RecoveryRequired`。持久选择保持 B，不自动重启 A，符合 DCR-004。
- **Ready 后响应通道丢失**：B 仍是已提交并实际 Ready；worker 从观测发布最终结果，不因前端未收到同步响应而回滚或标记失败。

停止失败或 spawn/Ready 失败发生在持久提交之后，因此允许“已选择 B、实际仍是 A/已停止/需恢复”的短期或故障状态。UI 必须如实展示两种身份，不能用 selected 字段冒充实际运行。

## 7. 超时、取消与迟到完成

每次 Use 由后端生成不含凭据的 `operation_id`，worker 维护：

```text
Queued → Checking → Prepared → Persisted → Applying → Ready
                  ↘ Cancelled / Failed
```

- Controller 等待超时后设置 `cancel_requested`，并返回 `pending { operationId }`，而不是返回一个可让 UI推断为失败的普通错误。
- worker 只在持久提交点前响应取消并清理候选；进入 `Persisted` 后必须继续完成 Apply。
- 同步接收端被丢弃不改变 worker 结果。观测快照/增量携带 operation ID、阶段和最终结果，UI 通过现有观测链或显式状态读取收敛，不自动重试、不猜测成功。
- UI 在 pending 后保持该 operation 的忙碌提示；只根据匹配 operation ID 的权威终态解除。迟到的旧 operation 不得覆盖新的页面状态。

## 8. 并发、Start/Stop、更新和退出

新增一个进程内、fail-fast 的 runtime operation guard，覆盖 Start、Stop、Use 与 Shutdown。它与现有 worker 串行队列配合，防止调用方超时后产生第二个生命周期操作。订阅写 guard 改为可随 worker 请求持有的 owned guard，覆盖 Use 全流程；导入、更新、编辑、删除和调度更新在 Use 期间返回 `busy`。

固定取得顺序为：

```text
runtime operation guard → subscription mutation guard → short state gate
```

普通订阅写不取得 runtime operation guard，且任何路径都不得在持有 state gate 时等待 worker、HTTP 或 core，避免锁环。

- **Start/Stop/Use 并发**：最先持有 runtime guard 的操作执行，其余 fail-fast `busy`；零参数 Start 在 Ready 时仍保持 `AlreadyRunning`，Stopped 时只启动持久所选订阅，未选择时返回 `subscriptionSelectionRequired`，不承担热切换。
- **Update/Use 并发**：先取得 subscription guard 者完成；Use 的提交前 reload 还会拒绝任何未受 guard 约束的状态漂移。
- **退出请求**：先原子设置 `shutdown_requested`，再尝试取得 runtime guard。若 Use 正持有 guard，不创建第二个生命周期操作，也不返回可退出许可；而是登记一个由同一 worker 在 Use 终态后消费的唯一 Shutdown intent。
- **退出发生在提交前**：共享 `shutdown_requested` 使 worker 取消候选，再处理已登记的 Shutdown。
- **退出发生在持久提交后**：worker 先完成 Apply，再处理已登记的 Shutdown 并停止新 B；不得在已提交后半途遗留无人拥有的 child。
- **Shutdown 等待超时**：托盘不得立即退出进程。worker 继续清理并设置 `shutdown_complete`；后续退出请求读取该状态。只有该状态为真才允许进程退出，响应 receiver 丢失不等于清理失败。

运行采样仍由同一 worker 串行执行。旧实例停止确认前不发布 B 的流量或连接身份。

## 9. 选择与实际运行观测

持久选择来自 StateStore；实际 applied 身份只来自持有 child 的 worker。安全 RuntimeObservation DTO 增加：

```text
appliedSubscriptionId: string | null
appliedConfigurationGeneration: number | null
subscriptionSwitch: { operationId, status, errorCode? } | null
```

不得返回 PID、临时配置路径、订阅 URL、请求头或生成配置。前端列表的 `selected` 只表达持久选择；展示文案按二者组合：

- 生命周期 Ready 且 selected/applied 的 ID 与 generation 全部相同：`正在使用`；
- operation 正在运行：目标卡片 `正在切换`；
- 持久已选 B，但 Ready 的 applied 仍是 A：A `仍在运行（旧选择）`，B `已选择，未应用`；
- ID 相同、generation 不同且旧版本仍 Ready：`正在运行旧版本`；
- 持久已选 B、生命周期 Stopped：`已选择，服务已停止`；
- 生命周期 RecoveryRequired：目标卡片 `切换未完成`；若仍持有旧 child 的 applied 元组，旧卡片显示 `旧实例状态待恢复`，不得显示正在使用。

应用重启后可以恢复 selected ID/generation，但 applied 元组初始为空；core 未 Ready 时不得显示“正在使用”。

## 10. 既有扩展入口的约束

proposal005 的表单、菜单、UA、代理、调度和二维码仍按其路径/哈希进入同一后续实现批次，新增 U2 规则如下：

- 卡片点击、右键“使用”和键盘入口调用同一个 `activate_subscription`；不可生成 Stop→Start 两请求。
- 远程更新、代理 Provider 更新、自动更新和编辑只更新管理数据；成功后若目标是当前已选订阅，UI 显示旧版本状态，等待显式“使用”。
- “删除”已选订阅仅在 core 已确认 Stopped、无切换且非 RecoveryRequired 时允许；删除时清空 active ID 并提高 generation。运行或恢复状态返回冲突，不自动 Stop。
- proposal005 标为与 sing-box 管理模式冲突或缺少真实语义的菜单项继续不生成按钮；本设计不把原文编辑、merge/script、规则/代理组编辑伪装为已支持。

## 11. IPC、权限和依赖候选

在已有 list/import/update 三命令外，proposal005 的五个新命令保持：

```text
get_subscription_settings
edit_subscription
activate_subscription
get_subscription_share_url
delete_subscription
```

对应 main capability 精确候选已物化于 `.sdlc/evidence/TASK-011/proposed-main-expansion-capability-006.json`，`sha256:1df7f987ba83ac1bd6e9cc70db95257cf67cb178b34c879507c4949abfd95891`：

```text
allow-get-subscription-settings
allow-edit-subscription
allow-activate-subscription
allow-get-subscription-share-url
allow-delete-subscription
```

本设计不要求改该清单。二维码依赖候选是 `qrcode.react ^4.2.0`，记录于 `.sdlc/evidence/TASK-011/proposed-qr-dependency-006.json`，`sha256:254b2b1a7dacaf2dc6441b54125ed2a64bbd2992cf0dbe7038f582f2d9a20dad`；尚未安装，也未修改锁文件。

## 12. 验证责任

实现后必须用隔离 StateStore、受控 sidecar port 与 UI 验证以下自有契约：

1. V4→V5 保留全部历史身份/引用，active 为空；迁移后的 Start 返回 `subscriptionSelectionRequired`；V5 重启恢复 selected，但未启动不显示正在使用。
2. 运行 A 时 Use B：check 前后 A 保持；B Ready 后才更新 applied；A 数据仍存在。
3. Stopped 时 Use B 会实际 build/check/start B。
4. B 已选且更新节点后再 Use B，应用最新完整快照；只有 ID 相同不得跳过。
5. `Unconfigured` 默认目标使用运行时隐式当前订阅 Pool；显式跨订阅 Pool/Route 引用返回 `selectionConflict` 并保护旧状态/旧 child。
6. compile/check、state 失配、save、旧 stop、spawn/Ready 各故障点符合第 6 节，不伪回滚；持久提交后进程中断并重启时恢复 B 为“已选择，服务已停止”。
7. Start/Stop/Use/Update/Shutdown 竞争只有一个生命周期操作；调用方超时后的迟到 commit 由 operation 状态收敛。
8. 原生 WebView 验证卡片、右键和键盘“使用”同一动作，并覆盖正在切换、旧版本、已选择但停止、恢复失败文案和焦点/忙碌状态。
9. 普通导入、编辑、更新、自动更新不触发 Apply。

不得用 parser 单测、Rust 构建或 core 启动代替真实 UI 状态验收；也不扩展为 sing-box 协议矩阵。

## 13. 实现前仍需一次确认

用户已经确认 U2 产品语义。以下 Material 变更仍需在独立 Design Review 后一次性明确批准：

1. V5 字段与迁移：`active_subscription_id`、`active_configuration_generation`，以及 proposal005 的订阅请求/更新设置字段；
2. list/update DTO 扩展、上述五个 IPC 和五项 main capability；
3. RuntimeObservation DTO/事件扩展、worker 的 prepare/commit/cancel 最小拆分、runtime/subscription guard 与应用内调度；
4. `qrcode.react ^4.2.0` 生产依赖及工具生成的 `pnpm-lock.yaml` 变化。

本候选不批准这些变更，不修改权限、依赖、持久数据或生产实现。
