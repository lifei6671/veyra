# DCR-026 revision008 — Stage S Dialog-local Notice scope

Status: CANDIDATE / design-only authorization; exact Human Design/Adoption Approval pending.

Frozen conflict: Candidate005/revision007 restricts non-Toast Subscription CSS to width, but TASK019-STAGES-NATIVE-014-001 requires existing local validation Notice to remain in modal presentation. Native014 FAIL and Impact014 establish the bounded need.

Decision proposed: adopt .sdlc/design/TASK-019-stage-s-local-notice-006.md as the sole additional scope overlay. Future writes are one direct-child CSS rule in src/styles.css and focused existing SubscriptionPage.test.ts regression only. No markup change is proposed or authorized. Unchanged DOM is sufficient for static block presentation on current evidence; independent Review must assess its scrolling, wrapping, alert/status and isolation constraints.

Alternative: markup/class changes require a demonstrated scoped-CSS insufficiency in independent Review and a new candidate; global positioning changes would violate the approved owner/context separation. No new abstraction/dependency is justified.

Contract0.4, R01–R10, all business/runtime semantics, Toast arbitration/duration/queue/owner/focus/Recovery Action and Stage order remain unchanged. DocumentEditor and Stage A/B work are not reopened. toast-persistent applicability remains separately unresolved; no frozen requirement is waived.

Sequence: Candidate006 and exact Task snapshot → design machine check → independent Design Review → exact Human Design/Adoption Approval → exact adoption. This authorization ends at the approval gate; no readiness/checkpoint, product/test source writes or Native capture this turn. Future implementation requires its own authorization and affected entry checks; fresh full source-bound certification remains required. All older candidates, approvals, target013, Native014 FAIL and 34 screenshots retain immutable provenance.
