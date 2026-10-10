**2026-10-10 历史移植说明**：本文件保留旧独立树交付及当时计数/结果；旧 ignored 原始证据仍只在 `/Users/lifeilin/.codex/worktrees/p2-05-outbound-clients/veyra/docs/openbox-rust-gpui-tasks/evidence/p2-05/production-clients/`。最新 main 整合及当前状态以 [P2-05 整合验收](P2-05-integration-acceptance.md) 为准，旧 PASS 不替代本候选复跑。

# P2-05 正式出站客户端交付

**P2-05 Native修复（2026-10-09）**：已定位自有fixture的macOS accepted socket继承O_NONBLOCK；显式blocking后同目标mixed712ms/指定NodeId705ms、双向1707/4081TLSbytes，正式Native1PASS与清理PASS。生产客户端未改，旧503保留；物理Direct UNVERIFIED/SystemProxy ON NOT_RUN，用户已明确选择“保持 ACCEPTANCE，等待完整网络组合验收”；继续ACCEPTANCE/owner保留。Host e530837；无新commit/merge/push；P2-06 DOING及原owner不变。[根因、实际证据与验收取舍](P2-05-native-fix.md)。下方历史不改写。

以下为e530837之前的交付历史，原失败与当时状态原样保留。

2026-10-09：**ACCEPTANCE**，owner `Codex Desktop · Core 网络客户端` 保留。正式服务与适用定向检查完成；物理 Direct/SystemProxy ON 未证实，Native 节点成功证据缺失，不提前 DONE。

输入主分支 `codex/dist-react-restore` / `fa4b655af692fe0a1d7f06da60ef12e01bdd8295`；通过 Codex Desktop 新建并登记托管工作树 `/Users/lifeilin/.codex/worktrees/p2-05-outbound-clients/veyra`，分支 `dev/p2-05-outbound-clients`。未使用历史工作树、未更改主树、未 push。commit SHA **NOT_CREATED**；merge SHA **NOT_MERGED**；主分支复验 **NOT_RUN**，因为全部验收条件尚未满足。

## 实现及 owner 边界

- P0-06 的 outbound/bootstrap 原型进入正式 Core；订阅管理的 Preview/Save/Refresh/Scheduler 消费同一个显式客户端。Direct 不继承环境/系统代理，独立 TLS DoH 固定 `cloudflare-dns.com→1.1.1.1`，无系统 DNS fallback。DNS 状态、空/畸形响应、FakeIP、TLS/证书、timeout、连接拒绝、HTTP 状态等保留有类型原因，普通错误不携带秘密 URL。
- Direct 对实际目标只读查 macOS route；loopback 可用，utun/未知路径返回 `DirectPathUnverified`。此检查不证明非 utun 就是物理直连；当前未实现受管 TUN 绕行。bootstrap 当前为 IPv4 A 查询，IPv6-only bootstrap 与非 macOS 公网 Direct 未宣称支持。
- ViaRunningProxy 消费 Runtime owner 发布的可撤销只读能力：完整当前 InstanceId、实际 loopback mixed、内部鉴权 controller、**已应用** NodeId→tag 索引。不能用旧端口/last_successful 重建能力，不能改成 Direct，也不启动候选实例。请求固定实例；撤销取消正在进行的请求。
- 订阅复用已有有界重定向（最多5次）、总预算和4MiB响应限制；首导无需运行代理，刷新失败保留旧节点/时间/磁盘内容。含 URL userinfo 的来源拒绝；HTTPS 不能降级 HTTP；跨源移除认证、Cookie、自定义来源头和缓存 validators。TLS 关闭选项明确拒绝，Legacy System 策略明确不可用，不采用第三方代理或端口猜测。
- `NetworkService` 提供 site_latency、node_latency、exit_ip、geo_ip，调用方显式选路径。节点只针对当前已应用 NodeId，通过当前 controller 封闭 GET；URL/tag 编码、有界超时/响应、不跟随 controller 重定向。controller节点零值为未知，失败保留类型原因；site实测耗时不足1ms可返回Some(0)，不将失败写成零延迟。仅允许 HTTPS 节点目标，因为固定 sing-box 会将 HTTP 目标换成默认 HTTPS，不可把另一目标结果归给用户指定目标；站点测速仍可明确请求 HTTP/HTTPS。
- 三种 provider 沿用现有 React/Chain helpers 字段：ip.sb / ipwho.is / ipapi.is；请求只有目标 IP，不带订阅秘密。兼容真实 ipapi.is free-tier flat ASN/company；缺 country_code 等保持 None，业务错误/错目标响应明确拒绝。
- 与 P2-06 原 owner 协调并复核，仅增加 ManualRuntime 只读能力与 Desktop composition、封闭 node_delay；公共契约 ownership 不转移。Ready 发布，候选 preflight 失败保留旧能力；Replace/Stop/Handoff/已发现退出/Recovering/Drop 先撤销。实际 App Quit 回调先取消订阅/网络，再等待 Runtime。Helper 无等效能力则不可用，Helper/IPC/Platform 没有改动；不新增 WS 采集连接。
- Legacy Tauri 因 Core setter 退役删除两处旧 port/System reader 注入；其 ObservationOnly 路径不能冒充正式受管能力。不提前开发 UI 页面。

