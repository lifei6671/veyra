# TASK-019 Technical Design 003 — composite candidate

Status: CANDIDATE / NOT FROZEN. 本轮仅Requirement、Contract/DCR候选、Design与Review，不实施。

## 组成与追踪

- requirements002 R01–R10逐项承接USER十项产品决定。
- desktop-ui-design003负责Proxies双视图/compactnode、RoutingPointer/Keyboarddrag、Toast3000ms/noClose/action和UIintegration。
- auto-apply-design002负责可信Ready-only admission、保存成功receipt、latestgeneration/operation归属、retry-currentDesired以及失败/unknown/Stop竞态。
- ui-contract-0.4-candidate002是非生效完整Contract候选，delta002给出相对当前0.3的精确变更；DCR026revision005披露Material与适用范围。

## 需Human精确批准的工程范围

Frontend：ProxiesPage.tsx、RoutingPage.tsx、styles.css的相应页面与反馈；proxy-routing.ts现有provider/types/strictparser/response-order；ToastHost.tsx success3000ms无close+typedRecoveryAction+scopedreplace/clear；App.tsx仅给现有ProxyRoutingProvider传入由setActivePage实现的RecoveryNavigation回调，不动AppShell/Sidebar几何或runtime按钮业务。现有threefront-endtests+Toast tests随对应行为更新，不新建测试框架。

Backend：application/proxy_routing.rs的post-save协调、savedreceipt/disposition与retryCurrentDesired union、snapshotattemptmatching；managed_observation_runtime.rs的Ready-only mode、routingattemptidentity、单volatile latesttuple+coalescedwakemarker及guard内检查。只在这两Rust文件的既有测试模块补必要deterministiccases。

这是对TASK019 DRAFT“后端只读”和App.tsx deny的精确拟议scope调整，必须由本Candidate HumanApproval采纳并正式回填后才能实现。existingcommand名不变，但Rust/TS严格IPCunion新增saved结果和retry variant是MaterialAPI变更，需同时审阅。无Domain/schema/migration/Compiler/Route排序语义/newdependency/config/newworker。Manualselector、Subscriptionactivate、现有显式Apply调用的运行意图不变；retryCurrentDesired独立Ready-only。

## 核心结果

1. 结构CRUD只执行一次；store成功是不可抹去的保存事实。后续Apply失败仍返回savedreceipt+failure，失败不回滚Desired。
2. Auto与Retry进入Ready-only admission，stopped/noactive/recovery不会触发runtime构造或Start。失败需要显式retry；unknown先刷新不伪造成功。
3. 仅旧routingApply busy期间已保存的后续generation使用单一内存latesttuple收敛；不排队CRUD、失败不自动重试、无后台timer、无持久化调度。该有界内存协调机制及其锁/marker风险为本Candidate明确审批项。
4. 普通success共享policy3000ms/noClose，只改变未来实现target；旧TASK018成功验收身份不改写。ProxyRoutingpersistentfailure提供retryCurrentDesired，匹配当前operation/generation更新，旧结果不覆盖新UI。

## 验证与集成顺序

文档阶段只校验manifest/hashes、sourcecontext、Contractdelta、designGate、independentreview。Implementation tests/Native/Coverage本轮NOT_RUN。

按照stage-plan003执行已授权设计顺序S→A→B。S共享Toast修订并建立完整Subscriptions Golden；A/B依赖实际current上游认证。共享源码变化显式触发append-only重新认证和新的Design/Task引用采用、独立Review及必要Human批准，不用隐式external ref替换。发布0.4和Task绑定也需精确新身份复核；本轮不实施。

必须保护R01–R10正反矩阵，尤其Stopped/noActive/Recovery零Start，busySave三次只处理最新tuple，oldcompletion两种时序，markerfull/idlewake/Stop/shutdown，无response重放，saved但snapshotnull，Retry不CRUD，Pointer/Keyboard取消与exactIDs，Toast2999/3000及persistentRecoveryexactonce。沿用当前验证命令，不复测sing-box协议矩阵。

## Prerequisite结论与批准限制

subscriptions-prerequisite-check001确认旧0.2原FAIL、无HumanApproval、7/15manifeststale。真实USER授权新增S后，旧ref只保留为provenance；S成为正式内部producer，缺口必须由真实证据填补。当前Contract0.3保持BINDING；新0.4及所有认证/实现检查本轮未生效或未执行。

TASK018仍DONE，HV038001–004 OPEN归019、005 OPEN归020。TASK020仅futurestub依赖019。本轮不修改bindingContract、sourceRequirement、产品源码/测试/lockfile、历史Evidence或Finding状态，不生成实现/视觉APPROVED。

## revision002 Review修订

修正完整0.4Contract的三处版本模板；autoApply把busy/owner发布与释放、pending tuple与wakeflag统一到单一短admission临界区，禁止分别采样owner，覆盖包括error/drop在内的全部guard路径。保留Candidate001和首次Review。该锁归属调整仍是待Human精确批准的Material边界，不修改任何生产源码。

Revision003: USER授权StageS后，Subscriptions正式纳入pages/stages，与Proxies/Routing取精确union。stage-plan003明确全部source、实际前置与shared-source失效后的新版本证据+Design/Task精确重绑定流程。不修改协议，不冒充只更新Evidence即可保持frozenidentity。
