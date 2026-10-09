# OBG-P5-04 五种共享入站交付记录

Task：OBG-P5-04。负责人：Codex · P5-04 Core/Config。日期：2026-10-09。
当前状态：**DONE**，本卡 owner/写范围/自有资源预约已释放。基线 `eec981f34386e1f2461dbba137707ee814b58e4c`，分支 `dev/p5-04-shared-inbounds`；本地一次逻辑提交，不 push。

## 范围与实际交付

按用户本轮明确的职责分离范围，P5-04 完成 Core/Store/Compiler 与隔离最小加载；P2-06 原 owner 负责正式 Runtime Apply/Restore/旧计划回退，P5-07 承接组合验收。不新增 P5-04 对 P2-06 的依赖，不以本卡 DONE 声称产品 Runtime 已应用或 GPUI 已完成。

- `domain/shared_inbounds.rs`：五种封闭协议、固定 `listen`/`port` 与分享 `address` 分离；React 四种 Shadowsocks method、VLESS UUID/可选 TLS、TUIC UUID/密码/必选 TLS、Hysteria2 密码/salamander/必选 TLS、mixed 双空或成对认证。协议切换必须构造对应 enum，不夹带其它协议字段。启用项按实际 TCP/UDP 监听冲突校验，关闭项保留业务配置。
- `AppConfig.shared_servers`：唯一业务事实源；兼容 schema v9 增量字段，旧文件缺省为空，不创造第二套数据库/运行状态。既有 v8/v9 迁移回归通过。只推进 config revision；no-op 不推进，selection revision 不动。
- `SnapshotService::edit_shared_server(expected, command)`：生产 gate/JsonStateStore CAS；Create/Update/SetEnabled/Delete；返回 `SaveOutcome`，effect 只为 `SavedOnly`。失败不发布候选配置，不把 enabled 解释为当前 listener Ready。
- `SharedTls::read_local`：导入真实 PEM 后校验叶证书与私钥匹配；资源内容随同一个快照原子保存，源文件删除后磁盘重建可读，不依赖临时文件/路径。证书来源 `LocalPem`/`SelfSigned` 只是来源标识，均不证明客户端信任、有效期或域名信任。Debug 脱敏，生产 state/backup 文件保持 0600；没有提交真实私钥或凭据。TLS 使用已锁定 rustls 依赖，不改 workspace 或 Cargo.lock。
- `preflight_shared_port`：显式接收 owner 保留端口，分别报告 Reserved/Server/Listening/Unavailable；短时 TCP/UDP bind 后释放，只作提示，不能当最终监听成功。
- `SingBoxCompiler::compile_shared_inbounds`：现有正式 Compiler 的封闭入站产物 `CompiledSharedInbounds`（Serialize 为 inbounds 数组）；只编译启用配置，固定端口/认证/证书 PEM 逐项映射，不管理 child，不另存运行配置或恢复状态。TLS 资源直接内联，无额外资源路径。`compile_product` 在 P2-06 接通前显式拒绝已启用共享入站，避免静默漏掉用户配置。

关键文件位于 `crates/veyra-core/src/`。旧 React `ShareNetworkSettings.tsx`、helpers/types/CSS 保留；P5-05 UI/URI/QR 未实施；P5-06 shares HTTP/token/host/listen 源码与状态保持。Runtime/Platform/Helper/IPC/Runtime DTO 与 recovery 源码均未修改。

