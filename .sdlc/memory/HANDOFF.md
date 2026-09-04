# Project Handoff

Project: PROJECT-001
Phase: REMEDIATION
Mode: ingest
Focus: TASK-009 (IN_PROGRESS; DCR011 runtime positive-control premise failed; checkpoint014 independently reviewed REWORK)
Requirement Source: `docs/veyra.md`
Requirement Identity: `sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`

## Current

2026-09-04: User explicitly requested "先把所有代码提交一版", authorizing one local
snapshot commit of the accumulated code, workflow files and delivery evidence. This supersedes
earlier no-commit instructions for this snapshot only; no push or Task acceptance is authorized.
Pre-commit frontend lint/build and all50 tests, Rust fmt and library Clippy pass; all55 checkpoint014
source hashes remain unchanged. TASK009-CR-006 and the known real comparison failure remain open.

Checkpoint014 frozen identity: `sha256:37feddf0c296a6db8e7552d203c645efa5cd9663b4649243465ace58e3d5c126`.
55 source entries and52 raw verification artifacts freshly match. Verification014 SHA256:
597b315d8e9015e651ad36a348c8aeeead47751c099ec369c87018d2230ccd76.
User explicitly approved DCR011; CHANGE009/readiness014 and50-source wg-reject-baseline-014 preserve
scope and prior bytes. Existing approval persists; it does not authorize alternate addresses/listeners.

Implementation wrote exact cfg(test) positive rules, closed three-phase peer protocol and continuous
Rust targets. Scope result FAIL: real diagnostic run15.47seconds, phase1 virtual TCP/UDP exact echoes,
host127TCP/UDP timeout. Both target counters1, payload cells1/3 only, full bootstrap and health summary.
DUT38304 andpeer82164 exited; private configs0. Peer exit1/protocol cleanup remainsFAIL because the
three required phases were not completed. Protected phase2 and positive phase3 NOT_RUN, no rejectionPASS.
Fixed DUT gVisor drops raw127/8 destinations before Router; Router exception cannot create this positive.
Diagnosis `wg-reject-runtime-diagnosis-014.md` SHA ccd53d058b07b72c6139e7269f1c0e25e47179c5edd85162983863ec1eeb2b32
binds fixed-source chain and runtime logs. Its actual-host-IPv4 candidate is PROPOSED only; it adds
non-loopback exposure and requires a new reviewed/approved DCR. No new addresses/listeners were run.

39uniqueRust PASS and1FAIL;22Go top-level/60subtests PASS, race/vet/moduleverify/build/gofmt,
Rust final build/fmt/non-test libraryClippy/diff PASS. Same helper b22d775adda493abe3ce4ddf3f511009cd271babb5cd66617ae7293b613792e9:
real TCP/ICMP23.66seconds,UDP23.46seconds,oldHold55.42seconds,newHold150.37seconds PASS.
Hold tests use realhelper but simulatedDUTstop=false, no liveDUT. NewHold confirms both owned targets
andpeer retained at3/148seconds, hardexit1 at150.062 and release by150.217; cleanup remainsFAIL asrequired.
Selected host interfaces/addresses/routes/DNS/proxy beforeafter hashes unchanged. Failed fixture dirs
remain for diagnostics but DUT privateconfigfiles empty; no cleanup of unrelated historical fixtures.
Compiler30tests unchanged; finalPort SHA3ab3a9e740ad36ec0ed1c6d23cb5dd4056b6d2ee4f522d86e1ab6956e05c31e3.
Initial Windows worker-handle timeout and Go context cancellation vet warning fixed and reverified;
new real comparison remains failing, not ignored or weakened.

Independent `task009_design_reviewer` reviewed frozen014: REWORK; OPEN P1 TASK009-CR-006 confirms
DCR011 host positive-control premise failed, and corrects the prior design review. No additional
implementation findings; CR005 remainsFIXED. Review SHA256:
f35aafdd16e69174319f0a6b06e76285d658fa04314c2f765f2caadeeb9590b6.
Technical design Gate is
PENDING due disproven runtime premise; delivery/Task acceptance remainPENDING andTask009IN_PROGRESS.
After the authorized local snapshot, next technical-design remediation, respecting fixed EXE and no
host settings; do not startTask010, push or claim complete SF003. Full DNS/forwarding/IPv6/native
obligations remain unfulfilled.

## Historical checkpoints and decisions
User explicitly approved DCR011 with 允许. CHANGE009 binds its unchanged54c01cbc... candidate.
Readiness014 PASS; all50 previous sources verified and copied to wg-reject-baseline-014 before Task sync.
Root owns compiler cfg(test); Rust worker Port tests; Go worker peer module. Real core runs root serial.
The historical proposal/approval-pending notes below are superseded by CHANGE009; no repeated approval.
Checkpoint013 is the prior frozen baseline; current implementation requires new identity and review.


