---
Version: "0.3"
Status: CANDIDATE
Contract: PROPOSED
---

# Veyra UI Spec

> 状态：CANDIDATE — Human Change/Design Approval pending
>
> 版本：0.3（提案，未生效）
>
> Contract：PROPOSED — canonical 0.2 remains binding
>
> 适用范围：Veyra Desktop 全部生产 UI，以及后续 TASK 的 UI 交付门禁
>
> 前置证据：`docs/ui/clash-verge-ui-analysis.md`、`docs/ui/veyra-current-ui-audit.md`

## 1. 目的与约束

本文冻结 Veyra Desktop 的视觉语言、密度、构图、交互状态与 UI 交付 Gate。目标不是新增 UI Framework，而是确保后续页面实现者不能仅依据个人判断或页面局部便利自行决定尺寸、间距、背景、状态、卡片边界、信息密度和交互反馈。

Veyra 的视觉目标为：

> 紧凑、克制、清晰、高信息密度、面向桌面工具的现代 UI。

视觉语言参考 Clash Verge Rev 的桌面密度、层级控制和状态表达，但不复制其源码、品牌、Logo、文案、Mihomo/Clash 数据模型、命令或依赖体系。

必须遵守：

1. 保留当前 Router/page switching、state、IPC、query、event、runtime 和 Tauri 行为。
2. 继续使用 React + 原生 HTML + CSS variables；本文不授权引入组件库、Router、CSS-in-JS、主题框架或 Design System Framework。
3. 先复用现有组件和 class API。只有当前不存在职责承载者时，才新增最小 primitive。
4. 普通 surface 不使用 gradient、glass、blur、decorative shadow；overlay 仍采用 border-first。
5. 高密度页面优先使用 Section + Row / Compact Grid，不使用“一个业务对象一张大型 Card”的 Dashboard 构图。
6. 正常状态保持安静；颜色只用于交互、选中、警告、错误和少量关键状态。
7. 每项页面改造分别验证 light/dark、loading/empty/error/busy/disabled/selected/focus 和实际操作。
8. 所有 UI TASK 必须绑定本 Contract；没有 Compliance Evidence 和 Human Visual Approval，不得进入 DONE/CLOSED。

---

## 2. 规范级别与执行语义

本文使用以下强制词：

- **MUST**：实现必须满足；违反即 Visual Gate FAIL。
- **MUST NOT**：实现不得出现；出现即 Visual Gate FAIL。
- **SHOULD**：默认采用；偏离必须在 TASK Design 中记录理由。
- **UNKNOWN**：当前没有批准值；Agent 不得自行填充。

### 2.1 UI Contract Source of Truth

生产 UI 的唯一规范来源为：

```text
`docs/ui/veyra-ui-spec.md`
```

实际 CSS token 的唯一 Source of Truth 为：

```text
`src/styles.css :root`
+ dark theme override
```

页面和组件不得复制同一语义 token 为新的局部 magic value。

### 2.2 Binding Rule

凡 TASK 包含以下任一内容：

```text
页面
组件
视觉状态
布局
Dialog / Menu / Toast
CSS token
Sidebar / Header / Shell
```

其 Task / Technical Design MUST 显式声明：

```yaml
ui_contract:
  document: docs/ui/veyra-ui-spec.md
  version: "0.3"
  required: true
```

缺少该声明时，UI Delivery Gate 不得判定 PASS。

---

## 3. 核心视觉原则

### 3.1 高密度优先

Veyra 是网络工具，不是内容展示产品。

节点、出口组、规则、连接、日志属于高密度信息。页面 MUST NOT 通过大量白卡、大标题、大按钮和大面积空白表达层级。

优先结构：

```text
Page
└── Section
    ├── Compact Header
    └── Dense Row / Compact Grid
```

### 3.2 单一强调

同一个状态最多保留一种主要视觉强调。

例如 Selected Node 默认只使用：

```text
selected background
+
check / selected glyph
```

MUST NOT 同时叠加：

```text
蓝色左边框
+ 蓝色标题
+ 蓝色 Badge
+ 蓝色背景
```

### 3.3 正常状态保持安静

以下状态默认不显示常驻 Badge：

```text
已保存
正常
已加载
成功
```

只突出：

```text
待应用
错误
警告
正在处理
不可用
```

### 3.4 少卡片、弱边界、无装饰性 Shadow

普通层级依靠：

```text
background
border
typography
spacing
```

表达。

普通 Surface MUST NOT 使用可见 box-shadow。

