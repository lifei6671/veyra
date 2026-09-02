# 跨平台 sing-box 桌面客户端技术设计方案 V0.1

## 1. 文档目标

本文定义一个基于 Tauri 2 + React + Rust + sing-box 的跨平台桌面代理客户端技术方案。

项目核心目标不是简单实现“一个 sing-box GUI”，而是构建一套具有独立领域模型的桌面网络客户端，使以下能力在产品层保持简单、稳定：

- 多订阅管理
- 多机场节点隔离
- 自定义出口组
- 跨机场节点组合
- 按网站、服务、应用进行分流
- TUN 与系统代理
- 节点测速与自动选择
- 实时连接、流量、日志监控
- 原生 sing-box 配置高级模式
- Windows / macOS / Linux 跨平台运行
- 快速托盘恢复
- 后续具备增加其他网络内核的架构空间

产品交互层参考 Clash Verge Rev 的简洁信息架构。

订阅解析、标准节点模型、配置编译思路参考 Satelite Proxy 的设计方向。

sing-box 运行时、跨平台生命周期等实现参考成熟 sing-box 桌面客户端的工程经验。

项目不以任何现有项目作为运行时依赖，其核心 Domain、Application 和 sing-box Compiler 独立实现。


---

# 2. 核心设计原则

## 2.1 UI 不暴露 sing-box 配置模型

用户不应该为了使用客户端而理解：

- inbound
- outbound
- selector
- urltest
- detour
- rule_set
- route action
- clash_api

这些属于基础设施实现。

用户层只暴露：

```text
订阅
节点
出口组
分流
连接
日志
设置
```

例如：

用户看到：

```text
高速下载
B机场 + C机场
自动选择
```

内部实际可能生成：

```text
urltest outbound
    ↓
B-HK
B-JP
C-HK
C-SG
```

Domain 与 UI 术语允许不同。


---

## 2.2 Subscription 与 Runtime 解耦

订阅不是运行时配置。

正确关系：

```text
Subscription
     ↓
Parser
     ↓
ProxyNode
     ↓
Domain
     ↓
Config Compiler
     ↓
sing-box Config
```

禁止采用：

```text
Clash YAML
     ↓
字符串/字段替换
     ↓
sing-box JSON
```

所有订阅格式先转换成统一领域模型。


---

## 2.3 配置转换是语义转换，而非字段转换

例如：

```text
Clash proxy-group
```

并不意味着必须转换成某一个固定 sing-box 字段。

客户端首先理解用户意图：

```text
手动选节点
```

然后 Compiler 决定：

```text
selector
```

用户意图：

```text
自动选择最快节点
```

Compiler 决定：

```text
urltest
```

未来如果增加 Xray：

```text
自动选择最快节点
    ↓
Xray balancer + observatory
```

Domain 不发生变化。


---

## 2.4 第一版单内核，领域模型不绑定内核

V0.1 仅支持：

```text
sing-box
```

不增加：

```text
CoreKind
Xray
Mihomo
Multi Core
```

避免过早抽象。

但以下 Domain 类型不得使用 sing-box 专属结构：

```text
ProxyNode
Subscription
Provider
NodePool
RoutePolicy
DnsPolicy
```

sing-box 专用类型只能存在：

```text
infra/singbox/
```

目录下。


---

# 3. 产品定位

产品定位：

> 一个面向普通用户和高级用户的跨平台 sing-box 桌面网络客户端，提供订阅管理、出口组、应用分流、网站分流、TUN、系统代理与网络状态监控能力。

主要使用场景：

### 场景 A：普通用户

```text
添加订阅
    ↓
选择节点
    ↓
开启系统代理/TUN
```

### 场景 B：多机场用户

```text
机场 A
普通网页

机场 B
下载/流媒体

机场 C
上传/云盘
```

### 场景 C：高级用户

```text
自定义出口组
应用分流
网站分流
Rule Set
DNS
完整 sing-box JSON
```

---

# 4. 非目标

V0.1 不追求：

- 完整兼容 Clash YAML 所有字段
- 完整转换 Clash `proxy-groups`
- 完整转换 Clash `rule-providers`
- 完整继承 Clash DNS 配置
- Clash Script
- Mihomo Script
- JS Profile Transform
- Xray/mihomo 多内核
- 自动识别“大文件下载后动态换机场”
- 单连接中途迁移出口
- 复杂流量 QoS
- MITM
- 内容层协议解析
- 提供代理服务器或公共节点

尤其需要明确：

> 流量出口通常在连接建立阶段确定。

因此：

