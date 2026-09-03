use std::fmt;

use serde_json::{Value, json};

use crate::domain::{
    NodeCredentials, ProxyNode, ProxyProtocol, RouteTarget, RuntimeIntent, SelectionPolicy,
    TlsOptions, TrafficMatcher, Transport,
};

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

    #[allow(dead_code)] // 仅供尚未接线的受管 sidecar 配置注入边界使用。
    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { bytes }
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
                SelectionPolicy::Manual { selected_node_id } => {
                    let mut outbound = json!({
                        "tag": pool_tag(&pool.id.0),
                        "type": "selector",
                        "outbounds": members,
                    });
                    if let Some(node_id) = selected_node_id {
                        outbound
                            .as_object_mut()
                            .expect("selector outbound is an object")
                            .insert("default".to_owned(), Value::String(node_tag(&node_id.0)));
                    }
                    outbound
                }
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

fn node_outbound(node: &ProxyNode) -> Result<Value, CompileError> {
    validate_node_for_compilation(node)?;
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
    if let Some(tls) = &node.tls {
        object.insert("tls".to_owned(), tls_config(tls));
    } else if node.protocol == ProxyProtocol::Https {
        object.insert("tls".to_owned(), json!({ "enabled": true }));
    }
    if let Some(transport) = &node.transport
        && let Some(transport) = transport_config(transport)
    {
        object.insert("transport".to_owned(), transport);
    }
    Ok(output)
}

fn validate_node_for_compilation(node: &ProxyNode) -> Result<(), CompileError> {
    let credentials_are_valid = match node.protocol {
        ProxyProtocol::Shadowsocks => matches!(
            node.credentials,
            NodeCredentials::Password {
                username: None,
                cipher: Some(_),
                ..
            }
        ),
        ProxyProtocol::Vmess | ProxyProtocol::Vless => {
            matches!(node.credentials, NodeCredentials::Uuid { .. })
        }
        ProxyProtocol::Trojan | ProxyProtocol::Hysteria2 | ProxyProtocol::AnyTls => matches!(
            node.credentials,
            NodeCredentials::Password {
                username: None,
                cipher: None,
                ..
            }
        ),
        ProxyProtocol::Socks5 | ProxyProtocol::Http | ProxyProtocol::Https => matches!(
            node.credentials,
            NodeCredentials::None | NodeCredentials::Password { cipher: None, .. }
        ),
        ProxyProtocol::Tuic => return Err(CompileError::UnsupportedNodeProtocol),
    };
    let has_reality_fields = node
        .tls
        .as_ref()
        .is_some_and(|tls| tls.reality_public_key.is_some() || tls.reality_short_id.is_some());
    let has_complete_reality = node.tls.as_ref().is_some_and(|tls| {
        tls.reality_public_key
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            && tls
                .reality_short_id
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    });
    if !credentials_are_valid
        || matches!(
            node.protocol,
            ProxyProtocol::Trojan | ProxyProtocol::Hysteria2 | ProxyProtocol::AnyTls
        ) && node.tls.is_none()
        || node.tls.is_some()
            && !matches!(
                node.protocol,
                ProxyProtocol::Vmess
                    | ProxyProtocol::Vless
                    | ProxyProtocol::Trojan
                    | ProxyProtocol::Hysteria2
                    | ProxyProtocol::Http
                    | ProxyProtocol::Https
                    | ProxyProtocol::AnyTls
            )
        || has_reality_fields && (node.protocol != ProxyProtocol::Vless || !has_complete_reality)
        || matches!(
            node.transport,
            Some(Transport::Websocket { .. } | Transport::Grpc { .. })
        ) && !matches!(
            node.protocol,
            ProxyProtocol::Vmess | ProxyProtocol::Vless | ProxyProtocol::Trojan
        )
    {
        return Err(CompileError::InvalidNodeConfiguration);
    }
    Ok(())
}

