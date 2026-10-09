//! sing-box v1.14.0 封闭入站编译，产物由正式 Runtime owner 合入现有计划。
//! TLS 使用同一快照中的内联 PEM，不生成临时证书或外部路径/第二套资源事实。
use super::SingBoxCompiler;
use crate::domain::{
    SharedInboundValidation, SharedProtocol, SharedServer, SharedShadowsocksMethod, SharedTls,
    validate_shared_servers,
};
use serde::Serialize;
use std::net::IpAddr;

#[derive(Clone, Serialize)]
pub struct CompiledSharedInbounds(Vec<CompiledSharedInbound>);
impl std::fmt::Debug for CompiledSharedInbounds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("CompiledSharedInbounds([redacted])")
    }
}
impl CompiledSharedInbounds {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
}
#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum CompiledSharedInbound {
    Shadowsocks {
        #[serde(flatten)]
        listener: Listener,
        method: SharedShadowsocksMethod,
        password: String,
    },
    Vless {
        #[serde(flatten)]
        listener: Listener,
        users: Vec<VlessUser>,
        #[serde(skip_serializing_if = "Option::is_none")]
        tls: Option<InboundTls>,
    },
    Tuic {
        #[serde(flatten)]
        listener: Listener,
        users: Vec<TuicUser>,
        tls: InboundTls,
    },
    Hysteria2 {
        #[serde(flatten)]
        listener: Listener,
        users: Vec<PasswordUser>,
        #[serde(skip_serializing_if = "Option::is_none")]
        obfs: Option<Salamander>,
        tls: InboundTls,
    },
    Mixed {
        #[serde(flatten)]
        listener: Listener,
        users: Vec<MixedUser>,
    },
}
#[derive(Clone, Serialize)]
struct Listener {
    tag: String,
    listen: IpAddr,
    listen_port: u16,
}
#[derive(Clone, Serialize)]
struct VlessUser {
    uuid: String,
}
#[derive(Clone, Serialize)]
struct TuicUser {
    uuid: String,
    password: String,
}
#[derive(Clone, Serialize)]
struct PasswordUser {
    password: String,
}
#[derive(Clone, Serialize)]
struct MixedUser {
    username: String,
    password: String,
}
#[derive(Clone, Serialize)]
struct Salamander {
    #[serde(rename = "type")]
    kind: &'static str,
    password: String,
}
#[derive(Clone, Serialize)]
struct InboundTls {
    enabled: bool,
    certificate: Vec<String>,
    key: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    alpn: Vec<&'static str>,
}
impl InboundTls {
    fn from_resource(tls: &SharedTls, quic: bool) -> Self {
        Self {
            enabled: true,
            certificate: vec![tls.certificate_pem.clone()],
            key: vec![tls.private_key_pem.clone()],
            alpn: if quic { vec!["h3"] } else { vec![] },
        }
    }
}
impl SingBoxCompiler {
    /// 与 compile_product 共用正式 Compiler；只产出受类型约束的 inbounds 数组。
    /// 不落盘运行配置、不持有 child、不维护 Applied/恢复资料。资源内联且不变。
    pub fn compile_shared_inbounds(
        &self,
        servers: &[SharedServer],
    ) -> Result<CompiledSharedInbounds, SharedInboundValidation> {
        validate_shared_servers(servers)?;
        let mut inbounds = Vec::new();
        for server in servers.iter().filter(|s| s.enabled) {
            let listener = Listener {
                tag: format!("shared-{}", server.id),
                listen: server.listen,
                listen_port: server.port,
            };
            let value = match &server.protocol {
                SharedProtocol::Shadowsocks { method, password } => {
                    CompiledSharedInbound::Shadowsocks {
                        listener,
                        method: *method,
                        password: password.clone(),
                    }
                }
                SharedProtocol::Vless { uuid, tls } => CompiledSharedInbound::Vless {
                    listener,
                    users: vec![VlessUser { uuid: uuid.clone() }],
                    tls: tls.as_ref().map(|t| InboundTls::from_resource(t, false)),
                },
                SharedProtocol::Tuic {
                    uuid,
                    password,
                    tls,
                } => CompiledSharedInbound::Tuic {
                    listener,
                    users: vec![TuicUser {
                        uuid: uuid.clone(),
                        password: password.clone(),
                    }],
                    tls: InboundTls::from_resource(tls, true),
                },
                SharedProtocol::Hysteria2 {
                    password,
                    obfs,
                    tls,
                } => CompiledSharedInbound::Hysteria2 {
                    listener,
                    users: vec![PasswordUser {
                        password: password.clone(),
                    }],
                    obfs: obfs.as_ref().map(|p| Salamander {
                        kind: "salamander",
                        password: p.clone(),
                    }),
                    tls: InboundTls::from_resource(tls, true),
                },
                SharedProtocol::Mixed { username, password } => CompiledSharedInbound::Mixed {
                    listener,
                    users: if username.is_empty() {
                        vec![]
                    } else {
                        vec![MixedUser {
                            username: username.clone(),
                            password: password.clone(),
                        }]
                    },
                },
            };
            inbounds.push(value);
        }
        Ok(CompiledSharedInbounds(inbounds))
    }
}
