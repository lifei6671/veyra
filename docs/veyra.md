# 跨平台 sing-box 桌面客户端技术设计方案 V0.1

## 1. 文档目标

本文定义一个基于 Tauri 2 + React + Rust + sing-box 的跨平台桌面网络代理客户端技术方案。

项目目标不是简单实现一个 sing-box GUI，而是建立一套独立于具体代理内核配置格式的应用领域模型，在产品层提供简单、稳定、可扩展的网络管理能力。

核心能力包括：

* 多订阅管理
* 多节点来源隔离
* 节点标准化
* 自定义出口组
* 跨订阅节点组合
* 手动节点选择
* 自动节点选择
* 按网站和服务分流
* 按应用程序分流
* 自定义规则分流
* TUN 模式
* 系统代理模式
* DNS 管理
* Rule Set 管理
* 实时连接查看
* 实时上下行流量
* 运行日志
* 原生 sing-box 配置高级模式
* Windows / macOS / Linux 跨平台运行
* 快速托盘恢复
* 内核配置校验与回滚
* 后续增加其他网络内核的架构空间

产品交互层参考 Clash Verge Rev 的信息架构与桌面交互经验。

订阅标准化、统一节点模型、配置编译方式参考 Satelite Proxy 的架构思想。

底层运行能力由 sing-box 提供。

本项目拥有自己的：

```text
Subscription Model
ProxyNode Model
Provider Model
NodePool Model
RoutePolicy Model
RuntimeIntent
SingBox Compiler
```

避免应用层与 sing-box JSON 直接耦合。

---

# 2. 核心设计原则

## 2.1 用户不需要理解 sing-box

用户不应该为了使用客户端而理解：

```text
inbound
outbound
selector
urltest
detour
route action
rule_set
clash_api
endpoint
```

这些属于基础设施层。

用户层只使用：

```text
订阅
节点
出口组
分流
连接
日志
设置
```

例如用户看到：

```text
高速下载
B机场 + C机场
自动选择
```

实际运行配置可能是：

```text
urltest outbound
   ├── B-HK
   ├── B-JP
   ├── C-HK
   └── C-SG
```

UI Model 与 Runtime Model 不要求使用相同术语。

---

## 2.2 Subscription 与 Runtime 完全解耦

订阅不是运行配置。

正确流程：

```text
Subscription
     ↓
Subscription Parser
     ↓
Normalized ProxyNode
     ↓
Domain
     ↓
RuntimeIntent
     ↓
SingBoxCompiler
     ↓
config.json
```

禁止：

```text
Clash YAML
     ↓
字段替换
     ↓
sing-box JSON
```

任何外部订阅都必须先进入统一领域模型。

---

## 2.3 配置转换必须是语义转换

例如：

用户配置：

```text
出口组：
手动选择节点
```

Compiler：

```text
selector
```

用户配置：

```text
出口组：
自动选择最快节点
```

Compiler：

```text
urltest
```

未来增加其他 Backend 时：

```text
自动选择节点
       ↓
Domain Intent
       ↓
Xray Backend
       ↓
balancer + observatory
```

应用业务语义不随内核变化。

---

## 2.4 V0.1 只支持 sing-box

V0.1 只实现：

```text
sing-box
```

不提前引入：

```text
CoreKind
XrayBackend
MihomoBackend
MultiCore
```

避免过度抽象。

但是以下模型必须保持中性：

```text
Subscription
Provider
ProxyNode
NodePool
RoutePolicy
DnsPolicy
RuntimeIntent
```

sing-box 专属结构只能存在：

```text
infrastructure/singbox/
```

---

## 2.5 配置是一个整体状态图

本项目的配置数据本质不是数据库业务数据，而是一份：

> 可以整体加载、整体验证、整体编译的应用配置状态。

因此 V0.1 采用：

```text
Versioned JSON State Store
```

而不是 SQLite。

应用启动：

```text
state.json
   ↓
Deserialize
   ↓
Migration
   ↓
Reference Validation
   ↓
Domain State
```

运行期间以内存 Domain State 为主要 Source of Truth。

---

## 2.6 配置态与运行态严格分离

配置态：

```text
订阅
Provider
节点
出口组
规则
DNS
TUN
设置
```

需要持久化。

运行态：

```text
当前连接
实时速度
进程状态
当前 CPU
当前内存
临时连接数据
```

默认只存在内存中。

禁止为了显示监控数据频繁写入 `state.json`。

---

# 3. 产品定位

产品定位：

> 一个基于 sing-box 的现代跨平台网络分流与多出口桌面客户端。

主要面向两类用户。

## 3.1 普通用户

典型流程：

```text
添加订阅
   ↓
自动得到出口组
   ↓
选择节点
   ↓
开启系统代理或 TUN
```

普通用户无需接触 sing-box 配置格式。

---

## 3.2 高级用户

支持：

```text
多个订阅
多个出口组
跨订阅节点组合
网站分流
服务分流
应用分流
Rule Set
自定义 DNS
自定义本地入口
Native sing-box Profile
```

---

# 4. 典型场景

## 4.1 多机场

例如：

```text
A机场：
稳定、便宜

B机场：
下载速度快

C机场：
上传速度快
```

配置：

```text
普通网页
→ A机场

YouTube / 视频
→ B机场

qBittorrent / aria2
→ B机场

rclone / WinSCP
→ C机场

默认
→ A机场
```

---

## 4.2 跨机场组合

例如：

```text
高速下载
```

包含：

```text
B机场
├── 香港
├── 日本
└── 新加坡

C机场
├── 香港
└── 日本
```

然后由 URLTest 自动选择。

---

## 4.3 应用分流

例如：

```text
Chrome
→ A机场

qBittorrent
→ 高速下载

rclone
→ 大文件上传

Steam
→ DIRECT
```

---

# 5. 非目标

V0.1 不追求：

* 完整兼容 Clash 配置
* 完整继承 Clash Proxy Group
* 完整转换 Clash Rule Provider
* 完整继承 Clash DNS
* Clash Script
* Mihomo Script
* JavaScript Profile Transform
* Xray
* Mihomo
* Multi-Core
* QoS
* MITM
* 自动内容识别
* 单连接出口动态迁移
* 流量已经开始后更换出口
* 云端账号体系
* 移动端
* 内置公共代理节点
* 代理服务运营

特别说明：

```text
连接建立
   ↓
路由决策
   ↓
选择出口
   ↓
建立代理连接
```

连接建立后通常不能无感切换到另一条代理链路。

