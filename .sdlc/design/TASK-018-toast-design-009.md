# TASK-018 bounded Toast design009

Status: CANDIDATE. DCR-024 / TASK018-HV-004. Supersedes Candidate008 only for Toast integration write scope; Card, pending/completion, runtime and historical implementation remain unchanged. The prior impact009 is a preserved pre-authorization analysis; the current USER authorization resolves its host/queue scope question.

## Actual owners and existing lifetimes

App.tsx:63/69-73/92/99/104/230: failures have 6000ms expiry, explicit close, and clear on next runtime action. Preserve text and result classification.
SubscriptionPage.tsx:91/135-139/188/500/529/536: one Notice, 6000ms timer; contextual clears and Dialog behavior. Only messages produced for non-Dialog context enter the global queue. Keep Dialog message rendering and timer path unchanged. DocumentEditor.tsx remains entirely unchanged.
ProxyRoutingProvider in src/lib/proxy-routing.ts:224-270 is ONE owner shared by Proxies/Routing, not two producers: persistent success/error/warning until dismiss or next mutation clears current Notice; no auto timeout. Preserve mutation classification, snapshot/response ordering and refresh. Page components remove only duplicate global Notice markup; their inline/dialog validation remains unchanged.

## Closed production write list

- NEW src/components/ToastHost.tsx: React Context/provider, minimal in-memory presentation queue, enqueue/remove, stable monotonically assigned presentation id, expiration scheduling, fixed host rendering; no dependencies or external store. Keep all queue logic in this file.
- src/main.tsx: wrap production App in ToastProvider only; preserve StrictMode, context menu and existing settings harness branch.
- src/App.tsx: replace only failureToast presentation storage/timer/markup with the host adapter. Existing setFailureToast call sites and business action/observation flow stay unchanged.
- src/components/subscriptions/SubscriptionPage.tsx: presentation sink routes global vs dialog-local; replace only global Notice markup/its timer ownership. Preserve all existing message-producing conditions and local Dialog Notice. activationCompletion, PendingActivation, useLayoutEffect completion body/order, invoke handling, Card functions/props/classes stay unchanged; setNotice(completion) remains at its existing site, resolved through the presentation adapter.
- src/lib/proxy-routing.ts: only Notice presentation sink and dismiss adapter; keep existing mutation setNotice call ordering, mutation semantics, query/event/poll logic and Context business fields unchanged. One enqueue per occurrence at this provider, never at both pages.
- src/components/proxies/ProxiesPage.tsx and src/components/routing/RoutingPage.tsx: remove only global proxy-routing-notice rendering and now-unused notice/dismiss destructuring; no page/dialog logic edits.
- src/styles.css: new host stack positioning; reuse existing Toast visual tokens/classes within host, neutralize fixed left/right/bottom for host items. Do NOT globally relocate .subscription-notice, because Dialog and DocumentEditor still consume it. No SubscriptionCard or unrelated token/style changes.

## Queue and adapter semantics

Each non-null global presentation emission receives a new stable id, original text/severity, existing dismiss affordance, owner-local explicit-clear identity and optional expiresAt. No message-text deduplication, no sorting by severity, no retry, no cross-process events, no business operation identity. Emit at the existing presentation setter boundary, not in an effect watching a nullable slot: same-batch occurrences must not disappear, StrictMode rerender/effect replay must not duplicate them. Never enqueue during render or a React state updater callback. Stable context operations must not retrigger business effects.

Visible items are the first three live FIFO entries. A new item appends below existing visible items; fourth waits. Removal closes its gap and promotes earliest live waiting entry at bottom; appending alone never moves existing items. Queue immutable transitions support direct deterministic tests. No extra capacity/drop/dedup policy. Provider unmount disposes timers; no persistence.

App/Subscription global entries expire 6000ms after emission, including time waiting. Promotion never restarts the clock. A waiting entry that reaches its existing deadline is removed as expired, not silently reported displayed. This preserves lifetime rather than extending it to six seconds after promotion. Persistent routing entries have no synthetic timeout. The user-visible stacking capacity does not override an owner's existing explicit clear: setNotice(null)/setFailureToast(null) removes that owner's current presentation id (including if waiting), never another owner's entry or all entries. Replacing a non-null slot no longer overwrites earlier still-live entries; this is the specifically requested multi-presentation retention. Each timer/manual dismissal removes only its own id, and an old closure cannot dismiss the owner's newer current item. Existing explicit next-operation clear conditions remain unchanged.

