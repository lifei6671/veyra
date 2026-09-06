# Clash Verge Rev UI 源码分析

> 目的：从本地 Clash Verge Rev 源码提取可供 Veyra 参考的 UI 语言、布局规则与组件组织方式。本文不评价或复述 Clash Verge Rev 的业务功能，也不授权复制其源码、品牌、Logo、文案或 Mihomo 数据模型。

## 0. 分析范围与证据口径

- 实际分析路径：`E:/wx_lifeilin/github.com/clash-verge-rev`（需求文本中的 `E:/wx/_lifeilin/...` 在本机不存在）。
- 源码身份：Git commit `ff4b4dd8229ebf3de630c141dff189b5ed5bb7fb`，提交时间 `2026-09-03 11:28:00 +0800`。分析时参考仓库除未跟踪的 `.codegraph/` 索引外无源码改动。
- Source Evidence 格式：`相对路径:行号 symbol`。行号对应上述源码身份；SCSS 等没有语言级 symbol 的文件以 selector（例如 `.base-page > header`）作为 symbol。
- `Source of Truth` 表示运行时实际生成视觉结果的本地定义；`局部约定` 表示只对所列组件成立；`UNKNOWN` 表示本地源码没有足够证据确定，本文不会用通用 UI 经验补齐。
- MUI 的 `sx` 数字通常由 MUI theme 解释。项目没有覆盖 `theme.spacing` 或 `theme.shape.borderRadius`，但这些默认值的实现不在本仓库；因此本文优先保留源码表达式（如 `spacing={1.5}`、`borderRadius: 2`），不擅自换算为像素。

## 1. UI 工程架构

### 1.1 总体组件关系

```text
main.tsx / initializeApp
├── 全局 SCSS：assets/styles/index.scss
├── ComposeContextProvider
│   ├── ThemeModeProvider
│   ├── LoadingCacheProvider
│   └── UpdateStateProvider
├── BaseErrorBoundary
├── SWRConfig
├── WindowProvider
├── AppDataProvider
└── RouterProvider(router)
    └── Layout
        ├── ThemeProvider(custom MUI theme)
        ├── NoticeManager + 全局 Dialog
        └── layout-content
            ├── Sidebar
            │   ├── Logo / UpdateButton
            │   ├── List → SortableItem → LayoutItem
            │   ├── Sidebar context Menu
            │   └── LayoutTraffic
            └── Content
                └── BaseErrorBoundary → Outlet → Page → BasePage
```

Source Evidence：

- `src/main.tsx:1-27` 导入全局样式、Router、Providers 与状态 Context；`src/main.tsx:43-66 initializeApp` 给应用套入 Context、错误边界、SWR、窗口与应用数据 Provider。
- `src/pages/_routers.tsx:6-17 router` 以 `Layout` 为根 route，将 `navItems` 映射为 children。
- `src/pages/_layout.tsx:214-263 Layout` 建立 MUI `ThemeProvider` 与外层 `Paper`；`src/pages/_layout.tsx:269-380 Layout` 组合左右栏、导航、全局 notice/dialog 和 `Outlet`。
- `src/components/base/base-page.tsx:15-52 BasePage` 是路由页面内部的共同页面骨架。

### 1.2 App Shell 与 Layout

`Layout` 同时承担应用壳、主题挂载、窗口装饰、侧栏和路由出口。它不是纯布局组件：还接入语言切换、通知、服务迁移弹窗、系统代理权限弹窗、菜单顺序和窗口装饰状态。这说明可借鉴的是壳层 composition，而不是把它作为可直接复制的通用组件。

- 外层：无 elevation 的 MUI `Paper`，背景取 `palette.background.paper`；Linux 分支增加 8px 外框圆角和 `100vw × 100vh`。证据：`src/pages/_layout.tsx:234-263 Layout`。
- 主体：`.layout-content` 是水平 flex；左侧是 sidebar，右侧绝对定位 `.the-content`。证据：`src/assets/styles/layout.scss:1-109 .layout/.layout-content`。
- 自定义标题栏与拖拽区按窗口装饰状态挂载；Windows/Linux 与 macOS 有不同排列和留白。证据：`src/pages/_layout.tsx:138-150 Layout.customTitlebar`；`src/assets/styles/layout.scss:197-263`。
- 页面内容始终经过 `BaseErrorBoundary → Outlet`，把单页渲染错误限制在内容区。证据：`src/pages/_layout.tsx:372-378 Layout`。

### 1.3 Sidebar

源码没有名为 `Sidebar` 的独立组件。Sidebar 由 `Layout` 中的 `.layout-content__left` 容器、MUI `List`、`SortableItem` 和 `LayoutItem` 共同构成。

- 导航元数据：`navItems` 同时携带 `path`、i18n label、单色/彩色两套 icon 和 Page component；Router 与 Sidebar 共用这份表，避免两套导航清单漂移。证据：`src/pages/_navigation.tsx:30-77 NavigationItem/navItems`；`src/pages/_routers.tsx:6-17 router`。
- Item composition：`Layout → SortableItem(render prop) → LayoutItem`；拖动能力通过 `sortable.ref/style/handleRef` 注入视觉项。证据：`src/pages/_layout.tsx:193-212 Layout.navMenuItems`；`src/components/layout/layout-item.tsx:14-93 LayoutItem`。
- 路由选中：`LayoutItem` 用 `useResolvedPath + useMatch` 计算 `selected`，用 `useNavigate` 导航；不是由 Sidebar 自己维护重复的 selected state。证据：`src/components/layout/layout-item.tsx:20-40 LayoutItem`。
- 折叠：布局类 `layout--nav-collapsed` 改为固定 72px，隐藏文本、流量与品牌字标，保留 52px 方形 icon item。证据：`src/assets/styles/layout.scss:265-360`。
- 上下文菜单：右键 sidebar 以鼠标坐标锚定 MUI `Menu`，提供折叠、锁定排序、恢复默认顺序；恢复项可 disabled。证据：`src/pages/_layout.tsx:105-136,320-365 Layout`。

### 1.4 Router 与 Page

