# Veyra Settings Contract V0.1

> 状态：产品契约已冻结，可进入技术拆分与实现
> 目标平台：Windows V0.1
> 目的：定义 Veyra Settings 的产品语义、默认值、状态所有权、持久化边界、运行时副作用与页面信息架构。
> 视觉实现继续遵守 `docs/ui/veyra-ui-spec.md`。

---

# 1. 核心边界

## 1.1 Settings Page 是多个领域的聚合入口

Settings 页面不等于 `AppSettings`。

```text
SettingsPage
├── AppSettings              应用级长期偏好
├── RuntimeSupervisor        CaptureMode 与运行态切换
├── DnsPolicy                DNS / IPv6 相关策略
├── LocalInbound             Mixed / SOCKS / HTTP 等本地入口
├── RuleSet                  规则集与更新策略
├── Tauri Desktop AutoStart  AutoStart API / plugin
├── SingBox Runtime          内核版本、日志、生成配置
└── App Update              Veyra 更新检查
```

禁止为了让 Settings 页面拥有足够的内容，把所有字段塞进一个巨大的 `AppSettings`。

## 1.2 配置态与运行态分离

以下属于配置态，可按各自 Owner 持久化：

```text
ThemePreference
AutoStart
SilentStart
DnsPolicy
IPv6 Policy
LogLevel
LocalInbound
RuleSet metadata
```

以下属于运行态或观测态，不作为 Settings 主体配置：

```text
当前连接数
当前 CPU / 内存
初始化状态
当前 observation 来源
当前运行 revision
当前 CaptureMode 的未确认候选状态
```

## 1.3 Generated Config 只读

`generated/active.json` 允许：

```text
查看
复制
```

不允许直接编辑。

需要完整控制 sing-box JSON 的用户使用 Native Profile。

---

# 2. V0.1 Settings 信息架构

V0.1 使用双栏、扫描式 Settings 页面。

```text
设置

LEFT
├── 系统与启动
└── 网络与内核

RIGHT
├── Veyra
└── 维护与诊断
```

## 2.1 系统与启动

```text
捕获模式                 [关闭 / 系统代理 / TUN]
开机自启                                  [Switch]
静默启动                                  [Switch]
```

语义：

- **开机自启**：Veyra 随 Windows 用户登录/系统启动流程启动。
- **静默启动**：Veyra 启动后不自动展示主 UI 窗口，后台与托盘继续运行。
- 两者是独立设置，不互相替代。

## 2.2 网络与内核

```text
DNS                                         >
IPv6                                    [Switch]
日志级别                                [Select]
本地入口                                    >
规则集                                      >
当前运行配置                                >
sing-box 内核                         [Version]
```

本地入口默认端口：

```text
Mixed    4040
SOCKS    4041
HTTP     4042
```

V0.1 默认启用组合：

```text
Mixed    4040  ON
SOCKS    4041  OFF
HTTP     4042  OFF
```

该组合属于 `LocalInbound` 产品契约，不得由 UI Task 改写或自行推断。

## 2.3 Veyra

```text
主题模式                       [系统 / 浅色 / 深色]
```

V0.1 正式支持三种主题偏好。

暂不把以下偏好加入 V0.1：

```text
语言
启动页面
托盘点击行为
关闭窗口行为
热键
```

## 2.4 维护与诊断

```text
配置目录                                    >
日志目录                                    >
导出诊断信息                                >
检查 Veyra 更新                             >
Veyra 版本                             [Version]
sing-box 版本                          [Version]
```

V0.1：

- 支持 Veyra 应用更新检查。
- 更新检查直接查询 Veyra GitHub Release 信息。
- 发现新版本后展示版本与发布信息，并允许用户前往对应 Release。
- 不要求 V0.1 实现应用内自动下载、替换和安装。
- 不实现 sing-box 内核更新。
- sing-box 内核版本只读展示。

---

# 3. V0.1 设置项契约

