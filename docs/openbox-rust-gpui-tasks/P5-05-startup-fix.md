# P5-05 启动回归修复（2026-10-09）

Finding `P5-05-STARTUP-SNAPSHOT-001`：CLOSED。P5-05 保持既有 DONE；本次修复 owner/预约释放。P2-06 继续 DOING，原 owner 和 Runtime/Platform/Helper/IPC/公共 DTO 保留。历史 P5-05/P5-06 验收与失败记录未覆盖。

## 范围与身份

从主开发分支 `codex/dist-react-restore` 的 `e56c7322a91b97c1d663b9ee855c09fac5c180a6`，通过 Codex Desktop 原生 Worktree 创建 `p5-05-startup-drag`；分支 `dev/p5-05-startup-drag`。实际根目录 `/Users/lifeilin/.codex/worktrees/p5-05-startup-drag/veyra`，HEAD/文件写入/Git 暂存锁写权限通过。主树已有未跟踪 `tools/tests/__pycache__/` 保留；独立提交后本地合并、主分支复验，不 push。

仅修改 Desktop `app.rs`、`state_bridge.rs`、`ui/shared_network.rs` 和拖动局部 Tokens，以及本次文档。不改五协议模型、生产 Store、URI/QR、P5-06 独立 HTTP 或 P2-06 代码。

## 行为

- `SharedNetworkView::Updated` 和相邻 `SharesUpdated` 复用既有 `accept_page_snapshot()`，成功页面快照收敛 Ready，并通过既有 Runtime request gate 发起首次 Refresh。
- epoch/config_revision/selection_revision 的旧页面结果在取消当前请求之前被拒绝，UI 重新投影 bridge 权威快照；订阅页面同类投影顺序一并修正。
- 全局先到、页面先到、重复刷新均复用同一 gate；未增加状态来源或运行管理器。
- 用户本轮明确要求拖动占位/吸附：只从手柄开始拖动；按实际排版行高保留占位，最近行吸附，浮卡长名称省略。首个越过阈值的事件立即计算位置，极快拖放也不会漏掉落点。松手调用既有 Worker/Reorder/CAS 保存，拖动期间不改持久化顺序，取消恢复原列表。

## 实际验证

原始本地证据目录：[evidence/p5-05-startup-fix](evidence/p5-05-startup-fix)。长命令使用 Python subprocess 的 600s 外层超时；每条结果和完整命令见 `release-checks.json`。

| 验证 | 实际结果 | 原始日志 |
| --- | --- | --- |
| Desktop StateBridge | 11 PASS（3 新增） | bridge-release.log |
| 拖动占位/吸附排列 | 1 PASS（新增；含换行高度） | drag-release.log |
| SharedNetwork Worker/生产 Store | 6 PASS；1 locked-kernel 辅助入口 ignored，未计通过 | worker-release.log |
| Shares 状态回归 | 3 PASS | shares-release.log |
| 实际 Store 空/失败/重试/迟到回调 | 1 PASS | store-ordering.log |
| 分享删除重建启动（80轮） | 1 PASS | shares-startup.log |
| Desktop all-targets Clippy -D warnings / workspace fmt / Desktop build / diff | PASS | *-release.log |
| 旧 Tauri Clippy / fmt | FAIL / PASS；缺既有 Windows libcronet.dll，exit101 | legacy-clippy.log / legacy-fmt.log |

合计 **23 个唯一测试实际 PASS**，ignored 和过滤数量不计通过。既有锁定内核 check 不因 UI 启动/拖动修改重复认证，Core/Compiler 本轮无改动。保留早期宏歧义、prepaint 方法位置编译失败的原始日志，不改写为通过。

新增三项启动测试直接保护：全局 Busy→导航→页面先到→旧全局后到仍 Ready/首次 Refresh=1；正常全局先到/页面后到/重复刷新；旧 epoch/config/selection 回调不能覆盖新状态或取消新读取。拖动测试保护用户排序不丢 ID、向上/向下/原位及换行行高吸附。

## 真实 macOS GUI

测试只使用自有 `/private/tmp/veyra-p505-fix` 和独立 `me.disign.veyra.p505-fix` bundle，生产 Service/JsonStateStore；150% 浅色，内容1280×720逻辑像素，原图2560×1504。未启动内核、未修改真实用户配置/系统代理/TUN/现有实例。

正常启动最终签名 SHA256：`2a23fa9b41baf7abd8186f494c583f5eab6ae492f71a78e080cb851bff6bafa3`。`gui-normal-final.log` 只有一次 global Ready/initial Refresh=1，结果 Ok(Refreshed)；重建 App/Service 后排序仍从磁盘读回。原图 `gui-normal-final.png`。

同一最终源码为实机 page-first 使用已有 debug evidence_refresh 延迟真实全局读取回调投递3000ms，并打开既有 UI evidence 状态展示（不是 Mock 快照）。环境改变后的签名 SHA256：`c18cee0ea6612db7d253171f689ae33c99c5b305576e9ac3ddd63f3fbd3d534a`。`gui-page-first-final.log` 实际顺序：Busy request1→快速导航/Busy request2→page snapshot Ready→initial Runtime Refresh request1/Ok(Refreshed)→旧全局1/2 StaleGeneration→snapshot_loaded=true。首次 Refresh 恰好一次。

`gui-page-first-ready-final.png` 展示“本地状态”及真实 SavedOnly；底部“未运行”符合本轮没有启动内核，不以本地 Ready 冒充已应用/可连接。正常/延迟日志唯一 Refresh 的核对见 `gui-final-summary.json`。

真实拖动：末行向首行排列保存 `[3,0,1,2]`；最终构建长名称卡向下拖放保存 `[3,0,2,1]`；列表外取消保留同一磁盘顺序/config_revision，重建再读取确认。`drag-disk-final.json`、`drag-long-down-final.png`、`drag-cancel-final.png` 及 begin/snap/drop 日志留存。通知明确“已保存，待应用”。

**独立保留的视觉证据缺口**：CUA 原生 drag 是整段手势，截图请求在手势结束后返回；并发截图/按住点击未取得 active-drag 中途帧，系统 Screenshot 尝试也未得到可控录屏界面。因此占位/吸附有实现、算法测试与真实 drop/Store 证据，但动态占位视觉仍 `NOT_CAPTURED`，不能称该新增视觉已逐帧验收；后续补该帧属于本次 Desktop 拖动局部证据，不归 P2-06。旧 plain-list 原图保留，不冒充占位图。

## 独立 Review 与状态

只读 Reviewer `/root/p505_startup_review` 使用项目 code-delivery-review，核对最终 diff、23项原始日志、两个最终 GUI 路径与真实排序/回读。结论：启动 Finding 可关闭，无剩余可操作代码缺陷；上述动态视觉证据限制单独记录，P5-05 保持既有 DONE，不改历史验收。

68卡状态及显式依赖重算：DONE24/DOING1/READY4/TODO32/DEFERRED7；READY=P0-08/P2-05/P3-01/P4-03，均未领取，详见 `dag-final.json`。本次不创建下游任务，不扩大 Runtime 范围。

## 2026-10-09 动态补验增量

旧轮次 NOT_CAPTURED 与构建证据保留。后续用户手势发现完整原卡、松手晃动和排序通知问题，已在隔离 Desktop 范围修正；本轮真实原帧、候选 REWORK、最终构建身份和剩余在线同态对照见 [动态补验记录](P5-05-drag-visual.md)。本段不改变原启动 Finding CLOSED 或 P5-05 既有 DONE。
