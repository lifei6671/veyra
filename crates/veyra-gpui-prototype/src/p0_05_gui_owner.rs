//! P0-05 验收专用：GUI executable 自己连接 helper，不导出生产协议或执行清理。
use serde_json::{Value, json};
use std::{
    io::{ErrorKind, Read, Write},
    os::unix::net::UnixStream,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};

const SOCKET: &str = "/Library/Application Support/VeyraP005/control.sock";
const MAX_FRAME: usize = 4096; // 与 helper 一致：payload 上限，不含 newline。
const REQUEST_BUDGET: Duration = Duration::from_secs(15);
type Result<T> = std::result::Result<T, &'static str>;

// 仅这三个固定请求；没有 path/service/config/command 输入或 Stop/Restore 分支。
#[derive(Clone, Copy)]
enum Request {
    Hello,
    StartSystemProxyTest,
    Status,
}
impl Request {
    fn frame(self) -> &'static [u8] {
        match self {
            Self::Hello => {
                br#"{"command":"Hello"}
"#
            }
            Self::StartSystemProxyTest => {
                br#"{"command":"StartSystemProxyTest"}
"#
            }
            Self::Status => {
                br#"{"command":"Status"}
"#
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Instance {
    child_pid: u32,
    mixed_port: u16,
    controller_port: u16,
}

fn parse_response(bytes: &[u8], gui_pid: u32, require_instance: bool) -> Result<Option<Instance>> {
    if bytes.len() > MAX_FRAME {
        return Err("response exceeds 4096 bytes");
    }
    let response: Value = serde_json::from_slice(bytes).map_err(|_| "invalid response JSON")?;
    if response["ok"] != true {
        // 不回显 helper error、config 或 readiness 原始日志。
        return Err("helper response ok is not true");
    }
    let status = &response["result"]["status"];
    if status["helper_euid"] != 0 || status["recovery_required"] != false {
        return Err("helper identity/recovery check failed");
    }
    let instance = &status["instance"];
    // 新实例 Hello 允许 null；Start 与 Status 必须有实例。缺失字段不等于 null。
    if instance.is_null() && status.get("instance").is_some() && !require_instance {
        return Ok(None);
    }
    if instance["owner"]["pid"] != gui_pid {
        return Err("instance owner PID does not match GUI PID");
    }
    if instance["readiness"]["outcome"] != "ready"
        || instance["readiness"]["config_input"]["config_transport"] != "anonymous_pipe_fd3"
    {
        return Err("instance readiness/FD3 check failed");
    }
    let child_pid = instance["child"]["pid"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
        .ok_or("invalid child PID")?;
    let port = |name: &str| {
        instance[name]
            .as_u64()
            .and_then(|n| u16::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or("invalid instance port")
    };
    Ok(Some(Instance {
        child_pid,
        mixed_port: port("mixed_port")?,
        controller_port: port("controller_port")?,
    }))
}

fn check_same(first: Instance, next: Instance) -> Result<()> {
    if first != next {
        return Err("child PID/ports changed across disconnects");
    }
    Ok(())
}

// stream 已 nonblocking；只用固定总预算，避免 peer close 后 set_read_timeout 的 EINVAL。
fn read_frame(reader: &mut impl Read, deadline: Instant) -> Result<Vec<u8>> {
    let mut frame = Vec::with_capacity(MAX_FRAME);
    let mut byte = [0];
    loop {
        if Instant::now() >= deadline {
            return Err("response deadline exceeded");
        }
        match reader.read(&mut byte) {
            Ok(0) => return Err("response closed before newline"),
            Ok(_) if byte[0] == b'\n' => return Ok(frame),
            Ok(_) if frame.len() == MAX_FRAME => return Err("response exceeds 4096 bytes"),
            Ok(_) => frame.push(byte[0]),
            Err(e) if e.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(5)),
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return Err("response read failed"),
        }
    }
}

fn request(command: Request, gui_pid: u32) -> Result<Option<Instance>> {
    // 每次独立 connect/write/read/drop；peer PID 来自这个 GUI 进程的 OS 身份。
    let deadline = Instant::now() + REQUEST_BUDGET;
    let mut stream = UnixStream::connect(SOCKET).map_err(|_| "fixed socket connect failed")?;
    stream
        .set_nonblocking(true)
        .map_err(|_| "socket nonblocking failed")?;
    let mut bytes = command.frame();
    while !bytes.is_empty() {
        if Instant::now() >= deadline {
            return Err("request deadline exceeded");
        }
        match stream.write(bytes) {
            Ok(0) => return Err("request write returned zero"),
            Ok(n) => bytes = &bytes[n..],
            Err(e) if e.kind() == ErrorKind::WouldBlock => thread::sleep(Duration::from_millis(5)),
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return Err("request write failed"),
        }
    }
    let frame = read_frame(&mut stream, deadline)?;
    parse_response(&frame, gui_pid, !matches!(command, Request::Hello))
}

fn run(gui_pid: u32) -> Result<Instance> {
    let hello = request(Request::Hello, gui_pid)?;
    let first = request(Request::StartSystemProxyTest, gui_pid)?.ok_or("start has no instance")?;
    if let Some(hello) = hello {
        check_same(hello, first)?;
    }
    let repeated =
        request(Request::StartSystemProxyTest, gui_pid)?.ok_or("start has no instance")?;
    check_same(first, repeated)?;
    for _ in 0..2 {
        thread::sleep(Duration::from_secs(2));
        let status = request(Request::Status, gui_pid)?.ok_or("status has no instance")?;
        check_same(first, status)?;
    }
    Ok(first)
}

fn ready_marker(gui_pid: u32, instance: Instance) -> Value {
    // 输出白名单固定且有界；绝不复制整个 readiness（含 child stdout/stderr）或 config。
    json!({
        "event": "p0_05_gui_owner_ready",
        "gui_pid": gui_pid,
        "child_pid": instance.child_pid,
        "mixed_port": instance.mixed_port,
        "controller_port": instance.controller_port,
        "transient_disconnects": 5,
        "same_instance": true,
        "helper_euid": 0,
        "recovery_required": false,
        "readiness": {"outcome": "ready", "config_input": {"config_transport": "anonymous_pipe_fd3"}}
    })
}

pub(super) fn start() -> Option<Receiver<&'static str>> {
    if std::env::var_os("VEYRA_P0_05_GUI_OWNER").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return None;
    }
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let gui_pid = std::process::id();
        let result = run(gui_pid).and_then(|instance| {
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "{}", ready_marker(gui_pid, instance))
                .map_err(|_| "ready marker write failed")?;
            stdout.flush().map_err(|_| "ready marker flush failed")
        });
        let status = match result {
            Ok(()) => "P0-05 GUI Owner: ready",
            Err(error) => {
                eprintln!(
                    "{}",
                    json!({"event":"p0_05_gui_owner_failure", "gui_pid":gui_pid, "error":error})
                );
                "P0-05 GUI Owner: failure (stderr)"
            }
        };
        let _ = sender.send(status);
        // 成功/失败都保留真实窗口，不做 privileged cleanup；由 Host 精确 SIGKILL GUI PID。
    });
    Some(receiver)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn response() -> Value {
        json!({"ok":true,"result":{"status":{
            "helper_euid":0,"recovery_required":false,
            "instance":{"owner":{"pid":42},"child":{"pid":84},
                "mixed_port":12345,"controller_port":23456,
                "readiness":{"outcome":"ready","config_input":{"config_transport":"anonymous_pipe_fd3"},
                    "stdout":"do not log this", "stderr":"secret"}, "config":"secret"}
        }}})
    }
    fn parse(value: &Value) -> Result<Option<Instance>> {
        parse_response(value.to_string().as_bytes(), 42, true)
    }

    // 保护真实 GUI peer 的身份及已就绪 FD3 实例契约。
    #[test]
    fn parses_ready_instance_and_allows_only_hello_without_instance() {
        let mut value = response();
        assert_eq!(
            parse(&value).unwrap().unwrap(),
            Instance {
                child_pid: 84,
                mixed_port: 12345,
                controller_port: 23456
            }
        );
        value["result"]["status"]["instance"] = Value::Null;
        assert_eq!(
            parse_response(value.to_string().as_bytes(), 42, false),
            Ok(None)
        );
        assert!(parse(&value).is_err());
        value["result"]["status"]
            .as_object_mut()
            .unwrap()
            .remove("instance");
        assert!(parse_response(value.to_string().as_bytes(), 42, false).is_err());
    }
    #[test]
    fn rejects_owner_pid_mismatch() {
        let mut value = response();
        value["result"]["status"]["instance"]["owner"]["pid"] = json!(43);
        assert_eq!(
            parse(&value),
            Err("instance owner PID does not match GUI PID")
        );
    }
    #[test]
    fn rejects_not_ready_and_wrong_transport() {
        for (field, bad) in [("outcome", "timeout"), ("config_transport", "disk")] {
            let mut value = response();
            let readiness = &mut value["result"]["status"]["instance"]["readiness"];
            if field == "outcome" {
                readiness[field] = json!(bad);
            } else {
                readiness["config_input"][field] = json!(bad);
            }
            assert!(parse(&value).is_err());
        }
    }
    #[test]
    fn rejects_failed_nonroot_recovery_and_invalid_identity() {
        for pointer in [
            "/ok",
            "/result/status/helper_euid",
            "/result/status/recovery_required",
            "/result/status/instance/child/pid",
            "/result/status/instance/mixed_port",
            "/result/status/instance/controller_port",
        ] {
            let mut value = response();
            *value.pointer_mut(pointer).unwrap() = Value::Null;
            assert!(parse(&value).is_err(), "{pointer}");
        }
    }
    // 保护每次连接关闭后，重复 Start/Status 仍指向同一个 child 与端口。
    #[test]
    fn repeated_responses_must_keep_same_instance() {
        let first = parse(&response()).unwrap().unwrap();
        assert_eq!(
            check_same(first, parse(&response()).unwrap().unwrap()),
            Ok(())
        );
        for pointer in [
            "/result/status/instance/child/pid",
            "/result/status/instance/mixed_port",
            "/result/status/instance/controller_port",
        ] {
            let mut value = response();
            *value.pointer_mut(pointer).unwrap() = json!(99);
            assert!(check_same(first, parse(&value).unwrap().unwrap()).is_err());
        }
    }
    // 保护拒绝畸形/超大响应，且必须用 newline 完整封帧。
    #[test]
    fn json_and_frame_limits() {
        assert!(parse_response(b"{", 42, true).is_err());
        assert!(parse_response(&vec![b' '; MAX_FRAME + 1], 42, true).is_err());
        let read = |bytes: Vec<u8>| {
            read_frame(
                &mut Cursor::new(bytes),
                Instant::now() + Duration::from_secs(1),
            )
        };
        let mut exact = vec![b' '; MAX_FRAME];
        exact.push(b'\n');
        assert_eq!(read(exact).unwrap().len(), MAX_FRAME);
        let mut oversized = vec![b' '; MAX_FRAME + 1];
        oversized.push(b'\n');
        assert_eq!(read(oversized), Err("response exceeds 4096 bytes"));
        assert_eq!(read(b"{}".to_vec()), Err("response closed before newline"));
    }
    // 保护 peer 已 close 的缓存帧，以及慢速/无 newline 不能重置总 deadline。
    #[test]
    fn closed_peer_and_total_deadline() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(b"{}\n").unwrap();
        drop(writer);
        reader.set_nonblocking(true).unwrap();
        assert_eq!(
            read_frame(&mut reader, Instant::now() + Duration::from_secs(1)).unwrap(),
            b"{}"
        );
        let (mut reader, _writer) = UnixStream::pair().unwrap();
        reader.set_nonblocking(true).unwrap();
        assert_eq!(
            read_frame(&mut reader, Instant::now() + Duration::from_millis(20)),
            Err("response deadline exceeded")
        );
    }
    // 保护 Desktop 可捕获的一行有界 marker；只输出白名单，不传播 config/secret/raw logs。
    #[test]
    fn marker_is_bounded_and_redacted() {
        let marker = ready_marker(42, parse(&response()).unwrap().unwrap()).to_string();
        assert!(marker.len() < 1024);
        assert!(!marker.contains('\n'));
        for forbidden in ["secret", "stdout", "stderr", "do not log"] {
            assert!(!marker.contains(forbidden));
        }
        assert_eq!(
            serde_json::from_str::<Value>(&marker).unwrap()["transient_disconnects"],
            5
        );
    }
    #[test]
    fn requests_are_fixed_enum_frames() {
        for (command, name) in [
            (Request::Hello, "Hello"),
            (Request::StartSystemProxyTest, "StartSystemProxyTest"),
            (Request::Status, "Status"),
        ] {
            let value: Value = serde_json::from_slice(command.frame()).unwrap();
            assert_eq!(value, json!({"command":name}));
            assert!(command.frame().ends_with(b"\n"));
        }
    }
}
