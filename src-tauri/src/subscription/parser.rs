use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde_json::Value;

use crate::domain::{NodeCredentials, ProxyProtocol, TlsOptions, Transport};

use super::ProxyNodeDraft;

const MAX_BASE64_DEPTH: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionFormat {
    Json,
    ClashYaml,
    UriList,
    Base64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseResult {
    pub format: SubscriptionFormat,
    pub nodes: Vec<ProxyNodeDraft>,
    pub skipped: Vec<SkippedNode>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SkippedNode {
    UnsupportedProtocol,
    InvalidNode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParseError {
    EmptyInput,
    UnsupportedInput,
    InvalidBase64,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyInput => "subscription input is empty",
            Self::UnsupportedInput => "subscription input is unsupported",
            Self::InvalidBase64 => "subscription input is not valid base64",
        })
    }
}

impl std::error::Error for ParseError {}

pub fn parse_subscription(body: &str) -> Result<ParseResult, ParseError> {
    parse_with_depth(body.trim(), 0)
}

fn parse_with_depth(body: &str, base64_depth: usize) -> Result<ParseResult, ParseError> {
    if body.is_empty() {
        return Err(ParseError::EmptyInput);
    }
    if body.starts_with('{') {
        return parse_json(body);
    }
    if looks_like_yaml(body) {
        if let Ok(result) = parse_clash_yaml(body) {
            return Ok(result);
        }
    }
    if let Some(result) = parse_uri_list(body) {
        return Ok(result);
    }
    if base64_depth < MAX_BASE64_DEPTH {
        let decoded = decode_base64(body)?;
        let mut result = parse_with_depth(&decoded, base64_depth + 1)?;
        result.format = SubscriptionFormat::Base64;
        return Ok(result);
    }
    Err(ParseError::UnsupportedInput)
}

fn parse_json(body: &str) -> Result<ParseResult, ParseError> {
    let document: Value = serde_json::from_str(body).map_err(|_| ParseError::UnsupportedInput)?;
    let candidates = document
        .get("proxies")
        .or_else(|| document.get("outbounds"))
        .and_then(Value::as_array)
        .ok_or(ParseError::UnsupportedInput)?;
    let (nodes, skipped) = parse_json_candidates(candidates);
    Ok(ParseResult {
        format: SubscriptionFormat::Json,
        nodes,
        skipped,
    })
}

fn parse_json_candidates(candidates: &[Value]) -> (Vec<ProxyNodeDraft>, Vec<SkippedNode>) {
    let mut nodes = Vec::new();
    let mut skipped = Vec::new();
    for candidate in candidates {
        match draft_from_fields(&JsonFields::from(candidate)) {
            Ok(draft) => nodes.push(draft),
            Err(reason) => skipped.push(reason),
        }
    }
    (nodes, skipped)
}

fn looks_like_yaml(body: &str) -> bool {
    body.lines()
        .any(|line| line.trim_start().starts_with("proxies:"))
}

fn parse_clash_yaml(body: &str) -> Result<ParseResult, ParseError> {
    let document: ClashDocument =
        serde_yaml_ng::from_str(body).map_err(|_| ParseError::UnsupportedInput)?;
    let mut nodes = Vec::new();
    let mut skipped = Vec::new();
    for proxy in document.proxies {
        match draft_from_fields(&proxy.into()) {
            Ok(draft) => nodes.push(draft),
            Err(reason) => skipped.push(reason),
        }
    }
    Ok(ParseResult {
        format: SubscriptionFormat::ClashYaml,
        nodes,
        skipped,
    })
}

fn parse_uri_list(body: &str) -> Option<ParseResult> {
    let lines = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect::<Vec<_>>();
    if lines.is_empty() || !lines.iter().any(|line| line.contains("://")) {
        return None;
    }

    let mut nodes = Vec::new();
    let mut skipped = Vec::new();
    for line in lines {
        match parse_uri(line) {
            Ok(draft) => nodes.push(draft),
            Err(reason) => skipped.push(reason),
        }
    }
    Some(ParseResult {
        format: SubscriptionFormat::UriList,
        nodes,
        skipped,
    })
}

