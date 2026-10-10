//! 每项测试保护 Veyra 的版本选择、固定来源、下载保旧/取消/完整性交接。
//! 仅普通用户 loopback HTTP fixture；不启动 sing-box/helper 或操作系统网络。
use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicUsize, Ordering},
    thread,
};
use veyra_core::subscription::outbound::{BootstrapResolver, RunningProxy};

fn client() -> UpdateClient {
    UpdateClient::new(
        &RoutePolicy::Direct,
        Arc::new(BootstrapResolver::new(Duration::from_secs(1)).unwrap()),
    )
    .unwrap()
}
fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}
fn sample(body: &[u8]) -> (Release, Manifest) {
    let name = "VeyraPrototype-0.1.1-macos-arm64.zip";
    let manifest = Manifest {
        schema: 1,
        repository: REPOSITORY.into(),
        app_version: "0.1.1".into(),
        helper_version: "0.1.0".into(),
        sing_box_version: "1.14.0".into(),
        resources_version: "0.1.1".into(),
        platform: "macos-arm64".into(),
        minimum_system_version: "26.0".into(),
        asset_name: name.into(),
        size: body.len() as u64,
        sha256: format!("{:x}", Sha256::digest(body)),
        source_commit: "7".repeat(40),
        cargo_lock_sha256: "a".repeat(64),
    };
    let release = Release {
        tag_name: "v0.1.1".into(),
        html_url: format!("https://github.com/{REPOSITORY}/releases/tag/v0.1.1"),
        body: Some("原型发行说明，下载后手动安装".into()),
        draft: false,
        prerelease: false,
        assets: vec![
            Asset {
                name: name.into(),
                size: manifest.size,
                browser_download_url: asset_url("0.1.1", name),
            },
            Asset {
                name: MANIFEST_NAME.into(),
                size: serde_json::to_vec(&manifest).unwrap().len() as u64,
                browser_download_url: asset_url("0.1.1", MANIFEST_NAME),
            },
        ],
    };
    (release, manifest)
}
fn read_request(stream: &mut TcpStream) -> String {
    // macOS accept 会继承监听 socket 的非阻塞状态；连接读写仍要受正常timeout控制。
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut data = Vec::new();
    let mut byte = [0];
    while !data.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        data.push(byte[0]);
    }
    String::from_utf8(data).unwrap()
}
fn server(replies: Vec<Vec<u8>>) -> (Url, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let handle = thread::spawn(move || {
        let mut requests = Vec::new();
        for reply in replies {
            let deadline = std::time::Instant::now() + Duration::from_secs(4);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            && std::time::Instant::now() < deadline =>
                    {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            };
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            requests.push(read_request(&mut stream));
            // 失败/取消时客户端可以提前关闭流；这不是服务器业务失败。
            let _ = stream.write_all(&reply);
        }
        requests
    });
    (url, handle)
}
fn ok(body: &[u8]) -> Vec<u8> {
    let mut reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    reply.extend_from_slice(body);
    reply
}
fn directory() -> std::path::PathBuf {
    // macOS 并发测试可能读到同一时钟值；进程内序号避免自有目录互相碰撞。
    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "veyra-p008-test-{}-{:x}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&path).unwrap();
    path
}

#[test]
fn stable_version_check_never_downgrades_or_guesses_prerelease() {
    assert!(Version::parse("0.10.0").unwrap() > Version::parse("0.9.9").unwrap());
    assert_eq!(
        Version::parse("0.1.0").unwrap(),
        Version::parse("0.1.0").unwrap()
    );
    for bad in ["0.1", "01.2.3", "1.2.3-beta", "v1.2.3", "1.2.3.4"] {
        assert!(Version::parse(bad).is_err());
    }
}

#[test]
fn release_binding_rejects_wrong_repository_asset_platform_size_and_component() {
    let (release, mut manifest) = sample(b"package");
    manifest.validate(&release).unwrap();
    manifest.repository = "other/repo".into();
    assert!(manifest.validate(&release).is_err());
    manifest.repository = REPOSITORY.into();
    manifest.platform = "windows-x64".into();
    assert!(manifest.validate(&release).is_err());
    manifest.platform = "macos-arm64".into();
    manifest.size += 1;
    assert!(manifest.validate(&release).is_err());
    manifest.size -= 1;
    manifest.resources_version = "0.1.0".into();
    assert!(manifest.validate(&release).is_err());
    manifest.resources_version = "0.1.1".into();
    manifest.asset_name = "other.zip".into();
    assert!(manifest.validate(&release).is_err());
    let (mut release, manifest) = sample(b"package");
    release.assets[0].browser_download_url = "https://github.com/other/repo/other.zip".into();
    assert!(manifest.validate(&release).is_err());
    release.prerelease = true;
    assert!(release.version().is_err());
}

