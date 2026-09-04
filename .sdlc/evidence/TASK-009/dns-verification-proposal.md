# TASK-009：系统 DNS 位置验证资源方案

状态：**PROPOSED，未授权实施、未运行 DNS 用例**。2026-09-04。
仅为 SF-003 剩余 DNS 验证准备资源决定；不修改 Task、DCR、源码或验收，不涉及 WG UDP/
主动入站拒绝方案。采用 technical-design Skill 的有界只读方案模式。

## 目标与输入身份

- TASK-009：`sha256:d5c3163a22806ca596caa9d4ba839c34a73654d31dc304b7a710a0a2f265362a`。
- DCR-003：`sha256:3040951342ef465fb177b3abb1a7537d73c55bc87e88e1a7b3ba93677033b26d`。
- DCR-009：`sha256:b193f16e6262f8695780af2f18cebc4a4f91dbbbe999f4b91f716059edbb9470`。
- `network-prerequisites.md`：`sha256:6297797be5627fade5a4aa77af253e261dfec0922e74992ba3b49b06ba36c1a5`。

需要分别证明：节点服务器 hostname 由 Windows 系统解析；HTTP/SOCKS 转发原目的域名；
WG 的探测目的 hostname 在本地解析成 IP；原 URL、HTTP Host 和 TLS SNI 不被改成 IP。
既有真实 WG TCP/ICMP 结果不证明 DNS，历史前提中的 WG 未运行描述不在此重新采纳。

## 本次确实读取的本机条件

只读 `Get-WinEvent -ListProvider 'Microsoft-Windows-DNS-Client'` 的元数据，不读取实际查询日志：

- provider 存在，GUID=`1c95126e-7eea-49a9-a3fe-a378b03ddb4d`。
- 3006/v0：QueryName、QueryType、QueryOptions、ServerList、IsNetworkQuery 等。
- 3008/v0：QueryName、QueryType、QueryStatus、QueryResults。
- 3010/v1、3011/v1/v2：网络查询/响应及 ClientPID；3020/v1/v2：结果及 ClientPID。
- 3018/v1/v2：缓存结果及 ClientPID，不能混称为网络查询。
- `Microsoft-Windows-DNS-Client/Operational` 当前 `IsEnabled=false`。
- 当前 token 的 ElevatedAdministrator=false，Performance Log Users SID 不在组列表中。

上述是 schema/权限现状，不是事件实际产生或采集成功证据。未启用日志、创建 ETW session、
发送 DNS 查询、启动 core/peer/listener、安装依赖或更改用户组。未执行 StartTrace，不能
虚构具体 AccessDenied 返回码；按当前 token 和官方权限要求，常规新建采集会话尚无条件。

## 路径比较与建议

| 路径 | 可证明内容 | 缺口/决定 |
| --- | --- | --- |
| 任意公共域名成功、抓到 DNS 包 | 域名有结果或发生过网络 DNS | 无法独立证明该 DUT 使用 Windows 系统解析；目标不受控，不采用 |
| localhost、hosts、手动清缓存/改 DNS | 本地特例或人为改变的环境 | 不是普通系统 DNS 证据，且越过现有主机边界，不采用 |
| 用户控制域名 + 权威 DNS 日志 | 哪个递归解析器何时访问过该随机名字 | 通常无 DUT PID；自研 DNS 和系统 DNS 都可能经同一递归器，单独不足 |
| 用户已有系统/递归 DNS 日志 | 可能证明已有解析链 | 只有能关联本次 DUT 客户端归属及请求/结果才可用；当前未提供 |
| 用户控制域名 + 短时 DNS Client ETW | 本机 API/服务查询、ClientPID、结果与实际目标 IP 可联合归因 | 推荐；需要域名记录、允许的查询范围和采集权限/资源授权 |

