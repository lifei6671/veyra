# Windows 后续任务

[长期开发规范](DEVELOPMENT_WORKFLOW.md) · [任务总表](IMPLEMENTATION_PHASES.md) · [当前交接](SESSION.md) · [方案](../openbox-rust-gpui-implementation-plan.md)

**范围状态**：全部 DEFERRED，不计入 macOS 进度。启动前单独确认 Windows 范围与排期；W0–W3 只作里程碑分组，各卡按显式技术依赖计算 READY，不等待整个 macOS P7 候选版。记录选定 core/desktop 交付版本和公共契约 owner；macOS 证据不能当成 Windows 验收。

<a id="obg-w0-01"></a>
## OBG-W0-01 Windows 构建与 GPUI 原型

**类型**：平台原型；**依赖**：OBG-P1-01、OBG-P1-03；另需 Windows 范围排期确认。**依据/范围**：方案 §11.4、§15.3；desktop、core platform/windows 与 Windows 构建配置。

**泳道 / 写范围**：GPUI + Runtime/Platform；Windows 构建/输入/托盘原型与适配，选定共享 core/desktop 版本；workspace 修改由单一 owner 整合。

**执行与交付**：固定 Windows/CPU 目标、toolchain 和内核资源；验证 GPUI、字体/输入法、托盘、文件操作，确定平台差异。

**验收**：

- [ ] Windows 目标真实构建并运行，输入/窗口/托盘有设备证据。
- [ ] 复用核心不依赖 macOS 实现；平台差异和开发/验证估算可审查。
- [ ] 明确 W1–W3 的范围和设备要求，状态总表转为实际排期。

<a id="obg-w0-02"></a>
## OBG-W0-02 服务权限、安装与 TUN 资源原型

**类型**：权限原型；**依赖**：OBG-W0-01、OBG-P2-03。**依据/范围**：方案 §11.4；目标 `crates/veyra-service/`。

**泳道 / 写范围**：Runtime/Platform；Windows SCM/Named Pipe/权限与资源原型，消费 P2-03 Runtime 公共契约。

**执行与交付**：验证 SCM 注册、UAC、LocalSystem 候选账户、Named Pipe 身份和锁定内核的驱动/资源加载方式。

**验收**：

- [ ] UAC 取消/拒绝正常返回；安装命令完成即退出，不增加常驻安装 helper。
- [ ] 固定服务权限和 IPC 身份真实可用，UAC 管理员身份不误当业务 owner。
- [ ] 按实际内核确定驱动/架构/资源，不预设必须另发 wintun.dll；必要权限有证据。

<a id="obg-w1-01"></a>
## OBG-W1-01 普通代理与当前用户系统代理

**类型**：平台与 UI；**依赖**：OBG-W0-01、OBG-P2-01、OBG-P2-02B、OBG-P2-04、OBG-P2-05、OBG-P2-08。**依据/范围**：方案 §11.4.1；既有 Windows sidecar/WinINet 适配、desktop 设置与托盘。

**泳道 / 写范围**：Runtime/Platform + GPUI；platform/windows 普通 child/WinINet 与 Windows 控件，复用共享订阅/Compiler/选择/网络/Proxy service。

**执行与交付**：桌面管理普通代理 child，复用当前用户代理快照/条件恢复；无服务时仍可使用。

**验收**：

- [ ] 未安装服务时导入、启动、访问、停止及退出可用，WinINet 在业务用户上下文修改。
- [ ] 外部代理修改不被覆盖，异常清理有归属证据；不在 LocalSystem HKCU 写用户代理。
- [ ] Windows 界面和托盘实际操作、浅深色及安装启动验收通过。

<a id="obg-w2-01"></a>
## OBG-W2-01 生产 Windows 服务与 Named Pipe

**类型**：权限宿主；**依赖**：OBG-W0-02、OBG-W1-01、OBG-P2-06。**依据/范围**：方案 §11.2.1、§11.4.2–11.4.3；service、core platform/windows。

