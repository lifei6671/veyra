# TASK-009：最小 WireGuard 测试资源提案

状态：**PROPOSED，未授权、未实现、未运行**。2026-09-04。
本文件只为下一步资源决定提供具体对象，不修改 Requirement、Task、DCR 或 Gate。
采用 technical-design Skill 的有界方案模式；完整 WG/DNS 验收仍留在 SF-003。

## 当前身份与结论

- TASK-009：`sha256:a1341b15a48840a5e15616eba486ff79a4a432c1e7e3c54be2f978b8632093cd`。
- 输入 `network-prerequisites.md`：`sha256:6297797be5627fade5a4aa77af253e261dfec0922e74992ba3b49b06ba36c1a5`。
- DCR-003：`sha256:3040951342ef465fb177b3abb1a7537d73c55bc87e88e1a7b3ba93677033b26d`。
- DCR-008：`sha256:689ee9ff2cebfd499e27646c7ea7863b1898e2b5afe739482cee6a57ad06d41f`。
- 实际只读复核固定 EXE SHA-256 为
  `aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc`。
  `go version -m` 确认 1.14.0、Windows amd64、Go 1.26.7，revision
  `0b8995879f29a9b98ee027bc17b75e101445b238`，包含 with_wireguard/with_gvisor。

固定 sing-box 继续作为 DUT（被测内核）。仅再启动一份固定 EXE 不能提供全部测试端能力：
已有 URLTest 只主动建立 TCP；`tools connect` 不驱动当前受管 child，且不足以启动 WG endpoint；
没有已核定的 ICMP 注入与解密后逐包计数入口。因此不申请第二份 sing-box 作为完整 peer，
改申请独立、只用于测试的 Go peer。该结论沿用输入前提中的固定源码核定。

## 本次请求授权的最小范围

1. 创建独立测试目录 `scripts/task009-wg-peer/`，含 Go 源码、`go.mod`、工具生成的 `go.sum`
   及简短运行说明；只用于此测试，不加入产品打包、Cargo、npm、CI 默认流程。
2. 下载并编译下表两个固定直接依赖及正常传递依赖；使用现有 Go 1.27.1 Windows amd64，
   不自动安装/切换工具链。模块下载可访问官方模块分发/校验服务，测试业务不访问外网。
3. 在现有 TASK-009 crate 内受控网络内测中接入该 peer；保持生产 Compiler 和受管链不变。
   首批仅验证真实握手、WG TCP 出站应答、虚拟地址 ICMP 例外、资源归属及清理。
4. 允许固定 DUT 在单用例期限内产生 WG 协议 UDP socket，**包括尚未排除的全接口绑定**；
   测试 peer 自己的 UDP socket 必须精确绑定 `127.0.0.1:动态端口`。
   不新增防火墙规则、不关闭防火墙、不创建 TUN、系统接口、路由、DNS、代理或服务。

这四项是一个可独立交付的资源与首批验证范围，不是完整 SF-003 已具备运行条件的承诺。
批准后仍先形成对应设计/Scope 候选并独立审查，再实施；本文件本身不冻结设计。

## 精确依赖及用途

| 独立 Go 模块的直接依赖 | 固定版本 | 用途 |
| --- | --- | --- |
| `github.com/sagernet/wireguard-go` | `v0.0.5-0.20260823125007-8bd032a91a30` | 复用 WG 握手、加解密、密钥路由和设备生命周期；不自行实现密码学 |
| `github.com/sagernet/gvisor` | `v0.0.0-20260727.0-sing-box-mod.1` | 内存 IP/TCP/UDP/ICMP 栈、可计数包边界及 TCP/UDP 测试服务；不自行实现 TCP 栈 |

两项版本均来自本次固定 EXE 的 `go version -m`，不是猜测或 latest。
上游对应 `go.mod` 的 module 名称也已核对。它们是两个直接依赖，不代表只有两个下载包；
正常传递图须由 Go 工具解析并保存 `go.sum`，实施记录实际 `go list -m all` 与校验结果。
不能宣称新 harness 的传递版本与固定 EXE 全部一致。任何直接依赖增加/升级须另行决定。
本次未下载依赖、未创建清单/锁文件、未进行兼容性编译，Windows harness 能否直接构建仍为 NOT_RUN。

