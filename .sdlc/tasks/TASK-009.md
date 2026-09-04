---
id: TASK-009
milestone_ref: M6
dependencies: [TASK-008]
risk: HIGH
status: IN_PROGRESS
design_refs:
  - .sdlc/design/DCR-015-test-wg-domain-host-sni.md
  - .sdlc/design/DCR-014-test-dns-color-argument.md
  - .sdlc/design/DCR-013-test-dns-observation.md
  - .sdlc/design/DCR-012-test-wg-host-address.md
  - .sdlc/design/DCR-011-test-wg-local-reject.md
  - .sdlc/design/DCR-010-test-wg-udp.md
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
  - .sdlc/design/task008-observation-dns.md
  - .sdlc/design/DCR-003-fixed-core-configuration.md
  - .sdlc/design/DCR-004-runtime-update-failures.md
  - .sdlc/design/DCR-005-traffic-observation.md
  - .sdlc/design/DCR-006-memory-traffic-trend.md
  - .sdlc/design/DCR-007-dual-traffic-windows.md
  - .sdlc/design/DCR-008-test-loopback-metering.md
  - .sdlc/design/DCR-009-controlled-wg-peer.md
approval_refs:
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-013
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-012
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-011
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-010
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-009
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-008
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-007
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-006
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-005
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-004
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-003
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-002
  - .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-001
  - USER:lifei 2026-09-04 TASK-008 Human Task acceptance sha256:d2f866af0f53ad28e8ba1cbe2d9b686b59f5c5f94703a25eb2d55cabaef5bc82
  - .sdlc/evidence/TASK-008/technical-design-review.yaml#EVIDENCE-TASK-008-DCR003-APPROVAL-001
---

# TASK-009：受管运行、失败停止与受控网络验证

## 本次需求

落实 `docs/veyra.md` 第 70、75–78 节及用户明确确认的三阶段失败行为，以 TASK-008 已验收的
完整 ObservationOnly 配置驱动固定 Windows amd64 sing-box 1.14.0。证明真实受管 child、
固定鉴权 API、私有配置、实例身份和后端脱敏观测在正常启动、失败及停止时一致，
并完成 DCR-003 明确要求的 WireGuard、节点 DNS 与 URLTest 受控网络验证。

配置编译、finalize、allowlist/check 失败只记录脱敏日志与 Toast，明确“未应用新配置”，
不停止已经运行的旧 child，不自动重试。已编译配置的 spawn/Ready 失败则停止并清理
失败候选，不自动恢复上一配置；只有退出和清理确认后才显示 Stopped，不能确认时保留
资源所有权与 RecoveryRequired。必要清理不等于配置回滚，手动重试始终使用当前有效状态。

失败日志与 Toast 只接入现有产品 Start/Stop。订阅获取/解析/校验/保存失败时保留旧节点与
最后成功更新时间、允许手动重试的完整订阅入口由 TASK-011 交付，本 Task 不创建订阅页。
用户新指令已授权上述产品语义及必要最小响应调整，新设计已独立审查通过，待 Readiness，
不再次要求用户批准同一决定，也不继承旧回滚候选的审查通过结论。

已有产品入口仍是零参数 Start/Stop；Ready 时重复 Start 返回 AlreadyRunning，不能借此
加入热更新或替换入口。替换及失败停止验证针对内部运行时事务。完整 GUI/Tray 联合验收属于
TASK-010；Tor 仍等待独立资源 Gate，当前不得下载、启动或声称支持。

## Scope

### allow

- CHANGE-013/DCR-015：compiler.rs、Windows managed_sidecar_port.rs及必要managed_sidecar.rs
  的cfg(test)封闭WG域名HTTP/TLS元组、最终资格与真实测试；Go测试端main/protocol/stack/observe、
  新domain.go/domain_test.go及对应既有测试/README新增两个业务模式与固定内存别名198.20.0.255。
  复用有界私有DNS采集，HTTP原Host/204累计ACK、TLS SNI观察及其底层失败锁存；详细协议、
  期限、清理和负例按已批准DCR015。不改依赖/锁、产品或主机，不削减其余SF003验收。
