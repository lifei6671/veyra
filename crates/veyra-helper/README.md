# Veyra helper

P2-06 **DOING / 第十八轮工程部分交付**。`production` feature 提供 macOS `veyra-helper` executable 与 client/server library；原 `p0-05-prototype` bin、源码和 native harness 独立保留。Core 持有封闭 DTO；Cargo.lock 仅增加本地依赖与已锁定 tokio 的 helper 依赖边，不升级三方版本。

**仍不能进入 Native 安装验收。** 正式路径已支持本次OS认证会话内、有正在运行且可观察退出的Desktop旧child及last-successful资料的正常交接。`Capabilities.handoff` 与 `runtime` 分离；固定源记录/OS退出证明/Released/Local均成立后，才按会话放行Start/Apply，复用既有ManualRuntime/ProcessPort。本轮新增下述同OS会话的首次bootstrap独立Start；已经Stopped但未被本helper观察的旧child、跨进程重开与崩溃后未知owner/slot仍拒绝。只有 `#[cfg(test)]` 可注入源root及测试exe；正式协议没有路径、PID、UID或命令输入。代码可达与隔离测试不等于真实root部署通过。

- 协议 v2（新增handoff能力，旧版本先拒绝）：5-byte header，控制16KiB，配置1MiB + envelope16KiB，compiler外部资源仍为0；交接专用class3允许封闭plan≤1MiB、关闭cache≤2MiB、JSON bundle≤13MiB（请求另加配置1MiB/envelope16KiB），Preflight及Selection Confirm使用配置class2，其余命令仍为class1控制16KiB；未知字段/非法版本/大小先拒绝。Hello无需实例；UID/PID/start time 来自 getpeereid/LOCAL_PEERPID/proc_pidinfo。socket root:业务GID 0660；所有父目录root且不可被业务用户写入，无symlink，客户端校验前后inode与root peer。
- 每连接先Hello再单请求，连接/帧2秒；4连接worker + 4 queue，一个业务worker；Host操作30秒预算，Port单阶段最多10秒并受剩余预算限制。共享terminate发SIGTERM后最多等待3秒再kill/wait。disconnect不取消业务；128条结果，120秒TTL/LRU，UNKNOWN必须Status核验，无journal/跨重启exactly-once。
- Start/Apply为完整epoch/config/selection CAS；worker执行前再查自有child/实例，防止排队期间退出。P2-02B compiler通过ManualRuntime生成受控配置，匿名pipe FD3传递，token不进入命令行/日志。固定安装kernel的SHA必须等于Core锁定1.14.0资产，所有父目录/文件owner/mode/type检查。
- 正式spawn清空环境、setsid、清空supplementary groups、setgid/setuid并复核非root；无shell，无任意PID终止。check→run→解析该child日志中的实际loopback端口→认证Ready与固定版本→启动选择reconcile→manifest；Stop先终止/reap，再归还当前cache inode给root并由Core快照。执行器失败不发布Ready。空闲250ms检查真实child，退出后撤销applied并发布事件。
- cache文件仅固定私有root/kernel-cache中的当前inode；目录临时0711但不可业务写入，文件0600仅授予登记用户。Core状态/plan/secret保持私有；旧session/重启恢复材料拒绝自动消费。第三轮新增交接在源Stop/reap后比对关闭cache与live bytes，并在目标提交前同时冻结双方；不解析DB内容，不承诺FakeIP连续性。
- RuntimeCommand Observe为Core Refresh；Desktop RuntimeService将Select路由到封闭Selection事务：本地fence+pending→helper实际PUT/GET→Desktop全版本CAS→helper manifest/已确认输入投影→解除fence。仅支持有明确旧confirmed NodeId的真实Manual pool，未应用配置版本与运行配置不同先拒绝。正常会话必须通过独立来源验证；未知恢复仍拒绝。事件ring256/单批16；SystemProxy请求Unsupported，P2-07没有实现。
- Desktop实际RuntimeService单worker连接固定Client；Start/Apply要求本地Stopped/Released、helper同票据Local、Status及业务完整版本匹配。手动Start/Select仍被owner fence阻止；缺失交接不启动第二writer。
- `install`/`uninstall` 是固定管理员CLI代码，开发过程未执行。安装仅从 `/Applications/Veyra.app/Contents/Resources/helper` 的root保护资源读取，核对deny-unknown manifest和SHA；登记/dev/console普通用户UID与passwd GID，写固定root资产/authorization/plist，固定launchctl bootstrap。资源由root保护是前置条件，manifest自签不是信任来源。**打包资源准备及普通用户授权UI尚未接通**。
- 卸载新增固定root保护的 `uninstall-request` / `uninstall-ack`，不把root伪装成业务Peer。串行Runtime worker停止/reap自有child，再复用ClosedBundle校验关闭cache与manifest后ACK；未确认选择只Stop且不ACK/不清fence，封存失败保留现场。服务持有 `lifecycle-lock`，安装器持有独立 `uninstall-lock`；收到ACK才bootout，取得服务退出锁后才进入下述完整Archive事务。离线只允许完全没有runtime历史；锁释放不作为旧writer死亡证据。所有命令和新服务都拒绝drain marker，超时不自动解冻。第六轮以root外 `deployment-lock` 串行安装/卸载；ACK与bootout成功记录齐备、取得退出锁后，把完整旧root原子迁入固定 `Veyra/RecoveryArchive/installation-<dev>-<ino>`，保留资产、plist、cache/manifest/fence/owner/marker，释放安装ROOT。archive父目录及条目0700、同文件系统、RENAME_EXCL不覆盖、fsync双方父目录。新安装只验证归档证据，不消费旧业务资料；未知Archive、活动ROOT或plist仍拒绝。plist已迁移的半卸载可以重试；bootout已成功但记录尚未落盘的崩溃窗口仍需人工核验。正式root安装/卸载未执行。
- 新 `installation_manifest(helper, kernel)` 是无权限纯摘要生成API，生成前强制核对Core锁定kernel SHA；不执行字节或接受安装路径。尚未接入真实打包/授权UI，不能把自签manifest视为信任。
- 初始root必须完全无历史条目；缺失manifest但存在cache也拒绝初始化/预检。重建backend时即报告recovery_required，Host不再先发布空默认快照。该检查只收紧拒绝，不授权cold-start。

