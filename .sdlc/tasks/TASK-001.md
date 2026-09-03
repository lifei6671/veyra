---
id: TASK-001
milestone_ref: M1
dependencies: []
risk: MEDIUM
status: DONE
design_refs:
  - .sdlc/design/foundation.md
approval_refs:
  - .sdlc/state.yaml#gates.technical_design
  - .sdlc/state.yaml#gates.delivery
---

# TASK-001：桌面工程骨架与默认拒绝能力基线

## 本次需求

为 Windows V0.1 建立可构建、可测试的 Tauri 2 / React / TypeScript / Rust 工程骨架，并建立最小
Tauri capability 基线。依赖只能使用已冻结 Foundation 中列出的家族和范围，前端仅能调用本任务明确
开放的类型化命令。

本任务不实现订阅、状态持久化、sing-box sidecar、System Proxy、TUN 或任何 macOS 平台能力。

## Scope

### allow

- `package.json`、`pnpm-lock.yaml`、TypeScript/Vite/测试配置与 `src/` 下的 React 工程骨架。
- `src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/src/`、`src-tauri/capabilities/` 与 Tauri 配置。
- 一个仅返回固定应用启动信息的类型化 Tauri command，以及调用它的最小前端入口。
- 针对上述骨架、命令和 capability 的最小单元/类型检查用例与开发脚本。

### deny

- SQLite、SQLx 或其他配置数据库；任意文件、Shell、进程、任意 HTTP 或 sidecar capability。
- 订阅解析、`AppState` 持久化、配置编译、sing-box binary、Clash API、System Proxy、TUN、UAC、
  Windows 原生依赖、签名、Updater、发布和 macOS 实现。
- 未列于 Foundation 的第三方依赖、锁文件手工编辑或隐式 capability 放宽。

## 子功能

### SF-001：Tauri/React/Rust 工程骨架

**需求：** 使用 `pnpm` 建立 React 19、TypeScript、Vite 与 Tauri 2 的桌面工程；Rust 侧形成
`application`、`commands`、`domain`、`storage`、`subscription`、`singbox`、`platform` 目录边界，但不
实现后续业务能力。

**验收：** `package.json`、`src-tauri/Cargo.toml` 与锁文件由各自包管理器生成；实际版本符合
Foundation 批准的范围；不存在数据库依赖或 sidecar binary。

**验证：** `pnpm lint`、`pnpm test`、`pnpm build`、
`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、
`cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`、
`cargo test --manifest-path src-tauri/Cargo.toml`。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

### SF-002：默认拒绝的 IPC capability 基线

**需求：** 仅开放一个返回固定启动信息的类型化 command；前端经该 command 验证 IPC 通路。Capability
不授予文件、Shell、进程、任意网络或 sidecar 访问权。

**验收：** capability 定义与 Rust command/前端调用一一对应；不存在 wildcard、任意路径、任意参数或
未使用权限。应用启动信息不包含凭据、路径、系统代理或运行时 secret。

**验证：** 针对 command 返回类型和 capability 清单的单元/结构检查；`pnpm test` 与
`cargo test --manifest-path src-tauri/Cargo.toml` 通过。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

### SF-003：可重复的本地质量入口

**需求：** 在 `package.json` 和 Rust 工程中提供与 Foundation 一致的 lint、test、build、fmt、clippy
入口，供后续 Task 复用。

**验收：** 所有命令可从仓库根目录按 Task 记录执行；不存在要求手动跳过检查的脚本或未解释的失败。

**验证：** 执行 SF-001 所列全部命令，并保存各命令的真实退出码和摘要 Evidence。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## Task 独立验收

**验收：** 在干净依赖安装后，Windows V0.1 工程能够以默认拒绝 capability 构建并运行最小 IPC 通路；
所有本任务质量命令通过，且交付清单不包含数据库、sidecar、网络代理、TUN、原生 Windows API 或
macOS 代码。

**验证：**

1. 运行 SF-001 中列出的全部命令并记录真实结果；
2. 检查 capability 文件、Cargo/npm 依赖与交付 diff，确认均在 Scope.allow 内；
3. 运行最小桌面启动/IPC smoke，并验证前端只取得固定启动信息。

**acceptance_status：** PASSED
