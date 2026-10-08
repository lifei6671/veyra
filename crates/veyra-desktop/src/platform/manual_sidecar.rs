//! P2-03 普通用户 child owner。固定内核/参数；不接触 helper、系统代理或 TUN。
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    net::{Ipv4Addr, SocketAddr},
    os::unix::{
        fs::{OpenOptionsExt, PermissionsExt},
        process::CommandExt,
    },
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant},
};
use veyra_core::singbox::{
    GeneratedConfig,
    clash_api::{ClashApiClient, ManagedControllerEndpoint},
    runtime::{ManagedRuntimeEndpoints, ManagedSidecar, SidecarPort, SidecarPortError},
};

pub const KERNEL_SHA: &str = "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4";
const BUDGET: Duration = Duration::from_secs(10);

/// 路径只由 composition root 的开发入口提供；每次执行前重新验证固定 binary。
pub struct VerifiedKernel(PathBuf);
impl VerifiedKernel {
    pub fn development() -> Option<Self> {
        #[cfg(debug_assertions)]
        {
            std::env::var_os("VEYRA_SING_BOX_PATH")
                .map(PathBuf::from)
                .and_then(|p| Self::verify(p).ok())
        }
        #[cfg(not(debug_assertions))]
        {
            None
        } // 正式分发资源由 P0-08 决定。
    }
    fn verify(path: PathBuf) -> Result<Self, SidecarPortError> {
        let path = path.canonicalize().map_err(|_| SidecarPortError)?;
        let bytes = fs::read(&path).map_err(|_| SidecarPortError)?;
        if format!("{:x}", Sha256::digest(bytes)) != KERNEL_SHA {
            return Err(SidecarPortError);
        }
        Ok(Self(path))
    }
    fn command(&self, action: &str, path: &std::path::Path) -> Result<Command, SidecarPortError> {
        Self::verify(self.0.clone())?;
        let mut command = Command::new(&self.0);
        command
            .args([action, "-c"])
            .arg(path)
            .stdin(Stdio::null())
            .process_group(0);
        Ok(command)
    }
}
struct Candidate {
    directory: PathBuf,
    config: GeneratedConfig,
}
impl Candidate {
    fn path(&self) -> PathBuf {
        self.directory.join("config.json")
    }
}
struct OwnedChild {
    child: Child,
    identity: ManagedSidecar,
    candidate: Candidate,
    lines: Receiver<Listener>,
    readers: Vec<thread::JoinHandle<()>>,
    endpoints: Option<ManagedRuntimeEndpoints>,
    controller: Option<ManagedControllerEndpoint>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Listener {
    Mixed(SocketAddr),
    Controller(SocketAddr),
}
/// 只接受固定日志后缀及严格 IPv4 loopback；调用方仅传本 child 的两根管道。
fn parse_listener(line: &str) -> Option<Listener> {
    let (value, mixed) =
        if let Some((_, v)) = line.split_once("inbound/mixed[mixed]: tcp server started at ") {
            (v, true)
        } else {
            (
                line.split_once("clash-api: restful api listening at ")?.1,
                false,
            )
        };
    let address: SocketAddr = value.trim_end().parse().ok()?;
    if address.ip() != Ipv4Addr::LOCALHOST || address.port() == 0 {
        return None;
    }
    Some(if mixed {
        Listener::Mixed(address)
    } else {
        Listener::Controller(address)
    })
}
fn reader(
    pipe: impl std::io::Read + Send + 'static,
    sender: mpsc::Sender<Listener>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // 持续 drain 防止 pipe 堵塞；不存储/输出原始行，只发送两个监听记录。
        let mut input = BufReader::new(pipe);
        let mut bytes = Vec::new();
        loop {
            bytes.clear();
            match input.read_until(b'\n', &mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if let Ok(line) = std::str::from_utf8(&bytes)
                        && let Some(endpoint) = parse_listener(line)
                    {
                        let _ = sender.send(endpoint);
                    }
                }
            }
        }
    })
}
pub struct ManualSidecarPort {
    kernel: Option<VerifiedKernel>,
    configs: PathBuf,
    pending: Option<Candidate>,
    owned: Option<OwnedChild>,
    check_child: Option<Child>,
    next_identity: u64,
    #[cfg(test)]
    faults: std::sync::Arc<std::sync::Mutex<(bool, bool)>>,
}
impl ManualSidecarPort {
    pub fn new(root: PathBuf) -> Self {
        Self {
            kernel: VerifiedKernel::development(),
            configs: root.join("runtime/configs"),
            pending: None,
            owned: None,
            check_child: None,
            next_identity: 0,
            #[cfg(test)]
            faults: Default::default(),
        }
    }
    pub fn kernel_available(&self) -> bool {
        self.kernel.is_some()
    }
    fn remove(candidate: &Candidate) -> Result<(), SidecarPortError> {
        match fs::remove_dir_all(&candidate.directory) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(SidecarPortError),
        }
    }
    fn owned_mut(
        &mut self,
        instance: &ManagedSidecar,
    ) -> Result<&mut OwnedChild, SidecarPortError> {
        self.owned
            .as_mut()
            .filter(|c| c.identity == *instance)
            .ok_or(SidecarPortError)
    }
}
impl SidecarPort for ManualSidecarPort {
    fn check(&mut self, config: &GeneratedConfig) -> Result<(), SidecarPortError> {
        self.cancel_pending()?;
        config.validate_final().map_err(|_| SidecarPortError)?;
        fs::create_dir_all(&self.configs).map_err(|_| SidecarPortError)?;
        fs::set_permissions(&self.configs, fs::Permissions::from_mode(0o700))
            .map_err(|_| SidecarPortError)?;
        let directory = self
            .configs
            .join(format!("candidate-{}", self.next_identity + 1));
        fs::create_dir(&directory).map_err(|_| SidecarPortError)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))
            .map_err(|_| SidecarPortError)?;
        self.pending = Some(Candidate {
            directory,
            config: config.clone(),
        });
        let candidate = self.pending.as_ref().unwrap();
        let result = (|| {
            OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(candidate.path())
                .and_then(|mut f| f.write_all(config.as_bytes()))
                .map_err(|_| SidecarPortError)?;
            #[cfg(test)]
            if self.faults.lock().unwrap().0 {
                fs::write(candidate.path(), b"{invalid acceptance candidate")
                    .map_err(|_| SidecarPortError)?;
            }
            let mut command = self
                .kernel
                .as_ref()
                .ok_or(SidecarPortError)?
                .command("check", &candidate.path())?;
            self.check_child = Some(
                command
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .map_err(|_| SidecarPortError)?,
            );
            let child = self.check_child.as_mut().unwrap();
            let deadline = Instant::now() + BUDGET;
            loop {
                if let Some(status) = child.try_wait().map_err(|_| SidecarPortError)? {
                    return status.success().then_some(()).ok_or(SidecarPortError);
                }
                if Instant::now() >= deadline {
                    terminate(child)?;
                    return Err(SidecarPortError);
                }
                thread::sleep(Duration::from_millis(20));
            }
        })();
        if result.is_err() {
            self.cancel_pending()?;
        } else {
            self.check_child = None;
        }
        result
    }
    fn prepare(&mut self, config: &GeneratedConfig) -> Result<(), SidecarPortError> {
        self.pending
            .as_ref()
            .filter(|p| p.config.as_bytes() == config.as_bytes())
            .map(|_| ())
            .ok_or(SidecarPortError)
    }
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        if self.owned.is_some() {
            return Err(SidecarPortError);
        }
        let candidate = self.pending.as_ref().ok_or(SidecarPortError)?;
        let mut command = self
            .kernel
            .as_ref()
            .ok_or(SidecarPortError)?
            .command("run", &candidate.path())?;
        let mut child = match command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => child,
            Err(_) => {
                self.cancel_pending()?;
                return Err(SidecarPortError);
            }
        };
        self.next_identity += 1;
        let identity = ManagedSidecar::from_port_identity(self.next_identity);
        let (tx, lines) = mpsc::channel();
        let readers = vec![
            reader(child.stdout.take().unwrap(), tx.clone()),
            reader(child.stderr.take().unwrap(), tx),
        ];
        self.owned = Some(OwnedChild {
            child,
            identity: identity.clone(),
            candidate: self.pending.take().unwrap(),
            lines,
            readers,
            endpoints: None,
            controller: None,
        });
        Ok(identity)
    }
    fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
        #[cfg(test)]
        let suppress_discovery = self.faults.lock().unwrap().1;
        #[cfg(not(test))]
        let suppress_discovery = false;
        let owned = self.owned_mut(instance)?;
        let deadline = Instant::now()
            + if suppress_discovery {
                Duration::from_millis(200)
            } else {
                BUDGET
            };
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
            match owned.lines.recv_timeout(Duration::from_millis(25)) {
                Ok(Listener::Mixed(a)) => {
                    eprintln!("manual-runtime discovered mixed={a}");
                    mixed = Some(a);
                }
                Ok(Listener::Controller(a)) => {
                    eprintln!("manual-runtime discovered controller={a}");
                    controller = Some(a);
                }
                _ => {}
            }
            if suppress_discovery {
                continue;
            }
            if let (Some(mixed), Some(controller)) = (mixed, controller) {
                let endpoint = ManagedControllerEndpoint::from_owned_child(
                    controller,
                    &owned.candidate.config,
                )
                .map_err(|_| SidecarPortError)?;
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| SidecarPortError)?;
                let remaining = deadline.saturating_duration_since(Instant::now());
                runtime.block_on(async {
                    tokio::time::timeout(remaining, async {
                        let client = ClashApiClient::managed(&endpoint)?;
                        client.read_ready().await.inspect_err(|e| {
                            eprintln!("manual-runtime ready root failed {e:?}");
                        })?;
                        client.read_version().await.inspect_err(|e| {
                            eprintln!("manual-runtime ready version failed {e:?}");
                        })
                    })
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
                    endpoint.invalidate();
                    return Err(SidecarPortError);
                }
                owned.endpoints = Some(ManagedRuntimeEndpoints { mixed, controller });
                owned.controller = Some(endpoint);
                eprintln!(
                    "manual-runtime ready instance={} pid={} pgid={} mixed={} controller={}",
                    instance.identity(),
                    owned.child.id(),
                    owned.child.id(),
                    mixed,
                    controller
                );
                return Ok(());
            }
        }
        Err(SidecarPortError)
    }
    fn endpoints(&self, instance: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
        self.owned
            .as_ref()
            .filter(|c| c.identity == *instance)
            .and_then(|c| c.endpoints)
    }
    fn is_alive(&mut self, instance: &ManagedSidecar) -> Result<bool, SidecarPortError> {
        let owned = self.owned_mut(instance)?;
        let alive = owned
            .child
            .try_wait()
            .map_err(|_| SidecarPortError)?
            .is_none();
        if !alive {
            owned.endpoints = None;
            if let Some(e) = &owned.controller {
                e.invalidate();
            }
        }
        Ok(alive)
    }
    fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
        let owned = self.owned_mut(instance)?;
        owned.endpoints = None;
        if let Some(e) = &owned.controller {
            e.invalidate();
        }
        terminate(&mut owned.child)?;
        for reader in owned.readers.drain(..) {
            reader.join().map_err(|_| SidecarPortError)?;
        }
        Self::remove(&owned.candidate)?;
        self.owned = None;
        Ok(())
    }
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        if let Some(child) = self.check_child.as_mut() {
            terminate(child)?;
        }
        self.check_child = None;
        if let Some(candidate) = self.pending.as_ref() {
            Self::remove(candidate)?;
        }
        self.pending = None;
        Ok(())
    }
    fn has_pending_cleanup(&self) -> bool {
        self.pending.is_some() || self.check_child.is_some()
    }
}
/// 只使用持有的 Child；try_wait 后不再对已回收 PID 发信号。
fn terminate(child: &mut Child) -> Result<(), SidecarPortError> {
    if child.try_wait().map_err(|_| SidecarPortError)?.is_some() {
        return Ok(());
    }
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if child.try_wait().map_err(|_| SidecarPortError)?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().map_err(|_| SidecarPortError)?;
    child.wait().map_err(|_| SidecarPortError)?;
    Ok(())
}
impl Drop for ManualSidecarPort {
    fn drop(&mut self) {
        if let Some(identity) = self.owned.as_ref().map(|c| c.identity.clone()) {
            let _ = self.stop(&identity);
        }
        let _ = self.cancel_pending();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // 保护动态地址归属边界，不通过扫描发现端口。
    #[test]
    fn dynamic_endpoint_parser_rejects_foreign_hosts_and_invalid_ports() {
        assert_eq!(
            parse_listener(
                "INFO[0000] inbound/mixed[mixed]: tcp server started at 127.0.0.1:54321"
            ),
            Some(Listener::Mixed("127.0.0.1:54321".parse().unwrap()))
        );
        assert_eq!(
            parse_listener("INFO[0000] clash-api: restful api listening at 127.0.0.1:1234"),
            Some(Listener::Controller("127.0.0.1:1234".parse().unwrap()))
        );
        for address in [
            "0.0.0.0:99",
            "127.0.0.1:0",
            "127.0.0.1:65536",
            "example.com:99",
            "[::1]:99",
            "127.0.0.1:99 trailing",
        ] {
            assert!(
                parse_listener(&format!("clash-api: restful api listening at {address}")).is_none()
            );
        }
    }
}

#[cfg(test)]
mod real_tests {
    use super::*;
    use veyra_core::{
        application::selected_subscription::project_selected_runtime,
        domain::{AppState, OutboundId},
        singbox::{
            LoopbackListener, ManagedCacheFile, ProductCompileRequest, ProductRuntimeResources,
            SingBoxCompiler,
            runtime::{SidecarError, SidecarRuntime},
            secret::generate_api_secret,
        },
    };
    fn config(root: &std::path::Path, mixed: u16, controller: u16) -> GeneratedConfig {
        let mut state = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../veyra-core/tests/fixtures/compiler/p2-02b-selected.json"
        ))
        .unwrap();
        for (k, v) in fixture.as_object().unwrap() {
            state[k] = v.clone();
        }
        let state: AppState = serde_json::from_value(state).unwrap();
        let projection = project_selected_runtime(&state).unwrap();
        let resources = ProductRuntimeResources {
            mixed: LoopbackListener::new(([127, 0, 0, 1], mixed).into()).unwrap(),
            controller: LoopbackListener::new(([127, 0, 0, 1], controller).into()).unwrap(),
            cache: ManagedCacheFile::new(
                root,
                root.join("cache.db"),
                "real-test".into(),
                false,
                false,
            )
            .unwrap(),
        };
        SingBoxCompiler
            .compile_product(ProductCompileRequest {
                state: &state,
                runtime_intent: &projection.runtime_intent,
                default_outbound: &OutboundId::from_route_target(
                    &projection.projected_default_target,
                )
                .unwrap(),
                resources: &resources,
            })
            .unwrap()
            .finalize(&generate_api_secret().unwrap())
            .unwrap()
    }
    // 保护真实 owner 的失败清理/旧实例保留；只在显式固定内核验收时运行，绝不下载替代版本。
    #[test]
    #[ignore = "requires explicitly verified v1.14.0 VEYRA_SING_BOX_PATH"]
    fn p203_real_owned_child_failure_matrix() {
        let root = std::env::temp_dir().join(format!("veyra-p203-real-{}", std::process::id()));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let port = ManualSidecarPort::new(root.clone());
        assert!(port.kernel_available());
        let faults = port.faults.clone();
        let mut runtime = SidecarRuntime::new_dynamic(port);
        let control = std::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let control_address = control.local_addr().unwrap();
        runtime.start_or_replace(config(&root, 0, 0)).unwrap();
        let old = runtime.active_identity().unwrap();
        let endpoints = runtime.endpoints().unwrap();
        faults.lock().unwrap().0 = true;
        assert_eq!(
            runtime.start_or_replace(config(&root, 0, 0)),
            Err(SidecarError::CandidateCheck)
        );
        assert_eq!(runtime.active_identity(), Some(old));
        assert_eq!(runtime.endpoints(), Some(endpoints));
        assert!(runtime.refresh_alive().unwrap());
        faults.lock().unwrap().0 = false;
        runtime.stop().unwrap();
        assert!(std::net::TcpStream::connect(endpoints.mixed).is_err());
        assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
        for (mixed, controller) in [(control_address.port(), 0), (0, control_address.port())] {
            assert!(
                runtime
                    .start_or_replace(config(&root, mixed, controller))
                    .is_err()
            );
            assert!(runtime.endpoints().is_none());
            assert!(runtime.active_identity().is_none());
        }
        faults.lock().unwrap().1 = true;
        assert_eq!(
            runtime.start_or_replace(config(&root, 0, 0)),
            Err(SidecarError::CandidateReady)
        );
        assert!(runtime.endpoints().is_none());
        assert!(runtime.active_identity().is_none());
        faults.lock().unwrap().1 = false;
        runtime.start_or_replace(config(&root, 0, 0)).unwrap();
        let crashed = runtime.active_identity();
        let endpoints = runtime.endpoints().unwrap();
        runtime
            .with_active_port(|port, instance| {
                let owned = port.owned_mut(instance)?;
                owned.child.kill().map_err(|_| SidecarPortError)?;
                owned.child.wait().map_err(|_| SidecarPortError)?;
                Ok(())
            })
            .unwrap();
        assert!(!runtime.refresh_alive().unwrap());
        assert!(runtime.endpoints().is_none());
        assert!(std::net::TcpStream::connect(endpoints.mixed).is_err());
        assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
        runtime.start_or_replace(config(&root, 0, 0)).unwrap();
        assert_ne!(runtime.active_identity(), crashed);
        // 对照 listener 从未归 Runtime 所有，Stop 后仍可连接。
        runtime.stop().unwrap();
        assert!(std::net::TcpStream::connect(control_address).is_ok());
        let port = runtime.into_port();
        assert!(port.owned.is_none());
        assert!(port.pending.is_none());
        assert!(port.check_child.is_none());
        assert_eq!(
            fs::read_dir(root.join("runtime/configs")).unwrap().count(),
            0
        );
        drop(port);
        drop(control);
        fs::remove_dir_all(root).unwrap();
    }
}
