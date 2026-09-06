# Veyra UI Visual Reference

## Evidence boundary

This inventory records both the immutable Clash Verge reference screenshots and the
Veyra native-window verification captures. The Clash files under
`docs/ui/reference/clash-verge/` were not modified.

All listed captures are 1985 × 1434 pixels. The capture host reported 144 DPI
(150%), so the comparable WebView client is approximately 1323 × 956 logical pixels.
The PNG metadata supports the DPI value but does not identify the Windows edition;
the exact Windows edition is therefore `UNKNOWN`.

## Clash Verge reference inventory

| Screenshot filename | Platform | Display scale | Window size | Theme | Language | Sidebar state | Primary acceptance target |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `windows-light/app-shell.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Light | Simplified Chinese | Expanded | Shell geometry, 200 px logical sidebar, 58 px header and background hierarchy |
| `windows-dark/app-shell.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Dark | Simplified Chinese | Expanded | Dark shell/sidebar/page separation, divider and selected navigation |
| `windows-light/settings.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Light | Simplified Chinese | Expanded; Settings selected | Future Settings composition reference only; not the current Golden Page |
| `windows-dark/settings.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Dark | Simplified Chinese | Expanded; Settings selected | Future dark Settings reference only; implementation waits for an approved Settings Domain |
| `windows-light/proxies.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Light | Simplified Chinese | Expanded; Proxies selected | Dense group/list reference; outside this implementation round |
| `windows-light/subscriptions.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Light | Simplified Chinese | Expanded; Subscriptions selected | First production Golden Page: shell, header, toolbar, grid, card density, selection and controls |
| `windows-light/conn.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Light | Simplified Chinese | Expanded; Connections selected | Dense table reference; outside this implementation round |
| `windows-light/dialog.png` | Windows (edition UNKNOWN) | Host observed 150%; PNG ≈143.99 DPI | 1985 × 1434 px; ≈1323 × 956 logical px | Light | Simplified Chinese | Expanded | Auxiliary reference for backdrop, dialog shell, fields and footer alignment |

## Reference measurements used for Subscriptions

These are bitmap observations used to validate values already frozen in
`veyra-ui-spec.md`; they are not newly invented tokens.

- Sidebar divider: physical x = 299, corresponding to 200 logical pixels.
- Page inset: 15 physical pixels, corresponding to 10 logical pixels.
- Subscription toolbar controls: approximately 54 physical pixels high,
  corresponding to 36 logical pixels; horizontal gap is approximately 8 logical pixels.
- Reference subscription cards: approximately 270 logical pixels wide and 99 logical
  pixels high, with 8 logical pixels radius and a 3-pixel selected edge.
- Reference light hierarchy: shell `#F5F5F5`, page `#ECECEC`, white content surfaces,
  primary `#007AFF`, selected sidebar `#D0E2F6`; cards do not use an outer shadow.
- `windows-light/dialog.png`: dialog shell is approximately 498 × 665 logical pixels,
  with an 8-pixel radius, 1-pixel border and approximately 50% backdrop.

## Veyra production capture inventory

All captures below were made from an actual Tauri Windows WebView at the same client
size and display scale as the reference. The isolated capture state uses Veyra's real
subscription model and interaction paths; it does not add production fields.

| Screenshot filename | Platform | Display scale | Window size | Theme | Language | Sidebar state | State and acceptance target |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `windows-light/subscriptions.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Light | Simplified Chinese | Expanded; Subscriptions selected | Two real manual subscriptions, one active; baseline Golden Page geometry and selected state |
| `windows-light/subscription-dialog.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Light | Simplified Chinese | Expanded | Real New Subscription dialog, backdrop, inputs, select and focus |
| `windows-light/subscription-context-menu.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Light | Simplified Chinese | Expanded | Actual card context-menu invocation and compact menu surface |
| `windows-dark/subscriptions.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Dark | Simplified Chinese | Expanded; Subscriptions selected | Dark hierarchy, grid, controls and active card |
| `windows-dark/subscription-dialog.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Dark | Simplified Chinese | Expanded | Dark real dialog shell and form controls |
| `windows-dark/subscription-busy.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Dark | Simplified Chinese | Expanded | Delayed real quick-import request; inline `导入中` and disabled conflicting actions |
| `windows-dark/subscription-notice.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Dark | Simplified Chinese | Expanded | Actual empty quick-import validation Notice with an empty input |
| `windows-dark/subscription-imported.png` | Windows (Tauri WebView) | 150% | 1985 × 1434 px; 1323 × 956 logical px | Dark | Simplified Chinese | Expanded | Result of isolated real quick-import path; imported subscription card visible |

## Development-only Visual Harness

The harness is available only when `import.meta.env.DEV` is true and the query is
`?visual-harness=settings`. It is loaded dynamically, has no navigation entry, imports
no IPC/business module and is absent from the production bundle. Labels explicitly say
that the content is a visual fixture, not a Veyra setting.

| Screenshot filename | Platform | Display scale | Window size | Theme | Sidebar state | Acceptance target |
| --- | --- | --- | --- | --- | --- | --- |
| `visual-harness/settings-primitives.png` | Windows (Tauri WebView, dev only) | 150% | 1985 × 1434 px | Light | None | SettingSection, SettingRow, optional Description, Switch on/off/disabled, Select, Chevron, inline loading, Button, IconButton, Input and Notice |
| `visual-harness/settings-dialog.png` | Windows (Tauri WebView, dev only) | 150% | 1985 × 1434 px | Light | None | Minimal dialog shell, field focus, footer alignment and buttons |

## Settings gate

Settings is currently a truthful placeholder. Runtime status, observation, connection
count, config revision and runtime memory are not Settings content. A production
Settings page may be composed only after product and technical review freezes its
fields, state ownership, persistence, IPC, platform behavior, and error/loading
semantics.
