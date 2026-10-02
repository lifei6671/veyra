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

## 2026-10-02 面板设置对齐

- 当前原版：`https://openbox.disign.me/#/settings`；本地迁移页：`http://127.0.0.1:1420/openbox.html#/settings`，通过 Vite 代理连接同一后端。
- 在内置浏览器对照 1280 × 900 浅色/深色和 999 × 850 窄窗口。修正固定列宽、行顺序、字段尺寸、开关、背景上传/调整按钮、滑块、站点清空按钮与弹窗。两列断点为 1024 px，列间距按 1024/1280/1536 px 对应 32/48/64 px。
- 1280 px 展开状态：通用/延迟/测试站点面板分别位于 y=56/308/604；图标弹窗 x=280、y=339.5、宽 256、高 320；密码弹窗宽 512、高 366、y=267，与原版实测一致。
- 完整保留图标菜单 896 项代码、原图资源、分类和搜索，菜单开启保持根容器 scrollTop=0。
- 原版默认值采用圆角 16、黄色阈值 400、红色阈值 800、双列；有后端持久化数据时渲染后端值。文字、数值和滑块输入约 400 ms 后自动保存。
- API 保持 `GET/PATCH /api/storage`（PATCH 请求体 `{entries, removed: []}`），`GET/PUT/DELETE /api/background-image`（PUT 请求体 `{image}`），`POST /api/auth/change-password`（请求体 `{currentPassword,newPassword}`）。支持原版 `text/plain` JSON 响应。背景引用离开 `local-image` 时按原版清理后台图片。
- 真实操作证据：本地圆角 15→16 后原版回读 16，再恢复并回读 15；亮色→暗色后原版回读暗色，再恢复并回读亮色；测速超时未失焦保存 5001，原版回读 5001，再恢复并回读 5000；当前百度图标选择保存成功；图标分类/搜索、背景展开、密码弹窗关闭和恢复默认取消已操作。
- 没有提交真实密码、上传/删除现有背景图，或执行恢复默认；这些写入 API 的方法和请求体由定向测试校验。
- 验证：`pnpm lint`、3 个测试文件 / 13 项定向测试、`pnpm build`、`git diff --check`。构建保留已有的大 chunk 提示；完整原版图标目录本地打包约 1.9 MB。
- 一次测试命令误触发全量：22 文件 / 454 项中 452 通过、2 项 `scripts/sdlc-ui-contract.test.mjs` 门禁测试失败。本次按用户要求不调整门禁；随后仅运行本次相关测试并通过。
- 本次证据位于 `/Users/lifeilin/.codex/visualizations/2026/10/02/01a0fb2a-a6f0-7c41-adbf-936f28103039/panel-settings/`。范围为桌面面板设置；没有重新验收其他页面、移动端手势或整个应用。

## 2026-10-02 Tooltip 与 Select 修正

- IP信息API 提示逐字对照原版：此API会用于IP检查中全球节点IP信息查询、连接详情中的IP地理信息查询、面板DNS查询中的IP地理信息查询。长提示允许换行，并提升所在卡片层级以避免被下一张卡片遮住。
- OpenBox 迁移入口 `src/openbox/**` 的原生 Select 均改为本地 shadcn/ui Radix Select 组合（`ui/select.tsx`），沿用既有 Radix 依赖与 OpenBox 样式；已有 SelectControl 也使用同一组件。原生 JSX select/option/optgroup 已无残留。旧 Veyra 独立入口不属于本次迁移范围。
- 保留 API、存储键、动态选项与回调值；空字符串“全部”只在组件内部编码，回调还原空字符串。更新小时继续转换为数字，日志保留等级/类型分组。
- 内置浏览器真实操作：IP 三个选项、代理排序菜单、实时日志分类菜单；订阅更新周期选择“每 3 天”、时间“06:00”、地区菜单和分享 https 协议；分组规则切换 failover、手动页签模式、空值筛选、自动分组添加德国后选择器复位。订阅与分组弹窗均取消，未保存测试编辑。Esc 关闭菜单后弹窗保持打开。
- 浅色与深色菜单已截图核验，修正深色触发器背景覆盖；测试后恢复后端原有亮色主题，IP 服务保留 ipwho.is。
- 验证：`pnpm lint`、`pnpm test src/openbox`（11 文件 / 98 项）、`pnpm build`、`git diff --check`。
- 截图：`/Users/lifeilin/.codex/visualizations/2026/10/02/01a0fb2a-a6f0-7c41-adbf-936f28103039/select-correction/`。

