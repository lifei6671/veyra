# 技术基线

状态：候选（DCR-001 多版本目录与 Windows ACL feature 修订，等待独立复审后重新冻结）

## 范围与追踪

本技术基线落实已确认的 `docs/veyra.md` V0.1：以独立网络领域模型管理订阅、节点、出口组和分流，
由 sing-box sidecar 执行，并以版本化 JSON 保存完整配置状态。它是首个规划窗口的工程基线；不授权
编写业务代码、安装依赖、签名、特权 Helper 或发布操作。

Windows 的通用桌面能力归 Tauri，系统代理、权限与进程/路径等 OS 能力归 Platform Adapter；
sing-box 是 Wintun、路由、DNS 和进程分流数据面的唯一运行时所有者。

V0.1 的首个桌面目标是 Windows；macOS 平台实现排入 V0.2，并复用同一 Application、Domain、
StateStore、Compiler 和 sidecar 边界，不反向扩大 V0.1 Scope。

## 技术栈

- 桌面端：Tauri `2.x`、Rust stable `>=1.77.2`；前端为 React `19.x`、TypeScript、Vite，采用
  React Router、Zustand 与 TanStack Query 分别处理路由、界面状态和异步查询。
- Rust 运行时：Tokio `^1`、Serde `^1`、仅在基础设施边界使用的 `serde_json ^1`、`thiserror ^2`、
  `tracing ^0.1`、`tracing-subscriber ^0.3`；订阅传输使用启用 Rustls TLS 的 Reqwest `^0.12`。TASK-006
  还批准 `sha2 = "=0.10.9"`（关闭 default features），只用于离线验证内置 archive 与 executable 的
  SHA-256，不产生网络、Shell、前端或系统权限能力。`getrandom = "=0.4.3"` 只从 Windows 系统熵为
  每个受管实例生成 32-byte API secret；不得用于其它标识或任何 UI/网络输入。
- 配置状态：V0.1 使用版本化 JSON `state.json`，由 `StateStore` 抽象和 `JsonStateStore` 实现；不引入
  SQLx、SQLite 或其他配置数据库依赖。未来的历史遥测数据库不属于 V0.1。
- 运行内核：`CoreVersionCatalog` 以固定 URL、archive/executable SHA-256、资源名和兼容 profile 管理
  可选 sing-box 内核，拒绝任意版本、路径、URL、hash 或二进制。当前唯一 `Supported` 且进入 TASK-006
  实施/E2E 的 Windows amd64 条目为 `1.14.0`：archive
  `sing-box-1.14.0-windows-amd64.zip`，SHA-256
  `3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6`。1.12/1.13 是后续必须逐项
  验证后才能登记的兼容线，不能由 1.14.0 的结果推断支持。Windows arm64 不在当前 TASK-006 范围。
- 参考边界：UI 依赖组合与 clash-verge-rev 的 Tauri/React 桌面形态保持同类；Rust 边界参考
  satelite-proxy 的 Tauri sidecar 形态，但不复制其完整依赖树或功能集。首个脚手架 Task 固化具体
  package 版本、包管理器与 Tauri capability，且不得擅自加入数据库依赖。Capability 基线为默认拒绝：
  前端不直接获得文件、Shell、进程、任意 HTTP 或 sidecar 权限；首个 Task 只能在这个基线下列出确切
  capability、调用方、用途和证明用例。
- Windows 原生适配尚不批准任何 Cargo 依赖或 manifest 变更。实现 System Proxy 前必须单独确认
  `windows`、`winreg` 或经审查替代项的精确版本、feature、用途、影响和验证方式；V0.1 不引入
  `windows-service`、WFP 或全系统 TCP/UDP 表扫描依赖。
- macOS V0.2 原生适配同样尚不批准 Cargo/Swift/Objective-C 依赖、Xcode Target、Entitlement、
  Signing 或 Notarization 变更；`objc2`、Keychain、`SMAppService`/XPC 与 NetworkExtension 均不是
  V0.1/V0.2 默认依赖。

## 架构

应用采用四层单向依赖：

```text
React UI -> 类型化 Tauri commands/events -> application services -> domain
                                                        -> infrastructure adapters
```

- `domain` 持有 `AppState` 聚合、`Subscription`、`Provider`、`ProxyNode`、`NodePool`、
  `RoutePolicy`、`DnsPolicy`、`RuntimeIntent`、稳定 ID 与全局引用校验；不得依赖 Tauri、HTTP、
  文件系统或 sing-box 配置类型。
