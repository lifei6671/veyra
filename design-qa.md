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

---

# Rules Predicted Route, Busy Animation and Mode Persistence — Visual QA Addendum

## Evidence and method

- Defect and source captures: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-1b4b771b-5358-49d3-9a5e-8414e3c20d89.png`, `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-f5f03e29-064c-4682-9de6-dd56bd9f3d29.png`, and `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-559e2cbc-174e-4b56-9bc3-45fad676f1b0.png`.
- Live source truth: `http://192.168.1.10:3036/#/rules` in Edge. Captured `POST /api/openbox/penetration` for `telegram.org` and inspected `matched`, `chain`, `owner`, `resolved`, and `dns` fields.
- Implementation evidence: `http://localhost:1420/openbox.html#/rules` in the same Edge viewport and light theme.

## Comparison

- The predicted final exit removes the policy owner from the visible chain, renders `所有-自动 → 订阅 | 美国-04`, and labels the result `代理`; the direct Baidu response renders `直连` from the returned leaf instead.
- FakeIP responses render `返回 FakeIP 占位地址`, the original FakeIP explanation, and `解析经由 国外`. Real-address responses retain the returned address count and address explanation.
- During a real-route request, stages 2–5 show `等待查询...`, stage 1 keeps the selected target and kernel-origin description, and the header status includes a 0.8-second rotating indicator.
- The previous left prediction remains visible while the next prediction is pending, matching the original transition instead of replacing it with empty stages.
- Selecting `内核诊断` and then switching Baidu → Telegram → OpenAI keeps `内核诊断` active during and after each request; no terminal capability request or silent mode reset occurs.
- Edge confirmed the final Telegram predicted-route text, icon chain, FakeIP details, kernel-mode persistence and loading animation. The console reported no warnings or errors.
- Automated verification: focused Rules tests passed 14/14; `pnpm lint`, `pnpm build`, and scoped `git diff --check` passed.

## Findings

No actionable P0/P1/P2 findings remain in this predicted-route, busy-state and mode-persistence scope. Human Visual Approval remains a separate project gate.

final result: passed

---

# Rules OpenAI Cached Route Details — Visual QA Addendum

## Evidence and method

- Defect capture: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-3a0cb77e-d71d-4cb4-854e-da843a539532.png`.
- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-c59b40e4-ae42-4811-b128-9d06ae262d8e.png` and the live original `http://192.168.1.10:3036/#/rules` in Edge.
- The live original was rerun for `api.openai.com` in `内核诊断` mode. Network evidence captured `POST /api/openbox/route-test` with `{"target":"api.openai.com"}` and a response containing `dns.runtimeChain`, `dns.server.detour`, `resolve.cached`, `resolve.ttl`, `exit.chains`, `exit.error`, `exit.viaProxy`, and `exit.destinationIP`.
- Implementation evidence: `http://localhost:1420/openbox.html#/rules` in the same light-theme Edge viewport, with all five right-column stages expanded.

## Comparison

- The failed header pill, elapsed-time badge, and inline `访问失败` result now use the original red failure semantics instead of amber or muted text.
- Stage 5 removes the policy owner from the observed exit chain and renders the remaining `所有-自动 → 订阅 | 美国-04` path with the existing proxy icons and arrow icon.
- `exit.error` and the cached destination explanation render as separate amber `route-note` rows. Their measured 2 px left border, 2/8 px padding, 0/6/6/0 px radius, and 12/16 px text match the live original.
- Stage 2 maps `dns.runtimeChain` to the original `解析经由` icon chain, replaces the normal responder line with the `resolve.cached` note, includes the live TTL, and retains the A-address badge.
- The cached DNS row uses the same measured amber note geometry as the original. Non-cached successful diagnostics continue to show `<server> 应答`.
- The mapping is response-driven and is not special-cased to OpenAI or to the captured IP address.
- Edge confirmed all requested text, chains, colors, and backgrounds together and reported no console warnings or errors.
- Automated verification: focused Rules tests passed 13/13, `pnpm lint` passed, and `pnpm build` passed.

## Findings

No actionable P0/P1/P2 findings remain in the OpenAI cached-route detail scope.

final result: passed

---

# Rules Route Diagnostic Controls and Icons — Visual QA Addendum

## Evidence and method

- User defect capture: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-2ca65ceb-4390-4fac-9ad9-b7add16dd043.png`.
- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-a50a9588-26df-42e3-b945-fa472da352e4.png` and the live original at `http://192.168.1.10:3036/#/rules`.
- Implementation capture: `http://localhost:1420/openbox.html#/rules` in the user's Edge browser at the existing 2552 × 1223 CSS-pixel viewport.
- State: light theme, `www.baidu.com`, kernel diagnostic selected, successful HTTP 200 result.

