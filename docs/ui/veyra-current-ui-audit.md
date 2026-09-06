# Veyra Current UI Audit

> 状态：已确认
>
> 审计日期：2026-09-05
>
> 范围：当前 Veyra 前端实现与 `docs/ui/clash-verge-ui-analysis.md` 的视觉、布局和组件组织差异
>
> 禁止项：本文不授权修改 React、CSS、Theme、业务逻辑、IPC、Router 或依赖

## 1. 审计边界与证据口径

本审计读取当前工作区实际源码，并在本地 Vite 预览中检查了浅色主题下的
Overview、Proxies、Subscriptions、Routing、Connections、Logs、Settings，以及
Subscriptions 的错误、Toast 和创建 Dialog 状态。预览运行在普通浏览器环境，Tauri
IPC 不可用，因此其中的“运行观测不可用”和订阅读取失败只用于检查视觉状态，不作为
业务缺陷证据。截图未写入仓库，以遵守本阶段只交付两份 Markdown 文档的限制。

证据分为三类：

- **Current Source**：当前 Veyra 源码，是现状的事实来源。
- **Reference Evidence**：`docs/ui/clash-verge-ui-analysis.md`，只提供可迁移的视觉和
  composition 依据，不提供 Veyra 业务模型。
- **Veyra Decision**：参考缺少精确值、但 Veyra 后续实现必须统一时，由
  `veyra-ui-spec.md` 明确冻结的项目决策。

不得根据文件名或 class 名推断能力。以下结论均来自 JSX、状态处理与实际 CSS。

## 2. Current UI Inventory

### 2.1 工程与组件关系

```text
src/main.tsx
└── StrictMode
    └── App
        ├── aside.sidebar（App 内联）
        │   ├── brand
        │   ├── navigationItems → 原生 button × 7
        │   └── SidebarTraffic
        ├── main.home-content（Overview）
        │   ├── home-header
        │   └── page-scroll
        │       ├── traffic-panel → TrafficChart + TrafficRates
        │       └── observation → runtime actions + details
        ├── SubscriptionPage
        │   ├── toolbar / state dispatcher / SubscriptionCard grid
        │   ├── inline Dialog / SubscriptionMenu / Notice
        │   └── lazy DocumentEditor
        ├── main.page-placeholder × 5
        │   └── Proxies / Routing / Connections / Logs / Settings
        └── failure-toast
```

Source Evidence：入口和全局样式挂载见 `src/main.tsx:1-9 createRoot`；导航元数据见
`src/App.tsx:36-46 navigationItems/NavigationPage`；完整 Shell composition 见
`src/App.tsx:164-227 App`；订阅页 composition 见
`src/components/subscriptions/SubscriptionPage.tsx:448-517 SubscriptionPage`。

### 2.2 架构清单

