# DCR-009：受控 WireGuard 测试 peer

状态：设计候选，待独立审查；不是 Gate 批准或运行证据。
TASK-009 / SF-003。用户已明确“允许”批准 `wg-network-proposal.md`
`sha256:ee61ea74e60237abae9453aa1103680b801561f29c06d86be9e46dcd87399843` 的全部最小包。
原提案字节保持不变；其 PROPOSED 标记为历史来源，不表示需要重复资源授权。

## 目标和冻结边界

补齐正常 ObservationOnly 固定核心的 WG TCP 出站应答、虚拟 IPv4 ICMP Echo 例外，以及
对应测试资源的归属和清理。保留 DCR-003 的用户态 WG、唯一 peer、route/DNS 置顶 reject；
不修改生产 Compiler、RuntimeProfile、Domain、AppState、产品 IPC、Cargo/npm 依赖或系统设置。
不采用 DCR-008 TCP 计量入口；不添加 DUT UDP 入站、DNS 资源、拒绝规则例外或外部目标。
TCP/UDP/ICMP 自测只验证 peer 本身；DUT UDP、IPv6、完整主动入站/转发拒绝和系统 DNS
仍是 SF-003 必需但本候选不覆盖的后续项，不改变其 Acceptance。

## 文件与依赖

- Go 责任范围：`scripts/task009-wg-peer/` 的实现、单元测试、README、`go.mod`、工具生成
  的 `go.sum`。模块名固定 `veyra.local/task009-wg-peer`；仅测试程序，不加入发行或默认 CI。
- Rust 责任范围：`src-tauri/src/platform/windows/managed_sidecar_port.rs` 现有 cfg(test)
  模块新增集成内测和局部 helper，以便复用私有 running child 归属及既有计量测试的
  socket 观测方式；不增加生产访问器，不修改其非 cfg(test) 代码。
- 设计/授权/Task 同步及证据由 Orchestrator 负责。Go 与 Rust 按下述协议并行实施，
  不互改对方文件；所有真实核心运行由 Orchestrator 串行调度，复用固定 API 测试锁。
- 两个固定直接依赖为 `github.com/sagernet/wireguard-go`
  `v0.0.5-0.20260823125007-8bd032a91a30` 和 `github.com/sagernet/gvisor`
  `v0.0.0-20260727.0-sing-box-mod.1`，均已由固定 EXE 元数据核实。
- 使用已安装 Go 1.27.1，`GOTOOLCHAIN=local`、`GOWORK=off`，不自动换工具链。
  正常间接依赖由 Go 工具解析；保存实际模块清单和校验摘要，不手改 go.sum，
  不声称间接图全部等同 EXE。新增直接依赖或更改固定版本须返回设计问题。
- helper 构建输出固定为 `src-tauri/target/task009-wg-peer.exe`。
  Rust 不接收任意 helper 路径、不在测试中自动下载或编译；文件缺失直接失败并给固定提示。
  每轮运行前记录其 SHA-256、Go 模块清单及代码目标身份，不能只凭文件名复用未知二进制。

## Rust ↔ Go 测试进程协议 v1

仅父子私有 stdin/stdout 管道的测试协议，不是产品 API/IPC。无 TCP 管理端口、CLI 密钥参数
或环境变量凭据。UTF-8 单行 JSON，以 LF 结束；每行最多 4096 字节（含 LF），单方向
整次会话最多 16384 字节，stderr 独立有界读取最多 4096 字节但不得直接转储原文。
只允许以下字段；未知字段、错误类型、缺失字段、越界值、错误版本/run_id/顺序或重复
操作均失败。每个帧只解码一个 JSON 对象，拒绝对象后的非空白数据；不回显拒绝帧。

共同字段 `v` 固定整数 1，`run_id` 为 Rust 用现有 getrandom 生成的 16 字节小写 hex。
`token` 是另外独立生成的 16 字节小写 hex，不复用 WG/API 密钥。
两组私钥也是 Rust 用 getrandom 生成的独立 `[u8;32]`；在构造 X25519 私钥前执行标准
clamp（首字节 &=248，末字节 &=127、|=64），禁止测试固定密钥。Go 验证数组长度，
使用标准库 `crypto/ecdh` X25519 推导公钥；不自行实现曲线运算。

父 → 子固定消息如下（方括号为类型说明，不是字面量）：

```text
init       {v, op:"init", run_id, dut_private_key:[32个0..255整数],
            peer_private_key:[32个0..255整数], token}
probe_icmp {v, op:"probe_icmp", run_id}
shutdown   {v, op:"shutdown", run_id, dut_stopped:true}
```

