# TASK-009：主动入站/转发拒绝的 peer 侧可行性

状态：PROPOSED / NOT_RUN。仅核定候选资源与证据边界；没有新增资源运行、实现、依赖变更、
系统变更或验收结论。需与 Orchestrator 的固定 DUT Router/DNS 研究合并，再形成待审 DCR。

## 输入与已核定事实

- Requirement：`docs/veyra.md`，SHA-256 `4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`。
  TASK-009 SF-003 要求主动 TCP/UDP/DNS、宿主/受控非宿主转发拒绝；超时须有同路径阳性对照。
- checkpoint013：`5f6438663e7d08e0241fde9a733e466c2dc7031dba31dbe050c1b3f36e52f0d5`。
  已有出站 TCP/UDP 和虚拟 ICMP 证据只作可复用基线，不代替本候选拒绝矩阵。
- DCR-009：`b193f16e6262f8695780af2f18cebc4a4f91dbbbe999f4b91f716059edbb9470`；
  DCR-010：`36a910c90c247e1b46958ed7890dceabc7d769aee1ca1f18b062154de8ea9c79`。
- `newMemoryTun` 创建独立 IPv4/TCP/UDP/ICMP gVisor 栈，仅给 NIC 1 注册一个地址，
  默认内存路由指向 channel。`memoryTun.Read` 是送往 WG 加密的明文边界；`Write` 是
  WG 已认证解密后交付的边界。现有观察器仅允许既有固定方向，不能直接复用为任意目标观察器。
- `newLive` 目前给 Go WG 注册唯一 DUT 公钥，`allowed_ip=198.18.0.1/32`，依靠认证后端点学习。
  固定 wireguard-go `device/send.go:330-362` 对 tun 包做 allowed-IPs peer 查询，找不到即丢弃。
  因此只调用 gonet 或改 gVisor route，不能发送到 host/非宿主目的。
- 固定 gVisor `ipv4.go:956-970` 默认丢弃非 loopback NIC 收到的 127/8 源或目的包。
  `NewProtocolWithOptions(Options{AllowExternalLoopbackTraffic:true})` 已存在（2052-2058），
  可仅在新测试场景允许 host 回包；它不改变 Windows loopback、防火墙或路由。
- Orchestrator 已核定固定 DUT 将本地虚拟目的 `198.18.0.1` 映射到 `127.0.0.1`，端口不变。
  本文不重复该 DUT 源码研究；最终 DCR 应引用其精确来源与配置约束。

上述 Go 库来源均为已下载的固定模块一手源码：
`github.com/sagernet/wireguard-go@v0.0.5-0.20260823125007-8bd032a91a30`、
`github.com/sagernet/gvisor@v0.0.0-20260727.0-sing-box-mod.1`。
本次仅读本地模块，没有下载或安装。现直接依赖足够候选 peer 能力，不建议新增库。

## 最小虚拟地址/宿主矩阵

```text
攻击者 gonet（内存 198.18.0.2） → WG → DUT endpoint → Router
                                                 ↓ 阳性例外
                                  Rust 持有的 127.0.0.1:目标端口
```

1. Go 主动 TCP 使用既有 `gonet.DialContextTCP`，UDP 使用 `DialUDP`；全部指定 NIC 1、
   数字地址，不调用主机 resolver 或 `net.Dial`。TCP 使用有界固定请求及带本次 token 的应答；
   UDP 复用固定 token/序号载荷。TCP 栈可能自动重传 SYN，应用只发起一次连接，证据必须区分
   原始包数与唯一业务请求，不能把 TCP 重传当三次独立攻击。
2. `198.18.0.1:目标端口` 与 `127.0.0.1:同一目标端口` 分别测试，不能因 DUT 内部映射而合并。
   Go 新场景的唯一 DUT peer 仅增加目标 `127.0.0.1/32` 的 cryptokey 路由；不开放任意 IP。
   host 场景同时启用上述内存 IPv4 loopback 选项，并按目标精确校验回包来源/四元组。
3. Rust 新建并持有一个 loopback TCP listener、一个 loopback UDP socket，动态端口由实际
   bind 返回；服务只接受本次 token 的固定请求/回显，无任何上游或命令执行。虚拟地址和宿主
   地址共享该真实目标。不能用 API 9090 或用户已有服务作阳性目标，也不能只预留后释放目标。
4. 现内存 peer 的 HTTP/UDP listener 不能代替这个宿主目标：其地址只属于 Go 栈，Windows
   Direct 无法自然到达；把目标加到攻击栈又会令流量本地终结，绕过 DUT。
5. 阴性必须使用保留置顶 route/DNS reject 的 Compiler 最终配置。阳性需要测试专用、封闭的
   Router 例外，将本次 endpoint 的精确目标地址/端口/网络路由到 Direct；如何表达和核验
   由 Orchestrator 的 DUT 研究决定。不能手改最终 JSON、开放产品入站或任意 Direct 转发。

