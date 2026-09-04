# TASK-009 WireGuard 测试端

仅供 DCR-009/010/011/012/013/015 的 Windows 受控测试使用，不是产品组件或独立代理服务。
固定 Go 1.27.1；两个直接依赖及版本见 `go.mod`。不安装系统 TUN，不改主机网络设置。

在此目录执行下面的显式步骤，外层进程调度器应另外设置合理超时（单测 70 秒，下载/构建
120 秒）。不自动更换工具链、依赖版本，也不自动运行真实 DUT。

```powershell
$env:GOTOOLCHAIN='local'
$env:GOWORK='off'
go mod download
go mod verify
go list -m all
go test -v -count=1 -timeout 60s ./...
go test -race -count=1 -timeout 60s ./...
go vet ./...
gofmt -l .
go build -o ../../src-tauri/target/task009-wg-peer.exe .
```

`gofmt -l .` 应无文件输出；有输出时先格式化再重验。`go.sum` 由 Go 工具生成，不能手改。
输出 EXE 位于忽略的 target 目录；集成前记录 SHA-256，Rust 只运行该固定路径的已核定文件。
上述显式 Go build 是新增 Rust WG 内测的先决条件；缺少该 EXE 时内测失败，不自动构建。
WG TCP/ICMP 和独立 UDP 集成由 Rust Windows Port 内测串行驱动，不能与其它固定 API 9090 测试并行。

父进程通过私有 stdin 输入 DCR-009 v1 单行 JSON。每行 ≤4096 字节、单向总量 ≤16384 字节。
init 包含本次 run_id、token 和两组随机私钥；stdout 只输出公钥及固定摘要，不输出 init、
密钥、完整配置或原始网络报文。不要手工把 init 命令写入终端、命令行或日志。

`init` 事件顺序为 ready → tcp → icmp → stopped；`init_udp` 使用相同输入字段，事件顺序
为 ready → udp → stopped。场景之间不能切换，UDP 场景不能发送 probe_icmp。
ready 前自测两个纯内存栈的 TCP/UDP/ICMP；
随后只在 OS 绑定一个 `127.0.0.1:动态端口` 的 UDP4 socket。HTTP 服务
`198.18.0.2:18080` 仅在内存栈，不是主机 TCP listener。

tcp 摘要要求精确 HEAD、固定 204 响应字节、同一连接完整响应序列覆盖和 DUT 累计 ACK。
`response_acked:true` 只证明交付至 DUT TCP 层，不证明 URLTest 已解析响应。
icmp 摘要要求 DUT 虚拟地址三次 Echo 的类型、ID、序号、地址和 token 全部一致。

UDP 服务 `198.18.0.2:18081` 仅存在于内存栈。每包必须恰为 20 字节：16 字节解码后的
随机 token，随后是大端 u32 序号 1、2、3。服务校验 DUT 虚拟源地址、首包源端口及顺序；
观察器另在 tun 入口和出口验证地址、端口、载荷、IPv4/UDP 校验和及恰好三次请求/响应。
IPv4 UDP 零校验和合法，非零校验和必须匹配。成功 udp 摘要字段为 received/replied=3、
sequences=[1,2,3]、rx_udp_packets/tx_udp_packets=3，以及 payloads_valid、addresses_valid、
authenticated=true。该摘要证明经过 peer 报文边界；实际交付还要求 Rust 客户端收到三个
完整回显。业务失败使用既有 protocol 摘要，绝对期限使用 deadline，不增加错误分类。

工作窗口到期或业务/协议失败会报告 failed 并保留 peer 端口。父必须先停止并确认 DUT，
再发送 `shutdown` 且 `dut_stopped:true`。正常会话限 30 秒工作，父最迟 45 秒开始清理，
helper 55 秒硬截止，真实用例外层 60 秒。异常终止/清理失败必须记 FAIL，不能假称端口
归属连续；未确认 DUT 退出不得删除其私有资源。

