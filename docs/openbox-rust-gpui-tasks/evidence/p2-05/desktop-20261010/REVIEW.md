# OBG-P2-05 独立 Review · 2026-10-10

Reviewer：独立子 Agent `review_p205`，使用 `code-delivery-review`。仅只读源码、diff、任务文档和原始收据；没有执行 Native/GUI、运行 child、修改源码或 Git。唯一写入为本文件。

## 当前结论

**最终源码审查：0 个可信可操作 Finding。本候选 Native、已执行的局部 GPUI 与最终清理收据复核：PASS。P2-05 保持 ACCEPTANCE，旧完整组合等待决定由 Host 收口，不改变 Task 状态。**

范围为指定 `dev/p2-05-main-integration`，HEAD/main 基线 `bda8be5c469dcbd6093a28fd4fd549be70c22194`；包含所有 tracked diff、未跟踪 `application/network.rs` / `network/tests.rs` 及六份任务文档。`source-before.json` 对 24 份文件绑定 SHA-256，并与既有 main-integration 最终清单一致。结构查询明确使用本工作树的 CodeGraph `projectPath`，不借用主树索引。

## 源码核对

- `ManualRuntime::mark_ready` 在 check/run、鉴权和选择核对完成后发布完整实例 ID、实际 loopback mixed 与实际 applied plan 的 NodeId/tag 映射。订阅及 `NetworkService` 只消费 `ProxySource`，不存在历史端口构造或候选实例启动。
- 预检失败保留当前有效能力；停止旧 writer 前、Stop、失活/清理失败、Drop 撤销。`RunningProxy::while_valid` 的 watch 取消和迟到结果检查保护旧客户端；RwLock 保护发布/撤销。Group 选择复用同一实例能力；Restart 撤销旧能力。P4-03 选择拒绝未应用配置的新规则保留。
- 订阅 Preview/Save/Refresh 继续生产 Store/CAS 写路径；下载、解析、写入失败保旧。正式出站没有静默换路或系统代理 fallback。订阅 Source 的 URL 用户凭据被拒绝；跨源认证头/条件验证器剥离；HTTPS 降级拒绝；TLS 校验不可被旧配置关闭；错误分类不转发含秘密 URL 的 reqwest Display。
- 三 provider 使用独立无订阅凭据的请求；未知字段保留 None。节点测速只用当前实例的认证 Controller GET 与 applied tag，编码路径/目标、禁止 redirect，不写 selector。`NetworkService` shutdown 和订阅 closing 在真正 app Quit 先触发，再等待 Runtime 清理；隐藏窗口不关闭服务。
- 保留单一 ObservationService/reader 与原 failover runner；本次未修改 Helper/IPC、Runtime 公共 DTO、持久 owner-transfer/fence 格式或 Cargo workspace/lock。Legacy ObservationOnly Runtime 没有能力时准确拒绝 ManagedCore/System。

## 自动验证收据复核

独立解析九组 main-integration 原始 log 的 `test ... ok`，去重为 **225 PASS：Core 212 + Desktop 13**。数量 72/46/4/46/16/8/20/7/6；Desktop Runtime 四个 ignored 不计 PASS，早轮 Runtime 不重复累计。旧 183 中 182 同名；另一项由输入 main 替换为 `selection_rejects_unapplied_config_without_controller_write`，当前 PASS，没有删弱断言。`application::failover::` 的 0 测试/exit0 不计 PASS；实际 `domain::groups::tests::` 为 20 PASS。

三包 all-targets Clippy/check/build、最终 Core Clippy、examples check、workspace/src-tauri fmt 和 diff 收据 exit0。Legacy Clippy **FAIL101** 原日志是 Windows `libcronet.dll` 缺失，未绕过资源配置。上述是收据复核，不声称 Reviewer 独立复跑。

## 本候选 Native 收据复核

`native-production.json` 为用户唯一指定 exact ignored 测试，600 秒外层 runner，**exit0 / 1 PASS / 23.027 秒**。`kernel.json` 固定 1.14.0、darwin/arm64、SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`。复核新日志而非旧 1 PASS：

- 生产 Preview/Save 和 HTTP503 刷新失败保旧磁盘字节；初始无有效代理准确不可用。
- Ready 当前完整 identity 与 `127.0.0.1:63991` mixed 一致。三个 provider exit/geo 请求成功，8.8.8.8 对应 ASN15169；不同公网出口是当前外部 TUN 分流结果，不推断物理 Direct 或相同公网出口。
- 固定 `https://www.gstatic.com/generate_204` site 708ms；正式 NodeId=a、实际 candidate config 的 node-a 和同一认证 Controller，806ms。
- accept 实际继承 nonblocking，显式恢复 blocking；TLS 双向 1707/4083 bytes，未更换目标或降低 TLS。
- Replace 新 PID/instance，旧能力无效；Quit 后第二能力撤销、source None，两代 mixed/controller 拒绝连接、隔离 root 移除，关闭后请求 Closed。

本测试只证明 node→自有 HTTP fixture→当前 mixed→既有外部 TUN 的受控拓扑；物理 Direct 返回 `DirectPathUnverified`，原 UNVERIFIED/SystemProxy ON NOT_RUN 保留。原 HTTP503 与旧 Legacy FAIL 不改写。正式 SystemProxy/TUN 组合分别归 P2-07/P2-09/P6-01/P6-05；不新增反向依赖或启动下游。

## 最终源码与本轮自动复跑

