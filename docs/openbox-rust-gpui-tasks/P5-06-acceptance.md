# OBG-P5-06 订阅分享交付与局部验收

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
