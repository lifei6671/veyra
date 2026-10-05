# Tokens 与共享组件审查（实施中）

层级：`ui/tokens.rs + ui/theme.rs → ui/components + ui/icons.rs → setting_section / settings_pair / compact_setting / navigation_item → Shell / Settings / Panel`。未修改 Core 数据模型或建立第二份持久化事实。

| 当前真实模式 | 共享实现 / 消费位置 | 来源 |
|---|---|---|
| Button / IconButton | components，Panel、BehaviorPanel、Modal、Shell | controls.tsx、CSS compact-button/icon-button |
| Input | PanelInput：Kit InputState/IME，外观单独包装；设置数值、站点与 evidence 输入 | CSS number-field/test-site-row/site-url-field |
| Select | components/select.rs；语言、主题、provider、分列、evidence 弹窗 | React SelectControl；Kit base focus/Popover/Select |
| Switch | PanelSwitch，IPv6、隐藏不可用及 evidence | controls.tsx + Panel 局部覆盖 |
| Slider | panel_slider；透明度、模糊 | .panel-range；Kit base SliderState |
| Surface / section / row | setting_section、section_heading、CompactSetting、settings_pair | .surface、.settings-block、.compact-setting、.panel-settings-grid |
| NavigationItem | navigation_item：Sidebar / Category 两种真实规格 | AppShell、SettingsPage |
| 图标选择器 / 分类 tab | IconPicker，每个实际测试站点持有 Entity 与订阅，支持 1–32 项投影 | PanelIconPicker.tsx、panel-icons.json |
| Modal | panel_dialog；生产恢复默认与既有 evidence 弹层分开 | shared.tsx、.panel-settings-dialog |
| Tooltip | 共享 Tooltip 覆写外观；窗口定位仍由 Kit/GPUI 管理，定位差异待验收 | React tooltip-trigger |
| Toast / Notice | notify；视觉保存、行为保存、导入/失败共用 | sonner.tsx 与局部 CSS，自有 NoticeCenter；Kit base Alert 语义，单一通知身份替换与 4s 自动关闭 |
| Empty | unavailable；未迁移主路由/设置分类共用 | .empty-state；明确未接入，不展示假业务数据 |
| Loading / Error / StatusBadge | 原 Core/SaveStatus 语义保留；失败重试保留生产入口；工程状态和 synthetic 仅 evidence | 现有 P1 契约；尚需状态矩阵复核 |
| PageToolbar | React Shell 没有全局标题+Refresh；原工程工具条仅 evidence | AppShell.tsx |

没有为后续业务页建立通用框架。Select 仅支持本轮实际有限字符串选项；不增加搜索/多选等假设 API。

工程视觉隔离开关：debug 构建且显式 `VEYRA_UI_EVIDENCE`。默认 debug/release 采用 production 布局。原 Entity/A-B/Busy/Synthetic、schema/revision/selection、UI changes/Save submissions、视觉/行为两标签、拖放/焦点/平台 evidence 控件仍保留用于验收。生产中失败草稿与重试没有隐藏。

# CSS 数值映射

| CSS 来源 | 迁移规格 |
|---|---|
| sidebar / sidebar-collapsed | 256 / 64；顶部36、底部六统计+uptime+三动作 |
| .openbox-app | 14/20；accent #70c996；字体差异另见 font-differences.md |
| .main-nav button | 高36、横 padding12、gap12、radius11、icon20 |
| .settings-top-nav / nav-scroll | 高48、padding8、gap8；分类32高、radius9、icon18 |
| .surface / .settings-block | padding16，表格行gap4、段间gap8，动态全局圆角 |
| .panel-settings-grid | 1280 logical viewport 双列、column gap48 |
| .compact-setting | 40高，label weight500 |
| Panel Select | 高32、radius9、语言/主题192、IP96、分列112 |
| .ob-switch Panel override | 40×24、padding2、边框1、thumb18、位移16、radius12 |
| .number-field | 80×32，字号14、padding12 |
| .panel-range | 256×24、track12、thumb24、边框4 |
| background | 152 input、40上传、32调整；本地受管资产替代远程URL输入 |
| test-site-row | 高40；picker64×32、name112×32、URL弹性宽 |
| h2 | 16/600/24，min32、bottom8 |
| Select menu | padding4、item32、radius7、popup radius10；同源 Check/Chevron SVG |
| Modal | 512×142确认框、radius13、header41、body padding16、footer padding8/16、遮罩40% |
| Toast | width380、top24、radius14、generic 渐变、22px 关闭按钮；Panel toast(message) 无状态图标 |

