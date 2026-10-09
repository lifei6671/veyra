# OBG-P5-06 订阅分享交付与局部验收

## 2026-10-09 最终产品架构与验收收口（当前）

**DONE / PASS_WITH_ACCEPTED_TECHNICAL_DIFFERENCES。** 基于 `820526cf02fb3ccc26fd87a7c600109149d4e3b0` 的本地增量；本卡 Core/Config + GPUI owner/预约释放，不 push、不领取下游。以下旧决策、失败和候选证据全部保留为历史，不覆盖本节。

### 产品语义与边界

Veyra 自身的 ShareService/Axum 独立监听 HTTP 端口；GET `/sub/{token}` 返回 sing-box 节点订阅 JSON；二维码编码分享 URL，供其它客户端获取并导入。`listen` 是实际 HTTP bind socket，`host` 是对外 URL authority；URL 使用保存的 host、协议和 token，不把 `0.0.0.0` 当成客户端地址。sing-box 入站只处理代理连接，P5-04/P5-05 独立负责其配置。**解除先前错误的 P2-06 端口依赖阻断。** ShareService、SubscriptionShare、Axum 和独立端口配置保留；本轮不修改 RuntimeService/Platform/Helper/IPC/公共 DTO、Core 生产实现或 Cargo。P2-06 保持 DOING、原 owner/写范围不变。

新增生产 Service/Store 测试保护 wildcard bind 与 advertised host 分离：`listen=0.0.0.0:端口`、`host=192.168.1.20:相同端口`，本地 GET 返回200/JSON；通配 advertised host、端口0拒绝且保旧。真实 GUI 又使用当前 en0 地址与相同独立端口保存，通过本机 LAN 接口地址 GET200；见 `final-lan-interface.json`。这是同一 Mac 对自身 LAN 接口的访问，**不是第二台设备或公网可达性证明**；跨设备/网络组合留在 P5-07。仅 HTTP，没有证书时不开放 HTTPS。

字段三语说明明确“分享监听地址”：127.0.0.1 仅本机；局域网可监听0.0.0.0，右侧填写本机局域网IP与相同端口。线上没有此字段，用户已接受保留；最终产品架构再次确认独立端口能力。

### 启动根因与本轮修复

前轮生产80轮删除重启在第36轮捕获 StateAccessGate `Busy`：Runtime 独立线程首次 Refresh 与全局 snapshot 竞争；不是磁盘读失败、迁移失败或已证实的运行态恢复错误。既有修复将首次 Runtime Refresh 放在 Accepted/Ready 快照后，本轮真实生产80轮回归再次执行通过。

本轮独立复核在自有 Store FIFO 读取背压下又定位一个真实导航分支：启动尚 Busy 时进入订阅页，页面权威快照先到，`leave_page()` 作废旧全局请求并把无快照的 Busy 置 Idle；随后只赋 snapshot，没有 Ready/首次 Runtime 观测。旧回调被正确拒绝却没有收敛路径，原 `loading-final-busy.png` 左下仍为“正在读取服务状态”。

最小修复仅 Desktop：订阅页面回调先由 `view.complete` 验证请求归属；被接受的快照通过 `StateBridge::accept_page_snapshot` 作废旧全局读取、设置快照并进入 Ready，再经既有 once-only 判定启动首次 Runtime Refresh。旧页面回调仍返回None；旧全局回调仍拒绝；无 sleep、延时注入、自动重试或吞错。新增回归覆盖“Busy→导航→页面成功→两条旧global回调→仍Ready且Runtime只启动一次”。正式 GUI 慢启动复验无分享错误、Runtime Refreshed；本次实际由global先完成，精确page先完成分支由测试覆盖，二者不混称。最终删除后无导航重启在 Overview 直接 snapshot_loaded=true、Runtime request1 Refreshed。

### 本轮实际工程验证

| 检查 | 结果 |
| --- | --- |
| Core `shares` | 10 PASS，含新 wildcard/host 生产HTTP回归 |
| Desktop `sharing` | 5 PASS，含生产80轮删除重启；末次源码后复跑 |
| Desktop `subscriptions` | 7 PASS，与sharing重叠3项；加载修正后复跑 |
| Desktop `state_bridge::` | 10 PASS，含本轮页面先完成启动回归 |
| Core/Desktop all-targets Clippy、正式Desktop build、workspace fmt、diff check | PASS，末次源码后执行 |
| 指定旧 Tauri fmt | PASS |
| 指定旧 Tauri Clippy | FAIL101：Windows LICENSE 资源缺失；既有libcronet.dll失败另保留，不扩大本卡范围 |

合计 **Core10 + Desktop19 = 29个唯一定向测试**，不累加重复执行，不把过滤零项计为通过。本轮没有重跑历史 `real_store_empty_error_retry_and_late_delivery`，不计入29。命令/退出码/原始日志：`checks.json`、`checks-final.json`、`checks-qr-final.json`、`checks-loading-final-valid.json`、`checks-loading-review.json`、`checks-startup-page-final.json`、`legacy-checks.json`。中间缺失token编译失败、误用Desktop `--lib` 无target失败及无效忙碌采图原样保留，不改写成PASS。

### 真实 GUI / 功能链路

证据均在 [architecture-final-20261009](evidence/p5-06-sharing/architecture-final-20261009/)，生产链路仍为 GPUI→Worker→ShareService→SnapshotService→JsonStateStore。仅自有状态目录/端口/测试Token，不操作用户系统代理/TUN/管理员配置。