```text
连接已经上传 1GB
    ↓
检测到是大文件
    ↓
切换到 C机场
```

不属于设计目标。

---

# 5. 技术栈

## 5.1 Desktop

```text
Tauri 2
```

负责：

- Desktop Window
- System Tray
- Native Menu
- Auto Start
- IPC
- Updater
- OS integration
- filesystem
- privileges

---

## 5.2 Frontend

建议：

```text
React
TypeScript
Vite
```

状态层建议：

```text
Zustand
```

服务端状态/异步缓存：

```text
TanStack Query
```

路由：

```text
React Router
```

UI 可以采用：

```text
Radix UI / shadcn/ui
```

但视觉体系自行设计。

---

## 5.3 Backend

```text
Rust
Tokio
Serde
reqwest
```

职责：

```text
Domain
Subscription Parser
Storage
sing-box Config Compiler
Runtime
Process Management
System Proxy
TUN privilege
Traffic monitor
Connection monitor
Updater
```

---

## 5.4 Core

```text
sing-box
```

以独立 sidecar 二进制方式运行。

V0.1 不考虑将 sing-box 静态链接成 Rust library。


---

# 6. 总体架构

```text
┌────────────────────────────────────────────┐
│                    UI                      │
│                                            │
│ Overview                                   │
│ Proxies                                    │
│ Subscriptions                              │
│ Routing                                    │
│ Connections                                │
│ Logs                                       │
│ Settings                                   │
└─────────────────────┬──────────────────────┘
                      │
                   Tauri IPC
                      │
┌─────────────────────▼──────────────────────┐
│             Application Layer              │
│                                            │
│ SubscriptionService                        │
│ ProxyService                               │
│ PoolService                                │
│ RoutingService                             │
│ RuntimeService                             │
│ SettingsService                            │
└─────────────────────┬──────────────────────┘
                      │
┌─────────────────────▼──────────────────────┐
│                  Domain                    │
│                                            │
│ Subscription                               │
│ Provider                                   │
│ ProxyNode                                  │
│ NodePool                                   │
│ RoutePolicy                                │
│ DnsPolicy                                  │
│ RuntimeIntent                              │
└─────────────────────┬──────────────────────┘
                      │
┌─────────────────────▼──────────────────────┐
│              Infrastructure                │
│                                            │
│ subscription parser                        │
│ sing-box compiler                          │
│ sing-box runtime                           │
│ clash api client                           │
│ storage                                    │
│ platform adapter                           │
└─────────────────────┬──────────────────────┘
                      │
                   sing-box
                      │
        ┌─────────────┼─────────────┐
        │             │             │
      TUN         System Proxy    Mixed
```

---

# 7. 推荐目录结构

```text
src-tauri/src/

application/
    subscription_service.rs
    proxy_service.rs
    pool_service.rs
    route_service.rs
    runtime_service.rs
    settings_service.rs

domain/
    subscription.rs
    provider.rs
    node.rs
    pool.rs
    route.rs
    dns.rs
    runtime.rs

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
        inbound.rs
        selector.rs
        route.rs
        dns.rs
        ruleset.rs
    api/
        client.rs
        proxy.rs
        traffic.rs
        connection.rs
    runtime/
        process.rs
        config.rs
        health.rs
        log.rs

storage/
    mod.rs
    repository.rs
    migration.rs

platform/
    mod.rs
    windows/
    macos/
    linux/

commands/
    subscription.rs
    proxy.rs
    route.rs
    runtime.rs
    settings.rs
```

前端：

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
    node/
    pool/
    routing/
    runtime/

stores/
services/
components/
```

---

# 8. 核心领域模型

## 8.1 Subscription

表示：

> 节点来自哪里。

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

Source：

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

# 9. Provider

Provider 表示：

> 用户认知中的一个机场/节点来源。

正常情况下：

```text
1 Subscription
    ↓
1 Provider
```

但 Domain 不强制 1:1。

```rust
struct Provider {
    id: ProviderId,

    name: String,

    subscription_ids: Vec<SubscriptionId>,

    enabled: bool,
}
```

UI 一般不暴露 Provider 术语。

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

---

# 10. ProxyNode

所有订阅最终转换成：

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

## 10.1 Protocol

V0.1：

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

## 10.2 TLS

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

## 10.3 Transport

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

# 11. Node Identity

节点不能使用：

```text
name
```

作为唯一标识。

机场常出现：

```text
香港01
香港01
香港01
```

甚至 server/port 相同但密码不同。

建议：

```text
NodeId =
hash(
    provider_id
    protocol
    server
    port
    credentials identity
)
```

显示名称与节点身份分离。

---

# 12. 订阅解析系统

统一入口：

```rust
fn parse_subscription(
    input: &str,
) -> Result<ParseResult>
```

ParseResult：

```rust
struct ParseResult {
    format: SubscriptionFormat,