- CHANGE-011/DCR-013：compiler.rs、managed_sidecar.rs、Windows managed_sidecar_port.rs 的
  cfg(test) 封闭DNS预检日志配置、最终字节资格、私有有界stderr采集及真实预检；Go测试端
  main/protocol/stack、新模式对应既有测试及README新增init_dns_probe有界丢包模式。
  精确查询veyra.disign.me，唯一loopback WG/URLTest，无业务转发；采集最多10秒、65536字节。
  普通产品日志、旧peer模式、依赖/锁和主机配置不变；详细元组/生命周期以DCR013为准。
- CHANGE-010/DCR-012：在CHANGE009既有文件范围修订host对照为172.26.192.1，精确绑定
  Default Switch GUID 3D816D4D-97AF-48FA-89DC-EA7945796D10 的TCP/UDP目标各一个，保留两个
  loopback目标。四端口封闭Compiler/Go元组与独立计数；批准非loopback可达面，执行前及每阶段
  只读复核地址/路由，漂移失败且无其它地址fallback。原三阶段/期限/清理和其它验收保持。
- CHANGE-009/DCR-011：compiler.rs 的 cfg(test) 精确阳性规则及最终读回校验、Windows Port
  现有测试模块、scripts/task009-wg-peer 新增封闭三阶段主动探测。两个自有 loopback TCP/UDP
  回显目标跨阳性/受保护/阳性持续绑定；四格仅198.18.0.1与127.0.0.1。阶段40秒、工作135秒、
  helper150秒、父159秒、外层160秒；原场景期限保持。无依赖、锁、产品或系统配置修改。
  其中host127目的和两个目标契约由CHANGE010明确替换；其原运行FAIL证据保留。
- CHANGE-008/DCR-010：compiler.rs 的 cfg(test) 固定 WG UDP 编译元组与校验/内测，Windows Port
  wg_peer_test 内的真实 UDP 客户端、协议与资源核验，以及 scripts/task009-wg-peer 既有模块的
  init_udp/udp 私有协议、内存 UDP 回显/观察器、测试和 README。仅入口127.0.0.1:动态端口，
  固定目标198.18.0.2:18081，保留原 TCP 测试限制、产品空入口和所有权/截止；不改依赖或锁。
- CHANGE-007/DCR-009：`scripts/task009-wg-peer/` 独立 Go 测试端、固定两个直接依赖及工具生成的
  传递依赖清单；既有 Windows Port 内测和必要 cfg(test) 归属读取。只验证真实 WG 握手、TCP
  出站应答、虚拟地址 ICMP 和资源清理；正常 ObservationOnly 编译器、产品依赖和打包不变。
  peer 精确 loopback；DUT 在单用例内可使用已批准的短时全接口 WG 协议 UDP socket。
- `src-tauri/src/singbox/runtime.rs`：内部完整配置的 check/prepare/apply/run/Ready/stop、
  active 候选提交与失败停止；删除仅为自动恢复旧配置存在的分支和无用状态，不添加重试。
- `src-tauri/src/singbox/managed_sidecar.rs`、
  `src-tauri/src/platform/windows/managed_sidecar_port.rs`：候选与运行实例的私有配置归属、
  固定资产 check/run、最终字节读回、pending 消费、child 退出确认和候选清理。
- `src-tauri/src/platform/windows/private_runtime.rs`：仅修复创建后准备与删除双失败时的
  所有权返回及内测；由既有 Port 持有未清理对象，保留原 ACL/身份/reparse 校验和手动 Stop。
- `src-tauri/src/singbox/clash_api.rs`：仅适配固定鉴权 Logs 的正常空摘要及内测：HTTP 204，
  或实测 HTTP 200 且显式 Content-Length: 0、无 Transfer-Encoding、响应体为空。
  其他 200、Traffic 200/204、401、异常与握手超时仍失败。CHANGE-003 另批准 DCR-005 的流量修正：
  核心一秒窗口值直接映射速率，累计量取现有 REST 摘要；删除旧差分状态，补边界与实例隔离内测。
  不改变公开接口、字段单位、固定鉴权/采样限制或依赖。
