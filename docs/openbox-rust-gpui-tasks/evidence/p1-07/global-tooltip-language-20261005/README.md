# 全局 Tooltip 与面板语言修复（2026-10-05）

本轮按用户新指令，将全部应用 Tooltip 在 Light/Dark 下统一为墨绿色。通过内置浏览器只读检查在线 OpenBox；具体 CSS、字号和颜色见 reference.json。历史黑色处理不代表本轮目标，旧证据不覆盖。

## 实现

- tokens.rs 集中 14px / 19.6px / #19362D / #CDD3D1；components/tooltip.rs 的共享 bubble 同时用于侧栏、帮助说明和普通 Button TooltipContent。触发行为保留已有实现。
- 新 ui/i18n.rs 从现有 DesktopVisualPreferences.language 投影 GPUI Locale，切换后刷新窗口；不增加持久化事实、修改 CAS 或翻译用户填写内容。
- 当前 Shell、Settings/Panel/Behavior、Select、图标库分类/搜索/名称、Modal、Notice、占位页及托盘菜单接入翻译。图标翻译和搜索字符串缓存，保留虚拟列表与 IME 提交后筛选。
- AGENTS.md 增加全局 Tooltip 和语言的持续规范，后续 UI 必须遵守。

## 实际验证

final-* 截图来自 identity.json 同一最终构建；其他截图属于 initial-identity.json，单独保留。截图映射见 screenshots.json。交互结果见 interaction-results.json。

实际验证英文全页与重启恢复、繁体全页、切换语言保留未保存 6601 输入（之后恢复 6600）、英文取消确认弹窗、图标库显示、Light/Dark 同一 IP 帮助提示及暗色侧栏键盘提示。没有把 disabled password 的无弹层截图算作 Tooltip PASS，也没有把本轮源码接入托盘翻译算作新的真人托盘验收。

最终自动验证：Desktop 55 tests PASS；Desktop clippy -D warnings、fmt、git diff --check PASS。此前同轮 Desktop check/build、resource override workspace --locked check、旧 Tauri lib clippy 均 PASS；详细命令和退出码见 validation.json / final-validation.json。未启动内核、System Proxy、TUN，未 commit/push。

## 明确边界

MiSans 分片加载问题仍使用系统字体 fallback；只统一字体选择、大小、行高和字重，不声称与网页 MiSans 字形完全一致。系统原生文件窗口文字由 macOS 控制。用户站点名称、URL、内部枚举和技术错误详情保持原值。未逐个强制触发全部失败路径。

实施者自查：翻译仅作用于展示，Select 回调保留原稳定值；没有修改业务持久化层；工作树已有图标、窗口/动画/输入反馈变更保持原状。任务状态与 DAG 未改动。应用保留运行供用户复核。
