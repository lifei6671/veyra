# P0-08 独立 Review（2026-10-10）

结论：**工程复审通过，未发现剩余可操作代码 Finding；macOS 首次打开验收未通过，Task 必须保持 ACCEPTANCE。** 静态签名通过不能证明浏览器下载后的应用可运行，不能 DONE、提交已完成任务或提前合并。

本记录由独立 Reviewer 按 `code-delivery-review` Skill 作只读源码与证据审查，仅新增本文件。未重复执行测试、GUI、helper、网络设置、Git 写入或发布。范围为独立树 `dev/p0-08-update-distribution` 相对基线 `733b6ea3d20b23ba90bb933971a08b076bc0caa3` 的更新 example、清单/打包工具、定向测试和安装说明，包含未跟踪源码；已阅读本目录 JSON/日志及三张 macOS 原图。

## Finding 与修复复查

**P0-08-PYTHON-OPTIMIZE-001：CLOSED。** 原工具将 archive 来源摘要、资源大小/SHA、架构、版本及 ad-hoc 检查写成 `assert`；`python3 -O` 删除这些检查后，错误 archive 可继续构建/提取，验证反馈也失去声明保障。现有 `tools/p0-08-package.py` 全部相关检查均改为显式条件失败，在创建输出/调用 Cargo 前拒绝错误 archive。新增 `test_optimized_python_still_rejects_wrong_archive_and_corrupt_bundle` 真正调用优化解释器，检查错误 archive 无输出，以及破损内核在执行前因摘要错误被拒绝；实际测试日志通过。修复聚焦原契约，没有引入新依赖或框架。

**剩余工程 Finding：无。** 已复核固定 `lifei6671/veyra` Releases API、稳定版本数字比较、固定 tag/资源 URL、app/helper/kernel/resources 清单绑定、P0-06 `client_builder` 的显式出站/TLS/超时与手工重定向、GitHub CDN 限制、流式二进制大小/SHA-256、取消/失败清理本次 `.part`、已有文件保留，以及仅返回手动安装交接而不安装的行为。helper 明确是 P0-05 原型，不冒充 P2-06 production。内嵌资源先签名并计摘要，再签外层应用；官方 kernel 保持固定字节及已有 linker ad-hoc。未见另建代理下载体系、重复持久事实或错误安装成功反馈。

相对基线，Core 正式源码、Desktop、Helper、`src-tauri`、workspace `Cargo.toml` 和 `Cargo.lock` 的只读 `git diff --quiet` 为 exit 0；Core 局部 Cargo 仅注册新 example。P2-05/P2-06 代码与公共契约未受影响。

## 工程与包证据

| 核验项 | 实际结果与依据 |
| --- | --- |
| Rust updater 定向测试 | `update-tests.json/.log`，exit 0，8 passed / 0 failed / 0 filtered |
| 真实包正反测试 | `package-tests.json/.log`，exit 0，8/8；含 zip 解压往返、资源缺失/多余、版本混用、内核破损、封印篡改、错误 archive、优化解释器拒绝 |
| 新包构建 | `bundle-build.json/.log` 与 `build-0/1/2.log`，exit 0；GPUI 构建保留第三方 `block` future-incompat warning，不改写为无警告 |
| Core Clippy | `core-clippy.json/.log`，`--all-targets -D warnings`，exit 0 |
| fmt | `fmt.json/.log`，指定 legacy manifest 的 `--check`，exit 0 |
| 离线版本/清单检查 | `manifest-inspect.json/.log`，exit 0；本地 Release fixture，不是远端发行验证 |
| 最终 Bundle 静态验证 | `final-static-verify.json/.log`，exit 0，明确 `PASS_STATIC_ONLY`；四个 arm64 Mach-O 均 ad-hoc，完整资源封印与固定 kernel 摘要通过 |
| Legacy Clippy | `legacy-clippy.json/.log`，exit 101；构建脚本缺 `binaries/sing-box-1.14.0-windows-amd64/sing-box.exe`。基线相关文件未修改，保留失败，不扩展本卡修复 |

独立读取 `build-inputs.json` 的 120 项输入并重新计算 SHA-256，全部与当前源码一致。独立重新计算生成 ZIP、浏览器下载 ZIP 及四个最终二进制摘要，与收据一致。最终包为 `target/p0-08-desktop/VeyraPrototype.app`，ZIP 大小 **54,830,834 bytes**，SHA-256 **`fdb7472d4792bf4f7f88568f3c1f288746da5efcf55cdf0bded7b28262221a26`**。app/helper/resources 0.1.0、kernel 1.14.0；整体最低 macOS 26.0，来源为 kernel 实际 minos，不能以 GPUI/helper/updater 的 15.0 编译目标声称整个组合支持 15.0。

## macOS 验收边界与剩余阻断

**P0-08-MACOS-FIRST-OPEN-001：OPEN（验收阻断，根因尚未完整证明）。** 真实 Edge 从受控 loopback 下载 ZIP 的截图、where-from 元数据及 `browser-artifact.json` 支持浏览器下载事实；该 ZIP 大小/摘要与发行候选完全一致。ZIP 及 Finder 解压 app 均记录 `com.apple.quarantine`；`spctl --status` 为 assessments enabled，SIP enabled。解压 app 打开前的 `codesign --verify --deep --strict` exit 0，主二进制摘要与新候选一致。

随后 Finder 首次打开没有确认出现应用 GUI。`first-open-system.log:66`、`:252` 有 amfid `Code=-423`（ad-hoc/unknown certificate chain），`:206` 有 Gatekeeper denial breadcrumb；日志另有 launchd/RunningBoard 的 signal 9 终止记录。`first-open-process.json` 未发现存活候选进程。`first-open-no-window.png` 展示 Finder 中仅剩 ZIP，不能据此还原原系统提示或证明 GUI 曾可运行。原提示原文/截图未捕获，原解压 app 后来不在原位，原因 **UNPROVEN**；不得推断系统删除、用户删除或包自身删除。

因此本轮结果应为：浏览器 loopback 下载/摘要/quarantine **PASS**；首次打开 **FAIL / ACCEPTANCE_NOT_COMPLETED**；正常单应用授权后打开、真实窗口、托盘退出及手动安装/替换 **NOT_RUN**。恢复该验收只能保留现有系统保护与 quarantine，捕获完整原提示并走正常单应用授权路径后复验；不能以终端直接启动、移除 quarantine、关闭 Gatekeeper/SIP 或改签身份替代。当前证据不足以宣称“Apple Silicon 可运行”。

GitHub Releases 页面原图为无 Releases，`github-latest.json` 与 headers 为 HTTP 404。真实 GitHub 资产下载 **NOT_RUN：无对应远端产物**；loopback 浏览器下载只证明下载/quarantine链路，不能冒充 GitHub CDN 实测。未创建 Release。

`takeover.json` 支持原执行 cancelled 后接管。`macos-result.json` 记录独立树本轮缓存清理、fixture 停止请求及无本次存活应用进程；既有主树未跟踪缓存按原状保留。审核未发现源码夹入临时缓存，但运行资源退出结论仍以实际记录为边界，不从静态源码推断系统状态。

最终验收意见：工程与静态包结果可保留为 PASS；保持 **ACCEPTANCE** 和独立分支。首次打开/正常授权/GUI/托盘退出/手动交接的真实证据未齐，禁止提前 DONE 或合并；无 commit SHA、merge SHA 或正式发布结论。
