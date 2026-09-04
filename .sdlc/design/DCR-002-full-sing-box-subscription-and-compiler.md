---
id: DCR-002
status: ACCEPTED
change_source: USER:lifei 2026-09-04 confirmed all sing-box user proxy protocols, Clash/sing-box/URI/paste import, and complete runnable sing-box configuration
approval_ref: USER:lifei 2026-09-04 scope confirmation
approved_target_identity: sha256:1a02b56d8861bb7c936599d952c822fa99667e8109d6b9b95427cb49e1966989
human_gate_ref: .sdlc/evidence/foundation/technical-design-review.yaml#EVIDENCE-DESIGN-024
affected_design:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
affected_task:
  - TASK-002
  - TASK-003
  - TASK-006
  - TASK-007
  - TASK-008
  - TASK-009
---

# DCR-002：完整 sing-box 协议、订阅归一化与可运行配置编译

## Problem

冻结基线只实现了部分节点协议、有限 URI/订阅字段和 `outbounds + route.rules` 语义快照。它不能满足
“只使用 sing-box、覆盖其用户代理节点协议、接收 Clash/sing-box/URI/粘贴导入，并生成可直接运行的
完整配置”的已确认产品语义。

## Current Frozen Design

`.sdlc/design/foundation.md` 要求订阅先归一化为不依赖 sing-box JSON 的领域模型；
`DCR-001` 固定当前可执行内核为 Windows amd64 sing-box `1.14.0`，并禁止任意二进制、特权 bridge、
TUN/UAC、原始 JSON 透传和额外监听器。该边界继续有效。

## Proposed Change

1. 将 sing-box `1.14.0` 的 16 种用户代理节点协议列为强制支持集：SOCKS、HTTP、Shadowsocks、
   VMess、VLESS、Trojan、WireGuard、Hysteria、Hysteria2、TUIC、ShadowTLS、SSH、Naive、Tor、
   AnyTLS、Snell。
2. 扩展领域模型为协议专有的强类型选项、TLS 和 Transport。存储版本提升为 V3：`StoredStateV2` 经
   显式 V2→V3 转换后才可反序列化当前 `AppState`；未知或不兼容的订阅字段逐项拒绝并报告，而非保存
   原始 JSON 或静默删改。
3. 统一远程、URI 与粘贴输入：仅从 Clash `proxies` 和 sing-box `outbounds` 提取上述节点，不继承源配置
   的路由、DNS、TUN、API、规则组、脚本或实验字段。解析可返回逐项诊断，但提交必须是全有或全无。
4. 引入内部 `SingBoxPlan` 和观察专用 `RuntimeProfile`；最终 `GeneratedConfig` 在固定 runtime secret/resource
   绑定后必须包含完整 `log`、`dns`、`inbounds`、`outbounds`、`route.final` 及受管 Clash API 结构，
   并以其最终字节运行 `sing-box check`。
5. `direct`、`block`、`selector`、`urltest` 只由编译器生成；`dns` 不是节点；`bridge` 不进入订阅或
   节点模型。Tor 只可使用后续冻结的受控 Tor 资源目录，禁止用户路径、参数、torrc 和数据目录。

## Frozen Data, Import and Profile Contract

### V2 → V3 migration

- V3 的 `ProxyNode` 采用 `ProtocolOptions` 及扩展后的 TLS/Transport；V2 的 `NodeCredentials`、
  `TlsOptions` 和 `Transport` 不是在反序列化时隐式猜测，而是在迁移步骤逐字段转换：
  `Shadowsocks`、`VMess`、`VLESS`、`Trojan`、`Hysteria2`、`TUIC`、`AnyTLS` 迁入对应 V3 options；
  `Socks5` 迁为 `Socks { version: 5 }`；`Http`/`Https` 迁为同一 HTTP 协议及明确 TLS 开关；V2 TCP、
  WebSocket、gRPC、Reality 和缺省 TLS 字段迁为语义相同的 V3 值。
- 每个成功迁移节点保留 V2 中已存储的 `NodeId` 字节完全不变，故现有 Pool 成员、手工选择和路由引用
  不变。新导入节点使用版本化 canonical identity material；显示名称、未知源字段和可选字段的默认展开
  不进入 identity。V2 中任一无法映射、缺失必需值、重复 ID 或迁移后整体引用校验失败，都使整个 V2→V3
  迁移失败，保留原文件和当前内存/运行实例，不写入半迁移状态。
- `JsonStateStore` 沿用既有顺序：读 V2 → 创建升级前备份 → 内存中完成 V3 转换、反序列化和整体校验 →
  原子写 V3。备份同样必须可独立迁移与校验；没有有效 V2/V3 状态时显式失败，绝不重置或删除状态。
- V2 没有可证明的默认路由目标。迁移将 V3 `default_target` 置为 `Unconfigured`，而不是猜测首节点、
  Pool 或 Direct；任何会承载用户流量的 Profile 必须因此拒绝编译，直至用户在版本化状态中明确选择
  Pool、Direct 或 Block。

### Import transaction

- 文档层失败（空输入、格式/编码无效、缺少 `proxies`/`outbounds`）直接为 `BatchRejected`；不会产生替换
  候选。
- 条目层的未知协议、未知字段、缺少必需字段、非法字段组合、Tor 资源未就绪或协议版本不兼容均写入不含
  凭证的 `SkippedNode` 诊断。诊断可供预览，但只要存在任一 skipped 项、归一化失败、稳定 ID 冲突或
  Provider/Pool/Route 引用校验失败，远程刷新、URI 导入和粘贴导入都必须拒绝整批提交。
