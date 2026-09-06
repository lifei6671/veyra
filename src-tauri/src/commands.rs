use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::application::managed_observation_runtime::{
    ActivationError, ActivationResult, DocumentApplyError, DocumentApplyResult, DocumentSaveError,
    DocumentSaveResult, ManagedObservationRuntimeController, ManagedRuntimeStartResult,
    ManagedRuntimeStopResult, RunningConfigurationError, RunningConfigurationResult,
};
use crate::application::observability::{
    InMemoryRuntimeObservations, ObservationLogCategory, ObservationLogLevel, ObservationSource,
    ObservedCaptureMode, ObservedSidecarLifecycle, RuntimeObservationDelta, RuntimeObservationPort,
    RuntimeObservationSnapshot, SubscriptionSwitch, TrafficHistoryPoint,
};
use crate::application::proxy_routing::{
    MutateProxyRoutingRequest, MutationResult, ProxyRoutingManager, SnapshotResult,
};
use crate::application::subscription_management::{
    DocumentOperationError, DocumentSaveInput, EditSubscription, ImportRequestSource,
    RemoteImportOptions, RemoteRequestPatch, SubscriptionImport, SubscriptionManager,
    SubscriptionOperationError, SubscriptionSettings, SubscriptionSourceKind, SubscriptionSummary,
    UpdatePolicyPatch, UpdateResult, UpdateRouteOverride, UserAgentEdit,
};

const MAIN_WINDOW_LABEL: &str = "main";
const RUNTIME_OBSERVATION_DELTA_EVENT: &str = "runtime_observation_delta";

#[tauri::command]
pub(crate) fn get_proxy_routing_snapshot(
    routing: State<'_, Arc<ProxyRoutingManager>>,
) -> SnapshotResult {
    routing.snapshot()
}

