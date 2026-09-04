---
id: DCR-003
status: PROPOSED
change_source: TASK-008 fixed-core compatibility checks and independent DNS design review
affected_design:
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
  - .sdlc/design/task008-observation-dns.md
affected_task:
  - TASK-008
  - TASK-009
---

# DCR-003：固定核心的 WireGuard 表示与 DNS 解析边界

## 问题与证据

TASK-008 使用已冻结的 Windows amd64 sing-box 1.14.0。EXE、DLL 与 LICENSE 的 SHA-256
均与 DCR-001 的资源常量匹配，`version` 为 1.14.0，revision 为
`0b8995879f29a9b98ee027bc17b75e101445b238`。

- `block` outbound 的最小配置 `check` 退出 0，无需改写为其它产品语义。
- `wireguard` outbound 的 type-only 配置 `check` 退出 1，明确报告该 outbound 已在 1.13
  移除，要求 WireGuard endpoint；更换旧协议字段不能解决。
- 同一固定核心接受用户态 WireGuard endpoint、Pool selector 及明确系统 DNS 的配置，
  `check` 退出 0。加入 endpoint 入站拒绝规则后仍退出 0。
- 固定版本 `common/urltest/urltest.go` 将 URL 的原始 hostname 直接传给成员 outbound
  的 `DialContext`，不是先通过 Route DNS 解析。`route.default_domain_resolver` 对普通
  代理出站处理代理服务器的地址，不保证 HTTP/SOCKS 等协议的探测目的域名在本机解析。

因此 DCR-002 的“全部节点均为 outbound”及六项顶层白名单无法表达 WireGuard；之前
DNS 提案对 URLTest 目标域名全部采用系统解析的承诺也不能仅通过现有配置兑现。

## 提议一：封闭的用户态 WireGuard endpoint

- Domain 仍使用现有 WireGuard `ProxyNode`、NodeId 与 ProtocolOptions；不修改订阅格式、
  AppState schema 或持久化字段。Compiler 将该节点唯一映射至 `endpoints` 内的一个
  `type: wireguard`，tag 仍为 `node-<NodeId>`。其余 14 种协议继续生成 outbound。
- 仅当存在 WireGuard 节点时允许新增顶层 `endpoints`。数组中只能包含上述编译器构建的
  WireGuard 项，不允许原始 JSON、未知 endpoint 类型、任意字段或用户提供 tag。
- 固定 `system: false`；不输出 `name`、`listen_port`、detour、namespace、bind_interface、
  routing mark、UDP NAT 调整或其它系统接口选项。使用固定资产已包含的用户态网络栈，
  不引入驱动、外部资源、依赖、TUN、UAC 或系统网络配置变更。
- 字段映射：`local_addresses -> address`，`private_key -> private_key`，保留可选 mtu；
  唯一 peer 的 address/port 来自节点 server/port，public_key、pre_shared_key、reserved
  来自对应强类型字段。peer.allowed_ips 固定为 `0.0.0.0/0` 与 `::/0`，仅作为该虚拟
  出口内的目的地址匹配，不写入操作系统路由。密钥、地址或 MTU 不合法则拒绝候选。
- Pool 成员继续引用同一稳定 NodeId tag，不暴露 endpoint 术语到 UI，不改变选择或默认
  Pool 语义。普通 Route target 仍仅限 Pool、Direct、Block。

### 入站边界与最小控制

WireGuard endpoint 同时具备入站和出站能力。固定版本源码的 `NewConnectionEx` 与
`NewPacketConnectionEx` 会把来自已认证 peer 的新流交给 Router，并可能把 endpoint
本地虚拟地址映射到 loopback；`NewDNSPacket` 会进入 DNS Router。只有空 `inbounds`
不能证明这些入口不存在。

因此编译器必须在所有用户规则之前生成两项封闭的拒绝规则：

- route.rules 首项：`inbound` 为所有生成的 WireGuard endpoint tags，`action: reject`；
- dns.rules 首项：同一 `inbound` 集合，`action: reject`。

