# P2-05 最终实机验收（2026-10-10）

**本卡客户端功能及本轮局部 Native / GPUI 检查通过；状态保持 ACCEPTANCE，等待 Host 收口决定。** 本轮独立 Review 未发现可信源码缺陷，未修改源码；不提交、合并、推送或启动下游。此前用户等待完整组合验收的决定未被自行取消。系统代理真实出口、Veyra TUN 正式组合及最低系统版本兼容仍属于后续分期验证，不能从本轮局部结果推导为通过。

## 候选与证据身份

- 唯一开发树 `.worktrees/p2-05-main-integration`，分支 `dev/p2-05-main-integration`，基线及最终 HEAD `bda8be5c469dcbd6093a28fd4fd549be70c22194`。Serena Execution `execution-23397-1791600666634124-239` 已完成，无实现 Agent 并发写入。
- 读取当前 AGENTS、DEVELOPMENT_WORKFLOW、SESSION、总表及整合验收；按授权初始化本树 CodeGraph。18 个变更源码/配置文件（包含两个未跟踪源码）绑定前后 SHA，一并交独立 Reviewer 复核；旧树及主树既有文件只读保留。
- 本轮真实收据独立保存在 [desktop-20261010](evidence/p2-05/desktop-20261010/README.md)，不覆盖旧证据。工程整合的 225 个唯一定向 PASS 及原始失败见 [整合记录](P2-05-integration-acceptance.md)。本轮 50 项定向复跑与旧集合有重叠，不重复累计为新覆盖。
- 本树实际 `cargo build --offline --locked -p veyra-desktop` 成功。测试包 `target/p205-desktop/VeyraP205Acceptance.app`；原构建二进制 SHA256 `2b9321af9337f64f90f2b09e7b36b994bb8071ee40c9c0422d47609e07d8475a`，最终 ad-hoc 包内主程序 SHA256 `7429f20ad0d6b8c7f1dd8d4c4b92f917757b4a16c84f91ed919658535b5b0650`。最终 Info.plist / `codesign --verify --deep --strict` 退出 0 见 `gui-build-final.json`；旧 `gui-build.json` 保留重新签名之前的摘要，不代替最终身份。
- 固定内核 sing-box 1.14.0 darwin-arm64，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，执行前核验。Host 为 Apple M4 Pro / macOS 27.0.1。该内核及测试包最低系统版本为 26.0；设置构建变量 15.0 不证明能在 macOS 15 运行。

## 实际命令结果

所有长命令复用既有 runner，外层限时 600 秒；JSON 保存完整 argv、实际退出码与耗时，日志记录真实测试数量。

| 本轮执行 | 结果 | 退出码 / 收据 |
| --- | --- | --- |
| 唯一指定 ignored Native test | 1 PASS，136 filtered，测试 22.50s / 外层 23.027s | 0，`native-production.json/.log` |
| Core `application::network::` | 4 PASS；指定路径、目标、归一化及 shutdown 取消 | 0，`core-network-review.json/.log` |
| Core `application::manual_runtime::` | 46 PASS；发布/撤销、applied NodeId、P4-03 选择及 Observation 共存 | 0，`core-runtime-review.json/.log` |
| Desktop offline/locked build | PASS | 0，`desktop-build.json/.log` |
| Desktop all-targets Clippy `-D warnings` | PASS | 0，`desktop-clippy.json/.log` |
| workspace fmt check | PASS | 0，`workspace-fmt.json/.log` |

Native 使用校验后的绝对内核路径，实际执行：

```sh
env MACOSX_DEPLOYMENT_TARGET=15.0 \
VEYRA_SING_BOX_PATH=/Users/lifeilin/.codex/worktrees/p2-05-outbound-clients/veyra/docs/openbox-rust-gpui-tasks/evidence/p2-05/production-clients/veyra-sing-box \
cargo test --offline --locked -p veyra-desktop \
runtime_service::quit_tests::outbound_clients_real_production_services \
-- --ignored --exact --nocapture --test-threads=1
```

## Native 用户行为与边界

正式 SubscriptionManagement Preview / Save 成功，HTTP503 刷新失败时旧磁盘字节保留。当前受管实例 Ready，实际鉴权 controller/mixed 属于本次 child；不是历史端口扫描或额外下载 Runtime。

IpSb、IpWhoIs、IpApiIs 的真实出口查询及指定 `8.8.8.8` 查询均成功归一化；出口 IP 不一致保留原日志，不声称绕过外部 TUN。指定 HTTPS `https://www.gstatic.com/generate_204` 测速成功（708ms）。正式 `NodeId=a` 经当前 Controller 的实际 applied `node-a` 测速成功（806ms），accepted socket 从继承 nonblocking 改为 blocking 后实际断言通过；TLS record type 22，双向传输 1707 / 4083 bytes，均大于 0。

Node 路径为自有 HTTP 节点 → 同一当前 mixed 的 Direct 出口 → 用户已有外部 TUN；验证 Veyra 正式 NodeId、Controller 与 TLS 连通，不能冒充独立远端代理或物理 Direct。Replace 后旧能力失效，新实例发布；Quit 后 NetworkService Closed，两个 child、测试进程、四个端口和临时根均清理，见 `native-cleanup.json`。没有执行其它 ignored Native test。

## 真实 GPUI 操作

本轮保留当前 macOS 默认显示配置：用户明确回复“按当前 macOS 默认显示配置验收”。只读显示设置截图 20 记录默认 1512×982、面板 3024×1964；GPUI 报告 scale factor 2，内容 1280×720。本轮浅色；没有将该配置称为 macOS 的“150%”，也没有修改系统显示设置。深色及其它缩放不从本轮推导。