固定 wireguard-go fork 没有已核定的 `tun/netstack/tun.go` 现成包装（定向读取返回 404）。
实际最小接线工作是实现内存 `tun.Device`、只绑定 loopback 的 `conn.Bind`，用 gVisor
`channel.Endpoint` 连接报文队列，用 `gonet` 建立 TCP/UDP 服务；ICMP 用栈 endpoint 收发。
这些测试胶水的 Close/取消/队列所有权需要独立审查，不能称为“直接调用现成完整 peer”。
无需导入 sing-box 主模块、sing-tun、系统 raw socket、Wintun 设备或安装驱动。
wireguard-go 自身模块列出的间接 Wintun 包不代表授权调用系统 TUN 创建入口。

替代方案是前提文档中的 Rust boringtun/smoltcp 候选，会修改 Cargo 依赖图；本提案选择
独立 Go harness，复用固定内核已有的库版本，但新增 Go 测试维护和传递依赖成本。

## 首批可运行拓扑与客观断言

```text
受管固定 DUT：虚拟 198.18.0.1/32
        ↕ WireGuard 加密 UDP，仅向 127.0.0.1 的自有 peer 发送
Go peer：127.0.0.1:动态端口；内存虚拟 198.18.0.2/32
        ├─ TCP HTTP 服务：精确 HEAD 目标、一次请求、204 应答及计数
        ├─ ICMP 注入/接收：固定 ID、序号和随机测试 token
        └─ TCP/UDP/ICMP 包计数：只存用例字段，不落原始载荷
```

虚拟地址只存在于两个用户态栈，不配置到 Windows；首批仅 IPv4，不能据此声称 IPv6 通过。
DUT 使用正常强类型 RuntimeIntent、WG Pool 与 ObservationOnly 编译/finalize/check/run，
节点服务器为 peer 的 loopback IP；探测 URL 为 peer 虚拟 IP，不引入域名解析。
peer 先持有 UDP socket，再启动 DUT；原生 URLTest 发起握手和 TCP HEAD。
peer 从认证成功的会话确认 DUT 端点，不能把未经认证的首个 UDP 来源当成对端身份。

- TCP 阳性控制：peer 的 HTTP listener 必须真正收到精确方法/目标并应答；DUT URLTest
  成功、解密后双向 TCP 计数和单次请求记录相互对应。只见握手不算 TCP 成功。
- ICMP：在已认证同一 WG 会话中向 DUT 虚拟地址发 Echo，验证 Reply 类型、ID、序号、
  token、源/目的地址及精确数量；超时为失败，不能用 TCP 成功替代 ICMP。
- 计数绑定本次 peer 及 DUT 身份/配置 hash。先自测内存栈 TCP/UDP/ICMP 往返，再测真实
  加密链，防止测试端本身完全不通造成假阴性；自测不算 DUT 验收。

## 拒绝、UDP 出站与 DNS：明确剩余边界

peer 具备发起 TCP、UDP、ICMP 的能力后，可以构造已认证的主动流，但拒绝证据还必须
同时具备目标计数、正确注入证明和同拓扑阳性控制。后续设计须逐项处理：

- 虚拟地址/宿主 loopback TCP、UDP：受控 echo listener 在 DUT 生命周期内一直持有
  地址与端口；控制请求先成功并计数归零后，再发送唯一 token 的拒绝流。
  普通宿主直连成功只证明目标活着，不自动等同经过 DUT 的路由阳性控制。
- 主动 DNS：可在内存生成固定格式 DNS 报文，但需证明进入相应 DNS 路径以及目标计数；
  不能把一份 UDP/53 超时直接登记为 DNS reject PASS。
- 受控外部目的转发：仍缺少已冻结、不会落到 Windows 默认路由/公网的目标拓扑。
  不能仅向一个不存在的文档地址发包，以超时代替拒绝；第三虚拟栈或额外测试路由的
  阳性控制必须单独具体设计，首批不执行该项，也不删去 SF-003 要求。
- DUT UDP 出站应答：正常 ObservationOnly 与现有 URLTest 均没有 UDP 业务注入入口。
  DCR-008 只批准 Direct TCP 计量且排除 WG，不能复用其授权扩大为 WG UDP。
  后续若采用单一 loopback UDP 入口、固定 peer 虚拟地址/回显端口、只路由至指定 WG，
  必须另行列出强类型测试白名单及 DCR；本提案不授权添加入口或移除任何 reject 规则。
- 节点 hostname 系统 DNS、WG 目的域名本地解析：仍缺少用户控制的域名及可观测系统解析
  资源。`localhost`、hosts、更换 DNS、外部公共域名或只看 SNI 均不能补该证据。
  DNS 资源另行提出具体问题；本次不捆绑系统 DNS、域名、证书信任库或 VM 修改。

## 实际监听范围、期限与失败清理

