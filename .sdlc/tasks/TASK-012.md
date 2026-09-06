---
id: TASK-012
milestone_ref: M7
dependencies: [TASK-011]
risk: HIGH
status: DONE
requirement_ref: docs/veyra.md
requirement_identity: sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-004-runtime-update-failures.md
  - docs/decisions/DCR-017-ui-integration-boundary.md
  - docs/decisions/DCR-018-subscription-use.md
  - docs/decisions/DCR-019-task012-custom-pool-projection-amendment.md
  - docs/ui/veyra-ui-spec.md
  - .sdlc/design/TASK-012-proxy-routing.md
  - .sdlc/design/TASK-012-proxy-routing-candidate-004.json
approval_refs:
  - .sdlc/evidence/TASK-012/design-review-004.json
  - .sdlc/evidence/TASK-012/design-approval-004.json
  - .sdlc/evidence/TASK-012/change-control-001.json
---

# TASK-012：出口组选择与分流编辑

## 本次需求

在不改变 Veyra 既有 AppState、StateStore、RuntimeSupervisor、Router 和订阅业务行为的前提下，
交付基于真实 `ProxyNode`、`NodePool`、`RoutePolicy` 与当前受管 sing-box 实例的 Proxies 和 Routing 页面。
页面必须消费同一个 authoritative snapshot，支持 Custom Pool、默认出口、Route 编辑，以及 Manual Pool
即时持久化、Clash API 原地切换和 read-back；结构编辑遵循明确的 `Save → Apply` 用户语义。

本 Task 以 Candidate-004
`sha256:c0d90aba66e5dbe20e2187b83138288f57c119b162f4fab933c16205aa9138a6` 为冻结技术设计，
并落实 DCR-019 的 enabled Custom Pool 最小跨订阅闭包。参考 Clash Verge Rev 仅用于页面 composition 和
视觉语言，不复制其品牌、文案、Mihomo API、配置模型或业务能力。

本 Task 不实现 Settings、RuleSet、LocalInbound、DNS、TUN、Connections、Logs、节点凭据编辑、服务 preset、
任意 JSON 配置编辑或新调度器；不新增依赖、schema version、持久字段、通用 mutation API 或额外 IPC。
`T012-DELIVERY-001` 的批准 Scope override 仅允许 `SingBoxCompiler` 将默认 Pool/Direct/Block 映射为
`pool-*`/`direct`/`block`，并将 final outbound 合法性校验限定为这三类；Unconfigured 与缺失 Pool 继续拒绝。

## Scope

### allow

- `src-tauri/src/application/{mod.rs,proxy_routing.rs,selected_subscription.rs,
  managed_observation_runtime.rs}`：authoritative query/mutation、进程内 `routingRevision`、DCR-019 投影、
  structural Apply、Manual selector worker 协调与 `AppliedRoutingIndex`。
- `src-tauri/src/singbox/{clash_api.rs,runtime.rs}`：只实现当前 owned runtime 的封闭 selector PUT/GET、
  applied artifact/index 所需的最小内部接线；不得改变 Compiler 或新增运行模式。
- `src-tauri/src/singbox/compiler.rs`：仅按 `.sdlc/evidence/TASK-012/change-control-001.json` 实现默认
  Pool/Direct/Block 的精确 `route.final` 映射、对应的封闭 final outbound 校验及定向测试。
- `src-tauri/src/{commands.rs,lib.rs}`、`src-tauri/build.rs`、`src-tauri/capabilities/default.json`：只新增
  `get_proxy_routing_snapshot`、`mutate_proxy_routing` 两个 main-only command、精确 DTO 和对应权限。
- `src/lib/proxy-routing.ts`、`src/App.tsx`、`src/styles.css`、`src/components/proxies/**`、
  `src/components/routing/**` 及直接相关前端测试：共享 snapshot Provider、Proxies/Routing 页面、Dialog、
  loading/empty/error/busy/selected/disabled/Notice 与键盘/focus 行为。
- 上述生产路径中的定向 Rust/TypeScript 测试、受控 fixture，以及 `.sdlc/evidence/TASK-012/**` 中不包含
  凭据、secret、runtime tag、路径、原始 core body 或用户真实订阅的有界 Evidence。

### deny

- 不修改 `src-tauri/src/domain/state.rs` 的 AppState/NodePool/RoutePolicy 持久模型，不新增 schema version 或
  migration；除已批准的默认出口映射和封闭合法性校验外，不修改 `SingBoxCompiler`；不修改 RuntimeIntent、
  RouteTarget、tag 规则或 StoredStateV6。
