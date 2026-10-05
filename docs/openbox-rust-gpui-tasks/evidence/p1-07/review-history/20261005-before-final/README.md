# OBG-P1-07 · DOING · owner GPUI

Host Review 三项 blocker 已分别完成复核：错误 Light Shell evidence 已替换；默认背景 CPU 派生移到 blocking task；相同真实 Mach-O bundle 下 baseline/current 托盘人工可见、三轮恢复及显式退出通过。**整卡仍 DOING，不声称 95% 或完整视觉 PASS**。历史 INCOMPLETE/FAIL 均保存在 `review-history/` 及 JSON `history`，不改写为重跑通过。

## 修改及层级

`Tokens/Theme → 基础组件 → NavigationItem/CompactSetting/Section → 页面` 保留。

- `ui/tokens.rs` / `theme.rs`：React/CSS 普通与 Panel 局部主题、固定几何、字体和状态。
- `ui/components/{mod,select,icon_picker,notice,tooltip}.rs`：共享 Button/Navigation 的 Kit 稳定焦点 handle 外层装饰；IconButton、Input、Switch、Slider、Modal、Tooltip、generic Notice。修复 Slider 缺失 Track/Indicator 绑定、输入默认厚阴影、按钮焦点被裁切、Tooltip 锚点。
- `ui/{shell,panel,behavior_panel,pages/mod}.rs`：Shell、9分类导航、通用/延迟/布局/测试站点组合；既有 Entity/保存/CAS 不回退。
- `ui/background.rs`、`services.rs`、`state_bridge.rs`、`app.rs`：同源默认 JPEG 异步派生；单任务合并最新 key、generation 校验、防 stale 覆盖。Render 只读 ready cache，无 decode/resize/blur/PNG encode；未新增持久化事实。
- `ui/icons.rs` + `assets/heroicons/`：本地 @heroicons 2.2.0 原 SVG/path，MIT notice 保留。
- `main.rs` 明确1280×720 content 与 debug 日志重定向；`tray.rs` 仅 debug 原生诊断，未重写创建/持有/MenuEvent。

工程 header/Refresh、schema/config/selection、底部 GPUI preview、Entity/A-B/Busy/Synthetic、UI changes/Save submissions、视觉/行为二级切换仅在 debug + `VEYRA_UI_EVIDENCE` 展示，不进入生产截图。修改文件全表见 [git-status.txt](git-status.txt)，CSS/复用见 [component-audit.md](component-audit.md)。

## 三项 Host blocker

1. **Shell theme evidence**：全新隔离 root，显式保存 Light→Core snapshot/apply_theme→Overview expanded/collapsed，再 Dark 重复；有效偏好/AppView.dark/Kit 同步，content1280×720。原 Light 为 System 解析 Dark 的错误采集；未盲调颜色。原图及错误 manifest 保留历史。[状态](comparison/shell-theme-state.json)、[逐项视觉矩阵](visual-matrix.md)。只比较 Shell 自有区域，Overview 业务体排除。
2. **默认背景 CPU**：三条新增回归保护 stale size/blur 结果拒绝、同 key 不重复派生、render 只读/服务派生职责边界。实际拖 Blur33→9 随即打开 Select，无明显 UI 阻塞；blocking job 耗时与线程日志保留。cache 不是第二配置源。
3. **Tray A/B**：[tray-baseline-ab.json](tray-baseline-ab.json)。独立 b982e8d worktree/probe-only patch，不破坏 dirty tree。shell wrapper bundle 两者皆不可见/34×0 logical，用户确认非全屏、无自动隐藏、菜单栏有空间。换同一实际 Mach-O bundle 后 baseline“可见，已 Tray Quit”、current“可见，菜单正常”；延迟采样 button/window 正常，68×66 physical。结论为启动包装环境差异；isVisible 和早期68×0都不能替代真人可见。没有 tray 架构修复。

最后 Tooltip 定位修复后的截图暂为 PENDING：Mac 锁屏，已请求手动解锁；修复前实测与截图保留，没有伪造新版本视觉通过。

## 来源、截图与差异

