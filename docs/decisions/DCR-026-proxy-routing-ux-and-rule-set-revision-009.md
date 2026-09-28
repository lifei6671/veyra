# DCR-026 revision009 — Stage S Native state applicability

Status: CANDIDATE / exact Human Design and Adoption Approval pending.

Frozen conflict: Candidate006 Stage S declares toast-persistent although independent Review018 proves no current Subscriptions product path. USER explicitly decides to correct certification applicability rather than modify product behavior.

Proposed decision: adopt TASK-019-stage-s-certification-scope-007.md. Remove only toast-persistent from ui_stages[S].ui_scope.states. Keep TASK-level and A/B persistent state requirements, full state union, Stage S Toast/RecoveryAction/§6.7/shared primitive verification, all other UI metadata and Contract0.4/R01–R10 unchanged. No N/A/exception/evaluator change; no source/test change.

Alternatives rejected by explicit product decision: changing Subscription lifetime, adding recovery or debug paths, borrowing ProxyRouting recovery, substituting primitive tests for Native evidence, or weakening screenshots/Human approval. These are not within this Change Control.

Impact: affected certification design/adoption/readiness/checkpoint require new exact scope identities and gates. Unchanged target018 automation/code partition evidence may remain applicable only after explicit independent source-bound review; old Native/Compliance FAIL and incomplete evidence never become PASS. Historical Candidate006/Design006/Review018 and all approvals remain append-only provenance.

Sequence: Candidate007 + Task snapshot → Design Gate → independent Design Review → exact Human Design/Adoption Approval. Stop there this turn. Following adoption and affected entry checks, continue remaining full reduced-S Native matrix; current Compliance + complete independent Delivery precede Human Visual. Stage A/B retain persistent recovery obligations and remain locked until current Subscriptions Golden.
