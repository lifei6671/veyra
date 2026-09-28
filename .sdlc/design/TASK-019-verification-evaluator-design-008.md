# TASK-019 two-level evaluator design008

Status: PROPOSED DESIGN, no executable evaluator or fixture changes this turn. Baseline scripts/sdlc-ui-contract.mjs and scripts/sdlc-ui-contract.test.mjs hashes are bound by Candidate008. This file is the reviewable evaluator change specification, not an implementation/PASS claim.

## Metadata and routing

Add one declaration to adopted Candidate/Task: ui_verification.model = stage_reference_then_final_matrix. Contract0.5 defines its fixed environments; no caller-supplied arbitrary matrix or waiver flag. This model is approved only for TASK-019 with ordered S=Subscriptions, A=Proxies, B=Routing and existing prerequisite graph. Unknown model, wrong Task/stages, cross_cutting_shared combination or partial declaration fails. Other tasks/standalone/shared paths retain their existing policy; no automatic conversion of historical records.

Task/Design materialization comparison adds ui_verification deep equality to existing ui_contract/ui_scope/visual_gate/ui_stages equality. Stage-state union remains exact. The declaration does not change pages/components/rules/source_paths. Candidate007's S-only toast-persistent removal is the only state-set change.

Use evidence verification_level to distinguish stage_reference from task_final_matrix. Require matching level and model in certificate/compliance/independent-review/human records; this is a positive evidence obligation, not an exclusion. Stage_reference records require a valid task_id+stage_id and the adopted Task's exact ui_scope; a standalone certification_id cannot claim reference status. The model is obtained from the actual adopted Task/Design, never solely from the evidence's own field. Unknown/missing/mismatched levels fail for the new model.

## Changes to existing functions

1. declaration/stagesFor: validate the explicit new model under binding0.5, exact TASK-019 stage/page/prerequisite shape and complete unchanged scope. Keep existing applicability, standard required states, visual_gate and union checks. R01–R10 and runtime remain outside evaluator mutation.
2. evidence: keep subject/Contract/source/compliance/reviewer/native-hash/frontend-command/human checks. Select the screenshot environment predicate from validated context. For stage_reference every submitted certification screenshot must be that page, tauri-windows, Light, width1280/height720/scale150, with current stage source identity and matching file hash. Require normal and every declared applicable state within those reference shots; independent review retains all actual behavior/focus/keyboard/error/recovery/Dialog acceptance. Extra historical/other-environment images stay outside that submitted reference evidence set.
3. evidence task_final_matrix: every page must have normal Native screenshot at each pair of theme={light,dark} and environment={(1280,720,100),(1280,720,125),(1280,720,150),(1440,900,100),(1440,900,125)}: at least30 distinct page/environment/theme coverage cells. Require each screenshot's current final source identity/hash. Do not require every state at every environment. Final checked_states/coverage still equal the whole Task obligations; source-current reference evidence supplies full state/behavior coverage. Final screenshot coverage checks normal at every cell plus any concrete visual acceptance observations required by Reviewer; final independent Review assesses cross-page interaction and final layout.
4. pageEvidence: preserve exact scoped manifest/current-byte/source hash/Git/prerequisite/human checks. Accept stage_reference only when called with authenticated same-Task stage context by checkpoint/compliance/delivery; recurse prerequisites with the same owning Task and expected producer stage/scope/model. Generic certification/externalPrerequisites callers reject stage_reference, even if kind is golden_page_certification. This prevents reference Golden from unlocking external Tasks or standing in for full certification. Do not change ordinary legacy standalone certification behavior.
5. Stage checkpoint/compliance/delivery: S still has no producer; A requires current approved S reference Golden; B requires current approved A reference Golden and transitive S. Stage compliance may run before its own Human approval, while prerequisite Human approvals remain mandatory. Stage delivery requires own current Human approval. Existing kind golden_page_certification for S/A and ui_stage_acceptance for B remains, qualified by verification_level=stage_reference; no new cert kind, alias or empty placeholder.
6. Task final compliance: for this validated new model, permit phase=compliance with no stage only through the final route. Validate all current S/A/B reference certificates including their real Human approvals and immutable prerequisite hashes, then final source/matrix/compliance/independent review, excluding only final Human approval until requested. Checkpoint without stage remains invalid. Other existing phase behavior is unchanged.
7. Task final delivery: validate those same current reference certificates, full final source/matrix/compliance/review, and final exact Human approval. Missing any stage/reference/matrix/page/theme/viewport/state lineage/final Human fails. This is the required UI result before Orchestrator can mark TASK-019 DONE or release TASK-020; evaluator still does not mutate Task state.

