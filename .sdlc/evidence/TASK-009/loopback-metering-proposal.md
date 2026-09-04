# 真实聚合流量验证：仅测试构建的本机入口候选

状态：PROPOSED / 尚未批准、尚未实现或启动。2026-09-04。

当前产品 ObservationOnly 配置没有业务入站，因此正常启动与观测API读取不能证明流量计量。
固定核心 direct 入站会经过 Router.RouteConnectionEx；支持仅TCP及固定目标地址/端口。
依据：[官方字段文档](https://sing-box.sagernet.org/configuration/inbound/direct/)、
[v1.14.0实际入站实现](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/protocol/direct/inbound.go)。
这是候选路径的源码依据，不是已运行成功的证据。

## 请求授权的最小范围

- 仅 `cfg(test)` 的 Compiler 测试入口生成一个严格白名单的 `direct` TCP 入站：
  `listen=127.0.0.1`，动态测试端口，`override_address=127.0.0.1`，
  `override_port` 为本次测试自有回显服务端口；无UDP、无任意代理目标、无外部网络。
- 用既有RuntimeIntent的 `IpCidr(127.0.0.1/32) → Direct` 规则，保留正常Compiler的
  模型校验、实例secret绑定、最终字节读回校验及固定核心check；禁止手工修改生成JSON。
- 在 `compiler.rs` 为该测试入口、测试构建的封闭入站模型和对应校验做必要适配；
  正常产品构建继续拒绝全部入站，产品IPC/RuntimeProfile、配置文件和页面无新增入口。
- 测试放在既有 `singbox/mod.rs` 或 Windows Port 文件的cfg(test)模块，沿用固定资产、
  私有目录/ACL、受管child生命周期及固定API锁。无新增第三方依赖或锁文件变更。

## 拓扑、期限与清理

```text
本机测试客户端 → 127.0.0.1:测试入口 → 同一受管sing-box Router
              → Direct → 127.0.0.1:本次回显服务
                        127.0.0.1:9090 固定鉴权观测
```

两个新增TCP端口均仅loopback，回显服务没有业务副作用，最多接受一条流、收发各不超过2MiB。
本机其他进程可以连接该短期loopback入口；入口目的固定为这个无副作用、有限流量的测试服务，
不提供到其他地址的转发。端口冲突立即失败，不终止无关进程、不自动重试。
测试占用端口后释放给child存在竞争窗口；启动后核对listener归属，无法确认即失败并清理自有资源。
单次测试外层60秒，服务全局30秒，单次I/O最多2秒；沿用check/Ready/stop既有期限。
断言失败仍停止并等待自有child、关闭测试socket并join线程；确认退出后才删除本次私有目录。
只读比较运行前后系统代理、DNS、路由和接口；不修改hosts、DNS、代理、路由、TUN或权限。

## 必须取得的证据

1. 按最终配置哈希和受管child身份绑定目标收发记录、鉴权观测的非零速率与累计量。
2. 持续发送已知字节模式跨越采样窗口，核对回显内容/总量；停止发送后累计量稳定而速率归零。
3. Stop与新实例启动后观测身份/secret隔离，双图历史不混入旧实例。
4. 正常非test编译仍生成空inbounds；未知字段、非loopback、UDP及不受控目的均被拒绝。
5. 全部记录标为“测试专用计量配置”；不能冒充原ObservationOnly的计量结果。

这只解决真实TCP聚合计量前提，不关闭WG UDP、ICMP、系统DNS或真正worker网络in-flight Stop缺口。
若批准，下一步用technical-design技能把测试配置例外纳入DCR-001/Task变更并独立审查后实施。
本候选不更改当前冻结设计、Task验收、生产实现或现有验证结论。
