# P4-03 最终视觉对照（已批准）

**2026-10-10 最终视觉人工审批：PASS。** Host 逐项查看 React/GPUI 同数据原图，54/54 张原图 SHA256 核对一致；列表/添加/普通组/主备高级/焦点键盘/数值步进/保存加载与失败，以及实际 Runtime 选择反馈在用户当前 macOS 显示设置、浅色、1280×720 范围内接受。最后 UI 18 个定向测试（Groups 6、Components 12）、Desktop Clippy/build/fmt/diff 全部 PASS。该结论不声明测出了数值化的全产品 95% 像素相似度；P4-02 已批准白色不透明弹窗、CoreText 字体与原生标题栏差异保留，深色/其他缩放交后续组合卡。原独立只读 Review 零剩余 Finding，旧 Legacy Windows libcronet.dll Clippy FAIL 保留；锁定 sing-box1.14.0 正式功能 1 Native PASS 与 P4-03 Runtime 107 定向 PASS 的独立原证据不变。P4-03 由 ACCEPTANCE 验收为 **DONE**，本卡 owner/预约释放；P2-05 仍 ACCEPTANCE、P2-06 仍 DOING/原 Runtime owner 不变。最终提交、合并及主分支复验按本次批准流程实施，不 push。

日期：2026-10-09。状态：**ACCEPTANCE / AWAIT_HUMAN**。工作树 `dev/p4-03-runtime`，输入检查点 `8d3b47a84fc4987df2e8396684126a5efa318811`。本轮只有 UI 修正与视觉验收；正式 Runtime/Native 功能沿用既有交付，没有修改选择事务、Helper/IPC、P2-05 或用户网络。P2-06 DOING、P2-05 ACCEPTANCE 与 owner/预约保持。**尚未提交、合并、主分支复验或释放 owner；不 push。**

## 对照方法与证据身份

唯一视觉事实为仓库 `src/openbox/pages/settings/GroupSettings.tsx`、`src/openbox/openbox.css` 及真实 OpenBox 页面。在线 `http://192.168.1.6:3036/#/settings` 只读打开列表/添加并取消，不保存、不操作其网络。在线部署数据与源码 fixture 不同，00 原图仅核对在线结构，不用于冒充同数据对照。

同数据对照使用真实 React AppShell/SettingsPage/GroupSettings、CSS、MiSans-VF 与相同背景资源；临时 loopback harness 只替代 API 为五组、三个合成节点，不改产品 React。GPUI 是实际桌面 binary/服务与独立 root `/tmp/veyra-p403-final-8d3b47a`。两者逻辑内容视口均 **1280×720**、浅色；保持用户当前 macOS 显示设置（屏幕逻辑1512×982、backing2），没有切换显示缩放。React 原图1280×720，GPUI原图2560×1504包含64px物理原生标题栏。`compare-*` 仅剔除原生标题栏并还原逻辑尺寸拼接，左React右GPUI，原图全部保留，逐图hash见 [截图清单](evidence/p4-03-visual-final/screenshots.json)；没有补绘、更改截图内容或计算任意相似分数。

[完整原图与日志目录](evidence/p4-03-visual-final/)，[binary/root身份](evidence/p4-03-visual-final/identity.json)。正常页面与 Runtime 状态在本轮连续修正过程中拍摄；最后焦点/数值步进修正不影响 Runtime 浮层，18–25 对应的实际 binary 单独记为 `runtime_visual_binary_sha256`，不声称所有照片同一 binary。

## 实际修正

- 显式 CSS focus 覆盖现在优先于共享 Input 的绿色边框；名称保持绿色，单位输入、搜索、页签名称按局部 CSS 保持正常边框。
- 分组规则 Escape 回焦后绘制绿色边框，焦点只登记一次；通过局部 `border_focus()` 启用，成员筛选/页签模式及其他页面默认不扩大鼠标 focus 样式。真实 Escape、Tab 离开、Shift+Tab 返回分别留图。
- 数值输入复用既有 numeric 控件，在 hover/focus 显示步进按钮；只读/忙碌时不允许通过步进绕过草稿保护。
- 保存按钮保持“保存”文案，14px loading 图标在左、间距7px；保存忙碌时控件保留 React 颜色，真实禁用条件和防重复提交仍有效。Cancel、关闭、成员操作及页签操作沿用已有业务保护，不通过本地状态伪造成功。

## 五项流程结论

