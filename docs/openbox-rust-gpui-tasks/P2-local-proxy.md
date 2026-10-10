# P2 本机代理闭环

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：目标是从订阅导入到真实代理访问，再停止并恢复网络；先形成 Base OutboundCatalog，再交付最小 Compiler。各功能带最小可操作 UI，主体展示在 P3 补齐。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

**2026-10-08 分期适用范围**：仅未完成卡按[唯一调度政策](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)执行。P2-05 客户端与 P2-08 节点选择/延迟测速/临时实例清理须前期真实实现，使用受控 loopback/自有资源完成局部定向集成；适用持久配置/历史须真实保存和重建回读，UI 绑定真实服务和实际结果，局部基线为 150% 浅色，不能用 Mock/静态配置/禁用按钮完工。P2-09 仅复验真正 SystemProxy 出口及全链路组合、未执行 Native 和其它主题/缩放；P2-08 业务实现及局部验证不延期，TUN 专属生产路径在 P6-01/05 复验。无授权的具体验证记 NOT_RUN/原因，不免除接口、状态与安全控制实现。P2-06 checkpoint 已 commit 但仍 DOING，GUI/Native/Helper 多轮恢复后补，不新增其它 READY 功能 Gate。P2-07 SystemProxy 最后阶段真实实施，沿用全部接管/条件恢复规则；前期 SystemProxy/TUN 开关禁用且不报假成功，其它局部功能不得因缺这两种系统能力而盲目禁用。原依赖、验收覆盖和已 DONE 历史不变，后期只补 Native/完整组合，不承接前期全部实际业务测试。

<a id="obg-p2-01"></a>
## OBG-P2-01 订阅预览、保存与刷新

**类型**：业务与 UI；**依赖**：OBG-P1-02、OBG-P1-03、OBG-P1-04A、OBG-P0-06。**依据/范围**：方案 §8.1；core subscription/application 与订阅设置；API 46–47、56–59。

**泳道 / 写范围**：Core/Config + GPUI；subscription 基础导入/刷新服务、订阅基础表单。

**执行与交付**：复用 parser/normalize，接通 URL/多 URL/粘贴、预览、新增、编辑、删除和刷新；下载使用 P0 确认的显式 Direct 路径，后续由 OBG-P2-05 完成所有路径。

**验收**：

- [x] 预览不持久化；成功、跳过、失败项可解释；保存后重启仍能读取。
- [x] 刷新保留可确认的稳定节点身份与手工信息，不能按同名合并不同节点；失败保留旧有效内容。
- [x] 删除引用产生明确提示；旧预览/下载结果不覆盖新输入或新 epoch。
- [x] 最小订阅界面的实际操作、空/错误/忙碌状态和浅深色通过界面检查（历史功能PASS；Host最终确认build38-final完整订阅页面“符合”，完整页面Visual PASS）。

**当前状态**：DONE；**owner**：已释放 Codex Desktop · Full Subscription UI Parity。P2-01-VISUAL-FULL-PAGE-001 CLOSED（2026-10-07）；Host明确确认build38-final完整订阅页面“符合”，绑定executable SHA256 `de220c173fb4cd70163419e93a54d34324b4dbf53617a87e46b32be8fafcbff3` 与[最终收口](evidence/p2-01/final-closeout-20261007-122027/README.md)。历史功能PASS继续有效；所有历史FAIL/REWORK/build及局部批准保留原结果。

**上一轮局部视觉验收历史（2026-10-06）**：[最终矩阵、构建、回归与清理](evidence/p2-01/visual-parity-20261006-190927/README.md)。build29合并订阅/节点/规则，DNS默认折叠、11px按钮、32px输入及说明，移除Preview按钮/面板，单次SaveDraft复用原Core非持久化Preview/校验/重新下载/一次commit；body自然高度仅溢出滚动。浅深色、真实GUI/重启/cleanup、Host“符合”、Core304/Desktop69/outbound9与check/clippy/fmt/diff、Independent Review PASS。仅已批准字体/局部blur差异；Share/Rules/DNS业务/Node Runtime不提前实现。完整DAG DONE16/ACCEPTANCE0/READY3/TODO42/DEFERRED7，READY为P0-08/P2-02A/P5-06；未启动下游、未commit/push。

**历史真实 GUI 验收（2026-10-06）**：[完整结果与截图](evidence/p2-01/gui-final-20261006-180816/README.md)。Preview/Save/Edit/Refresh/Delete、逐来源 partial、稳定 NodeId/同名不同 endpoint、失败保旧、ReferenceConflict、stale Preview/Refresh generation、Host实际中文IME与三语/重启/cleanup PASS。最终 Core304/Desktop68/outbound9、check/clippy/fmt/diff及独立Review PASS。仅有界修复本页 Retry旧报告、modal焦点/滚动、tab与反馈样式、Name翻译/多URL保存解释。视觉重开期间P0-08 READY、P2-02A/P5-06 TODO；最终当前队列见上文视觉收口。未启动、未commit/push。

**本轮交付（2026-10-06）**：[实现/合同/验证/自查与 GUI 待验清单](evidence/p2-01/README.md)。application-level `preview` 返回无凭据/节点对象的逐来源 DTO；`save_preview` 重新校验输入指纹与 ConfigVersion、重新下载/解析，共用既有 import 候选与单次 JsonStateStore commit。多 URL 按输入顺序分别形成独立 Remote Subscription/Provider（当前 domain 为单 URL 来源），有效来源一起原子保存；失败来源不创建实体，保留逐来源报告。全部失败不提交。Manual 来源明确为 pasted/manual。正式 `refresh_direct` / `edit_direct_with_content` / `delete_at_version` 共用既有 update/edit/delete 合同及 provider replacement；旧入口的 scheduler/失败尝试时间语义保留，正式 Direct 失败不改旧 bytes/metadata。

Direct 消费当前正式 fetch 的 no_proxy、30s 默认总预算、手工 redirect validation、跨源凭据剥离、userinfo/HTTPS downgrade 拒绝和 body limit。P0-06 custom resolver/Via 原型仍 feature-gated，未迁入；不推断 external-TUN physical bypass。Settings → Subscriptions 接入 AppServices、后台 worker、generation/request/epoch 隔离，复用 tokens/shared components/全局 i18n；没有 Compiler、运行、选择、测速、分享或 Via chooser。保留 React 视觉参考。P0-08/P2-02A/P5-06 READY，均未启动，未 commit/push。

**最新完整UI候选（2026-10-07，build37）**：[矩阵、最终构建及独立Review](evidence/p2-01/all-ui-20261007-111729/BUILD37-FULL-UI-CANDIDATE.md)。补只读Share创建弹窗、field gap4、分享长列表及页面/modal分层；真实Light/Dark、Rules上下/DNS/Nodes/Share、滚动、错误/Busy/离页generation、三语留证。Core304/Desktop70/outbound9与九项验证、独立Review PASS；Host Visual PENDING，ACCEPTANCE/OPEN/owner保持，未开放未来条件业务、未启动下游、未commit/push。

**最新窄范围候选（2026-10-07，build38）**：[Source常驻说明移除及真实截图](evidence/p2-01/source-final-20261007-115918/BUILD38-SOURCE-CANDIDATE.md)。只删除正文Direct/TUN hint，不新增占位/帮助UI；P0-06合同与未来能力边界保持。真实metadata保存description保留，重启后两订阅可读；Core304/Desktop70/outbound9与九项检查PASS。build38窄范围Independent Review PASS（不等于Host Visual PASS）。原版空feedback margin及自然高度差异明示，Host整页确认PENDING，ACCEPTANCE/OPEN/owner保持，无下游/commit/push。

### P2-01-VISUAL-FULL-PAGE-001 · CLOSED（2026-10-07；2026-10-06开启）

**开启与返工历史（2026-10-06，按当时结果保留）**：Host指出“目前应该只实现了添加订阅的UI视觉，其他都未实现”。上一轮Visual PASS仅覆盖当时限定实现区域，不能代表完整SubscriptionSettings页面。保留历史业务/Core/功能PASS、旧视觉记录和evidence，不重写。当前P2-01重新ACCEPTANCE；owner：Codex Desktop · Full Subscription UI Parity。完整页面一级视觉区域必须呈现，未来业务以完整禁用视觉壳表示，视觉与业务分别判定。仅最终完整页面Host明确确认后关闭Finding并恢复DONE；P2-02A/P5-06回TODO，P0-08保持READY。

**前轮全控件候选（2026-10-07，build29-final，历史）**：[逐控件新旧矩阵、真实构建与验证](evidence/p2-01/full-reaudit-20261007-094122/BUILD29-CONTROLS-REWORK.md)。按actual OpenBox级联修复primary/ghost/focus、小按钮、完整Rules/DNS说明、saved Nodes table与CSS动画，移除Kit额外modal滑入；真实浅深色最终截图与键盘/URL增删已采集。Core304/Desktop70/outbound9及九项检查、Independent Review PASS；Host完整视觉仍PENDING，不能由工程结果关闭Finding。最后交接遇Mac锁屏待解锁核验，ACCEPTANCE/OPEN/owner与DAG保持，无下游、commit/push。

**Host 按钮增量修复（2026-10-07，build25）**：[Plus 居中及 Share 主色](evidence/p2-01/full-reaudit-20261007-094122/BUILD25-PLUS-REWORK.md)。顶部空label文字按钮改为共享primary IconButton；Share去额外45%透明度，保留disabled/no-handler。双主题真实截图、Add/Cancel、Share bytes无副作用、九项自动验证及独立增量Review PASS。Host整页确认仍PENDING，ACCEPTANCE/Finding OPEN/owner不变；旧build24证据保留。

**完整页面再次复核（2026-10-07，build24）**：[新旧 gap、actual OpenBox 对照及最终构建](evidence/p2-01/full-reaudit-20261007-094122/README.md)。以本机 actual DOM/CSS 重查完整区域，修正全局无背景层次、六列节点 grid、node geometry/status、Share/card header/action spacing、modal自然居中、Rules上下半页及Source输入样式。Host新增“原版没有说明字段”已移除额外UI，既有description编辑保存保留。最终Core304/Desktop70/outbound9及全部规定check/clippy/fmt/diff PASS，Independent Review PASS；真实浅深色Source/Nodes/Rules/DNS/empty/loading/error/busy重新采集。当前工程候选不代表Host Visual PASS；保持ACCEPTANCE、Finding OPEN与owner，最终真实.app保留供Host整页查看。不启动下游、不commit/push。


<a id="obg-p2-02a"></a>

**完整页面修复进度（2026-10-06）**：[本轮完整矩阵/最终build16/功能回归](evidence/p2-01/full-page-20261006-214140/README.md)。Share、五action、折叠健康点、AppState只读节点网格、Source/Rules/Nodes完整编辑架构及未来业务禁用壳已呈现。最终Core304/Desktop70/outbound9和check/clippy/fmt/diff PASS，独立源码Review PASS；浅深色最终真实.app对应截图已采集，待本轮完整页面Host明确确认，保持ACCEPTANCE/Finding OPEN。未启动下游、未commit/push。


**Host完整页面复核（2026-10-06 22:43）**：最终build16 Host回复“仍有偏差”，本轮Visual FAIL/REWORK；Finding继续OPEN、ACCEPTANCE/owner保留，等待具体偏差定位。工程矩阵/自动验证/Independent Review不替代Host验收；旧evidence及结果保留。


**暗色节点重修（build17）**：Host明确暗色节点卡片太亮，直接读取本地原版实际颜色，修复Dark base-200/正文/辅助文字/边框/状态块/hover，Light不变。最终Core304/Desktop70/outbound9及check/clippy/fmt/diff PASS；[实测与当前截图](evidence/p2-01/full-page-20261006-214140/DARK-NODE-REWORK-BUILD17.md)。仍待build17 Host确认，ACCEPTANCE/Finding OPEN，不启动下游。

<a id="p2-01-final-closeout"></a>
### P2-01 最终收口（2026-10-07）

Host已明确确认 **build38-final完整订阅页面“符合”**，来源为本轮用户明确传达的Host最终批准。批准绑定executable SHA256 `de220c173fb4cd70163419e93a54d34324b4dbf53617a87e46b32be8fafcbff3`、[final身份](evidence/p2-01/source-final-20261007-115918/build38-final-identity.json)、[Host展示身份](evidence/p2-01/source-final-20261007-115918/build38-host-identity.json)及[build38候选与截图](evidence/p2-01/source-final-20261007-115918/BUILD38-SOURCE-CANDIDATE.md)。[新批准记录](evidence/p2-01/final-closeout-20261007-122027/host-final-visual-approval.json)补足完整页面Visual PASS；自然高度/空feedback margin及字体/blur技术差异仍明示，未声称精确几何或字体完全一致。未来业务保持原禁用边界。

P2-01 ACCEPTANCE → DONE，Finding OPEN → CLOSED，释放Codex Desktop · Full Subscription UI Parity owner及资源预约。旧FAIL/REWORK、局部批准、PENDING与所有build记录按当时结果保留。完整68-card DAG按原显式依赖重算：DONE16 / READY3 / TODO42 / DEFERRED7，ACCEPTANCE/DOING/REVIEW/BLOCKED均0；READY仅P0-08/P2-02A/P5-06，均未领取/启动。见[最终证据与验证](evidence/p2-01/final-closeout-20261007-122027/README.md)。

cleanup NOT_RUN：沙箱拒绝`ps`进程身份查询（Operation not permitted），无法确认旧PID当前归属；没有终止app/fixture或删除app/隔离root/证据，资源保留。owner释放不表示现场已清理。未接触用户LAN/OpenBox、正式状态、Runtime/TUN/SystemProxy/helper/admin；不改产品源码，不执行commit/push/fetch/pull/rebase/reset/restore/clean/stash，不运行产品构建/测试。

<a id="obg-p2-02a"></a>
## OBG-P2-02A Base OutboundCatalog 与 OutboundId

**类型**：领域目录；**依赖**：OBG-P2-01。**依据/范围**：方案 §6.2、§7、§8.2；core domain/application 的出口目录。

**泳道 / 写范围**：Core/Config；domain/application OutboundCatalog/OutboundId 公共契约；单一 owner。

**执行与交付**：从已保存订阅/节点事实派生 Base OutboundCatalog，提供稳定 OutboundId、种类、显示信息、可用状态与引用查询；明确至少包含 Node、implicit provider/subscription group、首个闭环所需的 selector/urltest 语义，以及 Direct、Block。Domain 主模型使用 Direct / Block；外部/内核的 reject 只在导入/适配边界映射到 Block。目录是派生读模型，不再持久化另一份出口真相。为 Group/Failover 和 ChainProxy 的后续注册固定最小公共契约。

**验收**：

- [x] Node、隐式 provider/subscription group、selector/urltest、Direct、Block 均由目录定义并可查询；同名/重命名/刷新不混淆身份；删除或禁用后的引用结果明确，显示名不能作为 ID。
- [x] Base 目录的 self/cycle/dangling 校验契约统一；后续注册沿用同一图校验，不能由 Routing 或 UI 各写一套。
- [x] **PASS（本层验收解释，Host确认）**：现有 Compiler 与 selected/runtime projection 消费 Catalog/共享解析契约；`OutboundQueryService`/`OutboundQuerySnapshot` 提供未来消费者的唯一查询 seam，未知目标不猜测为 Direct；隐式组与 selector/urltest 已包含，无伪造 P4 高级 Group/Failover/Chain。P2-08 的正式 ProxyService/基础 Proxy UI 必须消费该 seam，不得建立平行出口列表；实际业务/视觉集成仅属于 P2-08 验收，本卡不宣称已测试 UI。

<a id="p2-02a-closeout"></a>
### P2-02A Host 独立审查与范围收口（2026-10-07）

**状态：DONE；Core/Config owner 已释放。** Host确认当前产品代码 SHA 与交付证据一致；独立审查未发现可操作代码 bug；Host在当前源码上运行 Core 313/313、core check、all-targets clippy `-D warnings`、workspace fmt 与 `git diff --check`，全部 PASS。来源为用户本轮明确转达的 Host 结果；本次仅文档/evidence收口，没有重跑产品构建或测试。

**第三项 PASS 的精确解释**：P2-02A 交付目录和解析契约；现有 Compiler、selected/runtime projection 已消费它，`OutboundQueryService`/`OutboundQuerySnapshot` 是后续唯一查询 seam。正式 ProxyService/最小 Proxy UI 明确由 P2-08 交付，而 P2-08 显式依赖 P2-02A；若要求 P2-08 的真实集成先于 P2-02A DONE，会形成验收依赖环。因此本层第三项通过；P2-08 必须消费上述 seam，禁止自建另一份出口列表。实际 ProxyService/UI 行为及视觉集成仍在 P2-08 验收，当前保持 NOT_RUN，未宣称“UI tested”。这是 Host 批准的作用域/验收解释，不是产品代码修改，也不修改任何依赖。

`P2-02A-CONSUMER-001` **CLOSED：SCOPE_RESOLVED / DEFERRED_CONSUMER_INTEGRATION**。该关闭只消除本卡对下游 UI 集成的错误前置要求，不把未实现/未测试的 P2-08 能力改为 PASS。初轮编译 FAIL、311 PASS/1 FAIL、既有实现者自查措辞及 ACCEPTANCE 历史完整保留于下方初次交付和旧 evidence；新增 [收口证据](evidence/p2-02a/closeout-20261007-124330/README.md)单列 Host 独立结果与当前状态。

不变依赖重算68卡：**DONE17 / READY3 / TODO41 / DEFERRED7**，ACCEPTANCE/DOING/REVIEW/BLOCKED均0；READY仅 **P0-08、P2-02B、P5-06**，均未领取/启动。P2-02B前置P2-02A/P0-04均DONE；P2-08仍缺P2-04/P2-05，保持TODO。无下游开工；未 commit/push/fetch/pull/rebase/reset/restore/clean/stash。

<a id="p2-02a-delivery"></a>
### P2-02A 初次实现交付（2026-10-07，历史）

**状态：ACCEPTANCE；owner：Codex Core/Config（单一 owner 保留，交 Host Review）。** 仅领取 P2-02A；不启动 P0-08/P5-06/P2-02B/P2-08/P4，不改变依赖或验收范围。

