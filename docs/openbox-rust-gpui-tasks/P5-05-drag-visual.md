# P5-05 动态拖动补验与反馈修正

日期：2026-10-09。基线 `d64f77ca8056f38a6add079e893d075d7e02bd99`；分支 `dev/p5-05-drag-visual`，隔离工作树 `p5-05-startup-drag`。P5-05 既有 DONE、启动 Finding CLOSED 保持；本次补验独立登记，不改写旧 NOT_CAPTURED。

## 实现范围

用户在真实手势中追加三项要求：完整原卡片跟随鼠标；松手时页面不能晃动；成功排序无需 toast。仅修改 Desktop `ui/shared_network.rs` 和清理局部 `DRAG_WIDTH` Token。列表和浮层复用完整卡片，使用实际宽高、字体、颜色与按下时相对卡片的抓取坐标。排序仍走原 AppServices Worker → SnapshotService → JsonStateStore。

松手后仅临时保留视觉顺序，保存期间不插入额外加载行；成功/错误/接收端关闭均清除临时顺序，由生产快照收敛。此状态不持久化，不修改 Runtime 或另建配置事实。只对 Reorder 成功关闭 toast，其它保存和错误提示保留。P2-06 Runtime/Helper/IPC/公共 DTO、Core/Store/Compiler/URI/QR、P5-06 HTTP 均未改。

## 证据与构建

本地证据目录：`evidence/p5-05-drag-visual/`（忽略的本机原图、原视频、原日志，不强行提交二进制）。只使用自有 `/private/tmp/p505-drag-visual` 的四条生产 Store 测试记录，loopback 18401–18404；一张三行长名称卡和三张普通卡。未启动内核、未改变用户真实配置、系统代理或 TUN。

最终签名 executable SHA256：`1250c9df1495541c3b136384fa2155d2969f9bef9180554fbe0b6e83c2002e4c`，构建清单 `wrapper-final-build-identity.json`。macOS、当前约定 150% 浅色环境，内容 1280×720 逻辑像素；渲染 backing scale 2 单独记录，不把该值当作 150% 的证明。

最终采集限定自有 Veyra Drag Verified 窗口6632/PID68251。真实系统录像 `gpui-wrapper-controlled.mov`（30秒）、`gpui-wrapper-motion.mov`（20秒）、`gpui-wrapper-held.mov`（120秒），均自然结束；`gpui-wrapper-held-stills/` 使用 `screencapture -C -l 6632`，索引记录时间/退出码。AVFoundation 零容差提取真实原视频60fps帧，保留实际时间和完整原图，不绘制模拟状态、不拼接图像。

此前候选真实 Store 写锁失败：`gui-write-failure-before.json` / `gui-write-failure-after.json` revision47、顺序和磁盘SHA完全一致，真实UI显示错误且恢复旧卡片。解除自有 writer-lock 后重试 revision48，`gui-anchor-retry-quiet.png` 无成功toast；释放旧App/Service/Store并重建最终构建后 `gui-wrapper-disk-reload.png` 回读同一顺序。最终构建取消 `wrapper-cancel-before/after.json` revision51和磁盘SHA完全一致。快速连续排序曾遇到生产版本冲突，真实错误提示保留；后续再次保存已恢复无错误列表，不能把该冲突帧算成功路径。

### 最终真实中途原图

`wrapper-held-evidence-index.json` 将固定窗口6632、PID68251、命令、时间与原图SHA关联到最终 executable SHA。`gpui-wrapper-held-stills/held-0072.png` 至 `held-0079.png` 是鼠标仍按住的真实中途帧：完整三行长卡、协议/端口/地址、全部操作按钮、手形和绿色占位同时可见。`held-0070` 是静态箭头，`0071` 尚未形成占位；`0080/0081` 已松手，严禁当作中途证据。用户完成单次长卡向上并按住5秒。

![最终长卡向上中途原图](evidence/p5-05-drag-visual/gpui-wrapper-held-stills/held-0076.png)

| 检查项 | 本机最终构建事实 |
| --- | --- |
| 等高占位 | 长卡 PASS：移除源卡后留下同源实测高度的空位，随吸附移动到第二行；原始位置并非另增一张副本 |
| 完整浮卡跟随 | 长卡 PASS：0072→0073→0076，手形与完整卡片一起移动，抓取点保持在原手柄附近 |
| 指针对应吸附 | 长卡向上 PASS：begin source3→slot2→slot1；0073/0076显示第二行占位 |
| 长名称 | PASS：三行与协议信息、操作区保留，字体/颜色继承原卡；浮卡水平移动到窗口外的部分按窗口边界裁切，未缩窄卡片 |
| 列表总高度 | 长卡 PASS：0072–0079其它三卡与列表底边保持稳定，无额外加载行 |
| 松手恢复 | 长卡 PASS：0080/0081恢复四张完整卡，边界和底边稳定，无成功toast |
| 普通卡下拖、列表外取消最终held原帧 | NOT_CAPTURED：真实自动drop/取消及磁盘结果已有，最终快速CUA录像未确认有效普通卡中途帧；旧候选该类原帧保留但不绑最终SHA |
| 四条同数据 OpenBox 中途对照 | NOT_CAPTURED，见下节 |

