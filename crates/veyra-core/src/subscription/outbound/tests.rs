//! 只保护Veyra自有合同：路径固定、DNS、redirect、凭据与失败不改道。
//! 全部使用合成证书及本测试的loopback listener，不启动内核/TUN/公网请求。
use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::Instant,
};

type Requests = Arc<Mutex<Vec<String>>>;
struct Server {
    address: SocketAddr,
    requests: Requests,
    stop: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.worker.take().unwrap().join().expect("fixture worker");
    }
}
fn read_headers(stream: &mut impl Read) -> std::io::Result<String> {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b"\r\n\r\n") && bytes.len() < 16384 {
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        bytes.push(byte[0]);
    }
    Ok(String::from_utf8(bytes).expect("synthetic headers"))
}
fn response(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
fn redirect(url: &str) -> String {
    format!(
        "HTTP/1.1 302 Found\r\nLocation: {url}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    )
}
fn serve(tls: bool, reply: impl Fn(&str) -> String + Send + 'static) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let requests: Requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let config = tls.then(|| {
        let provider = rustls::crypto::ring::default_provider();
        Arc::new(
            rustls::ServerConfig::builder_with_provider(Arc::new(provider))
                .with_safe_default_protocol_versions()
                .unwrap()
                .with_no_client_auth()
                .with_single_cert(
                    vec![include_bytes!("fixtures/server.der").to_vec().into()],
                    rustls::pki_types::PrivateKeyDer::Pkcs8(
                        include_bytes!("fixtures/server-key.der").to_vec().into(),
                    ),
                )
                .unwrap(),
        )
    });
    let worker = thread::spawn(move || {
        while !stopping.load(Ordering::Relaxed) {
            let (socket, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("fixture accept: {error}"),
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            socket
                .set_write_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut stream: Box<dyn ReadWrite> = if let Some(config) = &config {
                Box::new(rustls::StreamOwned::new(
                    rustls::ServerConnection::new(config.clone()).unwrap(),
                    socket,
                ))
            } else {
                Box::new(socket)
            };
            if let Ok(headers) = read_headers(&mut stream) {
                captured.lock().unwrap().push(headers.clone());
                let _ = stream.write_all(reply(&headers).as_bytes());
                let _ = stream.flush();
            }
        }
    });
    Server {
        address,
        requests,
        stop,
        worker: Some(worker),
    }
}
trait ReadWrite: Read + Write {}
impl<T: Read + Write> ReadWrite for T {}

fn proxy() -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let requests: Requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let worker = thread::spawn(move || {
        while !stopping.load(Ordering::Relaxed) {
            let (mut socket, _) = match listener.accept() {
                Ok(value) => value,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("proxy accept: {error}"),
            };
            socket.set_nonblocking(false).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let Ok(headers) = read_headers(&mut socket) else {
                continue;
            };
            captured.lock().unwrap().push(headers.clone());
            if headers.starts_with("CONNECT ") {
                let authority = headers.split_whitespace().nth(1).unwrap();
                let port: u16 = authority.rsplit(':').next().unwrap().parse().unwrap();
                let mut target = TcpStream::connect(("127.0.0.1", port)).unwrap();
                target
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                socket
                    .write_all(b"HTTP/1.1 200 Connection established\r\n\r\n")
                    .unwrap();
                let mut input = socket.try_clone().unwrap();
                let mut output = target.try_clone().unwrap();
                let transfer = thread::spawn(move || {
                    let _ = std::io::copy(&mut input, &mut output);
                    let _ = output.shutdown(std::net::Shutdown::Write);
                });
                let _ = std::io::copy(&mut target, &mut socket);
                let _ = socket.shutdown(std::net::Shutdown::Both);
                transfer.join().unwrap();
            } else {
                let _ = socket.write_all(response("proxy marker").as_bytes());
            }
        }
    });
    Server {
        address,
        requests,
        stop,
        worker: Some(worker),
    }
}