| 场景 | 本轮实际结果与证据 |
| --- | --- |
| 真端口冲突、失败保旧、失败后刷新、原草稿重存 | 占用自有18786，旧18787 HTTP200/2节点；失败保草稿，刷新不抹去错误；释放后同草稿保存200、18787关闭。`final-retry.json`、`final-save-failed.png`、`final-failed-refresh.png` |
| 轮换取消/确定 | 取消保留旧URL；确定后旧token404、新200。`final-rotation.json`、`final-rotate-confirm.png` |
| LAN监听/URL | wildcard监听、en0 advertised同端口，直接GET200/2节点；旧18786关闭。`final-lan-interface.json` |
| 正常托盘退出 | 用户亲手退出 Architecture Final；进程消失且18785/86/87关闭。`final-tray-exit.json`；绑定签名SHA `6fc4e13fb92da02d93e0576f143d5e4b4855050ff6a4991fa6b1eb7fb9cbe805`，不伪称对后续每个SHA重复操作 |
| 保存中 | 真实自有HTTP订阅刷新占用生产worker，分享Save排队；系统截图显示禁用Save与spinner。`final-busy-valid.png`、修正spinner后的`loading-final-busy.png`；未改变产品超时。后一轮刷新成功引起版本变化，保存正确拒绝，刷新版本后原草稿再Save成功，不计为首次Save成功 |
| 加载与旧列表重载 | 自有Store FIFO背压；先保留分享卡、订阅区spinner；已有列表重载时旧卡隐藏，模型未清空；加载中可打开新建且未选择订阅时Save禁用。`loading-final-first.png`、`loading-final-retained-list.png`、`loading-final-new-before-ready.png` |
| 最终构建编辑/HTTP/删除 | `accepted-edit.png`、`accepted-http.json`200/2节点、`accepted-delete-confirm.png`具体名称、`accepted-delete.json`磁盘shares0/18784关闭、`accepted-empty.png` |
| 最终慢启动/删除后无导航重启 | `accepted-slow-start.log`、`accepted-slow-start-ready.png`；`accepted-deleted-restart.log`、`accepted-deleted-restart.png`、`accepted-restart.json`，Overview直接Ready/Runtime Refreshed |

### 150%浅色逐态视觉结论

固定 macOS、用户当前150%显示设置、浅色、逻辑内容1280×720、scale factor2。真实在线 `http://192.168.1.6:3036` 为目标；仓库React源码/CSS保留用于语义及级联溯源。在线目前为带data-v属性的自绘Dialog，仓库React仍是window.confirm，差异明确分列。在线独立标签仅以自有fixture拦截响应，保存/删除响应也本地完成，未写线上后端；结束恢复视口并关闭标签。系统 `screencapture` 取得真实窗口图，没有合成参考图；无新增屏幕录制授权。

本輪逐项修正颜色、字体CSS字重、卡片/输入边框、checkbox SVG polygon、radius、QR quiet zone、disabled/hover、局部加载布局及spinner。分享标题16/24 semibold、说明12/16，主色/文字/线色分别来自在线计算值；编辑框768、栏间20、双栏1:0.9、字段14、按钮32、checkbox20、QR176、确认框512；输入radius9.3、QRradius16。spinner复用线上24px圆路径、stroke3、2秒旋转/1.5秒dash，不再使用旧静态/不同色图形。所有固定值沿用tokens与页面局部palette，不覆盖无关页面主题。

| 状态 | 在线原图 | GPUI有效图 / 结论 |
| --- | --- | --- |
| 分享列表 | online-list.png | final-list.png；标题/说明/动作/行内容与局部颜色对照通过 |
| 新建 | online-new.png | final-new.png；空值、禁用保存、输入/取消通过 |
| 编辑 | online-edit.png | qr-final-edit.png、accepted-edit.png；几何/颜色/字段/按钮通过，字体与模糊见下方已接受差异 |
| 多订阅选择 | online-selection.png | final-selection.png；20px选框、勾选几何/行边界与选择行为通过 |
| 链接与QR | online-edit.png | qr-final-edit.png；Vision分别解码为完全相同URL；black bbox线上319px/GPUI320px（2x），quiet zone已修复；不同QR mask不宣称bitmap相同，见qr-final-decode.json/qr-margins.json |
| 加载 | online-loading-matched.png | loading-final-first.png、loading-final-retained-list.png；分享卡保留、列表区136px/24px spinner；旧整页loading候选不计通过 |
| 保存中 | online-saving.png | loading-final-busy.png；spinner/disabled按钮对照，真实生产worker排队 |
| 保存失败/刷新/再次保存 | online-save-failed.png | final-save-failed.png、final-failed-refresh.png；原草稿可修改、重试；GPUI明确常驻错误与刷新入口保护用户要求，不把在线瞬态反馈冒称一致 |
| 删除/轮换确认 | online-delete-confirm.png、online-rotate-confirm.png | final-delete-confirm.png、final-rotate-confirm.png；具体名称、512px布局、取消/确定按钮及实际行为通过 |
| 删除空态 | online-empty.png | final-empty.png、accepted-empty.png；卡片标题、说明、添加及居中空文本通过 |

用户明确接受的技术差异：① 在线局部 `backdrop-filter:blur(10px)`，GPUI当前后端只有窗口级材质，半透明弹窗会透出清晰背景文字；② 在线MiSans4.003网页分片可变字体，GPUI使用已登记MiSans4.009静态字体，标题笔画仍较粗；不把CSS weight相同说成字体像素相同。用户接受监听字段差异，最终架构要求保留。QR编码mask差异以同URL实际解码和尺寸/quiet zone核验；不要求二维码位图相同。

仓库React原生window.confirm的macOS系统截图仍保留于前轮 `final-20261009/react-window-confirm-delete-system.png`、`react-window-confirm-rotate-full-system.png`；原生来源标题/系统字体/默认确定焦点属于浏览器系统区域，与线上/GPUI自绘Dialog不等价。线上和GPUI默认Enter未触发操作的观察保留，不声称所有键盘路径都通过；取消/确定实际鼠标行为已验。最终结论为本卡应用自绘区域逐态对齐通过，保留用户接受的技术差异，**不宣称像素完全一致或全应用95%相似度**；其它页面、主题/缩放和跨模块组合不由本卡截图代验。

