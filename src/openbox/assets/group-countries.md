# OpenBox 自动分组地区规则来源

- 对照日期：2026-10-02，原版设置 → 出站节点 → 自动分组。
- `group-countries.json` 对应原版 52 项地区目录的代码、简体中文名称和完整关键词，保留来源顺序。
- 地区目录来自实际页面加载的 [windowResizeState-DNdNlz3k.js](https://openbox.disign.me/assets/windowResizeState-DNdNlz3k.js)（`Gs` / 导出 `iX`）；SHA-256：`c48d6d0d3cce1ae88583e8092e7e7d921054bb0337b67372edab076aaf81debf`。
- 匹配与生成行为来自 [index-R7_omgFM.js](https://openbox.disign.me/assets/index-R7_omgFM.js) 的 `NodeGroupsPanel`、`bx`、`B5`；SHA-256：`d96c6e1495827653645e78c09a8a4c30d0b8dad29e2b115f9e3abf26359f3ff1`。
- 国旗先转为两字母代码；两到三字母关键词按字母边界匹配，`cn` 单独排除 CN2（但允许 CN20）；其他关键词按子串匹配。地区计数与动态成员预览共用该规则，不将多地区命中强行改为互斥。
- 默认地区：HK、TW、SG、JP、KR、US。新增列表仅包含有节点且未选的地区，按节点数降序排列；用户拖动后的选择顺序参与生成。默认地区即使没有节点也可生成。
- 同名分组跳过；按地区图标归拢到已有首个位置，urltest 在 selector 前。新增组使用动态关键词和空 members，urltest 的 interval 为 `300s`、tolerance 为 `100`；不额外发送 enabled 或 iconScale。
- 原版保存为 `PUT /api/openbox/groups`，JSON 为 `{ groups }`，以返回的 `groups` 更新卡片，并显示 `dropped` / `dangling` 提示。
