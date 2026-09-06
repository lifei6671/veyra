# DCR-013：固定内核 DNS 结果预检

状态：PROPOSED，待独立设计审阅与用户批准。TASK-009 / SF-003。
需求身份：`sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`。
当前 Task：`sha256:9885a00487945e06e67c545c9478ef2be38fd75b2ff0b2c08a5af6fe5ff6ef9a`。
实现基线 checkpoint015：`sha256:d6ef4fa08dfbecec5d332dfc8a6faa54efeefe80df6e2976e613356805031a84`。

## 要解决的问题和批准范围

用户选择 `veyra.disign.me`，提供 A=203.88.124.39、AAAA=2607:f130:0:14d::7162:ad8f，
并说明本机 TUN 会返回虚假地址。PowerShell 先前取得 198.20.0.255 / fc00::fe；这不能
预设为固定 DUT 的结果。需要先观察 DUT 自己的解析，再决定后续 WG/节点 hostname
业务目标应如何配置。现在不要求调整 Cloudflare、TUN 或主机 DNS。

本 DCR 仅申请一个 **cfg(test) DNS 诊断预检**：沿用已批准的本机 WG peer，正常编译
唯一 WG + URLTest Pool，由原生 URLTest 触发 `veyra.disign.me` 的本地查询；临时启用该
测试 child 的 debug 日志，从 spawn 开始私有收集 DNS 白名单摘要。没有新增系统采集器。
该例外修改 DCR-001/当前 Compiler 的关闭日志、运行 child 标准流为 null 的测试约束，
须明确批准；普通产品、既有用例及发布构建仍使用原关闭日志配置和 null 标准流。

预检最多证明“此固定 child 在唯一 local DNS 配置下完成 transport exchange，并返回
这些地址”。它不证明目标可达、地址已用于成功业务、所选 DNS 服务器的网包身份、
Windows DNS API、Cloudflare 权威记录、没有 TUN 截获或完整 SF-003 通过。
节点 hostname、WG 的 Host/SNI、完整拒绝/非宿主转发/IPv6 等必需验收全部保留。
既有 HTTP/SOCKS Host/SNI 四项的历史通过不被抹去，也不扩张为上述缺项的证据。

## 精确拓扑与请求

- 复用 DCR-009 的 Go helper，新增封闭 `init_dns_probe` 模式，沿用随机密钥及 token、私有管道、
  127.0.0.1:动态端口 WG 传输、DUT 198.18.0.1/32、peer 198.18.0.2/32。
  运行前核验本轮重建 helper 与源/模块身份；不增加地址、内存服务或新依赖。
- peer ready 后用实际归属端口/公钥构建唯一 WG 节点。节点 server 固定 127.0.0.1，
  所有其它节点和业务 inbounds 为空；route/DNS 的 WG 置顶 reject 保持正常编译结果。
- 唯一 Pool 为 URLTest，URL 固定
  `http://veyra.disign.me:18080/task009-dns-preflight`，interval=300s、tolerance=50ms。
  不调用第二次 delay API，不更换 host、端口、scheme，不接收任意 URL 或地址输入。
  DNS 保持唯一 `dns-system`、`type: local`、final/default_domain_resolver 固定引用；
  不增加地址覆盖、DNS transport、hosts、family preference、缓存设置或 IP 预解析。
- 仅允许该精确域名的普通 A/AAAA 查询经当前 local transport/系统网络配置处理；
  内核本身的服务器尝试行为保留。业务 TCP 由 WG 用户态栈发起，经已有本机加密 peer，
  不由 OS 向解析结果（包括用户公网地址或 fake IP）发 TCP/UDP 业务连接。
  helper 的预检模式不创建 HTTP/UDP 业务服务或转发器；解密业务包按下节有界丢弃。
- 不要求这次 HTTP 成功，不添加兼容响应或新目标来消除连接失败。预检完成即停止 DUT。
  如果收到 tcp/udp/icmp 业务事件或 failed 事件，应作为不符合此预检拓扑的失败处理，
  不能借其结果宣称 Host/SNI 成功。预检模式在 ready 后允许 shutdown 并正常退出0；
  不放宽既有 init/init_udp/init_reject 的业务成功或非零退出规则。
- 不访问用户现有代理 API、不修改代理配置、不直接连接用户提供的公网服务器。
  DNS 本身受当前 TUN 影响是预检要观察的事实，不能描述为已隔离出原生 Windows 路径。

## 编译、配置和 child 归属

Rust 改动仅限下列三个文件的 cfg(test) 代码/字段及必要条件编译位置：

- `src-tauri/src/singbox/compiler.rs`：测试构造方法在正常 compile 之后、finalize 之前，
  仅为上述完整封闭元组生成 `log:{disabled:false,level:"debug",output:"stderr",disable_color:true}`。
  不生成 timestamp 或文件路径字段。测试 final 校验必须重新确认整个元组、唯一 Pool/WG、
  原 DNS/reject/无 inbounds，以及精确 log 四字段；不能因日志例外跳过普通字段检查。
  其它候选含这些字段或 disabled=false 必须拒绝。非测试构建不接受这些额外字段。
