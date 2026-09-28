# TASK-012 UI Contract supplement 005

Status: CANDIDATE — Human Design Approval PENDING

## 1. Impact Analysis 与权威关系

Change source：USER:lifei 2026-09-07 明确要求为 TASK-012 创建 Candidate-005，采用已接受的 UI Contract 0.2 / ui_stages / Golden Page Certification 基线；本轮仅设计及独立 Technical Design Review。

仅增加 UI delivery contract/stage/certification 约束。Candidate-005 JSON 是本补充的机器可读冻结目标，包含完整 UI scope、stage scope、source_paths 和 manifest；不复制或替代原业务设计。Candidate-004、原设计、DCR-004/017/018/019、change-control-001、历史 Design Approval/Delivery/Human acceptance 均按各自原身份保留。Candidate-004 的业务语义及已批准 Compiler override 不重新设计、不重新批准为本轮成果。

受影响：TASK-012 的新 UI Design binding、后续物化、视觉 checkpoint、UI acceptance 和当前 Delivery。SF-001/002 的功能完成事实及历史功能证据不失效；SF-003 的历史功能验收不等于 Contract 0.2 视觉批准。Task 继续 VERIFYING，本轮不改 Task/State、不给 Gate PASS、不进入 Certification 或实现。

无新的 L2 Material Change：不改架构、Domain、Application Service、IPC、StateStore、RuntimeIntent、Compiler、Pool/Route/持久化/运行语义，不新增依赖或框架。无数据 migration 或部署变化。后续表现层代码回滚不回滚业务数据；源码/共享样式变化会使对应视觉认证失效，须按实际新身份重新取证和人工验收。

原 Task 对 AppShell/Sidebar 的 deny 不在原文件中改写。本补充只把两页所依赖的 Shell/Sidebar 纳入视觉覆盖；获批后物化时须明确表现层的有限范围，继续禁止 Router、activePage + hidden、业务 state/handler 或结构性重构。共享 foundation 若未通过 Subscriptions certification，应停止并提出单独有界 remediation change unit，不能借 TASK-012 绕过 external prerequisite。

## 2. 真实实现与允许边界

| 视觉对象 | 当前承载位置 | 本补充约束 |
| --- | --- | --- |
| AppShell、Sidebar、导航 icon、Sidebar selected、global failure Toast | src/App.tsx | 作为两页共同视觉上下文覆盖；仅 presentation 可在合法 readiness 后按批准范围调整，事件、页面切换、observation/runtime handler 不动 |
| SidebarTraffic | src/components/layout/SidebarTraffic.tsx 与 SidebarTraffic.css | 纳入共享 manifest 和侧栏比例验证；采样、绘图算法、观测语义只读 |
| Page、PageHeader、Toolbar、Surface、Button/IconButton/Input/Select/Checkbox、字体/token | src/styles.css、src/main.tsx、src/assets/fonts/Twemoji.Mozilla.ttf、src-tauri/icons/128x128.png | 复用既有 class/token；main 入口、字体/logo 资产只读，不能将 source_paths 理解为写权限 |
| ProxiesPage、PageState、PoolList/PoolSection、NodeRow、AllNodes、PoolDialog、ContextMenu、Notice | src/components/proxies/ProxiesPage.tsx | PoolList/NodeRow 等是当前 JSX 的 composition 名称，不要求新建组件文件；只调整呈现和本地视觉反馈，不改 filter、validation、mutation 或 selector 事实 |
| RoutingPage、RoutingState、DefaultTargetRow、RouteList/RouteRow、RouteDialog、Notice | src/components/routing/RoutingPage.tsx | 保留真实 matcher、target、priority/reorder、enabled 和 Save/Apply 操作；复用 A 的密度与选中语言 |
| ProxyRoutingProvider、notice 内容/来源、刷新、CAS 和 snapshot | src/lib/proxy-routing.ts | 纳入状态来源 manifest；只读业务基线，不移动 state ownership，不改 parser/encoder/query/mutation/invalidation；成功反馈定时隐藏只属页面 presentation，不能抹去 dirty/error 或改变 authoritative snapshot |
| 直接消费的状态/格式依赖 | src/lib/bootstrap.ts、observability.ts、traffic-trend.ts、subscriptions.ts | 两页共享宿主/Provider 的只读来源，纳入 source_paths，不授权业务变更 |

source_paths 是页面交付身份的完整直接相关源码/资产清单，A/B 都包含共享依赖；Task 清单必须精确等于两阶段并集。生产代码新增路径或跨出上述边界必须先停止，不能靠漏列共享文件维持认证。

## 3. Contract 映射与最小视觉目标

引用唯一 docs/ui/veyra-ui-spec.md Version 0.2 / APPROVED / BINDING。规则定位以 JSON 的 rules 为准，不在此重新定义数值。

