# 769px 最小窗口内容宽度

用户本轮明确要求不实施手机布局，只限制最小宽度。仅 main.rs 的 WindowOptions 增加 window_min_size=769×0 logical，macOS GPUI 映射至 NSWindow.setContentMinSize；0 高度不新增高度限制。此前 dirty UI 修改全部保留，原 P1-07 历史与状态不变。

相同 Feedback Mach-O bundle 更新后，用户通过托盘退出旧进程，本轮重开新构建。原生右边缘从1280向675 logical拖动，实际停在769；展开侧栏后继续向550拖动，仍为769。截图宽1538 physical、scale2；展开保持单列，折叠保持既有双列。没有新建手机顶部/底部导航，没有修改持久化值。应用保持运行供用户复核。

验证与构建身份见 validation.json。只验证原生宽度限制，未重新执行 Tray 组合验收；Cmd+Q 无响应记录仅为旧测试进程退出工具限制，不计 Tray FAIL。未提交/推送，未访问公网、启动P2/内核或改系统代理/TUN。
