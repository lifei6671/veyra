use super::*;
// 保护三个真实 provider 字段、缺失值和业务失败，不制造国家/ASN。
#[test]
fn three_provider_schemas_and_unknown_fields() {
    let ip = "8.8.8.8".parse().unwrap();
    for (provider, body) in [
        (
            GeoIpProvider::IpSb,
            r#"{"ip":"8.8.8.8","asn":15169,"country_code":"us","organization":"Google","city":"Test"}"#,
        ),
        (
            GeoIpProvider::IpWhoIs,
            r#"{"ip":"8.8.8.8","success":true,"country_code":"US","connection":{"asn":15169,"org":"Google"}}"#,
        ),
        (
            GeoIpProvider::IpApiIs,
            r#"{"ip":"8.8.8.8","asn":{"asn":15169,"org":"Google"},"location":{"country_code":"US","state":"Test"}}"#,
        ),
    ] {
        let value = normalize_geo_ip(provider, body, Some(ip)).unwrap();
        assert_eq!(value.asn, Some(15169));
        assert_eq!(value.country_code.as_deref(), Some("US"));
        assert_eq!(value.organization.as_deref(), Some("Google"));
    }
    let flat = normalize_geo_ip(GeoIpProvider::IpApiIs, r#"{"ip":"8.8.8.8","company":"Google LLC","asn":"AS15169 Google LLC","country":"United States","city":"Mountain View","region":"California"}"#, Some(ip)).unwrap();
    assert_eq!(flat.asn, Some(15169));
    assert_eq!(flat.organization.as_deref(), Some("Google LLC"));
    assert_eq!(flat.country_code, None);
    let unknown = normalize_geo_ip(GeoIpProvider::IpSb, "{}", Some(ip)).unwrap();
    assert_eq!(unknown.asn, None);
    assert_eq!(unknown.country, None);
    assert_eq!(unknown.organization, None);
    for body in [
        r#"{"success":false,"message":"private-error"}"#,
        r#"{"error":"bad request"}"#,
    ] {
        assert_eq!(
            normalize_geo_ip(GeoIpProvider::IpWhoIs, body, Some(ip)),
            Err(NetworkError::ProviderRejected)
        );
    }
    assert_eq!(
        normalize_geo_ip(GeoIpProvider::IpSb, r#"{"ip":"1.1.1.1"}"#, Some(ip)),
        Err(NetworkError::InvalidResponse)
    );
    assert!(normalize_geo_ip(GeoIpProvider::IpApiIs, "[]", Some(ip)).is_err());
}
// 保护不可用/关闭不发网络请求，且请求中无订阅凭据或自动 provider fallback。
#[test]
fn explicit_paths_unavailable_shutdown_and_geo_queries() {
    let service = NetworkService::new(Default::default());
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert_eq!(
        rt.block_on(service.site_latency(
            "https://example.invalid",
            Duration::from_secs(1),
            RequestedPath::ViaRunningProxy
        )),
        Err(NetworkError::ProxyUnavailable)
    );
    assert_eq!(
        rt.block_on(service.node_latency(
            &NodeId("anything".into()),
            "http://www.gstatic.com/generate_204",
            Duration::from_secs(1)
        )),
        Err(NetworkError::UnsupportedNodeTestUrl)
    );
    service.shutdown();
    assert_eq!(
        rt.block_on(service.site_latency(
            "http://127.0.0.1:9",
            Duration::from_secs(1),
            RequestedPath::Direct
        )),
        Err(NetworkError::Closed)
    );
    for provider in [
        GeoIpProvider::IpSb,
        GeoIpProvider::IpWhoIs,
        GeoIpProvider::IpApiIs,
    ] {
        let url =
            reqwest::Url::parse(&provider.url(Some("2001:4860::8888".parse().unwrap()))).unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.username(), "");
        assert_eq!(url.password(), None);
        assert!(!url.as_str().contains("token"));
    }
}

// 实际 loopback controller 保护 GET 编码、认证、未知节点、零值未知、畸形/redirect。
#[test]
fn node_delay_fixed_controller_encoding_unknown_and_response_boundaries() {
    use crate::singbox::{GeneratedConfig, clash_api::ManagedControllerEndpoint};
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    for (reply, expected) in [
        ("HTTP/1.1 200 OK\r\n", Ok(Some(17))),
        ("HTTP/1.1 200 OK\r\n", Ok(None)),
        (
            "HTTP/1.1 200 OK\r\n",
            Err(NetworkError::Controller(ClashApiError::InvalidResponse)),
        ),
        (
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:9/leak\r\n",
            Err(NetworkError::Controller(ClashApiError::Request(
                crate::subscription::FetchError::HttpStatus(302),
            ))),
        ),
    ] {
        let body = match &expected {
            Ok(Some(_)) => r#"{"delay":17}"#,
            Ok(None) => r#"{"delay":0}"#,
            _ => r#"{"delay":"invalid"}"#,
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let mut config: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/compiler/p2-02b-base.expected.json"
        ))
        .unwrap();
        config["experimental"]["clash_api"]["secret"] = "a".repeat(64).into();
        let config = GeneratedConfig::from_bytes(serde_json::to_vec(&config).unwrap());
        let endpoint =
            Arc::new(ManagedControllerEndpoint::from_owned_child(address, &config).unwrap());
        let mut proxy = RunningProxy::ready("current-node-test".into(), address).unwrap();
        let node = NodeId("business-node".into());
        proxy.nodes.insert(node.clone(), "node/with ?#中文".into());
        proxy.controller = Some(endpoint);
        let source = ProxySource::default();
        source.publish(proxy);
        let service = NetworkService::new(source);
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        assert_eq!(
            rt.block_on(service.node_latency(
                &NodeId("saved-not-applied".into()),
                "https://www.gstatic.com/generate_204",
                Duration::from_secs(1)
            )),
            Err(NetworkError::NodeNotApplied)
        );
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                socket.read_exact(&mut byte).unwrap();
                bytes.push(byte[0]);
            }
            let response = format!(
                "{reply}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).unwrap();
            String::from_utf8(bytes).unwrap()
        });
        let value = rt
            .block_on(service.node_latency(
                &node,
                "https://www.gstatic.com/generate_204?test=1",
                Duration::from_secs(1),
            ))
            .map(|v| v.milliseconds);
        assert_eq!(value, expected);
        let request = server.join().unwrap();
        assert!(request.starts_with("GET /proxies/node%2Fwith%20%3F%23%E4%B8%AD%E6%96%87/delay?url=https%3A%2F%2Fwww.gstatic.com%2Fgenerate_204%3Ftest%3D1&timeout=1000 "));
        assert!(request.to_lowercase().contains("authorization: bearer "));
    }
}

