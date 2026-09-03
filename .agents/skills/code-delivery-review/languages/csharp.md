# C# Review Profile

适用于 `.cs/.csx`；以实际 `.csproj`、Solution Entry、Directory Build Props/Targets、`global.json`、
Target Framework、Language/Nullable Setting、Analyzer 和 Host Lifecycle 为权威。

- Null/Value：Runtime Source 与 Nullable Annotation、Suppression/Cast、Await 后 Narrowing、
  Missing/Empty/Default、Equality/Hash 和 Mutable Key；
- Resource/Event：`IDisposable/IAsyncDisposable` Owner、`using/await using`、Borrowed Service、
  Stream/Response/Transaction/Registration 和 Event/Observable Unsubscribe；
- Async/Concurrency：Task/ValueTask Owner、Exception Observation、Sync-over-async、CancellationToken、
  Background Service/Channel/Timer Shutdown、Lock Across Await 和 Shared State；
- LINQ：Expensive/Stateful/Remote Source Re-enumeration、Deferred Query Escape、Ordering/Cardinality、
  Side Effect；
- Contract：Exception/Cancellation、Assembly API、Serialization、Default Parameter、Generic、
  Reflection/Trimming/AOT（仅声明部署适用时）；
- Tests/Tools：覆盖 Null、Dispose on Failure、Cancellation、Task Failure 和确定性 Lifecycle；只用
  仓库 Build/Test/API Compatibility/Analyzer。

不要机械要求 Dispose、`ConfigureAwait(false)`、CancellationToken 或禁止 `async void`；先确认
Owner、Host Model 和可触发风险。
