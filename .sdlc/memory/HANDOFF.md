# Project Handoff

Project: PROJECT-001
Phase: EXECUTING
Mode: ingest
Task: TASK-006
Requirement Source: `docs/veyra.md`
Requirement Identity: `sha256:13a1769134ecaa9a1361ff9571513ee2b2a662a7771a056ac974e5bd078349ae`

## Current

The accepted plan uses a versioned JSON `AppState` and `StateStore` for configuration.
Windows is the V0.1 desktop target. macOS platform realization is V0.2, based on DMG
direct distribution, a normal Tauri desktop app, and a sing-box sidecar; System Proxy
and explicit privilege remain Adapter concerns, while NetworkExtension is a future ADR
route. TASK-001 through TASK-005 are DONE. TASK-006 is the current BLOCKED Task for real
sidecar/Clash API observability and final E2E; Mock evidence remains non-substitutive.

The first frozen TASK-005 candidate was independently reviewed as REWORK. `FINDING-TASK-005-001`
is a P1: the Mock Port Delta is not bridged to the fixed Tauri event, and hidden-window rendering
has no visibility gate. Await explicit user authorization before remediation; the candidate must
be re-verified and independently re-reviewed after any repair.

User authorized that remediation and Windows GUI-only startup. The new frozen candidate is
`sha256:03cf332021191ee1d93f45d974ca3f17cb6834df241268c2c94878a7957fde09`:
the controlled bridge drops hidden-window Deltas without backlog, restore emits the current safe
Snapshot, and Debug/Release PE headers both report GUI subsystem `2`. It awaits independent
incremental review. Actual Windows UI Automation remains `UNABLE_TO_VERIFY`; real sidecar/Clash
API/System Proxy evidence remains `NOT_RUN`.

The independent incremental re-review passed with no P0-P3 findings and marked
`FINDING-TASK-005-001` fixed. User accepted TASK-005, which is now DONE at
`sha256:03cf332021191ee1d93f45d974ca3f17cb6834df241268c2c94878a7957fde09`.
Windows UI Automation remains `UNABLE_TO_VERIFY`; real sidecar/Clash API final E2E remains a
separately authorized `NOT_RUN` requirement. The project is now planning M5's next Task and must
not start a real sidecar, access a Clash API, or change System Proxy during planning.

User approved DCR-001 to move the managed runtime baseline to official Windows amd64 sing-box
`1.14.0`, archive SHA-256 `3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6`,
fixed backend-only `127.0.0.1:9090` Clash API, per-instance secret, direct `reqwest 0.12.28`
with `json`/`rustls-tls`, and a no-SystemProxy/TUN real E2E. The asset has not been downloaded.
DCR-001 reopens the Technical Design Gate against Foundation identity
`sha256:577e12b7abb56d195f55be7818290d0528ef03bbb9838854c5fe047503b326f0`; an independent review
recorded `EVIDENCE-DESIGN-009` as the candidate PASS and `EVIDENCE-DESIGN-010` as the final frozen
identity PASS, both with no P0-P3. User approved the Human Technical Design Gate; TASK-006 is now
READY/EXECUTING within the approved asset, direct HTTP and loopback API boundary. System Proxy, TUN,
UAC, WFP, Service and arbitrary UI/network capability remain out of scope.

User then approved `sha2 = "=0.10.9"` with default features disabled, solely to calculate archive and
executable SHA-256 locally. This amends the frozen Foundation dependency contract, so the Technical
Design Gate is reopened at identity `sha256:f0adffcc957c90fb99c9b873348c39b3d3cbe9020c36047781d524729e6da71a`.
TASK-006 is BLOCKED until its independent review and a new Human Gate approval complete; no Cargo
manifest/lockfile change has been made for this amendment.

The independent merged review recorded `EVIDENCE-DESIGN-011` as PASS with no P0-P3 for the sha2
amendment. The only remaining blocker is the new Human Technical Design Gate approval. A prior
approved identity probe ran `sing-box version`; no sidecar service has started and no Clash API has
been accessed. Further implementation verification remains prohibited until that approval is recorded.

User approved the sha2 amendment's Human Technical Design Gate and final identity review recorded
`EVIDENCE-DESIGN-012` as PASS with no P0-P3. TASK-006 is READY/EXECUTING within the approved
asset, direct HTTP, offline hash and loopback API boundaries. No sidecar service or Clash API call
has yet occurred; System Proxy, TUN, UAC, WFP, Service and arbitrary UI/network capability remain
out of scope.

User approved `getrandom = "=0.4.3"` exclusively for a per-instance 32-byte API-secret entropy
source. It reopens the Technical Design Gate at Foundation identity
`sha256:b0cb498f3b01db90aa471ee89ff16e7f9e03dd404b661bbc458e3113039c9682`; the independent review
recorded `EVIDENCE-DESIGN-013` as PASS with no P0-P3. TASK-006 remains BLOCKED only pending the new
Human Technical Design Gate; `getrandom` has not been added to Cargo.

