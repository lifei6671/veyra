# DCR-006：sing-box 聚合流量与内存趋势图

目标：TASK-009。用户明确要求仅内存统计与趋势图，并确认采用 sing-box 自带聚合统计；
包含由它处理的 Direct 流量，不统计绕过 sing-box 的系统网卡流量。
该功能要求授权其必要的有界内存读侧和安全展示字段；不新增核心接口、依赖或磁盘存储。
本候选须独立设计审查后实施。

## 数据与生命周期

- 实时上下行速率、当前内核累计字节沿用 DCR-005，不二次差分、不积分速率重建累计量。
- 趋势窗口默认 60 秒、最多 60 个点；只有一次成功的当前受管采样且 traffic 为 Some 时追加。
  日志、连接数、动作结果、Snapshot 读取、事件订阅都不能产生新点或网络请求。
- 内存 owner 为现有 `InMemoryRuntimeObservations`，不由窗口/React 组件持有权威采样历史。
  保留现有单条最新 Delta 队列；最新 Snapshot/Delta 携带一个有界窗口，不积压事件。
- 时间使用该内存 owner 创建以来的单调毫秒数，记录采样成功发布时刻，非核心精确测量时刻。
  后端更新时和读 Snapshot 时裁剪过期点；采样间隔仍由既有 bridge 控制，不能为画图额外采样。
- 启动新内核 Ready、确认 Stop、recovery 或已确认失败停止清空趋势。配置生成失败但旧
  实例仍健康时保留趋势。无实例变化的 AlreadyRunning、窗口隐藏/恢复不清空。
- 应用退出自然释放内存；不写 state.json、日志数据文件、数据库、localStorage/sessionStorage。
- 累计量显示“本次内核累计”，重启后使用新核心的累计值，不跨实例相加。

## 唯一新增安全读侧契约

既有零参数 Snapshot 命令与最新 Delta 事件同形新增：

```text
observedAtMs: 非负安全整数，当前内存 owner 的单调相对时间
trafficHistory: 最多 60 个点，按 sampledAtMs 严格递增
  sampledAtMs: 非负安全整数，不大于 observedAtMs，年龄不超过 60000ms
  uploadRateBps: 非负有限数字
  downloadRateBps: 非负有限数字
```

不包含原始日志、连接、目标、PID、secret、路径或核心内部实例标识。旧字段和枚举保持。
前后端同时更新 exact-key 校验，缺字段/多字段/超长/乱序/未来/过期点均拒绝；不加旧 DTO 兼容垫片。
对内部相同毫秒时刻的采样不可输出重复时间点，可覆盖同时间点末项；实际生产采样至少间隔1s。
Snapshot 的 observedAtMs 可在 revision 不变时前进；UI 接受更高 revision，或同 revision
但更晚 observedAtMs 的读取结果，避免恢复窗口时延用旧时间基准。较旧结果仍丢弃。

## 界面

- 在既有运行观测区域内展示两个速率数字、两个累计量及一幅 SVG 趋势图，不另建导航或产品入口。
- 图表横轴最近60秒到现在，纵轴自动按上下行共同最大值选择 B/s、KiB/s、MiB/s 等单位。
  以数值为准，上传实线、下载虚线及文字图例，不能只靠颜色辨认。
- 用真实时间间距定位点；相邻点超过5秒则断线，缺口不补零，不平滑编造中间测量。
  零速率是有效点，空窗口显示“等待网速采样”，读取失败显示不可用，停止显示已停止。
- 客户端只用 performance.now 的经过时间推动横轴和淘汰显示，不发网络请求，不增加采样点。
  卸载时清除显示计时器；收到新合法摘要时重新锚定单调相对时间。
- 320px窄屏不横向溢出；提供可读图名、轴标签、图例和独立当前数字，保留原Start/Stop/Toast行为。

## 实现边界与验收

- `application/observability.rs`：内存有界窗口、生命周期、Snapshot/Delta及定向测试。
- `commands.rs`：只映射上述两个新安全字段，补封闭序列化测试；必要的现有内测字面量适配。
- `src/App.tsx`、`src/styles.css`、`src/lib/observability.ts` 和对应现有/新纯函数测试：
  严格解析、时间窗SVG和交互；允许独立 `src/lib/traffic-trend.ts` / `.test.ts` 容纳图形计算，
  不增加依赖、网络地址、命令、capability、存储、后台采样循环或捕获模式。
- 后端验证容量/时间裁剪、只有真实采样追加、配置失败保留、Stop/新Ready/recovery清空、
  隐藏无订阅继续保留最新窗口、读取不制造点、DTO无敏感字段。
- 前端验证严格payload、同revision新时间基准、旧结果拒绝、60s淘汰、断线、零值、单位缩放。
  浏览器Mock IPC验证非零两曲线、空/错误/停止状态、窄屏、计时器及已有Toast，截图检查。
- 用现有Cargo/fmt/libraryClippy、pnpm lint/test/build与有界本机浏览器验证。
  Mock趋势不等于真实核心正流量；TASK-009既有网络/并发剩余项保持未通过。

## 既有设计变更

DCR-001禁止后台历史缓存收窄为禁止无界/持久/原始流历史；允许本文件明确的60秒/60点
安全速率窗口。Source Requirement与Task scope同步用户指令。此前checkpoint004保留历史身份，
不把既有41项测试追认成新字段/图表通过。
