# Veyra UI Contract Gate

本项目唯一生产 UI Contract 为 `docs/ui/veyra-ui-spec.md`。它覆盖参考图和历史审计中与其不一致的视觉值；参考文件仍是来源证据。仅在本 Veyra 项目应用以下规则，不迁移全部历史 Task，不改写历史 Design approval。

## 识别与工件

每次新建或重新进入 Design/Planning/Implementation/Delivery Gate，Producer 和 Reviewer 必须从正向 Scope、affected components、requirements 判断用户可见影响，不按 tsx/css 扩展名分类。AppShell、Sidebar、Page、PageHeader、视觉组件、CSS/style/token、Dialog、Toast/Notice、dense list、前端交互状态或用户可见布局的修改属于 UI-impacting。纯 DTO、数据解析或非视觉前端调整不因此成为 UI Task。只出现在 deny/history 的名称不算影响。语义分类由 Reviewer 对 Scope/diff 复核，自动关键词检测是补漏，不能覆盖人工识别出的 UI 影响。

已有 JSON Design Candidate 在顶层保存下列字段；Markdown Design 和 Task 在唯一的 `## UI Delivery Metadata` 下保存一个标准 `json` fenced block。这只是现有自由格式 Design/Markdown Task 的可选机器可读部分，不改变 front matter 或历史 Evidence schema。非 UI Task 不要求此块。Task 的 `task_id` 必须等于 front matter `id`；Design 的 task_id 必须与待物化 Task 一致。

```json
{
  "task_id": "TASK-XXX",
  "ui_impact": { "required": true, "reason": "正向 Scope 中的具体视觉/交互改动" },
  "ui_contract": { "document": "docs/ui/veyra-ui-spec.md", "version": "0.2", "required": true },
  "ui_scope": {
    "pages": ["Subscriptions"],
    "components": ["PageHeader", "Dialog", "Notice"],
    "states": ["normal", "hover", "selected", "disabled", "loading", "empty", "error", "busy", "focus", "success"],
    "rules": ["§4 Typography", "§6 Composition", "§11 Screenshots"]
  },
  "visual_gate": { "required": true, "human_approval": true }
}
```

`pages` 使用当前 Contract 的标准页面名：Overview、Subscriptions、Proxies、Routing、Settings、Connections、Logs；不接受拼写变体，以免遗漏 Golden Page 前置。共享组件列出实际影响页面。`rules` 必须指向本 Task 适用的真实 token/rule，不复制整份 Spec。状态不适用时只能在 `ui_scope.state_exclusions` 中以状态名记录具体理由，Reviewer 对照真实操作确认；dense page 还须声明 expanded/collapsed/dirty/applied/all-nodes，Dialog/Menu 等按实际组件追加。不得为通过机器检查缩小 scope。物化时逐项保留 Design 的 ui_contract/ui_scope/visual_gate/ui_stages/ui_verification；缺字段、路径不存在、版本不等于当前 APPROVED/BINDING 版本、scope 不完整均为 Design Review `FAIL/REWORK`。

## 执行点

现有 Skill 仍负责流程路由和状态迁移。`scripts/sdlc-ui-contract.mjs` 是只读检查函数，不是 CLI、状态写入器或 Policy Runtime。必须在对应 Gate 调用它并保存实际结果；未执行是 NOT_RUN，不得仅用本文作为通过依据。

从仓库根目录使用已有 Node 运行函数，例如 Design Review：

```powershell
node --input-type=module -e "import {evaluateUiGate} from './scripts/sdlc-ui-contract.mjs'; const r=evaluateUiGate({root:process.cwd(),artifact:'.sdlc/design/实际候选.json',phase:'design'}); console.log(JSON.stringify(r)); process.exitCode=r.result==='PASS'?0:1"
```

调用参数使用当前实际路径，不执行上面的占位候选。各入口：

