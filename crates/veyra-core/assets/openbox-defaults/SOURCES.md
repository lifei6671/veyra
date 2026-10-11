# OpenBox 内置默认快照

用户于 2026-10-11 指定当前 OpenBox 配置作为 Veyra 内置默认。配置以只读 GET 接口取得，未调用原版写入/刷新/运行接口。

- 来源：`http://192.168.1.6:3036/api/openbox/profile` 的 routing 字段与 `/api/openbox/groups` 的 groups 字段。
- 原版自带 defaults 接口也已只读保留供比较，但本轮选用用户指定的当前配置。
- `routing.json` 保留 12 个普通策略、1 个兜底、名称、图标、缩放、规则条件和默认出口；`display-order.json` 保留当前显示顺序，命中顺序仍由 policies 数组决定。
- 24 个规则集来自 `/api/openbox/rulesets/entries?tag=…&offset=…&limit=100`。全部分页验证 total/offset/条目数量，共 139986 条，未只截取首100条。每类条件单独成无头规则以保留 OR，域名/后缀/关键词/正则/IP均保留原值和顺序。
- 内联规则集结构依据 https://sing-box.sagernet.org/configuration/rule-set/ 。编译不联网、不生成下载任务或系统规则文件。
- 用户确认故障转移仅内置结构，不复制其私人订阅节点/凭据。主用引用本项目所有-自动、备用引用所有-手动，两者从正式订阅目录派生，不能承诺不同提供商的独立性。新安装无节点时该结构停用；有节点时恢复默认启用。
- Stable IDs 转为内置身份；策略 default_outbound 为配置意图，Controller 当前状态仍由正式 Runtime/Observation 读取，缺当前来源保持未知。两处恢复是独立事务；只恢复目标分流而默认出站不存在/停用时仅保存默认意图，不虚构该出口已生效。需在出站配置中提供对应结构后正式应用。
- 既有用户快照不因升级自动覆盖。恢复目标分流只替换 routing 及其显示排序；恢复出站只替换 groups 并沿用既有引用/pending 校验；保存成功不自动 Apply/Restart。
- 原接口未提供规则内容的上游许可元数据，不猜测上游仓库或重写来源。现有 OpenBox 同源字体/图标及其许可仍保留各原始 SOURCES。

规则集文件 SHA、分页数量与来源详见本目录 `manifest.json`；只读原件及失败读取证据保留于 task evidence `resumed-native/policy87-*`。完整原始 typed entries 保留为 `policy87-full-entries-*.json`，逐页 offset/count/total/原始响应 SHA 及聚合 entries SHA 见 `policy87-full-pagination-receipt.json`；第二次完整只读提取与随包每类有序值严格相等。