| Area | Current implementation | 关系与职责 | Source Evidence |
|---|---|---|---|
| App Shell | `App` + `.app-shell` | 两栏全高 Grid；`App` 同时持有壳层、运行时状态、动作、页面显隐和全局失败提示 | `src/App.tsx:48-227 App`; `src/styles.css:67-78 .app-shell` |
| Layout | `.sidebar` + 多个平级 `<main>` | 左侧固定导航，右侧每个页面自己带 header/content；没有 Outlet | `src/App.tsx:164-224 App` |
| Sidebar | `App` 内联 `<aside>` | brand、七个导航按钮、底部 `SidebarTraffic` | `src/App.tsx:164-180 App`; `src/styles.css:79-170` |
| Router | `activePage` 本地 state | 没有 React Router；按钮调用 `setActivePage`，所有主页面同时挂载并用 `hidden` 切换 | `src/App.tsx:49,62-64,169-175,181,212-224 App`; `src/styles.css:49 [hidden]` |
| Page container | `.home-content` | 2 行 Grid：58px header + 可滚动 content；名称虽含 home，实际跨页面复用 | `src/styles.css:172-180`; `src/App.tsx:181,213-224` |
| Page header | `.home-header` | 标题、状态或 actions；58px、横向 20px、下 divider | `src/App.tsx:182-183`; `SubscriptionPage.tsx:449-455`; `src/styles.css:181-207` |
| Card / Surface | CSS selector 组合 | 首页 section、placeholder、订阅 card、Dialog 各自实现，无统一 React `Card` | `src/styles.css:219-239,482-505,607-615` |
| Settings components | 不存在 | Settings 仍为通用 placeholder；不存在 SettingSection/SettingRow/Switch | `src/App.tsx:43,213-224` |
| List components | 不存在通用层 | Sidebar buttons、订阅 grid、诊断 `ul`、流量 RateRow 各保留领域语义 | `src/App.tsx:168-176,201-207`; `SubscriptionPage.tsx:468-472`; `SidebarTraffic.tsx:200-225` |
| Button | 原生 button + 3 个 class variant | `.primary-button/.secondary-button/.danger-button`，无 wrapper | `src/styles.css:452-468`; `SubscriptionPage.tsx:461-467,490-515` |
| IconButton | 四套原生 button | header、refresh、field trailing、document tools 具有不同 box/glyph 尺寸 | `SubscriptionPage.tsx:450-455,458-460,564`; `DocumentEditor.tsx:106-110` |
| Switch | 不存在 | 当前 checkbox 用于多选/确认，不是即时设置开关 | `SubscriptionPage.tsx:464,509-511,563` |
| Input / Select | 原生控件 | toolbar input 为 36px；Dialog input/select 为 38px；`.outlined-field` 提供浮动 label composition | `SubscriptionPage.tsx:457-460,496-514`; `src/styles.css:409-424,637-661` |
| Dialog | 页面内联 + `DocumentEditor` | 订阅 Dialog 由 mode 状态机渲染；DocumentEditor 是专用 Monaco Dialog | `SubscriptionPage.tsx:18-27,438-446,474-516`; `DocumentEditor.tsx:13-115` |
| Tooltip | 原生 `title` | 无可控视觉 Tooltip；同时保留 `aria-label` 的按钮可访问名 | `SidebarTraffic.tsx:201`; `SubscriptionPage.tsx:450-454,543,564-567`; `DocumentEditor.tsx:107-109` |
| Notice / Toast | 两套局部实现 | App failure toast 右下且可关闭；subscription notice 左下且不可关闭 | `src/App.tsx:60,66-70,225`; `SubscriptionPage.tsx:57,107-111,448`; `src/styles.css:335-386` |
| Theme | CSS media theme | `:root` 为 light，`prefers-color-scheme:dark` 覆盖；无 Theme Provider | `src/styles.css:7-46` |
| Design Tokens | 部分 CSS variables | 颜色集中；几何、spacing、radius、control size 仍散落在 selectors | `src/styles.css:7-46,67-713` |
| Global styles | `src/styles.css` | font、reset、Shell、Overview、Subscriptions、Dialog、Menu、Toast 混在同一文件 | `src/main.tsx:3`; `src/styles.css:1-727` |
| Component Library | 无通用 UI library | React + 原生 HTML/CSS；Monaco 和 QRCode 是功能依赖，不是 UI Framework | `package.json:13-27` |

### 2.3 Theme 与当前 Token Source of Truth

当前唯一全局视觉 Source of Truth 是 `src/styles.css`：

- `@font-face` 打包 Twemoji：`src/styles.css:1-5`。
- Light variables：`src/styles.css:7-26 :root`。
- Dark overrides：`src/styles.css:28-46 @media (prefers-color-scheme:dark)`。
- reset、字体继承、root/body：`src/styles.css:48-65`。

| Semantic role | Current light | Current dark | Current issue |
|---|---:|---:|---|
| Window/App | `#ececec` | `#2e303d` | dark window 与 shell 语义未分开 |
| Sidebar | `#f6f6f6` | `#282a36` | light 仅比参考 `#F5F5F5` 差 1 个色阶；dark 与目标 surface 角色相同 |
| Card/Header/Input/Dialog | `#ffffff` | `#1e1f27` | 一个变量承担 page header、surface、control、popup 多种角色 |
| Primary | `#007aff` | `#0a84ff` | 与参考一致 |
| Secondary | `#fc7849` | `#ff9f0a` | light 与参考 `#FC9B76` 不同；Veyra 是否采用参考值需明确 |
| Primary text | `#000` | `#fff` | 与参考一致 |
| Secondary text | `#3c3c4399` | `#ebebf599` | 与参考一致 |
| Divider | black 8% | white 8% | 参考结构 divider 为 6% |
| Hover selection | black 5% | white 8% | 当前已有独立 hover token |
| Selected | primary 15% | primary 35% | 与参考 Sidebar selected 一致 |

