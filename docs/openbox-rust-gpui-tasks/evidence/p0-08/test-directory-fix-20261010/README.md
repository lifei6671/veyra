# P0-08 合并后临时目录碰撞修复

首次合并主分支 `4e8f9cb92e93b40bfb3fc272d1382062d74dc6d6` 复验的更新测试7/8、exit101，取消测试在创建自己的临时目录时 `AlreadyExists`。原命令和原日志完整保留为 `initial-main-update-tests.json/.log`，没有改写为通过。

原因是同一PID内并发测试可读到同一个SystemTime时间戳，旧目录名因此重复。仅在 `#[cfg(test)]` 的 tests.rs 路径中加入 AtomicUsize 进程内递增序号，保留PID/时间戳用于跨进程区分。业务断言、下载/取消实现、签名和生产组件不变，无重试、sleep、新依赖或弱化测试。

修复后默认并发8项更新测试连续3轮各exit0，Core feature/all-target Clippy `-D warnings` exit0，fmt exit0；各原命令/退出码和日志保留。独立审查见 `REVIEW.md`。

`candidate-source-binding.json` 重新核对120项原构建输入与首次独立提交 `f0fb874805c95b251b1b90465a8336cf302902dd` 完全一致；之后只有cfg(test) tests.rs发生变化。原浏览器ZIP/原App/签名未重建或替换，不能把新测试源码冒充原包的构建输入。原实机验收对应上述初始源码身份，修复只改变测试夹具，保留ZIP原SHA256。主分支修复后的验证另记本地收口记录。