| ID | 设置项 | UI | Owner | 持久化 | 默认值/语义 | 修改后动作 | 权限 | V0.1 |
|---|---|---|---|---|---|---|---|---|
| SET-CAP-001 | 捕获模式 | Segmented / Select | RuntimeSupervisor | 当前稳定运行态不作为普通 AppSettings 持久化 | `Off` | 串行 CaptureMode transition | TUN 时显式 UAC | 必须 |
| SET-APP-001 | 开机自启 | Switch | AppSettings + Tauri Desktop AutoStart API/plugin | 是 | `false` | 更新 Windows AutoStart，确认后提交 | 普通用户 | 必须 |
| SET-APP-002 | 静默启动 | Switch | AppSettings.Startup | 是 | `false` | 下次启动按偏好决定是否显示主窗口 | 无 | 必须 |
| SET-DNS-001 | DNS | Chevron -> Dialog/Page | DnsPolicy | 是 | `Auto` | Build -> check -> apply | 无额外提权 | 必须 |
| SET-NET-001 | IPv6 | Switch | DnsPolicy / Network Policy | 是 | `false` | Build -> check -> apply | 无 | 必须 |
| SET-CORE-001 | 日志级别 | Select | RuntimePreferences | 是 | `Info` | Build -> check -> apply | 无 | 必须 |
| SET-IN-001 | Mixed 默认端口 | Input / LocalInbound Dialog | LocalInbound | 是 | `4040`，默认启用 | Build -> check -> restart/apply | 无 | 必须 |
| SET-IN-002 | SOCKS 默认端口 | Input / LocalInbound Dialog | LocalInbound | 是 | `4041`，默认禁用 | Build -> check -> restart/apply | 无 | 必须 |
| SET-IN-003 | HTTP 默认端口 | Input / LocalInbound Dialog | LocalInbound | 是 | `4042`，默认禁用 | Build -> check -> restart/apply | 无 | 必须 |
| SET-RULE-001 | 规则集 | Chevron -> Page/Dialog | RuleSet | 是 | 自动更新策略见第 7 节 | 更新缓存；需要时重编译 | 网络访问 | 必须 |
| SET-CONF-001 | 当前运行配置 | Chevron -> Read-only Viewer | Generated Config | 否 | active config | 只读查看/复制 | 只读文件能力 | 必须 |
| SET-CORE-002 | sing-box 内核 | Read-only Version | Core Runtime | 否 | bundled/current version | 无更新动作 | 无 | 必须 |
| SET-UI-001 | 主题模式 | Segmented / Select | AppSettings.Appearance | 是 | `System` | 即时应用主题 | 无 | 必须 |
| SET-MAINT-001 | 配置目录 | Chevron | StateStore | 否 | 当前 data dir | 打开目录 | 受控 Open capability | 建议 |
| SET-MAINT-002 | 日志目录 | Chevron | Runtime/App Logs | 否 | 当前 logs dir | 打开目录 | 受控 Open capability | 建议 |
| SET-DIAG-001 | 导出诊断信息 | Chevron / Action | Diagnostics Service | 否 | 无 | 生成脱敏诊断包 | 文件保存能力 | 建议 |
| SET-UPD-001 | 检查 Veyra 更新 | Chevron / Action | App Update Service | 可选保存最近检查元数据 | 手动检查 GitHub Release | 查询并展示结果 | 网络访问 | 必须 |
| SET-APP-VER | Veyra 版本 | Read-only | App Metadata | 否 | 当前版本 | 无 | 无 | 必须 |
| SET-CORE-VER | sing-box 版本 | Read-only | Core Runtime | 否 | 当前版本 | 无 | 无 | 必须 |

---

# 4. AppSettings 数据模型

V0.1 的 `AppSettings` 只保存应用级长期偏好：

```rust
struct AppSettings {
    appearance: AppearanceSettings,
    startup: StartupSettings,
    runtime: RuntimePreferences,
}

struct AppearanceSettings {
    theme: ThemePreference,
}

enum ThemePreference {
    System,
    Light,
    Dark,
}

struct StartupSettings {
    auto_start: bool,
    silent_start: bool,
}

struct RuntimePreferences {
    log_level: LogLevel,
}
```

默认：

```text
theme        = System
auto_start   = false
silent_start = false
log_level    = Info
```

以下不放入 `AppSettings`：

```text
DnsPolicy
RuleSet
LocalInbound
当前 CaptureMode
Updater runtime state
实时连接/流量/CPU/内存
```

---

# 5. 捕获模式契约

V0.1 保持：

