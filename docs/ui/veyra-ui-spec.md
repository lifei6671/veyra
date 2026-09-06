# Veyra UI Spec

> 状态：已确认
>
> 版本：0.1-draft（2026-09-05）
>
> 适用范围：Veyra 后续渐进式 UI 调整
>
> 前置证据：`docs/ui/clash-verge-ui-analysis.md`、`docs/ui/veyra-current-ui-audit.md`

## 1. 目的与约束

本文冻结 Veyra 自己采用的视觉规则，避免后续页面实现者自行决定尺寸、间距、背景、
状态或组件边界。目标是统一和收敛当前实现，不是新建 UI Framework。

必须遵守：

1. 保留当前 Router/page switching、state、IPC、query、event、runtime 和 Tauri 行为。
2. 视觉语言参考 Clash Verge Rev，但不复制其源码、品牌、Logo、文案、Mihomo/Clash
   数据模型、命令或依赖体系。
3. 继续使用 React + 原生 HTML + CSS variables；本规格不授权引入组件库、Router、
   CSS-in-JS 或主题框架。
4. 先复用现有组件和 class API。只有当前不存在职责承载者时，才新增最小组件。
5. 普通 surface 不用 gradient、glass、blur 或 shadow；overlay 也采用 border-first。
6. 每项页面改造分别验证 light/dark、loading/empty/error/busy/disabled 和实际操作。

Reference Evidence：可迁移边界见 `docs/ui/clash-verge-ui-analysis.md:409-460`；
当前行为冻结见 `docs/ui/veyra-current-ui-audit.md` 第 7 节。

## 2. Token 决策规则

每个 token 必须标记来源：

- **REFERENCE**：参考源码提供精确、可迁移值。
- **KEEP**：当前 Veyra 已存在合理值，直接冻结。
- **VEYRA DECISION**：参考为 `UNKNOWN` 或非全局值，当前又必须统一，由本文明确决定。
- **UNKNOWN**：当前阶段没有足够证据；后续 Agent 不得自行填值。

未来实现时，token 的唯一 Source of Truth 应继续位于 `src/styles.css :root` 及 dark
media override。本文只定义目标，不在本阶段修改 CSS。

## 3. Design Tokens

### 3.1 Layout geometry

| Token | Final value | Source | Rule / Evidence |
|---|---:|---|---|
| Sidebar width | `200px` fixed | KEEP / VEYRA DECISION | 当前 `src/styles.css:79-89`；参考只有 `flex-basis:200px`，不是固定宽度，见 reference `:160,419,454` |
| Collapsed sidebar width | `UNKNOWN` | UNKNOWN | Veyra 当前无 collapse；不得仅因参考有 72px 就默认实现 |
| Page header height | `58px` | KEEP + REFERENCE | 当前 `src/styles.css:181-190`；reference `:162,398` |
| Header horizontal padding | `20px` | KEEP + REFERENCE | 当前 `src/styles.css:181-190`；reference `:162,398` |
| Header horizontal padding, narrow | `14px` | KEEP | 当前窄窗口 media rule；参考未提供 Veyra narrow 值 |
| Page padding, default | `10px` | KEEP + REFERENCE | 当前 `src/styles.css:208-217`；reference `:162,398` |
| Page padding, narrow | `8px` | KEEP | 当前窄窗口 media rule |
| Page padding, full | `0` | REFERENCE | 仅用于数据密集页，由页面内部 toolbar/list 管理；reference `:162,398` |
| Content/surface padding | `16px` standard; `12px` dense | VEYRA DECISION | 当前首页 20、subscription 12；参考无全局 token，本文取现有高频值之间的收敛值 |
| Section gap | `12px` | VEYRA DECISION | 参考 Home/Settings 使用同一 `spacing=1.5` 但像素取决于 MUI；本文固定为 12px，reference `:163,365,421` |
| Component gap | `8px` compact; `12px` standard; `16px` form/large | KEEP / VEYRA DECISION | 当前 toolbar、button group、card/form 中反复出现；不允许页面自创其它常规 gap |
| Settings columns | desktop `2 × 1fr`; narrow `1fr` | REFERENCE pattern / VEYRA DECISION | 参考为 6/12 两列，像素与 breakpoint 不可复制；reference `:333-370,402` |
| Settings breakpoint | `720px` | VEYRA DECISION | 参考 breakpoint 依赖 MUI，当前无设置页；后续实现统一使用此 Veyra 值 |