---

# 6. 技术栈

## 6.1 Desktop

```text
Tauri 2
```

职责：

* Desktop Window
* System Tray
* Native Menu
* Auto Start
* Updater
* IPC
* File System
* Native Dialog
* OS Integration
* Privilege Management

---

# 7. Frontend

建议：

```text
React
TypeScript
Vite
```

推荐：

```text
React Router
Zustand
TanStack Query
```

UI 基础组件：

```text
Radix UI
或
shadcn/ui
```

设计风格保持独立。

不直接复制其他客户端：

```text
Logo
品牌
视觉资产
像素级布局
文案
```

---

# 8. Backend

```text
Rust
Tokio
Serde
serde_json
reqwest
```

主要职责：

```text
Domain State
Subscription Parser
State Persistence
Schema Migration
Configuration Validation
SingBox Compiler
SingBox Runtime
Process Manager
Clash API Client
System Proxy
TUN
Platform Integration
Traffic Monitoring
Connection Monitoring
Updater
```

---

# 9. Core

```text
sing-box
```

第一阶段采用：

```text
Sidecar Binary
```

即：

```text
Desktop App
    ↓
Process Manager
    ↓
sing-box binary
```

暂不直接嵌入 sing-box Go library。

---

# 10. 总体架构

```text
┌───────────────────────────────────────┐
│                 UI                    │
│                                       │
│ Overview                              │
│ Proxies                               │
│ Subscriptions                         │
│ Routing                               │
│ Connections                           │
│ Logs                                  │
│ Settings                              │
└────────────────────┬──────────────────┘
                     │
                  Tauri IPC
                     │
┌────────────────────▼──────────────────┐
│           Application Layer           │
│                                       │
│ SubscriptionService                   │
│ ProxyService                          │
│ PoolService                           │
│ RoutingService                        │
│ RuntimeService                        │
│ SettingsService                       │
└────────────────────┬──────────────────┘
                     │
┌────────────────────▼──────────────────┐
│               Domain                  │
│                                       │
│ AppState                              │
│ Subscription                          │
│ Provider                              │
│ ProxyNode                             │
│ NodePool                              │
│ RoutePolicy                           │
│ DnsPolicy                             │
│ RuntimeIntent                         │
└────────────────────┬──────────────────┘
                     │
┌────────────────────▼──────────────────┐
│           Infrastructure              │
│                                       │
│ StateStore                            │
│ Subscription Parser                   │
│ SingBox Compiler                      │
│ SingBox Runtime                       │
│ Clash API                             │
│ Platform Adapter                      │
└────────────────────┬──────────────────┘
                     │
                  sing-box
```

---

# 11. 推荐工程目录

```text
src-tauri/src/

application/
    subscription_service.rs
    proxy_service.rs
    pool_service.rs
    routing_service.rs
    runtime_service.rs
    settings_service.rs

domain/
    state.rs
    subscription.rs
    provider.rs
    node.rs
    pool.rs
    routing.rs
    dns.rs
    runtime.rs

storage/
    mod.rs
    store.rs
    snapshot.rs
    migration.rs
    validation.rs

subscription/
    mod.rs
    detector.rs
    clash.rs
    singbox.rs
    uri.rs
    base64.rs

singbox/
    compiler/
        mod.rs
        outbound.rs
        pool.rs
        inbound.rs
        routing.rs
        dns.rs
        ruleset.rs

    api/
        client.rs
        proxies.rs
        connection.rs
        traffic.rs

    runtime/
        process.rs
        config.rs
        health.rs
        logs.rs

platform/
    mod.rs
    windows/
    macos/
    linux/

commands/
    subscription.rs
    proxy.rs
    pool.rs
    routing.rs
    runtime.rs
    settings.rs
```

Frontend：

```text
src/

pages/
    Overview/
    Proxies/
    Subscriptions/
    Routing/
    Connections/
    Logs/
    Settings/

features/
    subscription/
    provider/
    node/
    pool/
    routing/
    runtime/

components/
stores/
services/
hooks/
```

---

# 12. AppState

整个用户配置由一个顶层 Aggregate 管理。

```rust
struct AppState {
    schema_version: u32,

    subscriptions: Vec<Subscription>,

    providers: Vec<Provider>,

    nodes: Vec<ProxyNode>,

    pools: Vec<NodePool>,

    routes: Vec<RoutePolicy>,

    rule_sets: Vec<RuleSet>,

    local_inbounds: Vec<LocalInbound>,

    dns: DnsPolicy,

    settings: AppSettings,
}
```

所有配置编辑操作首先修改：

```text
AppState
```

然后整体校验和持久化。

---

# 13. Subscription

Subscription 表示：

> 节点数据从哪里获得。

```rust
struct Subscription {
    id: SubscriptionId,

    name: String,

    source: SubscriptionSource,

    enabled: bool,

    auto_update: bool,

    update_interval: Duration,

    last_update_at: Option<DateTime>,

    last_error: Option<String>,

    traffic: Option<SubscriptionTraffic>,
}
```

---

# 14. SubscriptionSource

```rust
enum SubscriptionSource {
    RemoteUrl {
        url: String,
    },

    LocalFile {
        path: PathBuf,
    },

    InlineText {
        content: String,
    },

    SingleNode {
        uri: String,
    },

    NativeSingBox {
        content: String,
    },
}
```

---

# 15. Provider

Provider 表示：

> 用户认知中的节点来源，例如一个机场。

```rust
struct Provider {
    id: ProviderId,

    name: String,

    subscription_ids: Vec<SubscriptionId>,

    enabled: bool,
}
```

大多数情况下：

```text
1 Subscription
    ↓
1 Provider
```

但 Domain 不强制 1:1。

---

# 16. 用户层不暴露 Provider

用户看到：

```text
A机场
```

内部：

```text
Subscription A
Provider A
Implicit Pool A
```

Provider 是内部领域概念。

---

# 17. ProxyNode

所有订阅解析最终产生：

```rust
struct ProxyNode {
    id: NodeId,

    provider_id: ProviderId,

    name: String,

    protocol: Protocol,

    server: String,

    port: u16,

    tls: Option<TlsConfig>,

    transport: Option<Transport>,

    config: ProtocolConfig,

    udp: Option<bool>,
}
```

---

# 18. Protocol

V0.1 优先支持：

```rust
enum Protocol {
    Shadowsocks,

    VMess,

    VLess,

    Trojan,

    Hysteria2,

    TUIC,

    Socks5,

    Http,

    WireGuard,

    AnyTLS,
}
```

