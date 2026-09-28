# OpenBox React Web — Design QA

## Result

passed

## Scope

- Reference: live OpenBox page in the user's Edge browser.
- Implementation: `openbox.html`, `src/openbox/**`, and the Vite `/api` proxy.
- Viewport: 2552 × 1274 in the same Edge tab for the final overview and settings comparisons.
- Comparison sheet: `C:/Users/lifei/.codex/visualizations/2026/09/28/01a0e6d6-9864-7cc1-8978-42e069bded89/fidelity-qa/comparison-contact-sheet.jpg`.

## Comparison runs

- Run 1 aligned the 256 px sidebar, 48 px top bar, background asset, translucent cards, primary page spacing, route controls, proxy cards, data tables, rules and settings layout.
- Run 2 matched the overview and traffic card dimensions, monthly/day chart geometry, chart labels, table structure, settings fixed columns and log select widths to live-page measurements.
- Run 3 imported the packaged `MiSans-VF` subsets and `NotoEmoji` font, matched the live 14 px / 20 px control and table typography, and replaced visible native selects, switches, segmented filters and search groups with locally themed shadcn-style Radix primitives.

## Interaction checks

- Navigation switches all six pages.
- The login screen reads `/api/auth/status` from `http://192.168.1.10:3036` through the local Web proxy and submits the real authentication request.
- Proxy selection, policy-group latency tests, connection closing, subscription CRUD, profile/group/DNS saves, service control and password changes call the real backend endpoints.
- Connections, logs, traffic and memory use the real controller WebSocket streams.
- Closing a connection keeps a local history of the real connection closed by this Web session.
- Log filtering, pause/resume and downloads operate on received backend log frames.
- Rule filters operate on `/api/controller/rules` data.
- Light/dark theme selection persists through `/api/storage`.
- Select menus expose keyboard-accessible options and update controlled values.
- Switches expose checked state and segmented radio groups update their associated page content without dangling `aria-controls` references.

## Verification

- `pnpm lint`: passed.
- `pnpm test`: 12 files and 359 tests passed.
- `pnpm build`: passed, including `openbox.html`.
- `GET http://127.0.0.1:1420/api/auth/status`: returned the real backend state with HTTP 200.
- Both `pnpm dev` and `pnpm preview` proxy `/api` and controller WebSockets to `http://192.168.1.10:3036`; a separate static host must provide the same reverse proxy.
- Edge visual check: the local page rendered authenticated real data for overview, proxies, connections, logs, rules and every settings section; the shared refresh action returned a success status.

## 2026-09-28 Logs, proxies and settings correction

- Source truth: live `http://192.168.1.10:3036/#/logs`, `#/proxies` and `#/settings` in Edge.
- Implementation: `http://127.0.0.1:1420/openbox.html#/logs`, `#/proxies` and `#/settings` at the same 2558 × 1276 desktop viewport.
- Logs: direct filter toolbar, dynamic type selector, Regex input and invalid state, URL/host formatter, download, pause/resume and clear controls match the source structure.
- Proxies: the real backend renders 13 strategy groups, 2 node groups and 1 subscription; strategy selection, group/node/subscription latency actions, refresh and edit navigation are wired to real endpoints.
- Settings: the nine-item horizontal navigation and matching icons, active state, panel dimensions, labels, selects, switches and test-site layout were visually compared with the live source.
- Typography: the bundled MiSans variable font is active for text/form controls; the separate emoji/flag font remains bundled.
- Backend: JSON bodies returned as `text/plain` are parsed and displayed instead of being treated as strings.
- Review fixes: panel saves are serialized so rapid blur events cannot reorder snapshots; test-site icons are non-interactive; proxy delay pills remain clickable re-test controls; radius/background/column preferences are consumed by the live UI; unsupported language choices were removed and the actual `zh-CN` request locale is synchronized.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 374 tests), `pnpm build` and `git diff --check` passed after final review fixes.

## 2026-09-28 Overview correction

- Reference and implementation were captured at the same 2558 × 1276 Edge viewport with current real-backend data.
- The source hierarchy is restored: four interactive site-latency cards, a six-value status strip, four independently pausable realtime charts, and the full traffic-insight panel.
- Site cards run a single-site test; the heading action retests all sites; every chart has its own pause/resume control.
- Month navigation, day bars, hourly filtering, restore-day action, dimension tabs, search, expandable rows and both drill dimensions call the real traffic endpoints.
- Browser interaction checks covered chart pause/resume, hour 15 filtering, client-to-host drill, client-to-node drill and August/September navigation.
- Dynamic chart scales, MiB memory formatting, stacked inbound/outbound bars, percentage denominators, latency history marks and the overview-only promotion strip now follow the live source.
- Final review fixes reject stale drill responses after a newer row/dimension request and keep table headers aligned with data columns by removing the obsolete competing rules.

