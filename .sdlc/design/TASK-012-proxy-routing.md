---
id: DESIGN-TASK-012-PROXY-ROUTING
status: CANDIDATE
task_ref: .sdlc/tasks.yaml#TASK-012
requirement_ref: docs/veyra.md
requirement_identity: sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4
design_refs:
  - .sdlc/design/foundation.md
  - docs/decisions/DCR-017-ui-integration-boundary.md
  - docs/decisions/DCR-018-subscription-use.md
  - docs/decisions/DCR-019-task012-custom-pool-projection-amendment.md
  - docs/ui/veyra-ui-spec.md
---

# TASK-012 出口组选择与分流编辑技术设计

本候选只解决 TASK-012 首次实现前的 Material Engineering Decision：状态事务、封闭 IPC、
当前受管实例的节点切换，以及自定义 Pool 在“激活订阅投影”中的运行语义。它不冻结 Task Scope，
不批准实现，也不修改既有领域或 `SingBoxCompiler` 契约。正式 Task 仍须在本设计经独立 Review 和
人工确认后由 `task-breakdown` 物化。

## 1. 现状与约束

### 1.1 可直接复用

- `AppState` 已持久化 `default_target`、`providers`、`nodes`、`pools` 和 `routes`，并在
  `AppState::validate` 中整体校验稳定 ID、Pool source、筛选、Manual/UrlTest 选择与 Route target。
  Source：`src-tauri/src/domain/state.rs:47-70 AppState`、`:74-162 AppState::validate`、
  `:164-180 AppState::resolve_pool_members`。
- `NodePool`、`PoolSource`、`NodeFilter`、`SelectionPolicy`、`RoutePolicy`、`TrafficMatcher` 与
  `RouteTarget` 已是 sing-box 中立领域；`RuntimeIntent::from_state` 只编译启用 Pool/Route，并按稳定
  ID/priority 确定性排序。Source：`src-tauri/src/domain/state.rs:196-236 RuntimeIntent::from_state`、
  `:747-906` 对应领域 symbols。
- `JsonStateStore::save` 在完整校验后备份并原子替换；`StateAccessGate::try_lock` 是当前进程内
  `state.json`、迁移和备份的共享短临界区。Source：`src-tauri/src/storage/store.rs:75-127
  JsonStateStore::{load,save}`；`src-tauri/src/application/state_access.rs:3-16 StateAccessGate`。
- `SubscriptionManager`、运行时与调度器已经共享同一个 `StateAccessGate`；网络和 sidecar 生命周期
  不得持锁。Source：`src-tauri/src/lib.rs:61-94 run`；
  `src-tauri/src/application/subscription_management.rs:352-383 SubscriptionManager::new`。
- 当前手动 Pool 编译为 selector、UrlTest Pool 编译为 urltest；Route 只映射为 Pool、Direct、Block。
  Source：`src-tauri/src/singbox/compiler.rs:289 compile`、`:904 route_rule`。

### 1.2 当前缺口

- `Proxies` 与 `Routing` 仍是 placeholder，现有 Tauri command 与 `src/lib/subscriptions.ts` 没有
  Pool/Route 查询或写入入口。Source：`src/App.tsx:213-224`；`src-tauri/build.rs:1-23 main`；
  `src-tauri/capabilities/default.json`；`src/lib/subscriptions.ts:1-15 commands`。
- `ClashApiClient` 目前只读 ready、连接、流量和日志，没有 selector 读取/切换能力。Source：
  `src-tauri/src/singbox/clash_api.rs:187-238 ClashApiClient`。
- `project_selected_runtime` 当前过滤到激活订阅。它会把多 Provider Pool 缩为激活订阅内子集，
  并可能因 Manual 选择落在其它 Provider 而返回 `SelectionConflict`。Source：
  `src-tauri/src/application/selected_subscription.rs:26-106 project_selected_runtime` 及
  `:252-323` 投影测试。

## 2. 产品与 UI 边界

### 2.1 页面 Composition

```text
ProxiesPage (Page full)
├── PageHeader
│   ├── 出口组 / 全部节点 segmented control
│   └── 新建出口组 Button
├── toolbar / Notice
└── loading | empty | error | render
    ├── PoolGroupList
    │   └── PoolGroup
    │       ├── GroupHeader + expand/collapse + actions
    │       └── ProxyNodeRow × N
    └── AllNodeList
        └── ProxyNodeRow × N

RoutingPage (Page full)
├── PageHeader
│   └── 新建规则 Button
├── DefaultTargetRow
├── toolbar / Notice
└── loading | empty | error | render
    └── RoutePolicyRow × N
```

- 视觉以 `docs/ui/veyra-ui-spec.md:180-205,233-243,291-311` 为唯一 Veyra 规格；参考
  `docs/ui/reference/clash-verge/windows-light/proxies.png` 和 Clash 的
  `ProxyPage → ProxyGroups → group header/tools/rows` composition。不得复制 Clash mode、Mihomo
  group/provider/rule schema、API、品牌或文案。
