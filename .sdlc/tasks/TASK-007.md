---
id: TASK-007
milestone_ref: M6
dependencies: [TASK-006]
risk: HIGH
status: DONE
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
approval_refs:
  - USER:lifei 2026-09-04 DCR-002 Human Technical Design Gate sha256:1a02b56d8861bb7c936599d952c822fa99667e8109d6b9b95427cb49e1966989
  - USER:lifei 2026-09-04 TASK-007 Human Task acceptance sha256:c83dd3e2df82c45ab6cb9d7c46c1e16ef266093e3db8abc2cf73686b4ea1442c
---

# TASK-007：全协议领域模型与受限订阅转换

## 本次需求

将现有 V2 的部分节点模型扩展为 sing-box 1.14.0 的 15 种非 Tor 用户代理协议，并交付强类型
Clash、sing-box、URI 与粘贴文本归一化及全有或全无的 Provider 替换事务。Tor 保持 `BLOCKED`：本 Task
不下载、打包、执行或宣称运行 Tor。

## Scope

### allow

- `src-tauri/src/domain/{state.rs,mod.rs}`：V3 强类型协议、协议专有选项、TLS/Transport、
  `default_target: Unconfigured` 与整体校验。
- `src-tauri/src/storage/{migration.rs,store.rs}`：V2→V3 显式转换、升级前备份、原子提交和失败恢复。
- `src-tauri/src/subscription/`：Clash/sing-box/URI/粘贴文本的受限解析、canonical identity、
  脱敏诊断、15 协议 fixture 与批次归一化。
- `src-tauri/src/application/` 中与 Provider 节点替换直接相关的内部服务及 Rust tests：仅从完整候选
  `AppState` 原子提交，失败保留旧状态与运行配置。
- `src-tauri/src/singbox/compiler.rs`：仅为本 Task 新增、尚未由当前编译器表达的协议返回既有
  `UnsupportedNodeProtocol`，以保持编译边界显式且可构建；不生成配置、不改变现有协议输出，
  完整协议配置生成仍属于 TASK-008。

### deny

- `src/` UI、Tauri commands/capabilities、sidecar、除上述显式拒绝分支外的 Compiler、Clash API、SystemProxy、TUN、UAC、
  新监听器、依赖/lockfile 变更。
- Tor binary、用户 executable path/torrc/data directory、原始 sing-box JSON 透传、部分节点提交，
  以及任何不含明确版本/字段映射的 serde_json::Value 领域存储。

## 子功能

### SF-001：V3 领域与迁移

**需求：** 用强类型 `ProtocolOptions` 表达 15 种非 Tor 协议；V2→V3 保留既有 `NodeId`、Pool、选择和
路由引用。缺少可证明默认路由的 V2 状态迁为 `Unconfigured`，不能自动选择节点/Pool/Direct。

**验收：** V2 所有可映射节点迁移为语义等价的 V3；任何无法映射、重复 ID 或整体校验失败都不写入半迁移
状态，旧文件与运行配置保留。

**验证：** 映射表驱动的 V2→V3 单测、升级前备份/原子写/恢复测试、NodeId 与引用逐字断言、
`Unconfigured` fail-closed 断言。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

**evidence_refs：** `.sdlc/evidence/TASK-007/implementation.yaml#EVIDENCE-TASK-007-001`

### SF-002：四入口严格归一化

**需求：** 对每一种可表达组合解析 Clash `proxies`、sing-box `outbounds`、URI 与粘贴文本；未知字段、
未知协议、缺少必需字段和非法组合只产生脱敏诊断，并阻止整批提交。

**验收：** 至少一个节点且零诊断、归一化和整体引用校验均成功时才得到提交候选；其它情况不覆盖旧
Provider 节点。无稳定 URI 的组合标为 `N/A` 并有 Clash/sing-box fixture。

**验证：** 15 协议 × 可表达输入格式的正反 fixture、远程/粘贴等价性、无凭证错误断言、
Base64 深度/空输入/ID 冲突/未知字段拒绝测试。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

**evidence_refs：** `.sdlc/evidence/TASK-007/implementation.yaml#EVIDENCE-TASK-007-001`

### SF-003：Provider 原子替换服务

**需求：** 在 Application 层将解析、归一化、候选整体校验、StateStore 保存与内存交换组合为唯一事务；
该内部服务不暴露 UI/IPC、文件路径或原始配置入口。

**验收：** 解析、归一化、引用校验或保存任一步失败均保持旧 Provider 节点、持久化状态和现有运行配置；
成功后才以稳定 NodeId 更新候选状态。

**验证：** 成功替换、每一步失败保持旧值、并发调用串行化及敏感字段不进入诊断的单元/集成测试。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

**evidence_refs：** `.sdlc/evidence/TASK-007/implementation.yaml#EVIDENCE-TASK-007-001`

## Task 独立验收

15 种非 Tor 用户节点协议可经受限四入口归一化进入 V3 领域状态，且迁移与替换始终 fail-closed；没有
raw JSON 领域存储、部分提交、UI/sidecar/SystemProxy 扩范围或 Tor 运行支持。

**验证：** 目标 Rust 测试、格式化、`cargo test` 的适用分区、`git diff --check` 与独立交付审查；
Tor 相关运行验收为 `BLOCKED`，不记为 PASS。

**acceptance_status：** PASSED

**evidence_refs：**

- `.sdlc/evidence/TASK-007/implementation.yaml#EVIDENCE-TASK-007-001`
- `.sdlc/evidence/TASK-007/code-delivery-review.yaml#EVIDENCE-TASK-007-REVIEW-001`
