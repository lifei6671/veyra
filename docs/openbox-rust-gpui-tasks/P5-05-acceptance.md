# OBG-P5-05 共享网络编辑与 URI/二维码交付

2026-10-09；Task **DONE**，Codex · P5-05 GPUI/Core owner/写预约释放。按本轮明确范围验收 macOS / 150% / 浅色；其它主题、缩放和完整跨模块交互由 P5-07/P7-04 组合验收承接，未冒充已执行。P2-06 保持 DOING、原 owner 不变。

## 身份与范围

原生 Codex Desktop Worktree 从主开发分支 `codex/dist-react-restore` 的 `8c220437ab6a4f9528e39e84261bebd916d0080a` 创建：`/Users/lifeilin/.codex/worktrees/p5-05-share-ui/veyra`，分支 `dev/p5-05-share-ui`。会话默认 cwd 原为主树；全部开发命令显式指定新 worktree。实际根目录/HEAD/分支、文件写入、index lock、临时 index 暂存/write-tree/commit-tree 权限通过；原有 P5-04/P5-06 worktree 未复用，主树 `tools/tests/__pycache__/` 未跟踪文件保留。P5-04/P1-04A/P1-05 均 DONE 后领取。

正式交付：

- `crates/veyra-desktop/src/ui/shared_network.rs`：列表/空态/加载/错误、新增/编辑、五协议字段、启停、删除确认、拖动排序、复制、二维码、本地地址警示、保存中禁用、失败保草稿与重试。
- `crates/veyra-desktop/src/shared_network.rs`、`services.rs`：GPUI → AppServices blocking Worker → SnapshotService → JsonStateStore；CAS 冲突重新读取权威快照但保留草稿，用户可再次保存。启用时做真实端口预检；预检不声称内核启动成功。
- Core `application/shared_inbounds/sharing.rs`：五协议 URI、UTF-8/特殊字符编码、IPv6 方括号、随机凭据及最小自签证书生成；`state_service.rs` 仅增加完整 ID 集合排序事务。已有 P5-04 证书导入/解析足够，UI 新增证书只补原版自签语义，仍将 PEM 保存到同一 SharedTls 事实，未另建资源存储。
- `subscription/parser.rs`：接纳本产品生成的标准 URI（含 TUIC ALPN/insecure、Hy2 obfs、VLESS encryption=none、mixed 编码凭据）；继续拒绝未知选项。
- 提取共享 `ui/components/qr.rs`，复用 Tokens/输入/按钮/Select/Modal/i18n；新增同源 Heroicons QrCode（既有 MIT 来源），未删除 Legacy React/CSS。

`SharedServer.listen` 是 bind 地址，`address` 只供分享。原版表单只有连接地址，编辑保留已有 listen；新增为 P5-04 模型的 `0.0.0.0`，不通过改 address 重写 listen。所有保存仅提示 **已保存，待应用**；不调用 Runtime Apply/Restore、不将 SavedOnly 当作 Ready。P5-06 仅复用 QR 绘制，HTTP/token/host/listen 语义未改；Runtime/Platform/Helper/IPC/公共 Runtime DTO、Cargo workspace 均未改。

## 自动与真实 Store 验证

每项测试保护 Veyra 自有用户行为：生产 CRUD/排序/凭据资源回读、写失败/CAS 保旧和草稿重试、真实端口占用、协议切换不夹带字段、URI 导入与 QR 内容一致。未重新认证内核算法。

原始日志位于本地忽略目录 [evidence/p5-05](evidence/p5-05/)。命令均有 Python subprocess 外层有界 timeout（测试/检查 180–300 秒，构建最长 600 秒）。下表从原始 `running N tests` / 逐项 `... ok` / `test result` 核对，**116 个唯一 PASS**；重复执行不累加，ignored/filtered 不计通过。