Source Evidence：当前值见 `src/styles.css:7-46`；参考 palette 与背景层级见
`docs/ui/clash-verge-ui-analysis.md:157-193`。

当前最大的 Theme 缺口不是“颜色不好”，而是暗色下一个
`--card-background:#1e1f27` 同时服务 header、card、input、dialog、menu，无法表达
参考中 `Shell/Sidebar → Page canvas → Content surface` 的三层层级。具体使用见
`src/styles.css:181-190,219-228,409-418,607-615,686-696`。

## 3. Existing Component Classification

分类只评价现有结构是否适合渐进式视觉统一。`REFACTOR` 不授权改变业务 API；
`REPLACE` 仅在现有模式无法承担目标职责时使用。本次没有需要 `REPLACE` 的现有组件。

| Existing Component | Classification | Current Issue | Target Rule | Recommended Change |
|---|---|---|---|---|
| `App` | REFACTOR | 同时持有 Shell、导航、runtime/IPC、Overview、placeholder 和 Toast | Shell 只组合 Sidebar、页面出口和 feedback host | 后续仅抽离 presentational boundary；保留全部 state、effect、handler、IPC 和页面挂载语义 |
| `.app-shell` | KEEP | 无明显几何问题 | 两栏、全高、内部滚动 | 冻结 Grid/full-height/overflow 行为 |
| 内联 `.sidebar` | REFACTOR | 不是独立组件，改视觉会触碰 `App` | 同一 `navigationItems` 驱动 SidebarItem，selected 仍来自 `activePage` | 可抽纯展示组件；不新增路由模型 |
| `navigationItems` | KEEP | 职责清楚 | 单一 metadata 驱动顺序、label、icon 和选择 | 保留 ID、顺序、文案和事件契约 |
| Sidebar 原生 button | ADJUST | 视觉值未 token 化，label 定位较专用 | 44px Veyra row、24px icon、8px radius、明确 hover/selected | 保留 button/aria/click API，只改 CSS token |
| `SidebarTraffic` | KEEP | 无结构性问题 | Sidebar 专用 telemetry，不抽成业务无关 Card | 保留 props、canvas、采样、缺口与 theme redraw |
| `RateRow` | KEEP | 仅服务 SidebarTraffic | 私有领域子组件 | 不抽象成通用 ListItem |
| `.home-content` | ADJUST | 名称偏首页，但已跨页面工作 | 统一 Page container：58px header + scroll content | 保留布局和滚动所有权；可增加语义 alias，不要求重命名 |
| `.home-header` | KEEP | 当前值已匹配参考核心基线 | 58px、20px x-padding、1px divider、20/700 title | 冻结为 PageHeader 基线 |
| `.page-scroll` | KEEP | 当前 desktop/narrow spacing 合理 | 默认 10px，窄窗口 8px；dense/full 页面可由页面管理内部滚动 | 保留 scroll reset 和 overflow 语义 |
| `.traffic-panel/.observation` | ADJUST | 20px padding、10px radius，单列大 surface 信息密度偏低 | surface 8px、16px padding、无 shadow；Overview 使用响应式 grid | 不改 runtime handlers、chart 或 details 数据 |
| `TrafficChart/TrafficRates` | ADJUST | 220px 图表和大数值 tile 形成事实上的 hero 重心 | 聚合图可全宽，但其他卡片保持同一 surface/density | 只调整 composition/尺寸；不改 observation 数据映射 |
| `.placeholder-card` | ADJUST | 240px 最小高度制造大面积空 Card；不是数据 empty state | 紧凑 Empty Surface，保留“未开放”语义 | 只收敛 padding/min-height/radius；不伪装成已实现页面 |
| `SubscriptionPage` | REFACTOR | 业务协调、Card、Dialog、Menu、Notice 同文件 | 保留页面状态机，只允许提取重复的纯视觉 shell | 不改 props、query/IPC、pending、validation、事件或 focus 语义 |
| `SubscriptionCard` | ADJUST | min-width 300、radius 10、常态全边框较重；active title 不变色 | dense grid 260px；8px surface；selected 3px edge + primary title | 保留 props、键盘/右键/选择/刷新行为 |
| `SubscriptionMenu` | ADJUST | 视觉 shadow 独立定义 | 统一 popup background/border/radius/shadow | 保留 position、focus、键盘和 danger action |
| Button class variants | ADJUST | 36px 合理，但缺统一 hover/pressed；状态值分散 | 36px、14px、8px radius、完整 hover/active/disabled | 保留 class API 和 handlers，先统一 CSS |
| Header IconButton | ADJUST | 32px box / 22px glyph 与其他 action 不一致 | 32px box / 20px glyph / 4px radius | 保留原生 button 和 aria |
| Refresh IconButton | ADJUST | 32px circle / 18px glyph 是领域特例 | 32px box；circular refresh 可作为显式 variant | 不把领域动画并入通用 API |
| Field trailing action | KEEP | 30px/18px 与 input 内槽位相符 | compact IconButton variant | 明确其 compact 角色，不强制拉到 32px |
| Document tool buttons | ADJUST | 32px box / 22px glyph | 与 header action 共用 32px/20px 规则 | 不改编辑器命令 |
| Raw checkbox | KEEP | 语义是多选、确认或表单选项 | Checkbox 与即时 Switch 分离 | 不得视觉替换为 Switch |
| Toolbar Input | KEEP | 36px 是当前稳定合理值 | 标准单行 control 36px | 保留 |
| Dialog Input/Select | ADJUST | 38px 与 toolbar 36px 不一致 | 统一 36px | 不改 value/onChange/disabled |
| `.outlined-field` | KEEP | label + control composition 清楚 | 可作为 Dialog form field | 保留 DOM 和 label 语义 |
| Inline subscription Dialog | REFACTOR | shell、focus 逻辑和 mode 内容混在页面 | 统一 Dialog shell；原 mode 状态机继续所有 | 提取时完整透传 busy、close、focus trap、return focus |
| `DocumentEditor` | ADJUST | 专用行为成熟；surface/header 与普通 Dialog 不一致 | 专业 Dialog 保留尺寸，复用 popup surface token | 不并入普通 Dialog，不改 Monaco 生命周期 |
| Native `title` tooltip | ADJUST | 无统一视觉，键盘/时机不可控 | 需要时采用轻量 Tooltip，同时保留 accessible name | 渐进替换；未替换前保留 `title/aria-label` |
| `.failure-toast` | REFACTOR | 与 subscription notice 的位置、关闭能力、surface 不一致 | 统一 Notice/Toast presentational host | 保留 App state、role、6 秒 timer 和关闭 handler |
| `.subscription-notice` | REFACTOR | 左下、无关闭；与 failure toast 覆盖同组 selector | 共用 feedback surface、位置和 stack 规则 | 保留本地消息、role 和生命周期 |
| Light/Dark media theme | KEEP | 无手动主题系统，但机制完整 | 继续随 OS；浅深分别验证 | 不引入 Theme Provider |
| Global font/reset | KEEP | 与参考字体方案一致 | 系统字体栈 + 打包 Twemoji + controls inherit | 保留资产、license 和继承 |
| CSS variables | REFACTOR | 颜色集中，几何和语义 surface 不完整 | 在现有 `:root` 建立单一 token SoT | 不引入 CSS-in-JS、Tailwind 或组件库 |
| Generic List/ListItem | KEEP（不存在通用层） | 现有列表语义不同 | 不预建万能 List | 后续按页面族复用 dense row token |

