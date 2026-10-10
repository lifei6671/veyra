# P2-05 最新主分支整合验收（2026-10-10）

**P2-05 本轮最终实机结果（2026-10-10，ACCEPTANCE）**：本候选唯一指定 Native 1 PASS（exit0），定向复跑 Core50 PASS、Desktop build/all-targets Clippy/fmt exit0；真实订阅添加/刷新/失败保旧/保存草稿重试/重启/语言草稿、主备Controller选择、P3日志、同PID托盘恢复与正常退出通过。独立Review及完整收据见[本轮实机验收](P2-05-desktop-acceptance.md)。当前默认macOS显示配置经用户确认，非额外150%设置；物理Direct UNVERIFIED、SystemProxy ON NOT_RUN、历史HTTP503和Legacy Clippy FAIL101保留。后续P2-07/P2-09/P6-01/P6-05组合单列，旧等待组合决定未取消，Host收口前保持ACCEPTANCE/owner，无commit/merge/push/下游。68卡DONE30/ACCEPTANCE1/DOING1/READY0/TODO29/DEFERRED7，P2-06 DOING/Runtime owner不变。

**以下为工程整合结束时的快照，其中 Native/GUI/Review 未执行描述只属于当时；本候选后续实机结果见上方新记录。**

**工程整合完成；P2-05 保持 ACCEPTANCE。** 实现者自查未发现剩余可操作代码 Finding。独立只读 Review 和本候选 Native/GUI 尚未执行；旧 183 PASS、旧 Native 修复后 1 PASS 不替代本候选实测。未 commit/merge/push，没有解锁下游。

## 身份与授权边界