正常交接链路为 `RuntimeService::transfer_owner` → Core `helper_transfer::transfer` → Unix `Handoff::{Preflight,Query,Export,Prepare,Release,Commit}` → InstalledBackend → ManualRuntime。正向和反向使用同一合同；没有自动启动，接受方仍需独立Restore/check/Ready。第四轮在任何Freezing/Stop前检查业务全版本/pending、Configuration合法性/1MiB上限、固定endpoint的OS身份/协议/宿主与kernel能力、目标是否可接入；未知/未安装/不支持均保留旧child。预检通过后，源持久Freezing再停止自有Child，再封存Closed；目标Prepared冻结、源Released持久落盘、目标提交manifest后Local。传输ID+全版本+SHA组成票据，同票据可重试。半完成或版本变化不撤销冻结，不回滚为另一个可启动owner。

每个root只有一份owner记录/一份有界bundle与生命周期绑定，不是请求journal。Runtime nonce不匹配时阻止新进程把空Port当作旧child退出证明；helper的OS peer绑定由内核身份写入固定0600文件，重启后的Query只读且recovery_required，不重新绑定未知session。**跨崩溃未知owner仍HandoffRequired/RecoveryRequired；本轮仅授权经过完整证明的正常会话**。

新requestId的重复Start会进入串行worker同步核验当前child，返回Inflight后由Operation给出实际结果；不会直接以250ms旧快照报告已运行。同requestId仍返回历史操作结果，需Status了解当前状态。

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo build -p veyra-helper-prototype --features production --bin veyra-helper --offline
MACOSX_DEPLOYMENT_TARGET=15.0 cargo test -p veyra-helper-prototype --features production --lib --offline
```

第四轮回归：Core373（1个内部子进程fixture ignored，由父测试调用）、Desktop88（11 Native ignored）、helper28（1个内部kernel fixture ignored，由OS测试调用）、prototype47、Python2。新增OS测试覆盖预检失败保留活child、真实Unix断线后选择查询、同池GET屏障期间拒绝新pending/换epoch、桌面CAS后manifest失败重试，以及actual old/third/unknown语义。真实root降权、正式kernel、管理员安装/取消、Native controller/UI全部NOT_RUN。详见[第四轮交付](../../docs/openbox-rust-gpui-tasks/P2-local-proxy.md#p2-06-round4)。

`JsonStateStore` 普通save/commit/整体replace与远程事务共用OS writer文件锁，另保留一份16KiB上限的未完成选择fence（requestId/instance/full version/PoolId/NodeId/previous）。普通业务写入口，包括独立gate的SubscriptionManager/ProviderReplacement，不能跨越fence；损坏state也不能通过旧backup恢复来清除它。pending和确认仍只由Desktop持久化，helper仅更新受管controller、manifest和桌面明确回告的输入投影。短RPC超时不解除fence；实际GET=old才clear且返回选择失败，third/unknown保留；最终回告前不报告成功。Host选择结果复用128条/120秒缓存，Operation过期UNKNOWN；一个未解决的活动选择slot只用于当前实例核验，不是永久请求日志。若请求未到达helper、helper重启丢失slot或OS会话变化，当前仅安全阻塞，不能自动重新Execute/解除冻结。


## P0-05 prototype（原有独立入口）

The isolated `p0-05-prototype` entry remains named `veyra-helper-prototype`.
It is not a fallback for the production helper.

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

The traditional LaunchDaemon uses label `me.disign.veyra.p005` and fixed plist
`/Library/LaunchDaemons/me.disign.veyra.p005.plist`. Its `ProgramArguments` are
`/Library/Application Support/VeyraP005/helper` and `daemon`. Helper, fixed `veyra-sing-box`,
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
`veyra-sing-box` may execute. HTTP/HTTPS/SOCKS/PAC/discovery/bypass restoration compares
related field groups; native writes recheck the observed dictionary under the
SCPreferences lock. Root service writes require the recorded service to remain
disabled, P005-named and absent from all Network Sets.

If restore fails, the daemon retains the endpoint and recovery record, and the
uninstaller refuses removal. A prior recovery record prevents a new writer.
This P0 entry deliberately does not implement production restart recovery,
multi-service support, TUN, updates or GUI. Never treat successful compilation
or ordinary-user API tests as proof of privileged platform acceptance.

Current outcome and exact limitations: [P0-05 evidence](../../docs/openbox-rust-gpui-tasks/evidence/p0-05/README.md).

第五轮新增6个非特权OS/文件边界测试：管理员drain真实Stop/reap/封存、未确认选择保留fence、封存失败不ACK、服务进程退出但独立writer仍持FD、恶意marker、缺失manifest的旧cache。没有真实root/launchctl/正式kernel；新增内部crash fixture由父测试实际启动，不能当Native验收。详见[第五轮](../../docs/openbox-rust-gpui-tasks/P2-local-proxy.md#p2-06-round5)。

## P2-06 第六轮：打包输入与恢复门槛

构建输入仍为 `cargo build -p veyra-helper-prototype --features production --bin veyra-helper --offline`（macOS deployment target 15.0）。发布打包方把该binary、SHA与Core锁定1.14.0一致的darwin-arm64 `sing-box`、`installation_manifest(helper_bytes, kernel_bytes)` 的结果放入固定 `/Applications/Veyra.app/Contents/Resources/helper/{veyra-helper,veyra-sing-box,install-manifest.json}`。两个binary要求0755、manifest0644；整个安装输入路径必须root拥有且不可被普通用户写入，installer再次校验，生成manifest本身不授予信任。`installation_plist()` 与安装器共用固定label/程序/serve参数；安装器只写固定 `/Library/LaunchDaemons/me.disign.veyra.helper.plist`，不接受客户端plist。

尚无实际bundle打包/授权UI接线；上述是明确的源码API与输入合同，不是已验收的分发产物。没有执行任何管理员命令。Archive不自动恢复：未知旧owner/丢失selection slot保留pending/fence和恢复状态，禁止删除marker/cache以解冻。普通已成功handoff的正式启用仍缺helper对源固定释放证据/旧writer关闭的独立核验；client自报Released不能替代该证明。因此helper与Desktop的正式gate仍保留，不能把测试fixture可运行当成生产可用。

第六轮增加5项Archive OS隔离测试：两轮模拟卸载/重装、权限/symlink和重复条目、真实RENAME_EXCL失败与半提交重试、失败bootout/缺失ACK/实际锁占用、SIGKILL自有服务fixture后仍不能绕过ACK。bootout与管理员授权为Mock/NOT_RUN；rename/fsync/锁/测试进程为真实非特权OS。详见[第六轮](../../docs/openbox-rust-gpui-tasks/P2-local-proxy.md#p2-06-round6)。

## P2-06 第七轮：有限正常生产交接

`production/source.rs` 从OS认证UID的 `getpwuid_r` home推导固定Desktop preview root，不读取env或IPC路径。逐级目录与固定文件检查owner/mode/type/no-symlink，使用openat/O_NOFOLLOW读取有界资料。预检绑定真实Peer UID/PID/start、source-session incarnation、完整版本；枚举该Peer直接child，固定kernel SHA匹配且唯一时注册kqueue NOTE_EXIT。Prepare/Commit独立读取Closed/Released、incarnation、ticket/bundle/manifest/plan/cache摘要；旧进程必须实际退出且reap。每次启动/Apply/选择再次核验同Peer、helper Local和源Released。凭据改变时拒绝副作用并在Status报告RecoveryRequired；自有Stop仍允许。

**信任边界**：OS证明是本helper亲自观察的直接child退出，不是业务UID文件的不可伪造签名，也不证明全系统无隐藏/orphan writer。kernel校验是OS报告路径的磁盘SHA，不是已加载代码签名证明。同UID恶意进程能改写用户文件；当前不声称抵抗它伪造完整一致历史。helper重启丢失退出观察、Desktop重启换Peer、源历史不一致均拒绝自动接管。该有限边界需Host确认；无跨重启exactly-once承诺。Desktop root override与默认路径不一致时生产拒绝，不能用开发配置绕过。

新增2个非特权OS父测试：真实自有kernel fixture、Unix socket、退出/reap及真实controller Ready/Apply/Stop；覆盖伪造source PID/仍活writer/缺失incarnation/未Released/版本与Peer错误/bundle破坏/权限错误，正常Commit及重复Start，已接受后源凭据变化拒绝Apply且RecoveryRequired。负面Prepare/Commit直接调用真实Backend；启动链路经真实IPC，未冒充全部负面都经过socket。完整回归Core373/Desktop88/helper41/prototype47/Python2。原Archive5项保留。证据与OPEN见[第七轮](../../docs/openbox-rust-gpui-tasks/P2-local-proxy.md#p2-06-round7)。

## P2-06 第八轮：产品身份与退出清理

当前产品标识统一为 `me.disign.veyra`；Desktop namespace与helper固定来源同时使用 `me.disign.veyra.gpui-preview`，Tauri identifier一致。production label/plist使用 `.helper`，P0-05 label/plist及probe使用 `.p005`。原型只改这些身份字符串，行为/native不变。其它独立GPUI实验项目的不同bundle ID不属于本次精确替换。

新Desktop数据根为 `~/Library/Application Support/me.disign.veyra.gpui-preview`。**不迁移、不覆盖、不删除旧目录，也不fallback旧身份**；新启动不会自动显示旧目录的数据。专门迁移需用户另行批准。旧daemon/plist、已有App及实际签名/授权均未操作；固定Helper/P005资产root没有随label改名，若存在旧安装材料仍拒绝覆盖，不能把标识替换当成真实升级/卸载。历史evidence中的旧标识保持原样。

RuntimeService Quit/worker断连现在按持久owner清理：Released通过固定Client核对helper同票据Local后，Status→Stop(当前instance)→Operation→Status；Stop响应丢失只查询，不重复提交。40秒operation轮询预算加每次transport固定阶段超时，不承诺整个Quit仅40秒。UNKNOWN、跨session、半交接、pending或清理失败返回StopFailed/Recovering并保留票据/fence；不冒充Stopped。正常无owner/本地Local仍用ManualRuntime Stop；窗口隐藏不触发Quit。成功退出不为发布快照额外读取业务state，保留“取消订阅预览后不初始化状态”的原契约。P2-07系统网络恢复仍Unsupported，未实现。

源Witness增加kqueue注册后同PID/start/UID/parent复核，防止注册与采样间的身份变化；Closed/Released/manifest/cache独立读取与未知来源拒绝未放宽。测试与限制见[第八轮](../../docs/openbox-rust-gpui-tasks/P2-local-proxy.md#p2-06-round8)。

第九轮仅按用户纠正基础身份为 `me.disign.veyra`，上文当前说明同步更新。正确数据目录为 `~/Library/Application Support/me.disign.veyra.gpui-preview`；原始namespace及第八轮误写namespace下的两个历史目录均保持原样，不读取、迁移、覆盖、删除或fallback。App/Bundle/LaunchDaemon身份变化不等于完成已有安装升级；历史数据迁移与旧daemon处置需后续用户批准。本轮不改变任何Runtime/恢复功能。历史证据保留当时真实标识，见 [round9](../../docs/openbox-rust-gpui-tasks/evidence/p2-06/round9/README.md)。

## P2-06 round10：只读恢复查询

Handoff Query不进入管理员Stop/封存barrier；Preflight只读检查drain状态并拒绝准入。独立管理员poll仍可处理已授权的卸载请求，因此不承诺后台不会同时清理。重启后Query只能展示绑定同OS会话的旧owner记录，不能因此恢复child或解除pending/fence。权限错误/跨session继续拒绝，无新公开API。

冷启动仍不支持：pristine_root只是目录材料检查，真实持有已unlink FD的writer可与空目录并存；安装资产/授权UID也不能证明没有旧Desktop writer。仅已有完整last-successful、可观察退出/reap并提交Released的正常交接受控启用。首次冷启动范围待Host明确决策，不能将同UID文件摘要作为root可信首次启动证明。本轮3项OS/权限fixture及全量回归见 [round10](../../docs/openbox-rust-gpui-tasks/evidence/p2-06/round10/README.md)，Native NOT_RUN。

## P2-06 round11：真实内核basename

受控可执行文件名统一为 `veyra-sing-box`，上游sing-box协议/版本1.14.0/原始SHA不改。production bundle输入为 `Contents/Resources/helper/veyra-sing-box`，安装目标为 `/Library/Application Support/Veyra/Helper/veyra-sing-box`；安装/卸载均验证新固定文件，Archive整root原子封存语义不变。source Witness拒绝basename仍为旧名称的来源，不能先Stop旧实例才发现不兼容。P0-05 staged与root资产同步使用新名字，probe只将上游归档成员保存为新名；旧Native审批证据只证明旧baseline，新名字Native NOT_RUN。

Desktop开发env `VEYRA_SING_BOX_PATH`必须指向真实basename为 `veyra-sing-box` 且SHA一致的文件；原名或同名symlink指向原名都拒绝（明确输出basename不匹配，Runtime仍KernelUnavailable）。不自动拷贝/迁移用户缓存。未设置开发override时，仅从实际`Contents/MacOS`入口对应`Resources/helper/veyra-sing-box`读取并验证；release忽略开发env。P0-08尚未打包这些资产，不宣称当前发行包可用。旧安装root/两处旧用户目录不修改、不删除，新旧basename不fallback；旧安装升级/重新授权须后续Desktop安排。

P0-04/P0-07诊断工具也仅在本次新建临时root内重命名新解压文件后执行，不修改上游archive或历史cache；本轮只作Python解析/合同测试，未运行其联网/真实kernel入口。Legacy Windows仍有`sing-box.exe`受管资源与Tauri资源映射，Windows fixture当前缺失且本轮不能验证；明确留W0-01/W1-01/P0-08后续平台打包适配，不伪称全平台完成。详见round11证据的路径审计。


### P2-06 round12：安装世代与Start重传

固定安装器新增root-owned 0600 `installation.json`（随机安装世代、协议、授权UID/GID、安装root dev/inode），在发布plist前持久创建；不接收客户端字段、不覆盖已有记录。正式Assets加载/check/run检查同一安装身份，Archive原样保留。旧/半安装缺记录不自动补写或读取旧用户目录，报告HandoffRequired/RecoveryRequired；需要后续受控安装/修复流程。记录不是代码签名，也不是Desktop退出证明，首次启动gate尚未解除。

同requestId Start的旧成功结果不能用作当前Ready：重传进入worker同步poll，返回Inflight后查询Operation；退出/Stop/版本已改变时拒绝，不重启child，原绝对TTL不延长。单独Operation仍是留存操作查询，UI须结合Status；不承诺跨helper重启exactly-once。首次Desktop无历史让权与正常关闭后新OS会话准入仍待工程闭合，见round12证据；不是已支持普通首次使用。


### P2-06 round13：writer lease

正式helper在固定安装root取得0600 `runtime-writer-lock`，先于业务输入/owner落盘；check/run继承同一OS租约至FD4，配置pipe仍FD3，其余非必要FD在exec关闭。父端关闭/死亡后，仍继承FD的child继续阻止新owner取锁；原有持久owner/恢复拒绝仍必须检查。Preflight只读检查锁冲突，不创建文件；Archive额外检查该锁，不以服务lifecycle锁释放替代writer退出。正常Stop不单独释放宿主所有权，lease随Port退出释放；锁不是clean-stop凭据。

隔离fixture验证了FD继承与服务持有者SIGKILL、writer退出前不能取锁；实际固定veyra-sing-box的FD保留和root降权仍Native NOT_RUN。首次Desktop合作冻结/Commit和跨进程clean-stop尚未接通，不能称新安装独立Start已支持。没有新增IPC路径/PID/命令入口，P2-07系统设置仍Unsupported。


## P2-06 round15：首次Preflight/Prepared（不授权Start）

正式新增封闭 `BootstrapAction::{Preflight,Prepare,Query}`。Preflight经固定OS peer、安装世代/资产、完整业务版本、空历史和lease/管理员撤销检查返回Eligible票据；只读，不创建目录/锁/owner/cache。Desktop `RuntimeService.prepare_bootstrap`在同worker先做目标预检，再持久BootstrapFrozen并发布当前SourceSession，随后请求Prepare；未安装/预检失败不冻结Desktop。

Prepare从业务UID固定home路径独立读取BootstrapFrozen/incarnation/SourceSession并核对UID/PID/启动时间/完整票据，取得root writer lease后持久0600 `bootstrap-prepared.json`（peer/ticket/incarnation/config digest，不存配置秘密），同OS会话可幂等重试与Query。记录是单份owner状态，不是永久请求journal。半写入保留slot/租约，不擦除；重启丢slot后同peer只能Query RecoveryRequired。旧handoff与bootstrap互斥，Prepared期间Start拒绝。

**首次独立Start、正常跨进程重开仍未实现**：没有Commit action。SourceSession/冻结文件属于同UID可写材料，OS peer证明连接进程身份，不证明其加载代码或持有Desktop `PrimaryInstance._lock`；这些文件不是旧writer退出或双端排他的独立OS证据。Commit需接入已有Desktop单实例/writer约束及完整版本复核、受限资源投影，再单独接Start；不能只删gate。产品首次使用目标已确认，无需再问用户。真实root权限/内核/签名/GUI Native均NOT_RUN。

本轮Core377/Desktop92/helper60/P0原型47/Python4及工程检查通过；实际命令与实现者自查见[round15](../../docs/openbox-rust-gpui-tasks/evidence/p2-06/round15/README.md)。


## P2-06 round16：首次Committed与同会话独立Start

生产新增固定`locks/desktop.lock` + `runtime/desktop.sock`的OS合作Primary核验。Prepare/Commit重新核验flock占有、socket对端UID/PID/start、文件权限/类型与inode；OWNER_PROBE/ACK只读，不激活窗口。SourceSession/Frozen绑定同一incarnation与完整票据。安装世代不是token，只有固定root安装资料和当前已验证peer可进入准备阶段；同UID恶意代码/代码签名不在此证明范围。

root在Prepared持有的唯一writer lease下持久`bootstrap-committed.json`（0600/create_new/fsync），随后才报告Committed。Core/RuntimeService显式`prepare_bootstrap`→`commit_bootstrap`→`helper_request(Start)`→Operation/Status；没有自动GUI入口。Start严格匹配冻结版本和配置摘要，将同一lease open-file-description副本交给既有ProcessPort，真实compiler/check/run/鉴权Ready/manifest流程不变。首次无last-successful仅此分支允许；同会话干净Stop/reap可同版本再次Start。原手动handoff路径保持独立。

半写/丢slot/异常退出继续RecoveryRequired或HandoffRequired，不消费未知旧cache或清fence。退出允许同peer在primary listener关闭后只读查询Committed并停止自有实例；新Start仍需重新核验primary。bootstrap分支Apply/远程选择/跨进程clean-stop重开/反向移交尚未完成，保持拒绝，不宣称整个产品首次安装闭环已验收。OS隔离测试使用自己的测试exe副本`veyra-sing-box`，不是实际安装kernel；root降权、签名、管理员安装/授权拒绝、GUI Native均NOT_RUN。完整初始FAIL和最终验证见[round16](../../docs/openbox-rust-gpui-tasks/evidence/p2-06/round16/README.md)。


## P2-06 round17：CleanStop事实与异常Primary退出

bootstrap正常Stop后复用Core的关闭材料校验，持久一份root-only `bootstrap-lifecycle.json`，包含既有安装/peer/incarnation/config绑定、递增cycle及ClosedBundle摘要。每次新Start先持久closed=None以失效旧清洁凭据，再spawn；成功Stop/reap、完整版本和无pending/fence、owner-session及plan/cache/manifest一致才能标CleanStop。原子写入使用固定create_new pending文件、fsync、rename及父目录fsync；残留/失败保留现场，不能自动再启动。

Prepare捕获当前OS peer的NOTE_EXIT；poll只在明确事件后Stop/reap自己的Child，不用任意PID、不以断线或隐藏窗口当退出。异常owner退出保持RecoveryRequired和原Running记录，不冒充可重新接管的CleanStop。此修复通过自有Primary子进程+kernel fixture验证，非真实GUI/Native。

跨Session rebind仍未实现：本轮没有清旧Frozen、重写旧session或将持久CleanStop当授权token。新Primary还需要有限Prepare/Commit、关闭资料重验和新incarnation绑定。原同Session首次Start和重启继续支持；跨helper重启/未知orphan/lost slot继续拒绝。详见[round17](../../docs/openbox-rust-gpui-tasks/evidence/p2-06/round17/README.md)。


## P2-06 round18：CleanStop后的新Primary准备

新增RebindPreflight/Prepare/Query，Desktop同worker与Core先验证业务完整版本/无pending/fence，再保持旧Frozen/incarnation/source-session并写独立新session申请。root独立重算last-applied/plan/cache/live-cache的ClosedBundle摘要，不把CleanStop.closed当token；必须持原Runtime真实Stop/reap事实、安装generation/owner-session与lease、旧Peer NOTE_EXIT及新Primary OS lock/socket身份。

root只持久bootstrap-rebind-prepared.json并允许同ticket/peer查询；Prepared不会替换旧运行owner，任何新Start仍HandoffRequired。Rebind Commit、新owner确认与跨Session重新启动尚未完成；helper重启不从空槽或文件存在自动恢复。仅合作参与者边界，不声称恶意同UID代码签名认证。测试是自有Primary/内核fixture/Unix socket，Native未运行。[round18证据](../../docs/openbox-rust-gpui-tasks/evidence/p2-06/round18/README.md)。


### P2-06 round19：受限跨Primary Commit

`RebindCommit`在同helper生命周期内、真实CleanStop与旧NOTE_EXIT、新OS Primary及完整Prepared证据成立时，持久写入独立rebind提交/current-owner，并保留原owner历史。写入全成功才切内存会话，既有ProcessPort/FD4 lease连续持有；新Start仍走正式Compiler/check/run/Controller Ready。正常Quit可先关闭Primary listener再查询/Stop，窗口隐藏不触发此路径。

非特权双Primary/IPC/测试内核证据已覆盖一轮旧→新Primary Ready/Stop及超时Query、重复/错误/半提交。它不是无限多次重开或helper重启恢复：第三Desktop会话轮换、丢slot、未知orphan仍拒绝，bootstrap Apply/Select与GUI消费未闭合。真实root/授权/真实内核/GUI NOT_RUN，P2-06保持DOING。见任务evidence/p2-06/round19；旧记录不改写。