- Technical Design Producer 完成候选后、Reviewer 审查及 Orchestrator 冻结前：`phase: 'design'`，artifact 为实际 Candidate。
- Task materialization/readiness：`phase: 'readiness'`，artifact 为实际 Task，design 为当前已批准候选；原有 Design approval/identity 检查仍必须通过。
- Implementation 开始和 checkpoint：`phase: 'checkpoint'`，artifact 为 Task；有 ui_stages 时必须传 `stage: 'A'` 等实际阶段 ID。先检查外部 Certification，再检查本阶段使用的同 Task 上游阶段认证；不得以 Task readiness PASS 代替阶段入口检查。
- Compliance：`phase: 'compliance'`，artifact 为 Task（分阶段时传 stage；TASK-019 两级模型的最终 Compliance 不传 stage）或独立 Certification envelope。只检查机器/独立 Review 证据；不要求 human approval，可在此 PASS 后展示给人工。
- 阶段验收：`phase: 'delivery', stage: 'A'`，只核验该阶段及前置，返回 `scope: 'stage'`，绝不代表 Task Delivery PASS。
- 最终 Delivery Reviewer 和 Orchestrator DONE 前：`phase: 'delivery'` 不传 stage，targetIdentity 为当前整体交付身份；检查所有 stage 的当前 compliance/human approval，再检查整体 Task compliance/human approval。缺任一阶段或最终整体批准都不能 DONE。
- 独立 Golden Page certification：`phase: 'certification'`，artifact 是下述 page envelope，不是 Task。它只验证当前页面证据，不授予实现权限，不迁移 Task/Phase/Gate。Compliance PASS 不等于 Certification PASS。

Subscriptions 是第一张生产 Golden Page。Task 的 `ui_scope.golden_pages` 只绑定外部认证：`{"page":"Subscriptions","certification_ref":".sdlc/evidence/golden-pages/Subscriptions/<版本化认证记录>.json"}`。不再按 task_id 回读原 Feature Task；旧记录保持可读，但不能用旧功能验收冒充新版 Certification。新引用需由当前 Design/Task 评审后采用，不批量修改历史 Task。

external 指产生于当前 Task 之外：既可引用 standalone certification_id，也可复用另一个 Task 已建立的 `golden_page_certification` stage envelope。两者都必须通过当前源码/证据检查，且都不读取历史 Task；下述 stage_reference certification 仅限同 Task，禁止用作 external 或 standalone。当前 Task 自己产生的 Golden Page 只能走下面的内部 stage 前置，不能伪装为 external。

### 同 Task 的阶段前置

`ui_stages` 是可选的、有序的 UI 验收清单，不增加 Task/Phase 状态，也不调度代码。每项包含：`id`、`ui_scope`（一张页面及实际 components/states/rules/source_paths）、`requires_golden_pages`、`evidence_ref`，建立 Golden Page 的阶段额外声明 `produces_golden_page`。页面不得重复；各 stage 的 pages/components/states/rules 并集必须与 Task scope 一致；source_paths 必须覆盖影响该页的真实源码及共享 CSS/primitive，由独立 Reviewer 核实完整性。

Task-level readiness 仅检查未由本 Task stage 产生的 Golden Page；阶段 checkpoint 才检查它消费的内部 Golden Page。前置必须由更早 stage 产生；不能删掉 producer 声明或倒置顺序绕过。无 intra-task dependency 的旧单阶段 Task 沿用原模型；存在内部依赖时必须显式分阶段，不能靠声明一个外部自引用绕过。

TASK-012 的新 Candidate-005/supplement 应表达下列结构（以下是设计形状，不是已批准 Candidate；完整 ui_scope 必须有真实映射，不使用占位内容通过 Gate）：

