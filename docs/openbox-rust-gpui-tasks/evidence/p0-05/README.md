# P0-05 本地 helper、授权与进程归属原型

**ACCEPTANCE，未 DONE。** 起始 HEAD `3111a57e0444ad5c8fe4e2ae5707e343e44867f3`、工作树干净；Codex /root · Runtime/Platform 单一 owner。只领取本卡，未启动 P1-03/下游；未访问 OpenBox、未做视觉对齐、未 commit/push。

已实现任务卡路径 `crates/veyra-helper/` 的显式 `p0-05-prototype` feature / `src/p0_05/` 二进制入口，包名 `veyra-helper-prototype`；[说明与复跑](../../../../crates/veyra-helper/README.md)。core 不依赖此原型或平台 crate；根 workspace/lock 仅增加原型成员及已有依赖关系。原生 Objective-C adapter 直接使用 SystemConfiguration，未使用 networksetup。

## 实际授权缺口

[install-uninstall.json](install-uninstall.json)记录一次标准系统路径：普通用户侧先验证官方固定 archive/binary，再执行 `osascript do shell script "<tmp>/helper privileged-probe <authorized-uid>" with administrator privileges`。不收集、保存或传递密码，也不依赖 sudo ticket。

实际退出码 **1**：`Error received in message reply handler: Connection invalid`、`Connection Invalid error for service com.apple.hiservices-xpcservice`、`授权失败。 (-60008)`。管理员测试入口没有执行；未获得 root，未创建 LaunchDaemon 或 Network Service。没有真实用户取消证据，不将该系统错误伪造为“用户取消”，也不将签名/证书缺失当成阻塞。本机标准授权 UI 无法安全获得是当前缺口；没有使用另一权限入口绕过系统限制。

`attempted_helper_sha256` 绑定实际授权尝试当时的二进制；[platform-identity.json](platform-identity.json)绑定最终修正/显式入口构建的产物。最终产物没有重新尝试已确定失败的管理员授权；该缺口覆盖所有特权分支。

## 五项验收

| 项 | 当前事实 | 判定 |
| --- | --- | --- |
| 无 Developer ID 安装运行、protected dir、OS peer UID/PID | 当前 macOS 27 arm64 本地运行；minos 15.0、adhoc/linker-signed、TeamIdentifier not set、codesign verify 0；普通用户 OS peer 成功。root 安装和目录保护未执行 | PARTIAL，未勾选 |
| 拒绝授权保留 manual、批准后重复不需授权 | 非 root install 精确 AdministratorRequired、无受保护写入；真实普通用户 manual/controller/FD/清理 PASS。真实取消及批准后的两轮日常 IPC 未执行 | PARTIAL，未勾选 |
| SystemProxy child 业务身份与受保护读取 | 固定官方 binary、普通用户身份与继承 FD 兼容性 PASS；root helper 降权 child/读取 root-only 资源未执行 | NOT_RUN |
| GUI SIGKILL 后恢复/清 child，瞬断不清理 | 普通用户 peer + kqueue 独立探针：EOF 后存活 2 秒、SIGKILL/NOTE_EXIT/回收 PASS。root helper 实例与测试服务恢复/child group 未执行 | PARTIAL，未勾选 |
| 未授权用户与任意 command/path 拒绝、权限不足精确报告 | 封闭协议/超限/总时限单测 PASS；真实不同 UID socket ACL/server check、非法 IPC 无副作用未执行；管理员失败精确记录 | PARTIAL，未勾选 |

五项需要真实特权结果才能全部通过。P0-06/P0-08 未释放 READY；P0-09 仍缺其他显式依赖。

## 已运行的普通用户事实

- [平台身份](platform-identity.json)：Rust/cargo 1.99.0、macOS 27.0 (26A428)、arm64；helper/client 是同一显式原型产物。LC_BUILD_VERSION minos 15.0 / SDK 27.0；`Signature=adhoc`，不是 unsigned。macOS 15 仅证明 minos，**没有 macOS 15 实机结果**。
- [内核身份](kernel-identity.json)：官方 v1.14.0 darwin-arm64 archive SHA256 `a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`，binary SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，与 P0-04 一致；普通用户 digest 匹配后才执行，root 下载为 0。P0-04 cache/handoff 只作依赖，不声称本轮跨 UID cache 交接已验证。
- [manual fallback](manual-without-helper.json)：helper 不存在，非 root install 拒绝后，普通用户启动真实固定 sing-box，纯 loopback mixed/controller、鉴权 `/version` 200，继承 `/dev/fd/<inherited-fd>` 配置读取成功，proc_pidinfo 读取 real/effective UID/GID，SIGTERM/exit 0/wait/group/listeners 清理。system proxy writes=0，没有公网、TUN 或外部 DNS。
- [普通用户 OS API](local-os-api.json)：accepted socket 用 `getpeereid()` 取得 UID/GID、`getsockopt(SOL_LOCAL, LOCAL_PEERPID)` 取得真实 child PID；客户端也核对服务端 OS peer。持有的 child 与 peer PID 相符；socket EOF 后进程继续 2 秒，kqueue 没有 NOTE_EXIT；SIGKILL 后收到 `EVFILT_PROC/NOTE_EXIT` 并回收。服务端是普通用户，**不计 root server 身份、protected socket 或系统代理崩溃清理通过**。

