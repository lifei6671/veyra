# Technical Design

## Decision Discovery 先于方案

从 Anchor、Requirement Source 和项目约束中只识别会改变真实工程边界的技术决策。先区分
`greenfield` 与 `established`：已有工程沿用可验证的现有架构；绿地工程在第一行实现前用一次
Foundation 确认固定会产生路径依赖的技术基线。

分类：

- **L0 局部实现决策**：私有命名、局部 helper、fixture 与测试结构。Implementation 自行决定。
- **L1 可逆工程决策**：如 Logging、Validation 或小型 helper。优先使用仓库约定；
  无法推断时记录最小 Assumption 后继续。
- **Foundation Decision**：Framework/stdlib 组合、项目布局与 Module Boundary、Data Access、
  API/Transport、Migration、测试基础设施和核心依赖策略。即使理论可逆，只要当前规划窗口完成后
  的迁移成本会被大量代码放大，就必须在 Greenfield Foundation 中一次集中确认。
- **L2 Material Decision**：Architecture、Module Boundary、Runtime、持久数据、公共 Protocol、
  权限/安全、并发、Deployment、不可逆成本。必须按适用范围设计并请求所需授权。

Foundation 或 Material runtime/operational dependency 必须在修改 manifest/lockfile/checksum 或运行
安装命令前获得用户明确确认。Foundation 的一次确认可以覆盖其中精确列出的 package、version/range、
用途和影响；标准库、仓库已有且已批准依赖、低影响 dev/test dependency 与可逆开发工具按项目规则或
L1 Assumption 处理，不重复询问。安全、权限、生产、外部成本依赖始终按 Material 处理。

## Greenfield Foundation

`foundation` 模式不需要 Current Task，只读取 Anchor、完整 Requirement Source、Repository
Snapshot 与 Project Constraints，形成唯一 `.sdlc/design/foundation.md`：

```text
# Foundation
## Stack
## Architecture
## Project Layout
## Data / API
## Key Decisions
## Test Strategy
## Verification Commands
```

集中展示真正需要用户决定的路径依赖选项和拟新增 Material dependency，其余建议作为同一技术基线
一次确认。首次生成 Foundation 时，Orchestrator 创建 `gates.technical_design` 的 `PENDING` record，
其 identity 绑定 foundation 当前内容。standard/critical（以及触及 Material concern 的 prototype）必须先
由 `technical-design-review` 独立审查：`reviewed_by: null` 表示等待 Review；Review Evidence 与 identity
当前、`approved_by: null` 才只等待 Human approval；两者匹配后才冻结。Review `REWORK` 时仅修复该
Foundation concern 并重新审查。人工改写 Foundation 会清空旧 review/approval evidence、保持 `PENDING`
并重新进入 Review，不得再次调用 Producer 生成同一 Foundation。Foundation 未确认前不拆 `READY` Task、
不安装未确认的 Material dependency、不开始实现。`established` 项目不得重新询问已有框架或创建 Foundation。

## 模块化 Design

单一 concern 可直接创建一个目标 Design 文件；只有多文件设计确实需要导航时才按需创建
`.sdlc/design/INDEX.md`。不要创建一个所有阶段都要读取的巨型文档。按适用性覆盖：

- architecture / module boundaries / dependency rules；
- data / API / protocol / compatibility；
- transaction / concurrency / error handling；
- security / observability；
- deployment / migration / rollback；
- experiment / data collection；
- testing strategy。

只有实际适用的领域需要展开；未覆盖项不创建文件，也不写 `not_applicable` 占位。

## Technical Review

Reviewer 第一次只读，验证：

- Requirement 到 Design 的可追踪性；
- Material Decision 是否完整、一致、可执行；
- Architecture 与当前仓库/约束是否兼容；
- 只检查受影响 concern 的 Failure、Security、Migration、Rollback、Observability、Test 或
  Experiment 风险；未受影响 concern 的缺失不构成 Finding；
- 是否存在未授权的 Material Scope/Contract 变化。

返回 P0–P3 Finding 和 `PASS/PASS_WITH_CONDITIONS/REWORK`。Producer 修复后必须重新冻结目标并复审受影响范围。

## Design Freeze

Technical Design Gate 通过并完成所需 Human Gate 后：

1. 更新 Design version 和 `status: FROZEN`；
2. 记录实际适用的 ADR；只有修改已冻结 Design 或 accepted ADR 时才创建 DCR；
3. 让 Task 绑定该 Design version/引用；
4. 更新 State 和 HANDOFF。

Freeze 后 Implementation 的目标是实现已批准设计，不是边写边设计。冲突走 DCR。

## Context 契约

`foundation` 默认加载 Anchor、完整 Requirement Source、Repository Snapshot 和 Constraints；
`task-boundary/remediation` 默认加载 Current Task 和存在时的相关 Existing Architecture。按问题
加载相关 Design、ADR/DCR；多文件设计存在时再读取 Design Index。不读取
全部历史 Task、QA 或 Evidence。
