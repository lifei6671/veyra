# DCR-015：WireGuard 域名请求与 Host/SNI 受控验证

状态：PROPOSED；待独立技术审阅及本文件所列资源/Scope 扩展批准。
TASK-009 / SF-003；模式 remediation。设计与审阅不代表实现或网络验证通过。
Requirement：`docs/veyra.md`，sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45。
Task：sha256:681d94863c9f7bcc62451447e6cd7073ec139d02bd42ba04332235f0dcdcb790。
实现基线 checkpoint021：sha256:95d90d9614a5b05b583d3c2d20cd0d122d3876080b6fb676a0bbd797eb89e083。

## Problem 与 Current Frozen Design

SF-003 验收4要求需要 IP 的 WG 成员使用系统解析，HTTP Host 与 TLS 身份保持 URL 域名。
checkpoint021 已由固定 child 实测 `veyra.disign.me` 的 local DNS exchange 返回
`198.20.0.255` / `fc00::fe`（TTL1），但 peer 仅丢包，尚无请求到达或 Host/SNI 证据。
这些是当时实际答案，不是固定权威 DNS 记录，也不能推断下一 child 必然取得同一答案。

DCR-009 的内存 HTTP 仅服务198.18.0.2；DCR-013/014 的 DNS 模式明确禁止业务服务，
其采集资格只允许 `/task009-dns-preflight`。因此不能把旧批准解释为新地址服务授权。
本候选新增两个封闭业务模式，并扩展仅测试日志资格；旧模式及其证据保持原语义。

## Proposed Change 与完整拓扑

两个用例串行且每个新建 DUT、peer、密钥、API secret 和 run_id：

| 用例 | 唯一 URLTest URL | peer 内存 listener | 证明目标 |
| --- | --- | --- | --- |
| HTTP | `http://veyra.disign.me:18080/task009-wg-domain` | 198.20.0.255:18080 TCP | HEAD、原 Host、完整204响应及累计ACK |
| TLS | `https://veyra.disign.me:18443/task009-wg-domain` | 198.20.0.255:18443 TCP | 真实 ClientHello 的 SNI |

WG 外层仍是 helper 自有 `127.0.0.1:动态UDP端口`，DUT虚拟源198.18.0.1/32、
peer198.18.0.2/32、MTU1280；仅在新模式给 Go 用户态 NIC 加固定198.20.0.255/32别名。
每次只创建表中一个内存 TCP listener；该地址不绑定 Windows 网卡或 OS TCP socket。
不创建内存 IPv6 NIC/地址，不尝试绑定 fc00::fe，不改 DUT 的 IPv4-only local_address。
固定内核缺少 IPv6 本地地址时直接返回错误，原生地址尝试由核心决定；本次不修改
DNS family preference、不过滤 AAAA、不新增 harness fallback。最终没有IPv4请求则本例失败。

每个 DUT 仍仅一个 WG 节点，server=127.0.0.1、端口来自本轮peer归属证据，唯一 URLTest
Pool、interval300s/tolerance50ms、零业务 inbounds/用户route。正常 Compiler 生成唯一local
`dns-system`、final/default_domain_resolver和置顶WG route/DNS reject，不能覆写JSON。
不调用第二次delay，不重试候选，不用公网服务器、用户代理API或HTTP/SOCKS中转。
业务包只在WG用户态栈和本机peer之间；helper不调用OS业务Dial、不转发到外部网络。

HTTP固定HEAD路径，不含token/凭据；只接受Host=`veyra.disign.me:18080`、无body/TE的
一个请求，头部总长最多16384字节。新生成WG密钥与私有父子管道提供本次身份隔离。
响应复用原固定204字节，记录完整响应字节被同一TCP四元组的DUT累计ACK覆盖，
不能以服务端Write成功或收到连接代替出站应答成功。