- Router 是单层嵌套路由：根 `/` 渲染 `Layout`，所有主页面来自 `navItems.map`。没有在页面之间复制壳层。证据：`src/pages/_routers.tsx:6-17 router`。
- `BasePage` 暴露 `title`、`header`、`contentStyle`、`full`、`children`。`title/header` 分离后，页面把全局操作放在标题栏右侧；`full` 让数据密集页自行管理滚动。证据：`src/components/base/base-page.tsx:7-16 Props/BasePage`。
- `BasePage` 不是抽象的业务 Page：它只负责 header、背景层、滚动容器和错误边界。证据：`src/components/base/base-page.tsx:21-50 BasePage`。
- 常规卡片页使用默认内容 padding；Proxies、Profiles 使用 `full` 和 `height: 100%`，把高度/滚动交给虚拟列表或页面内部区域。证据：`src/pages/home.tsx:322-364 HomePage`；`src/pages/settings.tsx:41-117 SettingPage`；`src/pages/proxies.tsx:139-202 ProxyPage`；`src/pages/profiles.tsx:714-1001 ProfilePage`。

### 1.5 Shared Components 与目录职责

```text
src/components/
├── base/        页面、Dialog、Empty、Loading、Switch、输入、Tooltip、虚拟列表
├── layout/      导航项、流量、窗口控制、全局通知、壳层 Dialog
├── home/        Overview 专用卡片与图表
├── proxy/       代理组、工具栏、列表项、虚拟化渲染
├── profile/     订阅卡片、编辑器、更多配置、上下文菜单
├── setting/     设置分组，以及 mods/ 下的设置 Dialog/控件
├── connection/  连接页专用视图
├── rule/        规则页专用视图
├── log/         日志页专用视图
├── shared/      跨业务页但带应用语义的组合组件
└── test/        测试页专用视图
```

组织规律是两层复用：`base/` 提供低业务耦合封装；领域目录提供页面专用复合组件。并非所有 MUI 控件都再封装：Button、Grid、List、Select、Menu、Dialog、Tooltip 仍常直接使用。

Source Evidence：

- `src/components/base/index.ts` 汇出 base primitives；代表性 symbols 为 `BasePage`、`BaseDialog`、`BaseEmpty`、`BaseLoading`、`Switch`、`BaseStyledSelect`、`BaseStyledTextField`、`TooltipIcon`、`VirtualList`、`StickyVirtualList`。
- `src/components/base/base-dialog.tsx:12-76 Props/BaseDialog`；`src/components/base/base-switch.tsx:4-58 Switch`；`src/components/base/base-tooltip-icon.tsx:9-23 Props/TooltipIcon`。
- `src/components/setting/mods/setting-comp.tsx:15-90 ItemProps/SettingItem/SettingList` 是设置域内的复用模式，没有放入 `base/`。
- `src/components/home/enhanced-card.tsx:5-123 EnhancedCardProps/EnhancedCard` 是首页卡片模式，没有被提升为全局 Card abstraction。

### 1.6 Store / State

项目没有单一 Redux/Zustand store。状态按寿命和所有权分层：

```text
后端/Tauri query state
└── query-client cache + useQuery hooks
    ├── useVerge / useProfiles / useClash
    └── AppDataProvider → typed React Contexts → page/components

跨树纯 UI state
└── createContextState
    ├── ThemeMode
    ├── LoadingCache
    └── UpdateState

页面/组件临时 state
└── useState/useReducer/useRef

需要跨重启保留的局部 UI 偏好
└── localStorage（如 proxy head、链式模式、排序）
```

- `useVerge` 从 preload/query cache 读取配置，提供本地 optimistic cache 写入与后端 patch 后 refetch。证据：`src/hooks/use-verge.ts:7-54 useVerge`。
- `useProfiles` 用 query key `getProfiles` 管理数据、刷新与 patch 后 cache 协调。证据：`src/hooks/use-profiles.ts:7-58 useProfiles`。
- `AppDataProvider` 汇聚 proxy/config/system/rules 等查询，并通过多个 typed Context 分发，避免所有页面重复订阅。证据：`src/providers/app-data-provider.tsx:53-86 AppDataProvider`；`src/providers/app-data-context.ts:6-57 Context types/contexts`。
- 纯 UI Context 来自 `createContextState`。证据：`src/services/states.ts:1-22 ThemeModeProvider/LoadingCacheProvider/UpdateStateProvider`。
- 页面局部状态例：`HomePage.settingsOpen`、`ProxyPage.isChainMode`、`ProfilePage.batchMode`。证据：`src/pages/home.tsx:232-254 HomePage`；`src/pages/proxies.tsx:30-46 ProxyPage`；`src/pages/profiles.tsx:618-659 ProfilePage`。

### 1.7 Theme、CSS / Style System、Component Library

主题流如下：

```text
defaultTheme/defaultDarkTheme + user theme_setting + theme_mode
→ useCustomTheme
→ createTheme(MUI palette/typography/shadows)
→ 写入运行时 CSS variables + 注入 global styles/user CSS
→ Layout 中 ThemeProvider
→ MUI sx/styled + SCSS classes
```

- 默认 palette/font：`src/pages/_theme.tsx:5-32 defaultTheme/defaultDarkTheme`。
- theme 生成与用户覆盖：`src/pages/_layout/hooks/use-custom-theme.ts:144-200 useCustomTheme`。
- 运行时 CSS variables：`src/pages/_layout/hooks/use-custom-theme.ts:202-243 useCustomTheme`。
- 注入滚动条、body 背景、Paper/Dialog 和无阴影规则：`src/pages/_layout/hooks/use-custom-theme.ts:261-309 useCustomTheme.globalStyles`。
- SCSS 入口：`src/assets/styles/index.scss:1-3` 依次引入 layout/page/font；布局和页面骨架分别在 `layout.scss`、`page.scss`。
- 局部样式同时使用 MUI `sx`、Emotion `styled` 和少量 inline style；不存在单一 CSS methodology。
- 组件库主体为 MUI；Emotion 提供 styled engine，React Router 提供路由，虚拟列表与拖放分别使用 TanStack Virtual 与 dnd-kit。证据：`package.json:41-48,76,80`。

## 2. Visual Tokens

### 2.1 Token 表