不存在的 `SettingSection`、`SettingRow`、`Switch` 和可控 `Tooltip` 不属于
`REPLACE`。它们只允许在对应真实需求进入实现时新增最小组件。

## 4. Current Page Composition

### 4.1 Overview

```text
App / page-home
├── PageHeader
│   ├── h1
│   └── runtime summary
└── PageScroll
    ├── traffic-panel
    │   ├── heading + eyebrow
    │   ├── TrafficChart
    │   ├── TrafficRates
    │   └── totals
    └── observation
        ├── status summary
        ├── runtime controls
        └── details → diagnostic list
```

Source Evidence：`src/App.tsx:181-210 App`、`src/App.tsx:245-278
TrafficRates/TrafficChart`。

现状规律：Header 和 page padding 已统一；内容是两个全宽 section 纵向堆叠。Loading、
stopped、unavailable 用文案和 `—` 表达，动作 busy 通过
`runtimeActionPending` 禁用，诊断使用原生 `<details>`。证据见
`src/App.tsx:157-162,182-208`。

### 4.2 Proxies / Outbounds

```text
page-outbounds
├── PageHeader → title
└── PageScroll → placeholder-card → “此功能暂未开放”
```

当前不存在代理组、节点、group list、selected node 或相关 state。不能将导航项存在解释为
页面已经实现。Source Evidence：`src/App.tsx:38,213-224`。