### 构建、独立复核、交付

最终正常构建，无产品延时注入，ad-hoc executable SHA256：`879c5d775ff0adce3c0a80fc313df79c32751ba971632321f8d20485ba652b0b`，见 `accepted-build.json` / `accepted-bundle/`。此前6fc4功能/托盘版、b52b二维码版、f222加载版分别保留构建身份；末次仅Desktop启动快照收敛改变，既有Service/端口/退出生命周期实现未变。最终版另实测编辑/HTTP/删除/无导航重启，不把旧截图替换为新SHA截图。

独立agent按 code-delivery-review 只读最终diff、测试原始日志、构建身份及真实截图；发现的整页加载、旧列表与spinner共显、页面快照抢先导致启动不收敛三项P2均已修复复核。最终无剩余可执行P0–P3发现；其没有亲自操作GUI，图像/日志结果复核与主agent实际操作分开记录。分享restore与Runtime后续try_lock竞争没有新失败复现，不凭假设改受保护锁/Runtime合同。见 `independent-review.md`。

所有自有测试监听18783–18787关闭、临时FIFO恢复普通文件、浏览器自有标签关闭；最终测试进程使用SIGTERM清理，单列为cleanup而非托盘正常退出。自有状态、截图、日志、失败候选与各版bundle全部保留；截图SHA256清单为 `screenshots-sha256.json`。证据按仓库约定本地留存、不强制纳入Git。旧Windows资源缺失独立记录；没有本卡未完成阻断。68卡重算：DONE22 / DOING1 / ACCEPTANCE0 / READY5 / TODO33 / DEFERRED7；READY仍为P0-08/P2-05/P3-01/P4-03/P5-04，未启动。P5-06完成后释放owner，P2-06不变。


## 2026-10-09 启动修复与最终复核（历史；监听字段决策已被下述最终架构覆盖）

**ACCEPTANCE，owner 保留。** 基于 `7a6eb7dc6ede8b7d68a6eee7c81a67a0b5926176` 的本地增量；不 push。P2-06 继续 DOING、原 owner/Runtime/Platform/Helper/IPC/公共 DTO 范围不变。本轮没有修改 Core、Cargo 或用户系统代理/TUN/管理员配置。

### 启动故障根因与修复

使用自有目录中的生产 SubscriptionManager → ShareService → SnapshotService → JsonStateStore 创建订阅、保存分享、删除分享，再重建 AppServices。旧启动顺序80轮在第36轮真实失败：`StorageFailed / 数据未能保存或读取 / detail: Some(Busy)`，同时 Runtime Refresh 成功，磁盘字节未变化；见 [原始失败](evidence/p5-06-sharing/final-20261009/repro-startup.log)。

`AppServices::new` 的 shares.restore 与全局 snapshot 在同一单 blocking-worker 队列中串行；AppView 同时向 RuntimeService 的独立线程发出首次 Refresh，后者读取同一 StateAccessGate。SnapshotService 的 try_lock 在竞争时返回 Busy，StateBridge 将这一实际失败保持为 Error；没有新读取就不会自动转 Ready。这是启动读顺序的锁竞争，不是磁盘读取/迁移失败，也没有证据表明是运行态恢复失败。

最小修复仅在 Desktop 编排：首次全局快照 Accepted 且 Ready 后才调用既有 Runtime Refresh；错误/陈旧回调不启动，后续回调不重复启动。AppView 与测试共用 `begin_initial_runtime_refresh`。没有新增 sleep、延长延时、吞错误或修改 Gate/Runtime 契约。生产删除重启80轮均 Ready；另一个测试保护 Error/陈旧回调/成功重试/只启动一次。最终 GUI 删除重启仍停留 Overview，日志顺序为 `Snapshot Accepted → snapshot_loaded=true → Runtime Refreshed`，无需导航。

### 当前产品阻断：监听字段语义

用户最初接受保留“监听地址”布局差异，随后明确纠正：**此字段应配置 sing-box 入站端口，需协调 owner**。后者是当前有效要求，不能把先前接受解释为批准独立 HTTP 语义。

现有 `SubscriptionShare.listen` 实际由 ShareService 的 TcpListener/Axum 绑定以提供 `/sub/{token}`；并非 sing-box 入站。两个服务不能直接绑定同一端口。停止相交修改，不能通过改标签冒充入站配置。最小协调：P2-06 owner 确认入站字段、保存/应用接口及公共 DTO 归属；P5-06 消费已确认接口，同时明确分享 HTTP endpoint 的独立监听/复用方案。未发送其它任务消息，未占用或修改其范围。当前测试只证明已有 HTTP 实现，不证明新的入站配置要求。

### 测试与构建

| 检查 | 本轮实际结果 |
| --- | --- |
| Core shares | 9 PASS |
| Desktop sharing | 5 PASS，含生产80轮删除重启 |
| Desktop subscriptions:: | 7 PASS，与sharing重叠3项；最后视觉修改后再次执行 |
| Desktop state_bridge:: | 9 PASS，新增1项 |
| Desktop real_store_empty_error_retry_and_late_delivery | 1 PASS |
| Core/Desktop all-targets Clippy、Desktop build、workspace fmt | PASS；最后视觉修改后再次执行 |
| 指定旧Tauri fmt | PASS |
| 指定旧Tauri Clippy | FAIL 101，Windows libcronet.dll缺失；历史LICENSE失败独立保留，不扩范围 |

