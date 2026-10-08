use super::*;
use crate::{
    application::{state_access::StateAccessGate, state_service::SubscriptionAppend},
    domain::*,
    storage::JsonStateStore,
};
struct Fixture {
    root: std::path::PathBuf,
    runtime: tokio::runtime::Runtime,
    snapshots: Arc<SnapshotService>,
    service: ShareService,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("veyra-p506-{}", random().unwrap()));
        let snapshots = Arc::new(SnapshotService::new(
            JsonStateStore::new(root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        ));
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let service = ShareService::new(snapshots.clone(), runtime.handle().clone());
        let mut batch = SubscriptionAppend::default();
        for id in ["selected", "private"] {
            batch.subscriptions.push(Subscription {
                id: SubscriptionId(id.into()),
                name: id.into(),
                description: String::new(),
                source: SubscriptionSource::Manual,
                last_success_at_ms: None,
                last_attempt_at_ms: None,
                http_metadata: None,
                remote_request: None,
                update_policy: SubscriptionUpdatePolicy::manual(),
                skipped_unsupported_nodes: 0,
                document: None,
            });
            batch.providers.push(Provider {
                id: ProviderId(id.into()),
                subscription_id: SubscriptionId(id.into()),
                name: id.into(),
            });
            batch.nodes.push(ProxyNode {
                id: NodeId(id.into()),
                provider_id: ProviderId(id.into()),
                name: id.into(),
                protocol: ProxyProtocol::Shadowsocks,
                server: "example.invalid".into(),
                port: 443,
                options: ProtocolOptions::Shadowsocks {
                    method: "aes-128-gcm".into(),
                    password: format!("{id}-credential"),
                },
                transport: None,
                tls: None,
            });
        }
        snapshots
            .append(snapshots.snapshot().unwrap().config_version(), batch)
            .unwrap();
        Self {
            root,
            runtime,
            snapshots,
            service,
        }
    }
    fn draft(&self) -> SubscriptionShare {
        let socket = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut share = ShareService::draft().unwrap();
        share.name = "phone".into();
        share.listen = socket.local_addr().unwrap();
        share.host = share.listen.to_string();
        share.subscription_ids = vec![SubscriptionId("selected".into())];
        share
    }
    fn execute(&self, command: ShareCommand) -> AppState {
        self.service
            .execute(self.snapshots.snapshot().unwrap().config_version(), command)
            .unwrap()
    }
    fn get(&self, url: &str) -> Option<(u16, String)> {
        self.runtime.block_on(async {
            let response = reqwest::Client::builder()
                .no_proxy()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .unwrap()
                .get(url)
                .send()
                .await
                .ok()?;
            Some((response.status().as_u16(), response.text().await.unwrap()))
        })
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.service.shutdown();
        release(
            self.service
                .snapshots
                .snapshot()
                .unwrap()
                .app_config
                .subscription_shares
                .iter()
                .map(|s| s.listen),
        );
        std::fs::remove_dir_all(&self.root).unwrap();
    }
}
fn release(addresses: impl Iterator<Item = SocketAddr>) {
    for address in addresses {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            let socket = if address.is_ipv4() {
                tokio::net::TcpSocket::new_v4()
            } else {
                tokio::net::TcpSocket::new_v6()
            }
            .unwrap();
            socket.set_reuseaddr(true).unwrap();
            if socket.bind(address).is_ok() {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "listener not released"
            );
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
/// 保护真实 GET 仅暴露选择集合、token 撤销以及 CRUD 后无残留监听。
#[test]
fn shares_http_selected_token_rotation_disable_delete_and_release() {
    let f = Fixture::new();
    let share = f.draft();
    let address = share.listen;
    let url = share.url();
    f.execute(ShareCommand::Save(share.clone()));
    let (status, body) = f.get(&url).unwrap();
    assert_eq!(status, 200);
    assert!(body.contains("selected-credential"));
    assert!(!body.contains("private-credential"));
    let parsed = crate::subscription::parse_subscription(&body).unwrap();
    assert_eq!(parsed.nodes.len(), 1);
    assert!(parsed.skipped.is_empty());
    assert_eq!(
        f.get(&format!("http://{address}/sub/wrong")).unwrap().0,
        404
    );
    let state = f.execute(ShareCommand::Regenerate(share.id.clone()));
    let new = state.app_config.subscription_shares[0].url();
    assert_eq!(f.get(&url).unwrap().0, 404);
    assert_eq!(f.get(&new).unwrap().0, 200);
    f.execute(ShareCommand::Enabled(share.id.clone(), false));
    release([address].into_iter());
    assert!(f.get(&new).is_none());
    f.execute(ShareCommand::Enabled(share.id.clone(), true));
    assert_eq!(f.get(&new).unwrap().0, 200);
    f.execute(ShareCommand::Delete(share.id));
    release([address].into_iter());
    assert!(f.get(&new).is_none());
}
/// 保护新 Store/Service 恢复 enabled、字段、版本，退出释放本服务端口。
#[test]
fn shares_disk_reopen_restores_configuration_and_listener() {
    let f = Fixture::new();
    let mut share = f.draft();
    let address = share.listen;
    f.execute(ShareCommand::Save(share.clone()));
    share.name = "edited".into();
    let saved = f.execute(ShareCommand::Save(share));
    f.service.shutdown();
    release([address].into_iter());
    let snapshots = Arc::new(SnapshotService::new(
        JsonStateStore::new(f.root.join("state.json")).unwrap(),
        StateAccessGate::default(),
    ));
    let service = ShareService::new(snapshots, f.runtime.handle().clone());
    let restored = service.restore().unwrap();
    assert_eq!(restored, saved);
    assert_eq!(
        f.get(&restored.app_config.subscription_shares[0].url())
            .unwrap()
            .0,
        200
    );
    service.shutdown();
    release([address].into_iter());
}
/// 保护 bind、CAS 与原子写失败不撤销旧链接、不改变磁盘旧配置。
#[test]
fn shares_bind_cas_and_write_failure_preserve_previous_configuration() {
    let f = Fixture::new();
    let mut share = f.draft();
    let saved = f.execute(ShareCommand::Save(share.clone()));
    let occupied = TcpListener::bind("127.0.0.1:0").unwrap();
    share.listen = occupied.local_addr().unwrap();
    share.host = share.listen.to_string();
    assert_eq!(
        f.service
            .execute(saved.config_version(), ShareCommand::Save(share)),
        Err(ShareError::Bind)
    );
    let old = &saved.app_config.subscription_shares[0];
    std::fs::create_dir(f.root.join("state.tmp")).unwrap();
    assert_eq!(
        f.service.execute(
            saved.config_version(),
            ShareCommand::Regenerate(old.id.clone())
        ),
        Err(ShareError::Storage)
    );
    std::fs::remove_dir(f.root.join("state.tmp")).unwrap();
    assert_eq!(f.snapshots.snapshot().unwrap(), saved);
    assert_eq!(f.get(&old.url()).unwrap().0, 200);
    f.execute(ShareCommand::Regenerate(old.id.clone()));
    assert_eq!(
        f.service
            .execute(saved.config_version(), ShareCommand::Delete(old.id.clone())),
        Err(ShareError::Storage)
    );
}
/// 保护同端口多分享撤销相互独立，HTTP 不提供管理入口。
#[test]
fn shares_same_listener_read_only_and_independent_revocation() {
    let f = Fixture::new();
    let one = f.draft();
    let mut two = ShareService::draft().unwrap();
    two.name = "second".into();
    two.listen = one.listen;
    two.host = one.host.clone();
    two.subscription_ids = one.subscription_ids.clone();
    f.execute(ShareCommand::Save(one.clone()));
    f.execute(ShareCommand::Save(two.clone()));
    let status = f.runtime.block_on(async {
        reqwest::Client::new()
            .post(one.url())
            .send()
            .await
            .unwrap()
            .status()
    });
    assert_eq!(status, 405);
    f.execute(ShareCommand::Delete(one.id.clone()));
    assert_eq!(f.get(&one.url()).unwrap().0, 404);
    assert_eq!(f.get(&two.url()).unwrap().0, 200);
}

/// 保护删除订阅后的撤销入口仍然可用；不让失效内容阻止停用分享。
#[test]
fn shares_revocation_survives_removed_subscription() {
    let f = Fixture::new();
    let share = f.draft();
    f.execute(ShareCommand::Save(share.clone()));
    let mut state = f.snapshots.snapshot().unwrap();
    state.subscriptions.clear();
    state.providers.clear();
    state.nodes.clear();
    f.snapshots.replace(state.version(), state).unwrap();
    assert_eq!(f.get(&share.url()).unwrap().0, 503);
    f.execute(ShareCommand::Enabled(share.id.clone(), false));
    f.execute(ShareCommand::Delete(share.id));
}
/// 保护旧 v9 快照缺失分享字段时仍可读取；HTTP-only 与地址语义不被 URL 绕过。
#[test]
fn shares_legacy_schema_and_address_validation() {
    let f = Fixture::new();
    let path = f.root.join("state.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["app_config"]
        .as_object_mut()
        .unwrap()
        .remove("subscription_shares");
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        f.snapshots
            .snapshot()
            .unwrap()
            .app_config
            .subscription_shares
            .is_empty()
    );
    let mut share = f.draft();
    share.protocol = "https".into();
    assert!(share.validate().is_err());
    share.protocol = "http".into();
    share.host = "user:secret@example.invalid".into();
    assert!(share.validate().is_err());
    share.host = format!("0.0.0.0:{}", share.listen.port());
    assert!(share.validate().is_err());
    // 展示地址只能是 authority，否则 URL 拼接会生成无法路由的双斜线。
    share.host = format!("127.0.0.1:{}/", share.listen.port());
    assert!(share.validate().is_err());
    share.host = format!("share.example.invalid:{}", share.listen.port());
    assert!(share.validate().is_ok());
    assert!(!format!("{share:?}").contains(&share.token));
}

/// 保护地址与订阅集合编辑原子切换，旧端口释放且无旧订阅内容混入。
#[test]
fn shares_edit_moves_listener_and_selected_content() {
    let f = Fixture::new();
    let mut share = f.draft();
    let old = share.listen;
    let first = f.execute(ShareCommand::Save(share.clone()));
    let new = f.draft().listen;
    share.listen = new;
    share.host = new.to_string();
    share.subscription_ids = vec![SubscriptionId("private".into())];
    share.name = "changed".into();
    let saved = f.execute(ShareCommand::Save(share.clone()));
    assert_eq!(saved.config_revision, first.config_revision + 1);
    release([old].into_iter());
    let response = f.get(&share.url()).unwrap();
    assert_eq!(response.0, 200);
    assert!(response.1.contains("private-credential"));
    assert!(!response.1.contains("selected-credential"));
}

/// 保护真实 HTTP 服务主动关闭连接后，TIME_WAIT 不阻止同端口恢复。
#[test]
fn shares_immediate_restart_after_http_connection_close() {
    use std::io::{Read, Write};
    let f = Fixture::new();
    let share = f.draft();
    f.execute(ShareCommand::Save(share.clone()));
    let mut stream = std::net::TcpStream::connect(share.listen).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    write!(
        stream,
        "GET /sub/{} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
        share.token
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    drop(stream);
    f.service.shutdown();
    let reopened = ShareService::new(f.snapshots.clone(), f.runtime.handle().clone());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        match reopened.restore() {
            Ok(_) => break,
            Err(ShareError::Bind) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            other => panic!("listener restore failed: {other:?}"),
        }
    }
    assert_eq!(f.get(&share.url()).unwrap().0, 200);
    reopened.shutdown();
}
