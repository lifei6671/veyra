# TASK-018 pending completion design007

Status: CANDIDATE. Revises design006/Candidate006 after TASK018-HV002-DESIGN-002. Exact new Human Change/Design Approval pending.

## Preserved projection and composition

visualActive = observation != null && observation.sidecarLifecycle === "ready" && observation.appliedSubscriptionId === card.id && observation.appliedConfigurationGeneration !== null.

Never consult subscription.active or persisted activeConfigurationGeneration, compare persisted/applied generations, or derive visualActive from invoke success. The comparison with result generation below ONLY settles AlreadyCurrent pending; it never determines active class. Existing persisted selection business uses stay untouched.

activating = pendingActivation?.id === card.id. Old runtime-active A and pending B may coexist. Same-card active+pending is one2px primary border, not4px. Retain design006's local composition: existing single-line description box displays compact inline loading/切换中… while pending and retains description in title; no new content row/height, no content structure redesign. Existing spinner/reduced-motion pattern may be reused without changing refresh action semantics. Restore description on completion in the same fixed line box. Normal1px+14px inner spacing, active OR pending2px+13px compensation; active/pending suppress normal hover, preserve normal content surface. Focus remains independent with visible gap; retain approved local ring. No check/Badge/shadow/stripe/primary title. Disabled action guards/tab order remain; no new whole-card opacity rule. Batch selection remains separate.

## One pending record; one completion site

Extend existing pendingActivation minimally: id, nullable operationId, completion kind (operation or alreadyCurrent), and expectedGeneration only for alreadyCurrent. Initial click sets the existing record before await with null operationId; it cannot settle from a previous operation's observation. No parallel store, copied runtime state, last-active ref or extra operation lifecycle.

Invoke continuation:
- pending response: record its operationId and operation completion kind; preserve pendingOperation and pending presentation.
- Activated/Reactivated synchronous success: preserve existing safe summary upsert/readback; attach operationId and operation completion kind; DO NOT clear pending or emit success yet.
- AlreadyCurrent: attach operationId, alreadyCurrent kind and result.activeConfigurationGeneration; do not fabricate observation or clear unconditionally.
- Immediate error/exception: preserve existing clear-pending/clear-operation/error Toast behavior. No extra backend invocation.

Replace the current activation completion passive effect with a single useLayoutEffect (pre-paint) reacting to the current delivered observation and pending record. Use a small pure completion decision in the same SubscriptionPage.tsx, shared by this integration and deterministic tests; it returns wait/success/error, without side effects or state ownership. This is local code, not a new module/framework/state machine. The effect consumes the decision: clear existing pendingActivation/pendingOperation, existing refreshList and exactly one existing success/error Notice. Keep all other handlers/transaction steps unchanged. No side effects inside React state updater callbacks. No async continuation reads a render-captured stale observation; metadata triggers the latest render/effect instead. useLayoutEffect state updates settle before paint, not a timer/delay trick. On SSR effects do not run; static markup tests alone cannot prove sequencing.

## Completion predicate ordering

1. No pending/unknown operationId: wait. Old switch records are not this activation.
2. Matching operation terminal failed/cancelled: error/cancellation outcome with current existing messages; never wait for success. Matching ready with lifecycle not-ready, wrong applied target or absent generation: existing mismatch error. Nonmatching terminal records must not terminate this request.
3. Operation kind: wait during matching queued/checking/prepared/persisted/applying, no/mismatched operation; success only matching switch ready AND lifecycle ready AND applied target AND non-null generation. Async pending and synchronous Activated/Reactivated use exactly this same predicate and success text: 订阅已应用，服务正在使用最新配置.
4. AlreadyCurrent kind: success when delivered lifecycle ready AND applied id target AND applied generation exactly expectedGeneration, irrespective of current switch operationId (after the genuine matching failure/mismatch checks above). Text: 订阅已是当前运行配置. No future matching event is assumed. If tuple has not caught up, wait, not fake ready; unrelated old failed records do not defeat a valid tuple. A matching ready tuple with wrong expected generation is a confirmed inconsistency, using existing mismatch error, not infinite wait. Unknown/mismatched stale observations cannot be declared a terminal failure just to stop loading.

