---
id: DCR-004
status: PROPOSED
change_source: TASK-009 readiness inspection of fixed-port replacement and instance-secret ownership
affected_design:
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
affected_task:
  - TASK-009
---

# DCR-004：固定端口替换与重新绑定的配置回滚

## 来源与已证实问题

需求 `docs/veyra.md` §76 保持 Config Build / Apply 分离；Foundation 的受管 Runtime 要求
Health Check 失败回滚上一份配置。DCR-002 Verification 写成“编译/check/start 失败保持当前实例”，
但唯一 `127.0.0.1:9090` 不能由两个普通 child 同时持有；现有 Runtime 实际先停旧 child 再启动候选。
因此 start/Ready 失败后的恢复只能是上一份配置的新实例，不能承诺原进程不断线。

具体调用证据（本轮只读，未运行 child）：

- `src-tauri/src/singbox/runtime.rs:163` 的 `start_or_replace` 在 check/prepare 后停止旧 child，
  再 run/Ready；`:300` 的 `restore_previous` 仅 prepare(active) → run → Ready。
- `src-tauri/src/platform/windows/managed_sidecar_port.rs:132` 的 prepare 忽略入参，只看 pending；
  `:137` 的 run 消耗 pending，`:181` 的 stop 删除旧 config 与实例 secret。
  候选已经 run 后，旧配置回滚没有 pending；Mock 的 prepare 成功不能证明 Windows 恢复成功。
- DCR-001 要求每个受管实例独立生成 secret、不复用；Runtime 当前 ConfigSlots 保存的
  GeneratedConfig 已包含旧 secret，原字节直接用于新 child 不能满足该契约。

## 拟批准的运行语义

1. 编译、绑定、私有配置准备或固定核心 check 失败：不停止当前 Ready child、不改变其配置、
   identity 或观测归属。候选必须清理；清理失败如实失败并保留清理归属，禁止后续替换，直至成功清理。
2. 候选通过最终字节 check 后，停止并确认旧 child 退出，再启动候选。这一过程允许短暂中断，
   不承诺无损连接迁移或保持同一 PID。旧 child 停止未确认时不得启动候选；保持旧 child 所有权，
   进入 RecoveryRequired，不把未确认状态报告为 Ready。
3. 候选 start/Ready 失败：先停止并确认候选退出、清理其私有配置和 API secret，再从上一份
   已验证的语义 Plan 创建一次新的恢复尝试；新 secret、新私有 config、完整 allowlist、固定核心
   check、同一最终字节 run、单次 Ready 均必须重新执行。恢复成功后返回候选失败，同时将实际
   运行态标为已恢复 Ready；配置语义与默认 Pool 回到旧值，child identity 和 API secret 为新值。
4. 恢复 check/start/Ready 或清理任一步失败，进入 RecoveryRequired；不循环重试，不启动第三份
   替代配置，不猜测 direct 默认出口。尚未确认退出的 child 与待清理资源必须仍由同一 owner 持有，
   仅成功 Stop/清理后才能解除 recovery。没有旧 Plan 的首次启动失败不得伪造恢复成功。
5. 正常 Stop 也必须确认退出并完成私有配置清理后才成功。恢复 child 的 Ready 失败后若 Stop 失败，
   仍保留该 child 的归属，不能因回滚函数返回错误而丢失句柄。

## 最小内部实现与敏感数据生命周期

- 复用已有 `SingBoxPlan`（内部 Document 尚未绑定 API secret）作为 Runtime 的 candidate/active/
  previous 语义槽；应用调用方提交 Plan。槽的晋升只在候选 Ready 后提交，失败保留此前语义槽。
  Plan 仍含节点凭证，只在后端有界内存中存活且 Debug 脱敏，不写入新的持久状态。
- Windows Port 独占每次尝试的绑定与准备：生成 secret → Plan.finalize → 严格结构校验 → 私有
  ACL config 写入/readback → 固定 check → readback。把现有无独立行为的 check/prepare 合并为
  一次内部“准备已检查候选”操作；成功只保留一个 pending，run 消费它一次。取消候选有显式清理路径，
  清理失败保留资源归属。接口仅 pub(crate)，不新增通用句柄系统、回调队列或公开能力。
- `GeneratedConfig` 仍表示一次尝试的不可变最终字节。恢复不是修改通过 check 的旧文件，而是
  由旧 Plan 产生新的独立候选并完整重走检查。准备结束后丢弃并擦除无用途的最终字节副本；停止或
  放弃实例后不在 ConfigSlots 中留存该实例 API secret。新旧 API 身份不可交叉用于 Ready/采样。
- 每次 run 前必须确认被启动文件仍是该 pending 已检查的最终字节；不通过改写文件、绑定新端口、
  核心热重载或复用旧 secret 绕过检查。现有当前用户精确 ACL/reparse 拒绝契约保持不变；本方案不
  声称防御具有同一当前用户权限的任意恶意进程并发篡改，后者不在既有信任边界之外。
