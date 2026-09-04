# 固定核心空日志的实际响应

用户已批准修复正常空日志误判。首次方案把源码中的 `render.Status(...204)` 当作实际
响应状态，仅支持 204；真实回归证明该推断不成立，不能继续按这个假设交付。

`src-tauri/target/task009-remediation/real-log-framing.stderr.txt` 的固定核心诊断只记录
状态和布尔值：status=200、zero_content_length=true、body_empty=true、transfer_encoding=false。
没有记录凭据、原始头或载荷。该结果来自原真实受管 child、同一固定鉴权 Logs 请求。

固定版本源码的无日志分支只调用 render.Status 然后直接返回；render.Status 仅设置请求
context，实际写出状态发生在后续 responder，因而这条分支没有写出 204。

- [固定核心 getLogs](https://raw.githubusercontent.com/SagerNet/sing-box/v1.14.0/experimental/clashapi/server.go)
- [render.Status 与 NoContent](https://raw.githubusercontent.com/go-chi/render/v1.0.3/responder.go)

修正后的最小识别条件：仅对固定、携带当前实例鉴权的 Logs 请求接受 204，或 HTTP 200
且显式 Content-Length: 0、无 Transfer-Encoding、已收响应体为空。不接受一般 200，
不接受未声明长度/分块/非空响应；Traffic 200/204、401、其他状态和握手超时继续失败。
这仍是已授权的同一文件、同一正常空日志修复，不新增公开接口、请求、重试或降级。

对应测试需覆盖上述阳性和阴性，真实核心空日志/整体采样/停止必须重新通过。
