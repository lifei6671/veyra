use serde_json::{Map, Value, json};

use super::StateStoreError;
use crate::domain::CURRENT_SCHEMA_VERSION;

pub fn migrate_to_current(mut document: Value) -> Result<(Value, bool), StateStoreError> {
    let mut version = document
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(StateStoreError::MissingSchemaVersion)?;
    let mut migrated = false;

    while version < u64::from(CURRENT_SCHEMA_VERSION) {
        match version {
            1 => migrate_v1_to_v2(&mut document),
            2 => migrate_v2_to_v3(&mut document),
            3 => migrate_v3_to_v4(&mut document),
            4 => migrate_v4_to_v5(&mut document),
            5 => migrate_v5_to_v6(&mut document),
            6 => migrate_v6_to_v7(&mut document),
            7 => migrate_v7_to_v8(&mut document),
            8 => migrate_v8_to_v9(&mut document),
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

fn migrate_v5_to_v6(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let subscriptions = object
        .get_mut("subscriptions")
        .and_then(Value::as_array_mut)
        .ok_or(StateStoreError::InvalidStoredState)?;
    for subscription in subscriptions {
        subscription
            .as_object_mut()
            .ok_or(StateStoreError::InvalidStoredState)?
            .insert("document".to_owned(), Value::Null);
    }
    object.insert("schema_version".to_owned(), json!(6));
    Ok(())
}

fn migrate_v4_to_v5(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let subscriptions = object
        .get_mut("subscriptions")
        .and_then(Value::as_array_mut)
        .ok_or(StateStoreError::InvalidStoredState)?;
    for subscription in subscriptions {
        let subscription = subscription
            .as_object_mut()
            .ok_or(StateStoreError::InvalidStoredState)?;
        let source_kind = subscription
            .get("source")
            .and_then(Value::as_object)
            .and_then(|source| source.get("kind"))
            .and_then(Value::as_str)
            .ok_or(StateStoreError::InvalidStoredState)?;
        let (remote_request, update_policy) = match source_kind {
            "remote" => (
                json!({
                    "user_agent": null,
                    "timeout_seconds": 30,
                    "proxy_mode": "direct",
                    "verify_tls": true
                }),
                json!({"allow_auto_update": true, "interval_minutes": null}),
            ),
            "manual" => (
                Value::Null,
                json!({"allow_auto_update": false, "interval_minutes": null}),
            ),
            _ => return Err(StateStoreError::InvalidStoredState),
        };
        subscription.insert("description".to_owned(), Value::String(String::new()));
        subscription.insert("last_attempt_at_ms".to_owned(), Value::Null);
        subscription.insert("remote_request".to_owned(), remote_request);
        subscription.insert("update_policy".to_owned(), update_policy);
    }
    object.insert("active_subscription_id".to_owned(), Value::Null);
    object.insert("active_configuration_generation".to_owned(), json!(0));
    object.insert("schema_version".to_owned(), json!(5));
    Ok(())
}

fn migrate_v3_to_v4(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let subscriptions = object
        .get_mut("subscriptions")
        .and_then(Value::as_array_mut)
        .ok_or(StateStoreError::InvalidStoredState)?;
    for subscription in subscriptions {
        let subscription = subscription
            .as_object_mut()
            .ok_or(StateStoreError::InvalidStoredState)?;
        subscription.insert("source".to_owned(), json!({ "kind": "manual" }));
        subscription.insert("last_success_at_ms".to_owned(), Value::Null);
        subscription.insert("http_metadata".to_owned(), Value::Null);
    }
    object.insert("schema_version".to_owned(), json!(4));
    Ok(())
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

fn migrate_v6_to_v7(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    // Legacy Veyra and OpenBox are separate sources. Preserve legacy facts, add an explicitly
    // defaulted supported profile; never reinterpret StoredStateV6 as an OpenBox Profile.
    object.insert(
        "state_epoch".into(),
        serde_json::to_value(
            crate::domain::StateEpoch::fresh().map_err(|_| StateStoreError::WriteFailed)?,
        )
        .map_err(|_| StateStoreError::SerializationFailed)?,
    );
    object.insert("config_revision".into(), json!(0));
    object.insert("selection_revision".into(), json!(0));
    object.insert(
        "profile".into(),
        serde_json::to_value(crate::domain::Profile::default())
            .map_err(|_| StateStoreError::SerializationFailed)?,
    );
    object.insert(
        "app_config".into(),
        serde_json::to_value(crate::domain::AppConfig::default())
            .map_err(|_| StateStoreError::SerializationFailed)?,
    );
    object.insert("schema_version".into(), json!(7));
    Ok(())
}

fn migrate_v7_to_v8(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let config = object
        .get_mut("app_config")
        .and_then(Value::as_object_mut)
        .ok_or(StateStoreError::InvalidStoredState)?;
    config.insert(
        "visual".into(),
        serde_json::to_value(crate::domain::DesktopVisualPreferences::default())
            .map_err(|_| StateStoreError::SerializationFailed)?,
    );
    object.insert("schema_version".into(), json!(8));
    Ok(())
}

#[cfg(test)]
mod visual_tests {
    use super::*;
    #[test]
    fn v7_to_v8_preserves_identity_versions_and_config() {
        let mut state = crate::domain::AppState::empty();
        state.config_revision = 17;
        state.selection_revision = 4;
        let mut old = serde_json::to_value(&state).unwrap();
        old["schema_version"] = json!(7);
        old["app_config"].as_object_mut().unwrap().remove("visual");
        old["app_config"]["check_updates_on_start"] = json!(false);
        let (value, changed) = migrate_to_current(old).unwrap();
        assert!(changed);
        let migrated: crate::domain::AppState = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(migrated.state_epoch, state.state_epoch);
        assert_eq!(
            (migrated.config_revision, migrated.selection_revision),
            (17, 4)
        );
        assert_eq!(migrated.app_config.visual, Default::default());
        assert!(!migrated.app_config.check_updates_on_start);
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(migrated.validate().is_ok());
        assert!(!migrate_to_current(value).unwrap().1);
    }
}

fn migrate_v8_to_v9(document: &mut Value) -> Result<(), StateStoreError> {
    let object = document
        .as_object_mut()
        .ok_or(StateStoreError::InvalidStoredState)?;
    let config = object
        .get_mut("app_config")
        .and_then(Value::as_object_mut)
        .ok_or(StateStoreError::InvalidStoredState)?;
    config.insert(
        "behavior".into(),
        serde_json::to_value(crate::domain::DesktopBehaviorPreferences::default())
            .map_err(|_| StateStoreError::SerializationFailed)?,
    );
    object.insert("schema_version".into(), json!(9));
    Ok(())
}

#[cfg(test)]
mod behavior_tests {
    use super::*;
    #[test]
    fn v8_to_v9_preserves_all_facts_and_load_is_stable() {
        let mut original = crate::domain::AppState::empty();
        original.config_revision = 19;
        original.selection_revision = 7;
        original.app_config.visual.theme_mode = crate::domain::DesktopThemeMode::Dark;
        original.app_config.visual.global_radius = 5;
        original.app_config.check_updates_on_start = false;
        original.profile.ipv6 = false;
        let mut old = serde_json::to_value(&original).unwrap();
        old["schema_version"] = json!(8);
        old["app_config"]
            .as_object_mut()
            .unwrap()
            .remove("behavior");
        let (new, changed) = migrate_to_current(old).unwrap();
        assert!(changed);
        assert_eq!(
            serde_json::from_value::<crate::domain::AppState>(new.clone()).unwrap(),
            original
        );
        assert!(!migrate_to_current(new).unwrap().1);
    }
}

#[cfg(test)]
mod behavior_store_tests {
    use crate::{
        application::{state_access::StateAccessGate, state_service::SnapshotService},
        domain::*,
        storage::JsonStateStore,
    };
    #[test]
    fn actual_v8_store_migrates_once_without_business_edit() {
        let mut state = AppState::empty();
        state.config_revision = 12;
        state.selection_revision = 3;
        state.app_config.visual.theme_mode = DesktopThemeMode::Dark;
        let root = std::env::temp_dir().join(format!("veyra-v8-migration-{:?}", state.state_epoch));
        std::fs::create_dir_all(&root).unwrap();
        let mut old = serde_json::to_value(&state).unwrap();
        old["schema_version"] = 8.into();
        old["app_config"]
            .as_object_mut()
            .unwrap()
            .remove("behavior");
        std::fs::write(root.join("state.json"), serde_json::to_vec(&old).unwrap()).unwrap();
        let service = SnapshotService::new(
            JsonStateStore::new(root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        );
        assert_eq!(service.snapshot().unwrap(), state);
        assert_eq!(service.snapshot().unwrap(), state);
        let disk: AppState =
            serde_json::from_slice(&std::fs::read(root.join("state.json")).unwrap()).unwrap();
        assert_eq!(disk, state);
        assert!(root.join("state.json.pre-migration").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