## Comparison

- The right-header mode controls now use the original 24px joined two-button treatment. `内核诊断` carries the active gray surface while `模拟终端` remains transparent.
- The retry action is the original 80px text button with both the refresh icon and `重新测试`; it is no longer an icon-only square.
- The observed route renders the shared Direct target icon before `直连` and the shared China shield before `国内`. Both are sourced through the global `proxyIcon` mapping used by proxy cards.
- The final outbound is derived from the last route-chain member, while policy ownership remains the first/declared owner. The verified result therefore shows `直连` at stage 5 and `国内` at stage 4, matching the live source.
- The predicted stage-4 content order now matches the original: `站点集`, China shield/name, `域名后缀`, `baidu.com`, then `geosite-cn`.
- Edge returned a live successful diagnostic with 51ms, HTTP 200, Direct final outbound, China ownership, direct DNS and two resolved addresses. No fabricated success state was introduced for partial responses.

## Findings

No actionable P0/P1/P2 findings remain in the requested right-header and route-icon scope.

final result: passed

---

# Rules Quick Route Lookup and Diagnostics — Visual QA Addendum

## Evidence and method

- User reference: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-dd8d9a76-7aaa-4ae4-89d4-2ba5b5024b4a.png`.
- Authoritative live reference: `http://192.168.1.10:3036/#/rules` in Edge, checked in terminal-unavailable and kernel-diagnostic states for `www.baidu.com`.
- Implementation: `http://localhost:1420/openbox.html#/rules` in the same Edge viewport against the authenticated real backend.
- Original request contract observed before implementation: `POST /api/openbox/penetration`, `GET /api/openbox/terminal-test/capability`, conditional `POST /api/openbox/terminal-test`, and `POST /api/openbox/route-test` for kernel diagnostics.

## Comparison

- Baidu, Google, OpenAI and Telegram remain right-aligned icon buttons with the original hover/active transition and a descriptive tooltip. Clicking an icon fills the target field and starts the real lookup flow.
- The result surface follows the original two-column structure: predicted rule route on the left and observed route on the right, each with five stages from request origin through DNS, ingress, rule/ownership and final exit.
- The incapable terminal response `{ ok: false, missing: ["lan", "dhcp"] }` renders `无法模拟`, the four unavailable stages, the explanatory first stage and `改用内核诊断`; it does not fabricate a terminal result.
- Kernel mode is requested only after selecting `内核诊断`. The verified Baidu response rendered `50 ms`, `HTTP 200`, `国内`, `直连 DNS`, two resolved addresses and `面板回环入站` from the returned route-test payload.
- GET/HEAD/TCP/TLS selection, terminal/kernel switching, detail disclosure and retry remain interactive. Async request identities prevent an older response from replacing a newer icon or mode selection.
- The sticky 48px toolbar remains outside the scrolling results. The diagnostic surface uses the existing theme, surface, text, accent and warning tokens in light and dark modes.
- No temporary response logging or raw backend routing data remains in the implementation.

## Verification

- `pnpm test`: 19 files and 431 tests passed.
- `pnpm build`: passed.
- Edge runtime: terminal-unavailable and kernel-success interactions passed with real backend data.
- Human Visual Approval: pending user confirmation.

## Findings

No actionable P0/P1/P2 implementation finding remains in the requested quick-route interaction scope. Delivery remains open until Human Visual Approval.

final result: implementation passed; human visual approval pending

---

# Rules Shared Proxy Cards, Latency Action, and Transition — Visual QA Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-1a432760-e8cd-4e4b-a955-d3128ea59ec4.png`.
- Authoritative live reference: `http://192.168.1.10:3036/#/rules` in Edge with Speed and its policy penetration expanded.
- Implementation capture: `http://localhost:1420/openbox.html#/rules` in the user's Edge browser at the existing 2552 × 1223 CSS-pixel viewport.
- Regression surface: `http://localhost:1420/openbox.html#/proxies`, checked after extracting the shared card implementation.
- States checked: collapsed and expanded Speed rule, collapsed and expanded policy penetration, untested and measured option latency, active node test, and proxy-page policy/node cards.

## Comparison

