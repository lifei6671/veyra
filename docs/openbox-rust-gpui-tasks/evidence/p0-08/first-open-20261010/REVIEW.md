# P0-08 首次打开与最终独立 Review（2026-10-10）

**结论：PASS_NATIVE_PROTOTYPE；未发现剩余可操作工程 Finding。** 本轮补齐正常系统授权后的真实窗口、关闭隐藏、同一进程托盘恢复和正常退出证据，`P0-08-MACOS-FIRST-OPEN-001` 在本轮本地候选原型范围内 **CLOSED**。P0-08 原卡四项验收可收口；维护者更新当前任务/状态文档后可按用户已授权的独立提交、合并与主分支定向复验继续。GitHub 无远端资产的 **NOT_RUN** 仍保留，不把本地原型 PASS 当作正式发行通过。

本次由独立 Reviewer 按 code-delivery-review Skill 核读最终源码、完整 tracked diff、未跟踪交付源码/测试及实机证据；只新增本文件。没有亲自操作 GUI、授权、托盘或系统设置，没有执行测试、启动 helper/内核、改 quarantine、Git 写入或发布。旧 `desktop-20261010/REVIEW.md` 及当轮失败结论完整保留；本轮通过不改写旧现场。

## 工程与身份复查

更新 CLI、打包脚本与测试保持前轮已审查版本。固定 GitHub 仓库/稳定 tag/资产、版本绑定、现有 P0-06 出站 builder、有限重定向、下载取消/大小/SHA-256/失败保旧和手动交接契约没有新缺陷。`P0-08-PYTHON-OPTIMIZE-001` 仍 CLOSED：显式条件失败保留优化解释器下的来源/摘要/签名检查，新增真实 `python -O` 错误 archive/破损包拒绝测试通过。未新增另一套代理、安装服务或持久状态，也未复制 P2-05 未合并实现。

已检查独立树相对 `733b6ea3d20b23ba90bb933971a08b076bc0caa3` 的完整 tracked diff，工程 tracked 变更仅为 Core Cargo 的 example 注册，其他为本卡文档。Core 正式源码、Desktop、Helper、`src-tauri`、workspace `Cargo.toml`/`Cargo.lock` 的基线差异检查 exit 0；P2-05/P2-06 公共契约不受影响。未跟踪交付源码/测试已在前轮及本轮连续审查，不把 tracked diff 当作全部改动。

独立重新计算的 120 项构建输入与当前源码完全一致。原浏览器 ZIP 及 Downloads 的保留属性复制 ZIP 均为 **54,830,834 bytes**、SHA-256 **`fdb7472d4792bf4f7f88568f3c1f288746da5efcf55cdf0bded7b28262221a26`**。Finder 解压 app 的 manifest 与最终构建收据完全相同，四个最终二进制及全部资源的摘要/大小匹配。`running-identity.json` 进一步绑定 Downloads app、实际 AppTranslocation 运行 app 与 PID 16877；两处 Info.plist、CodeResources、主程序、清单和全部资源身份一致，签名验证均 exit 0。app/helper/resources 0.1.0、kernel 1.14.0、整体最低 macOS 26.0 的说明保持正确。

工程测试证据复用 `../desktop-20261010/` 的原命令与原日志：Rust updater 8/8、真实包正反/优化模式 8/8、实际 bundle build、Core all-target Clippy、fmt、离线 manifest inspect、最终 codesign/static verify 均 exit 0。Legacy Clippy exit 101 的既有 Windows `sing-box.exe` 资源缺失保留；相关基线文件未改，不伪造通过。源码与最终包未改，不为此次实机复验扩大测试。

## 实机证据与操作归属

`zip-identity.json` 记录原 Edge 下载 ZIP 经保留属性复制到用户拥有的 Downloads 隔离目录；复制不是第二次浏览器下载。`finder-extracted.json` 保留首次打开前 quarantine 与 codesign 结果。此前真实 Edge loopback 下载及字节一致性证据继续有效。