子 → 父固定消息如下：

```text
ready   {v, event:"ready", run_id, udp_port:u16, peer_public_key:string, dut_public_key:string,
         selftest:{tcp:true,udp:true,icmp:true}}
tcp     {v, event:"tcp", run_id, requests:1, response_status:204,
         rx_tcp_packets:u64, tx_tcp_packets:u64, authenticated:true, response_acked:true}
icmp    {v, event:"icmp", run_id, sent:3, received:3, id:9,
         sequences:[1,2,3], payloads_valid:true, addresses_valid:true}
failed  {v, event:"failed", run_id, stage:enum, code:enum}
stopped {v, event:"stopped", run_id, resources_closed:true}
```

两项 public_key 都是标准 Base64 编码的 32 字节公钥；Rust 无需新增 X25519 依赖，
Go 内部使用 DUT 公钥注册 peer，Rust 可在构建/比对 fixture 时使用公开部分；不返回私钥。
`udp_port` 必须非零且非 9090，宿主地址隐含固定 127.0.0.1，不由消息输入。
`stage` 枚举为 init/selftest/bind/tcp/icmp/protocol/deadline/cleanup；`code` 枚举为
invalid_input/io_error/timeout/unexpected_packet/limit_exceeded/resource_error。
失败仅输出第一条固定 failed；不包含 error 字符串、私钥、token、原配置或包体。
正常只产生 ready → tcp → icmp → stopped；父收到 failed/EOF/非法帧即进入 DUT 清理。
Go 不等待 probe_icmp 才接受 HTTP；TCP 事件可能紧接 ready 到达，Rust 队列须保留它。
probe_icmp 必须在 tcp 完成后且最多一次；shutdown 在成功或失败后均可用于清理，
也可在 ready 后取消测试。父只有确认 DUT 从未启动或已退出后才发送 dut_stopped:true。

Rust 采用有界消息通道与持续排空 stdout/stderr 的读侧；所有管道写入/等待读取都由
绝对期限约束，不能在测试线程无界 read_line、write_all 或 join。超大行要在读完前拒绝，
不能先无界分配再查长度。Go 同样分离有界输入读取与状态机，不允许阻塞 stdin 延长期限。
读侧在 child 确认退出/管道关闭后有界 join；未完整收到 stopped 或退出非零不能记清理 PASS。
业务失败即使随后正常清理，helper 最终仍非零退出；完整正常流程退出 0。

## 内存栈与 WG 传输

地址固定：DUT 198.18.0.1/32，peer 198.18.0.2/32，MTU 1280；只创建用户态 IPv4 NIC。
Go 使用 gVisor stack/channel/gonet；channel 队列容量 64、WG BatchSize=1。
内存 tun.Device 的 Read/Write 负责原始 IP 包与 channel 的转接，每次 offset/包长按接口
处理，PacketBuffer 引用恰好释放一次；关闭 done 后所有阻塞 Read/Write 都必须退出。
TCP、UDP、ICMP 协议由 gVisor 实现；不开系统 raw socket、tun.CreateTUN 或主机路由。

自有 conn.Bind 使用标准库 UDP4，在 Open 时精确 bind 127.0.0.1:0；不得调用
StdNetBind 代替。Open 实际取得 9090 则失败，不重选端口。Close 幂等且关闭底层 socket，
使 ReceiveFunc 返回关闭错误；Send/ParseEndpoint 只接受 IPv4 loopback，单批单包，
SetMark 对 0 无操作，对非零失败；reserved 只允许固定零值。对输入限长、关闭和超时
显式处理，不能悄悄忽略库接口错误。

初始化 WG device 时仅注册本次 DUT 公钥，allowed IPs 为 198.18.0.1/32，无预置远端端点；
让 WG 自身在认证后学习端点。内存 tun 写入收到匹配 DUT IP 的解密包才作为认证佐证；
不能用未经认证的首个 UDP 来源设置对端。连接过程中 OS UDP 包可能来自其它本机进程，
认证和学习由库处理；helper 自己不添加“首包信任”或任意转发。

## 自测与真实流量断言

ready 前运行一次纯内存阳性自测：两个隔离 gVisor 栈经有界 channel 相连，分别确认
TCP、UDP 回显已知字节完全一致及 ICMP Echo 的 ID/序号/载荷；不创建 OS 网络 socket。
自测栈和计数清理后再创建真实 peer；不能把自测计数混入真实 WG 结果。自测失败不发 ready。