    nodes: Vec<ProxyNodeDraft>,

    skipped: Vec<SkippedNode>,
}
```

---

# 13. 格式检测顺序

推荐：

```text
JSON
 │
 ├─ sing-box JSON
 └─ Clash JSON

YAML
 │
 └─ Clash YAML

URI List

Base64
 │
 └─ decode
      ↓
    recursive detect
```

支持：

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

# 14. Clash Subscription 处理原则

V0.1：

只提取：

```yaml
proxies:
```

不承诺继承：

```yaml
proxy-groups:
rules:
rule-providers:
dns:
tun:
hosts:
script:
```

因此：

```text
Clash Subscription
       ↓
Proxy Node Parser
       ↓
ProxyNode
```

客户端自己的：

```text
出口组
分流
DNS
TUN
```

由 Domain 控制。

---

# 15. Native sing-box Profile

必须提供高级模式。

类型：

```text
Native sing-box Profile
```

用户提供完整：

```json
{
  "inbounds": [],
  "outbounds": [],
  "route": {}
}
```

客户端：

```text
validate
   ↓
write
   ↓
sing-box run
```

不经过：

```text
ProxyNode
NodePool
RoutePolicy
```

---

## 15.1 Native Mode 限制

Native Profile 启用时：

客户端只提供：

```text
启动/停止
日志
Traffic
Connections
Clash API
TUN 状态
```

不能保证提供：

```text
出口组编辑
分流编辑
节点池编辑
```

因为配置已经由用户完全控制。

---

# 16. 出口组 NodePool

这是本项目最核心的产品能力之一。

用户层叫：

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

# 17. Pool 类型

```rust
enum PoolKind {
    ImplicitProvider,
    Custom,
}
```

### ImplicitProvider

订阅自动生成。

例如：

```text
Subscription A
    ↓
Provider A
    ↓
Pool A
```

用户不需要手动创建。

### Custom

用户主动创建：

```text
高速下载
大文件上传
AI服务
低延迟
```

---

# 18. PoolSource

```rust
struct PoolSource {
    provider_id: ProviderId,

    filter: NodeFilter,
}
```

不能简单定义：

```rust
providers: Vec<ProviderId>
```

否则无法表达：

```text
B机场：
只用 香港 + 日本

C机场：
只用 美国
```

---

# 19. NodeFilter

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

# 20. SelectionPolicy

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

对应：

```text
Manual
   ↓
sing-box selector

UrlTest
   ↓
sing-box urltest
```

---

# 21. 自动 Provider Pool

用户添加：

```text
A机场
```

后台自动生成：

```text
pool/provider-a
```

节点：

```text
A-HK
A-JP
A-US
```

编译：

```json
{
  "type": "urltest",
  "tag": "pool-provider-a",
  "outbounds": [
    "node-a-hk",
    "node-a-jp",
    "node-a-us"
  ]
}
```

---

# 22. 组合出口组

例如：

```text
高速下载
```

来源：

```text
B机场
香港、日本、新加坡

C机场
香港、日本
```

得到：

```text
B-HK
B-JP
B-SG
C-HK
C-JP
```

编译：

```json
{
  "type": "urltest",
  "tag": "pool-download",
  "outbounds": [...]
}
```

---

# 23. Pool Tag

不能使用名称直接作为 sing-box tag。

建议：

```text
pool-<stable-id>
```

例如：

```text
pool-b3d2910f
```

UI name：

```text
高速下载
```

这样重命名：

```text
高速下载
    ↓
下载专线
```

不会导致 route 关系失效。

---

# 24. RoutePolicy

用户层叫：

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

# 25. TrafficMatcher

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

# 26. RouteTarget

```rust
enum RouteTarget {
    Pool(PoolId),

    Direct,

    Block,
}
```

V0.1 不允许：

```rust
Node(NodeId)
```

作为普通分流规则目标。

原因：

节点可能在订阅刷新后消失。

Pool 是稳定抽象。

---

# 27. 分流优先级

推荐固定分类优先级：

```text
显式域名
    ↓
Service / RuleSet
    ↓
Application
    ↓
IP / Network
    ↓
Default
```

用户可以在同类别中排序。

避免普通用户需要理解所有 sing-box rule precedence。

---

# 28. 应用分流

例：

```text
Chrome
    → A机场