- Rules and Proxies now render the same exported `ProxyGroupCard`, `ProxyOption`, `PolicyLatency`, and `PenetrationGroup` components. Rules only supplies an embedded outer-shell mode; option cards, latency actions, node grouping, testing state and selection behavior are shared.
- The option latency action uses the original card geometry: absolute bottom-right placement (`right: 8px`, `bottom: 7px`), a 20px-high rounded badge, and the same success/warning/danger state colors as the proxy page.
- Clicking an unmeasured child latency action immediately replaces the bolt with the animated three-dot testing indicator and sets the button busy state; completion refreshes the shared proxy data and replaces the indicator with the measured latency.
- Expanding and collapsing a strategy-backed rule preserves the grid-row/opacity transition instead of mounting or removing the detail abruptly. The shared policy penetration control opens the nested selector header, provider heading, child node cards and each child latency action.
- The proxy-page regression capture still shows both URLTest and Selector groups with their existing selection, test actions, bottom-right latency badges, grouping and layout.
- Non-strategy rules remain non-expandable and no duplicate rule-only proxy-card style or interaction implementation remains.

## Findings

No actionable P0/P1/P2 findings remain in the shared proxy-card, latency-test, penetration, and transition scope.

final result: passed

# Logs Type Filter Long Value — Initial Attempt (Superseded)

## Evidence and method

- Reported defect capture: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-cb376b12-513a-49e6-9748-72261b12ebeb.png` (1000 × 107 pixels).
- Authoritative live reference: `http://192.168.1.10:3036/#/logs` in Edge with `outbound/vless[订阅 | 美国-04]:` selected.
- Implementation capture: `http://localhost:1420/openbox.html#/logs` in Edge with the longest available outbound category selected, at the existing 2552 × 1223 CSS-pixel viewport and 1.5 device pixel ratio.
- State: light theme; closed trigger checked first, then the picker was expanded to confirm option layout was unchanged.

## Comparison

- The defect capture shows the selected value wrapping to a second line and crossing the fixed 112 × 32 trigger boundary.
- Computed-style comparison isolated the missing reference rules: the original closed select uses `white-space: nowrap` and `text-overflow: ellipsis`; the implementation previously used `white-space: normal` and `text-overflow: clip`.
- The implementation now matches the original closed-trigger rules while preserving `overflow: visible`, its 112 × 32 geometry, centered alignment, padding, font, surface and chevron.
- The expanded option remains `display: flex; white-space: normal`, so the picker can present the full value without changing its grouping or interaction behavior.
- Post-fix Edge capture keeps the selected value on one line with ellipsis and no overlap with the search control. The picker still opens, the selected check mark remains visible, and console error output is empty.
- No image assets, colors, copy, grouping, filter values or data behavior changed.

## Comparison history

1. The grouped native select allowed the selected option's normal wrapping to leak into the closed trigger, producing the reported two-line overlap.
2. Added only the original trigger's `text-overflow: ellipsis` and `white-space: nowrap` rules.
3. A later user capture showed that Edge still painted the closed selected value beyond the native control despite these declarations. This attempt was therefore superseded by the final clipped-label implementation recorded below.

## Findings

The later overlap report remained actionable; this intermediate result is superseded.

result: superseded

---

# Logs Type Filter — Visual Parity Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-78d30be6-da08-4adf-a4b5-528ee5fb13a0.png` (574 × 379 pixels).
- Authoritative live reference: `http://192.168.1.10:3036/#/logs` in Edge with the second filter expanded.
- Implementation capture: `http://localhost:1420/openbox.html#/logs` in Edge at a 2552 × 1223 CSS-pixel viewport and 1.5 device pixel ratio, with the second filter expanded.
- State: light theme, `全部` selected, live `info` logs containing `dns:`, `inbound/tun[tun-in]:` and one runtime-specific outbound category. Dark theme was expanded and inspected separately, then the user setting was restored to light.

## Comparison

- The second trigger matches the reference at 112 × 32 CSS pixels with 12px left padding, 28px right padding, 9px radius and 14/20 text metrics.
- The expanded control now preserves the reference information hierarchy: `全部`, the `日志等级` group and the `日志类型` group, with level and category values indented beneath their labels.
- Picker geometry matches the measured reference rules: 8px outer margin and inner padding, -8px horizontal translation, 13px radius, 28px option rows, 7px group spacing and a maximum height of 384px / 70dvh.
- Picker and trigger surfaces use the same 75% theme surface as the reference; light and dark theme tokens were both checked in the real browser.
- Font and copy match the reference (`MiSans`, 14px/20px; `全部`, `日志等级`, `日志类型`). The chevron uses the project's existing Heroicons asset rather than a newly drawn icon.
- Runtime data may add categories that are not present in the static reference screenshot; this changes menu content only, not the approved grouping, sizing or styling.
- Selecting `dns:` through the real native select changed the value to `type:dns:` and all inspected visible rows contained `dns:`; returning to `全部` restored the full list.

## Comparison history

1. The previous implementation rendered a flat Radix option list, so log levels and log categories had no headings or indentation.
2. Only the second filter was replaced with the native grouped select used by the original UI; the first level selector and all shared selectors remain unchanged.
3. Post-fix expanded light and dark captures showed no remaining P0/P1/P2 mismatch in the requested filter region.