- `Proxies` 使用现有领域字段：Provider/Node 名称、协议、Pool source/filter、Manual/UrlTest、enabled。
  `Routing` 使用 `default_target` 和当前已有 matcher：Domain、DomainSuffix、Application、IpCidr、
  Port、Protocol；不伪造 Service preset、RuleSet、Inbound 或延迟数据。
- ImplicitProvider Pool 的身份、名称、kind 和 source 由订阅生命周期拥有，页面不可删除或改写；
  允许在有效成员内选择 Manual 节点。Custom Pool 才允许创建、编辑、启停和删除。
- “全部节点”是只读清单，不成为节点凭据编辑器。UI 不展示 server、密码、UUID、private key、
  原始 options、sing-box tag 或生成 JSON。
- 列表第一版不引入虚拟化或新 UI library。以 100 subscriptions / 5000 nodes / 100 pools /
  500 routes fixture 记录测量；只有实测证明完整 DOM 不满足交互验收时，才在同一 Task 内提出有界
  windowing 修订，不预先新增框架。Source：`docs/veyra.md:4371-4393`；
  `docs/ui/veyra-ui-spec.md:233-243`。

### 2.2 编辑行为

- 新建/编辑 Custom Pool 使用 Dialog：名称、enabled、Provider sources、每来源 NodeFilter、
  Manual/UrlTest 策略和成员预览。`regions` 必须说明为当前实现的“节点名称包含词”，不得声称来自
  独立地理元数据。Source：`NodeFilter::matches` at `src-tauri/src/domain/state.rs:808-829`。
- Manual 只允许选择 `resolve_pool_members` 返回的稳定 NodeId；UrlTest 只接受非空 probe URL、
  正 interval 和 tolerance。前端约束用于即时反馈，后端领域校验仍是权威。
- Default target 在 UI 映射为“跟随当前订阅”（`Unconfigured`）、Pool、Direct、Block。
  RoutePolicy target 只允许 Pool、Direct、Block，禁止 `Unconfigured` 和直接 NodeId。
- Route dialog 编辑 name、enabled、matcher kind/values 与 target；列表顺序是唯一用户优先级输入，
  后端提交时按顺序归一化为连续 `priority`。删除被 default/route 引用的 Pool 返回
  `referenceConflict`，不得级联删除或自动改写目标。
- 所有 Dialog 失败保留用户输入；成功才关闭。加载、空、错误、保存中、冲突、disabled、selected、
  Notice 与 Dialog 都使用现有 primitive/token，不重构 AppShell、Sidebar、Router 或
  `activePage + hidden`。

## 3. 状态事务、generation 与进程内 CAS

新增内部 `ProxyRoutingManager`，但不建立第二份持久或 AppState 内存 Source of Truth：

```text
query / mutation
  → shared StateAccessGate.try_lock (fail-fast busy)
  → JsonStateStore.load latest complete AppState
  → RoutingRevision.observe(CasMaterial)
  → compare expectedRevision
  → apply one closed typed mutation
  → advance desired generation only for structural mutation
  → AppState::validate
  → JsonStateStore.save (backup + atomic replace)
  → RoutingRevision.commit(candidate material)
  → return authoritative snapshot
```

### 3.1 desired / applied identity

- 复用持久 `active_configuration_generation` 作为 **desired configuration generation**。所有实际改变
  Pool topology、Pool source/filter/strategy/enabled、Route 或 default 的成功结构保存都检查安全整数上限
  后加一；保存失败不推进。没有 active subscription 时仍推进全局 desired generation，后续首次
  Use/Start 使用当前值。
- Manual Pool 的 `selected_node_id` 是可持久化 hot-swappable selection，不属于需要完整 compile/apply 的
  结构版本。单纯 `setManualSelection` 保存不得推进 `active_configuration_generation`；但它属于持久
  mutation，成功保存仍推进 `routingRevision`。
- 复用 worker 已拥有的 `applied_subscription_id / applied_configuration_generation` 作为当前 child 的
  **applied structural runtime identity**。结构保存和 selector PUT/GET 都不改 applied generation；只有
  完整 configuration Apply 达到 Ready，才把 applied generation 对齐该次 desired generation。
- `desired != applied`（或 subscription ID 不同）且存在 Ready/Recovery runtime 时表示“有未应用更改”。
  Stopped 时显示“已保存，将在下次启动时生效”，不伪造当前 applied identity。App/WebView 重载后仍由
  persisted desired 与 worker observation applied 重新计算，不依赖一次性 Toast。
- 不新增持久字段、schema_version 或 migration。旧 Runtime 的状态、清理与失败恢复必须读取 worker
  已持有的 applied `GeneratedConfig` artifact/identity；不得从已修改 AppState 重新计算一个“旧投影”。

### 3.2 routingRevision

- `routingRevision` 是 `ProxyRoutingManager` 内 `Mutex<RoutingRevisionState>` 拥有的进程内单调安全整数，
  初次成功 query 为 1，进程重启重置；它不是 Domain、schema、运行 generation 或跨进程令牌。
