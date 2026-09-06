# TASK-012 Visual Parity Review

## Evidence boundary

- Reference: `docs/ui/reference/clash-verge/windows-light/proxies.png` (`sha256:a80118a55e699579692394114eb942ca81a085370544a6b4062ed26d33cd132c`).
- Dialog reference: `docs/ui/reference/clash-verge/windows-light/dialog.png` (`sha256:ea82ef2fe102b48d35c16ce92b986fafc62c6820a4f5a3876d71477bdd45aca3`).
- Veyra: actual Windows Tauri/WebView2 captures at 150% display scale. The 960×640 CSS viewport capture is 1462×1016 physical pixels including the native frame; the narrow 520×520 capture is 802×836.
- Source of truth: `docs/ui/veyra-ui-spec.md`. Clash Verge supplies composition and visual-language evidence only; Veyra does not copy Mihomo-specific latency, group, provider or proxy-card fields.

## Comparison

| Dimension | Clash Reference | Veyra | Difference | PASS/FAIL |
| --- | --- | --- | --- | --- |
| Shell geometry | Expanded left rail, fixed top page header and independently scrolling canvas. | The 960 capture uses the frozen 200 px sidebar, 58 px header and independent content canvas. | No geometry divergence from the Veyra token contract; native title-bar height is platform-owned. | PASS |
| Sidebar | Expanded icon-and-label items with one tinted selected row. | Same expanded state, icon/label alignment and a single tinted selected row for 出口组/分流. | Veyra retains its own page names and logo. | PASS |
| Page header | One-line page title with actions aligned on the trailing edge. | 出口组/分流 title and status/action controls share one 58 px row. | Veyra adds an explicit desired/applied state chip required by Save→Apply. | PASS |
| Content density | Multiple proxy-group surfaces begin immediately below the header and use most available width. | Pool surface and all-node surface start 10 px from the header/canvas edge and span the canvas. | Fewer surfaces reflect the two-subscription fixture; no placeholder groups are fabricated. | PASS |
| Grid geometry | Responsive multi-column proxy cards within group surfaces. | All-node cards use a responsive grid; pool members use dense full-width rows because selection state and read-back feedback are row-level. | The row treatment is a Veyra business-composition decision, not a geometry defect. | PASS |
| Surface | Low-elevation white/light surfaces on a grey canvas. | White/light surfaces on the specified canvas background with divider-defined hierarchy. | No extra glass, gradient or elevated dashboard shadow. | PASS |
| Card density | Compact proxy cards carry name, type and latency metadata. | Compact node cards carry only name and protocol; pool rows carry selection/runtime feedback. | Latency and Mihomo metadata are intentionally absent because TASK-012 has no authoritative source for them. | PASS |
| Typography | Page title, section title, primary row label and muted metadata form four distinct levels. | The same hierarchy is implemented with Veyra token sizes/weights and muted secondary text. | Chinese labels can occupy more width; row height remains token-bound. | PASS |
| Toolbar | Reference actions remain in the page-header trailing region. | New pool and Apply remain in the page-header trailing region; dirty state is adjacent. | Veyra exposes explicit Apply because structural edits are Save→Apply. | PASS |
| Control dimensions | Compact buttons and icon-only menu affordances. | Buttons, menu trigger, select/input and dialog controls use the frozen Veyra control heights and icon sizes. | No per-page control sizing is introduced. | PASS |
| Border | One-pixel neutral dividers define surfaces and rows. | One-pixel token borders/dividers define cards, rows and dialogs. | Selected emphasis uses color, not a heavier border. | PASS |
| Radius | Restrained surface and control radii. | Surface and control radii use the frozen Veyra tokens. | No pill-shaped content cards or oversized radii. | PASS |
| Background hierarchy | Sidebar, header, canvas and surface are visibly distinct but close in luminance. | The same four layers are distinguishable in both captured themes. | Dark theme uses the Veyra token mapping rather than sampled one-off colors. | PASS |
| Selected state | Selected proxy/card uses a stable accent treatment. | Selected Manual member uses accent background, marker and authoritative read-back text. | Veyra does not display fabricated latency status. | PASS |
| Disabled state | Disabled controls reduce emphasis and stop interaction. | During actual Apply, New Pool and Apply are visibly disabled while the state chip says 正在应用. | The disabled state is tied to a real operation rather than a presentation-only fixture. | PASS |
| Inline pending | Reference communicates operations at the affected control/group. | The clicked Manual member receives `aria-busy`, pending styling and `切换中…`; selected state is not patched locally. | The local loopback read-back completed before a stable screenshot frame; component markup tests and the real final selected/read-back capture cover the transient and terminal states separately. | PASS |
| Loading / empty / error | Group/page content retains shell geometry while its state content changes. | Page-level SSR tests cover loading, empty and error/Notice without changing shell geometry. | These deterministic fixtures are test-only and are not production settings or fake persisted data. | PASS |
| Dialog | Centered modal with dim backdrop, compact fields and trailing Cancel/Confirm actions. | Pool and Route dialogs reproduce that shell, focus order and action alignment in actual Tauri captures. | Fields and copy are Veyra Pool/Route domain fields only. | PASS |
| Context menu | Compact anchored menu tied to the row/card action. | Actual right-click opens the Pool menu at the pointer with 编辑/删除 and existing dropdown tokens. | Menu actions are limited to Veyra edit/delete behavior. | PASS |
| Notice | Reference uses transient foreground feedback without changing page layout. | Apply, selector and error outcomes render through the existing bottom-corner Notice shell. | Text reflects authoritative Veyra outcomes. | PASS |
| Narrow window | Reference desktop composition preserves hierarchy as width contracts. | At 520×520 CSS px the page becomes a single-column composition with no horizontal overflow. | Header actions wrap/condense according to Veyra spec; sidebar remains unchanged. | PASS |

## Veyra evidence

- `docs/ui/reference/veyra/windows-light/task012-final-proxies-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-proxies-light-520.png`
- `docs/ui/reference/veyra/windows-dark/task012-final-proxies-dark-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-pool-dialog-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-routing-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-routing-dialog-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-applying-busy-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-context-menu-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-loading-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-query-error-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-apply-failed-old-ready-light-960.png`
- `docs/ui/reference/veyra/windows-light/task012-final-runtime-exit-refresh-light-960.png`

No FAIL remains in the reviewed visual dimensions. This statement is limited to TASK-012 composition, visual states and the captured viewports; it is not product acceptance and does not promote the Delivery Gate.
