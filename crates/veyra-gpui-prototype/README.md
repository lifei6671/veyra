# P0-03 GPUI 隔离原型

默认仅 synthetic 输入/列表/托盘状态，无 core/service bridge，不启动内核。当前验收为 **ACCEPTANCE**，详见[证据与缺口](../../docs/openbox-rust-gpui-tasks/evidence/p0-03/README.md)。

## 重跑

从仓库根目录执行；Rust 1.99.0 已在本轮安装并验证。原型目录的 toolchain 文件不更改根 workspace/core 的默认 toolchain。

```sh
rustup toolchain install 1.99.0 --profile minimal --component rustfmt --component clippy
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 check --locked -p veyra-gpui-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 build --locked -p veyra-gpui-prototype
sh crates/veyra-gpui-prototype/package-macos.sh
open target/p0-03/VeyraPrototype.app
```

默认 content size 为 1280×720 points。窗口关闭被拦截为应用隐藏，托盘提供显示、状态更新、隐藏、退出。输入为空，验收者需使用真实输入法操作。可从托盘退出；自动化无法访问托盘时，可按精确 executable/PID 清理测试进程，这不算通过菜单退出验收。

## 版本与边界

先核对 [GPUI Kit v0.7.0 manifest](https://github.com/longbridge/gpui-kit/blob/v0.7.0/Cargo.toml)（commit `0c830f4d257e69fdd17200650533ab4ca9a40cc0`），其中 GPUI 来源是 crates.io 的 **gpui-pre =0.3.7**。原型仅依赖 `gpui-kit =0.7.0` 并使用其重导出；`tray-icon =0.24.2` 沿用原 lock 版本。不要另加 `gpui` crate。

macOS 27.0 arm64 实机运行；构建 deployment target 15.0，Mach-O minos 15.0。这不是 macOS 15 实机运行认证，也不是已证明更旧系统不能运行。MiSans 完整桌面 TTF/OTF 尚未锁定，使用 macOS system font/fallback。

## 主事件循环

`gpui_kit::application().run` 是唯一 UI loop。在其主线程初始化 tray/menu/window。`MenuEvent` handler 只向标准 channel 发送事件，GPUI foreground task 每 40ms 读取并更新 UI；不移动 Entity/Context 到 Send 线程。托盘由 Prototype 持有，Prototype Entity 由 foreground task 持有，关闭窗口不 drop。没有 Tokio UI loop、tao/winit loop 或 render/click 中的 block_on。

`uniform_list` 只为可见 range 创建临时行元素，10,000 行由索引生成，没有常驻 10,000 个 Entity。Input/Textarea Entity 在创建窗口时建立，render 只引用，不重建输入状态。

WindowBackgroundAppearance::Blurred 在选定 GPUI macOS backend 中由 NSVisualEffectView 实现；它模糊窗口背后内容。它不自动提供 CSS 的卡片 backdrop-filter。本轮没有添加自有 AppKit 适配层。

## P0-05 GUI owner acceptance probe（默认关闭）

这里只提供验收入口代码，不是生产 GUI/helper 集成；真实 GUI owner lifecycle **NOT_RUN**，不改变 P0-05/DAG 状态或历史证据。只有 macOS、`p0-05-gui-owner` feature 和显式环境变量 `VEYRA_P0_05_GUI_OWNER=1` 同时满足时才启用。默认 feature 不引入 serde_json 或 helper 操作。

从仓库根目录构建和启动真实 GUI executable（不是 CLI owner 子进程）：

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 build --locked -p veyra-gpui-prototype --features p0-05-gui-owner
VEYRA_P0_05_GUI_OWNER=1 ./target/debug/veyra-gpui-prototype
```

该启动命令是后续已授权 Desktop 验收用入口；本次代码验证不运行它，也不连接真实 socket。必须先由独立授权的 Host 准备好既有 P0-05 helper 和隔离测试 Network Service。probe 不安装 helper、不请求管理员权限、不写 /Library、不直接调用 SystemConfiguration 或启动 sing-box；启用后的 Start 请求会让既有 root helper 执行已约定的测试操作。

真实主窗口先创建，GUI 同进程后台线程仅连接固定 `/Library/Application Support/VeyraP005/control.sock`。没有可覆盖 socket/path/service/config/command 的参数，仅三种固定 enum 请求：Hello → StartSystemProxyTest → StartSystemProxyTest → 等待 2 秒 → Status → 等待 2 秒 → Status。五次请求各自 connect/write/read/close，不持有 IPC 连接来模拟 owner 存活。每次写/读共用 15 秒总预算，newline payload 上限 4096 bytes；nonblocking read 不设置 SO_RCVTIMEO。

新实例 Hello 的 instance=null 是既有协议正常结果；若 Hello 已有实例，也必须检查 GUI owner PID/ready/FD3 并与后续相同。从首次 Start 起，每次严格要求 ok=true、instance 非空、owner.pid=GUI PID、child PID/两个端口不变、readiness.outcome=ready、config_transport=anonymous_pipe_fd3、helper_euid=0、recovery_required=false。成功后 stdout 立即 flush 一行 JSON：

```json
{"event":"p0_05_gui_owner_ready","gui_pid":42,"child_pid":84,"mixed_port":12345,"controller_port":23456,"transient_disconnects":5,"same_instance":true,"helper_euid":0,"recovery_required":false,"readiness":{"outcome":"ready","config_input":{"config_transport":"anonymous_pipe_fd3"}}}
```

以上 PID/端口仅为字段示例。marker 使用固定白名单，长度小于 1024 bytes，不输出 secret、config、child stdout/stderr 或原始 readiness。失败仅 stderr 输出固定有界错误 JSON（event=p0_05_gui_owner_failure），窗口继续显示 failure；成功显示 ready 并保持 GUI 进程存活。两种结果均无 Stop/Restore/privileged cleanup，失败后已有实例也必须由 Host 检查现场。

后续真实 Desktop 验收概要（本次未执行）：

1. 在已授权、隔离且可恢复的 P0-05 helper 现场启动上述 executable，确认真实窗口可见；捕获 stdout marker，并核对 gui_pid 是该 executable 的 OS PID，helper LOCAL_PEERPID/instance.owner.pid 与之相同。
2. 确认五次 IPC 断连后同 GUI PID 仍存活，重复 Start/Status 的 child PID/端口一致；marker 只证明请求期间同实例，不能单独证明 GUI 生命周期清理。
3. Host/Codex Desktop 对 marker 中精确 GUI PID 执行 SIGKILL；不要先 Stop/Restore、不要只关闭或隐藏窗口。
4. 独立核对 helper NOTE_EXIT、仍归属自己的代理恢复、child reaped/group empty/listeners closed，以及 cleanup/RecoveryRequired 实际结果；另记真实 evidence，不能将本轮代码/纯测试标为 GUI lifecycle PASS。

无特权验证（全部从仓库根目录运行，不启动 GUI 或真实 helper）：

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 test --locked -p veyra-gpui-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 check --locked -p veyra-gpui-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 build --locked -p veyra-gpui-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 clippy --locked -p veyra-gpui-prototype --all-targets -- -D warnings
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 test --locked -p veyra-gpui-prototype --features p0-05-gui-owner
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 check --locked -p veyra-gpui-prototype --features p0-05-gui-owner
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 build --locked -p veyra-gpui-prototype --features p0-05-gui-owner
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 clippy --locked -p veyra-gpui-prototype --features p0-05-gui-owner --all-targets -- -D warnings
cargo +1.99.0 fmt -p veyra-gpui-prototype -- --check
```

测试只解析内存响应和使用无特权 UnixStream::pair，不调用真实 socket；覆盖 owner/ready/FD3、重复实例、JSON/frame 限制、peer-close/总 deadline、请求白名单与脱敏 marker。