- `application` 持有用例：克隆当前 `AppState`、应用变更、执行整体校验、持久化成功后交换内存状态，
  再构建 `RuntimeIntent`。内存中的有效 `AppState` 是运行期配置事实源。
- `infrastructure` 持有订阅 Parser、`StateStore`、JSON 快照/迁移/恢复、类型化 sing-box Compiler、
  sidecar Process/API Client、文件存储和平台适配器。
- `commands` 只将有界 Request DTO 转换为 Application 调用；事件只承载类型化运行时增量和快照，
  不是第二个配置事实源。

通用桌面能力经 Tauri API/插件实现：窗口、托盘、显示/聚焦、开机启动、单实例、通知、Deep Link 和
后续更新。Platform Adapter 只承接 Tauri 不能表达的系统能力，且 Application 只面对封闭的 Port，
不依赖 Win32 类型、注册表路径或 UAC 参数。

托管模式与原生 Profile 模式是互斥的运行时来源。托管模式从类型化领域数据编译配置；原生模式接收
单独校验的 Profile，并仅暴露需求中声明的受限观测能力。

## 项目布局

```text
src/                         # React UI、pages、router、features、stores、services
src-tauri/
  src/
    application/
    commands/
    domain/
      state.rs               # AppState 与领域校验入口
    storage/
      store.rs
      snapshot.rs
      migration.rs
      validation.rs
    subscription/
    singbox/
    platform/
      windows/
        system_proxy.rs
        privilege.rs
        recovery.rs
  binaries/                  # 各受支持目标的打包 sing-box
  capabilities/
```

`storage` 是 `state.json`、备份、版本迁移和恢复的唯一所有者。`singbox`
是 Compiler、固定 Process 调用、Health Check、loopback Clash API Client 和运行时缓存的唯一位置。
`platform` 是系统代理、权限、运行态恢复、进程归属和 OS 路径等 Tauri 不能直接表达的差异行为的唯一位置；
自启动由 Tauri Desktop API/插件承接。

## Windows 平台边界

```text
Application
  ├─ Tauri Desktop APIs：窗口、托盘、通知、单实例、开机启动、Deep Link、Updater
  └─ PlatformAdapter
       └─ WindowsAdapter
            ├─ SystemProxy：WinINet + 当前用户代理配置
            ├─ Privilege：权限检查 + 固定目标的 UAC runas
            └─ RuntimeRecovery：受管运行态恢复记录

Application -> SingBoxCompiler -> sing-box sidecar -> Wintun / Route / DNS / 进程分流
```

- **System Proxy。** Application 只调用 `capture`、`enable`、`disable`、`restore` 等封闭语义，V0.1
  仅管理当前用户的默认 WinINet 连接，不改写未纳入快照的命名连接。Windows Adapter 显式区分三种状态：
  `ProxySnapshot` 是变更前完整的每连接 flags、ProxyServer、Bypass、AutoConfig URL 和自动发现值；
  `ManagedProxyState` 是应用唯一支持的托管状态，即显式 loopback proxy + 必需 bypass，关闭
  `PROXY_TYPE_AUTO_PROXY_URL` 与 `PROXY_TYPE_AUTO_DETECT`；`ObservedProxyState` 是每次写入、恢复或
  退出前的实际回读状态。

  变更在单一 PlatformAdapter 串行锁内执行：捕获 Snapshot → 写入处于 `transitioning` 的私有恢复记录
  → 写入 Managed state → 以 `InternetSetOption` 通知 settings changed/refresh → 回读与 Managed state
  语义比较 → 仅在成功后标记稳定。写入失败时，只有 Observed state 仍可证明由本次操作产生，才恢复
  Snapshot；恢复前也必须回读。关闭或恢复仅在 Observed 与 Managed state 语义相等时写回 Snapshot；
  若用户手动改写，保留用户状态并报告可恢复冲突，绝不强制覆盖。
- **权限与 TUN。** 桌面 UI 始终保持普通用户权限。仅当用户显式开启 TUN 时，Windows Adapter 才可用
  `ShellExecuteEx` 的 `runas` 启动固定、随包且已校验的提升目标；UI 不能提供任意可执行文件、路径或
  参数。该目标只执行受限的 sing-box/TUN 启动或停止语义。实现该提升目标及其认证 IPC 前，必须形成
  TUN 专项 ADR 并获得人工确认；V0.1 不以管理员身份常驻整个 UI。
