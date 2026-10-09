//! UI 草稿不是持久化事实；所有命令通过 AppServices worker 与生产快照提交。
use veyra_core::{
    application::{
        shared_inbounds::{
            SharedServerCommand, preflight_shared_port,
            sharing::{generate_shared_certificate, random_secret, random_uuid},
        },
        state_service::SnapshotService,
    },
    domain::*,
};
#[derive(Clone)]
pub struct Draft {
    pub id: String,
    pub name: String,
    pub address: String,
    pub listen: String,
    pub port: String,
    pub protocol: usize,
    pub method: usize,
    pub password: String,
    pub username: String,
    pub uuid: String,
    pub obfs: String,
    pub use_tls: bool,
    pub tls: Option<SharedTls>,
    pub enabled: bool,
}
pub const PROTOCOLS: [&str; 5] = ["Shadowsocks", "VLESS", "TUIC", "Hysteria2", "SOCKS5 / HTTP"];
pub const METHODS: [&str; 4] = [
    "aes-256-gcm",
    "aes-128-gcm",
    "chacha20-ietf-poly1305",
    "2022-blake3-aes-256-gcm",
];
impl Draft {
    pub fn new() -> Result<Self, &'static str> {
        Ok(Self {
            id: random_uuid()?,
            name: String::new(),
            address: "127.0.0.1".into(),
            listen: "0.0.0.0".into(),
            port: "8388".into(),
            protocol: 0,
            method: 0,
            password: random_secret(0)?,
            username: String::new(),
            uuid: random_uuid()?,
            obfs: String::new(),
            use_tls: true,
            tls: None,
            enabled: true,
        })
    }
    pub fn from_server(s: &SharedServer) -> Self {
        let mut d = Self {
            id: s.id.clone(),
            name: s.name.clone(),
            address: s.address.clone(),
            listen: s.listen.to_string(),
            port: s.port.to_string(),
            protocol: 0,
            method: 0,
            password: String::new(),
            username: String::new(),
            uuid: String::new(),
            obfs: String::new(),
            use_tls: false,
            tls: None,
            enabled: s.enabled,
        };
        match &s.protocol {
            SharedProtocol::Shadowsocks { method, password } => {
                d.method = match method {
                    SharedShadowsocksMethod::Aes256Gcm => 0,
                    SharedShadowsocksMethod::Aes128Gcm => 1,
                    SharedShadowsocksMethod::Chacha20IetfPoly1305 => 2,
                    SharedShadowsocksMethod::Blake3Aes256Gcm2022 => 3,
                };
                d.password = password.clone();
            }
            SharedProtocol::Vless { uuid, tls } => {
                d.protocol = 1;
                d.uuid = uuid.clone();
                d.use_tls = tls.is_some();
                d.tls = tls.clone();
            }
            SharedProtocol::Tuic {
                uuid,
                password,
                tls,
            } => {
                d.protocol = 2;
                d.uuid = uuid.clone();
                d.password = password.clone();
                d.tls = Some(tls.clone());
            }
            SharedProtocol::Hysteria2 {
                password,
                obfs,
                tls,
            } => {
                d.protocol = 3;
                d.password = password.clone();
                d.obfs = obfs.clone().unwrap_or_default();
                d.tls = Some(tls.clone());
            }
            SharedProtocol::Mixed { username, password } => {
                d.protocol = 4;
                d.username = username.clone();
                d.password = password.clone();
            }
        }
        d
    }
    pub fn switch(&mut self, protocol: usize) -> Result<(), &'static str> {
        if self.protocol == protocol {
            return Ok(());
        }
        self.protocol = protocol;
        self.port = ["8388", "8443", "8444", "8445", "7080"][protocol].into();
        self.username.clear();
        self.obfs.clear();
        self.tls = None;
        self.use_tls = true;
        self.method = 0;
        self.password = if protocol == 4 || protocol == 1 {
            String::new()
        } else {
            random_secret(0)?
        };
        self.uuid = if protocol == 1 || protocol == 2 {
            random_uuid()?
        } else {
            String::new()
        };
        Ok(())
    }
    pub fn needs_tls(&self) -> bool {
        self.protocol == 2 || self.protocol == 3 || self.protocol == 1 && self.use_tls
    }
    pub fn server(&self) -> Result<SharedServer, &'static str> {
        let tls = || self.tls.clone().ok_or("证书尚未生成");
        let protocol = match self.protocol {
            0 => SharedProtocol::Shadowsocks {
                method: [
                    SharedShadowsocksMethod::Aes256Gcm,
                    SharedShadowsocksMethod::Aes128Gcm,
                    SharedShadowsocksMethod::Chacha20IetfPoly1305,
                    SharedShadowsocksMethod::Blake3Aes256Gcm2022,
                ][self.method],
                password: self.password.clone(),
            },
            1 => SharedProtocol::Vless {
                uuid: self.uuid.trim().into(),
                tls: if self.use_tls { Some(tls()?) } else { None },
            },
            2 => SharedProtocol::Tuic {
                uuid: self.uuid.trim().into(),
                password: self.password.clone(),
                tls: tls()?,
            },
            3 => SharedProtocol::Hysteria2 {
                password: self.password.clone(),
                obfs: (!self.obfs.is_empty()).then(|| self.obfs.clone()),
                tls: tls()?,
            },
            _ => SharedProtocol::Mixed {
                username: self.username.clone(),
                password: self.password.clone(),
            },
        };
        let server = SharedServer {
            id: self.id.clone(),
            name: self.name.trim().into(),
            enabled: self.enabled,
            address: self.address.trim().into(),
            listen: self.listen.parse().map_err(|_| "监听地址须为本机 IP")?,
            port: self.port.parse().map_err(|_| "端口要在 1 到 65535 之间")?,
            protocol,
        };
        server.validate().map_err(validation_key)?;
        Ok(server)
    }
}
fn validation_key(e: SharedInboundValidation) -> &'static str {
    match e {
        SharedInboundValidation::MixedCredentials => "用户名和密码要一起填，或者都留空",
        SharedInboundValidation::Certificate => "证书或私钥无效",
        SharedInboundValidation::Port => "端口要在 1 到 65535 之间",
        SharedInboundValidation::Uuid => "UUID 格式无效",
        SharedInboundValidation::Identity => "备注不能为空",
        SharedInboundValidation::Address => "连接地址无效",
        SharedInboundValidation::ShadowsocksKey => "密码不符合加密方式",
        _ => "输入无效，请检查标记的字段",
    }
}
#[derive(Clone)]
pub enum Command {
    Read,
    Save(Box<Draft>, bool),
    Edit(Box<SharedServerCommand>),
}
pub struct Completion {
    pub result: Result<AppState, &'static str>,
    pub snapshot: Option<AppState>,
}
pub fn execute(
    snapshots: &SnapshotService,
    expected: Option<ConfigVersion>,
    command: Command,
) -> Completion {
    let result = (|| {
        let current = snapshots.snapshot().map_err(|e| e.code().message())?;
        let core = match command {
            Command::Read => return Ok(current),
            Command::Edit(c) => {
                if let SharedServerCommand::SetEnabled { id, enabled: true } = c.as_ref()
                    && let Some(old) = current
                        .app_config
                        .shared_servers
                        .iter()
                        .find(|s| &s.id == id)
                {
                    let mut candidate = old.clone();
                    candidate.enabled = true;
                    if !preflight_shared_port(
                        &candidate,
                        &current.app_config.shared_servers,
                        &[],
                        Some(id),
                    )
                    .is_empty()
                    {
                        return Err("端口已被占用，请修改后重试");
                    }
                }
                *c
            }
            Command::Save(mut d, existing) => {
                if d.needs_tls() && d.tls.is_none() {
                    d.tls =
                        Some(generate_shared_certificate().map_err(|_| "证书生成失败，请重试")?);
                }
                let server = d.server()?;
                // 实际 socket 预检只作提示，不能冒充 Runtime Apply。
                if server.enabled
                    && !preflight_shared_port(
                        &server,
                        &current.app_config.shared_servers,
                        &[],
                        existing.then_some(server.id.as_str()),
                    )
                    .is_empty()
                {
                    return Err("端口已被占用，请修改后重试");
                }
                if existing {
                    SharedServerCommand::Update(server)
                } else {
                    SharedServerCommand::Create(server)
                }
            }
        };
        snapshots
            .edit_shared_server(expected.ok_or("请先读取配置")?, core)
            .map(|o| o.value)
            .map_err(|e| e.code().message())
    })();
    let snapshot = result.is_err().then(|| snapshots.snapshot().ok()).flatten();
    Completion { result, snapshot }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::AppServices;
    use std::{path::PathBuf, sync::Arc};
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("veyra-p505-{}", random_uuid().unwrap()));
            std::fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn services(&self) -> Arc<AppServices> {
            Arc::new(AppServices::new(self.0.clone()).unwrap().0)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn send(s: &AppServices, state: &AppState, c: Command) -> Completion {
        s.shared_network_command(Some(state.config_version()), c)
            .blocking_recv()
            .unwrap()
    }
    fn read(s: &AppServices) -> AppState {
        s.shared_network_command(None, Command::Read)
            .blocking_recv()
            .unwrap()
            .result
            .unwrap()
    }
    fn draft(protocol: usize) -> Draft {
        let mut d = Draft::new().unwrap();
        d.switch(protocol).unwrap();
        d.name = format!("测试{protocol}");
        d.listen = "127.0.0.1".into();
        let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        d.port = socket.local_addr().unwrap().port().to_string();
        d
    }
    /// 保护UI→实际AppServices worker→生产Store的CRUD/排序/启停、固定证书与重建回读。
    #[test]
    fn shared_network_worker_crud_reorder_disk_reload() {
        let fixture = Fixture::new();
        let services = fixture.services();
        let mut state = read(&services);
        for i in 0..5 {
            state = send(&services, &state, Command::Save(Box::new(draft(i)), false))
                .result
                .unwrap();
        }
        assert_eq!(state.app_config.shared_servers.len(), 5);
        let mut d = Draft::from_server(&state.app_config.shared_servers[2]);
        let tls = d.tls.clone();
        d.name = "修改".into();
        state = send(&services, &state, Command::Save(Box::new(d), true))
            .result
            .unwrap();
        assert_eq!(
            Draft::from_server(&state.app_config.shared_servers[2]).tls,
            tls
        );
        let id = state.app_config.shared_servers[0].id.clone();
        state = send(
            &services,
            &state,
            Command::Edit(Box::new(SharedServerCommand::SetEnabled {
                id: id.clone(),
                enabled: false,
            })),
        )
        .result
        .unwrap();
        let ids = state
            .app_config
            .shared_servers
            .iter()
            .rev()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>();
        state = send(
            &services,
            &state,
            Command::Edit(Box::new(SharedServerCommand::Reorder(ids))),
        )
        .result
        .unwrap();
        assert_eq!(state.app_config.shared_servers.last().unwrap().id, id);
        state = send(
            &services,
            &state,
            Command::Edit(Box::new(SharedServerCommand::Delete { id })),
        )
        .result
        .unwrap();
        drop(services);
        let reconstructed = fixture.services();
        assert_eq!(read(&reconstructed), state);
        assert_eq!(state.app_config.shared_servers.len(), 4);
    }
    /// 保护失败保存不覆盖旧字节、CAS不丢草稿，重读权威版本后原草稿可重试。
    #[test]
    fn shared_network_write_failure_cas_and_retry() {
        let fixture = Fixture::new();
        let services = fixture.services();
        let initial = read(&services);
        let mut state = send(
            &services,
            &initial,
            Command::Save(Box::new(draft(0)), false),
        )
        .result
        .unwrap();
        let path = fixture.0.join("state.json");
        let bytes = std::fs::read(&path).unwrap();
        let mut d = Draft::from_server(&state.app_config.shared_servers[0]);
        d.name = "未丢的草稿".into();
        std::fs::create_dir(fixture.0.join("state.tmp")).unwrap();
        let failed = send(&services, &state, Command::Save(Box::new(d.clone()), true));
        assert!(failed.result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert_eq!(failed.snapshot, Some(state.clone()));
        std::fs::remove_dir(fixture.0.join("state.tmp")).unwrap();
        let conflict = send(
            &services,
            &initial,
            Command::Save(Box::new(d.clone()), true),
        );
        assert_eq!(
            conflict.result.unwrap_err(),
            AppErrorCode::RevisionConflict.message()
        );
        state = send(
            &services,
            conflict.snapshot.as_ref().unwrap(),
            Command::Save(Box::new(d), true),
        )
        .result
        .unwrap();
        assert_eq!(state.app_config.shared_servers[0].name, "未丢的草稿");
        drop(services);
        assert_eq!(read(&fixture.services()), state);
    }
    /// 保护真实socket占用被预检拒绝且旧配置/启停不变；不是内核运行声明。
    #[test]
    fn shared_network_occupied_port_keeps_old() {
        let fixture = Fixture::new();
        let services = fixture.services();
        let before = read(&services);
        let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let mut d = draft(4);
        d.port = socket.local_addr().unwrap().port().to_string();
        let result = send(
            &services,
            &before,
            Command::Save(Box::new(d.clone()), false),
        );
        assert!(result.result.is_err());
        assert_eq!(read(&services), before);
        d.enabled = false;
        let disabled = send(&services, &before, Command::Save(Box::new(d), false))
            .result
            .unwrap();
        let id = disabled.app_config.shared_servers[0].id.clone();
        assert!(
            send(
                &services,
                &disabled,
                Command::Edit(Box::new(SharedServerCommand::SetEnabled {
                    id: id.clone(),
                    enabled: true
                }))
            )
            .result
            .is_err()
        );
        drop(socket);
        let state = send(
            &services,
            &disabled,
            Command::Edit(Box::new(SharedServerCommand::SetEnabled {
                id,
                enabled: true,
            })),
        )
        .result
        .unwrap();
        assert!(state.app_config.shared_servers[0].enabled);
    }
    /// 保护协议切换清除不适用字段，并始终保留独立listen；mixed严格成对。
    #[test]
    fn shared_network_protocol_switch_and_mixed_pair() {
        let mut d = draft(0);
        d.address = "2001:db8::1".into();
        d.listen = "127.0.0.1".into();
        d.obfs = "old".into();
        d.username = "old".into();
        d.switch(4).unwrap();
        assert!(
            d.username.is_empty()
                && d.password.is_empty()
                && d.uuid.is_empty()
                && d.obfs.is_empty()
                && d.tls.is_none()
        );
        d.username = "user".into();
        assert!(d.server().is_err());
        d.password = "pass".into();
        let server = d.server().unwrap();
        assert_eq!(server.listen.to_string(), "127.0.0.1");
        assert_eq!(server.address, "2001:db8::1");
        d.switch(0).unwrap();
        assert!(d.username.is_empty() && d.obfs.is_empty() && d.tls.is_none());
    }
    /// 保护只确认当前协议不会改写端口、凭据、证书或草稿。
    #[test]
    fn shared_network_same_protocol_keeps_credentials_and_resources() {
        let mut d = draft(2);
        d.tls = Some(generate_shared_certificate().unwrap());
        let before = d.server().unwrap();
        d.switch(2).unwrap();
        assert_eq!(d.server().unwrap(), before);
    }
    /// 保护新生成并持久化的资源被固定内核实际接纳；仅check，不申请Runtime所有权。
    #[test]
    #[ignore = "requires VEYRA_P505_SING_BOX; runs check on an owned copy of locked 1.14.0"]
    fn shared_network_generated_resources_locked_check() {
        use sha2::{Digest, Sha256};
        let source = std::env::var_os("VEYRA_P505_SING_BOX").expect("locked kernel path");
        let bytes = std::fs::read(source).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            "973388c3f720e918fc64dff7fd75dde14b31cc1aa6fc15855e2f00c5291dd4f4"
        );
        let fixture = Fixture::new();
        let binary = fixture.0.join("sing-box");
        std::fs::write(&binary, bytes).unwrap();
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
        let services = fixture.services();
        let mut state = read(&services);
        for protocol in 0..5 {
            state = send(
                &services,
                &state,
                Command::Save(Box::new(draft(protocol)), false),
            )
            .result
            .unwrap();
        }
        drop(services);
        let reconstructed = fixture.services();
        assert_eq!(read(&reconstructed), state);
        let inbounds = veyra_core::singbox::SingBoxCompiler
            .compile_shared_inbounds(&state.app_config.shared_servers)
            .unwrap();
        let config =
            serde_json::json!({"inbounds":inbounds,"outbounds":[{"type":"direct","tag":"direct"}]});
        let path = fixture.0.join("candidate.json");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        std::io::Write::write_all(
            &mut file,
            serde_json::to_string(&config).unwrap().as_bytes(),
        )
        .unwrap();
        drop(file);
        let output = std::process::Command::new(&binary)
            .args(["check", "-c"])
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "check rejected generated saved resources: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!(
            "locked 1.14.0 check PASS: five saved inbounds, reconstructed Store, same self-signed PEM"
        );
    }
    /// 保护排序完整ID集合；遗漏/重复/外来ID绝不写入。
    #[test]
    fn shared_network_invalid_reorder_retains_disk() {
        let fixture = Fixture::new();
        let services = fixture.services();
        let before = read(&services);
        let saved = send(&services, &before, Command::Save(Box::new(draft(0)), false))
            .result
            .unwrap();
        for ids in [
            vec![],
            vec!["missing".into()],
            vec![saved.app_config.shared_servers[0].id.clone(); 2],
        ] {
            assert!(
                send(
                    &services,
                    &saved,
                    Command::Edit(Box::new(SharedServerCommand::Reorder(ids)))
                )
                .result
                .is_err()
            );
            assert_eq!(read(&services), saved);
        }
    }
}