Shadow 只允许用于确有层级浮起需求的 overlay，并保持极轻；当前 Dialog/Menu/Toast 优先仍使用 border-first。

---

## 4. Design Tokens

### 4.1 Layout Geometry

| Token | Final value | Source | Rule |
|---|---:|---|---|
| Sidebar width | `188px` fixed | VEYRA DECISION | 降低 Shell 的视觉占比；Windows DPI 由系统缩放，不通过扩大 CSS px 补偿 |
| Collapsed sidebar width | `UNKNOWN` | UNKNOWN | 当前不实现 collapse |
| Logo area height | `52px` | VEYRA DECISION | 与 compact Sidebar 密度一致 |
| Navigation item height | `36px` | VEYRA DECISION | 固定，禁止由 padding 自由撑高 |
| Navigation item horizontal margin | `8px` | VEYRA DECISION | Sidebar 内统一 |
| Navigation item horizontal padding | `10px` | VEYRA DECISION | 与 18px glyph 匹配 |
| Page header height | `58px` | KEEP + REFERENCE | 保持既有 Header 逻辑，但内部字号与 action 收敛 |
| Header horizontal padding | `20px` | KEEP + REFERENCE | 默认桌面 |
| Header horizontal padding, narrow | `14px` | KEEP | 窄窗口 |
| Page padding, default | `10px` | KEEP + REFERENCE | 卡片/设置类页面 |
| Page padding, full | `0` | REFERENCE | Proxies / Connections / Logs 等高密度页 |
| Dense page inner padding | `12px` | VEYRA DECISION | full page 内 toolbar / list 的统一边距 |
| Content/surface padding | `14px` standard; `10px` dense | VEYRA DECISION | 比 V0.1 进一步收紧 |
| Section gap | `10px` | VEYRA DECISION | 页面级垂直 gap |
| Component gap | `6px` compact; `10px` standard; `14px` form/large | VEYRA DECISION | 不允许页面自创常规 gap |
| Settings columns | desktop `2 × 1fr`; narrow `1fr` | REFERENCE pattern / VEYRA DECISION | 真实 Settings Domain 获批后使用 |
| Settings breakpoint | `720px` | VEYRA DECISION | 保持 Veyra 独立值 |

`full` 不是“无布局”。它表示 Page 不额外包 10px 外边距，页面 MUST 自己提供 toolbar padding、list padding 和内部滚动。

### 4.2 Background Hierarchy and Color

Veyra 使用结构背景层，不使用单一 `--card-background` 混合所有层级。

| Token role | Light | Dark | Source |
|---|---:|---:|---|
| `window` | `#F1F2F4` | `#252731` | VEYRA DECISION |
| `shell/sidebar/header` | `#F7F8FA` | `#2A2C36` | VEYRA DECISION |
| `page-canvas` | `#F1F2F4` | `#1F2129` | VEYRA DECISION |
| `content-surface` | `#FFFFFF` | `#292B35` | KEEP / VEYRA DECISION |
| `control-surface` | `#FFFFFF` | `#292B35` | VEYRA DECISION |
| `popup-surface` | `#FFFFFF` | `#292B35` | VEYRA DECISION |
| `primary` | `#1677FF` | `#4096FF` | VEYRA DECISION |
| `text-primary` | `#20242A` | `#F3F4F6` | VEYRA DECISION |
| `text-secondary` | `#667085` | `#A7ADBA` | VEYRA DECISION |
| `text-muted` | `#98A2B3` | `#7F8796` | VEYRA DECISION |
| `error` | `#D92D20` | `#F97066` | VEYRA DECISION |
| `warning` | `#B26A00` | `#F5A524` | VEYRA DECISION |
| `success` | `#27864B` | `#45B26B` | VEYRA DECISION |
| `divider` | `rgba(16,24,40,.08)` | `rgba(255,255,255,.08)` | VEYRA DECISION |
| `hover` | `rgba(16,24,40,.045)` | `rgba(255,255,255,.06)` | VEYRA DECISION |
| `selected` | `rgba(22,119,255,.10)` | `rgba(64,150,255,.18)` | VEYRA DECISION |

正文 MUST NOT 使用纯黑 `#000000` 作为默认主文字。

Accent 只用于：

```text
Primary Action
Selected State
Interactive Focus
重要链接
```

### 4.3 Border, Radius and Shadow

