---
id: TASK-011
milestone_ref: M6
dependencies: [TASK-010]
risk: HIGH
status: IN_PROGRESS
requirement_ref: docs/veyra.md
requirement_identity: sha256:93b60509f5ff04b07dcab093aa82f26ff17cf6feb446b82c82dd6615fc2dbad4
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
  - .sdlc/design/DCR-004-runtime-update-failures.md
  - docs/decisions/DCR-017-ui-integration-boundary.md
  - docs/decisions/DCR-018-subscription-use.md
  - .sdlc/design/TASK-011-subscription-ui.md
  - .sdlc/design/TASK-011-expansion-frozen-009.json
approval_refs:
  - .sdlc/evidence/TASK-011/approval-004.json
  - .sdlc/evidence/TASK-011/use-decision-006.json
  - .sdlc/evidence/TASK-011/expansion-approval-009.json
  - USER:lifei closure-012 accepted TASK-010 and authorized materializing TASK-011
---

# TASK-011：订阅导入、管理、自动更新与即时切换 UI

## 目标与冻结边界

在现有订阅导入、列表和手动更新基础上，按 Clash Verge Rev 的订阅页交互补齐远程/本地新建设置、
编辑、分享二维码、删除、请求代理、应用运行期间自动更新，以及卡片“使用”即时切换；底层继续使用
Veyra 的严格 Clash/sing-box/URI 解析、版本化 AppState、原子保存、受管 sidecar 与安全 DTO。

本 Task 的增量技术边界已经 USER:lifei 以“确定”完整批准，证据为
`.sdlc/evidence/TASK-011/expansion-approval-009.json`。冻结组合身份记录于
`.sdlc/design/TASK-011-expansion-frozen-009.json`，优先级为 008→007→006→proposal005；未改变的
导入/更新边界继续遵循 `.sdlc/design/TASK-011-subscription-ui.md`。已批准的 V5、DTO、五个 main 命令、
运行切换/观测、应用内调度和 `qrcode.react ^4.2.0` 不再重复申请授权。

页面布局、卡片、对话框与菜单映射沿用 `.sdlc/evidence/TASK-011/ui-reference-003.md`：普通文字使用
参考系统字体栈，Emoji/旗帜使用已打包的 Twemoji Mozilla 字体；保持 200px 左栏、520px 最小窗口、
浅色/深色主题与参考色值。参考只约束可交付交互，不引入 Mihomo API、provider/group/rule/script 运行语义。

## Scope

### allow

- `src/App.tsx`、`src/styles.css`、`src/lib/{subscriptions.ts,observability.ts}`、
  `src/components/subscriptions/**` 及对应前端测试：实现顶部 URL 快捷导入、远程/本地新建与编辑表单、
  列表/卡片、菜单、二维码、更新策略、选择/实际运行状态及加载/空/忙碌/错误反馈。
- `package.json`、`pnpm-lock.yaml`：只允许安装已批准的 `qrcode.react ^4.2.0`，锁文件必须由 pnpm 生成。
- `src-tauri/src/domain/{state.rs,mod.rs}`：实现 V5 的订阅描述、远程请求选项、更新策略、最后尝试时间、
  `active_subscription_id` 与 `active_configuration_generation`；保持既有 Subscription/Provider/Node/
  Pool/Route 身份和引用。
- `src-tauri/src/storage/{migration.rs,store.rs,mod.rs}`：实现 V4→V5 完整迁移、备份/恢复、整体校验和
  原子保存；V1→V2→V3→V4 历史语义保持。
- `src-tauri/src/subscription/{mod.rs,parser.rs,normalize.rs,fetch.rs}`：实现批准的默认/自定义 User-Agent、
  Clash `proxies` 兼容、远程请求选项、条件请求、代理选择、重定向、4 MiB/超时和安全响应元数据。
- `src-tauri/src/application/{mod.rs,subscription_management.rs,provider_replacement.rs,
  managed_observation_runtime.rs,observability.rs,state_access.rs,selected_subscription.rs,
  subscription_scheduler.rs}`：实现设置读取/编辑/删除、应用内自动更新、共享短 state gate、选择投影、
  operation 状态、runtime/subscription guard 与 Use 的 prepare→persist→apply。
- `src-tauri/src/singbox/runtime.rs`：只按冻结设计把既有 last-known-good 替换拆成内部
  prepare/commit/cancel 阶段；DCR-004 的 stop/spawn/Ready 失败语义不变。
- `src-tauri/src/platform/windows/system_proxy.rs`：只增加当前用户 WinINet snapshot 的只读、受限
  HTTP/HTTPS `host:port` 解析；不修改系统代理。
