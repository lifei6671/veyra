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

**类型**：平台原型；**前置**：OBG-P0-03、OBG-P0-04。**依据**：方案 §11.1–11.3、§14；目标 `crates/veyra-helper/`，P0 仅显式 `p0-05-prototype` feature / `src/p0_05/` 入口，不作为正式 P2 helper。

**执行与交付**：验证管理员安装本地 launchd helper、Unix socket 对端身份、安装/取消/卸载；对一个测试 Network Service 写入、回读、恢复；helper 启动普通用户 SystemProxy child 并检测 GUI 实际退出。

**验收**：

- [x] 最低 macOS 目标上无 Developer ID 也可按方案运行；安装目录受保护，对端 UID/进程身份来自系统，不能以自报 PID 或 ad-hoc 证明开发者身份。
- [x] 拒绝授权可继续手动代理；批准后的重复操作不依赖未经验证的授权假设。
- [x] helper 管理 SystemProxy child 时内核使用已验证用户身份，受保护资源可按需读取。
- [x] 强制结束测试 GUI 后，helper 恢复仍归属自己的代理并清理 child；瞬时 IPC 断开不被当成进程死亡。
- [x] 拒绝其他未授权用户和任意命令/路径；管理员权限或系统限制不足时记录具体错误，不把证书缺失列为阻塞。

**当前状态（2026-10-06最终收口）**：原卡五项A/B/C/D/E全部PASS，P0-05 DONE，owner释放。[真实 GUI owner 最终验收](evidence/p0-05/gui-owner-final-20261006-142752/README.md)证明真正GUI PID74759、Host确认ready窗口、五次连接同实例、SIGKILL后owner_NOTE_EXIT、精确恢复、child/组/listener清理、无RecoveryRequired、卸载/finalcleanup与主网络hash不变；13条自动命令exit0（默认GPUI0 tests不计用例PASS），独立Review通过。E消费原卡已有真实拒绝证据，executable/service/root-total-deadline明确optional hardening NOT_RUN，不再作为DONE blocker。仅P0-06/P0-08转READY，不启动、不commit/push。

以下Desktop/新路径段落为各轮当时的历史结果；FAIL/NOT_RUN/ACCEPTANCE不改写。

**Desktop 补验（2026-10-06）**：保持 ACCEPTANCE。标准 macOS 授权 UI 实际出现，两次批准均执行固定 root 安装/卸载；原 linker-signed 与 explicit ad-hoc staging 候选均未建立 daemon socket，内层返回 `launchd helper socket startup timeout`。第一轮 AMFI CT 错误、第二轮 amfid -423 与启动时间线见[本轮真实证据](evidence/p0-05/desktop-privileged-20261006-113208/README.md)；尚未证明完整退出根因，不把证书缺失列为既定阻塞。真实取消、root 日常 IPC/降权 child、完整 Proxies 写回恢复/conflict、GUI SIGKILL 均未完成。独立现场 cleanup PASS，主用网络 hash 不变；5 定向测试及 prototype check/build/clippy/fmt/core check PASS。历史 -60008/NOT_RUN 不改写；P0-06/P0-08 仍 TODO，不启动下游、未 commit/push。

**新路径复验（2026-10-06 12:03 起）**：保持 ACCEPTANCE。[本轮证据](evidence/p0-05/desktop-pathfix-20261006-120343/README.md)取得真实 UI 取消 -128/Host 确认与取消后 manual PASS。批准安装在 bootstrap 前因 `fs::copy` 的业务用户 owner 被保护检查拒绝；同一标准 UI 经固定 SHA/type/空 runtime 等检查归正自有文件后，既有卸载与独立 cleanup PASS。只修复实测复制 bug：helper/kernel 由安装进程新建 0600 文件并仅复制字节，原 hash/mode/protected 检查保留，新增一项回归。14 Rust/2 Python 与指定检查 PASS、独立代码 Review 通过；修复后批准调用 240 秒超时，Host 未看到授权窗口，停止特权分支。新路径 daemon/socket 与完整 A–E 尚未实测；主用网络三组 hash 不变，P0-06/P0-08 仍 TODO，未 commit/push。

<a id="obg-p0-06"></a>
## OBG-P0-06 应用自身出站策略原型

**类型**：网络路径验证；**前置**：OBG-P0-04、OBG-P0-05。**依据**：方案 §8.9。

**执行与交付**：最小 Rust 下载客户端显式选择 Direct/ViaRunningProxy；验证系统代理、TUN、独立 DNS 引导和重定向路径，记录真实出口及失败结果。

**验收**：