上游普通 `StdNetBind` 明确绑定 `:port` 和 `[::]:port`。固定核心单 IP peer 分支调用
`SetSinglePeerMode`，但声明依赖 commit 无对应方法的既有不一致仍未解决。
因此不能从 loopback peer 地址推断固定 EXE 的实际本地绑定；本次也没有启动探测。
拟议运行必须按已持有的 DUT PID 只读枚举 UDP 地址/端口，并记录结果。

可能的全接口 UDP 暴露在首次启动即发生，事后检测不能消除这段暴露。因此第 4 项明确
请求这项短时资源边界；并不请求全接口业务 TCP/UDP 服务。未授权该边界则不运行 DUT WG。
新随机密钥、唯一 peer、既有置顶 route/DNS reject 限制已认证流；外部仍可能触达 WG
握手处理面，不能说风险为零。不修改防火墙来假造隔离；需更强隔离时另核定环境。

- 每个真实用例外层 60 秒；harness 工作窗口 30 秒；每次业务 I/O 最多 2 秒且继承
  工作窗口的绝对截止时间。WG 内部协议定时由库处理，但取消后不得延长全局期限。
- 首批至多一个 DUT、一个 peer；一条 HTTP 流、三次 ICMP，业务累计上限 1 MiB，
  内存队列有界。异常、端口冲突、未知 socket 或无法确认归属立即终止，不自动重试。
- peer 不设管理端口；仅使用父进程私有管道传入本次密钥/资源参数，stdout 只出固定
  结果摘要。密钥从密码学随机源生成，仅驻留内存及既有 ACL 私有 DUT 配置，不传命令行、
  环境变量或普通日志，不输出完整配置。Go GC 无法保证所有密钥副本即时擦除，如实保留此限制。
- 结束或达到工作期限后停止发起业务，父进程先执行 DUT Stop 并确认退出；peer 此时
  仍持有端口和服务。随后关闭业务 socket、内存栈、WG device/bind，并等待自有 goroutine。
  父进程有界等待且只终止其创建、仍持有句柄的 helper；沿用 DUT 私有目录清理规则。
- 不在 DUT 存活时先释放 peer 端口，避免被其它进程重占。若 helper 意外提前退出，
  立即判失败并停止自有 DUT，不能继续用例或宣称归属连续。清理未确认则保留归属记录
  和私有资源；只在确认停止后删除解析路径位于本次自有根内的文件。
- 前后只读比较接口、地址、路由、DNS、代理配置；不因失败自动恢复系统设置，不操作
  其它 sing-box/peer PID。结果绑定固定资产、最终配置、DUT/peer 身份和实际 socket 清单。

## 本次只读证据与下一步

已执行：固定 EXE hash/构建元数据、现有 Go 版本、上述 live 契约及定向上游源码读取。
未执行：依赖下载、Go 编译、任何新 peer/core/listener、网络用例或主机设置变更。
首批能力预计可实施，但兼容性和实际 UDP 绑定必须运行核实；完整拒绝/UDP/DNS 拓扑仍不完备。
需要用户决定的是“两个固定测试依赖 + 独立 peer + 首批 TCP/ICMP 验证 + 短时可能全接口 WG UDP”，
不能将一次批准解释为已同意 UDP 业务入口、外部网络、DNS 修改或降低完整验收要求。

一手来源（均为本次定向读取，不是运行证据）：

- [wireguard-go 固定模块声明](https://raw.githubusercontent.com/SagerNet/wireguard-go/8bd032a91a30/go.mod)
- [gVisor 固定模块声明](https://raw.githubusercontent.com/SagerNet/gvisor/v0.0.0-20260727.0-sing-box-mod.1/go.mod)
- [WireGuard Bind 接口](https://raw.githubusercontent.com/SagerNet/wireguard-go/8bd032a91a30/conn/conn.go)
- [WireGuard 内存设备接口](https://raw.githubusercontent.com/SagerNet/wireguard-go/8bd032a91a30/tun/tun.go)
- [标准 UDP 绑定](https://raw.githubusercontent.com/SagerNet/wireguard-go/8bd032a91a30/conn/bind_std.go)
- [固定核心 WG 启动分支](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/transport/wireguard/endpoint.go)
- [gVisor 内存包通道](https://raw.githubusercontent.com/SagerNet/gvisor/v0.0.0-20260727.0-sing-box-mod.1/pkg/tcpip/link/channel/channel.go)
- [gVisor TCP/UDP 适配器](https://raw.githubusercontent.com/SagerNet/gvisor/v0.0.0-20260727.0-sing-box-mod.1/pkg/tcpip/adapters/gonet/gonet.go)
