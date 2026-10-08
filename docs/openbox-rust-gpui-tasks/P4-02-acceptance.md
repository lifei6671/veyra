# P4-02 验收修正与最终交付

状态：**DONE**（2026-10-08）。在原隔离工作树 `p4-02-acceptance/veyra` 上继续，原提交 `88d7d137bca6f9068cf2e6bb946b97eff3a65e60` 及 `p4-02-roundtrip/` 证据保留。本次仅 Desktop groups/共享控件/i18n/tokens 与本卡文档增量；未改 Core、Runtime/Platform、helper/IPC、DTO 或 Cargo owner 范围。P2-06 保持 DOING。

## 本次修正

- 成员选择池与显示列表分离。搜索/筛选只改变显示；加入、移出所选按完整成员池的原始顺序处理全部勾选，包括隐藏项。两栏标题显示完整数量，全选/反选/清空仍作用于当前显示项。新增定向回归保护隐藏勾选、顺序及移出后的其余成员。
- 按 React/CSS 对齐出站节点徽标、24px 搜索/筛选、14px/20px 成员文字与36px行、16px复选框、禁用态、规则后缀、缩放30/48/30/52px组合及重置；补齐动态说明/命中预览、自动分组标题/说明/选项/生成数量。共享 Select、IconButton、Spinner 和通知在相关状态复用。
- 首次加载明确显示加载环；保存忙碌期间禁止重复保存/关闭及编辑，结果后恢复。失败通知复用全局 NoticeCenter，并修复通知被打开的弹窗遮挡的问题；失败保留草稿，重试仍走正式 Worker → Core Service → Store。

## 本轮验证结果

完整命令、退出码、唯一测试名及源码/截图 SHA256 见 [manifest](evidence/p4-02-corrections/manifest.json)、[checks](evidence/p4-02-corrections/checks.json)。Rust 命令均设900秒外层超时，`MACOSX_DEPLOYMENT_TARGET=15.0`。

| 检查 | 最终结果 |
| --- | --- |
| Core groups / catalog / product compiler | 12 / 7 / 18 PASS |
| Core selected projection / provider replacement / Store | 6 / 5 / 20 PASS；Store 1 ignored 子进程入口由父测试调用 |
| Core 唯一定向测试 | 68 PASS |
| Desktop `ui::`（含 groups 6、Select 3、icon picker 3及共享 UI） | 37 PASS |
| Desktop build、Core/Desktop all-targets Clippy `-D warnings`、workspace fmt | PASS，最终 `*-toast-final*.log` |
| 旧 Tauri Clippy / fmt | LICENSE 资源缺失 FAIL / PASS；无绕过、无扩大修复范围 |
| 正式 Store/Service 候选 → locked sing-box 1.14.0 `check` | PASS，exit 0；未 run 内核 |
| diff 与保护范围检查 | PASS；services.rs 临时延迟已还原，所有 Core/Runtime/Platform/helper/IPC/DTO 文件相对原提交无改动 |

旧 Tauri 失败仍为 `binaries/sing-box-1.14.0-windows-amd64/LICENSE` 缺失。早期 `--lib` 误用于 Desktop、Div 无 accessibility_label、私有 popup 模块导入等失败日志保留；均修复后重跑相关检查，不改写失败记录。编译器已有 `block v0.1.6` future-incompat 提示，当前编译/Clippy通过。

## 实机与视觉复核

当前 macOS 150% 显示缩放、浅色；逻辑内容1280×720，原生原图2560×1504含64px标题栏。比较图仅去掉原生标题栏并等比缩小；原图保留。React 使用原始 GroupSettings/helpers/CSS 和同一合成3节点/6组；授权内网参考仅查看，精确依据仍为本地源码。临时 harness 只替换 API 数据与受控延迟，未改 React 版式；外围导航是简化承载，不据其计算页面相似度。