- `src-tauri/src/{commands.rs,lib.rs}`、`src-tauri/build.rs`、`src-tauri/capabilities/default.json`：
  扩展 list/update/Start/RuntimeObservation 安全 DTO，并新增 main-only
  `get_subscription_settings`、`edit_subscription`、`activate_subscription`、
  `get_subscription_share_url`、`delete_subscription` 及对应五项精确权限；保留原三个订阅权限。
- 上述文件内的定向测试，以及 `.sdlc/evidence/TASK-011/**` 中不含真实订阅、URL 凭据、节点 secret、
  私有路径或原始响应的有界 Evidence。

### deny

- 不显示或实现“全部更新”、批量更新/删除、拖拽排序、原文/文件直接编辑、打开配置文件、merge、
  script、规则编辑、节点编辑或代理组编辑；“节点/代理组/规则”待 TASK-012 有真实页面后另行接线。
- 不递归执行 Clash `proxy-providers`，不持久化或执行 Mihomo groups/rules/DNS/TUN/merge/script；只有
  `proxy-providers` 而无非空 `proxies` 时固定失败。
- 不扩展 `SingBoxCompiler` 的 default target 能力，不新增 mixed inbound、通用代理监听、TUN、UAC、
  WFP、Service、系统级调度或开机任务。当前 ObservationOnly runtime 未提供 owned mixed port 时，
  ManagedCore 更新入口必须禁用并显示“当前内核未提供代理端口”，后端返回 `proxyUnavailable`；
  不回退 Direct/System，也不借本 Task 解锁新监听。
- 不自动重试、补跑错过周期、因窗口恢复/唤醒形成更新风暴，且普通导入、编辑、手动/自动更新不得
  自动 Apply 或切换运行实例。
- 不向 list/settings/observation/event/error 暴露完整 URL、User-Agent、validator、响应头、订阅原文、
  节点凭据、生成配置、PID 或文件路径。完整分享 URL 只允许 remote、main 窗口、明确用户点击后短暂返回，
  不写日志、事件或持久二维码缓存。
- 除已批准 `qrcode.react ^4.2.0` 与八个订阅命令的 main capability 外，不新增依赖、权限、任意文件/
  Clipboard/Shell/进程能力；不 commit、push、release，不访问公网或用户真实订阅。

## 冻结 IPC 输入与安全设置 DTO

下列 camelCase 结构是前端、commands 与 `SubscriptionManager` 的唯一共享契约；全部 Rust request 使用
`deny_unknown_fields`，未列出的字段、null（仅明确允许处除外）和非法 enum 固定拒绝。名称和描述仍按
本 Task 的标量/控制字符规则验证。

```ts
type ProxyMode = "direct" | "system" | "managedCore";

type RemoteImportOptions = {
  userAgent?: string; // 省略使用默认 UA；导入时不接受空字符串
  timeoutSeconds: number;
  proxyMode: ProxyMode;
  verifyTls: boolean;
  allowAutoUpdate: boolean;
  intervalMinutes: number | null;
};

type ImportSubscriptionRequest = {
  name: string;
  description: string;
  source:
    | { kind: "remote"; url: string; options: RemoteImportOptions }
    | { kind: "manual"; content: string };
};

type RemoteRequestPatch = {
  userAgent?: string; // 省略=保留；""=清除并恢复默认 UA；非空=替换
  timeoutSeconds?: number;
  proxyMode?: ProxyMode;
  verifyTls?: boolean;
};

type UpdatePolicyPatch = {
  allowAutoUpdate?: boolean;
  intervalMinutes?: number | null; // null=清除周期
};

type EditSubscriptionRequest = {
  id: string;
  name?: string;
  description?: string;
  urlReplacement?: string;
  remoteRequest?: RemoteRequestPatch;
  updatePolicy?: UpdatePolicyPatch;
};

type UpdateSubscriptionRequest = {
  id: string;
  content?: string;
  routeOverride?: "managedCore";
};
```

manual 导入不接受 remote options；manual 编辑只允许 name/description，重新粘贴/选文件继续走
`update_subscription.content`。remote 更新禁止 content；manual 更新必须提供 content 且禁止
routeOverride。普通 remote 更新省略 routeOverride 并使用持久 proxyMode；菜单“使用 Veyra 代理更新”
只为该次传 `managedCore`，不修改持久设置。

`get_subscription_settings({id})` 的 ok payload 固定为：

```ts
type SubscriptionSettings = {
  id: string;
  name: string;
  description: string;
  sourceKind: "remote" | "manual";
  urlPreview: string | null;
  hasCustomUserAgent: boolean;
  remoteRequest: {
    timeoutSeconds: number;
    proxyMode: ProxyMode;
    verifyTls: boolean;
  } | null;
  updatePolicy: {
    allowAutoUpdate: boolean;
    intervalMinutes: number | null;
  };
  shareable: boolean;
};
```