## Findings

No actionable P0/P1/P2 findings remain in the requested second-filter region.

final result: passed

---

# Logs Toolbar — Visual Parity Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-86fa7d74-0807-4db6-80fa-8847f7e4cdab.png`.
- Authoritative live reference: `http://192.168.1.10:3036/#/logs` in Edge.
- Implementation capture: `http://localhost:1420/openbox.html#/logs` in Edge, clipped to the 2296 × 64 CSS-pixel toolbar region.
- Browser viewport: 2552 × 1223 CSS pixels at the current desktop scale.
- Source pixels: 3529 × 110. The source is a narrow physical-pixel crop, so comparison used the matching focused toolbar region rather than treating the different pixel densities as a geometry defect.
- State: expanded sidebar, light theme, query empty, logs running; dark theme was checked separately and then restored to light.

## Comparison

- Full toolbar composition matches the live reference: 48px sticky bar, 8px inset, 32px controls, 8px gaps, 256px search field and right-aligned actions.
- Focused transparent-surface comparison matches the live reference's computed values: 75% base surface with `backdrop-filter: blur(7px)`.
- Focused icon-button comparison matches the live reference's computed values: `oklch(93% 0 0)` rest background, 9px radius and 200ms color/background/border/shadow transition.
- Hover was exercised through real pointer movement. The settled background is `oklab(0.8649 0 0)`, matching the reference's 7% black `color-mix` rule.
- Disabled hover was exercised with a no-match query that disables download. Its background remained `oklch(0.93 0 0)` after real pointer movement, so disabled controls do not gain hover emphasis.
- Dark mode preserves the same transparent/blur treatment and uses the reference dark button surface `oklch(23.26% .014 253.1)`.
- Edge console error check returned no errors after the light, hover and dark-state interactions.
- Fonts, copy, icon assets, control labels and layout rhythm are unchanged from the previously verified implementation.

## Comparison history

1. Earlier implementation used an opaque toolbar and visually weak icon-button surfaces.
2. The toolbar was changed to the measured transparent blurred surface; icon rest and hover colors and transitions were changed to the measured reference values.
3. Post-fix light, hover and dark captures showed no remaining P0/P1/P2 mismatch in the requested toolbar region.
4. Independent review found disabled controls inherited the hover rule; the selectors were narrowed with `:not(:disabled)` and the repaired state was rechecked in Edge.

## Findings

No actionable P0/P1/P2 findings remain in the requested toolbar region.

final result: passed

---

# Logs Type Filter Label Alignment — Visual Parity Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-53f982e3-213b-44c9-b4d6-a49a37175587.png` (922 × 118 pixels).
- Authoritative live reference: `http://192.168.1.10:3036/#/logs` in Edge with the second filter closed.
- Implementation capture: `http://localhost:1420/openbox.html#/logs` in Edge, focused on the closed toolbar controls at the existing 2552 × 1223 CSS-pixel viewport and 1.5 device pixel ratio.
- State: light theme, second filter closed, `全部` selected; live log reception was paused only for measurement and restored afterward.

## Comparison

- The source and implementation retain the same 112 × 32 CSS-pixel trigger, 12px/28px horizontal padding, 14px/20px MiSans text, surface color, border, radius and chevron asset.
- Computed-style comparison isolated the drift: the original native select resolves to `display: flex; align-items: center`, while the implementation resolved to `display: flex; align-items: normal`.
- The implementation now explicitly sets `align-items: center`; the post-fix Edge capture shows `全部` vertically aligned with the first selector and search placeholder.
- The grouped picker contents and filtering behavior are unchanged. No image asset, color token, spacing rhythm or copy change was needed.
- Edge console error check returned no errors after hot reload and the live-log state was restored.

## Comparison history

1. The previous grouped native select matched the open picker but omitted the original control's closed-state cross-axis alignment.
2. Added the missing `align-items: center` declaration only to `.log-category-select select`.
3. Post-fix computed styles and focused Edge capture show no remaining P0/P1/P2 mismatch in the requested closed-label region.

## Findings

No actionable P0/P1/P2 findings remain in the requested closed-label region.

final result: passed

---

# Logs Type Filter Long Value — Visual Parity Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-aa927eb1-535b-4908-ab93-864a889f90ea.png`.
- Authoritative behavior: the original grouped native picker at `http://192.168.1.10:3036/#/logs` and the supplied closed-state defect capture.
- Implementation capture: `http://localhost:1420/openbox.html#/logs` in Edge with `outbound/vless[订阅 | 美国-04]:` selected.
- State: light theme; the long category was checked both closed and expanded.

