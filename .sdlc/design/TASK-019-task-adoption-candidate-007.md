---
id: TASK-019
milestone_ref: M7
dependencies: [TASK-018]
risk: HIGH
status: BLOCKED
requirement_ref: docs/veyra.md
requirement_identity: sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4
design_candidate_ref: .sdlc/design/TASK-019-candidate-007.json
design_candidate_identity: sha256:195350aec5a38be8d7315fb9906ee25e8a13d33962b54ce12733b27998a05b91
design_refs:
  - .sdlc/design/TASK-019-stage-s-certification-scope-007.md
  - docs/decisions/DCR-026-proxy-routing-ux-and-rule-set-revision-009.md
  - .sdlc/design/TASK-019-stage-s-local-notice-006.md
  - docs/decisions/DCR-026-proxy-routing-ux-and-rule-set-revision-008.md
  - .sdlc/design/TASK-019-stage-s-scope-remediation-005.md
  - docs/decisions/DCR-026-proxy-routing-ux-and-rule-set-revision-007.md
  - .sdlc/design/TASK-019-publication-adoption-004.md
  - docs/decisions/DCR-026-proxy-routing-ux-and-rule-set-revision-006.md
  - .sdlc/design/TASK-019-requirements-002.md
  - .sdlc/design/TASK-019-desktop-ui-design-003.md
  - .sdlc/design/TASK-019-auto-apply-design-002.md
  - .sdlc/design/TASK-019-technical-design-003.md
  - .sdlc/design/TASK-019-stage-plan-003.md
  - .sdlc/design/TASK-019-ui-contract-0.4-candidate-002.md
  - .sdlc/design/TASK-019-ui-contract-delta-002.diff
  - docs/ui/veyra-ui-spec.md
approval_provenance_refs:
  - .sdlc/evidence/TASK-019/design-review-003.json
  - .sdlc/evidence/TASK-019/human-design-approval-003.json
decision_evidence_refs:
  - .sdlc/evidence/TASK-019/contract-publication-004.json
  - .sdlc/evidence/TASK-019/stage-s-design-authorization-003.json
  - .sdlc/evidence/TASK-019/subscriptions-prerequisite-check-001.json
  - .sdlc/evidence/TASK-018/acceptance-scope-binding-039.json
  - .sdlc/evidence/TASK-018/task-acceptance-042.json
  - .sdlc/evidence/TASK-018/delivery-completion-042.json
---

# TASK-019：Proxies / Routing Desktop UX Redesign

## Adoption 与 Readiness

Proposed exact Task adoption for Candidate007. Canonical Task remains adopted Candidate006 until new exact Human Design/Adoption Approval. Certification-scope007 / DCR026revision009 removes only Stage S toast-persistent; TASK-level and Stage A/B retain it. Product write scope is inherited unchanged and is not reopened by this design-only request. Readiness/checkpoint run only after byte-for-byte adoption; no product/test/Contract/evaluator change or Native capture this turn. Existing target018 evidence requires explicit independent applicability confirmation; no old FAIL or incomplete coverage becomes PASS.

## Objective

按已批准 R01–R10 交付 compact Proxies、compact Routing、可访问 drag reorder、可信运行时边界内的
structural save→auto Apply→recovery，以及全局 ordinary success 3000ms/no-close 与 persistent Recovery Action。
保持现有 Pool/Route/priority/稳定 ID、Desired/Applied、Manual selector、Subscription activation、owned runtime
和失败清理语义；按 S→A→B 建立当前 Subscriptions 与 Proxies Golden，再完成 Routing 和 Task 整体验收。

## Scope

### allow

- src/components/proxies/ProxiesPage.tsx: compact sections/tabs/node grid only
- src/components/routing/RoutingPage.tsx: compact rows/drag and recovery only
- src/styles.css: Proxies/Routing/Toast scoped styles; Stage S Subscription Dialog width declarations and the exact direct-child local Notice override in design006 only
- src/lib/proxy-routing.ts: structural receipt/autoApply feedback/strict parsing/retryCurrentDesired
- src/components/ToastHost.tsx: success3000/noClose, typed actions, owner-scoped replace-clear
- src/App.tsx: recovery navigation callback to existing setActivePage only
- src-tauri/src/application/proxy_routing.rs: post-save disposition/retry union/attempt snapshot projection and inline tests
- src-tauri/src/application/managed_observation_runtime.rs: Ready-only guard/volatile latest slot/coalesced marker/attempt identity and inline tests
- src/lib/proxy-routing.test.ts
- src/components/proxy-routing-pages.test.ts
- src/components/ToastHost.test.ts
- src/components/subscriptions/DocumentEditor.css: .document-dialog border 1px solid var(--divider) only
- src/components/subscriptions/SubscriptionPage.tsx: ordinary/complex dialog class selection only
- src/components/subscriptions/SubscriptionPage.test.ts: existing Dialog regressions plus design006 local/global Notice presentation isolation regressions only
- scripts/sdlc-ui-contract.test.mjs: synthetic version0.4 and old0.3 wrong-version fixture alignment only
- Stage S: existing Toast Foundation plus precisely bounded Dialog border/width and fixture remediation; complete Subscriptions Golden certification remains required; design006 adds only local Notice CSS/test scope; all other Subscription page/editor/business source readonly.

