---
id: DCR-023
status: PROPOSED
change_source: USER TASK018-HV-002 and explicit runtime visual-active projection clarification
affected_task: [TASK-018]
---
# DCR-023: TASK-018 Subscription activation pending presentation

## Source and impact

Bind human-interaction-finding-002.json and pending-design-scope-authorization-006.json in Candidate006. The original blocked impact006 and independent pending-impact-review006 remain historical; USER now explicitly permits the Card-only read-only runtime projection. This DCR changes Frozen implementation scope, not Contract0.3 or runtime/business ownership.

## Narrow supersession

Supersede only DCR021 / active-card-design003 / execution004 / Candidate005 restrictions forbidding TSX and active calculation changes, solely for SubscriptionCard active CSS class presentation authority and the existing pendingActivation mapping. Keep DCR021 active2px primary border/content surface/no glyph language and DCR022 focus separation. Do not alter persisted subscription.active or its generation, other business uses, handlers, DTO, operation state machine, IPC, persistence, Runtime, Domain, Compiler or Router. Original38 Finding inventory and all historical implementations remain provenance, not reopened write authority.

## Proposed future scope

SubscriptionPage.tsx: compute visualActive from the existing observation prop (ready lifecycle, applied id equals Card id, applied generation non-null), never persisted fields; pass visualActive and activating derived from pendingActivation.id; only Card class/aria-busy and compact inline loading presentation change. styles.css: local pending border, inline busy label/spinner and active/pending/hover/focus precedence with existing geometry compensation. All other source writes prohibited, including App.tsx, observation transport/error handling, Notice6000ms, other pages and backend. No new dependency, token, framework or persisted state.

## Delivery ordering concern requiring Design Review

Current synchronous success clears pending before returning to the existing observation-driven effect. invoke response and observation delivery are independent; response-first can render old observation with pending cleared, producing a target border gap before applied observation arrives. Pure projection solves premature persisted active, but alone cannot prove seamless success. Candidate006 explicitly records this unresolved bounded concern; Review must not PASS by assuming delivery ordering or silently changing completion handlers. If correction needs extra local completion synchronization authority, report that bounded impact first; do not expand transport/backend scope.

## Gates and preservation

DCR023 and Candidate006 need exact independent Review and subsequent USER Change/Design Approval; the scope authorization is not that approval. No Task rebinding, implementation, new target or native evidence this turn. Preserve Contract0.3 identity, Candidate005 approval, target008/native review and all earlier history. Approved future implementation invalidates affected source/verification/visual evidence for the new delivery; historical PASS remains history. Reverify Subscriptions separately later; no Golden Certification or TASK012 readiness implied. No business/architecture Material Change is proposed; DCR required because Frozen implementation scope changes. Any unresolved ordering Finding blocks Design Gate.
