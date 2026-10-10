# P0-08 主分支最终独立 Review（2026-10-10）

**结论：PASS_FINAL_MAIN_REVALIDATION；无剩余可操作 Finding。** 最终合并代码已在主分支真实完成受影响验证，P0-08 DONE 与 READY仅P0-09一致。当前未提交改动只有三份任务文档的Git/复验收口信息；可继续已授权的纯文档/evidence收口，不需再改业务源码或扩大测试。

Reviewer实际只核读源码差异、只读Git身份、完整命令收据/原日志、当前任务文档和清理证据，仅新增本文件。没有亲自重跑 tests、build、Clippy、fmt、GUI、授权或托盘操作，没有执行Git写入、删除、系统设置或发布。实机结论仍由前轮用户实际授权/托盘操作与原图、运行身份和系统日志支持，不把本次审查冒充亲自实机验收。

## 合并身份与变更范围

- 初始任务提交：`f0fb874805c95b251b1b90465a8336cf302902dd`。
- 初次合并：`4e8f9cb92e93b40bfb3fc272d1382062d74dc6d6`。
- 测试目录修复提交：`8e7bfded9650157eed456aa26cb6ad237861adc6`。
- 当前实际主分支：`codex/dist-react-restore`，HEAD/最终代码合并 `1cd477eeaf0bd4f1371d0504b530f67aae1c6613`。

只读核验初始任务提交到当前HEAD的 `crates/` 与 `tools/` 工程差异，仅 `examples/p0_08/tests.rs` 的已审查原子序号修复。当前HEAD与修复分支的Core/工具源码差异检查exit0；正式Core、Desktop、Helper、`src-tauri`、workspace Cargo/lock相对开发基线的保护范围检查exit0。生产更新/打包契约和P2-05/P2-06公共边界未改变，不存在新代理体系、重复实现、弱化业务断言或错误安装成功反馈。

原浏览器ZIP和Native验收候选不重构、不重签、不替换；其输入身份绑定初始任务提交与原构建摘要，后续唯一输入变化是cfg(test) fixture。测试修复不能反向改写原包清单、原Native Review或旧FAIL。

## 实际主分支验证

本目录命令JSON都记录主项目 cwd、HEAD `1cd477ee…`、共享构建目录环境、实际退出码；原始日志与收据吻合，不以独立树结果替代主分支运行。

| 验证 | 核读到的实际结果 |
| --- | --- |
| 更新 example默认并发测试 | `update-tests.json/.log`：8 passed、0 failed、0 filtered，exit0 |
| 更新 example实际构建 | `update-build.json/.log`：exit0 |
| Core feature/all-target Clippy，`-D warnings` | `core-clippy.json/.log`：exit0 |
| 约定manifest fmt `--check` | `fmt.json/.log`：exit0 |
| 真实包正反/优化模式测试 | 主分支首次合并时 `../main-20261010/package-tests.json/.log`：8/8、exit0；打包源码及该测试在修复后未变，复用这一真实证据，不声称最新HEAD又重跑 |

首次主分支更新测试7 PASS/1 FAIL、exit101的 `AlreadyExists` 原日志保持在 `../main-20261010/update-tests.*`；后续明确以测试目录进程内原子序号修复，未串行化/删除断言或重写旧FAIL。旧Legacy Clippy Windows资源缺失exit101、GitHub远端无资产NOT_RUN、首轮Gatekeeper失败同样保留。

## 状态、清理与最后边界

独立解析任务表得到68张卡：DONE29 / ACCEPTANCE1 / DOING1 / READY1 / TODO29 / DEFERRED7；READY仅P0-09。SESSION、IMPLEMENTATION_PHASES和P0-08验收文档的当前结论、SHA、复验范围一致，仍明确P2-05 ACCEPTANCE/P2-06 DOING、远端下载NOT_RUN、实际用户操作及中间授权/托盘菜单截图限制。当前三份未提交文档没有夹入业务源码变化。

Reviewer重新读取既有主树缓存并计算摘要，与领取前记录相同（1个文件）；未清理该用户既有未跟踪内容。`empty-fixture-cleanup.json` 记录三个09:32/09:34早期P0-08命名空间空目录，在旧PID不存在、空且非symlink检查后按精确路径rmdir；失败日志和原App/ZIP继续保留。此记录支持任务临时夹具收尾，不扩展为任意系统目录或用户数据清理结论。`closeout.json` 的进程不存在、安全机制enabled与未发布边界和前轮证据一致。

本次只证明当前代码合并和上述主分支验证完成。最终纯文档/evidence提交SHA须由维护者完成后另记，Reviewer没有创造或预测该SHA。不得push、创建Release或执行正式发布；本卡PASS是本地Apple Silicon原型路线与具体候选的验收，不冒充正式升级/管理员helper安装或所有系统版本兼容性通过。