- **不进入 V0.1。** Windows Service/SCM、WFP、直接创建 Wintun、手工路由或 DNS 接管，以及
  `GetExtendedTcpTable`/`GetExtendedUdpTable` 全系统 PID 关联均不实现。连接页面优先消费 sing-box
  Clash API 已提供的 process/process_path 等运行数据；缺失时显示未知，不以系统扫描补齐。
- **运行态恢复。** 系统代理原始快照、预期应用状态和受管 sidecar 身份属于私有运行态恢复记录，
  不属于 `AppState` 或 `state.json`。它与配置状态使用同一私有目录权限策略，并只在恢复/停止所需的
  最短周期内保留。恢复记录有 `transitioning` 与 `stable` 两种阶段；新 CaptureMode 未经全部 Ready
  与回读验证不得写为 stable。

## CaptureMode 切换事务

`Off`、`SystemProxy`、`TUN` 由 Runtime Supervisor 串行化，任一时刻最多一个已验证的捕获模式。
每次切换先写入包含前一稳定状态、Snapshot 和候选目标的 `transitioning` 恢复记录；只有所有平台操作和
sidecar Ready 判据满足后，才提交新的 `stable` 记录。未验证的候选状态从不作为崩溃恢复事实。

- **Off → SystemProxy。** 先生成配置并启动受管 sidecar，确认 process、loopback API 与 mixed port
  Ready；再应用并验证 `ManagedProxyState`。失败则停止新 sidecar，最终保持 Off。
- **Off → TUN。** 先生成配置并验证提升目标，再由用户显式确认 UAC；仅在同意后启动提升的
  sing-box，等待 TUN、sidecar Health 与路由就绪。UAC 拒绝、Ready 超时或启动失败时停止候选目标，
  最终保持 Off。
- **SystemProxy → TUN。** 先准备 TUN 配置和提升目标，但不启动 TUN 数据面；恢复并验证原始系统代理，
  再停止旧 sidecar，随后请求 UAC 并启动提升目标，等待 TUN、sidecar Health 与路由就绪。UAC 拒绝、
  Ready 超时或启动失败时，停止候选 TUN；仅在 Observed 仍等于恢复后的 Snapshot 时重新应用并验证先前的
  `ManagedProxyState`，否则最终为 Off 并报告用户改写冲突。此顺序允许短暂无捕获，但禁止双重捕获。
- **TUN → SystemProxy。** 先准备普通 sidecar 并确认其可提供 mixed port；停止提升的 TUN sidecar 并
  确认 TUN/路由释放后，才应用并验证 `ManagedProxyState`。若新代理启用失败，优先在仍有效的已授权会话内
  有界恢复旧 TUN；无法证明可恢复时停在 Off 并显式报错，不自动再次触发 UAC。
- **任意模式 → Off。** SystemProxy 仅按上述语义相等规则恢复 Snapshot；TUN 必须确认停止和资源释放。
  所有停止失败保持 `transitioning` 并显示恢复所需状态，不能声明已关闭。

## 状态、数据与 IPC

- 启动路径固定为：读取 `state.json` → 解析存储版本 → 按顺序迁移 → 反序列化当前模型 → 校验跨对象
  引用 → 形成内存 `AppState`。不支持的 schema 或无有效备份的损坏状态必须显式失败，不得静默重置。
- `StateStore` 只暴露 `load() -> AppState` 与 `save(&AppState)` 一类整体状态语义；Application 不依赖
  JSON 文件的具体布局。`StoredStateV1/V2/...` 与领域 `AppState` 分离，迁移不污染 Domain。迁移只能按
  明确的 `Vn -> Vn+1` 链在内存中顺序执行；当前版本跳过，重试同一原始文件不得产生二次变换。
- 配置事务固定为：克隆当前状态 → 应用变更 → 完整校验 → 写入快照 → 临时文件序列化、flush、fsync、
  原子替换 → 保存成功后交换内存状态。写入或校验失败时，保留旧内存状态和当前运行配置。
- 备份与恢复保留当前 `state.json`、最近备份及受限数量的升级前备份；损坏文件需另存诊断副本。恢复只
  能使用同样完成迁移和引用校验的备份，不能覆盖有效状态。
- 旧 schema 迁移前必须先创建升级前备份；只有完整迁移、反序列化和引用校验全部成功，才原子提交新版
  `state.json`。任一步失败均不得覆盖最后一份有效状态，恢复路径也不得写入未经校验的候选状态。
- `platform` 在应用私有数据目录创建并检查最小权限边界：目录仅当前用户可访问；`state.json`、备份、
  损坏诊断副本和 native profile 使用同一私有目录策略（Unix 目录 `0700`、文件 `0600`；Windows 使用
  仅当前用户可读写的 ACL）。权限创建、修复或读取错误不得在 UI/日志/事件中泄露 secret 或文件内容。
