# DCR-014：DNS 预检禁色编码纠正

状态：CANDIDATE，待独立技术审阅。TASK-009 / SF-003 / CHANGE011 的等义验证实现修正。
原设计 DCR013 身份：3b5bf2627d009844e340277adbeffdf73e47a6638afeeedcd675d405f6877181。
失败实现 checkpoint020：8f017b9861baa57aeeb1a593cc45c6bcf3774504b791598c90d360d5627f6406。

## 原因与影响

真实预检在 CandidateCheck 失败，run/DNS 未启动；peer 0包正常关闭，私有配置清理完成。
固定核心 option/options.go 的 LogOptions.DisableColor 为 `json:"-"`，不是 JSON 字段；
未知字段被解码器拒绝。DCR013 的 `disable_color:true` 因此不可实现。原失败、预审和
已批准DCR013字节保留，不能把源码推断或本地parser单测当作该核心的接受证据。

只纠正“无色日志”的编码位置：配置保留三字段
`log:{disabled:false,level:"debug",output:"stderr"}`；只在通过最终候选完整校验、
与pending字节hash绑定的 cfg(test) DNS run 启动分支中增加固定 `--disable-color`。
命令参数为 `run --disable-color -c <同一已check的私有配置路径>`。它不是可变参数接口，
不接受用户flag或额外路径；普通产品run、其它测试run与check仍使用原参数及null标准流。

## 保留的约束

DCR013全部其它范围、拓扑、域名、DNS transport、reject规则、无业务inbound、唯一WG/
URLTest、熵源、私有ACL、check→同字节run→Ready、采集限额、精确日志链、25ms轮询、
三个收集截止、Stop→EOF/join→peer和完整SF003验收原样继承。没有ANSI过滤兼容层。
JSON中 `disable_color`、`timestamp` 或其它字段一律拒绝；三字段须全部显式存在、正确，
禁止null/重复/缺失或与普通关闭日志配置混合。未知格式仍失败，不降低证据要求。

允许实现变更仅 compiler.rs 的 cfg(test) LogConfig/封闭元组与用例，以及
managed_sidecar.rs 的 cfg(test) 精确DNS run参数及本地argv用例。Port只在确有必要时
更新相应用例，不改变生命周期或Go协议。Go实现和依赖均无需修改。

## 变化控制与授权判断

这是当前Scope内的 equivalent Verification，按 change-control.md 的等效实现条款
复用用户对CHANGE011“无色日志+私有有界采集+指定域名运行”的明确批准。没有公共API、
产品/持久化/运维契约、主机配置、权限、外部成本或测试所证明语义的变化；无需再次
请求同一能力的Human批准。不是借新flag扩大命令入口或放宽日志内容。
仍先独立技术审阅本纠正，再由Orchestrator记录采纳；不修改旧冻结设计来掩盖错误。

失效：本次编译器/child测试、实现候选身份、交付审阅与真实DNS证据；新运行必须再次
check成功、同字节run、记录创建时间/PID/hash/typed摘要与清理。旧Go纯本地/race/vet/
build及其它源未变化部分可逐hash继承。产品Clippy和fmt重新检查；HTTP/SOCKS与WG回归
若不重复，明确以未受影响的原分支和源码hash为继承依据，不冒称重跑。

## 验证与停止条件

1. 本地argv断言：只有固定DNS测试run带一次固定flag；普通run/check无flag，stdio不变。
2. 编译器三字段与整个拓扑封闭；旧错误字段、重复/null/缺失/未知字段负例通过。
3. 定向Rust验证与独立交付复审后，执行已批准同一真实预检一次。失败保留证据并定位，
   不换域名、transport、TUN或目标，也不放宽解析器。
4. 五类主机快照前后一致，DUT/reader/peer和私有资源完整清理。缺查询/结果是UNAVAILABLE。

## 一手依据

固定 revision `0b8995879f29a9b98ee027bc17b75e101445b238`：
- [LogOptions](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/option/options.go)：未知字段拒绝，DisableColor不接受JSON。
- [CLI flag](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/cmd/sing-box/cmd.go)：固定disable-color布尔参数。
- [run create](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/cmd/sing-box/cmd_run.go)：该参数设置内部Log.DisableColor。
- [check](https://raw.githubusercontent.com/SagerNet/sing-box/0b8995879f29a9b98ee027bc17b75e101445b238/cmd/sing-box/cmd_check.go)：原配置检查保持，标准流仍null。
