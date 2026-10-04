# Veyra desktop · P1-03

正式桌面入口，当前只交付本地壳、导航、会话输入与 Core 状态桥。所有业务页面明确显示后续 Task；没有 Runtime、托盘、HTTP、登录或网络入口。

```sh
MACOSX_DEPLOYMENT_TARGET=15.0 cargo +1.99.0 run -p veyra-desktop --locked --offline
```

沿用 GPUI Kit 0.7.0 → gpui-pre 0.3.7。默认在系统临时目录创建 `veyra-p1-03-<pid>/state.json`，正常退出后清除。本地验收可以显式设置 `VEYRA_P1_03_STATE_ROOT=/private/tmp/veyra-p1-03-<unique>`，此目录由验收者清理；禁止指向真实用户数据。这是 P1-03 临时位置，正式平台目录由 P1-05 定义。

- `AppView` 一次创建六个页面 Entity，持有导航、session theme、Core 只读快照与前台接收任务。
- `AppServices` 装配共享 gate/store 的 Snapshot/Profile/Selection 服务；仅暴露本卡实际需要的异步读取，Profile/Selection 没有 UI 写入口。一个 Tokio runtime、两个 worker、一个阻塞 worker；不创建第二 UI loop。
- 短本地快照读取在有界阻塞 worker 执行；类型化 `AppEvent` 经 Tokio channel 唤醒 GPUI foreground。请求 ID、页面 generation、捕获 epoch 通过才更新快照并 notify。
- 最新权威读取可以发现合法的新 epoch；更早请求或旧 epoch 的事件不能覆盖已采用状态。synthetic instance identity + generation + epoch 检查不代表真实 Runtime owner 已实现。
- 每页输入 Entity 持续存在；设置分类和 draft 不写 Core。`Pages<T>` 只将六槽持有表参数化以运行离线纯状态保留测试，生产始终为 `Entity<PageView>`，没有页面工厂/插件接口。
- Debug 按钮：`A500 / B50` 对真实快照读取的投递分别延迟 500/50ms；`Busy 3s` 有界延迟；`实例 evidence` 发送明确标记的 synthetic A/B 事件；`Entity 自检` 用真实 Entity 断言身份、filter、category/draft 保留，再还原原输入。release 不包含这些按钮或延迟逻辑。
- 关闭窗口或点击退出结束进程；正式关闭隐藏/托盘流程留 P1-06。

验证命令、截图、真实操作与边界见 [P1-03 evidence](../../docs/openbox-rust-gpui-tasks/evidence/p1-03/README.md)。纯测试不启动 child/网络；一个服务测试读写自有临时 StateStore 后清除。实际 GPUI 自动断言与纯测试分别记录。
