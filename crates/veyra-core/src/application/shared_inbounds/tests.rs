//! 保护 Veyra 的真实保存/CAS/失败保旧、证书资源及端口提示契约。
use super::*;
use crate::{
    application::{
        state_access::StateAccessGate,
        state_service::{ApplyEffect, SnapshotService},
    },
    domain::*,
    singbox::SingBoxCompiler,
    storage::JsonStateStore,
};
use std::{path::PathBuf, process::Command, sync::OnceLock};

struct Fixture {
    root: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let mut nonce = [0u8; 12];
        getrandom::fill(&mut nonce).unwrap();
        let root =
            std::env::temp_dir().join(format!("veyra-p504-{}-{:x?}", std::process::id(), nonce));
        std::fs::create_dir(&root).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self { root }
    }
    fn service(&self) -> SnapshotService {
        SnapshotService::new(
            JsonStateStore::new(self.root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn tls() -> SharedTls {
    static TLS: OnceLock<SharedTls> = OnceLock::new();
    TLS.get_or_init(|| {
        let f = Fixture::new();
        let cert = f.root.join("cert.pem");
        let key = f.root.join("key.pem");
        let output = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "ec",
                "-pkeyopt",
                "ec_paramgen_curve:P-256",
                "-nodes",
                "-days",
                "2",
                "-subj",
                "/CN=localhost",
                "-addext",
                "subjectAltName=DNS:localhost,IP:127.0.0.1",
                "-keyout",
            ])
            .arg(&key)
            .arg("-out")
            .arg(&cert)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "test certificate generation failed"
        );
        SharedTls::read_local(&cert, &key, SharedCertificateSource::SelfSigned).unwrap()
    })
    .clone()
}
fn sample(kind: usize) -> SharedServer {
    let protocol = match kind {
        0 => SharedProtocol::Shadowsocks {
            method: SharedShadowsocksMethod::Aes256Gcm,
            password: "test-only-ss-password".into(),
        },
        1 => SharedProtocol::Vless {
            uuid: "6c6fe1d0-42a8-4c24-a801-c6e1a867184d".into(),
            tls: Some(tls()),
        },
        2 => SharedProtocol::Tuic {
            uuid: "6c6fe1d0-42a8-4c24-a801-c6e1a867184d".into(),
            password: "test-only-tuic-password".into(),
            tls: tls(),
        },
        3 => SharedProtocol::Hysteria2 {
            password: "test-only-hy2-password".into(),
            obfs: Some("test-only-obfs".into()),
            tls: tls(),
        },
        _ => SharedProtocol::Mixed {
            username: "test-user".into(),
            password: "test-only-mixed-password".into(),
        },
    };
    SharedServer {
        id: format!("p504-{kind}"),
        name: format!("server-{kind}"),
        enabled: true,
        address: "shared.example.invalid".into(),
        listen: "127.0.0.1".parse().unwrap(),
        port: 24010 + kind as u16,
        protocol,
    }
}

