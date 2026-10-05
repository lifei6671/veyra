# 最新状态：DOING / GPUI · 最后构建待实际采集

本轮末补查图标选择器展开态，发现分类tab均分宽度和搜索框局部focus覆盖缺口，已做最小源码修正，并重新通过完整11项自动验证（45 Desktop / 294 Core）。新构建 `8bd7a366d24fa2eff8bf78e22604f461030cedae9d1184c11a28ca8eb4f5c28d`。当前运行bundle仍是627728a2，等待用户完成已请求的Tray Quit后再换入新构建。**以下627728a2截图/组合只保留为本轮中途证据，不能算最后源码视觉PASS。新build尚未截图/组合，不建议DONE。**

# OBG-P1-07 · DOING · owner GPUI

2026-10-05 最终视觉收口：最终源码/构建身份 **627728a2abb3b8ce3804c7cea69ff12e9c5c10733abd4b9823858d1c3aa511d6**。Light/Dark Shell expanded/collapsed、Panel、Select、Switch、Slider、Modal、Tooltip hover/keyboard、Notice、Input focus 与 disabled 控件已重新采集同一构建；最后Tooltip定位/箭头截图不复用旧版本。逐区域结论见 [visual-matrix.md](visual-matrix.md)，没有虚构全图95%/96.3%数字。

P1-07 按 Host 要求保持 **DOING、owner GPUI**。最终组合正在等待真人三轮 Tray Show 与两次 Tray Quit；尚未到达时不计 PASS，实时结果见 [interaction-results.json](interaction-results.json)。本轮不 commit/push、不启动P2、不访问公网、不启动sing-box/System Proxy/TUN、不扩展占位业务页。

## 最终改动与组件层级

保留 `Tokens/Theme → 基础组件 → NavigationItem/CompactSetting/Section → 页面`。

最后一轮只修改 `ui/tokens.rs`、`ui/components/{mod,tooltip}.rs`、`ui/{shell,panel,behavior_panel}.rs`：补CSS8×8旋转45°Sidebar Tooltip箭头与展开/折叠定位；三处number-field复用numeric spinner（React浏览器spinner实际存在）；Light/Dark UA focus与spinner色值分别对应；help图标使用muted、品牌PNG色和Panel Dark局部filter、collapse原SVG tint按实际级联收口。输入/数值验证/草稿/保存/CAS语义保留，±1通过原InputState事件更新。没有重写Tray、Theme架构或组件体系。

完整P1-07变更文件见 [git-status.txt](git-status.txt)。[component-audit.md](component-audit.md) 包含复用消费位置、CSS数值映射及本轮补充。此前工程header/Refresh、schema/config/selection、GPUI preview、Entity/A-B/Busy/Synthetic、UI changes/Save submissions、视觉/行为二级标签仍只在debug + `VEYRA_UI_EVIDENCE`显示；production截图无这些工程布局，测试能力保留。

## 三项 Host blocker 的既有处理

1. Shell旧Light截图采错System解析Dark。新隔离root显式Light/Dark、snapshot load/apply_theme后采图；当前final截图再验证1280×720与Kit同步。没有盲调主题色。普通Dark Shell与Panel Dark局部CSS覆盖独立保留，Overview业务体排除。
2. 默认背景decode/resize/blur/PNG encode已移到blocking服务；Render只读ready cache，App按key/generation接收结果再notify，单in-flight合并最新key，不建立第二配置事实。三条回归保护stale结果拒绝、同key不重复派生、Render职责边界。当前日志显示CPU在ThreadId(5)，输入可响应；此前真实Blur拖动结果保留。
3. [tray-baseline-ab.json](tray-baseline-ab.json)：独立b982e8d基线/current同环境A/B。shell wrapper两者都不可见；同一实际Mach-O bundle两者真人可见、菜单正常，status window从34×0到34×33 logical。不是P1-07 Tray架构回归；未重写tray-icon。68×0初始rect/isVisible不替代真人判断。此前三轮恢复/显式退出/重启结果保留，最终build组合另记录。

## 当前视觉证据与差异

[final-20261005/react-source-identity.json](final-20261005/react-source-identity.json) / [source-identity.json](source-identity.json) 固定当前React/CSS与本地资源来源，P0-01仅辅助身份核对。React本地Vite+P0 API/WS mock，所有非127.0.0.1请求被阻止，审计和采集脚本保留；无公网访问。之前独立授权线上只读检查属于历史，本轮没有执行。

- [截图清单与SHA](comparison/manifest.json)：所有当前GPUI图来自final build；[图名索引](final-20261005/screenshots.md)列每个状态。canonical `react/`、`gpui/`基本图同步最后版本。
- `final-20261005/comparison/`保存sRGB归一化原图与50% overlay。原GPUI2560×1504裁64physical原生titlebar→1280×720；先ICC→sRGB，避免错误raw JPEG色值。鼠标halo、原生title/menu/OpenPanel、未迁移业务主体不参与视觉判定。
- [最终逐区域矩阵](visual-matrix.md)独立列固定几何、颜色、同源图标、字体、实际状态、PASS/TECHNICAL_DIFFERENCE。可迁移的Tooltip arrow/spinner/focus细节已补；不把技术差异计为一致。
- [font-differences.md](font-differences.md)：WOFF2可被CoreText解码，但GPUI load_family按PS名去重/字形要求不能等价组合MiSans unicode-range分片；NotoEmoji亦未等价。系统fallback仍造成笔画、字宽、Tooltip中心/换行差异；最终图没有明显布局溢出、分类截断、控件错位。未下载/重新分发许可不明确字体，不称完全一致。
- React Card/Modal局部backdrop-filter采样各元素后方内容；GPUI现有element style API未等价，当前同源JPEG预blur+surface alpha。Modal后方UI文字仍透出；普通Panel视觉接近，仍记录技术差异。没有用window-level blur冒充element backdrop-filter，也不改窗口架构。

UI config/speedtest-url与Backend Profile testUrl/directTestUrl语义不同。按此前Host“没有则不用编辑”保留UI前者模型/持久化；四项Proxies偏好编辑留evidence-only，不扩展本轮业务。

## 自动验证

[final-20261005/validation/results.json](final-20261005/validation/results.json)：最后源码修改后完整11项 **全部exit0**，Desktop **45 PASS**、Core **294 PASS**（单线程）。desktop/core check、desktop build、两个clippy `-D warnings`、fmt、resource override workspace `--locked` check、旧Tauri lib clippy、git diff check；全部offline、每命令900s外层超时。`TAURI_CONFIG='{"bundle":{"resources":[]}}'`仅验证workspace/legacy编译，不作为完整打包验收。旧失败/中途构建结果保留。

## 历史、清理与DAG

[interaction-results.json](interaction-results.json)区分旧wrapper FAIL、此前真实bundle PASS、最后build新组合。最后一次源码修改后构建身份/值恢复/同窗口恢复/Tray Quit与socket/flock检查均单独记录；不以secondary Activate或SIGTERM替代Tray行为。

[cleanup.json](cleanup.json)区分旧人工/诊断清理与本轮最终清理。`.DS_Store`本轮生成未跟踪文件已移除，不纳入交付；不动无关用户文件。隔离root/bundle、日志、基线source-only worktree保留Host检查，最终不留自有writer/process/socket。

`P1-01/02/03/04A/04B/05/06 DONE → P1-07 DOING (GPUI)`。其他Task状态不变，不开启下游任务。历史FAIL/INCOMPLETE/PENDING及锁屏记录保存在 `review-history/20261005-before-final/` 与 `final-20261005/history/`，没有覆盖成通过。