React 当前 HEAD 的 `src/openbox/**` + CSS 是唯一视觉来源；[source-identity.json](source-identity.json) 核对 P0-01 source 后相关视觉源码未变化，仍重新采集本地 reference。API/WS fixture 拦截且外部 URL 禁止；本轮 Host 修订不访问公网。此前用户另行授权的只读线上编辑入口核对属于历史，未修改线上值。

`react/` 包含原16张基本状态和两主题10种交互 reference；`gpui/` 与 `gpui/interactions/` 保存对应原图。清单、SHA、状态/尺寸/排除范围见 [comparison/manifest.json](comparison/manifest.json)。GPUI 原图2560×1504，裁64 physical标题栏→1280×720。**先将嵌入 macOS Display ICC 转 sRGB，再裁剪/测色/overlay**；此前直接比较 raw JPEG RGB 的绿颜色差异是采集颜色空间问题。叠图没有整图 PASS 分数。

字体 [font-differences.md](font-differences.md)：CoreText 解码 WOFF2 与 GPUI 分片组合限制、许可未明确分别说明；MiSans/NotoEmoji 保留系统 fallback。Panel 卡片/Modal 的局部 backdrop-filter 当前元素 API 未等价迁移，不用 window material 冒称等价。仍有 Tooltip 排版/字宽、sidebar tooltip arrow、数字输入 UA spinner 等未完成细节，见矩阵；这些实现缺口不包装成平台不可实现。

UI `config/speedtest-url` 与 Backend Profile `testUrl/directTestUrl` 为不同语义；此前全 React 搜索及 Host 只读核对未找到 UI 前者编辑控件，按 Host“没有则不用编辑”保留模型/持久化，未发明生产入口。四项 Proxies 偏好仅 evidence 编辑，未扩展业务页。

## 自动验证

[validation/results-final.json](validation/results-final.json) 完整11项全部 exit0，Desktop **45 PASS** / Core **294 PASS**（单线程）。包括 desktop/core check、desktop build、两包 clippy `-D warnings`、fmt、resource override workspace locked check、旧 Tauri lib clippy、diff check。Tooltip 最后定位修复的受影响补验另存 `results-tooltip-final.json`。均 offline，每命令900s外层限制；override `TAURI_CONFIG='{"bundle":{"resources":[]}}'` 不作为完整打包证明。初次 clippy FAIL 及此前验证保留。没有启动内核/System Proxy/TUN。

## 真实组合及清理

[interaction-results.json](interaction-results.json) / `validation/review-native-combination.log`：新隔离 root 首次启动 writer1；视觉 Light、opacity90/blur10/radius16，行为6500ms/IPv6on；保存 accepted revision12；真实 Close 隐藏；用户完成三轮 Close→Tray Show，日志始终同 WindowId(1v1)/Settings；用户 Tray Quit；进程消失/socket由应用移除/flock可再取。重新打开恢复 Light、6500、IPv6on；用户再次 Tray Quit，清理再次通过。重开默认 Overview 是现有会话模型；隐藏恢复保留 Settings。没有使用 secondary Activate/SIGTERM 冒充这段通过。

组合二进制 `bf6e6b3...`；后续共享焦点/Tooltip 纯展示修复独立验证及截图，最终源码/二进制身份见 [build-identity.json](build-identity.json)，没有冒称所有图片均来自同一二进制。

组件实际状态已补 mouse/keyboard Select、Switch on/off/focus、Button/IconButton hover/focus/disabled、Input focus、Tooltip hover/focus/Escape/blur、Modal Escape/focus return、真实 generic Notice close/auto dismiss。timeout0 Save 为已有 Core validation failure；abc 为本地 invalid draft/Save disabled，6500恢复保存成功。未构造网络/Runtime错误。

[cleanup.json](cleanup.json) 分开记录两次真人 Tray Quit 与之后 diagnostic keyboard Quit。最后桌面/本地 React 服务无残留，locks 全部可取；native acceptance root socket由应用清除。另一个早期 wrapper诊断 root stale socket经无进程/锁可取后手动清理，**不计作 Tray cleanup**。隔离配置、日志及 baseline worktree 保留 Host 审查；未动用户配置。

## DAG

`P1-01/02/03/04A/04B/05/06 DONE → P1-07 DOING (GPUI)`；其他任务状态不变。不启动P2/业务占位页；Legacy React参考保留；无 commit/push。
