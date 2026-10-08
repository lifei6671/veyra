# P4 完整配置能力

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：目标是配置编辑、保存、编译、生效和恢复形成切片；基础订阅后的 P4-01 与 P4-02 可并行，Chain 先于 Routing。复用 P1/P2 的版本与应用入口。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

**2026-10-08 分期适用范围**：未完成卡遵循[唯一调度政策](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)。下一 READY 首选 P4-02 节点组 UI + 真实持久服务；其它正式功能仍按原 DAG。订阅高级刷新、Group/Failover 策略、规则资源下载/更新、Chain 核心行为/受控测试、Compiler 与应用接口、Routing/Client Routing/诊断服务均须在前期各卡真实实现；用受控 loopback/自有资源做针对业务的单元和局部定向集成，不能仅保存静态配置或通过 Mock 完工。Store/资源目录 CRUD、schema/版本/统一目录引用校验、失败保旧与重建回读同步完成，UI 绑定真实服务/结果，Saved 与 Applied/Ready 分开，并验证局部 150% 浅色操作/加载/空/失败/忙碌。P4-07 仅补跨模块链路、正式内核应用/网络 E2E、联合回归和其它主题/缩放，不能接管前期组/主备/链核心行为和局部验证；P4-01 睡眠真机组合在 P6-02/05 同批复验。无授权的具体 Native 步骤记 NOT_RUN/原因，不延期接口/状态/安全控制，不因缺 TUN/SystemProxy 盲目禁用其它功能；全部原依赖与需求保留。

<a id="obg-p4-01"></a>
## OBG-P4-01 订阅高级设置与自动刷新

**类型**：业务与 UI；**依赖**：OBG-P2-01、OBG-P2-05。**依据/范围**：方案 §8.1；core subscription/scheduler、订阅设置；API 46–48、56–59 的高级语义。

**泳道 / 写范围**：Core/Config + GPUI；subscription 高级配置/scheduler 与高级 UI；与 P4-02 分开写 subscription/groups。

**执行与交付**：重命名、排除/禁用、节点 DNS、多 URL 顺序和重复策略、自动更新时间与刷新反馈；睡眠后合并一次错过的任务。

**验收**：

- [ ] 配置→预览→保存→刷新保持已确认节点身份及用户覆盖，dangling 有提示。
- [ ] 自动刷新仅在应用运行时执行；唤醒不逐次补跑，旧 epoch 下载不落入新快照。
- [ ] 高级字段、错误输入、部分失败、拖拽/排序与浅深色通过实际验收；分享功能单列 P5。

<a id="obg-p4-02"></a>
## OBG-P4-02 静态/动态节点组与编辑器

**类型**：业务与 UI；**依赖**：OBG-P2-01、OBG-P2-02A、OBG-P2-02B、OBG-P1-03、OBG-P1-04A。**依据/范围**：方案 §8.2；core groups/compiler、出站节点设置；API 43–45。

**泳道 / 写范围**：Core/Config + GPUI；groups/编译/编辑器；OutboundCatalog 公共注册契约由目录 owner 整合。

**执行与交付**：依赖基础订阅、Base OutboundCatalog、最小 Compiler 与已有 GPUI 基础，扩展 P2-02A 已有的隐式组/selector/urltest 语义并注册高级 Group 出口，使用稳定 OutboundId；selector/urltest、静态/动态成员、国家分组、图标、拖拽、默认组与成员引用；返回 dropped/dangling 结果。group health URL 与 UI latency/Runtime health URL 分别建模，留空只按已有规则继承 Runtime 全局 health URL。Group 领域/Compiler/编辑器不以 P2-08 Proxy UI/service 为前置；其运行态增强在 P4-05C/P4-07 集成验证。

**验收**：

- [ ] 动态组只使用实际存在且符合条件的节点，排除禁用节点，保留手工顺序与新增成员。
- [ ] OutboundCatalog 统一校验 self/cycle/dangling，重名与悬空成员可定位；保存失败不丢草稿，合法配置可由最小 Compiler 编译，组配置可重启桌面恢复。Runtime 实际应用与 Proxy service 运行态增强在 P4-05C/P4-07 验收。
- [ ] 编辑弹窗、默认恢复、排序和节点展示通过界面检查；failover 策略由下一任务完成。

<a id="obg-p4-03"></a>
## OBG-P4-03 failover 编排与手动写权

