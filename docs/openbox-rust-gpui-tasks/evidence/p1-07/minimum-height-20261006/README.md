# 最小窗口高度

用户要求防止纵向挤压。WindowOptions.window_min_size 从 769×0 调整为 769×600 logical content，保留纵向内容滚动。只修改 main.rs 的尺寸和注释。

Desktop check/build、fmt、git diff --check PASS。未新增低价值尺寸常量单测。新真实 bundle 已启动，沿用 Feedback root。原生底边拖动工具返回 windowNotFoundAtPosition，上边缘拖动未改变尺寸，因此不声称自动拖动验收 PASS；待用户手动拖动复核。无 commit/push、P2、内核或代理操作。