- `CasMaterial` 是仅进程内可比较的 typed value，字段固定为：active subscription ID、desired generation、
  Provider `(id, subscription_id, name)`、Node `(id, provider_id, name, protocol)`、完整 Pool、default target、
  完整 Route；按 AppState 各 Vec 当前顺序保留，不含 credentials/server/options。无需 JSON canonical encoding。
- 每次 query/mutation 在 state gate 内加载最新状态后调用 `observe`：material 与上次不同时 revision 加一并
  替换 material，相同则保持。这样订阅刷新即使不直接调用 manager，也会在下一 query/CAS 时被识别。
  revision 达到 JS safe integer 上限时 fail closed 为 `stateUnavailable`，不回绕。
- mutation 在同一 state gate 内先 `observe(latest)`，再比较 `expectedRevision`；不等返回 `conflict`。
  有状态变化的成功保存必须把 revision 加一并返回新 snapshot；完全相同的 Manual selection 进入第 4.3
  节 reconcile-only 路径，不写文件、不推进 generation/revision。Manual selection 的真实保存只推进
  revision，不推进 generation。
- App 根部只挂载一个轻量 `ProxyRoutingProvider`，是同一 WebView 内唯一 query/snapshot owner；Proxies 与
  Routing 两个保持 `hidden` 的页面消费同一 snapshot。成功 mutation 原子替换 provider snapshot，两个页面
  同次 render 收敛；既有 `subscription-state-changed` 只作为 provider 重新 query 的触发器，不承载 Pool/
  Route mutation。无第二份页面 cache、轮询或新后台队列。

## 4. 运行时语义

### 4.1 DCR-019 最小跨订阅依赖闭包

`docs/decisions/DCR-019-task012-custom-pool-projection-amendment.md` 是用户已批准的正式产品 amendment，
明确 supersede TASK-011 SF-004/DCR-018 的绝对 active-only 表述。算法冻结如下：

1. 根集合 = active subscription 的 Provider/Node/runtime implicit Pool + **所有 enabled Custom Pool**；
2. Custom Pool 唯一扩展边 = `PoolSource.provider_id → 该 Provider nodes → NodeFilter::matches`；
3. 加入每个根 Custom Pool、被其 source 精确引用的 Provider，以及 filter 精确命中的 Node；不递归沿 Route、
   sibling Provider、Subscription、其它 Pool 或显示名称扩展；
4. disabled Custom/所有跨订阅 Implicit Pool、未引用 Provider/Node 和 filter 未命中 Node 必须排除；
5. Manual selected Node 必须在本 Pool 闭包；UrlTest 至少一名成员；
6. default/enabled Route 引用缺失、disabled 或空闭包 Pool，或刷新后 Manual Node 离开闭包，返回
   `selectionConflict`，不自动改写 desired state、选择替代节点或回退 implicit Pool。

只调整 `project_selected_runtime` 与测试；不改 AppState、StoredStateV6、RuntimeIntent、Compiler 或 tag
契约。无 enabled Custom Pool 时，原 active-subscription isolation 正反行为必须保持。

### 4.2 Pool / Route / Default：Save → Apply 用户两阶段

这是两个明确的用户操作，不只是实现阶段拆分：

1. **Save**：CAS、整体校验、原子保存成功后推进 desired generation，立即返回 authoritative snapshot。
   Ready runtime 的 applied identity 保持旧值，页面持续显示“有未应用更改”和“应用到当前服务”。
2. **Apply**：用户点击后复用 `activate_subscription(activeId, force=true)`；worker 从刚保存的完整 AppState
   build/check/prepare 并按 DCR-004 替换。只有 Ready 才令 applied generation 等于 desired。

| Apply 阶段/结果 | Persisted desired | 实际 runtime / applied identity | 用户状态与重试 |
| --- | --- | --- | --- |
| Save 成功、未 Apply | 新 generation | 旧 Ready identity 不变；Stopped 为 null | 持续 dirty；显式 Apply/下次 Start |
| queued/checking/prepared | 新 generation | 旧 Ready identity 不变 | `正在应用`，按 operationId 等待 |
| compile/finalize/check/prepare 失败 | 新 generation | 旧 child 与旧 applied identity 不变 | dirty + `配置未应用`；可重试 Apply |
| state 冲突/Apply 前取消 | 新 generation | 旧 child/identity 不变 | dirty + `状态已变化`；刷新后重试 |
| 旧 child stop 未确认 | 新 generation | RecoveryRequired；保留仍拥有的旧 applied identity，不称 Ready | dirty + `停止未完成`；先手动恢复/Stop |
| 旧 child 已停，spawn/Ready 失败 | 新 generation | Stopped；applied identity 清空；清理失败为 RecoveryRequired | `已保存，服务未启动`；手动 Apply |
| Ready | 新 generation | 新 child，applied = desired | 清除 dirty，显示已应用 |
| 前端超时但 worker 未终态 | 新 generation | 以 worker 实际阶段为准 | pending，不猜失败；按 operationId 收敛 |

保存后的新配置不自动回滚。DCR-004 禁止 spawn/Ready 失败后自动启动旧配置；“保留旧实际 identity”仅在
旧 child 仍被拥有时成立，已确认停止后必须清空 applied identity。RuntimeSupervisor、System Proxy、TUN
和 PlatformAdapter 不因本 Task 改写。

