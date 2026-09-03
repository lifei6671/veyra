use std::collections::HashSet;
use std::fmt;

use crate::domain::{
    NodeCredentials, NodeId, ProviderId, ProxyNode, ProxyProtocol, TlsOptions, Transport,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyNodeDraft {
    pub name: String,
    pub protocol: ProxyProtocol,
    pub server: String,
    pub port: u16,
    pub credentials: NodeCredentials,
    pub transport: Option<Transport>,
    pub tls: Option<TlsOptions>,
}

pub fn normalize_nodes(
    provider_id: ProviderId,
    drafts: Vec<ProxyNodeDraft>,
) -> Result<Vec<ProxyNode>, NormalizationError> {
    let mut ids = HashSet::new();
    let mut nodes = Vec::with_capacity(drafts.len());
    for draft in drafts {
        if draft.name.trim().is_empty() || draft.server.trim().is_empty() || draft.port == 0 {
            return Err(NormalizationError::InvalidNode);
        }
        let id = NodeId(format!("node-{:016x}", stable_hash(&provider_id, &draft)));
        if !ids.insert(id.clone()) {
            return Err(NormalizationError::DuplicateNodeIdentity);
        }
        nodes.push(ProxyNode {
            id,
            provider_id: provider_id.clone(),
            name: draft.name,
            protocol: draft.protocol,
            server: draft.server,
            port: draft.port,
            credentials: draft.credentials,
            transport: draft.transport,
            tls: draft.tls,
        });
    }
    Ok(nodes)
}

fn stable_hash(provider_id: &ProviderId, draft: &ProxyNodeDraft) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in provider_id
        .0
        .bytes()
        .chain(protocol_tag(draft.protocol).bytes())
        .chain(draft.name.bytes())
        .chain(draft.server.bytes())
        .chain(draft.port.to_be_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn protocol_tag(protocol: ProxyProtocol) -> &'static str {
    match protocol {
        ProxyProtocol::Shadowsocks => "shadowsocks",
        ProxyProtocol::Vmess => "vmess",
        ProxyProtocol::Vless => "vless",
        ProxyProtocol::Trojan => "trojan",
        ProxyProtocol::Hysteria2 => "hysteria2",
        ProxyProtocol::Tuic => "tuic",
        ProxyProtocol::Socks5 => "socks5",
        ProxyProtocol::Http => "http",
        ProxyProtocol::Https => "https",
        ProxyProtocol::AnyTls => "anytls",
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizationError {
    InvalidNode,
    DuplicateNodeIdentity,
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidNode => "subscription contains an invalid node",
            Self::DuplicateNodeIdentity => "subscription contains duplicate node identities",
        })
    }
}

impl std::error::Error for NormalizationError {}
