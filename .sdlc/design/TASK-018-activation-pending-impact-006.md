# TASK-018 activation pending impact analysis006

Status: BLOCKED — active presentation authority scope requires clarification before Candidate006.
Source: USER confirmed TASK018-HV-002. This is design-only analysis, not native certification or implementation authorization.

## Requested bounded change

Keep Contract0.3, DCR021 active language and DCR022 independent focus/gap. Map existing pendingActivation.id to a presentation-only activating prop on SubscriptionCard; reuse ProxiesPage is-pending/aria-busy/切换中… pattern. No new operation, state machine, IPC, handler, timer or runtime change. Requested future files are SubscriptionPage.tsx and local styles.css only. All historical38 findings, Task011/012, Candidates and target008/native-preview-review008 remain unchanged.

## Static conflict: TASK018-HV002-DESIGN-001

The required non-optimistic active semantics cannot be guaranteed by adding only a pending prop while preserving the existing active presentation source:

1. SubscriptionPage.tsx:424-435 sets pendingActivation before await and already clears it on immediate error, success or exception. Lines93-109 handle matching operation terminal observation and require ready/lifecycle ready/applied id/generation for the async success Toast. Lines84-92 independently refresh the list on subscription-state events.
2. SubscriptionPage.tsx:494-497 passes the unmodified subscription summary to Card; line589 derives active class from subscription.active. refreshList:151-155 accepts list results without an observation guard.
3. commands.rs:794-804 returns SubscriptionManager.list summaries. subscription_management.rs:386-391 and1468-1479 load persisted AppState; summary_for:1607-1610 derives active and generation from persisted selection, not applied runtime observation.
4. managed_observation_runtime.rs:1475-1478 saves the candidate;1525-1530 emits Activated before commit_prepared:1538. Thus the frontend event can refresh B as active while applying is still pending. commit_prepared can fail at1538-1539 after the persisted selection changed. No native reproduction is claimed: this is a source-proven reachable ordering.

Therefore removing a new pending class on failure does not necessarily remove B's existing active class. The current active summary must not be described as authoritative runtime truth. Keeping A permanently active on every failure would also be false: existing runtime semantics may stop the previous runtime; no rollback may be invented.

## Minimal proposed resolution needing scope confirmation

Keep backend ownership, persisted state, activate(), effects, completion/Toast and IPC unchanged. Explicitly authorize a separate read-only Card active presentation projection from the existing observation appliedSubscriptionId/appliedConfigurationGeneration and lifecycle, alongside the independent pending prop. Do not mutate subscription.active, persist another active state, add fallback or create another operation state machine. A is shown active only while the existing runtime actually supports that projection; stopped/unavailable runtime must not be depicted as a retained ready A. Exact projection and direct-success observation timing require Candidate006 design/review after this source-of-presentation scope is confirmed.

This is beyond the currently stated 'only map pendingActivation' delta and prohibition on changing active ownership as presently interpreted; do not silently expand it. It need not change runtime/business ownership or Contract0.3, but requires the user to distinguish presentation projection from changing business authority.

## Impact and gates

Frozen Candidate005 focus-only write permission requires DCR023 for any pending TSX change. DCR023 is available but no approval-ready DCR/Candidate has been created while this scope conflict remains unresolved. Candidate005 binding and its approval remain historical/current frozen identity; implementation and matrix stay paused. After clarified scope: DCR023 -> Candidate006 -> design validator -> independent full bounded review -> exact Human Change/Design Approval -> legal rebind/checkpoints -> implementation/new target/verification -> pending native review -> later visual gates. No new Contract revision is proposed. No Task is materialized or completed.

Future pending precedence must retain primary2px/surface and padding compensation against hover; focus remains independent; disabled conflict locking remains as-is; loading differs from active and must not move content. Pending terminal cleanup must be verified together with active authority, not simply by absence of a pending class. The visual composition and terminal matrix are not frozen by this blocked analysis.

Existing 6000ms success Notice discrepancy and other C/N items remain outside scope. Shared CSS changes would require fresh affected visual identities/reviews; target008 and its native PASS remain history, not evidence for pending behavior. Subscriptions Golden Certification and TASK012 readiness remain paused. No native captures, production tests, implementation target or Human Approval are produced in this design-only analysis.