TLS只收集ClientHello，不发证书、不装CA、不关闭证书校验，也不要求HTTPS成功。
使用Go标准库`crypto/tls.Server`的GetConfigForClient检查`ClientHelloInfo.ServerName`
精确等于`veyra.disign.me`，随后返回私有固定哨兵错误终止握手。只有该回调确实执行、
SNI正确且HandshakeContext以该哨兵原因退出时记SNI观察成功；超时、EOF、其它TLS错误
不能归一化成功。通过有界net.Conn计数限制ClientHello输入总量16384字节、输出4096字节，
单次I/O2秒与工作绝对期限共同约束。在这个既有拟用包装中锁存首个底层读写错误、短写、
超限和期限失败；最终成功另要求没有锁存失败且所有相关期限尚未到达。Go标准库回调
报错路径会忽略sendAlert写错误，故不能仅看HandshakeContext返回哨兵；主动结束握手
只豁免该预期TLS结果，不豁免alert的写失败或超时。不增加独立观察器框架。
只报告匹配布尔、字节计数和固定失败分类，不存原文。
该用例的URLTest预期不产生成功延迟；报告字段明确`sni_observed=true, https_success=false`。
SNI是SF-003允许的TLS身份客观证据，不宣称证书链验证、HTTPS响应或IPv6业务已通过。

## DNS 采集与最终字节资格

沿用DCR-013/014的私有有界stderr reader、固定`run --disable-color -c`、三字段log
`disabled:false,level:debug,output:stderr`；新增资格仅允许表中两个完整元组。
旧DNS预检资格和正常产品关闭日志/null标准流不变；check仍使用普通null标准流。
在现有compiler.rs测试方法新增封闭模式枚举，不能加入任意URL、host、IP或argv输入。
finalize与最终读回重验唯一endpoint/Pool、精确URL、原DNS/reject/无inbounds/无用户route、
IPv4地址/MTU、公钥/端口、log字段和全部普通结构约束；资格继续绑定pending最终digest。
不能只依赖路径后缀、某一个字段或“本测试传入true”取得采集资格。

每个新child从spawn开始最多10秒采集，沿用65536总字节、4096行、512块、128记录、
8个地址的硬上限，持续排空、同一owner、失败仍须先Stop再EOF/join的原语义。
仍只解析精确域名DNS白名单，不增加TCP/TLS原始日志收集。每个用例需本child新鲜的
Lookup→Exchanged NOERROR→A RR→LookupSucceeded链，A唯一198.20.0.255；AAAA如存在
只允许fc00::fe。同域出现其它结果、无A、矛盾或仅缓存命中均失败，不动态接纳新地址。
允许仅IPv4查询链成立，不要求没有IPv6本地地址的DUT必须发AAAA。
若结果漂移，停止并保留固定原因，回到资源核定；不自动改别名或要求用户改DNS。

成功条件是同一child的DNS链、peer已解密TCP目的198.20.0.255、该连接HTTP或SNI证据
及最终归属/清理同时成立。历史021结果只用于选择受控别名，不能代替新child DNS链。

## 私有协议、包观察与生命周期

沿用v1私有管道、原字段/重复字段校验、4096单行/16384会话上限、随机密钥/token规则。
新增init操作`init_domain_http`和`init_domain_tls`，字段与`init_dns_probe`相同，不接收
地址、端口、URL、TLS设置或服务参数。新模式只接收一次init和一次shutdown。
ready沿用原ready结构，明确原selftest字段只证明旧内存栈自测；新模式自测另由本地测试覆盖。

```text
ready → domain_http → shutdown(dut_stopped:true) → stopped → exit0
ready → domain_tls  → shutdown(dut_stopped:true) → stopped → exit0
```

