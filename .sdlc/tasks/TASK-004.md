---
id: TASK-004
milestone_ref: M4
dependencies: [TASK-003]
risk: HIGH
status: DONE
design_refs:
  - .sdlc/design/foundation.md
approval_refs:
  - .sdlc/state.yaml#gates.technical_design
  - USER:lifei 2026-09-03 TASK-004 Tokio, thiserror, tracing, and Windows WinINet bindings
  - USER:lifei 2026-09-03 TASK-004 Win32_Foundation GlobalFree for WinINet strings
---

# TASK-004：受管 Sidecar、配置事务与 Windows System Proxy

## 本次需求

实现仅由 Rust 后端使用的受管 sing-box 运行时：从有效 `AppState` 构建候选配置，按
`Build -> check -> Prepare -> Apply -> Ready` 事务启动或替换受管 sidecar，并在失败时保持或恢复
可证明的前一稳定状态。实现 Windows 当前用户默认 WinINet 连接的 System Proxy Adapter，以
Snapshot/Managed/Observed 三态、通知、回读和条件恢复保护用户手动改写。Runtime Supervisor 只支持
`Off` 与 `SystemProxy`；TUN 仍需专项 ADR 与人工批准。

## 启动前确认

本 Task 的下列依赖已获用户确认，并已写入 manifest 与 lockfile：

- `tokio = { version = "=1.53.1", features = ["process", "sync", "time"] }`：受管子进程、串行化与 Ready 超时。
- `thiserror = "=2.0.20"`：不泄露凭据或路径的运行时/平台错误类型。
- `tracing = "=0.1.44"`：后端结构化运行时事件，不向前端事件或错误暴露 secret。
- `[target.'cfg(windows)'.dependencies] windows = { version = "=0.61.3", features = ["Win32_Networking_WinInet"] }`：当前用户默认 WinINet 连接的读写、`InternetSetOption` 通知与回读。

完整 WinINet 字符串查询由 API 分配内存，并要求调用方使用 `GlobalFree` 释放。用户已确认将
Windows feature 精确扩展为：

- `Win32_Foundation`：仅用于调用 `GlobalFree` 回收 WinINet 返回的 ProxyServer、Bypass 与 PAC URL 字符串。

只允许上述依赖及 Cargo 正常生成的 lockfile 变更；不引入 `winreg`、`windows-service`、WFP、
全系统 TCP/UDP 表扫描、Shell 命令封装或任意命令执行。

## Scope

### allow

- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`：仅在上述依赖确认后写入精确版本、功能集及正常解析结果。
- `src-tauri/src/application/{mod,runtime}.rs`：封闭 Runtime Supervisor、运行状态与对 Sidecar/SystemProxy Port 的编排；不把运行态写入 `AppState`。
- `src-tauri/src/singbox/{mod,runtime}.rs`：固定打包 sidecar 的身份/路径解析、候选/active/previous 配置事务、`check`/`run` 固定参数、受管 child 身份、Ready/停止/回滚 Port。
- `src-tauri/src/platform/{mod.rs,windows/mod.rs,windows/system_proxy.rs,windows/recovery.rs}`：Windows 当前用户默认 WinINet 的三态代理模型、私有恢复记录及条件恢复；Application 只依赖封闭 Port。
- 上述模块的 Rust 单元测试、`src-tauri/tests/fixtures/runtime/` 下不含真实凭据和可执行文件的最小 fixture。

### deny

- `src-tauri/src/commands.rs`、`src-tauri/capabilities/`、前端、Tauri 配置、任意 UI IPC、网络下载、外部二进制、sidecar 资产/Hash 清单、任意文件路径或命令参数输入。
- 真实用户 System Proxy 写入、真实 sidecar 启动、`sing-box check`、自动下载或打包 `sing-box`；这些仅在已验证资产和明确人工运行授权后作为独立 Evidence 执行。
- TUN、UAC、提权 Helper、Windows Service、WFP、Wintun、路由/DNS 接管、PAC/WPAD 以外的命名连接写入、全系统连接/PID 扫描、Clash API 或连接观测。
- 运行态、恢复记录或生成配置写入 `AppState`/`state.json`；无条件覆盖用户改写的 System Proxy；未确认依赖。

## 子功能

### SF-001：受管 Sidecar 配置与生命周期事务

**需求：** 定义封闭的 Sidecar Port 与 Runtime 状态机，只以应用控制的工作目录、固定 executable identity
与固定 `check`/`run` 参数操作 sidecar。候选配置必须先通过 check，成功后才推进 active/previous；新实例
未达到进程与 Ready 判据时停止候选并恢复前一已验证配置。停止只作用于当前 child 或恢复记录可证明的实例。

**验收：** Mock Sidecar Port 证明候选失败不替换 active，启动失败回滚 previous，停止不接受任意 PID/命令，
且错误、日志与运行快照不包含节点凭据或配置原文。

**验证：** Rust 单元测试覆盖 Build/check/Apply/Ready 顺序、check 失败、Ready 超时、重启回滚、受管实例
身份与停止边界；真实 sidecar check/run 为 `NOT_RUN`，直到存在经 hash 验证的打包资产并获得独立运行授权。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

### SF-002：Windows System Proxy 三态 Adapter 与恢复

**需求：** 以 `ProxySnapshot`、`ManagedProxyState`、`ObservedProxyState` 表达当前用户默认 WinINet
连接的完整代理语义。Adapter 在单一锁内执行 capture、写入 transitioning 恢复记录、应用 loopback Managed
状态、`InternetSetOption` 通知、回读比较与 stable 提交；恢复前再次回读，只有 Observed 与 Managed
语义相等才恢复 Snapshot。

**验收：** Mock Windows Port 证明 PAC/WPAD 初始启用时仍能形成完整 Snapshot，写入或通知失败不覆盖用户
状态，用户手动改写时保留 Observed 并报告冲突，恢复记录不会把未验证候选标为 stable。

**验证：** Rust 单元测试覆盖 capture/enable/notify/readback/restore、PAC/WPAD、写入中途失败、通知失败、
回读不匹配、用户冲突、transitioning 崩溃恢复与不泄露原始代理内容；真实 WinINet 改写为 `NOT_RUN`，不在
开发机自动执行。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

### SF-003：Off 与 SystemProxy 的串行可补偿切换

**需求：** Runtime Supervisor 只实现 `Off -> SystemProxy -> Off`。进入 SystemProxy 时先确认 sidecar Ready，
再应用并验证 Managed 状态；退出时先按语义相等规则恢复 Snapshot，再停止 sidecar。任一失败只回到可证明的
前一稳定状态，不能同时启用两种捕获模式，也不得暗中触发 UAC 或 TUN。

**验收：** 使用可控 Mock Ports 证明调用顺序、失败补偿、单一串行切换、用户冲突时收敛为安全状态；Runtime
恢复记录与配置状态分离。

**验证：** Rust 单元测试覆盖 Off/SystemProxy 双向成功路径、sidecar Ready 失败、代理应用/回读失败、停止
失败和并发切换请求；任务级检查运行 `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、
`cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`、`cargo test --manifest-path src-tauri/Cargo.toml`、
`pnpm lint`、`pnpm test`、`pnpm build` 与 `git diff --check`。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## Task 独立验收

**验收：** 在不暴露任意进程、路径、命令或 Windows 原生参数给 UI 的前提下，应用可使用 Mock Ports 验证受管
sidecar 的配置替换/回滚和 Windows System Proxy 的三态条件恢复；任一失败或用户改写都不会替换最后稳定的
运行状态或强制覆盖用户代理。TUN、真实系统代理写入、真实 sidecar 运行、二进制提供与发布不属于本 Task
验收。

**验证：**

1. 在确认依赖后运行 Task 声明的 Rust、前端与差异检查；
2. 独立交付审查核对固定 sidecar 操作边界、三态恢复语义、错误脱敏、依赖/Scope 与 mock 覆盖；
3. 将真实 sidecar/WinINet 操作、CI、提交与推送分别标记为 `NOT_RUN`，除非后续获得对应授权和证据。

**acceptance_status：** PASSED