### 4.3 Manual Pool：persist → switch → read-back

- Manual 节点选择是即时用户操作。desired Node 不同时，先按第 3 节保存 selection，只推进
  `routingRevision`；随后由唯一 runtime worker 绑定 `(owned instance identity, applied subscription ID,
  applied structural generation, pool tag, node tag)`。Stopped 或非 Ready 时不发请求，返回
  `selectorSavedOnly`。
- 每次完整 Apply Ready 时，worker 从**当次已应用的 projection/artifact**建立最小
  `AppliedRoutingIndex { configuration_generation, pool_members }`。`pool_members` 只保存
  `PoolId → NodeId → (pool runtime tag, node runtime tag)` 的映射，不含名称、凭据或配置内容；它不进入
  AppState、state.json 或 IPC。新 child Ready 时原子替换，确认 Stopped 时清空；RecoveryRequired 且仍
  拥有旧 child 时与旧 applied artifact 一起保留。
- 只有目标 Pool/Node 都存在于当前 owned instance 的 `AppliedRoutingIndex` 时才允许 hot switch；映射
  必须来自该 index，禁止从已经修改的 AppState/projected state 重算 tag。不存在时只保留 desired
  selection，返回 `selectorSavedOnly`。已有 structural dirty 不阻止对旧 artifact 中仍存在的 Pool/Node
  hot switch，但该操作绝不清除 dirty 或改变 applied structural generation。
- 对匹配的 owned Ready instance，`ClashApiClient` 使用 backend 生成的固定 selector URL/header/tag 发 PUT，
  无论 PUT 返回成功、明确拒绝还是 transport/response-loss，都在 instance identity 未变化时对同一 selector
  执行固定 GET read-back。前端不能传 URL、secret、header、runtime tag 或 response schema。
- GET 前后都复核 owned instance identity、applied subscription、applied structural generation 与 index
  identity；发生实例漂移不向新实例发送旧操作，结果为 `selectorApplyUnknown`。read-back 只能通过
  同一 index 映射回稳定 NodeId。read-back 完成后重新取得最新 authoritative snapshot；若该 Pool 的
  persisted desired 已被后续 mutation 改写，本操作返回 `selectorApplyUnknown(reason:"superseded")`，
  不把旧操作结果冒充当前 desired 已应用。

互斥结果：

| outcome | 判据 | structural / selector 状态 | UI |
| --- | --- | --- | --- |
| `selectorApplied` | 同一 owned instance read-back == 本次 persisted desired，且未被后续 mutation supersede | structural generation 永不改变；selector runtime 与 persisted desired 一致 | `已切换`；structural dirty 原样保留 |
| `selectorNotApplied` | 同一 instance read-back 明确为 applied index 内其它 Node | selection 已保存；applied generation 不变；runtime Node 使用 read-back 值 | `选择已保存，当前节点未切换` + 重试 |
| `selectorSavedOnly` | Stopped/非 Ready、Pool/Node 不在 applied index，或保存后可证明 PUT 尚未 dispatch 而 worker dispatch 不可用 | selection 已保存；不发 API；applied generation 不变 | `已保存，将在下次完整应用时生效` |
| `selectorApplyUnknown` | read-back 失败、未知 tag、instance/index 漂移或 desired 被 supersede | selection 已保存；applied generation 不变；selector runtime 不能对当前 desired 作肯定结论 | `选择已保存，当前节点状态未知` + Apply/重试 |

transport error 绝不直接等同 apply failed。持久 desired 不做补偿回滚：Core 写是否发生可能未知，而再次
写旧值也可能失败并制造第二个不确定结果。read-back 为旧 Node 是 authoritative 的“未应用”，必须返回
`selectorNotApplied`，不把它误报为 unknown 或 transport failure。用户重试同一 desired selection 时走 reconcile-only：不再次保存、不推进
generation/revision，只在当前 owned instance/index 上重复 PUT + GET；完整 Apply 仍可作为确定性恢复路径。
Selector GET/PUT 的 sing-box 1.14.0 实际 shape 必须先以固定 core fixture 验证，未匹配即 fail closed，
不把任意 JSON 透传 UI。

强制状态序列：

| 场景 | API 行为 | 结果 |
| --- | --- | --- |
| CAS/校验/`JsonStateStore::save` 失败 | 不发送 PUT/GET | error + 当前 revision；desired selection、generation、revision、runtime 均不变 |
| Runtime stopped/非 Ready | 不发送 PUT/GET | `selectorSavedOnly(runtimeStopped/runtimeNotReady)` |
| Pool 或 Node 不在 AppliedRoutingIndex | 不发送 PUT/GET | `selectorSavedOnly(notInAppliedArtifact)` |
| selection 已保存，但 worker dispatch/queue 在 PUT 前失败且可证明 PUT 尚未 dispatch | 不发送 PUT/GET | `selectorSavedOnly(dispatchUnavailable)`；最新 snapshot 反映 persisted desired，applied structural generation 不变 |
| PUT 明确成功 | 必须继续同 instance GET，不能以 PUT 响应判定 | 只由下列 read-back 行决定 |
| PUT timeout/response loss/明确拒绝 | 只要同 instance 仍成立就必须 GET | 只由下列 read-back 行决定，transport 本身不产出终态 |
| GET == desired Node | 无补偿写 | `selectorApplied`；applied structural generation 不变 |
| GET == index 内旧 Node | 无补偿写 | `selectorNotApplied`，携带 runtimeNodeId |
| GET transport/parse 失败或未知 tag | 无补偿写 | `selectorApplyUnknown(readBackUnavailable/unknownRuntimeNode)` |
| GET 前后 instance/index 漂移 | 不向新 instance 重发 | `selectorApplyUnknown(instanceChanged)` |
| read-back 后 persisted desired 已变化 | 保留 read-back observation，但不声称当前 desired 已应用 | `selectorApplyUnknown(superseded)` + 最新 snapshot |

