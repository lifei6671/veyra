---
id: TASK-008
milestone_ref: M6
dependencies: [TASK-007]
risk: HIGH
status: VERIFYING
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
  - .sdlc/design/task008-observation-dns.md
  - .sdlc/design/DCR-003-fixed-core-configuration.md
approval_refs:
  - USER:lifei 2026-09-04 DCR-002 scope confirmation
  - USER:lifei 2026-09-04 DCR-002 Human Technical Design Gate sha256:1a02b56d8861bb7c936599d952c822fa99667e8109d6b9b95427cb49e1966989
  - USER:lifei 2026-09-04 TASK-008 non-persistent Domain DNS and runtime test-fixture scope approval
  - USER:lifei 2026-09-04 DCR-003 Human Technical Design approval sha256:3040951342ef465fb177b3abb1a7537d73c55bc87e88e1a7b3ba93677033b26d
---

# TASK-008：完整 sing-box 配置编译与最终字节校验

## 本次需求

在 TASK-007 已交付的 V3 强类型节点、Pool 和 Route 之上，建立内部 `SingBoxPlan`、封闭的
`RuntimeProfile::ObservationOnly` 与最终 `GeneratedConfig` 链路。15 种非 Tor 用户协议按 DCR-003
生成 14 种 outbound 与一种受控用户态 WireGuard endpoint；它们与编译器生成的
`direct`/`block`/`selector`/`urltest`、受控 DNS 与路由必须成为可独立运行的完整 sing-box `1.14.0`
配置；固定运行期 secret/resource 绑定后，白名单校验与 `sing-box check` 均针对同一最终字节。

本次需求沿用 `docs/veyra.md` 的 Domain → RuntimeIntent → Compiler、DNS 与 Build/Apply 边界。
代理节点服务器域名使用系统 DNS；URLTest 目标域名保留固定核心与成员协议的原生解析行为，
保留原始 URL 的 Host/TLS 身份，不在编译期联网解析或改写为 IP。

本 Task 只构建并校验配置；受管 child 的替换、启动、Ready、回滚和真实运行态事务属于 TASK-009。Tor
资源与运行仍为 `BLOCKED`，不能下载、打包、启动或标记为已支持。

## Scope

### allow

- `src-tauri/src/singbox/{compiler.rs,managed_sidecar.rs,mod.rs}`：将现有语义快照替换为完整的内部
  `SingBoxPlan`、`RuntimeProfile::ObservationOnly`、最终 `GeneratedConfig`、固定 secret 绑定后的
  结构白名单与序列化链路；只按 DCR-003 生成 WireGuard endpoint 及置顶 route/DNS 入站拒绝规则，
  保持产物不向 UI/IPC 暴露。
- `src-tauri/src/domain/{state.rs,mod.rs}`：仅新增封闭、非持久化的系统 `DnsPolicy`、必要导出及
  定向测试；不改变 AppState、RuntimeIntent 既有结构、协议模型、序列化字段或迁移。
- `src-tauri/src/singbox/runtime.rs`：仅适配测试中的编译输入和有效 Pool fixture；保留既有
  check/prepare/run/ready/stop/回滚事件断言，不修改生产生命周期代码。
- `src-tauri/src/platform/windows/managed_sidecar_port.rs`：仅调整固定 Windows amd64 sing-box `1.14.0`
  `check` 的消费边界，保证它接收白名单已通过且绑定后的最终字节，不得在 check 后改写候选配置。
- `src-tauri/src/application/{managed_observation_runtime.rs,runtime.rs}`：仅为既有后端调用方显式构造
  ObservationOnly 编译意图，携带整体校验后的 AppState 精确 `default_target`、既有 RuntimeIntent
  与系统 DnsPolicy，并适配已收紧的内部编译接口；不得改变 CaptureMode、进程生命周期或 IPC。
- 上述 Rust 模块的单元/集成测试，以及固定受控 asset 的最终字节 `sing-box check` 验证。

### deny

- `src/` UI、Tauri command/capability、订阅 Parser、除上述非持久化 DNS 类型外的 Domain 修改、
  AppState schema/Storage/迁移、Provider 替换、依赖/lockfile、
  sidecar `run`/`stop`/Ready/回滚事务、SystemProxy、TUN、UAC、WFP、Service 和新监听器。
- `RuntimeProfile::SystemProxy`、`Tun` 或 `NativeProfile` 编译输出；mixed inbound、Dashboard、远程 API、
  文件日志、原始日志导出、任意 endpoint/path/secret 输入、raw JSON 或未知字段透传。