真实 peer 的 HTTP 服务固定在内存 198.18.0.2:18080，不是 Windows listener。
Rust 将本次 peer 端口/公钥、DUT 私钥 Base64 和固定虚拟地址通过现有 Parser → normalize →
AppState/RuntimeIntent → Compiler → per-instance secret/finalize → fixed check/run
建立唯一 WG 节点和唯一 URLTest Pool。探测 URL 精确为
`http://198.18.0.2:18080/task009-wg?token=<token>`，间隔 300 秒，不手动追加第二次探测。
正常生产编译无业务 inbounds，route/DNS 首项仍拒绝该 endpoint；最终字节不做 JSON 改写。

HTTP 服务最多接受一条真实流，精确要求 HEAD、上述 RequestURI、Host=198.18.0.2:18080，
请求头上限 16 KiB，不读 body；回复 HTTP/1.1 204、Content-Length:0、Connection:close。
发送完成后关闭该连接，listener 持有至 shutdown；若又有连接到达，记失败，不再次应答。
TCP 事件必须在请求校验、完整响应写成功、完整响应已从 tun.Read 发出且 DUT 已确认
接收之后发出；rx/tx_tcp_packets 都 >0，以真实 tun 的解密接收/待加密发送为方向，
排除握手和自测。服务端 Write 成功只证明发送缓冲接受数据，不能单独满足阳性条件。

响应使用固定序列化字节 `HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n`。
在 tun.Read 跟踪唯一连接四元组和实际 TCP seq/payload：由服务端 SYN 序号加一确定
首数据序号 S；逐段对照预期响应字节，允许相同字节重传，必须连续覆盖全部 N 字节。
S+N（模 2^32）是响应末字节之后的序号，SYN/FIN 消耗的序号不得误计作响应数据。
在 tun.Write 的已认证解密入口，仅接受该四元组反向、IPv4/TCP 长度与校验有效、
ACK 标志置位且无 RST 的 DUT 报文作为确认；累计 ACK 必须覆盖 S+N，且不超过
已观察的发送序列末端（只在实际观察到 FIN 时允许再确认一个 FIN 序号）。
序号比较使用 RFC 式半区间：`after_or_equal(a,b) = uint32(a-b) < 2^31`；
每连接观测跨度严格小于 2^31，实际受 1 MiB 上限约束，拒绝模差恰为 2^31 的歧义值。
保留同一连接已见的有效累计 ACK，处理读写两侧观察的并发顺序；只有完整字节覆盖和
有效 ACK 两项同时成立才置 response_acked:true。等待 ACK 同样 ≤2 秒且继承工作截止；
未确认、只有部分 ACK、RST 或超时均失败，不能发成功 tcp 事件。

父同时断言 Ready、精确请求/204、双向计数和 response_acked:true。该证据证明响应完整
交付到 DUT TCP 层，不证明 URLTest 应用层已解析/消费响应；不把该结论称为 URLTest
应用层成功，也不以 Clash API 在线或任意延迟数字替代交付证据。

父收 tcp 后发 probe_icmp；Go 使用 ICMP 栈 endpoint，从 198.18.0.2 向 198.18.0.1
依次发送三次 Echo，ID=9、序号 1/2/3，payload 为 token 的 16 原始字节加序号单字节。
每次只发送一次，不作应用重试；逐次等待匹配应答后再发送下一次。检查 IPv4 源/目的、
Echo Reply type/code、ID、序号、完整 payload 和数量。错包/重复/额外 Reply 判失败；
并从解密边界核对三次目标 Echo Reply，不能只相信业务函数返回 nil。
完成后报告 icmp；从最后 Reply 到 DUT 停止仍累计检查不得有额外 Echo Reply，异常可再发 failed。
每次 TCP/ICMP I/O ≤2 秒且不超过统一工作截止；实际业务计数累计上限 1 MiB。

## 时间、失败传播与所有权

真实用例外层 60 秒。父从 helper spawn 起计绝对截止，init/ready 最多 10 秒（含自测
总计最多 5 秒）；ready 后 peer 工作窗口 30 秒，DUT check 沿用 10 秒、Ready 2 秒、
Stop 2 秒，父不因阶段切换重置外层期限。父最迟 spawn 后 45 秒开始最终清理。
Go 在工作期限到达时报告 failed(deadline,timeout)，停止发起业务，进入 Hold，保留 UDP
socket 和 listener 等待 shutdown；不能先关闭 peer 再等待父停止 DUT。