fn parse_uri(uri: &str) -> Result<ProxyNodeDraft, SkippedNode> {
    let (scheme, remainder) = uri.split_once("://").ok_or(SkippedNode::InvalidNode)?;
    if scheme.eq_ignore_ascii_case("vmess") {
        let (payload, fragment) = remainder.split_once('#').unwrap_or((remainder, ""));
        let json = decode_base64(payload).map_err(|_| SkippedNode::InvalidNode)?;
        let mut value =
            serde_json::from_str::<Value>(&json).map_err(|_| SkippedNode::InvalidNode)?;
        let object = value.as_object_mut().ok_or(SkippedNode::InvalidNode)?;
        object.insert("type".to_owned(), Value::String("vmess".to_owned()));
        let mut draft = draft_from_fields(&JsonFields::from(&value))?;
        if !fragment.is_empty() {
            draft.name = percent_decode(fragment);
        }
        return Ok(draft);
    }
    let protocol = protocol_from_name(scheme).ok_or(SkippedNode::UnsupportedProtocol)?;
    let (before_fragment, fragment) = remainder.split_once('#').unwrap_or((remainder, ""));
    let (authority, query) = before_fragment
        .split_once('?')
        .unwrap_or((before_fragment, ""));
    let (credentials, host_port) = authority.rsplit_once('@').unwrap_or(("", authority));
    let (server, port) = split_host_port(host_port)?;
    let query = parse_query(query);
    let name = if fragment.is_empty() {
        format!("{scheme} {server}")
    } else {
        percent_decode(fragment)
    };
    let credentials = parse_uri_credentials(protocol, credentials, &query)?;
    let tls = query
        .get("security")
        .filter(|value| value.as_str() == "tls" || value.as_str() == "reality")
        .map(|_| TlsOptions {
            server_name: query.get("sni").cloned(),
            allow_insecure: query.get("allowInsecure").is_some_and(|value| value == "1"),
            reality_public_key: query.get("pbk").cloned(),
            reality_short_id: query.get("sid").cloned(),
        });
    let transport = match query.get("type").map(String::as_str) {
        Some("ws") => Some(Transport::Websocket {
            path: query.get("path").cloned().unwrap_or_else(|| "/".to_owned()),
            host: query.get("host").cloned(),
        }),
        Some("grpc") => Some(Transport::Grpc {
            service_name: query.get("serviceName").cloned().unwrap_or_default(),
        }),
        _ => Some(Transport::Tcp),
    };
    Ok(ProxyNodeDraft {
        name,
        protocol,
        server,
        port,
        credentials,
        transport,
        tls,
    })
}

fn parse_uri_credentials(
    protocol: ProxyProtocol,
    raw: &str,
    query: &BTreeMap<String, String>,
) -> Result<NodeCredentials, SkippedNode> {
    let raw = percent_decode(raw);
    match protocol {
        ProxyProtocol::Vmess | ProxyProtocol::Vless => {
            if raw.is_empty() {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(NodeCredentials::Uuid {
                uuid: raw,
                flow: query.get("flow").cloned(),
            })
        }
        ProxyProtocol::Shadowsocks => {
            let decoded = if raw.contains(':') {
                raw
            } else {
                decode_base64(&raw).map_err(|_| SkippedNode::InvalidNode)?
            };
            let (cipher, password) = decoded.split_once(':').ok_or(SkippedNode::InvalidNode)?;
            Ok(NodeCredentials::Password {
                username: None,
                password: password.to_owned(),
                cipher: Some(cipher.to_owned()),
            })
        }
        ProxyProtocol::Socks5 | ProxyProtocol::Http | ProxyProtocol::Https => {
            let (username, password) = raw.split_once(':').unwrap_or(("", ""));
            Ok(NodeCredentials::Password {
                username: (!username.is_empty()).then(|| username.to_owned()),
                password: password.to_owned(),
                cipher: None,
            })
        }
        _ => {
            if raw.is_empty() {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(NodeCredentials::Password {
                username: None,
                password: raw,
                cipher: None,
            })
        }
    }
}

fn split_host_port(input: &str) -> Result<(String, u16), SkippedNode> {
    let (host, port) = input.rsplit_once(':').ok_or(SkippedNode::InvalidNode)?;
    let port = port.parse().map_err(|_| SkippedNode::InvalidNode)?;
    if host.is_empty() || port == 0 {
        return Err(SkippedNode::InvalidNode);
    }
    Ok((host.trim_matches(['[', ']']).to_owned(), port))
}

fn protocol_from_name(value: &str) -> Option<ProxyProtocol> {
    Some(match value.to_ascii_lowercase().as_str() {
        "ss" => ProxyProtocol::Shadowsocks,
        "vmess" => ProxyProtocol::Vmess,
        "vless" => ProxyProtocol::Vless,
        "trojan" => ProxyProtocol::Trojan,
        "hysteria2" | "hy2" => ProxyProtocol::Hysteria2,
        "tuic" => ProxyProtocol::Tuic,
        "socks5" => ProxyProtocol::Socks5,
        "http" => ProxyProtocol::Http,
        "https" => ProxyProtocol::Https,
        "anytls" => ProxyProtocol::AnyTls,
        _ => return None,
    })
}

fn parse_query(query: &str) -> BTreeMap<String, String> {
    query
        .split('&')
        .filter_map(|entry| entry.split_once('='))
        .map(|(key, value)| (percent_decode(key), percent_decode(value)))
        .collect()
}

fn percent_decode(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    decoded.push(char::from(byte));
                    index += 3;
                    continue;
                }
            }
        }
        decoded.push(char::from(bytes[index]));
        index += 1;
    }
    decoded
}