qBittorrent
    → 高速下载

rclone
    → 大文件上传
```

编译：

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

# 29. 网站服务分流

客户端可以预置逻辑服务：

```text
YouTube
Netflix
Google
GitHub
OpenAI
OneDrive
Dropbox
Google Drive
```

服务不是硬编码域名数组。

Domain：

```rust
struct ServiceRule {
    id: ServiceId,

    name: String,

    rule_sets: Vec<RuleSetId>,
}
```

底层使用 Rule Set。

---

# 30. 专用入口分流

支持额外本地入口：

```text
127.0.0.1:2080 → 默认出口
127.0.0.1:2081 → 下载池
127.0.0.1:2082 → 上传池
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

sing-box：

```text
inbound tag
    ↓
route rule
    ↓
pool
```

这是处理复杂场景的确定性方案。

---

# 31. 临时出口

产品支持：

```text
临时出口
```

例如：

```text
C机场

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

Scope：

```rust
enum TemporaryScope {
    Global,

    Application(ApplicationIdentity),
}
```

只影响：

> 新建立连接。

已有连接不迁移。

---

# 32. RuntimeIntent

Application 不直接构造 sing-box JSON。

先生成：

```rust
struct RuntimeIntent {
    nodes: Vec<ProxyNode>,

    pools: Vec<NodePool>,

    routes: Vec<RoutePolicy>,

    dns: DnsPolicy,

    inbounds: Vec<LocalInbound>,

    tun: TunPolicy,

    default_target: RouteTarget,

    runtime_settings: RuntimeSettings,
}
```

Compiler：

```text
RuntimeIntent
      ↓
SingBoxCompiler
      ↓
GeneratedConfig
```

---

# 33. SingBoxCompiler

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

# 34. Compiler Pipeline

推荐：

```text
validate domain
      ↓
normalize nodes
      ↓
generate node tags
      ↓
compile node outbounds
      ↓
compile pools
      ↓
compile direct/block
      ↓
compile inbounds
      ↓
compile route rules
      ↓
compile rule sets
      ↓
compile DNS
      ↓
compile clash_api
      ↓
serialize JSON
      ↓
sing-box check
```

---

# 35. Outbound Tag

节点：

```text
node-<id>
```

Pool：

```text
pool-<id>
```

系统：

```text
direct
block
dns
```

禁止使用：

```text
节点名称
机场名称
```

作为配置 identity。

---

# 36. Runtime API

启用：

```json
{
  "experimental": {
    "clash_api": {
      "external_controller": "127.0.0.1:9090"
    }
  }
}
```

应用通过 Clash API 获取：

```text
proxies
connections
traffic
selector state
```

sing-box 当前支持 Clash API，因此可以承担大量运行时 UI 状态交互。 

---

# 37. Runtime 设计

```rust
struct Runtime {
    process: SingBoxProcess,

    config: Option<RuntimeConfig>,

    api: ClashApiClient,

    status: RuntimeStatus,

    connection_cache: ConnectionCache,

    traffic_cache: TrafficCache,
}
```

---

# 38. Runtime 状态机

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

禁止只靠：

```text
Child::spawn()
```

成功判断启动完成。

Ready 条件至少包括：

```text
进程存活
+
Clash API ready
+
mixed port ready
```

TUN 可增加：

```text
TUN adapter ready
```

---

# 39. Config Validation

任何 Generated Config 启动前：

```text
写临时文件
    ↓
sing-box check -c
    ↓
success
    ↓
atomic replace
    ↓
restart
```

失败：

```text
保留当前运行配置
```

不能：

```text
生成失败
    ↓
把旧内核停掉
    ↓
用户断网
```

---

# 40. 配置事务

核心规则：

> Build 与 Apply 分离。

```text
Build
 │
 ├ validate
 ├ compile
 ├ check
 └ prepare

Apply
 │
 ├ stop old
 ├ atomic config replace
 ├ start new
 └ health check
```

如果启动失败：

优先支持：

```text
rollback previous config
```

---

# 41. 节点切换

Manual Pool：

```text
selector
```

节点切换通过 Clash API：

```text
PUT selector
```

不需要重启 sing-box。

Auto Pool：

```text
urltest
```

由内核选择。

---

# 42. Pool 更新

如果订阅刷新导致：

```text
Node A 删除
Node B 新增
```

Pool membership 发生变化。

处理流程：

```text
subscription refresh
      ↓
node store update
      ↓
rebuild RuntimeIntent
      ↓
compile config
      ↓
check
      ↓
