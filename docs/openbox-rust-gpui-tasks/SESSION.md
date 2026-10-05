# OpenBox Rust / GPUI 当前进度

最后更新：2026-10-05。

**当前结果：OBG-P0-01、P0-02、P0-03、P0-04、P0-07、P1-01、P1-02、P1-03、P1-04A、P1-04B、P1-05、P1-06、P1-07 DONE；P0-05 ACCEPTANCE；P1-07 owner 已释放，READY 为空。** P1-06 Human Visual / Tray interaction 由当前 Host 会话用户真实操作补足，截图未复制入仓库。P0-07 完成标准四流/区间 traffic/短连接/DNS 三能力/本地诊断；DNS records/hot rewrite unsupported，flush supported。P0-04 存在对账前流量窗口。P0-03 人工确认和历史限制保留；helper/TUN 运行未验证。阶段用于里程碑分组及组合验收，READY 只由显式依赖决定。

- [长期开发总规范](DEVELOPMENT_WORKFLOW.md)：后续 Veyra Rust/GPUI 默认遵守；四泳道、并行与单一 owner。
- [技术方案](../openbox-rust-gpui-implementation-plan.md)：架构、行为、接口与平台边界。
- [任务总表](IMPLEMENTATION_PHASES.md)：唯一状态表、显式依赖与覆盖归属。

