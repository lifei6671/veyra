# P2-08 Core/服务端阶段1检查点

2026-10-10，**DOING，未完成整卡验收**。正式 ProxyService、节点/已确认手动组测速、SQLite 历史与隔离草稿测试服务已实现；Core 49 + Desktop 服务装配 1 = **50 个唯一定向测试 PASS/exit0**。GPUI、视觉、真实 sing-box/Native、托盘、SystemProxy/TUN、外部网络全部 **NOT_RUN**，由 Host 交 Codex Desktop 接续。本轮不提交、合并、push，不释放 P2-08 owner，不解锁下游。

## 身份与范围

- WorkRun：`work-23397-1791604235718790-269`。
- 唯一项目读写树：`/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/.worktrees/p2-08-proxy-latency`；分支 `dev/p2-08-proxy-latency`。
- 输入及最终 HEAD：`6857b2e39633ca0564cac66b40ee0f9129cf275a`；开始干净，最终修改未暂存。未创建其它 worktree，未写主树或清理其 `tools/tests/__pycache__`。
- P2-06 仍 DOING，由原 Codex 独占 Runtime/Platform/Helper/IPC/公共 Runtime DTO。未改 `crates/veyra-helper`、Desktop `runtime_service.rs`、`platform/`、Core helper protocol/transfer、remote selection/owner transfer、公共 Runtime DTO、Cargo.lock。仅只读消费既有能力，无第二选择 writer、Runtime 命令队列或 cache writer。
- 已读当前 AGENTS、2026-10-08 分期规则、SESSION、唯一任务行、P2-08 五项验收、P2-02A/04/05、P1-04B、方案 §6.2.2/8.1–8.2 与指定源码。React/CSS 仅用于行为核对；未访问参考网站或操作电脑。
- 68 卡：**DONE31 / READY4 / DOING2 / TODO24 / DEFERRED7**。READY 恰 P3-05/P4-01/P4-04/P5-01，均未领取；P3-02/P3-07 等下游继续 TODO。历史计数、FAIL/NOT_RUN、P0-09 macOS15 估算原样保留。

## 正式实现与调用边界

`ProxyService::snapshot(&ManualRuntimeSnapshot)` 通过唯一 `OutboundQueryService/OutboundQuerySnapshot` 取得目录，并以 `SnapshotService` 核对版本；不持久第二份目录、不以显示名称识别出口。快照提供稳定 OutboundId/NodeId/PoolId、目录可用性、应用事实 `Option<bool>`、已确认当前选择、完整状态/应用版本和现有 UI 偏好。未知保持未知；未选择不会显示为 Direct；保存与已应用不合并。组卡片延迟通过 `selected_latency` 沿已确认选择链找到当前 Node，不能取组历史中任意成员充当当前出口。

`selection_request(runtime, pool, member, mode)` 只构造既有 `ManualSelectionRequest<SelectionChoice>`，携带 instance、config、expected selection 版本。普通手动节点、Group 及主备 lane 的 selector/父 Group 身份分别保留，非法成员/模式/过期快照拒绝。后续 UI **直接调用既有 `services.manual_runtime.select_manual(request)`**，消费原 `Result<SelectionVersion, SelectionError>` 及后续 Runtime 快照；pending 保存、Controller 确认/读回、最终存盘或恢复记录失败仍按原入口报告。ProxyService 不进行 Controller PUT、选择存盘或伪成功确认。

`begin_test(runtime, scope)` 每次读取当前唯一目录及 `app_config.behavior.latency`，固定当前 Ready 实例能力、稳定成员和 generation。`run_test` 使用 P2-05 正式 `NetworkService`，沿用用户 UiLatencyUrl/timeout_ms；全服务共享并发上限 4，支持排队/在途取消和关闭。已确认组按稳定 ID 遍历、去重，Direct/Block 单列 untestable；不可由这些结果推导物理 Direct 成功。组配置变化、节点新请求/组新请求、实例替换均使迟到结果分别 StateChanged/Superseded/InstanceChanged；selection revision 独立变化不被测速写回覆盖。写历史前使用既有 `with_runtime_version` 短 gate 与 SQLite 事务，未修改 owner 契约。

`UiLatencyPreferences.low_ms/medium_ms` 决定节点延迟档位；Success、Zero、Unknown、Timeout、Failed 分开，未测为空历史。零值保留 `Some(0)` 并呈现 Zero，不能当成功；typed NetworkError 同时保留。`visible_nodes()` 实时消费排序和隐藏明确不可用项；失败测速不把目录 alive 推成 false。其它 proxy_view 字段完整暴露供后续 UI 消费，没有伪称列数、宽度、按供应商分组或策略顺序的视觉行为已完成。中文名称排序差异见 OPEN。

