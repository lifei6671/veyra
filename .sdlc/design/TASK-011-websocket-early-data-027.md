# TASK-011 WebSocket early-data compatibility 027

Status: `CANDIDATE`

## 1. Authority and purpose

This candidate is the minimum model and compiler extension needed to complete the user's requested repair of the affected subscription. The user already authorized repairing this subscription and selected the sing-box User-Agent path; earlier compatibility work also authorized the necessary domain/compiler expansion. Therefore this candidate requires independent technical-design review, but no repeated user approval, new dependency, capability permission, or product setting.

It extends, rather than rewrites, the frozen TASK-011 subscription policies:

- `.sdlc/design/TASK-011-subscription-policy-025.md` (`8a67b971579b7b348ac4fd04921b9f90a5d1e606088be086ec8b9d6544a186fe`)
- `.sdlc/design/TASK-011-subscription-policy-frozen-025.json` (`4f38958326faf105ff29c8ac51fb32e783bfa704eb68d4d8b80c2da1c4ed2500`)
- `.sdlc/evidence/TASK-011/websocket-shape-027.json` is the credential-free representative shape: ten WebSocket nodes, each with `max_early_data = 2048`, `early_data_header_name = "Sec-WebSocket-Protocol"`, and a one-element `headers.Host` array.

The current implementation accepts 31 of 41 sing-box outbounds; the remaining ten valid WebSocket nodes are rejected because `Transport::Websocket` stores only `path` and scalar `host`, while the parser rejects the two early-data fields and the Host array representation. The eight duplicate nodes are governed separately by frozen policy 025 and are not changed here.

## 2. External semantics and reference mapping