`full` 不是“无布局”。它表示 Page 不再包 10px 外边距，页面必须自行提供 toolbar、
列表内 padding 和内部滚动，不能让内容贴到窗口边缘。

### 3.2 Background hierarchy and color

Veyra 采用三个结构背景层。以下值来自参考 palette，并修正当前暗色语义混用：

| Token role | Light | Dark | Source |
|---|---:|---:|---|
| `window` | `#ECECEC` | `#2E303D` | REFERENCE：`clash-verge-ui-analysis.md:157` |
| `shell/sidebar/header` | `#F5F5F5` | `#2E303D` | REFERENCE：`:158-160,193` |
| `page-canvas` | `#ECECEC` | `#1E1F27` | REFERENCE：`:161` |
| `content-surface` | `#FFFFFF` | `#282A36` | REFERENCE：`:161,399-400` |
| `control-surface` | `#FFFFFF` | `#282A36` | VEYRA DECISION：与 content surface 同层，用 border 区分 |
| `popup-surface` | `#FFFFFF` | `#282A36` | VEYRA DECISION：与 content surface 同层，用 backdrop/border 区分 |
| `primary` | `#007AFF` | `#0A84FF` | KEEP + REFERENCE：`:185` |
| `secondary` | `#FC9B76` | `#FF9F0A` | REFERENCE；当前 light `#FC7849` 后续收敛，见 current audit 2.3 |
| `text-primary` | `#000000` | `#FFFFFF` | KEEP + REFERENCE：`:187` |
| `text-secondary` | `#3C3C4399` | `#EBEBF599` | KEEP + REFERENCE：`:188` |
| `error` | `#FF3B30` | `#FF453A` | REFERENCE：`:190` |
| `warning` | `#FF9500` | `#FF9F0A` | REFERENCE：`:191` |
| `success` | `#06943D` | `#30D158` | REFERENCE：`:192` |
| `divider` | `rgba(0,0,0,.06)` | `rgba(255,255,255,.06)` | REFERENCE：`:169,398-400` |

层级规则：Shell/Sidebar/Header 同层；Page canvas 承载滚动内容；Card、SettingSection、
Dialog、Menu、Input 使用 content/control/popup surface。不得继续用一个
`--card-background` 同时代表 Page canvas 和所有 surface。

### 3.3 Border, radius and shadow

| Token | Final value | Source | Rule |
|---|---:|---|---|
| Structural border | `1px solid divider` | KEEP + REFERENCE | Sidebar 右边界、PageHeader 下边界、surface/popup 分隔 |
| Surface radius | `8px` | VEYRA DECISION | 参考高频 8px但无有效全局 token；本文为 Veyra 统一值，reference `:171,401,456` |
| Control radius | `8px` | KEEP | 当前 Button/Input 已稳定使用 |
| Popup radius | `8px` | VEYRA DECISION | 将当前 Dialog 12、Menu 8、Toast 6/8 收敛 |
| IconButton radius | `4px` standard; `50%` circular variant | KEEP | 当前 header/document 与 refresh 的真实角色 |
| Regular surface shadow | `none` | KEEP + REFERENCE | current cards 已满足；reference `:172,399,424` |
| Popup shadow | `none` | VEYRA DECISION | 参考全局禁用 shadow；Veyra 使用 backdrop + 1px border 表达层级，不保留三套强 shadow |
| Drag shadow | `UNKNOWN` | UNKNOWN | Veyra 当前无拖拽 surface；不得预建参考的拖拽例外 |

### 3.4 Typography

字体继续使用当前系统字体栈，Windows 追加打包的 `Twemoji Mozilla`。表单控件必须
`font-family:inherit`；Twemoji 只处理 emoji/旗帜，不作为中文正文字体。

