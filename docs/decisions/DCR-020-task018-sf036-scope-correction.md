---
id: DCR-020
status: PROPOSED
change_source: USER:lifei explicit TASK018-DR-001 scope-correction request
affected_design:
  - .sdlc/design/TASK-018-shared-foundation-candidate-001.json
  - .sdlc/design/TASK-018-shared-foundation-alignment-001.md
affected_task: [TASK-018]
---

# DCR-020: TASK-018 SF036 DocumentEditor disabled presentation scope correction

## Frozen Design conflict

Candidate-001 identity sha256:b3325039b029027b3acb49a99fbe2b9e17c832c5628ee1414306900973be8ce3 已经获得真实 Human Design Approval。原38项 Finding 包含 SF-V02-036，但其 scope.allow 和 alignment-001 第2节将 src/components/subscriptions/DocumentEditor.css 限定为 SF021/024/029/031/033/034。

Independent Delivery Review 的 TASK018-DR-001 已证明：DocumentEditor.tsx:107–109 的现有 busy/disabled 路径会到达工具按钮；DocumentEditor.css:10 的 .document-tools > button:hover 仍匹配 disabled，且工具按钮未消费 .50 opacity/default cursor。Contract 0.2 §5 的 expected 不变：disabled opacity .50、default cursor、hover不改变视觉。此次仅修正 Frozen Design Scope，不增加第39项 Finding，不是新的业务/架构 Material Change。

## Proposed decision / 唯一生产权限差异

before: DocumentEditor.css: SF021/024/029/031/033/034

after: DocumentEditor.css: SF021/024/029/031/033/034/036

批准后，本DCR仅覆盖 Candidate-001 allow-list 及其引用的 alignment-001 第2节对应行中遗漏的 SF036；旧文件、旧hash、旧批准保持原身份，不重新解释历史许可。Candidate-002 是拟替代 TASK-018 当前执行设计身份的新修订；Human Change/Design Approval 前不替换当前 Task binding，不解除 BLOCKED。

SF036 将来仅允许在 src/components/subscriptions/DocumentEditor.css 调整现有 disabled 工具按钮 presentation：不消费hover visual、opacity消费现有 --disabled-opacity 的 .50 语义、cursor为default语义。不冻结具体CSS写法；不得增加新visual value。不修改native disabled来源、busy状态或handler。

其它所有 allow/deny、38项 inventory、ui_contract、ui_scope、source_paths、visual_gate、原业务边界保持不变。继续禁止 DocumentEditor.tsx、SubscriptionPage.tsx handler、SidebarTraffic.tsx、src/lib/**、Router/IPC/Domain/Compiler/Runtime/StateStore/Rust、TASK-011/012、Candidate-004/005、6000ms成功Notice、其它C/N项、新需求/依赖/框架/页面重设计。原文关于首次design-only权限和无需DCR是历史上下文；本次DCR的必要性仅来自冻结scope纠正，不授权其它动作。

## Impact Analysis

- Change source: 用户确认的Frozen scope contradiction，来源TASK018-DR-001；不是新UI Requirement或Contract修改。
- Requirements / Task: 仅TASK-018已有SF036文件级write permission。38项inventory原身份不变，SF036仍OPEN，其余37项现有生产修改保留，不回滚也不宣称独立PASS。
- Design / ADR: 仅Candidate-002加此修订关系及DCR引用；不改accepted业务ADR、Candidate004005或原审计。无新数据/协议/权限/架构决定。
- Code / tests / data / deployment: 本轮零生产修改；未来只在已有DocumentEditor.css修上述状态。现有测试方式不变，无数据migration/deployment变更、无新依赖。
- Gate invalidation: 批准后才物化新binding并重新执行受影响Design/materialization/readiness/checkpoint。旧Design Approval仍证明Candidate-001历史批准，不适用于Candidate-002。旧implementation/test/Review仅证明旧target；修复后必须生成新的implementation target identity、closure、相关lint/test/build及独立Delivery/UI Review和真实Human Visual/Task Approval，绝不复用旧target作为新实现身份。
- Compatibility / rollback: 无业务兼容性风险；保留当前工作区，不自动回滚37项。未来CSS变化会使包含该文件的视觉manifest/截图/Review身份不再代表当前源码，必须刷新适用Evidence；不改历史记录。停止修复即可保留当前BLOCKED恢复点。
- Downstream: TASK018 scoped remediation仍不等于Subscriptions完整Certification。不重跑Golden Certification/TASK012 readiness、不实施Proxies/Routing。6000ms及其它C/N差异仍待后续page-specific只读审计。
- Required approval: 独立Technical Design Review通过后，等待USER对本DCR内容hash与Candidate-002内容hash的精确Human Change/Design Approval。该批准不等于Human Visual Approval。本轮不物化、不修SF036、不更新旧target或closure、不生成Human APPROVED。

## Change provenance / identity semantics

| Artifact | Identity |
| --- | --- |
| .sdlc/design/TASK-018-shared-foundation-candidate-001.json | sha256:b3325039b029027b3acb49a99fbe2b9e17c832c5628ee1414306900973be8ce3 |
| .sdlc/evidence/TASK-018/implementation-target-001.json | file sha256:e3acea8a4adece3a163893dcffeceda9fb859fa6cc3cb3d1bc1c8c114f066060; implementation target sha256:ecac1f890451b1ff42679ea51c230ef1641c067e94dd7a3f2a47b08bd1446767 |
| .sdlc/evidence/TASK-018/delivery-review-001.json | sha256:56d991010ae83ba5d63abbc456d432fc87ec89987ee453e8c9cdf7247249e7cc; FAIL/REWORK TASK018-DR-001 |
| .sdlc/evidence/TASK-018/finding-closure-002.json | sha256:b0efaaa9e3bae4c142bd83e30c083f060973c777cfc2d7e7596d694015f74868; FAIL / SF036 OPEN |

Candidate-002 source_manifest原样保留Candidate-001的设计时源码快照，不声称四个已修改文件仍是当前hash。当前执行源码身份由上表implementation-target-001的manifest绑定，独立核对时必须验证此当前manifest；这只是provenance，不是更新implementation target。审计/Contract/design manifest继续精确绑定旧内容；新增DCR manifest绑定本补充。后续修复后必须产生全新交付身份。