| 项目 | 源码结论 | 层级 / Source Evidence |
|---|---|---|
| Window background | 稳态 body：浅 `#ECECEC`，暗 `#2E303D`；可叠加用户背景图。主题未就绪的 `#fff/#181a1b` 仅是 loading fallback。 | 运行时 SoT：`src/pages/_layout/hooks/use-custom-theme.ts:202-243,275-290 useCustomTheme`；fallback：`src/pages/_layout.tsx:176-189 Layout` |
| App Shell background | 外层 `Paper` 使用 `palette.background.paper`，默认浅 `#F5F5F5`、暗 `#2E303D`。 | `src/pages/_layout.tsx:234-263 Layout`；`src/pages/_theme.tsx:5-32` |
| Sidebar background | 无独立声明，继承 App Shell `Paper`；因此默认同上。 | `src/pages/_layout.tsx:234-270 Layout`；`src/assets/styles/layout.scss:14-29` |
| Sidebar width | 展开态 computed width `UNKNOWN`：只有 `flex: 1 0 200px` nominal basis，不是固定 200px；折叠态固定 72px。 | `src/assets/styles/layout.scss:14-28,265-277` |
| Content background | container：浅 `#fff`、暗 `#1e1f27`；滚动 section：浅 `#ECECEC`、暗 `#1e1f27`；Home/Settings card：浅 `#fff`、暗 `#282a36`。 | `src/components/base/base-page.tsx:35-47 BasePage`；`src/pages/settings.tsx:77-114 SettingPage`；`src/components/home/enhanced-card.tsx:42-50 EnhancedCard` |
| Page padding | header `0 20px`；默认滚动区纵向 10px，内容左右各 10px；`full` 为 0。Home 另有 React inline `padding: 2`，浏览器解释为 2px。 | `src/assets/styles/page.scss:6-17 .base-page > header/.base-container > section`；`src/pages/home.tsx:322-350 HomePage` |
| Section spacing | 无全局 token。Home/Settings Grid 都用 `spacing={1.5}`；Settings 同列 group 用 `marginBottom:1.5`。像素值由外部 MUI spacing 决定。 | `src/pages/home.tsx:350-354`；`src/pages/settings.tsx:77-114` |
| Component spacing | 无统一 token。`EnhancedCard` header `px:2, py:1`，icon/title `mr:1.5`，content `p:2`；Settings row `pt/pb:5px`。 | `src/components/home/enhanced-card.tsx:53-115 EnhancedCard`；`src/components/setting/mods/setting-comp.tsx:63-67 SettingItem` |
| Typography scale | 完整比例尺 `UNKNOWN`。稳定局部层级：Page title 20px；Setting section 16px；Setting row 14px；Card title 18px。其余多用 MUI variants。 | `src/components/base/base-page.tsx:23-30`；`src/components/setting/mods/setting-comp.tsx:32-37,70-87`；`src/components/home/enhanced-card.tsx:89-101` |
| Font weight | 全局 scale `UNKNOWN`。Page/Setting section 为 700；Card title 为 `medium`；Sidebar label 700。 | 同上；`src/components/layout/layout-item.tsx:49-52 LayoutItem` |
| Line height | 全局 `UNKNOWN`；局部组件单独指定，例如日志项 1.35、rule item body2 为 2。 | `src/components/log/log-item.tsx:6-13 Item`；`src/components/rule/rule-item.tsx:37-55 RuleItem` |
| Font family | 系统字体栈；Windows 追加 `twemoji mozilla`。用户字体可放在默认栈之前。Twemoji 资产通过 `@font-face` 加载。 | SoT：`src/pages/_theme.tsx:15-17 defaultTheme.font_family`；`src/pages/_layout/hooks/use-custom-theme.ts:172-176`；`src/assets/styles/font.scss:1-4` |
| Border color | Shell/page divider：浅 `rgba(0,0,0,.06)`、暗 `rgba(255,255,255,.06)`；window Paper border color：浅 `#ccc`、暗 `#1E1E1E`。MUI `palette.divider` 未覆盖，不能与 CSS variable 自动等同。 | `src/pages/_layout/hooks/use-custom-theme.ts:207-229`；`src/assets/styles/layout.scss:28`；`src/assets/styles/page.scss:16` |
| Border width | 无统一 token；主要结构 divider 反复为 1px。 | `src/assets/styles/layout.scss:28,208,241`；`src/assets/styles/page.scss:16`；`src/components/home/enhanced-card.tsx:60-62` |
| Radius | 全局有效 token `UNKNOWN`。`:root --border-radius:8px` 的活动引用不存在；Card/Settings 常见 `borderRadius:2`，折叠 SidebarItem 明确 12px，Switch track 明确 13px。 | `src/assets/styles/index.scss:18-26`；`src/assets/styles/page.scss:22-24`（引用被注释）；`src/pages/settings.tsx:77-114`；`src/assets/styles/layout.scss:265-269,334-345` |
| Shadow | 应用有效规则为无阴影：MUI 25 级 shadows 全为 `none`，全局 `box-shadow:none!important`。拖拽态是明确例外：`0 6px 18px rgba(0,0,0,.32)!important`。 | `src/pages/_layout/hooks/use-custom-theme.ts:150-177,302-306`；`src/assets/styles/index.scss:63-65` |
| Control height | 全局 `UNKNOWN`。精确局部值：Switch 26px；BaseStyledSelect 33.375px；Profile import bar 36px。 | `src/components/base/base-switch.tsx:10-18`；`src/components/base/base-styled-select.tsx:3-15`；`src/pages/profiles.tsx:827-897` |
| Button size | `UNKNOWN`。没有项目级 Button override；页面混用 MUI `small/medium`，exact px 由外部组件库实现。 | `src/pages/home.tsx:327-346`；`src/pages/settings.tsx:45-73` |
| Input size | 全局 `UNKNOWN`。`BaseStyledTextField` 为 `fullWidth/small/outlined`，input `py:.65, px:1.25`，未固定总高度；BaseStyledSelect 固定 120×33.375。 | `src/components/base/base-styled-text-field.tsx:4-24`；`src/components/base/base-styled-select.tsx:3-19` |
| Icon size | 全局 `UNKNOWN`。明确局部值：logo 36×36、card icon 容器 38×38、search action icon 24×24；Sidebar icon 自身未定尺寸。 | `src/pages/_layout.tsx:280-290`；`src/components/home/enhanced-card.tsx:73-88`；`src/components/base/base-search-box.tsx:122-129`；`src/components/layout/layout-item.tsx:71-84` |
| Hover state | 无全局项目 token。常见做法是 `action.hover` 或 primary alpha；具体值按组件列于交互章节。普通 MUI hover 色 `UNKNOWN`。 | `src/components/home/home-profile-card.tsx:252-280 EmptyProfile`；`src/components/layout/scroll-top-button.tsx:15-28` |
| Active state | 页面分段按钮使用 `contained`（active）/`outlined`（inactive）；Sidebar selected 浅 primary 15%、暗 35%。 | `src/pages/proxies.tsx:166-193`；`src/components/layout/layout-item.tsx:54-64` |
| Disabled state | 通用色/opacity `UNKNOWN`，多数交给 MUI。共享 Switch 有精确 disabled 规则；部分业务组合另设 opacity。 | `src/components/base/base-switch.tsx:19-43`；`src/components/shared/proxy-control-switches.tsx:91-103` |

