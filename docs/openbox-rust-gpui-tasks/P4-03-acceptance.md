# P4-03 主备线路与手动选择：独立切片交付

2026-10-09，**ACCEPTANCE**，owner 保留。隔离工作树 `/Users/lifeilin/.codex/worktrees/p4-03-failover/veyra`，分支 `dev/p4-03-failover`，基线 `d713fa4621a0816fc56cefd473cf1b52b4b92218`。没有 commit、merge、push；P2-05 ACCEPTANCE、P2-06 DOING 和原 owner 不变。

## 已交付的真实行为

- `NodeGroup` 保存 Failover、有序 lanes、稳定 lane ID、线路内自动/手动模式及探测超时、连续失败阈值、恢复开关/等待时间。旧 v8/v9 普通组缺少新字段仍按 serde 默认读取，不新增第二份持久化出口目录。
- 主备外层 selector 和子 lane selector/urltest 全部进入已有 `OutboundCatalog`；沿用图校验、selected projection、正式 Compiler 和恢复计划。改名、排序不改变 lane PoolId。空线路、非法设置、重复 ID、缺失成员、自引用、循环和 ID 冲突明确失败。新建至少两条线路，既有主备编辑允许保留一条；上限三条，与 React 一致。
- 沿用 `SnapshotService::save_groups` / `JsonStateStore` 的原子写入和版本冲突反馈。失败保留旧磁盘，Service/Store 重建恢复数据；配置保存推进 config_revision，不触碰 selection_revision。
- `FailoverPolicy` 是独立的纯策略建议模块：连续失败切首个可用备用、稳定恢复后切主、新失败重置恢复计时、全失败明确错误、Auto/ManualPin 使旧批失效。每批身份包含 InstanceId、Group ID、SelectionVersion（epoch/revision）、ConfigVersion 和 generation。模拟单调时钟验证，不等待数分钟，不实现 URLTest 算法。
- GPUI 复用现有 GroupsView、成员双栏/排序、弹窗、Select、IconPicker、IconButton、NoticeCenter 和 i18n。实现主用/备用页签、添加/删除、名称/图标、成员顺序、线路自动/手动、高级参数、真实保存/失败保草稿/重试/取消。

## Runtime owner 协调及明确缺口

开始前向原 P2-06 owner（聊天 `1008 | 功能 | 生产 helper 与最小 IPC`）确认写范围，收到只读核对：现有唯一入口为 `RuntimeService::select_manual(ManualSelectionRequest)` → 单 worker → `ManualRuntime::select_manual`。它只支持 `state.pools` 中真实 Manual pool，不能把 Group 或合成 lane PoolId 当作已经支持的 pool。

本轮没有修改 `manual_runtime*`、RuntimeService、恢复/pending CAS、Runtime 公共 DTO、Helper/IPC 或 Platform；没有直接写 Controller。共享控件仅作现有组件的外观参数补充，旧默认值保留。

以下仍需原 Runtime owner 接通，均 **NOT_RUN / 未实现**：

1. Group/lane 的唯一选择命令入口、实例/Group/epoch/selection_revision 校验及 Runtime 确认结果；成功仅推进 selection_revision，失败保旧已确认选择。
2. 真正的组健康探测和定时编排消费；新实例/配置必须重建策略，人工操作先撤销旧批，恢复自动清累计后发新探测。
3. 模式持久化与重建、失败模式回退、响应不确定 pending、重连读回/核对/CAS。纯策略里的 ManualPin 不是已经完成的产品持久状态。
4. GPUI 手动固定/恢复自动的真实命令、保存中/错误/pending/重试反馈。当前“页签模式”只编辑子线路 selector/urltest，不等于外层 ManualPin，也没有伪造已应用状态。
5. 固定 1.14.0 自有 loopback 实例经上述唯一入口实际选择、读回、自动切换与重连验收。不能用直接 Controller PUT 或独立内核加载替代正式链路。

## 自动化与 Native 证据

所有定向命令都有 600 秒外层超时，原始输出和命令/exit 保留于本地 [evidence/p4-03](evidence/p4-03/)。未执行全量真实 child 测试。

