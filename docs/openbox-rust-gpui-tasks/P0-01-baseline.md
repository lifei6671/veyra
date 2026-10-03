# OBG-P0-01 UI、接口与行为基线

[任务卡](P0-feasibility.md#obg-p0-01) · [主表](IMPLEMENTATION_PHASES.md) · [交接](SESSION.md) · [fixtures](fixtures/p0-01/README.md)

## 1 结果与身份

当前最终验收完成，P0-01 **DONE**：接口去向、同尺寸浅深色视觉、未知项登记与脱敏样本四项均 PASS。最新证据见[本轮最终验收](#current-acceptance)；下方旧批次的 NOT_RUN/403/启动失败与尺寸修复记录保留为历史。没有产品改动、生产配置写入、导入/重置/升级、commit/push/publish。owner=Codex /root，Core/Config + Observation + GPUI；实际写范围为本记录、fixtures/p0-01、P0 卡、主表、SESSION。产品 DTO、Cargo workspace、Runtime 公共契约只读；没有生产实例/网络资源预约。

- 源码 HEAD `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`，分支 `codex/dist-react-restore`；文档/fixtures 为未提交交付。
- 记录日期 2026-10-03（host 日期）。原会话创建 `2026-10-03T09:10:43.333Z` 转 Asia/Shanghai 为 10-03；官方标题工具确认写入 `1003 | 文档 | 固定 UI 与接口基线`。
- 起始 dirty-tree 仅 `.gitignore`、`AGENTS.md`、`DEVELOPMENT_WORKFLOW.md`；只读保留。AGENTS SHA256 `bb9b12eac752c4c7f71a3e3cfb94ffc32fd663eecd7f2ddb7515b5f244771947`，WORKFLOW `2cc6eb0173aacebffb30874f543ead90de234e09e7ed8c66ee2a0760fa29d925`，P0 卡起始 `c1df6b802b8c72862249ba0a1c153201063d709a5d1796872ff73f859a59d3f8`，与 host 引用一致。交付后 P0 卡预期改变；保留文件结束重新核对。
- 权威输入：[client](../../src/openbox/api/client.ts)、[types](../../src/openbox/api/types.ts)、[根组件](../../src/openbox/OpenBoxApp.tsx)、[页面](../../src/openbox/pages)、[CSS](../../src/openbox/openbox.css)、相关 hooks/tests；方案 §2–3、§18–19 用作计划索引，Rust 去向不算已观测 API。
- [Vite](../../vite.config.ts) 默认转发到 `https://openbox.disign.me`；client 的 VITE_OPENBOX_API_BASE 可以覆盖。本轮未读取凭据/env 内容、未确认实际远端版本。package.json 的 Veyra 0.1.0 不等于 Open-Box 版本。GPUI/内核/OS 未新增运行身份观测。
- CodeGraph discovery 被工具审批策略拒绝，未初始化/同步。改用已安装 TypeScript 7.0.2 AST 枚举属性、全 src 导入 api 的属性引用，分离 tests；[结构清单](fixtures/p0-01/source-inventory.json)包含完整声明、所有业务/测试引用与 69 个源码指纹（66 个 OpenBox TS/TSX + CSS/Vite/package）。只是源码证据，不是 HTTP 响应。

## 2 API 清点与去向

**77 方法，72 有非测试业务引用，5 无当前业务引用**：controllerConfig、version、devices、createDevice、deleteDevice。静态引用不代表所有分支实际执行。deployState 在 BackendSettings.helpers.ts 间接调用，用于 runKernelAction 断连恢复，不是页面直接入口。

77 按 api 属性顺序计数，与方案编号核对一致，不等于唯一 HTTP URL 数。每个实际 method/path/参数/返回泛型/解包/超时完整保存在结构清单 declaration 字段。下表“后续任务”是主表计划归属；业务路径省略 `src/openbox/` 前缀。没有逐一向远端执行接口。

| # | 方法 / client.ts 行 | 当前非测试业务引用 | 后续任务 |
| --- | --- | --- | --- |
| 01 | `authStatus` :153 | `OpenBoxApp.tsx:75` | P1-03 |
| 02 | `login` :154 | `components/AuthScreen.tsx:23` | P1-03 |
| 03 | `setupPassword` :155 | `components/AuthScreen.tsx:23` | P1-03 |
| 04 | `changePassword` :156 | `pages/settings/PanelSettings.tsx:135` | P1-03 |
| 05 | `storage` :158 | `OpenBoxApp.tsx:44` | P1-04B |
| 06 | `patchStorage` :159 | `pages/OverviewPage.tsx:141`、`pages/settings/PanelSettings.tsx:71`、`OpenBoxApp.tsx:61`、`OpenBoxApp.tsx:113` | P1-04B |
| 07 | `backgroundImage` :160 | `OpenBoxApp.tsx:51` | P1-04A |
| 08 | `saveBackgroundImage` :161 | `pages/settings/PanelSettings.tsx:107` | P1-04A |
| 09 | `deleteBackgroundImage` :162 | `pages/settings/PanelSettings.tsx:85` | P1-04A |
| 10 | `serviceStatus` :164 | `pages/settings/BackendSettings.tsx:35`、`pages/settings/BackendSettings.tsx:52`、`OpenBoxApp.tsx:70` | P2-03 |
| 11 | `serviceAction` :165 | `pages/settings/BackendSettings.helpers.ts:47`、`OpenBoxApp.tsx:124` | P2-03 |
| 12 | `controllerConfig` :166 | **无当前业务引用** | P3-07 |
| 13 | `version` :167 | **无当前业务引用** | P3-07 |
| 14 | `updateStatus` :168 | `pages/settings/BackendUpdateControls.tsx:24` | P6-04 |
| 15 | `kernelVersion` :169 | `pages/settings/BackendSettings.tsx:33` | P2-03 |
| 16 | `deployState` :170 | `pages/settings/BackendSettings.helpers.ts:52` | P2-03 |
| 17 | `checkUpdate` :171 | `pages/settings/BackendUpdateControls.tsx:51` | P6-04 |
| 18 | `runUpdate` :172 | `pages/settings/BackendUpdateControls.tsx:54` | P6-04 |
| 19 | `cancelUpdate` :173 | `pages/settings/BackendUpdateControls.tsx:58` | P6-04 |
| 20 | `factoryReset` :174 | `pages/settings/BackendSettings.tsx:55` | P7-02 |
| 21 | `trafficUsage` :175 | `pages/settings/BackendSettings.tsx:34` | P3-06 |
| 22 | `backup` :176 | `pages/settings/BackendDataSettings.tsx:33` | P7-01 |
| 23 | `importBackup` :177 | `pages/settings/BackendDataSettings.tsx:39` | P7-01 |
| 24 | `diagnostics` :178 | `pages/settings/BackendDataSettings.tsx:55` | P7-02 |
| 25 | `proxies` :180 | `pages/ProxiesPage.tsx:52`、`pages/ConnectionsPage.tsx:385`、`pages/RulesPage.tsx:48`、`pages/settings/SubscriptionSettings.tsx:104`、`components/OutboundPicker.tsx:63`、`pages/settings/ChainProxySettings.tsx:77` | P2-08 |
| 26 | `proxyLatencyHistory` :181 | `pages/ProxiesPage.tsx:52`、`pages/settings/SubscriptionSettings.tsx:106` | P2-08 |
| 27 | `selectProxy` :182 | `pages/ProxiesPage.tsx:121`、`pages/ConnectionsPage.tsx:469`、`pages/RulesPage.tsx:141` | P2-08 |
| 28 | `testProxy` :183 | `pages/ProxiesPage.tsx:162`、`pages/ProxiesPage.tsx:178`、`pages/ProxiesPage.tsx:193`、`pages/ConnectionsPage.tsx:497`、`pages/RulesPage.tsx:176`、`pages/settings/SubscriptionSettings.tsx:219`、`pages/settings/SubscriptionSettings.tsx:237`、`components/OutboundPicker.tsx:53`、`pages/settings/ChainProxySettings.tsx:109` | P2-08 |
| 29 | `testProxyGroup` :184 | `pages/ProxiesPage.tsx:139`、`pages/ConnectionsPage.tsx:482`、`pages/RulesPage.tsx:156` | P2-08 |
| 30 | `geoIp` :185 | `pages/ConnectionsPage.tsx:395` | P2-05 |
| 31 | `rules` :186 | `pages/RulesPage.tsx:48` | P3-07 |
| 32 | `penetration` :187 | `pages/RulesPage.tsx:100` | P4-05C |
| 33 | `terminalTestCapability` :188 | `pages/RulesPage.tsx:78` | P4-05C |
| 34 | `terminalTest` :189 | `pages/RulesPage.tsx:82` | P4-05C |
| 35 | `routeTest` :190 | `pages/RulesPage.tsx:74` | P4-05C |
| 36 | `closeConnection` :191 | `pages/ConnectionsPage.tsx:425` | P3-02 |
| 37 | `closeAllConnections` :192 | `pages/ConnectionsPage.tsx:435` | P3-02 |
| 38 | `siteLatencyHistory` :194 | `pages/OverviewPage.tsx:96`、`pages/OverviewPage.tsx:129` | P3-05 |
| 39 | `testSites` :195 | `pages/OverviewPage.tsx:128` | P3-05 |
| 40 | `trafficMonth` :196 | `pages/OverviewPage.tsx:96` | P3-06 |
| 41 | `trafficDay` :197 | `pages/OverviewPage.tsx:113` | P3-06 |
| 42 | `trafficDrill` :198 | `pages/OverviewPage.tsx:179` | P3-06 |
| 43 | `groups` :200 | `pages/settings/SubscriptionSettings.tsx:105`、`pages/settings/GroupSettings.tsx:70`、`pages/settings/RoutingSettings.tsx:42`、`pages/settings/ClientRoutingSettings.tsx:45`、`pages/settings/ChainProxySettings.tsx:75`、`pages/settings/ChainProxySettings.tsx:93` | P4-02 |
| 44 | `defaultGroups` :201 | `pages/settings/GroupSettings.tsx:214` | P4-02 |
| 45 | `saveGroups` :202 | `pages/settings/GroupSettings.tsx:86` | P4-02 |
| 46 | `subscriptions` :204 | `pages/ProxiesPage.tsx:52`、`pages/settings/SubscriptionSettings.tsx:102`、`pages/settings/BackendDataSettings.tsx:18`、`pages/settings/BackendDataSettings.tsx:52` | P2-01 |
| 47 | `previewSubscription` :205 | `pages/settings/SubscriptionSettings.tsx:139` | P2-01 |
| 48 | `testSubscriptionNodes` :206 | `pages/settings/SubscriptionSettings.tsx:519` | P2-08 |
| 49 | `testChainProxy` :207 | `pages/settings/ChainProxySettings.tsx:41`、`pages/settings/ChainProxySettings.tsx:109` | P4-06 |
| 50 | `checkServerPort` :208 | `pages/settings/ShareNetworkSettings.tsx:34` | P5-04 |
| 51 | `subscriptionShares` :209 | `pages/settings/SubscriptionSettings.tsx:103` | P5-06 |
| 52 | `createSubscriptionShare` :210 | `pages/settings/SubscriptionSettings.tsx:283` | P5-06 |
| 53 | `updateSubscriptionShare` :211 | `pages/settings/SubscriptionSettings.tsx:275`、`pages/settings/SubscriptionSettings.tsx:304` | P5-06 |
| 54 | `regenerateSubscriptionShare` :212 | `pages/settings/SubscriptionSettings.tsx:323` | P5-06 |
| 55 | `deleteSubscriptionShare` :213 | `pages/settings/SubscriptionSettings.tsx:337` | P5-06 |
| 56 | `createSubscription` :214 | `pages/settings/SubscriptionSettings.tsx:164` | P2-01 |
| 57 | `updateSubscription` :215 | `pages/settings/SubscriptionSettings.tsx:164`、`pages/settings/SubscriptionSettings.tsx:192` | P2-01 |
| 58 | `refreshSubscription` :216 | `pages/ProxiesPage.tsx:207`、`pages/ProxiesPage.tsx:221`、`pages/settings/SubscriptionSettings.tsx:178`、`pages/settings/BackendDataSettings.tsx:45` | P2-01 |
| 59 | `deleteSubscription` :217 | `pages/settings/SubscriptionSettings.tsx:206` | P2-01 |
| 60 | `profile` :219 | `pages/settings/RoutingSettings.tsx:26`、`pages/settings/ClientRoutingSettings.tsx:45`、`pages/settings/ChainProxySettings.tsx:75`、`pages/settings/ShareNetworkSettings.tsx:75`、`pages/settings/DnsSettings.tsx:55`、`pages/settings/DnsSettings.tsx:56`、`pages/settings/BackendSettings.tsx:29`、`pages/settings/BackendSettings.tsx:32` | P1-02 |
| 61 | `saveProfile` :220 | `pages/settings/RoutingSettings.tsx:32`、`pages/settings/ClientRoutingSettings.tsx:58`、`pages/settings/ChainProxySettings.tsx:90`、`pages/settings/ShareNetworkSettings.tsx:84`、`pages/settings/DnsSettings.tsx:64`、`pages/settings/BackendSettings.tsx:45` | P1-02 |
| 62 | `defaultRouting` :221 | `pages/settings/RoutingSettings.tsx:98` | P4-05A |
| 63 | `ruleSetEntries` :222 | `pages/settings/RoutingPickers.tsx:15`、`pages/settings/RoutingPickers.tsx:103` | P4-04 |
| 64 | `importRuleSet` :227 | `pages/settings/RoutingPickers.tsx:131` | P4-04 |
| 65 | `refreshRuleSet` :228 | `pages/settings/RoutingPickers.tsx:80` | P4-04 |
| 66 | `clients` :229 | `pages/settings/ClientRoutingSettings.tsx:47` | P4-05B |
| 67 | `devices` :230 | **无当前业务引用** | P0-02 |
| 68 | `createDevice` :231 | **无当前业务引用** | P0-02 |
| 69 | `deleteDevice` :232 | **无当前业务引用** | P0-02 |
| 70 | `dnsFilter` :233 | `pages/settings/DnsSettings.tsx:55`、`pages/settings/DnsSettings.tsx:56`、`pages/settings/DnsSettings.tsx:57` | P5-02 |
| 71 | `saveDnsFilter` :234 | `pages/settings/DnsSettings.tsx:92` | P5-02 |
| 72 | `applyDnsFilter` :235 | `pages/settings/DnsSettings.tsx:66` | P5-02 |
| 73 | `dnsRecords` :236 | `pages/settings/DnsQueryRecords.tsx:18` | P5-03 |
| 74 | `dnsPreview` :237 | `pages/settings/DnsFilterSettings.tsx:15` | P5-02 |
| 75 | `testDnsUpstream` :238 | `pages/settings/DnsSettings.tsx:24`、`pages/settings/DnsSettings.tsx:76` | P5-01 |
| 76 | `dnsRewriteDefaults` :239 | `pages/settings/DnsSettings.tsx:89` | P5-01 |
| 77 | `flushDns` :240 | `pages/settings/DnsSettings.tsx:67` | P5-03 |

传输消费事实：request 带 Accept/语言头、JSON body Content-Type 和 Cookie；空 body→null，application/json 或 `{`/`[` 开头尝试 JSON，解析失败保留原文。非 2xx 从 code/error、message/error 提取 ApiError，401 发 unauthorized 事件。成功只 `as T`，无 schema 校验。site/chain 30s、ruleset refresh 120s、DNS apply 300s 为显式 AbortSignal，其他 request 无统一显式超时。geoIp 独立请求三种第三方 provider。

## 3 四类流通道

| 通道与消费字段 | 当前去向与行为 | 计划归属/边界 |
| --- | --- | --- |
| connections；connections、closedConnections?/downloadTotal?/uploadTotal? | `/api/controller-ws/connections`→useLiveMetrics→壳层/Connections；按 id 对 download/upload 差分，消失连接合并去重，本地 closed history 最多 100；UI 反向显示 chains | P3-01→P3-02；closedConnections 不应当作 wire 保证；短连接缺口、总量和实例身份未实测 |
| logs；type/payload | `/api/controller-ws/logs?level=...`→LogsPage；UI 添加 id/time/category，最多 1000；暂停丢弃收到帧，不补录；无效 Regex 提示 | P3-01→P3-03；服务端时间、丢帧/级别/类别语义未实测 |
| memory；inuse/oslimit? | `/api/controller-ws/memory`→useLiveMetrics→壳层/Overview；只消费 inuse，按 MiB 展示 | P3-01→P3-05；oslimit 不消费，单位和归属不能从格式反推 |
| traffic；up/down | `/api/controller-ws/traffic`→useLiveMetrics→壳层/Overview；接收时间差分，elapsed 最小 0.1s，负值截零，首帧 rate=0 | P3-01→P3-04/06；源码作累计差分不证明实际累计/区间/单位 |

依据：[socket hook](../../src/openbox/hooks/useControllerSocket.ts)、[metrics hook](../../src/openbox/hooks/useLiveMetrics.ts)、[日志](../../src/openbox/pages/LogsPage.tsx)。JSON.parse 后断言泛型；connecting/open/closed/error，close 后 2s 重连，清理关闭 socket。metrics.connected 为三个指标通道任一 open，不能表示四通道齐备；previousTraffic/previousConnections 没有按实例变更清零。本卡只登记，不修产品行为。

## 4 六主页面与九设置分类

RouteKey、OpenBoxApp routeKeys 与 AppShell routes 同六项顺序；SettingsPage sections 与组件分支逐项核对。下面是源码行为/状态，实际操作本轮 NOT_RUN。

| 路由 / 页面 | 关键动作、弹层和状态 |
| --- | --- |
| overview / 概览 / OverviewPage | 四站测速、四图独立暂停、月日小时/direct 计量偏好、node/host/client 下钻；loading/请求错误/防旧回写；最新 delay=0 不沿用旧成功 |
| proxies / 代理 / ProxiesPage | 策略/节点/订阅 tab、选择/局部测速/刷新、搜索排序/穿透；策略设置 Modal :374、历史 Tooltip；加载错误/空筛选/未测/超时/busy。测试 13/2 是参考样本数量，不是生产约束 |
| connections / 连接 / ConnectionsPage | active/closed、排序聚合/列拖拽/标签、关闭、IP/关联策略；详情 Modal :612、设置 Modal :941；空/详情错误/策略不存在/busy；IP 查询失败保持未知 |
| logs / 日志 / LogsPage | 级别/类别/Regex/URL 格式化、暂停/清空/导出；空/无效正则；没有单独呈现 socket error 的面板，不能凭通用规范称已覆盖 |
| rules / 规则 / RulesPage | 已加载列表、穿透/内核诊断/终端能力、出口选择测速；两区五阶段面板（非 Modal）；加载失败/空规则/不可用/不完整/局部失败/busy |
| settings / 设置 / SettingsPage | 横向九分类，sessionStorage 恢复，订阅 header action；状态由子页负责 |

| 分类 id / 名称 / 组件 | 关键弹层/动作 | 静态状态与边界 |
| --- | --- | --- |
| panel / 面板设置 / PanelSettings | 密码、四站恢复、图标 picker、背景文件/URL/透明/模糊/圆角、主题/测速偏好 | 串行保存队列、预览/busy/失败 Toast；主题/侧栏会 PATCH storage，无写生产采样不可点击这些保存操作 |
| subscriptions / 订阅管理 / SubscriptionSettings | 添加/修改订阅或节点、预览/测试、删除确认、分享编辑/再生/删除/二维码 | 预览代次/busy/空/错误/部分测试刷新失败；token/url/content 必须脱敏 |
| groups / 出站节点 / GroupSettings | 编辑/国家自动组/成员 picker、主备 lanes/manual/阈值、排序、默认恢复/删除确认 | 加载错误/空/busy/dropped/dangling；保存 manual 不证明运行选择写权 |
| routing / 目标分流 / RoutingSettings | 站点集/自定义/兜底编辑、删除/恢复、规则详情/导入/出口 picker | loading/error/busy/校验；最多 200 导入条目、请求代次、部分 patch、保存提示重启 |
| clients / 终端分流 / ClientRoutingSettings | IP/MAC/已知终端、编辑/删除、旁路/准入/出口/排序 | loading/error/busy/空；clients 请求失败退为空，与实际空不能区分；保存提示重启 |
| chain / 链式代理 / ChainProxySettings | 字段/链接模式、上游、草稿测试、编辑/删除、启停/排序 | 请求 revision/防旧回写、延迟与 IP 失败分开；保存成功但候选刷新失败另提示 |
| share / 共享网络 / ShareNetworkSettings | 五协议编辑/端口检查/URI/二维码/删除 | loading/error/busy/空/凭据校验；App 扫码 tab 禁用“即将开放”；保存提示重启 |
| dns / DNS 设置 / DnsSettings | 模式/FakeIP、上游/重写/恢复、名单/预览/删除/放行、查询表/分页、应用/清缓存 | loading/error/busy/先测后存/pending/connected/applied/queryLog.reason/空与失败/防旧响应；UI 提示不是热更证据 |
| backend / 后端设置 / BackendSettings | 启停/参数/保留期、更新检查/下载/进度/取消、append/replace 导入预览、导出诊断、10s 重置确认 | ok:false/断连 deployState 时间判别、关闭更新窗不取消、导入刷新部分失败；客户端配置禁用 |

Modal 声明 31 处，一个动态 title 可对应多个弹窗；不等于 31 次实测。共享 Modal Portal 到 .openbox-app/body，role=dialog/aria-modal、关闭按钮/遮罩点击；无通用 Escape、焦点 trap、关闭后焦点归还实现，不能把方案要求写成已实现。反馈来自 shared ErrorState(role=alert)、spinner、Toast 和各页逻辑。

相关测试仅阅读：api/client、六页面与九分类、OutboundPicker、controls，共 18 文件；helper/SSR/字符串结构断言都不冒充实测。重点样本保护部分 patch、先测再存、排序/引用、迟到响应、部分失败、备份校验、日志暂停/历史上限。没有重跑产品 tests。

## 5 样式与资源

main.tsx 加载 MiSans 分片 CSS 和 openbox.css；CSS @font-face 引用 NotoColorEmoji flag 字体，文本 MiSans-VF/NotoEmoji/system-ui。包内 panel-background.jpg 或 CSS variable 提供背景；基础 sidebar=256px、14px/20px 字体，圆角/节点 min-width/透明度/模糊由 AppShell appearance 传入。dataset/body.theme-dark 驱动主题；有浅深色 token、页面覆盖/响应式规则，最终值不能只看文件头。

图标源为 Heroicons、包内 SVG/品牌旗帜、public Twemoji 等。找到 public/licenses/twemoji 的 NOTICE/LICENSE；没有核实 MiSans/Noto 独立完整许可凭据，不能声明 GPUI 再分发许可已完成。完整 TTF/OTF 与 Emoji 呈现留 P0-03/P1-03。Tooltip Portal 限宽 320px、Modal/Picker 层级/窄屏覆盖是 CSS 事实，不是视觉验收。

## 6 明确未知项

| 未知项 | 已确认消费事实 | 最小后续证据 |
| --- | --- | --- |
| failover 手动选择 | lanes.manual/阈值/恢复/稳定期可存；selectProxy 仅 group/name | P0-07/P4-03 隔离实例观察 lane/node 选择与自动切换交错、manual 是否禁止改写/重启恢复、now 所在层和写权；不从布尔值推断 |
| traffic 单位/累计或区间 | number；前端差分/截零 | P0-07 原始帧+接收时间+受控访问前后/重连/重启，区分 Bps、delta bytes、累计 bytes 与重置范围；不套标准 controller 语义 |
| DNS 热更 | rewriteNeedsRestart 区分 target/note 与 source/enable/add/delete；apply unknown | P0-07 固定版本隔离实例，保存/应用前后同域查询，分别证明重写/过滤及重启；Toast/pending 不证明内核接受 |
| DNS 查询记录 | rows/at/elapsed/source/via/self/queryLog.reason；无 DNS WS 通道 | P0-07 真实采集源、时间单位/缺口、关闭/失败/平台不可用；connections 不能当 query 流 |
| DNS flush | POST void，成功 Toast | P0-07 隔离实例证明实际缓存/实例范围和不支持错误；2xx 不足以证实 |
| 宽响应 | patchStorage/saveBackgroundImage/runUpdate/cancelUpdate/factoryReset/diagnostics/saveDnsFilter/applyDnsFilter 返回 unknown | owner 补实际响应再收窄，不自建假 diagnostics schema |
| 宽 DTO/字段 | profile/routing/diagnostic/config/device/node index signature；preview.usage、backup.nodes/panelSettings/backgroundImage 宽类型；Group type/mode 含 string | 完整 profile/备份/预览另补；未知字段不补默认值 |
| 时间/枚举/单位 | Subscription 时间 string/number、kernelStale boolean/number；memory.oslimit 不消费；Proxy type、DNS result/reason、update stage/platform 宽 string；lan unknown | 不猜秒/ms/大小写/空值；geoIp region/city 声明但 normalizer 不填 |
| 状态归属 | 无 epoch/revision/instance_id；metrics 任一 open；壳层 serviceAction 未检查 ok:false，Backend helper 检查 | 记录当前行为边界，不把设计 StateVersion/RuntimeCommand 投射成当前 DTO；本卡不修产品代码 |

完整 profile/备份、实际错误与四类流未新采样；已有测试/源码 fixture 可离线使用，不是运行能力证据。以上歧义列待验证即 P0-01 第三验收 PASS，不提前实施 P0-07。

## 7 视觉检索与 ACCEPTANCE 下一动作

先 `rg --files docs src/openbox`，再读取 docs/ui/visual-reference.md 和 src/openbox/design-qa.md，检查 magic/尺寸/hash，目视两张代表图。仓库 reference 41 PNG（Clash Verge 8、旧 Veyra 33）；Windows 同尺寸 pair 属于旧 Tauri，与当前 src/openbox 不同。task012-final-proxies 1462×1016 的标题/侧栏/颜色可目视确认，不能作本卡 PASS。

QA 指向本机 2026-10-02 DNS/Backend 21 历史图，文件存在。DNS local-top/local-filter-records 名叫 .png，实际 JPEG 1070×850；dark-upstream JPEG 600×500。QA 1280×720 是历史 viewport 记录，不能覆盖文件实测尺寸；没有完整 source commit/捕获命令/缩放绑定与同尺寸五类浅深色 pair。

| 代表历史图 | 来源/状态 | 实测文件尺寸 | 用途 |
| --- | --- | --- | --- |
| [旧 proxies light](../ui/reference/veyra/windows-light/task012-final-proxies-light-960.png) / [dark](../ui/reference/veyra/windows-dark/task012-final-proxies-dark-960.png) | TASK-012 Windows 旧出口组，仓库图 | 1462×1016 同尺寸 | 确认不同 UI，不计本卡 |
| [旧 subscriptions light](../ui/reference/veyra/windows-light/subscriptions.png) / [dark](../ui/reference/veyra/windows-dark/subscriptions.png) | visual-reference.md 旧 Tauri | 1985×1434 同尺寸 | 历史保留 |
| DNS local-filter-records.png | design-qa.md 2026-10-02；空表/过滤 pending/背景；本轮目视 | JPEG 1070×850 | OpenBox 历史参考，未新采集 |
| DNS dark-upstream.png | 同历史目录深色上游窗 | JPEG 600×500 | 不能与 light 拼同尺寸 pair |

本机目录 `/Users/lifeilin/.codex/visualizations/2026/10/02/01a0fb2a-a6f0-7c41-adbf-936f28103039/{dns-settings,backend-settings}/`。代表文件 hash/magic/尺寸见 [视觉清单](fixtures/p0-01/visual-inventory.json)。没有复制/改写原图。

当前 GUI inventory `cua.getState()` 成功，可见既有 OpenBox tabs；随后 `cua.getTab({url:'http://127.0.0.1:1420/openbox.html#/settings'}, {browser:'1'})` 被浏览器策略拒绝，理由为用户拒绝该访问请求。未走 alternate browser/CDP/终端浏览器绕过，未启动服务器或补远端采样。**本轮实际 UI 操作与新截图 NOT_RUN**，并非没有 GUI 工具。

精确下一步：在允许访问的既有 GUI/browser 会话核对 HEAD/远端版本；固定同一 1280×720 CSS viewport、记录 DPR/文件像素/浏览器版本。采用隔离本地 fixtures 或只读现有响应，阻断生产写入；主题/背景仅在隔离 fixture 内切换，不能 PATCH 生产 storage。分别捕获 light/dark 壳层+背景、代理节点、非写弹层、DNS 表（可真实空态）；登记 URL/分类/状态/操作或命令/源码 commit/数据来源/hash，检查无凭据/私有节点/IP。loading/empty/error/busy 可用隔离 fixture，标数据模拟边界；真实浏览器渲染/操作仍需证据，历史图片/源码阅读不能替代。齐全后逐项复核四验收，转 DONE 并重算直接依赖；本轮不开始其他 Task。

## 8 验证与验收

API/流/页面/设置身份、JSON 解析/脱敏/provenance/源码指纹、Markdown links/anchors、git diff --check 与新增文件 whitespace 为最小检查。没有产品代码变化，pnpm/cargo/build/test、内核/网络/权限 NOT_RUN。由 Codex /root 定向自查事实/计划、混合来源/外壳、视觉限制与 DAG，不声称独立 Review。实际检查计数见任务卡交付记录。

| 验收 | 结果 |
| --- | --- |
| 77 方法/4 通道均有去向，未使用与业务入口分开 | PASS |
| 同尺寸浅深色覆盖壳层/节点/弹层/DNS/背景 | NOT_RUN / ACCEPTANCE |
| failover/traffic/DNS 歧义列待验证，无猜测事实 | PASS |
| 可复用脱敏样本，不为采样改生产 | PASS（22 case；运行观测 0） |

DAG：仅 P0-02 DONE；P0-01 ACCEPTANCE。P0-03/04 等 P0-01；P0-07 等 P0-01+04；P1-01 等 P0-01+02，均 TODO，Ready Queue 空。macOS 1/61 DONE、59 TODO、1 ACCEPTANCE；Windows 7 DEFERRED；无实际 BLOCKED。

<a id="final-validation"></a>
### 最终验证结果

2026-10-03 Codex /root，预期为身份/样本/链接一致且无生产/产品改动；实际如下，均为静态定向检查，不包含产品测试：

| 检查与命令/操作 | 实际结果 |
| --- | --- |
| `git rev-parse HEAD`、`git branch --show-current`、`shasum -a 256` 三份 host 引用 | HEAD/分支/起始三个 SHA256 与输入一致 |
| `node /private/tmp/veyra-p0-01-inventory.mjs`：TypeScript AST 取 api properties、遍历导入 api 引用；可重跑最小计数见 fixture README | 77 properties、72 有业务引用、5 无业务；源码指纹 69。初次空 snapshot 无 project 的尝试失败，改用 openProjects=[tsconfig.json] 后成功；未安装依赖 |
| `python3 /private/tmp/veyra-p0-01-verify.py`：JSON、来源行/commit、嵌套 JSON/IP/URL/凭据字段、源码 hash、union/调用、RouteKey/导航/分支、方法与方案编号、任务依赖/状态、Markdown 本地链接/anchors、whitespace/保留 hash | 7 JSON，22 case，34 来源引用，69 指纹；4 流/6 页面/9 分类；5 Markdown 文件 112 本地 links/anchors；68 任务（1 DONE、1 ACCEPTANCE、59 TODO、7 DEFERRED），READY=0；3 保留 hash 一致。首次校验发现 memory fixture source 行号越界，改为实际 :52；随后通过 |
| `rg --files docs src/openbox`、`file -b`、SHA256、两张已有图 `view_image` | 仓库 41 reference PNG；本机 DNS/Backend 21 历史图存在；7 代表图 magic/hash 已登记；首次尺寸解析存在 JFIF density 被误当 pixels 的缺陷，见下方独立 Review 修复记录；历史用途限制保留 |
| `git diff --check`、新增文档/JSON 逐行 whitespace 检查 | PASS；无新增产品源码改动 |
| 既有 .gitignore/AGENTS/WORKFLOW 再次 SHA256 | 3/3 保持领取前原文；未提交/推送/发布 |

临时清点/校验/生成脚本与中间 JSON 放 /private/tmp，结束删除，不增加仓库框架。可重跑的 API/JSON 最小命令在 fixtures README；结构清单保留完整声明/引用，复核身份与语义时直接检查所列当前文件。脱敏自动断言之外，Codex /root 对 22 个 value 逐项检查了来源/合成边界。没有重新运行 pnpm/cargo/产品单元或真实内核测试。

#### 独立 Review 后的有界修复

独立 Review 指出视觉元数据的真实证据 bug：首次对 `file` 输出取第一个 `NxN`，把 JPEG 的 **JFIF density 1x1 误解析为 pixels**。因此 visual-inventory.json 三项写成 [1,1]，与 §7 实际尺寸不一致；首次检查未覆盖这项一致性，不能记为首次就通过。

本轮以 `sips -g pixelWidth -g pixelHeight -g format <file>` 读取实际图像属性，交叉核对 `file -b <file>` 的 PNG dimensions / JPEG baseline dimensions（排除 density），并重新计算 SHA256。三项 pixels 修复为：local-top.png 与 local-filter-records.png 均 1070×850，dark-upstream.png 为 600×500；format 均保留 JPEG。其余四张代表 PNG 的尺寸/format/hash 均与文件一致，七张 hash 全部一致；没有修改图片。

修复后重跑：7 JSON parse、22 cases、34 source refs、69 source hashes、5 Markdown 文件的 112 本地 links/anchors 与 git diff --check 均 PASS；七张图片尺寸经 sips/file 一致核对。仅修改 visual-inventory.json 和本记录；既有 .gitignore、AGENTS.md、DEVELOPMENT_WORKFLOW.md、产品源码及任务状态未改，未 commit/push。P0-01 仍 ACCEPTANCE，真实视觉验收 NOT_RUN，READY 队列仍为空。

<a id="authorization-attempt"></a>
### 追加授权后的取证尝试（2026-10-03）

用户已明确授权本机 OpenBox 与 openbox.disign.me 的只读取证，同时继续禁止任何生产写操作。恢复规范并核对 HEAD 后，实际重试 `cua.getTab({url:'http://127.0.0.1:1420/openbox.html#/settings'}, {browser:'1'})`；工具仍拒绝，原因是已保存的用户权限设置阻止该地址。独立尝试已获授权的远端 `cua.getTab({url:'https://openbox.disign.me/#/settings'}, {browser:'1'})` 也被拒绝，返回用户拒绝此次访问请求。文字授权与工具实际可用权限尚未一致。

没有通过 CLI、CDP、其他浏览器、curl 或间接请求实现被拒绝的相同访问，没有读取/导出浏览器 Cookie 或凭据。本轮无新 screenshot、HTTP 响应或 WebSocket 帧；没有 observed=true 新 fixture。历史访问失败与 JFIF 修复记录保持原样，不能将本次授权写成已完成取证。

P0-01 继续 ACCEPTANCE；视觉与运行观测 NOT_RUN；READY 空。解除条件为在工具访问权限界面允许这两个地址，并使正常工具调用实际成功；已请求用户解除工具层权限限制。解除后继续 §7 的 1280×720/DPR/版本/隔离只读视觉步骤，以及本轮授权的 GET/四类只读流采样。failover manual、traffic 单位/累计或区间、DNS 热更/flush 等未知项没有新实证结论。

此次恢复的定向复核：TypeScript AST 77 API 与已有清单一致；4 流/6 页面/9 设置身份一致；7 JSON、22 case、34 source refs、69 source hashes 与脱敏断言 PASS；7 张历史代表图用 sips/file 核对尺寸/format/hash PASS；5 Markdown 文件 113 本地 links/anchors（新增授权尝试链接，原 112）、68 Task DAG/READY 推导 PASS；git diff --check PASS；3 受保护文件 SHA256 与恢复前一致。仅恢复证据文档变化，不生成或修改产品代码/实际运行 fixture。

<a id="readonly-observation"></a>
### 明确授权 CLI 替代路径后的实际结果（2026-10-03）

用户随后明确授权本机 Playwright + curl/WebSocket 替代取证；此次依据新授权执行，不将此前工具拒绝记录改写为成功。HEAD 仍为 `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb`，环境 macOS 27.0 (26A428)/arm64、Node 22.22.0、已有 Playwright 1.62.1、curl 8.7.1。

本机 1420 起初无服务（curl exit 7）。用已有 `pnpm dev --host 127.0.0.1` 启动本次自有临时 Vite 8.2.2；`curl --request GET http://127.0.0.1:1420/openbox.html` 返回 HTML，title 为 Open-Box React、入口为 `/src/openbox/main.tsx`，与当前 `openbox.html`、源码入口一致。仅证明服务/入口身份，不冒充浏览器视觉验收。

计划使用全新 context、viewport 1280×720、deviceScaleFactor 1、serviceWorkers block、全部 API route fulfill 与 WebSocket mock，不连接远端。实际默认 Chromium 1234 未安装；改用已安装 1226 后在创建 context 前因 `bootstrap_check_in ... MachPortRendezvousServer: Permission denied (1100)`/SIGTRAP 退出。已有 WebKit 2287 也在创建 context 前 Abort trap/exit 134，详细原因未证实。没有安装浏览器、申请额外权限或修改 OS 策略。浏览器版本、实际 DPR、物理像素均未取得；**新截图 0，实际 UI 操作/加载/空/错误/忙碌与浅深色验收仍 NOT_RUN**。启动失败与命令见 [环境证据](evidence/p0-01/browser-attempt.json)。临时服务收尾停止，未改变生产配置。

只读响应：`python3 /private/tmp/p0-observe.py` 对用户列出的 13 个 endpoint 执行独立 `curl --silent --show-error --connect-timeout 5 --max-time 12 --request GET --header 'Accept: application/json' --write-out '\n%{http_code}\n%{content_type}' https://openbox.disign.me<endpoint>`，不跟随重定向、不传凭据。13/13 均 curl exit 0、HTTP **403**、`text/html; charset=UTF-8`；不能认定为业务 API 的 JSON 错误或推断服务版本。HTML 原文未保存，落盘仅 status/content-type、时间、字节长度与 bodyRemoved。见 [真实 HTTP 观测](fixtures/p0-01/observed-http.json)。

`node /private/tmp/p0-ws.mjs` 用 Node 原生 WebSocket 对 `wss://openbox.disign.me/api/controller-ws/{connections,logs,memory,traffic}`（logs 带 `?level=info`）各尝试读取 4 秒后主动 close，不发送业务帧；四通道均 handshake-or-transport-error，帧数均 0。Node 不公开此次握手 HTTP status，不能伪报 401/403。见 [真实 WS 尝试](fixtures/p0-01/observed-ws.json)。全部 17 新 case 均 observed=true，但只表示真实请求/失败观测，**没有业务 DTO、没有四类运行帧**；22 份已有离线消费样本继续 observed=false。响应仅内存处理，所有字符串值与动态/未知字段名删除，数组采样有界；未读取、导出或持久化 Cookie/Authorization/token。

实际对目标远端的方法汇总：HTTP GET **13**；HEAD **0**；WebSocket 握手 GET **4**（只读，业务 send 0）；POST/PUT/PATCH/DELETE **0**。浏览器在启动阶段失败，没有向远端发送页面请求；本机 HTML GET 只用于入口核对。CLI help 自带更新提示不属于 OpenBox 请求，未安装或更新依赖。未调用 latency/test/diagnostic/refresh/update/reset/import/export/service action/selectProxy，未 PATCH storage。

未知项无新增消除：profile/groups/subscriptions/proxies/rules/DNS records/config/controller version 因 403 未取到业务数据；traffic 无连续帧，单位及累计/区间仍未知；failover manual、DNS hot reload/flush 需后续经授权写验证，继续归 P0-07/P4 等。P0-01 三项 PASS、视觉一项 NOT_RUN，保持 **ACCEPTANCE**；68 Task DAG 仅 P0-02 DONE，READY=0，P0-03/04/P1-01 尚未解锁，P0-07 仍需 P0-04。

精确下一步：在允许浏览器正常创建子进程/Mach 服务的 macOS 执行环境运行现有本机页面（不需要生产写授权），先成功 `chromium.launch()` 和创建 fresh context；用上文 viewport/全 API+WS 拦截边界，分别加载 light/dark storage 合成响应，禁用 auto-connection-check。打开策略设置弹窗后关闭，切到 DNS 查询记录表，以延迟/空/503 fixture 验证 loading/empty/error/busy；捕获壳层+内置背景、节点、非写弹层、DNS 表的浅深色 1280×720 pair，记录实际 DPR、浏览器版本、物理尺寸和 SHA256，逐图脱敏检查。若远端继续 403/握手失败，保留本轮失败事实，不要求提取登录凭据；业务能力验证归后续任务。本次缺口是执行环境的浏览器启动，已获站点授权不再是待确认条件。

#### CLI 取证后的最终定向验证

`python3 /private/tmp/p0-final.py` 与 TypeScript API AST snippet：10 JSON parse（9 fixture/元数据 + 1 环境证据）、39 case（22 离线/17 observed 请求失败）、34 文件行来源引用、69 源码 SHA256、77 API AST identity、4 通道/6 页面/9 分类、5 Markdown 的 120 本地 links/anchors、68 Task 主表/卡依赖一致/无环/READY 推导、脱敏与 whitespace、git diff --check 全部 PASS。首次附加卡片检查把历史交付标题当任务定义而失败；限定有前置/依赖的卡正文后，68 张卡与主表逐项一致。最终状态 1 DONE、1 ACCEPTANCE、59 TODO、7 DEFERRED，READY=0。三份受保护文件 hash 保持原值；无产品源码 diff、无 commit/push。

7 张历史图经 sips pixelWidth/pixelHeight/format + file PNG/JPEG 图像 dimensions + SHA256 再核对 PASS；JFIF density 修复历史保留，新截图仍 0，不计视觉 PASS。没有执行产品构建/单元测试、内核或权限测试。真实远端 GET 与 WS 尝试已实际执行，不能继续统记“网络 NOT_RUN”。

新增证据 SHA256：observed-http.json `a07c0546a80c070a4eed3c4692bdecac66f6e8acc1094ae386606856d1ee835e`；observed-ws.json `10710776040bea1242b859860fd036a07e715b222a99bacf26e7d95db6ed61c6`；browser-attempt.json `ccb88ffbf4e15cd2d87792a1b2623a262e0648a2d3c2167b15b348581e26e515`。临时调查/校验脚本收尾删除，不增加通用测试框架；JSON parse/API AST 复现保留在 fixture README，实际 curl/WS 参数保留上文。

<a id="temporary-auth-observation"></a>
### 临时登录授权后的新增实证（2026-10-03）

用户新增仅 `POST /api/auth/login` 的一次登录例外，随后仅允许既定 GET/只读 WS。本轮从新建 `/private/tmp/veyra-p0-readonly.cookies`（umask 077）开始；未读取浏览器既有会话。临时鉴权输入只经进程内构造并通过 curl stdin 传递，不进入 argv、记录或仓库。此前无登录观测与失败记录保持历史，不改写为本次结果。

`python3 /private/tmp/p0-session-read.py`：先 GET auth/status，再 POST auth/login **1 次，无重试**，然后沿用同一临时 jar 请求原批准 13 个 GET。curl 使用 `--connect-timeout 5 --max-time 12 --request <method> --cookie <temporary-jar> --header 'Accept: application/json'`；登录额外 `--cookie-jar <temporary-jar> --header 'Content-Type: application/json' --data-binary @-`，不跟随重定向。15 请求均 exit 0、HTTP **403**、`text/html; charset=UTF-8`。登录没有成功，未建立已鉴权会话；随后请求只能标为“登录尝试后”，不能称“已认证响应”。因为 WS 结果不同，另以 `curl --http1.1 --request GET`（同超时与 Accept）对 auth/status 做一次无凭据只读确认，仍 403；仅记录固定 Cloudflare/Attention required 通用标记，不保存 HTML 内容或推断应用认证结果。未再次提交登录。见 [临时登录与 GET 结果](fixtures/p0-01/observed-session-http.json)，共 16 case。

`node /private/tmp/p0-session-ws.mjs`：使用已有 playwright-core utilsBundle 的 ws 客户端，仅在内存读取本次 jar 并构造会话 header；没有导出既有浏览器 Cookie、打印/保存请求 header。四个既定通道各采样上限 4 秒、handshake timeout 3.5 秒、不跟随重定向、不发业务帧，结束 close/terminate 本次连接。connections/logs/memory 返回 **401 application/json; charset=utf-8**，traffic 返回 **502 text/html**；均 handshake-rejected、frames=0。此次库暴露握手状态，因此新增状态可实证登记；不倒推此前 Node 原生 WS 失败的未知状态。见 [临时会话 WS 结果](fixtures/p0-01/observed-session-ws.json)，4 case。前置登录失败，不能把它们当成功认证后的控制器观测。

全部 20 新 case observed=true，表示实际请求失败观测，不表示取得业务 DTO/错误码或流帧。落盘只保留 endpoint/transport、时间、HTTP status/content-type、类型/长度/通用标记；响应原文只在进程内处理，字符串内容、真实域名/节点名/IP/进程路径/所有凭据一律不留。服务/控制器版本与响应 DTO 未核实；traffic 没有连续帧，单位/累计/区间仍 unknown。failover manual 与 DNS hot reload/flush 等未写验证，继续归后续 P0-07/P4/P5。

本机只做 **1 次**已有 Chromium 1226 启动确认（Playwright 1.62.1，`node /private/tmp/p0-browser-confirm.mjs`），仍因 MachPortRendezvousServer permission denied (1100) 在创建 context 前退出，没有页面请求或新截图，未修改系统权限/安装依赖。见 [浏览器单次确认](evidence/p0-01/browser-session-confirm.json)。实际浏览器版本/DPR/物理尺寸仍未知；视觉 NOT_RUN。

本轮远端方法：curl GET **15**（前置 auth/status 1 + 原 13 + HTTP/1.1 确认 1）；POST **1**，仅 `/api/auth/login`；WS 握手 GET **4**、业务 send **0**；HEAD/PUT/PATCH/DELETE **0**。加上此前已记录的 13 GET/4 WS，累计为 curl GET **28**、登录 POST **1**、WS 握手 GET **8**，其余写方法 0。未调用 select/test/refresh/update/reset/import/export/service action，不因登录例外扩大授权。

结束后删除临时 jar 与三份本轮临时鉴权/WS/浏览器脚本，并以路径不存在断言确认；原始响应只在内存，没有落盘原文需要保留。P0-01 仍三项 PASS、视觉一项 NOT_RUN，状态 **ACCEPTANCE**，READY=0；缺口仍是允许浏览器正常启动的执行环境，精确视觉步骤沿用上节。不要求重复提供凭据、不进行登录重试或额外权限变更。

新增证据 SHA256：observed-session-http.json `79bec9348b801da49c2e3068ad3270bd8795b18848991034d77d016afa9b891b`；observed-session-ws.json `a6a1b6db2454e2a75367db4b63eb74d93acc41990629365ca0eeabcabd35efd9`；browser-session-confirm.json `2d511adad4d84c1eaa72e2801d500ddbd3c8592e6c5444bad190e4110978d350`。

#### 临时鉴权尝试后的最终验证

`python3 /private/tmp/p0-session-check.py` 与现有 TypeScript API AST：**77 API、4 通道、6 页面、9 设置；13 JSON parse、59 case（22 离线/37 实际失败观测）、34 文件行 source refs、69 source hashes、7 历史图 sips/file 尺寸/format/SHA256、5 Markdown 的 126 本地 links/anchors、68 Task 卡/主表显式依赖一致与无环/READY 推导、脱敏/临时会话清理断言、git diff --check 均 PASS**。API 方法身份与既有清单逐项相同，69 源码指纹不变；没有修改产品源码或运行无关产品测试。

观测 transport 计数独立校验：累计 HTTP GET 28、HTTP POST 1（仅 auth/login）、WS read 8；本轮 GET 15/登录 POST 1/WS 4，与逐项 fixture 一致。所有 POST/PUT/PATCH/DELETE 中只有已授权 auth/login 一次；无登录重试。输入凭据的精确值未出现在本任务 docs/fixtures/evidence，未保存 Cookie/请求头/请求体/原始响应；临时 jar、鉴权及 WS 脚本均已删除。三份受保护文件原 SHA256 保持不变。检查脚本收尾删除，未 commit/push。新截图 0，视觉 NOT_RUN，P0-01 ACCEPTANCE，READY=0。

<a id="current-acceptance"></a>
## 当前最终验收（2026-10-03，Asia/Shanghai）

负责人 Codex /root。分支 `codex/dist-react-restore`、HEAD `7fab0e47d2a28446e131b7cd737ef4a5a43d55fb` 未变，产品源码 69 个既有指纹匹配；既有 dirty-tree 保留。仅更新本卡基线、状态/交接与 fixtures/evidence，不修改 .gitignore、AGENTS.md、DEVELOPMENT_WORKFLOW.md，不启动下游实现，不 commit/push/publish。当前会话创建时间 2026-10-03T10:55:05.468Z；官方工具确认标题 `1003 | 文档 | P0-01 最终基线验收`。

### 远端真实只读观测

前置 curl GET auth/status 返回 403 text/html；按 client 的 JSON、same-origin Origin、临时会话协议执行首次 login POST，仍 403 HTML。Node HTTPS 对同一 auth/status 返回 200 application/json，说明传输结果不同，不能将 curl 的 HTML 拒绝解释为密码错误或断言具体代理根因。基于该协议诊断仅重试 login 一次，200 且 authenticated=true；没有第三次尝试。随后用户列出的 13 GET 全部 200，其中 proxies 为 text/plain 但 body 可解析 JSON，其他为 JSON。当前全部普通 GET 为 **15：14 成功、1 失败**；登录 **2：1 成功、1 失败**。

四个真实 WS 均 101，connections/logs/memory/traffic 分别取得 **6/6/5/5 帧**，接收时间逐帧记录；只读订阅、无业务 send，达到帧数或 7.5 秒上限即主动关闭。控制器 version 字段中可识别的版本部分为 **1.14.1-openbox**，完整附加文本不保留；面板/服务发布版本 UNKNOWN。源自真实响应的脱敏文件为 [HTTP](fixtures/p0-01/observed-current-http.json) 和 [WS](fixtures/p0-01/observed-current-ws.json)，新增 18 case observed=true，原静态 case 仍 observed=false。

脱敏采用保守缩减：所有字符串换为固定 fixture/example/文档 IP，动态字典键匿名化，敏感字段删除，数组最多保留两个元素，普通非零数值只保留正负类型。traffic 单独保留每字段排序秩（不是实际使用量、单位或比例）和真实接收时间，保留上升/下降关系。样本只证明结构及所说明的变化模式，不能作为完整配置导入或字段枚举/数值语义依据。

traffic 单次不中断订阅内 up 变化符号为 −/+/+/+，down 为 −/−/+/+，因此不能支持“在此窗口单调累计”的假设。源码 useLiveMetrics 对帧差分、负差截零、首帧速率置零；previousTraffic 未在 socket 重连时主动清空。这是消费行为，不是服务单位证据。未做受控流量、重连或内核重启实验，**速率/区间字节、bytes 单位及重置范围仍 UNKNOWN**。failover manual 写权/自动切换、DNS 热更/flush/采集源真实性与平台能力继续留后续任务，不以读取成功当作已验证。

### 当前源码真实浏览器视觉

[visual-manifest](evidence/p0-01/visual-manifest.json) 登记 8 张 PNG、URL、状态、接收日期、源码 HEAD、SHA256、浏览器版本与尺寸。**Chromium 149.0.7827.22；CSS viewport 1280×720；devicePixelRatio=1；物理尺寸全部 1280×720。** 浅深色各有 shell/proxies/modal/dns，背景为当前包内背景；DNS 图滚动到真实空表，表头和空态完整。

浏览器运行当前 Vite `/openbox.html` → `src/openbox/main.tsx`，未替换 UI、样式或 DOM。全部 API route 本地 fulfill，全部 WS route mock 且不 connectToServer，禁止任何非 GET/HEAD 请求并记录为失败；外部 host 一律阻断。主题只来自隔离 storage fixture，自动站点测速关闭。数据见 [visual-data](fixtures/p0-01/visual-data.json)，明确 **真实浏览器渲染 + 模拟数据状态**，不代表 GPUI 或后续真实服务 UI 集成验收。

两种主题均实际完成六页面导航、九设置分类、代理卡片收起/展开、策略设置弹层打开/关闭、DNS 七列表头/空态；延迟 GET 与 503 fixture 验证 DNS loading/busy/error，不执行保存、测速、刷新生产数据或服务动作。最终 54 项操作/状态断言 PASS、JS runtime error 0、写入/未映射 API 违规 0。逐图查看无明显布局破坏或页面横向溢出；正常纵向滚动、设置分类横向滚动及节点名省略不当作 clipping。当前深色源样式存在浅侧栏/卡片低对比度，代理页残留分号；忠实记录现状，本任务没有产品修复范围。

初次 REPL 深色批次超时并重置；有界脚本再次发现同文档 hash 导航不会重新加载 theme storage，保留该次 FAIL。改用新文档后深色通过；这是采集脚本问题，未改产品实现。manifest 保留失败项并另标 final_result=PASS；不将旧失败改为从未发生。

### 四项验收与 DAG

| 任务卡验收 | 当前结果与证据 |
| --- | --- |
| 77 方法/4 通道去向、业务与未使用入口区分 | PASS；当前 AST 77/72/5，全部业务及测试引用文件和数量与 source-inventory 一致，四通道真实握手和帧补齐 |
| 同尺寸浅深色壳层/节点/弹层/DNS/背景 | PASS；8 PNG、54 实际操作/状态断言；来源、尺寸、DPR、版本和 SHA256 已登记 |
| failover/traffic/DNS 事实与未知分开 | PASS；traffic 仅缩小到非单调序列，单位/速率或区间/重置未知不猜测 |
| 脱敏离线样本、不修改生产配置 | PASS；22 静态 case + 55 observed case（含历史失败），另有隔离视觉数据；远端仅 2 登录 POST、15 GET、4 WS |

因此 P0-01 **DONE**。从 68 张卡的显式 DAG 重新推导：P0-03、P0-04、P1-01 READY；P0-07 因 P0-04 未 DONE 仍 TODO。macOS 2 DONE、3 READY、56 TODO、0 ACCEPTANCE；Windows 7 DEFERRED。本轮未实现任何 READY 任务。

### 请求审计、验证与清理

[request-audit](evidence/p0-01/request-audit.json) 逐项记录远端 method/host/path/purpose/status/timestamp：login POST 2，普通 GET 15，WS handshake GET 4，其他 POST/PUT/PATCH/DELETE 全 0，业务 WS send 0。视觉完整有界批次逐请求记录 mock/本机静态来源；初始 REPL 批次明细因重置未保留，最后检查违规为 0，拦截器没有任何 API 写透传分支，该限制在审计中明确记录，不伪造逐请求数量。

定向验证：TypeScript 7.0.2 AST、4/6/9 身份、全部 JSON、source 行号与 69 SHA256、严格脱敏值允许集、PNG magic/尺寸/DPR/hash、Markdown links/anchors、68 Task 主表/卡片依赖一致/无环/READY 推导、受保护文件 hash、git diff --check。验证工具首次使用不适用的 AST 导出和不同换行计数后已修正；均属临时验证脚本，无产品更改。最终计数见 [validation](evidence/p0-01/validation.json)。未执行无关 cargo/pnpm 构建或产品测试；审查为 Codex /root 自查，不声称独立 Review。

原始响应和帧仅在进程内，生成脱敏 fixture 后清空；Cookie jar 在首次登录失败后删除，成功会话仅在内存并已清空。最终 browser/context 已关闭，未发现本轮遗留浏览器；临时 Vite 自有会话已停止。临时脚本/校验文件在收尾删除，正式截图、脱敏 fixture、manifest 和审计保留。