restart
```

V0.1 不追求：

```text
动态增删 outbound 无重启
```

节点切换才走热更新。

---

# 43. Subscription 更新

流程：

```text
fetch
 ↓
parse
 ↓
normalize
 ↓
validate
 ↓
build new node set
 ↓
transaction update storage
 ↓
recompute pools
 ↓
recompile config
```

如果新订阅解析失败：

```text
保留旧节点
```

不能直接清空。

---

# 44. Subscription HTTP

客户端请求订阅时记录：

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

避免无意义重解析。

---

# 45. Subscription Traffic

Domain：

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

剩余 218GB
12月31日到期
```

---

# 46. Storage

V0.1 推荐：

```text
SQLite
```

而不是 JSON 文件堆。

原因：

数据已经具有明显关系：

```text
Subscription
Provider
Node
Pool
PoolSource
RoutePolicy
Settings
```

需要：

```text
transaction
migration
query
foreign key
```

---

# 47. 数据表建议

```text
subscriptions
providers
nodes

node_pools
pool_sources

route_policies

rule_sets

local_inbounds

settings

runtime_snapshots
```

---

# 48. nodes

节点表不保存 latency 作为核心配置字段。

建议：

```text
nodes
node_runtime_stats
```

分离：

```text
identity/config
```

和：

```text
latency/history
```

---

# 49. UI 信息架构

左侧：

```text
概览

代理
订阅
分流

连接
日志

设置
```

这是 V0.1 默认结构。

避免：

```text
Inbound
Outbound
Rule Set
DNS Server
Provider
```

成为一级导航。

---

# 50. 代理页面

默认：

```text
代理

[出口组] [全部节点]
```

出口组：

```text
A机场
订阅 · 28节点
当前 香港03 · 48ms

B机场
订阅 · 16节点
当前 新加坡02 · 61ms

自定义

高速下载
B机场 + C机场 · 21节点
当前 B机场 / 日本01 · 39ms
```

---

# 51. 创建出口组

Wizard：

```text
名称
 ↓
来源
 ↓
筛选
 ↓
选择策略
 ↓
预览
```

来源：

```text
B机场
C机场
```

筛选：

```text
全部

地区

协议

关键词

指定节点
```

策略：

```text
自动最快
手动选择
```

---

# 52. 出口组详情

```text
高速下载

B机场 + C机场
21 个节点

选择方式：自动

当前：
B机场 / 日本01
39ms


B机场

香港01   43ms
日本01   39ms


C机场

香港01   52ms
美国01   128ms
```

必须显示 Provider 来源。

---

# 53. 分流页面

```text
默认出口

A机场
```

应用：

```text
qBittorrent
→ 高速下载

rclone
→ 大文件上传
```

服务：

```text
YouTube
→ 高速下载

Google Drive
→ 大文件上传
```

高级：

```text
域名
CIDR
端口
Inbound
Rule Set
```

---

# 54. Connections

字段：

```text
Host
Destination
Protocol
Process
Rule
Pool
Node
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

不要只显示：

```text
node-b3858a
```

Runtime tag 必须反向映射 Domain。

---

# 55. Connection Cache

不要让前端每 500ms 获取全部 connections。

推荐：

Backend：

```text
poll core
 ↓
connection cache
 ↓
diff
```

Frontend：

```text
revision based delta
```

至少：

```text
added
updated
removed
```

避免 TUN 模式几千连接时频繁发送完整 JSON。

---

# 56. Traffic

Backend 维护：

```text
upload_total
download_total
upload_speed
download_speed
```

前端只消费 snapshot。

不要让每一个 UI 组件自己轮询 Clash API。

---

# 57. Tray 性能设计

这是明确的非功能需求。

目标：

> 从托盘恢复主窗口时，无明显重新初始化卡顿。

原则：

```text
Hide != Destroy
```

默认：

```text
关闭窗口
    ↓
hide
```

而不是：

```text
destroy webview
```

---

# 58. UI 状态保活

隐藏窗口以后：

保留：

```text
React Root
Router
Store
Navigation State
```

但停止：

```text
高频动画
不可见图表更新
DOM-heavy connection render
```

Backend Runtime 始终运行。

---

# 59. 恢复窗口

托盘：

```text
show()
focus()
```

不能在恢复路径执行：

```text
读取全部订阅
刷新订阅
扫描节点
重新解析规则
重建 config
重新启动 WebSocket
查询全部 connections
```

恢复只能：

```text
显示现有 View
+
读取已有 snapshot
```

---

# 60. 性能指标

建议直接作为验收条件：

### Window Restore

目标：

```text
P95 < 150ms
```

从用户点击托盘到窗口可交互。

### Navigation

普通页面：

```text
P95 < 100ms
```

### Proxy Node List

1000 nodes：

```text
无明显卡顿
```

必须虚拟列表。

### Connections

5000 live connections：

UI 不允许冻结。

---

# 61. 后台刷新策略

窗口隐藏：

降低：

```text
UI render frequency
```

但 Backend 可以持续：

```text
Traffic
Connections
Health
```

根据资源压力调整：

```text
Visible:
connections 500ms