## Comparison

- The second control remains 112 × 32 CSS pixels with its existing border, radius, background, typography and chevron.
- The closed-state label now has its own clipped content box between the 12px left inset and the chevron. It stays on one line and renders an ellipsis instead of painting into the following search field.
- The native select remains the interactive and accessible control, retaining the full selected value and the original grouped popup. The visual label is pointer-transparent and hidden from accessibility APIs.
- The post-fix closed capture shows the label ending inside the control, the chevron visible, and an 8px gap before the search field.
- The post-fix expanded capture preserves the complete `全部`, `日志等级` and `日志类型` groups and the selected long option. Edge reported no console errors.
- No image asset, color token, copy, filter behavior or toolbar geometry changed.

## Comparison history

1. Applying `white-space: nowrap` and `text-overflow: ellipsis` directly to the customizable native select was insufficient because Edge continued to paint the selected value with visible overflow.
2. Clipping only the wrapper prevented overlap but also clipped the native closed-state arrow/text presentation.
3. The final implementation keeps the native select for interaction and popup rendering while using a dedicated clipped visual label for the closed state.

## Findings

No actionable P0/P1/P2 findings remain in the requested long-value state.

final result: passed

---

# Logs Filter Width and Action Tooltips — Visual QA Addendum

## Evidence and method

- Source visual context: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-86fa7d74-0807-4db6-80fa-8847f7e4cdab.png` plus the user's explicit request that the second filter be at least as wide as the search input and that the three right-side actions expose tooltips.
- Browser-rendered implementation capture: `http://localhost:1420/openbox.html#/logs` in the user's Edge browser at the existing 2552 × 1223 CSS-pixel viewport.
- State: light theme, long outbound category selected, filter closed; tooltip states were checked with real pointer interaction and keyboard focus.

## Comparison

- Layout: the second filter and search field both measure 256 CSS pixels, with the existing 8px gap preserved. The long category remains contained and the chevron remains visible.
- Interaction: download, pause/resume and clear expose interface tooltips on hover and `:focus-visible`; their accessible names remain `下载日志`, `暂停接收`/`继续接收` and `清空当前日志`.
- Typography: existing MiSans control text is unchanged; tooltip text uses the existing compact 12px/18px UI scale.
- Colors: tooltip foreground/background derive from the existing theme tokens. The second filter trigger and expanded picker both use the opaque `var(--surface-solid)` token (`rgb(255, 255, 255)` in the verified light state), while the toolbar's separate transparent transition remains unchanged.
- Assets: all three actions continue to use the existing Heroicons; no image or icon assets changed.
- Copy: tooltip text matches each action's accessible label, including the pause/resume state change.
- Edge console error output remained empty after the width update and tooltip interactions.

## Comparison history

1. The second filter was previously 112px wide, so long values relied heavily on truncation even after the overflow defect was fixed.
2. The three action buttons previously exposed only native browser `title` hints, which were not a reliable visible interface tooltip.
3. The filter was widened to 256px and the three actions received scoped, state-aware hover and keyboard-focus tooltips without changing their behavior.
4. The filter trigger and picker initially retained a 75% transparent surface; both were changed to the opaque theme surface and the expanded Edge capture confirms that log rows no longer show through the menu.

## Findings

No actionable P0/P1/P2 findings remain in the requested width and tooltip states.

final result: passed

---

# Rules Toolbar and Expandable Categories — Visual QA Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-f40a979f-677f-411b-8df9-4a782a7a6d1a.png` (3796 × 1430 pixels).
- Authoritative live reference: `http://192.168.1.10:3036/#/rules` in Edge, including the collapsed and expanded Speed rule states.
- Implementation capture: `http://localhost:1420/openbox.html#/rules` in the user's Edge browser at the existing 2552 × 1223 CSS-pixel viewport.
- States checked: light and dark themes, full list, scrolled list, search-filtered list, expandable Speed rule, non-expandable reject/direct rules, URL formatting and all four site shortcuts.

## Comparison

- The rules toolbar now matches the live reference structure: a fixed 48px translucent/blurred strip with a 320 × 32px search field, 32px formatter and right-aligned Baidu, Google, OpenAI and Telegram buttons. The previous page title, count, segmented type filter and refresh action are removed.
- The search placeholder and quick-filter values match the original. URL formatting reduces `https://www.google.com/path?q=1` to `www.google.com`, while host-and-port input remains unchanged.
- Rule cards keep the original 92px collapsed rhythm, rounded translucent surfaces, green payload text and route-chain icon treatment.
- Expandability is data-driven: a rule is interactive only when its `route(...)` target resolves to a proxy record with candidate members. Reject, sniff, DNS hijack and direct actions remain non-interactive; Speed, AI, Youtube and the other selector-backed routes expose an accessible expanded state.
- Expanded cards show the original group icon, group/type/member summary, current route chain, latency where applicable and the four proxy-category cards. Direct and reject leaves do not show misleading latency values.
- Scrolling is confined to the rule list, so the toolbar remains fixed. Light and dark captures preserve contrast, background translucency and selected-option emphasis without introducing a new color system.
- The browser reported no new console error after the final reload; the only retained entry was the timestamped Vite hot-reload message created while the source file was momentarily replaced during editing.

