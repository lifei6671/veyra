---
id: TASK-018
milestone_ref: M7
dependencies: [TASK-011]
risk: MEDIUM
status: DONE
requirement_ref: docs/ui/veyra-ui-spec.md
requirement_identity: sha256:bd598a07008e6fc4bf210a412289d6c60d9e5f5f5dd372e4cd47090b301c86c0
design_refs:
  - .sdlc/design/TASK-018-shared-foundation-candidate-010.json
  - docs/decisions/DCR-024-task018-toast-presentation-revision-002.md
  - .sdlc/design/TASK-018-toast-design-010.md
  - .sdlc/design/TASK-018-shared-foundation-candidate-011.json
  - docs/decisions/DCR-025-task018-toast-primary-action-obstruction-revision-002.md
  - .sdlc/design/TASK-018-toast-design-011.md
approval_refs:
  - .sdlc/evidence/TASK-018/shared-scope-review-010.json
  - .sdlc/evidence/TASK-018/design-review-010.json
  - .sdlc/evidence/TASK-018/design-approval-010.json
  - .sdlc/evidence/TASK-018/dcr-024-approval-002.json
  - .sdlc/evidence/TASK-018/shared-scope-review-011.json
  - .sdlc/evidence/TASK-018/design-review-011.json
  - .sdlc/evidence/TASK-018/design-approval-011.json
  - .sdlc/evidence/TASK-018/dcr-025-approval-002.json
---

# TASK-018: bounded safe Toast inset and closing Subscription preload errors

## 最终有界验收 042（当前；以下 checkpoint 保留历史）

TASK-018 = DONE。USER:lifei 已实际通过四张 contact sheet 审阅 request041 的全部25张原始截图，并精确批准 target018 / binding039 / Compliance041 / Independent Review041。真实批准见 `human-visual-approval-042.json`；六个分区及整体集成验收见 `task-acceptance-042.json`，Completion Predicate 与交付结果见 `delivery-completion-042.json`，均位于 `.sdlc/evidence/TASK-018/`。

SF-042、SF-043 与 binding039 其余四项已交付 presentation 的当前 acceptance = PASSED；实现状态 IMPLEMENTED。原始 validator 的 info FAIL 以 `raw-ui-delivery-check-042.json` 原样披露，已有 applicability037 的适用性与当前完整审查见 `ui-delivery-resolution-042.json`。不把原始机器结果改为 PASS。

五项 HV038 保持 OPEN / transferred；历史 REWORK038 不变。本批准不认可背景 Proxies/Routing 整页 UX，不批准 drag、auto apply、新 Toast lifetime 或 Rule Set。TASK-019 在本 Task DONE 后仅物化为待 Requirement/Design 的任务，未授权实施；TASK-020 继续依赖 TASK-019。无产品源码修改、commit/push 或 Golden Certification。


## 当前 acceptance ownership（039，优先于以下历史 checkpoint）

USER:lifei 已明确批准 DCR-026 revision002 的 ownership transfer：HV038-001～004 → TASK-019，HV038-005 → TASK-020。五项保持 OPEN，原 findings038 和 Human Visual REWORK 不变；它们不再属于本 Task 当前最终 Completion Predicate，不要求在 TASK-018 修复。

当前绑定：`.sdlc/evidence/TASK-018/acceptance-scope-binding-039.json`；决定与完整边界：`docs/decisions/DCR-026-proxy-routing-ux-and-rule-set-revision-002.md`。最终人工视觉评审只含 shared foundation 已交付修复、Subscription pending/active transition、单一 Card emphasis、Toast positioning/stacking、closing preload errors 及原已批准直接 presentation delta。排除 Proxies/Routing 整页最终 composition、drag、auto apply、新 Toast lifetime 策略与 Rule Set；页面截图仅作 shared integration context 时不批准整页。

