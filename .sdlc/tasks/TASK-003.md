---
id: TASK-003
milestone_ref: M3
dependencies: [TASK-002]
risk: HIGH
status: READY
design_refs:
  - .sdlc/design/foundation.md
approval_refs:
  - .sdlc/state.yaml#gates.technical_design
---

# TASK-003：出口组、分流与语义配置编译

## 本次需求

在既有 `AppState` 上实现类型化的 `NodePool`、`RoutePolicy` 和 `RuntimeIntent`，并将经验证的
语义模型确定性编译为受管配置快照。普通路由只能指向出口组、`Direct` 或 `Block`；本任务不启动
sidecar、不变更系统代理，也不接受前端传入任意配置片段。

## Scope

### allow

- `src-tauri/src/domain/state.rs` 与 `src-tauri/src/domain/mod.rs`：Pool、Route、RuntimeIntent、稳定 ID、
  成员解析和跨对象引用校验。
- `src-tauri/src/storage/{store,migration,validation}.rs`：将当前存储模型演进为包含 Pool 与 Route 的
  受控 schema，并完成仅内存顺序迁移与回归测试。
- `src-tauri/src/singbox/{compiler,mod}.rs`：仅实现类型化 `ConfigCompiler`、稳定 tag 和确定性受管配置
  快照生成；不执行或写入 sidecar 配置文件。
- 上述模块的 Rust 单元测试、`src-tauri/tests/fixtures/state/` 下最小状态迁移 fixture。

### deny

- 新增依赖、Tauri command/capability、前端配置页面、网络下载、任意 JSON 输入、任意文件路径、
  sing-box binary 的下载、调用、启动或 `check`，以及任何 System Proxy、TUN、UAC、Windows API 或发布操作。
- 完整 RuleSet、DNS、LocalInbound、原生 Profile、任意路由直接指向 Node、运行时 Build/Apply、
  进程或配置文件写入。
- 未经确认的 schema 跳跃、静默丢弃 Pool/Route、将 UI 或 sing-box JSON 作为领域事实源。

## 子功能

### SF-001：出口组与分流领域模型

**需求：** 定义稳定的 `PoolId`、`RoutePolicyId`、`NodePool`、`PoolSource`、`NodeFilter`、
`SelectionPolicy`、`RoutePolicy`、`TrafficMatcher` 与封闭 `RouteTarget`；把它们纳入 `AppState` 的整体校验。

**验收：** PoolSource 必须引用现有 Provider；Route 的 Pool 目标必须存在；重复稳定 ID、空 Pool
来源、无效筛选与直接 Node 路由均被拒绝。手动与 URLTest 选择语义可被区分。

**验证：** Rust 单元测试覆盖有效多 Provider Pool、筛选成员、悬空 Provider/Pool、重复 ID、无效
Manual/URLTest 与 `Direct`/`Block` 目标。

**implementation_status：** NOT_STARTED
**acceptance_status：** PENDING

### SF-002：RuntimeIntent 与确定性语义编译

**需求：** 从验证后的 `AppState` 构建 `RuntimeIntent`，解析 Pool 成员，生成不依赖显示名称的稳定 node/
pool tag；仅以类型化输入编译受管配置快照。

**验收：** 相同有效状态重复编译结果字节一致；Manual Pool 编译为 selector、URLTest Pool 编译为
urltest；路由只能引用 pool tag、direct 或 block，且错误不包含节点凭据。

**验证：** Rust fixture/单元测试覆盖确定性、tag 稳定性、Manual、URLTest、Direct、Block、空成员与
非法引用拒绝；不运行 sing-box。

**implementation_status：** NOT_STARTED
**acceptance_status：** PENDING

### SF-003：状态 schema 演进

**需求：** 将 Pool/Route 保存为当前版本化状态，并以明确 V1→V2 内存迁移补全旧状态的空集合。

**验收：** V1 fixture 只迁移一次且保留已有订阅、Provider、Node；迁移后完整引用校验仍失败时不覆盖
有效快照；未来 schema 显式失败。

**验证：** Rust fixture 测试覆盖 V1→V2、迁移幂等、带 Pool/Route 的 V2 往返，以及无效迁移候选的
拒绝行为。

**implementation_status：** NOT_STARTED
**acceptance_status：** PENDING

## Task 独立验收

**验收：** 应用可从版本化状态恢复正确的出口组和分流关系，并从同一有效 `AppState` 生成稳定、可检查的
受管配置快照；无效引用、候选迁移或编译失败不得替换最后有效状态，也不触发 sidecar 或系统能力。

**验证：**

1. `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、
   `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` 与
   `cargo test --manifest-path src-tauri/Cargo.toml`；
2. `pnpm lint`、`pnpm test`、`pnpm build`；
3. 检查 delivery diff、Cargo 依赖、Tauri capability 与运行进程均在本 Task Scope 内。

**acceptance_status：** PENDING
