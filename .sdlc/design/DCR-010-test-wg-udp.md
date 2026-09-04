# DCR-010：WireGuard UDP 出站应答的测试入口

状态：PROPOSED，等待独立设计审阅和本项新增边界的用户批准；不是运行证据。
TASK-009 / SF-003；Requirement `docs/veyra.md`
`sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`。
基线为 checkpoint012 `sha256:1827c6b72fe6466836eb4e6d4fd32e006a36e581d07c8731ed6757c59b17fc59`。
沿用 CHANGE-007 的既有依赖、用户态 peer、短时 DUT WG 协议 UDP 绑定和 DCR-009 生命周期。
本候选仅例外扩展 DCR-008/009 中测试配置的 UDP 业务入口限制；其它冻结契约不变。

## 需要批准的增量

新增一个仅 `cfg(test)` 使用的 `127.0.0.1:动态端口` UDP 业务入口，固定转发到本次
WG peer 内存栈的 `198.18.0.2:18081`，证明 DUT 发起的 UDP 请求能经 WG 收到应答。
同时扩展既有 Rust/Go 私有测试协议和回显服务。无新增依赖、锁文件、产品入口或主机配置。
先审阅此具体设计，再请求用户批准新增入口及其测试实现；“继续下一步”不视为扩大原批准范围。

```text
Rust 本机 UDP 客户端
  → DUT direct UDP 127.0.0.1:动态入口
  → Router → 唯一 Manual Pool → WG 198.18.0.1
  → 既有本机 WG 协议 socket → peer WG 198.18.0.2
  → 内存 UDP 198.18.0.2:18081 → 原路返回同一 Rust 客户端
```

产品 ObservationOnly 仍无业务入站。本用例为测试配置，不能称作产品 UDP 入口支持，
也不证明主动入站/转发拒绝、IPv6、系统 DNS、URLTest 或整个 TASK-009 通过。
这些原有验收义务仍保留。单纯 peer 自测或发送成功不能代替 DUT 应答证据。

## 编译与资源约束

1. 在 `compiler.rs` 增加 `cfg(test)` 内部 `compile_wireguard_udp`，输入为既有
   RuntimeIntent/default_target/DnsPolicy 和一个 NonZeroU16 入口端口；返回 SingBoxPlan。
   正常 compile ObservationOnly 后建立强类型测试入站；不接收地址、目标端口、任意 JSON
   或 network 参数。沿用每实例新 secret、finalize、严格 validate、最终字节读回、check/run。
2. 唯一入站固定 `type=direct, tag=test-wg-udp, listen=127.0.0.1, network=udp,
   override_address=198.18.0.2, override_port=18081`；入口端口非 9090，且不同于 peer
   协议端口。私有 cfg(test) 入站模型可改为中性名称，按固定 tag 校验两种完整封闭元组。
   保留 DCR-008 原 TCP/Direct 元组的全部限制，禁止混用两个元组、增加第二入口或放宽地址。
3. UDP 元组必须恰有一个 WG endpoint（server=127.0.0.1、peer 实际端口、唯一 peer、
   local_address=198.18.0.1/32、MTU=1280）和一个 Manual Pool，成员/选中项只能是它。
   default_target 只能是此 Pool；除固定 Direct/Block 终端外无其它节点或 Pool，无 URLTest、
   hostname、用户路由。route/DNS 置顶 endpoint reject、系统 DNS 类型及其它现有白名单不变。
   最终校验也必须检查此完整拓扑，不能仅信任构造函数。普通 compile 在 test 和产品中都为空入口。
4. 先启动并持有 peer 的 OS UDP4 `127.0.0.1:0`，再预留不同的入口端口；入口预留句柄
   只在 DUT 启动前释放。Rust 客户端 bind loopback:0 并 connect 到该入口，保持唯一 socket。
   端口争用直接失败，不重试、不终止无关进程。Ready 后按私有受管 child PID 核定 socket：
   TCP 仅鉴权 API `127.0.0.1:9090`；业务 UDP 仅本次入口；另允许 CHANGE-007 已批准的
   WG 协议 wildcard IPv4/IPv6 同端口组。不能将额外 UDP listener 当作 WG 组自动放行。
5. Go 只增加内存 UDP listener，无 OS TCP/DNS listener。沿用用户态 WG、MTU、密钥生成、
   私有管道、队列/流量上限和绑定所有权规则。不改路由、接口、代理、DNS、hosts、权限或防火墙。

## 私有协议与应答证据

沿用 DCR-009 v1 帧限制、随机 key/run_id/token、严格未知字段和顺序拒绝、错误摘要及截止。
新增首帧操作 `init_udp`，字段与 `init` 完全相同，仅选择本次封闭测试场景；不增加任意目标输入。
原 `init → ready → tcp → probe_icmp → icmp` 场景保持；新场景正常顺序为
`init_udp → ready → udp → shutdown(dut_stopped:true) → stopped`。
UDP 场景拒绝 probe_icmp、再次初始化及 tcp/icmp 事件；原场景拒绝 udp 事件。
ready 的既有内存 TCP/UDP/ICMP 自测继续保留，但自测计数不得混入真实 WG 计数。

新增单个成功事件（共同 v/run_id 同旧协议）：

```text
{v:1, event:"udp", run_id, received:3, replied:3,
 sequences:[1,2,3], rx_udp_packets:3, tx_udp_packets:3,
 payloads_valid:true, addresses_valid:true, authenticated:true}
```