final result: passed

## 2026-09-28 Proxy toolbar alignment correction

- Source truth: `codex-clipboard-966dc53a-9cc3-4cef-b1cf-7a98d8577691.png` supplied by the user.
- Implementation: `http://127.0.0.1:1420/openbox.html#/proxies` in Edge at the current desktop viewport.
- The sidebar collapse control keeps its existing 36 × 36 click target and source icon but no longer paints a separate background in rest, hover or focus states, including dark theme.
- The proxy segmented group and search input now both measure 40 px high, share an 8 px top position and use the same 11 px outer radius. The proxy toolbar remains 40 px high, so neither control changes the surrounding layout.
- Focused Edge comparison confirmed the transparent top-left icon control and equal-height proxy tabs/search field. No API, navigation or interaction behavior changed.
- Verification: `pnpm lint`, `pnpm build` and `git diff --check` passed. CSS-only visual change; no unit-test behavior changed.
- Human Visual Approval remains pending; this focused implementation and browser comparison passed.

final result: passed

## 2026-09-28 Sidebar collapse control correction

- Source truth: the user-provided crop and the live Edge page at `http://192.168.1.10:3036/#/overview`.
- The control now uses the source SVG geometry (`24 × 24` viewBox, `20 × 20` rendered icon), a `36 × 36` button and the measured neutral background/radius.
- Expanded state shows `收起侧边栏` below the control; collapsed state shows `展开侧边栏` to its right. Both states expose the same accessible button label and avoid a duplicate native tooltip.
- Edge interaction checks covered expanded/collapsed clicks, hover-only tooltip visibility, immediate hiding after collapse and final collapsed resting state at the 2552 × 1228 desktop viewport.
- Browser console warnings/errors: none. Focused visual result: passed.

## 2026-09-28 Proxy latency history correction

- Source truth: the strategy-card latency tag and long-hover history timeline on the live Edge page at `http://192.168.1.10:3036/#/proxies`.
- Strategy cards load their real history from `/api/openbox/latency-history`; the newest ten records are shown first and `delay: 0` is rendered as `超时` rather than omitted.
- The latency pill remains a working group-speed-test control. A deliberate one-second hover opens the dark timeline with node name, second-precision timestamp, success/timeout marker and result.
- Every strategy card keeps the lightning glyph in its top-right pill regardless of whether its latest history entry succeeded; numeric latency is confined to the history tooltip.
- The three page-level proxy actions expose custom hover-only title tooltips and do not duplicate the browser-native `title` popup.
- Edge check at the live 2552 px viewport confirmed a `40 × 20` Speed pill, a `214 × 340` ten-row tooltip above sibling cards, and immediate hiding after pointer exit; all three toolbar titles were checked individually.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 381 tests), `pnpm build` and `git diff --check` passed.

## 2026-09-28 Proxy policy penetration correction

- Source truth: the user-provided expanded Speed card and the live Edge `策略穿透` interaction at `http://192.168.1.10:3036/#/proxies`.
- `策略穿透` is shown only when the selected item is another proxy group. Opening it keeps the parent options visible, changes the control to `收起穿透`, and renders the selected group's real node list below a divider.
- The nested group uses the live controller graph (`group.now` → selected group → `all` nodes); node selection and latency tests keep using the existing real backend actions.
- Edge interaction check at the 2546 × 2013 desktop capture confirmed that Speed and AI can expand independently, resolve `所有-手动` and `所有-自动` respectively, and each render 11 real nodes. Measured nodes are ordered by latency and untested nodes remain after them.
- Ten strategies whose current selection is a proxy group expose the control; Apple, 国内 and 其他 currently select the terminal `直连` node and therefore do not show a non-functional penetration control.
- Collapse/expand was checked in Edge: the nested region is removed and restored while the parent strategy options remain stable.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 384 tests), `pnpm build` and `git diff --check` passed.

## 2026-09-28 Proxy penetration latency feedback