```rust
enum CaptureMode {
    Off,
    SystemProxy,
    Tun,
}
```

UI 使用单一互斥控件，不使用：

```text
系统代理 [Switch]
TUN      [Switch]
```

## 5.1 状态

至少区分：

```text
requested_mode
stable_mode
transitioning
error
```

只有完成平台操作、sidecar Ready 与必要回读验证后的模式，才能成为 `stable_mode`。

## 5.2 System Proxy

由 Windows PlatformAdapter 执行：

```text
capture snapshot
-> write transitioning recovery record
-> apply managed proxy
-> notify/refresh
-> readback verify
-> stable
```

不得让 UI 绕过 PlatformAdapter 直接修改 WinINet。

## 5.3 TUN

```text
用户选择 TUN
-> 显式 UAC
-> 启动固定提升目标
-> sing-box Ready
-> TUN Interface / Route Ready
-> stable
```

UAC 被拒绝或 Ready 失败时不得显示为已开启。

## 5.4 启动恢复

V0.1 不把最后一次 CaptureMode 作为普通偏好自动恢复。

若未来需要“启动后恢复上次捕获模式”，单独设计 `StartupCapturePolicy`。

---

# 6. 启动设置契约

## 6.1 开机自启

```text
auto_start = true
```

表示 Veyra 注册 Windows 自动启动。

变更流程：

```text
candidate AppSettings
-> Tauri Desktop AutoStart API/plugin apply
-> verify where available
-> persist
-> publish state-updated
```

平台操作失败时不提交新状态。

## 6.2 静默启动

```text
silent_start = true
```

表示 Veyra 启动后：

```text
Backend Ready
Tray Ready
主窗口不自动 show/focus
```

用户仍可从托盘主动显示主窗口。

静默启动不等价于开机自启：

```text
auto_start   控制“是否随 Windows 启动”
silent_start 控制“Veyra 启动后是否自动显示 UI”
```

两项独立持久化。

---

# 7. RuleSet Remote 自动更新契约

默认自动更新策略：

```text
12 小时
```

同时支持“短时启动不满 12 小时”的日常更新语义。

## 7.1 每日首次启动规则

应用启动完成后：

```text
读取 RuleSet.last_successful_update_at
-> 转换为当前系统本地日期
-> 若今天尚无成功更新
-> 自动执行一次 Remote RuleSet 更新
```

因此用户每天只短时间启动应用，也能在当天首次启动时获得一次更新。

## 7.2 连续运行规则

如果应用持续运行：

```text
最近一次成功更新
-> 启动/重置 12h monotonic timer
-> 连续运行达到 12h
-> 自动更新一次
-> 成功后重新开始 12h timer
```

手动更新成功同样重置该 12 小时计时。

应用进程重启后连续运行计时重新开始；每日首次启动规则负责覆盖频繁短时启动场景。

## 7.3 成功时间

至少记录：

```rust
last_successful_update_at: Option<DateTime>
```

只有更新成功后才能推进该时间。

更新失败：

```text
保留旧 RuleSet 缓存
不清空现有规则
不伪造 last_successful_update_at
```

失败后的自动重试间隔不在本合同中定义，由 RuleSet Update Task 单独冻结。

---

# 8. DNS 与 IPv6

## 8.1 DNS

DNS 使用独立 `DnsPolicy`。

Settings 主页面：

```text
DNS                      自动 >
```

高级配置至少可承载：

```text
本地 DNS
代理 DNS
Fake IP
IPv6
```

Managed DNS 不保存任意 sing-box JSON 作为核心模型。

## 8.2 IPv6

默认：

```text
Off
```

UI 可以是一个简单 Switch，但 Compiler 必须集中映射其对：

```text
DNS strategy
TUN
Route
```

的影响。

不得在代码库中散落多个无统一语义的 `if ipv6` 分支。

## 8.3 旧 DnsPolicy 迁移兼容

旧状态中的封闭 `DnsPolicy::System` 迁移为：

```text
DnsPolicy = Auto
IPv6      = Off
```

在用户未主动修改 DNS 或 IPv6 设置前，迁移后的配置必须保持有效运行行为等价。
迁移验收必须比较迁移前后的 RuntimeIntent、最终受管配置语义和最小真实运行结果；不得只验证
字段反序列化成功。

---

