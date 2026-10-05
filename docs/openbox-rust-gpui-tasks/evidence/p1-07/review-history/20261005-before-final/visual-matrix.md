# P1-07 逐区域视觉及交互审查

**整卡 DOING；没有整图相似度 PASS/95% 声明。** 本轮三项 Host blocker 与整卡视觉一致性分别记录。对应 content1280×720，Light/Dark 显式偏好；GPUI2560×1504裁64physical标题栏。先嵌入ICC→sRGB，再缩放/叠图；原图保留。鼠标标记/原生标题栏/系统菜单/文件面板排除。

| 区域/状态 | 对应事实及实际结果 | 判定 |
|---|---|---|
| Shell theme | 新root明确保存Light/Dark，Core已load、AppView.dark=false/true、Kit同步；expanded/collapsed四图 | **Host theme blocker resolved**；旧Light=System→Dark采错，历史保留，未调主题颜色 |
| sidebar | expanded256/collapsed64；nav36、padding8、gap12、radius11，同源Heroicons20 | 两主题几何对应；不比较Overview业务卡片 |
| brand/collapse | Veyra126.33×24 footprint；collapse36/radius9/原SVG | Veyra命名授权；系统字体字形与React品牌图不同；hover/focus回归 |
| status/uptime/actions | 六统计、uptime、三个36动作 | Light surface不再深灰/inactive nav不是浅色Dark文字；GPUI unavailable/disabled保真，Reactfixture0/0B不作数值比较 |
| Settings top nav | 九分类48容器、32按钮、padding8/gap8/radius9、Heroicons18 | 结构/active主题对应，字体字宽字重差异保留 |
| Panel geometry | 双列48gap、section16padding、heading16/600/24、CompactSetting40、行gap4 | 基本卡片边缘对应；字体基线影响label/tooltip实际位置，不能用整图平均掩盖 |
| Select | 32/radius9，192/96/112宽；menu4/item32 | mouse open/select；Up/Down/Return、Escape、outside dismiss真实执行；两主题IP96展开图有效 |
| Switch | 40×24/thumb18/travel16，2px focus+2px offset | 两主题off/on/focus实际截图；React等controlled保存后aria=true再采On，旧错误On图历史保留 |
| Button/IconButton | 32/radius9；同源图标 | hover/focus/disabled截图；Save外层2px blue focus不再被Kit overflow裁切；mouse Save真实accepted+generic Notice。禁用密码/Runtime操作没有伪造服务 |
| Input | 80×32/112×32；Kit厚shadow取消，保留输入/IME | focus蓝2outline。数字输入浏览器UA spinner尚未迁移，不当作平台不可实现 |
| Slider | 256×24，track12、fill24、thumb24/border4，travel232 | 修复缺失Track/Indicator/Thumb事件bounds；实际Blur33→9+马上打开Select响应。三异步缓存回归通过 |
| Modal | 512×142/radius13、header41，遮罩40%/surface75% | 打开、Escape、focus return→Return再打开，两主题有效；局部backdrop blur当前GPUI元素API未等价 |
| generic Notice | 380×50/top24/radius14，close22，无success/error类型icon | 真实保存通知Light/Dark；close移除、4秒auto dismiss实际确认；不同文案长度不作像素评分 |
| Tooltip | IP280宽/12×18、padding5/8、radius7；help16；site350/14×19.6 | hover/keyboard/Escape/blur已执行；最后修复Local anchored流式原点重复加trigger高度，最后定位修复后截图目前PENDING（Mac锁屏）；tooltip测量仍是修复前真实截图，不冒称新版本。sidebar arrow仍未实现，独立未完成项 |
| local failure/invalid draft | timeout0 Save→Core LatencyTimeout失败、draft保留；abc→本地非法整数/Save disabled；6500恢复accepted | 使用已有受控路径，有实际截图/日志；未制造网络/Runtime错误 |
| Empty/Loading/Error | 占位真实unavailable；snapshot/save异步加载，失败草稿保留重试 | 不把占位当业务页完成；加载瞬态没有伪造可长期停留服务，不声称完整业务异常矩阵 |

## 具体差异

- MiSans unicode-range WOFF2可由CoreText解码，但GPUI注册分片/相同PostScript名合并不等价；许可未明确，不另行分发；系统fallback影响笔画、字重、字宽及说明浮层换行。NotoEmoji文本资源亦未等价，见`font-differences.md`。这与Heroicons同源几何无关。
- Card/Modal局部backdrop-filter是已有渲染能力差异；同源默认JPEG cover→blur缓存已后台化，受管source-image blur缩放顺序仍不同。没有称GPUI平台永久不能做blur，也没有用window material冒充CSS局部blur。
- Sidebar tooltip arrow、数字输入UA spinner属于尚未完成实现细节，不获技术豁免。文字导致的tooltip水平锚点/排版剩余误差须与字体差异分开测量，最终整体不设DONE。
- JPEG原RGB与ReactRGB直接比较会误报绿色偏差：按Display ICC转sRGB后Dark active约[118,215,165]对应React[118,216,165]，剩余1值是JPEG采样；见`shell-region-samples.json`。50%overlay不是评分。

## Evidence identity

- 首次fresh root显式主题四图及真人组合来自`bf6e6b3...`，状态日志固定保存；主Shell四图随后共享focus展示构建重采，来源另记录。
- Button/Nav/Tooltip焦点展示随后修复，组件原图逐次保留；最终`build-identity.json`与manifest原图SHA不冒称同一构建。
- `review-history/`保留原错误Light/DarkShell、旧manifest/矩阵、所有INCOMPLETE/FAIL；`interaction-results.json.history`不改写。
- `comparison/manifest.json`列明36组对应图片、content crop、颜色转换和排除范围；本矩阵按实现区域判断，不汇总全局PASS。
