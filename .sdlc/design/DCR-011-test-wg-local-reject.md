# DCR-011：WG 到虚拟地址和宿主的 TCP/UDP 拒绝对照

状态：PROPOSED，待独立设计审阅与新增测试边界批准；没有实现或运行证据。
TASK-009 / SF-003，第2项拒绝验收的 IPv4 虚拟地址/宿主 TCP、UDP 子集。
需求身份：`sha256:4a2cd1e2b9698087bcbc4ac892d7b052a5e2c06554e372479fe31c81cbea9d45`。
交付基线：checkpoint013 `sha256:5f6438663e7d08e0241fde9a733e466c2dc7031dba31dbe050c1b3f36e52f0d5`。
复用 CHANGE007/008 的固定核心、Go依赖、用户态 WG、私有配置、资源归属与错误处理。

## 本次新增范围与意义

验证四条路径：认证 peer 发起 TCP/UDP，分别到 DUT 虚拟地址 `198.18.0.1` 和宿主
`127.0.0.1`。两个目的均应被产品置顶 route reject 挡住，不能被后续 Direct 用户规则覆盖。
先、后各运行一次同路径阳性对照，中间运行正常受保护配置；同一组目标服务持续存活。
仅连接失败、peer发送成功、目标没有监听，均不能单独证明拒绝成立。

需要用户批准的最小增量：

- 两个本次自有的 `127.0.0.1:动态端口` 回显目标，分别 TCP 与 UDP，不是代理或 DNS 服务。
- 仅 cfg(test) 可生成的阳性对照：在原拒绝规则之前加入两条精确放行，只允许该 WG tag
  访问上述两个 loopback 端口及对应网络类型。其它目的仍由原拒绝规则阻断。
- Go 测试端新增封闭的三阶段主动探测场景；该场景全局工作截止135秒、helper硬截止150秒、
  父最终截止159秒、用例外层160秒。原TCP/ICMP与UDP应答场景的55秒helper期限保持不变。

不增加产品配置选项、产品 listener、依赖、锁文件、系统网络设置或特权操作。
受保护实例仍走普通 ObservationOnly 编译；阳性对照明确标为测试配置，不是放宽产品规则。
本次不覆盖 DNS Router 独立拒绝、DNS 报文、非宿主目标转发、IPv6或系统DNS解析；这些
SF-003义务仍保留，不能据本次四格宣称完整拒绝矩阵或 TASK-009 通过。

## 固定拓扑与三个实例

```text
Go peer 198.18.0.2 ──本机加密 WG── DUT 198.18.0.1
  ├─ TCP → 198.18.0.1:tcp_port ──虚拟目的映射── 127.0.0.1:tcp_port
  ├─ TCP → 127.0.0.1:tcp_port ───────────────── 127.0.0.1:tcp_port
  ├─ UDP → 198.18.0.1:udp_port ──虚拟目的映射── 127.0.0.1:udp_port
  └─ UDP → 127.0.0.1:udp_port ───────────────── 127.0.0.1:udp_port
       同一目标、端口与peer：阳性前测 → 受保护测试 → 阳性后测
```

目标为 Rust 本次创建并全程持有的两个 listener；端口非零、互异、非9090、非peer协议端口。
先bind并持有目标，再启动peer；随机端口冲突即失败，不重试、不腾挪无关服务。
peer仍只有一个 OS UDP4 loopback协议socket；内存HTTP18080服务只用于WG通道就绪。
peer的gVisor栈在此新场景启用已有IPv4 `AllowExternalLoopbackTraffic`，以接收隧道返回的
127/8源地址；它不是主机loopback或路由设置。peer WG allowed_ips只增加精确127.0.0.1/32，
保持198.18.0.1/32；不加入0/0、不把目标地址配置成本栈本地地址导致探测绕过WG。