推荐最后一项。权威日志是增强证据，**不是必需的新服务器**：当完整 ETW 查询/完成链、
本次 PID 和实际目的 IP 都可验证时，不必额外部署 DNS 服务。若 ETW 无法关联客户端，
权威日志不能替它补齐，只能记 UNAVAILABLE 并返回新的具体选择。

## 最小用户输入与拟授权内容

1. 一个用户控制且当前 Windows 默认解析链可解析的专用测试子域；允许为每轮生成随机
   FQDN，例如 `<nonce>.node.<用户子域>` 和 `<nonce>.target.<用户子域>`。这些只是格式，
   不是本方案已获授权或可请求的真实域名。需用户提供精确子域、控制权和可查询授权。
2. 节点类 A 记录固定回答 `127.0.0.1`；WG 目的类 A 固定回答 `198.18.0.2`；AAAA 为
   NOERROR/NODATA。记录不得重定向到其它域名或意外地址，避免 CNAME 引入未授权查询链。
   可由用户预配两类 wildcard 记录，使随机名字不需要每轮改 DNS；给出记录内容及 TTL。
   权威服务日志若已有，可提供仅限这些名字/时间段的脱敏摘要，无需给本 Agent 管理凭据。
3. 明确允许当前默认解析器对上述名字发 A/AAAA 查询；这属于外部 DNS 请求，虽然最终
   TCP/WG 业务仍只到本机或内存栈。前述 WG 资源批准没有覆盖这项外部域名授权。
4. 允许一个短时、单 provider 的测试诊断采集器。当前普通 token 不能作为已具备权限；
   优先由用户在已有管理员诊断会话启动采集器，DUT 仍保持普通权限。不得自动 UAC、
   添加 Performance Log Users 成员、改 provider ACL 或启用永久 Operational 日志。
   这项诊断权限例外须明确决定，不能从“无主机 DNS 修改”推导成无需授权。

系统/递归 DNS 可能拦截公开域名返回 loopback 或保留地址的答案。仅配置权威记录不保证
本机最终能收到它；实测被拦截则记录资源 UNAVAILABLE，不改 DNS、不加 hosts、不关闭
防重绑定策略，也不擅自换公共服务。这是该最小拓扑的真实环境限制。

## 采集器边界与位置判定

候选只用 Windows 自带 ETW/TDH API，在独立测试采集脚本内以 Win32 互操作实现，不新增
第三方包或安装驱动。它是有权限的测试工具，仍有事件解码、过滤、取消/清理的实现成本；
具体代码及文件范围须随后纳入 DCR 并独立审查，不能把它当作现成已运行工具。

在 DUT 启动前开启唯一描述性 session `VeyraTask009DnsTrace`，首先按本次已知 QueryName
集合及所需事件 ID 过滤；会话同名已存在即停止本用例，不接管或关闭未知来源 session。
不能只设置 DUT EventHeader.ProcessId 过滤：DNS Client 服务代发事件的 header PID 可能
是服务 PID，payload ClientPID 才是请求者。3006/3008 等缺 ClientPID 的版本，仅在实际
阳性对照验证了 header PID 语义且同名/类型/时间链无歧义时归因；否则标记关联不足。

消费端再次严格筛选 run nonce、QueryName、A/AAAA、DUT PID/创建时间窗口和事件版本。
每条证据至少包括查询调用、完成状态、实际结果，以及可用的网络请求/响应或缓存命中
类别；同时读回系统解析器配置，结合 ServerList/实际服务器字段核对未使用测试覆盖值。
配置里引用 dns-system 不能单独充当运行证明，单条 DNS 包/同域名其它进程事件也不能。

先后各做一次同采集条件、独立名字的系统解析阳性对照，验证 schema、客户端归属和结果
采集可用；确认 ETW lost events/buffers 为零，再考虑“未观察到本地目的域名解析”的负向
结论。按随机新名字区分缓存，不调用 flush，不强求系统内部只发一个包或只有 A 请求。
若存在丢事件、只见失败、缺完成/关联、权限不足，不能把事件缺失记作“远端解析 PASS”。