最小新增资源是两个 Rust loopback 目标 socket，加 peer 新场景的固定目的和内存栈选项；
不是系统 TUN，不修改宿主配置。root 拟将下列四格作为 DCR-011 唯一增量：虚拟 TCP、虚拟
UDP、host TCP、host UDP。非宿主与 DNS Router 继续保留后续要求，不属于本次待批运行包。

### DCR-011 拟定的同路径配对

同一 Go peer、密钥和 Rust 目标 listener 保持存活，顺序启动三个独立 DUT 实例：阳性前置、
阴性、阳性后置；每实例生成新 API secret，旧实例退出已确认后才进入下一阶段。阳性仅在
测试 Compiler 中、原置顶 reject 之前插入两条精确例外：指定 inbound、`127.0.0.1/32`、
各自已持有目标端口、TCP 或 UDP → Direct。原 reject 及其后的一条
`Port([tcp_port,udp_port]) → Direct` 用户规则继续保留。阴性使用正常产品 compile，
保留上述用户规则，证明用户规则不能覆盖 reject。
虚拟目的由固定 DUT 先映射至 loopback，最终字段/排序和完整允许元组由 DCR-011 冻结。

三个 DUT 使用唯一 URLTest Pool（interval=300 秒），每阶段由新 DUT 主动请求 peer 内存
HTTP `198.18.0.2:18080`，完成精确 HEAD/204 及完整响应 ACK 后才发起四格主动探测。
这一 bootstrap 既验证该阶段 WG 可往返，又让 Go WG 通过认证流量学习新 DUT 的实际协议
端口；不能依靠 API Ready、上阶段端点缓存或单纯等待开始主动发送。父先发 `begin_phase`
重置该阶段有限摘要，然后启动新 DUT；所有已完成阶段的目标 token 计数仍保留，不能因重置
而漏掉晚到攻击。bootstrap 流不得计入主动请求/目标计数。旧阶段迟到或跨阶段事件均失败。

该新场景期限按 DCR-011：每 phase 40 秒，父全局工作 135 秒、helper 硬截止 150 秒、
父最终截止 159 秒、外层 160 秒，专门容纳三次真实 check/run/stop。
这是需要 DCR-011 审阅与批准的生命周期增量；原 TCP/UDP/ICMP 场景保持原 55 秒硬截止。
父须在工作截止后进入清理，先确认 DUT 停止再释放 peer；不能将外层期限视为允许 DUT
活到 peer 释放之后。本文仍是 PROPOSED 可行性说明，具体契约以 DCR-011 为准。

## DNS 载荷与 DNS Router 必须分开

Go 可用现 UDP/TCP 栈发送完整 DNS wire query：固定 A/IN、单问题，名字为本次随机 token
组成的 `<token>.invalid`；TCP 加两字节长度前缀。受控目标只对精确问题返回固定测试响应，
匹配事务 ID、QR、问题及答案，不递归、不转发、不调用系统解析。由有界字节编码/校验即可，
无需新增 DNS 库、域名供应者或主机 DNS 配置。

- 在动态业务端口发送 DNS 格式载荷，只能证明 TCP/UDP Router 路径拒绝该载荷，不能证明
  `NewDNSPacket` / DNS Router reject 被执行。
- 目标端口 53 可能触发 DUT 特殊 DNS 路径；不能假定它会抵达普通 loopback listener。
  如固定核心确实走 Direct 且原端口不变，宿主阳性需实际持有 `127.0.0.1:53` 的 UDP/TCP
  socket。冲突即 UNAVAILABLE，不关闭已有 DNS 服务、不改系统 DNS、不换端口冒充同一路径。
- 如由 DUT DNS Router 处理，阳性应是新测试专用 DNS 例外的受控答案或受控 resolver 计数，
  由 root 核定 hijack/predefined 等可执行表达。该项未核定前，保留 DNS Router 拒绝未验证。
  网络 DNS-query 拒绝与“节点/URLTest 使用系统解析”的位置证明仍是不同验收项。

## 可控非宿主目标：候选与取舍

目的选为测试内存 `198.18.0.3`，不是 Internet 主机、Windows 地址或新增系统路由。
攻击源保持 `198.18.0.2`；只向其 DUT peer 增加 `.3/32` cryptokey 路由。

**候选 A：同一 WG device，独立目标栈。** 另建仅拥有 `.3` 的 gVisor 栈，通过受限 tun
分流/发送合流接在同一 WG device 后。DUT 阳性将 `.3/32` 流重新路由回此 WG endpoint；
解密后按精确目标 IP 分配到攻击栈或目标栈，两个栈无直接通道、无本地互路由，目标收到的
请求必须先经过 DUT。目标响应亦经 WG → DUT → 攻击栈；不能直接复制回攻击客户端。
优点是不增 OS socket/密钥；代价是新内存分流及双栈队列生命周期，且依赖固定 DUT 支持
同 endpoint 的 Router 回送及返回映射。当前未核定该 DUT 语义，不能把它当可运行方案。

