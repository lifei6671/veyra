# 2026-10-05 Tooltip 鼠标移开关闭

收起按钮经鼠标点击后保留 GPUI focus，旧 visible 判断将所有 focus 视为提示来源，导致鼠标移到内容区仍显示。本轮只在共享 tooltip.rs 区分 pointer focus 和 keyboard focus，沿用原 hovered/dismissed/Escape/blur 状态与全部样式。

新增两条定向测试保护鼠标焦点不能让移开后的提示继续显示，以及键盘提示的 blur/Escape 关闭。实际最终 Mach-O build 重开后：展开按钮 hover 显示、离开关闭；折叠按钮点击后移到内容区关闭；Tab/Shift-Tab 回到按钮显示 focus ring 与提示，Escape 关闭，Tab 失焦关闭，均 PASS。无新主题/业务/持久化逻辑，Dark 没有重新截图，不冒充全套 P1 验收。

before-mouse-away.jpg 是修复前真实复现。history-old-process 两帧是初次重启尝试仍运行 PID23020 的旧代码，提示仍未关闭；不作为修复结果。确认 PID23020 经已有 debug Ctrl-Alt-Q Quit 正常退出后再启动，final-* 才来自新构建（build 与 ad-hoc 签名后的安装 digest 分开登记）。final-hover 在启动背景异步任务尚未 ready 时采集，仅作为提示显示证据；其后背景已 ready。此 debug 退出仅用于换构建，不代替或重记 Tray acceptance。

新构建保持运行供用户复核。既有反馈、769px 下限、响应式、原 P1-07 历史/Task 状态不改写；没有 commit/push、公网或 P2/内核/系统代理/TUN 操作。命令结果、截图 hash 与 PID 见 validation.json。
