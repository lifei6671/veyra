# DCR-016：WireGuard IPv6 TCP/UDP 出站应答验证

状态：PROPOSED；待独立技术审阅及 Owner 对本文件精确 Scope/测试资源的批准。
TASK-009 / SF-003；模式 remediation。设计不代表实现、网络验证或 Gate 通过。
Requirement：`docs/veyra.md`，
`sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`。
当前 Task：`sha256:4d5bcb1bd3d6eba729f1db041fefe6a75f2b26fbef7f58507c5881f95aae3743`。
实现基线 checkpoint025：
`sha256:787deafa226ed2674e7e7a0f78927ee83df380df73e1e664351471d99431c664`；
当前 HEAD：`506553d2f26421a23cc15c3c770da3ab346fb0f9`。

## Problem 与冻结边界

SF-003 要求 WireGuard 已有出站连接分别用 TCP、UDP 收到应答。checkpoint025 已完成
IPv4 TCP/UDP、域名 HTTP Host/TLS SNI 等已批准子集，但 IPv6 业务仍未验证。
DCR-015 明确不创建 IPv6 NIC；当前 Go helper 只安装 IPv4 协议、地址与路由，所有实际
业务包也只由 IPv4 parser/observer 接受。因此不能从既有 IPv4 PASS、AAAA DNS 结果、
固定核心 `check` 或源码推断 IPv6 业务通过。

本候选只新增两个彼此独立的真实 IPv6 阳性用例：TCP HEAD/HTTP 204 完整累计 ACK，
以及 UDP 三个精确应答。复用现有 loopback WireGuard 外层传输、固定 sing-box 1.14.0、
Go 用户态栈、私有父子协议和清理链。内层地址固定为 DUT `fc00::1/128`、peer
`fc00::fe/128`，只存在于进程内存；不创建 Windows IPv6 地址、路由、TUN、DNS/hosts、
System Proxy、防火墙、UAC、WFP、Service 或外部目标。

这是 SF-003 既有 Acceptance 的补证设计，不修改 Requirement、产品语义或既有验收。
它也不替代 IPv6 域名解析/Host/SNI、IPv6 主动入站或 Router/DNS reject、节点 hostname、
完整 DNS 拒绝、受控非宿主转发及整 Task/Human 验收。HTTP025 首次 `DnsError` 和
reject025 首次 5 秒 socket 查询超时继续保留为未闭合 FAIL；后续确认 PASS 及本候选
均不构成其根因修复或稳定性证明。

## 两个独立封闭拓扑

两个用例必须串行运行；每个用例新建 helper、DUT child、run_id、token、两组 WG 密钥和
API secret，不复用另一个用例的配置、端口或业务结果。

### TCP IPv6

```text
sing-box URLTest HEAD
  -> 普通 ObservationOnly / 唯一 URLTest Pool / 唯一 WG endpoint
  -> DUT 用户态地址 fc00::1/128
  -> DUT 自有 WG 协议 UDP socket
  -> helper 自有 127.0.0.1:动态 UDP4 socket
  -> peer IPv6-only 内存 NIC fc00::fe/128
  -> 内存 TCP [fc00::fe]:18080
  -> 固定 HTTP 204 响应，原路返回并由 DUT 累计 ACK
```

URL 精确为
`http://[fc00::fe]:18080/task009-wg-ipv6?token=<本轮32位小写hex token>`；使用普通
Compiler 路径，不增加 TCP test inbound 或日志资格。请求必须是唯一 HEAD，
Request-URI 与本轮 token 精确匹配，Host 精确为 `[fc00::fe]:18080`，无 body、
Transfer-Encoding 或头后多余字节；完整头最多 16384 字节。响应复用现有固定字节：

```text
HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n
```

服务最多接受一个连接。第二连接、第二请求、错误 Host/路径/方法、额外字节或连接异常
均锁存失败。服务端 Write 成功不等于应答完成；只有 peer 的 IPv6/TCP 出方向完整覆盖
上述响应字节，且同一四元组上 DUT 的有效累计 ACK 覆盖响应末字节，才报告成功。

### UDP IPv6

