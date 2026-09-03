# `contract-data` Lane

**Activate**

- Public API、Protocol、Schema、Serialization、Configuration Contract 或 Consumer 行为变化；
- Persistent Data、Migration、Backfill、Version Coexistence 或生成契约变化；
- 内部改动会 material 影响外部调用方、持久状态或跨版本兼容。

**Inspect**

- 实现与声明、Consumer、Default、Optionality、Error、Versioning 的一致性；
- Schema/Migration 的 Rollout 顺序、Expand/Contract、Lock、Idempotency、Partial Failure 和 Recovery；
- 配置的 Parsing、Range、Precedence、Unknown Field、Environment 与 Secret 语义；
- 权威源与生成输出、依赖声明与锁定身份是否一致。

**Do NOT flag**

- 没有已声明 Consumer、持久数据或兼容要求的假想兼容层；
- 仅因 Lockfile 或生成文件行数多而逐行报告风格或实现问题；
- 未改变契约语义的内部重命名、机械格式变化或历史 Schema 债务。

**Escalation signals**

- Producer/Consumer、Schema/Serializer 或 source/generated output 出现可证实的不一致；
- Rollout 会造成新旧版本不可共存、不可逆数据损坏、长锁或无法恢复的 Partial Failure；
- 变更公共、持久化或配置边界，已超出当前授权或需要 Change Control。