## 2026-10-02 设置菜单固定与测试站点提示

- 设置分类菜单从页面滚动内容中分离，保留 48 px 高的顶部栏；仅 `.settings-content` 纵向滚动。菜单横向溢出仍可滚动，初始卡片 y=56 保持不变。
- 原版测试站点问号实测全文：概览里的延时小卡片和规则页右上角的快捷查询共用这四个站点。图标从图标库选;名称空着就用图标的品牌名(没有品牌名就用网址的主机名);网址填 http(s) 地址,延时按内核经当前分流访问这个地址计
- 在内置浏览器 999 × 850 视口中，测试站点滚动至可见时内容 scrollTop=182，菜单 top=0；长提示完整显示于问号上方。未修改 API 或持久化数据。
- 验证：`pnpm lint`、`pnpm test src/openbox/pages/settings/PanelSettings.test.ts src/openbox/pages/SettingsPage.test.ts`（4 项）、`pnpm build`、`git diff --check`。

## 2026-10-02 IP 提示浮层裁切修复

- 根因：居中的 CSS 伪元素越过 `.settings-content` 左边界（overflow-x:hidden），同时处于 workspace 堆叠上下文内；提高卡片 z-index 不能越过裁切边界。
- IP 说明改为 React Portal，挂载到 body，以 fixed 位置和 z-index:1200 显示。保持原文、尺寸和颜色；支持焦点、Esc，并在滚动/窗口尺寸变化时关闭，清理事件监听。
- 999 × 850 实测：提示范围 x=213.7–493.7，侧栏右边界 x=256，左侧跨界部分完整可见。Esc 关闭成功；内容滚动至 scrollTop=182 后提示关闭，顶部菜单 top=0。
- `pnpm lint`、面板设置 3 项定向测试、`pnpm build`、`git diff --check` 通过。截图位于 `tooltip-layer/ip-tooltip-fixed.png`（同本次 visualization 目录）。

## 2026-10-02 订阅分享弹窗层级修复

- 订阅分享弹窗通过 Portal 挂载到 `.openbox-app` 根节点，脱离 workspace 堆叠上下文，同时继承现有主题变量；仅调整该弹窗的挂载位置，未改变分享 API 或请求数据。
- 999 × 850 实测弹窗 x=131.5、宽 736，侧栏区域 x=100 的命中元素属于遮罩，侧栏不再盖住弹窗。协议下拉显示 http/https，Esc 只关闭菜单；取消和关闭按钮均能关闭弹窗。未保存测试分享。
- `pnpm lint`、订阅设置 5 项定向测试、`pnpm build`、`git diff --check` 通过。截图位于 `share-overlay/subscription-share-fixed.png`（同本次 visualization 目录）。

## 2026-10-02 订阅编辑弹窗层级修复

- 订阅卡片编辑（及共用该表单的新增订阅/节点）通过 Portal 挂载到 `.openbox-app` 根节点，继承主题变量并脱离 workspace 堆叠上下文；表单状态、API 和提交逻辑保持原样。
- 999 × 850 实测编辑弹窗 x=163.5、宽 672，侧栏区域由遮罩覆盖。更新周期菜单完整显示，Esc 关闭菜单后编辑弹窗仍在；规则页 9 个地区选择器正常渲染，取消和关闭按钮有效，未保存测试编辑。
- `pnpm lint`、订阅设置 5 项定向测试、`pnpm build`、`git diff --check` 通过。截图位于 `subscription-editor-overlay/subscription-editor-fixed.png`（同本次 visualization 目录）。