Task 保持 VERIFYING。独立 Change/Design Review 完成后重新生成有界 request；当前 Compliance/独立 Review 的全范围适用性、真实人工视觉批准、最终 Task Acceptance/Delivery Completion 全部满足后才 DONE。产品 Finding 的责任转移不代表产品已解决；未来 Design 不阻断本 Task 有界收束。TASK-018 DONE 后才 materialize TASK-019，之后依赖 TASK-019 启动 TASK-020。本轮不改源码或协议，不生成 APPROVED。

## 历史人工视觉结论（038，保留 REWORK，ownership 以039为准）

2026-09-08 USER:lifei 对 target-018 明确 REWORK。Task 保持 VERIFYING，Human Visual Review = REWORK，Task acceptance PENDING，aggregate Delivery FAILED。当前生产源码修改权限暂停；保留 target-018、Toast、Subscription transition 与全部历史 Evidence，不生成 APPROVED Human Evidence。

新 Findings 见 `.sdlc/evidence/TASK-018/human-visual-findings-038.json`。Proxies/Routing 整页 redesign、drag reorder、auto apply、Toast lifetime 正式修订归后续 TASK-019；Rule Set Domain/State/IPC/Compiler/UI 归后续 TASK-020。二者只登记 future stub，不在 TASK-018 内实施，也不因拆出而关闭 Findings 或验收本 Task。

有界影响与 Requirement clarification / Design 入口：`.sdlc/design/TASK-019-020-impact-038.md`；提出的 Material Change：`docs/decisions/DCR-026-proxy-routing-ux-and-rule-set.md`。当前仅文档分析授权，后续先明确需求、完成独立 Design 及所需批准，再物化和实现；focus 不自动迁移。以下既有执行授权与 checkpoint 仅保留为历史，不覆盖本限制。

## 当前执行绑定

Human Change / Design Approval 已精确绑定 DCR-025 revision002
`sha256:7284798c7a8cd7f3eb3057432d0f015e5e5d815d1a829fdd2b5b4f1971b6e1e2` 与
Candidate-011 `sha256:8344676f1dd3f648923315ef5eddbff33acf99a4421e8982597b185fec3f0d2b`；
接受 Independent Design Review 011 的 `PASS / FULL_BOUNDED_SCOPE / COMPLETE`。
Candidate-011 仅补充 Candidate-010：target-015、Candidate-010、Review、失败 Native Evidence 和此前正文均保留为历史 provenance。

状态保持 `VERIFYING` remediation。此批准授权重新绑定、readiness/checkpoint、最小实现和新的 implementation target；它不是 target-015 Finding 关闭、Native PASS、UI Compliance、Human Visual Approval、Golden Certification、TASK-018 Completion 或 TASK-012 readiness。

## 本次需求

仅解决 `TASK018-DR015-001` 与 `TASK018-DR015-002`。ToastHost 使用一个固定共享 content-safe inset，首个实现候选为
`calc(var(--page-header-height) + var(--page-padding) + var(--control-height) + var(--notice-gap))`（当前约 112px），保持右侧 `var(--notice-edge)`、向下 FIFO、最多三条可见、waiting/expiration/owner/item identity/duration 语义及无页面 reflow。

Subscription edit/QR 的 closing preload failure 保持既有 close outcome，按 `current-dialog epoch/target guard -> reset/close Dialog -> exactly one global Toast` 展示。保留 `subscriptionErrorMessage`、exception 文案 `订阅响应不可用，请重试`、classification、6000ms owner lifetime、preload API/参数/调用顺序、finally 及 close decision；stale completion 不得关闭新 Dialog 或发出旧 Toast。保留打开 Dialog 的 validation、submit failure、overlong QR 与其他 contextual errors 为 local；DocumentEditor 与 Proxy/Routing invocation-local sink 不迁移。

## Scope

### allow