`LatencyStore` 复用现有 rusqlite 依赖与 SQLite 存储模式，不新增依赖/锁文件或泛型 DB 框架。Desktop 装配 `root/latency.sqlite3`，存实际完成时间、state_epoch、稳定 Node/组 scope、实例/generation/outcome；不保存节点凭据或另一份业务配置。节点与可选组历史同一事务 append/trim，每个 epoch/scope 按实际测试时间及写入序号保留最近 10 条，WAL/FULL、有界 busy timeout。真实文件回读、reopen 与重建服务可恢复；组写失败回滚节点/组并保留旧历史。React 本地 `latencyHistory` 使用内存合并并取最近 10 条；本轮新增本地跨重启 SQLite 真实持久化，未推断 React 远端 API 是否持久化。

`DraftLatencySession<SidecarPort>` 使用既有 SidecarRuntime/Compiler/NetworkService。每次独有 0700 root、运行配置、动态 loopback mixed/controller、cache.db 与临时鉴权；草稿正式 normalize 后只在独立内存 AppState 编译，不写主 state/config/manifest/cache、不替换 ManualRuntime。Desktop factory `services.draft_latency_session()` 已绑定现有 `ManualSidecarPort::new(owned_root)`，本轮未启动真实 child。测试/超时/取消后先撤销本实例能力、stop 确认，再删除其 root；cleanup 错误与测试结果独立，失败保留 root/port，可显式 cleanup，不伪报资源已释放。check/run/ready 沿用现有 Port 的有界预算，期间取消在返回后清理；不新增 shell/协议栈、管理员/TUN/系统代理操作。

## 原五项验收的阶段边界

| 原验收项 | 本轮已验证 | 未完成部分 |
| --- | --- | --- |
| 选择成功新连接使用所选出口；失败/保存分开 | 精确 Request adaptor；既有唯一选择入口 16 项回归 PASS，错误/确认/保存语义保留 | 本候选实际 UI 选择与固定内核新连接出口 Native NOT_RUN；不勾整项 |
| 测速不盖新选择；未测/超时/失败/零分别展示 | 正式服务 generation/版本隔离、typed outcome、loopback/真实历史 PASS | GPUI 实际展示/忙碌/取消 NOT_RUN |
| 消费 UI latency 与阈值；不覆盖 health | URL/path/timeout/阈值/偏好测试 PASS；未写 profile/group health | 默认 HTTP 支持限制 OPEN；真实页面偏好消费 NOT_RUN |
| 草稿成功/取消/超时独立清理且主实例继续 | 生产 Compiler/Network 服务 + Port fixture，真实文件/loopback 端口释放、主文件哨兵不变 PASS；cleanup 失败保 root PASS | 固定版本真实 child/主实例并存与退出 Native NOT_RUN |
| 卡片/测速按钮互不误触；浅深色实际验收 | 本轮未写任何 GPUI 视图 | 全部 UI/视觉/交互 NOT_RUN，由 Desktop 接续 |

因此原卡五个 checkbox 均保持未勾选；本轮服务交付不能替代整卡 UI/Native 结果。

## OPEN 与原 owner 精确需求

1. **已应用自动组目录/当前出口事实不足**。当前公开 ProxySource 只有已应用 Node 映射，ManualRuntimeSnapshot 有已确认手动/Group 选择，但没有所有已应用 Pool 成员边与自动组当前选择。服务对同配置且已有确认事实的手动组测速；配置未应用返回 ConfigNotApplied，自动组/未知组返回 GroupApplicationUnknown，不能拿“节点存在”推导组已应用。请原 Runtime owner 基于既有 AppliedArtifactIndex/owned snapshot 增加**只读**事实：绑定 `InstanceId + 完整 ConfigVersion` 的 `PoolId → Vec<OutboundId>` 已应用成员边，以及可确认的 `PoolId → OutboundId` 当前出口（不可确认用 None）。保持唯一 OutboundQuery 目录、原 writer/队列/鉴权封装，不把 secret/tag/写权限交给 UI。本轮停止在 owner 边界，未写这些文件；该能力补齐后再支持自动组/外部联动的实际当前结果。
2. **默认 HTTP UiLatencyUrl 不被 P2-05 节点接口支持**。正式 NetworkService 仍按原契约返回 UnsupportedNodeTestUrl，本轮准确归 Unknown 并保留错误，不暗改默认偏好、不替换目标 URL、不修改 Runtime profile/group health URL。loopback 用支持的用户目标 URL 验证 Controller 实际收到了 path/query/timeout。Desktop 必须显示该错误；支持 HTTP 若需要，另在正式 Network 契约中完成有证据的最小变更，不能静默测试另一站点。
3. **名称排序 locale 差异**。当前 Core NameAsc/NameDesc 是 Rust Unicode 比较；React `localeCompare(..., "zh-CN")` 是中文 locale 排序，如北京/上海顺序可能不同。独立 Reviewer 保留此 P2 Finding。未引入排序依赖或改 Cargo.lock；Desktop 接续需在真实中文混合名称行为中修复/验证，不能宣称完全一致。
4. **草稿取消句柄归 UI session 持有**。后续后台 blocking worker 必须持有 DraftLatencySession 到 cleanup 完成；离页/Quit 发送独立 TestCancellation 并等待/核对清理。AppServices::request_closing 已关闭正式 ProxyService/NetworkService，尚无 UI 草稿 worker 可注册。本轮不能拿 Drop 单独代替真实退出验收；cleanup 失败要反馈并保留可清理 session。