单元测试包含内存协议阳性、完整 ACK 和分段/重传/回绕、缺 ACK/部分 ACK/错误连接/RST
负例、协议限长/重复字段/场景混用、UDP 错 token/顺序/源端口/截断/校验和、缺少报文边界
证据、取消保留端口和 stdout 写入截止。它们不是真实 DUT 的集成证据。
UDP 集成仅覆盖上述三次受控回显；IPv6、主动入站/转发拒绝矩阵及系统 DNS 仍不在范围内。

## 三阶段虚拟地址与宿主拒绝对照（DCR-011/012）

`init_reject` 仅增加四格：virtual_tcp、host_tcp、virtual_udp、host_udp。Rust 全程持有
127.0.0.1 的 TCP/UDP 目标和 172.26.192.1 的 TCP/UDP 目标，共四个独立句柄，依次启动
阳性、受保护、阳性三个 DUT；peer/密钥保持。
本场景内存 IPv4 栈允许 loopback 回包，唯一 WG peer 的 allowed_ips 仅为
198.18.0.1/32 和 172.26.192.1/32；宿主地址不加入 peer 内存栈本地地址。
无主机路由或地址变化、无新增 Go OS listener。Rust 在绑定前及每阶段核对已批准的
Default Switch GUID/IP/路由；漂移即失败，不自动选择其它宿主地址。

共同字段为 `v:1,run_id`；下面列出全部追加字段，未列字段与重复/跨场景操作都拒绝：

```text
父→子
init_reject: op, dut_private_key, peer_private_key, token,
             virtual_tcp_port, host_tcp_port, virtual_udp_port, host_udp_port
begin_phase: op, phase（整数 1→2→3）
probe_local: op
finish_phase: op, dut_stopped:true
shutdown: op, dut_stopped:true

子→父
ready: event, udp_port, peer_public_key, dut_public_key, selftest:{tcp:true,udp:true,icmp:true}
phase_ready / phase_stopped: event, phase
bootstrap: event, phase, requests:1, response_status:204, rx_tcp_packets, tx_tcp_packets,
           authenticated:true, response_acked:true
local_probe: event, phase,
             cases:[{case_id:1..4, sent:bool, equal_echo:bool, error:enum}],
             icmp:{sent:3,received:3,id:9,sequences:[1,2,3],payloads_valid:true,addresses_valid:true}
failed / stopped: 同原场景的固定字段
```

四端口必须非零、互异、非9090，且绑定 peer 后核对非 peer 协议端口。旧 tcp_port/udp_port
二字段帧、host_ip 输入、未知字段和混合场景一律拒绝。固定四格目的依次为：
198.18.0.1:virtual_tcp_port TCP、172.26.192.1:host_tcp_port TCP、
198.18.0.1:virtual_udp_port UDP、172.26.192.1:host_udp_port UDP。
虚拟目的由固定 DUT 映射到对应 loopback 目标，宿主目的保持原地址。

`error` 仅 none/refused/reset/eof/timeout；其它错误、阶段/全局截止、未见实际 tun 提交或
错误报文都发送 failed，不当作预期拒绝。cases 恰为四个、按 case_id 1..4 排序。
sent 表示有效 TCP SYN/UDP 请求实际经过 tun 出口；equal_echo 另要求完整应用回显和
双向报文边界一致。它们不证明静默请求逐个被 DUT 认证接收，Go 不发整组“拒绝 PASS”。
Rust 必须合并前后阳性、负向目标零到达、实际资源与 DUT API 健康证据。

顺序为 init_reject→ready；每阶段 begin_phase→phase_ready→bootstrap→probe_local→
local_probe→finish_phase→phase_stopped，三阶段完成后 shutdown→stopped。每阶段新 DUT
主动 HEAD `/task009-wg?token=<token>&phase=<1|2|3>`，完整 204/ACK 后才探测，借已认证
流量学习新 DUT WG 端口。旧阶段观察器不复用成功计数，累计 1MiB 硬限额不重置。
探测载荷恰为 token 的 16 字节、phase 单字节、case_id 单字节及两个零字节。
每格只建一个流，无业务重试；TCP 协议重传不当作独立业务。四格后内部运行三次 ICMP；
ICMP 的内存 token 末字节按 phase 异或，防止旧回复满足新阶段，这不改变外部 schema/目标。

