# P0-08 工程原型安装与后续验收

本目录候选是 `VeyraPrototype.app` 能力原型，非正式 Veyra 发布。本轮完整组合最低 macOS 26.0、Apple Silicon；GPUI/updater/helper 自身编译目标为15.0，但固定官方 sing-box 1.14.0 的 Mach-O 实际 minos=26.0，包声明必须取全部二进制的最高要求，不能声称支持15.0；使用现有 GPUI Kit 0.7.0 / gpui-pre 0.3.7、Rust 1.99.0、Cargo.lock。界面沿用 P0-03，不新增正式更新 UI（P6）或完整升级/回滚（P7）。包内 P0-05 helper 和固定 sing-box 1.14.0 只做版本绑定/资源验证，原型 GUI 不启动它们，不能认为正式 Runtime/helper 已交付。没有发布 GitHub Release。

## 固定检查与下载契约

唯一发行仓库为 `lifei6671/veyra`；检查入口为 `https://api.github.com/repos/lifei6671/veyra/releases/latest`。仅消费非 draft、非 prerelease 的 `vX.Y.Z` 稳定 tag，数字比较版本；相同/更旧不下载，不猜测预发行渠道。展示该 Release 的原始 `body` 发行说明和固定仓库的 `html_url`，CLI 不渲染/执行 Markdown。

固定资源为 `veyra-release.json` 与 `VeyraPrototype-X.Y.Z-macos-arm64.zip`。schema=1 清单绑定最低系统26.0及 app/helper/sing-box/resources 版本、源码 HEAD、Cargo.lock SHA256、包名/大小/SHA256；resources 版本等于 app，sing-box 固定1.14.0，helper 单独记录其 Cargo 版本。包名标明 Prototype，正式产品资源名在 P7 明确迁移，不能以本地 fixture 声称远端 Release/资产存在。

使用现有 P0-06 `client_builder`，默认显式 Direct，也可由调用方给出已确认的当前最后有效实例 ID 与 loopback mixed 地址。原型不发现 Runtime、不证明调用方提供的实例真实存在，不扫描端口、不读环境/系统代理、不静默回退。Direct 沿用 BootstrapResolver；现有外部 TUN 下物理出口及受管绕过尚未证明，版本检查/下载不得冒称已绕过 TUN。P2-05 ACCEPTANCE 独立树未消费；后续 P6/P2-05 owner 需提供已确认当前实例/代理凭据及受管 TUN 出口条件，正式产品不使用手填参数替代实例事实。

元数据上限4MiB、包上限512MiB；仅允许固定 GitHub HTTPS 来源及 GitHub 资源 CDN 重定向，最多5次，同一客户端保持出站策略，连接/读取使用 P0-06 builder 的有界 timeout，总操作最多300秒。安装包按实际字节流检查大小/SHA256，非UTF8和超过4MiB均支持。按 Enter 取消检查或下载，stdin EOF 不算取消；取消/断线/大小/摘要错误清理本次 `.part`，已有包或别人的 `.part` 不覆盖。校验成功只返回 `VERIFIED_MANUAL_INSTALL_REQUIRED`，不解压安装、不打开包、不运行脚本。

在仓库根执行（所有输出限定独立 worktree/target）：

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 build --offline --locked -p veyra-core --features p0-06-prototype --example p0_08_update
target/debug/examples/p0_08_update inspect 0.0.0 target/p0-08/github-release-fixture.json target/p0-08/veyra-release.json
# 以下是将来真实远端源存在后才可运行的入口，本阶段没有执行公网请求。
target/debug/examples/p0_08_update check 0.0.0 direct
target/debug/examples/p0_08_update download 0.0.0 /path/to/owned-download-directory direct
# 当前有效实例信息必须由 owner 提供；代理停止直接失败，不转直连。
target/debug/examples/p0_08_update check 0.0.0 via CONFIRMED_INSTANCE_ID 127.0.0.1:CONFIRMED_PORT
```

## 本地构建与静态核验

```sh
python3 tools/p0-08-package.py build --kernel-archive /path/to/known/sing-box.tar.gz
python3 tools/p0-08-package.py verify target/p0-08/VeyraPrototype.app
```

复用 P0-04 archive SHA256：`a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`；必须在提取或执行前匹配。当前 P0-05 原型 bin 未消费空 lib，普通 build 的 native 链接缺失；打包命令仅用 cargo rustc 给该原型 bin 补齐已有 build.rs 的 static=p005_network / Foundation / SystemConfiguration 链接，不改 helper/IPC 源码或生产入口。构建只读取 archive 两个固定 regular member，不下载/安装内核或启动服务。显式签名 updater/helper，保留官方内核有效的 linker ad-hoc 及 P0-05 固定摘要，再签外层 .app；静态核验四个 arm64 Mach-O 的 ad-hoc、最终资源大小/SHA、包封印与内核 version。内嵌 manifest 绑定资源/helper/kernel；最终主程序 SHA 在外部 build-receipt 记录，避免主程序签名的资源封印与 manifest 摘要循环。对 zip 的 SHA 在外部 `veyra-release.json` 中核对；校验和证明完整性，不证明发布者身份，不代替 Developer ID 或公证。

`build-inputs.json` 记录实际源码输入摘要（含未提交实现），`build-receipt.json` 记录最终签名后文件/包身份与执行命令；源码HEAD仍是工程基线，不声称这些未提交变更已属于该commit。校验不得替代 Gatekeeper/quarantine/首次打开实测。

## Host 交 Codex Desktop 的实际操作

本工程阶段不执行下面步骤。P0-08 保持 ACCEPTANCE，P0-09 TODO。

1. Host 先复核工程证据、最终包 SHA256 与资源静态检查，将同一候选交 Codex Desktop。需真正浏览器下载：Host 提供已授权的受控 loopback HTTP fixture（不要发布 GitHub Release）。Browser 从 fixture 下载原样 zip 与清单；这是浏览器/quarantine验收，不证明 GitHub 真实远端下载成功。
2. 对下载文件记录路径、大小、SHA256、`xattr -l` 与 quarantine 值；对解压后的 .app 同样记录。未出现 quarantine 时如实记观察结果，不自行添加/移除属性或称已通过 quarantine 场景。
3. Finder 双击下载解压后的 .app；保留首次阻拦/提示/失败原文及截图。系统允许时只使用 macOS 正常的“打开”/“隐私与安全性”单应用授权路径；不得关闭 Gatekeeper/SIP、运行 `xattr -d` 或以终端启动绕过首次打开。若无法打开，记 FAIL/NOT_RUN，不把本地无quarantine包成功当作通过。
4. 真正窗口出现后验证 P0-03 窗口及托盘正常退出，记录候选身份与本次自有进程清理；不操作用户已运行服务。helper 仅嵌入、不授权安装，不测试系统代理/TUN。浏览器下载、解压、Finder首开、托盘退出均由Desktop记录真实结果。
5. 本原型只手动交接到用户拥有的临时目录。正式安装/替换留 P6/P7；更新顺序为取消应用下载/刷新 → 停止本应用受管 Runtime/分享/观测 → 确认缓存writer关闭与网络恢复成功 → GUI正常退出 → 手动替换完整 app/helper/kernel/resources 组合。停止或恢复失败时不替换；正式helper替换只能消费原owner的一次性授权安装入口，普通GUI不得改管理员目录。本工程没有实施或验收这个正式升级序列。

拒绝出现混版本、缺组件、错误摘要的候选。不要单独替换 sing-box/helper；不要覆盖主用户配置。P7 的完整备份、失败回退/回滚按其任务执行，本原型没有实现自动安装或恢复服务。