- Domain 新增 `OutboundId::{Node(NodeId),Pool(PoolId),Direct,Block}`；`OutboundCatalog::from_state/from_runtime_intent`、`list/get/resolve/require_available/validate_graph`。entry 返回 kind、display_name、pool_kind、provider/subscription 来源、成员引用、selection 事实及显式 availability。没有新持久字段/serde/schema；无 sing-box tag；无 Group/Failover/Chain 业务实例。
- `ImplicitProvider` 直接消费已有 pool；Manual→Selector、UrlTest→UrlTest，选中节点必须属于实际成员，probe_url/interval/tolerance 原样保留。禁用 pool、空成员、缺失 provider/subscription/node、过滤/选择失效均明确报告；lookup Missing、resolve Unavailable/Unconfigured，不猜 Direct。当前 Subscription/Provider/Node 没有独立 enabled 字段，不能把 auto-update 或未选中订阅解释为禁用；本轮不扩 schema。
- `validate_outbound_graph<Id>` 接受稳定 ID→引用边，统一拒绝 Duplicate/Dangling/SelfReference/Cycle。后续注册交同一机制验证；正常 AppState 仅包含已有 Node/Pool/Direct/Block。`StateValidation` 引用规则保留，成员计算共用 `resolve_pool_members`。
- 现有消费者：`RuntimeIntent::from_state`、`selected_subscription` 的 pool/target projection 和 Compiler。选定订阅的既有 `runtime-active-*` 适配及 Unconfigured 默认策略保留，目录本身不新增隐式默认或伪造高级组。Compiler 消费目录成员/selection/RouteTarget 解析，仅基础身份合法性接线，标签仍在 singbox 边界；无 P2-02B 的 check/apply/cache/DNS 新能力。
- application 新增 `OutboundQueryService::snapshot()` 和同版本 `OutboundQuerySnapshot::{list,get,resolve}`，复用 SnapshotService/gate，为 P2-08 提供查询 seam；不新增缓存、后台任务或持久化。**第三验收项仅 Compiler/现有投影/查询 seam PASS，正式 ProxyService 与基础 Proxy UI 尚不存在，真实 UI 消费 NOT_RUN，保留未勾选。** Host Review 决定该 gap 的范围处理前不伪造 DONE，不修改依赖。

**验证 PASS**：`cargo test -p veyra-core --offline -- --test-threads=1`：313/313（新增9，含 catalog7/query1/compiler1；doc-tests0 不计用例）；`cargo check -p veyra-core --offline`；`cargo clippy -p veyra-core --all-targets --offline -- -D warnings`；`cargo fmt --all -- --check`；`git diff --check`。全量覆盖 selected_subscription/subscription_management/state validation/compiler 回归。Compiler 改接前固定16个既有输入×Pool/Direct/Block=48个配置字节摘要，改接后完全匹配；原有名称/排序/tag稳定性测试继续 PASS。查询测试证明 state/schema 字段不增加，查询前后磁盘与序列化字节不变，刷新后读到当前 SnapshotVersion。

**历史 FAIL 保留**：首轮新增测试误用私有 `state_file()`，编译 FAIL；次轮311 PASS/1 FAIL，发现空池含旧选择时错误优先级改变，修复后恢复 `EmptyPoolMembership`。未删/跳过/弱化旧测试。验证包含既有文件/mock/loopback fixtures，无真实 sing-box child、系统代理、TUN/helper/正式状态或公网操作。Desktop源码/seam未改，Desktop checks N/A；Proxy UI及正式消费者验收 NOT_RUN。

**code-delivery-review 实现者自查**：按 correctness/contract-data/verification/Rust 关注面重新检查本轮代码、调用边界、测试与基线差分，未发现未修复代码 Finding；这是实现者自查，非独立审查。交付 gap `P2-02A-CONSUMER-001 OPEN`：正式 ProxyService/基础 Proxy UI尚未实现，查询 seam不能冒充真实 UI集成。既有 dirtytree无关文件保持原字节，本轮仅9个Core文件与3个任务文档；local-only [证据](evidence/p2-02a/README.md)包含失败/最终日志、源码身份、自查与DAG。未 commit/push/fetch/pull/rebase/reset/restore/clean/stash。

**DAG**：68卡 DONE16 / ACCEPTANCE1 / READY2 / TODO42 / DEFERRED7，DOING/REVIEW/BLOCKED0；READY仅P0-08/P5-06且未领取；P2-02B/P2-08等下游保持TODO。

<a id="obg-p2-02b"></a>
## OBG-P2-02B 最小可用 Compiler

