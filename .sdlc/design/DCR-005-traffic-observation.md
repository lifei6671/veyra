# DCR-005：固定核心流量速率与累计值

状态：PROPOSED，待独立设计审阅及用户批准；本文件不修改已冻结 DCR-001 或 TASK-009 Scope。
目标：TASK-009 SF-002 正流量观测。需求仍为当前实例的安全速率、累计量与连接数摘要。

## 已证实的问题

固定 sing-box 1.14.0 的 `/traffic` 在每次 WebSocket 连接建立时保存累计量，每个一秒 tick
返回与上一 tick 的差值。它是该窗口的字节数，不是实例启动以来的累计量。
当前 `RuntimeObservationBridge::record_traffic_sample` 对相邻窗口再次差分，并把窗口值写为
累计量。例如连续两秒各上传 100 字节，现算法第二次速率为 0、总量仍为 100。
这是静态契约缺陷证据，尚不等同于真实核心正流量验证。

`/connections` 的 `uploadTotal` / `downloadTotal` 来自同一核心 traffic manager 的累计量，
已包含已关闭连接。现有 `read_connections` 已安全提取这些字段并丢弃连接明细。

## 最小修正

1. `/traffic.up/down` 作为固定核心一秒窗口的字节速率直接映射，不再对相邻窗口二次差分。
   这是核心提供的一秒窗口采样值；不宣称进程调度停顿时仍是精确墙钟瞬时速率。
2. 累计量来自同一受管实例的现有 `/connections` 安全摘要。不对离散 WS 窗口求和，
   因为每次单帧连接之间存在采样间隙，窗口求和会漏计。
3. REST 与 WS 是同一串行采样中的两个时间点，不宣称它们为原子快照；累计量代表
   REST 读取时点、速率代表随后 WS 窗口。既有 DTO、IPC、字段单位及安全摘要形状不变。
4. 单调时间仅用于每流至少一秒的采样节流，不用本地握手/两次请求间隔除核心窗口值。
   保留每流最多一条 in-flight、固定地址/鉴权、2s 握手及首帧、16 KiB 和单消息限制。
5. 新 child 使用新的 bridge；停止、失败和替换继续丢弃旧采样状态。错误仍闭合失败。
   删除仅服务于错误累计差分的内部状态/函数；不增加重试、持久化、配置项或依赖。

## 拟授权范围

- 扩展 `src-tauri/src/singbox/clash_api.rs`：上述流量语义、内部字段/注释和定向内测。
- 现有 `application/observability.rs` 与 `application/runtime.rs`：仅必要的安全 DTO 映射
  断言/内部调用适配；不更改公开事件、IPC、存储或 UI 字段。
- TASK-009 已允许的测试文件/内测：正流量与实例隔离验证；真实网络拓扑仍需单独核定。
- 批准后同步 DCR-001 对应流量段和 TASK-009 allow，并重新冻结、验证、独立审阅。

WireGuard peer、UDP 流输入、系统 DNS、新第三方依赖或非 loopback 暴露不随本方案批准。

## 验证与验收

- 固定边界测试：连续等量窗口、增大/减小窗口、静默窗口、非零首帧，速率均等于本帧
  的核心窗口值；累计值严格来自 REST，不能被窗口值覆盖。
- 单调时间间隔、同流串行、未知/负数/超大 JSON 失败、不同实例清空旧状态继续验证。
- 安全 DTO 断言同时核对速率与累计值，保持既有字段，不泄露连接/凭证/原始载荷。
- 真实核心正流量：先核定当前 ObservationOnly 配置是否有受控、可计量的业务流入口。
  仅 HTTP 请求成功或 URLTest 成功不能证明 traffic manager 已计量；必须同时有目标端
  收发记录与同一受管 child 的非零 API 数值。若现有 profile 无法产生被计量流量，记录
  UNAVAILABLE 并另行提出最小测试入口，不改写生成 JSON、不增加产品入口。
- 使用 TASK-009 现有 Cargo/fmt/Clippy 命令；所有执行有界，记录目标、退出码及真实结果。

## 一手依据

- [固定版本 traffic handler](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/experimental/clashapi/server.go)，`traffic`。
- [固定版本 connectionsSnapshot](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/experimental/clashapi/connections.go)。
- [固定版本累计量实现](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/common/trafficcontrol/manager.go)，`Total`。
- 当前源 `clash_api.rs` 的 `record_traffic_sample` 与 DCR-001 固定 WebSocket 流摘要契约。

评估日期：2026-09-04。现有 CHANGE-002 明确排除流量算法修正，因此不能用此前两文件批准
替代本提案的授权。实现及真实正流量验证尚未执行。
