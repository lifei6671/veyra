# TASK-019 Stage S scope remediation 005

Status: CANDIDATE pending exact Human Design/Adoption Approval. Candidate005 + DCR026revision007 is the current proposed scope overlay; Candidate004 stays historical approved provenance. This overlay supersedes only its Stage S page/editor readonly prohibition for the paths and deltas below, plus the bounded verification fixture. All other design semantics remain unchanged.

## Exact proposed implementation

- DocumentEditor.css: add border: 1px solid var(--divider) to .document-dialog only. Keep radius/background/editor geometry, fullscreen and all DocumentEditor.tsx behavior.
- SubscriptionPage.tsx: only dialog section class selection changes. Base subscription-dialog is ordinary; subscription-dialog-expanded applies exactly to create/edit, replacing the current qr-versus-other predicate. No added helper, mode, handler or field.
- styles.css: only Subscription Dialog width declarations: base width min(460px, calc(100vw - 24px)); expanded width min(520px, calc(100vw - 24px)). qr/delete/batchDelete/replace ordinary; create/edit complex. Preserve other selectors, scrolling, focus and busy behavior.
- SubscriptionPage.test.ts: only corresponding dialog-mode/width and required popup-border contract regressions, using existing test infrastructure. No new dependency; tests must protect emitted mode classes and CSS contract rather than changing handlers.
- scripts/sdlc-ui-contract.test.mjs: TEST_CONTRACT_VERSION='0.4'; invalid-version inputs ['0.1',0.2,'0.3']. Explicitly retain old0.3 rejection and valid0.4 acceptance. No evaluator edits, no Golden/human/source identity/stale/coverage relaxation.

The existing Stage S Toast implementation and R01-R10 intent are unchanged. This does not permit Subscription business, DocumentEditor.tsx, activation, IPC, runtime or Stage A/B writes.

## Identity and adoption

Candidate005 carries all Candidate004 manifest inputs unchanged as provenance plus this overlay, Impact010, DCRrevision007, Approval004 and Review009. ui_contract/ui_scope/visual_gate/ui_stages and source_paths are identical to004; scope changes only by the explicit bounded writes. Candidate target hashes its compact JSON excluding target_identity. Proposed Task snapshot is hashed separately to avoid circular identity; canonical TASK-019 stays BLOCKED at004 until exact adoption. Human approval must bind Candidate005 and snapshot bytes; after approval copy that snapshot byte-for-byte to canonical Task, rerun design/readiness/S checkpoint, then lifecycle updates by Orchestrator. This turn performs design checks only, not readiness or implementation.

## Verification and certification

Adopt the complete existing StagePlan003 and Candidate005 Stage S pages/components/states/rules/source_paths, without narrowing. Fresh post-remediation source identity uses sha256(JSON.stringify({git_identity,manifest})) with ordered path/sha256 entries. Tests additionally bind test-file hashes. Full pnpm test cannot waive the fixture failure. Required targeted Subscription/Dialog and all Toast tests, lint/test/build/diff; fresh full Native Light/Dark at1280x720@100/125/150 and1440x900@100/125, six Dialog modes, DocumentEditor border, Esc/focus trap/return/busy/fullscreen regression, Toast expiry focus and all Stage S states precede current Compliance and independent Delivery/CSS/token/composition Review. Only both PASS permit Human Visual request; real Human Visual Approval is required for certification. Existing reserved certification-001 remains absent until actual completion. Later A/B shared-source changes retain the approved versioned recertification/adoption model. No old target009 or TASK018 evidence is promoted to current PASS.
