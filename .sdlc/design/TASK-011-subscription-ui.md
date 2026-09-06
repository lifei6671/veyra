---
id: DESIGN-TASK-011-SUBSCRIPTION-UI
status: FROZEN
task_ref: .sdlc/tasks/TASK-011.md
requirement_ref: docs/veyra.md
requirement_identity: sha256:ddd1d20999348e6a1f8f2091539e27bc6729597f59e35357ed2cb2c7c29cad74
design_refs:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-004-runtime-update-failures.md
  - docs/decisions/DCR-017-ui-integration-boundary.md
---

# 订阅导入与手动更新 UI 接线

本候选落实需求 §13–14、§22、§69–72 的当前订阅 UI 切片，以及 DCR-004 的失败行为。
现有解析、归一化、Provider 原子替换与 JSON Store 可以复用；当前 command 没有订阅入口，
Subscription 只有 id/name，schema 为 V3。以下 IPC、持久字段与 V3→V4 迁移须在明确批准后实施。
本文件未修改已冻结 Foundation；为此前尚未落地的订阅 UI 接口给出首次具体边界。

## 用户操作与参考映射

- 参考 Clash Verge Rev `src/pages/profiles.tsx` 的 URL 导入栏、新建入口和卡片列表，
  `src/components/profile/profile-viewer.tsx` 的导入对话框，`profile-item.tsx` 的更新时间和更新按钮。
- Veyra 支持输入订阅 URL，以及粘贴/选择文件导入已有解析器支持的 Clash、sing-box、URI 文本。
  文件使用浏览器用户主动选择的 File API 读取内容；不向后端传路径，不授予任意文件读取能力。
- 卡片显示名称、来源类别、节点数、最后成功更新时间及手动更新按钮；有有效 userinfo 时显示流量/剩余量与到期时间，不展示带凭据 URL。
  远程订阅一键更新，手动导入订阅通过再次粘贴/选择文件替换；不默默重新读取旧文件路径。
- 沿用已验收字体、浅深主题、固定 200px 左栏和 520px 最小窗口，右侧卡片从多列重排为单列。
  不复制参考项目的自动重试、订阅切换即重启内核、脚本编辑、定时更新或批量操作。
- 导入/更新完成只表示保存成功，提示“订阅已更新，运行配置未切换”。首页已有手动启动负责从
  当前保存状态生成配置；本 Task 不新增 Apply/Reload。Compiler 失败继续使用 DCR-004 的日志/Toast。

## 交互细化（按本地源码对齐）

本轮用户明确要求操作流程对齐。具体出处与能力差异见
`.sdlc/evidence/TASK-011/ui-reference-003.md`，不以“参考风格”代替操作序列。
顶部保留 URL 快捷导入栏及“导入 / 新建”按钮，Enter 忽略输入法组合；快捷导入不弹出必填名称表单。
“新建”另开约375px表单，默认远程，远程/本地选择；名称可空，本地选文件回填未填写名称。
前端向现有IPC提交前补安全默认名“远程订阅”或“本地订阅”，文件名超过80字符时生成范围内显示名；
不改变后端最终1–80字符校验，也不从带凭据URL派生名称。粘贴文本放在本地导入内，非额外三步向导。
底部“保存 / 取消”；成功关闭并清空、失败保留输入、取消释放本次内容。输入和控件继承既有字体。

卡片网格采用参考 minmax(260px,1fr)/8px间距，右上刷新图标在途旋转禁用且stopPropagation；
同一更新动作也从右键菜单进入。时间采用相对值和精确时间提示，流量采用已用/总量与进度、到期日期。
没有激活业务时不伪造卡片激活选中条；当前未交付的编辑信息/删除/批量更新/原文编辑等单独记录
能力差异，不将无效按钮放入产品。后续交付这些能力时仍按参考入口补齐。

顶部空输入粘贴图标必须是有效操作；优先验证用户点击触发的既有 WebView Clipboard API。
参考依赖原生clipboard插件，当前三权限候选不包含剪贴板读取；若现有API不能提供等价行为，
须把最小原生读取方案并入具体候选后审批，不能默默新增依赖/权限，也不能自动读取剪贴板。
普通Ctrl+V始终可用，但不把它当参考粘贴按钮已完成的证据。
## 三个封闭 IPC 与权限候选

仅 main 窗口增加下列自有权限；保留已有权限，不增加通用 HTTP、文件、Shell 或事件发送能力。

| Command / permission | 请求 | 返回 |
| --- | --- | --- |
| `list_subscriptions` / `allow-list-subscriptions` | 无 | 订阅安全摘要列表或封闭错误 |
| `import_subscription` / `allow-import-subscription` | `{name, source}`，source 为 `{kind: remote, url}` 或 `{kind: manual, content}` | 已提交条目安全摘要或封闭错误 |
| `update_subscription` / `allow-update-subscription` | `{id, content?}`；remote 禁止 content，manual 必须提供 content | 已提交条目摘要、`contentChanged` 或封闭错误 |