合计 **Core9 + Desktop19 = 28个唯一定向测试**，没有将重复或过滤零项计为通过。命令/退出码与原始日志见 [checks](evidence/p5-06-sharing/final-20261009/checks.json)、[checks-final](evidence/p5-06-sharing/final-20261009/checks-final.json)、[checks-online](evidence/p5-06-sharing/final-20261009/checks-online.json)。初始fixture Invalid、旧启动Busy、旧翻译key测试及中间编译失败均保留，不改写历史结果。

最终正常构建、无分享延时注入的 ad-hoc executable SHA256：`018f13a57898ea4d3f05198c0165a5c9acf472d75677518ae239077c35d31b91`。Cargo原始SHA256：`8d2a3c42f27a89b6eb19cc5164a1b247408c50320215dea4f98e9b3235ed3e54`。见 [build.json](evidence/p5-06-sharing/final-20261009/build.json)、[final-bundle](evidence/p5-06-sharing/final-20261009/final-bundle/)。此前 build4/pre-online 构建及其GUI图分开保存，不绑定到最终SHA。

### 真实 GUI 与功能结果

固定 macOS、现有150%显示设置、Light、逻辑内容1280×720，scale factor2；不更改用户显示/系统网络设置。两条自有订阅、loopback测试端口及随机测试Token，不含用户凭据。

| 场景 | 本轮结果与范围 |
| --- | --- |
| 启动全局读取 | 最终构建 Ready，无页面导航；删除后重启同样通过，[日志](evidence/p5-06-sharing/final-20261009/gui-delete-restart-final.log) |
| 真端口冲突失败保旧、刷新保错、释放后原草稿保存 | build4生产链路实际操作；旧文件字节不变、旧18787 HTTP200；释放后18786 HTTP200、18787拒绝；[失败](evidence/p5-06-sharing/final-20261009/gui-bind-failure.json)、[重试与重启](evidence/p5-06-sharing/final-20261009/gui-retry-restart.json) |
| 轮换确认 | build4真实GUI操作，旧URL404、新URL200，Token改变；[结果](evidence/p5-06-sharing/final-20261009/gui-rotation.json) |
| 托盘正常退出 | 用户操作测试实例并回复“已退出”；PID67396退出、18786/18787拒绝且可重绑，[结果](evidence/p5-06-sharing/final-20261009/gui-user-tray-quit.json)；后续构建替换用SIGTERM，单列且不冒充托盘证据 |
| 删除、空态、删除后重启 | 最终构建shares为空，18786/18787拒绝，重启无需导航Ready；[删除](evidence/p5-06-sharing/final-20261009/gui-delete-online-final.json)、[重启](evidence/p5-06-sharing/final-20261009/gui-delete-restart-online-final.json) |
| 新建、多选、保存禁用 | 最终GUI可选两项、空标题保持禁用；当前字段语义阻断，未继续保存新入站配置 |
| 加载、保存忙碌完整最终逐态对照 | 本轮最终构建未取得对应线上原图，NOT_RUN；10-08旧证据保留但不冒充本轮完成 |

### 视觉事实、原生截图与差异

用户指定在线 `http://192.168.1.6:3036` 为一比一目标。实际页面为带 `data-v-*` 的自绘Dialog，而仓库 `SubscriptionSettings.tsx` 仍用 window.confirm，二者不是同一视觉基线。线上使用独立标签页、只对该标签拦截只读分享/订阅响应以提供相同自有fixture，未写线上后端。实际DOM/CSS取值后修正：编辑宽768、栏间20、比例1:0.9、字段14px、checkbox20px白底、QR176、按钮32px、确认框512px及名称模板；列表去掉额外框，名称/所选订阅同行，host及URL用Menlo。具体tokens集中定义，未改变全局组件外观。

本地真实 React 组件以隔离API fixture加载，没有替换确认实现；macOS `screencapture` 取得两张真实浏览器原生确认图，不用CDP截图，不合成图，无需新增屏幕录制权限。删除/轮换取消后返回页面。原生浏览器框有来源标题、系统字体及默认确定按钮，线上自绘框与GPUI Dialog为应用布局；不得声称三者逐像素相同。GPUI/线上确认默认Enter没有触发操作的观察记录，不扩大为全部键盘行为通过。

| 状态/来源 | 本轮证据 |
| --- | --- |
| 本地React原生删除/轮换 | [删除](evidence/p5-06-sharing/final-20261009/react-window-confirm-delete-system.png)、[轮换](evidence/p5-06-sharing/final-20261009/react-window-confirm-rotate-full-system.png) |
| 在线真实自绘删除/轮换 | [删除](evidence/p5-06-sharing/final-20261009/react-delete-native.png)、[轮换](evidence/p5-06-sharing/final-20261009/react-rotate-native.png)；历史文件名native仅表示系统截图方式，内容不是window.confirm |
| 在线编辑同订阅fixture | [在线](evidence/p5-06-sharing/final-20261009/online-edit-final-system.png) |
| 最终GPUI空态 | [空态](evidence/p5-06-sharing/final-20261009/gpui-empty-online-final-system.png)；最终列表/删除确认本次保存图误捕前台，标为无效、不计视觉通过，实际点击与删除磁盘结果另有记录 |
| 最终GPUI新建/多选 | [新建](evidence/p5-06-sharing/final-20261009/gpui-new-online-final-system.png)、[多选](evidence/p5-06-sharing/final-20261009/gpui-selected-online-final-system.png) |
| build4失败/刷新保错 | [失败](evidence/p5-06-sharing/final-20261009/gpui-save-failed.png)、[刷新](evidence/p5-06-sharing/final-20261009/gpui-refresh-retains-error.png) |