- Runtime/Port 的操作继续串行；replacement 期间不采样旧 identity，未 Ready 的候选不得采样；
  完成后只对新的 active identity 发起读取。失败状态与日志仍使用既有封闭、脱敏 DTO。

允许修改的生产范围限于 `src-tauri/src/singbox/{runtime.rs,managed_sidecar.rs,compiler.rs,mod.rs}`、
`src-tauri/src/platform/windows/managed_sidecar_port.rs` 与
`src-tauri/src/application/{managed_observation_runtime.rs,runtime.rs}`。
Compiler 仅调整 Plan/绑定使用所需内部支持，不改变 TASK-008 协议映射、DNS、WireGuard 控制、
allowlist 或公开 ConfigCompiler 契约。application/runtime 只适配内部 Plan 传递及其 Mock 测试，
不运行或修改 System Proxy 事务。

## 产品入口与时间边界

- 现有零参数 Start 保持幂等：Ready 时返回 AlreadyRunning，不触发编译、check 或 replacement。
  TASK-009 的 replacement 只通过内部 Runtime 集成验证；不新增 UI/IPC 编辑或替换入口。
- 既有 check 10s（超时 kill 后最多 2s）、Stop 2s、Ready 单次 2s 的步骤预算不变，不能在
  Ready 内增加重试。内部 replacement 最多两次准备/check（候选与一次恢复），各阶段按上述期限
  串行执行；测试外层预算 60s，期限到达后只清理本测试持有的 child。该内部流程不进入现有 15s
  Start IPC 等待路径；若后续产品需要暴露 replacement，须另行冻结响应预算与产品语义。
- 固定 API、无 System Proxy/TUN/UAC/WFP/Service、无新增产品 listener、无依赖/lockfile/持久化
  变更。受控测试服务与 WireGuard peer 的具体隔离拓扑须在真实运行前记录；新权限、非 loopback
  暴露、第三方依赖或外部资源成本不在本修订授权内。

## 验证与批准边界

- 用真实固定核心、隔离私有 config 和同一 Runtime 覆盖初次 start/Ready/stop、成功替换、编译/check
  失败保留原 identity、候选 spawn/Ready 失败后重新 check 并恢复旧语义的新 identity/new secret。
  失败注入仅为测试私有适配层，不新增运行期 flag；明确标识注入点与真实执行步骤。
- 定向 Mock/故障注入覆盖旧 child stop 失败、候选 cleanup/stop 失败、恢复 check/start/Ready/stop
  失败，断言 recovery、句柄/资源归属、无继续启动、无旧观测提交。不可故意制造无法清理的宿主进程。
- 记录每次尝试的最终字节一致性与核心身份，secret 仅在测试内比较、不输出其值；正常/失败路径均验证
  私有文件清理、旧 API secret 不复用、只有受管 child 的固定 TCP listener，未知 PID 不受影响。
- DCR-003 要求的真实受控 WireGuard ICMP、TCP/UDP/DNS 拒绝、转发拒绝、出站应答、无系统接口/
  路由变化，以及真实 DNS/URLTest 行为仍是 TASK-009 必需验收，不由本回滚修订或 TASK-008 check 替代。
  测试 harness 未实现不记 PASS；实施中须先形成受控拓扑和可观测断言，再执行相应用例。

## 影响分析、替代方案与决定

- 仅提议修订 DCR-002 的 start 失败“保持当前实例”为“恢复上一已验证配置的新实例，允许中断”；
  编译/check 失败保留原实例的承诺不变。DCR-001 每实例独立 secret 与固定 API 的约束不降低。
- 影响内部 Runtime/Port/调用方契约及测试；不迁移数据、不加后台线程、资源包、驱动或公开 API。
  原来的 Mock 回滚证据不能当作本真实事务的验证；TASK-009 Design/Readiness/Delivery 必须重新取得。
  TASK-008 已验收的编译/check 范围保持历史有效；后续接口适配仍须重验相关编译/check回归。
- 备选：双进程并行探测需要不同端口或额外路由，违反当前固定 listener 边界；核心热重载需要新
  API 能力和事务契约；直接复用旧文件/secret 违反实例隔离。这些方案均不采用。
- 代价：启动失败时存在可观察中断，恢复还可能失败；最多一次恢复尝试使复杂度有界。采用已有 Plan
  和串行 Port 可消除真实 pending 缺口，不为此创建新的持久恢复系统或进程管理服务。
- 当前为 PROPOSED。独立技术审查通过后，须由用户明确批准上述运行语义及内部 Scope，才能同步
  DCR-002/Task 契约、通过 TASK-009 Readiness 并开始实现。此次文档与审查不授权真实运行。
