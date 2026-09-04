# TASK-009 已证实的相邻修复范围

状态：FIXED，用户明确回复“允许”；授权记录为 CHANGE-002。下述两项已扩入 TASK-009 并完成修复，
独立复审 code-review-checkpoint-002.yaml 确认两项 P1 已关闭；不更改既有用户确认的失败语义。

## 1. 私有目录准备失败后的所有权

- 位置：`src-tauri/src/platform/windows/private_runtime.rs` 的 `PrivateRuntime::create` 与
  `cleanup_after_prepare_failure`，以及已在 Scope 内的 Windows Port 调用方。
- 原触发：目录创建成功，文件/ACL准备失败，随后的删除也失败。原实现只返回错误，局部
  `PrivateRuntime` 对象被丢弃，调用方拿不到待清理对象。
- 后果：无法按 DCR-004 保留资源归属供手动 Stop 清理，可能错误报告没有待清理资源。
- 已实施：创建错误使用封闭枚举，清理失败变体携带 `PrivateRuntime`；只有清理未完成时
  将对象交给 Port 持有。后续 Start 封闭失败，手动 Stop 使用该对象清理。同步适配内测。
- 保持：不改变 ACL/身份/reparse 校验，不扫目录，不重试，不恢复旧配置，不增加产品接口。
- 验证：注入“准备失败 + 清理失败”，证明目录所有权仍可达；Stop 成功后才清除 recovery。

## 2. 固定核心的正常空日志响应

- 位置：`src-tauri/src/singbox/clash_api.rs` 的固定 Logs 握手路径与单帧读取。
- 初始证据：`real-observation-diagnostic`（2026-09-04）真实 REST、traffic 读取成功，
  disabled Logs 读取报 `Unavailable`。初读源码推定 HTTP 204，实际核心返回 HTTP 200、
  Content-Length: 0、空 body 且没有 Transfer-Encoding；详见 empty-log-wire-contract.md。
  design-scope-review-002.yaml 已独立确认精确空响应契约。
- 后果：正常空日志被当作采样失败；当前失败停止语义会因此停止健康核心，不能交付。
- 已实施：仅固定 Logs 路径的鉴权请求收到 HTTP 204 或上述精确空 HTTP 200 时返回空摘要；Traffic、401、
  非预期状态、错误鉴权、超时仍失败，不泛化所有 WebSocket 握手错误。
- 验证：真实固定核心空日志通过；13 个 Mock 边界场景通过，全库 165 项测试通过且无过滤。

## 已批准并实施的流量契约修正

固定核心 `/traffic` 源码输出每秒区间计数，原 Bridge 将其当累计值再次差分；
当前零流量用例不能证明正流量速率正确。DCR-005 已核对固定核心一手源码并提出最小适配：
窗口值映射速率，已有 REST 摘要提供累计量；保持现有 DTO 和固定采样限制。
`.sdlc/design/DCR-005-traffic-observation.md` 的独立设计审阅 PASS 记录于
`traffic-design-review.yaml`。用户明确“确认”批准 CHANGE-003，算法已在 checkpoint004 实施，
41 项定向验证 PASS，记录于 verification-004.yaml。真实阳性流量入口/读数仍需验证，
当前前提缺口见 positive-traffic-prerequisites.md；不能凭 Mock 或零流量测试标记 SF-002 通过。

## 审批边界与下一步

实施 Skill 要求发现必须越出冻结 Task Scope 的修复时交回 Change Control。
上面两个文件现已纳入 TASK-009 allow 清单及内测范围；范围不涉及依赖、系统网络或公开 IPC。

WireGuard/DNS 验证仍有独立前提，详见 `network-prerequisites.md`。新 peer 依赖、
主动 UDP 入口和可观测系统 DNS 拓扑尚未授权，不借本次两文件修复批准一并引入。
