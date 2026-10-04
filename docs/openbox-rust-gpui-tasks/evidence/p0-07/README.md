# P0-07 本地 Observation 能力核实

2026-10-04，owner Codex /root / Observation，**DONE，四项验收 PASS**。起始 HEAD `fac6f12`、工作树干净。交付仅原型脚本、脱敏证据、任务文档与方案能力结论；产品 Rust/前端/Cargo 未改，无 commit/push。独占自己的 child/controller/origin/UDP DNS fixture，不与 P0-05 helper 共用实例。

## 身份与范围

[内核身份](kernel-identity.json)：官方 sing-box v1.14.0 darwin-arm64 archive 在执行前核对 SHA256 `a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`；binary SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，Revision `0b8995879f29a9b98ee027bc17b75e101445b238`。官方 tag 通过 `git ls-remote` 解析到同 commit，按 commit 下载源码 archive；13 个相关源码文件记录 hash、path、行区间和简述，完整 controller route registration inventory 见 [source evidence](kernel-source-evidence.json)，没有复制大段源码。

只启动 loopback mixed/controller，自有 HTTP origin 与一个精确 A/IN question 的 UDP DNS fixture。诊断 host `p007.example.invalid` 通过内核 hosts 映射为 `127.0.0.1`，DNS 缓存 fixture 使用 TEST-NET 答案且不连接该地址；路由允许 loopback，其他目的地 reject。没有访问 UI 参考环境、没有公网协议测试、系统代理/TUN/系统 DNS/管理员操作。官方 archive/source 下载是身份核对，不是公网连通测试。

[配置 check](config-check.json) PASS；[配置模板](config-template.json) 移除鉴权凭据，端口是该轮动态端口，复跑由脚本重新分配。[controller](controller.json) 从本 child stdout 获取端口，再做 PID-scoped lsof 身份确认；`/version` 鉴权 200/未鉴权 401 只作 readiness。目标诊断另有真实 HTTP 200，未用 readiness 冒充访问成功。

## 四类真实 WebSocket

[stream-frames.json](stream-frames.json) 含每帧 UTC、monotonic_ns、sequence、脱敏 JSON 与握手/开始/关闭时刻；标准 controller，四类握手均 101，没有 OpenBox wrapper。

| endpoint | 本轮帧数 | 实际 frame shape / 口径 |
| --- | --- | --- |
| `/connections` | 21 | `{connections: []/null/[{id,metadata,upload,download,start,chains,rule,rulePayload}],uploadTotal,downloadTotal,memory}`；active snapshots，立即首帧，随后默认 1000 ms；totals 累计 bytes |
| `/logs?level=debug` | 72 | `{type: string,payload: string}`；按日志发生时间推送，文本 payload 无稳定结构化 DNS history |
| `/memory` | 22 | `{inuse: integer,oslimit: integer}`；约每秒，首帧 inuse=0 为源码刻意的 sentinel；后续是 Go allocation bytes，oslimit=0 为未实现占位，不是实际 OS limit/RSS |
| `/traffic` | 5 + 16（两次订阅） | `{up: integer,down: integer}`；每个相邻计数区间的 bytes，不是累计或已除以时间的速率 |

connections metadata 实测有 `network,type,sourceIP,destinationIP,sourcePort,destinationPort,host,dnsMode,processPath`。host 可用于本次诊断；空 processPath 不填进程。源码 `dnsMode="normal"` 是固定值，不证明 DNS 实际路径；rule 文本不是规则序号，rulePayload 本次为空。

## Traffic 双证据

