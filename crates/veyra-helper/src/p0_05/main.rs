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
mod probe;

const ROOT: &str = "/Library/Application Support/VeyraP005";
const HELPER: &str = "/Library/PrivilegedHelperTools/com.lifei6671.veyra.p005";
const PLIST: &str = "/Library/LaunchDaemons/com.lifei6671.veyra.p005.plist";
const LABEL: &str = "com.lifei6671.veyra.p005";
const KERNEL_HASH: &str = "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4";
const MAX: usize = 4096;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

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
    let op = CString::new(op)?;
    let id = CString::new(id)?;
    let config = CString::new(config.to_string())?;
    native_value(unsafe { p005_network(op.as_ptr(), id.as_ptr(), config.as_ptr()) })
}
fn process(pid: i32) -> Result<Value> {
    native_value(unsafe { p005_process(pid) })
}
fn path(name: &str) -> String {
    format!("{ROOT}/{name}")
}
fn hash(file: &Path) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(fs::read(file)?)))
}
fn save(name: &str, value: &Value) -> Result<()> {
    let temporary = path(&format!("{name}.new"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(value.to_string().as_bytes())?;
    file.sync_all()?;
    fs::rename(temporary, path(name))?;
    File::open(ROOT)?.sync_all()?;
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
    if meta.uid() != 0
        || meta.mode() & 0o022 != 0
        || meta.file_type().is_symlink()
        || meta.is_dir() != directory
    {
        return Err(format!("unprotected prototype path: {file}").into());
    }
    Ok(())
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
        return Err(std::io::Error::last_os_error().into());
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
        || p.pid <= 0
    {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(p)
}
fn connect() -> Result<UnixStream> {
    protected(ROOT, true)?;
    let meta = fs::symlink_metadata(path("control.sock"))?;
    let settings = load_public_settings()?;
    if meta.uid() != settings.0 || meta.mode() & 0o777 != 0o600 {
        return Err("socket ownership/mode mismatch".into());
    }
    let stream = UnixStream::connect(path("control.sock"))?;
    let p = peer(&stream)?;
    if p.uid != 0 {
        return Err("server peer is not root".into());
    }
    stream.set_read_timeout(Some(Duration::from_secs(15)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
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
fn read_frame(stream: &mut UnixStream, budget: Duration) -> Result<Vec<u8>> {
    let deadline = Instant::now() + budget;
    let mut bytes = Vec::new();
    let mut byte = [0];
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("MessageDeadlineExceeded".into());
        }
        stream.set_read_timeout(Some(remaining))?;
        if stream.read(&mut byte)? == 0 {
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
fn request(op: Operation) -> Result<Value> {
    let mut stream = connect()?;
    let server = peer(&stream)?;
    writeln!(
        stream,
        "{}",
        serde_json::to_string(&Request { command: op })?
    )?;
    let response: Value =
        serde_json::from_slice(&read_frame(&mut stream, Duration::from_secs(15))?)?;
    Ok(json!({"server_peer":server,"response":response}))
}

fn watch(pid: i32) -> Result<OwnedFd> {
    let fd = unsafe { libc::kqueue() };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    let fd = unsafe { OwnedFd::from_raw_fd(fd) };
    if unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(std::io::Error::last_os_error().into());
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
        return Err(std::io::Error::last_os_error().into());
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
        return Err(std::io::Error::last_os_error().into());
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
            "mixed_port":i.port,"controller_port":i.controller_port,"config":"root-only config inherited FD 3"})),
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
        let owner_watch = watch(owner.pid)?;
        let owner_identity = process(owner.pid)?;
        if owner_identity["uid"] != owner.uid || owner_identity["gid"] != owner.gid {
            return Err("peer credentials/process mismatch".into());
        }
        let service = fs::read_to_string(path("service-id"))?;
        let read = network("read", &service, &json!({}))?;
        let snapshot = read["proxies"].clone();
        let mut random = [0u8; 32];
        File::open("/dev/urandom")?.read_exact(&mut random)?;
        let secret: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let config = json!({"log":{"level":"info"},
            "inbounds":[{"type":"mixed","tag":"p005-mixed","listen":"127.0.0.1","listen_port":0}],
            "outbounds":[{"type":"direct","tag":"p005-direct"}],
            "route":{"rules":[{"ip_cidr":["127.0.0.0/8","::1/128"],"action":"route","outbound":"p005-direct"},{"action":"reject"}]},
            "experimental":{"clash_api":{"external_controller":"127.0.0.1:0","secret":secret}}});
        save("runtime/config.json", &config)?;
        let config_file = File::open(path("runtime/config.json"))?;
        let config_fd = config_file.as_raw_fd();
        if hash(Path::new(&path("sing-box")))? != KERNEL_HASH {
            return Err("protected kernel digest mismatch".into());
        }
        protected(&path("sing-box"), false)?;
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
                if config_fd != 3 && libc::dup2(config_fd, 3) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(3, libc::F_SETFD, 0) < 0
                    || libc::setsid() < 0
                    || libc::setgroups(0, std::ptr::null()) != 0
                    || libc::setgid(gid) != 0
                    || libc::setuid(uid) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn()?;
        let child_pid = child.id() as i32;
        // Register before any proxy write. If preparation fails, no system proxy
        // references the child; this exact Child handle may be stopped safely.
        let prepared = (|| -> Result<(Value, OwnedFd, u16, u16)> {
            let child_watch = watch(child_pid)?;
            let identity = process(child_pid)?;
            for key in ["uid", "ruid"] {
                if identity[key] != uid {
                    return Err("child UID was not dropped".into());
                }
            }
            for key in ["gid", "rgid"] {
                if identity[key] != gid {
                    return Err("child GID was not dropped".into());
                }
            }
            if identity["pgid"] != child_pid {
                return Err("child process group mismatch".into());
            }
            let mut output = String::new();
            let stdout = child.stdout.as_mut().unwrap();
            let stderr = child.stderr.as_mut().unwrap();
            for fd in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
                unsafe { libc::fcntl(fd, libc::F_SETFL, libc::O_NONBLOCK) };
            }
            let deadline = Instant::now() + Duration::from_secs(10);
            let (mut port, mut controller) = (None, None);
            while Instant::now() < deadline {
                let mut bytes = [0; 2048];
                for pipe in [&mut *stdout as &mut dyn Read, &mut *stderr as &mut dyn Read] {
                    if let Ok(n) = pipe.read(&mut bytes) {
                        output.push_str(&String::from_utf8_lossy(&bytes[..n]));
                    }
                }
                if output.len() > 65536 {
                    return Err("child startup output exceeded budget".into());
                }
                for line in output.lines() {
                    if let Some((prefix, address)) = line.rsplit_once("127.0.0.1:")
                        && let Ok(p) = address.trim().parse::<u16>()
                    {
                        if prefix.contains("inbound/mixed[p005-mixed]: tcp server started at ") {
                            port = Some(p);
                        }
                        if prefix.contains("clash-api: restful api listening at ") {
                            controller = Some(p);
                        }
                    }
                }
                if let (Some(port), Some(controller)) = (port, controller) {
                    let mut stream = std::net::TcpStream::connect_timeout(
                        &format!("127.0.0.1:{controller}").parse()?,
                        Duration::from_secs(1),
                    )?;
                    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
                    write!(
                        stream,
                        "GET /version HTTP/1.0\r\nHost: localhost\r\nAuthorization: Bearer {secret}\r\n\r\n"
                    )?;
                    let mut response = String::new();
                    stream.take(4096).read_to_string(&mut response)?;
                    if !response.starts_with("HTTP/1.0 200")
                        && !response.starts_with("HTTP/1.1 200")
                    {
                        return Err("controller authentication/readiness failed".into());
                    }
                    return Ok((identity, child_watch, port, controller));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err("child startup timeout; no proxy write".into())
        })();
        let (child_identity, child_watch, port, controller_port) = match prepared {
            Ok(value) => value,
            Err(error) => {
                unsafe { libc::kill(-child_pid, libc::SIGTERM) };
                wait_child(&mut child)?;
                return Err(error);
            }
        };
        let desired = managed(&snapshot, port);
        let record = json!({"owner":owner,"owner_identity":owner_identity,"child":child_identity,"service":service,"snapshot":snapshot,"managed":desired});
        // No network mutation until the durable recovery record exists.
        if let Err(error) = save("recovery.json", &record) {
            unsafe { libc::kill(-child_pid, libc::SIGTERM) };
            wait_child(&mut child)?;
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
        });
        let instance = self.instance.as_ref().unwrap();
        let applied = (|| -> Result<()> {
            network(
                "write",
                &instance.service,
                &json!({"desired":instance.managed,"expected":instance.snapshot}),
            )?;
            if network("read", &instance.service, &json!({}))?["proxies"] != instance.managed {
                return Err("proxy independent readback mismatch".into());
            }
            Ok(())
        })();
        if let Err(error) = applied {
            self.stop("apply_failed")?;
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
    protected(ROOT, true)?;
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
                stream.set_read_timeout(Some(Duration::from_millis(500)))?;
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
                    Err(e) => json!({"ok":false,"error":e.to_string()}),
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
        if Path::new(file).exists() {
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
    fs::create_dir(ROOT)?;
    fs::set_permissions(ROOT, fs::Permissions::from_mode(0o755))?;
    fs::create_dir(path("runtime"))?;
    fs::set_permissions(path("runtime"), fs::Permissions::from_mode(0o700))?;
    fs::copy(executable, HELPER)?;
    if hash(Path::new(HELPER))? != helper_digest {
        return Err("installed helper digest mismatch".into());
    }
    fs::set_permissions(HELPER, fs::Permissions::from_mode(0o755))?;
    fs::copy(kernel, path("sing-box"))?;
    fs::set_permissions(path("sing-box"), fs::Permissions::from_mode(0o755))?;
    if hash(Path::new(&path("sing-box")))? != KERNEL_HASH {
        return Err("installed kernel digest mismatch".into());
    }
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
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\"><plist version=\"1.0\"><dict><key>Label</key><string>{LABEL}</string><key>ProgramArguments</key><array><string>{HELPER}</string><string>daemon</string></array><key>RunAtLoad</key><true/><key>StandardOutPath</key><string>{ROOT}/daemon.log</string><key>StandardErrorPath</key><string>{ROOT}/daemon.log</string><key>ExitTimeOut</key><integer>10</integer></dict></plist>"
    );
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o644)
        .open(PLIST)?
        .write_all(plist.as_bytes())?;
    launch(&["bootstrap", "system", PLIST])?;
    Ok(json!({"installed":true,"service":service,"uid":uid,"gid":gid,"kernel_hash":KERNEL_HASH}))
}
fn uninstall() -> Result<Value> {
    root()?;
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
            command.pre_exec(move || {
                if libc::setgroups(0, std::ptr::null()) != 0
                    || libc::setgid(gid) != 0
                    || libc::setuid(uid) != 0
                {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let output = command.output()?;
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
    if Path::new(HELPER).exists() {
        fs::remove_file(HELPER)?;
    }
    if Path::new(ROOT).exists() {
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
        Some("request") if args.len()==2=>request(parse_operation(&args[1])?),
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
        Some("cycle") if args.len()==1=> {
            let mut results=Vec::new();
            for _ in 0..2 {
                for op in [Operation::Status,Operation::StartSystemProxyTest,Operation::Restore,Operation::Stop] {
                    let result=request(op)?;
                    if result["response"]["ok"]!=true {return Err(result.to_string().into());}
                    results.push(result);
                }
            }
            Ok(json!({"cycles":results}))
        },
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
    // Protect the IPC owner monitor against slow partial or oversized messages.
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
    #[test]
    fn slow_message_has_total_deadline() {
        let (mut server, mut client) = UnixStream::pair().unwrap();
        let writer = std::thread::spawn(move || {
            for _ in 0..10 {
                let _ = client.write_all(b"x");
                std::thread::sleep(Duration::from_millis(50));
            }
        });
        let start = Instant::now();
        assert!(read_frame(&mut server, Duration::from_millis(100)).is_err());
        assert!(start.elapsed() < Duration::from_millis(400));
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
