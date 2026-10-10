# P0-08 macOS 首次打开最终证据（2026-10-10）

结果：PASS_NATIVE_PROTOTYPE。只验收已浏览器下载的原发行候选，没有重新构建、签名或终端启动。旧 desktop-20261010 首开 FAIL 及 Review 原样保留。

1. [ZIP身份](zip-identity.json)：原文件及保留属性的 Downloads 副本均 54,830,834 bytes / SHA256 `fdb7472d4792bf4f7f88568f3c1f288746da5efcf55cdf0bded7b28262221a26`，与原 `veyra-release.json` 一致。路径 `/Users/lifeilin/Downloads/Veyra-P0-08-first-open-20261010/`，原 ZIP 和 App 均保留。
2. Finder 双击 ZIP，经系统归档工具新解压；[App初始身份](finder-extracted.json)为 0.1.0、最低 macOS26.0、全部文件哈希与原候选一致。App/ZIP 均保留 Edge quarantine `0081;6ac998ed;Edge;BD4479A9-E9E8-4BEC-A8F5-41E4B65E9FD1`，深层严格签名验证 exit0。
3. Finder 首开 [原提示文字](gatekeeper-prompt.txt) / [原截图](gatekeeper-prompt.png)：未打开“VeyraPrototype.app”；Apple无法验证是否包含可能危害Mac安全或泄漏隐私的恶意软件。仅选择“完成”，没有选择“移到废纸篓”。系统设置 → 隐私与安全存在 [单应用仍要打开原图](open-anyway-visible.png)，全局来源仍为 App Store 与已知开发者。
4. 实际用户在该系统页面完成“仍要打开”及系统确认，回复“已完成系统授权并打开”。中间密码/授权弹窗未截图，不宣称自动化完成或读取密码。[运行身份](running-identity.json)证明 PID16877/UID501 的 AppTranslocation 全部文件与新解压候选完全相同，主程序SHA `367e25f453abc580b779ee7508f3f050b71074ae9f57c7e11188f54b3d640954`，签名仍 exit0。系统授权将 App quarantine 标志变为 `00c1`，属性与 Edge 来源仍在；ZIP标志仍0081。本Agent没有任何 xattr 写/删除动作。
5. [真实主窗口](gui-initial.png) / [AX](gui-initial.txt)，[中文粘贴/主题切换/模拟托盘状态交互](gui-interaction.png) / [AX](gui-interaction.txt)。只操作原型合成数据，没有网络业务或管理员helper。
6. 自动化点击真实关闭按钮隐藏窗口，[关闭后同PID仍在](after-close-process.json)。原生自动化不暴露系统托盘图标/菜单，实际用户展开绿色原型托盘，点击“显示窗口”并回复“窗口已恢复”；[恢复原图](gui-tray-restored.png)与[同PID记录](tray-restored-process.json)确认中文草稿和深色状态保留。托盘菜单自身未截图，证据明确为用户实际操作确认 + 实际窗口/身份记录。
7. 实际用户点击同一托盘“退出”并回复“已通过托盘退出”。[限定原系统日志](first-open-system.log)第2318行：16877 `termination reported by launchd (0, 0, 0)`；第2330行 `Process exited: <RBSProcessExitContext| voluntary>`。系统随退出清理16892/16893两个普通AppKit/AutoFill XPC，它们不是Veyra管理员helper。[退出清理](exit-cleanup.json)、[所有PID消失](all-owned-processes-absent.json)、[最终原生清单](final-app-inventory.json)。只关闭本轮Finder窗口和本轮系统设置，用户其它窗口保持。
8. Gatekeeper assessments / SIP 均 enabled；无Veyra管理员文件或daemon，未运行嵌入helper/内核或修改系统代理/TUN/路由/DNS。GitHub真实远端资产下载仍 NOT_RUN_NO_RELEASE_ASSETS，本轮用户允许本地浏览器候选原型收口；没有自行发布。

[结果收据](result.json) / [独立 Review](REVIEW.md)。所有只读命令退出码在相应JSON，完整系统日志不改写；ps/pgrep exit1表示目标不存在。构建时收据中的GUI/首开NOT_RUN代表构建时事实，最终实机结果另由本目录记录，禁止覆盖原收据。