Hidden:
connections 2s
```

具体值配置化。

---

# 62. TUN

支持：

```text
Off
System Proxy
TUN
```

用户心智：

```text
代理模式
```

而不是同时暴露：

```text
system proxy toggle
tun toggle
```

避免出现两者同时开启且用户不知道谁生效。

---

# 63. CaptureMode

```rust
enum CaptureMode {
    Off,
    SystemProxy,
    Tun,
}
```

状态必须互斥。

---

# 64. DNS

V0.1 UI 尽量简单：

```text
DNS

默认
自动

高级
国内 DNS
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

不要保存 sing-box raw JSON。

---

# 65. Rule Set

Domain：

```rust
struct RuleSet {
    id: RuleSetId,

    name: String,

    source: RuleSetSource,

    enabled: bool,
}
```

支持：

```text
Local
Remote
Builtin
```

Remote：

```rust
url
update_interval
last_update
```

sing-box 原生支持 remote rule-set 与更新间隔。 

---

# 66. Platform Layer

必须隔离：

```text
Windows
macOS
Linux
```

以下实现不能散落在 Application：

```text
system proxy
autostart
privilege
process detection
tun permission
path
service
```

---

# 67. Windows

主要考虑：

```text
WinINET/System Proxy
UAC
Wintun
process ownership
startup
service/helper
```

TUN 启动可能需要权限提升。

---

# 68. macOS

主要考虑：

```text
Network permissions
root/helper
setuid/helper strategy
system proxy
launch at login
utun
```

权限处理必须做到：

```text
用户明确触发
```

不要后台偷偷要求 root。

---

# 69. Linux

考虑：

```text
CAP_NET_ADMIN
system proxy desktop environment
systemd user
TUN
```

第一阶段优先：

```text
GNOME/KDE
```

---

# 70. Process Manager

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
validate
```

---

# 71. Graceful Stop

停止：

```text
SIGTERM
 ↓
等待
 ↓
timeout
 ↓
kill
```

Windows 使用对应 process terminate。

避免 orphan process。

---

# 72. Crash Recovery

App 启动：

```text
检查旧 PID
 ↓
确认是否属于本 App 启动的 sing-box
 ↓
必要时清理
```

不能简单：

```text
kill all sing-box
```

因为用户可能自己运行其他 sing-box 实例。

---

# 73. Port Ownership

记录：

```text
mixed port
clash api port
extra inbound ports
```

Restart 前等待释放。

避免：

```text
address already in use
```

---

# 74. Error Model

统一：

```rust
enum AppError {
    Subscription,
    Parse,
    Config,
    Core,
    Network,
    Platform,
    Storage,
    Permission,
}
```

错误必须包含：

```text
code
user_message
technical_detail
recoverable
```

例如：

```text
CONFIG_NODE_UNSUPPORTED

节点“香港01”的协议参数无法转换为当前 sing-box 配置。

technical:
VLESS Reality + xxx unsupported
```

---

# 75. 日志

分：

```text
App Log