后续：

```text
SSH
Hysteria1
Tor
Naive
ShadowTLS
Snell
```

---

# 19. TLS

```rust
struct TlsConfig {
    enabled: bool,

    server_name: Option<String>,

    insecure: bool,

    alpn: Vec<String>,

    utls: Option<UtlsConfig>,

    reality: Option<RealityConfig>,
}
```

---

# 20. Transport

```rust
enum Transport {
    Tcp,

    WebSocket {
        path: String,
        headers: Map<String, String>,
    },

    Grpc {
        service_name: String,
    },

    HttpUpgrade {
        path: String,
        host: Option<String>,
    },
}
```

---

# 21. Node Identity

禁止使用：

```text
Node Name
```

作为唯一 ID。

例如：

```text
香港01
香港01
香港01
```

非常常见。

建议：

```text
NodeId =
stable_hash(
    provider_id
    protocol
    server
    port
    credential_identity
)
```

显示名和 identity 分离。

---

# 22. Subscription Parser

统一入口：

```rust
fn parse_subscription(
    source: &SubscriptionSource,
    body: &str,
) -> Result<ParseResult>;
```

结果：

```rust
struct ParseResult {
    format: SubscriptionFormat,

    nodes: Vec<ProxyNodeDraft>,

    skipped: Vec<SkippedNode>,

    metadata: SubscriptionMetadata,
}
```

---

# 23. 格式识别

建议顺序：

```text
JSON
 │
 ├── sing-box
 └── Clash JSON

YAML
 │
 └── Clash YAML

URI List

Base64
 │
 └── Decode
       ↓
     Recursive Detect
```

---

# 24. URI 支持

第一阶段：

```text
ss://
vmess://
vless://
trojan://
hysteria2://
hy2://
tuic://
socks5://
http://
https://
anytls://
```

---

# 25. Clash Subscription 原则

Clash 配置：

```yaml
proxies:
proxy-groups:
rules:
rule-providers:
dns:
tun:
```

V0.1 只承诺解析：

```yaml
proxies:
```

也就是：

```text
Clash Subscription
       ↓
Node Extraction
       ↓
ProxyNode
```

不会承诺完整继承：

```text
Proxy Groups
Rules
DNS
TUN
Script
```

---

# 26. 为什么只提取节点

因为客户端自己的：

```text
出口组
分流
DNS
TUN
```

由独立 Domain 控制。

否则很容易出现：

```text
Clash Domain
     +
SingBox Domain
     +
App Domain
```

三套模型互相污染。

---

# 27. Native sing-box Profile

高级用户允许直接运行完整 sing-box 配置。

例如：

```json
{
  "log": {},
  "dns": {},
  "inbounds": [],
  "outbounds": [],
  "route": {}
}
```

执行：

```text
Native Config
    ↓
sing-box check
    ↓
sing-box run
```

不进入 Managed Domain。

---

# 28. Native Mode 限制

Native Mode 下客户端提供：

```text
启动
停止
日志
连接
流量
Clash API
```

不保证提供：

```text
出口组编辑
分流编辑
节点编辑
DNS 编辑
```

因为这些配置已经由用户自己控制。

---

# 29. NodePool

用户层：

> 出口组

Domain：

```rust
struct NodePool {
    id: PoolId,

    name: String,

    kind: PoolKind,

    sources: Vec<PoolSource>,

    selection: SelectionPolicy,

    enabled: bool,
}
```

---

# 30. PoolKind

```rust
enum PoolKind {
    ImplicitProvider,

    Custom,
}
```

ImplicitProvider：

```text
Subscription
    ↓
Provider
    ↓
自动出口组
```

Custom：

用户创建：

```text
高速下载
上传专线
AI服务
低延迟
```

---

# 31. PoolSource

```rust
struct PoolSource {
    provider_id: ProviderId,

    filter: NodeFilter,
}
```

这样可以表达：

```text
B机场：
香港、日本

C机场：
美国
```

---

# 32. NodeFilter

```rust
struct NodeFilter {
    regions: Vec<String>,

    protocols: Vec<Protocol>,

    include_keywords: Vec<String>,

    exclude_keywords: Vec<String>,

    include_node_ids: Vec<NodeId>,

    exclude_node_ids: Vec<NodeId>,
}
```

---

# 33. SelectionPolicy

```rust
enum SelectionPolicy {
    Manual {
        selected_node_id: Option<NodeId>,
    },

    UrlTest {
        probe_url: String,

        interval: Duration,

        tolerance_ms: u32,
    },
}
```

Compiler：

```text
Manual
  ↓
selector

UrlTest
  ↓
urltest
```

---

# 34. 自动出口组

新增订阅：

```text
A机场
```

自动产生：

```text
A机场
订阅 · 28节点
自动选择
```

内部：

```text
Provider A
    ↓
Implicit Pool A
```

---

# 35. 组合出口组

例如：

```text
高速下载
```

来源：

```text
B机场
├── 香港
├── 日本
└── 新加坡

C机场
├── 香港
└── 日本
```

最终：

```text
B-HK
B-JP
B-SG
C-HK
C-JP
```

---

# 36. Pool Tag

禁止使用：

```text
出口组名称
```

作为运行时 ID。

例如：

```text
高速下载
```

重命名为：

```text
下载专线
```

不能导致规则失效。

因此内部：

```text
pool-<stable-id>
```

例如：

```text
pool-b3d2910f
```

---

# 37. RoutePolicy

用户层：

> 分流

Domain：

```rust
struct RoutePolicy {
    id: RoutePolicyId,

    name: String,

    enabled: bool,

    priority: i32,

    matcher: TrafficMatcher,

    target: RouteTarget,
}
```

---

# 38. TrafficMatcher

```rust
enum TrafficMatcher {
    Domain {
        domains: Vec<String>,
    },

    DomainSuffix {
        suffixes: Vec<String>,
    },

    RuleSet {
        rule_set_id: RuleSetId,
    },

    Application {
        applications: Vec<ApplicationIdentity>,
    },

    IpCidr {
        cidrs: Vec<String>,
    },

    Port {
        ports: Vec<PortRange>,
    },

    Protocol {
        protocols: Vec<NetworkProtocol>,
    },

    Inbound {
        inbound_id: InboundId,
    },
}
```

---

# 39. RouteTarget

```rust
enum RouteTarget {
    Pool(PoolId),

    Direct,

    Block,
}
```

V0.1 不支持普通规则直接引用：

