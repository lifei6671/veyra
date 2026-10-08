# OBG-P5-06 订阅分享交付与局部验收

## 2026-10-08 验收修正（当前）

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
