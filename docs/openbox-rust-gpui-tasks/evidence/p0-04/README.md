# OBG-P0-04 固定内核、控制器与缓存原型

2026-10-04，Codex /root，Runtime/Platform 泳道。起始 HEAD `19696abde8a27a36c4eb490a4236006fcd09c9cc`，工作树干净；仅领取 P0-04。四项验收 PASS，状态 DONE。本目录为真实 macOS child/loopback/controller/cache 证据，非 UI/协议认证结果。

## 身份与代表配置

官方 [v1.14.0 Release](https://github.com/SagerNet/sing-box/releases/tag/v1.14.0) 的 `sing-box-1.14.0-darwin-arm64.tar.gz` 下载仅位于隔离临时目录；Host 提供的发布时间为 `2026-08-31T04:05:01Z`。archive SHA-256 在执行前匹配官方 digest：

```text
a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9
```

binary SHA-256：`973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`；`file` 为 Mach-O 64-bit executable arm64。版本 1.14.0、Go 1.26.7、darwin/arm64、CGO enabled、Revision `0b8995879f29a9b98ee027bc17b75e101445b238`。完整 version 输出及全部 build tags 见 [kernel-identity.json](kernel-identity.json)。本机 macOS 27.0 / arm64；不外推其他 OS/设备。

[checks.json](checks.json) 记录五份实际配置的 SHA-256、命令、20 秒超时和 exit 0：

| Fixture | 实际 check | 用途 |
| --- | --- | --- |
| [loopback-smoke](fixtures/loopback-smoke.json) | PASS | 同份实际配置 check → run → 鉴权 Ready → SIGTERM；随后原文件重启 |
| [dns-fakeip](fixtures/dns-fakeip.json) | PASS | 新格式 typed DNS server / FakeIP / hosts fallback / cache_file |
| [tun-check-only](fixtures/tun-check-only.json) | PASS | 当前 TUN address/stack 字段接受性；从未 run，不修改系统网络 |
| [handoff-smoke](fixtures/handoff-smoke.json) | PASS | 仅更换关闭后缓存的受管 path；稳定 cache_id/tag/pool |
| [cold-control](fixtures/cold-control.json) | PASS | 空缓存对照，排除默认分配碰巧相同 |

fixture 是脱敏模板，`<tmp>` 和 `<random-temporary-secret>` 需由脚本实例化，不能直接当运行配置。实际 secret 随机生成，实际文件 0600、临时根 0700；它们已删除，只保留 hash。无旧 DNS server/address 格式、旧 `dns.fakeip` 或 `experimental.clash_api.store_selected`。普通 DNS response cache 显式关闭，`store_dns=false`，恢复证据来自 FakeIP 映射。

前期字段调查的两次 check FAIL 保留：FakeIP 不得作为默认 DNS server；direct dial 需要显式 `route.default_domain_resolver`。最终使用 typed hosts fallback，并将 `.invalid` A 查询路由到 typed FakeIP。前期三个试探配置（包含随后成功的试探）未捕获配置 hash，原文件已删除，不伪造其 hash；正式可复跑五份配置 hash 均已记录。最终配置不使用 deprecated 环境开关。没有发现要求本卡修改现有 Compiler 的阻断缺陷；Compiler/runtime/clash_api Rust 源码保持原样。

## 动态 controller 与生命周期

[controller-dynamic-port.json](controller-dynamic-port.json) 记录四个真实受管 child 的 PID、配置摘要、监听身份、鉴权响应及关闭结果。配置 `external_controller=127.0.0.1:0`，原始启动输出由本实例 stdout/stderr 合并管道捕获，10 秒预算内按下面正则解析：

```text
clash-api: restful api listening at (127\.0\.0\.1:([1-9][0-9]{0,4}))$
```

实际日志为 `INFO[0000] clash-api: restful api listening at 127.0.0.1:<port>`；本次四轮地址分别为 `127.0.0.1:64849`、`127.0.0.1:64865`、`127.0.0.1:64875`、`127.0.0.1:64885`。解析后才使用 `lsof -a -p <owned-pid>` 独立确认该 child 持有 controller/mixed listener 和 cache writer，未扫描本机端口或其他进程发现地址。

本实例临时 secret 鉴权 GET `/version` 为 HTTP 200，shape 为 `{"meta":true,"premium":true,"version":"sing-box 1.14.0"}`；无鉴权同 endpoint 返回 401。单请求超时 3 秒。四 child 均以 SIGTERM 正常退出（exit 0），wait 完成、process group 不存在；已知 controller/mixed 端口拒绝连接并可重新 bind，cache 文件无打开 writer。没有触发 fallback 或实现通用端口管理器。

对应脱敏启动日志：[initial](initial-child.log)、[restart](restart-same-config-child.log)、[handoff](closed-writer-handoff-child.log)、[fresh-cache control](fresh-cache-control-child.log)。原始启动行顺序保留；内核自动输出的主机 gateway 整行脱敏为 `<host-network-redacted>`。四轮均出现 `network: listen network update: operation not permitted`，是本沙箱的网络更新订阅限制；实际 loopback/controller/cache 操作成功，不据此宣称网络变化监听或完整 Runtime 已验证。

## selector、FakeIP 与关闭 writer 的 handoff

[cache-recovery.json](cache-recovery.json) 包含完整请求结果和时间：

1. selector 稳定 tag `p004-selector`，两候选为内置 direct（`p004-direct-a` / `p004-direct-b`），无需公网。初始 GET 为 a；鉴权 PUT 为 b（204），GET/readback 确认为 b。
2. 先查询 `p004-sentinel.example.invalid` 得 `198.18.0.2`，再查询目标 `p004.example.invalid` 得 `198.18.0.3`；同 child 重复目标查询仍为 `.3`。
3. 正常停止，随后用**同一实际配置文件/摘要、cache path/cache_id**重启；首次读 selector 为 b，重启后的第一个 DNS 查询直接查询目标，仍为 `.3`。
4. 第二个 child 完全退出、process group 清空且 writer 无打开句柄后记录缓存 size/hash/mtime。缓存 131072 字节；复制关闭缓存到另一个受管目录，复制前后 SHA-256 均为 `a0749679a4b28dd2fbe44bb1d70ce72b3d6170fa9457cefec1c37b640dd5d7f8`，mtime 保留。
5. 新目录 child Ready 后 selector 为 b、目标 FakeIP 为 `.3`，handoff PASS。没有打开或修改缓存内部格式，没有 SQLite 或业务 migration 操作。
6. 独立空缓存对照：selector 回到默认 a；目标作为第一次查询得到 `.2`，与恢复 `.3` 不同。这排除了“同一个域名重新分配碰巧一样”的假阳性。

FakeIP 使用固定 1.14.0 的鉴权 loopback `/dns/query?name=p004.example.invalid&type=A`，响应 `Server=internal` / `Status=0`。路径调查依据固定版本的 [DNS controller 源码](https://github.com/SagerNet/sing-box/blob/v1.14.0/experimental/clashapi/dns.go)，最终依据实际 binary 响应。DNS server 仅 hosts/FakeIP，不使用系统或外部上游。route 仅允许 loopback，其他目标 reject；受控流量只到脚本自己创建的 loopback HTTP origin。

## 对账前流量窗口与兼容性

**存在对账前流量窗口**。同配置 restart 的 `/version` Ready 至 selector/FakeIP 对账完成为 **13.433084 ms**；handoff 为 **13.750209 ms**。每轮记录 UTC 与 monotonic 时间；Ready 后、读取 selector/FakeIP 之前，都通过实际 mixed inbound 完成了自有 loopback origin 的 HTTP 200 请求，child 日志显示出站走恢复后的 direct-b。计时包含身份/cache 句柄核对和受控请求，是本原型的观测区间，不是 Runtime 性能保证或原子切换保证。inbound 实际监听发生在 controller Ready 之前。

后续 P2：selector/cache 对账完成前不把 Runtime 报成业务 Ready，不打开新的系统代理接管；是否进一步 gate inbound 由后续 Runtime 实现决定。没有实现正式 macOS SidecarPort/owner。

结论仅限相同 binary/version、cache_id/generation、稳定 tag 和同一候选池/FakeIP IPv4 地址池。跨版本、tag 删除/改名、pool/FakeIP range 改变、跨 UID/helper owner、会话无缝连续性均未验证。TUN 只有 check，无运行或系统路由/DNS 证据。结果不形成 TCP/UDP/WireGuard/IPv6 或任何公网协议认证矩阵。

## 复跑与验证

保护的 Veyra 契约为：固定内核身份/配置加载、实例地址归属与鉴权、正常停止/资源释放、选择及 FakeIP 持久恢复、单 writer handoff、对账前 Ready 边界。仅这些真实原型断言，无新增产品单测。

macOS arm64、Python 3.12+，使用系统 `curl` / `file` / `lsof`；执行会下载固定官方 archive、启动隔离 loopback child，不启动 TUN、不修改系统设置。使用新的输出目录，避免与已有证据混合：

```sh
python3 tools/p0-04-kernel-probe.py --output /private/tmp/veyra-p004-replay-evidence
```

脚本下载 120 秒上限、check 20 秒、发现 10 秒、请求 3 秒、停止 10 秒；本轮外层为 Python `subprocess.run(..., timeout=240)`；脱敏修复重跑未指定 `--archive`，重新下载官方 archive，匹配 digest 后才执行。输出目录只含脱敏文本，内核、实际配置及缓存所在内部临时目录在 finally 清理。正常退出不能由强制 kill 冒充 PASS。保留的复跑输出可审阅后自行删除。

[probe-result.json](probe-result.json) 为真实原型 PASS 和四 child 清理；前期试探 child 也 wait/exit 0，其 cache 无打开 writer，试探 archive/binary/cache/config/指针均已清理。[validation.json](validation.json) 记录 JSON/脱敏扫描、68 Task DAG、本地 links/anchors、脚本语法及身份、`cargo check -p veyra-core`、`git diff --check` 和最终清理核验；[core-check.log](core-check.log) 为本轮实际 core 编译证据。

按 [code-delivery-review](../../../../.agents/skills/code-delivery-review/SKILL.md) 作实现者自查：检查同配置身份、controller 来源、鉴权、FakeIP 对照、关闭后复制和清理证据，未发现尚未处理的本卡行为缺陷；不是独立审查。未修改产品 Rust/前端，未运行无关 frontend/clippy/协议测试。没有 commit/push，P0-05/P0-07/P1-03 只进入 READY，不启动。

## Host Review 脱敏修复重跑（2026-10-04）

Host 独立 Review 发现此前 lsof 输出保留了本机用户名，原脱敏扫描未覆盖这一身份字段；技术结论不受影响。`clean()` 现在通过 `getpass.getuser()` 读取当前用户名，以大小写敏感的完整 token 匹配替换为 `<user>`，不硬编码本机身份，保留 `USER` 列标题。

完整 probe 已重新下载官方 1.14.0 archive、先匹配 digest，再执行五份 check/四 child，覆盖并重新生成正式 JSON、fixtures 和 child 日志。动态 controller、鉴权 200/无鉴权 401、selector/FakeIP（sentinel/target/fresh-cache 对照）、closed-writer handoff、对账前受控流量和 SIGTERM 清理均 PASS。此前窗口 22.990292/14.466458 ms 属于首轮历史结果，上文为本次实测；前期 check FAIL、未捕获试探 hash、环境/兼容性限制均保留。

本轮扫描整个证据目录与原型脚本：无实际本机用户名、真实用户/临时路径或输出的鉴权 header/secret；源码保留请求所需的鉴权协议标识及通用脱敏匹配模式。JSON/AST、core check、68 DAG、links/anchors、diff/新增文件 whitespace 与摘要均重新验证，validation 记录本次脚本/证据 hash。所有新 child/group/listener 和临时 archive/binary/config/cache 已清理。产品 Rust/Cargo、任务文档/状态/DAG 均未修改，P0-04 保持 DONE，无 commit/push。