## 新测试保护的用户行为

每一条新增测试对应 Veyra 行为；Port/Controller fixture 不冒充内核或桌面验收。

| 新测试（Core 路径前缀见命令表） | 保护的用户行为/自有契约 |
| --- | --- |
| snapshot_and_selection_adaptor_use_only_ids_and_owner_facts | 同名节点仍用稳定 ID；选择 Request 只消费 owner 事实，未应用/未知不猜 Direct |
| group_loopback_preferences_zero_failure_and_service_reopen | 组按 ID 测成员，用户 URL/path/timeout 原样，Zero/失败分开，正式服务重建读历史 |
| newer_node_generation_wins_and_selection_change_is_preserved | 迟到测速不覆盖新请求或刚选择的节点 |
| timeout_cancel_and_unapplied_node_are_distinct | 超时、取消、未应用节点分别反馈，取消不写历史 |
| concurrent_batches_share_limit_and_cancel_waiters | 两批组测共享上限 4；排队取消不能继续发请求/写历史 |
| group_edit_and_instance_replacement_discard_late_results | 当期成员变更、正式实例更换后迟到结果不落入新状态 |
| live_view_preferences_sort_thresholds_and_hide_only_explicit_unavailability | 偏好实时读，阈值/延迟排序/隐藏行为不把测速失败当目录不可用 |
| failover_lane_selection_adaptor_keeps_parent_group_and_selector_ids | 子线路选择保留父组与 selector 身份，不破坏原 owner 的 lane 契约 |
| selected_group_latency_uses_current_node_not_latest_group_sample | 当前组卡片取实际选中节点延迟，避免任意最近成员误导 |
| shutdown_cancels_inflight_and_preserves_long_timeout_preference | 退出取消在途/排队；超过 30 秒的合法用户超时不被截断 |
| draft_success_timeout_cancel_cleanup_real_files_and_loopback | 单草稿成功/超时/取消清理本 root、两端口/cache，主文件/外部监听不变 |
| cleanup_failure_preserves_owned_root_until_explicit_cleanup | stop 失败不删活实例 root，清理结果可见并可再次清理 |
| disk_roundtrip_reopen_keeps_ten_actual_times_and_stable_ids | 跨重启历史保稳定身份与实际时间，每 scope 最近 10 条 |
| failed_group_append_rolls_back_node_and_preserves_reopen_history | 历史磁盘写失败保旧，组/节点事务一致，重开仍旧值 |
| Desktop proxy_and_draft_services_use_owned_storage_without_runtime_start | AppServices 真装配历史磁盘与独立草稿 root，关闭无真实 Runtime 启动 |

既有 Network 4、Query 1、Selection 6、P4-03 Group selection 10、Clash API 14 共 35 条定向回归分别保护节点目标路径/鉴权与错误、唯一查询无状态改写、唯一选择 Controller/保存/版本语义、Group/lane 选择及 API 解析。零值断言保留 Some(0) 并由新测试验证 Zero；Query 旧计数 14 改为当前内置 groups 后的 15，其余 schema/bytes/身份/version 断言未删除或弱化。

## 可重跑命令与本轮结果

所有命令 cwd 均为本隔离树。Cargo 构建/测试/Clippy 使用 `--offline --locked`；Python `subprocess.run(timeout=...)` 外层预算：Core 600s，Desktop/Legacy 900s，diff 60s。格式命令不接受上述 Cargo 依赖解析 flags。没有全量无过滤库测试，零过滤/重复轮次不计 PASS。

