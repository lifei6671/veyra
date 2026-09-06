# TASK-011 editor, runtime actions and owned-core memory 034

Status: CANDIDATE. TASK-011 remains `IN_PROGRESS`; this document does not change Task, State or Gate status. The user explicitly requested this amendment and approved locally bundled `monaco-editor` 0.56.0. The former TASK-011 exclusions for source editing, bulk actions and automatic apply after an active-document save are superseded only for the behavior below. This revision remediates `ED034-DESIGN-001/002`: Save preserves exact edited text and returns one definitive terminal result. Implementation starts after independent technical-design review; no further product authorization is required for this bounded scope.

Authority: `.sdlc/tasks/TASK-011.md` amendment 034 and `.sdlc/evidence/TASK-011/editor-runtime-backend-discovery-034.json`. Existing activation failure semantics remain governed by `DCR-004-runtime-update-failures.md`. The Windows memory unit follows Microsoft `PROCESS_MEMORY_COUNTERS.WorkingSetSize`, the current working set in bytes ([GetProcessMemoryInfo](https://learn.microsoft.com/en-us/windows/win32/api/psapi/nf-psapi-getprocessmemoryinfo), [PROCESS_MEMORY_COUNTERS](https://learn.microsoft.com/en-us/windows/win32/api/psapi/ns-psapi-process_memory_counters)).

## 1. Current boundary and resulting decisions

1. `Subscription` persists normalized nodes and safe remote metadata but no source body. A JSON/YAML source cannot be reconstructed from normalized nodes. Source editing therefore requires persisted document bytes.
2. `SidecarRuntime` owns separate candidate and active `GeneratedConfig` values. Only the active value represents bytes actually committed to the owned child; persisted state and a newly compiled candidate do not.
3. Current activation returns `AlreadyCurrent` when the Ready observation's subscription ID and generation match. Force reactivation must bypass only that early return and use the existing compile/check/prepare/save/commit worker path.
4. Runtime observation resolves an opaque managed identity to the current owned `std::process::Child`. Memory sampling uses that child handle and never searches by PID, process name or system-wide enumeration.

## 2. V6 persisted document

Add the following typed fields. `content` is never included in `Debug`, summary, settings, snapshot, delta, event, error or log DTOs.

```rust
pub struct Subscription {
    // existing fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document: Option<SubscriptionDocument>,
}

pub struct SubscriptionDocument {
    pub format: SubscriptionDocumentFormat, // Json | Yaml
    pub content: String,                    // UTF-8, 1..=4 MiB bytes
    #[serde(default, skip_serializing_if = "is_false")]
    pub local_override: bool,
}
```

- Increment `CURRENT_SCHEMA_VERSION` to 6 and add `StoredStateV6` plus V5→V6 migration. Migration sets every `document` to `None` without fetching, parsing or synthesizing content, and preserves IDs, references, selected subscription and generation. Explicit V6 prevents an older V5 writer from silently accepting and then dropping raw documents.
- `local_override` may be true only for a remote subscription. Manual documents always use false. State validation enforces the byte bound and source relationship; syntax and conversion are writer-boundary checks, not repeated on every unrelated state load.
- Remote/manual imports and 200/manual updates retain the exact body only when the parser classified it as JSON or Clash YAML. URI-list/Base64 inputs remain usable but return `documentUnavailable` for editing. A successful editor save preserves the exact submitted UTF-8 text, including YAML comments and intentional layout.
- Existing private state snapshot, backup, atomic replace and recovery boundaries protect the new content. The content may contain node credentials. It is never copied to separate storage, browser storage or telemetry.
- A document revision is lowercase SHA-256 of a domain-separated `format || content` byte sequence. It is computed on read and save and is not another persisted field.

### Remote refresh and local override

- A successful editor save on a remote subscription sets `local_override=true`, clears only `etag` and `last_modified`, and retains safe traffic/content-disposition metadata.
- Explicit or scheduled remote refresh sends validators only when the document exists, `local_override=false`, and normalized nodes exist. A missing migrated document or local override forces an unconditional request.
- The unconditional refresh must receive a 200 body. An unexpected 304 returns `cacheUnavailable`, records the failed attempt under the existing deadline rule, and preserves document, nodes, metadata, generation and current child.
- A successful 200 applies the normal import policy, replaces the stored JSON/YAML document (or sets it to `None` for a supported non-document format), sets `local_override=false`, and replaces validators. It overwrites a local edit. Ordinary remote refresh still does not activate automatically; an active subscription's generation changes and the applied/candidate mismatch remains visible until Use/force.

## 3. Formatting and strict editor save

Formatting is a bounded application service backed by existing `serde_json` and `serde_yaml_ng`; it performs no fetch, schema download or core execution. JSON becomes pretty JSON and YAML is parsed and serialized canonically. Comments and presentation-only YAML details may be lost. Only the user's explicit Format action replaces the editor buffer with this returned text, which the user can review before a later Save.

- Reject empty input and content over 4 MiB before parsing, and reject canonical output over 4 MiB rather than truncating it. Run parsing/formatting in one process-wide blocking slot with a fixed 5-second response timeout. A timed-out worker cannot commit state; its late result is discarded and it retains the slot until exit, so repeated requests return `busy` rather than accumulating detached work.
- Save does not format or rewrite the submitted text. It reparses that exact text for syntax, then uses the existing subscription parser and requires at least one supported node, zero `SkippedNode` entries, no provider-only Clash document, successful full-tuple dedup/normalization, and complete `AppState::validate`. The revision is computed from the exact bytes that will be stored. This strict all-or-nothing editor rule does not change ordinary import/update filtering.
- Syntax and conversion validation finish before acquiring the subscription write guard. Save then acquires the existing owned write guard, loads the expected state under the short state gate, verifies `expectedRevision`, builds the candidate, and reloads/compares before save. Update/edit/delete/activate/scheduler/shutdown therefore resolve as `busy` or `documentConflict`; they cannot overwrite an unseen edit.
- Inactive save atomically stores the exact document and normalized provider replacement. Active means the subscription is the selected `summary.active` subscription. Its save increments `active_configuration_generation` whenever document revision changes, even if normalized nodes are equal, and follows the Use path immediately. If the core is Stopped, this starts the saved candidate; it never selects or starts another subscription.
- Active save uses the same guard order and serialized runtime worker as activation. It projects, compiles, checks and prepares the replacement before the state save. Format, conversion, projection, compile, check, prepare, CAS or save failure aborts any prepared candidate, leaves the original document/state and old child unchanged, and keeps the editor open.
- After state save, `commit_prepared` applies or starts the child. A stop/start/ready failure cannot honestly roll the persisted document back under existing DCR-004 semantics. The response reports `status:"ok"`, the new revision and `apply.status:"failed"`; the editor stays open but changes its conflict baseline to the new revision. The UI must say that the document was saved while application failed.
- Save has one 30-second internal operation deadline and no public pending state. The async command awaits a one-shot terminal worker result instead of applying the activation command's 15-second response timeout. Strict parse work is limited to the existing single blocking slot and 5 seconds; core check is bounded at 10 seconds, owned-child stop at 2 seconds, and Ready API at 2 seconds. The worker checks the shared deadline before persistence and before commit. Deadline expiry before persistence aborts the candidate and returns `operationTimedOut` with no change. Expiry after persistence aborts any uncommitted candidate and returns `status:"ok"` with the new revision and `apply:{status:"failed",error:"operationTimedOut"}`. Inactive persistence that completed is terminal `notRequired` even if the deadline is then exhausted.
- IPC/future cancellation does not cancel or detach a save worker. The worker retains both guards and finishes to a terminal state; a disconnected caller receives no fabricated error. Existing safe subscription-change/switch events are still emitted on actual changes, and reopening the editor performs a fresh read/revision comparison. Neither event contains content.
- Shutdown rejection before save leaves everything unchanged. Once persistence begins, shutdown waits for the owned save worker through the existing subscription/runtime guard barrier; after-save failure retains the new document and reports the actual application result when a caller remains connected.

## 4. Closed IPC contracts

All request structs use `deny_unknown_fields`, camelCase and the existing main-window origin check. `content` is returned only in a direct response to an explicit command.

```ts
type DocumentFormat = "json" | "yaml";
type ErrorLocation = { line: number; column: number }; // positive safe integers

get_subscription_document({ request: { id: string } }) =>
  | { status: "ok"; document: {
      id: string; format: DocumentFormat; content: string; revision: string;
    } }
  | { status: "error"; error:
      "invalidInput" | "notFound" | "documentUnavailable" |
      "stateUnavailable" | "busy" };

format_subscription_document({ request: { content: string; format: DocumentFormat } }) =>
  | { status: "ok"; content: string }
  | { status: "error"; error:
      "invalidInput" | "contentTooLarge" | "formatFailed" | "busy";
      location?: ErrorLocation };

save_subscription_document({ request: {
  id: string; content: string; format: DocumentFormat; expectedRevision: string;
} }) =>
  | { status: "ok"; subscription: SubscriptionSummary;
      documentRevision: string;
      apply:
        | { status: "notRequired" }
        | { status: "ready"; operationId: string; generation: number }
        | { status: "failed"; operationId: string; generation: number;
            error: "stopFailed" | "startFailed" | "recoveryRequired" |
                   "operationTimedOut" } }
  | { status: "error"; operationId?: string; error:
      "invalidInput" | "notFound" | "documentUnavailable" |
      "documentConflict" | "contentTooLarge" | "formatFailed" |
      "parseFailed" | "unsupportedNodes" | "unsupportedClashProviders" |
      "normalizationFailed" | "validationFailed" | "configurationFailed" |
      "saveFailed" | "stateUnavailable" | "identityFailed" | "busy" |
      "recoveryRequired" | "operationTimedOut";
      location?: ErrorLocation };

get_running_configuration() =>
  | { status: "ok"; format: "json"; content: string;
      appliedSubscriptionId: string; appliedConfigurationGeneration: number }
  | { status: "error"; error:
      "notRunning" | "recoveryRequired" | "contentTooLarge" | "busy" };
```

`get_running_configuration` is a serialized worker request. It succeeds only when lifecycle is Ready and returns at most 4 MiB from `ConfigSlots.active.as_bytes()` together with the worker's applied ID/generation. It never loads/recompiles AppState and never reads candidate bytes. During a transition the queue orders it after the transition or returns `busy`. Stop/replacement clears any UI-held content. The exact generated JSON contains credentials and the ephemeral Clash API secret by user request; it stays main-window-only and never enters persistence, clipboard automatically, snapshot, event, error or log.

Extend the existing command only:

```ts
activate_subscription({ request: { id: string; force?: boolean } })
```

`force` defaults false. When true it bypasses only `AlreadyCurrent`; it recompiles/checks/prepares and replaces the child from the current persisted projection. It does not increment generation when selection/content is unchanged. Same-ID success returns existing response shape with `outcome:"reactivated"`; a different selected ID remains `outcome:"activated"`. All existing activation error and pending semantics remain closed and authoritative.

## 5. Refresh all, batch delete and running memory

- No bulk backend command is added. Refresh All snapshots current summaries, selects remote IDs only, and invokes existing `update_subscription` sequentially. Manual subscriptions are reported `skipped`; each remote reports success/error and processing continues. Each update retains its own atomic/CAS behavior and does not auto-apply.
- Batch Delete snapshots selected IDs and invokes existing `delete_subscription` sequentially, reporting every result. An active subscription is protected while runtime is Ready, switching or recovery-required. When authoritative runtime is Stopped, deletion may clear the selected ID and increment generation, but must not select another subscription or start/stop a child. Other selected items continue after a refusal.
- Add `coreMemoryBytes: number | null` (`core_memory_bytes: Option<u64>`) to runtime snapshot/delta and the existing event. It is the owned sing-box child's current working set, not Veyra memory. On Windows, call `K32GetProcessMemoryInfo`/`PROCESS_MEMORY_COUNTERS.WorkingSetSize` using the already-owned `Child` handle after identity, Ready and liveness checks. Do not call `OpenProcess`, enumerate processes or expose a PID.
- Sample memory on the existing runtime observation cadence and retain only the latest value in the in-memory observation snapshot. Do not add a second timer or persistent/browser history. Stop/replacement sets it to null. Memory-query failure or a value outside the JavaScript safe-integer range yields null, never zero or a clamped value, and does not discard an otherwise valid traffic sample or stop the child.
- Enable `Win32_System_ProcessStatus` on the existing pinned `windows` dependency; add no crate, listener or platform permission.

## 6. Permissions and implementation ownership map

Add four generated main-only command permissions and include them in `capabilities/default.json`:

1. `allow-get-subscription-document`
2. `allow-format-subscription-document`
3. `allow-save-subscription-document`
4. `allow-get-running-configuration`

The force flag reuses `allow-activate-subscription`; refresh-all/delete reuse existing permissions; the observation field reuses existing snapshot/event permission.

Backend paths: `domain/state.rs`, `storage/migration.rs`, `storage/store.rs`, new `subscription/document.rs` plus `subscription/mod.rs`, `application/subscription_management.rs`, `application/managed_observation_runtime.rs`, `application/observability.rs`, `singbox/runtime.rs`, `singbox/managed_sidecar.rs`, `platform/windows/managed_sidecar_port.rs`, `commands.rs`, `lib.rs`, `build.rs`, `capabilities/default.json`, generated command-permission files, `Cargo.toml`, and directly affected Rust fixtures/tests.

Frontend paths: `src/lib/subscriptions.ts`, `src/lib/observability.ts`, the subscription editor/runtime-view components, subscription/home/sidebar styles and directly affected tests. The already approved Monaco package/lock change is tracked by its producer. No `monaco-yaml`, CDN, schema service or network permission is added.

## 7. Required verification

1. V5→V6 migration and restart preserve all IDs/references/selection/generation and expose `documentUnavailable`; V6 round-trip covers 1 byte/4 MiB/over-limit and remote/manual override invariants.
2. JSON/YAML formatting covers canonical output, syntax location, timeout/single-slot busy and sanitized errors without content. Save-without-Format proves byte-exact comment/layout preservation while still rejecting mixed supported/unsupported, all unsupported, provider-only, duplicate-node, invalid-format, revision-conflict and invalid-state inputs.
3. Inactive success replaces document/nodes atomically. Every pre-save failure preserves exact state bytes. Selected-active save proves check/prepare→save→commit order both while Ready and Stopped; post-save start/stop failure proves the new revision remains and `apply.failed` is honest. Tests cover pre/post-persistence deadline expiry and IPC cancellation, and assert that no Save response contains pending.
4. Remote local override clears validators; manual and scheduler refresh both send no validator, successful 200 overwrites, unexpected 304 fails without changing the override, and later 200 recovers. Ordinary refresh does not auto-apply.
5. Force on the same ID/generation performs a real replacement and changes owned child identity without generation inflation; precheck failure retains the old child. Running-config view returns byte-identical active config and matching applied identity, never a newer candidate; stopped/recovery/busy and size limits are covered.
6. An owned harmless Windows child produces a positive working-set sample through its existing handle; stopped/replaced/failed-query states publish null. Traffic observation remains intact when memory sampling fails. No user subscription or node execution is required.
7. UI/browser cases cover editor conflict/failure retention, saved-but-apply-failed wording, explicit config reveal/close clearing, refresh-all remote skip/per-item results, batch active protection/no implicit selection, and null/zero/positive memory rendering in light/dark themes.
8. Run only Task-specified targeted tests plus `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`, `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`, `pnpm lint`, `pnpm test`, and `pnpm build`. Preserve raw logs and nonzero test counts; do not run a user URL, node or broad core matrix.

## 8. Residual risks and review focus

- Persisting editable source increases private state size and stored credential surface; the existing private state/backup ACL and 4 MiB per-document bound are the controlling boundary. Evidence must prove no raw-content DTO leak outside explicit read/save/view responses.
- Explicit YAML Format removes comments and may alter presentation. Save itself preserves the current editor buffer exactly; semantic conversion remains the acceptance authority.
- A post-persist runtime failure intentionally leaves saved content ahead of the actually applied child. Applied ID/generation plus `apply.failed` must remain visible until force/use succeeds or the core stops.
- Independent review must confirm the V6 downgrade boundary, terminal save deadline behavior, validator suppression, active-slot read ownership and child-handle memory API before freezing this candidate.
