use std::collections::BTreeMap;
use std::fmt;

use serde_json::Value;

use crate::domain::{ProtocolOptions, ProxyProtocol, TlsOptions, Transport};

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
    if looks_like_yaml(body)
        && let Ok(result) = parse_clash_yaml(body)
    {
        return Ok(result);
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
        if has_unknown_json_fields(candidate) {
            skipped.push(SkippedNode::InvalidNode);
            continue;
        }
        let fields = JsonFields::from(candidate);
        if has_illegal_protocol_fields(&fields) {
            skipped.push(SkippedNode::InvalidNode);
            continue;
        }
        match draft_from_fields(&fields) {
            Ok(draft) => nodes.push(draft),
            Err(reason) => skipped.push(reason),
        }
    }
    (nodes, skipped)
}

fn has_unknown_json_fields(candidate: &Value) -> bool {
    const KNOWN: &[&str] = &[
        "name",
        "tag",
        "ps",
        "type",
        "server",
        "server_port",
        "port",
        "add",
        "uuid",
        "id",
        "password",
        "username",
        "user",
        "method",
        "cipher",
        "network",
        "net",
        "tls",
        "sni",
        "skip-cert-verify",
        "ws-opts",
        "grpc-opts",
        "transport",
        "path",
        "host",
        "alter_id",
        "security",
        "flow",
        "private_key",
        "private-key",
        "peer_public_key",
        "peer-public-key",
        "pre_shared_key",
        "pre-shared-key",
        "local_address",
        "local-address",
        "local_addresses",
        "local-addresses",
        "mtu",
        "reserved",
        "auth",
        "auth_str",
        "auth-str",
        "obfs",
        "up_mbps",
        "up-mbps",
        "down_mbps",
        "down-mbps",
        "congestion_control",
        "congestion-control",
        "udp_relay_mode",
        "udp-relay-mode",
        "zero_rtt",
        "zero-rtt",
        "version",
        "private_key_passphrase",
        "private-key-passphrase",
        "host_key",
        "host-key",
        "psk",
    ];
    candidate.as_object().is_none_or(|object| {
        object.keys().any(|key| !KNOWN.contains(&key.as_str()))
            || has_unknown_nested_fields(candidate)
    })
}

fn has_unknown_nested_fields(candidate: &Value) -> bool {
    const WEBSOCKET: &[&str] = &["path", "headers"];
    const GRPC: &[&str] = &["grpc-service-name"];
    const HEADERS: &[&str] = &["Host"];

    let contains_unknown = |value: &Value, allowed: &[&str]| {
        value
            .as_object()
            .is_none_or(|object| object.keys().any(|key| !allowed.contains(&key.as_str())))
    };
    let nested_has_unknown = |key: &str, allowed: &[&str]| {
        candidate
            .get(key)
            .is_some_and(|value| contains_unknown(value, allowed))
    };
    let headers_have_unknown = |value: &Value| {
        value
            .get("headers")
            .is_some_and(|headers| contains_unknown(headers, HEADERS))
    };

    candidate.get("tls").is_some_and(invalid_tls_value)
        || candidate
            .get("transport")
            .is_some_and(invalid_transport_value)
        || candidate.get("transport").is_some_and(headers_have_unknown)
        || nested_has_unknown("ws-opts", WEBSOCKET)
        || candidate.get("ws-opts").is_some_and(headers_have_unknown)
        || nested_has_unknown("grpc-opts", GRPC)
}

fn invalid_tls_value(tls: &Value) -> bool {
    const TLS: &[&str] = &["enabled", "server_name", "insecure", "reality"];
    const REALITY: &[&str] = &["enabled", "public_key", "short_id"];
    match tls {
        Value::Bool(_) => false,
        Value::String(value) => value != "tls",
        Value::Object(object) => {
            if object.keys().any(|key| !TLS.contains(&key.as_str()))
                || !object.get("enabled").is_some_and(Value::is_boolean)
            {
                return true;
            }
            if !object
                .get("enabled")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return object.len() != 1;
            }
            if object
                .get("server_name")
                .is_some_and(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
                || object
                    .get("insecure")
                    .is_some_and(|value| !value.is_boolean())
            {
                return true;
            }
            object.get("reality").is_some_and(|reality| {
                let Some(reality) = reality.as_object() else {
                    return true;
                };
                if reality.keys().any(|key| !REALITY.contains(&key.as_str()))
                    || !reality.get("enabled").is_some_and(Value::is_boolean)
                {
                    return true;
                }
                if !reality
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    return reality.len() != 1;
                }
                ["public_key", "short_id"].iter().any(|key| {
                    reality
                        .get(*key)
                        .and_then(Value::as_str)
                        .is_none_or(|value| value.trim().is_empty())
                })
            })
        }
        _ => true,
    }
}

fn invalid_transport_value(transport: &Value) -> bool {
    const TRANSPORT: &[&str] = &["type", "path", "headers", "service_name"];
    let Some(object) = transport.as_object() else {
        return true;
    };
    if object.keys().any(|key| !TRANSPORT.contains(&key.as_str())) {
        return true;
    }
    match object.get("type").and_then(Value::as_str) {
        Some("ws") => object
            .get("service_name")
            .is_some_and(|value| !value.is_null()),
        Some("grpc") => {
            object.get("path").is_some_and(|value| !value.is_null())
                || object.get("headers").is_some_and(|value| !value.is_null())
        }
        _ => true,
    }
}