## 已实现但未运行的特权路径

独立身份：label/helper `com.lifei6671.veyra.p005`；plist `/Library/LaunchDaemons/com.lifei6671.veyra.p005.plist`；root `/Library/Application Support/VeyraP005/`；socket 在该 root 下。不会覆盖正式 helper 路径或真实 Veyra 配置/缓存。

安装入口只复制已校验的本地 helper/固定内核，拒绝已有原型资源；目标 root-owned、普通用户不可写，socket 授权 UID/GID + 0600 且仍检查 peer；显式业务 UID 在安装时登记，日常请求不带 UID/PID。协议只有 Hello/Status/StartSystemProxyTest/Stop/Restore/ShutdownForUninstall；4096-byte 输入上限、整个消息 500 ms 预算。不存在 shell/executable/config/service/PID 透传。

root helper 配置 0600、runtime 0700；打开 config 后继承 FD 3；child pre_exec 按 setgroups→setgid→setuid 降权并 setsid，固定 protected kernel hash 再核对。owner/child kqueue FD 设置 CLOEXEC；peer UID/GID 与 proc_pidinfo 独立核对，socket 断开不撤销实例，同 PID 重复 Start 不启动第二个 child。

测试服务仅使用现有物理硬件接口，名称 `Veyra-P005-Isolated-Disabled`，始终 disabled、不加入任何 Network Set、不建立 IPv4/IPv6 主用路径。所有 native read/write/delete 只作用于安装记录的服务 ID，并重新验证 disabled/名称/不在 set；client 不能选择写目标。快照包含整个 Proxies 字典，包括 HTTP/HTTPS/SOCKS、PAC URL/enable、discovery、exceptions/simple hostnames 和其余未改字段；恢复按相关字段组比较，外部值保留且报告 conflict。native write 在 SCPreferences lock 下再对 observed/expected 核对，发生竞态拒绝写入。

先持久化 recovery，再 commit/apply，独立 session 回读；恢复成功才停止自有 child、wait 和 group 检查。恢复失败且仍引用自有 endpoint 时保留 child/state，标记 RecoveryRequired；卸载拒绝删除。固定 root harness 的 finally 先等待真实 owner NOTE_EXIT、restore/stop，再 shutdown/bootout/delete；失败不删除恢复记录。完整生产多服务/重启恢复/TUN 留给 P2/P6。

[ipc-peer-identity.json](ipc-peer-identity.json)、[child-identity.json](child-identity.json)、[network-service.json](network-service.json)、[owner-lifecycle.json](owner-lifecycle.json)均明确 **NOT_RUN**。当前没有 root-owned mode/UID/GID、不同 UID ACL、两轮 SystemConfiguration 真实写回恢复、外部 PAC 改写或管理员卸载的实测值；不能把上述实现描述成 PASS。

## 验证、历史与清理

[validation.json](validation.json)记录最终源码/lock/产物 hash、命令、5 个实际测试及文档/DAG/脱敏检查。测试保护 Veyra 自有封闭请求、超限与总时限、完整快照恢复和外部 PAC 字段组保留，不启动 root、sing-box 或写系统设置。prototype check/build/clippy/fmt 与 core check PASS。旧入口标准 lib clippy 因基线 Windows LICENSE 资源缺失 FAIL；resource-only TAURI_CONFIG 覆盖的 scoped lib clippy 单独记录，不代表打包成功。

历史：[首次普通探针](history/attempt-01/probe-error.json)因 sandbox 禁止 ps 在管理员阶段前终止；[初轮清理](history/attempt-01/cleanup.json)仍 PASS。后改用 proc_pidinfo。local peer 初次在握手前关闭 socket得到 [ENOTCONN](history/local-api-attempt-01.json)，二次发现 macOS accepted socket 继承非阻塞导致 [EAGAIN](history/local-api-attempt-02.json)；已用握手及显式 blocking 修正，并实际复跑 PASS。首轮 Rust packed-struct 编译错误和 collapsible-if clippy 失败保留在 validation 的历史说明；不改写为通过。

[cleanup.json](cleanup.json)：本轮 staging/archive/binary/config/local socket 已删除；普通用户 child 均 wait，group/listeners 关闭；精确 launchd label 不存在；所有 `/Library` 原型 path/plist/socket 不存在。管理员入口未执行，所以没有创建过测试 Network Service 或 root child。没有修改当前主用网络，也不存在待恢复的系统代理现场。

按 [code-delivery-review](../../../../.agents/skills/code-delivery-review/SKILL.md)作实现者自查：修复 accepted socket 非阻塞继承、总消息时限、降权顺序、kqueue CLOEXEC 和写前 observed 核对；不声称独立 Review。特权集成未验证是明确验收限制。后续只需在标准管理员 UI 可安全呈现的本机环境复跑固定编排，完成其真实分支及清理，再判断 DONE；不恢复旧人工流程门禁。