| Role | Size / weight / line-height | Source |
|---|---|---|
| Page title | `20px / 700 / 24px` | KEEP + REFERENCE：current `styles.css:193-197`; reference `:165-166,403` |
| Section title | `16px / 700 / 22px` | REFERENCE + VEYRA DECISION：参考 size/weight 明确，line-height 为 Veyra 决策 |
| Card title | `18px / 600 / 24px` | KEEP / VEYRA DECISION：当前 600；参考仅为 medium |
| Body / Setting label | `14px / 400 / 20px` | REFERENCE + VEYRA DECISION：参考 row 14，完整 line-height 原为 UNKNOWN |
| Supporting text | `13px / 400 / 18px` | KEEP |
| Caption / metadata | `12px / 400 / 16px` | KEEP |
| Sidebar label | `16px / 400 / 24px` | KEEP |

不得把参考中日志 1.35 或 rule item line-height 2 提升为全局正文规则。页面如需等宽字体，
必须由真实 Logs/Editor 需求单独决定。

### 3.5 Controls and icons

| Token | Final value | Source | Scope |
|---|---:|---|---|
| Standard control height | `36px` | KEEP / VEYRA DECISION | Button、Input、Select；参考全局为 UNKNOWN，但 Profile import 局部也是 36px |
| Compact control height | `32px` | KEEP | header actions、document tools、card actions |
| Input trailing action | `30px` box / `18px` glyph | KEEP | 仅 input 内 compact variant |
| Button horizontal padding | `14px` | KEEP | 文本按钮 |
| IconButton standard | `32 × 32px` | KEEP | Header、Card、Document toolbar |
| Standard action glyph | `20px` | VEYRA DECISION | 收敛当前 18/22；input trailing 仍为 18 |
| Navigation glyph | `24px` | KEEP | Sidebar only |
| Inline/status glyph | `16px` | KEEP | RateRow、caption/status |
| Switch | `42 × 26px`; thumb `22px`; track radius `13px` | REFERENCE | 当前不存在；后续 Settings 即时开关唯一规格，reference `:197-205` |
| Checkbox | platform/native size | KEEP | 多选、确认、表单选项；不得替换成 Switch |

参考没有全局 Button/Input/Icon 精确值，因此 36/32/20 是 Veyra 对当前实现的收敛决策，
不是“Clash Verge 精确值”。

## 4. Interaction State Spec

| State | Required visual rule | Behavior boundary | Evidence / source |
|---|---|---|---|
| Hover | 非 selected item 使用 `selection` 背景；bordered card 可轻微提升 border 到 primary 30%，不得位移/加 shadow | 不触发业务动作 | current Sidebar `styles.css:152`; reference provider hover `:378` |
| Pressed / Active | Button 使用 primary 背景轻微加深或 0.92 opacity；持续 active/selected 不用 pressed 表达 | 原 handler 不变 | VEYRA DECISION；参考通用 exact value UNKNOWN |
| Sidebar selected | primary 15% light / 35% dark；hover 保持同一 selected 背景 | `aria-current=page` 继续绑定 `activePage` | KEEP + REFERENCE `:178,380` |
| Card/row selected | 左侧 `3px solid primary`；title 使用 primary；可叠 primary alpha surface | `aria-selected`/真实 state 驱动 | REFERENCE `:381,406`; current Subscription active edge |
| Disabled | `opacity:.55`; default cursor; hover/pressed 不改变视觉 | 原生 disabled + handler guard 必须同时保留 | KEEP / VEYRA DECISION |
| Loading, local action | spinner/rotating icon/文案在原动作位置，20px inline spinner | 只锁定冲突动作；保留业务 pending owner | REFERENCE `:383-384,392` |
| Loading, page/card | `loading | empty | error | render` 互斥；surface 内使用中性文字或 skeleton | 不把 Tauri 不可用误作 empty | REFERENCE `:383`; current Subscriptions dispatcher |
| Expand / Collapse | chevron 明确方向；row/header 保持原位；动画值 `UNKNOWN` | 状态由领域组件持有 | Reference animation 为 UNKNOWN `:385` |
| Dialog | backdrop + bordered popup；title/content/actions；Esc、focus trap、return focus、busy close guard | 复用现有 focus 和 async 行为 | current `SubscriptionPage.tsx:438-446`; `DocumentEditor.tsx:93-115` |
| Dropdown / Context Menu | 8px popup、1px border、无 shadow、dense items；danger 保留 error 色 | position、keyboard、focus 逻辑不变 | current `SubscriptionPage.tsx:430-436,531-545` |
| Tooltip | 可聚焦触发；tooltip 视觉延迟/时长 `UNKNOWN` | 未引入组件前保留 `title/aria-label` | Reference tooltip 形态不统一 `:388` |
| Toast / Notice | fixed corner stack；edge 20px、gap 10px、max-width 360px、8px radius、1px border、可关闭 | status/error role 与现有 timer 保留；默认时长不在本阶段统一 | REFERENCE `:389`; duration remains VEYRA UNKNOWN |