**泳道 / 写范围**：Runtime/Platform；service、Named Pipe ACL/宿主适配，共享 IPC DTO 与 Runtime 契约交单一 owner。

**执行与交付**：受保护部署目录、SCM 按需启动、有限用户权限、显式 Named Pipe ACL、版本握手和受控 DTO；ProgramData 运行资料与 LocalAppData 业务数据分离。

**验收**：

- [ ] 仅本机且获授权的用户/会话能修改受管实例，任意路径/PID/命令与不兼容版本被拒绝。
- [ ] 去重、限额、超时查询与慢客户端处理符合共用契约，不重复创建 child。
- [ ] 普通用户没有服务配置/二进制改写权限；GUI 不因服务不可用而失去配置编辑能力。

<a id="obg-w2-02"></a>
## OBG-W2-02 TUN、Job Object 与 owner 交接

**类型**：平台与 UI；**依赖**：OBG-W2-01、OBG-P6-01。**依据/范围**：方案 §11.4.4；service 实例管理、core Runtime、TUN 控件。

**泳道 / 写范围**：Runtime/Platform + GPUI；Windows TUN/Job Object/owner 交接，消费 P6-01 交付的共享 TUN Compiler/Runtime 契约，不以 macOS 网络验收替代 Windows。

**执行与交付**：服务持有 child/Job Object/退出身份，普通↔TUN 交接、缓存与最后成功配置、跨用户独占和异常清理。

**验收**：

- [ ] 普通用户经授权可启停 TUN；交接无双 writer，服务与 GUI 状态一致。
- [ ] GUI 死亡与短暂断线区分；服务异常、会话切换、系统重启按真实资源恢复。
- [ ] 第二用户不能接管/停止他人实例；端口/驱动失败、恢复失败反馈准确。
- [ ] 真机验证 TUN 出站、应用下载策略、停止清理与 UI 操作。

<a id="obg-w3-01"></a>
## OBG-W3-01 Windows 安装、升级与卸载

**类型**：发布工程；**依赖**：OBG-W2-02、OBG-P7-01、OBG-P6-04。**依据/范围**：方案 §11.4.5；安装器/服务安装入口与发布配置。

**泳道 / 写范围**：Runtime/Platform；Windows 安装/升级/卸载，消费稳定备份 schema 与共享 GitHub 更新/版本契约。

**执行与交付**：GitHub 无 Authenticode 发布签名包、安装注册、版本握手、停止/替换/启动与失败回退；卸载先恢复本应用网络，再移除服务和程序。

**验收**：

- [ ] 干净设备安装及旧版升级可复现；文件占用/清理失败不强行覆盖运行文件。
- [ ] 卸载不遗留自动启动服务，不删除无法证明独占的共享驱动；用户数据按明确选择处理。
- [ ] 发布版本/资源/摘要一致；未知发布者提示如实记录，内核驱动的系统签名要求仍遵守，失败回退可用；实际发布另需明确授权。

<a id="obg-w3-02"></a>
## OBG-W3-02 Windows 独立交付验收

**类型**：平台里程碑；**依赖**：OBG-W3-01、OBG-P3-08、OBG-P4-07、OBG-P5-07、OBG-P7-02。

**泳道 / 写范围**：组合验收；Windows 实际设备证据及共享主页面/配置/DNS/维护行为在 Windows 的验证；不替代子任务验收。

**执行与交付**：在声明支持的 Windows/架构上执行方案 §16.2 的 Windows 场景，归档 GitHub 安装包、操作、权限、网络和人工视觉证据。

**验收**：

- [ ] 未装服务、UAC 拒绝、服务禁用/版本不符、TUN 失败、第二用户、GUI/服务崩溃、重启和卸载均有对应结果。
- [ ] Windows 自有行为的定向回归、必要审查和实际验收完成；macOS 结果不代替设备证据。
- [ ] 更新 SESSION 的 Windows 状态与已知限制，不修改历史 macOS 验收结论。
