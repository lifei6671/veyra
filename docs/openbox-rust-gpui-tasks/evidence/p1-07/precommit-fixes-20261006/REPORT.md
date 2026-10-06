# UI / Select 提交前修复报告

2026-10-06；实现者自查，非独立 Review。仅处理 Host 指定的未提交修改。

- Branch：`codex/dist-react-restore`
- HEAD：`3ee99dd758e062641bf7d7ca73a8fb5431c61cdf`
- 无 commit / push / add / reset / restore / clean；没有启动 P2、sing-box、System Proxy 或 TUN。
- Cargo 全部使用 `CARGO_NET_OFFLINE=true`；未访问公网。未重新执行整套 P1-07 视觉验收，不修改任务状态或历史验收记录。
- 本轮增量为下列 7 个 Rust 文件和本报告。保留其他已有 UI、i18n、IconPicker、图标与工作树变更；Cargo.lock、desktop Cargo.toml 与本轮开始时逐字节一致。

| Review Finding | 修复及验证 |
|---|---|
| spinner / URL clear 与 draft 分叉 | 两个主动编辑入口改用 pinned `InputState::replace_all()`。真实 Change 进入原有 InputKey.patch → input_errors / BehaviorPanel.edit → DesktopBehaviorPreferencesPatch → BehaviorCoordinator / CAS / SaveCoordinator 路径。project/rebase 继续静默 set_value。单测及实际输入、保存、重启通过。 |
| Select hover 与 cursor 分裂 | hover 直接更新同一 cursor，删除独立 CSS hover 背景；Up/Down、Enter、click 均使用同一 Selection。实机 keyboard→pointer hover→Enter 确认第三项，仅一个 active 背景。 |
| AX active descendant 缺失 | cursor 对应 option 调用 aria_active_descendant；aria_selected 继续表示已提交值。原生 AX 记录显示 option 0→1→0→1→2，待确认第二/第三项 selected=false。 |
| outside dismiss 焦点错误 | close 统一关闭生命周期。Confirm/Escape 明确 focus trigger；outside 延迟至当前点击处理完毕，仅在无焦点或焦点仍落在 popup 内时恢复 trigger。点击真正可聚焦 Input 时不抢焦点。原生空白→Tab、Input dismiss 和 Escape/Confirm 检查通过。 |
| cached Settings 高度 | 去掉 viewport_size().height，Settings 根使用 size_full/min_h_0，保持内部独立 ScrollHandle；cached 仍按父 page-content 的确定 bounds 布局。真实保存失败横条下滚到底、编辑最后一组、操作 Retry 均通过。 |
| Settings max-width | tokens 增加 SETTINGS_GRID_MAX_WIDTH=1280，SettingsLayout::grid 统一 max_w；768/1024/1280/1536 断点和 gap 保留。单测验证 1600/1920/2560 viewport 的样式约束；实机可用宽度1395时 grid=1280。 |
| all-targets clippy | sidebar_transition_tests 原样移至 shell.rs 末尾；无 allow、无删除测试。desktop all-targets clippy 通过。 |
| Tray 换语言重置 Runtime 展示 | TrayPresentation 缓存最近 project 的展示 key。project 是该 key 唯一更新入口；set_language 只重译，不调用 from_core(None,None) 重新推断。语言值仍由现有 DesktopVisualPreferences 投影，无新增持久化事实或 Runtime truth，start/stop enablement 保留 P1 契约。Ready/Failed/Stopped/Starting/Recovering 参数化测试通过。 |
| Select 空集合 / 越界 | 构造和 set_selected_index 将非法索引规范为 None；empty cursor=0、禁用 trigger、拒绝 open，方向键无 modulo，confirm/get bounds-safe。空、越界、正常 wrap 与稳定原值测试通过。 |

本轮文件：

