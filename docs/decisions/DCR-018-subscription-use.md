---
id: DCR-018
status: DESIGN_PENDING
affected_task: TASK-011
product_approval_ref: .sdlc/evidence/TASK-011/use-decision-006.json
---

# 订阅“使用”与参考客户端对齐

用户对“点击后立即切换到选中的订阅”明确回答“是”，确认替代 proposal005 的 U1 多订阅启停。
Requirement §1.3 记录已接受语义。其它订阅仍可管理，但其节点不自动进入本次选择生成的运行配置。

与冻结 TASK-011 设计的冲突是其禁止卡片激活及新增 Apply 命令；与 DCR-004 的关系是新增显式
订阅应用入口，保留旧零参数 Start 的 AlreadyRunning 行为和所有已批准的分阶段失败约束。
本变更不允许用 Stop→Start 的前端拼接绕过候选 check 之前保留旧实例的要求。

影响：订阅选择持久化/迁移、RuntimeIntent 输入投影、应用 worker 串行切换、活动订阅与实际运行
状态的区分、IPC/main 权限、卡片及菜单反馈、定向事务与最小原生集成。既有订阅解析及 UA 修复
证据仍证明其原范围，不能证明切换能力。TASK011 技术 Gate 对此增量为待审，交付/验收保持未完成。

技术候选：`.sdlc/design/TASK-011-subscription-expansion-006.md`；它还落实用户此前要求的新建选项
和菜单。Root 记录产品决定，独立作者形成方案，独立 Reviewer 审查；新增依赖/精确能力授权未获批前
不安装包或修改 capability。保留 baseline001 和全部预存改动，无提交、推送或发布授权。