同一peer、WG密钥、虚拟地址、目标句柄跨三阶段保持。每阶段启动一个新的固定核心实例，
API secret/私有配置独立；始终先确认前一DUT退出才启动下一DUT，不存在双核心并行。
Go在每阶段DUT启动前清空本阶段观察状态。每个DUT使用一个WG节点和唯一URLTest Pool，
周期300秒；首次HEAD到peer内存HTTP18080，完成完整204响应及ACK后才开始主动探测。
URL为固定 `http://198.18.0.2:18080/task009-wg?token=<本次token>&phase=<1|2|3>`。
这让新的DUT先发起认证通信，peer学习其实际WG协议端口，避免向上个实例的旧端口探测。
三阶段逐一断言已认证握手/HTTP应答路径；新阶段不能继承旧计数、旧连接或旧成功帧。

## 编译例外与最终读回

受保护配置的RuntimeIntent仅包含上述节点、Pool和一条enabled用户规则：
`Port([tcp_port, udp_port]) → Direct`。默认目标为该Pool；两条route/DNS置顶reject仍
完全由普通Compiler生成。用这条实际可放行用户规则检验置顶reject优先级。

阳性配置由仅cfg(test)的内部方法在同一个强类型普通Plan上前置两条CoreRule：

```text
{inbound:[唯一WG tag], ip_cidr:["127.0.0.1/32"], port:[tcp_port], network:["tcp"], outbound:"direct"}
{inbound:[唯一WG tag], ip_cidr:["127.0.0.1/32"], port:[udp_port], network:["udp"], outbound:"direct"}
```

后面完整保留原route reject和原用户规则；DNS置顶reject完全不变。端点虚拟目的到loopback
的映射由固定核心本身执行，不新增override或系统地址。本方法不接受任意规则、JSON、地址、
action、network或Profile参数；只接收上述严格意图及两个NonZeroU16端口。

必须封闭校验唯一WG（127.0.0.1 peer、198.18.0.1/32、MTU1280、无PSK/reserved）、唯一
指定URLTest及固定phase/token格式、两个精确前缀规则、无业务入站/其它节点/用户规则。
两端口顺序及Direct用户规则也须一致。prefix之外剥离后的Document仍走原通用validate；
只有完整测试元组匹配才允许该前缀，不能泛化为“test时允许缺reject/任意inbound规则”。
最终配置反序列化读回时同样辨认并校验完整元组，不能只在构造时校验或依赖序列化不带的标记。
非test模型/校验仍拒绝这个规则顺序，普通compile在test中也不产生例外。
保留DCR008/010两个既有入站元组限制。使用原Plan/finalize、新secret、私有字节check/run；
成功执行配置不得用from_bytes或JSON篡改构造。

## 探测协议与成功标准

新增私有v1首帧 `init_reject`，沿用init密钥/run_id/token字段，另含tcp_port、udp_port。
端口仅用于固定两个目的，不允许任意地址输入。帧/方向总字节上限、禁止密钥日志保持。
新场景消息固定为：init_reject→ready；每phase的 begin_phase→phase_ready；DUT启动后
bootstrap摘要→probe_local→local_probe摘要→finish_phase(dut_stopped:true)→phase_stopped；
三个phase只允许1、2、3顺序出现，最后shutdown(dut_stopped:true)→stopped。
begin_phase只有phase整数；phase1/3必为阳性，phase2必为受保护，无布尔开关或自由组合。
旧场景拒绝新消息，新场景拒绝probe_icmp等不适用命令；新场景的通道健康检查由probe_local
内部在四格结束后运行既有3次虚拟ICMP例外，并并入本阶段摘要，不增加独立可变目标命令。
失败stage/code沿用现有protocol/deadline等固定枚举，不输出原始错误或报文。

四格固定顺序为virtual_tcp、host_tcp、virtual_udp、host_udp，各用一个新流，禁业务重试。
应用载荷20字节：16字节token + phase(u8) + case_id(u8,1..4) + 两个零字节。
TCP须完整写入并收到精确20字节回显；TCP连接建立本身不算应用成功。UDP同socket收精确
应答；所有read/write/connect最多2秒并受阶段/全局绝对截止约束。
peer tun出口观察每格TCP SYN/UDP的正确目的、来源、载荷/长度及实际向WG提交，收到的
回包在tun入口验证地址/端口/校验和与所属phase；不能声称仅发送统计证明DUT认证接收。