- `src-tauri/src/singbox/{compiler.rs,mod.rs}`：仅为实例绑定与已检查候选一次消费作必要适配，
  保持应用层现有 Plan → GeneratedConfig 绑定责任，不增加恢复 Plan 缓存；不得改变协议映射、DNS、
  endpoint、RuntimeProfile 或结构白名单语义。
  CHANGE-006/DCR-008仅为cfg(test)新增封闭的本机TCP计量编译入口和白名单；普通产品配置保持空入站。
  Windows Port内测及必要的managed_sidecar.rs cfg(test)归属读取可验证同一child实际listeners与流量。
- `src-tauri/src/application/{managed_observation_runtime.rs,runtime.rs}`：以整体校验后的
  AppState 和精确默认 Pool 接入上述事务、保持串行 controller 与采样身份边界；在已有
  零参数 Start 封闭响应增加 ConfigurationFailed（JSON configurationFailed）。其它输入、
  AlreadyRunning/Busy 语义保持；runtime.rs 只适配内部失败状态并保留 SystemProxy Mock 补偿断言。
- `src-tauri/src/application/observability.rs`：复用既有安全日志摘要记录固定失败阶段，
  保持真实停止/仍运行/recovery 状态，不增加原始错误输出或持久日志。
- `application/observability.rs` 与 `application/runtime.rs` 的流量 DTO 映射断言及必要内部适配：
  仅落实已批准 DCR-005，不改变公开事件、IPC、存储或 UI 字段。
- `src/App.tsx`、`src/styles.css`、`src/lib/{observability.ts,observability.test.ts}`：仅适配
  新封闭响应并为现有 Start/Stop 添加可关闭、自动消失的失败 Toast 及定向验证；不新增依赖。
  CHANGE-004另授权DCR-006的聚合实时网速、本次内核累计和最近60秒趋势图及严格读侧解析。
  CHANGE-005将其扩展为首页10分钟图与左侧栏底部60秒图，参考Clash Verge Rev的首页/侧栏布局；
  只实现现有功能的布局、双图及同一显示时钟，完整其它页面/Tray仍不在本次范围。
- `src-tauri/src/application/observability.rs`、`src-tauri/src/commands.rs`：为DCR-006维护
  内存窗口、生命周期清空、单调相对时间与两个安全读侧字段；DCR-007将上限调整为10分钟/600点，不新增命令或采样。
- `src/lib/traffic-trend.ts`、`src/lib/traffic-trend.test.ts`：如需拆分，仅负责时间窗和SVG
  纯计算及边界验证。上述现有文件内测可作必要字面量适配，不修改存储或内核Profile。
- 上述文件内的定向测试，以及 `src-tauri/tests/task009_managed_runtime.rs`、
  `src-tauri/tests/task009_controlled_network.rs`：仅容纳受管生命周期与 DCR-003 受控网络
  验证。新测试 harness 的资源、地址、所有权及运行方式须先完成实施前提中的核定。
- `.sdlc/evidence/TASK-009/`：仅在实际产生证据后保存绑定目标身份的有界记录；原始日志
  放在忽略的验证输出目录，不把密钥、私有完整配置或原始敏感载荷写入记录。

### deny

- 上述失败反馈之外的 UI、订阅页、Tauri commands/capabilities、GUI/Tray 行为、公共 IPC、
  任意配置/路径/端口/参数输入、额外 Start/Apply/Reload 产品入口；不把重复 Start 改为 replacement。
- Parser/Domain/AppState/Storage/schema/迁移、Provider 替换、第三方依赖与锁文件、构建资源
  目录和固定资产版本；不为验证任意安装 peer 工具或使用未冻结 binary。
  CHANGE-007 仅例外允许上述独立 Go 测试模块的已列固定依赖及正常传递依赖，不适用于产品。
- System Proxy、TUN、UAC、WFP、Service、系统接口/路由/DNS/hosts 文件修改、生产或第三方
  未授权网络目标；不增加产品 listener、mixed inbound、Dashboard、远程 API 或 raw JSON 旁路。
- 降低固定 ACL、TokenUser 身份、无 reparse point、每实例新 secret、check 后字节不可变、
  固定 127.0.0.1:9090 或 child 所有权规则；不通过扫描或终止无关 PID 腾出端口。
- Tor 运行、其它核心版本或架构、所有协议握手通过的泛化结论、完整 GUI/Tray E2E、CI、commit/push
  和发布。协议握手只可按本 Task 实际执行的受控用例逐项声明。
