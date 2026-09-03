use std::fmt;

use serde_json::{json, Value};

use crate::domain::{NodeCredentials, RouteTarget, RuntimeIntent, SelectionPolicy, TrafficMatcher};

pub trait ConfigCompiler {
    fn compile(&self, intent: &RuntimeIntent) -> Result<GeneratedConfig, CompileError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedConfig {
    bytes: Vec<u8>,
}

impl GeneratedConfig {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Default)]
pub struct SingBoxCompiler;

impl ConfigCompiler for SingBoxCompiler {
    fn compile(&self, intent: &RuntimeIntent) -> Result<GeneratedConfig, CompileError> {
        let mut outbounds = intent
            .nodes
            .iter()
            .map(node_outbound)
            .collect::<Result<Vec<_>, _>>()?;
        for pool in &intent.pools {
            if pool.members.is_empty() {
                return Err(CompileError::EmptyPoolMembership);
            }
            let members = pool
                .members
                .iter()
                .map(|id| Value::String(node_tag(&id.0)))
                .collect::<Vec<_>>();
            let outbound = match &pool.selection {
                SelectionPolicy::Manual { .. } => json!({
                    "tag": pool_tag(&pool.id.0),
                    "type": "selector",
                    "outbounds": members,
                }),
                SelectionPolicy::UrlTest {
                    probe_url,
                    interval_secs,
                    tolerance_ms,
                } => json!({
                    "tag": pool_tag(&pool.id.0),
                    "type": "urltest",
                    "outbounds": members,
                    "url": probe_url,
                    "interval": format!("{interval_secs}s"),
                    "tolerance": tolerance_ms,
                }),
            };
            outbounds.push(outbound);
        }
        outbounds.push(json!({ "tag": "direct", "type": "direct" }));
        outbounds.push(json!({ "tag": "block", "type": "block" }));

        let rules = intent
            .routes
            .iter()
            .map(route_rule)
            .collect::<Result<Vec<_>, _>>()?;
        let document = json!({ "outbounds": outbounds, "route": { "rules": rules } });
        Ok(GeneratedConfig {
            bytes: serde_json::to_vec(&document).map_err(|_| CompileError::SerializationFailed)?,
        })
    }
}

fn node_outbound(node: &crate::domain::ProxyNode) -> Result<Value, CompileError> {
    let mut output = json!({
        "tag": node_tag(&node.id.0),
        "type": protocol_name(node.protocol),
        "server": node.server,
        "server_port": node.port,
    });
    let object = output.as_object_mut().expect("json object");
    match &node.credentials {
        NodeCredentials::None => {}
        NodeCredentials::Password {
            username,
            password,
            cipher,
        } => {
            object.insert("password".to_owned(), Value::String(password.clone()));
            if let Some(username) = username {
                object.insert("username".to_owned(), Value::String(username.clone()));
            }
            if let Some(cipher) = cipher {
                object.insert("method".to_owned(), Value::String(cipher.clone()));
            }
        }
        NodeCredentials::Uuid { uuid, flow } => {
            object.insert("uuid".to_owned(), Value::String(uuid.clone()));
            if let Some(flow) = flow {
                object.insert("flow".to_owned(), Value::String(flow.clone()));
            }
        }
    }
    Ok(output)
}

fn route_rule(route: &crate::domain::RoutePolicy) -> Result<Value, CompileError> {
    let (field, values) = match &route.matcher {
        TrafficMatcher::Domain(values) => ("domain", values.clone()),
        TrafficMatcher::DomainSuffix(values) => ("domain_suffix", values.clone()),
        TrafficMatcher::Application(values) => ("process_name", values.clone()),
        TrafficMatcher::IpCidr(values) => ("ip_cidr", values.clone()),
        TrafficMatcher::Port(values) => ("port", values.iter().map(ToString::to_string).collect()),
        TrafficMatcher::Protocol(values) => (
            "network",
            values
                .iter()
                .map(|value| format!("{value:?}").to_ascii_lowercase())
                .collect(),
        ),
    };
    let outbound = match &route.target {
        RouteTarget::Pool(id) => pool_tag(&id.0),
        RouteTarget::Direct => "direct".to_owned(),
        RouteTarget::Block => "block".to_owned(),
    };
    Ok(json!({ (field): values, "outbound": outbound }))
}

fn node_tag(id: &str) -> String {
    format!("node-{id}")
}
fn pool_tag(id: &str) -> String {
    format!("pool-{id}")
}

