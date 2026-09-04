---
id: TASK-006
milestone_ref: M5
dependencies: [TASK-005]
risk: HIGH
status: DONE
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
approval_refs:
  - USER:lifei 2026-09-03 DCR-001 exact 1.14.0 runtime contract
  - USER:lifei 2026-09-03 sha2 0.10.9 offline asset-integrity verification
  - USER:lifei 2026-09-03 getrandom 0.4.3 per-instance API secret generation
  - USER:lifei 2026-09-03 Windows ACL feature expansion and selectable core-version direction
  - USER:lifei 2026-09-03 DCR-001 multi-version and ACL Human Technical Design Gate
  - USER:lifei 2026-09-03 build-stage sidecar asset delivery and Git ignore
  - USER:lifei 2026-09-03 fixed loopback WebSocket stream dependency approval
  - USER:lifei 2026-09-03 explicit managed-observation runtime start/stop authorization
  - USER:lifei 2026-09-03 DCR-001 managed-observation runtime entry Human Technical Design Gate sha256:914b051a1c0c02d7fe6d49c7b7e6ae9e88a8111a7e7ad43e7c55cfc7b0bf054a
  - USER:lifei 2026-09-04 TASK-006 Human Task acceptance
---

# TASK-006：受管 sidecar 与脱敏观测基础

## 本次需求

将已验收的 Mock-only 观测替换为受管、可验证的真实运行时来源：仅使用经 hash 验证并随应用打包的
sing-box sidecar，经 Rust 后端访问固定 loopback Clash API，并将连接、流量和分类日志摘要映射为既有
固定 IPC/事件可消费的安全 DTO。真实 GUI E2E 与 Tray 联合验收移至后续 TASK-010；本 Task 不以 Mock
替代真实受管 child、loopback API 或资产完整性验证。

## 变更控制（2026-09-03）

**来源与批准：** USER:lifei 明确要求将 GUI E2E 后置，优先完成订阅、Pool、语义配置和受管观测基础。

**影响：** 订阅归一化已由 TASK-002 验收，Pool/路由/确定性配置编译已由 TASK-003 验收；当前 Task
保留真实 sidecar、固定 loopback API 与脱敏摘要的基础交付。原 SF-003 和 Task 级 GUI/Tray E2E 验收
移入 TASK-010 future stub。DCR-001 的安全、资源、固定地址和零系统代理契约不变；TASK-005 不受影响。

**失效与后续：** TASK-006 的旧部分 checkpoint 不能单独支持修订后的交付验收；基础代码完成后必须运行
当前目标验证并接受独立交付审查。TASK-007 至 TASK-009 的基础能力及 TASK-010 的 GUI E2E 都必须在
TASK-006 独立验收后才可展开和实现。

## 已知事实与已批准契约

- 当前开发和首个真实 E2E 的唯一可执行基线为 sing-box `1.14.0`；官方 GitHub Release 的 Windows amd64
  archive 为 `sing-box-1.14.0-windows-amd64.zip`，SHA-256 为
  `3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6`。本机为 64 位 Windows；该资产
  已在 Git 忽略缓存中仅作内容核验：archive 哈希匹配，且 `sing-box.exe`、`libcronet.dll`、`LICENSE` 的
  精确成员/哈希/资源名已冻结于 DCR-001；未打包或运行。Windows arm64 不在本 Task 范围。产品最终将
  支持用户选择经验证的 1.12/1.13/1.14，但本 Task 不得把尚未有独立兼容证据的版本标为可用。
- 当前 `SidecarPort`、`RuntimeSupervisor` 和 UI 只具有 Mock/闭合语义；仓库没有实际 sidecar
  adapter、Clash API client 或直接 HTTP client。Tauri 的 transitive `reqwest 0.13.4` 不是可由本项目
  使用的直接依赖。
- 用户已批准上述精确资产、固定 `127.0.0.1:9090` endpoint、每实例 32-byte secret 生命周期、直接
  `reqwest = "=0.12.28"`（仅 `json`、`rustls-tls`）、`sha2 = "=0.10.9"`（关闭 default features）、
  `getrandom = "=0.4.3"`（每实例 32-byte secret 熵源）和无 System Proxy/TUN 的真实 E2E；这些既有
  修订已通过先前 Gate。当前新增的多版本目录与 ACL feature 候选仍须独立审查及新的 Human Gate；在该 Gate
  通过前不得打包资源、改写 manifest/lockfile、启动受管 sidecar 或访问固定 API。