DCR011 `.sdlc/design/DCR-011-test-wg-local-reject.md` candidate identity:
`sha256:54c01cbc8f99b080a2d68e5b999ab6fc9f97fe49b1ae30e53e36ae2d402f6641`.
Independent `wg-reject-design-review-001.yaml` PASS, no findings; review SHA256
abd7bbce2d0a5ed100782157a4a3716857004e4f0a20e5444168338f5307dde6.
Four cases: authenticated peer→virtual198.18.0.1/host127.0.0.1 TCP/UDP. Same two owned loopback
TCP/UDP echo targets and samepeer/keys persist across positive/normal-protected/positive DUTs.
Each new DUT first initiates URLTest bootstrap so peer learns new WG endpoint; exact fixed allow
prefixes exist only in cfg(test) positive controls, all original rejects remain. Only one Direct user
rule Port([tcp_port,udp_port]) follows original reject; protected config uses normal compile.
New resources/contract pending approval: two loopback targets, exact allow prefixes, new test scene
with40second phase/135work/150helper hardlimit/159parent final/160outer limits. Existing scenes keep55.
No implementation/Task/requirement/dependency/host change or real network test in this design turn;
all50checkpoint013 source hashes and46verification artifacts checked unchanged. This candidate does
not cover DNS, non-host forwarding or IPv6; those Task obligations remain. Await explicit approval,
then synchronize CHANGE/Task/readiness and implement this increment without repeating old approvals.
Peer feasibility source: `wg-reject-peer-feasibility.md`, SHA256
0cae6cee398f3c1f6874c2271015464a196822db56730287d682de5049bf869a.
It confirms required exact127.0.0.1/32 WG cryptokey route and test-scene-only gVisor loopback option.
Design PASS is not runtime evidence. The current technical-design Gate targets DCR011 and is PENDING;
the accepted DCR010/CHANGE008 and checkpoint013 delivery below retain their existing bounded scope.

Checkpoint013 frozen identity:
`sha256:5f6438663e7d08e0241fde9a733e466c2dc7031dba31dbe050c1b3f36e52f0d5`.
50 source entries match; verification013 sha2b54070715b3634efaaed5a4dc59503af79c20aac085f180e068737e31de3568
binds46raw artifacts, all freshly checked.32Rust tests and16Go top-level/38subtests PASS, Windows Go race,
vet/module/build/gofmt, Rust non-test libraryClippy/fmt/diff and selected host before/after hashes PASS.
New helper SHA70850e6deaf35e6da560dc55b30690d5648aadb9c1d77e0a53bc7505796a4d55.
Real UDP21.40seconds: same client receives3exact20byte datagrams,60request/60response bytes, peer WG
rx3/tx3 authenticated; exactloopback business inlet plus existing WG protocol group, ownedDUT/peer.
Client then DUT then peer cleanup confirmed. Same new helper TCP/ICMP21.05seconds PASS full204ACK
rx4/tx4 and3IPv4ICMP. Hold regression55.46seconds uses realhelper with simulatedDUTstop=false,
retains UDP binding/stdin at3.057/53seconds then actualhardexit1; cleanup remainsFAIL as required.
No actualOS stop refusal is claimed. Independent code-review-checkpoint-013.yaml locally PASSes the
entire CHANGE008 increment with no new findings; CR005 remains FIXED. Review SHA256:
e1f0be1c2496775730fbcc1e4ed8b0076454acf93c62545df7d2203fe79365f0.
WholeTask review remains REWORK/INCOMPLETE, Gate and human acceptance PENDING. No source still being
edited; no new dependency,lock,product inlet or host settings. Next: concrete full reject/forward topology
with same-path positive controls; controlled DNS still requires supplied domain and diagnostic permission.
No duplicate UDP approval, Task010, CI, commit or push. The earlier planning/implementation notes below
are historical; the current completed UDP scope and remaining gaps are those stated above.

User explicitly replied "允许" to the reviewed DCR010 proposal; CHANGE008 now records its exact
36a910c9... identity. Readiness013 PASS, old47 sources preserved in wg-udp-baseline-013 before scoped
Task synchronization. No duplicate approval is needed. Root owns compiler cfg(test); Rust worker owns
Port wg_peer_test; Go worker owns existing peer module without dependency/lock changes.
Only root runs real-core tests serially after final binary build. The candidate's PROPOSED bytes below
remain immutable provenance; current authorization is CHANGE008, not the older pending narrative.
Delivery012 remains historical while the new source is being implemented; no UDP PASS is claimed yet.

