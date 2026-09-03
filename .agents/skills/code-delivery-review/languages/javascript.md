# JavaScript Review Profile

适用于 `.js/.jsx/.mjs/.cjs` 和 Task 内嵌可执行 JavaScript；以 Runtime、Module、Bundler、
Browser Support、Linter 和 Test Config 为权威。

- Async：Promise 必须 Await/Return/Aggregate 或显式交给失败 Owner；检查 Rejection、Out-of-order
  Result、Cancellation/Timeout/Cleanup 和 `Promise.all/race` 语义；
- Values/State：Missing/undefined/null/empty/zero/false/NaN、Coercion、`||` Default、Alias、
  Mutation、Closure Capture 和 Prototype/Dynamic Key Boundary；
- Browser/UI：Untrusted Data 到 HTML/URL/CSS/DOM Sink、Listener/Observer/Timer Cleanup、Stale
  Closure、Unmount 后更新和 Client-only Authorization；
- Runtime：ESM/CJS、Import Side Effect、Initialization Order、SSR/Worker Global、Stream/Socket/
  Child Process/Subscription Lifecycle；
- Tests/Tools：覆盖 Rejection、Falsey、Malformed External Data、Ordering、Cleanup 和 DOM Sink；
  只使用仓库 Script、ESLint、Formatter、Test Runner 和 Bundler。

不要机械要求 `===`、Await、Cleanup 或 Immutable；必须结合明确 Contract、Owner 和可观察后果。