- WireGuard 的系统接口、固定 listen_port、任意 endpoint 类型或字段、用户 tag、运行期网络验证；
  DCR-003 的编译器生成 endpoint 是唯一配置表示例外，不授权真实 UDP 传输或 host 网络改动。
- 将 DNS、bridge、内部/特权 outbound 当作用户节点；Tor binary、用户 torrc/path/data directory 或 Tor
  成功运行证据。

## 子功能

### SF-001：完整强类型 Plan 与全协议节点编译

**需求：** 从已整体校验的 `RuntimeIntent`、精确默认目标与封闭 DnsPolicy 构建内部强类型 `SingBoxPlan`，
为 14 种非 Tor、非 WireGuard 用户协议生成 outbound，为每个 WireGuard 节点生成唯一受控 endpoint，
并只由编译器生成稳定 tag 的 `direct`、`block`、`selector` 与 `urltest`。
Pool、Route 和 `default_target` 的身份与排序保持确定性；`Unconfigured`、无可用 Pool 或任一不可表达
协议/字段组合必须 fail-closed。

WireGuard 保留 `node-<NodeId>` tag，Pool 引用保持不变；`local_addresses` 映射至 `address`，
保留 private_key 与可选 mtu，唯一 peer 的 address/port/public_key/pre_shared_key/reserved 仅来自
既有强类型节点字段，allowed_ips 固定为 `0.0.0.0/0` 与 `::/0`。不改写系统路由或普通 Route target。

**验收：** 对每种支持协议及其 Pool/Route 组合，编译产物都有可解析的完整 Plan 和稳定 tag；显示名变化
不改变语义字节。任何无默认目标、失效 Pool、内部/特权 outbound、Tor 或非法协议选项均不产生候选配置。
WireGuard 不生成 legacy outbound，peer/地址/密钥/MTU 映射准确，非法值拒绝。协议版本不得静默转换；
固定核心不支持的 Snell v3 等字段组合必须显式失败。

**验证：** 15 协议 × Pool/Route 的表驱动编译断言；selector/urltest/direct/block 和 route rule/final
断言；14 outbound 与 WireGuard endpoint 的精确类型/字段/稳定 tag 及 Pool 引用断言；稳定排序、
默认目标精确传递、Pool 为空/禁用/缺失、默认 Direct/Block/Unconfigured、非法协议、非法 WireGuard
参数和 Tor `BLOCKED` 的负例；显式 Snell v4 正例与 v3 拒绝断言。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PENDING

### SF-002：ObservationOnly 最终配置与结构白名单

**需求：** 只为 `RuntimeProfile::ObservationOnly` 将 Plan 最终化为完整 JSON：必须含 `log`、`dns`、
`inbounds`、`outbounds`、`route` 与必要的 `experimental.clash_api`；`inbounds` 显式为空，`route.final`
必须是已启用 Pool，受管 API 只能是持有私有 secret 的 `127.0.0.1:9090`。secret/resource 绑定后执行
严格结构白名单，不允许 check 后再改写字节。

DNS 只由非持久化 Domain 系统策略生成：唯一 `type: local`、tag 为 `dns-system` 的 server，
`dns.final` 和 `route.default_domain_resolver` 均引用该 tag；无 FakeIP、额外代理 DNS、任意地址、
路径或 IPv4/IPv6 偏好强制设置。URLTest 保留原始 URL 和成员引用，不承诺所有目标域名在本机解析。

仅存在 WireGuard 节点时增加顶层 `endpoints`；仅允许编译器生成的 WireGuard 项，固定 `system: false`，
不输出 name、listen_port、detour、namespace、bind_interface、routing mark、UDP NAT 或其它系统选项。
所有生成 endpoint tags 必须完整、稳定地出现在 `route.rules` 和 `dns.rules` 的首项 `inbound` 集合，
两项 action 均为 `reject`，位于用户规则之前且不可被覆盖；没有 WireGuard 时不生成这些 endpoint 入口规则。

**验收：** `GeneratedConfig` 是可独立解析的最终配置，而非 `outbounds + route.rules` 片段；它只有批准的
顶层/嵌套结构、一个固定 loopback Clash API，且不含 Dashboard、远程 API、文件日志、危险键或 raw JSON
旁路。未配置目标、非 Pool 默认目标、额外 listener 或不在白名单的结构一律拒绝。
WireGuard 的顶层例外、字段及两项置顶拒绝规则都必须通过严格结构验证；缺失、移位、漏 tag、
错误 action 或未知字段一律拒绝。该控制关闭经 Router 的主动 TCP/UDP/DNS 与转发路径；
用户态虚拟地址可能直接响应已认证 peer 的 ICMP Echo，不承诺全协议入站静默。

