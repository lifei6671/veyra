# TASK-008 ObservationOnly DNS 编译契约

本文件细化 DCR-002 的 Domain DNS 来源约束。授权来源为 USER:lifei 在 2026-09-04 对
TASK-008 最小范围与 DNS 行为修订的明确回复“批准”，对应
`.sdlc/evidence/TASK-008/readiness.yaml#EVIDENCE-TASK-008-READINESS-001` 的提案。
该授权不改变 AppState schema、Storage、CaptureMode、IPC 或 child 生命周期。

## 强类型输入与来源

- Domain 提供非持久化、封闭的 `DnsPolicy`，当前只有系统解析策略。它没有任意地址、
  JSON、远程 DNS、FakeIP 或路径字段，不进入 AppState 的序列化与迁移。
- Application 从经过整体校验的 AppState 构造编译输入：保留其精确 `default_target`，
  携带既有 `RuntimeIntent`，并显式选择系统 DNS 策略及 `RuntimeProfile::ObservationOnly`。
- 编译输入中的默认目标必须是已启用、非空的 Pool。`Unconfigured`、Direct、Block、
  不存在或禁用的 Pool 一律拒绝；不从节点、Pool 的位置或名称推断目标。
- 不增加其它 RuntimeProfile 或可配置 DNS 模式。现有 `RuntimeIntent` 的持久化无关结构
  可以保持不变，由后端内部编译输入将默认目标和 DNS 策略一起传递。

## 系统 DNS 的配置映射

- DNS 仅生成一个固定 tag 为 `dns-system`、type 为 `local` 的 server，设置
  `dns.final = dns-system`。不设置远程地址、detour、FakeIP、neighbor_domain、缓存文件或
  文件路径。使用内核的系统解析默认行为，不强制 IPv4/IPv6 偏好。
- `route.default_domain_resolver` 明确引用 `dns-system`，供需要解析代理服务器域名的
  dialer 使用；不得依靠空 `dns: {}` 或第一个 DNS server 的隐式选择。
- URLTest 请求使用其既定 Pool 成员，代理服务器的域名由系统 DNS 解析。支持将目的域名
  交给远端代理的协议仍保留协议原有行为：系统 DNS 策略不额外启用本地目的地址预解析，
  不将订阅节点改写为固定 IP，也不额外建立 DNS 代理服务器。
- `inbounds` 显式为空，唯一受管 API 仍为持有私有 secret 的 `127.0.0.1:9090`。
  DNS 配置本身不增加监听器或修改 Windows 系统 DNS。

## 最终化与失败边界

- `SingBoxPlan` 包含上述类型化 DNS、Pool/Route/default 与节点配置；绑定运行期 secret 后，
  严格结构白名单验证完整配置，再产生不可由调用方重写的最终 `GeneratedConfig` 字节。
- 最终字节经既有私有 ACL 目录写入并执行固定资产 `sing-box check`；check 后不得注入
  secret、改写 DNS、默认目标或其它字段。任何失败均不触及 active/previous 配置槽。
- 生命周期实现留在 TASK-009；本任务新增真实核心验证只执行 version/check。

## 最小范围补充

- `src-tauri/src/domain/{state.rs,mod.rs}`：仅新增上述非持久化 DNS 类型、必要导出及定向测试。
- `src-tauri/src/singbox/runtime.rs`：仅适配测试中的编译输入和有效 Pool fixture，保留既有
  check/prepare/run/ready/stop/回滚事件断言；不修改生产生命周期代码。
- 原 TASK-008 已允许的 Compiler、固定 check 边界和 Application 调用方按此契约接线。

## 验证

- 验证 DNS server 唯一、固定 local 类型、final 和 default_domain_resolver 引用一致，
  未生成 FakeIP、远程 DNS、额外 listener、任意路径或未知字段。
- 以 hostname 节点和 URLTest Pool fixture 验证系统解析器引用；明确区分配置 check 与
  真实 DNS/代理通信证据，后者不由 check 冒充。
- 覆盖默认 Pool 的精确传递与所有拒绝分支、显示名与排序稳定性、相同绑定输入的相同字节、
  secret 脱敏、最终配置字节写入读回及固定核心接受性。
- AppState 序列化字段不变；既有 runtime 事务测试保留完整断言。

## 一手参考

- [sing-box Local DNS](https://sing-box.sagernet.org/configuration/dns/server/local/)
- [sing-box Route](https://sing-box.sagernet.org/configuration/route/)

字段语法最终由仓库固定 Windows amd64 sing-box 1.14.0 资产的 check 验证；线上文档不是
其它协议、资源或运行能力的额外授权。