### 2.2 Palette Source of Truth

| Semantic color | Light | Dark | Source Evidence |
|---|---:|---:|---|
| primary | `#007AFF` | `#0A84FF` | `src/pages/_theme.tsx:5-6,21-24` |
| secondary | `#FC9B76` | `#FF9F0A` | `src/pages/_theme.tsx:7,24` |
| primary text | `#000000` | `#FFFFFF` | `src/pages/_theme.tsx:8,25` |
| secondary text | `#3C3C4399` | `#EBEBF599` | `src/pages/_theme.tsx:9,27` |
| info | `#007AFF` | `#0A84FF` | `src/pages/_theme.tsx:10,28` |
| error | `#FF3B30` | `#FF453A` | `src/pages/_theme.tsx:11,29` |
| warning | `#FF9500` | `#FF9F0A` | `src/pages/_theme.tsx:12,30` |
| success | `#06943D` | `#30D158` | `src/pages/_theme.tsx:13,31` |
| theme paper/default | `#F5F5F5` | `#2E303D` | `src/pages/_theme.tsx:14,26`；`src/pages/_layout/hooks/use-custom-theme.ts:166-169` |

注意：`theme_setting` 允许覆盖主色、语义色、文字和字体，但 `palette.background.paper/default` 仍取当前默认 light/dark theme 的 `background_color`，不是用户的 background field。证据：`src/pages/_layout/hooks/use-custom-theme.ts:144-177 useCustomTheme`。

### 2.3 Shared Switch 精确尺寸

`Switch` 是少数具有完整本地视觉规格的 control：

- 外框 42×26，padding 0；thumb 22×22；track radius 13；checked 位移 16px。
- checked track 使用 `primary.main`；unchecked 为浅 `#BBBBBB`、暗 `#39393D`。
- checked+disabled track opacity 0.5；disabled thumb 取 MUI grey 100/600；disabled unchecked track opacity 浅 0.7、暗 0.3。

Source Evidence：`src/components/base/base-switch.tsx:4-58 Switch`。

## 3. Component Patterns

### 3.1 模式矩阵

| 视觉模式 | 实际实现与职责 | Composition / Props | Style Source | 主要使用位置 |
|---|---|---|---|---|
| AppShell | `Layout`：窗口壳、主题、全局 overlay/dialog、两栏布局和 route outlet。 | 无公开 props；从 hooks 读取主题、窗口、导航偏好。 | `layout.scss` + MUI `Paper/sx` + inline。 | 根 route。证据：`src/pages/_layout.tsx:57-386 Layout` |
| Sidebar | 无独立组件；由 Layout 左栏、List、SortableItem、LayoutItem 组合。 | `navItems` 驱动；折叠/排序状态来自 `useVerge/useNavMenuOrder`。 | `layout.scss` + `LayoutItem.sx`。 | App Shell。证据：`src/pages/_layout.tsx:193-212,269-370` |
| SidebarItem | `LayoutItem`：路由匹配、导航、icon mode、selected visual；可接 sortable render props。 | `to/children/icon[2]/sortable?`。 | MUI `ListItem/Button/Icon/Text` + `sx`。 | Sidebar。证据：`src/components/layout/layout-item.tsx:14-93` |
| PageHeader | 没有单独组件；`BasePage` 的 `title/header` 两个 slot 共同形成。 | title 为 ReactNode；header 为任意操作组。 | `page.scss` + title `Typography.sx`。 | 所有主页面。证据：`src/components/base/base-page.tsx:7-47` |
| Section | 没有全局 `Section` primitive。Settings 使用 `SettingList`，Home 使用 `EnhancedCard`，数据页使用 group header/list。 | 随页面族变化。 | 各领域组件。 | Settings/Home/Proxy。 |
| Card | 没有全局 Card。首页复用 `EnhancedCard`；Settings 用 Box surface；Profile 用 `ProfileBox`。 | EnhancedCard：title/icon/action/children/iconColor/minHeight/noContentPadding。 | `EnhancedCard.sx`；领域 style。 | Overview。证据：`src/components/home/enhanced-card.tsx:5-123` |
| SettingSection | `SettingList`：透明 List + 非 sticky ListSubheader + children。 | `title/children`。 | `setting-comp.tsx` 内 `sx`。 | 四组 Settings。证据：`src/components/setting/mods/setting-comp.tsx:70-90` |
| SettingRow | `SettingItem`：label/extra/secondary/children 或整行 onClick。异步 onClick 时 disabled 并用 progress 取代 chevron。 | `label/extra?/children?/secondary?/onClick?`。 | MUI ListItem/ListItemButton。 | 所有 Settings group。证据：`src/components/setting/mods/setting-comp.tsx:15-68` |
| List | 简单列表直接用 MUI `List`；大数据集封装 `VirtualList/StickyVirtualList`。 | 虚拟列表接收 items、estimateSize、renderItem 等。 | MUI + TanStack Virtual。 | Settings、Proxies、Rules/Connections。证据：`src/components/base/virtual-list.tsx:11-33`；`sticky-virtual-list.tsx:35-59` |
| ListItem | 无全局 wrapper；导航、设置、代理、订阅各自封装不同语义项。 | 领域 props。 | MUI selected/disabled + 局部 `sx/styled`。 | 多页面。 |
| Switch | 统一视觉 wrapper，透传 MUI `SwitchProps`。 | 所有 MUI Switch props。 | Emotion `styled`。 | Settings 与跨页代理控制。证据：`src/components/base/base-switch.tsx:4-58` |
| Select | `BaseStyledSelect` 只在部分数据页使用；Settings 仍直接用 MUI Select 并设置宽度/内 padding。 | 透传 `SelectProps<string>`。 | styled + per-use `sx`。 | Connections/Logs；Settings 直接用 MUI。证据：`src/components/base/base-styled-select.tsx:3-19`；`setting-verge-basic.tsx:91-106` |
| Input | `BaseStyledTextField` 提供常规 filter/import 外观；`BaseSearchBox` 组合搜索状态按钮。 | TextField props；SearchBox 支持 controlled/uncontrolled search options。 | styled + MUI `sx`。 | Profiles、Logs/Connections/Proxy tools。证据：`base-styled-text-field.tsx:4-24`；`base-search-box.tsx:20-267` |
| Button | 主要直接用 MUI Button/IconButton/ButtonGroup，没有统一 wrapper。active 常用 contained/outlined。 | MUI props。 | MUI theme + local `sx`。 | 所有页面 header/dialog。 |
| Tabs | 未发现全局/MUI Tabs abstraction。代理模式用 ButtonGroup；Home 的网络卡使用局部 `TabButton`。 | 局部 active/onClick/disabled。 | 页面/组件内 `sx`。 | Proxies header；`ProxyTunCard`。证据：`src/pages/proxies.tsx:166-193`；`src/components/home/proxy-tun-card.tsx:37-102 TabButton` |
| Dialog | `BaseDialog` 提供标准 title/content/actions、可隐藏 footer、禁用按钮、OK loading；复杂场景也直接用 MUI Dialog。 | `open/title/buttons/disable*/loading/on*`；`DialogRef.open/close`。 | MUI。 | Settings viewers、删除确认；Home/Provider 直接 MUI。证据：`base-dialog.tsx:12-76` |
| Dropdown | 选择输入用 Select/MenuItem；上下文操作用 Menu/MenuItem，以 anchorEl 或 anchorPosition 定位。无额外全局 wrapper。 | MUI props。 | MUI + local `sx`。 | Settings、Sidebar、Profile cards。 |
| Tooltip | `TooltipIcon` = top Tooltip + small IconButton + 0.75 opacity icon；页面也直接用 MUI Tooltip 或 HTML title。 | `title/icon` + IconButtonProps。 | MUI + inline icon style。 | Settings、Proxy、Home header。证据：`base-tooltip-icon.tsx:9-23` |
| Status Indicator | 无单一 status primitive。空/加载分别用 BaseEmpty/BaseLoading；进度用 Circular/LinearProgress；全局结果用 NoticeManager/Alert。 | 随状态组件而变。 | MUI + local styled。 | 全局与数据页。证据：`base-empty.tsx:8-40`；`base-loading.tsx:3-48`；`notice-manager.tsx:154-248` |