fn decode_base64(input: &str) -> Result<String, ParseError> {
    let input = input.trim().replace(['\r', '\n'], "");
    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut buffer = 0_u32;
    let mut bits = 0_u8;
    for byte in input.bytes() {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => return Err(ParseError::InvalidBase64),
        };
        buffer = (buffer << 6) | u32::from(value);
        bits += 6;
        while bits >= 8 {
            bits -= 8;
            output.push(((buffer >> bits) & 0xff) as u8);
        }
    }
    String::from_utf8(output).map_err(|_| ParseError::InvalidBase64)
}

#[derive(Deserialize)]
struct ClashDocument {
    #[serde(default)]
    proxies: Vec<ClashProxy>,
}

#[derive(Deserialize)]
struct ClashProxy {
    name: Option<String>,
    #[serde(rename = "type")]
    protocol: Option<String>,
    server: Option<String>,
    port: Option<u16>,
    uuid: Option<String>,
    password: Option<String>,
    username: Option<String>,
    cipher: Option<String>,
    network: Option<String>,
    tls: Option<bool>,
    sni: Option<String>,
    #[serde(rename = "skip-cert-verify")]
    skip_cert_verify: Option<bool>,
    #[serde(rename = "ws-opts")]
    websocket: Option<WebsocketOptions>,
    #[serde(rename = "grpc-opts")]
    grpc: Option<GrpcOptions>,
}

#[derive(Deserialize)]
struct WebsocketOptions {
    path: Option<String>,
    headers: Option<BTreeMap<String, String>>,
}

#[derive(Deserialize)]
struct GrpcOptions {
    #[serde(rename = "grpc-service-name")]
    service_name: Option<String>,
}

struct JsonFields {
    name: Option<String>,
    protocol: Option<String>,
    server: Option<String>,
    port: Option<u16>,
    uuid: Option<String>,
    password: Option<String>,
    username: Option<String>,
    cipher: Option<String>,
    network: Option<String>,
    tls: Option<bool>,
    sni: Option<String>,
    skip_cert_verify: Option<bool>,
    websocket_path: Option<String>,
    websocket_host: Option<String>,
    grpc_service_name: Option<String>,
}