- `src/styles.css`: `.toast-host` 共享固定 safe top 与对应 viewport `max-height`；如 Native hit-test 所需，仅最小 Toast item `pointer-events` 范围调整；不改 geometry token。
- `src/components/subscriptions/SubscriptionPage.tsx`: 既有 closing edit/QR preload error branches 与最小私有 reset-then-global presentation helper。
- `src/components/subscriptions/SubscriptionPage.test.ts`: 使用既有 Vitest 执行真实异步 production edit/QR callback 的 deterministic preload tests，以及 retained/stale regressions。

### deny

- 不改 ToastHost queue/FIFO/capacity/waiting expiration/lifetime/owner 或 item identity/duration。
- 不改 `activationCompletion`、`PendingActivation`、`useLayoutEffect` completion sequencing、SubscriptionCard arbitration 或其 CSS；不改 Proxy/Routing local sink。
- 不改 backend、IPC、runtime、domain、compiler、persistence、request、transaction、message、classification、close outcome、Contract 0.3 或 Toast duration。
- 不改 DocumentEditor source，不迁移 retained Dialog errors；不改 TASK-011/TASK-012、Candidate-008/010、target-014/015、Review 或既有失败 Evidence。
- 不引入 DOM measurement、ResizeObserver、dynamic collision、per-page positioning、page rearrangement/control relocation、dependency/framework/configuration/business-state abstraction/event bus。
- 不生产 Golden、不声明 `ui_stages`、不触发 page migration 或重新认证七页；不改 UI Gate infrastructure。

Candidate-011 中的 “No source or test edits, Task rebind, implementation target or Human Approval record during this design turn” 是其历史 design producer turn 的限制。当前 exact Human Change / Design Approval 已单独授权以上 implementation scope；其余冻结与 deny 仍有效。

## SF-042: shared Toast safe inset remediation

需求：将 `.toast-host` 的固定顶部起点由 `var(--notice-edge)` 改为一个基于现有几何 token 的共享固定 content-safe inset，并以相同 inset 约束 host 的 viewport `max-height`。不测量 DOM，不按页面改变位置，不预留或重排页面空间。若完整 Native matrix 证明首个 112px 候选失败，只能对同一个共享固定 inset 作最小调整；每次调整必须产生新 implementation target，并重跑受影响完整 Native geometry matrix。固定 shared inset 无法满足时停止并报告 Design impact。

Acceptance：Overview、Subscriptions、Proxies、Routing 的 Light/Dark 与 Windows 100%/125%/150% DPI 下，1/2/3 条可见 Toast 均保持 right edge、safe top、FIFO gap 与 append-below，已有 Toast rectangle 不移动，页面及 SubscriptionCard geometry 不发生 layout shift。Header、Toolbar 和 first-row primary controls 与每个可见 Toast rectangle 不相交，标签可见，`elementFromPoint` 命中 control 或 descendant，实际 Native 点击触发预期 control。Subscriptions 必须重测 Import、New 和 Header actions；pointer-events click-through 不能作为几何避让结论。

Verification：保留现有 Toast queue/lifetime/identity Vitest 回归；真实 Tauri 四个 representative contexts 的原生 rect、hit-test、实际点击、截图与 geometry capture，覆盖上述 theme/DPI/stack matrix；记录 target identity、每个 inset 值及无 layout shift 的页面/Card bounds。

implementation_status: PENDING
acceptance_status: PENDING

## SF-043: closing Subscription preload global presentation

需求：在真实 production edit/QR callbacks 中，以下六种 guarded closing preload failure 在 reset 后恰好发送一次 global error Toast：edit settings result-error、edit settings exception、edit remote share result-error、edit remote share exception、QR share result-error、QR share exception。不得先写 local Notice 后 reset，也不得关闭后 replay 造成双重 presentation。

Acceptance：六种情形各自保持原请求参数和调用顺序，Dialog 最终关闭且无残留 local Notice，exact baseline message 的 global error Toast 恰好一次，reset 先于 global emission。stale edit/QR completion 不影响新 Dialog 且无旧 Toast；retained submit/validation errors 与 overlong QR 仍 local；successful preload 与 DocumentEditor ownership 不回归。

