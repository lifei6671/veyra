# TASK-009 受控网络验证前提与已实现子集

本记录为 `task009_network_contract` 的只读核定与有界测试实现交接。
核定日期：2026-09-04。网络测试运行结果由 Orchestrator 在实际执行后另行记录；
本文件不构成网络 PASS、Task 验收或范围变更批准。

## 输入身份与资源

- TASK-009：`sha256:54eac955afef162e457fc660106d97924b1323810773302a42b168028e50b244`。
- DCR-003：`sha256:3040951342ef465fb177b3abb1a7537d73c55bc87e88e1a7b3ba93677033b26d`。
- DCR-004：`sha256:6610b072db7f8044f5681140a9241c5da8e89e4ceb80a78d56ae2674ff3542b7`。
- 固定 EXE 的只读 SHA-256 核对匹配
  `aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc`。
- `go version -m src-tauri/binaries/sing-box-1.14.0-windows-amd64/sing-box.exe`
  确认 sing-box 1.14.0、Windows amd64、Go 1.26.7、with_wireguard/with_gvisor，revision
  `0b8995879f29a9b98ee027bc17b75e101445b238`，vcs.modified=false。
  WireGuard 依赖为 `github.com/sagernet/wireguard-go v0.0.5-0.20260823125007-8bd032a91a30`。
- 专家核定未启动核心、peer 或 listener，未安装依赖，未修改系统网络。

## 强制用例与资源矩阵

| 用例 | 拓扑与客观断言 | 前提核定 |
| --- | --- | --- |
| WG 虚拟地址 ICMP Echo | 内存网络栈 peer 通过加密 UDP loopback 向 DUT 虚拟地址发 Echo；核对 Reply 的 ID/序号/payload | UNAVAILABLE：现有核心工具没有 ICMP 注入入口，尚无受控 peer |
| 主动 TCP/UDP/DNS 与转发拒绝 | 同 peer 发往 DUT 虚拟地址、宿主 loopback 与受控虚拟外部目标；阳性控制先成功，目标计数为零并观察拒绝 | UNAVAILABLE：需完整 peer 与可观测目标；单独超时不算拒绝证据 |
| WG TCP 出站应答 | DUT URLTest 经 WG 到 peer 内存 HTTP 服务，记录请求/应答及实例身份 | NOT_RUN：先补 peer 及实际 UDP socket 绑定边界证据 |
| WG UDP 出站应答 | DUT 主动建立 WG UDP 五元组，peer 应答，DUT 确认收到 token | UNAVAILABLE：当前受管 ObservationOnly 没有 UDP 业务流输入；仅新增 peer 不能补足 |
| HTTP/SOCKS5 目的域名、HTTP Host | DUT 节点指向 loopback mock；URL 使用合成 `.invalid` 域名；记录 CONNECT authority 或 SOCKS domain 地址，再核对 HEAD 的 Host/path/query | 已实现子集，专家未运行 |
| TLS 身份 | 同 tunnel 中读取有界 ClientHello，核对 SNI 与原 URL hostname 一致 | 已实现 SNI 子集，专家未运行；不等于可信 TLS 握手成功 |
| 节点 hostname 系统 DNS、WG 目的本地解析 | 受控系统 DNS 查询计数与 peer 所见目的 IP，对比 HTTP/SOCKS 所见域名 | UNAVAILABLE：没有已授权且可观测的系统 DNS/域名资源 |
| 系统接口/路由/DNS/代理不变与资源归属 | 前后只读快照；仅核对已知 DUT/harness PID 的 socket；清理后归属资源消失 | 现有资源可做，须绑定实际运行另记证据 |

`localhost` 的特殊本地解析不能代替一般系统 DNS 查询证据。TLS ClientHello 中正确的
SNI 也不能代替证书校验与完整握手。所有未执行强制项继续留在 SF-003，不能迁出本 Task。

## 固定版本的能力证据

1. 原生 URLTest 明确调用 `DialContext(..., "tcp", 原 hostname)`，然后用原 URL 建立 HEAD；
   因而不能生成 WG UDP 阳性业务流。
   [URLTest 固定源码](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/common/urltest/urltest.go)。
2. `tools connect -n udp` 创建自己的 Box 并只调用 PreStart。该阶段没有 endpoint 的
   Start/PostStart；WG 的 started 标志在 PostStart 才建立。它不是当前受管 child 的 UDP 输入。
   [工具入口](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/cmd/sing-box/cmd_tools.go)、
   [Box 生命周期](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/box.go)、
   [WG endpoint](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/wireguard/endpoint.go)。