local_probe摘要严格给出phase、四格各自的sent/equal_echo、固定错误类别及通道健康结果，
不在peer单独发“拒绝PASS”。Rust合并目标计数后判断：

- phase1/3四格全部精确回显、目标每格收到唯一对应载荷；两轮都有完整WG bootstrap。
- phase2四格必须实际开始探测且无精确回显。只有受保护阶段的预期连接拒绝/reset/EOF/有限
  超时可作为负向观察，解码失败、未启动、未就绪、队列溢出、全局截止等不算拒绝成功。
- phase2所有目标接受/收到计数为零，且本次phase2载荷在整个测试生命周期均未到达；
  任意迟到请求/多余连接/未知载荷均FAIL。最终TCP共4条已验证连接、UDP共4个已验证数据报，
  全部属于phase1/3。目标不回显错误token/phase/序号，也不服务任意业务。
- 四格后虚拟ICMP3/3及同一DUT API存活证明测试期间核心和认证通道仍可工作。
  前后阳性缺一、目标计数读取失败、任一实例异常退出，都只能FAIL/UNAVAILABLE。

这是有限窗口、固定本机拓扑的对照证据；不保证逐个静默丢包都能观察到DUT内部拒绝动作，
也不把历史TCP/UDP阳性记录当作本次前后对照。

## 期限、所有权及实施退出

每phase最多40秒（含DUT check/Ready、bootstrap、四格、ICMP与Stop），全局工作截止135秒；
超时不运行下一phase。listener服务等待事件受全局截止/取消约束，实际业务I/O最多2秒。
目标与peer都保持绑定到所有DUT退出已确认，再关目标socket/线程和peer。任意断言/panic都
先取消探测，停止并确认当前DUT及pending。未确认Stop则不发finish_phase/shutdown、不
启动下一DUT，父保持目标句柄及peer.stdin直至此场景150秒硬截止；随后仍记清理FAIL，
不得声称连续归属或删除未确认的DUT私有资源。父最终159秒、外层160秒；原场景期限不改。
按私有PID核对DUT只有TCP API及既有WG UDP单端口组；Go无OS TCP、只有loopback UDP；
Rust仅所创建两个目标句柄。每阶段有新secret、配置hash/identity和停止确认记录。
运行前后只读核对接口/地址/路由/DNS/代理；不改主机设置、不扫描或终止无关进程。

代码责任限 compiler.rs 的cfg(test)构造/最终校验和负例、Windows Port现有测试模块、
scripts/task009-wg-peer内新场景/主动探测/观察/内测/README。无产品接口、Domain、存储、
依赖或锁修改。实现前完成Task/批准/Readiness同步，Rust/Go可按文件分工；真实核心串行。
验证：封闭规则的错tag/地址/端口/网络/次序/未知字段、非test拒绝例外；消息跨场景/阶段、
迟到/缺bootstrap/假发送/目标关闭不能报PASS；持有资源与截止；真实完整前后对照四格。
回归原DCR009/010 TCP/ICMP/UDP与Hold，使用已约定有界Cargo/Go命令，冻结后独立交付审阅。

必要复杂度为两处目标服务、封闭阳性规则与三阶段测试生命周期。只向关闭端口探测更简单，
但不能区分目标不可用与拒绝；开放通用入站/移除全部reject则超出需要。选择精确例外并保持
原reject，新增风险只限本机进程干扰本次无副作用目标、竞争端口及较长有界资源占用。
这些会导致测试失败，不据此增加防火墙、权限服务、重试、后台守护或产品开关。

一手依据：[固定v1.14.0 WG endpoint](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/wireguard/endpoint.go)
的本地虚拟目的映射、Router调用及WG就绪；[固定Router匹配顺序](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/route/route.go)。
peer侧具体依赖依据见 `wg-reject-peer-feasibility.md`；源码可行性不代替实际固定EXE验证。