## 2026-10-02 出站节点提示与编辑浮层修复

- 出站节点卡片在按钮提示 hover/focus-visible 时提升层级，避免后续卡片的 backdrop-filter 堆叠上下文盖住提示。其余状态保留原有卡片顺序与拖拽逻辑。
- 分组编辑表单通过 Portal 挂载到 `.openbox-app` 根节点，保留主题变量并脱离 workspace 层级；共用表单的添加分组及编辑内部确认内容沿用相同挂载位置。
- 999 × 850 实测：第一张卡片启停提示完整覆盖第二张卡片，提示所在卡片 z-index=4。内置直连和所有-自动的编辑弹窗完整可见；侧栏由遮罩覆盖，分组规则菜单三个选项正常，Esc 关闭菜单后弹窗仍在，取消与关闭有效。未保存编辑或切换真实出站启停状态。
- `pnpm lint`、出站节点 5 项定向测试、`pnpm build`、`git diff --check` 通过。截图位于 `group-overlay/group-tooltip-fixed.png` 和 `group-overlay/group-editor-fixed.png`（同本次 visualization 目录）。

## 2026-10-02 节点预览空状态对齐

- 原版新增订阅弹窗的右侧“节点”页签在未填写输入时显示 1 px 虚线边框、15.8 px 圆角、24 px 内边距、24 px 搜索图标和 14/20 px 提示文字；区域实测高 102 px。
- 补齐上述样式与图标；仅空状态移除迁移版 330/350 px 最小高度，恢复原版自然高度。预览数据、加载/错误分支和 API 未改变。
- 内置浏览器对照原版与本地，空区域均为 102 px；类型检查、构建及 `git diff --check` 通过。截图位于 `subscription-preview-border/preview-empty-fixed.png`（同本次 visualization 目录）。

## 2026-10-02 窄屏设置菜单横向滚动修正

- 原版菜单列表为独立的 flex:1 / min-width:0 / overflow-x:auto 区域，右侧操作按钮为固定兄弟区域；菜单按剩余内容使用 28 px 边缘渐隐。迁移版旧规则覆盖了横向 overflow，且操作按钮与菜单共用滚动容器。
- 拆分菜单与操作区，删除覆盖规则；使用浏览器原生横向滚动，滚动和尺寸变化更新边缘渐隐。修正操作按钮 padding 导致图标被压缩，恢复 32 px 按钮 / 16 px 图标。API、请求与数据渲染逻辑未改动。
- 999 × 850 展开侧栏：菜单可见宽 615 px、内容宽 1023 px，实际横向滚动 scrollLeft=408；三个操作按钮始终位于 x=887/923/959，完整可见且命中检查有效。
- 871 × 650 收起侧栏：实际左右滚动 scrollLeft=0→344→0；操作按钮始终位于 x=759/795/831。添加分组与添加订阅按钮均成功打开对应弹窗，随后取消，未保存测试数据。面板内容纵向 scrollTop=74 时顶部栏 y=0、高 48 px。
- 验证：`pnpm lint`、设置导航/分组/订阅三文件定向测试（11 项）、`pnpm build`、`git diff --check` 通过。截图：`settings-header-scroll/narrow-scrolled.png`（同本次 visualization 目录）。

## 2026-10-02 分组编辑缩放与动态/静态 UI 对齐