### 4.3 Subscriptions

```text
SubscriptionPage
├── PageHeader → title + IconButton × 4
├── PageScroll
│   ├── URL import toolbar
│   ├── optional batch actions
│   ├── loading | error | empty | ready
│   └── subscription-grid → SubscriptionCard × N
├── optional subscription Dialog
├── optional SubscriptionMenu
├── local Notice
└── Suspense → DocumentEditor Dialog
```

Source Evidence：`SubscriptionPage.tsx:448-517 SubscriptionPage`、
`SubscriptionPage.tsx:531-568 SubscriptionMenu/SubscriptionCard`。

这是当前最完整的页面 composition。列表状态互斥；active card 有 3px primary inset；
selection mode 使用真实 checkbox；busy 由 `pendingOperation` 统一门控；Dialog 有 Escape、
backdrop、focus trap 和 return focus；context menu 支持鼠标与键盘。证据见
`SubscriptionPage.tsx:430-446,451-482,496-515,559-564`。

### 4.4 Routing

当前只有 `PageHeader → PageScroll → placeholder-card`。不存在规则列表、筛选、状态分派
或 Dialog。Source Evidence：`src/App.tsx:41,213-224`。参考文档也没有给出足够的
Routing 字段结构，因此后续只能复用 Page/Dense List 视觉规则，不能推导 Clash/Mihomo
业务模型。

### 4.5 Connections

当前只有 `PageHeader → PageScroll → placeholder-card`。不存在连接表、工具栏、分页、
virtual list 或 loading/error/empty/render。Source Evidence：
`src/App.tsx:40,213-224`。

### 4.6 Logs

当前只有 `PageHeader → PageScroll → placeholder-card`。不存在日志 row、level filter、
内部滚动或数据状态。Source Evidence：`src/App.tsx:42,213-224`。参考中的日志
line-height 1.35 是局部实现，不可升级为 Veyra 全局 token。Reference Evidence：
`docs/ui/clash-verge-ui-analysis.md:167`。

### 4.7 Settings

```text
page-settings
├── PageHeader → title
└── PageScroll → placeholder-card → “此功能暂未开放”
```

当前没有 Settings page component、SettingSection、SettingRow、Switch、Settings Select 或
Settings Dialog。Source Evidence：`src/App.tsx:43,213-224`。

目标 composition 只能采用视觉模式：桌面两列 section、每个 section 为扫描式 rows、
短操作行内完成、复杂配置进入 Dialog。不能复制 Clash Verge 设置项、配置模型或命令。
Reference Evidence：`docs/ui/clash-verge-ui-analysis.md:333-370,407`。

## 5. Current Interaction Patterns