再次只读核对 `source-review-final.json` 的 18 个源码/配置 SHA 与实际文件，全数相同；本轮没有源码修复或候选漂移。四项 NetworkService、46 项 ManualRuntime 定向复跑的实际 log 为 4/46 PASS、exit0；Desktop build、all-targets Clippy、workspace fmt 收据均 exit0。这 50 项与既有 225 集合重叠，不增加独立覆盖数量。Native 两个 child PID42989/43041、测试进程和四端口的退出/拒绝连接、无 root 残留由 `native-cleanup.json` 补证。

## 真实 GPUI 证据复核

Reviewer 独立读取并实际查看本轮关键 PNG（03/04/07/08/09/10/12/13/14/15/18/19/22/23/24/25），结合真实操作、磁盘摘要、进程、运行日志复核，没有亲自操作 GUI。验证边界如下：

- `gui-build-final.json` 绑定本树构建源二进制 SHA `2b9321af9337f64f90f2b09e7b36b994bb8071ee40c9c0422d47609e07d8475a`，最终测试 bundle 主程序 SHA `7429f20ad0d6b8c7f1dd8d4c4b92f917757b4a16c84f91ed919658535b5b0650`，固定 kernel SHA 正确，Info/LSEnvironment 指向自有短 root，codesign verify exit0。不同签名快照保留，不冒充原始未改字节。
- 03 空页、04 添加下载 busy、05 实际保存、06 刷新后节点名更新、07 HTTP503 准确失败。`refresh-failure.json` 旧 state SHA 完全不变。08 保存错误保留「P205 保存失败草稿」与原字节；故障只施加在自有文件，恢复后真实重试成功。
- 09 中文→English 后未提交面板 latency 数字 6789 保留；此项只证明该字段草稿，不声称订阅弹窗跨语言或数字重启持久化。13 重启恢复 English/Light，14 原初始 Busy 错误保留，23 稳定订阅/两节点回读，没有错误/Reload 控件。
- 10 主备 lane 编辑及保存、重启恢复；Ready 的正式 Controller 选择 ManualPin→Auto，由 `gui-pin-final.json` / `gui-auto-final.json` 实际 group_selections 证明 mode=manual_pin→auto、pending=null、selection_revision6→8；config_revision6及同实例配置 SHA `5906e680e1629e99770f20305a293cf118c881a2b2cec58ebc45167896e12c30` 不变。24/25 稳定页面从 Pinned lane unavailable 转到 All lanes unavailable，符合 fixture 没有节点服务的事实，不称健康故障转移连通 PASS。
- 15 运行态页面存在本次自有 mixed→loopback fixture 的三条真实日志；16过滤/暂停操作有截图。P3 指标 `—` 按未知记录，没有伪造全指标通过。
- 同 PID46045 的 Close→Hide、Show→RevealExisting、native hidden=false、22 窗口截图与用户确认相互一致。最终托盘 Quit 的日志、用户确认和 `gui-final-quit.json` 显示 App/child 退出，控制 socket 消失，端口关闭。

本轮用户明确确认按当前 macOS 默认显示配置验收；1512×982/Retina scale2/内容1280×720/Light 为实际配置，没有修改显示设置，不声称 independently verified 150% 档位。其它主题/缩放未由本轮证明。

证据纠错边界保留完整：最初工作树长路径超过 Unix sun_path 的失败未改写；仅缩短自有数据目录。应用退出后自有 fixture 激活订阅/Direct 默认仅为 Runtime 验证准备，不算 P2-08 UI 激活功能。第二次“恢复”实际是 Quit，旧 PID45244/45317 消失，自动化重新打开46045；21截图和 `tray-restore-identity-mismatch.json` 不计同实例恢复，之后22补足。旧 `gui-pin-state.json` 错误字段 null 不作模式证明；18/19中间状态保留，最终使用正确字段的 final 收据和24/25。

## 最终系统保护及清理

`network-before/after.json`、`network-protection-final.json` 的完整路由字节、默认路由、DNS、proxy、utun、preferences摘要相同；Karing1267、扩展1356、既有 Clash helper822 的进程开始时间与命令不变。没有 Veyra 管理员 helper 运行证据，也没有管理员安装或全局网络修改。

`gui-final-quit.json` 中43302/45244/45317/46045/46983全消失；新旧50711/50712/65494/65495拒绝连接、control socket移除、短 root 和初始失败长 root 均清理。自有 HTTP fixture43410/64413退出；最终 state/摘要保留在本轮证据，App 包保留。进程退出支撑其线程释放，不将普通进程列表冒称逐线程内部诊断。

本轮证据可以支撑 P2-05 当前客户端功能和上述局部 Native/GPUI 结果。正式 SystemProxy/TUN出口、物理 Direct、完整组合、macOS15兼容及未执行UI状态不从本轮推导为已通过。P2-05 ACCEPTANCE、P2-06 DOING/单一 owner及原依赖均不改变，不 commit/merge/push，不启动下游。

最终文档准确性复核指出 SESSION Active Tasks 仍有本候选 Native/GUI NOT_RUN 的旧描述，主 Agent 已修正当前行；历史原 NOT_RUN 段保留，问题 CLOSED。`final-validation.json` 核对18源码无变、主/旧树文件保护摘要一致、68张P/W卡DAG无环、DONE30/ACCEPTANCE1/DOING1/TODO29/DEFERRED7、computed_READY=[]、引用存在、diff check exit0；最初校验漏W卡的脚本计数错误也保留说明，修正匹配后通过，不是任务状态变化。Reviewer 最终独立 `git diff --check` exit0。
