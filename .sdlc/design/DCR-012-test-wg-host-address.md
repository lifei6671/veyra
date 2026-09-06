# DCR-012：WG 宿主实际地址拒绝对照

状态：PROPOSED；待独立设计审阅及用户批准。TASK-009 / SF-003，修复 TASK009-CR-006。
需求身份：`sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`。
实现基线：checkpoint014 `sha256:37feddf0c296a6db8e7552d203c645efa5cd9663b4649243465ace58e3d5c126`。
当前 Git 快照：`b8c705f`；快照提交不重新归属 checkpoint014 的历史身份。

## 冲突、追踪与影响

DCR-011 的原始宿主目的 127.0.0.1 在固定 DUT 的 IPv4 栈中、进入 Router 前被丢弃。
verification-014 和 wg-reject-runtime-diagnosis-014.md 记录虚拟 TCP/UDP 阳性成功、host127
两格超时；phase2/3 NOT_RUN，原比较 FAIL。不能通过 Router 放行规则修复更早的丢弃。

本提案只修订 DCR-011 的宿主 TCP/UDP 对照目的及其资源，虚拟地址两格保持；申请将
SF-003 CHANGE009 的 host127 同路径对照替换为下面的实际宿主 IPv4 同路径对照。
这是需批准的验证契约变更。它证明实际宿主地址的 Router 隔离，不能声称原始127/8的
Router拒绝已被正向对照验证；原127诊断和FAIL证据永久保留，不能改写为PASS。
SF-003 的 DNS、受控非宿主转发、系统解析/URLTest 及其余既定验收均保留。

影响只限 compiler.rs 的 cfg(test)、Windows managed_sidecar_port.rs 现有测试模块、
scripts/task009-wg-peer 既有测试程序和README。批准后才同步 Task/Change/Readiness，
修订测试并产生新的实现身份。产品拒绝、固定 EXE、Domain、IPC、存储、依赖及锁不变。
本候选不使交付通过；技术设计 Gate 待本候选审阅和批准，Delivery/Task 验收仍 PENDING。

## 具体资源及授权增量

2026-09-04 只读 Get-NetIPAddress/Get-NetAdapter/Get-NetRoute 核定：

- 适配器：`vEthernet (Default Switch)`，当前 ifIndex 19，Up。
- GUID：`{3D816D4D-97AF-48FA-89DC-EA7945796D10}`。
- 现有 IPv4：`172.26.192.1/20`，Preferred、SkipAsSource=false。
- 已有本地 `172.26.192.1/32` 路由，NextHop=0.0.0.0；不创建地址或路由。

拟新增且仅新增两个 Rust 自有 OS socket：`172.26.192.1:动态端口` TCP、UDP 各一个。
保留原两个127.0.0.1目标，共四个socket；每个动态端口独立bind返回，四端口数值必须
互异、非9090、非peer协议端口，冲突即失败。始终精确绑定，不使用wildcard或SO_REUSEADDR。
先持有四目标，再启动peer；部分bind失败必须关闭本次已持有资源，不启动DUT。

该地址是宿主在现有虚拟交换机上的实际地址。可达该地址的虚拟机或其它已有路由主机
可能连接/发包；本设计不假定Hyper-V、NAT或防火墙已隔绝它。固定20字节、随机token
和有界I/O只限制服务内容，不能消除连接、UDP干扰或资源耗尽导致的测试失败。
服务无代理、DNS、递归请求和持久写入；不返回未知载荷，不增加自动重试或并发工作池。

本次批准仅覆盖上述 GUID/IP，不自动选择LAN、WSL、Karing TUN或其它地址。执行前只读
核对GUID、IP/前缀、状态与本地路由；每阶段前及结束后重复确认。同一GUID可重解析ifIndex，
但地址变化/缺失/非Preferred/接口不Up即 UNAVAILABLE 或运行失败，不能回退选择新IP。
任意中途漂移使整轮证据无效。读取与解析失败也不能继续bind/run。
不修改主机网络、现有TUN/代理、防火墙或权限，不启动虚拟机或新适配器。

## 路径和编译约束

```text
同一 Go peer 198.18.0.2 → 加密 WG → 固定 DUT 198.18.0.1
  case1 TCP → 198.18.0.1:vt → 核心虚拟映射 → 127.0.0.1:vt
  case2 TCP → 172.26.192.1:ht → Router Direct → 172.26.192.1:ht
  case3 UDP → 198.18.0.1:vu → 核心虚拟映射 → 127.0.0.1:vu
  case4 UDP → 172.26.192.1:hu → Router Direct → 172.26.192.1:hu
  三个串行DUT：阳性前测 → 正常受保护 → 阳性后测
```

每一格在三个阶段的peer/密钥/目的/端口/目标原句柄完全相同。host不设为DUT或peer的
内存栈本地地址，不做目的重写。Go cryptokey路由精确使用198.18.0.1/32和172.26.192.1/32；
新场景不再使用127/32目的。虚拟路径及其既有peer回包支持不改变。
每个DUT的固定URLTest bootstrap、完整204/ACK、phase身份、ICMP健康检查和API存活沿用DCR011。

受保护配置仍由普通compile产生，唯一用户规则是Port([vt,ht,vu,hu])→Direct，按该顺序。
阳性仅cfg(test)在原route reject前添加四条精确CoreRule，顺序对应case1..4：
唯一WG tag + 127.0.0.1/32:vt TCP；同tag + 172.26.192.1/32:ht TCP；
同tag + 127.0.0.1/32:vu UDP；同tag + 172.26.192.1/32:hu UDP。
每条只含一个IP/32、一个端口、一个network和Direct。原route/DNS reject及用户规则保留。