- `src-tauri/src/singbox/managed_sidecar.rs`：私有测试启动分支只把该已核定 run child 的
  stderr 改为 pipe；stdin/stdout 仍 null、CREATE_NO_WINDOW 保持，check 标准流仍 null。
  child owner 持有读取任务和诊断状态，不能出现 detached 读取线程或全局日志开关。
- `src-tauri/src/platform/windows/managed_sidecar_port.rs`：现有测试模块驱动预检，必要
  cfg(test) 接线根据完整测试候选校验结果选择上述启动分支。没有公开产品入口或任意 PID。
  分支资格必须绑定 pending 的最终字节哈希，run 前读回仍匹配；失败/消费/停止时一起清理，
  不能由测试布尔值使另一候选获得采集资格。

保留 Parser → normalize → AppState/RuntimeIntent → Compiler → 每实例 secret/finalize
→ PrivateRuntime/ACL → fixed check → 同字节 run → Ready/stop。配置未检查不得运行；
不把生成 JSON 写回修改，不从用户现有 state 或订阅取配置。采用新 child owner、PID、
 创建时间和最终哈希共同归属；日志上下文 ID 不是 PID。

## Go 预检模式与无转发边界

初审 DNSR018-01/02 发现既有 init 必须完成 TCP/ICMP 才退出0，而且旧 observer 会将
非198.18.0.2的目的报错。因此新增模式明确实现预检的结束语义及包处理，不能靠抢先
Stop、忽略 failed 或把旧模式 exit1 归一化为成功来修复。

只允许 `scripts/task009-wg-peer/{main.go,protocol.go,stack.go}` 的模式接线、对应既有
`{protocol_test.go,lifecycle_test.go,stack_test.go}` 的用例及 README 更新。无新Go模块、
依赖/锁、通用转发器或地址配置。独立预检状态可定义在 stack.go 中，main.go 仅在
init_dns_probe 模式选择；其它模式不进入该分支，其观察器和业务校验不改。

私有协议v1新增如下封闭分支，行/总量/字段类型/重复字段约束沿用DCR009：

```text
父→子 init_dns_probe {v:1,op:"init_dns_probe",run_id,dut_private_key,peer_private_key,token}
子→父 ready         {v:1,event:"ready",run_id,udp_port,peer_public_key,dut_public_key,
                     selftest:{tcp:true,udp:true,icmp:true}}
父→子 shutdown      {v:1,op:"shutdown",run_id,dut_stopped:true}
子→父 stopped       {v:1,event:"stopped",run_id,resources_closed:true,
                     discarded_packets:u32,discarded_bytes:u32}
```

ready前仍执行既有纯内存自测，不能混入真实预检计数。只有init_dns_probe→ready→shutdown
→stopped顺序可返回0；该模式不接受probe_icmp、UDP命令或阶段切换。其余模式的stopped
不接受新增计数字段。所有failed事件沿用原固定枚举并最终非零；超时、EOF和父异常仍
进入既有Hold，不能因尚未收业务包而当成成功。

真实peer仍使用固定用户态NIC和唯一WG密钥/allowed_ip=198.18.0.1/32，以及原loopback
conn.Bind。预检模式不创建真实TCP listener/UDP service，不启动serveHTTP/serveUDP。
memoryTun.Write 只在此模式、WG认证解密后，检查现有offset边界和每个IP包长度1..1280，
累计最多64包、81920字节，然后丢弃：不调用旧业务observer，不创建PacketBuffer，不向
link/stack InjectInbound，不生成任何业务应答。计数并发受同一锁保护；越界记录失败并
通知主循环进入Hold。WG错误和bind错误仍按原方式传播，不能连同业务包一起吞掉。
本模式无业务outbound队列写入，Read由原取消/Close唤醒；允许0包/0字节，不要求握手。
这些丢弃计数只是资源边界，不证明目的IP、握手成功、DNS答案使用或Host/SNI。

close按模式处理不存在的listener/service，仍关闭WG/bind/内存栈并等待其工作退出；
完成关闭后才读取稳定最终计数、发stopped和exit0。Rust严格核对模式/run_id、计数上限
及0包⇔0字节、字节数不少于包数，只有无failed、完整stopped、管道结束、stderr零且
exit0才记peer清理通过。纯本地测试覆盖新模式取消、任意目的包不进入栈、上限/越界、
迟到/模式串用、缺stopped和非零退出；回归旧模式仍需要原业务结果。

## 有界收集与判断

从 spawn 返回后立即并发排空 stderr，以固定 512 字节读取块处理。单行上限 4096 字节，
整次接收累计上限 65536 字节，摘要最多 128 条、地址集合最多 8 项。原始日志不写文件、
终端、UI 或 Evidence；仅在内存中按行解析，保留固定事件类型、精确域名、A/AAAA、
合法 IP、RCODE、TTL 和相对时间。URL、token、密钥、secret、任意原始错误不得透传。