| 实际操作 | 结果 / 原图及收据 |
| --- | --- |
| 订阅空状态 → 添加 | 03 空页；自有 loopback `127.0.0.1:64413` 真实下载，04 busy 禁用反馈，05 保存一订阅两节点。05 弹窗仍在，未声称自动关闭。 |
| 成功刷新 / HTTP503 | 06 节点名更新、稳定 NodeId 保留；07 错误反馈，`refresh-failure.json` 完整旧 state 字节及 SHA 相同。 |
| 保存失败 / 重试 | 自有 state 文件故障注入；08 草稿名仍在、错误可见，旧缓存及备份字节未改；恢复自有文件后同一草稿重试成功，备份故障已撤除。 |
| 语言切换保持草稿 | 09 Panel Settings 未提交 latency timeout=6789 在中文→English 后保持；English 持久恢复。未提交数字不是保存结果，不宣称订阅弹窗跨语言或该数字重启持久化。 |
| 重启回读 | 用户托盘退出 PID43302 后重启同根；13 恢复 English/Light，14 初次 Busy 与旧数据保留后 Reload；23 稳定回读订阅和两节点，无 Reload/错误。 |
| 主备组既有功能 | 10 实际编辑 primary / Backup1、11 保存，重启恢复；Ready 下 Controller 确认 ManualPin→Auto，24/25 稳定截图，`gui-pin-final.json` / `gui-auto-final.json` mode 正确、pending=null、selection_revision 6→8、config_revision=6及本实例 config SHA 不变。节点 1080/1081 无服务，All lanes unavailable 是真实反馈，不称健康故障转移连通 PASS。 |
| P3 日志共存 | 自有 mixed 访问自有 HTTP fixture，15 实际三条日志；16 搜索过滤与暂停/恢复。观测指标未知显示 `—`，不称所有 P3 指标均已验证。 |
| 隐藏 / 同实例托盘恢复 | 关闭按钮 Hide；最终 PID46045 从托盘 Show→RevealExisting，native hidden=false、同 PID 存活；用户确认窗口恢复，22 原图及 `gui-tray-restored-final.json`。 |
| 正常托盘退出 | 用户最终 Quit 后 PID46045及自有child46983退出；本轮其它测试PID也不存在。50711/50712、旧65494/65495关闭，control socket 移除，锁/网络/观测线程随自有进程退出，根目录及自有HTTP fixture清理；`gui-final-quit.json` / `network-protection-final.json`。 |

隔离根最初在工作树中导致 Unix socket 路径超过 sun_path 104 字节，应用尚未初始化失败；原失败日志和 `gui-launch-first-fail.json` 保留。测试根改为自有 `/private/tmp/veyra-p205-gui-20261010`，未改产品代码。本轮为验证运行态，在应用退出后仅将自有 fixture 的 `active_subscription_id` 指向已保存订阅、`default_target` 设为 Direct；`runtime-gui-fixture-preparation.json` 保存前后值，不能计为产品 UI 启用订阅验收（该 UI 属 P2-08）。

第二轮用户回复恢复时，实际日志为 Quit，旧 PID45244/45317 已退出；自动化 screenshot 方法重新启动 PID46045。21 原图及 `tray-restore-identity-mismatch.json` 明确保留为新启动，不能当作同 PID 恢复。此后重新隐藏并取得真实 Show 与同 PID 的 22 证据。18/19 旧截图与字段误取的 `gui-pin-state.json` 保留，最终模式语义只使用正确字段的 final 收据和24/25。

## 系统保护、独立 Review 与收口

`network-before.json` / `network-after.json` 实际完整路由、DNS、代理、utun4、SystemConfiguration preferences 摘要相同；IPv4/IPv6 默认路由仍为外部 utun4，DNS仍为10.20.0.2，HTTP/HTTPS/SOCKS/PAC仍关闭。Karing PID1267、系统扩展1356及既有 Clash Verge helper822 的开始时间/命令一致。未启动或安装 Veyra 管理员 helper，未使用 sudo/launchctl 修改服务，未操作其它实例。

用户确认正常托盘退出后，测试应用/child 全部不存在，自有 server43410及64413也停止。最终自有 state 和文件摘要复制入本轮证据后删除短根及失败的长根；App 包保留，未清理用户资料或旧证据。

独立 Reviewer 核对最终源码（包含未跟踪文件）、发布/撤销时序、applied NodeId、认证/重定向、失败保旧、P4-03/P3 与 P2-06 边界，以及 Native/GUI、身份误判、签名与清理证据，结论见 [REVIEW](evidence/p2-05/desktop-20261010/REVIEW.md)。Reviewer 复核收据，不冒称本人操作 GUI。

历史 HTTP503 Native FAIL、Legacy Windows libcronet.dll Clippy FAIL101 和全部旧证据原样保留。物理 Direct **UNVERIFIED**、SystemProxy ON **NOT_RUN**；正式 SystemProxy 实施/真实出口与完整网络组合归 P2-07/P2-09，Veyra TUN 归 P6-01/P6-05。最低 macOS15设备/兼容资产未验证的事实继续保留。上述后续平台项目不扩大本轮修改范围，也不被本轮局部 PASS 覆盖。

状态、owner 和依赖不变：68卡 DONE30 / ACCEPTANCE1 / DOING1 / READY0 / TODO29 / DEFERRED7；P2-05 ACCEPTANCE、P2-06 DOING及其Runtime/Helper/IPC所有权保留。无新 commit SHA、merge SHA 或 push。Host 依据本轮结果与分期规则确认收口前，独立分支保留。