- §4：复用 :root 与 dark override 的 background/text/accent/divider/hover/selected、geometry、typography、control、radius token。将既有 token 映射至批准值；确实缺失的语义角色只能在 src/styles.css 声明 Contract 已定义的值，不能引入新设计值或局部同义 magic value。UNKNOWN 不补值，无新增自创 token。
- §5–6：同一操作区最多一个 primary；数据行单一 selected language；Dialog 保留 Esc/focus trap/return focus/busy guard；Menu 保留键盘定位；成功 Notice 按 Contract 自动消失、无关闭按钮，错误反馈与手动恢复保留。普通 Surface 和 Popup 不增加阴影、渐变、blur 或大圆角。
- Stage A / §8.3、§9、§14：full Page、紧凑 Pool Section/48–52px header、40–44px NodeRow；All Nodes 使用 dense list 或 220px min compact grid/48–54px item。All Nodes compact grid 是 §8.3 明确专用值，不把它误作普通 Row 的 §9 上限。正常状态无“已保存”Badge；dirty、pending、不可用和错误保持准确可见，不以颜色替代事实。
- Stage B / §8.4：default target 与规则 Section/Row 复用 A 的 dense language，不发明字段、规则组合或另一套 selection。当前 Routing 无 Pool expand/collapse 和 All Nodes，这三项仅在 B 的 state_exclusions 说明，不从 Task/A scope 删去。Routing selected 指真实导航/选择控件状态，不新增 Route 选择业务。

参考映射：Clash Verge Rev 的 src/pages/_navigation.tsx、src/components/base/base-page.tsx → Shell/Page；src/components/proxy/proxy-groups.tsx、proxy-item.tsx、proxy-item-mini.tsx → Pool/Node 密度；src/components/layout/notice-manager.tsx → 反馈。路径已定位于本地 E:/wx_lifeilin/github.com/clash-verge-rev。参考只帮助理解 composition，不复制 Mihomo 模型、API 或源码；数值以 Veyra 0.2 为准，Routing 使用 Veyra 原设计字段。

## 4. 有序 Gate 与证据

1. 本 Candidate design validator + 独立 Technical Design Review → 等待本身份的真实 Human Design Approval。Review PASS 不等于 Design Gate PASS。
2. 正式批准后由现有流程合法物化：Task 保留原功能/历史段落，只绑定新 Candidate、批准引用和完全相同的 ui_contract/ui_scope/visual_gate/ui_stages；不得提前把 Task 标 READY/DONE。
3. 在只读 scope 中检查 Subscriptions 当前实现：通过前端验证、源码/token review、真实 Windows Tauri light/dark + DPI/state 截图、独立 Review，取得真实人工视觉批准后建立 standalone certification。JSON 中 external certification_ref 是未来证据地址，不声称已存在。若不符合 Contract，Certification FAIL，停止并提出正常 Task/Design 管理的有界 UI remediation unit；不改 TASK-011、不直接修 UI。
4. 候选批准、物化与 Subscriptions 认证均成立后才执行 TASK-012 readiness；只要求 external Subscriptions，不要求未来 Proxies。
5. A checkpoint → 必要且已授权的 Proxies presentation 实现 → 机器检查/独立 review evidence → compliance PASS → Human Visual Approval → A golden_page_certification。
6. B checkpoint 必须验证 A 当前 human/hash；然后 Routing presentation → 机器检查/独立 review → compliance PASS → Human Visual Approval → B ui_stage_acceptance。
7. Task-level compliance 与最终 Human Visual Approval → 独立 Delivery Review。所有适用 stage 和整体批准都必须当前有效，才可允许 Task 最终收口。

每阶段 envelope 按现有 veyra-ui-gate.md：page、task_id+stage_id、Contract/version、完整 ui_scope、capture-time Git、source manifest 与 target_identity、compliance/hash、human/hash；A prerequisites 绑定 Subscriptions certification/hash，B 绑定 A/hash。compliance 和 independent review 同时绑定 certification_kind 与 prerequisites；人工批准绑定 compliance hash 和实际截图。认证不回读或改写原 Feature Task。

共享文件变化会使 Subscriptions/A/B 中相关认证失效。此时停止 dependent stage，重新检查实际来源和所需批准；若要更换冻结引用，走现有影响分析/评审/批准再物化，不改写历史 Evidence 或偷偷替换已批准 Candidate 的引用。

## 5. 验证与停止

当前设计阶段只运行 design validator、manifest/历史 hash/scope 并集检查、独立语义 Review、git diff --check；不运行应用或以旧测试报告充当当前 UI Evidence。

后续每阶段必须真实运行 pnpm lint（含 tsc --noEmit）、pnpm test、pnpm build，保留与新交付身份关联的结果。定向交互回归覆盖当前 ProxyRouting 页面/Provider，保留所有业务断言，不重跑未受影响 Rust/Compiler/Domain 实现。

每页真实 Windows Tauri 验证 light/dark normal 与全部适用状态：hover/pressed/selected/disabled/loading/empty/error/busy/focus/success、dirty/applied、Dialog/validation error/conflict；A 另含 expanded/collapsed/all-nodes/context-menu/inline-pending 和 Manual/UrlTest 呈现，B 含 default-target/Route CRUD/reorder 呈现。状态来源必须真实，不把 mockIPC 截图或静态稿当原生证据。

两页均验证 1280×720@100%、1440×900@100%、1280×720@125%、1440×900@125%、1280×720@150%，记录实际窗口/DPI、主题、状态、截图/hash/target；检查 overflow、按钮重叠、Header、Sidebar、文字、Dialog 和 Toast。原 520/960 历史结果保留原身份，不替代新版矩阵；受影响时保留原窄窗口回归义务。

任何新 Material Change、Candidate-004 业务冲突、Subscriptions 不合规、需要改 TASK-011/Compiler/Domain/Runtime、需要 Human Approval 或超出批准路径的源码改动均停止。机器与 Reviewer 不代判视觉品质。本轮终点是 Human Design Approval PENDING，不提前执行上述第 2–7 步。