impl From<&Value> for JsonFields {
    fn from(value: &Value) -> Self {
        let get_string = |name| value.get(name).and_then(Value::as_str).map(str::to_owned);
        let get_port = || {
            value
                .get("port")
                .or_else(|| value.get("server_port"))
                .and_then(Value::as_u64)
                .and_then(|port| u16::try_from(port).ok())
                .or_else(|| {
                    value
                        .get("port")
                        .and_then(Value::as_str)
                        .and_then(|port| port.parse().ok())
                })
        };
        let transport = value.get("transport");
        Self {
            name: get_string("name")
                .or_else(|| get_string("tag"))
                .or_else(|| get_string("ps")),
            protocol: get_string("type"),
            server: get_string("server").or_else(|| get_string("add")),
            port: get_port(),
            uuid: get_string("uuid").or_else(|| get_string("id")),
            password: get_string("password"),
            username: get_string("username"),
            cipher: get_string("cipher").or_else(|| get_string("method")),
            network: get_string("network")
                .or_else(|| get_string("net"))
                .or_else(|| {
                    transport
                        .and_then(|item| item.get("type"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
            tls: value
                .get("tls")
                .and_then(Value::as_bool)
                .or_else(|| {
                    value
                        .get("tls")
                        .and_then(|item| item.get("enabled"))
                        .and_then(Value::as_bool)
                })
                .or_else(|| get_string("tls").map(|value| value == "tls")),
            sni: get_string("sni").or_else(|| {
                value
                    .get("tls")
                    .and_then(|item| item.get("server_name"))
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            }),
            skip_cert_verify: value.get("skip-cert-verify").and_then(Value::as_bool),
            websocket_path: value
                .get("ws-opts")
                .and_then(|item| item.get("path"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| get_string("path")),
            websocket_host: value
                .get("ws-opts")
                .and_then(|item| item.get("headers"))
                .and_then(|item| item.get("Host"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| get_string("host")),
            grpc_service_name: value
                .get("grpc-opts")
                .and_then(|item| item.get("grpc-service-name"))
                .and_then(Value::as_str)
                .map(str::to_owned),
        }
    }
}

impl From<ClashProxy> for JsonFields {
    fn from(value: ClashProxy) -> Self {
        let websocket_host = value
            .websocket
            .as_ref()
            .and_then(|options| options.headers.as_ref())
            .and_then(|headers| headers.get("Host"))
            .cloned();
        Self {
            name: value.name,
            protocol: value.protocol,
            server: value.server,
            port: value.port,
            uuid: value.uuid,
            password: value.password,
            username: value.username,
            cipher: value.cipher,
            network: value.network,
            tls: value.tls,
            sni: value.sni,
            skip_cert_verify: value.skip_cert_verify,
            websocket_path: value.websocket.and_then(|options| options.path),
            websocket_host,
            grpc_service_name: value.grpc.and_then(|options| options.service_name),
        }
    }
}

fn draft_from_fields(fields: &JsonFields) -> Result<ProxyNodeDraft, SkippedNode> {
    let protocol = fields
        .protocol
        .as_deref()
        .and_then(protocol_from_name)
        .ok_or(SkippedNode::UnsupportedProtocol)?;
    let name = fields
        .name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or(SkippedNode::InvalidNode)?;
    let server = fields
        .server
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or(SkippedNode::InvalidNode)?;
    let port = fields
        .port
        .filter(|port| *port > 0)
        .ok_or(SkippedNode::InvalidNode)?;
    let credentials = match protocol {
        ProxyProtocol::Vmess | ProxyProtocol::Vless => NodeCredentials::Uuid {
            uuid: fields
                .uuid
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            flow: None,
        },
        ProxyProtocol::Shadowsocks => NodeCredentials::Password {
            username: None,
            password: fields
                .password
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            cipher: fields.cipher.clone(),
        },
        ProxyProtocol::Socks5 | ProxyProtocol::Http | ProxyProtocol::Https => {
            NodeCredentials::Password {
                username: fields.username.clone(),
                password: fields.password.clone().unwrap_or_default(),
                cipher: fields.cipher.clone(),
            }
        }
        _ => NodeCredentials::Password {
            username: None,
            password: fields
                .password
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            cipher: None,
        },
    };
    let transport = match fields.network.as_deref() {
        Some("ws") => Some(Transport::Websocket {
            path: fields
                .websocket_path
                .clone()
                .unwrap_or_else(|| "/".to_owned()),
            host: fields.websocket_host.clone(),
        }),
        Some("grpc") => Some(Transport::Grpc {
            service_name: fields.grpc_service_name.clone().unwrap_or_default(),
        }),
        _ => Some(Transport::Tcp),
    };
    let tls = fields.tls.filter(|enabled| *enabled).map(|_| TlsOptions {
        server_name: fields.sni.clone(),
        allow_insecure: fields.skip_cert_verify.unwrap_or(false),
        reality_public_key: None,
        reality_short_id: None,
    });
    Ok(ProxyNodeDraft {
        name,
        protocol,
        server,
        port,
        credentials,
        transport,
        tls,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ProviderId;
    use crate::subscription::normalize_nodes;

    #[test]
    fn detects_json_and_extracts_supported_proxy_nodes() {
        let result = parse_subscription(include_str!(
            "../../tests/fixtures/subscriptions/proxies.json"
        ))
        .expect("parse json fixture");
        assert_eq!(result.format, SubscriptionFormat::Json);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.skipped, vec![SkippedNode::UnsupportedProtocol]);
    }

    #[test]
    fn extracts_supported_sing_box_outbounds() {
        let result = parse_subscription(include_str!(
            "../../tests/fixtures/subscriptions/singbox.json"
        ))
        .expect("parse sing-box fixture");
        assert_eq!(result.format, SubscriptionFormat::Json);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].server, "singbox.example.invalid");
        assert!(result.nodes[0].tls.is_some());
    }

    #[test]
    fn extracts_only_clash_yaml_proxies() {
        let result = parse_subscription(include_str!(
            "../../tests/fixtures/subscriptions/clash.yaml"
        ))
        .expect("parse yaml fixture");
        assert_eq!(result.format, SubscriptionFormat::ClashYaml);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].name, "Clash VMess");
    }

    #[test]
    fn parses_uri_lists_and_normalizes_provider_ownership() {
        let result =
            parse_subscription(include_str!("../../tests/fixtures/subscriptions/nodes.uri"))
                .expect("parse uri fixture");
        let nodes = normalize_nodes(ProviderId("provider-a".to_owned()), result.nodes)
            .expect("normalize nodes");
        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().all(|node| node.provider_id.0 == "provider-a"));
    }

    #[test]
    fn decodes_one_base64_wrapped_subscription() {
        let result = parse_subscription(include_str!(
            "../../tests/fixtures/subscriptions/nodes.base64"
        ))
        .expect("parse base64 fixture");
        assert_eq!(result.format, SubscriptionFormat::Base64);
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].name, "Base64 Node");
    }

