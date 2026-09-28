# TASK-018 single-emphasis bounded impact analysis

Change source: TASK018-HV-003, current USER Human Visual Direction REWORK on target013. The confirmed direction is an at-most-one emphasized SubscriptionCard invariant; no applied runtime may correctly yield zero cards.

## Frozen conflict and actual execution points

Candidate007/design007 explicitly permits old runtime-active A and pending B to coexist visually. SubscriptionPage.tsx:518 passes the raw runtime predicate into Card; :615 derives active and pending classes independently. isSubscriptionVisualActive at :26 is also consumed by activationCompletion at :35. Therefore do not change that predicate's meaning or add pending arbitration inside completion: doing so would alter the already-approved completion mechanism.

styles.css:572 already gives the union of active and pending classes exactly the same 2px primary border/content surface/compensated padding; :570 excludes both from normal hover. Removing the old Card active class is sufficient. CSS remains read-only, including focus, hover, tokens and padding. The old Card may return to normal border while its real Runtime Observation remains Ready/applied; presentation is not runtime ownership.

SubscriptionPage.test.ts:25-38 currently routes the raw predicate into SSR and explicitly asserts A stays visually active during B pending. That assertion must be revised. Preserve the underlying true runtime projection assertions, all completion/order/failure tests, and add pair-of-Cards presentation assertions against the real production arbitration function, not a copied model.

## Minimal action

DCR023 revision003 + Candidate008 supersede only the coexisting-border presentation rule and narrow future write permissions to SubscriptionPage.tsx presentation arbitration plus SubscriptionPage.test.ts. Keep the existing runtime predicate and activationCompletion/useLayoutEffect/invoke continuations/metadata byte-for-byte semantically unchanged. A small pure local presentation predicate shared by the Card call-site and tests is justified to test the actual production arbitration without changing completion or adding state. No new store, framework, dependency, prop/state ownership or business behavior.

Contract0.3 §5 and §8.2 define the border language and separation from focus; they do not require every runtime-active Card to remain visually emphasized while another Card is pending. No Contract edit/version upgrade. This is a frozen Task design/visual acceptance change requiring DCR and exact approval, not a backend/business/architecture Material Change.

## Affected delivery and historical facts

Only TASK018 SubscriptionCard presentation and its tests are impacted. Preserve target013, all reviews/approvals/native traces and Candidate007 unchanged. Their PASS remains true for the old design and cannot close HV003. Current Human Visual Direction is REWORK, final Delivery/Acceptance cannot pass. After exact new approval: rebind, materialization/readiness/checkpoint, two-file implementation, new target/verification, independent source and minimal native review, then Human pending direction review. No implementation or Task rebind in this design turn.

No data/deployment/schema migration or rollback of existing work. Future source change makes target013-based current visual evidence stale for new acceptance, not historically false. Shared CSS identity remains intact; SubscriptionPage source identity changes, so any certification depending on that page must use the new source identity. This does not establish Subscriptions Golden Certification. TASK012 Candidate005 and Golden dependency chain stay untouched and blocked until separate applicable certification/approvals. No TASK011 changes.

## Verification and explicit boundary

See design008 for paired-state traces, both response/observation orders, failure Ready-vs-stopped/recovery, same-card and null-operation first render. Test file already authorized, no new testing scope. Native success evidence cannot substitute native failure evidence; unavailable scenarios remain NOT_RUN. Full matrix remains paused. No CSS/DOM/content restructuring, handler/mutation/Runtime/IPC/persistence/App.tsx/transport/observationError changes, timer6000ms, C/N repair, Proxies/Routing, or extra source paths. If two-file scope cannot satisfy acceptance, stop for bounded impact; do not enlarge it.
