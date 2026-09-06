---
id: TASK-009
milestone_ref: M6
dependencies: [TASK-008]
risk: HIGH
status: DONE
requirement_ref: docs/veyra.md
requirement_identity: sha256:ddd1d20999348e6a1f8f2091539e27bc6729597f59e35357ed2cb2c7c29cad74
frozen_design_identity: sha256:1331f0bd5a7a470ea12c5bd776a5bf4224ab80c82b4b203a180b280048a9788a
design_review_ref: .sdlc/evidence/TASK-009/ui-boundary-design-review-028.yaml
design_refs:
  - docs/decisions/DCR-017-ui-integration-boundary.md
  - .sdlc/design/DCR-001-sing-box-1.14.0.md
  - .sdlc/design/DCR-003-fixed-core-configuration.md
  - .sdlc/design/DCR-004-runtime-update-failures.md
  - .sdlc/design/DCR-005-traffic-observation.md
  - .sdlc/design/DCR-006-memory-traffic-trend.md
  - .sdlc/design/DCR-007-dual-traffic-windows.md
approval_refs:
  - .sdlc/evidence/TASK-009/change-control-017.json
  - .sdlc/evidence/TASK-009/change-control-016.json
  - .sdlc/evidence/TASK-009/change-control-015.json
  - .sdlc/evidence/TASK-009/change-control-014.yaml#EVIDENCE-TASK-009-CHANGE-014
  - .sdlc/evidence/TASK-008/acceptance.yaml#EVIDENCE-TASK-008-ACCEPTANCE-001
---

# TASK-009：受管运行与现有首页集成

## 本次需求

把 TASK-008 已验收的最终配置接入 Veyra 的受管生命周期和现有首页，完成 Start/Stop、
失败反馈、固定鉴权观测与流量展示的一条最小真实应用链路。验证 Veyra 自有配置、安全、
资源和 UI 契约；sing-box 的协议、地址族、DNS/TLS 算法和包级行为不再作为完成前置。

## Scope

### allow

- 直接引用用户现有 `src-tauri/icons/128x128.png` 作为首页左上角 Logo；保留原始资产字节，不复制或修改图标集。

- `src/assets/fonts/Twemoji.Mozilla.ttf`、`public/licenses/twemoji/*`：用户CHANGE016明确要求的参考前端字体资产与来源/许可；在现有 `src/styles.css` 声明加载并保持控件字体继承，不安装系统字体或引入包依赖。

- `src-tauri/src/application/{runtime.rs,managed_observation_runtime.rs,observability.rs}`、
  `src-tauri/src/{commands.rs,singbox/{runtime.rs,managed_sidecar.rs,clash_api.rs,compiler.rs,mod.rs}}`：
  仅做完成三个 SF 必需的局部修复和定向测试。
- `src-tauri/src/platform/windows/{managed_sidecar_port.rs,private_runtime.rs}`：仅处理固定资产、
  自有 child、私有配置、ACL、退出确认和清理的 Veyra 责任。
- `src/{App.tsx,styles.css,lib/{observability.ts,observability.test.ts,traffic-trend.ts,traffic-trend.test.ts}}`：
  仅维护现有首页、Start/Stop、Toast、双时间窗流量与严格 DTO 解析。
- `src-tauri/tests/task009_managed_runtime.rs` 及上述文件内测试；实际产生的新 Evidence 保存到
  `.sdlc/evidence/TASK-009/`，不得写入 secret、完整私有配置或原始敏感载荷。

### deny

- 不修改 `scripts/task009-wg-peer/**` 或新增 peer、网络栈、WG/DNS/IPv6/Host/SNI、checksum、
  TCP ACK/重传验证设施；旧测试和 PASS/FAIL/NOT_RUN 证据保留原字节与身份。
- 不修改 Parser、订阅/Provider、数据库/schema/migration、公共 IPC、依赖或锁文件；不新增
  listener、任意配置/path/port 输入、热更新、自动重试、旧配置自动恢复或 fallback。
- 不改变固定 `127.0.0.1:9090`、每实例 secret、私有 ACL、无 reparse point、最终字节 check、
  child 所有权和脱敏规则；不操作 System Proxy、TUN、UAC、WFP、Service 或主机网络配置。
