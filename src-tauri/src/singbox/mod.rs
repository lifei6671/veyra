//! 强类型受管配置编译、最终字节校验与受控 sidecar 边界。

pub(crate) mod clash_api;
mod compiler;
pub(crate) mod managed_sidecar;
pub(crate) mod runtime;

#[cfg(test)]
pub(crate) mod test_support {
    use std::sync::Mutex;

    pub(crate) static FIXED_CLASH_API_TEST_LOCK: Mutex<()> = Mutex::new(());
}

pub use compiler::{ConfigCompiler, GeneratedConfig, RuntimeProfile, SingBoxCompiler, SingBoxPlan};

#[cfg(all(test, target_os = "windows"))]
mod task009_controlled_network {
    use std::{
        fs,
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        path::PathBuf,
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use sha2::{Digest, Sha256};

    use super::{
        ConfigCompiler, GeneratedConfig, RuntimeProfile, SingBoxCompiler,
        managed_sidecar::generate_api_secret,
        runtime::{SidecarLifecycle, SidecarRuntime},
        test_support::FIXED_CLASH_API_TEST_LOCK,
    };
    use crate::{
        domain::*,
        platform::windows::managed_sidecar_port::WindowsManagedSidecarPort,
        subscription::{normalize_nodes, parse_subscription},
    };

    const PROBE_HOST: &str = "task009-probe.invalid";
    const MAX_MESSAGE: usize = 16 * 1024;
    type ProbeResult<T> = Result<T, &'static str>;

    fn read_exact_until(
        stream: &mut TcpStream,
        buffer: &mut [u8],
        deadline: Instant,
    ) -> ProbeResult<()> {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or("fixture read deadline")?;
        stream
            .set_read_timeout(Some(remaining.min(Duration::from_secs(2))))
            .map_err(|_| "fixture read timeout")?;
        stream.read_exact(buffer).map_err(|error| {
            eprintln!("task009-network read_error_kind={:?}", error.kind());
            "fixture read"
        })
    }

    fn read_headers(stream: &mut TcpStream, deadline: Instant) -> ProbeResult<String> {
        let mut bytes = Vec::new();
        while bytes.len() < MAX_MESSAGE {
            let mut byte = [0];
            read_exact_until(stream, &mut byte, deadline)?;
            bytes.push(byte[0]);
            if bytes.ends_with(b"\r\n\r\n") {
                return String::from_utf8(bytes).map_err(|_| "fixture HTTP encoding");
            }
        }
        Err("fixture HTTP header limit")
    }

    fn take<'a>(input: &mut &'a [u8], length: usize) -> ProbeResult<&'a [u8]> {
        let value = input.get(..length).ok_or("truncated ClientHello")?;
        *input = &input[length..];
        Ok(value)
    }

    fn read_u16(input: &mut &[u8]) -> ProbeResult<usize> {
        let value = take(input, 2)?;
        Ok(usize::from(u16::from_be_bytes([value[0], value[1]])))
    }

    // 只解析固定核心发来的有界 ClientHello，不终止 TLS、不安装证书、不解密应用流量。
    fn client_hello_sni(mut input: &[u8]) -> ProbeResult<&[u8]> {
        let handshake = take(&mut input, 4)?;
        if handshake[0] != 1 {
            return Err("expected ClientHello");
        }
        let length = (usize::from(handshake[1]) << 16)
            | (usize::from(handshake[2]) << 8)
            | usize::from(handshake[3]);
        input = take(&mut input, length)?;
        take(&mut input, 34)?;
        let session_length = usize::from(take(&mut input, 1)?[0]);
        take(&mut input, session_length)?;
        let cipher_length = read_u16(&mut input)?;
        take(&mut input, cipher_length)?;
        let compression_length = usize::from(take(&mut input, 1)?[0]);
        take(&mut input, compression_length)?;
        let extensions_length = read_u16(&mut input)?;
        let mut extensions = take(&mut input, extensions_length)?;
        while !extensions.is_empty() {
            let kind = read_u16(&mut extensions)?;
            let length = read_u16(&mut extensions)?;
            let mut extension = take(&mut extensions, length)?;
            if kind == 0 {
                let names_length = read_u16(&mut extension)?;
                let mut names = take(&mut extension, names_length)?;
                while !names.is_empty() {
                    let name_type = take(&mut names, 1)?[0];
                    let name_length = read_u16(&mut names)?;
                    let name = take(&mut names, name_length)?;
                    if name_type == 0 {
                        return Ok(name);
                    }
                }
            }
        }
        Err("missing ClientHello SNI")
    }

    fn serve_probe(listener: TcpListener, socks: bool, tls: bool) -> ProbeResult<()> {
        let deadline = Instant::now() + Duration::from_secs(20);
        listener
            .set_nonblocking(true)
            .map_err(|_| "fixture nonblocking listener")?;
        let (mut stream, peer) = loop {
            match listener.accept() {
                Ok(accepted) => break accepted,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if Instant::now() >= deadline {
                        return Err("fixture accept deadline");
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return Err("fixture accept"),
            }
        };
        if !peer.ip().is_loopback() {
            return Err("fixture peer is not loopback");
        }
        // Windows 接受的 socket 可继承监听器的非阻塞属性；后续协议读取使用有界阻塞 I/O。
        stream
            .set_nonblocking(false)
            .map_err(|_| "fixture accepted socket mode")?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|_| "fixture write timeout")?;
        let target_port = if tls { 443_u16 } else { 80_u16 };
        if socks {
            let mut hello = [0; 2];
            read_exact_until(&mut stream, &mut hello, deadline)
                .map_err(|_| "SOCKS greeting read")?;
            if hello[0] != 5 || hello[1] == 0 {
                return Err("fixture SOCKS version");
            }
            let mut methods = vec![0; usize::from(hello[1])];
            read_exact_until(&mut stream, &mut methods, deadline)
                .map_err(|_| "SOCKS methods read")?;
            if !methods.contains(&0) {
                return Err("fixture SOCKS authentication");
            }
            stream
                .write_all(&[5, 0])
                .map_err(|_| "fixture SOCKS reply")?;
            let mut request = [0; 5];
            read_exact_until(&mut stream, &mut request, deadline)
                .map_err(|_| "SOCKS CONNECT read")?;
            if request[..4] != [5, 1, 0, 3] {
                return Err("SOCKS must preserve domain address type");
            }
            let mut host = vec![0; usize::from(request[4])];
            read_exact_until(&mut stream, &mut host, deadline)
                .map_err(|_| "SOCKS hostname read")?;
            let mut port = [0; 2];
            read_exact_until(&mut stream, &mut port, deadline).map_err(|_| "SOCKS port read")?;
            if host != PROBE_HOST.as_bytes() || u16::from_be_bytes(port) != target_port {
                return Err("SOCKS destination changed");
            }
            stream
                .write_all(&[5, 0, 0, 1, 127, 0, 0, 1, 0, 0])
                .map_err(|_| "fixture SOCKS connect reply")?;
        } else {
            let headers =
                read_headers(&mut stream, deadline).map_err(|_| "HTTP CONNECT headers read")?;
            if headers.lines().next()
                != Some(format!("CONNECT {PROBE_HOST}:{target_port} HTTP/1.1").as_str())
            {
                return Err("HTTP CONNECT destination changed");
            }
            stream
                .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                .map_err(|_| "fixture CONNECT reply")?;
        }
        if tls {
            let mut record_header = [0; 5];
            read_exact_until(&mut stream, &mut record_header, deadline)
                .map_err(|_| "TLS record header read")?;
            let length = usize::from(u16::from_be_bytes([record_header[3], record_header[4]]));
            if record_header[0] != 22 || length > MAX_MESSAGE {
                return Err("fixture TLS record limit or type");
            }
            let mut record = vec![0; length];
            read_exact_until(&mut stream, &mut record, deadline)
                .map_err(|_| "TLS ClientHello read")?;
            if client_hello_sni(&record)? != PROBE_HOST.as_bytes() {
                return Err("TLS SNI changed");
            }
            // 此例只证明原 URL 的 SNI；对端在握手阶段关闭，不伪造可信 TLS 成功。
        } else {
            let headers =
                read_headers(&mut stream, deadline).map_err(|_| "HTTP HEAD headers read")?;
            if headers.lines().next() != Some("HEAD /task009-check?token=controlled HTTP/1.1")
                || !headers.lines().any(|line| {
                    line.split_once(':').is_some_and(|(key, value)| {
                        key.eq_ignore_ascii_case("host") && value.trim() == PROBE_HOST
                    })
                })
            {
                return Err("HTTP path or Host changed");
            }
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .map_err(|_| "fixture HEAD response")?;
        }
        Ok(())
    }

    fn compile_probe(port: u16, socks: bool, tls: bool) -> GeneratedConfig {
        let input = serde_json::json!({"outbounds": [{
            "type": if socks { "socks" } else { "http" },
            "tag": "controlled-proxy", "server": "127.0.0.1", "server_port": port
        }]});
        let parsed = parse_subscription(&input.to_string()).expect("parse controlled subscription");
        assert!(parsed.skipped.is_empty());
        assert_eq!(parsed.nodes.len(), 1);
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            id: SubscriptionId("controlled-subscription".into()),
            name: "controlled".into(),
        });
        state.providers.push(Provider {
            id: ProviderId("controlled-provider".into()),
            subscription_id: SubscriptionId("controlled-subscription".into()),
            name: "controlled".into(),
        });
        state.nodes = normalize_nodes(ProviderId("controlled-provider".into()), parsed.nodes)
            .expect("normalize controlled subscription");
        state.pools.push(NodePool {
            id: PoolId("controlled-pool".into()),
            name: "controlled".into(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("controlled-provider".into()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::UrlTest {
                probe_url: format!(
                    "{}://{PROBE_HOST}/task009-check?token=controlled",
                    if tls { "https" } else { "http" }
                ),
                interval_secs: 300,
                tolerance_ms: 50,
            },
            enabled: true,
        });
        state.default_target = RouteTarget::Pool(PoolId("controlled-pool".into()));
        let intent = RuntimeIntent::from_state(&state).expect("validate controlled AppState");
        SingBoxCompiler
            .compile(
                &intent,
                &state.default_target,
                DnsPolicy::System,
                RuntimeProfile::ObservationOnly,
            )
            .expect("compile controlled plan")
            .finalize(&generate_api_secret().expect("per-instance secret"))
            .expect("finalize exact controlled bytes")
    }

    fn run_probe(socks: bool, tls: bool) {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let cache = manifest.join("binaries/sing-box-1.14.0-windows-amd64");
        assert!(
            cache.is_dir(),
            "fixed bundle unavailable; cannot claim network PASS"
        );
        let name = format!(
            "task009-network-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let resources = manifest.join("target").join(&name);
        let assets = resources.join("sing-box/1.14.0");
        fs::create_dir_all(&assets).expect("create controlled resource root");
        for file in ["sing-box.exe", "libcronet.dll", "LICENSE"] {
            fs::hard_link(cache.join(file), assets.join(file)).expect("link fixed asset");
        }
        let app_data = std::env::temp_dir().join(&name);
        fs::create_dir(&app_data).expect("create controlled app-data root");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind controlled peer");
        let candidate = compile_probe(
            listener.local_addr().expect("peer address").port(),
            socks,
            tls,
        );
        let digest = Sha256::digest(candidate.as_bytes());
        let port = WindowsManagedSidecarPort::new(resources.clone(), app_data.clone())
            .expect("verify fixed resources and private-runtime boundary");
        let mut runtime = SidecarRuntime::new_observation_only(port);
        let peer = thread::spawn(move || serve_probe(listener, socks, tls));
        let started = runtime.start_or_replace(candidate);
        let identity = runtime.with_active_port(|_, child| Ok(child.identity()));
        let observed = peer.join();
        // 在断言任何运行结果之前停止实例，失败断言也不会跳过正常归属清理。
        let stopped = runtime.stop();
        let snapshot = runtime.snapshot();
        drop(runtime);
        let runtime_directory = app_data.join("sidecar-runtime");
        let cleaned = fs::read_dir(&runtime_directory).map(|mut entries| entries.next().is_none());
        // 仅删除本测试创建的唯一子目录；不枚举或清理其它运行目录。
        assert!(resources.starts_with(manifest.join("target")));
        assert!(app_data.starts_with(std::env::temp_dir()));
        if stopped.is_ok() {
            fs::remove_dir_all(&app_data).expect("remove controlled app-data root");
            fs::remove_dir_all(&resources).expect("remove controlled resource links");
        }
        // 归属清理成功后，协议断言失败不应污染其余串行用例；清理失败仍保持封闭。
        if stopped.is_ok() && cleaned.as_ref().is_ok_and(|empty| *empty) {
            drop(_lock);
        }
        assert!(started.is_ok(), "controlled core did not become Ready");
        assert_eq!(observed.expect("controlled peer thread"), Ok(()));
        assert!(stopped.is_ok(), "controlled child cleanup failed");
        assert_eq!(snapshot.lifecycle, SidecarLifecycle::Stopped);
        assert!(cleaned.expect("read private runtime directory"));
        println!(
            "task009-network socks={socks} tls_sni_only={tls} sha256={digest:x} identity={}",
            identity
                .expect("read owned identity")
                .expect("active child")
        );
    }

    #[test]
    fn task009_http_urltest_preserves_remote_domain_and_host() {
        run_probe(false, false);
    }

    #[test]
    fn task009_socks_urltest_preserves_remote_domain_and_host() {
        run_probe(true, false);
    }

    #[test]
    fn task009_http_urltest_preserves_tls_sni() {
        run_probe(false, true);
    }

    #[test]
    fn task009_socks_urltest_preserves_tls_sni() {
        run_probe(true, true);
    }
}