- Source truth: the two user-provided captures showing the penetration-group latency action in progress and after completion.
- The Open-Box entry now uses the shadcn-recommended Sonner notification component, adapted to the existing Heroicons and Open-Box semantic theme variables instead of introducing a second Tailwind theme system.
- Notifications are fixed at the viewport's top center. The running state shows `0/N 测试完成`; the same notification updates to a success/warning/error result and remains visible for the configured completion duration.
- Both right-side controls for the selected nested group expose `aria-busy` and replace the lightning glyph with the existing rotating refresh glyph while that real backend group test is active. Reduced-motion users receive the static glyph.
- Edge check against `http://127.0.0.1:1420/openbox.html#/proxies` used the real `所有-自动` group: the running state showed `0/11`, then the completion state showed `6 成功、5 失败（超时 5）`; node delays refreshed from the backend and the icon returned to lightning.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 386 tests), `pnpm build` and `git diff --check` passed.

final result: passed

## 2026-09-28 Proxy latency Toast visual refinement

- Scope is visual-only: the existing shadcn/Sonner lifecycle, real backend requests, completion timing and top-center placement are unchanged.
- The plain white surface is replaced by compact state cards: cyan for running/info, mint for success, amber for partial failure/timeout and coral for errors. Each state uses a restrained gradient, colored leading rail, icon well, themed border and softer layered shadow.
- The close control now sits inside the top-right corner, while title, result text and the MiSans stack retain the existing Open-Box hierarchy.
- Edge checks covered the light running state, light timeout-warning state, dark running state and dark backend-error state. The theme was restored to the user's original light setting after verification.
- Real backend evidence: one light test completed with `6 成功，5 失败（超时 5）`; the dark error path rendered the live `请求失败 (502)` response without changing error semantics.
- Human Visual Approval remains pending; this entry records implementation and browser evidence only.

final result: ready for human visual approval

## 2026-09-28 Proxy node action and Toast icon correction

- Source truth: `codex-clipboard-209da0a9-ba32-426f-865c-f5c69b8f8e16.png`, `codex-clipboard-6dd38d0e-da9c-4fc9-b79f-6689de98f670.png` and `codex-clipboard-0dbea441-bded-4d9a-9ac1-3f82446cba75.png` supplied by the user.
- Implementation: `http://127.0.0.1:1420/openbox.html#/proxies` in Edge at the current 2556 × 1440 desktop viewport, using the real backend.
- Sonner's icon slot now owns the 30 px state well and contains a separate centered 17 px Heroicon. The loading and completion captures both keep the glyph centered instead of showing an empty or displaced icon.
- Nested automatic (`URLTest`) node bodies are read-only; their accessibility tree exposes no selection controls. Nested manual (`Selector`) node bodies expose explicit selection controls.
- Every nested node has an independent bottom-right latency button. Edge verification selected `订阅 | 美国-05` through the manual card body, tested it through only the latency badge without changing the selection, then restored Speed to `所有-手动 → 订阅 | 美国-04`.
- Delay badges use the configured low/medium thresholds to render success, warning and danger states; the live measured nodes rendered the success color and unit tests cover all three ranges.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 388 tests), `pnpm build` and `git diff --check` passed.
- Human Visual Approval remains pending; this focused implementation and browser comparison passed.

final result: passed

## 2026-09-28 Proxy node two-level collapse correction

- Source truth: `codex-clipboard-9450ecb6-a926-42e3-a016-cc65b65570c2.png` plus direct interaction with the live Edge page at `http://192.168.1.10:3036/#/proxies`.
- The obsolete bottom `收起节点` / `展开节点` control is removed. The entire upper card header is now the outer collapse target.
- Outer collapse hides the complete subscription section and leaves the group header plus ordered latency-status dots. Reopening the outer card restores the provider section in its fully expanded state.
- The `订阅` heading is a separate inner collapse target. Its collapsed state keeps the heading and right-side group latency action visible while replacing only the node cards with status dots.
- Node cards and status dots use the same measured-latency ordering as the source: measured nodes first by delay and untested nodes last. Node-only card/header icons that are absent from the source were removed.
- Edge checks covered both collapse paths, outer reopen/reset behavior and the upper latency action. The latency action kept the card expanded, completed against the real backend and showed the existing completion Toast.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 390 tests), `pnpm build` and `git diff --check` passed before this documentation-only evidence entry.
- Human Visual Approval remains pending; the focused implementation and browser comparison passed.

final result: ready for human visual approval