The Human Gate was then approved. `reqwest 0.12.28`, `sha2 0.10.9` and `getrandom 0.4.3` are now
direct dependencies and `cargo check --manifest-path src-tauri/Cargo.toml` passed. TASK-006 is
READY/EXECUTING; implementation must next add fixed-resource integrity validation and the no-console
managed child path without reusing the System Proxy supervisor entry point.

User then clarified the product direction: users must ultimately select among verified sing-box 1.12,
1.13 and 1.14 cores, while current implementation and the first real E2E continue on 1.14.0. DCR-001
is now a CANDIDATE amendment defining a closed `CoreVersionCatalog`: no arbitrary version, URL, path,
hash or binary is accepted; 1.14.0 is its sole current Supported entry, and 1.12/1.13 require separate
asset, profile, `check`, API and Windows E2E evidence before they can be selectable. The user also
confirmed expanding the existing `windows 0.61.3` binding with ACL-only features. This invalidates the
prior Technical Design Gate; TASK-006 is BLOCKED in PLANNING until independent DCR review and a new
Human Technical Design Gate. Do not implement the ACL feature, package/start the sidecar, or access the
Clash API before that gate.

Independent incremental review `EVIDENCE-DESIGN-014` passed at Foundation identity
`sha256:101fe9258cc97c73b46c4d5f30ceadcf8ef13eca5d0480b2154486dc15dc3054`, with no P0-P3 findings.
The next and only blocker is the Human Technical Design Gate for DCR-001's closed multi-version catalog
and ACL-only feature expansion.

User approved that Human Technical Design Gate. DCR-001 is ACCEPTED and Foundation identity
`sha256:101fe9258cc97c73b46c4d5f30ceadcf8ef13eca5d0480b2154486dc15dc3054` is frozen. TASK-006 is
READY/EXECUTING: implement the 1.14.0 resource/ACL/sidecar/API path only. 1.12/1.13 remain future
compatibility lines and must not be exposed as selectable before their own assets and real evidence.

User corrected core asset delivery: `src-tauri/binaries/` must not be Git-tracked. DCR-001 is now a
CANDIDATE amendment: fixed assets are fetched, SHA-256/version/member verified and bundled only during
the controlled build; application runtime remains offline and cannot download a core. The Git-tracked
archive, executable and DLL are being removed from the index while local caches remain. TASK-006 is
BLOCKED in PLANNING pending an independent design review and new Human Technical Design Gate.

## Completed

- Completed the ingest Readiness Pass for `docs/veyra.md`.
- User confirmed the updated `docs/veyra.md` as the canonical requirement baseline.
- Updated the Intent Anchor from database-backed persistence to versioned JSON state
  with migration, overall validation, atomic persistence, backup and recovery.
- Recorded independent review `EVIDENCE-DESIGN-002` as REWORK and remediated its
  generated-config, private-file permission, migration-recovery, and capability
  baseline findings in a new candidate.
- Reissued `.sdlc/design/foundation.md` as a candidate with identity
  `sha256:eb5a87eb4024cbead4034e88615c39a32af9bb2eaedc8750224bc1ce7ff4f756`.
- Earlier review records remain historical evidence for their former Foundation
  identities and cannot support the current Technical Design Gate.
- Recorded `EVIDENCE-DESIGN-003`: independent re-review PASS with no P0/P1/P2/P3.
- Added Windows System Proxy snapshot/restore, low-privilege UI with explicit UAC,
  sing-box data-plane ownership, and V0.1 exclusions for Service, WFP, and full-system
  connection-to-PID scanning. The prior PASS review is stale for this new candidate.
- Recorded `EVIDENCE-DESIGN-004` as REWORK and added explicit PAC/WPAD proxy-state
  semantics plus serial CaptureMode transition and compensation rules.
- Reissued `.sdlc/design/foundation.md` as a candidate with identity
  `sha256:a619805ae8793db54d161e71fd34b716178be0d5e355711f2186c19e9e43f078`.
- Recorded `EVIDENCE-DESIGN-005`: independent Windows re-review PASS with no P0/P1/P2/P3.
- User authorized and applied a requirement-source revision in `docs/veyra.md` for
  Tauri/Platform ownership, System Proxy state, CaptureMode transitions, UAC/TUN,
  and Windows V0.1 exclusions. The document contains its own dated revision record.
- Recorded `EVIDENCE-DESIGN-006` as REWORK and aligned Auto Start with Tauri Desktop
  capability ownership.
- Reissued `.sdlc/design/foundation.md` as a candidate with identity
  `sha256:0e03bfcf5cf87f56a5c12ccd05c3fd9ef2627c32ed3c05bd73637b8c89ed6307`.
- Recorded `EVIDENCE-DESIGN-007`: independent source-sync re-review PASS with no
  P0/P1/P2/P3.
- User authorized a macOS V0.2 requirement-source revision. The source and Foundation
  identities changed; all earlier Technical Design review evidence remains historical
  for its former targets and cannot support the current Gate.
