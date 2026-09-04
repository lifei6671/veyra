---
id: DCR-001
status: ACCEPTED
change_source: USER:lifei 2026-09-03 explicit managed-observation runtime start/stop authorization
approval_ref: USER:lifei 2026-09-03 DCR-001 exact 1.14.0 runtime contract
amendment_ref: USER:lifei 2026-09-03 sha2 0.10.9 offline asset-integrity verification
secret_rng_amendment_ref: USER:lifei 2026-09-03 getrandom 0.4.3 per-instance API secret generation
multi_version_amendment_ref: USER:lifei 2026-09-03 selectable 1.12/1.13/1.14 core family
acl_feature_amendment_ref: USER:lifei 2026-09-03 Windows ACL implementation feature expansion
multi_version_gate_approval_ref: USER:lifei 2026-09-03 Human Technical Design Gate approved
build_asset_delivery_amendment_ref: USER:lifei 2026-09-03 build-stage sidecar asset delivery, Git ignore
pre_gate_inspection_approval_ref: USER:lifei 2026-09-03 fixed archive content inspection in Git-ignored cache only
private_dacl_policy_approval_ref: USER:lifei 2026-09-03 current-user SID exclusive DACL, no inheritance or reparse points, ACL before secret write
websocket_stream_amendment_ref: USER:lifei 2026-09-03 fixed loopback WebSocket stream dependency approval
managed_runtime_entry_amendment_ref: USER:lifei 2026-09-03 explicit managed-observation runtime start/stop authorization
managed_runtime_entry_gate_approval_ref: USER:lifei 2026-09-03 Human Technical Design Gate approved sha256:914b051a1c0c02d7fe6d49c7b7e6ae9e88a8111a7e7ad43e7c55cfc7b0bf054a
affected_design:
  - .sdlc/design/foundation.md
affected_task:
  - .sdlc/tasks/TASK-006.md
---

# DCR-001：受管 sing-box sidecar 的 1.14.0 开发基线与多版本演进

## 变更与原因

开发与首个真实 E2E 以 sing-box `1.14.0` Windows amd64 为固定基线。产品最终必须支持用户在已验证的
`1.12`、`1.13`、`1.14` 内核之间选择；但“支持”不等于接受任意版本号、任意路径或滚动下载。每个可选
版本均须先进入受控目录并完成其自身的兼容验证。独立 sidecar、固定后端 loopback Clash API、前端默认
拒绝，以及 System Proxy、TUN、UAC、WFP、Service 和任意进程能力的排除边界均不变。

待审批的唯一资产是官方 GitHub Release 的 Windows amd64 archive：

- URL：`https://github.com/SagerNet/sing-box/releases/download/v1.14.0/sing-box-1.14.0-windows-amd64.zip`
- SHA-256：`3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6`
- archive size：`32,809,391` bytes
- target architecture：Windows amd64；Windows arm64 不进入 TASK-006。

该文件可作为本机构建缓存下载、解压并完成 archive/executable SHA-256 readback；不得作为 Git 跟踪内容。
发布构建必须从固定官方 URL 获取、验证后才将资源写入安装包，应用运行期不得下载或更新 sidecar。

## 决定

用户已批准本 DCR 的固定 `1.14.0` Windows amd64 asset、SHA-256、私有 loopback API、direct `reqwest`
依赖和不改写系统网络设置的真实 E2E 边界。用户随后批准 `sha2 = "=0.10.9"`、关闭 default features，
其用途仅限离线计算内置 archive 与 extracted executable 的 SHA-256。用户还批准 `getrandom = "=0.4.3"`，
用途仅限从 Windows 系统熵生成每实例 32-byte API secret，不得作为通用随机、标识、网络或前端输入来源。
用户还确认使用现有 `windows = "=0.61.3"` 的 `Win32_Security_Authorization`、
`Win32_System_Threading` 与 `Win32_System_Memory` feature 实现并回读当前用户专属 ACL。该 feature
扩展不增加 crate 或版本，只限私有运行目录和 config 的 ACL；不得用于提权、服务、WFP 或系统网络设置。
用户随后确认，为了读取 Clash API 的 `/traffic` 与 `/logs` 持续流，可以新增受控 WebSocket 依赖；本修订
固定 `tokio-tungstenite = "=0.30.0"`（关闭默认 feature，仅启用 `connect`）和
`futures-util = "=0.3.34"`（关闭默认 feature，仅启用 `std`、`async-await`、`sink`）。前者只可连接下述固定
明文 loopback `ws://` 地址，后者只用于读取首个受限消息；不得启用 TLS、代理、任意 URL 或通用连接入口。

