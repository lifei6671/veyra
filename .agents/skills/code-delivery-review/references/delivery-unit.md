# Delivery Unit 与 Freshness

## 边界定义

Delivery Unit 是一个 Task 从开始实现到形成一次完整交付结果的连续范围。它可以跨越多轮
会话、暂停、用户澄清、子代理和上下文压缩，但不能因为恢复会话而重置起始基线。

Delivery-owned 内容包括：

- 当前 Task 产生的 task-time commits；
- Agent 为当前 Task 产生的 staged、unstaged、untracked、deleted 内容；
- 用户或协作者明确要求纳入当前交付的贡献，且保留真实作者与来源；
- 当前 Task 改变的 generated、supporting、verification-created 工件。

预存用户改动、无关并发提交和历史缺陷不属于 Delivery Unit。可读取它们判断交互影响，
但不得归因、修改或让它们阻断当前 Task。无法在 hunk 层可靠区分且会影响完整覆盖时，
返回 `outcome: BLOCKED`，不要扩大到整个 Branch。

## 起始与恢复

Implementation 在 Task 开始时记录可获得的：

```text
starting HEAD or non-Git snapshot
git status --short --untracked-files=all
host edit record or equivalent
task intent and authority boundary
```

Skill 若在实现开始后才介入，必须从会话/工具历史、宿主编辑记录和 Git provenance 重建
原始基线，不能把介入时工作树当成起点。跨会话最小 handoff 至少包含：Task、起始身份、
inventory/provenance、checkpoint、规则/Contract、验证结果和未解决 Finding。未解决 Finding 少时
从当前 Review Result/Evidence 派生复制以下最小身份；数量超过 Context 预算时，把同一结构持久化
为 compact review Evidence，HANDOFF 只保存引用：

```yaml
unresolved_findings:
  - finding_id: FINDING-003
    fingerprint: auth/service.go:UpdateUser:ownership-check
    severity: P1
    status: UNFIXED
    target_identity: <current reviewed target>
    evidence_ref: <review evidence reference>
```

恢复时不得只保留自然语言摘要；`finding_id` 与 `fingerprint` 必须能稳定对账增量复审，位置、
Evidence、Impact 和 Severity 仍按当前 Target 刷新。HANDOFF 是恢复摘要，不是 Finding 的独立
事实源。写入前必须确认 `evidence_ref` 在会话结束后仍可解析；否则先持久化 compact review
Evidence，再生成 HANDOFF。

本项目不为此创建第二套状态根。Orchestrator 只在 `.sdlc/memory/HANDOFF.md` 和按需创建的
`.sdlc/evidence/TASK-xxx/` 中保存经校验的摘要或 Evidence；
`.sdlc/state.yaml` 仍是唯一流程事实源。

## Inventory

每个相关路径或 hunk 记录：

```text
path | change kind | staged state | provenance | in-scope reason | review partition
```

Review 前至少对账适用的：

```text
git status --short --untracked-files=all
git diff --stat
git diff --name-status
git diff
git diff --cached
known task-time commit overlay
```

普通 Diff 不包含 untracked 文件，必须显式读取每个 delivery-owned untracked 文件。Rename、
delete、mode/symlink、binary/LFS、submodule、generated 和 lockfile 按真实语义进入 inventory。

## Coverage Manifest

将每个 in-scope 路径或责任以及每个适用 Lane 映射到审查结果：

```text
partition | paths/responsibility | applicable lanes | target identity | reviewer mode | result | freshness
```

同时记录 justified exclusions、ambiguous ownership、跨分区依赖和 uncovered paths。小型/中型
变更可以只有一个全范围分区；大型/多模块变更必须所有分区都有结果并增加 Integration Review。
Coverage 还必须显式列出 `applicable_lanes`、每个 Lane 的终端 `result` / `freshness`，以及安全等
必需 Validation 的状态。任一 material uncovered path、漏跑的适用 Lane 或未完成的必需 Validation
都不能得到 `review_result: PASS`。

Checkpoint 只有在其目标、规则、Contract、验证前提和 material interaction 未变化时才保持
有效；它从不替代最终 Integration 与 Freshness 检查。

## Target Identity 与 Review Context

Target identity 识别被审实现状态，至少覆盖适用的 HEAD/task-time commits、delivery-owned
Diff、untracked 内容、测试、Contract、配置和生成工件。可使用宿主 snapshot、内容哈希或
等价身份；只有文件名列表不充分。

Review Context 单独记录 Task intent、Acceptance、适用规则/Contract、Design/ADR、change map、
coverage 和真实 Verification Evidence。交付前比较两个维度：

- `FRESH`：目标未变化，且没有会改变结论的重要 Review Context 前提发生变化；
- `STALE`：目标变化，或规则、Contract、Acceptance、验证结果、跨分区交互发生实质变化。

`STALE` 不能通过。重新执行受影响验证，刷新受影响分区和 Integration 边界；无法恢复新鲜
完整覆盖时返回 `outcome: BLOCKED` 或能力/环境导致的 `outcome: UNAVAILABLE`。