Core Log
```

不要混。

App：

```text
subscription
config
runtime
platform
storage
```

Core：

```text
sing-box stdout/stderr
```

---

# 76. 日志敏感信息

必须脱敏：

```text
subscription URL token
password
uuid
private key
api secret
```

默认日志不能完整打印 ProxyNode。

---

# 77. 配置预览

高级设置提供：

```text
查看生成配置
```

但只读。

允许：

```text
复制
```

不允许直接编辑 Managed Profile 的 Generated Config。

否则：

```text
Domain
```

和：

```text
Runtime config
```

会出现双写。

需要修改 raw JSON 的用户使用：

```text
Native Profile
```

---

# 78. Security

Clash API 默认：

```text
127.0.0.1
```

禁止默认：

```text
0.0.0.0
```

Secret：

可以默认生成随机值。

UI 无需默认展示。

---

# 79. Subscription URL

Subscription URL 经常包含 credential。

Storage：

可明文保存于本地 DB，但：

```text
UI 列表脱敏
日志脱敏
Crash Report 脱敏
```

后续可以考虑 OS keychain。

---

# 80. Updater

App 与 Core 分离更新。

```text
App Update
Core Update
```

Core 版本必须可：

```text
升级
回退
```

不能把：

```text
最新 sing-box
```

和 App 强绑定。

---

# 81. Core Compatibility

需要：

```rust
struct CoreCompatibility {
    min_version: Version,
    max_tested_version: Option<Version>,
}
```

如果用户自行安装超出测试范围版本：

UI：

```text
该版本尚未经过完整兼容性测试
```

---

# 82. 配置版本兼容

sing-box 配置持续变化。

因此 Compiler 必须：

```text
根据 CoreVersion
```

生成配置。

例如：

```rust
trait SingBoxDialect {
    fn compile_dns(...);
    fn compile_tun(...);
}
```

V0.1 可以先实现：

```text
CurrentDialect
```

但必须预留版本层。

---

# 83. Application API

Frontend 不应该直接调用几十个底层命令。

建议：

```text
subscription.list
subscription.add
subscription.update
subscription.refresh

pool.list
pool.create
pool.update
pool.delete

routing.list
routing.save

runtime.start
runtime.stop
runtime.status

proxy.select_node

connection.snapshot
connection.subscribe

settings.get
settings.update
```

---

# 84. Frontend Store

不应该缓存整个 Backend Domain。

建议只存：

```text
UI state
```

例如：

```text
selected page
filter
sort
drawer
modal
```

业务真实状态：

```text
Backend
```

通过 Query/Snapshot 获取。

避免 Rust 和 React 同时成为 source of truth。

---

# 85. Events

Backend → Frontend：

```text
runtime-status
traffic-update
connection-delta
subscription-updated
core-log
core-error
```

不能把所有数据都通过 polling。

---

# 86. Testing Strategy

分四层。

### Domain Test

不启动 sing-box。

测试：

```text
NodeFilter
Pool membership
Route priority
identity
```

---

# 87. Parser Test

建立 fixtures：

```text
fixtures/

clash/
singbox/
uri/
```

每种协议至少：

```text
basic
tls
reality
ws
grpc
invalid
edge case
```

---

# 88. Compiler Test

输入：

```text
RuntimeIntent
```

输出：

```text
JSON
```

进行：

```text
snapshot test
```

同时必须调用：

```text
sing-box check
```

做真实语法验证。

---

# 89. Integration Test

测试链路：

```text
Subscription
 ↓
Parser
 ↓
Domain
 ↓
Compiler
 ↓
sing-box check
```

覆盖：

```text
A机场
B机场
自定义 Pool
RoutePolicy
TUN
DNS
```

---

# 90. Runtime Test

启动真实 sing-box：

```text
mixed inbound
 ↓
HTTP request
 ↓
mock/local upstream
```

验证：

```text
start
ready
traffic
connection
selector
restart
stop
```

---

# 91. Multi-Provider Test

这是核心验收。

Fixture：

```text
Provider A
Node A1
Node A2

Provider B
Node B1

Provider C
Node C1
```

Pool：

```text
Default = A
Download = B + C
Upload = C
```

规则：

```text
example-video.test
→ Download

rclone
→ Upload

default
→ Default
```

验证 Compiler 的 routing 与 selector/urltest 关系。

---

# 92. UI E2E

至少覆盖：

```text
导入订阅
新建出口组
设置默认出口
新增应用分流
启动代理
切换节点
从托盘隐藏
从托盘恢复
```

---

# 93. Performance Test

必须成为自动/半自动测试项：

```text
100 subscriptions

5000 nodes

5000 connections

100 pools

500 route policies
```

目标：

UI 不阻塞。

---

# 94. Phase 1：最小可运行

目标：

```text
sing-box 启停
```

实现：

```text
Tauri shell
Rust runtime
Core Manager
config validate
mixed inbound
system proxy
basic tray
```

验收：

```text
可以启动 sing-box
可以停止
不会留下 orphan
```

---

# 95. Phase 2：Subscription + Node

实现：

```text
Clash YAML
sing-box JSON
URI
Base64

Subscription
Provider
ProxyNode
SQLite
```

验收：

```text
3 个机场同时导入
节点归属正确
订阅刷新不破坏旧数据
```

---

# 96. Phase 3：出口组

实现：

```text
Implicit Provider Pool
Custom Pool
Manual
UrlTest
```

UI：

```text
代理
出口组
全部节点
```

验收：

```text
可以建立 B+C 组合池
可以按地区筛节点
```

---

# 97. Phase 4：Routing

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
下载 → B
上传工具 → C
```