```text
Rust 唯一 connected UDP4 客户端
  -> DUT cfg(test) direct UDP 127.0.0.1:动态入口
  -> override_address=fc00::fe / override_port=18081
  -> 唯一 Manual Pool / 唯一 WG endpoint / fc00::1/128
  -> 同一 loopback WG 外层传输
  -> peer IPv6-only 内存 UDP [fc00::fe]:18081
  -> 三个精确回显，原路返回同一 Rust socket
```

入口端口由 Rust 先用自有 UDP socket 预留，必须非零、非 9090、非 peer 协议端口；只在
最终配置已生成且即将启动 DUT 时释放。争用直接 FAIL，不重选端口、不终止其它进程。
每个载荷固定 20 字节：16 字节 token + 大端 `u32` 序号 1、2、3。Rust 每次只发送一次，
收到完全相等的应答后才发下一包；无应用重试。Go 首个合法请求固定 DUT 源端口，随后
只接受同一 IPv6/UDP 四元组、长度、token 和严格顺序。多余、乱序、错误或截短数据报
不应答并锁存失败。

成功同时要求：Rust 同一 connected socket 收到三个精确应答；Go 在 WG 解密/待加密边界
分别观察恰好三个请求和三个应答；最终配置、child、入口和 WG socket 归属成立；完整清理。
Go 的“已提交应答”不能替代 Rust 的实际接收。

## Compiler 与最终字节资格

两个配置都由 Parser → normalize → AppState/RuntimeIntent → Compiler → per-instance
secret/finalize → fixed `check`/`run` 生成，禁止手改 JSON。

- endpoint 恰为一个，`system:false`、无 `listen_port`，local address 仅
  `fc00::1/128`，MTU 1280；peer server 固定 `127.0.0.1:<本轮helper端口>`，唯一 peer、
  无 PSK/reserved。endpoint 内的既有 allowed IPs 保持 `0.0.0.0/0`、`::/0`；这只是
  用户态出口匹配，不写 OS route。
- 两用例均保持唯一 `dns-system`、route/default resolver、首条 endpoint route reject 和
  首条 DNS reject。测试不删除、后移或例外放宽拒绝规则。
- TCP 使用普通 ObservationOnly 编译。唯一 URLTest Pool 只含该 WG 成员，interval=300s、
  tolerance=50ms；最终读回必须逐字段核对精确 IPv6 URL、endpoint、Pool、空 inbounds、
  reject 顺序、API 与其余标准白名单。IPv6 字面量使本用例不依赖 DNS；不启用日志采集，
  不声明无关 DNS 行为已通过。
- UDP 新增一个 `cfg(test)` 私有 `compile_wireguard_ipv6_udp`，唯一可变输入为现有
  `NonZeroU16` 入口端口；它只接受无 user route 的单 WG 节点、唯一 Manual Pool 和
  `fc00::1/128`。唯一入站固定为
  `type=direct, tag=test-wg-ipv6-udp, listen=127.0.0.1, network=udp,
  override_address=fc00::fe, override_port=18081`。最终 `Document::validate`/readback 必须
  完整识别该元组后再执行标准校验；不能只凭 tag 或调用布尔值取得例外。
- 普通产品 compile 在 test/non-test 均不产生上述 UDP inlet。既有 `test-wg-udp`、
  `test-metering`、domain/DNS/local-positive 资格及旧最终字节语义保持封闭；新旧测试
  元组不能组合。

## IPv6 内存栈、包边界和校验

ready 前的现有 TCP/UDP/ICMP selftest 仍是 IPv4 自测，只证明旧栈；不得作为 IPv6 Evidence。
实际新模式创建 IPv6-only peer NIC：gVisor IPv6 network protocol、TCP/UDP transport、
`fc00::fe/128` protocol address 和 IPv6 default route。该实例不注册 198.18/198.20 地址或
IPv4 route；WireGuard peer IPC 的 allowed IP 仅为 `fc00::1/128`。外层 WG 仍仅由 helper
标准库 UDP4 精确绑定 `127.0.0.1:0`，不能把外层 UDP 与内层 IPv6 UDP 混为同一层证据。

`memoryTun` 为每个实例固定一个 network protocol number；Write 在复制前检查 IP 版本与
长度，并只用该实例协议号 InjectInbound。旧构造器固定 IPv4，新构造器固定 IPv6；禁止
一个模式按来包动态切换协议或回退到另一地址族。

新 IPv6 observer 独立解析新模式，不重构或放宽现有 IPv4 `parsePacket/observer`：