本 Task 仍不承诺 UrlTest 延迟展示；禁止模拟数据。

## 5. 封闭 IPC、wire contract 与权限

仅 main 窗口新增两个命令及精确 generated permissions：

| Command | Request | Response |
| --- | --- | --- |
| `get_proxy_routing_snapshot` | 无参数 | `SnapshotResult` |
| `mutate_proxy_routing` | `MutateProxyRoutingRequest` | `MutationResult` |

所有 Rust request DTO 层级都使用 `#[serde(rename_all = "camelCase", deny_unknown_fields)]`；所有 enum 用
`#[serde(tag = "type", rename_all = "camelCase")]`。禁止 `serde_json::Value`、`flatten` map、任意 payload、
直接接收 AppState/NodePool/RoutePolicy 或前端提供新 ID。JSON `null` 只在下文明确 nullable 字段出现；
其它字段全部必填且不得用 omitted 表达默认。

### 5.1 顶层与 mutation exact keys

```text
MutateProxyRoutingRequest = { expectedRevision, mutation }

createCustomPool = { type, name, enabled, sources, selection }
updateCustomPool = { type, id, name, enabled, sources, selection }
deleteCustomPool = { type, id }
setManualSelection = { type, poolId, nodeId }
setDefaultTarget = { type, target }
createRoute = { type, name, enabled, matcher, target, insertAt }
updateRoute = { type, id, name, enabled, matcher, target }
deleteRoute = { type, id }
reorderRoutes = { type, routeIds }
applyConfiguration = { type }
```

- `expectedRevision` 为 1..JS safe integer；`insertAt` 为 0..当前 route 数，创建成功由 backend 生成
  `route-<random>`；create 不接受 id，update/delete 必须是现存对应 kind 的 ID。
- `reorderRoutes.routeIds` 必须与当前全部 Route ID 构成完全相等集合，禁止重复、遗漏、未知 ID；成功按
  数组顺序写连续 `priority`。create 的 insertAt 后也归一化全部 priority。

### 5.2 嵌套 discriminated union

```text
PoolSourceDto = {
  providerId,
  filter: {
    regions, protocols, includeKeywords, excludeKeywords,
    includeNodeIds, excludeNodeIds
  }
}

SelectionDto =
  { type: "manual", selectedNodeId: string | null }
  | { type: "urlTest", probeUrl, intervalSeconds, toleranceMs }

MatcherDto =
  { type: "domain" | "domainSuffix" | "application" | "ipCidr", values: string[] }
  | { type: "port", values: number[] }
  | { type: "protocol", values: ("tcp" | "udp")[] }

DefaultTargetDto =
  { type: "followActiveSubscription" }
  | { type: "pool", poolId }
  | { type: "direct" }
  | { type: "block" }

RouteTargetDto =
  { type: "pool", poolId }
  | { type: "direct" }
  | { type: "block" }
```

每种对象只能有所列 exact keys。`followActiveSubscription` 只在 default 映射 `Unconfigured`；Route 禁止。
空 filter 数组表示该维度不过滤；`selectedNodeId` 是唯一允许 null 的 mutation 字段。

### 5.3 边界

- name：trim 后 1..80 Unicode scalar，无 control；filter/matcher 文本：trim 后 1..255 scalar，无 control；
  probe URL：1..2048 UTF-8 bytes，HTTPS，或无 userinfo/query/fragment 的字面 loopback HTTP；
- 每个文本 filter 数组最多 64、protocol 最多 15 且去重、include/exclude NodeId 各最多 5000 且去重；
  同一 NodeId 不得同时 include/exclude；每个 Pool sources 最多 100 且 providerId 去重；
- matcher values 最多 500 且去重；port 为 1..65535；`intervalSeconds` 1..86400；
  `toleranceMs` 0..60000；全局维持最多 100 Pools、500 Routes，匹配需求性能规模。
- reference ID 只接受现有 snapshot 中 exact non-empty string；新 Pool/Route ID 由 backend 生成并查重。
  所有数量/字符/引用边界前后端均检查，后端为权威。

### 5.4 Response

所有 response struct/enum 同样使用 `rename_all = "camelCase"` 与 `deny_unknown_fields`；每个 union 都用
`type` discriminant。除明确标为 nullable 的字段外不允许 null/omitted。