- 引用校验至少覆盖 Provider→Subscription、Node→Provider、PoolSource→Provider、Route→Pool、
  LocalInbound→Pool。配置状态只写 `state.json`；连接、测速、流量、临时日志等运行时数据不得写入它。
- Tauri command 是 UI 到后端的唯一传输。它只开放订阅刷新、保存出口组、保存路由、启动运行时和
  切换 selector 等封闭语义操作；前端不能运行任意进程、传递任意 sidecar 参数、读取任意路径或访问
  Clash API 凭证。普通预览、日志、错误事件和 telemetry 只可使用脱敏文本；用户显式触发的“查看/复制
  生成配置”可取得一次性、可运行的原文，但原文不得进入 Zustand 持久化状态、事件载荷、日志、错误报告
  或 telemetry，复制后即从后端与前端临时内存释放。

## 关键决策

1. **整体状态而非关系数据库。** V0.1 配置规模和编译路径都以完整状态图为中心，采用整体载入、整体
   校验、整体提交的 `AppState`；不以 SQL 查询、外键或数据库事务作为配置正确性的前提。
2. **状态迁移和回滚。** `schema_version` 驱动存储模型迁移；`Vn -> Vn+1` 仅在内存中顺序执行，迁移前
   保存升级前备份，完整校验成功才原子提交。迁移、引用校验或原子写失败均不得覆盖最后一个有效状态；
   恢复与回滚面向完整状态快照，而非单表补偿。
3. **配置 Build/Apply 分离。** 先 Build、Validate、`sing-box check` 和 Prepare，再 Apply。Apply
   只能原子替换已批准的 generated config、重启受管 sidecar、验证 Process/API/mixed port Ready，并在
   Health Check 失败时回滚上一份配置。
4. **sidecar、版本目录与本地 API 边界。** 后端拥有单一串行化 Runtime Supervisor 和封闭
   `CoreVersionCatalog`；它只能以应用控制的配置路径
   与固定参数调用随包 sing-box `check`、`run`，且只能停止可证明为本应用 child/已记录 instance 的进程。
   Clash API 仅绑定 loopback、使用生成的 secret，并仅由 Rust 后端使用。每个目录版本须由自己的 profile
   验证生成配置和真实运行；当前 1.14.0 profile 的生成配置
   只允许既有托管字段与唯一 `127.0.0.1:9090` Clash API listener；不得启用 API service、Dashboard、
   远程控制、TUN、bridge、TLS spoof、USB/IP 或任何额外 listener。私有 runtime config 只允许当前
   Windows 用户读取，API secret 仅存在于该实例 config 与短生命周期内存。
5. **Windows 适配边界。** Tauri 负责通用 Desktop/IPC/Tray 能力，Windows Adapter 负责用户级
   System Proxy、权限检查、显式 UAC 和受管运行态恢复。系统代理必须快照、通知、回读和有条件恢复；
   CaptureMode 以串行、可补偿事务切换，sing-box 独占 TUN/路由/DNS 数据面。Service、WFP、全系统
   连接扫描不属于 V0.1。
6. **敏感信息与权限。** 受限查看/复制生成配置是显式用户操作，返回的原文必须可运行；日常 UI 展示、
   日志、错误事件与 telemetry 一律脱敏 credential、UUID、password、private key 与 API secret。真实值
   不得写入前端持久化状态或事件。`platform` 同时负责 state、备份、诊断副本和 native profile 的私有
   数据目录权限。System Proxy 与 TUN 均须用户显式触发；TUN 所需 elevation/helper 另行形成 Scoped ADR
   并获得人工确认。
7. **平台与更新。** Windows 是 V0.1 目标，macOS 是 V0.2 的 DMG 直发目标，Linux 为 Beta。macOS V0.2
   的通用窗口、Tray、Dock、通知、Single Instance、Auto Start、Deep Link 与 Updater 均由 Tauri 承接；
   macOS Adapter 只负责 Network Service 级 System Proxy、显式特权运行和恢复。App/Core 更新、签名、
   托管 metadata
   与回滚均属于发布期决策，未形成发布计划前不启用 Updater。
8. **macOS V0.2 路线。** V0.2 使用普通 Tauri 桌面程序与 sing-box sidecar；System Proxy 保存每个
   Network Service 的 HTTP/HTTPS/SOCKS、PAC、Auto Discovery 与 Exceptions，并以 Snapshot/Managed/
   Observed 三态有条件恢复。TUN 数据面仍归 sing-box。长期 Helper、`SMAppService`/XPC、Keychain、
   高定制 Menu Bar 与 NetworkExtension 均不进入 V0.2；后者是面向 App Store/无 root TUN 的独立 Runtime
   架构升级，必须先经 ADR 与人工确认。

