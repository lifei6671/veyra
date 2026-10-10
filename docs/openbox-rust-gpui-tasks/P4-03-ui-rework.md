# P4-03 出站列表与编辑弹窗视觉返工

2026-10-09，**ACCEPTANCE**。用户明确指出列表卡片、添加和修改弹窗有明显差距；前轮局部 PASS 不等于用户接受整体视觉。独立工作树 `dev/p4-03-runtime`，HEAD `2f8e20fb2f3d082ffc439512dde140455665fecb`；无新增 Commit、Merge SHA、主分支复验或 owner 释放，不 push。P2-06 DOING 与原 owner、P2-05 ACCEPTANCE 不变。

## 写范围与行为

原 P2-06 owner 确认本轮 UI 文件没有相交预约。修改仅 `ui/groups.rs`、既有 `tokens`/`i18n`/`IconPicker` 与本卡文档；此前正式 Runtime 增量完整保留。`failover_editor.rs`/`Select` 的此前增量未在本轮重复修改。证据目录 `evidence/p4-03-ui-rework` 保存返工前六个 UI 文件及 SHA，`ui-rework-only.patch` 区分本轮与此前 Runtime diff。

- 卡片恢复 React 两行结构，实际内容高度 62px、间隔8px、标题14px、摘要11px/16px；固定三按钮30px。取消常驻主备控制区，避免把每张主备卡撑高。浅色卡片恢复 `--surface` 的白色75%，hover边框17%。
- 主备名称打开既有 Popover，复用 `runtime_controls → GroupsRuntimeEvent → RuntimeService → ManualRuntime`。没有第二个 Controller 写入口。切换中、outer/lane pending、失败仍直接显示在原摘要行。独立 Review 找到 lane pending 遗漏后已修复。
- 添加/修改尺寸896×648、header41、footer49、padding16；224px图标、158px缩放、规则174px、单位输入80px、双栏404px/中间32px/间隔12px，沿用已有共享组件。
- 空 placeholder、全局测速提示、初始非输入焦点、工具按钮 neutral hover、保存禁用50%透明度、内置组说明按 React 对齐。IconPicker显式MiSans Regular/14px，不改变业务图标。
- **编辑弹窗采用不透明白色**，依据 `P4-02-acceptance.md:46,74` 的已批准 GPUI backdrop blur 替代方案。没有借本轮恢复未经批准的透明无模糊背景。

## 真实视觉证据

沿用用户当前 macOS 显示设置：3024×1964物理、1512×982逻辑、2× backing；浅色、应用内容1280×720。backing不被重新解释为200%用户缩放。原生标题栏排除比较，字体栅格化差异单列。

在线 `192.168.1.6:3036/#/settings` 只读打开列表/添加/主备修改，未保存；`01-original-list`、`02-original-add`、`04-original-edit-failover` 为真实参考。部署页为 Vue，当前仓库的唯一视觉事实仍是 `src/openbox/**`/CSS；不能把不同数据的在线页与夹具做全图评分。

本轮补齐完整 React `AppShell + SettingsPage`，不再用手写简化 shell。两侧同五组、三个人工 loopback 节点；共同使用仓库自有背景图，背景偏好100%/blur10。GPUI原图2560×1504只去64px原生标题栏，再缩至1280×720；拼图仅拼接真实截图，不重绘内容。

| 核心状态 | 证据 | 结果 |
| --- | --- | --- |
| 五组列表 | `compare-list-final.png` | PASS：卡片尺寸、排序、两行、图标/按钮位置逐项核读 |
| 添加：静态/未填/禁用保存 | `compare-add-final.png` | PASS；不透明弹窗技术差异单列 |
| 普通自动组修改：动态/命中节点 | `compare-edit-dynamic.png` | PASS |
| 主备修改：主用Manual | `compare-edit-primary.png` | PASS |
| 主备修改：备用Auto/高级设置 | `compare-edit-backup-advanced.png` | PASS |
| 内置修改：短弹窗 | `compare-edit-builtin.png` | PASS；截图后说明已逐字改为React文案，位置不变 |
| 添加/删除新备用、改名后取消 | `gui-cancel.json` | PASS：原5组/2线路、config1保留，无草稿写盘 |
| 真实保存失败、原地重试 | `17-gpui-save-failed`、`18-gpui-save-retry`、`gui-save-retry.json` | PASS：只让自有state.tmp成为目录；失败原state SHA不变，恢复后config2/新动态组落盘 |

**完整95%视觉仍 OPEN**。上述是逐态局部对齐与实际操作，不是全局百分比证明；完整控件focus/hover矩阵尚不足。React MiSans-VF与GPUI官方静态face/CoreText栅格化、已批准白色弹窗、全局shell背景/品牌/运行态差异不以全图平均掩盖。没有删除 Legacy 视觉源码。深色/其它缩放仍 NOT_RUN，留最终组合验收。

## 新增 Native 浮层复验

固定自有 sing-box **1.14.0**，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。根 `/tmp/veyra-p403-ui-01a12040`、自有peer端口57759/57760、child69327；Controller57887/mixed57886来自该child的ready日志，未扫描端口发现控制器。只读 GET `/proxies`，选择写操作全部通过真实 UI/原Runtime。

- 人工固定备用：浮层即时显示“手动固定”与“选择已确认”；state/Controller一致，config1、selection2，`native-popup-pin.json` / `12-gpui-popup-pin.png`。
- 自有child受控SIGSTOP12秒，并在监督脚本finally SIGCONT。线路内Manual选择进入切换中，然后真实不确定 pending；outer仍ManualPin，config1、selection3。`13-gpui-popup-busy` / `14-gpui-popup-pending` / `14-gpui-summary-lane-pending` / `native-popup-pending.json`。未伪造成功或清旧已确认选择。
- child恢复后 UI“核对选择”读回实际node-b、清pending；UI恢复Auto重新检测并实际切回主用，config仍1、selection8。`15-gpui-popup-reconcile` / `16-gpui-popup-auto-final` / `native-popup-auto.json`。浮层随真实Runtime投递更新。
- 没有操作用户TUN、系统代理、DNS或默认路由，也未操作真实订阅。UI Stop确认自有child退出；进程/端口/预览清理见 `cleanup.json`。

本轮没有重复此前全套Native/重启策略验收；历史结果继续在 `P4-03-runtime-acceptance.md`，不能把本轮3份JSON计为3个自动化Native测试。

## 验证与独立 Review

- Groups **6 PASS**、共享components **12 PASS**、i18n **3 PASS**，共21个唯一定向UI测试。新增case不以重复运行累计数量。最终groups/i18n日志与components日志保留实际测试数。
- `cargo clippy -p veyra-desktop --all-targets -- -D warnings`、Desktop build、`cargo fmt --all -- --check`、`git diff --check` **PASS**。
- 初次build因Popover trigger的Selectable/错误custom API **FAIL**保留；修正后build03至build06及build-final PASS。未把历史失败改为PASS。
- 旧Tauri Clippy缺 `libcronet.dll` 的历史 **FAIL** 保持，本轮纯GPUI未重复该无关失败；不能据此宣布全部项目检查PASS。前端产品源码未改，pnpm lint/test/build **NOT_RUN**。
- 独立只读 Reviewer `p403_review` 审本轮真实patch、React对照截图及NativeJSON：1个P2 lane pending反馈遗漏已修复，最终代码与收据复核**0 Finding**；Reviewer未亲自运行GUI/内核/测试。

任务继续 **ACCEPTANCE**。当前卡片/添加/修改的明确差异已修正并可审查；全局95%边界及历史Clippy不通过尚未收口，不提前DONE、合并或释放owner。
