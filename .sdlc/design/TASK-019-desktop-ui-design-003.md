# TASK-019 Desktop UI / Interaction Design 003

Status: CANDIDATE / NOT FROZEN. Producer: Codex /root. Implements R07–R10 and presentation of R01–R06; runtime semantics live in auto-apply design.

## Reference mapping and bounded differences

Local reference `E:/wx_lifeilin/github.com/clash-verge-rev`:
- `src/components/proxy/proxy-render.tsx`: head followed by multi-column ProxyItemMini; map to Pool compact section and node grid.
- `proxy-item-mini.tsx`: two-line node name/type, real selected and disabled state, Light/Dark surfaces. Reference is56px with3px left stripe and selected alpha0.15/0.35; Veyra intentionally keeps approved Contract node48–54px and excludes stripe / full-row blue-fill emphasis per accepted direction. No copied Mihomo fields, delay probes or new runtime calls.
- `proxy-head.tsx`: compact tools only; Veyra renders existing edit/delete/expand tools, no copied latency/search/network features.
- `src/components/rule/rule-item.tsx`: compact text, secondary type and target plus divider. Veyra needs editable Route semantics and accessible drag, which the reference read-only row does not supply.
- Existing Veyra Contract0.3 tokens and their reference provenance remain binding for both themes. No new color system, library, asset, font or theme persistence.

## Proxies composition

`full Page -> PageHeader(existing create) -> [出口组][全部节点] -> exactly one panel`.
Tabs use role tablist/tab/tabpanel, aria-selected/controls, roving tabIndex; Left/Right/Home/End switch tab and focus, default出口组 each page mount; view preference not persisted. Do not render duplicate All Nodes under group panel. Data/provider state not reset by tab switch.

出口组 panel: compact section per stable Pool ID, thin divider and shared surface; header48–52px with名称、类型、当前节点 and compact expand/edit/delete where existing capabilities allow. Long name ellipsis with accessible full name; node count secondary only where useful, not another large header band. Expanded content uses repeat(auto-fill,minmax(min(220px,100%),1fr)), existing8px gap and node48–54px (candidate52px) height. Node name+protocol at existing font scale; no per-node padded white parent card. Collapse hides content, header/focus retained, one expansion click never selects node.

Selected uses one compact check glyph in reserved icon slot and normal surface/text; no large blue fill/leftstripe/primary title/badge combination. Hover existinghover token; focus existing independentfocus token. Selection source remains existing SelectorSnapshot: show actualruntime selection, desired-only distinction remains textual/contextual and never fabricates runtime truth. UrlTest does not become manual selection. 未知/不可用节点禁用真实选择操作。

全部节点 panel: same node grid, one item per stableNodeId, secondaryprovider attribution, read-only browsing for nodes without an existing unambiguous selector context. Never invent a global“使用节点”mutation or duplicate nodes by group membership. Uses existing data, no newly fetched performance metrics.

## Routing composition and drag

Compact fullPage/header/toolbar; remove normalHeader Apply and all persistent ↑/↓ reorder buttons. Row exposes matcher+target plus existing edit/delete; reserves a24px left handle slot to avoid content shifting. Handle is a real button with aria-label“移动规则 {name}”, hidden visually until row:hover / focus-within; remains focusable, pointer-events disabled outsidehover/focus. Disabled when current structural IPC save is pending, modalopen, no valid snapshot or list cannot be safely edited. RuntimeApply busy is displayed separately; see save/admission design.

Pointer: primary button on handle only; pointerdown capturespointer and stores {routeId, baselineOrderedIds, revision}. Movement threshold4CSSpx prevents click causing reorder. During drag, local provisional order only, source row retains layout slot; lifted rendering uses original row dimensions, existing surface+border and slight shadow allowed by proposedContract only while dragging. No entire-page translate/reflow; insertionindicator2px primary between targets. Hit-test vertical row midpoints relative scroll container; near edges allow bounded requestAnimationFrame autoscroll only while actively dragging, cancel RAF on every terminal/unmount. No timer-based backend tasks. Pointer capture coordinates update after scroll, not stale viewport coordinates.

Keyboard: Space/Enter on handle picksup; Up/Down move provisional insertion one row, Home/End start/end; Space/Enter commits, Escape cancels. Polite live region announces rule/position/total and drop/cancel. Tab or focus leaving the list while pickedup cancels before moving focus; no trap. No aria-grabbed deprecated contract. UI has no visible sort arrows. Pointercancel/lostcapture/windowblur/unmount/modalopen/snapshot orderedID or revision change cancels and leaves authoritative order. Same-position drop noIPC. Stable-ID focus returns to handle after success/cancel; deletedtarget falls to nearest remaining row or toolbar.

