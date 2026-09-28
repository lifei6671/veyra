# TASK-019 two-level verification impact008

Status: DESIGN ONLY. Candidate007 / Review019 are reviewed-but-not-adopted provenance. Canonical Task remains Candidate006. USER now authorizes a new Verification Contract Change Control and carries forward only Candidate007's S-only toast-persistent removal.

## Current authorities and conflicts

- Contract0.4 §10 defines five logical viewport/DPI environments; §11.3 requires Light/Dark and Native state screenshots; §11.4 requires human-owned source/compliance/screenshot-bound approval. §13 and StagePlan003 require S→A→B Golden dependencies. StagePlan003 currently repeats the full environment matrix at stage certification and requires current source recertification when shared files change.
- scripts/sdlc-ui-contract.mjs evidence():129–161 uses the same screenshot/state/environment checks for pageEvidence and Task final evidence. Theme-normal coverage and five viewport/DPI checks are separate existential checks: the current code does not require all ten Light/Dark × environment combinations per page. Final task state screenshot checks also currently repeat the full state union in final screenshots.
- pageEvidence():174–219 validates current manifest, scoped source identity, recursive prerequisite hashes, scope, Compliance and Human approval. Its generic prerequisite reader does not distinguish reference certification from full-page certification. A reference certificate must not become a weaker external Golden dependency.
- stagesFor():222–252 validates stage order/scope union. Final compliance without a stage is currently rejected for staged Tasks (:293); final delivery (:308–310) checks every stage plus whole-Task evidence. Two-level final Compliance needs an explicit final route without weakening stage selection for checkpoint.
- scripts/sdlc-ui-contract.test.mjs compliance():31–58 constructs five screenshots with Light only at index0 and Dark at other entries, assigns all states to each synthetic image, and shares that helper across stage/final/standalone tests. This fixture must be separated into reference-state and final-matrix fixture construction after approval. Synthetic approvals remain disposable unit-test data only.

## Decision and scope

Propose Contract0.5: for the explicitly declared TASK-019 staged model, Stage Reference Validation at Windows Tauri / Light /150% /1280×720 logical viewport covers every actual stage state and behavior, including keyboard/focus, errors/recovery and Dialog interactions. S/A reference Golden certificates unlock only their next same-Task stage. Final Visual Matrix covers all three pages at all five environments in both themes on final stable source, with final Compliance/independent Delivery/Human acceptance before DONE or TASK-020.

No state×theme×DPI Cartesian screenshot requirement: reference captures cover complete state/behavior; final captures cover each page's ten normal layout environments plus concrete visual regressions identified by acceptance/review. Final state acceptance is traced through current reference certificates, not silently dropped. Both levels retain real Native evidence, full scope and Human Visual approval.

Only Candidate007 semantic carry-forward is removal of toast-persistent from S states; overall/A/B retain it. Stage S Toast/RecoveryAction/§6.7/shared typed action automated/source review stay. No Subscription lifetime or recovery product change, no N/A/waiver/bypass, no Mock substitution. Product allow/deny, Contract visual tokens and R01–R10 are unchanged.

## Affected artifacts and deferred implementation

New candidate Contract0.5 and exact .4→.5 delta, DCR026revision010, evaluator/fixture design008, Candidate008 and Task snapshot008 are proposed. No edits to binding Contract0.4, evaluator, test fixtures, product source or existing Evidence this turn. The minimal future infrastructure write scope is scripts/sdlc-ui-contract.mjs, its existing test fixture, and the UI Gate protocol documentation needed to describe the approved model; no new tool/framework/CLI. Product source target018 remains exact.

Gate effects: Candidate007's pending adoption is superseded as a pending proposal, not revoked approval. New Technical Design/Contract Human gate is pending. Readiness/checkpoint/Native/Compliance/Delivery on new policy are NOT_RUN. Old approvals remain historical; no scope adjustment upgrades Native014 FAIL, partial018 INCOMPLETE or compliance018 FAIL.

## Source freshness and shared changes

This is environment scheduling, not a stale-source exception. Existing whole-file source freshness remains. Later A/B changes to shared CSS/App/provider may require new reference-environment recertification and new Human approval for affected upstream pages, following existing immutable reference adoption. They do not trigger a repeated full environment matrix at each stage. The full final matrix runs after final source stabilizes; any later relevant source edit invalidates its evidence and requires affected final verification again.

At final delivery every page manifest is current, and every current stage reference manifest is an exact matching subset of the final source manifest. The final union identity and current reference certificate hash chain bind all three pages. Historical state screenshots cannot be relabeled current. TASK-020 remains locked until complete TASK-019 acceptance.

## Current evidence reuse boundary

target018 source sha256:e1cae12a1f4bddc4bb7933c780faf5db74cbb019746ffb4c68a9ad3057533b34 and 16 source/3 test hashes remain untouched. automation018 focused75/full273/lint/build/diff and code-review018 retain original source/test-partition meaning. Review019's current-source applicability finding is provenance, not approval of this new policy. New independent Review must distinguish product evidence reuse from NOT_RUN future evaluator/fixture tests and new policy-bound Native/Human evidence. No older Contract0.4 approval certifies Contract0.5.