**当前状态**：DONE；**owner**：已释放 Codex Core/Config。P2-02B-RUNTIME-PROJECTION-001 CLOSED，Host independent review PASS；见[最终收口](#p2-02b-host-closeout)。首次DONE、重开与修复候选历史保留。

**类型**：配置编译；**依赖**：OBG-P2-02A、OBG-P0-04。**依据/范围**：方案 §7；core compiler/domain；API 60–61 对应配置消费。

**泳道 / 写范围**：Core/Config；compiler 基础配置与 fixtures；公共模型/目录交 owner。

**执行与交付**：将 Base OutboundCatalog 定义的 Node、implicit provider/subscription group、selector/urltest、Direct、Block 语义编译成锁定 sing-box 格式；Compiler 不拥有第二份出口定义。支持基础路由/DNS、loopback mixed、控制器与 cache_file；封闭模型与固定资源；加入代表性 Compiler fixtures。完整 Group/Failover 与 Chain 的编译分别在 P4 扩展。

**验收**：

- [x] 目录中的节点、隐式组、selector/urltest、Direct、Block 按同一 OutboundId/引用语义编译，Compiler 不自建出口目录；合法模型生成锁定内核可接受的配置；统一目录报告 self/cycle/dangling，Compiler 拒绝不允许字段。
- [x] 保存不启动 `sing-box check`，显式应用前才检查同一候选；check 失败不停止旧实例。
- [x] cache path/tag 与身份稳定；不输出锁定版本已移除的 DNS/缓存字段。
- [x] IPv6、rejectQuic、directForNodes 等本阶段开放选项映射真实规则；尚未支持选项明确拒绝应用。
- [x] Runtime health test URL（含 directTestUrl）按 profile 消费，独立于 UI latency preference test URL；后续 group health URL 只使用其明确的覆盖/继承规则。

<a id="obg-p2-03"></a>
## OBG-P2-03 手动代理 Runtime 与服务状态

**类型**：进程集成与 UI；**依赖**：OBG-P2-02B、OBG-P1-03。**依据/范围**：方案 §4.3、§7.3、§8.7、§11.3；core runtime/platform/macos、后端服务卡；API 10–11、15–16。

**泳道 / 写范围**：Runtime/Platform + GPUI；runtime 启停/实例公共契约与服务卡；Runtime 契约单一 owner。

**当前状态（2026-10-07）**：DONE；Runtime/Platform + GPUI owner、Runtime 公共契约 owner 与本卡预约已释放。Host明确确认当前Basic2 build功能与视觉“符合”，绑定executable SHA256 `59280c3146502e1b9474d08f976d807cfae4390751493badf9f89aeca9b325c4`；见[最终收口](#p2-03-final-closeout)。历史候选、失败及BUSY-001记录保留；本轮仅文档/evidence，不操作app/runtime资源。

**实施约束**：正式运行配置必须使用 P2-02B `compile_product(ProductCompileRequest { state, runtime_intent, default_outbound, resources })`，显式消费 `application::selected_subscription::project_selected_runtime()` 的 `runtime_intent` 和 `projected_default_target`（转换为领域 `OutboundId`）。不得继续使用 `application/runtime.rs` 现有 ObservationOnly `compile(...)` 作为正式运行配置；运行资源由 Runtime owner 显式提供，tag仍仅Compiler adapter内部。

**执行与交付**：桌面管理手动代理 child；check→启动→地址发现→鉴权就绪→停止，统一操作互斥和实例身份。基于 P1-03 的 AppServices/状态桥和基础服务壳接通状态、版本、启停/重启与操作反馈；Runtime 不依赖 P1-06 托盘完成，托盘未接通时仍显示准确不可用状态。

**验收**：

- [x] 同一候选配置完成一次受控真实访问与停止，端口和 child 归属有证据。
- [x] 重复启动、check 失败、端口冲突、超时、异常退出不误报 Ready；只停止自有实例。
- [x] 内核停机时仍可编辑；保存/运行版本与实际状态分别展示。
- [x] Host最终确认当前Basic2 build功能与视觉“符合”（绑定59280c31…及最终evidence），点击已提交不冒充启动完成；托盘与 Runtime 联动仍在 P2-09 经 P1-07 组合验收。

<a id="obg-p2-04"></a>
## OBG-P2-04 选择持久化、缓存与最后成功配置

**当前状态：DONE；P2-04-PENDING-PERSISTENCE-001 与前轮 Host P1/P2 Finding 均 CLOSED。Runtime/Platform 及 Runtime 公共契约 owner/本卡预约已释放；仅文档预约释放，不表示已有 app/root/child 被操作或清理。** 见 [Host FINAL ACCEPTANCE](#p2-04-final-host-closeout)。

**类型**：运行恢复；**依赖**：OBG-P2-03。**依据/范围**：方案 §7.5–7.6、§8.2.1；core runtime/storage。

**泳道 / 写范围**：Runtime/Platform；runtime 选择/恢复、storage manifest/cache；公共契约交 owner。

**执行与交付**：串行选择入口、pending/已确认状态、单一 cache writer；落盘 last-applied manifest 和必要资源；失败一次回退与两种恢复入口。

**验收**：

- [x] 控制器确认与业务/缓存持久化失败分别报告；重启对账完成后才报告应用就绪。pending修复自动验证与绑定源码 Native PASS；Host FINAL ACCEPTANCE PASS，Task DONE、Finding CLOSED，历史结果保留。
- [x] 已保存 12/最后成功 11 时可按用户选择恢复；停止后 applied=None，不沿用旧 Ready。
- [x] manifest 写失败显示实际运行成功及恢复记录失败，保留旧记录和回退材料。
- [x] 缓存仅在 writer 退出后快照/交接；新代际或资源缺失不静默恢复，旧 PID 不作当前身份依据。

<a id="obg-p2-05"></a>
## OBG-P2-05 下载、测速和 IP 查询的出站策略

**最新 Host 验收（2026-10-10）：P2-05 DONE，`SCOPED_ACCEPTANCE_WITH_EXPLICIT_DEFERRED_PLATFORM_VERIFICATION`，本卡 owner 释放。** 正式客户端和本轮局部功能/Native/GPUI已验收，完整决定、源码/收据SHA及延期归属见[Host收口](P2-05-host-closeout.md)。原第2项保持未勾选：物理Direct UNVERIFIED、SystemProxy ON NOT_RUN、生产TUN/完整组合由P2-07/P2-09/P6-01/P6-05承接；不将后期卡反向作为本卡依赖。下方2026-10-09/10交接及ACCEPTANCE说明均为决定前的历史原文，保留旧用户决定与失败，不覆盖本次Host明确收口。

**P2-05 本轮最终实机结果（2026-10-10，ACCEPTANCE）**：本候选唯一指定 Native 1 PASS（exit0），定向复跑 Core50 PASS、Desktop build/all-targets Clippy/fmt exit0；真实订阅添加/刷新/失败保旧/保存草稿重试/重启/语言草稿、主备Controller选择、P3日志、同PID托盘恢复与正常退出通过。独立Review及完整收据见[本轮实机验收](P2-05-desktop-acceptance.md)。当前默认macOS显示配置经用户确认，非额外150%设置；物理Direct UNVERIFIED、SystemProxy ON NOT_RUN、历史HTTP503和Legacy Clippy FAIL101保留。后续P2-07/P2-09/P6-01/P6-05组合单列，旧等待组合决定未取消，Host收口前保持ACCEPTANCE/owner，无commit/merge/push/下游。68卡DONE30/ACCEPTANCE1/DOING1/READY0/TODO29/DEFERRED7，P2-06 DOING/Runtime owner不变。

以下整合快照及原验收勾选保留；不自行勾选系统组合或改DONE。

**2026-10-10 当前整合 / ACCEPTANCE**：指定 `dev/p2-05-main-integration`（bda8be5）有界移植 e530837 与旧未提交 Native fixture 修复，保留最新 P4-03/Observation 和 P2-06 原 owner。225 个唯一定向 PASS、三包 all-targets Clippy/check/build/fmt 通过；Legacy Windows资源 Clippy FAIL101，本候选 Native/GUI NOT_RUN，物理Direct旧UNVERIFIED/SystemProxy ON旧NOT_RUN。用户等待完整网络组合的决定不取消，不DONE/解锁/commit/merge/push。见[精确结果、分期判断与Host补验](P2-05-integration-acceptance.md)。以下原验收勾选不改写。

**类型**：网络客户端；**依赖**：OBG-P2-03、OBG-P0-06。**依据/范围**：方案 §8.9；core 下载/GeoIp/控制器客户端；API 30。

**泳道 / 写范围**：Runtime/Platform；下载/GeoIP/测速网络客户端与出站策略。

**执行与交付**：落实 Direct/ViaRunningProxy、独立引导解析、超时/重定向限额与脱敏；控制器只访问当前实例 loopback；三种 GeoIp provider 正常归一化。

**验收**：

- [x] 初次订阅、无有效代理及明确选择代理下载的路径符合策略；失败不静默换路。
- [ ] 系统代理下真实出口符合设计；TUN 专用规则保留 P0 证据并在 P6 用生产实现复验。
  本项平台子验证延期，原覆盖保留且清单仍显示未完成：SystemProxy ON NOT_RUN由P2-07/P2-09承接，物理Direct UNVERIFIED及正式受管TUN由P6-01/P6-05复验。按2026-10-08分期规则与Host本轮明确决定，本卡已完成的客户端/局部验收可DONE；本项没有假勾PASS，详见[逐项结果](P2-05-host-closeout.md#原卡逐项结果与延期归属)。
- [x] 跨源不传递认证头，凭据 URL 不降级明文；日志/诊断无订阅秘密。
- [x] 测速/IP 请求使用指定目标路径，失败保留未知状态；地理查询不发送多余凭据。

<a id="obg-p2-06"></a>
## OBG-P2-06 生产 helper 与最小 IPC

**类型**：权限宿主；**依赖**：OBG-P2-03、OBG-P2-04、OBG-P0-05。**依据/范围**：方案 §11.2.1；新 helper、core platform/macos 与受控 DTO。

**泳道 / 写范围**：Runtime/Platform；helper/IPC、platform/macos；共享 DTO/Runtime 契约交 owner。

**执行与交付**：把 P0 launchd/Unix socket 原型接入 Runtime，支持握手、启停、状态、选择/事件和系统代理请求；受控路径/配置；普通用户 SystemProxy child；有界请求结果留存。

**验收**：

- [ ] Hello 无 instance ID 可用；对端系统身份/版本/资源大小校验失败时拒绝修改。
- [ ] 重复 Start 不创建第二个 child；超时/重连查询真实操作，同一进程会话不因连接断开而重建。
- [ ] 任意执行路径/PID/命令、过期实例请求被拒绝；不添加永久请求日志。
- [ ] 管理员安装/卸载、root 拥有目录、child 实际用户及控制器事件通路有真实证据；授权拒绝仍可手动代理。

**2026-10-09 观测接线增量**：P2-06 保持 DOING。正式 ManualRuntime/两个生产 Port 已接 P3-01 唯一 ObservationService；普通用户锁定真实内核四WS/三指标重连/替换/Stop隔离通过，root/GUI NOT_RUN。未修改本卡验收勾选或P2-07状态。见[本轮交付](P2-06-observation-integration.md)。

<a id="obg-p2-07"></a>
## OBG-P2-07 系统代理、多服务与崩溃恢复

**类型**：平台与 UI；**依赖**：OBG-P2-06、OBG-P2-05。**依据/范围**：方案 §11.1–11.3；helper SystemConfiguration 适配、后端开关/恢复反馈。

**泳道 / 写范围**：Runtime/Platform + GPUI；SystemConfiguration 恢复记录/owner 交接与系统代理控件。

**执行与交付**：按 Network Service ID 管理原值/本应用值/实际值；完整代理字段组、写前恢复记录、回读与条件恢复；网络变化串行处理；接通系统代理开关和 owner 交接。

**用户已批准的产品决策（P2-06 round11登记，仅P2-07实施）**：用户主动开启SystemProxy时，允许接管其他软件当前已设置的代理，不因已有代理而拒绝。以Network Service ID识别服务，写前持久保存HTTP/HTTPS/SOCKS、PAC、自动发现、绕过名单的原始状态（含缺失/启停语义）；Veyra自身check/认证Ready/选择对账完成后，才核对预期实际值、条件应用并回读。关闭时仅恢复实际仍匹配Veyra managed的相关字段组；其它软件后来修改的状态保留，报冲突，不自动抢回、不无限重试。按服务分别记录未写/成功/失败/冲突/已删除；部分失败只回退可确认仍属于本次托管的修改，未知结果保留恢复材料，禁止笼统报告全部恢复。新服务先独立快照，服务切换不覆盖旧快照，不按显示名寻找替代服务。此决策不是本轮实现：P2-07仍TODO，依赖P2-06/P2-05，不领取owner，不执行SystemConfiguration。

**验收**：

- [ ] HTTP/HTTPS/SOCKS、PAC、自动发现和例外列表正确接管/恢复；外部修改保留并报告。
- [ ] Wi-Fi/有线切换、新服务、删除服务及部分失败都有准确结果；不会按显示名误操作。
- [ ] GUI 实际退出后 helper 清理网络和 child；恢复失败仍指向本实例时保留必要端点及 RecoveryRequired 资料。
- [ ] 手动↔SystemProxy 交接无双 writer/双实例；重启中断可解释；开关及部分失败状态经真机和界面验证。

<a id="obg-p2-08"></a>
## OBG-P2-08 最小代理选择与节点测速界面

**类型**：业务与 UI；**依赖**：OBG-P2-02A、OBG-P2-04、OBG-P2-05、OBG-P1-04B。**依据/范围**：方案 §8.1–8.2、§9；core latency/proxies、代理页/订阅节点测试；API 25–29、48。

**泳道 / 写范围**：Runtime/Platform + GPUI；Proxy service、latency 服务及最小代理选择/测速 UI。

**执行与交付**：接通代理快照、手动选择、单节点/组测速及草稿节点测试；接入 Base OutboundCatalog 的 Proxy service；稳定身份、局部忙碌和历史结果；临时实例不替换主实例。

**目录消费约束（P2-02A Host收口）**：正式 ProxyService 与最小 Proxy UI 必须消费 `OutboundQueryService`/`OutboundQuerySnapshot` 这一唯一查询 seam（底层 Catalog/共享解析契约），禁止建立平行出口列表或以显示名重建身份。此处的真实业务、操作与视觉集成由本卡验收；P2-02A 的接口交付不代表本卡已实现或 UI 已测试。见[P2-02A分层验收解释](#p2-02a-closeout)。

**验收**：

- [ ] 选择成功的新连接使用所选出口；失败不伪装成功，保存失败与实际选择分开显示。
- [ ] 测速结果不覆盖刚选择的节点；未测、超时、失败、零值分别展示。
- [ ] 测速消费 P1-04B 的 UI latency 偏好与阈值，Runtime health/group health 配置不被覆盖。
- [ ] 草稿测试成功/取消/超时均清理独立实例、端口和缓存，主实例继续正常。
- [ ] 卡片选择与测速按钮互不误触，最小界面经过实际交互和浅深色验收。

<a id="obg-p2-09"></a>
## OBG-P2-09 首个可用 macOS 版本验收

**类型**：里程碑；**依赖**：OBG-P1-07、OBG-P2-01、OBG-P2-02A、OBG-P2-02B、OBG-P2-03、OBG-P2-04、OBG-P2-05、OBG-P2-06、OBG-P2-07、OBG-P2-08。

**泳道 / 写范围**：组合验收；所列依赖的证据/页面组合与本卡验收记录；修复先预约相关 owner 写范围。

**执行与交付**：固定构建，走通导入→保存→启动→选择→真实访问→开启/关闭系统代理→停止→重开恢复；提交必要审查和实际验收材料。

**验收**：

- [ ] 完整用户流程和代表性失败路径有同一构建的真实证据，网络设置恢复可核对。
- [ ] 检查配置/选择版本、manifest、cache 与 child 归属，未遗留本次资源。
- [ ] 所有已开放 UI 完成实际操作检查；TUN/高级功能尚未交付的状态准确，首个可用版不称为全量完成。

<a id="p2-02b-closeout"></a>
### P2-02B 最小产品 Compiler 交付（2026-10-07，历史）

**状态：DONE；Core/Config 单一 owner 已释放。** 本轮仅领取 P2-02B，五项验收 PASS；无 UI/真实 Runtime child/平台网络操作。现有大 dirty tree 保留，无 commit/push/fetch/pull/rebase/reset/restore/clean/stash。local-only [gap/API/结果与自查](evidence/p2-02b/README.md)，[初始差距与设计决定](evidence/p2-02b/GAP-ANALYSIS.md)。本段是前轮历史验收摘要，初轮编译/fixture/Clippy FAIL 与各轮 PASS 日志分别保留。

**API / 文件**：`SingBoxCompiler::compile_product(ProductCompileRequest { state, default_outbound: &OutboundId, resources })` 消费同一 AppState/Catalog/RuntimeIntent；没有第二套出口事实。`ProductRuntimeResources` 明确携带 `LoopbackListener(SocketAddr)` 的 mixed/controller（0 为动态）、`ManagedCacheFile` 的受管 absolute path/stable cache_id/显式 store 标志。运行期资源不持久化；tag 仅在 Compiler adapter。产品路径复用既有 15 协议/node/selector/urltest/封闭验证；ObservationOnly 原入口输出保持，48-config combined digest `4204c8cc2383c67fbfc23d1f71d49fe124c773958deefb8532c434c3d1faf167` 回归 PASS。产品 Block 转 route reject，mixed 不固定 1080；controller secret 仅 finalize 注入，Plan/GeneratedConfig Debug 脱敏。cache exact fields 为 enabled=true/path/cache_id/store_fakeip=false/store_dns=false；当前不生成 FakeIP、不持久恢复 DNS，旧 clash_api 缓存字段 absent。

新增 `singbox/compiler/product.rs`、`product_tests.rs`、`examples/p2_02b_candidate.rs`、`tests/fixtures/compiler/` 输入/两份完整 expected JSON/说明；修改 `singbox/compiler.rs`/`mod.rs`、domain `profile.rs`/`desktop_behavior.rs`/`error.rs`，`singbox/runtime.rs` 仅加强/新增回归，生产 lifecycle 未修改。无新依赖/Cargo/DTO/UI/存储实现修改。

**选项 / health**：Profile 增加 ipv6Proxy/node 默认、directForNodes=true、rejectQuic=false、testUrl/directTestUrl typed RuntimeHealthUrl。IPv6 off 对候选 DNS ipv4_only 与内核 IPv6 route reject 生效，on/node 按同一规则处理 IPv4/IPv6；不改系统 IPv6。ipv4/bypass、非默认 Tun、DNS extras、活动非 IP DNS upstream、split 的非域名站点规则与 split+Block default 类型化 UnsupportedOption；未知 option/null/非法 URL 严格拒绝，不静默忽略。rejectQuic 仅在按优先级确定的 proxy UDP443 路径拒绝，Direct 不受影响。directForNodes 的 subscription hostname/node hostname/IP 引导规则在 sites 前，URL query/凭据不进入规则；不能作为 Rust reqwest/TUN bypass 证据。基础 split DNS 按真实 Node/Pool 派生 proxy upstream detour，Direct default 使用 direct DNS。Profile custom 域名规则与基础 state routes 均通过 Catalog 解析。`SingBoxPlan::runtime_health()` 明确给出 proxy/direct 两个 URL，独立于 UI latency；pool 已有 probe_url 不被覆盖，Group health 留 P4。

**schema / 保存**：CURRENT_SCHEMA_VERSION/StoredStateV9 仍为 9；仅新增字段有严格 missing defaults，原有必填字段不放宽，null/未知 enum/凭据 URL 不通过。真实 JsonStateStore 测试证明旧 v9 读取不重写 bytes、不改变 epoch/双 revision/业务事实；v8→v9 原迁移保留事实/生成原始备份，二次 load 稳定，保存后再读稳定。Profile/State/Subscription 普通保存仍 SavedOnly，无 Compiler/SidecarPort/check/child 依赖；应用拒绝不影响意图保存。产品 GeneratedConfig 通过既有显式 SidecarRuntime check seam；Mock check fail 对不同候选保持旧 child/旧配置 bytes，事件增量仅 Check，不能 Stop/Prepare/Run。

**真实 check**：下载 official darwin-arm64 archive，执行前 SHA256 匹配 `a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`；binary SHA256 匹配 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，version `1.14.0`、darwin/arm64、revision `0b8995879f29a9b98ee027bc17b75e101445b238`。同一产品 fixture 生成 base/options/Direct/Block 四个 finalized GeneratedConfig，0700 temp root/0600 config、随机临时 secret，每个固定 binary `check -c <candidate>` 20s 有界、exit0 PASS。保留原始 candidate 摘要与脱敏 JSON；未 run、未创建 cache、没有真实 child lifecycle/Ready 结论。

**验证 / Review**：P2-02B 定向 **14/14**、Core **327/327**（既有313全部保留，包含 compiler/selected_subscription/state/subscription）PASS；`cargo check -p veyra-core --offline`、`cargo clippy -p veyra-core --all-targets --offline -- -D warnings`、`cargo fmt --all -- --check`、`git diff --check` PASS。长命令有180/240s外层预算，真实 check 20s；Core全量含既有文件/Mock/loopback fixtures，不称纯单测，doc-tests0不计用例 PASS。按 code-delivery-review 的 correctness/contract-data/verification/Rust 完成实现者自查；首版跨 pool DNS detour Finding已修正/回归，最终无未解决可操作 Finding，非独立审查。

**DAG**：68-card DONE18 / READY4 / TODO39 / DEFERRED7，其余0；依赖不变、无环。READY=P0-08/P2-03/P4-02/P5-06，均未领取/启动。P2-08/P4其他/P5其他/P6不启动；P2-03 仍负责正式 owner/child/Ready/服务 UI。本轮候选 secret/config、下载 archive/解包 binary 与临时脚本已按本次路径清理；历史 P2-01 app/fixture 的 cleanup NOT_RUN 记录不改写。

<a id="p2-02b-runtime-projection-fix"></a>
### Finding P2-02B-RUNTIME-PROJECTION-001（2026-10-07）

**修复候选历史状态：FIXED_PENDING_HOST_REVIEW；P2-02B：ACCEPTANCE；owner：Codex Core/Config（当时保留，等待 Host 独立 review）。** 当前 Finding CLOSED、P2-02B DONE、owner释放，见[Host最终收口](#p2-02b-host-closeout)。 首次导入允许 default_target=Unconfigured，selected projection生成runtime-only pool；产品Compiler却用AppState catalog校验默认出口并重建intent，拒绝合法投影。directForNodes同时引入全部保存订阅host，超出实际运行闭包。责任归P2-02B契约，不转交P2-03。

修复范围：显式AppState + RuntimeIntent + projected OutboundId + resources；运行成员/默认出口/Profile custom/runtime routes复用runtime catalog与既有graph校验；bootstrap仅实际节点及provider对应Remote订阅host。持久pool fixture/ObservationOnly digest保持。旧DONE、327 Core、locked-check-final.json及所有历史evidence原样保留。

开始时 P2-02B DONE→DOING / Finding OPEN，P2-03/P4-02 READY→TODO；修复后只转 ACCEPTANCE / FIXED_PENDING_HOST_REVIEW，不恢复 DONE。修复候选当时68卡 DONE17/ACCEPTANCE1/READY2/TODO41/DEFERRED7，其余0；P0-08/P5-06保持READY，未领取。仅Core/Config，不启动下游/UI/真实run。

**修复 API / 闭包**：`ProductCompileRequest { state, runtime_intent, default_outbound, resources }` 明确区分持久事实与实际运行投影。Compiler 不再调用 `RuntimeIntent::from_state()`；完整 configured-default 调用方仍可显式使用该函数。运行 identity/成员/default、Profile custom rules、runtime routes 均复用 `OutboundCatalog::from_runtime_intent()` / 既有 `validate_outbound_graph`；AppState.validate仅检查持久事实。runtime-only pool正常生成selector/default final，不持久化；OutboundId→tag仍仅adapter内部。directForNodes只取实际节点server及其provider对应Remote订阅URL host，去query/凭据，IPv4/IPv6精确/32、/128。无第二套graph、无Direct fallback；schema9/health/rejectQuic/IPv6/资源选项未无关改动。

**新增5项回归**：真实3订阅active+Unconfigured首启、filtered custom跨订阅闭包及runtime-only route、实际intent节点host与Remote IPv4/IPv6、闭包外默认/空成员/悬空成员/Unconfigured与闭包外runtime route拒绝、已保存但闭包外Profile custom rule拒绝。原14项保持：定向19/19，projection5/5，P2-02A catalog7/7，Core332/332全部PASS；check/all-targets clippy/workspace fmt/diff PASS。原base/options expected JSON未修改、完整配置比较PASS；ObservationOnly48-config digest `4204c8cc2383c67fbfc23d1f71d49fe124c773958deefb8532c434c3d1faf167` PASS。初次测试代码误用Result.is_none导致编译FAIL，修正后PASS；原FAIL日志保留，不改写。

**新 fixed-kernel check**：official archive SHA256 `a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`先匹配，再匹配binary `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。base/options/direct/block/真实selected projection五个新candidate全部20s `sing-box check -c` exit0；root0700/config0600，随机secret只在私有临时config，证据仅脱敏副本及原candidate digest；没有run或cache生成。临时archive/binary/config/secret清理PASS，原locked-check-final.json与旧证据未覆盖。

**实现者自查**：按 code-delivery-review 的correctness/contract-data/verification/Rust复查selected projection→compiler、运行闭包、bootstrap host隔离、custom route closure、旧fixture/digest与locked check，未发现剩余可操作Finding；这是实现者自查，非独立review。现有dirty tree无关内容保持；未commit/push/fetch/pull/rebase/reset/restore/clean/stash。

本轮 local-only [报告与验证](evidence/p2-02b/runtime-projection-fix-20261007-131829/README.md)。

<a id="p2-02b-host-closeout"></a>
### P2-02B Host 独立 review / 最终收口（2026-10-07）

**当前状态：DONE；Finding P2-02B-RUNTIME-PROJECTION-001：CLOSED；Codex Core/Config owner已释放。** Host明确独立review PASS，批准仅文档/evidence收口；FIXED_PENDING_HOST_REVIEW→CLOSED、ACCEPTANCE→DONE。首次DONE、重开DOING、初始FAIL与修复候选ACCEPTANCE及其evidence按当时事实保留，不覆盖旧证据。

**Host独立复核**：当前source SHA与runtime-projection-fix evidence一致；重新运行P2-02B19/19、projection5/5、P2-02A catalog7/7、Core332/332、core check/all-targets clippy -D warnings/workspace fmt/diff全部PASS；重新生成base/options/direct/block/selected，secret/cache path脱敏归一化后，与修复轮fixed-kernel checked redacted candidates五个全部语义一致。selected具有pool-runtime-active-subscription selector及同名route.final，无关保存订阅/节点absent。独立代码review无剩余可操作Finding。此为Host在会话提供的复核事实；本收口轮未重新执行Cargo、候选生成或kernel check，不伪造原始日志。

**DAG与边界**：68卡逐卡依赖与任务总表一致，原依赖无变更、无环；DONE18/READY4/TODO39/DEFERRED7，其余0。READY仅P0-08/P2-03/P4-02/P5-06，均未领取/未启动；Core/Config owner/本卡预约释放。P2-03正式运行必须使用compile_product+显式selected runtime projection，不得沿用application/runtime.rs ObservationOnly compile(...)；本轮只记录下一动作。仅三份状态文档及新增local-only evidence，产品源码/旧evidence不改，无真实child/UI/SystemProxy/TUN/helper或任何禁止的Git操作。

[Host独立review与本轮文档验证](evidence/p2-02b/host-closeout-20261007-132657/README.md)。

<a id="p2-03-delivery"></a>
## P2-03 工程交付（2026-10-07，历史）

**ACCEPTANCE；owner Runtime/Platform + GPUI保留；Host Visual/Interaction PENDING。** 仅本卡，68卡 DONE18 / ACCEPTANCE1 / READY3 / TODO39 / DEFERRED7，显式依赖不变；P0-08/P4-02/P5-06保持READY且未启动，所有依赖P2-03的下游保持TODO。

正式链路为 SnapshotService → selected projection（含首次导入Unconfigured）→ projected OutboundId → compile_product → 随机secret → check/prepare → owned普通用户child → 本child动态mixed/controller输出 → 鉴权root/version → Ready。Desktop单一串行worker拥有Runtime，StateBridge按request/sequence接收真实快照；saved/applied独立，Stop后applied=None。候选check/prepare失败保留旧实例。稳定cache按内核版本/state epoch归属，不实现P2-04 manifest/rollback。

真实run发现旧Product的dns-direct显式detour=direct虽然check接受，run却拒绝empty direct detour；本轮按真实v1.14.0语义移除此无效detour并更新精确fixture。首轮失败原样保留，未改写P2-02B历史check结果。

固定内核v1.14.0 darwin-arm64，archive/binary两级SHA校验；每次check/run前复验binary。最终bundle executable SHA256 `fd127c1c1af4879d784efa46fe65fb6be6b894f39f2a9f2fe54f09fcef83825f`。受控本机HTTP唯一marker经actual mixed和SOCKS fixture成功；重复Start、check失败保留旧实例、mixed/controller真实冲突、Ready超时、真实异常退出/再启动、串行操作及独立进程Stop ownership均PASS。关闭窗口继续代理；显式Quit后全部owned child回收、旧端口关闭、候选目录清理PASS。

Core **336/336**；Desktop **73/73**（默认1项真实内核测试ignore，已显式单独执行 **1/1 PASS**）；两crate check、all-targets clippy、fmt与diff检查PASS。按code-delivery-review实现者自查和独立review无剩余可操作Finding；未发现任意UI executable/PID/URL、SystemProxy/TUN副作用、secret泄漏或提前实现下游。既有dirtytree按开始时源码SHA核对，无关文件未覆盖；无commit/push或其他禁止Git操作。

[完整local-only evidence](evidence/p2-03/README.md)包含源码/构建/kernel身份、真实PID/PGID/Instance/endpoints、请求、失败路径、测试和截图。最终app保持Settings→Backend、Light、Ready；沿用Host当前显示设置（GPUI backing scale=2与系统150%为不同概念），其它缩放/Dark本轮NOT_RUN。Host仅需确认当前150%下服务卡、左下Runtime状态、Start/Stop/Restart实际交互；明确回复符合/通过/可以前不DONE、不释放owner、不解锁下游。

<a id="p2-03-visual-busy-001"></a>
## P2-03-VISUAL-BUSY-001（2026-10-07）

**OPEN / FIX_PENDING_HOST；P2-03保持ACCEPTANCE，owner Runtime/Platform + GPUI保留。** Host已独立确认工程/Runtime/真实代理/失败路径；本轮只修UI Busy/status：StateBridge记录具体RuntimeCommand，当前按钮显示spinner，其它保留原Heroicon并disabled；request/sequence拒绝旧completion。Backend移除额外Busy行，固定结果行高度。Stopped采用muted、Starting/Recovering过渡色、Ready绿、Failed红。Runtime/Compiler/Sidecar/进程生命周期和业务语义完全不变，固定kernel与历史build evidence保留；不重跑完整真实failure matrix。

本轮验证/新bundle/Starting与Ready截图见[evidence](evidence/p2-03/visual-busy-001/README.md)。等待Host当前系统显示设置、Light下最终Visual PASS；未确认前不DONE、不释放owner、不解锁下游。


<a id="p2-03-visual-full-page-002"></a>
## P2-03-VISUAL-FULL-PAGE-002 · CLOSED（2026-10-07）

**当前状态：CLOSED；P2-03 DONE；Runtime/Platform + GPUI owner已释放。** Host确认Basic2最终build功能与视觉“符合”；见[最终收口](#p2-03-final-closeout)。以下为开启、修复与候选历史，保留当时状态和结果。

**OPEN；P2-03 ACCEPTANCE；owner Runtime/Platform + GPUI保留。** Host明确指出BUSY-001的局部修复不代表Full Visual Parity。本轮先观察实际OpenBox（内置浏览器192.168.1.6:3036）和旧Veyra ce5fc933…，记录初始geometry gap，再重建Backend完整层级。BUSY-001当前工程修复FIXED，历史OPEN/FIX_PENDING_HOST与原截图保留；本Full Visual Finding未关闭。

写范围仅Desktop UI backend/pages/tokens/i18n/icons/sidebar，app中的只读Profile投影与现有Notice反馈，相关同源SVG和本状态文档。正式Runtime/Compiler/Sidecar/runtime_service及StateBridge未修改。顶部服务卡按品牌、版本、Geo、三组状态、hint/版本metadata、真实操作、更新、自动更新排列；下方按actual四张连续卡的grouped sections复现。未知Geo/更新/自启/端口策略/retention/数据工具显示—或未接入，全部禁用，无网络/保存/后台任务。已建模Profile字段仅只读；Controller就绪只来自Ready且当前endpoints存在，不伪造Panel。

新bundle executable SHA256：`acadf5a6e83bf61804daa7130d4e4ff7ace265d351ecc3ba9e0b6e7c22f4d8c8`。当前系统显示设置/Light，1280×720逻辑内容视口对照。Core336、Desktop80 + 既有1 ignored；双方test/check/all-targets clippy，fmt/diff PASS；早期断言格式失败和clippy失败日志保留。未重跑完整真实failure matrix，沿用Host独立Review已通过的业务证据；本轮真实Start/Ready/Restart与只读UI操作另记。实现者按code-delivery-review自查，非独立审查；工程检查不等于Host Visual PASS。

完整初始差距、最终geometry matrix、同视口截图、service/grouped/sidebar并排图、源码与build身份、只读边界、验证日志见[local-only evidence](evidence/p2-03/full-page-002/README.md)。DAG与依赖不变：DONE18 / ACCEPTANCE1 / READY3 / TODO39 / DEFERRED7；P0-08/P4-02/P5-06保持READY，其它下游不启动。锁屏前最终app为Backend/Light/真实Ready；收尾最底部截图与回到页面顶部时Mac锁屏，待Host手动解锁后完成最终长图/交接（不能声称完整采集已完成）。等待Host明确确认后才能关闭本Finding；本轮不DONE、不释放owner。

解锁后补充：最终build同一源码/实例已完成后半页实际滚动截图与2036px内容长图，回到Backend/Light/Ready。矩阵诚实记录TUN内部MTU/MSS相对参考−12/−8px（超出4px目标）；本轮只补证据不改产品源码。P2-03 ACCEPTANCE、Full Finding OPEN、owner与DAG不变，未写Host Visual PASS。


<a id="p2-03-visual-fidelity-003"></a>
## P2-03-VISUAL-FIDELITY-003 · CLOSED（2026-10-07）

**当前状态：CLOSED；P2-03 DONE；Runtime/Platform + GPUI owner已释放。** Host确认Basic2最终build功能与视觉“符合”；见[最终收口](#p2-03-final-closeout)。以下为开启、修复与候选历史，保留当时状态和结果。

**OPEN；归属 FULL-PAGE-002（OPEN）；P2-03 ACCEPTANCE；owner Runtime/Platform + GPUI 保留。** Host 明确否定以外框坐标一致替代视觉验收。本轮仅调整 Backend / shell / 分类栏视觉、三语文案与字体资源接线；移除补偿高度，恢复完整说明，静态只读控件不再继承 Kit disabled opacity，未知开关用 off 轮廓并在外部标注未接入。Runtime、Compiler、Sidecar、child lifecycle 及八个保护文件未修改。

Host 已授权采用官方 MiSans 4.009；GPUI 使用同一官方包未改动的静态字重 face，以适配当前 macOS 文本系统不展开 VF axis 的限制。与网页 4.003 的版本及栅格差异明确保留，不声明字体像素完全相同。字体来源、许可与 attribution 保存在 Desktop assets，字体文件不进入 evidence。

完整同视口截图、内部文字/控件测量、无 mask alpha overlay / absolute difference、工程验证与候选身份见 [fidelity matrix](evidence/p2-03/fidelity-003/FULL-BACKEND-FIDELITY-MATRIX.md)。本轮不重跑 Runtime failure matrix，不修改业务语义；正常 Quit/Start 仅用于切换候选和最终 Ready smoke。Core 336 / Desktop 80（既有 1 ignored）及 check/clippy/fmt/diff 通过。最终 Host 视觉与交互确认仍待定，任何局部测量通过都不关闭 Full Visual。

DAG 不变：DONE18 / ACCEPTANCE1 / READY3 / TODO39 / DEFERRED7。不领取其它 READY，不启动下游，不 commit/push。

<a id="p2-03-backend-basic-settings-004"></a>
## P2-03-BACKEND-BASIC-SETTINGS-004 · CLOSED（2026-10-07）

**当前状态：CLOSED；P2-03 DONE；Runtime/Platform + GPUI owner已释放。** Host确认Basic2最终build功能与视觉“符合”；见[最终收口](#p2-03-final-closeout)。以下为开启、修复与候选历史，保留当时状态和结果。

- Status: FIX_PENDING_HOST；owner Runtime/Platform + GPUI。
- 范围：direct_for_nodes/reject_quic/ipv6/test_url/direct_test_url，复用 ProfileService.patch CAS/SavedOnly；其余控件保持只读。
- 写范围：Desktop backend_profile/services/state_bridge/app/ui backend/pages/i18n，定向测试与本卡记录。Runtime/Compiler/Sidecar七个保护文件不改。
- P2-03保持ACCEPTANCE，FULL-PAGE-002/FIDELITY-003保持OPEN，DAG不变，下游不启动。
- 本地证据：`evidence/p2-03/basic-settings-004/`。

- 最终候选 SHA `59280c3146502e1b9474d08f976d807cfae4390751493badf9f89aeca9b325c4`。五字段真控件、SavedOnly/CAS/字段局部busy已接通；真机保存10/运行5→显式Restart10/10，同一build正常退出重开后五字段保持。Core336/Desktop84（既有1 ignored）及check/clippy/fmt/diff通过。七个保护文件SHA不变；本轮实现者自查，未声明独立审查或Host PASS。最终Backend/Light/Ready。


<a id="p2-03-final-closeout"></a>
## P2-03 Host approval / 最终收口（2026-10-07）

**P2-03：ACCEPTANCE → DONE；Runtime/Platform + GPUI owner及Runtime公共契约owner/本卡预约已释放。** 用户在当前会话明确转达：Host已确认当前P2-03 build的功能与视觉“符合”。批准对象为Basic2 executable SHA256 `59280c3146502e1b9474d08f976d807cfae4390751493badf9f89aeca9b325c4`，精确绑定[basic-settings-004最终身份](evidence/p2-03/basic-settings-004/build-identity.json)、[五字段实际操作/重启持久化及最终截图](evidence/p2-03/basic-settings-004/README.md)、[full-page-002最终矩阵与全长截图](evidence/p2-03/full-page-002/README.md)、[fidelity-003最终细节矩阵/叠图/差分](evidence/p2-03/fidelity-003/FULL-BACKEND-FIDELITY-MATRIX.md)；详见[新Host approval与证据绑定](evidence/p2-03/final-closeout-20261007-164453/README.md)。

P2-03-BACKEND-BASIC-SETTINGS-004 FIX_PENDING_HOST → CLOSED；P2-03-VISUAL-FULL-PAGE-002 OPEN → CLOSED；P2-03-VISUAL-FIDELITY-003 OPEN → CLOSED。BUSY-001记录不改写；其历史OPEN/FIX_PENDING_HOST与后续工程FIXED分别保留，不另作状态变更。全部历史FAIL/REWORK/候选/PENDING及已知差异保留原结果。FullAcceptance `acadf5a6…`、Fidelity6 `3d98e8de…`均保留各自构建身份，不冒称来自Basic2；当前Host结论是功能/视觉符合，不改写旧测量为PASS，不声明字体/像素完全一致。Dark/其它缩放既有NOT_RUN及未来只读能力边界不扩大。

按原有显式依赖完整重算68-card DAG：DONE19 / READY7 / TODO35 / DEFERRED7，ACCEPTANCE/DOING/REVIEW/BLOCKED均0；新增READY为P2-04/P2-05/P3-01/P5-04，原P0-08/P4-02/P5-06保持READY。全部未领取/未启动；没有改变依赖、覆盖归属或Windows排期。P2为4/10、macOS为19/61完成。owner释放仅为文档预约释放，不表示当前app/child/fixture已退出或清理。

本轮仅更新三份状态文档并新增local-only evidence；现有产品源码与无关dirty tree保留。当前app/runtime资源未读取、重启、停止或清理，cleanup与live executable复验本轮NOT_RUN（按用户限定仅文档/evidence）；没有产品构建/测试、网络/权限操作，也没有commit/push/fetch/pull/rebase/reset/restore/clean/stash。DAG/链接/状态一致性与git diff --check结果写入最终evidence；原始evidence不进入Git。


<a id="p2-04-delivery"></a>
## P2-04 交付与验收（2026-10-07，历史）

**历史状态：DONE（已由 P2-04-PENDING-PERSISTENCE-001 撤销；当前以下方修复记录为准）**。仅领取 P2-04，owner Runtime/Platform（本轮 Runtime 公共契约单一 owner）已释放。无恢复 UI/Backend 视觉改动，无 SystemProxy/helper/TUN/下载/测速/failover；未触碰 P2-03 的现有 app/child/root。现有大 dirty tree 保留，无 commit/push/fetch/pull/rebase/reset/restore/clean/stash；evidence local-only。实现、完整日志/源码身份/自查见 [P2-04 evidence](evidence/p2-04/README.md)。本次是实现者按 code-delivery-review 自查，不是独立审查。

### 架构与 API

- Compiler `AppliedArtifactIndex` 绑定 ConfigVersion/计划 SelectionVersion，映射 NodeId→runtime tag、Manual PoolId→selector tag/成员/显式首成员 default，涵盖 runtime-active-* 投影；synthetic pool 不写 AppState，业务 DTO 不含 tag。
- `ManualRuntime::select_manual(ManualSelectionRequest { instance, expected, pool, node })` 和 Desktop `RuntimeService::select_manual` 共用应用命令的唯一串行 worker。依次校验当前实际存活实例/完整选择版本/applied membership，owned controller write→同实例 read-back→既有 SelectionService 原子保存→确认版本。写/读回/保存/恢复记录各自报告。保存失败一次补偿，失败保留 pending/unconfirmed；新选择在 pending 时拒绝。同 epoch 的未应用 Profile 修改不阻止合法选择，选择不会推进 config/applied config version。
- Snapshot 明确暴露 saved/applied/last-successful config、last-successful/当前 confirmed selection、pending、默认回退的 pool 列表及 recovery availability/reason。重建 owner/独立进程启动只读记录，Stopped/applied=None，不自动启动。`ApplySaved` 使用当前保存状态；`RestoreLastSuccessful` 使用旧封闭计划；原 Start/Stop/Restart/Refresh 入口保留。

### schema、缓存与回退

schema/compiler/recovery version=1；manifest 绑定 state_epoch、config、plan_selection、selection_at_apply、confirmed_selection、kernel version/SHA256、plan 相对引用/SHA256、具体生成 config SHA256、cache generation/快照引用及 resources 摘要列表（当前为空，非空拒绝）。Runtime 私有目录0700/文件0600，原子 temp write+fsync+rename+目录fsync；最终提交失败保留旧文件。pre-finalize Document + RuntimeHealthPlan + index 作为单独私有恢复 artifact，严格未知字段/模型/摘要/版本/成员校验，重建 secret/动态 ports；不读最新 state.json 猜旧配置，不解析 cache 内部格式，不保存旧PID/ports/secret/Ready。

cache path/id按固定 binary兼容身份与epoch稳定；普通revision/instance不变。Sidecar check/prepare在旧child仍活着时进行，stop_old_writer返回表示wait/reap完成，之后复制opaque关闭缓存，再run_prepared；candidate写cache不影响不可变rollback快照。copy失败返回CacheSnapshotFailed并阻止candidate，实际已停止则applied=None。当前无FakeIP，首次基线可以是明确空缓存；有引用的cache/resource缺失、generation/kernel/epoch不兼容及digest失败拒绝Restore。损坏材料不自动删除，显式ApplySaved修复前保留诊断副本。

候选check/prepare失败不停止旧实例；旧writer已停止后，run/Ready/reconcile失败只自动回退一次，使用旧计划和对应cache、重新secret/port/check/Ready/reconcile。成功返回CandidateRolledBack且Ready/applied=旧版；失败按实际Failed/RecoveryRequired报告。没有旧记录不编造回退。Ready对外发布及applied更新均晚于selector对账；最终manifest失败返回“运行成功，恢复记录保存失败”，当前applied=new、last-successful=old。selection-only只轻量更新manifest selection section，不复制配置。

### 实际验证与历史失败

- PASS：完整 Core **354/354**（既有336项保留）、Desktop **85/85**；默认3 ignored为原P2-03真实测试和新增P2-04两阶段测试，均另行显式运行，不记作默认通过。Core/Desktop check、all-targets offline clippy `-D warnings`、workspace fmt与diff检查见完整日志。
- PASS：固定 sing-box **1.14.0 darwin-arm64** / SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，隔离root的真实selector写/读回与选择保存、writer停止/关闭cache快照、saved12/last11两入口、独立OS进程重启读取、随机secret/新instance、一次Ready失败回退和最后端口/候选目录清理。
- PASS：P2-03既有真实failure matrix另行显式回归；最终结果见 evidence。P2-02B product compile/first-import runtime-active-*与Catalog/Compiler既有测试仍由完整Core覆盖，无弱化。
- initial FAIL保留：首轮私有字段可见性编译失败；故障测试选了已选节点、漏算synthetic selector、恢复fixture节点数断言错误；clippy nested-if/deprecated test atomic API。真实首轮cache被内核创建0644导致下一次应用UnsafePath，是本次发现的实际P1问题，已由owner预建0600修复，最终真实回归PASS；失败root及日志保留供诊断。
- Native gap：**无本卡恢复契约未验项**。真实平台验收以正式ManualRuntime/ManualSidecar和独立进程执行，未操作既有GUI。恢复UI/视觉、SystemProxy/helper/TUN/FakeIP和后续算法为本卡范围外N/A，不能据此宣称已完成这些能力。无剩余可操作Finding。

完整68-card DAG重算：**DONE20 / READY7 / TODO34 / DEFERRED7**，其余0，无环、依赖不变。READY=P0-08/P2-05/P2-06/P3-01/P4-02/P5-04/P5-06；P2-06仅因本卡完成变为READY，全部未领取/启动。P2为5/10，macOS20/61。后续消费只能使用本卡owner seam，不另建选择写入路径。

<a id="p2-04-pending-persistence-fix"></a>

## P2-04 pending 持久化 Finding 修复（2026-10-07）

**当前状态：ACCEPTANCE，owner Runtime/Platform保留。P2-04-PENDING-PERSISTENCE-001 = FIXED_PENDING_HOST_REVIEW（P1，Host independent review）。** 原交付DONE撤销；此前FAIL/PASS/source/evidence原样保留。§8.2.1不得用文档迁就内存pending实现。本fix只做代码与自动/Mock；真实进程中断/重启留给Codex Desktop。P2-06回到TODO，不启动下游。新 [fix evidence](evidence/p2-04/pending-persistence-fix/README.md)。


### 修复后的权威契约

- 根因：仅内存 `ManualRuntime.pending` 无法跨进程保存意图；§8.2.1被错误改写以迁就实现。已恢复原方案；前轮选择顺序/自动补偿描述与“无Native gap”结论不再适用于当前Finding，其原日志与源码快照保留，不改写历史FAIL/PASS。
- 业务schema仍为9：`SelectionPolicy::Manual { selected_node_id, pending_node_id }`，pending `serde(default, skip_serializing_if = Option::is_none)`；旧v9缺省None且不迁移。pending与confirmed可不同，成员/Manual边界严格校验；不存在第二业务JSON/journal。
- `SelectionService::{begin_manual_pending, confirm_manual_pending, clear_manual_pending}` 共享StateAccessGate和state.json原子commit。begin保存pending保留selected，confirm/clear必须完整SelectionVersion CAS并匹配requested；每阶段selection_revision+1，config不变。legacy select_manual拒绝未解决pending。
- 正式选择校验Ready实例/expected/applied成员后：pending commit→owned controller PUT→同实例GET→confirmed commit并清pending→active/manifest confirmed轻量更新。成功返回最终+2版本。pending保存失败不PUT；write错误后仅额外GET明确读回旧值才CAS清pending；read-back超时/不一致、确认commit失败、clear commit失败均保留磁盘pending。取消默认controller补偿，不自动重发；新选择在pending时拒绝。
- Snapshot每次从业务state读取pending（单项兼容字段及完整`pending_selections`），Stopped/新owner也暴露pool/requested；存在pending时不把旧active选择映射或版本冒充当前runtime确认。last-successful仍保持manifest对应confirmed。
- 新child在Ready之前，对每个运行Manual pool先GET：actual=requested则confirm；actual=old confirmed/default则clear；third/unknown/读取失败保留pending，返回SelectionPending，不PUT，不Ready，不通过新rollback实例猜测消除意图。无pending保持confirmed authority对账。新增显式`RuntimeCommand::ReconcileSelection`供当前child重连核对，不新增UI。
- 恢复旧plan时，当前confirmed节点若不在旧artifact中，不能把fallback/default冒充旧confirmed来清pending；只有actual=requested仍可确认，其余保留待核对。自查定向测试先FAIL（错误Started），修复后两种actual子场景PASS；失败日志保留。
- ApplySaved与RestoreLastSuccessful都看当前同epoch业务pending；解决产生的selection revision进入ActivePlan/manifest。manifest schema1不变，仅confirmed section轻量前进，pending不作为last-success事实；缓存/计划/回退契约保持既有实现，未扩大config_digest强化。
- Provider替换/refresh共用事务撤销被删除或过滤的pending；subscription delete删除所属Pool时自动推进selection revision，引用冲突仍拒绝。AppState成员变更禁止dangling pending提交。snapshot replacement/backup recovery新epoch清旧pending，旧CAS不能跨epoch；P7不实现。

### 自动验证与Host边界

Core **361/361**、Desktop **85/85** PASS；原P2-04 manifest/cache/rollback测试及336 Core基础行为全部保留。按新批准契约替换原自动补偿测试的预期，原测试源码留在新fix initial-source；新增schema9/staged CAS/持久pending/重建owner/两入口actual核对/显式重连/provider删除过滤与epoch测试。Core/Desktop offline check、all-targets clippy `-D warnings`、workspace fmt、diff检查PASS。日志、初始失败与实现者code-delivery-review自查记录见新fix evidence；不称独立审查。

**Native gap（本fix NOT_RUN）**：Codex Desktop仍须用隔离root/自有child验收真实进程在pending commit后、controller切换后/confirm前被中断，OS进程重启后从state/cache核对actual=requested和actual=old/default；验证actual=third/unknown时不重发、不Ready、pending保留，以及两个恢复入口版本/manifest一致。本Agent未启动真实sing-box、未操作真实桌面或既有app/child/root；前轮Native PASS不能替代新增pending中断验收。Host review通过之前不得恢复DONE。

完整68卡DAG：**DONE19 / ACCEPTANCE1 / DOING0 / READY6 / TODO35 / DEFERRED7**（其它0，显式依赖不变、无环）。READY=P0-08/P2-05/P3-01/P4-02/P5-04/P5-06；P2-06 TODO（缺P2-04 DONE）。owner Runtime/Platform保留；无下游启动。无commit/push/fetch/pull/rebase/reset/restore/clean/stash。


<a id="p2-04-uncovered-pending-pool-fix"></a>

## P2-04 同一Finding第二边界：pending pool未被旧plan覆盖（2026-10-08）

**当前状态仍ACCEPTANCE / P2-04-PENDING-PERSISTENCE-001 FIXED_PENDING_HOST_REVIEW / owner Runtime/Platform保留。** Host independent review指出，旧reconcile仅遍历artifact index，遗漏当前业务新pool的pending，可能错误Ready；原361 Core/85 Desktop PASS及全部旧日志作为当时结果保留，不代表该边界已覆盖。新增[fix evidence](evidence/p2-04/pending-persistence-fix/uncovered-pool-20261008/README.md)，不覆盖前轮日志。

Runtime `require_pending_coverage`按PoolId/NodeId校验所有业务pending均被当前plan的index覆盖。在compile/recovery-load之后、finalize/check/prepare之前拒绝静态不兼容candidate，返回SelectionPending，无PUT/CAS/clear，不停止当前child；prepare完成后、stop_old_writer之前再检查最新快照，如期间新增未覆盖pending则取消prepared candidate，保留旧child/identity/endpoints与业务意图。选择此边界是因为pool/index覆盖无需run或controller事实即可判定，不应破坏旧writer。恢复旧plan不删除或默认替换当前pending，不使用其它pool/manifest/display name猜测。

reconcile开始读取最新业务快照、验证epoch并全量覆盖，再逐组GET按既有actual=pending/old/third语义处理；ReconcileSelection同样遵守。成功前最终验证`pending_in(latest).is_empty()`，优先报告SelectionPending，不能仅依赖循环覆盖或选择版本。若循环期间已有confirmed CAS，又出现新的未覆盖pending，则保留已确认结果与剩余意图，candidate停止、不Ready、不自动rollback另一实例消除不确定事实；已Ready的原child在显式重连拒绝时仍保留真实实例资源。

自动验证：新增5 test函数，覆盖v11成功→v12新pool Ready但manifest失败→不确定选择→Mock进程结束/重建owner→Restore v11静态拒绝；已有v12 child与两pending的旧active child都保持identity/endpoints/counters；部分覆盖不GET/PUT/CAS；完整ApplySaved可确认两个pending；Restart裁剪pool拒绝；prepare/read期间新增pending及部分CAS后最终gate保留另一意图。完整Core **366/366**、Desktop **85/85** PASS（3 Native ignored、NOT_RUN）；两crate offline check/all-targets clippy `-D warnings`、fmt、diff PASS。初始2回归失败和完整365 PASS/1 FAIL的版本断言失败（漏算新增pool confirmed事实的revision）分别保留，最终纠正后PASS。实现者按code-delivery-review自查，无新增未修复代码Finding；不称独立review。

本轮只改Core manual_runtime及其Mock/tests，配套文档/local-only evidence；无schema/API/依赖/Backend视觉变更，无Native、桌面、真实sing-box、现有app/child/root操作。真实pending进程中断/重启仍留给Codex Desktop/Host，不能自恢复DONE。DAG依赖/状态不变：DONE19 / ACCEPTANCE1 / READY6 / TODO35 / DEFERRED7；P2-06 TODO，owner保留，无下游启动，无commit/push/fetch/pull/rebase/reset/restore/clean/stash。


<a id="p2-04-pending-native-acceptance"></a>
## P2-04 pending 最终 Native Acceptance（2026-10-08）

**Native Gate PASS；当前 P2-04 ACCEPTANCE / P2-04-PENDING-PERSISTENCE-001 FIXED_PENDING_HOST_REVIEW / owner Runtime/Platform 保留，等待 ChatGPT/Host 独立 review。** 本轮仅增加 Desktop `manual_sidecar.rs` 的 ignored `#[cfg(test)]` harness，production code/UI 未修改。fixed sing-box 1.14.0 darwin-arm64、binary SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4` 每次执行前复核。最终 source aggregate SHA256 `5a62b80a19b1c26b46f32f2e5b1fdba8079dc04622654da9679df01f74fd6671`；完整身份与日志见[本轮 Native evidence](evidence/p2-04/pending-native-acceptance-20261008-082538/README.md)。

六组独立 OS stage（old/pending/third × ApplySaved/RestoreLastSuccessful）全部 PASS：真实 pending 先原子落盘、controller GET=a/b/c；Stop/reap 关闭唯一 writer、setup OS 退出并由 parent wait 后，新进程读同 root/cache。old→clear、pending→confirm，selection 0→1→2；third→SelectionPending、selected=a/pending=b、selection 0→1→1、applied=None/no Ready、candidate 清理。startup PUT 均0；config/applied配置版本保持11、manifest仅confirmed；成功恢复新secret/new instance，动态端口仅由自有日志取得。本轮正常关闭child后退出stage，不冒称SIGKILL/crash注入。

正式v11 Apply/Stop/Restore后增加新pool，真实旧plan缺一个及两个pending的preflight均PASS：当前instance/PID/两端点不变、listener存活，check/prepare/stop/run/GET/PUT均0、无部分CAS。无需伪造plan或Native N/A。unknown/read timeout = CORE_FAULT_INJECTION_ONLY。

修改前与最终源码的P2-03/P2-04旧Native均PASS。最终Core366/366、Desktop85/85（8 ignored已通过显式父test/stage覆盖）、两包check/all-targets clippy -D warnings、fmt/diff PASS。逐阶段lsof单writer/关闭SHA/mtime交接、child/group/reap/listener/candidate目录与成功root清理PASS。首次harness路径canonicalization FAIL保留；该诊断root无存活child/listener/writer，其余成功root清理。实现者code-delivery-review自查无剩余问题，不称独立review。

完整68卡DAG不变：DONE19 / ACCEPTANCE1 / READY6 / TODO35 / DEFERRED7；P2-06 TODO，全部下游未启动。未操作正式Veyra root/既有app或child/System Proxy/TUN，无commit/push/fetch/pull/rebase/reset/restore/clean/stash。

<a id="p2-04-host-rework-fix"></a>
## P2-04 Host REWORK 有界修正（2026-10-08）

**Host 独立审查结论 REWORK（P1 + P2）保留；当前修正 FIXED_PENDING_HOST_REVIEW。P2-04 仍 ACCEPTANCE，owner Runtime/Platform 保留；P2-06 TODO，无下游或 DAG 状态变更。** 这是实现者修正与自查，不是独立复审/Host PASS，不自行 DONE。前轮 Native PASS 只绑定其原 source；`pending-native-acceptance-20261008-082538` 全部证据未写入，本轮新源码 Native/GUI NOT_RUN。

- **P1 旧 cache 误清 pending**：v11 cache=a 成功、v12 Ready 但 manifest 提交失败；PUT=b 成功后 GET 失败留下持久 pending=b；下一次 ApplySaved 的 Run/Ready 失败，原路径恢复 v11 cache=a，再 GET=a/CAS clear 并错误 Ready。现候选清理后先读持久 pending，存在则 SelectionPending，保留意图/live cache，不起旧 cache rollback child。rollback 自身的 reconcile 也禁止解决后来出现的 pending，不通过新的回退实例猜测。无 pending 的一次回退/cleanup 保持既有测试。
- **显式 Restore 同样受来源约束**：有 active 时恢复 artifact SHA 必须与当前 active plan 一致（不仅比较 config revision 或 cache 字节）；关闭 writer 后使用其最新 cache。重建 owner 仅接受 live cache 与 manifest cache 引用的字节摘要一致，不拿不同的旧快照覆盖 live 再推断 actual。不满足则在 prepare/check/Stop 前保留 child/identity/endpoints/pending 并拒绝；prepare 后再次检查新增 pending。actual 仍只由本次受管 controller GET 提供，不解析 cache 内容判断节点。可信当前 child 的显式 ReconcileSelection 或 ApplySaved 成功确认后，才可按已确认业务事实恢复；v12 确认不能把 v11 manifest 记录保存冒称成功。
- **P2 同池 GET 并发后的旧 PUT**：初次 state 无 pending，GET 时同池发布 pending=b 且 actual=b，原路径仍 PUT=a 再由最终 gate 报 pending。现全部 controller PUT 共用 SnapshotService 的共享 selection write gate；持锁核对完整 SelectionVersion（epoch + revision），持续覆盖有超时的 PUT/GET，避免核对与写入之间再发生业务提交。gate 不覆盖 check/run/Stop/cache；现有 platform 每次 PUT/GET 的 10s 超时不变，其他业务写在此期间按既有 Busy 失败，释放后正常提交。确认/clear 在释放锁后仍需完整版本与 requested 的 CAS；冲突不丢已提交的部分确认，不新 Ready/manifest 假确认。

**验证**：新增 5 项确定性 Mock 回归，覆盖 P1 Run/Ready 两故障、active/重建 owner 显式 Restore、cache 字节相同但 active plan 不同、ReconcileSelection/ApplySaved/Restore 三入口同池 GET 并发、两 controller 写入口共享 gate、epoch 替换以及部分确认 CAS 冲突。初始 **4 FAIL / 1 PASS** 原日志保留；修正中 **27 PASS / 1 FAIL**（测试误将 ApplySaved 后最新成功版仍断言为11）及完整 Core **370 PASS / 1 FAIL**（断言误假定 pending 排序）各自保留，最终断言精确核对版本、PoolId/requested、业务事实、manifest、PUT 数、cache 字节、identity/endpoints/cleanup，不弱化旧测试。最终 Core **371/371**（既有366 + 新增5，doc-tests0不计）、Desktop **85/85，8 ignored NOT_RUN**；两 crate offline `check --all-targets`、`clippy --all-targets -- -D warnings`、workspace fmt 与 `git diff --check` PASS。Core 包含既有自有 loopback fixtures/测试 executable 子进程，不称纯 Mock，更不作为 Native/GUI/公网验收；不执行真实 sing-box 或 ignored Native。Cargo 原有 `block 0.1.6` future-incompat 提示保留，无本次 lint 错误。

**改动与身份**：产品/测试只改 `crates/veyra-core/src/application/manual_runtime.rs`、`manual_runtime/tests.rs`、必要 `state_service.rs` 共享 gate（14行新增）；配套仅本 Markdown、SESSION 与新 ignored evidence。无 Desktop/UI/schema/依赖/任务总表变更；开始时631文件工作树 hash 核对无关变更保留。三文件 source aggregate SHA256 `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8`，完整算法/逐文件 hash/真实命令退出码与初始 FAIL、最终 PASS 见[新证据](evidence/p2-04/host-rework-fix-20261008-085206/README.md)。按 [code-delivery-review](../../.agents/skills/code-delivery-review/SKILL.md) 完成实现者自查，本次修正未发现剩余可操作 Finding；独立 Host review 与新 Native 验收仍待完成。无真实内核/GUI/现有 app、child、正式 root/系统代理/TUN/权限操作，无禁止的 Git 写操作。


<a id="p2-04-host-rework-native"></a>
## P2-04 Host修复绑定源码 Native复验（2026-10-08）

**既有Native回归PASS；P1/P2真实控制器+限定故障注入补验PASS。P2-04继续ACCEPTANCE / Finding FIXED_PENDING_HOST_REVIEW / owner Runtime/Platform保留，等待Host对本轮证据最终复核。** Host已通过Core修复独立代码复审（当前用户说明）；本轮实现者自查不冒充独立review。

严格重验三文件aggregate `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8`，每条最终命令前核对源码/内核，production/UI未修改；只扩展Desktop real_tests的cfg(test) wrapper及3个ignored tests。固定sing-box1.14.0 darwin-arm64，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。完整身份、实际命令/退出码/日志和注入边界见[本轮独立evidence](evidence/p2-04/host-rework-native-20261008-091526/README.md)。

P2-03/P2-04初始及最终回归、old/pending/third×ApplySaved/Restore六组跨OS进程、旧plan缺一个/两个pending preflight均PASS。P1真实v11→v12 Ready配合隔离目录写失败、丢弃真实GET=b结果、Run/Ready错误注入，证明pending12/1不被旧cache回退消除；active与closed来源不一致的Restore预检拒绝，manifest11/selection0保留；随后无PUT的实际GET=b确认到12/2。P2三入口GET结果返回前发布同池pending，不PUT旧a；Reconcile保留原child/端点，replacement入口清理candidate/no applied；旧revision、错误epoch、重复CAS被拒绝，精确确认到11/2，另验证epoch替换与共享写锁Busy。

本轮注入不等于内核自然故障：自然HTTP超时/断线、自然spawn/内核启动失败、无控制线程race、SIGKILL仍NOT_RUN；unknown/read timeout/相同opaque cache但不同plan等为CORE_FAULT_INJECTION_ONLY。未伪造HTTP响应或写运行中cache。没有GUI/SystemProxy/TUN/既有app、child、正式root/openbox.disign.me操作。

新增3/3（7个fixture）PASS；最终Core371/371、Desktop85/85（11 ignored已显式覆盖）、两包check/all-targets clippy、fmt/diff PASS。精确lsof单writer、关闭cache交接、candidate清理及成功root删除PASS；独立核验46内核PID/PGID、13 stage/test PID均退出、102端点拒绝连接。历史PASS/FAIL及失败root不改写/不触碰。DAG仍DONE19/ACCEPTANCE1/READY6/TODO35/DEFERRED7，P2-06 TODO，无owner释放/下游启动/commit/push。


<a id="p2-04-final-host-closeout"></a>
## P2-04 Host FINAL ACCEPTANCE 收口（2026-10-08，当前）

**OBG-P2-04：ACCEPTANCE → DONE；P2-04-PENDING-PERSISTENCE-001 与前轮 Host P1 旧 cache/pending、P2 同池 GET/PUT 竞态两项 Finding：FIXED_PENDING_HOST_REVIEW → CLOSED。Runtime/Platform 及 Runtime 公共契约 owner/本卡预约释放。** Host 已在本轮独立确认本 Task 验收 **PASS** 并授权文档收口；这是记录 Host 的确认，不是实现者自查冒充独立复审。原首次 DONE、重开、Finding、FAIL/REWORK/候选与历史 PASS/NOT_RUN 段按当时事实保留；不改写原证据。

**Host 绑定身份与核验**：三 Core 文件 SHA 与 [source identity](evidence/p2-04/host-rework-native-20261008-091526/source-identity.json) 精确一致，aggregate `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8`；Desktop `manual_sidecar.rs` test harness SHA256 `06d2b0f93e69af498ca7e298ffc259b9575f5acd1fc3e84c2d06bc402f78b24a`。Host 已核验77条 Native结构化事件与逐行原日志完全一致，六组 old/pending/third × ApplySaved/RestoreLastSuccessful 的 actual、startup PUT=0、manifest版本及旧plan缺一个/两个pending preflight全面 PASS；P1 两类真实child配合限定故障注入、保留cache后无PUT的实际GET=b确认，P2同池GET交错、精确CAS、共享gate/epoch均 PASS。原14条最终命令exit0；完整 [Native/注入证据与边界](evidence/p2-04/host-rework-native-20261008-091526/README.md) 保持原样。

Host 自己经 MCP 新复跑 Core **371/371，exit0**，receipt `CommandRun command-5654-1791423312116059-98`；Desktop **85/85，exit0**，receipt `CommandRun command-5654-1791423346868107-99`，**11 ignored由显式Native父test覆盖**。这是 Host 在当前会话提供的人工确认和命令 receipt；本收口轮不另跑 Cargo/Native/GUI，不伪造新的运行日志。自然HTTP/OS故障、SIGKILL、无控制线程竞态压力仍 **NOT_RUN**；unknown/read timeout等限定Core注入边界保留，不扩为本Task额外Native PASS。正常Stop/reap不称SIGKILL。

**当前 DAG**：68条唯一任务及其原显式依赖独立核验一致、无环；**DONE20 / ACCEPTANCE0 / READY7 / TODO34 / DEFERRED7**，DOING/REVIEW/BLOCKED均0；macOS **20/61**，P2 **5/10**。只有 P2-04 ACCEPTANCE→DONE 与 P2-06 TODO→READY；READY=P0-08/P2-05/P2-06/P3-01/P4-02/P5-04/P5-06，全部未领取/未启动，其它任务状态/依赖不变。

**本轮范围与版本控制**：仅本卡、IMPLEMENTATION_PHASES、SESSION及新 ignored [Host收口记录/独立静态核验](evidence/p2-04/final-host-closeout-20261008/README.md)。业务源码与harness SHA前后不变；未操作正式root、已有app/live child、GUI、网络设置、SystemProxy/TUN、权限或openbox.disign.me，cleanup/live资源复验 **NOT_RUN**。owner释放不意味着现场资源已清理。只读提交隔离评估 **BLOCKED**：HEAD `0230aa72539375b622422e0e76e009a399b81669` 缺P2-04所依赖的前序未提交实现及本卡Runtime/adapter模块，当前index无已暂存修改；无法形成不混入前序内容且可重建自洽的独立P2-04提交。未stage/commit、不制造空提交，不触碰index/dirtytree；后续基线整合由Host处理，无push/fetch/pull/rebase/reset/restore/clean/stash。


<a id="p2-06-local-contract"></a>
## P2-06 首轮最小契约（2026-10-08，历史，DOING）

当前进度以[第二轮工程记录](#p2-06-round2)为准；以下首轮结果与失败原文保留。

唯一 owner：Codex Runtime/Platform / Runtime共享DTO。基线 ac919f3729fed5a3cf0a2b3dbcbd4164b06a3597，初始工作树干净。

生产 executable 与 prototype 分 feature/bin 隔离；封闭 IPC，无任意执行参数。Hello 不需要实例；业务会话由内核 UID/PID/进程启动身份确定，重连不撤销操作。固定帧/配置/事件限额、Host 时间预算、有限并发与 TTL/LRU 操作结果；UNKNOWN 必须重新查 Status，无持久 journal。Start/Apply 使用类型化业务配置与完整 epoch/config/selection CAS；受控 P2-02B compiler 本地生成。生产未具备安装/安全 writer-cache-manifest 交接时 fail closed；不得接管手动 child。SystemProxy 仅类型化接口/不支持状态，P2-07 实现未启动。

测试保护：非法请求在副作用前拒绝；重复与断线不制造双 child；结果不把失败写成 Ready；跨进程不能 Stop；版本冲突不应用；事件和资源有界；客户端拒绝不可信 socket。OS 身份用隔离 socket 实测，其余生命周期用 Mock 并单列范围。Native 安装/卸载、普通用户 child、真实控制器与 GUI 均 NOT_RUN；实现者自查不等于 Host review。


### 本轮交付与未完成边界

- **生产源码已接线**：Core `application/helper_protocol.rs`；helper `src/production/{host,transport,main}.rs` / library；Desktop `runtime_service.rs` 与 `platform/helper.rs`。现有 AppServices composition 创建的 RuntimeService 持有真实生产 Client；UI 消费未开放。Helper worker 调用真实 P2-02B compiler，未把 opaque JSON 直接交内核。
- **生产能力 fail closed**：缺固定资产/登记为 NotInstalled；属性不安全为 UnsafeEndpoint；即使固定 kernel 文件存在，仍 HandoffRequired、不启动进程。实际 kernel version 未核验，Hello 返回 None，另列 required 1.14.0。没有 installer/uninstaller、降权 child、controller adapter、跨 owner cache/manifest 的完整生产代码，**不能仅补 Native 测试后就 DONE**。旧手动 Runtime/恢复实现未改，prototype 源码/native 未改。
- **有界本地合同**：协议1；控制16KiB、配置1MiB、资源0；connect/每帧2s，4连接worker+4队列，一个业务worker/在途，backend固定30s预算；操作结果128条/完成后120s TTL+LRU，UNKNOWN先Status，无journal；事件ring256/单批16/gap。UID/PID/start time 由 OS 取得，payload无owner；请求ID只在同一进程会话有效。SystemProxy仅封闭请求和Unsupported，没有系统写入。

| 原卡验收 | 本轮证据范围 | 整项结论 |
| --- | --- | --- |
| 1 Hello/身份/版本/大小 | 无实例Hello、真实非特权peer UID/PID/start、拒绝版本/UID/额外字段/oversize/invalid frame；root目录/socket规则只作非特权拒绝及属性测试 | 本地合同 PASS；真实已安装root端点 NOT_RUN，不勾整项 |
| 2 幂等/重连/操作查询 | Mock单child计数、同requestId/新ID重复Start、断线再连Inflight/Completed、TTL/LRU Unknown；真实Unix socket + 真实产品编译 | MOCK/OS隔离 PASS；真实child NOT_RUN，不勾整项 |
| 3 受控请求/旧实例/无journal | 封闭schema拒绝任意path/PID/command/owner/timeout，跨session与复用PID启动身份拒绝、完整版本CAS、pending与旧writer拒绝、无持久journal | 本地合同 PASS；跨owner实际恢复未实现，不勾整项 |
| 4 安装/用户身份/controller | 正式生产代码未完整实连；未执行管理员、launchctl、真实helper/kernel/GUI/System Proxy/TUN | NOT_RUN；不能以P0-05旧结果代替 |

验证命令/exit/源码身份及初次失败见新 ignored [evidence/p2-06](evidence/p2-06/README.md)。Core **371**、Desktop **87**（原85+2，11 Native ignored）、production helper **14**、P0-05 **47**、原Python **2** PASS；Core/Desktop/helper production+prototype 的 `check --all-targets`、`clippy --all-targets -- -D warnings`、helper生产bin构建与fmt/diff结果分别记录。旧 `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --offline -- -D warnings` **FAIL**：缺 `binaries/sing-box-1.14.0-windows-amd64/LICENSE`；未改旧Cargo/资源、未用TAURI_CONFIG覆盖。零测试的main/doc-test不计行为通过。

实现者自查修复：AlreadyRunning结果漏留存、同版本不同配置的假确认、连接阶段缺超时、缺失/权限错误混淆；初次测试误把port=0当作AppState层拒绝，修正为真实compiler断言。两次clippy候选失败（collapsible_if、unused import/expect）原日志保留；不是Host独立review，Host review NOT_RUN。

**状态**：P2-06 继续 **DOING**，不是只待第四项的 ACCEPTANCE；owner保留。完整68卡 DONE20 / DOING1 / ACCEPTANCE0 / READY6 / TODO34 / DEFERRED7。READY=P0-08/P2-05/P3-01/P4-02/P5-04/P5-06均未领取；P2-07仍TODO。仅本卡dirtytree保留，无commit SHA、无stage/commit/push；本卡未达到可按Task提交的完整实现候选，且规定旧Tauri检查失败，不提交不自洽候选。

**下一步（仍属P2-06）**：补正式受控安装/卸载源码、固定资产身份验证、普通用户child+Ready/controller adapter，复用P2-04选择/manifest语义，完成单writer/cache的明确交接；未具备安全交接时继续拒绝，不接管已有手动child。上述代码完成后由Host独立review，再由Codex Desktop执行管理员安装/卸载、root目录/socket、真实普通用户child身份/FD配置、控制器选择/观测、断线重连/重复Start、停止清理与授权拒绝后手动代理 Native/GUI验收。P2-07多Service SystemConfiguration、P2-05应用出站、P6正式实现均不在本轮。


<a id="p2-06-round2"></a>
## P2-06 第二轮工程记录（2026-10-08，历史，DOING）

当前见[第三轮工程记录](#p2-06-round3)，以下结果与OPEN原文保留。

同一Task / WorkRun `work-5654-1791427311971442-122`，基线HEAD `ac919f3729fed5a3cf0a2b3dbcbd4164b06a3597`，继承首轮dirtytree，Codex继续唯一Runtime/Platform与Runtime DTO owner。写范围增加helper `assets/backend/process/install/process_tests.rs`、Core仅抽取原Desktop `owned_child::terminate`、Desktop复用与交接拒绝；不改P0-05 src/native，也不改ManualRuntime/recovery业务语义。没有领取P2-05/P2-07/P6。

**实质新增**：固定root资产SHA/版本/父目录验证；复用ManualRuntime与RecoveryStore的真正check/run/Ready/reconcile/Stop/reap执行器；受保护FD3配置、清环境/setsid/清附加组/非root uid/gid启动代码；复用鉴权ClashApiClient；单owner当前cache inode授权与回收后快照；250ms自有child存活刷新和有界事件。安装CLI只接受固定bundle资源和OS console用户，SHA及root保护前置校验，固定plist/launchctl代码；非特权拒绝路径已测。以上root分支一律未执行。

**尚未闭合的工程边界（不是Native测试欠缺）**：

1. **P1 OWNER-HANDOFF OPEN**：Desktop独立worker与helper之间没有stop/reap + writer关闭的持久交接凭证，也没有受保护目录间cache/manifest身份/摘要/epoch核对与撤销机制。现有ManualRuntime仅有本进程gate，不能把status Stopped当作跨进程排他权。正式Backend资产校验后仍HandoffRequired，Desktop Start/Apply校验业务版本后也拒绝；不会为测试停止旧child。Host需确认后续最小交接契约，再继续本Task工程。
2. **P1 REMOTE-SELECTION OPEN**：ManualRuntime::select_manual依赖本地SnapshotService/gate完成持久pending→PUT/GET→精确CAS确认。helper收到的AppState只能作编译输入；不能作为第二份可编辑事实。缺少desktop持久pending与helper操作结果的事务边界，因此Select仍HandoffRequired；启动reconcile和Observe已有真实执行器。不能把初始reconcile当作用户选择完成。
3. **P1 INSTALL-LIFECYCLE OPEN**：固定install代码已提供，root保护的bundle/manifest打包与管理员授权调用UI未接通；uninstall只支持无socket/无runtime材料的离线完整安装。活动资源Stop/恢复/卸载barrier尚缺，失败保留材料；未尝试真机操作。

| 四项验收 | 本轮证据 | 当前结论 |
| --- | --- | --- |
| 1 Hello/系统身份/版本/资源限额 | 无实例Hello、OS peer与请求拒绝合同已有测试；正式root端点未执行 | 非特权范围PASS，非完整平台验收 |
| 2 会话/幂等/未知结果 | 原14合同/Mock/OS项保留；真实Backend经自有Unix socket验证重复Start、断线重连Operation、CAS、进程退出后Status撤销 | 非特权范围PASS，非Native |
| 3 禁止任意执行/过期实例/永久日志 | 封闭路径/PID/命令拒绝、真实Backend stale/CAS复核、内存TTL结果；无journal | 非特权范围PASS；不能据此宣称交接与Select完成 |
| 4 管理员安装/业务用户/controller | 仅固定源码和非特权拒绝测试；root身份变化、正式kernel与UI未运行 | NOT_RUN，禁止直接Native验收 |

本轮Core371/Desktop87（11 ignored）/production helper20（1子进程fixture ignored）/prototype47/Python2均PASS；三包all-targets check/clippy -D warnings、production bin build、workspace/legacy fmt与diff均exit0。完整测试命令、exit、失败历史、源码SHA与自查见新 ignored [round2 evidence](evidence/p2-06/round2/README.md)，不覆盖首轮日志。原Tauri缺Windows LICENSE是已知外部fixture失败，不修改Legacy，也不以它作为本轮阻断依据。实现者自查不等于Host独立review。

**状态保持DOING**，不能ACCEPTANCE/DONE；Runtime/Platform owner保留，DAG DONE20/DOING1/READY6/TODO34/DEFERRED7。READY=P0-08/P2-05/P3-01/P4-02/P5-04/P5-06；P2-07 TODO。工程合同未完整，按用户Git条件不stage/commit，继续保留本Task dirtytree，无新commit。先补上述工程缺口并由Host独立review，之后才交Codex Desktop管理员安装/取消/卸载、root目录socket、实际降权child/FD、Native鉴权controller/选择与必要UI验收；P2-07系统代理写入不在本卡。


<a id="p2-06-round3"></a>
## P2-06 第三轮工程记录（2026-10-08，DOING）

同一Task，继承round2 dirtytree，HEAD仍`ac919f3729fed5a3cf0a2b3dbcbd4164b06a3597`，Runtime/Platform与Runtime DTO owner不变。没有改变P2-04 DONE、领取P2-05/P2-07；所有新增证据仅ignored `evidence/p2-06/round3/`。用户报告的Host round2源码复核与helper20复跑只属于前轮，不能当作本轮Host审批。

**A 已实做的范围**：Desktop `RuntimeService::transfer_owner`使用真实串行worker；Core `helper_transfer.rs`、`manual_runtime/handoff.rs`、`runtime_recovery/handoff.rs`；Helper `Handoff`协议/Unix framing/worker/backend。双向同一流程：源先写Freezing并禁止Start/Select，实际Stop/reap后导出封闭plan/关闭cache/manifest；目标Prepared禁止启动，源Released持久确认后目标原子保存材料/manifest并标Local。转移不自动启动，恢复需新check/Ready。双epoch/config/selection精确CAS在桌面现有gate内核对；缓存代际、内核SHA/版本、plan/index/选择映射、关闭cache与live内容摘要一致。目标引用由摘要推导，拒绝payload任意路径。plan≤1MiB/cache≤2MiB/bundle JSON≤13MiB，仅Prepare/携带bundle的响应用资源class3，其余控制16KiB。业务pending、运行manifest与新业务epoch/已确认选择版本不符都在freeze/Stop前拒绝，不用旧cache清pending。

票据同ID/版本/摘要可重试，半提交不解冻。每root仅一份当前交接记录、一份bundle、Runtime生命周期绑定；helper另用内核peer写固定0600会话记录，没有持久请求journal。Runtime重建不能用空Child冒充旧writer退出：nonce不匹配阻止Start/再次释放；helper重启只能由相同OS会话只读Query并返回recovery_required，不能自动恢复或跨session接管。干净首次启动无manifest时先拒绝且不写冻结标志，不冒充cold-start已交付。

**Host新Finding修复**：同session新requestId Start不再从Host缓存直接Completed(Ready)，改为Inflight→owner worker同步poll/核验→实际Completed/错误；不创建第二child。同ID仍是历史结果。新增确定性Mock测试只暂停空闲poll，保留请求同步核验，证明退出后不能复用缓存Ready；真实OS退出状态撤销测试仍通过。

**OPEN / 工程尚未完整**：

- A的正常双向及半提交重试已在非特权真实进程/Unix socket验证；崩溃后的安全解除冻结、旧writer处置/重建尚无完整入口，当前只查询并fail closed。正式安装Backend和Desktop Start/Apply总gate仍HandoffRequired，不能称正式模式切换已可用。
- **B REMOTE-SELECTION仍OPEN**：没有交付远程durable pending→PUT/GET→Desktop CAS→manifest闭环。关键缺口是本地gate若在IPC超时后释放，helper仍可能继续PUT，而同池新pending/epoch替换已发生；不能把持锁等待一次RPC或helper修改received-state当作解决。需要跨超时/断线仍有效、所有业务writer共同遵守的有界冻结/终止查询合同，再在实际GET后做完整SelectionVersion CAS确认及helper manifest回告。当前Select继续HandoffRequired，未添加影子业务CAS。
- C活动Stop/恢复/卸载barrier未实现，沿用round2仅离线未使用安装可卸载的限制；未扩大到P2-07 SystemConfiguration。

**验证**：最终Core371、Desktop88（新增1，11 Native ignored）、helper25（新增5，1内部子进程fixture ignored）、prototype47、Python2通过；三包all-targets check/clippy -D warnings、production build、fmt/diff结果及精确source SHA见[round3 evidence](evidence/p2-06/round3/README.md)。新增OS测试真正运行生产Backend与当前测试exe的kernel HTTP fixture，不是正式sing-box或root身份转换。P0-05源码/native未改；P2-04原选择/恢复逻辑未替换，新增专门交接模块、入口fence与full-version gate，旧371测试全部保留。

仅code-delivery-review实现者自查，Host独立review本轮NOT_RUN。初次编译/Clippy失败日志保留，旧evidence不覆盖；没有真实root、已装helper、既有app/child、sudo/launchctl、SystemProxy/TUN、公网或GUI操作。**P2-06保持DOING**，不ACCEPTANCE/DONE，owner保留；工程未完整故不stage/commit，HEAD不变。DAG DONE20/DOING1/READY6/TODO34/DEFERRED7；READY P0-08/P2-05/P3-01/P4-02/P5-04/P5-06未领取，P2-07 TODO。下一步仍为本Task工程：补B及安全解冻/卸载，再Host独立review，之后才交Codex Desktop管理员安装/取消/真实降权child/Native controller/UI验收。


<a id="p2-06-round4"></a>
## P2-06 第四轮工程部分交付（2026-10-08）

**DOING / Runtime-Platform及Runtime DTO owner保留；未提交，不能进入Native。** WorkRun `work-5654-1791427311971442-122`。用户提供Host已独立review第三轮并复跑Core371/Desktop88/helper25/prototype47的事实（CommandRun `command-5654-1791432070225921-130` exit0）；这不代表第四轮源码已获Host批准。本轮证据独立存入ignored [round4](evidence/p2-06/round4/README.md)，旧FAIL/SHA/源码历史按原结果保留。

**P1 HANDOFF PREFLIGHT ORDER 修复候选**：`helper_transfer::transfer`在Stop/freeze前先读Desktop完整版本/pending/远程fence，ToHelper再只读核验当前confirmed/manifest、Configuration合法性和1MiB上限，经固定Client完成OS peer/socket/Hello协议/宿主/kernel能力检查，发送只读`Handoff::Preflight`确认目标资产/目录/历史owner可接入。生产HandoffRequired能力也会在此拒绝。之后prepare_handoff重新检查业务事实才持久freeze/Stop；执行期失败仍可能中断并冻结，未承诺不中断。真实隔离child回归：missing endpoint、bad protocol、接入拒绝、stale epoch/selection、合法超大配置都无Stop、无owner冻结，真实child仍存活、cache/manifest原字节保持。

**B 已接通范围**：Desktop `RuntimeService` 的RuntimeCommand Select在原串行worker中路由到Core `helper_transfer::select_remote`；`JsonStateStore`先持久化一份16KiB有界fence再保存pending。所有普通save/commit/replace共用OS writer锁和fence检查，commit版本CAS与写文件同锁，局部controller PUT gate也持有该锁。SubscriptionManager/ProviderReplacement不需要另立状态机；独立gate或新进程仍不能跨越fence。Helper在自身当前实例、OS session、完整版本和稳定PoolId/NodeId核验后映射artifact tags，发真实controller PUT/GET；只有Desktop按完整SelectionVersion及requested/previous CAS确认或clear后，helper才更新manifest并接收桌面回告的非权威输入投影，最终解除fence。读回old时保存clear但返回选择失败；third/unknown不清pending。复用Host有界请求结果/Operation/事件；相同requestId不同payload拒绝，重试查询不重发PUT。没有helper第二份业务权威或永久请求journal。

**确定性OS验证**：自有测试exe持有真实cache writer FD，正式Backend/Unix socket/controller loopback完成选择、发送端socket真实关闭/重连Query、同池GET屏障期间新pending/换epoch/原始写拒绝、桌面CAS成功后manifest失败重试；PUT计数核验无旧PUT重放。新OS进程验证writer互斥及持久fence，corrupt state不从旧backup清pending。Core373/Desktop88/helper28/prototype47/Python2 PASS；Core额外1 ignored和helper1 ignored为父测试实际调用的内部子进程fixture，Desktop11 Native ignored均NOT_RUN。三包all-targets check/clippy -D warnings、fmt/diff见最终receipt；最初两次oversize测试fixture不合法、初始方法可见性编译错误、初始PUT绝对计数断言、收尾只读handoff_state误拒绝pending的FAIL保留；后者已移回专用preflight_transfer并重跑helper28，不改写为PASS。实现者自查按code-delivery-review，不称独立review。

**剩余OPEN**：A崩溃安全解冻/旧writer证明、首次cold-start与完整模式启用；B缺失存活slot/OS会话变更时仅保留fence，尚无安全重新绑定和终结UNKNOWN的恢复入口（不能自动Execute）；C活动Stop/恢复/卸载barrier及安装授权集成。B当前限定明确旧confirmed NodeId的Manual pool和已应用完整版本；未应用业务配置不混入旧运行实例。正式HandoffRequired继续保留，不用cfg(test)运行证据解除。P2-07真实SystemConfiguration仍Unsupported。完成工程缺口并经Host独立review后才可交Codex Desktop执行管理员安装/取消、真实降权child/controller和Native/UI。当前无commit条件，保留dirtytree。

DAG保持DONE20 / DOING1 / READY6 / TODO34 / DEFERRED7；P2-04 DONE不变，P2-05 READY未领取，P2-07 TODO。Ready Queue：P0-08/P2-05/P3-01/P4-02/P5-04/P5-06。


<a id="p2-06-round5"></a>
### P2-06 第五轮工程交付（DOING）

同一WorkRun `work-5654-1791427311971442-122`，parentExecution `execution-5654-1791432203384243-131`；Runtime/Platform、Runtime DTO owner保留。第四轮Host复跑CommandRun `command-5654-1791434121002287-132` exit0属于历史输入，本轮仅实现者自查。旧源码/FAIL/evidence保留，新证据 [round5](evidence/p2-06/round5/README.md)。

本轮新增固定管理员文件请求→串行worker实际Stop/reap→ClosedBundle关闭cache/manifest校验→持久ACK→bootout→服务退出锁→删除固定安装文件的代码。管理员身份不进入普通客户端Peer。service生命周期锁与installer互斥锁是固定root文件；请求永久封住该安装，新命令/新服务不能越过。未确认remote selection只停止自有child，保留Desktop pending/fence且不ACK。封存/停止失败或未知旧材料不bootout/不删除恢复资料；生命周期锁释放不能证明旧writer已死。卸载成功分支仍保留封存runtime和marker，不擅自删恢复材料。增加无权限安装manifest摘要生成API，未接入真实打包/授权UI。

初始root检查从已知文件列表收紧为完全空目录；没有manifest但有旧cache同样拒绝，不写session/新cache。重建backend即报告recovery_required，Host初始状态取实际backend。正式HandoffRequired总gate未解除，该检查不是完整cold-start合同。

新增6项helper定向回归覆盖真实受控child Stop/reap/封存、未确认选择不清fence、manifest路径故障不ACK、OS服务进程kill后独立writer仍持FD而拒绝卸载、marker权限/symlink、旧cache无manifest。仅测试exe/自有temp；不是root/native结果。首轮3 FAIL来自错误的manifest字节不变预期与可被Stop正常刷新覆盖的故障注入；修正为业务字段不变和不可写manifest路径，原FAIL保留。

**OPEN**：跨helper/Desktop崩溃及会话重建的安全解冻/接管、lost selection slot/UNKNOWN的有界终结、双方无历史材料的cold-start接受合同、卸载失败/持久marker后的恢复与重装、真实打包和授权UI接线。不能只凭新进程空Child handle清fence，不能自动恢复旧cache。P2-06仍DOING，四项验收未全部完成，不进入Native、不提交；P2-04 DONE不变，P2-05 READY未领，P2-07 TODO。

第五轮完整离线验证：Core373 PASS/1内部fixture ignored，Desktop88 PASS/11 Native ignored，production-helper34 PASS/2内部fixture ignored，P0-05 prototype47 PASS，Python2 PASS。内部fixture由父测试显式运行；三包两feature all-targets check/clippy -D warnings、production bin build、workspace与Legacy fmt PASS。Native root/安装/授权/真实kernel/GUI均NOT_RUN。DAG仍DONE20/DOING1/READY6/TODO34/DEFERRED7；index空、HEAD不变。


<a id="p2-06-round6"></a>
### P2-06 第六轮：卸载归档与可重装候选（DOING）

同一WorkRun `work-5654-1791427311971442-122`，parent `execution-5654-1791434235257029-133`。输入第五轮source `b8d0db73f6fbe8d765b8bbbbe4d974f7cc18537fcd2ab164ee4fc04ce2b3c682`，用户提供Host helper34/clippy复跑exit0；本轮不冒充Host独立review。Runtime/Platform owner保留，旧FAIL/证据不改，新证据 [round6](evidence/p2-06/round6/README.md)。

修复“成功卸载留下固定root而永远Busy”：安装/卸载持有root外固定deployment-lock；ACK、固定bootout成功记录和服务退出锁齐备后，将plist移入旧root，再将整root以RENAME_EXCL原子迁到固定RecoveryArchive，条目名取OS dev/inode；0700、root归属、同文件系统、禁止symlink/可写目录/覆盖，fsync双方父目录。旧runtime/cache/manifest/fence/owner和凭据留在受保护Archive。新安装只验证Archive的归属/ACK/bootout记录，不消费历史数据。未知原root/Archive/残留plist拒绝安装。plist移入后的半卸载可重试；rename后的响应/同步失败可补同步。bootout真实成功但记录未落盘的SIGKILL窗口仍fail closed，需管理员人工核验，不假定已卸载。

新增5项非特权OS测试覆盖两轮模拟卸载/重装、目录权限/符号链接、Archive冲突、真实rename不覆盖失败、半提交、缺失ACK、bootout模拟失败、自有SIGKILL子进程和锁。旧Core/Desktop/Runtime选择/owner协议没有改动。导出固定installation_plist生成函数，与原manifest生成/核验为bundle打包提供确定输入，但真实打包/授权UI仍未接通。

**收口决策交Host**：Archive闭环可独立review；正式普通handoff启用仍是工程OPEN。现有Released由客户端DTO提供，helper未独立验证源固定释放记录/旧writer关闭，Desktop Start/Apply也仍保持拒绝；本轮未简单移除gate。Cold-start双方无历史且无writer的证明尚缺。未知owner或lost slot可考虑限定人工修复分支，但这需要Host明确确认验收取舍；当前没有宣称该取舍已批准，亦不清pending/fence或重发PUT。P2-06仍DOING、不提交、不Native，P2-04 DONE/P2-05 READY/P2-07 TODO不变。

第六轮完整离线回归：Core373/1内部fixture ignored、Desktop88/11 Native ignored、helper39/2内部fixture ignored、prototype47、Python2 PASS；三包两feature all-targets check/clippy -D warnings、production build、workspace/Legacy fmt PASS。Native root/launchctl/真实kernel/GUI均NOT_RUN。DAG保持DONE20/DOING1/READY6/TODO34/DEFERRED7，index空、无提交。

<a id="p2-06-round7"></a>
### P2-06 第七轮：固定来源核验与有限正常交接（DOING）

同一WorkRun `work-5654-1791427311971442-122`，parent `execution-5654-1791435120590446-137`。输入为round6及用户提供的Host Core373/Desktop88/helper39/prototype47 exit0。唯一Runtime/Platform及DTO owner保留；新ignored证据 [round7](evidence/p2-06/round7/README.md)，历史FAIL不改。

正式Hello协议升级v2：handoff可预检与runtime已授权分离，解除预检依赖runtime授权的循环。Core在Stop/freeze前发布本进程OS身份与Runtime incarnation；helper从getpwuid_r固定Desktop root独立读取，不能由payload指定来源。预检观察唯一固定digest直接child的kqueue退出事件；Prepare要求真实退出/reap与Closed证据，Commit重读Released与完整ticket/manifest/关闭cache摘要。仅同Peer/start/incarnation在本helper生命周期内接受Local后，Start/Apply/选择每次重新核验来源；未授权仍HandoffRequired。Desktop同worker核对本地Stopped/Released、helper Local/Status及业务全版本，正式Start/Apply不再无条件拒绝；手动写入口仍冻结。

新增2个真实非特权OS父测试，helper共41：Preflight只读成功且旧child保持运行；正常Stop/reap/Closed/Released/Commit后真实Unix IPC Start→认证Ready→重复Start同child→Apply→Stop；伪造PID、活源writer、错误incarnation/Peer/epoch/revision、非Released及坏bundle拒绝；已接受来源被修改时不启动新child且RecoveryRequired。源凭据/manifest保留，fixture验证自身child均reap并清理临时root。负面交接为真实Backend直接调用；执行链路才是实际socket。没有root/正式kernel/真实安装，不能冒充Native。

**OPEN与收口决策**：仅适用预检时仍有可观察旧child、完整last-successful的正常会话。cold-first-start与预先Stopped来源仍拒绝；helper SIGKILL/丢slot、Desktop换session、未知orphan writer保留fence/RecoveryRequired，需明确人工修复决定。OS NOTE_EXIT证明观察到的直接child已退出，不等于同UID可写资料不可伪造，也不能排除隐藏writer；磁盘kernel SHA不等于加载代码签名。此有限信任边界待Host判断是否满足范围，不能称已完整工程收口。打包/授权UI仍缺；未进入Native，保持DOING、不提交。

离线Core373/1内部fixture ignored、Desktop88/11 Native ignored、helper41/2内部fixture ignored、prototype47/Python2及check/clippy/build/fmt/diff见round7 receipts；内部fixture由父测试启动。最初Desktop参数漏接编译失败与proc_listchildpids返回值误读失败保留，修复后重跑。实现者按code-delivery-review自查，不冒充Host独立review。DAG DONE20/DOING1/READY6/TODO34/DEFERRED7；P2-05 READY、P2-07 TODO不启动。

<a id="p2-06-round8"></a>
### P2-06 第八轮：统一产品标识与helper退出清理（DOING）

同一WorkRun `work-5654-1791427311971442-122`，parent `execution-5654-1791435786077906-140`。用户明确授权精确重命名：7文件13处原标识改为 `me.disign.veyra`，保留 `.gpui-preview/.helper/.p005` 后缀。Desktop固定数据目录与helper source同时更新，Tauri JSON、安装plist/label与probe一致；prototype仅LABEL/PLIST字符串变更，native和其余行为不变。全体有效tracked+非ignored untracked文本/文件名复扫旧值零残余，历史ignored evidence不动。新数据目录不会自动加载旧数据；没有迁移、fallback、删除、旧daemon卸载，专门迁移需另行批准。实际bundle签名/安装/升级未执行。

Quit及worker断连在同一Runtime worker按持久owner处理。Released先通过固定Client核对helper Local同票据，按真实instance Stop并查询Operation及Status；超时不重发、UNKNOWN不假成功，半交接不冒充已停止。失败事件为Recovering/StopFailed，持久fence/owner保留。Query允许在未确认选择时只读核对owner，Stop不能清pending；只有实际无instance/applied且无RecoveryRequired才确认清理。窗口隐藏不等于退出；普通手动Start仍受Core owner fence阻止。P2-07 SystemConfiguration/TUN未实现。source Witness在kqueue注册后重新核对实际进程身份；原NOTE_EXIT+reap、固定source Closed→Released与完整hash校验继续保留，cold/未知session/丢slot不自动接管。

新增Desktop半交接退出/拒绝第二writer测试、helper UNKNOWN/失败清理语义测试、Python跨组件身份测试；既有正常交接OS测试增加真实Client退出、错误票据拒绝、注入响应丢失后的同Operation查询、重复Quit。真实自有socket/child/wait与controller fixture；响应丢失为客户端故障注入，不冒充物理断网。初始Python测试遗漏system/前缀FAIL、两次Desktop退出读取snapshot导致state意外初始化FAIL保留；后者已去掉成功Quit快照读取，不修改原订阅回归。最终结果与源身份见 [round8 evidence](evidence/p2-06/round8/README.md)。

仍DOING、Runtime/Platform+DTO owner不变。cold-first-start、未被观察的旧实例、跨崩溃owner/slot人工恢复与同UID文件信任边界、实际打包/授权接线仍OPEN。没有Native/root/launchctl/正式sing-box/GUI操作，没有stage/commit。DAG DONE20/DOING1/READY6/TODO34/DEFERRED7，P2-05 READY未领取，P2-07 TODO。

<a id="p2-06-round9"></a>
### P2-06 第九轮：限定应用标识纠正（DOING）

用户纠正正确基础身份为 `me.disign.veyra`；只更新当前源码/配置/工具/测试/说明，保留 `.p005/.helper/.gpui-preview` 后缀和独立实验标识。当前第八轮说明同步为正确值，历史原始round8 evidence只读不变。本轮不推进Quit/交接/冷启动/恢复等功能。新数据根 `~/Library/Application Support/me.disign.veyra.gpui-preview` 与两个历史namespace隔离，不自动读取、迁移、覆盖或删除旧数据，不fallback、不卸载旧daemon；专门迁移需后续用户批准。Bundle/LaunchDaemon标识变更未进行真实安装/签名/授权验收。

WorkRun `work-5654-1791427311971442-122`，parent `execution-5654-1791437186920664-143`。验证命令/输出、精确本轮diff、源码身份及零残留扫描见 [round9](evidence/p2-06/round9/README.md)。P2-06仍DOING、唯一Runtime/Platform+DTO owner保留，index空、不提交。DAG与round8不变，P2-05 READY/P2-07 TODO均不启动；无Native/root/GUI/系统配置操作。

<a id="p2-06-round10"></a>
### P2-06 round10：恢复查询只读边界（DOING）

同一WorkRun `work-5654-1791427311971442-122`，parent `execution-5654-1791437881532671-148`。本轮修复Handoff Query/Preflight进入admin_barrier可触发Stop/封存的副作用：Query不进入drain；Preflight只读检查管理员请求并拒绝准入，不执行drain。已授权管理员poll/变更路径仍按原合同处理，后台独立drain不属于查询的无副作用承诺。没有新协议/API、冷启动开关或数据迁移。

新增3项非特权测试：真实fixture child在Query与Preflight后仍活且无ACK/owner封存；重建Backend只读返回Closed+RecoveryRequired，跨session/宽权限凭据拒绝，Start仍拒绝；真实自有writer持有已unlink FD且目录完全空时，pristine_root虽返回空，正式requires_source准入仍HandoffRequired、不创建第二child。因此目录/文件/PID缺失不能等价于“从未有writer”。所有资源为测试自有，未碰真实安装/用户目录。

四条验收现状：①Hello/身份/版本/限额已有生产协议及OS/Mock证据；②重复Start/Operation/断线结果查询已有实现，重启不承诺exactly-once；③封闭命令、CAS、session归属及无journal已有，未知恢复仍拒绝；④安装/卸载代码与隔离fixture已有，但真实管理员/root权限/降权kernel/controller/授权拒绝证据NOT_RUN，打包授权接线未闭合。不能把前三项自动测试等价于全卡通过。

首次冷启动OPEN：固定安装资产和授权UID不证明Desktop此前没有writer，现有安装合同没有可信的独占首次owner接受事实。同UID用户资料及hash不等于root可信记录/加载代码签名。已向Host提出唯一范围决策：是否接受首版只从正在运行且可观察退出的手动实例交接、首次helper冷启动明确不支持；未有答复前不改变验收范围，继续DOING。若保持首次冷启动要求，需要单独明确可信安装准入合同，不能以空目录放行。旧Stopped、orphan、lost slot、跨崩溃情况仍独立拒绝。

源码增量/测试输出/SHA见 [round10](evidence/p2-06/round10/README.md)。Runtime/Platform+DTO owner保持，P2-04 DONE、P2-05 READY未领取、P2-07 TODO不变；不stage/commit、不进入Native。

<a id="p2-06-round11"></a>
### P2-06 round11：内核真实basename / SystemProxy决策登记（DOING）

同一WorkRun `work-5654-1791427311971442-122`，parent `execution-5654-1791438327148817-151`。用户决策A仅增补P2-07卡/方案§11.1.1/SESSION：主动开启允许接管已有代理，按Service ID先持久快照，Veyra Ready后条件应用/回读，关闭仅恢复仍匹配managed的字段组，外部更改保留报冲突，多服务/部分失败分别记录；没有SystemConfiguration代码，P2-07仍TODO无owner。

决策B实际落地：共享受控文件名为`veyra-sing-box`；production安装bundle输入、受保护root、卸载固定文件检查、source Witness来源basename统一；ProcessPort从该固定资源exec，Archive仍整体封存不改格式。Desktop开发override及canonical结果均检查实际basename，旧名字/别名指向旧名字拒绝并给出明确诊断；bundle固定Resources/helper入口验证同名同SHA，P0-08产物尚未交付。P0-05原型安装/执行/资源探针与Python staging同步；P0-04/P0-07诊断工具只在自身新temp内重命名新解压文件后执行。没有exec-a、没有通过改进程名伪装、没有查杀外部sing-box。上游归档成员/字节SHA/1.14.0/Clash版本字符串/JSON语义及全部App identity不变。

新增OS测试实际exec测试exe字节副本，通过proc_pidpath与proc_bsdinfo.pbi_comm核验均为veyra-sing-box，认证fixture Ready后Stop/reap；这是真OS basename证据但不是正式kernel/root Native。新增Desktop拒绝旧名/symlink/错误字节测试，Python跨链合同测试。离线Core373/Desktop90/helper46/prototype47/Python4均通过，all-targets check/clippy -D warnings、build/fmt/diff见 [round11](evidence/p2-06/round11/README.md)，本轮0失败。实现者code-delivery-review自查非Host独立review。

明确剩余路径：Legacy Windows `src-tauri/src/platform/windows/managed_sidecar_port.rs:88`仍join sing-box.exe，配套managed_sidecar常量/Tauri Windows bundle映射亦未改；W0-01/W1-01及P0-08需在Windows fixture/打包资源可验证时完成，不能称全平台命名完成。P0-08真实macOS bundle产物与新basename Native check/run/降权/安装卸载均NOT_RUN。原P0-05 DONE审批只绑定旧baseline，历史证据不改，新basename实现不能复用旧Native PASS；不改变既有Task状态。

已有安装/用户cache/旧namespace完全未操作、不迁移、不fallback，新installer不会覆盖旧root。P2-06原cold/未知owner恢复/打包授权等OPEN保留，DOING、Runtime/Platform+DTO owner不变；index空，不提交、不启动P2-05/P2-07/P6。DAG不变。


<a id="p2-06-round12"></a>
### P2-06 round12：安装归属事实与重试存活修复（DOING）

用户要求首次安装/正常重启直接启动helper，不能依赖先运行手动代理；这是实现目标，不再请求产品确认。本轮安装器在固定deployment锁下、发布plist之前创建0600 `installation.json`，绑定随机安装世代、固定root dev/inode与授权UID/GID；资产加载和实际check/run复核绑定。复制到另一个root、缺失/半写/不安全权限拒绝，重试不覆盖，Archive保留原件。该记录只证明管理员安装归属，不证明旧Desktop writer已退出，不单独授予cold-start。

同requestId成功Start重传现在进入串行worker同步poll并核对完整版本，不执行第二次Start；已Stopped/退出返回StaleInstance，仍活返回真实状态。Operation在重试前仍可查询历史结果，重新核验不延长原TTL，过期UNKNOWN。缺失/损坏安装身份的Backend报告RecoveryRequired，不以空slot冒充干净停止。非特权OS fixture覆盖实际Unix peer、NOTE_EXIT、Stop/reap与安装Archive，Mock覆盖TTL。

首次bootstrap/正常重启仍OPEN：现有Desktop Released票据依赖last-successful+已观察的活child，尚无首次无历史的持久让权合同；root安装世代尚未与Desktop Runtime worker跨进程独占及可重试提交绑定。下一步应限定同一安装世代/本产品自有资源，补双方合作的独占与启动前提交，不再要求证明全机器从未存在writer。已知旧root/崩溃/孤儿/lost selection slot继续拒绝，不能重置pending/fence；同UID用户文件hash不是管理员凭据/代码签名。新旧安装不自动迁移。

精确diff、失败与修复、源码SHA/命令、四场景与四项验收边界、Host review建议及后续Native提示词见 [round12](evidence/p2-06/round12/README.md)。本轮不启动P2-05/P2-07，未实现SystemConfiguration/TUN，Native NOT_RUN，不提交。


<a id="p2-06-round13"></a>
### P2-06 round13：跨父端失联的writer租约（DOING，未开放首次Start）

首次合作准入最小合同仍为 `未占有 → Desktop持久冻结 → helper Prepared → 同票据Commit → helper已占有 → Start`，半提交保持冻结。不能用安装记录/空目录直接跳到Start；正常跨进程重启另需真实Stop/reap、关闭cache/manifest及无pending的持久clean-stop凭据。

本轮切成可运行底层增量：固定安装root `runtime-writer-lock` 由正式ProcessPort持有，check/run继承同一open-file-description到FD4（配置仍FD3），父端死亡/关闭自身FD不释放仍活child的租约；新owner不能取得同一锁。每次spawn校验锁path与持有inode、owner/mode/nlink一致，替换/权限攻击拒绝。Assets只读Preflight检查既有锁，不创建文件，冲突在旧Desktop冻结/Stop之前拒绝；Backend在写输入投影/owner记录之前实际取锁。Archive在已有ACK、lifecycle条件之外再取writer锁，失败不移动原root/资料。租约只能证明参与本合同的writer互斥，不证明任意旧writer已死，不放宽旧owner/pending/fence。

新增非特权OS测试实际继承FD4、杀死自有服务持锁进程、保持另一自有writer存活，确认新租约及Archive拒绝；writer真实退出/wait之后才可再次取锁。服务模拟进程和writer均由父测试持有Child，不是运行真实helper/sing-box，bootout为Mock。现有真实Backend/kernel fixture所有check/run也验证FD4继承；正式降权kernel是否保留该FD需Native确认。

**首次独立Start仍OPEN**：本轮未增加bootstrap IPC、Desktop首次无历史冻结/确认、可查询提交或跨会话clean-stop恢复；不是首要业务目标已达成。最小下一实现范围是把既有安装世代与当前Desktop worker持久owner、完整版本、OS peer绑定到受限prepare/commit/query，不新增任意path/PID输入；未知分支继续拒绝。本轮Native NOT_RUN，不提交，全部精确diff/SHA/命令/Host review及后续提示词见 [round13](evidence/p2-06/round13/README.md)。


<a id="p2-06-round14"></a>
### P2-06 round14：首次本地冻结与初始化竞争修复（DOING）

**已实装的有限增量（a）**：`RuntimeService.freeze_first_bootstrap`由同一worker调用Core，在完整版本、无pending/remote fence、受限Configuration、本地Stopped且无成功历史及无已知旧材料前提下，持久绑定incarnation及`BootstrapFrozen{id, installation, version}`。同一进程同票据幂等；`owner_transfer`可在回复丢失及重启后查询；重启不允许冒用旧incarnation重试/解冻，Manual Start/Select拒绝。历史材料或symlink拒绝且不清除。安装世代参数仅关联候选，不是授权token；空目录/空Child槽没有被用来开放helper Start。

**Host finding修复**：`InstalledBackend::initialize`在借用assets时取得/验证writer lease，成功后才take。真实OS锁竞争失败保留assets且不写owner/received-state；持久写阶段一旦开始，失败锁定HandoffRequired/RecoveryRequired，保留owner资料。相应隔离测试覆盖锁释放后同backend重新取得租约、owner写后失败不重试、真实fixture child运行时拒绝first-freeze且保持原实例/manifest。

**明确未完成（b/c）**：首次root受限prepare/commit/query、root安装预检到Desktop freeze的调用方编排、helper核验Desktop的合作让权与OS排他证明、正常跨进程clean-stop重开仍OPEN。新freeze是内部阶段API，尚未由产品/UI自动调用；调用方在未来接线前必须完成目标安装/世代/版本/能力预检，不能直接调用freeze后才发现helper不可用。`BootstrapFrozen`既不是Released也不是Local，现有Start/Handoff Commit均不因此放行。没有新增可用bootstrap IPC，也不称用户首次独立Start已支持。

**验证**：Core376（1内部fixture ignored）、Desktop91（11 Native ignored）、helper56（3内部fixture ignored）、P0原型47、Python4；三包all-targets check/clippy -D warnings、production build、fmt/diff通过。最初定向测试漏导入StateEpoch编译失败已修复，原FAIL保留。OS隔离只运行测试自有child/Unix socket/文件锁，不代表root、真实veyra-sing-box、Native授权或UI验收。精确增量/逐文件SHA/命令收据/实现者自查与后续Host要点见[round14](evidence/p2-06/round14/README.md)。P2-06保持DOING，不提交，不启动P2-05/P2-07。


<a id="p2-06-round15"></a>
### P2-06 round15：root首次Preflight/Prepare/Query已接通（DOING，Start未开放）

**实际生产通路**：封闭Bootstrap协议进入既有Host单worker，沿用版本/UID/配置大小/frame校验与固定客户端endpoint。只读Preflight验证安装身份/generation、实际OS peer、完整版本、root旧资料/lease/管理员撤销条件，返回Eligible；不创建运行目录/文件、不停止child。Desktop同worker先预检成功再Core freeze并发布SourceSession；root Prepare从固定UID home相对路径读取冻结记录/incarnation/session，核对票据和OS UID/PID/start，再持有writer lease、create_new+fsync单份0600 prepared记录。配置只保留digest，不产生第二份业务事实。相同peer/票据/配置可幂等Prepare和Query，半状态/未知旧owner拒绝，跨helper重建丢slot仅RecoveryRequired，不重新夺锁或消除证据。

**Host finding**：incarnation存在但owner-transfer缺失的半写入，root Prepare拒绝；同一Desktop Runtime的Start及重新freeze均拒绝，保留现场。测试模拟该磁盘半状态，不声称已经注入真实硬盘fsync失败。

**明确剩余OPEN**：没有Commit或BootstrapCommitted，没有首次独立Start；Desktop单实例PrimaryInstance持锁事实尚未作为受系统可验证证据传给helper，source同UID资料不是代码签名或旧writer退出证明；需在现有锁/Runtime边界闭合合作排他与提交版本后再准入。正常跨进程clean-stop重开仍缺。旧live handoff Witness/NOTE_EXIT/reap与P2-04业务状态/远程选择fence继续走原通路，P2-07网络修改未实现。首次使用产品决策无需重问。

**验证范围**：新增Core/真实Desktop worker、Helper固定安装角色/真实Unix socket与OS peer/文件锁测试；客户端处理root Prepared ACK后模拟返回Timeout，后续真socket重试/Query保持记录；源Port禁止spawn，真实child/FD4沿用完整回归。Core377（1内部fixture ignored）、Desktop92（11 Native ignored）、Helper60（3内部fixture ignored）、P0原型47、Python4及check/clippy -D warnings/build/fmt/diff通过。root/签名/真实kernel/GUI/SystemConfiguration/TUN NOT_RUN。精确diff、source identity、命令receipt、威胁边界和Host review见[round15](evidence/p2-06/round15/README.md)。仍DOING，不stage/commit，不领取其它Task。


<a id="p2-06-round16"></a>
### P2-06 round16：同OS会话首次Committed→独立Start（DOING）

新增`BootstrapAction::Commit`/`BootstrapPhase::Committed`。Prepare及Commit独立连接业务UID passwd home派生的固定primary socket，以OS返回的UID/PID/start与当前RPC Peer比较；固定desktop.lock必须实际被flock占有，固定socket/lock权限、类型和inode均检查。2-byte OWNER_PROBE/ACK不发送ACTIVATE、不打开窗口。承诺只覆盖守约产品参与者，不认证加载代码签名、不声称抵御恶意同UID修改文件/伪造合作协议。

Core先核对完整业务版本/pending/fence并持久BootstrapFrozen；root绑定安装generation、ticket、配置摘要、source incarnation与primary inode，持唯一writer lease。create_new 0600 committed marker和文件/父目录fsync成功后才确认Committed。同会话Query/Commit幂等；半写、丢slot、旧peer、异常child退出保留记录并拒绝新Start。ProcessPort继承同open-file-description的lease副本，无重新加锁或释放窗口；check/run继续真实FD4。首次没有last-successful仅在此独立分支允许，旧手动Released/Local与Source Witness/NOTE_EXIT路径不放宽。

Desktop同worker新增commit_bootstrap，只有该worker确认Commit后才允许Frozen分支Start，实际请求再次核对helper Committed/Status和当前业务版本；本地Manual Start/Select继续被持久Frozen阻止。Quit可以在primary listener先关闭后查询已提交owner并Stop/Operation/Status；不把本地空Port当远端Stopped。无GUI自动开关接线。

**真实能力及OPEN**：非特权OS fixture已证明源Desktop从未运行child，root角色后端经真实Unix IPC独立启动本测试`veyra-sing-box`可执行文件，鉴权controller Ready，重复Start不新建、Stop/reap后同会话同配置重启。它不是实际root/真实sing-box验收。跨Desktop/helper进程clean-stop重开、bootstrap分支Apply/远程选择/反向移交仍拒绝或Unsupported；已有未知owner/orphan/lost-slot不能并入首次路径。P2-06四项验收仍未全闭合，保持DOING。P2-07 SystemConfiguration/TUN不实现。

Core377（1内部ignored）、Desktop93（11 Native ignored）、Helper65（4内部fixture ignored，父测试会启动对应fixture）、P0原型47、Python4通过；初始测试FAIL、修复、check/clippy/build/fmt/diff与源码身份见[round16](evidence/p2-06/round16/README.md)。这是实现者自查，后续Host独立review；无暂存/提交、无Native/管理员/系统设置操作。


<a id="p2-06-round17"></a>
### P2-06 round17：CleanStop凭据及Primary退出清理（DOING，跨Session未开放）

本轮按用户允许的A增量收口：Core复用既有ClosedBundle校验，只有当前ManualRuntime真正Stop/reap、生命周期Stopped、last-successful完整版本一致且无pending/fence时，读取plan、关闭cache与live cache、manifest并核验。bootstrap Stop绑定当前root安装世代、旧Peer UID/PID/start、source incarnation、ticket/config摘要及owner-session，原子持久0600 `bootstrap-lifecycle.json`。记录仅一份当前cycle/关闭材料摘要；不是journal，也不是新Peer授权。下一次Start先持久Running（closed=None）、cycle+1，再执行check/run；半写pending文件不擦除，失败RecoveryRequired。

P1修复：Prepared注册OS kqueue NOTE_EXIT并前后核验peer启动身份；Backend.poll收到明确Primary退出事件时仅通过已持有的Runtime/Child句柄Stop/reap，异常退出不签发CleanStop。socket断线/窗口隐藏/缺PID不触发清理。保留FD4孤儿writer租约与Archive阻断。新真实Primary子进程+内核fixture测试覆盖断线仍Ready、Primary退出后内核回收/非Ready、根记录仍Running/未知，旧Frozen不变；不得解释为真实GUI验收。

跨Desktop/helper进程的rebind Prepare/Commit、读取CleanStop后完整资料重验并切换新incarnation/owner-session仍OPEN；未修改旧Frozen或接受新Peer，不声称跨Session重开通过。同Session首次Start/Stop重启保留。bootstrap Apply/选择/反向移交仍未开放。shutdown_owner的bootstrap_touched限制保留：新worker不能拿旧Frozen假定有权停止或接管旧session；正常退出同worker仍走Query/Stop/Operation/Status，listener先关闭不等同owner退出。Native/真实root/kernel/GUI及P2-07网络写入均NOT_RUN。

Core377（1内部ignored）、Desktop93（11 Native ignored）、Helper67（4内部fixture ignored）、原型47、Python4 PASS；all-targets check/clippy、production build、fmt/diff通过。初始clippy FAIL及完整收据、精确diff/SHA/自查和下一步提示词见[round17](evidence/p2-06/round17/README.md)。P2-06 DOING，P2-04 DONE/P2-05 READY/P2-07 TODO不变；未暂存/提交。


<a id="p2-06-round18"></a>
### P2-06 round18：新Primary受控Rebind准备（DOING，Commit/Start未开放）

新增封闭BootstrapAction RebindPreflight/RebindPrepare/RebindQuery，沿用配置1MiB/控制帧上限及既有OS peer/worker。Desktop RuntimeService.prepare_rebind→Core helper_transfer先校验当前业务完整版本/无pending/fence→root只读CleanStop预检→本地写固定runtime/bootstrap-rebind.json→root Prepare→Query。申请包含旧/新ticket和新OS session/nonce，不覆盖旧Frozen、incarnation或source-session；原Frozen持续阻止本地Start/Select。

root必须仍持有原ManualRuntime真实Stop/reap事实和writer lease：重新通过Core读取并校验last-applied/plan/关闭cache/live cache、重算ClosedBundle摘要，与当前CleanStop/cycle及安装generation/owner-session绑定比较；旧Peer必须已产生NOTE_EXIT。新Peer同UID/GID但PID/start不同，固定Primary lock/socket OS证明和保留旧source资料必须成立。root写0600/create_new/fsync bootstrap-rebind-prepared.json；同票据/peer/资料匹配可重试Query。半写保留slot/材料；新peer不获运行权，Status恢复状态保持。

新增多进程OS用例：旧Primary启动受控fixture内核，正常Stop后退出；新Primary预检/本地申请/Prepare/Query通过，旧来源字节不变；旧owner仍活、dirty live cache被拒绝；Prepared后Start仍拒绝；重建helper Backend因缺旧Runtime/租约/退出观察而拒绝。Query另经新Primary真实Unix客户端与Host执行。Prepare正例直调真实Backend并使用OS socket采样Peer，不冒充全动作端到端IPC。

**OPEN**：原子/可查的Rebind Commit、新root owner-session与Desktop本地确认、同lease下新Peer Start及后续多次重开尚未实现。不能只改self.session，也不能将Prepared当Local/Committed。helper重启恢复及既有bootstrap Apply/选择/反向交接继续拒绝。Native/GUI/真实kernel/root/授权、P2-07 SystemConfiguration/TUN均NOT_RUN。Core377/Desktop93/helper68/原型47/Python4及check/clippy/build/fmt/diff通过；完整FAIL/修复/收据/SHA/自查见[round18](evidence/p2-06/round18/README.md)。P2-06仍DOING，其它卡状态不变，无stage/commit。


<a id="p2-06-round19"></a>
### P2-06 round19：同helper生命周期的一轮跨Primary提交与启动

状态仍DOING，唯一Runtime/Platform+shared DTO owner保持；P2-04 DONE/P2-05 READY/P2-07 TODO未变。实际新增Core `commit_rebind` 与本worker申请读取、Desktop `RebindCommit`/受控ticket路由、Helper独立RebindCommit/新owner生命周期。原Frozen/incarnation/SourceSession及原root Prepared/Committed/owner-session不覆盖；新root提交记录保留旧cycle/closed证据，单独current-owner记录指向经OS认证的新peer。文件及目录fsync全成功后才切内存session/NOTE_EXIT；半写保留RecoveryRequired。ProcessPort与lease不重建、不释放重flock。新Start先失效CleanStop且cycle递增。

双真实Primary、Unix RPC/Core编排、受控`veyra-sing-box`测试exe经FD3/FD4、鉴权Controller fixture验证：首个Commit回复丢失→Query确认、重复Commit/Start、Ready→Stop/reap；旧Peer/版本漂移/新ticket/Prepare后cache污染/半marker拒绝，原凭据字节保持。Core377/Desktop93/helper70/原型47/Python4、三包all-targets check/clippy -D warnings及fmt通过。完整收据、失败与修复、源码身份见[round19](evidence/p2-06/round19/README.md)。这是实现者自查，不是Host review或Native证据。

**能力限制**：只开放同一个仍存活helper中一轮旧Primary→新Primary；新peer同session继续Stop/Start沿用已有生命周期规则。第三Desktop会话再次轮换尚未实现（固定rebind申请/marker不覆盖旧历史），helper重启/lost slot/orphan仍拒绝自动恢复。Bootstrap Apply/Select、产品GUI自动消费与真实管理员安装/降权/授权/内核/视觉仍OPEN/NOT_RUN。P2-07 SystemConfiguration/TUN不在此实现，不允许提前Native或标DONE。