| Pattern | Current handling | Visual expression | Source Evidence |
|---|---|---|---|
| Hover | Sidebar、icon action、card/menu 各自 CSS | Sidebar 用 `--selection`；subscription card 改 primary border | `src/styles.css:152,426-450,500-505,686-713` |
| Active / Selected | `aria-current`、active subscription、checkbox set | Sidebar primary alpha；card 左侧 3px inset；checkbox 显式选择 | `src/App.tsx:169-175`; `SubscriptionPage.tsx:451,559,563`; `src/styles.css:153,503-505` |
| Disabled | 原生 `disabled`、`aria-disabled`、handler guard | Button opacity .55；icon/menu 另有 .4/.45/.5 | `src/styles.css:452-468,508-560,686-713`; `SubscriptionPage.tsx:451-464,559-564` |
| Loading | 文案、`—`、旋转 icon、button 文案、Suspense | 没有统一 skeleton/loader；局部动作在原位反馈 | `src/App.tsx:157-162,182-205`; `SubscriptionPage.tsx:461,465,484,492,515,564` |
| Expand / Collapse | 原生 `<details>` | 浏览器默认 disclosure；没有共享动画 | `src/App.tsx:201-208` |
| Modal / Dialog | 条件挂载；focus trap、Escape、backdrop；busy guard | backdrop + bordered surface + 独立 radius/shadow | `SubscriptionPage.tsx:438-446,474-516`; `DocumentEditor.tsx:93-115` |
| Dropdown / Select | 原生 `<select>`；操作使用 `SubscriptionMenu` | select 复用 field surface；menu 是 fixed popup | `SubscriptionPage.tsx:496-514,531-545`; `src/styles.css:660-661,686-713` |
| Tooltip | 原生 `title` | 浏览器默认视觉，无法统一 | `SubscriptionPage.tsx:450-454,543,564-567` |
| Toast / Notice | 两套本地 state + timer | 一左一下、一右一下；padding/radius/shadow/close 能力不同 | `src/App.tsx:60,66-70,225`; `SubscriptionPage.tsx:57,107-111,448`; `src/styles.css:335-386` |
| Context Menu | 阻止默认菜单，坐标定位；支持 Shift+F10/ContextMenu key | 152px dense menu，danger item 独立颜色 | `SubscriptionPage.tsx:430-436,478-482,531-545`; `src/styles.css:686-713` |

交互结构总体应保留。需要统一的是视觉表达，而不是将每一种状态改成同一种组件。

## 6. Visual Gap Analysis

Priority 定义：P0 破坏整体视觉语言；P1 明显视觉偏差；P2 局部一致性问题；P3 polish。
尚未实现的业务页面会标为“coverage gap”，不把缺业务功能伪装成视觉缺陷。

### 6.1 Cross-application gaps

| Area | Current Veyra | Target | Gap | Priority |
|---|---|---|---|---|
| Background hierarchy | dark 下 header/card/input/dialog 共用 `#1e1f27` | Shell/Sidebar、Page、Surface 三层独立语义 | 暗色层级反转或粘连，影响全应用 | P0 |
| Token Source of Truth | 颜色集中，spacing/radius/control 尺寸散落 | 所有后续页面从 Veyra token 取值 | 页面继续新增会扩大漂移 | P0 |
| Page composition boundary | `App` 内联多个 `<main>`，Subscription 自写同类壳 | 稳定 PageHeader/Content/full composition | 修改壳层触碰业务页面的风险较高 | P1 |
| Surface density | 首页/placeholder 20px padding、10px radius | 16px/8px 常规 surface；dense row 更紧凑 | Dashboard/SaaS 式大 Card 与目标桌面密度有偏差 | P1 |
| Notice system | 两套位置、padding、radius、关闭能力 | 一个外观规则和角落 stack | 跨页反馈明显不一致 | P1 |
| Overlay surfaces | Dialog/Menu/Toast 三套强 shadow | 同一 popup token；常规 surface 无 shadow | 局部浮层语言不一致 | P1 |
| Control height | toolbar/button 36px，Dialog fields 38px | standard 36px | 相邻页面与弹窗不齐 | P2 |
| IconButton | box 30/32，glyph 18/22 | standard 32/20；input trailing 30/18 作为 compact 例外 | 作用域未明示 | P2 |
| Radius | 4/6/8/10/12/50% 散落 | surface 8、control 8、popup 8；circle 显式 variant | 语义不清 | P2 |
| Divider | light/dark 8% | 参考结构 divider 6% | 边界略重 | P2 |
| Typography | Page 20/700 已对齐；section 16/600 | Section 16/700；其余冻结为 Veyra scale | section hierarchy 略弱 | P2 |
| Sidebar | fixed 200、44px item、24px icon | Veyra 固定 200；reference 展开高度 UNKNOWN | 视觉接近；只缺 token 化 | P3 |