## 三组实际用例及原始身份

1. **节点 hostname**：HTTP/SOCKS 节点服务器改为已授权 node FQDN，由现有真实链正常
   compile/finalize/check/run。ETW 中匹配 DUT 的该名字解析为 127.0.0.1，同时自有
   loopback mock 确认连接；目的 URL 仍保留原 hostname，mock 记录 CONNECT authority/
   SOCKS ATYP=domain 和 HTTP Host/path/query。辅助 mock 不做外部转发。
2. **WG 目的本地解析**：WG 服务器仍使用已批准本机 peer；URL 使用 target FQDN，
   ETW 中该 DUT 得到 198.18.0.2，peer 解密后看见目的 IP=198.18.0.2；HTTP 服务核对
   Host=原 FQDN（含显式端口）、原 path/query，以及完整响应/ACK。不预解析或改写 URL。
   DCR-009 当前固定 IP Host 的测试 helper 需明确有界设计修订，不偷偷接受任意 hostname。
3. **TLS 名称保留**：使用同类受控名字的独立 HTTPS 用例，保存相同 DNS 归因及原始
   ClientHello SNI；允许复用既有有界 ClientHello 解析方法。只读到正确 SNI 不代表
   可信 TLS 握手或 HTTP 成功。如验收要求完整可信 HTTPS，还需用户提供对应已信任
   公共证书及受控服务授权，作为单独输入；不修改系统信任库、不忽略证书校验。

HTTP/SOCKS 目的名是否在本地查询，要结合其传输帧携带原域名和无丢失 ETW 对照判断；
mock 收到域名只能证明转发语义，不能冒充远端真的执行过 DNS。

## 资源、保留与剩余问题

采集窗口每用例最多 60 秒，ETW 内存缓冲总上限拟为 8 MiB，消费端只保留至多 256 条
白名单摘要；超限/丢事件即失败。以实时消费为优先，不创建全机 DNS 明细 ETL；若本机
provider 无法做到所需过滤，则停止该候选，不默认把其它应用的查询长期落盘。
事件仍可能在过滤前短暂进入系统缓冲，应披露这一诊断可见性，而非声称绝不接触其它事件。

停止 DUT 后关闭本次采集，按创建时持有的 session 身份确认停止和消费者退出；不得关闭
其它 session。父失败/被中断的 session 清理也必须有界且归属明确。主机 DNS/hosts/
接口/路由/代理前后只读比对；不自动“恢复”未修改的设置。私有摘要不包含未知域名、
用户浏览记录、凭据或完整配置，原始敏感事件不进入 Git。

**当前最小问题是：用户能否提供上述专用子域及两类记录，并授权短时诊断采集权限？**
当前没有域名输入，Operational 未启用且 token 无常规采集权限，因此无法完整执行。
本文件只完成可审阅方案；DNS 用例状态仍为 NOT_RUN/资源 UNAVAILABLE，不生成 PASS。

一手依据：

- 本机 provider manifest、日志配置与 token 的上述只读元数据。
- [Windows DNS 查询与缓存路径](https://learn.microsoft.com/en-us/windows-server/networking/dns/queries-lookups)
- [StartTrace 权限与会话资源要求](https://learn.microsoft.com/en-us/windows/win32/api/evntrace/nf-evntrace-starttracew)
- [ETW 过滤描述及 PID 过滤语义](https://learn.microsoft.com/en-us/windows/win32/api/evntprov/ns-evntprov-event_filter_descriptor)
- [配置 ETW 会话与 payload 过滤](https://learn.microsoft.com/en-us/windows/win32/etw/configuring-and-starting-an-event-tracing-session)
- [DNS_QUERY_REQUEST 默认服务器与显式服务器输入](https://learn.microsoft.com/en-us/windows/win32/api/windns/ns-windns-dns_query_request)