| 验证 | 实际结果 | 证据 |
| --- | --- | --- |
| Core Groups（含 7 个 P4-03、12 个 P4-02） | 19 PASS，0 FAIL/ignored | `core-groups-02.log/json` |
| 统一 OutboundCatalog 回归 | 7 PASS | `core-catalog.log/json` |
| 正式 product Compiler/selected projection 回归 | 19 PASS | `core-compiler-02.log/json` |
| Desktop Groups / i18n / IconPicker | 6 / 3 / 3 PASS | `desktop-groups-last`、`desktop-i18n-last`、`desktop-icons-last` |
| Core / Desktop all-targets Clippy，`-D warnings` | PASS | `clippy-core`、`clippy-desktop-03` |
| Desktop 最终 build | PASS | `build-final-03` |
| workspace / 指定 legacy fmt | PASS | `fmt-last`、`legacy-fmt` |
| 指定 legacy Tauri Clippy | **FAIL，exit 101**：基线缺 `binaries/sing-box-1.14.0-windows-amd64/LICENSE` | `legacy-clippy.log/json`；未补造资源 |
| 最终配置 `sing-box check` | PASS，exit 0 | `candidate`、`kernel-check` |
| 正式 Runtime 实际选择/读回/切换/pending | **NOT_RUN** | owner seam 缺失，未绕过 |

固定内核为 sing-box **1.14.0**，revision `0b89958`，SHA256 `973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`；只对自有合成配置执行 check，没有 run 或 Controller 写入。

历史失败不覆盖：首轮 Core fixture 误用 commit 前配置版本而 FAIL，修正后通过；Desktop 首次命令错误使用不存在的 `--lib` 而 FAIL；首次 Compiler 过滤器、组件测试过滤器均命中 **0 tests，不计 PASS 用例**，Compiler 已用正确名称重跑 19 项；compact Switch 初次 Clippy 的多余 `.into()` FAIL 已修正，后续 PASS。实际 GUI 初次因未登记 ArrowLeft SVG 崩溃，保存 `gui-crash-before-fix.log`，同源 SVG 注册后最终构建正常。

## 真实 GUI 与视觉边界

自有 root `/tmp/veyra-p403-fixture-01a12040`，三个 loopback 合成节点和一个主备组。独立 app bundle `VeyraP403.app` 使用 root override；没有内核资源，Runtime 保持不可用。未操作用户 TUN、系统代理、DNS、路由、订阅或网络服务。

最终 unsigned binary SHA256 `65e30936f9a0f42cdd514028e2a40d5487370820bef446744de8193bb28fa434`；ad-hoc bundle executable SHA256 `08b8d14c340224c09f79e650035af346704801c53031e7813bab32b677465b5d`。保存失败/首次重启证据是前一构建，不能重绑最终 SHA；最终构建重新从磁盘恢复并验证真实排序保存。

| 真实操作 | 结果 / 证据 |
| --- | --- |
| 主备切页、线路内手动模式、添加备用、加入成员、非空删除确认、取消 | PASS；AX 实际状态；取消后磁盘 SHA 与初始相同 |
| 真实保存失败与保草稿 | PASS；在私有 root 建 `state.tmp` 目录，旧磁盘 SHA 不变，显示“保存失败，草稿已保留”；`gpui-save-failure.png` / `gui-save-failure.json` |
| 移除障碍后原草稿重试 | PASS；编辑器关闭，磁盘名称更新，稳定 lanes 不变；config_revision 1→2，selection_revision=0；`gpui-save-retry.png` / `gui-state-after.json` |
| 重建 Service/Store 并重启 GUI | PASS；名称、主用两成员、备用一成员、模式/设置恢复；`gpui-restart.png` |
| 最终构建真实成员拖动排序与保存 | PASS；主用 `a,b` → `b,a`，config_revision 2→3，selection_revision=0；`gpui-member-order.png` / `gui-order-after.json` |
| 高级展开、正文滚动至完整成员区 | PASS；`gpui-advanced-scrolled.png`；工具滚动方向需按实际内容移动核对，不是布局缺陷 |
| 瞬时加载/忙碌截图、所有错误/空状态、三语言即时切换 | 本轮未完整采集/验收，保留 **NOT_RUN**；不以静态 fixture 或用例 PASS 代替 |
| 外层人工固定/恢复自动与 pending GUI | **NOT_RUN**；缺 Runtime 正式入口 |

