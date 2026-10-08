//! P0-05 only. Fixed privileged resources and a closed, bounded IPC protocol.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
mod config_pipe;
mod probe;
mod readiness;

const ROOT: &str = "/Library/Application Support/VeyraP005";
// 传统 LaunchDaemon 使用本原型专用目录，不进入 Service Management helper 目录。
const HELPER: &str = "/Library/Application Support/VeyraP005/helper";
const PLIST: &str = "/Library/LaunchDaemons/com.lifei6671.veyra.p005.plist";
const LABEL: &str = "com.lifei6671.veyra.p005";
const KERNEL_HASH: &str = "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4";
const MAX: usize = 4096;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

// 只在父进程添加文字上下文，保留原错误的显示（包括 errno）。
fn at_stage<T, E: std::fmt::Display>(stage: &str, result: std::result::Result<T, E>) -> Result<T> {
    result.map_err(|error| format!("{stage}: {error}").into())
}

#[derive(Debug)]
struct ClientError {
    phase: &'static str,
    error: Box<dyn std::error::Error>,
}
impl ClientError {
    fn new(phase: &'static str, error: impl Into<Box<dyn std::error::Error>>) -> Self {
        Self {
            phase,
            error: error.into(),
        }
    }
}
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "client.{}: {}", self.phase, self.error)
    }
}
impl std::error::Error for ClientError {}
type ClientResult<T> = std::result::Result<T, ClientError>;

#[derive(Clone, Copy, Debug)]
#[repr(i32)]
enum PreExecStage {
    Dup2 = 1,
    Fcntl,
    Setsid,
    Setgroups,
    Setgid,
    Setuid,
}
const PRE_EXEC_BASE: i32 = 0x50000;
const PRE_EXEC_STRIDE: i32 = 0x1000;
impl PreExecStage {
    fn name(self) -> &'static str {
        match self {
            Self::Dup2 => "dup2",
            Self::Fcntl => "fcntl",
            Self::Setsid => "setsid",
            Self::Setgroups => "setgroups",
            Self::Setgid => "setgid",
            Self::Setuid => "setuid",
        }
    }
}
fn pre_exec_error(stage: PreExecStage, errno: i32) -> std::io::Error {
    // Command 的 fork/exec 错误通道只传 raw_os_error，不传任意 Error 字符串。
    // macOS errno 小于 stride；固定整数编码无分配、无锁，不改变 syscall/失败次序。
    std::io::Error::from_raw_os_error(PRE_EXEC_BASE + stage as i32 * PRE_EXEC_STRIDE + errno)
}
fn decode_pre_exec_error(code: i32) -> Option<(PreExecStage, i32)> {
    let offset = code.checked_sub(PRE_EXEC_BASE)?;
    let stage = match offset / PRE_EXEC_STRIDE {
        1 => PreExecStage::Dup2,
        2 => PreExecStage::Fcntl,
        3 => PreExecStage::Setsid,
        4 => PreExecStage::Setgroups,
        5 => PreExecStage::Setgid,
        6 => PreExecStage::Setuid,
        _ => return None,
    };
    Some((stage, offset % PRE_EXEC_STRIDE))
}
fn spawn_error(stage: &str, error: std::io::Error) -> Box<dyn std::error::Error> {
    match error.raw_os_error().and_then(decode_pre_exec_error) {
        Some((syscall, errno)) => format!(
            "{stage}.pre_exec.{}: {}",
            syscall.name(),
            std::io::Error::from_raw_os_error(errno)
        )
        .into(),
        None => format!("{stage}: {error}").into(),
    }
}
unsafe fn drop_credentials(uid: u32, gid: u32) -> std::io::Result<()> {
    // 与 child、root harness client、卸载 client 共用同一降权顺序，不绕过失败。
    if unsafe { libc::setgroups(0, std::ptr::null()) } != 0 {
        return Err(pre_exec_error(PreExecStage::Setgroups, unsafe {
            *libc::__error()
        }));
    }
    if unsafe { libc::setgid(gid) } != 0 {
        return Err(pre_exec_error(PreExecStage::Setgid, unsafe {
            *libc::__error()
        }));
    }
    if unsafe { libc::setuid(uid) } != 0 {
        return Err(pre_exec_error(PreExecStage::Setuid, unsafe {
            *libc::__error()
        }));
    }
    Ok(())
}