fn has_illegal_protocol_fields(fields: &JsonFields) -> bool {
    const COMMON: &[&str] = &[
        "name",
        "tag",
        "ps",
        "type",
        "server",
        "server_port",
        "port",
        "add",
        "network",
        "net",
        "tls",
        "sni",
        "skip-cert-verify",
        "ws-opts",
        "grpc-opts",
        "transport",
        "path",
        "host",
    ];
    let Some(protocol) = fields.protocol.as_deref().and_then(protocol_from_name) else {
        return false;
    };
    let protocol_fields = match protocol {
        ProxyProtocol::Socks => &["username", "password", "version"][..],
        ProxyProtocol::Http => &["username", "password"][..],
        ProxyProtocol::Shadowsocks => &["method", "cipher", "password"][..],
        ProxyProtocol::Vmess => &["uuid", "id", "alter_id", "security"][..],
        ProxyProtocol::Vless => &["uuid", "id", "flow"][..],
        ProxyProtocol::Trojan => &["password"][..],
        ProxyProtocol::WireGuard => &[
            "private_key",
            "private-key",
            "peer_public_key",
            "peer-public-key",
            "pre_shared_key",
            "pre-shared-key",
            "local_address",
            "local-address",
            "local_addresses",
            "local-addresses",
            "mtu",
            "reserved",
        ][..],
        ProxyProtocol::Hysteria => &[
            "auth",
            "auth_str",
            "auth-str",
            "obfs",
            "up_mbps",
            "up-mbps",
            "down_mbps",
            "down-mbps",
        ][..],
        ProxyProtocol::Hysteria2 => &["password", "obfs"][..],
        ProxyProtocol::Tuic => &[
            "uuid",
            "id",
            "password",
            "congestion_control",
            "congestion-control",
            "udp_relay_mode",
            "udp-relay-mode",
            "zero_rtt",
            "zero-rtt",
        ][..],
        ProxyProtocol::ShadowTls => &["version", "password"][..],
        ProxyProtocol::Ssh => &[
            "user",
            "username",
            "password",
            "private_key",
            "private-key",
            "private_key_passphrase",
            "private-key-passphrase",
            "host_key",
            "host-key",
        ][..],
        ProxyProtocol::Naive => &["username", "password"][..],
        ProxyProtocol::AnyTls => &["password"][..],
        ProxyProtocol::Snell => &["psk", "password", "version"][..],
    };
    fields
        .raw
        .keys()
        .any(|key| !COMMON.contains(&key.as_str()) && !protocol_fields.contains(&key.as_str()))
}

fn looks_like_yaml(body: &str) -> bool {
    body.lines()
        .any(|line| line.trim_start().starts_with("proxies:"))
}

fn parse_clash_yaml(body: &str) -> Result<ParseResult, ParseError> {
    let document: Value =
        serde_yaml_ng::from_str(body).map_err(|_| ParseError::UnsupportedInput)?;
    let candidates = document
        .get("proxies")
        .and_then(Value::as_array)
        .ok_or(ParseError::UnsupportedInput)?;
    let (nodes, skipped) = parse_json_candidates(candidates);
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
        if has_invalid_legacy_vmess_fields(object) {
            return Err(SkippedNode::InvalidNode);
        }
        object.remove("v");
        object.remove("type");
        if let Some(alter_id) = object.remove("aid") {
            object.insert("alter_id".to_owned(), alter_id);
        }
        if let Some(security) = object.remove("scy") {
            object.insert("security".to_owned(), security);
        }
        if object.get("net").and_then(Value::as_str) == Some("grpc") {
            let service_name = object.remove("path").ok_or(SkippedNode::InvalidNode)?;
            object.insert(
                "grpc-opts".to_owned(),
                serde_json::json!({ "grpc-service-name": service_name }),
            );
        }
        if object
            .get("tls")
            .is_some_and(|value| value.as_str() == Some(""))
        {
            object.remove("tls");
        }
        object.insert("type".to_owned(), Value::String("vmess".to_owned()));
        if has_unknown_json_fields(&value) {
            return Err(SkippedNode::InvalidNode);
        }
        let fields = JsonFields::from(&value);
        if has_illegal_protocol_fields(&fields) {
            return Err(SkippedNode::InvalidNode);
        }
        let mut draft = draft_from_fields(&fields)?;
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
    let query = parse_query(query)?;
    if has_unknown_uri_query(protocol, &query) {
        return Err(SkippedNode::InvalidNode);
    }
    if query.iter().any(|(key, value)| {
        !matches!(key.as_str(), "allowInsecure" | "zero_rtt") && value.trim().is_empty()
    }) {
        return Err(SkippedNode::InvalidNode);
    }
    let name = if fragment.is_empty() {
        format!("{scheme} {server}")
    } else {
        percent_decode(fragment)
    };
    let options = parse_uri_options(protocol, scheme, credentials, &query)?;
    if !options.is_compatible_with(protocol) {
        return Err(SkippedNode::InvalidNode);
    }
    let tls = parse_uri_tls(protocol, &query)?;
    let transport = parse_uri_transport(protocol, &query)?;
    Ok(ProxyNodeDraft {
        name,
        protocol,
        server,
        port,
        options,
        transport,
        tls,
    })
}

