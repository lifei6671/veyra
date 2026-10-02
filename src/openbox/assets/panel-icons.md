# OpenBox 面板图标来源

- 来源：`https://openbox.disign.me/#/settings` → 面板设置 → 测试站点 → 图标菜单。
- 采集日期：2026-10-02。
- `panel-icons.json` 保存原版菜单的 896 个唯一图标代码、中文名称、分类和 SVG 资源。
- 内联 SVG 来自原版实际渲染的 DOM；原版 `/assets/*.svg` 图标以原始 SVG 内容打包成 data URI，保留原图形和配色。
- API 存储仍使用原版图标代码（如 `brand:baidu`、`brand:openai-light`、`JP`），资源内容只用于本地渲染。
- 品牌标识及图形归其原始权利人所有，沿用来源资源的授权条件。
