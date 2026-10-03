# Veyra Rust / GPUI 长期开发总规范

本规范自 2026-10-03 起默认适用于后续 Veyra Rust/GPUI 开发，包括本轮迁移及后续功能、修复与平台适配。与 [AGENTS.md](../../AGENTS.md) 一起执行；当前用户明确要求和更具体的项目规则优先。技术事实以[技术方案](../openbox-rust-gpui-implementation-plan.md)、当前代码/契约和实际证据为准。

[任务总表](IMPLEMENTATION_PHASES.md)是唯一状态表；[SESSION](SESSION.md)记录 Active Tasks / Ready Queue / Blocked；任务卡定义范围、依赖和完成条件。本规范规定协作方法，不作为功能已实现的证据，也不恢复旧 SDLC/UI 门禁。

## 1 DAG 与 READY

P0–P7 和 Windows 编号保留为里程碑分组及组合验收。阶段不是统一串行 Gate；禁止给本阶段所有任务隐式继承上一阶段出口，也不按任务编号大小决定开工次序。组合验收只汇总显式依赖的切片，不阻止其他依赖已满足的任务开始。

每张卡的 **依赖**（P0 现有 **前置** 同义）列出实际技术前置 Task ID；不存在阶段默认依赖。依赖交付物需已验收 DONE，接口/版本可用；仅有计划、fixture 或正在 DOING 的实现不能满足依赖。无需重复罗列传递依赖，必要时在卡中解释所消费的公共契约。

READY 是显式状态：当前范围内尚未开始、全部显式依赖 DONE 的任务从 TODO 转为 READY。TODO 表示尚未满足普通依赖；BLOCKED 只用于实际遇到的外部条件或待决定事项，不能把所有等待依赖的任务标成 BLOCKED。Windows DEFERRED 在范围排期确认前不进入 Ready Queue；排期后消费显式共享核心/GPUI/Runtime 契约，不以整个 macOS 候选版作为统一前置。

允许多个 READY 且写范围不冲突的任务同时 DOING。就绪由依赖决定，领取还需确认写范围、公共契约 owner 与资源占用；等待同一写范围的 READY 仍保留在队列并注明原因。并行开发不是自动修改系统或发布的授权，动作仍按用户/Task 已授权范围执行。

最小拆分保留原编号加后缀：P1-04A/B、P2-02A/B、P4-05A/B/C；原未开始父任务被子任务替代，不作为额外任务计数。未来仅因真实独立交付边界拆分，不为增加并行度切开强耦合实现。

## 2 四泳道与公共契约 owner

泳道用于安排工作，不产生另一套依赖或状态。一个任务可以跨泳道，但仍有一个负责整合与验收的负责人。

| 泳道 | 主要写范围 | 典型任务 |
| --- | --- | --- |
| Core/Config | 领域、持久化配置、派生目录、Compiler、订阅/规则/DNS 资源 | P1-01/02、P2-01/02A/02B、P4 配置、P5 配置、P7 数据 |
| Runtime/Platform | 实例、选择写权、下载路径、helper、系统代理、TUN、生命周期 | P2-03/04/05/06/07、P6、Windows 宿主 |
| Observation | 单一采集、事件、统计存储/查询、实例/版本映射 | P3-01/04/06、P5-03 |
| GPUI | 壳层、组件、各页面、偏好反馈、交互及视觉验证 | P1-03/04A/04B/05/06、各业务 UI |

公共领域模型、共享 DTO、Cargo workspace（含成员/共享依赖/锁文件）和 Runtime 公共契约在同一时间各有**单一 owner**。OutboundCatalog/OutboundId、Preferences、StateVersion、ProfilePatch、RuntimeCommand/Event/Snapshot 等公共类型的修改需由对应 owner 整合；消费任务提交明确字段/语义需求，不能同时各自改公共契约或另建替代版本。

领取时在 SESSION Active Tasks 登记 owner、实际写路径/模块及公共契约/资源预约；未启动实现时 owner 留空，不虚构任命。owner 可随任务交接，但需记录交付版本、未合并变更与下一动作。技术依赖按交付契约判断：Runtime 不等待托盘，Group 不等待 Proxy UI/service，终端分流消费 Observation/client discovery 而不等待完整 Connections 页，TUN 只需基础 DNS 而不等待 DNS Filter。公共文件有重叠时先由 owner 完成最小契约修改，再让消费方按约定版本工作；不为此把完整阶段变成 Gate。