```text
UI Contract: docs/ui/veyra-ui-spec.md / 0.2 / required
Task scope: Proxies + Routing
visual_gate: required=true, human_approval=true
external golden_pages: Subscriptions -> certification_ref
ui_stages:
  A: ui_scope.pages=[Proxies]
     requires_golden_pages=[Subscriptions]
     produces_golden_page=Proxies
     evidence_ref=.sdlc/evidence/TASK-012/ui-stages/A/certification.json
  B: ui_scope.pages=[Routing]
     requires_golden_pages=[Proxies]
     evidence_ref=.sdlc/evidence/TASK-012/ui-stages/B/acceptance.json
```

顺序是：新 Candidate Review/所需批准 → Task external readiness → A checkpoint/实现/compliance → A human approval → Proxies Certification 成立 → B checkpoint/实现/compliance/human approval → 整体 compliance/最终 human approval/Delivery。Stage B 改动共享 CSS 或 A 页面源码时，A 认证失效，必须重新检查并取得对新目标的批准；不能为让认证保持 PASS 而漏列共享文件。

### Golden Page / Stage Evidence envelope

独立认证建议放在 `.sdlc/evidence/golden-pages/<Page>/`；Task stage 放在 `.sdlc/evidence/<Task>/ui-stages/<Stage>/`。实际引用路径由工件声明，无需迁移历史 Feature Task。

```text
kind: golden_page_certification | ui_stage_acceptance
subject: standalone uses certification_id;
         stage uses task_id + stage_id (mutually exclusive)
page: canonical page name
contract: docs/ui/veyra-ui-spec.md
contract_version: "0.2"
ui_scope: pages/components/states/rules/source_paths (+ justified state_exclusions)
git_identity: 40-character capture-time Git commit SHA
manifest: [{path: repository-relative source path, sha256: "sha256:..."}, ...]
prerequisites: [{page: Subscriptions, certification_ref: path, sha256: "sha256:..."}]
target_identity: sha256(JSON.stringify({git_identity, manifest}))
compliance: {path, sha256}
human_approval: {path, sha256} # only referenced after real human supplies approval
```

`sha256(...)` 对 UTF-8 JSON 字节计算 SHA-256 并加 `sha256:` 前缀；对象顺序固定 git_identity、manifest，manifest 按记录中的顺序，元素为 path、sha256。校验每个源码文件当前 hash；Git 身份作为捕获 provenance 绑定在 target identity 中，不要求后来无关 commit 的 HEAD 仍相同。截图/DPI/独立 Review 沿用下面 compliance 格式，不复制第二份。独立认证的 compliance/human record 用 certification_id 代替 task_id；stage record 用 task_id 并加 stage_id；身份与 scope 必须一致。

Envelope 不包含 Agent 自批的 APPROVED 状态；只有引用的真实 human record 满足批准条件时，certification 检查才 PASS。`ui_stage_acceptance` 用于不建立 Golden Page 的阶段，kind 必须与阶段的 produces_golden_page 声明一致。Envelope 所引用的 compliance 和独立 Review record 还必须保存同值 `certification_kind`，人工批准的 compliance hash 因而绑定认证类型；不得只改 envelope 的 kind 把普通阶段提升为 Golden Page。截图文件/hash、适用环境矩阵、状态、前端验证、独立 Review、compliance hash 和 human hash 都实际检查；TASK-019 两级模型按下节执行，其他路径保留现有完整环境要求。Compliance 阶段可暂缺 human_approval；不得因此进入下一受约束阶段。

认证的 `prerequisites` 必须精确对应 Contract 顺序：Subscriptions 为 `[]`，Proxies 引用 Subscriptions，Routing/Connections/Logs 引用 Proxies。每项绑定认证文件 hash，并递归检查上游当前源码、compliance、真实 human approval；独立认证与跨 Task 阶段认证均不能遗漏上游。Compliance 和独立 Review 必须保存同值 `prerequisites`，由人工批准的 compliance hash 绑定；单改 envelope 无法补造认证链。

