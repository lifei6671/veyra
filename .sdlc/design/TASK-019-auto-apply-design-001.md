# TASK-019 structural auto-Apply / concurrency Design 001

Status: CANDIDATE / NOT FROZEN / Human approval required. Producer: Codex specialist
`/root/task019_apply_design`, integrated and clarified by `/root`. This is the bounded runtime/concurrency constituent for the
TASK-019 composite Design; it does not grant implementation authority or change State/Gate/Task.

## Sources and fixed decisions

- Source: `docs/veyra.md` identity `sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4`,
  `.sdlc/tasks/TASK-019.md`, `TASK-019-requirements-001.md`, and DCR-026 revision003 candidate.
- Structural set is exactly create/update/delete custom Pool, Pool enable/disable through update,
  setDefaultTarget, create/update/delete Route, Route enable/disable through update, and reorderRoutes.
  Manual selector and subscription activation retain their current paths.
- A real changed structural save increments `active_configuration_generation`; a no-op does not save,
  advance generation, or Apply. Save failure performs zero Apply and preserves existing bytes.
- Auto-Apply is admitted only for an authoritative active subscription while the owned runtime is
  actually Ready. stopped, no active, and recoveryRequired never start or recover a child.
- Retry reads current authoritative Desired and performs Apply only. No CRUD payload is retained,
  queued, replayed, or inferred from an unknown response.

## Existing invariants to retain

`ProxyRoutingManager.mutate` currently serializes the state.json critical section with
`StateAccessGate::try_lock`, compares process-local `expected_revision`, persists the full state,
then commits `RevisionState(CasMaterial)`. `CasMaterial` already includes active subscription,
desired generation, providers/nodes/pools/default/routes; therefore revision identifies the complete
Desired material seen by the UI, while generation identifies runtime-relevant structural content.

Current explicit Apply reads `(active_subscription_id, active_configuration_generation)` under the
state gate, releases that short gate, then calls `apply_proxy_routing ->
activate_with_expected_generation(force=true)`. The runtime `busy` guard and subscription write guard
are held by `ActivationRequest` through worker completion. Worker preflight compares the tuple, and
the worker checks the full saved `AppState` again before commit. These guards remain the only lifecycle
serialization; `StateAccessGate` must never be held across compile/check/sidecar work.

`operationId` identifies one dispatched Apply attempt. It is not a revision or generation. The
current process-memory `apply_tuple` lacks operation identity; this candidate replaces it with a
runtime-owned attempt record `{operationId, activeSubscriptionId, desiredGeneration}`. Snapshot
projection may call an operation applying/failed/unknown only when both its operationId and tuple
match the current Desired. `Applied` additionally requires actual Ready plus matching applied
subscription and generation.

## Authoritative post-save table

| State after a changed save | Backend action | Returned/observable result | Feedback and recovery |
| --- | --- | --- | --- |
| save failed | no runtime call | mutation error `saveFailed`; old revision/generation | local error; no Apply action |
| no effective active subscription (unless recoveryRequired takes precedence) | clear pending auto intent; no runtime call | saved + `noActiveSubscription`, `savedPendingApply` | persistent “已保存，当前无可应用订阅”; navigate/select subscription |
| runtime stopped | clear pending auto intent; no runtime call | saved + `runtimeStopped`, `savedPendingApply` | success 3000ms “已保存，将在下次启动时应用” |
| runtime recoveryRequired | clear pending auto intent; no runtime call | saved + `recoveryRequired`, unchanged recovery runtime state | persistent recovery feedback; never Apply/start |
| runtime transitioning because Start/Stop/non-Apply owner is busy | no auto intent and no queued CRUD | saved + `runtimeBusy`, `savedPendingApply` | show saved/pending or drift and explicit recovery; never infer a routing Apply continuation |
| older routing Apply is busy | overwrite the single pending intent with latest tuple | saved + `runtimeBusy`, `savedPendingApply` | old operation may finish, but cannot resolve current generation |
| runtime Ready and admission succeeds | dispatch existing forced Apply for exact current tuple | `applyStarted(operationId)` or synchronous `applyCompleted(operationId)` | “已保存，正在应用”; no early success |
| matching operation reaches Ready and applied tuple matches | no further dispatch | `applied` | one current-tuple success Toast: rule/pool wording, 3000ms |
| matching Apply deterministically fails | no automatic retry | `savedApplyFailed(operationId,error)` | persistent “已保存，但应用失败” + “重试应用” |
| matching response/terminal is unknown | no automatic retry | `applyUnknown(operationId)` | persistent unknown/drift recovery; refresh then explicit retry-current-Desired |
| superseded operation completes | ignore it for current result | current tuple remains pending/applying/failed/applied from its own attempt | must not clear/replace current Toast or snapshot state |

“Effective active” is resolved from current AppState, not a UI string. SelectionRequired/NotFound is
the no-active branch. Once an active selection resolves, projection/compiler/check failures are Apply
failures, preserving the existing failure taxonomy rather than mislabelling invalid configuration as
no subscription.

## Ready-only admission and one-slot latest convergence