`urlPreview` 必须省略 query/fragment/userinfo，custom UA 本文永不返回。`edit_subscription` ok 返回
`{subscription, contentChanged}`；`update_subscription` 保持同一 ok 形状；settings/edit/update 均使用冻结
`{status:"ok",...}|{status:"error",error:<固定码>}`。Import 的 remote options 是完整新建值，Edit 的
remoteRequest/updatePolicy 是 patch；前端不得用默认值覆盖被省略的已存设置。

## SF-001：基础导入、列表、卡片与原子更新

**需求：** 保留顶部 36px URL 快捷栏：placeholder“订阅文件链接”，空时粘贴、非空时清除，Enter 忽略
IME composing，右侧“导入/新建”。快捷 URL 不先询问名称；安全默认名不从 URL、内容或凭据派生。
“新建”对话框类型为远程/本地，名称可空；本地允许浏览器 File API 读取文件或粘贴文本，不传路径。
三种输入共用严格解析/归一化和完整 AppState 原子提交。远程/手动更新继续遵守 200/304/cache 和身份保持。

**验收：** 加载、空、失败和手动重试状态明确；卡片显示安全名称、来源、节点数、相对/完整更新时间、
可选流量/到期，未知值不伪造为零。网格为 `auto-fill/minmax(260px,1fr)`、gap 8px，标题
18px/600/26px；520×520 与 960×640 无横向溢出，浅/深主题可读。导入或更新仅在零 skipped、至少一个
节点、完整校验与保存成功后可见；失败保留旧节点、时间、validator、state 与 child。新对象 ID 使用后端
16-byte 随机身份和既有命名空间，碰撞/熵源失败整批不提交。304 仅在来源、缓存和节点仍一致时成功。

**Verification：** Tauri mockIPC 覆盖三类输入、IME、空/忙碌/错误、文件不传路径、安全默认名、
200/304/contentChanged 与敏感字段反向断言；隔离 Store、可控熵源和 loopback HTTP fixture 覆盖原子提交、
首次创建竞争、碰撞/熵源失败、save failure、无缓存 304、reload 失配和重启身份。真实 Windows WebView
覆盖 URL、粘贴、文件各一次及浅/深、520/960 布局。

**implementation_status：** PENDING
**acceptance_status：** PENDING

## SF-002：V5 设置、新建/编辑与卡片菜单

**需求：** 远程表单提供名称、描述、URL、User-Agent、总超时、更新周期、代理模式、TLS 校验与允许自动
更新；本地只提供名称、描述和重新选择文件/粘贴内容。名称 trim 后 1–80 Unicode 标量；描述可空，
非空 trim 后最多 280 标量且无控制字符；自定义 UA 为 1–256 可见 ASCII 且拒绝 CR/LF；超时 5–120 秒、
默认 30。代理是 `Direct|System|ManagedCore` 互斥枚举，默认 Direct；TLS 默认校验，关闭时二次确认。
remote 默认允许自动更新但周期为空；填写周期时至少 1440 分钟。manual 固定禁止自动更新。

**需求：** 卡片点击、右键“使用”和键盘入口共用同一 Activate 动作。菜单只渲染真实可用的“使用”、
“分享二维码”、“编辑信息”、“替换订阅源”、“更新”、“使用 Veyra 代理更新”和“删除”；local 隐藏 URL/
二维码/代理/自动更新入口。remote URL 替换须先获取、解析、校验，再与设置和节点同事务提交。二维码只用
用户动作取得的 share URL 在内存渲染，关闭即清除。删除需通过引用和运行状态检查，不自动 Stop。

**验收：** V4→V5 保留所有历史 ID/引用，新增字段按冻结默认值迁移，active 为空且迁移不联网。表单
成功关闭清空，失败保留可修正输入且只显示一次固定 Toast。编辑名称/描述不误改节点 generation；URL
替换失败不改设置、节点或成功时间。二维码仅 remote/shareable 可见，不进入 DOM 文本、日志、event 或
截图工件。非 active 删除原子清理其自有对象且不破坏跨引用；active 仅在 Stopped、无切换且非 Recovery
时可删，成功后清空 active 并增加 generation；冲突固定拒绝。所有被 deny 的菜单项完全不渲染。

**Verification：** 前端表单、菜单、focus trap、键盘、loading/disabled、TLS 二次确认、QR 生命周期与
exact visible-condition 测试；V5 完整迁移/重启、设置校验、URL 替换事务、引用冲突、active/non-active
删除和安全 DTO 精确键集合 Rust 测试。真实 WebView 使用无凭据 loopback URL 验证新建、编辑、二维码、
删除和失败保留输入，不保存真实 token 截图。

