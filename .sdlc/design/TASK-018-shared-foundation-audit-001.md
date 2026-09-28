# Shared Foundation Static Impact Analysis 001

穷尽范围是Subscriptions所消费的shared foundation明确Final值与直接token/geometry消费者。不是全部页面审计；同一文件中的Settings、Proxies/Routing专用规则只登记共享token影响，不审计其局部composition。只读静态匹配不构成computed/native/Certification PASS。

## A. CONFIRMED_STATIC_MISMATCH

每行均为 purely presentation；影响所有共享 Shell/token 页面（Overview、Subscriptions、Proxies、Routing 以及未来页面宿主）。专用消费者只改变共享角色映射，不改页面业务。

| ID | Contract / role | Expected | Actual | Source / consumers | Remediation |
|---|---|---|---|---|---|
| SF-V02-001 | §4.1/6.2 Sidebar width | 188px | 200px | src/styles.css:30; .app-shell grid / Sidebar | --sidebar-width →188px; 同步旧200px说明 |
| SF-V02-002 | §4.2 text-primary light/dark | #20242A / #F3F4F6 | #000000 / #ffffff | src/styles.css:20; body、Sidebar、输入、Dialog、Menu、Notice | 同步 --primary-text 与 :root color 两主题 |
| SF-V02-003 | §4.2 window light/dark | #F1F2F4 / #252731 | #ececec / #2e303d | src/styles.css:12; body / window | 同步 --window-background 与 :root background |
| SF-V02-004 | §4.2 shell light/dark | #F7F8FA / #2A2C36 | #f5f5f5 / #2e303d | src/styles.css:13; Sidebar / PageHeader | --shell-background |
| SF-V02-005 | §4.2 page-canvas light/dark | #F1F2F4 / #1F2129 | #ececec / #1e1f27 | src/styles.css:14; page-scroll | --page-background |
| SF-V02-006 | §4.2 content/control/popup dark | #292B35 (三种角色) | #282a36 (三种角色) | src/styles.css:76; Surface / Input / Dialog / Menu / Toast | 三个现有背景token dark override；light #fff 已匹配 |
| SF-V02-007 | §4.2 primary light/dark | #1677FF / #4096FF | #007aff / #0a84ff | src/styles.css:18; primary action / focus / selected glyph | --primary；不把 --secondary 流量色当成通用primary |
| SF-V02-008 | §4.2 text-secondary light/dark | #667085 / #A7ADBA | #3c3c4399 / #ebebf599 | src/styles.css:21; supporting copy / metadata / Sidebar | --secondary-text |
| SF-V02-009 | §4.2 error light/dark | #D92D20 / #F97066 | #ff3b30 / #ff453a | src/styles.css:26; danger / error Notice | --error |
| SF-V02-010 | §4.2 warning light/dark | #B26A00 / #F5A524 | #ff9500 / #ff9f0a | src/styles.css:27; 共享 warning token 的消费者 | --warning |
| SF-V02-011 | §4.2 success light/dark | #27864B / #45B26B | #06943d / #30d158 | src/styles.css:28; success Notice / shared success | --success |
| SF-V02-012 | §4.2 divider light/dark | rgba(16,24,40,.08) / rgba(255,255,255,.08) | rgba(0,0,0,.06) / rgba(255,255,255,.06) | src/styles.css:22; 结构 border / SidebarTraffic grid | --divider |
| SF-V02-013 | §4.2/5 hover light/dark | rgba(16,24,40,.045) / rgba(255,255,255,.06) | --selection rgba(0,0,0,.05) / rgba(255,255,255,.08)；SubscriptionCard :hover仅改border，没有hover背景（styles.css:563-566） | src/styles.css:23; nav / refresh / menu / document tools hover；非selected SubscriptionCard:hover（563-566） | 现有 --selection 对应hover角色；调整两主题及错误使用selected背景的非selected hover；非active/非selected且非disabled SubscriptionCard:hover使用hover背景，selected背景保持独立，focus语义不改变 |
| SF-V02-014 | §4.2/5 selected light/dark | rgba(22,119,255,.10) / rgba(64,150,255,.18) | --primary-alpha rgba(0,122,255,.15) / rgba(10,132,255,.35) | src/styles.css:24; Sidebar selected / shared selected consumers | 现有 --primary-alpha 对应selected角色；保留primary-text高对比，不新创颜色 |
| SF-V02-015 | §4.1/6.2 Logo area height | 52px | flex-basis --page-header-height=58px | src/styles.css:138; .brand | 只分离brand高度52；PageHeader仍58 |
| SF-V02-016 | §4.1/9 Navigation item geometry | 固定36px；水平margin8px/padding10px | min-height44px；margin水平10px；padding水平12px；width calc(100%-20px) | src/styles.css:169; .sidebar nav button | height36、水平8/10、宽度配套；不改导航行为 |
| SF-V02-017 | §4.4 Sidebar label | 14/500/20 | 16/400/24 | src/styles.css:179; .sidebar nav button / .nav-label | 字号/字重/行高准确映射；既有绝对定位需适配新glyph但不重新布局导航 |
| SF-V02-018 | §4.5/5 Navigation glyph & selected glyph | 18px线性体系；selected primary glyph | 24px；App内path fill=currentColor stroke=none；选中仍继承普通文字色 | src/styles.css:201; App.tsx:178 svg / nav current | 仅导航SVG presentation path/属性和CSS18px/selected glyph颜色；保留id/viewBox可视裁切检查和handlers |
| SF-V02-019 | §4.1 Section gap | 10px | 12px | src/styles.css:36; .page-scroll | --section-gap |
| SF-V02-020 | §4.1 Component gap compact/standard/form | 6 / 10 / 14px | 8 / 12 / 16px | src/styles.css:37; header/control/表单容器；消费者见SF-021 | 三个现有component gap token |
| SF-V02-021 | §4.1/9 消费者硬编码常规gap | compact6 / standard10 / form14；Dialog footer专用8 | 8/12/16/20px直接常规gap | src/styles.css:450; nav button168=12; subscription toolbar453=8; batch-actions575=12; card header588=12; meta627=12; traffic children652=8; dialog form683=16; options726=12; qr-panel740=12; manual modes989=20; mode labels991=8; file-picker993=12/button994=8; DocumentEditor.css header3=16/footer7=12/tools8=8; SidebarTraffic.css:37 margin-right8; subscription-state:533 gap12 | compact6: nav gap、meta、traffic行、file-picker按钮、document工具、SidebarTraffic glyph间距；standard10: toolbar、batch-actions、card header、state、file-picker容器；form14: dialog form、options、QR、manual modes、document header；mode label6；DocumentEditor footer12→专用8。已是footer8、QR quiet zone、form label6、微间距不改 |
| SF-V02-022 | §4.1/6.4 Surface padding standard/dense | 14 / 10px | 16 / 12px | src/styles.css:40; shared surface tokens；subscription-card padding12 | 现有token14/10；普通订阅Surface消费者padding14，保留对象构图 |
| SF-V02-023 | §4.3 Control radius | 6px | 8px | src/styles.css:43; Button/Input/Select | --control-radius |
| SF-V02-024 | §4.3 非圆形IconButton radius | 5px | subscription header=control8；document-tools=4px | src/styles.css:570; subscription-header-actions:570；DocumentEditor.css:9 | 各标准icon按钮5px；refresh明确circular50%保留 |
| SF-V02-025 | §4.4 Page title | 20/600/26 | 20/700/24 | src/styles.css:54; .home-header h1:238 | title line token26，h1 weight600 |
| SF-V02-026 | §4.4 Section title token | 14/600/20 | size16/line22；Subscription dialog h2硬编码18 | src/styles.css:56; dialog标题/共享section角色 | section token14/20；实际section标题600；不修改隐藏Settings专用结构 |
| SF-V02-027 | §4.4 Body role | 13/400/20 | token14/20；body未显式绑定字体大小/行高；text button/input14 | src/styles.css:58; body、text buttons、quick input、form inputs | body及明确body消费者绑定13/400/20 |
| SF-V02-028 | §4.4 Supporting role | 12/400/18 | token13/18；description13/16 | src/styles.css:60; home-status / subscription-description / file-picker说明 | supporting token12/18；直接角色覆盖同步 |
| SF-V02-029 | §4.4 Caption/metadata | 11/400/16 | token12/16；subscription-meta13/18；traffic12/14；field-label12/14 | src/styles.css:62; subscription-meta/traffic、浮动label、document header supporting label | caption token11/16；floating label是supporting12/18，不误降为metadata |
| SF-V02-030 | §4.4 Subscription item title | 14/500/20 | 18/600/26 | src/styles.css:595; .subscription-card h2 | item title角色直接映射，不变Card内容 |
| SF-V02-031 | §4.5 Standard/compact control height | 34 / 30px | standard36；inline text controls仍继承standard | src/styles.css:48; primary/secondary/danger button、Input/Select、Dialog footer、document inline action | standard34；实际header/card inline30；多行textarea不强制34 |
| SF-V02-032 | §4.5 Text button horizontal padding | 12px | 14px | src/styles.css:509; .primary-button/.secondary-button/.danger-button | 现有button padding-inline12 |
| SF-V02-033 | §4.5 IconButton geometry | 30×30 | shared token32；DocumentEditor tools32 | src/styles.css:49; refresh/header actions/document tools；field-action已30 | token及硬编码消费者30；field-action top3与34高配套居中 |
| SF-V02-034 | §4.5 Action glyph size | 16px | field/refresh18；header20；file-picker18；document-tools22 | src/styles.css:494; 对应现有svg | 统一16，不增加图标库；App导航另为18 |
| SF-V02-035 | §4.5 Inline/status glyph | 14px | SidebarTraffic .sidebar-rate svg16px | src/components/layout/SidebarTraffic.css:35; 侧栏速率/内存glyph | 只调整14px与已有间距，不变采样/绘图算法 |
| SF-V02-036 | §5 Disabled opacity/hover | 0.50；disabled hover不变化 | token.55；header action.4/menu.45；header disabled仍匹配:hover | src/styles.css:29; buttons/input/select；subscription header/menu | 统一已声明disabled消费者为.50/default cursor及hover guard；原生input/select未显式opacity的最终值归N03验证，不改业务guard |
| SF-V02-037 | §5/6.7 Toast max width / success border | success300px/error340px；无粗绿色边条 | 共用360px；success border-left4px | src/styles.css:47; failure-toast/subscription-notice；success规则430附近 | 按现有success/error class限定300/340；success恢复1px divider边界；不碰定时handler |
| SF-V02-038 | §4.3/5 Surface selected composition | 无shadow；selected背景+单一indicator | subscription-card-active inset3px shadow + 蓝h2，无selected背景 | src/styles.css:565; .subscription-card-active | 用既有selected背景+一个非布局性indicator，标题正常色；CSS限定，不改变active state |

