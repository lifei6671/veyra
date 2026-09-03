---
id: DCR-001
status: ACCEPTED
change_source: USER:lifei 2026-09-03
approval_ref: USER:lifei 2026-09-03 DCR-001 exact 1.14.0 runtime contract
amendment_ref: USER:lifei 2026-09-03 sha2 0.10.9 offline asset-integrity verification
secret_rng_amendment_ref: USER:lifei 2026-09-03 getrandom 0.4.3 per-instance API secret generation
multi_version_amendment_ref: USER:lifei 2026-09-03 selectable 1.12/1.13/1.14 core family
acl_feature_amendment_ref: USER:lifei 2026-09-03 Windows ACL implementation feature expansion
multi_version_gate_approval_ref: USER:lifei 2026-09-03 Human Technical Design Gate approved
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

该文件已下载、解压并完成 archive/executable SHA-256 readback；尚未作为应用资源打包或启动服务。

## 决定

用户已批准本 DCR 的固定 `1.14.0` Windows amd64 asset、SHA-256、私有 loopback API、direct `reqwest`
依赖和不改写系统网络设置的真实 E2E 边界。用户随后批准 `sha2 = "=0.10.9"`、关闭 default features，
其用途仅限离线计算内置 archive 与 extracted executable 的 SHA-256。用户还批准 `getrandom = "=0.4.3"`，
用途仅限从 Windows 系统熵生成每实例 32-byte API secret，不得作为通用随机、标识、网络或前端输入来源。
用户还确认使用现有 `windows = "=0.61.3"` 的 `Win32_Security_Authorization`、
`Win32_System_Threading` 与 `Win32_System_Memory` feature 实现并回读当前用户专属 ACL。该 feature
扩展不增加 crate 或版本，只限私有运行目录和 config 的 ACL；不得用于提权、服务、WFP 或系统网络设置。

这些批准不放开任意下载、任意二进制、Shell、前端或系统权限。

## 影响分析

| 维度 | 影响 | 处置 |
| --- | --- | --- |
| Requirement / 产品语义 | 有变化：用户可选择已支持的 1.12/1.13/1.14 内核 | 当前仅落定可扩展目录和 1.14.0 开发/E2E；旧版本各自的资产与兼容性留待后续受控 Task。 |
| 冻结设计 | 有变化：单一资产变为版本目录与兼容 profile；新增 ACL 绑定 feature | 重开技术设计 Gate；Foundation 与 TASK-006 只保留 1.14.0 的当前实施范围。 |
| 安全与运行边界 | 1.14.0 新增独立 API service、Dashboard、远程控制及更多特权网络能力 | 生成配置必须不启用 `api` service、Dashboard、远程管理、TUN、bridge、TLS spoof、USB/IP 或任何新监听器；仅保留已批准的 loopback Clash API。 |
| 实现 | 尚无真实 sidecar/API adapter，因而没有已实现代码需要迁移 | TASK-006 按更新后的版本新建固定 adapter、完整性校验与安全 DTO 桥接。 |
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

## 备选方案与决定

1. 运行时接受任意内核、路径或 URL：拒绝，无法证明来源、hash、配置兼容性或 child 归属。
2. 将 1.12/1.13 现在标记为支持：拒绝，尚无对应资产和真实兼容证据。
3. 以受控版本目录演进，当前只执行 1.14.0：采用；保留产品演进空间而不稀释当前安全和验证边界。

## 保持不变的实施契约

- 后端独占固定 `127.0.0.1:9090` Clash API；每次 sidecar 启动生成 32-byte secret，只存在于私有运行期
  配置与短生命周期内存，绝不进入日志、事件、UI 或持久化状态。
- 新增直接依赖只能是 `reqwest = "=0.12.28"`（关闭默认 feature，仅启用 `json` 与 `rustls-tls`）和
  `sha2 = "=0.10.9"`（关闭 default features）；前者用途限于后端类型化 loopback API client，后者只
  计算内置 archive 与 extracted executable 的 SHA-256。`getrandom = "=0.4.3"` 只为每个受管实例
  生成一次 32-byte API secret；secret 仍只存在于私有 runtime config 与短生命周期内存。
- 真实 E2E 必须证明主程序与 child sidecar 均不出现控制台窗口，且不写 System Proxy、不启用 TUN、不请求
  UAC、不创建 WFP/Service 或非 loopback listener。

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

每个受管实例使用应用私有运行时目录；目录及生成 config 仅允许当前 Windows 用户访问，创建后必须回读 ACL。
config 的唯一 secret 是该实例生成的 32-byte 值，不得复用或持久化。清理是幂等的：

- 正常 stop：先停止并确认本应用 child 已退出，再擦除内存 secret 与删除实例 config；
- check/start 失败：候选 child 不得可用，立即执行同一清理并确认 config 不存在；
- child 崩溃：Supervisor 记录封闭失败状态、擦除内存 secret 并删除对应 config；
- 主程序异常退出：下次启动仅清理由本应用命名、归属可证明且 ACL 符合预期的遗留实例目录；无法证明归属或
  无法删除时停止进入真实运行态并报告不含路径/secret 的恢复失败。

验证必须覆盖 ACL readback、上述四条清理路径及 secret 不出现在私有运行期 config 之外的文件、
`state.json`、备份、日志、错误、DTO、事件或 UI 的断言。

## 批准与失效

此前针对 `1.12.13` archive/hash 的资产确认未被自动迁移至 `1.14.0`；用户已对本 DCR 的精确 URL、
SHA-256、amd64-only 范围、HTTP 依赖与真实 E2E 边界作出批准。批准后的强制顺序为：

1. 将 `.sdlc/state.yaml` 的 `technical_design` Gate 重开为 `PENDING`，清除旧 Foundation identity、reviewer、
   approval 与 evidence；TASK-005 的独立 Delivery Gate 不受影响；
2. 修订 `.sdlc/design/foundation.md` 的受管内核版本，并以 DCR-001 更新 TASK-006 的资产契约和
   `approval_refs`；TASK-006 的独立验收从“hash/签名”收紧为“官方 URL + 固定 SHA-256”，不声称未配置
   信任根的签名校验；
3. 以新 Foundation identity 重新进行独立技术设计审查并取得新的人类 Gate 批准；
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
