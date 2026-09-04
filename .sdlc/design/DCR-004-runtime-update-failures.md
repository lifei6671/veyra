---
id: DCR-004
status: FROZEN
change_source: USER:lifei 2026-09-04 explicit three-stage update failure behavior
approval_ref: .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-001
scope_extension_ref: .sdlc/evidence/TASK-009/change-control.yaml#EVIDENCE-TASK-009-CHANGE-002
affected_design:
  - .sdlc/design/foundation.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
  - .sdlc/design/DCR-002-full-sing-box-subscription-and-compiler.md
affected_task: [TASK-009, TASK-011]
---

# DCR-004：更新失败反馈与内核启动失败停止

## 已确认的产品行为

用户明确要求区分订阅更新、编译执行配置、启动内核三类操作。失败仅记录脱敏日志与 Toast，
用户手动重试；启动内核失败后停止服务，不自动恢复旧配置，避免把旧配置运行误认为最新配置已生效。
此次用户指令已授权本节产品语义和为实现其必需的最小契约调整；无需再次请求相同产品决策。

| 操作失败 | 保留内容与运行态 | 反馈与后续 |
| --- | --- | --- |
| 订阅获取/解析/校验/保存 | 旧有效订阅及最后成功更新时间；当前运行态不变 | 脱敏日志、一次 Toast；手动更新 |
| Compile/finalize/allowlist/check | 已运行的旧 child 保持；失败候选不得变为 active | 脱敏日志、配置生成失败且新配置未应用 Toast；不自动重试 |
| 候选 spawn/Ready | 清理失败候选，服务停止；不启动 previous | 脱敏日志、内核启动失败 Toast，展示实际停止状态；等待手动操作 |

只有确认退出及私有配置清理成功才能显示 Stopped；未确认退出/清理失败时保留所有权与
RecoveryRequired，提示停止未完成。必要清理不属于配置回滚，不触发自动重启或额外业务操作。
订阅成功、编译成功、核心 Ready 是不同事实；只有 Ready 才代表配置已生效。

## 内部事务与最小实现

1. 应用从整体校验状态编译 Plan，每次新的手动启动尝试生成新 secret 并 finalize 为
   GeneratedConfig。维持现有 Compiler/应用绑定责任，不为恢复引入 Plan 缓存或新的运行时抽象。
2. Windows Port 创建私有配置，校验 ACL、allowlist、字节 readback、固定核心 check，保留唯一
   已检查 pending。候选准备失败不停止旧 child。准备成功后，才停止旧 child 并确认退出，然后 run
   一次消费对应 pending；run 前核对文件仍是同一已检查最终字节，check 后不改写。
3. 新 child 通过单次 Ready 后才提交 active 状态。候选 spawn 失败或 Ready 失败，停止并清理
   候选后结束本次事务，清除候选/失效 active 状态，不调用 restore_previous、重新 check 旧配置、
   重启旧 child 或 fallback 到 direct。已有 previous 槽不能再成为自动启动来源；删除仅为自动
   回滚存在的生产分支及无用状态，不能把历史已检查配置误作当前 active。
4. 旧 child 未确认停止时不启动候选；保留旧 child 所有权，清理尚未运行候选，并进入
   RecoveryRequired。候选或 check 子进程未确认退出、私有目录删除失败时，同一 owner 保留
   待停止/清理资源；后续启动封闭失败，直到手动 Stop 成功清理。正常 Stop 无界等待和扫描/终止
   无关 PID 均禁止。check 子进程超时后的所有权同样不能被临时值 Drop 隐藏。
5. 已检查 GeneratedConfig 只代表这次尝试，废弃/停止后擦除其 API secret 和无用字节副本，
   不为了潜在回滚保留旧密钥。正常运行后的 child 崩溃也只做既有归属清理并反馈停止/失败，不自动重启。
6. 串行 controller 和采样保持不变。停止/替换先使旧 identity 不可采样；未 Ready 的候选不能
   采样，失败后不提交旧 Ready DTO、不保留旧非零流量作为当前流量。只清理确定归属的资源。

内部 SidecarPort 的 check/prepare/run 允许为保持已检查候选的一次消费作最小适配；不为本需求
合并框架、增加通用队列、持久恢复日志、后台重试或 API 重载机制。DCR-001 的精确 ACL、固定资源、
固定 127.0.0.1:9090、每实例新 secret、check 10s/退出确认2s、单次Ready 2s、Stop 2s 期限保持。

## 日志、Toast 与用户状态

- 现有零参数 Start 在 Ready 时仍返回 AlreadyRunning，不隐式替换。当前 TASK-009 只验证内部
  replacement，不新增 Apply/Reload IPC。新手动启动从当前有效状态重新编译，不从 previous 启动。