```rust
Node(NodeId)
```

避免节点更新导致规则失效。

---

# 40. 路由优先级

推荐应用层固定大类：

```text
Explicit Domain
       ↓
Service / RuleSet
       ↓
Application
       ↓
IP / Network
       ↓
Default
```

同类型内部允许用户排序。

---

# 41. 应用分流

例如：

```text
Chrome
→ A机场

qBittorrent
→ 高速下载

rclone
→ 上传专线
```

Compiler：

```json
{
  "process_name": [
    "rclone.exe"
  ],
  "action": "route",
  "outbound": "pool-upload"
}
```

---

# 42. 网站与服务分流

客户端可以提供：

```text
YouTube
Netflix
Google
GitHub
OpenAI
Google Drive
OneDrive
Dropbox
```

服务定义不建议写成 UI 硬编码域名。

应使用：

```text
Service
    ↓
RuleSet
```

---

# 43. LocalInbound

为高级用户支持确定性出口。

例如：

```text
127.0.0.1:2080
→ A机场

127.0.0.1:2081
→ 高速下载

127.0.0.1:2082
→ 上传专线
```

Domain：

```rust
struct LocalInbound {
    id: InboundId,

    kind: InboundType,

    listen: IpAddr,

    port: u16,

    target_pool: Option<PoolId>,
}
```

---

# 44. 临时出口

托盘可以提供：

```text
临时出口

A机场
B机场
C机场
```

持续时间：

```text
15分钟
30分钟
1小时
直到手动恢复
```

Domain：

```rust
struct TemporaryRouteOverride {
    target: PoolId,

    scope: TemporaryScope,

    expires_at: Option<DateTime>,
}
```

---

# 45. TemporaryScope

```rust
enum TemporaryScope {
    Global,

    Application(ApplicationIdentity),
}
```

只影响：

```text
新连接
```

已有连接继续使用原出口。

---

# 46. RuntimeIntent

应用层不直接构造 sing-box JSON。

统一产生：

```rust
struct RuntimeIntent {
    nodes: Vec<ProxyNode>,

    pools: Vec<NodePool>,

    routes: Vec<RoutePolicy>,

    rule_sets: Vec<RuleSet>,

    dns: DnsPolicy,

    inbounds: Vec<LocalInbound>,

    tun: TunPolicy,

    default_target: RouteTarget,

    runtime_settings: RuntimeSettings,
}
```

---

# 47. SingBoxCompiler

接口：

```rust
trait ConfigCompiler {
    fn compile(
        &self,
        intent: &RuntimeIntent,
    ) -> Result<GeneratedConfig>;
}
```

当前：

```rust
struct SingBoxCompiler;
```

---

# 48. Compiler Pipeline

```text
Validate Domain
      ↓
Resolve Pool Membership
      ↓
Normalize Nodes
      ↓
Generate Stable Tags
      ↓
Compile Node Outbounds
      ↓
Compile Pool Outbounds
      ↓
Compile Direct / Block
      ↓
Compile Inbounds
      ↓
Compile Rules
      ↓
Compile Rule Sets
      ↓
Compile DNS
      ↓
Compile Clash API
      ↓
Serialize JSON
      ↓
sing-box check
```

---

# 49. Runtime Tag

节点：

```text
node-<stable-id>
```

出口组：

```text
pool-<stable-id>
```

系统：

```text
direct
block
```

名称只负责显示。

---

# 50. State Persistence

V0.1 使用：

```text
Versioned JSON State Store
```

主文件：

```text
state.json
```

---

# 51. state.json

示例：

```json
{
  "schema_version": 1,

  "subscriptions": [],

  "providers": [],

  "nodes": [],

  "pools": [],

  "route_policies": [],

  "rule_sets": [],

  "local_inbounds": [],

  "dns": {},

  "settings": {}
}
```

---

# 52. 为什么不使用 SQLite

核心配置的数据规模通常较小。

例如：

```text
5 个订阅
500 个节点
10 个出口组
30 条分流策略
```

甚至：

```text
5000 nodes
```

对内存模型仍然非常轻。

应用运行时本来也需要：

```text
完整 Nodes
完整 Pools
完整 Routes
完整 DNS
```

才能生成 sing-box Config。

因此 SQL Query、Join、Index 并不是核心需求。

---

# 53. StateStore

定义：

```rust
trait StateStore {
    fn load(&self) -> Result<AppState>;

    fn save(&self, state: &AppState) -> Result<()>;
}
```

实现：

```rust
struct JsonStateStore;
```

Application 不依赖具体文件格式。

未来更换实现不会影响 Domain。

---

# 54. State Load Pipeline

启动：

```text
Read state.json
       ↓
Parse JSON
       ↓
Read schema_version
       ↓
Run Migrations
       ↓
Deserialize Current State
       ↓
Validate References
       ↓
Build AppState
       ↓
Application Ready
```

---

# 55. Schema Version

必须有：

```json
{
  "schema_version": 1
}
```

禁止直接对旧 JSON 使用最新 struct 强行反序列化。

---

# 56. Migration

推荐：

```text
State V1
   ↓
migrate_v1_to_v2
   ↓
State V2
   ↓
migrate_v2_to_v3
   ↓
State V3
```

Migration 必须：

```text
Deterministic
Idempotent where possible
Tested
```

---

# 57. Storage Version Model

推荐保存层和 Domain 层分离。

例如：

```rust
StoredStateV1
StoredStateV2
StoredStateV3
```

然后：

```text
StoredState
    ↓
Migration
    ↓
CurrentStoredState
    ↓
Domain AppState
```

避免 Domain struct 背负大量历史兼容字段。

---

# 58. Reference Validation

加载 State 后检查：

```text
Provider
→ Subscription exists

Node
→ Provider exists

PoolSource
→ Provider exists

RoutePolicy
→ Pool exists

LocalInbound
→ Pool exists
```

发现引用损坏：

默认：

```text
拒绝覆盖原文件
+
尝试恢复 Backup
```

---

# 59. State Transaction

配置更新采用：

```text
Clone State
    ↓
Apply Mutation
    ↓
Validate
    ↓
Save Snapshot
    ↓
Swap In-Memory State
```

即：

> Persist Success 之后，新的配置才正式成为 Current State。

避免：

```text
内存修改成功
磁盘写入失败
```

产生两份 Source of Truth。

---

# 60. Atomic Write

禁止直接：

```text
truncate state.json
    ↓
write
```

推荐：

