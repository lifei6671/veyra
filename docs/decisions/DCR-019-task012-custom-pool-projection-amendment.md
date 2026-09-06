---
id: DCR-019
status: PRODUCT_SEMANTICS_ACCEPTED_DESIGN_PENDING
change_source: USER:lifei 2026-09-06 explicit TASK-012 remediation decision
affected_design:
  - docs/decisions/DCR-018-subscription-use.md
  - .sdlc/design/TASK-011-subscription-expansion-006.md
  - .sdlc/tasks/TASK-011.md
affected_task: [TASK-011, TASK-012]
---

# DCR-019：显式 Custom Pool 跨订阅投影 amendment

## 已接受的产品语义

本决定只 amendment TASK-011/DCR-018 的 active-subscription runtime projection；不改变订阅“使用”
立即应用、其它订阅默认隔离、稳定身份或 `selectionConflict` 的 fail-closed 原则。

原规则“运行投影只含 active subscription 的 Provider/Node”替换为：

1. active subscription 的 Provider、Node 与 runtime implicit Pool 是基础闭包；
2. 用户显式启用的 `PoolKind::Custom` 是唯一跨订阅闭包根；
3. 每个根只沿 `Custom Pool → PoolSource.provider_id → 该 Provider 的 Node → NodeFilter::matches`
   扩展，加入精确命中的 Provider/Node 与该 Custom Pool；
4. 不递归沿 Route、其它 Pool、Subscription sibling 或显示名称扩展；未命中的节点、同订阅其它
   Provider、其它未被 source 引用的订阅和 disabled Custom Pool 均排除；
5. `ImplicitProvider` Pool 仍只能使用所属 active subscription 的 Provider/Node，不能成为跨订阅根；
6. Custom Pool 的 Manual `selected_node_id` 必须属于该 Pool 的闭包成员；UrlTest 闭包至少一个成员；
7. default target 或 enabled Route 引用 disabled、缺失或闭包为空的 Pool，或者 Manual selection 在
   订阅刷新后离开闭包，继续返回 `selectionConflict`；不自动换节点、启用 Pool、改 target 或回退隐式 Pool；
8. 只有已通过完整 `AppState::validate` 的状态可以构建闭包，闭包不持久化、不修改 PoolSource。

这明确 supersede：

- `.sdlc/design/TASK-011-subscription-expansion-006.md` 第 3 节第 1、2、5、7 项中“只纳入目标订阅”
  及“跨订阅硬引用一律冲突”的绝对表述；
- `.sdlc/tasks/TASK-011.md` SF-004 中与上述绝对隔离相同的验收表述；
- `docs/decisions/DCR-018-subscription-use.md` 中“其它订阅节点不自动进入”的含义收窄为：不得自动
  引入，但用户显式 Enabled Custom Pool 的精确 source/filter 闭包属于明确配置，不属于自动引入。

其它 TASK-011/DCR-018 条款继续有效。TASK-011 的历史交付与验收 Evidence 不被重写；本 amendment
的生产实现和验证归 TASK-012，Technical Design Review 通过前不得实施。

## 最小实现与影响

- 只修改 `project_selected_runtime` 的节点/Provider/Pool 投影算法及定向测试；不改 StoredStateV6、
  AppState/RuntimeIntent/SingBoxCompiler 类型或 tag 规则。
- 不引入 RuleSet、Inbound、递归依赖图、自动订阅合并、后台同步或新的持久字段。
- 订阅刷新造成闭包失效时沿用整体校验/应用失败保护；当前已运行 child 不从修改后的 AppState
  重建为“旧配置”，仍由其已应用 artifact 和 identity 表达。

## 回滚约束

一旦用户保存跨订阅 Custom Pool 并将其作为 default/enabled Route 目标，旧 projection 代码虽然能
读取 StoredStateV6，但可能无法运行该配置。因此该增量发布后默认只允许 roll-forward。若必须代码
回滚，先用新版本把 default/Route 改到 active subscription 可用的 implicit/same-subscription Pool，
再禁用跨订阅 Custom Pool并成功 Apply；完成前不得把旧版本标记为可安全运行。运行中的旧 child
不自动停止或重建；Stopped 与重启后必须在旧 projection 下通过一次显式 Apply/Start 验证。

## 必须验证

- active subscription 基础闭包保持；未引用 subscription/provider/node 不进入；
- enabled Custom Pool 精确加入多个 source/filter 命中的成员；disabled、空成员、悬空引用和 Manual
  selection 漂移返回明确冲突；
- default/Route 只消费闭包内有效 Pool；同名对象不会通过显示名产生依赖；
- 旧绝对隔离用例更新为“无显式 Custom Pool 时仍隔离”，并新增 amendment 正反矩阵。