sing-box WebSocket transport defines `path`, `headers`, `max_early_data`, and `early_data_header_name`. A nonzero `max_early_data` enables early data; `early_data_header_name` selects the header used to carry it and must match the server. The official documentation also permits an array-valued option to be represented by its single item. Source: [sing-box V2Ray transport](https://sing-box.sagernet.org/configuration/shared/v2ray-transport/).

The local satelite-proxy reference already preserves `max_early_data` in its WebSocket model, parses the underscore/dash spellings, and emits the value:

- `E:/wx_lifeilin/github.com/satelite-proxy/src-tauri/src/domain/node.rs:133`
- `E:/wx_lifeilin/github.com/satelite-proxy/src-tauri/src/subscription/singbox.rs:286`
- `E:/wx_lifeilin/github.com/satelite-proxy/src-tauri/src/subscription/clash.rs:839`
- `E:/wx_lifeilin/github.com/satelite-proxy/src-tauri/src/config/builder.rs:1891`

That reference does not retain `early_data_header_name`; Veyra must preserve it because the observed value is nonempty and changes the wire handshake. Copying the omission would produce a configuration with different behavior.

## 3. Persistent domain contract

Extend only the existing enum variant:

```rust
Transport::Websocket {
    path: String,
    host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_early_data: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    early_data_header_name: Option<String>,
}
```

Validation is closed and deterministic:

- `max_early_data = None` means disabled; parser input `0` is normalized to `None`.
- `Some(value)` requires `value > 0`. Negative, fractional, overflowing, string, or boolean values are invalid.
- `early_data_header_name = None` means sing-box's default path carriage and is valid with positive early data.
- Parser input containing an empty header-name string is normalized to `None`, matching sing-box's default value. A persisted `Some(header_name)` must be nonempty, requires positive `max_early_data`, and must be a valid ASCII HTTP field-name token. Whitespace-containing, control-containing, or otherwise invalid nonempty names are rejected.
- These fields are valid only on WebSocket transport. Existing protocol eligibility, path, and host checks remain unchanged.

No schema version migration is required. Existing V5 documents omit both fields and deserialize them as `None`; serializing those values omits both keys. Because stable node identity includes the serialized transport, an old WebSocket node retains the exact identity material and ID. A node with nondefault early-data semantics includes the new values in identity material, which correctly distinguishes a connection with a different handshake. The existing policy does not support older binaries as writers of newer state, so a version bump solely for downgrade detection would add no accepted behavior.

## 4. Parser contract

For sing-box `transport.type = "ws"`, accept the canonical keys `max_early_data` and `early_data_header_name`. For Clash `ws-opts`, accept the equivalent underscore and dash spellings used by the local reference; if both spellings are present they must carry the same typed value, otherwise the node is invalid.

`headers.Host` is accepted in exactly two equivalent forms:

- a nonempty string;
- an array containing exactly one nonempty string.

Both normalize to the existing scalar `host`. Empty arrays, arrays with more than one value, non-string entries, or multiple distinct Host values are rejected. The parser must never select the first value and discard the rest. Other WebSocket header names remain outside the model and therefore remain fail-closed.

The parser retains every meaningful early-data value. It may canonicalize only explicit zero to disabled `None`, an empty header name to absent, equivalent field aliases to one value, and a one-element Host array to its sole scalar. A nonempty header without positive early-data, conflicting aliases, invalid types, and unknown nested fields remain `InvalidNode`; they are not reclassified as a supported skip or imported partially.

## 5. Compiler and round-trip contract

Add the same optional fields to `CoreTransport::Websocket`, with omission only for `None`. `core_transport` emits the domain values unchanged together with `path` and optional scalar Host.

The final-config typed deserialization used by `validate_final_config_document` must retain both fields. Validation rejects a document whose early-data/header relationship violates section 3. Thus the following round-trip is lossless for accepted state:

```text
subscription response
  -> Transport::Websocket
  -> sing-box JSON transport
  -> typed final-config reconstruction
```

This change does not add a listener, change a protocol implementation, alter runtime lifecycle, or relax unrelated final-config validation. Compiler/preflight failure continues to preserve the last-known-good configuration and running child through the existing TASK-011 transaction boundary.

## 6. Implementation inventory

Production changes are limited to:

- `src-tauri/src/domain/state.rs`: add optional fields and domain validation; mechanically update in-file WebSocket constructors.
- `src-tauri/src/subscription/parser.rs`: parse the exact fields/aliases and strict Host representations; mechanically update in-file constructors.
- `src-tauri/src/singbox/compiler.rs`: emit and reconstruct both fields; mechanically update in-file domain/core constructors.

The stable identity algorithm in `src-tauri/src/subscription/normalize.rs` already serializes the complete transport and needs no production change. A focused regression may be added there only to bind old-ID stability and new-field differentiation. Persistence continues through the existing V5 generic serde representation; storage production code and migration chain remain unchanged.

No dependency, lock file, Tauri capability, IPC shape, scheduler, proxy setting, or UI file changes are part of this candidate.

## 7. Verification and acceptance

Use only credential-free synthetic fixtures and focused Veyra tests:

1. Parse the safe shape equivalent to `websocket-shape-027.json` and prove all ten WebSocket nodes retain `2048`, `Sec-WebSocket-Protocol`, and the single Host value.
2. Prove string Host and one-element array Host normalize identically; empty, multi-value, mixed-type arrays and unsupported header keys fail without dropping values.
3. Prove zero/positive/boundary early-data handling, alias equality/conflict, invalid numeric types, invalid header names, and header-without-positive-max behavior.
4. Prove old V5 WebSocket JSON lacking the new keys loads and serializes without the keys, and preserves its prior stable node ID. Prove changing either new semantic field changes the stable ID.
5. Prove compiler JSON contains both accepted values, typed final-config reconstruction retains them, and inconsistent or malformed values fail validation.
6. Run the precise parser, normalization, persistence, and compiler test targets plus repository-required Rust format and clippy checks. Record nonzero test counts and exact exit codes.

Do not access the user's subscription URL, run real nodes, start sing-box, or add protocol/network conformance tests. These checks establish Veyra's lossless mapping and failure behavior; they do not revalidate sing-box WebSocket behavior.

## 8. Risks and residual boundaries

- Accepting a multi-value Host array would require a broader domain/compiler representation and server-semantics decision. This candidate rejects it without data loss.
- An invalid early-data node remains a whole-node parse failure under the existing strict import contract; no partial transport downgrade is allowed.
- Because semantic fields participate in stable identity, a previously rejected node receives its first ID when it becomes importable. Existing WebSocket nodes with absent fields keep their IDs.
- Concurrent import/update, persistence failure, cancellation, and last-known-good preservation use the already approved SubscriptionManager gate and transaction behavior; this candidate introduces no separate concurrency mechanism.
