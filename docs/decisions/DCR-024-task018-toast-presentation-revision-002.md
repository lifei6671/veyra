---
id: DCR-024
revision: 2
status: PROPOSED
affected_task: [TASK-018]
---
# TASK-018 global Toast presentation and bounded display queue

Source TASK018-HV-004 and USER's explicit bounded host/queue design authorization. Revises Candidate008's no-CSS/no-App/no-other-source restrictions solely for the exact files and message presentation adapters in design010. Candidate008, target014 and all prior reviews remain immutable. No implementation authority until new exact Human Change/Design Approval.

Revision002 preserves revision001 sha256:76ce60d384d5165641c237ef64644ad81b55253f7a6d376a2ee9a667814e422e and Candidate009/Review009 REWORK as history. Adds only the USER-authorized modal error sink and adopts the separately reviewed infrastructure mode; validator files never enter product write scope.

Top-right WebView client viewport; existing notice-edge20px and notice-gap10px; oldest visible first, append below, at most three visible with FIFO waiting. No page layout shift. Only App failureToast, non-Dialog Subscription Notice, and the single ProxyRoutingProvider message owner enter the host. Contextual Dialog and DocumentEditor errors remain local. No business state, message classification, outcome inference, runtime or activation sequencing in the host. Existing lifetime/explicit-clear/dismiss conditions remain authoritative, including expiration while waiting; no timer reset on promotion.

DCR is necessary for frozen execution scope change, not a business/architecture Material Change or UI Contract version change. Contract0.3 stays binding; existing 6000ms discrepancy remains separately tracked, not silently fixed or treated as full compliance. Exact design010 lists files, ownership, queue lifecycle, deterministic/native checks and rollout. Independent review → exact Human Change/Design Approval → rebind/gates → implementation/new target; no Task rebind, production changes, native capture or Human APPROVED Evidence in this design-only turn.

## Modal mutation failure routing

ProxyRoutingProvider.mutate(mutation, optional presentation options) may take onPresentationError(message), used only by Proxies/Routing Dialog submit. It never enters MutationResult, mutation payload, IPC or operation state. Existing backend business-error and invoke/response-exception messages go to that sink instead of global error Toast. Dialog submit clears old formError before validation/request, passes its existing setFormError, remains open on failure, and displays existing dialog-error role=alert inside the focus boundary. Existing success/warning classification still uses global presentation and normal successful Dialog close; non-Dialog calls omit the sink. Snapshot/refresh/result/operation behavior unchanged. No new backend or observation handling.

Gate model evolution is an independent infrastructure delivery, not part of this DCR write authority. cross_cutting_shared retains all seven affected pages, with Overview/Subscriptions/Proxies/Routing as explicit verification contexts; it produces no Golden Page and does not rerun full page migration matrices. Independent scope-classification evidence plus formal Design validator/review and subsequent exact human approval are required.
