# OBG-P1-04B 跨页面行为偏好与消费契约

2026-10-04，DONE。owner：Codex Desktop · Core/Config + GPUI；共享 Preferences DTO owner 已释放。本轮只执行 P1-04B，不启动下游。起始 HEAD `c18432544e63d18cc91011fa40f6cc874f780677`，工作树干净；本轮未 commit/push。

## 模型、迁移和保存

`AppConfig.behavior: DesktopBehaviorPreferences` 是唯一持久事实，分为 latency、proxy_view、diagnostics、ip_info、test_sites。沿用 AppState → StateAccessGate → JsonStateStore → ConfigVersion CAS。没有第二个 Preferences 文件、任意 key/value Settings 或浏览器扫描。

schema **8 → 9** 显式补 canonical behavior 默认；保留 visual、Profile、state_epoch 和 config/selection revision。迁移不算编辑、不推进 revision；重复 load 稳定。旧 v7→v8 与所有 store/version 回归仍通过。

`DesktopBehaviorPreferencesPatch` 以 Option 表示未修改/赋值；test_sites 和 strategy_order 提供时整体替换。strategy display ID 去重且有数量/长度上限，不成为 Group membership/order。CAS 成功仅 config_revision +1，no-op 不推进；返回 SavedOnly，selection/epoch/Profile/visual 不变。非法 URL、数值关系、列数/宽度、site ID/URL/name/icon 等通过具体 FieldPath 定位。数值使用有范围无符号整数，UI 拒绝非法文本，不静默 clamp。

canonical latency 为 **HTTP** `http://www.gstatic.com/generate_204`、5000ms、400/800ms；columns 2。React 消费者 fallback 曾为 500/1000 与 columns 1；P0-01 observed storage 已脱敏，不能裁决真实原值。按用户可见 PanelSettings 定义及其 original-defaults/double-columns 测试归一化，明确不是全部旧 fallback 行为一致。其余默认和来源见 [default-normalization](default-normalization.json)。旧 localStorage 不自动导入，Proxy tab 继续是 session state。

## 语义与消费者边界

UI latency URL / RuntimeHealthUrl / GroupHealthUrl 使用不同类型；Group 空值只继承 Runtime URL，不能使用 UI URL。本卡没有增加 Backend Profile 的 testUrl/directTestUrl，也没有实现 Group 持久化、Compiler 或 Runtime。UI diagnostics.ipv6_test 与 Profile.ipv6 独立，修改诊断不产生 apply/restart。见 [URL 三语义](url-semantics.json)、[IPv6 隔离](ipv6-isolation.json)。

类型化读取 `behavior_snapshot()` 和 broadcast `subscribe_behavior()` 交付 ConfigVersion、完整新 DesktopBehaviorPreferences 与精确 BehaviorField 列表。提交后在同一 gate 发通知，no-op/失败无通知；四类 Proxies/Overview/Connections/Diagnostics 的真实 service subscription 测试通过。未来消费者应先订阅再读 snapshot，按版本接受；Lagged 时重新读取 typed snapshot。后续真实业务接入仍由 P2-08、P3-02/05/07、P4-05C、P5-03 验收，六页 placeholder 不当作真实消费完成。见 [consumer-notifications](consumer-notifications.json)。

## 桌面操作与并发

Settings → 面板设置 → 行为偏好，四个 session 分区：延迟、代理显示、诊断/IP 信息、测速站点。复用 A 的 Input/Select/Switch/Toast；Behavior 自有 draft、pending patch、generation 和 saving/failed 状态，不与 Visual 共用 generation。Input Entity 持久，保存/刷新保留非法输入或失败草稿。Save 成功仅确认仍与提交值相同的 dirty fields；新编辑不会被迟到回执覆盖。

Codex Desktop 实际操作了 URL、timeout、阈值及错误、columns 1/2/3、hide unavailable、sort、provider grouping、width、strategy display IDs、IPv6、IP provider、四站点编辑/恢复默认。全部 URL 只编辑、解析、存储，未请求。Google 的 invalid ftp URL 真实返回字段错误，随后正确 URL 保存。

