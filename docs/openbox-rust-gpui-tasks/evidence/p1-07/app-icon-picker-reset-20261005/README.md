# 应用图标与 Icon Picker 反馈修复（2026-10-05）

本轮只修改应用图标、重新展开/清空筛选同步、用户指定的暗色选中 Tab 黑字。未更改 Task/DAG、Tray、保存/CAS、IME 提交事件、虚拟列表或侧栏动画；不 commit/push。

## 原因与改动

- 固定依赖 gpui-base 0.7.0 的 InputState::set_value 只设置内容，不发送 Change。此前 textbox 为空但 committed query 仍是百度。本轮 clear_search 同步清空 query 和 textbox，供展开及清空按钮两处调用。
- 暗色选中 Tab 用 PICKER_SELECTED_DARK_TEXT=#000000；未选中仍用 muted_foreground。既有 normal/hover/active 配色保持一致；浅色逻辑保留。
- macOS 复用 src-tauri/icons/icon.icns，通过 NSApplication.setApplicationIconImage 设置 Dock 图标。仅开启既有 objc2-app-kit 的 NSImage feature，无新依赖。测试真实 app bundle 同时复制 icon.icns 至 Contents/Resources，并设置 CFBundleIconFile=icon.icns，再 ad-hoc codesign/verify。
- 图标复制映射属于当前本地测试 bundle；项目尚无 GPUI 发行打包入口，本轮未建立新的打包系统。运行时嵌入路径是永久源码接入，cargo run 不依赖临时 bundle/cwd。

## 实际验证

同一修复 build、1280×720 logical content、显式 Dark、同一隔离 root。身份见 build-identity.json；对应截图/结果见 interaction-results.json。

1. 百度输入提交 -> 三项；Escape 收回再打开 -> 清空，完整目录恢复。
2. 再输入百度 -> 清空按钮 -> 清空，完整目录恢复。
3. Company + 百度 -> 外部点击关闭再打开 -> All + 空筛选 + 完整目录。
4. All/Company 暗色绿色选中项黑字；未选中灰字。Company 点击悬停与移开后配色不变。
5. 用户确认 Dock 图标“图标已正确显示”。没有声称自动化截图覆盖 Dock。
6. 中文 IME 的前轮人工结果保留，本轮未重新执行真人候选流程；提交后筛选契约未修改。

## 自动验证与自查

52 Desktop tests PASS；desktop check/build/clippy -D warnings、fmt、旧 Tauri lib clippy（resource override）、旧 Tauri fmt、workspace --locked check（resource override）、git diff --check 全部 PASS。全部离线执行；详细命令/退出码见 validation.json 和 validation-*.log。

实现者按 code-delivery-review 自查：同步清空入口覆盖展开/clear，分类与图标 identity/scroll 保持原行为，主题覆盖限暗色选中项，NSImage 在主线程启动阶段加载一次。未发现本轮范围内未解决问题；这不是独立审查。

应用保持运行供用户体验。历史证据未覆盖。