- 单包 1..1280 字节；IPv6 基础头必须完整，Version=6，Payload Length 精确等于
  `len(raw)-40` 且不是 jumbogram；Next Header 必须直接为 TCP(6) 或 UDP(17)。
- 本候选不解析 extension header，因而 Hop-by-Hop、Routing、Fragment、AH/ESP、未知
  Next Header、截短或尾随字节全部失败；不重组分片。
- 源/目的必须按方向精确为 `fc00::1`/`fc00::fe`；每个场景只接受自己的 transport、
  端口和唯一业务流。每方向最多 1024 包、合计最多 1 MiB。
- TCP header/data offset、长度与 IPv6 pseudo header + 完整 TCP segment checksum 必须
  成立。TCP checksum 字段没有“零值非法”规则；即使字段为 0，只要按 RFC 9293 §3.1
  对 IPv6 pseudo header 和完整段验证结果成立也应接受，并须有专门边界用例。
- IPv6 UDP 长度必须与 upper-layer length 一致，checksum 字段为 0 必须拒绝；非零值按
  IPv6 pseudo header + 完整 UDP datagram 验证。若发送端计算结果为 0，线上字段写
  `0xffff`。该规则来自 RFC 8200 §8.1，只适用于这里的内层 IPv6 UDP payload，不改变
  外层 WireGuard tunnel UDP 的现有处理。

TCP 响应跟踪沿用现有已验证语义，但在新 IPv6 observer 内局部实现，避免改变旧 observer：
服务端 SYN+1 确定响应首序号 S；分段/相同字节重传可以乱序到达，冲突字节和超出固定响应
范围失败；完整覆盖 N 字节后，累计 ACK 必须覆盖 `S+N`，不得超过已观察发送末端，只有
实际 FIN 才可再确认一个序号。比较继续使用半区间
`uint32(a-b) < 2^31`，拒绝差值恰为 `2^31`。读写边界并发顺序不影响最终判断；RST、
部分覆盖、仅 Write、部分/越界 ACK 或 ACK 超时均不得成功。