#[derive(Default)]
struct FixtureResolver {
    calls: Mutex<Vec<String>>,
    fail: bool,
}
impl Resolve for FixtureResolver {
    fn resolve(&self, name: Name) -> Resolving {
        self.calls.lock().unwrap().push(name.as_str().to_owned());
        let fail = self.fail;
        Box::pin(async move {
            if fail {
                Err("synthetic resolver failure".into())
            } else {
                Ok(Box::new(vec![SocketAddr::from(([127, 0, 0, 1], 0))].into_iter()) as Addrs)
            }
        })
    }
}
fn client(policy: &RoutePolicy, dns: Arc<FixtureResolver>, timeout: Duration) -> Client {
    client_builder(policy, dns, timeout)
        .unwrap()
        .add_root_certificate(
            reqwest::Certificate::from_der(include_bytes!("fixtures/ca.der")).unwrap(),
        )
        .build()
        .unwrap()
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}
fn tls_url(host: &str, server: &Server) -> String {
    format!("https://{host}:{}/source", server.address.port())
}
fn credentials() -> HeaderMap {
    let mut headers = HeaderMap::new();
    for key in [
        "authorization",
        "proxy-authorization",
        "cookie",
        "x-source-credential",
    ] {
        headers.insert(key, "synthetic-p006-only".parse().unwrap());
    }
    headers
}
fn has_credential(request: &str, name: &str) -> bool {
    request
        .to_ascii_lowercase()
        .contains(&format!("\r\n{name}:"))
}

#[test]
fn direct_and_redirect_use_custom_dns_strip_all_source_credentials_and_bypass_proxy() {
    let marker = proxy();
    let b = serve(true, |_| response("origin B"));
    let b_url = tls_url("b.p006.test", &b);
    let a = serve(true, move |_| redirect(&b_url));
    let dns = Arc::new(FixtureResolver::default());
    let client = client(&RoutePolicy::Direct, dns.clone(), Duration::from_secs(3));
    let result = runtime()
        .block_on(fetch_on_client(
            &client,
            &tls_url("a.p006.test", &a),
            credentials(),
            Duration::from_secs(3),
        ))
        .unwrap();
    assert!(matches!(result.result, FetchResult::Modified { body, .. } if body=="origin B"));
    assert_eq!(*dns.calls.lock().unwrap(), ["a.p006.test", "b.p006.test"]);
    for name in [
        "authorization",
        "proxy-authorization",
        "cookie",
        "x-source-credential",
    ] {
        assert!(has_credential(&a.requests.lock().unwrap()[0], name));
        assert!(!has_credential(&b.requests.lock().unwrap()[0], name));
    }
    assert_eq!(result.hops.len(), 2);
    assert!(marker.requests.lock().unwrap().is_empty());
}

#[test]
fn via_redirect_pins_explicit_instance_and_never_uses_direct_dns() {
    let marker = proxy();
    let b = serve(true, |_| response("proxied B"));
    let b_url = tls_url("b.p006.test", &b);
    let a = serve(true, move |_| redirect(&b_url));
    let dns = Arc::new(FixtureResolver::default());
    let instance =
        RunningProxy::from_current(Some(("last-valid-42".into(), marker.address))).unwrap();
    let policy = RoutePolicy::ViaRunningProxy(instance.clone());
    let client = client(&policy, dns.clone(), Duration::from_secs(3));
    let result = runtime()
        .block_on(fetch_on_client(
            &client,
            &tls_url("a.p006.test", &a),
            credentials(),
            Duration::from_secs(3),
        ))
        .unwrap();
    assert!(matches!(result.result, FetchResult::Modified { body, .. } if body=="proxied B"));
    assert_eq!(instance.instance_id, "last-valid-42");
    assert!(dns.calls.lock().unwrap().is_empty());
    assert_eq!(marker.requests.lock().unwrap().len(), 2);
    assert!(result.hops.iter().all(|h| h.peer == Some(marker.address)));
    assert!(!has_credential(
        &b.requests.lock().unwrap()[0],
        "authorization"
    ));
}