```text
serialize
   ↓
write state.tmp
   ↓
flush
   ↓
fsync
   ↓
rename current → backup
   ↓
atomic rename tmp → state.json
```

---

# 61. Backup

至少保留：

```text
state.json

state.backup.json
```

版本迁移前可以额外保留：

```text
state.pre-v2.backup.json
```

数量应有限制。

---

# 62. Corruption Recovery

如果：

```text
state.json
```

JSON 损坏：

```text
读取失败
   ↓
读取 backup
   ↓
校验
   ↓
恢复
```

损坏文件可以保存：

```text
state.corrupt.<timestamp>.json
```

用于诊断。

---

# 63. 保存时机

只在配置变化时持久化。

例如：

```text
添加订阅
删除订阅
更新订阅
修改出口组
修改分流
修改 DNS
修改设置
修改本地入口
```

---

# 64. 不写入 State 的数据

以下默认不进入 `state.json`：

```text
实时连接
当前速度
当前 CPU
当前内存
当前连接总数
临时日志
实时 Traffic
```

---

# 65. Node Runtime State

例如：

```text
延迟
最近测速时间
连续失败次数
```

第一阶段优先：

```text
Memory Only
```

如果需要跨重启保存，可以增加：

```text
runtime-cache.json
```

但不属于核心配置。

---

# 66. Runtime Cache

示例：

```json
{
  "node-xxxx": {
    "latency_ms": 43,
    "tested_at": 1788420100
  }
}
```

写入方式：

```text
Batch
+
Debounce
```

禁止每次测速立即同步写磁盘。

---

# 67. Future Telemetry Storage

未来如果实现：

```text
30天流量统计
连接历史
节点延迟历史
节点可靠性分析
每日使用量
```

可以单独引入：

```text
telemetry.db
```

使用 SQLite。

结构：

```text
Configuration
     ↓
state.json


Runtime History
     ↓
telemetry.db
```

两者职责分离。

---

# 68. 文件目录

推荐：

```text
data/

├── state.json
├── state.backup.json
│
├── native-profiles/
│
├── generated/
│   ├── active.json
│   └── previous.json
│
├── cache/
│   └── runtime-cache.json
│
├── rulesets/
│
└── logs/
```

---

# 69. Subscription Update

```text
Fetch Subscription
       ↓
Detect Format
       ↓
Parse
       ↓
Normalize
       ↓
Validate Nodes
       ↓
Create Candidate Node Set
       ↓
Create Candidate AppState
       ↓
Validate References
       ↓
Persist
       ↓
Rebuild RuntimeIntent
```

---

# 70. 订阅更新失败

如果：

```text
HTTP Error
Parse Error
Invalid Nodes
```

则：

```text
旧节点继续保留
```

禁止：

```text
刷新失败
   ↓
清空原订阅
```

---

# 71. Subscription HTTP

记录：

```text
ETag
Last-Modified
subscription-userinfo
Content-Disposition
```

支持：

```text
304 Not Modified
```

---

# 72. SubscriptionTraffic

```rust
struct SubscriptionTraffic {
    upload: Option<u64>,

    download: Option<u64>,

    total: Option<u64>,

    expire_at: Option<DateTime>,
}
```

UI：

```text
A机场

剩余 218 GB
2026-12-31 到期
```

---

# 73. Pool Refresh

订阅更新后：

```text
Node Set Changed
      ↓
Re-evaluate NodePool Filters
      ↓
New Pool Membership
      ↓
Rebuild RuntimeIntent
      ↓
Compile
```

Pool 保存的是：

```text
Selection Intent
```

而不是物化后的节点列表。

---

# 74. Pool Membership

例如：

```text
高速下载

Provider B
Filter: 香港,日本

Provider C
Filter: 香港
```

每次 RuntimeIntent 构建时重新计算。

因此订阅增加新：

```text
B-日本03
```

会自动进入该 Pool。

---

# 75. Config Validation

任何 Managed Config 应：

```text
Generate Candidate Config
       ↓
Write temporary file
       ↓
sing-box check
       ↓
Success
```

才能 Apply。

---

# 76. Config Build / Apply 分离

```text
Build
 │
 ├── Domain Validation
 ├── Compile
 ├── Serialize
 └── sing-box check


Apply
 │
 ├── Prepare
 ├── Replace Active Config
 ├── Restart
 └── Health Check
```

---

# 77. Runtime Config Files

```text
generated/

active.json
previous.json
candidate.json
```

candidate：

```text
只用于 check
```

成功后：

```text
active → previous
candidate → active
```

---

# 78. Rollback

如果新配置：

```text
check success
```

但启动后：

```text
sing-box crash
```

应：

```text
停止新实例
   ↓
恢复 previous.json
   ↓
启动旧配置
```

---

# 79. Runtime State Machine

```text
Stopped
   ↓
Starting
   ↓
Running
   ↓
Stopping
   ↓
Stopped
```

异常：

```text
Starting → Error
Running → Error
```

---

# 80. Ready 条件

不能简单以：

```text
spawn success
```

判定 Running。

至少：

```text
Process Alive
+
Clash API Ready
+
Mixed Port Ready
```

TUN：

```text
+
TUN Interface Ready
```

---

# 81. Process Manager

```rust
struct SingBoxProcess {
    child: Option<Child>,

    state: ProcessState,

    binary_path: PathBuf,

    config_path: PathBuf,

    pid: Option<u32>,
}
```

职责：

```text
start
stop
restart
kill
poll
check
```

---

# 82. Graceful Stop

Unix：

```text
SIGTERM
   ↓
Wait
   ↓
Timeout
   ↓
SIGKILL
```

Windows 使用平台对应的结束机制。

---

# 83. Orphan Recovery

App 启动时：

```text
读取自己的 runtime metadata
       ↓
验证旧 PID
       ↓
确认进程 identity
       ↓
必要时清理
```

禁止：

```text
kill all sing-box
```

---

# 84. Runtime Metadata

可以使用一个轻量：

```text
runtime.json
```

记录：

```json
{
  "pid": 12345,
  "started_at": "...",
  "binary_path": "...",
  "config_path": "..."
}
```

只用于进程恢复。

不属于 AppState。

---

# 85. Clash API

默认启用：

```text
127.0.0.1
```

客户端通过 Clash API 获取：

```text
proxy groups
selector state
connections
traffic
```

---

# 86. Clash API Security

默认：

```text
127.0.0.1
```

不监听：

```text
0.0.0.0
```

Secret：

```text
自动随机生成
```

用户无需默认感知。

---