- 自动恢复旧配置、后台重试、Direct/空配置 fallback、为回滚保存旧 secret、编译成功即
  宣称已生效。状态文件损坏恢复、迁移备份和 SystemProxy 安全补偿不在本次删除范围。

## 子功能

### SF-001：真实受管生命周期及配置事务

**需求：** 经完整 AppState 校验、Compile、实例绑定、结构 allowlist 和私有文件的固定核心
check 后，才可运行同一最终字节。启动和停止只认已证明归属的 child，Process/API Ready
成功前不得发布 Ready 或提交未经验证的 active 状态。编译/check/preparation 失败不触碰
上一已验证进程、配置和采样身份。准备成功后才能停止旧 child，确认退出后 run 一次消费
已检查 pending；新 child Ready 成功才提交 active。spawn/Ready 失败清理候选并结束事务，
不调用 restore_previous，不重新 check 或启动旧配置。

**验收：** 冷启动、正常停止、child 崩溃、后续启动清理均符合 DCR-001。实际 TCP listener
只有当前 child 拥有的 127.0.0.1:9090；端口被无关进程占用时封闭失败，不停止占用者。
每个新实例使用独立 secret，运行字节匹配该实例通过 allowlist/check 的最终字节。
spawn/Ready 失败且清理确认后处于 Stopped，无活跃 child、无失效 active/旧 Ready DTO。
Stop 未确认退出或清理失败时保留资源所有权与 RecoveryRequired，不得报告 Stopped；
旧 child 停止未确认时不得运行候选。check 子进程超时后的未退出所有权同样保留，
后续 Start 封闭失败直到手动 Stop 成功；不能因临时值 Drop 丢失未退出的进程。

**验证：** 真实固定资产的 start → Process/API Ready → stop；对 compile、finalize、check、
prepare、旧 child 停止、run、Ready 及清理分别注入失败，断言错误传播、配置槽、
pending、child 身份与文件生命周期。Mock 用于稳定覆盖精确失败点；必须另有真实 Windows
adapter 的成功内部替换及真实失败停止用例，特别覆盖候选 run 消费 pending 后的 Ready
失败，断言清理后没有重新启动 previous。配置/check失败证明旧 child identity 不变，
旧 child 停止失败证明候选未运行。记录每实例最终字节哈希和 child 身份，分别证明停止确认、
secret 不复用及旧 secret 失效，不记录 secret 值。继承 ACL/reparse 拒绝和无关进程不受影响
用例保留；正常 stop、启动失败、child 崩溃和后续启动清理四条路径分别断言私有配置清理。

所有 check/Ready/stop 的期限沿用 DCR-001：check 10s，超时 kill 后退出确认 2s；单次 API
Ready 2s；Stop 退出确认 2s，50ms 轮询；产品 Start response 15s、Stop/Shutdown 3s。
所有异常路径同样有界，不能隐式叠加无界等待或重试。

**implementation_status：** PENDING
**acceptance_status：** PENDING

### SF-002：同一实例的观测、失败日志与 Toast

**需求：** controller 串行持有 child、运行时与 observation bridge。只为当前已鉴权 Ready
实例采样连接、流量和日志摘要；停止、替换、失败或身份切换先使旧身份不可采样，迟到的
旧结果不得进入 Snapshot/Delta。保留容量一队列、Busy 与零参数 Start 的幂等语义。
配置生成失败响应为 ConfigurationFailed，进程准备后启动/Ready或清理等失败为 StartFailed；
StateUnavailable 继续表示输入状态无效。失败摘要只使用固定阶段/文本，不透传原始错误。

**验收：** Ready 重复 Start 不重新编译、不重新启动、不替换 child；并发 Start/Stop/sample
无双实例、无跨实例归属、无停止后的旧 Delta。真实 API 连接/流量/空日志读取只产生既有
安全 DTO；凭证、UUID、私钥、API secret、原始日志、连接目标不进入错误、事件、state.json
或日志。无效鉴权、读取超时、child 退出或 worker 失败必须映射为封闭失败/恢复状态。
一次用户操作最多一条失败 Toast，可关闭并自动消失；后台采样失败只更新状态/日志。
配置生成失败提示“配置生成失败，未应用新配置”，旧 child 健康时持续显示其真实运行状态；
StartFailed 提示“内核启动失败，请查看运行状态”。只有后端确认 Stopped 才显示“服务已停止”，
recovery 显示“停止未完成”；快照未知不能猜测停止，Toast 消失不能把失败改为成功。
Stop 失败按相同原则显示固定失败提示，成功或 AlreadyRunning 不误报失败 Toast。

