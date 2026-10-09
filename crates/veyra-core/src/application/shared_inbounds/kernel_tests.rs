//! 显式 opt-in 的固定内核验收；只管理本测试创建的 child、loopback 和临时资源。
use super::*;
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Child, ExitStatus, Stdio},
    time::{Duration, Instant},
};

const BINARY_SHA: &str = "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4";
struct OwnedChild {
    child: Child,
    log: PathBuf,
}
impl OwnedChild {
    fn spawn(binary: &Path, action: &str, config: &Path, log: PathBuf) -> Self {
        let output = fs::File::create(&log).unwrap();
        let child = Command::new(binary)
            .arg(action)
            .arg("-c")
            .arg(config)
            .stdin(Stdio::null())
            .stdout(output.try_clone().unwrap())
            .stderr(output)
            .spawn()
            .unwrap();
        Self { child, log }
    }
    fn exited(&mut self, timeout: Duration) -> ExitStatus {
        let end = Instant::now() + timeout;
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < end, "owned child did not exit in time");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn ready(&mut self, transports: usize) {
        let end = Instant::now() + Duration::from_secs(5);
        loop {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "kernel exited before inbound ready: {}",
                fs::read_to_string(&self.log).unwrap()
            );
            let log = fs::read_to_string(&self.log).unwrap();
            if log.matches("server started at 127.0.0.1:").count() >= transports {
                return;
            }
            assert!(Instant::now() < end, "kernel readiness timeout: {log}");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    fn stop(&mut self) {
        // PID 来自本测试的 Child，不读取、查找或停止任何外部实例。
        assert_eq!(
            unsafe { libc::kill(self.child.id() as i32, libc::SIGTERM) },
            0
        );
        assert!(self.exited(Duration::from_secs(3)).success());
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
fn write_config(root: &Path, name: &str, value: &serde_json::Value) -> PathBuf {
    let path = root.join(format!("{name}.json"));
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    file.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    path
}
fn fixture_config(server: &SharedServer) -> serde_json::Value {
    // 仅测试夹具组合 direct 出口；正式产品的 route/DNS/cache/secret 仍由原 Runtime 管理。
    serde_json::json!({"log": {"level": "info", "timestamp": false},
        "inbounds": SingBoxCompiler.compile_shared_inbounds(std::slice::from_ref(server)).unwrap(),
        "outbounds": [{"type": "direct", "tag": "direct"}], "route": {"final": "direct"}})
}
fn free_port(server: &mut SharedServer) {
    for _ in 0..20 {
        server.port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        if preflight_shared_port(server, &[], &[], None).is_empty() {
            return;
        }
    }
    panic!("no free owned test port");
}

#[test]
#[ignore = "requires VEYRA_P504_SING_BOX: locked v1.14.0 darwin-arm64; starts only isolated loopback children"]
fn shared_inbounds_locked_kernel_check_load_conflict_cleanup() {
    let f = Fixture::new();
    let source = PathBuf::from(
        std::env::var_os("VEYRA_P504_SING_BOX").expect("explicit locked binary required"),
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&source).unwrap())),
        BINARY_SHA,
        "binary identity mismatch: do not execute"
    );
    let binary = f.root.join("veyra-sing-box");
    fs::copy(source, &binary).unwrap();
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&binary).unwrap())),
        BINARY_SHA
    );
    let version = Command::new(&binary).arg("version").output().unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    assert!(version.contains("sing-box version 1.14.0") && version.contains("darwin/arm64"));
    let mut records = Vec::new();
    for kind in 0..5 {
        let mut server = sample(kind);
        free_port(&mut server);
        let service = f.service();
        let current = service.snapshot().unwrap();
        let state = service
            .edit_shared_server(
                current.config_version(),
                SharedServerCommand::Create(server.clone()),
            )
            .unwrap()
            .value;
        drop(service);
        // 从新建生产 Store/Service 磁盘回读后编译；固定端口与证书不是内核临时生成值。
        let restored = f.service().snapshot().unwrap();
        assert_eq!(restored, state);
        server = restored
            .app_config
            .shared_servers
            .iter()
            .find(|s| s.id == server.id)
            .unwrap()
            .clone();
        let valid = fixture_config(&server);
        let config = write_config(&f.root, &format!("valid-{kind}"), &valid);
        let mut check = OwnedChild::spawn(
            &binary,
            "check",
            &config,
            f.root.join(format!("check-{kind}.log")),
        );
        assert!(
            check.exited(Duration::from_secs(10)).success(),
            "valid check failed: {}",
            fs::read_to_string(&check.log).unwrap()
        );
        let mut invalid = valid.clone();
        match kind {
            0 => invalid["inbounds"][0]["method"] = "not-a-method".into(),
            // 固定内核允许任意字符串派生 VLESS UUID；用确实非法的 TLS key 做反例。
            1 => invalid["inbounds"][0]["tls"]["key"] = serde_json::json!(["not-a-key"]),
            2 => invalid["inbounds"][0]["tls"]["key"] = serde_json::json!(["not-a-key"]),
            3 => invalid["inbounds"][0]["obfs"]["type"] = "not-an-obfs".into(),
            _ => invalid["inbounds"][0]["users"][0]["username"] = 42.into(),
        }
        let bad_config = write_config(&f.root, &format!("invalid-{kind}"), &invalid);
        let mut bad = OwnedChild::spawn(
            &binary,
            "check",
            &bad_config,
            f.root.join(format!("invalid-{kind}.log")),
        );
        assert!(!bad.exited(Duration::from_secs(10)).success());
        let bad_log = fs::read_to_string(&bad.log).unwrap();
        let mut stable = OwnedChild::spawn(
            &binary,
            "run",
            &config,
            f.root.join(format!("run-{kind}.log")),
        );
        stable.ready(server.transports().len());
        let stable_pid = stable.child.id();
        let mut conflict_records = Vec::new();
        for &transport in server.transports() {
            let mut candidate = server.clone();
            free_port(&mut candidate);
            assert!(preflight_shared_port(&candidate, &[], &[], None).is_empty());
            let path = write_config(
                &f.root,
                &format!("candidate-{kind}-{transport:?}"),
                &fixture_config(&candidate),
            );
            // 预检结束后另一个自有 socket 抢占端口：真实加载必须失败，预检不能冒充成功。
            let (tcp, udp) = match transport {
                SharedTransport::Tcp => (
                    Some(TcpListener::bind(candidate.socket_addr()).unwrap()),
                    None,
                ),
                SharedTransport::Udp => (
                    None,
                    Some(UdpSocket::bind(candidate.socket_addr()).unwrap()),
                ),
            };
            let mut failed = OwnedChild::spawn(
                &binary,
                "run",
                &path,
                f.root.join(format!("conflict-{kind}-{transport:?}.log")),
            );
            assert!(!failed.exited(Duration::from_secs(5)).success());
            let log = fs::read_to_string(&failed.log).unwrap();
            assert!(
                log.contains("address already in use"),
                "missing actual bind failure: {log}"
            );
            assert!(stable.child.try_wait().unwrap().is_none());
            assert_eq!(stable.child.id(), stable_pid);
            // 原计划未被本测试停止/替换；正式 Runtime 失败回退是 P2-06 的独立契约。
            conflict_records.push(serde_json::json!({"transport": format!("{transport:?}"), "exit": "failure", "stderr": log, "existing_child_alive": true}));
            drop(tcp);
            drop(udp);
            assert!(preflight_shared_port(&candidate, &[], &[], None).is_empty());
        }
        let ready_log = fs::read_to_string(&stable.log).unwrap();
        stable.stop();
        assert!(preflight_shared_port(&server, &[], &[], None).is_empty());
        records.push(serde_json::json!({"protocol": valid["inbounds"][0]["type"], "fixed_address": server.socket_addr().to_string(),
            "config_sha256": format!("{:x}", Sha256::digest(fs::read(config).unwrap())), "check": "PASS", "invalid_check": "rejected", "invalid_stderr": bad_log,
            "load": "PASS", "ready_log": ready_log, "bind_conflicts": conflict_records, "sigterm_reap_ports_released": true}));
        println!(
            "P5-04 {} check PASS; invalid rejected; load PASS; {} bind conflict(s) rejected; old child retained; exit/reap/ports PASS",
            valid["inbounds"][0]["type"],
            server.transports().len()
        );
    }
    // 其余 React 选项也用真实 check 保护：四种 SS method、VLESS 无TLS、mixed 无认证。
    for (i, method) in [
        SharedShadowsocksMethod::Aes128Gcm,
        SharedShadowsocksMethod::Chacha20IetfPoly1305,
        SharedShadowsocksMethod::Blake3Aes256Gcm2022,
    ]
    .into_iter()
    .enumerate()
    {
        let mut ss = sample(0);
        ss.protocol = SharedProtocol::Shadowsocks {
            method,
            password: if method == SharedShadowsocksMethod::Blake3Aes256Gcm2022 {
                "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into()
            } else {
                "test-only-password".into()
            },
        };
        let path = write_config(&f.root, &format!("ss-option-{i}"), &fixture_config(&ss));
        let mut check = OwnedChild::spawn(
            &binary,
            "check",
            &path,
            f.root.join(format!("ss-option-{i}.log")),
        );
        assert!(check.exited(Duration::from_secs(10)).success());
    }
    for i in [1, 4] {
        let mut s = sample(i);
        if i == 1 {
            s.protocol = SharedProtocol::Vless {
                uuid: "6c6fe1d0-42a8-4c24-a801-c6e1a867184d".into(),
                tls: None,
            };
        } else {
            s.protocol = SharedProtocol::Mixed {
                username: "".into(),
                password: "".into(),
            };
        }
        let path = write_config(&f.root, &format!("plain-{i}"), &fixture_config(&s));
        let mut check = OwnedChild::spawn(
            &binary,
            "check",
            &path,
            f.root.join(format!("plain-{i}.log")),
        );
        assert!(check.exited(Duration::from_secs(10)).success());
    }
    if let Some(path) = std::env::var_os("VEYRA_P504_KERNEL_RECEIPT") {
        fs::write(path, serde_json::to_vec_pretty(&serde_json::json!({"binary_sha256": BINARY_SHA, "version": version, "valid_checks": 10, "invalid_checks": 5, "loads": 5, "bind_conflicts": 6, "records": records})).unwrap()).unwrap();
    }
}