- 对照原版在线分组编辑：32 px 缩放组合、14 px 加减 SVG、48 px 数值、文字重置按钮；补齐缩放数值说明全文与按钮短提示。图标选择触发器恢复 16 px 预览。32 px 表单、224 px 图标选择、158 px 缩放、独立整行测速地址、90dvh 弹窗和背景模糊均按原版排列。
- 动态组恢复单列，计数放入边框内的标题行；节点列表字号 14/20 px，最大高 224 px，边框区域总高 255 px。关键词 Cloudcone 实测命中后端 5 个节点，空关键词命中 11 个，不匹配时为 0 并显示原版空状态文案。
- 静态组采用紧凑搜索/工具栏、256 px 列表滚动区、成员类型标记、右侧添加和左侧移出箭头。两侧筛选根据各自真实成员生成：空已选仅“全部”，加入直连后出现“全部节点组”；订阅项按真实数据去重。新增 1 项定向测试保护此数据渲染契约。
- 浏览器验证缩放 0→1→0→-1→0、完整说明/短提示、动态过滤、静态勾选添加/移出、搜索、shadcn 筛选菜单。999 × 850 与 600 × 850 布局核验；窄屏弹窗覆盖完整视口，缩放长提示向内对齐。原版和本地测试表单均取消，未保存测试配置。
- `pnpm lint`、分组 6 项定向测试、`pnpm build`、`git diff --check` 通过。截图位于 `group-editor-alignment/dynamic-scale.png`、`dynamic-tooltip.png`、`static.png`（同本次 visualization 目录）。

## 2026-10-02 内置分组高度与编辑提示修正

- 原版直连编辑弹窗为内容自适应：桌面实测高 206 px。内置分组单独使用 auto 高度，提示文字 12/16 px；普通分组继续保留 90dvh 大面板。窄屏内置弹窗保持居中、圆角与 70dvh 上限。
- 卡片层级提升条件与提示显示条件统一为 hover / focus-within，覆盖鼠标点击后保留焦点的情况；任一出站节点弹窗打开时关闭背景卡片提示，避免在遮罩后残留。
- 内置弹窗及其内容允许图标选择浮层越界显示，防止收缩后裁切菜单。999 × 850 实测直连弹窗高 206 px，图标菜单延伸到 y=790 的部分可命中；600 × 850 实测内置弹窗高 266 px。未打开弹窗时第一张卡片提示完整覆盖第二张卡片，z-index=4；弹窗打开后提示 hidden。普通分组高 765 px 保持不变。测试表单均取消，未保存。
- 验证：类型检查、分组 6 项定向测试、构建及 `git diff --check` 通过。截图位于 `builtin-group-fix/direct-auto-height.png`、`edit-tooltip.png`（同本次 visualization 目录）。

## 2026-10-02 分组图标选择器与缩放提示对齐

- 对照原版所有-自动编辑：选择浮层为 256 px 宽、最大 320 px 高的单列列表，包含 896 个图标及“无”；搜索“日本”时自然收缩到 154 px。复用完整图标目录、分类、源 SVG 与国家代码，移除旧 14 项双列选择器及其失效样式、资源引用。
- 搜索框右上角 × 只清空搜索并保留面板。实测外部点击、选择图标及 Esc 关闭选择器；Esc 保留分组编辑弹窗。品牌 OpenAI、地区日本及空图标均正确更新表单预览。直连图标面板优先向下展开，桌面坐标与原版同为 x=208 / y=370 / 256×320。
- 图标面板 Portal 到应用根节点，缩放短提示及完整说明使用独立 Portal（z-index 1200，高于选择器 1000），避免被图标面板或弹窗滚动区域遮挡。600×850 实测两浮层完整处于视口内；图标列表实际滚动后保持打开，600×500 编辑内容实际滚动 269 px 后关闭图标面板，避免与触发器错位。
- 回归面板设置 Google 图标选择器：896 项、256×320、Esc 关闭正常。测试表单均取消，未保存后端配置；关闭测试弹窗后无残留图标面板或缩放提示，临时标签页已清理，视口已恢复。
- 验证：`pnpm lint`、`pnpm test src/openbox`（11 文件 / 99 项）、`pnpm build`、`git diff --check` 通过。构建保留已有大 chunk 提示。截图位于本次 visualization 目录 `group-icon-picker/picker-tooltip.png`、`group-icon-picker/narrow.png`。