DCR010 candidate `.sdlc/design/DCR-010-test-wg-udp.md`:
`sha256:36a910c90c247e1b46958ed7890dceabc7d769aee1ca1f18b062154de8ea9c79`.
Independent `wg-udp-design-review-001.yaml` PASS, no findings; review SHA256
`3f37478bfb514e861d87e4d910d35aca5dd6ce453f3cc4ca43a4492654472899`.
Only new approval requested: cfg(test) fixed loopback UDP business inlet to peer in-memory198.18.0.2:18081,
closed init_udp/udp private protocol and matching tests. No new dependency or host setting; reuse CHANGE007
WG resources and checkpoint012 Hold lifecycle. Three exact client-received UDP echoes and WG boundary
summaries are required. No UDP implementation or network test ran this planning turn. All47 checkpoint012
source hashes and requirement identity remain unchanged; Task acceptance and delivery remain pending.
Current technical-design Gate targets this new candidate and is PENDING only for its explicit user approval;
the existing DCR009/CHANGE007 approval and prior bounded delivery evidence are not revoked.
On approval, record exact candidate identity, synchronize Task scope/readiness, then implement and verify.
Do not repeat resource approval for existing TCP/ICMP or interpret this candidate as full reject/DNS approval.

Parallel DNS resource proposal: `.sdlc/evidence/TASK-009/dns-verification-proposal.md`,
`sha256:a2f62a565c3915f9b777a2cc4731eb015e8405221cf74018a99bc2ec837fff53`.
PROPOSED/NOT_RUN: requires a controlled subdomain returning loopback/virtual A records and explicit short
DNS Client ETW diagnostic permission while DUT remains unelevated. Current provider/schema/token were
read only; Operational is disabled and ordinary token lacks conventional collection permission.
No DNS query, trace session, settings change, dependencies or external service was started.
The proposal has not undergone independent review and is not an authorization or runtime evidence.

Current checkpoint012 identity:
`sha256:1827c6b72fe6466836eb4e6d4fd32e006a36e581d07c8731ed6757c59b17fc59`.
Checkpoint011 review found unique P1 TASK009-CR-005: finish(false) killed peer before DUT Stop was
confirmed. Only Rust Port cfg(test) changed: false branch sends no shutdown/kill and retains stdin
until actual peer hard exit; parent pipe deadline59seconds avoids preceding Go's55second hardlimit.
It always returns cleanup failure. wg-repair-baseline-012 preserves all47prior source bytes.
verification012:3Rust tests PASS. Real helper with simulated dut_stopped=false (no DUT launched)
keeps UDP binding and stdin at3.179/53.001seconds, exits code1 at55.173seconds and releases exact port;
fixture55.64seconds, no fake clock or shortened timer. This is not evidence of actual OS Stop refusal.
Normal real WG TCP response ACK(rx4/tx3),3IPv4ICMP and DUT-then-peer cleanup PASS31.91seconds.
Protocolnegative/libraryClippy/fmt/diff and before/after host config hashes PASS. Same final Go helper
57a95f5c... and all13Go files unchanged; Go tests/race/vet/module/build coverage carried from011.
Independent checkpoint012 repair review PASS; TASK009-CR-005 FIXED, no new findings. Review hash d3c6446a0b1c1c4684eae27c082ed40aa032f2f34048517eb47b2791dd58420b. FullTask still REWORK/INCOMPLETE; full reject/forward,
DUT UDP business entry and controlled systemDNS remain. No Task acceptance,TASK010,CI,commit,push.

Previous checkpoint011 identity:
`sha256:980308083ee557c8eafcbfc6f6526bf326b28afde88b17c3e24af042a780a642`.
CHANGE007 explicitly authorizes proposal ee61ea74...; DCR009 b193f16e... independently PASSed review002.
readiness011 PASS.13new Go files implement fixed-dependency standalone test peer; only Rust Windows
Port cfg(test) integrates it.47manifest entries include prior33 plusGo13 andDCR009; prior Task bytes
are preserved before its Change007-only additions. Original source baselines/authorship remain intact.
Final helper SHA25657a95f5c7382df6b20583c45eaf41068a62162ac70abff593c744d8e57dcad57 is built and
bound to actual real test. verification011:2Rust tests and11Go top-level tests PASS (one contains11ACK
subcases); Windows race/vet/module verify/build/fmt/libraryClippy/diff PASS. Final real run29.16seconds:
normal typed WG compile/finalize/check/run, complete204response acknowledged(rx4/tx4),3matchingIPv4
ICMP Echo replies, confirmed DUT-then-peer cleanup and no owned sockets/private instances afterwards.
DUT actual UDP bindings wildcard IPv4/IPv6 same dynamic port; peer only127.0.0.1 UDP and no OS TCP.
Selected host interfaces/addresses/routes/DNS/proxy before/after hashes unchanged. TCP ACK proves
transport delivery,not URLTest application consumption. The new helper must be explicitly built before
running the Rust WG test; tests do not auto-download/build it. See scripts/task009-wg-peer/README.md.
Initial helper PASS is historical; final helper initially hit the3second Windows CIM query timeout,
with cleanup confirmed. Candidate identity db3f5389... is preserved at wg-delivery-candidate-001.yaml.
The local query budget is now5seconds from pre-spawn and still inherits the45second work cutoff;
no automatic retry or business I/O timeout change. Final real test passed after this bounded repair.
Independent checkpoint011 review REWORK for unique P1 TASK009-CR-005: finish(false) kills peer before unconfirmed DUT Stop. Other partitions have no new findings. Rust-only bounded Hold repair is underway, baseline wg-repair-baseline-012 retains47sources. FullTask remains REWORK/INCOMPLETE/IN_PROGRESS; full WG
reject/forwarding, DUT UDP business entry and observable system DNS topology remain separate work.
No Task acceptance,TASK010,CI,commit or push. Earlier checkpoint narratives are historical scopes.

