# P0 阶段验收与剩余工作重新估算

[任务卡](P0-feasibility.md#obg-p0-09) · [唯一状态表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [分期规则](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)

<a id="decision"></a>
## 1 当前结论与身份

2026-10-10，WorkRun `work-23397-1791598715449274-213`；Host 建树 CommandRun `command-23397-1791598722823333-214`。本轮全部写入及通常读取均在 `.worktrees/p0-09-stage-estimate` / `dev/p0-09-stage-estimate`；本次补充读取仅按用户明确授权只读主树历史证据，输入 HEAD `b90f612d828dcb1e957cbd0755f1738f5e43206a`；只读 Git 引用 `refs/heads/codex/dist-react-restore` 同 SHA，未切换/修改主树或其未跟踪缓存。初始隔离树干净。

**P0-09 DONE，owner/文档预约释放。** 用户在同一WorkRun接续指令中正式决定首版最低支持 **macOS15+**，维持兼容目标。兼容同1.14.0/必需特性的资产取得或构建、最低15真机验证归P7-03/P6-05/P7-05；P0-08固定内核minOS26.0原事实和未证明在15运行的边界保留。原始证据的授权只读追溯已完成，四项文档验收及本轮独立Review/校验通过，详见第8～9节。当前DONE30 / ACCEPTANCE1 / DOING1 / READY0 / TODO29 / DEFERRED7，macOS30/61；P2-05 ACCEPTANCE、独立树未合并，P2-06 DOING及原owner不变。P7-05仍缺P7-03/P7-04 DONE，故TODO；READY=[]。

首轮Execution曾保持ACCEPTANCE：最低OS决定未到，隔离Git树不含P0-01/03～07原证据，无法直接追溯。原FAIL_EXISTING_MISSING_EVIDENCE及首轮Review原文留在收据历史段，不能改写为当时已读。接续授权后，本轮只读主树 `docs/openbox-rust-gpui-tasks/evidence/p0-01`、P0-03/04/05最终/06最终/07 README及 `fixtures/p0-01` 必要文本/摘要；五份README全文件UTF8 SHA256与Host给值逐一相等。证据被.gitignore排除、未入Git，不复制目录、不写主树、不操作其缓存；隔离树缺少历史文件仍在引用检查中单列，与原证据真实存在区分。P0-08证据/主分支复验仍从隔离树核读，不重跑原型、GUI、网络或平台验收。

<a id="evidence"></a>
## 2 八项原型的证据、实测结论与限制

下表由隔离树记录、用户授权的主树原始文本及其全文件摘要直接追溯。README中的底层日志/实机结论按原作者证据引用，本轮不重跑或逐个读取所有链接日志；视觉清单内图Hash是历史声明，不冒充本轮逐图重算。原型通过不等于正式生产 Native E2E。各原卡既有 FAIL、REWORK、NOT_RUN 原样保留。

| 原型 | 现存权威记录与历史身份 | 关键结论与首期处理 | 证明边界/缺口 |
| --- | --- | --- | --- |
| P0-01 基线 | [当前最终验收](P0-01-baseline.md#current-acceptance)，源码 `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`；[任务卡](P0-feasibility.md#obg-p0-01) | 77 方法/4流/6页/9分类；13指定GET与四WS取得脱敏样本；8图1280×720/DPR1/Chromium149.0.7827.22、54操作状态断言 | 浏览器真实渲染使用 fixture/mock/阻断外部请求；不证明GPUI真服务。traffic单位在此仍UNKNOWN，由P0-07核实；现存[fixtures](fixtures/p0-01/README.md)可读：77 case（22静态/55 observed）、14当前HTTP case及4当前WS case；历史结构清单69源码指纹仅绑定原commit，不要求匹配已迁移的当前源码。主树原visual-manifest/request-audit/validation已读并Hash；8图身份可追溯清单，未逐图重算Hash或重新视觉验收；browser-session-confirm仍NOT_RUN、清单harness FAIL保留 |
| P0-02 范围规则 | [卡及交付](P0-feasibility.md#obg-p0-02)，2026-10-03用户范围决定；当前[项目规范](../../AGENTS.md)及分期规则可核读 | GPUI/OpenBox视觉迁移；GitHub Releases、手动更新、无发布证书/公证，不恢复旧门禁；无业务调用的设备分发不新增 | 文档动作不计功能实现；管理员权限与运行所需ad-hoc仍有条件，第三方驱动签名不取消 |
| P0-03 UI/托盘 | [卡及人工收口](P0-feasibility.md#obg-p0-03)，2026-10-04 Host/User确认；当前[原型依赖](../../crates/veyra-gpui-prototype/Cargo.toml)锁定gpui-kit=0.7.0、Rust1.99 | 真实中文Input/Modal及托盘用户操作；11真实图、主题/窗口blur/版本组合；保留可复用基础能力 | 主树原README全文件已读/Hash；起始029578c、人工批准及11图事实由原索引追溯，未逐图/候选帧复验；人工确认不是自动IME候选帧。MiSans/Emoji fallback、nested dropdown/card backdrop及macOS15实机未证明 |
| P0-04 内核/缓存 | [最终交付](P0-feasibility.md#p0-04-delivery)，起始`19696abde8a27a36c4eb490a4236006fcd09c9cc`，官方1.14.0 darwin-arm64 | 5配置check、4真实child、鉴权动态controller、selector/FakeIP同配置恢复与writer关闭后handoff；最终脱敏重跑restart13.433084ms、handoff13.750209ms均有实际对账前流量窗口；原卡22.990292/14.466458ms为首轮历史值 | TUN只check；同版本/cache_id/tag/pool/range，不证明正式Runtime或动态切网；就绪前不接管系统代理。主树README已读/Hash，原archive/binary摘要与P0-08固定身份相等；原两次check FAIL保留，未重新启动内核 |
| P0-05 提权helper | [最终卡](P0-feasibility.md#obg-p0-05)及“最终真实GUI owner收口”，历史起始`3111a57` | 真实标准UI取消/批准安装/卸载、root daemon、普通用户child、FD3匿名pipe；GUI/marker/helper owner74759一致，child74789，SIGKILL→NOTE_EXIT→完整隔离Service恢复/进程组/监听清理；A～E原卡PASS | 仅显式原型feature/入口。executable/service/root-total-deadline增强仍optional NOT_RUN；未证明macOS15真机。主树最终README已读/Hash，最终基线18a54d26、helper签名后SHA与GUI身份详见2.2；P2-06生产IPC/GUI/Native仍DOING，不以本原型补勾 |
| P0-06 自身出站 | [最终交付](P0-feasibility.md#p0-06-delivery)，历史HEAD `18a54d26ca171c1b00cc497b233c6f493aac43a9`，20261006-144632批 | no_proxy+固定IP TLS DoH、Via当前自有mixed、跨源凭据剥离、无静默fallback；OFF/external TUN ON下两HTTPS endpoint及DNS有观测 | 实际route仍utun7；外部owner/interface仅candidate。SystemProxy ON=`UNSUPPORTED_IN_CURRENT_TEST_ENV`；TUN OFF/managed bypass=`INCOMPLETE_EXTERNAL_TUN_ACTIVE`。不是物理Direct PASS；完整route-table hash变化原因未证。主树最终README已读/Hash；记录实际自有mixed PID84634、port55978与9项新定向测试，未重新联网/动外部TUN |
| P0-07 Observation/DNS | [最终交付](P0-feasibility.md#p0-07-delivery)，历史起始`fac6f12`；[方案DNS/诊断](../openbox-rust-gpui-implementation-plan.md#86-dns) | 四WS；traffic为约1秒区间bytes，重连有缺口，短请求可能未进入连接快照；DNS records UNSUPPORTED、flush SUPPORTED、hot rewrite UNSUPPORTED；受管HTTP200只关联实际字段 | 无稳定DNS query ID/source/elapsed/resolver链/process/filter hit；debug match[1]非稳定rule ID。主树原README已读/Hash；四流101、traffic/短连接/DNS三项及缺失字段由真实原索引追溯；不自造DNS内核/查询记录；[探针源码](../../tools/p0-07-observation-probe.py)存在但未执行，不能代替原日志 |
| P0-08 更新/分发 | [验收](P0-08-acceptance.md)、[首开最终证据](evidence/p0-08/first-open-20261010/README.md)、[主分支复验](evidence/p0-08/main-revalidation-20261010/README.md)；开发基线`733b6ea3d20b23ba90bb933971a08b076bc0caa3` | updater/包各8项、固定版本/大小/摘要/取消/保旧；真实Edge loopback下载、Finder新解压保留quarantine、用户单应用授权、GPUI交互、关闭→同PID托盘恢复→voluntary exit0，全部自有PID清理 | `PASS_NATIVE_PROTOTYPE`而非正式Veyra发行；helper仅嵌入，内核仅version。GitHub远端无资产=`NOT_RUN_NO_RELEASE_ASSETS`；中间授权/托盘菜单未截图；首轮Gatekeeper FAIL保留。仅macOS27.0.1 arm64实际首开，整体minOS26.0不证明15或26真机 |

P0-08 ZIP=54,830,834 bytes / SHA256 `fdb7472d4792bf4f7f88568f3c1f288746da5efcf55cdf0bded7b28262221a26`；实际GUI主程序SHA256 `367e25f453abc580b779ee7508f3f050b71074ae9f57c7e11188f54b3d640954`，PID16877。读取[构建收据](evidence/p0-08/desktop-20261010/build-receipt.json)、[ZIP身份](evidence/p0-08/first-open-20261010/zip-identity.json)、[运行身份](evidence/p0-08/first-open-20261010/running-identity.json)可追溯这些值；本轮只核对收据，不访问树外ZIP/App或执行签名/OS命令。

P0-08提交`f0fb874805c95b251b1b90465a8336cf302902dd`，首合并`4e8f9cb92e93b40bfb3fc272d1382062d74dc6d6`；主分支首次更新测试7/8 FAIL（目录碰撞）保留。测试夹具修复`8e7bfded9650157eed456aa26cb6ad237861adc6`、代码合并`1cd477eeaf0bd4f1371d0504b530f67aae1c6613`后更新8/8/build/Core Clippy/fmt PASS，包代码未变复用首次主分支8/8。当前b90f612d是后续文档收口输入，不能说生产测试在b90f612d重新跑过；[最后独立Review](evidence/p0-08/main-revalidation-20261010/REVIEW.md)亦只核读证据。Legacy Windows资源Clippy FAIL和GitHub NOT_RUN不改写。

### 2.1 本轮核读文件的 SHA256

下列摘要绑定本轮实际读取的文本/收据，主树原文件使用绝对本地链接以标明只读位置；不是Git已收录声明。P0-01～08卡正文区段摘要以输入HEAD计算，本轮仅修改P0-09段；验证检查这些区段字节保持不变。

<!-- 文本摘要表由本轮本地读取计算；不表示重新运行原型。 -->

| 可核读文件/区段 | SHA256 |
| --- | --- |
| [P0-01-baseline.md](P0-01-baseline.md) | `aae39577b57e330b67ddcfa33a95ecf1eb9e567be30d03faed45762ac6dae585` |
| [evidence/p0-08/desktop-20261010/build-receipt.json](evidence/p0-08/desktop-20261010/build-receipt.json) | `6cd2eede9d8e2605965e48b02d2e10292db2ae44d1c47022774ce0f25c372bea` |
| [evidence/p0-08/first-open-20261010/zip-identity.json](evidence/p0-08/first-open-20261010/zip-identity.json) | `1207c8757a6d743cb0c2627836d15e6e3ebef45c75733ec91e6ab65f3f412778` |
| [evidence/p0-08/first-open-20261010/running-identity.json](evidence/p0-08/first-open-20261010/running-identity.json) | `f14dbc990d6048b4f6c8403fa75076bcaf5a3b6e5ba08531c5c282c8b0091b6e` |
| [evidence/p0-08/main-revalidation-20261010/README.md](evidence/p0-08/main-revalidation-20261010/README.md) | `7669b4f3b837defcace45e6001c82f5e64dc22916d444518d14369fcb1cc2fa6` |
| [fixtures/p0-01/source-inventory.json](fixtures/p0-01/source-inventory.json) | `ab8befa872ec8ca9a95b3451b7d66d3a38c5774a3dd20f8325618b4f9b819ab0` |
| [fixtures/p0-01/observed-current-http.json](fixtures/p0-01/observed-current-http.json) | `eb9f8a30bc2073a090b33d77f5188cc91a7dc540f1cabba44209be6a6222c312` |
| [fixtures/p0-01/observed-current-ws.json](fixtures/p0-01/observed-current-ws.json) | `23d0b6b253842d5034effc41a9d6c5368153bb817fec962a1fc50a5d26adebab` |
| [fixtures/p0-01/visual-data.json](fixtures/p0-01/visual-data.json) | `a013228b338b2cd09db1b79a92907dc80cc19e0d69e0dfd11fed654117cb9608` |
| [P0-01 当前卡区段](P0-feasibility.md#obg-p0-01) | `18df9f2f1e21217e6d77ddb3412dbe83d50495ae06a5f84a1ae87ec7794f21ea` |
| [P0-02 当前卡区段](P0-feasibility.md#obg-p0-02) | `5070a603956f3a2181daa9942d1bcbf5e95ecf38ec39a5f7b2dc88fd002d7d35` |
| [P0-03 当前卡区段](P0-feasibility.md#obg-p0-03) | `db63ccd93a9d40df4a47da9966504612912a4518d22c50040e69a19bb51f7aa8` |
| [P0-04 当前卡区段](P0-feasibility.md#obg-p0-04) | `bad90b989c916a4bc42ca1c913a5acda438c3925ffcbd06b0fc7e0f1fc4abef3` |
| [P0-05 当前卡区段](P0-feasibility.md#obg-p0-05) | `ec8f59870a61457494331257f94ce379718456558c9cc51fa7bed2084cebc16c` |
| [P0-06 当前卡区段](P0-feasibility.md#obg-p0-06) | `ffe4674a88dc563710fd540c96699975f8e150193bdaba75162e7ca871c8004b` |
| [P0-07 当前卡区段](P0-feasibility.md#obg-p0-07) | `939cfc3708e87562cf04a457a89e6cfa18109dd52d5600769b75d7192ac3ff61` |
| [P0-08 当前卡区段](P0-feasibility.md#obg-p0-08) | `cd7c1bc047bfac06ebd1d60db60bc1bfbeb9ab9c40d27ee46a349c34d5274a54` |
| [主树原文件 evidence/p0-01/visual-manifest.json](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-01/visual-manifest.json) | `06694aefbfddba2feca6e6ea7454505f7d93c21eec3fbb8632058e92b451737f` |
| [主树原文件 evidence/p0-01/request-audit.json](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-01/request-audit.json) | `736ecf67d84a51f1d7e03d7f38779a71896429060d94ed7aadecbe59c332266e` |
| [主树原文件 evidence/p0-01/validation.json](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-01/validation.json) | `07aecd30ca54031bd51ec50e6d34611626f6db2bab1502935f9ae1570e766956` |
| [主树原文件 evidence/p0-01/browser-session-confirm.json](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-01/browser-session-confirm.json) | `2d511adad4d84c1eaa72e2801d500ddbd3c8592e6c5444bad190e4110978d350` |
| [主树原文件 evidence/p0-03/README.md](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-03/README.md) | `a01e39c269b25c048f03a712ed979293fe566a5d3d90135defb2cf6799a87b28` |
| [主树原文件 evidence/p0-04/README.md](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-04/README.md) | `d324a371a09d27774e382be2fe9454392448cacb0362b695af1e3a3b0ae200cb` |
| [主树原文件 evidence/p0-05/gui-owner-final-20261006-142752/README.md](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-05/gui-owner-final-20261006-142752/README.md) | `c738ba4c883b844d109e2cd2124c78f1487ce099f00665232691f6c82c45e21c` |
| [主树原文件 evidence/p0-06/20261006-144632/README.md](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-06/20261006-144632/README.md) | `1478cdde951e328337a1e784adaf7f5b14a40517224e18b30357d75d57beb850` |
| [主树原文件 evidence/p0-07/README.md](/Users/lifeilin/wx_lifeilin/github.com/lifei6671/veyra/docs/openbox-rust-gpui-tasks/evidence/p0-07/README.md) | `0141494b01c83d5a8d647ada775d78757aad1a7039b52eb7d1bc8e8ecaa1e36f` |

### 2.2 补读原索引后的事实核对

P0-01 request-audit记录21次真实参考环境请求（含两次login、15 GET、4 WS）及450次本地视觉请求；visual-manifest明确所有视觉API fulfilled locally/WS mocked/外部请求阻断，54 PASS和一次harness FAIL同时保留。原validation的77 API/69源码Hash/8图与当前基线一致；一次旧浏览器启动确认NOT_RUN不替代后续真实浏览器渲染。本轮只读JSON，未访问任何接口或查看GUI。

P0-03原索引为起始029578c81554f2aeef9bb4e0fd6aaa8a19defd55、gpui-kit0.7.0/gpui-pre-macos0.3.7；中文composition依用户直接确认，不以候选帧截图证明；真实10000行合成数据不等于性能基准/一比一页面完成。窗口NSVisualEffectView可行不证明卡片backdrop，原Windows资源workspace check FAIL保留。

P0-04原索引最终脱敏重跑时序替代首轮时序作为本审计的最终实测值，原卡历史段不改。缓存writer关闭后handoff SHA `a0749679a4b28dd2fbe44bb1d70ce72b3d6170fa9457cefec1c37b640dd5d7f8`，selector b、FakeIP .3与空缓存a/.2对照；未证明跨版本/UID或正式Runtime。

P0-05最终索引绑定helper SHA `30a6c3a53cc51d903a8b03b5e7f54464172a0b72482f72832d44302de54092de`、签名后GUI SHA `f9b649646690b42c02294d0c67bea354811b95e386d638a44388b9b4f4d526d1`；GUI/marker/helper owner74759、child74789、5次socket短连接仍同实例、14:31:12 SIGKILL后的NOTE_EXIT/完整隔离Service恢复及标准UI卸载。原GUI图仅在会话中，不虚构落盘图Hash；旧root/不同UID拒绝结果为原证据消费，增强NOT_RUN保留。P2-06生产未完成不受本原型替代。

P0-06原索引仅OFF/external TUN ON；no_proxy+固定IP TLS DoH仍走utun7，外部owner→interface仅candidate，完整route-table Hash变化原因未知。两HTTPS同出口不证明真正物理Direct。P0-07原索引四流帧数connections21/logs72/memory22/traffic5+16；两burst区间bytes与REST delta逐项对账（131224/524449、65687/262305），3.688625ms短请求未进快照；DNS flush204且自有上游请求1→2/答案.10→.20，records404及hot update无实际变化。受管HTTP200仍缺结构化DNS事件/解析链/进程/过滤命中，不按配置填预测值。首轮port checker FAIL、后续脱敏重跑及正常停止均在原索引保留。

<a id="scope"></a>
## 3 首期范围、能力差异及最低系统决定

DNS沿用P0-07及方案既有决定：记录显示“当前内核不提供DNS查询记录”，无来源不伪零、不从connections/日志推造记录；重写/规则/hosts/predefined配置保存成功后显示“已保存，需重启应用配置”（`RestartRequired`），不承诺热更。`POST /cache/dns/flush`只清当前受管实例response cache，成功才反馈已清理；UI历史、FakeIP reset、系统DNS均不同。P5-01/02实现真配置/资源/测试及失败保旧；P5-03负责不可用原因、flush及Rules诊断缺口，P5-07/P7-04复验披露。受管诊断缺少DNS事件/解析链/进程/过滤命中时如实标“不完整”，配置预测与真实访问分开，TCP成功不冒充HTTP/TLS成功。

路由器专属MAC、LAN模拟、系统脚本、网关/前置旁路按[既有平台边界](../openbox-rust-gpui-implementation-plan.md#115-平台能力矩阵)保留导入配置并明确不支持；macOS不错误应用、不自行补路由器服务。P4-05B/C与P7导入/发布说明承接，未增加卡或改依赖。

P0-06明确只证明SystemProxy OFF/external TUN ON的HTTP客户端与出口/DNS观察；外部TUN不能绕过。SystemProxy ON真实出口由P2-05/P2-07及P2-09组合落实，TUN OFF与受管bypass由P6-01/05落实；无法证明物理Direct或独立DNS时拒绝并说明，不能把`no_proxy()`当直连证明。外部TUN不接管/停止；需专用隔离机器/授权时保留NOT_RUN，不能替代依赖DONE。

P0-05真实提权原型支持路线可行性；正式helper/IPC/GUI、root降权child、多轮及恢复缺口仍归P2-06原owner。P0-08本地下载包的真实首开/托盘通过不代替正式生产包或GitHub实际资产。P6-04/05、P7-03/05以后须在明确发布授权且资产存在时完成真实下载/升级/回退/设备验收，本轮不发布，也不将授权等待算开发人日。

### 3.1 macOS 15 与固定内核 minOS 26.0 冲突

[构建收据](evidence/p0-08/desktop-20261010/build-receipt.json)明确：GUI/helper/updater Mach-O minOS15.0，固定`veyra-sing-box` minOS26.0；整包取最大值26.0，安装说明/发行fixture亦为26.0。固定内核1.14.0、Revision `0b8995879f29a9b98ee027bc17b75e101445b238`、Go1.26.7 darwin/arm64、CGO enabled；archive SHA256 `a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`，最终内嵌binary SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。tags见原收据，不推断CGO/工具链就是根因。

方案 §1最低支持macOS15+已由本轮用户正式决定，兼容整包仍未验收。设置应用deployment target15或改Info.plist不能降低现有内核Mach-O的最低要求，也不能修改现有二进制头后声称兼容。原型在27.0.1运行只证明该设备，不证明Apple Silicon整体macOS15支持，甚至26.0设备正式包仍须独立验证。P0-05卡历史“最低目标”勾选沿用原验收，不能外推为macOS15真机已验收。

**已批准路线：首版最低支持macOS15+，保留产品兼容目标；当前产物兼容尚未验收。** P7-03在现有包构建职责内取得可追溯的同1.14.0/必需特性、minOS≤15构建：官方兼容资产若存在先核实，否则按固定source revision/许可证可重现构建；保留toolchain/SDK/tags/CGO/摘要并重新绑定四组件及包身份，只做Veyra所用check/controller/生命周期定向检查，不patch内核或删必需特性。P6-05在macOS15 Apple Silicon独立机器验证平台权限/系统代理/TUN/恢复，P7-05用同一最终候选真实下载包补首开/首用/升级回退/退出。P6-05的最低系统平台复验可以使用P7-03内部准备的兼容测试资产，不把P7-03卡DONE反加为P6-05前置；最终同候选跨卡一致性由P7-05验收，既有DAG不变。

未取得兼容产物或最低设备结果之前，禁止声明当前包已支持15，不能用Info.plist/deployment target或二进制头改写替代兼容。P0-08原Hash/minOS26.0/27.0.1实测保持；本轮兼容资产可得性、源码构建与15真机全部NOT_RUN。若同版本必需特性无法兼容，应带具体失败提交路线调整决定，不能静默升为26+。无需发布证书/公证；真实发布必须另有授权，本轮不发布。兼容增量4–9已分配进第4节既有三卡和总估算，第5.1节仅说明分配，不重复计费。

<a id="estimate"></a>
## 4 逐卡剩余人日估算口径

1人日=8小时有效工作；五栏为“开发、集成、定向验证、独立Review、人工视觉/平台返工”，每栏上下界、总计上下界与中点估计均可加总。中点是预算点值而非概率期望，区间是范围情景而非统计置信区间。基于卡的真实范围与当前主表状态自下而上估计，不复用已撤回61–97或21–32数字，不虚构实际已花人日。

开发包括该卡真实服务/持久化/界面实现；集成包括跨层接线和资源/版本；定向验证包括适用业务回归、真实磁盘重建及受控集成；独立Review包括另一审查者核读及复核；最后一栏包括真人操作/截图比对/平台演练及预计针对性修复复测。功能卡150%浅色优先，最终组合卡补全要求的主题/缩放和实际GUI E2E。组合卡开发=0，发现缺陷的修复预算在最后一栏；正常集成和验证分开计，不把前期功能转嫁组合卡。

DONE五栏=0仅表示该卡当前验收范围没有剩余工作；既有未运行的跨页/深色/缩放/正式平台组合在已有承接卡计量，未重新打开已DONE卡。P2-05仅按主表和SESSION“独立树未合并/待完整网络组合”估剩余整合/验证，不访问那棵树或重估完整客户端；其owner核验如发现未完成开发须更新本估算。P2-06按已记录checkpoint/正式观测接线扣除已完部分，只计未完成生产GUI/Native/Helper恢复，不重写IPC。工作点值不代表批准或功能完成。

每行链接到对应卡，显式依赖由[总表](IMPLEMENTATION_PHASES.md#4-任务状态总表)和原卡定义，本轮逐项比较一致且不变；无需复制另一张依赖表。仅依赖DONE可领取，未合并ACCEPTANCE不满足依赖。公共领域/DTO/workspace/Runtime单owner与共享UI文件使可并行程度小于泳道数；表中上下界按既有模式可复用、通常一轮定向修复至数轮视觉/真机返工假设，超出范围的新功能需另估。

### 4.1 P1逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P1-01 核心抽取](P1-core-and-shell.md#obg-p1-01) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-02 类型/持久化/版本](P1-core-and-shell.md#obg-p1-02) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-03 GPUI 壳与状态桥](P1-core-and-shell.md#obg-p1-03) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-04A 视觉桌面偏好/主题/组件](P1-core-and-shell.md#obg-p1-04a) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-04B 跨页面行为偏好](P1-core-and-shell.md#obg-p1-04b) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-05 目录/单实例/文件](P1-core-and-shell.md#obg-p1-05) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-06 托盘与关闭](P1-core-and-shell.md#obg-p1-06) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P1-07 桌面壳验收](P1-core-and-shell.md#obg-p1-07) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
### 4.2 P2逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P2-01 订阅基础闭环](P2-local-proxy.md#obg-p2-01) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P2-02A Base OutboundCatalog/OutboundId](P2-local-proxy.md#obg-p2-02a) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P2-02B 最小 Compiler](P2-local-proxy.md#obg-p2-02b) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P2-03 Runtime/服务状态](P2-local-proxy.md#obg-p2-03) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P2-04 选择/缓存/成功记录](P2-local-proxy.md#obg-p2-04) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P2-05 自身出站客户端](P2-local-proxy.md#obg-p2-05) / ACCEPTANCE | 0 | 0.5–1 | 1–2 | 0.5–1 | 0.5–1 | 2.5–5；3.75 | 仅整合/验收；独立树未合并，待 owner 核实，无新增完整客户端开发 |
| [P2-06 helper/IPC](P2-local-proxy.md#obg-p2-06) / DOING | 1–2 | 2–4 | 2–4 | 0.75–1.5 | 1.5–3 | 7.25–14.5；10.875 | 仅剩 GUI 消费、生产权限/降权、Helper多轮/重启恢复；既有IPC和观测接线不重做 |
| [P2-07 系统代理与恢复](P2-local-proxy.md#obg-p2-07) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 消费生产helper；多服务接管、外部改值条件恢复与真实出口风险 |
| [P2-08 选择与节点测速](P2-local-proxy.md#obg-p2-08) / TODO | 3–5 | 1–2 | 1–2 | 0.5–1 | 1–2 | 6.5–12；9.25 | 复用唯一目录/选择入口；真实测速、历史、草稿临时实例清理与卡片误触 |
| [P2-09 首个可用版本验收](P2-local-proxy.md#obg-p2-09) / TODO | 0 | 0.5–1 | 1–2 | 0.5–1 | 1–2 | 3–6；4.5 | 同一构建闭环和正式SystemProxy；局部证据复用，首次功能实现不转入此卡 |
### 4.3 P3逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P3-01 观测与事件](P3-observability.md#obg-p3-01) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P3-02 连接](P3-observability.md#obg-p3-02) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 连接稳定身份/历史100/断开/GeoIP与偏好；5000行实测和详情视觉风险 |
| [P3-03 日志](P3-observability.md#obg-p3-03) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P3-04 统计存储](P3-observability.md#obg-p3-04) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P3-05 概览/站点测速](P3-observability.md#obg-p3-05) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 真实指标/站点测速和持久历史；图表时间轴、单位与未知状态 |
| [P3-06 历史/下钻/容量设置](P3-observability.md#obg-p3-06) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 真实SQLite查询/下钻/保留设置；取消不影响writer、旧响应及容量边界 |
| [P3-07 代理基础视图/已加载规则](P3-observability.md#obg-p3-07) / TODO | 1.5–3 | 0.75–1.5 | 0.75–1.5 | 0.5–1 | 1–2 | 4.5–9；6.75 | 基础代理/已加载规则；1000节点与偏好，不新建无调用的页面 |
| [P3-08 主页面基础能力验收](P3-observability.md#obg-p3-08) / TODO | 0 | 0.5–1 | 1.5–3 | 0.5–1 | 1–2 | 3.5–7；5.25 | 容量/长时/性能、跨页和其它主题缩放；已DONE采集/日志/存储不重估 |
### 4.4 P4逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P4-01 订阅高级项](P4-configuration.md#obg-p4-01) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 高级字段、刷新调度/稳定身份；睡眠组合留P6-02/05，不另造调度器 |
| [P4-02 静态/动态组](P4-configuration.md#obg-p4-02) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P4-03 failover](P4-configuration.md#obg-p4-03) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P4-04 规则资源](P4-configuration.md#obg-p4-04) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | JSON/SRS资源版本/分页/下载保旧；last-applied引用不能清理 |
| [P4-06 链式代理](P4-configuration.md#obg-p4-06) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 链保存注册/图校验、同候选测速和出口；取消清理，复用Compiler |
| [P4-05A 目标分流/统一目录](P4-configuration.md#obg-p4-05a) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 统一目录的目标分流、顺序/兜底/资源；真实保存应用重建 |
| [P4-05B 终端分流](P4-configuration.md#obg-p4-05b) / TODO | 1.5–3 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5–10；7.5 | 当前Observation终端投影；MAC/LAN不可用，保留数据及真实来源风险 |
| [P4-05C Routing/Terminal 基础诊断](P4-configuration.md#obg-p4-05c) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 配置预测与GET/HEAD/TCP/TLS实测；版本绑定与目录跨页面消费，缺DNS标不完整 |
| [P4-07 配置阶段验收](P4-configuration.md#obg-p4-07) / TODO | 0 | 0.75–1.5 | 1.5–3 | 0.5–1 | 1–2 | 3.75–7.5；5.625 | 跨模块主备/链/规则及应用失败保旧、其它主题缩放；不开发第二份目录 |
### 4.5 P5逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P5-01 DNS 上游/重写](P5-dns-and-sharing.md#obg-p5-01) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 测试输入绑定DNS patch；锁定格式与RestartRequired，无热改接口开发 |
| [P5-02 DNS 过滤](P5-dns-and-sharing.md#obg-p5-02) / TODO | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 过滤语法规范化/unsupported/资源版本；失败保旧与真实应用 |
| [P5-03 DNS 观测/热更边界](P5-dns-and-sharing.md#obg-p5-03) / TODO | 1–2 | 1–2 | 1–2 | 0.5–1 | 0.75–1.5 | 4.25–8.5；6.375 | 记录不可用UI、当前实例flush、Rules缺字段；不造DNS流或内核 |
| [P5-04 五种共享入站](P5-dns-and-sharing.md#obg-p5-04) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P5-05 共享 UI/URI](P5-dns-and-sharing.md#obg-p5-05) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P5-06 订阅分享](P5-dns-and-sharing.md#obg-p5-06) / DONE | 0 | 0 | 0 | 0 | 0 | 0；0 | 剩余0；沿用主表当前验收边界，未记录/未估实际已花人日 |
| [P5-07 DNS/共享验收](P5-dns-and-sharing.md#obg-p5-07) / TODO | 0 | 0.75–1.5 | 1.5–3 | 0.5–1 | 1–2 | 3.75–7.5；5.625 | DNS/共享/Rules组合和其它主题缩放；五入站及分享已DONE部分只复验 |
### 4.6 P6逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P6-01 TUN/模式交接](P6-macos-lifecycle.md#obg-p6-01) / TODO | 2–4 | 1.5–3 | 1.5–3 | 0.75–1.5 | 1.5–3 | 7.25–14.5；10.875 | 普通↔SystemProxy↔TUN交接、唯一writer及真实Direct/引导DNS；外部TUN隔离 |
| [P6-02 睡眠/切网/恢复](P6-macos-lifecycle.md#obg-p6-02) / TODO | 2–4 | 1–2 | 1.5–3 | 0.75–1.5 | 1.5–3 | 6.75–13.5；10.125 | 睡眠/切网/恢复与RecoveryRequired；真机环境易产生返工 |
| [P6-03 登录启动/退出](P6-macos-lifecycle.md#obg-p6-03) / TODO | 1–2 | 0.75–1.5 | 0.75–1.5 | 0.5–1 | 1–2 | 4–8；6 | 登录项、统一退出、调度/分享清理；Finder/登录/托盘结果分开 |
| [P6-04 更新集成](P6-macos-lifecycle.md#obg-p6-04) / TODO | 1.5–3 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5–10；7.5 | 复用P0更新契约接生产UI/helper升级；未授权发布不计已完成 |
| [P6-05 macOS 平台验收](P6-macos-lifecycle.md#obg-p6-05) / TODO | 0 | 1–2 | 2.5–5 | 0.75–1.5 | 2–4.5 | 6.25–13；9.625 | 最低macOS15平台权限/系统代理/TUN/恢复复验，新增1–2.5已计入；与P7-05同候选最终验收分工 |
### 4.7 P7逐卡

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [P7-01 备份与导入](P7-data-and-release.md#obg-p7-01) / TODO | 2–4 | 1–2 | 1.5–3 | 0.75–1.5 | 1–2 | 6.25–12.5；9.375 | 稳定schema备份/预览/append引用重映射/原子导入；重建与失败保旧 |
| [P7-02 诊断/重置](P7-data-and-release.md#obg-p7-02) / TODO | 1.5–3 | 1–2 | 1–2 | 0.75–1.5 | 1–2 | 5.25–10.5；7.875 | 脱敏导出/10秒重置；停止或恢复失败终止删除，自有隔离数据 |
| [P7-03 GitHub 包/手动升级](P7-data-and-release.md#obg-p7-03) / TODO | 2–4 | 1.5–3 | 1.5–3 | 1–2 | 1–2 | 7–14；10.5 | 兼容同1.14.0/必需特性资产取得或构建及包身份绑定、失败回退；新增2.5–5已计入，远端发布待授权 |
| [P7-04 全范围验收](P7-data-and-release.md#obg-p7-04) / TODO | 0 | 1–2 | 2–4 | 1–2 | 2–4 | 6–12；9 | 77API/4流/19场景、6页9分类实际统一E2E；全主题缩放并真实截图 |
| [P7-05 macOS 候选版验收](P7-data-and-release.md#obg-p7-05) / TODO | 0 | 0.5–1 | 1.5–3 | 0.75–1.5 | 2–4.5 | 4.75–10；7.375 | 同一最终候选在macOS15真实下载/首开/首用/恢复/升级/退出；新增0.5–1.5已计入，仍缺P7-03/P7-04 DONE |

<a id="summary"></a>
## 5 阶段汇总与显式DAG关键路径

P1～P7共52卡，21卡已DONE剩余0、其余31卡计剩余工作；P0的9卡另属阶段审计，不混入功能工时。以下是工作量合计，不是按编号串行的排期，也不是已完成百分比。

| 阶段 | 卡数 / DONE | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| P1 | 8 / 8 | 0 | 0 | 0 | 0 | 0 | 0；0 |
| P2 | 10 / 5 | 6–11 | 5–10 | 6–12 | 2.75–5.5 | 5–10 | 24.75–48.5；36.625 |
| P3 | 8 / 3 | 7.5–15 | 4.25–8.5 | 5.25–10.5 | 2.5–5 | 5–10 | 24.5–49；36.75 |
| P4 | 9 / 2 | 11.5–23 | 6.75–13.5 | 7.5–15 | 3.5–7 | 7–14 | 36.25–72.5；54.375 |
| P5 | 7 / 3 | 5–10 | 3.75–7.5 | 4.5–9 | 2–4 | 3.75–7.5 | 19–38；28.5 |
| P6 | 5 / 0 | 6.5–13 | 5.25–10.5 | 7.25–14.5 | 3.25–6.5 | 7–14.5 | 29.25–59；44.125 |
| P7 | 5 / 0 | 5.5–11 | 5–10 | 7.5–15 | 4.25–8.5 | 7–14.5 | 29.25–59；44.125 |
| P1～P7 | 52 / 21 | 42–83 | 30–60 | 38–76 | 18.25–36.5 | 34.75–70.5 | 163–326；244.5 |

以每卡五栏合计为节点权重，已DONE=0；最早工作完成值 `E(v)=w(v)+max(E(显式前置))`。P0-09本轮DONE、文档剩余权重0；P2-05 ACCEPTANCE及P2-06 DOING仅用剩余权重计算，不当作已满足依赖。对每张卡逐项计算，组合终点如下。

| 终点 | 最长依赖链工作量 下界–上界；中点 | 中点控制链（省略权重0的已DONE前置） |
| --- | --- | --- |
| P2-09 | 15.75–31.5；23.625 | P2-06 → P2-07 → P2-09 |
| P3-08 | 18–35；26.5 | P2-05 → P2-08 → P3-02 → P3-08 |
| P4-07 | 34.25–67.5；50.875 | P2-05 → P2-08 → P4-06 → P4-05A → P4-05B → P4-05C → P4-07 |
| P5-07 | 38.5–76；57.25 | P2-05 → P2-08 → P4-06 → P4-05A → P4-05B → P4-05C → P5-03 → P5-07 |
| P6-05 | 38–76.5；57.25 | P2-06 → P2-07 → P6-01 → P6-02 → P6-04 → P6-05 |
| P7-05 | 49.75–100.5；75.125 | P2-06 → P2-07 → P6-01 → P6-02 → P6-04 → P6-05 → P7-03 → P7-05 |

关键链与资源冲突需分别判断：P2-05验收/合并是网络/大部分业务的前置，P2-06是平台链的前置；其一未DONE不能启动消费卡。P2-05完成后P2-08、P3-05、P4-01、P4-04、P5-01的已满足依赖可重算READY；P2-06/P2-05均DONE后P2-07才可READY。P4-06先注册链出口，再P4-05A→B→C；P7-01要等持久schema前置，不等待Updater行为；P7-05仍须P0-09/P7-03/P7-04全部DONE。本轮未修改任何依赖。

macOS正式剩余 **163–326人日，中点244.5**，已含macOS15兼容增量。假设四个全时贡献者且无资源冲突，纯工作量/4=40.75–81.5个工作日，DAG理想最长链49.75–100.5更长；中点max(244.5/4,75.125)=75.125个工作日。这个模型把每卡五栏按同一顺序完成，并假设审查者/机器总可用，不能作承诺日历工期。单一贡献者仅工作负载为163–326工作日，还需另一审查者；真实排期另受单owner/共享文件串行、最低15机器、管理员/发布授权和环境等待影响，时长未定。兼容增量使中点控制链从业务配置链转为平台/兼容包链；未另加无依据百分比缓冲。

### 5.1 macOS15兼容增量分配（已计入主表，勿再加总）

<!-- 首轮主范围159–317/中点238；用户决定后的4–9仅分配一次。 -->

| 已有卡 / 新增职责 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 增量合计；中点 |
| --- | --- | --- | --- | --- | --- | --- |
| P7-03 兼容同版本内核取得/构建及身份绑定 | 1–2 | 0.5–1 | 0.5–1 | 0.5–1 | 0 | 2.5–5；3.75 |
| P6-05 最低15平台定向复验 | 0 | 0 | 0.5–1 | 0 | 0.5–1.5 | 1–2.5；1.75 |
| P7-05 同一候选最低15首开/首用/恢复/升级/退出 | 0 | 0 | 0 | 0 | 0.5–1.5 | 0.5–1.5；1 |
| 已计入增量合计 | 1–2 | 0.5–1 | 1–2 | 0.5–1 | 1–3 | 4–9；6.5 |

首轮159–317 + 本次4–9 = 正式163–326，中点238 + 6.5 = 244.5。仅覆盖一次可行的兼容资产取得/构建与最低机器复验，不包含内核补丁、删特性、协议认证或无限工具链排障；若失败需具体证据与路线决定。最低设备取得/排队是外部等待；不新增Task、不改依赖，兼容未实测不等于P0-09文档未完成。

<a id="platforms"></a>
## 6 Windows独立逐卡与Linux条件范围

Windows七卡继续DEFERRED，不创建READY或owner；下表仅假设先确认一个Windows/CPU目标及独占设备、共享核心依赖已按原卡交付。不是复用macOS验收或全平台报价。

| 任务/当前状态 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 | 剩余范围、风险与假设 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| [W0-01 构建/GPUI 原型](WINDOWS.md#obg-w0-01) / DEFERRED | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 | 固定Windows/CPU与真实GPUI输入/字体/托盘；机器和范围未确认 |
| [W0-02 服务/TUN 原型](WINDOWS.md#obg-w0-02) / DEFERRED | 2–4 | 1–2 | 1.5–3 | 0.75–1.5 | 1–2 | 6.25–12.5；9.375 | 真实UAC/SCM/Named Pipe身份/锁定驱动资源；不预设另发wintun.dll |
| [W1-01 普通代理/WinINet](WINDOWS.md#obg-w1-01) / DEFERRED | 2–4 | 1.5–3 | 1.5–3 | 0.75–1.5 | 1.5–3 | 7.25–14.5；10.875 | 普通child与业务用户WinINet/条件恢复；复用共享核心不重做订阅 |
| [W2-01 服务/Named Pipe](WINDOWS.md#obg-w2-01) / DEFERRED | 3–5 | 1.5–3 | 2–4 | 1–2 | 1.5–3 | 9–17；13 | 生产服务/保护目录/ACL/握手/有界IPC；消费P2-06稳定契约 |
| [W2-02 TUN/owner 交接](WINDOWS.md#obg-w2-02) / DEFERRED | 2–4 | 1.5–3 | 2–4 | 0.75–1.5 | 1.5–3 | 7.75–15.5；11.625 | Job Object/TUN/跨用户独占与异常恢复；必须Windows设备证据 |
| [W3-01 安装/升级/卸载](WINDOWS.md#obg-w3-01) / DEFERRED | 2–4 | 1.5–3 | 1.5–3 | 0.75–1.5 | 1.5–3 | 7.25–14.5；10.875 | 无Authenticode安装升级卸载/回退；第三方驱动签名仍适用 |
| [W3-02 Windows 独立验收](WINDOWS.md#obg-w3-02) / DEFERRED | 0 | 1–2 | 2–4 | 1–2 | 2–4 | 6–12；9 | 独立Windows全场景/多用户/重启/卸载；不复用macOS平台PASS |

Windows合计 **49–97人日，中点73**；五栏依次为13–25 / 9–18 / 11.5–23 / 5.5–11 / 10–20。 下界独立最长链42.75； 上界独立最长链84.5； 中点独立最长链63.625；共享依赖均已交付条件下，控制链W0-01→W1-01→W2-01→W2-02→W3-01→W3-02；共享依赖仍未完成时还要等待P2-05/06/08、P6-01/04、P7-01/02及各组合卡，不能将49–97直接转为Windows开工后日历交付日。

Linux没有已定义实际Task，以下仅为条件工作包分类，非新增卡/READY。假设一个发行版/CPU、桌面环境及一个明确的系统代理后端；X11/Wayland其中一个作首期目标，额外环境/后端另估；复用稳定Core/GPUI/Compiler，具备真实普通/管理员会话、网络与独立干净设备。不预设NetworkManager/桌面代理/授权接口一定通用。

| 条件工作类别 | 开发 | 集成 | 定向验证 | 独立Review | 人工视觉/平台返工 | 合计；中点 |
| --- | --- | --- | --- | --- | --- | --- |
| 构建/GPUI输入字体/托盘/文件适配 | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 |
| 目录/单实例/会话与当前用户代理适配 | 2–4 | 1–2 | 1–2 | 0.5–1 | 1–2 | 5.5–11；8.25 |
| 授权宿主/IPC/安装卸载 | 3–6 | 1.5–3 | 1.5–3 | 0.75–1.5 | 1–2 | 7.75–15.5；11.625 |
| TUN/owner/睡眠切网恢复 | 3–6 | 1.5–3 | 2–4 | 0.75–1.5 | 1.5–3 | 8.75–17.5；13.125 |
| 包/手动升级/回退与版本绑定 | 1–2 | 1–2 | 1–2 | 0.5–1 | 1–2 | 4.5–9；6.75 |
| 平台/全UI组合验收及已知差异披露 | 0 | 1–2 | 2–4 | 1–2 | 2–4 | 6–12；9 |

Linux条件合计 **38–76人日，中点57**；无实际DAG/卡就不能给可信关键路径或宣布开工。发行版/后端/CPU/显示协议范围、第三方资源及真机能力确认后再拆卡估算，不能称为已READY。三平台数字不合并为一个已承诺全平台工期。

<a id="constraints"></a>
## 7 独立外部条件、风险和不包含项

| 条件/风险 | 影响与解除条件 | 工时处理/归属 |
| --- | --- | --- |
| 最低OS/固定内核差异 | 首版15+已批准；当前固定内核仍minOS26.0，兼容构建/最低15设备未实测 | 4–9已分配进P7-03/P6-05/P7-05；原型不等于支持声明 |
| 原始证据未入Git | 主树授权只读原索引与Hash已追溯；隔离Git树历史相对链接仍缺文件，不得假称入Git或复制补齐 | 文档验收已闭合；Host保留原档案/精确纳入本轮收据，历史缺失引用仍单列 |
| P2-05未合并、P2-06单owner | 各自收口/整合后才能满足显式依赖；未读P2-05独立树，剩余预算依赖owner确认；P2-06缺口有跨生产边界风险 | 只估剩余，超出既有交付需更新；资源冲突不靠增加Agent自动解除 |
| 真实SystemProxy/TUN与主网络保护 | 独立可恢复机器、管理员授权、Wi-Fi/有线、睡眠/切网及外部TUN隔离；无授权/机器结果NOT_RUN | P2-05/07/09、P6-01/02/05；已有正常工时计五栏，机器/授权等待不计 |
| GPUI视觉返工 | 同源React/CSS级联/同视口数据状态，150%浅色优先，全局i18n三语言/键盘/IME；字体和blur技术差异有据，不能近似替代或虚称95% | 各功能卡局部五栏，组合卡深色/其它缩放及全E2E；人工返工已列不再叠加百分比 |
| 正式远端发布/下载 | GitHub无资产历史NOT_RUN；需要明确发布授权及真实候选资产才能做远端browser/quarantine首开/升级回退 | P6-04/05、P7-03/05；授权等待单列，本轮不创建Release |
| 机器/架构 | Apple Silicon正式最低OS及当前支持设备；Intel不默认纳入。Windows/Linux各需已确认OS/CPU/会话/干净设备 | 不把macOS27/arm64证明外推；额外架构/环境需新范围条件估算 |
| 签名与系统限制 | 无Developer ID、Authenticode发行签名/Apple公证/开发者账户；Apple Silicon最低ad-hoc和系统单应用授权仍需验证，第三方驱动系统签名仍适用 | 不计证书/公证/旧门禁等待，不取消OS保护；驱动资源异常在W0-02/W2-02/W3-01实测 |

不包含：自建/patch DNS或sing-box内核、协议/IPv4/IPv6/WireGuard等重新认证、路由器MAC/LAN/脚本/网关新增实现、自动安装更新/新常驻安装服务、额外CPU/多Linux环境、发布证书/公证/旧流程工件、用户数据迁移的未批准扩展。已有Veyra安全归属/脱敏/凭据权限、目录图校验、失败保旧、真实持久化及本地服务契约继续验证，不因范围差异删除。

<a id="acceptance"></a>
## 8 四项验收与交接

| P0-09验收项 | 本轮结果 | 依据与剩余条件 |
| --- | --- | --- |
| 1 影响路线未知项有结论及明确决定 | PASS_DOC_DECISION_TRACE | 用户正式批准15+；原型索引直接核读/Hash，固定内核26与未验证15事实保留；兼容实现/最低设备验证明确归已有三卡 |
| 2 范围与任务规则、DNS/路由器差异 | PASS_DOC_SCOPE | 第3/7节固定既有差异和承接卡；不改变已验收事实，不以文档证明正式Native |
| 3 P1～P7工作量/假设/风险/排除项 | PASS_DOC_ESTIMATE | 52卡（含已DONE）五栏逐卡与阶段/关键路径，DOING/ACCEPTANCE只计剩余；算术及DAG见验证收据 |
| 4 Windows/Linux独立估算和依赖边界 | PASS_DOC_ESTIMATE | Windows7卡DEFERRED逐卡；Linux分类条件区间，无实际卡不称READY；平台未决只沿原DAG阻塞 |

本轮四项文档验收满足，P0-09 DONE、owner/文档预约释放。P0-01～08历史卡段及历史FAIL/NOT_RUN不改写；P2-05 ACCEPTANCE/P2-06 DOING保留；P7-05已满足本卡前置，但其它前置未DONE，仍TODO，READY=[]。兼容资产与最低15平台真正验收尚未发生；DONE仅是路线决定/阶段证据审计/剩余估算完成。

Agent不stage/commit/merge/push/发布。由Host再次独立review后负责提交/合并；`.gitignore:44`忽略evidence，本轮两个收据已在隔离树更新，Host须按精确路径强制暂存 `docs/openbox-rust-gpui-tasks/evidence/p0-09/REVIEW.md` 和 `docs/openbox-rust-gpui-tasks/evidence/p0-09/validation.json`，另纳入五份Markdown，不扩大强制暂存目录、不改忽略规则。主树历史证据严格只读且未复制，绝不冒充Git交付物。

<a id="validation"></a>
## 9 本轮纯文档验证与独立Review

仅本地文本/Git只读检查：Markdown本地文件与锚点（新增/变更必须有效，历史缺失另列）、68卡唯一/61+7/状态一致、卡与表显式依赖逐项一致且存在/无环/READY推导、77API/19场景/4流/6页9分类覆盖归属字节不变、逐卡/阶段/平台五栏求和及中点、输入身份与P0-01～08原卡区段未变、保护生产路径无diff、`git diff --check`及新文件空白。

完整收据见[文档验证](evidence/p0-09/validation.json)，最后独立只读审查见[Review](evidence/p0-09/REVIEW.md)。首轮原文件追溯未授权且缺于隔离树的FAIL_EXISTING_MISSING_EVIDENCE保留在previous_execution；本轮授权只读原索引及Host给Hash逐项吻合，当前追溯PASS。隔离树历史相对链接缺失仍逐条单列，不声称全部本地链接恢复。独立Reviewer仅文本核读和本地定向检查，不亲自复跑历史测试/GUI/授权/平台操作；Host仍需再次review。本轮产品tests/build/网络/root/GUI/原型重跑均NOT_RUN（纯文档任务不适用），不用于能力PASS。
