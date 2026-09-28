# TASK-018 Candidate-011 bounded presentation verification design

Status: CANDIDATE; exact Human Change/Design Approval pending. Producer: /root.

Decision authority: DCR-025 revision002, only TASK018-DR015-001/002. Prior Candidate010 design remains in force except for those two decisions. Requirement source is docs/veyra.md; UI Contract is docs/ui/veyra-ui-spec.md Version 0.3. Existing reference mapping in design010 remains: Clash Verge Rev notification presentation/theme source; Veyra-specific safe content inset is an explicitly documented necessary difference prompted by actual obstruction, not a new palette or page design.

## Minimal implementation after approval

In styles.css change only the host top and corresponding max-height to the shared existing-token sum in DCR025 revision002. Keep right notice edge, flex column, notice gap, max-width, overflow, z-index, item identity and queue unchanged. A local CSS custom property on the host can avoid repeating the formula; it is not a root token or runtime configuration. Minimal item pointer-event adjustment is allowed only if needed, with dismiss behavior retained. Native rectangle checks must pass regardless of click-through.

In SubscriptionPage.tsx the existing current-dialog guard continues to precede each closing error. For result errors compute the existing mapped Notice; for exceptions use the existing response-unavailable Notice. Reset first, then call the existing `presentGlobalNotice` directly exactly once. A private presentation-only helper can accept that Notice and perform reset then emission. Do not route these paths through dialog-context dispatch. Do not change request functions, epoch/target identity checks, request sequencing, successful form population, finally blocks, business decisions or closing behavior. No new dependencies or test framework.

## Deterministic tests: actual async production handlers

Use existing Vitest and the repository's React hook/element test conventions. Drive the actual edit/QR menu callbacks and API mock promises through the production component, including settled state/rerender, rather than testing only a dispatcher/helper or source-string pattern. Deferred promises permit inspection of opened/loading Dialog before failure. For every row below assert final Dialog mode/target closed, no rendered or retained local Notice, exactly one global owner emission with kind error and exact baseline message, and reset recorded before emission. Observe request call arguments/order as an invariant. No copied implementation masquerading as a test subject.

| Case | API sequence | Expected message |
| --- | --- | --- |
| edit settings result-error | settings resolves existing business error | subscriptionErrorMessage(error), exact fixture text |
| edit settings exception | settings rejects | 订阅响应不可用，请重试 |
| edit remote share result-error | settings succeeds remote, share returns error | subscriptionErrorMessage(error), exact fixture text |
| edit remote share exception | settings succeeds remote, share rejects | 订阅响应不可用，请重试 |
| QR result-error | share returns error | subscriptionErrorMessage(error), exact fixture text |
| QR exception | share rejects | 订阅响应不可用，请重试 |

The extra edit-share exception row proves both awaits covered by the shared catch. Record fixtures' pre-change mapped literal message, not only equality to a helper called by both test and implementation. Also prove stale edit/QR completion after opening a newer Dialog emits nothing and leaves that Dialog intact. Negative regressions cover retained submit/validation errors and overlong QR local error, successful preload, and DocumentEditor contextual errors. Preserve existing queue, repeated-equal-message, stale timer/owner, waiting lifetime/promotion, multi-owner and Proxy/Routing success/global-close and error/local-exactly-once tests; execute these unchanged as targeted regressions. Do not weaken or delete existing tests.

Run project frontend lint, relevant Vitest tests and build after future code changes; broaden only if a failure justifies it. No Rust, kernel, backend or network test expansion. This design turn runs only identity/reference/Design-gate checks and diff whitespace validation.

## Native placement matrix after implementation

Actual Tauri/WebView contexts: Overview (App failure owner), Subscriptions (Subscription owner), Proxies and Routing (shared provider in each real page). Preserve all seven affected pages including Settings, Connections, Logs in shared scope; representatives do not certify those pages. Use real existing user action paths; record operation, resulting message owner and exact implementation identity. Deterministic mocks are not Native integration evidence.

For each of four contexts × Light/Dark × actual Windows DPI 100/125/150%, capture baseline and 1/2/3 live Toasts before expiry. Record actual DPR/DPI, CSS viewport and native window dimensions; verify current Contract window-size requirements without presenting browser zoom as Windows DPI. At every cardinality:

- Right gap equals notice-edge; first top equals the candidate shared inset; subsequent entries append below in FIFO with notice-gap. Existing entries' rectangles stay fixed on append, including overflow/scrollbar effects.
- Enumerate real Header, Toolbar and first-row primary controls before notification. Record their rectangles and every Toast rectangle; require nonintersection and visible labels. In Subscriptions explicitly include 导入, 新建 and Header actions, reproducing the target015 obstruction operation.
- At original main-action coordinates, elementFromPoint must resolve the control or its descendant; actual Native clicks must invoke that same intended action. Geometry remains mandatory even if item pointer-events is changed. Restore test state between clicks as needed without changing business outcomes.
- Compare page/container and SubscriptionCard rectangles to baseline (no layout shift) and confirm single-border active/pending arbitration and failure recovery remain unchanged. Ordinary content overlap is recorded without inventing an unbounded no-overlap requirement.

Record images, time-aligned rectangles, hit-test node ownership, click outcomes, owner/message sequence and target hashes. Preserve checks for fourth waiting and promotion, no waiting lifetime reset, stale timer/owner isolation and independent equal messages from Candidate010; fresh Native evidence and deterministic results remain separately labelled. A fixed inset adjustment requires a new implementation identity and rerun of affected full geometry matrix, not reuse of the failed target's PASS claims.

If Windows DPI/theme tooling is unavailable, mark affected cells UNAVAILABLE/NOT_RUN and leave UI acceptance pending. Historical target015 Light150 Subscriptions 1/2 failure evidence remains untouched. Design PASS does not close either delivery Finding: DR015-001 requires async branch evidence on the new target; DR015-002 requires fresh successful Native geometry/hit-test/click evidence. No Human Visual Approval or duration exemption inferred.

## Failure, rollback and review boundaries

If the static safe position fails required contexts, adjust only the shared fixed inset with evidence inside the authorized bound; if a broader layout requirement is necessary, report for separate design scope. If presentation change fails, rework only these branches/CSS; do not restore known error loss as a successful delivery. Any rollback restores the three changed files from their recorded pre-implementation baseline and leaves target015 failures documented. No data migration or persistence/security/runtime mechanism is involved.

After independent classification, formal Design validation and full bounded independent Design Review, request exact approval for DCR025 revision002 and Candidate011. Do not rebind TASK018 or edit source before that approval. Later readiness/checkpoint, implementation target, Compliance, independent Review and Human Visual Approval are separate gates. TASK012 readiness stays unchanged.