**implementation_status：** PENDING
**acceptance_status：** PENDING

## SF-003：Clash 获取兼容、代理路径与应用内自动更新

**需求：** 默认 UA 为 `veyra/<应用版本> clash-verge/v2.5 flclash/1`；自定义 UA 遵守 V5 校验。Clash
YAML 只提取非空 `proxies`，支持已接受的 BOM、quoted/flow 写法与 VMess `alterId/cipher/servername`
映射；混合无效/不支持节点继续整批失败。请求沿用 HTTPS/受控 loopback HTTP、userinfo 拒绝、最多五次
跳转、4 MiB 累计正文、同源 validator 和总超时；跨 origin 不转发 validator。

**需求：** Direct 固定 no-proxy；System 每次请求前只读受限 WinINet snapshot，PAC/WPAD/userinfo/歧义
配置固定 `systemProxyUnavailable`；ManagedCore 仅使用当前 Veyra owned、Ready 且确实提供的 loopback mixed
port，否则 `proxyUnavailable`。自动更新协调器仅在应用进程运行时按有效周期执行，失败记录
`last_attempt_at_ms` 后等待完整下一周期，不立即重试；完整成功才更新节点、validator、流量和
`last_success_at_ms`，且无论成功失败都不自动 Apply。

**验收：** 代理路径不读取前端端口、不接受代理凭据、不自动 fallback、不修改系统代理。当前
ObservationOnly runtime 的 mixed port 为 unavailable 时，UI 禁用“使用 Veyra 代理更新”并显示固定说明；
后端直接调用仍固定失败且旧状态不变。自动更新在无周期、manual、应用关闭时不运行；并发编辑/更新/Use
由 guard fail-fast busy，网络等待不持 state gate，prefetch→unlock HTTP→reload source/cache→commit
顺序保持。更新 active 订阅只增加 generation 并显示旧版本，等待用户再次“使用”。

**Verification：** loopback fixture 覆盖默认/自定义 UA、Clash 兼容、redirect/validator、timeout、4 MiB、
200/304 和失败旧值；隔离 WinINet parser 验证 System 正反例且不写用户设置；注入 mock owned port 只验证
Veyra 的 ManagedCore 路由逻辑，并另测真实 unavailable 状态的 UI disabled/`proxyUnavailable`，不把 mock
冒充现有 core 具备 mixed listener。可控时钟覆盖首次调度、完整周期、失败不重试、关闭取消与更新不 Apply。

**implementation_status：** PENDING
**acceptance_status：** PENDING

## SF-004：所选订阅投影与“使用”即时切换

**需求：** V5 在 AppState 持久化 `active_subscription_id` 与单调、安全整数范围内的
`active_configuration_generation`。完整状态保留其它订阅，但运行投影只包含目标订阅的 Provider/Node；
Pool source 过滤后为空则不进入候选，用户 Route/default/manual-selected 的跨订阅硬引用固定
`selectionConflict`。`Unconfigured` default 在投影中映射到当前订阅的 runtime-only 隐式 Pool；有效显式
Pool 可保留，Direct/Block default 按当前 Compiler 能力在 preflight 固定冲突。Use 与零参数 Start 必须
共同消费强类型 `{runtime_intent, projected_default_target, selected id/generation}`，不得分别拼接持久 default。

**需求：** Activate 在同一 worker 中执行投影/compile/finalize、prepare/check、短 gate reload/CAS、按需
save、stop/run/Ready。旧 child 保持至 check 和持久提交完成；提交前可取消，提交后必须继续收敛。
Stop→Start 使用持久所选订阅；active 为空 Start 返回 `subscriptionSelectionRequired`。同一 B 节点更新后
generation 变化，再次“使用”必须应用最新完整快照，不能只按 ID 返回 AlreadyCurrent。普通导入/编辑/
更新不调用 Activate。

**验收：** 运行 A→Use B 与 Stopped→Use B 都在 B Ready 后才显示“正在使用”。compile/check 失败保持
A；save 失败不停止 A；旧 stop 失败保持 B 为持久选择并进入 Recovery；spawn/Ready 失败不自动重启 A，
实际为 Stopped 或 Recovery，遵循 DCR-004。runtime/subscription owned guard 随 worker 存活，Start/Stop/
Use/Update 竞争只有一个操作；同步超时返回 pending operation ID，提交前迟到 cancelled、提交后迟到
ready/failed 均由权威观测收敛。reload I/O 是 failed/stateUnavailable，expected-state 失配是 failed/busy，
清理不确定是 failed/recoveryRequired。退出在提交前取消，在提交后先完成 Apply 再由同一 worker Shutdown。

