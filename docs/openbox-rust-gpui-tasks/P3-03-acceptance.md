# P3-03 日志页验收记录（2026-10-09）

状态：DONE，owner/预约释放。隔离工作树 `dev/p3-03-logs`，基线 `fa4b655af692fe0a1d7f06da60ef12e01bdd8295`。按用户授权本地独立提交、合并主开发分支并复验；不 push。本轮只验 macOS、150%、浅色；深色及其他缩放 NOT_RUN，按用户要求留给 P3-08。

## 生产实现与边界

- 复用 P3-01 唯一四路 WS，日志端点固定 `/logs?level=trace`；页面和多个订阅者不创建 WS。受管 loopback 身份和代际检查不变。
- 采集入口生成类型安全 Record：受管 Identity、流代际/序号、真实接收时间、Level、可确认 Category、安全正文。未知字段不推断。正文先脱敏再限制到 4096 UTF-8 bytes；敏感 key/凭据整行遮蔽、URL 遮蔽、结构化原文不展示。旧 ClashLogSummary allowlist 诊断保持原逻辑。
- 广播容量 64，慢消费者 Lagged→Gap；每页 Buffer 最多 1000 条。隐藏/暂停排空页面订阅，恢复 resubscribe 拒绝积压，清空仅页面。筛选和导出使用同一投影，非法正则可直接修复。
- Route::Logs 持久 Entity 接入正式 AppServices 与唯一 ObservationReader，虚拟列表只构造可见行。仅 P2-06 owner 授权的 Desktop composition 初始化 Weak reader 槽和只读 getter；不持有 writer/owner，不修改公共 Runtime DTO、Helper、IPC、流量统计语义。P2-06 仍 DOING；P2-05 ACCEPTANCE/独立树未合并。
- 导出复用 NSSavePanel 与原子私有文件写入，点击时冻结当前筛选结果，取消非失败，失败固定安全提示。显示/导出均只消费安全 Record。
- 用户文案统一 i18n；尺寸/颜色集中 logs tokens，SVG 同源 Heroicons Pause，保留许可与 Legacy React 视觉参考。

## 已执行验证

证据均在本工作树本地 `evidence/p3-03/logs/`，旧文件不覆盖。定向测试按唯一 case 计 99 PASS：

| 范围 | 结果 | 证据 |
| --- | --- | --- |
| Core application::observability | 34 PASS | core-final.log/json |
| Core singbox::clash_api | 16 PASS | clash-final-04.log/json |
| Desktop ui:: | 41 PASS | desktop-ui-final-05.log/json |
| Desktop runtime_service:: 普通用例 | 5 PASS、3 ignored | desktop-runtime-final.log/json |
| 真实保存组件取消/成功/失败/重试 | 1 PASS | desktop-tests-01.log |
| 正式 AppServices 真实日志/替换/Quit | 1 PASS | native-final-03.log/json |
| 原统计 writer 的真实 Quit 回归 | 1 PASS | traffic-quit-final-01.log/json |

3 ignored 中两项真实 Native 已显式单独运行，另一个是 GUI fixture 准备；fixture 的单项通过不算功能验收。原 Core 全量基线 FAIL、Desktop 全量 Store Busy 未覆盖或改写。本轮没有通过删除/禁用旧业务测试处理失败。

真实内核 1.14.0 SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。Native 用例经正式 worker 建立受管实例，向自有 loopback HTTP 目标发送实际 mixed 请求，收到 204，验证持续日志、暂停不回补、真实筛选文件回读、实例替换身份隔离，Quit 后旧/新内核端口和 SQLite 资源关闭。无系统代理/TUN/DNS/路由改动。

Core/Desktop `clippy --all-targets -D warnings`、build、fmt、diff 已通过，最终 GUI 对应 build-12 / final-clippy-06 / final-fmt-07 / legacy-fmt-02 均 PASS。Legacy Tauri clippy 本轮仍因缺失 `binaries/sing-box-1.14.0-windows-amd64/LICENSE` FAIL（legacy-clippy-01），原样保留，不扩展修复。

## 实际 GUI 与修复历史

- 真实 GUI03：经受管 mixed 产生日志；级别/类别/正则筛选、非法正则修正、暂停/恢复不回补、原生导出取消/成功、Stop 清空旧实例记录。证据 gui-live、gui-filter、gui-invalid-regex、gui-paused、gui-resume-no-replay 截图及原生导出文件。
- GUI04：格式化查询同步更新真实筛选，清除搜索按钮有效，长正文单行省略。gui-format-final / gui-long-final。
- 用户确认单行复制；发现多行复制/输入 placeholder 裁切/非文本光标。GUI05 补 document_order、正文焦点、I 形光标及 py(0)，三行复制实证顺序 18→16→15，用户确认光标与复制正常。gui-multiline-selection / gui-multiline-copy / gui-placeholder-fixed。
- 用户发现跨行拖选闪烁/跳行。独立 Reviewer 确认正文 truncate 的 20px clip mask 与库内 16px 自动滚动触发区冲突。移除正文 overflow mask，保留 nowrap/ellipsis，继承外层 UniformList 视口；去掉每次 render 无条件 set_placeholder notify。用户确认闪烁消失，但发现鼠标松开才显示高亮。
- GUI09：独立行选择句柄与库内 refresh_window_on_change 订阅使拖动即时重绘；订阅随可见行 keyed state 释放。用户实际复验确认“现在跟手且稳定”，I 形光标、单/多行复制已通过。
- 用户随后指出时间字号太大，明确要求与级别一致；最终时间与级别同为 12px、MiSansMedium，时间保持 tabular-nums / 单行。这是用户覆盖 React time 14px 的明确差异；不改 React 参考。
- GUI09：真实 NSSavePanel 成功导出，取消不报失败；另以自有超长文件名触发真实原子写入失败，页面给出明确安全错误（gui-export-failure.png）。
- GUI12：最终构建在新自有 root 正式 Ready，mixed 向自有目标 3×204；21 条真实日志导出回读。gui-final-12.png 与 react-final-12.png 使用相同记录、trace/全部/空搜索、列表顶部、1280×720 逻辑内容区域；Native 另含原生标题栏与 2× backing。逐项核对工具栏/控件/行高/间距/圆角/字体/级别颜色/省略；debug 背景按 React rgba(247,247,248,.76)，导出/暂停继续/清空使用全局 Tooltip。React Shell 品牌和静态指标与 Native 运行指标不同，不作为日志页像素一致的证据；时间 12px 为上述用户明确例外。