**全局规范新增（2026-10-04）**：[UI 视觉迁移与组件复用强制规范](../../AGENTS.md#ui-视觉迁移与组件复用强制规范)及[长期视觉/组件流程](DEVELOPMENT_WORKFLOW.md#31-长期-ui-视觉迁移与组件抽象流程)已固化。除 OS 绘制且 React 不拥有的 macOS 原生区域外，React/OpenBox 源码与 CSS 是唯一视觉事实来源；固定值精确迁移、95% 最低目标、逐页面/核心状态验收、Tokens/共享组件层级与 Legacy 视觉参考保留均为强制要求。规范新增不代表现有实现已满足或通过视觉验收，历史证据保持原结果。

## 1 进度总览

| 里程碑 | DONE / 任务数 | 当前结果 | 组合验收 |
| --- | --- | --- | --- |
| P0 基线与可行性 | 5 / 9 | P0-01/03/04/07 验收通过；固定内核/缓存/观测能力核实完成，历史限制保留 | P0-09 未开始 |
| P1 核心库与桌面壳 | 8 / 8 | 壳层/设置/基础组件视觉 PASS_WITH_TECHNICAL_DIFFERENCES；人工三轮、重启恢复、最终 Tray Quit 与清理 PASS；未实现业务页不计完成 | P1-07 DONE，组合 PASS |
| P2 本机代理闭环 | 0 / 10 | 未开始，按各卡依赖推进 | P2-09 未开始 |
| P3 观测与主页面 | 0 / 8 | 未开始，基础能力不计完整 DNS/分流 | P3-08 未开始 |
| P4 完整配置能力 | 0 / 9 | 未开始，Chain 在 Routing 前交付 | P4-07 未开始 |
| P5 DNS 与共享 | 0 / 7 | 未开始，Rules 在 DNS 后最终闭合 | P5-07 未开始 |
| P6 macOS TUN 与生命周期 | 0 / 5 | 未开始，按各卡依赖推进 | P6-05 未开始 |
| P7 数据与发布收尾 | 0 / 5 | 未开始，schema/清理按显式依赖等待 | P7-05 未开始 |
| Windows W0–W3 | 0 / 7 | 后续排期 DEFERRED | W3-02 未开始 |

macOS：**13 / 61 完成**；READY 0、TODO 47、DOING 0、REVIEW 0、ACCEPTANCE 1、BLOCKED 0。Windows 7 项 DEFERRED 单列，共 68 项。数量不等于工期权重或代码完成百分比；P1-04/P2-02/P4-05 父项被后缀子项替代，不重复计数。

## 2 Active Tasks

| Task | owner / 泳道 | 写范围 / 公共契约 | 实际资源 / 下一动作 |
| --- | --- | --- | --- |
| P0-05 · ACCEPTANCE | Codex /root · Runtime/Platform | `crates/veyra-helper/` 显式 `p0-05-prototype`、固定编排、P0-05 evidence/任务文档；本轮根 Cargo workspace/lock 已整合，core/公共 DTO 不改 | 系统授权 -60008，管理员入口未运行；原型 /Library/launchd/service 从未创建，普通用户 child/temp 已清。待可呈现标准管理员 UI 的本机环境补真实特权验收；不能计 DONE |

P0-05 历史起始 HEAD `3111a57`。历史 P1-03 起始 HEAD `f460e767957569e9f4a035132e26cc216277e98f`，工作树干净；当时只领取 P1-03；历史 P1-04A 起始 HEAD a71b664，干净；现已由 Host 提交 c184325，成为历史 P1-04B 基线；P1-04B 已由 Host 提交 f9adda4，本轮 P1-05 基线干净、完成后未提交，不启动下游。独立 label/path/socket/service/child/tmp 的现场预约已释放；补验时重新检查身份、版本与无残留。

P1-07 已 DONE 并从 Active Tasks 移除，GPUI owner 与本卡资源预约释放；隔离 root/bundle 和 evidence 保留供 Host 复核，无自有 Veyra 验收进程残留。

## 3 Ready Queue

本队列由主表状态与任务卡依赖计算，不按阶段或编号统一放行。写范围冲突的 READY 留在队列并注明等待；未完成普通依赖的任务仍为 TODO。

| Task | 依赖满足依据 | 候选写范围 / 并行条件 | 下一具体动作 |
| --- | --- | --- | --- |



当前 68 卡：DONE13 / ACCEPTANCE1 / DOING0 / READY0 / TODO47 / DEFERRED7。按[完整显式依赖 DAG](evidence/p1-07/final-20261005/dag.json)重算无环、Ready Queue 为空。P1-07 DONE；消费它的 P2-09/P7-04 仍缺其他显式依赖，保持 TODO；P0-05 ACCEPTANCE、P0-06/P0-08 TODO、Windows DEFERRED 不变，未启动 P2。P1-07 实现与最终 Evidence 已由 Host 提交为 `25eb020`；其后仅有本交接页元数据更新，工作树干净，未 push。


## 4 Blocked

当前没有实际 BLOCKED。普通依赖未完成和未启动原型不伪标为外部阻塞；实际遇到问题后填写下表。

| Task / owner | 实际阻塞原因 | 解除条件 / 下一动作 | 受影响任务 |
| --- | --- | --- | --- |
| 无 | — | — | — |

后续限制按卡追踪：P1-04A 已验证 nested Select/Escape/focus 与用户 IME；MiSans desktop 资源未锁定、card backdrop 仍明确差异，macOS 15 实机及历史受控 blur/截图缺口保留于 P0-03；P0-04/07 已核实内核/缓存/观测/DNS 边界，出站 P0-06 待办；管理员 helper P0-05 原型已实现，但标准授权 -60008 待补特权验收；首次打开 P0-08、新版估算 P0-09。GitHub 无发布签名/公证和旧门禁退役已由 P0-02 记录，无补办流程前置。

## 5 基线与验证记录

| 字段 | 当前值 |
| --- | --- |
| 当前分支 / 提交基线 | `codex/dist-react-restore`；P1-07 Host acceptance commit `25eb020c349fdc8f716bd5a991d37b1eb407aae7`，其后仅有 SESSION 元数据提交；工作树干净，未 push |
| 新路线实现基线 | P1-07 起始基线 b982e8d；最终 visual build SHA 8bd7a366d24fa2eff8bf78e22604f461030cedae9d1184c11a28ca8eb4f5c28d；Host acceptance commit `25eb020`；[build identity](evidence/p1-07/final-20261005/build-identity.json) 的源码身份与提交内容一致 |
| 方案调查基线 | `bda242a920d471b9598f98b57dde5d7c2505c35e`；只作原调查身份，不能当当前 HEAD |
| P0-01 交付（历史保留） | [最终验收](P0-01-baseline.md#current-acceptance)：8 当前源码 PNG、18 新真实脱敏 case、视觉数据、manifest/audit；四项 PASS |
| P0-01 验证（历史保留） | 77 AST（72/5）、4/6/9、54 浏览器操作/状态断言、8 PNG 与来源/脱敏/JSON/links/68 DAG；[最终计数](evidence/p0-01/validation.json) |
| P0-01 NOT_RUN（历史保留） | 当时未运行产品构建/测试、内核/权限原型；traffic 重连/重置与 DNS/failover 写语义未验证，保持 UNKNOWN |
| P1-01 本次验证 | [抽取记录](P1-core-and-shell.md#p1-01-delivery)；core 243 测试（178 明确纯单测）、旧入口 79 Mock/DTO 回归；build/tree/静态检查、两条 lib clippy/fmt/diff PASS；额外测试 lint FAIL 见记录 |
| P1-02 本次验证 | [类型/快照交付](P1-core-and-shell.md#p1-02-delivery)：275 core（新增 32）、65 旧入口定向回归；四项验收 PASS，check/build/lib clippy/fmt/tree/静态/文档/diff 通过；不调用真实内核 |
| P0-03 原型验证（历史保留） | [实机原型](evidence/p0-03/README.md)：check/build/clippy/fmt/core/tree 通过；11 截图，中文编辑/Modal/list 真实操作；无资源覆盖 workspace check FAIL（旧 Windows LICENSE 缺失） |
| P0-03 人工验收收口（2026-10-04） | [人工确认](evidence/p0-03/host-manual-acceptance.json)：用户亲手中文 Input/Modal 输入及托盘点击可用，明确批准四项 PASS/DONE；[验证](evidence/p0-03/validation.json)：两条指定 cargo check 复跑 PASS、JSON/11 PNG/68-card DAG/links/diff 通过；resource override 不代表打包资源完整 |
| P0-04 本次真实验证 | [真实内核交付](P0-feasibility.md#p0-04-delivery)：官方 1.14.0 arm64 digest、5 configs check、4 child 动态 controller/鉴权/SIGTERM 清理、selector/FakeIP restore、关闭 writer handoff PASS；存在对账前窗口；[validation](evidence/p0-04/validation.json)含 core check/脱敏/JSON/DAG/links/diff；仅同版本/cache_id/tag/pool/range |
| P0-07 本次真实验证 | [本地 Observation 交付](P0-feasibility.md#p0-07-delivery)：固定 archive/source、四 WS、traffic 两 burst/reconnect、long/short、DNS records/flush/hot rewrite、局部诊断 PASS；[validation](evidence/p0-07/validation.json)含 JSON/AST/脱敏/core check/68 DAG/links/diff/cleanup；首轮 checker FAIL 保留，无产品 Rust 改动 |
| 保留的限制 | candidate-window/tray screenshot 与独立前台 focus 缺证据、受控 window blur 对照缺失；card-level backdrop blur、MiSans、nested dropdown、macOS 15 实机未验证，不阻止 P0-03 DONE；Windows 仍 DEFERRED |
| P0-05 本轮原型 | [授权与平台证据](evidence/p0-05/README.md)：macOS 27 arm64 / minos 15.0 / adhoc，固定内核/manual FD、普通用户 OS peer/NOTE_EXIT PASS；管理员授权失败 -60008，所有 root/network 实测 NOT_RUN，ACCEPTANCE；现场干净 |
| P1-04B 本次验证 | [行为偏好交付](evidence/p1-04b/README.md)：293 core/24 desktop；规定 check/build/clippy/fmt、resource override workspace check/旧 lib clippy/tree PASS；实际字段编辑/校验、两轮真实冲突 rebase、磁盘失败 Retry、两轮重启和9张图通过；无网络/TUN/内核操作 |
| P1-05 本次验证 | [平台交付](evidence/p1-05/README.md)：294 core/33 desktop，规定构建/检查与脱敏截图 PASS；真实五次 secondary active/key、busy unconfirmed、SIGKILL/stale恢复、原生 Open/Save Cancel/Accept、边界拒绝、Cmd+V/已物化 payload 恢复、loopback OS handoff；正常退出 socket 修复后 PASS。额外旧 clippy 原资源 FAIL/override PASS 明示；所有 task root/进程已清，无系统代理/TUN/内核/公网动作 |
| P1-05 Host Review 修订 | [修订证据](evidence/p1-05/README.md#host-review-revision)：仅三处边界修复及证据措辞；新增 3 Desktop 回归，修复前均 FAIL，修复后 294 core / 36 desktop、用户指定检查与旧 lib clippy PASS。既有 GUI/源码身份保留，本轮未重跑 GUI；P1-06 保持 READY/未开始，无 commit/push、公网、sing-box/System Proxy/TUN |
| P1-06 人工验收收口（2026-10-04） | [User/Host manual acceptance](evidence/p1-06/host-manual-acceptance.json)：V 图标、真实菜单/准确未接入状态/灰色禁用启停、连续三轮关闭→托盘恢复、最终托盘退出均人工 PASS；Host 已查看两张截图，未复制入仓库，无本地图片/SHA。用户确认 Host 此前独立复核及 41 Desktop/294 Core 与指定检查全部 exit 0；本次不复跑产品验证，窗口 Quit 清理与托盘人工结果分开 |
| P1-07 最终收口 | [Host 复核](evidence/p1-07/final-20261005/host-final-review.json)：视觉 PASS_WITH_TECHNICAL_DIFFERENCES、组合 PASS，三项 blocker 已处理；[最终人工重启/退出](evidence/p1-07/final-20261005/human-host-final-restart-quit.json) 及 process/socket/flock cleanup PASS。首跑 Desktop 44 PASS/1 FAIL 保留，定向5/5、单线程45/45、默认并行45/45 PASS；仅记未复现的瞬态测试环境/flock 时序失败，目前无稳定回归证据，根因未证明；原日志不改 |
| 下一里程碑 | P0-09 仍等待显式前置；当前 READY 为空，不启动后续 Task |

## 6 里程碑记录

- [ ] P0 路线可行、范围明确与估算完成。
- [x] P1 桌面壳、配置保存与托盘可用（视觉 PASS_WITH_TECHNICAL_DIFFERENCES；业务页面仍由后续任务验收）。
- [ ] P2 首个可用 macOS 本机代理版本验收通过。
- [ ] P3 主页面基础能力、实时观测与历史查询验收通过。
- [ ] P4 高级配置、统一出口目录与 routing/client routing 验收通过。
- [ ] P5 DNS、Rules 完整集成与共享功能验收通过。
- [ ] P6 TUN、生命周期与更新集成验收通过。
- [ ] P7 macOS 候选安装包完成实际验收。
- [ ] Windows 单独排期及设备验收完成。

## 7 最近记录

2026-10-04 全局 UI 视觉迁移规范（仅文档）：更新 AGENTS.md、长期开发规范、方案 §9.3/§9.4 与迁移退役说明、P1-07 验收及本交接页。新增唯一 React/CSS 视觉事实来源、固定值精确迁移、GPUI Kit 外观覆写、字体/图标同源、95% 最低工程目标与逐页面/核心状态对照、Tokens/共享组件体系及视觉参考退役条件。P1-07 下一动作先审查当前壳层/设置视觉与组件体系，Proxies/Connections/Logs/Rules 留各自后续任务验收；仍唯一 READY、未领取/启动。68 卡状态/依赖不变：DONE12 / ACCEPTANCE1 / READY1 / TODO47 / DEFERRED7。本次不改产品源码、不运行 GUI/产品测试/构建、不访问公网或远端参考、不 commit/push。

| 日期 | Task / 文档动作 | 结果 | 后续 |
| --- | --- | --- | --- |
| 2026-10-03 | 创建任务清单与交接页（历史） | 当时 57 macOS + 7 Windows，初始均待办 | 历史计数保留 |
| 2026-10-03 | P0-02 流程/范围清理（历史） | 当时 1/57；64 任务、77 API、19 场景、150 本地链接与格式通过；仅发现 29 产品测试文件，未运行；Ruby 技能字段校验通过，Python 因缺 PyYAML 未运行 | 详细历史证据见 [P0 清理记录](P0-feasibility.md#cleanup-record)，不改写为本次结果 |
| 2026-10-03 | DAG 与长期开发规范（文档动作） | 最小拆分后 61 macOS + 7 Windows；仅 P0-02 DONE、P0-01 READY；153 个本地 Markdown links/anchors、68 Task 唯一/依赖存在/无环、计数与 READY 状态、77 API/19 场景归属、指定依赖/并行关系及差异格式检查 PASS | 实现从 Ready Queue 领取；本次无产品构建/网络/权限结果 |
| 2026-10-03 | F1–F7 Review 问题修复（仅文档） | 重新读取文件校验：153 本地链接/锚点、68 Task ID/依赖存在/无环、主表/卡一致、61+7 计数/状态、77 API 单一归属、19 场景、F1–F7 断言与格式/新文件 whitespace PASS；仅 P0-02 DONE、P0-01 READY | 未运行产品构建/测试/网络/权限；修复者做定向自查，不声称另有独立人工 Review |
| 2026-10-03 | P0-01 基线调查 | 静态 77/4/6/9、22 case 交付；仅视觉 NOT_RUN，ACCEPTANCE；[交付证据](P0-feasibility.md#baseline-record) | 补同尺寸五类浅深色视觉及身份后复核 DONE，重算下游；未改生产配置 |

| 2026-10-04 | P1-05 平台目录/单实例/原生文件 | 三项验收 DONE；P1-06 唯一 READY；P0-05 保持 ACCEPTANCE，P0-06/P0-08 未解锁 | 未 commit/push，未开启下游，Host Review 决定提交 |

每次实际推进后同步 Active Tasks / Ready Queue / Blocked、主表与计数；详细证据放相关任务文件或正式交付记录，旧交接留 Git 历史。

2026-10-03 CLI 替代路径实际完成只读请求及本机 HTML 身份核对；真实视觉未完成，站点授权不是当前缺口。详见 [只读观测与环境下一步](P0-01-baseline.md#readonly-observation)。临时 Vite 自有进程已停止；未安装依赖、改产品源码或生产配置，未 commit/push。

2026-10-03 仅 auth/login 的临时鉴权例外已实际执行一次，仍 403，未建会话；13 只读 GET 重采样及 4 WS 失败观测已脱敏登记，未取得 DTO/帧。jar/鉴权脚本删除；单次浏览器确认仍失败。P0-01 ACCEPTANCE、READY 空；见 [临时登录证据](P0-01-baseline.md#temporary-auth-observation)。

2026-10-03 最终验收：P0-01 DONE（四项 PASS），P0-02 DONE；3 READY/56 TODO/7 Windows DEFERRED。当前真实 Chromium 8 图和实际操作已通过；早前 403、启动失败、无帧与尺寸解析错误均保留在历史记录。本轮 GET 15、登录 POST 2、WS 4，其余写 0；临时资源清理完成，无 commit/push。见[当前最终验收](P0-01-baseline.md#current-acceptance)。

2026-10-03 P1-01 DONE：根 workspace/core 和旧入口共享实现，三处耦合及订阅系统代理读取已处理；3/61 DONE、3 READY、55 TODO。P1-02 READY，P0-03/P0-04 仍 READY；未启动下游，P0 历史保留，无 commit/push。见[交付记录](P1-core-and-shell.md#p1-01-delivery)。

2026-10-03 P1-02 DONE：schema v7、StateVersion/独立配置与选择版本、三态 ProfilePatch、类型化保存/错误与 Runtime owner 快照边界；四项验收 PASS。275 core/65 旧入口定向回归与规定检查通过。macOS 4/61 DONE、2 READY、55 TODO；READY 仅 P0-03/P0-04，P1-03 等待 P0-03 保持 TODO。owner 释放、未启动下游、无 commit/push/publish；[交付记录](P1-core-and-shell.md#p1-02-delivery)。

2026-10-03 P0-03 追加：ACCEPTANCE；Rust 1.99.0 / Kit 0.7.0 / gpui-pre 0.3.7 / tray-icon 0.24.2，macOS 27 arm64 构建 minos 15.0。四项卡仅构建项勾选；P0-04 READY，P1-03/P0-05 继续 TODO。未 commit/push，未开始下游。

P0-03 Host 追加确认：对绿色托盘图标、三轮显示/隐藏、菜单状态更新和退出回复“完成”；退出后精确进程检查无残留。按 Host 报告记录，不代替缺失的托盘截图/关闭后前台 focus 与 IME 证据。


2026-10-04 P0-03 Host/User Manual Acceptance：用户亲手完成真实中文输入（单行 Input 与 Modal Textarea）及托盘点击并确认可用，明确要求“就当验收通过了”。四项卡验收 PASS、P0-03 DONE；既有 IME candidate/composition、tray screenshot/focus、controlled blur LIMITATION 和无资源覆盖 workspace check FAIL 保留，不补造截图。两条指定 cargo check 本次复跑 PASS；[完整验证](evidence/p0-03/validation.json)。按 68-card DAG 重算 macOS DONE 5、READY 2、TODO 54、ACCEPTANCE 0；Windows DEFERRED 7。P0-04/P1-03 READY，P0-05 TODO；未启动下游、未改原型功能代码、未访问 OpenBox 远端、未运行真实 sing-box、无 commit/push。

2026-10-04 P0-04 DONE：固定官方 v1.14.0 macOS arm64 archive digest 匹配后运行；5 config checks / 4 child，动态 controller 从本 child 日志取得，鉴权 200/无鉴权 401；selector/FakeIP 及 closed-writer handoff 全通过、空缓存对照排除假恢复。restart/handoff 对账前窗口 22.990292/14.466458 ms；正式及试探资源已清理。68-card DAG：DONE 6、READY 3、TODO 52、Windows DEFERRED 7；READY P0-05/P0-07/P1-03、P0-06 TODO。前期字段 check FAIL 与试探配置未捕获 hash 的限制保留；sandbox 网络更新监听受限，不声称正式 Runtime/网络监听已实现。无系统设置修改、TUN run、公网协议测试、后续 Task、commit/push；[完整记录](evidence/p0-04/README.md)。

2026-10-04 P0-07 DONE：四项验收 PASS；traffic 区间 bytes、两 burst/reconnect、短连接缺口、DNS records/hot rewrite unsupported、flush supported、真实局部诊断；全部自有资源清理。READY P0-05/P1-03，无新增，未启动下游，未 commit/push；[完整记录](evidence/p0-07/README.md)。


2026-10-04 P0-05 ACCEPTANCE：隔离 Rust/原生 adapter、closed IPC、受管配置 FD/身份/恢复逻辑与固定安装/卸载 harness 已实现；普通用户 API/手动代理及 5 定向测试有真实证据。标准系统管理员授权实际失败 -60008 / hiservices-xpcservice Connection Invalid；未创建 daemon、受保护目录或 Network Service，root/network 项 NOT_RUN，无真实用户取消证据。现场已清理，68-card DAG 不变，READY 仅 P1-03，未启动下游、无 commit/push；[完整证据](evidence/p0-05/README.md)。

2026-10-04 P1-03 ACCEPTANCE：正式 desktop 六页/九分类与单 Tokio/Core bridge 完成；11 tests、desktop check/build/clippy、fmt/tree、resource override workspace check 及旧 lib clippy PASS。实际 GUI 六页/九分类、filter/draft、Busy、迟到/旧实例、坏 JSON/Retry、浅深色 11 图已取得；随后仅持有表参数化以增加两项离线纯回归，最终原生 Entity 复核因 Mac locked 未运行。保留 ACCEPTANCE、READY 空，未启动下游；详情及现场清理见 [P1-03](evidence/p1-03/README.md)。

2026-10-04 P1-03 最终补验 DONE：用户解除锁屏后，最终构建原生 Entity 三组断言、filter/DNS category/draft 往返全部 PASS；新增 final-settings-retained.png，共 12 图。通过 UI 正常退出，无 desktop/child 残留，四个隔离 root/坏 JSON/临时 app 清除。68 卡重算：macOS DONE 8、ACCEPTANCE 1、READY 3、TODO 49；Windows DEFERRED 7。READY 仅 P1-04A/P1-04B/P1-05，下游未开始。无 commit/push；[完整证据](evidence/p1-03/README.md)。

2026-10-04 P1-04A READY → DOING；owner Codex Desktop · GPUI / Core-Config，公共视觉 DTO 单一 owner。P1-04B/P1-05 保持 READY，不启动下游。

2026-10-04 P1-04A DONE：三项验收通过；schema8 的 v7→v8 显式迁移保留 epoch/双 revision，视觉 CAS/SavedOnly/no-op、合并保存及背景 reference 原子提交完成。280 core（新增5）/17 desktop（新增6）、指定 check/build/clippy/fmt/workspace check PASS；resource override 仅 check，不代表旧包资源齐全。实际 GPUI 重启/保存失败/Retry/键盘/焦点与浅深色27图，用户亲手 IME/连续 Slider/stable drag 回复“全部正常”；Input裁切、侧栏icon、底栏对比三项反馈修复并复测。所有隔离根/资产/app/staging和进程已清；无代理/TUN/公网/内核操作，无 commit/push。68卡实际依赖重算：macOS DONE9、ACCEPTANCE1、READY2、TODO49；Windows DEFERRED7。READY仅P1-04B/P1-05；P0-05仍ACCEPTANCE，下游未启动，owner释放。[完整证据](evidence/p1-04a/README.md)。

2026-10-04 P1-04B READY → DOING：只领取行为 Preferences；P1-05 保持 READY；P0-05 保持 ACCEPTANCE；AppConfig/Panel 共享 owner 由 Codex Desktop · Core/Config + GPUI 预约。

2026-10-04 P1-04B DONE：schema8→9 保留 identity/双 revision/visual/Profile；行为 typed partial patch、SavedOnly/CAS/no-op、三种 URL 与两种 IPv6 独立、四类消费者通知契约完成。293 core（新增13）/24 desktop（新增7）及所有规定构建检查 PASS。实际 Panel 全字段/错误、真实磁盘失败与草稿、两轮冲突仅重放 timeout 并保留外部列数、最终重启 config16/selection0/同 epoch 和9图通过。隔离资源/进程已清，无公网/代理/TUN/sing-box/HTTP/login，未 commit/push。68卡重算：DONE10/ACCEPTANCE1/READY1/TODO49、Windows DEFERRED7；READY仅P1-05，未领取；P0-05仍ACCEPTANCE，下游未启动；共享 owner 释放。[完整证据](evidence/p1-04b/README.md)。

2026-10-04 P1-05 READY → DOING；owner Codex Desktop · Runtime/Platform + GPUI。只领取平台目录/单实例/文件操作；P0-05 保持 ACCEPTANCE，不启动 P1-06/P1-07/P2。

2026-10-04 P1-06 READY → DOING：基线 f9adda4，保留全部 P1-05/Host Review 既有修改；Codex Desktop 只接正式托盘/窗口生命周期。未 commit/push、未启动 P1-07/P2。

2026-10-04 P1-06 ACCEPTANCE：正式 tray-icon 0.24.2 主线程长期持有、类型化 channel 直接唤醒 GPUI、关闭只隐藏与统一显式退出已实现。41 desktop（既有36+新增5）/294 core 与规定检查通过；真实关闭/native hidden、五次 secondary 同窗口 active/key/writer0、筛选保留、窗口 Quit 清理 PASS。原生工具 AX 未暴露托盘入口，SystemUIServer 超时、键盘入口无效果；等待用户展开菜单，不能计三轮托盘恢复/菜单图/托盘退出通过。READY 空，P1-07/P2 未启动，P0-05 仍 ACCEPTANCE；保留既有修改，未 commit/push。[完整证据](evidence/p1-06/README.md)。

2026-10-04 P1-06 User/Host manual acceptance 收口 DONE：用户看到 V 托盘图标并真实展开菜单，确认准确未接入状态与灰色禁用启停；连续三轮“关闭窗口 → 托盘‘显示窗口’恢复”均正常，最终托盘“退出”正常、无异常。Host 会话已查看用户两张截图，未复制入仓库，不伪造本地图片/SHA。Human Visual / Tray interaction 已补足，三项任务验收全部勾选；此前工具限制/NOT_RUN 和窗口 Quit 程序化清理保留，托盘退出无新增 process/socket/flock post-check。用户确认 Host 此前独立复核及 41 Desktop/294 Core 与指定检查均 exit 0，本次只做最小文档校验。68 卡重算：macOS DONE12 / ACCEPTANCE1 / READY1 / TODO47；Windows DEFERRED7，唯一 READY 为 P1-07，未领取/启动；P0-05 仍 ACCEPTANCE，P0-06/P0-08 仍 TODO。保留产品源码/AGENTS.md 等既有修改；无公网/sing-box/System Proxy/TUN 操作，无 commit/push。[人工证据](evidence/p1-06/host-manual-acceptance.json)。

2026-10-04 P1-07 本轮进行中：从干净 b982e8d 建立本地 React 16 张1280×720 reference，实施 Tokens/Theme、共享基础组件与 Shell/Panel 重组，工程入口隔离为 evidence-only；Desktop42/Core294 与要求的自动检查通过（初次clippy失败历史保留）。Host 本轮未看到托盘，native rect68×0且visible=true，根因未确定；真实保存/关闭隐藏/secondary writer0恢复及诊断重启值恢复有证据，但托盘恢复/显式退出组合未通过。字体分片/模糊差异与 hover/focus/error 等尚未完成的视觉状态分开记录。P1-07 保持 DOING、owner GPUI，其他状态不变，不启动P2、不commit/push。详见 [本轮证据](evidence/p1-07/README.md)。用户后续仅授权只读线上编辑入口核对，已恢复原页；其余 reference/API/WS 均本地fixture。

2026-10-04 P1-07 Host Review 修订：原Light Shell是System解析Dark的采集错误；新root显式Light/Dark/Core-load后1280×720四状态复核并保留历史。默认背景decode/resize/blur/encode移入blocking服务，Render只读ready cache，三条stale/dedup/职责回归；真实Blur drag后Select即时响应。同环境shell-wrapper baseline/current均不可见，真实Mach-O bundle二者真人可见且菜单正常；用户完成三轮Close→Tray Show、Tray Quit、重开值恢复Light/6500ms/IPv6on、再次Tray Quit，native root socket自动清除/锁释放/无进程。后续Button焦点/Tooltip定位修复和适用组件状态独立补验；45 Desktop/294 Core及完整检查PASS，最后Tooltip影响范围另记录。字体/局部backdrop blur和尚未完成视觉细节继续明确记录，整卡保持DOING、owner GPUI，不声称95%/DONE；其他Task不变，未启动P2/业务页/内核/公网，未commit/push。见[evidence](evidence/p1-07/README.md)。

2026-10-05 P1-07最终视觉续录（DOING/GPUI）：627728a2统一1280×720显式Light/Dark新采图及Tooltip箭头/spinner修正已复核，45 Desktop/294 Core与完整检查PASS；用户完成至少三轮真实Tray Show和Tray Quit，重启Settings值恢复Light/6600ms/IPv6on。末补查IconPicker展开态发现可修tab均分/局部focus，已最小源码修正并重跑11项自动验证PASS，新build8bd7a366尚待GUI采集与最终组合；旧627728a2帧/操作完整归档，不能冒称最后源码视觉PASS。保留旧重启窗口PID77189等待用户已请求的Tray Quit，不以keyboard/SIGTERM替代该操作；之后换新实际Mach-O bundle继续。整卡DOING、owner GPUI，其他Task不变；无commit/push/P2/公网/sing-box/System Proxy/TUN。[完整当前状态](evidence/p1-07/README.md)。

2026-10-05 最后构建8bd7a366视觉复核续录：Light/Dark显式1280×720 Shell展开/折叠、Panel及当前组件状态已重采；Tooltip箭头/定位、spinner、IconPicker分类均分/局部focus/trigger已对照React级联和叠图复核，技术差异单列字体fallback/element backdrop blur/UA栅格。最后源码完整11项检查PASS（45 Desktop/294 Core），无新源码改动。实际bundle保存Light/6600ms/IPv6on，用户回报完成三轮与退出；日志仅Show2/Close3，明确不计3次日志PASS。Tray Quit后进程/socket清理、flock可重新获得，重启实际Settings值恢复PASS；已重做视觉/行为保存并隐藏，等待真人明确3Show→Tray Quit补齐。P1-07保持DOING、owner GPUI，其他Task/DAG不变；无commit/push/P2/公网/内核。见[evidence](evidence/p1-07/README.md)。

2026-10-05 P1-07 最终 Host 验收收口 DONE（仅文档/evidence）：同 8bd7a366 build 最终视觉/AX/comparison 与人工查看齐备，无新的可修视觉 blocker，MiSans/NotoEmoji fallback、element-level Card/Modal backdrop blur、少量 UA 栅格差异继续为 TECHNICAL_DIFFERENCE；Tokens/Theme → 基础组件 → NavigationItem/CompactSetting/Section → 页面保留。明确人工 Show #1 → Close → Show #2 → Close → Show #3 → Close → Tray Quit 及 Host post-check PASS，旧 first-launch Show2/Close3 日志不改写。相同 Mach-O bundle/root 重启 PID94362，Light/6600ms/IPv6on、snapshot_loaded、单 primary/writer、state bytes 不变 PASS；用户“最终退出完成”后 Tray Quit，Host PID gone/socket absent/flock free PASS，瞬态 PID96193 不计残留。Host 首跑 Desktop44/1 FAIL 与后续5/5、45/45、45/45 PASS 分别保留，根因未证明；Core294及指定检查 PASS 不覆盖旧日志。六项验收勾选、owner 释放并移出 Active Tasks。68 卡 DONE13 / ACCEPTANCE1 / READY0 / DOING0 / TODO47 / Windows DEFERRED7，无环、无新 READY；P0-05/P0-06/P0-08 与后续业务页状态不变。未修改源码/Cargo/assets/测试/AGENTS.md/长期规范，未删 Legacy 或 Host 复核文件，未 commit/push，未启动 P2/公网/sing-box/System Proxy/TUN。见[最终交付](P1-core-and-shell.md#p1-07-final-closeout)及[文档验证](evidence/p1-07/final-20261005/closeout-validation.json)。
2026-10-05 P1-07 Host 提交：最终实现、共享组件、Heroicons 资产、完整视觉/交互 Evidence 与 DONE 状态由 Host 提交为 `25eb020c349fdc8f716bd5a991d37b1eb407aae7`；其后仅提交本交接页元数据，不改变 P1-07 实现或验收身份。未 push，P2 未启动。