Previous checkpoint010 identity:
`sha256:ce5ac15928e55ffed92739e1873baf9f84bf6919a2fc0679d39efcc01243bfc2`.
Only test instrumentation/cases in Clash API and Application worker changed; production behavior,
requirements, design, dependencies and frontend are unchanged. All33 source hashes verified.
verification010:24 unique tests PASS(11 controller+13 API), non-test library Clippy/fmt/diff-check PASS;
selected host interface/address/route/DNS/proxy configuration hashes unchanged. Standalone test overlaps.
The real Traffic socket future reaches Poll::Pending; a bounded test observer holds that poll boundary
while Stop is queued. Old reads finish and sample publishes before Stopped; no event/revision follows
for1.2seconds. Same worker then starts a fresh config/secret with cleared history and a new-only sample.
Traffic/Logs client-operation max1 each,final0,started=finished2 each. These are client attempts including
handshake/disabled Logs empty replies,not server live WebSocket counts or enabled Logs stress.
Old work finishes before Stop/restart; no artificially delayed response after replacement is claimed.
Independent checkpoint010 increment review PASS with no new findings; whole Task review remains REWORK, coverage INCOMPLETE and Gate PENDING.
inflight-baseline-010 preserves prior33 source bytes; original Task authorship boundaries remain.

WG resource proposal is ready at TASK-009/wg-network-proposal.md, identity
`sha256:ee61ea74e60237abae9453aa1103680b801561f29c06d86be9e46dcd87399843`.
User explicitly approved this exact proposal with "允许。" under CHANGE007. The preserved original PROPOSED bytes are historical. Two fixed Go direct dependencies+normal transitive modules, independent loopback
peer, first TCP/ICMP subset, and possible short all-interface DUT WG UDP binding are authorized. DCR009 has passed independent review002; no duplicate user approval for this scope.
DCR009 independent review002 PASS at sha256:b193f16e6262f8695780af2f18cebc4a4f91dbbbe999f4b91f716059edbb9470; readiness011 PASS. Go helper and Rust integration are now being implemented in disjoint files. Initial fixed-module tidy and import-compatibility build PASS, but that provisional binary is NOT a completed helper and must not be used for integration. Full reject/UDP input/system DNS topology still
needs distinct design; do not bundle it into this resource approval or mark SF003 complete.
The checkpoint009 and earlier gaps below are historical;010 supersedes only the bounded worker ordering
and client concurrency evidence. TASK009 remains IN_PROGRESS; do not start TASK010,commit or push.

Previous checkpoint009 identity:
`sha256:da3b834d77fd6954d19737188c381849e41f09255e5e30accc8c4b0ce3506b74`.
Independent review008 found CR003(P1) echo listener released before all cores stopped and CR004(P2)
block I/O did not inherit the service's absolute deadline. Root repaired only Port cfg(test) code:
main fixture keeps echo listener until both cores/pending cleanup confirmed; service uses a clone.
Bind attempts prove target ownership after echo thread completion and during the second core.
Every block I/O uses min(service deadline, now+2s); connect/pacing share the same cutoff. New actual
TCP test verifies expired writes send nothing and a50ms service deadline overrides the2s I/O limit.
verification009 has26 unique tests PASS(24compiler+real metering+socket deadline), non-test library
Clippy/fmt/diff-check PASS and unchanged before/after host config hashes. Real positive/silence/new
instance assertions reran successfully. Independent checkpoint009 repair review PASS; CR003/CR004 FIXED and no new findings. Whole Task coverage remains INCOMPLETE.
metering-repair-baseline-009 preserves all33 checkpoint008 bytes. No new design or scope expansion.

Previous checkpoint008 identity:
`sha256:d85943538edb81eab0ae70023d27a811c1388bb99bd2838894c6e4819ca7f9d9`.
User explicitly approved the concrete loopback metering proposal with "允许"; CHANGE006 records the
approved proposal hash. Its original PROPOSED bytes are preserved for provenance, not a pending approval.
DCR008 independently PASSed; readiness008 PASS. Do not request duplicate approval for this scope.
Compiler now has a cfg(test)-only typed direct TCP inlet fixed to127.0.0.1 and this test's echo port.
Normal product ObservationOnly remains empty-inbound; no new product Profile,IPC,dependency or settings.

