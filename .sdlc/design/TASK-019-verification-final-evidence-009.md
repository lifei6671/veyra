# Verification design009 — immutable final evidence selection

Status: PROPOSED, no implementation. This bounded overlay supersedes design008 only for final evidence location/selection. Contract0.5 candidate008 and its exact normative delta stay byte-identical. All two-level environment/state/source/Human obligations remain.

Independent Review020 identified that existing evidence() defaults to fixed ui-contract-compliance.json/ui-human-approval.json paths. Those defaults cannot select a fresh append-only final attempt. Candidate008 remains reviewed REWORK, not adopted. Candidate007/Review019 remain reviewed-but-not-adopted.

## Frozen metadata and selection

Candidate/Task ui_verification contains model plus final_evidence:

~~~json
{
  "compliance_ref": ".sdlc/evidence/TASK-019/final-visual/compliance-001.json",
  "human_approval_ref": ".sdlc/evidence/TASK-019/final-visual/human-approval-001.json"
}
~~~

These are reserved immutable paths, not existing evidence or PASS claims. Do not create placeholders this turn. Task/Design comparison includes the complete object. For the new model require exactly these two fields with repository-relative paths in .sdlc/evidence/TASK-019/final-visual/, names compliance-NNN.json and human-approval-NNN.json, matching positive three-digit attempt number. Reject missing/unknown fields, cross-task/traversal/absolute paths, mismatched attempt numbers, mutable aliases/latest pointers. Other models retain existing policy. Selected files are ordinary immutable records; no alias/symlink indirection is permitted.

The final route reads compliance exclusively from adopted ui_verification.final_evidence.compliance_ref; it never falls back to legacy fixed paths. Both compliance and final delivery validate current final union identity, Contract/model/level, source manifest, exact current stage references, screenshot matrix, coverage, verification and independent review as specified in design008. Final compliance.review additionally contains sha256 of evidence_ref bytes; verify it as well as existing reviewer/target/coverage checks. The human compliance hash therefore binds the exact independent review bytes and reference chain.

Final compliance can run before Human: validate the declared human path shape but do not require/read/create the Human file. Final delivery reads only the predeclared human_approval_ref and checks genuine human ownership/decision, current final target, Contract/model/level, compliance_sha256 and full final screenshots. Missing Human fails delivery; no synthetic approval or provisional PASS. Human need not approve its own file hash, avoiding a circular identity.

## Append-only attempts and adoption

Once any final attempt record exists, never overwrite it. A failed/incomplete record, changed source, changed review/compliance or renewed Human decision requires a fresh unused matching numbered pair and immutable review/screenshots as needed. Preserve prior evidence outcome; no FAIL promotion. Create a new Candidate/Task reference revision and obtain the exact adoption required by the current protocol before evaluating that new final route. Selected attempt is derived only from exact adopted metadata; no highest-number discovery, mutable alias or caller override. A reserved path does not grant approval or evidence collection outside current authority.

This locator design is infrastructure only. Publication/evaluator/fixture/protocol implementation and post-publication exact adoption remain deferred as in design008. Canonical Task006 and binding0.4 remain unchanged.

## Additional focused fixtures (NOT_RUN)

- New final model without either ref, unknown field, path outside dedicated Task directory, mismatched/zero attempt, alias or legacy fallback: FAIL.
- Valid final compliance at adopted versioned path with no Human file: compliance can PASS when all other obligations pass; delivery FAIL.
- Valid records only at old fixed filenames or an unadopted newer attempt: cannot satisfy selected final route.
- Changed review bytes with unchanged compliance.review.sha256: FAIL; Human binds current compliance hash.
- Changed compliance bytes under existing Human approval: FAIL. A new current-source attempt does not reuse older Human approval.
- Exact adopted new numbered pair with fresh evidence can pass; prior pair bytes remain unchanged, no scan or auto-selection.

Use existing disposable-root fixtures only; no product/test source edits this turn.
