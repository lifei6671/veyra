# P1 核心库与桌面壳

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：目标是可独立启动、保存配置并通过托盘恢复的桌面壳；视觉与跨页面行为偏好最小拆分，不用占位页面冒充业务完成。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

<a id="obg-p1-01"></a>
## OBG-P1-01 抽取无 Tauri 依赖的核心库

**类型**：核心重构；**依赖**：OBG-P0-01、OBG-P0-02。**依据/范围**：方案 §4–5；`src-tauri/src/{application,domain,subscription,singbox,storage}/` → 新 `crates/veyra-core/` 与根 workspace。

**泳道 / 写范围**：Core/Config；根 Cargo workspace/锁文件、core 抽取与旧入口接线；workspace 单一 owner。

**执行与交付**：只移动本阶段需要的核心与有效测试；调度改用 Tokio runtime/Handle，抽离现有 SidecarPort/SystemProxyController；保留旧入口对共享核心的调用及 Windows 条件编译。

**验收**：

- [x] core 在 macOS 独立构建，依赖树不含 Tauri/GPUI，公共路径不引用具体 Windows 实现。
- [x] 相关纯单测实际执行且有测试数量；旧入口受影响的回归通过。
- [x] 平台装配留在入口，不新增通用 Executor/平台工厂；记录新 workspace 的真实验证命令。