### 3.2 页面如何组合这些模式

- 卡片页：`BasePage → Grid → domain card → EnhancedCard → header/content`。Page 控制跨卡片布局；Card 控制自身 header/content，不反向决定 Grid span。证据：`src/pages/home.tsx:256-320 HomePage.renderCard/criticalCards/nonCriticalCards`。
- 设置页：`BasePage → Grid column → surface Box → SettingList → SettingItem → control/viewer trigger`。复杂表单退出主页面，放入由 ref 打开的 Viewer/Dialog。证据：`src/pages/settings.tsx:77-116`；`src/components/setting/setting-verge-basic.tsx:52-120`。
- 数据页：`BasePage(full) → toolbar/header → state dispatcher → virtual list → semantic item`。Loading/Empty/Render 在进入大列表前互斥处理。证据：`src/pages/proxies.tsx:139-202`；`src/components/proxy/proxy-groups.tsx:600-627 ProxyGroups`。

## 4. Page Composition

### 4.1 Overview（源码：HomePage）

```text
HomePage
└── BasePage
    ├── title
    ├── header → Tooltip → IconButton × 3
    ├── Grid(columns xs/sm=6, md=12; spacing=1.5)
    │   ├── Grid(6) → HomeProfileCard → EnhancedCard
    │   ├── Grid(6) → CurrentProxyCard
    │   ├── Grid(6) → NetworkSettingsCard → EnhancedCard → ProxyTunCard
    │   ├── Grid(6) → ClashModeEnhancedCard → EnhancedCard → ClashModeCard
    │   ├── Grid(12) → EnhancedCard → EnhancedTrafficStats
    │   └── Grid(6) → Suspense → optional card / Skeleton(height=200)
    └── HomeSettingsDialog（按需挂载）
```

布局规律：

- `renderCard` 集中处理显示开关、Grid wrapper 和 span；卡片不自行决定页面跨度。
- 首屏关键卡直接加载；非关键卡使用 lazy/Suspense 并保留固定高度 skeleton，降低结构跳变。
- 页面 header 放全局操作，card header 只放局部 action。
- 弹窗在需要时挂载，内部 checkbox 使用草稿状态，Save 时一次提交。

Source Evidence：`src/pages/home.tsx:114-230 HomeSettingsDialog`；`src/pages/home.tsx:232-320 HomePage/renderCard`；`src/pages/home.tsx:322-395 HomePage JSX/局部 Card`；`src/components/home/enhanced-card.tsx:42-120 EnhancedCard`。

### 4.2 Proxies

```text
ProxyPage
└── BasePage(full)
    ├── title → 条件式 warning TooltipIcon
    ├── header
    │   ├── ProviderButton → Dialog
    │   ├── ButtonGroup → mode Button × 3
    │   └── chain-mode Button
    └── ProxyGroups
        ├── direct → BaseEmpty
        ├── loading → BaseLoading
        ├── empty → ProxyEmptyState
        └── render
            ├── ChainProxyGroups → ProxyGroupsChain → virtual list
            └── NormalProxyGroups
                └── StickyVirtualList
                    └── ProxyRender
                        ├── group header + expand control
                        ├── ProxyHead tools
                        ├── ProxyItem rows
                        ├── group empty
                        └── ProxyItemMini grid
```