| 流程 | 当前工程结果 | 证据 | 人工结果 |
| --- | --- | --- | --- |
| 列表卡片 | PASS：62px卡片、间距、两行摘要、图标、按钮、hover同状态核对 | 01、13 | 待确认 |
| 添加分组 | PASS：静态默认空态、表单、保存disabled；名称/搜索focus逐态核对 | 02–04 | 待确认 |
| 普通组修改 | PASS：动态/静态、勾选、转移、HK搜索和US/HK顺序；规则下拉键盘状态 | 05–08、14、26–30 | 待确认 |
| 主备组修改 | PASS：主用/备用、高级设置、超时focus、恢复开关关闭；输入/成员操作 | 09–12 | 待确认 |
| 实际反馈 | PASS：保存loading/失败/重试、真实pin/切换中/pending/核对/Auto/全部失败；22仅核对过渡 | 15–25、JSON与Native日志 | 待确认 |

这里的 PASS 是当前限定视口与状态的工程核验；**不等于用户最终批准，不宣称全产品95%或全主题验收。** 页面首次加载属于此前验收记录；本轮 loading 专指真实保存与选择忙碌，未把页面首次加载重新记为本轮新 PASS。

## 同数据原图索引

React/GPUI 分别为同前缀 `*-react-*`、`*-gpui-*`。以下链接为并排预览，完整原图可在证据目录按同编号读取。

- [01 列表](evidence/p4-03-visual-final/compare-01-list.png)
- [02 添加空态/保存禁用](evidence/p4-03-visual-final/compare-02-add-empty.png)
- [03 名称焦点](evidence/p4-03-visual-final/compare-03-name-focus.png)
- [04 成员搜索焦点](evidence/p4-03-visual-final/compare-04-search-focus.png)
- [05 普通动态组](evidence/p4-03-visual-final/compare-05-edit-dynamic.png)
- [06 静态成员勾选](evidence/p4-03-visual-final/compare-06-static-selected.png)
- [07 转移后](evidence/p4-03-visual-final/compare-07-static-transferred.png)
- [08 HK搜索](evidence/p4-03-visual-final/compare-08-search-selected.png)
- [09 主用](evidence/p4-03-visual-final/compare-09-primary.png)
- [10 备用/高级设置](evidence/p4-03-visual-final/compare-10-backup-advanced.png)
- [11 超时焦点](evidence/p4-03-visual-final/compare-11-timeout-focus.png)
- [12 恢复主用关闭](evidence/p4-03-visual-final/compare-12-switch-off.png)
- [13 卡片hover](evidence/p4-03-visual-final/compare-13-card-hover.png)
- [14 成员顺序](evidence/p4-03-visual-final/compare-14-member-order.png)
- [15 保存忙碌](evidence/p4-03-visual-final/compare-15-save-loading.png)
- [16 保存失败](evidence/p4-03-visual-final/compare-16-save-failed.png)
- [26 规则菜单](evidence/p4-03-visual-final/compare-26-rule-open.png)
- [27 键盘游标](evidence/p4-03-visual-final/compare-27-rule-keyboard.png)
- [28 Escape回焦](evidence/p4-03-visual-final/compare-28-rule-focus.png)
- [29 Tab离开规则](evidence/p4-03-visual-final/compare-29-rule-blur.png)
- [30 Shift+Tab返回规则](evidence/p4-03-visual-final/compare-30-rule-tab-focus.png)

14 的顺序状态由 GPUI 真实拖动、React 源码支持的移出/重新加入达成；未将 React 不支持的拖动假称为相同操作。数字步进实操300→301→300后取消，31原图只证明增加后的301；[Tab输入收据](evidence/p4-03-visual-final/tab-input-proof.txt)证明焦点从规则移入间隔（试输入1得到3001后撤回），[Shift+Tab收据](evidence/p4-03-visual-final/shift-tab-rule-proof.txt)证明返回规则后Down打开菜单，未保存。

15/16 的 React loading/error 是临时 fixture API 的异步/失败响应，GPUI是实际服务文件写入，不把 fixture当Native证据。

## 真实反馈与资源清理