fn tls_config(options: &TlsOptions) -> Value {
    let mut tls = json!({
        "enabled": true,
        "insecure": options.allow_insecure,
    });
    let object = tls.as_object_mut().expect("tls configuration is an object");
    if let Some(server_name) = &options.server_name {
        object.insert("server_name".to_owned(), Value::String(server_name.clone()));
    }
    if let (Some(public_key), Some(short_id)) =
        (&options.reality_public_key, &options.reality_short_id)
    {
        let reality = json!({
            "enabled": true,
            "public_key": public_key,
            "short_id": short_id,
        });
        object.insert("reality".to_owned(), reality);
    }
    tls
}

fn transport_config(transport: &Transport) -> Option<Value> {
    match transport {
        Transport::Tcp => None,
        Transport::Websocket { path, host } => {
            let mut output = json!({ "type": "ws", "path": path });
            if let Some(host) = host {
                output
                    .as_object_mut()
                    .expect("WebSocket transport is an object")
                    .insert("headers".to_owned(), json!({ "Host": host }));
            }
            Some(output)
        }
        Transport::Grpc { service_name } => {
            Some(json!({ "type": "grpc", "service_name": service_name }))
        }
    }
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
    InvalidNodeConfiguration,
    UnsupportedNodeProtocol,
    SerializationFailed,
}

