# P0-03 GPUI 隔离原型

仅 synthetic 输入/列表/托盘状态，无 core/service bridge，不启动内核。当前验收为 **ACCEPTANCE**，详见[证据与缺口](../../docs/openbox-rust-gpui-tasks/evidence/p0-03/README.md)。

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