### 4.1 Hover color tokens

| Token | Light | Dark | Source |
|---|---:|---:|---|
| Generic hover | `rgba(0,0,0,.05)` | `rgba(255,255,255,.08)` | KEEP：current `styles.css:20,41` |
| Selected background | `rgba(0,122,255,.15)` | `rgba(10,132,255,.35)` | KEEP + REFERENCE |
| Selected hover | same as selected | same as selected | REFERENCE：selected 不因 hover 改变，`:376,380` |

## 5. Component Composition Spec

### 5.1 AppShell

```text
App（保留 state / IPC / handlers）
└── AppShell（pure presentation）
    ├── Sidebar
    ├── Page outlet（仍由 activePage + hidden 驱动）
    └── FeedbackHost presentation
```

AppShell 不拥有业务 state，不迁移 Router，不决定页面数据。当前 `App` 可在后续阶段仅抽取
DOM 边界；调用和页面挂载行为保持不变。

### 5.2 Page

```text
Page
├── PageHeader
│   ├── title
│   └── optional status/actions
└── PageContent(default | full)
```

- `default`：10px 外 padding，适合卡片页、Overview、Settings。
- `full`：0 外 padding，适合 Proxies、Connections、Logs 等 dense page；页面自己管理
  toolbar padding、list padding 和内部 scroll。
- PageHeader 不允许 hero title、subtitle marketing block、gradient 或大面积留白。

### 5.3 Surface / Card

Surface 是视觉规则，不要求立即创建万能 React `Card`：

```text
Surface
├── optional header → icon + title + action
└── content
```

标准为 content-surface、1px divider、8px radius、16px padding、无 shadow；dense variant
用 12px padding。领域 Card 继续持有自己的数据、选中和动作。

### 5.4 Settings（future gate）

```text
SettingsPage
└── Page(default)
    └── SettingsGrid
        └── SettingSection × N
            ├── SectionTitle
            └── SettingRow × N
                ├── Label
                ├── optional Description
                └── Control | Chevron | InlineLoading
```

- 上述 composition 仅描述真实 Settings Domain 获批后的映射目标，不授权创建字段。
- 桌面两列，窄窗口单列。
- SettingSection 采用 standard Surface；组间 gap 12px。
- SettingRow 使用 label 14/20、description 13/18；控制器右对齐。
- 行高度不设独立固定值，由上下 `10px` padding、文字和 control 共同决定；单行最小
  44px。该值是 Veyra Decision。
- 即时布尔值用 Switch；多选/确认继续用 Checkbox。
- 复杂配置进入 Dialog，主页面只保留扫描式 summary。
- Settings 的字段、值、IPC 和保存模型必须来自 Veyra 自己的已批准需求，不能复制参考项目。
- 在此之前生产 Settings 保持简洁 placeholder；Runtime status、Observation、连接数、
  config revision 和 runtime memory 不得作为 Settings 主体。

### 5.5 Dense lists

不建立通用 `List` 平台。Proxies、Connections、Logs 各自拥有领域 row，但共享：

- `full` Page；
- toolbar/control tokens；
- 1px divider；
- 14/20 body 和 12/16 metadata；
- loading/empty/error/render 互斥入口；
- selected 3px primary edge；
- 只有真实数据量和测量证明需要时才使用 virtualization。

Reference Evidence：`docs/ui/clash-verge-ui-analysis.md:221-237,435-436`。

## 6. Target Pattern → Existing Veyra Component

