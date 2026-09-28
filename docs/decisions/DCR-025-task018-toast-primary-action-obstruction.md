# DCR-025: TASK-018 Toast placement and closing-dialog error presentation

Status: PROPOSED / HUMAN DECISION REQUIRED. No design change or implementation permission is granted by this record.

## Current frozen authority

- DCR-024 revision002: `sha256:caf4b9ab34d33334e32bc433970fa977f456a5c4227d541d7b4ea95ea7bcf94e`.
- Candidate-010: `sha256:00a2dc101fb8a2864bce11cc0723c74c7eace394a2e8fc2b840a69c44714c018`.
- Implementation target-015: `sha256:b1613fc8ebc995b98440327f2cf587bc83780984181b29337fbda36a105b0a97`.
- Exact Human Change / Design Approval remains valid for that design. It is not implementation or visual acceptance.

## Observed conflict

Candidate-010 fixes the host at the WebView top/right using the existing 20px notice edge, appends downward with the existing 10px gap, and keeps items pointer-interactive. Acceptance also requires primary actions to remain unobstructed and forbids page layout shifts or page redesign.

Real Windows Tauri, Light, 1280×720 CSS pixels, DPR 1.5: an empty-link Import action emits the unchanged error message. The first Toast occupies y=20–62.177 and covers enabled header actions. The second occupies y=72.177–114.354 and covers Import/New at y=68–102. Both action-center hit tests resolve to the Toast. Four actual native mouse clicks emit only two notices; later clicks select Toast text. No injected business result, synthetic event, emulated theme/DPI or React state was used.

Finding: `TASK018-TOAST-NATIVE-001` / P1 / OPEN. Evidence:

- `.sdlc/evidence/TASK-018/toast-native-analysis-015.json`
- `.sdlc/evidence/TASK-018/native-toast-015/subscriptions-obstruction/trace.json`
- `.sdlc/evidence/TASK-018/native-toast-015/subscriptions-obstruction/frame-0004.jpg`

The same trace confirms stable first-item position when the second appends, unchanged page/Card geometry, and existing approximately 6000ms expiration. Those successful properties do not resolve the obstruction.

## Impact and bounded decision

Affected requirement: Candidate-010 primary-action reachability / Toast placement. Queue identity, FIFO, lifetime, owner integration, Dialog sinks and protected Card/completion semantics do not require redesign based on this finding.

The next bounded design decision must reconcile top-right placement with the occupied primary-action region. A presentation-only placement/avoidance amendment is the preferred scope to investigate; exact geometry must be validated before a new Candidate is frozen. This record does not invent an unverified safe offset. Moving page actions, introducing page migration, changing notice tokens, making text disappear, changing message lifetime or silently relaxing the reachability requirement is not authorized.

Simply making the Toast body click-through would not establish the existing visual non-obstruction criterion. Returning to a different corner would change the approved placement. Neither is silently applied.

## Closing preload errors

Independent source review also found that existing edit/QR preload failure paths execute `fail(...); resetDialog()` (or `failResponse(); resetDialog()`). Target-015 routes the first call through the still-open Dialog identity into local Notice, then reset clears it. The error is never visibly presented. Before this delta, the final closed-dialog render displayed the existing Notice globally.

The current design simultaneously requires emission-context routing, no replay of contextual Dialog errors globally on close, and preservation of existing transaction/close decisions. The bounded amendment must explicitly distinguish these existing closing preload failures from errors in a Dialog that remains open. Proposed decision scope: preserve their existing message and close outcome while specifying their single visible presentation destination. This does not authorize moving retained Dialog validation/submit errors or DocumentEditor errors to global Toast, or changing any backend/error classification/transaction decision.

See `.sdlc/evidence/TASK-018/toast-source-review-015.json` for the independent finding, exact paths and coverage. No silent routing exception or handler-close change is applied to target-015.

## Gates and recovery

Target-015 implementation and automated evidence are preserved; `pnpm lint`, 249 tests, build and diff check passed. Native/UI Compliance is FAIL for the observed finding; remaining native contexts and matrix are incomplete. The unavailable Windows Settings UI separately prevents current automated Light/Dark and 100/125/150% switching.

TASK-018 remains VERIFYING with Delivery PENDING. No Human Visual Approval, Golden Certification, Task completion or TASK-012 readiness is recorded. After the bounded placement decision: independent design review and exact required approval, Task rebind/readiness/checkpoint, minimal remediation, a new implementation target, relevant verification and fresh native review. DCR-024/Candidate-010/target-015 and all earlier evidence remain immutable history.