- `crates/veyra-desktop/src/ui/components/mod.rs`
- `crates/veyra-desktop/src/ui/behavior_panel.rs`
- `crates/veyra-desktop/src/ui/components/select.rs`
- `crates/veyra-desktop/src/ui/pages/mod.rs`
- `crates/veyra-desktop/src/ui/shell.rs`
- `crates/veyra-desktop/src/ui/tokens.rs`
- `crates/veyra-desktop/src/tray.rs`

## Select 最终状态机

| 事实/操作 | 语义 |
|---|---|
| selected | 已提交的原始 options 索引，可为 None；只决定显示值、checkmark、aria_selected。 |
| cursor | 打开时从合法 selected 或0开始；唯一 active option。 |
| hover | 进入 option 时将 cursor 改为该索引；无第二份 hover 状态。 |
| Up/Down | 非空时循环移动 cursor；不修改 selected。 |
| Enter / click | Enter 取 cursor，click 将点击项写入同一 cursor，再 bounds-safe 提交原始 value。tr 仅用于展示。 |
| dismiss | 不提交 cursor；同一 close 入口把 open 置 false，发一次 DismissEvent。重新打开从 selected 开始。 |
| focus | BaseSelect 是 trigger 的唯一 tab stop；内部 Button tab_stop(false)。Confirm/Escape 回 trigger；outside 保留新控件焦点，空白则回 trigger。 |
| AX | cursor 项 aria_active_descendant；selected 项 aria_selected。这两个概念独立。 |

## pinned API 与最小额外修复

本地检查 `gpui-base 0.7.0`，没有假定 set_value 会触发 Change：

- `input/base/state.rs:925` 起明确说明 set_value 不发 Change；内部 emit_events=false。
- `state.rs:956` 起 replace_all 保留 undo，调用 replace_text；实际编辑路径 emit_events=true 时发 InputEvent::Change。主动 spinner/clear 因此真正更新 draft，业务验证和 CAS 保存入口不变。
- BaseSelect 的 open callback 先执行，随后 focus popup；Popover 在自身 open transition 时捕获 previous focus，因此可能记到 popup。outside callback 早于 clicked Input 的最终焦点，不能立即无条件 focus trigger；延迟检查解决这个顺序。

targeted AX 验证额外发现：pinned `gpui-pre 0.3.7/src/view.rs` cached 路径在复用 prepaint/paint 时未重建 AX 子树（492、568行）；真实界面显示控件但后续 AX 树丢失页面控件。shell 仅在 window.is_a11y_active() 时使用同一页面 Entity 正常绘制，普通路径仍 cached(size_full())。不重建页面状态，不增加配置/feature flag。已临时强制 cached 路径验证高度后移除探针。

## 新增测试

新增 **8** 个测试；现有测试保留：

| 数量 | 测试 |
|---|---|
| 3 | spinner_values_match_typed_draft_and_save_patch；clear_url_keeps_empty_draft_and_original_validation；rebase_projects_values_without_new_edit_or_pending_patch |
| 3 | hover_replaces_keyboard_cursor_before_confirm；empty_options_and_invalid_selection_are_normalized；arrows_wrap_and_projection_does_not_commit_cursor |
| 1 | grid_has_react_max_width |
| 1 | cached_runtime_projection_keeps_status_across_languages：Ready/Failed/Stopped/Starting/Recovering |

上述单测保护 typed patch / 草稿 / CAS request、Select 状态及 layout style 契约。原生 Change、AX、focus、真实滚动和重启由下述 GUI 验证；不将纯逻辑单测冒充 GUI。

尝试 test-support 时，本地缺少 proptest，离线依赖解析失败；未下载，已撤销临时 dev-dependency。AX/focus 使用真实 native GUI 和 GPUI AX dump 验证，而非新增 test-support 测试。该尝试日志保留在 /tmp，不计为通过。

## 全部指定验证

每条命令外层 Python subprocess timeout=600s；退出码均为0。workspace/Tauri 的 resources override 仅用于这两条源码检查，不作为包验收。