**类型**：运行策略与 UI；**依赖**：OBG-P4-02、OBG-P2-04。**依据/范围**：方案 §8.2.1；core groups/runtime、主备线路编辑与代理选择。

**泳道 / 写范围**：Runtime/Platform + GPUI；groups failover/选择编排与手动写权 UI；公共目录/Runtime 契约交 owner。

**执行与交付**：将 Failover 的出口与 lanes 纳入统一目录；有序 lanes、失败阈值、恢复保持时间、Auto/ManualPin、线路内 manual；复用唯一选择入口和独立 selection_revision。

**验收**：

- [ ] 模拟时钟验证连续失败切备、稳定恢复切主、全失败保留错误；不重写 URLTest。
- [ ] 手动固定使旧探测失效；恢复自动重新探测；迟到结果不覆盖人工选择。
- [ ] 成员变化、进程重启、pending 不确定及保存失败都有一致结果；自动切换不增加 profile revision。
- [ ] 主备编辑、手动固定/恢复自动提示和实际选择经过 界面与最小内核集成验证。

<a id="obg-p4-04"></a>
## OBG-P4-04 规则资源导入、分页与刷新

**类型**：资源服务与 UI；**依赖**：OBG-P2-02B、OBG-P2-04、OBG-P2-05、OBG-P1-04A。**依据/范围**：方案 §8.3；core rulesets、目标分流的规则集弹窗；API 63–65。

**泳道 / 写范围**：Core/Config + GPUI；rulesets 资源生命周期与弹窗。

**执行与交付**：受支持 JSON/SRS 资源下载/校验/版本索引、导入明细与 URL 引用、搜索分页和刷新；资源生命周期兼容 last-applied。

**验收**：

- [ ] 导入保留 URL 与数量约束，不支持项报告；旧分页响应不覆盖新筛选。
- [ ] 空/损坏/超时内容不覆盖有效资源；当前或恢复配置引用的版本不被清理。
- [ ] 热加载只有经证实才返回即时生效，否则提示待应用；弹窗操作与浅深色验收通过。

<a id="obg-p4-06"></a>
## OBG-P4-06 链式代理与候选测试

**类型**：业务与 UI；**依赖**：OBG-P2-02A、OBG-P2-02B、OBG-P2-05、OBG-P2-08。**依据/范围**：方案 §8.4；core chain/compiler/latency、链式代理设置；API 49。

**泳道 / 写范围**：Core/Config + GPUI；chain/编译/候选测速与编辑器；公共目录注册由目录 owner 整合。

**执行与交付**：链接解析、上游选择、detour、启停/排序；Chain 保存后以稳定 OutboundId 注册为 Outbound，在 Routing/Client Routing 开始前完成目录接入；连通与出口 IP 在同一候选链上测试，绑定草稿版本。

**验收**：

- [ ] OutboundCatalog 负责 self/cycle/dangling 的统一图校验，对当前已注册出口完成真实保存/编译验证；非法保存不注册出口，有效链路可应用。Chain↔Group 跨类型校验先用契约 fixtures 验证，真实组/主备集成在 P4-05A/07 补验，不以 fixture 冒充组合结果。
- [ ] 改字段后旧测试失效，测速和出口 IP 来自同一候选路径；取消/超时清理临时实例。
- [ ] 主实例不被预览测试替换；界面状态、保存/应用区别和浅深色通过验收。

<a id="obg-p4-05a"></a>
## OBG-P4-05A 目标分流与 Unified OutboundCatalog 集成

**类型**：业务与 UI；**依赖**：OBG-P4-03、OBG-P4-04、OBG-P4-06。**依据/范围**：方案 §8.3；core routing/compiler、目标分流；API 62 的主要闭合。

**泳道 / 写范围**：Core/Config + GPUI；routing 规范化模型/Compiler/目标分流；公共目录/规则 DTO 单一 owner。

**执行与交付**：将 Base OutboundCatalog + Group/Failover + ChainProxy 接为 Unified OutboundCatalog；迁移策略优先级、自定义/兜底、域名/IP/Geo/资源引用与默认路由。同一规范化模型用于预览和编译；Routing 不再维护独立出口列表。

**验收**：

- [ ] Chain 已先保存注册为 Outbound，目标策略可选节点、组/主备与 Chain；统一目录拒绝 self/cycle/dangling，刷新/删除后的失效引用可定位。
- [ ] 规则顺序、出口与默认恢复可保存、应用并重启恢复；不支持字段明确拒绝应用。
- [ ] 保存成功与已应用分开反馈；目标分流编辑、排序、错误、忙碌及浅深色通过验收。