Add a dedicated `auto_apply_current_if_ready` mode beside explicit `ApplyConfiguration`; do not weaken
explicit Apply/activation behavior. Admission is linearized by the existing runtime `busy` guard, then
the worker checks its actual `SidecarRuntime.snapshot().lifecycle` before the existing code path that
constructs a runtime when `runtime.is_none()`. Only `Ready` proceeds. `Stopped` and
`RecoveryRequired` return the saved-only disposition. Thus a frontend Ready snapshot is only a hint;
the locked worker check is authoritative.

When admission finds the worker busy with an identified routing Apply, store one process-memory `pending_auto_apply` tuple. It is an
overwrite-only latest intent, not a list: every later successful structural save replaces it. Use at
most one `AutoApplyLatest` wake marker on the existing worker channel. The marker carries no CRUD and,
when the current owner releases its guards, takes the slot, re-reads AppState, and proceeds once only
when slot tuple equals current Desired and actual lifecycle is Ready. It then clears the slot before
dispatch. A deterministic failure, unknown result, stopped/no-active/recovery state, shutdown, or a
tuple mismatch clears that consumed intent and does not schedule another attempt.

This one-slot drain closes the real “new save arrived while old Apply owned `busy`” gap without a
general scheduler. It is volatile: process restart loses it, and the authoritative snapshot then shows
Desired/Applied drift with explicit recovery. It has no timer, retry loop, persistent field, migration,
or per-mutation queue. A wake marker already present is reused while the tuple is overwritten.

## Race and failure rules

- New structural saves still compete only on expected revision/state gate. A conflict or busy save is
  not queued; the caller refreshes and decides whether to submit a new user mutation.
- If a newer save happens before an old worker's second AppState check, the old request becomes
  StateChanged and cannot commit. If it happens after that check, the old runtime may briefly reach
  Ready at the old generation; tuple filtering exposes drift and the one-slot intent applies only the
  latest generation. Desired is never rolled back.
- Stop wins when it owns `busy` first: the later save is persisted, its marker observes Stopped and is
  discarded without start. Auto-Apply wins when it owns `busy` first: concurrent Stop returns existing
  Busy, and the user can retry Stop after convergence. There is no hidden Stop queue.
- A lost structural mutation response triggers authoritative refresh only. It never installs a pending
  intent in the frontend and never replays CRUD. The backend may already have saved/dispatched; only
  revision/generation/operation observation determines recovery presentation.
- A lost Apply response retains its operation/tuple attempt. Matching observation may settle it;
  otherwise it becomes unknown. Explicit retry first refreshes, then submits `retryCurrentDesired` for the refreshed revision; backend reads the authoritative current tuple and uses the same Ready-only admission. Stale revision conflicts and refreshes; retry never starts a stopped runtime or saves state. Existing legacy `ApplyConfiguration` remains unchanged for any independent explicit caller.
- Backend busy after a save is an OK saved disposition, not a mutation error. Compiler/check/stop/start
  failures after save are also represented as saved Apply failure; they cannot rewrite save success.
- `ProxyRoutingResponseOrder` invalidates in-flight queries when mutation starts and rejects mutation
  snapshots older than its accepted revision. Equal-revision runtime updates are refreshed from the
  authoritative event stream; old operation identity cannot clear a newer tuple's persistent notice.

## Exact internal IPC result and wake handoff proposal

The existing command name `mutate_proxy_routing` stays. Add mutation `{type:"retryCurrentDesired"}` to the existing strict Rust/TS union, using the existing expectedRevision envelope; no CRUD payload. Backend reads current state under existing state gate, CAS-validates that fresh revision, captures tuple, releases state gate, and applies with Ready-only mode. A concurrent tuple change fails admission as StateChanged; never apply the stale tuple. Legacy ApplyConfiguration and ordinary Subscription.activate do not inherit Ready-only behavior. A disallowed retry returns no-save `ok/outcome:applySkipped(reason)` with a current snapshot, or existing query error when snapshot unavailable. Its UI text must not claim a new save; stopped/noactive/recovery points to existing user navigation and no start call.

For a changed structural mutation, add a `status:"saved"` result variant, separate from no-op/manual/explicit-Apply existing `ok/error`. It contains receipt `{desiredGeneration:number,activeSubscriptionId:string|null,revision:number|null}` captured from the committed Desired; `snapshot:ProxyRoutingSnapshot|null`; `apply:{type:"skipped",reason:"noActiveSubscription"|"runtimeStopped"|"recoveryRequired"|"runtimeBusy"}|{type:"started",operationId:string}|{type:"completed",operationId:string}|{type:"failed",operationId:string|null,error:MutationError}|{type:"unknown",operationId:string|null}`. Runtime observation, not receipt, proves Applied. Null snapshot/revision marks successful persistence but unavailable current readback; show saved/refresh recovery, do not claim current snapshot or runtime success. Keep the exact committed receipt even if revision bookkeeping/runtime coordination/readback fails after successful store.save. Failures before successful persistence stay error. No external storage/schema migration.

