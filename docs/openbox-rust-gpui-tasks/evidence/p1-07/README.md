# OBG-P1-07 · DONE · owner 已释放

2026-10-05 最终视觉收口：最终源码/构建身份 **8bd7a366d24fa2eff8bf78e22604f461030cedae9d1184c11a28ca8eb4f5c28d**。Light/Dark Shell expanded/collapsed、Panel、Select、Switch、Slider、Modal、Tooltip hover/keyboard、Notice、Input focus 与 disabled 控件已重新采集同一构建；最后Tooltip定位/箭头截图不复用旧版本。逐区域结论见 [visual-matrix.md](visual-matrix.md)，没有虚构全图95%/96.3%数字。

P1-07 最终状态 **DONE，owner 已释放**。最终视觉验收 **PASS_WITH_TECHNICAL_DIFFERENCES**，组合验收 **PASS**。Host 已实际查看最终 React/GPUI 代表截图：Shell、Panel、Select、Tooltip、number spinner、Dark Panel、Modal；没有新的可修视觉 blocker，MiSans/NotoEmoji fallback、element-level Card/Modal backdrop blur、少量 UA 栅格差异继续单列，不声称完全一致。

用户明确执行 `Show #1 → Close → Show #2 → Close → Show #3 → Close → Tray Quit`，见[最终三轮人工证据](final-20261005/human-host-final-three-rounds.json)。随后同一 `8bd7a366` binary、真实 Mach-O app bundle、同一 `/tmp/veyra-p107-final-20261005/root` 重启，PID 94362，snapshot_loaded=true；Settings → Panel Settings 恢复 Light / timeout_ms=6600 / ipv6_test=true，primary=1、writer_created=1、secondary_writer=0，state bytes 前后 hash 不变。用户回复“最终退出完成”，通过 Tray Quit 退出；Host 独立确认 PID94362 gone、socket absent、Python `fcntl.flock(LOCK_EX|LOCK_NB)` free。瞬态 pgrep PID96193 后续 ps 已不存在，不计残留。见[最终重启与退出](final-20261005/human-host-final-restart-quit.json)。

历史 `final-combination-first-launch.log` 仍只有 Show2/Close3，SHA `3233da734c925e5d866aa1b098466c0fe5f240beebcdf4faaf27ff52d48aac26` 原样保留；三轮 PASS 依据独立人工证据，不依赖旧日志。[final-restart-recovery.json](final-20261005/final-restart-recovery.json) 保留原“running for final Human Tray Quit”和 PENDING 字段，仅追加 final_quit/closeout。Proxies/Connections/Logs/Rules 业务页面仍未实现，不计入本卡业务完成，各自任务状态不变；Legacy React/CSS/视觉资源保留。本轮仅更新文档/evidence，不 commit/push、不启动 P2、不访问公网、不启动 sing-box/System Proxy/TUN。

## 已交付实现与组件层级（本轮未修改源码）

保留 `Tokens/Theme → 基础组件 → NavigationItem/CompactSetting/Section → 页面`。

最后一轮只修改 `ui/tokens.rs`、`ui/components/{mod,tooltip,icon_picker}.rs`、`ui/{shell,panel,behavior_panel}.rs`：补CSS8×8旋转45°Sidebar Tooltip箭头与展开/折叠定位；三处number-field复用numeric spinner（React浏览器spinner实际存在）；Light/Dark UA focus与spinner色值分别对应；help图标使用muted、品牌PNG色和Panel Dark局部filter、collapse原SVG tint按实际级联收口。另补图标选择器真实展开态：分类flex移至共享Button外层，搜索focus按局部CSS text色与2px offset覆盖，trigger按space-around排列。输入/数值验证/草稿/保存/CAS语义保留，±1通过原InputState事件更新。没有重写Tray、Theme架构或组件体系。

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

## 自动验证（Codex 原日志与 Host 独立复核分开）

[final-20261005/validation/results.json](final-20261005/validation/results.json)：最后源码修改后完整11项 **全部exit0**，Desktop **45 PASS**、Core **294 PASS**（单线程）。desktop/core check、desktop build、两个clippy `-D warnings`、fmt、resource override workspace `--locked` check、旧Tauri lib clippy、git diff check；全部offline、每命令900s外层超时。`TAURI_CONFIG='{"bundle":{"resources":[]}}'`仅验证workspace/legacy编译，不作为完整打包验收。旧失败/中途构建结果保留。


[Host 最终独立复核](final-20261005/host-final-review.json) 保留首次 `cargo test -p veyra-desktop` 的 **44 PASS / 1 FAIL**：`platform::single_instance::tests::live_socket_without_veyra_lock_is_busy_and_remains_connectable`，断言 `locked_file(&d).unwrap().is_some()`。随后定向连续 **5/5 PASS**，单线程全量 **45/45 PASS**，默认并行全量 **45/45 PASS**。结论仅为“未复现的瞬态测试环境/flock 时序失败，目前无稳定回归证据”，根因未证明；不改写首跑 FAIL，不篡改 Codex 原验证日志。

Host 继续独立执行 Core 单线程 **294/294 PASS**，desktop/core check、desktop build、desktop `--all-targets` clippy、core `--lib` clippy（均 `-D warnings`）、fmt、resource override 下 workspace locked check/旧 Tauri lib clippy、diff check 全部 PASS。resource override 不代表完整打包验收。本轮仅转录用户已确认的 Host 结果，未重跑产品验证或新采 GUI。

## 历史、清理与 DAG

[interaction-results.json](interaction-results.json) 区分旧 wrapper FAIL、此前真实 bundle PASS 与最终 `8bd7a366` 组合 PASS；更新前 latest_build 内容另存历史字段。没有用 secondary Activate、keyboard Quit 或 SIGTERM 替代人工 Tray 行为。

[cleanup.json](cleanup.json) 记录最终 PID94362 gone、socket absent、flock free，无自有 Veyra 验收进程残留；历史有意保留进程的 PENDING 原值可追溯。本轮没有删除 /tmp 或 evidence 内 Host 复核文件。

[完整 68 卡 DAG](final-20261005/dag.json) 按每卡显式依赖重算：**DONE13 / ACCEPTANCE1 / READY0 / DOING0 / TODO47 / Windows DEFERRED7**，无环，Ready Queue 为空。P1-07 DONE 并释放 owner；P2-09、P7-04 虽消费 P1-07，仍缺其他显式依赖，保持 TODO。P0-05 保持 ACCEPTANCE，P0-06/P0-08 保持 TODO；Windows DEFERRED 不变。历史 FAIL/INCOMPLETE/PENDING、锁屏及旧截图/AX/log 保持原始结果。

[本轮文档收口验证](final-20261005/closeout-validation.json) 记录 JSON、六项验收/DONE、DAG/READY、本地链接/anchors、diff check、源码/Cargo/assets/测试字节不变与最终 git status。