| 命令 | 结果 |
|---|---|
| cargo test -p veyra-desktop | PASS：63 passed，0 failed / ignored / filtered |
| cargo test -p veyra-core --lib -- --test-threads=1 | PASS：294 passed，0 failed / ignored / filtered |
| cargo check -p veyra-desktop | PASS |
| cargo build -p veyra-desktop | PASS |
| cargo clippy -p veyra-desktop --all-targets -- -D warnings | PASS |
| cargo clippy -p veyra-core --lib -- -D warnings | PASS |
| cargo fmt --all -- --check | PASS |
| TAURI_CONFIG='{"bundle":{"resources":[]}}' cargo check --workspace --locked | PASS |
| TAURI_CONFIG='{"bundle":{"resources":[]}}' cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings | PASS |
| git diff --check | PASS |

机器结果及各命令日志：[validation.json](validation.json)。中间编译失败日志保留，最终表仅记录修复后的实际执行。

## Targeted 真实 GUI

使用本轮隔离 root、独立测试 bundle；最终源已移除所有临时探针。最终 GUI PID89893 用 lsof 确认持有隔离 root/locks/desktop.lock。最终构建无需联网或 Runtime。测试结束已停止该 PID，并移除本轮临时 bundle / root / launcher；保留日志和最终状态快照。

| 操作 | 结果 |
|---|---|
| Select keyboard open / Down / Up / Enter / Escape | PASS；键盘修改 cursor，未确认时 selected 不变。 |
| Down 到第二项，pointer hover 第三项，再 Enter | PASS；third=ipapi.is，单一 active 背景，Confirm 原值映射 draft.ip_info=IpApiIs。pointer 移动通过从菜单空白拖入第三项完成，未触发 option click；popup 保持打开直到 Enter。 |
| Select AX | PASS；active descendant 从 option0→1→0→1→2，第二/第三项待确认时 selected=false。Confirm/Escape/outside 关闭时 GPUI focus 记录为 ComboBox trigger。 |
| keyboard open → outside blank → Tab | PASS；关闭后回 trigger，Tab 正常进入 timeout Input。最终构建再次检查输入框 focus outline。 |
| outside 点击另一个 Input | PASS；可直接输入802，再恢复801，未被 trigger 抢焦点。 |
| spinner +1/-1，保存/重启 | PASS；800→801→800，探针显示 UI值=draft值。再保存801，state.json timeout_ms=801；最终无探针构建重启恢复801。最终构建另实际点击箭头801→800→801，CAS save completion Idle/pending=[]。 |
| test URL clear | PASS；UI和draft同为""。提交 fields=[TestSites]，原有 Validation/TestSiteUrl 拒绝，Failed 且 pending=[TestSites]，磁盘有效URL不变；恢复有效URL后保存为Idle。最终构建重复通过。 |
| 正常 / visual save failed 横条下滚动 | PASS；正常 parent高度672，横条后624；scroll max_offset44→92，滚到底offset=-92。最后一组Telegram输入可编辑，Save可达；Retry失败/恢复后成功。cached 与 AX 正常绘制路径都检查。load failed 未单独注入，用户要求的二选一使用真实 visual save failure。 |
| 超宽 max-width | PASS；实际viewport约1459 logical px、侧栏64、父区域1395，四个settings grid测得1280。未声称执行1600+ GUI；1600/1920/2560由layout contract单测覆盖。 |
| Tray语言与projection | PASS（纯单测）；Ready中文“内核：运行中”→English“Core: running”，其他四个状态同理。未接入真实Runtime。 |

原始 GUI 日志是临时运行产物，未纳入仓库，关键可审计结果已提取到 gui-summary.json / gui-final-state.json。提取的AX/最终值：[gui-summary.json](gui-summary.json)；隔离最终状态：[gui-final-state.json](gui-final-state.json)。截图只在本次 targeted 工具观察中展示，没有新增截图矩阵或声称整体视觉分数。