## 2026-10-02 缩放提示脱离应用裁切容器

- 用户反馈直连编辑中的完整说明被底栏裁切。新打开页面原有提示已可跨越底栏，但仍 Portal 在 `overflow: hidden` 的应用根内；本次将共用缩放提示直接 Portal 到 `document.body`，保留 z-index 1200。根据渲染后的实际高度定位，下方空间不足时向上显示，避免应用或弹窗容器裁切及视口底部溢出。
- 1280×720 逐一验证直连、所有-自动、所有-手动、拒绝：长说明及缩小/放大/重置短提示均正常。直连和拒绝的长提示 bottom=418 px、弹窗 footer top=414 px，提示完整跨越底栏。四张卡片关闭弹窗后均无提示残留。
- 600×850 再逐一验证四张卡片，图标选择面板与长提示同时显示；提示均挂载 BODY、完整位于视口内。较矮窗口 600×180 与浏览器最小高度 600×160 的提示也未超出视口。未保存测试配置，视口已恢复，临时标签页已关闭。
- `pnpm lint`、分组 6 项定向测试、`pnpm build`、`git diff --check` 通过；构建保留已有大 chunk 提示。截图位于本次 visualization 目录 `scale-tooltip-boundary/direct.png`、`scale-tooltip-boundary/narrow-direct.png`。

## 2026-10-02 图标列表选中与悬停背景修复

- 当前项定位与 `data-active` 原已生效，但列表将选中/悬停背景复用了白色表单 `--field`，导致白色浮层上看不到状态。本次只修改列表背景规则，保留当前值与滚动逻辑。
- 在线原版实测：未悬停选中项为 `oklab(0.93 0 0 / 0.75)`，实际鼠标悬停项为不透明 `oklch(0.93 0 0)`；本地修正后计算值逐项一致。深色值对应原版 CSS 中 forest 主题 `--color-base-200: oklch(18.522% .007 17.911)`，同样保留选中 75% / 悬停不透明的差别。
- 1070×850 逐一验证直连、所有-自动、所有-手动、拒绝：展开时各有且仅有一个当前项，高亮完整可见。临时选择“地球·彩色(欧非)”后重新展开，唯一高亮跟随新值；取消测试表单，未保存后端配置。真实鼠标悬停与当前项高亮同时可见，视口与临时标签页已恢复/清理。
- `pnpm lint`、`pnpm build`、`git diff --check` 通过。纯样式修正未新增复述样式的单元测试。截图位于本次 visualization 目录 `icon-option-states/selected-and-hover.png`。

## 2026-10-02 公共弹窗挂载与自动分组逻辑

