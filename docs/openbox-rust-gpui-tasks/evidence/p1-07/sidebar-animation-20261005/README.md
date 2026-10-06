# 侧栏展开/收起过渡（2026-10-05）

本次仅响应用户新增的动画要求，保留已有 feedback、responsive、minimum-width、tooltip-dismiss 改动。不 commit/push，不启动 P2 或 Runtime，不改任务状态，也不改写此前验收历史。

## 事实来源与实现

- `src/openbox/openbox.css:78`：sidebar 256px、collapsed 64px、`width .24s ease`。复用 240ms 和标准 CSS ease Bezier `(0.25, 0.1, 0.25, 1)`。
- `ui/tokens.rs` 集中时长和曲线；`ui/shell.rs` 计算呈现宽度；`app.rs` 仅持有瞬时动画状态。
- 使用 GPUI `request_animation_frame()` 驱动下一帧；该 API 通知当前 view，结束后不再请求。尊重已有 reduce-motion。无新计时器、后台任务、配置项或持久化事实。
- 快速反向切换从当前插值宽度继续。偏好保存/CAS 及既有响应式断点保持原路径。内容 flex 区随侧栏宽度调整；导航子项仍采用既有折叠/展开结构。

## 实际验证与边界

最终二进制及 ad-hoc 签名后的安装二进制分别记录于 `build-identity.json`。真实 Mach-O Feedback.app 使用同一隔离 root；当前 PID 27759 保持运行。

Dark、1280×720 logical content，已实际操作鼠标展开/收起、Return 切换、120ms 内两次 Return 反向切换。最终展开256/折叠64几何正常，Settings 数据为 Dark / 6600ms / IPv6 on，终态无明显溢出。

`dark-expand-frame-*.jpg` / `dark-collapse-frame-*.jpg` 文件名表示采集尝试顺序，**不是动画中间帧**：工具自动等待后已到终态。键盘采集也在 Return 后788ms完成，超过240ms动画时长。截图证明终态，不证明逐帧流畅度；人工流畅度验收 NOT_RUN。Light 动画 GUI 本轮未重新执行，样式与颜色未修改。

回归测试保护展开/收起精确端点、真实中间宽度、结束及反向连续性。51 Desktop tests、desktop check/clippy/build、workspace fmt、旧 Tauri lib clippy/fmt、git diff check 全部 PASS；旧 Tauri clippy 使用既有 `TAURI_CONFIG` bundle.resources=[] override，属于源码检查，不是打包验收。无需因本次瞬时呈现状态变化重跑 Core 全量测试。

初次验证中 `super::*` 引入 GPUI 的 test 宏造成编译失败，已改为精确导入后通过；原始 FAIL 保存在 `history-initial-validation.json`，未改写。此次未重复 Tray 组合验收。