| Token | Final value | Source | Rule |
|---|---:|---|---|
| Structural border | `1px solid divider` | KEEP + REFERENCE | Sidebar、Header、Surface、Popup |
| Surface radius | `8px` | KEEP | 不增加更大普通卡片圆角 |
| Control radius | `6px` | VEYRA DECISION | Button/Input/Select 更偏桌面工具 |
| Popup radius | `8px` | KEEP / VEYRA DECISION | Dialog/Menu/Toast |
| IconButton radius | `5px` standard; `50%` circular variant | VEYRA DECISION | 仅明确 circular variant 才可 50% |
| Regular surface shadow | `none` | BINDING | 普通 Surface 禁止 shadow |
| Popup shadow | `none` by default | BINDING | 只有专项设计批准后才能增加 |

### 4.4 Typography

字体继续使用系统字体栈，Windows 保留现有 emoji fallback；所有表单控件 MUST `font-family: inherit`。

| Role | Size / weight / line-height | Source |
|---|---|---|
| Page title | `20px / 600 / 26px` | VEYRA DECISION |
| Section title | `14px / 600 / 20px` | VEYRA DECISION |
| Item title | `14px / 500 / 20px` | VEYRA DECISION |
| Body / Setting label | `13px / 400 / 20px` | VEYRA DECISION |
| Supporting text | `12px / 400 / 18px` | VEYRA DECISION |
| Caption / metadata | `11px / 400 / 16px` | VEYRA DECISION |
| Sidebar label | `14px / 500 / 20px` | VEYRA DECISION |

MUST NOT 在普通页面大量使用 `700` 字重。

节点名、协议、订阅来源等数据型信息 MUST NOT 使用 Page/Section Title 级字号。

### 4.5 Controls and Icons

| Token | Final value | Source | Scope |
|---|---:|---|---|
| Standard control height | `34px` | VEYRA DECISION | Button/Input/Select |
| Compact control height | `30px` | VEYRA DECISION | header/card inline action |
| Button horizontal padding | `12px` | VEYRA DECISION | 文本按钮 |
| IconButton standard | `30 × 30px` | VEYRA DECISION | Header/Card/Toolbar |
| Standard action glyph | `16px` | VEYRA DECISION | Button/IconButton |
| Navigation glyph | `18px` | VEYRA DECISION | Sidebar only |
| Inline/status glyph | `14px` | VEYRA DECISION | RateRow/status |
| Switch | `40 × 24px`; thumb `20px` | VEYRA DECISION | 后续 Settings |
| Checkbox | platform/native size | KEEP | 多选/确认 |

Icon style MUST 统一为单一线性体系；不得混合粗黑 Filled、大尺寸 Outline、Emoji 风格图标。

---

## 5. Interaction State Spec

| State | Required visual rule | Behavior boundary |
|---|---|---|
| Hover | 非 selected item 使用 `hover` 背景；不得位移、scale 或增加 shadow | 不触发业务动作 |
| Pressed | Button foreground/background 轻微变深或 `opacity:.92` | 原 handler 不变 |
| Sidebar selected | `selected` 背景 + primary glyph；文字保持高对比 | `aria-current=page` 绑定真实 state |
| Row selected | `selected` 背景 + component-appropriate 单个 indicator | 不再默认使用 3px 左边框 + 蓝 title 的双重强调；SubscriptionCard active 适用下列专用规则，其他组件行为不因此改变 |
| SubscriptionCard active | 唯一 active indicator 为 `2px solid primary` 强调边框；正常 content-surface 背景 | 无 selected/check glyph、Badge、primary title、shadow、左侧竖条；hover不得覆盖active边框；focus为独立focus state；避免布局跳动 |
| Disabled | `opacity:.50`; default cursor；hover 不改变视觉 | 原生 disabled + handler guard |
| Loading, local action | 原动作位置 inline spinner / busy label | 只锁定冲突动作 |
| Loading, page/card | `loading | empty | error | render` 互斥 | 不把运行环境异常伪装成 empty |
| Expand / Collapse | 16px chevron；header 原位；动画 `150~180ms ease-out` | 状态由领域组件持有 |
| Dialog | backdrop + bordered popup；Esc、focus trap、return focus、busy close guard | 保留现有 focus/async 语义 |
| Dropdown / Context Menu | dense items、8px popup、1px border、无 shadow | position/keyboard/focus 不变 |
| Tooltip | 保持可聚焦触发；未有统一 primitive 时保留 `title/aria-label` | 不为视觉补全创造复杂框架 |
| Success Toast | max-width `300px`; 2.5~3s 自动消失；无显式关闭按钮 | 普通成功反馈 |
| Error Toast | max-width `340px`; 可持久并提供 Recovery Action | 错误/需要决策 |