布局规律：

- `full` 页面不使用外部 section padding/scroll，由列表内部负责 100% 高度、sticky header 与虚拟化。
- 页头用 ButtonGroup 表达互斥模式；active 是 `contained`，其余是 `outlined`，没有额外 Tabs abstraction。
- `ProxyGroups` 先压缩为 direct/loading/empty/render 四态，再进入普通/链式渲染，避免在每个列表项重复状态判断。
- 列表以“group header → optional tools → rows”组织，不套通用 Card。

Source Evidence：`src/pages/proxies.tsx:139-202 ProxyPage`；`src/components/proxy/proxy-groups.tsx:329-352 ChainProxyGroups`；`src/components/proxy/proxy-groups.tsx:355-440,560-627 NormalProxyGroups/ProxyGroups`；`src/components/proxy/proxy-render.tsx:45-290 ProxyRender`。

### 4.3 Subscriptions（源码：ProfilePage）

```text
ProfilePage
└── BasePage(full)
    ├── header
    │   ├── normal actions
    │   └── batch actions + selection summary
    ├── Stack(height=36) import toolbar
    │   ├── BaseStyledTextField + paste/clear adornment
    │   ├── import Button
    │   └── new Button
    ├── scroll region
    │   ├── DragDropProvider
    │   │   └── CSS Grid(auto-fill, minmax(260px, 1fr))
    │   │       └── ProfileItem × N
    │   ├── Divider
    │   └── Grid → ProfileMore × 2
    ├── ProfileViewer
    └── ConfigViewer
```

布局规律：

- 工具条高度固定，剩余区域滚动；卡片网格按 260px 最小宽度自动换列。
- batch mode 只替换 header actions 并在 card 上增加 selection control，不更换数据/卡片结构。
- 页面统一持有 DragDropProvider；单卡只接收 id/index/handle 等排序接口。
- viewer/dialog 邻接挂载在页面尾部，不通过新 route 承载编辑流程。

Source Evidence：`src/pages/profiles.tsx:714-825 ProfilePage header`；`src/pages/profiles.tsx:827-897 import toolbar`；`src/pages/profiles.tsx:899-988 cards/ProfileMore`；`src/pages/profiles.tsx:991-1001 viewers`；`src/components/profile/profile-item.tsx:57-73 ProfileItemProps,622-959 ProfileItemBase`。

### 4.4 Settings

```text
SettingPage
└── BasePage
    ├── header → ButtonGroup → IconButton × 3
    └── Grid(columns xs/sm=6, md=12; spacing=1.5)
        ├── Grid(6)
        │   ├── Box → SettingSystem → SettingList → SettingItem × N
        │   └── Box → SettingClash → SettingList → SettingItem × N
        └── Grid(6)
            ├── Box → SettingVergeBasic
            │   ├── Viewer/Dialog refs
            │   └── SettingList → SettingItem × N
            └── Box → SettingVergeAdvanced
                ├── Viewer/Dialog refs
                └── SettingList → SettingItem × N
```

典型设置行：

```text
SettingItem
├── ListItemText
│   ├── primary → Label + optional TooltipIcon
│   └── secondary → Description
├── direct-control row → children(Switch/Select/Input)
└── navigation row → ChevronRight | CircularProgress
```

布局规律：

- 桌面宽度下两列，每列垂直堆叠两组；Grid gap 与组间 margin 使用同一 `1.5` MUI spacing 表达式。
- SettingList 只处理 section title；SettingItem 统一 label/description/control 或 clickable row。
- clickable async row 在执行期间禁用整行，并用 20px progress 取代 chevron。
- 复杂设置预挂载为 Viewer，通过 `DialogRef.open()` 打开；主页面保持扫描式列表密度。

Source Evidence：`src/pages/settings.tsx:41-117 SettingPage`；`src/components/setting/mods/setting-comp.tsx:15-90 SettingItem/SettingList`；`src/components/setting/setting-system.tsx:17-98`；`src/components/setting/setting-verge-basic.tsx:52-120`；`src/components/setting/setting-verge-advanced.tsx:34-103`。

## 5. Interaction Patterns

