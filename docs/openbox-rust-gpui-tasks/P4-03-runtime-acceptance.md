# P4-03 正式 Runtime 组合集成

后续用户明确要求列表、添加/修改弹窗视觉返工；[最新 UI REWORK 收据](P4-03-ui-rework.md)补充本记录，历史 PASS/FAIL 不改写。后续已恢复 P4-02 批准的不透明白色编辑弹窗，并将真实主备操作收进名称浮层。

2026-10-09，**ACCEPTANCE**。检查点 `2f8e20fb2f3d082ffc439512dde140455665fecb`；独立工作树 `/Users/lifeilin/.codex/worktrees/p4-03-runtime/veyra`，分支 `dev/p4-03-runtime`。增量尚未 commit/merge，主分支复验与 owner 释放 NOT_RUN；不 push。P2-05 ACCEPTANCE、P2-06 DOING 与原 owner 不变。[前一切片及历史结果](P4-03-acceptance.md)保留。

## 当前行为

- 原 `ManualRuntime::select_manual` / Desktop 单 worker 是唯一选择入口。闭合类型区分真实 Node、Group 外层与 Manual 子线路，Group ID 不编码为 NodeId。普通 Manual pool 回归保留。
- 入口校验实例身份、Group、state_epoch、config_revision、selection_revision、已应用版本和全局 pending；沿用 pending → Controller PUT → GET → 精确 CAS。自动切换、手动固定和核对只推进 selection_revision，不生成配置待应用提示。
- 手动操作在提交前使旧探测失效。实例、配置或成员变化取消旧探测；迟到结果不能覆盖用户选择。恢复自动清累计并重新探测；其它 selector 的 selection_revision 变化只重新绑定 generation，保留当前线路阈值和恢复保持时间。
- Auto/ManualPin、已确认成员及 pending 使用现有 AppState/JsonStateStore。已切换但确认保存失败明确报错；不确定结果保留 pending。重连/重建读取真实 Controller，核对过程不盲重发 PUT；旧恢复文件缺少 groups 字段兼容读取。
- FailoverPolicy 接入实际受管实例，按配置间隔调用现有 authenticated URLTest GET。Compiler 的只读健康 companion selector 展开既有目录叶子，不建立第二份 OutboundCatalog、不注册第二个选择写入口、不重写 URLTest 算法；用于避免内核 URLTest 自身忙碌时返回空结果而误报全部失败。
- 达到连续失败阈值切至首个可用备用，主用持续恢复达到等待时间再切回；全部失败保留已确认选择并显示“全部线路不可用”。线路内 Manual 选择不固定外层。
- GPUI 接入真实结果：切换中、成功/错误、待确认、核对选择、恢复自动、实例替换后的过期响应拒绝。共用已有弹窗、Select、IconButton、NoticeCenter、tokens 与三语言 i18n；Empty/Error 状态位于已有共享组件模块。

## owner 协调和审查

P2-06 原 owner 在原 chat `01a11964-1a8a-7923-b510-f4194f29c2e1` 明确授权本组合树的临时精确写范围：ManualRuntime/邻接选择子模块、SelectionService、Group 选择的 Store/校验、恢复 index/Compiler、Desktop 单 worker/services/Groups UI、受管 Controller 只读健康 seam 与必要测试。公共选择类型、恢复格式、只读 `ManualRuntimeSnapshot.saved_selection_version` 均单独复核。

Helper/IPC/remote-selection fence/owner-transfer/Platform bootstrap/rebind/Observation 实现不改。Helper mutating inlets 在 FirstFreeze/Prepare/Commit 等修改前检测不支持的 Group/pending/Failover 并 HandoffRequired；Stop/Status/Query/清理不被一概禁用。仅复用既有 observation.stop 撤销统计与观测。

原 owner 最终只读复核关闭失活 Finding：select/reconcile 共用 `selection_child_alive`；退出时不再发布旧 Ready，清理失败进入 Recovering、保留原恢复计划供 Stop；pending 和持久业务状态不变，Controller 不被访问。owner 核读 45 PASS 日志但未独立执行；其 DOING 与所有权保持。

独立只读 Reviewer `/root/p403_review` 检查实际源码、未跟踪新模块、GUI 原图/JSON、Native 日志。已修复并复核关闭：pending 成员保护、非法 pin 回退、旧恢复格式、跨组 revision 误清计时、URLTest 忙碌误判、普通 Group 非法 ManualPin、GPUI pending 版本拼接、核对入口二次完整 CAS、pending 确认后的错误回退、pending 跨重建、弹窗透明度、advanced hover、错误卡额外 8px。最终所查范围无剩余可信 Finding；Reviewer 未亲自运行测试/Native/GUI/系统缩放，不替代整卡验收。