#[test]
fn checks_release_notes_and_downloads_binary_larger_than_subscription_limit() {
    let body = vec![0xff; 4 * 1024 * 1024 + 17];
    let (release, manifest) = sample(&body);
    let (url, server) = server(vec![
        ok(&serde_json::to_vec(&release).unwrap()),
        ok(&serde_json::to_vec(&manifest).unwrap()),
        ok(&body),
    ]);
    let mut client = client();
    client.fixture = Some(url);
    let root = directory();
    let output = root.join(&manifest.asset_name);
    runtime().block_on(async {
        let (_tx, mut cancel) = watch::channel(false);
        let (release, checked) = client.check(&mut cancel).await.unwrap();
        assert!(release.body.unwrap().contains("手动安装"));
        client
            .download(&checked, &output, &mut cancel)
            .await
            .unwrap();
    });
    assert_eq!(std::fs::read(&output).unwrap(), body);
    assert!(!output.with_extension("zip.part").exists());
    let requests = server.join().unwrap();
    assert!(requests[0].starts_with("GET /repos/lifei6671/veyra/releases/latest "));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_bad_hash_short_and_oversized_bytes_and_preserves_existing_files() {
    for mode in ["hash", "short", "oversized"] {
        let body = b"package";
        let (_, mut manifest) = sample(body);
        let reply = match mode {
            "hash" => {
                manifest.sha256 = "0".repeat(64);
                ok(body)
            }
            // 无Content-Length时依然按实际累计字节与最终摘要检查。
            "short" => b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\npack".to_vec(),
            _ => b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\npackage-extra".to_vec(),
        };
        let (url, server) = server(vec![reply]);
        let mut client = client();
        client.fixture = Some(url);
        let root = directory();
        let output = root.join(&manifest.asset_name);
        runtime().block_on(async {
            let (_tx, mut cancel) = watch::channel(false);
            assert!(
                client
                    .download(&manifest, &output, &mut cancel)
                    .await
                    .is_err()
            );
        });
        assert!(!output.exists());
        assert!(!output.with_extension("zip.part").exists());
        server.join().unwrap();
        std::fs::write(&output, "旧包").unwrap();
        runtime().block_on(async {
            let (_tx, mut cancel) = watch::channel(false);
            assert!(
                client
                    .download(&manifest, &output, &mut cancel)
                    .await
                    .is_err()
            );
        });
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "旧包");
        std::fs::remove_file(&output).unwrap();
        std::fs::write(output.with_extension("zip.part"), "其它下载").unwrap();
        runtime().block_on(async {
            let (_tx, mut cancel) = watch::channel(false);
            assert!(
                client
                    .download(&manifest, &output, &mut cancel)
                    .await
                    .is_err()
            );
        });
        assert_eq!(
            std::fs::read_to_string(output.with_extension("zip.part")).unwrap(),
            "其它下载"
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn cancellation_during_stalled_transfer_removes_only_owned_partial() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = Url::parse(&format!("http://{}/", listener.local_addr().unwrap())).unwrap();
    let (started_tx, started_rx) = watch::channel(false);
    let server = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(4);
        let mut stream = loop {
            if let Ok((stream, _)) = listener.accept() {
                break stream;
            }
            assert!(std::time::Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        };
        read_request(&mut stream);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\np")
            .unwrap();
        started_tx.send(true).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let mut byte = [0];
        // 取消未读完响应可产生正常EOF或TCP reset；两者均证明本次连接已关闭。
        match stream.read(&mut byte) {
            Ok(0) => {}
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => {}
            other => panic!("取消后连接未关闭：{other:?}"),
        }
    });
    let mut client = client();
    client.fixture = Some(url);
    let (_, manifest) = sample(b"package");
    let root = directory();
    let output = root.join(&manifest.asset_name);
    runtime().block_on(async {
        let (tx, mut cancel) = watch::channel(false);
        let signal = async move {
            let mut started = started_rx;
            if !*started.borrow() {
                started.changed().await.unwrap();
            }
            tx.send(true).unwrap();
        };
        let (result, _) = tokio::join!(client.download(&manifest, &output, &mut cancel), signal);
        assert!(result.unwrap_err().to_string().contains("取消"));
    });
    server.join().unwrap();
    assert!(!output.exists());
    assert!(!output.with_extension("zip.part").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn redirects_keep_explicit_proxy_and_stopped_proxy_never_falls_back() {
    let (_, manifest) = sample(b"package");
    let (proxy, server) = server(vec![
        b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec(),
        ok(b"package"),
    ]);
    let policy = RoutePolicy::ViaRunningProxy(
        RunningProxy::from_current(Some((
            "fixture-current".into(),
            format!("{}:{}", proxy.host_str().unwrap(), proxy.port().unwrap())
                .parse()
                .unwrap(),
        )))
        .unwrap(),
    );
    let mut client = UpdateClient::new(
        &policy,
        Arc::new(BootstrapResolver::new(Duration::from_secs(1)).unwrap()),
    )
    .unwrap();
    // 目标端口没有监听：成功字节只能来自显式代理。
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    client.fixture =
        Some(Url::parse(&format!("http://{}/", target.local_addr().unwrap())).unwrap());
    let root = directory();
    let output = root.join(&manifest.asset_name);
    runtime().block_on(async {
        let (_tx, mut cancel) = watch::channel(false);
        client
            .download(&manifest, &output, &mut cancel)
            .await
            .unwrap();
    });
    let requests = server.join().unwrap();
    assert!(
        requests
            .iter()
            .all(|r| r.starts_with("GET http://127.0.0.1:"))
    );
    assert!(requests[1].contains("/next "));
    std::fs::remove_file(&output).unwrap();
    runtime().block_on(async {
        let (_tx, mut cancel) = watch::channel(false);
        assert!(
            client
                .download(&manifest, &output, &mut cancel)
                .await
                .is_err()
        );
    });
    target.set_nonblocking(true).unwrap();
    assert_eq!(
        target.accept().unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    assert!(!output.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_external_redirect_before_second_request() {
    let (url, server) = server(vec![b"HTTP/1.1 302 Found\r\nLocation: http://outside.invalid/package\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()]);
    let mut client = client();
    client.fixture = Some(url);
    let (_, manifest) = sample(b"package");
    let root = directory();
    let output = root.join(&manifest.asset_name);
    runtime().block_on(async {
        let (_tx, mut cancel) = watch::channel(false);
        assert!(
            client
                .download(&manifest, &output, &mut cancel)
                .await
                .unwrap_err()
                .to_string()
                .contains("GitHub HTTPS")
        );
    });
    assert_eq!(server.join().unwrap().len(), 1);
    assert!(!output.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn metadata_limit_status_redirect_limit_and_pre_cancel_are_bounded() {
    // 防止版本检查挂住/消耗无界内存；取消尚未发送请求也必须清理临时文件。
    let oversized = vec![b'x'; MAX_METADATA + 1];
    let redirect =
        b"HTTP/1.1 302 Found\r\nLocation: /next\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_vec();
    for replies in [
        vec![ok(&oversized)],
        vec![b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()],
        vec![redirect; 6],
    ] {
        let (url, server) = server(replies);
        let mut client = client();
        client.fixture = Some(url);
        runtime().block_on(async {
            let (_tx, mut cancel) = watch::channel(false);
            assert!(client.check(&mut cancel).await.is_err());
        });
        server.join().unwrap();
    }
    let root = directory();
    let (_, manifest) = sample(b"package");
    let output = root.join(&manifest.asset_name);
    runtime().block_on(async {
        let (_tx, mut cancel) = watch::channel(true);
        assert!(
            client()
                .download(&manifest, &output, &mut cancel)
                .await
                .unwrap_err()
                .to_string()
                .contains("取消")
        );
    });
    assert!(!output.exists());
    assert!(!output.with_extension("zip.part").exists());
    std::fs::remove_dir_all(root).unwrap();
}
