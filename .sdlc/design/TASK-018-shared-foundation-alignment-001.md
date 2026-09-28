# UI Contract 0.2 Shared Foundation Alignment

Status: CANDIDATE / Human Design Approval PENDING
Proposed Task: TASK-018（已检查未占用；只作为候选subject，不物化Task或变更focus）

## 1. Impact Analysis

USER:lifei 授权本次穷尽式静态 shared-foundation audit、remediation Design 与独立 Technical Design Review。唯一数值来源为 docs/ui/veyra-ui-spec.md 0.2 APPROVED/BINDING。完整可机读Finding inventory和逐项覆盖矩阵见 TASK-018-shared-foundation-audit-001.json/.md。

旧 SUBS-V02-001/002 及其 Certification/Compliance FAIL 保留原hash；新SF-V02-001/002仅引用这两个事实并扩展当前审计覆盖，不把旧FAIL改写为穷尽式审计。审计不是Certification，没有原生截图、computed style、DPI、状态或Human Visual Approval结论。

无新的业务 Material Change，无需DCR：将现有presentation映射至已批准Contract值，不改Frozen业务设计、accepted ADR或Candidate-004/005。若实际修复需要改变它们，立即停止并报告，不自行创建DCR。

## 2. Finding → 修复范围

修复授权只能来自最终批准的本候选身份，且逐项受A类Finding限定：

| Finding IDs | 有界动作 | 允许生产文件（将来批准后） |
| --- | --- | --- |
| SF001–014、019–020、022–023、025–033、036–038 | 准确对齐已存在颜色、几何、文字、control与状态视觉角色；静态消费者覆盖以inventory为准 | src/styles.css |
| SF015–018、021、024、034 | Shell/nav几何与线性glyph；直接consumer gap/radius/action glyph修正；不能换Router或提取新架构 | src/styles.css；src/App.tsx仅导航SVG presentation |
| SF021、035 | Sidebar RateRow glyph14px与已存在compact间距 | src/components/layout/SidebarTraffic.css |
| SF021、024、029、031、033、034 | DocumentEditor外围现有共享Button/IconButton、caption和gap映射 | src/components/subscriptions/DocumentEditor.css；不改编辑器尺寸、布局模式或Monaco配置 |

表中SF前缀等于inventory的SF-V02-；仅这些Finding支持的规则可改。允许在同一src/styles.css将消费者连接到既有语义token；不创建新视觉值、第二套主题、UI库、primitive framework或新依赖。没有明确Final值/角色的项目不得顺手修正。

Item title使用Contract 14/500/20；Section title14/600/20；Body13/400/20；Supporting12/400/18；Caption11/400/16。名称看似相似不能机械套值，inventory已声明各consumer角色；N类未判定角色不进入修复。surface标准14/dense10；常规gap6/10/14，Dialog footer8等专项值优先；不对所有数字8/12/16全局替换。IconButton圆形refresh保留50%，普通5px；导航glyph18、action16、status14分别映射。

## Scope

### allow

- 仅上述四个生产文件中与A类Finding一一对应的presentation/CSS声明。src/App.tsx只允许现有navigation icon path/呈现属性，不修改navigationItems的业务id、activePage、handlers、observation或Provider。
- 将来必要验证可使用当前pnpm/Vitest以及本Task Evidence；不得借测试修改生产业务行为。
- ui_scope.pages=[Subscriptions]是本change unit的验收/Golden A目标，不是宣称共享token只影响一个页面。affected_shared_pages完整登记在inventory；其它页只做共享基础回归，不实施其专用composition或宣布其Golden成立。

### deny

- 不改SubscriptionPage.tsx业务/反馈handler、数据与state ownership、DocumentEditor.tsx/Monaco配置、SidebarTraffic.tsx采样/绘图算法、src/lib/**、Router、IPC、任何Rust/Compiler/Domain/Runtime/StateStore。
- 不改TASK-011、TASK-012、Candidate-004/005、历史Evidence；不创建TASK013–017或物化TASK018。未来执行顺序与Task materialization须由现有Orchestrator在用户批准后单独处理，不能自动切换focus。
- 不改Subscriptions grid/业务Card构图、编辑器大尺寸/全屏、Dialog流程、成功反馈timer；C类明确页面专用问题留给后续认证/正常有界变更，不掩盖其存在。
- 不新增功能、值、依赖、通用framework或没有Finding的美化；不commit/push/release/deploy。

## 3. 影响链与身份

Shared Foundation → Subscriptions → 全新Golden Page Certification → TASK-012 readiness → A/Proxies → B/Routing。

src/styles.css及App宿主变化影响Overview、Subscriptions、Proxies、Routing以及未来页面共享宿主；DocumentEditor外围只直接影响Subscriptions。这里不是对其它页全量视觉审计或迁移授权。回归范围记录这些共享消费者，发现专用页面问题只报告，不顺手修改。

当前Subscriptions certification-0.2-001仍为原身份FAIL；修改后其source manifest不再代表当前代码。未来所有包含改动共享文件的Subscriptions/A/B/Task visual identity和截图/Review/Human approval都会stale，必须按新identity重新取证，不能回填旧记录。TASK-011历史功能验收、Candidate004业务设计与Candidate005内容保持原样。

注意Candidate005的external certification_ref当前指向旧FAIL记录。未来新认证必须用新版本化文件；采用新引用须经既有影响分析/新候选评审与批准再物化，不覆盖旧FAIL、不在本轮修改Candidate005或TASK012。影响链不意味着当前引用能自动解锁readiness。

## 4. 验证与Gate

本轮仅做静态穷尽覆盖、Source/Contract/protected manifest身份、Finding-to-scope对应、design validator、独立Technical Design Review和diff检查。Review PASS后停在本候选的Human Design Approval，不运行Certification/readiness/实现，不生成Human APPROVED。

未来修复（另经正式物化及授权）逐项关闭A类Finding并保留定位/实际值；运行pnpm lint、pnpm test、pnpm build（现有lint/build包含typecheck），必要的原生验证覆盖共享基础在light/dark下的Subscriptions与其它共享宿主回归。保持本Task自身Compliance/独立Review/真实Human Visual Gate，不以静态修正关闭交付。

修复完成不等于Subscriptions Certification PASS。必须从头执行新的Contract0.2 certification：新Git/source manifest/hash、全部适用状态、真实Windows Tauri截图与五组DPI/窗口矩阵、当前frontend verification、独立Review、Human Visual Approval。N类逐项在此判断；C类页面专用的已知差异仍可能阻断，尤其成功Notice当前6000ms，不能因为本change unit完成而假装符合2500–3000ms。

不为解决这些页面专用差异扩大本候选。native/state/DPI还可能发现新的页面级Finding，按当时实际Finding停止并提出新的有界变更。最终只有当前完整认证及人工批准成立、且TASK012合法采用新引用后，才可重跑其readiness。

无数据migration；表现层回滚仅回滚本Task允许文件的对应声明，不回滚业务数据；回滚同样需要刷新视觉证据身份。

## Source identity完整性

Candidate source_paths同时包含bootstrap.ts、observability.ts、traffic-trend.ts、subscriptions.ts四个src/lib只读来源，绑定Subscriptions及共享宿主的实际状态/数据呈现。source manifest不等于写权限；生产allow仍只有四个表现文件。依赖状态来源变化同样要求重新判断视觉Evidence freshness。

SF-V02-013同时覆盖styles.css:563–566的SubscriptionCard hover：现状只改border；非selected且非disabled的卡片应消费已批准hover背景。只修该共享状态角色的CSS消费者，不改active/selection/focus业务语义；selected呈现由SF-V02-038约束。