// 正式 site Service 真实 204 与指定代理；关闭取消下载，不等完整网络时限。
#[test]
fn site_service_direct_proxy_and_shutdown_cancel_real_request() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
    };
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    for path in [RequestedPath::Direct, RequestedPath::ViaRunningProxy] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let source = ProxySource::default();
        source.publish(RunningProxy::ready("owned-site".into(), address).unwrap());
        let service = NetworkService::new(source);
        let server = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            let mut bytes = [0; 8192];
            let count = socket.read(&mut bytes).unwrap();
            socket
                .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .unwrap();
            String::from_utf8_lossy(&bytes[..count]).into_owned()
        });
        let value = rt
            .block_on(service.site_latency(
                &format!("http://{address}/site"),
                Duration::from_secs(1),
                path,
            ))
            .unwrap();
        assert!(value.milliseconds.is_some());
        let request = server.join().unwrap();
        match path {
            RequestedPath::Direct => {
                assert_eq!(value.path, UsedPath::Direct);
                assert!(request.starts_with("GET /site "));
            }
            RequestedPath::ViaRunningProxy => {
                assert!(matches!(value.path, UsedPath::ViaRunningProxy { .. }));
                assert!(request.starts_with(&format!("GET http://{address}/site ")));
            }
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let service = NetworkService::new(Default::default());
    let start = Instant::now();
    let stalled = format!("http://{address}/stalled");
    let error = rt.block_on(async {
        tokio::join!(
            service.site_latency(&stalled, Duration::from_secs(10), RequestedPath::Direct),
            async {
                tokio::time::sleep(Duration::from_millis(30)).await;
                service.shutdown();
            }
        )
        .0
    });
    assert_eq!(error, Err(NetworkError::Closed));
    assert!(start.elapsed() < Duration::from_secs(1));
    listener.set_nonblocking(true).unwrap();
    if let Ok((mut socket, _)) = listener.accept() {
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut bytes = [0; 8192];
        while socket.read(&mut bytes).unwrap_or(0) != 0 {}
    }
}
