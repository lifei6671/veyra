# Proxies / Routing UX 与 Rule Set：有界影响分析

日期：2026-09-08。来源：USER:lifei 对 target-018 的 Human Visual Review，结论 REWORK。
状态：Impact Analysis 已记录；产品方向明确，详细 Requirement clarification / Design 尚未冻结；无实现授权。
关联：`.sdlc/evidence/TASK-018/human-visual-findings-038.json`、`docs/decisions/DCR-026-proxy-routing-ux-and-rule-set.md`。

## 当前事实与权限

- TASK-018 保留 VERIFYING，当前 Human Visual Review 为 REWORK，Delivery FAILED；focus 不迁移。target-018、Toast、Subscription transition、所有历史 Evidence 保留原身份，不转换为 APPROVED。
- 本轮仅记录 Findings、Impact、后续 planning stub 与设计入口；TASK-018 的历史生产源码修改权限暂停。不得借 shared foundation 修补整页 redesign、排序、auto apply、Toast lifetime 或 Rule Set。
- Requirement Source `docs/veyra.md` SHA256 仍为 `93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4`。当前 UI Contract0.3 不在本轮改写；新产品方向通过本记录追踪，待对应任务正式修订冲突条款。
- 当前代码依据：`src/components/proxies/ProxiesPage.tsx`、`src/components/routing/RoutingPage.tsx`、`src/lib/proxy-routing.ts:174` 的 mutation 编码和 `:234` 附近的 Provider；`useToastOwner("proxy-routing", null)` 明确为无自动过期。`TrafficMatcher` 位于 `src-tauri/src/domain/state.rs:855`，仅六种手工匹配器；`src-tauri/src/storage/store.rs:131` StoredStateV6 只有 routes，无 Rule Set；`src-tauri/src/singbox/compiler.rs:479` RouteConfig 只有 rules/final/default_domain_resolver。

## TASK-019 — Proxies / Routing Desktop UX Redesign

已确认需求：

- Proxies 采用 compact section；header 仅名称、类型、当前节点和必要操作；主要内容为紧凑自适应节点 grid/card。轻量 selected，减少大白卡嵌套与留白，优先扫描效率和桌面工具密度。“全部节点”改为 tab、secondary section 或 collapsed view，具体选型在 Design 比较后冻结，不保留大面积重复区块。
- Routing 使用 compact rows，移除每行常驻 ↑/↓。hover/focus 时左侧出现六点 drag handle；拖动有轻量 lifted state 与 insertion indicator；drop 提交现有 `reorderRoutes(routeIds)`。后端 priority、ID 全量排序和业务语义不变。
- structural create/update/delete/reorder 持久化成功后自动触发 Runtime Apply。Desired / Applied 内部边界保留；只有确认应用完成后显示“规则已保存并应用”；失败显示“规则已保存，但应用失败”及“重试应用”。不回滚已保存 Desired；保存失败不得调用 apply。
- 正常状态去掉 Proxies/Routing Header 常驻“应用”；仅 apply failure / unresolved desired-applied drift 展示恢复入口或状态。应用中提供忙碌反馈，不提前宣称成功。现有手动节点选择的运行时确认语义不得被盲目套用 structural apply。
- ordinary success 短时自动消失；需用户处理的 warning/error 可以持久；persistent error 必须有明确 Recovery Action。Contract0.3 §6.7 与既存 duration discrepancy 在本任务正式解决，数值及 owner 分类待冻结，不能仅修改 CSS 或设置一个全局 timer。

参考映射（已定位文件，具体布局、token 与浅深值在 Design 阶段提取）：

| Veyra concern | Clash Verge Rev 本地参考 |
| --- | --- |
| Proxies section / header / nodes | `E:/wx_lifeilin/github.com/clash-verge-rev/src/pages/proxies.tsx`、`src/components/proxy/proxy-groups.tsx`、`proxy-head.tsx`、`proxy-render.tsx`、`proxy-item.tsx`、`proxy-item-mini.tsx` |
| Routing 密度与规则集入口 | 同仓库 `src/pages/rules.tsx`、`src/components/rule/rule-item.tsx`、`provider-button.tsx` |

必要差异：使用 Veyra Pool、节点稳定 ID、sing-box Route 与未来 RuleSet，不复制 Mihomo rule-provider 配置/API；drag reorder 是用户要求，不以参考项目是否支持作为前置。

影响边界：前述两页、`src/styles.css` 对应页面样式、`src/lib/proxy-routing.ts` mutation/presentation、ToastHost/owner 的 action 和 duration 接口及其测试。auto apply 需要审阅 `src-tauri/src/application/proxy_routing.rs` 与现有 runtime apply/operationId/revision 结果链；优先复用已有业务入口，是否需要最小应用层编排变更由独立 Design 决定，不预先许可后端改写。不包含 Rule Set schema、通用状态机、新通知总线或整站重设计。

Design 必须闭合的决策：

1. 停止、无 active subscription、recoveryRequired 时的自动应用结果：是否启动内核会改变用户运行意图，不能默认为自动启动。须明确产品语义，再冻结状态表。
2. 保存返回后的新 revision 如何传递给 apply；连续修改、切页、重复 drop、applyUnknown、响应丢失如何复用现有 pending/operationId/refresh 机制。已保存但结果未知不能标为“保存失败”，也不能盲目重放 mutation。
3. 默认出口、Pool enable/disable 等 structural 操作纳入清单；selector 与订阅 activation 保持独立。重试只应用当前 Desired，不重复 CRUD；只有对应 generation 的运行态确认才能结束成功反馈。
4. drag 的键盘拾取/移动/drop/Escape、focus 恢复、取消时无提交、busy 时禁用，以及列表已变更时的冲突反馈；不恢复常驻箭头作为替代。
5. Toast kind/owner/duration/Recovery Action 矩阵、普通成功的具体时长、持久错误解除条件；无新自动重试后台任务。