- [17 保存重试成功](evidence/p4-03-visual-final/17-gpui-retry-succeeded.png)：真实写入失败后草稿保留；[故障断言](evidence/p4-03-visual-final/save-fault-result.json)记录writer7363bytes与原state hash未变；移除独立root注入后UI重试，[落盘结果](evidence/p4-03-visual-final/retry-state.json)确认config2、selection0及新名称。
- [18 自动](evidence/p4-03-visual-final/18-gpui-runtime-auto.png)、[19 手动固定](evidence/p4-03-visual-final/19-gpui-runtime-pin.png)、[20 切换中](evidence/p4-03-visual-final/20-gpui-runtime-switching.png)、[21 pending待确认](evidence/p4-03-visual-final/21-gpui-runtime-pending.png)。
- [22 核对过渡](evidence/p4-03-visual-final/22-gpui-runtime-reconciled.png)保留上次健康摘要，同时已读回子线路选择，不用该瞬间图证明最终Auto；[23 恢复自动](evidence/p4-03-visual-final/23-gpui-runtime-auto-restored.png)及[Controller/版本结果](evidence/p4-03-visual-final/native-visual-auto.json)证明最终Auto/pending清除。
- [24 全部失败](evidence/p4-03-visual-final/24-gpui-runtime-all-failed.png)、[25 停止](evidence/p4-03-visual-final/25-gpui-runtime-stopped.png)。正式选择均从现有GPUI入口经唯一Runtime执行，仅用GET核对当前child的Controller，无直接PUT。

这一轮为补拍真实反馈，沿用已有正式功能：固定 sing-box1.14.0，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，自有loopback peers55015/55016、受管child87685、mixed55088与Controller55089；端口来自实际ready log，不扫描用户服务。只暂停/恢复该自有child制造响应不确定，只令自有peers拒绝探测制造全失败。Native过程config保持1、selection推进2→5→8，真实读回primary/node-b；[日志](evidence/p4-03-visual-final/native-visual-desktop.log)、pin/pending/auto/all-failed JSON均保留。[清理断言](evidence/p4-03-visual-final/native-cleanup.json)确认child已reap与四个端口关闭。没有操作用户TUN/系统代理/DNS/默认路由/真实订阅。最终临时Vite1445和隔离预览也已退出，[最后清理结果](evidence/p4-03-visual-final/final-cleanup.json)留存，临时node_modules链接已移除。

## 最后修正后的定向检查

18个唯一定向测试（不是重复轮次相加）：

| 命令 | 结果 | 日志 |
| --- | --- | --- |
| `cargo test -p veyra-desktop ui::groups::tests -- --test-threads=1` | PASS，6 | test-groups.log |
| `cargo test -p veyra-desktop ui::components:: -- --test-threads=1` | PASS，12 | test-components.log |
| `cargo clippy -p veyra-desktop --all-targets -- -D warnings` | PASS | clippy.log |
| `MACOSX_DEPLOYMENT_TARGET=15.0 cargo build -p veyra-desktop` | PASS | build.log |
| `cargo fmt --all -- --check` | PASS | fmt.log |
| `git diff --check` | PASS | diff-check.log |

日志在同一证据目录。独立 Reviewer `/root/p403_review` 只读检查源码、原图与Native结果；未亲自重跑测试或操作Native。最终结论：**剩余0 Finding**。初始busy颜色/Save图标和焦点覆盖Finding已修正；天然Tab疑点以真实输入及规则menu收据闭合。审查不代替用户最终人工确认，也不代表95%全产品、深色或其他缩放通过。

## 保留的失败与可见边界

- 旧 Tauri Clippy缺Windows `libcronet.dll` 的历史 FAIL原样保留，本轮仅Desktop UI作用域，未重复执行/改写。
- 本轮错误焦点方案曾共享登记handle两次，Escape触发 `set_focus called more than once in a single frame`；`rejected-duplicate-focus-crash.log`、`rejected-28-duplicate-focus-crash.png`保留为FAIL/REWORK，已撤回，仅单一焦点登记的后续28–30为最终证据。
- 最初 unchanged草稿故障注入未产生写入、最初FIFO fsync未失败，以及错时截到loading/误点删除取消、参考fixture Sonner重复实例/selector超时，均不能作为PASS。`rejected-*`与`*-before.png`保留，不删除或改写旧失败；后续通过有独立真实writer/hash/重试/Controller证据。
- 保留用户此前批准的**不透明白色弹窗**。React半透明背景与模糊透出仍不同，影响弹窗大面积背景与输入外层，不声称完全一致。
- GPUI/macOS与Chromium文本栅格化/字重边缘及系统fallback可能不同，不追逐任意像素分数；标题栏属于macOS原生边界。Veyra品牌与停止态统计空值属于既有外壳语义，未作为本卡重设计。
- Runtime操作浮层在React参考中没有同等正式Runtime事务入口；保留真实业务状态截图，不制造React运行结果。
- 忙碌时GPUI继续禁止草稿修改/关闭/取消，React仅Save按钮禁用；本轮保留已有事务保护，修正其外观，没有扩大为交互事务重写。
- 深色/其他缩放按用户本轮要求NOT_RUN，留最终组合验收。最终人工确认、DONE、commit、merge、主分支复验、owner释放均**NOT_RUN / 待批准**。