verification008: 24 compiler tests plus one real fixed-core two-instance metering test PASS, non-test
library Clippy/fmt/diff-check PASS. Known pattern received/echoed1310720bytes each way matches exact
final REST counters; rates positive during transfer then zero in two quiet samples with stable totals.
Owned child listener set is exactly fixedAPI and loopback inlet. After confirmed Stop, new identity,
config hash and fresh secret; old auth fails, totals zero. Real safe samples feed the existing memory
owner, and explicit lifecycle notifications clear shared graph history; this is not controller/GUI E2E.
Host interfaces,addresses,routes,DNS/proxy selected config hashes are unchanged. The initial Option
compile error and inherited nonblocking echo-socket failure were fixed and raw failures retained.
Independent checkpoint008 delivery review was REWORK for CR003/CR004 above. Full TASK009 remains REWORK/IN_PROGRESS: true worker
network in-flight/late-publication/stream concurrency and full WG/DNS obligations remain. No CI/commit/push.
metering-baseline-008 retains all32 checkpoint007 source bytes; compiler is now PRIMARY with its
original TASK009 baseline retained. Previous UI and full-suite evidence keeps its historical scope.

Previous checkpoint007 identity:
`sha256:efc1766a8bf8b2773709008b180dce7df4a51a72e8cbac0a15908ce5a3894d08`.
User "继续下一步开发任务" authorized the next existing-scope SF002 verification step.
Only two cfg(test) additions: real fixed-core lost Start response preserves owner until manual Stop;
actual loopback TCP/WebSocket first-frame timeout releases old socket and fresh authentication reads
new counters. Preset history in the first test is synthetic; second test uses a Mock API server.
verification-007 records 22 unique affected-module tests (12 API, 10 controller), library Clippy,
fmt and diff-check PASS. Two standalone new-test passes overlap this count. Initial E0382 compile
iteration is retained with resolved final results. Production, frontend, requirements and design are
unchanged. Independent checkpoint007 increment review PASSes with no findings; whole Task remains REWORK. Prior UI/full-suite results are historical,
not rerun evidence. concurrency-baseline-007 captures all 32 checkpoint006 source bytes; original
baseline and all prior authorship boundaries remain retained.

The checkpoint007 proposal and unverified-positive-flow statements below are historical; CHANGE006,
DCR008 and verification008 supersede that authorization/evidence gap only. TASK009 remains IN_PROGRESS;
do not mark acceptance, start TASK010, commit or push.

User explicitly requested a 10-minute homepage traffic chart and a 60-second bottom-left chart,
referencing E:/wx_lifeilin/github.com/clash-verge-rev UI. Its layout and traffic components were
inspected read-only. CHANGE-005 synchronizes Requirement, TASK-009 and DCR-001; DCR-007 independently
passed. This request authorizes the necessary bounded history limit and layout changes.

Previous dual-window checkpoint006 identity:
`sha256:0b6834cf6ec90174cbc6f7452513f0e2533a5adbf2f7f2a27e0b34947cd593cb`.
The existing owner retains at most 600 points / 10 minutes. Same strict safe Snapshot/Delta fields,
no new command, sampling, queue, dependency, core input, storage or host settings. The frontend uses
one accepted snapshot and one monotonic display clock for both windows, with independent time crop
and vertical scale. Homepage shows 10-minute graph, rates and core cumulative totals; 60-second
mini graph is at desktop sidebar bottom and reflows below content at 320px. Start/Stop/Toast remain,
with diagnostic details collapsed. New core/stop/recovery clear both views; healthy config failure
keeps prior runtime data. Aggregation includes core-handled Direct, not whole-system NIC traffic.

verification-006 records PASS for 24 targeted Rust tests, 50 frontend tests, 21 browser Mock checks,
TypeScript lint/build, library Clippy, fmt and diff-check. Root inspected desktop/narrow screenshots;
Mock cases prove old peaks only affect the homepage scale, both expiry windows and no additional IPC.
The local browser and Vite processes were closed. Independent checkpoint006 review passes this bounded increment with no new findings; whole-Task review remains REWORK.
Whole TASK-009 remains IN_PROGRESS and unaccepted: real metered positive flow, true network
in-flight/late result and full WG/DNS obligations remain. No current full Rust suite/CI/commit/push.

