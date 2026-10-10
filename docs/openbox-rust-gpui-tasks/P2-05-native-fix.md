**2026-10-10 历史移植说明**：本文件保留旧独立树交付及当时计数/结果；旧 ignored 原始证据仍只在 `/Users/lifeilin/.codex/worktrees/p2-05-outbound-clients/veyra/docs/openbox-rust-gpui-tasks/evidence/p2-05/production-clients/`。最新 main 整合及当前状态以 [P2-05 整合验收](P2-05-integration-acceptance.md) 为准，旧 PASS 不替代本候选复跑。

# P2-05 指定节点 Native 503 修复

2026-10-09：指定 NodeId Native **1 PASS / 0 FAIL**；旧503 finding已关闭。Task仍 **ACCEPTANCE**，owner保留；用户已明确选择“保持 ACCEPTANCE，等待完整网络组合验收”。P2-06 DOING/原owner不变。当前托管树 `dev/p2-05-outbound-clients`，输入Host checkpoint `e530837976257d489d3bf455d5d96e63e704f442`；Host已独立复跑183 PASS/2ignored及工程检查。当前增量尚未提交/合并/push，不提前DONE。

## 根因与最小修复

旧 `native-relay-final` 保留0bytes/503原始结果；本轮 `native-node-phase-diagnostic`补时间：2ms收到CONNECT、3ms回复200、立即WouldBlock并半关闭、7ms Controller503，实际未等10秒timeout，也未开始TLS交换。

macOS自有fixture将TcpListener设nonblocking以有界accept，但没有将accepted TcpStream切回blocking；accepted socket继承O_NONBLOCK，read_timeout不能替代blocking模式。修复的真实fcntl收据：`inherited_nonblocking=true` → `blocking_after_fix=true`。只在已有Native fixture的accept后显式 `set_nonblocking(false)`；强化两个方向bytes>0及原有node有效delay聚合断言，保留有限read/write timeout。增加脱敏请求、阶段及TLS首record类型/byte count，不记录TLS payload。

只改 `crates/veyra-desktop/src/runtime_service.rs` 的ignored Native测试，生产Core/Runtime/Platform/Helper/IPC没有改动。不换目标、没有关闭TLS验证、沒有替代正式NodeId调用、没有自动fallback。没有以Content-Length变通修复：隔离固定kernel的原样200 CONNECT和CL0对照均完成TLS/返回200，因此已排除该假设。

## 固定目标的真实证据

固定 sing-box 1.14.0 darwin arm64，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。自有AppServices/Store/root，通过正式Runtime Ready及既有服务。以下最终值来自 `native-node-final-pass2.log/json`：

| 项目 | 实际结果 |
| --- | --- |
| 唯一目标 | `https://www.gstatic.com/generate_204`，未替换 |
| 当前实例 | `manual-23bb0ad51908515f772dd31487589c888311aad298be49b07ea8c3c12eb5a33f-1` |
| 当前mixed/controller | `127.0.0.1:54414` / `127.0.0.1:54415`，真实Ready发现 |
| ViaRunningProxy site | **712ms** |
| 指定节点 | 正式 `node_latency(NodeId("a"))` → 当前applied `node-a` → **705ms** |
| 自有HTTP节点 | `127.0.0.1:54389`收到`CONNECT www.gstatic.com:443` |
| 固定fixture上游 | 同一实例mixed CONNECT200；node→owned HTTP fixture→current mixed(default Direct)→既有外部TUN，不冒充独立远端节点/物理Direct |
| 双向转发 | **1707 / 4081 bytes**，首TLS record type22，未保存payload |
| 三provider实际出口 | ip.sb=`2a09:bac5:624d:78::c:3c2`；ipwho.is/ipapi.is=`123.121.111.183`；查询8.8.8.8均ASN15169 |
| 生命周期 | Replace撤销旧能力，Quit撤销当前能力，两个实例listeners关闭，线程join/root删除：PASS |

请求记录为从本次**实际加载**的私有candidate配置核对NodeId=a、HTTP节点地址、applied tag后，按封闭controller client契约记录的URL；不是抓包，也没有用tag替代正式NodeId服务调用：

```text
GET http://127.0.0.1:54415/proxies/node-a/delay?url=https%3A%2F%2Fwww.gstatic.com%2Fgenerate_204&timeout=10000
Authorization: REDACTED
```

正式服务收据是成功HTTP响应解析出的705ms；精确线级HTTP状态/响应另由隔离固定kernel诊断 `node-native-diagnostic-compare.log` / `node-native-diagnostic-events.json` 提供：同目标、显式HTTP node、同mixed上游，GET `/proxies/node-diagnostic/delay?url=https%3A%2F%2Fwww.gstatic.com%2Fgenerate_204&timeout=10000`，实际 **HTTP200 / delay704**（原样无CL CONNECT），另CL0对照 **HTTP200 / delay812**。该诊断用默认系统信任的TLS验证同目标mixed HEAD204，双向1707/4082 bytes；它是定位分层对照，不能代替上述正式NodeId/AppServices验收。动态localhost端口/实例仅绑定本次自有测试，不采用扫描发现。

## 检查、独立Review与保护边界

本轮定向RuntimeService **5 PASS / 2ignored**，显式Native **1 PASS / 0FAIL**；5项与Host183重合，不重复计数。三包all-targets Clippy、三包build、workspace/src-tauri fmt及diff通过；生产源码未变化，不扩大运行全量Store测试。旧Legacy Windowslibcronet.dll资源失败/历史Core两项FAIL/Desktop全量Store Busy/旧Native失败不改写。

独立readonly Reviewer `/root/review_p205_native` 核对修复、fcntl模式、同目标/同当前实例/applied节点、字节/结果/清理及出口边界；最终报告放本机 `evidence/p2-05/production-clients/native-fix-review.md`。独立Review不冒充另一次Native复跑。

本轮网络快照前后核对utun4与所有utun配置/index、DNS、路由、系统代理、网络服务及外部owner；不修改系统代理/DNS/默认路由/network services，不关闭/重启/接管TUN。物理Direct仍 **UNVERIFIED**（正式服务`DirectPathUnverified`），SystemProxy ON仍 **NOT_RUN**；受管TUN绕行仍P6、SystemProxy组合仍P2-07/P2-09后续，不把TLS成功或两个公网IP当物理Direct证明。

## 有限范围验收建议与用户决定

本轮曾建议只确认P2-05自身生产客户端的显式逻辑Direct（自有HTTP/HTTPS/DNS fixture）、真实受管ViaRunningProxy/指定NodeId到同一HTTPS目标、订阅凭据/失败保旧、三provider与关闭清理。物理Direct及SystemProxy ON两项保留原UNVERIFIED/NOT_RUN，在P2-07/P2-09/P6相应平台组合继续验收；不修改原始历史证据，不放开系统网络开关，不宣布TUN绕行已完成。

用户已答复：**“保持 ACCEPTANCE，等待完整网络组合验收”**。因此本轮仅关闭Native503问题；P2-05保持ACCEPTANCE，owner保留，物理Direct UNVERIFIED/SystemProxy ON NOT_RUN继续作为未完成验收项。本轮不提交增量、不合并主分支、不push；不修改用户网络以补齐组合。DAG仍68卡DONE26/DOING1/ACCEPTANCE1/READY3/TODO30/DEFERRED7，READY=P0-08/P3-03/P4-03。
