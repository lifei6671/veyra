# Incremental Re-review

仅在 Producer 修复后重新审查时加载；首次审查不读取本参考。Reviewer 首轮只读。Producer 获得授权后
执行最小修复；每轮是：Finding -> repair -> affected verification -> new target identity -> re-review。
默认最多三轮。修复需要扩大 Scope 或改变公共/持久化/运行/运维契约、重要运行依赖、权限安全或架构时，
立即交回 Change Control。

Planner 使用 Previous Findings、修复 Diff、新 target identity 和受影响 Verification 调度增量复审。
窄修复且 provenance 清晰时，只重开相关 Finding 对应的 Lane、受影响分区和 Interaction；广泛修复、
Contract 变化、Scope drift、先前 Coverage 不完整或 material interaction 变化时，必须刷新相关分区与
Integration Review，不得为了节省 Token 沿用 stale 结论。

## Finding Identity and Reconciliation

增量复审才扩展初审的 `status: NEW`，使用：

```yaml
status: FIXED | UNFIXED | ACKNOWLEDGED | DISPUTED | WORSENED | NEW
```

- `finding_id` 在首次准入时生成；同根因后续复审保持不变；
- `fingerprint` 表达稳定的语义身份，不包含行号或 `target_identity`；
- `target_identity`、location、Evidence、Impact 和 Severity 必须依据当前 Target 刷新；
- `FIXED`：当前 Target 已消除触发路径；不再重复输出为开放 Finding，但保留 resolved Evidence；
- `UNFIXED`：问题仍可触发；沿用原 `finding_id` / `fingerprint`，刷新位置、证据和影响；
- `ACKNOWLEDGED`：有权限的 Owner 已明确接受并关闭非阻断 P2/P3，且 Review Evidence 同时记录
  `resolution: ACCEPTED_RISK`、`accepted_by`、`accepted_at` 与 `evidence_ref`；缺任一字段时，
  仅靠确认知悉而关闭无效。P0/P1 不能接受风险关闭，必须保持 `UNFIXED` / `REWORK`，直至修复或被独立
  反证；只有已关闭项的风险 materially worsened 时才重新输出；
- `DISPUTED`：Producer 提供了反证或异议；Judge 必须验证理由，证明确属误报则关闭，问题仍
  成立则以同一 `finding_id` 重新断言为 `UNFIXED`；
- `WORSENED`：同一根因在当前 Target 的可触发范围或可观察影响扩大；沿用原身份并按当前证据
  重新定级；
- `NEW`：当前 Target 首次出现、且不能映射到既有 `fingerprint` 的问题；生成新 `finding_id`。

复审不是完整重审；它不免除对修复 Diff、受影响调用关系、验证和新 material interaction 的检查。
若修复扩大了 Review Scope，应为新增范围运行适用 Review，而不是强行归入旧 Finding。
