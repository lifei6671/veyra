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