impl fmt::Display for CompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyPoolMembership => "enabled pool resolves to no nodes",
            Self::InvalidNodeConfiguration => {
                "node configuration cannot be compiled by the current typed model"
            }
            Self::UnsupportedNodeProtocol => {
                "node protocol cannot be compiled by the current typed model"
            }
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
        TlsOptions, Transport,
    };

    fn intent() -> RuntimeIntent {
        let mut state = AppState {
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
                transport: Some(Transport::Websocket {
                    path: "/ws".to_owned(),
                    host: Some("cdn.example.invalid".to_owned()),
                }),
                tls: Some(TlsOptions {
                    server_name: Some("example.invalid".to_owned()),
                    allow_insecure: false,
                    reality_public_key: Some("fixture-public-key".to_owned()),
                    reality_short_id: Some("fixture-short-id".to_owned()),
                }),
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
                        selected_node_id: Some(NodeId("node-b".to_owned())),
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
        state.nodes.push(ProxyNode {
            id: NodeId("node-b".to_owned()),
            name: "Other node".to_owned(),
            ..state.nodes[0].clone()
        });
        RuntimeIntent::from_state(&state).expect("valid runtime intent")
    }

    #[test]
    fn compiles_a_deterministic_typed_snapshot() {
        let compiler = SingBoxCompiler;
        let first = compiler.compile(&intent()).expect("compile first");
        let second = compiler.compile(&intent()).expect("compile second");
        let document: Value = serde_json::from_slice(first.as_bytes()).expect("parse snapshot");
        let outbounds = document["outbounds"].as_array().expect("outbounds array");
        let rules = document["route"]["rules"].as_array().expect("rules array");

        assert_eq!(first, second);
        assert!(outbounds.iter().any(|outbound| {
            outbound["tag"] == "pool-manual"
                && outbound["type"] == "selector"
                && outbound["default"] == "node-node-b"
        }));
        assert!(outbounds.iter().any(|outbound| {
            outbound["tag"] == "pool-auto"
                && outbound["type"] == "urltest"
                && outbound["url"] == "https://example.invalid/probe"
                && outbound["interval"] == "60s"
                && outbound["tolerance"] == 50
        }));
        let node = outbounds
            .iter()
            .find(|outbound| outbound["tag"] == "node-node")
            .expect("VLESS outbound");
        assert_eq!(node["tls"]["enabled"], true);
        assert_eq!(node["tls"]["server_name"], "example.invalid");
        assert_eq!(node["tls"]["reality"]["public_key"], "fixture-public-key");
        assert_eq!(node["tls"]["reality"]["short_id"], "fixture-short-id");
        assert_eq!(node["transport"]["type"], "ws");
        assert_eq!(node["transport"]["path"], "/ws");
        assert_eq!(node["transport"]["headers"]["Host"], "cdn.example.invalid");
        assert!(rules.iter().any(|rule| {
            rule["domain_suffix"] == json!(["example.com"]) && rule["outbound"] == "pool-manual"
        }));
        assert!(rules.iter().any(|rule| rule["outbound"] == "direct"));
        assert!(rules.iter().any(|rule| rule["outbound"] == "block"));
    }

    #[test]
    fn stable_tags_do_not_change_when_display_names_change() {
        let compiler = SingBoxCompiler;
        let original = intent();
        let mut renamed = original.clone();
        renamed.nodes[0].name = "Renamed node".to_owned();
        renamed.routes[0].name = "Renamed route".to_owned();

        assert_eq!(
            compiler.compile(&original).expect("compile original"),
            compiler.compile(&renamed).expect("compile renamed")
        );
    }

    #[test]
    fn compiles_grpc_transport() {
        let compiler = SingBoxCompiler;
        let mut intent = intent();
        intent.nodes[0].transport = Some(Transport::Grpc {
            service_name: "TunService".to_owned(),
        });

        let document: Value = serde_json::from_slice(
            compiler
                .compile(&intent)
                .expect("compile grpc transport")
                .as_bytes(),
        )
        .expect("parse snapshot");
        let node = document["outbounds"]
            .as_array()
            .expect("outbounds array")
            .iter()
            .find(|outbound| outbound["tag"] == "node-node")
            .expect("VLESS outbound");

        assert_eq!(node["transport"]["type"], "grpc");
        assert_eq!(node["transport"]["service_name"], "TunService");
    }

    #[test]
    fn compiles_https_outbound_with_tls_enabled() {
        let compiler = SingBoxCompiler;
        let mut intent = intent();
        intent.nodes[0].protocol = ProxyProtocol::Https;
        intent.nodes[0].credentials = NodeCredentials::Password {
            username: Some("fixture-user".to_owned()),
            password: "fixture-password".to_owned(),
            cipher: None,
        };
        intent.nodes[0].transport = None;
        intent.nodes[0].tls = None;

        let document: Value = serde_json::from_slice(
            compiler
                .compile(&intent)
                .expect("compile HTTPS outbound")
                .as_bytes(),
        )
        .expect("parse snapshot");
        let node = document["outbounds"]
            .as_array()
            .expect("outbounds array")
            .iter()
            .find(|outbound| outbound["tag"] == "node-node")
            .expect("HTTPS outbound");

        assert_eq!(node["type"], "http");
        assert_eq!(node["tls"]["enabled"], true);
    }

    #[test]
    fn rejects_a_protocol_the_typed_model_cannot_represent_without_credentials() {
        let compiler = SingBoxCompiler;
        let mut intent = intent();
        intent.nodes[0].protocol = ProxyProtocol::Tuic;

        let error = compiler
            .compile(&intent)
            .expect_err("TUIC needs both UUID and password");

        assert_eq!(error, CompileError::UnsupportedNodeProtocol);
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn rejects_tls_for_a_protocol_that_does_not_support_it() {
        let compiler = SingBoxCompiler;
        let mut intent = intent();
        intent.nodes[0].protocol = ProxyProtocol::Socks5;
        intent.nodes[0].credentials = NodeCredentials::None;
        intent.nodes[0].transport = None;

        let error = compiler
            .compile(&intent)
            .expect_err("SOCKS5 does not support TLS fields");

        assert_eq!(error, CompileError::InvalidNodeConfiguration);
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn rejects_incomplete_or_blank_reality_configuration() {
        let compiler = SingBoxCompiler;
        for public_key in [None, Some("".to_owned()), Some("  ".to_owned())] {
            let mut intent = intent();
            intent.nodes[0]
                .tls
                .as_mut()
                .expect("fixture TLS")
                .reality_public_key = public_key;

            let error = compiler
                .compile(&intent)
                .expect_err("Reality requires non-empty public key and short ID");

            assert_eq!(error, CompileError::InvalidNodeConfiguration);
            assert!(!error.to_string().contains("fixture-secret"));
        }
        for short_id in [None, Some("".to_owned()), Some("  ".to_owned())] {
            let mut intent = intent();
            intent.nodes[0]
                .tls
                .as_mut()
                .expect("fixture TLS")
                .reality_short_id = short_id;

            let error = compiler
                .compile(&intent)
                .expect_err("Reality requires non-empty public key and short ID");

            assert_eq!(error, CompileError::InvalidNodeConfiguration);
            assert!(!error.to_string().contains("fixture-secret"));
        }
    }
}