这些批准不放开任意下载、任意二进制、Shell、前端或系统权限。

## 影响分析

| 维度 | 影响 | 处置 |
| --- | --- | --- |
| Requirement / 产品语义 | 有变化：用户可选择已支持的 1.12/1.13/1.14 内核 | 当前仅落定可扩展目录和 1.14.0 开发/E2E；旧版本各自的资产与兼容性留待后续受控 Task。 |
| 冻结设计 | 有变化：单一资产变为版本目录与兼容 profile；新增 ACL 绑定 feature；固定 WebSocket 流读取 | 重开技术设计 Gate；Foundation 与 TASK-006 只保留 1.14.0 的当前实施范围。 |
| 安全与运行边界 | 1.14.0 新增独立 API service、Dashboard、远程控制及更多特权网络能力 | 生成配置必须不启用 `api` service、Dashboard、远程管理、TUN、bridge、TLS spoof、USB/IP 或任何新监听器；仅保留已批准的 loopback Clash API。 |
| 实现 | 已有固定 REST Ready/连接摘要 client；新增流读取改变 API client 依赖面 | Gate 通过前不得修改 manifest/lockfile 或实现流读取；通过后仅扩展固定 adapter 的流摘要桥接。 |
| 验证 | 每个版本的配置与运行行为均不可由其它版本证明 | 1.14.0 执行当前真实 E2E；1.12/1.13 入目录前分别执行 archive/hash/version、配置 fixture、`check`、API 与 Windows E2E。 |
| 已完成任务 / Evidence | TASK-001 至 TASK-005 未打包或执行真实 sidecar | 它们的交付验收不失效；仅当前技术设计基线、TASK-006 Planning 与后续 TASK-006 Evidence 需要重新建立。 |

## 版本目录与兼容契约

- `CoreVersionCatalog` 是后端拥有的封闭目录。目录项必须包含精确 semver、Windows amd64 archive URL、
  archive/executable SHA-256、打包资源名、兼容 profile 与验证状态；不接受用户提供版本、URL、路径、hash
  或二进制。
- 目录的初始唯一 `Supported` 项为 `1.14.0`。它是本 TASK-006 的唯一可执行、可打包、可 E2E 的版本。
  `1.12`、`1.13` 仅是后续必交付兼容线，当前不得显示为已支持或可选。
- 未来用户选择只可引用已经 `Supported` 的目录 ID。该选择的领域建模、持久化迁移、UI、资源加入及每个
  旧版本的配置差异均须在新的 scoped Task 中完成；不得把当前 1.14.0 的配置假设复制为兼容性结论。
- 共用的 typed `RuntimeIntent` 保持 sing-box 无关；每个兼容 profile 在编译后执行严格 allowlist。若某字段
  在目标版本不支持，编译失败并保持当前已验证实例，而不是静默删改或回退到其它内核。

当前 `1.14.0` 的唯一目录项冻结为以下完整内容清单；资源名是 Tauri bundle 内的相对目标名，不接受
额外或替换文件：

| archive member | SHA-256 | bundle resource name |
| --- | --- | --- |
| `sing-box-1.14.0-windows-amd64/sing-box.exe` | `aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc` | `sing-box/1.14.0/sing-box.exe` |
| `sing-box-1.14.0-windows-amd64/libcronet.dll` | `eee741046f0a3975124bae349aeac237aa306f3cc4de59ff5de070e74dbfdaeb` | `sing-box/1.14.0/libcronet.dll` |
| `sing-box-1.14.0-windows-amd64/LICENSE` | `bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c` | `sing-box/1.14.0/LICENSE` |