构造方法只接受已有强类型意图及四个NonZeroU16端口，不接受地址、JSON或任意规则。
IP为本次已批准测试常量；普通非test模型/validate继续拒绝前缀。构造与最终字节反序列化
均严格验证完整四元组、唯一WG/URLTest、phase/token格式和用户规则，然后剥离精确前缀
走原validate。DCR008/010其它测试元组不变；不通过from_bytes或JSON修改构造成功运行配置。

Go私有init_reject仍为封闭场景，将两个端口字段修订为virtual_tcp_port、host_tcp_port、
virtual_udp_port、host_udp_port，四者严格校验；没有host_ip输入。Rust与Go固定IP必须一致，
纳入纯本地协议测试和最终配置读回检查。旧二字段帧/未知字段/混合场景拒绝，不保留兼容壳。
消息顺序、phase1/2/3和各阶段重置不变；观察器以固定case四元组核对送入WG和回包。

## 对照成功、失败与生命周期

四格顺序、20字节token+phase+case_id载荷、2秒单次I/O及无业务重试沿用DCR011。
四个目标各自记录接受/接收和载荷计数，绑定允许的case_id，不能靠错误目标回显凑总数。
loopback目标来源仍要求loopback；host目标只接受源IP等于172.26.192.1且目标端口匹配的流，
所有未知源也计入异常而非静默过滤。该来源约束是本地自连的可行性假设，必须真实验证；
若本机Direct选择其它源，不得扩大来源白名单后重试，返回具体失败再修订设计。

- phase1/3：每格实际经固定DUT完成精确回显；各目标各收到一个本阶段对应载荷。
- phase2：四格实际开始探测、peer出口证据匹配，无回显且四目标接受/接收增量全部零。
  预期reset/EOF/拒绝/有限超时只在前后阳性成立且ICMP/API健康时可作负向证据。
- 最终每个TCP目标各2次合法连接、每个UDP目标各2个合法报文；总TCP4、UDP4。
  phase2载荷全生命周期为零；迟到/重复/未知载荷、空TCP或额外UDP均FAIL。
- 缺bootstrap、错误源、目标关闭、阶段错配、地址漂移、全局超时不能报告拒绝PASS。

这仍是有限窗口配对证据；不宣称逐包看到DUT内部拒绝，也不覆盖其它宿主IP/IPv6。
新增host目的不在127/8，避开已证实的早期丢弃，但通过Router和往返成功仍为NOT_RUN。

沿用每phase40秒、全局工作135秒、helper硬限150秒、父159秒、外层160秒。执行前只读
核定可在DUT启动前完成；四目标bind及持有纳入父外层截止，不额外延长期限。
原TCP/ICMP、UDP、55秒Hold和新150秒Hold回归保留；无后台任务或新超时配置项。
正常/异常/panic均先取消探测并确认当前DUT/pending退出，再结束peer协议及关闭四目标。
Stop未确认：禁止下一DUT/finish_phase/shutdown，四目标句柄及peer.stdin保留到既定硬限，
始终清理FAIL，不删除未确认DUT私有资源。地址漂移、部分启动失败也走相同所有权收束。
证据记录四socket和DUT/peer实际PID所有权、每阶段最终配置hash/实例identity/停止确认，
不输出token、密钥或secret；前后只读比较接口/地址/路由/DNS/代理。

## 最小实现、替代方案及实施顺序

必要增量是两个精确host目标、四端口封闭元组和对应四格计数；复用现有线程、阶段协议、
Deadline/Hold和独立Go模块。无新第三方依赖、驱动、特权、系统配置或产品功能。
只改Router放行仍无法使127目的进入Router；仅测已成功虚拟两格无法覆盖独立宿主目的；
修改DUT栈/EXE违反固定资产契约。将两个目标改为wildcard能减少句柄，但扩大暴露，故不采用。
维持原127对照则当前契约继续失败。本方案的成本和新增可达面须由用户接受，不能默认采用。

批准后：同步CHANGE/Task/Readiness → 有界实现 → 构造/读回负例、协议与目标负例 →
既定Cargo/Go检查 → 一次串行真实三阶段对照及原TCP/ICMP/UDP/Hold回归 → 新身份独立交付审阅。
负例须覆盖错地址/tag/网络/端口/规则顺序、旧帧、错误目标case、目标缺失、迟到、漂移和
四资源Hold。保留原断言强度，只按获批拓扑修订用例；不删除或ignore失败测试来通过构建。
命令从既有verification-014和模块README确定，真实验证前先核验最终helper/EXE身份。
任一阳性失败停止后续阶段并保留FAIL，不能再次把静态设计审阅解释为运行通过。

## 依据

- `.sdlc/design/DCR-011-test-wg-local-reject.md`及CHANGE009：原授权边界。
- `.sdlc/evidence/TASK-009/code-review-checkpoint-014.yaml`：OPEN P1 TASK009-CR-006。
- `.sdlc/evidence/TASK-009/wg-reject-runtime-diagnosis-014.md`：固定栈丢弃链及真实失败分布。
- [固定核心WG endpoint源码](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/wireguard/endpoint.go)：
  NewConnectionEx/NewPacketConnectionEx仅映射endpoint本地虚拟目的，之后调用Router。