### 6.2 Page gaps

| Page / Area | Current Veyra | Target | Gap | Priority |
|---|---|---|---|---|
| Overview | 两个全宽 section；220px chart + 大数字 tile | 6/12 响应式 grid；聚合图可 12 列，其余 6 列 | 单一流量 surface 形成事实 hero，信息密度偏低 | P1 |
| Overview states | 文案、`—`、button disabled、details | 局部 loading/error 视觉共用 token | 状态完整但表达未统一 | P2 |
| Proxies | placeholder | full page + state dispatcher + domain group/list | 页面尚未实现，是后续 coverage gap | P1 |
| Subscriptions grid | min 300px | dense min 260px | 同屏卡片数量偏少 | P2 |
| Subscription card | full border、radius 10、active inset | 8px、低强度边界、3px selected edge + primary title | 常态边界与 selected 表达偏重/不完整 | P2 |
| Subscription Dialog | radius 12、重 shadow、field 38 | popup radius 8、统一 overlay rule、field 36 | 与页面 surface 断层 | P1 |
| Routing | placeholder | Veyra 自有 toolbar/state/dense rows | 页面尚未实现；字段规格 UNKNOWN | P2 |
| Connections | placeholder | Veyra 自有 toolbar/state/dense table/list | 页面尚未实现；列规格 UNKNOWN | P2 |
| Logs | placeholder | Veyra 自有 filter/state/dense log list | 页面尚未实现；行规格 UNKNOWN | P2 |
| Settings | placeholder | 两列 SettingSection → SettingRow → Control/Dialog | 第一阶段视觉骨架缺失 | P0 |

### 6.3 明确不存在的问题

当前源码未发现 gradient、`backdrop-filter`、glass、blur surface。页面 header 是常规 58px
标题栏，不是 hero header；普通 Card 无外投影。真正需要处理的是 Overview 的事实 hero
重心、surface 密度和 popup shadow，不应新增“去渐变/去玻璃”工作。

Source Evidence：全局样式 `src/styles.css:1-727`、专用样式
`SidebarTraffic.css:1-64`、`DocumentEditor.css:1-12`；普通 surface 规则见
`src/styles.css:219-230,482-505`。

## 7. Preserve Existing Behavior

视觉迁移默认冻结以下边界：

- `activePage`、`setActivePage`、`hidden` 页面切换、页面保活和滚动复位。
- App runtime bootstrap、observation subscription、start/stop handlers、错误传播和 Tauri
  command/event contract。
- `SubscriptionPage` 的 query/IPC、validation、`pendingOperation` 串行化、activation 和
  observation readback。
- Subscription loading/error/empty/ready 互斥状态，以及 card Enter/Space、右键、
  selection mode、键盘 context menu。
- Dialog epoch identity、异步 stale-result 防护、focus return、Escape、Tab trap、busy
  close guard。
- `DocumentEditor` 的 lazy load、Monaco model 生命周期、dirty/busy、Ctrl+S、format
  undo boundary、fullscreen。
- `SidebarTraffic` 的数据映射、断点分段、ResizeObserver、帧率限制和 CSS variable
  theme redraw。
- Notice 的消息、role 与既有生命周期；checkbox 的多选/确认语义。

Source Evidence：`src/App.tsx:48-162`；`SubscriptionPage.tsx:37-446`；
`DocumentEditor.tsx:23-115`；`SidebarTraffic.tsx:17-225`。

如果后续需要抽组件边界，只允许将 DOM 与视觉 props 移到 presentational component；
上述 state/effect/handler 的所有者、调用顺序和行为不得顺带重写。

## 8. 结论

Veyra 已经有可工作的两栏 Shell、稳定的 PageHeader、浅深主题、系统字体/Twemoji、
无阴影常规 surface，以及交互完整的 Subscriptions 页面。后续不需要从零建立 UI
Framework，也不需要替换现有技术栈。

需要优先冻结的是：三层背景语义、几何/spacing/radius/control token、统一 Page
composition，以及 Settings 的最小视觉骨架。现有业务行为保持不变；没有组件仅因为
Clash Verge 使用不同的组件结构而被判为 `REPLACE`。