截图复核发现一批全屏截图捕获了前台Codex或未激活浏览器页，已保留为 `invalid-*` 并从有效清单排除。原生轮换在激活测试标签后用系统截图重拍并目视核验；GPUI新建/多选/空态/删除后启动用新进程当前窗口ID5306系统截图重拍并核验。最终列表与删除确认图未重拍，不以失败图冒充证据。

**视觉未通过最终验收，不宣称95%或完全一致。** 仍可见字体字重（尤其标题/按钮）、周围页面背景/节点区域、二维码编码/quiet-zone和部分控件细节差异；字体技术差异不能自动豁免肉眼可见偏差。最终编辑/加载/忙碌/失败整套相同状态对照尚未完成。监听字段应接入sing-box入站的新语义与owner边界先行协调，再继续逐态修正及复测。

### 独立复核与收尾

[独立Review](evidence/p5-06-sharing/final-20261009/independent-review.md)：独立agent只读最终diff与实际日志，未发现本轮新增P0–P2并发/状态/异步回调问题；确认未跨P2-06保留范围。其未独立执行GUI，不据此宣称视觉通过。用户新确认的字段语义冲突列为当前产品blocker。

自有应用/端口/React服务器关闭、临时node_modules链接移除、独立浏览器标签关闭及视口恢复；原图、日志、bundle及自有已删除分享的测试目录保留，[cleanup](evidence/p5-06-sharing/final-20261009/cleanup.json)。截图SHA256清单见 [manifest](evidence/p5-06-sharing/final-20261009/screenshots-sha256.json)。证据沿用仓库约定本地保留、不强制纳入Git；文档及源码按精确范围提交新本地commit。

当前最小剩余项：**P2-06 owner协调入站字段/服务归属；完成剩余逐态视觉修正和本轮最终构建验证**。P5-06维持ACCEPTANCE、不释放owner；68卡计数与Ready Queue不变。

## 2026-10-08 验收修正（历史）

**状态仍为 ACCEPTANCE，未释放本卡 owner。** 原提交 `9cf86ad860075c392b68766a73db4084e20f2256` 与下方首轮记录、候选图和失败证据全部保留。本轮只修正分享交互与局部外观；Core/Runtime/Helper/IPC/Runtime DTO 未修改，不操作系统代理、TUN 或管理员配置。

失败保存保留全部输入、订阅选择和 Token；再次点击“保存”提交真正的 `ShareCommand::Save`。原“重试”读取按钮改名“刷新”，读取成功不会清除失败的保存/列表操作错误，不会显示原操作成功。新增定向测试保护 Bind/Storage/NotFound 失败在刷新后仍保留、实际写操作成功才清错。页面 Load 完成后才读取/恢复分享，避免页面内部两个读取争抢 Store gate。

外观按实际 CSS 级联修正：736px 弹窗的1px边框/339px双栏、header20px行高、checkbox16px及浏览器margin、字段间距、HTTP控件34px、Menlo链接、QR164px及顶部2px、空态底部、列表标题/说明与新增按钮；分享 footer 使用全局 primary-button 的36px高度、11px文字、750字重、15px水平padding、11px圆角和绿色白字。保存禁用及spinner位于文本前；字段/取消在忙碌中沿用 React 可操作行为。错误复用已有通知中心并在监听说明下保留可刷新错误，白色不透明弹窗保持。

### 本轮验证结果

| 检查 | 数量 | 退出码 / 实际结果 |
| --- | ---: | --- |
| Core `cargo test -p veyra-core shares -- --test-threads=1` | 9 | 0 PASS，正式 Service/HTTP/磁盘测试 |
| Desktop `sharing` | 4 | 0 PASS |
| Desktop `subscriptions::` | 7 | 0 PASS，与上一行重叠3项；Desktop合计8个唯一测试 |
| Core/Desktop all-targets Clippy `-D warnings` | — | 0 PASS |
| Desktop build、workspace fmt check | — | 各0 PASS |
| 指定旧 Tauri fmt check | — | 0 PASS |
| 指定旧 Tauri Clippy | — | 101 FAIL；本轮首先报 Windows `libcronet.dll` 缺失；首轮 Windows `LICENSE` 缺失仍保留，未绕过/扩展修复 |

本轮 **Core9 + Desktop8 = 17个唯一定向测试**，过滤后均非零。日志及命令见 [checks-final.json](evidence/p5-06-sharing/closeout/checks-final.json)、[checks.json](evidence/p5-06-sharing/closeout/checks.json)。首轮 parser36/state40/compiler60 是历史回归，本轮未重复计数；Core源码未变。`block 0.1.6` 既有 future-incompatibility 警告保留。最终 `git diff --check` 见本轮收尾检查。

**正式无分享测试延迟构建（ad-hoc signed executable）SHA256：** `d8d65d1b3f8277310ea08c43401c3ce324dc7241a77c525d0f9b9f31f3173b49`。Cargo原始可执行文件SHA256 `6f0825c4249c3428177d42c10f4a1237d73337b6aa6ecba5c4233ce72772bcc8`。最终签名bundle作为本地证据保留在 [final-build](evidence/p5-06-sharing/closeout/final-build/)，身份见 [build.json](evidence/p5-06-sharing/closeout/build.json)；不把早期候选图绑定到此SHA。

### 最终构建的真实 macOS 操作

用户手动解锁后操作 `dev.veyra.p506.closeout`，使用自有目录和 loopback；150%显示设置未改变，Light，逻辑内容1280×720。所有 GUI 保存经过 UI → Worker → 正式 ShareService → SnapshotService/JsonStateStore。

