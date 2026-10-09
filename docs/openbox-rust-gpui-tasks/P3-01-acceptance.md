# OBG-P3-01 交付记录（2026-10-09）

状态 **ACCEPTANCE**，唯一 owner **Codex · Observation**。依赖 P2-03/P0-07 均 DONE，不增加对 P2-06 的依赖。分支 `dev/p3-01-observation`，隔离 worktree `.worktrees/p3-01-observation`，基线 `ed174e6aed2b8a331da8d93073a580c76ea6949d`。临时文件写/读/删预检 PASS；未触碰主树用户文件。本次本地提交未生成：git add/commit 因 `.git/worktrees/p3-01-observation/index.lock` Operation not permitted 被沙箱拒绝，立即停止Git写入；HEAD仍为基线。Host需在具备Git元数据权限的环境精确暂存本卡文件（evidence目录被既有ignore规则排除，须明确保留本卡证据）后完成独立commit，不push。

生产实现位于 `crates/veyra-core/src/application/observability/controller.rs`，经现有 `ClashApiClient::managed()` / `ManagedControllerEndpoint` 建立 `/traffic`、`/memory`、`/connections`、`/logs` WS。复用 token header、仅受管 loopback、endpoint撤销与固定错误类别。一个 Runtime owner 持有一个 ObservationService，页面只读取 snapshot/observed 或订阅事件，不自己开网络。

- Snapshot 和小型身份/版本 broadcast 通知（64条）；落后返回 `Delivery::Gap { dropped }`，调用 `snapshot()` 重新取得最新状态。采集不等待消费者；日志1000条，每条仅原有静态allowlist摘要，原始payload不进入快照。日志字节数由固定摘要长度×1000约束。
- 直接消费已有 `runtime_snapshot::InstanceId`，另带观测generation/stream_generation/sequence；不建立Runtime状态或新增Runtime DTO。换源/stop先清快照和clients并abort旧任务，过时publish拒绝。撤销endpoint在有连接时每100ms检查；连接创建沿用2s期限。指标3s断流标不可用，日志正常沉默不误判；endpoint仍有效且Identity仍匹配时以100ms至2s有界退避自动重连，不依赖事件订阅数，停止不重连。
- traffic按interval bytes×1000/monotonic到达interval近似速率，绝不再次差分；每代sequence只累计一次。重连保留已确认session sum并记录gap；新instance清零。lifetime totals来自connections，独立于WS session sum，回退记counter_resets，不补计断流缺口、不制造可归属恒等式。断流保留最后确认累计/速率但available=false、interval_ms=0，消费者不得作为实时速率展示。
- 内存未采到/断流为None；每条memory generation首帧0为P0-07哨兵，保持None。connections:null归一化为空活动集合。仅投影真实sourceIP/inboundName，`observed()`无来源返回None、确有空快照返回Some(empty)；不猜设备名/MAC或网络扫描。完整connection稳定ID/详情留P3-02。
- connections原始消息/帧限制1MiB，其它流16KiB；大于上限拒绝并暴露gap/Unavailable，不声称无容量上限。

## 实际验证

全部在本worktree，测试端口由自有listener bind(127.0.0.1:0)取得，未扫描实际内核或用户网络，未运行sing-box child/GUI/系统权限操作。长命令由Python subprocess timeout 90–180s约束。完整命令和原始日志在 [evidence/p3-01](evidence/p3-01/README.md)。

| 检查 | 实际结果 | 日志 |
| --- | --- | --- |
| 最终观测回归（含2新增） | 23 PASS / 0 ignored | delivery-observability.log |
| 既有controller/P0-07口径回归 | 16 PASS / 0 ignored | verified-controller.log |
| P2-03 Runtime Mock定向回归 | 11 PASS / 0 ignored | p2-03-regression.log |
| Domain基线回归 | 45 PASS / 1既有FAIL，MissingPool/InvalidFilter断言；未修改 | domain-baseline.log |
| Core all-targets Clippy -D warnings | PASS | delivery-clippy.log |
| Core build / workspace fmt --check | PASS | delivery-build.log / delivery-fmt.log |
| 旧Tauri --lib Clippy -D warnings | FAIL，Windows sing-box.exe资源缺失；旧LICENSE缺失历史也保留，不称通过 | legacy-clippy.log |
| diff检查/68卡DAG | PASS | final-checks.log / dag.json |
| 独立只读Review | 3具体Finding修复后复核关闭；未发现剩余可信问题，Reviewer未跑测试 | review.md |
| P2-06正式child生命周期接线、真实GUI/用户网络 | NOT_RUN / 未实现接线，不在本轮授权范围 | 下述接口 |