| 命令/范围 | 实际结果 | 原始日志 |
| --- | --- | --- |
| `cargo test -p veyra-desktop shared_network -- --test-threads=1` | 6 PASS，1 内核入口 ignored（下行显式执行） | desktop-end-fixed.log |
| `cargo test -p veyra-core application::shared_inbounds -- --test-threads=1` | 10 PASS，1 P5-04 加载入口 ignored | core-sharing-retry.log |
| `cargo test -p veyra-core application::state_service::tests -- --test-threads=1` | 36 PASS | state-service.log |
| `cargo test -p veyra-core storage:: -- --test-threads=1` | 24 PASS，1 独立入口 ignored | storage.log |
| `cargo test -p veyra-core subscription::parser -- --test-threads=1` | 36 PASS | parser-regression.log |
| Desktop sharing QR 受影响回归（`ui::subscriptions::sharing`） | 3 PASS | desktop-ui-regression.log |
| 显式 `shared_network_generated_resources_locked_check --ignored --exact`（完整 test 名见日志） | 1 PASS，真实五协议 check | locked-check.log |
| `cargo clippy -p veyra-core -p veyra-desktop --all-targets --offline -- -D warnings` | PASS | clippy-release.log |
| `cargo build -p veyra-desktop --offline` | PASS | build-release.log |
| `cargo fmt --all -- --check`；旧 Tauri 指定 fmt | PASS | fmt-release.log / legacy-fmt.log |
| `git diff --check` | PASS | diff-check.log |
| 旧 Tauri 指定 `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | **FAIL**：既有 Windows `binaries/sing-box-1.14.0-windows-amd64/LICENSE` 缺失；未修改无关打包资源 | legacy-clippy.log |

Worker 测试通过生产 AppServices 创建全部五协议、修改/启停/删除/排序；释放全部 Service 后重建并真实回读磁盘，固定端口/listen/证书保持。`state.tmp` 目录故障注入确认磁盘原字节与原配置保持；撤除故障后同草稿重试成功。CAS 旧版本拒绝、重读后再保存成功。真实自有 TCP listener 占用时创建/启用失败保旧，停用配置可保存；释放端口后启用成功。无效重复/缺失排序 ID 不改变磁盘。

初轮编译/Clippy/fixture 错误及 `core-sharing-final.log` 中一次预检端口瞬态 FAIL 保留；串行定向重跑 `core-sharing-retry.log` 为 10 PASS，未删改断言。旧 P5-04 Domain 基线 FAIL 不在本轮执行范围，不改写为 PASS。

## 固定内核与资源

锁定 sing-box **1.14.0 darwin-arm64**，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。显式测试从 `/private/tmp/veyra-p402-check-20261008/veyra-sing-box` 验证后复制到自有临时目录；真实 Worker 保存五协议及新生成 TLS → 释放/重建 Store → 正式 `SingBoxCompiler::compile_shared_inbounds()` → 实际 `sing-box check`，五协议全部通过。证书生成两次资源不同且解析/私钥匹配通过；编辑和重启沿用已保存 PEM，不重新生成。

本卡新增真实 check，不额外实现运行管理器；P5-04 已完成最小加载/bind 冲突/退出清理，见 [P5-04 验收](P5-04-acceptance.md)。本卡不宣称正式 Runtime Apply/Restore 或代理已连通。

## 真实 UI、复制与二维码

最终本地 signed bundle `evidence/p5-05/Veyra P505.app` 可执行 SHA256：`71930ed61a3c710e292268079e921181dc6be9fa3a5f2a47a7fbf2644cb550e1`。隔离 root `/private/tmp/veyra-p505-gui`，未操作用户正式 root、系统代理、TUN 或现有内核运行实例。

已实际操作：五协议创建；修改 SS 名称/端口；mixed 单边凭据拒绝、成对保存；启用/停用；拖动 mixed 从末尾到首位并真实保存；删除取消/确认；普通进程重启回读与删除后空态；复制到自有输入框核对；协议切换字段与焦点/悬停；端口占用错误保草稿；生产 Store 写入失败后原草稿再次保存。最终候选的删除四记录/空态、新建及五协议连续切换、扫码及普通保存重启亦复验。

QR 使用同一 `shared_server_uri` 字符串，未维护第二份内容。Swift Vision 从真实原图解码：最终 SS `gpui-scan-delivery.png` → `qr-delivery-decoded.txt` 与 `gpui-scan-delivery.ax.txt` 中 **家里/18388** URI 完全相同；四协议 `gpui-{vless,tuic,hy2,mixed}-delivery.png` 分别与 `protocol-delivery.ax.json` 显示 URI 完全相同，结果见 `protocol-qr-decoding.json`。五协议 Core roundtrip 同时验证 IPv6/中文/特殊字符经现有导入解析器恢复凭据、地址、TLS。旧 `qr-decoded.txt` 的家/8388 是历史样本，不能代替最终对应验证。

忙碌证据 `gpui-saving-delivery.png` 通过自有临时 root 的受控 FIFO 使真实 Worker 等待，确认保存中按钮和 Select 禁用；未伪造 UI 状态。该人工非普通文件实验在 macOS 上曾被 fsync/rename 接受，**不作为保存失败或持久化通过证据**；已停止自有 app、移除自有 pipe 并恢复自有普通备份。真实失败保旧另由目录故障证明。恢复后最终候选正常 GUI 保存并重启回读，见 `gpui-post-cleanup-restart.png` / `post-cleanup-restart.txt` / `busy-fault-cleanup.txt`。

## 在线原版与逐态视觉

实际打开 `http://192.168.1.6:3036/#/settings` 并操作原版新增/协议选择/扫码/删除取消；无线上保存或删除。为了同数据比较，第二个自有参考 tab 只对已观察到的读取请求使用临时响应夹具，没有写远端后端。最终清除拦截、恢复 viewport、关闭自有参考 tab。原始截图未裁剪/修饰；在线视口 1280×720，GPUI 内容同逻辑尺寸（原图 2560×1504 含 macOS 32px 原生标题栏），系统为本轮约定 150% 浅色。

