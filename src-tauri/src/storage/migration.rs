use serde_json::{json, Value};

use super::StateStoreError;
use crate::domain::CURRENT_SCHEMA_VERSION;

pub(crate) fn migrate_to_current(mut document: Value) -> Result<(Value, bool), StateStoreError> {
    let mut version = document
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or(StateStoreError::MissingSchemaVersion)?;

    let mut migrated = false;
    while version < u64::from(CURRENT_SCHEMA_VERSION) {
        let object = document
            .as_object_mut()
            .ok_or(StateStoreError::InvalidStoredState)?;
        match version {
            0 => {
                object.insert("schema_version".to_owned(), json!(1));
                version = 1;
            }
            1 => {
                object.insert("schema_version".to_owned(), json!(CURRENT_SCHEMA_VERSION));
                object.insert("pools".to_owned(), json!([]));
                object.insert("routes".to_owned(), json!([]));
                version = u64::from(CURRENT_SCHEMA_VERSION);
            }
            _ => return Err(StateStoreError::UnsupportedSchemaVersion),
        }
        migrated = true;
    }
    if version == u64::from(CURRENT_SCHEMA_VERSION) {
        Ok((document, migrated))
    } else {
        Err(StateStoreError::UnsupportedSchemaVersion)
    }
}
