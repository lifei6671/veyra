**P5-05 启动修复收口（2026-10-09）**：Finding P5-05-STARTUP-SNAPSHOT-001 CLOSED；共享网络与 SharesUpdated 复用 accept_page_snapshot，拒旧结果并重投影；23唯一定向PASS，最终真实macOS页面先到/全局先到各首次Refresh=1。按用户要求实现真实行高占位/吸附，实际拖放/取消/磁盘回读通过；动态占位中途帧 NOT_CAPTURED 单列，不冒充视觉验收。独立Review无剩余代码问题；P5-05既有DONE、修复owner/预约释放，P2-06 DOING及原范围不变。DONE24/DOING1/READY4/TODO32/DEFERRED7，READY未变；本地独立commit/合并/主树复验、不push。见[本次记录](P5-05-startup-fix.md)。

**P5-05 启动回归修复领取（2026-10-09，历史）**：Finding P5-05-STARTUP-SNAPSHOT-001 OPEN；P5-05保持DONE。原生新worktree `p5-05-startup-drag`，分支 `dev/p5-05-startup-drag`，基线e56c732。owner=Codex · Desktop修复；预约app.rs/StateBridge定向回归/shared_network拖动交互及局部Tokens/本次记录。复用accept_page_snapshot，不改Runtime/Helper/IPC/公共DTO或五协议/Store/URI/QR；补占位与吸附，真实macOS验证、独立Review后独立commit合并。P2-06 DOING及原owner保留，历史证据不覆盖。

**P5-05 最终收口（2026-10-09，当前）**：五协议共享网络GPUI、生产Worker/Store CRUD/排序/CAS/失败保草稿与重建回读、自签证书和URI/二维码完成。116个唯一定向测试实际PASS，固定1.14.0五协议check及最终截图二维码解码通过；Core/Desktop Clippy/fmt/build通过，旧Tauri Windows LICENSE既有FAIL保留。150%浅色在线同态原图与独立Review通过，保留具体blur/MiSans技术差异，不称完全一致。最终binary SHA256 `71930ed61a3c710e292268079e921181dc6be9fa3a5f2a47a7fbf2644cb550e1`。P5-05 DONE、owner/预约释放；P2-06仍DOING及原范围，P5-06 HTTP不变。68卡DONE24/DOING1/READY4/TODO32/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03，未启动下游。一Task本地commit后合并主开发分支并重验，不push，保护主树未跟踪文件。见[最终记录](P5-05-acceptance.md)。

**P5-05 领取（2026-10-09，历史）**：原生 Codex Worktree `/Users/lifeilin/.codex/worktrees/p5-05-share-ui/veyra`，分支 `dev/p5-05-share-ui`，基线 `8c220437ab6a4f9528e39e84261bebd916d0080a`。三前置均DONE；P5-05 READY→DOING，owner=Codex · P5-05 GPUI/Core。预约共享网络 UI/Worker、URI/QR、Core shared_inbounds 最小排序/证书接口及本卡文档；复用现有 Tokens/组件/i18n。P2-06 Runtime/Platform/Helper/IPC/公共Runtime DTO不改，P5-06 HTTP独立保持；不动主树未跟踪文件，不push。当前 DONE23/DOING2/READY4/TODO32/DEFERRED7。

**P5-04 最终收口（2026-10-09，当前）**：按用户明确的职责分离范围完成五协议领域/验证、SnapshotService/JsonStateStore CRUD与真实失败保旧/重建回读、TLS资源和端口预检、正式SingBoxCompiler封闭入站产物；10新增测试实际PASS，合并回归去重184PASS/1基线既有FAIL/1辅助入口ignored；固定1.14.0的10正check/5反check/5加载/6bind冲突及退出清理通过。Core/Desktop build/Clippy/fmt通过；旧Tauri Clippy缺Windows LICENSE资源FAIL保留。独立只读Review无可操作Finding。P5-04 → DONE、owner/预约释放；正式Runtime Apply/Restore/回退由P2-06原owner负责，本卡未越权修改，不冒充Applied。68卡 DONE23/DOING1/READY5/TODO32/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03/P5-05，P5-05只转READY未启动。仅隔离worktree一次本地commit、不push；详见[交付记录与最小Runtime接口](P5-04-acceptance.md)。

**P5-04 领取（2026-10-09，历史）**：Local 模式限定 `/Users/lifeilin/.codex/worktrees/p5-04-shared-inbounds/veyra` / `dev/p5-04-shared-inbounds`，基线 `eec981f34386e1f2461dbba137707ee814b58e4c`、干净。文件/临时 index 暂存/提交对象/index lock 写权限 PASS；会话默认 cwd 仍为主树，所有操作显式指定隔离 worktree，不改主树。P5-04 READY → DOING；owner = Codex · P5-04 Core/Config。预约 `domain/shared_inbounds.rs`、AppConfig 的 shared_servers 字段/校验、`application/shared_inbounds*`、SnapshotService 最小 CAS 保存入口、`singbox/compiler/shared_inbounds*`、Compiler 接口与受控测试、本卡文档/evidence；Core Cargo 仅将既有锁定 rustls 依赖用于证书解析（不改 workspace/锁文件）。P2-06 Runtime/Platform、Helper/IPC、Runtime DTO/恢复契约及其 owner 保留；P5-06 HTTP/token/host/listen 不动；P5-05 UI/URI/QR 不实施。正式入站/恢复/失败回退接线需原 owner 最小协调，不以独立配置加载冒充产品 Applied。自有临时目录、loopback 动态测试端口，禁止系统代理/TUN/现有实例操作。当前 68 卡 DONE22/DOING2/READY4/TODO33/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03，均未领取。

**P5-06 最终收口 / DONE（2026-10-09，当前）**：最终产品架构为Veyra独立ShareService/Axum HTTP端口、sing-box节点JSON和URL二维码；解除错误P2-06端口依赖。listen/host/LAN生产回归、29唯一定向测试、Clippy/fmt/build、真实GUI功能与150%浅色逐态视觉通过；新增页面快照先完成的启动收敛修复，最终删除无导航重启Ready。最终signed SHA256 `879c5d775ff0adce3c0a80fc313df79c32751ba971632321f8d20485ba652b0b`。独立复核三项P2均关闭，无剩余可执行finding。用户明确接受局部blur、MiSans静态字体及独立监听字段差异，不声称像素完全一致；同机LAN验证不代替第二设备，较早构建托盘证据不重绑最终SHA。所有历史证据保留，本卡owner/预约释放；P2-06 DOING、原owner/范围不变。68卡DONE22/DOING1/ACCEPTANCE0/READY5/TODO33/DEFERRED7，READY队列未变且不启动下游；本轮一次本地增量commit、不push。详见[最终验收记录](P5-06-acceptance.md)。

**P5-06 / ACCEPTANCE（2026-10-09，历史；端口决策已覆盖）**：启动故障真实复现为Store gate Busy；首次全局Accepted/Ready后才发起Runtime Refresh，生产80轮删除重启与真实GUI无需导航Ready。Core9/Desktop19唯一定向测试、最终build/Clippy/fmt通过；旧Tauri Windows资源失败独立保留。真实本地React原生confirm系统截图已补，线上自绘Dialog另列；修正具体分享名称、线上几何/控件/列表。最终signed SHA256 `018f13a57898ea4d3f05198c0165a5c9acf472d75677518ae239077c35d31b91`。独立只读Review无新增阻断代码问题。**用户最新明确监听字段应配置sing-box入站端口，需协调P2-06 owner；现有ShareService HTTP监听不等价，已停止相交修改。** 字体等剩余逐态视觉未闭合，维持ACCEPTANCE、owner不释放。P2-06继续DOING及原范围；不动系统代理/TUN/管理员配置。详见[P5-06当前记录](P5-06-acceptance.md)。68卡DONE21/DOING1/ACCEPTANCE1/READY5/TODO33/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03/P5-04。历史证据保留，精确范围新本地增量commit，不push。

**P5-06 验收修正 / ACCEPTANCE（2026-10-08，历史）**：保留9cf86ad8；读取按钮改“刷新”，刷新成功不抹去失败写操作，原草稿Save实际重试。Core9/Desktop8个唯一定向测试、build/Clippy/fmt PASS；旧Tauri本轮先报Windows libcronet.dll缺失exit101，历史LICENSE FAIL保留。最终无分享测试延迟签名SHA256 `d8d65d1b3f8277310ea08c43401c3ce324dc7241a77c525d0f9b9f31f3173b49`。真实GUI绑定冲突保旧/刷新保错/释放后Save200、用户托盘退出后PID及监听释放、删除确认与重启磁盘空态均补验；同Token/URL React与GPUI各状态原图、Vision二维码同URL留存。React原生确认参考原图缺失、启动瞬间读取失败原因和最终视觉逐项结论仍OPEN，不能DONE/释放owner。临时资源清理完成，最终bundle作为本地证据保留；本轮独立增量commit、不push。见[当前记录](P5-06-acceptance.md)。68卡DONE21/DOING1/ACCEPTANCE1/READY5/TODO33/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03/P5-04。P2-06 owner及P4状态不变，未领取下游。下方旧轮次按历史结果保留。

**P5-06 首轮 / ACCEPTANCE（2026-10-08，历史）**：正式分享 CRUD/CAS/磁盘恢复、真实 HTTP/token 撤销与监听生命周期、GPUI Worker/复制/二维码/三语言已接通；Core 144 / Desktop 7 个唯一定向测试通过，build/Clippy/fmt PASS；旧 Tauri Windows LICENSE 独立 FAIL。最终 UI 复核遇到 macOS 锁屏，正常 Quit、GUI 删除/端口冲突及最终同状态截图仍待完成，保持 ACCEPTANCE。见[交付记录](P5-06-acceptance.md)。68 卡 DONE21/DOING1/ACCEPTANCE1/READY5/TODO33/DEFERRED7；P2-06 owner 不变，P4-03 READY，未启动下游。下方领取及旧轮次保留为历史。

**P5-06 领取（2026-10-08）**：从 `30a4c7ddc1ef40dadc0a68ebebad8afe3f7d70c3` 创建独立工作树 `/Users/lifeilin/.codex/worktrees/p5-06-sharing/veyra` / `dev/p5-06-sharing`。三项依赖均 DONE，P5-06 → DOING。预约 shares 专属领域/Service、AppState 分享字段及 SnapshotService 最小保存入口、Desktop 分享 Worker/UI/国际化、QR 局部依赖与本卡文档。保留 P2-06 Runtime/Platform、Helper、IPC、Runtime DTO owner；退出仅消费 AppServices 自有资源关闭接口。自有临时目录与 loopback 监听，不动用户资源。当前 DONE21 / DOING2 / READY5 / TODO33 / DEFERRED7；P4-02 DONE、P4-03 READY。下方旧轮次是历史。

**P4-02 / DONE（验收修正）**：隐藏勾选批量范围、React局部视觉、正式UI→Worker→Core的加载/忙碌/失败保草稿重试通过。Core68个唯一定向测试、Desktop37个UI测试、构建/Clippy/fmt和locked check通过；旧Tauri Windows LICENSE缺失独立FAIL。正式构建SHA256 `09ab52c9dbd3b10609f92c88cfdbe473d2fa7da3e54993e1a00ba3ae574e4247`。原88d7d137及证据保留，本次增量本地commit、不push；本卡owner/预约释放，临时资源已清理。见[最终记录](P4-02-acceptance.md)。68卡：DONE21 / DOING1 / READY6 / TODO33 / DEFERRED7。P4-03依赖齐备转READY但未领取；下一可并行首选P5-06。P2-06 DOING及Runtime/Platform、helper/IPC、DTO owner不变。下方旧轮次保留为历史。

<a id="schedule-20261008"></a>
## 领取前调度决定（2026-10-08，历史）