真实磁盘失败：只在隔离 root 建立 `state.tmp` 目录，阻止 JsonStateStore 原子临时文件创建。Core 保持旧值，UI 新草稿保留并显示 error Toast；切页再回仍保留。移除 fixture 后 Retry 成功。最终构建另复测一次，config15 → 失败仍15 → Retry16。见 [failure-retry](failure-retry.json)。

真实 RevisionConflict 两轮：UI9 / Core外部合法 CAS10 → 冲突 → Rebase/Retry11；最终构建 UI13 / 外部 Core14 → 冲突 → Rebase/Retry15。用户只编辑 timeout，Retry patch 仅 LatencyTimeout，外部 columns 完整保留。debug-only evidence 按钮调用真实 Core 服务，没有写死 error。见 [revision-conflict](revision-conflict.json) 与 [gui-final.log](gui-final.log)。

同一隔离 root 正常退出/重启两次，最终全 AppConfig 恢复，schema9 / config16 / selection0 / 同一 epoch。重启后实际逐分区确认 URL、7800ms、320/650、columns3、hide on、nameDesc、grouping off、width180、去重 display IDs、diagnostic IPv6 on、ipwho.is、Google 自定义 name/URL/icon。见 [preferences](preferences.json)、[interactions](interaction-results.json)、[restart log](gui-restart.log)。

## 验证与视觉

Rust1.99.0、GPUI Kit0.7.0、gpui-pre0.3.7；macOS27.0 arm64。全部 Cargo 离线、deployment target15.0、外层 timeout600s。最终 **293 core / 24 desktop** tests 全通过（新增13/7）；core/desktop check、desktop build、两者指定 clippy、fmt、workspace locked check、旧 Tauri lib clippy、desktop tree 全通过。[完整命令/日志](validation.json)。`TAURI_CONFIG={"bundle":{"resources":[]}}` 仅用于旧入口 compile check，不表示旧 Tauri 包资源完整。已有 block0.1.6 future-incompatibility notice 保留在日志。

单一 GPUI source，无新增依赖；Core 不依赖 GPUI，desktop 不依赖 Tauri/helper/prototype，不增加 tao/winit loop。render/click 无 block_on，一个 AppServices runtime；Tokio 只持有 core Arc/类型化数据/channel，GPUI Entity/Context 留在 foreground update/notify。见 [依赖](dependency-boundary.json)、[实现者自查](static-review.json)。自查使用 code-delivery-review Skill，未声称独立 Host Review。

9 张最终构建真实 native 图：light-behavior、dark-behavior、validation-error、revision-conflict、save-failure-draft、test-sites、proxy-display、diagnostics-ip、restart-restored。content1280×720，DPR2，截图2560×1506，包含66px物理 titlebar，未裁图；仅原始 JPEG 编码成 PNG。SHA256 及来源见 [visual manifest](visual-manifest.json)、[source provenance](source-provenance.json)。

布局沿用 A，新增四分区适应窗口，长代理区可垂直滚动；未观察到输入文字裁切。最终核对发现 primary hover 默认变黑使标签不可读，最小补绿色 hover/active token 并重新构建/实际复测。错误文案不显示 Rust Debug/None；输入附近显示字段错误。字体仍为系统字体，站点 icon key 有界文本，未下载图标；中英技术标签混排，不声称完整翻译或全六页像素级复刻。

## 四项卡验收与清理

1. 保存/恢复、具体字段验证、partial patch、typed consumer notification：PASS。
2. UI/Runtime/Group URL 三语义隔离，Group 空只用 Runtime：PASS。
3. diagnostics IPv6 与 Profile IPv6 双向独立：PASS。
4. Panel 实际操作、真实写失败、Retry、CAS/rebase、浅深色/重启：PASS；后续业务不计完成。

无 OpenBox HTTP/login、公网请求、实际测速/IP provider、sing-box、系统代理或 TUN 操作。用户已有 TUN 保持原样，不作为结论数据。所有 desktop 已正常退出，隔离 state/app/fixture/screenshot staging 已删除；正式 evidence 保留。[cleanup](cleanup.json)。68 卡显式 DAG 全部重算无环；macOS DONE10 / ACCEPTANCE1 / READY1 / TODO49，Windows DEFERRED7；READY 仅 P1-05，未领取。P0-05 仍 ACCEPTANCE，P0-06/P0-08/P2-01 等不解锁。[DAG](dag.json)。