字段依据：[固定 v1.14.0 Shadowsocks](https://github.com/SagerNet/sing-box/blob/v1.14.0/option/shadowsocks.go)、[VLESS](https://github.com/SagerNet/sing-box/blob/v1.14.0/option/vless.go)、[TUIC](https://github.com/SagerNet/sing-box/blob/v1.14.0/option/tuic.go)、[Hysteria2](https://github.com/SagerNet/sing-box/blob/v1.14.0/option/hysteria2.go)、[TLS](https://github.com/SagerNet/sing-box/blob/v1.14.0/option/tls.go)，并以当前锁定二进制真实运行验证。

## 实际验证与证据

Local 执行仅 `/Users/lifeilin/.codex/worktrees/p5-04-shared-inbounds/veyra`。会话默认 cwd 仍为主树，已如实确认；所有执行显式指定隔离路径。开始时分支/HEAD 符合且工作树干净；文件创建/读回/删除、临时 index 的 git add/write-tree、commit-tree 对象写入及该 worktree index lock 创建/删除通过；未移动主树或修改其文件。

本地原始日志/receipt 保存在 [evidence/p5-04](evidence/p5-04/)，该目录按仓库现有规则不进入 Git；本记录保存可审查结果。源文件清单/摘要见 `source-identity.json`，aggregate `0e017f0d32ca3801afafb7065990470e54fc747e843d4e70a34ca69ac68917b1`。

| 验证命令/目标 | 实际结果 | 原始日志 |
| --- | --- | --- |
| `cargo test -p veyra-core shared_inbounds -- --nocapture` | 9 PASS，1 opt-in ignored；该 ignored 随后显式执行 | core-shared-final.log |
| `cargo test -p veyra-core domain:: -- --nocapture` | **45 PASS / 1 FAIL**，既有引用错误断言，见下文 | domain.log |
| `cargo test -p veyra-core storage:: -- --nocapture` | 24 PASS / 1 ignored；既有跨进程辅助入口，不算通过 | storage.log |
| `cargo test -p veyra-core application::state_service::tests:: -- --nocapture` | 36 PASS | state-service.log |
| `cargo test -p veyra-core singbox::compiler:: -- --nocapture` | 61 PASS | compiler.log |
| `cargo test -p veyra-core application::shares::tests:: -- --nocapture` | 9 PASS，真实自有 loopback HTTP 回归 | p506-regression.log |
| `cargo test -p veyra-core application::shared_inbounds::tests::kernel_tests::shared_inbounds_locked_kernel_check_load_conflict_cleanup -- --ignored --exact --nocapture` | 实际 1 PASS，非零项；真实内核矩阵见下表 | kernel-run.log / kernel-results.json |
| `cargo clippy -p veyra-core --all-targets -- -D warnings` | PASS | core-clippy-final.log |
| `cargo clippy -p veyra-desktop --all-targets -- -D warnings` | PASS | desktop-clippy.log |
| `cargo build -p veyra-core -p veyra-desktop` | PASS；仅工程构建，不冒充 GUI/包验收 | build.log |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings` | **FAIL**：旧 Tauri 缺 `binaries/sing-box-1.14.0-windows-amd64/LICENSE` | legacy-clippy.log |
| `cargo fmt --all -- --check`、`cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | PASS | fmt-check.log / legacy-fmt.log |
| `git diff --check` | PASS | 最终收口执行 |

从原始 test 名去重：**184 PASS / 1 既有 FAIL / 1 既有辅助入口 ignored**。新增 10 个测试均实际执行通过（9 定向 + 1 opt-in 内核）；Compiler 的产品拒绝测试与前述9项有交集，没有重复计数。计数清单见 `test-counts.json`。

生产 Store 证据包括五协议创建/更新/删除、启停/no-op、版本/CAS、重复与缺失 ID、真实 state.tmp 路径写失败保持原 JSON 字节、失败删除保旧、释放并独立重建 Service/Store 回读、旧 schema 缺省、证书导入/源文件删除后的重建回读与错误私钥/不匹配证书拒绝。没有 Mock 保存或预置假恢复。

### 固定内核

仓库锁定 official archive SHA256 `a150c94012ff768b7261939cd236b9c8554127f45137230295d23a5660225cc9`；binary SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，与 archive 内二进制逐字节摘要一致。版本 `1.14.0`、darwin/arm64、revision `0b8995879f29a9b98ee027bc17b75e101445b238`。使用现有官方资产的只读副本，执行验收前复核摘要；测试只执行自有临时目录副本。身份见 `kernel-identity.json`，不操作原资产或现有实例。

| 协议 | 合法 check | 非法 check | 实际加载 | bind 冲突 | 退出/reap/端口释放 |
| --- | --- | --- | --- | --- | --- |
| Shadowsocks | PASS，另3种 method 正例也 PASS | 错 method 拒绝 | PASS | TCP + UDP 各1次拒绝 | PASS |
| VLESS | TLS 正例 PASS，无TLS正例也 PASS | 非法 TLS key 拒绝 | PASS | TCP拒绝 | PASS |
| TUIC | PASS | 非法 TLS key 拒绝 | PASS | UDP拒绝 | PASS |
| Hysteria2 | PASS | 未知 obfs 类型拒绝 | PASS | UDP拒绝 | PASS |
| mixed | 认证正例 PASS，无认证正例也 PASS | 非法认证字段类型拒绝 | PASS | TCP拒绝 | PASS |

合计10正例/5反例 check、5加载、6真实冲突。每个固定监听端口先保存，再重建生产 Store 读回后编译；候选端口预检通过后用另一自有 socket 抢占，真实内核返回 address already in use，候选退出而旧测试 child 仍存活。只测试本卡配置及自有 listener，不实现第二运行管理器、不把保留未停止 child 当正式 Runtime 回退证明。测试 child 均 SIGTERM + wait/reap，重新 bind 核对端口释放；临时证书/私钥/配置/二进制/数据目录由 RAII 删除。未操作系统代理、TUN、Helper、用户现有实例或真实业务目录。

### 历史失败原样保留

- 初次编译失败（测试误用私有 compiler module）、初次 Clippy slice clone 警告已修复，原日志保留。
- 首次内核反例使用 VLESS 非标准 UUID 字符串，固定内核接受；该样本不能充作非法配置证明。改为确实非法 TLS key 后矩阵通过；原 `kernel-run-attempt1-fail.log` 保留。
- Domain 的 `rejects_invalid_pool_and_route_references` 预期 MissingPool，实际 InvalidFilter；用 `git archive eec981f...` 自有临时源码运行该完整 test 名，**1 FAIL** 同样复现。证据 `baseline-domain-failure.log`；未修改/删除/跳过该测试，不扩大范围修复。
- 旧 Tauri Clippy 的 Windows LICENSE 缺失保留 FAIL。Core/Desktop 工程检查通过不掩盖该限制，不伪造全仓通过。

## 独立只读 Review

审查者：独立子 Agent `p504_review`，使用 `code-delivery-review` Skill，实际读取最终 diff、未跟踪源码及上述原始日志/receipt。结论：**PASS，未发现可操作 P0/P1/P2/P3 finding**。覆盖协议语义、生产持久化/CAS/失败保旧、证书与凭据、真实端口冲突/退出、owner 边界。确认并保留 Domain 与 Legacy 失败；正式 Runtime 生命周期/恢复 Ready、P5-05 UI 和 P5-07 组合不在本次通过声明中。原报告见 `independent-review.md`。

## 给 P2-06 owner 的最小接口清单

1. **事实输入**：`AppState.app_config.shared_servers: Vec<SharedServer>`。`listen: IpAddr`、`port: u16`、protocol 凭据以及 TLS PEM 都来自已保存配置；`address` 只供后续分享，不参与 bind。不能将固定端口重绑为0，不能重新生成证书。
2. **编译调用**：现有 `SingBoxCompiler.compile_shared_inbounds(&state.app_config.shared_servers)` → `Result<CompiledSharedInbounds, SharedInboundValidation>`，Serialize 是封闭的入站数组，证书/密钥已内联，无额外资源路径。原 owner 在现有 `compile_product`/`CoreInbound`/`Document` 封闭模型中合入此产物，并替换当前显式 `UnsupportedProductOption::SharedInbounds` guard；不要绕过 GeneratedConfig 最终校验或增加另一运行配置。
3. **资源/预检**：`preflight_shared_port(server, servers, reserved, exclude_id)`，保留端口由 owner 的真实 mixed/controller/其它自有监听提供；返回 `SharedPortWarning { transport, issue }` 只是提示。证书随原计划/快照保留，不生成并行权威资源库。
4. **Apply/Restore 与失败结果**：原 owner 扩展既有 recovery 编码/恢复，以同一固定入站/PEM 完整恢复；监听失败传播真实错误，按既有机制尝试上一稳定计划，只有实际恢复成功才 Ready，否则保留错误/恢复材料。持续区分 SavedOnly 与 Applied/Ready。不修改 Helper/IPC 信任边界以绕过模型。
5. **验收归属**：正式 Runtime Apply/Restore/替换失败回退由 P2-06 完成；完整跨页/外部客户端链路由 P5-07 组合验收。本轮 P5-04 的独立真实配置加载已完成，不新增依赖、不领取 P5-05。

## 收口

68 卡按原显式依赖完整重算：DONE23 / DOING1 / READY5 / TODO32 / DEFERRED7，其余0。P5-04 DONE并释放 owner；P5-05 原三依赖全部 DONE，因此转 READY，**未领取/未启动**。READY=P0-08/P2-05/P3-01/P4-03/P5-05。P2-06 仍 DOING及原 owner/范围，P5-06仍DONE。没有本卡剩余验收缺口；上述已明确划归其它任务的正式生命周期/GUI/组合不冒充通过。独立基线失败不隐瞒、不改写历史。