四泳道不等于自动委派四个 Agent。只有子任务独立、边界明确且能节省时间时才按 AGENTS 的协作规则委派；同一模型、共享文件或连续设计决策由 owner 串行处理。

## 3 UI fixture 与真实集成

依赖尚未满足时，GPUI 可以先做离线 fixture、布局与状态交互探索，记录为草案或隔离试作。主任务仍按 DAG 保持 TODO，不能通过 fixture 将依赖改成满足。依赖满足后转 READY/DOING，再完成真实服务/Runtime/Observation 接入。

fixture 验证要标明样本来源及模拟边界；截图、编译通过、解析单测都不能代替实际操作或真实集成。只有任务卡要求的真实集成、加载/空/错误/忙碌、必要浅深色/尺寸操作与截图证据齐全，才可 DONE。实现完成但缺真实平台/界面结果用 ACCEPTANCE。

## 4 事实与数据流

```text
配置事实（Profile / Subscriptions / Groups / ChainProxy / Preferences）
  → 派生目录（OutboundCatalog、规则/资源引用与能力）
  → Compiler / Runtime（候选配置、当前实例与已应用版本）
  → Observation（实际实例事件、速率、连接、日志、DNS 能力）
  → UI（配置意图、运行事实、观测结果及可操作状态）
```

配置事实由领域服务按原子快照提交；派生目录不复制为第二份可编辑真相。Runtime 只说明当前真实运行状态；Observation 按实例/来源归一化，缺来源时保持未知/不可用。UI 通过服务读取，不能按名称或展示字段猜测运行出口，不能用旧快照覆盖新选择。

保存成功只说明业务配置已落盘。编译/check、应用、Runtime 就绪和持久化最后成功恢复记录分别有结果；保存后待应用不是失败，运行成功但恢复记录落盘失败也不能笼统显示全部成功。配置编辑可在内核停机时进行，保存不自动执行 `sing-box check`。

## 5 OutboundCatalog 原则

**Unified OutboundCatalog = Base OutboundCatalog + Group/Failover + ChainProxy**，是同一个目录的逐步扩展；不增加第二份目录持久化或新的“统一目录”任务。

| 交付 | 负责内容 | 可被消费的范围 |
| --- | --- | --- |
| P2-02A Base | 稳定 OutboundId；Node、implicit provider/subscription group、首个闭环的 selector/urltest、Direct、Block；引用与图校验契约 | 最小 Compiler、Proxy service、基础 Proxy/Connections/已加载 Rules |
| P4-02 + P4-03 Group/Failover | 静态/动态成员、组出口、lanes 与可用状态 | 目录扩展；不要求订阅高级任务先完成 |
| P4-06 ChainProxy | 保存后注册 Outbound、候选 detour/测速与引用 | 在 Routing/Client Routing 之前完成 |
| P4-05A/B/C + P4-07 | 统一目录消费及组合验证 | Routing / Client Routing / Rules / Connections / Proxy UI |
| P5-03 + P5-07 | DNS 能力、完整诊断与 Rules 最终集成 | 完整 Rules 行为闭合 |

目录从真实配置事实派生，以稳定 ID 引用，展示名可变。Domain 主模型使用 Direct / Block；外部/内核 reject 在导入/适配边界映射到 Block。Base 已定义节点、隐式 provider/subscription group 和首个闭环的 selector/urltest 语义，P2-02B 只将目录语义编译为 sing-box，不自建第二份出口定义；P4 的 Group/Failover 扩展高级成员与编排。self/cycle/dangling 校验由 OutboundCatalog 统一负责，覆盖节点、组/主备、Chain 跨类型引用；保存边界和编译使用同一校验结果。非法 Chain 保存不得注册出口；删除/禁用/刷新使引用失效时明确报告，不猜测 Direct 或静默删除用户规则。

P3 的代理基础视图和已加载规则只交付当时已支持能力；后续增量接入必须在 P4/P5 的消费任务补真实证据。组合验收链为订阅高级→动态组→failover→chain→Unified OutboundCatalog→routing/client routing；该链不强制 P4-01 与 P4-02 串行实现。

## 6 偏好语义与状态版本