| Target Pattern | Existing Veyra Component | Action |
|---|---|---|
| AppShell | `App` 内 `.app-shell` | REFACTOR presentational boundary；业务 owner 保留 |
| Sidebar | `App` 内 `.sidebar` | REFACTOR 为纯展示；继续用 `navigationItems/activePage` |
| SidebarItem | `navigationItems.map → button` | ADJUST |
| Sidebar telemetry | `SidebarTraffic` | KEEP |
| Page | `.home-content` | ADJUST/semantic alias；保留布局行为 |
| PageHeader | `.home-header` | KEEP |
| PageContent | `.page-scroll` | KEEP；增加 full 规则时不得改 scroll owner |
| Surface | `.traffic-panel/.observation/.placeholder-card` | ADJUST token，不先造万能 Card |
| Overview Card | `.traffic-panel/.observation` | ADJUST composition/density |
| Subscription Card | `SubscriptionCard` | ADJUST |
| Dense card grid | `.subscription-grid` | ADJUST min-width 到 260px |
| Button | `.primary-button/.secondary-button/.danger-button` | ADJUST CSS states，保留 class API |
| IconButton | header/document/refresh/field-action buttons | ADJUST 并明确 standard/circular/compact variant |
| Input | toolbar raw input | KEEP |
| Select / Dialog Input | raw select/input + `.outlined-field` | ADJUST height；KEEP composition |
| Checkbox | raw checkbox | KEEP |
| Switch | `Switch` | KEEP primitive；当前只在 development-only Visual Harness 验证，生产调用等待真实 Domain |
| Dialog | Subscription inline Dialog | REFACTOR shell，保留 state/focus/API |
| Specialized editor Dialog | `DocumentEditor` | ADJUST |
| Context Menu | `SubscriptionMenu` | ADJUST |
| Tooltip | native `title` | NEW minimal wrapper only when a real call site migrates |
| Notice / Toast | `.failure-toast/.subscription-notice` | REFACTOR presentational layer；保留各自 state/timer |
| Empty / Loading | `.subscription-state/.placeholder-card` | ADJUST shared visual tokens，不合并业务状态 |
| SettingSection | `SettingSection` | KEEP primitive；当前只在 development-only Visual Harness 验证 |
| SettingRow | `SettingRow` | KEEP primitive；当前只在 development-only Visual Harness 验证 |
| Generic List | 无 | 不新增；使用领域列表 + shared dense tokens |

这里没有 `REPLACE`。SettingSection/SettingRow/Switch 可作为明确标识的开发期 visual
fixture 存在，但进入生产 Settings 必须由真实页面需求触发；不能为了提高密度而创造字段。

## 7. Page-specific Target Rules

### 7.1 Overview

- 使用 default Page 和 12px grid gap。
- desktop 为 12 列；普通状态 surface 占 6 列，聚合流量图可占 12 列；narrow 回落单列。
- 保留 TrafficChart、TrafficRates、runtime controls 和 diagnostics 的数据/事件。
- 图表不是 hero：不得通过超大标题、gradient、shadow 或额外大留白强化。
- loading/unavailable/stopped 继续使用真实 observation state，不伪造 skeleton 数据。

### 7.2 Proxies

- 在 Subscriptions Golden Page 经人工视觉确认后另行实施。
- 使用 full Page；header actions、mode control、loading/empty/error/render、GroupList 分层。
- GroupHeader → optional tools → domain rows；selected 使用 3px primary edge。
- expand/collapse、延迟、测试、节点字段全部由 Veyra/sing-box 模型决定。

### 7.3 Subscriptions

- 第一张生产 Golden Page，用于冻结 Shell、Sidebar、PageHeader、toolbar、surface、
  card/grid、controls 和反馈状态的视觉语言。
- 保留现有 toolbar → states → grid → dialogs/menu/editor composition。
- grid min column `260px`；Card radius 8、dense padding 12、常态低强度 divider。
- active card 使用 3px primary edge 和 primary title，不通过外投影表达。
- Button/Input/Dialog/Menu/Notice 迁移到统一 token；不改业务状态机。

### 7.4 Routing / Connections / Logs

- 当前均为 placeholder；具体字段、列、filter、排序、行高和虚拟化阈值为 `UNKNOWN`。
- 只预先冻结 `full Page + toolbar + state dispatcher + dense domain rows` 视觉骨架。
- 不复制 Clash/Mihomo schema，不在本规格中创造 Veyra 业务字段。

### 7.5 Settings