## 2026-09-28 Proxy node card minimum-width correction

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-07b513b8-f0ff-4120-865f-744b0e3a2dd8.png` (`2910 × 324` physical pixels), showing the fully expanded `所有-自动` node group.
- Implementation evidence: `http://127.0.0.1:1420/openbox.html#/proxies` in Edge at the current `2552 × 1228` desktop viewport, with both node groups fully expanded and current real-backend values.
- Density normalization: the source is a focused high-density crop while the implementation is a full-viewport Edge capture, so comparison used the node-card content region and preserved each capture's native density rather than scaling text independently.
- Full-view comparison: the card row remains compact, wraps through the existing flex container when space is insufficient, and does not create viewport overflow.
- Focused comparison: every visible card now shows the complete node name, protocol and bottom-right latency/test badge. `订阅 | 美国-IPV6-01` through `订阅 | 美国-IPV6-05` no longer truncate.
- Fonts and typography: the existing MiSans stack, 14 px node title and 12 px metadata are unchanged; only the usable title width was restored.
- Spacing and layout rhythm: node cards keep the source `152 px` minimum/flex basis, 8 px gap, 61 px height and existing padding/radius. The latency badge continues to reserve only the lower-right metadata row instead of reducing the title row.
- Colors and visual tokens: unchanged; selected, success, warning and untested states continue to use the existing Open-Box tokens.
- Image and icon fidelity: unchanged; no new assets or replacement icons were introduced.
- Copy and content: all backend-provided node names and protocols are now visible without ellipsis at the verified viewport.
- Interaction regression check: automatic/manual selection rules, node latency actions and both collapse levels remain unchanged.
- Findings after fix: no actionable P0/P1/P2 visual differences in the focused card-content comparison.
- Human Visual Approval remains pending and is not replaced by this implementation QA.

final result: passed

## 2026-09-28 Proxy node single-collapse and selected-latency correction

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-f8017265-57ec-4ceb-a59f-7f3e8e49ad8b.png`, showing the compact node-group state, selected-node hollow marker and measured top-right result.
- Implementation evidence: `http://127.0.0.1:1420/openbox.html#/proxies` in Edge at the current `2552 × 1228` desktop viewport, using the real `http://192.168.1.10:3036` backend.
- This entry supersedes the earlier inner `订阅` collapse behavior: `订阅` is now a static section heading, and only the upper card header collapses or expands the node group.
- The collapsed card shows one status dot per node; the current selection uses a transparent center with a latency-colored outline, while the other dots remain solid.
- The node-group and strategy top-right action resolves the current selection through nested proxy groups. It shows the selected terminal node's measured delay when available and keeps the lightning icon when that node is unmeasured.
- Edge interaction evidence: testing the selected `订阅 | 美国-04` node refreshed both node-group pills to `251`; Speed and strategies routed through the same selected node also showed `251`, while node cards without measurements retained their lightning actions.
- The top collapse was exercised and restored. The collapsed selected dot had `success selected`, a transparent computed background and a colored border; the subscription section was absent while collapsed and returned fully expanded afterward.
- Automated verification: focused proxy tests cover nested selection resolution, cyclic/unmeasured fallback, delay/lightning rendering, the single collapse target and selected-dot state; `pnpm lint`, `pnpm test` (17 files / 393 tests), `pnpm build` and `git diff --check` passed.
- Human Visual Approval remains pending and is not replaced by this implementation QA.

final result: passed

## 2026-09-28 Proxy collapse motion and view restoration

- Scope: the Open-Box React proxy view at `http://127.0.0.1:1420/openbox.html#/proxies`; no backend contract or node-selection behavior changed.
- Node-group content remains mounted during the `280 ms` collapse/expand transition. CSS Grid interpolates the content row between `1fr` and `0fr`, while margin and opacity transition with the same motion curve.
- The compact status-dot summary transitions in as the node cards collapse. The hidden node section uses `aria-hidden`, `inert`, `visibility: hidden` and zero grid height so its controls are not keyboard reachable while collapsed.
- `prefers-reduced-motion: reduce` disables these transitions without changing the resulting expanded or collapsed states.
- The selected proxy subview is stored in session storage. Edge switched from Strategy to Nodes, performed a full reload, and returned with `节点 (2)` still selected (`aria-checked=true`, `data-state=on`). Invalid or missing stored values fall back to Strategy.
- Edge computed-style evidence confirmed the provider panel has a `grid-template-rows 0.28s cubic-bezier(0.4, 0, 0.2, 1)` transition. The final expanded view retained the real-backend node cards and measurements.
- Automated verification: `pnpm lint`, `pnpm test` (17 files / 394 tests), `pnpm build` and `git diff --check` passed.
- Human Visual Approval remains pending and is not replaced by this implementation QA.

final result: passed