---

## 6. Component Composition Spec

### 6.1 AppShell

```text
App（保留 state / IPC / handlers）
└── AppShell（pure presentation）
    ├── Sidebar
    ├── Page outlet（activePage + hidden）
    └── FeedbackHost presentation
```

AppShell MUST NOT 拥有业务 state，不迁移 Router，不决定页面数据。

### 6.2 Sidebar

Sidebar 是辅助导航，不得成为页面视觉主角。

结构：

```text
Logo Area
Navigation
Flexible Space
Telemetry / Runtime Summary
```

要求：

```text
width: 188px
logo area: 52px
nav item: 36px
nav glyph: 18px
nav label: 14px
```

Sidebar 内 MUST NOT 出现大号黑色图标、大面积 selected block、多个无意义 separator。

### 6.3 Page

```text
Page
├── PageHeader
│   ├── title
│   └── optional status/actions
└── PageContent(default | full)
```

PageHeader MUST NOT 使用 hero title、marketing subtitle、gradient 或大面积留白。

PageHeader 同一操作区最多允许一个 Primary Action。

### 6.4 Surface

Surface 是布局边界，不等于 Dashboard Card。

标准：

```text
content-surface
1px divider
8px radius
14px standard / 10px dense padding
no shadow
```

对于密集页面，一个领域对象 SHOULD 优先映射为 Row / Section，而不是独立大型 Card。

### 6.5 Dense Row

默认高密度 Row：

```text
height: 40~44px
horizontal padding: 10~12px
primary text: 13~14px
metadata: 11~12px
```

行内信息 SHOULD 在同一水平扫描线上完成，不通过两三层垂直堆叠扩大高度。

### 6.6 Dialog

普通 Dialog：

```text
width: 440~480px
padding: 18~20px
footer gap: 8px
```

复杂创建流程最大：

```text
560px
```

不得把普通编辑操作扩展为大型全屏表单，除非业务复杂度明确要求。

### 6.7 Feedback

普通 Success Feedback：

```text
✓ 配置已应用
```

不得增加长期“已保存” Badge、粗绿色边条或手动关闭按钮。

---

## 7. Golden Page Strategy

Veyra 使用两类 Golden Page 分别冻结普通 surface 和 dense surface。

### 7.1 Golden Page A — Subscriptions

用于冻结：

```text
Shell
Sidebar
PageHeader
Toolbar
Surface
Input
Button
Dialog
Menu
Notice
Empty / Loading / Error
```

### 7.2 Golden Page B — Proxies

用于冻结：

```text
full Page
dense Section
dense Row
expand/collapse
selected state
high-density node presentation
```

Golden Page B 经 Human Visual Approval 后，Routing / Connections / Logs MUST 复用其 dense visual language，不得重新定义另一套密度和 selection 规则。

---

## 8. Page-specific Target Rules

### 8.1 Overview

- 使用 default Page。
- 图表不是 Hero；不得用超大标题、gradient、shadow 或额外大留白强化。
- 保留 TrafficChart、TrafficRates、runtime controls 和 diagnostics 的真实数据与事件。
- loading/unavailable/stopped 继续使用真实 observation state。

### 8.2 Subscriptions

- 使用 Golden Page A 视觉语言。
- Grid min column `260px`；Card/Surface 只表达订阅对象自身，不扩大成 Dashboard 模块。
- SubscriptionCard active 仅以 `2px solid primary` 边框强调，保持正常 `content-surface` 背景；不使用整卡 selected background、selected/check glyph、Badge、primary title、shadow或左侧竖条。hover保留active边框；focus必须独立可辨认，不能复用active边框冒充focus；避免边框变化引起布局跳动。此规则仅适用于SubscriptionCard，不改变Sidebar/NodeRow/Routing/Menu等selected规则。
- Button/Input/Dialog/Menu/Notice 统一 token；不改业务状态机。

### 8.3 Proxies

Proxies 是高密度数据页，必须使用 `full Page`。

页面层级：

```text
PageHeader
↓
Pool Section(s)
↓
All Nodes Section
```

#### Pool Section

Pool MUST 表现为紧凑 Section，而不是大型展示 Card。

Pool Header：

```text
height: 48~52px
line 1: pool name
line 2: type · node count · selection policy
right: compact actions
```

正常状态 MUST NOT 显示“已保存” Badge。

只有 Dirty 状态显示弱 Warning：