真正WS证据：新增 `production_four_ws_reuse_gap_replacement_reconnect_and_clients` 使用生产Service/Client，fixture验证4条path和Bearer鉴权；两个订阅不增加连接，100条日志让慢消费者实际收到Gap；实际连接JSON投影sourceIP；空活动null与>16KiB原始JSON均读取成功；memory断线重连首帧0→None→456；瞬时替换到第二自有endpoint，旧publish拒绝、旧socket释放；endpoint撤销/stop清clients，fixture任务最后abort并await。解析/累计/重置/过期generation/日志脱敏另有单测，不能把fixture说成真实sing-box或正式Runtime接通。

初轮编译FAIL（新增enum分支/fixture类型）、typed identity调整编译FAIL保留原日志；修复后最终命令通过，未跳过测试。P5-05动态NOT_CAPTURED和其它历史失败不改写。

## 下一接口（仅2项，交原Runtime owner）

1. 正式受管Ready提交成功后，以既有 `InstanceId` 和 `Arc<ManagedControllerEndpoint>` 调用唯一Service的 `bind(instance_id, endpoint)`；由实际owner交付endpoint，不扫描端口，不产生新的Runtime事实。
2. owner在stop/replace/handoff前调用同Service `stop()`，新Ready重新bind；页面/未来P4-05B只读取snapshot/subscribe/observed。

本轮没有修改Runtime/Platform/Helper/IPC、RuntimeEvent/RuntimeSnapshot公共DTO或Cargo workspace/lock。上述正式owner接线尚未落地，保留ACCEPTANCE和owner，不标DONE、不解锁下游。当前68卡：DONE24/ACCEPTANCE1/DOING1/READY3/TODO32/DEFERRED7，READY=P0-08/P2-05/P4-03，无新任务启动。Host后续独立复核并合并；本轮不push。


## P3-01-HOST-001 修复（2026-10-09，当前）

Host发现：断流重连等待依赖 `state.events.receiver_count()>0`，只读 snapshot()/observed() 的消费者没有广播订阅时会永久失去观测。已修复：删除该条件及等待订阅的内层循环，保留原100ms–2s退避、endpoint有效性与Identity检查；持续采集归属于受管实例。停止/换代仍退出，原64条有界broadcast不改。

只新增一个测试 `production_reconnects_without_event_subscribers_and_stop_releases_sockets`，全程不调用 subscribe()；沿用生产Service/Client和自有动态loopback fixture。四路鉴权WS建立，memory收到64后断开，随后确有新的 `/memory` 鉴权握手、generation=2/sequence=1及snapshot=128；其它三流不重复建立，stop清来源并释放全部socket，随后300ms观察窗无新握手。测试有界accept/read等待，不操作真实内核、GUI、权限或用户网络。

| 本轮验证 | 实际结果 | 原始证据 |
| --- | --- | --- |
| 修复前新测试 | 0 PASS / 1 FAIL，3s内没有重连握手，准确复现Host问题 | host-001/before-fix.log |
| 修复后新测试 | 1 PASS / 0 ignored | host-001/new-test.log |
| 全部P3-01 observability回归 | 24 PASS / 0 ignored，含上项，计数不重复累加 | host-001/observability.log |
| controller回归 | 16 PASS / 0 ignored | host-001/controller.log |
| Core all-targets Clippy -D warnings / fmt / build / diff | 全部PASS | host-001/commands.json及同名日志 |
| 修复审查 | 实现者按code-delivery-review自查通过，不称新独立Review；Host最终复核未运行 | review.md追加项 |

[本轮精确身份与命令](evidence/p3-01/host-001/README.md)。历史23/16/11及初轮FAIL日志均保留；未重跑domain/Tauri，原FAIL不改写。仅修改collector重连及该测试/本卡记录，Runtime/Helper/IPC/公共Runtime DTO及P2-06 owner不改。P3-01仍ACCEPTANCE，正式child生命周期接线未完成，owner不释放、DAG不变。不再尝试Git commit；Host之后使用受管Git命令完成checkpoint，本轮没有推送。

## Host 独立复核与提交边界（2026-10-09）

P3-01-HOST-001 **CLOSED**。Host 独立检查最终 controller.rs、WS 客户端及测试，并实际复跑 Observation **24 PASS**、Controller **16 PASS**、P2-03 Runtime **11 PASS**；Core Clippy/build/fmt、git diff --check 全部 PASS。新增的无事件订阅者自动重连测试实测通过；原有独立 Domain/Legacy FAIL 与未执行的 Native/GUI 项目原样保留。Host 未复跑用户真实网络或正式 P2-06 child 生命周期，不能以 loopback 冒充正式实例。

本轮只将可独立复核的 **ACCEPTANCE 检查点**提交到 dev/p3-01-observation。**不提升 P3-01 为 DONE，不释放 Observation owner，不合并 codex/dist-react-restore，不推送。** 仍须 P2-06 原 owner 在正式实例 Ready/Stop/Replace 时消费 bind()/stop() seam，并通过相应真实集成与复核才能收口。