| 语义 | 事实来源 / 主要 owner 任务 | 消费与边界 |
| --- | --- | --- |
| UI latency preference test URL | Preferences，P1-04B | UI 发起的节点/站点/候选链测速，按相应操作契约传入 |
| Runtime health test URL | Runtime profile，P2-02B/03 | Runtime 全局健康检测地址（含对应 directTestUrl），改变按配置应用契约处理 |
| group health URL | Group/Failover 配置，P4-02/03 | 组内健康策略；空值只按既有明确规则继承 Runtime 全局地址 |
| UI diagnostics ipv6-test | Preferences `config/ipv6-test`，P1-04B | UI 诊断是否执行 IPv6 测试，P4-05C/P5-03 消费 |
| Runtime profile ipv6 | Profile，P2-02B/P5-01 | Runtime/DNS/路由配置的 IPv6 行为，按编译/应用生效 |

三类 URL 不合并；诊断 IPv6 偏好与 Runtime IPv6 配置不合并。值恰好相同不代表语义相同；迁移/备份也要保持各自归属，不能让 UI 测试偏好反向改 Runtime 配置。

一个业务快照共享 `state_epoch`；整体替换导入、恢复出厂或整份快照回退产生新 epoch。`config_revision` 是同一 epoch 内的配置计数，方案中的 saved_revision/applied_revision 分别是该配置版本的已保存/已应用位置；`selection_revision` 是独立选择计数。跨快照必须比较完整 StateVersion，不能只比较 revision 数字。

普通配置 patch 推进 config_revision；手动选择/自动换线/覆盖模式仅推进 selection_revision，不制造配置待应用提示。写操作带预期版本；读/测试请求用请求代次抑制旧结果；Runtime 事件与清理按真实 instance ID/资源身份执行。换 epoch 后旧任务不能回写新配置，也不能丢失仍需清理的旧实例归属。

备份的 schema 前置来自持久业务数据定义，不等于登录启动/Updater 行为完成。若桌面 update preference 属于版本化 AppConfig，其字段、默认值与迁移由 P1-02 的早期类型/持久化契约定义；P7-01 不依赖 P6-03/P6-04。OS 登录项/注册状态、运行中 updater 状态、pending 与实例身份不进入业务备份，实际退出/网络清理和全功能验收仍在 P7-02/04/05 闭合。

P1-04A 包含 sidebar 折叠/展开、layout 类纯视觉桌面偏好；proxy columns、hide unavailable 等跨页面行为偏好属于 P1-04B，不能仅因影响显示而合并到视觉布局。

## 7 并行冲突与变更升级

开始并行前检查：写路径是否相交，是否共改公共 DTO/workspace/Runtime 契约，是否操作同一持久快照迁移、数据库 schema、真实实例/端口/网络服务或验证环境。同一文件不同区域也不自动视为安全并行；共享资源的真实集成验证预约串行，不能用另一任务的环境结果冒充本任务通过。

发现冲突时暂停相交写入，交由 owner 先整合最小契约，消费方重读版本后继续；记录需要重跑的定向验证。独立文件和独立资源可继续，禁止以冲突为由重置/覆盖无关 dirty-tree 工作。

范围、公共语义、schema 或依赖改变时先写清触发原因和受影响消费者，同步方案相关小节、任务卡、主表依赖/覆盖/计数与 SESSION。普通实现细节在已授权范围内直接处理；超出当前目标的公共 API 语义、重要依赖、部署/安全边界或重大兼容破坏才需用户决定。已批准 Task 内的变更不重复申请。

新依赖必须指向存在的 Task，不能依赖整阶段；调整后校验无环。拆分最小化，已有证据保留身份与历史结果，不能把旧 FAIL/NOT_RUN 改写为 PASS。

## 8 Legacy 切片退役

迁移采用“完成一片，退役一片”，不等待整个 macOS GPUI 版本结束后统一删除旧工程。`src/openbox` 与 `src-tauri` 在对应切片完成前是行为、视觉、业务实现和测试的迁移输入，不因为新目录出现就提前删除。

