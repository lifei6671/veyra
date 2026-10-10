# P2-05 Host 最终收口（2026-10-10）

**Host 正式批准 P2-05 ACCEPTANCE → DONE，释放本卡 owner；决定为 `SCOPED_ACCEPTANCE_WITH_EXPLICIT_DEFERRED_PLATFORM_VERIFICATION`。** 已完成正式客户端与本轮局部功能、Native、GPUI 验收；物理 Direct 仍 **UNVERIFIED**，SystemProxy ON 仍 **NOT_RUN**，正式受管 TUN 与完整网络组合由 P2-07/P2-09/P6-01/P6-05 承接。原卡第二项系统组合复选框保持未勾选、可追溯。DONE 不表示这些延期能力通过，不启动下游任务。

## 决定依据与历史关系

本记录属于同一 WorkRun `work-23397-1791600619806627-237`。用户 2026-10-09 明确“保持 ACCEPTANCE，等待完整网络组合验收”；工程整合结束和本轮实机交接均遵守该决定，原文、失败及未运行收据保留。用户本轮最新请求为“P2-05 客户端功能及本轮局部Native/GPUI验收通过，保持ACCEPTANCE，由Host确认最终收口”。本次由用户委托的 Host 依据新实机证据及已批准的[2026-10-08 分期规则](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)正式审定 DONE，明确处理旧等待决定；并非将旧 ACCEPTANCE/NOT_RUN 静默改写成历史 PASS。

该规则要求前期正式 Service/Store、持久化/重建回读、局部受控集成与实际局部 UI 已完成，允许 OS 授权/正式内核网络的具体子验证与完整组合延期。Host 确认本卡已具备这些前期证据。P2-07 的显式依赖包含 P2-05，P2-09 又依赖 P2-07；不能反向要求 P2-07/P2-09 完成后才将本卡 DONE，否则形成循环等待。原 DAG、业务需求、平台归属与安全约束不变。

[工程整合记录](P2-05-integration-acceptance.md)、[实机交接](P2-05-desktop-acceptance.md)及[独立 REVIEW](evidence/p2-05/desktop-20261010/REVIEW.md)中的 ACCEPTANCE/待 Host 结论是本次决定之前的快照，原样保留；当前状态以本记录、总表唯一任务行与 SESSION 当前队列为准。

## 候选、源码与收据身份

唯一开发树为 `.worktrees/p2-05-main-integration`，分支 `dev/p2-05-main-integration`，输入及当前 HEAD `bda8be5c469dcbd6093a28fd4fd549be70c22194`。本候选包含该 HEAD 上的未提交整合变更，不能单用 HEAD 表示全部交付源码。旧独立检查点 `e530837976257d489d3bf455d5d96e63e704f442`、原树 dirty Native 修复及主树既有内容保留，不覆盖或重写。没有新 commit/merge SHA。

Host 通过 MCP 独立比对最终清单的 **18/18 源码/配置 SHA 一致**，包含未跟踪 `crates/veyra-core/src/application/network.rs` 与 `network/tests.rs`。清单 [source-review-final.json](evidence/p2-05/desktop-20261010/source-review-final.json) 绑定完整路径与逐文件 SHA；本次文档收口再次只读核对 18/18 MATCH，无源码修改或候选漂移。保留 P4-03 Failover/Group/Observation 与 P2-06 的 Runtime/Platform/Helper/IPC/共享 DTO 单一 owner。

原构建 desktop SHA256 为 `2b9321af9337f64f90f2b09e7b36b994bb8071ee40c9c0422d47609e07d8475a`；最终 ad-hoc 包 `target/p205-desktop/VeyraP205Acceptance.app` 主程序 SHA256 为 `7429f20ad0d6b8c7f1dd8d4c4b92f917757b4a16c84f91ed919658535b5b0650`，不同签名快照不混用。固定 sing-box 1.14.0 darwin-arm64 SHA256 为 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，身份见本机 `gui-build-final.json` / `kernel.json`。

以下路径相对本任务文档目录，SHA256 为本次只读读取已有文件所得；不编辑原始收据。

| 本机收据 | SHA256 |
| --- | --- |
| [REVIEW.md](evidence/p2-05/desktop-20261010/REVIEW.md) | `71e13a1ef6c05ccb237103b4dc7a3de17ad0d7d69220884d784c017479d6e97c` |
| [native-production.json](evidence/p2-05/desktop-20261010/native-production.json) | `02338b710bc2f78d19bb98718db2024a7f9a0c3cd07c8128f9e6b12639767cb6` |
| [native-production.log](evidence/p2-05/desktop-20261010/native-production.log) | `31c873459e48b3fcec27a37330302506ac68ce8673c5f22499aefa22428dcb68` |
| [source-review-final.json](evidence/p2-05/desktop-20261010/source-review-final.json) | `c494d7c4da6e925bd21223ec2245a4067b9e0b048ec8d21d31189cde08a161e3` |
| [native-cleanup.json](evidence/p2-05/desktop-20261010/native-cleanup.json) | `3e005f8c870679b0ab06f5ccc6b1ede9d13040210e9b66a04da2912a36a0ee65` |
| [gui-final-quit.json](evidence/p2-05/desktop-20261010/gui-final-quit.json) | `f7517d2f3616fbee0f50dd605635fe0286d914b2db586a2ae771b3bd72632ed9` |
| [network-protection-final.json](evidence/p2-05/desktop-20261010/network-protection-final.json) | `50759c35f2bb192965f7178901b4243f043ced0e59cd0d2acab9ec9c814bc30f` |

