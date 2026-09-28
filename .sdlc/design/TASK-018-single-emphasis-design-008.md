# TASK-018 single-emphasis presentation design008

Status: CANDIDATE; exact Human Change/Design Approval pending. Revision of Candidate007 only for TASK018-HV-003, through DCR023 revision003. Contract0.3 remains unchanged.

## Presentation arbitration (no completion change)

Keep isSubscriptionVisualActive(observation,id) unchanged: observation exists, sidecarLifecycle ready, appliedSubscriptionId id, appliedConfigurationGeneration non-null. It remains the runtime predicate used inside activationCompletion. Never feed presentation suppression back into completion or runtime truth.

Add a small pure same-file presentation predicate (e.g. isSubscriptionCardActive(observation,id,pendingActivation)), used at the SubscriptionCard visualActive prop and directly by existing Vitest/SSR tests. It combines the existing runtime predicate with (pendingActivation === null || pendingActivation.id === id). It owns no state/effect and creates no abstraction module. Derive activating from the same existing pendingActivation id as today. Do not change Card DOM, prop shape, class concatenation, content, input/keyboard behavior or handlers.

Arbitration runs on every render, including the initial pending record with operationId null. Do not defer old-border suppression to an effect, invoke response, timer or observation delivery. Pending target wins presentation regardless of whether old applied runtime is still ready. Runtime-active facts may coexist with pending-target facts; only display is exclusive. Zero emphasized cards is valid when there is no pending and no real Ready/applied tuple. Focus ring remains an independent focus state and is not a second active indicator; no other selected component changes.

## Existing CSS and completion are frozen

No styles.css write. Its existing .subscription-card-active, .subscription-card-pending union already gives one2px primary border, normal content-surface and the same compensated padding. Thus target pending → target active reuses identical geometry, even when both classes briefly coexist on the target. Preserve loading/切换中…/aria-busy, description line height, no glyph/Badge/shadow/stripe/primary title, active/pending hover precedence and independent focus gap. The normal↔emphasized150%DPI rounding limitation from target013 is not a new CSS authorization; native validation must still assess perceptible movement.

Keep activationCompletion, PendingActivation metadata, the single useLayoutEffect completion site, invoke response handling, safe list upserts/readback and existing success/error/cancel Toasts unchanged. No fallback, delay or timeout. AlreadyCurrent response never manufactures active. Same-card pending keeps its emphasized border even if runtime is temporarily not-ready: pending supplies it; an active class is only permitted when the true runtime predicate also holds.

## Paired-state acceptance and tests

Use the real new production arbitration in the existing SubscriptionPage.test.ts SSR helper. Retain all Candidate007 production completion tests; adjust only the obsolete old-A-visible-during-B-pending assertion and related presentation expectations. Test runtime truth separately from presentation so suppressing a border never asserts A stopped. Do not implement a duplicate model in tests. Support with a call-site wiring assertion that page passes the actual arbitration, while existing completion still uses the unchanged runtime predicate. No new test files/framework/dependencies/config.

| Input sequence | Expected pair of Card presentations |
|---|---|
| A Ready, no pending | A active / B normal |
| click B creates pending with null operationId, observation still A | A normal / B pending immediately; at most one emphasized Card |
| metadata/intermediate statuses/persisted precommit refresh | A normal / B pending; persisted active/generation irrelevant |
| response-before-observation, then matching Ready B | A normal / B pending → A normal / B active; no B normal gap |
| observation-before-response B Ready while pending metadata absent, then response | B may be active+pending, still one border; completion unchanged, no A border or B normal gap |
| AlreadyCurrent delivered tuple or response first | same unchanged completion and target-only emphasis while pending; no invented event or optimistic active |
| failed/cancelled/immediate error, latest observation still A Ready/applied | pending cleanup → A active / B normal |
| failed/cancelled with stopped/recoveryRequired/no applied id/null applied generation | pending cleanup → A normal / B normal; no fallback |
| same-card A reactivation | A active → A active+pending when true Ready → A active; always one border; if transient runtime not-ready only pending class remains |

Across every pair snapshot count cards with active OR pending classes, not class count: <=1. Cover three cards as a simple invariant check if useful, without expanding UI scope. Restore A only after pending is cleared and only from the current observation; when a terminal observation arrives before response metadata, retain target pending until existing completion can identify it. No premature presentation cleanup independent of completion.

Future verification after approval/implementation: pnpm lint, pnpm test, pnpm build, existing TASK018 targeted Vitest tests plus updated SubscriptionPage.test.ts, git diff --check; bind to a new implementation target. Independently review the complete two-file delta and preservation of completion. Native minimal Light/Dark frame-by-frame transition from first pending render must show A normal/B pending, then B active without normal/geometry regression. Same-card and failure restoration/no-runtime paths require honest scoped evidence; native fault paths not observed remain NOT_RUN, not inferred from unit tests. Stop for Human pending visual direction review before complete DPI/window/state matrix or final compliance. Old target013 success screenshots are history, not evidence of this new exclusivity.

## Bounds and dependency chain

Only SubscriptionPage.tsx presentation predicate/call-site and SubscriptionPage.test.ts may be changed after exact approval. Full Task ui_scope/source_paths/finding inventory remain as identity/context, not broad write authority. No CSS, business handler/state ownership, Backend/IPC/Runtime/Domain/Compiler/StateStore/persistence/App.tsx/observation transport/observationError, timer6000ms, other38finding work, Task011/012 or Candidate004/005 updates. Source identity and affected Review/visual evidence must refresh after future edits. This does not certify Subscriptions Golden Page or unlock TASK012. Need new scope: stop, do not silently change completion/CSS.
