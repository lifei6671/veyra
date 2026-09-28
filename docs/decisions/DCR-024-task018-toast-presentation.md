---
id: DCR-024
revision: 1
status: PROPOSED
affected_task: [TASK-018]
---
# TASK-018 global Toast presentation and bounded display queue

Source TASK018-HV-004 and USER's explicit bounded host/queue design authorization. Revises Candidate008's no-CSS/no-App/no-other-source restrictions solely for the exact files and message presentation adapters in design009. Candidate008, target014 and all prior reviews remain immutable. No implementation authority until new exact Human Change/Design Approval.

Top-right WebView client viewport; existing notice-edge20px and notice-gap10px; oldest visible first, append below, at most three visible with FIFO waiting. No page layout shift. Only App failureToast, non-Dialog Subscription Notice, and the single ProxyRoutingProvider message owner enter the host. Contextual Dialog and DocumentEditor errors remain local. No business state, message classification, outcome inference, runtime or activation sequencing in the host. Existing lifetime/explicit-clear/dismiss conditions remain authoritative, including expiration while waiting; no timer reset on promotion.

DCR is necessary for frozen execution scope change, not a business/architecture Material Change or UI Contract version change. Contract0.3 stays binding; existing 6000ms discrepancy remains separately tracked, not silently fixed or treated as full compliance. Exact design009 lists files, ownership, queue lifecycle, deterministic/native checks and rollout. Independent review → exact Human Change/Design Approval → rebind/gates → implementation/new target; no Task rebind, production changes, native capture or Human APPROVED Evidence in this design-only turn.