## 验证结果

本地证据目录：[`evidence/p2-05/production-clients/`](evidence/p2-05/production-clients/README.md)，ignored 本机证据保留；本文件为可跟踪交付摘要。

| 最终验证 | 结果 | 测试保护的 Veyra 行为 / 收据 |
| --- | --- | --- |
| Core subscription | 72 PASS | 生产HTTP/HTTPS、DNS fixture、凭据/redirect、proxy无fallback/撤销；`core-subscription-closeout` |
| Core subscription_management | 46 PASS | Preview/Save/首导/刷新/条件缓存/失败保旧/取消/版本；`core-services-closeout` |
| Core NetworkService | 4 PASS | 三schema/未知、loopback controller编码/认证/302/零值、指定Direct/Via和关闭取消；`core-network-closeout` |
| Core ManualRuntime | 36 PASS | Ready、换实例、Stop/异常、handoff及能力撤销；`core-runtime-closeout` |
| Core controller | 16 PASS | 固定鉴权loopback/WS与既有controller边界；`controller-final` |
| Desktop subscriptions / RuntimeService | 4 + 5 PASS，另2 ignored | 正式桥接、失败/旧结果与退出；`desktop-subscriptions-final`、`desktop-runtime-final` |
| 定向合计 | **183 PASS** | Core174 + Desktop9，互不重复；2 ignored不计通过 |
| 正式三包 all-targets Clippy / build | PASS | `workspace-clippy-post-fixture`、`workspace-build-final` |
| fmt（workspace及指定src-tauri manifest） | PASS | `workspace-fmt-final`、`legacy-fmt-final` |
| P0 example feature 编译 | PASS | `p0-example-final`，不把原型编译当正式网络验收 |
| Legacy Tauri lib Clippy | **FAIL** | `legacy-clippy-final`：既有资源 `binaries/sing-box-1.14.0-windows-amd64/libcronet.dll` 缺失；不修改无关资源/配置绕过 |
| Native 正式服务聚合 | **0 PASS / 1 FAIL** | `native-relay-final`；节点503，不能将整个测试标PASS |

Native 使用固定 sing-box **1.14.0 darwin arm64**，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，独立 AppServices/Store/root，当前实例真实 Ready/mixed。正式 loopback 首导 Preview→Save 与 HTTP503 Refresh 保留同一磁盘 bytes 已验证。当前 Via 的三种 provider 均成功回读 `8.8.8.8` / ASN15169，ipapi.is 缺 country_code 保持未知。实际出口如下，仅表示经受管 mixed 后的观测出口，不证明物理 Direct：

| provider | Native 出口 | site / 身份 |
| --- | --- | --- |
| ip.sb | `2a09:bac5:624d:78::c:3c2` / ASN13335 | Cloudflare Warp（provider实际报告） |
| ipwho.is | `123.121.111.183` / ASN4808 | provider实际响应 |
| ipapi.is | `123.121.111.183` / ASN4808 | free-tier flat response，缺失值不补造 |
| site | `https://www.gstatic.com/generate_204`，**806ms** | 相同当前 InstanceId，ViaRunningProxy |

Native node：正式 NodeId→applied tag→当前 controller→自有 HTTP node listener 收到 `CONNECT www.gstatic.com:443`。系统 DNS fixture 上游曾连到 `198.20.1.39`，无 TLS 响应；改为固定 `node→owned HTTP fixture→同一current mixed(default Direct)→既有外部TUN` 拓扑亦 CONNECT200/转发0bytes/controller503。没有自动fallback；不能确定503的最终原因，不能记录独立远端节点或物理Direct成功。聚合断言失败前已完成 Replace，旧能力撤销，新实例退出，两个实例 mixed/controller 不可连接、network Closed、自有fixture线程join、services drop/root删除；另已盘点并删除首轮失败遗留的1个自有隔离root，文件摘要留存cleanup.json，失败日志保留；清理局部 PASS 不提升节点/聚合结果。