```text
SnapshotResult =
  { status: "ok", snapshot: ProxyRoutingSnapshot }
  | { status: "error", error: QueryError }

QueryError = "busy" | "stateUnavailable" | "runtimeUnavailable"

ProxyRoutingSnapshot = {
  revision,
  desiredGeneration,
  appliedGeneration,
  runtimeState,
  applyState,
  activeSubscriptionId,
  appliedSubscriptionId,
  providers,
  nodes,
  defaultTarget,
  pools,
  routes,
  selectors
}
```

- `revision`、`desiredGeneration` 为 JS safe integer；`appliedGeneration`、`activeSubscriptionId`、
  `appliedSubscriptionId` 可 null，且其余 snapshot keys 均必填。
- `runtimeState` 是 string 闭集：`stopped | ready | transitioning | recoveryRequired`。
- `QueryError` 是上述 string 闭集。Stopped 与 RecoveryRequired 必须返回成功 snapshot，并分别由
  `runtimeState:"stopped"` / `runtimeState:"recoveryRequired"` 表达，不得编码为 QueryError。
- `applyState` 是 exact union：
  - `{type:"applied"}`：Ready 且 active/applied ID 与 desired/applied generation 均相等；
  - `{type:"savedPendingApply"}`：没有在途 Apply，desired/applied tuple 不等且没有更新的失败终态；
  - `{type:"applying", operationId}`：匹配当前 worker operation 的 queued/checking/prepared/persisted/applying；
  - `{type:"savedApplyFailed", operationId, error}`：error 闭集为
    `configurationFailed | stateChanged | stopFailed | startFailed | recoveryRequired`；
  - `{type:"applyUnknown", operationId}`：调用端超时/响应丢失且 worker 尚无可观察终态。
- App/worker 重启后没有内存 operation failure 记录：若 desired 未由 Ready applied identity 证明，统一
  `savedPendingApply`；不得从日志猜 `savedApplyFailed`。Stopped 不等于 applied。

DCR-004 到 Snapshot 的 canonical mapping 由 backend 单独拥有；前端只能渲染返回值，不得根据 generation、
runtime observation、operation 或错误自行重新推导：

| 后端事实 | 唯一 `runtimeState` | 唯一 `applyState` |
| --- | --- | --- |
| Ready，desired/applied structural identity 完全一致 | `ready` | `{type:"applied"}` |
| Ready，desired/applied structural identity 不一致 | `ready` | `{type:"savedPendingApply"}` |
| Stopped，desired 已保存 | `stopped` | `{type:"savedPendingApply"}` |
| queued/checking/prepared/stop/spawn/ready in progress | `transitioning` | `{type:"applying",operationId}` |
| compile/finalize/check/prepare 失败，旧 runtime 仍 Ready | `ready` | `{type:"savedApplyFailed",operationId,error:"configurationFailed"}` |
| Apply 前 state conflict/cancel | 实际确定的 `runtimeState` | `{type:"savedApplyFailed",operationId,error:"stateChanged"}` |
| 旧 child stop 未确认 | `recoveryRequired` | `{type:"savedApplyFailed",operationId,error:"stopFailed"}` |
| 旧 child 已停止，candidate spawn/Ready 失败且清理确定 | `stopped` | `{type:"savedApplyFailed",operationId,error:"startFailed"}` |
| candidate spawn/Ready 失败且清理不确定 | `recoveryRequired` | `{type:"savedApplyFailed",operationId,error:"recoveryRequired"}` |
| caller timeout/response loss，operation 仍在途且没有确定终态 | `transitioning` | `{type:"applyUnknown",operationId}` |
| process restart 后没有旧 operation failure memory，且 desired/applied structural identity 不一致 | 当前实际 `runtimeState` | `{type:"savedPendingApply"}` |

表中的“实际 `runtimeState`”只能来自 backend 当前 owned-runtime observation；失败终态一旦确定便不再返回
`transitioning/applyUnknown`。`operationId` 是产生该次 Apply 状态的同一 operation，前端不得合成。

Snapshot collection element exact keys：

```text
ProviderSnapshot = { id, subscriptionId, name }
NodeSnapshot = { id, providerId, name, protocol }
PoolSnapshot = { id, name, kind, enabled, sources, selection, resolvedNodeIds }
RouteSnapshot = { id, name, enabled, priority, matcher, target }
SelectorSnapshot = { poolId, desiredNodeId, runtimeNodeId, state }
```

- Pool 的 `sources/selection`、Route 的 `matcher/target` 使用第 5.2 节同一 exact DTO；`kind` 闭集为
  `implicitProvider | custom`；`protocol` 闭集为当前 15 个 `ProxyProtocol` camelCase 值。
- `desiredNodeId`、`runtimeNodeId` 可 null；selector `state` 闭集为
  `inSync | savedOnly | notApplied | unknown`。runtime Node 只从当前 AppliedRoutingIndex read-back 得出；
  从未 read-back 或已 Stopped 时为 null，不从 desired 推断。
- safe DTO 不含 URL、server/port、credentials、options、secret、PID、路径、runtime tag 或生成配置。

