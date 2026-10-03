# P5 DNS 与共享

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：DNS 按 P0 已批准能力交付；不以连接数据补造 DNS 查询，不实现额外 DNS 协议栈。Shared Inbound 不等整个 P4；Subscription Share 与共享 UI 可并行，Rules 在 DNS 后最终闭合。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

<a id="obg-p5-01"></a>
## OBG-P5-01 DNS 上游、模式与重写

**类型**：业务与 UI；**依赖**：OBG-P2-02B、OBG-P2-03、OBG-P2-05、OBG-P0-07、OBG-P1-04A。**依据/范围**：方案 §7.7、§8.6；core dns/compiler、DNS 设置；API 75–76 与 profile DNS 字段。

**泳道 / 写范围**：Core/Config + GPUI；dns 上游/重写/Compiler 与 DNS 基础表单。

**执行与交付**：直连/代理两侧、UDP/TCP、主备用、模式/FakeIP、IPv6 与重写默认/自定义；测试输入绑定保存内容；新增真实 Compiler fixtures。Runtime 与网络路径来自 P2-03/05，不等待 P4 里程碑；profile ipv6 不与 UI diagnostics ipv6-test 合并。

**验收**：

- [ ] 每侧最多四项；测试失败不保存，测试 A 后修改成 B 必须重新验证；只提交 DNS patch。
- [ ] 恢复默认保留自定义且不产生重复来源；目标/来源修改按已验证 ApplyEffect 提示。
- [ ] FakeIP、缓存、IPv6 等字段通过锁定版本 check，不输出被移除格式；代理路径不可用时明确报前置条件。
- [ ] 表单、测试、错误与浅深色通过实际验收。

<a id="obg-p5-02"></a>
## OBG-P5-02 DNS 过滤资源与应用

**类型**：业务与 UI；**依赖**：OBG-P5-01、OBG-P2-05。**依据/范围**：方案 §8.6；core dns/filter/storage、列表/预览/应用弹窗；API 70–72、74。

**泳道 / 写范围**：Core/Config + GPUI；dns filter 资源/应用与弹窗；公共 DNS schema 交 owner。

**执行与交付**：过滤列表、允许域名、支持语法规范化、预览分页、下载/解析/应用进度和资源版本；受控配置生成；下载使用 P2-05 downloader/出站策略，不另建网络路径。

**验收**：

- [ ] 允许优先级、通配符与顺序符合样本；unsupported 数量和示例可见。
- [ ] 解析空结果/失败及网络失败不覆盖旧有效资源；保存和实际应用版本分别反馈。
- [ ] 超时/取消不误报成功，旧分页不覆盖新筛选；UI 的加载/错误/忙碌与浅深色通过验收。

<a id="obg-p5-03"></a>
## OBG-P5-03 DNS 记录、缓存清理与热更边界

**类型**：观测与 UI；**依赖**：OBG-P5-02、OBG-P3-01、OBG-P4-05C。**依据/范围**：方案 §8.6、§17；core dns/observation、查询记录与动作反馈；API 73、77。

**泳道 / 写范围**：Observation + GPUI；dns observation/cache capability、DNS 完整诊断与 Rules 集成。

**执行与交付**：按 P0 结论接入可靠事件/日志及清缓存能力；实现分页/筛选与热更或 RestartRequired；缺口呈现为明确能力状态。在 P4-05C 的 Routing/Terminal 基础诊断上补齐 DNS 解析链/记录与完整诊断，闭合 Rules 最终集成，消费同一 Unified OutboundCatalog。

**验收**：

- [ ] 有数据源时记录字段可追溯；无来源时显示不可用原因，不生成猜测记录。
- [ ] 清内核缓存与清 UI 历史独立；结果仅属于当前实例，失败不显示成功。
- [ ] 热更有实际生效证据；否则采用已批准的重启差异；范围变更不能冒充原能力通过。
- [ ] Rules 的 DNS 阶段来自实际可关联来源，无法关联明确标不完整；UI ipv6-test 不改变 Runtime profile ipv6，真实访问/加载/错误/忙碌完成实际验收。
- [ ] 分页竞态、空/错误/不可用状态与浅深色通过验收。

<a id="obg-p5-04"></a>
## OBG-P5-04 五种共享入站的配置与资源

