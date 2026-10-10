# P0-08 Desktop 接管与验收（2026-10-10）

本卡最终状态 **DONE**（2026-10-10）：原浏览器ZIP身份、保留quarantine、正常单应用系统授权、真实GPUI窗口/交互、关闭→托盘恢复→正常退出及清理均已完成；独立Review无剩余代码Finding。GitHub远端资产下载仍NOT_RUN，不push或正式发布。源码/签名仍为同一候选；原失败和构建时收据不改写。开发基线 `733b6ea3d20b23ba90bb933971a08b076bc0caa3`，独立分支 `dev/p0-08-update-distribution`。本地提交/合并及主分支复验记录在后续收口收据。

## 首次打开最终验收

完整步骤、原图、用户操作确认、运行身份与系统退出日志见[最终证据](evidence/p0-08/first-open-20261010/README.md)、[结果收据](evidence/p0-08/first-open-20261010/result.json)及[最终独立Review](evidence/p0-08/first-open-20261010/REVIEW.md)。Downloads验收包位于 `/Users/lifeilin/Downloads/Veyra-P0-08-first-open-20261010/VeyraPrototype.app`；同目录保留原ZIP副本，原浏览器ZIP在 `target/p0-08-browser-download/`。SHA256与原发行清单一致。用户亲自完成“仍要打开”与系统授权、托盘显示窗口/退出；自动化核验窗口原图、中文/主题/模拟状态、同PID16877、全部文件摘要、签名和最终进程消失。中间密码授权弹窗与托盘菜单未截图，明确为实际用户操作确认，Reviewer只核验证据。

系统首次提示仅无法验证是否含恶意软件，“完成”后在正常单应用入口授权；无损坏/已检测恶意软件提示。App quarantine仍在，系统授权标志0081→00c1，ZIP保持0081；没有Agent属性写入、重签名或终端启动。主进程系统日志退出码0/voluntary，两个系统XPC也清理，未安装/运行管理员helper；Gatekeeper/SIP启用，未改用户网络或其它运行实例。下列工程结果与首轮FAIL为历史真实事实，以本节最终验收为当前结论。

## 工程交付

先通过 Serena 读取 WorkRun `work-23397-1791595539522424-205` 与 Execution `execution-23397-1791595708757289-208`。原执行仍running；请求cancel并observe确认cancelled后才写入，原执行无最终完成结论。接管前摘要见 [takeover.json](evidence/p0-08/desktop-20261010/takeover.json)。主树一直保持原HEAD与既有 `tools/tests/__pycache__/`，未修改主树。

更新CLI只消费固定 `lifei6671/veyra` GitHub Releases，稳定 `vX.Y.Z` 版本、固定资产名、发行说明和app/helper/sing-box/resources清单。复用现有P0-06显式HTTP builder，保持Direct/ViaRunningProxy、TLS/超时/手工重定向、无系统或环境代理发现/直连fallback；不消费未合并P2-05，不改Runtime/Helper/IPC公共契约。二进制流支持大于4MiB/非UTF8，声明大小和SHA256一致后才交接；失败/取消拒绝、清理本次partial、保留已有文件，成功仅反馈 `VERIFIED_MANUAL_INSTALL_REQUIRED`，不冒充已安装。正式更新UI和升级序列留P6/P7；[安装说明](P0-08-installation.md)明确这些边界。

修复了一项真实缺陷：Python `assert` 在 `python3 -O` 下会消失，使错误archive或摘要检查失效。现改为显式条件失败；新增优化模式错误archive和破损包拒绝测试。独立Reviewer复查记录见 [REVIEW.md](evidence/p0-08/desktop-20261010/REVIEW.md)。

## 本轮实际验证

每条完整命令/退出码由同目录同名JSON记录，原始stdout/stderr为LOG，外层timeout已设置。以下仅代表实际运行范围，没有零测试PASS。