fn has_invalid_legacy_vmess_fields(object: &serde_json::Map<String, Value>) -> bool {
    const KNOWN: &[&str] = &[
        "v", "ps", "add", "port", "id", "aid", "scy", "net", "type", "host", "path", "tls", "sni",
        "security", "pbk", "sid",
    ];
    let required_string = |key: &str| {
        object
            .get(key)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
    };
    if object.keys().any(|key| !KNOWN.contains(&key.as_str()))
        || object.get("v").and_then(Value::as_str) != Some("2")
        || ["ps", "add", "id"].iter().any(|key| required_string(key))
        || object
            .get("type")
            .is_some_and(|value| value.as_str() != Some("none"))
        || object.get("port").is_none_or(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
                .is_none_or(|port: u64| port == 0 || port > u64::from(u16::MAX))
        })
        || object.get("aid").is_some_and(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
                .is_none()
        })
        || object
            .get("scy")
            .is_some_and(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
        || object
            .get("security")
            .is_some_and(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
    {
        return true;
    }
    if object.get("tls").is_some_and(|value| {
        value
            .as_str()
            .is_none_or(|value| !value.is_empty() && value != "tls")
    }) || object
        .get("sni")
        .is_some_and(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
        || (object.contains_key("sni") && object.get("tls").and_then(Value::as_str) != Some("tls"))
    {
        return true;
    }
    match object.get("net").and_then(Value::as_str) {
        Some("tcp") => object.contains_key("path") || object.contains_key("host"),
        Some("ws") => {
            required_string("path")
                || object
                    .get("host")
                    .is_some_and(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
        }
        Some("grpc") => required_string("path") || object.contains_key("host"),
        _ => true,
    }
}

fn parse_uri_tls(
    protocol: ProxyProtocol,
    query: &BTreeMap<String, String>,
) -> Result<Option<TlsOptions>, SkippedNode> {
    let has_tls_query = ["security", "sni", "allowInsecure", "pbk", "sid"]
        .iter()
        .any(|key| query.contains_key(*key));
    if has_tls_query
        && !matches!(
            protocol,
            ProxyProtocol::Vmess
                | ProxyProtocol::Vless
                | ProxyProtocol::Trojan
                | ProxyProtocol::Hysteria
                | ProxyProtocol::Hysteria2
                | ProxyProtocol::Tuic
                | ProxyProtocol::Naive
                | ProxyProtocol::AnyTls
        )
    {
        return Err(SkippedNode::InvalidNode);
    }
    if (query.contains_key("pbk") || query.contains_key("sid"))
        && query.get("security").map(String::as_str) != Some("reality")
    {
        return Err(SkippedNode::InvalidNode);
    }
    let allow_insecure = match query.get("allowInsecure") {
        None => false,
        Some(value) if value == "0" => false,
        Some(value) if value == "1" => true,
        Some(_) => return Err(SkippedNode::InvalidNode),
    };
    let server_name = query
        .get("sni")
        .filter(|value| !value.trim().is_empty())
        .cloned();
    if query.get("sni").is_some() && server_name.is_none() {
        return Err(SkippedNode::InvalidNode);
    }
    match query.get("security").map(String::as_str) {
        Some("tls") => Ok(Some(TlsOptions {
            server_name,
            allow_insecure,
            reality_public_key: None,
            reality_short_id: None,
        })),
        Some("reality") => Ok(Some(TlsOptions {
            server_name,
            allow_insecure,
            reality_public_key: Some(required_query_string(query, &["pbk"])?),
            reality_short_id: Some(required_query_string(query, &["sid"])?),
        })),
        Some("none") if !allow_insecure && server_name.is_none() => Ok(None),
        Some(_) => Err(SkippedNode::InvalidNode),
        None if matches!(
            protocol,
            ProxyProtocol::Trojan
                | ProxyProtocol::Hysteria
                | ProxyProtocol::Hysteria2
                | ProxyProtocol::Tuic
                | ProxyProtocol::Naive
        ) =>
        {
            Ok(Some(TlsOptions {
                server_name,
                allow_insecure,
                reality_public_key: None,
                reality_short_id: None,
            }))
        }
        None if allow_insecure || server_name.is_some() => Err(SkippedNode::InvalidNode),
        None => Ok(None),
    }
}

fn parse_uri_transport(
    protocol: ProxyProtocol,
    query: &BTreeMap<String, String>,
) -> Result<Option<Transport>, SkippedNode> {
    if query.contains_key("type")
        && !matches!(
            protocol,
            ProxyProtocol::Vmess | ProxyProtocol::Vless | ProxyProtocol::Trojan
        )
    {
        return Err(SkippedNode::InvalidNode);
    }
    match query.get("type").map(String::as_str) {
        None | Some("tcp") => {
            if query.contains_key("path")
                || query.contains_key("host")
                || query.contains_key("serviceName")
            {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(Some(Transport::Tcp))
        }
        Some("ws") => {
            if query.contains_key("serviceName") {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(Some(Transport::Websocket {
                path: query.get("path").cloned().unwrap_or_else(|| "/".to_owned()),
                host: query.get("host").cloned(),
            }))
        }
        Some("grpc") => {
            if query.contains_key("path") || query.contains_key("host") {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(Some(Transport::Grpc {
                service_name: query.get("serviceName").cloned().unwrap_or_default(),
            }))
        }
        Some(_) => Err(SkippedNode::InvalidNode),
    }
}

fn parse_uri_options(
    protocol: ProxyProtocol,
    scheme: &str,
    raw: &str,
    query: &BTreeMap<String, String>,
) -> Result<ProtocolOptions, SkippedNode> {
    let raw = percent_decode(raw);
    match protocol {
        ProxyProtocol::Vmess => {
            if raw.is_empty() {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(ProtocolOptions::Vmess {
                uuid: raw,
                alter_id: None,
                security: None,
            })
        }
        ProxyProtocol::Vless => {
            if raw.is_empty() {
                return Err(SkippedNode::InvalidNode);
            }
            Ok(ProtocolOptions::Vless {
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
            Ok(ProtocolOptions::Shadowsocks {
                method: cipher.to_owned(),
                password: password.to_owned(),
            })
        }
        ProxyProtocol::Socks => {
            let (username, password) = raw.split_once(':').unwrap_or(("", ""));
            Ok(ProtocolOptions::Socks {
                version: query
                    .get("version")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(5),
                username: (!username.is_empty()).then(|| username.to_owned()),
                password: (!password.is_empty()).then(|| password.to_owned()),
            })
        }
        ProxyProtocol::Http => {
            let (username, password) = raw.split_once(':').unwrap_or(("", ""));
            Ok(ProtocolOptions::Http {
                username: (!username.is_empty()).then(|| username.to_owned()),
                password: (!password.is_empty()).then(|| password.to_owned()),
                tls: scheme.eq_ignore_ascii_case("https"),
            })
        }
        ProxyProtocol::Trojan => {
            password_option(raw).map(|password| ProtocolOptions::Trojan { password })
        }
        ProxyProtocol::Hysteria2 => {
            password_option(raw).map(|password| ProtocolOptions::Hysteria2 {
                password,
                obfs: query.get("obfs-password").cloned(),
            })
        }
        ProxyProtocol::AnyTls => {
            password_option(raw).map(|password| ProtocolOptions::AnyTls { password })
        }
        ProxyProtocol::WireGuard => Ok(ProtocolOptions::WireGuard {
            private_key: password_option(raw)?,
            peer_public_key: required_query_string(query, &["peer_public_key", "publickey"])?,
            pre_shared_key: query
                .get("pre_shared_key")
                .or_else(|| query.get("presharedkey"))
                .cloned(),
            local_addresses: query_string_list(query, &["local_address", "address"])?,
            mtu: optional_query_u32(query, "mtu")?,
            reserved: optional_query_reserved(query)?,
        }),
        ProxyProtocol::Hysteria => Ok(ProtocolOptions::Hysteria {
            auth: (!raw.is_empty())
                .then_some(raw)
                .or_else(|| query.get("auth").cloned())
                .or_else(|| query.get("auth_str").cloned())
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            obfs: query.get("obfs").cloned(),
            up_mbps: optional_query_u32_alias(query, &["up_mbps", "upmbps"])?,
            down_mbps: optional_query_u32_alias(query, &["down_mbps", "downmbps"])?,
        }),
        ProxyProtocol::Tuic => {
            let (uuid, password) = raw.split_once(':').ok_or(SkippedNode::InvalidNode)?;
            Ok(ProtocolOptions::Tuic {
                uuid: password_option(uuid.to_owned())?,
                password: password_option(password.to_owned())?,
                congestion_control: query.get("congestion_control").cloned(),
                udp_relay_mode: query.get("udp_relay_mode").cloned(),
                zero_rtt: optional_query_bool(query, "zero_rtt")?.unwrap_or(false),
            })
        }
        ProxyProtocol::ShadowTls => Ok(ProtocolOptions::ShadowTls {
            version: optional_query_u8(query, "version")?.unwrap_or(3),
            password: password_option(raw)?,
        }),
        ProxyProtocol::Ssh => {
            let (user, password) = raw.split_once(':').unwrap_or((&raw, ""));
            Ok(ProtocolOptions::Ssh {
                user: password_option(user.to_owned())?,
                password: (!password.is_empty()).then(|| password.to_owned()),
                private_key: query.get("private_key").cloned(),
                private_key_passphrase: query.get("private_key_passphrase").cloned(),
                host_key: query.get("host_key").cloned(),
            })
        }
        ProxyProtocol::Naive => {
            let (username, password) = raw.split_once(':').ok_or(SkippedNode::InvalidNode)?;
            Ok(ProtocolOptions::Naive {
                username: password_option(username.to_owned())?,
                password: password_option(password.to_owned())?,
            })
        }
        ProxyProtocol::Snell => Ok(ProtocolOptions::Snell {
            psk: password_option(raw)?,
            version: optional_query_u8(query, "version")?.unwrap_or(3),
        }),
    }
}

fn has_unknown_uri_query(protocol: ProxyProtocol, query: &BTreeMap<String, String>) -> bool {
    const COMMON: &[&str] = &[
        "security",
        "sni",
        "allowInsecure",
        "type",
        "path",
        "host",
        "serviceName",
        "pbk",
        "sid",
    ];
    let protocol_fields = match protocol {
        ProxyProtocol::Socks => &["version"][..],
        ProxyProtocol::Vless => &["flow"][..],
        ProxyProtocol::Hysteria2 => &["obfs-password"][..],
        ProxyProtocol::WireGuard => &[
            "peer_public_key",
            "publickey",
            "pre_shared_key",
            "presharedkey",
            "local_address",
            "address",
            "mtu",
            "reserved",
        ][..],
        ProxyProtocol::Hysteria => &[
            "auth",
            "auth_str",
            "obfs",
            "up_mbps",
            "upmbps",
            "down_mbps",
            "downmbps",
        ][..],
        ProxyProtocol::Tuic => &["congestion_control", "udp_relay_mode", "zero_rtt"][..],
        ProxyProtocol::ShadowTls | ProxyProtocol::Snell => &["version"][..],
        ProxyProtocol::Ssh => &["private_key", "private_key_passphrase", "host_key"][..],
        ProxyProtocol::Vmess
        | ProxyProtocol::Http
        | ProxyProtocol::Shadowsocks
        | ProxyProtocol::Trojan
        | ProxyProtocol::Naive
        | ProxyProtocol::AnyTls => &[][..],
    };
    query
        .keys()
        .any(|key| !COMMON.contains(&key.as_str()) && !protocol_fields.contains(&key.as_str()))
}

fn required_query_string(
    query: &BTreeMap<String, String>,
    keys: &[&str],
) -> Result<String, SkippedNode> {
    keys.iter()
        .find_map(|key| query.get(*key))
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or(SkippedNode::InvalidNode)
}

fn query_string_list(
    query: &BTreeMap<String, String>,
    keys: &[&str],
) -> Result<Vec<String>, SkippedNode> {
    let values = required_query_string(query, keys)?
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return Err(SkippedNode::InvalidNode);
    }
    Ok(values)
}

fn optional_query_u32(
    query: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<u32>, SkippedNode> {
    query
        .get(key)
        .map(|value| value.parse().map_err(|_| SkippedNode::InvalidNode))
        .transpose()
}

fn optional_query_u32_alias(
    query: &BTreeMap<String, String>,
    keys: &[&str],
) -> Result<Option<u32>, SkippedNode> {
    let values = keys
        .iter()
        .filter_map(|key| query.get(*key))
        .collect::<Vec<_>>();
    match values.as_slice() {
        [] => Ok(None),
        [value] => value
            .parse()
            .map(Some)
            .map_err(|_| SkippedNode::InvalidNode),
        _ => Err(SkippedNode::InvalidNode),
    }
}

fn optional_query_u8(
    query: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<u8>, SkippedNode> {
    optional_query_u32(query, key)?.map_or(Ok(None), |value| {
        u8::try_from(value)
            .map(Some)
            .map_err(|_| SkippedNode::InvalidNode)
    })
}

fn optional_query_bool(
    query: &BTreeMap<String, String>,
    key: &str,
) -> Result<Option<bool>, SkippedNode> {
    match query.get(key).map(String::as_str) {
        None => Ok(None),
        Some("0") => Ok(Some(false)),
        Some("1") => Ok(Some(true)),
        Some(_) => Err(SkippedNode::InvalidNode),
    }
}

fn optional_query_reserved(
    query: &BTreeMap<String, String>,
) -> Result<Option<[u8; 3]>, SkippedNode> {
    let Some(value) = query.get("reserved") else {
        return Ok(None);
    };
    value
        .split(',')
        .map(|value| value.parse::<u8>().map_err(|_| SkippedNode::InvalidNode))
        .collect::<Result<Vec<_>, _>>()?
        .try_into()
        .map(Some)
        .map_err(|_| SkippedNode::InvalidNode)
}

fn password_option(raw: String) -> Result<String, SkippedNode> {
    if raw.is_empty() {
        Err(SkippedNode::InvalidNode)
    } else {
        Ok(raw)
    }
}

fn protocol_from_name(value: &str) -> Option<ProxyProtocol> {
    Some(match value.to_ascii_lowercase().as_str() {
        "ss" | "shadowsocks" => ProxyProtocol::Shadowsocks,
        "vmess" => ProxyProtocol::Vmess,
        "vless" => ProxyProtocol::Vless,
        "trojan" => ProxyProtocol::Trojan,
        "wireguard" | "wg" => ProxyProtocol::WireGuard,
        "hysteria" | "hy" => ProxyProtocol::Hysteria,
        "hysteria2" | "hy2" => ProxyProtocol::Hysteria2,
        "tuic" => ProxyProtocol::Tuic,
        "shadowtls" => ProxyProtocol::ShadowTls,
        "ssh" => ProxyProtocol::Ssh,
        "naive" | "naive+https" => ProxyProtocol::Naive,
        "socks" | "socks5" => ProxyProtocol::Socks,
        "http" | "https" => ProxyProtocol::Http,
        "anytls" | "any_tls" => ProxyProtocol::AnyTls,
        "snell" => ProxyProtocol::Snell,
        _ => return None,
    })
}

fn split_host_port(input: &str) -> Result<(String, u16), SkippedNode> {
    let (host, port) = input.rsplit_once(':').ok_or(SkippedNode::InvalidNode)?;
    let port = port.parse().map_err(|_| SkippedNode::InvalidNode)?;
    if host.is_empty() || port == 0 {
        return Err(SkippedNode::InvalidNode);
    }
    Ok((host.trim_matches(['[', ']']).to_owned(), port))
}

fn parse_query(query: &str) -> Result<BTreeMap<String, String>, SkippedNode> {
    if query.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut values = BTreeMap::new();
    for entry in query.split('&') {
        let (key, value) = entry.split_once('=').ok_or(SkippedNode::InvalidNode)?;
        let key = percent_decode(key);
        if key.is_empty() || values.insert(key, percent_decode(value)).is_some() {
            return Err(SkippedNode::InvalidNode);
        }
    }
    Ok(values)
}

fn percent_decode(value: &str) -> String {
    let mut decoded = String::with_capacity(value.len());
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3])
            && let Ok(byte) = u8::from_str_radix(hex, 16)
        {
            decoded.push(char::from(byte));
            index += 3;
            continue;
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
    reality_public_key: Option<String>,
    reality_short_id: Option<String>,
    websocket_path: Option<String>,
    websocket_host: Option<String>,
    grpc_service_name: Option<String>,
    raw: BTreeMap<String, Value>,
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
            skip_cert_verify: value
                .get("skip-cert-verify")
                .and_then(Value::as_bool)
                .or_else(|| {
                    value
                        .get("tls")
                        .and_then(|item| item.get("insecure"))
                        .and_then(Value::as_bool)
                }),
            reality_public_key: value
                .get("tls")
                .and_then(|item| item.get("reality"))
                .and_then(|item| item.get("public_key"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            reality_short_id: value
                .get("tls")
                .and_then(|item| item.get("reality"))
                .and_then(|item| item.get("short_id"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            websocket_path: value
                .get("ws-opts")
                .and_then(|item| item.get("path"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    transport
                        .and_then(|item| item.get("path"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .or_else(|| get_string("path")),
            websocket_host: value
                .get("ws-opts")
                .and_then(|item| item.get("headers"))
                .and_then(|item| item.get("Host"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    transport
                        .and_then(|item| item.get("headers"))
                        .and_then(|item| item.get("Host"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .or_else(|| get_string("host")),
            grpc_service_name: value
                .get("grpc-opts")
                .and_then(|item| item.get("grpc-service-name"))
                .and_then(Value::as_str)
                .map(str::to_owned)
                .or_else(|| {
                    transport
                        .and_then(|item| item.get("service_name"))
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                }),
            raw: value
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect(),
        }
    }
}

fn required_raw_string(fields: &JsonFields, key: &str) -> Result<String, SkippedNode> {
    optional_raw_string(fields, key)?.ok_or(SkippedNode::InvalidNode)
}

fn raw_value<'a>(fields: &'a JsonFields, key: &str) -> Result<Option<&'a Value>, SkippedNode> {
    let dashed = key.replace('_', "-");
    match (fields.raw.get(key), fields.raw.get(&dashed)) {
        (Some(_), Some(_)) if dashed != key => Err(SkippedNode::InvalidNode),
        (Some(value), _) | (_, Some(value)) => Ok(Some(value)),
        (None, None) => Ok(None),
    }
}

fn optional_raw_string(fields: &JsonFields, key: &str) -> Result<Option<String>, SkippedNode> {
    match raw_value(fields, key)? {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        _ => Err(SkippedNode::InvalidNode),
    }
}

fn optional_raw_u32(fields: &JsonFields, key: &str) -> Result<Option<u32>, SkippedNode> {
    match raw_value(fields, key)? {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or(SkippedNode::InvalidNode),
        _ => Err(SkippedNode::InvalidNode),
    }
}

fn optional_raw_u8(fields: &JsonFields, key: &str) -> Result<Option<u8>, SkippedNode> {
    optional_raw_u32(fields, key)?.map_or(Ok(None), |value| {
        u8::try_from(value)
            .map(Some)
            .map_err(|_| SkippedNode::InvalidNode)
    })
}

fn optional_raw_bool(fields: &JsonFields, key: &str) -> Result<Option<bool>, SkippedNode> {
    match raw_value(fields, key)? {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        _ => Err(SkippedNode::InvalidNode),
    }
}

fn raw_string_list(fields: &JsonFields, key: &str) -> Result<Vec<String>, SkippedNode> {
    let Some(value) = raw_value(fields, key)? else {
        return Err(SkippedNode::InvalidNode);
    };
    let values = match value {
        Value::String(value) => vec![value.clone()],
        Value::Array(values) => values
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .map(str::to_owned)
                    .ok_or(SkippedNode::InvalidNode)
            })
            .collect::<Result<Vec<_>, _>>()?,
        _ => return Err(SkippedNode::InvalidNode),
    };
    if values.is_empty() || values.iter().any(|value| value.trim().is_empty()) {
        return Err(SkippedNode::InvalidNode);
    }
    Ok(values)
}

fn optional_reserved(fields: &JsonFields) -> Result<Option<[u8; 3]>, SkippedNode> {
    let Some(value) = raw_value(fields, "reserved")? else {
        return Ok(None);
    };
    let Value::Array(values) = value else {
        return Err(SkippedNode::InvalidNode);
    };
    let values = values
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| u8::try_from(value).ok())
                .ok_or(SkippedNode::InvalidNode)
        })
        .collect::<Result<Vec<_>, _>>()?;
    values
        .try_into()
        .map(Some)
        .map_err(|_| SkippedNode::InvalidNode)
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
    let options = match protocol {
        ProxyProtocol::Vmess => ProtocolOptions::Vmess {
            uuid: fields
                .uuid
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            alter_id: optional_raw_u32(fields, "alter_id")?,
            security: optional_raw_string(fields, "security")?,
        },
        ProxyProtocol::Vless => ProtocolOptions::Vless {
            uuid: fields
                .uuid
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            flow: optional_raw_string(fields, "flow")?,
        },
        ProxyProtocol::Shadowsocks => ProtocolOptions::Shadowsocks {
            method: fields
                .cipher
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            password: fields
                .password
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
        },
        ProxyProtocol::Socks => ProtocolOptions::Socks {
            version: optional_raw_u8(fields, "version")?.unwrap_or(5),
            username: fields.username.clone(),
            password: fields.password.clone(),
        },
        ProxyProtocol::Http => ProtocolOptions::Http {
            username: fields.username.clone(),
            password: fields.password.clone(),
            tls: fields.tls.unwrap_or(false)
                || fields
                    .protocol
                    .as_deref()
                    .is_some_and(|value| value.eq_ignore_ascii_case("https")),
        },
        ProxyProtocol::Trojan => ProtocolOptions::Trojan {
            password: fields
                .password
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
        },
        ProxyProtocol::Hysteria2 => ProtocolOptions::Hysteria2 {
            password: fields
                .password
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            obfs: optional_raw_string(fields, "obfs")?,
        },
        ProxyProtocol::WireGuard => ProtocolOptions::WireGuard {
            private_key: required_raw_string(fields, "private_key")?,
            peer_public_key: required_raw_string(fields, "peer_public_key")?,
            pre_shared_key: optional_raw_string(fields, "pre_shared_key")?,
            local_addresses: raw_string_list(fields, "local_address")
                .or_else(|_| raw_string_list(fields, "local_addresses"))?,
            mtu: optional_raw_u32(fields, "mtu")?,
            reserved: optional_reserved(fields)?,
        },
        ProxyProtocol::Hysteria => ProtocolOptions::Hysteria {
            auth: optional_raw_string(fields, "auth")?
                .or(optional_raw_string(fields, "auth_str")?)
                .ok_or(SkippedNode::InvalidNode)?,
            obfs: optional_raw_string(fields, "obfs")?,
            up_mbps: optional_raw_u32(fields, "up_mbps")?,
            down_mbps: optional_raw_u32(fields, "down_mbps")?,
        },
        ProxyProtocol::Tuic => ProtocolOptions::Tuic {
            uuid: fields
                .uuid
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            password: fields
                .password
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            congestion_control: optional_raw_string(fields, "congestion_control")?,
            udp_relay_mode: optional_raw_string(fields, "udp_relay_mode")?,
            zero_rtt: optional_raw_bool(fields, "zero_rtt")?.unwrap_or(false),
        },
        ProxyProtocol::ShadowTls => ProtocolOptions::ShadowTls {
            version: optional_raw_u8(fields, "version")?.unwrap_or(3),
            password: fields
                .password
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
        },
        ProxyProtocol::Ssh => ProtocolOptions::Ssh {
            user: optional_raw_string(fields, "user")?
                .or(optional_raw_string(fields, "username")?)
                .ok_or(SkippedNode::InvalidNode)?,
            password: fields.password.clone(),
            private_key: optional_raw_string(fields, "private_key")?,
            private_key_passphrase: optional_raw_string(fields, "private_key_passphrase")?,
            host_key: optional_raw_string(fields, "host_key")?,
        },
        ProxyProtocol::Naive => ProtocolOptions::Naive {
            username: fields
                .username
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            password: fields
                .password
                .clone()
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
        },
        ProxyProtocol::AnyTls => ProtocolOptions::AnyTls {
            password: fields
                .password
                .clone()
                .filter(|value| !value.is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
        },
        ProxyProtocol::Snell => ProtocolOptions::Snell {
            psk: optional_raw_string(fields, "psk")?
                .or(fields.password.clone())
                .filter(|value| !value.trim().is_empty())
                .ok_or(SkippedNode::InvalidNode)?,
            version: optional_raw_u8(fields, "version")?.unwrap_or(3),
        },
    };
    if !options.is_compatible_with(protocol) {
        return Err(SkippedNode::InvalidNode);
    }
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
        reality_public_key: fields.reality_public_key.clone(),
        reality_short_id: fields.reality_short_id.clone(),
    });
    Ok(ProxyNodeDraft {
        name,
        protocol,
        server,
        port,
        options,
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
        assert_eq!(result.skipped, vec![SkippedNode::InvalidNode]);
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
    fn parses_the_remaining_stable_non_tor_uri_schemes() {
        let result = parse_subscription(
            "wireguard://private@wg.example.invalid:51820?publickey=public&address=10.0.0.2%2F32#WG\n\
             hysteria://auth@hy.example.invalid:443?upmbps=10&downmbps=20#HY\n\
             tuic://uuid:password@tuic.example.invalid:443?congestion_control=bbr&udp_relay_mode=native#TUIC\n\
             shadowtls://password@shadowtls.example.invalid:443?version=3#ShadowTLS\n\
             ssh://user:password@ssh.example.invalid:22#SSH\n\
             naive+https://user:password@naive.example.invalid:443#Naive\n\
             snell://psk@snell.example.invalid:443?version=3#Snell",
        )
        .expect("parse URI schemes");

        assert_eq!(result.nodes.len(), 7);
        assert!(result.skipped.is_empty());
        assert_eq!(
            result
                .nodes
                .iter()
                .map(|node| node.protocol)
                .collect::<Vec<_>>(),
            vec![
                ProxyProtocol::WireGuard,
                ProxyProtocol::Hysteria,
                ProxyProtocol::Tuic,
                ProxyProtocol::ShadowTls,
                ProxyProtocol::Ssh,
                ProxyProtocol::Naive,
                ProxyProtocol::Snell,
            ]
        );
    }

    #[test]
    fn rejects_an_unknown_uri_query_field() {
        let result = parse_subscription(
            "tuic://uuid:password@example.invalid:443?congestion_control=bbr&unsupported=secret",
        )
        .expect("URI list is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(result.skipped, vec![SkippedNode::InvalidNode]);
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
    fn rejects_uri_nodes_with_incomplete_reality_fields() {
        let result = parse_subscription(
            "vless://fixture-uuid@example.invalid:443?security=reality&sid=fixture-short-id\n\
             vless://fixture-uuid@example.invalid:443?security=reality&pbk=",
        )
        .expect("uri list is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(
            result.skipped,
            vec![SkippedNode::InvalidNode, SkippedNode::InvalidNode]
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
    fn parses_clash_hyphenated_wireguard_fields() {
        let result = parse_subscription(
            "proxies:\n  - name: WireGuard\n    type: wireguard\n    server: example.invalid\n    port: 443\n    private-key: private\n    peer-public-key: public\n    local-address: [10.0.0.2/32]",
        )
        .expect("parse yaml");

        assert_eq!(result.skipped, Vec::new());
        assert!(matches!(
            result.nodes[0].options,
            ProtocolOptions::WireGuard {
                ref private_key,
                ref peer_public_key,
                ref local_addresses,
                ..
            } if private_key == "private"
                && peer_public_key == "public"
                && local_addresses == &["10.0.0.2/32"]
        ));
    }

    #[test]
    fn preserves_representable_sing_box_tls_and_transport_fields() {
        let result = parse_subscription(
            r#"{"outbounds":[{"tag":"node","type":"vless","server":"example.invalid","server_port":443,"uuid":"uuid","tls":{"enabled":true,"server_name":"tls.example.invalid","insecure":true,"reality":{"enabled":true,"public_key":"public","short_id":"short"}},"transport":{"type":"ws","path":"/ws","headers":{"Host":"edge.example.invalid"}}}]}"#,
        )
        .expect("parse sing-box node");

        assert_eq!(result.skipped, Vec::new());
        assert_eq!(
            result.nodes[0].transport,
            Some(Transport::Websocket {
                path: "/ws".to_owned(),
                host: Some("edge.example.invalid".to_owned()),
            })
        );
        assert_eq!(
            result.nodes[0].tls,
            Some(TlsOptions {
                server_name: Some("tls.example.invalid".to_owned()),
                allow_insecure: true,
                reality_public_key: Some("public".to_owned()),
                reality_short_id: Some("short".to_owned()),
            })
        );
    }

    #[test]
    fn rejects_protocol_specific_fields_on_the_wrong_node() {
        let result = parse_subscription(
            r#"{"outbounds":[{"tag":"node","type":"trojan","server":"example.invalid","server_port":443,"password":"secret","uuid":"not-applicable"}]}"#,
        )
        .expect("document is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(result.skipped, vec![SkippedNode::InvalidNode]);
    }

    #[test]
    fn rejects_a_tls_value_that_cannot_be_preserved_in_the_domain_model() {
        let result = parse_subscription(
            r#"{"outbounds":[{"tag":"node","type":"vless","server":"example.invalid","server_port":443,"uuid":"uuid","tls":"not-a-tls-setting"}]}"#,
        )
        .expect("document is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(result.skipped, vec![SkippedNode::InvalidNode]);
    }

    #[test]
    fn rejects_disabled_or_malformed_nested_tls_settings() {
        let result = parse_subscription(
            r#"{"outbounds":[{"tag":"node","type":"vless","server":"example.invalid","server_port":443,"uuid":"uuid","tls":{"enabled":true,"reality":{"enabled":false,"public_key":"public","short_id":"short"}}},{"tag":"other","type":"vless","server":"example.invalid","server_port":443,"uuid":"uuid","tls":{"enabled":false,"server_name":"not-allowed"}}]}"#,
        )
        .expect("document is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(
            result.skipped,
            vec![SkippedNode::InvalidNode, SkippedNode::InvalidNode]
        );
    }

    #[test]
    fn rejects_legacy_vmess_and_uri_values_that_cannot_be_represented() {
        let legacy = serde_json::json!({
            "v":"2", "ps":"node", "add":"example.invalid", "port":"443", "id":"uuid",
            "net":"tcp", "type":"none", "evil":"not-allowed"
        });
        let invalid_websocket_path = serde_json::json!({
            "v":"2", "ps":"node", "add":"example.invalid", "port":"443", "id":"uuid",
            "net":"ws", "type":"none", "path":123
        });
        let legacy_encoded = base64_encode(&legacy.to_string());
        let invalid_websocket_path_encoded = base64_encode(&invalid_websocket_path.to_string());
        let result = parse_subscription(&format!(
            "vmess://{legacy_encoded}\nvmess://{invalid_websocket_path_encoded}\nvless://uuid@example.invalid:443?security=xtls\nvless://uuid@example.invalid:443?type=http\ntuic://uuid:password@example.invalid:443?congestion_control=bbr&congestion_control=reno",
        ))
        .expect("URI list is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(result.skipped.len(), 5);
        assert!(
            result
                .skipped
                .iter()
                .all(|reason| *reason == SkippedNode::InvalidNode)
        );
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
            default_target: crate::domain::RouteTarget::Unconfigured,
            subscriptions,
            providers,
            nodes,
            pools: Vec::new(),
            routes: Vec::new(),
        };

        assert_eq!(state.validate(), Ok(()));
    }

    #[test]
    fn normalizes_every_non_tor_protocol_from_sing_box_outbounds() {
        let input = serde_json::json!({
            "outbounds": [
                {"tag":"socks","type":"socks","server":"example.invalid","server_port":1},
                {"tag":"http","type":"http","server":"example.invalid","server_port":2},
                {"tag":"ss","type":"shadowsocks","server":"example.invalid","server_port":3,"method":"aes-128-gcm","password":"secret"},
                {"tag":"vmess","type":"vmess","server":"example.invalid","server_port":4,"uuid":"uuid"},
                {"tag":"vless","type":"vless","server":"example.invalid","server_port":5,"uuid":"uuid"},
                {"tag":"trojan","type":"trojan","server":"example.invalid","server_port":6,"password":"secret"},
                {"tag":"wg","type":"wireguard","server":"example.invalid","server_port":7,"private_key":"private","peer_public_key":"public","local_address":["10.0.0.2/32"]},
                {"tag":"hy","type":"hysteria","server":"example.invalid","server_port":8,"auth":"secret"},
                {"tag":"hy2","type":"hysteria2","server":"example.invalid","server_port":9,"password":"secret"},
                {"tag":"tuic","type":"tuic","server":"example.invalid","server_port":10,"uuid":"uuid","password":"secret"},
                {"tag":"stls","type":"shadowtls","server":"example.invalid","server_port":11,"password":"secret"},
                {"tag":"ssh","type":"ssh","server":"example.invalid","server_port":12,"user":"root","password":"secret"},
                {"tag":"naive","type":"naive","server":"example.invalid","server_port":13,"username":"user","password":"secret"},
                {"tag":"anytls","type":"anytls","server":"example.invalid","server_port":14,"password":"secret"},
                {"tag":"snell","type":"snell","server":"example.invalid","server_port":15,"psk":"secret"}
            ]
        });

        let result = parse_subscription(&input.to_string()).expect("parse all protocol fixture");

        assert_eq!(result.nodes.len(), 15);
        assert!(result.skipped.is_empty());
        assert!(
            result
                .nodes
                .iter()
                .any(|node| node.protocol == ProxyProtocol::WireGuard)
        );
        assert!(
            result
                .nodes
                .iter()
                .any(|node| node.protocol == ProxyProtocol::Snell)
        );
    }

    #[test]
    fn rejects_unknown_json_fields_without_echoing_node_credentials() {
        let result = parse_subscription(
            r#"{"outbounds":[{"tag":"node","type":"trojan","server":"example.invalid","server_port":443,"password":"secret-not-in-diagnostics","unsupported_field":true}]}"#,
        )
        .expect("document is recognized");

        assert!(result.nodes.is_empty());
        assert_eq!(result.skipped, vec![SkippedNode::InvalidNode]);
        assert!(!format!("{:?}", result.skipped).contains("secret-not-in-diagnostics"));
    }

    fn base64_encode(input: &str) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut encoded = String::new();
        for chunk in input.as_bytes().chunks(3) {
            let value = chunk
                .iter()
                .fold(0_u32, |value, byte| (value << 8) | u32::from(*byte))
                << (8 * (3 - chunk.len()));
            encoded.push(ALPHABET[((value >> 18) & 0x3f) as usize] as char);
            encoded.push(ALPHABET[((value >> 12) & 0x3f) as usize] as char);
            encoded.push(if chunk.len() > 1 {
                ALPHABET[((value >> 6) & 0x3f) as usize] as char
            } else {
                '='
            });
            encoded.push(if chunk.len() > 2 {
                ALPHABET[(value & 0x3f) as usize] as char
            } else {
                '='
            });
        }
        encoded
    }
}
