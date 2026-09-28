# TASK-019 Requirement clarification 002

Status: CANDIDATE for detailed Design approval; the ten product decisions below are explicitly USER accepted. No implementation authority.

来源：本轮 USER:lifei 十项决定；TASK019 DRAFT；DCR026 revision002 / binding039。基线 docs/veyra.md 与有效 UI Contract0.3 保持不变；详细行为通过本候选追踪并在 Human Design Approval 后正式绑定。

| ID | 已决定的需求与可观察 Acceptance |
| --- | --- |
| R01 | structural 保存成功后，仅当前 Runtime Ready 且有效 active subscription 时自动 Apply；运行态检查必须在可信 runtime 调度边界落实。 |
| R02 | stopped 只保存 Desired，不启动内核，提示“已保存，将在下次启动时应用”。 |
| R03 | 无有效 active subscription 只保存，不 Apply，提示“已保存，当前无可应用订阅”。有效指 authoritative state 中该 ID 存在且可解析为当前受管运行意图，不凭 UI 非空字符串判定。 |
| R04 | recoveryRequired 只保存，persistent recovery feedback；不在后台自动恢复/启动。 |
| R05 | 连续 structural mutations latest authoritative Desired / generation wins；旧 operation completion 不覆盖新 generation；CRUD 不排队、不自动重放。 |
| R06 | Retry Apply 先读取当前 authoritative Desired，仅 Apply；不能重放 create/update/delete/reorder。 |
| R07 | Routing 用现有 React Pointer/Keyboard Events，零新增 DnD runtime dependency；依赖只有独立 Review 证明必要后另申请。 |
| R08 | Proxies `[出口组] [全部节点]` 互斥视图，默认出口组，消除页面底部重复 All Nodes 大区块。 |
| R09 | ordinary success Toast 精确3000ms，无关闭按钮；需要操作的 warning/error 可 persistent。 |
| R10 | apply failure persistent Toast 必有“重试应用”，只调用 retry-current-Desired。 |

结构操作清单：create/update/delete custom Pool（update含enable/disable）、setDefaultTarget、create/update/delete Route（update含enable）、reorderRoutes。Manual selector 和 Subscription activation 排除；它们保留现有语义。No-op reorder不提交；其余后端判断无变化的结果不得虚构新 generation 或启动 Apply。

状态优先级：已保存且 recoveryRequired 优先显示恢复反馈；其次无有效 active subscription；其次 stopped；其余非Ready（启动/停止/应用中）只保存并呈现真实 pending/drift；只有 Ready 可产生新的自动 Apply。应用中与快速后续保存的收敛细节见 auto-apply design，不能借“latest wins”回滚新 Desired 或启动 stopped runtime。

自动检查失败不改变“保存已成功”：配置/check/stop/start失败均报告已保存与应用失败；unknown 只说应用结果暂不可确认，不能说成功或保存失败。保存IPC响应未知时刷新 authoritative snapshot，不盲目重放CRUD，不能从 generation变化推定特定create已经成功。

恢复按钮语义：apply failure 一律“重试应用”；点击时重新查询，并按当前 runtime/active/generation 检查是否可Apply。若stopped、无active、recoveryRequired，明确说明下一用户动作，不能自动启动。另可有“查看运行状态”导航至现有Overview；它不是新的修复/后台任务。停止态普通保存属于成功3000ms；需处理的无订阅/恢复/Apply错误为persistent带操作反馈。

用户反馈文案在 Routing 使用“规则已保存并应用”；Pool/default出口使用“出口设置已保存并应用”。applying只显示“已保存，正在应用”，不能提前成功。正常Header不常驻应用；failure/unknown/unresolved drift才有恢复入口。

视觉 Acceptance：compact sections + node grid主内容；轻量selected，Light/Dark一致，长名称可读、键盘focus可见、无重复区块；Routing drag handle平时隐藏但可键盘访问，拖动有lift/insertion，取消零提交、drop全量稳定ID一次提交且后端priority语义不变。

验证追踪：R01–06/R10由Rust可信调度+前端generation/operation返回序列测试和最小受控Native Apply验证；R07由pointer/keyboard实际操作+准确ID/取消冲突测试；R08由双视图/long-name/空错忙状态与Native主题DPI；R09由fake clock3000ms（含waiting到期）/无关闭按钮/recovery action与多Toast positioning回归。仅本轮文档检查已运行，产品验证NOT_RUN。

Prerequisite与阶段决定：Candidate前只读检查确认旧Golden0.2原FAIL、无HumanApproval、7/15manifest不匹配；保留旧文件和原引用为历史provenance，不提升为current。USER随后明确授权 S（共享Toast修订+完整Subscriptions Golden）→ A（Proxies Golden）→ B（Routing）。新候选正式声明Subscriptions页面和内部producer S；不再把旧FAIL记录用作当前外部prerequisite。S从无上游Golden的页面开始；A必须消费S真实current认证，B必须消费A真实current认证。该责任/阶段调整有单独真实USER授权，不是删引用绕过readiness。独立Review及精确Human Design/Contract批准、Task正式绑定与阶段checkpoint仍必需，本轮不实施。具体固定引用、证据失效与新版本采用流程见stage-plan003。

四项HV038001–004仍OPEN归TASK019，005归TASK020；本候选不修复或关闭。RuleSet / Domain / State schema migration / Compiler新能力 / TASK020物化全部排除。