# 87. Node Switch

Manual Pool：

```text
selector
```

节点切换：

```text
Clash API
```

无需重启 sing-box。

---

# 88. UrlTest Pool

自动模式：

```text
urltest
```

内核自动选节点。

应用负责：

```text
展示当前选中节点
展示延迟
```

---

# 89. CaptureMode

用户只看到：

```text
关闭
系统代理
TUN
```

Domain：

```rust
enum CaptureMode {
    Off,

    SystemProxy,

    Tun,
}
```

互斥。

---

# 90. 为什么互斥

避免：

```text
System Proxy = On
TUN = On
```

用户不知道实际行为。

高级用户如果将来需要特殊组合，再单独设计。

---

# 91. DNS

UI：

```text
DNS

自动

高级：
本地 DNS
代理 DNS
Fake IP
IPv6
```

Domain：

```rust
struct DnsPolicy {
    local_servers: Vec<DnsServer>,

    remote_servers: Vec<DnsServer>,

    fake_ip: bool,

    ipv6: bool,
}
```

---

# 92. DNS Domain Model

禁止：

```rust
raw_singbox_json: Value
```

作为 Managed DNS 主模型。

sing-box JSON 由 Compiler 产生。

---

# 93. RuleSet

```rust
struct RuleSet {
    id: RuleSetId,

    name: String,

    source: RuleSetSource,

    enabled: bool,
}
```

Source：

```text
Builtin
Local
Remote
```

---

# 94. Remote RuleSet

```rust
struct RemoteRuleSet {
    url: String,

    update_interval: Duration,

    last_update_at: Option<DateTime>,
}
```

文件缓存存：

```text
rulesets/
```

State 只保存元数据。

---

# 95. Connections

Connections 页面建议字段：

```text
Host
Destination
Protocol
Process
Rule
出口组
节点
Upload
Download
Duration
```

例如：

```text
youtube.com

高速下载
→ B机场 / 日本01
```

---

# 96. Runtime Domain Mapping

sing-box 返回：

```text
pool-b3d291
node-fa2291
```

UI 不展示这些 tag。

需要：

```text
Runtime Tag
    ↓
Domain ID Mapping
    ↓
Display Name
```

---

# 97. Connection Cache

避免：

```text
Frontend
每 500ms
GET 全量 Connections
```

Backend 维护：

```text
ConnectionCache
```

然后：

```text
added
updated
removed
```

增量发送给 UI。

---

# 98. Connection Revision

可以：

```rust
struct ConnectionDelta {
    revision: u64,

    added: Vec<Connection>,

    updated: Vec<Connection>,

    removed: Vec<ConnectionId>,
}
```

---

# 99. Traffic

Backend 持有：

```text
upload_speed
download_speed
upload_total
download_total
```

Frontend 消费 Snapshot/Event。

不直接管理内核轮询。

---

# 100. Tray 性能目标

明确非功能要求：

> 从托盘恢复窗口时无明显卡顿。

默认：

```text
Close Window
    ↓
Hide
```

而不是：

```text
Destroy WebView
```

---

# 101. UI Lifecycle

隐藏后保留：

```text
React Root
Router
Store
Navigation State
```

暂停：

```text
High Frequency Animation
Heavy Chart Rendering
Large Connection DOM Updates
```

---

# 102. Tray Restore

恢复：

```text
show
focus
```

禁止恢复时：

```text
重新读 state.json
重新解析订阅
重新扫描所有节点
重新启动 sing-box
重新初始化 Router
重新创建整个 React Root
```

---

# 103. Backend Always Alive

窗口隐藏时：

```text
Rust Backend
+
sing-box
```

继续运行。

UI 只是停止不必要刷新。

---

# 104. Visible / Hidden Sampling

建议：

```text
Visible:

Connections
500ms ~ 1s

Traffic
500ms


Hidden:

Connections
2s ~ 5s

Traffic
2s
```

具体可配置。

---

# 105. UI 性能指标

建议：

### Tray Restore

```text
P95 < 150ms
```

### 普通导航切换

```text
P95 < 100ms
```

### Node List

```text
1000 nodes
流畅滚动
```

### Connections

```text
5000 live connections
UI 不冻结
```

---

# 106. Virtual List

以下页面必须考虑虚拟列表：

```text
Proxy Nodes
Connections
Logs
```

不允许一次性渲染数千 DOM Row。

---

# 107. UI 信息架构

一级导航：

```text
概览

代理
订阅
分流

连接
日志

设置
```

避免：

```text
Inbound
Outbound
RuleSet
Provider
DNS Server
```

成为一级入口。

---

# 108. 代理页面

```text
代理

[出口组] [全部节点]
```

默认显示：

```text
A机场
订阅 · 28节点
当前 香港03 · 48ms

B机场
订阅 · 16节点
当前 新加坡02 · 63ms

自定义

高速下载
B机场 + C机场 · 21节点
当前 B机场 / 日本01 · 39ms
```

---

# 109. 新建出口组

Wizard：

```text
名称
 ↓
节点来源
 ↓
节点筛选
 ↓
节点选择策略
 ↓
预览
```

---

# 110. 节点来源

例如：

```text
☐ A机场
☑ B机场
☑ C机场
```

每个来源可以单独编辑：

```text
地区
协议
关键词
指定节点
```

---

# 111. 出口组详情

```text
高速下载

B机场 + C机场

21 个节点

选择方式
自动选择

当前
B机场 / 日本01
39ms
```

节点必须标识来源：

```text
B机场 · 香港01
C机场 · 香港01
```

---

# 112. 分流页面

```text
默认出口

A机场
```

应用：

```text
qBittorrent
→ 高速下载

rclone
→ 上传专线
```

服务：

```text
YouTube
→ 高速下载

Google Drive
→ 上传专线
```

---

# 113. 高级规则

折叠展示：

```text
域名
Domain Suffix
CIDR
端口
Inbound
Rule Set
```

普通用户无需接触。

---

# 114. Platform Layer

平台相关逻辑必须隔离。

```text
platform/
├── windows
├── macos
└── linux
```

---

# 115. Windows

关注：

```text
WinINET Proxy
Wintun
UAC
Process Identity
Auto Start
Privilege Helper
```

---

# 116. macOS

关注：

```text
System Proxy
utun
Launch at Login
Privilege Helper
Network Permission
```

---

# 117. Linux

关注：

```text
CAP_NET_ADMIN
TUN
GNOME Proxy
KDE Proxy
systemd user
```

Linux 第一版可以标记 Beta。