# 9. 本地入口与默认端口

V0.1 默认端口：

```text
Mixed: 4040
SOCKS: 4041
HTTP:  4042
```

建议领域表达：

```rust
struct LocalInbound {
    id: InboundId,
    kind: InboundType,
    listen: IpAddr,
    port: u16,
    target_pool: Option<PoolId>,
}
```

默认端口属于 Veyra 产品契约，不来自 Clash Verge。

## 9.1 UI

主 Settings 页面：

```text
本地入口
Mixed 4040 · SOCKS 4041 · HTTP 4042       >
```

进入 Dialog/Page 后分别编辑。

## 9.2 修改语义

端口修改必须：

```text
Candidate LocalInbound
-> Validate port collision
-> RuntimeIntent
-> Compile
-> sing-box check
-> Apply/restart
-> health check
-> commit
```

如果 System Proxy 当前指向 Mixed 端口，修改 Mixed port 时必须把：

```text
sidecar listener
+
Managed System Proxy target
```

作为协调变更处理，不能留下系统代理仍指向旧端口的状态。

---

# 10. 主题契约

V0.1 正式支持：

```rust
enum ThemePreference {
    System,
    Light,
    Dark,
}
```

默认：

```text
System
```

持久化：

```text
state.json
-> settings.appearance.theme
```

主题修改：

```text
persist AppSettings
-> frontend state update
-> immediately apply theme
```

不触发 sing-box RuntimeIntent rebuild。

Windows V0.1 只要求 Veyra WebView UI 正确同步主题；不要把主题 Task 扩张成原生窗口架构重写。

---

# 11. Veyra 更新检查

V0.1 只实现应用更新检查，不实现 sing-box 内核更新。

## 11.1 数据源

查询 Veyra GitHub Repository 的 Release 信息。

至少比较：

```text
current_version
latest_release_version
```

发现更新后展示：

```text
最新版本
当前版本
Release 标题
Release 页面入口
```

## 11.2 V0.1 边界

允许：

```text
手动“检查更新”
查询 GitHub
显示检查中
显示已是最新
显示有新版本
打开 GitHub Release
显示网络/解析错误
```

不要求：

```text
后台自动下载安装
静默升级
应用内二进制替换
应用更新回滚事务
sing-box 内核升级
```

网络失败不得影响正常代理功能。

---

# 12. Settings UI 交互契约

视觉规格以 `docs/ui/veyra-ui-spec.md` 为准。

主结构：

```text
SettingSection
└── SettingRow
    ├── Label
    ├── optional Description
    └── Switch / Select / Value / Chevron / InlineLoading
```

## 12.1 Description

默认使用单行扫描式 SettingRow。

Description 只用于：

```text
存在权限或明显副作用
当前状态异常
设置名称无法充分表达语义
```

优先使用 Info/Tooltip，而不是给每一行增加长说明。

## 12.2 Busy

```text
控件原位 loading
只锁定冲突操作
不冻结整个 Settings 页面
```

## 12.3 Error

失败后：

```text
恢复最后已确认值
显示 Notice
必要时在当前行显示 recoverable error
```

UI 不得长期显示一个实际上未生效的 optimistic value。

---

# 13. V0.1 Settings 页面草图

```text
设置

┌────────────────────────────────┐   ┌────────────────────────────────┐
│ 系统与启动                     │   │ Veyra                          │
│ 捕获模式     关闭/系统代理/TUN  │   │ 主题模式           系统        │
│ 开机自启                    ◯  │   │                                │
│ 静默启动                    ◯  │   └────────────────────────────────┘
└────────────────────────────────┘

┌────────────────────────────────┐   ┌────────────────────────────────┐
│ 网络与内核                     │   │ 维护与诊断                     │
│ DNS                       自动 >│   │ 配置目录                      >│
│ IPv6                        ◯  │   │ 日志目录                      >│
│ 日志级别                Info ▼ │   │ 导出诊断信息                  >│
│ 本地入口 4040/4041/4042        >│   │ 检查 Veyra 更新               >│
│ 规则集                    N 个 >│   │                                │
│ 当前运行配置                  >│   │ Veyra 版本              0.x.x │
│ sing-box 内核            1.x.x │   │ sing-box 版本             1.x.x│
└────────────────────────────────┘   └────────────────────────────────┘
```