| 操作 | 实际结果 / 证据 |
| --- | --- |
| 读取等待、Save忙碌 | 无分享延迟钩子。自有 FIFO 只对真实 Store read 施加背压；恢复原普通文件后才释放读取。取得加载和保存busy原图，完成后控件恢复。[final-loading.json](evidence/p5-06-sharing/closeout/final-loading.json) |
| 已有18787分享，另以自有socket占用18786，GUI改地址保存 | 真实 Bind失败；name/host/listen/选择/Token草稿完整；点击刷新后失败仍显示。旧磁盘逐字节不变、旧18787 HTTP GET200且含Selected-01。[signed-bind-failure.json](evidence/p5-06-sharing/closeout/signed-bind-failure.json) |
| 释放18786冲突，再点原弹窗保存 | 实际Save成功，磁盘更新、18786 GET200且只含指定节点，18787连接拒绝。正常退出再启动，重建Store/Service仍读取相同配置并GET200。[signed-retry.json](evidence/p5-06-sharing/closeout/signed-retry.json)、[signed-restart.json](evidence/p5-06-sharing/closeout/signed-restart.json) |
| 正常托盘退出 | 用户明确执行测试实例托盘“退出”并回复“已退出”；随后核验PID不存在、18786连接errno61、同端口重绑成功，无信号。与早期SIGTERM及debug快捷键分开。[signed-normal-user-quit.json](evidence/p5-06-sharing/closeout/signed-normal-user-quit.json) |
| GUI删除确认、删除、重启 | 真实确认后列表空、磁盘shares=[]、最后18786监听退出。新进程/Store回读仍空且无分享监听。[signed-delete.json](evidence/p5-06-sharing/closeout/signed-delete.json)、[signed-delete-restart.json](evidence/p5-06-sharing/closeout/signed-delete-restart.json) |
| 同URL二维码 | macOS Vision分别从最终React/GPUI编辑原图解码，两者均为同一个完整18787 URL；码矩阵允许编码库不同mask，未拿不同Token对比。[qr-final-decode.json](evidence/p5-06-sharing/closeout/qr-final-decode.json) |

实际监听字段仍是本机bind地址，展示host仅生成链接；只验证127.0.0.1，不宣称LAN/公网或HTTPS。初轮真实写入失败保旧、token轮换404/200、停启及复制证据继续有效，没有以视觉harness替代正式网络测试。

### 视觉原图、差异与剩余验收

React仍加载实际 `SubscriptionSettings.tsx` / `openbox.css` / MiSans-VF；临时harness只供相同合成订阅/Token/URL，白色背景用产品已有背景变量注入，不改基线源码。新建使用同一个原生随机草稿Token。原图未改像素，GPUI含32px原生标题栏且为2倍像素，React为1倍；对照按相同逻辑内容坐标。

| 状态 | GPUI原图 | React原图 |
| --- | --- | --- |
| 列表 | [列表](evidence/p5-06-sharing/closeout/gpui-list-final.png) | [列表](evidence/p5-06-sharing/closeout/react-list-final.png) |
| 编辑/选择/URL/QR | [编辑](evidence/p5-06-sharing/closeout/gpui-edit-final.png) | [编辑](evidence/p5-06-sharing/closeout/react-edit-final.png) |
| 新建空草稿 | [新建](evidence/p5-06-sharing/closeout/gpui-new-final.png) | [新建](evidence/p5-06-sharing/closeout/react-new-final.png) |
| 新建选择及输入 | [选择](evidence/p5-06-sharing/closeout/gpui-new-selected-final.png) | [选择](evidence/p5-06-sharing/closeout/react-new-selected-final.png) |
| 加载 | [加载](evidence/p5-06-sharing/closeout/gpui-loading-final.png) | [加载](evidence/p5-06-sharing/closeout/react-loading-final.png) |
| 忙碌 | [忙碌](evidence/p5-06-sharing/closeout/gpui-busy-final.png) | [忙碌](evidence/p5-06-sharing/closeout/react-busy-final.png) |
| 保存失败 | [真实Bind](evidence/p5-06-sharing/closeout/gpui-bind-failure-final.png) | [失败视觉基线](evidence/p5-06-sharing/closeout/react-bind-failure-final.png) |
| 删除后空态 | [空态](evidence/p5-06-sharing/closeout/gpui-empty-final.png) | [空态](evidence/p5-06-sharing/closeout/react-empty-final.png) |
| 删除/轮换确认 | [删除](evidence/p5-06-sharing/closeout/gpui-delete-confirm-final.png)、[轮换](evidence/p5-06-sharing/closeout/gpui-rotate-confirm-final.png) | 原生confirm阻塞截图接口，未获得原图，**未记通过** |

几何核对已覆盖736宽、339双栏/24间距、32字段、34协议/复制、164二维码、16checkbox和footer；截图清单/hash见 [final-screenshots.json](evidence/p5-06-sharing/closeout/final-screenshots.json)。**没有宣称整体95%或完全一致。** GPUI静态MiSans与React VF字重/光栅、白色不透明modal与React透明/背景blur、周围页面背景和节点卡的差异仍能观察到；不能以大致几何一致替代最终逐态视觉结论。节点卡/实时健康等已有P2行为不是本轮分享业务结果，不伪造未知为0。

剩余项（本卡保持ACCEPTANCE）：

1. React删除/轮换原生confirm的实际参考原图仍缺。点击后CDP截图/取消/关闭均超时，用户手动点确定后解除，已清理标签页；不使用自绘confirm或旧图代替。结合当前字体/按钮/背景差异，最终视觉逐项验收尚未闭合。
2. 最后一次删除后重启瞬间观察到全局“读取失败”，随后导航触发真实读取后恢复，订阅页正确为空、磁盘未丢失。保留 [启动失败原图](evidence/p5-06-sharing/closeout/gpui-restart-initial-read-failure.png)；根因尚未确认，不能宣称启动全通过。需要定位启动分享恢复与既有Snapshot/Runtime读取之间是否存在gate竞争；涉及P2-06 owner范围时先协调，未改其代码。