Original baseline and extensions still define authorship; dual-trend-baseline preserves checkpoint005
bytes for incremental comparison. Do not repeat settled aggregation or two-window questions, change
unrelated pre-existing dirty work, start TASK-010 or mark the whole Task accepted. The checkpoint001–005
narrative below is historical; it does not supersede the current identity, limits or verification.
Veyra is a sing-box-only desktop client. The accepted path normalizes subscriptions into a typed
domain model, then compiles a managed configuration. UI and Windows GUI/Tray E2E are deferred until
the runtime/configuration chain is complete. System Proxy, TUN, UAC, WFP, Service, arbitrary
endpoints and raw-config IPC remain outside the current work.

TASK-007 is `DONE` after USER:lifei Human Task acceptance. Its frozen identity is
`sha256:c83dd3e2df82c45ab6cb9d7c46c1e16ef266093e3db8abc2cf73686b4ea1442c`.
It delivered the V3 typed 15 non-Tor user-protocol model, fail-closed V2-to-V3 migration, strict
Clash/sing-box/URI/paste normalization, and atomic Provider replacement. The final independent
review is PASS with no P0/P1; see `EVIDENCE-TASK-007-REVIEW-001`.

## Verification

For the prior TASK-007 delivery, `cargo test --manifest-path src-tauri/Cargo.toml --lib` passed 129 tests; scoped formatting and
`cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` passed; `git diff --check`
passed (only CRLF warnings for the pre-existing dirty worktree). Full all-target Clippy is recorded
as `FAIL` for two out-of-scope existing findings in `src/singbox/mod.rs` and `src/singbox/clash_api.rs`.
Tor runtime remains `BLOCKED`; CI, commit and push are `NOT_RUN`.

## Current

User explicitly approved DCR011 with 允许. CHANGE009 binds its unchanged54c01cbc... candidate.
Readiness014 PASS; all50 previous sources verified and copied to wg-reject-baseline-014 before Task sync.
Root owns compiler cfg(test); Rust worker Port tests; Go worker peer module. Real core runs root serial.
The historical proposal/approval-pending notes below are superseded by CHANGE009; no repeated approval.
Checkpoint013 is the prior frozen baseline; current implementation requires new identity and review.
 Task

TASK-008 is DONE after USER:lifei explicitly said "通过 TASK-008" on 2026-09-04.
EVIDENCE-TASK-008-ACCEPTANCE-001 records all three subfeatures and overall Task acceptance PASSED.
USER:lifei approved DCR-003 identity
`sha256:3040951342ef465fb177b3abb1a7537d73c55bc87e88e1a7b3ba93677033b26d`;
EVIDENCE-TASK-008-DCR003-APPROVAL-001 and READINESS-002 establish the accepted design and readiness.
The reviewed proposal is retained byte-for-byte; its historical proposal wording does not revoke approval.
Non-persistent Domain DNS and runtime test-fixture scope are also approved.

The compiler produces 14 outbound protocols and controlled user-space WireGuard endpoints with
leading route/DNS reject rules. Node server DNS is local; URLTest retains native member protocol
resolution. The authenticated-peer virtual-address ICMP Echo exception is explicitly accepted.

Implementation baseline is `.sdlc/evidence/TASK-008/baseline.yaml` plus `baseline.patch`;
ignored byte copies are under `src-tauri/target/task008-baseline`. Preserve pre-existing staged/dirty
work. Compiler/Domain and final-byte check/caller adaptation are disjoint implementation partitions.

## Next

Implementation is frozen at `sha256:d2f866af0f53ad28e8ba1cbe2d9b686b59f5c5f94703a25eb2d55cabaef5bc82`;
see TASK-008/delivery.yaml and delivery.patch for full inventory, baseline and source hunk ownership.
EVIDENCE-TASK-008-002 records 143 passing Rust tests (one real-child test excluded), 34 final config
checks including Reality TCP/WebSocket, scoped formatting, library Clippy and diff-check PASS.
EVIDENCE-TASK-008-REVIEW-002 independently passes both partitions and integration with no open findings.
All TASK-008 subfeatures, independent review and explicit Human Task acceptance pass. TASK-009 is now
IN_PROGRESS with three subfeatures: real runtime/failure-stop, identity-bound observation and toast, and
all mandatory DCR-003 controlled WireGuard/DNS/URLTest cases. tasks.yaml now points to TASK-009;
TASK-010/011 remain future stubs. EVIDENCE-TASK-009-READINESS-002 passed and focus is now TASK-009.
M6 remains IN_PROGRESS. TASK-009 implementation and bounded real lifecycle/UI/network checks have run;
the full Task is incomplete. CI, commit and push remain NOT_RUN.
Pre-existing user staging and dirty changes are preserved. Before acceptance all 22 manifest entries
and 12 final logs matched. reviewed-task.md preserves the reviewed Task bytes; canonical Task changes
only record acceptance status, approval and evidence links, with no scope or implementation changes.