`domain_http`只包含v/event/run_id以及requests:1、host_matches:true、response_status:204、
response_acked:true、destination_matches:true、authenticated:true、rx_tcp_packets、tx_tcp_packets。
`domain_tls`只包含v/event/run_id以及connections:1、sni_matches:true、https_success:false、
destination_matches:true、authenticated:true、client_hello_bytes、rx_tcp_packets、tx_tcp_packets。
两个事件均只能在对应模式产生，计数为正，TCP包计数各≤1024；ClientHello字节1..16384。
stopped沿用resources_closed:true，仅增加mode固定为domain_http或domain_tls，不含连接计数。
它只证明资源完整关闭，适用于ready后零连接取消、业务失败及正常完成，不证明业务成功。
正常exit0另要求唯一对应成功事件、唯一连接与无后续失败；取消、第二连接或其它业务失败
完成清理后仍为非零退出。零连接取消可输出合法stopped，不能伪造domain成功事件。
原有模式不接受新字段或事件。失败沿用固定failed(stage/code)，不得透传原错误或包体。

Go在解密Write/加密Read边界用新模式观察器匹配唯一IPv4 TCP连接，源198.18.0.1与
目的198.20.0.255和模式端口；首个合法SYN固定客户端端口，此后只允许该四元组及回复。
允许TCP重传、ACK、FIN/RST等该连接结束流量；不是把重复包当重复业务请求。
超过一个连接/请求、未知目的/协议/错误校验和/碎片/超限均报失败，不能进入OS转发。
沿用现有IPv4/TCP解析与校验函数；每方向最多1024包、合计≤1MiB、单IP包≤1280。
不扩展旧observer对任意地址的接受集合；新字段和分支只在两个新模式设置。
HTTP复用现有响应覆盖/ACK算法；TLS不套用HTTP ACK成功条件。

用例外层60秒；helper自出生55秒上限、ready后30秒业务窗口，父工作最迟helper出生45秒。
本轮业务/DNS等待止于min(DUT spawn+10秒, peer ready+30秒, helper出生+45秒)。
候选check10秒、Ready2秒；启动前必须有这些期限以及Stop/reader/peer清理的实际余量，
不足直接取消，不开始新child。业务等待期间每25ms响应reader/peer失败，不能阻塞CIM。
收到成功事件仍监视后续failed、重复事件和额外连接，直到DUT确认退出；迟到失败优先于成功。

沿用现有Hold：先确认DUT退出，再reader EOF/join，再shutdown/peer关闭资源和确认退出。
正常exit0必须观察到对应业务成功事件及stopped；取消/失败即使完成清理也不能业务PASS。
父异常或超时未确认DUT停止时不能发dut_stopped:true；保留owner/原Hold和绝对期限，
不得为新模式降低旧55秒/150秒模拟的停止所有权要求。所有线程、listener、WG设备、
PacketBuffer引用与私有配置逐一清理；未确认退出/清理时明确FAIL/RecoveryRequired。

## 文件范围、兼容与迁移

仅申请以下有界实现增量：

- `src-tauri/src/singbox/compiler.rs`：cfg(test)两元组编译/完整资格、最终读回及负例。
- `src-tauri/src/platform/windows/managed_sidecar_port.rs`：cfg(test)两真实用例、严格帧与
  生命周期接线，复用现有DNS采集资格绑定、身份和快照；必要接线处仍为条件编译。
- `src-tauri/src/singbox/managed_sidecar.rs`：只允许必要cfg(test)资格/现有采集接线内测，
  不新增日志类别、采集线程或产品访问器；若无需变化则不改。
- `scripts/task009-wg-peer/{main.go,protocol.go,stack.go,observe.go}`：新模式接线，固定
  内存别名与独立观察器；允许`domain.go`/`domain_test.go`集中服务、观察和本地用例，
  以及对应既有protocol/lifecycle/stack/observe测试与README。
- `.sdlc/evidence/TASK-009/`保存目标绑定的安全摘要、验证与审阅；批准后由Orchestrator
  同步Task scope/approval、state和HANDOFF，不改已接受验收。

不增改第三方依赖、go.mod/go.sum、Cargo/npm锁或构建资源；TLS使用Go标准库。
无产品API、存储、运行配置或系统设置变更、迁移；测试私有协议新模式双方同轮更新，
旧模式保持原严格schema，不添加兼容fallback。失败时移除本轮自有资源即可，不恢复配置。

## Risk、复杂度与替代方案