旧 gui-01 初始化失败、gui-02 未注册 Pause panic、core-tests-01 取消时序失败、clash-final query fixture 旧路径失败、desktop-ui-final 重复 i18n key 失败、build-06 API getter 编译失败均保留；修复后使用新编号证据。gui-06 误复用上一个成功 binary，没有执行验收，明确 NOT_RUN。

GUI07 在复用旧临时目录时两次 CandidateFailed；只读确认 manual_sidecar check 的 candidate-1 create_dir 与残留目录冲突。已交接原 P2-06 owner，不修改其范围，不删除旧证据；GUI08 使用新自有 root，正常 Ready 并经实际 mixed→自有 HTTP 3×204。该边界不能冒充 Runtime 异常退出重建通过。

## 独立 Review、清理与交付

独立 /root/review_p303 使用项目 code-delivery-review skill，只读审查实际 diff、服务边界、操作证据与最终同数据截图。原格式化筛选不同步、搜索图标/清除、正文省略、复制顺序/焦点/光标/placeholder、拖选闪烁与刷新、Tooltip 遗漏均已复核关闭，最终未发现剩余可操作 Finding。实现者未冒充独立 Review。

限定 macOS、150%、浅色的功能与视觉验收通过；深色/其他缩放留 P3-08，NOT_RUN。跨虚拟屏全缓冲选择未执行，不写 PASS。Core 全量基线失败、Desktop Store Busy、Legacy Windows 资源问题与各轮旧失败记录保留。GUI07 异常退出后旧 candidate 目录冲突交由原 Runtime owner，未在本卡修复或冒充其验收通过。

最终自有 GUI 经产品 Stop 后 mixed/controller/目标端口均关闭；随后只向自有预览父进程发 SIGTERM 清理，不把该操作计为 GUI Quit 验收。正式服务 Quit 的 child/SQLite 清理由两个真实 Native 用例证明。自有 Vite、临时浏览器页和视口已清理，证据/bundle/私有测试数据保留（cleanup-final-12.json）。用户现有网络与其他工作树未修改。

本卡 DONE，owner/预约释放。按授权本地独立 commit → 合并 codex/dist-react-restore → 同范围主分支复验，不 push；合并后命令与实际结果追加在本地 main-final-* 证据，不以预期结果填 PASS。READY 仍为 P0-08/P4-03，未启动下游；P2-05 ACCEPTANCE 未合并，P2-06 DOING/原 owner 保留。

## 用户追加：导出失败 toast，成功安静（2026-10-09）

- 用户明确将导出失败反馈改为全局 toast，成功不提示；取消仍不是失败。只改 ui/logs.rs：复用 NoticeCenter.notify_app，只对 Err 发布固定安全文案，Ok(true)/Ok(false)均无通知；删除 export_error 与永久错误横幅，不影响筛选数据或列表布局。文案继续由全局 i18n 渲染，不拼接错误、文件路径或原始日志。
- 最终 toast-build-01 真实 macOS/150%/浅色 GUI13：正式受管 mixed 向自有 loopback 204 产生日志；NSSavePanel 选取自有超长文件名触发原子写入错误，toast 显示“日志导出失败，请重试”（gui-toast-failure-13.png）。重试正常文件名成功，实际回读 3 条 info 筛选记录，无通知（gui-toast-success-13.png/txt/log）；Cancel 后无通知（gui-toast-cancel-13.png/txt）。旧 GUI09 的错误横幅截图作为历史保留，不覆盖。
- 42 个唯一定向测试 PASS（ui 41、真实保存组件 1）；Desktop all-targets Clippy -D warnings、build、workspace fmt 和 diff 检查 PASS，toast-*-01 证据保留。未新增只镜像分支的低价值测试，真实 GUI 验证负责提示行为。
- 独立 /root/review_p303 只读复核实际代码与三种 GUI 结果，最终无剩余 Finding。仅自有预览/实例，产品 Stop 后 mixed/controller/目标端口关闭，再停止自有预览父进程；不把 SIGTERM 计为 GUI Quit。gui-toast-cleanup-13.json 已记录。
- P3-03 保持 DONE，追加修正 owner/预约释放；P2-06 Runtime/Helper/IPC 与全局通知样式不改，P2-05/READY 队列不变。用户原任务授权范围内本地增量提交、合并主分支及定向复验，不 push；主分支实际结果另存 toast-main-*。