USER:lifei explicitly confirmed three distinct failure behaviors in CHANGE-001. Subscription refresh
failure preserves old valid data and last successful update time, logs and toasts; manual retry only.
Compile/finalize/check failure logs and toasts without stopping the current healthy child or claiming
the new configuration applied. Candidate spawn/Ready failure stops and cleans that candidate and
does NOT automatically restart the old configuration. Confirmed cleanup means Stopped; unconfirmed
exit/cleanup retains owned resources and RecoveryRequired. Do not reintroduce automatic rollback.

Frozen DCR-004-runtime-update-failures.md has independent review PASS at
`sha256:8f06957ff73919f9529dd1740b72f4dd4ccf73745d6481b124133f3ca8d8d944`;
see design-scope-review-002.yaml for the approved CHANGE-002 scope and exact empty-log response contract.
The explicit user directive already authorizes these semantics; no duplicate approval request.
Requirement §70/78, Foundation, DCR-001/002, state anchor and TASK-009 are synchronized. Preserve the
existing Application Plan/final-byte binding; no rollback Plan cache is needed. Add only the closed
configurationFailed outcome and existing-entry failure toast/safe log changes in the Task Scope.
TASK-009 baseline was captured before implementation in TASK-009/baseline.yaml and baseline.patch;
ignored source copies are under src-tauri/target/task009-baseline. Preserve all pre-existing dirty/staged work.
Current partial checkpoint is TASK-009/delivery-checkpoint-004.yaml with identity
`sha256:1e08f4e589f3d1aa0aac5665a4aed2c6875fcbb8cd7114872309dace7136b64a`.
It contains fail-stop without previous configuration restoration, pending/check ownership, fixed failure
logs and current-entry toast. Independent review and verification records distinguish this checkpoint
from completed Task acceptance. See verification-004.yaml and code-review-checkpoint-004.yaml for current results;
verification-002.yaml and code-review-checkpoint-002.yaml retain the prior production repair evidence.

USER:lifei explicitly approved CHANGE-002 with "允许". Windows private_runtime.rs creation-error
ownership and singbox/clash_api.rs normal disabled-log handling are now repaired and independently
reviewed. Failed creation plus failed cleanup retains the owned directory through Windows Port until
manual Stop succeeds. Authenticated Logs accepts HTTP 204 and the fixed core's exact empty HTTP 200;
Traffic and unauthorized/unexpected/timeout responses remain failures. Do not reintroduce rollback.
Both P1 findings are FIXED with no new open findings. Full Rust library regression passes all 165 tests,
with 0 failed, ignored or filtered tests (238.05 seconds). Real fixed-core full observation/Stop,
the 13-case Logs boundary, creation cleanup ownership/manual Stop, library Clippy and formatting pass.
implementation-002.yaml records bounded repair PASS; full Task review remains REWORK/INCOMPLETE.
The previous filtered regression is historical, superseded by this unfiltered run for checkpoint002.
Task-local reviewer findings about stale stopped state on response loss, stale UI snapshots and sensitive
assertion diagnostics were previously fixed and reverified. No duplicate approval is needed for CHANGE-002.

HTTP/SOCKS controlled Host/domain and ClientHello SNI tests passed as a limited subset. Full WG ICMP,
allowed TCP/UDP responses, reject/forward cases and controlled system DNS remain unavailable under
the current topology, detailed in network-prerequisites.md. The proposed peer dependencies, test input
profile and DNS resources are not approved. Do not remove SF-003 or mark Task complete.
TASK-011 will deliver the subscription page/manual refresh/time display; it is not implemented here.
Positive traffic rate/total semantics and complete identity/concurrency verification are also pending.
Checkpoint003 adds three cfg(test)-only cases with real worker Start/Start and Start/Stop ordering,
bounded queue Busy, repeated Start ownership/config preservation, real sample publication spacing,
no Delta/revision after confirmed Stop, and Mock active identity lifecycle isolation. Controller module
9 tests and runtime module 16 tests PASS; library Clippy, formatting and diff-check PASS. The
test increment independently PASSes code-review-checkpoint-003.yaml with no new findings; full Task
review remains REWORK with INCOMPLETE coverage. Production
bytes are unchanged from checkpoint002. The 165-test full run is checkpoint002 evidence; no new full
regression is claimed for checkpoint003. True in-flight network Stop, cross-instance late network results,
Start response loss and live stream concurrency count remain incomplete. See verification-003.yaml.
Automatic approval rejected deletion of failed test fixture directories task009-worker-36284-0 and
task009-worker-64148-0 under src-tauri/target with only 'blocked by policy'; retained, no workaround.
Worker read-only inspection found three fixed bundle resource hardlinks and no config in each.