```text
● 有待应用更改
```

#### Pool Node Row

```text
height: 40~44px
```

选中节点默认：

```text
selected background
+
check glyph
```

MUST NOT 同时使用 3px 左边框、蓝色节点名、蓝色“已选”文本。

#### All Nodes

All Nodes 采用 dense list 或 compact grid。

若使用 grid：

```text
grid-template-columns: repeat(auto-fill, minmax(220px, 1fr))
```

单 Node Item 高度：

```text
48~54px
```

节点是数据对象，不得通过宽大白块人为拉高视觉体积。

### 8.4 Routing

- 使用 Golden Page B 的 full Page、toolbar、Section、Row、selected、状态语言。
- 具体字段、规则组合、filter、排序来自 Veyra 已批准业务模型。
- 不复制 Clash/Mihomo schema。
- 不为填满页面制造业务字段。

### 8.5 Connections / Logs

- 复用 Golden Page B 的 dense language。
- 只有真实数据规模证明需要时才加入 virtualization。
- 不以大型 Card 表达单连接或单日志记录。

### 8.6 Settings

- 只有真实 Settings Domain 获批后进入生产页面。
- Settings 两列布局只作为真实字段映射，不为了使用 primitive 创造设置项。
- SettingRow 使用 compact label + optional description + right-aligned control。

---

## 9. Visual Density Guardrails

以下为全局 FAIL 条件：

1. Sidebar 导航项实际 CSS 高度超过 `40px`。
2. 普通数据节点/规则 Row 实际 CSS 高度超过 `48px`，且没有业务必要说明。
3. 一个简单状态同时使用 3 种及以上颜色/边框/Badge/背景强调。
4. 正常状态常驻显示“已保存”“成功”“正常”等 Badge。
5. 普通页面出现大面积无信息价值的 Surface nesting。
6. 一个 Primary Action 区域同时存在两个或更多高饱和 Primary Button。
7. 普通正文使用 16px 及以上字号作为默认数据行文字。
8. 普通 Surface 使用 box-shadow。
9. 页面为了“精致”引入 gradient、glass、blur、scale hover 或 decorative animation。
10. 页面新增未在本文或批准 DCR 中定义的常规 spacing/radius/control height。

---

## 10. Windows DPI / Window Size Verification

CSS 使用逻辑像素；不得为高 DPI 手工放大 token。

最低验证矩阵：

```text
1280 × 720 @ 100%
1440 × 900 @ 100%
1280 × 720 @ 125%
1440 × 900 @ 125%
1280 × 720 @ 150%
```

要求：

```text
无异常横向滚动
操作按钮不重叠
Header 不断裂
Sidebar 不异常占据内容区
导航项不因 DPI 被额外放大
文字不截断
Dialog 不超出窗口
Toast 不遮挡主要操作
```

---

## 11. UI Delivery Gate

### 11.1 Task / Design Required Input

UI TASK 必须在 Task 和 Technical Design 中声明：

```yaml
ui_contract:
  document: docs/ui/veyra-ui-spec.md
  version: "0.3"
  required: true

visual_gate:
  required: true
  human_approval: true
```

Technical Design MUST 列出：

```text
涉及页面
涉及组件
使用 token
新增 token（如有）
未知值处理
Golden Page 复用关系
```

Agent 不得在实现阶段自行把 `UNKNOWN` 改成具体值。

### 11.2 Machine Compliance Evidence

每个 UI Delivery 至少提交：

```text
lint
frontend tests
typecheck（如项目存在）
build
```

以及 UI Contract Compliance Report，至少检查：

```text
Sidebar width
Nav item height
Nav glyph size
Typography roles
Surface radius
Control height
非法 shadow
非法 gradient/blur
selected state composition
Toast width/behavior
```

推荐 Evidence：

```text
.sdlc/evidence/<TASK>/ui-contract-compliance.json
```

### 11.3 Screenshot Evidence

生产 UI 变更必须提供真实 Tauri Windows Screenshot，不允许用 HTML mock / Storybook / 静态设计稿替代。

最低截图：

```text
Light normal
Dark normal
Selected
Disabled / Busy（适用）
Loading / Empty / Error（适用）
Dialog / Popup（适用）
Success Toast
1280×720 @ 125%
1440×900 @ 100%
```

Proxies / dense page 额外：

```text
Pool Expanded
Pool Collapsed
Selected Node
Dirty State
Applied State
All Nodes
```

推荐 Evidence：

```text
.sdlc/evidence/<TASK>/ui-screenshots/
```

