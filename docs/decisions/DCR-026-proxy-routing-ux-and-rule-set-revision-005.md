# DCR-026 revision005 — TASK-019 Desktop UX / structural Apply design candidate

日期2026-09-08。Status: CANDIDATE / Human Design Approval PENDING。本文不替换revision002已批准的acceptance ownership；只追加TASK019详细行为与Contract变更候选。Producer: Codex。

## 已接受产品决定与当前权限

USER接受TASK019 DRAFT，并明确十项产品决策，详见 `.sdlc/design/TASK-019-requirements-001.md` R01–R10。设计、Contract修订候选与独立Review已授权；实现、依赖安装、运行实例、schema、发布未授权。产品决定接受不等于此完整技术Candidate冻结。

## 变更理由与边界

- 当前Proxies大GroupCard/行式节点/底部重复All Nodes不符合桌面代理客户端扫描需求。改compact sections+nodegrid、互斥出口组/全部节点视图。
- 当前Routing常驻↑/↓被否决，改Pointer/Keyboard drag，无DnD依赖，不改reorderRoutes后端ID/priority语义。
- 当前结构保存只产生Desired而需额外Apply，改Ready且有效active时自动Apply。Stopped/noactive/recoveryRequired保存后不自动运行。Applyfailure保留Desired，持久恢复；重试仅currentDesired。
- 当前ProxyRouting success owner duration=null导致永久占据页面、其他success存在6000ms，与既有Contract2.5–3s不符。新普通success统一3000ms/noClose，明确owner recovery action生命周期。TASK018历史差异仍保留，不伪称当时已修复。

## Contract候选与发布

`.sdlc/design/TASK-019-ui-contract-0.4-candidate-002.md` 是完整非生效文本，基于当前0.3原文，仅修订普通success时间、Proxiescomposition/selection、Routingdrag/structuralApply、feedback与临时drag浮层条款，并补齐§5专用selection优先级与§14不追溯重开历史TASK012的适用文案。唯一当前有效 `docs/ui/veyra-ui-spec.md` 保持0.3/BINDING。本轮不发布或改其hash。

Human精确批准Candidate（包含Contract候选hash）后，才可另行发布该文本到有效路径、标记其真实批准状态并重绑定Design/Task元数据和Requirement引用；必须产生当时发布identity并重跑design/readiness，不以本轮0.3结构检查代替0.4 Gate。无全站样式重做；sharedsuccesspolicy实际影响其他页面的success需纳入回归/认证applicability。

## Material工程影响与最小方案

现有frontend `ProxyRoutingProvider.mutate` 单IPC pending gate，保存结果success后没有autoApply；Rust `ProxyRoutingManager.mutate` 使用StateAccessGate/CAS与generation；`apply_configuration -> apply_proxy_routing -> activate_with_expected_generation(force=true)` 本来可启动stopped内核。只加前端Ready check存在检查后Stop竞态，因此候选必须包含runtime busy guard内的Ready-only自动Apply admission，以及post-save结果不能改写为保存失败的最小响应表达。精确路径/接口/备选与generation处理以 auto-apply设计为准；不是在本轮取得后端写权限。

保持Domain/AppState/schema/Compiler/reorder语义、Manualselector、订阅activate现有生命周期。若候选包含Rust application或strict IPC DTO调整，须由此次Human Design明确批准后纳入Task scope；不把它称作CSS修补或既有小细节。没有新dependency/worker/持久化队列/CRUD重放。

## Prerequisite与影响复核

生成Candidate前只读检查 `.sdlc/evidence/golden-pages/Subscriptions/certification-0.2-001.json`：原FAIL、无HumanApproval、版本0.2，7/15manifest不匹配；当前Gate失败Page certification identity/Contract mismatch。结果见 TASK019/subscriptions-prerequisite-check-001.json。

该缺口不是由文件名推断的stale，而是原Golden从未建立且当前identity不同。用户已明确答复“授权新增 Stage S 并继续设计复审”，授权记录见stage-s-design-authorization-003.json。新Candidate正式采用S（共享Toast+完整Subscriptions Golden）→A（Proxies）→B（Routing），旧0.2记录/path/hash保留为不可变provenance，不再用作新Candidate的外部prerequisite。S须完整Compliance、IndependentReview与真实HumanVisual才能供A消费。这是有真实授权的acceptance/stage ownership变更，不是把FAIL改PASS、内部自认证或修改Orchestrator协议。

后续共享源改变使S/A证据stale时，必须新增版本化认证，执行新的Candidate/DCR引用revision、独立DesignReview和所需精确Human批准，并与Task元数据一致后再通过依赖checkpoint。不能覆盖已批准envelope或宣称原frozenhash不变。见stage-plan003完整执行模型。认证发现未授权缺陷仍报告真实gap，不授予无界Subscription修复。

## 风险、替代与验证

方案A仅前端save.then(apply)最小代码但不能保证stopped不启动、保存结果与Apply失败区分、旧generation竞态，不采用。方案B新调度器/自动CRUD重试队列可排队但违背用户决定、增加持久/运行复杂度，不采用。方案C沿用现有runtime worker/guard/CAS并最小补足admission和结果边界，配合前端currenttuple归属，提交独立Review确认可实现性。

Design验证：十需求逐项映射、状态矩阵、错误恢复、source-bound接口、Contractdiff/引用/identity、Pointer+Keyboard方案、无新增依赖、source不变、原Golden真实FAIL。产品tests/NativeUI/LintBuild实现验证本轮NOT_RUN，不冒充交付通过。实现阶段测试验证新行为而非重复认证sing-box能力。

## Ownership与单Task顺序不变

TASK018 DONE及其历史target、REWORK、Approval不可改写。HV038001–004仍OPEN归019，005仍OPEN归020。TASK020依赖019，不在此设计RuleSet或物化它。TASK019只有Design/Contract批准、Task绑定、readiness及对应stage checkpoint全部满足后才可实施；此Candidate不产生HumanVisualAPPROVED或DeliveryPASS。

## 相对revision003的独立Review修订

完整Contract三处版本模板更新为0.4；autoApply的busy/owner/pending/wake原子归属使用同一admission短临界区，覆盖全部guard获取/释放和错误路径。详见auto-apply-design002与technical-design003。revision003/Candidate001保留，仍不改有效Contract0.3或已批准ownership revision002。

revision005取代revision004的external-ref采用方案，关闭目标为Review002 TASK019-DR002-001。UI scope明确Subscriptions页，S是第一个合法producer；Candidate003须经新的独立Review确认可执行，本文不预设PASS。R01–R10不变，autoApply002和完整0.4Contract002保持相同被审字节。仅新增完整Subs认证责任及明确的阶段顺序，不新增Subscriptions源码重设计权限。