Verification：`SubscriptionPage.test.ts` 使用现有 Vitest、真实 production edit/QR callback 及 controlled asynchronous result/error/exception 执行六条路径，断言 close、local/global destination、exactly-once、message、reset-before-emission、request arguments/order；并覆盖 stale、retained、overlong QR、success 与 DocumentEditor regression。不得以 helper-only 或 source-string test 替代。

implementation_status: PENDING
acceptance_status: PENDING

## Task 独立验收

新 implementation target 对账仅允许三条 write path；冻结的 Toast queue/lifetime/identity、activation/PendingActivation/useLayoutEffect/SubscriptionCard、Proxy/Routing sink、DocumentEditor 和 Contract 0.3 均有定向回归证据。SF-042 与 SF-043 自动化及原生验收完整、独立 Delivery Review 对新 target 无 P0/P1 Finding 后，仍须分别完成 UI Compliance 与真实 Human Visual Approval 才能完成 Task。

`cross_cutting_shared` 保留完整 affected pages（Overview、Subscriptions、Proxies、Routing、Settings、Connections、Logs），并只以 Overview/Subscriptions/Proxies/Routing 作为 representative verification contexts；不生产 Golden Page、不声明 `ui_stages`、不触发页面 migration 或重新认证七页。默认 `page_migration` Task 的 Golden dependency enforcement 不变。

## Risk

固定 overlay 不能承诺永远不视觉覆盖任意普通页面内容；本轮只验证 Header、Toolbar 与首行主要操作的几何安全且不改变页面布局。完整 matrix 若固定共享 inset 不足，必须停止报告新的 Design impact。现有 6000ms 与 Contract Toast duration 的差异仍是独立既有问题，不修复、不豁免、不记为 PASS。

## UI Delivery Metadata

