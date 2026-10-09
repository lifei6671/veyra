# OpenBox Rust / GPUI 执行任务总表

基线日期：2026-10-03。技术范围以[实现方案](../openbox-rust-gpui-implementation-plan.md)为准；本目录负责显式技术依赖 DAG、任务状态和验收记录；P0–P7/Windows 保留为里程碑分组。后续开发默认遵守[长期开发总规范](DEVELOPMENT_WORKFLOW.md)。

**看当前进度先读 [SESSION.md](SESSION.md)，领取任务再读下表对应任务卡。** 本表是任务状态的唯一记录位置；任务卡中的复选框记录验收项，SESSION 只汇总当前进度和下一步。

## 1 当前基线与范围

- 新路线共 **61 个 macOS 任务、7 个 Windows 后续任务**。当前完成 **22项**：P0-01至07、P1全部8项、P2-01/P2-02A/P2-02B/P2-03/P2-04、P4-02/P5-06；P2为 **5/10**、macOS为 **22/61**。P2-04 Host FINAL ACCEPTANCE PASS，P2-04-PENDING-PERSISTENCE-001及前轮P1/P2 Finding CLOSED，Runtime/Platform及Runtime公共契约owner/本卡预约释放；见[Host最终收口](P2-local-proxy.md#p2-04-final-host-closeout)。P4-02 DONE（[本轮记录](P4-02-acceptance.md)）；P5-06 DONE（[最终验收](P5-06-acceptance.md)）；P2-06 DOING；READY=P0-08/P2-05/P3-01/P4-03/P5-04，未领取/未启动；其余任务状态/依赖与历史PASS/FAIL/REWORK/NOT_RUN保持。
- 已有 React UI 和旧 Rust 模块是迁移输入，不直接算 GPUI 新路线完成。P0-02 的完成仅指本次明确要求的范围/规则调整，该历史文档动作不计功能实现；现 P1-01 已完成核心抽取，P1-02 已完成类型/原子快照与版本。
- 当前已建立根 Cargo workspace、单一 Cargo.lock 与 `crates/veyra-core/`；旧入口接共享核心。实际构建/纯测试见 P1-01 记录，不表示原型或真实设备验收完成。
- 旧 SDLC 状态与 UI 门禁已按用户要求退役；不恢复缺失文件，不补办历史 DCR/UI Contract。当前进度以本目录为准。
- 首期优先 macOS；Windows 任务 DEFERRED，单独排期估算；Linux 只保留适配边界，开始前另做范围确认与任务拆解。
- 所有工期暂为“待重估”。OBG-P0-09 按实测逐任务回填开发、集成、验证与返工，无签名包与管理员安装成本单列。任务数是范围计数，不是按权重计算的工程完成百分比。

<a id="priority-20261008"></a>
## 开发优先级与延期清单（2026-10-08）

唯一政策见 [WORKFLOW：先功能真数据 → 后系统能力 → 最终 E2E](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)，WorkRun `work-5654-1791456602781005-182`；此表只排调度优先级，不变更状态或显式依赖。

| 顺序 | 当前入口 / 后续安排 | 验收边界 |
| --- | --- | --- |
| 前期 | P4-02 已DONE；P5-06 分享 UI 已DONE并释放本卡owner，P4-03 新增READY（协调Runtime owner）；P2-05/P3-01/P5-04 按依赖/写范围并行或适时领取；P2-08/P3–P5/P7-01 后续仍按原 DAG | 真实用户业务动作、Service/Store CRUD 与重建回读、loopback/自有资源局部定向集成及 150% 浅色 UI；只 Mock/静态配置/禁用按钮不能 DONE |
| 后期平台实施 | P2-06 checkpoint 后仍 DOING，GUI/Native/Helper 多轮恢复后补；P2-07 SystemProxy、P6-01 TUN/P6-02 恢复与依赖平台任务后做 | 保留接管/恢复与安全归属；前期系统开关禁用，真实能力完成后才开放；不新增 P3–P5 前置 |
| 最终组合验收 | P2-09/P3-08/P4-07/P5-07/P6-05 仅补未执行 Native 与跨模块完整组合；P7-04 统一 GUI E2E，P7-05 最终包，P7-03 保留包构建/升级职责 | 系统能力、正式安装/真实内核网络、Sleep/Wake 切网真机组合、完整跨页/主题/缩放、6 主页/9 分类/77 API/19 场景整套复验；不承接前期全部局部功能测试 |

P0-08 保留 READY 作路线准备，Windows 7 卡仍 DEFERRED。P2-06 checkpoint `f2457e6a4e2fe0ac3c9186bc7d17323a70c4c0a8` 已存在，但不证明 DONE 或恢复全通过；不继续以复杂崩溃恢复阻塞其它已 READY 功能。仅 OS 授权/正式内核网络的具体验证与完整组合项可记 NOT_RUN/原因/承接卡；选择测速、实际观测/连接操作、主备/Compiler/应用接口、DNS 本地测试/资源下载/应用、分享 HTTP/token 失效/退出清理及真实保存/加载均在各业务卡实现并局部定向验证，不得一并延期。68 卡与 DONE22/DOING1/ACCEPTANCE0/READY5/TODO33/DEFERRED7，已按实际显式依赖重算。

## 2 如何执行与更新

1. 读 [长期开发总规范](DEVELOPMENT_WORKFLOW.md)、SESSION 的 Active Tasks/Ready Queue/Blocked，再读对应任务卡与相关方案小节；核对显式依赖及实际交付身份。
2. 所有显式依赖 DONE 后 TODO 转 READY。领取时核对写范围、公共契约 owner 和真实资源预约，登记负责人及下一动作；允许多个 READY 且写范围不冲突的任务并行 DOING。
3. 公共领域模型、共享 DTO、Cargo workspace、Runtime 公共契约各设单一 owner；冲突写入先串行整合，独立消费任务继续推进。UI fixture 可先做，但不满足依赖或真实集成 DONE 条件。
4. 按任务卡实现、验证与必要审查；仍缺真实 UI/平台结果使用 ACCEPTANCE。验收项完成才 DONE，不强制逐 Task 人工签字。
5. 更新本表状态/负责人/证据，同步 SESSION 计数、Active Tasks/Ready Queue/Blocked 与里程碑；完成后重算下游 READY，中止也登记精确下一动作。
6. 外部真实阻塞才记 BLOCKED，列缺失条件和解除动作；普通依赖未完成保持 TODO，READY 写冲突等待在队列注明。
7. 范围/契约/依赖变更同步技术方案必要小节、任务卡、本表依赖/覆盖/计数及 SESSION；只做最小后缀拆分，保持历史证据与未运行结果。

阶段是里程碑分组和组合验收，不作统一串行 Gate，不继承上一阶段出口。每卡依赖是 DAG 的定义来源，下表是同一依赖的摘要。依赖已满足可跨阶段推进；动作授权仍以当前用户/已批准 Task 为准，发布不因 READY 自动获授权。

### 状态约定

| 状态 | 含义 | 是否计完成 |
| --- | --- | --- |
| TODO | 普通显式依赖尚未完成 | 否 |
| READY | 所有显式依赖 DONE，待无冲突领取 | 否 |
| DOING | 正在实现或验证，有明确下一动作 | 否 |
| REVIEW | 按风险需要的审查或修复反馈尚未完成 | 否 |
| ACCEPTANCE | 仍待真实平台/界面结果，或用户明确要求的人工验收 | 否 |
| BLOCKED | 外部条件或必须作出的决定阻塞，已记录解除条件 | 否 |
| DONE | 当前任务的实际验收项与结果记录全部完成 | 是 |
| DEFERRED | 已明确排除在当前排期之外 | 不进入当前平台分母 |

### 通用完成条件

- 本任务范围的交付物存在；关键正常、失败、并发/恢复行为按卡片验证。
- 验证记录有命令或操作步骤、预期/实际结果、测试数量、构建/环境身份和证据路径；过滤后零测试、mock、截图不能冒充真实集成。
- 未完成 UI 任务按[调度政策](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)先验证真实用户功能、局部受控定向集成、真实保存/重载及局部 150% 浅色操作、加载/空/错误/忙碌；其它主题/缩放和完整交互延期至指定组合卡，记录 NOT_RUN 与承接归属，不要求旧 Compliance 或人工签字工件。
- 受影响的旧行为做定向回归，必要审查发现的有效问题已处理；不重新认证 sing-box 协议算法。
- 没有未解决的本任务缺陷；明确推迟的后续事项有单独记录，不能把移出范围写成通过。
- 任务表和 SESSION 已同步。非 UI 任务的视觉项可注明不适用，不能免除其实际业务/平台验收。

### 验证命令与证据

文档变更只检查链接、编号、依赖、覆盖和 `git diff --check`。实施阶段遵循 `AGENTS.md`：受影响 React 使用已有 `pnpm lint`、`pnpm test`、`pnpm build`；旧 Rust 使用定向测试及规定的 clippy/fmt。新 workspace 的实际命令/超时/旧资源覆盖/测试数见 [P1-01 交付记录](P1-core-and-shell.md#p1-01-delivery)。

执行前确认是否启动真实 child/外部资源，并使用合理外层超时。真实权限/网络/导入/重置验收在已授权的隔离环境完成；已有证据足够时不重复扩大测试。

每次交付在对应阶段文件末尾追加一段记录，主表“证据”列链接到该段或现有交付记录。无需预先创建 68 份证据模板；证据字段与更新规则见长期规范。

```text
Task：OBG-Px-xx
负责人 / 日期：
源码 commit 与未提交 diff / 构建身份：
交付物路径：
环境：OS / 架构 / GPUI 与内核版本（按任务适用）
验收项：逐项 PASS / FAIL / NOT_RUN / N/A，N/A 附批准依据
验证：命令或操作 → 预期 → 实际；测试数量、日志/截图路径
必要审查：执行人或记录、结论、问题处理；自查不得称为独立审查
实际验收：操作/结果/日期；仅在用户明确要求时另记人工确认
剩余问题与解除条件：
下一动作：具体文件或入口与待执行步骤
```

## 3 阶段与里程碑

| 里程碑 | 可交付结果 | 组合验收范围 | 出口任务 |
| --- | --- | --- | --- |
| P0 | 已核实路线、能力差异、当前范围与新版估算 | 基线与原型结论；未依赖原型的任务可先行 | OBG-P0-09 |
| P1 | 无 Tauri 核心库、配置保存、桌面壳和托盘 | 核心/状态/壳层/两类偏好/桌面资源 | OBG-P1-07 |
| P2 | 首个可用 macOS 代理版本 | 订阅→Base 目录→Compiler→Runtime/网络/选择/系统代理 | OBG-P2-09 |
| P3 | 主页面基础能力、可靠观测与历史下钻 | 基础代理/已加载规则/连接/日志/概览，不含完整 DNS 诊断 | OBG-P3-08 |
| P4 | 高级配置及 Unified OutboundCatalog | 订阅高级→动态组→failover→chain→统一目录→routing/client routing | OBG-P4-07 |
| P5 | DNS、五种共享入站、订阅分享与完整 Rules | DNS 后闭合 Rules；Shared Inbound/Share 按自身依赖并行 | OBG-P5-07 |
| P6 | macOS TUN/生命周期/更新集成 | helper/系统代理/恢复/编译/网络/DNS 的平台组合 | OBG-P6-05 |
| P7 | 数据维护、GitHub 包与 macOS 候选版验收 | 稳定 schema、生命周期清理及首期全部证据 | OBG-P7-05 |
| W0–W3 | Windows 单独验证的版本 | 排期确认、选定 core 基线和 Windows 实际结果 | OBG-W3-02 |

组合验收出口只约束显式依赖它的任务，不作为下一编号阶段的统一入口。任务表按里程碑分组；同组行顺序不代表串行。P4-01/02、P5-05/06 等在 READY 且无写/资源冲突时可并行。

## 4 任务状态总表

“估算”在P0出口回填人日及假设；当前macOS61项：**DONE22、ACCEPTANCE0、READY5、DOING1、REVIEW0、TODO33、BLOCKED0**；Windows7项DEFERRED，共68项。P2阶段5/10，macOS22/61。P2-03既有Host收口及Finding CLOSED保持；P2-04经Host独立FINAL ACCEPTANCE PASS，ACCEPTANCE→DONE，P2-04-PENDING-PERSISTENCE-001及前轮P1/P2 Finding均CLOSED，Runtime/Platform及Runtime公共契约owner/本卡预约释放，见[当前收口](P2-local-proxy.md#p2-04-final-host-closeout)。绑定Core source aggregate `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8` 与Desktop harness身份；Host已核验Native/限定故障注入及原14条exit0，并独立新复跑Core371/Desktop85（11 ignored由显式Native父test覆盖）。自然HTTP/OS故障、SIGKILL、无控制线程竞态仍NOT_RUN；历史PASS/FAIL/REWORK保留，不扩大验收。P2-06 READY→DOING，Codex 保留 Runtime/Platform 与 Runtime DTO owner；仅本卡生产执行器/IPC/固定安装部分代码与OS隔离测试交付，已有双向关闭cache交接/半提交重试，已有停止前预检及远程选择fence/IPC/CAS/manifest，新增卸载Archive/新安装隔离闭环；已增加固定来源与OS退出观察后有限正常会话授权；cold-start/预先Stopped来源、未知slot/旧owner人工恢复及同UID信任边界仍OPEN；新增统一产品身份与helper Quit清理，[第八轮记录](P2-local-proxy.md#p2-06-round8)。P4-02 DONE；P5-06 DONE（独立HTTP架构、启动收敛、29唯一定向测试、真实GUI及150%浅色逐态验收完成；保留用户接受技术差异）；READY为P0-08/P2-05/P3-01/P4-03/P5-04，未领取/启动；原显式依赖不变，其它任务状态不变。原P1-04/P2-02/P4-05父项不重复计数。

### P0 基线与可行性（9 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P0-01 UI/接口基线](P0-feasibility.md#obg-p0-01) | 无 | DONE | Codex /root | — | [最终验收](P0-01-baseline.md#current-acceptance) |
| [OBG-P0-02 范围与规则](P0-feasibility.md#obg-p0-02) | 无 | DONE | Codex | 文档调整，不计实现工期 | [清理记录](P0-feasibility.md#cleanup-record) |
| [OBG-P0-03 GPUI/输入/托盘原型](P0-feasibility.md#obg-p0-03) | P0-01 | DONE | Codex Desktop | — | [人工验收收口](P0-feasibility.md#p0-03-manual-acceptance) |
| [OBG-P0-04 内核/控制器/缓存原型](P0-feasibility.md#obg-p0-04) | P0-01 | DONE | Codex /root | — | [真实内核验收](P0-feasibility.md#p0-04-delivery) |
| [OBG-P0-05 本地 helper 原型](P0-feasibility.md#obg-p0-05) | P0-03、P0-04 | DONE | —（owner 已释放） | — | [原型历史](evidence/p0-05/README.md)；[Desktop 批准/启动失败/清理](evidence/p0-05/desktop-privileged-20261006-113208/README.md)；[新路径取消/复制修复/待批准](evidence/p0-05/desktop-pathfix-20261006-120343/README.md)；[旧现场清理/read_frame Status PASS/child readiness timeout](evidence/p0-05/desktop-readframe-fix-20261006-132202/README.md) ；[readiness child_exited / FD3 open权限拒绝 / cleanup PASS](evidence/p0-05/desktop-readiness-diagnostics-20261006-134101/README.md) ；[FD3真实probe历史与当时E扩展缺项](evidence/p0-05/desktop-fd3-pipe-20261006-135940/README.md)；[真实 GUI owner 最终验收](evidence/p0-05/gui-owner-final-20261006-142752/README.md) |
| [OBG-P0-06 自身出站原型](P0-feasibility.md#obg-p0-06) | P0-04、P0-05 | DONE | —（owner 已释放） | — | [出站/范围调查](P0-feasibility.md#p0-06-delivery) |
| [OBG-P0-07 观测/DNS/诊断能力](P0-feasibility.md#obg-p0-07) | P0-01、P0-04 | DONE | Codex /root | — | [能力核实](P0-feasibility.md#p0-07-delivery) |
| [OBG-P0-08 更新与分发路线](P0-feasibility.md#obg-p0-08) | P0-03、P0-05 | READY | — | — | — |
| [OBG-P0-09 出口与重新估算](P0-feasibility.md#obg-p0-09) | P0-01、P0-02、P0-03、P0-04、P0-05、P0-06、P0-07、P0-08 | TODO | — | — | — |

### P1 核心库与桌面壳（8 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P1-01 核心抽取](P1-core-and-shell.md#obg-p1-01) | P0-01、P0-02 | DONE | Codex /root | — | [抽取验收](P1-core-and-shell.md#p1-01-delivery) |
| [OBG-P1-02 类型/持久化/版本](P1-core-and-shell.md#obg-p1-02) | P1-01 | DONE | Codex /root | — | [类型/快照验收](P1-core-and-shell.md#p1-02-delivery) |
| [OBG-P1-03 GPUI 壳与状态桥](P1-core-and-shell.md#obg-p1-03) | P1-02、P0-03 | DONE | Codex Desktop | — | [壳层/桥与最终复核](evidence/p1-03/README.md) |
| [OBG-P1-04A 视觉桌面偏好/主题/组件](P1-core-and-shell.md#obg-p1-04a) | P1-02、P1-03 | DONE | Codex Desktop · GPUI / Core-Config | — | [视觉偏好/组件交付](evidence/p1-04a/README.md) |
| [OBG-P1-04B 跨页面行为偏好](P1-core-and-shell.md#obg-p1-04b) | P1-02、P1-03 | DONE | Codex Desktop · Core/Config + GPUI | — | [行为偏好/消费契约](evidence/p1-04b/README.md) |
| [OBG-P1-05 目录/单实例/文件](P1-core-and-shell.md#obg-p1-05) | P1-02、P1-03 | DONE | Codex Desktop · Runtime/Platform + GPUI | — | [P1-05 平台验收](evidence/p1-05/README.md) |
| [OBG-P1-06 托盘与关闭](P1-core-and-shell.md#obg-p1-06) | P1-03、P1-05 | DONE | Codex Desktop | — | [正式托盘/窗口与人工验收收口](evidence/p1-06/README.md) |
| [OBG-P1-07 桌面壳验收](P1-core-and-shell.md#obg-p1-07) | P1-01、P1-02、P1-03、P1-04A、P1-04B、P1-05、P1-06 | DONE | —（owner 已释放） | — | [最终验收](P1-core-and-shell.md#p1-07-final-closeout) |

### P2 本机代理闭环（10 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P2-01 订阅基础闭环](P2-local-proxy.md#obg-p2-01) | P1-02、P1-03、P1-04A、P0-06 | DONE | —（owner 已释放） | — | [build38-final最终收口](P2-local-proxy.md#p2-01-final-closeout)；完整页面Host Visual PASS，Finding CLOSED；历史FAIL/REWORK/build与未来业务禁用边界保留 |
| [OBG-P2-02A Base OutboundCatalog/OutboundId](P2-local-proxy.md#obg-p2-02a) | P2-01 | DONE | —（Core/Config owner 已释放） | — | [Host独立审查/范围收口](P2-local-proxy.md#p2-02a-closeout) |
| [OBG-P2-02B 最小 Compiler](P2-local-proxy.md#obg-p2-02b) | P2-02A、P0-04 | DONE | —（Core/Config owner 已释放） | — | [Host独立review/332 Core/五候选语义一致/Finding CLOSED](P2-local-proxy.md#p2-02b-host-closeout) |
| [OBG-P2-03 Runtime/服务状态](P2-local-proxy.md#obg-p2-03) | P2-02B、P1-03 | DONE | —（Runtime/Platform + GPUI owner已释放） | Host功能/视觉符合；59280c31… | [最终Host收口](P2-local-proxy.md#p2-03-final-closeout) |
| [OBG-P2-04 选择/缓存/成功记录](P2-local-proxy.md#obg-p2-04) | P2-03 | DONE | —（Runtime/Platform及Runtime公共契约owner已释放） | — | [Host FINAL ACCEPTANCE / Findings CLOSED](P2-local-proxy.md#p2-04-final-host-closeout) |
| [OBG-P2-05 自身出站客户端](P2-local-proxy.md#obg-p2-05) | P2-03、P0-06 | READY | — | — | — |
| [OBG-P2-06 helper/IPC](P2-local-proxy.md#obg-p2-06) | P2-03、P2-04、P0-05 | DOING | Codex · Runtime/Platform | — | [第十八轮工程](P2-local-proxy.md#p2-06-round19) |
| [OBG-P2-07 系统代理与恢复](P2-local-proxy.md#obg-p2-07) | P2-06、P2-05 | TODO | — | — | — |
| [OBG-P2-08 选择与节点测速](P2-local-proxy.md#obg-p2-08) | P2-02A、P2-04、P2-05、P1-04B | TODO | — | — | — |
| [OBG-P2-09 首个可用版本验收](P2-local-proxy.md#obg-p2-09) | P1-07、P2-01、P2-02A、P2-02B、P2-03、P2-04、P2-05、P2-06、P2-07、P2-08 | TODO | — | — | — |

### P3 观测与主页面（8 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P3-01 观测与事件](P3-observability.md#obg-p3-01) | P2-03、P0-07 | READY | — | — | — |
| [OBG-P3-02 连接](P3-observability.md#obg-p3-02) | P3-01、P2-02A、P2-05、P2-08、P1-04B | TODO | — | — | — |
| [OBG-P3-03 日志](P3-observability.md#obg-p3-03) | P3-01、P1-05 | TODO | — | — | — |
| [OBG-P3-04 统计存储](P3-observability.md#obg-p3-04) | P3-01 | TODO | — | — | — |
| [OBG-P3-05 概览/站点测速](P3-observability.md#obg-p3-05) | P3-01、P3-04、P1-04B、P2-05 | TODO | — | — | — |
| [OBG-P3-06 历史/下钻/容量设置](P3-observability.md#obg-p3-06) | P3-04、P3-05 | TODO | — | — | — |
| [OBG-P3-07 代理基础视图/已加载规则](P3-observability.md#obg-p3-07) | P3-01、P2-02A、P2-08、P1-04B | TODO | — | — | — |
| [OBG-P3-08 主页面基础能力验收](P3-observability.md#obg-p3-08) | P3-01、P3-02、P3-03、P3-04、P3-05、P3-06、P3-07 | TODO | — | — | — |

### P4 完整配置能力（9 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P4-01 订阅高级项](P4-configuration.md#obg-p4-01) | P2-01、P2-05 | TODO | — | — | — |
| [OBG-P4-02 静态/动态组](P4-configuration.md#obg-p4-02) | P2-01、P2-02A、P2-02B、P1-03、P1-04A | DONE | —（本卡owner/预约已释放） | — | [验收修正与交付](P4-02-acceptance.md) |
| [OBG-P4-03 failover](P4-configuration.md#obg-p4-03) | P4-02、P2-04 | READY | — | — | 依赖均DONE；未领取，协调P2-06 Runtime/DTO owner |
| [OBG-P4-04 规则资源](P4-configuration.md#obg-p4-04) | P2-02B、P2-04、P2-05、P1-04A | TODO | — | — | — |
| [OBG-P4-06 链式代理](P4-configuration.md#obg-p4-06) | P2-02A、P2-02B、P2-05、P2-08 | TODO | — | — | — |
| [OBG-P4-05A 目标分流/统一目录](P4-configuration.md#obg-p4-05a) | P4-03、P4-04、P4-06 | TODO | — | — | — |
| [OBG-P4-05B 终端分流](P4-configuration.md#obg-p4-05b) | P4-05A、P3-01 | TODO | — | — | — |
| [OBG-P4-05C Routing/Terminal 基础诊断](P4-configuration.md#obg-p4-05c) | P4-05A、P4-05B、P1-04B、P2-05、P3-07 | TODO | — | — | — |
| [OBG-P4-07 配置阶段验收](P4-configuration.md#obg-p4-07) | P4-01、P4-02、P4-03、P4-04、P4-06、P4-05A、P4-05B、P4-05C | TODO | — | — | — |

### P5 DNS 与共享（7 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P5-01 DNS 上游/重写](P5-dns-and-sharing.md#obg-p5-01) | P2-02B、P2-03、P2-05、P0-07、P1-04A | TODO | — | — | — |
| [OBG-P5-02 DNS 过滤](P5-dns-and-sharing.md#obg-p5-02) | P5-01、P2-05 | TODO | — | — | — |
| [OBG-P5-03 DNS 观测/热更边界](P5-dns-and-sharing.md#obg-p5-03) | P5-02、P3-01、P4-05C | TODO | — | — | — |
| [OBG-P5-04 五种共享入站](P5-dns-and-sharing.md#obg-p5-04) | P2-02B、P2-03 | READY | — | — | — |
| [OBG-P5-05 共享 UI/URI](P5-dns-and-sharing.md#obg-p5-05) | P5-04、P1-04A、P1-05 | TODO | — | — | — |
| [OBG-P5-06 订阅分享](P5-dns-and-sharing.md#obg-p5-06) | P2-01、P1-05、P1-04A | DONE | — | — | [独立HTTP/启动收敛/逐态验收与独立复核](P5-06-acceptance.md) |
| [OBG-P5-07 DNS/共享验收](P5-dns-and-sharing.md#obg-p5-07) | P5-01、P5-02、P5-03、P5-04、P5-05、P5-06、P4-07 | TODO | — | — | — |

### P6 macOS TUN 与生命周期（5 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P6-01 TUN/模式交接](P6-macos-lifecycle.md#obg-p6-01) | P2-06、P2-07、P2-04、P2-02B、P2-05、P5-01 | TODO | — | — | — |
| [OBG-P6-02 睡眠/切网/恢复](P6-macos-lifecycle.md#obg-p6-02) | P6-01 | TODO | — | — | — |
| [OBG-P6-03 登录启动/退出](P6-macos-lifecycle.md#obg-p6-03) | P6-02、P1-06、P4-01、P5-06 | TODO | — | — | — |
| [OBG-P6-04 更新集成](P6-macos-lifecycle.md#obg-p6-04) | P6-02、P0-08、P2-05 | TODO | — | — | — |
| [OBG-P6-05 macOS 平台验收](P6-macos-lifecycle.md#obg-p6-05) | P6-01、P6-02、P6-03、P6-04 | TODO | — | — | — |

### P7 数据与发布收尾（5 项）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-P7-01 备份与导入](P7-data-and-release.md#obg-p7-01) | P1-02、P1-04A、P1-04B、P1-05、P3-06、P4-01、P4-03、P4-05B、P4-06、P5-02、P5-04、P5-06 | TODO | — | — | — |
| [OBG-P7-02 诊断/重置](P7-data-and-release.md#obg-p7-02) | P7-01、P6-02、P6-03、P2-07、P5-06 | TODO | — | — | — |
| [OBG-P7-03 GitHub 包/手动升级](P7-data-and-release.md#obg-p7-03) | P7-01、P7-02、P6-05 | TODO | — | — | — |
| [OBG-P7-04 全范围验收](P7-data-and-release.md#obg-p7-04) | P1-07、P2-09、P3-08、P4-07、P5-07、P6-05、P7-01、P7-02 | TODO | — | — | — |
| [OBG-P7-05 macOS 候选版验收](P7-data-and-release.md#obg-p7-05) | P7-03、P7-04、P0-09 | TODO | — | — | — |

### Windows 后续（7 项，不计 macOS 进度）

| 任务 | 显式依赖 | 状态 | 负责人 | 估算 | 证据 |
| --- | --- | --- | --- | --- | --- |
| [OBG-W0-01 构建/GPUI 原型](WINDOWS.md#obg-w0-01) | P1-01、P1-03；Windows 排期确认 | DEFERRED | — | — | — |
| [OBG-W0-02 服务/TUN 原型](WINDOWS.md#obg-w0-02) | W0-01、P2-03 | DEFERRED | — | — | — |
| [OBG-W1-01 普通代理/WinINet](WINDOWS.md#obg-w1-01) | W0-01、P2-01、P2-02B、P2-04、P2-05、P2-08 | DEFERRED | — | — | — |
| [OBG-W2-01 服务/Named Pipe](WINDOWS.md#obg-w2-01) | W0-02、W1-01、P2-06 | DEFERRED | — | — | — |
| [OBG-W2-02 TUN/owner 交接](WINDOWS.md#obg-w2-02) | W2-01、P6-01 | DEFERRED | — | — | — |
| [OBG-W3-01 安装/升级/卸载](WINDOWS.md#obg-w3-01) | W2-02、P7-01、P6-04 | DEFERRED | — | — | — |
| [OBG-W3-02 Windows 独立验收](WINDOWS.md#obg-w3-02) | W3-01、P3-08、P4-07、P5-07、P7-02 | DEFERRED | — | — | — |

## 5 功能覆盖与验收归属

本节是遗漏检查表，不是第二份状态表。状态仍以任务总表为准，详细 API 契约仍以技术方案 §18 为准。

### 六主页面与九设置分类

| 范围 | 主要交付任务 |
| --- | --- |
| 概览 | P3-05、P3-06 |
| 代理 | P2-08、P3-07 基础视图；P4-03、P4-05C/07 完整目录/组/主备/Chain 集成 |
| 连接 | P3-02 基础连接；P4-05C/07 Unified OutboundCatalog 映射 |
| 日志 | P3-03 |
| 规则 | P3-07 基础已加载列表；P4-05A/B/C 分流与基础诊断；P5-03/07 DNS 后最终集成 |
| 设置壳与导航 | P1-03；下列九分类分别验收 |
| 面板设置 | P1-04A 含 sidebar/layout 纯视觉偏好；P1-04B 含 proxy columns/hide unavailable 跨页面行为偏好 |
| 订阅管理 | P2-01、P4-01、P5-06 |
| 出站节点 | P4-02、P4-03 |
| 目标分流 | P4-04、P4-05A；P4-05C 基础诊断 |
| 终端分流 | P3-01 Observation/client discovery → P4-05B/C；P3-02 仅为连接页面消费者，不可用项有批准差异 |
| 链式代理 | P4-06 |
| 共享网络 | P5-04、P5-05 |
| DNS 设置 | P5-01、P5-02、P5-03 |
| 后端设置 | P2-02B/03/07、P3-06、P6-01/03/04、P7-01/02；基础字段、保留周期、TUN、更新与数据工具逐项验收 |

### 77 个 client 方法

下表编号与方案 §18 一致。每个编号只分配一个主要闭合任务；相关 UI/共享底层通过任务依赖一起交付。这里的 `Px-yy` 均指 `OBG-Px-yy`。

| API 编号 | 主要任务 | 交付或范围处理 |
| --- | --- | --- |
| 01–04 | P1-03 | 本地启动/OS 用户边界替代 Web 鉴权，桌面范围由 P0-02 记录 |
| 05–06 | P1-04B | 偏好快照/patch 公共行为；视觉字段由 P1-04A 验收 |
| 07–09 | P1-04A | 背景资源与视觉偏好 |
| 10–11、15–16 | P2-03 | Runtime 状态、动作、版本与操作结果 |
| 12–13 | P3-07 | 当前无业务页面调用，只按内部实际需要使用，不强制新增 UI |
| 14、17–19 | P6-04 | 更新检查/下载/进度/取消，P7 验证分发 |
| 20、24 | P7-02 | 重置与脱敏诊断 |
| 21、40–42 | P3-06 | 历史查询、下钻和容量；P3-04 提供存储 |
| 22–23 | P7-01 | 备份与导入 |
| 25–29、48 | P2-08 | 选择、测速与延迟历史；基础展示在 P3-07，完整 Group/Failover/Chain 在 P4-05C/07 集成 |
| 30 | P2-05 | GeoIp 来源、字段与出站策略 |
| 31 | P3-07 | 已加载规则 |
| 32–35 | P4-05C | Routing/Terminal 基础诊断；DNS 完整阶段与 Rules 最终集成在 P5-03/07 闭合 |
| 36–37 | P3-02 | 当前实例的连接关闭 |
| 38–39 | P3-05 | 站点测速与历史 |
| 43–45 | P4-02 | 分组与默认组；P4-03 完成主备策略 |
| 46–47、56–59 | P2-01 | 订阅基础闭环，P4-01 完成高级项 |
| 49 | P4-06 | 链路测速 |
| 50 | P5-04 | 按协议检查共享端口 |
| 51–55 | P5-06 | 订阅分享生命周期 |
| 60–61 | P1-02 | 类型化 profile/patch；编译与各字段行为随对应任务验收 |
| 62 | P4-05A | 默认路由与目标分流，统一出口目录 |
| 63–65 | P4-04 | 规则资源预览/导入/刷新 |
| 66 | P4-05B | 基于 P3-01 Observation 的 clients.observed()/已知终端投影与 Client Routing；不依赖 Connections 页面，平台不可用状态 |
| 67–69 | P0-02 | 当前无业务调用，不新增设备分发系统；记录不迁移范围 |
| 70–72、74 | P5-02 | 过滤状态/保存/应用/预览 |
| 73、77 | P5-03 | DNS 记录与真实缓存清理，按已批准能力交付 |
| 75–76 | P5-01 | 上游测试与重写默认值 |

四类流通道由 P3-01 接入；connections/logs/memory/traffic 分别通过 P3-02、P3-03、P3-05 和 P3-04/06 验收。文件、剪贴板、窗口/托盘在 P1-05/06；分享二维码在 P5-05/06。

### 方案 §16.2 的最低场景

| 场景编号 | 主要证据任务 |
| --- | --- |
| 1 首次启动 | P1-03、P2-09 |
| 2 订阅 | P2-01、P4-01 |
| 3 出口选择 | P2-04、P2-08 |
| 4 分流 | P4-05A/B/C；DNS 完整诊断在 P5-03/07 补证 |
| 5 DNS | P5-01/02/03 |
| 6 连接 | P3-02 |
| 7 日志 | P3-03 |
| 8 共享 | P5-04/05/06 |
| 9 备份 | P7-01 |
| 10 服务与回退 | P2-03/04、P6-02 |
| 11 平台授权/窗口/退出 | P1-06、P2-07、P6-01/03 |
| 12 恢复出厂 | P7-02 |
| 13 failover | P4-03 |
| 14 运行缓存 | P2-04、P6-01 |
| 15 应用出站 | P0-06、P2-05、P6-01 |
| 16 长读与日志 | P3-03、P3-06 |
| 17 macOS 基础集成 | P1-06、P2-03/07 |
| 18 网络归属 | P2-07、P6-02 |
| 19 权限 IPC | P2-06、W2-01 |

OBG-P7-04 汇总上述覆盖的实际证据；Windows 补充场景由 OBG-W3-02 独立闭合。