#[test]
fn unavailable_and_invalid_proxy_never_start_a_candidate_or_fall_back() {
    assert!(matches!(
        RunningProxy::from_current(None),
        Err(OutboundError::ProxyUnavailable)
    ));
    assert!(matches!(
        RunningProxy::from_current(Some(("bad".into(), "1.1.1.1:80".parse().unwrap()))),
        Err(OutboundError::InvalidProxy)
    ));
    let closed = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = closed.local_addr().unwrap();
    drop(closed);
    let origin = serve(false, |_| response("direct would succeed"));
    let dns = Arc::new(FixtureResolver::default());
    let policy = RoutePolicy::ViaRunningProxy(
        RunningProxy::from_current(Some(("dead-last-valid".into(), address))).unwrap(),
    );
    let result = runtime().block_on(fetch_on_client(
        &client(&policy, dns.clone(), Duration::from_secs(1)),
        &format!("http://{}/source", origin.address),
        HeaderMap::new(),
        Duration::from_secs(1),
    ));
    assert!(result.is_err());
    assert!(origin.requests.lock().unwrap().is_empty());
    assert!(dns.calls.lock().unwrap().is_empty());
    // 没有subscription/profile/runtime也可以单独构建Direct，失败proxy不影响它。
    assert!(
        runtime()
            .block_on(fetch_on_client(
                &client(&RoutePolicy::Direct, dns, Duration::from_secs(1)),
                &format!("http://{}/source", origin.address),
                HeaderMap::new(),
                Duration::from_secs(1)
            ))
            .is_ok()
    );
}

#[test]
fn custom_resolver_failure_does_not_fall_back_to_system_localhost() {
    let origin = serve(true, |_| response("system localhost would work"));
    let dns = Arc::new(FixtureResolver {
        fail: true,
        ..Default::default()
    });
    let result = runtime().block_on(fetch_on_client(
        &client(&RoutePolicy::Direct, dns.clone(), Duration::from_secs(1)),
        &tls_url("localhost", &origin),
        HeaderMap::new(),
        Duration::from_secs(1),
    ));
    assert!(result.is_err());
    assert_eq!(*dns.calls.lock().unwrap(), ["localhost"]);
    assert!(origin.requests.lock().unwrap().is_empty());
}