- 导航、原生 WebView/Tray 联合验收属于 TASK-010；订阅流程属于 TASK-011；不提前实现后续页面。

## SF-001：配置事务与受管生命周期

**需求：** 经完整 AppState 校验、Compile、实例绑定、结构 allowlist、私有文件和固定核心
`check` 后，只运行同一最终字节。旧实例仅在候选准备完成后停止；旧 child 退出确认后才消费
pending。新 child Process/API Ready 后才提交 active。compile/finalize/check/prepare 失败不触碰
旧运行态；spawn/Ready 失败停止并清理候选，不自动恢复旧配置。

**验收：** Start/重复 Start/Stop、冷启动、child 崩溃和后续清理不产生双实例；只管理已证明
归属的 child，不终止端口占用者。每实例 secret 不复用，运行字节与已 check 哈希相同。
退出和清理确认后才为 Stopped；Stop、check 子进程退出或清理无法确认时保留资源所有权并进入
RecoveryRequired，后续 Start 封闭失败直到手动 Stop 成功。既有超时保持 DCR-001 的有界值。

**Verification：** 复用既有定向 Mock 失败矩阵和 Windows adapter 证据；仅对受影响路径补跑。
新鲜最小链路须记录最终配置哈希、PID/child 身份、Ready、停止确认、私有文件清理和 secret
失效，不记录 secret 值。至少选择并回读现有生命周期测试的非零执行数，实际命令与超时在运行前冻结。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## SF-002：现有首页响应、Toast 与观测展示

**Logo要求（CHANGE017）：** 用用户图标替换左上角字母占位；保持32px正方形、原图比例和颜色，浅深主题及窄屏可见，构建包含资产。品牌名已由相邻文字提供，图片避免重复朗读。

**字体要求（CHANGE016）：** 加载与参考前端字节一致的Twemoji内置字体；保留系统正文栈和控件继承。
验证构建产物包含相同字节字体、浏览器实际请求/解码及代表性Emoji/旗帜使用该字体，浅深主题保持可读。

**视觉要求（USER:lifei CHANGE015）：** 当前首页沿用 Clash Verge Rev 的浅色/深色主题，
背景、文字、边框、按钮状态、错误提示和图表颜色对应参考源码，不自造色系。按系统颜色偏好
切换，并用真实浏览器回读两种主题的计算样式及桌面/320px截图。此处仅校正现有首页CSS；
持久化主题设置、导航与原生窗口主题接线留给对应后续Task，不引入依赖或公共IPC。

**需求：** controller 串行持有 child 与 observation bridge，只采样当前已鉴权 Ready 实例；
停止、失败或身份切换先失效旧采样，迟到结果不得进入 Snapshot/Delta。保留容量一队列、Busy、
零参数 Start 和重复 Start 幂等。ConfigurationFailed、StartFailed、StateUnavailable 与 Stop/
RecoveryRequired 使用固定脱敏响应，不透传原始错误。

**验收：** 现有 Start/Stop 每次操作至多一条 Toast，可关闭并自动消失；配置失败明确“未应用
新配置”并保留健康旧实例显示，启动失败、停止完成和“停止未完成”不混淆。凭证、UUID、私钥、
API secret、原始日志和连接目标不进入错误、事件或持久状态。连接/流量/日志安全 DTO 只来自当前
实例；无效鉴权、超时、child/worker 失败及迟到结果封闭处理。

实时网速与本次内核累计分开显示；首页最近 10 分钟与侧栏最近 60 秒共用最多 600 点的单调时间
内存窗口。空态、零值、超过 5 秒断线、淘汰、320px 重排、Stop/新 Ready/recovery 清空和配置失败
保留旧健康历史均正确；隐藏不造点、不丢历史、不增加 IPC，不写入磁盘。

**Verification：** 复用既有 TS 契约、React browser mockIPC、桌面/320px 截图和趋势定向测试；
若实现字节未受影响，不要求全量重跑。对本次最终 UI 身份补足受影响交互证据，并与 SF-003 当前
实例的真实 API/代表性流量摘要对账。原生窗口、导航和 Tray 恢复留给 TASK-010。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## SF-003：最小真实内核集成与应用安全边界

