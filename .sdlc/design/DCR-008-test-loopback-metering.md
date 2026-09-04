# DCR-008：测试构建的本机聚合流量入口

TASK-009 / CHANGE-006。用户于2026-09-04回复“允许”，批准
`loopback-metering-proposal.md` 的本机测试方案。仅替代DCR-001在本测试中的无业务入站限制；
ObservationOnly产品配置、公共RuntimeProfile及其余安全、生命周期、统计契约保持。

## 编译与配置

- 在`compiler.rs`新增`cfg(test)`的`SingBoxCompiler::compile_loopback_metering`内部方法，
  接受既有RuntimeIntent/default_target/DnsPolicy和两个NonZeroU16测试端口，返回SingBoxPlan。
  先正常编译ObservationOnly，再在私有强类型Document内建立唯一direct TCP入站。
  不增加公开Profile，不把任意地址、JSON、字段或网络类型作为方法输入。
- 入站只含type=direct、tag=test-metering、listen=127.0.0.1、listen_port、network=tcp、
  override_address=127.0.0.1及override_port；两个端口互不相同且均非9090。
  测试编译模型deny_unknown_fields；验证数量、所有固定字段、端口和对应路由。
  非test编译保留空入站模型与拒绝全部业务入站的规则。普通compile在test中也仍生成空入站。
- 测试意图使用既有`IpCidr(127.0.0.1/32) → Direct`，优先级先于其它用户路由；
  没有WireGuard端点、URLTest、hostname节点或其它会主动联网的配置。默认Pool沿用必需模型，
  其中仅Manual选择的loopback节点；本次唯一业务流必定经过Direct。
- 同一Plan绑定每实例新secret、validate、序列化、最终字节读回及固定核心check，
  check/run前仍验证完全相同字节。不使用from_bytes或JSON修改制造可执行配置；
  from_bytes仅用于负向拒绝测试，不能进入成功运行路径。

## 本机拓扑与归属

测试客户端 → 127.0.0.1:动态入口 → 受管sing-box Router → Direct → 本次回显服务。
核心另有固定鉴权API127.0.0.1:9090。只使用仓库已固定的Windows amd64 1.14.0资产及标准库。
真实用例放在WindowsManagedSidecarPort已有cfg(test)模块，直接读取私有running child归属，
复用PrivateRuntime/ACL和SidecarRuntime，不暴露PID或secret到产品契约。

回显listener先bind 127.0.0.1:0并保持句柄；入口先预留另一个loopback端口，启动前释放。
释放存在端口争用窗口；Ready后按受管child PID枚举实际TCP listeners，精确断言只有
固定API和本次入口，全部loopback。无法取得归属证据或端口被占用即失败，不杀无关进程、
不自动重试。可复用既有Windows TCP枚举能力；必要只增加cfg(test)内部访问，不新增依赖。

本机其它进程可能连接短时入口，但仅能到本次无副作用回显服务。每实例只允许一条测试流，
最多收发各2MiB，不使用外网、任意代理、UDP或DNS。连续发送已知模式跨多个采样窗口后静默。
每次I/O最多2秒，服务全局30秒，单次用例外层60秒；沿用check10秒、Ready2秒、Stop2秒。
有界等待不作为错误重试；只为既有期限内进程/采样观察和受控发送节奏使用。

无论断言成功失败均关闭测试socket、通知服务退出、join线程、停止并确认自有child及pending；
确认退出与私有资源清理后才删除本次路径，解析后的路径必须在本次自有根内。
运行前后只读比较系统代理、DNS、接口与路由；不改hosts、网络设置、权限或服务。

## 验收与范围

1. 普通编译无入站；测试模型拒绝未知/重复字段、非loopback地址、UDP、额外入站、
   零/9090/相同端口及不满足Direct隔离路由的意图。库非test编译通过。
2. 正向用例绑定最终配置SHA256、opaque identity及私有child PID；枚举精确listeners，
   记录回显正确字节总量和相同child鉴权API的非零上下行窗口速率、REST累计。
   已知字节收发完成后累计精确相等；REST/WS不原子，最终比较在静默阶段。
3. 静默跨完整核心窗口后速率归零、累计不变；停止后不可采样。
   同一Port新实例identity增加、新secret不等且旧secret鉴权失败，新实例累计从零开始。
   用真实采样喂既有InMemoryRuntimeObservations，确认Stop/新Ready清空两图共用历史；
   UI布局/时间窗本身沿用已通过的DCR-007验证，不冒充真实WebView E2E。
4. 同一固定核心的完整配置与计量路径分别记录；这是测试专用配置，不把它作为
   ObservationOnly产品支持入口、完整WG/DNS或worker网络in-flight Stop的证明。

源范围仅`compiler.rs`测试构建适配/测试、`platform/windows/managed_sidecar_port.rs`内测，
必要时既有`singbox/managed_sidecar.rs`的cfg(test)归属读取，以及Task/DCR和证据。
无依赖/锁文件、产品IPC/UI、Domain、存储、主机配置改变。计划独立设计审阅通过后实施，
以现有Cargo test/clippy/fmt及git diff --check验证，并独立审阅交付增量。

依据：已批准本机测试提案；固定v1.14.0 direct入站源码经RouteConnectionEx路由，
与[官方direct入站字段](https://sing-box.sagernet.org/configuration/inbound/direct/)相符。
这些依据不替代实际计量测试。完整TASK-009仍需其它并发及SF-003验证，不迁移Task/Gate验收状态。