```json
{
  "task_id": "TASK-018",
  "ui_impact": {
    "required": true,
    "reason": "Shared Toast safe content inset and closing-preload error presentation only"
  },
  "ui_contract": {
    "document": "docs/ui/veyra-ui-spec.md",
    "version": "0.3",
    "required": true
  },
  "ui_scope": {
    "pages": ["Overview", "Subscriptions", "Proxies", "Routing", "Settings", "Connections", "Logs"],
    "components": ["ToastHost", "GlobalToast", "Dialog-local Notice"],
    "states": ["normal", "hover", "focus", "success", "error", "warning", "info", "stacked", "queued", "dismissed", "expired", "modal-local-error", "closing-preload-error"],
    "rules": ["§4.1 Layout Geometry — existing notice edge/gap", "§4.2 Background Hierarchy and Color — existing Toast semantic colors", "§5 Interaction State Spec — Toast and modal context", "§6 Component Composition Spec — Toast", "§10 Windows DPI / Window Size Verification — shared component contexts", "§11 UI Delivery Gate", "§12 Implementation Boundary"],
    "source_paths": ["src/styles.css", "src/App.tsx", "src/main.tsx", "src/components/layout/SidebarTraffic.tsx", "src/components/layout/SidebarTraffic.css", "src/components/subscriptions/SubscriptionPage.tsx", "src/components/subscriptions/DocumentEditor.tsx", "src/components/subscriptions/DocumentEditor.css", "src/assets/fonts/Twemoji.Mozilla.ttf", "src-tauri/icons/128x128.png", "src/lib/bootstrap.ts", "src/lib/observability.ts", "src/lib/traffic-trend.ts", "src/lib/subscriptions.ts", "src/components/ToastHost.tsx", "src/lib/proxy-routing.ts", "src/components/proxies/ProxiesPage.tsx", "src/components/routing/RoutingPage.tsx"],
    "delivery_mode": "cross_cutting_shared",
    "shared_components": ["ToastHost"],
    "verification_pages": ["Overview", "Subscriptions", "Proxies", "Routing"],
    "shared_review": ".sdlc/evidence/TASK-018/shared-scope-review-011.json",
    "state_exclusions": {
      "selected": "Toast has no selectable item; existing SubscriptionCard selection/arbitration is protected historical target014 and a regression constraint, not new write scope.",
      "disabled": "Toast host and existing dismiss affordance introduce no disabled control; modal business disabled conditions remain unchanged.",
      "loading": "Host receives completed presentation messages and does not own operation loading. Existing activation loading remains protected.",
      "empty": "An empty queue renders no Toast overlay; deterministic zero-entry behavior is tested, not a page empty-state redesign.",
      "busy": "Host does not derive busy or runtime state; existing modal/activation busy behavior remains unchanged and must regress unchanged."
    }
  },
  "visual_gate": {
    "required": true,
    "human_approval": true
  },
  "scope": {
    "allow": [
      "src/styles.css: .toast-host shared fixed safe top and corresponding viewport max-height only; minimal item pointer-events if needed; no geometry token changes",
      "src/components/subscriptions/SubscriptionPage.tsx: existing closing edit/QR preload error branches and minimal private reset-then-global presentation helper only",
      "src/components/subscriptions/SubscriptionPage.test.ts: deterministic actual asynchronous preload callback tests and retained/stale Dialog regressions in existing Vitest only"
    ],
    "deny": [
      "No source or test edits, Task rebind, implementation target or Human Approval record during this design turn",
      "No ToastHost queue/FIFO/capacity/waiting expiration/lifetime/owner identity/duration changes",
      "No activationCompletion/PendingActivation/useLayoutEffect sequencing/SubscriptionCard arbitration or CSS changes",
      "No Proxy/Routing mutation error sink changes; retained Dialog and DocumentEditor errors remain local",
      "No backend/IPC/runtime/domain/compiler/persistence/request/transaction/message/classification/close-outcome changes",
      "No Contract0.3 edits or duration exemption; no historical Candidate008/010, target014/015, Review or failure Evidence edits",
      "No DOM measurement/ResizeObserver/dynamic collision/per-page positioning/page rearrangement/control relocation",
      "No new dependencies/frameworks/configuration/business-state abstraction/event bus",
      "No UI Gate infrastructure changes, Golden production, ui_stages fabrication, page migration or recertification"
    ]
  }
}
```

## Current target-015 verification result (historical failed baseline)

Implementation target `sha256:b1613fc8ebc995b98440327f2cf587bc83780984181b29337fbda36a105b0a97` and its verification/protected-region evidence remain immutable. Lint, all 249 tests, build and diff checks were PASS; Card/completion protected regions were unchanged. Independent source review is `FAIL / REWORK` for `TASK018-DR015-001` and `TASK018-DR015-002`; its Native Light 1280x720@150% evidence established the Subscription Import/New/Header obstruction. These historical results are neither replaced nor closed by the rebind.

## Historical task body (read-only provenance; superseded as current execution authority)

# TASK-018: UI Contract 0.2 Shared Foundation Alignment

## 本次需求

仅将SF-V02-001～038已确认shared foundation静态违约对齐到批准值。不是Subscriptions全页重设计、TASK012 remediation或Golden Certification。原审计JSON/MD与Candidate manifest精确绑定，本Task逐项引用全部38项，不复制改写历史。

## Scope

### allow

- src/styles.css: only A-class finding declarations and necessary listed consumers
- src/App.tsx: navigation SVG presentation only
- src/components/layout/SidebarTraffic.css: SF021/SF035 only
- src/components/subscriptions/DocumentEditor.css: SF021/024/029/031/033/034/036 only

### deny