**验证：** 真实 child 的固定 REST/WS 鉴权与摘要读取；受控正流量证明观测来自当前 child，
并验证正常空日志分支。Mock 可补全恶意载荷、未知日志类别、超时和迟到结果。定向并发
测试覆盖 Start/Start、Start/Stop、sample/Stop、身份替换与 worker response 失败；断言
停止后无新采样、旧结果丢弃、队列有界、相同流至多一条 in-flight、采样间隔至少 1s。
两条固定流沿用 2s 握手/首帧、16 KiB frame/message、单消息读取上限。重复 Start 不替换
是产品行为；内部 replacement 的观测失效测试不能据此增加产品入口。
补充 TS 封闭响应解析与未知响应拒绝测试；实际操作现有 Start/Stop UI，验证每次失败恰有
一条 Toast、关闭与自动消失、后端失败后的快照请求失败不覆盖操作结果、停止/recovery
持续状态、重复 Start 不误报及原始错误不外泄。仅解析单测不能替代 Toast 交互证据，
该局部验证不等于 TASK-010 GUI/Tray 联合验收。

**CHANGE-004 流量展示验收：** 使用 sing-box 聚合统计（包含 Direct），实时网速与本次内核
累计分开展示。CHANGE-005/DCR-007更新为后端只由成功采样追加最近10分钟、最多600点的内存趋势；日志/读取/订阅不造点，
窗口隐藏不丢采样历史。Stop/新Ready/recovery清空，配置失败且旧实例健康时保留；不落盘。
Snapshot/Delta使用DCR-006既有单调时间与安全速率字段，DCR-007严格校验600点/600000ms上限。
首页10分钟图与左下角60秒小图共用数据及时钟，较早点只出现在首页，分别按各自窗口缩放。
两图曲线、可读单位、零值与空态、大于5秒断线、60秒/10分钟淘汰、桌面定位及320px重排、
旧结果拒绝、停止清空两图和无额外IPC均需自动测试与浏览器Mock交互/截图验证。
真实核心正流量和SF-003仍必需，Mock图表验证不能替代它们。

**CHANGE-006 真实计量验证：** 按DCR-008使用唯一127.0.0.1 direct TCP测试入口、固定本次回显目标、
既有Direct路由与每实例新secret。经同一Plan/finalize/check/run取得非零双向网速、精确最终累计、
静默归零且累计稳定；核对最终配置哈希和child实际listener归属。Stop/新实例后不可读旧身份，
旧secret失效且共用图表历史清空。普通产品仍只有API listener，测试构建例外不得推广为产品能力。
单用例60秒、服务30秒、I/O2秒、每实例一条流及收发各2MiB上限，运行前后只读核对主机网络配置。
此项不移除SF-003或真实网络in-flight/晚到结果验收。

**implementation_status：** PENDING
**acceptance_status：** PENDING

### SF-003：WireGuard 与 DNS/URLTest 的受控网络边界

**CHANGE-013 WG域名用例：** 用户明确“允许”DCR015
sha256:00b0455ffe4f997f58856f6112bea4c02b735387c96361a2b6264638d0fbde79。
同一child的新鲜DNS结果与peer解密IPv4目的、HTTP Host/204 ACK或TLS SNI关联；固定内存
别名，不更改DNS/TUN。TLS只观察SNI而非HTTPS成功；零连接取消、迟到失败及真实清理必须
分别诚实记录。两个新用例及共享路径回归均需执行；节点hostname、完整DNS拒绝、非宿主
转发、IPv6业务和下列整体验收不被替代。候选历史PROPOSED字样由身份绑定批准记录覆盖。