#[test]
fn direct_ignores_all_proxy_environment_in_an_isolated_child() {
    if let Ok(source) = std::env::var("P006_ENV_CHILD_SOURCE") {
        let client = OutboundClient::new(
            RoutePolicy::Direct,
            &FetchClientOptions {
                timeout: Duration::from_secs(1),
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            runtime()
                .block_on(client.fetch(&source, ConditionalHeaders::default()))
                .is_ok()
        );
        return;
    }
    let origin = serve(false, |_| response("no subscription bootstrap"));
    let marker = proxy();
    let status=std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact","subscription::outbound::tests::direct_ignores_all_proxy_environment_in_an_isolated_child"])
        .env("P006_ENV_CHILD_SOURCE",format!("http://{}/source",origin.address))
        .env("HTTP_PROXY",format!("http://{}",marker.address)).env("HTTPS_PROXY",format!("http://{}",marker.address))
        .env("ALL_PROXY",format!("http://{}",marker.address)).env("http_proxy",format!("http://{}",marker.address))
        .env("https_proxy",format!("http://{}",marker.address)).env("all_proxy",format!("http://{}",marker.address))
        .env("NO_PROXY","").env("no_proxy","").status().unwrap();
    assert!(status.success());
    assert_eq!(origin.requests.lock().unwrap().len(), 1);
    assert!(marker.requests.lock().unwrap().is_empty());
}

#[test]
fn fake_ip_and_failed_empty_or_invalid_dns_answers_are_not_real_bootstrap() {
    for data in [
        "198.18.0.1",
        "198.19.255.254",
        "fc00::1",
        "0.0.0.0",
        "127.0.0.1",
        "invalid",
    ] {
        assert!(
            real_addresses(DohResponse {
                status: 0,
                answers: vec![DohAnswer {
                    kind: 1,
                    data: data.into()
                }]
            })
            .is_err()
        );
    }
    assert!(is_fake_ip("fc00::1".parse().unwrap()));
    assert!(!is_fake_ip("1.1.1.1".parse().unwrap()));
    assert!(
        real_addresses(DohResponse {
            status: 3,
            answers: vec![]
        })
        .is_err()
    );
    assert!(
        real_addresses(DohResponse {
            status: 0,
            answers: vec![]
        })
        .is_err()
    );
    assert_eq!(
        real_addresses(DohResponse {
            status: 0,
            answers: vec![DohAnswer {
                kind: 1,
                data: "9.9.9.9".into()
            }]
        })
        .unwrap(),
        vec!["9.9.9.9".parse::<IpAddr>().unwrap()]
    );
}

#[test]
fn stalled_dns_is_cancelled_by_the_same_request_total_budget() {
    struct StalledDns;
    impl Resolve for StalledDns {
        fn resolve(&self, _: Name) -> Resolving {
            Box::pin(async {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Err("late DNS".into())
            })
        }
    }
    let origin = serve(true, |_| response("must not be reached"));
    let client = client_builder(
        &RoutePolicy::Direct,
        Arc::new(StalledDns),
        Duration::from_millis(40),
    )
    .unwrap()
    .build()
    .unwrap();
    let start = Instant::now();
    assert!(
        runtime()
            .block_on(fetch_on_client(
                &client,
                &tls_url("localhost", &origin),
                HeaderMap::new(),
                Duration::from_millis(40)
            ))
            .is_err()
    );
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(origin.requests.lock().unwrap().is_empty());
}

#[test]
fn same_origin_redirect_retains_only_its_source_credentials() {
    let origin = serve(true, |request| {
        if request.starts_with("GET /source ") {
            redirect("/final")
        } else {
            response("same origin")
        }
    });
    let dns = Arc::new(FixtureResolver::default());
    let c = client(&RoutePolicy::Direct, dns, Duration::from_secs(2));
    assert!(
        runtime()
            .block_on(fetch_on_client(
                &c,
                &tls_url("a.p006.test", &origin),
                credentials(),
                Duration::from_secs(2)
            ))
            .is_ok()
    );
    let requests = origin.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|r| has_credential(r, "authorization") && has_credential(r, "cookie"))
    );
}

#[test]
fn downgrade_userinfo_redirect_limit_and_read_total_timeout_are_bounded() {
    let dns = Arc::new(FixtureResolver::default());
    let plain = serve(false, |_| response("must not receive downgrade"));
    let url = format!("http://{}/target", plain.address);
    let a = serve(true, move |_| redirect(&url));
    let c = client(&RoutePolicy::Direct, dns.clone(), Duration::from_secs(2));
    let result = runtime().block_on(fetch_on_client(
        &c,
        &tls_url("a.p006.test", &a),
        HeaderMap::new(),
        Duration::from_secs(2),
    ));
    assert!(matches!(
        result,
        Err(OutboundError::Fetch(FetchError::InvalidUrl))
    ));
    assert!(plain.requests.lock().unwrap().is_empty());
    assert!(
        runtime()
            .block_on(fetch_on_client(
                &c,
                &format!("https://user:synthetic@a.p006.test:{}/", a.address.port()),
                HeaderMap::new(),
                Duration::from_secs(1)
            ))
            .is_err()
    );
    let looping = serve(true, |_| redirect("/again"));
    assert!(
        runtime()
            .block_on(fetch_on_client(
                &c,
                &tls_url("a.p006.test", &looping),
                HeaderMap::new(),
                Duration::from_secs(2)
            ))
            .is_err()
    );
    assert_eq!(looping.requests.lock().unwrap().len(), 6);
    let stalled = serve(true, |_| {
        thread::sleep(Duration::from_millis(200));
        response("late")
    });
    let start = Instant::now();
    assert!(
        runtime()
            .block_on(fetch_on_client(
                &client(&RoutePolicy::Direct, dns, Duration::from_millis(40)),
                &tls_url("a.p006.test", &stalled),
                HeaderMap::new(),
                Duration::from_millis(40)
            ))
            .is_err()
    );
    assert!(start.elapsed() < Duration::from_secs(1));
}