本场景每阶段最多 40 秒，全局工作 135 秒，helper 硬截止 150 秒、父最终 159 秒、外层
160 秒；单次 connect/read/write 仍最多 2 秒。原 init 和 init_udp 仍使用工作 30 秒、
helper 55 秒及外层 60 秒，不能借新场景延长期限。失败取消业务并 Hold；只有确认 DUT
停止才可 finish_phase 或 shutdown，未确认则保留资源至硬退出并记录清理失败。

单测另覆盖四格独立纯内存目标、逐格固定地址/端口/载荷、目标关闭、错误载荷/四元组/校验和、假发送、阶段计数重置、
旧 ICMP、缺 bootstrap 与资源保留。它们不代表真实 Router 拒绝。DNS、非宿主转发、IPv6
仍未覆盖，DCR-011/012 不构成完整 SF-003 验收。

## DNS 结果预检（DCR-013）

`init_dns_probe` 与 `init` 使用相同的六个输入字段，只更换 op；不接受端口、phase、
URL 或地址字段。执行 ready 前的既有纯内存自测，实际 peer 仍仅绑定 loopback WG UDP。
本模式不创建 HTTP listener/UDP service，也不运行 TCP/UDP/ICMP 业务；唯一正常顺序是：

```text
init_dns_probe → ready → shutdown(dut_stopped:true)
→ stopped(resources_closed:true, discarded_packets:u32, discarded_bytes:u32) → exit 0
```

WG 认证解密后的每包只检查 offset 和长度 1..1280，累计最多 64 包、81920 字节，然后
丢弃；不调用旧业务 observer、不送入内存 IP 栈、不产生应答或转发。计数由同一锁保护，
越界后固定失败并进入原 Hold；WG/bind 错误继续传播。只在完整关闭后输出稳定最终计数，
允许 0 包/0 字节。计数不证明握手、目的 IP、DNS 结果使用或 Host/SNI。

30 秒工作窗口、55 秒硬截止及父先确认 DUT 停止的规则保持。EOF、错误帧、模式串用和
未确认停止都不能成功退出；失败后保留 peer 端口等待有效 shutdown，最终仍退出非零。
其它模式的 stopped 不带丢弃计数，仍要求原业务结果。定向纯本地检查可执行：

```powershell
go test -v -count=1 -timeout 60s -run 'TestDNSProbe|TestProtocolFailureHoldsPortUntilShutdown' ./...
```

这些测试包含无业务正常停止、模式串用/Hold、无服务、计数上限、错误传播、禁止入栈、
并发计数、关闭后迟到写入及读侧取消；不启动 sing-box，也不是实际 DNS 预检运行证据。

## 域名 HTTP Host 与 TLS SNI（DCR-015）

`init_domain_http` / `init_domain_tls` 只使用原 init 六字段，不接受目标参数。
仅在这两个模式的内存 NIC 增加固定198.20.0.255/32；各自唯一内存TCP listener为
18080/18443。源固定198.18.0.1，首SYN锁定唯一连接，允许其重传和结束报文，
错误地址/端口/协议、第二连接、超限或后续失败均保持失败，绝不转发到OS网络。

HTTP只接受HEAD `/task009-wg-domain`、Host `veyra.disign.me:18080`、无body/TE、
最多16384字节的唯一请求；完整固定204字节和同连接累计ACK后输出domain_http。
TLS通过标准库GetConfigForClient验证SNI `veyra.disign.me`，用私有哨兵终止握手。
输入最多16384、输出最多4096字节；底层错误/短写/超时独立锁存，不能被哨兵掩盖。
domain_tls只证明SNI，`https_success:false`；没有证书、CA或跳过证书验证设置。

两模式stopped只增加对应mode及原resources_closed，零连接取消也可确认清理；
exit0另外要求唯一成功事件和无迟到失败。工作30秒、硬截止55秒、单I/O2秒和先确认DUT
停止再shutdown的Hold要求保持。ready的selftest仍只代表旧栈自测；新别名HTTP/TLS
阳性与负例由TestDomain本地内测覆盖，不能代替新child DNS链和真实DUT证据。
