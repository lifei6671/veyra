# C 与 C++ Review Profile

适用于 C/C++ Source/Header/Module 及改变 Compile、Link、ABI、Sanitizer 的 Build 变更；以实际
Target、Compiler、Standard、Platform、Definition、Feature、Warning 和 ABI Contract 为权威。

- Ownership/Lifetime：Allocation、Handle、Pointer/View/Iterator/Reference/Callback Owner，RAII 或
  C Cleanup、Copy/Move/Destructor、Escape、Use-after-move、Double Release 和 Partial Construction；
- UB/Bounds：Buffer/Index/Pointer Arithmetic、Size Overflow、Signed Overflow、Narrowing、Shift、
  Alignment、Initialization、Aliasing、Union/Cast、Iterator Invalidation；
- Failure Safety：Exception/Status/Error Contract、Invariant、Cleanup、Destructor 和 C/FFI Boundary；
- Concurrency：Shared State、Lock/Reentrancy/Condition Predicate、Atomic Protocol/Memory Order、
  Thread/Queue/Callback Shutdown 和 Destruction Order；
- API/ABI：Public Header、Layout、Calling Convention、Visibility、Virtual/Inline/Template/Enum、
  Packing、Macro/Conditional Compilation 和 Interop Width/Encoding/Lifetime；
- Tests/Tools：覆盖 Boundary Size、Failure Injection、Ownership Transfer、Platform/Feature/Build Mode；
  只使用项目 Compiler Warning、Analyzer、Sanitizer、Formatter、ABI/Fuzz/Test 流程。

Raw Pointer、Cast、Macro、`goto` Cleanup、Manual Allocation 或 `unsafe`-looking Syntax 不是自动
缺陷；必须证明 Lifetime、UB、ABI 或其他不变量被破坏。