The TS strict parser recognizes only this exact union and safe integer/null identities, page submit closes only after a matching operation's confirmed `saved`/applicable `ok`, and continues to treat malformed/lost response as unknown plus refresh. Never infer a create succeeded merely because some other producer increased generation. Existing `shouldDismissDialog`/page handlers require matching request context and deterministic tests for saved-with-null-snapshot as well as stale dialog. Generation receipts cannot replace authoritative provider snapshot; response-order rejects older revision snapshots, then refreshes equal-revision runtime truth.

Wake handoff: pending tuple publication is monotonic by committed generation for the same active subscription; a delayed older post-save callback cannot replace newer intent. Before publishing and before consuming, compare authoritative active subscription+generation; stale tuples are discarded rather than revived. A single mutex around pending tuple+marker-needed flag protects that small nonblocking memory operation; do not hold it with StateAccessGate, runtime/Subscription guards or channel wait. Existing busy AtomicBool remains the lifecycle owner, not this mutex. Publish slot before nonblocking wake attempt; at most one marker, no requests list. A Full existing channel is safe only because worker drains the pending slot after *every* completed owner request and before its next blocking recv, including errors. Disconnected/shutdown clears the intent with visible unknown/recovery. Worker atomically takes current slot, releases its mutex, then acquires existing guards and rechecks actual lifecycle/current Desired. If a new save races drain, retain the newer generation slot and ensure another marker is armed; do not clear a replacement using an old taken token. Idle-worker wake and completion-drain therefore have no lost-wakeup window. No failed/unknown tuple is reinserted.

This pending slot is only for a save made while a *routing Apply* holds the worker. Busy Start/Stop/Subscription-activation or an unknown owner does not arm auto intent: it saves with pending/drift and explicit recovery only. A routing Apply may produce a brief old-generation Ready before the latest intent is consumed; it may never overwrite persisted Desired, report current success, or clear current recovery. Drain must not bypass an accepted Stop/shutdown; the actual stopped/recovery check always dominates.

## Minimal implementation boundary requiring approval

The current TASK-019 DRAFT authorizes these Rust files only for read-only Design. Human approval must
add the following exact write boundary before implementation:

- `src-tauri/src/application/proxy_routing.rs`: post-save structural orchestration; saved-only reason
  response variants; canonical attempt `{operationId,tuple}` matching; keep no-op/manual/explicit Apply
  contracts and the existing revision/generation CAS.
- `src-tauri/src/application/managed_observation_runtime.rs`: Ready-only admission mode, one volatile
  latest-tuple slot plus one coalesced wake marker, runtime-owned current attempt record, and worker-side
  drain before any stopped-runtime construction. Existing Start/Stop/activation failure cleanup stays.
- `src/lib/proxy-routing.ts`: strict parsing/types for saved-only reason, response-order rejection,
  current-tuple notice ownership, and retry-current-Desired. No local authoritative snapshot patching.
- Tests only in existing `src/lib/proxy-routing.test.ts` and the `#[cfg(test)]` modules of the two Rust
  application files. `state_access.rs`, commands, Domain/AppState/storage/compiler/schema and manifests
  require no change. If implementation proves otherwise, stop and return a new scope decision.

No dependency, config, persistence, migration, network listener, general scheduler, background retry,
feature flag, fallback, or new worker is proposed.

## Alternatives and disclosed risk

1. Frontend `save.then(apply)` was rejected: its Ready check races Stop and cannot bind lifecycle truth.
2. “Busy means manual retry only” was rejected: a later successful structural save during an older
   Apply would violate the promised automatic convergence and add an avoidable second user action.
3. A durable/per-operation queue was rejected: CRUD replay and persistent scheduling are prohibited and
   add recovery/migration semantics. The one overwrite slot is the minimal sufficient mechanism.

Residual risk: after process loss, response loss, or a consumed intent ending non-Ready, Desired may
remain unapplied until an explicit current-Desired retry or next user Start. This is visible drift, not
silent success. Runtime failure still follows existing owned-child cleanup and redacted error rules;
this design does not prove or change sing-box internals.

## Verification contract

- Frontend: `pnpm exec vitest run src/lib/proxy-routing.test.ts`; then task-level `pnpm lint` and `pnpm build`.
- Rust focused suites: `cargo test --manifest-path src-tauri/Cargo.toml application::proxy_routing::tests::`
  and `cargo test --manifest-path src-tauri/Cargo.toml application::managed_observation_runtime::tests::routing_apply`.
  Every filter must report non-zero tests.
- Add deterministic double-order cases: old completion before/after newest save; three saves coalesce to
  one latest tuple; Stop-before-marker; Apply-before-Stop; no-active/stopped/recovery clear; failure and
  unknown do not retry; response loss never replays CRUD; old operation cannot clear newer feedback.
- After approved Rust changes run `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`,
  `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`, and `git diff --check`.
  Minimal fixed sing-box check/run/API/stop evidence validates Veyra orchestration only.

UI Gate execution is intentionally NOT_RUN for this constituent: the final composite TASK-019 Candidate
must merge this state machine with the UI metadata/Contract candidate and run the required design check.
No Compliance, Human Visual Approval, Task acceptance, or Gate result is claimed here.