#[test]
fn shared_inbounds_disk_crud_restart_preserves_fixed_resources_and_versions() {
    let f = Fixture::new();
    let service = f.service();
    let initial = service.snapshot().unwrap();
    let mut state = initial.clone();
    for i in 0..5 {
        let result = service
            .edit_shared_server(
                state.config_version(),
                SharedServerCommand::Create(sample(i)),
            )
            .unwrap();
        assert_eq!(result.effect, ApplyEffect::SavedOnly);
        state = result.value;
    }
    assert_eq!(state.config_revision, initial.config_revision + 5);
    assert_eq!(state.selection_revision, initial.selection_revision);
    drop(service);
    let service = f.service();
    assert_eq!(service.snapshot().unwrap(), state);
    for i in 0..5 {
        let mut changed = sample(i);
        changed.name = "changed".into();
        changed.port += 100;
        state = service
            .edit_shared_server(state.config_version(), SharedServerCommand::Update(changed))
            .unwrap()
            .value;
    }
    drop(service);
    let service = f.service();
    assert_eq!(service.snapshot().unwrap(), state);
    state = service
        .edit_shared_server(
            state.config_version(),
            SharedServerCommand::SetEnabled {
                id: "p504-2".into(),
                enabled: false,
            },
        )
        .unwrap()
        .value;
    let unchanged = service
        .edit_shared_server(
            state.config_version(),
            SharedServerCommand::SetEnabled {
                id: "p504-2".into(),
                enabled: false,
            },
        )
        .unwrap()
        .value;
    assert_eq!(unchanged.version(), state.version());
    drop(service);
    let service = f.service();
    assert_eq!(service.snapshot().unwrap(), state);
    let compiled = serde_json::to_value(
        SingBoxCompiler
            .compile_shared_inbounds(&state.app_config.shared_servers)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(compiled.as_array().unwrap().len(), 4);
    assert_eq!(compiled[1]["listen_port"], sample(1).port + 100);
    assert_eq!(compiled[1]["tls"]["key"][0], tls().private_key_pem);
    for i in 0..5 {
        state = service
            .edit_shared_server(
                state.config_version(),
                SharedServerCommand::Delete {
                    id: format!("p504-{i}"),
                },
            )
            .unwrap()
            .value;
    }
    drop(service);
    assert_eq!(f.service().snapshot().unwrap(), state);
    assert!(state.app_config.shared_servers.is_empty());
}

#[test]
fn shared_inbounds_write_and_validation_failures_keep_disk_bytes() {
    let f = Fixture::new();
    let service = f.service();
    let initial = service.snapshot().unwrap();
    let saved = service
        .edit_shared_server(
            initial.config_version(),
            SharedServerCommand::Create(sample(0)),
        )
        .unwrap()
        .value;
    let before = std::fs::read(f.root.join("state.json")).unwrap();
    let mut invalid = sample(0);
    invalid.port = 0;
    assert!(
        service
            .edit_shared_server(saved.config_version(), SharedServerCommand::Update(invalid))
            .is_err()
    );
    assert!(
        service
            .edit_shared_server(
                initial.config_version(),
                SharedServerCommand::Delete { id: sample(0).id }
            )
            .is_err()
    );
    assert!(
        service
            .edit_shared_server(
                saved.config_version(),
                SharedServerCommand::Create(sample(0))
            )
            .is_err()
    );
    assert!(
        service
            .edit_shared_server(
                saved.config_version(),
                SharedServerCommand::Delete {
                    id: "missing".into()
                }
            )
            .is_err()
    );
    // 真实生产原子写路径失败；不使用 Mock，也不依赖当前用户是否为 root。
    std::fs::create_dir(f.root.join("state.tmp")).unwrap();
    let mut changed = sample(0);
    changed.name = "must-not-persist".into();
    assert!(
        service
            .edit_shared_server(saved.config_version(), SharedServerCommand::Update(changed))
            .is_err()
    );
    assert!(
        service
            .edit_shared_server(
                saved.config_version(),
                SharedServerCommand::Delete { id: sample(0).id }
            )
            .is_err()
    );
    assert_eq!(std::fs::read(f.root.join("state.json")).unwrap(), before);
    drop(service);
    assert_eq!(f.service().snapshot().unwrap(), saved);
}

#[test]
fn shared_inbounds_old_v9_schema_without_servers_loads_and_saves() {
    let f = Fixture::new();
    let service = f.service();
    let initial = service.snapshot().unwrap();
    drop(service);
    let mut old = serde_json::from_slice::<serde_json::Value>(
        &std::fs::read(f.root.join("state.json")).unwrap(),
    )
    .unwrap();
    old["app_config"]
        .as_object_mut()
        .unwrap()
        .remove("shared_servers");
    std::fs::write(f.root.join("state.json"), serde_json::to_vec(&old).unwrap()).unwrap();
    let service = f.service();
    assert_eq!(service.snapshot().unwrap(), initial);
    let saved = service
        .edit_shared_server(
            initial.config_version(),
            SharedServerCommand::Create(sample(4)),
        )
        .unwrap()
        .value;
    drop(service);
    assert_eq!(f.service().snapshot().unwrap(), saved);
}

#[test]
fn shared_inbounds_credentials_and_protocol_fields_are_closed_and_redacted() {
    for i in 0..5 {
        let mut server = sample(i);
        assert!(server.validate().is_ok());
        server.protocol = match server.protocol {
            SharedProtocol::Shadowsocks { .. } => SharedProtocol::Shadowsocks {
                method: SharedShadowsocksMethod::Blake3Aes256Gcm2022,
                password: "short".into(),
            },
            SharedProtocol::Vless { tls, .. } => SharedProtocol::Vless {
                uuid: "bad-uuid".into(),
                tls,
            },
            SharedProtocol::Tuic { uuid, tls, .. } => SharedProtocol::Tuic {
                uuid,
                password: "".into(),
                tls,
            },
            SharedProtocol::Hysteria2 { obfs, tls, .. } => SharedProtocol::Hysteria2 {
                password: "".into(),
                obfs,
                tls,
            },
            SharedProtocol::Mixed { username, .. } => SharedProtocol::Mixed {
                username,
                password: "".into(),
            },
        };
        assert!(SingBoxCompiler.compile_shared_inbounds(&[server]).is_err());
        let mut raw = serde_json::to_value(sample(i)).unwrap();
        raw["protocol"]["arbitrary_field"] = true.into();
        assert!(serde_json::from_value::<SharedServer>(raw).is_err());
        let debug = format!("{:?}", sample(i));
        assert!(!debug.contains("password"));
    }
    let mut ss = sample(0);
    ss.protocol = SharedProtocol::Shadowsocks {
        method: SharedShadowsocksMethod::Blake3Aes256Gcm2022,
        password: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".into(),
    };
    assert!(ss.validate().is_ok());
    let mut raw = serde_json::to_value(&ss).unwrap();
    raw["protocol"]["password"] = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAB=".into();
    assert!(
        serde_json::from_value::<SharedServer>(raw)
            .unwrap()
            .validate()
            .is_err()
    );
    let mut mixed = sample(4);
    mixed.protocol = SharedProtocol::Mixed {
        username: "".into(),
        password: "".into(),
    };
    assert!(mixed.validate().is_ok());
}

#[test]
fn shared_inbounds_tls_import_survives_source_deletion_and_rejects_bad_pair() {
    let f = Fixture::new();
    let material = tls();
    let cert = f.root.join("cert.pem");
    let key = f.root.join("key.pem");
    std::fs::write(&cert, &material.certificate_pem).unwrap();
    std::fs::write(&key, &material.private_key_pem).unwrap();
    let imported = SharedTls::read_local(&cert, &key, SharedCertificateSource::LocalPem).unwrap();
    let service = f.service();
    let current = service.snapshot().unwrap();
    let mut server = sample(1);
    server.protocol = SharedProtocol::Vless {
        uuid: "6c6fe1d0-42a8-4c24-a801-c6e1a867184d".into(),
        tls: Some(imported.clone()),
    };
    let saved = service
        .edit_shared_server(
            current.config_version(),
            SharedServerCommand::Create(server),
        )
        .unwrap()
        .value;
    drop(service);
    std::fs::remove_file(&cert).unwrap();
    std::fs::remove_file(&key).unwrap();
    assert!(imported.validate().is_ok());
    assert_eq!(f.service().snapshot().unwrap(), saved);
    assert!(SharedTls::read_local(&cert, &key, SharedCertificateSource::LocalPem).is_err());
    let mut bad = imported.clone();
    bad.private_key_pem = "not a key".into();
    assert!(bad.validate().is_err());
    bad = imported.clone();
    bad.certificate_pem = "not a certificate".into();
    assert!(bad.validate().is_err());
    let other_key = f.root.join("other.pem");
    let result = Command::new("openssl")
        .args([
            "genpkey",
            "-algorithm",
            "EC",
            "-pkeyopt",
            "ec_paramgen_curve:P-256",
            "-out",
        ])
        .arg(&other_key)
        .output()
        .unwrap();
    assert!(result.status.success());
    bad = imported;
    bad.private_key_pem = std::fs::read_to_string(other_key).unwrap();
    assert!(bad.validate().is_err());
}

#[test]
fn shared_inbounds_port_preflight_distinguishes_tcp_udp_reserved_and_servers() {
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut ss = sample(0);
    ss.port = tcp.local_addr().unwrap().port();
    assert_eq!(
        preflight_shared_port(&ss, &[], &[], None),
        vec![SharedPortWarning {
            transport: SharedTransport::Tcp,
            issue: SharedPortIssue::Listening
        }]
    );
    let udp = UdpSocket::bind(ss.socket_addr()).unwrap();
    assert_eq!(preflight_shared_port(&ss, &[], &[], None).len(), 2);
    drop(tcp);
    drop(udp);
    assert!(preflight_shared_port(&ss, &[], &[], None).is_empty());
    let reserved = SharedPortReservation {
        address: ss.socket_addr(),
        transport: SharedTransport::Tcp,
    };
    assert_eq!(
        preflight_shared_port(&ss, &[], &[reserved], None)[0].issue,
        SharedPortIssue::Reserved
    );
    assert_eq!(
        preflight_shared_port(&ss, &[ss.clone()], &[], None)[0].issue,
        SharedPortIssue::Server
    );
    assert!(preflight_shared_port(&ss, &[ss.clone()], &[], Some(&ss.id)).is_empty());
    let mut mixed = sample(4);
    mixed.port = ss.port;
    assert!(validate_shared_servers(&[ss.clone(), mixed]).is_err());
    let mut tuic = sample(2);
    tuic.port = ss.port;
    let mut vless = sample(1);
    vless.port = ss.port;
    assert!(validate_shared_servers(&[tuic, vless]).is_ok());
}

#[test]
fn shared_inbounds_compiler_maps_auth_tls_obfs_and_excludes_share_address() {
    for i in 0..5 {
        let source = sample(i);
        let compiled = SingBoxCompiler
            .compile_shared_inbounds(std::slice::from_ref(&source))
            .unwrap();
        let value = serde_json::to_value(&compiled).unwrap();
        assert_eq!(value[0]["listen"], "127.0.0.1");
        assert_eq!(value[0]["listen_port"], source.port);
        assert!(
            !serde_json::to_string(&value)
                .unwrap()
                .contains("shared.example.invalid")
        );
        match source.protocol {
            SharedProtocol::Shadowsocks { password, .. } => {
                assert_eq!(value[0]["password"], password)
            }
            SharedProtocol::Vless { uuid, .. } => assert_eq!(value[0]["users"][0]["uuid"], uuid),
            SharedProtocol::Tuic {
                uuid,
                password,
                tls,
            } => {
                assert_eq!(value[0]["users"][0]["uuid"], uuid);
                assert_eq!(value[0]["users"][0]["password"], password);
                assert_eq!(value[0]["tls"]["key"][0], tls.private_key_pem);
            }
            SharedProtocol::Hysteria2 { password, obfs, .. } => {
                assert_eq!(value[0]["users"][0]["password"], password);
                assert_eq!(value[0]["obfs"]["type"], "salamander");
                assert_eq!(value[0]["obfs"]["password"], obfs.unwrap());
            }
            SharedProtocol::Mixed { username, password } => {
                assert_eq!(value[0]["users"][0]["username"], username);
                assert_eq!(value[0]["users"][0]["password"], password);
            }
        }
    }
}

#[test]
fn shared_inbounds_state_and_backup_are_private() {
    let f = Fixture::new();
    let service = f.service();
    let initial = service.snapshot().unwrap();
    service
        .edit_shared_server(
            initial.config_version(),
            SharedServerCommand::Create(sample(2)),
        )
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for name in ["state.json", "state.json.bak"] {
            assert_eq!(
                std::fs::metadata(f.root.join(name))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }
    let state = f.service().snapshot().unwrap();
    assert!(!format!("{state:?}").contains("BEGIN PRIVATE KEY"));
}

#[cfg(unix)]
#[path = "kernel_tests.rs"]
mod kernel_tests;