## Findings

No actionable P0/P1/P2 findings remain in the requested toolbar and rule-category expansion scope.

final result: passed

---

# Rules Strategy Card Expansion Height — Visual QA Addendum

## Evidence and method

- User-reported failing state: strategy-backed rows received focus but appeared not to open.
- Reference state: `http://192.168.1.10:3036/#/rules` in Edge with the Speed strategy card visibly expanded.
- Implementation state: `http://localhost:1420/openbox.html#/rules` in the user's Edge browser at the existing 2552 × 1223 CSS-pixel viewport.
- Pointer checks: AI and Youtube strategy rows were clicked at visible card coordinates; accessibility state and rendered geometry were checked after each click.

## Comparison

- The click handler was firing before this repair, but the grid kept every implicit row at the collapsed 92px height. The expanded article had a 92px client height and a 249px scroll height, so `overflow: hidden` clipped all strategy details.
- The rule list now sizes implicit tracks to their max-content height. Collapsed rows remain 92px; the expanded AI/Youtube row grows to 254px and displays its strategy heading, route chain and four proxy-category cards inside the rounded surface.
- Opening Youtube collapses AI, matching the existing single-expanded-row behavior. Non-strategy reject/direct rows remain non-interactive.
- Edge reported no console warnings or errors after the pointer interactions.

## Findings

No actionable P0/P1/P2 findings remain in strategy-card pointer expansion.

final result: passed

---

# Shared Proxy Card — Per-Node Test Lock Addendum

## Evidence and method

- User requirement: clicking a child card's bottom-right latency action must not disable the surrounding card region; only that child card may be locked while its test runs.
- Implementation state: `http://localhost:1420/openbox.html#/rules` in Edge with the Speed rule expanded.
- Shared surfaces checked: Rules, Proxies, and the connection-detail policy card all consume the same `ProxyGroupCard` state contract.
- Runtime probe: clicked `测试 直连 延迟`, then inspected every sibling `.policy-option` before the request completed.

## Comparison

- The active Direct child card alone reported its `disabled` and testing classes; both its selection action and latency action were disabled during the request.
- The sibling URLTest, Selector, and Reject cards remained enabled, without testing state, and retained their selection and latency actions.
- Completion removed the three-dot testing state and refreshed the measured latency. Edge reported no console warnings or errors for the final interaction.
- Individual node tests are tracked by node identity in a set, so concurrent tests on different child cards do not overwrite one another's pending state. Group-wide tests and other group-level mutations keep their existing broader busy behavior.
- Each completion merges only that node's returned latency into the latest in-memory proxy state, preserving any newer sibling selection or concurrent node result instead of committing an older full snapshot.
- Group and subscription tests also merge their returned latencies in place. A broader refresh preserves any node history that completed after that refresh began, so keeping sibling actions enabled cannot reintroduce stale latency data.
- Group-test completion appends the same result set to the tested group's ten-entry history source, keeping the top-right latency tooltip synchronized without reloading the full proxy snapshot.

## Findings

No actionable P0/P1/P2 findings remain in the per-node interaction-lock scope.

final result: passed

---

# Rules Route Detail Field Mapping — Visual QA Addendum

## Evidence and method

- Defect capture: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-8da7f001-4e4d-420b-8df6-d3a18e80ef77.png`.
- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-d45973e7-75bd-4850-bfc2-69d90ed11035.png` and the live original `http://192.168.1.10:3036/#/rules` in Edge.
- Implementation capture: `http://localhost:1420/openbox.html#/rules` in Edge at the existing 2552 × 1223 CSS-pixel viewport.
- State: light theme, `www.baidu.com`, simulated terminal unavailable, predicted stages 4 and 3 expanded, actual stage 1 expanded.

## API evidence

- The live original sends `POST /api/openbox/penetration` with `{"target":"www.baidu.com"}`; the local implementation already uses the same endpoint and body.
- The original response provides `matched.rule.rule_set=["geosite-cn"]`, `firstLayer.nativeBypass.sets=["geoip-cn","geoip-private"]`, and `firstLayer.entryMode.reason`. The previous UI discarded most of these fields and serialized the rule object as JSON.
- The local terminal capability response exposes internal missing identifiers `lan` and `dhcp`; the original presents them as `找不到 LAN 网桥或它的 IPv4 地址` and `没有 udhcpc`.