// 保护正式 OutboundClient 的 Direct HTTPS / redirect / TLS / 容量契约。
#[test]
fn production_client_direct_tls_redirect_and_failure_causes() {
    let target = serve(true, |_| response("production marker"));
    let next = tls_url("b.p006.test", &target);
    let source = serve(true, move |_| redirect(&next));
    let dns = Arc::new(FixtureResolver::default());
    let policy = RoutePolicy::Direct;
    let typed = OutboundClient {
        client: client(&policy, dns.clone(), Duration::from_secs(2)),
        policy,
        timeout: Duration::from_secs(2),
    };
    let FetchResult::Modified { body, .. } = runtime()
        .block_on(typed.fetch(
            &tls_url("a.p006.test", &source),
            ConditionalHeaders::default(),
        ))
        .unwrap()
    else {
        panic!()
    };
    assert_eq!(body, "production marker");
    assert_eq!(*dns.calls.lock().unwrap(), ["a.p006.test", "b.p006.test"]);
    for request in source
        .requests
        .lock()
        .unwrap()
        .iter()
        .chain(target.requests.lock().unwrap().iter())
    {
        assert!(!has_credential(request, "authorization"));
        assert!(!has_credential(request, "cookie"));
    }
    let invalid_tls =
        OutboundClient::with_resolver(RoutePolicy::Direct, &FetchClientOptions::default(), dns)
            .unwrap();
    let error = runtime()
        .block_on(invalid_tls.fetch(
            &tls_url("a.p006.test", &source),
            ConditionalHeaders::default(),
        ))
        .unwrap_err();
    assert!(matches!(
        error,
        OutboundError::Fetch(FetchError::CertificateInvalid)
    ));
    let large = serve(false, |_| response(&"x".repeat(4 * 1024 * 1024 + 1)));
    assert_eq!(
        runtime().block_on(typed.resource(&format!("http://{}/", large.address))),
        Err(OutboundError::Fetch(FetchError::BodyTooLarge))
    );
}

// 保护正式 bootstrap 的固定 TLS 上游、真实 A、FakeIP 与失败无系统 DNS fallback。
#[test]
fn production_bootstrap_https_json_and_fakeip_rejection() {
    for (answer, success) in [("1.1.1.1", true), ("198.18.0.1", false)] {
        let upstream = serve(true, move |_| {
            response(&format!(
                r#"{{"Status":0,"Answer":[{{"type":1,"data":"{answer}"}}]}}"#
            ))
        });
        let dns = BootstrapResolver {
            client: client(
                &RoutePolicy::Direct,
                Arc::new(FixtureResolver::default()),
                Duration::from_secs(2),
            ),
            observations: Default::default(),
            timeout: Duration::from_secs(2),
            endpoint: tls_url("a.p006.test", &upstream),
            upstream_ip: upstream.address.ip(),
            fixture_skip_route_validation: true,
        };
        let result = runtime().block_on(dns.resolve("source.test".parse().unwrap()));
        assert_eq!(result.is_ok(), success);
        let observations = dns.observations();
        assert_eq!(observations.len(), 1);
        assert!(observations[0].tls_verified);
        assert_eq!(observations[0].upstream_peer, Some(upstream.address));
        let requests = upstream.requests.lock().unwrap();
        assert!(requests[0].contains("name=source.test&type=A"));
        assert!(!requests[0].contains("authorization"));
    }
}

