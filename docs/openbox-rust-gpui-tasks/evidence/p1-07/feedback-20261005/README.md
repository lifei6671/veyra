# 2026-10-05 用户追加的三项 UI 修正

本轮在 Host 已提交的 HEAD3ee99dd 后修改4个Rust文件；原8bd7a366/P1-07验收历史不改写，Task/DAG状态不变。新build身份见build-identity.json。无commit/push/P2/公网/内核/系统代理/TUN。

1. 侧栏36×36/radius9收起/展开按钮hover增加共享IconButton背景rgba(75,82,99,.07)。React原CSS此处强制透明，本次为用户明确授权覆盖；不改导航/Tray架构。Light/Dark实际hover与Lightcollapsed截图，像素背景变化可见。
2. 测试站点/IP信息Tooltip共用12px字体、18px行高，集中tokens。原React测试站点14px/19.6px，本次按用户要求统一；宽度350/280、定位、Escape/blur等不变。两主题实际说明浮层已查看。
3. 视觉自动保存不弹Saving或Saved成功toast，保存/CAS/rebase和草稿状态不变，失败notify保留。行为保存/其他通知不改。真实sidebar/主题4次commit accepted=true/Ok，完成前后AX无关闭通知控件。

验证：Desktop45PASS，check/clippy-Dwarnings/build/fmt/diff-check全PASS，每条900s外层超时、cargooffline；不新增复述样式的低价值测试，不重跑无关Core全量检查。实现者自查按code-delivery-review Skill，不称独立审查。

GUI使用实际Mach-O Feedback.app与独立clone root；从未写原root或关闭原用户窗口，PID2839保留（旧8bd7a366）。新验证窗口debugQuit后进程/socket清理、flockfree，不冒充Tray验收。用户原待保存草稿未主动提交或丢弃。

results.json含build绑定截图和清理；JPEGICC→sRGB裁64physical原生标题栏再缩1280×720PNG。history-before-saving-toast-removal保留首次仅取消Success时发现Saving仍出现的截图/日志/自动检查，不作为最终静默PASS。最终测试/截图均来自最后源码修改后构建。