参考：
[RFC 9293 §3.1](https://www.rfc-editor.org/rfc/rfc9293.html#section-3.1)；
[RFC 8200 §8.1](https://www.rfc-editor.org/rfc/rfc8200.html#section-8.1)。

## 私有协议与失败传播

协议仍为 v1 私有 stdin/stdout LF 单行 JSON、单行≤4096字节、单方向会话≤16384字节，
保留严格未知/重复字段、run_id、随机密钥/token、首失败摘要和有界单写者。

新增 init 操作仅为 `init_ipv6_tcp`、`init_ipv6_udp`；字段与普通 init 完全相同，不接受
地址、端口、URL、family、MTU 或服务参数。ready 字段不变，selftest 仍只表示旧 IPv4
自测。正常序列严格为：

```text
init_ipv6_tcp -> ready -> ipv6_tcp -> shutdown(dut_stopped:true)
                -> stopped(mode:"ipv6_tcp",resources_closed:true) -> exit 0
init_ipv6_udp -> ready -> ipv6_udp -> shutdown(dut_stopped:true)
                -> stopped(mode:"ipv6_udp",resources_closed:true) -> exit 0
```

成功事件固定为：

```text
ipv6_tcp {v,event,run_id,ip_version:6,requests:1,response_status:204,
          response_acked:true,source_matches:true,destination_matches:true,
          authenticated:true,rx_tcp_packets:u64,tx_tcp_packets:u64}
ipv6_udp {v,event,run_id,ip_version:6,received:3,replied:3,sequences:[1,2,3],
          rx_udp_packets:3,tx_udp_packets:3,payloads_valid:true,
          addresses_valid:true,authenticated:true}
```

TCP 包计数各为 1..1024；UDP 边界计数精确为 3。stopped 的 mode 只证明对应模式资源关闭，
零业务取消也可合法输出 stopped 但必须非零退出且没有成功事件。新模式拒绝 probe_icmp、
phase/port字段、旧 tcp/udp/domain 事件和场景切换；旧模式拒绝新操作、mode 和事件。
failed 继续只允许既有固定 stage/code，不透传 Go/WG/包解析原文。

首个错误由 observer 锁存并唤醒主循环；取消业务 Context 后停止新 Accept/Read/Write，
但保留 helper WG socket、内存 listener/service 和 owner，直到 DUT 停止已确认。成功事件
不是终态：Rust/Go 必须继续监视重复事件、额外连接/数据报、worker/pipe/WG错误和迟到
failed，直到 DUT 退出；任何迟到失败优先于先前成功。

## 并发、期限、Stop/Hold 与清理

两个真实用例都持有现有固定 API 测试互斥锁，禁止并行占用 9090；Go 内一个场景仍采用
单一业务 worker、observer mutex/changed channel 和单一有界 stdout writer，不新增后台
服务、重试器、共享缓存或跨用例状态。

沿用 DCR-009/010 的绝对期限：helper spawn 后 init/ready≤10秒（旧selftest≤5秒），
ready 后工作≤30秒，父最迟第45秒进入清理，单次 I/O/ACK≤2秒，helper第55秒硬退出，
父管道/最终等待至第59秒，用例外层60秒。各期限不因阶段切换重置；开始 fixed check/run
前必须预留 check 10秒、Ready 2秒、Stop 2秒和剩余清理时间，不足直接取消。

所有退出路径共用以下顺序：

1. 停止新业务；UDP 先关闭 Rust client，TCP 无宿主业务 client。
2. 由真实 owner 停止并确认 DUT/pending；在确认前不发 shutdown、不关闭 stdin、
   不释放 peer 内存服务或 WG socket。
3. DUT 已退出（或从未启动）后才发送 `shutdown(dut_stopped:true)`；Go 关闭 listener/service、
   WG device/bind、栈和队列，等待所有 worker/PacketBuffer 引用，发 stopped 并退出。
4. 父确认 stopped、exit code、stdout/stderr EOF/join及 socket 消失后，才删除已解析且属于
   本轮的私有目录。业务 PASS 另要求唯一成功事件且全程无迟到失败。

若 DUT 退出未确认，复用 `finish(false)` Hold：不发送虚假 `dut_stopped:true`，不 kill peer、
不提前关闭 stdin、不删除私有资源；持续有界排空固定失败帧，等待 helper 在第55秒自行释放
并非零退出，整例仍 FAIL/RecoveryRequired。helper 早退、强制退出、清理成功或 API 健康
均不能替代业务成功；只可终止本轮明确持有的进程，绝不操作无关 PID。

## 阳性、负例与证据要求

- Compiler：TCP普通编译和UDP test元组的精确最终读回；错IPv6地址/prefix/端口/tag/network、
  IPv4或双地址混入、额外/双入口、额外节点/Pool/route、非唯一WG、reject缺失/后移、
  API/secret/最终字节篡改均失败。普通 compile 保持空 inbounds，新旧test元组串用失败。
- Go：IPv6-only NIC/TCP/UDP本地阳性；IPv4包误入、错误地址/端口/protocol、extension/
  fragment/jumbogram、payload length/transport length/数据偏移错误、截短、超限均失败。
  校验覆盖TCP错误checksum及合法零字段，UDP零字段、错误checksum及计算零写`0xffff`。
- TCP：精确HEAD/Host/路径、唯一连接/请求；完整204分段、乱序和相同重传+合法累计ACK
  阳性；仅Write、缺/部分/越界/歧义ACK、冲突重传、RST、第二连接和超时负例。
- UDP：三个精确包及Rust三应答阳性；错token/序号/四元组/源端口、丢包、额外/乱序、
  只有Go提交或只有Rust接收而无边界摘要均失败。
- 协议/生命周期：未知/重复字段、跨模式事件、ready后零业务取消、业务成功后迟到failed、
  worker/pipe错误、stdin EOF、Stop未确认、Hold55及既有模拟Hold150均覆盖。
- 真实固定核心：两个用例分别保存最终配置/helper/固定EXE身份、DUT/peer PID与创建身份、
  socket归属、精确业务摘要、清理和前后 interfaces/addresses/routes/dns/proxy 只读快照。
  不保存私钥、API secret、完整配置、token、原始包或stderr原文。

批准后按现有 README 执行 Go `mod verify/list/test/race/vet/gofmt/build`，按 TASK-009 现有
Cargo入口执行 compiler/协议/生命周期定向测试、两个真实 Windows 用例、产品
`cargo clippy --lib -- -D warnings`、`cargo fmt --check` 与 `git diff --check`。两个真实用例
由 Orchestrator 串行运行；缺 helper、固定资源、权限或环境时如实记 UNAVAILABLE，已识别
但未运行记 NOT_RUN。设计审查、单测、check、IPv4回归或清理不能替代 IPv6 真实业务证据。

共享 stack/protocol/lifecycle 变化要求重跑现有 WG TCP/ICMP、UDP、domain HTTP/TLS、
DNS预检、Hold55/150及完整 `init_reject` 三阶段四格。该拒绝场景必须在新交付身份下由
同一peer/密钥持续持有虚拟与宿主TCP/UDP四个目标：前阳性与后阳性逐格精确回显，中间
保护阶段四目标全生命周期零到达；各目标独立计数，最终每个TCP目标2连接、每个UDP目标
2报文（总TCP4/UDP4），并核对原资源持有、Stop/Hold、清理和前后快照。它沿用DCR-011/012
的阶段40秒、工作135秒、helper150秒、父159秒、外层160秒合同，不套用两个IPv6用例的
60秒期限，也不得从checkpoint025历史运行继承。其它源未变时可由完整manifest/影响闭包
说明继承；不得把历史全Task review改写为本新身份审阅。

## 精确文件 Scope、复杂度与批准

申请批准的实现文件仅为：

- `src-tauri/src/singbox/compiler.rs`：仅 `cfg(test)` IPv6 UDP固定入口、完整资格和负例；
  TCP走普通编译，仅增加对应测试断言。
- `src-tauri/src/platform/windows/managed_sidecar_port.rs`：现有 `wg_peer_test` 私有模块内的
  两个串行真实用例、严格Frame/PeerStage、迟到失败、socket和清理核对。
- `scripts/task009-wg-peer/main.go`、`protocol.go`、`stack.go`、`observe.go`：只接线两个
  新模式、固定family/allowed_ip、IPv6 stack分派和独立observer，不改旧模式契约。
- 新增 `scripts/task009-wg-peer/ipv6.go`、`ipv6_test.go`：集中固定IPv6地址、TCP/UDP内存服务、
  IPv6 parser/checksum/ACK观察及其单测。
- `scripts/task009-wg-peer/README.md`：只记录新模式、证明边界与既有命令。

不修改 `domain.go`、`udp.go`、Go/Cargo/npm manifest或锁、产品 API/配置/运行源码、Task、
state、HANDOFF、既有 Evidence 或固定资产；不新增依赖、binary、网络服务、配置项、通用
network abstraction、fallback 或兼容层。若实现发现上述文件不足，或需要产品路径、依赖、
DNS/TUN/OS网络、第三个业务模式或新错误协议，必须停止并返回新的影响分析，不能自行扩Scope。

最小复杂度是一个 IPv6 test编译元组、两个私有协议模式和一个局部IPv6 observer/service
文件；旧IPv4 parser/ACK代码保持原字节语义。复制少量已冻结ACK算法会产生局部重复，但避免
重构共同观察器导致checkpoint025全部模式的语义和证据扩大失效。等价替代包括：

- 只做gVisor自测或固定核心check：不能证明真实DUT/WG IPv6；
- 复用IPv6域名：会把DNS漂移/解析与本项业务混在一起，扩大DCR-015日志和资源边界；
- 增加OS IPv6地址/TUN或公网目标：需要新的平台/外部资源授权且不再是内存受控拓扑；
- 把TCP也改成本机direct入口：增加无必要的OS TCP listener，且绕开现成URLTest业务路径。

因此采用IPv6字面量URLTest TCP + 单个test-only UDP入口。Owner 需要明确批准的增量是：
上述精确文件Scope、两个新私有模式、进程内`fc00::1/128`/`fc00::fe/128`、一次短时loopback
UDP测试入口，以及两个串行60秒真实固定核心用例。批准不授权实现外文件、DNS/TUN/主机
配置、外部资源、commit/push、Task/Gate迁移或TASK-010。

受影响 Gate：当前候选保持 PROPOSED；独立设计审阅与 Owner 批准后才能由 Orchestrator
同步 Scope/冻结新设计身份。实现会产生新的交付身份，使checkpoint025对受影响共享路径的
Delivery/Review Evidence stale；需重新验证及独立交付审阅。Task、Delivery Gate和Human
acceptance在本候选中均保持现状。
