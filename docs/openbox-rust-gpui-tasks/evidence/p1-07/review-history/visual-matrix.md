# 逐区域视觉审查

状态：**收敛已实施，整卡视觉验收未完成**。不提供整图相似度，也不声称达到 95%。React/GPUI 均 1280×720 logical content，Light/Dark，半透明 overlay 只辅助检查边界。GPUI 原图 2560×1504，裁去 64 physical pixels 原生标题栏再缩为 1280×720。原图保留；鼠标指示器造成的阴影不属于产品样式。

| 区域 / 状态 | React 事实与本轮结果 | 判定边界 |
|---|---|---|
| sidebar expanded/collapsed | 256/64；padding8；nav36、radius11、gap12、同源 Heroicons20 | 已修正 Kit rem 导致的 7px padding；两主题对应图齐全 |
| brand/collapse | Veyra 文本保留126.33×24 footprint；collapse36，原 SVG | 产品名称差异授权；字体、文字品牌本身不冒充 Open-Box 图形 |
| sidebar status | 六项、uptime、三个36动作；真实 unavailable/disabled | React fixture 0/0B 与 GPUI — 不做数值像素比较，不伪造 Runtime |
| Settings top nav | 9分类、48容器、32按钮、padding8/gap8/radius9、Heroicons18 | Panel 图/overlay 核对边界；系统字体导致文字宽度/字重不同 |
| Panel sections | general x264 y56 w1008；常规高200；latency/layout y264高288；sites y560 | 对应 Light overlay 主要卡片边缘重合；未当作全局通过 |
| CompactSetting | 40高、48列距、label500、行底线 | 两主题局部覆盖独立 |
| Select closed/open | 32/radius9；192/96/112；menu4/item32 | 鼠标与键盘操作已执行；文字和菜单颜色细节仍需最终逐状态复核 |
| Switch off/on | 40×24/thumb18/travel16 | 独立截图；React switch-toast 只比 Switch 区域，不能把不同 Toast 状态整图评分 |
| Button/IconButton/Input | 32高/radius9；input80/112，icon同源 | 禁用密码：当前没有访问密码服务。聚焦/hover/disabled/loading 的完整截图矩阵未完成 |
| Slider | 256×24、track12/thumb24、fill24 | 修正 fill 原12的明显差异；两个主题最后图为修正后版本 |
| Modal | 512×142/radius13，遮罩40%，surface75% | CSS 对底下整个场景的 backdrop blur 不能由当前 GPUI 元素 API 等价提供；没有冒用 window material blur 声称等价 |
| Toast | 380×50、top24/radius14、plain toast 无类型icon、22关闭按钮 | 已捕获真实保存通知；hover 暂停、动画、与 Modal 的层级仍未完成验收 |
| Tooltip | 共享组件覆写12/18、padding5/8、radius7、背景30394c；sidebar14/20 | GPUI 窗口定位与 CSS anchor/arrow 不完全一致；尚无完整 hover/focus 对照证据 |
| 图标选择器 | 64×32 trigger、256×320 popup、本地896目录、搜索/分类/选择 | 同源资源；选中定位与阴影已修；地区code/无结果内容等仍需完整状态核对 |
| Loading/Empty/Error | 未实现业务共用 unavailable，保存失败保留草稿与重试 | 单元测试保留；生产真实 loading/error/busy 本轮最终视觉矩阵未齐全 |

## 技术差异与未完成项的区分

- 字体：见 [font-differences.md](font-differences.md)。有具体 GPUI 文本系统与 CoreText 探针，允许保留系统 fallback 的依据；笔画/字宽/字重影响整个界面。
- 背景：同源 JPEG，默认背景按 cover 后 blur 缓存；受管资产使用既有 source-image blur 缓存，缩放顺序与 CSS 不完全等价。每卡 backdrop-filter/Modal 场景模糊仍有差异。不能将“当前未实现”一概写为平台永远不支持。
- 窗口：macOS 原生32 logical标题栏不参与对照。关闭/打开文件面板后曾出现截图停留旧帧，Raise 后立即更新；记录为可见性/激活待复核，没有伪称保存失败或已修复。
- Focus、Tooltip定位、Toast hover/stack、完整异常态覆盖仍是未完成工作，不属于已获豁免的技术差异。
- `comparison/manifest.json` 指向每个对应图和转换方式；此前 iteration 图仅作迭代记录。`*-overlay.png` 是50%叠图，不是 PASS 分数。