## 验证收据

所有日志/原图位于本机 ignored [evidence/p4-03-runtime](evidence/p4-03-runtime/README.md)。Rust 使用 offline 与 `MACOSX_DEPLOYMENT_TARGET=15.0`，长命令由外层 600 秒限时 runner 记录。过滤零测试不算 PASS；未执行无过滤的真实资源全库测试。

| 验证 | 真实结果 | 日志 |
|---|---|---|
| ManualRuntime 定向回归 | 45 PASS | core-runtime-liveness-final.log |
| Groups/保存/成员/pending | 20 PASS | core-group-regression.log |
| 旧及新 Compiler | 20 PASS | compiler-regression.log |
| Groups UI 契约 | 6 PASS | gui-groups-final.log |
| Select/IconPicker/Tooltip/共享组件 | 12 PASS | desktop-components-final.log |
| i18n 三语言 | 3 PASS | i18n-states-final.log |
| Helper 入站修改前拒绝 | 1 PASS | helper-admission-final.log |
| Core + Desktop all-targets Clippy，-D warnings | PASS | clippy-liveness-final.log |
| Desktop build | PASS | build-liveness-final.log |
| workspace fmt / 旧 Tauri fmt | PASS / PASS | fmt-final.log / legacy-fmt-final.log |
| 固定内核 Native ignored test，显式单线程 | 1 PASS，0 ignored | native-liveness-final.log/json |
| 旧 Tauri Clippy --lib -D warnings | **FAIL 101**，缺 binaries/sing-box-1.14.0-windows-amd64/libcronet.dll | legacy-clippy.log |
| diff 空白检查 | PASS | diff-final.log |

上面定向测试按完整测试名去重共 **107 PASS**，Native 另计 1；之前阶段的 57 项不叠加冒充本轮数量。Clippy/build 的早期实现错误以及 Native 初始失败原样保留；最终 PASS 不改写历史日志。

Native 固定 `/tmp/veyra-p402-check-20261008/veyra-sing-box`：1.14.0 / revision `0b89958` / SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。生产 `ManualSidecarPort` 实际 check/run/PUT/GET，自有随机 loopback、严格受控 HTTP peer、私有 root，不接触用户网络资源。

最终 Native 实际验证切备/切回、Auto 备用 URLTest、人工固定、Stop 后重建 Store/Service/owner 与新实例恢复、一次真实 GET 成功结果丢弃后的 pending 核对、pending 跨 Stop/重建恢复、全部失败保留已确认选择、同健康 tag 并发读取。配置 revision=11 全程不变，selection_revision=12。核对与 pending 重启恢复 PUT=0。三个自有 child PID 57721/57804/57823 均 reaped、进程组消失、监听端口关闭；受控故障注入不声称自然网络故障。

## macOS 当前显示设置的实际 GUI

用户明确“沿用当前 macOS 显示设置”。保持系统物理 3024×1964、逻辑 1512×982、GPUI backing scale 2；应用内容视口 1280×720，浅色。没有修改显示设置，也不把 backing 2 解释为用户选择了 200%。React 同视口使用真实 GroupSettings/helpers/openbox.css，仅隔离 API fixture 与页面 shell；未连接用户订阅。

最终 unsigned Desktop SHA256 `82cc8e1d82fcde510b4e262d3b86935b4fe286e396c8a5bfe73eec05d5e80fb4`；自有 Runtime bundle ad-hoc 后 SHA256 `85d4f6cf506a83de2c726a65bd37ab21da59a460fffc00815f925cf49e390255`。此前保存、重启、编辑截图来自不同候选构建，不能倒绑到最终 SHA；实际绑定详见 evidence README。