### deny

- No implementation under this design authorization; exact Candidate006 Human Design/Adoption Approval, future implementation authorization and affected readiness/Stage S checkpoint required
- No Domain/AppState/schema/migration/Compiler/RuleSet/reorder backend semantic change
- No new dependency/worker/durable queue/background retry
- No changes to historical TASK018 Evidence/findings or TASK020 materialization
- No fabricated Golden, mutable certification alias or protocol change; old0.2 FAIL preserved as provenance; valid internal S must gate A
- Stage S remediation: no Subscription handlers/fields/CRUD/persistence/activation, DocumentEditor.tsx, subscription IPC/runtime, evaluator semantics, Proxies/Routing/Rust or other shared layout changes
- Change006 does not reopen historical implementation allowances: only proposed src/styles.css direct-child local Notice override and focused SubscriptionPage.test.ts tests; no markup/class, DocumentEditor, Proxies/Routing/Rust, Contract0.4/R01-R10 or Toast lifecycle changes.

## Approved requirements R01–R10

- R01：changed structural save 仅在 authoritative active subscription 有效且 owned runtime 实际 Ready 时自动 Apply；可信检查在 runtime 调度边界完成。
- R02：Stopped 只保存 Desired，不启动内核；ordinary success 为“已保存，将在下次启动时应用”。
- R03：无有效 active subscription 只保存、不 Apply；有效性由 authoritative state 与受管运行意图解析判定。
- R04：RecoveryRequired 只保存并给出 persistent recovery feedback，不后台恢复或启动。
- R05：连续 structural mutation 由 latest authoritative Desired/generation 获胜；旧 operation completion 不覆盖新 generation，CRUD 不排队、不重放。
- R06：Retry Apply 先读取当前 authoritative Desired，只 Apply，不重放 create/update/delete/reorder。
- R07：Routing 使用现有 React Pointer/Keyboard Events；不新增 DnD runtime dependency。
- R08：Proxies 使用“出口组 / 全部节点”互斥 tabs，默认出口组，不再渲染页面底部重复 All Nodes 大区块。
- R09：global ordinary success Toast 精确 3000ms、无关闭按钮；需用户操作的 warning/error 可 persistent。
- R10：apply failure persistent Toast 必须提供“重试应用”，且只调用 retry-current-Desired。

Structural 集合精确为 create/update/delete custom Pool（update 含 enable/disable）、setDefaultTarget、
create/update/delete Route（update 含 enable/disable）和 reorderRoutes。Manual selector 与 Subscription activation
排除并保留既有路径。no-op reorder 不提交；其它 no-op 不产生新 generation 或 Apply。

## Stage plan：S → A → B

### Stage S：Subscriptions Golden producer

Certification-scope007 supersedes the earlier S-only toast-persistent applicability: remove that single Native state, retain Toast/RecoveryAction/§6.7 and shared primitive automation/source review. All other states, source paths, components, rules and Windows screenshot/Human requirements remain. Stage A/B and TASK-level persistent obligations stay unchanged.

Stage S requires [] and produces Subscriptions, reserved .sdlc/evidence/TASK-019/ui-stages/S/certification-001.json remains absent until actual certification. Candidate005 keeps the full Stage S scope and adds only DocumentEditor border, Subscription dialog mode classes/widths and corresponding tests plus Contract fixture alignment described in scope-remediation005. Design006 adds only the exact Dialog-local Notice CSS override and focused isolation tests; no other historical source work is reopened by this request. All other Subscription page/editor/activation/business sources remain readonly. Current Toast implementation is preserved; no stage implementation/certification PASS is implied by adoption.

Remediation verification: Dialog/Subscription targeted tests, all ToastHost tests, pnpm lint, full pnpm test, pnpm build, git diff --check; full tests must be green. Complete Stage S Native additionally covers all six modes, DocumentEditor bordered popup and Esc/focus trap/return/busy/fullscreen plus Toast expiry focus. Current source identity must be regenerated after actual source changes.

S 验收必须覆盖 Light/Dark × 1280×720@100/125/150% 与 1440×900@100/125%，真实 Tauri 页面状态与构图、
activation pending→active、single emphasis、local validation/preload errors、focus/dialog/context-menu，以及
ordinary success 2999/3000、no-close、waiting expiry、safe positioning、downward stack、queue promotion 和 modal
layering。必须有 current Compliance、独立 Delivery Review、真实 Human Visual Approval 后才能形成 Golden。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