父持有 helper Child 句柄，Windows 隐藏窗口启动，继承 stdin/stdout/stderr 私有管道。
固定资产和 DUT 继续使用现有 Windows Port/PrivateRuntime/ACL 与每实例 secret。
成功、协议失败、panic 和超时路径共享清理顺序：

1. 停止安排新业务，停止并确认自有 DUT/pending；在此之前不发 shutdown、不关 helper stdin。
2. 若 DUT 已退出（或从未启动），发 shutdown；Go 关闭业务 endpoint、WG device/bind、
   栈和队列并有界等待 goroutine，2 秒内发 stopped，随后退出。父确认 exit 和管道结束。
3. 若 helper 不响应，父只终止自己持有句柄的 helper 并确认退出，记录清理 FAIL，不能
   将强制退出冒充正常 stopped。若 DUT 清理失败，保留归属/私有目录并报告 FAIL；
   不删除文件或宣布 Stopped，也不启动其它 DUT。
4. 只有退出已确认，才按已解析自有根边界删除本次测试资源/私有目录；保留失败证据。

Go 若 stdin EOF、父异常消失或协议失败，停止业务并进入 Hold；为避免孤儿长期占用，
在 spawn 后 55 秒硬上限关闭自有资源退出非零。正常父必须先在 45 秒触发 DUT 清理。
helper 意外提前退出/硬期限释放端口时，父立即停止 DUT 并判失败，不继续流量。若操作系统
拒绝停止 DUT，则不能同时保证“端口永久保留”和“helper 有界退出”；这是真实清理失败
风险，必须保留归属与失败证据，不通过无限等待、杀无关进程或宣称无风险掩盖。

## 实际 socket 与主机边界

Go ready 时和 DUT Ready/流量完成后，父按持有句柄确定的 PID 读取 socket，不能采信
helper 自报 PID。helper OS 网络资源应恰为一个 127.0.0.1:ready.udp_port UDP4，零 TCP
listener；198.18.0.2:18080 只在内存栈。DUT TCP listener 必须恰为 127.0.0.1:9090。
记录 DUT 实际 UDP4/UDP6 端点；只接受本次 WG 产生的动态端口，允许 wildcard/loopback
绑定并明确记录，不能出现意外业务 listener。对 UDP 双栈同端口视为一组协议资源；
若出现额外未知端口、无法核定归属或归属改变则失败，不推测其用途并忽略。

固定版本单 peer 的具体 UDP 绑定仍须实测；用户已批准短时可能全接口 WG UDP，
此处不把 loopback 目标写成 loopback 监听保证。前后只读比较接口、地址、路由、DNS、
代理配置，并核对退出后自有 socket 消失。只枚举已知 PID 的相关 socket，不扫描全系统
TCP/UDP 流量。无防火墙、DNS/hosts、系统代理、TUN、UAC、WFP、Service、证书修改。

## 必需验证与交付证据

Go 实现先做 framing/状态/超限、内存栈阳性、Bind loopback/关闭唤醒、绝对期限与失败脱敏
单测；TCP 阳性判定必须包含仅握手/HEAD、Write 成功但无 ACK、仅部分 ACK、错误四元组/
RST、响应分段/重传及序号回绕等用例：缺完整确认不得产生成功 tcp 摘要，完整匹配 ACK
才通过，超时沿用固定错误。纯内存 TCP 自测也通过同一观察器证明完整响应确认。
执行 `go test -timeout 60s ./...`、`go vet ./...`、gofmt 检查、模块校验和显式 build。
Windows race 如不可用记 UNAVAILABLE，不改依赖或安装 C 工具链来补。
Rust 只按现有 Cargo 约定运行新增的 WG 定向内测及受影响静态检查；真实用例由 root 串行
运行且有外层 60 秒，不能让两个 agent 同时占用 9090 或启动 peer。

证据包含代码/配置/helper/固定 EXE hash、模块版本、opaque DUT 实例身份、父持有的
两个进程归属、实际 socket、三个 selftest 结果、精确 HEAD/204 和 TCP 双向计数、三次
ICMP 匹配结果、deadline/退出/清理及主机前后对比摘要。私钥、API secret、init 帧、完整
配置及原始包体不落盘；只记录固定错误类别，不能将 Go/WG 原始错误转发到产品日志。
Go GC 不能保证所有密钥副本立即擦除；清理时丢弃引用和有限可擦除缓冲，不声称安全擦除。
此候选没有安装、编译或真实网络运行证据；来源和精确版本核定见已批准资源提案及 DCR-003。