后续验收建议：两页真实操作与 Light/Dark、Contract DPI/窗口矩阵；grid 扫描与长名称、空/加载/错误/忙碌；drag 提交准确 ID 顺序且取消不提交；保存失败不 apply、保存成功自动 apply、apply 失败保留 Desired、重试仅 apply、unknown 不冒充成功；成功提示自动过期与持久错误 recovery。使用现有测试工具，最小真实 config/check/run/stop，不重认证 sing-box 内核能力。

## TASK-020 — sing-box Rule Set Support

已确认范围：Domain、AppState/schema compatibility、IPC、Compiler、CRUD、inline/local/remote、Route Rule 引用、UI、tests。规则与规则集是两种管理能力；Routing 推荐 `[规则] [规则集]`，规则编辑支持“手工匹配条件”与“引用 Rule Set”。

候选建模方向（尚未冻结）：独立稳定 RuleSetId 与唯一 tag，类型化 inline/local/remote 数据；Route 以稳定 ID 引用，在 Compiler 投影为 tag。inline 保存受控 headless 条件；local/remote 有 source/binary format，分别有 local path / remote URL，remote 有 update interval。禁止通过任意 JSON passthrough 绕过 Domain/Compiler。

影响链：Domain/AppState → storage DTO/migration/validation → application CRUD/reference checks → IPC DTO/strict parsing → runtime intent → Compiler `route.rules` 引用和 `route.rule_set` 定义 → Routing UI 与可观察 lifecycle。必须检查 `src-tauri/src/application/selected_subscription.rs` 的运行意图投影，避免状态有 RuleSet 而编译输入丢失。扩展前端 strict snapshot/mutation parser 的同时同步测试与 fixtures。

上游依据：[sing-box 官方 Rule Set 文档](https://sing-box.sagernet.org/configuration/rule-set/)（2026-09-08 查询）：支持 inline/local/remote；inline 使用 headless rules，local/remote 使用 source/binary；remote 定义 URL 和 update_interval，缓存与下载选项依内核配置。设计必须针对项目固定 sing-box 1.14.0 核对，不能沿用旧 download_detour 假定。此处是接口依据，不是已运行的内核验证。

Design 必须闭合的决策：

1. inline 条件支持边界、单/多 RuleSet 引用与 AND/OR 语义、引用缺失、重命名和被引用删除策略；不自动级联删除用户规则。
2. local path 的相对基准、原文件引用还是托管副本、移动/缺失/无权限与重启恢复；source/binary 校验与有界错误反馈。
3. remote 首次下载、周期更新、手动更新/重试、取消/关闭、离线/超时/无效内容、最后有效版本、缓存和文件清理，以及 URL 更新与旧缓存关系。明确 Veyra 与 sing-box 的下载/更新责任，不能让两者同时维护竞争调度；尚不批准新增 downloader/service。
4. 更新状态的真实来源及 UI 能否可靠显示成功/失败，不以 CRUD 保存成功冒充下载或应用成功；远端 URL/日志沿用凭据脱敏边界。
5. schema 显式迁移、旧 routes 保留、原子写入/升级备份/损坏恢复/降级策略。当前 StoredStateV6 与 TASK-013 规划的 V7 存在序号冲突风险：以实施时真实 schema 为准，不预占版本，不用 serde default 隐式掩盖迁移。
6. compiler 的定义去重、引用解析、受控投影与最终校验；保持既有路由顺序、置顶拒绝和默认出口语义。TASK-019 auto apply 契约在此复用，不重新发明第二条应用路径。

后续验收建议：三类 CRUD/持久化 round-trip，旧状态迁移保留 routes，引用/删除/重命名一致性，strict IPC 正反样例，rules 与 rule_set 投影断言；受控本地资源和测试 HTTP server 验证 Veyra 负责的 lifecycle/错误；各类代表配置用固定内核 check 和最小加载验证。UI 两 tab、手工/引用编辑、下载/更新/失败 recovery 必须真实验证；Mock 不等于原生结果。

## Gate、兼容与下一步

- TASK-018 Human Visual Review REWORK；当前 aggregate Delivery FAILED，人工 Task acceptance 保持 PENDING。历史 source/native/compliance PASS 仅证明原目标对应范围，不删除、不改成 FAIL，也不能覆盖新人工否决。Candidate-011 的技术批准仅对原 bounded scope 保留，不授权新方向。
- TASK-012 既有验收和 VERIFYING 状态保留，本轮不授予 readiness。其旧手动 Apply/页面设计不能作为 TASK-019/020 新需求的 Design、QA、Delivery 或 Golden 证据；后续须对受影响部分重新评审。未运行的新任务 Design/QA/实现验证均 NOT_RUN。
- 后续顺序建议：先完成本 impact 后的 Requirement clarification → 正式 Contract/DCR 修订与独立 Design Review/所需人工批准 → Task materialization/readiness → 实现/测试/独立 Delivery Review/真实 Human Visual Approval。本轮只加入 future stubs，不提前建立完整 Task 或启动实现。
- TASK-019/020 的排期与 TASK-018 收束是独立决定：拆出 Findings 不等于 TASK-018 已验收；没有授权不得自动关闭 TASK-018 或切换 focus。TASK-020 复用 TASK-019 的 UX/apply 结果；与 TASK-013 的 schema 顺序在物化前协调。
- 本轮无部署、运行实例、网络配置、依赖或数据操作；生产源码与原 Evidence 不变。后续 auto apply 改变运行时触发，Rule Set 涉及外部资源和迁移，不能视作 CSS 修补。
