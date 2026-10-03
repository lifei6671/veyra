# P1 核心库与桌面壳

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：目标是可独立启动、保存配置并通过托盘恢复的桌面壳；视觉与跨页面行为偏好最小拆分，不用占位页面冒充业务完成。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

<a id="obg-p1-01"></a>
## OBG-P1-01 抽取无 Tauri 依赖的核心库

**类型**：核心重构；**依赖**：OBG-P0-01、OBG-P0-02。**依据/范围**：方案 §4–5；`src-tauri/src/{application,domain,subscription,singbox,storage}/` → 新 `crates/veyra-core/` 与根 workspace。

**泳道 / 写范围**：Core/Config；根 Cargo workspace/锁文件、core 抽取与旧入口接线；workspace 单一 owner。

**执行与交付**：只移动本阶段需要的核心与有效测试；调度改用 Tokio runtime/Handle，抽离现有 SidecarPort/SystemProxyController；保留旧入口对共享核心的调用及 Windows 条件编译。

**验收**：

- [ ] core 在 macOS 独立构建，依赖树不含 Tauri/GPUI，公共路径不引用具体 Windows 实现。
- [ ] 相关纯单测实际执行且有测试数量；旧入口受影响的回归通过。
- [ ] 平台装配留在入口，不新增通用 Executor/平台工厂；记录新 workspace 的真实验证命令。

<a id="obg-p1-02"></a>
## OBG-P1-02 类型化服务、原子快照与版本

**类型**：状态与持久化；**依赖**：OBG-P1-01。**依据/范围**：方案 §6、§12；core 的 domain/application/storage。

**泳道 / 写范围**：Core/Config；domain/application/storage 公共领域模型、共享 DTO、StateVersion/Preferences；单一 owner。

**执行与交付**：定义稳定 ID、ProfilePatch、ApplyEffect、AppError 和 RuntimeSnapshot；单文件原子业务快照、共享 state_epoch、配置/选择计数与整体替换入口。若桌面 update preference 属于版本化 AppConfig，其字段、默认值与迁移在此定义，不依赖 P6 Updater 行为先实现。

**验收**：

- [ ] Missing/Null/Value 与数组替换行为明确；保存局部字段不丢失未编辑的 DNS/routing/TUN 字段。
- [ ] 写入失败保留有效旧快照；整体替换后旧表单/任务不能跨 epoch 写回，append 正确推进当前版本。
- [ ] 保存、当前运行、最后成功版本分别表达；没有运行实例时不凭历史记录显示 Ready。
- [ ] 服务错误保留稳定码/字段位置/脱敏详情；内部类型不退回任意 JSON 命令。

<a id="obg-p1-03"></a>
## OBG-P1-03 GPUI 壳层与异步状态桥

**类型**：UI；**依赖**：OBG-P1-02、OBG-P0-03。**依据/范围**：方案 §9.1–9.3；新 desktop `app.rs`、`ui/pages/`；参考 `OpenBoxApp.tsx`、`SettingsPage.tsx`。

**泳道 / 写范围**：GPUI；desktop app/navigation/state bridge；公共 DTO 变更交对应 owner。

**执行与交付**：建立六主页面/九分类导航和页面实体；AppServices 通道与 GPUI 更新桥；启动引导、错误/忙碌状态及草稿生命周期。

**验收**：

- [ ] 无配置可进入本地界面；无需启动 OpenBox HTTP 服务或浏览器登录流程。
- [ ] 页面切换保留适用的筛选/草稿；迟到回调、旧实例事件不污染新页面。
- [ ] render/点击回调不阻塞等待网络，GPUI 实体不被错误传入 Send 任务。
- [ ] 壳层与导航完成实际操作、浅深色及适用尺寸的 UI 验收；未实现业务明确标记。

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