- Recorded `EVIDENCE-DESIGN-008`: independent macOS V0.2 source/Foundation review
  PASS with no P0/P1/P2/P3 findings.
- User approved the reviewed Foundation. The Technical Design Gate is `PASSED`, and
  `.sdlc/design/foundation.md` is frozen at
  `sha256:2da0f1f334126d978659a0389e7a4838732cfb810667001985f81381aaa9fd0f`.
- Created the rolling plan with TASK-001 current and TASK-002 through TASK-005 as
  lightweight future stubs. TASK-001 passed its contract/readiness check and is READY.
- Implemented TASK-001: a pinned pnpm/Tauri/React/Rust scaffold, one fixed typed
  bootstrap command, command-only main-window capability, Windows icon, and front-end IPC
  wrapper test.
- Recorded implementation verification and an independent delivery review. The first review
  found a P2 rendered-IPC smoke gap; the follow-up Windows UI Automation read `Veyra · ready`
  from the running WebView, and the incremental re-review passed with no remaining findings.
- No subscription, state persistence, database, sidecar, System Proxy, TUN, UAC, native
  Windows adapter, signing, updater, release, or macOS implementation was created.
- User accepted TASK-001. Its delivery gate records `approved_by: USER:lifei` and the task is
  now `DONE`.
- User approved `serde_yaml_ng 0.10.0` for TASK-002's Clash YAML `proxies` extraction only.
- Implemented TASK-002's typed `AppState`, JSON state store, V0 migration, atomic backup/recovery,
  subscription parser/normalizer and fixtures. Rust, frontend and scope checks passed; independent
  review was remediated and re-reviewed without blocker/major findings.
- User accepted TASK-002. Its delivery evidence is recorded under `.sdlc/evidence/TASK-002/`; M2 is DONE.
- Materialized TASK-003 and completed its readiness check for typed pools, routes, runtime intent,
  deterministic semantic compilation, and V1-to-V2 state evolution.
- Implemented TASK-003's typed pools, routes, runtime intent, deterministic semantic compiler,
  and V1-to-V2 state evolution. Rust (40 tests), frontend verification, and scope checks passed;
  the independent delivery review passed after the user-authorized Reality repair.
- User accepted TASK-003. Its delivery evidence is recorded under `.sdlc/evidence/TASK-003/`;
  M3 is DONE.
- Materialized TASK-004 for the managed sidecar transaction, Windows System Proxy three-state
  adapter, and Off/SystemProxy compensation. It intentionally excludes TUN, UAC, UI IPC, actual
  sidecar execution, and real proxy writes from implementation verification.
- User approved Tokio, thiserror, tracing, and the Windows WinInet binding for TASK-004. The
  sidecar partition implemented its closed candidate/active/previous transaction and passed seven
  focused tests. The concrete WinINet adapter is paused pending the additional GlobalFree feature.
- TASK-004's first candidate had a P1 in uncertain WinINet enable compensation. The
  user-authorized repair added `SafelyUnapplied` versus `StateUncertain` enable outcomes: only
  the former permits sidecar stop. Concrete Adapter/Supervisor tests cover readback, notification
  rollback, and stable-record failures; 69 Rust tests and independent re-review PASS. User
  accepted TASK-004; M4 is DONE. TUN remains explicitly excluded pending a separate ADR and Task.
  TASK-005 is materialized with Mock-only observability (A) selected but BLOCKED before
  implementation: it requires user confirmation of fixed new IPC/Capability permissions and
  Tray enablement. Real sidecar/Clash API observation remains a later independent delivery and
  required final E2E path.

## Remaining

- Implement TASK-005's Mock-only observability, fixed IPC and Tray lifecycle. Do not enter any
  real sidecar/WinINet operation without separate authorization.

## Blocker

No current blocker. Mock-only observability (A) is the active scope; real sidecar/Clash API
observation remains a later independent delivery and E2E requirement.

## Next

After the user selects the approved fixed IPC/Tray contract and observation source, return to
EXECUTING, run Task Readiness Check, and only then route implementation.

## Provenance

- Pre-existing user worktree changes remain out of delivery scope: untracked
  `.agents/` and `skills-lock.json`.

## Relevant References

- `.sdlc/state.yaml`
- `.sdlc/tasks.yaml`
- `.sdlc/tasks/TASK-001.md`
- `.sdlc/tasks/TASK-002.md`
- `.sdlc/evidence/TASK-001/implementation.yaml`
- `.sdlc/evidence/TASK-001/code-delivery-review.yaml`
- `.sdlc/evidence/TASK-002/implementation.yaml`
- `.sdlc/evidence/TASK-002/code-delivery-review.yaml`
- `.sdlc/evidence/TASK-003/implementation.yaml`
- `.sdlc/evidence/TASK-003/code-delivery-review.yaml`
- `.sdlc/tasks/TASK-004.md`
- `.sdlc/tasks/TASK-005.md`
- `.sdlc/design/foundation.md`
- `.sdlc/evidence/foundation/technical-design-review.yaml`
- `docs/veyra.md`