The failure classification follows delivered lifecycle. On failure with old applied Ready, only old applied Card remains visual-active. On stopped/recoveryRequired no Card is active. A matching failed observation must not leave pending active. No transport reliability correction or invented timeout; if delivered observation never confirms/terminates, pending remains honestly pending. That limitation is outside scope and does not authorize observationError/App.tsx changes.

## Deterministic trace acceptance (not results)

Each step records rendered active class, pending class, aria-busy, Notice count and existing pending metadata; assert no normal target frame between pending and confirmed active in valid success sequences.

| Case | Ordered input | Expected |
|---|---|---|
| persisted precommit refresh | A ready; click B; persisted list now B active/A generation null; observation still A | A visual-active, B pending, never promote B from list |
| observation-before-response Activated | pending id unknown; delivered matching Ready/B; response metadata | B active while loading until metadata; same pre-paint commit settles once, no extra painted loading frame |
| response-before-observation Activated | response metadata then later matching Ready/B | B stays pending through old observation; next render B active then pre-paint cleanup, no normal gap |
| Reactivated both orders | same as above using new matching operation and same id | one2px border, pending until new operation ready; old ready operation cannot settle |
| async pending -> ready | response operationId; intermediate statuses; matching Ready | same predicate and one existing success Toast |
| AlreadyCurrent observed | existing ready/applied target/generation; response alreadyCurrent | settle pre-paint without new switch event |
| AlreadyCurrent response first | old applied tuple; response; later exact target/generation | retain pending until tuple; response does not create active; wrong generation does not succeed |
| cancel/failed | matching terminal after metadata, or before metadata then response attaches identity | clear pending with existing error/cancel Toast, active solely observation |
| stopFailed/startFailed/recoveryRequired | matching failure with ready old tuple vs stopped/recovery | old Card only if lifecycle ready; no fallback on stopped/recovery |
| immediate error | rejected/error response | clear pending using existing error path; no promotion |
| ready mismatch | matching Ready but wrong target/absent generation/notready | clear with mismatch error, never active from persisted/response |
| same-card reactivation | active id also pending id | one border, busy label, unchanged bounds, independent focus |

## Test scope and method

Existing Vitest/node and SSR tests do not currently exercise SubscriptionPage pending sequencing. USER explicitly authorized exactly src/components/subscriptions/SubscriptionPage.test.ts (test-only; pending-test-scope-authorization-007.json). No new dependencies/config/framework. Deterministic unit traces invoke the real pure completion predicate in SubscriptionPage.tsx, including operation mismatch and both response paths. Do not write a copied model that can pass while production differs. Wiring assertions plus actual Card SSR projection tests cover response handlers attaching metadata rather than clearing success, the single layout-effect integration and active/pending union. Static/source assertions are supporting evidence, not substitutes for executable predicate traces or native pre-paint continuity checks. If actual mounted React scheduling cannot be proven with existing test infrastructure, report that explicit limitation; native frame-by-frame validation remains mandatory, not a fake unit PASS. A new mounted DOM framework is not authorized.

Future required commands: pnpm lint, pnpm test, pnpm build, pnpm exec vitest run src/components/subscriptions/SubscriptionPage.test.ts plus existing Subscriptions/shared foundation tests, git diff --check. The one test file is authorized for future approved implementation; no test file is created this turn.

## Gate and boundary

Only new production writes: SubscriptionPage.tsx as described and SubscriptionCard local styles.css. No backend/IPC/App.tsx/transport/persistence/Runtime/Domain/Compiler/StateStore, no global active substitution, no timer6000ms, no C/N items, no other38 finding rework. Canonical0.3, DCR021 language, DCR022 focus, TASK011/012 and historical Candidates/reviews/targets preserved. Candidate006 REWORK stays unchanged; new identity required. New implementation target and verification/independent source+native review only after exact approval/rebind/checkpoint. No visual approval, complete matrix, Golden certification or TASK012 readiness this turn. Existing static visual direction acceptance is not pending acceptance.