## B. REQUIRES_NATIVE_OR_STATE_VALIDATION

- N-01 Windows两主题/五组窗口-DPI矩阵、overflow、文字裁切、Dialog/Toast遮挡与导航比例：必须真实Windows Tauri；本轮NOT_RUN
- N-02 loading/empty/error/render互斥、busy只锁冲突动作、focus trap/return/Esc、menu keyboard、tooltip可聚焦：源码handler不修改；实际状态序列需原生交互验证
- N-03 颜色/字号的最终computed role、CSS cascade和原生控件/Monaco跨主题结果：静态值完整盘点，但不能由它证明所有实际显示状态
- N-04 pressed轻微变深、selected单一强调效果、密度舒适度/成熟度：Contract未给brightness唯一数值；人工视觉判断不能由Agent替代
- N-05 SidebarTraffic数值/品牌字体角色：Contract未单独给brand或telemetry number的Final字号，不能武断以Body替代16/18px；实际层级需后续视觉审阅
- N-06 text-muted角色消费映射：Contract颜色明确#98A2B3/#7F8796，当前没有该token也没有显式muted角色；需确认哪些copy属于muted，不能仅因缺变量名判不一致或任意换色

## C. NOT_APPLICABLE / UNKNOWN（相对本次有界修复）

- C-01 UNKNOWN — Collapsed Sidebar：Contract UNKNOWN，不实现collapse
- C-02 NOT_APPLICABLE — Settings columns/breakpoint/Switch：虽token42×26/thumb22与未来40×24/thumb20不同，但Subscriptions未消费Switch，禁止纳入当前修复
- C-03 NOT_APPLICABLE — full page/dense inner padding/Pool header/NodeRow/AllNodes/expand动画：Subscriptions为default Page；属于后续TASK012 dense stage
- C-04 UNKNOWN — 品牌Logo尺寸、流量图几何/专用upload颜色、QR quiet zone、z-index/backdrop opacity/menu offset：Contract无相应Final值；不移植参考值或顺手修改
- C-05 NOT_APPLICABLE — Checkbox：Contract为platform/native size，无统一px；现有原生checkbox16不能无环境证据当作固定值违规
- C-06 NOT_APPLICABLE — Subscriptions Grid300 vs §8.2 min260、普通Dialog映射375/520 vs §6.6普通440–480、DocumentEditor大尺寸/全屏：页面专用composition不属shared foundation修复；明确保留后续页面级认证检查，不能隐含为PASS或在本设计修复
- C-07 NOT_APPLICABLE — Success Notice timeout6000ms（SubscriptionPage.tsx:112） vs §5 2500–3000ms：这是可静态确定的页面专用反馈行为差异，不属于token/geometry；明确记录而非假装native未知。本轮禁止handler改动，后续认证仍可能FAIL
- C-08 UNKNOWN — --page-padding-narrow8px override：Contract规定default10px与header narrow14px，但未给page narrow独立Final值；不擅自创设/删除适配，应在后续窗口验证与角色评审确认

