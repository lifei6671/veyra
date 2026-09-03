# Java Review Profile

适用于 `.java`；以 Maven/Gradle Wrapper、Module、Compiler Release、Annotation Processor、
Analyzer、Framework Lifecycle 和项目 Contract 为权威。

- Null/Object：Nullable Boundary、Unboxing、Framework-populated Field、`equals/hashCode/compareTo`、
  Mutable Key、Alias、Record/Sealed/Switch 与版本；
- Exceptions/Resources：Cause、Interrupt Preservation、Public Error、Try-with-resources、JDBC、
  Transaction/Rollback、Suppressed Exception 和 Cleanup Order；
- Concurrency：Safe Publication、Synchronization、Lock/volatile/Atomic Invariant、Executor/Future/
  CompletableFuture、Timeout、Cancellation、Shutdown、ThreadLocal Removal；
- Generics/Streams：Raw/Cast/Heap Pollution、Stream Side Effect/Reuse/Order/Close、Optional Semantics；
- API：Signature、Overload、Generic Bound、Checked Exception、Serialization、Reflection、Module/
  Service、Source/Binary Compatibility；
- Tests/Tools：覆盖 Null/Empty、Exception、Equality、Cleanup、Partial Failure 和确定性并发；只用
  项目 Wrapper、Configured Analyzer/Formatter/Test。

不要强加 Google Java Style、Optional、Stream、Null Annotation 或异常哲学。
