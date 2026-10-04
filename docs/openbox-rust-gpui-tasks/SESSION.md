# OpenBox Rust / GPUI 当前进度

最后更新：2026-10-04。

**当前结果：OBG-P0-01、P0-02、P0-03、P0-04、P0-07、P1-01、P1-02 DONE；P0-05 ACCEPTANCE；P1-03 DONE。** P0-07 完成标准四流/区间 traffic/短连接/DNS 三能力/本地诊断；DNS records/hot rewrite unsupported，flush supported。P0-04 存在对账前流量窗口。P0-03 人工确认和历史限制保留；helper/TUN 运行未验证。阶段用于里程碑分组及组合验收，READY 只由显式依赖决定。

- [长期开发总规范](DEVELOPMENT_WORKFLOW.md)：后续 Veyra Rust/GPUI 默认遵守；四泳道、并行与单一 owner。
- [技术方案](../openbox-rust-gpui-implementation-plan.md)：架构、行为、接口与平台边界。
- [任务总表](IMPLEMENTATION_PHASES.md)：唯一状态表、显式依赖与覆盖归属。

## 1 进度总览

| 里程碑 | DONE / 任务数 | 当前结果 | 组合验收 |
| --- | --- | --- | --- |
| P0 基线与可行性 | 5 / 9 | P0-01/03/04/07 验收通过；固定内核/缓存/观测能力核实完成，历史限制保留 | P0-09 未开始 |
| P1 核心库与桌面壳 | 3 / 8 | core、类型/快照及正式 GPUI 壳/异步桥通过 | P1-07 未开始 |
| P2 本机代理闭环 | 0 / 10 | 未开始，按各卡依赖推进 | P2-09 未开始 |
| P3 观测与主页面 | 0 / 8 | 未开始，基础能力不计完整 DNS/分流 | P3-08 未开始 |
| P4 完整配置能力 | 0 / 9 | 未开始，Chain 在 Routing 前交付 | P4-07 未开始 |
| P5 DNS 与共享 | 0 / 7 | 未开始，Rules 在 DNS 后最终闭合 | P5-07 未开始 |
| P6 macOS TUN 与生命周期 | 0 / 5 | 未开始，按各卡依赖推进 | P6-05 未开始 |
| P7 数据与发布收尾 | 0 / 5 | 未开始，schema/清理按显式依赖等待 | P7-05 未开始 |
| Windows W0–W3 | 0 / 7 | 后续排期 DEFERRED | W3-02 未开始 |

macOS：**8 / 61 完成**；READY 3、TODO 49、DOING 0、REVIEW 0、ACCEPTANCE 1、BLOCKED 0。Windows 7 项 DEFERRED 单列，共 68 项。数量不等于工期权重或代码完成百分比；P1-04/P2-02/P4-05 父项被后缀子项替代，不重复计数。

## 2 Active Tasks

| Task | owner / 泳道 | 写范围 / 公共契约 | 实际资源 / 下一动作 |
| --- | --- | --- | --- |
| P0-05 · ACCEPTANCE | Codex /root · Runtime/Platform | `crates/veyra-helper/` 显式 `p0-05-prototype`、固定编排、P0-05 evidence/任务文档；本轮根 Cargo workspace/lock 已整合，core/公共 DTO 不改 | 系统授权 -60008，管理员入口未运行；原型 /Library/launchd/service 从未创建，普通用户 child/temp 已清。待可呈现标准管理员 UI 的本机环境补真实特权验收；不能计 DONE |

P0-05 历史起始 HEAD `3111a57`。本轮 P1-03 起始 HEAD `f460e767957569e9f4a035132e26cc216277e98f`，工作树干净；只领取 P1-03，不启动下游，不 commit/push。独立 label/path/socket/service/child/tmp 的现场预约已释放；补验时重新检查身份、版本与无残留。

## 3 Ready Queue

本队列由主表状态与任务卡依赖计算，不按阶段或编号统一放行。写范围冲突的 READY 留在队列并注明等待；未完成普通依赖的任务仍为 TODO。