摘要仅含 id、name、sourceKind、nodeCount、lastSuccessAtMs，以及可空的已校验 traffic（upload/download/total/expireAtMs）。未知流量/到期时间显示“未知”，不伪造零；剩余量仅在三个所需计数均有效时计算并最低为零。业务错误为固定枚举：
invalidInput、busy、fetchFailed、parseFailed、normalizationFailed、validationFailed、saveFailed、
stateUnavailable、notFound、cacheUnavailable、identityFailed。错误文案由前端固定映射，底层错误与原始输入不跨 IPC。
本地失败只显示一次 Toast；日志为封闭阶段/类别，不打印请求、响应、URL、路径、节点凭据或错误链。
名称去首尾空白后 1–80 字符、URL 最多 8192 字节、输入/响应正文最多 4 MiB；UI 与后端均做边界校验。
无效/空/部分可解析批次整体失败。额外未知字段和 source/content 不匹配均拒绝。

实现接线范围：`src-tauri/build.rs` command 清单、`src-tauri/capabilities/default.json`、
`src-tauri/src/commands.rs`、`src-tauri/src/lib.rs`、新增 `src/lib/subscriptions.ts` 与订阅页组件。
生成权限只由 Tauri build 生成，不手改生成校验文件。

## 持久数据与迁移

在 Subscription 增加类型化 source（remote URL / manual）、last_success_at_ms 可空值和可空 HTTP
元数据。HTTP 元数据包含 etag、last_modified、subscription_userinfo 与 content_disposition；
只有通过固定语法/长度校验的元数据进入状态。userinfo 使用可空 upload/download/total/expire 数字，
content-disposition 只保留安全显示文件名，不作为路径；其它异常非关键响应头忽略，不保存原始头。
来源 URL 可能包含凭据，始终位于现有私有状态目录；安全列表/错误/日志不回显它。
手动内容在本次解析后释放，不持久化一份重复的原始订阅；更新时用户重新选择内容。

存储升为 V4，经既有 V1→V2→V3 后追加 V3→V4 内存迁移。旧订阅一律 source=manual、时间=null、
HTTP 元数据=null；不猜历史 URL 或成功时间，保留所有订阅/Provider/节点/出口组/路由身份和引用。
沿用升级前备份、完整校验、原子保存与当前私有目录权限。任一迁移失败保留旧有效文件，不清空重置。
旧版本不支持 V4，不设计反向自动迁移；回退必须使用既有升级前 V3 备份且停止新版本写入。

首次导入在真正没有 state.json 及其备份时，允许由用户导入创建 AppState::empty 的候选；读取错误、
损坏或无有效恢复状态不能当首次使用。候选同事务创建 Subscription、Provider 和节点，
为 Provider 创建既有类型的隐式出口组，default_target 保留 Unconfigured（不替用户选择默认出口）。
列表在首次使用时返回空，不主动持久化空文件；首次失败不留下半成品订阅。
新导入的身份全部由后端生成：使用已有 getrandom 一次取得 16 字节随机值，编码为小写十六进制，
分别以 sub-、provider-、pool- 前缀产生本次 Subscription/Provider/隐式出口组 ID；在提交锁内检查
候选中对应 ID 均未存在。熵源失败或碰撞返回 identityFailed，整批不保存，不循环重试。
NodeId 继续使用 normalize_nodes 对稳定 ProviderId 与节点参数的既有算法；更新不重新生成上述 ID。
一次导入成功后重复执行新的导入视为新订阅（允许同内容多个来源条目），不按名称/URL猜测合并；
UI 在 pending 时锁定提交，成功后关闭并清空输入。响应丢失时先手动刷新列表核对，不自动重发导入。
不引入幂等键、去重持久表或重试队列；碰撞/熵源失败用可控测试输入证明无半提交。

## 原子更新、并发与运行态

复用 `ProviderReplacementService` 的 normalize→clone→validate→save→swap 事务，
把导入的新 Subscription/Provider/隐式组、HTTP 元数据和成功时间与节点变更放在同一个候选提交。
更新保留 Provider 身份；引用已选节点若失效则按现有整体校验拒绝，不自行更换选择或修复路由。
解析/归一化/引用校验/保存失败时旧节点、时间和缓存校验器均保持。

