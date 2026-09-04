# TASK-009 WireGuard 测试端

仅供 DCR-009/010/011 的 Windows 受控测试使用，不是产品组件或独立代理服务。
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

## 三阶段本地拒绝对照（DCR-011）

`init_reject` 仅增加四格：virtual_tcp、host_tcp、virtual_udp、host_udp。Rust 全程持有
loopback TCP/UDP 两个目标，依次启动阳性、受保护、阳性三个 DUT；peer/密钥保持。
本场景内存 IPv4 栈允许 loopback 回包，唯一 WG peer 的 allowed_ips 仅为
198.18.0.1/32 和 127.0.0.1/32。无主机路由或地址变化、无新增 Go OS listener。

共同字段为 `v:1,run_id`；下面列出全部追加字段，未列字段与重复/跨场景操作都拒绝：

```text
父→子
init_reject: op, dut_private_key, peer_private_key, token, tcp_port, udp_port
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

单测另覆盖四格纯内存目标、目标关闭、错误载荷/四元组/校验和、假发送、阶段计数重置、
旧 ICMP、缺 bootstrap 与资源保留。它们不代表真实 Router 拒绝。DNS、非宿主转发、IPv6
仍未覆盖，DCR-011 不构成完整 SF-003 验收。
