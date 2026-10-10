# P0-08 测试目录碰撞修复独立 Review（2026-10-10）

**结论：PASS；未发现剩余可操作 Finding。** 修改仅增加测试 fixture 的进程内原子序号，解决普通并行测试产生相同临时目录的真实失败，不改变下载/取消业务行为、断言、生产输入或已验收 App/ZIP。独立 Reviewer 只读核查 diff 与原始日志，仅新增本记录，没有重复运行测试、修改产品/系统/历史证据或执行 Git 写入。

主分支首次合并 `4e8f9cb92e93b40bfb3fc272d1382062d74dc6d6` 后的实际失败保留在主树 `evidence/p0-08/main-20261010/update-tests.json/.log`：默认并行执行8项，7 PASS / 1 FAIL，exit101；`cancellation_during_stalled_transfer_removes_only_owned_partial` 在 `tests.rs:125` 的目录 `create_dir` 因 `AlreadyExists` 失败，而不是下载/取消断言失败。原路径只使用 PID 与 `SystemTime::as_nanos`，同进程并行调用在相同系统时钟值下可产生同名目录。

最终 diff 仅涉及 `crates/veyra-core/examples/p0_08/tests.rs`：导入 `AtomicUsize/Ordering`；`directory()` 内新增一个共享 static 序号；路径后缀加入 `fetch_add(1, Ordering::Relaxed)` 返回值，并补中文原因注释。同进程所有调用共用该 static，原子递增给每次调用分配不同序号，时钟精度或线程交错不再影响路径互斥。这里仅分配编号，没有需同步的其他共享数据，Relaxed 足够。原 `create_dir().unwrap()` 明确失败语义与所有取消/清理/保旧业务断言均保留；无 sleep、retry、测试串行化、新依赖或生产 fallback。

## 修复后验证证据

| 证据 | 实际结果 |
| --- | --- |
| `update-tests-1.json/.log` | 默认并行8/8 PASS，exit0；无 `--test-threads=1` |
| `update-tests-2.json/.log` | 默认并行8/8 PASS，exit0 |
| `update-tests-3.json/.log` | 默认并行8/8 PASS，exit0 |
| `core-clippy.json/.log` | Core `--all-targets -D warnings`，exit0 |
| `fmt.json/.log` | 约定 manifest fmt `--check`，exit0 |
| Reviewer只读 `git diff --check` | exit0 |

## 产物与历史边界

当前独立树修复前 HEAD 为初始任务提交 `f0fb874805c95b251b1b90465a8336cf302902dd`。[candidate-source-binding.json](candidate-source-binding.json)记录120项原构建输入在该提交全部匹配。Reviewer重新核对当前旧构建清单的120项输入，仅 `examples/p0_08/tests.rs` 与旧记录不同；该文件通过 `#[cfg(test)]` 引入，更新原型生产源码及所有打包生产输入字节不变。旧构建输入/收据保留当时基线与未提交源码身份，不改写为新的修复输入。

Reviewer重新计算原浏览器 ZIP，仍为54,830,834 bytes、SHA-256 `fdb7472d4792bf4f7f88568f3c1f288746da5efcf55cdf0bded7b28262221a26`。已验收候选源代码由初始任务提交与原构建输入共同追溯，本次测试修复不重新构建/签名/替代该候选，也不重新声称其覆盖修改后的测试源码。原 Native Review、首开历史FAIL、GitHub远端下载NOT_RUN与Legacy Clippy既有FAIL均保持原样。

本记录仅支持该测试修复的独立 Review 通过。维护者仍需按已授权流程提交修复、合并并完成最新主分支受影响验证；首次主分支失败不能被覆盖成通过，也不能用独立树三轮通过冒充修复后的主分支结果。