- 只有至少一个节点、零 skipped 项、归一化完成且候选 `AppState` 整体校验通过时，才可通过既有原子
  save/交换事务替换该 Provider 的节点。失败始终保留旧 Provider 节点、持久化状态和当前运行配置。

### RuntimeProfile allowlist

| Profile | `inbounds` | `route.final` | 受管 API 与禁止项 |
| --- | --- | --- | --- |
| `ObservationOnly` | 显式空数组；不创建 mixed、TUN 或其它用户流量 listener | 必须为已启用 Pool；无 Pool/默认值时拒绝启动 | 仅一个私有 secret 的 `127.0.0.1:9090` Clash API；无 Dashboard、远程 API、文件日志或原始日志导出。 |
| `SystemProxy` / `Tun` / `NativeProfile` | 不属于本 DCR 的受管 Compiler 输出 | 不适用 | SystemProxy mixed inbound 需要单独 DCR 冻结 host/port、所有权、启停和验证；TUN/NativeProfile 保持既有专项 ADR/受限 Profile 边界。 |

所有 Profile 的最终 JSON 仅允许 `log`、`dns`、`inbounds`、`outbounds`、`route` 和必要的
`experimental.clash_api` 顶层键；已批准 DCR-003 为 WireGuard 增加唯一受控 `endpoints` 例外，
14 种其它非 Tor 节点继续生成 outbound。精确字段、强制入站拒绝和虚拟地址 ICMP 例外以
`.sdlc/design/DCR-003-fixed-core-configuration.md` 为准。`dns` 只允许由 Domain 的非持久化
`DnsPolicy` 产生；节点服务器域名使用系统 DNS，URLTest 目的域名采用核心/成员协议的原生解析行为。
`log` 不允许配置文件路径。
secret 在私有 ACL 已验证的 config 写入前绑定；结构 allowlist 在绑定后、`sing-box check` 前执行，
`check` 后不再修改字节。

## Reason

用户明确要求协议能力不再按当前最小子集分期，且要求 Compiler 的产物是可运行配置而非语义片段。
sing-box 1.14.0 的 outbounds 同时包含内部/特权类型；因此必须区分用户代理节点和运行时内部出站，避免
将 `bridge` 或原始配置入口带入当前普通用户安全边界。

## Affected Task / Gates

- TASK-002、TASK-003 已完成的交付与证据仍准确描述其原始子集，但不再证明扩展协议、迁移或完整配置。
- TASK-006 保持其已冻结的受管观测范围，不加入协议实现；其待决人工验收不自动通过或取消。
- Foundation 技术设计 Gate 及后续规划对该 concern 失效，必须完成本 DCR 的独立技术设计审查与 Human
  Technical Design Gate 后，才可物化并实施 TASK-007 及之后的任务。
- GUI/Tray 联合 E2E 继续后置至 TASK-010，且依赖完整配置的运行时验收。

## Migration / Compatibility / Risk

- 持久化：V2→V3 显式迁移的身份、默认路由和失败语义见上；不能以 enum 反序列化失败替代迁移。
- 导入：逐项诊断不是部分替换授权。任何 skipped 项或后续归一化/引用失败都保留旧 Provider 节点和
  当前运行配置；报告不得泄露凭证。
- Tor：当前 DCR-001 仅冻结 sing-box 资源。实现 Tor 前必须另行冻结上游发行版、许可证、Windows amd64
  archive/hash、成员集、私有 data directory、停止/清理及真实 E2E；在此之前 Tor fixture 的状态为
  `BLOCKED`，不得伪装为已支持。
- 完整配置：本 DCR 的受管 Profile 继续拒绝 System Proxy、TUN、UAC、WFP、Service、bridge、TLS spoof、任意
  listener、远程 API 和原始配置透传。编译失败或 `check` 失败必须保留已验证运行实例。

## Verification

- 协议 × 输入格式矩阵：15 种非 Tor 协议对每种可表达的 Clash、sing-box、URI 与粘贴入口各有正反
  fixture；无稳定 URI 的组合显式标记 `N/A` 并测试其 Clash/sing-box 表达。Tor 整行保持 `BLOCKED`，
  直至资源 Gate 通过。断言归一化、无凭证诊断和无 raw JSON 保留。
- 文档层失败、任一 skipped 条目、ID 冲突和引用失败都验证为不替换 Provider；等价远程和粘贴内容产生
  相同节点、稳定身份、Pool 成员和配置。
- 每一协议的最终 `GeneratedConfig` 通过固定 `sing-box 1.14.0 check`；完整配置结构、固定 API、
  无危险键与无原始 JSON 透传都需独立断言。
- V2→V3 映射表、V2 NodeId 保留、`Unconfigured` 默认路由、升级前备份、迁移失败恢复和原子提交分别验证；
  编译/check 失败保持当前实例；start/Ready 失败停止并清理候选，不自动恢复旧配置，记录脱敏日志与
  Toast，Windows amd64 受管运行路径按 DCR-004 单独验证。
- Tor 只有在其独立资源决策、资产完整性和真实运行证据均通过后才可标记为 `PASS`。

## Decision

ACCEPTED. USER:lifei approved the exact independently reviewed candidate
`sha256:1a02b56d8861bb7c936599d952c822fa99667e8109d6b9b95427cb49e1966989` on 2026-09-04.
