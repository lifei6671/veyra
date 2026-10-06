# Dock / Tray 图标修复

用户要求 Dock 主体放大并将旧 V 托盘换成提供的图标。

application_icon.rs 共用 src-tauri/icons/icon.png；展示时裁掉多余透明留白、等比居中留边。源主体 654×664 / 1024，调整后约 831×844 / 1024。源 PNG/ICNS 不覆盖；Finder/bundle 文件图标仍保留源资源尺寸，此次修正作用于运行中的 Dock 和托盘。托盘使用 36px RGBA，tray-icon 以 18pt 显示，禁用 template 以保留用户图案颜色。不改托盘菜单、退出或窗口生命周期。

Desktop 55 tests、desktop clippy -D warnings、build、fmt、旧 Tauri lib clippy（resource override）、git diff --check PASS，详见 validation.log。实施者自查，无第二持久化层或新依赖。未启动 P2/内核/代理/TUN，未 commit/push。任务状态未修改。

真实 Mach-O bundle 已重新启动，沿用原 root。Dock 与菜单栏不在当前窗口截图工具范围内；最终肉眼大小、托盘可辨识度待用户验收，不以构建通过冒充视觉 PASS。