- 用户已确认仅为固定 `ws://127.0.0.1:9090/traffic` 与 `/logs` 摘要读取增加
  `tokio-tungstenite = "=0.30.0"`（关闭默认 feature，仅 `connect`）和
  `futures-util = "=0.3.34"`（关闭默认 feature，仅 `std`、`async-await`、`sink`）。此修订重开 DCR-001
  Technical Design Gate；在新的独立审查与 Human Gate 通过前不得改写 manifest/lockfile 或实现流读取。
- 用户已确认增加零参数 `start_managed_observation_runtime` / `stop_managed_observation_runtime` 产品入口，
  以既有、完整校验的 AppState 驱动受管 sidecar 与固定采样。该授权不包含 System Proxy、TUN、UAC、
  WFP、Service、CaptureMode 变化、配置编辑、任意进程/路径/端口/secret/参数输入或 direct/空配置启动。
  这是 DCR-001 候选修订；在独立设计审查和匹配 Human Gate 完成前不得实现该入口、实际启动 sidecar 或
  访问 API。

## Scope

### allow（全部待下方契约获得确认后）

- `src-tauri/tauri.conf.json`、构建资源配置与受控获取脚本：只按版本目录的已确认 URL、平台、文件名、
  archive 哈希、成员集合与每个解压内容哈希下载 sing-box 资产；构建时验证并回读唯一 bundle resource
  内容清单，
  不接受用户路径或替代 binary。`src-tauri/binaries/` 仅为 Git 忽略的本机/CI 缓存。
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
  仅可新增已批准的零参数启动/停止命令与安全状态；不能新增任意 endpoint、header、secret、路径、命令、PID
  或配置内容入口。
- Rust/TypeScript 测试、hash/包完整性检查，以及受管 child + 固定 loopback API 的进程级验证证据。

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

**验证：** 官方 URL + archive SHA-256、精确 archive 成员/内容/bundle resource 哈希、二进制版本、生成
配置 allowlist/危险键拒绝、私有目录 ACL（含继承与 reparse-point 拒绝）、Mock/真实 sidecar 事务、失败
回滚与受管 child 清理。

**implementation_status：** IMPLEMENTED
**acceptance_status：** ACCEPTED

### SF-002：固定 loopback Clash API 与脱敏观测桥接

**需求：** Rust 后端仅向确认的 loopback API 发起类型化请求，secret 仅留在私有配置/短生命周期内存；
将受控连接、流量、日志摘要转换为 TASK-005 的固定安全 DTO，不开放前端网络访问。

**验收：** 非 loopback/secret/原始日志/连接目标不能穿过任一 DTO、事件、错误或 UI 状态；API 不就绪时
只能报告封闭恢复状态。

**验证：** client 合约/脱敏单测、固定地址拒绝测试、Mock API 集成测试和真实 loopback API 读取。

**implementation_status：** IMPLEMENTED
**acceptance_status：** ACCEPTED

## 实施前提

DCR-001 的固定受管观测运行时入口已完成独立审查与匹配 Human Technical Design Gate。TASK-006 仍以固定
asset/hash、后端专属 loopback、私有 secret、配置默认拒绝、无控制台和零捕获约束为边界。1.12/1.13 不属于
本 Task 的可执行资源或可选 UI。

## Task 独立验收

**验收：** 真实 sidecar 和固定 loopback API 的数据在后端受控、可验证且默认脱敏；固定 DTO 不包含
连接目标、原始日志或 secret，且未引入系统代理、TUN、UAC 或任意能力。GUI/Tray 联合 E2E 由 TASK-010
独立验收。

**验证：** 官方 URL + 固定 SHA-256 与 package readback、Rust/TypeScript 全量验证、真实受管 child 的
固定 loopback API 读取、私有运行时清理与独立交付审查；CI/commit/push 依授权分别记录。

**acceptance_status：** ACCEPTED