## Comparison

- Rule details now render `rule_set=geosite-cn` in the original compact monospace treatment and exclude the unrelated outbound field.
- Entry details now render the original three separate lines derived from `nativeBypass.sets` and `entryMode.reason`, with 6px vertical spacing.
- The predicted final-outbound stage no longer exposes a non-original chain-details control.
- The unavailable terminal stage no longer repeats `www.baidu.com` as inline metadata. Its expanded content contains the mapped missing-capability warning and the original explanation that no automatic kernel-loopback fallback occurs.
- The first unavailable detail uses the original 2px amber left border, pale amber background, 2px/8px padding and 12px/16px text scale.
- Edge shows the requested expanded text in both panels and reports no console warnings or errors.

## Findings

No actionable P0/P1/P2 findings remain in the requested expanded-detail state.

final result: passed

---

# Rules DNS Detail Completeness — Visual QA Addendum

## Evidence and method

- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-e01d41a5-2be9-4b75-b4c6-9edae84b7324.png` and the live original `http://192.168.1.10:3036/#/rules` in Edge.
- Implementation evidence: `http://localhost:1420/openbox.html#/rules` in Edge at the existing 2552 × 1223 CSS-pixel viewport.
- State: light theme, `www.baidu.com`, predicted stages 4, 3 and 2 expanded; the terminal-unavailable detail remained independently expanded.

## API and field mapping

- The existing request remains `POST /api/openbox/penetration` with `{"target":"www.baidu.com"}`; no endpoint or request-body fallback was introduced.
- The DNS detail now projects `dns.server.tag` as the resolver name and `resolved.addresses` as the inferred kernel-DNS result.
- The two explanatory lines retain the original wording that domain visits do not use arbitrary IP conditions and that terminal queries entering the kernel are controlled by the entry policy above.

## Comparison

- Stage 2 now expands to the same three information rows as the original: `解析器 dns-direct`, the resolved-address explanation, and the kernel-DNS/entry-policy explanation.
- Stage 3 still exposes the original three entry-policy rows shown in the upper red box; the DNS addition does not replace or collapse them.
- Each detail row uses the existing original-aligned 12 px / 16 px text treatment and 6 px vertical rhythm.
- Edge confirms both red-box regions are visible together and reports no console warnings or errors.
- Automated verification: focused Rules tests passed 11/11, `pnpm lint` passed, and `pnpm build` passed.

## Findings

No actionable P0/P1/P2 findings remain in this DNS-detail completeness scope.

final result: passed

---

# Rules Diagnostic Action Styling — Visual QA Addendum

## Evidence and method

- Defect capture: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-9408e686-d7ba-4bd9-b66e-ba1c814f371c.png`.
- Source visual truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-7699b3cb-e4b8-44c9-b816-7ca52cb4bd65.png` plus computed styles from the live original `http://192.168.1.10:3036/#/rules` in Edge.
- Implementation evidence: `http://localhost:1420/openbox.html#/rules` at the same existing Edge viewport and light theme.

## Comparison

- The probe-method control now uses the original native select structure and double-triangle indicator instead of the generic custom-select trigger.
- Its measured box matches the original at 62.67 × 24 px, with 12/28 px inline padding, 8.7 px radius, 20% text-color border and 11 px regular text.
- The mode buttons retain the original 60 × 24 px geometry, 11 px semibold label, joined 1 px overlap and asymmetric joined radii.
- The active `模拟终端` state now uses the original neutral-gray emphasis instead of the nearly transparent field background.
- The retry button and 14 px refresh icon retain the original 80 × 24 px geometry and 6 px icon/text gap.
- Edge exercised `HEAD` and restored `GET`; the value changed both times, the route test completed, and no console warnings or errors were reported.
- Keyboard focus uses the existing accent border and 2 px accent-soft focus ring, replacing the native-select outline reset with a visible `:focus-visible` state.
- Dark-theme Edge evidence retained the same 62.67 × 24 px select and 60 × 24 px active mode button. The select used dark surface/text tokens for its surface, border and arrow, while the active button used the existing dark `--field` token (`rgb(51, 58, 58)`). `HEAD` → `GET` interaction and the 80 × 24 px retry button with its 14 px icon were rechecked in that theme.
- The user's original light theme was restored after the dark-theme verification.

## Findings

No actionable P0/P1/P2 findings remain in the requested diagnostic-action styling scope.

final result: passed

---

# Rules Actual Route Expanded Details — Visual QA Addendum

## Evidence and method

