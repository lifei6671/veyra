# 流量统计与内存趋势图：需求变更核定

日期：2026-09-04。来源：用户要求实时网速仅指经过代理的网速，历史流量由 sing-box
提供，不保存到磁盘；需要趋势图展示一段时间内的流量变化。

状态更新：用户随后明确“就是singbox提供的聚合统计”，包含内核处理的 Direct。
以下问题与影响描述保留为核定前记录，不再阻挡实施。CHANGE-004 已同步 Requirement、
TASK-009 与 DCR-001；DCR-006 已独立审查通过，readiness-005 PASS。
当前实施与验证以 DCR-006、delivery-checkpoint-005 和 verification-005 为准；
checkpoint004 仍仅代表此前速率/累计算法修正。

## 已明确的要求

- 将上下行实时网速与累计上下行流量分别展示，数据源为 sing-box，客户端不采集系统网卡总流量。
- 累计值是核心提供的字节总量；客户端不将离散速率采样相加来冒充完整累计量。
- 统计与趋势数据仅在内存保存，不写 AppState、数据库、文件或浏览器本地存储。
- 增加上下行网速趋势图，横轴为时间、纵轴为速率；累计字节数保持独立数值展示。

## 可逆的默认展示方案

- 默认最近 60 秒，上下行两条曲线，窗口有界；按真实采样时间定位点。
- 未采到的数据表示缺口，不插入虚构流量；无有效样本时显示等待采样。
- 界面显示“本次内核运行累计”，避免把重启后重置的核心计数标为跨运行历史总量。
- 应用退出清空内存；核心实例切换时不跨实例连接趋势曲线或混合累计值。
- 图表不新增网络请求、采样频率、后台重连或第三方依赖。具体内存 owner 和窗口恢复
  行为须在受影响的观测设计中明确，不能由 UI 展示行为触发核心采样。

## 已解决的产品口径问题（历史记录）

“仅经过代理”是指所有经过 sing-box 的业务流量（包括规则命中 Direct 的直连），
还是只指最终经远端代理节点发出的流量（排除 Direct）？

当前固定 `/traffic` 和 `/connections` 总量来自核心同一 Manager.Total，并未按出口
类型排除 Direct。因此直接使用这两个聚合接口不能声称它们是“远端代理节点专用统计”。
若要求排除 Direct，需核定 sing-box 可用的分类累计能力，保持短连接/已关闭连接计数正确，
且不能将连接明细、目的地址或凭据暴露给 UI；轮询当前连接列表求和不能冒充完整累计量。

## 影响与当前边界

- 当前 `src/App.tsx` 只有即时速率和累计数字，没有趋势图。
- TASK-009 的 UI scope 目前仅限失败 Toast；本请求明确提出新的趋势图需求，需要同步
  Requirement、观测设计、Task scope 与 UI 验收。不是再次请求批准已经完成的 DCR-005。
- DCR-001 的最新安全 Snapshot/Delta 与禁止无界历史缓存约束仍适用；有界内存趋势的
  具体契约须纳入设计，不隐式新增公开 DTO 或后台机制。
- 当前已完成 checkpoint004 算法修正与 41 项测试保持其原有“核心总量”验证范围，
  不将其追认成 Direct 分类验证或趋势图证据。TASK-009 整体验收仍未通过。
- 此记录只核定变更和缺口，没有修改生产代码、采样行为、公共接口、依赖或磁盘存储。
  统计口径确定后，按用户已明确要求的功能范围更新事实源并完成设计/实现与验证。

## 一手依据

- [固定 traffic handler](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/experimental/clashapi/server.go)：来自 Manager.Total 的一秒窗口。
- [固定 connectionsSnapshot](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/experimental/clashapi/connections.go)：REST 累计量来自 Manager.Total。
- [固定 Manager.Total](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/common/trafficcontrol/manager.go)：汇总打开及已关闭连接，未按 Direct 出口过滤。