所有真实状态访问都经过同一个短门控，包括远程更新发请求前的 source/validator/cache 加载、首次使用的 state/backup 存在性判断与 load、list、Start load、迁移/恢复以及最终提交。远程操作固定为“锁内加载并取得来源/缓存快照 → 解锁并请求网络 → 再锁内重新加载并核对来源/缓存仍匹配 → 提交”；来源或缓存已变化时返回 busy，不把旧响应用于新来源，也不自动重试。首次文件缺失判断与首次创建提交在同一门控内重新确认，避免覆盖并发产生的有效状态。

订阅写操作同一应用内最多一个在途；第二次调用立即 busy，不排后台重试。网络请求不占用状态
文件读写锁、不影响当前内核采样或停止。拿到有效批次后，再在状态事务锁内重新加载最新完整状态
并提交，避免旧快照覆盖。列表和 Start 的 state load（含可能的迁移/恢复写入）与订阅提交共享一个
短临界区，防止固定临时文件、备份和迁移写入相互覆盖；锁忙可沿用封闭 busy，退出前释放。
不把网络等待或整个 sidecar 启停放在该锁内，不把两个独立 AppState 内存副本当同一个事务源。
只为这三个真实调用点共享门控，不新增通用工作队列、持久作业或后台调度器。

订阅操作不调用 Runtime Start/Stop/Apply、System Proxy 或 TUN。并发手动 Start 只使用其取得的
完整已提交快照，不把稍后完成的订阅写入冒充该次运行配置。成功时间在候选中生成并只随保存成功
对外可见；保存失败不会刷新页面时间。

## HTTP 与 304

使用已经安装的 reqwest 0.12.28 / Rustls，TLS 校验保持开启。远程地址必须 HTTPS；HTTP 仅允许 URL host 为字面量 127.0.0.1 或 [::1] 的本机受控服务，且不接受 userinfo/query/fragment。所有来源均拒绝 URL userinfo（用户名/密码），HTTPS 查询参数可用于订阅 token。此规则逐跳用于重定向，不能通过跳转绕过。明确禁用系统代理
自动推断（订阅请求不依赖正在运行的内核）。连接超时 10s、整次请求 30s、最多 5 次重定向，
不允许 HTTPS 降级到 HTTP；不跨 origin 转发条件请求头或授权头。正文按 chunk 累计 4 MiB 上限，
不能仅信 Content-Length；不加 HTTP 自动重试，不新增依赖或改锁文件。

已成功提交的该订阅 Provider 节点集合是有效缓存；etag/last_modified 与它同事务保存。
只有来源与缓存一致、节点集合非空且完整校验通过才发送条件请求。304 复用这份已提交节点，
按同样整体校验/保存更新成功时间，返回 contentChanged=false；不声称节点变化或运行配置生效。
没有有效缓存的 304 返回 cacheUnavailable，不清空节点、不伪成功、不自动追加一次请求。
200 必须完整解析归一化后再提交，更新失败不得提前覆盖旧校验器。HTTP 失败不记为 Compiler 失败。
远程订阅 URL 的 path/query 常携带 token；私有文件权限与日志脱敏无法保护明文传输，所以使用上述少量 URL/redirect 校验封闭真实明文泄漏路径。本机 HTTP 只服务明确回环地址，禁用系统代理，不能把凭据交给外部网络；不增加代理发现、DNS 探测、证书安装或网络监控机制。测试覆盖公网 HTTP、URL userinfo、回环 HTTP 查询参数、HTTPS 降级/非法重定向拒绝，以及合法 HTTPS 和无凭据回环 HTTP。

## 验证与变更影响

- UI：用户导入/更新流程、列表加载/空/忙碌/错误、重复点击、失败保留时间、切页期间单次结果、
  520/960 浅深截图；使用 mockIPC 证明交互，用隔离原生最小链证明实际接线，明确区分证据来源。
- Rust：使用隔离 StateStore 与本机受控 HTTP fixture 验证 200/304/超时/超限/失败、原子保存、
  旧状态和时间不变、迁移保持身份、并发 busy 与 Start load/提交互斥，以及 userinfo 缺失/非法/超量的诚实展示。只验证应用自身契约。
- 权限：main 三个精确 command、新请求 DTO 正反例、错误脱敏；无任意文件或网络工具授权。
- 执行仓库 pnpm lint/test/build；Rust 精确非零测试、cargo fmt/clippy，均外层限时。
  不访问用户真实订阅或生产数据，不运行上游协议矩阵。
- 影响范围：Domain Subscription、Storage V4/migration、Application 订阅事务与 Start 读状态门控、
  HTTP 取订阅、Tauri 三个 IPC/权限、前端订阅页及相关测试。既有运行 worker/协议/编译输出不改。
- 本候选仅需已有依赖。替代方案是暂留不可操作页壳，不能满足持久更新 UI；只在内存保存 URL/时间
  会导致重启后不可更新，故建议批准上述最小持久数据和封闭接口。需要用户明确批准后才实施。
