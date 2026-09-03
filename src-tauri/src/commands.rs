use serde::Serialize;

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    application: &'static str,
    status: &'static str,
}

#[tauri::command]
pub fn bootstrap_status() -> BootstrapStatus {
    BootstrapStatus {
        application: "Veyra",
        status: "ready",
    }
}

#[cfg(test)]
mod tests {
    use super::bootstrap_status;

    #[test]
    fn returns_only_fixed_bootstrap_information() {
        assert_eq!(
            bootstrap_status(),
            super::BootstrapStatus {
                application: "Veyra",
                status: "ready",
            }
        );
    }
}