unsafe extern "C" {
    fn p005_network(
        op: *const libc::c_char,
        id: *const libc::c_char,
        json: *const libc::c_char,
    ) -> *mut libc::c_char;
    fn p005_process(pid: libc::c_int) -> *mut libc::c_char;
}
fn native_value(raw: *mut libc::c_char) -> Result<Value> {
    if raw.is_null() {
        return Err("native adapter returned null".into());
    }
    let bytes = unsafe { CStr::from_ptr(raw).to_bytes().to_vec() };
    unsafe { libc::free(raw.cast()) };
    let value: Value = serde_json::from_slice(&bytes)?;
    if value.get("error").is_some() {
        return Err(value.to_string().into());
    }
    Ok(value)
}
fn network(op: &str, id: &str, config: &Value) -> Result<Value> {
    at_stage(
        &format!("SystemConfiguration.{op}"),
        (|| -> Result<Value> {
            let op = CString::new(op)?;
            let id = CString::new(id)?;
            let config = CString::new(config.to_string())?;
            native_value(unsafe { p005_network(op.as_ptr(), id.as_ptr(), config.as_ptr()) })
        })(),
    )
}
fn process(pid: i32) -> Result<Value> {
    at_stage(
        "process.proc_pidinfo",
        native_value(unsafe { p005_process(pid) }),
    )
}
fn path(name: &str) -> String {
    format!("{ROOT}/{name}")
}
fn hash(file: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(file)?)))
}
fn copy_installed(source: &Path, destination: &str) -> Result<()> {
    // macOS fs::copy 可能保留 staging 的 owner；由安装进程新建目标，
    // 仅复制字节，让目标归当前 root 安装进程所有，并拒绝覆盖/symlink。
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)?;
    std::io::copy(&mut input, &mut output)?;
    output.sync_all()?;
    Ok(())
}
fn save(name: &str, value: &Value) -> Result<()> {
    let temporary = path(&format!("{name}.new"));
    let mut file = at_stage(
        "save.open_temporary",
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary),
    )?;
    at_stage("save.write", file.write_all(value.to_string().as_bytes()))?;
    at_stage("save.sync_file", file.sync_all())?;
    at_stage("save.rename", fs::rename(temporary, path(name)))?;
    let directory = at_stage("save.open_directory", File::open(ROOT))?;
    at_stage("save.sync_directory", directory.sync_all())?;
    Ok(())
}
fn load(name: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&fs::read(path(name))?)?)
}
fn root() -> Result<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(
            "AdministratorRequired: effective UID is not root; no privileged mutation".into(),
        );
    }
    Ok(())
}
fn protected(file: &str, directory: bool) -> Result<()> {
    let meta = fs::symlink_metadata(file)?;
    if !protected_identity(meta.uid(), meta.mode(), directory) {
        return Err(format!("unprotected prototype path: {file}").into());
    }
    Ok(())
}
fn protected_identity(uid: u32, mode: u32, directory: bool) -> bool {
    let kind = if directory {
        u32::from(libc::S_IFDIR)
    } else {
        u32::from(libc::S_IFREG)
    };
    uid == 0 && mode & 0o022 == 0 && mode & u32::from(libc::S_IFMT) == kind
}
fn resource_exists(file: &str) -> Result<bool> {
    // exists() 会漏掉悬空 symlink；安装与清理不能把它视作空路径。
    match fs::symlink_metadata(file) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn protected_root() -> Result<()> {
    for directory in ["/Library", "/Library/Application Support", ROOT] {
        protected(directory, true)?;
    }
    Ok(())
}
fn daemon_plist() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>{LABEL}</string><key>ProgramArguments</key><array><string>{HELPER}</string><string>daemon</string></array><key>RunAtLoad</key><true/><key>StandardOutPath</key><string>{ROOT}/daemon.log</string><key>StandardErrorPath</key><string>{ROOT}/daemon.log</string><key>ExitTimeOut</key><integer>10</integer></dict></plist>"
    )
}
fn cleanup_paths() -> [(String, bool); 8] {
    [
        (HELPER.to_owned(), false),
        (path("sing-box"), false),
        (path("runtime"), true),
        (path("authorization.json"), false),
        (path("service-id"), false),
        (path("recovery.json"), false),
        (path("daemon.log"), false),
        (path("runtime/config.json"), false),
    ]
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    command: Operation,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum Operation {
    Hello,
    Status,
    StartSystemProxyTest,
    Stop,
    Restore,
    ShutdownForUninstall,
}
#[derive(Clone, Copy, Debug, Serialize)]
struct Peer {
    uid: u32,
    gid: u32,
    pid: i32,
}
fn peer(stream: &UnixStream) -> Result<Peer> {
    let mut p = Peer {
        uid: 0,
        gid: 0,
        pid: 0,
    };
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut p.uid, &mut p.gid) } != 0 {
        return at_stage("getpeereid", Err(std::io::Error::last_os_error()));
    }
    let mut length = std::mem::size_of::<i32>() as libc::socklen_t;
    // macOS sys/un.h: SOL_LOCAL = 0, LOCAL_PEERPID = 0x002.
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            0,
            2,
            (&mut p.pid as *mut i32).cast(),
            &mut length,
        )
    } != 0
    {
        return at_stage(
            "getsockopt LOCAL_PEERPID",
            Err(std::io::Error::last_os_error()),
        );
    }
    if p.pid <= 0 {
        return Err("LOCAL_PEERPID returned non-positive PID".into());
    }
    Ok(p)
}
fn connect() -> ClientResult<UnixStream> {
    let stream = (|| -> Result<UnixStream> {
        at_stage("protected_root", protected_root())?;
        let meta = at_stage(
            "socket_metadata",
            fs::symlink_metadata(path("control.sock")),
        )?;
        let settings = at_stage("authorization", load_public_settings())?;
        if meta.uid() != settings.0 || meta.mode() & 0o777 != 0o600 {
            return Err("socket ownership/mode mismatch".into());
        }
        at_stage(
            "UnixStream::connect",
            UnixStream::connect(path("control.sock")),
        )
    })()
    .map_err(|e| ClientError::new("connect", e))?;
    let p = peer(&stream).map_err(|e| ClientError::new("server_peer", e))?;
    if p.uid != 0 {
        return Err(ClientError::new("server_peer", "server peer is not root"));
    }
    // 读取只由 read_frame 的总 deadline 管理，不设置 SO_RCVTIMEO。
    at_stage(
        "set_write_timeout",
        stream.set_write_timeout(Some(Duration::from_secs(2))),
    )
    .map_err(|e| ClientError::new("connect", e))?;
    // 保留 request 原有的第二次 OS peer 检查，错误仍归 server_peer。
    Ok(stream)
}
fn load_public_settings() -> Result<(u32, u32)> {
    protected(&path("authorization.json"), false)?;
    let value = load("authorization.json")?;
    Ok((
        value["uid"].as_u64().ok_or("missing authorized UID")? as u32,
        value["gid"].as_u64().ok_or("missing authorized GID")? as u32,
    ))
}
fn read_with_deadline(stream: &UnixStream, buffer: &mut [u8], deadline: Instant) -> Result<usize> {
    // 只借用 stream 持有的 fd，不复制、不关闭，也不改变 socket 的 blocking/timeout 设置。
    let mut fd = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("MessageDeadlineExceeded".into());
        }
        // poll 使用整数毫秒；正的不足 1ms 向上取整，且不能溢出成负数（无限等待）。
        let timeout = remaining
            .as_nanos()
            .div_ceil(1_000_000)
            .min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut fd, 1, timeout) };
        if ready < 0 {
            let error = std::io::Error::last_os_error();
            if error.kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return at_stage("poll", Err(error));
        }
        if Instant::now() >= deadline {
            return Err("MessageDeadlineExceeded".into());
        }
        if ready == 0 {
            continue;
        }
        if fd.revents & libc::POLLNVAL != 0 {
            return at_stage("poll", Err(std::io::Error::from_raw_os_error(libc::EBADF)));
        }
        // HUP/ERR 也先读：peer close 后仍可能有完整 frame 留在内核缓存，不能直接判 EOF。
        // 单次 MSG_DONTWAIT 防止 readiness 失效后的读取阻塞；不使用 MSG_WAITALL。
        let count = unsafe {
            libc::recv(
                fd.fd,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                libc::MSG_DONTWAIT,
            )
        };
        if count < 0 {
            let error = std::io::Error::last_os_error();
            if matches!(
                error.kind(),
                std::io::ErrorKind::Interrupted | std::io::ErrorKind::WouldBlock
            ) {
                continue;
            }
            return at_stage("recv", Err(error));
        }
        if Instant::now() >= deadline {
            return Err("MessageDeadlineExceeded".into());
        }
        return Ok(count as usize);
    }
}
fn read_frame(stream: &mut UnixStream, budget: Duration) -> Result<Vec<u8>> {
    let deadline = Instant::now() + budget;
    let mut bytes = Vec::new();
    let mut byte = [0];
    loop {
        // 整条消息共用一个 monotonic deadline，收到字节/EINTR 都不能重新开始预算。
        if read_with_deadline(stream, &mut byte, deadline)? == 0 {
            return Err("incomplete message".into());
        }
        if byte[0] == b'\n' {
            return Ok(bytes);
        }
        bytes.push(byte[0]);
        if bytes.len() > MAX {
            return Err("MessageTooLarge".into());
        }
    }
}
fn request(op: Operation) -> ClientResult<Value> {
    let stream = connect()?;
    let server = peer(&stream).map_err(|e| ClientError::new("server_peer", e))?;
    request_on_stream(op, stream, server)
}
fn request_on_stream(op: Operation, mut stream: UnixStream, server: Peer) -> ClientResult<Value> {
    let payload = serde_json::to_string(&Request { command: op })
        .map_err(|e| ClientError::new("write", e))?;
    writeln!(stream, "{payload}").map_err(|e| ClientError::new("write", e))?;
    let bytes = read_frame(&mut stream, Duration::from_secs(15))
        .map_err(|e| ClientError::new("read", e))?;
    let response: Value =
        serde_json::from_slice(&bytes).map_err(|e| ClientError::new("parse", e))?;
    Ok(json!({"server_peer":server,"response":response}))
}

// 每一步完成即写一行并 flush；第 N 步失败也保留 1..N 的结果。
// request 的 wire contract 与最后成功聚合的 cycles 字段保持原样。
fn daily_cycle(
    mut send: impl FnMut(Operation) -> ClientResult<Value>,
    output: &mut impl Write,
) -> Result<Value> {
    let mut results = Vec::new();
    for cycle_index in 1..=2 {
        for (index, op) in [
            Operation::Status,
            Operation::StartSystemProxyTest,
            Operation::Restore,
            Operation::Stop,
        ]
        .into_iter()
        .enumerate()
        {
            let mut step = json!({"event":"cycle_step", "cycle_index":cycle_index,
                "step_index":index + 1,"operation":op});
            let result = send(op);
            let error = match &result {
                Ok(value) => {
                    step["client_phase"] = json!("response");
                    step["result"] = value.clone();
                    if value["response"]["ok"] == true {
                        step["outcome"] = json!("ok");
                        None
                    } else {
                        step["outcome"] = json!("daemon_error");
                        step["daemon_error"] = value["response"]["error"].clone();
                        Some(value.to_string())
                    }
                }
                Err(error) => {
                    step["client_phase"] = json!(error.phase);
                    step["outcome"] = json!("client_error");
                    step["client_error"] = json!(error.error.to_string());
                    Some(error.to_string())
                }
            };
            writeln!(output, "{step}")?;
            output.flush()?;
            if let Some(error) = error {
                return Err(
                    format!("cycle {cycle_index} step {} {op:?}: {error}", index + 1).into(),
                );
            }
            results.push(result.unwrap());
        }
    }
    Ok(json!({"cycles":results}))
}