一个 Legacy 切片只有同时满足以下条件才允许退役：新 Core/Runtime 或 GPUI 实现已经接管该职责；真实服务/运行链路已接通；该 Task 要求的测试和实际 UI/平台验收已通过；仓库内已无生产/测试引用指向旧实现；必要的接口样本、视觉基线和行为差异已留在任务证据或 Git 历史。满足后在同一切片或紧随其后的清理提交中删除旧实现、专用测试/脚本和不再使用的依赖。

退役必须按功能边界执行，例如 Subscription、Outbound/Group、Runtime、Observation、Routing、DNS 分别清理；禁止提前一次删除整个 React/Tauri 基线。删除前使用仓库搜索确认引用，删除后运行受影响的新链路验证和 `git diff --check`。仍被其他 Legacy 测试/入口消费的 helper、fixture、图标或脚本继续保留，并在消费者迁移后再删。

生成物和本机状态不属于 Legacy 迁移资产：`dist/`、Rust `target/`、Tauri 生成 schema 等可随时清理；`node_modules/` 在仍需运行 React 基线时可保留。本地 Agent/Workspace 状态必须被 Git 忽略，不作为项目源码提交。

## 9 Task 生命周期与验证证据

| 状态 | 条件与退出动作 |
| --- | --- |
| TODO | 普通依赖未完成；fixture 试作不改变就绪判断 |
| READY | 显式依赖全 DONE；进入 Ready Queue，等待无冲突领取 |
| DOING | 已领取、有负责人和写范围；执行实现、定向验证与修复 |
| REVIEW | 必要审查/反馈未完成；自查不得声称独立审查 |
| ACCEPTANCE | 缺真实 UI/平台验收或用户明确要求的人工结果 |
| BLOCKED | 真实外部阻塞或必要决策；登记负责人、缺失条件和解除动作 |
| DONE | 本 Task 的交付、验收、必要审查和证据完成 |
| DEFERRED | 明确未排期范围；不进入当前平台分母 |

DONE 后重新计算依赖任务的 READY，未就绪任务保持 TODO。返工使依赖证据失效时同步标记受影响任务并补验，不能保留失效 READY/DONE。组合验收不替代子任务完成，也不要求无关任务先通过。

每项测试先写清保护的 Veyra 用户行为/自有契约，按 AGENTS 和任务卡选择最小定向验证。真实 sing-box 只做相关配置 check、加载、固定鉴权 API、必要受控访问及停止清理，不扩建协议认证矩阵。执行前确认 child/外部资源和授权，长命令有外层超时；零测试不能算通过。

证据记录 Task/owner/日期、源码 commit 与未提交 diff、构建身份、环境/内核/GPUI 版本、交付路径、操作或命令、预期/实际、测试数、日志/截图、审查和剩余问题。逐项区分 PASS / FAIL / NOT_RUN / N/A，N/A 给范围依据；mock/fixture 和真实设备结果分开。证据放任务文件末尾或正式交付记录，主表链接；历史证据不改写。

文档变更只校验 Markdown links/anchors、Task ID 唯一、依赖存在与无环、任务计数和 77 API/19 场景归属、`git diff --check`；不运行产品构建、网络或权限测试。产品实现时使用实际已有命令，新 workspace 命令在 P1-01 创建后登记。

## 10 如何更新任务表与 SESSION

1. 恢复时读本规范、SESSION、主表和候选卡，再读取相关方案小节；重核源码/工作树及依赖证据。
2. 主表登记状态、负责人、估算、证据及显式依赖；任务卡是依赖定义来源，主表依赖摘要必须一致。
3. READY 领取前在 SESSION Active Tasks 登记 Task、owner、泳道、实际写范围、公共契约/资源预约、当前动作与下一步。允许多行；REVIEW/ACCEPTANCE 也保留交接。
4. 完成或中止后更新证据、主表状态及计数；刷新所有受影响任务的 Ready Queue，记录写冲突等待和可并行条件。
5. Blocked 仅列实际阻塞，登记 Task/owner、原因、解除条件和下一动作；普通未满足依赖在主表/卡查询，不复制成大量阻塞行。
6. SESSION 保持简短，保留真实实现基线与最近动作；旧记录留 Git/证据。阶段表展示 DONE/总数和组合验收结果，不展示“等待上一阶段出口”。

本次文档调整后仍只有 P0-02 DONE，功能实现为 0；P0-01 READY，其他未满足依赖的 macOS 任务 TODO，Windows 继续 DEFERRED。后续状态只能随真实证据更新。