- 禁止任何未由38项Finding支持的变化、新值、依赖、framework、页面composition或业务handler/state ownership。
- 禁止SubscriptionPage.tsx的6000ms行为、DocumentEditor.tsx、SidebarTraffic.tsx、src/lib/**、Router、IPC、Domain、Compiler、Runtime、StateStore、Rust。
- 禁止TASK011/012、Candidate004/005和历史Evidence修改；禁止TASK019或其它任务实现。
- Candidate中“本轮不物化/实现”是此前design-only权限；USER本次明确授权本Task物化和四文件实现，除此之外deny不变。

## SF-001: 38项静态Finding修复

需求：依据 .sdlc/design/TASK-018-shared-foundation-audit-001.json 全部A类Finding修改现有token/geometry及列出的直接消费者。source_paths只读依赖不等于write scope。

Acceptance：每项保存before、expected、after、位置、CLOSED/OPEN及新增consumer/magic-value判断；不得漏项或机械替换数字。

Verification：pnpm lint、pnpm test、pnpm build；现有Subscriptions/shared foundation相关测试；38项源码/computed证据对账和独立Review。

implementation_status: IMPLEMENTED
acceptance_status: PENDING

## Task 独立验收

完整38项closure、四文件scope/diff、light/dark token/consumer、nav presentation-only、保护文件hash、独立Code Delivery/UI Review与本Task有界视觉Evidence。Task compliance只证明修复范围，不宣称完整Subscriptions Certification。真实Human Visual/Task Approval前不得DONE/Delivery PASSED。

## Risk

共享CSS改变相关visual identity；保留旧FAIL原身份，后续认证使用新记录。Candidate004005和TASK011历史功能验收不变。C/N项包括6000ms Notice留待后续只读page-specific审计；不直接重跑Golden Certification。不引入业务Material Change/DCR；若确需改变Frozen语义立即停止报告。

## Execution authority

USER明确授权将TASK018作为当前唯一实施unit，TASK012仍VERIFYING且不改文件，不执行其readiness。前置TASK011历史功能已完成，不把本Task依赖自身未来Golden。此次顺序覆盖默认等待TASK012关闭的习惯，非架构/业务变更。

## Historical UI Delivery Metadata (read-only provenance)

```json
{
  "task_id": "TASK-018",
  "ui_impact": {
    "required": true,
    "reason": "Subscriptions shared visual foundation tokens/geometry and direct presentation consumers"
  },
  "ui_contract": {
    "document": "docs/ui/veyra-ui-spec.md",
    "version": "0.3",
    "required": true
  },
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
      "Button",
      "IconButton",
      "Input",
      "Select",
      "Dialog",
      "Menu",
      "Notice",
      "Toast",
      "SubscriptionCard",
      "DocumentEditor controls"
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
      "menu",
      "pending activation"
    ],
    "rules": [
      "§4.1 Layout Geometry",
      "§4.2 Background Hierarchy and Color",
      "§4.3 Border, Radius and Shadow",
      "§4.4 Typography",
      "§4.5 Controls and Icons",
      "§5 Interaction State Spec",
      "§6 Component Composition Spec",
      "§7.1 Golden Page A — Subscriptions",
      "§9 Visual Density Guardrails",
      "§10 Windows DPI / Window Size Verification",
      "§11 UI Delivery Gate",
      "§12 Implementation Boundary",
      "§13 Migration / Rollout Plan",
      "§8.2 Subscriptions active state"
    ],
    "source_paths": [
      "src/styles.css",
      "src/App.tsx",
      "src/main.tsx",
      "src/components/layout/SidebarTraffic.tsx",
      "src/components/layout/SidebarTraffic.css",
      "src/components/subscriptions/SubscriptionPage.tsx",
      "src/components/subscriptions/DocumentEditor.tsx",
      "src/components/subscriptions/DocumentEditor.css",
      "src/assets/fonts/Twemoji.Mozilla.ttf",
      "src-tauri/icons/128x128.png",
      "src/lib/bootstrap.ts",
      "src/lib/observability.ts",
      "src/lib/traffic-trend.ts",
      "src/lib/subscriptions.ts"
    ]
  },
  "visual_gate": {
    "required": true,
    "human_approval": true
  }
}
```

## Implementation checkpoint / review blocker

Historical blocker TASK018-DR-001 remains in delivery-review-001.json / finding-closure-002.json. USER approved DCR-020 and Candidate-002 exact identities; new binding permits only DocumentEditor.css SF036 remediation. Candidate-001 and its approval remain historical. SF036 must be reverified on a new implementation identity; full four-file/38-finding independent Review required, not incremental. Human Visual/Task Acceptance remains PENDING.

## Current Review checkpoint

Target sha256:af3020e9275072ec772abdd750e773fac95f4e953b5222b181f90a36925809d4 retains SF036 correction. Independent FULL_SCOPE review found SF021 QR gap overridden to12px at src/styles.css:763, expected form14px. See delivery-review-003.json and finding-closure-004.json. Stop per USER instruction on any new Review Finding; resume point IN_PROGRESS. No further CSS change/native evidence/Human approval. Task acceptance remains PENDING.

## Current remediation authority

USER authorizes TASK018-DR-002/SF021 remediation and full38 final-effective producer audit under Candidate002; existing approved finding defects may be corrected in authorized files only. Prior Review/closure remain historical. No new DCR or Candidate; new scope/C/N/handler/6000ms changes require STOP. Fresh independent FULL_SCOPE Review before native evidence.

## Current verification checkpoint

Target sha256:ac954467fe73906e97d1a9da5e9ce13bfda1761138afc144fd717933e30d089d: producer38/38 final-effective static checks and fresh independent FULL_SCOPE/COMPLETE source+automated Review PASS. SF021/SF036 closed on this target. Native Evidence partial (light1280x720@150); environment blocker prevents remaining theme/DPI matrix via supported Computer Use. See native-visual-005.json. Task/Human acceptance PENDING, aggregate Delivery not PASS. Earlier blocker records retained as history; no new scope changes.

## Current Candidate-004 execution authority (supersedes earlier current checkpoints)

USER exact Candidate004 / published Contract0.3 approval supersedes earlier execution permissions for this delta. Original 38 findings, four-file delivery and earlier checkpoints remain historical provenance. Only SF038/HV001 expected active-card language changes via DCR021. Current new write permission is src/styles.css SubscriptionCard active/hover/focus/border compensation only; all other original writes suspended. Candidate003 design-only denial applied to its earlier proposal turn; current USER authorizes legal binding and this delta. TASK-012 Candidate004/005 remain protected; TASK-018 Candidate004 is the approved current design.

No TSX, handler/state, other37 finding changes, tokens, Dialog/Notice/6000ms, Sidebar/Proxies/Routing/DocumentEditor, backend, C/N work. Keep VERIFYING and acceptance PENDING. After implementation/verification/review collect only Light active, Dark active, Light active hover, keyboard focus at one normal desktop window/DPI. Stop for explicit USER visual direction acceptance before full matrix. No Golden Certification or TASK012 readiness.

## Current Candidate-005 focus correction authority

Supersedes earlier current delta authority after exact USER DCR022/Candidate005 approval. Only src/styles.css SubscriptionCard focus-visible/local focus inset may change. Active border/background/padding, hover, DOM/TSX/handler/state, other37 findings and tokens unchanged. Start -6px as trial, adjust only local focus inset within approved scope if native gap is insufficient. New target and fresh verification/review required; four native previews (focus1280x720@150) then STOP for Human Visual Direction. No final compliance/matrix/acceptance/Golden/TASK012. Prior approval/target007/nativeREWORK remain history. VERIFYING and acceptance PENDING.

## Current Candidate-007 pending activation authority

Exact USER approval supersedes prior current write restrictions only per DCR023 revision002. Current execution design identity sha256:cd3a17078ba19f54d5d3c9bb733ea9bf3e896a1ad8ce37c9ca5536fad2ccc170. Prior scopes/checkpoints remain historical, not additional write permission.

Allow:
- src/components/subscriptions/SubscriptionPage.tsx: Card runtime visualActive/pending presentation and existing pendingActivation local success completion sequencing/metadata per design007
- src/styles.css: SubscriptionCard local pending/active/hover/focus precedence and existing geometry compensation only
- src/components/subscriptions/SubscriptionPage.test.ts: USER-authorized verification-only file, existing Vitest, real production completion predicate/Card presentation, no new framework/dependency

Deny:
- No other source writes; historical broad scope is provenance only
- No persisted subscription.active ownership or generation comparison; no global replacement of business consumers
- No business transaction/ownership/DTO/IPC/backend/persistence/Runtime/Domain/Compiler/Router/StateStore changes; only local presentation completion sequence expressly allowed
- No App.tsx, observationError or observation transport failure handling
- No Contract0.3 change, active2px/focus separation change, new token/dependency/framework, other38 finding cleanup
- No TASK011/TASK012 or historical Candidate/Evidence changes; no Notice6000ms or C/N items
- No Task materialization, implementation, screenshots, new implementation target or Human approval in design-only turn

Local pending completion sequencing and readonly runtime visual-active are required per design007; all specified ordering/terminal cases and fresh source/native reviews required. VERIFYING, Task acceptance and aggregate Delivery PENDING. No Golden/TASK012 readiness.

## Candidate007 implementation checkpoint (current)

USER exact approval authorizes implementation in this turn; the copied design-only-turn denial above is historical and does not prohibit this approved execution. Task remains VERIFYING, acceptance PENDING. Target013 sha256:635b407699b503d541dffd84999ac27a9ac55d621c5d03ee10a2e72226a9852b binds two production paths and the single authorized test file delta. verification013: lint/build PASS, full199/199, targeted61/61, diff PASS. Independent pending-source-review-013.json FULL_SCOPE/COMPLETE/PASS covers source and automated verification only. Prior target010 Review FAIL and subsequent failed verification attempts remain historical.

Light native transition evidence: native-transition-013/light-transition-evidence.json covers real Tauri switch, same-card reactivation, AlreadyCurrent. Independent native review and manual Dark preview pending. Native fault scenarios are not claimed verified; deterministic tests cover failure/cancel/stopped/recovery and both delivery orders. No Human Visual/Task approval, full DPI matrix, Golden Certification or TASK012 readiness.

Independent pending-native-light-review-013.json: PASS / FULL_SCOPE / COMPLETE only for three captured Light paths at1280x720@150. No new Finding; dark preview PENDING, native fault paths NOT_RUN, aggregate Delivery/Human Visual Acceptance PENDING. Await manual Dark theme before next bounded preview; no full matrix.

## Dark pending preview checkpoint

2026-09-08: USER manually switched Windows Dark. Target013 unchanged; no production/test edits. dark-transition-evidence.json records initial activation and cross-subscription transition at1280x720@150. Independent pending-native-dark-review-013.json PASS/FULL_SCOPE/COMPLETE for these two paths only (7202 RAF samples,1020 compositor hashes). Together with the previous Light preview, stop for Human pending visual direction review. No new Finding; native fault paths/fullmatrix/finalCompliance/HumanAcceptance/Golden/TASK012 readiness not completed. VERIFYING and acceptance/Delivery PENDING.

## Current Candidate008 execution authority

USER exact DCR023 revision003/Candidate008 approval supersedes old coexisting-border visual rule and previous broader write permissions. Only SubscriptionPage.tsx pure Card presentation arbitration/visualActive call-site and SubscriptionPage.test.ts corresponding paired-card/single-emphasis tests may change. CSS and isSubscriptionVisualActive/activationCompletion/PendingActivation metadata/useLayoutEffect/invoke response sequencing/Toast/timer stay unchanged. Inherited completion permission is not current authorization. Runtime truth remains observation-only; pending target exclusively owns Card emphasis from first pending render. Failure restores only current Ready/applied Card; same-card one border. Preserve all Candidate007 completion/order/terminal tests and all historical Evidence.

New target/verification/independent source/native review required. VERIFYING, acceptance/Delivery PENDING; no Human Visual approval/Golden/TASK012 readiness.