- 当前不是生产 Golden Page，保持简洁 placeholder。
- 不使用 runtime/observation/connection/config revision/memory 填充页面。
- 只有产品与技术方案先冻结 Settings field、state ownership、persistence、IPC、platform
  behavior、error/loading semantics 后，才映射为 two-column SettingsGrid →
  SettingSection → SettingRow。
- 暂无真实调用场景的基础 primitive 只允许在 development-only Visual Harness 验证；
  Harness 不进入 Sidebar、Router 或生产信息架构。

## 8. Migration Plan

本计划是后续工作顺序，不授权本阶段改代码。每阶段应作为独立、可回滚、可验收的
UI delivery unit，不一次性重写全部页面。

### Phase 1 — AppShell + Sidebar + Subscriptions Golden Page

目标：用已有真实订阅行为冻结整个应用的背景层级、Page geometry、surface、toolbar、
card/grid、control 和 feedback state 语言。

顺序：

1. 在现有 CSS variable 体系中补齐本规格 token；不引入新 Theme Framework。
2. 仅抽离 AppShell/Sidebar/Page 的纯展示边界；保留 `activePage + hidden`、runtime 与 IPC。
3. SidebarTraffic 直接 KEEP；SidebarItem 只做 token 收敛。
4. 收敛 Subscriptions 的 Grid/Card/Button/Input/Dialog/Menu/Notice；保持其 state、IPC、
   query、focus、keyboard 和 DocumentEditor 行为。
5. 对无生产调用场景的 SettingSection/SettingRow/Switch 等 primitive，只建立
   development-only Visual Harness，不创建设置字段。
6. Settings 收敛为 truthful placeholder，等待真实 Domain gate。
7. 验证 light/dark、键盘 focus、selected、disabled/busy、Dialog、Context Menu 和 Notice。

停止条件：Shell、Sidebar、PageHeader、Page spacing、Surface、Button、IconButton、Input、
Select、Switch、Dialog、Notice 的值均能追溯到本文，不再由页面实现者临时决定；
Subscriptions 截图与逐项 parity review 可供人工验收。人工确认前不进入 Proxies。

### Phase 2 — Proxies（需 Phase 1 人工确认）

目标：用已冻结的视觉语言验证 full Page、dense group list、selected、expand/collapse 和
内部 scroll。

- 只接入 Veyra 现有/批准的 outbounds 数据和动作。
- 先做非虚拟领域列表；只有真实数据量证明需要时再加入 virtualization。
- 不从 Clash Verge 复制 group/provider/rule model。

### Phase 3 — Settings Domain gate

目标：先冻结真实 Settings 契约，再决定生产页面内容；这不是当前实现授权。

- 冻结 Settings field、state ownership、persistence、IPC、platform behavior。
- 冻结 error/loading/busy/rollback 语义。
- 冻结后才把真实设置映射为 SettingSection → SettingRow → Control/Chevron。
- 不反向为了使用 visual primitive 而创造业务设置。

### Phase 4 — Overview

目标：把两个全宽 section 调整为 6/12 响应式 Card grid，降低事实 hero 重心。

- 保留 observation、chart、runtime actions 和 details 行为。
- 不在 UI 改造中重写采样、IPC 或诊断模型。

### Phase 5 — Routing → Connections → Logs

目标：逐页替换真实 placeholder，每页独立定义 Veyra 业务 composition。

- 共同复用 full Page、toolbar、state dispatcher 和 dense visual tokens。
- 每页单独确认字段、列表语义、空/错/忙状态与是否需要 virtualization。
- 一个页面验收完成后再进入下一个，禁止全量并行重写。

## 9. 后续实现 Gate

每个迁移阶段至少需要：

- 变更范围只包含该阶段明确组件；业务 state/IPC/event diff 为零或有逐项理由。
- light 与 dark 的实际截图对比。
- normal、hover、selected、disabled、loading、empty、error、Dialog/Popup 等适用状态。
- 键盘导航、focus return、Escape、context menu 等现有交互回归。
- 页面 geometry 与 token 的 computed value 检查。
- `pnpm lint`、相关前端 tests、`pnpm build`；构建不替代 UI 实际操作验收。

本文经人工确认前只是一份候选规格。确认后仍应按 Phase 拆分实现，不得把本文视为一次
全量前端重构授权。
