# Subscriptions Golden Page — Visual Parity Review

## Evidence and method

- Primary reference: `docs/ui/reference/clash-verge/windows-light/subscriptions.png`.
- Auxiliary dialog reference: `docs/ui/reference/clash-verge/windows-light/dialog.png`.
- Veyra production captures: `docs/ui/reference/veyra/windows-light/` and
  `docs/ui/reference/veyra/windows-dark/`.
- Primitive-only captures: `docs/ui/reference/veyra/visual-harness/`.
- Both products were compared at 1985 × 1434 physical pixels, 150% display scale,
  approximately 1323 × 956 logical pixels, expanded sidebar and the Subscriptions page.
- The primary and implementation captures were placed side by side before judgment;
  screenshots were not treated as a substitute for the comparison below.

## Production-page parity

| Dimension | Reference | Veyra | Difference | PASS/FAIL |
| --- | --- | --- | --- | --- |
| Shell geometry | 200 px sidebar; 58 px header; client-only capture | Same 200 px sidebar, 58 px header and client bounds | No measured geometry difference | PASS |
| Sidebar | Light surface, 1 px divider, compact items, blue selected fill | Same hierarchy, item geometry and selected treatment | Product logo and destination set remain Veyra-owned | PASS |
| Page header | Single-line title in fixed header; low-emphasis divider | Same title position, height, weight and divider | Title is Veyra's own `订阅` label | PASS |
| Content density | 10 px page inset and dense toolbar-to-grid rhythm | 10 px inset; toolbar and cards occupy the top canvas continuously | Veyra omits Clash-only Merge/Script business cards | PASS |
| Grid geometry | Auto-fill cards around 270 px wide with 8 px gap | `minmax(260px, 1fr)` produces comparable card widths and 8 px gap | Final column count follows Veyra's real subscription count | PASS |
| Surface | White/light raised content layer, 1 px border, no shadow | Same background role, border and no-shadow treatment | None visible at comparison size | PASS |
| Card density | Approximately 99 px high; compact metadata/actions | Minimum 100 px; empty optional description/traffic rows collapse | Veyra shows only fields present in its model | PASS |
| Typography | 18 px medium title; compact body/metadata hierarchy | 18/600 title and shared body/metadata tokens | System-font glyph widths differ slightly; hierarchy matches | PASS |
| Toolbar | 36 px input/buttons; 8 px gaps; compact right actions | Same dimensions and gaps | Empty import stays clickable and returns Veyra validation Notice; existing behavior preserved | PASS |
| Control dimensions | 36 px standard controls; approximately 32 px icon targets | 36 px input/buttons, 32 px icon buttons, 20 px glyphs | No measured control-height difference | PASS |
| Border | Low-emphasis 1 px dividers and card borders | Shared 1 px divider token | None visible | PASS |
| Radius | 8 px cards and dialog shell | Shared 8 px surface/dialog radius | None visible | PASS |
| Background hierarchy | `#F5F5F5` shell/sidebar, `#ECECEC` page, white surface in light | Same light hierarchy; corresponding frozen dark tokens in dark capture | Dark comparison uses Veyra screenshot because no dark subscription reference was supplied | PASS |
| Selected state | 3 px primary edge and primary title | Active card uses 3 px primary edge and primary title | Selection data is Veyra's active subscription | PASS |
| Disabled state | Muted foreground/background and suppressed action emphasis | Real busy capture disables conflicting toolbar/card actions; harness covers disabled Switch/Button | Production disabled state is triggered by Veyra busy state, not a fabricated setting | PASS |
| Loading / busy | Inline progress replaces the initiating action state | Real delayed import shows `导入中` in place and locks conflicts | Wording and state owner remain Veyra-specific | PASS |
| Dialog | ≈498 px wide, 8 px radius, 1 px border, ~50% backdrop, dense form/footer | 520 px existing expanded shell, same radius/border/backdrop and compact controls | 22 px wider; fields and height are content-driven by Veyra's real create flow | PASS |
| Context Menu | Exact visual value is `UNKNOWN` in the supplied subscription reference | Real card menu uses compact surface, 1 px border, 8 px radius and shared states | No reference menu is visible; judged only against frozen Veyra tokens | PASS |
| Notice | Exact visual value is `UNKNOWN` in the supplied reference | Actual validation Notice uses shared error surface and spacing | No reference Notice is visible; no Clash behavior was invented | PASS |

## State coverage

- Normal, active/selected, dialog, context menu and validation Notice were exercised
  through the production Subscriptions page.
- Busy/disabled was exercised through a delayed local subscription response using the
  real quick-import path and an isolated Veyra data directory.
- Empty/error/loading dispatcher behavior remains owned by the existing Subscription
  state machine. No state, query, IPC, event, focus, keyboard or DocumentEditor contract
  was rewritten in this visual pass.
- The development-only harness covers primitives that currently lack an honest product
  call site: SettingSection/Row, Description, Switch on/off/disabled, Select, Chevron,
  inline loading, buttons, input, dialog and Notice. It is not production information
  architecture and is not bundled into production.

## Settings disposition

The rejected runtime/observation Settings composition was removed. Settings now remains
a concise placeholder until its real Domain contract is approved. No ThemePreference,
AutoStart, Update, CaptureMode, persistence, IPC or platform capability was introduced.

## Verification

- `pnpm lint`: passed.
- `pnpm test`: passed, 7 files / 99 tests.
- `pnpm build`: passed; the existing large-chunk warning remains non-blocking.
- Production bundle scan for harness labels/module/query markers: no match.
- `git diff --check`: passed; Git emitted only existing CRLF conversion warnings.
- Native Windows evidence: light/dark, dialog, busy/disabled, context menu, Notice and
  development harness captures generated at the reference geometry.

Human visual acceptance is still pending. Proxies and all other page migrations remain
out of scope until that acceptance.

final result: passed
