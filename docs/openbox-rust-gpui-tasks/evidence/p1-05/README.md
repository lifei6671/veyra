# OBG-P1-05 平台目录、单实例与原生文件操作

基线 `f9adda4ebe8a836a2f1113747cb672ea8b834c15`，起始工作树干净。本轮只实现 P1-05，不提交、不启动下游。Rust 1.99.0 / GPUI Kit 0.7.0 / gpui-pre 0.3.7；macOS 27.0 arm64 实机，构建 deployment target 15.0，不冒充 macOS 15 实机验收。

产品 identifier 为 `com.lifei6671.veyra`，当前 GPUI namespace 为 `com.lifei6671.veyra.gpui-preview`。Foundation 标准 API 解析 Application Support、Caches、Library/Logs，再追加 preview namespace；kernel-cache 在 Support。所有实测使用显式隔离 root，正常用户 preview 目录和旧应用目录均未创建/读取。`state.json.bak` 与 runtime manifest 路径保留既定布局；Runtime manifest 尚未创建，不冒充 Runtime 接入。[目录](directories.json)、[权限](permissions.json)。

启动先取得真实 `flock`，然后 primary 才创建 AppServices/JsonStateStore 和唯一 Tokio runtime。控制 Unix socket 使用固定 version/Activate/ACK，双方 `getpeereid` 验证同 UID。secondary 不删除 socket、不构造 writer。真实 5 次启动均 exit 0，现有窗口 active/key；占用不明 exit 1/InstanceBusyUnconfirmed；SIGKILL 后锁释放、stale socket 恢复、state 哈希相同。GPUI 原生退出不展开 Rust main，初测残留 socket 的问题已修复：`on_app_quit` 停止 IPC 并移除 socket，composition 保留锁 FD 到实际进程结束。[单实例](single-instance.json)。

NSOpenPanel / NSSavePanel 在 GPUI 主线程使用异步 sheet/completion，不调用 runModal、不占 StateAccessGate。图片按钮正式接入 native picker；手填路径仅 debug 保留。原生选择、ExternalPaths 和已有稳定 ID drag 都汇入 FileSelection：一次打开 FD、O_NOFOLLOW、regular file、8 MiB、PNG/JPEG，再经过现有解码/4096×4096 上限。父目录组件未固定、源 inode 就地并发修改未锁定，不能宣称完全消除 TOCTOU。PNG 重编码为受管内容哈希 ID，temp/write/fsync/rename/directory fsync/0600/readback，source 未改变。引用提交仍沿用 P1-04A CAS/旧资产保留契约。[打开/边界](file-open.json)。

Open Cancel 前后 state/epoch/revisions/assets/draft 不变，无成功 Toast；重复点击只有一个 sheet，取消后可重新打开并接受。伪 PNG 和超大文件真实原生选入后被拒绝，原背景不变。Symlink 拒绝以自动边界测试固定。本轮未重跑 P1-04A 的 drag 手势，当前统一消费路径已静态核对，不虚报拖放实操 PASS。

Save Panel 只导出明确 platform evidence fixture，非正式 Backup；Cancel 不创建文件，Accept 的 33 字节逐字读回一致、0600。非法父目录与 collision 的纯测试证明失败无半文件且原目标保留。[保存](file-save.json)。

NSPasteboard 对可物化 item/type payload 做内存快照；物化或复制失败时不覆盖 pasteboard。成功捕获后再写固定 marker，adapter readback 与真实 Cmd+V 到 GPUI Input 通过；原 pasteboard 已恢复，原内容从未进入日志/证据。[剪贴板](clipboard.json)。NSWorkspace 把固定 `http://127.0.0.1:9/veyra-p1-05` 交给 Edge，返回成功且 exact URL tab 可见；浏览器拒绝 unsafe port，未绕过、未验证加载、未发公网请求，两次测试 tab 已关闭。非法 scheme 单测拒绝。[外链](external-link.json)。

首次交付 Core 294 / Desktop 33 项通过，新增 1 Core + 9 Desktop（含 1 个受控 lock-child harness）。实际构造计数保护无第二 writer，并验证 prepare_quit 移除 socket 后仍持有锁。state/backup 创建及旧 0644 temp 再写均显式 0600，不依赖 umask。规定 check/build/clippy/fmt、resource override workspace check 通过；额外旧 clippy 原始资源检查 FAIL 保留，resource override 单独 PASS。覆盖只证明 compile/lint，不代表旧 Tauri package 资源完整。[验证](validation.json)、[依赖](dependencies.json)。

