# DCR-026: Proxies / Routing UX 与 Rule Set 分离设计

Status: PROPOSED — product direction recorded; detailed design / approval pending

Source: USER:lifei，2026-09-08，target-018 Human Visual Review = REWORK。

用户明确要求将 Proxies/Routing 桌面 UX 与 Rule Set 新领域能力移出 TASK-018；已授权 Findings 记录与 bounded impact analysis，未授权生产源码修改。

冲突：现有页面构成、显式二次 Apply、持久 success Notice 与新交互方向不一致；当前 Domain/State/Compiler 尚无 Rule Set。Contract0.3 §6.7、§7.2 及相关冻结页面/反馈设计需在后续任务中正式核对修订。旧 Candidate/Approval/失败和通过 Evidence 保留，不以本 DCR 推导历史目标 APPROVED。

建议：TASK-019 承担 compact Proxies/Routing、drag reorder、auto apply 与 Toast UX；TASK-020 承担完整 Rule Set 模型、迁移、IPC、Compiler、CRUD、引用、lifecycle 与 UI/tests。详细影响、明确需求、待决语义、验收建议与依赖见 `.sdlc/design/TASK-019-020-impact-038.md`。

当前决定：TASK-018 保持 VERIFYING，人工视觉结论 REWORK，Delivery FAILED；暂停生产源码修补。Requirement clarification、独立 Design、Contract 修订及所需批准完成前不进入后续实现。本 DCR 不是冻结 Design，不改变后端 reorder 语义、不回滚已保存 Desired、不修改 Requirement Source identity，不批准新依赖/下载后台服务。