[固定源码 server.go](https://github.com/SagerNet/sing-box/blob/0b8995879f29a9b98ee027bc17b75e101445b238/experimental/clashapi/server.go#L303-L353)：订阅时读取 `TrafficManager.Total()`，`time.Second` ticker 后取 new-old，成功发送后 old=new，不执行速率除法。`common/trafficcontrol/manager.go:124–139` 汇总 active/closed/已移出 closed 列表的计数；tracker 的 TCP/packet counter 包装 routed byte I/O。[真实计数证据](traffic-semantics.json)用 WS sum 与同口径 REST Total 差值核对单位，不从 payload 大小猜测精确总量。

| 独立受控阶段 | 上传 body / 下载 body 基础大小 | WS up/down sum（bytes） | REST up/down Total 差值 |
| --- | --- | --- | --- |
| 第一次 burst | 131072 / 524288；下载另有唯一 marker | 131224 / 524449 | 131224 / 524449 |
| 重订阅后第二次 burst | 65536 / 262144；下载另有唯一 marker | 65687 / 262305 | 65687 / 262305 |

每次先至少两个 idle=0 帧，burst 见非零，随后回到 0。计数包括被追踪 HTTP bytes/headers，不能等同于 body 或网卡/IP wire bytes。两次 burst 的 down sum 也分别等于客户端实际读取的 HTTP response 字节数 524449/262305，直接核对 bytes 单位。一次仅该测试请求，controller 自身读取不经 mixed，因此两个阶段可与 Total 差值精确对账。

- 单位是区间 **bytes**。名义区间 1000 ms，traffic 无 interval 参数；frame 无服务端 interval/timestamp。所有到达时间保留，实际到达间隔可由 monotonic_ns 计算；及时排空时近似计数区间，传输/调度滞后时不能承诺精确速率。
- 转换：`bytes/s = frame_bytes × 1000 / interval_ms`。524288 bytes、1002 ms → 523241.516966 bytes/s。不能对 frame 再差分，也不能用整个 UI 轮询间隔替代内核计数区间。
- Rust 后续由单一持续 collector 给 `(instance_id, stream_generation, sequence)` 定位，每个增量仅累加一次；session sum 与 lifetime REST total 不重复叠加。这是已核定的实现契约，本原型没有实现产品 collector。
- 第一个 WS 完全关闭后，间隙中产生一次 8192-byte upload 请求；新订阅仍先收到两个 0 帧，第二 burst 的 WS sum 只等于第二 burst 的 REST delta。重新建立 baseline，不回放旧总量或间隙 bytes。
- 同实例重连关闭旧 generation、清除 timing baseline，保留已确认 session sum 并标明缺口；若以后用 REST 补缺口，必须对齐窗口并与已消费 WS 去重。本卡不实现补计。instance change 清 session/timing/generation，拒绝迟到旧实例帧；累计计数回退建立新 baseline/不连续标识。

现 core `RuntimeObservationBridge` 读取固定端口的一条 traffic 帧，把窗口 bytes 直接当一秒速率，累计取此前 REST；`application/observability.rs` 承接安全摘要并在 child lifecycle 清零。相关测试保护独立窗口、不再差分和 child replacement。它们没有证明连续 collector 或精确 interval；本轮仅阅读并 `cargo check -p veyra-core`，不修改产品代码，P3 按此证据归一化。

## 长/短连接实验

[connections-gap.json](connections-gap.json)：诊断 origin 延迟 2.4 秒，长请求跨两个 connections snapshots，看到同一个 UUID、实际入口、host/port、chains、start 与计数。随后紧随一个新 snapshot 发起短请求，整个请求耗时 **3.688625 ms**，完整落在两个已收到快照之间；用 source port 排查所有实际帧，**短请求未出现在 connections**。

短请求仍返回 HTTP 200、origin marker 正确；对应 inbound log 存在，REST Total 增长 up **125 bytes** / down **4253 bytes**，附近 traffic 非零。标准 endpoint 是 active snapshots，internal closed list/events 未作为 controller event/history route 暴露；不能把低频 snapshots 当完整连接历史，也不能将未归属量按配置猜给某节点/终端。仅证明本轮缺口，不断言所有短连接都漏。SQLite 桶宽、索引、保留期、容量留 P3。

## DNS 三项分别核定

[DNS 能力](dns-capabilities.json)保留源码依据、真实 response、缓存前后行为、普通 DNS 日志；三项互不替代。

| 能力 | 判定 / 证据 | 首期产品语义 |
| --- | --- | --- |
| Query records/history | **UNSUPPORTED**：源码 `/dns` 仅 GET `/query`；records/history/tracing 实测 404；普通 `dns: exchanged/cached` 只有文本 domain/RR/qtype/result/TTL 和可选 debug summary，无稳定 query ID/source/elapsed/filter-hit | `dns_query_records unavailable`，显示原因；不由 logs 或 connections 合成记录/耗时/来源/命中；collector receive time 不冒充 DNS query time |
| DNS response cache flush | **SUPPORTED**：源码注册 **POST `/cache/dns/flush`**→DNSRouter.ClearCache；实测 204；错误 GET 为 405 | 仅当前受管实例缓存清理；与查询历史、FakeIP mapping reset、系统 DNS 分开 |
| Rewrite/rule/hosts/predefined hot update | **UNSUPPORTED**：没有受控 mutation API；`/dns/rewrite`、`/dns/hosts`、rule provider update 为 404；configs PATCH/PUT 的 DNS payload 为 204，但前后 hosts 查询答案仍 127.0.0.1；源码 PATCH 仅 mode、PUT no-op | macOS 首期 DNS rewrite 保存后 `RestartRequired`；不承诺目标/备注编辑可直接运行生效，不扩展自建 DNS 或 patch 内核 |

缓存实验：首次自有上游答 `192.0.2.10`；把 fixture 答案改为 `192.0.2.20`，再次查询仍返回 `.10`，上游请求数仍 1；真实 flush 204 后查询得到 `.20`，上游请求数增为 2。没有删除 cache.db、清 UI 列表或动系统 DNS。

另一真实 route **POST `/cache/fakeip/flush`** 调用 optional CacheFile.FakeIPReset，与 DNS response cache 不同；本原型未启用持久 cache/FakeIP，未操作或宣称这一 mapping reset 的效果。source route/method 枚举包含它；不以普通 DNS flush 证明 FakeIP flush。

## 真实受管诊断与缺失路径

[diagnostic.json](diagnostic.json)记录唯一非秘密 path、有界窗口、origin receipt/body marker、HTTP 200、connections 和相关日志/traffic。只存在一个 pending 请求，source port 在此时间窗唯一，跨帧 connection UUID 相同。日志 numeric context ID 由该 source port 关联，**不是** snapshot UUID。

实测字段：request success/status、`mixed/p007-mixed` inbound、`p007-direct` outbound、`p007.example.invalid` 和 origin port、connection UUID、start、loopback source port、snapshot 当前 upload/download、独立诊断窗口 Total delta（up 130/down 4258 bytes）。收到的 debug logs 同 context 出现 `router: match[1] ... => route(p007-direct)`，因此本次规则 index **1** 可关联；只限本配置/日志，不是稳定规则 ID/公开结构化 API。snapshot rule 文本本身不提供 index。

**仍不完整**：唯一结构化 DNS event、完整 resolver chain、process identity、filter hit 均 absent/unknown。普通 DNS lookup/resolve 文本不能推出唯一 DNS query ID、完整解析链或 filter hit；没有填写预测结果。长请求快照在 origin 延迟时 download=0 是当时观察值，不冒充最终单连接下载量；最终字节来自该独立窗口 aggregate Total 差值。

## 验证、自查与清理

[validation.json](validation.json)：官方 source/tag/commit/path、archive digest、config check、真实四流/traffic 两 burst+reconnect/long-short/DNS三项/local diagnostic、JSON parse、严格脱敏、脚本 AST、core check、68-card DAG、Markdown links/anchors、diff 与资源清理。每个实验保护 Veyra 的单位/归属/能力声明/真实诊断/自有实例清理契约，不新增内核协议认证测试或产品单测。

按 [code-delivery-review](../../../../.agents/skills/code-delivery-review/SKILL.md) 作**实现者自查**，不是独立审查。首轮实验取得结果，但 macOS port checker 在 connect_ex 后对同一 socket bind 失败；[首轮 FAIL](initial-probe-result.json)保留，改成独立 socket 后旧 listener/group 回查通过并完整重跑。自查又发现启动默认 interface 行需要整行脱敏，修复后第三轮完整重跑为最终证据；[history](probe-history.json)保留过程，不把早期失败改成 PASS。最终输出不保存真实用户名、用户/temp 路径、凭据或主机 gateway/interface/IP。普通 loopback/Test-NET/`*.example.invalid` 数据保留。

[probe-result.json](probe-result.json)：最终 child PID 82356 正常 SIGTERM/exit 0、wait 回收且 process group 不存在；全部 WS/reader/fixture threads 已关闭，controller/mixed/origin TCP 拒绝连接且独立 socket 可重新 bind，UDP fixture 可重新 bind，内部 archive/source/binary/config 临时目录删除。没有强制 kill。启动网络更新监听仍报 operation not permitted，保留环境限制；本卡只证明 loopback，不证明主机网络变化监听。

复跑要求 macOS arm64、Python 3.12+、系统 curl/git/lsof；写到全新输出目录，避免覆盖历史：

```sh
python3 tools/p0-07-observation-probe.py --output <new-evidence-directory>
```

本轮外层 Python `subprocess.run(..., timeout=330)`；下载各 120 秒、tag 40 秒、check 20 秒、启动发现 10 秒、API/握手 3 秒、WS frame 2 秒且大小/数量有上限、mixed 请求 5 秒、正常停止 10 秒。内部临时目录 finally 删除；用户指定的脱敏输出目录保留供审阅。当前任务 evidence 中只保留正式脱敏输出与必要历史摘要。

DAG 仍 68 卡/61 macOS+7 Windows；DONE 7、READY 2、TODO 52、Windows DEFERRED 7。READY 为 P0-05/P1-03；本卡没有释放新 READY，P0-09/P3-01/P5-01 等仍缺其他依赖。未启动下游。