临时端口18786/18787/1438均连接拒绝且可重绑；自有测试进程/监听、state根目录、FIFO、React harness、node_modules临时symlink和两个测试标签页均清理，视口已恢复。签名bundle/原图/日志作为本地验收证据保留，见 [cleanup-final.json](evidence/p5-06-sharing/closeout/cleanup-final.json)。早期失败的read/write FIFO实验及候选截图按原结果保留，不计最终业务通过。实现者自查不是独立Host审查。

68卡重算：**DONE21 / DOING1 / ACCEPTANCE1 / READY5 / TODO33 / DEFERRED7**。READY仍为 **P0-08、P2-05、P3-01、P4-03、P5-04**。P4-02 DONE、P4-03 READY；P2-06 DOING及owner不变，P5-06 owner保留；不领取下游。

## 首轮交付（历史，9cf86ad8）

状态：**ACCEPTANCE**（2026-10-08），未宣称 DONE。基线 `30a4c7ddc1ef40dadc0a68ebebad8afe3f7d70c3`；独立分支 `dev/p5-06-sharing`，工作树 `/Users/lifeilin/.codex/worktrees/p5-06-sharing/veyra`。本任务本地提交，不 push。

## 交付与边界

- `AppState.app_config.subscription_shares` 为唯一持久事实，旧 schema 9 缺字段读取为空。稳定 ID、OS 安全随机 256-bit Token；独立列表、创建、编辑、启停、轮换和删除。
- SnapshotService 共用 StateAccessGate、配置版本 CAS 与 JsonStateStore 原子提交；新端口先绑定，磁盘/CAS/绑定失败保留旧配置及旧可用链接。监听、连接和请求不写入 AppState。
- 生产 ShareService 使用 Axum，只提供 `GET /sub/{token}`（HEAD 沿用 GET 语义），无 HTTP 管理接口。每次读取最新快照，只导出选定 subscription/provider 的节点；未知/旧 Token 404，内容不可用 503，无缓存、无 Token/凭据日志。多个分享共用同一监听，撤销独立生效。
- 输出是节点订阅 JSON `{"outbounds":[...]}`，由现有 Compiler 节点序列化及 parser 逐字段往返检查；不带本机路由、监听、控制器、源订阅 URL。WireGuard 使用现有导入器支持的节点订阅形式，不宣称是新版本内核可直接启动的完整配置。TUIC `zero_rtt_handshake` 与 Hysteria2 salamander 对象增加封闭归一化；无法无损导出的内容明确失败。
- `listen` 是本机 IP:port，`host` 是展示 authority，端口必须一致；拒绝通配展示地址、路径、用户信息及 HTTPS。没有证书/TLS 终止配置，因此 UI 只开放 HTTP；域名不用于 bind，不声称 LAN/公网已通。
- 启动根据保存的 enabled 恢复；AppServices Drop 和现有 on_app_quit 仅调用 `shares.shutdown()`。不改 P2-06 Runtime/Platform、Helper、IPC、Runtime DTO。端口地址复用解决真实 HTTP 关闭后的 TIME_WAIT 快速重启，不启用 SO_REUSEPORT。
- GPUI 正式 UI → AppServices Worker → ShareService → SnapshotService；全量快照回投桥接，避免下一次操作使用旧配置版本。草稿与服务状态分离，失败不清空；重试读取权威版本后可再次保存。三语言、订阅多选、确认、真实 QR 与本机复制接通。

## 工程验证

所有长命令使用 Python subprocess 外层 900 秒 timeout；`MACOSX_DEPLOYMENT_TARGET=15.0`，复用现有 Cargo target。日志保存在 [evidence](evidence/p5-06-sharing/)。

| 命令 / 检查 | 数量 | 退出码 / 结果 |
| --- | ---: | --- |
| `cargo test -p veyra-core shares -- --test-threads=1` | 9 | 0 PASS，`veyra-p506-rebind-core.log` |
| `cargo test -p veyra-core subscription::parser::` | 36 | 0 PASS |
| `cargo test -p veyra-core state_service::` | 40 | 0 PASS |
| `cargo test -p veyra-core singbox::compiler::` | 60 | 0 PASS |
| `cargo test -p veyra-desktop sharing -- --test-threads=1` | 3 | 0 PASS，最终 `veyra-p506-rebind-desktop.log` |
| `cargo test -p veyra-desktop subscriptions:: -- --test-threads=1` | 6 | 0 PASS |
| `cargo clippy -p veyra-core -p veyra-desktop --all-targets -- -D warnings` | — | 0 PASS |
| `cargo build -p veyra-desktop` | — | 0 PASS |
| `cargo fmt --all -- --check` | — | 0 PASS |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | — | 101 FAIL，已有 Windows `binaries/sing-box-1.14.0-windows-amd64/LICENSE` 缺失；独立记录，未修复/绕过 |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | — | 0 PASS |

Core 145 次断言测试执行中有 1 个重复测试，合计 **144 个唯一测试**；Desktop 9 次中有 2 个重复，合计 **7 个唯一测试**。过滤运行均非零，0 ignored。未运行真实 sing-box：本卡输出是订阅文档，未修改正式 runtime 配置生成；Compiler 回归覆盖受影响共享序列化。`block 0.1.6` 既有 future-incompatibility 警告保留。

最终无测试延迟构建 SHA256：`da92c3bed57f0bc5e7cdb4863f648e3e34cc2b4057deb66dc2f128cdcaf0adfe`。路径与身份见 [build.json](evidence/p5-06-sharing/build.json)。此最终构建没有完成最后一轮 GUI 截图，不能将早期截图绑定成最终通过。

## 正式服务与真实网络

