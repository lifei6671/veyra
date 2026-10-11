use super::*;
use crate::{
    domain::{ProtocolOptions, ProxyProtocol},
    singbox::{
        GeneratedConfig,
        clash_api::ManagedControllerEndpoint,
        runtime::{ManagedRuntimeEndpoints, ManagedSidecar, SidecarPortError},
    },
    storage::latency::LatencyOutcome,
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
};

// 此Port仅替代进程边界；使用真实文件/独立监听与HTTP，不冒充固定内核Native验收。
struct LoopbackPort {
    root: PathBuf,
    config: Option<GeneratedConfig>,
    endpoint: Option<Arc<ManagedControllerEndpoint>>,
    mixed: Option<TcpListener>,
    controller: Option<std::net::SocketAddr>,
    server: Option<thread::JoinHandle<()>>,
    release: Option<mpsc::Sender<()>>,
    started: Option<mpsc::Sender<()>>,
    timeout: bool,
    fail_cleanup: Arc<std::sync::atomic::AtomicBool>,
    addresses: Arc<Mutex<Vec<std::net::SocketAddr>>>,
    requests: Arc<Mutex<Vec<String>>>,
}
impl SidecarPort for LoopbackPort {
    fn check(&mut self, c: &GeneratedConfig) -> Result<(), SidecarPortError> {
        let v: serde_json::Value = serde_json::from_slice(c.as_bytes()).unwrap();
        assert_eq!(v["inbounds"].as_array().unwrap().len(), 1);
        assert_eq!(v["inbounds"][0]["listen"], "127.0.0.1");
        assert_eq!(v["inbounds"][0]["listen_port"], 0);
        assert_eq!(
            v["experimental"]["cache_file"]["path"],
            self.root.join("cache.db").to_str().unwrap()
        );
        assert!(
            v["inbounds"]
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x["type"] != "tun")
        );
        Ok(())
    }
    fn prepare(&mut self, c: &GeneratedConfig) -> Result<(), SidecarPortError> {
        std::fs::write(self.root.join("runtime/configs/config.json"), c.as_bytes()).unwrap();
        std::fs::write(self.root.join("cache.db"), b"owned temporary cache").unwrap();
        self.config = Some(c.clone());
        Ok(())
    }
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        self.controller = Some(address);
        self.endpoint = Some(Arc::new(
            ManagedControllerEndpoint::from_owned_child(address, self.config.as_ref().unwrap())
                .unwrap(),
        ));
        self.mixed = Some(TcpListener::bind("127.0.0.1:0").unwrap());
        self.addresses
            .lock()
            .unwrap()
            .extend([address, self.mixed.as_ref().unwrap().local_addr().unwrap()]);
        let (release, wait) = mpsc::channel();
        self.release = Some(release);
        let timeout = self.timeout;
        let started = self.started.take();
        let requests = self.requests.clone();
        self.server = Some(thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                if socket.read_exact(&mut byte).is_err() {
                    return;
                }
                bytes.push(byte[0]);
            }
            requests
                .lock()
                .unwrap()
                .push(String::from_utf8(bytes).unwrap());
            if let Some(started) = started {
                started.send(()).unwrap();
            }
            if timeout {
                let _ = wait.recv_timeout(Duration::from_secs(3));
            }
            let body = "{\"delay\":12}";
            let _ = socket.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .as_bytes(),
            );
        }));
        Ok(ManagedSidecar::from_port_identity(1))
    }
    fn ready(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
        Ok(())
    }
    fn endpoints(&self, _: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
        Some(ManagedRuntimeEndpoints {
            mixed: self.mixed.as_ref()?.local_addr().ok()?,
            controller: self.controller?,
        })
    }
    fn observation_endpoint(&self, _: &ManagedSidecar) -> Option<Arc<ManagedControllerEndpoint>> {
        self.endpoint.clone()
    }
    fn stop(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
        if self.fail_cleanup.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(SidecarPortError);
        }
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(address) = self.controller.take() {
            let _ = TcpStream::connect(address);
        }
        if let Some(server) = self.server.take() {
            server.join().unwrap();
        }
        self.mixed = None;
        self.endpoint = None;
        Ok(())
    }
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        self.config = None;
        Ok(())
    }
    fn has_pending_cleanup(&self) -> bool {
        false
    }
}
fn draft() -> ProxyNodeDraft {
    ProxyNodeDraft {
        name: "draft only".into(),
        protocol: ProxyProtocol::Socks,
        server: "127.0.0.1".into(),
        port: 20000,
        options: ProtocolOptions::Socks {
            version: 5,
            username: None,
            password: None,
        },
        transport: None,
        tls: None,
    }
}
fn port(
    root: &Path,
    timeout: bool,
    started: Option<mpsc::Sender<()>>,
    requests: Arc<Mutex<Vec<String>>>,
) -> LoopbackPort {
    LoopbackPort {
        root: root.into(),
        config: None,
        endpoint: None,
        mixed: None,
        controller: None,
        server: None,
        release: None,
        started,
        timeout,
        fail_cleanup: Default::default(),
        addresses: Default::default(),
        requests,
    }
}