**验证：** 完整顶层键、空 inbounds、route.final、唯一 API 地址/secret、危险键/额外 listener/未知字段
拒绝测试；绑定前后字节身份和 check 后不可变断言；错误与调试输出不含 secret 或节点凭证。
补充唯一 local DNS、final/resolver 一致性、hostname 节点与 URLTest 原始 URL/成员引用断言；
有/无 WireGuard 的顶层键集合、system=false、无 listen_port/系统选项、唯一 peer、完整 endpoint tag
集合及置顶 route/DNS reject 的正负例；AppState 序列化字段保持不变。确定性断言使用相同 secret 绑定。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PENDING

### SF-003：固定核心的最终字节 check 边界

**需求：** 通过现有受控 Windows amd64 sing-box `1.14.0` 资源对最终 `GeneratedConfig` 执行 `check`，
并使现有后端调用方只提交已最终化的候选。编译、绑定、allowlist 或 check 任一步失败都不得启动、替换或
修改当前受管实例；本 Task 不调用 `run`、不进行 Ready 或回滚。

**验收：** check 接收的字节就是通过结构检查并持有固定运行期绑定的同一份最终 JSON；对每种支持协议的
最终配置均能被固定核心接受。任何 check 失败或后处理改写尝试都保持既有 active/previous 配置槽不变。

**验证：** `Subscription → Parser → Domain → Pool → Route → RuntimeIntent → Compiler → check` 的受控集成
路径；最终字节哈希/读回断言、固定核心 version/check 调用断言，以及不调用 run/Ready/stop 的失败路径测试。
15 协议分别覆盖完整最终配置（14 outbound、一个用户态 WireGuard endpoint）；保留已批准的 runtime
测试 fixture 适配及全部原事务断言。手工预探测配置不能代替 Compiler 生成的最终字节证据，check
不能证明握手、数据通信、DNS 解析位置、URLTest 延迟、入站隔离或进程生命周期。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PENDING

## 实施前提

TASK-007 已独立验收为 DONE，DCR-002 的 RuntimeProfile、固定 API、最终字节 check 与 Tor 资源边界已完成
独立技术设计审查和 Human Technical Design Gate。DCR-003 的 WireGuard 表示、入站控制及 ICMP 例外、
URLTest 原生 DNS 语义已获明确 Human 批准；其与此前非持久化 Domain DNS/测试 fixture 的批准共同约束本 Task。
由 Orchestrator 对账已接受设计并通过 Readiness 后开始实现。
现有 `with_fixed_clash_api` 的后端边界必须收敛为同一
最终化顺序：绑定 secret/resource → 结构 allowlist → 最终字节 check；若无法在既定私有 ACL/固定资产边界
内证明该顺序，不得用片段 check 替代，而要将对应验证记为 `BLOCKED` 或 `NOT_RUN` 并停止在 TASK-008。

## Task 独立验收

受支持的 15 种非 Tor 用户节点、Pool 与 Route 能在 ObservationOnly 受管 Profile 下生成完整、确定且
可由固定 sing-box `1.14.0` check 的最终 `GeneratedConfig`，其中 14 协议为 outbound，WireGuard 为
封闭用户态 endpoint。默认 Pool 精确来自已校验状态，节点服务器域名明确使用系统 DNS，URLTest
保留固定核心及成员协议的原生解析语义。产物只含受批准结构和一个固定私有
loopback Clash API，Build/Apply 保持分离；不包含 raw JSON 透传、额外 listener、内部/特权节点、
SystemProxy/TUN/特权路径或 Tor 运行支持。

WireGuard 必须具有不可覆盖的置顶 route/DNS 入站拒绝规则；允许其虚拟地址 ICMP Echo 应答这一
已批准例外，不以空 inbounds 宣称所有入站路径不存在。协议本身需要 UDP socket，当前 Task 不启动它。
TASK-009 必须在受控 peer 下验证虚拟地址 ICMP 应答、宿主/外部目的转发拒绝、TCP/UDP/DNS 拒绝、
既有出站应答和无系统接口/路由变化，并验证实际 DNS/URLTest 行为；这些不由本 Task 的 check 代替。

**验证：** 目标 Rust 测试、格式化、warnings-denied library Clippy、最终字节的固定核心 check、
`git diff --check` 与独立交付审查。真实 child 启动/替换/回滚、CI、commit、push 和 Tor 运行验收分别
按 TASK-009 或单独资源 Gate 记录，不记为本 Task PASS。

**acceptance_status：** PENDING