**当前交付**：DONE；详见[抽取记录](#p1-01-delivery)。

<a id="obg-p1-02"></a>
## OBG-P1-02 类型化服务、原子快照与版本

**类型**：状态与持久化；**依赖**：OBG-P1-01。**依据/范围**：方案 §6、§12；core 的 domain/application/storage。

**泳道 / 写范围**：Core/Config；domain/application/storage 公共领域模型、共享 DTO、StateVersion/Preferences；单一 owner。

**执行与交付**：定义稳定 ID、ProfilePatch、ApplyEffect、AppError 和 RuntimeSnapshot；单文件原子业务快照、共享 state_epoch、配置/选择计数与整体替换入口。若桌面 update preference 属于版本化 AppConfig，其字段、默认值与迁移在此定义，不依赖 P6 Updater 行为先实现。

**验收**：

- [x] Missing/Null/Value 与数组替换行为明确；保存局部字段不丢失未编辑的 DNS/routing/TUN 字段。
- [x] 写入失败保留有效旧快照；整体替换后旧表单/任务不能跨 epoch 写回，append 正确推进当前版本。
- [x] 保存、当前运行、最后成功版本分别表达；没有运行实例时不凭历史记录显示 Ready。
- [x] 服务错误保留稳定码/字段位置/脱敏详情；内部类型不退回任意 JSON 命令。

**当前交付**：DONE；四项验收 PASS，详见[类型/快照记录](#p1-02-delivery)。

<a id="obg-p1-03"></a>
## OBG-P1-03 GPUI 壳层与异步状态桥

**类型**：UI；**依赖**：OBG-P1-02、OBG-P0-03。**依据/范围**：方案 §9.1–9.3；新 desktop `app.rs`、`ui/pages/`；参考 `OpenBoxApp.tsx`、`SettingsPage.tsx`。

**泳道 / 写范围**：GPUI；desktop app/navigation/state bridge；公共 DTO 变更交对应 owner。

**执行与交付**：建立六主页面/九分类导航和页面实体；AppServices 通道与 GPUI 更新桥；启动引导、错误/忙碌状态及草稿生命周期。

**验收**：

- [x] 无配置可进入本地界面；无需启动 OpenBox HTTP 服务或浏览器登录流程。
- [x] 页面切换保留适用的筛选/草稿；迟到回调、旧实例事件不污染新页面。
- [x] render/点击回调不阻塞等待网络，GPUI 实体不被错误传入 Send 任务。
- [x] 壳层与导航完成实际操作、浅深色及适用尺寸的 UI 验收；未实现业务明确标记。

**当前交付**：DONE；11 Cargo tests、最终原生 Entity 三组断言及真实六页/九分类/浅深色/异步/错误重试均通过。最终复核曾因 Mac 锁屏未运行，用户解锁后已完成；12 图及完整构建身份见 [P1-03 交付](evidence/p1-03/README.md)。

<a id="obg-p1-04a"></a>
## OBG-P1-04A 视觉桌面偏好、主题与基础组件

**类型**：UI 与偏好；**依赖**：OBG-P1-02、OBG-P1-03。**依据/范围**：方案 §9.3–9.4；`PanelSettings.tsx`、`openbox.css` → desktop theme/components/settings 与 core preferences/assets。

**泳道 / 写范围**：GPUI；theme/components、视觉偏好 UI/assets；Preferences DTO 交单一 owner。

**执行与交付**：建立输入、选择、弹层、通知、拖拽基础；迁移语言、主题、背景、透明度、模糊、圆角、侧栏折叠/展开（sidebar）及布局（layout）类纯视觉桌面偏好；打包许可明确的字体与图标。proxy columns、hide unavailable 等跨页面行为偏好由 OBG-P1-04B 负责，不并入纯视觉布局。

**验收**：

- [ ] 视觉偏好（含侧栏折叠/布局）保存并重启恢复；滑块合并保存、失败保留草稿，背景引用原子提交。
- [ ] Tab/Enter/Space/Escape、输入法、焦点归还正确；顶层弹层关闭不误关下层。
- [ ] 浅色/深色和局部覆盖可追溯到批准基线，字体/Emoji/背景差异有明确验收。

<a id="obg-p1-04b"></a>
## OBG-P1-04B 跨页面行为偏好与消费契约

**类型**：偏好与 UI；**依赖**：OBG-P1-02、OBG-P1-03。**依据/范围**：方案 §6、§9.4；core preferences、面板行为字段及各页面读取契约。

**泳道 / 写范围**：Core/Config + GPUI；行为 Preferences/消费通知与面板行为 UI；与 A 预约共享设置组件/DTO。

**执行与交付**：迁移测速站点/阈值、IP 信息源、proxy columns、hide unavailable、代理显示/排序默认值及 UI diagnostics `config/ipv6-test`；定义类型化读取与变更通知，供后续代理、概览、连接和诊断消费。共享 Preferences DTO 由单一 owner 修改，A/B 不同时写同一设置组件。

**验收**：

- [ ] 行为偏好可保存/恢复，非法输入可定位；消费者收到新值而不丢失未编辑字段。
- [ ] UI latency preference test URL、Runtime health test URL、group health URL 分别建模；group 的空值按现有明确规则使用 Runtime 全局地址，不回落到 UI 测速偏好。
- [ ] UI diagnostics ipv6-test 与 Runtime profile ipv6 分别保存/消费；修改诊断偏好不改变 Runtime IPv6 配置。
- [ ] 面板行为字段的实际操作与失败反馈通过；后续页面消费的真实集成分别在 P2-08、P3-02/05/07、P4-05C、P5-03 验收，不在本任务冒充完成。

<a id="obg-p1-05"></a>
## OBG-P1-05 平台目录、单实例与原生文件操作

**类型**：桌面适配；**依赖**：OBG-P1-02、OBG-P1-03。**依据/范围**：方案 §9.3、§12–13；core platform/macos、desktop 原生动作。

**泳道 / 写范围**：Runtime/Platform + GPUI；platform/macos 目录/锁/文件桥及原生动作。

**执行与交付**：用户数据/日志路径与受管文件权限、OS 文件锁；文件打开/保存/拖放、剪贴板和外部链接。新路线试运行使用独立目录。

**验收**：

- [ ] 第二次启动聚焦已有窗口，不能出现第二个业务 writer；占用无法确认时明确报告。
- [ ] 取消文件对话框无副作用；非法/超大输入在边界失败，含凭据文件权限符合方案。
- [ ] 不扫描浏览器 profile，不覆盖旧应用目录；文件复制与外部链接真实可操作。

<a id="obg-p1-06"></a>
## OBG-P1-06 托盘与窗口关闭流程

**类型**：原生 UI；**依赖**：OBG-P1-03、OBG-P1-05。**依据/范围**：方案 §9.5；desktop 装配/托盘/退出流程。

**泳道 / 写范围**：GPUI；托盘/窗口/退出装配；公共 Runtime 接口交 owner。

**执行与交付**：把 P0 托盘原型接入统一业务状态；实现显示、状态、启停入口及退出。此时内核尚未接通的动作显示准确可用性。

**验收**：

- [ ] 关闭最后窗口后可从托盘恢复、聚焦；重复操作不新建额外应用实例。
- [ ] 菜单状态来自 core，不维护另一套运行真相；无内核时不显示已启动。
- [ ] 明确退出走统一清理入口，关闭窗口只隐藏；真实 macOS 操作和视觉验收通过。

<a id="obg-p1-07"></a>
## OBG-P1-07 桌面壳阶段验收

**类型**：阶段验收；**依赖**：OBG-P1-01、OBG-P1-02、OBG-P1-03、OBG-P1-04A、OBG-P1-04B、OBG-P1-05、OBG-P1-06。

**泳道 / 写范围**：组合验收；所列依赖的证据/页面组合与本卡验收记录；修复先预约相关 owner 写范围。

**执行与交付**：固定可复测构建，验证首次启动→修改偏好→隐藏/恢复→退出→重新打开；完成本阶段必要审查和界面操作记录。

**验收**：

- [ ] core 独立验证、桌面构建、单实例及偏好恢复全部通过，命令/机器/构建身份有记录。
- [ ] 壳层/面板设置的视觉和交互均经真实操作验证；未完成业务未被计入交付。
- [ ] 无本阶段遗留失败，任务总表与 SESSION 更新；P2 各任务按自身依赖就绪，不等待本组合验收统一放行。

<a id="p1-01-delivery"></a>
## OBG-P1-01 交付记录（2026-10-03）

**负责人 / 泳道**：Codex /root / Core/Config，workspace、锁文件与 core/现有公共契约单一 owner。**状态**：DONE，三项卡验收 PASS。源码 HEAD `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`，分支 `codex/dist-react-restore`；本次源码/证据未提交。保留开始前 `.gitignore`、AGENTS、长期规范和全部 P0-01 证据/状态，无 commit/push/publish。环境 macOS 27.0 (26A428)、arm64，Rust/Cargo 1.99.0；GPUI 与真实内核运行 N/A（纯本地 Rust 抽取）。

### 迁移与边界

- 根 `Cargo.toml`：成员 `crates/veyra-core`、`src-tauri`，resolver 3，default-members 仅 core。原 `src-tauri/Cargo.lock` 移为根锁文件；新增唯一的本地 core 包，已有第三方包版本全部保持。输出统一为根 `target/`。
- core：完整 domain/subscription/storage；singbox 的 compiler/clash_api/runtime，以及从 managed_sidecar 抽出的 secret（ApiSecret、既有错误与生成/提取）；application 的 observability/provider_replacement/runtime/selected_subscription/state_access/subscription_management/subscription_scheduler/system_proxy。保持当前 singbox 目录命名，没有顺带重命名整个 Runtime/Compiler。
- 有效纯测试随代码迁移；12 个 state/subscription fixtures 移至 core，逐个对照 HEAD 字节一致。两个 secret 测试迁入 core；三个具体 Windows 代理与 RuntimeSupervisor 组合 Mock 回归/夹具留旧入口 `runtime_adapter_tests.rs`，没有删除或弱化断言。
- 旧 lib/application/singbox 重导出真实共享模块，commands 消费同一 core 类型/服务。IPC DTO、命令名、事件、窗口行为保留。已接管旧文件与 fixture 已移走，没有复制两套实现。
- **未迁**：旧 managed_observation_runtime/proxy_routing 编排及其测试；Windows managed sidecar/private runtime/recovery/WinINet、managed_sidecar 的进程/资产/DNS 测试；Tauri commands/lib/main/tray/event、React 和原打包资源配置。继续消费 core；不一次迁移整仓、不实现 P1-02 新领域契约或 P0/GPUI 工作。

### 真实耦合处理

1. SubscriptionScheduler 改为显式 Tokio Handle::spawn 与 tokio::task::JoinHandle。旧入口借用 Tauri runtime 的底层 Tokio Handle；core 不创建全局 runtime。测试自己持有 Tokio runtime，写屏障/取消/shutdown 语义保留。
2. managed_observation_runtime 生产路径改为注入 Handle 的 block_on；worker 对既有 SidecarPort/RuntimeObservationSidecarPort 参数化。三处 Windows 构造改为入口传入的延迟构造闭包，资源校验仍在 worker 首次使用时发生；没有 Executor、平台工厂类/注册表/平台选择服务。旧具体 Windows Port 只留装配/旧测试。
3. SystemProxyController、ProxyState、既有结果/错误移至 core application/system_proxy；RuntimeSupervisor 不再反向引用 Windows，旧 adapter 实现共享 trait。SidecarPort、opaque ManagedSidecar identity、观测接口/错误在 core singbox/runtime；具体 child/路径/Win32 仍留旧平台层。
4. 额外实际耦合：SubscriptionManager 的 System-mode Windows 代理读取改为入口注入函数。每次仍读当时系统代理，错误原样传播，无缓存/fallback。新增纯单测保护注入/不可用传播，证明失败前不会发 HTTP 请求。
5. Windows managed_sidecar/private_runtime 用 cfg(windows) 保留，WinINet 原 cfg 不变。macOS 旧入口无打包 observation sidecar，非 Windows 接线只返回既有 SidecarPortError，不伪装可运行内核。core 无具体 Windows/UI 引用，也无 cfg(windows) 业务分支；条件边界均在旧平台/入口。

跨 crate 后旧 Windows 测试需要原 cfg(test) Compiler fixture/流探针；core `legacy-test-support` **仅由旧 crate dev-dependency 启用**，复用原测试白名单、GeneratedConfig 夹具与探针，不是产品 feature 或新能力开关。普通生产 feature tree 未启用它，原生产配置白名单保持。旧入口无消费者的 serde_yaml_ng/futures-util/tokio-tungstenite 依赖移除。

### 实际验证与重跑

长命令均用 Python subprocess.run 的外层 timeout=600；旧 build/check/clippy 为 900 秒。以下均从根目录执行。旧入口命令必须前置 `TAURI_CONFIG='{"bundle":{"resources":[]}}'`，仅排除当前缺失的 Windows bundle 资源；原 tauri.conf.json 不变。这不是打包/真实运行验收。

| 实际命令/检查 | 结果/保护契约 |
| --- | --- |
| `cargo check -p veyra-core`；`cargo build -p veyra-core --lib` | PASS；macOS 独立核心构建，无 UI/内核资源 |
| `cargo tree --locked --offline -p veyra-core --target aarch64-apple-darwin -e normal,build,dev` | PASS，无 Tauri/GPUI；另查根锁文件全目标传递闭包，159 包中无 UI 包 |
| `rg -n -e platform::windows -e WindowsManagedSidecarPort -e WindowsSystemProxy -e tauri:: -e gpui:: -e windows:: crates/veyra-core/src` | 无匹配（rg exit 1 表示零匹配），core 无具体 Windows/UI 实现依赖 |
| `cargo test -p veyra-core --lib -- --test-threads=1` | **243 passed / 0 failed / 0 ignored / 0 filtered**；其中 **178 项明确纯单测**，其余为纯逻辑与本机 loopback fixture 混合集合；不启动真实 child/不改系统代理 |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | PASS；旧 main/lib/测试目标接线编译 |
| `cargo clippy -p veyra-core --lib -- -D warnings` | PASS；core 生产库 lint |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | PASS；旧入口生产库 lint |
| `cargo fmt --all -- --check`；`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | PASS；workspace/旧入口格式 |
| `git diff --check`；新增源码/文档 whitespace、引用/Task/DAG/状态检查 | PASS；无无意修改，状态/证据一致 |

旧入口定向命令：`cargo test --manifest-path src-tauri/Cargo.toml --lib FILTER -- --test-threads=1`（保留上述环境覆盖）。总计 **79 passed**，各组均 0 failed / 0 ignored：

| FILTER / 附加参数 | passed | 保护行为 |
| --- | --- | --- |
| `application::managed_observation_runtime::tests::`，追加 `--skip real_worker_` | 34 | 配置/订阅切换、取消/忙碌、保存/应用分离、shutdown、Mock child 归属；明确不执行四个真实 Windows worker 用例 |
| `application::proxy_routing::tests::` | 16 | 旧 routing 保存/CAS/选择与应用结果映射 |
| `application::runtime_adapter_tests::` | 3 | 共享 Runtime 与旧具体代理 Mock 组合恢复语义 |
| `commands::tests::` | 11 | 旧 IPC DTO/错误/请求与命令权限 |
| `platform::windows::system_proxy::tests::` | 14 | 原代理算法和共享 trait；内存 Mock，不调用 WinINet |
| `tests::hidden_window_drops_deltas_and_visible_window_resumes_without_backlog`，追加 `--exact` | 1 | 隐藏/恢复事件桥消费共享 Observation |

带超时的实际重跑形式（其他命令替换参数列表，旧入口保留 env）：

```sh
python3 - <<'PY'
import subprocess
subprocess.run(['cargo', 'test', '-p', 'veyra-core', '--lib', '--', '--test-threads=1'], timeout=600, check=True)
PY
TAURI_CONFIG='{"bundle":{"resources":[]}}' python3 - <<'PY'
import subprocess
subprocess.run(['cargo', 'check', '--manifest-path', 'src-tauri/Cargo.toml', '--all-targets'], timeout=900, check=True)
PY
```

[验证摘要](evidence/p1-01/validation.json)、[全目标锁闭包](evidence/p1-01/lock-closure.json)、[core 测试](evidence/p1-01/core-tests.log)、[依赖树](evidence/p1-01/core-tree-macos.log)、[生产 feature 树](evidence/p1-01/legacy-features.log)及全部最终命令日志见 evidence/p1-01/。

### 尝试历史、自查与限制

- 初次 core 测试编译有搬迁后的测试 import/字段初始化遗漏，已修复；最初旧检查缺 Windows 资源为 FAIL，使用检查专用覆盖后 PASS；后续跨 crate 探针/Handle 参数遗漏均修复。未伪造资源或改打包配置。
- `cargo tree -p veyra-core --target all -e normal,build,dev` 尝试因不可写 Cargo cache 中缺失 wasm crate 失败，不计 PASS；macOS tree + 根锁文件全目标闭包为等价依赖证明，未把空输出当成功。
- 额外 `cargo clippy -p veyra-core --all-targets -- -D warnings` **FAIL**：5 项继承测试 lint（Observation 借用 1、第三方 WS 回调固定大 Err 类型 3、WireGuard fixture 八参数 1）。表达式/签名来自 HEAD 旧测试；未删除测试、加全局 allow 或弱化断言。项目规定的两条生产 --lib clippy 已 PASS，此额外测试 lint 不改变本卡纯单测/生产库验收。[失败日志](evidence/p1-01/core-clippy.log)保留。
- 采用 code-delivery-review 做**实现者自查**：公共类型/secret/配置白名单、Tokio runtime 生命周期、worker 延迟资源构造/shutdown、旧 commands 接线、Windows cfg/有效测试、单锁文件/依赖版本与剩余引用。未发现未处理的本卡行为缺陷；不声称独立审查。
- Windows 真实运行/打包、macOS 可用 sidecar、GPUI/browser/截图/视觉均 NOT_RUN 或 N/A（不属本卡），不以 Mock/编译冒充。原 P0-01 历史含义不变。
- DAG 重算：P1-02 READY；P0-03/P0-04 仍 READY；未开始下游。当前公共 owner 释放，下一领取任务登记新 owner。

<a id="p1-02-delivery"></a>
## OBG-P1-02 交付记录（2026-10-03）

**负责人 / 泳道**：Codex /root / Core/Config，domain/application/storage 共享 DTO、版本和业务快照单一 owner；完成后释放。**状态**：DONE，四项卡验收 PASS。分支 `codex/dist-react-restore`，HEAD `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`；所有交付未提交，保留 P0/P1-01 dirty 基线。macOS arm64，沿用 P1-01 workspace/单一锁文件，没有新依赖；禁止的 `.gitignore`、AGENTS、长期规范字节保持不变。纯本地 Rust，本轮没有访问 openbox.disign.me、启动浏览器/视觉对齐、真实 sing-box、系统代理/helper 或 commit/push/publish。

### 类型、schema 与服务

- `domain/version.rs`：StateEpoch（OS 随机 128-bit 身份）、StateVersion（epoch + revision）、ConfigVersion、SelectionVersion、SnapshotVersion。版本不实现跨 epoch 数字排序；compare/require 对 epoch 或 revision 失配返回稳定 RevisionConflict。计数保持既有 MAX_SAFE_INTEGER 范围，耗尽拒绝写入；`try_empty` 将随机源失败传播为错误，保留旧 empty/Default 构造兼容。
- `AppState` / `StoredStateV7`：在原单一业务快照增加 state_epoch/config_revision/selection_revision、Profile 与 AppConfig；原 SubscriptionId/ProviderId/NodeId/PoolId/RoutePolicyId、引用和旧 generation 保留。pool 中手动选择是本卡单一选择事实，没有再复制 selection-state 文件或第二份可编辑订阅/路由真相。
- `domain/profile.rs`：只建立本卡代表字段。IPv6；DNS split/direct/proxy/port/extras；TUN autoRedirect/stack/mtu/tcpMss；routing custom name/enabled/domain/domainSuffix rules，目标继续使用已有 RouteTarget/PoolId。Profile custom 部分是后续 OpenBox 字段的基础，旧 routes 仍是旧 routing 事实；本卡不在两者间做导入同步，不实现完整 P4/P5 或新 Compiler 映射。
- ProfilePatch/Patch<T>：Missing 保留，Null 仅清空 routing.custom/name；必填对象、标量、数组的 Null 返回固定字段位置。嵌套对象只合并批准字段，DNS extras/custom rules 数组整体替换（空数组清空）。未知字段拒绝；端口/TUN 参数、非空值与已有 Pool 引用在保存前校验，内部没有任意 JSON merge/command。
- `AppConfig.check_updates_on_start` 默认 true，v6 migration 明确填入，仅持久化桌面版本检查偏好。登录项、注册状态、updater 运行进度及自动安装不进入业务快照/本卡实现。
- `application/state_service.rs`：ProfileService.patch、SelectionService.select_manual、SnapshotService.snapshot/append/replace，共享既有 StateAccessGate 与 JsonStateStore。写时加载最新完整快照、检查适用预期版本，构造候选并在持久化成功后返回版本/事实；没有内存先推进。StateStore.save 仍是已验证事务的物理写入原语，业务修改使用 commit 或明确 replace，未来消费者须共享同一 gate。

### 版本推进与原子性

| 操作 | epoch / 版本规则 | 保存/应用事实 |
| --- | --- | --- |
| 首次空快照 / v6 首次升级 | 新建一次 epoch，config=0、selection=0；稳定重载不重新生成 | SavedOnly |
| 普通 profile/配置/订阅事实变化、append | 当前 epoch；config +1，保留 selection；实际无变化不推进/不重复写 | SavedOnly |
| pool 手动节点选择 | 当前 epoch；selection +1，保留 config；相同选择 no-op | SavedOnly，不声称控制器已确认 |
| 结构变更同时移除/改变已有选择 | 两个实际受影响计数分别推进 | SavedOnly |
| 完整 replace/恢复/整份 backup 回退 | 新 epoch；丢弃导入 token，config=0、selection=0；旧表单/task 失效 | SavedOnly |
| v7 稳定读取 / 普通 schema 升级 | 已有 epoch 不无意义更换；不因读操作推进计数 | 不制造应用结果 |

配置 patch 只检查 ConfigVersion，因此并发选择不使其失效；选择只检查 SelectionVersion，但在最新配置上验证成员，所以不覆盖配置变更。完整 replace 必须同时匹配配置/选择版本。append 仅追加已经规范化 subscription/provider/node 批次，重复 ID/非法引用拒绝整批；完整 OpenBox/备份未知字段报告与冲突 ID 重映射仍由 P7-01 负责。

物理写入继续使用已有 temp/write/fsync/rename/backup 流程。模拟当前临时文件写入失败和备份 rename 失败，分别验证原 bytes、epoch、两个版本与全部状态保留；replace/append/selection 的失败也不返回新状态。整体 backup 恢复先产生候选新 epoch，成功原子写入才发布；旧备份不被当作可复用并发凭证。

### 错误、Runtime 与旧入口

- AppError/AppErrorCode/FieldPath/ErrorDetail：稳定 code（Validation、RevisionConflict、UnavailableOnPlatform、PermissionDenied、PortInUse、KernelUnavailable、Timeout、Cancelled、DownloadFailed、StorageFailed、ApplyFailed）、固定中文用户 message、可选批准字段路径、可选诊断类别。detail 不接受配置、secret、任意路径或原始响应；Debug/Display/JSON 经 secret fixture 与存储路径反向断言。旧 StateStoreError 映射保留 RevisionConflict 语义。
- ApplyEffect 明确定义 SavedOnly / Applied / RestartRequired。本卡没有 Runtime 应用证据，所有服务返回 SavedOnly；不根据是否历史 Ready 推测 RestartRequired，不返回 Applied。后续实际 Runtime owner 才能报告两者。
- RuntimeFacts/RuntimeState、InstanceId、AppliedInstance、LastSuccessfulVersion、RuntimeSnapshot：当前实例/status、业务 saved、当前 applied、历史 last_successful（含应用时选择版本）分别表达。Stopped 永远没有 instance/applied；Ready 必须由 owner 提供当前 ID/版本；Recovering 的 AppliedInstance 成对绑定 ID/版本；新业务 epoch 可与仍需清理的旧实例版本并存。
- 本卡不创建 `runtime/last-applied.json`，不伪造 applied/last-success 持久化；LastSuccessfulVersion 是可序列化的后续 owner 读写边界。测试覆盖 stopped+history、ready+旧 epoch、恢复/启动/失败、manifest 失败时 live 与 history 分离、selection 不制造配置待应用。
- v1→…→v6→v7 使用明确 migration；保留 pre-migration 和旧节点凭据/稳定 ID/引用，不把旧 v6 冒充 OpenBox Profile。v7 缺必需版本字段视为无效快照，沿用既有恢复策略，不默默生成默认并发 token。
- 旧 SubscriptionManager/ProviderReplacement/Runtime activation/routing 写入接同一 commit；返回的新版本仅在成功保存后用于内存/消费者。订阅 UpdateSnapshot 带 ConfigVersion；旧 routing 兼容 numeric CAS 的 material 包含 epoch，保留既有 IPC DTO/错误 envelope；同进程整体替换使旧 request Conflict。旧 missing-state routing 读取先持久化初始 epoch，避免每次空读取生成新 token。旧 IPC 不暴露 AppState 原始模型，因此升级字段不破坏原请求/响应 shape。

### 实际验证与验收

所有 Cargo 长命令使用既有 Python subprocess.run 外层超时：core/fmt/tree 600 秒，旧入口 900 秒。旧入口保留 `TAURI_CONFIG='{"bundle":{"resources":[]}}'` 检查专用覆盖，tauri.conf.json 未改；这不代表资源打包或真实设备运行通过。完整命令/日志见[命令记录](evidence/p1-02/commands.json)。

| 命令/检查 | 最终实际结果 |
| --- | --- |
| `cargo check -p veyra-core`；`cargo build -p veyra-core --lib` | PASS |
| `cargo test -p veyra-core --lib -- --test-threads=1` | **275 passed / 0 failed / 0 ignored / 0 filtered**；新增 32 项，本机文件事务/纯逻辑；继承集合还含 loopback HTTP/WS fixture，无真实 child/外站 |
| `cargo clippy -p veyra-core --lib -- -D warnings` | PASS |
| `cargo check --manifest-path src-tauri/Cargo.toml --all-targets` | PASS |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | PASS |
| 旧 lib 定向测试：routing / observation（`--skip real_worker_`）/ runtime_adapter / commands | **17 / 34 / 3 / 11 passed，共 65**；均 0 failed / 0 ignored，非零过滤数明列在各日志；Mock/local，未执行真实 worker |
| `cargo fmt --all -- --check` | PASS |
| `cargo tree --locked --offline -p veyra-core/veyra --target aarch64-apple-darwin -e normal,build,features` | PASS；两棵生产树均未启用 legacy-test-support；core 无 Tauri/GPUI |
| core 源码具体 Windows/UI 反依赖检查；受保护文件 SHA-256；HEAD/工作树核验 | PASS；原依赖/锁文件无新增，本轮未修改受保护文件 |
| `git diff --check`；任务引用/身份/计数/DAG 与新增文件 whitespace | PASS，见最终验证摘要 |

| 用户验收 | 对应实际保护契约 | 结果 |
| --- | --- | --- |
| A | Missing/Null/Value、局部 DNS/routing/TUN 保留、数组整替、字段定位/引用校验 | PASS |
| B | profile/replace/append/selection 保存失败旧完整 snapshot 保留、备份替换失败 | PASS |
| C | 跨 epoch 及 stale revision 拒绝；旧 routing form/订阅任务迟到写回被拒绝 | PASS |
| D / E | append 同 epoch config +1；手动 selection 单独 +1；replace 新 epoch 0/0 | PASS |
| F | saved/current applied/history 分离；Stopped/Starting/Recovery/Failed 不沿用历史 Ready | PASS |
| G | 11 稳定码、固定字段/detail、Debug/Display/JSON 无 secret fixture/存储路径 | PASS |
| H | 原 243 core 继续执行；旧入口 check 与 64 原定向回归 +1 新 epoch 回归 | PASS |

**必要审查**：按 code-delivery-review Skill 做实现者自查，未声称独立审查；关注原子事务/旧 CAS、schema 与稳定引用、选择归属、AppError 输出、Runtime owner 边界和生产 feature。没有本卡未解决缺陷。GPUI/UI/浏览器/视觉、真实 Runtime/manifest、helper/系统网络/Windows 设备验收均 N/A 或 NOT_RUN（当前用户明确纯本地 Rust，后续按卡执行）。

**尝试历史保留**：初次测试编译遗漏新测试模块/测试构造字段及借用问题，修复；首轮 core **263 PASS / 8 FAIL**，为旧 schema 字段清单/迁移 epoch/backup epoch 的旧预期；第二轮 **274 PASS / 1 FAIL**，为字段数量断言仍为 9，改为 schema v7 的明确 14 字段；最终 275 PASS。旧 observation 初次 **33 PASS / 1 FAIL**（断言候选未带提交后 revision），更新为 revised_from 的精确预期后 34 PASS。新增旧 routing 测试曾误用旧 application namespace 导致 check FAIL，改用实际 core namespace 后 check/17 routing PASS。失败日志/记录保留，不将旧 FAIL 改写为当时 PASS。

**后续边界**：Profile 是批准代表字段基础，完整 P4/P5 能力、OpenBox importer/ID remap、Runtime manifest 和 controller 确认不在本卡；后续写入使用同一 gate 与版本服务，save 只是物理原语。新 v7 数据不承诺旧二进制直读，回退须使用保留的旧 schema 快照。本卡没有待修复阻塞。

**DAG / 下一动作**：P1-02 DONE 后重新计算 68 卡：macOS 4/61 DONE、2 READY、55 TODO；Windows 7 DEFERRED。READY 仅 P0-03/P0-04，均未启动；P1-03 仍等待 P0-03，保持 TODO。共享 owner 已释放；不启动后续 Task，不 commit/push。
