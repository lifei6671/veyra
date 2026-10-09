//! 共享 URI 与自签资源：只从同一 SharedServer 编码，不产生另一份配置。
use crate::domain::{SharedCertificateSource, SharedProtocol, SharedServer, SharedTls};
use std::{
    fs, io,
    process::{Command, Stdio},
};

pub fn encode_component(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|&b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = ((chunk[0] as u32) << 16)
            | ((chunk.get(1).copied().unwrap_or(0) as u32) << 8)
            | chunk.get(2).copied().unwrap_or(0) as u32;
        for i in 0..4 {
            out.push(if i > chunk.len() {
                '='
            } else {
                ALPHABET[((n >> (18 - i * 6)) & 63) as usize] as char
            });
        }
    }
    out
}
/// 输入已经过领域校验；空分享地址只影响分享，不参与 listen/bind。
pub fn shared_server_uri(server: &SharedServer) -> String {
    let raw = server
        .address
        .trim()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if raw.is_empty() {
        return String::new();
    }
    let host = if raw.contains(':') {
        format!("[{raw}]")
    } else {
        raw.to_owned()
    };
    let endpoint = format!("{host}:{}", server.port);
    let tls_query = |tls: &SharedTls, insecure: &str| {
        let self_signed = tls.source == SharedCertificateSource::SelfSigned;
        let sni = encode_component(if self_signed { "open-box.local" } else { raw });
        format!("sni={sni}&{insecure}={}", u8::from(self_signed))
    };
    let link = match &server.protocol {
        SharedProtocol::Shadowsocks { method, password } => format!(
            "ss://{}@{endpoint}",
            base64(
                format!(
                    "{}:{password}",
                    serde_json::to_value(method)
                        .expect("closed method")
                        .as_str()
                        .expect("method string")
                )
                .as_bytes()
            )
        ),
        SharedProtocol::Vless { uuid, tls } => format!(
            "vless://{uuid}@{endpoint}?encryption=none&security={}&type=tcp",
            tls.as_ref()
                .map(|t| format!("tls&{}", tls_query(t, "allowInsecure")))
                .unwrap_or_else(|| "none".into())
        ),
        SharedProtocol::Tuic {
            uuid,
            password,
            tls,
        } => format!(
            "tuic://{}:{}@{endpoint}?congestion_control=bbr&alpn=h3&{}",
            encode_component(uuid),
            encode_component(password),
            tls_query(tls, "allow_insecure")
        ),
        SharedProtocol::Hysteria2 {
            password,
            obfs,
            tls,
        } => format!(
            "hysteria2://{}@{endpoint}/?{}{}",
            encode_component(password),
            tls_query(tls, "insecure"),
            obfs.as_ref()
                .map(|o| format!("&obfs=salamander&obfs-password={}", encode_component(o)))
                .unwrap_or_default()
        ),
        SharedProtocol::Mixed { username, password } => format!(
            "socks5://{}{endpoint}",
            if username.is_empty() {
                String::new()
            } else {
                format!(
                    "{}:{}@",
                    encode_component(username),
                    encode_component(password)
                )
            }
        ),
    };
    format!(
        "{link}#{}",
        encode_component(if server.name.is_empty() {
            &server.id
        } else {
            &server.name
        })
    )
}
/// macOS 自带 OpenSSL 生成 SAN=open-box.local 的证书。临时资源仅由本调用拥有，
/// 最终 PEM 回到既有快照，失败或成功都清理临时私钥；不创建证书目录事实。
#[cfg(unix)]
pub fn generate_shared_certificate() -> io::Result<SharedTls> {
    use std::os::unix::fs::DirBuilderExt;
    let mut random = [0; 16];
    getrandom::fill(&mut random).map_err(io::Error::other)?;
    let root = std::env::temp_dir().join(format!(
        "veyra-shared-cert-{}",
        random
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    ));
    fs::DirBuilder::new().mode(0o700).create(&root)?;
    struct Cleanup(std::path::PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _cleanup = Cleanup(root.clone());
    fs::write(
        root.join("request.cnf"),
        "[req]\ndistinguished_name=dn\nx509_extensions=ext\nprompt=no\n[dn]\nCN=open-box.local\n[ext]\nsubjectAltName=DNS:open-box.local\n",
    )?;
    let status = Command::new("/usr/bin/openssl")
        .current_dir(&root)
        .args([
            "req",
            "-x509",
            "-newkey",
            "rsa:2048",
            "-nodes",
            "-days",
            "3650",
            "-keyout",
            "key.pem",
            "-out",
            "cert.pem",
            "-config",
            "request.cnf",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if !status.success() {
        return Err(io::Error::other("certificate generation failed"));
    }
    SharedTls::read_local(
        &root.join("cert.pem"),
        &root.join("key.pem"),
        SharedCertificateSource::SelfSigned,
    )
    .map_err(|_| io::Error::other("certificate validation failed"))
}

pub fn random_secret(method: usize) -> Result<String, &'static str> {
    let mut bytes = vec![0; if method == 3 { 32 } else { 16 }];
    getrandom::fill(&mut bytes).map_err(|_| "随机生成失败")?;
    let value = base64(&bytes);
    Ok(if method == 3 {
        value
    } else {
        value
            .replace('+', "-")
            .replace('/', "_")
            .trim_end_matches('=')
            .to_owned()
    })
}
pub fn random_uuid() -> Result<String, &'static str> {
    let mut b = [0; 16];
    getrandom::fill(&mut b).map_err(|_| "随机生成失败")?;
    b[6] = (b[6] & 15) | 64;
    b[8] = (b[8] & 63) | 128;
    let h = b.iter().map(|v| format!("{v:02x}")).collect::<String>();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    ))
}