fn protocol_name(protocol: crate::domain::ProxyProtocol) -> &'static str {
    match protocol {
        crate::domain::ProxyProtocol::Shadowsocks => "shadowsocks",
        crate::domain::ProxyProtocol::Vmess => "vmess",
        crate::domain::ProxyProtocol::Vless => "vless",
        crate::domain::ProxyProtocol::Trojan => "trojan",
        crate::domain::ProxyProtocol::Hysteria2 => "hysteria2",
        crate::domain::ProxyProtocol::Tuic => "tuic",
        crate::domain::ProxyProtocol::Socks5 => "socks",
        crate::domain::ProxyProtocol::Http => "http",
        crate::domain::ProxyProtocol::Https => "http",
        crate::domain::ProxyProtocol::AnyTls => "anytls",
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompileError {
    EmptyPoolMembership,
    SerializationFailed,
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyPoolMembership => "enabled pool resolves to no nodes",
            Self::SerializationFailed => "generated configuration could not be serialized",
        })
    }
}

impl std::error::Error for CompileError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        AppState, NodeCredentials, NodeFilter, NodeId, NodePool, PoolId, PoolKind, PoolSource,
        Provider, ProviderId, ProxyNode, RoutePolicy, RoutePolicyId, Subscription, SubscriptionId,
    };

    fn intent() -> RuntimeIntent {
        let state = AppState {
            schema_version: crate::domain::CURRENT_SCHEMA_VERSION,
            subscriptions: vec![Subscription {
                id: SubscriptionId("subscription".to_owned()),
                name: "Subscription".to_owned(),
            }],
            providers: vec![Provider {
                id: ProviderId("provider".to_owned()),
                subscription_id: SubscriptionId("subscription".to_owned()),
                name: "Provider".to_owned(),
            }],
            nodes: vec![ProxyNode {
                id: NodeId("node".to_owned()),
                provider_id: ProviderId("provider".to_owned()),
                name: "Node".to_owned(),
                protocol: crate::domain::ProxyProtocol::Vless,
                server: "example.invalid".to_owned(),
                port: 443,
                credentials: NodeCredentials::Uuid {
                    uuid: "fixture-secret".to_owned(),
                    flow: None,
                },
                transport: None,
                tls: None,
            }],
            pools: vec![
                NodePool {
                    id: PoolId("manual".to_owned()),
                    name: "Manual".to_owned(),
                    kind: PoolKind::Custom,
                    sources: vec![PoolSource {
                        provider_id: ProviderId("provider".to_owned()),
                        filter: NodeFilter::default(),
                    }],
                    selection: SelectionPolicy::Manual {
                        selected_node_id: Some(NodeId("node".to_owned())),
                    },
                    enabled: true,
                },
                NodePool {
                    id: PoolId("auto".to_owned()),
                    name: "Auto".to_owned(),
                    kind: PoolKind::Custom,
                    sources: vec![PoolSource {
                        provider_id: ProviderId("provider".to_owned()),
                        filter: NodeFilter::default(),
                    }],
                    selection: SelectionPolicy::UrlTest {
                        probe_url: "https://example.invalid/probe".to_owned(),
                        interval_secs: 60,
                        tolerance_ms: 50,
                    },
                    enabled: true,
                },
            ],
            routes: vec![
                RoutePolicy {
                    id: RoutePolicyId("pool-route".to_owned()),
                    name: "Pool route".to_owned(),
                    enabled: true,
                    priority: 0,
                    matcher: TrafficMatcher::DomainSuffix(vec!["example.com".to_owned()]),
                    target: RouteTarget::Pool(PoolId("manual".to_owned())),
                },
                RoutePolicy {
                    id: RoutePolicyId("direct-route".to_owned()),
                    name: "Direct route".to_owned(),
                    enabled: true,
                    priority: 1,
                    matcher: TrafficMatcher::Domain(vec!["direct.example".to_owned()]),
                    target: RouteTarget::Direct,
                },
                RoutePolicy {
                    id: RoutePolicyId("block-route".to_owned()),
                    name: "Block route".to_owned(),
                    enabled: true,
                    priority: 2,
                    matcher: TrafficMatcher::Domain(vec!["block.example".to_owned()]),
                    target: RouteTarget::Block,
                },
            ],
        };
        RuntimeIntent::from_state(&state).expect("valid runtime intent")
    }

    #[test]
    fn compiles_a_deterministic_typed_snapshot() {
        let compiler = SingBoxCompiler;
        let first = compiler.compile(&intent()).expect("compile first");
        let second = compiler.compile(&intent()).expect("compile second");
        let text = std::str::from_utf8(first.as_bytes()).expect("utf8 json");

        assert_eq!(first, second);
        assert!(text.contains("\"type\":\"selector\""));
        assert!(text.contains("\"type\":\"urltest\""));
        assert!(text.contains("\"domain_suffix\""));
        assert!(text.contains("\"outbound\":\"pool-manual\""));
        assert!(text.contains("\"outbound\":\"direct\""));
        assert!(text.contains("\"outbound\":\"block\""));
    }
}