#[tauri::command]
pub(crate) async fn mutate_proxy_routing(
    request: MutateProxyRoutingRequest,
    routing: State<'_, Arc<ProxyRoutingManager>>,
) -> Result<MutationResult, ()> {
    let manager = Arc::clone(routing.inner());
    let join_failure = Arc::clone(&manager);
    Ok(
        tauri::async_runtime::spawn_blocking(move || manager.mutate(request))
            .await
            .unwrap_or_else(|_| join_failure.execution_unavailable()),
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivateSubscriptionRequest {
    id: String,
    #[serde(default)]
    force: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct FormatDocumentRequest {
    content: String,
    format: crate::domain::SubscriptionDocumentFormat,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveDocumentRequest {
    id: String,
    content: String,
    format: crate::domain::SubscriptionDocumentFormat,
    expected_revision: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DocumentResponse {
    id: String,
    content: String,
    format: crate::domain::SubscriptionDocumentFormat,
    revision: String,
    source_kind: &'static str,
    local_override: bool,
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ReadDocumentResponse {
    Ok { document: DocumentResponse },
    Error { error: DocumentOperationError },
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum FormatDocumentResponse {
    Ok {
        content: String,
    },
    Error {
        error: crate::subscription::DocumentErrorCode,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<crate::subscription::DocumentErrorLocation>,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum DocumentApplyResponse {
    NotRequired,
    Ready {
        operation_id: String,
        generation: u64,
    },
    Failed {
        operation_id: String,
        generation: u64,
        error: DocumentApplyError,
    },
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SaveDocumentResponse {
    Ok {
        subscription: Box<SubscriptionSummaryResponse>,
        document_revision: String,
        apply: DocumentApplyResponse,
    },
    Error {
        #[serde(skip_serializing_if = "Option::is_none")]
        operation_id: Option<String>,
        error: DocumentSaveError,
    },
}

impl From<DocumentSaveResult> for SaveDocumentResponse {
    fn from(value: DocumentSaveResult) -> Self {
        match value {
            DocumentSaveResult::Ok {
                subscription,
                document_revision,
                apply,
            } => Self::Ok {
                subscription: Box::new((*subscription).into()),
                document_revision,
                apply: match apply {
                    DocumentApplyResult::NotRequired => DocumentApplyResponse::NotRequired,
                    DocumentApplyResult::Ready {
                        operation_id,
                        generation,
                    } => DocumentApplyResponse::Ready {
                        operation_id,
                        generation,
                    },
                    DocumentApplyResult::Failed {
                        operation_id,
                        generation,
                        error,
                    } => DocumentApplyResponse::Failed {
                        operation_id,
                        generation,
                        error,
                    },
                },
            },
            DocumentSaveResult::Error {
                operation_id,
                error,
            } => Self::Error {
                operation_id,
                error,
            },
        }
    }
}

#[derive(Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum RunningConfigurationResponse {
    Ok {
        format: &'static str,
        content: String,
        applied_subscription_id: String,
        applied_configuration_generation: u64,
    },
    Error {
        error: RunningConfigurationError,
    },
}

// 原始配置只在主窗口显式调用时返回，不进入普通摘要、日志或观测事件。
#[tauri::command]
pub(crate) fn get_subscription_document(
    window: tauri::WebviewWindow,
    request: SubscriptionIdRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> Result<ReadDocumentResponse, ()> {
    if window.label() != MAIN_WINDOW_LABEL {
        return Err(());
    }
    Ok(match subscriptions.read_document(&request.id) {
        Ok(value) => ReadDocumentResponse::Ok {
            document: DocumentResponse {
                id: value.id,
                content: value.content,
                format: value.format,
                revision: value.revision,
                source_kind: match value.source_kind {
                    SubscriptionSourceKind::Remote => "remote",
                    SubscriptionSourceKind::Manual => "manual",
                },
                local_override: value.local_override,
            },
        },
        Err(error) => ReadDocumentResponse::Error { error },
    })
}

#[tauri::command]
pub(crate) async fn format_subscription_document(
    window: tauri::WebviewWindow,
    request: FormatDocumentRequest,
) -> Result<FormatDocumentResponse, ()> {
    if window.label() != MAIN_WINDOW_LABEL {
        return Err(());
    }
    tauri::async_runtime::spawn_blocking(move || {
        match crate::subscription::format_document(request.content, request.format) {
            Ok(content) => FormatDocumentResponse::Ok { content },
            Err(error) => FormatDocumentResponse::Error {
                error: error.code,
                location: error.location,
            },
        }
    })
    .await
    .map_err(|_| ())
}

#[tauri::command]
pub(crate) async fn save_subscription_document(
    window: tauri::WebviewWindow,
    request: SaveDocumentRequest,
    runtime: State<'_, Arc<ManagedObservationRuntimeController>>,
) -> Result<SaveDocumentResponse, ()> {
    if window.label() != MAIN_WINDOW_LABEL {
        return Err(());
    }
    let runtime = Arc::clone(runtime.inner());
    // 等待 worker 的明确保存终态，不能在落盘后用通用超时伪装成未保存。
    tauri::async_runtime::spawn_blocking(move || {
        runtime
            .save_document(DocumentSaveInput {
                id: request.id,
                content: request.content,
                format: request.format,
                expected_revision: request.expected_revision,
            })
            .into()
    })
    .await
    .map_err(|_| ())
}

#[tauri::command]
pub(crate) async fn get_running_configuration(
    window: tauri::WebviewWindow,
    runtime: State<'_, Arc<ManagedObservationRuntimeController>>,
) -> Result<RunningConfigurationResponse, ()> {
    if window.label() != MAIN_WINDOW_LABEL {
        return Err(());
    }
    let runtime = Arc::clone(runtime.inner());
    tauri::async_runtime::spawn_blocking(move || match runtime.running_configuration() {
        RunningConfigurationResult::Ok {
            content,
            applied_subscription_id,
            applied_configuration_generation,
        } => RunningConfigurationResponse::Ok {
            format: "json",
            content,
            applied_subscription_id,
            applied_configuration_generation,
        },
        RunningConfigurationResult::Error(error) => RunningConfigurationResponse::Error { error },
    })
    .await
    .map_err(|_| ())
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SubscriptionErrorCode {
    InvalidInput,
    Busy,
    FetchFailed,
    ParseFailed,
    NormalizationFailed,
    ValidationFailed,
    SaveFailed,
    StateUnavailable,
    NotFound,
    CacheUnavailable,
    IdentityFailed,
    InvalidOptions,
    UnsupportedClashProviders,
    ProxyUnavailable,
    SystemProxyUnavailable,
    ReferenceConflict,
    NotShareable,
}

impl From<SubscriptionOperationError> for SubscriptionErrorCode {
    fn from(error: SubscriptionOperationError) -> Self {
        match error {
            SubscriptionOperationError::InvalidInput => Self::InvalidInput,
            SubscriptionOperationError::Busy => Self::Busy,
            SubscriptionOperationError::FetchFailed => Self::FetchFailed,
            SubscriptionOperationError::ParseFailed => Self::ParseFailed,
            SubscriptionOperationError::NormalizationFailed => Self::NormalizationFailed,
            SubscriptionOperationError::ValidationFailed => Self::ValidationFailed,
            SubscriptionOperationError::SaveFailed => Self::SaveFailed,
            SubscriptionOperationError::StateUnavailable => Self::StateUnavailable,
            SubscriptionOperationError::NotFound => Self::NotFound,
            SubscriptionOperationError::CacheUnavailable => Self::CacheUnavailable,
            SubscriptionOperationError::IdentityFailed => Self::IdentityFailed,
            SubscriptionOperationError::InvalidOptions => Self::InvalidOptions,
            SubscriptionOperationError::UnsupportedClashProviders => {
                Self::UnsupportedClashProviders
            }
            SubscriptionOperationError::ProxyUnavailable => Self::ProxyUnavailable,
            SubscriptionOperationError::SystemProxyUnavailable => Self::SystemProxyUnavailable,
            SubscriptionOperationError::ReferenceConflict => Self::ReferenceConflict,
            SubscriptionOperationError::NotShareable => Self::NotShareable,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubscriptionSummaryResponse {
    id: String,
    name: String,
    description: String,
    allow_auto_update: bool,
    update_interval_minutes: Option<u32>,
    active: bool,
    active_configuration_generation: Option<u64>,
    shareable: bool,
    source_kind: &'static str,
    node_count: usize,
    skipped_node_count: u32,
    last_success_at_ms: Option<u64>,
    traffic: Option<SubscriptionTrafficResponse>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubscriptionTrafficResponse {
    upload: Option<u64>,
    download: Option<u64>,
    total: Option<u64>,
    expire_at_ms: Option<u64>,
}

impl From<SubscriptionSummary> for SubscriptionSummaryResponse {
    fn from(summary: SubscriptionSummary) -> Self {
        Self {
            id: summary.id,
            name: summary.name,
            description: summary.description,
            allow_auto_update: summary.allow_auto_update,
            update_interval_minutes: summary.update_interval_minutes,
            active: summary.active,
            active_configuration_generation: summary.active_configuration_generation,
            shareable: summary.shareable,
            source_kind: match summary.source_kind {
                SubscriptionSourceKind::Remote => "remote",
                SubscriptionSourceKind::Manual => "manual",
            },
            node_count: summary.node_count,
            skipped_node_count: summary.skipped_node_count,
            last_success_at_ms: summary.last_success_at_ms,
            traffic: summary.traffic.map(|traffic| SubscriptionTrafficResponse {
                upload: traffic.upload,
                download: traffic.download,
                total: traffic.total,
                expire_at_ms: traffic.expire_at_ms,
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ListSubscriptionsResponse {
    Ok {
        subscriptions: Vec<SubscriptionSummaryResponse>,
    },
    Error {
        error: SubscriptionErrorCode,
    },
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ImportSubscriptionResponse {
    Ok {
        subscription: Box<SubscriptionSummaryResponse>,
    },
    Error {
        error: SubscriptionErrorCode,
    },
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum UpdateSubscriptionResponse {
    Ok {
        subscription: Box<SubscriptionSummaryResponse>,
        content_changed: bool,
    },
    Error {
        error: SubscriptionErrorCode,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct ImportSubscriptionRequest {
    name: String,
    #[serde(default)]
    description: String,
    source: ImportSubscriptionSourceRequest,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields, rename_all = "camelCase")]
enum ImportSubscriptionSourceRequest {
    Remote {
        url: String,
        #[serde(default)]
        options: RemoteImportOptionsRequest,
    },
    Manual {
        content: String,
    },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
enum ProxyModeRequest {
    #[default]
    Direct,
    System,
    ManagedCore,
}
impl From<ProxyModeRequest> for crate::domain::SubscriptionProxyMode {
    fn from(mode: ProxyModeRequest) -> Self {
        match mode {
            ProxyModeRequest::Direct => Self::Direct,
            ProxyModeRequest::System => Self::System,
            ProxyModeRequest::ManagedCore => Self::ManagedCore,
        }
    }
}
impl From<crate::domain::SubscriptionProxyMode> for ProxyModeRequest {
    fn from(mode: crate::domain::SubscriptionProxyMode) -> Self {
        match mode {
            crate::domain::SubscriptionProxyMode::Direct => Self::Direct,
            crate::domain::SubscriptionProxyMode::System => Self::System,
            crate::domain::SubscriptionProxyMode::ManagedCore => Self::ManagedCore,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct RemoteImportOptionsRequest {
    user_agent: Option<String>,
    timeout_seconds: u16,
    proxy_mode: ProxyModeRequest,
    verify_tls: bool,
    allow_auto_update: bool,
    interval_minutes: Option<u32>,
}
impl Default for RemoteImportOptionsRequest {
    fn default() -> Self {
        Self {
            user_agent: None,
            timeout_seconds: 30,
            proxy_mode: ProxyModeRequest::Direct,
            verify_tls: true,
            allow_auto_update: true,
            interval_minutes: None,
        }
    }
}
impl From<RemoteImportOptionsRequest> for RemoteImportOptions {
    fn from(options: RemoteImportOptionsRequest) -> Self {
        Self {
            user_agent: options.user_agent,
            timeout_seconds: options.timeout_seconds,
            proxy_mode: options.proxy_mode.into(),
            verify_tls: options.verify_tls,
            allow_auto_update: options.allow_auto_update,
            interval_minutes: options.interval_minutes,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct UpdateSubscriptionRequest {
    id: String,
    content: Option<String>,
    route_override: Option<RouteOverrideRequest>,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
enum RouteOverrideRequest {
    ManagedCore,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct SubscriptionIdRequest {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct EditSubscriptionRequest {
    id: String,
    name: Option<String>,
    description: Option<String>,
    url_replacement: Option<String>,
    remote_request: Option<RemoteRequestPatchRequest>,
    update_policy: Option<UpdatePolicyPatchRequest>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct RemoteRequestPatchRequest {
    #[serde(default, deserialize_with = "decode_optional_string")]
    user_agent: Option<String>,
    timeout_seconds: Option<u16>,
    proxy_mode: Option<ProxyModeRequest>,
    verify_tls: Option<bool>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct UpdatePolicyPatchRequest {
    allow_auto_update: Option<bool>,
    #[serde(default, deserialize_with = "decode_nullable_interval")]
    interval_minutes: Option<Option<u32>>,
}
fn decode_optional_string<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(decoder).map(Some)
}
fn decode_nullable_interval<'de, D: serde::Deserializer<'de>>(
    decoder: D,
) -> Result<Option<Option<u32>>, D::Error> {
    Option::<u32>::deserialize(decoder).map(Some)
}
impl From<EditSubscriptionRequest> for EditSubscription {
    fn from(request: EditSubscriptionRequest) -> Self {
        Self {
            id: request.id,
            name: request.name,
            description: request.description,
            url_replacement: request.url_replacement,
            remote_request: request.remote_request.map(|patch| RemoteRequestPatch {
                user_agent: patch.user_agent.map(|value| {
                    if value.is_empty() {
                        UserAgentEdit::Clear
                    } else {
                        UserAgentEdit::Set(value)
                    }
                }),
                timeout_seconds: patch.timeout_seconds,
                proxy_mode: patch.proxy_mode.map(Into::into),
                verify_tls: patch.verify_tls,
            }),
            update_policy: request.update_policy.map(|patch| UpdatePolicyPatch {
                allow_auto_update: patch.allow_auto_update,
                interval_minutes: patch.interval_minutes,
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubscriptionSettingsResponse {
    id: String,
    name: String,
    description: String,
    source_kind: &'static str,
    url_preview: Option<String>,
    has_custom_user_agent: bool,
    remote_request: Option<RemoteRequestSettingsResponse>,
    update_policy: UpdatePolicySettingsResponse,
    shareable: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteRequestSettingsResponse {
    timeout_seconds: u16,
    proxy_mode: ProxyModeRequest,
    verify_tls: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdatePolicySettingsResponse {
    allow_auto_update: bool,
    interval_minutes: Option<u32>,
}
impl From<SubscriptionSettings> for SubscriptionSettingsResponse {
    fn from(settings: SubscriptionSettings) -> Self {
        Self {
            id: settings.id,
            name: settings.name,
            description: settings.description,
            source_kind: match settings.source_kind {
                SubscriptionSourceKind::Remote => "remote",
                SubscriptionSourceKind::Manual => "manual",
            },
            url_preview: settings.url_preview,
            has_custom_user_agent: settings.has_custom_user_agent,
            remote_request: settings
                .remote_request
                .map(|options| RemoteRequestSettingsResponse {
                    timeout_seconds: options.timeout_seconds,
                    proxy_mode: options.proxy_mode.into(),
                    verify_tls: options.verify_tls,
                }),
            update_policy: UpdatePolicySettingsResponse {
                allow_auto_update: settings.update_policy.allow_auto_update,
                interval_minutes: settings.update_policy.interval_minutes,
            },
            shareable: settings.shareable,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SettingsResponse {
    Ok {
        settings: SubscriptionSettingsResponse,
    },
    Error {
        error: SubscriptionErrorCode,
    },
}
#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ShareResponse {
    Ok { url: String },
    Error { error: SubscriptionErrorCode },
}
#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum DeleteResponse {
    Ok { deleted_id: String },
    Error { error: SubscriptionErrorCode },
}
#[derive(Debug, Serialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ActivateSubscriptionResponse {
    Ok {
        outcome: &'static str,
        operation_id: String,
        subscription: Box<SubscriptionSummaryResponse>,
        active_configuration_generation: u64,
    },
    Pending {
        operation_id: String,
    },
    Error {
        operation_id: Option<String>,
        error: ActivationError,
    },
}
impl From<ActivationResult> for ActivateSubscriptionResponse {
    fn from(result: ActivationResult) -> Self {
        match result {
            ActivationResult::Activated {
                operation_id,
                subscription,
                generation,
            } => Self::Ok {
                outcome: "activated",
                operation_id,
                subscription: Box::new(subscription.into()),
                active_configuration_generation: generation,
            },
            ActivationResult::Reactivated {
                operation_id,
                subscription,
                generation,
            } => Self::Ok {
                outcome: "reactivated",
                operation_id,
                subscription: Box::new(subscription.into()),
                active_configuration_generation: generation,
            },
            ActivationResult::AlreadyCurrent {
                operation_id,
                subscription,
                generation,
            } => Self::Ok {
                outcome: "alreadyCurrent",
                operation_id,
                subscription: Box::new(subscription.into()),
                active_configuration_generation: generation,
            },
            ActivationResult::Pending { operation_id } => Self::Pending { operation_id },
            ActivationResult::Error {
                operation_id,
                error,
            } => Self::Error {
                operation_id,
                error,
            },
        }
    }
}

#[tauri::command]
pub(crate) fn list_subscriptions(
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> ListSubscriptionsResponse {
    match subscriptions.list() {
        Ok(values) => ListSubscriptionsResponse::Ok {
            subscriptions: values.into_iter().map(Into::into).collect(),
        },
        Err(error) => ListSubscriptionsResponse::Error {
            error: error.into(),
        },
    }
}
#[tauri::command]
pub(crate) async fn import_subscription(
    request: ImportSubscriptionRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> Result<ImportSubscriptionResponse, ()> {
    let source = match request.source {
        ImportSubscriptionSourceRequest::Remote { url, options } => ImportRequestSource::Remote {
            url,
            options: options.into(),
        },
        ImportSubscriptionSourceRequest::Manual { content } => {
            ImportRequestSource::Manual { content }
        }
    };
    Ok(
        match subscriptions
            .import_with_options(SubscriptionImport {
                name: request.name,
                description: request.description,
                source,
            })
            .await
        {
            Ok(subscription) => ImportSubscriptionResponse::Ok {
                subscription: Box::new(subscription.into()),
            },
            Err(error) => ImportSubscriptionResponse::Error {
                error: error.into(),
            },
        },
    )
}
fn update_response(
    result: Result<UpdateResult, SubscriptionOperationError>,
) -> UpdateSubscriptionResponse {
    match result {
        Ok(UpdateResult {
            subscription,
            content_changed,
        }) => UpdateSubscriptionResponse::Ok {
            subscription: Box::new(subscription.into()),
            content_changed,
        },
        Err(error) => UpdateSubscriptionResponse::Error {
            error: error.into(),
        },
    }
}
#[tauri::command]
pub(crate) async fn update_subscription(
    request: UpdateSubscriptionRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> Result<UpdateSubscriptionResponse, ()> {
    Ok(update_response(
        subscriptions
            .update_with_route(
                request.id,
                request.content,
                request
                    .route_override
                    .map(|_| UpdateRouteOverride::ManagedCore),
            )
            .await,
    ))
}
#[tauri::command]
pub(crate) fn get_subscription_settings(
    request: SubscriptionIdRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> SettingsResponse {
    match subscriptions.get_settings(&request.id) {
        Ok(settings) => SettingsResponse::Ok {
            settings: settings.into(),
        },
        Err(error) => SettingsResponse::Error {
            error: error.into(),
        },
    }
}
#[tauri::command]
pub(crate) async fn edit_subscription(
    request: EditSubscriptionRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> Result<UpdateSubscriptionResponse, ()> {
    Ok(update_response(subscriptions.edit(request.into()).await))
}
#[tauri::command]
pub(crate) fn get_subscription_share_url(
    request: SubscriptionIdRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
) -> ShareResponse {
    match subscriptions.share_url(&request.id) {
        Ok(url) => ShareResponse::Ok { url },
        Err(error) => ShareResponse::Error {
            error: error.into(),
        },
    }
}
#[tauri::command]
pub(crate) fn delete_subscription(
    request: SubscriptionIdRequest,
    subscriptions: State<'_, Arc<SubscriptionManager>>,
    runtime: State<'_, Arc<ManagedObservationRuntimeController>>,
) -> DeleteResponse {
    match runtime.with_runtime_state(|stopped| subscriptions.delete(request.id, stopped)) {
        Ok(Ok(deleted_id)) => DeleteResponse::Ok { deleted_id },
        Ok(Err(error)) => DeleteResponse::Error {
            error: error.into(),
        },
        Err(_) => DeleteResponse::Error {
            error: SubscriptionErrorCode::Busy,
        },
    }
}
#[tauri::command]
pub(crate) async fn activate_subscription(
    request: ActivateSubscriptionRequest,
    runtime: State<'_, Arc<ManagedObservationRuntimeController>>,
) -> Result<ActivateSubscriptionResponse, ()> {
    let runtime = Arc::clone(runtime.inner());
    // worker 自有 operation ID；Join 失败拒绝 invoke，不制造一个没有身份的业务终态。
    tauri::async_runtime::spawn_blocking(move || runtime.activate(request.id, request.force).into())
        .await
        .map_err(|_| ())
}

/// 仅返回封闭、脱敏且带明确来源标记的内存观测快照。
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeObservationResponse {
    source: &'static str,
    revision: u64,
    observed_at_ms: u64,
    traffic_history: Vec<TrafficHistoryPointResponse>,
    capture_mode: &'static str,
    sidecar_lifecycle: &'static str,
    applied_subscription_id: Option<String>,
    applied_configuration_generation: Option<u64>,
    subscription_switch: Option<SubscriptionSwitch>,
    managed_proxy_available: bool,
    core_memory_bytes: Option<u64>,
    upload_rate_bps: u64,
    download_rate_bps: u64,
    upload_total_bytes: u64,
    download_total_bytes: u64,
    connection_count: u32,
    log_summary: Vec<ObservationLogSummaryResponse>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrafficHistoryPointResponse {
    sampled_at_ms: u64,
    upload_rate_bps: u64,
    download_rate_bps: u64,
}

impl From<TrafficHistoryPoint> for TrafficHistoryPointResponse {
    fn from(point: TrafficHistoryPoint) -> Self {
        Self {
            sampled_at_ms: point.sampled_at_ms,
            upload_rate_bps: point.upload_rate_bps,
            download_rate_bps: point.download_rate_bps,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationLogSummaryResponse {
    category: &'static str,
    level: &'static str,
    occurrences: u32,
}

impl From<RuntimeObservationSnapshot> for RuntimeObservationResponse {
    fn from(snapshot: RuntimeObservationSnapshot) -> Self {
        let log_summary = snapshot
            .latest_log
            .as_ref()
            .map(|summary| ObservationLogSummaryResponse {
                category: map_log_category(summary.category),
                level: map_log_level(summary.level),
                occurrences: 1,
            })
            .into_iter()
            .collect();

        Self {
            source: map_source(snapshot.source),
            revision: snapshot.revision,
            observed_at_ms: snapshot.observed_at_ms,
            traffic_history: snapshot
                .traffic_history
                .into_iter()
                .map(Into::into)
                .collect(),
            capture_mode: map_capture_mode(snapshot.capture_mode),
            sidecar_lifecycle: map_sidecar_lifecycle(snapshot.sidecar_lifecycle),
            applied_subscription_id: snapshot.applied_subscription_id,
            applied_configuration_generation: snapshot.applied_configuration_generation,
            subscription_switch: snapshot.subscription_switch,
            managed_proxy_available: snapshot.managed_proxy_available,
            core_memory_bytes: snapshot.core_memory_bytes,
            upload_rate_bps: snapshot.traffic.upload_bytes_per_second,
            download_rate_bps: snapshot.traffic.download_bytes_per_second,
            upload_total_bytes: snapshot.traffic.upload_total_bytes,
            download_total_bytes: snapshot.traffic.download_total_bytes,
            connection_count: snapshot.connections.active,
            log_summary,
        }
    }
}

impl From<RuntimeObservationDelta> for RuntimeObservationResponse {
    fn from(delta: RuntimeObservationDelta) -> Self {
        let log_summary = delta
            .latest_log
            .as_ref()
            .map(|summary| ObservationLogSummaryResponse {
                category: map_log_category(summary.category),
                level: map_log_level(summary.level),
                occurrences: 1,
            })
            .into_iter()
            .collect();

        Self {
            source: map_source(delta.source),
            revision: delta.revision,
            observed_at_ms: delta.observed_at_ms,
            traffic_history: delta.traffic_history.into_iter().map(Into::into).collect(),
            capture_mode: map_capture_mode(delta.capture_mode),
            sidecar_lifecycle: map_sidecar_lifecycle(delta.sidecar_lifecycle),
            applied_subscription_id: delta.applied_subscription_id,
            applied_configuration_generation: delta.applied_configuration_generation,
            subscription_switch: delta.subscription_switch,
            managed_proxy_available: delta.managed_proxy_available,
            core_memory_bytes: delta.core_memory_bytes,
            upload_rate_bps: delta.traffic.upload_bytes_per_second,
            download_rate_bps: delta.traffic.download_bytes_per_second,
            upload_total_bytes: delta.traffic.upload_total_bytes,
            download_total_bytes: delta.traffic.download_total_bytes,
            connection_count: delta.connections.active,
            log_summary,
        }
    }
}

/// 固定、无参数的只读 IPC 入口；不接受 endpoint、命令、路径或 secret。
#[tauri::command]
pub(crate) fn runtime_observation_snapshot(
    observations: State<'_, InMemoryRuntimeObservations>,
) -> RuntimeObservationResponse {
    observations.snapshot().into()
}

/// 固定、零参数的受管观测运行时启动入口；配置、路径、端口和 secret 都由后端拥有。
#[tauri::command]
pub(crate) async fn start_managed_observation_runtime(
    runtime: State<'_, Arc<ManagedObservationRuntimeController>>,
) -> Result<ManagedRuntimeStartResult, ()> {
    let runtime = Arc::clone(runtime.inner());
    Ok(
        tauri::async_runtime::spawn_blocking(move || runtime.start())
            .await
            .unwrap_or(ManagedRuntimeStartResult::StartFailed),
    )
}

/// 固定、零参数的受管观测运行时停止入口；只处理当前 controller 所拥有的 child。
#[tauri::command]
pub(crate) async fn stop_managed_observation_runtime(
    runtime: State<'_, Arc<ManagedObservationRuntimeController>>,
) -> Result<ManagedRuntimeStopResult, ()> {
    let runtime = Arc::clone(runtime.inner());
    Ok(tauri::async_runtime::spawn_blocking(move || runtime.stop())
        .await
        .unwrap_or(ManagedRuntimeStopResult::StopFailed))
}

/// 主窗口恢复失败时仅回传封闭错误类别，不泄露平台细节或路径。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShowMainWindowError {
    MainWindowUnavailable,
    WindowOperationFailed,
    ObservationNotificationFailed,
}

/// 固定、无参数的主窗口恢复命令；只恢复已有窗口并发出当前安全快照。
#[tauri::command]
pub(crate) fn show_main_window(
    app: AppHandle,
    observations: State<'_, InMemoryRuntimeObservations>,
) -> Result<(), ShowMainWindowError> {
    restore_main_window(&app, &observations)
}

pub(crate) fn restore_main_window(
    app: &AppHandle,
    observations: &InMemoryRuntimeObservations,
) -> Result<(), ShowMainWindowError> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or(ShowMainWindowError::MainWindowUnavailable)?;
    window
        .show()
        .and_then(|_| window.set_focus())
        .map_err(|_| ShowMainWindowError::WindowOperationFailed)?;
    app.state::<crate::MainWindowVisibility>().mark_visible();
    app.emit_to(
        MAIN_WINDOW_LABEL,
        RUNTIME_OBSERVATION_DELTA_EVENT,
        RuntimeObservationResponse::from(observations.snapshot()),
    )
    .map_err(|_| ShowMainWindowError::ObservationNotificationFailed)
}

fn map_source(source: ObservationSource) -> &'static str {
    match source {
        ObservationSource::MockOnly => "mock",
        ObservationSource::ManagedSidecar => "runtime",
    }
}

fn map_capture_mode(mode: ObservedCaptureMode) -> &'static str {
    match mode {
        ObservedCaptureMode::NotObserved => "off",
        ObservedCaptureMode::Off => "off",
        ObservedCaptureMode::SystemProxy => "systemProxy",
        ObservedCaptureMode::RecoveryRequired => "recoveryRequired",
    }
}

fn map_sidecar_lifecycle(lifecycle: ObservedSidecarLifecycle) -> &'static str {
    match lifecycle {
        ObservedSidecarLifecycle::NotObserved => "notAttached",
        ObservedSidecarLifecycle::Stopped => "stopped",
        ObservedSidecarLifecycle::Ready => "ready",
        ObservedSidecarLifecycle::RecoveryRequired => "recoveryRequired",
    }
}

fn map_log_category(category: ObservationLogCategory) -> &'static str {
    match category {
        ObservationLogCategory::Runtime => "runtime",
        ObservationLogCategory::Connectivity => "proxy",
        ObservationLogCategory::Recovery => "subscription",
    }
}

fn map_log_level(level: ObservationLogLevel) -> &'static str {
    match level {
        ObservationLogLevel::Info => "info",
        ObservationLogLevel::Warning => "warn",
        ObservationLogLevel::Error => "error",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::observability::{
        InMemoryRuntimeObservations, ObservationLogCategory, ObservationLogLevel,
        RuntimeObservationPort,
    };

    #[test]
    fn proxy_routing_ipc_capability_is_main_only_and_names_exactly_two_commands() {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))
                .expect("capability json");
        assert_eq!(capability["windows"], serde_json::json!(["main"]));

        let routing_permissions: Vec<&str> = capability["permissions"]
            .as_array()
            .expect("permissions")
            .iter()
            .filter_map(|entry| entry.as_str())
            .filter(|permission| permission.contains("proxy-routing"))
            .collect();
        assert_eq!(
            routing_permissions,
            [
                "allow-get-proxy-routing-snapshot",
                "allow-mutate-proxy-routing"
            ]
        );

        let build_manifest = include_str!("../build.rs");
        for command in ["get_proxy_routing_snapshot", "mutate_proxy_routing"] {
            assert_eq!(
                build_manifest.matches(&format!("\"{command}\"")).count(),
                1,
                "command must be registered exactly once"
            );
        }
        assert!(
            include_str!("../permissions/autogenerated/get_proxy_routing_snapshot.toml")
                .contains("commands.allow = [\"get_proxy_routing_snapshot\"]")
        );
        assert!(
            include_str!("../permissions/autogenerated/mutate_proxy_routing.toml")
                .contains("commands.allow = [\"mutate_proxy_routing\"]")
        );
    }

    #[test]
    fn empty_supported_result_has_a_closed_wire_code() {
        assert_eq!(
            serde_json::to_value(SubscriptionErrorCode::ParseFailed).unwrap(),
            serde_json::json!("parseFailed")
        );
    }

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

    #[test]
    fn observation_response_exposes_only_fixed_safe_fields() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_runtime_log(
            ObservationLogLevel::Error,
            ObservationLogCategory::Connectivity,
            "http://127.0.0.1:9090 fixture-secret-token",
        );

        let response = RuntimeObservationResponse::from(observations.snapshot());

        assert_eq!(response.source, "mock");
        assert_eq!(response.log_summary.len(), 1);
        assert_eq!(response.log_summary[0].category, "proxy");
        assert_eq!(response.log_summary[0].level, "error");
        let encoded = serde_json::to_string(&response).expect("observation response serializes");
        assert!(!encoded.contains("fixture-secret-token"));
        assert!(!encoded.contains("127.0.0.1"));
        assert!(!encoded.contains("message"));
    }

    #[test]
    fn delta_response_uses_the_same_fixed_safe_dto() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let subscription = observations.subscribe();
        observations.record_connection_count(2);
        let delta = subscription
            .take_latest_delta()
            .expect("subscription receives the latest delta");

        let response = RuntimeObservationResponse::from(delta);

        assert_eq!(response.source, "mock");
        assert_eq!(response.connection_count, 2);
        assert_eq!(response.revision, 1);
    }

    #[test]
    fn traffic_history_response_has_only_fixed_timing_and_rate_fields() {
        let observations = InMemoryRuntimeObservations::new_mock();
        let mut snapshot = observations.snapshot();
        snapshot.observed_at_ms = 2_000;
        snapshot.traffic_history = vec![TrafficHistoryPoint {
            sampled_at_ms: 1_000,
            upload_rate_bps: 100,
            download_rate_bps: 200,
        }];
        let delta_response =
            RuntimeObservationResponse::from(RuntimeObservationDelta::from(&snapshot));
        let snapshot_response = RuntimeObservationResponse::from(snapshot);
        assert_eq!(delta_response, snapshot_response);
        assert_eq!(
            serde_json::to_value(snapshot_response).expect("safe response"),
            serde_json::json!({
                "source": "mock",
                "revision": 0,
                "observedAtMs": 2_000,
                "trafficHistory": [{
                    "sampledAtMs": 1_000,
                    "uploadRateBps": 100,
                    "downloadRateBps": 200,
                }],
                "captureMode": "off",
                "sidecarLifecycle": "notAttached",
                "appliedSubscriptionId": null, "appliedConfigurationGeneration": null,
                "subscriptionSwitch": null, "managedProxyAvailable": false, "coreMemoryBytes": null,
                "uploadRateBps": 0,
                "downloadRateBps": 0,
                "uploadTotalBytes": 0,
                "downloadTotalBytes": 0,
                "connectionCount": 0,
                "logSummary": [],
            })
        );
    }

    #[test]
    fn stopped_managed_runtime_has_a_distinct_safe_lifecycle_value() {
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_managed_stopped();

        let response = RuntimeObservationResponse::from(observations.snapshot());

        assert_eq!(response.source, "runtime");
        assert_eq!(response.sidecar_lifecycle, "stopped");
    }

    #[test]
    fn subscription_requests_reject_unknown_fields_and_source_mismatches() {
        let remote = serde_json::from_value::<ImportSubscriptionRequest>(serde_json::json!({
            "name": "Remote",
            "source": {"kind": "remote", "url": "https://example.invalid/sub"}
        }))
        .expect("valid remote request");
        assert!(matches!(
            remote.source,
            ImportSubscriptionSourceRequest::Remote { .. }
        ));
        assert!(
            serde_json::from_value::<ImportSubscriptionRequest>(serde_json::json!({
                "name": "Remote",
                "source": {"kind": "remote", "url": "https://example.invalid/sub", "content": "secret"}
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateSubscriptionRequest>(serde_json::json!({
                "id": "sub-id",
                "content": null,
                "url": "https://example.invalid/secret"
            }))
            .is_err()
        );
    }

    #[test]
    fn subscription_responses_use_the_frozen_safe_tagged_union() {
        let response = UpdateSubscriptionResponse::Ok {
            subscription: Box::new(SubscriptionSummaryResponse::from(SubscriptionSummary {
                id: "sub-safe".to_owned(),
                name: "Fixture".to_owned(),
                description: "Fixture description".into(),
                allow_auto_update: true,
                update_interval_minutes: Some(1440),
                active: true,
                active_configuration_generation: Some(2),
                shareable: true,
                source_kind: SubscriptionSourceKind::Remote,
                node_count: 2,
                skipped_node_count: 0,
                last_success_at_ms: Some(1_234),
                traffic: Some(crate::domain::SubscriptionTraffic {
                    upload: Some(1),
                    download: None,
                    total: Some(10),
                    expire_at_ms: None,
                }),
            })),
            content_changed: false,
        };

        assert_eq!(
            serde_json::to_value(response).expect("serialize response"),
            serde_json::json!({
                "status": "ok",
                "subscription": {
                    "id": "sub-safe",
                    "name": "Fixture", "description": "Fixture description", "allowAutoUpdate": true, "updateIntervalMinutes": 1440,
                    "active": true, "activeConfigurationGeneration": 2, "shareable": true,
                    "sourceKind": "remote",
                    "nodeCount": 2,
                    "skippedNodeCount": 0,
                    "lastSuccessAtMs": 1_234,
                    "traffic": {
                        "upload": 1,
                        "download": null,
                        "total": 10,
                        "expireAtMs": null
                    }
                },
                "contentChanged": false
            })
        );
        assert_eq!(
            serde_json::to_value(ListSubscriptionsResponse::Error {
                error: SubscriptionErrorCode::FetchFailed
            })
            .expect("serialize error"),
            serde_json::json!({"status": "error", "error": "fetchFailed"})
        );
    }
    #[test]
    fn edit_patch_distinguishes_omitted_clear_and_set_without_exposing_user_agent() {
        let omitted: EditSubscriptionRequest = serde_json::from_value(
            serde_json::json!({"id":"sub","remoteRequest":{},"updatePolicy":{}}),
        )
        .expect("omitted");
        let omitted: EditSubscription = omitted.into();
        assert_eq!(omitted.remote_request.expect("request").user_agent, None);
        assert_eq!(
            omitted.update_policy.expect("policy").interval_minutes,
            None
        );
        let clear: EditSubscriptionRequest = serde_json::from_value(serde_json::json!({"id":"sub","remoteRequest":{"userAgent":""},"updatePolicy":{"intervalMinutes":null}})).expect("clear");
        let clear: EditSubscription = clear.into();
        assert_eq!(
            clear.remote_request.expect("request").user_agent,
            Some(UserAgentEdit::Clear)
        );
        assert_eq!(
            clear.update_policy.expect("policy").interval_minutes,
            Some(None)
        );
        for invalid in [
            serde_json::json!({"userAgent":null}),
            serde_json::json!({"endpoint":"http://127.0.0.1:1080"}),
        ] {
            assert!(
                serde_json::from_value::<EditSubscriptionRequest>(
                    serde_json::json!({"id":"sub","remoteRequest":invalid})
                )
                .is_err()
            );
        }
        assert!(
            serde_json::from_value::<SubscriptionIdRequest>(
                serde_json::json!({"id":"sub","port":1080})
            )
            .is_err()
        );
        assert!(
            serde_json::from_value::<UpdateSubscriptionRequest>(
                serde_json::json!({"id":"sub","routeOverride":"direct"})
            )
            .is_err()
        );
    }

    #[test]
    fn activation_wire_uses_closed_pending_error_and_atomic_observation_fields() {
        let pending = ActivateSubscriptionResponse::from(ActivationResult::Pending {
            operation_id: "operation".into(),
        });
        assert_eq!(
            serde_json::to_value(pending).expect("wire"),
            serde_json::json!({"status":"pending","operationId":"operation"})
        );
        let error = ActivateSubscriptionResponse::from(ActivationResult::Error {
            operation_id: Some("operation".into()),
            error: ActivationError::StopFailed,
        });
        assert_eq!(
            serde_json::to_value(error).expect("wire"),
            serde_json::json!({"status":"error","operationId":"operation","error":"stopFailed"})
        );
        assert_eq!(
            serde_json::to_value(ManagedRuntimeStartResult::SubscriptionSelectionRequired)
                .expect("wire"),
            "subscriptionSelectionRequired"
        );
        let observations = InMemoryRuntimeObservations::new_mock();
        observations.record_subscription_ready(
            "selected".into(),
            3,
            Some("operation".into()),
            false,
        );
        let snapshot = observations.snapshot();
        let wire =
            serde_json::to_value(RuntimeObservationResponse::from(snapshot.clone())).expect("wire");
        assert_eq!(
            wire,
            serde_json::to_value(RuntimeObservationResponse::from(
                RuntimeObservationDelta::from(&snapshot)
            ))
            .expect("delta")
        );
        assert_eq!(wire["appliedSubscriptionId"], "selected");
        assert_eq!(wire["appliedConfigurationGeneration"], 3);
        assert_eq!(
            wire["subscriptionSwitch"],
            serde_json::json!({"operationId":"operation","status":"ready","errorCode":null})
        );
        assert!(wire["coreMemoryBytes"].is_null());
        assert_eq!(wire.as_object().expect("safe object").len(), 17);
    }
}