- Defect capture and source truth: `C:/Users/lifei/AppData/Local/Temp/codex-clipboard-870dc7d5-f524-4a2b-a52d-5b6cc520f2e6.png` plus the live original `http://192.168.1.10:3036/#/rules` in Edge.
- The original was run in `内核诊断` mode for `www.baidu.com`; all five right-column detail toggles were expanded and inspected.
- Network evidence: the original sends `POST /api/openbox/route-test` with `{"target":"www.baidu.com"}`. The 200 response supplied `exit.connectTo`, `exit.rule`, `exit.url`, `dns.server`, and `resolve.answers`; the implementation maps those fields without hard-coding the observed IP addresses.
- Implementation evidence: `http://localhost:1420/openbox.html#/rules` in the same light-theme Edge viewport, with all five right-column stages expanded.

## Comparison

- Stage 5 renders the observed target address as `目标 <connectTo>`.
- Stage 4 renders the matched connection rule and the original explanation that a panel-originated connection cannot fill missing terminal-side conditions.
- Stage 3 renders both original kernel-ingress explanations, including the domain-only matching behavior and the exclusion of IP/GeoIP conditions.
- Stage 2 renders the IPv6-disabled note, resolver tag, responding server, and each IPv4 answer as a separate original-style badge.
- Stage 1 renders both original caveats about the panel loopback source and the absence of LAN-entry visibility.
- DNS answer badges match the original 20 px height, 9 px horizontal padding, 8.7 px radius, 12/16 px monospace text and 4 px row gap.
- The IPv6 note is tied to the existing `config/ipv6-test` setting and is omitted when that profile switch is enabled.
- Edge showed all requested detail content together and reported no console warnings or errors.
- Automated verification: focused Rules tests passed 12/12, `pnpm lint`, `pnpm build`, and `git diff --check` passed.

## Findings

No actionable P0/P1/P2 findings remain in the actual-route expanded-detail scope.

final result: passed


---

# OpenBox Terminal Routing — Visual and API QA

- Reference: `https://openbox.disign.me/#/settings`, terminal-routing menu; public bundle `assets/index-R7_omgFM.js` components `ClientRoutingPage` and `ClientRouteEditDialog`, plus the live light-theme dialog.
- Mapping: `ClientRoutingSettings.tsx` replaces the former terminal JSON editor. It reuses the common Modal, SortableList, portal Tooltip and shadcn/Radix outbound picker; the known-client picker also uses shadcn/Radix Select.
- API: load `/api/openbox/profile` and `/api/openbox/groups`, then `/api/openbox/clients`. Save with `PUT /api/openbox/profile` containing only `{clientRoutes}` and render the returned Profile. The terminal-config/device APIs are no longer used by this menu.
- Source behavior reproduced: IP/MAC matching, known-client search and repeated append without closing, selected checks, route/bypass/admit modes, inactive-input clearing on save, source/MAC normalization and validation, enabled state, missing-outbound warning, unique active allowlist count, delete confirmation and persisted sorting.
- Visual measurements: editor width 576 px, 16 px body padding, 32 px inputs, 86.56 px textarea, 20 px radios, 52 × 32 px footer buttons with 14 px semibold labels. Primary and warning colors trace to the original dialog tokens. Light and dark dialogs were inspected with the actual bundled MiSans font.
- Real backend verification was read-only: the local menu rendered the returned terminal rule and known clients, opened the editor, appended a known client, and exercised IP/MAC and outbound controls. The real page reported no console warnings/errors. No production terminal rules were written during verification.
- Save, validation errors, failure draft retention, disable/re-enable, delete cancel/confirm, allowlist counting and dragging were exercised in an isolated browser fixture using the real UI with an in-memory API. Captured save payloads contained only `clientRoutes`. Temporary fixture files and browser tabs were removed after verification. Fixture-only hot reloads emitted duplicate-createRoot warnings; these were absent from the real application tab.
- At 600 × 500, the editor measured 568 × 350 px with the footer visible; its body scrolled independently. Known-client and outbound popovers remained inside the viewport. Escape dismissed the picker while preserving the editor.
- Evidence directory: `/Users/lifeilin/.codex/visualizations/2026/10/02/01a0fb2a-a6f0-7c41-adbf-936f28103039/terminal-routing/`. Final captures: `local-editor-final.png`, `local-page-final.png`, `cards-final.png`, `dark-editor-final.png`; source capture: `source-editor.png`.
- Automated verification: `pnpm test src/openbox` passed 123 tests across 13 files; `pnpm lint`, `pnpm build`, and `git diff --check` passed. Build retains the existing large-chunk advisory.
- Intentional failure behavior improvement: saving failures preserve the dialog draft rather than closing it before the request finishes.