---

# 118. 权限原则

任何：

```text
Admin
Root
UAC
```

都必须由用户明确动作触发。

不允许应用后台静默获取高权限。

---

# 119. Error Model

统一：

```rust
enum AppError {
    Subscription,

    Parse,

    State,

    Migration,

    Config,

    Core,

    Network,

    Platform,

    Permission,
}
```

---

# 120. Error Payload

至少包括：

```text
code
user_message
technical_detail
recoverable
```

例如：

```text
CONFIG_NODE_UNSUPPORTED

节点“香港01”的当前协议参数无法转换。

technical_detail:
unsupported Reality transport combination
```

---

# 121. 日志分类

分：

```text
App Log
Core Log
```

App Log：

```text
subscription
storage
migration
compiler
runtime
platform
```

Core Log：

```text
sing-box stdout/stderr
```

---

# 122. Sensitive Data

日志禁止记录完整：

```text
Subscription Token
Password
UUID
Private Key
Reality Key
API Secret
```

---

# 123. State Sensitive Data

`state.json` 可能包含敏感信息。

需要：

```text
文件权限最小化
```

但 V0.1 不强制全数据库加密。

---

# 124. Secret Storage

未来可以将特别敏感字段转移到：

```text
Windows Credential Manager

macOS Keychain

Linux Secret Service
```

State 中只保存：

```text
secret_ref
```

---

# 125. SQLite 不代表安全

明确：

```text
SQLite plaintext
```

和：

```text
JSON plaintext
```

在 credential 保护方面没有本质区别。

安全问题应该通过：

```text
OS Credential Store
```

解决。

---

# 126. Generated Config Preview

高级用户允许：

```text
查看生成配置
复制
```

不允许：

```text
直接编辑 Managed Generated Config
```

否则产生：

```text
Domain State
        ↕
Generated JSON
```

双 Source of Truth。

---

# 127. Native Profile 负责 Raw JSON

需要完全控制 sing-box 的用户使用：

```text
Native Profile
```

而不是修改 Generated Config。

---

# 128. App Update 与 Core Update 分离

```text
App Version
```

与：

```text
sing-box Version
```

独立。

用户可以分别升级。

---

# 129. Core Version

需要：

```rust
struct CoreCompatibility {
    min_version: Version,

    max_tested_version: Option<Version>,
}
```

---

# 130. Core Rollback

至少保留：

```text
current core
previous core
```

升级失败可以回退。

---

# 131. SingBox Dialect

sing-box 配置语义会演化。

Compiler 应考虑：

```text
CoreVersion
```

未来：

```rust
trait SingBoxDialect {
    fn compile_dns(...);

    fn compile_tun(...);

    fn compile_route(...);
}
```

---

# 132. V0.1 Dialect

第一版只需要：

```text
CurrentSupportedDialect
```

但禁止在整个代码库散落：

```rust
if version >= ...
```

应该集中在 Compiler 层。

---

# 133. Frontend API

建议：

```text
subscription.list
subscription.add
subscription.update
subscription.delete
subscription.refresh

pool.list
pool.create
pool.update
pool.delete

routing.list
routing.update

proxy.select

runtime.start
runtime.stop
runtime.status

connection.snapshot

settings.get
settings.update
```

---

# 134. Backend 是业务 Source of Truth

Frontend Store 主要保存：

```text
selected page
filter
sort
drawer
dialog
layout
```

不承担持久化 Domain State。

---

# 135. Backend Event

Backend → Frontend：

```text
runtime-status
traffic-update
connection-delta
subscription-updated
state-updated
core-log
core-error
```

---

# 136. State Revision

建议 AppState 维护内存 Revision：

```text
revision: u64
```

每次成功配置更新：

```text
revision += 1
```

Frontend 可以：

```text
发现 revision 变化
   ↓
重新 Query 必要数据
```

---

# 137. Domain Test

不启动 sing-box。

测试：

```text
Node Identity
Provider Relationship
NodeFilter
Pool Membership
Route Priority
Reference Validation
```

---

# 138. Storage Test

必须覆盖：

```text
load state
save state
atomic replace
backup
corrupt recovery
migration
unsupported schema
```

---

# 139. Migration Test

每个版本准备 Fixture：

```text
fixtures/state/v1.json
fixtures/state/v2.json
```

测试：

```text
v1
 ↓
latest
```

结果符合预期。

---

# 140. Parser Test

```text
fixtures/subscriptions/

clash/
singbox/
uri/
base64/
```

每种协议覆盖：

```text
basic
tls
reality
ws
grpc
invalid
edge
```

---

# 141. Compiler Test

输入：

```text
RuntimeIntent
```

输出：

```text
JSON Snapshot
```

然后运行：

```text
sing-box check
```

---

# 142. Integration Test

```text
Subscription
    ↓
Parser
    ↓
Domain
    ↓
Pool
    ↓
Route
    ↓
RuntimeIntent
    ↓
Compiler
    ↓
sing-box check
```

---

# 143. Runtime Test

真实启动：

```text
sing-box
```

验证：

```text
start
ready
traffic
connections
selector
restart
stop
```

---

# 144. Multi-Provider Test

Fixture：

```text
Provider A
├── A1
└── A2

Provider B
└── B1

Provider C
└── C1
```

Pool：

```text
Default = A

Download = B + C

Upload = C
```

Route：

```text
video.example
→ Download

rclone
→ Upload

default
→ Default
```

---

# 145. UI E2E

覆盖：

```text
导入订阅
刷新订阅
创建出口组
设置默认出口
新增应用规则
启动代理
节点切换
开启 TUN
托盘隐藏
托盘恢复
退出程序
```

---

# 146. Performance Test

测试规模：

```text
100 subscriptions

5000 nodes

100 pools

500 route policies

5000 connections
```

验证：

```text
UI 不阻塞
Compiler 时间可接受
State 保存可接受
```

---

# 147. State Size

即使：

```text
5000 nodes
```

state.json 通常仍然属于：

```text
MB 级
```

整体加载完全可接受。

未来只有在出现明显：

```text
几十万记录
大量历史数据
复杂查询
```

时才考虑数据库。

---

# 148. Phase 1：Desktop Runtime

实现：

```text
Tauri Shell
React Shell
Rust Runtime
SingBox Process Manager
Config Validation
Mixed Inbound
System Proxy
Basic Tray
```

验收：

```text
Core 可以可靠启动
Core 可以可靠停止
无 orphan
```

---

# 149. Phase 2：State + Subscription