Commit: revalidate baseline snapshot/revision, submit complete current stableID order exactly once to existing reorderRoutes. Mark drag ended before invoking async handler so duplicate pointerup/Enter cannot double-submit. On conflict/busy refresh, restore authoritative list and show recoverable feedback; never retry/replay reorder automatically. An optimistic list is provisional only and cannot replace provider authoritative snapshot. Drag cancellation has zeroDesired/runtime side effects.

## Toast policy and recovery lifecycle

Shared ToastHost ordinary success is always3000ms from original enqueue time, independent oflegacy owner durations; waiting expiry remains existingenqueue-based and no success close button. The prior6000ms success discrepancy is explicitly changed prospectively, not declared retrospectively fixed inTASK018. ExistingFIFO/capacity3/append-below/itemidentity/fixedsafeinset and pointer/modal rules preserved. Needed UI blast radius includes all consumers of sharedsuccess policy for verification; no unrelatedpage redesign.

ProxyRouting ordinarysuccess: exactly one completion for current tuple, no permanent已保存badge. SavedStopped success3000ms; applying is nonterminal status, notsuccess. Noactive/recovery/applyfailure: warning/error persistent with explicit action. Applyfailure label“重试应用” calls shared retryCurrentDesired() and passes nocapturedCRUD/oldrevision/payload. While retry running buttondisabled, statuschanges to正在应用; doubleclick cannotdispatch twice. Retry availability from currentstate: blockedruntime yields explanatory feedback withoutstart. Secondary查看运行状态 navigatesOverview via existingnavigation; no automaticruntime recovery. `App.tsx` already owns activePage/setActivePage and renders ProxyRoutingProvider: candidate proposes only an onRecoveryNavigate callback prop wired to existing setActivePage("home" / "subscriptions"). No new Router/context/event bus; no AppShell geometry change. This narrow App.tsx callback write must be added to the Human-approved scope before implementation.

Minimal sharedAPI proposal: ToastMessage optional action {label,onInvoke,disabled} for local UI callbacks only (noIPCserializingcallbacks); successdismissable=false and duration3000 override centrally. Add owner replace/clear operation for the ProxyRouting recovery notice so repeated failures replace the sameowner/currenttuple message and successful/newer superseding resolution clears the exactold persistententry, not unrelatedToasts. Preserve existingappend behavior for ordinary messages and oldowner operations. Do not create notificationregistry/eventbus or timers forautomaticApply.

Persistentrecovery is derived from authoritative failure/unknown/drift and must remain accessible after navigation/remount through Header recovery state; transientToast dismissal cannot resolve businessfailure. Clear replacedoperation Toast only when current authoritative tuple provesresolved, explicit userdismissesnotification, or newer feedbacksupersedes it; no stalecompletion clearingnewerror. Keep modal submit/validationerrors local. Recoveryaction focus must remain visible; removing focusedToast returnsfocus to triggeringcontrol if stillmounted or page recoveryentry. Shared controls use existing accessiblecontrast and no color-only state.

## Verification and gate

Use existingVitest and targetedproductionevent handlers. Fakeclock2999/3000, waitingexpiration, successnoClose, persistentaction callback exactonce/currentDesired, no cross-ownerclear. Native pointer+keyboarddrag/cancel/focus, Light/Dark withfive Contractwindow/DPIgroups, combineddrag/Toast/modal/noActionobstruction and reflow checks. Empty/loading/error/busy, longnames and existinglargefixture count; no newvirtualization unless measuredcurrentcase needs separatelyreviewedchange.

Stages are S Subscriptions shared Toast/full Golden, then A Proxies Golden, then B Routing. See stage-plan003 for authoritative source scope, entry checks and append-only re-certification. No internal certificate exists yet. This design-only turn performs no implementation or visual certification.

## Shared-success consumer scope (revision003)

Actual ordinary-success consumers are ProxyRoutingProvider and SubscriptionPage's useToastOwner("subscription",6000); App's failure owner is error-only. R09 is global ordinarysuccess3000/noClose. Subscriptions is therefore an actual declared page in Task-level ui_scope, not an ignored affected_consumers label. S covers the full current Subscriptions Golden criteria and the changed success behavior; only shared ToastHost/scoped Toast CSS and existing Toast tests may change during S after a later implementation authorization. SubscriptionPage/business arbitration/DocumentEditor composition remain read-only. If full certification finds unrelated defects, report the exact gap and seek bounded remediation authority; do not silently repair or certify them.

S requires real current-source Light/Dark × all five Contract viewport/DPI groups, full page state/composition checks, activation pending/active/single emphasis, local modal errors and focus, plus success2999/3000/noClose/expiry/queue/stack. Historical target018 is supporting provenance only. A cannot begin on S Compliance alone: Independent Review and actual Human Visual Approval must complete S certification. Later shared-file changes make S stale and block dependent gates until the exact new evidence and Design/Task references are formally adopted as described in stage-plan003.
