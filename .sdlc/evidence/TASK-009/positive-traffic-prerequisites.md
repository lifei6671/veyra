# TASK-009 正流量计量验证前提

日期：2026-09-04。DCR-005 / CHANGE-003 已获用户确认；本记录是实现前提核定，
不构成真实正流量测试通过，也不授权增加业务流入口。

## 当前可用路径

- 当前 Compiler 只有 `RuntimeProfile::ObservationOnly`，生成 `inbounds: []`；
  固定 TCP API 提供观测，不是 HTTP/SOCKS 业务代理入口。
- 既有 `task009_controlled_network::run_probe` 通过受控 loopback HTTP/SOCKS outbound
  与原生 URLTest 验证 Host/SNI。此前请求成功是协议证据，未断言被 traffic manager 计量。
- 固定上游 URLTest 直接调用成员 outbound 的 `DialContext`，HTTP outbound 直接调用
  client；该调用路径未经过 Router 的 `RoutedConnection` tracker 包装。
  Router 在路由成功后才为业务连接注册 tracker。因此从源码推断，现有 HTTP/SOCKS
  URLTest 的成功不能用来生成或证明被 Clash API 统计的正流量。
- WireGuard peer、合法 TCP/UDP 输入和系统 DNS 拓扑仍未建立授权资源，见
  `network-prerequisites.md`；本次未启动它们，也未据此推断所有可能的 WG 路径都不可计量。

## 结果与后续最小条件

真实核心正流量验证：UNAVAILABLE，当前已核定测试拓扑缺少经 Router 计量的业务输入。
本次未执行新的正流量探测；以上是现有配置/测试与固定上游调用链核定。
本轮可完成 bridge 的非零 Mock 边界、REST/WS 契约和真实核心现有观测回归，
但不能把它们写成真实正流量 PASS。

后续需在单独核定后提出受控业务输入：只能访问 harness 自有目标，精确绑定 loopback、
明确地址/认证/所有权、超时及清理，并由同一 Compiler 产生和 check 最终配置。
需要调整 RuntimeProfile、允许的 inbound 或结构白名单时必须另经设计与用户授权；
CHANGE-003 不授权这些变化，不手改生成 JSON 或借用用户现有代理端口。

验收至少要同时取得目标端可核对的收发字节/请求记录、当前受管 child 的非零窗口值与
累计量、静默后累计量保持、停止/替换后的身份隔离；超时或 URLTest 延迟不是计量证据。

## 可追溯依据

- `src-tauri/src/singbox/compiler.rs`：`RuntimeProfile`、`SingBoxCompiler::compile`。
- `src-tauri/src/singbox/mod.rs`：`compile_probe`、`run_probe`。
- [固定 URLTest](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/common/urltest/urltest.go)：`urlTest`。
- [固定 HTTP outbound](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/http/outbound.go)：`DialContext`。
- [固定 Router](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/route/route.go)：`routeConnection` 中 tracker 注册。
