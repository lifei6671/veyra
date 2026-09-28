# DCR-025 revision002 — shared safe inset and closing-preload presentation

Status: CANDIDATE / INDEPENDENT DESIGN REVIEW REQUIRED / HUMAN CHANGE-DESIGN APPROVAL PENDING

## Authority and history

USER authorized this bounded design revision on 2026-09-08, only for TASK018-DR015-001 and TASK018-DR015-002. This is permission to design and independently review, not approval of this exact revision or implementation. The original DCR-025 proposal, Candidate-010, target-015, Reviews and failed Evidence are immutable history. Candidate-010 and DCR-024 revision002 remain the previously approved baseline; this revision supersedes only the two presentation decisions below after exact Human Approval.

Sources: `.sdlc/evidence/TASK-018/toast-source-review-015.json`, `.sdlc/evidence/TASK-018/toast-native-analysis-015.json`, `.sdlc/evidence/TASK-018/implementation-target-015.json`. Native target-015 evidence actually observed obstruction of Subscriptions Import/New and Header actions. It is not a hypothetical risk and is not closed by this design.

## TASK018-DR015-002: shared content safe start

Keep the right edge at `var(--notice-edge)`. Replace the fixed top notice edge with one shared, fixed, auditable content-safe inset composed of existing tokens: `calc(var(--page-header-height) + var(--page-padding) + var(--control-height) + var(--notice-gap))`. Current values are 58 + 10 + 34 + 10 = 112 CSS px. This is the first candidate, not a validated safe value. Bound host max-height by viewport height minus this same top inset and the existing bottom notice edge; preserve existing overflow behavior and below-Dialog layering.

All pages use this same position. Native verification may justify a minimal adjustment to the shared fixed inset within this bounded design, recorded with exact CSS and target identity and rerun across all four contexts. No root geometry token changes, page reflow, primary-control relocation, DOM measurement, ResizeObserver, per-page positioning or runtime collision/layout framework. If the fixed inset cannot satisfy the required Native matrix, report the failure rather than introduce these mechanisms.

Header, Toolbar and first-row primary controls must be visually unobstructed and clickable. Check rectangle nonintersection against every visible Toast, `elementFromPoint` ownership and real Native clicks. Click-through alone cannot satisfy geometric avoidance. Only a minimal Toast-item pointer-event adjustment is permitted if needed; preserve existing dismiss interaction. Existing Toast positions remain stable when another is appended below; page and Card rectangles do not shift. FIFO, max three visible, waiting expiry, promotion, lifetime and owner/item identities remain unchanged.

An overlay without reserved space cannot universally promise never to cover ordinary page content. This revision makes no such promise. A product requirement of zero visual coverage of any interactive content needs a separate decision about reserved layout space or more complex avoidance, outside DCR-025.

## TASK018-DR015-001: closing preload failures

Errors in a retained Dialog remain local, including validation, submit failure, QR errors expressible after opening (including overlong QR), and all other retained contextual errors. DocumentEditor stays local. Proxy/Routing invocation-local error sinks are frozen.

Only these existing immediate-close paths emit one global error Toast: edit settings result-error; edit remote share result-error; edit preload exception from either await; QR share result-error; QR preload exception. In the existing current-dialog epoch/target guard, close/reset the Dialog first, then emit directly through the existing global presentation owner. A minimal local presentation-only helper may perform this ordering. Never route through the generic contextual `setNotice` before reset, and never emit local then replay global.

Preserve result-error mapping through `subscriptionErrorMessage`, exception text `订阅响应不可用，请重试`, error classification, existing 6000ms owner lifetime, preload requests/order/arguments, transaction decisions, current-dialog guards, finally behavior and close outcome. Stale responses must neither close a newer Dialog nor emit an obsolete Toast. The helper must not become a request wrapper, business-state owner, event bus or global mutable state. Successful preload and retained-error behavior remain unchanged.

## Scope and frozen contracts

Future implementation writes are limited to `.toast-host` safe positioning/viewport bound and minimally necessary item pointer events in `src/styles.css`; the identified closing error branches/minimal presentation helper in `SubscriptionPage.tsx`; and actual asynchronous handler regression tests in `SubscriptionPage.test.ts`. No source or test writes in this design turn.

Do not change Toast queue/capacity/waiting expiration/owner identity/duration, activationCompletion, PendingActivation, useLayoutEffect completion sequencing, SubscriptionCard arbitration/single border/failure recovery, Proxy/Routing local sink, Backend/IPC/Runtime/Domain/Compiler/persistence, Contract 0.3 or Candidate-008/target014 history. The existing 6000ms versus Contract Toast-duration discrepancy remains a separate unwaived issue; no fix or Compliance exemption is implied.

## Acceptance and lifecycle

The executable verification plan is `.sdlc/design/TASK-018-toast-design-011.md`; Candidate-011 binds all artifacts by hash. Seven affected pages and four representative contexts remain truthful `cross_cutting_shared` scope. No Golden Page production, false ui_stages, page migration or page recertification; default page-migration Golden dependency enforcement remains unchanged. Independent classification and Design Review precede exact Human Change/Design Approval. Only afterward may TASK-018 rebind, readiness/checkpoint and a new implementation target occur. Native proof, UI Compliance, independent Delivery Review and Human Visual Approval remain required; this revision grants none of them and does not close TASK-018 or advance TASK-012.