### 11.4 Human Visual Approval

Human Visual Approval 是不可由 Agent 自判的 Gate。

Agent/Reviewer 可以检查：

```text
token
computed value
状态完整性
功能回归
截图是否齐全
```

但不得自行声明以下判断为 PASS：

```text
是否精致
信息密度是否舒服
视觉层级是否成熟
是否存在明显 AI Dashboard 风格
整体桌面感是否达到要求
```

推荐 Evidence：

```text
.sdlc/evidence/<TASK>/ui-human-approval.json
```

最小结构（由人工填写并提供，Agent 不得生成 APPROVED 文件；`target_identity` 和哈希绑定本轮实际交付材料）：

```json
{
  "task_id": "TASK-XXX",
  "contract": "docs/ui/veyra-ui-spec.md",
  "contract_version": "0.3",
  "target_identity": "sha256:<reviewed-delivery-target>",
  "decision": "APPROVED",
  "approved_by": "human",
  "recorded_by": "human",
  "timestamp": "<ISO-8601 approval time>",
  "reviewed_screenshots": [
    ".sdlc/evidence/TASK-XXX/ui-screenshots/<reviewed-screenshot>.png"
  ],
  "compliance_sha256": "sha256:<ui-contract-compliance.json file hash>"
}
```

没有 `APPROVED` Evidence：

```text
UI Delivery Review != PASS
TASK != DONE/CLOSED
后续依赖该 Golden Page 的 UI TASK 不得开始
```

---

## 12. Implementation Boundary

UI 调整默认只能修改：

```text
AppShell presentation
Sidebar presentation
Page layout
shared CSS variables
shared UI primitive
page-local presentation
Dialog/Menu/Toast presentation
```

不得以视觉调整为理由修改：

```text
Domain Model
Application Service
IPC Contract
StateStore
RuntimeIntent
SingBoxCompiler
Routing Semantics
Pool Semantics
Persistence Semantics
Runtime State Machine
```

如确有 Material Change，必须退出 UI Delivery，单独走 Technical Design / DCR。

---

## 13. Migration / Rollout Plan

### Phase 1 — Visual Foundation + Subscriptions Golden Page A

目标：冻结 Shell、Sidebar、PageHeader、Surface、Toolbar、Control、Dialog、Menu、Toast 的视觉语言。

停止条件：

```text
Machine Compliance PASS
Human Visual Approval APPROVED
```

### Phase 2 — Proxies Golden Page B

目标：冻结 full Page、dense Section、dense Row、selected、expand/collapse、All Nodes 的高密度语言。

停止条件：

```text
Machine Compliance PASS
Human Visual Approval APPROVED
```

### Phase 3 — Routing

复用 Golden Page B；只定义 Veyra Routing 的真实业务 composition。

### Phase 4 — Settings Domain Gate

先冻结真实 Settings field、state ownership、persistence、IPC、platform behavior，再实现生产 Settings。

### Phase 5 — Overview

在不改变 runtime/observation 数据模型的前提下收敛为 compact dashboard。

### Phase 6 — Connections → Logs

复用 Golden Page B 的 dense language；逐页独立验收。

---

## 14. TASK-012 Visual Remediation Acceptance

TASK-012 当前 UI 收尾使用本文作为 Binding Contract。

必须满足：

```text
Sidebar <= 188px CSS logical width
Nav item 36px
Nav glyph 18px
Sidebar label 14px
Page title 20/600
Section title 14/600
Body 13px
普通数据 Row 40~44px
正常状态无“已保存” Badge
Selected Node 使用单一 selected language
Success Toast 自动消失且无关闭按钮
普通 Surface 无 shadow
Proxies 不采用大型 Dashboard Card 构图
Routing 复用同一 dense visual language
Windows 100% / 125% / 150% DPI 验证
```

Human Visual Approval APPROVED 后，TASK-012 才可完成 UI Gate。

---

## 15. 最终约束

本文是生产 UI 的 Binding Contract，不是设计建议合集。

后续 Agent 的职责是：

```text
读取 Contract
→ 映射真实业务
→ 实现
→ 机器校验
→ 提供真实截图
→ 等待 Human Visual Approval
```

而不是：

```text
读取参考
→ 自行发挥
→ 以“功能完成”代替 UI 验收
```

只要涉及 Veyra Desktop 的生产 UI，本 Contract 即适用。后续批准版本在本路径原位更新，项目只保留一个当前有效的 UI Contract；历史版本由 Git 保留。
