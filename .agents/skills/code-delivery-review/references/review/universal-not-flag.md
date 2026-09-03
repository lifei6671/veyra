# Universal NOT-Flag Contract

所有 Lane 在 Finding 准入前先应用本规则。规则分为 Hard Suppress 与 Needs Validation，避免把
TIER_3 高风险安全候选因早期证据尚不完整而过早丢弃。

## Hard Suppress

直接抑制以下 Candidate：

1. 与当前 Delivery Unit 没有 material interaction 的旧问题；
2. 主防御已经充分时的 defense-in-depth 建议；
3. “可以更优雅”或“可以考虑重构”的偏好型建议；
4. “建议换成某个库或框架”；
5. 不违反项目规则的 Naming、Formatting 或 Style 偏好；
6. 为假想未来需求提出的 Abstraction、Fallback 或 Compatibility Layer；
7. 没有 Acceptance 或风险依据的“建议增加测试”；
8. 单纯因为某领域存在而要求补充对应 Artifact；
9. 与本次变更无关的 `AGENTS.md`、文档或配置历史债务。

疑似重复或同根因 Candidate 不在 Lane 侧 Hard Suppress；全部交给 Judge 执行 Dedup /
Root-cause Merge，因为不同 Lane 可能提供互补的触发路径、影响或反证。

## Needs Validation

以下 Candidate 默认不准入 Finding；在满足 `TIER_3_DEEP + security` 且可能为 P0/P1 时，必须先走
Security Validation Pass，不得直接以“无 Trigger Path”“Trust Boundary 未确认”“Compensating Control
未知”或“Data/Control Flow 未读全”抑制：

- 疑似严重、但 Trigger Path 尚未闭合的 AuthN/AuthZ、注入、Secret 或任意访问风险；
- 入口可达性、攻击者能力、Trust Boundary 或补偿性控制尚未确认的安全候选；
- 尚未完成数据流/控制流读取、但当前变更可能削弱安全边界的候选。

其他 Lane 中没有 Trigger Path、无证据、无可观察后果或无法高置信定位的候选，仍直接抑制。
Validation 后确认的 Candidate 回到 Finding Admission；被反证的 Candidate 标记 `DISPROVEN`，不输出
Finding。多个 Lane 命中同一根因时交给 Judge 合并，不得以数量抬高 Severity。
