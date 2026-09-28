---
id: DCR-023
revision: 2
status: PROPOSED
affected_task: [TASK-018]
---
# DCR-023 revision002 — local pending completion sequencing

Revises proposed DCR023 first revision sha256:217d7e3a14a42536fb050e81343360d198fa54ecb6596a0844d8c020c7294fc1. That file, Candidate006 and design-review006 FAIL/REWORK remain unchanged. USER explicitly resolves TASK018-HV002-DESIGN-002 through pending-completion-scope-authorization-007.json. No exact approval of this new revision/Candidate is implied.

## Retained scope

Only SubscriptionPage.tsx Card presentation and existing pendingActivation lifecycle plus local SubscriptionCard styles.css. Retain the narrow supersession of DCR021/Candidate005 no-TSX/no-active-calculation restrictions for Card-only observation-derived visual active. No Contract0.3 change: active2px primary/content surface/no glyph, focus independent visible gap, no shift remain. Persisted subscription.active and its generation retain all existing business uses and ownership. No backend/IPC/Runtime/Domain/Compiler/StateStore/persistence/App.tsx/transport/observationError modifications. No Notice6000ms change, C/N repair, new tokens/framework/dependencies or other page redesign.

## Additional precise supersession

Replace Candidate006 prohibition on local success completion changes ONLY to resolve response-first pending disappearance. Add minimal completion metadata to the existing pendingActivation record; no second state machine/store. Synchronous Activated/Reactivated must not clear pending in the invoke-success branch. They join async pending in waiting for matching operation Ready plus lifecycle Ready/applied target/non-null generation. AlreadyCurrent instead requires delivered ready/applied target/exact result generation and does not require a new operation event. Responses cannot manufacture visualActive. Matching failure/cancel/inconsistent terminal observations clear pending via existing Toast language; immediate errors retain existing cleanup. No delay/timer/fallback masks observation latency.

Use a single pre-paint completion site observing current render observation plus current pending metadata; no stale captured observation in invoke continuation. Thus already-delivered confirmation clears on metadata arrival before another painted frame. Backend transaction, invocation, subscription list updates and response parsing remain unchanged. Detailed algorithm and deterministic cases are bound in design007.

## Impact and approval

Frozen implementation scope changes; no business/architecture Material Change or Contract revision. Existing target008 and all historical approvals/reviews stay intact. After independent Review PASS and exact USER Change/Design Approval, legal Task rebinding/checkpoints precede implementation. New source target/tests/review/native evidence then required. Current turn is design-only. No Task materialization, production/test source, native evidence, final visual approval, Golden certification or TASK012 readiness.

USER separately authorized one future test file src/components/subscriptions/SubscriptionPage.test.ts using existing Vitest (pending-test-scope-authorization-007.json). This is verification-only scope; no test implementation this turn and no testing framework/config/dependency changes.