Do not infer global/local from whichever page is visible later. Subscription adapter routes by current committed Dialog context at emission; dialog mode transitions must use an up-to-date presentation ref, avoiding stale async closures. Existing dialog identity guards remain unchanged. Contextual errors remain local for that dialog lifetime and never replay globally on close. Success that closes the dialog must route as the existing non-Dialog success outcome after its existing reset (same-batch reset+success handled explicitly in presentation sink, not by delaying completion). Opening a dialog must not move an old global entry into it; global and local slots are separate. Keep original local reset/timer conditions and DocumentEditor errors intact. No changes to handlers' transaction decisions or result text.

## Geometry, accessibility and theme

Host fixed top/right var(--notice-edge), bottom/left auto, flex-column with var(--notice-gap), align-items:flex-end, no shadow or animation framework. Width bounded by existing 340px/error and300px/success rules and available client width. pointer-events:none on empty host area and auto only on items; host max-height:calc(100dvh - 2 * var(--notice-edge)), vertical overflow for exceptionally long messages; stable width prevents scrollbar-induced horizontal jump. Max three minimizes overlap; no reserved page padding/layout displacement. At narrow supported widths verify main actions remain reachable; record REWORK if material obstruction cannot be solved in this positioning scope, do not redesign pages.

Use existing popup/text/semantic colors and severity classes; support info presentation without inventing producer or business classification. No focus stealing. Preserve role alert for error and status for ordinary feedback and existing close button labels. Native decorated titlebar lies outside WebView; top20px does not overlap native controls. No custom titlebar introduction. Global host remains below existing modal layer; local modal notices stay within modal and readable/focusable. Do not portal contextual errors out of modal or allow global dismiss to escape modal focus containment.

## Closed verification write list

NEW src/components/ToastHost.test.ts: existing Vitest, production queue/expiration/owner-id operations + real host SSR, no dependencies/config/framework. Fake-clock tests:1/2/3 ordered items, fourth waits/promotes, enqueue doesn't reorder, automatic expiry including waiting, explicit dismissal/next-operation clear, late old-id timer cannot clear new, owner separation, repeated equal messages distinct, strict replay no duplicate via integration contract.
Existing src/components/subscriptions/SubscriptionPage.test.ts: only presentation routing/adapter assertions (Dialog errors stay local; close/reset+success routes global; globals never migrate into Dialog). Retain all Card/completion/order/failure tests unchanged.
Existing src/components/proxy-routing-pages.test.ts: remove obsolete inline global Notice expectations, assert page/modal behavior retained and no duplicate global render; existing mocks may be supplied ToastProvider context without weakening business assertions.
Existing src/lib/proxy-routing.test.ts: only one-owner presentation adapter regression when needed; existing business tests untouched.
Test adapter production functions directly; SSR is not proof of native effects/timing. Native verification must cover integration, not substitute synthetic screenshots. No new test entrypoint/harness or fault framework.

## Gates and verification

Design validator+independent review then exact human approval. No materialization now. Later: target based on target014 full source identity plus explicit delta; fresh lint/test/build, related tests, diff-check and source Delivery Review. Historical source/native PASS stays intact for prior target.

Real Tauri: single activation success and StateUnavailable error top-right;2/3 entries ordered;4th FIFO; expiry/dismiss promote; cross-owner messages not overwritten; Dialog/DocumentEditor contextual errors still local; no append/layout shift; no native control or primary-action obstruction. Light/dark at100/125/150 DPI using normal desktop window (no unrelated full matrix). Record waiting expiration honestly. Preserve native old-active→target-pending→failure→old-active-restored and zero dual emphasis every sampled frame. Reuse approved isolated fault method with backup/restore; no new stop/start/cancel fault campaign. If safe actual UI cannot produce a required combination, record NOT_RUN rather than fabricate runtime/messages.

No Contract update, DCR023 semantics revision, Golden Certification, TASK012 readiness, new Task or Human Visual Approval. Existing6000ms-versus-Contract discrepancy remains explicitly outside this change. Scope-only DCR required; no business/architecture Material Change. After implementation stop at applicable Human Visual gate, never infer acceptance from source/native review.