React 使用本仓库 **原 GroupSettings.tsx / helpers / openbox.css / 原字体图标**，临时本地壳只提供同数据 API，不作为完整应用壳对齐证据。已保留 harness 于 `evidence/p4-03/reference-source/`；临时工作目录与 node_modules 链接收尾移除。

最终对应原图：`react-primary-final.png` ↔ `gpui-primary-final.png`、`react-backup-final.png` ↔ `gpui-backup-final.png`、`react-advanced-final.png` ↔ `gpui-advanced-final.png`。两侧为同名称、成员顺序/模式、URL、参数、浅色与 1280×720 内容视口；GPUI 原图 2560×1504，顶部 64px 为原生标题栏，scale factor=2。沿用当前 macOS 显示设置，但没有独立采集系统设置来证明“150%”档位，因此 **150% 完整验收仍待确认**。

已修复高级按钮居中、方向不翻转、共用 lane 焦点 ID、成员移出图标位置、搜索外观、lane 图标字段固定 224px 覆盖页签名、高级 Switch 外观及单位字段宽度。弹窗边界同为 x192/y36、896×648 内容逻辑像素；高级展开后须滚动，保留真实截图。

**完整视觉仍 OPEN，不声称达到 95% 或完全一致**：复用 P4-02 的不透明白色弹窗与静态 MiSans 技术差异；下方局部几何、hover/focus/disabled/loading/error/空态以及 150% 系统档位仍需完整逐态复核。深色和其它缩放留最终组合验收。Legacy React/CSS/字体/图标均保留。

## 独立 Review 与交付状态

独立只读 Reviewer `/root/p403_review` 按 `code-delivery-review` 审查源码，无剩余源码 Finding。最初发现配置变更后旧探测 token 可撞代、lane 按钮焦点身份共用；分别加 ConfigVersion fence/实际旧 token 回归及 lane 稳定 ID scope 后复查关闭。随后复查共享 Switch/图标字段默认行为、高级方向与局部外观修改通过。Reviewer 没有运行 GUI、测试或 child；不把源码 Review 当 Runtime/视觉验收。详见 `review.json`。

P4-03 **ACCEPTANCE**：Core/Config/GPUI 编辑切片交付，Runtime/Native/完整 GUI 仍有上述明确工作；owner 与写范围保留，不解锁下游。Commit、Merge SHA、主分支重新验证、释放 owner 均 **NOT_RUN**，等待完整任务验收条件满足。不 push。

## Host 独立 Review 与 ACCEPTANCE 检查点（2026-10-09）

Host 核查基线 d713fa4 的唯一隔离分支、Group/OutboundCatalog/FailoverPolicy/正式保存及 GPUI 编辑器源码、原独立 reviewer 回执和 sing-box check 收据。Host **独立受管复跑** Core Groups **19 PASS**、OutboundCatalog **7 PASS**、Product Compiler **19 PASS**、Desktop Groups **6 PASS**、i18n **3 PASS**、IconPicker **3 PASS**，合计 **57 PASS/0 FAIL/0 ignored**；Core/Desktop all-targets Clippy -D warnings、Desktop build、workspace fmt、git diff --check 全部 PASS。旧 Legacy Tauri Clippy 缺 Windows LICENSE 的 FAIL 及历史编译失败记录未改写。

Host 查阅同数据 GPUI/React 主用、备用原图，确认主窗口与编辑区已有实际图像证据；暂不把未完整对齐的 hover/focus/disabled/loading/空/错误状态、150% 系统档位和完整视觉宣布 PASS。原始 GUI 失败保草稿、重试、重建与成员顺序收据为实现者实操证据，Host 未亲自操作 GUI。配置 check 不是实例切换；纯 FailoverPolicy 的 Auto/ManualPin 不是已完成 Runtime write authority。

与 P2-06 owner 边界保持：本轮仅 Group/Config/GPUI 独立切片，无新增已授权 Runtime 公共契约、pending/Controller/Helper/IPC 修改；其健康探测、手动固定与恢复自动、selection_revision/pending 不确定读回、正式 1.14.0 实际切换仍 NOT_RUN。**P4-03 保持 ACCEPTANCE、owner 保留，下游不解锁；当前 commit 仅为有证据的检查点，不标 DONE、不合并 codex/dist-react-restore、不 push。** P2-05 ACCEPTANCE、P2-06 DOING 均不改变。
