# Legacy V0.1 Migration

只在恢复旧状态时读取本文件。完成归一化后停止使用旧字段，并只运行 V0.2 Router。

## 识别

以下任一情况进入迁移，不把旧状态直接交给主 Router：

- `schema_version == 1`；
- 缺少 V0.2 `workflow_version`、`anchor` 或 `tasks.yaml`，同时存在明确旧字段；
- `schema_version: 2` 但缺少 `project.engineering_context` 或其值仍为 `null`；
- `schema_version: 2` 仍带 `implementation_ready`、`focus.story`、预建 `workstreams` 或整组
  `NOT_EVALUATED` Gate。

## 权威与安全

恢复优先级：

```text
canonical legacy artifacts + Git/Evidence
  > legacy state
  > HANDOFF
  > current.md
  > DAILY/LESSONS
  > chat recollection
```

迁移不推断 Human Approval，不把旧 PASS 无条件沿用。目标身份或 Evidence 不匹配时将相关
Gate 记为 `STALE`；事实冲突且无法可靠归一化时保留原文件、机读保持 `STALE` 或 `BLOCKED`，并仅在
用户表面报告 `UNABLE_TO_VERIFY`。

## 归一化

1. 若缺少 `project.engineering_context` 或其值仍为 `null`，先按当前 Scope 的仓库事实推导并持久化：必须同时
   有可沿用的 stack、布局/模块约定、build/test 约定和架构边界才记为 `established`；仅有 README、manifest
   或需求文档，以及成熟 monorepo 中的新 service，均记为 `greenfield`。若是
   `greenfield` 且尚未开始实现，恢复后先走 Foundation；`established` 直接沿用仓库约定。
   只有仓库事实不足且两种判断会实质改变路径时才集中询问，不因旧字段缺失本身阻塞。
2. 从既有 Requirement、Task 和已接受决策提取 Anchor；不要为缺失的 Epic、Story、Design
   或其他旧目录补建空文件。
3. 从旧 focus/workstream 与实际 Task 状态找到一个当前 Task，写入 `focus.task`。
4. 按事实选择 V0.2 Phase：
   - Anchor 缺失：`PROJECT_INIT`；
   - Anchor 已有、当前 Task 未建立：`ANCHORED` 或 `PLAN`；
   - Task 为 `READY` / `IN_PROGRESS`：`EXECUTING`；
   - 候选等待 Delivery/QA/Release/Observation 结论：`REVIEW`；
   - Finding 要求修改目标：`REMEDIATION`；
   - 当前 Task 所有适用验收有新鲜 Evidence：`DONE`。
5. 只迁移实际进入且有 Evidence、Finding 或 Approval 的 Gate；丢弃空
   `NOT_EVALUATED` Gate。
6. 不迁移 `implementation_ready`。从 Task、Blocker 和适用 Gate 推导是否可实现。
7. `blocked.active: false` 归一化为 `blocked: null`；真实 Blocker 改写为 V0.2 Blocker 对象。
8. Story/Epic/Design/Experiment 只保留现有引用，不成为 V0.2 主路由前提。

## Legacy Memory

`current.md` 只可用于找回缺失引用，不能覆盖 canonical artifact。`DAILY.md` 和 `LESSONS.md`
只可提取仍影响当前执行、且已有事实支持的内容到 Task、ADR 或 HANDOFF。默认不自动删除旧文件；
归一化后停止读写。清理旧文件属于独立、显式迁移动作。

## 完成条件

新状态满足 Sparse Canonical State，Requirement Source/Task 引用可解析，适用 Gate 身份与
Evidence 已对账，HANDOFF 已从新事实重建。此后只读取
[workflow-protocol.md](workflow-protocol.md) 执行 V0.2 Router。