目录项的 archive 成员集合必须与该表精确相等。构建脚本在下载后先校验 archive SHA-256、archive
成员集合和每个解压文件 SHA-256，之后才允许将这三个文件复制到对应 bundle resource 目标；打包后的
资源 readback 也必须重新断言三个目标名、文件集合和 SHA-256。`sing-box version` 仍在构建阶段验证
精确 `1.14.0`，但不替代 archive 或资源内容身份校验。

## 构建期资产交付契约

- Git 只跟踪版本目录的 URL、精确版本、archive/executable SHA-256、资源名和获取脚本；`src-tauri/binaries/`
  是本机/CI 构建缓存，必须被 Git 忽略。
- 构建阶段仅允许按当前 `Supported` 目录项下载官方 archive，先验证 archive SHA-256、精确成员集合、
  每个解压内容的 SHA-256 与 `sing-box version`，再将唯一清单中的 executable、DLL 与许可证放入 Tauri
  bundle resource。任一验证或打包后 readback 失败必须使打包失败，且不得产出安装包。
- CI 或本机构建环境的网络访问只发生在上述受控获取步骤；最终安装包离线携带已验证资源，应用运行期不访问
  下载源。后续 1.12/1.13 仍需各自的固定元数据与验证才能进入该步骤。

## 备选方案与决定

1. 运行时接受任意内核、路径或 URL：拒绝，无法证明来源、hash、配置兼容性或 child 归属。
2. 将 1.12/1.13 现在标记为支持：拒绝，尚无对应资产和真实兼容证据。
3. 以受控版本目录演进，当前只执行 1.14.0：采用；保留产品演进空间而不稀释当前安全和验证边界。
4. 将受验证内核二进制提交 Git：拒绝，放大仓库与克隆成本，也不符合受控构建资源交付模型。
5. 构建期下载、验证并打包，运行期离线使用：采用。

## 保持不变的实施契约

- 后端独占固定 `127.0.0.1:9090` Clash API；每次 sidecar 启动生成 32-byte secret，只存在于私有运行期
  配置与短生命周期内存，绝不进入日志、事件、UI 或持久化状态。
- 新增直接依赖只能是 `reqwest = "=0.12.28"`（关闭默认 feature，仅启用 `json` 与 `rustls-tls`）、
  `sha2 = "=0.10.9"`（关闭 default features）；前者用途限于后端类型化 loopback API client，后者只
  计算内置 archive 与 extracted executable 的 SHA-256。`getrandom = "=0.4.3"` 只为每个受管实例
  生成一次 32-byte API secret；secret 仍只存在于私有 runtime config 与短生命周期内存。仅为固定
  `/traffic`、`/logs` 流摘要增加 `tokio-tungstenite = "=0.30.0"`（`default-features = false`，仅
  `connect`）和 `futures-util = "=0.3.34"`（`default-features = false`，仅 `std`、`async-await`、`sink`）。
  两者不得启用 TLS 或代理 feature，也不得为 UI、任意 URL 或其它网络目的使用。
- 真实 E2E 必须证明主程序与 child sidecar 均不出现控制台窗口，且不写 System Proxy、不启用 TUN、不请求
  UAC、不创建 WFP/Service 或非 loopback listener。

## 固定 WebSocket 流摘要契约

- API client 仅可在已认证 Ready 的受管 child 上，以内部 `ApiSecret` 建立
  `ws://127.0.0.1:9090/traffic` 与 `ws://127.0.0.1:9090/logs`。地址、路径、`Authorization: Bearer`
  header、握手超时 `2s`、首帧等待超时 `2s`、最大 frame `16 KiB`、最大 message `16 KiB` 和每次读取的
  消息上限 `1` 均为后端常量；不能由 UI、配置、事件或命令传入。