可信失败路径：动态DNS答案漂移会让固定内存目标收不到业务；旧observer会拒绝新地址，
无界或误判的TLS观察可能产生假PASS。现有WG私钥/loopback/期限/无OS转发已限制网络影响；
最小补充是两个封闭模式、一个固定内存别名、DNS与同连接交叉断言、标准库TLS回调限量。
这是测试运行资源及Scope扩展，需要Owner批准；不是因为假想攻击增加通用隔离系统。
工程影响限于既有三份Rust测试边界和Go测试端；新增一个文件集中新服务/观察，不引入
外部daemon、平台采集、底层driver或可配置代理框架。实作若发现必须改产品或依赖，停止重设计。

替代方案：只用198.18.0.2/IP URL无法证明域名语义；只收DNS不能证明业务；要求公网TLS
服务/证书/主机DNS覆盖会引入当前没有授权的配置及资源。双栈一次实现会增加IPv6解析与
地址/资源边界，本轮选择复用已成功的IPv4链；IPv6业务必需验收保持NOT_RUN，未从Task移除。
TLS仅SNI的边界与现有HTTP/SOCKS四项一致，真实HTTP204+ACK另例证明WG可正常收应答。

## Verification 与 Decision

批准前只做设计/一手源码核对、引用与基线freshness检查、独立技术审阅；不启动DUT或服务。
批准后用Go既有test/race/vet/gofmt/mod verify/build命令验证，Rust沿Task已有Cargo入口
执行compiler/reader/帧本地用例、两个真实用例、fmt及产品Clippy；记录真实命令、运行数和期限。
必要负例包含错URL/Host/SNI/地址/端口、未知/重复协议字段、模式串用、DNS漂移/缓存冒充、
TLS回调未执行/超量/超时、正确SNI后alert写超时/失败/短写（握手仍返回哨兵也须失败）、
响应缺ACK、零连接取消的合法stopped且非零退出、第二连接、迟到失败、EOF/Stop未确认及资源Hold。
复用Go固定内存自测并追加新别名HTTP/SNI自测；自测不能代替真实DUT证据。

由于新模式触及共同Go生命周期/stack及Rust资格，每轮新身份须回归旧DNS预检、WG
TCP/ICMP、UDP及Hold55/150；旧宿主四格拒绝测试在共享路径变化时重跑，否则只有逐文件
hash和影响闭包核对支持明确继承。HTTP/SOCKS四项同样记录重跑或继承依据，不混淆。
现有测试Clippy的7项既有诊断保留；不得抑制lint或声称全测试lint通过。
真实运行前后只读核验interfaces/addresses/routes/dns/proxy五快照；保留固定资产、
helper/最终配置digest、DUT同句柄PID/创建时间、peerUDP归属和最终自有端点消失证据。
9090被占用立即取消，不读取其API或终止占用者。不能以清理成功代替业务成功。

本候选仅补齐WG IPv4域名HTTP Host及TLS SNI证据；节点服务器hostname解析、完整
DNS拒绝、受控非宿主转发、IPv6业务及整Task/Human验收保持原必需项。本轮不改DNS/TUN，
只有后续具体需要改记录时通知用户；不进入TASK-010、不commit/push。

受影响Gate：新设计待审/待批准；批准后新实现需新验证与独立交付审阅，Delivery保持PENDING。
旧DCR009/013/014字节和checkpoint021证据保持；本候选尚未成为已接受Scope或冻结实现依据。

### 一手依据（已读回固定revision）

- [URLTest使用原URL构造HEAD和TLS transport](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/common/urltest/urltest.go)。
- [WG用户态Dial绑定已配置本地地址，缺IPv6地址直接失败](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/transport/wireguard/device_stack.go)。
- [WG Endpoint DNS与transport边界](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/protocol/wireguard/endpoint.go)。
- 本仓库`main.go:serveHTTPURI`、`stack.go:newMemoryTun`、`observe.go:observer`与
  `src-tauri/src/singbox/mod.rs:task009_http_urltest_preserves_tls_sni`提供已有测试路径。