### Stage A：Proxies Golden producer

Stage A requires [Subscriptions]，produces Proxies，future evidence_ref 为
.sdlc/evidence/TASK-019/ui-stages/A/certification-001.json。只有当前 S certificate 有效后才能 checkpoint/实现；
完成 SF-019-001 及适用 structural/recovery 集成后，执行完整 Proxies Golden Compliance、独立 Review 与真实
Human Visual Approval。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

### Stage B：Routing acceptance

Stage B requires [Proxies]，future evidence_ref 为
.sdlc/evidence/TASK-019/ui-stages/B/acceptance-001.json。只有当前 A Golden 有效后才能 checkpoint/实现；
完成 SF-019-002 与适用 auto-apply/recovery 集成后，执行完整 Routing stage acceptance。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

S/A/B 的 evidence_ref 是保留的未来输出，不表示文件存在或 PASS。A/B 若修改共享 source 使上游认证 stale，
必须停止当前 target，按 stage-plan003 生成 append-only 新版本认证、Candidate/DCR/Task 引用采用、独立 Review
和必要 Human approval；不得覆盖 certification-001、使用 mutable alias、漏列共享路径或以旧 checkpoint 继续。

## SF-019-001：compact Proxies（TASK018-HV038-001）

需求：实现 R08。页面为 full Page → PageHeader → tablist → exactly one panel；tabs 使用
tablist/tab/tabpanel、aria-selected/controls、roving tabIndex，Left/Right/Home/End 切换并聚焦，mount 默认出口组，
不持久化 view preference，不因切 tab 重置 provider state。Pool header 48–52px，仅名称、类型、当前节点和现有
必要操作；node grid 为 repeat(auto-fill,minmax(min(220px,100%),1fr))，gap 8px，item candidate height 52px
并保持 Contract 48–54px。selected 只用预留槽中的 compact check glyph、正常 surface/text；hover 与 focus 独立。
全部节点复用同一 grid 和 stableNodeId，显示 provider attribution；没有明确 selector context 的节点只读，不新增
全局“使用节点”mutation或新性能数据。

Acceptance：出口组/全部节点互斥且无重复 All Nodes；expanded/collapsed、selected、Manual/UrlTest、长名称、
未知/不可用节点、loading/empty/error/busy/disabled/focus 均保持 authoritative truth、可访问、无横向溢出、
无大型 Dashboard Card。TASK018-HV038-001 仅在当前 Stage A target 的自动化、Compliance、独立 Review 与真实
Human Visual Approval 全部通过后关闭。

Verification：现有 Vitest/mockIPC 覆盖 tab keyboard/roving focus、stable IDs、两面板互斥、展开折叠、长名称、
状态互斥、Manual/UrlTest 回归和 no invented selector；使用既有 100 subscriptions/5000 nodes/100 pools fixture，
无实测必要性不引入 virtualization。真实 Windows Tauri 覆盖两主题、五组窗口/DPI、全部声明状态和操作，保存
source-bound screenshot/hash/parity，并完成 Proxies Golden Certification。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

## SF-019-002：compact Routing 与 accessible drag reorder（TASK018-HV038-002）

需求：实现 R07。移除常驻 ↑/↓ 与正常 Header Apply；row 保留 24px 左 handle slot，handle 是 aria-label
“移动规则 {name}”的真实 button，视觉仅在 row hover/focus-within 出现但始终可键盘 focus。Pointer 仅由 primary
button on handle 启动，4 CSS px threshold，capture pointer；provisional order 不替代 authoritative snapshot，
lifted item 保持原 row 尺寸，insertion indicator 为 2px primary，active drag 可使用 bounded RAF edge autoscroll，
所有 terminal/unmount 取消 RAF。Keyboard Space/Enter pickup/drop，Up/Down/Home/End 移动，Escape 取消，polite
live region 报告位置；Tab/focus leave、pointercancel/lostcapture/windowblur/unmount/modalopen、ordered IDs 或
revision 改变均取消且零提交。成功前 revalidate baseline snapshot/revision，结束 drag 后 exactly once 提交完整
stable ID order 到现有 reorderRoutes；same-position 不提交，conflict/busy refresh authoritative list，永不自动重放。

Acceptance：pointer 与 keyboard 产生同一准确顺序；取消/失焦/stale/busy/no-op 零 Desired/runtime 副作用；
duplicate pointerup/Enter 不双提交；focus 按 stable ID 回 handle，删除目标回最近 row 或 toolbar。Light/Dark 与
五组窗口/DPI 中 handle、lifted、insertion、focus 与 live feedback 可辨认，Route CRUD/default target/priority
后端语义不变。TASK018-HV038-002 仅在 Stage B 当前 target 全部验收后关闭。

