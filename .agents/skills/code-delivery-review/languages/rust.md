# Rust Review Profile

适用于 `.rs`、Rust Build/Feature/Target 与 FFI 变更；以 Cargo Workspace、Pinned Toolchain、
Edition、Feature 和 Target 配置为权威。

- Ownership：检查 Move/Borrow/Clone、Stored Reference、Pin、Interior Mutability、Thread/Async
  Boundary、Invalid State、Integer Conversion、Index 和 UTF-8 Boundary；
- Unsafe/FFI：逐个验证 Safety Invariant、Aliasing、Provenance、Alignment、Initialization、
  Lifetime、Drop、Send/Sync、ABI、Ownership Transfer、Nullability、Buffer 和 Unwind；
- Error/Panic：普通外部输入能否触发 `unwrap`/Index/Panic，Error Conversion 是否丢失区分；
- Concurrency/Async：Lock Across Await、Atomic Ordering、Detached Task、JoinHandle、Cancellation
  Safety、Channel Lifetime 和 Shutdown；
- API：Public Item、Trait、Enum Variant、Blanket Impl、Feature Flag、Generic Bound、MSRV/Edition；
- Tooling：只运行仓库采用的 `fmt/check/test/clippy` 及相关 Feature/Target；Miri/Sanitizer/Fuzz/
  Loom 仅在项目已有且风险适用时使用。

`clone`、Allocation、Dynamic Dispatch、Interior Mutability、`unsafe` 或 `unwrap` 不因语法本身
成为缺陷；必须证明不变量被破坏或存在可信影响。