// 保护草稿成功/超时/取消均只清理本实例；生产Compiler+NetworkService实际loopback且主文件字节不变。
#[test]
fn draft_success_timeout_cancel_cleanup_real_files_and_loopback() {
    let parent = std::env::temp_dir().join(format!(
        "veyra-p208-draft-{:?}",
        StateEpoch::fresh().unwrap()
    ));
    fs::create_dir(&parent).unwrap();
    let main = parent.join("main");
    fs::create_dir(&main).unwrap();
    for name in ["cache.db", "config.json", "manifest.json"] {
        fs::write(main.join(name), b"main unchanged").unwrap();
    }
    let external = TcpListener::bind("127.0.0.1:0").unwrap();
    for mode in 0..3 {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let (started, wait) = mpsc::channel();
        let addresses = Arc::new(Mutex::new(Vec::new()));
        let mut session = DraftLatencySession::new(&parent, |root| {
            let mut p = port(
                root,
                mode != 0,
                if mode == 2 { Some(started) } else { None },
                requests.clone(),
            );
            p.addresses = addresses.clone();
            p
        })
        .unwrap();
        let root = session.root().to_owned();
        let preferences = UiLatencyPreferences {
            test_url: crate::domain::UiLatencyUrl(
                "https://target.example.invalid/draft?q=1".into(),
            ),
            timeout_ms: if mode == 1 { 30 } else { 2000 },
            ..Default::default()
        };
        let cancellation = TestCancellation::new();
        let cancel = cancellation.clone();
        let canceller = (mode == 2).then(|| {
            thread::spawn(move || {
                wait.recv_timeout(Duration::from_secs(3)).unwrap();
                cancel.cancel();
            })
        });
        let result = session.test(draft(), &preferences, &cancellation);
        assert!(result.cleanup.is_ok());
        match mode {
            0 => assert_eq!(result.result.unwrap().milliseconds, Some(12)),
            1 => assert!(
                matches!(result.result,Err(DraftLatencyError::Network(ref e)) if LatencyOutcome::from_result(&Err(e.clone()))==LatencyOutcome::Timeout)
            ),
            _ => assert!(matches!(result.result, Err(DraftLatencyError::Cancelled))),
        }
        if let Some(canceller) = canceller {
            canceller.join().unwrap();
        }
        assert!(!root.exists());
        assert_eq!(addresses.lock().unwrap().len(), 2);
        for address in addresses.lock().unwrap().iter() {
            assert!(TcpListener::bind(address).is_ok());
        }
        assert!(
            requests.lock().unwrap()[0]
                .contains("url=https%3A%2F%2Ftarget.example.invalid%2Fdraft%3Fq%3D1")
        );
        for name in ["cache.db", "config.json", "manifest.json"] {
            assert_eq!(fs::read(main.join(name)).unwrap(), b"main unchanged");
        }
        assert!(external.local_addr().is_ok());
    }
    drop(external);
    fs::remove_dir_all(parent).unwrap();
}

// 保护测试成功但清理失败单独报告，保留owned Port/root，显式重试才能删除材料。
#[test]
fn cleanup_failure_preserves_owned_root_until_explicit_cleanup() {
    let parent = std::env::temp_dir().join(format!(
        "veyra-p208-draft-retain-{:?}",
        StateEpoch::fresh().unwrap()
    ));
    fs::create_dir(&parent).unwrap();
    let fail = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let mut session = DraftLatencySession::new(&parent, |root| {
        let mut p = port(root, false, None, Default::default());
        p.fail_cleanup = fail.clone();
        p
    })
    .unwrap();
    let result = session.test(
        draft(),
        &UiLatencyPreferences {
            test_url: crate::domain::UiLatencyUrl("https://target.example.invalid".into()),
            ..Default::default()
        },
        &TestCancellation::new(),
    );
    assert!(result.result.is_ok());
    assert!(matches!(result.cleanup, Err(DraftLatencyError::Sidecar(_))));
    assert!(session.root().join("cache.db").exists());
    fail.store(false, std::sync::atomic::Ordering::SeqCst);
    session.cleanup().unwrap();
    drop(session);
    fs::remove_dir_all(parent).unwrap();
}
