---
id: DCR-023
revision: 3
status: PROPOSED
affected_task: [TASK-018]
---
# DCR-023 revision003 — single SubscriptionCard emphasis during activation

Revises approved revision002 sha256:26e35a49092bfa3a3b8c5b010646e51c0c3a0e76de0d031443a00bf87a3f4d77 and Candidate007 only for TASK018-HV-003. Prior DCR/Candidate/approvals and target013 remain immutable. USER has confirmed the new visual requirement and authorized design/review; exact approval of this new DCR/Candidate is still required before implementation.

## Decision delta

Before: old runtime-active A and pending B may both display a primary border.
After: pending target has presentation priority. Card active class = runtimeActive && (pendingActivation === null || pendingActivation.id === card.id). Pending class remains pendingActivation?.id === card.id. From the first pending render no other Card retains an active border. At most one Card has active-or-pending emphasis. Same-card may have both semantic classes, still one2px border.

runtimeActive remains the existing read-only Ready + applied id + non-null applied generation predicate. Do not change isSubscriptionVisualActive, used by activationCompletion; apply arbitration only at Card presentation through the design008 local pure predicate. Persisted active/generation never participate. Upon pending clearance, current delivered observation alone determines whether an old applied Card is restored; no last-active/persisted fallback. No runtime truth, ownership or observation mutation.

## Retained constraints

Retain Candidate007 completion exactly: Activated/Reactivated/async matching Ready, AlreadyCurrent exact delivered tuple/no required new event, single pre-paint site, current failure/cancel/mismatch/immediate-error cleanup and existing Toasts. No metadata, sequencing, operation or timer change. Keep Contract0.3, DCR021 border language, DCR022 focus separation and all other findings unchanged.

Future write scope is only SubscriptionPage.tsx Card arbitration and SubscriptionPage.test.ts presentation assertions/regression coverage. styles.css is read-only because existing active/pending union already supplies identical borders. No DOM/handler/backend/IPC/Runtime/Domain/Compiler/StateStore/persistence/App.tsx/transport/observationError, no6000ms, no other UI components or new dependencies. This narrows execution write authority without dropping full Task UI scope/context.

## Review and adoption

Impact and deterministic/native acceptance: TASK-018-single-emphasis-impact-008.md and TASK-018-single-emphasis-design-008.md. DCR required solely because approved frozen visual design changes; no new business/architecture Material Change, no Contract revision or migration. Independent Design Review then exact Human Change/Design Approval; only afterward legal Task rebind/gates and implementation/new target. Old target013 reviews remain PASS under Candidate007; Human direction is REWORK, not delivery acceptance. No source, native captures, Task binding, Human APPROVED evidence or Golden/TASK012 progression in this turn.