| 验证 | 结果 | 证据 |
| --- | --- | --- |
| 更新原型定向测试，loopback/显式proxy/取消/摘要/保旧/版本 | PASS，8/8，exit0 | [update-tests.log](evidence/p0-08/desktop-20261010/update-tests.log) |
| GPUI prototype/helper prototype/updater三项实际构建、生成ZIP | PASS，exit0 | [bundle-build.json](evidence/p0-08/desktop-20261010/bundle-build.json)、[build-receipt.json](evidence/p0-08/desktop-20261010/build-receipt.json) |
| 真实包正反校验及优化模式回归 | PASS，8/8，exit0 | [package-tests.log](evidence/p0-08/desktop-20261010/package-tests.log) |
| Core含feature/example all-targets Clippy `-D warnings` | PASS，exit0 | [core-clippy.log](evidence/p0-08/desktop-20261010/core-clippy.log) |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | PASS，exit0 | [fmt.json](evidence/p0-08/desktop-20261010/fmt.json) |
| 旧Tauri `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | FAIL，exit101：缺既有Windows `sing-box.exe` 资源 | [legacy-clippy.log](evidence/p0-08/desktop-20261010/legacy-clippy.log) |
| 最终包四个arm64/ad-hoc/资源与外壳codesign strict/deep、固定内核version | PASS_STATIC_ONLY，exit0 | [final-static-verify.log](evidence/p0-08/desktop-20261010/final-static-verify.log) |
| 离线发行清单/Release结构检查 | PASS，exit0，明确fixture | [manifest-inspect.log](evidence/p0-08/desktop-20261010/manifest-inspect.log) |

原型helper为P0-05能力探针，仅嵌入，未安装/运行管理员服务。内核只执行普通用户 `version`，不启动业务child。包为 `VeyraPrototype.app`，并非正式生产Veyra发行。app/helper/resources版本0.1.0，内核固定1.14.0；整体最低macOS26.0取四个最终Mach-O最高要求（GUI/updater/helper为15.0，内核为26.0）。ad-hoc与校验和不证明发布者身份或公证，不引入开发者账户/正式发布签名。

候选位置：

- `target/p0-08-desktop/VeyraPrototype.app`
- `target/p0-08-desktop/VeyraPrototype-0.1.0-macos-arm64.zip`，54,830,834 bytes，SHA256 `fdb7472d4792bf4f7f88568f3c1f288746da5efcf55cdf0bded7b28262221a26`
- `target/p0-08-desktop/veyra-release.json`；[构建输入摘要](evidence/p0-08/desktop-20261010/build-inputs.json)绑定实际源码与未提交实现；[构建收据](evidence/p0-08/desktop-20261010/build-receipt.json)绑定最终四个签名后文件。

旧 `target/p0-08` 候选与构建记录原样保留，不覆盖为新结果。

## 首轮实际浏览器与macOS结果（历史，未完成）

本机Apple Silicon，macOS27.0.1。实际Edge访问GitHub Releases显示无Release，[原图](evidence/p0-08/desktop-20261010/github-no-releases.png)与[API HTTP404](evidence/p0-08/desktop-20261010/github-latest.json)分别保留；GitHub真实资产下载NOT_RUN，不自行发布。

另以仅loopback、自己拥有的短时HTTP候选目录提供同一ZIP，Edge真实点击下载→原生另存为→隔离 `target/p0-08-browser-download`。浏览器事件等待因下载需另存为超时，后续原生对话框完成保存；不是工具下载/复制替代浏览器。[下载完成原图](evidence/p0-08/desktop-20261010/browser-download-complete.png)。ZIP字节与发行清单完全一致，[browser-artifact.json](evidence/p0-08/desktop-20261010/browser-artifact.json)记录大小、SHA256及来源属性；其quarantine为 `0081;6ac998ed;Edge;BD4479A9-E9E8-4BEC-A8F5-41E4B65E9FD1`。初次 `xattr -l` 包含二进制metadata导致UTF8读取失败，后改用 `-lx` 保留十六进制原值，不更改属性。

Finder打开ZIP，由系统归档工具解压；解压 `.app` 同样保留上述quarantine，`codesign --verify --deep --strict --verbose=2` exit0，[首开前记录](evidence/p0-08/desktop-20261010/before-first-open.json)。Finder双击app后没有取得可确认的真实窗口；应用读取连续timeout，不计运行成功。系统[限定日志](evidence/p0-08/desktop-20261010/first-open-system.log)记录AppTranslocation、amfid Code=-423（ad-hoc/unknown chain）、Gatekeeper `Prompt shown` / `Adding Gatekeeper denial breadcrumb (open)`，launchd终止；**这不是确认GUI成功的证据**。原提示文字/原图未捕获，随后解压app已不在原目录，[现场原图](evidence/p0-08/desktop-20261010/first-open-no-window.png)；消失原因未证明，本Agent未发出移动/删除app动作，不推断用户或系统具体操作。

首次打开验收 **FAIL / 尚未完成**；正常单应用“打开/隐私与安全性”授权、真实窗口和托盘退出 **NOT_RUN**。没有移除quarantine，没有关闭Gatekeeper/SIP；两者只读状态均enabled。不把本地无quarantine构建目录、静态签名通过或瞬态进程登记作为首次打开通过。浏览器loopback结果不替代远端GitHub下载。

仅测试候选曾尝试启动，未启动helper/sing-box业务、未改用户系统代理/TUN/路由/DNS/管理员服务或操作现有Veyra实例。自有HTTP监听已停止、临时服务目录和独立树本轮Python缓存已清理，浏览器下载ZIP和新旧候选/evidence保留；[最终现场](evidence/p0-08/desktop-20261010/macos-result.json)。

## 首轮后续验收与Git（历史计划，已由本轮首开验收闭合）

仍需保留quarantine，通过macOS正常单应用授权路径取得同一候选的真实窗口、必要原图和正常托盘退出/清理；远端资产不存在时继续准确标NOT_RUN，不为验收发布。P0-08保留ACCEPTANCE与owner/独立分支，不提前DONE。Commit SHA、Merge SHA均**无（未执行）**；主分支受影响复验**NOT_RUN（未合并）**。

68卡显式依赖重算：DONE28 / ACCEPTANCE2 / DOING1 / READY0 / TODO30 / DEFERRED7，无悬空依赖/环，无新增READY；P2-05 ACCEPTANCE、P2-06 DOING及原owner不变。历史证据不改写，本轮只更新当前P0-08状态及汇总。

## 合并后定向验证发现与最小修复

初次独立提交 `f0fb874805c95b251b1b90465a8336cf302902dd`、初次合并 `4e8f9cb92e93b40bfb3fc272d1382062d74dc6d6` 后，主分支打包正反测试8/8 exit0；更新测试首次7/8 exit101，取消测试创建临时目录时发生同PID/同时间戳碰撞。没有改写失败或串行化掩盖并发，只在cfg(test)目录名增加进程内原子序号；业务源码/断言/候选ZIP/签名不改。独立树默认并发8项连续3轮PASS、Core Clippy/fmt exit0，独立修复Review与原FAIL见[修复证据](evidence/p0-08/test-directory-fix-20261010/README.md)。主分支再次合并后复验另记真实命令与SHA。

原候选120项构建输入与首次提交f0fb874完全一致；当前唯一不同是上述cfg(test) tests.rs。原包仍绑定原输入和原实机结果，不重建/重新签名后冒充已下载候选，见[来源绑定](evidence/p0-08/test-directory-fix-20261010/candidate-source-binding.json)。