**验收：** Summary wire 只使用 `active` 和 active 项非空的 `activeConfigurationGeneration`，禁止同时输出
`selected/enabled`。实际 applied ID/generation 与持久 active 分开：只有 lifecycle Ready 且元组一致显示
“正在使用”；否则准确显示正在切换、旧实例仍运行、旧版本、已选择但服务停止或切换未完成。Start 七值、
Activate ok/pending/error、switch status/errorCode 均为冻结闭集；未知请求字段、discriminant、错误码和非法
status/error 组合 fail closed，响应不回显原始 payload。

**Verification：** 隔离 Store 与受控 SidecarPort 覆盖迁移 active=None、A→B、Stopped→B、同 B 新
generation、隐式/显式 Pool、Direct/Block/cross-reference 冲突、compile/check/save/stop/spawn/Ready 故障、
并发/取消/退出和重启状态。命令与前端 decoder 穷举固定 union、operation ID 迟到终态和敏感值反向断言。
真实 WebView 覆盖卡片/右键/键盘同一 Use、loading/focus、pending 收敛及 selected/applied 不一致文案。

**implementation_status：** PENDING
**acceptance_status：** PENDING

## Task 独立验收

在同一隔离数据目录、身份有记录的多个 Veyra 会话完成一条真实 Windows WebView 用户链：从空态用
loopback URL、粘贴和文件导入；新建带完整远程设置的订阅；编辑名称/描述与 URL；查看无凭据二维码；
完成 direct 手动更新；验证当前 runtime 无 mixed port 时代理更新禁用；运行 A 后 Use B、更新 B 再次 Use、
Stop→Start B；删除一个可删除订阅；重启后读回 V5 状态。证据覆盖浅/深主题、520/960 布局、加载/空/
忙碌/失败、菜单条件、Toast、pending 与 active/applied 文案，不访问用户真实订阅或公网。

HTTP、System/ManagedCore mock 路由、自动调度、迁移、引用、保存和 runtime 各故障矩阵由隔离 Rust/
browser 测试覆盖，不要求在同一原生会话注入。Mock owned port 只证明 Veyra 路由，不证明当前
ObservationOnly core 已有 mixed listener；parser、build 或 core 单独启动也不能代替 UI 验收。

冻结完整 Delivery Unit 后由未参与实现的 Reviewer 对全部前后端、schema、IPC/capability、依赖/锁和
Evidence 做独立审查。四个 SF 与 Task 仍须 USER:lifei 明确验收；旧 Review PASS、技术 Gate PASSED 或
构建成功都不代替本 Task 验收。

### 固定验证命令

所有命令使用合理外层超时，并确认非零测试数：

```text
pnpm lint
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml application::subscription_management::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml subscription::fetch::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml storage::store::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml commands::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml application::managed_observation_runtime::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml application::observability::tests:: -- --nocapture
cargo test --manifest-path src-tauri/Cargo.toml singbox::runtime::tests:: -- --nocapture
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings
```

**acceptance_status：** PENDING

## 既有证据、Dependencies 与 Risk

- `.sdlc/evidence/TASK-011/baseline-001.json` 继续定义本 Task 开始前的 HEAD、源清单和用户既有改动；
  不得 reset、覆盖或把其后他人改动归为本增量。
- 原始三 IPC/前端交付历史保留于 `backend-implementation-001/002.json`、`backend-review-001/002.json`、
  `frontend-implementation-001..006.json`、`frontend-review-001..006.json`；兼容修复历史保留于
  `compatibility-implementation-005.json` 与 `compatibility-review-005.json`。它们只证明各自冻结身份，
  对本次 V5/Use/菜单/调度/依赖变化后的完整 Delivery Unit 不自动新鲜。
- 设计与批准历史保留 `design-review-001/002.json`、`ui-reference-003.md/json`、
  `ui-alignment-review-003.json`、`approval-004.json`、`use-decision-006.json`、
  `expansion-design-review-006/007/008.json`、`expansion-approval-009.json` 和冻结 manifest；历史 REWORK/
  CANDIDATE 不改写为 PASS。
- TASK-010 已由 USER:lifei closure-012 验收为 DONE；`FOLLOWUP-TASK010-NATIVE-012` 仍 OPEN/NOT_RUN，
  不由本 Task 自动关闭。TASK-012 依赖本 Task，但其页面与业务不提前进入本实现。
