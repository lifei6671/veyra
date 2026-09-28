---
id: DCR-021
status: PROPOSED
change_source: USER:lifei TASK018-HV-001
affected_task: [TASK-018]
---
# DCR-021: SubscriptionCard active selected language

## Impact / decision proposed
Human Finding .sdlc/evidence/TASK-018/human-visual-finding-001.json (sha256:d1e12cf78f4c85d7209177d4ce191c49c9312b33f4370ba0434876cadea5a127) rejects current selected background + check. This is a component-specific visual Requirement/Acceptance change to Frozen UI Contract, requiring DCR and explicit Human Change/Design Approval; no business/architecture/API/runtime Material Change.

Current canonical docs/ui/veyra-ui-spec.md 0.2 identity sha256:bb8783d3ad64cd01fe6f98e53082a794be2adeb10617f2826bc90b976d0751b6 remains APPROVED/BINDING. Exact historical snapshot: .sdlc/evidence/TASK-018/ui-contract-0.2-historical-snapshot.md (same bytes). Proposed0.3 .sdlc/design/veyra-ui-spec-0.3-proposal-001.md (sha256:2ed79ce3423ae7eb657f456dfe3f10f1c2772421eeadd3f6d752a8e2a1aba6a1) is inactive; not another binding Contract. Only §5 Row selected clarification/new SubscriptionCard row and §8.2 active rule change, plus version/status/example metadata. Other Row/Sidebar/NodeRow/Routing/Menu behavior, tokens, Contract rules stay unchanged.

New SubscriptionCard active indicator: primary border2px only, content surface background, no check/glyph/Badge/title-primary/shadow/left stripe. Hover must preserve border; keyboard focus remains separate. No business state/handler/mutation changes. This supersedes original SF038 expected only after approval, not a false closure against old0.2. Other37 Findings and SF036 DCR020 scope correction remain unchanged.

## Minimum implementation design (future approval only)
See .sdlc/design/TASK-018-active-card-design-003.md. Production write permission for this delta is src/styles.css SubscriptionCard active/hover/focus and border compensation only. No Card DOM/content, TSX, handler, active calculation, Dialog, Notice,6000ms, Sidebar/Proxies/Routing, backend, token or unrelated polish changes. Current four-file implementation is retained; no new implementation target this turn.

## Contract version mechanism and gates
The existing validator reads only canonical APPROVED/BINDING version. It has no future-Contract preview/revision API. Do not modify validator/core protocol or use a fabricated APPROVED temporary Contract.

Two-phase adoption is therefore required:
1. Independently review DCR/Contract proposal/design semantic delta; prepare Candidate003 with intended ui_contract.version0.3. Live design validator MUST fail version mismatch while canonical remains0.2. This is an explicit activation dependency, not a waiver or Task Design Gate PASS.
2. USER approves exact DCR/proposal/design/Candidate package. Record genuine approval against proposed identities. In a later authorized adoption, preserve0.2 snapshot, publish only canonical docs/ui/veyra-ui-spec.md as0.3 with APPROVED/BINDING status, calculate its new published hash, and regenerate Candidate manifest/requirement identity to that actual published file. This changes Candidate identity and requires matching independent Review and Human Design Approval for the published execution candidate; do not reuse approval for a changed hash. This round's Candidate003 is not materializable.
3. Only after matching published Contract/candidate review+approval: bind Task, readiness/checkpoint, implement scoped active presentation, new target/closure/verification/FULL_SCOPE review/native/Human Visual Gate. No automatic approval, no screenshot/implementation this round.

This additional identity step preserves existing protocol; it is preferable to changing validator or silently editing an approved artifact. Current source Review005 remains historical PASS under0.2. Existing target sha256:ac954467fe73906e97d1a9da5e9ce13bfda1761138afc144fd717933e30d089d, closure and native screenshots remain unchanged; none proves revised rule. New0.3 visual identity invalidates use of old Review/screenshots for current acceptance without altering their historical results.

## Downstream / compatibility / rollback
TASK012 Candidate005 still binds0.2 and old external Subscriptions certification reference. After canonical0.3 activation, it cannot re-enter Gates until a separately approved candidate/reference update; do not edit Candidate005/TASK012 here. Business Candidate004/Domain/Runtime/Compiler remain untouched. Golden chain remains Shared Foundation -> Subscriptions page-specific audit/remediation and full certification -> TASK012 external prerequisite -> Proxies -> Routing; not automatically approved by this change. No existing Golden success is assumed; all current identities remain history.

No data/deployment migration, API or runtime change. CSS border compensation preserves border+padding footprint; reverting presentation also requires refreshed visual evidence. Pause native collection now; Task acceptance and Delivery remain PENDING. Human Finding is rejection/design input, not Human Visual Approval.