| Pattern | 状态处理 | 视觉变化 | Source Evidence |
|---|---|---|---|
| Hover：Sidebar | selected hover 被固定为 selected 背景；普通 hover 无本地 override。 | selected 浅 primary 15%、暗 35%；普通 hover exact color `UNKNOWN`（MUI）。 | `src/components/layout/layout-item.tsx:54-64 LayoutItem` |
| Hover：Proxy row | CSS descendant selectors 随 hover 切换 check/delay/icon 显隐。 | selected icon 隐藏；根据已有 delay 显示 delay 或 test action；局部 action hover 为 primary alpha。 | `src/components/proxy/proxy-item.tsx:73-98,135-189 ProxyItem` |
| Hover：Provider row | `&:hover` 改 surface/border。 | 浅 primary 10%、暗 primary 20%，border primary 30%。 | `src/components/rule/provider-button.tsx:161-186 ProviderButton` |
| Active：segmented control | 比较当前 state 与 item value。 | active=`contained`，inactive=`outlined`；chain mode 同时切换 filled/outline icon。 | `src/pages/proxies.tsx:166-193 ProxyPage` |
| Selected：Sidebar | Router match 直接绑定 MUI selected。 | 浅/暗 primary alpha 背景；selected hover不再改变；文字强制浅 `#1f1f1f`/暗 `#fff`。 | `src/components/layout/layout-item.tsx:25-40,54-64` |
| Selected：Proxy/Profile | state 绑定 item selected / `aria-selected`。 | 左侧 3px primary border、元素左移与宽度补偿；Profile title 变 primary。 | `src/components/proxy/proxy-item.tsx:68-98`；`src/components/profile/profile-box.tsx:3-55 ProfileBox` |
| Disabled | 多数直接传 MUI disabled；业务 handler 同时避免执行。 | 通用 disabled 色 `UNKNOWN`；Switch 有专属 opacity/grey；个别 provider disabled 保留 warning。 | `src/components/base/base-switch.tsx:19-43`；`src/components/proxy/proxy-item.tsx:54-72`；`src/pages/profiles.tsx:804-812` |
| Loading：page/card | 按作用域采用 Skeleton、overlay、rotating icon、CircularProgress。 | Home lazy card 为 200px rectangle Skeleton；Profile activating 为 card overlay + 20px progress。 | `src/pages/home.tsx:282-318`；`src/components/profile/profile-item.tsx:641-663,718-740` |
| Loading：通用 inline | `BaseLoading` 或 clickable SettingItem local state。 | 三个 6px 圆点 0.7s opacity/scale 动画；SettingItem 用 20px progress 取代 chevron。 | `src/components/base/base-loading.tsx:3-48`；`src/components/setting/mods/setting-comp.tsx:39-60` |
| Expand / Collapse | Proxy group 的 `headState.open` 更新 render list。 | 箭头 `ExpandLess/ExpandMore`；展开后插入 tools/items。该路径未使用统一 Collapse 动画，动画规则 `UNKNOWN`。 | `src/components/proxy/proxy-render.tsx:87-108,183-216,225-286` |
| Modal / Dialog | local boolean 条件挂载，或 viewer 通过 imperative `DialogRef` 控制。 | 通常 title/content/actions 三段；BaseDialog 支持 footer/button disabled 和 OK loading。 | `src/components/base/base-dialog.tsx:12-76`；`src/pages/home.tsx:114-230,356-363` |
| Dropdown | Select/MenuItem 用于选择；Menu/MenuItem 用于操作菜单。 | Settings 常用 `size=small` 与局部宽度/内 padding；popup hover/selected exact color `UNKNOWN`（MUI）。 | `src/components/setting/setting-verge-basic.tsx:91-106`；`src/pages/_layout.tsx:326-365` |
| Tooltip | 共享 TooltipIcon、直接 MUI Tooltip、原生 title 三种并存。 | TooltipIcon 固定 top、small icon button、icon opacity .75；Home Tooltip 开 arrow。 | `src/components/base/base-tooltip-icon.tsx:9-23`；`src/pages/home.tsx:327-346` |
| Toast / Notice | `showNotice` 写外部 store；NoticeManager 用 `useSyncExternalStore` 订阅。 | fixed 角落纵向堆叠，20px edge、10px gap、max 360px；filled Alert；可关闭。默认 success/info/error 为 3s/5s/8s。 | `src/services/notice-service.ts:7-23,54-58,505-531`；`src/components/layout/notice-manager.tsx:154-248` |
| Context Menu | 组件阻止默认菜单，用 `clientX/clientY` 作为 Menu anchorPosition；并非全页面强制同一 menu。 | Sidebar/Profile 使用 dense MenuItem；Toast 右键直接复制文本，不显示菜单。 | `src/pages/_layout.tsx:105-136,326-365`；`src/components/profile/profile-item.tsx:634-639,826-862`；`notice-manager.tsx:217-225` |

交互层级的共同规律：局部动作在原位置反馈（button loading、icon rotation、card overlay），跨组件结果进入全局 Notice；Dialog 保持在当前页面上下文中，不以页面跳转代替临时编辑。证据：`src/components/setting/mods/setting-comp.tsx:39-60`；`src/components/profile/profile-item.tsx:641-740`；`src/services/notice-service.ts:505-531`。

## 6. Visual Consistency Rules

以下只总结源码中反复出现、可追溯的规则：

1. **页面壳一致**：58px header、横向 20px padding、底部 1px divider；默认内容纵向 10px、左右 10px，full 页面清零。证据：`src/assets/styles/page.scss:1-48`；`src/components/base/base-page.tsx:21-49 BasePage`。
2. **背景靠层级区分，不靠阴影**：Shell/Sidebar paper → page section → white/`#282a36` card；MUI shadows 与全局 box-shadow 被禁用。证据：`src/pages/_layout.tsx:234-263`；`BasePage:35-47`；`settings.tsx:77-114`；`use-custom-theme.ts:171,302-306`。
3. **边界主要用低透明 1px divider**：Sidebar 右边界、Page header 下边界一致使用 `--divider-color`。但 EnhancedCard 使用 MUI `palette.divider`，源码没有证明二者相同。证据：`layout.scss:28`；`page.scss:16`；`enhanced-card.tsx:60-62`。
4. **8px 形态高频但不是有效全局 token**：Settings/Home/Profile surfaces 反复使用 `borderRadius:2` 或 8px；`:root --border-radius:8px` 当前没有活动引用，不应称为统一 token。证据：`index.scss:18-26`；`page.scss:22-24`；`settings.tsx:77-114`。
5. **Home/Settings 共用响应式 6/12 栅格和 `spacing=1.5`**：普通块半宽，强调/聚合块可全宽。证据：`home.tsx:256-320,350-354`；`settings.tsx:77-116`。
6. **标题层级是局部稳定、非全局 scale**：Page 20/700、Setting section 16/700、Setting row 14、Card title 18/medium；正文继续使用 MUI variants。证据：`base-page.tsx:23-30`；`setting-comp.tsx:32-37,70-87`；`enhanced-card.tsx:89-101`。
7. **Sidebar 折叠态比展开态更严格**：折叠 72px，item 52×52、radius 12px；展开态只有 200px flex-basis，item 高度由内容/MUI 决定。证据：`layout.scss:14-28,265-360`；`layout-item.tsx:32-53`。
8. **控件不共享同一个高度 token**：Switch、BaseStyledSelect 有明确尺寸；Button/Input/Icon 主要依赖 MUI size variant。不能把某个页面的 36px toolbar 当成全局 control height。证据：`base-switch.tsx:10-18`；`base-styled-select.tsx:3-15`；`profiles.tsx:827-897`。
9. **选中态偏向 primary 色左边界与浅色底，而不是只靠 checkbox**：Sidebar、Proxy item、Profile card 分别实现相近语义，但不是共享代码。证据：`layout-item.tsx:54-64`；`proxy-item.tsx:68-98`；`profile-box.tsx:3-55`。
10. **复杂设置留在 Dialog，主设置页保持可扫描列表**：SettingItem 的行内 control 用于短操作，复杂 viewer 用 ref 打开。证据：`setting-verge-basic.tsx:52-120`；`setting-verge-advanced.tsx:34-103`；`base-dialog.tsx:29-76`。

## 7. Veyra UI Mapping

