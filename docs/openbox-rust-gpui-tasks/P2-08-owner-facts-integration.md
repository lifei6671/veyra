# P2-08 owner 只读事实消费者集成

2026-10-10；原 WorkRun `work-23397-1791604235718790-269`；本卡 **DOING**，P2-06 DOING/原 owner、68 卡状态不变。唯一写树 `.worktrees/p2-08-proxy-latency` / `dev/p2-08-proxy-latency`；输入/当前 HEAD `6857b2e39633ca0564cac66b40ee0f9129cf275a`。本轮不启动 GUI/真内核、不操作 OS 网络、不访问 OpenBox，不 stage/commit/merge/rebase/push。

## 有界引入与消费者

只读读取 `.worktrees/p2-06-applied-proxy-facts` 的交接文档、精确 git diff 和新增模块。原 owner 编写的完整改动原样引入以下 5 个源码/测试文件，最终 SHA 与 owner 树一致：

- `crates/veyra-core/src/application/applied_proxy_facts.rs`（新增）；
- `crates/veyra-core/src/application/manual_runtime.rs`；
- `crates/veyra-core/src/application/manual_runtime/tests.rs`；
- `crates/veyra-core/src/singbox/compiler/recovery.rs`；
- `crates/veyra-core/src/subscription/outbound.rs`。

第 6 个 owner 文件 `application/mod.rs` 仅合并 `pub mod applied_proxy_facts`，保留全部 P2-08 导出/注释。未改写 owner 契约或交接文档，未触碰 Helper/IPC/OwnerTransfer/Platform/Desktop RuntimeService/Cargo.lock。owner-tracked.patch SHA256：`07cfa27827a4958c63fbf4f0796f398b3c028047a2f3bd3b2beb421a0deedfa3`；来源和逐文件 SHA 在本地 `owner-provenance.json`，Host 原核验 CommandRun `command-23397-1791610154887527-293` 仅为交接事实，不冒充本轮执行。

本轮业务消费只改 `application/proxies.rs` 及 `proxies/tests.rs`：

- 同步 `snapshot` 调用 `RunningProxy.applied_proxy_facts(instance, applied_config)`，不等待 HTTP。新增 `ProxyEntry.applied_members` 为有序应用成员，与 Saved `outbound.references` 分开；配置/实例不匹配或旧索引缺失保持 Unknown，未运行/未应用项不可用。旧确认手动选择必须在应用成员内，自动组不从 Saved/Compiler 默认造 current。
- `begin_test` 对完整配置版本严格要求 Saved=Applied，并以实际应用图展开自动/URLTest/手动组及内部线路；反向压栈保留 owner 顺序，visited 去重/去环，节点必须属于发布能力。没有应用事实拒绝；不从保存目录补造成员，Direct/Block 只列 untestable。旧 generation、取消、存储、选择隔离保持。
- 新最小异步 `ProxyService::observe_applied_pool(runtime, &PoolId)` 返回 `Result<Option<AppliedPoolObservation>, ProxyError>`，复用 owner 的固定鉴权 Controller GET。返回前核对原能力、当前实例/应用配置和在途 Saved 配置变化；外层 None 表示能力/请求失效，`Some(current=None)` 是已应用但当前出口未知。Node/Pool/Direct 保留领域类型，不递归猜叶子，不写选择或新缓存。
- 可显式读取仍运行的旧 applied 版本；DTO 的 config 是 applied 而非新 Saved。UI 必须核验 DTO instance/config/页面 generation，Unknown 清除旧显示，不能把旧应用观测并入新保存配置页面。本轮 **未改任何 GPUI/视觉源码，也未接 UI 自动观测任务**。
- 默认 HTTP UI 目标与原错误提示保留。新增用例确认 UnsupportedNodeTestUrl、没有 Controller delay 请求、磁盘 URL bytes 不变；不替换 URL 或 health 配置。

## 验证与失败

每条 Cargo 构建/测试/Clippy 命令采用 `MACOSX_DEPLOYMENT_TARGET=15.0`、`--offline --locked`、Python subprocess 600 秒外层超时；cwd 为本隔离树。命令完整 argv/exit/原日志位于 `evidence/p2-08/owner-facts-integration/`，现有 Git ignore 保持，Host 须随交接保存。

Core 测试统一为 `cargo test -p veyra-core --offline --locked <下表filter> -- --test-threads=1`。

| filter | PASS / exit | 收据 |
| --- | --- | --- |
| `application::proxies::tests` | 17 / 0（原11+新6） | 04-proxies-final |
| `application::manual_runtime::tests` | 47 / 0（含 owner 新鉴权 loopback 用例） | 05-owner-runtime |
| `singbox::compiler::` | 62 / 0 | 06-compiler |
| `application::network::tests` | 4 / 0 | 07-network |
| `application::observability::tests` | 21 / 0 | 08-observer |
| `storage::latency::tests` | 2 / 0 | 09-history |
| `application::draft_latency::tests` | 2 / 0 | 10-draft |
| `application::outbound_query::tests` | 1 / 0 | 11-query |
| `singbox::clash_api::tests` | 14 / 0 | 12-clash-api |

