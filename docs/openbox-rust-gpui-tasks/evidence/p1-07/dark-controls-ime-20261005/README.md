# 暗色控件与中文筛选反馈修复 · 2026-10-05

仅处理用户本轮四项反馈；不改 Task/DAG 状态，不 commit/push，不启动 P2、内核或网络。先前 dirty tree 与所有历史证据保留。

本轮增量产品文件：ui/panel.rs、ui/components/mod.rs、ui/components/tooltip.rs、ui/components/icon_picker.rs、ui/tokens.rs（均在 crates/veyra-desktop/src）。不修改之前已确认流畅的侧栏动画、列表虚拟化、Tray 或 Core/CAS。

- 圆角中段：保持 React CSS 856–862 的 190px 总宽、48/96/48px 与 -1px 拼接，在右按钮绘制后补绘被盖住的同一条 1px 分隔线；暗/浅色均肉眼复验。
- 数字 spinner：独立 occluding hitbox 与 default cursor，MouseDown 阻止输入文字区域截获；仍通过原 InputState Change 更新、验证和保存。使用截图物理坐标点击，焦点下暗/浅色都完成 800→801→800（不是 AX 直接调用按钮代替鼠标命中）。数值恢复，未提交测试草稿。
- Tooltip：本轮运行的旧版本实际为 #30394c（没有复现用户截图墨绿色），按用户明确要求把共享暗色 Tooltip 及箭头设为 #000000；这是经用户指定的基线变更。Light #30394c 不变；字体保持此前用户要求的统一12/18。
- Tab：上一轮 custom variant 已让 normal/hover/active 使用相同 accent 与 foreground。本轮同最终 build 保存 selected hover/idle 截图，未复现背景或字体翻色；本轮没有再改 Tab 样式。输入框焦点时选中项同样保持颜色。
- 中文筛选：Popover 原先每次重绘直接读取含 IME 预编辑的 InputState.value。改为持有已提交 query，只在 InputEvent::Change 更新。pinned gpui-base 0.7.0 的 replace_and_mark_text_in_range 只 notify，replace_text_in_range commit 发出 Change。工具粘贴 百度 验证三项结果与清空，不能冒充输入法；用户独立确认“候选时不筛选，提交后筛选正常”，见 human-ime-acceptance.json。

实际 UI 使用同一真实 Mach-O bundle/root，1280×720 logical content、2x、明确 Dark/Light，最终恢复 Dark，应用保持运行；identity/PID 见 build-identity.json。截图文件列于当前目录，均来自本轮最后源码构建，没有复用旧帧；spinner 截图包含中段边框。

52 Desktop tests、Desktop check/clippy -D warnings/build、workspace fmt、旧 Tauri lib clippy/fmt、git diff check 均 PASS。旧 Tauri lib 使用 bundle.resources=[] 的 TAURI_CONFIG override，仅源码检查，不代表打包验收。日志与命令见 validation.json / validation-*.log；本轮没有新增依赖或扩大测试框架，没有重跑 Core 或 Tray 组合验收。

按 code-delivery-review 做实现者定向自查：Change 仍由原输入状态发出，数值/CAS未绕过；列表仍按可见行构造，筛选仍保留目录索引；Tooltip 明确主题颜色且箭头继承。此为实现者自查，不称独立审查。首次工具坐标按1x误点空白未产生数值变更，改用实际2x截图坐标后成功，不记首次点击为 PASS。