如果 Subscriptions 当前实现无需修改，可在明确的只读 certification 范围内收集当前版本的上述证据并等人工批准，不重新打开 TASK-011 的 Design/Approval。如果检查发现不符合 Contract，停止认证并报告实际差异；提出仅涉及受影响页面/shared visual sources 的有界 UI remediation change unit，经正常 Scope/Design/Task 授权与 readiness 后再改代码。Certification 本身不提供代码修改授权，也不允许自动调度后续 Task。

## TASK-019 两级验收（Contract0.5 §11.5）

本节实现 Gate022 批准的 evaluator design008 与 final-evidence design009。仅当当前已采用 Task/Design 同时声明 ui_verification.model=stage_reference_then_final_matrix，且为 TASK-019、S=Subscriptions → A=Proxies → B=Routing 的既有依赖结构时适用。未知/不完整 model、其它 Task、shared delivery 混用或不一致 Task/Design metadata 均失败；不能由 Evidence 自报来启用模型。

Stage Reference Validation：每个 Stage 提交的 Native screenshot 必须为 Windows Tauri + Light + 150% + 1280×720 logical viewport，覆盖该阶段全部真实 applicable states 与行为。focus/keyboard、error/recovery、Dialog、完整 Compliance、独立 Review 与真实 Human Visual 要求不变。仅 S 移除 toast-persistent；总体/A/B 保留；S 的 Toast、RecoveryAction、§6.7 与 shared typed action 自动化/source review 保留。不能伪造不存在的 Subscription persistent 产品路径。

Stage envelope/compliance/review/Human 都保存 verification_model=stage_reference_then_final_matrix 与 verification_level=stage_reference，并匹配 Task、Stage、scope 和 Contract。S/A 保留 golden_page_certification kind，B 保留 ui_stage_acceptance；reference Golden 仅供同 Task 下游 checkpoint 使用。generic certification、外部 Task、standalone 不接受 reference Golden。递归上游检查沿用同一个已校验 Task 的 expected producer、exact evidence_ref、scope、hash 与真实 Human。A 必须先有当前 S；B 必须先有当前 A 及其当前 S。

S/A/B 全部完成且共享源码稳定后，Task final matrix 在 Subscriptions、Proxies、Routing 各覆盖五环境（1280×720@100/125/150%、1440×900@100/125%）的 Light 与 Dark normal，共30个覆盖单元。不要求 state×theme×DPI 笛卡尔积；所有状态和行为仍由当前参考认证完整证明并由 final Review 对账。最终阶段不能仅凭五张混合主题图满足十个组合。Final Compliance/Review/Human 使用同一 model 和 verification_level=task_final_matrix。

Final Compliance 保存 git_identity、精确 source union manifest（Task 与全部 Stage source_paths 的并集，无重复或遗漏）、target_identity=sha256(JSON.stringify({git_identity,manifest}))。每条当前 Stage source manifest 必须与 final union 中对应 bytes/hash 一致。Final Compliance 和独立 Review 保存相同 stage_references=[{stage_id,path,sha256}]，严格按 S/A/B 顺序匹配 adopted Task evidence_ref 与实际证书 hash。当前 Stage certificate 及真实 Human/prerequisite 链全部通过后，才可通过 final Compliance；最终 Human 则绑定 final compliance hash 与全部最终截图。

Task/Design 的 ui_verification.final_evidence 明确声明 compliance_ref 和 human_approval_ref，恰好两个字段，路径为 .sdlc/evidence/TASK-019/final-visual/compliance-NNN.json 与 human-approval-NNN.json，NNN 为相同的非零三位版本号。此处只声明将来的不可变输出，不创建 placeholder。路径不得越界、指向其它 Task、通过 alias/symlink/latest 间接选择；无旧固定路径 fallback、扫描最新版本或 caller override。Final Compliance.review.sha256 验证其 evidence_ref 原始 bytes，真实 Human 的 compliance_sha256 进一步绑定该 Review 与 Stage chain。

