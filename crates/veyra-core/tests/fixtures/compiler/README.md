# P2-02B 产品 Compiler fixtures

`p2-02b.json` 是 AppState 的业务事实片段，叠加到新建空状态；三个 SOCKS 节点均无凭据，覆盖 hostname/IPv4/IPv6，两个 implicit provider pools 为 selector/urltest，站点路由包含 Direct/Proxy/Block。订阅 query 是 synthetic 脱敏回归标记，不能出现在生成配置中。

两个 expected JSON 覆盖完整产品输出：base 为 IPv6 off / rejectQuic off / directForNodes on / basic direct DNS；options 为 IPv6 node / rejectQuic on / split DNS。测试以固定路径/cache_id 与 `<runtime-secret>` 比较；这些不是可直接运行的 secret 配置。其他定向测试覆盖 directForNodes off、default Node/Direct/Block、分配端口、资源拒绝、旧快照兼容和 health 计划。

`cargo run -p veyra-core --offline --example p2_02b_candidate -- <0700-absolute-temp-root> base|options|direct|block|selected` 从同一输入生成 0600 的 finalized GeneratedConfig；只生成，不 check/run。命令必须在实际隔离 root 调用，示例用 loopback DNS/health；secret 随机，不能复制到 evidence。P2-02B 真实 check 与二进制/配置摘要记录在 local-only `docs/openbox-rust-gpui-tasks/evidence/p2-02b/`；P0-04 fixtures 仅提供 locked-kernel mixed/controller/cache 字段事实。

`p2-02b-selected.json` 是 P2-01 首次导入后的未配置默认状态，含三个保存订阅，active_subscription_id 仅选择第一个；其它 provider 的 implicit pool 不属于运行闭包。`selected` 模式实际调用 `project_selected_runtime()`，再以显式 `RuntimeIntent` / projected `OutboundId` 交给产品Compiler；runtime-active-* 不写回 state。定向测试还加入只匹配一个跨订阅节点的 custom pool，验证 sibling/无关订阅 host 和节点均排除，以及闭包外默认出口/Profile custom/runtime route 显式失败。

原 base/options expected JSON 保持原字节；完整已配置路径由调用方用 `RuntimeIntent::from_state()` 明确生成 intent。产品API只接收显式投影，所有运行身份和路由解析来自 runtime catalog。
