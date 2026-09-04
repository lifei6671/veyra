# TASK-009：WG host loopback 阳性对照失败的静态诊断

日期：2026-09-04。
状态：STATIC_CONFIRMED / RUNTIME_PATTERN_CONFIRMED。真实 phase 1 四格分布吻合静态诊断；
DCR-011 四格阳性前提不成立，本用例 FAIL，phase 2/3 NOT_RUN。不是拒绝 PASS 或设计批准。
本记录只读核对源码与既有失败日志；未修改 Go/Rust、DCR、Task/state 或主机设置，未运行新资源。

## 输入与已发生的失败

- DCR-011：`54c01cbc8f99b080a2d68e5b999ab6fc9f97fe49b1ae30e53e36ae2d402f6641`。
- 固定核心为 sing-box 1.14.0 Windows amd64，SHA-256
  `aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc`；版本/模块核定沿用
  `network-prerequisites.md` 和当前受管测试资产核验，不以新编译核心替代固定 EXE。
- `src-tauri/target/task009-remediation/wg-reject-real-014.stdout.txt` / `.stderr.txt`：
  首轮真实用例 FAIL，15.21 秒；phase 1 收到完整 local_probe 后，`case.equal_echo` 断言失败。
  原日志没有输出四格 error/echo 分布，无法仅由这一断言确定是哪格失败。
  随后的 `peer cleanup incomplete` 是同轮清理结果，不抹去前面的阳性失败。
- 完整 local_probe 的实现前提包括四格 sent 及末尾 ICMP 3/3；它不证明每个静默请求被 DUT
  解密，也不能替代目标端接受/收到计数。root 补充摘要后的串行复验结果如下。

### 带四格摘要的真实复验