**CHANGE-011 DNS结果预检：** 用户明确“批准”DCR013
sha256:3b5bf2627d009844e340277adbeffdf73e47a6638afeeedcd675d405f6877181。
按该封闭测试元组，仅证明固定child的local transport exchange及返回地址；测试日志
白名单、有界排空、配置/进程归属、先DUT后peer清理、新模式不入栈/不转发、旧模式回归
均需验证。此预检不证明实际DNS服务器网包、业务成功、Host/SNI或完整DNS验收；下列
全部必需矩阵保留。现有TUN与Cloudflare记录保持，依据结果再核定业务测试资源。

**CHANGE-010 宿主对照修订：** 用户明确“批准”DCR012
sha256:3f15ace010e05e1cd201cc4747dddd30101af89d15bdfe613251bcf627330968。
host TCP/UDP两格改为已核定的172.26.192.1，虚拟两格保持。同一peer/密钥/四目标原句柄跨
前阳性/保护/后阳性持续持有；每目标按case独立计数，各TCP目标最终2连接、各UDP目标2报文，
总TCP4/UDP4且phase2全生命周期零载荷。原127失败不改写为PASS，不声称验证其Router拒绝。
前后每格实际精确回显、phase2真实探测及目标零增量、ICMP/API健康、最终读回/错元组负例、
地址漂移/错误源/迟到/缺目标负例、四资源Hold及原TCP/ICMP/UDP/55/150秒回归均必需。
该条只覆盖CHANGE009的宿主地址/资源修订；完整DNS、非宿主转发等下列义务仍保留。

**CHANGE-009 拒绝子集：** 用户明确“允许”实施 DCR-011
sha256:54c01cbc8f99b080a2d68e5b999ab6fc9f97fe49b1ae30e53e36ae2d402f6641。
同一 peer/密钥/两个真实目标，三个串行 DUT 各自完成 URLTest bootstrap；前后阳性四格精确回显，
正常编译的中间阶段实际发起四格且目标全生命周期无中间载荷，ICMP及API仍健康。
最终TCP4连接、UDP4报文均须属于阳性阶段；迟到、目标/通道故障不能报拒绝通过。
保持原拒绝与唯一用户Port→Direct规则，阳性精确前缀仅cfg(test)，最终读回同样封闭校验。
验证封闭元组/阶段、资源持有/截止及真实四格，并回归旧TCP/ICMP/UDP/Hold；不删减DNS与非宿主转发验收。

**CHANGE-008 UDP 子集：** 用户已批准 DCR-010 sha256:36a910c90c247e1b46958ed7890dceabc7d769aee1ca1f18b062154de8ea9c79。
仅测试固定 loopback UDP 入口，经唯一 Manual Pool/WG 至内存目标，客户端实际收到三个精确应答，
结合 peer 加解密边界计数、最终配置/进程/socket 身份及先 DUT 后 peer 的清理证明 UDP 出站应答。
沿用 CHANGE-007 资源和 checkpoint012 Hold 修正；不扩展 DNS、完整拒绝或产品入口，不删减下列验收。

**CHANGE-007 首批范围：** 按 DCR-009 实现独立 loopback peer、正常 ObservationOnly 的 TCP
应答和认证 peer 虚拟地址 ICMP Echo 子集；单用例 60 秒、工作窗口 30 秒、每次 I/O 至多 2 秒。
运行前后只读比较宿主配置，记录实际 DUT/peer UDP 所有权，先确认 DUT 停止再释放 peer。
该资源批准不包括 WG UDP 业务入口、完整拒绝路径或系统 DNS 拓扑，不删减下列必需验收。

**需求：** 使用 TASK-008 Compiler 实际生成、按同一实例最终化并 check 的配置，在已核定
的受控 peer/目标拓扑中运行。分别证明 WireGuard 的允许出站应答、拒绝主动入站/转发及
已批准虚拟地址 ICMP 例外，并观察实际 DNS/URLTest 行为。不能改写 JSON 来方便测试。

**验收：** 以下用例全部必需；任何未执行项不能由 check 或源码推断替代：

1. 已认证 WireGuard peer 可触发虚拟地址 ICMP Echo 应答，按已批准例外记录；不能将此
   描述为宿主 TCP/UDP 服务暴露，也不能宣称“所有入站静默”。
2. 该 peer 主动发起到虚拟地址/宿主目的的 TCP、UDP、DNS，以及经 Router 到宿主和
   受控外部目的的转发均被拒绝；置顶 route/DNS reject 不得被用户规则覆盖。