- 每次受控读取只等待最多一条文本消息，随后主动关闭并释放 WebSocket、header 临时值与 secret 借用；没有
  日志消息是正常的空摘要，握手/协议/超时失败只映射为封闭 recovery 状态。关闭仅发送 WebSocket 协议
  Close 帧，不允许业务数据帧；不得建立无限重连、无界或持久历史缓存、任意转发或常驻日志管道。
  用户请求的趋势图按 DCR-007 保留10分钟/600点安全速率内存窗口，首页展示10分钟、侧栏底部展示60秒，沿用现有采样与最新Delta队列及DCR-006生命周期。
- 仅后端串行 `RuntimeObservationBridge` 可在当前受管 child 的 Ready 后触发采样：每个流同一时刻最多一条
  in-flight 读取、两次同类流采样间隔至少 `1s`。IPC Snapshot、事件订阅、窗口隐藏/恢复只读取最新安全
  Snapshot，不得直接触发网络；child identity 变化或停止时取消/丢弃旧读取结果且不再发起新读取。
- `/traffic` 仅解析非负 `up`、`down` 数字，作为固定核心名义一秒窗口速率直接映射，不对相邻窗口再次差分；
  累计值来自同一实例既有 `/connections` REST 摘要，不累加有间隙的 WS 窗口。REST 总量与随后 WS 速率
  是不同采样时点，不宣称原子快照；单调时间仅用于流采样节流。见已批准 DCR-005 / TASK-009 CHANGE-003。
  累计值、连接数与速率均保持既有 DTO 数字类型。`/logs.type` 仅接受 `debug`/`info`→`Info/Runtime`、
  `warn`/`warning`→`Warning/Runtime`、`error`/`fatal`/`panic`→`Error/Runtime`；其它值固定映射
  `Error/Recovery`。内部摘要 message 只能为固定文本 `sidecar log observed` 或 `sidecar log type rejected`，
  `payload` 与原始 `type` 在映射后立即丢弃，不进入内存 Snapshot、Delta、错误、事件、日志或 UI。
- 测试以同一固定 loopback 地址验证 Bearer 鉴权、单消息上限、未知路径拒绝、traffic/log 脱敏、空日志和
  流失败的 recovery 映射；真实 E2E 仅在 Gate 通过后以受管 1.14.0 child 验证握手和安全摘要。

## 1.14.0 生成配置的默认拒绝契约

生成器只允许 TASK-006 所需的既有托管配置字段、一个 `127.0.0.1:9090` Clash API listener，以及由现有
Runtime Intent 产生的协议/出站/路由字段。不得通过原始 JSON 透传或自由表单扩展该 allowlist。

- 顶层或 `experimental` 中的 `api` service、Dashboard、远程控制、下载/更新/服务暴露配置必须缺席。
- TUN、bridge、TLS spoof、USB/IP、Tailscale SSH/Taildrop、OpenVPN server、OpenConnect server、cloudflared
  inbound、非 loopback listen 地址、额外 listen port 和任何特权网络字段必须缺席。
- 实现必须对生成 JSON 作结构断言：只存在一个 API listener，host 精确等于 `127.0.0.1`、port 精确等于
  `9090`；拒绝上述危险键、额外 listener 或不在 allowlist 的原始片段。`sing-box check` 只验证语法，不能
  替代这些断言。
- 真实 E2E 除访问批准 API 外，还必须枚举受管 child 的实际 TCP listen endpoints，断言仅有该固定 loopback
  endpoint；若无法取得可审计的 child-owned endpoint 证据，E2E 结果为 `UNAVAILABLE`，不得判通过。

## 私有配置、ACL 与异常清理契约

TASK-009 CHANGE-006 / DCR-008对上述无业务入站契约增加一个仅cfg(test)的验证例外：
唯一127.0.0.1 direct TCP入口只转发到本次有界本机回显服务，另保留固定API；
必须核对这两个listener的受管child归属。正常ObservationOnly产品配置仍只有固定API。
详细白名单、期限、资源归属及验收以DCR-008为准，不授权产品listener或WG/DNS拓扑变更。