| 状态 | 在线真实原图 | GPUI 最终原图 | 结论 |
| --- | --- | --- | --- |
| 同数据四记录列表 | [online-list-candidate.jpg](evidence/p5-05/online-list-candidate.jpg) | [gpui-list-delivery.png](evidence/p5-05/gpui-list-delivery.png) | 卡片/行距/图标操作布局对齐 |
| SS 编辑 | [online-edit-candidate.jpg](evidence/p5-05/online-edit-candidate.jpg) | [gpui-edit-delivery.png](evidence/p5-05/gpui-edit-delivery.png) | 576px 弹窗、双列/输入/QR 对齐；blur/字体差异下述 |
| 扫码 | [online-scan-candidate.jpg](evidence/p5-05/online-scan-candidate.jpg) | [gpui-scan-delivery.png](evidence/p5-05/gpui-scan-delivery.png) | 名称标题、无多余 Tabs、224px QR、提示对齐 |
| 空态 | [online-empty-candidate.jpg](evidence/p5-05/online-empty-candidate.jpg) | [gpui-empty-delivery.png](evidence/p5-05/gpui-empty-delivery.png) | 40px 图标、56px padding、12px gap、加号按钮对齐 |
| 删除确认 | [online-delete-final.jpg](evidence/p5-05/online-delete-final.jpg) | [gpui-delete-delivery.png](evidence/p5-05/gpui-delete-delivery.png) | 512×142、确定/取消及正文对齐（背后记录数不同，不作全图分数） |
| 新增与五协议字段 | online-ss-1280 / online-new-vless / online-tuic-1280 / online-hy2-1280 / online-mixed-1280 | gpui-new-ss-delivery / gpui-vless-delivery / gpui-tuic-delivery / gpui-hy2-delivery / gpui-mixed-delivery | 相应字段、TLS说明、凭据布局逐态核对；早期VLESS视口不同仅作字段参考 |
| 加载/失败/启停/拖动/复制/焦点/悬停/保存禁用 | 已观察在线相应交互；失败以真实生产故障为准 | gpui-list-final（历史加载图）/ gpui-port-error-final / gpui-store-error-final / gpui-disabled-final / gpui-reorder-final / gpui-copy-final / gpui-focus-delivery / gpui-saving-delivery | 真实操作与反馈；不把加载图作最终稳定列表 |

测量基准（逻辑 px）：列表卡片 x264/y56/w1008/h70、padding12/radius16、标题16/24、详情12/16、动作32/gap8；后续行 y134/212/290。SS 编辑 w576/h602，header41、body512、footer49，列264/gap16、控件32、标签12/16、QR176；扫码 w448/h369、QR224；删除512×142。

**保留的具体技术差异**：

