# TASK-018 activation pending design006

Status: CANDIDATE — independent review required; response-first continuity issue unresolved.

## Authority and exact boundary

USER scope clarification permits the Card-only active presentation projection. DCR023 narrowly supersedes the old no-TSX/no-active-calculation restriction. Canonical Contract0.3 stays byte-identical; §5 local-action loading and SubscriptionCard active plus §8.2 apply. Prior38 shared findings remain bound; no 39th shared-foundation finding is invented. HV002 is a separately tracked interaction delta. This candidate does not authorize implementation before exact Review/USER approval.

## Two independent read-only projections

Page computes:

visualActive = observation != null && observation.sidecarLifecycle === "ready" && observation.appliedSubscriptionId === subscription.id && observation.appliedConfigurationGeneration !== null

activating = pendingActivation?.id === subscription.id

Card gets two presentation booleans and does not inspect operation states. Neither persisted subscription.active nor subscription.activeConfigurationGeneration participates in visualActive. Do not compare persisted generation with applied generation: old A loses its persisted active generation when B is saved, while A may still run. Preserve all other persisted selection consumers, including existing reactivation toolbar behavior. Do not copy observation into state, infer readiness from response, add optimistic active, or retain a last-active fallback.

## Composition and precedence

Retain article/header/description/meta/traffic structure and all handlers. Render compact existing inline-pending language with 切换中… in the existing single-line description box while activating; temporarily replace its displayed description text, retaining original description in title. Reuse existing RefreshIcon/spin/reduced-motion behavior if a spinner is shown, without changing refresh-button updating semantics. No added Card row, height, badge or new animation system. Keep that existing text line's height and margins in all states; status disappearance must not shift Card/header/metadata. Native narrow/long-text checks must reject clipping or geometry shift.

normal: existing1px divider and content surface, original spacing.
hover: existing normal hover only if neither active nor pending and not disabled.
active OR pending: one2px primary border and existing1px padding compensation, normal content surface; union selector, never additive border. Same-card active+pending remains2px.
focus-visible: existing independent ring and visible surface gap, no focus/active substitution; keep current approved focus implementation and no layout shift.
disabled: preserve existing action guards, tab order and disabled controls; hover suppressed, no class-based loss of runtime-active/pending information. Do not add a whole-card opacity treatment or change locking semantics as part of this delta.
pending: aria-busy=true, visible loading label; false/unset after cleanup; no check, Badge, shadow, stripe, primary title or selected background. Selection-mode checked state stays distinct and does not trigger activation.

## Transition and ordering matrix

| Case | Expected projection / existing completion | Design disposition |
|---|---|---|
| Initial A ready | A active; B normal; no pending | Pure observation projection |
| Click B before invoke resolves | A remains active only while observation ready/applied A; B pending immediately | Existing setter precedes await |
| queued/checking/prepared/persisted/applying | B pending; A active if actual received ready/applied A; persisted list refresh cannot promote B or remove A | No persisted fields in active expression |
| Ready observation before pending response | B runtime-active; pending may briefly coexist; setting operationId lets existing effect clear it | Same2px border, no additive styling |
| Ready observation before synchronous success response | B active already; response clears pending, existing Toast unchanged | No border gap |
| Synchronous success response before ready observation | Existing activate():432 clears pending and upserts persisted result, but visualActive still uses old observation; B can become normal until next observation | OPEN: cannot claim no stale-active/target-border flash; no handler changes authorized |
| Asynchronous matching ready + lifecycle ready + applied target + generation | Existing effect clears pending and emits existing success Toast; B active, A normal | No new success state |
| Cancel / failure with old runtime still ready | Pending clears via existing error/cancel path; only actually applied A active | No persisted fallback |
| stop failure / start failure with recoveryRequired or stopped observation | Pending clears; no active Card | No fake A rollback; classification follows observed lifecycle, not assumed error category |
| Immediate error | Existing pending cleanup/error Toast; active follows observation | No optimistic target |
| ready but runtime target inconsistent | Existing effect emits error and clears pending; target cannot become active from response/summary | Other truly applied ready Card may remain active |
| Same-card reactivation | Same Card may active+pending, exactly2px; loading distinguishes; stopped/recovery removes active; no persisted generation comparison | Verify new applied generation and both delivery orders |

The table describes guarantees relative to delivered observation; it does not claim zero backend-to-UI transport latency. Monotonic observation acceptance already lives in App.tsx and stays unchanged. Delayed/missing global observation or observationError is explicitly not repaired here. Normal response-first ordering is a local integration concern, not an excuse to expand transport. Requirement of no obvious stale-active flash remains mandatory and currently unproven by this proposed pure mapping. Do not hide it with delay animation, timeout, optimistic response data, a second pending state or backend change.

## Verification after any exact approved design

Design review must trace persisted-list precommit refresh and both invoke/observation orders from current source. Implementation verification must exercise the entire table including cancellation, stop/start failure, same-card reactivation, old-generation loss, null observation and mismatched ready; verify zero promotion from persisted fields and unchanged business handlers. Reuse current Vitest/frontend verification and existing controlled harnesses; no new test framework, dependency or source-test write authorization is implied. If adequate deterministic tests require additional test-file scope, identify that bounded need before writing. Native verification is separate and cannot be replaced by synthetic tests. After approval and implementation, require fresh source/verification review, light/dark pending/active/focus/hover preview and geometry checks before full matrix and final Human gates. Current turn runs design validator/manifest/diff checks only.

## Risk and provenance

Only SubscriptionPage.tsx and local styles.css are proposed future writes; ui_scope source_paths also include readonly shared delivery dependencies. Reference ProxiesPage inline-pending/aria-busy as local pattern, never authorize Proxies edits. No TASK011/012 change, Notice6000ms repair, C/N items, shared token change or full redesign. Existing active direction acceptance is not repeated or converted into pending acceptance. target008 remains historical implementation identity. After future changes, new source/visual identities are required; prior native PASS does not certify pending. Subscriptions Golden chain stays blocked independently.

Review must return REWORK/BLOCKED if response-first continuity cannot be met without changing the protected existing completion sequence. This candidate is a reviewable proposal with a disclosed unresolved issue, not an execution-ready Frozen Candidate.