| 状态 / 核对项 | 最终证据 / 结果 |
| --- | --- |
| 列表卡片、标记、说明及禁用操作 | [同状态对照](evidence/p4-02-corrections/comparison-list-production.png)，PASS |
| 静态编辑、搜索/筛选、徽标、缩放、复选框/禁用态 | [同状态对照](evidence/p4-02-corrections/comparison-static-production.png)，PASS；弹窗896×648、两栏404px、行36px、缩放158×32 |
| 动态标签、完整说明及预览行 | [同状态对照](evidence/p4-02-corrections/comparison-dynamic-production.png)，PASS；预览标题16px行高、成员20px行高 |
| 自动标题/规则/国家列表/说明/生成按钮 | [同状态对照](evidence/p4-02-corrections/comparison-auto-production.png)，PASS；弹窗576×464；双规则显示12，清空显示0且禁用 |
| 首次加载 | [真实加载](evidence/p4-02-corrections/comparison-loading-delivery.png)，PASS；生产 Worker 前6秒受控延迟，随后真实 snapshot 完成并恢复列表 |
| 保存忙碌 | [真实忙碌](evidence/p4-02-corrections/comparison-busy-delivery.png)，PASS；真实 Worker 保存请求，按钮禁用、spinner/保存中文案；点击禁用的取消后弹窗仍在 |
| 失败、通知、保草稿、重试 | [最终失败对照](evidence/p4-02-corrections/comparison-failure-production.png)、[重试成功](evidence/p4-02-corrections/gpui-retry-production.png)，PASS |
| 隐藏已勾选项批量加入/移出 | gpui-hidden-added.png / gpui-hidden-removed.png，PASS；逆序勾选HK3/HK1，隐藏后按HK1/HK3加入；移出隐藏选择保留US/Direct |
| 重建后读取 | gpui-restart-persisted.png、restored-final.json、failure-toast.json，PASS |

故障通过自有 `desktop/state.tmp` 目录使正式原子写失败；最终通知构建的旧磁盘 SHA256 前后同为 `34b60478593bd6c83c1c295ec92d59dd0c3d7e4c97890f760c049971e9dc371a`。撤除目录后点击同一草稿保存成功，恢复“静态验收”；没有直接改界面布尔状态、替换 Worker/Core 或伪造截图。此前无变更保存绕过写盘而成功的尝试不计故障证据。

已批准的不透明白色浅色弹窗保持。MiSans 官方静态face/GPUI CoreText 与 React MiSans-VF 浏览器栅格化沿用已有技术边界；原生标题栏不比较。不把整幅像素分数当作验收，也不宣称100%像素相同；本次要求的局部几何、文案、图标和交互状态逐项复核，无剩余可复现的本卡阻塞差异。深色、其它缩放和完整跨模块 E2E 仍按既有最终组合卡，不冒充本轮已运行。

## 构建、清理与任务状态

正式 Desktop SHA256：`09ab52c9dbd3b10609f92c88cfdbe473d2fa7da3e54993e1a00ba3ae574e4247`。最终列表/静态/动态/自动和失败通知均来自该构建。加载/忙碌受控构建 SHA256：`1e20d7c651b428c35d62583b4045fa34d3e7cd253b0e294d850a73917244e022`，保留 `controlled-delay.patch`；其后只修正失败通知路径/层级，不改变加载/忙碌样式。测试延迟未进入提交。

锁定内核 SHA256：`973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`，本轮候选配置 SHA256：`d507602abb91e9a16552ffa6eebd55981eea1dcf7df017339c79b9c23fe1379d`。

实现者按 code-delivery-review 做范围内自查，无剩余发现，不称独立审查。自有 GPUI 进程、临时 bundle/state/candidate、Vite、harness、node_modules软链已清理；临时浏览器页关闭、视口恢复。截图/日志/复现fixture作为本地证据保留；原轮证据和资源未改动，见 cleanup.json。

P4-02 DONE，Core/Config + GPUI 本卡 owner/预约释放。全量68卡重新计算：DONE21 / DOING1 / READY6 / TODO33 / DEFERRED7，其余0。P4-03 的 P4-02/P2-04 依赖全部DONE，转READY但未领取；涉及 Runtime/DTO 的写范围仍须协调 P2-06 owner。READY：P0-08、P2-05、P3-01、P4-03、P5-04、P5-06。仅本地范围明确的增量commit，不push；提交SHA记录在本地commit.json与交付回复。

---

## 原始提交的 ACCEPTANCE 记录（历史，原结果保留）

# P4-02 实现与局部验收记录

状态：**ACCEPTANCE**。基线 `956a2fac23bf83232655a48a2257773cced4535b`，独立分支 `dev/p4-02-acceptance`；Core/Config + GPUI owner 保留。P2-06 继续 DOING，Runtime/Platform、helper/IPC 与 Runtime DTO owner 保留。没有操作正式 SystemProxy、TUN、管理员安装或生产网络，没有执行跨模块完整 E2E。

## 实现与契约