| 命令 | exit / 唯一 PASS | 原日志 |
| --- | --- | --- |
| `cargo test -p veyra-core --offline --locked application::proxies::tests -- --test-threads=1` | 0 / 10 | proxy-final.log |
| `cargo test -p veyra-core --offline --locked application::draft_latency::tests -- --test-threads=1` | 0 / 2 | draft-third.log |
| `cargo test -p veyra-core --offline --locked storage::latency::tests -- --test-threads=1` | 0 / 2 | history.log |
| `cargo test -p veyra-core --offline --locked application::network::tests -- --test-threads=1` | 0 / 4 | network.log |
| `cargo test -p veyra-core --offline --locked application::outbound_query::tests -- --test-threads=1` | 0 / 1 | query-repaired.log |
| `cargo test -p veyra-core --offline --locked application::manual_runtime::tests::selection_ -- --test-threads=1` | 0 / 6 | selection.log |
| `cargo test -p veyra-core --offline --locked application::manual_runtime::tests::p403_ -- --test-threads=1` | 0 / 10 | group-selection.log |
| `cargo test -p veyra-core --offline --locked singbox::clash_api::tests -- --test-threads=1` | 0 / 14 | clash-api.log |
| `cargo test -p veyra-desktop --offline --locked services::p208_tests::proxy_and_draft_services_use_owned_storage_without_runtime_start -- --exact --test-threads=1` | 0 / 1 | desktop-service.log |
| `cargo check -p veyra-core --all-targets --offline --locked` | 0 | core-check.log |
| `cargo clippy -p veyra-core --all-targets --offline --locked -- -D warnings` | 0 | core-clippy.log |
| `cargo check -p veyra-desktop --all-targets --offline --locked` | 0 | desktop-check.log |
| `cargo clippy -p veyra-desktop --all-targets --offline --locked -- -D warnings` | 0 | desktop-clippy.log |
| `cargo fmt --all -- --check` | 0 | workspace-format.log |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | 0 | legacy-format.log |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --offline --locked -- -D warnings` | **101 / FAIL** | legacy-clippy.log |
| `git diff --check` | 0 | diff-check.log / 最终身份收据 |

总计 **Core49 + Desktop1 = 50 唯一 PASS**，其中新 Core14 + Desktop1 = 15；不叠加 proxy 首轮4/第二轮7/第三轮9 或重复执行。P2-05 Host 历史 Clash API16 不是本轮14，未改写原历史。

### 失败历史保留

- Proxy 初次编译 exit101：async node borrow/move、epoch partial move、测试调用不存在 commit_selection。改为正确 ownership 与既有 Store commit，后续 PASS。该首次错误保留在本轮工具记录，非原始日志文件；`failures.md` 明确为摘要。
- draft-first.log：private normalize 模块导入，exit101；使用既有 public re-export 修复。
- draft-second.log：0 PASS/2 FAIL、exit101；fixture 向已丢 receiver 通知导致 panic，且 RecoveryRequired 时不能用 active-port helper 解除测试 stop 故障。fixture 改为只在取消 case 通知，外部 Arc<AtomicBool> 解除故障；生产 stop/root 保留逻辑未放宽。draft-third.log 2 PASS。
- query.log：0 PASS/1 FAIL、exit101；当前目录实际 15 项，旧测试预期14过期；只更新预期并保留其余断言，query-repaired.log 1 PASS。
- Legacy Clippy 本轮 exit101 原因：`binaries/sing-box-1.14.0-windows-amd64/LICENSE` 缺失，Tauri build.rs 在 lint 前失败。未修改无关 Windows 资产、未以 TAURI_CONFIG 绕过。历史 libcronet.dll FAIL101 仍属于原记录，不能将本次 LICENSE 错误改写为 libcronet.dll 或 PASS。

## 文件差分、审查与资源

新增六个 Core 文件：`application/proxies.rs`、`application/proxies/tests.rs`、`application/draft_latency.rs`、`application/draft_latency/tests.rs`、`storage/latency.rs`、`storage/latency/tests.rs`（均位于 `crates/veyra-core/src/`）。

修改七个源码文件：Core `application/mod.rs`、`storage/mod.rs` 模块导出；`application/network.rs` 固定实例内部测量 seam/尊重 timeout；`application/network/tests.rs` 零值断言；`singbox/clash_api.rs` 零值保留；`application/outbound_query.rs` 仅旧目录计数测试；Desktop `crates/veyra-desktop/src/services.rs` 正式服务装配、草稿生产 Port factory、关闭及装配测试。无 UI 实体/GPUI view 修改。

文档四个：本检查点新增；SESSION 当前摘要/Active/Ready、IMPLEMENTATION_PHASES 顶部当前摘要及唯一 P2-08 行、P2-local-proxy 本轮阶段说明。其它卡、五项原验收 checkbox、历史摘要不改写。最终源码/文档 SHA 与未暂存清单见本地身份收据，源码差分包括 untracked 新文件，自查不能只看 tracked diff。

实现者按 `code-delivery-review` 对源码、测试与输入主分支 HEAD 完成自查；独立只读 Reviewer 复核两轮，未亲自运行 Cargo/Native。lane 父组 Request、实例能力 race、组延迟任意成员、cleanup 故障 fixture Finding 已修复；仍保留中文 locale 排序 P2 Finding 与公开应用事实缺口，不宣称零问题。Host 还需独立最终 Review。

测试只创建自有临时 SQLite/文件与 loopback HTTP/listener，全部使用正式服务接口；草稿进程边界用 Port fixture，**不是固定 sing-box/OS 进程生命周期证据**。成功/超时/取消后两端口可重新绑定，主 cache/config/manifest 哨兵 bytes 与外部测试监听保持。首轮失败留下的两个草稿 root 和旧 Query root 已按精确本次身份删除，收据分别 `failed-test-cleanup.json`、`failed-query-cleanup.json`；最终 `veyra-p208-*` root 扫描为空。未操作已有 utun、系统代理、路由/DNS、管理员 Helper、外部订阅或真实 child。

原始日志、验证命令/exit/test数、task-audit、清理与身份 SHA 收据均在 `docs/openbox-rust-gpui-tasks/evidence/p2-08/backend/`。**该目录受现有 Git ignore 影响，未强制暂存；Host 应随检查点保留/传递这些本地证据，不能假定它们已在 Git 提交中。** 日志 PASS 的 loopback、真实文件、Port fixture 与未来 Native 必须分开。

## 给 Codex Desktop 的最短接续指令

继续本隔离树/分支及 WorkRun，先核对本检查点和 owner OPEN，保持 P2-06 唯一写入口、不改 owner 文件/Cargo.lock、不提交。通过 `services.proxies.snapshot(latest_manual_snapshot)` 接代理快照，选择只用 `selection_request → services.manual_runtime.select_manual` 并保留原错误/保存/已应用反馈；测速使用 `begin_test/run_test` 与取消句柄，只更新历史/刷新快照，不能用测速结果覆盖选择。自动组已应用图缺口先交原 Runtime owner，未知准确展示。

草稿通过 `services.draft_latency_session()` 在后台持有 session，使用当前 UI latency 偏好，离页/退出取消并核对独立 cleanup；真实固定版本 Native smoke 验证主实例持续运行、成功/超时/取消仅清理草稿 root/child/端口/cache。HTTP 不支持、Zero/Unknown/Timeout/Failed、忙碌/空/错误均必须真实反馈；处理中文 locale 排序差异。

随后按现有 React/CSS 同数据、同窗口/缩放逐态移植最小代理/测速 UI，复用共享组件和 `ui::i18n` 即时语言/重启恢复，执行实际卡片选择与测速按钮互不误触、加载/空/错误/忙碌、浅深色与截图对照。订阅/组/日志现有 UI 保持；未完成整卡 UI/Native 前继续 DOING，Host 独立 Review 后再决定状态和 Git 收口。


## 2026-10-10 owner 只读事实消费者追加（此前记录保留）

原 WorkRun 同隔离树/HEAD；已原样引入 P2-06 owner 的6个Core文件改动（mod只合并导出），接通 ProxyService 同步 Applied 成员、实际应用图组测速与异步鉴权 Now 入口。Core自动组事实消费缺口已关闭；同步无HTTP，Saved/Applied分开，实例/版本/能力替换拒绝旧结果，无选择writer或新缓存。详见[P2-08 owner事实集成](P2-08-owner-facts-integration.md)。

本轮 Core170 + Desktop10 = 180唯一定向PASS/exit0，含新增消费者6例；原55覆盖为历史，不重复累计，4个Native opt-in ignored，不执行新GUI/Native。check/Clippy/build/fmt/diff exit0；Legacy Windows LICENSE Clippy FAIL101及01/02初始测试FAIL保留。原UI、Native bundle/截图/证据与owner树SHA保持，临时测试资源已清理。

P2-08与P2-06继续DOING、原owner/68卡状态不变，未stage/commit/merge/push。GUI须接异步typed观测并核验instance/applied config/页面generation，Unknown清旧显示；默认HTTP产品决定、完整视觉/Zero-Unknown GUI、托盘恢复/真实在途GUIQuit等原缺口保持，不以Core集成覆盖历史NOT_RUN或重绑旧截图到新构建。