fn watch(pid: i32) -> Result<OwnedFd> {
    let fd = unsafe { libc::kqueue() };
    if fd < 0 {
        return at_stage("kqueue", Err(std::io::Error::last_os_error()));
    }
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return at_stage("fcntl FD_CLOEXEC", Err(std::io::Error::last_os_error()));
    }
    let event = libc::kevent {
        ident: pid as usize,
        filter: libc::EVFILT_PROC,
        flags: libc::EV_ADD | libc::EV_ONESHOT,
        fflags: libc::NOTE_EXIT,
        data: 0,
        udata: std::ptr::null_mut(),
    };
    if unsafe {
        libc::kevent(
            fd.as_raw_fd(),
            &event,
            1,
            std::ptr::null_mut(),
            0,
            std::ptr::null(),
        )
    } != 0
    {
        return at_stage(
            "kevent NOTE_EXIT register",
            Err(std::io::Error::last_os_error()),
        );
    }
    Ok(fd)
}
fn exited(fd: &OwnedFd) -> Result<bool> {
    let mut event: libc::kevent = unsafe { std::mem::zeroed() };
    let timeout = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    let n = unsafe { libc::kevent(fd.as_raw_fd(), std::ptr::null(), 0, &mut event, 1, &timeout) };
    if n < 0 {
        return at_stage(
            "kevent NOTE_EXIT poll",
            Err(std::io::Error::last_os_error()),
        );
    }
    if n == 0 {
        return Ok(false);
    }
    if event.flags & libc::EV_ERROR != 0 {
        let data = event.data;
        return Err(format!("kqueue EV_ERROR {data}").into());
    }
    Ok(event.fflags & libc::NOTE_EXIT != 0)
}

