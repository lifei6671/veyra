# Language Profile Routing

只根据 Delivery Unit 的实际变更加载适用 Profile。项目规则、冻结 Design/ADR、Compiler/Build
配置和 Task Contract 始终优先；Profile 只补充语言风险，不创建新 Style Policy、工具或依赖。

| Files | Profiles |
| --- | --- |
| `.go` | [Go](go.md) |
| `.rs` | [Rust](rust.md) |
| `.py`, `.pyi` | [Python](python.md) |
| `.js`, `.jsx`, `.mjs`, `.cjs` | [JavaScript](javascript.md) |
| `.ts`, `.tsx`, `.mts`, `.cts`, `.d.ts` | [TypeScript](typescript.md) + [JavaScript](javascript.md) |
| `.java` | [Java](java.md) |
| `.cs`, `.csx` | [C#](csharp.md) |
| `.c`, `.h`, `.cc`, `.cpp`, `.cxx`, `.hh`, `.hpp`, `.hxx` | [C/C++](cpp.md) |

Header、Module、Template、Generated Binding 和自定义扩展名按实际 Build Target 判断。配置或
构建文件改变某语言的 ABI、Feature、Dependency、Generated Output 或 Runtime Behavior 时，
即使没有对应源文件 Diff，也加载该语言 Profile。没有匹配项时继续使用通用 Review Lenses。