Desktop 测试命令把包改为 `veyra-desktop`：`runtime_service::` 6 PASS / 4 Native ignored、`ui::proxies::tests` 3 PASS、`services::p208_tests::` 1 PASS，均 exit0，收据13–15。**本轮 Core170 + Desktop10 = 180 个唯一 PASS**，不把重复轮次或 ignored 当 PASS；原 P2-08 55 项为历史覆盖，本轮新增消费者用例6项，不与重跑回归相加冒充新增数或新 Native。

`cargo check -p veyra-core -p veyra-desktop --all-targets --offline --locked`、对应 `cargo clippy ... -- -D warnings`、`cargo build -p veyra-core -p veyra-desktop --offline --locked`、`cargo fmt --all -- --check`、Legacy fmt check 均 exit0；最终 `git diff --check` exit0。Legacy `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --offline --locked -- -D warnings` **FAIL101**：Windows sing-box LICENSE 资源缺失；不绕过/修改资产，历史 libcronet.dll FAIL 也不改写。

新6个测试保护的具体行为：

| 用例 | 用户行为/自有契约 |
| --- | --- |
| applied_auto_pool_snapshot_latency_and_authenticated_now | 自动组已应用可测实际成员，同步不发HTTP，真实GET返回Now稳定ID，历史服务reopen且不写选择 |
| applied_nested_groups_and_lanes_keep_member_order | 嵌套组/线路按编译顺序遍历、重复叶子去重、Direct不当成功 |
| missing_stale_and_unapplied_facts_reject_saved_fallback | 缺索引/未应用Pool/实例或版本错误时拒绝，确认/Saved不补应用事实 |
| applied_pool_observation_keeps_pool_direct_and_unknown | 真实鉴权GET保Pool/Direct类型，非法now/503/断连Unknown，不猜叶子或写选择 |
| applied_pool_observation_discards_replacement_and_config_change | 在途相同文本实例能力替换/保存变化丢弃；真实两秒GET超时Unknown |
| applied_auto_pool_preserves_unsupported_http_preference | 原HTTP不改，不支持时准确未执行，不偷测别站 |

旧 group_edit 回归增加 Saved `[a,b]`、旧 Applied `[a,b,c]` 独立断言，新能力发布后才测新成员；未应用单节点现在在批次发送前拒绝，原 Network NodeNotApplied→Unknown 断言继续保留。fixtures 使用正式 Compiler 生成应用索引，网络由生产 Service/ClashApiClient 发起到自有真实 loopback Controller；Runtime child 为 fixture/Mock，不能当 sing-box/Native。

本轮初始失败原样保留：01-proxies 编译 FAIL101（测试 ApiSecret 类型、ConfigVersion 包装访问、闭包 ID ownership）；02-proxies 2 PASS/15 FAIL、exit101（fixture 无选中订阅而调用生产投影，被 SelectionRequired 拒绝）。修复仅测试 fixture 使用正式 RuntimeIntent/Compiler 与正确类型，未放宽生产投影/超时/断言；03 与最终04 均17 PASS。Legacy FAIL101另列。

## 审查、保护与后续

实现者按 code-delivery-review 自查，独立只读 Reviewer `/root/owner_facts_review` 两轮复核未发现可信阻断 Finding；Reviewer读源码/收据，不运行Cargo/GUI/Native。owner来源、启动前1053文件SHA与最终保护收据可重核。原 UI/Core 非消费点、bundle（最新 signed Mach-O `9b0a8112786d348e984f01c89e807fc70ffb99a7bc660e216d8866efbc604570`）、截图及全部旧证据保持原字节；owner 树保持原字节。测试自有临时文件/loopback资源已清理，本轮没有真正 App/内核/OS 资源操作。

**Core owner-facts 缺口已接通；GUI自动组观测接线仍交 Host/Codex Desktop**：在既有异步任务调用此typed入口，按实例/applied config/页面generation接收；Unknown清旧值，嵌套Pool如需叶子必须每层独立GET，不得读默认补齐。随后以新构建执行Auto实际操作与状态验证。默认HTTP产品决定、完整逐态视觉/Zero-Unknown GUI、最新同PID托盘恢复、真实在途GUI Quit等原缺口不因Core PASS关闭。保持DOING、不勾完整验收、不提交或合并。


## 2026-10-10 用户已批准默认HTTPS（此前记录保留）

