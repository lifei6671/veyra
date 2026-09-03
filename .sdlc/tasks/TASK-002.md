---
id: TASK-002
milestone_ref: M2
dependencies: [TASK-001]
risk: HIGH
status: DONE
design_refs:
  - .sdlc/design/foundation.md
approval_refs:
  - .sdlc/state.yaml#gates.technical_design
  - USER:lifei 2026-09-03 serde_yaml_ng 0.10.0
---

# TASK-002：版本化状态存储与订阅归一化

## 本次需求

实现只由类型化领域模型表示的版本化 `AppState`，以及其 `JsonStateStore`、顺序迁移、备份与损坏恢复。
实现 JSON、Clash YAML（仅 `proxies`）、URI 列表与 Base64 递归识别的订阅解析和归一化；候选解析、
校验或保存失败时，既有有效状态不得被替换。

## Scope

### allow

- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：仅新增已批准的 `serde_json ^1` 与
  `serde_yaml_ng = "0.10.0"` 及其正常传递依赖。
- `src-tauri/src/domain/state.rs` 与 `src-tauri/src/domain/mod.rs`：`AppState`、稳定身份、
  Subscription、Provider、ProxyNode 及其引用校验。
- `src-tauri/src/storage/{store,snapshot,migration,validation}.rs` 与 `src-tauri/src/storage/mod.rs`：
  整体 load/save、临时文件原子替换、备份、损坏恢复和内存顺序迁移。
- `src-tauri/src/subscription/{parser,normalize}.rs` 与 `src-tauri/src/subscription/mod.rs`：格式识别、
  支持输入的节点草稿和到类型化 `ProxyNode` 的归一化。
- 上述 Rust 模块的单元测试及 `src-tauri/tests/fixtures/state/`、`src-tauri/tests/fixtures/subscriptions/`
  下的最小 fixture。

### deny

- SQLite、SQLx、任意数据库、网络订阅下载、Tauri 新 capability、前端配置编辑、sing-box 编译/运行、
  Clash API、System Proxy、TUN、UAC、原生平台依赖与发布。
- Clash 的 `proxy-groups`、规则、rule-providers、DNS、TUN 或任意原始 sing-box JSON 的持久化/编译。
- 未获确认的依赖、手写 YAML 解析器、任意路径输入或静默重置损坏状态。

## 子功能

### SF-001：类型化状态与引用校验

**需求：** 建立含 schema 版本的领域 `AppState`，以稳定 ID 表达 Subscription、Provider 与 ProxyNode；
不让领域模型依赖 Tauri、文件系统、`serde_json::Value` 或 sing-box tag。

**验收：** Provider 必须引用现有 Subscription，Node 必须引用现有 Provider；重复稳定 ID 与悬空引用
被明确拒绝，且错误不泄露订阅凭据。

**验证：** Rust 单元测试覆盖有效多订阅状态、重复 ID、缺失 Subscription 与缺失 Provider。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-002/implementation.yaml#EVIDENCE-TASK-002-001`

### SF-002：JSON 状态、迁移与恢复

**需求：** `StateStore` 仅暴露整体 `load`/`save` 语义。读取按“schema → 内存迁移 → 当前模型 →
引用校验”执行；保存以临时文件、flush、同步与替换提交，并维护当前与升级前备份。

**验收：** 当前状态能往返恢复；旧版本 fixture 只迁移一次；不支持版本、损坏主文件或无效备份显式失败。
迁移、校验、写入或恢复失败均不覆盖最后有效状态。

**验证：** Rust fixture 测试覆盖 save/load、原子替换、迁移幂等、升级前备份、损坏恢复、无有效备份和
引用校验失败。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-002/implementation.yaml#EVIDENCE-TASK-002-001`

### SF-003：订阅解析与归一化

**需求：** 按 JSON、Clash YAML、URI 列表、Base64 递归识别顺序解析；Clash YAML 仅提取 `proxies`。
将支持协议产出 `ProxyNodeDraft` 再归一化为类型化 Node，非法或不支持项以类型化 skip/error 表示。

**验收：** 三个订阅可同时保留正确的节点归属；YAML 的非 `proxies` 内容不进入领域状态；解析失败不
替换既有节点。

**验证：** Rust fixture 测试覆盖 JSON、YAML、URI、Base64、无效输入和多订阅归属。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-002/implementation.yaml#EVIDENCE-TASK-002-001`

## Task 独立验收

**验收：** 应用可以从有效版本化 JSON 状态恢复类型化 AppState；在支持的订阅输入间保持稳定节点归属；
迁移、损坏恢复、解析和持久化任一失败均保留最后有效状态，不新增越权 Tauri 或系统能力。

**验证：**

1. `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、
   `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` 与
   `cargo test --manifest-path src-tauri/Cargo.toml`；
2. `pnpm lint`、`pnpm test`、`pnpm build`；
3. 检查 Cargo 依赖、Tauri capability 与 delivery diff 均在本 Task Scope 内。

**acceptance_status：** PASSED
**evidence_refs：** `.sdlc/evidence/TASK-002/implementation.yaml#EVIDENCE-TASK-002-001`、
`.sdlc/evidence/TASK-002/code-delivery-review.yaml#EVIDENCE-TASK-002-REVIEW-001`
