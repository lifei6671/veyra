# Go Review Profile

适用于 `.go` 及影响 Go Build 的变更；以 `go.mod`、`go.work`、Build Tags、Toolchain 和仓库
命令为权威。

- Errors：检查丢失/覆盖 Error、`errors.Is/As` 身份、Typed-nil 和普通输入可触发的 Panic；
- Context/Concurrency：沿调用链追踪取消与 Deadline；Goroutine 必须有 Owner、退出、取消和
  Wait/Handoff；检查 Channel Close、Lock、Atomic、Race 和复制含 no-copy 状态；
- Resource/Data：关闭 Body/Rows/File/Pipe，检查 `Rows.Err`、Transaction Owner、Slice Alias、
  Map Concurrency、nil/empty Contract 和 Loop 中 Defer；
- API：检查 Exported Behavior、Zero Value Contract、Method Set、Receiver Copy 和 Caller；
- Tests：覆盖 Error Identity、Cancellation、Timeout、Partial Result、Cleanup 和并发终止；
- Tooling：只使用仓库采用的 Formatter、`go vet`/Lint、受影响 Package Test/Build；并发风险且
  项目支持时才运行 Race。

不要因 `defer`、Interface、Goroutine、nil Slice 或 Error 未包装本身创建 Finding；必须证明
可触发语义、生命周期或性能影响。