Core 8 个 ShareService 用例 + 1 个全协议导出用例使用正式服务、自有临时目录、OS 分配 loopback 端口和真实 HTTP：创建/编辑/删除/磁盘重建，选中与未选中内容隔离，200/404、旧 Token 失效、新 Token 200、停用/删除不可访问、同端口多分享独立撤销、绑定冲突/CAS/错误写入保旧、旧端口释放、HTTP 主动关闭后立即重启。测试 Fixture Drop 清理目录与监听。Desktop Worker 测试执行真实管理保存、轮换、HTTP 200，Drop AppServices 后可重新绑定同端口。

实际 macOS GPUI 使用自有 `dev.veyra.p506.acceptance` 包和 `/private/tmp/veyra-p506-acceptance/state`，从 UI 创建单节点 `验收订阅 A` 和 `手机订阅`，监听 **127.0.0.1:18786**。仅 loopback 已验证。

| 实际操作 | 结果 / 证据 |
| --- | --- |
| UI 创建分享、重启读取 enabled 配置 | HTTP 200，返回 Selected-01 一个节点；正文 SHA256 `a248e3d5cb2ad30581bba838a218c8fb766b4aca796d2bbca875e122b59fd5f5` |
| `state.tmp` 被自有目录占用后 UI Save | 真实 Storage error；旧文件逐字节不变、旧 URL 200、草稿保留；移除故障后 Save 成功，[ui-write-failure.json](evidence/p5-06-sharing/ui-write-failure.json) |
| UI 重新生成确认 | ID 不变、旧 Token 404、新 Token 200，[ui-rotation.json](evidence/p5-06-sharing/ui-rotation.json) |
| UI 停用、重新启用 | 停用后 enabled=false、连接 errno61；重新启用后 200，[ui-disable.json](evidence/p5-06-sharing/ui-disable.json) |
| 本机复制与二维码 | 复制结果粘贴回原生输入框逐字匹配 URL；macOS Vision 从真实截图解码 1 个 QR，内容与 URL 完全一致，[copy.json](evidence/p5-06-sharing/copy.json)、[qr-decode.json](evidence/p5-06-sharing/qr-decode.json) |
| 退出已确认 PID 的测试应用 | SIGTERM 后无监听，地址复用重绑成功；普通 bind 暴露 TIME_WAIT 问题并据此修复。不是托盘 Quit 证据，[ui-exit.json](evidence/p5-06-sharing/ui-exit.json) |

## UI 证据与未完成项

保持用户当前 150% 显示设置、浅色主题；原生内容 viewport 1280×720（原图 2560×1504 含原生标题栏），React viewport 1280×720。React 使用仓库实际 `SubscriptionSettings.tsx` / CSS，临时 harness 只提供自有视觉数据；它不作为正式 Service/HTTP 证据。原图保留，不伪造状态截图。

- [React 编辑基线](evidence/p5-06-sharing/react-edit.png)、[GPUI 编辑候选](evidence/p5-06-sharing/gpui-qr.png)、[列表候选](evidence/p5-06-sharing/gpui-list-candidate.png)。编辑框宽736、双栏/间距24、字段尺寸、16复选框、真实QR164、原Heroicons、HTTP限定及独立监听字段已实现；浅色白色不透明弹窗保留。最后又修正空错误区域多余间距、列表背景/启用色、字号及共享 loading spinner，尚未获得最终截图复核。
- [首次加载](evidence/p5-06-sharing/gpui-loading.png)、[真实忙碌](evidence/p5-06-sharing/gpui-busy-second.png)、[写入失败草稿](evidence/p5-06-sharing/gpui-write-failure-candidate.png)、[轮换确认](evidence/p5-06-sharing/gpui-regenerate-confirm.png)、[停用](evidence/p5-06-sharing/gpui-disabled.png)、[English](evidence/p5-06-sharing/gpui-edit-english.png)、[繁体](evidence/p5-06-sharing/gpui-edit-traditional.png)。
- 加载/忙碌使用一次性4秒 Worker 延迟，仍经过正式 Core/Store 路径；源码已移除测试钩子，延迟构建身份见 delay-build.json。首次 busy 抓拍过晚，已明确保存为 `gpui-completed-first-busy-capture.png`，不计忙碌证据。
- 初次保存遇到缺失 SVG 导致渲染异常；补齐同源 ArrowPathRoundedSquare 并加注册回归。随后遇到桥接旧版本导致 CAS 失败，已修复全量快照回投，并实际重测保存后轮换。早期编译/测试/真实失败日志均保留。

**尚未完成，禁止据此标 DONE：**

1. macOS 在最终复核时锁屏，工具明确要求手动解锁；已通知用户，未收到解锁结果。最终无延迟构建的同数据 React/GPUI 列表、新建/编辑、空、错误、忙碌逐态截图与最终几何/字体对照仍需完成；现有 React/GPUI QR Token 不同，不能声称同数据最终像素验收。
2. 实际 GUI 删除确认后空态、真实端口占用错误及重试、正常应用 Quit 后端口释放仍待补验。正式 Service/Worker 自动集成已验证对应业务，但不能替代这些 GUI 证据。
3. 其它主题、缩放、跨模块完整 E2E 按用户范围归 P5-07/P7-04；LAN/公网、HTTPS 未宣称通过。

实现者按 code-delivery-review 自查，非独立 Review；没有修改 P2-06 保留范围。Host 可按上述剩余项独立代码/功能复核。测试资源清理结果见 [cleanup.json](evidence/p5-06-sharing/cleanup.json)。

## DAG

68 卡：DONE21 / DOING1 / ACCEPTANCE1 / READY5 / TODO33 / DEFERRED7。P4-02 DONE、P4-03 READY、P2-06 DOING 及 owner 保持。READY：P0-08、P2-05、P3-01、P4-03、P5-04；未领取任何下游。P5-06 未 DONE，不能解锁其组合依赖。
