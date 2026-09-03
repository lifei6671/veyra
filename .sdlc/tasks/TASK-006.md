---
id: TASK-006
milestone_ref: M5
dependencies: [TASK-005]
risk: HIGH
status: READY
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
approval_refs:
  - USER:lifei 2026-09-03 DCR-001 exact 1.14.0 runtime contract
  - USER:lifei 2026-09-03 sha2 0.10.9 offline asset-integrity verification
  - USER:lifei 2026-09-03 getrandom 0.4.3 per-instance API secret generation
  - USER:lifei 2026-09-03 Windows ACL feature expansion and selectable core-version direction
  - USER:lifei 2026-09-03 DCR-001 multi-version and ACL Human Technical Design Gate
---

# TASK-006：真实 sidecar 观测与最终端到端验证

## 本次需求

将已验收的 Mock-only 观测替换为受管、可验证的真实运行时来源：仅使用经 hash 验证并随应用打包的
sing-box sidecar，经 Rust 后端访问固定 loopback Clash API，并将连接、流量和分类日志摘要通过既有
固定 IPC/事件交给主窗口。完成最终 E2E 时必须覆盖真实 sidecar、固定 loopback API、脱敏 UI 和
Tray 隐藏/恢复的联合路径；不得以 Mock 或纯单元测试替代。

## 已知事实与已批准契约

- 当前开发和首个真实 E2E 的唯一可执行基线为 sing-box `1.14.0`；官方 GitHub Release 的 Windows amd64
  archive 为 `sing-box-1.14.0-windows-amd64.zip`，SHA-256 为
  `3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6`。本机为 64 位 Windows；该资产
  已下载、解压并完成 archive/executable SHA-256 readback；Windows arm64 不在本 Task 范围。产品最终将
  支持用户选择经验证的 1.12/1.13/1.14，但本 Task 不得把尚未有独立兼容证据的版本标为可用。
- 当前 `SidecarPort`、`RuntimeSupervisor` 和 UI 只具有 Mock/闭合语义；仓库没有实际 sidecar
  adapter、Clash API client 或直接 HTTP client。Tauri 的 transitive `reqwest 0.13.4` 不是可由本项目
  使用的直接依赖。
- 用户已批准上述精确资产、固定 `127.0.0.1:9090` endpoint、每实例 32-byte secret 生命周期、直接
  `reqwest = "=0.12.28"`（仅 `json`、`rustls-tls`）、`sha2 = "=0.10.9"`（关闭 default features）、
  `getrandom = "=0.4.3"`（每实例 32-byte secret 熵源）和无 System Proxy/TUN 的真实 E2E；这些既有
  修订已通过先前 Gate。当前新增的多版本目录与 ACL feature 候选仍须独立审查及新的 Human Gate；在该 Gate
  通过前不得打包资源、改写 manifest/lockfile、启动受管 sidecar 或访问固定 API。

## Scope

### allow（全部待下方契约获得确认后）

- `src-tauri/binaries/`、`src-tauri/tauri.conf.json`、构建资源配置：只加入已确认版本、平台、文件名和
  SHA-256 的 sing-box 资产；构建时验证 archive 与 extracted executable 身份，不接受用户路径或替代
  binary。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：仅加入已确认的 Rust HTTP client 及其精确 feature，及
  `sha2 = "=0.10.9"` 的离线 asset-integrity 实现、`getrandom = "=0.4.3"` 的每实例 secret 熵源；不引入
  通用 Shell、文件、进程或前端网络能力。可扩展现有 `windows = "=0.61.3"` 的
  `Win32_Security_Authorization`、`Win32_System_Threading` 与 `Win32_System_Memory` feature，仅用于
  当前用户私有 ACL 的写入与回读。
- `src-tauri/src/singbox/`、`src-tauri/src/application/`、`src-tauri/src/platform/`：实现固定参数的已验证
  sidecar adapter、私有配置/secret 生命周期、固定 loopback Clash API client、Ready/停止/失败回滚与脱敏
  runtime observation adapter。
- `src-tauri/src/{commands.rs,lib.rs,build.rs}`、`src-tauri/capabilities/default.json` 与
  `src/{App.tsx,lib/observability.ts,lib/observability.test.ts,styles.css}`：仅复用既有固定观测 DTO/事件，
  不能新增任意 endpoint、header、secret、路径、命令、PID 或配置内容入口。