// 保护已绑定代理的 in-flight 取消、旧 client 拒绝及失效不换 Direct。
#[test]
fn production_proxy_revoke_cancels_inflight_and_old_client() {
    let started = Arc::new(AtomicBool::new(false));
    let observed = started.clone();
    let proxy_server = serve(false, move |_| {
        observed.store(true, Ordering::Release);
        thread::sleep(Duration::from_millis(250));
        response("late marker")
    });
    let proxy = RunningProxy::ready("managed-fixture".into(), proxy_server.address).unwrap();
    let typed = OutboundClient::new(
        RoutePolicy::ViaRunningProxy(proxy.clone()),
        &FetchClientOptions::default(),
    )
    .unwrap();
    let revoke = proxy.clone();
    let task = thread::spawn(move || {
        while !started.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(2));
        }
        revoke.invalidate();
    });
    assert_eq!(
        runtime().block_on(typed.fetch(
            "http://127.0.0.1:9/synthetic-secret",
            ConditionalHeaders::default()
        )),
        Err(OutboundError::ProxyUnavailable)
    );
    task.join().unwrap();
    assert_eq!(
        runtime().block_on(typed.resource("http://127.0.0.1:9/")),
        Err(OutboundError::ProxyUnavailable)
    );
    assert_eq!(proxy_server.requests.lock().unwrap().len(), 1);
    assert!(!format!("{proxy:?}").contains("token"));
}

// 保护完整生产 HTTP→Bootstrap→HTTPS 错误链，避免把 TLS/HTTP/DNS 都折成未知。
#[test]
fn production_bootstrap_failures_reach_typed_fetch_errors() {
    for (body, expected) in [
        (r#"{"Status":3}"#, FetchError::DnsStatus(3)),
        (r#"{"Status":0,"Answer":[]}"#, FetchError::NoDnsAddress),
        ("not json", FetchError::InvalidDnsResponse),
        (
            r#"{"Status":0,"Answer":[{"type":1,"data":"198.18.0.1"}]}"#,
            FetchError::FakeIpAddress,
        ),
    ] {
        let upstream = serve(true, move |_| response(body));
        let resolver = BootstrapResolver {
            client: client(
                &RoutePolicy::Direct,
                Arc::new(FixtureResolver::default()),
                Duration::from_secs(2),
            ),
            observations: Default::default(),
            timeout: Duration::from_secs(2),
            endpoint: tls_url("a.p006.test", &upstream),
            upstream_ip: upstream.address.ip(),
            fixture_skip_route_validation: true,
        };
        let typed = OutboundClient::with_resolver(
            RoutePolicy::Direct,
            &FetchClientOptions {
                timeout: Duration::from_secs(2),
                ..Default::default()
            },
            Arc::new(resolver),
        )
        .unwrap();
        assert_eq!(
            runtime().block_on(typed.fetch(
                "https://unresolvable.p006.test/?token=synthetic-only",
                ConditionalHeaders::default()
            )),
            Err(OutboundError::Fetch(expected))
        );
    }
    let upstream = serve(true, |_| response(r#"{"Status":0}"#));
    let options = FetchClientOptions {
        timeout: Duration::from_secs(2),
        ..Default::default()
    };
    let resolver = BootstrapResolver {
        client: client_builder(
            &RoutePolicy::Direct,
            Arc::new(FixtureResolver::default()),
            options.timeout,
        )
        .unwrap()
        .build()
        .unwrap(),
        observations: Default::default(),
        timeout: options.timeout,
        endpoint: tls_url("a.p006.test", &upstream),
        upstream_ip: upstream.address.ip(),
        fixture_skip_route_validation: true,
    };
    let typed =
        OutboundClient::with_resolver(RoutePolicy::Direct, &options, Arc::new(resolver)).unwrap();
    assert_eq!(
        runtime().block_on(typed.fetch(
            "https://unresolvable.p006.test/?token=synthetic-only",
            ConditionalHeaders::default()
        )),
        Err(OutboundError::Fetch(FetchError::CertificateInvalid))
    );
}
