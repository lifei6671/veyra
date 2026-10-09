# OBG-P3-04 统计存储与容量交付

## 正式统计接线最终验收（2026-10-09）

**状态 DONE**；基线 `472a4f205083d24a43e2174a2c9e3810b1db2476`。用户授权真实记录接线、P2-06 owner协调、独立Review、验收后本地提交/合并并复验，不push。P3-01 DONE、P2-06 DOING不变；本卡Observation/Traffic Storage owner及预约释放。P3-05/P3-06仍TODO，各自剩余依赖未满足。下方检查点与历史FAIL/NOT_RUN保留。

### 正式通路与口径

`Desktop RuntimeService → 正式 ManualRuntime Ready → 原 ObservationService 四WS → /connections 同帧只读记录 / /traffic 原区间bytes → 原 TrafficWriter → traffic/traffic.sqlite3`。

- P2-06 原owner只读确认最小边界，允许本卡添加ManualRuntime统计配置和Desktop composition；不改Helper、IPC、bootstrap/rebind、公共Runtime DTO或系统代理/TUN。Helper默认不打开用户库，Helper记录跨进程投影仍归P2-06；不由Desktop另开采集连接。
- ConnectionRecords含InstanceId/观测generation、stream_generation/sequence、UTC客户端接收时间、稳定连接ID和累计bytes。Source.epoch为`observation-{generation}`，与包含owner随机nonce的实际InstanceId共同隔离；采样key含流代次/顺序/帧内索引，重放拒绝且SQLite累计checkpoint不重复加。
- node仅实际`chains[0]` outbound tag，host仅非空metadata.host，client仅实际sourceIP；不等同业务NodeId、设备名或所有者。API没有outbound type，direct为None；缺id/计数不写伪记录，incomplete_records明确报告。来源依据：[1.14.0 connections](https://github.com/SagerNet/sing-box/blob/v1.14.0/experimental/clashapi/connections.go)、[tracker链顺序](https://github.com/SagerNet/sing-box/blob/v1.14.0/common/trafficcontrol/tracker.go)。
- /traffic保存实际区间bytes，不差分或从速率反推。时间区间为客户端到达间隔近似，非服务器精确窗口；连接增量使用两次已确认累计的接收时间。首次只建baseline，短连接或最后采样后结束的尾量不推断、不用total-attributed拼平。observed和partial不能相加作权威总量。
- 原批量、队列、去重、重置/维度变化、容量/保留策略不变。正式Desktop统计日界固定UTC并写入库metadata，后续P3-06存储设置另处理变更策略。
- 仅Ready bind打开DB；候选预检失败保留旧统计，实际Replace/Stop/Recovering/Drop沿原观测stop关闭。先撤销identity并取走writer，旧任务不可再submit；drain/commit/join后返回，DB/文件lease已释放。Handoff为freeze→Stop→封存/release；Quit由实际RuntimeService shutdown走同一Stop，不由隐藏窗口触发。
- Open/Submit/Commit失败保留只读TrafficStorageStatus并输出固定诊断。已入队不冒充提交，最终失败批/损失状态在join后读取；统计失败不阻止必要的内核Stop。

### 实测与独立Review

原始命令、exit、日志、源码身份、清理与独立Review见 [formal-wiring](evidence/p3-04/formal-wiring/README.md)。Cargo offline，外层600s timeout；相关最终source aggregate `e3a3d297f69da9bf6b6251583de4f667fe72ec01da918792ae38d48ec77796eb`。

| 验证 | 实际结果 |
| --- | --- |
| Controller只读源/重连/未知/失败回滚/释放 | 6 PASS；真实四WS fixture与SQLite |
| Traffic Storage/容量/高基数/DST/重建 | 16 PASS；10000样本/5000组合，5042176bytes，797ms，索引窗口3313μs；每日强度推算仍非24h实跑 |
| Runtime/Observation定向、Handoff统计关闭 | 7 PASS + Handoff 1 PASS（Runtime fixture；未执行root交接） |
| Desktop worker定向 | 5 PASS/1 ignored；ignored是真实Quit父test，随后显式执行 |
| 真实1.14.0 WS→SQLite | 1 PASS：双向observed interval bytes、连接确认下载增量(0,16384)、连接结束、两实例隔离、落盘、writer/Runtime重建回读、Stop/reap/端口/lease释放 |
| 实际RuntimeService.shutdown | 1 PASS：Ready后唯一writer、Quit回告后真实child端口与SQLite lease释放 |
| Core/Desktop/Helper production all-targets Clippy、build、fmt/diff | PASS |
| 旧Tauri Clippy --lib | FAIL：既有Windows libcronet.dll资源缺失；未改变旧配置，不能记PASS |

真实内核复制品1.14.0 darwin/arm64，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`；只访问测试自有127.0.0.1 HTTP服务，不改变系统代理/TUN。真实连接上传仅建立baseline后零增量，双向连接累计增量由受控WS/SQLite测试验证；不声称实测两方向连接增量。root helper/GUI/完整组合仍NOT_RUN，分别归P2-06/P3-05/06/08及后期组合卡。

独立agent `/root/review_traffic`按code-delivery-review只读源码/调用链/收据，未发现可操作行为Finding，明确支持P3-04 DONE。注释建议已修正，不修改执行逻辑。初始u64 FromSql编译FAIL、CONNECT读取超时、HTTP响应先结束导致无最终连接帧、空AppState Quit fixture CompileFailed以及旧Tauri FAIL日志均保留；真实fixture失败资料归档后清理，本轮6个owned child均退出，成功用例数据库锁/端口释放。没有全量Core/Desktop PASS声明，旧两项Core基线FAIL和Desktop Busy历史不变。

本卡仅后端数据，不增加UI或下钻查询。Task四项现有验收已满足，P3-04 ACCEPTANCE→DONE；本地提交/主分支合并与合并后复验收据追加到本轮evidence，不覆盖本次及旧失败。

## 472a4f2 检查点历史（以下保留原记录）

2026-10-09；owner=Codex · Observation/Traffic Storage。状态 **ACCEPTANCE**，owner保留，不解锁 P3-05/P3-06；唯一依赖仍P3-01 DONE，P2-06 DOING，不新增硬依赖。

隔离 `.worktrees/p3-04-traffic-storage`，分支 `dev/p3-04-traffic-storage`，源码基线 `a410d579b9768de43639d24b42a03afb3b77f236` 加本次未提交diff。仅新Core storage/traffic、模块出口、Core Cargo.toml、Cargo.lock和本卡文档/evidence。主开发树未写入，不改分支/暂存/清理，不push、不尝试Git commit；交Host受管本地checkpoint。

## 生产存储与数据口径

真实 bundled SQLite/rusqlite 0.40.2，schema user_version=1；目录由受管app或测试注入，数据库traffic.sqlite3。每目录OS文件lease保证单writer，退出join释放；SQLite DELETE journal、FULL同步、2秒busy timeout。队列1024、批256、2秒flush，每字符串最大512bytes；submit只确认接收，flush确认事务。错误批回滚保留旧事实、writer停止；Status区分rejected、failed_batch、uncommitted_lost及已commit事实，错误详情通过Result传播，日志不打印连接/目标/用户信息。

Source(epoch,instance)+稳定sample key幂等；connection id累计checkpoint与事实同事务提交、重开回读。首次累计只建baseline，旧累计不猜归属；重复/逆序不重加，单向reset不抹掉另一方向确认增量，维度变更区间记dimension_gap，部分维度记partial。ObservedInterval直接保存P0区间bytes，不差分；Unattributed仅接收直接确认的未归属量。各kind不能直接相加当权威总流量，也不以max(0,total-attributed)拼平。无全机流量推断。

真实区间start_ms/end_ms、来源身份、实际组合与测量终点60秒桶均保留；跨桶/跨日增量属于测量终点，保留区间，不伪造按秒均摊。day由固定IANA统计时区转换，时区绑定DB，改时区拒绝静默重分桶。chrono已锁版本直接使用，chrono-tz为必要DST转换依赖。每日连接数是每source稳定id的日内去重；月跨日求和为connection-days，不声称跨日distinct。明细连接presence按桶去重。

## 固定保留和容量

默认明细滚动168小时（不假定本地一天24小时），长期day/direct汇总保留365个本地日；长期不承诺node/host/client关联下钻。超过范围旧输入拒绝，checkpoint与去重凭据有界清理，旧连接再次出现先建立baseline。默认256MiB预算，真实page_count/freelist/page_size与数据库文件/WAL/SHM测量，WAL/SHM为0。整桶按时间淘汰，持久化cutoff与shortened，重开恢复；容量不足以保留权威daily/必要去重时显式Capacity并停止，不偷偷TopN/other。事务后incremental_vacuum；实际文件超预算会显式停止，不能声明不可瞬时越界。DELETE rollback journal临时额外磁盘需预算，非持久WAL；256MiB不是进程全部磁盘峰值上限。

必要索引：detail_window(epoch,instance,bucket_ms,node,client,host)、detail_bucket、detail_time、counters_time、seen_time及visits_day；查询中断/UI/P3-05延迟/DNS不在本卡。

独立测试作者设计高基数实验，真实10000样本/5000不同host/client/combo：5,042,176bytes，1231页×4096，freelist0，最终766ms约13042样本/秒；100次索引窗口SUM平均3122μs，EXPLAIN命中detail_window。每分钟新增5000组合/10000样本模型×1440=7,260,733,440bytes/日（6.76GiB），为保守推算，未实跑24小时。256MiB约53分钟该强度；不能承诺此负载七日明细。成功容量缩短与重开恢复已由独立设计基础上追加的真实SQLite测试PASS；首次15PASS/1FAIL定位incremental_vacuum结果未排空，修复后16PASS，旧失败日志保留。

## 验证与真实生产缺口

原始日志：`evidence/p3-04/`。首次cache目录不可写、首次u64/usize SQLite绑定编译FAIL及零Runtime过滤历史原样保留。使用隔离cargo-home从普通registry取得真实依赖，不升级既有crate版本。

- Traffic最终16 PASS（含成功容量清理、重开范围、单writer lease），见traffic-delivery-2.log。早期12PASS/15PASS及编译FAIL、容量15PASS/1FAIL日志均保留。
- Observation 24 PASS（本地四WS受控测试，没有落盘接线）；Store20 PASS/1内部child fixture ignored。
- Core all-targets Clippy/build PASS；最终版本见clippy-delivery/build-delivery/fmt-delivery，均PASS。
- runtime.log错误过滤0 tests，不算PASS；正确正式ManualRuntime Observation回归2 PASS，见runtime-final.log。
- 已知Core两项baseline FAIL、Desktop完整套件时序Busy历史不改写；本轮不运行全套、不扩大修复。

最小缺失正式入口：现有 RuntimeObservationSnapshot只暴露summary/active数量，controller Snapshot无稳定connection id/累计bytes/node/host/direct组合。新的storage::traffic::{Source,Sample,Count,Dimensions}是可消费有类型seam，但无正式生产调用方。需Observation owner发布带稳定id、累计口径、计数区间、实际维度与来源身份的最小记录流，再由受管生命周期绑定writer；不能修改P3-01已验收Collector语义，也不能以DTO测试替代真实四WS→落盘。因此本卡ACCEPTANCE。

独立只读Review由独立agent按code-delivery-review完成；checkpoint、容量整桶循环、掉队竞态、retention恢复、每日去重和DST保留finding已修复。最终无新的确定代码Finding；最终容量成功压缩已PASS，生产接线缺口保留。无主树改动、无Git写入。本Task未完成，Host可受管独立commit/checkpoint，不进行DONE合并流程、不释放owner。

P0 traffic证据依据方案§10.2中固定1.14.0的源代码/真实idle-burst-reconnect记录；原忽略JSON未包含在本隔离worktree，不重写旧evidence。

## Host 独立复核与提交边界（2026-10-09）

Host 复查最终 traffic.rs、真实 SQLite schema/单writer/整桶容量清理/归属区间/去重/重新打开、traffic/tests.rs、高基数测试及 P3-01/P2-06 写范围。独立受管命令真实复跑：Traffic **16 PASS**、Observation **24 PASS**、Core storage **40 PASS/1 ignored**（已包含 Traffic16，非额外不重复测试）、正式 ManualRuntime 观测定向 **2 PASS**；Core Clippy -D warnings、build、workspace fmt、git diff --check 全部 PASS。高基数性能数值来自 Agent 最终真实日志，Host 独立 traffic 复跑再次确认测试成功，不能冒充生产流量。

没有发现新增可操作代码 Finding。当前 P3-01/Runtime 接口只提供摘要，缺稳定连接ID、每连接累计上/下行字节及有证据的 node/host/client/direct 实例投影；P3-04 的 Sample seam 尚无正式运行消费者。因此本次只提交 **ACCEPTANCE 检查点**，P3-04 保留 owner、不能标 DONE、不能解锁 P3-05/P3-06，**不合并 codex/dist-react-restore、不 push**。已有 Core 两项基线失败及 Desktop 全量测试偶发 Store Busy 不在本次修复范围。随后由相关 Observation/Runtime owner 协调补最小生产记录来源，完成真实 WS→SQLite 端到端验收后再 DONE→合并。
