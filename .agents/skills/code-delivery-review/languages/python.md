# Python Review Profile

适用于 `.py`/`.pyi`；以 `pyproject.toml`、支持版本、Lock/Environment、Formatter、Linter、
Type Checker 和 Test Runner 配置为权威。

- Dynamic Semantics：`None`/Falsey、Identity/Equality、Mutable Default/Class State、Closure Late
  Binding、Alias、Iterator Exhaustion 和 Import Side Effect；
- Exceptions：Broad Catch、Cause/Contract、Retry Idempotency、`finally` Cleanup 和 State；
- Resources：Context Manager、Session/Transaction/Lock/Temp Resource 与 Partial Mutation；
- Async：Missing Await、Detached Task Exception、Cancellation、Blocking Event Loop、Async
  Context Manager/Iterator Cleanup；
- Compatibility：Signature、Default、Keyword、Return、Exception、Import Path、`.pyi` 与 Runtime，
  以及声明的 Python Version；
- Tests/Tools：覆盖 Falsey/Boundary/Exception/Cleanup/Async Termination；只运行项目已采用的
  Ruff/Black/mypy/pyright/pytest 或等价命令。

PEP 8、缺 Annotation、Dynamic Attribute 或 Broad Catch 不是自动 Finding；以项目 Contract 和
可触发行为为准。