Final phase=compliance 不传 stage：验证所有当前 Stage references（含其 Human），再验证精确 final compliance_ref；只校验声明的 Human 路径格式，不读取或创建尚未提供的最终 Human 文件。Final phase=delivery 不传 stage：还必须从精确 human_approval_ref 取得真实最终批准。Checkpoint 始终要求实际 stage，不能用 no-stage final route 绕过阶段入口。

Evidence 一旦存在不得覆盖。失败/不完整、源码变化、review/compliance 变化或新的 Human decision 产生新的版本化 pair，保留旧结论，经新的精确 Candidate/Task 引用采用后才读取新记录。共享文件变化仍使受影响上游 reference stale，需要按参考环境重新验证、独立 Review、真实 Human 与 immutable adoption；不得为保持 PASS 而漏列共享文件。最终源码变化也使旧 final matrix 失效。

完整 final matrix、最终 Compliance、完整独立 Delivery Review、最终 Human Visual 和 Task Acceptance 未完成，不得 DONE/CLOSED 或解锁 TASK-020。Reference Golden 不自动提升为外部完整 Golden。此模型不增加 waiver/N/A/bypass、不以 Mock/fixture 替代 Native，也不授予产品实现、Task adoption 或下一阶段权限。

## Evidence 与人工边界

复用 `.sdlc/evidence/<TASK>/`，新增记录不修改旧记录。Compliance 保存 `task_id, contract, contract_version, target_identity, produced_by, result (PASS|FAIL), checked_pages, checked_components, checked_states, checked_rules, screenshots, findings`。

- `verification`：每个 `pnpm lint`、`pnpm test`、`pnpm build` 保存 command、exit_code、result、target_identity、evidence_ref。被引用 JSON record 必须保存实际 command_or_method、exit_code、result、target_identity、timestamp、observer、summary，并与当前验证对账。可以定向 tests，实际命令以检查类别开头；command 字段表示所属检查类别。不得用同一无命令记录冒充三项检查。
- `review`：result、reviewer、evidence_ref；被引用 JSON record 保存 result、reviewer、target_identity、timestamp、command_or_method、summary 与 coverage（pages/components/states/rules 数组），与 Task 和当前独立 reviewer 对账。独立 Reviewer 对实际 CSS/token/composition 及 Spec §11.2 逐项检查，只检查 relevant rules，保留有定位依据的结果。机器检查这些证据的存在/关联，不宣称能自动判断任意 CSS 的视觉正确性。
- `screenshots`：每项为 path、sha256（`sha256:` 前缀）、target_identity、source=`tauri-windows`、page、states 数组、theme=`light|dark`、width、height、scale（百分数）。使用仓库相对路径。所有声明状态在 Task 涉及页面中覆盖，Reviewer 核对状态实际归属（例如 all-nodes 只属于 Proxies，不要求 Routing 伪造此状态）。TASK-019 两级模型分别按下节执行；其他路径逐页覆盖浅深 normal 及 Spec §10 完整五项矩阵：1280×720@100/125/150%、1440×900@100/125%。Mock 不可替代真实 Tauri。

人工文件 `ui-human-approval.json` 必须由 human 提供，Task 记录含 `task_id, contract, contract_version, target_identity, approved_by: human, recorded_by: human, decision: APPROVED, timestamp, reviewed_screenshots, compliance_sha256`。独立认证将 task_id 换为 certification_id，stage 记录增加 stage_id；后两项绑定实际看过的全部截图路径及 compliance 文件 SHA-256；任何变化须重新人工批准。Agent 可展示所需字段和计算哈希，但禁止生成、写入或补填 APPROVED 人工文件，禁止将旧功能验收转换为视觉批准。检查函数只读文件；它不能鉴别一个恶意伪造 `human` 字符串的操作者，因此 Orchestrator 必须核实真实人工来源，不把 JSON 标签当成身份认证。本项目保持既有 soft enforcement 边界，不新增认证系统。