0072–0081原尺寸2784×1728一致；0082系统捕获边界变为2652×1596（窗口shadow/边界），不把全图像素变化当作页面晃动，也不把该帧作为唯一松手对照。比较均以应用内容坐标为准。`gpui-wrapper-controlled.mov` 的1800次60fps取样没有确认有效held帧，保留限制；`gpui-wrapper-motion.mov` 的绿色候选包含错误通知，不能仅按绿色像素判定占位。原视频和原图保留，不将解码后取样数记为功能测试数。


## 历史失败保留

- 初始 region 录像误捕前台 Codex，已删除隐私内容，`invalid-region-attempt.txt` 记录无效，不能作为 Veyra 证据。
- 旧 baseline 定时录像只有静态/动作结束帧，保留 NOT_CAPTURED。
- 初始完整卡候选编译借用错误、首次页面布局 panic 已修正；原始失败日志与 `pre-first-load-fix/` 保留。
- 600 秒候选录像曾被提前中断，exit130 且无导出文件；`full-card-record-stop.json` 保留 NOT_CAPTURED。
- a4c6416 候选系统截图实际捕获长卡上拖、普通卡下拖、侧栏取消，但浮卡字体/颜色存在差异，记录 REWORK，不能绑定最终 SHA。
- aa1c9f 候选根卡片偏移仍未生效；`gpui-anchor-video-frames/frame-5728-95.453.png` 真实held帧记录 REWORK。锁定 GPUI 的 `prepaint_as_root` 直接指定根原点，因此最终改为定尺寸根容器及内部 absolute 原卡片，偏移进入父子布局。
- 0f8258 候选真实录像自然导出和带指针 PNG 留存。用户确认“未晃动且无成功提示”；`release-0031.png` / `release-0035.png` 暴露抓取锚点偏移，记录 REWORK。最终改为按下时抓取坐标，不能用该候选证明最终抓点。

## 定向验证

原始日志以 `*-wrapper-final.log` 为准，命令与退出码见 `wrapper-final-checks.json`；每条外层 600 秒超时。

| 验证 | 实际结果 |
| --- | --- |
| Desktop shared_drag | 1 PASS |
| Desktop shared_network::tests | 6 PASS、1 ignored（锁定内核辅助入口，未计通过） |
| Desktop state_bridge::tests | 11 PASS |
| Desktop all-targets Clippy -D warnings | PASS |
| workspace fmt --all --check | PASS |
| Desktop build --offline | PASS |
| git diff --check | PASS |

共 18 个唯一测试实际 PASS，过滤数量与 ignored 不计通过。保护已有排序/行高吸附、生产 CRUD/回读、失败保旧/CAS/重试与页面快照启动收敛。新增视觉契约以真实 GUI 原帧验收，单测不冒充动态视觉；纯 Desktop 修改不重复认证内核或运行无关 Core/Tauri 检查，旧 Tauri 失败不改写。

## 在线对照边界与任务状态

真实在线 `http://192.168.1.6:3036/#/settings` 已打开，1280×720，原图 `openbox-reference-current-only.png` 和 AX 留存。在线只有一条现有真实记录，本轮未写其后端或运行实例；四条相同数据的向上/向下/取消中途同态 OpenBox 原帧仍 NOT_CAPTURED。React SortableList/CSS 的完整原卡、源宽高和相对抓点只补充实现语义，不能代替对应原帧对照。不能据此宣布动态视觉整体 PASS 或完全一致。

P5-05 历史 DONE 不回写；本次动态视觉补验保持 ACCEPTANCE，剩余为 Desktop 最终普通下拖/取消中途原帧、同数据 OpenBox 中途对照与对应视觉结论，不归 P2-06。68 卡 DONE24/DOING1/READY4/TODO32/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03，未领取下游，不新增依赖。不 push。

## 独立 Review

只读 Reviewer `/root/p505_drag_visual_review` 使用项目 code-delivery-review，已确认完整原卡复用、权威快照/失败收敛、范围边界和旧抓点 REWORK。最终复核确认原图0072–0079有效、0080起为post-drop，完整长卡/抓点/等高/高度稳定/松手恢复成立；无新增可操作代码 Finding。补验整体仍 ACCEPTANCE，因为最终普通下拖/取消held和四条同数据OpenBox原帧尚缺，不能判动态视觉整体PASS。

## 本地交付与主树复验

本次授权增量独立本地 commit，合并回 `codex/dist-react-restore`，不 push。提交/合并身份和主树未跟踪文件保护核对留存 `merge-result.json`；主分支复验命令、退出码和原始日志留存 `main-checks.json` / `*-main.log`。源码修复预约释放，动态证据补验 owner=Codex · Desktop，仅保留本记录/evidence范围。自有测试App在采集自然结束后停止，原视频/原图及自有测试根保留便于复核；可选视频提取中止不影响已直接落盘的最终系统原图。