**需求：** 从隔离且整体校验的 AppState 生成最终配置，以固定 Windows amd64 sing-box 1.14.0
完成 `check → run → Process/API Ready → 固定鉴权 API 读回 → stop`。需要流量时只复用一个已有
受控阳性计量场景。Veyra 生成的 endpoint TCP/UDP/DNS 入站及 Router 转发拒绝规则须有结构、
顺序、用户规则不可覆盖的正反断言，并复用已有代表性接线证据。

**验收：** 同一最终字节、child、secret 和观测实例贯穿链路；固定 API 仅 loopback 且鉴权，
私有配置/ACL/清理和脱敏规则成立。代表性连通只证明 Veyra 接线和观测，不扩张为 sing-box
协议认证。HTTP025 DnsError 与 reject025 socket-query timeout 保留历史 FAIL；先按已有摘要检查
是否指向 Veyra 配置、进程或资源路径，有具体产品缺陷才做最小修复，不宣称根因已修。

**Verification：** 优先复用 `fixed_bundle_resources_run_and_stop_only_the_owned_loopback_child`、
`real_loopback_metering_tracks_bytes_silence_and_new_instance` 及拒绝规则 Compiler 断言的已有证据；
运行前回读精确测试名、资源归属、非零执行数与 60 秒外层超时。只补当前身份缺失的最小真实链路，
不重跑或补齐旧 WG/DNS/IPv6/Host/SNI 矩阵。

**implementation_status：** IMPLEMENTED
**acceptance_status：** PASSED

## Evidence 对账与新鲜度

- TASK-006 的实现/独立审阅支持固定 sidecar、私有运行目录、鉴权 API 与脱敏观测基础。
- `verification.yaml` 支持生命周期失败点、TS 响应和 React mockIPC Toast；`verification-005.yaml`、
  `verification-006.yaml` 支持双时间窗、截图和 browser Mock。它们不是 native GUI/Tray 证据。
- checkpoint025 的源/用例结论保留原身份；`verification-025.yaml` 的 aggregate 为 FAIL，旧整 Task
  Delivery Review 对本验收为 STALE。不得据此通过新 Task，也不要求把全部旧网络用例重跑。
- 下一项缺口是：按当前源身份选择最小真实 application 链路和受影响 UI 回归，生成新鲜 Evidence，
  冻结 Delivery Unit 后交独立 Reviewer；Task 与三个 SF 的人工验收仍为 PENDING。

## Dependencies、Risk 与验证命令

- TASK-008 已 DONE；当前 Task 完成前不得开始 TASK-010。主要风险是候选/active、child/实例、
  迟到观测和用户反馈混淆，以及错误复用旧 Evidence 身份。
- 命令来源为原 Task、`package.json` 与 `.sdlc/design/foundation.md`：按实际影响选择
  `cargo test --manifest-path src-tauri/Cargo.toml --lib <exact-filter> -- --nocapture --test-threads=1`、
  `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`、
  `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`、`pnpm test`、`pnpm lint`、`pnpm build`、
  `git diff --check`。真实 child 单例默认外层 60 秒；运行前必须冻结过滤器并确认非零执行。

## Task 独立验收

在同一 Veyra 应用事务中完成最终配置 check、受管 child Ready、固定鉴权的当前实例观测和
Stop/清理；流量观测复用一个已有代表性阳性场景。应用生成安全规则的结构、优先级正反断言与
已有代表性接线证据独立对账，不要求新建 peer 或协议矩阵。现有首页正确显示状态、Toast、网速、
累计和双时间窗；失败时不混淆旧/新配置、实例或迟到结果，且敏感数据、私有资源与平台边界保持。
三个 SF 分别验收后，仍须新身份的 Task 整体 Evidence、独立 Delivery Review 与 USER:lifei 明确验收。

**acceptance_status：** PASSED

验收记录：USER:lifei 明确确认，见 .sdlc/evidence/TASK-009/acceptance-001.json。状态更新不改变已验收实现身份。