WorkRun `work-5654-1791456602781005-182`：执行[唯一调度政策](DEVELOPMENT_WORKFLOW.md#delivery-order-20261008)及[优先级/延期清单](IMPLEMENTATION_PHASES.md#priority-20261008)。前期先 UI 与对应真实用户功能、局部受控定向集成、真实保存/读取与重建后磁盘恢复，UI 使用真实 Core Service/Store；只画 UI/Mock 不得 DONE。局部检查固定 150% 浅色并遵守 React/CSS；无实时来源显示未知，Saved 与 Applied/Ready 分开。

**明确下一 READY 首选：P4-02 节点组 UI + 持久服务**，P5-06 订阅分享 UI 可跟进；P2-05 出站客户端、P3-01 观测和 P5-04 按原依赖/写范围并行或适时领取，涉及 Runtime/DTO 时协调 P2-06 owner，不以网络客户端抢先阻碍 UI。P2-05 仍是 P2-08 前置。P0-08 保留 READY 作路线准备。本次仅改计划，所有 READY 仍未领取/未启动；依赖不足的正式功能继续 TODO，不抢跑。

P2-06 已阶段提交 `f2457e6a4e2fe0ac3c9186bc7d17323a70c4c0a8`，仍 DOING、原 owner 保留；GUI/Native/Helper 多轮及重启恢复后期补，不再持续钻研复杂崩溃恢复挡住其它功能。下方 round19 及其它轮次按当时结果保留，其“不提交/不推进”是历史约束，不覆盖本次已提交 checkpoint 和新调度。现有安全拒绝不解除，写冲突仍由 owner 协调。

后期实施 P2-07 SystemProxy 与 P6-01 TUN/P6-02 恢复，前期系统开关禁用且不得假成功。最后按原 DAG 执行 P2-09/P3-08/P4-07/P5-07/P6-05 组合，P7-04 统一 6 主页/9 分类/77 API/19 场景实际 GUI E2E，P7-05 最终包。仅正式安装、真实内核网络、系统写入/恢复、Sleep/Wake 切网真机组合、其它主题/缩放和完整跨页/全场景交互归最终批次。P2-08 选择/测速、P3 实际服务/连接操作、P4 主备/Compiler/应用/受控链测试、P5 DNS 本地测试/资源下载/应用与分享 HTTP 监听/token 失效/退出清理必须在各自前期卡实现并局部验证，不能交由组合卡代替；无授权的具体验证准确 NOT_RUN，原需求/安全规则不删。

**状态不变**：68 卡；DONE20 / DOING1 / READY6 / TODO34 / DEFERRED7，其余状态 0；原依赖、77 方法/19 场景归属和所有历史 PASS/FAIL/NOT_RUN 原样。本次无产品/GUI/网络/权限验证，不操作已有资源或未跟踪文件；不 add/commit/push，文档交 Host 独立 review 后按用户每 Task 一次 commit 规则提交。

**P2-06 round19 / DOING**：同helper生命周期内首轮跨Desktop RebindCommit→新Primary独立Start→鉴权Ready→Stop/reap已接通并由真实双Primary/Unix IPC/受控内核fixture验证。提交保留旧Frozen/source/root审计，root create_new提交/当前owner并fsync后切内存；cycle继承且新Start先使CleanStop失效，lease同open-file-description持续持有。首个Commit回复丢失只Query，重复Start/错版本/脏cache/半marker/旧Peer拒绝。Core377/Desktop93/helper70/原型47/Python4及check/clippy/fmt通过，初始测试FAIL保留，见[round19](evidence/p2-06/round19/README.md)。**仅一轮旧→新Primary转换；第三会话继续轮换、helper重启恢复、bootstrap Apply/Select、GUI消费与Native仍OPEN/NOT_RUN**。保持DOING/owner/DAG/index空，不提交，不推进P2-05/P2-07。

**P2-06 round18 / DOING**：新Primary的RebindPreflight→Core持久申请→root Prepared→Query已接生产协议与Desktop同worker。root持旧lease重算CleanStop关闭bundle，核验旧NOTE_EXIT、owner-session/安装generation及新OS Primary；保留旧Frozen/incarnation/source-session。真实两代Primary进程及IPC Query、脏cache拒绝/幂等测试通过。**Rebind Commit/新peer Start仍未实现，helper重启仍拒绝**，不声称跨Session重开完成。Core377/Desktop93/helper68/原型47/Python4及工程检查通过；初始编译FAIL与fixture IPC失败/中断保留，见[round18](evidence/p2-06/round18/README.md)。owner/DAG不变，index空，无提交/Native。

**P2-06 round17 / DOING**：交付A的同helper生命周期CleanStop固定凭据及P1修复：真实Stop/reap后核验完整版本/pending/fence、关闭plan/cache/manifest、owner-session，再写root-only当前生命周期记录；新Start先持久使旧CleanStop失效，cycle单调递增。bootstrap注册OS NOTE_EXIT，Primary异常退出时helper只清理自有Child，保留Unknown/RecoveryRequired且不签CleanStop。新Primary rebind Prepare/Commit与跨进程重开B/C仍未实现，继续拒绝。Core377/Desktop93/helper67/P0原型47/Python4及工程检查通过，clippy初始FAIL保留；[round17](evidence/p2-06/round17/README.md)。Native NOT_RUN；owner、DAG、index空和不提交约束保持。

**P2-06 round16 / DOING**：同一Runtime/Platform+DTO owner。已接通固定Primary socket只读probe+OS UID/PID/start与flock核验、root持久Committed、同一lease FD接入ProcessPort，以及Desktop同worker Commit确认后的独立Start。真实非特权socket/测试child证明无需先运行手动代理即可check/run/鉴权Ready，重复Start同实例、同会话Stop/reap后同版本重启；跨进程clean-stop、bootstrap Apply/远程选择、GUI消费与Native仍OPEN/NOT_RUN。Core377/Desktop93/helper65/P0原型47/Python4通过；详细receipt与初始FAIL见[round16](evidence/p2-06/round16/README.md)。未暂存/提交，P2-04 DONE/P2-05 READY/P2-07 TODO不变。

**P2-06 round15 / DOING**：同一Runtime/Platform+DTO owner。正式Bootstrap只读Preflight、root持久Prepared/Query已接IPC；Desktop同worker→Core先预检再freeze/发布SourceSession→root独立核验固定source/OS peer/安装generation→持有lease并Prepared。失联重试绑定同票据/配置摘要；丢slot仅RecoveryRequired；incarnation-only拒绝。未实现Commit/独立Start及跨进程clean-stop，不删除gate。Core377/Desktop92/helper60/P0原型47/Python4、all-targets check/clippy/build/fmt通过；[round15证据](evidence/p2-06/round15/README.md)。P2-04 DONE/P2-05 READY/P2-07 TODO不动，未Native/暂存/提交。

**P2-06 round14 / DOING**：交付 Desktop 单worker→Core durable `BootstrapFrozen`、同票据重试及只读查询；重启不解除冻结，Manual Start/Select继续拒绝。此记录仅是本地限制，不是root授权，未接产品/UI自动调用。修复helper lease竞争在取得独占前吞掉assets；owner写入后的失败保持RecoveryRequired。首次root prepare/commit/安装预检编排和跨进程clean-stop仍OPEN，独立First Start仍不可用。Core376/Desktop91/helper56/P0原型47/Python4，三包check/clippy/build/fmt通过；新证据 [round14](evidence/p2-06/round14/README.md)。Runtime/Platform+DTO owner保留；P2-04 DONE/P2-05 READY/P2-07 TODO不动，无Native/提交。

**P2-06 round13 / DOING**：唯一Runtime/Platform+DTO owner保持。生产helper新增安装级writer OS租约，check/run通过FD4继承；父端失联后child仍持锁，Archive不能越过活writer。预检不创建文件，持锁冲突在Desktop Stop/freeze前拒绝。首次bootstrap/跨进程clean-stop提交仍未实现，不解除HandoffRequired，不声称新安装独立Start可用；只做有界底层增量。证据 [round13](evidence/p2-06/round13/README.md)，无Native/系统配置/Git提交；P2-04 DONE/P2-05 READY/P2-07 TODO保持。

**P2-06 round12 / DOING**：Runtime/Platform+DTO唯一owner保留。新增固定root安装世代与资产绑定，修复同requestId Start重放旧Ready；未知安装身份报告RecoveryRequired。首次直接启动是已确认产品目标，但Desktop首次让权/正常重启凭据与bootstrap提交尚未实装，不能用安装世代独立解锁，HandoffRequired保持。仅自有fixture/离线检查，无Native，不stage/commit；P2-04 DONE、P2-05 READY、P2-07 TODO不变。新证据 [round12](evidence/p2-06/round12/README.md)，历史FAIL保留。

**P2-06 round11 / DOING**：执行用户已批准的内核本地basename `veyra-sing-box`；唯Runtime/Platform+DTO owner保留。只改受控启动链/测试/必要当前说明；P2-07仅登记主动接管/条件恢复的产品决策，仍TODO无owner。旧安装/缓存/历史证据不动，不stage/commit，无Native。新证据 `evidence/p2-06/round11/`。 用户已批准SystemProxy主动接管已有代理：写前按Service ID保存全字段，Ready后条件写入/回读，关闭只恢复仍匹配managed的字段组，外部变更保留报冲突，多服务/部分失败逐项记录；仅补P2-07/设计文档，无网络实现。

**P2-06 round10 / DOING**：唯一Runtime/Platform+DTO owner保留。限定只读恢复Query/Preflight不触发管理员drain，验证空目录/缺文件不能授权cold-start；不放宽准入、不推进其它Task，无Native/安装/迁移。证据 `evidence/p2-06/round10/`，原dirtytree与index空保留。

**P2-06 第九轮 / DOING · 仅标识纠正**：用户确认正确基础身份 `me.disign.veyra`；本轮仅精确替换身份及测试/当前说明，Runtime/Platform+DTO owner保留，不推进功能。两个历史namespace目录与旧daemon不读取、不迁移、不覆盖、不删除、不fallback；迁移另需批准。旧round8证据只读保留，新证据 [round9](evidence/p2-06/round9/README.md)。index空、不提交，P2-05 READY/P2-07 TODO不启动。

**P2-06 第八轮 / DOING**：同一Task及唯一Runtime/Platform/DTO owner保留。统一新身份 `me.disign.veyra`（Desktop/source/Tauri/helper/prototype/probe），不迁移旧数据或操作旧daemon。单worker Quit核验helper owner并Stop/Operation/Status，失败Recovering、UNKNOWN保留fence；窗口隐藏不变。源码/真实验证与初次FAIL见 [round8](evidence/p2-06/round8/README.md)。cold/未知崩溃恢复、同UID信任边界及打包授权仍OPEN；不Native、不stage/commit，原dirtytree保留，其它Task不启动。

**P2-06 第七轮 / DOING**：同一Task，Runtime/Platform与共享DTO owner保留。正式handoff预检能力与runtime授权分离，固定source Released+本helper观察的旧child退出/reap+完整票据验证后，有限正常会话可Start/Apply/Stop；Desktop同worker接线。Core373/Desktop88/helper41/prototype47/Python2，最终检查见 [round7](evidence/p2-06/round7/README.md)。cold/预先Stopped来源、崩溃后未知owner/slot、同UID资料信任边界、打包授权仍OPEN；DOING、不Native、不stage/commit。旧dirtytree/FAIL保留，其它Task不启动。

**P2-06 第六轮 / DOING**：同一Task，Runtime/Platform owner保留。先修复ACK后固定root原子封存与可重装，再核对正式handoff释放证明；仅helper安装/archive及必要测试/文档，无root/launchctl/Native操作，不领取其它任务。旧dirtytree/证据保留；新证据 [round6](evidence/p2-06/round6/README.md)。Archive释放ROOT且保留历史的隔离OS路径已测；Core373/Desktop88/helper39/prototype47/Python2通过。正式handoff释放证明、cold-start及未知owner/slot恢复仍OPEN；不解除gate、不提交、不Native。

**P2-06 第五轮工程实施 / DOING**：同一 Task，Runtime/Platform 与共享 DTO owner 保留。固定管理员卸载请求与串行 Stop/封存、服务生命周期锁为本轮写范围；未知旧 owner/丢失 selection slot 仍 fail closed，不解除正式 gate。仅隔离 fixture，旧证据保留；Core373/Desktop88/helper34/prototype47/Python2及check/clippy/build/fmt通过。新增活动停止封存barrier，旧owner/UNKNOWN恢复、cold-start与卸载失败恢复/重装仍OPEN；证据 [round5](evidence/p2-06/round5/README.md)，不领取 P2-05/P2-07、不提交。

**P2-06 第四轮工程部分交付 / DOING**：同一 Task / WorkRun `work-5654-1791427311971442-122`；Runtime/Platform 与 Runtime DTO owner 保留。先修复 HANDOFF PREFLIGHT ORDER，再补远程选择持久 fence/IPC/CAS；写范围为当前 Core Runtime/state storage 必要公共写边界、Desktop worker、production helper、当前文档。仅自有 temp/socket/test exe；保留旧 dirty tree 和证据，禁止 Native/root/真实系统配置，不领取 P2-05/P2-07。新证据 [round4](evidence/p2-06/round4/README.md)。预检顺序修复、Desktop持久fence/真实IPC PUT-GET/CAS/manifest正常与失败重试已实做；Core373/Desktop88/helper28/prototype47/Python2通过。缺失存活slot/未知旧owner的安全恢复与活动卸载仍OPEN；正式HandoffRequired保留，不进入Native、不提交。见[第四轮](P2-local-proxy.md#p2-06-round4)。

**P2-06 第三轮工程部分交付 / DOING**：同一Task，Runtime/Platform与Runtime DTO owner保留；优先双向writer/cache/manifest交接和远程pending/CAS，修复新requestId重复Start存活核验。写范围限定Core交接seam、Desktop worker/helper、production helper及本卡文档；锁为单Runtime worker与业务StateAccessGate，资源仅自有temp/socket/测试exe。P2-04 DONE不变，不领取P2-05/P2-07。新证据 `evidence/p2-06/round3/`，旧dirtytree/日志保留。 已交付双向关闭cache/manifest握手与持久fence、同票据重试/重启只读查询、重复Start同步存活修复；B远程pending/CAS、崩溃后安全解冻和C活动卸载仍OPEN，正式总gate保持。Core371/Desktop88/helper25/prototype47/Python2通过；不stage/commit，不直接Native。见[第三轮](P2-local-proxy.md#p2-06-round3)。

**P2-06 第二轮工程部分交付 / DOING**：同一Task，WorkRun `work-5654-1791427311971442-122`；保留首轮dirtytree与所有历史证据，owner不变。生产进程/固定资产/安装入口与跨owner缺口继续工程实现，仅非特权隔离进程测试，禁止Native/root/系统设置操作。独立证据目录 `evidence/p2-06/round2/`。Core371/Desktop87/helper20/prototype47/Python2、三包all-targets check/clippy、build/fmt/diff PASS；11 Desktop Native ignored，1 helper fixture由测试内部启动。 已新增真实ProcessPort/ManualRuntime执行器、固定安装代码和OS隔离测试；缺owner交接、远程选择pending闭环及活动卸载barrier，正式入口HandoffRequired。不得直接进入Native；见[本轮记录](P2-local-proxy.md#p2-06-round2)。工程未完整，不stage/commit。

**2026-10-08 P2-06 首轮工作（历史）**：基线 `ac919f3729fed5a3cf0a2b3dbcbd4164b06a3597` / `codex/dist-react-restore` / clean；READY → DOING。Codex 是唯一 Runtime/Platform、Runtime DTO owner。仅生产 helper/IPC、桌面 composition、必要 Core 契约与本卡文档；P0-05 prototype 保留隔离。Host 后续独立 review；Native 安装/GUI 交 Codex Desktop。禁止真实 root/child/网络设置/管理员操作；仅 Mock、测试自有 Unix socket 和临时目录。锁范围为 Runtime worker、helper 会话/操作缓存及 Cargo 局部依赖条目；不领取 P2-05/P2-07/P6。当前 DONE20 / DOING1 / READY6 / TODO34 / DEFERRED7。IPC/受控编译/客户端接线已完成本地检查；生产 child、安装、controller 和 writer/cache/manifest 交接代码仍未完整实现，故保持 DOING，不转 ACCEPTANCE。Core371/Desktop87/helper14/prototype47+Python2 PASS；新 workspace check/clippy PASS；旧 Tauri clippy 缺 Windows LICENSE 资源 FAIL。未 commit/push，保留本卡 dirtytree。见[交付边界](P2-local-proxy.md#p2-06-local-contract)。下方带日期历史原文保留，其“当前”不覆盖本段。

**任务文档同步（2026-10-08，当前版本控制状态）**：P0-05 → P0-06 → P2-01 → P2-02A → P2-02B → P2-03 → P2-04 已按依赖顺序形成七个独立提交，末端 `dcc9b7b2b7aa32e6f3c472eeae42f24ed2f22906`；远端历史已通过 `fdb8930a010ed321e496caa62a98257c64031b8a` 合并，文件树不变，七个 SHA 保留。此前“Git 隔离 BLOCKED / 未提交”的收口记录仅描述当时现场，现已解除。本次仅同步四份任务文档，不修改源码、不重跑 Native/GUI、不启动下游、不 push。P2-04 DONE、Finding CLOSED、P2-06 READY 及 owner 释放保持；完整提交验证见[提交链报告](evidence/p2-04/git-history-integration-20261008/integration-20261008T100426/REPORT.md)。

**P2-04 Host FINAL ACCEPTANCE（2026-10-08，当前）**：Host独立确认PASS，P2-04 ACCEPTANCE→DONE，P2-04-PENDING-PERSISTENCE-001及前轮P1/P2 Finding均CLOSED；Runtime/Platform及Runtime公共契约owner/本卡预约释放，不意味着已有app/root/child被操作或清理。Core source aggregate `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8` 与harness身份精确绑定，Host核验77 Native事件/逐行日志、六组恢复及preflight、P1/P2限定注入、原14条exit0，另MCP新复跑Core371/Desktop85 exit0（11 ignored显式覆盖），receipt见[收口证据](evidence/p2-04/final-host-closeout-20261008/README.md)。自然HTTP/OS故障、SIGKILL、无控制线程竞态仍NOT_RUN，历史REWORK与原证据不改写。本轮仅文档/静态核验，无Cargo/Native/GUI/真实资源操作。DAG DONE20/ACCEPTANCE0/READY7/TODO34/DEFERRED7，共68；P2 5/10、macOS20/61；P2-06仅TODO→READY，全部READY未领取/启动。Git独立提交BLOCKED：HEAD缺前序未提交实现，无法安全隔离P2-04-only自洽提交；index未触碰、无stage/commit/push。见[正式收口](P2-local-proxy.md#p2-04-final-host-closeout)。

**此前带日期摘要为历史原文，旧“当前/本轮”指当时；本次现态以上方收口及第1–4节为准。**

**P2-04 Host修复绑定源码Native（2026-10-08，当前）**：三文件aggregate `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8`逐轮MATCH，固定1.14.0 kernel MATCH；production/UI未改，只增cfg(test) harness。旧Native、六组跨OS恢复、uncovered preflight PASS；P1旧cache保护/P2同池GET交错、epoch、CAS与共享gate的真实控制器+限定故障注入3/3 PASS。自然HTTP/启动故障及SIGKILL等NOT_RUN，边界明确。Core371/Desktop85（11 ignored已显式覆盖）、check/clippy/fmt/diff及全部自有资源cleanup PASS。仍ACCEPTANCE / FIXED_PENDING_HOST_REVIEW / Runtime/Platform owner；等待Host最终复核证据，不自行DONE。DAG DONE19/ACCEPTANCE1/READY6/TODO35/DEFERRED7；P2-06 TODO，无下游/既有app或正式root/系统网络/Git提交推送操作。见[完整证据](evidence/p2-04/host-rework-native-20261008-091526/README.md)与[验收记录](P2-local-proxy.md#p2-04-host-rework-native)。

**P2-04 Host REWORK 有界修正（2026-10-08，前轮）**：Host 独立审查的历史结论 **REWORK（P1 + P2）** 保留。只改 Core `manual_runtime.rs`、其 `tests.rs` 及必要 `state_service.rs` 共享 gate；配套本卡/SESSION 与新 ignored evidence。candidate Run/Ready/reconcile 失败存在持久 pending 时清理后拒绝旧 cache 回退；显式 Restore 校验 active plan/关闭 live cache 来源，来源不符则 prepare/Stop 前拒绝。所有 controller PUT 将完整 SelectionVersion/epoch 核对与有界 PUT/GET 置于同一共享 gate，确认/clear 仍精确 CAS；覆盖同池 GET 并发及部分确认冲突。初始回归 4 FAIL/1 PASS 保留；最终 Core371（原366 + 5）、Desktop85（8 ignored NOT_RUN）、两 crate all-targets offline check/clippy -D warnings、workspace fmt/diff PASS。三文件 source aggregate SHA256 `987a01782c59d75c913f20a8133fd37df27638951f73cf3168cb2b35c3ff4fa8`。仅实现者自查，无 Host PASS/自DONE；当前修正 Native/GUI NOT_RUN，旧 Native evidence 未写入，不代表新源码验收。P2-04 仍 ACCEPTANCE / FIXED_PENDING_HOST_REVIEW / owner Runtime/Platform，P2-06 TODO，无下游/DAG状态变化，无真实内核/既有app、child、root/系统代理/TUN或Git写操作。见[新证据](evidence/p2-04/host-rework-fix-20261008-085206/README.md)及[修正记录](P2-local-proxy.md#p2-04-host-rework-fix)。

**P2-04 最终 Native（2026-10-08，历史；当前修正尚未复验）**：仅ignored test harness，production/UI未改；old/pending/third × ApplySaved/RestoreLastSuccessful 六组真实跨OS进程验收PASS，startup PUT=0；旧plan不覆盖一个/两个pending时active child不动、无GET/PUT/部分CAS。关闭cache交接及所有自有资源cleanup PASS；首轮测试路径FAIL与无活资源诊断root保留。修改前/最终P2-03/P2-04旧Native均PASS，最终Core366/Desktop85（8 ignored已显式覆盖）、check/clippy/fmt/diff PASS。仍ACCEPTANCE / FIXED_PENDING_HOST_REVIEW / owner Runtime/Platform，等待ChatGPT/Host独立review，不自行DONE。DAG DONE19/ACCEPTANCE1/READY6/TODO35/DEFERRED7；P2-06 TODO，无下游/系统代理/TUN/既有app操作，无commit/push。见[本轮完整证据](evidence/p2-04/pending-native-acceptance-20261008-082538/README.md)与[验收记录](P2-local-proxy.md#p2-04-pending-native-acceptance)。

**P2-04 pending全量覆盖修复（2026-10-08，前轮）**：同一Host P1 Finding第二边界已完成代码/自动修复；P2-04继续ACCEPTANCE、P2-04-PENDING-PERSISTENCE-001继续FIXED_PENDING_HOST_REVIEW、Runtime/Platform owner保留。candidate在finalize/check/prepare前及stop_old_writer前验证全部业务pending被index覆盖；reconcile读取最新state并全量预检，末尾以pending_in(latest).is_empty作为最终gate。不覆盖则SelectionPending、不PUT/clear/Ready；静态拒绝保留已有child/identity/endpoints。Core366/Desktop85及指定检查PASS；3 Native ignored、本轮Native/桌面NOT_RUN。旧日志不覆盖；DAG DONE19/ACCEPTANCE1/READY6/TODO35/DEFERRED7，P2-06 TODO，无下游/Git写操作。见[新fix evidence](evidence/p2-04/pending-persistence-fix/uncovered-pool-20261008/README.md)。

**P2-04 Finding 修复（2026-10-07，前轮）**：P2-04 DONE→DOING→ACCEPTANCE，owner Runtime/Platform保留；P2-04-PENDING-PERSISTENCE-001 OPEN→FIXED_PENDING_HOST_REVIEW。恢复业务pending持久化契约；Core361/Desktop85、check/clippy/fmt PASS；本轮仅代码/自动/Mock，真实中断/重启验收 NOT_RUN，等待Codex Desktop/Host，不操作真实桌面/child/root。旧交付与evidence全部作为历史保留，旧DONE已被本Finding撤销。新记录见 [fix evidence](evidence/p2-04/pending-persistence-fix/README.md)。

**前轮 P2-04 收口（2026-10-07，历史；DONE已撤销）**：P2-04 DONE；Runtime/Platform及Runtime公共契约owner释放。唯一串行选择入口、封闭恢复计划/index、last-applied v1、stop/reap后cache快照、一次回退与ApplySaved/RestoreLastSuccessful交付；隔离真实child/cache及独立OS进程重启验收PASS，P2-03真实failure matrix回归PASS。Core354/Desktop85（3个真实test默认ignored、已另行显式执行）及指定检查PASS；实现者code-delivery-review自查无剩余Finding，不称独立审查。初轮编译/测试/clippy FAIL及内核0644 cache引起的真实UnsafePath失败保留。无恢复视觉UI，无现有app/child/root操作，无Git写操作。68卡DONE20/READY7/TODO34/DEFERRED7；READY=P0-08/P2-05/P2-06/P3-01/P4-02/P5-04/P5-06，全部未领取/启动。见[交付](P2-local-proxy.md#p2-04-delivery)和[本轮evidence](evidence/p2-04/README.md)。

**前轮 P2-04领取（2026-10-07，历史）**：只领取 OBG-P2-04，DOING，owner Runtime/Platform（Runtime公共契约单一owner）。写范围 Core manual_runtime、Compiler恢复/index、Sidecar事务与受管recovery storage；Desktop manual_sidecar/runtime_service 必要契约与回归；本卡文档/local-only evidence。只使用隔离测试root，不触碰P2-03 live app/child，不启动下游，保留大dirtytree。

**当前P2-03最终收口（2026-10-07）**：Host明确确认Basic2 build功能与视觉“符合”；executable SHA256 `59280c3146502e1b9474d08f976d807cfae4390751493badf9f89aeca9b325c4`，绑定basic-settings-004/full-page-002/fidelity-003最终evidence。P2-03 DONE；BASIC-SETTINGS-004/FULL-PAGE-002/FIDELITY-003 CLOSED；Runtime/Platform + GPUI及Runtime公共契约owner/本卡预约已释放。BUSY-001、历史FAIL/REWORK/候选记录保留。仅文档/evidence，app/runtime资源不操作；释放预约不等于现场清理。本轮cleanup NOT_RUN。68卡DONE19/READY7/TODO35/DEFERRED7，其余0；所有READY未领取/未启动。见[最终收口](P2-local-proxy.md#p2-03-final-closeout)及[Host approval/验证](evidence/p2-03/final-closeout-20261007-164453/README.md)。

**前轮基础设置最终候选（历史）**：`59280c3146502e1b9474d08f976d807cfae4390751493badf9f89aeca9b325c4`；P2-03-BACKEND-BASIC-SETTINGS-004 FIX_PENDING_HOST。仅五字段通过ProfileService.patch保存；真机10/5→Restart10/10，同一build重启持久化通过。Core336/Desktop84（既有1 ignored）、check/clippy/fmt/diff通过，七个Runtime保护文件SHA不变。P2-03 ACCEPTANCE，FULL-PAGE-002 / FIDELITY-003 OPEN；owner与DAG不变，等待Host功能+视觉确认。证据 `evidence/p2-03/basic-settings-004/README.md`。

**前轮基础设置接线（历史）**：P2-03-BACKEND-BASIC-SETTINGS-004 OPEN。owner Runtime/Platform + GPUI；仅五个既有 Profile 字段通过 ProfileService.patch 保存，Desktop services/state bridge/backend/i18n 与队列测试为实际写范围。保存不自动应用。P2-03 ACCEPTANCE，FULL-PAGE-002 / FIDELITY-003 OPEN；DAG不变。

**前轮细节候选（历史）**：`3d98e8de375a626e376daab98f7474d917958b867268f8afa7a094a2147e030e`；官方MiSans4.009同包静态face、只读控件、完整文案与自然flow已接入；同视口内部ink/控件对照、叠图与差分已补齐，Core336/Desktop80（既有1 ignored）及check/clippy/fmt/diff通过。当前Backend/Light/Ready；FULL-PAGE-002与FIDELITY-003均OPEN，字体/颜色级联差异仍供Host最终判断，不作Visual PASS。DAG、owner不变。

**前轮视觉整改（历史）**：P2-03 ACCEPTANCE / FULL-PAGE-002 OPEN / VISUAL-FIDELITY-003 OPEN。owner Runtime/Platform + GPUI；仅Backend/shell视觉与必要字体/theme资源接线，Runtime八个保护文件不动，DAG不变。

**前轮解锁后续采集完成（历史）**：同一acadf5a6…候选/源码/child保留，已补齐最终Backend全长图与后半页矩阵并回到顶部Light/Ready。TUN内部MTU/MSS锚点分别−12/−8px已记录，未改源码，Full Finding仍OPEN/P2-03 ACCEPTANCE，等待Host确认。

**前轮收尾限制（历史）**：最终build `acadf5a6…` 已完成工程验证及Ready/Starting/Restart Busy、首屏分区对照；Mac锁屏阻断最后下半页长图与回到顶部交接，待Host手动解锁。Runtime未退出，锁屏前Backend/Light/Ready。Full Visual仍OPEN，不能将局部截图称为整页验收完成。

# OpenBox Rust / GPUI 当前进度

**前轮Host Full Visual修复（2026-10-07，历史）**：P2-03-VISUAL-FULL-PAGE-002 **OPEN**；BUSY-001工程修复FIXED，不代表Full Visual PASS。按本机实际OpenBox Backend DOM与React/CSS重建四组连续视觉骨架，未来能力disabled/未知—，Profile仅从snapshot只读，Runtime按钮真实可操作。P2-03仍ACCEPTANCE，owner Runtime/Platform + GPUI，DAG为DONE18 / ACCEPTANCE1 / READY3 / TODO39 / DEFERRED7。Core336/Desktop80（既有1 ignored）、check/clippy/fmt/diff PASS；Runtime/Compiler/Sidecar代码不变，不重跑failure matrix。等待Host对当前系统显示设置、Light、完整Backend与sidebar确认；[完整视觉交付](P2-local-proxy.md#p2-03-visual-full-page-002)。

**前轮交付（2026-10-07，历史）**：仅 P2-03 READY→DOING→ACCEPTANCE；owner Runtime/Platform + GPUI（本对话为 Runtime 公共契约单一 owner）。68卡 DONE18/ACCEPTANCE1/READY3/TODO39/DEFERRED7，依赖不变。写范围：Core application/runtime 服务、singbox/runtime/clash_api/compiler 的必要动态日志支持及相关 Mock/tests；Desktop platform/manual sidecar、AppServices/StateBridge、app退出、服务卡/sidebar/i18n/tokens；本卡状态文档和local-only evidence/p2-03。不启动其它 READY/下游，不改 SystemProxy/TUN，不实现 P2-04。隔离 binary/root/fixture/bundle 仅本卡使用。Core336、Desktop73及显式真实内核1测试、check/clippy/fmt/diff PASS；受控代理请求与失败/退出清理 PASS，独立review无剩余可操作Finding。最终bundle已打开Backend/Light/当前Host显示设置/Ready，Host 150% Visual/Interaction PENDING；未解锁下游。见[本卡交付](P2-local-proxy.md#p2-03-delivery)。

**前轮Host最终收口（2026-10-07，历史）**：Host independent review PASS，P2-02B-RUNTIME-PROJECTION-001 CLOSED；P2-02B ACCEPTANCE→DONE，Codex Core/Config owner释放。Host source SHA/19定向/5 projection/7 catalog/332 Core及check/clippy/fmt/diff PASS；五个重新生成candidate经secret/cache path归一化后与修复轮fixed-kernel checked redacted candidates语义一致，无剩余可操作Finding。仅文档/evidence收口，旧首次DONE/重开/FAIL/修复候选保留，产品源码无修改。68卡DONE18/READY4/TODO39/DEFERRED7，其余0；READY=P0-08/P2-03/P4-02/P5-06，均未领取/未启动。见[Host最终收口](P2-local-proxy.md#p2-02b-host-closeout)。

**前轮修复候选（2026-10-07，历史）**：P2-02B-RUNTIME-PROJECTION-001 FIXED_PENDING_HOST_REVIEW，P2-02B DONE→DOING→ACCEPTANCE，owner：Codex Core/Config保留，等待Host独立review。显式 selected runtime projection / runtime closure / directForNodes修复；19定向/5 projection/7 catalog/332 Core、指定检查、固定v1.14.0五候选check与实现者自查PASS。旧DONE/327 Core/locked-check证据原样保留。P2-03/P4-02 READY→TODO，P0-08/P5-06保持READY。68卡DONE17/ACCEPTANCE1/READY2/TODO41/DEFERRED7；本轮临时archive/binary/config/secret cleanup PASS。无下游/UI/真实run。见[Finding](P2-local-proxy.md#p2-02b-runtime-projection-fix)。

**前轮 P2-02B 收口（2026-10-07，历史）**：最小产品Compiler五项验收PASS/DONE，Core/Config单一owner释放。14定向/327 Core、check/clippy/fmt/diff与固定v1.14.0四候选真实check PASS；48-plan digest不变；code-delivery-review实现者自查无剩余可操作Finding（非独立审查）。schema9严格兼容新Profile字段，旧v9/v8读取/版本/二次load稳定；普通保存仍SavedOnly。本轮仅生成/check，无run/UI/网络接管；现有dirtytree保留，无Git写操作。68卡DONE18/READY4/TODO39/DEFERRED7，其余0；READY=P0-08/P2-03/P4-02/P5-06，均未领取/启动。见[交付与边界](P2-local-proxy.md#p2-02b-closeout)。

**前轮文档收口（2026-10-07，历史）**：Host独立审查确认代码SHA一致、Core313/313与check/clippy/fmt/diff PASS，无可操作代码bug；第三项按目录层契约PASS，P2-02A ACCEPTANCE→DONE，Core/Config owner释放。P2-02A-CONSUMER-001 CLOSED：SCOPE_RESOLVED/DEFERRED_CONSUMER_INTEGRATION；正式ProxyService/UI行为与视觉集成仍属P2-08，当前NOT_RUN。68卡DONE17 / READY3 / TODO41 / DEFERRED7，其余状态0；READY=[P0-08,P2-02B,P5-06]均未领取，不启动下游。见[Host收口](P2-local-proxy.md#p2-02a-closeout)。

**初次实现交付（2026-10-07，历史）**：仅 P2-02A READY→DOING→ACCEPTANCE，Core/Config 单一 owner保留；代码/313 Core/指定检查PASS，实现者自查无未修复Finding。正式ProxyService/基础Proxy UI消费gap待Host Review；DONE16 / ACCEPTANCE1 / READY2 / TODO42 / DEFERRED7，READY=[P0-08,P5-06]且未领取，不启动下游。见[本轮交付](P2-local-proxy.md#p2-02a-delivery)。

**前轮最终收口（2026-10-07，历史）**：Host已明确确认build38-final完整订阅页面“符合”，绑定executable SHA256 `de220c173fb4cd70163419e93a54d34324b4dbf53617a87e46b32be8fafcbff3` 与build38 evidence。P2-01 DONE，P2-01-VISUAL-FULL-PAGE-001 CLOSED，Codex Desktop · Full Subscription UI Parity owner/资源预约释放。完整68卡：DONE16 / READY3 / TODO42 / DEFERRED7，其余状态0；READY仅P0-08/P2-02A/P5-06，均未领取/启动。cleanup NOT_RUN（沙箱拒绝进程身份查询），app/fixture/隔离root保留。见[最终收口](evidence/p2-01/final-closeout-20261007-122027/README.md)。

**前轮候选（历史）（2026-10-07，build38）**：仅移除 Source 正文常驻 Direct/TUN hint；不新增帮助UI或空占位，不改P0-06/未来业务。真实URL/Paste浅深色及整页6截图，metadata编辑description保留、两订阅重启可见；Core304/Desktop70/outbound9与九项检查PASS。build38窄范围Independent Review PASS（不等于Host Visual PASS）。自然高度及actual空feedback margin差异明示；Host Visual PENDING，ACCEPTANCE/OPEN/owner与DAG68保持。见[build38候选](evidence/p2-01/source-final-20261007-115918/BUILD38-SOURCE-CANDIDATE.md)。

**前轮候选（2026-10-07，历史）：build37完整订阅UI候选新增只读Share弹窗、4px表单间距、38px分享列表与长页面modal分层。真实浅深色Source/Rules上下/DNS/Nodes/Share、11项滚动、busy/error/stale及三语已核对；Core304/Desktop70/outbound9与九项检查、独立Review PASS。Host全页视觉仍PENDING；P2-01 ACCEPTANCE、Finding OPEN、owner保留。未来能力控件保持禁用；periodic开启等条件分支未开放，不宣称全业务UI已验收。DAG68不变，仅P0-08 READY，无下游/commit/push。见[build37候选](evidence/p2-01/all-ui-20261007-111729/BUILD37-FULL-UI-CANDIDATE.md)。**

最后更新：2026-10-09。

**前轮候选（2026-10-07，历史）：完整页面候选 build29-final 按本机 actual OpenBox 逐控件修复按钮 hover/focus、说明、DNS/Rules、saved Nodes table、modal与动画；Core304/Desktop70/outbound9及九项验证、Independent Review PASS。完整矩阵与真实浅深色截图已采集，Host Visual仍PENDING；最后交接因Mac锁屏待解锁核验。P2-01 ACCEPTANCE、Finding OPEN、owner保留；仅P0-08 READY，无下游启动。见[build29全控件修复](evidence/p2-01/full-reaudit-20261007-094122/BUILD29-CONTROLS-REWORK.md)。**

**前轮完整页面结果（2026-10-06，历史）：Host对最终build16完整页面回复“仍有偏差”，当前Visual FAIL/REWORK；P2-01-VISUAL-FULL-PAGE-001 OPEN；P2-01 DONE→ACCEPTANCE，owner为Codex Desktop · Full Subscription UI Parity。上一轮视觉PASS仅覆盖当时限定实现区域，不代表完整SubscriptionSettings页面。历史功能PASS及旧evidence保留。P2-02A/P5-06 READY→TODO；仅P0-08 READY，不启动下游。**

**P0-06收口历史（2026-10-06）：P0-06 DONE，显式出站代码/本地契约/安全公网观测/cleanup与独立review完成，owner释放。** 原卡第4项允许未证明组合明确范围决定：SystemProxy ON为UNSUPPORTED，TUN OFF/managed bypass为INCOMPLETE_EXTERNAL_TUN_ACTIVE；外部TUN保护且未修改。共DONE15；READY仅P0-08/P2-01，未领取/启动。P0-05源码/evidence/验收与dirtytree完整保留；P1-07视觉PASS_WITH_TECHNICAL_DIFFERENCES与历史限制保留。[本轮完整证据](evidence/p0-06/20261006-144632/README.md)。

- [长期开发总规范](DEVELOPMENT_WORKFLOW.md)：后续 Veyra Rust/GPUI 默认遵守；四泳道、并行与单一 owner。
- [技术方案](../openbox-rust-gpui-implementation-plan.md)：架构、行为、接口与平台边界。
- [任务总表](IMPLEMENTATION_PHASES.md)：唯一状态表、显式依赖与覆盖归属。

**全局规范新增（2026-10-04）**：[UI 视觉迁移与组件复用强制规范](../../AGENTS.md#ui-视觉迁移与组件复用强制规范)及[长期视觉/组件流程](DEVELOPMENT_WORKFLOW.md#31-长期-ui-视觉迁移与组件抽象流程)已固化。除 OS 绘制且 React 不拥有的 macOS 原生区域外，React/OpenBox 源码与 CSS 是唯一视觉事实来源；固定值精确迁移、95% 最低目标、逐页面/核心状态验收、Tokens/共享组件层级与 Legacy 视觉参考保留均为强制要求。规范新增不代表现有实现已满足或通过视觉验收，历史证据保持原结果。

## 1 进度总览

| 里程碑 | DONE / 任务数 | 当前结果 | 组合验收 |
| --- | --- | --- | --- |
| P0 基线与可行性 | 7 / 9 | P0-01/03/04/05/06/07完成；P0-06保留未证明组合的明确范围决定，历史限制保留 | P0-09 未开始 |
| P1 核心库与桌面壳 | 8 / 8 | 壳层/设置/基础组件视觉 PASS_WITH_TECHNICAL_DIFFERENCES；人工三轮、重启恢复、最终 Tray Quit 与清理 PASS；未实现业务页不计完成 | P1-07 DONE，组合 PASS |
| P2 本机代理闭环 | 5 / 10 | P2-01/P2-02A/P2-02B/P2-03/P2-04 DONE；Host FINAL ACCEPTANCE PASS，Finding CLOSED | P2-09 未开始 |
| P3 观测与主页面 | 0 / 8 | 未开始，基础能力不计完整 DNS/分流 | P3-08 未开始 |
| P4 完整配置能力 | 1 / 9 | P4-02 DONE，P4-03 READY；Chain 在 Routing 前交付 | P4-07 未开始 |
| P5 DNS 与共享 | 3 / 7 | P5-04/P5-05/P5-06 DONE；Rules 在 DNS 后最终闭合 | P5-07 未开始 |
| P6 macOS TUN 与生命周期 | 0 / 5 | 未开始，按各卡依赖推进 | P6-05 未开始 |
| P7 数据与发布收尾 | 0 / 5 | 未开始，schema/清理按显式依赖等待 | P7-05 未开始 |
| Windows W0–W3 | 0 / 7 | 后续排期 DEFERRED | W3-02 未开始 |

macOS：**24 / 61 完成**；READY 4、TODO 32、DOING 1、REVIEW 0、ACCEPTANCE 0、BLOCKED 0。Windows 7 项 DEFERRED 单列，共 68 项。数量不等于工期权重或代码完成百分比；P1-04/P2-02/P4-05 父项被后缀子项替代，不重复计数。

## 2 Active Tasks

| Task | owner / 泳道 | 写范围 / 公共契约 | 实际资源 / 下一动作 |
| --- | --- | --- | --- |
| P2-06 · DOING | Codex · Runtime/Platform | helper production/IPC、Core Runtime DTO、desktop runtime_service/platform、局部 Cargo 接线、任务文档 | checkpoint f2457e6 已提交、仍 DOING；本轮无资源操作；既有实现/限制按下方历史保留，GUI/Native/Helper 多轮及重启恢复后期补齐；当前先协调 P4-02/P5-06 UI 及其它 READY 写范围，不继续以复杂恢复阻塞其它功能，现有安全拒绝保持 |

P5-05 owner/预约已释放，自有GUI已停止、临时请求拦截/视口已清理，原图/bundle/隔离测试数据保留；详见[P5-05验收](P5-05-acceptance.md)。

P5-06 本卡 Core/Config + GPUI owner/预约已释放；自有监听与进程清理，测试数据/日志/原图/bundle保留，详见[最终验收](P5-06-acceptance.md)。

P4-02 本卡Core/Config + GPUI owner/预约已释放，本轮自有测试App/数据/harness已清理（见cleanup.json），原轮资源与用户资源未改动。P2-04 Runtime/Platform及Runtime公共契约owner/本卡预约已释放；P2-03既有预约释放保持，其历史文档收口不追认现场清理。

P0-06/P0-05 owner与本轮隔离资源预约已释放。P0-05 历史起始 HEAD `3111a57`。历史 P1-03 起始 HEAD `f460e767957569e9f4a035132e26cc216277e98f`，工作树干净；当时只领取 P1-03；历史 P1-04A 起始 HEAD a71b664，干净；现已由 Host 提交 c184325，成为历史 P1-04B 基线；P1-04B 已由 Host 提交 f9adda4，本轮 P1-05 基线干净、完成后未提交，不启动下游。独立 label/path/socket/service/child/tmp 的现场预约已释放；补验时重新检查身份、版本与无残留。

P1-07 已 DONE 并从 Active Tasks 移除，GPUI owner 与本卡资源预约释放；隔离 root/bundle 和 evidence 保留供 Host 复核，无自有 Veyra 验收进程残留。

## 3 Ready Queue

本队列仅由完整68-card显式依赖计算；READY不等于领取或启动。

| Task | 依赖满足依据 | owner / 下一动作 |
| --- | --- | --- |
| P0-08 · READY | P0-03、P0-05均DONE；后续下载消费P0-06显式client与范围限制 | 未领取；不启动 |
| P2-05 · READY | P2-03、P0-06均DONE | 按依赖/写范围适时领取或并行；仍为 P2-08 前置，协调 Runtime/DTO owner；本轮未启动 |
| P3-01 · READY | P2-03、P0-07均DONE | 未领取；不启动 |
| P4-03 · READY | P4-02、P2-04均DONE；全量DAG重算新增 | 未领取；Runtime/DTO写范围需协调P2-06 owner，不在本轮启动 |

当前68卡：DONE24 / ACCEPTANCE0 / DOING1 / REVIEW0 / READY4 / TODO32 / BLOCKED0 / DEFERRED7；READY仅P0-08/P2-05/P3-01/P4-03，均未领取/未启动；P2-04 DONE、Finding CLOSED保持。P2-06 DOING、Runtime/Platform及Runtime公共契约owner=Codex；P2-07/P6依赖未满足，不转READY。其它状态/显式依赖不变。

**P2-03当前契约**：正式运行配置必须调用P2-02B `compile_product(ProductCompileRequest { state, runtime_intent, default_outbound, resources })`，显式消费 `project_selected_runtime()` 的 `runtime_intent` / `projected_default_target`（转换为 `OutboundId`）；不得继续使用 `application/runtime.rs` 现有ObservationOnly `compile(...)`作为正式运行配置。参见[P2-03任务卡](P2-local-proxy.md#obg-p2-03)。已实现并由Host最终确认功能与视觉“符合”；本轮仅文档收口，不启动下游。

## 4 Blocked

没有BLOCKED任务；P2-04 DONE、P2-04-PENDING-PERSISTENCE-001及前轮P1/P2 Finding CLOSED，owner/预约已释放；P2-06 DOING并保留Runtime/Platform owner。P2-04独立Git提交隔离曾BLOCKED，仅为当时版本控制基线阻断；现已随七任务提交链整合解除，不将已验收Task改为BLOCKED。P2-03-BACKEND-BASIC-SETTINGS-004/P2-03-VISUAL-FULL-PAGE-002/P2-03-VISUAL-FIDELITY-003 CLOSED，Host已确认Basic2 build功能与视觉“符合”；BUSY-001历史不改写。P2-01-VISUAL-FULL-PAGE-001 CLOSED，Host已确认build38-final完整页面“符合”。P0-05原卡真实GUI owner缺口已关闭，E按原始定义PASS；executable/service/root-total-deadline均optional hardening NOT_RUN。历史macOS15实机、字体/blur/截图等限制保留于原任务证据，不伪造新结果。

## 5 基线与验证记录

上轮P0-06基线（历史）：branch `codex/dist-react-restore` / HEAD `18a54d26ca171c1b00cc497b233c6f493aac43a9`，仅本卡core出站原型/测试/只读工具及文档/evidence，附带最小既有test clippy修订。开始已有dirtytree全部保留，P0-05 437文件与卡片区段SHA不变。下表P0-05历史行按各轮当时事实保留。

| 字段 | 当前基线 / 按日期保留的验证记录 |
| --- | --- |
| P0-06 本次验证 | [完整出站/范围调查](evidence/p0-06/20261006-144632/README.md)：新增9、默认core294/feature303串行、subscription68/bridge14/observation1、helper47与指定check/clippy/fmt/diff PASS；真实Direct/自有mixed两HTTPS均返回104.28.196.30，route仍utun7仅外部TUN观测。四组合UNSUPPORTED/INCOMPLETE明示；cleanup/主网络hash/P005保护/独立review PASS；原并行Document Busy与初轮fixture/clippy FAIL保留 |
| 历史分支 / Host收口时提交基线 | `codex/dist-react-restore` / `0230aa72539375b622422e0e76e009a399b81669`；P2-01～P2-04基础仍有大量未提交/未跟踪内容，当前index无已暂存修改。P2-04 DONE但独立可重建提交隔离BLOCKED，未stage/commit、index不动；待Host后续基线整合，无下游启动 |
| 当前分支 / 文档同步起点 | `codex/dist-react-restore` / `fdb8930a010ed321e496caa62a98257c64031b8a`；七任务提交链与远端历史合并已完成，产品文件树与 `dcc9b7b2` 一致；本轮仅四份文档同步，`.gitignore`、`AGENTS.md`、`.DS_Store` 保持原样 |
| P2-04 当前 Host FINAL ACCEPTANCE | [收口与receipt](evidence/p2-04/final-host-closeout-20261008/README.md)：Host PASS、三Core aggregate 987a0178…及harness精确匹配，77事件/14原命令/Native与限定注入PASS，Host新复跑Core371/Desktop85 exit0；Task DONE、Finding CLOSED、owner/预约释放。本轮仅静态文档验证，源码/harness SHA不变、Native/GUI/cleanup NOT_RUN |
| P0-06历史执行基线 | `18a54d26ca171c1b00cc497b233c6f493aac43a9`；local history rewrite前的历史身份保留，不冒称当前HEAD |
| 新路线实现基线 | P1-07 起始基线 b982e8d；最终 visual build SHA 8bd7a366d24fa2eff8bf78e22604f461030cedae9d1184c11a28ca8eb4f5c28d；Host acceptance commit `25eb020`；[build identity](evidence/p1-07/final-20261005/build-identity.json) 的源码身份与提交内容一致 |
| 方案调查基线 | `bda242a920d471b9598f98b57dde5d7c2505c35e`；只作原调查身份，不能当当前 HEAD |
| P0-01 交付（历史保留） | [最终验收](P0-01-baseline.md#current-acceptance)：8 当前源码 PNG、18 新真实脱敏 case、视觉数据、manifest/audit；四项 PASS |
| P0-01 验证（历史保留） | 77 AST（72/5）、4/6/9、54 浏览器操作/状态断言、8 PNG 与来源/脱敏/JSON/links/68 DAG；[最终计数](evidence/p0-01/validation.json) |
| P0-01 NOT_RUN（历史保留） | 当时未运行产品构建/测试、内核/权限原型；traffic 重连/重置与 DNS/failover 写语义未验证，保持 UNKNOWN |
| P1-01 本次验证 | [抽取记录](P1-core-and-shell.md#p1-01-delivery)；core 243 测试（178 明确纯单测）、旧入口 79 Mock/DTO 回归；build/tree/静态检查、两条 lib clippy/fmt/diff PASS；额外测试 lint FAIL 见记录 |
| P1-02 本次验证 | [类型/快照交付](P1-core-and-shell.md#p1-02-delivery)：275 core（新增 32）、65 旧入口定向回归；四项验收 PASS，check/build/lib clippy/fmt/tree/静态/文档/diff 通过；不调用真实内核 |
| P0-03 原型验证（历史保留） | [实机原型](evidence/p0-03/README.md)：check/build/clippy/fmt/core/tree 通过；11 截图，中文编辑/Modal/list 真实操作；无资源覆盖 workspace check FAIL（旧 Windows LICENSE 缺失） |
| P0-03 人工验收收口（2026-10-04） | [人工确认](evidence/p0-03/host-manual-acceptance.json)：用户亲手中文 Input/Modal 输入及托盘点击可用，明确批准四项 PASS/DONE；[验证](evidence/p0-03/validation.json)：两条指定 cargo check 复跑 PASS、JSON/11 PNG/68-card DAG/links/diff 通过；resource override 不代表打包资源完整 |
| P0-04 本次真实验证 | [真实内核交付](P0-feasibility.md#p0-04-delivery)：官方 1.14.0 arm64 digest、5 configs check、4 child 动态 controller/鉴权/SIGTERM 清理、selector/FakeIP restore、关闭 writer handoff PASS；存在对账前窗口；[validation](evidence/p0-04/validation.json)含 core check/脱敏/JSON/DAG/links/diff；仅同版本/cache_id/tag/pool/range |
| P0-07 本次真实验证 | [本地 Observation 交付](P0-feasibility.md#p0-07-delivery)：固定 archive/source、四 WS、traffic 两 burst/reconnect、long/short、DNS records/flush/hot rewrite、局部诊断 PASS；[validation](evidence/p0-07/validation.json)含 JSON/AST/脱敏/core check/68 DAG/links/diff/cleanup；首轮 checker FAIL 保留，无产品 Rust 改动 |
| 保留的限制 | candidate-window/tray screenshot 与独立前台 focus 缺证据、受控 window blur 对照缺失；card-level backdrop blur、MiSans、nested dropdown、macOS 15 实机未验证，不阻止 P0-03 DONE；Windows 仍 DEFERRED |
| P0-05 历史原型（2026-10-04） | [授权与平台证据](evidence/p0-05/README.md)：macOS 27 arm64 / minos 15.0 / adhoc，固定内核/manual FD、普通用户 OS peer/NOTE_EXIT PASS；管理员授权失败 -60008，所有 root/network 实测 NOT_RUN，ACCEPTANCE；现场干净 |
| P0-05 Desktop 补验（2026-10-06） | [真实批准/启动失败](evidence/p0-05/desktop-privileged-20261006-113208/README.md)：两次标准 UI 批准、安装/卸载执行，socket startup timeout；隔离 Service disabled/not-in-set 创建并删除，独立 cleanup PASS、主用网络 hash 不变。5 定向测试/prototype check/build/clippy/fmt/core check PASS；root IPC/child/Proxies/真实 GUI 未执行，保持 ACCEPTANCE |
| P0-05 新路径复验（12:03 起） | [真实取消/复制修复/待批准](evidence/p0-05/desktop-pathfix-20261006-120343/README.md)：USER_CANCELLED -128 PASS、取消后 manual PASS；初轮安装 owner 错误、标准 UI 固定清理 PASS。14 Rust/2 Python/指定检查/独立代码 Review PASS；修复后 UI 未见/240s 超时，新路径 daemon/socket/A–E 尚未完成。最终独立 cleanup PASS、主用网络三组 hash 不变，ACCEPTANCE |
| P1-04B 本次验证 | [行为偏好交付](evidence/p1-04b/README.md)：293 core/24 desktop；规定 check/build/clippy/fmt、resource override workspace check/旧 lib clippy/tree PASS；实际字段编辑/校验、两轮真实冲突 rebase、磁盘失败 Retry、两轮重启和9张图通过；无网络/TUN/内核操作 |
| P1-05 本次验证 | [平台交付](evidence/p1-05/README.md)：294 core/33 desktop，规定构建/检查与脱敏截图 PASS；真实五次 secondary active/key、busy unconfirmed、SIGKILL/stale恢复、原生 Open/Save Cancel/Accept、边界拒绝、Cmd+V/已物化 payload 恢复、loopback OS handoff；正常退出 socket 修复后 PASS。额外旧 clippy 原资源 FAIL/override PASS 明示；所有 task root/进程已清，无系统代理/TUN/内核/公网动作 |
| P1-05 Host Review 修订 | [修订证据](evidence/p1-05/README.md#host-review-revision)：仅三处边界修复及证据措辞；新增 3 Desktop 回归，修复前均 FAIL，修复后 294 core / 36 desktop、用户指定检查与旧 lib clippy PASS。既有 GUI/源码身份保留，本轮未重跑 GUI；P1-06 保持 READY/未开始，无 commit/push、公网、sing-box/System Proxy/TUN |
| P1-06 人工验收收口（2026-10-04） | [User/Host manual acceptance](evidence/p1-06/host-manual-acceptance.json)：V 图标、真实菜单/准确未接入状态/灰色禁用启停、连续三轮关闭→托盘恢复、最终托盘退出均人工 PASS；Host 已查看两张截图，未复制入仓库，无本地图片/SHA。用户确认 Host 此前独立复核及 41 Desktop/294 Core 与指定检查全部 exit 0；本次不复跑产品验证，窗口 Quit 清理与托盘人工结果分开 |
| P1-07 最终收口 | [Host 复核](evidence/p1-07/final-20261005/host-final-review.json)：视觉 PASS_WITH_TECHNICAL_DIFFERENCES、组合 PASS，三项 blocker 已处理；[最终人工重启/退出](evidence/p1-07/final-20261005/human-host-final-restart-quit.json) 及 process/socket/flock cleanup PASS。首跑 Desktop 44 PASS/1 FAIL 保留，定向5/5、单线程45/45、默认并行45/45 PASS；仅记未复现的瞬态测试环境/flock 时序失败，目前无稳定回归证据，根因未证明；原日志不改 |
| 下一里程碑 | P2-02B DONE；P2-03 ACCEPTANCE等待Host；P0-08/P4-02/P5-06 READY且未领取/启动 |

| P0-05最终GUI验收（2026-10-06） | [真实 GUI owner 最终验收](evidence/p0-05/gui-owner-final-20261006-142752/README.md)：Host确认ready窗口，GUI/marker/helper owner74759一致，child74789/ports59100–59101/FD3 ready，SIGKILL→owner_NOTE_EXIT/exactrestore/childcleanup/finalcleanup/mainhash unchanged PASS；13条命令exit0（9GUI+47helper+2Python，默认GPUI0 tests不计用例PASS），独立Review PASS。A–E原卡PASS，optional三项NOT_RUN；DONE/owner释放，仅两个下游READY |

## 6 里程碑记录

- [ ] P0 路线可行、范围明确与估算完成。
- [x] P1 桌面壳、配置保存与托盘可用（视觉 PASS_WITH_TECHNICAL_DIFFERENCES；业务页面仍由后续任务验收）。
- [ ] P2 首个可用 macOS 本机代理版本验收通过。
- [ ] P3 主页面基础能力、实时观测与历史查询验收通过。
- [ ] P4 高级配置、统一出口目录与 routing/client routing 验收通过。
- [ ] P5 DNS、Rules 完整集成与共享功能验收通过。
- [ ] P6 TUN、生命周期与更新集成验收通过。
- [ ] P7 macOS 候选安装包完成实际验收。
- [ ] Windows 单独排期及设备验收完成。

## 7 最近记录

2026-10-07 文档/evidence收口：Host独立审查和当前源码验证PASS；P2-02A第三项以Compiler/selected/runtime projection消费及唯一query seam交付满足，本层不得反向等待依赖它的P2-08实际UI集成。消费者gap CLOSED（SCOPE_RESOLVED/DEFERRED_CONSUMER_INTEGRATION），P2-02A DONE/owner释放。仅改三份任务文档与新增收口evidence，未改产品代码/重跑产品测试/启动下游；旧FAIL和自查历史保留。DAG依赖不变，DONE17/READY3/TODO41/DEFERRED7，READY=[P0-08,P2-02B,P5-06]。见[收口](P2-local-proxy.md#p2-02a-closeout)。

2026-10-07 初次实现交付（历史）仅P2-02A：Base OutboundCatalog/类型化OutboundId、统一图校验、application查询快照/service seam、既有projection与Compiler消费完成；313 Core/48字节回归/指定check-clippy-fmt-diff PASS，实现者自查无未修复代码Finding，正式ProxyService/Proxy UI尚不存在，P2-02A-CONSUMER-001 OPEN。第三卡项未全勾，ACCEPTANCE/owner保留，交Host Review；68卡DONE16/ACCEPTANCE1/READY2/TODO42/DEFERRED7，READY=[P0-08,P5-06]；无下游/commit/push。初轮编译FAIL与311/1错误优先级回归保留并已修复。见[交付](P2-local-proxy.md#p2-02a-delivery)。

2026-10-06 本轮仅 P2-01 READY→DOING→ACCEPTANCE：复用既有订阅服务、provider replacement、Direct fetch 与单一 JSON 事务；新增正式预览/重校验保存、Direct CAS 刷新/编辑/删除和 Settings → Subscriptions。Core304/Desktop67 串行 PASS、定向 core45（新增10在最终全量覆盖）、bridge/model、P0-06 outbound9、check/clippy/fmt PASS；初轮 fixture/compile/clippy FAIL 日志保留。实现者按 code-delivery-review 自查，无未修复代码 finding；全部缺失的真实 GUI/浅深色/截图留 NOT_RUN。P0-05/P0-06 历史不改；owner 保留，仅 P0-08 READY，P2-02A/其余下游未启动，未 commit/push。见[交付](evidence/p2-01/README.md)。

2026-10-06 14:27起最终GUI owner验收收口：P0-05 DONE、owner释放，READY=[P0-06,P0-08]，未启动下游、未commit/push。以下为各轮当时记录，历史失败/缺口不改写。

2026-10-06 12:03 起仅 P0-05 新路径实机复验：保留 Host 修复与历史 evidence，取得真实 UI cancel -128/Host 确认和取消后 manual PASS。初轮批准安装在 bootstrap 前被 owner 保护检查拒绝；独立诊断证实 fs::copy 保留业务 UID，部分安装先经标准 UI 固定 SHA/type/空 runtime/无 endpoint 等检查安全卸载，现场独立清理 PASS。只把 helper/kernel 改为 root 安装进程 create_new/0600 后复制字节，原 hash/mode/protected 保留，新增一项纯回归；14 Rust/2 Python/指定检查及独立代码 Review PASS。修复后批准调用 240 秒超时，Host 未见授权窗口，立即停止特权分支；root IPC/child/代理恢复/owner/安全拒绝尚未执行。最终清理 PASS、主用网络三组 hash 不变；68 卡 READY=[]、P0-05 ACCEPTANCE，未释放 owner、未启动下游、未 commit/push。见[本轮报告](evidence/p0-05/desktop-pathfix-20261006-120343/README.md)。

2026-10-06 仅 P0-05 真实特权补验：标准授权 UI 可显示，两次实际批准均执行固定安装/卸载，但原 linker-signed 及 explicit ad-hoc staging 候选均未建立 daemon socket。平台日志与缺失退出诊断、无真实取消/root IPC/child/代理恢复/GUI SIGKILL 样本如实保留；未把 CLI owner 当成 GUI。候选签名经独立 Review 后实测失败，已移除工作源码改动，最终只更新本卡文档/evidence。独立清理 PASS、主用网络三组 hash 不变；68-card DAG DONE13/ACCEPTANCE1/TODO47/DEFERRED7、READY 空，P0-06/P0-08 未解锁/未启动，未 commit/push。见[本轮报告](evidence/p0-05/desktop-privileged-20261006-113208/README.md)。

2026-10-04 全局 UI 视觉迁移规范（仅文档）：更新 AGENTS.md、长期开发规范、方案 §9.3/§9.4 与迁移退役说明、P1-07 验收及本交接页。新增唯一 React/CSS 视觉事实来源、固定值精确迁移、GPUI Kit 外观覆写、字体/图标同源、95% 最低工程目标与逐页面/核心状态对照、Tokens/共享组件体系及视觉参考退役条件。P1-07 下一动作先审查当前壳层/设置视觉与组件体系，Proxies/Connections/Logs/Rules 留各自后续任务验收；仍唯一 READY、未领取/启动。68 卡状态/依赖不变：DONE12 / ACCEPTANCE1 / READY1 / TODO47 / DEFERRED7。本次不改产品源码、不运行 GUI/产品测试/构建、不访问公网或远端参考、不 commit/push。

| 日期 | Task / 文档动作 | 结果 | 后续 |
| --- | --- | --- | --- |
| 2026-10-03 | 创建任务清单与交接页（历史） | 当时 57 macOS + 7 Windows，初始均待办 | 历史计数保留 |
| 2026-10-03 | P0-02 流程/范围清理（历史） | 当时 1/57；64 任务、77 API、19 场景、150 本地链接与格式通过；仅发现 29 产品测试文件，未运行；Ruby 技能字段校验通过，Python 因缺 PyYAML 未运行 | 详细历史证据见 [P0 清理记录](P0-feasibility.md#cleanup-record)，不改写为本次结果 |
| 2026-10-03 | DAG 与长期开发规范（文档动作） | 最小拆分后 61 macOS + 7 Windows；仅 P0-02 DONE、P0-01 READY；153 个本地 Markdown links/anchors、68 Task 唯一/依赖存在/无环、计数与 READY 状态、77 API/19 场景归属、指定依赖/并行关系及差异格式检查 PASS | 实现从 Ready Queue 领取；本次无产品构建/网络/权限结果 |
| 2026-10-03 | F1–F7 Review 问题修复（仅文档） | 重新读取文件校验：153 本地链接/锚点、68 Task ID/依赖存在/无环、主表/卡一致、61+7 计数/状态、77 API 单一归属、19 场景、F1–F7 断言与格式/新文件 whitespace PASS；仅 P0-02 DONE、P0-01 READY | 未运行产品构建/测试/网络/权限；修复者做定向自查，不声称另有独立人工 Review |
| 2026-10-03 | P0-01 基线调查 | 静态 77/4/6/9、22 case 交付；仅视觉 NOT_RUN，ACCEPTANCE；[交付证据](P0-feasibility.md#baseline-record) | 补同尺寸五类浅深色视觉及身份后复核 DONE，重算下游；未改生产配置 |

| 2026-10-04 | P1-05 平台目录/单实例/原生文件 | 三项验收 DONE；P1-06 唯一 READY；P0-05 保持 ACCEPTANCE，P0-06/P0-08 未解锁 | 未 commit/push，未开启下游，Host Review 决定提交 |

每次实际推进后同步 Active Tasks / Ready Queue / Blocked、主表与计数；详细证据放相关任务文件或正式交付记录，旧交接留 Git 历史。

2026-10-03 CLI 替代路径实际完成只读请求及本机 HTML 身份核对；真实视觉未完成，站点授权不是当前缺口。详见 [只读观测与环境下一步](P0-01-baseline.md#readonly-observation)。临时 Vite 自有进程已停止；未安装依赖、改产品源码或生产配置，未 commit/push。

2026-10-03 仅 auth/login 的临时鉴权例外已实际执行一次，仍 403，未建会话；13 只读 GET 重采样及 4 WS 失败观测已脱敏登记，未取得 DTO/帧。jar/鉴权脚本删除；单次浏览器确认仍失败。P0-01 ACCEPTANCE、READY 空；见 [临时登录证据](P0-01-baseline.md#temporary-auth-observation)。

2026-10-03 最终验收：P0-01 DONE（四项 PASS），P0-02 DONE；3 READY/56 TODO/7 Windows DEFERRED。当前真实 Chromium 8 图和实际操作已通过；早前 403、启动失败、无帧与尺寸解析错误均保留在历史记录。本轮 GET 15、登录 POST 2、WS 4，其余写 0；临时资源清理完成，无 commit/push。见[当前最终验收](P0-01-baseline.md#current-acceptance)。

2026-10-03 P1-01 DONE：根 workspace/core 和旧入口共享实现，三处耦合及订阅系统代理读取已处理；3/61 DONE、3 READY、55 TODO。P1-02 READY，P0-03/P0-04 仍 READY；未启动下游，P0 历史保留，无 commit/push。见[交付记录](P1-core-and-shell.md#p1-01-delivery)。

2026-10-03 P1-02 DONE：schema v7、StateVersion/独立配置与选择版本、三态 ProfilePatch、类型化保存/错误与 Runtime owner 快照边界；四项验收 PASS。275 core/65 旧入口定向回归与规定检查通过。macOS 4/61 DONE、2 READY、55 TODO；READY 仅 P0-03/P0-04，P1-03 等待 P0-03 保持 TODO。owner 释放、未启动下游、无 commit/push/publish；[交付记录](P1-core-and-shell.md#p1-02-delivery)。

2026-10-03 P0-03 追加：ACCEPTANCE；Rust 1.99.0 / Kit 0.7.0 / gpui-pre 0.3.7 / tray-icon 0.24.2，macOS 27 arm64 构建 minos 15.0。四项卡仅构建项勾选；P0-04 READY，P1-03/P0-05 继续 TODO。未 commit/push，未开始下游。

P0-03 Host 追加确认：对绿色托盘图标、三轮显示/隐藏、菜单状态更新和退出回复“完成”；退出后精确进程检查无残留。按 Host 报告记录，不代替缺失的托盘截图/关闭后前台 focus 与 IME 证据。


2026-10-04 P0-03 Host/User Manual Acceptance：用户亲手完成真实中文输入（单行 Input 与 Modal Textarea）及托盘点击并确认可用，明确要求“就当验收通过了”。四项卡验收 PASS、P0-03 DONE；既有 IME candidate/composition、tray screenshot/focus、controlled blur LIMITATION 和无资源覆盖 workspace check FAIL 保留，不补造截图。两条指定 cargo check 本次复跑 PASS；[完整验证](evidence/p0-03/validation.json)。按 68-card DAG 重算 macOS DONE 5、READY 2、TODO 54、ACCEPTANCE 1；Windows DEFERRED 7。P0-04/P1-03 READY，P0-05 TODO；未启动下游、未改原型功能代码、未访问 OpenBox 远端、未运行真实 sing-box、无 commit/push。

2026-10-04 P0-04 DONE：固定官方 v1.14.0 macOS arm64 archive digest 匹配后运行；5 config checks / 4 child，动态 controller 从本 child 日志取得，鉴权 200/无鉴权 401；selector/FakeIP 及 closed-writer handoff 全通过、空缓存对照排除假恢复。restart/handoff 对账前窗口 22.990292/14.466458 ms；正式及试探资源已清理。68-card DAG：DONE 6、READY 3、TODO 52、Windows DEFERRED 7；READY P0-05/P0-07/P1-03、P0-06 TODO。前期字段 check FAIL 与试探配置未捕获 hash 的限制保留；sandbox 网络更新监听受限，不声称正式 Runtime/网络监听已实现。无系统设置修改、TUN run、公网协议测试、后续 Task、commit/push；[完整记录](evidence/p0-04/README.md)。

2026-10-04 P0-07 DONE：四项验收 PASS；traffic 区间 bytes、两 burst/reconnect、短连接缺口、DNS records/hot rewrite unsupported、flush supported、真实局部诊断；全部自有资源清理。READY P0-05/P1-03，无新增，未启动下游，未 commit/push；[完整记录](evidence/p0-07/README.md)。


2026-10-04 P0-05 ACCEPTANCE：隔离 Rust/原生 adapter、closed IPC、受管配置 FD/身份/恢复逻辑与固定安装/卸载 harness 已实现；普通用户 API/手动代理及 5 定向测试有真实证据。标准系统管理员授权实际失败 -60008 / hiservices-xpcservice Connection Invalid；未创建 daemon、受保护目录或 Network Service，root/network 项 NOT_RUN，无真实用户取消证据。现场已清理，68-card DAG 不变，READY 仅 P1-03，未启动下游、无 commit/push；[完整证据](evidence/p0-05/README.md)。

2026-10-04 P1-03 ACCEPTANCE：正式 desktop 六页/九分类与单 Tokio/Core bridge 完成；11 tests、desktop check/build/clippy、fmt/tree、resource override workspace check 及旧 lib clippy PASS。实际 GUI 六页/九分类、filter/draft、Busy、迟到/旧实例、坏 JSON/Retry、浅深色 11 图已取得；随后仅持有表参数化以增加两项离线纯回归，最终原生 Entity 复核因 Mac locked 未运行。保留 ACCEPTANCE、READY 空，未启动下游；详情及现场清理见 [P1-03](evidence/p1-03/README.md)。

2026-10-04 P1-03 最终补验 DONE：用户解除锁屏后，最终构建原生 Entity 三组断言、filter/DNS category/draft 往返全部 PASS；新增 final-settings-retained.png，共 12 图。通过 UI 正常退出，无 desktop/child 残留，四个隔离 root/坏 JSON/临时 app 清除。68 卡重算：macOS DONE 8、ACCEPTANCE 1、READY 3、TODO 49；Windows DEFERRED 7。READY 仅 P1-04A/P1-04B/P1-05，下游未开始。无 commit/push；[完整证据](evidence/p1-03/README.md)。

2026-10-04 P1-04A READY → DOING；owner Codex Desktop · GPUI / Core-Config，公共视觉 DTO 单一 owner。P1-04B/P1-05 保持 READY，不启动下游。

2026-10-04 P1-04A DONE：三项验收通过；schema8 的 v7→v8 显式迁移保留 epoch/双 revision，视觉 CAS/SavedOnly/no-op、合并保存及背景 reference 原子提交完成。280 core（新增5）/17 desktop（新增6）、指定 check/build/clippy/fmt/workspace check PASS；resource override 仅 check，不代表旧包资源齐全。实际 GPUI 重启/保存失败/Retry/键盘/焦点与浅深色27图，用户亲手 IME/连续 Slider/stable drag 回复“全部正常”；Input裁切、侧栏icon、底栏对比三项反馈修复并复测。所有隔离根/资产/app/staging和进程已清；无代理/TUN/公网/内核操作，无 commit/push。68卡实际依赖重算：macOS DONE9、ACCEPTANCE1、READY2、TODO49；Windows DEFERRED7。READY仅P1-04B/P1-05；P0-05仍ACCEPTANCE，下游未启动，owner释放。[完整证据](evidence/p1-04a/README.md)。

2026-10-04 P1-04B READY → DOING：只领取行为 Preferences；P1-05 保持 READY；P0-05 保持 ACCEPTANCE；AppConfig/Panel 共享 owner 由 Codex Desktop · Core/Config + GPUI 预约。

2026-10-04 P1-04B DONE：schema8→9 保留 identity/双 revision/visual/Profile；行为 typed partial patch、SavedOnly/CAS/no-op、三种 URL 与两种 IPv6 独立、四类消费者通知契约完成。293 core（新增13）/24 desktop（新增7）及所有规定构建检查 PASS。实际 Panel 全字段/错误、真实磁盘失败与草稿、两轮冲突仅重放 timeout 并保留外部列数、最终重启 config16/selection0/同 epoch 和9图通过。隔离资源/进程已清，无公网/代理/TUN/sing-box/HTTP/login，未 commit/push。68卡重算：DONE10/ACCEPTANCE1/READY1/TODO49、Windows DEFERRED7；READY仅P1-05，未领取；P0-05仍ACCEPTANCE，下游未启动；共享 owner 释放。[完整证据](evidence/p1-04b/README.md)。

2026-10-04 P1-05 READY → DOING；owner Codex Desktop · Runtime/Platform + GPUI。只领取平台目录/单实例/文件操作；P0-05 保持 ACCEPTANCE，不启动 P1-06/P1-07/P2。

2026-10-04 P1-06 READY → DOING：基线 f9adda4，保留全部 P1-05/Host Review 既有修改；Codex Desktop 只接正式托盘/窗口生命周期。未 commit/push、未启动 P1-07/P2。

2026-10-04 P1-06 ACCEPTANCE：正式 tray-icon 0.24.2 主线程长期持有、类型化 channel 直接唤醒 GPUI、关闭只隐藏与统一显式退出已实现。41 desktop（既有36+新增5）/294 core 与规定检查通过；真实关闭/native hidden、五次 secondary 同窗口 active/key/writer0、筛选保留、窗口 Quit 清理 PASS。原生工具 AX 未暴露托盘入口，SystemUIServer 超时、键盘入口无效果；等待用户展开菜单，不能计三轮托盘恢复/菜单图/托盘退出通过。READY 空，P1-07/P2 未启动，P0-05 仍 ACCEPTANCE；保留既有修改，未 commit/push。[完整证据](evidence/p1-06/README.md)。

2026-10-04 P1-06 User/Host manual acceptance 收口 DONE：用户看到 V 托盘图标并真实展开菜单，确认准确未接入状态与灰色禁用启停；连续三轮“关闭窗口 → 托盘‘显示窗口’恢复”均正常，最终托盘“退出”正常、无异常。Host 会话已查看用户两张截图，未复制入仓库，不伪造本地图片/SHA。Human Visual / Tray interaction 已补足，三项任务验收全部勾选；此前工具限制/NOT_RUN 和窗口 Quit 程序化清理保留，托盘退出无新增 process/socket/flock post-check。用户确认 Host 此前独立复核及 41 Desktop/294 Core 与指定检查均 exit 0，本次只做最小文档校验。68 卡重算：macOS DONE12 / ACCEPTANCE1 / READY1 / TODO47；Windows DEFERRED7，唯一 READY 为 P1-07，未领取/启动；P0-05 仍 ACCEPTANCE，P0-06/P0-08 仍 TODO。保留产品源码/AGENTS.md 等既有修改；无公网/sing-box/System Proxy/TUN 操作，无 commit/push。[人工证据](evidence/p1-06/host-manual-acceptance.json)。

2026-10-04 P1-07 本轮进行中：从干净 b982e8d 建立本地 React 16 张1280×720 reference，实施 Tokens/Theme、共享基础组件与 Shell/Panel 重组，工程入口隔离为 evidence-only；Desktop42/Core294 与要求的自动检查通过（初次clippy失败历史保留）。Host 本轮未看到托盘，native rect68×0且visible=true，根因未确定；真实保存/关闭隐藏/secondary writer0恢复及诊断重启值恢复有证据，但托盘恢复/显式退出组合未通过。字体分片/模糊差异与 hover/focus/error 等尚未完成的视觉状态分开记录。P1-07 保持 DOING、owner GPUI，其他状态不变，不启动P2、不commit/push。详见 [本轮证据](evidence/p1-07/README.md)。用户后续仅授权只读线上编辑入口核对，已恢复原页；其余 reference/API/WS 均本地fixture。

2026-10-04 P1-07 Host Review 修订：原Light Shell是System解析Dark的采集错误；新root显式Light/Dark/Core-load后1280×720四状态复核并保留历史。默认背景decode/resize/blur/encode移入blocking服务，Render只读ready cache，三条stale/dedup/职责回归；真实Blur drag后Select即时响应。同环境shell-wrapper baseline/current均不可见，真实Mach-O bundle二者真人可见且菜单正常；用户完成三轮Close→Tray Show、Tray Quit、重开值恢复Light/6500ms/IPv6on、再次Tray Quit，native root socket自动清除/锁释放/无进程。后续Button焦点/Tooltip定位修复和适用组件状态独立补验；45 Desktop/294 Core及完整检查PASS，最后Tooltip影响范围另记录。字体/局部backdrop blur和尚未完成视觉细节继续明确记录，整卡保持DOING、owner GPUI，不声称95%/DONE；其他Task不变，未启动P2/业务页/内核/公网，未commit/push。见[evidence](evidence/p1-07/README.md)。

2026-10-05 P1-07最终视觉续录（DOING/GPUI）：627728a2统一1280×720显式Light/Dark新采图及Tooltip箭头/spinner修正已复核，45 Desktop/294 Core与完整检查PASS；用户完成至少三轮真实Tray Show和Tray Quit，重启Settings值恢复Light/6600ms/IPv6on。末补查IconPicker展开态发现可修tab均分/局部focus，已最小源码修正并重跑11项自动验证PASS，新build8bd7a366尚待GUI采集与最终组合；旧627728a2帧/操作完整归档，不能冒称最后源码视觉PASS。保留旧重启窗口PID77189等待用户已请求的Tray Quit，不以keyboard/SIGTERM替代该操作；之后换新实际Mach-O bundle继续。整卡DOING、owner GPUI，其他Task不变；无commit/push/P2/公网/sing-box/System Proxy/TUN。[完整当前状态](evidence/p1-07/README.md)。

2026-10-05 最后构建8bd7a366视觉复核续录：Light/Dark显式1280×720 Shell展开/折叠、Panel及当前组件状态已重采；Tooltip箭头/定位、spinner、IconPicker分类均分/局部focus/trigger已对照React级联和叠图复核，技术差异单列字体fallback/element backdrop blur/UA栅格。最后源码完整11项检查PASS（45 Desktop/294 Core），无新源码改动。实际bundle保存Light/6600ms/IPv6on，用户回报完成三轮与退出；日志仅Show2/Close3，明确不计3次日志PASS。Tray Quit后进程/socket清理、flock可重新获得，重启实际Settings值恢复PASS；已重做视觉/行为保存并隐藏，等待真人明确3Show→Tray Quit补齐。P1-07保持DOING、owner GPUI，其他Task/DAG不变；无commit/push/P2/公网/内核。见[evidence](evidence/p1-07/README.md)。

2026-10-05 P1-07 最终 Host 验收收口 DONE（仅文档/evidence）：同 8bd7a366 build 最终视觉/AX/comparison 与人工查看齐备，无新的可修视觉 blocker，MiSans/NotoEmoji fallback、element-level Card/Modal backdrop blur、少量 UA 栅格差异继续为 TECHNICAL_DIFFERENCE；Tokens/Theme → 基础组件 → NavigationItem/CompactSetting/Section → 页面保留。明确人工 Show #1 → Close → Show #2 → Close → Show #3 → Close → Tray Quit 及 Host post-check PASS，旧 first-launch Show2/Close3 日志不改写。相同 Mach-O bundle/root 重启 PID94362，Light/6600ms/IPv6on、snapshot_loaded、单 primary/writer、state bytes 不变 PASS；用户“最终退出完成”后 Tray Quit，Host PID gone/socket absent/flock free PASS，瞬态 PID96193 不计残留。Host 首跑 Desktop44/1 FAIL 与后续5/5、45/45、45/45 PASS 分别保留，根因未证明；Core294及指定检查 PASS 不覆盖旧日志。六项验收勾选、owner 释放并移出 Active Tasks。68 卡 DONE13 / ACCEPTANCE1 / READY0 / DOING0 / TODO47 / Windows DEFERRED7，无环、无新 READY；P0-05/P0-06/P0-08 与后续业务页状态不变。未修改源码/Cargo/assets/测试/AGENTS.md/长期规范，未删 Legacy 或 Host 复核文件，未 commit/push，未启动 P2/公网/sing-box/System Proxy/TUN。见[最终交付](P1-core-and-shell.md#p1-07-final-closeout)及[文档验证](evidence/p1-07/final-20261005/closeout-validation.json)。
2026-10-05 P1-07 Host 提交：最终实现、共享组件、Heroicons 资产、完整视觉/交互 Evidence 与 DONE 状态由 Host 提交为 `25eb020c349fdc8f716bd5a991d37b1eb407aae7`；其后仅提交本交接页元数据，不改变 P1-07 实现或验收身份。未 push，P2 未启动。

2026-10-06 12:35 起仅 P0-05 owner 修复后批准复验：Host真实看到标准UI并批准，正常构建776d…无额外签名。helper/kernel uid0/gid80、0755、不可写/hash正确，runtime0700、固定plist参数与业务UID/GID登记正确；launchd PID26627 running/execs1、root harness取得socket0600与完整Proxies snapshot。daily cycle exit1/errno22，聚合stdout空不能定位operation或断言无Start/写入；专门owner/安全拒绝未进入。当前固定path日志CT错误仍出现，另有exec allowed/legacy daemon，无PrivilegedHelperTool/-423观察。固定harness卸载和独立cleanup PASS，主用网络三hash unchanged，14 Rust/2 Python/全部指定自动验证PASS。一次性observer漏识别osascript的false/null已新增errata，raw保留。本轮无源码/DAG修改，状态/owner不变，READY=[]、未启动下游、未commit/push。见[本轮证据](evidence/p0-05/desktop-ownerfix-approved-20261006-123513/README.md)。

2026-10-06 12:59起P0-05分阶段诊断真实复验：当前正常构建ce84558e…仅一次标准UI真实批准。NDJSON唯一step为cycle1/step1/Status/client.read/client_error，明确read_frame:358 set_read_timeout(Some(remaining)) errno22，前序成功空、Start未请求。root安装/daemon/socket仍实际建立；无root业务child/recovery/managed写入，专门owner/rejection/conflict未进入。固定uninstall失败preserve resources；RecoveryRequired，ROOT/plist/注册label/disabled隔离Service仍在，daemon exited0/socket/recovery absent；fresh独立SC API回读完整Proxies==snapshot且非Set/主用v4v6，主用网络三hash不变。23 Rust/2 Python及所有指定offline验证PASS。只定位，不修改源码/权限/签名/路径或DAG，未启动下游、commit/push。见[新证据](evidence/p0-05/desktop-errno22-localized-20261006-125933/README.md)与[后续卸载指南](evidence/p0-05/desktop-errno22-localized-20261006-125933/RECOVERY.md)。

2026-10-06 13:20 两阶段 P0-05 复验：先经旧SHA helper标准UI卸载并独立清理PASS，再正常构建read_frame修复版完成真实批准。daemon/socket与首步Status PASS，第二步StartSystemProxyTest.child_readiness timeout；没有recovery/network_write，完整A–E未满足。最终独立cleanup/hash unchanged，Rust30/Python2及八条命令PASS；独立Review另列限制。保留ACCEPTANCE/owner，READY=[]，未改源码、未启动下游、未commit/push。[本轮证据](evidence/p0-05/desktop-readframe-fix-20261006-132202/README.md)。

2026-10-06 13:41 当前readiness版本真实复验：一次标准UI批准、root安装/daemon/socket/Status PASS。cycle1 step2 readiness=child_exited，child PID/PGID52916，业务real/effective UID/GID正确，731ms exit1/raw256；stderr81bytes EOF，明确 `read config at /dev/fd/3: open /dev/fd/3: permission denied`。未到recovery/managed代理写入，固定卸载与独立cleanup/hash unchanged PASS；40 Rust/2 Python及八条指定命令PASS。完整C失败、A–E未全部满足，保留ACCEPTANCE/owner/READY=[]，不改源码/依赖、不启动下游、未commit/push。[本轮诊断与独立Review](evidence/p0-05/desktop-readiness-diagnostics-20261006-134101/README.md)。

2026-10-06 13:59 FD3 anonymous pipe当前版本一次标准批准复验：root安装/daemon/socket/Status PASS，anonymouspipe425bytes/root0600、两轮child身份/readyports/auth PASS，8 daily steps全部ok/two_cycle_exact_restore。CLI prototype owner transientdisconnect保持、SIGKILL/NOTE_EXIT/restore/reaped/group/listener及externalPAC conflict/harnessfinalsnapshot PASS；固定卸载与独立allabsent/main3hash unchanged PASS。Rust47/Python2及八命令PASS；E真实仅pid/path/RunShell/>4096加nobody/rootcaller，executable/service/真实root总deadline仍NOT_RUN，原卡GUIowner未冒充。保持ACCEPTANCE/owner/READY=[]，不改源码/签名/权限路径、不启动下游、未commit/push。[本轮完整证据与独立Review](evidence/p0-05/desktop-fd3-pipe-20261006-135940/README.md)。

2026-10-06 P0-06 READY→DOING→DONE：仅领取应用自身出站原型，复用subscription/fetch，feature中typed Direct/ViaRunningProxy、固定IP TLS DoH、跨源凭据清除、无fallback、无订阅自举完成。保护现有external TUN，仅只读/自有loopback mixed及自身公网请求；SystemProxy ON未试验、TUN OFF/managed bypass未试验，原卡4允许明确范围决定并保留INCOMPLETE/UNSUPPORTED。全部适用验证、独立review、资源cleanup PASS；P005源码/card/evidence SHA保持不变。68卡重算DONE15/ACCEPTANCE0/READY2/TODO44/DEFERRED7，READY为P0-08/P2-01；owner释放，仅更新READY不启动、不commit/push。完整证据见[evidence](evidence/p0-06/20261006-144632/README.md)。

2026-10-06 P2-01 最终真实GUI收口：隔离root+loopback fixture+正式arm64/minos15 .app完成Preview/Save/Edit/Refresh/Delete、partial、无持久化、stable NodeId、失败保旧、ReferenceConflict、stale generation、busy/error/retry、Host中文IME/focus/Escape、Light/Dark五态、三语与重启；既有字体/blur技术差异保留。仅本页有界修复；Core304/Desktop68/outbound9及check/clippy/fmt/diff、独立Review、最终cleanup PASS。P2-01 DONE/owner释放，68卡DONE16/ACCEPTANCE0/READY3/TODO42/DEFERRED7；READY为P0-08/P2-02A/P5-06，未启动、未commit/push，P0-05/P0-06历史不变。[完整验收](evidence/p2-01/gui-final-20261006-180816/README.md)。

DAG计数说明：按P5-06原卡与总表既有显式依赖，P2-01完成后P5-06也READY；因此完整68-card实际为READY3/TODO42，与预期READY2/TODO43不同。未增加/修改依赖或启动P5-06。

2026-10-06 Host Finding重开P2-01：原功能PASS继续有效，上一轮gui-final evidence原样保留；订阅UI未复刻Finding需重新按React/CSS审计和验证，旧视觉PASS不再最终有效。P2-01 ACCEPTANCE/visual owner=Codex Desktop · GPUI visual parity；P2-02A/P5-06 TODO，READY仅P0-08，不启动下游。

2026-10-06 P2-01视觉Finding最终关闭：build23拒绝保留；build29补输入说明、DNS11px/32px输入与原版hint、默认折叠、去Preview区域、自然高度/溢出滚动；Host“符合”。合并tabs及一次SaveDraft复用Core原合同，Busy输入/关闭锁定Finding修复。最终浅深色/真实GUI/三语/同root12条目重启bytes一致/cleanup、Core304/Desktop69/outbound9与check/clippy/fmt/diff、Independent Review PASS。ACCEPTANCE→DONE/owner释放；完整68卡DONE16/ACCEPTANCE0/READY3/TODO42/DEFERRED7，无环；READY=P0-08/P2-02A/P5-06，未启动。P0-05/P0-06历史与无关dirty未修改，未commit/push。[最终视觉交付](evidence/p2-01/visual-parity-20261006-190927/README.md)。


2026-10-06 P2-01 FULL-PAGE-001完整页面修复：Share/五actions/health dots/真实只读节点grid与完整Source/Rules/Nodes、DNS/periodic/Rules未来能力禁用壳；最终build16 Core304/Desktop70/outbound9及check/clippy/fmt/diff PASS，源码独立Review PASS。1280×720当前真实缩放浅深色完整页面/编辑器/Loading/Error/Busy已采集；仅待Host明确完整页面确认，保持ACCEPTANCE/owner，READY仅P0-08。不启动下游、无commit/push；[本轮矩阵与报告](evidence/p2-01/full-page-20261006-214140/README.md)。


2026-10-06 22:43 Host完整页面复核：回复“仍有偏差”，Finding P2-01-VISUAL-FULL-PAGE-001继续OPEN，P2-01继续ACCEPTANCE，owner保留。当前工程/自动/Independent Review PASS不代替Host视觉确认；正在请求具体偏差位置，保持隔离现场。DAG不变：DONE15/ACCEPTANCE1/READY1/TODO44/DEFERRED7，仅P0-08 READY。


2026-10-06 P2-01 Host具体偏差：暗色节点卡片亮色刺眼。当前build17按用户本地原版实际DOM修复，仅本页node_grid颜色/对应tokens，Light不变；Core304/Desktop70/outbound9及指定九命令PASS。主列表/Editor真实浅深色重采；[build17实测和修复](evidence/p2-01/full-page-20261006-214140/DARK-NODE-REWORK-BUILD17.md)。仍ACCEPTANCE/Finding OPEN，等待当前Host确认，未启动下游/commit/push。

2026-10-07 P2-01最终收口：Host明确确认build38-final完整订阅页面“符合”，executable SHA256 `de220c173fb4cd70163419e93a54d34324b4dbf53617a87e46b32be8fafcbff3`。ACCEPTANCE → DONE、Finding OPEN → CLOSED，owner Codex Desktop · Full Subscription UI Parity释放；历史FAIL/REWORK/局部批准/PENDING/build保留。68卡DONE16/READY3/TODO42/DEFERRED7，其余状态0；READY P0-08/P2-02A/P5-06均未领取/启动。cleanup NOT_RUN（ps被沙箱拒绝），app/fixture及隔离状态保留。文档/DAG/工作树保护与diff检查见[最终证据](evidence/p2-01/final-closeout-20261007-122027/README.md)。

2026-10-07 P2-02B READY→DOING→DONE：仅Core/Config owner，复用既有Compiler/Catalog；产品loopback mixed/controller/cache、Profile选项与独立health计划/兼容schema9完成。14定向/327 Core及指定检查、四个locked-v1.14.0真实check PASS；初次FAIL/跨pool DNS detour自查修正记录保留。释放owner，DAG18DONE/4READY/39TODO/7DEFERRED，无环依赖不变；READY=P0-08/P2-03/P4-02/P5-06均未领取/启动。无run/Runtime child/SystemProxy/TUN/helper/UI/Git写操作；仅本轮私有候选/内核临时资源清理，历史cleanup NOT_RUN不改写。见[P2-02B交付](P2-local-proxy.md#p2-02b-closeout)。

2026-10-07 P2-02B Host独立review最终收口：Finding FIXED_PENDING_HOST_REVIEW→CLOSED、Task ACCEPTANCE→DONE、Codex Core/Config owner释放。原依赖逐卡核对/无环/68卡计数一致；P2-03/P4-02仅READY。仅文档/evidence，不改产品源码，不重跑Cargo/候选/真实child/UI/网络。[Host收口记录](evidence/p2-02b/host-closeout-20261007-132657/README.md)。


2026-10-07 P2-03最终Host收口（仅文档/evidence）：Host确认Basic2 `59280c3146502e1b9474d08f976d807cfae4390751493badf9f89aeca9b325c4` 功能与视觉“符合”；BASIC-SETTINGS-004 FIX_PENDING_HOST→CLOSED，FULL-PAGE-002/FIDELITY-003 OPEN→CLOSED，P2-03 ACCEPTANCE→DONE，Runtime/Platform + GPUI及Runtime公共契约owner/本卡预约释放。完整68卡DONE19/READY7/TODO35/DEFERRED7，其余0；新增READY P2-04/P2-05/P3-01/P5-04，全部未领取/启动。BUSY-001与历史失败/REWORK/候选保留，当前app/runtime资源不操作；仅本轮文档校验，不重跑产品测试/GUI/内核/网络，不执行禁止Git操作。[最终证据](evidence/p2-03/final-closeout-20261007-164453/README.md)。

2026-10-07 P2-04 P1 Finding修复：原DONE撤销；业务Manual SelectionPolicy原子保存pending，begin→controller/read-back→confirm两阶段CAS，selection revision +2、config/applied不变；不确定/确认保存失败保留pending，重建owner/Stopped仍可见。ApplySaved/RestoreLastSuccessful先GET actual，等于pending确认、等于old/default清除，其余不PUT/不Ready；解决后的版本同步active/manifest confirmed。Core361/Desktop85及指定检查PASS，3 Native ignored、真实中断/重启 NOT_RUN；新initial FAIL/最终PASS分开保留，旧evidence未写入。P2-04 ACCEPTANCE、P2-04-PENDING-PERSISTENCE-001 FIXED_PENDING_HOST_REVIEW、Runtime/Platform owner保留，DAG DONE19/ACCEPTANCE1/READY6/TODO35/DEFERRED7；P2-06 TODO，无下游启动。见[修复契约](P2-local-proxy.md#p2-04-pending-persistence-fix)与[fix evidence](evidence/p2-04/pending-persistence-fix/README.md)。

2026-10-08 P2-04同一P1 Finding第二边界：旧plan没有当前pending pool时，candidate静态检查先于prepare/旧writer停止，显式ReconcileSelection同样拒绝；循环期间剩余pending由最新业务快照最终gate阻止Ready。新增5自动测试保护v11/v12 manifest失败可达场景、已有child不动、多个pending部分覆盖不CAS、完整ApplySaved确认、Restart裁剪pool、prepare/read期间变化与已有CAS保留。Core366/Desktop85、check/clippy/fmt/diff PASS；初始2失败及版本断言365 PASS/1 FAIL日志保留。状态/owner/依赖不变，Native/桌面NOT_RUN。