**类型**：入站集成；**依赖**：OBG-P2-02B、OBG-P2-03。**依据/范围**：方案 §8.5；core servers/compiler、端口与 TLS 资源；API 50。

**泳道 / 写范围**：Core/Config；servers/shared inbound 领域/Compiler 与端口/TLS 资源。

**执行与交付**：按 Shadowsocks、VLESS、TUIC、Hysteria2、mixed 逐个完成封闭模型、凭据/证书、TCP/UDP 端口预检和 Compiler 样本；不一次混改全部协议；消费最小 Compiler 与 Runtime，不等整个 P4 完成。

**验收**：

- [ ] 每种当前协议都有字段映射、固定内核 check 及必要的最小加载证据；不重新验证内核协议算法。
- [ ] 监听地址与分享地址分离，证书存在且可读取；自签名/insecure 不伪装成可信证书。
- [ ] TCP/UDP 预检仅作提示，最终 bind 冲突准确报告；失败不破坏旧实例。
- [ ] 若某协议因固定内核/平台无法支持，取得明确范围修订后才能结束本任务，不默默删项。

<a id="obg-p5-05"></a>
## OBG-P5-05 共享网络编辑与 URI/二维码

**类型**：UI；**依赖**：OBG-P5-04、OBG-P1-04A、OBG-P1-05。**依据/范围**：方案 §2.3、§8.5；共享网络设置、core URI 与 QR 编码。

**泳道 / 写范围**：GPUI；共享网络编辑、URI/QR；共用编码/组件先由 owner 整合。

**执行与交付**：逐协议接通字段显示、校验、排序/启停、复制和二维码；保存与应用反馈来自同一受控配置。

**验收**：

- [ ] mixed 用户名/密码成对；IPv6 分享地址加方括号；切协议不带入不适用字段。
- [ ] URI/二维码凭据、地址和 TLS 标记与所显示配置一致，保存待应用时有清楚提示。
- [ ] 端口冲突、复制、取消和应用反馈真实；五种编辑界面的浅深色与实际操作通过验收。

<a id="obg-p5-06"></a>
## OBG-P5-06 订阅分享服务与界面

**类型**：按需服务与 UI；**依赖**：OBG-P2-01、OBG-P1-05、OBG-P1-04A。**依据/范围**：方案 §8.5、§13；core shares、订阅分享弹窗；API 51–55。

**泳道 / 写范围**：Core/Config + GPUI；shares 订阅分享服务/弹窗；与 P5-05 分开写入站/分享模块。

**执行与交付**：轻量只读 HTTP 服务、指定订阅集合、启停/编辑/删除、随机 token 与重新生成、URI/二维码；有实际证书才开放 HTTPS。本任务不依赖 Shared Inbound 或 P5-05，共用 URI/QR 组件由单一 owner 整合后可并行开发。

**验收**：

- [ ] 仅启用时监听，只暴露所选订阅；不能通过分享路由调用管理动作。
- [ ] 停用、删除或 token 更新后旧入口实际失效；应用退出清理监听。
- [ ] 不凭 URL 前缀声称 HTTPS/外网可达；复制与二维码一致，敏感内容不进入日志。
- [ ] 创建/编辑/错误/忙碌状态及浅深色通过界面检查。

<a id="obg-p5-07"></a>
## OBG-P5-07 DNS 与共享阶段验收

**类型**：阶段验收；**依赖**：OBG-P5-01、OBG-P5-02、OBG-P5-03、OBG-P5-04、OBG-P5-05、OBG-P5-06、OBG-P4-07。

**泳道 / 写范围**：组合验收；所列依赖的证据/页面组合与本卡验收记录；修复先预约相关 owner 写范围。

**执行与交付**：完成 DNS 保存/应用/恢复、共享入站与分享服务的组合回归；整理能力差异和各协议验证记录；复验 Routing/Client Routing/Rules 的 DNS 完整诊断及统一目录映射，Rules 最终能力在此组合闭合。

**验收**：

- [ ] DNS 测试失败不保存、旧分页抑制、资源失败保留等原行为有回归证据。
- [ ] 五种共享入站与订阅分享的可用按钮有真实行为；停用/退出可清理。
- [ ] 未实现的 DNS 等价能力继续标示差异；已批准范围内的功能与界面验证记录完整。