- `crates/veyra-core/src/domain/groups.rs`、`groups/tests.rs`：静态/动态组、稳定 OutboundId、关键词/国家匹配、成员/组顺序、默认恢复、启停、图标、selector/urltest、独立 health URL。动态成员来自真实存在的有效启用来源，复用既有 NodePool/NodeFilter/目录；不新增节点启停 DTO。
- `application/state_service.rs`、`storage/store.rs`、`domain/state.rs`：正式 Store 原子 CRUD、config revision/CAS、v8/v9 读取、销毁实例重建恢复、失败保旧；保存返回 SavedOnly 与明确 dropped，引用错误定位 dangling/self/cycle/被引用出口。schema 仍为 9，旧文件缺 groups 按兼容缺省读取。
- `domain/outbound_catalog.rs`、`application/selected_subscription.rs`、`singbox/compiler.rs`、`compiler/recovery.rs`：统一注册 Group，选中订阅投影保留跨订阅引用闭包；compile_product 输出 selector/urltest 及恢复索引。空动态组可保存但不编译，并报告 dropped。健康 URL 不与 UI 测速偏好混用。
- `crates/veyra-desktop/src/ui/groups.rs`、services/state_bridge/app、共享 select/icon_picker/tokens/i18n：真实 worker/service 保存与重载；编辑、筛选、成员拖拽、组拖拽、国家自动分组、默认恢复、图标与缩放、启停、错误保留草稿、三语言即时切换与恢复。保存反馈与 Applied/Ready 分开；本轮未执行 Apply/Run。
- `examples/p4_02_candidate.rs` 提供私有临时根的真实 Store/Service 候选生成。其节点和 URL 全为合成 fixture。runtime.rs、singbox/runtime.rs、旧 Tauri runtime_adapter_tests 等配套改动仅补测试结构体 groups 字段，不改 P2-06 执行器、IPC、平台行为或选择 DTO。

用户已确认：禁用“拒绝”作为成员但保留独立拒绝出口；150% 指当前 macOS 显示缩放；浅色弹窗采用不透明白色代替 GPUI 无法等价实现的 backdrop blur。P4-03 failover 编排不在本卡实现。

## 定向验证

所有命令均在隔离 worktree；Rust 长命令外层 timeout 900 秒。构建设置 MACOSX_DEPLOYMENT_TARGET=15.0，复用隔离 target 缓存。原始日志在 [本地证据](evidence/p4-02-roundtrip/)，不提交截图/运行日志。

| 检查 | 结果 |
| --- | --- |
| Core groups | 12 PASS |
| Core catalog / selected projection / product compiler / provider | 7 / 6 / 24 / 5 PASS |
| Core Store 串行定向回归 | 20 PASS，1 ignored 子进程入口由父测试调用 |
| Core 合计 | 74 次执行，72 个唯一测试 |
| Desktop groups / select / icon picker | 5 / 3 / 3 PASS，11 个唯一测试 |
| cargo build -p veyra-desktop | PASS，build-6.log |
| cargo clippy -p veyra-core -p veyra-desktop --all-targets -- -D warnings | PASS，clippy-last.log |
| cargo fmt --all -- --check | PASS，fmt-last-check.log |
| cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings | FAIL，缺 binaries/sing-box-1.14.0-windows-amd64/LICENSE，未绕过 build.rs |
| cargo fmt --manifest-path src-tauri/Cargo.toml -- --check | PASS |
| locked sing-box 1.14.0 check | PASS，exit 0，stdout/stderr 空；只 check，不 run |

生产磁盘 roundtrip：创建、更新、删除、销毁 Store/Service、重建读取、全字段/成员/顺序/版本、旧 v8/v9 读取、默认组恢复、引用删除拒绝、失败保旧均有定向测试。真实 GUI 在自有测试目录创建 state.tmp 目录阻止原子保存，旧 state.json SHA 前后相等，草稿仍在；撤除故障后保存并退出重启，名称“静态重载”和 Direct/US 成员顺序恢复。见 ui-disk.json。

测试唯一名称以 evidence/p4-02-roundtrip/manifest.json 为准；日志保留原结果。初次 groups fixture 重名、Desktop 借用编译错误已修复；初次 Store 批次 19 PASS/1 FAIL/1 ignored，v1 迁移 ReplaceFailed 原因未确证，单项重跑 1 PASS、串行回归 20 PASS，不把最初失败改写。FIFO 忙碌态实验未获得有效证据，错误截图保留；恢复自有文件并终止挂起测试实例后正常重启保存成功，不计忙碌态 PASS。