每个受管实例使用应用私有运行时目录；目录及生成 config 仅允许当前 Windows 用户访问。当前用户 SID
必须由当前进程访问令牌的 `TokenUser` 取得，不能由用户名、环境变量或前端输入推断。实例路径只可由
应用生成的非路径 instance ID 构成；创建和打开过程中必须拒绝 reparse point。目录与 config 都使用
受保护 DACL：owner 为该 SID、唯一允许 ACE 为该 SID 的完全控制、无 inherited ACE 或额外主体。不能
将 `SYSTEM`、Administrators、Users 或任何组作为例外加入。config 的唯一 secret 是该实例生成的 32-byte
值，不得复用或持久化。

顺序固定为：生成 instance ID → 创建/打开无 reparse point 的实例目录 → 设置 owner 与受保护 DACL →
回读完整 security descriptor 并与上述精确语义比较 → 使用 `CREATE_NEW` 创建同样受保护的 config →
再次回读 config DACL → 才可写入 secret。任一创建、设置或回读失败都必须在写入 secret 前失败并清理，
不得回退到继承 ACL、宽松 ACL 或既有路径。清理是幂等的：

- 正常 stop：先停止并确认本应用 child 已退出，再擦除内存 secret 与删除实例 config；
- check/start 失败：候选 child 不得可用，立即执行同一清理并确认 config 不存在；
- child 崩溃：Supervisor 记录封闭失败状态、擦除内存 secret 并删除对应 config；
- 主程序异常退出：下次启动仅清理由本应用命名、归属可证明且 ACL 符合预期的遗留实例目录；无法证明归属或
  无法删除时停止进入真实运行态并报告不含路径/secret 的恢复失败。

验证必须覆盖 inherited-ACL 与 reparse-point 拒绝、目录/config 在 secret 写入前的 ACL readback、上述
四条清理路径及 secret 不出现在私有运行期 config 之外的文件、`state.json`、备份、日志、错误、DTO、
事件或 UI 的断言。

## 固定受管观测运行时入口（候选修订）

用户已明确授权增加受控产品入口，以已加载的 `AppState` 启动和停止用于**观测**的受管 sidecar；这不是
System Proxy、TUN 或提升运行的入口。本修订只授权下列精确语义，任何配置、端点、secret、路径、PID、
capture mode 或命令行参数都不得从 IPC 或前端传入。

- Tauri 仅新增零参数命令 `start_managed_observation_runtime` 与
  `stop_managed_observation_runtime`。响应是封闭状态枚举：启动为 `Started`、`AlreadyRunning`、
  `StateUnavailable`、`ConfigurationFailed`、`StartFailed`、`Busy`；停止为 `Stopped`、`AlreadyStopped`、`StopFailed`、`Busy`。
  内部 `Shutdown` 只返回 `ShutdownComplete` 或 `ShutdownFailed`。错误中不得包含路径、
  配置、child identity、PID、secret、原始 Core 输出或网络 payload。
- 启动命令只从应用私有固定位置的既有 `state.json` 取得状态，经既有 `StateStore` 的完整
  load/migration/引用校验后形成 `AppState`，再生成 `RuntimeIntent`。该入口不提供保存、编辑、创建或修复
  配置的 IPC；`StateStore::load` 自身已经冻结的 migration/backup-recovery 写入语义不因本入口改变。
  不存在、损坏、恢复失败、
  无法形成 intent，或 intent 没有至少一个已校验节点时，返回 `StateUnavailable`，且绝不启动 direct/空配置
  sidecar。配置编辑、首次初始化和 native profile 不属于本修订。
- 应用在 setup 时创建唯一 `ManagedObservationRuntimeController`。它用一个应用拥有的串行 worker 和固定
  内部消息集 `{Start(RuntimeIntent), Stop, Shutdown}` 独占 `WindowsManagedSidecarPort`、`SidecarRuntime`、
  child identity 和 `RuntimeObservationBridge`；worker 在收到 `Start` 前不创建 child、不执行 check/run、
  不访问 Clash API。请求队列和每个 response channel 都固定为容量一的 `sync_channel`；`try_send` 失败即
  返回 `Busy`，不等待、合并或无界积压。为避免把同步 Windows 进程操作嵌入 Tokio runtime，worker 只在该
  线程执行进程操作，并用最多 `1s` 的 `recv_timeout` 驱动采样。它不是通用任务队列，不能接受调用方闭包、路径
  或任意消息。