3. WireGuard 由本应用发起的既有出站连接仍能收到应答；分别覆盖 TCP 与 UDP，拒绝
   用“所有通信失败”冒充入站隔离成功。
4. 节点服务器 hostname 由系统 DNS 解析；URLTest 经可转发域名的 HTTP/SOCKS 类成员
   时保持远端目的域名语义，经 WireGuard 等需要 IP 的成员时使用已配置系统解析行为。
   实际请求保留原始 URL 的 Host 与 TLS 身份，不在编译时预解析或把 URL 改成 IP。
5. 运行前后系统接口、路由、DNS、System Proxy 配置无任务引起的变化，无 TUN/UAC/WFP/
   Service 操作。区分产品唯一 TCP API listener 与 WireGuard 必需的协议 UDP socket；
   所有验证资源都只能在受控拓扑内，由 harness 明确拥有并清理。

**验证：** 保存目标配置哈希、固定资源身份、受管 child/peer 身份和逐用例断言的摘要。
用 peer/目标端的受控计数或请求记录证明合法请求成功、拒绝流未到达目标；超时仅能在
同拓扑阳性对照成立时作为拒绝证据。DNS 用系统解析观察和代理端所见 hostname/IP
区分解析位置；URLTest 使用受控 HTTP/TLS 目标的 Host/SNI 或等价客观证据，不触碰用户
现有代理服务或外部测速站。运行前后接口/路由/DNS/代理快照只读比较，不为测试改主机
配置。网络 fixture 不能完整建立时记录 NOT_RUN/UNAVAILABLE 与精确缺口，SF-003 和
Task 独立验收保持未通过，不把这组验证移出本 Task。

**implementation_status：** PENDING
**acceptance_status：** PENDING

## Dependencies、Risk 与实施前提

- TASK-008 已 DONE，三个 SF 与整体验收 PASSED，依赖证据为
  `.sdlc/evidence/TASK-008/acceptance.yaml#EVIDENCE-TASK-008-ACCEPTANCE-001`。
  沿用最终身份 `sha256:d2f866af0f53ad28e8ba1cbe2d9b686b59f5c5f94703a25eb2d55cabaef5bc82`。
  143 个 Rust 测试、34 份最终配置 check 不证明本 Task 的真实运行要求。
- DCR-003 候选文件保留原字节与历史 PROPOSED 标识；其批准由匹配身份的
  `EVIDENCE-TASK-008-DCR003-APPROVAL-001` 建立，不能误判为未批准的 WG/DNS 语义。
- **已确认变更：** `EVIDENCE-TASK-009-CHANGE-001` 记录用户明确授权的三阶段失败语义，
  不再要求用户批准同一决定。DCR-004 `runtime-update-failures` 已按新语义独立审查通过，
  当前冻结身份为 `sha256:6610b072db7f8044f5681140a9241c5da8e89e4ceb80a78d56ae2674ff3542b7`。
  旧回滚候选审查仅作为历史证据，不支持本目标。Orchestrator 回读本 Task 与新设计后
  判断 Readiness；Planning Producer 不设 READY。
- **运行契约：** 保持调用方绑定新 secret 的 GeneratedConfig 与固定最终字节，Windows
  pending 只消费一次，Ready 后才提交 active；删除自动恢复旧配置分支及专用无用状态。
  child 或 check 进程退出未确认、私有配置清理失败时保留所有权，阻止后续启动直到手动
  Stop 成功。无需缓存 Plan 或为回滚重新生成配置，不能放宽 ACL/身份/check 边界。
- **验证设计工作：** DCR-003 的受控 WireGuard peer、受控 HTTP/SOCKS/TLS 目标与
  DNS 可观测拓扑尚未有本 Task 的可执行资源/命令证据。实施前核定可复用的批准资源、
  地址/端口、所有权、权限、超时和清理方式；需要新增二进制/依赖/外部目标/运行组件时
  按 Material dependency/security 边界先决定。缺少现成脚本本身不构成阻塞，可以在既定
  范围内编写 harness；实际运行前必须明确完整拓扑，不得通过删减 SF-003 绕过验证。