/// 与原版扫码提示一致：局域网/loopback/本机名不声称外网可达。
pub fn shared_address_is_local(address: &str) -> bool {
    let host = address.trim().trim_matches(['[', ']']).to_lowercase();
    match host.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(ip)) => {
            ip.is_private()
                || ip.is_loopback()
                || ip.is_link_local()
                || ip.is_unspecified()
                || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        Ok(std::net::IpAddr::V6(ip)) => ip
            .to_ipv4_mapped()
            .map(|v| shared_address_is_local(&v.to_string()))
            .unwrap_or(
                ip.is_loopback()
                    || ip.is_unspecified()
                    || ip.is_unique_local()
                    || ip.is_unicast_link_local(),
            ),
        Err(_) => {
            !host.is_empty()
                && (!host.contains('.')
                    || [
                        ".lan",
                        ".local",
                        ".home",
                        ".internal",
                        ".localdomain",
                        ".home.arpa",
                    ]
                    .iter()
                    .any(|suffix| host.ends_with(suffix)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::*, subscription::parse_subscription};
    /// 保护五协议分享：IPv6、中文名称与保留字符在现有导入器中还原，TLS与资源来源一致。
    #[test]
    fn shared_uri_five_protocol_roundtrip() {
        let tls = generate_shared_certificate().unwrap();
        let password = "密:码@/#? &%".to_owned();
        let uuid = "550e8400-e29b-41d4-a716-446655440000".to_owned();
        let protocols = [
            SharedProtocol::Shadowsocks {
                method: SharedShadowsocksMethod::Aes256Gcm,
                password: password.clone(),
            },
            SharedProtocol::Vless {
                uuid: uuid.clone(),
                tls: Some(tls.clone()),
            },
            SharedProtocol::Tuic {
                uuid,
                password: password.clone(),
                tls: tls.clone(),
            },
            SharedProtocol::Hysteria2 {
                password: password.clone(),
                obfs: Some(password.clone()),
                tls: tls.clone(),
            },
            SharedProtocol::Mixed {
                username: "用:户@".into(),
                password: password.clone(),
            },
        ];
        for (i, protocol) in protocols.into_iter().enumerate() {
            let server = SharedServer {
                id: format!("s{i}"),
                name: "家里 / 测试#1".into(),
                enabled: true,
                address: "2001:db8::1".into(),
                listen: "127.0.0.1".parse().unwrap(),
                port: 18400 + i as u16,
                protocol,
            };
            server.validate().unwrap();
            let uri = shared_server_uri(&server);
            assert!(
                uri.contains("@[2001:db8::1]:") || uri.contains("//[2001:db8::1]:"),
                "{uri}"
            );
            let parsed = parse_subscription(&uri).unwrap();
            assert_eq!(parsed.nodes.len(), 1, "{uri} => {:?}", parsed.skipped);
            let node = &parsed.nodes[0];
            assert_eq!(node.name, server.name);
            assert_eq!(node.server, server.address);
            assert_eq!(node.port, server.port);
            match &node.options {
                ProtocolOptions::Shadowsocks { password: p, .. }
                | ProtocolOptions::Tuic { password: p, .. }
                | ProtocolOptions::Hysteria2 { password: p, .. } => assert_eq!(p, &password),
                ProtocolOptions::Socks {
                    username: u,
                    password: p,
                    ..
                } => {
                    assert_eq!(u.as_deref(), Some("用:户@"));
                    assert_eq!(p.as_deref(), Some(password.as_str()));
                }
                _ => {}
            }
            if (1..=3).contains(&i) {
                let client_tls = node.tls.as_ref().unwrap();
                assert!(client_tls.allow_insecure);
                assert_eq!(client_tls.server_name.as_deref(), Some("open-box.local"));
            }
        }
    }
    /// 保护真实生成的证书可被生产TLS校验、Compiler消费，不依赖临时源文件。
    #[test]
    fn shared_self_signed_material_valid_and_distinct() {
        let a = generate_shared_certificate().unwrap();
        let b = generate_shared_certificate().unwrap();
        a.validate().unwrap();
        b.validate().unwrap();
        assert_ne!(a.private_key_pem, b.private_key_pem);
        assert_eq!(a.source, SharedCertificateSource::SelfSigned);
    }
}