Verification：production handler Vitest 覆盖 pointer/keyboard exact IDs、threshold、cancel terminals、no-op、
double terminal、conflict refresh、revision/order drift、focus return、RAF cleanup、failure recovery；真实 Windows
Tauri 执行 pointer/keyboard drag/cancel/conflict/busy/focus 和主题/DPI矩阵，由独立 Reviewer 核对 Proxies Golden
dense language 与 no reorder semantic change。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

## SF-019-003：structural Save → Ready-only auto Apply → recovery（TASK018-HV038-003）

需求：实现 R01–R06。Changed structural mutation store 成功是不可回滚的 saved fact；save failure 零 runtime call。
post-save 优先级为 recoveryRequired → no effective active subscription → Stopped → non-Ready/other busy →
Ready。前三者与 other busy 只返回 saved disposition/pending-drift，绝不构造、启动或恢复 child；Ready-only
admission 在 runtime busy owner 与实际 lifecycle guard 内完成。Ready 才 dispatch exact tuple 的既有 forced Apply，
只有 matching operationId + activeSubscriptionId + desiredGeneration 且 actual Ready/applied tuple 匹配才可 Applied。

older routing Apply busy 时，在同一短 routing_admission critical section 内维护 one volatile overwrite-only
pendingLatest tuple 与 one coalesced wake marker；三次后续 save 只收敛最新 tuple，不保留 CRUD。owner 发布/释放、
busy 转换、pending/wake 决策使用同一 admission mutex；不持锁跨 StateAccessGate、subscription guard、compile/check/
sidecar/channel wait。worker 在 owner 结束（含错误）后 drain；queue full/idle wake/Stop/shutdown/旧 lease/新 token
均按 auto-apply-design002 原子规则处理。failure/unknown/stopped/no-active/recovery/tuple mismatch 清理已消费 intent
且不自动 retry；新 save 是新 intent。Stop 先取得 owner 时占优且 marker 不启动；Apply 先取得时 Stop 返回既有 Busy。

Changed save 返回精确 saved receipt 与 apply disposition；retryCurrentDesired 使用现有 expectedRevision envelope，
后端重读 current tuple 并走同一 Ready-only admission。lost mutation response 只 authoritative refresh，不重放 CRUD；
lost Apply response 仅由 matching observation 收敛，否则 unknown。旧 operation 不清 current tuple feedback，Desired
永不回滚；process loss 后 volatile slot 丢失并暴露 drift，等待用户 retry 或下次 Start。

Acceptance：saveFailed 保留旧 revision/generation 且零 Apply；noActive/runtimeStopped/recoveryRequired/runtimeBusy
准确返回 savedPendingApply；Ready 返回 started/completed/failed/unknown 的精确 operation identity；matching Ready
才发当前成功。old completion before/after newest save、three-save coalescing、publication-before/after-release、
queue-full、idle-wake、Stop-before-marker、Apply-before-Stop、shutdown、response loss、snapshot null 均不产生
stale success、重复 Apply、CRUD replay、hidden Start 或无限 pending。TASK018-HV038-003 仅在当前 target 自动化、
最小受控 native Apply、独立 Review 与 recovery chain 验收后关闭。

Verification：运行 pnpm exec vitest run src/lib/proxy-routing.test.ts；Rust 精确非零 suites：
cargo test --manifest-path src-tauri/Cargo.toml application::proxy_routing::tests:: 与
cargo test --manifest-path src-tauri/Cargo.toml application::managed_observation_runtime::tests::routing_apply。
覆盖完整 post-save table、严格 saved/retry union、safe integer/null、response order、guard release 的 success/error/drop、
latest tuple/wake/lease/token races、old operation Toast ownership。Rust 实际改动后运行 fmt、clippy -D warnings；
固定 sing-box 1.14.0 仅做最小 check/run/API/stop，验证 Veyra orchestration、owned child 清理与脱敏失败传播。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

## SF-019-004：Toast 3000ms 与 Recovery Action（TASK018-HV038-004）

需求：实现 R09–R10。ToastHost 对所有 global ordinary success 以 original enqueue time 精确 3000ms 自动过期，
无 close；保留 capacity3、FIFO、append-below、item identity、waiting enqueue-based expiry、fixed safe inset、
pointer/modal layering。warning/error 可 persistent。typed local action 不跨 IPC 序列化 callback；ProxyRouting
recovery 使用 owner/current-tuple scoped replace/clear，旧 completion 不清新 error。apply failure 的“重试应用”
只调用 retryCurrentDesired，不携带旧 CRUD/revision/payload；running 时 action disabled 并显示 applying，double click
exactly once。blocked runtime 给出说明且零 Start；secondary“查看运行状态”只通过 App.tsx 现有 setActivePage 导航。
业务恢复状态来自 authoritative failure/unknown/drift，Toast dismissal 不解决业务状态；modal submit/validation error
保持 local。移除 focused Toast 后 focus 返回触发控件或 page recovery entry。