Evidence 目录被 Git ignore；上述相对链接只保证本机可读取，不宣称会自动随 Git 交付。本次不复制截图、大日志、二进制或隐私数据，不修改 `.gitignore` 或已有 REVIEW。Host 提交时可选择精确纳入独立 REVIEW 与源码身份清单等轻量收据；未纳入的原始 Native/GUI/网络证据继续按上述本机路径与 SHA 留存。未跟踪源码仍需 Host 在任务提交中精确纳入，本 Agent 不操作 index。

## 实际验证与验收范围

| 执行 / 复核主体 | 实际结果与保护的行为 | 退出码 / 来源 |
| --- | --- | --- |
| 工程整合 Agent | **225 唯一 PASS：Core212 + Desktop13**；显式出站、凭据/重定向、Store/CAS/失败保旧及重建、能力撤销、applied NodeId、P4-03/Observation 共存 | 九组命令各 exit0；[整合验收](P2-05-integration-acceptance.md)及本机 main-integration 原日志 |
| Codex Desktop | 唯一指定 `runtime_service::quit_tests::outbound_clients_real_production_services` **1 PASS / 136 filtered**；正式 Preview/Save、HTTP503保旧、三 provider、指定 site/NodeId 路径、Replace/Quit 撤销和清理 | exit0；测试22.50s，runner23.027s，外层600秒；上表 Native JSON/log |
| Codex Desktop | Core `application::network::` **4 PASS**、`application::manual_runtime::` **46 PASS**；Desktop build/all-targets Clippy/workspace fmt 通过 | 各 exit0；本机 desktop-20261010 JSON/log；50项与225重叠，不累加 |
| Host MCP 独立执行 | Core `application::network::` **4 PASS**、`singbox::clash_api::` **16 PASS**；指定路径/关闭取消、当前实例鉴权/controller 既有边界 | CommandRun `command-23397-1791602919967485-251` **exit0**；Host 本轮明确提供的独立执行结果，20项与225重叠，不伪称本 Agent 新跑 |
| 独立 Reviewer / Host | REVIEW **0 可信可操作源码 Finding**；Host核18源码 SHA、Native exit0/1PASS及清理/网络保护收据，另 `git diff --check` PASS | 原 REVIEW SHA 如上；Reviewer 是只读复核，不能替代实机执行 |

工程三包 Core/Desktop/Helper all-targets Clippy/check/build 与 workspace/src-tauri fmt 已通过，详见原整合收据。旧183映射为182同名PASS及输入 main 已替换的新选择拒绝契约PASS，不恢复旧允许未应用配置选择行为；225不是历史累加。`application::failover::` 曾过滤零测试、exit0不计PASS，实际 Groups20已跑。历史 HTTP503 Native FAIL、Legacy Windows `libcronet.dll` Clippy **FAIL101**、初始 fixture/GUI失败及证据纠错全部保留，不因新局部成功覆盖旧记录。

正式客户端下载固定 Direct/当前可撤销 ViaRunningProxy，无静默换路、历史端口或下载专用 Runtime；GeoIP 请求独立于订阅凭据，三 provider 归一化、未知保留 None；节点测速只消费同一当前鉴权 Controller 的实际 applied NodeId/tag。跨源剥离认证/条件验证器、HTTPS降级拒绝、TLS校验和日志脱敏有定向正反回归。Native 指定 HTTPS site 708ms、NodeId=a/node-a 806ms，accepted socket blocking修复、双向TLS1707/4083bytes真实断言通过；这是 **node→自有 HTTP fixture→同当前 mixed→既有外部 TUN** 的受控拓扑，不是独立远端节点或物理 Direct 证明。

实际 GPUI 覆盖订阅空/下载busy/成功添加与刷新/HTTP503保旧、真实写失败保草稿及重试、重启回读；Panel latency草稿6789中文→English保持、English/Light重启恢复；主备编辑/保存/回读、同实例 Controller ManualPin→Auto、真实日志过滤/暂停、同PID46045托盘恢复与最终Quit。6789未提交不当保存结果，fixture节点无服务的 All lanes unavailable 不当健康故障转移PASS，指标 `—` 不当观测全指标PASS，退出后自有fixture准备不当P2-08 UI激活PASS。长socket根失败、旧21截图新启动误判及旧pin错误字段收据保留，最终正确收据补足实际验证。

