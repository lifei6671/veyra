# P0-05 helper prototype

This task-card path contains only the isolated `p0-05-prototype` entry, named
`veyra-helper-prototype`. It is not the production P2 helper, exports no library,
and is not a dependency of `veyra-core`.

macOS arm64 only; builds require `MACOSX_DEPLOYMENT_TARGET=15.0`:

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo check -p veyra-helper-prototype --features p0-05-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo build -p veyra-helper-prototype --features p0-05-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo test -p veyra-helper-prototype --features p0-05-prototype
MACOSX_DEPLOYMENT_TARGET=15.0 cargo clippy -p veyra-helper-prototype --features p0-05-prototype --all-targets -- -D warnings
cargo fmt -p veyra-helper-prototype -- --check
```

The pure tests protect closed requests, bounded framing/deadlines, exact or
conflict-aware proxy restoration, fixed installation/plist/cleanup paths, protected
ownership/modes/symlink rejection and startup failure classification. They do not
install a daemon, query system launchd, launch a kernel or modify SystemConfiguration.
The orchestrator path/preflight tests also use only temporary resources:

```sh
python3 -B -m unittest discover -s tools/tests -p 'test_p0_05_helper_probe.py'
```

`local-platform-probe` separately tests ordinary
user OS peer credentials and process exit events in an exclusive temporary socket.

The [fixed orchestrator](../../tools/p0-05-helper-probe.py) downloads/checks the
official archive as the ordinary user, demonstrates manual loopback/FD input,
then invokes the standard macOS administrator UI. Do not launch it with sudo;
do not provide passwords in arguments, files, environment or chat.

```sh
python3 tools/p0-05-helper-probe.py --output /private/tmp/veyra-p005-replay-evidence
```

Use a new empty output directory. Existing P005 protected resources cause refusal
without mutation or cleanup. `--no-authorize` records ordinary-user evidence only.
The administrator explicitly authorizes a fixed `privileged-probe <business UID> <verified helper SHA256>`
command; its root harness installs only the verified local helper/kernel,
creates one disabled service outside every Network Set, drops credentials for
clients, exercises fixed operations and finally restores/stops/bootouts/deletes.
Daily IPC has no payload UID/PID/path/service/config fields.

### daily cycle 分步诊断（代码事实）

Host 已在同一台 macOS / Rust 1.99 上最小复现：Unix peer 已 close 后重新设置
`SO_RCVTIMEO` 返回 `EINVAL(22)`，即使完整响应仍在本端缓存；正常连接上的动态
timeout 和小 Duration 均通过，因此不是 Duration 截断为零。此前 Desktop 失败
定位到 cycle 1 / step 1 / Status / client.read 的 `read_frame` 动态 timeout 设置。

`read_frame` 现以 `poll` readiness 和 `Instant` 单调总 deadline 读取，完整 frame
仍使用调用方的 500ms / 2s / 15s 预算。HUP/ERR 后先用 `recv(MSG_DONTWAIT)` 读取
缓存，再按 newline/EOF 判定；单次非阻塞接收避免 readiness 失效后的阻塞。
EINTR/EAGAIN 按原 deadline 重试，poll 毫秒向上取整并在返回后复查 deadline；
超时返回 `MessageDeadlineExceeded`，无 newline 的 EOF 仍为 `incomplete message`，
payload 超过 4096 字节仍为 `MessageTooLarge`。client connect 的固定 15s 读超时、
daemon 的 500ms 读超时及 local probe 的 EOF 读超时设置已移除，读取共用 readiness
deadline 这一 authority；既有写超时保持不变。这些代码事实不代表特权真实复验 PASS。

两轮顺序仍为 `Status → StartSystemProxyTest → Restore → Stop`，每步完成立即输出并
flush 一行 NDJSON，索引均从 1 开始，`step_index` 在每轮重新从 1 计数：

```json
{"event":"cycle_step","cycle_index":1,"step_index":2,"operation":"StartSystemProxyTest","client_phase":"response","outcome":"daemon_error","daemon_error":"StartSystemProxyTest.child_spawn.pre_exec.setgid: Invalid argument (os error 22)","result":{"response":{"ok":false,"error":"StartSystemProxyTest.child_spawn.pre_exec.setgid: Invalid argument (os error 22)"}}}
```

以上是格式示例，不是实测结果。成功行为为 `outcome=ok`；client 失败为
`outcome=client_error`，`client_phase` 区分 `connect/server_peer/write/read/parse`，
并带 `client_error`。daemon 返回 `ok=false` 时为 `client_phase=response`、
`outcome=daemon_error`，保留完整原 response 和 daemon error。全部成功后仍输出旧
`{"cycles":[...]}` 聚合行；失败立即停止后续步骤。root harness 的
`cycles_diagnostics` 保留 `capture`（code/stdout/stderr，超时另带 capture_error）、
`steps`、`summary`、`parse_error`，并在检查退出码之前登记，前序结果不会被丢弃。
正常 `cycles` 字段保持旧成功聚合结构。

`StartSystemProxyTest` 标注 owner watch/identity、service ID、network read、secret、
config save/read_private/pipe、kernel hash/protected、child spawn/watch/identity/readiness、
recovery save、network write/readback 和失败清理阶段。`save` 进一步标注 temporary
open/write/file sync/rename/directory open/sync；rename 后的 sync 失败不能证明文件
尚未保存。`pre_exec` 只用固定整数编码保存 dup2/fcntl/setsid/setgroups/setgid/setuid
及原 errno，父侧还原文字；fork 后不构造字符串、不分配、不绕过降权或失败。
native bridge 的错误含 read/write/create/delete operation 和具体 API，分别区分
SetConfiguration、CommitChanges、ApplyChanges；调用顺序和短路语义不变。

以下保留其他 I/O/errno 诊断入口；本次已确认的是上述 peer-close 后重新设置读超时，
不能据此把其他入口也认定为 EINVAL 根因：

| 路径 | 原裸错误入口与判断 |
| --- | --- |
| client `connect/peer/request/read_frame` | protected/settings 的 metadata/open/read、UnixStream connect、getpeereid、getsockopt LOCAL_PEERPID、write timeout 的 setsockopt、IPC write、poll/recv；读路径不再 setsockopt。错误仍保留 client phase/调用上下文。非正 PID 单列无效 PID；parse/序列化错误属于 serde。 |
| daemon Start / Restore / Stop（待分步确认） | watch 的 kqueue/fcntl/kevent；service/config/kernel/random 文件 open/read/write/sync/rename；child spawn（含六个 pre_exec syscall）；readiness TCP connect/timeout/write/read；wait_child/try_wait、Stop remove_file。业务错误经 daemon `ok=false` 包装，原 cycle stderr 应为 response JSON，因此与已提供的纯 errno 文本吻合较弱。目录 sync 在 rename 后，write/apply/readback 失败也不能推断没有部分副作用。 |
| daemon 启动/accept（间接） | bind/chown/chmod/nonblocking、accept、已接受 stream nonblocking/timeouts、Shutdown socket remove；部分设置在 response 包装之外失败可退出 daemon，再表现为 client transport 失败。启动已 PASS 不足以排除日常 accept；后续 Shutdown IPC 成功也不能排除一次性失败。 |
| root harness `probe.rs` | client spawn/降权、pipe read/read_to_end、try_wait/kill/wait、metadata/service/config read、owner kill；这些裸 I/O 会在 harness 层传播，但本次已提供的是成功 capture 到 client exit1/stderr22，因此 capture 自身 spawn/read/wait 不直接吻合。owner/reject/conflict 后续路径尚未进入。 |
| 安装/卸载、ordinary-user local probe | copy/open/read/write/sync、create/chmod/rename/remove、launchctl Command spawn/output/wait；local bind/connect/timeouts/IPC、child kill/wait。都有 I/O 边界，但不是 daily cycle 的直接调用链；保持既有权限和资源契约。 |
| native `network.m` | journal open/write/fsync、proc_pidinfo 保存 errno 后以 JSON 返回；SC API 使用 sc_error/detail，编码失败也是字符串/JSON，均不是直接产生 Rust 裸 std::io::Error 的入口。既有 kill/close 和 probe 的未检查 fcntl 不直接传播裸错误；child readiness 的 fcntl 现检查并标注 stream，不把这些调用凭空认定为根因。 |

下一次 Desktop 真实复验只需先观察 `cycles_diagnostics.steps` 的最后一项：轮次、
step、operation、outcome、client_phase；transport 错误看 client_error 中具体调用，
daemon 错误看 stage（pre_exec syscall / native API / save 子阶段）。前序 steps 和
原始 capture 一起保留，用于判断已完成动作；network write/readback 或 cleanup
失败仍须按现场 recovery/独立 readback 判断副作用，不能把诊断 stage 当作无写入证明。

### child readiness 分流诊断（代码事实，未做新的特权复验）

`src/p0_05/readiness.rs` 分别保存 stdout/stderr 字节与行边界；只对完整 newline 行
或 EOF 的最后一行解析固定 mixed/controller 日志，允许两端口分别来自任一 stream。
关键词、`127.0.0.1` 地址、非零 u16 端口保持与 manual parser 同一语义，不扫描
监听端口、不改变 sing-box 配置/log level。日志里的其它地址或尾部附加文字不能被
`rsplit_once` 猜成端口；partial 数字在行完成前不作为端口事实。

pipe 设置 nonblocking 的 fcntl 失败带 stream 名称；WouldBlock 继续下一轮，EINTR
重试，EOF 单独标记，其它读错误明确返回。每轮检查 `child.try_wait()`，检测退出后
drain 两流并立即返回 `StartSystemProxyTest.child_readiness.child_exited`；端口解析和
既有 controller 鉴权期间的退出也检查。失败后的 SIGTERM/wait、恢复记录和代理写入
顺序不变，诊断不会触发额外 SystemConfiguration 写入或恢复动作。

失败 IPC response 新增 `readiness`；成功后同一有界值保存在 Instance，并通过
`response.result.status.instance.readiness` 暴露。现有 cycle steps/raw capture 已保存
完整 response，因此下一次 privileged probe 的成功和失败路径均可留存它，无新增
持久化文件或 IPC operation。结构为：

```text
stage, outcome, child_pid, child_identity,
stdout/stderr: {text, eof, captured_bytes, truncated, overflow},
mixed_port, controller_port, elapsed_ms,
exit_status: null | {display, raw, code, signal, success}
```

`child_identity` 使用进入 readiness 前已通过 `proc_pidinfo` 校验的实际 UID/GID、
PGID、PID 和 start_sec/start_usec，测试注入值不冒充真实身份。两个捕获 buffer
各最多 32768 bytes、总量最多 65536 bytes，超出即失败；每流导出的 text 含 JSON
转义最多 768 bytes，以适配原 4096-byte frame。text 是有界原文片段，truncated
为 true 时不是完整日志。导出前固定替换本次 secret 和末尾可能未读全的 secret
前缀，再截断；HTTP 鉴权请求和响应原文不记录。

新增纯测试保护交错分块/跨流端口、提前退出/末尾 drain、单流 EOF、WouldBlock/
EINTR/读错误、内存与转义预算、secret 替换、timeout/UTF-8 和错误端口拒绝。
这些测试不启动真实 child、管理员授权或系统服务。

下一次 Desktop 真实复验须保存上述有界原始 stdout/stderr、EOF/截断标记、实际
child_identity、已识别端口、elapsed_ms、stage/outcome 和 exit_status；用这些字段
区分 child 提前退出、运行但日志/解析未就绪、pipe 错误和 controller 鉴权失败。
此前观测修订仅修复观测契约，当时未确认 sing-box/FD3 根因、未声明真实 child readiness PASS；
历史 evidence 与 P0-05 ACCEPTANCE / DAG 状态保持原样。

### FD3 anonymous pipe 修复（代码事实，无特权验证）

Host 已在同一台 macOS 最小复现：先 open regular file 再 chmod 000，直接读取
已继承的 fd 成功，但 `/bin/cat /dev/fd/3` reopen 返回 Permission denied；匿名
pipe read-end 经同一路径读取成功。macOS 对 regular vnode 的 `/dev/fd/N` reopen
仍检查底层文件权限，root 打开的 0600 文件不能因此被降权 child 重新打开。

`save("runtime/config.json", ...)` 继续原有 root-only 落盘契约。父侧以
`O_NOFOLLOW` 打开刚保存的文件，对实际 inode 校验 regular、UID 0、精确 0600，
只读取这些持久化字节送入匿名 pipe，避免双重序列化。固定配置当前 425 bytes；
读取和 pipe 写入均限制为 512 bytes（POSIX 最小原子 pipe 写大小）。write end
额外设 `O_NONBLOCK`，`write_all` 处理 EINTR/partial write；超限或 WouldBlock
直接失败，不依赖未经检查的容量假设，不创建 writer thread。

pipe 两端显式设置 CLOEXEC，唯一 writer 在 spawn 前关闭。`pre_exec` 将 read end
dup2 到 FD3 并显式清 CLOEXEC（包括原 read fd 已为 3 的分支）；非 3 的原 read fd
保持 CLOEXEC。父侧原 read end 在 spawn 成功后立即关闭，失败由 RAII 释放。
child 仍依次执行 setsid/setgroups/setgid/setuid，固定 kernel/hash/protected 检查
和 `run -c /dev/fd/3` 不变；未放宽磁盘权限，没有 user-readable config、stdin、
environment 或 argv 配置传输。readiness/退出/cleanup 时已无父侧配置 pipe 端点
或 writer；child 自己的 FD3 随既有 child 退出清理。

成功和 readiness 失败诊断的 `readiness.config_input` 安全记录
`config_transport="anonymous_pipe_fd3"`、`config_bytes`、`config_sha256`、
`disk_config_mode="0600"` 和 `fd3_contract` 的代码事实。摘要来自刚读回并实际
送入 pipe 的相同字节，不记录 JSON/secret；代码事实不冒充特权 fd 实测结果。
现有 cycle capture / Status 保留这一有界结构，没有新增 IPC operation 或状态模型。

无特权测试使用自有 tmp、`/bin/cat` 和 `/usr/bin/python3`，保护 regular-file
reopen 拒绝、pipe FD3 完整读取/EOF、两端 CLOEXEC、FD3 同号分支、超限拒绝、
EINTR/短写、spawn 失败映射、磁盘权限/内容/摘要一致性，以及新增诊断的 IPC 大小预算。
测试不启动 sing-box、root helper、
launchd 或 SystemConfiguration。下一次 Desktop 真实复验需保留上述
`readiness.config_input`，同时观察已有 stage/outcome、实际 UID/GID/PGID、
exit_status、stdout/stderr、controller 鉴权与后续 cycle/cleanup 结果。
本修复未执行新的特权 C 验收，不声明 C PASS；历史 evidence、任务/DAG 保持原样。

The traditional LaunchDaemon uses label `com.lifei6671.veyra.p005` and fixed plist
`/Library/LaunchDaemons/com.lifei6671.veyra.p005.plist`. Its `ProgramArguments` are
`/Library/Application Support/VeyraP005/helper` and `daemon`. Helper, fixed `sing-box`,
authorization and runtime resources all reside in `/Library/Application Support/VeyraP005`;
the helper is not installed in `/Library/PrivilegedHelperTools`. The root-owned
directories and files reject symlinks and group/world writes. Staging and installed
helper/kernel digests remain checked; uninstall validates fixed targets before
removing the dedicated ROOT, including its helper.

After bootstrap, socket waiting samples only `launchctl print` for the fixed label
with a bounded query. An exited/inactive job returns early with launchd status,
fixed installed helper/plist identities and up to 8192 bytes of fixed `daemon.log`,
captured before uninstall. Missing/empty logs are diagnostic states, never readiness.
Pending launch remains subject to the socket deadline. This code correction is not
proof of a successful privileged launch: the 2026-10-06 Desktop failures remain
historical evidence, P0-05 remains ACCEPTANCE, and the new path/startup diagnostics
still require real Desktop revalidation.

The daemon uses `getpeereid`, `LOCAL_PEERPID`, `proc_pidinfo` and kqueue `NOTE_EXIT`.
The root-only 0600 disk config bytes are passed through anonymous pipe FD 3;
the child creates an owned process group, clears groups and sets gid/uid before exec.
Only that fixed installed
sing-box may execute. HTTP/HTTPS/SOCKS/PAC/discovery/bypass restoration compares
related field groups; native writes recheck the observed dictionary under the
SCPreferences lock. Root service writes require the recorded service to remain
disabled, P005-named and absent from all Network Sets.

If restore fails, the daemon retains the endpoint and recovery record, and the
uninstaller refuses removal. A prior recovery record prevents a new writer.
This P0 entry deliberately does not implement production restart recovery,
multi-service support, TUN, updates or GUI. Never treat successful compilation
or ordinary-user API tests as proof of privileged platform acceptance.

Current outcome and exact limitations: [P0-05 evidence](../../docs/openbox-rust-gpui-tasks/evidence/p0-05/README.md).
