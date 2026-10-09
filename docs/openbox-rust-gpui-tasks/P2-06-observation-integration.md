# P2-06 / P3-01 正式 Runtime 观测接线（2026-10-09）

P2-06 原 Runtime/Platform owner 在组合树 `dev/p2-06-observation-integration` 完成此增量，输入 HEAD `b20832094e72d13f0c15085cb8376ec886c2b4cb`。最终独立 Review 通过；本组合分支 P3-01 **DONE**，Observation owner/本卡预约释放；P2-06 **DOING**及原 Runtime owner 保留。不启动下游，不修改或合并主工作树；本轮在此分支独立本地提交，不push。

## 正式通路

`Desktop ManualSidecarPort / helper ProcessPort → SidecarPort::observation_endpoint → ManualRuntime::mark_ready → ObservationService::bind`。

- Port 只在当前 owned identity 匹配时返回其已鉴权 `Arc<ManagedControllerEndpoint>`，复用同一撤销状态，不从端口号重建来源。
- 每个 ManualRuntime 持有唯一 ObservationService，同步 owner 懒建一个单线程 Tokio 执行器运行已有四路 collector；没有第二份 Runtime 状态、DTO 或进程管理器。执行器创建失败保留未知观测并输出固定诊断，不把真实代理 Ready 改成伪数据。
- check/run/鉴权 Ready/选择核对成功后绑定现有 InstanceId；重复 Start/Refresh 不重绑，候选 check 失败保留当前来源。实际替换 writer 前、Stop 前、失联/Recovering 和 owner Drop 时停止服务；handoff 沿用已有 Stop。回退成功获得新的实际 instance/source。
- `ManualRuntime::observation()` 只给只读 snapshot/subscribe/observed。页面接入与 helper 观测 IPC 投影不在此次范围，不声称 UI 已显示新数据。
- P3-01 collector、归一化/限额/重连算法及公共 Runtime DTO 无修改；系统代理/TUN/bootstrap/rebind 无改动。

## 可复核验证

全部 Cargo offline，`MACOSX_DEPLOYMENT_TARGET=15.0`，每条命令由 Python 600s timeout 约束；原命令/exit/stdout/stderr 在 [本轮证据](evidence/p2-06/observation-integration/README.md)。

| 项目 | 实际结果 |
| --- | --- |
| 新 owner 定向测试 | 2 PASS：保留旧来源/换实例/意外退出；Stop 失败清来源 |
| 锁定真实内核正式 Runtime 隔离测试 | 1 PASS：四 WS、loopback HTTP、三指标断流未知/重连、Restart 换源、Stop/reap/端口关闭 |
| Core 串行全量 | 413 PASS / 2 FAIL / 2 内部 fixture ignored；两 FAIL 在输入 HEAD 独立 target 重建也复现（411 PASS / 同2 FAIL / 2 ignored） |
| Desktop 串行全量 | 116 PASS / 13 ignored（真实观测用例另显式运行，其他 Native 不执行） |
| Helper production 串行全量 | 70 PASS / 4 内部 fixture ignored |
| P0-05 prototype | 47 PASS |
| 三包 all-targets check / Clippy -D warnings | PASS（helper production + prototype features） |
| Desktop/production-helper build、workspace fmt --check、diff检查 | PASS |

Core 既有失败：`outbound_query::query_reads_current_snapshot_without_changing_schema_or_state_bytes`、`state::rejects_invalid_pool_and_route_references`。未修改/跳过它们。初轮 Core/Desktop/Helper 并行测试另有 Busy/版本/fixture 竞争 FAIL；串行候选除上述 Core 两项全部通过，独立输入 HEAD 的 helper 并行也实际失败（63 PASS / 7 FAIL / 4 ignored）。这些失败保留，不能称默认并行全量 PASS。

锁定复制品 `veyra-sing-box`：1.14.0 darwin/arm64，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。只读取已知本机源并复制到本轮证据目录；未下载、签名或改变上游字节。真实用例所有 socket/root/child 均为自己创建；请求只访问自己的 127.0.0.1 HTTP listener。没有系统代理/TUN/现有 app/root/缓存操作。

真实四路初始 available 全 true、sequence `[2,2,3,3]`、活动 connection=1、sourceIP=127.0.0.1、memory=5586944、session bytes=(73,32810)。短时 SIGSTOP 只作用本测试 Child，并以 Drop guard SIGCONT；恢复后三指标 stream_generation 从1到2、同 InstanceId、累计不补计缺口。日志正常沉默不超时，generation仍1；**未证明真实日志单路断线重连**，该分支仍由 P3-01 固定 WS fixture 覆盖。Restart 新实例、Stop 清空 clients/logs/identity，旧任务不再发布。

## 实现者自查与边界

已按 code-delivery-review 自查调用链、候选失败保旧、未知清源、endpoint 撤销、Drop/同步线程执行器及敏感字段。未发现本增量剩余可操作代码缺陷；这不是 Host 独立 Review。建议 Host 核对：唯一服务归属、真正 stop_old_writer 前停止、Ready bind 的实例来自当前 sidecar、两个生产 Port 共享同一端点撤销状态，以及真实日志未重连的证据范围。

真实 root/helper 安装/降权/GUI：**NOT_RUN**；此次真实内核由普通用户正式 ManualRuntime 启动，不能替代 P2-06 Native 验收。后续由 Codex Desktop 在专门授权的隔离现场复验 helper Ready/Stop/handoff 的四路来源与清理，先核对安装/child身份；不得据此自动批准系统代理写入或开启 P2-07。

源码 aggregate（5个受影响 Rust 文件，排序 SHA256 行再次 SHA256）：`fe80b842855d40865d60b9ca5a18c7a5c9d8b34c0e338b93b3761cfb2c32ff02`。证据保留逐文件 SHA、增量 diff 和构建身份。

## Host 最终收口

独立 Reviewer 只读最终代码、源码身份和实际日志，未发现剩余可操作 Finding；[完整审查](evidence/p2-06/observation-integration/host-review.md)。Host 核对本卡四项服务验收及真实受管实例接线，P3-01 ACCEPTANCE → DONE，仅释放该卡 Observation owner/预约。P2-06 接线增量 PASS，总任务仍 DOING；其安装/授权/降权等 Native 缺口不变。未以 fixture 代替真实内核、也未以普通用户 ManualRuntime 代替 root Helper 验收。

组合分支 68 卡：DONE25 / DOING1 / READY5 / TODO30 / DEFERRED7。READY=P0-08、P2-05、P3-03、P3-04、P4-03，均未领取；原依赖不变。主开发分支仍为 ed174e6，不自动合并。本轮只在已验证代码上同步文档与状态，未重复运行无变化的代码测试。