颜色级联：Light Panel 文本由 OKLCH 精确转换为 sRGB #333c4d；Dark Panel text #cac9c9、bg/surface #1b1717、sidebar #161212，alpha 保留。常规 Shell 与 Panel override 分开。所有新增 Token 必须有实际消费者；不保留未使用的未来规格。

# 字段归属与 Host 答复

全仓 React 搜索确认 UI `config/speedtest-url` 只有读取方，后端测速字段是 Profile `testUrl/directTestUrl`，走 `api.saveProfile`，不是该 UI preference。Host 随后允许只读访问 openbox.disign.me 查入口，实际查看后端/Panel 并恢复原分类，没有改值/保存/操作内核。线上也未找到 Panel 的该字段编辑入口；按 Host “如果没有则不用编辑”，保留数据与持久化，不发明 UI。

`node_sort/group_by_provider/node_card_min_width/strategy_order` 对应 React Proxies 设置弹窗，本轮不实现代理业务；既有编辑与测试能力保留于 evidence，不塞入 Panel。其余真实 Panel 字段按对应 React 结构组合。后端服务测速语义与 UI latency 偏好不合并。

## Host Review 修订：真实状态与职责边界

- 默认背景：Render只读ready缓存；services blocking派生；App key/generation验收+notify，单in-flight合并最新请求，三回归保护stale/dedup/render边界。配置仍只有DesktopVisualPreferences。
- Button与NavigationItem复用PanelButton：同Kit父scope立即Render读取稳定focus handle，外层绘制outline；disabled无焦点装饰。保留Kit输入/键盘行为，实际mouse Save提交成功。IconButton另持有共享ControlFocus；Input关Kit厚shadow，Switch空心outline而非填充shadow。
- Slider显式装配Kit Track/Indicator/Thumb，保留事件/drag/cancel；几何统一tokens，不建新slider框架。
- Tooltip共享hover/focus/Escape/blur实体；help16来自CSSsettings-info；anchored局部坐标固定trigger左上角，避免流式高度叠加。sidebar36/radius9、普通help16/site文本规格仍保留局部差异。
- 视觉证据先做embedded Display ICC→sRGB，避免用截图色彩空间差异改产品颜色。
- 本轮未新增未来P2组件；字体、局部blur与实现未完成细节分开记录。

## 2026-10-05 最终收口补充（优先于上文进行中措辞）

保持原组件层级与共享Tooltip；没有新增另一套Tooltip或背景/Tray框架。最后源码修改仅位于 `ui/tokens.rs`、`components/mod.rs`、`components/tooltip.rs`、`shell.rs`、`panel.rs`、`behavior_panel.rs`：

- Sidebar Tooltip 按CSS8×8旋转45°补arrow；expanded/collapsed不同CSS锚点共用现有实体，hover/focus/Escape/blur都有最终build证据。
- Chromium1226 number-field实际有UA spinner，未出现CSS移除规则。三个真实number input启用共享numeric视觉，14×18 spinner在hover/focus显示；±1沿既有InputState Change/验证/CAS路径。
- UA focus与spinner Light/Dark分别取本地React真实颜色；Input focus2px画在80×32边界内；Button/IconButton仍按外部focus outline显示，不改键盘行为。
- help Heroicon使用Theme muted；brand固定PNG主色384452，Dark Panel遵守局部invert+hue过滤后的b2becc；collapse为原SVG30394c，Dark白色alpha.92。

最终共享组件、字体/局部blur与适用状态逐项判定见 `visual-matrix.md`。旧“定位待采/arrow与spinner未实现”的记录保留历史；当前缺口不再如此声明。

### 最后 IconPicker 收口 · 8bd7a366

当前真实Picker复用PanelInput/PanelButton/Popover；分类flex作用于直接父wrapper，四项等宽；搜索输入使用CSS局部focus text色/2px offset；trigger64×32按space-around排列。未建新体系。Light/Dark最终截图与叠图已复核，普通number输入focus/spinner语义保持。