- `SidecarRuntime` 将增加 observation-only 构造路径：其 `mixed_port` 为 absent，`Start` 只执行现有受控
  asset/preflight/config/check/run/Ready 事务，成功后仅使用已认证的固定 `127.0.0.1:9090` API。它绝不调用
  `SystemProxyController`、WinINet、`ShellExecuteEx`、UAC、TUN、WFP、Service 或任何额外 listener；也不
  改变 `CaptureMode`。现有 System Proxy supervisor 事务保持独立，不能被这两个命令调用或复用。
- worker 在活跃且 Ready 的 child 上按既有 `RuntimeObservationBridge` 契约最多每秒采样一次；每个读取仍是
  固定 URL、单消息、2s timeout、16 KiB 上限且无内部重连循环。Snapshot、事件订阅、窗口或 Tray 操作只读
  `InMemoryRuntimeObservations`。worker 串行化 start/stop/sample：停止或 replacement 先使旧 child identity
  不可采样，再停止、清理 secret/config 并发布无敏感字段的 `Stopped`/recovery DTO，杜绝旧采样结果越过停止。
- 此入口的 `Start` 是幂等而非 replacement：只有 `SidecarLifecycle::Stopped` 才会编译并启动；`Ready` 不编译、
  不采样、不替换，直接返回 `AlreadyRunning`；`RecoveryRequired` 返回 `StartFailed`，必须先由同一入口完成
  成功的 `Stop`。因此该 controller 永不调用 `start_or_replace` 来替换活跃 child，也没有其它重启触发条件。
- 进程与退出时间边界固定：受管 `check` 用轮询 `try_wait` 的至多 `10s` 期限；超时后 kill，最多再轮询
  `2s`，未确认退出即为 `StartFailed`。Ready 仍是既有单次 `2s` 固定 API 探测。`Stop` 先 kill 当前 child，
  最多轮询 `2s`（每次 `50ms`）；任何未确认退出、I/O 或 worker response 失败均为 `StopFailed`/`ShutdownFailed`
  和 recovery，且保留归属记录，不报告已停止。`ManagedSidecar` drop 只能 best-effort kill，绝不无界 wait。
  正常 UI Start 最多等待 `15s` response，Stop 最多等待 `3s`，超时是封闭失败而非成功。
- `stop`、窗口退出和 controller drop 都只停止该 controller 当前可证明归属的 child；停止失败返回
  `StopFailed` 并保留封闭 recovery 状态，不能声称已停止。Tray Quit 必须先发送 `Shutdown` 并在最多 `3s`
  内收到 `ShutdownComplete` 后才调用 `app.exit(0)`；超时或 `ShutdownFailed` 必须取消退出并保留应用以显示
  recovery。运行态、采样、错误、child identity 和 secret
  均不得写回 `AppState`、`state.json`、备份或浏览器持久化存储。
- 前端只显示由这两个固定命令返回的安全状态并调用既有 snapshot/event；不得新增 loopback HTTP/WebSocket、
  任意启停参数或配置编辑入口。真实 E2E 必须以预置的隔离有效 state fixture 证明 start → Ready/sample →
  stop、无控制台及无 System Proxy/TUN/UAC/WFP/Service/额外 listener 变化；初次安装没有有效 state 时
  `StateUnavailable` 是预期的 fail-closed 行为，而不是 E2E 通过。

### 备选方案与拒绝理由

按 DCR-004 和用户 2026-09-04 的明确决定，配置 Compile/finalize/allowlist/check 失败返回封闭
`ConfigurationFailed`，记录脱敏日志并 Toast；当前健康实例不被停止。候选 start/Ready 失败只停止
并清理失败候选，确认成功后显示 Stopped，不自动启动旧配置；仅退出/清理无法确认时保留 recovery
与所有权。旧的配置回滚描述不能作为自动恢复授权。既有 Start 幂等、Stop/Shutdown 与权限边界不变。