实现：

```text
AppState
JsonStateStore
Atomic Save
Backup
Migration

Clash YAML
sing-box JSON
URI
Base64

Subscription
Provider
ProxyNode
```

验收：

```text
3 个订阅同时存在
节点归属正确
App 重启后恢复
损坏 State 可恢复
```

---

# 150. Phase 3：出口组

实现：

```text
Implicit Pool

Custom Pool

Manual

UrlTest

NodeFilter
```

验收：

```text
可以组合 B+C
可以按地区过滤
可以自动选择
可以手动选择
```

---

# 151. Phase 4：Routing

实现：

```text
Default Pool

Domain
DomainSuffix
Application
RuleSet
CIDR
Inbound
```

验收：

```text
普通网页 → A

视频 → B

上传工具 → C
```

---

# 152. Phase 5：TUN + DNS

实现：

```text
CaptureMode

TUN

DNS

Fake IP

IPv6

LAN Bypass
```

重点测试：

```text
Windows
macOS
```

---

# 153. Phase 6：Observability

实现：

```text
Traffic
Connections
Logs
Pool Mapping
Node Mapping
Rule Mapping
Process Mapping
```

---

# 154. Phase 7：Desktop Polish

实现：

```text
Tray

Fast Restore

Auto Start

Updater

Core Update

Rollback

Crash Recovery

Window Lifecycle
```

---

# 155. V0.1 必须实现

```text
Tauri 2
React
Rust

sing-box sidecar

Versioned JSON State Store

Atomic State Persistence

State Backup / Recovery

State Migration

Clash YAML subscription

sing-box node import

URI import

Multiple subscriptions

Provider

Implicit Pool

Custom Pool

Manual selector

URLTest

Application Routing

Domain Routing

RuleSet Routing

System Proxy

TUN

DNS

Traffic

Connections

Logs

Tray

AutoStart

Windows

macOS
```

Linux：

```text
Beta
```

---

# 156. V0.1 不实现

```text
SQLite Configuration Store

Xray

Mihomo

Multi Core

Full Clash Config Conversion

Script

Automatic Traffic Classification

Dynamic Connection Migration

Traffic History Database

Cloud Sync

Account System

Mobile
```

---

# 157. 架构红线

以下实现不得进入主干。

## 红线 1

```text
UI
 ↓
直接修改 sing-box JSON
```

---

## 红线 2

Subscription Parser：

```text
Clash
 ↓
直接生成 SingBoxOutbound
```

---

## 红线 3

Domain 使用：

```rust
serde_json::Value
```

作为主要配置模型。

---

## 红线 4

Route 保存：

```text
outbound_tag
```

而不是：

```text
PoolId
```

---

## 红线 5

Pool 使用节点名称作为 Identity。

---

## 红线 6

Managed Config 允许用户直接编辑 Generated JSON。

---

## 红线 7

Window Restore 重新加载整个 State。

---

## 红线 8

Subscription Refresh Failure 清空旧节点。

---

## 红线 9

Config Check Failure 覆盖 Active Config。

---

## 红线 10

App Exit：

```text
kill all sing-box
```

---

## 红线 11

Runtime Connections 写入：

```text
state.json
```

---

## 红线 12

多个业务模块分别写：

```text
state.json
```

所有持久化必须通过统一：

```text
StateStore
```

---

# 158. 最终配置数据流

```text
                   state.json
                       │
                       ▼
                    AppState
                       │
        ┌──────────────┼──────────────┐
        │              │              │
Subscription A   Subscription B   Subscription C
        │              │              │
        ▼              ▼              ▼
   Provider A      Provider B      Provider C
        │              │              │
        ▼              ▼              ▼
      Nodes           Nodes          Nodes
        │              │              │
        └───────┬──────┴──────┬───────┘
                │             │
                ▼             ▼
           Provider Pools   Custom Pools
                │             │
                └──────┬──────┘
                       ▼
                  RoutePolicy
                       │
                       ▼
                  RuntimeIntent
                       │
                       ▼
                SingBoxCompiler
                       │
                       ▼
                  active.json
                       │
                       ▼
                    sing-box
```

---

# 159. 状态职责边界

```text
state.json
```

负责：

```text
用户配置
长期配置
领域关系
```

---

```text
runtime-cache.json
```

可选负责：

```text
最近延迟
非关键缓存
```

---

```text
runtime.json
```

负责：

```text
进程恢复信息
```

---

```text
generated/active.json
```

负责：

```text
运行配置
```

---

```text
telemetry.db
```

未来负责：

```text
长期历史统计
```

---

# 160. 核心长期资产

项目长期最重要的不是 UI。

也不是：

```text
sing-box process manager
```

而是：

```text
1. Subscription Normalization

2. Stable ProxyNode Domain

3. Provider / NodePool Model

4. RoutePolicy Model

5. RuntimeIntent

6. Semantic Config Compiler

7. Versioned Application State
```

---

# 161. 为什么这些才是核心

只要：

```text
Subscription
Provider
ProxyNode
Pool
Route
RuntimeIntent
```

保持稳定：

UI 可以重写。

sing-box 可以升级。

未来也可以出现：

```text
XrayCompiler
MihomoCompiler
```

而用户的：

```text
订阅
出口组
分流
```

无需推翻。

---

# 162. 最终架构定义

项目不定义为：

```text
Clash Verge with sing-box
```

也不定义为：

```text
sing-box Config Editor
```

而定义为：

> 一个拥有独立网络领域模型、以 sing-box 为第一运行后端、面向多订阅、多出口和策略分流场景的现代跨平台网络客户端。

产品层：

```text
订阅
 ↓
出口组
 ↓
分流
```

领域层：

```text
Subscription
 ↓
Provider
 ↓
ProxyNode
 ↓
NodePool
 ↓
RoutePolicy
 ↓
RuntimeIntent
```

基础设施层：

```text
Subscription Parser

StateStore

SingBoxCompiler

SingBoxRuntime

PlatformAdapter
```

最终运行：

```text
AppState
   ↓
RuntimeIntent
   ↓
SingBoxCompiler
   ↓
active.json
   ↓
sing-box
```

这是 V0.1 的核心架构基线。

后续开发中，所有新功能应优先判断：

> 它属于用户配置、领域模型、运行意图、编译器，还是 Runtime？

避免直接在 UI 或 sing-box JSON 上堆叠特殊逻辑。

只要这条边界保持稳定，项目就可以在功能持续增长的情况下仍然保持较低的架构复杂度。