    #[test]
    fn decodes_a_nested_base64_subscription() {
        let result = parse_subscription(include_str!(
            "../../tests/fixtures/subscriptions/nodes.nested-base64"
        ))
        .expect("parse nested base64 fixture");
        assert_eq!(result.format, SubscriptionFormat::Base64);
        assert_eq!(result.nodes.len(), 1);
    }

    #[test]
    fn parses_a_standard_vmess_uri_with_a_fragment_name() {
        let result =
            parse_subscription(include_str!("../../tests/fixtures/subscriptions/vmess.uri"))
                .expect("parse vmess fixture");
        assert_eq!(result.nodes.len(), 1);
        assert_eq!(result.nodes[0].name, "Fragment name");
        assert_eq!(result.nodes[0].server, "vmess.example.invalid");
    }

    #[test]
    fn reports_invalid_and_unsupported_entries_without_credentials() {
        let result = parse_subscription("vless://missing\nunknown://secret@example.invalid:1")
            .expect("uri list is recognized");
        assert_eq!(result.nodes.len(), 0);
        assert_eq!(
            result.skipped,
            vec![SkippedNode::InvalidNode, SkippedNode::UnsupportedProtocol]
        );
    }

    #[test]
    fn rejects_a_shadowsocks_node_without_a_password() {
        let result = parse_subscription(
            "proxies:\n  - name: missing password\n    type: ss\n    server: example.invalid\n    port: 443\n    cipher: aes-128-gcm",
        )
        .expect("parse yaml");
        assert!(result.nodes.is_empty());
        assert_eq!(result.skipped, vec![SkippedNode::InvalidNode]);
    }

    #[test]
    fn preserves_three_subscription_provider_relationships() {
        let inputs = [
            include_str!("../../tests/fixtures/subscriptions/proxies.json"),
            include_str!("../../tests/fixtures/subscriptions/clash.yaml"),
            include_str!("../../tests/fixtures/subscriptions/nodes.uri"),
        ];
        let subscriptions = (0..inputs.len())
            .map(|index| crate::domain::Subscription {
                id: crate::domain::SubscriptionId(format!("subscription-{index}")),
                name: format!("Subscription {index}"),
            })
            .collect::<Vec<_>>();
        let providers = (0..inputs.len())
            .map(|index| crate::domain::Provider {
                id: ProviderId(format!("provider-{index}")),
                subscription_id: crate::domain::SubscriptionId(format!("subscription-{index}")),
                name: format!("Provider {index}"),
            })
            .collect::<Vec<_>>();
        let nodes = inputs
            .iter()
            .enumerate()
            .flat_map(|(index, input)| {
                normalize_nodes(
                    ProviderId(format!("provider-{index}")),
                    parse_subscription(input).expect("parse fixture").nodes,
                )
                .expect("normalize fixture")
            })
            .collect();
        let state = crate::domain::AppState {
            schema_version: crate::domain::CURRENT_SCHEMA_VERSION,
            subscriptions,
            providers,
            nodes,
            pools: Vec::new(),
            routes: Vec::new(),
        };

        assert_eq!(state.validate(), Ok(()));
    }
}