- 不修改 AppShell、Sidebar、Router、`activePage + hidden`、SubscriptionPage、DocumentEditor、Settings，
  也不实现 TASK-013～TASK-017。
- 不引入新的 UI library、CSS framework、runtime dependency、lockfile、任意文件/网络/进程权限、非 loopback
  API、Windows PlatformAdapter 行为、System Proxy、TUN、DNS 或 RuleSet。
- 不把 AppState、`serde_json::Value`、任意 payload、URL、secret、header、runtime tag 或生成 JSON 暴露给
  前端；不从前端 patch authoritative Domain state。
- 不 commit、push、release、访问用户真实订阅或执行生产/破坏性操作。

## SF-001：Authoritative routing state、CAS 与封闭 IPC

**依赖：** 无 Task 内前置；可与 SF-003 的纯前端 fixture/composition 并行，但 backend/IPC 文件由本 SF 独占。

**需求：** 新增唯一 `ProxyRoutingManager`，在共享 `StateAccessGate` 下读取最新完整 AppState，以
`CasMaterial` 驱动进程内 `routingRevision`，对全部 Pool/Route/default/selector 持久 mutation 执行
`expectedRevision` CAS、整体校验和原子保存。结构 mutation 推进 desired generation；Manual selection
只推进 revision。两个命令及全部请求、响应、嵌套 DTO、边界和 error/outcome 必须与 Candidate-004 的
封闭 discriminated union 完全一致，并使用 `deny_unknown_fields`。

**Acceptance：** Proxies 与 Routing 的查询得到同一个包含固定 exact keys 的 authoritative snapshot；任一
成功 mutation 返回完整最新 snapshot，前端无须也不得本地 patch。CAS conflict、busy、非法边界、引用冲突、
保存失败均不产生部分写；成功结构保存只推进一次 desired generation/revision，Manual 保存不推进 generation。
`QueryError` 只允许 `busy/stateUnavailable/runtimeUnavailable`，Stopped/RecoveryRequired 通过成功 snapshot
表达。权限只允许 main 窗口调用精确的两个命令。

**Verification：** Rust 表驱动测试覆盖全部 mutation、exact keys、unknown/missing/null/extra field、边界±1、
重复/遗漏 reorder、CAS、订阅刷新 observe bump、save failure 和 generation/revision 规则；TypeScript exact
parser/encoder fixture 穷举成功与错误闭集；capability 正反调用证明无第三个 routing IPC 或额外权限。

**implementation_status：** ACCEPTED
**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-012/acceptance-005.json`

## SF-002：DCR-019 投影、结构 Apply 与 Manual hot switch

**依赖：** SF-001 的 manager、snapshot 和 mutation contract。

**需求：** 按 DCR-019 修改 `project_selected_runtime`：active subscription 保持基础隔离，只有显式 enabled
Custom Pool 沿 `PoolSource + NodeFilter` 引入精确跨订阅闭包。结构 Apply 复用既有 runtime worker 与 DCR-004，
由 backend 唯一映射 canonical `runtimeState + applyState`。每次完整 Apply Ready 从实际 applied artifact 建立
进程内 `AppliedRoutingIndex`；Manual selection 保存后仅在 Pool/Node 均存在于当前 index 时，对同一 owned
instance 执行固定 PUT 后 GET read-back。

**Acceptance：** 未显式引用的 sibling、disabled/空 Pool 和 filter 未命中节点不进入投影；冲突继续返回
`selectionConflict`。只有完整 Apply Ready 对齐 applied/desired structural generation；失败保留 persisted
desired，并严格呈现 Candidate-004 的 DCR-004 canonical mapping，不从已修改 AppState 重算旧 runtime。
Manual selector 覆盖 saved failure、Stopped、非 Ready、not-in-artifact、pre-PUT dispatch unavailable、PUT
成功/拒绝/timeout/response loss、read-back desired/old/failure/unknown、instance drift 和 superseded；任何
transport error 后只以 authoritative read-back 判定，selector 结果绝不改变 applied structural generation。

**Verification：** DCR-019 正反矩阵测试 active-only、多个 enabled Custom source/filter、disabled/empty、
manual drift、default/route conflict；Mock worker 表驱动覆盖全部 Apply canonical mapping 与 Manual 状态序列，
包括 `dispatchUnavailable` 不发送 PUT/GET、`selectorApplyUnknown` 仅用于 PUT 可能发生或 read-back 无法证明。
固定 sing-box 1.14.0 受控 fixture 锁定 selector PUT/GET shape，并验证同 child、同 index、无 secret/core body
泄露；不重跑协议、DNS、IPv6 或包级矩阵。

**implementation_status：** ACCEPTED
**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-012/acceptance-005.json`