---

# 98. Phase 5：TUN + DNS

实现：

```text
CaptureMode
TUN
DNS
Fake IP
LAN bypass
IPv6
```

重点进行：

```text
Windows
macOS
```

稳定性测试。

---

# 99. Phase 6：Observability

实现：

```text
Traffic
Connections
Logs
Process
Rule
Pool
Node
```

Connection UI 完成 Domain 映射。

---

# 100. Phase 7：Desktop Polish

实现：

```text
Tray
Autostart
Updater
Window lifecycle
Fast restore
Crash recovery
Core rollback
```

完成性能指标。

---

# 101. V0.1 功能范围

必须：

```text
Tauri 2
React
Rust

sing-box sidecar

Clash YAML subscription
sing-box JSON node import
URI import

Multiple subscriptions

Provider

Implicit Pool
Custom Pool

Manual selector
URLTest

Application routing
Domain routing
RuleSet routing

System Proxy
TUN

DNS

Traffic
Connections
Logs

Tray
Autostart

Windows
macOS
```

Linux 可以作为 Beta。

---

# 102. V0.1 不做

```text
Xray
Mihomo

Multi Core

Script

完整 Clash config migration

复杂 chain proxy

自动大流量识别

流量动态迁移

手机端

云同步

账号系统
```

---

# 103. 架构红线

以下实现禁止进入主干。

### 禁止 1

```text
UI → sing-box JSON
```

---

### 禁止 2

Domain struct 出现：

```text
serde_json::Value
```

作为主要配置模型。

---

### 禁止 3

Clash Parser 直接生成 sing-box Outbound。

---

### 禁止 4

RoutePolicy 直接保存：

```text
outbound_tag
```

---

### 禁止 5

Pool 使用节点名称作为 identity。

---

### 禁止 6

Managed Profile 允许直接编辑 Generated JSON。

---

### 禁止 7

Window Restore 时重新初始化整个应用。

---

### 禁止 8

订阅刷新失败时清空旧节点。

---

### 禁止 9

Config Check 失败后覆盖运行配置。

---

### 禁止 10

App 退出时：

```text
kill all sing-box
```

---

# 104. 最终核心数据流

```text
                    Subscription A
                          │
                    Subscription B
                          │
                    Subscription C
                          │
                          ▼
                 Subscription Parser
                          │
                          ▼
                     ProxyNode
                          │
              ┌───────────┼───────────┐
              │           │           │
              ▼           ▼           ▼
         Provider A  Provider B  Provider C
              │           │           │
              ▼           ▼           ▼
          Pool A       Pool B       Pool C
                           \         /
                            \       /
                             ▼     ▼
                         Download Pool
                              │
                              │
                         RoutePolicy
                              │
        ┌─────────────────────┼──────────────────┐
        │                     │                  │
     Browser              Download            Upload
        │                     │                  │
        ▼                     ▼                  ▼
     Pool A              Download Pool          Pool C
        │                     │                  │
        └─────────────────────┼──────────────────┘
                              │
                              ▼
                       RuntimeIntent
                              │
                              ▼
                      SingBoxCompiler
                              │
                              ▼
                         config.json
                              │
                              ▼
                           sing-box
```

---

# 105. 项目的核心资产

这个项目真正需要建立的长期资产不是 UI，也不是 sing-box 启动逻辑，而是以下四层：

```text
1. Subscription Normalization

2. Network Domain Model

3. NodePool / RoutePolicy

4. SingBox Semantic Compiler
```

只要这四层稳定：

UI 可以迭代。

sing-box 可以升级。

未来可以支持：

```text
XrayBackend
MihomoBackend
```

而不影响用户的：

```text
订阅
出口组
分流
```

配置。

---

# 106. 最终架构判断

项目不应定位为：

```text
Clash Verge + sing-box
```

更准确的定位应该是：

> 一个拥有独立网络领域模型、以 sing-box 为第一运行后端、采用简洁桌面交互设计的跨平台代理客户端。

Clash Verge 提供的是产品交互参考。

Satelite Proxy 提供的是订阅归一化和配置编译架构参考。

sing-box 提供实际网络运行能力。

真正属于本项目自己的核心，是：

```text
Subscription
    ↓
Provider
    ↓
NodePool
    ↓
RoutePolicy
    ↓
RuntimeIntent
    ↓
Compiler
```

这一条链。

如果未来要形成长期可维护的独立开源项目，应优先保证这条链的模型稳定性，而不是优先追求兼容所有现有代理客户端配置格式。