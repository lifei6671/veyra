# P0 基线与可行性

[进度与任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [技术方案](../openbox-rust-gpui-implementation-plan.md)

目标：用最小原型消除会改变路线的未知量，确定首期范围并重新估算。允许隔离原型；正式实现按任务卡中的技术依赖推进。原型不要求完成生产级全部失败分支。

<a id="obg-p0-01"></a>
## OBG-P0-01 固定 UI、接口和行为基线

**类型**：调查与契约；**前置**：无。**依据**：方案 §2–3、§18–19；`src/openbox/api/{client,types}.ts`、`src/openbox/pages/`、`openbox.css`。

**执行与交付**：登记源码 commit、参考环境版本、六主页面/九分类及关键弹窗；采集会影响模型的脱敏 DTO、错误与流式帧；列出当前未知字段。输出基线记录和少量 fixtures，后续功能继续补样本。

**验收**：

- [x] 77 个 client 方法、4 个流通道均有去向；未使用接口与当前页面入口区分清楚。
- [x] 同尺寸浅色/深色截图覆盖壳层、节点卡片、弹层、DNS 表格与背景；记录来源、状态和尺寸。
- [x] failover 手动选择、traffic 单位、DNS 热更/记录等歧义被列为待验证项；没有将猜测写成接口事实。
- [x] 样本脱敏，可供后续离线契约测试使用；不为采样修改生产配置。

**当前结果**：四项验收全部 PASS，P0-01 DONE。当前源码 8 张同尺寸浅深色截图及真实操作通过；有界登录重试成功，13 指定 GET 与四 WS 取得真实脱敏样本。详见[最终验收](P0-01-baseline.md#current-acceptance)。以下旧交付/失败记录保留历史身份。

<a id="obg-p0-02"></a>
## OBG-P0-02 落实当前范围与轻量任务规则

**类型**：文档与规则清理；**前置**：无（用户已直接明确路线和流程调整）。**依据**：2026-10-03 用户要求；方案 §1.1、§1.3、§17；`AGENTS.md` 与本任务目录。

**执行与交付**：停用旧 SDLC 状态和 UI 门禁入口，保留当前任务清单与真实行为验证；固定 GPUI/OpenBox、GitHub Releases、无发布签名/公证、手动更新。DNS 等能力差异继续由原型结果决定。

**验收**：

- [x] 当前规则不再要求恢复隐藏状态、补办 DCR/UI Contract 或逐 Task 人工签字。
- [x] 旧门禁脚本及专用测试删除，审查技能不再依赖该流程；历史文档明确不再强制绑定。
- [x] 方案/任务已采用 GitHub 无发布签名路线，区分 Apple Silicon 最低 ad-hoc 要求和系统管理员权限。
- [x] 未运行的技术原型保持待办，未实现能力不因文档调整算通过；普通业务测试与界面检查保留。

<a id="obg-p0-03"></a>
## OBG-P0-03 GPUI 版本、输入与托盘原型

**类型**：原生 UI 原型；**前置**：OBG-P0-01。**依据**：方案 §9、§14；隔离原型目录，后续成果迁入 `crates/veyra-desktop/`。

**执行与交付**：锁定 toolchain、GPUI Kit/GPUI 来源及版本、最低 macOS；构建输入框、长文本弹窗、虚拟列表、主题/字体样例和托盘。保留可重跑命令与锁文件。

**验收**：

- [x] 中文输入法、选区、粘贴、撤销和 Escape/焦点归还有真实操作记录。
- [x] 关闭最后窗口后托盘仍响应，可显示并聚焦窗口、更新菜单、退出；不引入第二个主事件循环。
- [x] 浅色/深色、字体/Emoji、透明度与模糊的可实现程度有截图；无法还原的差异明确列出。
- [x] 依赖树没有不兼容的 GPUI 类型组合；在锁定的 macOS/架构上可复现构建。

**当前交付**：DONE（2026-10-04），四项验收 PASS。来源为 [Host/User Manual Acceptance](evidence/p0-03/README.md#host-user-manual-acceptance) 与 [人工确认记录](evidence/p0-03/host-manual-acceptance.json)：用户亲手完成真实中文 Input/Modal Textarea 输入及托盘点击，确认可用并明确批准“就当验收通过了”。真实手工输入补足 composition/candidate 真实性；托盘人工确认结合历史 Host 生命周期操作补足卡验收。视觉以 11 张真实截图、window blur/backend 与差异说明通过；版本/依赖/构建沿用 PASS，追加 resource override workspace check PASS。

**历史与限制**：2026-10-03 ACCEPTANCE 及 interaction-results 的 IME candidate/composition、tray screenshot/focus、controlled blur 等 LIMITATION 保留；本次不补造截图、不声称 card backdrop 已证明。MiSans、nested dropdown、card-level backdrop blur 与 macOS 15 实机作为后续限制，不阻止本卡 DONE。详见[实机原型记录](evidence/p0-03/README.md)。本轮不启动 P0-04/P1-03。

<a id="obg-p0-04"></a>
## OBG-P0-04 固定内核、控制器与缓存原型

**类型**：内核集成原型；**前置**：OBG-P0-01。**依据**：方案 §4.3、§7.5、§7.7；现有 `src-tauri/src/singbox/` 仅作复用依据。

**执行与交付**：固定 macOS sing-box 二进制/特性/摘要；检查代表性 DNS/FakeIP/TUN 配置，验证受管控制器地址发现及最小 selector/FakeIP 恢复。产出支持项、限制与资源身份记录。

**验收**：

- [x] 相同配置完成 check、启动、鉴权就绪与停止；使用 macOS 二进制证据。
- [x] `127.0.0.1:0` 的实际地址可从本实例输出取得；若不可行，明确选择方案中的有界 bind 冲突处理。
- [x] 选择/FakeIP 恢复及关闭 writer 后的缓存交接有最小证据；对账前流量窗口和兼容性限制被记录。
- [x] 结果只说明 Veyra 使用的配置/API/生命周期能力，不建立协议认证矩阵。

**当前状态**：2026-10-04 四项 PASS / DONE；详见[真实内核交付](#p0-04-delivery)。

<a id="obg-p0-05"></a>
## OBG-P0-05 本地 helper、授权与进程归属原型

**类型**：平台原型；**前置**：OBG-P0-03、OBG-P0-04。**依据**：方案 §11.1–11.3、§14；目标 `crates/veyra-helper/`。

**执行与交付**：验证管理员安装本地 launchd helper、Unix socket 对端身份、安装/取消/卸载；对一个测试 Network Service 写入、回读、恢复；helper 启动普通用户 SystemProxy child 并检测 GUI 实际退出。

**验收**：

- [ ] 最低 macOS 目标上无 Developer ID 也可按方案运行；安装目录受保护，对端 UID/进程身份来自系统，不能以自报 PID 或 ad-hoc 证明开发者身份。
- [ ] 拒绝授权可继续手动代理；批准后的重复操作不依赖未经验证的授权假设。
- [ ] helper 管理 SystemProxy child 时内核使用已验证用户身份，受保护资源可按需读取。
- [ ] 强制结束测试 GUI 后，helper 恢复仍归属自己的代理并清理 child；瞬时 IPC 断开不被当成进程死亡。
- [ ] 拒绝其他未授权用户和任意命令/路径；管理员权限或系统限制不足时记录具体错误，不把证书缺失列为阻塞。

<a id="obg-p0-06"></a>
## OBG-P0-06 应用自身出站策略原型

**类型**：网络路径验证；**前置**：OBG-P0-04、OBG-P0-05。**依据**：方案 §8.9。

**执行与交付**：最小 Rust 下载客户端显式选择 Direct/ViaRunningProxy；验证系统代理、TUN、独立 DNS 引导和重定向路径，记录真实出口及失败结果。

**验收**：

- [ ] 系统代理/TUN 开关组合中 Direct 的出口与 DNS 有证据，不能只以 `no_proxy()` 作为结论。
- [ ] 无订阅或代理失效时仍可尝试直连；不会依赖尚未下载的订阅自举。
- [ ] ViaRunningProxy 使用最后有效实例，失败不静默改道；跨源不转发认证头。
- [ ] 无法证明 Direct 的组合明确判为不支持或未完成，供 P0 出口决定范围。

<a id="obg-p0-07"></a>
## OBG-P0-07 流量、DNS 与诊断能力核实

**类型**：观测契约原型；**前置**：OBG-P0-01、OBG-P0-04。**依据**：方案 §8.6–8.7、§10。

**执行与交付**：取四类流式帧；确认 traffic 单位、短连接缺口、DNS 记录/清缓存/热重写来源；完成一个经受管入口的路由诊断。输出逐项能力表。

**验收**：

- [x] 区分速率、区间增量和累计值，给出转换示例及重置行为。
- [x] DNS 三项能力分别有支持证据或缺口说明；连接流不冒充 DNS 查询流。
- [x] 诊断只展示实际关联成功的字段；缺失路径明确为不完整。
- [x] 无能力时提出可审查的首期差异，不自动扩大为自建 DNS 内核；统计索引/容量留到 P3。

<a id="obg-p0-08"></a>
## OBG-P0-08 更新与分发路线

**类型**：分发原型；**前置**：OBG-P0-03、OBG-P0-05。**依据**：方案 §8.8、§14。

**执行与交付**：构建最小应用包，在浏览器实际下载后的产物上验证首次打开；固定 GitHub Releases 的版本检查、下载/摘要核对与手动安装，产出安装说明。

**验收**：

- [ ] GitHub 下载与手动安装路径明确；不引入 Sparkle、开发者账户或公证流程。
- [ ] 明确应用/helper/内核/资源的版本绑定及更新时退出顺序。
- [ ] 下载复用显式出站客户端；浏览器下载后的 quarantine 与首次打开步骤有真实记录，未要求关闭系统保护。
- [ ] Apple Silicon 产物保留可运行的最低 ad-hoc 签名并验证内嵌二进制；明确不提供发布者身份或公证，校验和不冒充签名。

<a id="obg-p0-09"></a>
## OBG-P0-09 P0 出口与重新估算

**类型**：阶段验收；**前置**：OBG-P0-01、OBG-P0-02、OBG-P0-03、OBG-P0-04、OBG-P0-05、OBG-P0-06、OBG-P0-07、OBG-P0-08。

**执行与交付**：汇总原型身份/结果、未决项与范围差异；按本清单逐任务重估开发、集成、定向验证、Review 和人工返工，管理员测试环境等外部条件单列；必要时拆细任务并更新依赖。

**验收**：

- [ ] 所有影响路线的未知项已有实测结论和明确决定；有未决项则保持本任务未完成。
- [ ] 当前范围与任务规则清楚；DNS/路由器缺口有实现或差异决定，无需旧流程批准文件。
- [ ] P1–P7 的工作量、假设、风险、排除项可审查；撤回的旧工期没有复用。
- [ ] Windows/Linux 单独估算；与 P1 相关的核心原型通过后可开展 P1；权限/TUN 未决项只阻塞依赖它们的功能。

<a id="execution"></a>
## 技术可行性怎么做

每个原型只回答一个会改变实现路线的问题，留下“机器/版本、命令、预期、实际、结论和下一步”。代码放在临时隔离目录或 `prototypes/` 的独立小项目，确认能用后再迁入正式 crate；不先建完整基础设施。

| 顺序 | 最小动作 | 通过标准 | 未通过时怎么处理 |
| --- | --- | --- | --- |
| 1 环境与基线 | 记录 macOS/CPU、Rust toolchain、Xcode Command Line Tools；核对当前 UI/API 样本 | 构建工具可用、参考身份明确 | 只补实际缺少的工具或样本，不要求 Apple 账户 |
| 2 GPUI smoke | 一个窗口、中文输入、弹层、浅深色、虚拟列表和托盘；打成最小 `.app` | 输入/焦点/隐藏恢复真实可用，可重现构建 | 定位具体版本/API/控件问题，不先重写全部页面 |
| 3 普通内核 smoke | 固定 macOS 内核，生成一个 loopback mixed + selector + DNS + cache 配置，check→启动→控制器鉴权→一次受控访问→停止 | 配置可加载、实际地址可发现、选择/缓存恢复可核对、无本次残留 | 输出最小复现配置与错误；不扩大协议测试 |
| 4 helper smoke | 管理员安装固定 launchd job；一个 Unix socket ping/status；再启动普通用户 child、修改并恢复一个测试网络服务 | 文件/UID/进程身份正确，GUI 死亡能清理；卸载能恢复 | 记录系统拒绝或 API 缺口，手动 loopback 仍可继续开发 |
| 5 TUN/出站/DNS | 独立测试环境短时启停 TUN，验证 Direct/ViaRunningProxy 与独立解析；采集 DNS/traffic 最小样本 | 出口与单位有证据；每个 DNS 能力单独有结论 | 无事件则记录能力差异；不可证明直连就拒绝该模式下载 |
| 6 GitHub 包 smoke | 用步骤 2 的包模拟真实浏览器下载，保留 quarantine；测试首次打开、复制安装、版本检查和手动替换 | 普通用户按文档可重复操作，需管理员的 helper 安装单独说明 | 修正 bundle/资源/最低 ad-hoc 与步骤，不关闭 Gatekeeper/SIP |
| 7 汇总 | 将各原型结果链接到任务表，更新方案的具体依赖版本、支持范围与估算 | 每个未知项成为已证实能力、明确差异或带最小复现的问题 | 只暂停依赖该问题的任务，继续其他已具备条件的工作 |

步骤 1–3 无需开发者账户，也不要求安装权限服务。步骤 4–5 需要目标机器的管理员授权和可恢复网络环境；本次文档清理没有执行这些系统操作。TUN 用 sing-box 的本机适配，首期不走 NetworkExtension/App Store 分发路线。

<a id="cleanup-record"></a>
### OBG-P0-02 交付记录（2026-10-03）

来源：用户明确停用旧流程并指定 GitHub 无发布签名分发。交付物：`AGENTS.md`、精简后的项目审查技能、已归档旧 UI 契约、当前方案/任务，以及旧门禁脚本、专用测试与旧流程技能锁记录的删除。当前工作树本就没有 `.sdlc/` 状态目录，本次移除其规则和配置引用，没有伪造状态恢复或历史验收结果。

验证：64 个任务卡及状态一致，依赖无环；77 个 API、19 个场景覆盖完整；150 个本地链接及章节引用有效，已跟踪与新增文档的差异格式检查通过。`pnpm exec vitest list --filesOnly` 成功发现 29 个产品测试文件，旧门禁已不在集合中；本次没有运行这些测试。技能 YAML/字段检查使用系统 Ruby 通过；Python `quick_validate.py` 因缺少 PyYAML 未能运行，没有为此新增依赖。此记录不代表任何 GPUI/helper/TUN 原型已通过。下一任务为 OBG-P0-01。

<a id="baseline-record"></a>
### OBG-P0-01 交付记录（2026-10-03）

负责人 Codex /root；源码 `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`，分支 codex/dist-react-restore。本轮先领取 DOING，实际写范围为 P0 卡、主表、SESSION、[基线文档](P0-01-baseline.md)、[fixtures](fixtures/p0-01/README.md)；Core/Config + Observation + GPUI 基线文档 owner 单一，产品公共契约/生产资源只读。既有 .gitignore、AGENTS、DEVELOPMENT_WORKFLOW 未覆盖，未 commit/push/publish。

交付：77 client 方法与逐项业务位置/后续任务、4 流通道、六页面/九分类/关键状态与弹层、未知字段/宽类型清单；五组 22 个离线 case（已有测试样本及源码消费契约，每项 commit/source/redaction/observed=false）；结构和历史视觉元数据另列，真实运行观测 0。

验收：接口去向 PASS；同尺寸五类浅深色截图 NOT_RUN；歧义登记 PASS；脱敏样本 PASS。当前仅差视觉验收，状态 ACCEPTANCE。GUI inventory 成功，但访问现有 localhost 页面被浏览器安全策略拒绝（用户拒绝访问权限），没有绕过；旧 Veyra pair 不属于当前 OpenBox，本机 OpenBox 历史图尺寸不一致/捕获身份不足，不用于凑 PASS。

验证：TypeScript 7.0.2 AST api properties=77，业务引用方法=72、无业务=5；四通道及实际消费者=4；RouteKey/根组件/侧栏六身份、SettingsPage 九身份与分支一致；JSON/来源/源码指纹/脱敏、本轮本地 Markdown links/anchors、新文件 whitespace 和 git diff --check 定向检查。具体最终计数在 [基线验证结果](P0-01-baseline.md#final-validation)。产品代码未改，pnpm/cargo/build/test/内核/网络/权限 NOT_RUN。自查者为 Codex /root，不声称独立 Review。

下一步：在允许访问的 GUI/browser 环境中，按基线 §7 固定 1280×720 CSS viewport + DPR，隔离生产写入，捕获 light/dark 壳层/背景、节点、非写弹窗、DNS 表并登记版本/操作/像素/hash/来源。四验收真实齐全后 DONE，再重算 P0-03/P0-04/P0-07/P1-01。当前仅 P0-02 DONE，四下游继续 TODO，Ready Queue 空；不开始本轮以外任务。

2026-10-03 追加授权恢复：用户允许 localhost 与远端只读取证，但两个浏览器访问调用仍被工具权限拒绝。无新增真实响应或截图，保持 ACCEPTANCE；详见 [授权后的实际尝试](P0-01-baseline.md#authorization-attempt)。需解除保存的工具访问限制后继续，不绕过、不改生产配置。

2026-10-03 明确授权 CLI 替代取证：本机 Vite/HTML 入口核对成功；Chromium Mach 服务权限拒绝、WebKit 启动退出，视觉仍 NOT_RUN。新增 17 observed=true 请求失败 case（13 GET 全 403、4 WS 无帧）；没有业务 DTO/流帧或语义结论。P0-01 继续 ACCEPTANCE，READY=0；当前精确环境下一步及方法汇总见 [CLI 只读实际结果](P0-01-baseline.md#readonly-observation)。先前访问失败与 JFIF density 修复历史保留。

2026-10-03 临时登录例外：仅一次 auth/login POST，返回 403；原 13 GET 与前置/HTTP1.1 确认同为 403，WS 3×401/1×502、无帧。单次浏览器确认仍因 Mach 服务拒绝而失败，视觉 NOT_RUN、P0-01 ACCEPTANCE、READY=0；临时会话/脚本已删除。详见 [临时登录实际结果](P0-01-baseline.md#temporary-auth-observation)。

2026-10-03 最终验收追加：四项 PASS，P0-01 DONE；8 PNG（1280×720/DPR 1/Chromium 149.0.7827.22），54 操作/状态断言、77/4/6/9、脱敏与 DAG 检查通过。登录两次（403 后一次传输修正重试 200）、普通 GET 14 成功/1 失败、四 WS 各有帧，traffic 单位/区间或速率/重置仍 UNKNOWN。P0-03/P0-04/P1-01 READY，P0-07 TODO；没有开始下游实现。完整证据和历史限制见[当前最终验收](P0-01-baseline.md#current-acceptance)。


<a id="p0-03-manual-acceptance"></a>
### OBG-P0-03 人工验收收口（2026-10-04）

Codex /root 仅更新证据与状态，原型功能代码、Cargo workspace/lock 和历史截图不变；用户亲手完成中文输入/托盘操作并明确批准本卡通过，四项 PASS，来源见 [Host/User Manual Acceptance](evidence/p0-03/README.md#host-user-manual-acceptance)。历史缺证据及无资源覆盖 workspace check FAIL 保留，新增人工确认与本次两条 check PASS 不改写历史。JSON、11 PNG hash/dimensions、68-card DAG/主表依赖、Markdown links/anchors 与 diff 检查见 [validation.json](evidence/p0-03/validation.json)。P0-04/P1-03 READY、P0-05 TODO，未启动后续任务，无 commit/push。

<a id="p0-04-delivery"></a>
## P0-04 真实 macOS 内核交付（2026-10-04）

Codex /root，Runtime/Platform；起始 HEAD `19696abde8a27a36c4eb490a4236006fcd09c9cc`，工作树干净。固定官方 v1.14.0 macOS arm64 archive 在执行前匹配 digest；binary 身份/version/build tags、5 份实际配置 check、4 个真实 child 的鉴权动态 controller、同配置 selector/FakeIP 恢复、关闭 writer 后新目录 handoff 均 PASS。目标 FakeIP `.3` 与空缓存对照 `.2` 区分，普通 DNS cache 关闭；没有用 cache 文件存在替代恢复证明。

四项卡验收全满足，P0-04 DONE；存在对账前流量窗口，同配置 restart 22.990292 ms、handoff 14.466458 ms，并通过对账前实际 loopback mixed 请求证明。P2 在对账前不报业务 Ready、不打开新的系统代理接管；不承诺原子无缝切换。仅证明同版本/cache_id/tag/pool/range；sandbox 网络更新监听 operation not permitted 保留，不证明动态网络事件或正式 Runtime。

完整证据、前期两次 check FAIL、试探 config hash 未捕获的限制、复跑步骤及实现者自查见 [README](evidence/p0-04/README.md)；[验证](evidence/p0-04/validation.json)记录 JSON/脱敏/68-card DAG/links/core check/diff 和清理。四个正式 child 均 SIGTERM/exit 0、wait/group empty、listener 释放、writer 关闭；前期 child 也正常退出，所有本任务临时 archive/binary/config/cache 清理。未访问 UI 远端、未改系统代理/DNS、TUN 只 check、未修改 Compiler 或实施 P2/P1-03/其他原型，无 commit/push。

按实际 68 卡依赖重算，P0-05/P0-07/P1-03 READY，P0-06 等待 P0-05；仅记录队列，不启动下游。Runtime/Platform 临时资源 owner 已释放。

<a id="p0-07-delivery"></a>
## P0-07 本地 Observation 能力交付（2026-10-04）

Codex /root，Observation 泳道；起始 HEAD `fac6f12`、工作树干净，P0-01/P0-04 DONE 后领取本卡 DOING。只拥有本任务独立 child/controller/origin/UDP DNS fixture/temp；没有与 P0-05 helper 共用实例，没有访问 UI 参考远端或公网协议测试，没有系统代理/TUN/系统 DNS/管理员操作，没有产品 Rust 或公共契约改动。

四项验收均 PASS，P0-07 DONE：官方 1.14.0 source/tag/commit、archive digest 与 binary Revision 核对；四 WS 多帧；traffic 区间 bytes/约 1 秒/两 burst/重连缺口；跨快照长连接与未被 snapshot 捕获的短请求；DNS records UNSUPPORTED、实例 response cache flush SUPPORTED、hot rewrite UNSUPPORTED；受管 HTTP 200 诊断只展示实际关联字段，日志规则 index 仅限本配置，DNS event/resolver chain/process/filter hit 不完整。DNS 重写首期 `RestartRequired`；不自建 DNS/内核扩展，SQLite 索引/容量等留 P3。

脚本：[p0-07-observation-probe.py](../../tools/p0-07-observation-probe.py)。完整证据/边界/复跑与实现者自查：[README](evidence/p0-07/README.md)；[validation](evidence/p0-07/validation.json)记录源码身份、真实原型、JSON/脱敏/AST/core check、68-card DAG、Markdown links/anchors、diff 与资源清理。首轮 port checker 的 FAIL 保留；自查修复默认 interface 日志脱敏后又完整重跑，最终正常 SIGTERM/exit 0、wait/group empty、全部 streams/fixture threads 关闭、4 个 listener 释放和 temp 删除，不用强杀冒充正常停止。

按 68 张任务卡重算，READY 为 P0-05/P1-03；P0-09、P3-01、P5-01 等仍缺其他依赖，无新增 READY。只释放本任务资源和记录下游，不启动下游，不 commit/push。