- 根因：公共 Modal 只渲染弹窗，各调用方自行决定 Portal；自动分组及确认框仍在 z-index=1 的 workspace 内，无法覆盖 z-index=2 的侧栏。现在由 Modal 统一 Portal 到应用根（保留主题变量），z-index=100，页面只负责调用。删除分组编辑、订阅编辑/分享及面板设置的重复 Portal；使用独立标题 ID 支持嵌套对话框。
- 已核对全部 OpenBox Modal 调用。1070×850 真实操作检查：直连、自动、手动、拒绝编辑，新增分组、删除分组、恢复默认、自动分组，新增/编辑订阅、新增分享，修改密码、测试站点恢复默认，新增终端、策略设置、连接设置及详情。14 种弹窗标题 / 21 次记录均脱离 workspace；侧栏区域命中遮罩，弹窗标题命中弹窗。故障转移中临时添加节点并打开“删除页签”，确认框位于编辑弹窗上方，取消后返回编辑。后端当前没有分享或故障转移配置，编辑分享及切换原有故障转移规则的确认分支核对共用组件，未创建线上数据来触发。
- 原版自动分组对应 `NodeGroupsPanel`，地区目录/匹配规则来源及资源哈希见 `assets/group-countries.md`。补齐 52 地区，复刻国旗归一化、短代码字母边界、CN2 排除、非互斥关键词匹配；自动分组计数和动态组预览共用匹配规则。
- 对照原版与本地：默认六地区中仅美国 11 个节点；清空选择后只有美国可添加。新增列表只显示有节点且未选的地区，并按节点数排序。空选择或空类型禁用生成，默认零节点地区仍可生成；勾选两种类型显示 12，单独添加美国并勾选两种类型显示 2。保留拖动排序、同名跳过及全部已存在提示，按国家归拢、自动在手动前；生成参数及 GET/PUT 请求、后端归一化返回与 dropped/dangling 字段通过 Mock API 测试。
- 600×500 实测自动分组弹窗位于视口内，高 450 px，内容可滚动、底栏按钮完整可见。测试弹窗全部取消，没有生成、删除、恢复或保存线上配置，未输入密码；临时标签页关闭，视口恢复。
- `pnpm lint`、`pnpm test src/openbox`（11 文件 / 104 项）、`pnpm build`、`git diff --check` 通过。构建保留已有大 chunk 提示。截图与原始检查记录位于本次 visualization 目录 `modal-auto-group-audit/`：`auto-groups.png`、`auto-groups-narrow.png`、`subscription-share.png`、`nested-confirm.png`、`modal-checks.json`。嵌套场景中底层编辑弹窗被确认遮罩覆盖是预期；窄屏记录的 x=20 探针位于弹窗边缘，命中弹窗而非遮罩本身，另核验其所属遮罩正常。

## 2026-10-02 自动分组地区排序与添加下拉框

- 对照原版 `NodeGroupsPanel` 的国家列表与 CountryPicker：列表使用统一边框、36 px 连续行、16×12 px 国旗、14 px 名称、12 px 计数和 24 px 删除按钮。原版排序指定把手、force-fallback、fallback-on-body 和 opacity-40 占位；本地取消整行原生拖拽，改为把手触发的 Pointer 拖动，预览独立挂载应用根，保留淡色占位与拖动到边缘时的滚动。
- 1280×720 实测：香港从首行拖至日本后，顺序为台湾、新加坡、日本、香港、韩国、美国；向上拖动恢复初始顺序。拖动过程中只有一个 542×36 px 完整预览和一个占位，释放后两者均为零；文字区域拖动不改变排序。测试排序未保存后端配置。
- 添加国家/地区改用共用 shadcn/Radix Select，补齐搜索、清空按钮、国旗/名称/代码、悬停与键盘高亮，以及“没有匹配的国家/地区”空状态。浮层脱离弹窗滚动区域，并保留应用主题。默认六地区全部选中且只有美国有节点时，原版和本地均无可添加项，完整空面板同为 256×94 px，不再显示空白条；保持只列出有节点且未选地区的原版规则。
- 浏览器验证：移除美国后可添加美国；搜索 JP 显示无匹配，× 清空搜索且面板保持打开；ArrowDown / Enter 可选择美国并关闭面板，Esc 仅关闭下拉框。高亮背景与原版同为 `oklch(0.93 0 0)`；600×500 窗口中浮层完整可见，弹窗高 450 px，底栏按钮完整可见。回归分组规则共用 Select 的当前项、三种选项与关闭行为。
- `pnpm lint`、`pnpm test src/openbox`（11 文件 / 104 项）、`pnpm build`、`git diff --check` 通过。构建保留已有大 chunk 提示。本次未修改 API 或生成逻辑，所有测试表单取消；临时标签页清理，视口恢复。截图位于本次 visualization 目录 `country-picker-drag/`：`empty-dropdown.png`、`original-empty-dropdown.png`、`drag-preview.png`、`narrow-dropdown.png`、`country-option.png`。