- 唯一写树：`/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/.worktrees/p2-05-main-integration`，`dev/p2-05-main-integration`，输入/当前 HEAD `bda8be5c469dcbd6093a28fd4fd549be70c22194`；本次变更未提交。Host 后续负责 Task commit 与 merge。
- 旧逻辑检查点 `e530837976257d489d3bf455d5d96e63e704f442`，旧基线 `fa4b655af692fe0a1d7f06da60ef12e01bdd8295`。原托管树 `/Users/lifeilin/.codex/worktrees/p2-05-outbound-clients/veyra` 的五份 dirty tracked 文件、未跟踪 Native 修复文档及 ignored 原始证据只读保留。主树未跟踪 `tools/tests/__pycache__/` 保留。
- 依据当前 AGENTS、[长期分期政策](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)、[任务卡](P2-local-proxy.md#obg-p2-05)、最新 SESSION/总表、[P0-09 阶段审计](P0-09-stage-review.md)。首版 macOS15+ 规划、现有固定内核 minOS26.0 与最低设备未验收事实不变。
- 用户此前明确“保持 ACCEPTANCE，等待完整网络组合验收”；本次“可以。继续吧。”只授权继续整合与适用验证，不把旧 NOT_RUN 改成 PASS，不自行更改该状态决定。

## 移植与 main 增量保护

逐文件读取 `fa4b655→e530837` 精确 diff；能够匹配的源码 patch 先 `git apply --check`，再仅写工作文件。五处有冲突的源码文件由实现者按当前内容增量合并，没有 checkout/覆盖重叠文件，没有操作 Git 元数据。

正式 OutboundClient/独立 bootstrap/fetch、订阅 Preview/Save/Refresh/Scheduler、NetworkService、三 GeoIP provider 归一化、封闭 controller NodeId 测速、Desktop composition 与真正 Quit 的网络取消从旧实现移植。生产路径固定 Direct 或当前可撤销 ViaRunningProxy，不自动换路、不读取历史端口、不启动下载专用候选，不添加 Runtime 写命令。

Runtime 最小接线契约延续旧 owner 已协调能力：`ManualRuntime` 发布/撤销 `ProxySource`，`RuntimeService`/`AppServices` 只读消费；Ready 后发布完整当前实例、实际 loopback mixed、已鉴权 controller 和 **实际 applied plan** 的 NodeId→tag。预检失败保留仍有效旧实例；Replace/Stop/handoff/失活/Recovering/Drop 撤销。选择和核对复用 main 的 `selection_child_alive` 撤销点。`ProxySource::publish/revoke` 与能力构造仍限 Core 内部。

保留 main 全部 Failover runner、Group 选择/pending/CAS、健康探测、只读 ObservationReader、唯一 collector/SQLite writer 和 Helper Group admission。没有修改 Helper/IPC/Platform、Runtime DTO、owner-transfer、remote-selection fence、恢复格式或 workspace/锁文件；P2-06 Runtime/DTO 单一 owner 不变。仅 Core 现有 tokio 增加旧实现需要的 `process` feature，用于有界只读路由查询，没有新增依赖。

旧 dirty Native 修复的完整 ignored 函数原样放回 `runtime_service::quit_tests::outbound_clients_real_production_services`；源码比对 `native-dirty-fixture-exact=true`，含 accepted socket 显式 blocking、有限超时、固定 HTTPS 目标、正式 NodeId 调用、实际配置 tag 核验、双向 TLS bytes>0、Replace/Quit 清理断言。main 自带 Native tests 全部保留。

本次强化既有 P4-03 测试的真实整合契约：Group 默认出口仍发布 applied 的 a/b/c 节点；选择不更换实例能力；Restart 撤销旧能力；Group reconcile 发现 child 退出/清理失败时同步撤销下载/测速能力，pending 和持久状态保留。

## 本轮验证与原始收据

本机 ignored 目录：[evidence/p2-05/main-integration](evidence/p2-05/main-integration/README.md)。每条 cargo 命令由复用旧证据 runner 外层 600 秒限时，`MACOSX_DEPLOYMENT_TARGET=15.0`，offline/locked，不更改系统设置。日志和 JSON 保存完整 argv、退出码和耗时；源码 manifest/测试映射另存该目录。

测试在生产 Store/Service、Mock SidecarPort、自有临时目录和 loopback HTTP/HTTPS/DNS/controller/WS 上验证 Veyra 契约。subscription 的隔离 child 仅重入自身测试可执行文件并设置该 child 环境；没有真实 sing-box child、管理员/Helper 安装、用户网络或 GUI 操作。

| 本轮套件 | PASS / FAIL / ignored | 退出码 | 保护的用户行为/自有契约 |
| --- | --- | --- | --- |
| Core subscription | 72 / 0 / 0 | 0 | Direct 显式路径、引导 DNS、TLS、重定向/凭据隔离、失败原因、预算/上限、代理撤销 |
| Core subscription_management | 46 / 0 / 0 | 0 | 首导/保存/刷新/条件缓存、磁盘 roundtrip、失败保旧、取消/CAS/epoch |
| Core NetworkService | 4 / 0 / 0 | 0 | 指定路径/目标、封闭 NodeId GET 编码/鉴权/零值未知、三 schema 和关闭取消 |
| Core ManualRuntime（最终） | 46 / 0 / 0 | 0 | 旧实例能力生命周期、P4-03 Group/pin/Auto/pending/失活、恢复/Bootstrap/Observation 归属 |
| Core Clash controller | 16 / 0 / 0 | 0 | 当前实例固定鉴权、既有读写/WS 和端点撤销边界 |
| Core Observation controller | 8 / 0 / 0 | 0 | 唯一四 WS、无订阅者重连、换源/Stop、日志/连接统计、SQLite 释放与失败传播 |
| Core Groups/policy | 20 / 0 / 0 | 0 | 实际 CRUD/重建、成员/图/Compiler、主备阈值/pin/恢复/过期探测 |
| Desktop subscriptions | 7 / 0 / 0 | 0 | 四项原页面 bridge 加 main 三项分享 UI 现有契约 |
| Desktop runtime_service | 6 / 0 / 4 | 0 | 五项旧 worker/Quit 加 main Helper Group admission；四项 Native/GUI fixture ignored 不计 PASS |
| **唯一合计** | **225 / 0** | 各实际命令均 0 | **Core212 + Desktop13**，重跑 ManualRuntime 不重复计数 |

旧 183 个名称逐项与新日志集合比对：**182 同名 PASS**；一个旧测试 `selection_with_unapplied_profile_preserves_applied_config` 已由 **输入 main** 替换为 `selection_rejects_unapplied_config_without_controller_write`。后者当前 PASS，保留 P4-03“配置已改但未应用时拒绝旧选择、不写 controller”的新契约，本次没有恢复旧允许行为或删除 main 测试。七组当前对应数量为 72+46+4+46+16+7+6=197，另 Groups20 + Observation8=225；不是把旧收据加到新测试数。

曾用 `application::failover::` 过滤运行 0 测试，exit0 **不计 PASS**，原收据 `core-failover.log/json` 保留。实际策略测试位于 `domain::groups::tests::`，已明确重跑 20 PASS。所有初始收据保留。

| 工程检查 | 结果 / 退出码 |
| --- | --- |
| Core/Desktop/Helper all-targets Clippy `-D warnings` | PASS / 0；测试断言增量后 Core all-targets 最终 Clippy 再验 PASS / 0 |
| 同三包 all-targets check/build | PASS / 0、PASS / 0 |
| P0 feature examples check（含 main P0-08） | PASS / 0，仅编译，不作正式网络/分发验收 |
| workspace fmt / 指定 src-tauri fmt | PASS / 0、PASS / 0 |
| Legacy `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | **FAIL / 101**：原 Windows `binaries/sing-box-1.14.0-windows-amd64/libcronet.dll` 缺失，未绕过资源配置 |
| 最终 diff/文档/保护核验 | PASS / 0；68卡状态/依赖/owner/估算不变、DAG无环；旧6份dirty文件与主树1份未跟踪文件hash不变，两个HEAD不变 |

## Native 与分期边界

本候选 **Native NOT_RUN**。只读前置快照中 `/bin/ps` 被沙箱拒绝；IPv4/IPv6 默认 route 查询均 exit71。不能完整核验外部进程身份及实际路径，因此未运行 pinned sing-box 或公网 Native，没有权限重试或改变用户环境。已读取 fixed binary SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，仅证明拟用输入身份。interfaces/routes/DNS/proxy/services 与系统配置 hash 的可读项留收据，不把不完整快照当完整保护验收。

旧原始证据保留在 **旧树** `docs/openbox-rust-gpui-tasks/evidence/p2-05/production-clients/`，未复制日志/二进制到新树。移植的 [历史交付](P2-05-acceptance.md) / [Native 修复](P2-05-native-fix.md) 内日期、旧状态和旧计数均是历史，当前以本文件及最新总表为准。旧 HTTP503 聚合 FAIL 和修复后指定 NodeId **1 PASS** 同时保留；后者只证明旧源码在 node→自有 HTTP fixture→同 current mixed→既有外部 TUN 的受控拓扑，不是物理 Direct 或独立远端节点证明。

| 未完成边界 | 本轮判断 | 既有承接范围 |
| --- | --- | --- |
| 新候选自身正式服务/NodeId Native、Replace/Quit 清理 | NOT_RUN，需 Host 在可核验环境复跑 | P2-05 当前整合复验；不得用旧 PASS 替代 |
| 物理 Direct、独立 DNS 的真实路径 | 旧 UNVERIFIED 保留；受外部 TUN 影响时正式客户端 `DirectPathUnverified` | P6-01 生产受管 bypass 与 P6-05 平台组合；专用无外部 TUN 环境补真实证明 |
| SystemProxy ON 的完整真实出口 | 旧 NOT_RUN 保留，当前正式 SystemProxy 任务未完成 | P2-07 实施；P2-09 全链路出口组合、P6-05 系统平台组合 |
| Helper 对等只读能力/root/IPC/系统网络 | 未新增或声明支持；没有能力则 ProxyUnavailable | P2-06 原 owner；不得自行发布另一能力或 Runtime 写入口 |
| 本构建实际订阅 UI、Quit、P4-03/日志共存检查 | NOT_RUN，自动 bridge/模型测试不代替实际 GUI | Host 的 Codex Desktop 局部复验；P2-08/P3-05 仍各自负责后续测速 UI，完整组合归既有 E2E 卡 |
| macOS15/Windows/正式包 | 未实测、不作兼容/发布结论 | P7-03/P6-05/P7-05 与既有 Windows 卡 |

**DONE 判断**：工程功能和局部受控自动验证已完成；本候选 Native/GUI 实测与独立 Review 未完成，且用户的“等待完整网络组合”决定尚未明确变更。因此没有足够依据自行标 DONE。状态仍 DONE30 / ACCEPTANCE1 / DOING1 / READY0 / TODO29 / DEFERRED7；P2-06 DOING 与单一 owner 不变。

分期政策允许把 SystemProxy 完整出口及 TUN 物理 Direct 验证分别留 P2-09/P6-01/05，却不代表这些能力已经 PASS。P2-07 本身显式依赖 P2-05 DONE，P2-09 又依赖 P2-07；不能给 P2-05 新加“P2-09 DONE”技术依赖，否则形成验收环。本次不改变 DAG，也不把用户旧决定静默解释为已撤回。后续如按本卡自身生产能力/局部验证收口，应在本候选补验后由 Host 明确处理旧等待决定及延期项归属；当前不抢跑下游。

## 实现者 Review

使用 `code-delivery-review`；这是实现者自查，**不是独立审查**。检查当前 main 的重叠 diff/新文件、能力发布/撤销/应用节点来源、Group/Failover/Observation/Helper ownership、订阅失败保旧/凭据/引导解析/取消、Desktop 真 Quit 时序和测试原始收据。

结果：**0 个剩余可操作代码 Finding**。对 main 的三个强耦合位置（ManualRuntime、RuntimeService、services）只增加最小生产接线；main 的新选择拒绝规则保留，原 Native 修复完整。其余限制为上表实际 NOT_RUN/UNVERIFIED 与旧 Legacy 基线 FAIL。独立 Reviewer 尚未执行，不能把这份自查当作必要的独立结论。

## Host 交给独立只读 Reviewer 的提示词

在 `/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/.worktrees/p2-05-main-integration` 只读审查 OBG-P2-05，基线 `bda8be5c469dcbd6093a28fd4fd549be70c22194`。先读 AGENTS、长期分期规则、最新 SESSION/总表、任务卡、本报告，再用 `code-delivery-review` 核对 diff 和所有未跟踪源码/文档，以及 ignored `evidence/p2-05/main-integration` 原始 log/json。旧树仅只读 `e530837` diff、dirty Native 函数及原证据。不得写任何树、Git、网络或运行 child/GUI。重点列具体代码 Finding：最新 P4-03 Group/Failover/Observation 的共存、选择失活撤销、applied NodeId/实例/端点身份、订阅凭据与失败保旧、真 Quit 取消、P2-06 单 owner/Helper/IPC/DTO 未越界；核实225唯一计数、旧测试被main新契约替换、零过滤不算PASS、Legacy101及Native NOT_RUN。输出严重度/file:line/触发条件/影响/最小修复；零Finding则说明实际审查边界。只读复核不等于真实 Native，勿改Task DONE或声称独立复跑。

## Host 交给 Codex Desktop 的真实 OS/GUI 提示词

仅验证同一 `.worktrees/p2-05-main-integration` 最终源码及未提交 diff，不写主树/旧树，不 commit/merge/push，不访问原版 OpenBox 或 openbox.disign.me。先固定源码清单/最终构建 SHA；固定上文 kernel SHA。检查当前进程/网络身份和是否有可安全拥有的临时资源；不得操作用户正在运行的 Veyra、外部 utun/SystemProxy/DNS/route，不管理员安装或更改系统网络。若核验或权限不能满足，逐项 NOT_RUN 并说明。

1. 在可完整核验进程与路由的环境，只运行已移植 opt-in Native：`env MACOSX_DEPLOYMENT_TARGET=15.0 VEYRA_SING_BOX_PATH=<核验后固定kernel绝对路径> cargo test --offline --locked -p veyra-desktop runtime_service::quit_tests::outbound_clients_real_production_services -- --ignored --exact --nocapture --test-threads=1`，使用既有600秒外层runner/收据，不跑其它ignored或全库真实OS测试。
2. 它只拥有新临时 AppServices/Store/root、loopback HTTP fixture 和普通用户实例。核验正式 Preview→Save/HTTP503保旧、当前 Ready mixed/完整 InstanceId、三provider、同固定 `https://www.gstatic.com/generate_204` 的 site 与正式 NodeId=a/controller/node-a、accepted socket blocking、双向TLS bytes>0、Replace撤销旧能力、Quit/端口/线程/root清理；失败收据和残留归属必须保留，不把局部成功抬为聚合PASS，不换目标或降低TLS。
3. Native前后对比所有可核验 utun/index、DNS、路由、SystemProxy/服务、外部进程身份。仅清理本次明确拥有的root/child/listener；不能根据端口扫描接管他人。若测试提前失败导致残留，按本次root/实例身份处理，不全局kill/clean。
4. 用本构建和自有隔离数据root打开实际 GPUI，保持用户显示设置；在既有订阅页实际首导本地fixture、失败保草稿/保旧数据、重载/重启、忙碌/错误/空、窗口隐藏与真Quit，核验换语言保草稿及准确反馈；检查 P4-03 出站主备页面和日志页仍可消费同一生产服务。只保存必要截图/日志，不把GUI当新设计或启动下游Task；需要真实child时限定同自有root/loopback。
5. 本轮不补 SystemProxy ON/TUN 物理Direct：这些需要后期生产能力/专用隔离机器及适用授权，继续原UNVERIFIED/NOT_RUN，并按P2-07/P2-09/P6-01/05归属记录。结束返回实际测试数/exit/截图身份/清理/未执行原因；由Host处理状态取舍，不能自行DONE。

## 修改文件清单

全部相对唯一新树；18份源码/Cargo文件和6份任务文档，共24份。ignored收据不在该清单，不提交二进制或日志。精确摘要另存 `source-manifest.json`。

```text
crates/veyra-core/Cargo.toml
crates/veyra-core/examples/p0_06_outbound.rs
crates/veyra-core/src/application/manual_runtime.rs
crates/veyra-core/src/application/manual_runtime/tests.rs
crates/veyra-core/src/application/mod.rs
crates/veyra-core/src/application/network.rs
crates/veyra-core/src/application/network/tests.rs
crates/veyra-core/src/application/subscription_management.rs
crates/veyra-core/src/application/subscription_management/preview.rs
crates/veyra-core/src/singbox/clash_api.rs
crates/veyra-core/src/subscription/fetch.rs
crates/veyra-core/src/subscription/mod.rs
crates/veyra-core/src/subscription/outbound.rs
crates/veyra-core/src/subscription/outbound/tests.rs
crates/veyra-desktop/src/app.rs
crates/veyra-desktop/src/runtime_service.rs
crates/veyra-desktop/src/services.rs
docs/openbox-rust-gpui-tasks/IMPLEMENTATION_PHASES.md
docs/openbox-rust-gpui-tasks/P2-05-acceptance.md
docs/openbox-rust-gpui-tasks/P2-05-integration-acceptance.md
docs/openbox-rust-gpui-tasks/P2-05-native-fix.md
docs/openbox-rust-gpui-tasks/P2-local-proxy.md
docs/openbox-rust-gpui-tasks/SESSION.md
src-tauri/src/lib.rs
```