- 已批准的唯一新增生产依赖为 `qrcode.react ^4.2.0`；已批准的权限仅为 main 窗口五个新命令加原三个
  订阅命令。当前实施无需再次确认；任何其它 dependency、lock、权限、listener 或 Compiler 能力仍须
  Change Control。
- 主要风险：V5 迁移破坏身份/引用，URL/QR/UA/节点凭据泄漏，System proxy 解析外部歧义，ManagedCore
  在无 owned port 时伪可用，调度重试风暴，更新与 Use 丢失写，持久 active 与实际 child 混淆，check 后
  stop/spawn 失败伪回滚，以及退出/超时后的迟到 operation 覆盖新状态。上述风险均有对应 SF 验收；
  不扩展为 sing-box 协议、地址族或系统网络认证。

## Approved compatibility amendment 015

USER “帮我修复” approves proposal013 node-model/compiler additions; USER explicitly approves importing supported10 nodes and warning for one TCP-only multiplex node. Design TASK-011-clash-compatibility-015.md supersedes only the corresponding zero-skipped/TLS-HY2 fields of frozen009. Only UnsupportedOption for validated VLESS TCP-only smux may be omitted, with persisted safe skippedNodeCount; invalid/unknown nodes still cause atomic failure. Scope additionally includes src-tauri/src/singbox/compiler.rs, required TLS/HY2/domain fields, subscription count in model/DTO/UI, tests/fixtures/subscriptions/clash-compatible.yaml, and mechanical existing test constructors. V5 additive defaults preserve previous states. No dependency/permission/listener or real-user node execution. Acceptance remains PENDING.

## Approved UA selection amendment 019

USER explicitly requests default Clash UA for import and UA dropdown for new subscriptions, and authorizes read-only client-format probing of the provided subscription URL. Scope: existing fetch default and request tests; SubscriptionPage preset/custom controls and UI regression evidence. Use clash-verge/v2.5 default, sing-box/1.14.0 and Clash.Meta presets, plus custom input. Existing optional userAgent DTO/persistence unchanged; omitted edit keeps stored custom UA; explicit restore resets to Clash default. No dependencies/permissions, no subscription bodies or credential-bearing URLs in evidence, no node execution. Server UA probing verifies response shape and application parsing only. Any newly discovered normalization policy change is separate from this UA amendment. Task/Human acceptance remains PENDING.

## Approved UI detail and subscription remediation amendment 021

USER explicitly requests reference-source alignment: stable card sizing during resize, compact right-click menu, no skipped/auto-update/active badge on cards, node count and update time on one row, absent traffic/expiry left blank with equal card heights, QR closes on backdrop/Escape without footer buttons, outlined floating labels for create/edit, buttonless reference-style Toast. Latest request supersedes prior card skipped-warning requirement; import-result warning remains. UI scope: SubscriptionPage.tsx, subscription CSS and directly related tests. User also requests repair of the new subscription under Clash and sing-box UAs; investigate exact rejected fields and duplicate identities, preserve supported semantics, and use sanitized regressions. Do not silently drop unsupported active settings or weaken existing atomic safety; material new model/normalization policies require a concrete decision. No dependency/permission changes, no node execution, no secrets in evidence. Task/Human acceptance remains pending.

Approved policy025: USER selects merging identical connections retaining first, refusing Clash certificate pins and importing server-provided sing-box response. Frozen TASK-011-subscription-policy-frozen-025.json/design review PASS governs import-boundary full tuple dedup and the closed unsupportedCertificatePin error across application/commands/TypeScript, with no model/normalizer algorithm or security downgrade. Scope includes those existing files and regressions; no repeated approval needed. Human Task acceptance remains pending.

## Necessary WebSocket compatibility remediation 027

The current explicit request to repair this subscription and import its sing-box response authorizes the minimal end-to-end preservation of its existing WebSocket parameters. Real-response diagnostic026 and independent finding REMED025-001 identify ten nodes with max_early_data=2048, early_data_header_name=Sec-WebSocket-Protocol and one-element Host arrays. Prior policy025 remains unchanged: duplicate complete connections merge first; Clash certificate pins fail; no automatic UA refetch or activation. Extend only the persisted transport representation and compiler mapping needed to retain those parameters, with backwards-readable defaults, stable old node identities and safe synthetic regressions. This is a necessary part of the requested repair, not an optional feature or permission/dependency change. Candidate design027 must receive independent review before implementation. Whole-subscription import is not yet passing; TASK-011 remains IN_PROGRESS and human acceptance pending.

## Remediation delivery checkpoint 029