## 已核对的静态匹配（不是Certification PASS）

- PageHeader height: 58px
- Header padding default/narrow: 20/14px
- Default Page padding: 10px
- Light content/control/popup: 全部#FFFFFF
- Surface/Popup radius: 8px
- Structural border: shared Sidebar/Header/Dialog/Menu 1px solid divider；颜色偏差见SF012
- Popup shadow: Toast/Dialog/Menu none
- Font: 系统栈+打包Twemoji、表单font-family inherit
- Menu text: 13/400/20，半径6，popup8
- Dialog padding/footer: 20px / 8px
- Field action / circular refresh: field-action30；refresh显式circular50%
- Focus: primary outline；具体效果N-02
- Static hover motion: 共享hover无translate/scale/装饰shadow；card selected例外SF038
- PageHeader primary count: Subscriptions header icon actions；无多个primary text actions

## 覆盖索引

- §4.1：Sidebar/logo/nav SF001/015–017；Header/default padding静态匹配；surface/gap SF019–022；full/dense/Settings C02/C03；narrow C08。
- §4.2：全部16颜色角色已盘点；SF002–014涵盖有实际角色消费者的差异；light三surface匹配；text-muted N06。
- §4.3：control/icon radius SF023/024，surface/popup8匹配；border颜色SF012；普通selected shadow SF038。
- §4.4：全部7文字角色 SF017/025–030；font与inherit匹配；特殊品牌/telemetry N05。
- §4.5：control/padding/icon/action/nav/status SF018/031–035；Switch/checkbox C02/C05。
- §5/6：hover/selected/disabled/Toast SF013/014/018/036–038；gap/padding消费者SF021/022；状态行为N02、成功timer C07、页面专用Dialog C06。
- §7/8/9/10/11/12/13：影响链由设计规定；页面专用composition C03/C06；native和Human Gate N01–04，不能以静态匹配替代。