`gatekeeper-prompt.png/.txt` 捕获系统“Apple 无法验证是否包含恶意软件”的真实提示，含“完成/移到废纸篓”。记录为选择完成；`open-anyway-visible.png` 与对应 AX 文本证明系统设置仍保留“App Store 与已知开发者”，并提供针对 VeyraPrototype 的“仍要打开”入口。授权和打开由**用户本人**完成并确认；中间密码/授权确认弹窗截图 **NOT_CAPTURED**。Reviewer 没有操作密码或授权，也不声称这些未捕获画面已留证。

`gui-initial.png/.txt` 为授权后的真实浅色 GPUI 原型窗口；`gui-interaction.png/.txt` 显示中文粘贴“P0-08 首次打开验收”、深色切换与 synthetic state=true。这是原型实际交互，不是正式代理业务或中文 IME 组合输入验收。`after-close-process.json` 记录 CUA 关闭按钮后进程16877保持；窗口截图可以捕获隐藏 NSWindow，不能单靠该截图宣称屏幕可见状态。

托盘原生目标未向自动化暴露。用户亲自展开托盘并点击“显示窗口”，确认窗口恢复；`gui-tray-restored.png/.txt` 与 `tray-restored-process.json` 证明恢复后的真实窗口保留中文文本/深色/模拟状态，并仍为 PID16877。用户又亲自点击托盘退出并确认。托盘菜单截图 **NOT_CAPTURED_NATIVE_TARGET_LIMITATION**；托盘操作依据是用户实际操作确认，加恢复原图、同 PID 与退出系统记录，不能称自动化或 Reviewer 亲自点击菜单。

`first-open-system.log:2318` 明确 PID16877 的 launchd 退出结果 `(0, 0, 0)`，`:2330` 为 voluntary；`exit-cleanup.json` 与 `all-owned-processes-absent.json` 确认16877、系统XPC16892/16893均已不存在。XPC的系统终止记录与应用自身正常退出分别记述，不把系统XPC视为Veyra helper。没有本次 helper/update/kernel 存活进程或管理员安装资源。

退出后的 app quarantine 仍存在，为系统单应用授权后的 `00c1`；原 ZIP 和复制 ZIP 仍 `0081`。codesign和文件身份未变，`spctl` assessments enabled、SIP enabled。没有删除 quarantine、关闭系统保护或改签重建后替代原下载候选。

## 原卡四项判定

| 原任务验收 | 本轮结论 |
| --- | --- |
| GitHub 下载与手动安装路径明确，无 Sparkle/开发者账户/公证依赖 | PASS。固定路径/清单/发行说明及手动交接有代码、测试和安装说明；真实候选交接到用户隔离目录。GitHub 对应 Release/资产不存在，远端下载仍 NOT_RUN；按本轮用户接受的本地候选原型范围，不阻塞原型收口 |
| app/helper/kernel/resources 版本绑定及更新退出顺序明确 | PASS。清单/摘要/封印/版本检查一致，安装说明明确完整组合和退出顺序；helper只嵌入原型，正式升级/管理员替换由P6/P7承接 |
| 下载复用显式客户端，真实浏览器 quarantine 与首次打开记录，系统保护保留 | PASS。原Edge真实下载、原ZIP、Finder解压、完整首次警告、正常单应用授权与实际窗口/退出证据连续绑定同一候选 |
| Apple Silicon 最低 ad-hoc、内嵌二进制验证且可运行，不冒充发布者身份/公证 | PASS_LOCAL_PROTOTYPE。四个arm64/ad-hoc及包封印通过；macOS27.0.1上正常系统授权后GUI实际运行、托盘恢复/正常退出通过。仅证明这台设备与这份原型候选，不证明全部受支持系统或正式产品安装 |

剩余代码 Finding：**0**。剩余本轮原型验收阻断：**0**。保留的限制是无 GitHub 对应远端资产、未捕获中间密码授权弹窗和托盘菜单，以及正式 production helper 安装/完整更新替换、回滚与正式发行留在原P6/P7任务。这些不被改写为已执行。

审查时当前任务文档仍保留上一轮“当前 ACCEPTANCE/首开 NOT_RUN”描述，维护者应追加本轮真实通过记录并更新当前状态/READY；不要覆盖历史失败。提交/合并/主分支复验尚未由本 Reviewer 执行，其结果须另记真实 SHA 与退出码。无 push 或 Release 创建授权。