## 实际界面与截图

使用 Codex Desktop 操作 macOS GPUI；当前显示缩放，浅色；内容逻辑视口 1280×720，native 截图 2560×1504（含 64px 原生标题栏），浏览器 1280×720。只在叠图时裁掉原生标题栏并按 2× 缩小，不改变原始证据。

真实浏览器读取授权内网参考；内网页面与仓库 React 实现存在差异，因此精确视觉依据仍为 GroupSettings.tsx/helpers、PanelIconPicker.tsx、types.ts、openbox.css。临时 localhost harness 直接加载这些真实 React 源码，以相同合成节点/组数据展示；其内存 API 仅服务视觉参考，GPUI 全程连接真实生产 Store/Service。没有访问外网 OpenBox API。

| 实际操作 | 证据 / 结果 |
| --- | --- |
| 静态编辑、成员排序、失败保草稿、保存及重启 | gpui-save-error.png、gpui-static-reloaded.png、ui-disk.json，PASS |
| 动态 US 筛选及独立健康 URL | gpui-dynamic.png，保存后待应用，PASS |
| 新建 HK 组、美国图标/+1、启停与组拖拽、删除 | gpui-order-disabled.png，PASS |
| 嵌套引用阻止删除 | gpui-reference-error.png，明确引用链，PASS |
| 国家组排序、空动态 dropped、默认恢复 | gpui-dropped.png、gpui-auto-final.png，PASS |
| 真实空 Store 与恢复默认 | gpui-empty.png，PASS |
| English / 繁体即时切换及重启恢复 | gpui-en-final.png、gpui-zh-tw.png，PASS |
| 加载/忙碌 | 未取得有效实际状态证据，NOT_RUN；FIFO 尝试失败不代替验收 |

[静态同状态并排图](evidence/p4-02-roundtrip/static-comparison.png)（左 React、右 GPUI）与 [50% 叠图](evidence/p4-02-roundtrip/static-overlay.png)保留差异；编辑弹窗主要几何 896×648、双栏/列表高度已对齐。动态/自动组分别保留 local-react-dynamic.jpg / local-react-auto.jpg 及对应 GPUI 原图。自动组背景组数不同，不以其全图计算相似度。

**未完成视觉验收**：静态列表“出站节点”类型徽标、筛选控件宽度、缩放控件外观、部分字重/图标尺寸及 disabled 样式仍需逐项对齐；未宣称 95% 或完全一致。浅色白色模态背景是已批准技术差异，不掩盖其余差异。其它主题/缩放及整体 E2E 按用户要求留最终组合卡。当前可实现的局部差异没有擅自改成永久设计取舍。

## 构建身份与独立 Review

Desktop binary SHA256：`cd8e1a45951ddc9e5d683459c1ba9c879a66fed822da09265cb1f827dae9294c`。隔离包 `/private/tmp/VeyraP402Acceptance.app`、`/private/tmp/VeyraP402Empty.app`；后续仅补 cfg(test) 翻译断言及格式化，无生产行为改动。完整源码与截图哈希见本地 manifest.json。

sing-box binary SHA256：`973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4`；候选配置 SHA256：`306fffcfd7b1dbca9f4e774e4c923460022cb018ec5ead46e0f3256e3ca4fa95`，见 sing-box-check.json。

独立子 Agent core_review 只读审查并复核：删除被引用组的明确错误、避免扩大已有不可用 Base pool 的校验、补移除/重试/加载中三语言均已修复；限定功能范围无剩余 finding。实现者另做 UI 实操与上述视觉差异登记。代码 review 不代替视觉验收。

## 状态与后续

P4-02 保持 ACCEPTANCE，剩余本卡工作是上述局部视觉修正及加载/忙碌实际证据，不推给完整 E2E。P4-03 仍 TODO。当前 68 卡：DONE20 / ACCEPTANCE1 / DOING1 / READY5 / TODO34 / DEFERRED7；READY 为 P0-08/P2-05/P3-01/P5-04/P5-06，下一可并行首选 P5-06（不在本轮领取）。P2-06 owner/DOING 不变。

本报告与实现仅形成一次本地 P4-02 commit，不 push；SHA 由提交后本地 receipt 及交付回复记录。临时 React 服务、页面、harness 和 node_modules 软链已清理，隔离 GPUI 已退出；原仓库仅保留原有 tools/tests/__pycache__/ 未跟踪目录。证据、隔离 bundle 与测试根保留用于复查，不清理用户资源。
