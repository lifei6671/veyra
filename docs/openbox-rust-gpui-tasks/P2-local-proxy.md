# P2 本机代理闭环

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：目标是从订阅导入到真实代理访问，再停止并恢复网络；先形成 Base OutboundCatalog，再交付最小 Compiler。各功能带最小可操作 UI，主体展示在 P3 补齐。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

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

- [ ] Node、隐式 provider/subscription group、selector/urltest、Direct、Block 均由目录定义并可查询；同名/重命名/刷新不混淆身份；删除或禁用后的引用结果明确，显示名不能作为 ID。
- [ ] Base 目录的 self/cycle/dangling 校验契约统一；后续注册沿用同一图校验，不能由 Routing 或 UI 各写一套。
- [ ] Compiler、Proxy service 和基础 UI 通过目录查询出口，未知目标不猜测为 Direct；首个闭环的隐式组与 selector/urltest 已包含，尚无 P4 高级 Group/Failover/Chain 时不伪造完整目录。

<a id="obg-p2-02b"></a>
## OBG-P2-02B 最小可用 Compiler

**类型**：配置编译；**依赖**：OBG-P2-02A、OBG-P0-04。**依据/范围**：方案 §7；core compiler/domain；API 60–61 对应配置消费。

**泳道 / 写范围**：Core/Config；compiler 基础配置与 fixtures；公共模型/目录交 owner。

**执行与交付**：将 Base OutboundCatalog 定义的 Node、implicit provider/subscription group、selector/urltest、Direct、Block 语义编译成锁定 sing-box 格式；Compiler 不拥有第二份出口定义。支持基础路由/DNS、loopback mixed、控制器与 cache_file；封闭模型与固定资源；加入代表性 Compiler fixtures。完整 Group/Failover 与 Chain 的编译分别在 P4 扩展。

**验收**：

- [ ] 目录中的节点、隐式组、selector/urltest、Direct、Block 按同一 OutboundId/引用语义编译，Compiler 不自建出口目录；合法模型生成锁定内核可接受的配置；统一目录报告 self/cycle/dangling，Compiler 拒绝不允许字段。
- [ ] 保存不启动 `sing-box check`，显式应用前才检查同一候选；check 失败不停止旧实例。
- [ ] cache path/tag 与身份稳定；不输出锁定版本已移除的 DNS/缓存字段。
- [ ] IPv6、rejectQuic、directForNodes 等本阶段开放选项映射真实规则；尚未支持选项明确拒绝应用。
- [ ] Runtime health test URL（含 directTestUrl）按 profile 消费，独立于 UI latency preference test URL；后续 group health URL 只使用其明确的覆盖/继承规则。

<a id="obg-p2-03"></a>
## OBG-P2-03 手动代理 Runtime 与服务状态

**类型**：进程集成与 UI；**依赖**：OBG-P2-02B、OBG-P1-03。**依据/范围**：方案 §4.3、§7.3、§8.7、§11.3；core runtime/platform/macos、后端服务卡；API 10–11、15–16。

**泳道 / 写范围**：Runtime/Platform + GPUI；runtime 启停/实例公共契约与服务卡；Runtime 契约单一 owner。

**执行与交付**：桌面管理手动代理 child；check→启动→地址发现→鉴权就绪→停止，统一操作互斥和实例身份。基于 P1-03 的 AppServices/状态桥和基础服务壳接通状态、版本、启停/重启与操作反馈；Runtime 不依赖 P1-06 托盘完成，托盘未接通时仍显示准确不可用状态。

**验收**：

- [ ] 同一候选配置完成一次受控真实访问与停止，端口和 child 归属有证据。
- [ ] 重复启动、check 失败、端口冲突、超时、异常退出不误报 Ready；只停止自有实例。
- [ ] 内核停机时仍可编辑；保存/运行版本与实际状态分别展示。
- [ ] 服务卡实际操作通过验收，点击已提交不冒充启动完成；托盘与 Runtime 联动在 P2-09 经 P1-07 组合验收。

<a id="obg-p2-04"></a>
## OBG-P2-04 选择持久化、缓存与最后成功配置

**类型**：运行恢复；**依赖**：OBG-P2-03。**依据/范围**：方案 §7.5–7.6、§8.2.1；core runtime/storage。

**泳道 / 写范围**：Runtime/Platform；runtime 选择/恢复、storage manifest/cache；公共契约交 owner。

**执行与交付**：串行选择入口、pending/已确认状态、单一 cache writer；落盘 last-applied manifest 和必要资源；失败一次回退与两种恢复入口。

**验收**：

- [ ] 控制器确认与业务/缓存持久化失败分别报告；重启对账完成后才报告应用就绪。
- [ ] 已保存 12/最后成功 11 时可按用户选择恢复；停止后 applied=None，不沿用旧 Ready。
- [ ] manifest 写失败显示实际运行成功及恢复记录失败，保留旧记录和回退材料。
- [ ] 缓存仅在 writer 退出后快照/交接；新代际或资源缺失不静默恢复，旧 PID 不作当前身份依据。

<a id="obg-p2-05"></a>
## OBG-P2-05 下载、测速和 IP 查询的出站策略

**类型**：网络客户端；**依赖**：OBG-P2-03、OBG-P0-06。**依据/范围**：方案 §8.9；core 下载/GeoIp/控制器客户端；API 30。

**泳道 / 写范围**：Runtime/Platform；下载/GeoIP/测速网络客户端与出站策略。

**执行与交付**：落实 Direct/ViaRunningProxy、独立引导解析、超时/重定向限额与脱敏；控制器只访问当前实例 loopback；三种 GeoIp provider 正常归一化。

**验收**：

- [ ] 初次订阅、无有效代理及明确选择代理下载的路径符合策略；失败不静默换路。
- [ ] 系统代理下真实出口符合设计；TUN 专用规则保留 P0 证据并在 P6 用生产实现复验。
- [ ] 跨源不传递认证头，凭据 URL 不降级明文；日志/诊断无订阅秘密。
- [ ] 测速/IP 请求使用指定目标路径，失败保留未知状态；地理查询不发送多余凭据。

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

<a id="obg-p2-07"></a>
## OBG-P2-07 系统代理、多服务与崩溃恢复

**类型**：平台与 UI；**依赖**：OBG-P2-06、OBG-P2-05。**依据/范围**：方案 §11.1–11.3；helper SystemConfiguration 适配、后端开关/恢复反馈。

**泳道 / 写范围**：Runtime/Platform + GPUI；SystemConfiguration 恢复记录/owner 交接与系统代理控件。

**执行与交付**：按 Network Service ID 管理原值/本应用值/实际值；完整代理字段组、写前恢复记录、回读与条件恢复；网络变化串行处理；接通系统代理开关和 owner 交接。

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