| 实际操作/状态 | 结果及证据 |
|---|---|
| 主用/备用/高级，同三节点、参数、顺序 | 实际编辑与局部几何对照；gpui/React *-geometry-final.png；advanced hover 原图 |
| 人工固定备用与恢复自动、切备/切回 | PASS；gui-pin-backup / resume-auto / auto-reviewed-final.png |
| 子线路手动选 US 主用 B，不固定外层 | PASS；gui-manual-lane.png / state.json |
| 重启恢复人工固定与子线路成员 | PASS；gui-restart-pin-restored.png |
| 全部失败仍保留旧已确认选择 | PASS；gui-all-failed-reviewed-final.png / state-final.json |
| 保存真实失败保留草稿、重试、取消 | PASS；gui-save-failure-final / retry-final.png，cancel-final.json；仅自有 state.tmp 故障 |
| 最终切换中、disabled、浅色加载 spinner | PASS；gui-busy-liveness-final / loading-captured-liveness-final.png；仅自有 child 受控 SIGSTOP，finally SIGCONT |
| 最终实际 pending → 核对 Controller → 确认 | PASS；gui-reconcile-liveness-final.png/json：config4，selection28，备用 ManualPin，pendingNone；后续主用核对同样确认 |
| 最终空态、读取失败、恢复文件后重新加载 | PASS；gpui-empty / read-error / read-retry-liveness-final.png；仅自有 state.json 临时目录故障，已恢复 |

React DOM 暖错误卡 x408/y56/w720/h75、margin-bottom12；紧随 Empty x264/y143/w1008/h180。GPUI root 的额外 gap8 已移除，保留 error margin12。弹窗 editing 白色 75%、auto94%、确认100%；lane header38px/mode10px/name36px 等对应源码和 DOM；advanced hover 使用 React neutral input 背景。

**完整视觉仍 OPEN**：现有实测支持上述局部状态、尺寸和实际操作，未证明整体达到 95%。React harness 不含完整原 App shell，不能用其全图评分证明 GPUI shell 一致；静态字体 fallback 的字重/度量、React backdrop blur 的 GPUI 技术差异保留，未声称完全相同。完整 focus/hover 逐控件同态测量仍不足。深色和其它缩放按用户要求留最终组合验收。

截图中原生标题栏不参与 React 对照；本轮没有删除 Legacy 参考。真实 UI Stop 后 child57744 已退出，再对自有 GUI apps 发送 SIGTERM；这不冒充 UI Quit 的完整退出验收。两个健康 peer 与 Vite 已停止，参考 viewport override 已 reset，自建 tab 已关闭，工作树临时 node_modules symlink 已移除；端口/进程清理见 cleanup-final.json。

## 保留状态与未完成项

正式 Runtime/受控 Native/列明 GUI 行为已接通且有证据；旧“未接 Runtime”缺口已消除。**P4-03 仍 ACCEPTANCE**，因为旧 Tauri Clippy 实际 FAIL 和完整视觉验收 OPEN；不把局部截图/构建成功替代全部验收。增量 Commit、Merge SHA、主分支复验、owner 释放均 NOT_RUN；当前 HEAD 仍为检查点，未 push、未解锁下游。任何继续修改 owner 持有范围仍按本次精确预约协调。

## Host 独立复核与代码检查点（2026-10-09）

Host 复查本分支基线2f8e20f，唯一Runtime入口、Failover Policy、Group/pending CAS/恢复、已授权 P2-06 owner 边界、Native 原始收据、P4-03 UI REWORK 与 React/GPUI 最终对照。独立执行的定向测试：Core ManualRuntime **45 PASS**、Groups **20 PASS**、Compiler **20 PASS**、Desktop Groups **6 PASS**、Components **12 PASS**、i18n **3 PASS**、Helper admission **1 PASS**，合计 **107 唯一 PASS，0 FAIL**。第一次 Helper 过滤名误写只命中0，已通过精确全名重跑1 PASS，0项未计入通过。Core/Desktop all-targets Clippy、Desktop build、Cargo fmt、git diff --check均通过。

受管 Native **1 PASS** 由本卡原始 `native-liveness-final.log/json` 确认，包含真实1.14.0、切备/切回、Auto/ManualPin、pending重新读回、Store重建和三组自有child退出；Host 未再次启动真实内核或操作系统网络。UI 证据 `compare-list-final`、`compare-add-final`、`compare-edit-primary`、`compare-edit-backup-advanced` 已逐张审阅，实际入口和编辑器几何收敛；完整95%视觉、细分 hover/focus、其余体系视觉验收仍 OPEN，不能转PASS。旧 Tauri Clippy因既有Windows libcronet.dll缺失FAIL，不属于本卡修复证据，不覆盖历史。

**P4-03 仍 ACCEPTANCE**，本地检查点提交后继续保留owner；不提升DONE、不合并 codex/dist-react-restore、不push。P2-05 ACCEPTANCE/P2-06 DOING和其原owner不变。下一步只需要在当前 macOS/浅色/显示缩放下补齐完整视觉签核或明确最小差异后再正式 DONE/合并；不再次开发已验收的主备 Runtime链路。