- Start 在既有封闭响应中增加 `ConfigurationFailed`（JSON `configurationFailed`），用于
  Compile/finalize/allowlist/check 等执行配置生成失败；`StateUnavailable` 继续表示输入状态无效，
  `StartFailed` 表示进程准备后启动/Ready或清理等失败。请求参数、能力和其它响应语义不变。
- 后端失败日志复用既有 InMemoryRuntimeObservations 安全日志摘要。阶段用固定文案/封闭类别，
  不携带订阅URL、凭证、路径、原始核心输出、网络payload或secret。配置生成失败不能把仍健康的
  原 child 改成失败停止；启动失败成功清理后记录 Stopped + Error 摘要，清理未完成则记录 recovery。
- `src/App.tsx` 使用现有状态渲染一个可关闭、自动消失的 Toast，不增加第三方依赖。一次用户
  操作最多一条失败Toast；后台采样失败只更新状态/日志，不重复弹窗。失败Toast与持续运行状态
  同时显示，Toast消失不将状态变成成功。Snapshot请求失败不能把已经失败的操作覆盖成成功。
- 文案：ConfigurationFailed 为“配置生成失败，未应用新配置”；StartFailed 为“内核启动失败，
  请查看运行状态”。只有后端已确认Stopped才显示“服务已停止”；recovery显示“停止未完成”。
  Start失败时没有准确快照也不猜测服务已停止。输入状态无效与Busy按现有语义提示，错误字符串
  不直接进入Toast。

订阅页当前尚未实现。TASK-011 交付订阅手动更新、失败日志/Toast、最后成功更新时间；失败不
刷新成功时间、不清空旧节点，成功时间只有获取/解析/整体校验/保存均成功后才更新。条件请求
304在已存在有效缓存且完成有效更新流程时算一次成功检查；页面文案不得承诺内容有变化。
订阅成功不等于运行配置生效，不能用成功Toast或时间误导用户。此处不提前更改持久化schema。

## 文件范围与验证

TASK-009 生产范围为 `src-tauri/src/singbox/{runtime.rs,managed_sidecar.rs,compiler.rs,mod.rs}`、
`src-tauri/src/platform/windows/managed_sidecar_port.rs`、
`src-tauri/src/application/{managed_observation_runtime.rs,runtime.rs,observability.rs}`，以及
`src/App.tsx`、`src/styles.css`、`src/lib/observability.ts` 的上述失败反馈及对应测试。
`application/runtime.rs` 仅适配内部失败状态并保留SystemProxy的Mock补偿断言；不运行或修改
平台系统代理实现。不改变订阅Parser/Domain/Storage、协议映射、DCR-003控制、Core资源或依赖。

CHANGE-002 已授权将 `platform/windows/private_runtime.rs` 与 `singbox/clash_api.rs` 纳入
有界修复及内测：目录创建后准备、删除同时失败时把待清理对象交还 Port；固定鉴权 Logs 的
HTTP 204，或实测 HTTP 200 且显式 Content-Length: 0、无 Transfer-Encoding、响应体为空，
作为正常空摘要。原权限策略保持，其他 HTTP 200、Traffic 200/204、401、异常与握手超时仍拒绝。
这落实既有归属与正常空日志要求，不扩展流量计算、网络拓扑或公开接口。

- 真实固定核心覆盖冷启动/Ready/停止、成功内部替换；构建/check失败证明原child identity
  不变；候选spawn/Ready失败证明失败已反馈且没有自动启动旧配置，清理后无活跃child。
- Mock与测试私有故障注入覆盖每个失败点、退出确认/清理失败、check子进程超时、并发与迟到
  观测，断言恰当日志类别、配置槽/句柄归属、无自动恢复/重试及无“最新配置已生效”假状态。
- UI验证新封闭响应映射、失败Toast与停止/recovery状态并存、成功/重复Start不误报、原始错误
  不外泄；使用现有pnpm lint/test/build，不能把仅解析测试当作Toast实际交互证据。
- DCR-003要求的WireGuard ICMP例外、TCP/UDP/DNS/转发拒绝、出站应答、无系统配置变化和
  DNS/URLTest真实行为继续属于TASK-009，未执行不记PASS。先记录受控拓扑再运行，不授权新
  依赖、非loopback暴露、特权或外部成本；缺少现成harness本身不是阻塞。

## 影响与审查

本设计落实已确认用户需求，替换“启动失败恢复旧配置”的运行语义；不改变状态文件损坏恢复、
数据库迁移备份、SystemProxy安全补偿或Core二进制版本回退等其它独立机制。后者不能成为配置
启动失败自动回滚的旁路。TASK-008编译/check交付仍有历史证据，本次变更不把它升级为真实运行证明。

产品语义已由 CHANGE-001 的用户指令确认，独立技术审查通过后按同一已授权语义冻结。
Task 同步后重新检查 Readiness，无需再次要求用户批准同一决定；实质扩大安全/依赖/产品契约
仍需对新增部分另行处理。业务实现与真实运行尚未执行。
