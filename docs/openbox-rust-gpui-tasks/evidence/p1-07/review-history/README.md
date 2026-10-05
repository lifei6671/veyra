# OBG-P1-07 · DOING · owner GPUI

从干净 `b982e8d6a8dfa69ee6e5f861e33a3d9366854d7f` 开始。已实施 Shell + Settings + Panel 与共享组件收敛，**没有完成 P1-07 验收，不能设 DONE**。Host 本轮真实反馈“没看到托盘”；native status item 标记 visible=true，但 rect 持续68×0 physical，根因未确定。托盘恢复和显式退出未通过；完整交互视觉矩阵也未齐全。

## 修改与层级

- `ui/tokens.rs` / `ui/theme.rs`：React/CSS 固定几何、普通主题与 Panel 覆盖。
- `ui/components/{mod,select,icon_picker,notice}.rs`：Button/IconButton/Input/Select/Switch/Slider/Modal/Tooltip/Notice/Empty；Kit 负责焦点、输入、Popover、基础行为，外观覆写。
- 跨页组合：NavigationItem、setting_section、section_heading、CompactSetting、settings_pair。
- `ui/shell.rs`、`ui/pages/mod.rs`、`ui/panel.rs`、`ui/behavior_panel.rs`：页面组合；9分类、通用/延迟/布局/测试站点双列；受管背景与行为偏好仍走原 coordinator / CAS / writer。
- `ui/icons.rs` + `assets/heroicons/`：本地 @heroicons 2.2.0 outline 原 SVG/path 与 MIT notice；SidebarToggle 来自 React 原几何。没有近似 glyph 替代主导航。
- `app.rs`：复用持有 behavior Entity、NoticeCenter、默认背景缓存及生产重试入口；`main.rs` 明确1280×720内容区；`tray.rs` 仅加 debug 原生矩形/可见性诊断。
- 工程 header + Refresh、schema/config/selection 条、底部偏好/GPUI preview、Entity/A-B/Busy/Synthetic、UI changes/Save submissions、视觉/行为切换只在 debug 且 `VEYRA_UI_EVIDENCE` 时出现；原证据/测试能力保留。

完整变化清单见 [git-status.txt](git-status.txt)，组件复用和 CSS 映射见 [component-audit.md](component-audit.md)。仍需收口 Tooltip/focus/Toast 状态及零散一次性样式的最终审查，不能称组件体系全部验收通过。

## 来源与截图

[source-identity.json](source-identity.json) 保存当前 React source identity；与 P0-01 source `7fab0e4...` 比较 React 视觉源码无变化，但本轮仍重新运行当前 HEAD 本地页面。`capture-react.cjs` 使用 P0-01 fixture，拦截本地 API/WS，外部 URL 拦截；修正 PATCH `{entries,removed}` 包装后重新采图。

React 16张：Light/Dark 各 shell、collapsed、panel、select、switch-toast、modal、icon-picker、slider。GPUI 对应图位于 `gpui/`，另有真实保存、单实例恢复、重启值恢复截图。截图清单与裁剪映射见 [comparison/manifest.json](comparison/manifest.json)，逐区域结论见 [visual-matrix.md](visual-matrix.md)。

[字体探针](font-differences.md) 区分 WOFF2 可解码与 GPUI 分片组合限制；不重新分发许可未明确的 MiSans。背景 fixture 是仓库 `src/openbox/assets/panel-background.jpg` 按1280×2275缩小的本地图，原5333高超出既有4096限制，没有放宽校验。

用户后续明确允许只读访问 openbox.disign.me 查编辑入口；仅检查 Panel/Backend，恢复原分类，没有修改远端值/保存/启停。UI `config/speedtest-url` 与 Profile `testUrl/directTestUrl` 分属不同配置，未找到前者编辑入口。按 Host“没有则不用编辑”保留模型/持久化，未发明生产入口。Proxies 四个偏好仍在 evidence 编辑范围，不启动代理业务。

## 自动验证

命令均本地 `CARGO_NET_OFFLINE=true`，Python subprocess 外层900s上限；resource override 是 `TAURI_CONFIG='{"bundle":{"resources":[]}}'`，不修改仓库配置。

| 验证 | 结果 |
|---|---|
| cargo test -p veyra-desktop | PASS 42 |
| cargo test -p veyra-core --lib -- --test-threads=1 | PASS 294 |
| cargo check -p veyra-desktop / -p veyra-core | PASS |
| cargo build -p veyra-desktop | PASS |
| desktop/core cargo clippy -- -D warnings | PASS；初次3条样式警告失败历史保留，修复后重跑 |
| cargo fmt --all -- --check | PASS |
| resource override cargo check --workspace --locked | PASS（scoped override，不是完整打包证明） |
| 旧 Tauri lib clippy -D warnings（同 override） | PASS |
| git diff --check | PASS |

`validation/results.json` 为初次完整执行，`results-current.json` 为 UI 收敛后的影响范围重跑，`results-tray.json` 为最终 debug 诊断新增后的 Desktop/fmt 回归。历史失败没有改写。[构建身份](build-identity.json)。新增 catalog 测试保护实际站点图标资源能解码；没有删除/弱化原 CAS/保存测试。未运行 sing-box/系统代理/TUN。

## 实际操作与清理

[interaction-results.json](interaction-results.json)、`validation/combination-first-run.log` / `combination-complete-log.txt`：

1. 空隔离目录首次启动，单 primary/writer1。
2. 真实修改主题为亮色、IPv6 测试开启，保存成功并核对磁盘。
3. 原生关闭 → hidden=true，进程仍在。
4. Host 无法看到托盘：该步 FAIL/阻塞，未冒充恢复成功。
5. 第二次启动 secondary exit0/writer0，原 Settings 页面和值保留；这不是托盘恢复。
6. 诊断 SIGTERM 后重新启动，亮色/IPv6开启恢复；这不是“显式退出→重开”通过。重开默认 Overview，页面会话保留仅在隐藏/恢复期间。
7. native 文件面板导入、Select、Switch、Modal取消、Slider 展示和保存通知有本轮实际截图；不冒称完整 hover/error/busy/焦点验收。

[cleanup.json](cleanup.json)：停止本轮桌面与127.0.0.1:1427 React服务，手动移除自有 stale socket；这种收尾不能证明应用 Quit cleanup。隔离配置/资产暂保留作托盘诊断输入，用户浏览器原标签未关闭，未动真实用户配置。

## DAG 与下一步

`P1-01, P1-02, P1-03, P1-04A, P1-04B, P1-05, P1-06 (既有 DONE) → P1-07 (DOING, GPUI)`。

本轮只把 P1-07 READY→DOING，其他 Task 状态不变。没有启动任何 P2 或 Proxies/Connections/Logs/Rules 业务能力，没有删除 Legacy 视觉源码，没有 commit/push。

剩余：定位真实菜单栏零高托盘；补齐 hover/focus/loading/error/Toast/Tooltip 视觉与交互；处理视觉矩阵未完成项后，重新完整执行规定的组合操作。当前自动验证成功不能覆盖这些缺口。
