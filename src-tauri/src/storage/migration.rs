use serde_json::{Map, Value, json};

use super::StateStoreError;
use crate::domain::CURRENT_SCHEMA_VERSION;

pub(crate) fn migrate_to_current(mut document: Value) -> Result<(Value, bool), StateStoreError> {
    let mut version = document
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(StateStoreError::MissingSchemaVersion)?;
    let mut migrated = false;

    while version < u64::from(CURRENT_SCHEMA_VERSION) {
        match version {
            1 => migrate_v1_to_v2(&mut document),
            2 => migrate_v2_to_v3(&mut document),
            _ => return Err(StateStoreError::UnsupportedSchemaVersion),
        }
        .map_err(|_| StateStoreError::MigrationFailed)?;
        version += 1;
        migrated = true;
    }

    if version == u64::from(CURRENT_SCHEMA_VERSION) {
        Ok((document, migrated))
    } else {
        Err(StateStoreError::UnsupportedSchemaVersion)
    }
}

fn migrate_v1_to_v2(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    object.insert("schema_version".to_owned(), json!(2));
    object.insert("pools".to_owned(), json!([]));
    object.insert("routes".to_owned(), json!([]));
    Ok(())
}

fn migrate_v2_to_v3(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let nodes = object
        .get_mut("nodes")
        .and_then(Value::as_array_mut)
        .ok_or(StateStoreError::InvalidStoredState)?;
    for node in nodes {
        migrate_v2_node(node)?;
    }
    object.insert("schema_version".to_owned(), json!(3));
    object.insert(
        "default_target".to_owned(),
        json!({ "kind": "unconfigured" }),
    );
    Ok(())
}

fn migrate_v2_node(node: &mut Value) -> Result<(), StateStoreError> {
    let object = node
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let protocol = required_string(object, "protocol")?;
    let credentials = object
        .remove("credentials")
        .ok_or(StateStoreError::InvalidStoredState)?;
    let options = v3_options(&protocol, credentials)?;
    object.insert(
        "protocol".to_owned(),
        Value::String(v3_protocol(&protocol)?.to_owned()),
    );
    object.insert("options".to_owned(), options);
    Ok(())
}

fn v3_protocol(protocol: &str) -> Result<&'static str, StateStoreError> {
    match protocol {
        "shadowsocks" => Ok("shadowsocks"),
        "vmess" => Ok("vmess"),
        "vless" => Ok("vless"),
        "trojan" => Ok("trojan"),
        "hysteria2" => Ok("hysteria2"),
        "tuic" => Ok("tuic"),
        "http" => Ok("http"),
        "any_tls" => Ok("any_tls"),
        "socks5" => Ok("socks"),
        "https" => Ok("http"),
        _ => Err(StateStoreError::InvalidStoredState),
    }
}

fn v3_options(protocol: &str, credentials: Value) -> Result<Value, StateStoreError> {
    let credentials = credentials
        .as_object()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let kind = required_string(credentials, "kind")?;
    let username = optional_string(credentials, "username")?;
    let password = optional_string(credentials, "password")?;
    let cipher = optional_string(credentials, "cipher")?;
    let uuid = optional_string(credentials, "uuid")?;
    let flow = optional_string(credentials, "flow")?;
    match protocol {
        "shadowsocks" if kind == "password" => Ok(json!({
            "kind": "shadowsocks",
            "method": cipher.ok_or(StateStoreError::InvalidStoredState)?,
            "password": password.ok_or(StateStoreError::InvalidStoredState)?,
        })),
        "vmess" if kind == "uuid" => Ok(json!({
            "kind": "vmess",
            "uuid": uuid.ok_or(StateStoreError::InvalidStoredState)?,
            "alter_id": null,
            "security": null,
        })),
        "vless" if kind == "uuid" => Ok(json!({
            "kind": "vless",
            "uuid": uuid.ok_or(StateStoreError::InvalidStoredState)?,
            "flow": flow,
        })),
        "trojan" | "hysteria2" | "any_tls" if kind == "password" => {
            let password = password.ok_or(StateStoreError::InvalidStoredState)?;
            let kind = match protocol {
                "trojan" => "trojan",
                "hysteria2" => "hysteria2",
                "any_tls" => "any_tls",
                _ => unreachable!("covered by outer match"),
            };
            let mut options = Map::new();
            options.insert("kind".to_owned(), Value::String(kind.to_owned()));
            options.insert("password".to_owned(), Value::String(password));
            if protocol == "hysteria2" {
                options.insert("obfs".to_owned(), Value::Null);
            }
            Ok(Value::Object(options))
        }
        "socks5" if kind == "none" || kind == "password" => Ok(json!({
            "kind": "socks",
            "version": 5,
            "username": username,
            "password": password,
        })),
        "http" | "https" if kind == "none" || kind == "password" => Ok(json!({
            "kind": "http",
            "username": username,
            "password": password,
            "tls": protocol == "https",
        })),
        _ => Err(StateStoreError::InvalidStoredState),
    }
}

fn required_string(object: &Map<String, Value>, key: &str) -> Result<String, StateStoreError> {
    optional_string(object, key)?.ok_or(StateStoreError::InvalidStoredState)
}

fn optional_string(
    object: &Map<String, Value>,
    key: &str,
) -> Result<Option<String>, StateStoreError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(Some(value.clone())),
        _ => Err(StateStoreError::InvalidStoredState),
    }
}