```text
MutationResult =
  { status: "ok", outcome: MutationOutcome, snapshot: ProxyRoutingSnapshot }
  | { status: "error", error: MutationError, revision: number | null }

MutationOutcome =
  { type: "saved" }
  | { type: "selectorApplied", poolId, nodeId }
  | { type: "selectorNotApplied", poolId, nodeId, runtimeNodeId }
  | { type: "selectorSavedOnly", poolId, nodeId, reason }
  | { type: "selectorApplyUnknown", poolId, nodeId, reason }
  | { type: "applyStarted", operationId }
  | { type: "applyCompleted", operationId }
```

- `selectorNotApplied.runtimeNodeId` 是同一 applied index 中 read-back 的旧 NodeId，必填；
  `selectorSavedOnly.reason` 闭集为
  `runtimeStopped | runtimeNotReady | notInAppliedArtifact | dispatchUnavailable`；
  `selectorApplyUnknown.reason` 闭集为
  `readBackUnavailable | unknownRuntimeNode | instanceChanged | superseded`。
- 结构 mutation（create/update/delete Custom Pool、default、Route CRUD/reorder）成功只返回 `{type:"saved"}`。
  `setManualSelection` 按第 4.3 状态表返回 selector 四类之一；`applyConfiguration` 用当前 snapshot CAS，
  同步 Ready 返回 `applyCompleted`，进入 existing pending worker 返回 `applyStarted`。
- `MutationError` 闭集为 `invalidInput | busy | conflict | notFound | referenceConflict | validationFailed |
  saveFailed | stateUnavailable | runtimeUnavailable | configurationFailed | stateChanged | stopFailed |
  startFailed | recoveryRequired`。`revision` 是错误发生时已知的 authoritative revision；只有在首次 state
  query/observe 前即 `stateUnavailable` 时允许 null。
- Manual selection **保存前**的 invalid/CAS/save failure 返回 error，revision 为当前值；一旦保存成功，
  后续 worker busy/dispatch/PUT/transport/read-back/instance 问题必须返回 `ok + authoritative snapshot` 和
  `selectorSavedOnly/selectorApplyUnknown`，不得退化为 error 或掩盖已保存事实。
- 保存后若 worker dispatch/queue 失败且 backend 能证明 PUT 尚未 dispatch，不发送 PUT/GET，返回
  `ok + selectorSavedOnly(reason:"dispatchUnavailable") + authoritative snapshot`；snapshot 必须反映新的
  persisted desired selection，且 applied structural generation 不变。`selectorApplyUnknown` 只允许用于
  PUT 可能已经发生，或 read-back 无法证明 runtime 结果的场景。
- `runtimeUnavailable` 只用于 `applyConfiguration` 尚未开始且没有任何本次持久提交的入口失败，或 query
  无法取得 runtime observation；不能用于 Manual post-save 阶段。
- `applyConfiguration` 的 compile/check/stop/spawn/Ready 等明确失败返回 error + revision；随后前端必须
  重新 query，snapshot 按上述唯一 `applyState` 映射保存结果和 runtime 事实。pending 只返回
  `applyStarted`，以 operationId 的 observation/query 收敛。

前端不得 patch Domain state；每个 ok 都用返回 snapshot 整体替换共享 Provider，error 则只比较 revision：
不同或 null 时重新 query。日志只记录 mutation type、封闭结果和不敏感稳定 ID，不记录 matcher values、
Node 名称或 core body。

预计接线仍限于 `application/proxy_routing.rs`、`commands.rs`、`lib.rs`、`singbox/clash_api.rs`、
`application/selected_subscription.rs`、`src/lib/proxy-routing.ts`、两个页面与局部组件、两个 command/
permission。无新依赖、lockfile、UI framework、任意网络/文件权限。

## 6. 验收与验证路由

### 6.1 Unit / contract

- Rust 枚举所有 mutation 与嵌套 union 的 round-trip/unknown tag/unknown field/missing/null/额外 key、非法
  create ID、重复/遗漏 reorder、边界±1、过大集合；证明无 Value/flatten/AppState request。
- 状态测试覆盖 CAS conflict、订阅刷新导致 observe bump、save failure 不推进 generation/revision、成功
  structural 保存推进 generation/revision、Manual 保存只推进 revision、reconcile-only 两者都不推进、
  双页面消费同一 provider snapshot。
- DCR-019 矩阵覆盖 active-only 基础、多个 enabled Custom source/filter 精确闭包、未引用 sibling 排除、
  disabled/empty/manual drift/default/route conflict。
- Rust/TypeScript 对 SnapshotResult、QueryError 三个闭集值、runtimeState、applyState、全部 collection DTO、MutationResult/
  MutationOutcome 和 error response 做 exact serialization/parser 正反 fixture；覆盖 unknown/missing/null/
  extra keys 与每个闭集非法值。
- 表驱动测试逐项覆盖第 5.4 节 DCR-004 canonical mapping，断言 backend 返回唯一的 runtime/apply 组合且
  frontend parser/Provider 不重新推导；覆盖 restart 丢失 failure memory 后回到 `savedPendingApply`。