本机 Apple M4 Pro / macOS27.0.1，实际默认1512×982、Retina scale2、内容1280×720、Light；用户明确批准“按当前 macOS 默认显示配置验收”。这是本轮局部显示条件的明确决定，不从截图推导额外150%档位、深色或其它缩放通过。固定内核及测试包 minOS26.0；构建变量15.0不证明macOS15支持，最低设备兼容继续归P7，不修改P0-09历史估算。

## 原卡逐项结果与延期归属

| 原验收项 | 本次结果 | 范围 / 承接 |
| --- | --- | --- |
| 1 首订/无代理/显式代理下载，无静默换路 | **PASS，勾选** | 正式客户端/Store定向回归及本候选局部Native；保留外部TUN下DirectPathUnverified准确拒绝 |
| 2 系统代理真实出口，TUN生产复验 | **未勾选**：SystemProxy ON **NOT_RUN**；物理Direct **UNVERIFIED**；正式受管TUN未实现/完整组合未跑 | [P2-07](P2-local-proxy.md#obg-p2-07)实施与恢复、[P2-09](P2-local-proxy.md#obg-p2-09)完整SystemProxy出口；[P6-01](P6-macos-lifecycle.md#obg-p6-01)生产TUN/bypass及[P6-05](P6-macos-lifecycle.md#obg-p6-05)真机网络组合。原P0证据保留；这些后期子条件不反向阻塞本卡前期DONE |
| 3 跨源认证隔离/凭据URL不降级/脱敏 | **PASS，勾选** | 原始源码Review及正式客户端定向回归；不依赖真实系统代理组合来首次实现 |
| 4 指定测速/IP路径，未知状态与Geo凭据隔离 | **PASS，勾选** | 自动回归与上述本候选受控Native；后续测速UI/临时实例功能仍由P2-08/P3-05分别实现验收 |

第二项的完整原文及未勾选状态保留在[原卡](P2-local-proxy.md#obg-p2-05)，清单仍可显示该平台子验证未完成。Host 此次是按已批准分期规则作任务级DONE决定，有明显延期例外；不是把整张卡所有网络系统条件宣布PASS。上述承接卡自身仍须按原DAG和实际权限/设备完成验收。

## 用户环境保护与本次交付

实机收据显示完整路由字节及默认路由/DNS/proxy/utun/preferences均未变；默认IPv4/IPv6仍utun4，HTTP/HTTPS/SOCKS/PAC关闭。Karing1267、扩展1356、既有Clash helper822身份/开始时间/命令一致。没有管理员Veyra helper安装或全局网络操作，不更改用户正在使用的应用、TUN、DNS或路由。Native仅自有普通用户实例、loopback及临时root；两child42989/43041、四监听及root已清理。最终GUI的测试App/child PID和新旧端口/socket均退出/释放，自有HTTPfixture43410/64413及短/初始失败长root清理；只保留包和原始证据。

本次仅新增本文件，更新 P2-local-proxy 的本卡验收、IMPLEMENTATION_PHASES 的必要状态行与当前摘要、SESSION 当前摘要/队列。P2-05本卡剩余 **0**；P0-09的2026-10-10估算快照保持原文，系统延期项继续由各承接卡承担，不重写其已DONE记录。当前 **68卡：DONE31 / ACCEPTANCE0 / DOING1 / READY5 / TODO24 / DEFERRED7**；macOS31/61，P2为6/10。READY恰为P2-08/P3-05/P4-01/P4-04/P5-01，全部未领取/未启动；P2-07仍缺P2-06 DONE，保持TODO。P2-06继续DOING、原Runtime/DTO owner保留。

本次只做文档、收据SHA、DAG/覆盖/链接/保护与格式核验，不重新运行产品测试、Native、GUI或网络，不修改任何产品源码/测试、Cargo.lock、原始Evidence、主树、旧树或Git index。没有commit/merge/push/发布。文档交Host再次独立Review；仅Host在无Finding后按受管Git完成一次任务提交、合并主开发分支及复验。

## 本次文档验证

只读脚本最终exit0：68卡唯一/逐卡依赖与原卡68/68一致、无环、READY恰5、状态与SESSION队列一致；77API唯一覆盖、19场景、4stream归属不变，覆盖节原文字节未改。原P2-05卡只增加最新决定/延期说明及1/3/4勾选，逆向去除这些授权修改后与收口前完整文件SHA一致，P2-06卡及owner行保持原样；日期历史段、P0-09估算及其它任务行保留。

本机20处新增引用/链接检查、7份收据SHA和18源码SHA核对、新文档空白检查、`git diff --check`均通过。收口前961份文件保护快照中，只有指定3份已有文档变动，另外958份SHA未变；新增仅本文件。`git status --short`只比本轮起点多本文件，源码dirty/未跟踪状态是既有整合交付，并非本次修改。校验脚本初次对零状态键及卡片边界解析的断言报错，已修正并实际exit0；聚合读取旧文档被工具截断的问题改用完整文件SHA逆向比对解决，不当作产品缺陷或测试PASS。