每个业务载荷固定 20 字节：本次 token 解码的 16 字节，加 big-endian u32 序号 1、2、3。
Rust 逐个发送，每次须先收到完全相等的应答才发下一个；无自动重发。
Go 内存服务只回显三个长度、token、顺序均正确的数据报；来源地址必须为 DUT 虚拟地址，
首次合法请求确定源端口，此后同一四元组。非法/多余数据报进入 failed/Hold，不回显任意数据。
只有 gVisor 实际收到三个请求、成功提交三个相同应答，并且 WG tun 边界观察到对应三个
已解密请求及三个送往加密路径的应答后才发送 udp 摘要。观察器匹配 IPv4 地址、UDP 端口、
长度和精确载荷；拒绝分片/截短或错误报文，不把 WG 握手和其它流量计作业务。
IPv4 UDP checksum 为零是协议允许值；非零值须验证，IPv4 头也验证；复用现有栈校验能力。
观察方向可因并发先后不同，不以摘要到达顺序推断客户端消费。

成功必须同时满足：Rust 同一 connected UDP socket 收到三个精确应答 + Go 上述边界摘要 +
严格配置/实际 socket 归属 + 正常清理。Go 的 replied 只代表提交，最终交付证据由 Rust 接收给出。
本用例只有 60 字节请求及 60 字节应答载荷；沿用 DCR-009 的 1MiB 硬上限。
不以总流量统计反推 UDP 完成，不增加 ACK 协议、重传层、后台组件或持久数据。

## 期限、失败与清理

沿用 DCR-009：单次 I/O 最多 2 秒且受绝对工作截止约束，工作窗口 30 秒，父进程最迟
45 秒转清理，Go 55 秒硬截止，父管道/最终等待 59 秒，用例外层 60 秒。仍使用现有 API
测试互斥锁。失败取消业务但 peer 保持 WG 绑定和内存 listener 至 DUT 退出已确认；
父先关闭客户端，再停止并确认 DUT/pending，才发送 shutdown、确认 peer 退出和归属资源清理。
复用 checkpoint012 的 `finish(false)`：未确认 DUT 退出不发 shutdown、不 kill peer、不提前
关 stdin，只等 helper 硬退出且本次仍 FAIL。不得提前释放目标，也不删除未确认归属的私有资源。
记录配置与 helper SHA、DUT/peer PID、固定摘要、socket 和清理结果；不输出私钥、完整配置、
管道首帧或原始载荷。运行前后只读对比现有主机网络/代理配置摘要。

短时入口可被本机其它进程访问，但只路由至本次无副作用内存回显目标，且随机 token 限制应答。
它可能造成测试失败，不能成为任意目标代理。现有固定目标、短期限和所有权已限制风险；
不为测试添加系统防火墙、ACL 服务或额外认证协议。端口预留释放争用和真实 UDP 丢包均如实 FAIL。

## 实施、验证及退出条件

- Rust：`src-tauri/src/singbox/compiler.rs` 的 cfg(test) 入口/模型校验与测试；
  `src-tauri/src/platform/windows/managed_sidecar_port.rs` 的现有 wg_peer_test 私有模块。
- Go：`scripts/task009-wg-peer/` 既有协议、内存栈、观察器、生命周期测试和 README；
  不修改 go.mod/go.sum，不新增工具或依赖。实施可以划分 Rust/Go 两个独立责任区。
- Orchestrator 在明确批准后才同步 Task/CHANGE/Readiness；重新冻结交付身份，串行运行真实核心，
  独立审阅完整增量。当前候选不改变 state、Task 验收或其它冻结设计文件。
- Compiler 定向测试：合法 UDP 元组；旧 TCP 元组的 UDP/WG 负例仍拒绝；普通编译无入口；
  错误地址/端口/网络/tag、双入口、额外节点/Pool/路由、非唯一 WG、reject 删除或顺序错误、
  额外字段和最终字节篡改均失败；非 test 库编译验证不含业务入口例外。
- Go 定向测试：UDP 三次内存回显；错误 token/顺序/四元组/截短报文、只有请求/只有提交却无
  边界应答不得发成功；新旧场景消息交叉、重复/未知字段和协议限长；继续满足 Hold/硬截止。
- Rust 真实固定 sing-box 1.14.0：同一最终字节 check/run、实际 WG 三次 UDP 应答、全部 socket
  归属、DUT 先退出及清理、主机配置不变。原 TCP/ICMP 实测与 finish(false) 定向回归保持。
- 使用现有 README 的 Go test/vet/gofmt/build/module verify 及仓库 Cargo test/clippy/fmt、
  git diff --check；测试/构建遵守已有有界 runner 超时。每个结果记录当前源码和 binary 身份。
  设计审查通过不替代真实运行；任何缺失证据记 NOT_RUN/UNAVAILABLE，不能提高 Task/Gate 状态。

最小实现仍需测试编译元组、新协议场景和内存 UDP 观察。直接复用旧 TCP 入口不能传 UDP；
只做 peer 自测不能经过 DUT；增加产品通用入口会扩大范围。因此采用此测试专用固定入口。

可行性依据：固定 [sing-box v1.14.0 direct inbound 源码](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/direct/inbound.go)
的 UDP override 和 RoutePacketConnectionEx/NAT 回写路径。源码依据只说明方案可实施，
不代替固定 EXE 的实际验证。