- **风险：** 固定端口冲突、pending 被消费、候选/旧配置混用、secret 复用、进程退出未确认、
  ACL/清理失败、采样跨身份传播和网络测试缺阳性对照是主要风险，验收分别覆盖。
  预存 staged/unstaged/untracked 修改需以本 Task baseline 区分；不得吸收或回退他人修改。
- **依赖顺序：** 新设计与 Task Readiness → SF-001 → SF-002/SF-003 集成验证 → Task
  整体验证及独立交付审查。SF-001/002 共用运行时与 Application 文件，实施需串行或由
  Orchestrator 指定不相交所有权；不并行实现 TASK-010。依赖图仅 TASK-008 → TASK-009 →
  TASK-010 → TASK-011，无循环；TASK-010/011 保持 future stub。TASK-011 在后续交付订阅
  手动更新、失败日志/Toast与最后成功检查时间；现有有效缓存的 304 只表示成功检查，
  不表示内容变化或运行配置生效，失败不改变最后成功时间。

## 验证命令与证据要求

命令来源为 `.sdlc/design/foundation.md` 验证命令、TASK-008 的已执行 Cargo 证据，以及
仓库现有真实 child 测试。当前规划只定义命令，不执行它们：

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib
cargo test --manifest-path src-tauri/Cargo.toml --lib fixed_bundle_resources_run_and_stop_only_the_owned_loopback_child -- --nocapture
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
git diff --check
pnpm lint
pnpm test
pnpm build
```

第二条使用仓库已有测试名作为过滤器，须检查真实执行数，不能把零测试运行
当通过。库测试不再继承 TASK-008 对真实 child 测试的 skip；新 integration harness 完成并
核定资源后，使用仓库 Cargo 测试入口运行相应集成目标，记录实际完整命令。现有真实
child 单测只证明它已有的路径，不自动覆盖失败停止、WG 或 DNS/URLTest 新矩阵。pnpm
命令来自现有 package.json，针对本 Task 的封闭响应与 Toast 改动；实际 Toast 交互证据单独记录。

格式检查遇预存文件问题时按交付文件做 scoped rustfmt 检查并分别记录全量结果；库级
Clippy 不能冒充 all-target Clippy。每次长验证必须有外层超时，单用例默认 60s；包含
固定核心 check 矩阵的整批 Cargo 可采用 TASK-008 既有 360s 上限，不能无界阻塞。
实际网络用例需按 harness 的有限生命周期核定超时；child 内部期限仍遵守 DCR-001。

每条 Evidence 绑定当前交付身份、命令/方法、退出码、时间、结果和日志引用；分别记录
Mock、真实 Windows adapter、真实网络、独立审查、人工验收。NOT_RUN/UNAVAILABLE 不算
PASS；不由历史 TASK-006/008 的通过推断当前目标通过。

## Task 独立验收

从隔离且整体校验的 AppState 经真实 Compiler 最终化/check 到同一受管 child 的 Ready、
后端连接/流量/日志摘要及停止，完成一条可重复的真实 Windows 链路；固定资产、最终
字节、私有 ACL、每实例 secret、唯一固定 TCP API 与无捕获/特权副作用均有客观证据。
编译/check/preparation 失败保留旧运行态，日志与 Toast 明确新配置未应用；spawn/Ready
失败后停止并清理失败候选，不自动运行上一配置。只有确认退出和清理完成才为 Stopped，
否则保留所有权与 RecoveryRequired，展示“停止未完成”，不报告新配置已生效。

SF-001 的失败矩阵、SF-002 的身份/并发/失败反馈及 SF-003 全部受控网络用例必须联合通过，
证明“旧进程/新进程”“已验证配置/候选”“受管观测/迟到结果”没有混淆，并且 WireGuard
允许流量与拒绝路径均成立。测试通过后仍需当前目标的独立交付审查与显式人工 Task
验收；子功能均通过不能替代本 Task 整体验收。GUI/Tray 和 Tor 继续由各自后续边界处理。

**acceptance_status：** PENDING

**Readiness：** PASSED；用户已授权新失败语义，Orchestrator 已回读并核对独立审查通过的新设计、
完整 Task 契约及依赖，记录于 `EVIDENCE-TASK-009-READINESS-002`。受控网络拓扑须在实际运行前核定；
Readiness 不是实现或运行验收通过。