GUI 中间出现临时 bundle 签名/复制导致的启动失败，以及一次 CUA 重新启动未继承 root override；该进程已停止，没有调用保存，不计入成功证据。后续成功检查均确认本轮隔离 root。最终临时 bundle 只做本机 ad-hoc 签名：raw cargo binary SHA256=3cad019b6192a7067fd5f9df34936dc7be3ec03c1e9fc5707480dccfe0900449；实际运行的签名后副本SHA256=865aa16202cd1548894636e56ac10612427f5561212c819e5950682c99de5330。没有改仓库图标或发布配置。

IconPicker uniform_list、原catalog索引、IME提交才筛选、clear_search / 打开重置均未改动。Sidebar 256/64、240ms ease、reverse、reduce motion 保留，原两项测试通过。翻译及语言持久化契约不变。本轮自查未发现剩余阻塞问题；实际验证限于上述操作，不扩写为完整视觉/发行包/Runtime验收。

## Git staging 前后

前后 `git diff --cached --name-status` 完全一致：

```text
A	src-tauri/icons/256x256.png
A	src-tauri/icons/512x512.png
A	src-tauri/icons/64x64.png
```

整个 index 文件前后SHA256也相同：`05800678aec4690b8a3956323346aa56aea2f85edffa68753b019dda6f7b11af`。Branch / HEAD 未变。

## 最终 git status --short

```text
 M AGENTS.md
 M Cargo.toml
 M crates/veyra-desktop/Cargo.toml
 M crates/veyra-desktop/src/app.rs
 M crates/veyra-desktop/src/main.rs
 M crates/veyra-desktop/src/platform/macos.rs
 M crates/veyra-desktop/src/tray.rs
 M crates/veyra-desktop/src/ui/behavior_panel.rs
 M crates/veyra-desktop/src/ui/components/icon_picker.rs
 M crates/veyra-desktop/src/ui/components/mod.rs
 M crates/veyra-desktop/src/ui/components/notice.rs
 M crates/veyra-desktop/src/ui/components/select.rs
 M crates/veyra-desktop/src/ui/components/tooltip.rs
 M crates/veyra-desktop/src/ui/mod.rs
 M crates/veyra-desktop/src/ui/pages/mod.rs
 M crates/veyra-desktop/src/ui/panel.rs
 M crates/veyra-desktop/src/ui/shell.rs
 M crates/veyra-desktop/src/ui/tokens.rs
 M src-tauri/icons/128x128.png
 M src-tauri/icons/128x128@2x.png
A  src-tauri/icons/256x256.png
 M src-tauri/icons/32x32.png
A  src-tauri/icons/512x512.png
A  src-tauri/icons/64x64.png
 M src-tauri/icons/Square107x107Logo.png
 M src-tauri/icons/Square142x142Logo.png
 M src-tauri/icons/Square150x150Logo.png
 M src-tauri/icons/Square284x284Logo.png
 M src-tauri/icons/Square30x30Logo.png
 M src-tauri/icons/Square310x310Logo.png
 M src-tauri/icons/Square44x44Logo.png
 M src-tauri/icons/Square71x71Logo.png
 M src-tauri/icons/Square89x89Logo.png
 M src-tauri/icons/StoreLogo.png
 M src-tauri/icons/icon.icns
 M src-tauri/icons/icon.ico
 M src-tauri/icons/icon.png
?? crates/veyra-desktop/src/application_icon.rs
?? crates/veyra-desktop/src/ui/i18n.rs
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/app-icon-picker-reset-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/dark-controls-ime-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/dock-tray-icon-20261006/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/feedback-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/global-tooltip-language-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/interaction-performance-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/minimum-height-20261006/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/minimum-width-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/precommit-fixes-20261006/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/responsive-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/sidebar-animation-20261005/
?? docs/openbox-rust-gpui-tasks/evidence/p1-07/tooltip-dismiss-20261005/
```