- TypeScript 覆盖共享 provider invalidation、切页/reload dirty 重建、Dialog 保留及四种 selector outcome、
  `selectorNotApplied` 与 Apply outcome 文案；任何 ok 都整体替换 snapshot，不 locally patch。

### 6.2 Integration / Runtime

- 隔离 state.json 证明 mutation/restart 往返、备份/原子替换；与订阅写竞争只有 busy/conflict，不丢对象。
- Mock worker 覆盖第 4.2 全状态表，并证明旧 runtime 使用 applied artifact/identity，绝不从新 AppState
  重算旧投影或自动恢复。
- Mock worker 覆盖 Manual 保存成功后 dispatch 前 queue failure，证明没有 PUT/GET、返回
  `selectorSavedOnly(dispatchUnavailable)`、snapshot 含新 desired 且 applied generation 不变；另以“PUT 可能
  已发生”正例证明只能进入 read-back/`selectorApplyUnknown` 路径。
- 固定 sing-box 1.14.0 最小测试先锁定 selector PUT/GET shape，再验证 PUT success、明确拒绝、response loss、
  GET other/desired/unknown、instance drift、Stopped；断言同 child 切换、selector 不改变 structural
  generation、existing dirty 不被清除，以及无 secret/core body 泄露。覆盖“先结构 Save 再 Manual”、
  “dirty 下 reconcile-only”和“Manual 期间并发结构 Save”。只验证 Veyra 接线，不重跑协议/DNS/IPv6/
  包级矩阵。

### 6.3 UI / Visual

- mockIPC 覆盖 Pool create/edit/delete、expand、Manual 即时选择、UrlTest、default、Route CRUD/reorder、
  Save→dirty→Apply，以及 loading/empty/error/busy/conflict/disabled/selected/Dialog/Notice。
- 实际 Windows Tauri 验证 keyboard/focus、两页面同步、reload 后 dirty、Apply 各终态和四种 selector 结果；
  浅/深截图。Proxies 对 `windows-light/proxies.png`；Routing 无专属 reference，只按冻结 token/composition。
- parity 表必须逐项记录 Shell、Page header、density、group/list、row、typography、toolbar/control、border/
  radius/background、selected/disabled/expand/Dialog/Notice 的明确差异与 PASS/FAIL。

### 6.4 Repository checks

- `pnpm lint`、相关 Vitest、`pnpm build`；Rust 精确非零测试、cargo fmt/clippy；
- capability 正反调用、delivery diff、无 schema/依赖/锁文件/System Proxy/TUN/任意 API/path 权限变化。

## 7. Non-Scope 与回滚

- 不实现 Settings、RuleSet、Inbound、DNS、TUN、Connections、Logs、节点编辑、服务 preset、Clash
  mode/chain、Mihomo schema/API、任意 JSON 编辑或新 scheduler。
- 不重构 AppShell、Sidebar、Router、SubscriptionPage、StateStore schema、RuntimeIntent 或 Compiler；
  不修改 `activePage + hidden` 和订阅业务事件语义。
- DCR-019 上线并保存跨订阅 Custom Pool 后默认只允许 roll-forward。必须回滚旧 projection 时，先用新
  版本把 default/enabled Routes 移到 active subscription 可用目标，禁用跨订阅 Custom Pool，并成功
  Apply；再回滚代码。运行中不自动重建 child；Stopped/重启后以旧 projection 显式 Start/Apply 验证。
- UI/IPC/manager 可以独立移除且 StoredStateV6 可读，但“schema 可读”不等于跨订阅行为可运行。任何
  未完成上述数据收敛的回滚必须标记 BLOCKED，不伪称安全回滚。

## 8. Review Finding closure

- `T012-DESIGN-001`：由 DCR-019 与第 4.1 节明确 amendment、根/边/排除/冲突矩阵。
- `T012-DESIGN-002`：第 4.3 节冻结 owned-instance binding、PUT 后 GET、unknown 终态与不回滚理由。
- `T012-DESIGN-003`：第 3.1、4.2 节冻结 generation identity、两阶段用户语义及完整失败状态表。
- `T012-DESIGN-004`：第 5 节冻结所有 variant exact keys、嵌套 union、边界和 deny_unknown_fields。
- `T012-DESIGN-005`：第 7 节与 DCR-019 冻结 roll-forward/default 及有界回滚前数据收敛。
- `T012-DESIGN-006`：第 3.2 节冻结进程内 routingRevision、CasMaterial 与单一 Provider 同步。
- `T012-DESIGN-007`：第 3.1、4.3 节把 structural generation 与 hot-swappable selection 正交化，新增
  AppliedRoutingIndex，并冻结 dirty/并发时 selector 不改变 applied generation。
- `T012-DESIGN-008`：第 5.4 节冻结完整 response exact union、snapshot DTO、Apply/selector outcome、
  error revision 与 post-save 结果映射。

用户已接受本轮三项产品语义和 DCR-019 amendment；其余工程细节仍须以新身份通过独立 Technical
Design Review。Review PASS 前不得物化 Task 或开始实现。