- Rust/TypeScript 测试、hash/包完整性检查、真实 sidecar + 固定 loopback API + 脱敏 UI + Tray 恢复的
  Windows E2E 证据。

### deny

- 未在 DCR-001 版本目录中精确批准的下载源、版本、hash、架构、第三方依赖、网络端点、监听地址/端口、secret
  生命周期、system proxy/TUN/UAC/WFP/Service/Wintun 操作。
- 前端直接访问 loopback API、任意 HTTP/文件/进程权限、任意 shell 参数、全系统 PID 扫描、连接目标或
  原始 Core Log/secret/credential 进入 IPC、事件、日志、错误或持久化状态。
- 将连接历史、流量或日志写入 `AppState`、`state.json`、数据库、浏览器持久化存储或外部 telemetry。

## 子功能

### SF-001：受信 sidecar 资产与私有运行时 adapter

**需求：** 下载并锁定唯一批准的 archive/hash/架构，将其作为应用受控 sidecar；固定 `check`/`run`/
`stop` 语义，验证 archive、executable、版本、配置与受管 child 身份。生成配置只允许一个
`127.0.0.1:9090` Clash API listener，拒绝 API service、Dashboard、远程控制、TUN、bridge、TLS spoof、
USB/IP、额外 listener 与原始 JSON 透传。失败不启动或停止候选，且不泄露路径、配置或 secret。

**验收：** 资产校验失败、版本不匹配、check/run/ready 失败都不能启用 API 或系统代理；仅可停止本应用
受管 child。

**验证：** 官方 URL + SHA-256、archive 成员、二进制版本、生成配置 allowlist/危险键拒绝、私有目录
ACL、Mock/真实 sidecar 事务、失败回滚与受管 child 清理。

**implementation_status：** DRAFT
**acceptance_status：** PENDING

### SF-002：固定 loopback Clash API 与脱敏观测桥接

**需求：** Rust 后端仅向确认的 loopback API 发起类型化请求，secret 仅留在私有配置/短生命周期内存；
将受控连接、流量、日志摘要转换为 TASK-005 的固定安全 DTO，不开放前端网络访问。

**验收：** 非 loopback/secret/原始日志/连接目标不能穿过任一 DTO、事件、错误或 UI 状态；API 不就绪时
只能报告封闭恢复状态。

**验证：** client 合约/脱敏单测、固定地址拒绝测试、Mock API 集成测试和真实 loopback API 读取。

**implementation_status：** DRAFT
**acceptance_status：** PENDING

### SF-003：真实联合 E2E 与托盘恢复

**需求：** 在已验证 asset 和 API 上，证明真实运行时的观测/脱敏 UI、关闭隐藏、Tray restore 共同工作；
隐藏期间不积压高频事件，恢复不重启 sidecar 或重建应用状态。

**验收：** Windows E2E 显示同一窗口恢复、同一受管 sidecar 保持运行、API 观测重启后继续有效，且无控制台
窗口、无系统代理/TUN 改写。

**验证：** 本机 Windows UI Automation 与真实受管 sidecar/固定 loopback API E2E；Mock 不可替代该项。

**implementation_status：** DRAFT
**acceptance_status：** PENDING

## 实施前提

DCR-001 的 1.14.0、sha2、getrandom、多版本目录与 ACL feature 修订均已通过独立复审及 Human Technical
Design Gate。TASK-006 可在已确认 Scope 内恢复实施；仍必须以固定 asset/hash、后端专属 loopback、私有
secret、配置默认拒绝及无控制台约束为边界。1.12/1.13 不属于本 Task 的可执行资源或可选 UI。

## Task 独立验收

**验收：** 真实 sidecar 和固定 loopback API 的数据在后端受控、可验证且默认脱敏；前端只经固定 DTO
读取；Tray 恢复不破坏同一运行时；最终 E2E 不遗漏真实链路，且未引入系统代理、TUN、UAC 或任意能力。

**验证：** 官方 URL + 固定 SHA-256 与 package readback、Rust/TypeScript 全量验证、真实 API/sidecar/
Tray Windows E2E、独立交付审查；CI/commit/push 依授权分别记录。

**acceptance_status：** PENDING