这些固定规则不接受用户编辑、不能被用户规则覆盖，也不影响既有出站连接的应答包。
它们关闭的是已认证 peer 主动建立的 TCP/UDP、DNS 以及经过 Router 的转发路径；
不新增监听服务、后台组件或持久状态。WireGuard 的用户态虚拟地址前缀仍可能直接响应
peer 的 ICMP Echo：固定版本 `JudgeFlow` 对本地虚拟前缀直接接受，用户态网络栈生成
Echo Reply，这条路径不经过上述拒绝规则。本提案拟允许这个虚拟地址 ICMP 行为，
不承诺全协议入站静默，也不把该行为表述为宿主 TCP/UDP 服务暴露。

使用 WireGuard 必然需要协议传输的 UDP socket，这不是“完全没有网络 socket”的承诺。
TASK-008 只 check 配置；TASK-009 必须以受控 peer 分别验证虚拟地址 ICMP 应答、宿主
及外部目的的转发拒绝、TCP/UDP/DNS 拒绝、既有出站连接应答，以及无系统接口与路由
变化；成功前不得宣称真实 WireGuard 运行通过。上述残余行为随本 DCR 一并请求 Owner
批准；本次不引入内核补丁或额外运行组件来改变 ICMP 行为。

## 提议二：DNS 的精确适用范围

保留已批准的非持久化 Domain DNS 类型、唯一系统解析器、不启用 FakeIP 或额外代理
DNS。补充明确区分两种地址：

- 代理节点服务器域名：由本机系统解析器解析，固定引用 `dns-system`。
- URLTest 目标域名：保留固定核心和成员协议的原生行为。可转发域名的代理协议允许
  远端解析；WireGuard 等需要本地 IP 的实现使用已配置的系统 DNS。保留原始 URL 的
  Host 与 TLS 身份，编译期不联网解析、不把 URL 改写为 IP。

这是对前次提案“所有 probe hostnames 使用操作系统解析器”的明确修订，必须获得
Owner 决定；此前回复“批准”不被解释为已批准此差异。该修订避免在当前任务引入
自研探测器、内核补丁或新的运行期预解析与缓存组件。

## 验证与边界

- 在相同绑定 secret 下，验证稳定排序、NodeId/Pool/default 身份与完整配置字节。
- WireGuard 验证精确字段映射、system=false、无固定 listen_port/系统接口选项、唯一
  peer、严格 endpoint 类型白名单及置顶 route/DNS 入站拒绝规则；危险字段与未知键拒绝。
- 15 协议逐项执行固定核心最终字节 check。check 仅证明配置被核心接受，不证明握手、
  数据通信、DNS 解析位置、入站隔离、URLTest 延迟或进程生命周期。
- DNS 验证分别覆盖节点解析器引用与 URLTest 原始 URL/成员引用；禁止用 check 证明
  探测目的域名在本地解析。真实行为在 TASK-009 的受控网络用例中验证。
- 不弱化结构 allowlist、最终字节绑定、私有 ACL、API secret 或 Build/Apply 分离；
  真实 run/stop/Ready/回滚仍由 TASK-009 单独交付。

## 影响与批准后动作

仅修改 DCR-002 中有关 WireGuard 配置表示、顶层 endpoints 例外与 DNS 精确语义的
设计；同步 TASK-008 的对应 Scope/Acceptance/Verification 及 DNS 小节。用户已批准的
Domain 非持久化 DNS 类型与 runtime 测试 fixture 范围继续有效。

设计审查通过并获本 DCR 的明确批准后，更新相关 canonical artifacts、重算设计身份、
重新执行 Task Readiness，再开始 TASK-008 实现。现有 TASK-007 的领域/订阅验收保持
其原始范围；TASK-008 当前尚无实现或交付审查可继承。

## 一手参考

- [固定版本 WireGuard endpoint](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/wireguard/endpoint.go)
- [固定用户态网络栈 ICMP](https://raw.githubusercontent.com/SagerNet/sing-tun/v0.9.0-beta.4/stack_gvisor_icmp.go)
- [固定版本 URLTest](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/common/urltest/urltest.go)
- [WireGuard endpoint 配置](https://sing-box.sagernet.org/configuration/endpoint/wireguard/)
- [域名解析器适用范围](https://sing-box.sagernet.org/configuration/shared/dial/#domain_resolver)

本文件为待批准候选，不授权业务代码、真实网络运行或后续 Task 的提前实施。