1. 复用 `RuntimeSupervisor::activate_system_proxy`：拒绝；其事务允许写 System Proxy，与本入口的零捕获边界冲突。
2. 让 IPC 接收 JSON、路径、端口或启动参数：拒绝；会扩大配置、文件和进程能力边界。
3. 无 state 或空 intent 时启动 direct core：拒绝；会把无效配置伪装为可用运行态。
4. 在 Tauri async runtime 内阻塞 stream 采样：拒绝；同步 Windows child/bridge 调用可能嵌套 runtime，且难以
   证明停止与旧采样的排序。
5. 在活跃 child 上把 Start 解释为 replacement：拒绝；这会使普通重复点击隐式重启运行时。
6. 让进程 check、stop、drop 或 Tray Quit 无界阻塞：拒绝；无法兑现安全停止或向用户说明恢复状态。

## Gate 身份契约

Technical Design Gate 的身份是三个设计输入的 lifecycle-normalized SHA-256 复合值，而不是单一
Foundation hash。为使 Gate 通过后的合法状态迁移不会反向使它 stale，规范化时只排除以下**动态生命周期
字段**：Foundation 首个以 `状态：` 开头的行、DCR YAML front matter 的 `status:` 行、TASK YAML front
matter 的 `status:` 行。规范化同时将 `CRLF`/`CR` 统一为 `LF`。其它全部 UTF-8 无 BOM 字节，包括 DCR
的已批准范围和 TASK 的 `approval_refs`，都必须参与哈希。先分别计算三份规范化内容的小写 SHA-256，再对
下列 ASCII 字节串计算 SHA-256：

```text
foundation=<foundation-sha256>\ndcr_001=<dcr-sha256>\ntask_006=<task-sha256>\n
```

除上述三个动态生命周期字段外，任一输入变化都使 Gate、Review 与 Human approval stale。Gate、Review
Evidence、Human approval 与后续 TASK-006 实现 Evidence 必须引用同一个复合 identity。

## 批准与失效

此前针对 `1.12.13` archive/hash 的资产确认未被自动迁移至 `1.14.0`；用户已对本 DCR 的精确 URL、
SHA-256、amd64-only 范围、HTTP 依赖与真实 E2E 边界作出批准。批准后的强制顺序为：

1. 将 `.sdlc/state.yaml` 的 `technical_design` Gate 重开为 `PENDING`，以本 DCR 的复合 identity 替换旧
   Foundation-only identity，并清除旧 reviewer、approval 与 evidence；TASK-005 的独立 Delivery Gate 不受影响；
2. 修订 `.sdlc/design/foundation.md` 的受管内核版本，并以 DCR-001 更新 TASK-006 的资产契约和
   `approval_refs`；TASK-006 的独立验收从“hash/签名”收紧为“官方 URL + 固定 SHA-256”，不声称未配置
   信任根的签名校验；
3. 以新复合 identity 重新进行独立技术设计审查并取得新的人类 Gate 批准；
4. 仅当新 Technical Design Gate 已通过时解除 TASK-006 的计划 Blocker；之后才可修改 manifest/lockfile、
   下载资产或启动真实 sidecar。

## 所需验证

- 独立技术设计审查本 DCR、Foundation 与 TASK-006 的边界及可追溯性；
- 下载后以官方 URL、固定 SHA-256、archive 成员、`sing-box version` 和 `sing-box check` 验证实际二进制；
- Rust/TypeScript 验证与真实 Windows E2E 必须覆盖无控制台、无额外监听器、无 System Proxy/TUN 变更、
  loopback API 与 Tray hide/restore。

## 来源

- sing-box `v1.14.0` 官方 GitHub Release metadata（2026-09-03 只读获取）。
- `.sdlc/design/foundation.md` 的受管 sidecar、默认拒绝与运行时边界。
- 用户对 TASK-006 方案的确认及 1.14 内核指令（2026-09-03）。
