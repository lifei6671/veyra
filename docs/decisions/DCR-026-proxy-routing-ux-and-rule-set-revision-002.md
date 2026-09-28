# DCR-026 revision002 — Finding ownership / acceptance scope transfer

日期：2026-09-08。产品拆分与本次 ownership transfer：USER:lifei 当前明确决定，已授权。
范围：仅 TASK-018 acceptance ownership；后续 TASK-019/020 详细产品 Design 仍未冻结。独立 Change/Design Review 由新的 review Evidence 记录，不由本文自批。

## 历史与修复责任

原 `human-visual-findings-038.json`、原 Human Visual REWORK、target-018、原 DCR-026 与原 Impact038 保持不可变历史。本 revision 对 acceptance ownership 和实施顺序优先于原 DCR/Impact 中的 current/next 叙述。

| 原 Finding | 保留状态 | 修复与后续验收 Owner | TASK-018 当前 Completion blocker |
| --- | --- | --- | --- |
| TASK018-HV038-001 | OPEN | TASK-019 | 否 |
| TASK018-HV038-002 | OPEN | TASK-019 | 否 |
| TASK018-HV038-003 | OPEN | TASK-019 | 否 |
| TASK018-HV038-004 | OPEN | TASK-019 | 否 |
| TASK018-HV038-005 | OPEN | TASK-020 | 否 |

这是用户授权的 Scope / Acceptance ownership transfer，不是修复、关闭、风险豁免或 target-018 APPROVED。原 Evidence 不新增 transfer 状态、不修改 OPEN；新 acceptance binding 给出 owner 与不再阻断依据。TASK-019/020 必须在物化时继承各自 OPEN Findings 和验收责任，不能只当建议或默默丢弃。

## TASK-018 最终人工视觉范围

仅评审已批准且实际交付的 presentation delta：shared foundation 已实施修复；Subscription activation pending / active transition；单一 SubscriptionCard emphasis；ToastHost positioning / stacking；closing Subscription preload error presentation；其他本 Task 已批准的直接 presentation delta，以已有 Candidate、implementation manifest、保护区及证据链为界。“其他”不是开放修改或扩大验收许可。

明确排除：Proxies 整页最终 UX/composition、Routing 整页最终 UX/composition、drag reorder、structural auto apply、Proxy/Routing Toast lifetime 新策略、sing-box Rule Set。这些页面可以作为共享 Toast 的 integration context 出现在截图里，但不会因 TASK-018 人工批准而获得页面视觉/功能批准。历史 6000ms discrepancy 保留，不在本轮修改或宣称解决。

## 影响、证据与 Completion Predicate

影响仅为 TASK-018 当前 acceptance binding、当前人工 review request、Gate 恢复摘要、后续 stub 的 ownership 追踪。无生产源码、测试、Contract0.3、Requirement Source、Orchestrator 协议或 runtime/data 变更。TASK-019/020 产品设计仍先 clarification / impact / independent design / approval 再实现。

重新评审请求必须绑定相同 target-018 和新 acceptance binding；旧 request037 不代表此范围的人工结果，旧 REWORK038 永不改写。现有 Compliance037/Independent Review037 只能用于其实际 Toast 范围；shared foundation、Subscription transition/emphasis 等历史证据须按范围与保护链检查适用性，不能只因源码 hash 一致就宣称全范围 Compliance PASS。

TASK-018 Completion = 当前有界交付范围的适用验证与 Compliance + current 独立 Review + 对新范围的真实 Human Visual Approval + 最终独立 Task Acceptance / Delivery Completion 的其他必需条件。若证据缺失/陈旧则只补证据或记录未满足项，本轮无源码权限。五个转移 Findings 不在该合取条件内；future Design 也不是 TASK-018 收束前置。新的范围内缺陷仍按 TASK-018 正常处理。

独立 Change/Design Review 通过后可生成新 Human Visual Review request，当前 Delivery 可由历史 FAILED 转为 PENDING（新范围等待评审/证据），但不得置 PASSED 或 DONE。只有全部 Completion Predicate 满足后才可 DONE。范围转移批准不是 Human Visual Approval；本轮不生产任何 APPROVED 人工文件。

## 单 Task 顺序

Findings 的产品责任已独立转移；实际实施顺序仍遵守 workflow：TASK-018 有界收束并 DONE 后，才 materialize / 启动 TASK-019；TASK-019 完成后按依赖启动 TASK-020。

保留 TASK-019 dependencies=[TASK-018]、TASK-020 dependencies=[TASK-019]。不修改 Orchestrator 允许 FAILED Task 启动后续 Task，不自动切换 focus。本条取代原 Impact038“排期与 TASK-018 收束独立”的歧义。