`wg-reject-real-diagnostic-014`：2026-09-04 18:03:02–18:03:18 +08:00，测试用时 15.47 秒，
Cargo exit 101，外层未超时。精确命令为：

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib real_wireguard_local_reject_has_positive_before_and_after_four_blocked_paths -- --nocapture
```

phase=1、reported_phase=1，摘要时 elapsed_ms=10594；bootstrap_acked=true，完整 local_probe
含健康检查。helper SHA `b22d775adda493abe3ce4ddf3f511009cd271babb5cd66617ae7293b613792e9`；
配置 SHA `0230a3d2899b1ae9a9d9adeb3b878155851c6f2a689ee10d61d2b0b9a0dc298e`，实例 identity=1。

| case | sent | equal_echo | error | 目标已核定载荷数 |
| --- | --- | --- | --- | --- |
| 1 virtual_tcp | true | true | None | 1 |
| 2 host_tcp | true | false | Timeout | 0 |
| 3 virtual_udp | true | true | None | 1 |
| 4 host_udp | true | false | Timeout | 0 |

目标 tcp_accepts=[1,0,0]、udp_receives=[1,0,0]，payloads=[[1,0,1,0],[0,0,0,0],[0,0,0,0]]，
active=0、failed=false；清理时这些计数保持。尚未进入受保护 phase 2 或后置阳性 phase 3。
虚拟两格成功证明目标并非全部关闭、WG 并非整体不可用；不能因此把 host 两格超时判成满足
当前要求的同路径拒绝对照。

DUT PID=38304、peer PID=82164；TCP目标端口12310、UDP目标50570、peer协议50571。
DUT Ready socket 为 TCP 127.0.0.1:9090 和 WG UDP [::]/[0.0.0.0]:60107。
清理 dut_stop_confirmed=true、dut_owned_empty=true、peer_exited=true、private_count=0；
peer exit=1、peer_cleanup=Err("peer cleanup incomplete")。未完成三阶段导致非零退出，
清理协议结果仍记 FAIL，不能因进程已退出和私有文件为空改写为完整正常清理。

原始证据位于 `src-tauri/target/task009-remediation/`，SHA-256：

```text
wg-reject-real-diagnostic-014.stdout.txt 3e2cf307531e630cdd063bc2faa60e77dff9d6589b06f3c2a79b790de04cc3ec
wg-reject-real-diagnostic-014.stderr.txt 0e8d904aec17d8cb31bb364f65e0b8f5e35867b79876c487f5306f848fe5b77b
wg-reject-real-diagnostic-014.result.json 85bd28a7244fd6150da018338485ae98e47f6d17ea8378737ed20b7926bb43d2
wg-reject-real-014.stdout.txt f06f6ed48e154d1319e7402efae3492e7fb8f71169052097c0b0473242730584
wg-reject-real-014.stderr.txt 6789aa398088e4147aeff0e6403425786c48596f4f02d3f4655f0d0c3cefdfa7
wg-reject-real-014.result.json 799e466da04ce47ef0a100c3980a24e25ed8a3e9d0b95423628a88b450bf7815
```

## 固定源码调用链与精确参数

1. [sing-box v1.14.0 transport/wireguard/device_stack.go](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/transport/wireguard/device_stack.go)
   第 45–52 行创建 wireEndpoint，通过
   `tun.NewGVisorStackWithOptions(..., stack.NICOptions{}, true)` 创建 DUT 内存栈。
   第 77–91 行才安装 TCP/UDP/ICMP forwarder。第 209–226 行在 WG 解密后的 Write 路径
   把报文交给网络层 `DeliverNetworkPacket`；不是直接跳入 Router。
2. [sing-box v1.14.0 go.mod](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/go.mod)
   第 42、56 行固定 gVisor `v0.0.0-20260727.0-sing-box-mod.1` 和
   sing-tun `v0.9.0-beta.4`。
3. [sing-tun v0.9.0-beta.4 stack_gvisor.go](https://raw.githubusercontent.com/SagerNet/sing-tun/v0.9.0-beta.4/stack_gvisor.go)
   第 204–205 行的第三参数明确是 `allowRawEndpoint`，不是允许外部 loopback；内部再调用
   `newGVisorStack(..., allowRawEndpoint, false)`。第 208–212 行选择默认 `ipv4.NewProtocol`；
   第 220–221 行只按该布尔值增加 raw.EndpointFactory。第 224 行使用普通 NICOptions；
   第 232–238 行的 spoofing/promiscuous 设置不修改 IPv4 的 loopback 选项。
4. 已下载的固定 gVisor 一手源码
   `C:/Users/lifei/go/pkg/mod/github.com/sagernet/gvisor@v0.0.0-20260727.0-sing-box-mod.1/pkg/tcpip/network/ipv4/ipv4.go`：
   第 2093–2096 行说明并实现 `NewProtocol` 等价于空 Options；第 2052–2054 行定义
   `AllowExternalLoopbackTraffic`。第 956–970 行在非 loopback NIC 且该选项为 false 时，
   对 127/8 源或目的递增非法地址统计并直接返回。目的检查在第 964–968 行。
   因而来自 WG、目的 `127.0.0.1` 的 TCP/UDP 尚未到运输层 forwarder 就被丢弃。
5. [sing-box v1.14.0 protocol/wireguard/endpoint.go](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/wireguard/endpoint.go)
   第 214–233 行 TCP、本地虚拟目的映射后才调用 RouteConnectionEx；第 235–254 行 UDP
   同样映射，并用 OriginDestination/目标封装 NAT 回写。目的为 `198.18.0.1` 的输入包
   不触发上述 127/8 网络层检查，因此可先到达映射逻辑；原始 host127 包则不能靠此映射获救。
6. [sing-box v1.14.0 route/rule/rule_item_cidr.go](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/route/rule/rule_item_cidr.go)
   第 73–88 行在普通 IP 目的时匹配 `metadata.Destination.Addr`，而非 OriginDestination。
   所以 DCR-011 的 127/32 阳性规则可匹配已经映射的虚拟目的，但不能跳过更早的网络层丢弃。

结论：`AllowExternalLoopbackTraffic` 只在本次 Go peer 新场景启用，保障它接收 host 源回包；
它不会更改固定 DUT 内存栈。固定 DUT 的 `true` 参数不能被解释为同一开关。单独调整 Router
规则或增加 host listener，无法使原始 `127.0.0.1` 隧道目的在该固定实现中抵达目标。

静态链预期的虚拟 TCP/UDP 阳性成功、host TCP/UDP 超时，已由上述四格与目标计数吻合。
该诊断不声称逐包观测到 DUT 内部丢弃计数；核心行为解释仍由固定源码与真实分布共同支持。

## 原路径是否有不增暴露的更小阳性方案

在固定 EXE、不改系统网络、不改变原 `127.0.0.1` WG 目的和同路径要求的边界内，未找到
可行方案。Router 例外位于丢弃之后；在 peer 上本地回显、通过另一业务入口直连目标、只证明
listener 可用、修改内存包的目的地址，都会绕过或改变待测路径。不能据这些对照宣称原 host
路径满足同路径阳性。修改 DUT 栈/核心二进制也超出固定资产契约，不能当作本轮局部修复。

可保留原始 host127 的“发送后未到达”作为客观现象，但当前契约要求同路径阳性，故它不能
独立成为该契约下的拒绝 PASS；如何调整验收解释必须显式进入设计/范围决定，不能自动删格。

## 后续最小可验证候选：宿主实际 IPv4

只读评估，PROPOSED / NOT_RUN；本记录不选取具体地址、不发请求、不授权新监听。

- 虚拟地址两格不变，继续抵达原 Rust 自有 loopback TCP/UDP 目标。
- host 两格改用经只读核定、确实分配给本机当前适配器的单个非 loopback IPv4；目标 TCP/UDP
  必须精确绑定该 IP，不能绑定 0.0.0.0。它是本机实际地址，不是本次新增地址、外部网站或
  可随意输入的目标。没有适合且稳定的本机 IPv4 时记 UNAVAILABLE，不创建接口或配置地址。
- 为保持虚拟路径及其真实目标不变，保留两个 loopback socket，另增两个精确宿主 IPv4
  TCP/UDP socket，均从 bind 返回动态端口。不能直接把原 listener 改绑实际 IP，因为虚拟
  目的仍映射到 loopback。是否可安全共用端口数值须在最终设计中核定，不以 wildcard 省资源。
- Go cryptokey 路由、探测目的/四元组观察及私有测试消息需支持该封闭、已核定的单地址。
  阳性测试 Compiler 需增加精确该 IP/32、各自端口和网络 → Direct 规则；阴性仍保留正常
  置顶 reject，并保留后续可命中的用户规则。不能将其泛化为任意 IP、范围或配置输入。
- 该目的不是 127/8，故避开已确认的网络层 loopback 检查；阳性仍必须通过实际固定 DUT、
  Router、相同目标服务、返回 WG 及应用精确回显验证。此为可行性推断，不是执行结果。

新增暴露和授权差异：精确绑定实际网卡 IP 的目标可能被局域网/已有路由可达主机访问，
范围超过原本仅本机可达的 loopback 资源。随机 token 与固定短回显限制服务内容，但不能阻止
其它主机建立 TCP 连接或发送 UDP、消耗有限资源并造成测试失败；不能声称无网络暴露。
不更改防火墙或假定现有防火墙一定阻断外部访问。目标只处理本次有界载荷、无代理/递归/持久
业务；继续精确 PID/socket 所有权、三阶段计数、绝对截止、先 DUT 后目标/peer 清理。

因此至少需要新的明确授权：具体适配器/IP 的选择规则、两个非 loopback 目标 listener、
新增可达面、测试目的与阳性规则/协议字段改变，以及同路径配对证据的对应调整。必须先形成
可审阅 DCR；不能在当前 DCR-011 授权下直接改地址或启动资源。无需新增依赖或系统设置。

本记录仅为诊断和候选资源披露，不迁移 Task/Gate、不改变冻结验收，也不宣称完整拒绝矩阵通过。