<a id="obg-p4-05b"></a>
## OBG-P4-05B 终端分流与平台能力

**类型**：业务与 UI；**依赖**：OBG-P4-05A、OBG-P3-01。**依据/范围**：方案 §8.3、§11.5；core client routing/compiler、终端分流；API 66 的主要闭合。

**泳道 / 写范围**：Core/Config + GPUI；client routing/终端 UI；共用规则模型交 owner。

**执行与交付**：迁移已知终端、来源条件、规则顺序和出口选择，消费 Unified OutboundCatalog，并基于 P3-01 Observation 构建/消费 `clients.observed()` 或等价已知终端投影；P3-02 只是连接页面消费者，不作为技术前置；与目标分流共用规范化规则模型。公共模型扩展由同一 owner 完成。

**验收**：

- [ ] 已知终端投影来自当前实例 Observation/client discovery，缺失来源明确未知，不从 Connections 页面状态推导；来源规则、默认出口和排序能保存/应用/恢复，节点、Group/Failover、Chain 均通过统一目录解析。
- [ ] macOS 不伪装支持 MAC/内核前旁路/LAN 模拟；导入数据保留并解释限制。
- [ ] 终端缺失、平台不可用、失败与忙碌状态明确；编辑/排序与浅深色实际验收通过。

<a id="obg-p4-05c"></a>
## OBG-P4-05C Routing / Terminal 基础诊断

**类型**：诊断与 UI；**依赖**：OBG-P4-05A、OBG-P4-05B、OBG-P1-04B、OBG-P2-05、OBG-P3-07。**依据/范围**：方案 §8.7、§11.5；core diagnostics、Rules 页；API 32–35 的主要闭合。

**泳道 / 写范围**：Observation + GPUI；diagnostics/Routing/Terminal 实测与 Rules/Connections/Proxy 目录消费。

**执行与交付**：接通配置预测与受管入站实测，展示 Routing/Terminal 候选规则、组链和叶子出口；Rules、Connections、Proxy UI 接入 Unified OutboundCatalog，完成 Group 与 Proxy service 的运行态增强集成。DNS 解析链/记录/完整诊断由 P5-03 及 P5-07 闭合，当前缺口明确显示。

**验收**：

- [ ] 预测与实测分区；GET/HEAD/TCP/TLS 结论符合实际动作，未唯一关联字段显示不完整。
- [ ] UI diagnostics ipv6-test 只决定本次诊断操作，不改 Runtime profile ipv6；三类 test URL 不混用。
- [ ] 结果绑定配置/选择版本、请求代次与实际实例；配置预测不冒充当前已应用事实。
- [ ] Routing/Terminal 基础诊断可操作，空/错误/忙碌/不可用及浅深色通过；DNS 完整诊断仍待 P5，不把本任务 DONE 写成 Rules 全部完成。

<a id="obg-p4-07"></a>
## OBG-P4-07 配置切片阶段验收

**类型**：阶段验收；**依赖**：OBG-P4-01、OBG-P4-02、OBG-P4-03、OBG-P4-04、OBG-P4-06、OBG-P4-05A、OBG-P4-05B、OBG-P4-05C。

**泳道 / 写范围**：组合验收；所列依赖的证据/页面组合与本卡验收记录；修复先预约相关 owner 写范围。

**执行与交付**：组合订阅高级→动态组→failover→chain→Unified OutboundCatalog→routing/client routing，检查真实保存/应用/恢复；汇总本阶段 UI 与 Compiler 契约证据；该顺序是组合验收链，P4-01/02 的实现可并行。

**验收**：

- [ ] Routing/Client Routing/Rules/Connections/Proxy UI 消费同一 Unified OutboundCatalog，组/主备/Chain 的显示与可选状态一致；Rules 的 DNS 完整集成仍由 P5 闭合。
- [ ] 删除/刷新引用和主备手动覆盖没有跨模块写权冲突，相关回归通过；Group 的 Proxy service 运行态增强有实际集成证据，不作为 P4-02 的 Core 前置。
- [ ] 应用失败保留有效旧运行配置与用户新草稿，恢复状态可解释。
- [ ] 本阶段页面/弹窗的操作、必要状态和视觉检查完整，差异有范围决定。