Implementation and independent incremental delivery review PASS for frozen remediation-target-029.json (12 source/fixture files; 8 verification artifacts). REMED025-001 is FIXED by reviewed WebSocket early-data preservation and strict Host normalization. Review: remediation-code-review-028.json, SHA256 c29a32c654aaaa76a659631dbcf3c157c75688c7a6dcf8181d03e6cf83c336ef. Real sing-box response read-only parsing accepted41/skipped0; ten exact duplicate connections merged to31 unique; normalization and application ConfigCompiler plan succeeded. Clash certificate pins still cause the approved whole-document refusal. Native028 debug build source-stable and isolated UI opened; actual native WebView import persistence and human Task acceptance remain PENDING_USER. No core protocol execution, commit, push, release or Task DONE transition.

## Approved partial node import and compact card amendment 030

USER explicitly requests filtering unsupported Clash/sing-box nodes and importing the remainder, superseding the prior whole-document certificate-pin refusal. Preserve rejected nodes' security parameters by filtering the complete node, never dropping a pin/transport option to import it. Reuse parser classifications and persisted filtered count; whole-document errors/all-filtered results retain existing state. Detailed candidate TASK-011-partial-import-030.md requires independent design review. The user also requests used / total on one compact card row like the reference and verification of SF-Pro.ttf actual usage. Scope extends existing manager/commands/client message and card files with direct tests, no schema/dependency/permission changes. No repeat authorization needed; Task acceptance remains pending.

## Partial import and compact-card checkpoint 032

Policy030 implemented under its independently reviewed frozen design. Target compact-import-target-032.json binds six sources and ten evidence artifacts. Independent partial-import-code-review-030.json PASS/No findings (SHA256 79383a3f8b47e75d56febe570d4fc67967a166229d125803320a82c85ab8edb7). Related manager29/commands10/provider4 tests, frontend82/lint, Rust fmt/clippy--lib, UI031 browser28 and source-stable native030 build PASS. Font reference030 confirms SF-Pro.ttf is not used by the default reference web font stack; no font changes. Real memory-only diagnostics: Clash filters22 and deduplicates to15; sing-box filters0 and deduplicates to31, normalization/config-plan checks pass. Actual native WebView persistence and human Task acceptance remain PENDING_USER; isolated native UI titled Veyra订阅精简版 is open. No commit/push/release/Task DONE transition.

## Approved sidebar and subscription stability amendment 033

USER requests the Clash Verge sidebar graph/number layout, removal of transient card badges and traffic divider, refresh-date fallback when expiry is absent, and stable card resizing. UI-only scope: App sidebar integration, layout components and SubscriptionPage/styles with browser checks. Reference profiles.tsx uses a fluid auto-fill 260px/1fr grid, not resize debounce. Retain its settled column rules; a 120ms resize settle on the card grid explicitly addresses the user's repeated drag-resize complaint. No data/IPC/dependency/permission changes. Verify continuous drag, stable mount/heights, date fallback, light/dark and real unavailable states. Native UI acceptance remains distinct; Task stays IN_PROGRESS.

## Sidebar/card checkpoint 033

UI033 implementation and independent incremental review PASS, frozen sidebar-cards-target-033.json binds six sources and fifteen artifacts. 85 frontend tests/lint, 26 card and 21 sidebar mockIPC browser checks, source-stable native033 build passed. Independent sidebar-cards-review-033.json has no findings and zero drift. Native isolated title Veyra 侧栏与卡片优化版 started; actual OS drag/hide-restore and Human acceptance PENDING. No Rust/core tests were repeated for this UI change, no lifecycle/Gate/commit/push/release transition.

## Approved editor, runtime actions and memory amendment 034

USER explicitly requests subscription source-file editing with JSON/YAML editor, copy/format/fullscreen; save validates syntax and sing-box conversion, rejects invalid edits without closing or replacing, valid save replaces local document without a remote fetch and immediately reapplies an active subscription. The next explicit/automatic remote refresh replaces the local edit. USER requests header selection/batch delete, refresh all remote subscriptions, view actually applied runtime configuration only while running, and force reactivation from current local subscription. USER requests zero-rate idle sidebar without waiting text, actual owned-core memory and monotonic card resize across column transitions. This explicit amendment supersedes the prior deny clauses only for these features; full Clash rules/groups/scripts migration, arbitrary paths/processes/listeners remain excluded. Raw document and applied configuration are returned only through explicit main-window document/view commands, not summary/event/log DTOs. Necessary version-compatible document storage, owned-runtime commands/observation field and precise main IPC permissions are in scope under technical design034 review; no user-node execution. User explicitly approved adding monaco-editor, locally bundled without CDN; installed exact0.56.0 by pnpm, lockfile generated. UI concerns can proceed independently while the material backend candidate receives independent design review. Task stays IN_PROGRESS; no repeat authorization for specified behavior.

## Editor/runtime candidate 038 (2026-09-06)