Acceptance：ordinary success 在 2999ms 仍存在、3000ms 移除，无 close；waiting success 以原 enqueue 时刻到期；
persistent warning/error 不自动丢失并有可访问 action；owner replace/clear 不影响其它 Toast，stale result 不清 current
feedback。SavedStopped success 文案与3000ms正确；noActive/recovery/applyFailure/unknown/drift 按批准状态 persistent；
applying 不提前成功；Routing 使用“规则已保存并应用”，Pool/default 使用“出口设置已保存并应用”。Stage S 的完整
Subscriptions Golden 和 A/B feedback 均通过后，TASK018-HV038-004 才关闭。

Verification：ToastHost fake clock 覆盖 2999/3000、waiting expiry、capacity/FIFO/queue promotion、success no-close、
persistent action exactly-once、disabled/double click、owner replace/clear、unmount cleanup、focus return、modal
layering 与 cross-owner regression；proxy-routing integration 覆盖 current tuple、retry-current-Desired、navigation
callback 和文案矩阵。Stage S 及 A/B 真实 Tauri 覆盖 Light/Dark、五组窗口/DPI、1/2/3 stack、safe positioning、
主要操作无遮挡与 action focus；Mock/build 不替代 current native/human Evidence。

implementation_status: PENDING
acceptance_status: PENDING
evidence_status: NOT_RUN

## Task 独立验收

先完成 S 当前 Subscriptions Golden，再完成 A 当前 Proxies Golden，最后完成 B Routing acceptance；任何共享 source
变化按 stage-plan003 重新建立并正式采用受影响上游认证。最终同一隔离 AppState / Windows Tauri 进程至少使用两个
订阅、多个 Pool/Route，完成 Subscriptions ordinary success 与关键页面状态认证、Proxies 双 tabs/selected、
Routing pointer+keyboard reorder、全部 structural mutation、Stopped/noActive/recovery/other-busy/Ready、
save failure、Apply Ready/failure/unknown/response loss、latest-generation convergence、persistent retry-current-
Desired、Toast expiry/queue/action、跨页 authoritative readback 与 restart drift/recovery。不得操作用户真实实例。

对账 Candidate007、Contract0.4 hash、R01–R10、完整 allow/deny、四项 OPEN Finding、S/A/B 当前 immutable Evidence、
统一 implementation target 和所有 source manifests。运行 pnpm lint、相关 targeted/full Vitest、pnpm build、两个
Rust 精确非零 suites、cargo fmt --check、cargo clippy --lib -- -D warnings 与 git diff --check。只对 Veyra 编排做
固定 sing-box 最小集成，不重认证内核协议。最终必须有 current S/A/B、整体 UI Compliance、覆盖 frontend/UI、
strict IPC、runtime concurrency/lifecycle、failure propagation 与 integration 的独立完整 Delivery Review、真实
final Human Visual Approval 及 Task-level acceptance；历史 TASK012/018 或旧 Golden Evidence 不替代。

acceptance_status: PENDING
evidence_status: NOT_RUN

## Dependencies 与 Risk

- Candidate007 proposes only removal of Stage S toast-persistent after exact adoption; overall and A/B requirements remain. Full Native certification stays incomplete; no recovery product change, Gate exception or Native PASS.

- TASK-018 已 DONE；acceptance042/completion042 只解锁 TASK-019，不提供本 target 实现或验收。
- Candidate006 / Approval017 / entry-gates017 remain adopted provenance. Candidate007 requires exact Human Design/Adoption Approval, byte-for-byte snapshot adoption and affected readiness/Stage S checkpoint. Current target018 implementation authorization018 is historical authority for its scope, not approval of this new certification metadata.
- S→A→B 是硬顺序；current upstream Golden stale 时停止，不能让下游 checkpoint 继续。
- TASK-020 依赖 TASK-019，Rule Set、Domain/schema/migration/Compiler 新能力均不在本 Task。

HIGH risk 包括：Ready-only admission 改变运行时触发；routing_admission owner/pending/wake/lease/token 的锁和生命周期
竞态；save fact 与 Apply failure/unknown 的失败传播；pointer/keyboard drag 的取消、focus、exact IDs；global
success policy 对 Subscriptions consumer 和上游 Golden 的影响。缓解方式由已批准设计固定为：同一短 admission
临界区、volatile overwrite-only latest tuple/coalesced marker、current tuple/operation identity、零 CRUD replay/
background retry/hidden Start、strict union、deterministic double-order/guard-drop tests、S→A→B current native/
independent/human certification。剩余风险是 process/response loss 或 consumed intent 非 Ready 后 Desired 保持 drift，
直到显式 retry-current-Desired 或下一次用户 Start；该风险必须可见，不得伪造成功。

