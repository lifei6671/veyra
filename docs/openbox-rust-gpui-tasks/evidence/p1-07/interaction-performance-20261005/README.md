# 暗色图标分类 Tab 与两处卡顿修复（2026-10-05）

只响应本轮用户反馈：暗色图标选择器分类 Tab，以及侧栏展开/收起和图标列表滚动卡顿。保留此前 dirty tree 与所有验收历史。不 commit/push，不启动 P2/Runtime，不访问公网，不改 Task/DAG 状态。

## 实际修改

- `ui/components/icon_picker.rs`：共享 Button 使用明确的 Custom normal/hover/active 配色，绕开 Kit Ghost 的默认颜色；normal 背景显式覆写，避免 Kit Custom 混合透明色。选中背景 `#70c996`、文字 `var(--text)`，未选中 `var(--muted)`，依据 `openbox.css:1698–1711` 与 `PanelIconPicker.tsx`。
- 同文件：896 个选项改用 GPUI 既有 `uniform_list`，仅构建可见32px行。搜索键随目录缓存；分类/搜索后回到列表起点；打开时把当前图标居中。原目录、选择事件、行为保存/CAS 保持原路径。新增一项测试保护分类、名称/代码大小写不敏感搜索及原目录索引映射。
- `ui/tokens.rs`：集中既有232px列表视口值，不改变256×320弹层规格。
- `ui/shell.rs`：固定页面视口使用既有 GPUI cached view 边界，避免父布局提前展开整棵页面控件树；尺寸/实体/依赖改变仍重绘。保留240ms ease宽度动画、256/64端点和快速反向语义。
- `Cargo.toml`：只给 `veyra-desktop`、`gpui-pre`、`taffy` 的 dev package profile 设置 opt-level=1，保留调试信息/断言，不增加依赖、配置项或运行状态。CPU sample 显示未优化布局/绘制为热路径。

## 真实证据与结论

用户提供的 FAIL/React参考图原样保存在 `host-gpui-tab-before.png` / `host-react-reference.png`。最终真实 Mach-O Feedback.app 的 Dark / Settings / 1280×720 logical content 截图为 `dark-picker-all-final.jpg` 与 `dark-picker-other-final.jpg`；选中和未选中文字、点击后停留的 hover 状态均使用 React 配色。该局部对照不声称整张页面新通过95%，MiSans fallback等既有技术差异仍保留。

候选版本已实际滚动图标列表：可见图标随滚动更新，AX仅出现8–9个选项 Button，不再构建896行。最终版本由用户亲自操作两处，明确回复“**两处都已流畅**”；见 `host-human-acceptance.json`。用户的人工作业与自动截图分开记录。

临时诊断记录保存在 `sidebar-before.json` / `sidebar-cache-only.json` / `sidebar-optimized.json` 及 `performance-summary.json`。日志追加历史导致 cache-only原始文件也包含之前三组；后四组才是缓存候选。展开的未优化稳定间隔约20–23ms；优化候选一轮完整动画30次Render采样、间隔中位数8.35ms，最大11.11ms。其他被连续反向打断的片段仍保留，某初始片段存在37.8ms间隔，不能用完成轮掩盖。这里是Render入口间隔，**不是GPU呈现FPS或所有交互的延迟承诺**。

采样是带临时logger的候选；其binary hash未记录，不冒充最终构建。最终源码已删除logger，并补齐Tab normal背景；最终build/hash单独见 `build-identity.json`。最终流畅度结论来自用户对实际最终构建的明确确认。没有为截图延长动画或重写历史FAIL。

52 Desktop tests、desktop check/clippy -D warnings/build、workspace fmt、旧Tauri lib clippy/fmt、resource override workspace locked check、git diff check全部PASS。命令/输出见 `validation.json` 和 `validation-*.log`；Tauri与workspace check使用bundle.resources=[] override，属于源码检查，不是产品打包验收。Core公共契约与实现未修改，本轮不重复Core全量测试或Tray组合验收。

本次按 code-delivery-review 做实现者定向自查，不称独立Review。最终应用保持运行，未替用户执行最终Tray Quit。
