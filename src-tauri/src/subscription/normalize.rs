use std::collections::HashSet;
use std::fmt;

use crate::domain::{
    NodeId, ProtocolOptions, ProviderId, ProxyNode, ProxyProtocol, TlsOptions, Transport,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProxyNodeDraft {
    pub name: String,
    pub protocol: ProxyProtocol,
    pub server: String,
    pub port: u16,
    pub options: ProtocolOptions,
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
            options: draft.options,
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
        .chain(canonical_material(draft).bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn canonical_material(draft: &ProxyNodeDraft) -> String {
    serde_json::to_string(&(
        "veyra-node-v3",
        draft.protocol,
        &draft.server,
        draft.port,
        &draft.options,
        &draft.transport,
        &draft.tls,
    ))
    .expect("typed node identity material serializes")
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

#[cfg(test)]
mod tests {
    use super::*;

    fn draft(name: &str) -> ProxyNodeDraft {
        ProxyNodeDraft {
            name: name.to_owned(),
            protocol: ProxyProtocol::Vless,
            server: "example.invalid".to_owned(),
            port: 443,
            options: ProtocolOptions::Vless {
                uuid: "fixture-uuid".to_owned(),
                flow: None,
            },
            transport: Some(Transport::Tcp),
            tls: None,
        }
    }

    #[test]
    fn display_name_does_not_change_the_stable_node_identity() {
        let provider = ProviderId("provider".to_owned());
        let first = normalize_nodes(provider.clone(), vec![draft("First name")])
            .expect("first draft normalizes");
        let renamed = normalize_nodes(provider, vec![draft("Renamed node")])
            .expect("renamed draft normalizes");

        assert_eq!(first[0].id, renamed[0].id);
    }
}
