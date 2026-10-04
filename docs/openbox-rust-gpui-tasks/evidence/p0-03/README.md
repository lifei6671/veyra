# P0-03 实机原型证据

当前状态：**DONE（2026-10-04）**，四项卡验收 PASS；来源为下文 [Host/User Manual Acceptance](#host-user-manual-acceptance)。本轮仅收口证据和状态，未启动 P0-04/P1-03。

原型交付日期：2026-10-03。owner：Codex Desktop。起始 HEAD `029578c81554f2aeef9bb4e0fd6aaa8a19defd55`，起始工作树干净。历史交付为 ACCEPTANCE，不是 GPUI 路线 BLOCKED。实现者当时按 code-delivery-review 自查；Host 后续独立 Review 记录见补证段。

## 四项任务卡验收

| 条件 | 结果 | 证据边界 |
| --- | --- | --- |
| 中文 IME、编辑、Escape/焦点 | PASS | 用户亲手完成真实中文 Input/Modal Textarea 输入并确认可用，补足 composition/candidate 真实性；结合历史编辑/Escape/焦点恢复操作。自动化未取得候选框的 LIMITATION 保留 |
| 最后窗口关闭后的托盘完整生命周期 | PASS | 用户亲手完成托盘点击并确认可用，结合历史 Host 显示/隐藏、菜单更新、退出和进程检查，明确批准本项通过；独立 tray screenshot/frontmost focus 缺证据保留 |
| light/dark、字体/Emoji、透明/blur | PASS | 11 张真实截图 + window blur 切换/backend/能力差异说明足以回答可实现程度与差异；受控 blur 缺证据保留，不声称 card backdrop 已证明 |
| 版本兼容与实机构建 | PASS | 沿用锁定版本/依赖/构建结果；Host 独立 resource override workspace check 及本次两条 check PASS；原无覆盖 FAIL 保留，不等于 macOS 15 实机或打包资源验收 |

[逐项操作](interaction-results.json)包含 PASS/FAIL/LIMITATION 和具体动作；[视觉 manifest](visual-manifest.json)记录截图身份、尺寸、编码来源、SHA256 与缺失证据；[版本](versions.json)包含 rustc -Vv、cargo -V、uname、sw_vers；[源码身份](source-provenance.json)绑定最终文件 SHA256。

## 可复现组合

先读取 [v0.7.0 Cargo.toml](https://github.com/longbridge/gpui-kit/blob/v0.7.0/Cargo.toml) 和 [Kit crate manifest](https://github.com/longbridge/gpui-kit/blob/v0.7.0/crates/kit/Cargo.toml)，再按 [input example](https://github.com/longbridge/gpui-kit/blob/v0.7.0/examples/input/src/main.rs) 与 [dialog example](https://github.com/longbridge/gpui-kit/blob/v0.7.0/examples/dialog_overlay/src/main.rs)编写原型。tag 指向 `0c830f4d257e69fdd17200650533ab4ca9a40cc0`。Kit 重导出 GPUI 类型，不另选最新版 gpui。

实际依赖是 crates.io 精确版本；Cargo.lock 保存 checksum。`cargo tree` 见 [prototype](tree.txt)、[core](core-tree.txt)、[duplicates](duplicates.txt)。原型没有 tao/winit，只有一套 gpui-pre 类型来源；旧 Tauri 仍留在根 workspace，但 core 依赖树不含 GPUI/Tauri。对比起始 lock，core tree 的全部版本身份均已存在，未升级 core 第三方依赖。新增 Kit 的锁文件解析会让非 core 的 toml/proc-macro-crate 等传递依赖重新解析；没有修改旧 Tauri manifest。

[原型与启动命令](../../../../crates/veyra-gpui-prototype/README.md)。根 default-members 仍为 core，原型不依赖 core。原型目录单独 pin Rust，不改变根默认 toolchain。

## 实际操作与视觉

窗口 content 初始请求 1280×720 points，原生截图包含标题栏，物理尺寸 2560×1506；比例 2x（由请求宽度与实际截图宽度对应确认）。resize 后为 2204×1386。browser=N/A。

CUA 原生截图返回 JPEG，初次以 `.png` 文件名保存后用文件签名检查发现；已用 sips 只做 PNG 转码，无拼图、重绘或后期深色处理。manifest 保留原始 JPEG SHA256。截图为应用真实渲染，标题栏中紫色图标是捕获环境指示，不是原型托盘图标。

- `light-main.png` / `dark-main.png`：真实切换主题，中文、英文、数字、彩色 Emoji 与旗帜。
- `light-modal.png` / `dark-modal.png`：Kit 长文本弹层；默认焦点是弹层容器，需点击 textarea 后编辑。Escape 返回先前输入框，由随后 Space 写入底层 Input 验证。不是“自动聚焦 textarea”或“返回触发按钮”的证据。
- `light-list.png` / `dark-list.png`：10,000 synthetic rows，中段/滚动与末尾；5002 选中项跨滚动保留。
- `list-resize.png`：拖动窗口角落后继续滚动，仍响应且选中项不变。无 benchmark 数字。
- `gradient-light.png` / `gradient-dark.png`：最终仅替换主区域背景为 GPUI linear_gradient 后重建/重启；真实浅深色 + RGBA card。此前 9 图保留旧背景身份；托盘逻辑未改。额外进程按精确 PID 清理，不算菜单退出验收。
- `transparency-blur.png` / `transparency-only.png`：实际切换 window appearance；没有已知纹理位于窗口后方，不能只凭这两张图判定 blur 质量。

`uniform_list` 按 visible range 生成行，不保存 10,000 个完整控件树。字体用 `.SystemUIFont` 和系统 fallback；MiSans 完整桌面 TTF/OTF 尚未锁定。已观察常见/旗帜 Emoji 为彩色、中英混排无明显缺字；未做精确 baseline/line-height 测量。参考 OpenBox dark 本身存在浅 sidebar/card 与低对比文字，不把原型主题能力验证当成一比一视觉完成。

2026-10-03 IME 调查（历史）：初始输入源 ABC/微信输入法；通过 CUA 发送切换快捷键与逐个字母，没有观察到候选窗口。临时添加系统简体拼音后仍未获得候选链路。已恢复原输入源与 Globe 设置。IME 已向 Host 请求真实操作但尚无对应确认；不能判定 GPUI 的中文输入法不支持。

2026-10-03 托盘调查（历史）：tray 在 GPUI 主线程构造，无 panic；CUA AX 只返回应用窗口和空应用菜单，没有状态栏项，SystemUIServer 访问超时。点击 close 后精确 executable 仍存活，但工具观察会返回窗口，隐藏状态没有独立确认。Host 随后对“绿色图标、至少三轮显示/隐藏、更新状态后查看菜单、最后退出”的请求回复“完成”。这些项目记作 Host 报告 PASS，随后精确 executable 查询无进程；不需要 SIGTERM。没有单独确认关闭最后窗口后的前台 focus，`tray-window-hidden.png`、`tray-menu.png` 仍未取得。未使用第二 loop 或换 AppKit 冒充 tray-icon 通过。

## Blur 与 P1-04A 建议

选定 GPUI 的 `gpui-pre-macos-0.3.7/src/window.rs` 中，`set_background_appearance`（约 1857 行）为 Blurred 插入 NSVisualEffectView，并在透明模式移除。故窗口背后模糊已有 backend 原生适配，本原型无需自建 AppKit 层。它不能证明窗口内某 card 下的 GPUI 内容得到 CSS backdrop-filter 模糊。

可复用：圆角、GPUI linear_gradient 背景、RGBA card、浅深色、系统字体与彩色 Emoji。仍未知：窗口背后纹理的 blur 对照、卡片级实时 backdrop blur。P1-04A 先做固定背景图片的模糊版本 + RGBA card 视觉对照；这只近似静态背景，不冒充任意内容的实时 blur。若必须实时窗口背后效果，可复用 WindowBackgroundAppearance::Blurred；若确需 card 级 AppKit 材料，先做单个 NSVisualEffectView 的尺寸/层级/生命周期小适配验证，不建立通用 AppKit 框架。本轮没有实现 P1-04A。

## 主循环自查

唯一入口 `gpui_kit::application().run`。主线程创建 tray/menu。MenuEvent 经 std channel 回 GPUI foreground task，再调用 window handle / Entity update；Entity/Context 不进入 Send 线程。持有关系：foreground task → Prototype Entity → TrayIcon/MenuItem。没有 render/click block_on，没有 Tokio/tao/winit 第二 UI loop。每 40ms 消费队列仅用于此原型。静态审查 PASS 不等于托盘真实菜单验收。

## 构建与限制

| 命令（仓库根） | 结果 |
| --- | --- |
| `MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 check --locked -p veyra-gpui-prototype` | PASS；[日志](check-final.txt) |
| `MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 build --locked -p veyra-gpui-prototype` | PASS；[日志](build-final.txt) |
| `MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 clippy --locked -p veyra-gpui-prototype -- -D warnings` | PASS；[日志](clippy-final.txt) |
| `cargo +1.99.0 fmt --all -- --check` | PASS；[日志](fmt-final.txt) |
| `cargo check --locked`（default core） | PASS；[日志](core-check.txt) |
| `cargo check --workspace --locked` | FAIL；[原始脱路径日志](workspace-check.txt)；旧 src-tauri/build.rs 引用缺失 `binaries/sing-box-1.14.0-windows-amd64/LICENSE`，未改此配置/补造资源 |
| `sh crates/veyra-gpui-prototype/package-macos.sh` | PASS；[日志](package.txt)，仅生成本地 ad-hoc .app |
| `xcrun vtool -show-build target/debug/veyra-gpui-prototype` | minos 15.0 / SDK 27.0 |

依赖 `block 0.1.6` 有 Rust future-incompatibility 提示：`src/lib.rs:64 static _NSConcreteStackBlock: Class` 为 uninhabited static。当前 clippy -D warnings 返回 0，没有抑制或 patch 此依赖；未来 toolchain 升级需重新检查。未跑无关大规模测试。

2026-10-03 历史交付到 ACCEPTANCE，曾等待 Host 补 IME/托盘实测；当时未 commit/push、未访问 OpenBox 远端、未启动内核。当前结构/links/DAG/hash 验证及历史验证快照见 [validation.json](validation.json)，验收以 2026-10-04 追加记录为准。


## 2026-10-04 最终验收补证（历史进行中记录）

Host 已独立 Review 当前代码、静态检查、11 张历史截图及 provenance；确认 core 与旧 veyra 的生产依赖树均无版本漂移。本轮不重做原型。

追加执行 `TAURI_CONFIG='{"bundle":{"resources":[]}}' MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 check --workspace --locked` 返回 0，见 [resource override 独立日志](workspace-check-resource-override.txt)。产品源码/tauri.conf 未修改；该 override 仅用于已有旧入口编译检查，不代表打包资源完整。上文原 workspace check FAIL 保留为历史事实。

最终 DONE 门槛按本轮明确范围：真人 IME、关闭最后窗口后的托盘显示/聚焦/状态更新/退出，以及受控 window blur 的真实能力结论。MiSans、nested dropdown、card 级实时 backdrop blur 和 macOS 15 实机不阻断本 Task。


<a id="host-user-manual-acceptance"></a>
## 2026-10-04 Host/User Manual Acceptance

来源：本轮用户明确指令及亲手操作确认，完整记录见 [host-manual-acceptance.json](host-manual-acceptance.json)。用户亲自完成真实中文输入并确认功能可用，确认单行 Input 与 Modal Textarea 的截图中均有真实中文内容；用户亲自完成托盘点击并确认可用，并明确要求“就当验收通过了”。据此四项卡验收判定 PASS，P0-03 从 ACCEPTANCE 更新为 DONE。

中文 composition/candidate 的真实性由用户真实输入确认补足；托盘生命周期由本次用户确认、历史 Host 操作报告及用户明确批准补足。该人工确认不等于补造 candidate-window / tray-menu 截图或独立 frontmost focus 观测。历史 41 项操作、LIMITATION、截图来源/manifest 与原无覆盖 workspace check FAIL 全部保留；用户提及的中文截图不虚构本地文件名，现有 PNG 仍为 11 张。

视觉项以现有真实截图、window blur/backend 和限制说明回答可实现程度与差异。受控 window blur 纹理对照未取得，card-level backdrop blur 未证明；MiSans、nested dropdown 与 macOS 15 实机仍为后续限制，按本次明确验收范围不阻止本卡 DONE。

本次使用项目既有 Python subprocess.run 外层超时复跑：

| 命令（仓库根） | 结果 / 新日志 |
| --- | --- |
| `cargo +1.99.0 check --locked -p veyra-gpui-prototype` | PASS，exit 0，timeout 600 秒；[日志](check-2026-10-04.txt) |
| `TAURI_CONFIG='{"bundle":{"resources":[]}}' MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 check --workspace --locked` | PASS，exit 0，timeout 900 秒；[日志](workspace-check-resource-override-2026-10-04.txt) |

resource override 仅用于编译检查，不改产品配置，不证明打包资源完整；无覆盖时旧 Windows LICENSE resource FAIL 继续保留。两条 check 仍有历史 `block 0.1.6` future-incompatibility 提示，未抑制或修改依赖。

68 张任务卡实际重算：macOS DONE 5、READY 2、TODO 54、ACCEPTANCE 0；Windows DEFERRED 7。P0-04、P1-03 READY；P0-05 因 P0-04 未 DONE 仍 TODO。本轮未启动下游，未改原型功能代码、未访问 OpenBox 远端、未运行真实 sing-box、未 commit/push。


### 2026-10-04 Host pre-commit review：后续人工运行清理

来源为用户转述的 Host 复核报告：Host 发现 PID `58786` 仍运行 `target/p0-03/VeyraPrototype.app/Contents/MacOS/veyra-gpui-prototype`。这是历史 Host tray Quit 无进程检查之后再次人工验收运行留下的进程；Host 推测来自本次人工输入/托盘验收后的再次运行。Host 已仅对该精确 PID 发送 TERM，并验证该 executable 无任何残留，结果 **PASS**。

此前“Host tray Quit 后无进程”的记录原样保留；两次检查对应不同运行，本次发现不推翻旧结果，TERM 清理也不计作 tray Quit 验收。P0-03 保持 DONE；本次仅追加证据，未改功能代码、Cargo 或任务状态/DAG，未 commit/push。
