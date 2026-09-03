# `security` Lane

**Activate**

- Authentication、Authorization、Permission、Identity、Secret 或 Cryptography 发生变化；
- Untrusted Input 跨越 Network、Filesystem、Database、Serialization 或 Execution Boundary；
- Change Map 给出可触达的 Trust Boundary 或敏感日志信号。

**Inspect**

- Validation、身份绑定、授权检查和权限作用域；
- Injection、Path Traversal、Unsafe Deserialization、SSRF、Command Execution 和 Secret Leakage；
- 密钥/随机数使用、敏感日志，以及失败路径是否破坏既有 Trust Boundary。

**Do NOT flag**

- 没有可信攻击路径或攻击者能力假设的理论攻击；
- 现有主防御充分时额外增加校验、过滤或隔离层的建议；
- 与当前变更无 material interaction 的既有安全问题或替换安全库的偏好。

`TIER_3_DEEP` 的潜在 P0/P1 Candidate 若只是尚未完成 Trigger Path、Trust Boundary、数据流或
Compensating Control 的验证，例外地遵循 [Needs Validation](universal-not-flag.md)，不能在这里直接
抑制。

**Escalation signals**

- 可利用的认证/授权绕过、注入、Secret 暴露或任意代码/路径访问；
- 当前变更削弱现有 Trust Boundary，或把可信数据变成外部可控数据；
- 安全结论依赖未读取的 Contract、调用方或部署边界，需要扩展相关 Coverage。

## Security Validation Pass

仅在 `TIER_3_DEEP + security + potential P0/P1` 且 Candidate 命中 Needs Validation 时执行。它是一次
有界反证，不是完整 Threat Model 或新增审查阶段：

1. 确认 Entry Point、攻击者能力与 Reachability；
2. 沿当前变更相关的 Data / Control Flow 检查 Trust Boundary；
3. 查明授权、验证、隔离或其他 Compensating Controls；
4. 标记 `CONFIRMED` 或 `DISPROVEN`，再交 Judge。

`NEEDS_VALIDATION` 是 Candidate 在此 Pass 开始时的临时状态：它不进入最终 Finding、canonical
State 或 `review_result`。只有 `CONFIRMED` 的 Candidate 才能通过 Finding Admission；`DISPROVEN`
的 Candidate 不输出 Finding。

宿主支持时，可由未参与 Candidate 生成的只读 Reviewer 执行反证；否则由同一独立 Reviewer 的
skeptical validation pass 完成。`DISPROVEN` 不输出 Finding。Validation 未完成时，Review 不得完成，
也不得通过 `remaining_risks + PASS` 绕过：

- Target、Required Context、Ownership 或 Coverage 不足：保留 `NEEDS_VALIDATION`，Lane
  `validation: BLOCKED`、`result: REWORK`，整体 `outcome: BLOCKED`、`review_result: REWORK`；
- 工具、权限、依赖或环境不可用：保留 `NEEDS_VALIDATION`，Lane `validation: UNAVAILABLE`、
  `result: UNAVAILABLE`，整体 `outcome: UNAVAILABLE`、`review_result: null`。

只有 `CONFIRMED` 或 `DISPROVEN` 才令该 Lane 的必需 Validation 为 `COMPLETE`。`CONFIRMED` 的
P0/P1 Candidate 必须进入 Finding Admission 并最终映射为 `REWORK`；`DISPROVEN` 后也只有其余
Coverage 全部完整时才可 PASS。
