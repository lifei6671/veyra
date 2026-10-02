# OpenBox 目标分流分类目录来源

- 对照日期：2026-10-02，原版设置 → 目标分流 → 站点集编辑。
- `geo-catalog.json` 保留原版 1899 项 geosite、260 项 geoip 目录及来源顺序。每行包含分类代码、简体中文、英文、繁体中文描述；带属性的分类可按原版规则从基础分类拼接描述。
- 数据来自页面实际加载的 [geo-catalog-DYpABpVm.js](https://openbox.disign.me/assets/geo-catalog-DYpABpVm.js)，SHA-256：`94a2eae60a811f59bf72cd724b527124b5e81ccaab8e32e81208c0e7c847261b`。
- 页面、规则转换和请求契约对照 [index-R7_omgFM.js](https://openbox.disign.me/assets/index-R7_omgFM.js) 的 `RoutingPoliciesCard`、`GeoCategorySelect`、`GeoEntriesDialog`、`OutboundPicker`，以及 `windowResizeState-DNdNlz3k.js` 的 API 函数。
- 分类目录仅用于选项及描述；规则条数和明细均读取后端 `/api/openbox/rulesets/entries` 或 `/preview`，不从目录伪造。搜索包含各语言描述，优先精确代码、属性、代码前缀和子串，保留当前选项并排除其他行已用的同类分类。
- 保存使用 `PUT /api/openbox/profile`，仅提交 `{ routing: 修改的部分 }`，以返回的 `profile` 更新界面。普通站点集归并六个字段；前置自定义规则保留逐行顺序和出口；备注使用 `规则类型:值` 键。安装包默认值来自 `/api/openbox/defaults/routing`。