当前 IPv4/IPv6 default 均为受保护的外部 **utun4**，系统代理 OFF；Native Direct 明确返回 `DirectPathUnverified`。`network-final-state-before/after.json` 对比：utun配置/index、route/default/table、DNS、系统代理、NetworkServices/Sets/CurrentSet摘要及外部owner PID/start/executable身份不变。未关闭/重启/接管/更改系统代理或TUN，没有管理员操作。

历史证据保持：原 Core 两项 baseline FAIL、Desktop 全量 Store Busy、未完成 Native 不重写。此次初始编译/断言/fixture FAIL均保留；`native-production-repaired`/`native-schema`/`native-final`早轮exit0没有node成功聚合断言，不能作为整体验收PASS。`native-node-diagnostic`为0通过/1ignored，无Native验证价值。`network-before.json`早轮输出名与runner收据重名，原完整state快照已丢失，不能当state证据；后续独立命名state-before/after才用于对比。

## 独立 Review 与下一动作

独立 readonly Reviewer `/root/review_p205` 对实际最终代码与收据审查，核验路径固定、秘密/跨源隔离、当前实例/appliedNode身份、失败保旧/撤销及183计数。Bootstrap错误被吞成DnsFailed的 finding 已修复并经精确TLS/DNS类型fixture复核关闭；最终未发现剩余可操作代码缺陷。原 P2-06 owner 另外审查最小接线，ProxySource可写绕过撤销、实际Quit取消时序两项 finding 已修复/关闭。二者不是新的独立Native复跑，也不代替未完成网络出口验收。

继续本卡需在保护既有用户网络的前提下证实物理Direct/SystemProxy ON组合，以及取得指定节点Native成功结果；需要改变用户网络/管理员权限时先获得用户授权，不自行切TUN。受管TUN绕行由P6按生产实现承接，Helper capability仍待P2-06原owner；Windows后续单列。当前不因这些缺口扩大Runtime/Helper/IPC写范围。

状态为 ACCEPTANCE、owner保留；只有实际全部验收满足后才 DONE、释放owner、独立commit/合并主开发分支并主分支复验。DAG保持68卡：DONE26 / DOING1 / ACCEPTANCE1 / READY3 / TODO30 / DEFERRED7；READY仅P0-08/P3-03/P4-03，P2-07/P2-08及其它下游未解锁、不启动。

固定内核目标行为依据：[1.14.0 controller](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/experimental/clashapi/proxies.go) 与 [URLTest](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/common/urltest/urltest.go)；provider文档：[ip.sb](https://ip.sb/api/)、[ipwho.is](https://ipwhois.io/documentation)、[ipapi.is](https://ipapi.is/developers.html)。

## Host 独立检查与 ACCEPTANCE 检查点（2026-10-09）

Host 阅读正式 OutboundClient/NetworkService、Runtime/Subscription 桥接、Legacy 兼容范围及 Native 原始收据，独立受管复跑七组无网络副作用的定向测试：Core subscription **72 PASS**、subscription management **46 PASS**、NetworkService **4 PASS**、ManualRuntime **36 PASS**、Clash controller **16 PASS**、Desktop subscriptions **4 PASS**、Desktop runtime **5 PASS/2 ignored**，合计 **183 唯一 PASS/2 ignored**；Core/Desktop/Helper all-targets Clippy -D warnings、三包 build、workspace fmt、git diff --check 全部 PASS。未重跑真实 Native 503 和用户现有网络操作。

Host 复核 Native node 失败收据：Controller 返回 **HttpStatus(503)**，自有节点 HTTP proxy 已收到 `CONNECT www.gstatic.com:443`，后续测试记录转发0字节。只能证明测试节点入口被触达，不能断定 Veyra、fixture 或外部网络哪一侧为根因。ip.sb/ipwho.is/ipapi.is 及 site 成功不提升 Native 聚合 0PASS/1FAIL。物理 Direct/SystemProxy ON 在受保护 utun4 下未验证，且不得自行改变用户网络。旧 Tauri Windows libcronet.dll 缺失和历史 Core/Desktop 回归不改写。

本轮仅为完成代码与可审查测试的 **ACCEPTANCE 检查点**；P2-05 owner 继续保留，**不标 DONE、不合并 codex/dist-react-restore、不 push**；P2-06 继续 DOING。剩余：受保护环境内定位节点503并取得指定节点实测成功；物理Direct/SystemProxy ON需要安全可验证环境或用户明确范围决定。READY 仍 P0-08/P3-03/P4-03。
