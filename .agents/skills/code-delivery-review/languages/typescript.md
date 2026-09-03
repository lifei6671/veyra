# TypeScript Review Profile

适用于 `.ts/.tsx/.mts/.cts/.d.ts`，并同时加载 [JavaScript Profile](javascript.md)。以实际
`tsconfig` Project Graph、TypeScript Version、Package Export 和 Build/Test 入口为权威。

- Runtime Boundary：外部 `unknown` 必须按 Contract 验证；Annotation/Assertion/Generic/`.d.ts`
  不验证 JSON、Storage、Environment、DOM 或 Network Data；
- Type Safety：检查可触发问题的 `any`、Assertion、Non-null、Narrowing Across Callback/Await、
  Optional/Indexed Access、Overload 与 Runtime 实现；
- Union/Generic：Reachable Variant、Exhaustiveness Contract、Constraint、Variance、Conditional/
  Mapped Type、Exported Type、Enum/Discriminant/Optionality 和支持版本；
- Async：在 JavaScript Profile 基础上检查 Floating Promise、Thenable、Callback Return 和 Await
  后 Shared State；
- Tests/Tools：Runtime Test 证明行为；公共 Type Contract 变化补 Compile-time Test；只运行项目
  正确 Project 的 `tsc`、已配置 Typed Lint、Test 和 Declaration/API Check。

不要要求更严格 `tsconfig`，也不要把每个 `any`、Assertion、Non-null 或开放 Union 当缺陷。