## 测试策略

- 状态存储：覆盖 load/save、原子替换、备份、损坏恢复、`v1/v2/...` fixture 迁移、不支持 schema，及
  跨对象引用失效时拒绝覆盖；重复迁移、迁移后校验失败和写入失败必须证明最后一份有效状态未改变。
- Domain：稳定身份、`NodeFilter`、Pool Membership、Route Precedence、托管/原生模式互斥，以及完整
  `AppState` 校验。
- Parser：提交支持的 Clash、sing-box、URI、Base64、TLS、Reality、WebSocket、gRPC、非法和边界输入
  fixture；不支持的数据必须产生类型化 skip/error，且不得替换旧节点。
- Compiler：确定性 Snapshot 加上固定 sidecar 的 `sing-box check`；Integration 覆盖
  `Subscription -> Parser -> AppState -> Compiler -> check`。
- Runtime：通过 Local/Mock Upstream 校验 Ready、流量、连接增量、selector 更新、重启、回滚、停止和
  受管进程清理；配置更新后重启应用须恢复同一有效状态。1.14.0 的真实验证还必须断言生成配置的
  allowlist、危险键拒绝、唯一 child-owned loopback listener、私有目录 ACL，以及正常 stop、启动失败、
  child 崩溃和后续启动清理时 secret 不泄露。
- 安全边界：覆盖默认拒绝的 Tauri capability、前端不能直连文件/进程/任意网络、私有数据目录权限，
  以及真实生成配置仅能经显式查看/复制路径出现且不进入持久化 UI 状态、事件和日志。
- Windows Adapter：用 mock Port 覆盖 System Proxy 快照、启用、InternetSetOption 通知、回读、失败恢复与
  用户手动改写冲突，以及 PAC/WPAD 已启用、写入中途失败和三态恢复；在真实 Windows Runner 覆盖当前
  用户代理开关和应用退出恢复。CaptureMode 覆盖 Off/SystemProxy/TUN 的每条转移、UAC 拒绝、TUN Ready
  超时、sidecar 启动失败、资源释放与不得双重捕获。TUN 的提升 IPC、Wintun/路由/DNS 只在其专项 ADR
  批准后进入真实机器验证。
- macOS V0.2：在其专项 Task 覆盖 Wi-Fi/Ethernet 等 Network Service 切换、PAC/Bypass/Auto Discovery
  恢复、用户手动改写冲突、显式授权取消、TUN 启动失败补偿与签名/公证发布证据；V0.1 不运行这些验证。

## 验证命令

在 Foundation 获得批准、脚手架与依赖存在后，初始验证契约为：

```text
pnpm lint
pnpm test
pnpm build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
<packaged-sing-box> check -c <generated-fixture-config>
```

上述命令目前均为 `NOT_RUN`：仓库尚无脚手架、Manifest、已安装依赖、sidecar、fixture 或 CI 工作流。

## 需要的批准

批准后可冻结本技术基线，并仅授权在已界定 Scope 内使用上述依赖族与版本范围。它不授权 TUN Helper、
Signing、Notarization、Release Hosting、启用自动更新、生产操作、Windows/macOS 原生 Cargo/Swift/
Objective-C 依赖，或任何未列出的依赖。

### 来源

- `docs/veyra.md` 的整体状态图、状态与运行态边界、版本化 JSON 状态存储、原子持久化和测试阶段。
- 用户指定的 clash-verge-rev `src-tauri/Cargo.toml`、`package.json` 作为 UI 技术形态参考。
- 用户指定的 satelite-proxy `src-tauri/Cargo.toml` 作为 Rust/Tauri sidecar 形态参考。
- 用户提供的 Windows API 边界建议，以及 Microsoft Learn 的 WinINet、ShellExecuteEx、IP Helper 与
  Windows Filtering Platform 文档（2026-09-03 复核）。
- 用户提供的 macOS V0.2 平台边界建议，以及其中引用的 SystemConfiguration、ServiceManagement、
  NetworkExtension、Tauri 与 sing-box 文档（2026-09-03 作为后续路线参考）。
- `.sdlc/design/DCR-001-sing-box-1.14.0.md`：用户批准的 1.14.0 runtime asset、默认拒绝、secret
  生命周期与 Gate 重开契约。