该页面借鉴 Clash Verge Settings 的双栏、高密度、扫描式视觉语言，但设置字段与运行语义均属于 Veyra。

---

# 14. V0.1 暂不进入的设置

```text
统一延迟
Clash 模式 / Clash 设置
启动脚本
语言切换
启动页面
托盘点击行为
关闭窗口行为
热键
轻量模式
Web UI 用户配置
公开 External Controller 到 LAN
sing-box 内核更新
```

`更新 GeoData` 不作为 Veyra 功能名存在，Veyra 使用自己的 RuleSet 更新模型。

---

# 15. 实施 Roadmap

Settings 不作为一个巨大 Task 一次性交付。正式 Task 使用真实领域名称；Settings 仅作为各领域能力
的聚合 UI，不把 CaptureMode、DnsPolicy、LocalInbound 或 RuleSet 建模为 Settings 内部状态。

| Roadmap ID | 领域 Task | 主要目标 | 技术依赖 |
|---|---|---|---|
| C01 | 版本化应用偏好与设置事务 | AppSettings、V6 -> V7 显式迁移、整体校验和安全读写事务 | 现有 StateStore / Migration 基础 |
| C02 | 主题偏好与设置聚合页面 | ThemePreference 与首个真实 Settings Composition | C01 |
| C03 | Windows 启动注册与静默启动 | Tauri Desktop AutoStart API/plugin、SilentStart 与真实 UI | C01、C02 |
| C04 | 受管内核运行日志级别 | LogLevel 的 Build -> check -> apply 与 Select | C01、C02 |
| C05 | DNS 与 IPv6 网络策略 | DnsPolicy、IPv6 Off、V7 -> V8 显式迁移及运行等价升级 | C01、C02、现有 Compiler 基础 |
| C06 | 本地入站监听配置 | Mixed/SOCKS/HTTP、V8 -> V9 显式迁移、碰撞校验与协调应用 | C02、C05（仅 Schema 顺序）、managed ingress/runtime 基础 |
| C07 | 捕获模式运行事务与 Windows TUN | Off/SystemProxy/TUN、UAC、稳定态与补偿 | C05、现有 RuntimeSupervisor、TUN Scoped ADR、所需 managed ingress/runtime foundation |
| C08 | 远程规则集生命周期 | RuleSet、V9 -> V10 显式迁移、每日首次启动、连续运行 12h、手动刷新与失败保留 | C02、C06（仅 Schema 顺序）；与现有 RoutePolicy UI Task 协调 |
| C09 | 运行配置查看、应用维护与诊断 | active config、受控目录、脱敏诊断导出 | C02、现有 active config viewer |
| C10 | Veyra Release 检查与版本信息 | 只查询 Veyra GitHub Release；展示 App/Core 版本 | C02 |

## 15.1 StoredState 演进

```text
V6 --C01/AppSettings--------------------> V7
V7 --C05/DnsPolicy + IPv6--------------> V8
V8 --C06/LocalInbound------------------> V9
V9 --C08/RuleSet metadata--------------> V10
```

每个新增持久化 Domain 必须增加新的 `schema_version` 和独立 migration。已经冻结的
`StoredStateV7` 不得在 C05、C06 或 C08 中继续增加字段；旧 migration 的既有字段含义也不得回改。
领域设计可以并行准备，但上述持久化主干必须按版本顺序串行集成。

## 15.2 技术依赖与执行顺序

Roadmap 表中的“技术依赖”只表达实现所需能力。rolling execution order 由 `.sdlc/tasks.yaml`
中的窗口顺序表达，不得通过虚假的 `dependencies` 模拟排期。

TASK-012 不是 C01 的技术依赖。只有 C08 因 RuleSet 与现有 RoutePolicy UI 边界需要与 TASK-012
明确协调。

C07 不依赖完整 C06 产品能力或 SOCKS/HTTP UI。C07 只依赖捕获模式实际需要的 managed
ingress/runtime foundation；C06 与 C07 中后交付的一方负责补充 Mixed listener 与 Managed System
Proxy target 的交叉集成回归。

每个 Task 必须独立验收。UI 纵向切片不得借机重写无关 Domain、Runtime、Platform 或持久化事务。
