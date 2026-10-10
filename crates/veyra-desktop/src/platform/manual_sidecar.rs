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

pub const KERNEL_SHA: &str = veyra_core::application::runtime_recovery::KERNEL_DIGEST;
const BUDGET: Duration = Duration::from_secs(10);

/// 路径只由 composition root 的开发入口提供；每次执行前重新验证固定 binary。
pub struct VerifiedKernel(PathBuf);
impl VerifiedKernel {
    pub fn development() -> Option<Self> {
        #[cfg(debug_assertions)]
        if let Some(path) = std::env::var_os("VEYRA_SING_BOX_PATH") {
            // 明确提供但不合法的开发资源不能静默fallback另一份内核。
            return Self::verify(PathBuf::from(path)).ok();
        }
        // P0-08尚未交付bundle；只接受与实际MacOS入口相邻的固定Resources/helper资产。
        let executable = std::env::current_exe().ok()?;
        let macos = executable.parent()?;
        if macos.file_name()? != "MacOS" {
            return None;
        }
        let contents = macos.parent()?;
        if contents.file_name()? != "Contents" {
            return None;
        }
        Self::verify(
            contents
                .join("Resources/helper")
                .join(veyra_core::application::runtime_recovery::KERNEL_EXECUTABLE),
        )
        .ok()
    }
    fn verify(path: PathBuf) -> Result<Self, SidecarPortError> {
        let expected =
            std::ffi::OsStr::new(veyra_core::application::runtime_recovery::KERNEL_EXECUTABLE);
        if path.file_name() != Some(expected) {
            eprintln!("kernel unavailable: executable basename must be veyra-sing-box");
            return Err(SidecarPortError);
        }
        let path = path.canonicalize().map_err(|_| SidecarPortError)?;
        // 同名symlink指向旧sing-box也不满足实际被执行文件的basename要求。
        if path.file_name() != Some(expected) {
            eprintln!("kernel unavailable: resolved executable basename must be veyra-sing-box");
            return Err(SidecarPortError);
        }
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
    controller: Option<std::sync::Arc<ManagedControllerEndpoint>>,
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
    #[cfg(test)]
    ready_failures: std::sync::Arc<std::sync::Mutex<usize>>,
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
            #[cfg(test)]
            ready_failures: Default::default(),
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
    fn read_selector(
        &mut self,
        instance: &ManagedSidecar,
        tag: &str,
    ) -> Result<String, SidecarPortError> {
        let owned = self.owned_mut(instance)?;
        if owned
            .child
            .try_wait()
            .map_err(|_| SidecarPortError)?
            .is_some()
        {
            return Err(SidecarPortError);
        }
        let endpoint = owned.controller.as_ref().ok_or(SidecarPortError)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| SidecarPortError)?;
        runtime.block_on(async {
            tokio::time::timeout(
                BUDGET,
                ClashApiClient::managed(endpoint)
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
        instance: &ManagedSidecar,
        tag: &str,
        node: &str,
    ) -> Result<(), SidecarPortError> {
        let owned = self.owned_mut(instance)?;
        if owned
            .child
            .try_wait()
            .map_err(|_| SidecarPortError)?
            .is_some()
        {
            return Err(SidecarPortError);
        }
        let endpoint = owned.controller.as_ref().ok_or(SidecarPortError)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| SidecarPortError)?;
        runtime.block_on(async {
            tokio::time::timeout(
                BUDGET,
                ClashApiClient::managed(endpoint)
                    .map_err(|_| SidecarPortError)?
                    .write_selector(tag, node),
            )
            .await
            .map_err(|_| SidecarPortError)?
            .map_err(|_| SidecarPortError)
        })
    }
    fn check(&mut self, config: &GeneratedConfig) -> Result<(), SidecarPortError> {
        self.cancel_pending()?;
        config.validate_final().map_err(|_| SidecarPortError)?;
        // 复用平台私有目录校验；runtime/configs 任一层是symlink时，创建候选前拒绝。
        super::files::private_directory(self.configs.parent().ok_or(SidecarPortError)?)
            .map_err(|_| SidecarPortError)?;
        super::files::private_directory(&self.configs).map_err(|_| SidecarPortError)?;
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
        let suppress_discovery = {
            let mut remaining = self.ready_failures.lock().unwrap();
            let once = *remaining > 0;
            *remaining = remaining.saturating_sub(1);
            self.faults.lock().unwrap().1 || once
        };
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
                owned.controller = Some(std::sync::Arc::new(endpoint));
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
    fn observation_endpoint(
        &self,
        instance: &ManagedSidecar,
    ) -> Option<std::sync::Arc<ManagedControllerEndpoint>> {
        self.owned
            .as_ref()
            .filter(|c| c.identity == *instance)
            .and_then(|c| c.controller.clone())
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
use veyra_core::application::owned_child::terminate;
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
    } // 保护真实child的选择、关闭缓存、重新创建owner后不自动运行、两种恢复和一次Ready回退。
    // 保护 pending 事务跨真正 OS 进程边界后的恢复；只包装真实 Port，不注入生产 failpoint。
    use serde_json::{Value, json};
    use std::sync::{Arc, Mutex};
    use veyra_core::application::{
        manual_runtime::{ManualRuntime, RuntimeCommand, RuntimeError},
        runtime_recovery::{RecoveryStore, cache_generation, digest},
        state_access::StateAccessGate,
        state_service::{SelectionService, SnapshotService},
    };
    use veyra_core::domain::{NodeId, PoolId};
    use veyra_core::storage::{JsonStateStore, StateStore};

    #[derive(Clone)]
    pub(super) struct RecordingSidecarPort {
        pub(super) inner: Arc<Mutex<ManualSidecarPort>>,
        events: Arc<Mutex<Vec<Value>>>,
        pub(super) rework: Arc<Mutex<NativeReworkFaults>>,
        root: PathBuf,
    }
    impl RecordingSidecarPort {
        pub(super) fn new(root: &std::path::Path) -> Self {
            let inner = ManualSidecarPort::new(root.to_owned());
            assert!(inner.kernel_available());
            Self {
                inner: Arc::new(Mutex::new(inner)),
                events: Default::default(),
                rework: Default::default(),
                root: root.to_owned(),
            }
        }
        fn record(&self, value: Value) {
            self.events.lock().unwrap().push(value);
        }
        pub(super) fn events(&self) -> Vec<Value> {
            self.events.lock().unwrap().clone()
        }
        fn owned_identity(&self) -> ManagedSidecar {
            self.inner
                .lock()
                .unwrap()
                .owned
                .as_ref()
                .unwrap()
                .identity
                .clone()
        }
    }
    impl SidecarPort for RecordingSidecarPort {
        fn read_selector(
            &mut self,
            instance: &ManagedSidecar,
            tag: &str,
        ) -> Result<String, SidecarPortError> {
            let result = self.inner.lock().unwrap().read_selector(instance, tag);
            self.record(json!({"op":"GET", "tag":tag, "actual":result.as_ref().ok()}));
            let mutation = {
                let mut faults = self.rework.lock().unwrap();
                if faults
                    .during_get
                    .as_ref()
                    .is_some_and(|(_, expected, _)| expected == tag)
                {
                    faults.during_get.take()
                } else {
                    None
                }
            };
            if let Some((snapshots, _, replace_epoch)) = mutation {
                assert!(result.is_ok());
                let current = snapshots.snapshot().unwrap();
                if replace_epoch {
                    snapshots.replace(current.version(), current).unwrap();
                } else {
                    SelectionService::new(snapshots)
                        .begin_manual_pending(
                            current.selection_version(),
                            PoolId("manual".into()),
                            NodeId("b".into()),
                        )
                        .unwrap();
                }
                self.record(json!({"op":"injection","kind":"after-real-GET-before-return","epoch_replaced":replace_epoch}));
            }
            if std::mem::take(&mut self.rework.lock().unwrap().lose_get_result) {
                assert!(result.is_ok());
                self.record(json!({"op":"injection","kind":"discard-successful-GET-result","returned":"SidecarPortError"}));
                return Err(SidecarPortError);
            }
            result
        }
        fn write_selector(
            &mut self,
            instance: &ManagedSidecar,
            tag: &str,
            node: &str,
        ) -> Result<(), SidecarPortError> {
            let contention = self.rework.lock().unwrap().during_put.take();
            if let Some(snapshots) = contention {
                let store = JsonStateStore::new(self.root.join("state.json")).unwrap();
                let before = store.load().unwrap();
                let error = SelectionService::new(snapshots)
                    .select_manual(
                        before.selection_version(),
                        PoolId("manual".into()),
                        Some(NodeId("c".into())),
                    )
                    .unwrap_err();
                assert_eq!(
                    error.code(),
                    veyra_core::domain::AppErrorCode::StorageFailed
                );
                assert_eq!(serde_json::to_value(&error).unwrap()["detail"], "Busy");
                assert_eq!(store.load().unwrap(), before);
                self.record(json!({"op":"injection","kind":"business-write-during-controller-gate","result":error}));
            }
            let result = self
                .inner
                .lock()
                .unwrap()
                .write_selector(instance, tag, node);
            self.record(json!({"op":"PUT", "tag":tag, "requested":node, "ok":result.is_ok()}));
            result
        }
        fn check(&mut self, config: &GeneratedConfig) -> Result<(), SidecarPortError> {
            self.record(json!({"op":"check"}));
            self.inner.lock().unwrap().check(config)
        }
        fn prepare(&mut self, config: &GeneratedConfig) -> Result<(), SidecarPortError> {
            self.record(json!({"op":"prepare"}));
            self.inner.lock().unwrap().prepare(config)
        }
        fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
            if std::mem::take(&mut self.rework.lock().unwrap().fail_run) {
                self.record(json!({"op":"injection","kind":"run-error-before-spawn"}));
                return Err(SidecarPortError);
            }
            let mut port = self.inner.lock().unwrap();
            let result = port.run()?;
            let child = port.owned.as_ref().unwrap();
            let pid = child.child.id();
            // 在任何后续断言之前登记本次 spawn 的资源；父测试只可清理此 PID/PGID。
            fs::write(
                self.root.join("owned.json"),
                serde_json::to_vec(&json!({"pid":pid, "pgid":pid})).unwrap(),
            )
            .unwrap();
            self.record(json!({"op":"run", "pid":pid, "pgid":pid}));
            Ok(result)
        }
        fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
            let mut port = self.inner.lock().unwrap();
            port.ready(instance)?;
            let child = port.owned.as_ref().unwrap();
            let config: Value = serde_json::from_slice(child.candidate.config.as_bytes()).unwrap();
            assert_eq!(config["inbounds"][0]["listen_port"], 0);
            assert_eq!(
                config["experimental"]["clash_api"]["external_controller"],
                "127.0.0.1:0"
            );
            let cache = PathBuf::from(
                config["experimental"]["cache_file"]["path"]
                    .as_str()
                    .unwrap(),
            );
            let endpoints = child.endpoints.unwrap();
            let record = json!({"op":"ready", "pid":child.child.id(), "mixed":endpoints.mixed.to_string(),
                "controller":endpoints.controller.to_string(), "ports_from_owned_logs":true,
                "secret_digest":digest(config["experimental"]["clash_api"]["secret"].as_str().unwrap().as_bytes()),
                "cache_id":config["experimental"]["cache_file"]["cache_id"],
                "cache":pending_cache(&self.root, &cache, Some(child.child.id()))});
            self.record(record);
            if std::mem::take(&mut self.rework.lock().unwrap().deny_manifest_after_ready) {
                fs::set_permissions(self.root.join("runtime"), fs::Permissions::from_mode(0o500))
                    .unwrap();
                self.record(
                    json!({"op":"injection","kind":"fixture-runtime-directory-not-writable"}),
                );
            }
            if std::mem::take(&mut self.rework.lock().unwrap().fail_ready) {
                self.record(json!({"op":"injection","kind":"discard-successful-ready"}));
                return Err(SidecarPortError);
            }
            Ok(())
        }
        fn endpoints(&self, instance: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
            self.inner.lock().unwrap().endpoints(instance)
        }
        fn observation_endpoint(
            &self,
            instance: &ManagedSidecar,
        ) -> Option<Arc<ManagedControllerEndpoint>> {
            self.inner.lock().unwrap().observation_endpoint(instance)
        }
        fn is_alive(&mut self, instance: &ManagedSidecar) -> Result<bool, SidecarPortError> {
            self.inner.lock().unwrap().is_alive(instance)
        }
        fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
            let mut port = self.inner.lock().unwrap();
            let child = port.owned.as_ref().unwrap();
            let pid = child.child.id();
            let endpoints = child.endpoints;
            port.stop(instance)?;
            pending_pid_absent(pid);
            if let Some(endpoints) = endpoints {
                assert!(std::net::TcpStream::connect(endpoints.mixed).is_err());
                assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
            }
            self.record(json!({"op":"stop", "pid":pid, "reaped":true, "group_absent":true, "listeners_closed":true}));
            Ok(())
        }
        fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
            self.inner.lock().unwrap().cancel_pending()
        }
        fn has_pending_cleanup(&self) -> bool {
            self.inner.lock().unwrap().has_pending_cleanup()
        }
    }
    fn pending_pid_absent(pid: u32) {
        // 精确查询本测试拥有的 PID 与进程组；ESRCH 同时排除仍存在的 zombie。
        for id in [pid as i32, -(pid as i32)] {
            assert_eq!(
                unsafe { libc::kill(id, 0) },
                -1,
                "owned process/group still exists"
            );
            assert_eq!(
                std::io::Error::last_os_error().raw_os_error(),
                Some(libc::ESRCH)
            );
        }
    }
    fn pending_cache(root: &std::path::Path, path: &std::path::Path, writer: Option<u32>) -> Value {
        // lsof 只查询已经由本测试生成的 cache path，不扫描未知 sing-box。
        let result = Command::new("/usr/sbin/lsof")
            .args(["-t", "--"])
            .arg(path)
            .output()
            .unwrap();
        assert!(matches!(result.status.code(), Some(0 | 1)));
        let pids: std::collections::BTreeSet<u32> = String::from_utf8(result.stdout)
            .unwrap()
            .lines()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(pids, writer.into_iter().collect());
        let metadata = fs::metadata(path).unwrap();
        json!({"path":format!("<isolated-root>/{}", path.canonicalize().unwrap().strip_prefix(root.canonicalize().unwrap()).unwrap().display()),
            "size":metadata.len(), "sha256":digest(&fs::read(path).unwrap()),
            "mtime_ms":metadata.modified().unwrap().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(),
            "writer_pids":pids, "observed_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis(),
            "writer_closed":writer.is_none(),
            "writer_closed_timestamp_ms":writer.is_none().then(||std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis())})
    }
    fn pending_fixture() -> AppState {
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../veyra-core/tests/fixtures/compiler/p2-02b.json"
        ))
        .unwrap();
        for (k, v) in fixture.as_object().unwrap() {
            value[k] = v.clone();
        }
        let mut state: AppState = serde_json::from_value(value).unwrap();
        state.active_subscription_id =
            Some(veyra_core::domain::SubscriptionId("subscription".into()));
        state.pools.retain(|p| p.id.0 == "manual");
        state.routes.clear();
        for node in &mut state.nodes {
            node.server = "127.0.0.1".into();
        }
        state.config_revision = 11;
        state.validate().unwrap();
        state
    }
    // 保护正式 Runtime 的四路真实观测、断流未知/重连、替换隔离及 Stop 清理。
    // 仅显式锁定内核、自己的 loopback listener/child；不访问公网或主机代理设置。
    #[test]
    #[ignore = "requires pinned VEYRA_SING_BOX_PATH; isolated real kernel observation"]
    fn observation_real_runtime_four_streams_reconnect_replace_stop() {
        use std::io::Read;
        use std::net::{TcpListener, TcpStream};
        use veyra_core::application::observability::controller::Snapshot;
        fn wait(
            owner: &ManualRuntime<RecordingSidecarPort>,
            what: &str,
            predicate: impl Fn(&Snapshot) -> bool,
        ) -> Snapshot {
            let until = Instant::now() + Duration::from_secs(12);
            loop {
                let snapshot = owner.observation().snapshot();
                if predicate(&snapshot) {
                    return snapshot;
                }
                assert!(Instant::now() < until, "{what}: {snapshot:?}");
                thread::sleep(Duration::from_millis(50));
            }
        }
        struct Resume(u32);
        impl Drop for Resume {
            fn drop(&mut self) {
                unsafe {
                    libc::kill(self.0 as i32, libc::SIGCONT);
                }
            }
        }
        let mut state = pending_fixture();
        state.default_target = veyra_core::domain::RouteTarget::Direct;
        let root = std::env::temp_dir().join(format!("veyra-observation-{:?}", state.state_epoch));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let port = RecordingSidecarPort::new(&root);
        let access = port.inner.clone();
        let mut owner = ManualRuntime::new(port, snapshots, root.clone(), true);
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let first = owner.observation().snapshot().identity.unwrap();
        assert_eq!(
            Some(first.instance_id.clone()),
            owner.snapshot().unwrap().runtime.instance_id
        );
        wait(&owner, "three periodic streams", |s| {
            s.available[..3].iter().all(|x| *x)
        });
        // 真实代理请求只发往本测试监听器；保持连接跨过 connections 的采样周期。
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        server.set_nonblocking(true).unwrap();
        let destination = server.local_addr().unwrap();
        let mixed = owner.snapshot().unwrap().endpoints.unwrap().mixed;
        let mut client = TcpStream::connect(mixed).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            client,
            "GET http://{destination}/ HTTP/1.1\r\nHost: {destination}\r\n\r\n"
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        let mut response = loop {
            match server.accept() {
                Ok((socket, _)) => break socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < until, "own HTTP listener not reached");
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("{e}"),
            }
        };
        response
            .set_write_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        response
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 65536\r\n\r\n")
            .unwrap();
        response.write_all(&[b'x'; 32768]).unwrap();
        let mut received = [0; 8192];
        assert!(client.read(&mut received).unwrap() > 0);
        let observed = wait(&owner, "four real WS and local connection", |s| {
            s.available.iter().all(|x| *x)
                && s.sequence.iter().all(|x| *x > 0)
                && s.connection_count.is_some_and(|n| n > 0)
                && s.traffic
                    .as_ref()
                    .is_some_and(|t| t.session_download_bytes > 0)
        });
        assert!(observed.memory_bytes.is_some_and(|n| n > 0));
        assert!(owner.observation().observed().is_some());
        eprintln!("REAL_OBSERVATION four-streams {observed:?}");
        let pid = access.lock().unwrap().owned.as_ref().unwrap().child.id();
        let resume = Resume(pid);
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGSTOP) }, 0);
        let unknown = wait(&owner, "stalled metrics become unknown", |s| {
            s.available[..3].iter().all(|x| !*x)
        });
        assert!(
            unknown.memory_bytes.is_none()
                && unknown.clients.is_none()
                && unknown.traffic.as_ref().is_none_or(|t| t.interval_ms == 0)
        );
        drop(resume);
        let resumed = wait(&owner, "real metrics reconnect", |s| {
            s.available[..3].iter().all(|x| *x)
                && (0..3).all(|i| s.stream_generation[i] > observed.stream_generation[i])
        });
        assert_eq!(resumed.identity, Some(first.clone()));
        eprintln!("REAL_OBSERVATION resumed {resumed:?}");
        drop(response);
        drop(client);
        drop(server);
        owner.execute(RuntimeCommand::Restart, |_| {}).unwrap();
        let replacement = owner.observation().snapshot().identity.unwrap();
        assert_ne!(replacement.instance_id, first.instance_id);
        wait(&owner, "replacement metrics", |s| {
            s.available[..3].iter().all(|x| *x)
        });
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        assert!(owner.observation().snapshot().identity.is_none());
        thread::sleep(Duration::from_millis(200));
        let stopped = owner.observation().snapshot();
        assert!(stopped.identity.is_none() && stopped.available.iter().all(|x| !*x));
        assert!(stopped.clients.is_none() && stopped.logs.is_empty());
        assert!(access.lock().unwrap().owned.is_none());
        eprintln!("REAL_OBSERVATION replacement/stop cleared and reaped");
        drop(owner);
        drop(access);
        fs::remove_dir_all(root).unwrap();
    }

    // P3-04：保护正式 owned Runtime 的双向区间字节、稳定连接下载增量、连接结束、换实例、落盘/重建和退出资源归属。
    #[test]
    #[ignore = "requires pinned VEYRA_SING_BOX_PATH; real WS to SQLite statistics"]
    fn traffic_wiring_real_runtime_sqlite_replace_rebuild_cleanup() {
        use std::io::Read;
        use std::net::{TcpListener, TcpStream};
        use veyra_core::application::observability::controller::Snapshot;
        use veyra_core::storage::traffic::TrafficWriter;
        fn wait(
            owner: &ManualRuntime<RecordingSidecarPort>,
            what: &str,
            predicate: impl Fn(&Snapshot) -> bool,
        ) -> Snapshot {
            let until = Instant::now() + Duration::from_secs(12);
            loop {
                let snapshot = owner.observation().snapshot();
                if predicate(&snapshot) {
                    return snapshot;
                }
                assert!(Instant::now() < until, "{what}: {snapshot:?}");
                thread::sleep(Duration::from_millis(30));
            }
        }
        let mut state = pending_fixture();
        state.default_target = veyra_core::domain::RouteTarget::Direct;
        let root = std::env::temp_dir().join(format!("veyra-traffic-real-{:?}", state.state_epoch));
        let traffic_root = root.join("traffic");
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let port = RecordingSidecarPort::new(&root);
        let access = port.inner.clone();
        let mut owner = ManualRuntime::new(port, snapshots.clone(), root.clone(), true);
        owner.enable_traffic_storage(traffic_root.clone());
        assert!(
            !traffic_root.exists(),
            "statistics must wait for formal Ready"
        );
        owner.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let first = owner.observation().snapshot().identity.unwrap();
        assert!(owner.observation().traffic_status().active);
        wait(&owner, "periodic WS", |s| {
            s.available[..3].iter().all(|v| *v)
        });
        let mixed = owner.snapshot().unwrap().endpoints.unwrap().mixed;
        let server = TcpListener::bind("127.0.0.1:0").unwrap();
        server.set_nonblocking(true).unwrap();
        let destination = server.local_addr().unwrap();
        let mut client = TcpStream::connect(mixed).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        client
            .set_write_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        write!(
            client,
            "GET http://{destination}/ HTTP/1.1\r\nHost: {destination}\r\n\r\n"
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(4);
        let mut response = loop {
            match server.accept() {
                Ok((socket, _)) => break socket,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(Instant::now() < until);
                    thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("{e}"),
            }
        };
        response
            .set_read_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        response
            .set_write_timeout(Some(Duration::from_secs(4)))
            .unwrap();
        response
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 32768\r\n\r\n")
            .unwrap();
        response.write_all(&vec![b'd'; 8192]).unwrap();
        let mut header = Vec::new();
        while !header.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            client.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
            assert!(header.len() < 4096);
        }
        assert!(String::from_utf8(header).unwrap().contains("200"));
        let mut received = vec![0; 8192];
        client.read_exact(&mut received).unwrap();
        assert!(received.iter().all(|b| *b == b'd'));
        let baseline = wait(&owner, "first stable connection baseline", |s| {
            s.connection_records.as_ref().is_some_and(|b| {
                b.records.iter().any(|r| {
                    r.upload.is_some_and(|n| n > 0) && r.download.is_some_and(|n| n >= 8192)
                })
            })
        });
        let record = baseline
            .connection_records
            .unwrap()
            .records
            .into_iter()
            .find(|r| r.id.is_some())
            .unwrap();
        let connection_id = record.id.unwrap();
        assert_eq!(record.dimensions.client.as_deref(), Some("127.0.0.1"));
        assert!(record.dimensions.node.is_some());
        assert_eq!(
            record.dimensions.host, None,
            "IP destination must not be invented as host"
        );
        assert_eq!(record.dimensions.direct, None, "API has no outbound type");
        response.write_all(&vec![b'd'; 16384]).unwrap();
        let mut received = vec![0; 16384];
        client.read_exact(&mut received).unwrap();
        assert!(received.iter().all(|b| *b == b'd'));
        wait(&owner, "second confirmed cumulative sample", |s| {
            s.connection_records.as_ref().is_some_and(|b| {
                b.records.iter().any(|r| {
                    r.id.as_deref() == Some(&connection_id)
                        && r.upload.is_some_and(|n| n > 0)
                        && r.download.is_some_and(|n| n >= 24576)
                })
            })
        });
        drop(response);
        drop(client);
        drop(server);
        wait(&owner, "connection ended", |s| {
            s.connection_count == Some(0)
        });
        owner.execute(RuntimeCommand::Restart, |_| {}).unwrap();
        let replacement = owner.observation().snapshot().identity.unwrap();
        assert_ne!(first.instance_id, replacement.instance_id);
        assert!(replacement.generation > first.generation);
        assert!(owner.observation().traffic_status().failure.is_none());
        wait(&owner, "replacement intervals", |s| s.sequence[0] >= 2);
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        assert!(!owner.observation().traffic_status().active);
        assert!(owner.observation().traffic_status().failure.is_none());
        assert!(access.lock().unwrap().owned.is_none());
        assert!(TcpStream::connect(mixed).is_err());
        let read = || {
            let db = rusqlite::Connection::open_with_flags(
                traffic_root.join("traffic.sqlite3"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            let total: (i64, i64) = db
                .query_row(
                    "SELECT SUM(up),SUM(down) FROM detail WHERE instance=? AND kind='partial'",
                    [&first.instance_id.0],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            let observed_sources: i64 = db
                .query_row(
                    "SELECT COUNT(DISTINCT instance) FROM detail WHERE kind='observed'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let conns: i64 = db
                .query_row(
                    "SELECT SUM(connections) FROM daily WHERE kind IN ('baseline','partial')",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            let direct: i64 = db
                .query_row(
                    "SELECT COUNT(*) FROM detail WHERE direct IS NOT NULL",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(direct, 0);
            assert_eq!(conns, 1);
            assert_eq!(observed_sources, 2);
            assert_eq!(total, (0, 16384));
            let observed: (i64, i64) = db
                .query_row(
                    "SELECT SUM(up),SUM(down) FROM detail WHERE kind='observed'",
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert!(
                observed.0 > 0 && observed.1 >= 24576,
                "actual interval bytes={observed:?}"
            );
            total
        };
        let saved = read();
        // 重建 SQLite writer（真实读取 schema/checkpoint/retention），不复制内存累计值。
        TrafficWriter::open(&traffic_root, "UTC".parse().unwrap())
            .unwrap()
            .close()
            .unwrap();
        drop(owner);
        drop(access);
        let port = RecordingSidecarPort::new(&root);
        let access = port.inner.clone();
        let mut reconstructed = ManualRuntime::new(port, snapshots, root.clone(), true);
        reconstructed.enable_traffic_storage(traffic_root.clone());
        assert_eq!(read(), saved);
        reconstructed
            .execute(RuntimeCommand::Start, |_| {})
            .unwrap();
        let final_mixed = reconstructed.snapshot().unwrap().endpoints.unwrap().mixed;
        wait(&reconstructed, "reconstructed WS", |s| s.sequence[0] >= 2);
        // 与 RuntimeService Quit 一致的正式 Stop，然后 owner Drop。
        reconstructed.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        drop(reconstructed);
        assert!(access.lock().unwrap().owned.is_none());
        assert!(TcpStream::connect(final_mixed).is_err());
        TrafficWriter::open(&traffic_root, "UTC".parse().unwrap())
            .unwrap()
            .close()
            .unwrap();
        assert!(!traffic_root.join("traffic.sqlite3-wal").exists());
        assert!(!traffic_root.join("traffic.sqlite3-shm").exists());
        eprintln!(
            "P3_04_REAL PASS up/down_delta={saved:?}; connection end; two instance sources; SQLite writer/Runtime reconstructed; Stop/reap/ports/lease released; root={root:?}"
        );
        drop(access);
        fs::remove_dir_all(root).unwrap();
    }

    fn pending_disk(store: &JsonStateStore) -> Value {
        // 从 state.json 重新加载，而不是把内存快照当作落盘证据。
        let state = store.load().unwrap();
        json!({"config":state.config_version(), "selection":state.selection_version(),
            "pools":state.pools.iter().map(|p| json!({"id":p.id,"selection":p.selection})).collect::<Vec<_>>()})
    }
    fn pending_emit(event: &str, value: Value) {
        eprintln!("P204_EVENT {}", json!({"event":event,"value":value}));
    }
    fn pending_closed(root: &std::path::Path, state: &AppState) -> Value {
        assert_eq!(
            fs::read_dir(root.join("runtime/configs")).unwrap().count(),
            0
        );
        let path = RecoveryStore::new(root)
            .unwrap()
            .cache_path(&cache_generation(&state.state_epoch))
            .unwrap();
        pending_cache(root, &path, None)
    }
    fn pending_stage_root() -> PathBuf {
        // stage 只接收父测试生成的随机 token，不接收任意路径/PID/命令。
        let token = std::env::var("VEYRA_P204_PENDING_TOKEN").expect("parent test token");
        assert_eq!(token.len(), 64);
        assert!(token.bytes().all(|b| b.is_ascii_hexdigit()));
        let root = std::env::temp_dir().join(format!("veyra-p204-pending-{token}"));
        assert!(
            !fs::symlink_metadata(&root)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(
            fs::read_to_string(root.join("parent")).unwrap(),
            unsafe { libc::getppid() }.to_string()
        );
        root
    }
    // 测试进程阶段：正式 Apply/Stop 产生封闭 recovery 材料，pending 只由正式业务服务保存。
    #[test]
    #[ignore = "isolated subprocess stage of pending native acceptance"]
    fn p204_pending_process_stage() {
        let root = pending_stage_root();
        let actual = std::env::var("VEYRA_P204_PENDING_ACTUAL").unwrap();
        assert!(["a", "b", "c"].contains(&actual.as_str()));
        let command = match std::env::var("VEYRA_P204_PENDING_COMMAND")
            .unwrap()
            .as_str()
        {
            "apply" => RuntimeCommand::ApplySaved,
            "restore" => RuntimeCommand::RestoreLastSuccessful,
            _ => panic!("fixed command enum required"),
        };
        let stage = std::env::var("VEYRA_P204_PENDING_STAGE").unwrap();
        assert!(["setup", "restart"].contains(&stage.as_str()));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        if stage == "setup" {
            store.save(&pending_fixture()).unwrap();
        }
        let state = store.load().unwrap();
        let snapshots = Arc::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let records = RecoveryStore::new(&root).unwrap();
        if stage == "restart" {
            let closed = pending_closed(&root, &state);
            let previous: Value =
                serde_json::from_slice(&fs::read(root.join("closed.json")).unwrap()).unwrap();
            for field in ["path", "size", "sha256", "mtime_ms"] {
                assert_eq!(
                    closed[field], previous[field],
                    "closed cache unchanged across OS process boundary"
                );
            }
            pending_emit("cache-before-restart", closed);
            let setup: Value =
                serde_json::from_slice(&fs::read(root.join("setup.json")).unwrap()).unwrap();
            assert_ne!(setup["stage_pid"], std::process::id());
            pending_pid_absent(setup["child_pid"].as_u64().unwrap() as u32);
        }
        let mut port = RecordingSidecarPort::new(&root);
        let mut owner = ManualRuntime::new(port.clone(), snapshots.clone(), root.clone(), true);
        assert!(owner.snapshot().unwrap().runtime.applied_version.is_none());
        if stage == "setup" {
            owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
            let ready = owner.snapshot().unwrap();
            let manifest = records.read_manifest().unwrap();
            // 使用正式 compiler index 定位 tag，不把显示名/手写 JSON 当恢复 plan。
            let generation = cache_generation(&state.state_epoch);
            let path = records.cache_path(&generation).unwrap();
            let resources = ProductRuntimeResources {
                mixed: LoopbackListener::new(([127, 0, 0, 1], 0).into()).unwrap(),
                controller: LoopbackListener::new(([127, 0, 0, 1], 0).into()).unwrap(),
                cache: ManagedCacheFile::new(
                    path.parent().unwrap(),
                    path.clone(),
                    generation,
                    false,
                    false,
                )
                .unwrap(),
            };
            let projection = project_selected_runtime(&state).unwrap();
            let plan = SingBoxCompiler
                .compile_product(ProductCompileRequest {
                    state: &state,
                    runtime_intent: &projection.runtime_intent,
                    default_outbound: &OutboundId::from_route_target(
                        &projection.projected_default_target,
                    )
                    .unwrap(),
                    resources: &resources,
                })
                .unwrap();
            let pool = &plan.artifact_index().unwrap().pools[&PoolId("manual".into())];
            let identity = port.owned_identity();
            assert_eq!(
                port.read_selector(&identity, &pool.runtime_tag).unwrap(),
                pool.members[&NodeId("a".into())]
            );
            let old = pending_disk(&store);
            let begun = SelectionService::new((*snapshots).clone())
                .begin_manual_pending(
                    state.selection_version(),
                    PoolId("manual".into()),
                    NodeId("b".into()),
                )
                .unwrap();
            assert_eq!(begun.version.0.revision, state.selection_revision + 1);
            assert_eq!(
                store.load().unwrap().config_version(),
                state.config_version()
            );
            let pending = pending_disk(&store);
            assert_eq!(pending["pools"][0]["selection"]["selected_node_id"], "a");
            assert_eq!(pending["pools"][0]["selection"]["pending_node_id"], "b");
            if actual != "a" {
                port.write_selector(
                    &identity,
                    &pool.runtime_tag,
                    &pool.members[&NodeId(actual.clone())],
                )
                .unwrap();
            }
            assert_eq!(
                port.read_selector(&identity, &pool.runtime_tag).unwrap(),
                pool.members[&NodeId(actual.clone())]
            );
            assert_eq!(pending_disk(&store), pending);
            let setup_ready = port
                .events()
                .into_iter()
                .find(|v| v["op"] == "ready")
                .unwrap();
            let setup = json!({"stage_pid":std::process::id(),"child_pid":setup_ready["pid"],"instance":ready.runtime.instance_id.unwrap().0,
                "actual":actual,"command":format!("{command:?}"),"old":old,"pending":pending,"ready":setup_ready,
                "before_interrupt_cache":pending_cache(&root,&path,Some(setup_ready["pid"].as_u64().unwrap() as u32)),
                "expected_tag":pool.members[&NodeId(actual.clone())],"manifest_confirmed":manifest.confirmed_selection});
            pending_emit("before-interrupt", setup.clone());
            // 不调用 confirm；Stop 仅关闭 writer 并快照实际 cache，manifest 仍为 confirmed=a。
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            assert_eq!(pending_disk(&store), pending);
            assert_eq!(
                records.read_manifest().unwrap().confirmed_selection.nodes
                    [&PoolId("manual".into())],
                NodeId("a".into())
            );
            let closed = pending_closed(&root, &state);
            fs::write(root.join("setup.json"), serde_json::to_vec(&setup).unwrap()).unwrap();
            fs::write(
                root.join("closed.json"),
                serde_json::to_vec(&closed).unwrap(),
            )
            .unwrap();
            pending_emit(
                "setup-closed",
                json!({"cache":closed,"events":port.events(),"manifest":records.read_manifest().unwrap()}),
            );
        } else {
            let setup: Value =
                serde_json::from_slice(&fs::read(root.join("setup.json")).unwrap()).unwrap();
            let closed: Value =
                serde_json::from_slice(&fs::read(root.join("closed.json")).unwrap()).unwrap();
            let before = pending_disk(&store);
            let mut published_ready = false;
            let outcome = owner.execute(command, |s| {
                published_ready |= s.runtime.applied_version.is_some();
            });
            if actual == "c" {
                assert_eq!(outcome, Err(RuntimeError::SelectionPending));
            } else {
                outcome.unwrap();
            }
            let events = port.events();
            let reads: Vec<_> = events.iter().filter(|v| v["op"] == "GET").collect();
            assert!(!reads.is_empty());
            assert_eq!(reads[0]["actual"], setup["expected_tag"]);
            assert!(!events.iter().any(|v| v["op"] == "PUT"));
            let fresh = events.iter().find(|v| v["op"] == "ready").unwrap();
            assert_ne!(fresh["secret_digest"], setup["ready"]["secret_digest"]);
            assert_eq!(fresh["cache_id"], setup["ready"]["cache_id"]);
            assert_eq!(fresh["cache"]["path"], closed["path"]);
            let snapshot = owner.snapshot().unwrap();
            let after = pending_disk(&store);
            let manifest = records.read_manifest().unwrap();
            assert_eq!(after["config"], before["config"]);
            if actual == "c" {
                assert_eq!(after, before);
                assert!(snapshot.runtime.applied_version.is_none());
                assert!(!published_ready);
                assert!(snapshot.endpoints.is_none());
                assert_eq!(snapshot.pending_selections.len(), 1);
                assert_eq!(
                    manifest.confirmed_selection.nodes[&PoolId("manual".into())],
                    NodeId("a".into())
                );
                assert_eq!(
                    manifest.confirmed_selection.version.0.revision,
                    state.selection_revision - 1
                );
            } else {
                assert_eq!(
                    store.load().unwrap().selection_revision,
                    state.selection_revision + 1
                );
                assert_eq!(after["pools"][0]["selection"]["selected_node_id"], actual);
                assert!(after["pools"][0]["selection"]["pending_node_id"].is_null());
                assert_eq!(
                    snapshot.runtime.applied_version.as_ref().unwrap().0,
                    state.config_version().0
                );
                assert!(snapshot.pending_selections.is_empty());
                assert_ne!(
                    json!(snapshot.runtime.instance_id.as_ref().unwrap().0),
                    setup["instance"]
                );
                assert_eq!(
                    manifest.confirmed_selection.nodes[&PoolId("manual".into())],
                    NodeId(actual.clone())
                );
                assert_eq!(
                    manifest.confirmed_selection.version,
                    store.load().unwrap().selection_version()
                );
                assert_eq!(
                    manifest.selection_at_apply,
                    store.load().unwrap().selection_version()
                );
            }
            pending_emit(
                "restart-result",
                json!({"stage_pid":std::process::id(),"actual":actual,"command":format!("{command:?}"),
                "before":before,"after":after,"outcome":format!("{outcome:?}"),"applied":snapshot.runtime.applied_version,
                "instance":snapshot.runtime.instance_id.map(|i|i.0),"events":events,"manifest":manifest,"startup_put_count":0}),
            );
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            pending_emit(
                "restart-closed",
                json!({"cache":pending_closed(&root,&state),"events":port.events()}),
            );
        }
        // 返回后 test executable 退出；下一 owner 只能在父测试 wait/reap 后的新进程创建。
    }
    fn pending_driver(actual: &str) {
        for command in ["apply", "restore"] {
            let token = digest(&serde_json::to_vec(&AppState::empty().state_epoch).unwrap());
            let root = std::env::temp_dir().join(format!("veyra-p204-pending-{token}"));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            fs::write(root.join("parent"), std::process::id().to_string()).unwrap();
            let mut stage_pids = Vec::new();
            for stage in ["setup", "restart"] {
                if stage == "restart" {
                    fs::remove_file(root.join("owned.json")).unwrap();
                }
                let mut process = Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "platform::manual_sidecar::real_tests::p204_pending_process_stage",
                        "--ignored",
                        "--nocapture",
                        "--test-threads=1",
                    ])
                    .env("VEYRA_P204_PENDING_TOKEN", &token)
                    .env("VEYRA_P204_PENDING_ACTUAL", actual)
                    .env("VEYRA_P204_PENDING_COMMAND", command)
                    .env("VEYRA_P204_PENDING_STAGE", stage)
                    .spawn()
                    .unwrap();
                let pid = process.id();
                stage_pids.push(pid);
                let deadline = Instant::now() + Duration::from_secs(90);
                let status = loop {
                    if let Some(status) = process.try_wait().unwrap() {
                        break status;
                    }
                    if Instant::now() >= deadline {
                        // 只查询登记 PID，核对它仍执行本 root 的候选后才发信号，避免陈旧 PID 误杀。
                        if let Ok(bytes) = fs::read(root.join("owned.json")) {
                            let owned: Value = serde_json::from_slice(&bytes).unwrap();
                            let child = owned["pid"].as_i64().unwrap() as i32;
                            let output = Command::new("/bin/ps")
                                .args(["-p", &child.to_string(), "-o", "command="])
                                .output()
                                .unwrap();
                            let command = String::from_utf8(output.stdout).unwrap();
                            if command.contains(root.file_name().unwrap().to_str().unwrap())
                                && command.contains("/runtime/configs/candidate-")
                            {
                                unsafe {
                                    libc::kill(-child, libc::SIGTERM);
                                }
                                let cleanup_deadline = Instant::now() + Duration::from_secs(3);
                                while unsafe { libc::kill(child, 0) } == 0
                                    && Instant::now() < cleanup_deadline
                                {
                                    thread::sleep(Duration::from_millis(20));
                                }
                                if unsafe { libc::kill(child, 0) } == 0 {
                                    unsafe {
                                        libc::kill(-child, libc::SIGKILL);
                                    }
                                }
                            }
                        }
                        if process.try_wait().unwrap().is_none() {
                            process.kill().unwrap();
                        }
                        process.wait().unwrap();
                        panic!("pending stage exceeded 90s; isolated root token={token}");
                    }
                    thread::sleep(Duration::from_millis(20));
                };
                assert!(
                    status.success(),
                    "pending stage failed; isolated root token={token}"
                );
                assert_eq!(unsafe { libc::kill(pid as i32, 0) }, -1);
                let state = JsonStateStore::new(root.join("state.json"))
                    .unwrap()
                    .load()
                    .unwrap();
                let owned: Value =
                    serde_json::from_slice(&fs::read(root.join("owned.json")).unwrap()).unwrap();
                pending_pid_absent(owned["pid"].as_u64().unwrap() as u32);
                pending_emit(
                    "stage-reaped",
                    json!({"stage":stage,"stage_pid":pid,"command":command,"actual":actual,
                    "root":format!("<temp>/veyra-p204-pending-{token}"),"cache":pending_closed(&root,&state)}),
                );
            }
            assert_ne!(stage_pids[0], stage_pids[1]);
            fs::remove_dir_all(&root).unwrap();
            pending_emit(
                "root-cleanup",
                json!({"actual":actual,"command":command,"stage_pids":stage_pids,"removed":!root.exists()}),
            );
        }
    }
    #[test]
    #[ignore = "requires explicitly verified v1.14.0; protects old actual clear across OS restart"]
    fn p204_pending_native_old() {
        pending_driver("a");
    }
    #[test]
    #[ignore = "requires explicitly verified v1.14.0; protects pending actual confirm across OS restart"]
    fn p204_pending_native_requested() {
        pending_driver("b");
    }
    #[test]
    #[ignore = "requires explicitly verified v1.14.0; protects third actual no PUT/no Ready across OS restart"]
    fn p204_pending_native_third() {
        pending_driver("c");
    }

    // 保护旧 plan 未覆盖 pending 的 preflight：真实 active child 不能被停止或替换，也不能部分 CAS。
    #[test]
    #[ignore = "requires explicitly verified v1.14.0; protects uncovered pending before replacing owned child"]
    fn p204_pending_native_uncovered_preflight() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p204-preflight-{}",
            digest(&serde_json::to_vec(&AppState::empty().state_epoch).unwrap())
        ));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&pending_fixture()).unwrap();
        let snapshots = Arc::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let port = RecordingSidecarPort::new(&root);
        let mut owner = ManualRuntime::new(port.clone(), snapshots.clone(), root.clone(), true);
        owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        owner
            .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
            .unwrap();
        let before = owner.snapshot().unwrap();
        let mut state = store.load().unwrap();
        let mut pool = state.pools[0].clone();
        pool.id = PoolId("new-manual".into());
        state.pools.push(pool);
        let saved = store.commit(&state).unwrap();
        let service = SelectionService::new((*snapshots).clone());
        service
            .begin_manual_pending(
                saved.selection_version(),
                PoolId("new-manual".into()),
                NodeId("b".into()),
            )
            .unwrap();
        for count in [1, 2] {
            if count == 2 {
                service
                    .begin_manual_pending(
                        store.load().unwrap().selection_version(),
                        PoolId("manual".into()),
                        NodeId("b".into()),
                    )
                    .unwrap();
            }
            let disk = pending_disk(&store);
            let events = port.events();
            let pid = port
                .inner
                .lock()
                .unwrap()
                .owned
                .as_ref()
                .unwrap()
                .child
                .id();
            assert_eq!(
                owner.execute(RuntimeCommand::RestoreLastSuccessful, |_| panic!(
                    "preflight must not publish candidate"
                )),
                Err(RuntimeError::SelectionPending)
            );
            assert_eq!(port.events(), events, "no check/prepare/stop/run/GET/PUT");
            assert_eq!(pending_disk(&store), disk, "no partial CAS");
            let after = owner.snapshot().unwrap();
            assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
            assert_eq!(
                after.runtime.applied_version,
                before.runtime.applied_version
            );
            assert_eq!(after.endpoints, before.endpoints);
            assert_eq!(after.pending_selections.len(), count);
            assert_eq!(
                port.inner
                    .lock()
                    .unwrap()
                    .owned
                    .as_ref()
                    .unwrap()
                    .child
                    .id(),
                pid
            );
            let endpoints = after.endpoints.unwrap();
            assert!(std::net::TcpStream::connect(endpoints.mixed).is_ok());
            assert!(std::net::TcpStream::connect(endpoints.controller).is_ok());
            pending_emit(
                "uncovered-preflight",
                json!({"pending_count":count,"pid":pid,"instance":after.runtime.instance_id.map(|i|i.0),
                "mixed":endpoints.mixed.to_string(),"controller":endpoints.controller.to_string(),"disk":disk,
                "GET":0,"PUT":0,"stop":0,"run":0,"check":0,"partial_cas":0,"listeners_alive":true}),
            );
        }
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        pending_emit(
            "preflight-cleanup",
            json!({"cache":pending_closed(&root,&state),"events":port.events()}),
        );
        drop(owner);
        drop(port);
        fs::remove_dir_all(&root).unwrap();
        pending_emit("preflight-root-removed", json!({"removed":!root.exists()}));
    }

    // Host P1/P2 只在 test Port 中调度错误/并发窗口，真实 HTTP 始终委托正式 Port。
    #[derive(Default)]
    pub(super) struct NativeReworkFaults {
        deny_manifest_after_ready: bool,
        pub(super) lose_get_result: bool,
        fail_run: bool,
        fail_ready: bool,
        during_get: Option<(SnapshotService, String, bool)>,
        during_put: Option<SnapshotService>,
    }

    fn rework_plan(state: &AppState, root: &std::path::Path) -> veyra_core::singbox::SingBoxPlan {
        let generation = cache_generation(&state.state_epoch);
        let path = RecoveryStore::new(root)
            .unwrap()
            .cache_path(&generation)
            .unwrap();
        let resources = ProductRuntimeResources {
            mixed: LoopbackListener::new(([127, 0, 0, 1], 0).into()).unwrap(),
            controller: LoopbackListener::new(([127, 0, 0, 1], 0).into()).unwrap(),
            cache: ManagedCacheFile::new(
                path.parent().unwrap(),
                path.clone(),
                generation,
                false,
                false,
            )
            .unwrap(),
        };
        let projection = project_selected_runtime(state).unwrap();
        SingBoxCompiler
            .compile_product(ProductCompileRequest {
                state,
                runtime_intent: &projection.runtime_intent,
                default_outbound: &OutboundId::from_route_target(
                    &projection.projected_default_target,
                )
                .unwrap(),
                resources: &resources,
            })
            .unwrap()
    }
    fn rework_root(case: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "veyra-p204-rework-{case}-{}",
            digest(&serde_json::to_vec(&AppState::empty().state_epoch).unwrap())
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        pending_emit(
            "rework-root",
            json!({"root":format!("<temp>/{}",root.file_name().unwrap().to_str().unwrap()),"test_pid":std::process::id()}),
        );
        root
    }
    fn rework_manifest(root: &std::path::Path) -> Value {
        serde_json::to_value(RecoveryStore::new(root).unwrap().read_manifest().unwrap()).unwrap()
    }
    fn rework_cleanup(root: &std::path::Path, state: &AppState, port: &RecordingSidecarPort) {
        assert!(port.inner.lock().unwrap().owned.is_none());
        assert!(!port.has_pending_cleanup());
        pending_emit(
            "rework-cleanup",
            json!({"root":format!("<temp>/{}",root.file_name().unwrap().to_str().unwrap()),
            "cache":pending_closed(root,state),"events":port.events()}),
        );
        fs::remove_dir_all(root).unwrap();
        assert!(!root.exists());
    }
    // P1：真实 v12 child + 真实 b/cache；三种失败属于显式测试注入，不冒充内核自然故障。
    #[test]
    #[ignore = "fixed kernel Native with explicit filesystem/read/run-ready fault injection"]
    fn p204_rework_native_failed_candidate_preserves_pending() {
        use veyra_core::application::manual_runtime::{ManualSelectionRequest, SelectionError};
        for ready_failure in [false, true] {
            let root = rework_root(if ready_failure { "p1-ready" } else { "p1-run" });
            let store = JsonStateStore::new(root.join("state.json")).unwrap();
            let initial = pending_fixture();
            store.save(&initial).unwrap();
            let snapshots = Arc::new(SnapshotService::new(
                store.clone(),
                StateAccessGate::default(),
            ));
            let mut port = RecordingSidecarPort::new(&root);
            let mut owner = ManualRuntime::new(port.clone(), snapshots, root.clone(), true);
            owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            let records = RecoveryStore::new(&root).unwrap();
            let v11 = records.read_manifest().unwrap();
            assert_eq!(v11.config.0.revision, 11);
            assert_eq!(v11.confirmed_selection.version.0.revision, 0);
            let v11_cache = records.artifact(v11.cache.as_ref().unwrap()).unwrap();
            let mut next = store.load().unwrap();
            next.profile.reject_quic = !next.profile.reject_quic;
            let saved = store.commit(&next).unwrap();
            assert_eq!(saved.config_revision, 12);
            port.rework.lock().unwrap().deny_manifest_after_ready = true;
            let result = owner.execute(RuntimeCommand::ApplySaved, |_| {});
            // 只更改本 fixture runtime 目录权限；及时恢复以便后续真实 Stop 清理候选。
            fs::set_permissions(root.join("runtime"), fs::Permissions::from_mode(0o700)).unwrap();
            assert_eq!(result, Err(RuntimeError::RecoveryRecordFailed));
            let ready = owner.snapshot().unwrap();
            assert_eq!(
                ready.runtime.applied_version.as_ref().unwrap().0.revision,
                12
            );
            assert_eq!(
                ready
                    .runtime
                    .last_successful_version
                    .as_ref()
                    .unwrap()
                    .0
                    .revision,
                11
            );
            assert_eq!(rework_manifest(&root), serde_json::to_value(&v11).unwrap());
            port.rework.lock().unwrap().lose_get_result = true;
            assert_eq!(
                owner.select_manual(ManualSelectionRequest {
                    config: ready.runtime.applied_version.clone().unwrap(),
                    instance: ready.runtime.instance_id.clone().unwrap(),
                    expected: store.load().unwrap().selection_version(),
                    pool: PoolId("manual".into()),
                    node: NodeId("b".into())
                }),
                Err(SelectionError::ControllerReadBack)
            );
            let pending = store.load().unwrap();
            assert_eq!(
                (pending.config_revision, pending.selection_revision),
                (12, 1)
            );
            assert_eq!(pending.state_epoch, initial.state_epoch);
            let disk = pending_disk(&store);
            assert_eq!(disk["pools"][0]["selection"]["selected_node_id"], "a");
            assert_eq!(disk["pools"][0]["selection"]["pending_node_id"], "b");
            let plan = rework_plan(&pending, &root);
            let pool = &plan.artifact_index().unwrap().pools[&PoolId("manual".into())];
            let instance = port.owned_identity();
            assert_eq!(
                port.read_selector(&instance, &pool.runtime_tag).unwrap(),
                pool.members[&NodeId("b".into())]
            );
            let cache = records
                .cache_path(&cache_generation(&initial.state_epoch))
                .unwrap();
            let pid = port
                .inner
                .lock()
                .unwrap()
                .owned
                .as_ref()
                .unwrap()
                .child
                .id();
            let live = pending_cache(&root, &cache, Some(pid));
            let before_events = port.events();
            let before_manifest = rework_manifest(&root);
            // active v12 与 v11 plan 来源不同：任何 prepare/stop/GET/PUT 之前拒绝。
            assert_eq!(
                owner.execute(RuntimeCommand::RestoreLastSuccessful, |_| panic!(
                    "preflight published candidate"
                )),
                Err(RuntimeError::SelectionPending)
            );
            assert_eq!(port.events(), before_events);
            assert_eq!(pending_disk(&store), disk);
            let after = owner.snapshot().unwrap();
            assert_eq!(after.runtime.instance_id, ready.runtime.instance_id);
            assert_eq!(after.endpoints, ready.endpoints);
            assert_eq!(after.runtime.applied_version, ready.runtime.applied_version);
            let endpoints = after.endpoints.unwrap();
            assert!(std::net::TcpStream::connect(endpoints.mixed).is_ok());
            assert!(std::net::TcpStream::connect(endpoints.controller).is_ok());
            assert_eq!(
                pid,
                port.inner
                    .lock()
                    .unwrap()
                    .owned
                    .as_ref()
                    .unwrap()
                    .child
                    .id()
            );
            assert_eq!(rework_manifest(&root), before_manifest);
            pending_emit(
                "p1-active-restore-preflight",
                json!({"mode_ready_failure":ready_failure,"pid":pid,"instance":after.runtime.instance_id.map(|i|i.0),"mixed":endpoints.mixed.to_string(),"controller":endpoints.controller.to_string(),"disk":disk,"manifest":before_manifest,"live_cache":live,"operations_delta":0}),
            );
            let offset = port.events().len();
            if ready_failure {
                port.rework.lock().unwrap().fail_ready = true;
            } else {
                port.rework.lock().unwrap().fail_run = true;
            }
            assert_eq!(
                owner.execute(RuntimeCommand::ApplySaved, |s| assert!(
                    s.runtime.applied_version.is_none()
                )),
                Err(RuntimeError::SelectionPending)
            );
            assert_eq!(store.load().unwrap(), pending);
            assert_eq!(rework_manifest(&root), before_manifest);
            assert_eq!(
                records.artifact(v11.cache.as_ref().unwrap()).unwrap(),
                v11_cache
            );
            let stopped = owner.snapshot().unwrap();
            assert!(stopped.runtime.applied_version.is_none());
            assert!(stopped.endpoints.is_none());
            assert!(stopped.confirmed_selection_version.is_none());
            assert_eq!(stopped.pending_selections.len(), 1);
            let events = port.events();
            let tail = &events[offset..];
            assert_eq!(
                tail.iter().filter(|v| v["op"] == "run").count(),
                usize::from(ready_failure)
            );
            assert!(!tail.iter().any(|v| v["op"] == "PUT" || v["op"] == "GET"));
            let closed = pending_closed(&root, &initial);
            assert_ne!(
                closed["sha256"],
                v11.cache.as_ref().unwrap().digest,
                "不得恢复旧cache a"
            );
            // 已关闭 cache 与旧 manifest 来源也不一致，拒绝 Restore 且不改任何字节。
            let cache_bytes = fs::read(&cache).unwrap();
            let before = port.events();
            assert_eq!(
                owner.execute(RuntimeCommand::RestoreLastSuccessful, |_| panic!(
                    "closed cache mismatch must preflight"
                )),
                Err(RuntimeError::SelectionPending)
            );
            assert_eq!(port.events(), before);
            assert_eq!(fs::read(&cache).unwrap(), cache_bytes);
            assert_eq!(store.load().unwrap(), pending);
            pending_emit(
                "p1-failed-candidate",
                json!({"mode_ready_failure":ready_failure,"disk":disk,"manifest":before_manifest,"closed_cache":closed,"events":tail,"applied":stopped.runtime.applied_version,"closed_restore_operations_delta":0}),
            );
            // 正式 ApplySaved 使用保留的 cache，真实 GET=b 后才 confirm；证明 cache 的选择语义未被 a 覆盖。
            let offset = port.events().len();
            owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
            assert!(!port.events()[offset..].iter().any(|v| v["op"] == "PUT"));
            assert!(
                port.events()[offset..]
                    .iter()
                    .any(|v| v["op"] == "GET" && v["actual"] == pool.members[&NodeId("b".into())])
            );
            let confirmed = store.load().unwrap();
            assert_eq!(
                (confirmed.config_revision, confirmed.selection_revision),
                (12, 2)
            );
            assert_eq!(confirmed.state_epoch, initial.state_epoch);
            assert_eq!(
                rework_manifest(&root)["confirmed_selection"]["version"],
                serde_json::to_value(confirmed.selection_version()).unwrap()
            );
            pending_emit(
                "p1-recovered-from-live-cache",
                json!({"disk":pending_disk(&store),"manifest":rework_manifest(&root),"events":&port.events()[offset..]}),
            );
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            rework_cleanup(&root, &initial, &port);
        }
    }
    // P2：实际 GET=b 返回到 Runtime 前发布同池 pending。调度为测试注入，GET/PUT 为真实 HTTP。
    #[test]
    #[ignore = "fixed kernel Native with explicit same-pool GET interleaving"]
    fn p204_rework_native_same_pool_during_get() {
        for command in [
            RuntimeCommand::ReconcileSelection,
            RuntimeCommand::ApplySaved,
            RuntimeCommand::RestoreLastSuccessful,
        ] {
            let root = rework_root("p2-get");
            let store = JsonStateStore::new(root.join("state.json")).unwrap();
            let initial = pending_fixture();
            store.save(&initial).unwrap();
            let snapshots = Arc::new(SnapshotService::new(
                store.clone(),
                StateAccessGate::default(),
            ));
            let mut port = RecordingSidecarPort::new(&root);
            let mut owner = ManualRuntime::new(port.clone(), snapshots.clone(), root.clone(), true);
            owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
            let plan = rework_plan(&initial, &root);
            let pool = &plan.artifact_index().unwrap().pools[&PoolId("manual".into())];
            let identity = port.owned_identity();
            port.write_selector(
                &identity,
                &pool.runtime_tag,
                &pool.members[&NodeId("b".into())],
            )
            .unwrap();
            let before = owner.snapshot().unwrap();
            let manifest = rework_manifest(&root);
            let offset = port.events().len();
            port.rework.lock().unwrap().during_get =
                Some(((*snapshots).clone(), pool.runtime_tag.clone(), false));
            assert_eq!(
                owner.execute(command, |_| {}),
                Err(RuntimeError::SelectionPending)
            );
            let pending = store.load().unwrap();
            assert_eq!(
                (pending.config_revision, pending.selection_revision),
                (11, 1)
            );
            assert_eq!(pending.state_epoch, initial.state_epoch);
            let disk = pending_disk(&store);
            assert_eq!(disk["pools"][0]["selection"]["selected_node_id"], "a");
            assert_eq!(disk["pools"][0]["selection"]["pending_node_id"], "b");
            let events = port.events();
            assert!(!events[offset..].iter().any(|v| v["op"] == "PUT"));
            assert!(
                events[offset..]
                    .iter()
                    .any(|v| v["op"] == "GET" && v["actual"] == pool.members[&NodeId("b".into())])
            );
            let after = owner.snapshot().unwrap();
            assert!(after.confirmed_selection_version.is_none());
            assert_eq!(
                rework_manifest(&root)["confirmed_selection"],
                manifest["confirmed_selection"]
            );
            let service = SelectionService::new((*snapshots).clone());
            // 同epoch旧revision和同revision错误epoch均不得提交 pending；精确 CAS 才能前进。
            for expected in [
                initial.selection_version(),
                veyra_core::domain::SelectionVersion(veyra_core::domain::StateVersion {
                    epoch: AppState::empty().state_epoch,
                    revision: 1,
                }),
            ] {
                assert_eq!(
                    service
                        .confirm_manual_pending(
                            expected,
                            PoolId("manual".into()),
                            NodeId("b".into())
                        )
                        .unwrap_err()
                        .code(),
                    veyra_core::domain::AppErrorCode::RevisionConflict
                );
                assert_eq!(store.load().unwrap(), pending);
            }
            if command == RuntimeCommand::ReconcileSelection {
                assert_eq!(after.runtime.instance_id, before.runtime.instance_id);
                assert_eq!(after.endpoints, before.endpoints);
                assert_eq!(
                    after.runtime.applied_version,
                    before.runtime.applied_version
                );
                let endpoints = after.endpoints.unwrap();
                assert!(std::net::TcpStream::connect(endpoints.mixed).is_ok());
                assert!(std::net::TcpStream::connect(endpoints.controller).is_ok());
                assert_eq!(
                    port.read_selector(&identity, &pool.runtime_tag).unwrap(),
                    pool.members[&NodeId("b".into())]
                );
                owner
                    .execute(RuntimeCommand::ReconcileSelection, |_| {})
                    .unwrap();
            } else {
                assert!(after.runtime.applied_version.is_none());
                assert!(after.endpoints.is_none());
                assert_eq!(
                    events[offset..].iter().filter(|v| v["op"] == "run").count(),
                    1
                );
                pending_closed(&root, &initial);
                owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
            }
            let confirmed = store.load().unwrap();
            assert_eq!(
                (confirmed.config_revision, confirmed.selection_revision),
                (11, 2)
            );
            assert_eq!(confirmed.state_epoch, initial.state_epoch);
            assert!(!port.events()[offset..].iter().any(|v| v["op"] == "PUT"));
            assert_eq!(
                service
                    .confirm_manual_pending(
                        pending.selection_version(),
                        PoolId("manual".into()),
                        NodeId("b".into())
                    )
                    .unwrap_err()
                    .code(),
                veyra_core::domain::AppErrorCode::RevisionConflict
            );
            pending_emit(
                "p2-get-result",
                json!({"command":format!("{command:?}"),"pending":disk,"confirmed":pending_disk(&store),"old_instance":before.runtime.instance_id.map(|i|i.0),"after_instance":after.runtime.instance_id.map(|i|i.0),"applied_after_interleave":after.runtime.applied_version,"manifest":rework_manifest(&root),"events":&port.events()[offset..],"stale_revision_epoch_duplicate_CAS":"rejected"}),
            );
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            rework_cleanup(&root, &initial, &port);
        }
    }
    #[test]
    #[ignore = "fixed kernel Native with deterministic epoch replacement and write-gate contention"]
    fn p204_rework_native_epoch_and_write_gate() {
        use veyra_core::application::manual_runtime::ManualSelectionRequest;
        for epoch_change in [true, false] {
            let root = rework_root("p2-epoch-gate");
            let store = JsonStateStore::new(root.join("state.json")).unwrap();
            let initial = pending_fixture();
            store.save(&initial).unwrap();
            let snapshots = Arc::new(SnapshotService::new(
                store.clone(),
                StateAccessGate::default(),
            ));
            let mut port = RecordingSidecarPort::new(&root);
            let mut owner = ManualRuntime::new(port.clone(), snapshots.clone(), root.clone(), true);
            owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
            let before = owner.snapshot().unwrap();
            let plan = rework_plan(&initial, &root);
            let pool = &plan.artifact_index().unwrap().pools[&PoolId("manual".into())];
            let offset;
            if epoch_change {
                port.write_selector(
                    &port.owned_identity(),
                    &pool.runtime_tag,
                    &pool.members[&NodeId("b".into())],
                )
                .unwrap();
                offset = port.events().len();
                port.rework.lock().unwrap().during_get =
                    Some(((*snapshots).clone(), pool.runtime_tag.clone(), true));
                assert_eq!(
                    owner.execute(RuntimeCommand::ReconcileSelection, |_| {}),
                    Err(RuntimeError::SelectionPending)
                );
                let state = store.load().unwrap();
                assert_ne!(state.state_epoch, initial.state_epoch);
                assert_eq!((state.config_revision, state.selection_revision), (0, 0));
                assert!(!port.events()[offset..].iter().any(|v| v["op"] == "PUT"));
                assert_eq!(
                    owner.snapshot().unwrap().runtime.instance_id,
                    before.runtime.instance_id
                );
                assert_eq!(owner.snapshot().unwrap().endpoints, before.endpoints);
            } else {
                offset = port.events().len();
                port.rework.lock().unwrap().during_put = Some((*snapshots).clone());
                let version = owner
                    .select_manual(ManualSelectionRequest {
                        config: before.runtime.applied_version.clone().unwrap(),
                        instance: before.runtime.instance_id.clone().unwrap(),
                        expected: initial.selection_version(),
                        pool: PoolId("manual".into()),
                        node: NodeId("b".into()),
                    })
                    .unwrap();
                assert_eq!(version.0.epoch, initial.state_epoch);
                assert_eq!(version.0.revision, 2);
                assert_eq!(
                    store.load().unwrap().config_version(),
                    initial.config_version()
                );
                assert!(port.rework.lock().unwrap().during_put.is_none());
                assert_eq!(
                    port.events()[offset..]
                        .iter()
                        .filter(|v| v["op"] == "PUT")
                        .count(),
                    1
                );
            }
            pending_emit(
                "p2-epoch-gate-result",
                json!({"epoch_change":epoch_change,"initial_epoch":initial.state_epoch,"disk":pending_disk(&store),"manifest":rework_manifest(&root),"events":&port.events()[offset..]}),
            );
            owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
            rework_cleanup(&root, &initial, &port);
        }
    }

    // 只使用新建隔离root/固定内核，不发代理请求，不接管主机网络，也不访问既有app/child。
    #[test]
    #[ignore = "requires explicitly verified v1.14.0 VEYRA_SING_BOX_PATH"]
    fn p204_real_selection_cache_restart_and_rollback() {
        use std::sync::Arc;
        use veyra_core::{
            application::{
                manual_runtime::{
                    CandidateFailure, ManualRuntime, ManualSelectionRequest, RuntimeCommand,
                    RuntimeError,
                },
                runtime_recovery::{RecoveryAvailability, RecoveryStore, digest},
                state_access::StateAccessGate,
                state_service::SnapshotService,
            },
            domain::{NodeId, PoolId},
            storage::{JsonStateStore, StateStore},
        };
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let parts: serde_json::Value = serde_json::from_str(include_str!(
            "../../../veyra-core/tests/fixtures/compiler/p2-02b.json"
        ))
        .unwrap();
        for (k, v) in parts.as_object().unwrap() {
            value[k] = v.clone();
        }
        let mut state: AppState = serde_json::from_value(value).unwrap();
        state.active_subscription_id =
            Some(veyra_core::domain::SubscriptionId("subscription".into()));
        state.pools.retain(|p| p.id.0 == "manual");
        state.routes.clear();
        state.config_revision = 11;
        let root = std::env::temp_dir().join(format!("veyra-p204-real-{:?}", state.state_epoch));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let port = ManualSidecarPort::new(root.clone());
        assert!(port.kernel_available());
        let faults = port.faults.clone();
        let mut owner = ManualRuntime::new(port, snapshots.clone(), root.clone(), true);
        assert!(owner.snapshot().unwrap().runtime.applied_version.is_none());
        owner
            .execute(RuntimeCommand::ApplySaved, |s| {
                assert!(s.endpoints.is_none())
            })
            .unwrap();
        let first = owner.snapshot().unwrap();
        let endpoints = first.endpoints.unwrap();
        let selection = owner
            .select_manual(ManualSelectionRequest {
                config: first.runtime.applied_version.clone().unwrap(),
                instance: first.runtime.instance_id.clone().unwrap(),
                expected: first.confirmed_selection_version.unwrap(),
                pool: PoolId("manual".into()),
                node: NodeId("b".into()),
            })
            .unwrap();
        let secret_digest = |root: &std::path::Path| {
            let folder = fs::read_dir(root.join("runtime/configs"))
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path();
            let v: serde_json::Value =
                serde_json::from_slice(&fs::read(folder.join("config.json")).unwrap()).unwrap();
            assert_eq!(v["inbounds"][0]["listen_port"], 0);
            digest(
                v["experimental"]["clash_api"]["secret"]
                    .as_str()
                    .unwrap()
                    .as_bytes(),
            )
        };
        let first_secret = secret_digest(&root);
        // check失败旧实例仍Ready，端点/身份不变。
        faults.lock().unwrap().0 = true;
        assert_eq!(
            owner.execute(RuntimeCommand::Restart, |_| {}),
            Err(RuntimeError::CandidateFailed)
        );
        assert_eq!(
            owner.snapshot().unwrap().runtime.instance_id,
            first.runtime.instance_id
        );
        faults.lock().unwrap().0 = false;
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
        assert!(std::net::TcpStream::connect(endpoints.mixed).is_err());
        let records = RecoveryStore::new(&root).unwrap();
        let old = records.read_manifest().unwrap();
        let old_cache = old.cache.clone().unwrap();
        let closed_digest = digest(&records.artifact(&old_cache).unwrap());
        assert_eq!(old.confirmed_selection.version, selection);
        assert!(old_cache.digest == closed_digest);
        let mut state = store.load().unwrap();
        state.profile.reject_quic = !state.profile.reject_quic;
        let saved = store.commit(&state).unwrap();
        assert_eq!(saved.config_revision, 12);
        drop(owner);
        // 独立 OS 进程读同一关闭后的root：不从父进程内存继承Ready/ports/secret。
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "platform::manual_sidecar::real_tests::p204_restart_process_load",
                "--ignored",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("VEYRA_P204_RESTART_ROOT", &root)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() >= deadline {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("isolated restart process exceeded 60s");
            }
            thread::sleep(Duration::from_millis(20));
        }
        let port = ManualSidecarPort::new(root.clone());
        let ready_failures = port.ready_failures.clone();
        let mut owner = ManualRuntime::new(port, snapshots, root.clone(), true);
        let stopped = owner.snapshot().unwrap();
        assert_eq!(stopped.runtime.applied_version, None);
        assert_eq!(stopped.runtime.saved_version.0.revision, 12);
        assert_eq!(
            stopped.runtime.last_successful_version.unwrap().0.revision,
            11
        );
        assert_eq!(stopped.recovery, RecoveryAvailability::Available);
        owner
            .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
            .unwrap();
        let restored = owner.snapshot().unwrap();
        assert_eq!(restored.runtime.applied_version.unwrap().0.revision, 11);
        assert_eq!(
            restored.confirmed_manual_selections[&PoolId("manual".into())],
            NodeId("b".into())
        );
        assert_ne!(secret_digest(&root), first_secret);
        assert_ne!(restored.runtime.instance_id, first.runtime.instance_id);
        owner.execute(RuntimeCommand::ApplySaved, |_| {}).unwrap();
        assert_eq!(
            owner
                .snapshot()
                .unwrap()
                .runtime
                .applied_version
                .unwrap()
                .0
                .revision,
            12
        );
        // candidate Ready故障只发生一次；rollback重新check/run/鉴权/选择对账。
        *ready_failures.lock().unwrap() = 1;
        assert_eq!(
            owner.execute(RuntimeCommand::Restart, |_| {}),
            Err(RuntimeError::CandidateRolledBack(
                CandidateFailure::RunOrReady
            ))
        );
        let ready = owner.snapshot().unwrap();
        assert_eq!(ready.runtime.applied_version.unwrap().0.revision, 12);
        assert_eq!(
            ready.confirmed_manual_selections[&PoolId("manual".into())],
            NodeId("b".into())
        );
        let endpoints = ready.endpoints.unwrap();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
        assert!(std::net::TcpStream::connect(endpoints.mixed).is_err());
        assert_eq!(
            fs::read_dir(root.join("runtime/configs")).unwrap().count(),
            0
        );
        eprintln!(
            "P2-04 native PASS: selector readback/save; cache stop/reap snapshot; saved12/last11; fresh secret/instance; one Ready rollback; ports closed"
        );
        drop(owner);
        fs::remove_dir_all(root).unwrap();
    } // 只由上面的隔离验收进程启动；root是本测试新建的，不用于产品启动路径。
    #[test]
    #[ignore = "subprocess stage of p204_real_selection_cache_restart_and_rollback"]
    fn p204_restart_process_load() {
        use veyra_core::{
            application::{
                manual_runtime::{ManualRuntime, RuntimeCommand},
                state_access::StateAccessGate,
                state_service::SnapshotService,
            },
            domain::{NodeId, PoolId},
            storage::JsonStateStore,
        };
        let root =
            PathBuf::from(std::env::var_os("VEYRA_P204_RESTART_ROOT").expect("isolated test root"));
        assert!(
            root.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("veyra-p204-real-")
        );
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        let snapshots =
            std::sync::Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let port = ManualSidecarPort::new(root.clone());
        assert!(port.kernel_available());
        let mut owner = ManualRuntime::new(port, snapshots, root, true);
        let snapshot = owner.snapshot().unwrap();
        assert_eq!(snapshot.runtime.applied_version, None);
        assert!(snapshot.endpoints.is_none());
        assert_eq!(snapshot.runtime.saved_version.0.revision, 12);
        assert_eq!(
            snapshot.runtime.last_successful_version.unwrap().0.revision,
            11
        );
        owner
            .execute(RuntimeCommand::RestoreLastSuccessful, |_| {})
            .unwrap();
        let ready = owner.snapshot().unwrap();
        assert_eq!(ready.runtime.applied_version.unwrap().0.revision, 11);
        assert_eq!(
            ready.confirmed_manual_selections[&PoolId("manual".into())],
            NodeId("b".into())
        );
        let endpoints = ready.endpoints.unwrap();
        owner.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        assert!(std::net::TcpStream::connect(endpoints.controller).is_err());
        eprintln!(
            "P2-04 independent-process restart PASS: Stopped saved12/last11 -> Restore11; confirmed b; stop/reap"
        );
    }
    // 保护受管配置目录注入symlink时不改变外部目录权限/内容，不启动check子进程。
    #[test]
    fn candidate_directory_symlink_is_rejected_before_write() {
        for parent in [true, false] {
            let epoch = AppState::empty().state_epoch;
            let root = std::env::temp_dir().join(format!("veyra-p204-path-{:?}", epoch));
            fs::create_dir(&root).unwrap();
            let external = root.join("external");
            fs::create_dir(&external).unwrap();
            fs::set_permissions(&external, fs::Permissions::from_mode(0o755)).unwrap();
            if parent {
                std::os::unix::fs::symlink(&external, root.join("runtime")).unwrap();
            } else {
                fs::create_dir(root.join("runtime")).unwrap();
                std::os::unix::fs::symlink(&external, root.join("runtime/configs")).unwrap();
            }
            let mut port = ManualSidecarPort::new(root.clone());
            assert_eq!(port.check(&config(&root, 0, 0)), Err(SidecarPortError));
            assert_eq!(
                fs::metadata(&external).unwrap().permissions().mode() & 0o777,
                0o755
            );
            assert_eq!(fs::read_dir(&external).unwrap().count(), 0);
            assert!(port.pending.is_none());
            assert!(port.check_child.is_none());
            drop(port);
            fs::remove_dir_all(root).unwrap();
        }
    }
}

#[cfg(test)]
mod kernel_name_tests {
    use super::*;
    #[test]
    fn development_rejects_old_name_and_alias_without_executing_it() {
        // 开发override不能通过新名字symlink执行旧basename；无真实内核或用户缓存访问。
        let root = std::env::temp_dir().join(format!(
            "veyra-kernel-name-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        fs::create_dir(&root).unwrap();
        let old = root.join("sing-box");
        fs::write(&old, b"not executable fixture").unwrap();
        assert!(VerifiedKernel::verify(old.clone()).is_err());
        let alias = root.join("veyra-sing-box");
        std::os::unix::fs::symlink(&old, &alias).unwrap();
        assert!(VerifiedKernel::verify(alias.clone()).is_err());
        fs::remove_file(alias).unwrap();
        fs::write(root.join("veyra-sing-box"), b"foreign bytes").unwrap();
        assert!(VerifiedKernel::verify(root.join("veyra-sing-box")).is_err());
        assert_eq!(fs::read(old).unwrap(), b"not executable fixture");
        fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
#[path = "manual_sidecar_p403_tests.rs"]
mod p403_tests;
