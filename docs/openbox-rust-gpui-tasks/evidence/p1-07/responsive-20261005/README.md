# 2026-10-05 窄窗口响应式布局修复

用户明确要求按所附 React 图二自适应，解决 GPUI 固定双列导致的组件穿透。继承 feedback-20261005 的 hover、Tooltip 字号和静默视觉保存修改；不改写原 8bd7a366 验收或 Task/DAG，不 commit/push，不访问公网或启动 P2/内核/系统代理/TUN。

## 实现与来源

React `openbox.css:1640–1644,1745–1748`：默认单列；展开侧栏 viewport >=1024 双列，折叠 >=768 双列；column gap 随 1024/1280/1536 为 32/48/64，默认16；row gap4。`SettingsLayout` 复用真实网格（minmax(0,1fr)），用于通用、延迟/布局、测试站点。去掉固定双列 helper 与空单元占位，单列保留 React DOM 顺序；CompactSetting/label/control 与站点行保留 min-width:0，宽度服从卡片。Sidebar 状态由现有视觉 draft 投影给 BehaviorPanel，仅用于布局；不新增持久化事实，不动保存、CAS、数值验证和通知错误语义。

本次增量产品修改：app.rs、ui/tokens.rs、ui/components/mod.rs、ui/panel.rs、ui/behavior_panel.rs。原先 shell.rs/tooltip.rs 等追加修改保留。

## 已执行 GUI

同一源码构建 `caa790b5`，真实 Mach-O Feedback.app、隔离 clone root。显式 Light / 6600 / IPv6 on。1280x720 展开：双列无重叠。原生右边缘拖到900x720、746x720：通用按语言/背景/圆角/主题/密码/IP 顺序单列；延迟与布局单列；四个站点单列，名称/网址/clear 均在卡片内，长网址在输入框内截断。滚动到保存按钮正常。746x720折叠侧栏：保持单列、站点网址得到更多空间。通过现有 debug Ctrl-Alt-0 还原1280后实际看到各区域恢复双列；此状态未保存单独最终截图，不能冒充768证据。

截图见 screenshots.json，JPEG保留原始capture，PNG经ICC转sRGB、仅去除64physical原生标题栏、2x还原实际logical尺寸。没有全图相似度指标，也不称重新完成全套P1验收。参考为用户提供的两张截图与本地React CSS；本轮未重新运行React页面或公网。

768px原生拖动的两次截图实际仍是746/1280，已保留到history-resize-attempt并标NOT_RUN；不是断点视觉PASS。折叠768断点与展开1024边界由新增回归测试覆盖。本轮未新增Dark响应式截图。最终进一步输入被用户窗口操作打断，重新只读确认后保留测试应用运行供查看，未代用户退出；原用户PID2839在过程中不再存在，未由本轮发送Quit或杀进程，不推断退出操作者。

## 验证

Desktop 47 PASS（含2个保护响应式断点/间距的回归）；desktop check/build/clippy -D warnings、fmt all、legacy lib clippy（resources=[] override，非打包验证）、legacy fmt、git diff --check 全PASS。每条外层900s超时，Cargo offline。没有重跑与本次布局无关的Core全量测试。源码hash和bundle身份在build-identity.json，验证后源码未再次修改。测试窗口PID15472保持运行，独立root有自己的primary，不构成同root第二writer；本轮未执行Tray组合验收或cleanup，不能把保留运行写成无残留进程。