3. 普通 WG `listen_port` 使用 StdNetBind.Open，声明依赖的实现绑定 `:port` 和 `[::]:port`。
   peer 地址为 loopback 不会把这类监听变成 loopback。
   [固定依赖 bind](https://raw.githubusercontent.com/SagerNet/wireguard-go/8bd032a91a30/conn/bind_std.go)。
4. IP 单 peer 分支有一手来源不一致：核心固定 revision 调用 SetSinglePeerMode，声明依赖
   commit 的 conn 目录却没有该方法；EXE metadata 也声明同一依赖。因此不能只根据方法名
   推定固定 EXE 的实际 UDP 绑定范围。本核定未执行启动探测，不把该不一致判成运行故障。
   [核心传输源码](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/transport/wireguard/endpoint.go)。

## 已核定并实现的 HTTP/SOCKS 子集

Orchestrator 授权仅在 `src-tauri/src/singbox/mod.rs` 添加 Windows 库测试模块
`task009_controlled_network`。不新增生产接口、不创建外部 integration target。
外部 integration 原本不能调用 pub(crate) finalize、ApiSecret、Windows Port 或 controller；
采用 crate 内测试后沿用这些封闭边界。

四个测试覆盖 HTTP/SOCKS5 各自的 HTTP Host 与 TLS SNI：

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib task009_controlled_network -- --nocapture
```

- 路径为真实 Parser → normalize → AppState/RuntimeIntent 整体验证 → Compiler → 每实例
  新 secret/finalize → Windows 私有配置与固定核心 check → Ready → 原生 URLTest → stop。
- 不修改最终 JSON、不添加 inbound、DNS 覆盖、系统配置、依赖或外部请求；测试节点服务器
  是 mock 的 `127.0.0.1:动态端口`。合成 `.invalid` 目的名由 mock 直接处理，不转发到网络。
- HTTP mock 只接受精确 CONNECT 目标；SOCKS5 只接受 CONNECT + ATYP=domain + 精确主机/端口。
  HTTP HEAD 校验固定路径、query、Host 并返回 204；TLS 只读 ClientHello 后关闭连接。
- 模拟端只有一个 loopback TCP listener，单连接，全局期限 20 秒，单次 I/O 最多 2 秒，
  HTTP/TLS 消息最多 16 KiB。每个测试必须有外层 60 秒上限；四项批量执行由主代理设置总期限。
- 复用 FIXED_CLASH_API_TEST_LOCK，避免并行占用 9090；子代理不运行核心，主代理统一串行验证。
- 固定三项资源在仓库 target 下唯一目录建立硬链接；Windows 私有配置使用系统临时目录下
  唯一 app-data 根并沿用 ACL/reparse 检查。仅清理自身目录；stop 未确认成功时保留目录。
- 所有运行结果在调用 stop 后才断言；peer thread 有界 join。记录最终配置 hash、opaque
  child identity 与用例类别，不记录 secret/私有完整配置/原始 TLS 载荷。
- 不调用观测采样接口，避免把其他已知观测缺口混入目的域名证据。真实 PID、系统快照、
  完整 TLS 成功、WG 和系统 DNS 均不由这四个测试宣称覆盖。

专家完成 rustfmt 和 scoped git diff --check；编译与真实运行均由主代理另行执行并记录。

## 完整 peer 的最小可审阅候选（未批准、未安装）

- `boringtun = "=0.7.1"`，关闭默认功能，只用 noise::Tunn 加解密；std UdpSocket
  精确绑定 loopback，不启用 device、TUN 或驱动。
  [版本清单](https://raw.githubusercontent.com/cloudflare/boringtun/boringtun-0.7.1/boringtun/Cargo.toml)、
  [Tunn API](https://raw.githubusercontent.com/cloudflare/boringtun/boringtun-0.7.1/boringtun/src/noise/mod.rs)。
- `smoltcp = "=0.12.0"`，关闭默认功能，仅 std、medium-ip、proto-ipv4、proto-ipv6、
  socket-tcp、socket-udp、socket-icmp。自有内存设备生成/接收协议流，不启用 raw_socket、
  tuntap、DHCP。避免自行重写 WireGuard 密码学或 TCP 栈。
  [版本与功能](https://raw.githubusercontent.com/smoltcp-rs/smoltcp/v0.12.0/Cargo.toml)。
- 影响：新增测试及传递依赖，需要将 Cargo 清单/锁文件正式纳入变更控制；不新增产品依赖。
  该候选只补 peer 能力，不能解决当前 DUT 主动 UDP 输入缺口。
- DUT 若需独立验证 profile 的 loopback UDP 输入，会改变当前同一 ObservationOnly 最终配置
  的验收前提，须由 Owner 决定并调整冻结契约；不可偷偷追加 inbound/raw JSON。
- 系统 DNS 仍需明确受控域名及可观测解析资源。隔离 VM/DNS 等替代拓扑也属于待核定新范围，
  不能据此更改宿主 hosts、DNS、路由、代理或信任库。