## Final source and reference binding

Final compliance includes git_identity and manifest over the exact union of Task ui_scope.source_paths and all stage source_paths, without omissions/duplicates. Recalculate target_identity = sha256(JSON.stringify({git_identity,manifest})) using stored order, check every byte current and compare the caller targetIdentity. Every stage manifest path/hash must match the corresponding final entry; stage source identities remain their own page-scoped hashes and are not coerced equal to the union identity.

Final compliance includes stage_references [{stage_id,path,sha256}] in S/A/B order, exactly matching adopted Task evidence_ref paths and current certificate file hashes. Final independent Review must bind the same list and final target; final Human approval binds final compliance hash and final screenshot list as already required, thereby binding the source and reference chain. Final snapshots cannot just reuse a stage source identity. Missing/stale/mismatched reference bytes or stale final source fail. Shared-source changes still require affected reference recertification/new adoption at the reference environment; no stale-source exemption is added.

## Golden and Contract publication boundaries

Reference Golden is only an intra-task unlock. Do not expose it as a standalone/global Golden or use it to unblock TASK-020. A later cross-Task full Golden is a separate explicit artifact/adoption requiring appropriate final-source full-matrix and Human evidence, not an automatic promotion of a reference certificate. That publication is not part of this change.

Contract0.5 candidate is NOT_BINDING. Current evaluator design check on Candidate008, which declares proposed0.5, must currently FAIL version mismatch against binding0.4; record that observed outcome rather than fabricating PASS with a synthetic approved Contract. Independent semantic Design/Contract Review can finish and request exact Human approval, but proposed-policy executable verification is NOT_RUN until separately authorized evaluator implementation/publication. No runtime “candidate Contract” override or N/A path is added to evaluator.

After exact Human approval: preserve approved bytes; publish0.5 with only approved publication metadata changes (body must match approved candidate); implement bounded evaluator/fixture changes and independently review/test them; create a post-publication adoption envelope binding actual published Contract and evaluator/test/protocol hashes, without modifying Candidate008 provenance. If publication/adoption identity changes, obtain exact adoption approval per current protocol. Only then canonical Task adoption, affected Readiness/S checkpoint and resumed Native. Ordinary user-authorized product work is not an excuse to use an unimplemented verification policy.

## Focused evaluator fixture plan (NOT_RUN)

Use existing Vitest temporary-root helpers only. Keep synthetic approvals/screenshots inside disposable fixture roots. Preserve legacy failure tests for hash/source/subject/scope/prerequisite/human and shared/standalone behavior. Do not globally rewrite all fixtures to0.5: retain legacy tests under explicit0.4 fixture context and add0.5 model fixtures; wrong-version tests remain meaningful.

| Case | Expected |
| --- | --- |
| S complete states/behaviors reference evidence, no Human | S compliance PASS; S delivery/A checkpoint FAIL |
| S complete reference evidence and real-shaped synthetic Human in unit root | S delivery/A checkpoint PASS only in fixture |
| Reference screenshot wrong theme/DPI/viewport/native origin/target/hash or missing state | FAIL |
| Missing RecoveryAction/Toast/§6.7 metadata or weakened union; A/B persistent removed | FAIL scope/materialization validation |
| Generic/standalone/external consumer uses a reference certificate; level changed only on envelope | FAIL |
| A reference missing S, B reference missing A or stale transitive S; prerequisite Human absent | FAIL |
| Current S/A/B references and30 normal final cells, state coverage only at references | Final compliance PASS, final delivery FAIL without final Human |
| Remove one theme/viewport cell from any of the three pages | Final compliance/delivery FAIL (table-driven all30 cells) |
| Five mixed-theme shots meet old existential checks but not all10 per page | New final route FAIL |
| Final record contains only reference shots or only two pages | FAIL |
| Missing/duplicate/foreign final source entry; stale bytes; stage manifest not subset; wrong final identity | FAIL |
| Wrong/missing final stage_references or stale reference/compliance/human hashes | FAIL |
| Shared CSS changes after S/A approval | A/B dependent entry and final delivery FAIL until fresh reference recertification |
| Final Human binds earlier compliance or omits final screenshot | FAIL |
| Missing model/new level, illegal Task/stage shape, no-stage checkpoint, shared-model mixture | FAIL |
| Unknown/new0.5 version used against binding0.4 or unapproved Contract | FAIL |

Later evaluator verification commands: pnpm exec vitest run scripts/sdlc-ui-contract.test.mjs and git diff --check, plus actual design/readiness/stage/final-route integration in disposable roots as covered by fixtures. Product automation018 is not evidence that these new checks execute correctly. No product tests need rerun in this design-only turn.