## UI Delivery Metadata

```json
{
  "task_id": "TASK-019",
  "ui_impact": {
    "required": true,
    "reason": "Shared ordinary success Toast affects Subscriptions full Golden; Proxies and Routing page composition, drag, structural apply and recovery"
  },
  "ui_contract": {
    "document": "docs/ui/veyra-ui-spec.md",
    "version": "0.4",
    "required": true
  },
  "ui_scope": {
    "pages": [
      "Subscriptions",
      "Proxies",
      "Routing"
    ],
    "components": [
      "AppShell",
      "Sidebar",
      "NavigationIcon",
      "SidebarTraffic",
      "Page",
      "PageHeader",
      "Toolbar",
      "Surface",
      "SubscriptionPage",
      "Button",
      "IconButton",
      "Input",
      "Select",
      "Checkbox",
      "Dialog",
      "ContextMenu",
      "Notice",
      "Toast",
      "DocumentEditor",
      "SubscriptionCard",
      "RecoveryAction",
      "ProxyRoutingProvider",
      "ProxiesPage",
      "PoolSection",
      "NodeGrid",
      "NodeCard",
      "AllNodes",
      "RoutingPage",
      "RouteList",
      "RouteRow",
      "DragHandle",
      "InsertionIndicator"
    ],
    "states": [
      "normal",
      "hover",
      "pressed",
      "selected",
      "disabled",
      "loading",
      "empty",
      "error",
      "busy",
      "focus",
      "success",
      "dialog",
      "context-menu",
      "warning",
      "activation-pending",
      "activation-active",
      "single-emphasis",
      "preload-error",
      "toast-expired",
      "toast-persistent",
      "toast-stacking",
      "queue-promotion",
      "dirty",
      "applied",
      "apply-failed",
      "apply-unknown",
      "drift",
      "expanded",
      "collapsed",
      "all-nodes",
      "recovery",
      "drag-idle",
      "drag-lifted",
      "drag-insertion",
      "drag-cancelled",
      "route-create",
      "route-edit",
      "route-delete",
      "route-reorder"
    ],
    "rules": [
      "§3 核心视觉原则",
      "§4.1 Layout Geometry",
      "§4.2 Background Hierarchy and Color",
      "§4.3 Border, Radius and Shadow",
      "§4.4 Typography",
      "§4.5 Controls and Icons",
      "§5 Interaction State Spec",
      "§6 Component Composition Spec",
      "§6.7 Feedback",
      "§7.1 Golden Page A — Subscriptions",
      "§8.2 Subscriptions",
      "§10 Windows DPI / Window Size Verification",
      "§11 UI Delivery Gate",
      "§12 Implementation Boundary",
      "§13 Migration / Rollout Plan",
      "§7.2 Golden Page B — Proxies",
      "§8.3 Proxies",
      "§9 Visual Density Guardrails",
      "§14 TASK-012 Visual Remediation Acceptance",
      "§8.4 Routing"
    ],
    "source_paths": [
      "src/App.tsx",
      "src/styles.css",
      "src/main.tsx",
      "src/components/layout/SidebarTraffic.tsx",
      "src/components/layout/SidebarTraffic.css",
      "src/lib/proxy-routing.ts",
      "src/lib/bootstrap.ts",
      "src/lib/observability.ts",
      "src/lib/traffic-trend.ts",
      "src/lib/subscriptions.ts",
      "src/assets/fonts/Twemoji.Mozilla.ttf",
      "src-tauri/icons/128x128.png",
      "src/components/subscriptions/SubscriptionPage.tsx",
      "src/components/subscriptions/DocumentEditor.tsx",
      "src/components/subscriptions/DocumentEditor.css",
      "src/components/ToastHost.tsx",
      "src/components/proxies/ProxiesPage.tsx",
      "src/components/routing/RoutingPage.tsx"
    ]
  },
  "visual_gate": {
    "required": true,
    "human_approval": true
  },
  "ui_stages": [
    {
      "id": "S",
      "ui_scope": {
        "pages": [
          "Subscriptions"
        ],
        "components": [
          "AppShell",
          "Sidebar",
          "NavigationIcon",
          "SidebarTraffic",
          "Page",
          "PageHeader",
          "Toolbar",
          "Surface",
          "SubscriptionPage",
          "Button",
          "IconButton",
          "Input",
          "Select",
          "Checkbox",
          "Dialog",
          "ContextMenu",
          "Notice",
          "Toast",
          "DocumentEditor",
          "SubscriptionCard",
          "RecoveryAction"
        ],
        "states": [
          "normal",
          "hover",
          "pressed",
          "selected",
          "disabled",
          "loading",
          "empty",
          "error",
          "busy",
          "focus",
          "success",
          "dialog",
          "context-menu",
          "warning",
          "activation-pending",
          "activation-active",
          "single-emphasis",
          "preload-error",
          "toast-expired",
          "toast-stacking",
          "queue-promotion"
        ],
        "rules": [
          "§3 核心视觉原则",
          "§4.1 Layout Geometry",
          "§4.2 Background Hierarchy and Color",
          "§4.3 Border, Radius and Shadow",
          "§4.4 Typography",
          "§4.5 Controls and Icons",
          "§5 Interaction State Spec",
          "§6 Component Composition Spec",
          "§6.7 Feedback",
          "§7.1 Golden Page A — Subscriptions",
          "§8.2 Subscriptions",
          "§10 Windows DPI / Window Size Verification",
          "§11 UI Delivery Gate",
          "§12 Implementation Boundary",
          "§13 Migration / Rollout Plan"
        ],
        "source_paths": [
          "src/App.tsx",
          "src/styles.css",
          "src/main.tsx",
          "src/components/layout/SidebarTraffic.tsx",
          "src/components/layout/SidebarTraffic.css",
          "src/lib/proxy-routing.ts",
          "src/lib/bootstrap.ts",
          "src/lib/observability.ts",
          "src/lib/traffic-trend.ts",
          "src/lib/subscriptions.ts",
          "src/assets/fonts/Twemoji.Mozilla.ttf",
          "src-tauri/icons/128x128.png",
          "src/components/subscriptions/SubscriptionPage.tsx",
          "src/components/subscriptions/DocumentEditor.tsx",
          "src/components/subscriptions/DocumentEditor.css",
          "src/components/ToastHost.tsx"
        ]
      },
      "requires_golden_pages": [],
      "produces_golden_page": "Subscriptions",
      "evidence_ref": ".sdlc/evidence/TASK-019/ui-stages/S/certification-001.json"
    },
    {
      "id": "A",
      "ui_scope": {
        "pages": [
          "Proxies"
        ],
        "components": [
          "Page",
          "PageHeader",
          "Toolbar",
          "Surface",
          "Button",
          "IconButton",
          "Dialog",
          "Toast",
          "ProxyRoutingProvider",
          "ProxiesPage",
          "PoolSection",
          "NodeGrid",
          "NodeCard",
          "AllNodes",
          "RecoveryAction"
        ],
        "states": [
          "normal",
          "hover",
          "pressed",
          "selected",
          "disabled",
          "loading",
          "empty",
          "error",
          "warning",
          "busy",
          "focus",
          "success",
          "dirty",
          "applied",
          "apply-failed",
          "apply-unknown",
          "drift",
          "expanded",
          "collapsed",
          "all-nodes",
          "toast-expired",
          "toast-persistent",
          "recovery"
        ],
        "rules": [
          "§3 核心视觉原则",
          "§4.1 Layout Geometry",
          "§4.2 Background Hierarchy and Color",
          "§4.3 Border, Radius and Shadow",
          "§4.4 Typography",
          "§4.5 Controls and Icons",
          "§5 Interaction State Spec",
          "§6.7 Feedback",
          "§7.2 Golden Page B — Proxies",
          "§8.3 Proxies",
          "§9 Visual Density Guardrails",
          "§10 Windows DPI / Window Size Verification",
          "§11 UI Delivery Gate",
          "§12 Implementation Boundary",
          "§13 Migration / Rollout Plan",
          "§14 TASK-012 Visual Remediation Acceptance"
        ],
        "source_paths": [
          "src/styles.css",
          "src/lib/proxy-routing.ts",
          "src/components/ToastHost.tsx",
          "src/components/proxies/ProxiesPage.tsx",
          "src/App.tsx",
          "src/components/subscriptions/SubscriptionPage.tsx",
          "src/lib/subscriptions.ts"
        ]
      },
      "requires_golden_pages": [
        "Subscriptions"
      ],
      "produces_golden_page": "Proxies",
      "evidence_ref": ".sdlc/evidence/TASK-019/ui-stages/A/certification-001.json"
    },
    {
      "id": "B",
      "ui_scope": {
        "pages": [
          "Routing"
        ],
        "components": [
          "Page",
          "PageHeader",
          "Toolbar",
          "Surface",
          "Button",
          "IconButton",
          "Dialog",
          "Toast",
          "ProxyRoutingProvider",
          "RoutingPage",
          "RouteList",
          "RouteRow",
          "DragHandle",
          "InsertionIndicator",
          "RecoveryAction"
        ],
        "states": [
          "normal",
          "hover",
          "pressed",
          "selected",
          "disabled",
          "loading",
          "empty",
          "error",
          "warning",
          "busy",
          "focus",
          "success",
          "dirty",
          "applied",
          "apply-failed",
          "apply-unknown",
          "drift",
          "drag-idle",
          "drag-lifted",
          "drag-insertion",
          "drag-cancelled",
          "route-create",
          "route-edit",
          "route-delete",
          "route-reorder",
          "toast-expired",
          "toast-persistent",
          "recovery"
        ],
        "rules": [
          "§3 核心视觉原则",
          "§4.1 Layout Geometry",
          "§4.2 Background Hierarchy and Color",
          "§4.3 Border, Radius and Shadow",
          "§4.4 Typography",
          "§4.5 Controls and Icons",
          "§5 Interaction State Spec",
          "§6.7 Feedback",
          "§7.2 Golden Page B — Proxies",
          "§8.4 Routing",
          "§9 Visual Density Guardrails",
          "§10 Windows DPI / Window Size Verification",
          "§11 UI Delivery Gate",
          "§12 Implementation Boundary",
          "§13 Migration / Rollout Plan",
          "§14 TASK-012 Visual Remediation Acceptance"
        ],
        "source_paths": [
          "src/styles.css",
          "src/lib/proxy-routing.ts",
          "src/components/ToastHost.tsx",
          "src/components/routing/RoutingPage.tsx",
          "src/App.tsx",
          "src/components/subscriptions/SubscriptionPage.tsx",
          "src/lib/subscriptions.ts"
        ],
        "state_exclusions": {
          "expanded": "Routing does not own Pool expansion; this state is verified in Stage A.",
          "collapsed": "Routing does not own Pool collapse; this state is verified in Stage A.",
          "all-nodes": "Routing does not render the All Nodes view; this state is verified in Stage A."
        }
      },
      "requires_golden_pages": [
        "Proxies"
      ],
      "evidence_ref": ".sdlc/evidence/TASK-019/ui-stages/B/acceptance-001.json"
    }
  ],
  "scope": {
    "allow": [
      "src/components/proxies/ProxiesPage.tsx: compact sections/tabs/node grid only",
      "src/components/routing/RoutingPage.tsx: compact rows/drag and recovery only",
      "src/styles.css: Proxies/Routing/Toast scoped styles; Stage S Subscription Dialog width declarations and the exact direct-child local Notice override in design006 only",
      "src/lib/proxy-routing.ts: structural receipt/autoApply feedback/strict parsing/retryCurrentDesired",
      "src/components/ToastHost.tsx: success3000/noClose, typed actions, owner-scoped replace-clear",
      "src/App.tsx: recovery navigation callback to existing setActivePage only",
      "src-tauri/src/application/proxy_routing.rs: post-save disposition/retry union/attempt snapshot projection and inline tests",
      "src-tauri/src/application/managed_observation_runtime.rs: Ready-only guard/volatile latest slot/coalesced marker/attempt identity and inline tests",
      "src/lib/proxy-routing.test.ts",
      "src/components/proxy-routing-pages.test.ts",
      "src/components/ToastHost.test.ts",
      "src/components/subscriptions/DocumentEditor.css: .document-dialog border 1px solid var(--divider) only",
      "src/components/subscriptions/SubscriptionPage.tsx: ordinary/complex dialog class selection only",
      "src/components/subscriptions/SubscriptionPage.test.ts: existing Dialog regressions plus design006 local/global Notice presentation isolation regressions only",
      "scripts/sdlc-ui-contract.test.mjs: synthetic version0.4 and old0.3 wrong-version fixture alignment only",
      "Stage S: existing Toast Foundation plus precisely bounded Dialog border/width and fixture remediation; complete Subscriptions Golden certification remains required; design006 adds only local Notice CSS/test scope; all other Subscription page/editor/business source readonly."
    ],
    "deny": [
      "No implementation under this design authorization; exact Candidate006 Human Design/Adoption Approval, future implementation authorization and affected readiness/Stage S checkpoint required",
      "No Domain/AppState/schema/migration/Compiler/RuleSet/reorder backend semantic change",
      "No new dependency/worker/durable queue/background retry",
      "No changes to historical TASK018 Evidence/findings or TASK020 materialization",
      "No fabricated Golden, mutable certification alias or protocol change; old0.2 FAIL preserved as provenance; valid internal S must gate A",
      "Stage S remediation: no Subscription handlers/fields/CRUD/persistence/activation, DocumentEditor.tsx, subscription IPC/runtime, evaluator semantics, Proxies/Routing/Rust or other shared layout changes",
      "Change006 does not reopen historical implementation allowances: only proposed src/styles.css direct-child local Notice override and focused SubscriptionPage.test.ts tests; no markup/class, DocumentEditor, Proxies/Routing/Rust, Contract0.4/R01-R10 or Toast lifecycle changes."
    ]
  }
}
```