1. 在线弹窗使用 `backdrop-filter: blur(10px)`；当前 gpui-pre 0.3.7 Style 只有阴影 blur、无元素背景 blur API。同样半透明弹窗在 GPUI 中可见未模糊的背后文字/卡片边缘，影响编辑/扫码/确认弹窗背景，见以上同态原图。整窗 macOS vibrancy 不能等价局部元素滤镜，本卡不越权修改 Platform 或伪造截图纹理。
2. GPUI 现有 MiSans 静态字体注册与在线 MiSansVF 的部分中文字重/抗锯齿不同；尺寸/行高按来源设置，影响标题、备注与说明字形，不称字体完全一致。系统原生标题栏、Veyra 品牌/无 Runtime 观测值不参与当前共享内容像素比较。

当前结论 **PASS_WITH_TECHNICAL_DIFFERENCES**，不声称像素完全一致。用户本轮明确在线页面为实际目标：线上无地区分流卡和扫码客户端 Tabs，故未加入仓库旧参考中的这些区域；删除按钮采用线上“确定”。mixed 原版声称仅局域网可用，但模型实际 bind=0.0.0.0 且 Veyra 未管理 WAN 防火墙，说明改为真实“SOCKS5 和 HTTP 共用端口，用户名密码同时留空无需认证”，不伪造网络隔离保证。listen/address 保持分离。

## 独立 Review 与状态

独立只读 Reviewer `/root/p504_review` 按 code-delivery-review 复核生产服务/持久化、CAS/失败保旧、协议切换、URI/凭据/TLS、监听地址及 owner 边界，实际查看最终逐态在线/GPUI 原图并独立核对 bundle SHA 和日志。

两个 P2 已修复并关闭：保存中 Select 可改变草稿；重复选择当前协议重置凭据/证书。当前实现锁定保存输入、当前协议选择 no-op，并有对应回归。线上无地区卡的历史建议已按实页撤回。最终结论：**无剩余可操作 P0–P3 代码或可修复显著视觉 Finding**；blur/MiSans 按本轮允许的技术边界记录；旧 QR 样本标注事项已以最终重解码关闭。最后文档复核指出的调度摘要和 READY 表空行两项小问题亦已修正。

所有本卡当前约定验收完成，P5-05 → DONE，owner/预约释放。全量 68 卡按原显式依赖重算：**DONE24 / DOING1 / READY4 / TODO32 / DEFERRED7**；READY=P0-08/P2-05/P3-01/P4-03，均不在本轮启动。P5-07 仍依赖 DNS/P4-07 等未完成卡，未转 READY。

P2-06 最小接入清单沿用 [P5-04 清单](P5-04-acceptance.md#给-p2-06-owner-的最小接口清单)：正式 compile_product 消费已保存 shared_servers、固定 listen/port/凭据及 SharedTls 资源；Apply/Restore 不重新选端口/生成证书，bind 失败真实反馈并按原恢复机制处理。本卡未增加 Runtime DTO/第二套配置或恢复状态，不新增 P5-05 对 P2-06 硬依赖。

本 Task 一次本地功能 commit 后才合并回主开发分支，合并后的验证/SHA 记录于本地 `evidence/p5-05/main-integration.json` 并在交付回复给出；不 push。历史失败、未执行项与旧证据不覆盖。

## 主开发分支合并后复验

本卡唯一功能提交 `9ee2294bafd740528c3489246edeae8423a6bf8f` 已通过 no-ff 合并进入 `codex/dist-react-restore`，没有冲突。合并验证在主树实际 cwd 执行；`main-integration.json` 保留精确命令/退出码，`main-test-counts.json` 按逐项日志去重：Desktop6、Core shared_inbounds11、StateService40、Storage24、Parser36、共享QR3、显式固定内核check1，合计 **121唯一PASS**。其中过滤范围比初验宽，新增覆盖1项正式Compiler拒绝未集成共享入站和4项已有行为设置事务测试；不与初验116累加。Core/Desktop all-targets Clippy -D warnings、workspace fmt、Desktop build、diff检查均exit0。主树原未跟踪文件SHA256与合并前相同，仅保留该既有未跟踪路径。合并SHA以交付回复及本地integration记录为准；无push。