日志格式按固定 revision 的 Formatter 和 DNS logger 解析，不把任意包含关键字的文本
当成功。须来自 dns logger，域名边界精确匹配，RR owner/class/type/地址合法；区分
exchanged、cached、optimistic、refreshed 和 lookup。只观察到缓存成功不算 transport
exchange 通过。至少要求查询名称、该名 exchanged NOERROR、至少一个同名 A/AAAA RR，
以及 lookup succeed 的非空地址集合；成功集合中每个地址都须由本次 exchanged RR 支持。
有 CNAME、未知相关 DNS 格式、额外地址或矛盾记录则不按本版本解析器判通过，保留固定
原因类别供设计分析，不猜测补全。不要求 A/AAAA 都成功，不强制返回真实地址或 fake IP。

无关行只累计字节并丢弃；不能把所有内核 ERROR 都当 DNS 失败，因为预检不要求 HTTP
连接成功。相关 DNS 错误、截断/非法编码、读取错误、越界使诊断失败。超过上限后继续
以固定缓冲丢弃排空并通知 owner 停止 DUT，不能停止读管道后阻塞 child；owner 至少每
25ms 检查状态。不得对原始错误打印 debug。缺日志/超时记录 UNAVAILABLE，不是负向 PASS。

不依据“没有日志”证明 HTTP/SOCKS 没有本地解析。实际 DNS 服务器 IP 不在这些日志中，
只读系统配置是上下文而非包目的证据。前后快照发生变化或结果归属不清则本轮无效。

## 时限、清理与失败退出

复用固定 API 9090 测试锁及 DCR-009 的 60 秒外层/55 秒 helper 硬限/45 秒最迟开始
清理规则。init/ready 最多10秒、check10秒、Ready2秒；收集截止取 DUT spawn+10秒、
peer ready+25秒和 helper spawn+40秒三者最早者。阶段切换不重置绝对期限。
期限耗尽后停止预检，不自动重试或换域名。Go 30秒工作窗口及 Hold 机制保持。

成功、业务失败、日志溢出、panic 和取消均先停止并确认 DUT，再完成 stderr EOF/读取
任务退出（最多2秒），最后发送 shutdown、关闭并确认 helper。读取线程异常通过
owner 状态传播；DUT 退出已确认而管道未结束则记录失败，不假称采集完整。
读取句柄只由 reader 持有；无后代进程继承本次 stderr 写端。未确认 DUT 退出时沿用
RecoveryRequired 与 peer Hold，不提前释放 helper/私有配置、不启动其它 DUT。
不得为本预检新增系统服务、Job 管理器、watchdog 或提权；若既有退出/pipe所有权不能
有界完成，返回设计修复，不扩大机制。只有确认子进程与读侧退出才释放所有者。

## 实施验证与影响分析

批准后先建立 readiness 和新的交付身份，再实施；checkpoint015 的56份源/154份工件
已于2026-09-04T19:47:05.8083978+08:00重算无漂移，仍是历史基线。
新增测试须验证封闭日志元组拒绝、final 字节绑定、身份串用拒绝，以及 parser 的缓存/
错域名/错RR/缺结果/冲突/非法UTF8/超大行/总量/摘要上限。管道测试覆盖实际 EOF、越界
后排空、读取错误和退出；真实预检需要 Ready、解析摘要、DUT先停、peer退出与私有清理。
回归原关闭日志/null 分支、HTTP/SOCKS四项和WG TCP/UDP既有场景；Go按README跑定向
纯本地测试、race/vet/fmt/build（原有时限），记录新helper哈希；不重跑无关UI验收。
命令沿用 Task 的 Cargo test/fmt/clippy；精确过滤名在实现时登记，检查非零测试数。
源变更使受影响编译器/child/Port验证和交付审阅需更新，不继承为新实现PASS；DCR012与
既有源证据不追溯改写。当前Task/Delivery仍未整体验收，不进入TASK010或commit/push。

复杂度：新增一个测试日志变体、一个有界私有读取任务，以及现有peer内的封闭丢包预检
模式；无新listener/服务/依赖、权限、系统
采集或核心补丁。只读PowerShell答案不能代替DUT结果；DNS-Client ETW不覆盖此固定内核
查询路径；全机抓包还需权限与PID归因；直接改DNS/TUN会改变被观察环境。选择此最小
诊断只为消除“DUT实际拿到什么地址”的未知，不把它提升为完整DNS证据替代品。
获准后仍需根据实际结果另行核定节点/WG业务目标；只有届时确实需要记录变更才通知用户。

## 一手依据

固定 revision 为 `0b8995879f29a9b98ee027bc17b75e101445b238`：

- [DNS Router](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/dns/router.go)、
  [Client](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/dns/client.go)、
  [DNS日志](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/dns/client_log.go)。
- [Local transport](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/dns/transport/local/local_shared.go)、
  [Windows配置来源](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/dns/transport/local/systemconfig/source_windows.go)。
- [WG域名Lookup与用户态Dial](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/protocol/wireguard/endpoint.go)、
  [日志writer](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/log/log.go)。

前提与资源记录：dns-preflight-016.md、dns-resource-017.json。根代理负责设计，
`/root/dns_resource_expert`仅提供只读日志可证明性分析，未充当本设计的独立审阅者。
