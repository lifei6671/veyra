//! 共享代理入站的唯一业务事实，与订阅分发 HTTP 服务分离。
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
    path::Path,
};

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SharedServer {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    /// 分享给客户端的域名/IP，不作为本机 bind 地址。
    pub address: String,
    pub listen: IpAddr,
    pub port: u16,
    pub protocol: SharedProtocol,
}
impl std::fmt::Debug for SharedServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedServer([redacted])")
    }
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SharedProtocol {
    Shadowsocks {
        method: SharedShadowsocksMethod,
        password: String,
    },
    Vless {
        uuid: String,
        tls: Option<SharedTls>,
    },
    Tuic {
        uuid: String,
        password: String,
        tls: SharedTls,
    },
    Hysteria2 {
        password: String,
        obfs: Option<String>,
        tls: SharedTls,
    },
    Mixed {
        username: String,
        password: String,
    },
}
impl std::fmt::Debug for SharedProtocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SharedProtocol([redacted])")
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SharedShadowsocksMethod {
    #[serde(rename = "aes-128-gcm")]
    Aes128Gcm,
    #[serde(rename = "aes-256-gcm")]
    Aes256Gcm,
    #[serde(rename = "chacha20-ietf-poly1305")]
    Chacha20IetfPoly1305,
    #[serde(rename = "2022-blake3-aes-256-gcm")]
    Blake3Aes256Gcm2022,
}

/// 来源标签不是信任结论；LocalPem 也不意味着客户端已经信任该证书。
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedCertificateSource {
    LocalPem,
    SelfSigned,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SharedTls {
    pub source: SharedCertificateSource,
    /// 证书/私钥随 state.json 原子保存；原文件删除后重启仍可使用同一资源。
    pub certificate_pem: String,
    pub private_key_pem: String,
}
impl std::fmt::Debug for SharedTls {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedTls")
            .field("source", &self.source)
            .field("material", &"[redacted]")
            .finish()
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedInboundValidation {
    Identity,
    Address,
    Port,
    Uuid,
    Password,
    MixedCredentials,
    ShadowsocksKey,
    Certificate,
    DuplicateId,
    ListenerConflict,
}
impl std::fmt::Display for SharedInboundValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid shared inbound field: {self:?}")
    }
}
impl std::error::Error for SharedInboundValidation {}

