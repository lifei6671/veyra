# Change Control

## 先分析，后修改

任何已批准输入发生变化时，先创建 Impact Analysis：

```text
change source
affected requirements/stories/tasks
affected design/ADR
affected code/tests/data/deployment
gates to invalidate
compatibility/migration/rollback risk
recommended action and required approval
```

未完成影响分析前，不得把“修改了文档”解释为已安全采用新需求。

已接受 Requirement、功能清单、产品语义或 Acceptance 的任何变化，都必须在 Impact Analysis 后获得
用户明确确认。Verification strengthening 或等效实现可在当前 Scope 内自主调整；删除必需证据、降低
覆盖或改变验证所证明的产品语义时，必须确认。生产/破坏性验证仍需显式授权。未确认前不得修改
canonical Requirement/Task、恢复受影响实现或启动后续 Task；确认后记录 approval reference，并失效
受影响的子功能验收、Task 验收及下游 Gate/Evidence。

Foundation/Material dependency 必须先确认。请求必须说明 package、version/range、用途、影响、
替代方案、验证方式和预期 manifest/lockfile/checksum 变化；未确认前不得运行安装命令或修改这些文件。
Foundation 的一次批准覆盖其中精确列出的依赖及正常传递依赖变化，超出批准范围时重新确认。低影响
dev/test dependency 或可逆开发工具可以按 L1 Assumption 自主引入并记录，除非项目规则要求
`dependency_policy: confirm_all`。

## 默认失效矩阵

| 变化类型 | 默认失效 |
| --- | --- |
| Business Goal / Requirement / Acceptance | Requirement、Design、Planning、QA Case、Delivery、QA、Release、Observation 中受影响部分 |
| L2 Material Decision（Architecture / Runtime Dependency / Protocol / Persistent Data / Security） | Design、Planning、Delivery、QA、Release 中受影响部分 |
| Task Scope / Material Dependency / Verification weakening | Planning、该 Task Delivery、相关 QA/Release |
| Implementation | Code/Delivery Review、相关 Test Evidence、QA、Release Readiness |
| QA Case | QA Case Review、QA Result、Release Readiness |
| Release Plan / Migration / Rollback / Operational Config Contract | Release Review、Release Readiness、相关演练 Evidence |
| Observation Metric / Experiment | Experiment/Observation Gate 与最终业务结论 |

矩阵给出保守默认值。Impact Analysis 可以缩小失效范围，但必须用可追踪的依赖和证据说明；不能为了少重验而猜测不受影响。

## Design Freeze 与 DCR

Implementation 发现以下情况时立即停止：

- 现有 Design 无法满足 Acceptance；
- 需要改变 Architecture、Module Boundary、重要 Runtime/Operational Dependency、持久数据语义、
  Public Protocol、Deployment 或 Security Model；
- 实现与已接受 ADR 冲突。

若尚无覆盖该问题的 Frozen Design 或 ADR，首次 L2 Material Decision 不是 Change：

```text
STOP implementation
  -> route technical-design for the affected concern
  -> create ADR only when the decision needs durable rationale
  -> review / request Human approval as required
  -> freeze the first design decision
  -> rebuild task context
  -> resume implementation
```

只有已有 Frozen Design / accepted ADR 且必须改变它时，才进入 DCR：

```text
STOP implementation
  -> record frozen-design conflict
  -> create DCR
  -> impact analysis
  -> technical review / human approval as required
  -> update design and ADR
  -> invalidate affected gates
  -> freeze new design version
  -> rebuild task context
  -> resume implementation
```

不要在代码里临时塞 fallback、兼容层、Feature Flag 或额外配置来绕开 DCR，除非它本身已被批准为方案。

## Scope Change

完成 Task 前比较：

```text
git diff --name-status
git diff --cached --name-status
untracked files
task.scope.allow / task.scope.deny
```

发现越界时：

1. 不继续修改越界区域；
2. 区分预存用户改动、当前 Delivery-owned 改动和未知归属；
3. 解释为何当前 Task 需要扩展；
4. 建议扩展当前 Scope、创建新 Task；只有 Scope 变化需要改写 Frozen Design/ADR 时才建议 DCR；
5. 等待明确授权。

不得为了让 Scope 检查看起来通过而移动、隐藏、回滚或删除用户改动。

## 用户变更请求

用户当前明确指令权威最高，但仍需把它转换成可追踪变更：

- 不改变 Requirement、Acceptance 或 Verification Contract 的小型同范围实现细节：在当前 Task
  Scope 内继续并记录必要 Assumption，不伪装成需求变化。
- Requirement、功能清单、产品语义、Acceptance 或 Verification weakening：Impact Analysis +
  用户明确确认 + Gate/Evidence 失效；strengthening/equivalent Verification 可自主执行。
- Foundation/Material dependency：Impact Analysis + 用户明确确认；批准引用写入当前 Task 或
  Foundation。低影响 dev/test dependency 记录 Assumption 与验证，除非项目规则要求 confirm-all。
- Scope、Material Dependency、持久/公共/运行契约或设计变化：Impact Analysis + 对应授权 + Gate 失效。
- 新业务目标：新 Story/Epic 或显式替换当前目标。
- “继续”：不是需求变更、第三方依赖、Scope、Material Contract 或高风险 Gate 批准；仅当前唯一待决项为刚展示的
  Requirement Baseline，或本轮刚展示、identity 未变化、独立 Review 当前的 Foundation 时，明确同意可确认
  该对象。它不能授权生产、破坏性 Migration 或未列出的 Material dependency。

## Blocker

无 Blocker 时写 `blocked: null`。项目级 Blocker 使用 State 顶层非空 `blocked` 对象，字段为
`origin_phase`、`reason`、`owner`、`scope`、`unlock_condition`、`next_check`。不要另造第二套
恢复字段或要求 State 中不存在的 Blocker ID。能够在现有授权内安全解决的技术阻塞继续排查；
需要新权限、外部协调、Material Contract 变化或重大范围扩张时停止并请求方向。