| Task | 依赖满足依据 | 候选写范围 / 并行条件 | 下一具体动作 |
| --- | --- | --- | --- |
| [P1-04A](P1-core-and-shell.md#obg-p1-04a) | P1-02/P1-03 DONE | GPUI 视觉偏好/组件；与 B 预约共享文件 | 未领取，不自动启动 |
| [P1-04B](P1-core-and-shell.md#obg-p1-04b) | P1-02/P1-03 DONE | 行为偏好/Core DTO；与 A 预约单一 owner | 未领取，不自动启动 |
| [P1-05](P1-core-and-shell.md#obg-p1-05) | P1-02/P1-03 DONE | 平台目录/单实例/文件桥 | 未领取，不自动启动 |

按 68 张实际任务卡依赖重算：READY 仅 P1-04A/P1-04B/P1-05，未启动下游。P1-06 仍缺 P1-05；P2-01 缺 P1-04A/P0-06，P2-03 缺 P2-02B，其余 P2 卡仍按各自依赖 TODO。P0-05 ACCEPTANCE，P0-06/P0-08 不解锁。P1-03 workspace/GPUI 资源 owner 已释放，修改未提交。

## 4 Blocked

当前没有实际 BLOCKED。普通依赖未完成和未启动原型不伪标为外部阻塞；实际遇到问题后填写下表。

| Task / owner | 实际阻塞原因 | 解除条件 / 下一动作 | 受影响任务 |
| --- | --- | --- | --- |
| 无 | — | — | — |

后续限制按卡追踪：GPUI 的 MiSans/nested dropdown/card backdrop 在 P1-04A 延续，macOS 15 实机及历史受控 blur/截图缺口保留于 P0-03；P0-04/07 已核实内核/缓存/观测/DNS 边界，出站 P0-06 待办；管理员 helper P0-05 原型已实现，但标准授权 -60008 待补特权验收；首次打开 P0-08、新版估算 P0-09。GitHub 无发布签名/公证和旧门禁退役已由 P0-02 记录，无补办流程前置。

## 5 基线与验证记录

| 字段 | 当前值 |
| --- | --- |
| 当前分支 / HEAD | `codex/dist-react-restore` / `f460e767`；P1-03 起始工作树干净，本轮修改未提交 |
| 新路线实现基线 | 根 workspace/core、旧入口共享接线、schema v7 与 GPUI 原型已在起始 HEAD；本轮新增正式 desktop 壳层、异步状态桥及 P1-03 证据 |
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
| 下一里程碑 | P0-09：汇总原型结论、实际范围和重新估算；不阻塞无关已就绪任务 |

## 6 里程碑记录

- [ ] P0 路线可行、范围明确与估算完成。
- [ ] P1 桌面壳、配置保存与托盘可用。
- [ ] P2 首个可用 macOS 本机代理版本验收通过。
- [ ] P3 主页面基础能力、实时观测与历史查询验收通过。
- [ ] P4 高级配置、统一出口目录与 routing/client routing 验收通过。
- [ ] P5 DNS、Rules 完整集成与共享功能验收通过。
- [ ] P6 TUN、生命周期与更新集成验收通过。
- [ ] P7 macOS 候选安装包完成实际验收。
- [ ] Windows 单独排期及设备验收完成。

## 7 最近记录

| 日期 | Task / 文档动作 | 结果 | 后续 |
| --- | --- | --- | --- |
| 2026-10-03 | 创建任务清单与交接页（历史） | 当时 57 macOS + 7 Windows，初始均待办 | 历史计数保留 |
| 2026-10-03 | P0-02 流程/范围清理（历史） | 当时 1/57；64 任务、77 API、19 场景、150 本地链接与格式通过；仅发现 29 产品测试文件，未运行；Ruby 技能字段校验通过，Python 因缺 PyYAML 未运行 | 详细历史证据见 [P0 清理记录](P0-feasibility.md#cleanup-record)，不改写为本次结果 |
| 2026-10-03 | DAG 与长期开发规范（文档动作） | 最小拆分后 61 macOS + 7 Windows；仅 P0-02 DONE、P0-01 READY；153 个本地 Markdown links/anchors、68 Task 唯一/依赖存在/无环、计数与 READY 状态、77 API/19 场景归属、指定依赖/并行关系及差异格式检查 PASS | 实现从 Ready Queue 领取；本次无产品构建/网络/权限结果 |
| 2026-10-03 | F1–F7 Review 问题修复（仅文档） | 重新读取文件校验：153 本地链接/锚点、68 Task ID/依赖存在/无环、主表/卡一致、61+7 计数/状态、77 API 单一归属、19 场景、F1–F7 断言与格式/新文件 whitespace PASS；仅 P0-02 DONE、P0-01 READY | 未运行产品构建/测试/网络/权限；修复者做定向自查，不声称另有独立人工 Review |
| 2026-10-03 | P0-01 基线调查 | 静态 77/4/6/9、22 case 交付；仅视觉 NOT_RUN，ACCEPTANCE；[交付证据](P0-feasibility.md#baseline-record) | 补同尺寸五类浅深色视觉及身份后复核 DONE，重算下游；未改生产配置 |

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