impl SharedTls {
    pub fn read_local(
        certificate: &Path,
        key: &Path,
        source: SharedCertificateSource,
    ) -> Result<Self, SharedInboundValidation> {
        let value = Self {
            source,
            certificate_pem: std::fs::read_to_string(certificate)
                .map_err(|_| SharedInboundValidation::Certificate)?,
            private_key_pem: std::fs::read_to_string(key)
                .map_err(|_| SharedInboundValidation::Certificate)?,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), SharedInboundValidation> {
        let invalid = SharedInboundValidation::Certificate;
        let certificates = CertificateDer::pem_slice_iter(self.certificate_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid)?;
        if certificates.is_empty() {
            return Err(invalid);
        }
        let key =
            PrivateKeyDer::from_pem_slice(self.private_key_pem.as_bytes()).map_err(|_| invalid)?;
        // 显式 provider 避免修改进程全局 TLS 状态；with_single_cert 验证密钥与叶证书匹配。
        rustls::ServerConfig::builder_with_provider(std::sync::Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| invalid)?
        .with_no_client_auth()
        .with_single_cert(certificates, key)
        .map_err(|_| invalid)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SharedTransport {
    Tcp,
    Udp,
}
impl SharedServer {
    pub fn socket_addr(&self) -> SocketAddr {
        SocketAddr::new(self.listen, self.port)
    }
    pub fn transports(&self) -> &'static [SharedTransport] {
        use SharedTransport::*;
        match self.protocol {
            SharedProtocol::Shadowsocks { .. } => &[Tcp, Udp],
            SharedProtocol::Vless { .. } | SharedProtocol::Mixed { .. } => &[Tcp],
            SharedProtocol::Tuic { .. } | SharedProtocol::Hysteria2 { .. } => &[Udp],
        }
    }
    pub fn validate(&self) -> Result<(), SharedInboundValidation> {
        use SharedInboundValidation::*;
        if self.id.trim().is_empty() || self.name.trim().is_empty() {
            return Err(Identity);
        }
        if self.port == 0 {
            return Err(Port);
        }
        // 地址仅可为一个 host，不接受 URL、端口或路径；IPv6 在模型中保存裸地址。
        if self.address.parse::<IpAddr>().is_err() {
            if self.address.is_empty()
                || self
                    .address
                    .chars()
                    .any(|c| c.is_whitespace() || matches!(c, '/' | '\\' | ':' | '@' | '?' | '#'))
            {
                return Err(Address);
            }
            let url =
                reqwest::Url::parse(&format!("https://{}", self.address)).map_err(|_| Address)?;
            if url.host_str().is_none() {
                return Err(Address);
            }
        } else if self
            .address
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_unspecified())
        {
            return Err(Address);
        }
        let password_ok = |value: &str| {
            if value.trim().is_empty() {
                Err(Password)
            } else {
                Ok(())
            }
        };
        match &self.protocol {
            SharedProtocol::Shadowsocks { method, password } => {
                password_ok(password)?;
                if *method == SharedShadowsocksMethod::Blake3Aes256Gcm2022
                    && !valid_2022_key(password)
                {
                    return Err(ShadowsocksKey);
                }
            }
            SharedProtocol::Vless { uuid, tls } => {
                if !valid_uuid(uuid) {
                    return Err(Uuid);
                }
                if let Some(tls) = tls {
                    tls.validate()?;
                }
            }
            SharedProtocol::Tuic {
                uuid,
                password,
                tls,
            } => {
                if !valid_uuid(uuid) {
                    return Err(Uuid);
                }
                password_ok(password)?;
                tls.validate()?;
            }
            SharedProtocol::Hysteria2 {
                password,
                obfs,
                tls,
            } => {
                password_ok(password)?;
                if let Some(obfs) = obfs {
                    password_ok(obfs)?;
                }
                tls.validate()?;
            }
            SharedProtocol::Mixed { username, password } => {
                if username.is_empty() != password.is_empty()
                    || (!username.is_empty()
                        && (username.trim().is_empty() || password.trim().is_empty()))
                {
                    return Err(MixedCredentials);
                }
            }
        }
        Ok(())
    }
}
fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}
fn valid_2022_key(value: &str) -> bool {
    const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = value.as_bytes();
    // 固定方法需要恰好32字节的规范 Base64 key，末字符的 padding bits 必须为零。
    bytes.len() == 44
        && bytes[43] == b'='
        && bytes[..43].iter().all(|b| BASE64.contains(b))
        && BASE64
            .iter()
            .position(|b| *b == bytes[42])
            .is_some_and(|i| i & 3 == 0)
}
/// IPv6 wildcard 的双栈行为取决于 OS；预检保守报告可能冲突，最终以 bind 为准。
pub fn shared_sockets_overlap(a: SocketAddr, b: SocketAddr) -> bool {
    a.port() == b.port()
        && (a.ip() == b.ip()
            || (a.is_ipv4() == b.is_ipv4() && (a.ip().is_unspecified() || b.ip().is_unspecified()))
            || a.ip() == IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED)
            || b.ip() == IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED))
}
pub fn validate_shared_servers(servers: &[SharedServer]) -> Result<(), SharedInboundValidation> {
    let mut ids = HashSet::new();
    for (i, server) in servers.iter().enumerate() {
        server.validate()?;
        if !ids.insert(&server.id) {
            return Err(SharedInboundValidation::DuplicateId);
        }
        if server.enabled
            && servers[..i].iter().any(|other| {
                other.enabled
                    && shared_sockets_overlap(server.socket_addr(), other.socket_addr())
                    && server
                        .transports()
                        .iter()
                        .any(|p| other.transports().contains(p))
            })
        {
            return Err(SharedInboundValidation::ListenerConflict);
        }
    }
    Ok(())
}