## SF-003：共享 Snapshot Provider 与 Proxies/Routing 页面

**依赖：** 可基于冻结 wire contract 先完成 mock fixture；真实 IPC 集成依赖 SF-001，runtime 状态集成依赖 SF-002。

**需求：** 在现有 `activePage + hidden` 页面切换中新增一个 WebView 内唯一的
`ProxyRoutingProvider`。Proxies 页面实现出口组/全部节点视图、Custom Pool Dialog、expand/collapse 和
Manual selector；Routing 页面实现 DefaultTargetRow 与 Route CRUD/reorder Dialog。两页只使用 Veyra 真实
领域字段和同一 snapshot，并保持现有 AppShell、Sidebar、PageHeader、token 与控制组件边界。

**Acceptance：** 两页覆盖 loading、empty、error、busy、conflict、disabled、selected、inline pending、Dialog、
Notice 和键盘/focus；每个 ok response 整体替换共享 snapshot，error revision 不同或 null 时重新 query。
结构 Save 后持续显示未应用状态并由明确 Apply 操作收敛；Manual 四类 selector outcome 给出准确反馈。
页面不得展示节点地址、凭据、options、API secret、runtime tag、生成配置或虚构延迟；520×520 和 960×640
无横向溢出，浅色/深色遵守 `veyra-ui-spec.md`。

**Verification：** Vitest/mockIPC 覆盖 Pool create/edit/delete、Manual、UrlTest、default、Route CRUD/reorder、
Save→dirty→Apply、共享 Provider 同步、exact response parser，以及所有可见状态、Dialog 保留输入、context
menu 和 keyboard/focus。实际 Windows Tauri 验证两页面切换、reload dirty、Apply/selector 终态，并保存浅/
深截图及逐项 parity 表；Proxies 参考 `windows-light/proxies.png`，Routing 只按冻结 Veyra token/composition。

**implementation_status：** ACCEPTED
**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-012/acceptance-005.json`

## Task 独立验收

在同一隔离 state 与同一 Windows Tauri 进程中准备至少两个订阅，完成一条真实用户链：创建 enabled
跨订阅 Custom Manual Pool、选择节点并确认 read-back；编辑 default 与多条 Route、保存后观察
`savedPendingApply`，显式 Apply 后观察 Ready/applied；制造一次可恢复 Apply 失败并确认旧实际 runtime identity
和 persisted desired 均准确；在 Proxies/Routing 间切换确认同一 revision/snapshot；重启应用确认持久 Domain、
dirty 状态和非持久 routingRevision/operation memory 的规定行为。

Task 验收还必须对账全部 SF Evidence、Candidate-004/DCR-019、delivery-owned inventory 与最终 target identity；
运行 `pnpm lint`、相关 Vitest、`pnpm build`、Rust 精确非零测试、
`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` 和
`cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`。实际 UI Evidence 覆盖浅/深主题、
520/960 窗口、loading/empty/error/busy/selected/disabled/Dialog/Notice，不以 mock、build 或 core 启动代替
真实 WebView 操作。独立 Delivery Review 必须覆盖 backend state/runtime、IPC/security、frontend/state/UI
和跨模块 integration，且无 P0/P1 Finding。

**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-012/delivery-target-005.json`、`.sdlc/evidence/TASK-012/delivery-gate-005.json`、`.sdlc/evidence/TASK-012/acceptance-005.json`

## Risk

- **状态一致性：** persistent desired、applied runtime identity、selector runtime 与进程内 revision 正交；
  对应 CAS、canonical mapping、same-instance read-back 和 restart 测试必须全部通过。
- **运行时副作用：** Apply 会停止/启动 owned child；只使用隔离数据和受控 fixture，实际 Windows 验收前确认
  不操作用户真实实例。DCR-004 的失败/清理语义不得弱化。
- **回滚：** DCR-019 数据一旦保存默认 roll-forward；回滚旧 projection 前必须先收敛 default/Route、禁用
  跨订阅 Custom Pool 并成功 Apply。UI/IPC 可移除不等于旧 projection 可安全运行。