用户正式批准新安装/新建 AppState 的 UI 节点测速默认值改为 `https://www.gstatic.com/generate_204`，这是相对 React 原 HTTP 默认的授权差异。timeout5000/low400/medium800不变；真实 state.json 已保存的 HTTP 不迁移，读取/重启/无关行为 patch/语言保存后原值保留。Runtime profile 原 HTTP、group health 继承/显式 URL、owner事实契约、React/UI几何和测速控制器保持。旧 HTTP 测速仍 UnsupportedNodeTestUrl、未发 Controller 请求；正式节点和草稿统一三语言提示，简中为“节点测速仅支持 HTTPS，请到设置中修改网址；原设置未改”。

本轮仅六源码（domain/desktop_behavior.rs、application/state_service.rs、application/proxies/tests.rs、desktop/ui/proxies.rs、subscriptions.rs、i18n.rs）及这两份验收文档追加。以下为本轮21个唯一PASS（Core14/Desktop7），新增3例，不与原55/上轮180或失败重跑相加。完整命令argv/exit/log与本轮增量diff在 [default-https-20261010](evidence/p2-08/default-https-20261010/)，现有ignore保持，Host需保留本地证据。Cargo统一 --offline --locked、MACOSX_DEPLOYMENT_TARGET=15.0，外层600秒。

测试命令为 `cargo test -p <package> --offline --locked <filter> -- --test-threads=1`，HTTP单例另有 `--exact`。

| package / filter | PASS / exit | 用户行为与收据 |
| --- | --- | --- |
| veyra-core / domain::desktop_behavior::tests | 7 / 0 | UI默认HTTPS/阈值、Runtime与Group URL分离；core-domain |
| veyra-core / application::state_service::behavior_tests | 6 / 0 | 新默认实际落盘重开、旧HTTP读取bytes/mtime不变、偏好patch/语言保存/重开完整保留；core-store-fixed |
| veyra-core / application::proxies::tests::applied_auto_pool_preserves_unsupported_http_preference | 1 / 0 | 显式保存旧HTTP，生产服务拒绝、自有loopback请求为空、文件bytes不变；core-old-http |
| veyra-desktop / ui::proxies::tests | 4 / 0 | 旧HTTP错误三语言指向Settings，原搜索/排序回归；desktop-proxies |
| veyra-desktop / ui::i18n::tests | 3 / 0 | 唯一完整翻译/业务标识不翻译/语言切换列顺序；desktop-i18n |

`cargo check -p veyra-core -p veyra-desktop --all-targets --offline --locked`、同参数 `cargo clippy ... -- -D warnings`、同参数 `cargo build ...`、`cargo fmt --all -- --check`、最终 `git diff --check` 均 exit0。首轮core-store **FAIL101，5 PASS/1 FAIL**保留：新增fixture误以为空AppState自带默认组；只修测试显式default_groups后6 PASS。自查与独立只读Reviewer未发现剩余源码Finding，见 [REVIEW.md](evidence/p2-08/default-https-20261010/REVIEW.md)。旧Legacy资源Clippy FAIL101原样保留，本轮未复跑或修改资产。

原1108文件SHA保护清单中除六源码及两份追加文档之外全部原字节保留，包括最新signed bundle `9b0a8112786d348e984f01c89e807fc70ffb99a7bc660e216d8866efbc604570`、截图/Native收据、owner导入文件、Cargo.lock、任务状态。测试只用自有临时文件/loopback，存储测试已删自有root；失败发生在写盘前，无失败root产生。未启动GUI/真实内核、未操作OS网络/外网/管理员，未stage/commit/merge/push。

此前“默认HTTP待决定”仅为历史，产品默认决定现已批准落实；**GUI新默认与旧HTTP三语言提示仍NOT_RUN**。Host交Codex Desktop：使用隔离新root验证HTTPS默认，另以显式保存HTTP的隔离root重启/切语言、测速仍拒绝且提示手动修改，确认设置原值保留；用户手动改HTTPS后再测速。owner typed自动组GUI接线、完整逐态视觉/实际Zero-Unknown、最新同PID托盘恢复及在途GUI Quit仍保留待验；P2-08/P2-06继续DOING、owner/68卡/READY不变。


## 2026-10-10 Desktop 真 GUI 消费追加

此前“尚未接GUI”保留为历史。本轮在既有GPUI异步通道消费正式observe_applied_pool，严格核Ready/InstanceId/完整Applied与Saved ConfigVersion/页面generation；身份变化先清旧快照，未知不从保存默认补值。真实固定1.14.0Controller证明auto-node now a、auto-mixed now Direct及nested Pool实际成员，生产GUI已投影；saved!=applied显示Unknown，正式Stop/Start后读取新实例。receiver取消/drop与测速批次同步，沿既有uptime刷新，不新建选择writer或公共Runtime状态。P2-06来源文件保持输入SHA。

最终包d8e0d506...正式694ms/草稿796ms、同PID Hide/TrayShow通过；原733/734ms历史写回FAIL继续OPEN，Core自有错误分类最小修不冒充竞争已消除。Zero、在途TrayQuit取消、完整视觉仍未通过，P2-08继续DOING；见[精确证据与限制](evidence/p2-08/final-gui-20261010/README.md)。