历史完成 Task 不批量重验；新产生/重新进入 Gate 的 UI Task 执行本规则。TASK-012 本次明确重新进入视觉验收，保留功能完成与旧批准，Task 使用 VERIFYING、整体验收 PENDING、Delivery PENDING，不重写冻结设计，也不开展 UI remediation。重新开始 UI 修改之前仍须满足当前 Design/readiness 与 Golden Page 前置。

## Cross-cutting shared UI delivery（显式、向后兼容）

ui_scope.delivery_mode 缺失或 page_migration 时，原有 Golden dependency/stages/readiness/compliance 行为不变。cross_cutting_shared 仅适用于共享 overlay/primitive 及必要展示接入；不得包含页面迁移/重设计、Golden 生产或认证。混合任务必须拆清边界，不能靠模式字段豁免页面迁移。

该模式保留 pages 的完整实际影响范围（blast radius）。另声明 shared_components（components 的非空子集）、source_paths，以及 verification_pages（pages 的非空代表集）。代表集必须覆盖真实消息 owner / integration context；ToastHost 至少 Overview、Subscriptions、Proxies、Routing，Settings/Connections/Logs 保留影响范围但不重复页面认证。scope.states/rules 仍必须完整描述本次共享交付；既有历史 concern 通过历史身份追踪，不能作为新的页面迁移权限。

不能仅由 Producer 添加开关。ui_scope.shared_review 必须引用独立分类 Review JSON：result=PASS、delivery_mode=cross_cutting_shared、page_migration=false、reviewer 与 produced_by 非空且不同、timestamp、summary 和 scope_identity。独立 Reviewer 必须检查正向 write scope、实际共享源码/消费者、完整影响页面、代表上下文和 deny，拒绝页面迁移或混合 scope。此 Review 不代替正式 Technical Design Review/Human Design Approval。

scope_identity 为 sha256(JSON.stringify({task_id, scope, ui_scope}))，其中 ui_scope 仅去掉 shared_review 引用本身，保留其余全部字段；scope 为 Candidate/Task 的正向/负向 scope 对象。序列化前递归按对象键名排序，数组顺序保留；对象键重排不改变分类身份。这样无自引用 Candidate hash；变更 allow/deny、pages、shared_components 或 verification_pages 都要求独立重新分类。JSON Task/Design 对齐检查保持原有 ui_scope 深相等；Markdown Task 必须在 UI Metadata 同时物化相同 scope 对象以保持分类身份，不能只复制模式开关。

该模式禁止声明 ui_stages、produces_golden_page 或 golden_pages（含空字段），禁止作为 page certification envelope 使用。不自动要求 external Golden，也不推导自己尚未产生的 Golden stage。该模式本身不是 Golden Page Certification，绝不授权后续 Migration。

Compliance 与独立 review 的 checked_pages/coverage.pages 仍覆盖所有 affected pages，保留 source coverage、命令证据、原生截图 hash/target identity、真实 Human Approval 等原检查。截图按 verification_pages 收集：每个 shared component 在每个代表上下文至少有实际截图；所有声明 states 在组件截图集合覆盖；light/dark ×100/125/150% DPI 在共享组件集合覆盖，使用正数实际 viewport 尺寸和 normal 状态。无需对每个 affected page 各跑原五组完整页面矩阵。截图须声明 components，不能用无共享组件的页面图替代。组件行为/stacking/modal错误等具体验收由完整 scope states/rules 和独立 Review 确认。未执行或缺失 native verification 必须 FAIL/NOT_RUN，不以模式跳过。

模式仅改变 Golden 前置推导和矩阵的聚合单位；不放宽 Contract、Review、Human gate，不改变 schema/state machine/CLI。历史 Evidence 不迁移。基础设施修改独立记录测试与 Review，不进入产品 Task 的源码 write scope。