截图来自实际 GPUI/原生 sheet，未裁切/缩放/改像素；只转换截图 JPEG 为 PNG。Main content 1280×720 logical、DPR 2、完整窗口 2560×1506（含 66 physical px titlebar）；sheet target 各自原生尺寸单列。[清单](visual-manifest.json)：open-panel、save-panel、background-picked、secondary-activated-primary、platform-evidence。原生面板隐藏系统 sidebar 后拍摄，未保存用户名/home/UID/原 clipboard/私人选择路径。

自动化滚动未能到达长 debug Panel 下方，拖动 scrollbar 曾使原生工具连接关闭；本卡平台入口放到顶部后实际可达，未把该手势记 PASS。P1-06 tray/close behavior、Runtime、Backup、旧数据导入均未实现。新 macOS 依赖均来自已有锁文件 registry/cache，离线验证版本/许可；Core 未依赖 GPUI/AppKit，desktop 不依赖 Tauri/helper/prototype。[实现者自查](static-review.json)，不代替 Host Review。

[交互记录](interaction-results.json)、[源码身份](source-provenance.json)、[清理](cleanup.json)：本轮 desktop/secondary 已退出、锁释放、socket 删除、隔离 state/test export/images/error fixtures/临时 app/staging 均清除，正式 evidence 保留。没有启动 sing-box/OpenBox HTTP，没有系统代理/TUN 操作，没有公网验证，没有扫描浏览器 profile/旧正式数据；legacy import NOT_PERFORMED。68 卡按实际显式依赖重算，不开启下游。

<a id="host-review-revision"></a>
## Host Review 修订（2026-10-04）

在已有 P1-05 未提交工作树上仅修订三处：`storage/snapshot.rs` 恢复 `atomic_replace` 直接返回 rename 结果，成功发布后不再因 parent fsync 返回保存失败，保留临时文件及 stale temp 的显式 0600；不引入新的 durability error model。`visual_assets.rs` 对已打开 preview temp 显式设置 0600，并在复用 final preview cache 前收敛到 0600，保留内容/inode 与 assets 0700。`single_instance.rs` 取得 flock 后仍检查同 UID socket；只有连接明确被拒绝才视为 stale 并删除，可连接或其他无法确认结果均返回 `InstanceBusyUnconfirmed`，不删除活 socket，Activate/ACK 协议未变。

新增 3 Desktop 回归分别保护旧 0644 temp 发布权限、旧 0644 cache 私有复用，以及无 Veyra flock 的独立活 listener 不被接管。修复前实际运行分别 2 FAIL / 1 FAIL，修复后全量 Desktop **36**、Core **294** 项通过（0 failed / 0 ignored）；既有 stale socket、SIGKILL、secondary Activate 和 Core 原子失败/权限回归通过。用户指定的两包 test/check/clippy、workspace locked resource-override check、fmt、diff check 均 PASS；按仓库规则追加旧入口 lib clippy（同 resource override）PASS。fmt 首轮换行 FAIL 已修正，原记录保留在 [validation.json](validation.json) 的 `host_review_revision`，本轮日志为 `checks/host-review-*`。

剪贴板仅承诺可物化 item/type payload 的内存快照；捕获、物化或复制失败时不覆盖 pasteboard，不宣称保留所有 promise/provider 表示。既有 GUI/原生证据与源码/二进制身份保留；本轮未重跑 GUI/原生操作，新源码身份另列在 [source-provenance.json](source-provenance.json)。已有 TOCTOU、macOS 15 实机及旧打包资源限制保留；目录 durability 不增加保证，既有 `block 0.1.6` future-incompatibility warning 未处理。

本轮三个修复前失败 fixture 已精确清理，成功测试清理自身 root 并回收自有 test child。P1-05 状态不变，P1-06 仍 READY、未领取/未开始；无公网、sing-box、System Proxy、TUN、commit/push。修复者使用 code-delivery-review 做定向自查，不声称新一轮独立 Host Review。
