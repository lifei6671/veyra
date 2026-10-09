//! 正式 SystemProxy child Port。复用 Core SidecarRuntime/ManualRuntime 与鉴权 controller。
use super::{
    CONFIG_BYTES,
    assets::{Assets, private_directory},
};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::SocketAddr,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{
            fs::{MetadataExt, OpenOptionsExt},
            process::CommandExt,
        },
    },
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use veyra_core::{
    application::owned_child::terminate,
    singbox::{
        GeneratedConfig,
        clash_api::{ClashApiClient, ManagedControllerEndpoint},
        runtime::{ManagedRuntimeEndpoints, ManagedSidecar, SidecarPort, SidecarPortError},
    },
};
const BUDGET: Duration = Duration::from_secs(10);
struct Running {
    config: GeneratedConfig,
    child: Child,
    identity: ManagedSidecar,
    lines: mpsc::Receiver<(bool, SocketAddr)>,
    readers: Vec<thread::JoinHandle<()>>,
    end_read: Arc<AtomicBool>,
    controller: Option<std::sync::Arc<ManagedControllerEndpoint>>,
    endpoints: Option<ManagedRuntimeEndpoints>,
}
pub(super) struct ProcessPort {
    assets: Assets,
    lease: Result<super::writer_lease::WriterLease, super::Error>,
    candidate: Option<GeneratedConfig>,
    cache: Option<(PathBuf, u64, u64)>,
    running: Option<Running>,
    checking: Option<Child>,
    next: u64,
    deadline: Arc<std::sync::Mutex<Instant>>,
}
impl ProcessPort {
    pub(super) fn acquire_writer_lease(
        _assets: &Assets,
    ) -> Result<super::writer_lease::WriterLease, super::Error> {
        #[cfg(test)]
        let lease_root = if _assets.fixture {
            _assets.root.as_path()
        } else {
            std::path::Path::new(super::transport::ROOT)
        };
        #[cfg(not(test))]
        let lease_root = std::path::Path::new(super::transport::ROOT);
        super::writer_lease::WriterLease::acquire(lease_root)
    }
    #[cfg(test)]
    pub fn new(assets: Assets, deadline: Arc<std::sync::Mutex<Instant>>) -> Self {
        let lease = Self::acquire_writer_lease(&assets);
        Self::with_writer_lease(assets, deadline, lease)
    }
    pub(super) fn with_writer_lease(
        assets: Assets,
        deadline: Arc<std::sync::Mutex<Instant>>,
        lease: Result<super::writer_lease::WriterLease, super::Error>,
    ) -> Self {
        Self {
            assets,
            lease,
            candidate: None,
            cache: None,
            running: None,
            checking: None,
            next: 0,
            deadline,
        }
    }
    fn time_left(&self) -> Result<Duration, SidecarPortError> {
        self.deadline
            .lock()
            .expect("operation deadline")
            .checked_duration_since(Instant::now())
            .ok_or(SidecarPortError)
            .map(|d| d.min(BUDGET))
    }
    fn spawn(
        &mut self,
        action: &'static str,
        config: &GeneratedConfig,
    ) -> Result<Child, SidecarPortError> {
        self.assets.verify().map_err(|_| SidecarPortError)?;
        let lease = self
            .lease
            .as_ref()
            .map_err(|_| SidecarPortError)?
            .child_fd()
            .map_err(|_| SidecarPortError)?;
        if config.as_bytes().len() > CONFIG_BYTES {
            return Err(SidecarPortError);
        }
        let mut fds = [-1; 2];
        if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
            return Err(SidecarPortError);
        }
        let read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
        let mut write = unsafe { File::from_raw_fd(fds[1]) };
        for fd in fds {
            if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
                return Err(SidecarPortError);
            }
        }
        if unsafe { libc::fcntl(write.as_raw_fd(), libc::F_SETFL, libc::O_NONBLOCK) } < 0 {
            return Err(SidecarPortError);
        }
        let mut command = Command::new(&self.assets.kernel);
        command.args([action, "-c", "/dev/fd/3"]);
        #[cfg(test)]
        if self.assets.fixture {
            command = Command::new(&self.assets.kernel);
            command
                .args([
                    "--exact",
                    "production::process_tests::kernel_fixture",
                    "--ignored",
                    "--nocapture",
                ])
                .env("VEYRA_FIXTURE_ACTION", action);
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(test)]
        let fixture = self.assets.fixture;
        #[cfg(not(test))]
        let fixture = false;
        if !fixture {
            command.env_clear();
        }
        let (uid, gid, fd) = (self.assets.uid, self.assets.gid, read.as_raw_fd());
        let lease_fd = lease.as_raw_fd();
        let max_fd = unsafe { libc::getdtablesize() };
        unsafe {
            command.pre_exec(move || {
                if fd != 3 && libc::dup2(fd, 3) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(3, libc::F_SETFD, 0) < 0 || libc::setsid() < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                // FD4承接同一OS锁；必须先dup2再处理CLOEXEC，不能让helper死亡变成租约释放。
                if libc::dup2(lease_fd, 4) < 0 || libc::fcntl(4, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                // 保留 Command 的 exec-error pipe 直到 exec；只设置 CLOEXEC，不提前关闭它。
                for fd in 5..max_fd {
                    libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
                }
                if !fixture
                    && (libc::setgroups(0, std::ptr::null()) != 0
                        || libc::setgid(gid) != 0
                        || libc::setuid(uid) != 0)
                {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::geteuid() != uid
                    || libc::getuid() != uid
                    || libc::getegid() != gid
                    || uid == 0
                {
                    return Err(std::io::Error::from_raw_os_error(libc::EPERM));
                }
                Ok(())
            });
        }
        let deadline = Instant::now() + self.time_left()?;
        self.checking = Some(command.spawn().map_err(|_| SidecarPortError)?);
        #[cfg(test)]
        self.assets
            .spawned
            .lock()
            .unwrap()
            .push(self.checking.as_ref().unwrap().id());
        drop(read);
        // 大配置不能在 spawn 前填 pipe；子进程读取后并行供给，写入服从同一总预算。
        let result = (|| {
            let mut bytes = config.as_bytes();
            while !bytes.is_empty() {
                if Instant::now() >= deadline {
                    return Err(SidecarPortError);
                }
                match write.write(bytes) {
                    Ok(0) => return Err(SidecarPortError),
                    Ok(n) => bytes = &bytes[n..],
                    Err(e)
                        if matches!(
                            e.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                        ) =>
                    {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(_) => return Err(SidecarPortError),
                }
            }
            Ok(())
        })();
        drop(write);
        if result.is_err() {
            terminate(self.checking.as_mut().expect("owned spawn"))?;
            self.checking = None;
            return Err(SidecarPortError);
        }
        Ok(self.checking.take().expect("owned spawn"))
    }
    fn cache_owner(&mut self, child: bool) -> Result<(), SidecarPortError> {
        let Some((path, dev, ino)) = &self.cache else {
            return Ok(());
        };
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| SidecarPortError)?;
        let m = file.metadata().map_err(|_| SidecarPortError)?;
        if !m.is_file()
            || m.nlink() != 1
            || m.dev() != *dev
            || m.ino() != *ino
            || m.mode() & 0o7777 != 0o600
        {
            return Err(SidecarPortError);
        }
        let uid = if child {
            self.assets.uid
        } else {
            unsafe { libc::geteuid() }
        };
        if m.uid() != uid && unsafe { libc::fchown(file.as_raw_fd(), uid, self.assets.gid) } != 0 {
            return Err(SidecarPortError);
        }
        file.sync_all().map_err(|_| SidecarPortError)
    }
    fn own(&mut self, i: &ManagedSidecar) -> Result<&mut Running, SidecarPortError> {
        self.running
            .as_mut()
            .filter(|c| c.identity == *i)
            .ok_or(SidecarPortError)
    }
}
fn reader(
    mut pipe: impl Read + AsRawFd + Send + 'static,
    tx: mpsc::SyncSender<(bool, SocketAddr)>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut line = Vec::new();
        let mut chunk = [0; 1024];
        let mut sent = [false; 2];
        while !stop.load(Ordering::Acquire) {
            let mut poll = libc::pollfd {
                fd: pipe.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            if unsafe { libc::poll(&mut poll, 1, 50) } <= 0 {
                continue;
            }
            let Ok(n) = pipe.read(&mut chunk) else {
                break;
            };
            if n == 0 {
                break;
            }
            for &b in &chunk[..n] {
                if b == b'\n' {
                    if let Ok(text) = std::str::from_utf8(&line) {
                        for (index, prefix) in [
                            "inbound/mixed[mixed]: tcp server started at ",
                            "clash-api: restful api listening at ",
                        ]
                        .iter()
                        .enumerate()
                        {
                            if !sent[index]
                                && let Some((_, s)) = text.split_once(prefix)
                                && let Ok(a) = s.trim().parse::<SocketAddr>()
                                && a.ip() == std::net::Ipv4Addr::LOCALHOST
                                && a.port() != 0
                            {
                                let _ = tx.try_send((index == 0, a));
                                sent[index] = true;
                            }
                        }
                    }
                    line.clear();
                } else if line.len() < 8192 {
                    line.push(b);
                }
            }
        }
    })
}
impl SidecarPort for ProcessPort {
    fn check(&mut self, c: &GeneratedConfig) -> Result<(), SidecarPortError> {
        self.cancel_pending()?;
        c.validate_final().map_err(|_| SidecarPortError)?;
        self.checking = Some(self.spawn("check", c)?);
        let deadline = Instant::now() + self.time_left()?;
        loop {
            let child = self.checking.as_mut().unwrap();
            if let Some(s) = child.try_wait().map_err(|_| SidecarPortError)? {
                self.checking = None;
                if !s.success() {
                    return Err(SidecarPortError);
                }
                self.candidate = Some(c.clone());
                return Ok(());
            }
            if Instant::now() >= deadline {
                self.cancel_pending()?;
                return Err(SidecarPortError);
            }
            thread::sleep(Duration::from_millis(5));
        }
    }
    fn prepare(&mut self, c: &GeneratedConfig) -> Result<(), SidecarPortError> {
        self.candidate
            .as_ref()
            .filter(|v| *v == c)
            .map(|_| ())
            .ok_or(SidecarPortError)
    }
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        if self.running.is_some() {
            return Err(SidecarPortError);
        }
        let config = self.candidate.as_ref().ok_or(SidecarPortError)?;
        // 此 GeneratedConfig 只能来自本地 compiler；二次限制 cache 必须为本 owner 固定目录的直接文件。
        let value: serde_json::Value =
            serde_json::from_slice(config.as_bytes()).map_err(|_| SidecarPortError)?;
        let cache = PathBuf::from(
            value["experimental"]["cache_file"]["path"]
                .as_str()
                .ok_or(SidecarPortError)?,
        );
        if cache.parent() != Some(self.assets.root.join("kernel-cache").as_path()) {
            return Err(SidecarPortError);
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&cache)
            .map_err(|_| SidecarPortError)?;
        let m = file.metadata().map_err(|_| SidecarPortError)?;
        if m.uid() != unsafe { libc::geteuid() } {
            return Err(SidecarPortError);
        }
        self.cache = Some((cache, m.dev(), m.ino()));
        private_directory(&self.assets.root, 0o711).map_err(|_| SidecarPortError)?;
        private_directory(&self.assets.root.join("kernel-cache"), 0o711)
            .map_err(|_| SidecarPortError)?;
        self.cache_owner(true)?;
        let config = self.candidate.as_ref().unwrap().clone();
        let mut child = match self.spawn("run", &config) {
            Ok(c) => c,
            Err(e) => {
                // terminate失败时仍持有checking句柄；writer未回收前不能转移cache所有权。
                if self.checking.is_none() {
                    self.cache_owner(false)?;
                }
                return Err(e);
            }
        };
        let (tx, lines) = mpsc::sync_channel(2);
        let end_read = Arc::new(AtomicBool::new(false));
        let readers = vec![
            reader(child.stdout.take().unwrap(), tx.clone(), end_read.clone()),
            reader(child.stderr.take().unwrap(), tx, end_read.clone()),
        ];
        self.next += 1;
        let identity = ManagedSidecar::from_port_identity(self.next);
        self.running = Some(Running {
            config: self.candidate.take().unwrap(),
            child,
            identity: identity.clone(),
            lines,
            readers,
            end_read,
            controller: None,
            endpoints: None,
        });
        Ok(identity)
    }
    fn ready(&mut self, i: &ManagedSidecar) -> Result<(), SidecarPortError> {
        let deadline = Instant::now() + self.time_left()?;
        let owned = self.own(i)?;
        let config = owned.config.clone();
        let (mut mixed, mut controller) = (None, None);
        while Instant::now() < deadline {
            if owned
                .child
                .try_wait()
                .map_err(|_| SidecarPortError)?
                .is_some()
            {
                return Err(SidecarPortError);
            }
            match owned.lines.recv_timeout(Duration::from_millis(20)) {
                Ok((true, a)) => mixed = Some(a),
                Ok((false, a)) => controller = Some(a),
                _ => {}
            }
            if let (Some(mixed), Some(controller)) = (mixed, controller) {
                let endpoint = ManagedControllerEndpoint::from_owned_child(controller, &config)
                    .map_err(|_| SidecarPortError)?;
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| SidecarPortError)?;
                rt.block_on(async {
                    tokio::time::timeout(
                        deadline.saturating_duration_since(Instant::now()),
                        async {
                            let api = ClashApiClient::managed(&endpoint)?;
                            api.read_ready().await?;
                            api.read_version().await
                        },
                    )
                    .await
                    .map_err(|_| SidecarPortError)?
                    .map_err(|_| SidecarPortError)
                })?;
                if owned
                    .child
                    .try_wait()
                    .map_err(|_| SidecarPortError)?
                    .is_some()
                {
                    return Err(SidecarPortError);
                }
                owned.controller = Some(std::sync::Arc::new(endpoint));
                owned.endpoints = Some(ManagedRuntimeEndpoints { mixed, controller });
                return Ok(());
            }
        }
        Err(SidecarPortError)
    }
    fn read_selector(&mut self, i: &ManagedSidecar, tag: &str) -> Result<String, SidecarPortError> {
        let budget = self.time_left()?;
        let o = self.own(i)?;
        if o.child.try_wait().map_err(|_| SidecarPortError)?.is_some() {
            return Err(SidecarPortError);
        }
        let ep = o.controller.as_ref().ok_or(SidecarPortError)?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| SidecarPortError)?
            .block_on(async {
                tokio::time::timeout(
                    budget,
                    ClashApiClient::managed(ep)
                        .map_err(|_| SidecarPortError)?
                        .read_selector(tag),
                )
                .await
                .map_err(|_| SidecarPortError)?
                .map_err(|_| SidecarPortError)
            })
    }
    fn write_selector(
        &mut self,
        i: &ManagedSidecar,
        tag: &str,
        node: &str,
    ) -> Result<(), SidecarPortError> {
        let budget = self.time_left()?;
        let o = self.own(i)?;
        if o.child.try_wait().map_err(|_| SidecarPortError)?.is_some() {
            return Err(SidecarPortError);
        }
        let ep = o.controller.as_ref().ok_or(SidecarPortError)?;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| SidecarPortError)?
            .block_on(async {
                tokio::time::timeout(
                    budget,
                    ClashApiClient::managed(ep)
                        .map_err(|_| SidecarPortError)?
                        .write_selector(tag, node),
                )
                .await
                .map_err(|_| SidecarPortError)?
                .map_err(|_| SidecarPortError)
            })
    }
    fn endpoints(&self, i: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
        self.running
            .as_ref()
            .filter(|c| c.identity == *i)
            .and_then(|c| c.endpoints)
    }
    fn observation_endpoint(
        &self,
        i: &ManagedSidecar,
    ) -> Option<std::sync::Arc<ManagedControllerEndpoint>> {
        self.running
            .as_ref()
            .filter(|c| c.identity == *i)
            .and_then(|c| c.controller.clone())
    }
    fn is_alive(&mut self, i: &ManagedSidecar) -> Result<bool, SidecarPortError> {
        Ok(self
            .own(i)?
            .child
            .try_wait()
            .map_err(|_| SidecarPortError)?
            .is_none())
    }
    fn stop(&mut self, i: &ManagedSidecar) -> Result<(), SidecarPortError> {
        let o = self.own(i)?;
        if let Some(ep) = &o.controller {
            ep.invalidate();
        }
        o.endpoints = None;
        terminate(&mut o.child)?;
        o.end_read.store(true, Ordering::Release);
        for h in o.readers.drain(..) {
            h.join().map_err(|_| SidecarPortError)?;
        }
        self.cache_owner(false)?;
        self.running = None;
        self.cache = None;
        Ok(())
    }
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        if let Some(c) = &mut self.checking {
            terminate(c)?;
        }
        self.checking = None;
        self.candidate = None;
        if self.running.is_none() {
            self.cache_owner(false)?;
            self.cache = None;
        }
        Ok(())
    }
    fn has_pending_cleanup(&self) -> bool {
        self.checking.is_some()
    }
}
impl Drop for ProcessPort {
    fn drop(&mut self) {
        if let Some(i) = self.running.as_ref().map(|r| r.identity.clone()) {
            let _ = self.stop(&i);
        }
        let _ = self.cancel_pending();
    }
}