DCR-005-traffic-observation.md was explicitly approved by USER:lifei ("确认") under CHANGE-003 at
`sha256:338e0a7f9bb8a4c6ca0722d2fe51eab253c0e1758124907cf759b9459e9714ba`.
traffic-design-review.yaml independently PASSes the design: fixed core traffic frames are one-second
window values, not totals; use those as nominal window rates and existing REST totals for cumulative
bytes. REST/WS sampling is not atomic. CHANGE-003 now authorizes this specific correction; do not ask
again. Proposal bytes remain unchanged as the approved identity despite their historical PROPOSED label.
DCR-001 and TASK-009 scope are synchronized. Checkpoint004 implements direct window rates plus REST
totals, removes obsolete differencing state/helper and strengthens existing DTO/Delta mapping assertions.
41 targeted tests PASS: Clash API 11, observability 13, Application runtime 14, real fixed-core observation
1 and real worker lifecycle/sample 2. Library Clippy, fmt and diff-check PASS with current logs.
code-review-checkpoint-004.yaml independently PASSes the bounded traffic correction with no new findings;
27 manifest entries matched at completion. Full Task review remains REWORK with incomplete coverage.
No current full 170-test regression is claimed. No public field, dependency, new inlet/profile or host
configuration changed. Original baseline and extension remain authoritative for source provenance.
Real positive-flow validation remains UNAVAILABLE: current ObservationOnly has no business inbound,
and the evaluated HTTP/SOCKS URLTest path bypasses the Router tracker; request success does not prove
metering. See positive-traffic-prerequisites.md for source evidence and explicitly unexecuted claims.
An additional routed/metered traffic inlet requires separate topology/design authorization; it is not
included in CHANGE-003. True network in-flight/late-result cases and the WG/DNS matrix remain pending.
Next work must resolve these existing TASK-009 obligations and the concrete controlled network topology;
do not start TASK-010 or mark TASK-009 accepted. This repair did not rerun frontend or host-network
before/after snapshots; earlier evidence retains its original scope. CI, commit and push remain NOT_RUN.

Old automatic-recovery candidate and review are superseded, with bytes retained at
TASK-009/reviewed-dcr004-001.md. Historical TASK-008 delivery is not reclassified as runtime evidence;
its requirement/design context changed under explicit user change control and must not be called current unchanged context.
Controlled network topology remains implementation/verification design work; no existing harness is
not itself a blocker, but no mandatory case may be removed. Extra dependency, non-loopback exposure,
privilege or external-cost changes require their own concrete authorization before execution.

Planning used CHILD_AGENT:/root/task009_planner (sdlc_expert); root inspected runtime feasibility in
parallel. Design author was root; independent reviewer was CHILD_AGENT:/root/task009_design_reviewer
(sdlc_reviewer). Implementation used separate runtime, UI and controlled-network workers plus root
Application changes. Actual model/usage telemetry is UNAVAILABLE. Business code and loopback test
processes changed; no host network setting or dependency was changed by this Task.

## References

Independent review identified P1 `TASK-008-REVIEW-001`, fingerprint
`singbox/compiler.rs:CoreTls:reality-missing-required-utls`.
It is FIXED in REVIEW-002: Reality-only required typed uTLS and strict negative tests, plus four
private-file/fixed-core Reality checks. Old REVIEW-001 and verification remain historical for
their old identity; delivery-001.yaml/patch preserve that review inventory.
Ignored diagnostic directories remain because automatic cleanup was rejected by tool policy.

- `.sdlc/tasks/TASK-007.md`
- `.sdlc/tasks/TASK-008.md`
- `.sdlc/tasks/TASK-009.md`
- `.sdlc/evidence/TASK-009/readiness.yaml`
- `.sdlc/evidence/TASK-009/technical-design-review.yaml`
- `.sdlc/design/DCR-004-runtime-update-failures.md`
- `.sdlc/evidence/TASK-009/change-control.yaml`
- `.sdlc/evidence/TASK-008/delivery.yaml`
- `.sdlc/evidence/TASK-008/acceptance.yaml#EVIDENCE-TASK-008-ACCEPTANCE-001`
- `.sdlc/evidence/TASK-008/implementation.yaml#EVIDENCE-TASK-008-002`
- `.sdlc/evidence/TASK-008/code-delivery-review.yaml#EVIDENCE-TASK-008-REVIEW-002`
- `.sdlc/evidence/TASK-008/readiness.yaml#EVIDENCE-TASK-008-READINESS-001`
- `.sdlc/evidence/TASK-008/readiness.yaml#EVIDENCE-TASK-008-AUTHORIZATION-001`
- `.sdlc/evidence/TASK-008/core-compatibility.yaml#EVIDENCE-TASK-008-CORE-001`
- `.sdlc/evidence/TASK-008/technical-design-review.yaml`
- `.sdlc/design/DCR-003-fixed-core-configuration.md`
- `.sdlc/evidence/TASK-007/implementation.yaml#EVIDENCE-TASK-007-001`
- `.sdlc/evidence/TASK-007/code-delivery-review.yaml#EVIDENCE-TASK-007-REVIEW-001`
- `.sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md`