**候选 B：第二受控 WG peer，独立目标栈。** 第二 WG device/密钥/loopback UDP socket
只服务 `.3` 内存 TCP/UDP/DNS；DUT 有第二封闭 endpoint，阳性精确路由 `.3/32` 到它。
攻击者与目标栈不直接连接。目标计数在第二 peer 的认证解密边界及应用服务分别记录，
响应通过第二 WG、DUT、第一 WG 返回。可在同一个 helper 进程持有两个 device，无需新增
二进制/依赖，但增加一个 OS socket、密钥对、DUT WG endpoint 及其协议 socket 归属核验。
两 peer 在 DUT 停止确认前都不能释放。DCR-009/010 单 peer/单 endpoint 模型需明确增量批准。

本轮不继续研究或实施 A/B；它们是后续非宿主转发所需资源的明确候选。届时先核定 A 的
同 endpoint 回送，不成立再决定 B；不能假定内存地址经 Windows Direct 可达或改访真实外网。
无论 A/B，纯内存两栈自测不算 DUT 转发证据。

## 观测与配对判定

每个目标/协议必须拥有同路径阳性 → 阴性 → 阳性的配对记录。目标服务在整个配对期间
持续持有真实 socket/内存 endpoint，独立记录阶段 token；不能在阴性前停止服务或清零后
忽略晚到请求。两次阳性都要求目标实际读取完整请求、客户端实际收到正确应答。TCP 仅
Dial 成功或 ACK、UDP Write 成功、DNS 仅返回某个错误码均不满足阳性。

阴性窗口要求攻击请求确实到达 Go tun 待加密边界、目标负向 token 到达数为零、没有成功
应答，并完成固定观察窗口；RST/超时只作结果之一。任何负向 token 晚到目标都判失败。
阳性失败、目标线程提前结束、无法核定 socket 所有权、配对配置差异超过批准例外时，本组
记 FAIL/UNAVAILABLE，不能计 reject PASS。服务自身 ping 成功不能代替经过 DUT 的阳性。

认证证据必须准确命名：Go tun.Write 回包证明收到该 DUT 公钥认证的解密数据；完整阳性
往返还证明该路径可达。Go tun.Read 只说明请求送入加密路径，不能独自证明 DUT 收到或
解密了每个被拒绝包。若 DUT 没有可绑定单包的认证/拒绝观测，本组只能报告“同路径前后
阳性成立，负向窗口目标零到达”，不能输出每个负向请求 `dut_authenticated:true`。
握手、加密 UDP 字节数、泛化总流量均不能填补这一缺口。

需要对照的身份包括 helper/固定核心/最终配置 SHA、opaque DUT 实例、父持有的 PID、
源/目的与阶段枚举、监听归属、目标读到数/应答收到数、tun 请求/认证回包计数、窗口起止、
清理结论。日志仅固定摘要，不保存密钥、init、token、完整配置、DNS 名或包体。

## 消息、资源寿命与尚需决定的项

候选私有协议采用封闭新初始化场景，例如 `init_reject`，复用旧密钥/run_id/token 字段；
增加的输入仅已持有目标端口及有限目标/协议/阶段枚举，不接收任意地址、URL、路由或 JSON。
`begin_phase` 严格按前阳性/阴性/后阳性顺序，`probe` 由父在该阶段 bootstrap 成功后发起；
`probe_result` 返回固定发送/应答/超时和边界计数。
真实目标计数由持有目标的 Rust 或第二内存栈独立生成；Go 不能自己宣称整组 reject PASS。
上述消息名尚未冻结，最终 DCR 须固定字段、操作次数、允许顺序及现有 4096/16384 字节限额。

原场景继续原业务 30 秒、单 I/O 2 秒、父 45 秒清理、helper 55 秒硬截止和外层 60 秒上限；
本候选四格新场景采用上述 DCR-011 期限分配，单 I/O 仍最多 2 秒并继承绝对截止。
失败取消新业务、保留两个方向的目标/peer，先确认自有 DUT 退出，再 shutdown helper，最后
关闭 Rust 目标并保留计数。内存队列有界，未确认 DUT 停止不关闭 stdin、不抢先释放 peer。
不能在阶段切换时重置总截止；目标租期、阶段身份和父丢失后的硬退出由最终 DCR 完整固定。
当前文档只描述新预算候选，尚未授权延长生命周期。

待 root 合并核定的最小问题：

1. DCR-011 固定上述四格的强类型例外、begin_phase/bootstrap、阶段 token/请求上限和完整摘要。
2. 按 DCR-011 的 40/135/150/159/160 秒分配落实停止新业务、停止 DUT、释放 peer，并批准新增资源。
3. 后续仍需端口 53/DNS Router 分支及答案/计数来源；当前不将 DNS-wire 计为完整 DNS 拒绝。
4. 后续仍需决定非宿主候选 A/B；当前四格不宣称完整 SF-003 或 TASK-009 验收通过。

本候选没有运行测试，未修改任何 Go/Rust/Compiler/Task/state 或既有设计文件。
