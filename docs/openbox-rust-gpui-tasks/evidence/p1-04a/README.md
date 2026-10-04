# OBG-P1-04A 视觉偏好与基础组件交付

状态：DONE。owner：Codex Desktop · GPUI / Core-Config。起始 HEAD `a71b6649eef3051d4b689d37b37298a18342ab14`，工作树干净；本轮修改未提交，未 push。只有本卡实施；P1-04B/P1-05 保持 READY。

## 验收与来源

三项任务验收均 PASS：[自动与构建验证](validation.json)、[实际交互](interaction-results.json)、[人工输入/拖动确认](human-interaction.json)。实现作者按仓库 code-delivery-review 技能自查；[记录](self-review.json)不声称独立 Host Review。用户后来反馈的 Input 上下裁切、侧栏小 icon、底栏高度与辨识度已修复并在实际浅深色窗口复测。

依据仓库当前 OpenBoxApp/AppShell/PanelSettings/CSS 和 P0-01、P0-03、P1-03 已有截图；没有访问远端参考站点。文件/构建/许可身份见 [source-provenance](source-provenance.json)。macOS 27.0 / 26A428、arm64、Rust 1.99.0、GPUI Kit 0.7.0 / gpui-pre 0.3.7，缓存离线构建，deployment target15.0；不声称已在 macOS15 实机运行。

## Core 与保存

现有 `AppState.app_config.visual` 是唯一持久事实，`JsonStateStore` 是唯一 state.json。类型化 DesktopVisualPreferences 保存 language(zh-CN/en-US/zh-TW)、theme_mode(system/light/dark)、None/ManagedAsset、opacity0–100(默认90)、blur0–40(默认10)、radius0–24(默认16)、sidebar_collapsed。sidebar256/64px，没有凭空新增 layout enum，没有行为偏好 B 字段。

schema7→8 显式迁移补视觉默认值，保留 check_updates、state_epoch、config_revision、selection_revision，不作为业务 edit；没有用 serde default 隐藏 shape 变化。Core DesktopPreferencesService 沿用共享 StateAccessGate、expected ConfigVersion CAS 和 atomic commit。冲突返回 RevisionConflict；真正变化推进 config revision、no-op 不推进；selection/epoch 不变，始终 SavedOnly，未启动 Runtime。

[Preferences](preferences.json)包含 System 和最终 Dark 两次真实退出/同根重启。最后值：zh-TW、dark、managed B、opacity20、blur0、radius13、collapsed；config41、selection0，epoch 从第一次 GUI 测试一直不变。重启前后七项、epoch/双 revision 完全相等，B 文件存在。UI Input/draft、搜索、页面 category 和 request generation 都是会话状态，不写入 Profile。

保存协调器400ms合并、每次仅一个 in-flight；陈旧 completion 不覆盖新 draft。自动10→20→30→40仅一次提交，最终40；真实人工连续 Slider 区间 generation7–396 有390次 change、2次提交（225/396），最终20/38。之后用户另行编辑最终 System/blur0，属于另一组合法操作，不把早前值当最终值。[保存计数](save-coalescing.json)区分 run总数与 Slider 区间。

真实隔离 state.tmp 目录阻断写入：saved radius14，draft13 保留，StorageFailed/error Toast；删故障 fixture，Retry 保存13/success Toast。背景 A 保存后准备 B，再阻断 state commit：Core仍A、A文件存在，B仅pending draft；Retry才提交B。旧资产只有在当前状态、备份状态、pending draft均无引用时才回收，并在共享 Gate 下检查，不能先删A。未引用的 prepared asset 属明确受管 orphan 集合，下次保存清理。[原子性](asset-atomicity.json)。

## Desktop 与控件

AppServices 只有一个 Tokio runtime，Arc Core services/asset store；后台仅搬运类型化请求、结果和路径。GPUI foreground receiver更新 Entity并notify。无 block_on，无 Entity/Context进入 Send task，无页面自建 runtime；[静态审查](static-review.json)。

正式组件使用成熟 Kit Input/Select/Switch/Dialog/Notification；theme按 OpenBox light/dark与 Panel dark局部覆盖，正文14/20、control32/row40、accent#70c996、panel圆角来自Preference，控件独立9px。Input最终用Styled单行height32、上下padding4、line20；Kit Input::h仅作用多行，Medium默认padding8导致原来文字视口不足。实际中英、Á与gjpq、Emoji在普通/Modal/页面Input完整。主导航变为许可明确的bundled Lucide SVG22px，折叠箭头20px，有accessible label/tooltip。底栏64px、说明/操作/计数分组、按钮绿边框与背景对比；跨route重置共享scroll，避免长Settings页偏移裁掉下一页标题。

Modal打开焦点到Input；Select用Down打开、Up/Down移动、Enter选择。Escape第一次只关dropdown，第二次关Modal；显式持久触发FocusHandle归还焦点，Space/Enter可重开。Tab/Shift+Tab有实际焦点环。最终独立窗口Switch以Shift+Tab取得焦点，Space off→on，Enter on→off。[focus-overlay](focus-overlay.json)。