- [x] 系统代理/TUN 开关组合中 Direct 的出口与 DNS 有证据，不能只以 `no_proxy()` 作为结论。
- [x] 无订阅或代理失效时仍可尝试直连；不会依赖尚未下载的订阅自举。
- [x] ViaRunningProxy 使用最后有效实例，失败不静默改道；跨源不转发认证头。
- [x] 无法证明 Direct 的组合明确判为不支持或未完成，供 P0 出口决定范围。

**当前状态（2026-10-06）**：DONE，owner释放。复选框表示原卡要求的证据调查/范围判定已完成，不表示所有物理路径PASS。原验收1：OFF/external-TUN-ON真实HTTPS出口和独立DoH证据；SystemProxy ON为UNSUPPORTED_IN_CURRENT_TEST_ENV，TUN OFF及managed bypass为INCOMPLETE_EXTERNAL_TUN_ACTIVE。原验收2/3/4 PASS。保护用户现有TUN，无系统网络写入；本地9项/全core294与303串行/helper47与规定检查通过；独立review PASS、cleanup PASS。完整限制、历史FAIL和证据见[本轮交付](#p0-06-delivery)。P0-08/P2-01仅READY，不启动、不commit/push。

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

- [x] GitHub 下载与手动安装路径明确；不引入 Sparkle、开发者账户或公证流程。
- [x] 明确应用/helper/内核/资源的版本绑定及更新时退出顺序。
- [x] 下载复用显式出站客户端；浏览器下载后的 quarantine 与首次打开步骤有真实记录，未要求关闭系统保护。
- [x] Apple Silicon 产物保留可运行的最低 ad-hoc 签名并验证内嵌二进制；明确不提供发布者身份或公证，校验和不冒充签名。

**2026-10-10 Desktop：ACCEPTANCE。** 工程、真实签名/摘要、Edge loopback下载与ZIP/解压app quarantine已记录；Finder首开仍未取得实际窗口，系统日志有Gatekeeper denial与amfid -423，正常单应用授权/托盘退出未完成。第四项静态ad-hoc检查PASS，但“可运行”尚未验收，复选框不提前勾选；第三项首开要求同样未闭合。GitHub无对应Release/资产，远端下载NOT_RUN，不自行发布。P2-05/P2-06代码与公共契约未改；无commit/merge/push。见[本轮验收](P0-08-acceptance.md)、[安装说明](P0-08-installation.md)。

**2026-10-10 最终 macOS：DONE。** 同一原浏览器 ZIP 在独立 Downloads 目录由 Finder 解压，保留 quarantine，经实际用户正常单应用系统授权后显示真实 GPUI 窗口；输入/主题、关闭、用户托盘恢复/退出及系统 voluntary exit0、全部自有进程清理有实际证据。未重签名、终端启动、关闭保护或安装管理员helper。独立Review无剩余代码Finding，本卡owner/预约释放；GitHub无资产远端下载仍NOT_RUN，不push/发布。上段为首轮历史结论，原失败不改写。见[最终证据](evidence/p0-08/first-open-20261010/README.md)、[Review](evidence/p0-08/first-open-20261010/REVIEW.md)。

<a id="obg-p0-09"></a>
## OBG-P0-09 P0 出口与重新估算

**类型**：阶段验收；**前置**：OBG-P0-01、OBG-P0-02、OBG-P0-03、OBG-P0-04、OBG-P0-05、OBG-P0-06、OBG-P0-07、OBG-P0-08。

**执行与交付**：汇总原型身份/结果、未决项与范围差异；按本清单逐任务重估开发、集成、定向验证、Review 和人工返工，管理员测试环境等外部条件单列；必要时拆细任务并更新依赖。

**验收**：

- [x] 所有影响路线的未知项已有实测结论和明确决定；有未决项则保持本任务未完成。
- [x] 当前范围与任务规则清楚；DNS/路由器缺口有实现或差异决定，无需旧流程批准文件。
- [x] P1–P7 的工作量、假设、风险、排除项可审查；撤回的旧工期没有复用。
- [x] Windows/Linux 单独估算；与 P1 相关的核心原型通过后可开展 P1；权限/TUN 未决项只阻塞依赖它们的功能。


**当前交付（2026-10-10，同一WorkRun接续收口）**：DONE，owner/文档预约释放。用户正式批准首版最低支持macOS15+；P0-08固定内核minOS26.0及尚未15实机验收保留，兼容同1.14.0/必需特性资产取得或构建、最低15真机验证归P7-03/P6-05/P7-05。授权只读主树原历史索引及全文件SHA与Host逐项相等，原证据被忽略、未复制/写入或冒充入Git。52张P1～P7卡五栏正式剩余163–326人日/中点244.5，已含一次4–9增量；Windows7卡49–97、Linux分类条件38–76。四项文档验收及本轮独立Review/纯文档验证通过，历史原型/DONE/FAIL/NOT_RUN不改写。首轮ACCEPTANCE及缺失追溯检查保留于收据历史。见[阶段审计](P0-09-stage-review.md)、[验证](evidence/p0-09/validation.json)、[独立Review](evidence/p0-09/REVIEW.md)。68卡DONE30/ACCEPTANCE1/DOING1/READY0/TODO29/DEFERRED7，P2-05/P2-06原状态及owner不变，P7-05仍TODO。未stage/commit/merge/push/发布；Host再次review后独立提交/合并。

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


<a id="p0-05-delivery"></a>
## P0-05 本地 helper 原型交付与授权缺口（2026-10-04）

Codex /root · Runtime/Platform；起始 HEAD `3111a57`、工作树干净。只领取 P0-05；根 Cargo workspace/lock 本轮单一 owner，core 不依赖 helper/平台 crate，旧产品逻辑无改动。`crates/veyra-helper/` 仅导出显式 `p0-05-prototype` feature 下的隔离二进制；[固定编排](../../tools/p0-05-helper-probe.py)不收集密码，不在 root 阶段下载，不接收任意命令/路径/Network Service/PID。

**状态 ACCEPTANCE，五项未全部满足，不勾选 DONE。** [证据与逐项边界](evidence/p0-05/README.md)：当前 macOS 27 arm64，minos 15.0、adhoc/linker-signed、TeamIdentifier not set、codesign verify PASS；固定 archive/binary digest 匹配；非 root install 拒绝且无受保护写入；无 helper 的真实普通用户 sing-box loopback/controller、继承 FD 兼容性与退出清理 PASS。普通用户 getpeereid/LOCAL_PEERPID、真实 peer PID、IPC EOF 后存活 2 秒、SIGKILL/NOTE_EXIT PASS，仅为 API 可行性，不代替 root helper 网络生命周期。

已尝试标准 `osascript do shell script ... with administrator privileges`。实际退出 1：`com.apple.hiservices-xpcservice` Connection Invalid、`授权失败 (-60008)`；未取得管理员授权、没有管理员测试入口运行或 Network Service 写入。不是用户手动取消证据，不把证书缺失当阻塞，不绕过系统策略。特权安装/保护权限、root peer、不同 UID ACL、降权 child 读取 root-only FD、两轮 SystemConfiguration snapshot/apply/readback/restore、GUI crash 后网络/child 清理及外部改写冲突均 **NOT_RUN**。

实现包含独立 disabled/unassociated service、OS peer 凭据二次校验、root-only config + 降权 child FD、按字段组保留外部改写、写前持久恢复记录、SCPreferences lock 下核对 expected 字段、owner/child NOTE_EXIT 与 RecoveryRequired 保留现场、restore/stop→bootout→delete 的 finally 路径；不能将已编译路径描述为实机通过。5 项纯定向测试保护输入/时限与恢复契约，不启动特权 child 或系统网络；[validation.json](evidence/p0-05/validation.json)区分构建、普通用户实测和 NOT_RUN。

所有本轮普通用户 child 已回收、group/listener 关闭；专用 tmp/staging/archive/config 删除；原型 `/Library` helper/plist/root/socket 与 launchd label 不存在，从未创建测试 Network Service。失败历史保留，未 commit/push。DAG 68 卡不变：DONE 7、ACCEPTANCE 1、READY 1、TODO 52、Windows DEFERRED 7；READY 仅 P1-03，P0-06/P0-08/P0-09 仍 TODO，不启动下游。下一步仅在可安全呈现标准系统管理员 UI 的本机环境，以固定原型入口补特权实测并严格清理。

### P0-05 read_frame 真实复验追加（2026-10-06 13:20）

[旧RecoveryRequired独立清理](evidence/p0-05/desktop-recovery-cleanup-20261006-132006/README.md) PASS 后，[新版真实批准](evidence/p0-05/desktop-readframe-fix-20261006-132202/README.md) 已证明 root安装/新路径daemon/socket/首步Status；read errno22未复现。cycle1 step2精确失败于 `StartSystemProxyTest.child_readiness: child startup timeout; no proxy write`，未到recovery_save/network_write。不猜测根因，不修改源码。最终cleanup独立PASS、主用网络hash unchanged；自动验证30 Rust/2 Python及指定八条命令PASS。完整daily/受管代理恢复/owner SIGKILL/conflict/security rejection仍未完成，P0-05保持ACCEPTANCE、owner保留，P0-06/P0-08 TODO、READY=[]，未启动下游、未commit/push。历史失败与旧RECOVERY现场说明原样保留，当前现场已完全清理。

### P0-05 child readiness诊断复验追加（2026-10-06 13:41）

[本轮真实诊断](evidence/p0-05/desktop-readiness-diagnostics-20261006-134101/README.md)：当前Host修复版本正常构建后仅一次标准批准，安装/新路径daemon/socket/Status PASS；child PID/PGID52916业务身份正确，但731ms exit1，完整stderr给出 `read config at /dev/fd/3: open /dev/fd/3: permission denied`，不是无信息timeout。未到recovery_save/network_write，无managed代理修改。保持ACCEPTANCE，C实际FAIL，完整daily/lifecycle/conflict/security NOT_RUN；不猜修复、不改源码。独立cleanup及主用网络三组hash unchanged PASS，Rust40/Python2与八条命令PASS。历史USER_CANCELLED继续消费，不重复取消或授权；P0-06/P0-08保持TODO、READY=[]，未启动下游、未commit/push。

### P0-05 FD3 pipe真实复验追加（2026-10-06 13:59）

[本轮证据](evidence/p0-05/desktop-fd3-pipe-20261006-135940/README.md)：同一标准UI一次真实批准，当前版本FD3 anonymouspipe实测PASS；磁盘config仍root0600，actual425bytes/hash来自读回同一字节，两轮ready ports/controller auth/business UID/GID/PGID通过，不再出现permission denied。fixed privileged probe全流程内层PASS：两轮exactrestore、CLI owner disconnect/SIGKILL/NOTE_EXIT/reaped/group/listener、externalPAC conflict/finalsnapshot及独立cleanup/mainhash unchanged。47Rust/2Python与八命令PASS，独立Review另列范围。但E尚缺实际executable/service字段及root daemon总消息deadline请求，原卡真实GUIowner未执行，不能把实现/单测/CLI owner替代这些证据；P0-05保持ACCEPTANCE/owner，P0-06/P0-08 TODO、READY=[]。本轮未改源码或历史evidence、未重复授权或取消、未启动下游、未commit/push。

### P0-05 最终真实GUI owner收口（2026-10-06 14:27起）

[真实 GUI owner 最终验收](evidence/p0-05/gui-owner-final-20261006-142752/README.md)补足原卡最后GUI缺口：GUI/marker/helper owner PID74759一致，child74789/ports59100–59101/ready FD3，Host确认实际ready窗口。14:31:12 SIGKILL后owner_NOTE_EXIT、restore_readback/reaped/group_empty true，实例null/无recovery，独立child/group/listener absent；isolated Service完整snapshot精确恢复。标准UI卸载与finalcleanup/main三组hash unchanged PASS。本轮没有源码变更，保留初次shell进程退出NOT_RUN与历史失败。A/B/C/D/E原卡PASS，增强三项optional NOT_RUN；自动验证和独立Review完成，P0-05 ACCEPTANCE→DONE、owner释放；68卡DONE14/READY2/TODO45/DEFERRED7，READY仅P0-06/P0-08且未启动，未commit/push。

<a id="p0-06-delivery"></a>
## P0-06 显式出站原型交付（2026-10-06）

[完整证据与四组合矩阵](evidence/p0-06/20261006-144632/README.md)：仅feature原型，复用subscription/fetch HTTP builder/manual redirect。Direct no_proxy + 固定IP TLS DoH、Via固定调用方current loopback instance；跨源四类source credential清除，无silent fallback，无订阅可自举，失效proxy明确失败。两独立HTTPS endpoint的Direct与自有mixed均返回104.28.196.30；实际route仍utun7，仅EXTERNAL_TUN_ACTIVE_OBSERVATION，不冒充物理Direct/TUN bypass PASS。

现有外部TUN为PROTECTED_EXTERNAL_RESOURCE，未停止/修改/重启/接管，未启动第二个TUN。SystemProxy ON未建立安全active Service写入/精确恢复/control marker条件，UNSUPPORTED；TUN OFF/process_path/auto_detect_interface/bind/FakeIP bypass统一INCOMPLETE_EXTERNAL_TUN_ACTIVE：Existing user TUN is protected and was not modified. 原卡第4项允许明确范围决定，因此本卡DONE；后续消费者不能据此声称受管TUN直连支持。

自有sing-box仅mixed PID84634/PGID84634/port55978，已reaped/group-empty/listener-closed；外部utun身份/配置、默认路由、DNS/系统代理和三组主网络hash相同。完整route-table hash不同、原因未证，不称完整表exact unchanged。后发现Karing provider PID6037的pre-run启动时间和final身份单列，owner→interface仍candidate。P0-05 437文件及卡片区段SHA保持不变。默认全core并行Document Busy历史FAIL、初轮fixture/clippy FAIL保留；全量串行和修订后验证均PASS，独立code-delivery-review PASS。[最新68-card DAG](evidence/p0-06/20261006-144632/dag.json)：DONE15/ACCEPTANCE0/READY2/TODO44/DEFERRED7，READY仅P0-08/P2-01，不启动。