- **安全：** main-only capability、固定 loopback selector API、backend-owned URL/header/tag、safe DTO 和脱敏
  Evidence 是硬边界；不得记录或回显凭据、secret、原始 core body 或用户真实订阅。

## Execution checkpoint

Implementation checkpoint `implementation-checkpoint-001.json` 已完成 Candidate-004 内可执行的主要分区。
`delivery-review-001.json`、`delivery-review-003.json` 和 `delivery-review-004.json` 的 REWORK 已按各自冻结
Finding 收敛；其中 Target-005 仅补齐 `T012-DELIVERY-006` 要求的 mockIPC 完整交互矩阵、实际 WebView
loading/error、跨页/reload/restart authoritative read-back，并对既有 FIXED 项做回归验证。

`T012-DELIVERY-001` 已由 USER:lifei 在 `.sdlc/evidence/TASK-012/change-control-001.json` 批准最小 Scope
override：只增加默认 Pool/Direct/Block 的精确 Compiler 映射和对应封闭校验；其余 Candidate-004、DCR-019、
RuntimeIntent、RouteTarget、持久化、wire、tag、runtime mode、依赖与 Compiler 结构继续冻结。
- **规模：** 以 100 subscriptions、5000 nodes、100 pools、500 routes fixture 测量；未有失败证据前不引入
  virtualizer 或新 UI framework。

## Implementation candidate 004 (2026-09-06)

冻结交付目标为 `delivery-target-004.json`，身份
`sha256:cabd42f6db8677468fafa7166c65559a5ab4df61bae20b30d3288475733681b3`。在不改变 Candidate-004、
DCR-019 或 Change Control 001 边界的前提下，补齐了 runtime observation 驱动的唯一 Provider 刷新、
Manual selector 点击目标的瞬态 inline pending，以及页面状态测试、真实可恢复 Apply failure、空态、busy、
Context Menu、运行实例退出刷新、状态回读和逐项 Visual Parity Evidence。

`verification-004.json` 记录 frontend 116 tests/lint/build、Compiler 38、routing manager 16、DCR-019 projection 6、
Apply 3、Manual 3、固定 selector 1、持久 default-target 完整链路 1、fmt/clippy/diff-check 全部 PASS。
`native-ui-final-004.json` 与 `native-integration-readback-001.json` 绑定最终 Windows Tauri executable 和安全回读。
独立 `delivery-review-004.json` 对 target-004 判定 REWORK，仅 `T012-DELIVERY-006` 未闭合；其它 Finding 保持
FIXED。本段 target-004 身份与证据作为历史 checkpoint 保留。

## Implementation candidate 005 (2026-09-06)

冻结交付目标为 `delivery-target-005.json`，身份
`sha256:c5b325d3de2d5c91a9d09fb44a075dc3e76be63be104ee940b3d0e0df4628762`。Target manifest
35 项逐项哈希、Git HEAD `f2dc91c3e1f444e0b1d3dca6c60e80a312dba443` 和最终 Windows Tauri executable
`sha256:22e6816c608db4e23f08f2cc5779de8695deed1c31ed59fa9116c4a04ba09275` 已重新核验一致。

`verification-005.json` 记录 frontend 116 tests/lint/build、冻结 Rust 分区、fmt/clippy、最终 Tauri build、
mockIPC 31 项交互和 diff-check PASS。`native-integration-readback-001.json` 覆盖同一 authoritative snapshot
的跨页、reload、process restart 与实际 query error；`native-ui-final-004.json` 覆盖最终 WebView loading、error、
busy 和 context menu，并保留完整浅/深、520/960 状态截图清单。Target-005 随后进入独立 Delivery Review；
当时 Task 与各 SF 的人工验收仍为 `PENDING`。

独立 `delivery-review-005.json` 已对上述精确身份给出 `PASS`：35/35 manifest、HEAD 与 native executable
身份匹配，`T012-DELIVERY-001`～`010` 全部 `FIXED`，无新增 P0/P1 Finding。`delivery-gate-005.json`
据此将 TASK-012 Delivery Gate 记录为 `PASSED`；该结果随后由下述 USER:lifei Human Task acceptance 收口。

## Human Task acceptance (2026-09-06)

USER:lifei 已明确验收通过 TASK-012。`.sdlc/evidence/TASK-012/acceptance-005.json` 将 SF-001～SF-003
及 Task 级验收记录为 `PASSED`，本 Task 状态更新为 `DONE`。该验收不授权提前实施 TASK-013、push、
release 或 deployment。