中文IME由用户亲手普通/Modal输入、候选窗口、选词commit，回复“全部正常”。稳定ID拖入目标/区外取消及连续Slider同样由用户确认。原生自动工具不能证明candidate overlay或连续drag；不把paste当IME，也没有补造candidate截图或internal drag事件日志。普通中文选区用Cmd+Left/Shift+Right，Cmd+C/V得到“中文测试中文”，Undo恢复“中文测试”，Redo恢复，再Backspace为“中文测试中”；AX selected-range工具不支持，改用实际键盘。所有非业务Input仅session。

## 视觉与明确差异

[27张实际截图及SHA256](visual-manifest.json)：content1280×720 logical、scale2、全窗口2560×1506 physical，含66px physical titlebar。原生JPEG只重新编码PNG，没有裁切/缩放冒充尺寸。principal light/dark/sidebar/modal/input/restart图已在最终反馈修复构建更新；错误Toast/背景差值等较早实际验收图明确标为历史阶段，旧失败不改成通过。

- light/dark panel；light/dark sidebar expanded/collapsed。
- modal、modal-select、escape-dropdown-only、focus-return。
- toast-success、toast-error、save-failure-draft。
- background、background-opacity、background-blur。
- system/english/traditional panel；human-input-drag-result。
- input-clipping-before/fixed、modal-input-fixed、page-input-fixed、keyboard-input-editing。
- restart-restored(System)、restart-restored-dark。

MiSans Web WOFF2资产存在，但完整且许可明确的desktop TTF/OTF未锁定，继续系统字体/fallback；中文、英文、数字、Emoji、国旗、中英混排、Regular/Semibold均实际可见。图标来自Kit bundled Lucide，ISC/Feather MIT notice保留在 [desktop THIRD_PARTY_NOTICES](../../../../crates/veyra-desktop/THIRD_PARTY_NOTICES.md)，没有下载新字体或图标。

背景是真实受管PNG/JPEG，限8MiB、4096px边长、64MiB decode allocation；验证→临时写/fsync→rename/directory fsync→CAS→引用安全清理。opacity真实作用图片。blur用image fast_blur三箱Gaussian近似缓存，key=assetID/算法/blur值；1280×720 debug 首次blur20约838ms、blur38约853ms，CPU/I/O在Tokio blocking worker，重用缓存；前版普通Gaussian3.1–6.2s已替换。0–40是图片像素sigma，不等同CSS backdrop-filter或native blur intensity。Window-level material blur只能非零开/零关，未实现card backdrop blur。

Shell/Panel已有明确三语dictionary和三种Preference恢复；业务placeholder、分类/部分说明未完全翻译，不声称完整国际化。原生titlebar、系统字体、局部spacing/控件形状、图片blur与Web仍有差异；本轮没有六页完整业务视觉完成宣称。除Panel视觉A字段外，其余Settings分类、Overview/Proxies/Connections/Logs/Rules业务仍明确placeholder，未启动tray/Runtime/helper/updater/native picker/正式目录。

## 验证、DAG 与清理

Core280 tests（新增5，含既有migration/store/version回归），desktop17 tests（新增6）与真实Entity3组断言通过。指定core check/test/lib clippy、desktop check/build/test/all-target clippy、fmtall、tree、workspace --locked check通过；旧入口lib clippy也通过。desktop最终反馈修复后重跑 [final-commands](final-commands.json)。旧Tauri新StateValidation枚举必须增加一条错误映射，属于公共契约最小配套修复，没有新业务。

workspace `TAURI_CONFIG={"bundle":{"resources":[]}}`仅check覆盖，不代表旧包resource完整；已有block0.1.6 future-incompat warning保留。初次desktop clippy与workspace check失败日志保留，均修复；初次Core测试因旧schema7断言有1失败，已按current schema更新并全量280通过；首轮Input patch fmt失败，格式化后最终PASS。

依赖保持单一Kit0.7.0/gpui-pre0.3.7；image/sha2/serde_json为已有缓存版本的直接引用，无新UI framework。core无GPUI反依赖；desktop不依赖prototype/helper/Tauri，无tao/winit第二loop，default-members仍core。未删除P0 prototype或未完成旧业务切片。

[68卡实际依赖重算](dag.json)：DONE9、ACCEPTANCE1、READY2、TODO49、Windows DEFERRED7。READY只有P1-04B/P1-05；P0-05仍ACCEPTANCE，P0-06/P0-08未解锁，P2-01仍缺P0-06，未启动任何下游。

[清理](cleanup.json)：实际UI正常退出，pgrep无desktop/sing-box；全部本轮隔离state root/背景图片/app/故障fixture/screenshot staging删除，正式证据保留。未启动OpenBox HTTP、远端站点/公网验证、login或sing-box；没有读取/修改系统代理或TUN，用户当前TUN不作为本任务数据。所有Cargo命令offline；Core既有本地Mock测试不冒充OpenBox服务或真实网络验收。无commit/push。