本节只定义参考边界，不修改或指定当前 Veyra 实现。

### 7.1 可以直接复用的视觉规则

这里的“复用”指在 Veyra 自己的代码与 token 中重新表达规则，不复制 Clash Verge Rev 源码。

- 两栏桌面壳：左导航、右内容；页面内部统一 `PageHeader + Content`。
- 58px page header、20px header horizontal padding、1px low-alpha divider，以及默认 10px 内容边距；数据密集页允许 `full` 变体自行管理滚动。
- 展开导航采用约 200px 的 nominal 设计基准，但实现时必须由 Veyra 明确是否固定；折叠导航可采用源码明确的 72px rail、52×52 item、12px radius。
- 浅/深背景层级：shell/sidebar、page section、card 三层；使用 border/divider 而非 shadow 区分层级。
- Home/Settings 的 6/12 响应式栅格与 `1.5` spacing 表达式；最终像素应由 Veyra 自己的 spacing token 固化。
- 可追溯的浅/深主题背景、文字、divider 和语义色可以作为 Veyra theme baseline；Veyra 应建立自己的单一 Source of Truth，并分别验证 light/dark。
- Page 20/700、Setting section 16/700、Setting row 14、Card title 18/medium 可作为已观察到的层级基线；正文 variant、line-height 和完整 scale 仍需 Veyra 明确定义。
- 无阴影 surface、拖拽态才出现阴影的原则。

Source Evidence：分别见第 2、6 节所列 `page.scss`、`layout.scss`、`_theme.tsx`、`use-custom-theme.ts`、`EnhancedCard` 与 `SettingPage`。

### 7.2 可以重新实现的组件模式

- `AppShell`：只保留 theme、sidebar、content outlet、全局 feedback host 与 desktop titlebar 的 Veyra 版本；不要把 Clash 服务迁移/权限 dialog 混入通用 shell API。
- `Sidebar/SidebarItem`：以同一 navigation metadata 同时驱动 route 和 menu；selected 从 router 派生；collapse 与 tooltip/aria-label 联动。
- `Page` / `PageHeader`：Veyra 可把 `BasePage` 的 title/header/full composition 明确化，避免页面各写一套 header。
- `Card`：重新实现 header（icon/title/action）+ content 的结构；是否保留 `iconColor/minHeight/noContentPadding` 只取决于 Veyra 当前页面的具体复用需求。
- `SettingSection/SettingRow`：分组标题 + label/description/right control；clickable row 用 chevron，异步时原位 loading；复杂表单进入 Dialog。
- `Switch/Input/Select/Tooltip/Dialog/Empty/Loading`：建立 Veyra 自己的轻量 wrapper，仅封装已经出现的共同视觉与可访问性规则，不包装所有 MUI props 形成新平台层。
- `VirtualList/StickyGroupList`：仅在 Veyra 节点/连接数据量确实需要时采用；保留 loading/empty/render 互斥入口。
- `NoticeHost`：页面外集中承载 success/error/info，局部 busy 仍留在动作位置。

对应参考 composition 证据：`src/components/base/*`、`src/components/layout/layout-item.tsx`、`src/components/setting/mods/setting-comp.tsx`、`src/components/home/enhanced-card.tsx`、`src/components/proxy/proxy-groups.tsx`。

### 7.3 与 Clash Verge 业务强耦合、不应复制的部分

- `IVergeConfig/IConfigData/IProfilesConfig`、Mihomo proxy group/provider/rule 数据结构、命令名、API、query keys 和更新流程。
- Clash 模式 `rule/global/direct`、链式代理配置、provider dialog、Merge/Script profile、Clash runtime config viewer。
- 服务迁移、系统代理提权、特定 core 更新、Mihomo WebSocket 与 Clash Verge 事件协议。
- `navItems` 中的品牌 Logo、彩色 icon assets、页面顺序、名称、文案和外部链接。
- `Layout` 中混合的 Clash Verge 配置字段（例如 `collapse_navbar/menu_icon/menu_order`）及其 persistence contract。
- 用户 CSS 注入机制本身。它是 Clash Verge 的产品能力，不是复制 UI 语言所必需的视觉规则。

Source Evidence：`src/pages/_layout.tsx:57-174,193-218`；`src/pages/_navigation.tsx:37-77`；`src/pages/proxies.tsx:24-137`；`src/pages/profiles.tsx:350-615`；`src/pages/_layout/hooks/use-custom-theme.ts:247-309`。

### 7.4 Veyra 需要自己定义的部分

- 固定或弹性的展开 Sidebar 宽度。参考源码只有 200px flex-basis，不能直接推出固定宽度。
- 完整 spacing scale、typography scale、line-height、font-weight tokens、control/button/input/icon exact sizes。
- 统一 radius token，以及 MUI `palette.divider` 与 CSS divider 是否合并为同一语义 token。
- Veyra 的 primary/brand accent、Logo、产品文案、route 信息架构和 sing-box 术语。
- Veyra 状态模型：订阅、节点、出口组、分流、运行模式、错误与 busy/disabled 的判定；视觉状态可以参考，但不能复用 Clash/Mihomo state。
- Windows/macOS/Linux titlebar、窗口按钮与权限提示的 Veyra 平台边界。
- Toast 是否允许右键复制、默认时长、位置偏好；这些是产品交互决策，不是基础视觉必然要求。
- Veyra 是否需要用户主题覆盖、背景图或 CSS injection。若当前需求没有明确要求，不应因参考源码存在而引入。

## 8. 结论

Clash Verge Rev 的可迁移价值主要在 composition：`Layout → Sidebar + Outlet`、`BasePage → PageHeader + Content`、卡片页的 responsive Grid、设置页的 `SettingList → SettingItem → Control/Dialog`，以及数据页在虚拟列表前先做互斥状态分派。它不是一个完整集中式 design-token 系统：颜色和字体相对集中，尺寸、间距、圆角与交互状态大量分散在 SCSS、MUI `sx` 和领域组件中。

因此 Veyra 后续若进入实现，应先把本文已确认的颜色、背景层级、页面壳尺寸和常用组件 composition 转写为 Veyra 自己的 token/component contract；本文标为 `UNKNOWN` 的值必须另行形成 Veyra 决策，不能伪装成 Clash Verge Rev 源码结论。
