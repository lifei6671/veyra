# P6 macOS TUN 与生命周期

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**里程碑范围**：扩展已交付 helper，不新增权限服务；TUN 只等待自身 Compiler/Runtime/网络/DNS 依赖，更新使用 GitHub 下载后手动安装。阶段不作统一 Gate；以下各卡的显式依赖独立决定 READY。

<a id="obg-p6-01"></a>
## OBG-P6-01 TUN 与模式交接

**类型**：平台集成与 UI；**依赖**：OBG-P2-06、OBG-P2-07、OBG-P2-04、OBG-P2-02B、OBG-P2-05、OBG-P5-01。**依据/范围**：方案 §7.5、§8.9、§11；helper、core compiler/runtime、后端 TUN 设置。

**泳道 / 写范围**：Runtime/Platform + GPUI；helper TUN、Compiler/Runtime 交接与 TUN 控件；公共契约交 owner。

**执行与交付**：消费 P2-06 helper、P2-07 system proxy、P2-04 runtime recovery、P2-02B Compiler、P2-05 network 与 P5-01 已可编译/运行的基础 DNS；DNS Filter（P5-02）是独立产品功能，不作 TUN 技术前置；helper 执行受控 TUN 配置、必要权限与网络清理；手动/SystemProxy/TUN 的唯一 owner、配置/缓存交接和组合开关。

**验收**：

- [ ] 普通权限 GUI 经已授权 IPC 启停真实 TUN，关闭 TUN 恢复本实例资源。
- [ ] 开关组合、拒绝权限、端口/资源不可用不误报；交接无双实例/双 cache writer。
- [ ] 使用基础 DNS 完成真实 TUN/网络路径验证，不等待过滤资源；DNS Filter 完整性仍由 P5/P7 组合验收。生产实现下复验 Direct/ViaRunningProxy 与引导 DNS，无法证明直连时准确拒绝下载。
- [ ] GUI/helper 异常路径、重启中断提示及 TUN 控件完成真机和 UI 验收。

<a id="obg-p6-02"></a>
## OBG-P6-02 睡眠、切网与异常恢复

**类型**：生命周期；**依赖**：OBG-P6-01。**依据/范围**：方案 §11.1–11.3；core runtime、helper 恢复记录与平台通知。

**泳道 / 写范围**：Runtime/Platform；睡眠/网络通知与 helper/runtime 恢复清理。

**执行与交付**：睡眠/唤醒、Wi-Fi/有线切换、GUI/helper 异常及启动时未完成清理的恢复流程；复用现有按服务记录与实例身份。

**验收**：

- [ ] 唤醒和重复网络通知不重复启动 child 或覆盖原代理快照，短暂探测失败不永久禁用全部节点。
- [ ] GUI 实际退出触发 helper 清理；helper 重启先核对真实归属，不按名称杀进程。
- [ ] 网络恢复失败保留 RecoveryRequired 和必要端点；未完成清理前不能启动新实例。
- [ ] macOS 真机失败结果与日志可复查，不宣称断电/强杀 helper 后即时清理。

<a id="obg-p6-03"></a>
## OBG-P6-03 登录启动与统一退出

**类型**：原生 UI 与生命周期；**依赖**：OBG-P6-02、OBG-P1-06、OBG-P4-01、OBG-P5-06。**依据/范围**：方案 §9.5、§11.3；desktop 启动/托盘/退出与平台登录项。

**泳道 / 写范围**：Runtime/Platform + GPUI；登录项/desktop 退出与调度/分享清理。

**执行与交付**：登录启动开关及授权/撤销反馈；窗口隐藏、应用退出和资源清理统一；启动时先展示本地配置及恢复状态。

**验收**：

- [ ] 用户启用后登录启动有效，禁用后撤销；不额外承诺 GUI 退出后的永久后台代理。
- [ ] 关闭窗口保持业务；明确退出停止调度/分享、恢复网络并清理实例后退出。
- [ ] 登录启动、Finder 启动与托盘恢复各有独立真机结果，不能互相代替；界面检查完成。

<a id="obg-p6-04"></a>
## OBG-P6-04 更新路径与 helper 升级集成

**类型**：更新服务与 UI；**依赖**：OBG-P6-02、OBG-P0-08、OBG-P2-05。**依据/范围**：方案 §8.8、§14；core updates、原生桥、后端更新卡；API 14、17–19。

**泳道 / 写范围**：Runtime/Platform + GPUI；updates 下载/安装桥与更新卡/helper 升级协调。

**执行与交付**：实现 GitHub Releases 检查/下载/摘要核对/进度/取消及打开本地安装包；提示用户手动安装，helper 升级通过管理员安装入口，协调内核退出与资源版本。

**验收**：

- [ ] 无更新、下载失败、校验失败与取消明确；下载完成不冒充安装完成；外部手动安装不提供虚假的远程取消。
- [ ] 应用/helper/内核/资源版本一致，文件占用或网络清理失败时暂停升级，不强行覆盖。
- [ ] 所有实际下载进程的出站路径符合策略；更新不执行订阅或配置提供的安装命令。
- [ ] 更新 UI 经过操作与视觉验收；首版路线在本阶段完成集成，P7 只做发布级验证。

<a id="obg-p6-05"></a>
## OBG-P6-05 macOS 平台阶段验收

**类型**：阶段验收；**依赖**：OBG-P6-01、OBG-P6-02、OBG-P6-03、OBG-P6-04。

**泳道 / 写范围**：组合验收；所列依赖的证据/页面组合与本卡验收记录；修复先预约相关 owner 写范围。

**执行与交付**：固定 GitHub 分发候选包，在目标 macOS/架构执行授权、模式切换、切网/睡眠、退出与升级协调；记录最低版本和不支持组合。

**验收**：

- [ ] helper/SystemProxy/TUN/登录项均有真实权限证据，进程和网络资源归属可核对。
- [ ] 更新集成、独立 审查与平台界面检查完成；不存在推迟到 P7 的首次选型。
- [ ] Intel 若列为本次交付目标，单独构建验收；否则发布范围明确为已验证架构。