User-approved Monaco Editor 0.56.0 is locally bundled and lazily loaded. Subscription Edit File supports JSON/YAML, copy, explicit format, fullscreen and strict Save. V6 retains byte-exact source; Save does not fetch remote content, selected subscriptions apply immediately, saved-but-apply-failed results remain editable and exact-buffer retries reapply without advancing generation. Manual/automatic remote 200 refresh replaces the local override and validators are suppressed while overridden. Legacy migrated subscriptions without retained source return documentUnavailable until a normal refresh/import supplies it.

Header selection/bulk delete, refresh-all remote subscriptions, actual active-slot JSON view and force reactivation are implemented. Cards cap at 300px with monotonic continuous resizing, idle rates show 0.00 B/s, and sidebar memory uses the owned child's Working Set. Main-only document permissions and the Windows ProcessStatus feature are limited to this approved behavior. No user-node or protocol matrix was run.

Frozen target editor-delivery-target-038.json SHA256 45d57e73261330a892858593a89dff874162e03b50294f7ccad8bb4e0114aad5. Frontend97/lint/build pass; light integrated15, dark integrated16, monotonic grid14 (1481 widths each direction), memory10 browser checks pass with official Tauri mockIPC. Document backend72 and original runtime14 are partition evidence; final038 selected-save4 and observation21 pass with preserved raw logs. Native038 build is source-stable, exit0, exe38505e2a04cc129d0b205ba5bf415d059515b6997bd5fb5704317646b39ad93e; isolated identifier editor034/resources-empty, not native GUI acceptance. Source manifest remained unchanged on resume.

ED034-CODE-001 timeout observation and ED034-CODE-002 recovery observation were remediated. Final independent editor-delivery-review-038.json PASS (SHA256 327f03aaa9215bb0eebafde615eb4be7050382ad373f40864ac34740181a6319), no new findings, all 39 source and 19 evidence hashes fresh. This is the bounded code-delivery review, not Human/Task acceptance. Additional broad runtime diagnostics had four real-worker failures and are explicitly FAIL, with fixed-directory/fixture provenance only an inference; they are not passing full-suite evidence. Native WebView interaction and Human acceptance remain NOT_RUN/PENDING. No Task/Gate/commit/push/release transition.

## Subscription link/input candidate 040 (2026-09-06)

Latest user amendment implemented: 编辑信息 prepopulates editable full remote URL through existing explicit share-url IPC; only a changed URL sends urlReplacement. Backend independently compares persisted URL, same URL causes zero fetches, changed URL fetches once; errors retain old source/document/metadata. Local new/import source uses mutually exclusive text/file routes, switching clears stale content and invalidates pending reads; styled file button follows reference profile/file-input.tsx using existing theme. All textarea resize is disabled; global capture contextmenu prevents native menu while preserving app card context menu.

LINK039-CODE-001 found a same-file retry issue and is FIXED by resetting the input value after obtaining File. Frozen target040 ae1442a9730743aa4513b751b2da3d3793ca8d097f071a2fb083616f2506737b; independent link-input-review-040 PASS e1a18215fbcacfe079a42086153033d15557f9861869da3f652b5732c20118be, no new findings. Frontend97/lint, browser main17/race5/retry4, backend focused1/manager34/fmt/clippy pass. Native040 source-stable build passes (includes frontend build), exe002645a3fddc13ab7e70b2245fc6371deaadc47001b3417ac6a6e6463e59f602. Final light proof is link-input-light-final-040.png; the light-040 filename inherited dark media and is not light proof. Browser evidence uses mockIPC; native GUI/Human remain NOT_RUN/PENDING. No dependency/API/schema/permission changes, no core capability retest, no Task/Gate/commit/push/release transition. Receipt: link-input-receipt-040.json.

## UI foundation drift remediation 041 (2026-09-06)

After later visual-foundation styling landed, independent incremental review found TASK011-DRIFT-001: the subscription grid had lost its frozen 300px cap and monotonic resize behavior, cards had fallen from 128px to 100px, and empty metadata placeholders were hidden. The bounded CSS remediation restores those three TASK-011 rules without changing React, state, IPC, runtime, Router, Settings, or future Tasks.

Fresh evidence `ui-foundation-regression-041.json` records frontend99/lint, browser input26, bidirectional per-pixel geometry2962, four metadata variants, and the isolated Tauri debug build as PASS. Independent CHILD_AGENT rereview marks TASK011-DRIFT-001 FIXED with no new findings. Browser checks remain official Tauri mockIPC evidence; native WebView interaction is NOT_RUN and Human acceptance remains PENDING. No Delivery Gate, Task status, commit, push, release, or focus transition is made.