const GROUPS: &[&[&str]] = &[
    &["HTTPEnable", "HTTPProxy", "HTTPPort"],
    &["HTTPSEnable", "HTTPSProxy", "HTTPSPort"],
    &["SOCKSEnable", "SOCKSProxy", "SOCKSPort"],
    &["ProxyAutoConfigEnable", "ProxyAutoConfigURLString"],
    &["ProxyAutoDiscoveryEnable"],
    &["ExceptionsList", "ExcludeSimpleHostnames"],
];
fn managed(snapshot: &Value, port: u16) -> Value {
    let mut value = snapshot.clone();
    for prefix in ["HTTP", "HTTPS", "SOCKS"] {
        value[format!("{prefix}Enable")] = json!(1);
        value[format!("{prefix}Proxy")] = json!("127.0.0.1");
        value[format!("{prefix}Port")] = json!(port);
    }
    value["ProxyAutoConfigEnable"] = json!(0);
    value["ProxyAutoDiscoveryEnable"] = json!(0);
    value["ExceptionsList"] = json!(["localhost", "127.0.0.1"]);
    value["ExcludeSimpleHostnames"] = json!(1);
    value
}
fn restore_groups(snapshot: &Value, expected: &Value, observed: &Value) -> (Value, Vec<String>) {
    let mut restored = observed.clone();
    let mut conflicts = Vec::new();
    for group in GROUPS {
        if group
            .iter()
            .all(|key| observed.get(*key) == expected.get(*key))
        {
            for key in *group {
                if let Some(value) = snapshot.get(*key) {
                    restored[*key] = value.clone();
                } else {
                    restored.as_object_mut().unwrap().remove(*key);
                }
            }
        } else {
            conflicts.push(group.join("/"));
        }
    }
    (restored, conflicts)
}
struct Instance {
    child: Child,
    owner: Peer,
    owner_identity: Value,
    owner_watch: OwnedFd,
    child_identity: Value,
    child_watch: OwnedFd,
    service: String,
    snapshot: Value,
    managed: Value,
    port: u16,
    controller_port: u16,
    readiness: Value,
}
fn system_proxy_config(secret: &str) -> Value {
    // 固定原型配置；共享构造供容量回归验证，不接收任意配置或路径。
    json!({"log":{"level":"info"},
            "inbounds":[{"type":"mixed","tag":"p005-mixed","listen":"127.0.0.1","listen_port":0}],
            "outbounds":[{"type":"direct","tag":"p005-direct"}],
            "route":{"rules":[{"ip_cidr":["127.0.0.0/8","::1/128"],"action":"route","outbound":"p005-direct"},{"action":"reject"}]},
            "experimental":{"clash_api":{"external_controller":"127.0.0.1:0","secret":secret}}})
}
struct Helper {
    uid: u32,
    gid: u32,
    instance: Option<Instance>,
    last: Value,
    recovery_required: bool,
}
impl Helper {
    fn status(&self) -> Value {
        json!({"helper_euid":unsafe {libc::geteuid()},"instance":self.instance.as_ref().map(|i| json!({
            "owner":i.owner,"owner_identity":i.owner_identity,"child":i.child_identity,"service":i.service,
            "mixed_port":i.port,"controller_port":i.controller_port,"readiness":i.readiness,"config":"root-only disk config via anonymous pipe FD 3"})),
            "last":self.last,"recovery_required":self.recovery_required})
    }
    fn start(&mut self, owner: Peer) -> Result<Value> {
        if self.recovery_required {
            return Err("RecoveryRequired".into());
        }
        if let Some(instance) = &self.instance {
            if instance.owner.pid != owner.pid {
                return Err("Busy: another process owns this instance".into());
            }
            return Ok(self.status());
        }
        let owner_watch = at_stage("StartSystemProxyTest.owner_watch", watch(owner.pid))?;
        let owner_identity = at_stage("StartSystemProxyTest.owner_identity", process(owner.pid))?;
        if owner_identity["uid"] != owner.uid || owner_identity["gid"] != owner.gid {
            return Err(
                "StartSystemProxyTest.owner_identity: peer credentials/process mismatch".into(),
            );
        }
        let service = at_stage(
            "StartSystemProxyTest.service_id",
            fs::read_to_string(path("service-id")),
        )?;
        let read = at_stage(
            "StartSystemProxyTest.network_read",
            network("read", &service, &json!({})),
        )?;
        let snapshot = read["proxies"].clone();
        let mut random = [0u8; 32];
        at_stage(
            "StartSystemProxyTest.secret_open",
            File::open("/dev/urandom"),
        )?
        .read_exact(&mut random)
        .map_err(|e| format!("StartSystemProxyTest.secret_read: {e}"))?;
        let secret: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let config = system_proxy_config(&secret);
        at_stage(
            "StartSystemProxyTest.config_save",
            save("runtime/config.json", &config),
        )?;
        // 读取刚由 save 落盘的同一字节，不二次序列化；生产 owner 必须为 root。
        let config_bytes = at_stage(
            "StartSystemProxyTest.config_read_private",
            config_pipe::read_private_config(Path::new(&path("runtime/config.json")), 0),
        )?;
        let config_evidence = config_pipe::evidence(&config_bytes);
        if at_stage(
            "StartSystemProxyTest.kernel_hash",
            hash(Path::new(&path("sing-box"))),
        )? != KERNEL_HASH
        {
            return Err(
                "StartSystemProxyTest.kernel_hash: protected kernel digest mismatch".into(),
            );
        }
        at_stage(
            "StartSystemProxyTest.kernel_protected",
            protected(&path("sing-box"), false),
        )?;
        let config_read = at_stage(
            "StartSystemProxyTest.config_pipe",
            config_pipe::anonymous_pipe(&config_bytes),
        )?;
        let config_fd = config_read.as_raw_fd();
        let (uid, gid) = (owner.uid, owner.gid);
        let mut command = Command::new(path("sing-box"));
        command
            .args(["run", "-c", "/dev/fd/3"])
            .current_dir(ROOT)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            command.pre_exec(move || {
                config_pipe::inherit_fd3(config_fd)?;
                if libc::setsid() < 0 {
                    return Err(pre_exec_error(PreExecStage::Setsid, *libc::__error()));
                }
                drop_credentials(uid, gid)
            });
        }
        let mut child = command
            .spawn()
            .map_err(|e| spawn_error("StartSystemProxyTest.child_spawn", e))?;
        // child 已继承 FD3；父侧原 read end 在任何 readiness/cleanup 前关闭。
        drop(config_read);
        let child_pid = child.id() as i32;
        // Register before any proxy write. If preparation fails, no system proxy
        // references the child; this exact Child handle may be stopped safely.
        let prepared = (|| -> Result<(Value, OwnedFd, u16, u16, Value)> {
            let child_watch = at_stage("StartSystemProxyTest.child_watch", watch(child_pid))?;
            let identity = at_stage("StartSystemProxyTest.child_identity", process(child_pid))?;
            for key in ["uid", "ruid"] {
                if identity[key] != uid {
                    return Err(
                        "StartSystemProxyTest.child_identity: child UID was not dropped".into(),
                    );
                }
            }
            for key in ["gid", "rgid"] {
                if identity[key] != gid {
                    return Err(
                        "StartSystemProxyTest.child_identity: child GID was not dropped".into(),
                    );
                }
            }
            if identity["pgid"] != child_pid {
                return Err(
                    "StartSystemProxyTest.child_identity: child process group mismatch".into(),
                );
            }
            let (port, controller, mut readiness) = readiness::wait(&mut child, &identity, &secret)
                .map_err(|mut error| {
                    if let Some(readiness) = error.downcast_mut::<readiness::ReadinessError>() {
                        readiness.diagnostics["config_input"] = config_evidence.clone();
                    }
                    error
                })?;
            readiness["config_input"] = config_evidence.clone();
            Ok((identity, child_watch, port, controller, readiness))
        })();
        let (child_identity, child_watch, port, controller_port, readiness) = match prepared {
            Ok(value) => value,
            Err(mut error) => {
                unsafe { libc::kill(-child_pid, libc::SIGTERM) };
                if let Err(cleanup) = wait_child(&mut child) {
                    // 保持原有停止/wait 语义，cleanup 失败也不能丢掉 readiness 结构化证据。
                    if let Some(readiness) = error.downcast_mut::<readiness::ReadinessError>() {
                        readiness
                            .detail
                            .push_str(&format!("; StartSystemProxyTest.child_cleanup: {cleanup}"));
                        return Err(error);
                    }
                    return Err(
                        format!("{error}; StartSystemProxyTest.child_cleanup: {cleanup}").into(),
                    );
                }
                return Err(error);
            }
        };
        let desired = managed(&snapshot, port);
        let record = json!({"owner":owner,"owner_identity":owner_identity,"child":child_identity,"service":service,"snapshot":snapshot,"managed":desired});
        // No network mutation until the durable recovery record exists.
        if let Err(error) = at_stage(
            "StartSystemProxyTest.recovery_save",
            save("recovery.json", &record),
        ) {
            unsafe { libc::kill(-child_pid, libc::SIGTERM) };
            if let Err(cleanup) = wait_child(&mut child) {
                return Err(
                    format!("{error}; StartSystemProxyTest.child_cleanup: {cleanup}").into(),
                );
            }
            return Err(error);
        }
        self.instance = Some(Instance {
            child,
            owner,
            owner_identity,
            owner_watch,
            child_identity,
            child_watch,
            service,
            snapshot,
            managed: desired,
            port,
            controller_port,
            readiness,
        });
        let instance = self.instance.as_ref().unwrap();
        let applied = (|| -> Result<()> {
            at_stage(
                "StartSystemProxyTest.network_write",
                network(
                    "write",
                    &instance.service,
                    &json!({"desired":instance.managed,"expected":instance.snapshot}),
                ),
            )?;
            if at_stage(
                "StartSystemProxyTest.network_readback",
                network("read", &instance.service, &json!({})),
            )?["proxies"]
                != instance.managed
            {
                return Err(
                    "StartSystemProxyTest.network_readback: proxy independent readback mismatch"
                        .into(),
                );
            }
            Ok(())
        })();
        if let Err(error) = applied {
            if let Err(cleanup) = self.stop("apply_failed") {
                return Err(
                    format!("{error}; StartSystemProxyTest.apply_cleanup: {cleanup}").into(),
                );
            }
            return Err(error);
        }
        self.last =
            json!({"event":"started","independent_readback":true,"kernel_hash":KERNEL_HASH});
        Ok(self.status())
    }
    fn stop(&mut self, reason: &str) -> Result<Value> {
        if let Some(instance) = self.instance.as_mut() {
            let restored = (|| -> Result<Vec<String>> {
                let observed = network("read", &instance.service, &json!({}))?["proxies"].clone();
                let (desired, conflicts) =
                    restore_groups(&instance.snapshot, &instance.managed, &observed);
                network(
                    "write",
                    &instance.service,
                    &json!({"desired":desired,"expected":observed}),
                )?;
                let readback = network("read", &instance.service, &json!({}))?["proxies"].clone();
                if readback != desired {
                    return Err("restore independent readback mismatch".into());
                }
                // An externally modified group must not leave this owned endpoint
                // referenced when stopping. Preserve process/state if it does.
                for prefix in ["HTTP", "HTTPS", "SOCKS"] {
                    if readback
                        .get(format!("{prefix}Enable"))
                        .and_then(Value::as_i64)
                        == Some(1)
                        && readback
                            .get(format!("{prefix}Proxy"))
                            .and_then(Value::as_str)
                            == Some("127.0.0.1")
                        && readback
                            .get(format!("{prefix}Port"))
                            .and_then(Value::as_u64)
                            == Some(instance.port as u64)
                    {
                        return Err(
                            "RecoveryRequired: proxy still references owned endpoint".into()
                        );
                    }
                }
                Ok(conflicts)
            })();
            let conflicts = match restored {
                Ok(c) => c,
                Err(error) => {
                    self.recovery_required = true;
                    self.last = json!({"event":"RecoveryRequired","error":error.to_string()});
                    return Err(error);
                }
            };
            let pid = instance.child.id() as i32;
            // Child handle and monitored generation were established by this helper.
            if instance.child.try_wait()?.is_none() {
                unsafe { libc::kill(-pid, libc::SIGTERM) };
            }
            let exit = wait_child(&mut instance.child)?;
            if unsafe { libc::kill(-pid, 0) } == 0
                || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
            {
                self.recovery_required = true;
                return Err("RecoveryRequired: owned process group not empty".into());
            }
            self.last = json!({"event":reason,"exit":exit,"reaped":true,"group_empty":true,"restore_readback":true,"conflicts":conflicts});
            fs::remove_file(path("recovery.json"))?;
            fs::remove_file(path("runtime/config.json"))?;
            self.instance = None;
        }
        Ok(self.status())
    }
    fn poll(&mut self) -> Result<()> {
        if self.recovery_required {
            return Ok(());
        }
        if let Some(instance) = &mut self.instance {
            if exited(&instance.owner_watch)? {
                self.stop("owner_NOTE_EXIT")?;
            } else if exited(&instance.child_watch)? {
                self.stop("child_NOTE_EXIT")?;
            }
        }
        Ok(())
    }
    fn handle(&mut self, p: Peer, op: Operation) -> Result<Value> {
        if p.uid != self.uid || p.gid != self.gid {
            return Err("UnauthorizedPeer".into());
        }
        match op {
            Operation::Hello | Operation::Status => Ok(self.status()),
            Operation::StartSystemProxyTest => self.start(p),
            Operation::Stop | Operation::Restore | Operation::ShutdownForUninstall => {
                if self.instance.as_ref().is_some_and(|i| i.owner.pid != p.pid) {
                    return Err("OwnerMismatch".into());
                }
                self.stop("explicit_restore_stop")
            }
        }
    }
}
fn wait_child(child: &mut Child) -> Result<i32> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code().unwrap_or(-1));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Proxies are restored or were never written. This Child remains unreaped.
    unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code().unwrap_or(-1));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err("RecoveryRequired: owned child could not be reaped after bounded termination".into())
}
fn daemon() -> Result<()> {
    root()?;
    protected_root()?;
    protected(HELPER, false)?;
    let (uid, gid) = load_public_settings()?;
    if Path::new(&path("recovery.json")).exists() {
        return Err(
            "RecoveryRequired: prior recovery record exists; do not start a second writer".into(),
        );
    }
    let listener = UnixListener::bind(path("control.sock"))?;
    let socket = CString::new(path("control.sock"))?;
    if unsafe { libc::chown(socket.as_ptr(), uid, gid) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    fs::set_permissions(path("control.sock"), fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    let mut helper = Helper {
        uid,
        gid,
        instance: None,
        last: json!(null),
        recovery_required: false,
    };
    loop {
        if let Err(error) = helper.poll() {
            eprintln!("{error}");
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                stream.set_nonblocking(false)?;
                let mut shutdown = false;
                stream.set_write_timeout(Some(Duration::from_millis(500)))?;
                let result = (|| -> Result<Value> {
                    let p = peer(&stream)?;
                    if p.uid != uid || p.gid != gid {
                        return Err("UnauthorizedPeer".into());
                    }
                    let bytes = read_frame(&mut stream, Duration::from_millis(500))?;
                    let request: Request = serde_json::from_slice(&bytes)?;
                    shutdown = matches!(request.command, Operation::ShutdownForUninstall);
                    let value = helper.handle(p, request.command)?;
                    Ok(json!({"peer":p,"status":value}))
                })();
                let successful = result.is_ok();
                let response = match result {
                    Ok(v) => json!({"ok":true,"result":v}),
                    Err(e) => {
                        let mut response = json!({"ok":false,"error":e.to_string()});
                        if let Some(readiness) = e.downcast_ref::<readiness::ReadinessError>() {
                            response["readiness"] = readiness.diagnostics.clone();
                        }
                        response
                    }
                };
                let _ = writeln!(stream, "{response}");
                if shutdown && successful {
                    fs::remove_file(path("control.sock"))?;
                    return Ok(());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => return Err(error.into()),
        }
    }
}

fn launch(args: &[&str]) -> Result<()> {
    let status = Command::new("/bin/launchctl").args(args).status()?;
    if !status.success() {
        return Err(format!("launchctl {args:?}: {status}").into());
    }
    Ok(())
}
fn install(uid: u32, helper_digest: &str) -> Result<Value> {
    root()?;
    if uid == 0 {
        return Err("business UID must be non-root".into());
    }
    let user = unsafe { libc::getpwuid(uid) };
    if user.is_null() {
        return Err("business UID does not exist".into());
    }
    let gid = unsafe { (*user).pw_gid };
    for file in [ROOT, HELPER, PLIST] {
        if resource_exists(file)? {
            return Err("prototype resources already exist; no overwrite".into());
        }
    }
    let executable = std::env::current_exe()?;
    if helper_digest.len() != 64
        || !helper_digest.bytes().all(|b| b.is_ascii_hexdigit())
        || hash(&executable)? != helper_digest
    {
        return Err("staged helper digest differs from administrator-approved command".into());
    }
    let kernel = executable
        .parent()
        .ok_or("no staging directory")?
        .join("sing-box");
    if hash(&kernel)? != KERNEL_HASH {
        return Err("staged kernel digest mismatch".into());
    }
    for directory in [
        "/Library",
        "/Library/Application Support",
        "/Library/LaunchDaemons",
    ] {
        protected(directory, true)?;
    }
    fs::create_dir(ROOT)?;
    fs::set_permissions(ROOT, fs::Permissions::from_mode(0o755))?;
    protected_root()?;
    fs::create_dir(path("runtime"))?;
    fs::set_permissions(path("runtime"), fs::Permissions::from_mode(0o700))?;
    protected(&path("runtime"), true)?;
    copy_installed(&executable, HELPER)?;
    if hash(Path::new(HELPER))? != helper_digest {
        return Err("installed helper digest mismatch".into());
    }
    fs::set_permissions(HELPER, fs::Permissions::from_mode(0o755))?;
    protected(HELPER, false)?;
    copy_installed(&kernel, &path("sing-box"))?;
    fs::set_permissions(path("sing-box"), fs::Permissions::from_mode(0o755))?;
    if hash(Path::new(&path("sing-box")))? != KERNEL_HASH {
        return Err("installed kernel digest mismatch".into());
    }
    protected(&path("sing-box"), false)?;
    save("authorization.json", &json!({"uid":uid,"gid":gid}))?;
    fs::set_permissions(
        path("authorization.json"),
        fs::Permissions::from_mode(0o644),
    )?;
    let service = network("create", "", &json!({}))?;
    let id = service["id"]
        .as_str()
        .ok_or("service create returned no ID")?;
    // Native create persists service-id before commit for interrupted-install cleanup.
    if fs::read_to_string(path("service-id"))? != id {
        return Err("service ID journal mismatch".into());
    }
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(PLIST)?
        .write_all(daemon_plist().as_bytes())?;
    protected(PLIST, false)?;
    launch(&["bootstrap", "system", PLIST])?;
    Ok(
        json!({"installed":true,"helper":HELPER,"helper_hash":helper_digest,"service":service,"uid":uid,"gid":gid,"kernel_hash":KERNEL_HASH}),
    )
}
fn uninstall() -> Result<Value> {
    root()?;
    // 先校验固定清理目标，再发送请求或删除；symlink/可写目录不得成为清理入口。
    if resource_exists(ROOT)? {
        protected_root()?;
        for (file, directory) in cleanup_paths() {
            if resource_exists(&file)? {
                protected(&file, directory)?;
            }
        }
    }
    if resource_exists(PLIST)? {
        protected("/Library/LaunchDaemons", true)?;
        protected(PLIST, false)?;
    }
    if Path::new(&path("recovery.json")).exists() {
        return Err(
            "RecoveryRequired: live or unrecovered instance; preserve state and daemon".into(),
        );
    }
    if Path::new(&path("control.sock")).exists() {
        let (uid, gid) = load_public_settings()?;
        let mut command = Command::new(HELPER);
        command.args(["request", "ShutdownForUninstall"]);
        unsafe {
            command.pre_exec(move || drop_credentials(uid, gid));
        }
        let output = command
            .output()
            .map_err(|e| spawn_error("uninstall.client_spawn", e))?;
        if !output.status.success() {
            return Err("shutdown for uninstall failed; preserve resources".into());
        }
        let response: Value = serde_json::from_slice(&output.stdout)?;
        if response["response"]["ok"] != true {
            return Err("shutdown refused; preserve resources".into());
        }
    }
    if Path::new(PLIST).exists() {
        let result = Command::new("/bin/launchctl")
            .args(["bootout", "system", PLIST])
            .output()?;
        // ESRCH/not registered is acceptable only if the exact label is absent.
        if !result.status.success()
            && Command::new("/bin/launchctl")
                .args(["print", &format!("system/{LABEL}")])
                .output()?
                .status
                .success()
        {
            return Err("bootout failed while prototype daemon remains registered".into());
        }
        fs::remove_file(PLIST)?;
    }
    // Remove the journaled disabled service only after restore/stop and bootout.
    let mut removed = json!(null);
    if Path::new(&path("service-id")).exists() {
        removed = network(
            "delete",
            &fs::read_to_string(path("service-id"))?,
            &json!({}),
        )?;
    }
    // helper 位于 ROOT 内，与内核/资源一起删除，不再有目录外 executable。
    if resource_exists(ROOT)? {
        fs::remove_dir_all(ROOT)?;
    }
    Ok(json!({"uninstalled":true,"service":removed}))
}
fn parse_operation(text: &str) -> Result<Operation> {
    Ok(serde_json::from_value(json!(text))?)
}
fn run() -> Result<Value> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("local-platform-probe") if args.len() == 1 => probe::local_platform(),
        Some("local-peer-client") if args.len() == 1 => probe::local_client(),
        Some("daemon") if args.len()==1=> {daemon()?;Ok(json!(null))},
        Some("install") if args.len()==3=>install(args[1].parse()?, &args[2]),
        Some("uninstall") if args.len()==1=>uninstall(),
        Some("privileged-probe") if args.len()==3=>probe::privileged_probe(args[1].parse()?, &args[2]),
        Some("reject-probe") if args.len()==1=>probe::reject_probe(),
        Some("request") if args.len()==2=>Ok(request(parse_operation(&args[1])?)?),
        Some("owner") if args.len()==1=> {
            for op in [Operation::Hello,Operation::StartSystemProxyTest,Operation::StartSystemProxyTest] {
                let result=request(op)?;
                if result["response"]["ok"]!=true {return Err(result.to_string().into());}
                println!("{result}"); std::io::stdout().flush()?;
            }
            // Each request closes its socket. Owner keeps the same OS process.
            for _ in 0..2 {
                std::thread::sleep(Duration::from_secs(2));
                println!("{}",request(Operation::Status)?); std::io::stdout().flush()?;
            }
            loop {std::thread::sleep(Duration::from_secs(1));}
        },
        Some("cycle") if args.len()==1=>daily_cycle(request, &mut std::io::stdout().lock()),
        Some("external-test-value") if args.len()==1=> {
            root()?;
            let id=fs::read_to_string(path("service-id"))?;
            let observed=network("read",&id,&json!({}))?["proxies"].clone();
            let mut current=observed.clone();
            current["ProxyAutoConfigURLString"]=json!("http://127.0.0.1:9/p005-external.pac");
            network("write",&id,&json!({"desired":current,"expected":observed}))?;
            network("read",&id,&json!({}))
        },
        Some("network-read") if args.len()==1=>network("read",&fs::read_to_string(path("service-id"))?,&json!({})),
        Some("identity") if args.len()==1=>process(std::process::id() as i32),
        _=>Err("fixed commands only: install <authorized UID> <verified helper SHA256>, uninstall, daemon, request <closed enum>, owner, cycle, network-read, external-test-value, identity".into())
    }
}
fn main() {
    match run() {
        Ok(value) => println!("{value}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    #[derive(Default)]
    struct CycleOutput {
        bytes: Rc<RefCell<Vec<u8>>>,
        flushes: Rc<Cell<usize>>,
    }
    impl Write for CycleOutput {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes.borrow_mut().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.flushes.set(self.flushes.get() + 1);
            Ok(())
        }
    }
    fn cycle_steps(bytes: &[u8]) -> Vec<Value> {
        String::from_utf8_lossy(bytes)
            .lines()
            .map(|s| serde_json::from_str(s).unwrap())
            .collect()
    }
    fn response(ok: bool) -> Value {
        json!({"server_peer":{"uid":0,"gid":0,"pid":42},"response":{"ok":ok,"error":if ok {Value::Null} else {json!("StartSystemProxyTest.owner_watch: Invalid argument (os error 22)")}}})
    }
    // 保护真实复验诊断：任意第 N 步失败前，前序结果已 flush，且不会继续调用下游步骤。
    #[test]
    fn cycle_failure_keeps_flushed_predecessors_and_identifies_operation() {
        for fail_at in 1..=8 {
            let mut output = CycleOutput::default();
            let flushes = output.flushes.clone();
            let mut calls = 0;
            let error = daily_cycle(
                |_| {
                    assert_eq!(flushes.get(), calls);
                    calls += 1;
                    if calls == fail_at {
                        Ok(response(false))
                    } else {
                        Ok(response(true))
                    }
                },
                &mut output,
            )
            .unwrap_err();
            assert_eq!(calls, fail_at);
            assert_eq!(output.flushes.get(), fail_at);
            let steps = cycle_steps(&output.bytes.borrow());
            assert_eq!(steps.len(), fail_at);
            assert!(steps[..fail_at - 1].iter().all(|s| s["outcome"] == "ok"));
            let failed = &steps[fail_at - 1];
            let operation =
                ["Status", "StartSystemProxyTest", "Restore", "Stop"][(fail_at - 1) % 4];
            assert_eq!(failed["cycle_index"], (fail_at - 1) / 4 + 1);
            assert_eq!(failed["step_index"], (fail_at - 1) % 4 + 1);
            assert_eq!(failed["operation"], operation);
            assert_eq!(failed["client_phase"], "response");
            assert_eq!(failed["outcome"], "daemon_error");
            assert_eq!(failed["daemon_error"], response(false)["response"]["error"]);
            assert!(error.to_string().contains(operation));
        }
    }
    // 保护首步 transport 错误也有 stdout；daemon ok=false 与五个 client phase 不混淆。
    #[test]
    fn cycle_transport_failure_is_visible_and_distinct_from_daemon_error() {
        for phase in ["connect", "server_peer", "write", "read", "parse"] {
            let mut output = CycleOutput::default();
            let error = daily_cycle(
                |_| {
                    Err(ClientError::new(
                        phase,
                        std::io::Error::from_raw_os_error(libc::EINVAL),
                    ))
                },
                &mut output,
            )
            .unwrap_err();
            let steps = cycle_steps(&output.bytes.borrow());
            assert_eq!(steps.len(), 1);
            assert_eq!(steps[0]["client_phase"], phase);
            assert_eq!(steps[0]["outcome"], "client_error");
            assert_eq!(steps[0]["operation"], "Status");
            assert!(
                steps[0]["client_error"]
                    .as_str()
                    .unwrap()
                    .contains("os error 22")
            );
            assert!(steps[0].get("daemon_error").is_none());
            assert!(error.to_string().contains(phase));
            assert_eq!(output.flushes.get(), 1);
        }
    }
    // 保护正常双轮顺序和旧成功聚合字段，且发送下一步前上一行已经可见。
    #[test]
    fn normal_cycle_preserves_two_rounds_and_success_summary() {
        let mut output = CycleOutput::default();
        let flushes = output.flushes.clone();
        let mut operations = Vec::new();
        let result = daily_cycle(
            |op| {
                assert_eq!(flushes.get(), operations.len());
                operations.push(format!("{op:?}"));
                Ok(response(true))
            },
            &mut output,
        )
        .unwrap();
        assert_eq!(
            operations,
            ["Status", "StartSystemProxyTest", "Restore", "Stop"].repeat(2)
        );
        assert_eq!(result["cycles"].as_array().unwrap().len(), 8);
        assert_eq!(cycle_steps(&output.bytes.borrow()).len(), 8);
        assert_eq!(output.flushes.get(), 8);
    }
    // 只用内存 socket pair：验证实际 request 的 write/read/parse phase 归属，无 daemon。
    #[test]
    fn request_transport_phases_are_classified_at_the_boundary() {
        let server = Peer {
            uid: 0,
            gid: 0,
            pid: 42,
        };
        let (client, _other) = UnixStream::pair().unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        assert_eq!(
            request_on_stream(Operation::Status, client, server)
                .unwrap_err()
                .phase,
            "write"
        );
        let (client, other) = UnixStream::pair().unwrap();
        other.shutdown(std::net::Shutdown::Write).unwrap();
        assert_eq!(
            request_on_stream(Operation::Status, client, server)
                .unwrap_err()
                .phase,
            "read"
        );
        let (client, mut other) = UnixStream::pair().unwrap();
        other.write_all(b"invalid-json\n").unwrap();
        assert_eq!(
            request_on_stream(Operation::Status, client, server)
                .unwrap_err()
                .phase,
            "parse"
        );
        let (client, mut other) = UnixStream::pair().unwrap();
        other
            .write_all(b"{\"ok\":false,\"error\":\"daemon-stage\"}\n")
            .unwrap();
        let result = request_on_stream(Operation::Status, client, server).unwrap();
        assert_eq!(result["response"]["ok"], false);
        assert_eq!(result["response"]["error"], "daemon-stage");
    }
    // 保护 fork 后纯整数错误通道：六个 syscall 与原 errno 可一一恢复，不误标 exec 错误。
    #[test]
    fn pre_exec_mapping_preserves_syscall_and_errno() {
        for syscall in [
            PreExecStage::Dup2,
            PreExecStage::Fcntl,
            PreExecStage::Setsid,
            PreExecStage::Setgroups,
            PreExecStage::Setgid,
            PreExecStage::Setuid,
        ] {
            for errno in [libc::EINVAL, libc::EPERM, libc::EBADF] {
                let error = pre_exec_error(syscall, errno);
                let (decoded, original) =
                    decode_pre_exec_error(error.raw_os_error().unwrap()).unwrap();
                assert_eq!(decoded.name(), syscall.name());
                assert_eq!(original, errno);
                let message = spawn_error("StartSystemProxyTest.child_spawn", error).to_string();
                assert!(message.starts_with(&format!(
                    "StartSystemProxyTest.child_spawn.pre_exec.{}:",
                    syscall.name()
                )));
                assert!(message.contains(&format!("os error {errno}")));
            }
        }
        assert!(decode_pre_exec_error(libc::EINVAL).is_none());
        assert!(decode_pre_exec_error(PRE_EXEC_BASE + 7 * PRE_EXEC_STRIDE).is_none());
        assert_eq!(
            spawn_error(
                "child_spawn",
                std::io::Error::from_raw_os_error(libc::ENOENT)
            )
            .to_string(),
            format!(
                "child_spawn: {}",
                std::io::Error::from_raw_os_error(libc::ENOENT)
            )
        );
    }
    // 保护 native 错误上下文透传，不调用 SystemConfiguration 或 proc_pidinfo。
    #[test]
    fn native_error_keeps_operation_api_and_error_code() {
        let payload = CString::new(r#"{"error":"SCPreferencesCommitChanges","operation":"write","sc_error":1001,"detail":"probe"}"#).unwrap();
        let raw = unsafe { libc::strdup(payload.as_ptr()) };
        let error = at_stage(
            "StartSystemProxyTest.network_write",
            at_stage("SystemConfiguration.write", native_value(raw)),
        )
        .unwrap_err()
        .to_string();
        for token in [
            "StartSystemProxyTest.network_write",
            "SystemConfiguration.write",
            "SCPreferencesCommitChanges",
            "1001",
        ] {
            assert!(error.contains(token));
        }
        let error = at_stage(
            "StartSystemProxyTest.config_save",
            at_stage::<(), _>(
                "save.sync_directory",
                Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
            ),
        )
        .unwrap_err()
        .to_string();
        assert!(error.starts_with("StartSystemProxyTest.config_save: save.sync_directory:"));
        assert!(error.contains("os error 22"));
    }
    // 保护传统 LaunchDaemon 的固定路径/参数契约，防止退回 privileged helper 目录。
    #[test]
    fn fixed_launchdaemon_paths_and_plist() {
        assert_eq!(HELPER, format!("{ROOT}/helper"));
        assert_eq!(PLIST, format!("/Library/LaunchDaemons/{LABEL}.plist"));
        let plist = daemon_plist();
        assert!(plist.contains(&format!("<key>ProgramArguments</key><array><string>{HELPER}</string><string>daemon</string></array>")));
        assert!(plist.contains(&format!("<key>Label</key><string>{LABEL}</string>")));
        assert_eq!(
            plist
                .matches(&format!("<string>{ROOT}/daemon.log</string>"))
                .count(),
            2
        );
        assert!(!plist.contains("PrivilegedHelperTools"));
    }
    // 保护新建安装资产的权限与覆盖边界；真实 root owner 另由平台复验验证。
    #[test]
    fn installed_copy_is_private_owned_by_creator_and_refuses_overwrite() {
        // 保护安装资产不继承源权限/owner，且不能覆盖已有文件或 symlink。
        let temporary =
            std::env::temp_dir().join(format!("veyra-p005-copy-test-{}", std::process::id()));
        fs::create_dir(&temporary).unwrap();
        let source = temporary.join("source");
        fs::write(&source, b"fixed installation bytes").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o777)).unwrap();
        let destination = temporary.join("helper");
        copy_installed(&source, destination.to_str().unwrap()).unwrap();
        let meta = fs::symlink_metadata(&destination).unwrap();
        assert_eq!(meta.uid(), unsafe { libc::geteuid() });
        assert_eq!(meta.mode() & 0o777, 0o600);
        assert!(meta.is_file());
        assert_eq!(fs::read(&destination).unwrap(), b"fixed installation bytes");
        assert!(copy_installed(&source, destination.to_str().unwrap()).is_err());
        fs::remove_file(&destination).unwrap();
        std::os::unix::fs::symlink(&source, &destination).unwrap();
        assert!(copy_installed(&source, destination.to_str().unwrap()).is_err());
        assert_eq!(fs::read(&source).unwrap(), b"fixed installation bytes");
        fs::remove_dir_all(temporary).unwrap();
    }
    // 保护安装/daemon/卸载共同的 owner、不可写与类型校验；无需真实 root 文件。
    #[test]
    fn protected_paths_reject_wrong_owner_writes_symlinks_and_special_files() {
        assert!(protected_identity(
            0,
            u32::from(libc::S_IFDIR) | 0o755,
            true
        ));
        assert!(protected_identity(
            0,
            u32::from(libc::S_IFREG) | 0o755,
            false
        ));
        for (uid, mode) in [
            (501, u32::from(libc::S_IFREG) | 0o755),
            (0, u32::from(libc::S_IFREG) | 0o775),
            (0, u32::from(libc::S_IFREG) | 0o757),
            (0, u32::from(libc::S_IFLNK) | 0o755),
            (0, u32::from(libc::S_IFSOCK) | 0o600),
        ] {
            assert!(!protected_identity(uid, mode, false));
        }
        assert!(!protected_identity(
            0,
            u32::from(libc::S_IFREG) | 0o755,
            true
        ));
        assert!(!protected_identity(
            0,
            u32::from(libc::S_IFDIR) | 0o777,
            true
        ));
    }
    // 保护预检：悬空 symlink 也是已占用资源，不能安装覆盖或进入清理。
    #[test]
    fn preflight_detects_dangling_symlink() {
        let temporary =
            std::env::temp_dir().join(format!("veyra-p005-path-test-{}", std::process::id()));
        fs::create_dir(&temporary).unwrap();
        let link = temporary.join("helper");
        std::os::unix::fs::symlink(temporary.join("absent"), &link).unwrap();
        assert!(resource_exists(link.to_str().unwrap()).unwrap());
        assert!(protected(link.to_str().unwrap(), false).is_err());
        assert!(!resource_exists(temporary.join("absent").to_str().unwrap()).unwrap());
        fs::remove_dir_all(temporary).unwrap();
    }
    // 保护卸载边界：校验目标全在 ROOT 内，helper 会随 ROOT 一起删除。
    #[test]
    fn cleanup_targets_stay_in_protected_root() {
        let targets = cleanup_paths();
        assert!(
            targets
                .iter()
                .any(|(file, directory)| file == HELPER && !directory)
        );
        assert!(
            targets
                .iter()
                .any(|(file, directory)| file == &path("runtime") && *directory)
        );
        for (file, _) in targets {
            assert!(Path::new(&file).strip_prefix(ROOT).is_ok());
            assert!(!file.contains("PrivilegedHelperTools"));
        }
    }
    // Protect the helper boundary: user-provided identity/path/command cannot execute.
    #[test]
    fn rejects_untrusted_extra_fields_and_commands() {
        for request in [
            r#"{"command":"Status","pid":1}"#,
            r#"{"command":"Status","path":"/bin/sh"}"#,
            r#"{"command":"RunShell"}"#,
            r#"{"command":"StartSystemProxyTest","uid":0}"#,
        ] {
            assert!(serde_json::from_str::<Request>(request).is_err());
        }
    }
    // Protect restoration: retain a changed related field group, restore the others.
    #[test]
    fn preserves_external_pac_and_restores_absent_snapshot_fields() {
        let snapshot = json!({"ProxyAutoConfigEnable":1,"ProxyAutoConfigURLString":"http://127.0.0.1:9/original"});
        let managed = managed(&snapshot, 12345);
        let mut observed = managed.clone();
        observed["ProxyAutoConfigURLString"] = json!("external");
        let (restored, conflicts) = restore_groups(&snapshot, &managed, &observed);
        assert_eq!(
            conflicts,
            vec!["ProxyAutoConfigEnable/ProxyAutoConfigURLString"]
        );
        assert_eq!(restored["ProxyAutoConfigURLString"], "external");
        assert_eq!(restored["ProxyAutoConfigEnable"], 0);
        assert!(restored.get("HTTPProxy").is_none());
        assert!(restored.get("ExceptionsList").is_none());
    }
    // 保护一请求一连接：peer 写完响应立即关闭，读取期间也不能丢弃缓存的完整 JSON。
    #[test]
    fn frame_survives_peer_write_then_immediate_drop() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        let peer = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(20));
            writer.write_all(b"{\"ok\":true}\n").unwrap();
            // write_all 后立即 drop，没有额外握手或延迟关闭。
        });
        let bytes = read_frame(&mut reader, Duration::from_secs(2)).unwrap();
        peer.join().unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&bytes).unwrap(),
            json!({"ok":true})
        );
    }
    // 保护本次确定性回归：read_frame 开始前 peer 已 close，HUP 与缓存数据同时存在。
    #[test]
    fn buffered_frame_survives_already_closed_peer() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        writer.write_all(b"{\"ok\":true}\n").unwrap();
        drop(writer);
        let mut fd = libc::pollfd {
            fd: reader.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        assert_eq!(unsafe { libc::poll(&mut fd, 1, 0) }, 1);
        assert_ne!(fd.revents & libc::POLLHUP, 0);
        assert_eq!(
            read_frame(&mut reader, Duration::from_millis(500)).unwrap(),
            b"{\"ok\":true}"
        );
    }
    // 保护封帧契约：EOF 前没有 newline 时不能把残缺/空消息作为成功响应。
    #[test]
    fn closed_peer_without_newline_is_incomplete() {
        for data in [b"".as_slice(), b"{\"ok\":true}"] {
            let (mut reader, mut writer) = UnixStream::pair().unwrap();
            writer.write_all(data).unwrap();
            drop(writer);
            assert_eq!(
                read_frame(&mut reader, Duration::from_secs(2))
                    .unwrap_err()
                    .to_string(),
                "incomplete message"
            );
        }
    }
    // 保护短预算：静默 peer 应按调用预算报总 deadline，而不是等待 connect 旧有的 15s。
    #[test]
    fn silent_peer_obeys_small_total_deadline() {
        let (mut reader, _writer) = UnixStream::pair().unwrap();
        let budget = Duration::from_millis(40);
        let start = Instant::now();
        assert_eq!(
            read_frame(&mut reader, budget).unwrap_err().to_string(),
            "MessageDeadlineExceeded"
        );
        let elapsed = start.elapsed();
        assert!(elapsed >= budget);
        assert!(elapsed < Duration::from_millis(500), "elapsed: {elapsed:?}");
    }
    // 保护 poll 的不足 1ms/零预算：不能变为无限等待，也不能在 deadline 之前误报。
    #[test]
    fn submillisecond_and_zero_budgets_expire() {
        let (mut reader, _writer) = UnixStream::pair().unwrap();
        for budget in [Duration::ZERO, Duration::from_micros(500)] {
            let start = Instant::now();
            assert_eq!(
                read_frame(&mut reader, budget).unwrap_err().to_string(),
                "MessageDeadlineExceeded"
            );
            let elapsed = start.elapsed();
            assert!(elapsed >= budget);
            assert!(elapsed < Duration::from_millis(500), "elapsed: {elapsed:?}");
        }
    }
    // 保护单一 timeout authority：读取不改 SO_RCVTIMEO 或 blocking flags；不使用源码字符串断言。
    #[test]
    fn framing_leaves_socket_timeout_and_blocking_mode_unchanged() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        let timeout = Some(Duration::from_secs(7));
        reader.set_read_timeout(timeout).unwrap();
        let flags = unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFL) };
        assert!(flags >= 0);
        assert_eq!(flags & libc::O_NONBLOCK, 0);
        writer.write_all(b"json\n").unwrap();
        assert_eq!(
            read_frame(&mut reader, Duration::from_millis(500)).unwrap(),
            b"json"
        );
        assert_eq!(reader.read_timeout().unwrap(), timeout);
        assert_eq!(
            unsafe { libc::fcntl(reader.as_raw_fd(), libc::F_GETFL) },
            flags
        );
    }
    // 保护 MAX 的含义：4096 个 payload 字节可封帧，newline 不占 payload 上限，不多读下条消息。
    #[test]
    fn frame_accepts_max_payload_and_preserves_next_frame() {
        let (mut reader, mut writer) = UnixStream::pair().unwrap();
        let payload = vec![b'x'; MAX];
        writer.write_all(&payload).unwrap();
        writer.write_all(b"\n\nnext\n").unwrap();
        drop(writer);
        assert_eq!(
            read_frame(&mut reader, Duration::from_millis(500)).unwrap(),
            payload
        );
        assert!(
            read_frame(&mut reader, Duration::from_millis(500))
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            read_frame(&mut reader, Duration::from_millis(500)).unwrap(),
            b"next"
        );
    }
    // 保护 IPC 上限：超过 MAX 的 payload 在无需 newline 的情况下立即拒绝。
    #[test]
    fn bounded_message_rejects_excess_bytes() {
        let (mut server, mut client) = UnixStream::pair().unwrap();
        client.write_all(&vec![b'x'; MAX + 1]).unwrap();
        assert!(
            read_frame(&mut server, Duration::from_millis(100))
                .unwrap_err()
                .to_string()
                .contains("MessageTooLarge")
        );
    }
    // 保护总预算：每个字节间隔小于 budget，整条消息超时仍必须报 deadline。
    #[test]
    fn slow_message_has_total_deadline() {
        let (mut server, mut client) = UnixStream::pair().unwrap();
        let writer = std::thread::spawn(move || {
            for _ in 0..10 {
                if client.write_all(b"x").is_err() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        let start = Instant::now();
        let budget = Duration::from_millis(100);
        assert_eq!(
            read_frame(&mut server, budget).unwrap_err().to_string(),
            "MessageDeadlineExceeded"
        );
        let elapsed = start.elapsed();
        assert!(elapsed >= budget);
        assert!(elapsed < Duration::from_millis(400), "elapsed: {elapsed:?}");
        drop(server);
        writer.join().unwrap();
    }
    // Protect exact restoration including PAC/autodiscovery/exceptions defaults.
    #[test]
    fn restores_exact_unconflicted_snapshot() {
        let snapshot = json!({"HTTPEnable":0,"ProxyAutoConfigEnable":1,"ProxyAutoConfigURLString":"http://127.0.0.1:9/p005.pac",
            "ProxyAutoDiscoveryEnable":1,"ExceptionsList":["p005.invalid"],"ExcludeSimpleHostnames":0});
        let managed = managed(&snapshot, 12345);
        let (restored, conflicts) = restore_groups(&snapshot, &managed, &managed);
        assert_eq!(restored, snapshot);
        assert!(conflicts.is_empty());
    }
}
