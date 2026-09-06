use std::{
    collections::{HashMap, HashSet},
    num::NonZeroU16,
    sync::{
        Arc, Mutex, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::future::{Either, select};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    application::{
        provider_replacement::{ProviderReplacementError, apply_provider_replacement},
        state_access::{StateAccessError, StateAccessGate},
    },
    domain::{
        AppState, MAX_SAFE_INTEGER, NodeFilter, NodePool, PoolId, PoolKind, PoolSource, Provider,
        ProviderId, ProxyNode, RemoteRequestOptions, RouteTarget, SelectionPolicy, Subscription,
        SubscriptionDocument, SubscriptionDocumentFormat, SubscriptionHttpMetadata, SubscriptionId,
        SubscriptionProxyMode, SubscriptionSource, SubscriptionTraffic, SubscriptionUpdatePolicy,
    },
    storage::{JsonStateStore, StateStore},
    subscription::{
        ConditionalHeaders, DocumentError, DocumentErrorCode, FetchClientOptions, FetchError,
        FetchResult, fetch_subscription_with_options, parse_exact_document, parse_subscription,
        validate_source_url,
    },
};

const MAX_CONTENT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(test)]
pub(crate) enum ImportSource {
    Remote { url: String },
    Manual { content: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RemoteImportOptions {
    pub user_agent: Option<String>,
    pub timeout_seconds: u16,
    pub proxy_mode: SubscriptionProxyMode,
    pub verify_tls: bool,
    pub allow_auto_update: bool,
    pub interval_minutes: Option<u32>,
}

impl Default for RemoteImportOptions {
    fn default() -> Self {
        Self {
            user_agent: None,
            timeout_seconds: 30,
            proxy_mode: SubscriptionProxyMode::Direct,
            verify_tls: true,
            allow_auto_update: true,
            interval_minutes: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ImportRequestSource {
    Remote {
        url: String,
        options: RemoteImportOptions,
    },
    Manual {
        content: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubscriptionImport {
    pub name: String,
    pub description: String,
    pub source: ImportRequestSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubscriptionSourceKind {
    Remote,
    Manual,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubscriptionSummary {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_kind: SubscriptionSourceKind,
    pub node_count: usize,
    pub skipped_node_count: u32,
    pub last_success_at_ms: Option<u64>,
    pub traffic: Option<SubscriptionTraffic>,
    pub allow_auto_update: bool,
    pub update_interval_minutes: Option<u32>,
    pub active: bool,
    pub active_configuration_generation: Option<u64>,
    pub shareable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UpdateResult {
    pub subscription: SubscriptionSummary,
    pub content_changed: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct SubscriptionDocumentView {
    pub(crate) id: String,
    pub(crate) format: SubscriptionDocumentFormat,
    pub(crate) content: String,
    pub(crate) revision: String,
    pub(crate) source_kind: SubscriptionSourceKind,
    pub(crate) local_override: bool,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct DocumentSaveInput {
    pub(crate) id: String,
    pub(crate) format: SubscriptionDocumentFormat,
    pub(crate) content: String,
    pub(crate) expected_revision: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DocumentOperationError {
    InvalidInput,
    ContentTooLarge,
    FormatFailed,
    ParseFailed,
    UnsupportedNodes,
    UnsupportedClashProviders,
    NormalizationFailed,
    ValidationFailed,
    DocumentConflict,
    DocumentUnavailable,
    NotFound,
    StateUnavailable,
    SaveFailed,
    Busy,
    OperationTimedOut,
}

impl std::fmt::Display for DocumentOperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "document input is invalid",
            Self::ContentTooLarge => "document content is too large",
            Self::FormatFailed => "document formatting failed",
            Self::ParseFailed => "document parsing failed",
            Self::UnsupportedNodes => "document contains unsupported nodes",
            Self::UnsupportedClashProviders => "provider-only Clash documents are unsupported",
            Self::NormalizationFailed => "document nodes could not be normalized",
            Self::ValidationFailed => "document candidate is invalid",
            Self::DocumentConflict => "document changed since it was read",
            Self::DocumentUnavailable => "subscription document is unavailable",
            Self::NotFound => "subscription was not found",
            Self::StateUnavailable => "subscription state is unavailable",
            Self::SaveFailed => "subscription document could not be saved",
            Self::Busy => "subscription state is busy",
            Self::OperationTimedOut => "document operation timed out",
        })
    }
}

impl std::error::Error for DocumentOperationError {}

pub(crate) struct PreparedDocumentSave {
    before: AppState,
    next: AppState,
    subscription_id: String,
    document_revision: String,
    summary: SubscriptionSummary,
    requires_apply: bool,
    state_changed: bool,
    _write_guard: SubscriptionWriteGuard,
}

impl PreparedDocumentSave {
    #[cfg(test)]
    pub(crate) fn before(&self) -> &AppState {
        &self.before
    }

    pub(crate) fn next(&self) -> &AppState {
        &self.next
    }

    #[cfg(test)]
    pub(crate) fn is_selected(&self) -> bool {
        self.next.active_subscription_id.as_ref().map(|id| &id.0) == Some(&self.subscription_id)
    }

    pub(crate) fn requires_apply(&self) -> bool {
        self.requires_apply
    }

    #[cfg(test)]
    pub(crate) fn document_revision(&self) -> &str {
        &self.document_revision
    }

    #[cfg(test)]
    pub(crate) fn summary(&self) -> &SubscriptionSummary {
        &self.summary
    }

    pub(crate) fn generation(&self) -> u64 {
        self.next.active_configuration_generation
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DocumentSaveCommit {
    pub(crate) subscription: SubscriptionSummary,
    pub(crate) document_revision: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SubscriptionSettings {
    pub id: String,
    pub name: String,
    pub description: String,
    pub source_kind: SubscriptionSourceKind,
    pub url_preview: Option<String>,
    pub has_custom_user_agent: bool,
    pub remote_request: Option<RemoteRequestSettings>,
    pub update_policy: UpdatePolicySettings,
    pub shareable: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RemoteRequestSettings {
    pub timeout_seconds: u16,
    pub proxy_mode: SubscriptionProxyMode,
    pub verify_tls: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UpdatePolicySettings {
    pub allow_auto_update: bool,
    pub interval_minutes: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum UserAgentEdit {
    Clear,
    Set(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub(crate) struct RemoteRequestPatch {
    pub user_agent: Option<UserAgentEdit>,
    pub timeout_seconds: Option<u16>,
    pub proxy_mode: Option<SubscriptionProxyMode>,
    pub verify_tls: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub(crate) struct UpdatePolicyPatch {
    pub allow_auto_update: Option<bool>,
    pub interval_minutes: Option<Option<u32>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct EditSubscription {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub url_replacement: Option<String>,
    pub remote_request: Option<RemoteRequestPatch>,
    pub update_policy: Option<UpdatePolicyPatch>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UpdateRouteOverride {
    ManagedCore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SubscriptionChange {
    Updated,
    Edited,
    Activated,
    Deleted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubscriptionChangeEvent {
    pub id: String,
    pub change: SubscriptionChange,
}

pub(crate) type SubscriptionChangeSink = Arc<dyn Fn(SubscriptionChangeEvent) + Send + Sync>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SubscriptionOperationError {
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

impl std::fmt::Display for SubscriptionOperationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidInput => "subscription input is invalid",
            Self::Busy => "subscription state is busy",
            Self::FetchFailed => "subscription fetch failed",
            Self::ParseFailed => "subscription parsing failed",
            Self::NormalizationFailed => "subscription normalization failed",
            Self::ValidationFailed => "subscription candidate is invalid",
            Self::SaveFailed => "subscription state could not be saved",
            Self::StateUnavailable => "subscription state is unavailable",
            Self::NotFound => "subscription was not found",
            Self::CacheUnavailable => "subscription cache is unavailable",
            Self::IdentityFailed => "subscription identity could not be created",
            Self::InvalidOptions => "subscription options are invalid",
            Self::UnsupportedClashProviders => "provider-only Clash subscriptions are unsupported",
            Self::ProxyUnavailable => "the managed core proxy is unavailable",
            Self::SystemProxyUnavailable => "the system proxy is unavailable or unsupported",
            Self::ReferenceConflict => "subscription objects are still referenced",
            Self::NotShareable => "subscription source cannot be shared",
        })
    }
}

impl std::error::Error for SubscriptionOperationError {}

pub(crate) struct SubscriptionManager {
    store: JsonStateStore,
    gate: StateAccessGate,
    write_in_progress: Arc<AtomicBool>,
    closing: Arc<AtomicBool>,
    closing_tx: tokio::sync::watch::Sender<bool>,
    process_deadlines: Mutex<HashMap<String, u64>>,
    change_sink: Mutex<Option<SubscriptionChangeSink>>,
    managed_proxy_port: RwLock<Option<NonZeroU16>>,
    identity_source: fn() -> Result<[u8; 16], ()>,
    clock: fn() -> Result<u64, ()>,
}

impl SubscriptionManager {
    pub(crate) fn new(
        state_file: std::path::PathBuf,
        gate: StateAccessGate,
    ) -> Result<Self, SubscriptionOperationError> {
        let (closing_tx, _) = tokio::sync::watch::channel(false);
        Ok(Self {
            store: JsonStateStore::new(state_file)
                .map_err(|_| SubscriptionOperationError::StateUnavailable)?,
            gate,
            write_in_progress: Arc::new(AtomicBool::new(false)),
            closing: Arc::new(AtomicBool::new(false)),
            closing_tx,
            process_deadlines: Mutex::new(HashMap::new()),
            change_sink: Mutex::new(None),
            managed_proxy_port: RwLock::new(None),
            identity_source: system_identity,
            clock: system_clock_ms,
        })
    }

    pub(crate) fn list(&self) -> Result<Vec<SubscriptionSummary>, SubscriptionOperationError> {
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let Some(state) = self.load_existing_or_empty()? else {
            return Ok(Vec::new());
        };
        summaries(&state)
    }

    #[cfg(test)]
    pub(crate) async fn import(
        &self,
        name: String,
        source: ImportSource,
    ) -> Result<SubscriptionSummary, SubscriptionOperationError> {
        self.import_with_options(SubscriptionImport {
            name,
            description: String::new(),
            source: match source {
                ImportSource::Remote { url } => ImportRequestSource::Remote {
                    url,
                    options: RemoteImportOptions::default(),
                },
                ImportSource::Manual { content } => ImportRequestSource::Manual { content },
            },
        })
        .await
    }

    pub(crate) async fn import_with_options(
        &self,
        input: SubscriptionImport,
    ) -> Result<SubscriptionSummary, SubscriptionOperationError> {
        let _write = self.begin_write_owned()?;
        let name = validate_name(input.name)?;
        let description = validate_description(input.description)?;
        match input.source {
            ImportRequestSource::Manual { content } => {
                let parsed = parse_content(&content)?;
                let document = document_for_body(content, parsed.format);
                let summary = self.commit_import(ImportCommit {
                    name,
                    description,
                    source: SubscriptionSource::Manual,
                    remote_request: None,
                    update_policy: SubscriptionUpdatePolicy::manual(),
                    parsed,
                    document,
                    http_metadata: None,
                    expected_initially_empty: None,
                })?;
                self.notify_change(&summary.id, SubscriptionChange::Updated);
                Ok(summary)
            }
            ImportRequestSource::Remote { url, options } => {
                validate_source_url(&url).map_err(map_url_error)?;
                let remote_request = validate_remote_import_options(&options)?;
                let mut update_policy =
                    validate_update_policy(options.allow_auto_update, options.interval_minutes)?;
                let initially_empty = {
                    let _access = self.gate.try_lock().map_err(map_access_error)?;
                    self.load_existing_or_empty()?.is_none()
                };
                let fetched = self
                    .fetch_remote(&url, ConditionalHeaders::default(), &remote_request, None)
                    .await?;
                let FetchResult::Modified {
                    body,
                    metadata,
                    profile_update_interval_minutes,
                    ..
                } = fetched
                else {
                    return Err(SubscriptionOperationError::CacheUnavailable);
                };
                adopt_profile_interval(&mut update_policy, profile_update_interval_minutes);
                let parsed = parse_content(&body)?;
                let document = document_for_body(body, parsed.format);
                let summary = self.commit_import(ImportCommit {
                    name,
                    description,
                    source: SubscriptionSource::Remote { url },
                    remote_request: Some(remote_request),
                    update_policy,
                    parsed,
                    document,
                    http_metadata: Some(metadata),
                    expected_initially_empty: Some(initially_empty),
                })?;
                self.notify_change(&summary.id, SubscriptionChange::Updated);
                Ok(summary)
            }
        }
    }

    #[cfg(test)]
    pub(crate) async fn update(
        &self,
        id: String,
        content: Option<String>,
    ) -> Result<UpdateResult, SubscriptionOperationError> {
        self.update_with_route(id, content, None).await
    }

    pub(crate) async fn update_with_route(
        &self,
        id: String,
        content: Option<String>,
        route_override: Option<UpdateRouteOverride>,
    ) -> Result<UpdateResult, SubscriptionOperationError> {
        let _write = self.begin_write_owned()?;
        if id.trim().is_empty() || id.len() > 128 {
            return Err(SubscriptionOperationError::InvalidInput);
        }
        let snapshot = {
            let _access = self.gate.try_lock().map_err(map_access_error)?;
            let state = self
                .load_existing_or_empty()?
                .ok_or(SubscriptionOperationError::NotFound)?;
            update_snapshot(&state, &id)?
        };

        let mut attempt_at_ms = None;
        let fetched = match (&snapshot.source, content, route_override) {
            (SubscriptionSource::Remote { url }, None, override_mode) => {
                let conditional = if snapshot.nodes.is_empty()
                    || snapshot
                        .document
                        .as_ref()
                        .is_none_or(|document| document.local_override)
                {
                    ConditionalHeaders::default()
                } else {
                    ConditionalHeaders {
                        etag: snapshot
                            .http_metadata
                            .as_ref()
                            .and_then(|metadata| metadata.etag.clone()),
                        last_modified: snapshot
                            .http_metadata
                            .as_ref()
                            .and_then(|metadata| metadata.last_modified.clone()),
                    }
                };
                let remote_request = snapshot
                    .remote_request
                    .as_ref()
                    .ok_or(SubscriptionOperationError::ValidationFailed)?;
                match self
                    .fetch_remote(url, conditional, remote_request, override_mode)
                    .await
                {
                    Ok(value) => {
                        let now = (self.clock)()
                            .map_err(|_| SubscriptionOperationError::StateUnavailable)?;
                        self.remember_process_deadline(&snapshot, now);
                        attempt_at_ms = Some(now);
                        value
                    }
                    Err(SubscriptionOperationError::Busy) if self.is_closing() => {
                        return Err(SubscriptionOperationError::Busy);
                    }
                    Err(error) => {
                        let now = (self.clock)()
                            .map_err(|_| SubscriptionOperationError::StateUnavailable)?;
                        self.remember_process_deadline(&snapshot, now);
                        self.record_failed_attempt(&snapshot, now)?;
                        return Err(error);
                    }
                }
            }
            (SubscriptionSource::Manual, Some(content), None) => FetchResult::Modified {
                body: content,
                metadata: SubscriptionHttpMetadata::default(),
                profile_update_interval_minutes: None,
                validators_match_source: true,
            },
            _ => return Err(SubscriptionOperationError::InvalidInput),
        };

        let parsed = match &fetched {
            FetchResult::Modified { body, .. } => match parse_content(body) {
                Ok(parsed) => Some(parsed),
                Err(error) => {
                    if matches!(snapshot.source, SubscriptionSource::Remote { .. }) {
                        self.record_failed_attempt(
                            &snapshot,
                            attempt_at_ms.ok_or(SubscriptionOperationError::StateUnavailable)?,
                        )?;
                    }
                    return Err(error);
                }
            },
            FetchResult::NotModified { .. } => None,
        };

        if matches!(&fetched, FetchResult::NotModified { .. })
            && (snapshot.nodes.is_empty()
                || snapshot
                    .document
                    .as_ref()
                    .is_none_or(|document| document.local_override))
        {
            self.record_failed_attempt(
                &snapshot,
                attempt_at_ms.ok_or(SubscriptionOperationError::StateUnavailable)?,
            )?;
            return Err(SubscriptionOperationError::CacheUnavailable);
        }

        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let current = self
            .load_existing_or_empty()?
            .ok_or(SubscriptionOperationError::NotFound)?;
        if update_snapshot(&current, &id)? != snapshot {
            return Err(SubscriptionOperationError::Busy);
        }
        let mut candidate = current;
        let subscription_index = candidate
            .subscriptions
            .iter()
            .position(|subscription| subscription.id.0 == id)
            .ok_or(SubscriptionOperationError::NotFound)?;
        let content_changed = match (fetched, parsed) {
            (
                FetchResult::Modified {
                    body,
                    metadata,
                    profile_update_interval_minutes,
                    ..
                },
                Some(mut parsed),
            ) => {
                let skipped_count = parsed.skipped.len() as u32;
                parsed.skipped.clear();
                let document = document_for_body(body, parsed.format);
                let changed = apply_provider_replacement(
                    &mut candidate,
                    snapshot.provider_id.clone(),
                    parsed,
                )
                .map_err(map_replacement_error)?;
                candidate.subscriptions[subscription_index].skipped_unsupported_nodes =
                    skipped_count;
                candidate.subscriptions[subscription_index].http_metadata =
                    matches!(snapshot.source, SubscriptionSource::Remote { .. })
                        .then_some(metadata);
                candidate.subscriptions[subscription_index].document = document;
                adopt_profile_interval(
                    &mut candidate.subscriptions[subscription_index].update_policy,
                    profile_update_interval_minutes,
                );
                changed
            }
            (
                FetchResult::NotModified {
                    metadata,
                    profile_update_interval_minutes,
                    validators_match_source,
                },
                None,
            ) => {
                candidate.subscriptions[subscription_index].http_metadata = Some(merge_metadata(
                    snapshot.http_metadata.clone().unwrap_or_default(),
                    metadata,
                    validators_match_source,
                ));
                adopt_profile_interval(
                    &mut candidate.subscriptions[subscription_index].update_policy,
                    profile_update_interval_minutes,
                );
                false
            }
            _ => return Err(SubscriptionOperationError::ParseFailed),
        };
        let now = match attempt_at_ms {
            Some(value) => value,
            None => (self.clock)().map_err(|_| SubscriptionOperationError::StateUnavailable)?,
        };
        candidate.subscriptions[subscription_index].last_success_at_ms = Some(now);
        candidate.subscriptions[subscription_index].last_attempt_at_ms = Some(now);
        if content_changed
            && candidate.active_subscription_id.as_ref()
                == Some(&candidate.subscriptions[subscription_index].id)
        {
            candidate.active_configuration_generation =
                next_generation(candidate.active_configuration_generation)?;
        }
        candidate
            .validate()
            .map_err(|_| SubscriptionOperationError::ValidationFailed)?;
        self.ensure_commit_open()?;
        self.store
            .save(&candidate)
            .map_err(|_| SubscriptionOperationError::SaveFailed)?;
        let result = UpdateResult {
            subscription: summary_for(&candidate, subscription_index),
            content_changed,
        };
        drop(_access);
        self.notify_change(&id, SubscriptionChange::Updated);
        Ok(result)
    }

    fn commit_import(
        &self,
        input: ImportCommit,
    ) -> Result<SubscriptionSummary, SubscriptionOperationError> {
        let ImportCommit {
            name,
            description,
            source,
            remote_request,
            update_policy,
            mut parsed,
            document,
            http_metadata,
            expected_initially_empty,
        } = input;
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let current = self.load_existing_or_empty()?;
        if expected_initially_empty.is_some_and(|was_empty| was_empty != current.is_none()) {
            return Err(SubscriptionOperationError::Busy);
        }
        let mut candidate = current.unwrap_or_else(AppState::empty);
        let suffix = identity_suffix(
            (self.identity_source)().map_err(|_| SubscriptionOperationError::IdentityFailed)?,
        );
        let subscription_id = SubscriptionId(format!("sub-{suffix}"));
        let provider_id = ProviderId(format!("provider-{suffix}"));
        let pool_id = PoolId(format!("pool-{suffix}"));
        if candidate
            .subscriptions
            .iter()
            .any(|value| value.id == subscription_id)
            || candidate
                .providers
                .iter()
                .any(|value| value.id == provider_id)
            || candidate.pools.iter().any(|value| value.id == pool_id)
        {
            return Err(SubscriptionOperationError::IdentityFailed);
        }
        let now = (self.clock)().map_err(|_| SubscriptionOperationError::StateUnavailable)?;
        candidate.subscriptions.push(Subscription {
            id: subscription_id.clone(),
            skipped_unsupported_nodes: parsed.skipped.len() as u32,
            name: name.clone(),
            description,
            source,
            last_success_at_ms: Some(now),
            last_attempt_at_ms: Some(now),
            http_metadata,
            remote_request,
            update_policy,
            document,
        });
        candidate.providers.push(Provider {
            id: provider_id.clone(),
            subscription_id,
            name: name.clone(),
        });
        candidate.pools.push(NodePool {
            id: pool_id,
            name,
            kind: PoolKind::ImplicitProvider,
            sources: vec![PoolSource {
                provider_id: provider_id.clone(),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
            },
            enabled: true,
        });
        parsed.skipped.clear();
        apply_provider_replacement(&mut candidate, provider_id, parsed)
            .map_err(map_replacement_error)?;
        candidate
            .validate()
            .map_err(|_| SubscriptionOperationError::ValidationFailed)?;
        self.ensure_commit_open()?;
        self.store
            .save(&candidate)
            .map_err(|_| SubscriptionOperationError::SaveFailed)?;
        Ok(summary_for(
            &candidate,
            candidate.subscriptions.len().saturating_sub(1),
        ))
    }

    pub(crate) fn get_settings(
        &self,
        id: &str,
    ) -> Result<SubscriptionSettings, SubscriptionOperationError> {
        validate_id(id)?;
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let state = self
            .load_existing_or_empty()?
            .ok_or(SubscriptionOperationError::NotFound)?;
        let subscription = state
            .subscriptions
            .iter()
            .find(|value| value.id.0 == id)
            .ok_or(SubscriptionOperationError::NotFound)?;
        Ok(settings_for(subscription))
    }

    pub(crate) fn read_document(
        &self,
        id: &str,
    ) -> Result<SubscriptionDocumentView, DocumentOperationError> {
        validate_id(id).map_err(|_| DocumentOperationError::InvalidInput)?;
        let _access = self.gate.try_lock().map_err(map_document_access_error)?;
        let state = self
            .load_existing_or_empty()
            .map_err(map_document_subscription_error)?
            .ok_or(DocumentOperationError::NotFound)?;
        let subscription = state
            .subscriptions
            .iter()
            .find(|value| value.id.0 == id)
            .ok_or(DocumentOperationError::NotFound)?;
        let document = subscription
            .document
            .as_ref()
            .ok_or(DocumentOperationError::DocumentUnavailable)?;
        Ok(SubscriptionDocumentView {
            id: subscription.id.0.clone(),
            format: document.format,
            content: document.content.clone(),
            revision: document_revision(document),
            source_kind: match &subscription.source {
                SubscriptionSource::Remote { .. } => SubscriptionSourceKind::Remote,
                SubscriptionSource::Manual => SubscriptionSourceKind::Manual,
            },
            local_override: document.local_override,
        })
    }

    pub(crate) fn prepare_document_save(
        &self,
        input: DocumentSaveInput,
    ) -> Result<PreparedDocumentSave, DocumentOperationError> {
        validate_id(&input.id).map_err(|_| DocumentOperationError::InvalidInput)?;
        validate_document_revision(&input.expected_revision)?;
        let exact =
            parse_exact_document(input.content, input.format).map_err(map_document_parse_error)?;
        let mut parsed = exact.parsed;
        let node_count = parsed.nodes.len();
        deduplicate_parsed_nodes(&mut parsed);
        if parsed.nodes.len() != node_count {
            return Err(DocumentOperationError::NormalizationFailed);
        }
        let _write_guard = self
            .begin_write_owned()
            .map_err(map_document_subscription_error)?;
        let before = {
            let _access = self.gate.try_lock().map_err(map_document_access_error)?;
            self.load_existing_or_empty()
                .map_err(map_document_subscription_error)?
                .ok_or(DocumentOperationError::NotFound)?
        };
        let index = before
            .subscriptions
            .iter()
            .position(|value| value.id.0 == input.id)
            .ok_or(DocumentOperationError::NotFound)?;
        let existing_document = before.subscriptions[index]
            .document
            .as_ref()
            .ok_or(DocumentOperationError::DocumentUnavailable)?;
        if document_revision(existing_document) != input.expected_revision {
            return Err(DocumentOperationError::DocumentConflict);
        }

        let document_revision = revision_for(input.format, &exact.content);
        let document_changed = document_revision != input.expected_revision;
        let is_remote = matches!(
            before.subscriptions[index].source,
            SubscriptionSource::Remote { .. }
        );
        let local_override = if document_changed && is_remote {
            true
        } else {
            existing_document.local_override
        };
        let mut next = before.clone();
        next.subscriptions[index].document = Some(SubscriptionDocument {
            format: input.format,
            content: exact.content,
            local_override,
        });
        next.subscriptions[index].skipped_unsupported_nodes = 0;
        if document_changed
            && is_remote
            && let Some(metadata) = &mut next.subscriptions[index].http_metadata
        {
            metadata.etag = None;
            metadata.last_modified = None;
        }
        let subscription_id = next.subscriptions[index].id.clone();
        let provider_id = next
            .providers
            .iter()
            .filter(|provider| provider.subscription_id == subscription_id)
            .map(|provider| provider.id.clone())
            .collect::<Vec<_>>();
        if provider_id.len() != 1 {
            return Err(DocumentOperationError::ValidationFailed);
        }
        let nodes_changed = apply_provider_replacement(&mut next, provider_id[0].clone(), parsed)
            .map_err(map_document_replacement_error)?;
        let is_selected = next.active_subscription_id.as_ref() == Some(&subscription_id);
        let requires_apply = is_selected && (document_changed || nodes_changed);
        if requires_apply {
            next.active_configuration_generation =
                next_generation(next.active_configuration_generation)
                    .map_err(map_document_subscription_error)?;
        }
        next.validate()
            .map_err(|_| DocumentOperationError::ValidationFailed)?;
        let summary = summary_for(&next, index);
        let state_changed = before != next;
        Ok(PreparedDocumentSave {
            before,
            next,
            subscription_id: input.id,
            document_revision,
            summary,
            requires_apply,
            state_changed,
            _write_guard,
        })
    }

    pub(crate) fn commit_prepared_document(
        &self,
        prepared: &PreparedDocumentSave,
    ) -> Result<DocumentSaveCommit, DocumentOperationError> {
        let _access = self.gate.try_lock().map_err(map_document_access_error)?;
        let current = self
            .load_existing_or_empty()
            .map_err(map_document_subscription_error)?
            .ok_or(DocumentOperationError::NotFound)?;
        if current != prepared.before {
            return Err(DocumentOperationError::DocumentConflict);
        }
        self.ensure_commit_open()
            .map_err(map_document_subscription_error)?;
        if prepared.state_changed {
            self.store
                .save(&prepared.next)
                .map_err(|_| DocumentOperationError::SaveFailed)?;
        }
        let result = DocumentSaveCommit {
            subscription: prepared.summary.clone(),
            document_revision: prepared.document_revision.clone(),
        };
        drop(_access);
        if prepared.state_changed {
            self.notify_change(&prepared.subscription_id, SubscriptionChange::Edited);
        }
        Ok(result)
    }

    pub(crate) fn share_url(&self, id: &str) -> Result<String, SubscriptionOperationError> {
        validate_id(id)?;
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let state = self
            .load_existing_or_empty()?
            .ok_or(SubscriptionOperationError::NotFound)?;
        let subscription = state
            .subscriptions
            .iter()
            .find(|value| value.id.0 == id)
            .ok_or(SubscriptionOperationError::NotFound)?;
        match &subscription.source {
            SubscriptionSource::Remote { url } => Ok(url.clone()),
            SubscriptionSource::Manual => Err(SubscriptionOperationError::NotShareable),
        }
    }

    pub(crate) async fn edit(
        &self,
        input: EditSubscription,
    ) -> Result<UpdateResult, SubscriptionOperationError> {
        let _write = self.begin_write_owned()?;
        validate_id(&input.id)?;
        let baseline = {
            let _access = self.gate.try_lock().map_err(map_access_error)?;
            self.load_existing_or_empty()?
                .ok_or(SubscriptionOperationError::NotFound)?
        };
        let index = baseline
            .subscriptions
            .iter()
            .position(|value| value.id.0 == input.id)
            .ok_or(SubscriptionOperationError::NotFound)?;
        let mut edited = baseline.subscriptions[index].clone();
        if let Some(name) = input.name {
            edited.name = validate_name(name)?;
        }
        if let Some(description) = input.description {
            edited.description = validate_description(description)?;
        }

        let replacement_url = input.url_replacement;
        let mut url_changed = false;
        match &mut edited.source {
            SubscriptionSource::Manual => {
                if replacement_url.is_some()
                    || input.remote_request.is_some()
                    || input.update_policy.is_some()
                {
                    return Err(SubscriptionOperationError::InvalidOptions);
                }
            }
            SubscriptionSource::Remote { url } => {
                if let Some(value) = replacement_url.as_ref() {
                    validate_source_url(value).map_err(map_url_error)?;
                    url_changed = value.as_str() != url.as_str();
                    if url_changed {
                        *url = value.clone();
                    }
                }
                let remote = edited
                    .remote_request
                    .as_mut()
                    .ok_or(SubscriptionOperationError::ValidationFailed)?;
                if let Some(patch) = input.remote_request {
                    apply_remote_patch(remote, patch)?;
                }
                if let Some(patch) = input.update_policy {
                    apply_update_policy_patch(&mut edited.update_policy, patch)?;
                }
            }
        }

        let replacement = if url_changed {
            let SubscriptionSource::Remote { url } = &edited.source else {
                return Err(SubscriptionOperationError::InvalidOptions);
            };
            let options = edited
                .remote_request
                .as_ref()
                .ok_or(SubscriptionOperationError::ValidationFailed)?;
            let baseline_snapshot = update_snapshot(&baseline, &input.id)?;
            let fetched = match self
                .fetch_remote(url, ConditionalHeaders::default(), options, None)
                .await
            {
                Ok(value) => value,
                Err(SubscriptionOperationError::Busy) if self.is_closing() => {
                    return Err(SubscriptionOperationError::Busy);
                }
                Err(error) => {
                    let now =
                        (self.clock)().map_err(|_| SubscriptionOperationError::StateUnavailable)?;
                    self.remember_process_deadline(&baseline_snapshot, now);
                    self.record_failed_attempt(&baseline_snapshot, now)?;
                    return Err(error);
                }
            };
            let now = (self.clock)().map_err(|_| SubscriptionOperationError::StateUnavailable)?;
            let FetchResult::Modified {
                body,
                metadata,
                profile_update_interval_minutes,
                ..
            } = fetched
            else {
                self.remember_process_deadline(&baseline_snapshot, now);
                self.record_failed_attempt(&baseline_snapshot, now)?;
                return Err(SubscriptionOperationError::CacheUnavailable);
            };
            let mut attempted_snapshot = baseline_snapshot.clone();
            attempted_snapshot.update_policy = edited.update_policy.clone();
            adopt_profile_interval(
                &mut attempted_snapshot.update_policy,
                profile_update_interval_minutes,
            );
            self.remember_process_deadline(&attempted_snapshot, now);
            let parsed = match parse_content(&body) {
                Ok(value) => value,
                Err(error) => {
                    self.record_failed_attempt(&baseline_snapshot, now)?;
                    return Err(error);
                }
            };
            let document = document_for_body(body, parsed.format);
            Some((
                parsed,
                document,
                metadata,
                profile_update_interval_minutes,
                now,
            ))
        } else {
            None
        };

        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let current = self
            .load_existing_or_empty()?
            .ok_or(SubscriptionOperationError::NotFound)?;
        if current != baseline {
            return Err(SubscriptionOperationError::Busy);
        }
        let mut candidate = current;
        candidate.subscriptions[index] = edited;
        let subscription_id = candidate.subscriptions[index].id.clone();
        let provider = candidate
            .providers
            .iter_mut()
            .find(|value| value.subscription_id == subscription_id)
            .ok_or(SubscriptionOperationError::ValidationFailed)?;
        provider.name = candidate.subscriptions[index].name.clone();
        let provider_id = provider.id.clone();
        for pool in &mut candidate.pools {
            if pool.kind == PoolKind::ImplicitProvider
                && pool.sources.len() == 1
                && pool.sources[0].provider_id == provider_id
            {
                pool.name = candidate.subscriptions[index].name.clone();
            }
        }
        let content_changed = if let Some((mut parsed, document, metadata, profile_interval, now)) =
            replacement
        {
            candidate.subscriptions[index].skipped_unsupported_nodes = parsed.skipped.len() as u32;
            parsed.skipped.clear();
            let changed = apply_provider_replacement(&mut candidate, provider_id, parsed)
                .map_err(map_replacement_error)?;
            candidate.subscriptions[index].http_metadata = Some(metadata);
            candidate.subscriptions[index].document = document;
            adopt_profile_interval(
                &mut candidate.subscriptions[index].update_policy,
                profile_interval,
            );
            candidate.subscriptions[index].last_success_at_ms = Some(now);
            candidate.subscriptions[index].last_attempt_at_ms = Some(now);
            if changed && candidate.active_subscription_id.as_ref() == Some(&subscription_id) {
                candidate.active_configuration_generation =
                    next_generation(candidate.active_configuration_generation)?;
            }
            changed
        } else {
            false
        };
        candidate
            .validate()
            .map_err(|_| SubscriptionOperationError::ValidationFailed)?;
        self.ensure_commit_open()?;
        self.store
            .save(&candidate)
            .map_err(|_| SubscriptionOperationError::SaveFailed)?;
        let result = UpdateResult {
            subscription: summary_for(&candidate, index),
            content_changed,
        };
        drop(_access);
        self.notify_change(&input.id, SubscriptionChange::Edited);
        Ok(result)
    }

    pub(crate) fn delete(
        &self,
        id: String,
        active_runtime_stopped: bool,
    ) -> Result<String, SubscriptionOperationError> {
        let _write = self.begin_write_owned()?;
        validate_id(&id)?;
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let mut candidate = self
            .load_existing_or_empty()?
            .ok_or(SubscriptionOperationError::NotFound)?;
        let subscription_id = candidate
            .subscriptions
            .iter()
            .find(|value| value.id.0 == id)
            .map(|value| value.id.clone())
            .ok_or(SubscriptionOperationError::NotFound)?;
        let is_active = candidate.active_subscription_id.as_ref() == Some(&subscription_id);
        if is_active && !active_runtime_stopped {
            return Err(SubscriptionOperationError::ReferenceConflict);
        }
        let provider_ids = candidate
            .providers
            .iter()
            .filter(|value| value.subscription_id == subscription_id)
            .map(|value| value.id.clone())
            .collect::<Vec<_>>();
        if provider_ids.is_empty() {
            return Err(SubscriptionOperationError::ValidationFailed);
        }
        let owned_pool_ids = candidate
            .pools
            .iter()
            .filter(|pool| {
                pool.kind == PoolKind::ImplicitProvider
                    && !pool.sources.is_empty()
                    && pool
                        .sources
                        .iter()
                        .all(|source| provider_ids.contains(&source.provider_id))
            })
            .map(|pool| pool.id.clone())
            .collect::<Vec<_>>();
        if candidate.pools.iter().any(|pool| {
            !owned_pool_ids.contains(&pool.id)
                && pool
                    .sources
                    .iter()
                    .any(|source| provider_ids.contains(&source.provider_id))
        }) || candidate.routes.iter().any(|route| {
            matches!(&route.target, RouteTarget::Pool(pool_id) if owned_pool_ids.contains(pool_id))
        }) || matches!(&candidate.default_target, RouteTarget::Pool(pool_id) if owned_pool_ids.contains(pool_id))
        {
            return Err(SubscriptionOperationError::ReferenceConflict);
        }
        candidate
            .nodes
            .retain(|node| !provider_ids.contains(&node.provider_id));
        candidate
            .pools
            .retain(|pool| !owned_pool_ids.contains(&pool.id));
        candidate
            .providers
            .retain(|provider| !provider_ids.contains(&provider.id));
        candidate
            .subscriptions
            .retain(|subscription| subscription.id != subscription_id);
        if is_active {
            candidate.active_subscription_id = None;
            candidate.active_configuration_generation =
                next_generation(candidate.active_configuration_generation)?;
        }
        candidate
            .validate()
            .map_err(|_| SubscriptionOperationError::ValidationFailed)?;
        self.ensure_commit_open()?;
        self.store
            .save(&candidate)
            .map_err(|_| SubscriptionOperationError::SaveFailed)?;
        drop(_access);
        self.notify_change(&id, SubscriptionChange::Deleted);
        Ok(id)
    }

    pub(crate) fn due_subscription_ids(
        &self,
        now_ms: u64,
    ) -> Result<Vec<String>, SubscriptionOperationError> {
        if now_ms > MAX_SAFE_INTEGER {
            return Err(SubscriptionOperationError::InvalidInput);
        }
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let Some(state) = self.load_existing_or_empty()? else {
            return Ok(Vec::new());
        };
        let deadlines = self
            .process_deadlines
            .lock()
            .expect("subscription process deadline mutex");
        let mut due = state
            .subscriptions
            .iter()
            .filter(|subscription| {
                matches!(subscription.source, SubscriptionSource::Remote { .. })
                    && subscription.update_policy.allow_auto_update
                    && subscription
                        .update_policy
                        .interval_minutes
                        .is_some_and(|minutes| {
                            let persisted_base = subscription
                                .last_attempt_at_ms
                                .into_iter()
                                .chain(subscription.last_success_at_ms)
                                .max()
                                .unwrap_or(0);
                            let persisted_deadline = persisted_base
                                .saturating_add(u64::from(minutes).saturating_mul(60_000));
                            let deadline = deadlines
                                .get(&subscription.id.0)
                                .copied()
                                .unwrap_or(0)
                                .max(persisted_deadline);
                            now_ms >= deadline
                        })
            })
            .map(|subscription| subscription.id.0.clone())
            .collect::<Vec<_>>();
        due.sort();
        Ok(due)
    }

    pub(crate) fn begin_write_owned(
        &self,
    ) -> Result<SubscriptionWriteGuard, SubscriptionOperationError> {
        if self.is_closing() {
            return Err(SubscriptionOperationError::Busy);
        }
        self.write_in_progress
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| SubscriptionOperationError::Busy)?;
        if self.is_closing() {
            self.write_in_progress.store(false, Ordering::Release);
            return Err(SubscriptionOperationError::Busy);
        }
        Ok(SubscriptionWriteGuard {
            flag: Arc::clone(&self.write_in_progress),
        })
    }

    pub(crate) fn request_closing(&self) {
        self.closing.store(true, Ordering::Release);
        self.closing_tx.send_replace(true);
    }

    pub(crate) fn is_closing(&self) -> bool {
        self.closing.load(Ordering::Acquire)
    }

    pub(crate) fn closing_receiver(&self) -> tokio::sync::watch::Receiver<bool> {
        self.closing_tx.subscribe()
    }

    pub(crate) fn shutdown_quiescent(&self, deadline: std::time::Instant) -> bool {
        self.request_closing();
        loop {
            if !self.write_in_progress.load(Ordering::Acquire)
                && let Ok(access) = self.gate.try_lock()
            {
                let complete = !self.write_in_progress.load(Ordering::Acquire);
                drop(access);
                if complete {
                    return true;
                }
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[cfg(test)]
    fn begin_write(&self) -> Result<SubscriptionWriteGuard, SubscriptionOperationError> {
        self.begin_write_owned()
    }

    pub(crate) fn install_change_sink(&self, sink: SubscriptionChangeSink) {
        *self
            .change_sink
            .lock()
            .expect("subscription change sink mutex") = Some(sink);
    }

    pub(crate) fn notify_change(&self, id: &str, change: SubscriptionChange) {
        let sink = self
            .change_sink
            .lock()
            .expect("subscription change sink mutex")
            .clone();
        if let Some(sink) = sink {
            sink(SubscriptionChangeEvent {
                id: id.to_owned(),
                change,
            });
        }
    }

    pub(crate) fn set_managed_proxy_port(&self, port: Option<NonZeroU16>) {
        *self
            .managed_proxy_port
            .write()
            .expect("managed subscription proxy port lock") = port;
    }

    fn ensure_commit_open(&self) -> Result<(), SubscriptionOperationError> {
        if self.is_closing() {
            Err(SubscriptionOperationError::Busy)
        } else {
            Ok(())
        }
    }

    fn remember_process_deadline(&self, snapshot: &UpdateSnapshot, attempted_at_ms: u64) {
        let Some(minutes) = snapshot.update_policy.interval_minutes else {
            return;
        };
        let deadline = attempted_at_ms.saturating_add(u64::from(minutes).saturating_mul(60_000));
        self.process_deadlines
            .lock()
            .expect("subscription process deadline mutex")
            .insert(snapshot.subscription_id.0.clone(), deadline);
    }

    async fn fetch_remote(
        &self,
        url: &str,
        conditional: ConditionalHeaders,
        remote: &RemoteRequestOptions,
        route_override: Option<UpdateRouteOverride>,
    ) -> Result<FetchResult, SubscriptionOperationError> {
        let mode = match route_override {
            Some(UpdateRouteOverride::ManagedCore) => SubscriptionProxyMode::ManagedCore,
            None => remote.proxy_mode,
        };
        let proxy_url = match mode {
            SubscriptionProxyMode::Direct => None,
            SubscriptionProxyMode::System => Some(
                crate::platform::windows::system_proxy::read_subscription_proxy_url()
                    .map_err(|_| SubscriptionOperationError::SystemProxyUnavailable)?,
            ),
            SubscriptionProxyMode::ManagedCore => {
                let port = *self
                    .managed_proxy_port
                    .read()
                    .expect("managed subscription proxy port lock");
                Some(format!(
                    "http://127.0.0.1:{}",
                    port.ok_or(SubscriptionOperationError::ProxyUnavailable)?
                        .get()
                ))
            }
        };
        if self.is_closing() {
            return Err(SubscriptionOperationError::Busy);
        }
        let mut closing = self.closing_tx.subscribe();
        let options = FetchClientOptions {
            user_agent: remote.user_agent.clone(),
            timeout: Duration::from_secs(u64::from(remote.timeout_seconds)),
            proxy_url,
            verify_tls: remote.verify_tls,
        };
        let fetch = fetch_subscription_with_options(url, conditional, &options);
        match select(
            Box::pin(closing.wait_for(|closing| *closing)),
            Box::pin(fetch),
        )
        .await
        {
            Either::Left((result, _)) => {
                let _ = result;
                Err(SubscriptionOperationError::Busy)
            }
            Either::Right((result, _)) => result.map_err(map_fetch_error),
        }
    }

    fn record_failed_attempt(
        &self,
        snapshot: &UpdateSnapshot,
        attempted_at_ms: u64,
    ) -> Result<(), SubscriptionOperationError> {
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let mut current = self
            .load_existing_or_empty()?
            .ok_or(SubscriptionOperationError::NotFound)?;
        if update_snapshot(&current, &snapshot.subscription_id.0)? != *snapshot {
            return Err(SubscriptionOperationError::Busy);
        }
        let index = current
            .subscriptions
            .iter()
            .position(|value| value.id == snapshot.subscription_id)
            .ok_or(SubscriptionOperationError::NotFound)?;
        current.subscriptions[index].last_attempt_at_ms = Some(attempted_at_ms);
        current
            .validate()
            .map_err(|_| SubscriptionOperationError::ValidationFailed)?;
        self.ensure_commit_open()?;
        self.store
            .save(&current)
            .map_err(|_| SubscriptionOperationError::SaveFailed)?;
        drop(_access);
        self.notify_change(&snapshot.subscription_id.0, SubscriptionChange::Updated);
        Ok(())
    }

    fn load_existing_or_empty(&self) -> Result<Option<AppState>, SubscriptionOperationError> {
        if !self
            .store
            .has_snapshot_or_backup()
            .map_err(|_| SubscriptionOperationError::StateUnavailable)?
        {
            return Ok(None);
        }
        self.store
            .load()
            .map(Some)
            .map_err(|_| SubscriptionOperationError::StateUnavailable)
    }

    #[cfg(test)]
    fn with_test_sources(
        state_file: std::path::PathBuf,
        gate: StateAccessGate,
        identity_source: fn() -> Result<[u8; 16], ()>,
        clock: fn() -> Result<u64, ()>,
    ) -> Self {
        let (closing_tx, _) = tokio::sync::watch::channel(false);
        Self {
            store: JsonStateStore::new(state_file).expect("valid test state path"),
            gate,
            write_in_progress: Arc::new(AtomicBool::new(false)),
            closing: Arc::new(AtomicBool::new(false)),
            closing_tx,
            process_deadlines: Mutex::new(HashMap::new()),
            change_sink: Mutex::new(None),
            managed_proxy_port: RwLock::new(None),
            identity_source,
            clock,
        }
    }
}

struct ImportCommit {
    name: String,
    description: String,
    source: SubscriptionSource,
    remote_request: Option<RemoteRequestOptions>,
    update_policy: SubscriptionUpdatePolicy,
    parsed: crate::subscription::ParseResult,
    document: Option<SubscriptionDocument>,
    http_metadata: Option<SubscriptionHttpMetadata>,
    expected_initially_empty: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct UpdateSnapshot {
    subscription_id: SubscriptionId,
    source: SubscriptionSource,
    http_metadata: Option<SubscriptionHttpMetadata>,
    remote_request: Option<RemoteRequestOptions>,
    update_policy: SubscriptionUpdatePolicy,
    document: Option<SubscriptionDocument>,
    provider_id: ProviderId,
    nodes: Vec<ProxyNode>,
}

fn update_snapshot(
    state: &AppState,
    id: &str,
) -> Result<UpdateSnapshot, SubscriptionOperationError> {
    let subscription = state
        .subscriptions
        .iter()
        .find(|subscription| subscription.id.0 == id)
        .ok_or(SubscriptionOperationError::NotFound)?;
    let providers = state
        .providers
        .iter()
        .filter(|provider| provider.subscription_id == subscription.id)
        .collect::<Vec<_>>();
    if providers.len() != 1 {
        return Err(SubscriptionOperationError::ValidationFailed);
    }
    let provider_id = providers[0].id.clone();
    Ok(UpdateSnapshot {
        subscription_id: subscription.id.clone(),
        source: subscription.source.clone(),
        http_metadata: subscription.http_metadata.clone(),
        remote_request: subscription.remote_request.clone(),
        update_policy: subscription.update_policy.clone(),
        document: subscription.document.clone(),
        nodes: state
            .nodes
            .iter()
            .filter(|node| node.provider_id == provider_id)
            .cloned()
            .collect(),
        provider_id,
    })
}

pub(crate) fn summaries(
    state: &AppState,
) -> Result<Vec<SubscriptionSummary>, SubscriptionOperationError> {
    state
        .validate()
        .map_err(|_| SubscriptionOperationError::ValidationFailed)?;
    Ok(state
        .subscriptions
        .iter()
        .enumerate()
        .map(|(index, _)| summary_for(state, index))
        .collect())
}

fn summary_for(state: &AppState, index: usize) -> SubscriptionSummary {
    let subscription = &state.subscriptions[index];
    let provider_ids = state
        .providers
        .iter()
        .filter(|provider| provider.subscription_id == subscription.id)
        .map(|provider| &provider.id)
        .collect::<Vec<_>>();
    SubscriptionSummary {
        id: subscription.id.0.clone(),
        name: safe_display_name(&subscription.name),
        description: subscription.description.clone(),
        source_kind: match &subscription.source {
            SubscriptionSource::Remote { .. } => SubscriptionSourceKind::Remote,
            SubscriptionSource::Manual => SubscriptionSourceKind::Manual,
        },
        skipped_node_count: subscription.skipped_unsupported_nodes,
        node_count: state
            .nodes
            .iter()
            .filter(|node| provider_ids.contains(&&node.provider_id))
            .count(),
        last_success_at_ms: subscription.last_success_at_ms,
        traffic: subscription
            .http_metadata
            .as_ref()
            .and_then(|metadata| metadata.subscription_userinfo.clone()),
        allow_auto_update: subscription.update_policy.allow_auto_update,
        update_interval_minutes: subscription.update_policy.interval_minutes,
        active: state.active_subscription_id.as_ref() == Some(&subscription.id),
        active_configuration_generation: (state.active_subscription_id.as_ref()
            == Some(&subscription.id))
        .then_some(state.active_configuration_generation),
        shareable: matches!(subscription.source, SubscriptionSource::Remote { .. }),
    }
}

fn safe_display_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "本地订阅".to_owned();
    }
    trimmed.chars().take(80).collect()
}

fn validate_name(name: String) -> Result<String, SubscriptionOperationError> {
    let name = name.trim().to_owned();
    let length = name.chars().count();
    if !(1..=80).contains(&length) {
        return Err(SubscriptionOperationError::InvalidInput);
    }
    Ok(name)
}

fn validate_description(value: String) -> Result<String, SubscriptionOperationError> {
    let value = value.trim().to_owned();
    if value.chars().count() > 280 || value.chars().any(char::is_control) {
        return Err(SubscriptionOperationError::InvalidInput);
    }
    Ok(value)
}

fn validate_id(id: &str) -> Result<(), SubscriptionOperationError> {
    if id.is_empty() || id.len() > 128 || id.trim() != id {
        return Err(SubscriptionOperationError::InvalidInput);
    }
    Ok(())
}

fn validate_user_agent(
    value: Option<String>,
) -> Result<Option<String>, SubscriptionOperationError> {
    if value.as_ref().is_some_and(|value| {
        !(1..=256).contains(&value.len())
            || !value.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
    }) {
        return Err(SubscriptionOperationError::InvalidOptions);
    }
    Ok(value)
}

fn validate_remote_import_options(
    value: &RemoteImportOptions,
) -> Result<RemoteRequestOptions, SubscriptionOperationError> {
    if !(5..=120).contains(&value.timeout_seconds) {
        return Err(SubscriptionOperationError::InvalidOptions);
    }
    Ok(RemoteRequestOptions {
        user_agent: validate_user_agent(value.user_agent.clone())?,
        timeout_seconds: value.timeout_seconds,
        proxy_mode: value.proxy_mode,
        verify_tls: value.verify_tls,
    })
}

fn validate_update_policy(
    allow_auto_update: bool,
    interval_minutes: Option<u32>,
) -> Result<SubscriptionUpdatePolicy, SubscriptionOperationError> {
    if interval_minutes.is_some_and(|minutes| minutes < 1_440) {
        return Err(SubscriptionOperationError::InvalidOptions);
    }
    Ok(SubscriptionUpdatePolicy {
        allow_auto_update,
        interval_minutes,
    })
}

fn apply_remote_patch(
    value: &mut RemoteRequestOptions,
    patch: RemoteRequestPatch,
) -> Result<(), SubscriptionOperationError> {
    if let Some(user_agent) = patch.user_agent {
        value.user_agent = match user_agent {
            UserAgentEdit::Clear => None,
            UserAgentEdit::Set(value) => validate_user_agent(Some(value))?,
        };
    }
    if let Some(timeout) = patch.timeout_seconds {
        if !(5..=120).contains(&timeout) {
            return Err(SubscriptionOperationError::InvalidOptions);
        }
        value.timeout_seconds = timeout;
    }
    if let Some(proxy_mode) = patch.proxy_mode {
        value.proxy_mode = proxy_mode;
    }
    if let Some(verify_tls) = patch.verify_tls {
        value.verify_tls = verify_tls;
    }
    Ok(())
}

fn apply_update_policy_patch(
    value: &mut SubscriptionUpdatePolicy,
    patch: UpdatePolicyPatch,
) -> Result<(), SubscriptionOperationError> {
    let allow_auto_update = patch.allow_auto_update.unwrap_or(value.allow_auto_update);
    let interval_minutes = patch.interval_minutes.unwrap_or(value.interval_minutes);
    *value = validate_update_policy(allow_auto_update, interval_minutes)?;
    Ok(())
}

fn adopt_profile_interval(
    value: &mut SubscriptionUpdatePolicy,
    profile_update_interval_minutes: Option<u32>,
) {
    if value.allow_auto_update && value.interval_minutes.is_none() {
        value.interval_minutes = profile_update_interval_minutes;
    }
}

fn settings_for(subscription: &Subscription) -> SubscriptionSettings {
    let (url_preview, shareable) = match &subscription.source {
        SubscriptionSource::Remote { url } => (safe_url_preview(url), true),
        SubscriptionSource::Manual => (None, false),
    };
    SubscriptionSettings {
        id: subscription.id.0.clone(),
        name: safe_display_name(&subscription.name),
        description: subscription.description.clone(),
        source_kind: if shareable {
            SubscriptionSourceKind::Remote
        } else {
            SubscriptionSourceKind::Manual
        },
        url_preview,
        has_custom_user_agent: subscription
            .remote_request
            .as_ref()
            .and_then(|value| value.user_agent.as_ref())
            .is_some(),
        remote_request: subscription
            .remote_request
            .as_ref()
            .map(|value| RemoteRequestSettings {
                timeout_seconds: value.timeout_seconds,
                proxy_mode: value.proxy_mode,
                verify_tls: value.verify_tls,
            }),
        update_policy: UpdatePolicySettings {
            allow_auto_update: subscription.update_policy.allow_auto_update,
            interval_minutes: subscription.update_policy.interval_minutes,
        },
        shareable,
    }
}

fn safe_url_preview(value: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    url.host_str()?;
    Some(format!(
        "{}{}",
        url.origin().ascii_serialization(),
        url.path()
    ))
}

fn next_generation(current: u64) -> Result<u64, SubscriptionOperationError> {
    current
        .checked_add(1)
        .filter(|value| *value <= MAX_SAFE_INTEGER)
        .ok_or(SubscriptionOperationError::ValidationFailed)
}

fn parse_content(
    content: &str,
) -> Result<crate::subscription::ParseResult, SubscriptionOperationError> {
    if content.is_empty() || content.len() > MAX_CONTENT_BYTES {
        return Err(SubscriptionOperationError::InvalidInput);
    }
    if let Some(error) = clash_document_error(content) {
        return Err(error);
    }
    let mut parsed =
        parse_subscription(content).map_err(|_| SubscriptionOperationError::ParseFailed)?;
    // 不支持的参数随整个节点过滤；全部不可用时保留原订阅，不能保存空结果。
    if parsed.nodes.is_empty() {
        return Err(SubscriptionOperationError::ParseFailed);
    }
    deduplicate_parsed_nodes(&mut parsed);
    Ok(parsed)
}

fn deduplicate_parsed_nodes(parsed: &mut crate::subscription::ParseResult) {
    // 只在导入边界合并完整连接参数相同的项；保留首项名称和原稳定 ID。
    let mut connections = HashSet::new();
    parsed.nodes.retain(|node| {
        connections.insert(
            serde_json::to_vec(&(
                node.protocol,
                &node.server,
                node.port,
                &node.options,
                &node.transport,
                &node.tls,
            ))
            .expect("typed connection material serializes"),
        )
    });
}

fn document_for_body(
    content: String,
    format: crate::subscription::SubscriptionFormat,
) -> Option<SubscriptionDocument> {
    let format = match format {
        crate::subscription::SubscriptionFormat::Json => SubscriptionDocumentFormat::Json,
        crate::subscription::SubscriptionFormat::ClashYaml => SubscriptionDocumentFormat::Yaml,
        crate::subscription::SubscriptionFormat::UriList
        | crate::subscription::SubscriptionFormat::Base64 => return None,
    };
    Some(SubscriptionDocument {
        format,
        content,
        local_override: false,
    })
}

fn document_revision(document: &SubscriptionDocument) -> String {
    revision_for(document.format, &document.content)
}

fn revision_for(format: SubscriptionDocumentFormat, content: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"veyra-subscription-document-v1\0");
    digest.update(match format {
        SubscriptionDocumentFormat::Json => b"json\0".as_slice(),
        SubscriptionDocumentFormat::Yaml => b"yaml\0".as_slice(),
    });
    digest.update(content.as_bytes());
    format!("{:x}", digest.finalize())
}

fn validate_document_revision(value: &str) -> Result<(), DocumentOperationError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(DocumentOperationError::InvalidInput)
    }
}

fn map_document_parse_error(error: DocumentError) -> DocumentOperationError {
    match error.code {
        DocumentErrorCode::InvalidInput => DocumentOperationError::InvalidInput,
        DocumentErrorCode::ContentTooLarge => DocumentOperationError::ContentTooLarge,
        DocumentErrorCode::FormatFailed => DocumentOperationError::FormatFailed,
        DocumentErrorCode::ParseFailed => DocumentOperationError::ParseFailed,
        DocumentErrorCode::UnsupportedNodes => DocumentOperationError::UnsupportedNodes,
        DocumentErrorCode::UnsupportedClashProviders => {
            DocumentOperationError::UnsupportedClashProviders
        }
        DocumentErrorCode::Busy => DocumentOperationError::Busy,
        DocumentErrorCode::OperationTimedOut => DocumentOperationError::OperationTimedOut,
    }
}

fn map_document_access_error(error: StateAccessError) -> DocumentOperationError {
    match error {
        StateAccessError::Busy => DocumentOperationError::Busy,
        StateAccessError::Unavailable => DocumentOperationError::StateUnavailable,
    }
}

fn map_document_subscription_error(error: SubscriptionOperationError) -> DocumentOperationError {
    match error {
        SubscriptionOperationError::InvalidInput | SubscriptionOperationError::InvalidOptions => {
            DocumentOperationError::InvalidInput
        }
        SubscriptionOperationError::Busy => DocumentOperationError::Busy,
        SubscriptionOperationError::ParseFailed => DocumentOperationError::ParseFailed,
        SubscriptionOperationError::NormalizationFailed => {
            DocumentOperationError::NormalizationFailed
        }
        SubscriptionOperationError::ValidationFailed => DocumentOperationError::ValidationFailed,
        SubscriptionOperationError::SaveFailed => DocumentOperationError::SaveFailed,
        SubscriptionOperationError::StateUnavailable => DocumentOperationError::StateUnavailable,
        SubscriptionOperationError::NotFound => DocumentOperationError::NotFound,
        SubscriptionOperationError::UnsupportedClashProviders => {
            DocumentOperationError::UnsupportedClashProviders
        }
        SubscriptionOperationError::FetchFailed
        | SubscriptionOperationError::CacheUnavailable
        | SubscriptionOperationError::IdentityFailed
        | SubscriptionOperationError::ProxyUnavailable
        | SubscriptionOperationError::SystemProxyUnavailable
        | SubscriptionOperationError::ReferenceConflict
        | SubscriptionOperationError::NotShareable => DocumentOperationError::ValidationFailed,
    }
}

fn map_document_replacement_error(error: ProviderReplacementError) -> DocumentOperationError {
    match error {
        ProviderReplacementError::RejectedBatch => DocumentOperationError::ParseFailed,
        ProviderReplacementError::MissingProvider | ProviderReplacementError::State(_) => {
            DocumentOperationError::ValidationFailed
        }
        ProviderReplacementError::Normalize(_) => DocumentOperationError::NormalizationFailed,
        ProviderReplacementError::Store(_) => DocumentOperationError::SaveFailed,
    }
}

fn clash_document_error(content: &str) -> Option<SubscriptionOperationError> {
    let document: serde_json::Value = serde_json::from_str(content)
        .or_else(|_| serde_yaml_ng::from_str(content))
        .ok()?;
    let has_external = document
        .get("proxy-providers")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|values| !values.is_empty());
    let proxies = document
        .get("proxies")
        .and_then(serde_json::Value::as_array);
    if has_external && proxies.is_none_or(Vec::is_empty) {
        return Some(SubscriptionOperationError::UnsupportedClashProviders);
    }
    None
}

fn merge_metadata(
    previous: SubscriptionHttpMetadata,
    response: SubscriptionHttpMetadata,
    validators_match_source: bool,
) -> SubscriptionHttpMetadata {
    SubscriptionHttpMetadata {
        etag: if validators_match_source {
            response.etag.or(previous.etag)
        } else {
            response.etag
        },
        last_modified: if validators_match_source {
            response.last_modified.or(previous.last_modified)
        } else {
            response.last_modified
        },
        subscription_userinfo: response
            .subscription_userinfo
            .or(previous.subscription_userinfo),
        content_disposition: response
            .content_disposition
            .or(previous.content_disposition),
    }
}

fn identity_suffix(bytes: [u8; 16]) -> String {
    let mut suffix = String::with_capacity(32);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut suffix, "{byte:02x}").expect("writing to String cannot fail");
    }
    suffix
}

fn system_identity() -> Result<[u8; 16], ()> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|_| ())?;
    Ok(bytes)
}

fn system_clock_ms() -> Result<u64, ()> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?
        .as_millis();
    u64::try_from(millis).map_err(|_| ())
}

fn map_access_error(error: StateAccessError) -> SubscriptionOperationError {
    match error {
        StateAccessError::Busy => SubscriptionOperationError::Busy,
        StateAccessError::Unavailable => SubscriptionOperationError::StateUnavailable,
    }
}

fn map_url_error(_: FetchError) -> SubscriptionOperationError {
    SubscriptionOperationError::InvalidInput
}

fn map_fetch_error(_: FetchError) -> SubscriptionOperationError {
    SubscriptionOperationError::FetchFailed
}

fn map_replacement_error(error: ProviderReplacementError) -> SubscriptionOperationError {
    match error {
        ProviderReplacementError::RejectedBatch => SubscriptionOperationError::ParseFailed,
        ProviderReplacementError::MissingProvider | ProviderReplacementError::State(_) => {
            SubscriptionOperationError::ValidationFailed
        }
        ProviderReplacementError::Normalize(_) => SubscriptionOperationError::NormalizationFailed,
        ProviderReplacementError::Store(_) => SubscriptionOperationError::SaveFailed,
    }
}

pub(crate) struct SubscriptionWriteGuard {
    flag: Arc<AtomicBool>,
}

impl Drop for SubscriptionWriteGuard {
    fn drop(&mut self) {
        self.flag.store(false, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        sync::{Arc, Mutex, atomic::AtomicU64, mpsc},
        thread,
        time::UNIX_EPOCH,
    };

    use super::*;

    const FIRST_BODY: &str = r#"{"outbounds":[{"type":"socks","tag":"first","server":"127.0.0.1","server_port":1080,"version":"5"}]}"#;
    const SECOND_BODY: &str = r#"{"outbounds":[{"type":"socks","tag":"second","server":"127.0.0.1","server_port":1081,"version":"5"}]}"#;

    fn fixed_identity() -> Result<[u8; 16], ()> {
        Ok([0x11; 16])
    }

    fn failing_identity() -> Result<[u8; 16], ()> {
        Err(())
    }

    fn fixed_clock() -> Result<u64, ()> {
        Ok(1_234)
    }

    static SCHEDULER_CLOCK: AtomicU64 = AtomicU64::new(1_000);

    fn scheduler_clock() -> Result<u64, ()> {
        Ok(SCHEDULER_CLOCK.load(Ordering::Acquire))
    }

    fn isolated_state_file(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "veyra-task011-{label}-{}-{nanos}",
                std::process::id()
            ))
            .join("state.json")
    }

    fn manager(path: PathBuf) -> SubscriptionManager {
        SubscriptionManager::with_test_sources(
            path,
            StateAccessGate::default(),
            fixed_identity,
            fixed_clock,
        )
    }

    #[test]
    fn manual_import_creates_one_atomic_identity_graph_and_reloads_it() {
        let state_file = isolated_state_file("manual-import");
        let manager = manager(state_file.clone());

        let summary = tauri::async_runtime::block_on(manager.import(
            "  Fixture  ".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("manual import succeeds");

        assert_eq!(summary.name, "Fixture");
        assert_eq!(summary.id, format!("sub-{}", "11".repeat(16)));
        assert_eq!(summary.source_kind, SubscriptionSourceKind::Manual);
        assert_eq!(summary.node_count, 1);
        assert_eq!(summary.last_success_at_ms, Some(1_234));
        let loaded = JsonStateStore::new(state_file.clone())
            .expect("store")
            .load()
            .expect("reload committed state");
        assert_eq!(loaded.subscriptions[0].id.0, summary.id);
        assert_eq!(
            loaded.providers[0].subscription_id,
            loaded.subscriptions[0].id
        );
        assert_eq!(
            loaded.pools[0].sources[0].provider_id,
            loaded.providers[0].id
        );
        assert!(matches!(
            loaded.default_target,
            crate::domain::RouteTarget::Unconfigured
        ));
        let document = loaded.subscriptions[0]
            .document
            .as_ref()
            .expect("JSON source retained");
        assert_eq!(document.format, SubscriptionDocumentFormat::Json);
        assert_eq!(document.content, FIRST_BODY);
        assert!(!document.local_override);
        let view = manager
            .read_document(&summary.id)
            .expect("read retained document");
        assert_eq!(view.content, FIRST_BODY);
        assert_eq!(view.revision, document_revision(document));
        let serialized = fs::read_to_string(&state_file).expect("read state");
        assert!(serialized.contains("document"));
        assert!(serialized.contains("outbounds"));
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn failed_update_preserves_nodes_time_and_persisted_bytes() {
        let state_file = isolated_state_file("failed-update");
        let manager = manager(state_file.clone());
        let summary = tauri::async_runtime::block_on(manager.import(
            "Fixture".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("initial import");
        let before = fs::read(&state_file).expect("read initial state");

        let result = tauri::async_runtime::block_on(
            manager.update(summary.id, Some("not a subscription".to_owned())),
        );

        assert_eq!(result, Err(SubscriptionOperationError::ParseFailed));
        assert_eq!(fs::read(&state_file).expect("read unchanged state"), before);
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn strict_document_save_preserves_exact_yaml_and_commits_by_revision() {
        let state_file = isolated_state_file("strict-document-save");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Fixture".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("initial import");
        let before = manager
            .read_document(&imported.id)
            .expect("read initial document");
        let edited = "# keep comment\nproxies:\n  - name: edited\n    type: socks5\n    server: edited.invalid\n    port: 1081\n";

        let prepared = manager
            .prepare_document_save(DocumentSaveInput {
                id: imported.id.clone(),
                format: SubscriptionDocumentFormat::Yaml,
                content: edited.to_owned(),
                expected_revision: before.revision,
            })
            .expect("prepare strict document");
        assert!(!prepared.is_selected());
        assert!(!prepared.requires_apply());
        assert_eq!(prepared.next().nodes[0].name, "edited");
        assert_eq!(prepared.before().subscriptions[0].id.0, imported.id);
        assert_eq!(prepared.summary().name, "Fixture");
        let expected_revision = prepared.document_revision().to_owned();
        let committed = manager
            .commit_prepared_document(&prepared)
            .expect("commit exact document");

        assert_eq!(committed.document_revision, expected_revision);
        assert_eq!(committed.subscription.id, imported.id);
        let readback = manager
            .read_document(&imported.id)
            .expect("read saved document");
        assert_eq!(readback.content, edited);
        assert!(readback.content.starts_with("# keep comment"));
        assert_eq!(readback.format, SubscriptionDocumentFormat::Yaml);
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn strict_document_save_rejects_unsupported_nodes_and_cas_conflict() {
        let state_file = isolated_state_file("strict-document-reject");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Fixture".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("initial import");
        let initial = manager
            .read_document(&imported.id)
            .expect("read initial document");
        let before = fs::read(&state_file).expect("read initial bytes");
        let mixed = "proxies:\n  - name: accepted\n    type: socks5\n    server: accepted.invalid\n    port: 1080\n  - name: rejected\n    type: unknown\n    server: rejected.invalid\n    port: 443\n";
        assert!(matches!(
            manager.prepare_document_save(DocumentSaveInput {
                id: imported.id.clone(),
                format: SubscriptionDocumentFormat::Yaml,
                content: mixed.to_owned(),
                expected_revision: initial.revision.clone(),
            }),
            Err(DocumentOperationError::UnsupportedNodes)
        ));
        assert_eq!(fs::read(&state_file).expect("unchanged bytes"), before);

        let duplicate = "proxies:\n  - name: first\n    type: socks5\n    server: duplicate.invalid\n    port: 1080\n  - name: second\n    type: socks5\n    server: duplicate.invalid\n    port: 1080\n";
        assert!(matches!(
            manager.prepare_document_save(DocumentSaveInput {
                id: imported.id.clone(),
                format: SubscriptionDocumentFormat::Yaml,
                content: duplicate.to_owned(),
                expected_revision: initial.revision.clone(),
            }),
            Err(DocumentOperationError::NormalizationFailed)
        ));
        assert_eq!(fs::read(&state_file).expect("unchanged bytes"), before);

        let prepared = manager
            .prepare_document_save(DocumentSaveInput {
                id: imported.id.clone(),
                format: SubscriptionDocumentFormat::Json,
                content: SECOND_BODY.to_owned(),
                expected_revision: initial.revision,
            })
            .expect("prepare valid edit");
        let mut concurrent = prepared.before().clone();
        concurrent.subscriptions[0].description = "concurrent".to_owned();
        manager
            .store
            .save(&concurrent)
            .expect("write concurrent state");
        assert_eq!(
            manager.commit_prepared_document(&prepared),
            Err(DocumentOperationError::DocumentConflict)
        );
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn selected_document_change_advances_generation_and_requires_apply() {
        let state_file = isolated_state_file("selected-document-save");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Fixture".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("initial import");
        let mut selected = manager.store.load().expect("load state");
        selected.active_subscription_id = Some(SubscriptionId(imported.id.clone()));
        selected.active_configuration_generation = 4;
        manager.store.save(&selected).expect("save selected state");
        let document = manager
            .read_document(&imported.id)
            .expect("read initial document");

        let unchanged = manager
            .prepare_document_save(DocumentSaveInput {
                id: imported.id.clone(),
                format: SubscriptionDocumentFormat::Json,
                content: FIRST_BODY.to_owned(),
                expected_revision: document.revision.clone(),
            })
            .expect("prepare exact no-op");
        assert!(unchanged.is_selected());
        assert!(!unchanged.requires_apply());
        assert_eq!(unchanged.generation(), 4);
        assert_eq!(unchanged.next(), unchanged.before());
        drop(unchanged);

        let prepared = manager
            .prepare_document_save(DocumentSaveInput {
                id: imported.id,
                format: SubscriptionDocumentFormat::Json,
                content: SECOND_BODY.to_owned(),
                expected_revision: document.revision,
            })
            .expect("prepare selected edit");

        assert!(prepared.is_selected());
        assert!(prepared.requires_apply());
        assert_eq!(prepared.generation(), 5);
        assert_eq!(prepared.next().active_configuration_generation, 5);
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn update_retains_subscription_provider_and_pool_identity() {
        let state_file = isolated_state_file("identity-retention");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Fixture".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("initial import");
        let before = manager.store.load().expect("load before update");

        let updated = tauri::async_runtime::block_on(
            manager.update(imported.id, Some(SECOND_BODY.to_owned())),
        )
        .expect("manual update");
        let after = manager.store.load().expect("load after update");

        assert!(updated.content_changed);
        assert_eq!(before.subscriptions[0].id, after.subscriptions[0].id);
        assert_eq!(before.providers[0].id, after.providers[0].id);
        assert_eq!(before.pools[0].id, after.pools[0].id);
        assert_eq!(after.nodes[0].name, "second");
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn identity_failure_and_collision_do_not_commit_or_retry() {
        let failed_file = isolated_state_file("identity-failure");
        let failed = SubscriptionManager::with_test_sources(
            failed_file.clone(),
            StateAccessGate::default(),
            failing_identity,
            fixed_clock,
        );
        assert_eq!(
            tauri::async_runtime::block_on(failed.import(
                "Fixture".to_owned(),
                ImportSource::Manual {
                    content: FIRST_BODY.to_owned(),
                },
            )),
            Err(SubscriptionOperationError::IdentityFailed)
        );
        assert!(!failed_file.exists());

        let collision_file = isolated_state_file("identity-collision");
        let collision = manager(collision_file.clone());
        tauri::async_runtime::block_on(collision.import(
            "First".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("first import");
        let before = fs::read(&collision_file).expect("read first state");
        assert_eq!(
            tauri::async_runtime::block_on(collision.import(
                "Second".to_owned(),
                ImportSource::Manual {
                    content: SECOND_BODY.to_owned(),
                },
            )),
            Err(SubscriptionOperationError::IdentityFailed)
        );
        assert_eq!(
            fs::read(&collision_file).expect("read unchanged state"),
            before
        );
        fs::remove_dir_all(collision_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn concurrent_subscription_write_is_rejected_immediately() {
        let state_file = isolated_state_file("busy");
        let manager = manager(state_file.clone());
        let _held = manager.begin_write().expect("hold first write");

        assert_eq!(
            tauri::async_runtime::block_on(manager.import(
                "Fixture".to_owned(),
                ImportSource::Manual {
                    content: FIRST_BODY.to_owned(),
                },
            )),
            Err(SubscriptionOperationError::Busy)
        );
        assert!(!state_file.exists());
    }

    #[test]
    fn remote_200_commit_and_304_refresh_share_one_persisted_cache() {
        let state_file = isolated_state_file("remote-cache");
        let manager = manager(state_file.clone());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture");
        let url = format!(
            "http://{}/subscription",
            listener.local_addr().expect("fixture address")
        );
        let fixture_body = include_str!("../../tests/fixtures/subscriptions/clash-compatible.yaml");
        let mut fixture: serde_json::Value = serde_yaml_ng::from_str(fixture_body).unwrap();
        let nodes = fixture["proxies"].as_array_mut().unwrap();
        let mut duplicate = nodes[0].clone();
        duplicate["name"] = serde_json::json!("duplicate alias");
        nodes.push(duplicate);
        let mut pinned = nodes[0].clone();
        pinned["fingerprint"] = "a".repeat(64).into();
        nodes.push(pinned);
        nodes.push(serde_json::json!({"type":"unknown"}));
        let fixture_body = fixture.to_string();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let server = thread::spawn(move || {
            for response in [
                format!(
                    "HTTP/1.1 200 OK\r\nETag: \"v1\"\r\nSubscription-Userinfo: upload=1; download=2; total=10; expire=3\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{fixture_body}",
                    fixture_body.len()
                ),
                "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nProfile-Update-Interval: 24\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut bytes = [0_u8; 8_192];
                let count = stream.read(&mut bytes).expect("read request");
                captured
                    .lock()
                    .expect("request lock")
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                stream.write_all(response.as_bytes()).expect("write response");
            }
        });

        let imported = tauri::async_runtime::block_on(
            manager.import("Remote".to_owned(), ImportSource::Remote { url }),
        )
        .expect("remote import");
        let imported_document = manager
            .read_document(&imported.id)
            .expect("remote 200 document");
        assert_eq!(imported_document.format, SubscriptionDocumentFormat::Json);
        assert!(!imported_document.local_override);
        let updated = tauri::async_runtime::block_on(manager.update(imported.id.clone(), None))
            .expect("304 refresh");
        server.join().expect("server exits");

        assert_eq!(imported.source_kind, SubscriptionSourceKind::Remote);
        assert_eq!(
            imported.traffic,
            Some(SubscriptionTraffic {
                upload: Some(1),
                download: Some(2),
                total: Some(10),
                expire_at_ms: Some(3_000),
            })
        );
        assert_eq!((imported.node_count, imported.skipped_node_count), (10, 3));
        assert_eq!(updated.subscription.skipped_node_count, 3);
        assert!(!updated.content_changed);
        assert_eq!(updated.subscription.id, imported.id);
        assert_eq!(updated.subscription.update_interval_minutes, Some(1_440));
        assert_eq!(
            manager
                .read_document(&imported.id)
                .expect("304 retained document")
                .revision,
            imported_document.revision
        );
        assert_eq!(
            manager.store.load().expect("reload interval").subscriptions[0]
                .update_policy
                .interval_minutes,
            Some(1_440)
        );
        let requests = requests.lock().expect("request lock");
        assert!(!requests[0].contains("if-none-match"));
        assert!(requests[1].contains("if-none-match: \"v1\""));
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn remote_local_document_override_suppresses_validators_and_rejects_304() {
        SCHEDULER_CLOCK.store(1_000, Ordering::Release);
        let state_file = isolated_state_file("remote-document-override");
        let manager = SubscriptionManager::with_test_sources(
            state_file.clone(),
            StateAccessGate::default(),
            fixed_identity,
            scheduler_clock,
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture");
        let url = format!(
            "http://{}/subscription",
            listener.local_addr().expect("fixture address")
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let server = thread::spawn(move || {
            for response in [
                format!(
                    "HTTP/1.1 200 OK\r\nETag: \"v1\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                    FIRST_BODY.len()
                ),
                "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
                format!(
                    "HTTP/1.1 200 OK\r\nETag: \"v2\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                    FIRST_BODY.len()
                ),
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut bytes = [0_u8; 8_192];
                let count = stream.read(&mut bytes).expect("read request");
                captured
                    .lock()
                    .expect("request lock")
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                stream.write_all(response.as_bytes()).expect("write response");
            }
        });

        let imported = tauri::async_runtime::block_on(
            manager.import("Remote".to_owned(), ImportSource::Remote { url }),
        )
        .expect("remote import");
        let initial = manager
            .read_document(&imported.id)
            .expect("read imported source");
        let prepared = manager
            .prepare_document_save(DocumentSaveInput {
                id: imported.id.clone(),
                format: SubscriptionDocumentFormat::Json,
                content: SECOND_BODY.to_owned(),
                expected_revision: initial.revision,
            })
            .expect("prepare local override");
        manager
            .commit_prepared_document(&prepared)
            .expect("commit local override");
        drop(prepared);

        SCHEDULER_CLOCK.store(2_000, Ordering::Release);
        assert_eq!(
            tauri::async_runtime::block_on(manager.update(imported.id.clone(), None)),
            Err(SubscriptionOperationError::CacheUnavailable)
        );
        let retained = manager.store.load().expect("load retained override");
        let document = retained.subscriptions[0]
            .document
            .as_ref()
            .expect("document retained");
        assert_eq!(document.content, SECOND_BODY);
        assert!(document.local_override);
        assert_eq!(retained.subscriptions[0].last_attempt_at_ms, Some(2_000));
        assert_eq!(
            retained.subscriptions[0]
                .http_metadata
                .as_ref()
                .and_then(|metadata| metadata.etag.as_deref()),
            None
        );
        SCHEDULER_CLOCK.store(3_000, Ordering::Release);
        tauri::async_runtime::block_on(manager.update(imported.id.clone(), None))
            .expect("later 200 replaces override");
        server.join().expect("server exits");
        let replaced = manager.store.load().expect("load remote replacement");
        let document = replaced.subscriptions[0]
            .document
            .as_ref()
            .expect("replacement document");
        assert_eq!(document.content, FIRST_BODY);
        assert!(!document.local_override);
        assert_eq!(
            replaced.subscriptions[0]
                .http_metadata
                .as_ref()
                .and_then(|metadata| metadata.etag.as_deref()),
            Some("\"v2\"")
        );
        let requests = requests.lock().expect("request lock");
        assert!(!requests[1].contains("if-none-match"));
        assert!(!requests[1].contains("if-modified-since"));
        assert!(!requests[2].contains("if-none-match"));
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn save_failure_keeps_the_previous_committed_state_and_time() {
        let state_file = isolated_state_file("save-failure");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Fixture".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("initial import");
        let before = fs::read(&state_file).expect("read initial state");
        fs::create_dir(state_file.with_extension("tmp")).expect("block atomic temp file");

        let result = tauri::async_runtime::block_on(
            manager.update(imported.id, Some(SECOND_BODY.to_owned())),
        );

        assert_eq!(result, Err(SubscriptionOperationError::SaveFailed));
        assert_eq!(fs::read(&state_file).expect("read unchanged state"), before);
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn remote_wait_releases_state_gate_and_reload_rejects_changed_cache() {
        let state_file = isolated_state_file("reload-mismatch");
        let manager = Arc::new(manager(state_file.clone()));
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback fixture");
        let url = format!(
            "http://{}/subscription",
            listener.local_addr().expect("fixture address")
        );
        let (accepted, accepted_rx) = mpsc::sync_channel(1);
        let (release, release_rx) = mpsc::sync_channel(1);
        let server = thread::spawn(move || {
            for (index, body) in [FIRST_BODY, SECOND_BODY].into_iter().enumerate() {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut bytes = [0_u8; 8_192];
                let _ = stream.read(&mut bytes).expect("read request");
                if index == 1 {
                    accepted.send(()).expect("signal network wait");
                    release_rx.recv().expect("release update response");
                }
                let response = format!(
                    "HTTP/1.1 200 OK\r\nETag: \"v{}\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    index + 1,
                    body.len()
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write response");
            }
        });
        let imported = tauri::async_runtime::block_on(
            manager.import("Remote".to_owned(), ImportSource::Remote { url }),
        )
        .expect("remote import");
        let update_manager = Arc::clone(&manager);
        let update_id = imported.id.clone();
        let update = thread::spawn(move || {
            tauri::async_runtime::block_on(update_manager.update(update_id, None))
        });
        accepted_rx.recv().expect("update is waiting on HTTP");

        assert_eq!(manager.list().expect("list during HTTP wait").len(), 1);
        {
            let _access = manager
                .gate
                .try_lock()
                .expect("state gate is not held by HTTP");
            let mut concurrent = manager.store.load().expect("load concurrent state");
            concurrent.subscriptions[0]
                .http_metadata
                .as_mut()
                .expect("remote metadata")
                .etag = Some("\"concurrent\"".to_owned());
            manager
                .store
                .save(&concurrent)
                .expect("save concurrent state");
        }
        release.send(()).expect("release update response");

        assert_eq!(
            update.join().expect("update joins"),
            Err(SubscriptionOperationError::Busy)
        );
        server.join().expect("server exits");
        let final_state = manager.store.load().expect("load final state");
        assert_eq!(
            final_state.subscriptions[0]
                .http_metadata
                .as_ref()
                .and_then(|metadata| metadata.etag.as_deref()),
            Some("\"concurrent\"")
        );
        assert_eq!(final_state.nodes[0].name, "first");
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn legacy_v1_v2_v3_names_migrate_restart_and_render_without_rewriting() {
        for version in [1_u64, 2, 3] {
            for original_name in [String::new(), "订".repeat(81)] {
                let state_file = isolated_state_file(&format!("legacy-name-{version}"));
                fs::create_dir_all(state_file.parent().expect("state parent"))
                    .expect("create state parent");
                fs::write(
                    &state_file,
                    serde_json::to_vec(&legacy_document(version, &original_name))
                        .expect("encode legacy state"),
                )
                .expect("write legacy state");
                let initial_manager = manager(state_file.clone());

                let first = initial_manager
                    .list()
                    .expect("migrate and list legacy state");
                assert_eq!(first.len(), 1);
                let expected_display = if original_name.is_empty() {
                    "本地订阅".to_owned()
                } else {
                    original_name.chars().take(80).collect()
                };
                assert_eq!(first[0].name, expected_display);
                assert_eq!(first[0].id, "subscription");
                assert_eq!(first[0].source_kind, SubscriptionSourceKind::Manual);
                let persisted = initial_manager.store.load().expect("read migrated state");
                assert_eq!(persisted.subscriptions[0].name, original_name);
                assert_eq!(persisted.subscriptions[0].id.0, "subscription");
                assert_eq!(persisted.providers[0].subscription_id.0, "subscription");
                drop(initial_manager);

                let restarted = manager(state_file.clone());
                assert_eq!(
                    restarted.list().expect("restart list")[0].name,
                    expected_display
                );
                assert_eq!(
                    restarted.store.load().expect("restart load").subscriptions[0].name,
                    original_name
                );
                fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
            }
        }
    }

    #[test]
    fn new_import_names_remain_strict_even_when_legacy_names_are_display_safe() {
        let state_file = isolated_state_file("strict-new-name");
        let manager = manager(state_file.clone());
        for invalid_name in ["   ".to_owned(), "订".repeat(81)] {
            assert_eq!(
                tauri::async_runtime::block_on(manager.import(
                    invalid_name,
                    ImportSource::Manual {
                        content: FIRST_BODY.to_owned(),
                    },
                )),
                Err(SubscriptionOperationError::InvalidInput)
            );
        }
        assert!(!state_file.exists());
    }

    #[test]
    fn first_remote_creation_loses_safely_to_a_new_state_created_during_http() {
        let state_file = isolated_state_file("first-create-race");
        let gate = StateAccessGate::default();
        let remote_manager = Arc::new(SubscriptionManager::with_test_sources(
            state_file.clone(),
            gate.clone(),
            fixed_identity,
            fixed_clock,
        ));
        let competing_manager = SubscriptionManager::with_test_sources(
            state_file.clone(),
            gate,
            fixed_identity,
            fixed_clock,
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind race fixture");
        let url = format!(
            "http://{}/subscription",
            listener.local_addr().expect("fixture address")
        );
        let (accepted, accepted_rx) = mpsc::sync_channel(1);
        let (release, release_rx) = mpsc::sync_channel(1);
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept remote import");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read remote import");
            accepted.send(()).expect("signal HTTP wait");
            release_rx.recv().expect("release response");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                FIRST_BODY.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });
        let importing = Arc::clone(&remote_manager);
        let remote = thread::spawn(move || {
            tauri::async_runtime::block_on(
                importing.import("Remote".to_owned(), ImportSource::Remote { url }),
            )
        });
        accepted_rx.recv().expect("remote waits on HTTP");

        let competing = tauri::async_runtime::block_on(competing_manager.import(
            "Manual".to_owned(),
            ImportSource::Manual {
                content: SECOND_BODY.to_owned(),
            },
        ))
        .expect("competing first creation commits");
        release.send(()).expect("release remote response");
        assert_eq!(
            remote.join().expect("remote import joins"),
            Err(SubscriptionOperationError::Busy)
        );
        server.join().expect("server exits");

        let final_state = competing_manager.store.load().expect("load winning state");
        assert_eq!(final_state.subscriptions.len(), 1);
        assert_eq!(final_state.subscriptions[0].id.0, competing.id);
        assert_eq!(final_state.subscriptions[0].name, "Manual");
        assert_eq!(final_state.nodes[0].name, "second");
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn list_and_v3_migration_wait_behind_the_same_fail_fast_gate() {
        let state_file = isolated_state_file("migration-gate");
        fs::create_dir_all(state_file.parent().expect("state parent"))
            .expect("create state parent");
        fs::write(
            &state_file,
            serde_json::to_vec(&legacy_document(3, "Legacy")).expect("encode v3 state"),
        )
        .expect("write v3 state");
        let gate = StateAccessGate::default();
        let manager = SubscriptionManager::with_test_sources(
            state_file.clone(),
            gate.clone(),
            fixed_identity,
            fixed_clock,
        );
        let commit = gate.try_lock().expect("hold simulated commit gate");

        assert_eq!(manager.list(), Err(SubscriptionOperationError::Busy));
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &fs::read(&state_file).expect("read blocked v3")
            )
            .expect("decode blocked v3")["schema_version"],
            3
        );
        drop(commit);

        assert_eq!(manager.list().expect("list after gate release").len(), 1);
        assert_eq!(
            manager
                .store
                .load()
                .expect("load migrated state")
                .schema_version,
            crate::domain::CURRENT_SCHEMA_VERSION
        );
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn scheduler_due_time_uses_last_attempt_and_failed_fetch_waits_a_full_period() {
        let state_file = isolated_state_file("scheduler-attempt");
        SCHEDULER_CLOCK.store(1_000, Ordering::Release);
        let manager = SubscriptionManager::with_test_sources(
            state_file.clone(),
            StateAccessGate::default(),
            fixed_identity,
            scheduler_clock,
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind import fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept import");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read import");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                FIRST_BODY.len()
            );
            stream.write_all(response.as_bytes()).expect("write import");
        });
        let imported =
            tauri::async_runtime::block_on(manager.import_with_options(SubscriptionImport {
                name: "Scheduled".to_owned(),
                description: String::new(),
                source: ImportRequestSource::Remote {
                    url: format!("http://{address}/subscription"),
                    options: RemoteImportOptions {
                        interval_minutes: Some(1_440),
                        ..RemoteImportOptions::default()
                    },
                },
            }))
            .expect("remote import");
        server.join().expect("fixture exit");
        let period = 1_440_u64 * 60_000;
        assert!(
            manager
                .due_subscription_ids(1_000 + period - 1)
                .expect("due")
                .is_empty()
        );
        assert_eq!(
            manager.due_subscription_ids(1_000 + period).expect("due"),
            vec![imported.id.clone()]
        );

        SCHEDULER_CLOCK.store(2_000, Ordering::Release);
        assert_eq!(
            tauri::async_runtime::block_on(manager.update(imported.id.clone(), None)),
            Err(SubscriptionOperationError::FetchFailed)
        );
        let state = manager.store.load().expect("load failed attempt marker");
        assert_eq!(state.subscriptions[0].last_attempt_at_ms, Some(2_000));
        assert_eq!(state.subscriptions[0].last_success_at_ms, Some(1_000));
        assert!(
            manager
                .due_subscription_ids(2_000 + period - 1)
                .expect("due")
                .is_empty()
        );
        assert_eq!(
            manager.due_subscription_ids(2_000 + period).expect("due"),
            vec![imported.id]
        );
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn failed_attempt_save_uses_a_process_deadline_before_the_next_scheduler_retry() {
        let state_file = isolated_state_file("scheduler-save-failure");
        SCHEDULER_CLOCK.store(1_000, Ordering::Release);
        let manager = SubscriptionManager::with_test_sources(
            state_file.clone(),
            StateAccessGate::default(),
            fixed_identity,
            scheduler_clock,
        );
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind import fixture");
        let address = listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept import");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read import");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                FIRST_BODY.len()
            );
            stream.write_all(response.as_bytes()).expect("write import");
        });
        let imported =
            tauri::async_runtime::block_on(manager.import_with_options(SubscriptionImport {
                name: "Scheduled".to_owned(),
                description: String::new(),
                source: ImportRequestSource::Remote {
                    url: format!("http://{address}/subscription"),
                    options: RemoteImportOptions {
                        interval_minutes: Some(1_440),
                        ..RemoteImportOptions::default()
                    },
                },
            }))
            .expect("remote import");
        server.join().expect("fixture exit");
        let before = fs::read(&state_file).expect("read initial state");
        fs::create_dir(state_file.with_extension("tmp")).expect("block atomic temp file");

        SCHEDULER_CLOCK.store(2_000, Ordering::Release);
        assert_eq!(
            tauri::async_runtime::block_on(manager.update(imported.id.clone(), None)),
            Err(SubscriptionOperationError::SaveFailed)
        );
        assert_eq!(fs::read(&state_file).expect("read unchanged state"), before);
        fs::remove_dir(state_file.with_extension("tmp")).expect("remove save blocker");

        let period = 1_440_u64 * 60_000;
        for tick_at in [1_000 + period, 2_000 + period - 1] {
            let outcomes = tauri::async_runtime::block_on(
                crate::application::subscription_scheduler::run_scheduler_tick(&manager, tick_at),
            )
            .expect("suppressed scheduler tick");
            assert!(outcomes.is_empty());
        }
        assert_eq!(
            manager.due_subscription_ids(2_000 + period).expect("due"),
            vec![imported.id]
        );
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn profile_interval_is_adopted_only_when_the_user_left_the_interval_empty() {
        let mut empty = SubscriptionUpdatePolicy {
            allow_auto_update: true,
            interval_minutes: None,
        };
        adopt_profile_interval(&mut empty, Some(1_440));
        assert_eq!(empty.interval_minutes, Some(1_440));

        let mut explicit = SubscriptionUpdatePolicy {
            allow_auto_update: true,
            interval_minutes: Some(2_880),
        };
        adopt_profile_interval(&mut explicit, Some(1_440));
        assert_eq!(explicit.interval_minutes, Some(2_880));

        let mut disabled = SubscriptionUpdatePolicy {
            allow_auto_update: false,
            interval_minutes: None,
        };
        adopt_profile_interval(&mut disabled, Some(1_440));
        assert_eq!(disabled.interval_minutes, None);
    }

    #[test]
    fn cross_origin_redirect_validators_are_not_persisted_or_sent_to_the_source_later() {
        let state_file = isolated_state_file("redirect-validator");
        let manager = manager(state_file.clone());
        let source = TcpListener::bind("127.0.0.1:0").expect("bind source");
        let source_address = source.local_addr().expect("source address");
        let target = TcpListener::bind("127.0.0.1:0").expect("bind target");
        let target_address = target.local_addr().expect("target address");
        let source_requests = Arc::new(Mutex::new(Vec::new()));
        let captured_source = Arc::clone(&source_requests);
        let source_server = thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = source.accept().expect("accept source request");
                let mut bytes = [0_u8; 8_192];
                let count = stream.read(&mut bytes).expect("read source request");
                captured_source
                    .lock()
                    .expect("source request lock")
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                let response = format!(
                    "HTTP/1.1 302 Found\r\nLocation: http://{target_address}/target\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream
                    .write_all(response.as_bytes())
                    .expect("write redirect");
            }
        });
        let target_server = thread::spawn(move || {
            for response in [
                format!(
                    "HTTP/1.1 200 OK\r\nETag: \"target-v1\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                    FIRST_BODY.len()
                ),
                "HTTP/1.1 304 Not Modified\r\nETag: \"target-v2\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
            ] {
                let (mut stream, _) = target.accept().expect("accept target request");
                let mut bytes = [0_u8; 8_192];
                let _ = stream.read(&mut bytes).expect("read target request");
                stream.write_all(response.as_bytes()).expect("write target");
            }
        });

        let imported = tauri::async_runtime::block_on(manager.import(
            "Remote".to_owned(),
            ImportSource::Remote {
                url: format!("http://{source_address}/subscription"),
            },
        ))
        .expect("redirected import");
        let imported_state = manager.store.load().expect("load imported state");
        assert_eq!(
            imported_state.subscriptions[0]
                .http_metadata
                .as_ref()
                .and_then(|metadata| metadata.etag.as_deref()),
            None
        );
        tauri::async_runtime::block_on(manager.update(imported.id, None))
            .expect("redirected update");
        source_server.join().expect("source exits");
        target_server.join().expect("target exits");

        let final_state = manager.store.load().expect("load final state");
        assert_eq!(
            final_state.subscriptions[0]
                .http_metadata
                .as_ref()
                .and_then(|metadata| metadata.etag.as_deref()),
            None
        );
        for request in &*source_requests.lock().expect("source requests lock") {
            assert!(!request.contains("if-none-match"));
            assert!(!request.contains("if-modified-since"));
        }
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn cancelling_an_http_update_releases_the_owned_write_guard_without_attempt_commit() {
        let state_file = isolated_state_file("cancelled-http");
        let manager = manager(state_file.clone());
        let import_listener = TcpListener::bind("127.0.0.1:0").expect("bind import fixture");
        let address = import_listener.local_addr().expect("fixture address");
        let server = thread::spawn(move || {
            let (mut stream, _) = import_listener.accept().expect("accept import");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read import");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                FIRST_BODY.len()
            );
            stream.write_all(response.as_bytes()).expect("write import");
            import_listener
                .set_nonblocking(true)
                .expect("nonblocking stalled fixture");
            let deadline = std::time::Instant::now() + Duration::from_secs(1);
            while std::time::Instant::now() < deadline {
                match import_listener.accept() {
                    Ok((mut stream, _)) => {
                        stream
                            .set_nonblocking(false)
                            .expect("blocking accepted update socket");
                        let _ = stream.read(&mut bytes).expect("read stalled update");
                        thread::sleep(Duration::from_millis(150));
                        return;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("accept stalled update: {error}"),
                }
            }
            panic!("cancelled update never reached loopback fixture");
        });
        let imported = tauri::async_runtime::block_on(manager.import(
            "Remote".to_owned(),
            ImportSource::Remote {
                url: format!("http://{address}/subscription"),
            },
        ))
        .expect("import");
        let before = fs::read(&state_file).expect("read state before cancellation");
        let elapsed = tauri::async_runtime::block_on(async {
            tokio::time::timeout(Duration::from_millis(40), manager.update(imported.id, None)).await
        });
        assert!(elapsed.is_err(), "outer cancellation must win");
        let guard = manager
            .begin_write_owned()
            .expect("cancelled future releases write ownership");
        drop(guard);
        assert_eq!(fs::read(&state_file).expect("read unchanged state"), before);
        server.join().expect("fixture exit");
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn settings_hide_custom_user_agent_and_query_while_share_requires_explicit_read() {
        let state_file = isolated_state_file("safe-settings");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Local".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("manual import");
        let mut state = manager.store.load().expect("load state");
        state.subscriptions[0].source = SubscriptionSource::Remote {
            url: "https://example.invalid/sub?token=sensitive#fragment".to_owned(),
        };
        state.subscriptions[0].remote_request = Some(RemoteRequestOptions {
            user_agent: Some("private-client-name".to_owned()),
            ..RemoteRequestOptions::default_remote()
        });
        state.subscriptions[0].update_policy = SubscriptionUpdatePolicy::default_remote();
        manager.store.save(&state).expect("save remote fixture");

        let settings = manager.get_settings(&imported.id).expect("settings");
        assert!(settings.has_custom_user_agent);
        assert_eq!(
            settings.url_preview.as_deref(),
            Some("https://example.invalid/sub")
        );
        let rendered = format!("{settings:?}");
        assert!(!rendered.contains("private-client-name"));
        assert!(!rendered.contains("sensitive"));
        assert_eq!(
            manager.share_url(&imported.id).expect("explicit share"),
            "https://example.invalid/sub?token=sensitive#fragment"
        );
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn edit_patch_preserves_omitted_user_agent_and_clear_removes_it_without_node_generation() {
        let state_file = isolated_state_file("edit-settings");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Manual".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("manual import");
        let mut state = manager.store.load().expect("load state");
        state.subscriptions[0].source = SubscriptionSource::Remote {
            url: "https://example.invalid/sub".to_owned(),
        };
        state.subscriptions[0].remote_request = Some(RemoteRequestOptions {
            user_agent: Some("custom-agent".to_owned()),
            ..RemoteRequestOptions::default_remote()
        });
        state.subscriptions[0].update_policy = SubscriptionUpdatePolicy::default_remote();
        state.active_subscription_id = Some(state.subscriptions[0].id.clone());
        state.active_configuration_generation = 7;
        manager.store.save(&state).expect("save remote fixture");

        let edited = tauri::async_runtime::block_on(manager.edit(EditSubscription {
            id: imported.id.clone(),
            name: Some("Renamed".to_owned()),
            description: Some(" Description ".to_owned()),
            url_replacement: None,
            remote_request: Some(RemoteRequestPatch {
                timeout_seconds: Some(45),
                ..RemoteRequestPatch::default()
            }),
            update_policy: None,
        }))
        .expect("edit without content replacement");
        assert!(!edited.content_changed);
        let state = manager.store.load().expect("reload edited state");
        assert_eq!(state.subscriptions[0].name, "Renamed");
        assert_eq!(state.subscriptions[0].description, "Description");
        assert_eq!(
            state.subscriptions[0]
                .remote_request
                .as_ref()
                .and_then(|value| value.user_agent.as_deref()),
            Some("custom-agent")
        );
        assert_eq!(state.active_configuration_generation, 7);

        tauri::async_runtime::block_on(manager.edit(EditSubscription {
            id: imported.id,
            name: None,
            description: None,
            url_replacement: None,
            remote_request: Some(RemoteRequestPatch {
                user_agent: Some(UserAgentEdit::Clear),
                ..RemoteRequestPatch::default()
            }),
            update_policy: None,
        }))
        .expect("clear custom user agent");
        assert_eq!(
            manager
                .store
                .load()
                .expect("reload cleared state")
                .subscriptions[0]
                .remote_request
                .as_ref()
                .and_then(|value| value.user_agent.as_ref()),
            None
        );
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn edit_remote_url_fetches_only_after_an_actual_change_and_failure_keeps_the_previous_source() {
        let state_file = isolated_state_file("edit-remote-url");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Remote".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("seed subscription");

        let same_listener = TcpListener::bind("127.0.0.1:0").expect("bind same-url fixture");
        same_listener
            .set_nonblocking(true)
            .expect("set same-url fixture nonblocking");
        let same_url = format!(
            "http://{}/subscription",
            same_listener.local_addr().unwrap()
        );
        let mut seeded = manager.store.load().expect("load seeded state");
        seeded.subscriptions[0].source = SubscriptionSource::Remote {
            url: same_url.clone(),
        };
        seeded.subscriptions[0].remote_request = Some(RemoteRequestOptions::default_remote());
        seeded.subscriptions[0].update_policy = SubscriptionUpdatePolicy::default_remote();
        seeded.subscriptions[0].last_success_at_ms = Some(777);
        manager.store.save(&seeded).expect("save remote fixture");
        let original_document = seeded.subscriptions[0].document.clone();

        let same_requests = Arc::new(AtomicU64::new(0));
        let captured_same_requests = Arc::clone(&same_requests);
        let (stop_same_sender, stop_same_receiver) = mpsc::channel();
        let same_server = thread::spawn(move || {
            loop {
                match same_listener.accept() {
                    Ok((mut stream, _)) => {
                        let mut bytes = [0_u8; 8_192];
                        let _ = stream.read(&mut bytes).expect("read same-url request");
                        captured_same_requests.fetch_add(1, Ordering::AcqRel);
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{SECOND_BODY}",
                            SECOND_BODY.len()
                        );
                        stream
                            .write_all(response.as_bytes())
                            .expect("write same-url response");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(error) => panic!("accept same-url request: {error}"),
                }
                if stop_same_receiver.try_recv().is_ok() {
                    break;
                }
                thread::sleep(Duration::from_millis(5));
            }
        });

        let same = tauri::async_runtime::block_on(manager.edit(EditSubscription {
            id: imported.id.clone(),
            name: None,
            description: Some("metadata only".to_owned()),
            url_replacement: Some(same_url.clone()),
            remote_request: Some(RemoteRequestPatch {
                timeout_seconds: Some(45),
                ..RemoteRequestPatch::default()
            }),
            update_policy: None,
        }))
        .expect("save same URL metadata without fetching");
        stop_same_sender.send(()).expect("stop same-url fixture");
        same_server.join().expect("same-url fixture exits");
        assert_eq!(same_requests.load(Ordering::Acquire), 0);
        assert!(!same.content_changed);
        let after_same = manager.store.load().expect("load same-url edit");
        assert_eq!(after_same.subscriptions[0].description, "metadata only");
        assert_eq!(after_same.subscriptions[0].last_success_at_ms, Some(777));
        assert_eq!(after_same.subscriptions[0].document, original_document);
        assert_eq!(
            after_same.subscriptions[0]
                .remote_request
                .as_ref()
                .map(|value| value.timeout_seconds),
            Some(45)
        );

        let changed_listener = TcpListener::bind("127.0.0.1:0").expect("bind changed-url fixture");
        let changed_url = format!(
            "http://{}/subscription",
            changed_listener.local_addr().unwrap()
        );
        let changed_requests = Arc::new(AtomicU64::new(0));
        let captured_changed_requests = Arc::clone(&changed_requests);
        let changed_server = thread::spawn(move || {
            let (mut stream, _) = changed_listener.accept().expect("accept changed URL");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read changed-url request");
            captured_changed_requests.fetch_add(1, Ordering::AcqRel);
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{SECOND_BODY}",
                SECOND_BODY.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write changed-url response");
        });
        let changed = tauri::async_runtime::block_on(manager.edit(EditSubscription {
            id: imported.id.clone(),
            name: None,
            description: None,
            url_replacement: Some(changed_url.clone()),
            remote_request: None,
            update_policy: None,
        }))
        .expect("replace changed URL");
        changed_server.join().expect("changed-url fixture exits");
        assert_eq!(changed_requests.load(Ordering::Acquire), 1);
        assert!(changed.content_changed);
        let after_changed = manager.store.load().expect("load changed URL");
        assert_eq!(
            after_changed.subscriptions[0].source,
            SubscriptionSource::Remote {
                url: changed_url.clone()
            }
        );
        assert_eq!(
            after_changed.subscriptions[0].last_success_at_ms,
            Some(1_234)
        );
        assert_eq!(
            after_changed.subscriptions[0]
                .document
                .as_ref()
                .map(|document| document.content.as_str()),
            Some(SECOND_BODY)
        );

        let failed_listener = TcpListener::bind("127.0.0.1:0").expect("bind failed-url fixture");
        let failed_url = format!(
            "http://{}/subscription",
            failed_listener.local_addr().unwrap()
        );
        let failed_server = thread::spawn(move || {
            let (mut stream, _) = failed_listener.accept().expect("accept failed URL");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read failed-url request");
            stream
                .write_all(
                    b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .expect("write failed-url response");
        });
        let before_failure = manager.store.load().expect("load before failed URL");
        assert_eq!(
            tauri::async_runtime::block_on(manager.edit(EditSubscription {
                id: imported.id,
                name: Some("must not commit".to_owned()),
                description: None,
                url_replacement: Some(failed_url),
                remote_request: None,
                update_policy: None,
            })),
            Err(SubscriptionOperationError::FetchFailed)
        );
        failed_server.join().expect("failed-url fixture exits");
        let after_failure = manager.store.load().expect("load after failed URL");
        assert_eq!(
            after_failure.subscriptions[0].source,
            before_failure.subscriptions[0].source
        );
        assert_eq!(
            after_failure.subscriptions[0].document,
            before_failure.subscriptions[0].document
        );
        assert_eq!(
            after_failure.subscriptions[0].last_success_at_ms,
            before_failure.subscriptions[0].last_success_at_ms
        );
        assert_eq!(after_failure.subscriptions[0].name, "Remote");
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn active_delete_requires_stopped_authority_then_clears_selection_and_increments_generation() {
        let state_file = isolated_state_file("active-delete");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Active".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("manual import");
        let mut state = manager.store.load().expect("load state");
        state.active_subscription_id = Some(state.subscriptions[0].id.clone());
        state.active_configuration_generation = 4;
        manager.store.save(&state).expect("save selected state");

        assert_eq!(
            manager.delete(imported.id.clone(), false),
            Err(SubscriptionOperationError::ReferenceConflict)
        );
        assert_eq!(
            manager.store.load().expect("unchanged").subscriptions.len(),
            1
        );
        assert_eq!(manager.delete(imported.id.clone(), true), Ok(imported.id));
        let state = manager.store.load().expect("reload deletion");
        assert!(state.subscriptions.is_empty());
        assert!(state.providers.is_empty());
        assert!(state.nodes.is_empty());
        assert!(state.pools.is_empty());
        assert_eq!(state.active_subscription_id, None);
        assert_eq!(state.active_configuration_generation, 5);
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn managed_core_override_requires_owned_port_and_routes_only_that_update_through_it() {
        let state_file = isolated_state_file("managed-proxy-update");
        let manager = manager(state_file.clone());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy fixture");
        let address = listener.local_addr().expect("fixture address");
        let requests = Arc::new(Mutex::new(Vec::new()));
        let captured = Arc::clone(&requests);
        let server = thread::spawn(move || {
            for response in [
                format!(
                    "HTTP/1.1 200 OK\r\nETag: \"v1\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{FIRST_BODY}",
                    FIRST_BODY.len()
                ),
                "HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned(),
            ] {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut bytes = [0_u8; 8_192];
                let count = stream.read(&mut bytes).expect("read request");
                captured
                    .lock()
                    .expect("request lock")
                    .push(String::from_utf8_lossy(&bytes[..count]).into_owned());
                stream.write_all(response.as_bytes()).expect("write response");
            }
        });
        let imported = tauri::async_runtime::block_on(manager.import(
            "Remote".to_owned(),
            ImportSource::Remote {
                url: format!("http://{address}/subscription"),
            },
        ))
        .expect("direct import");
        assert_eq!(
            tauri::async_runtime::block_on(manager.update_with_route(
                imported.id.clone(),
                None,
                Some(UpdateRouteOverride::ManagedCore),
            )),
            Err(SubscriptionOperationError::ProxyUnavailable)
        );
        manager.set_managed_proxy_port(NonZeroU16::new(address.port()));
        let update = tauri::async_runtime::block_on(manager.update_with_route(
            imported.id,
            None,
            Some(UpdateRouteOverride::ManagedCore),
        ))
        .expect("managed proxy update");
        server.join().expect("fixture exit");
        assert!(!update.content_changed);
        let requests = requests.lock().expect("request lock");
        assert!(requests[0].starts_with("GET /subscription "));
        assert!(requests[1].starts_with(&format!("GET http://{address}/subscription ")));
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn external_clash_providers_without_inline_proxies_have_a_distinct_closed_error() {
        assert_eq!(
            parse_content(
                "proxy-providers:\n  remote:\n    type: http\n    url: https://example.invalid/provider.yaml\n"
            ),
            Err(SubscriptionOperationError::UnsupportedClashProviders)
        );
        assert_eq!(
            parse_content("proxy-providers: {}\nproxies: []\n"),
            Err(SubscriptionOperationError::ParseFailed)
        );
    }

    #[test]
    fn delete_rejects_a_custom_pool_reference_without_partial_cleanup() {
        let state_file = isolated_state_file("delete-reference");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Referenced".to_owned(),
            ImportSource::Manual {
                content: FIRST_BODY.to_owned(),
            },
        ))
        .expect("manual import");
        let mut state = manager.store.load().expect("load state");
        state.pools.push(NodePool {
            id: PoolId("custom-reference".to_owned()),
            name: "Custom".to_owned(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: state.providers[0].id.clone(),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
            },
            enabled: true,
        });
        manager.store.save(&state).expect("save custom reference");
        let before = fs::read(&state_file).expect("read referenced state");

        assert_eq!(
            manager.delete(imported.id, true),
            Err(SubscriptionOperationError::ReferenceConflict)
        );
        assert_eq!(fs::read(&state_file).expect("read unchanged state"), before);
        fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    fn legacy_document(version: u64, name: &str) -> serde_json::Value {
        let mut document = serde_json::json!({
            "schema_version": version,
            "subscriptions": [{"id":"subscription","name":name}],
            "providers": [{"id":"provider","subscription_id":"subscription","name":"Default"}],
            "nodes": [{
                "id":"node","provider_id":"provider","name":"Node","protocol":"shadowsocks",
                "server":"example.invalid","port":443,
                "credentials":{"kind":"password","username":null,"password":"fixture-secret","cipher":"aes-128-gcm"},
                "transport":"tcp","tls":null
            }]
        });
        if version >= 2 {
            document["pools"] = serde_json::json!([]);
            document["routes"] = serde_json::json!([]);
        }
        if version == 3 {
            document["default_target"] = serde_json::json!({"kind":"unconfigured"});
            let node = document["nodes"][0]
                .as_object_mut()
                .expect("legacy node object");
            node.remove("credentials");
            node.insert(
                "options".to_owned(),
                serde_json::json!({
                    "kind":"shadowsocks",
                    "method":"aes-128-gcm",
                    "password":"fixture-secret"
                }),
            );
        }
        document
    }

    #[test]
    fn singbox_response_shape_imports_all_remote_nodes() {
        let state_file = isolated_state_file("singbox-response-shape");
        let manager = manager(state_file.clone());
        let imported = tauri::async_runtime::block_on(
            manager.import(
                "Sing-box response".to_owned(),
                ImportSource::Manual {
                    content: include_str!(
                        "../../tests/fixtures/subscriptions/singbox-response-shape-021.json"
                    )
                    .to_owned(),
                },
            ),
        )
        .unwrap();
        assert_eq!(imported.node_count, 41);
        assert_eq!(imported.skipped_node_count, 0);
        let saved = manager.store.load().unwrap();
        assert_eq!(saved.nodes.iter().filter(|node| matches!(
            &node.transport,
            Some(crate::domain::Transport::Websocket {
                max_early_data: Some(2048), early_data_header_name: Some(header), host: Some(_), ..
            }) if header == "Sec-WebSocket-Protocol"
        )).count(), 10);
        fs::remove_dir_all(state_file.parent().unwrap()).unwrap();
    }

    #[test]
    fn identical_connections_merge_first_without_merging_distinct_settings() {
        let base = serde_json::json!({"type":"vless","tag":"first","server":"fixture.example.invalid","server_port":443,"uuid":"00000000-0000-4000-8000-000000000001","tls":{"enabled":true,"server_name":"fixture.example.invalid"}});
        let mut alias = base.clone();
        alias["tag"] = "duplicate alias".into();
        let mut nodes = vec![base.clone(), alias];
        for (index, (key, value)) in [
            ("uuid", serde_json::json!("00000000-0000-4000-8000-000000000002")),
            ("server", serde_json::json!("other.example.invalid")),
            ("server_port", serde_json::json!(444)),
            ("tls", serde_json::json!({"enabled":true,"server_name":"fixture.example.invalid","insecure":true})),
            ("transport", serde_json::json!({"type":"ws","path":"/fixture"})),
            ("transport", serde_json::json!({"type":"ws","path":"/fixture","max_early_data":2048,"early_data_header_name":"Sec-WebSocket-Protocol"})),
            ("transport", serde_json::json!({"type":"ws","path":"/fixture","max_early_data":4096,"early_data_header_name":"Sec-WebSocket-Protocol"})),
            ("transport", serde_json::json!({"type":"ws","path":"/fixture","max_early_data":2048,"early_data_header_name":"X-Early-Data"})),
            ("flow", serde_json::json!("xtls-rprx-vision")),
        ].into_iter().enumerate() {
            let mut variant = base.clone();
            variant["tag"] = format!("variant-{index}").into();
            variant[key] = value;
            nodes.push(variant);
        }
        let body = serde_json::json!({"outbounds": nodes}).to_string();
        let parsed = parse_content(&body).expect("parse and deduplicate");
        assert_eq!(parsed.nodes.len(), 10);
        assert_eq!(parsed.nodes[0].name, "first");
        assert!(parsed.skipped.is_empty());
        let path = isolated_state_file("exact-connection-dedup");
        let manager = manager(path.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Duplicates".into(),
            ImportSource::Manual {
                content: body.clone(),
            },
        ))
        .expect("import");
        assert_eq!((imported.node_count, imported.skipped_node_count), (10, 0));
        let saved = manager.store.load().expect("persisted");
        let expected = crate::subscription::normalize_nodes(
            saved.providers[0].id.clone(),
            vec![parsed.nodes[0].clone()],
        )
        .expect("original identity");
        assert_eq!(saved.nodes[0].id, expected[0].id);
        let updated =
            tauri::async_runtime::block_on(manager.update(imported.id.clone(), Some(body)))
                .expect("repeat update");
        assert!(!updated.content_changed);
        assert_eq!(manager.store.load().expect("reload").nodes, saved.nodes);
        let before = fs::read(&path).expect("saved bytes");
        let invalid = serde_json::json!({"outbounds":[{"type":"vless","tag":"invalid","server":"fixture.example.invalid","server_port":443}]}).to_string();
        assert_eq!(
            tauri::async_runtime::block_on(manager.update(imported.id, Some(invalid))),
            Err(SubscriptionOperationError::ParseFailed)
        );
        assert_eq!(fs::read(&path).expect("retained bytes"), before);
        fs::remove_dir_all(path.parent().expect("parent")).expect("cleanup");
    }

    #[test]
    fn mixed_clash_and_singbox_imports_filter_whole_nodes_and_keep_supported_parameters() {
        for key in ["proxies", "outbounds"] {
            let node = serde_json::json!({"type":"vless","name":"first","tag":"first","server":"fixture.example.invalid","server_port":443,"uuid":"00000000-0000-4000-8000-000000000001","tls":{"enabled":true,"server_name":"fixture.example.invalid"},"transport":{"type":"ws","path":"/ws","max_early_data":2048,"early_data_header_name":"Sec-WebSocket-Protocol","headers":{"Host":["cdn.example.invalid"]}}});
            let mut alias = node.clone();
            alias["name"] = "duplicate".into();
            alias["tag"] = "duplicate".into();
            let mut pinned = node.clone();
            pinned["fingerprint"] = "a".repeat(64).into();
            let mut unsupported = node.clone();
            unsupported["unsupported_option"] = true.into();
            let body = serde_json::json!({key:[node.clone(), alias, pinned, unsupported, {"type":"unknown"}, {"type":"vless","tag":"missing-credentials","server":"fixture.example.invalid","server_port":443}]}).to_string();
            let path = isolated_state_file(key);
            let manager = manager(path.clone());
            let imported = tauri::async_runtime::block_on(
                manager.import("Filtered".into(), ImportSource::Manual { content: body }),
            )
            .unwrap();
            assert_eq!((imported.node_count, imported.skipped_node_count), (1, 4));
            let saved = manager.store.load().unwrap();
            assert_eq!(saved.nodes[0].name, "first");
            let expected = parse_subscription(&serde_json::json!({key:[node]}).to_string())
                .unwrap()
                .nodes
                .remove(0);
            assert_eq!(saved.nodes[0].options, expected.options);
            assert_eq!(saved.nodes[0].tls, expected.tls);
            assert_eq!(saved.nodes[0].transport, expected.transport);
            assert_eq!(saved.subscriptions[0].skipped_unsupported_nodes, 4);
            fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }

    #[test]
    fn all_filtered_certificate_nodes_preserve_saved_state() {
        let path = isolated_state_file("certificate-pin-refusal");
        let manager = manager(path.clone());
        let imported = tauri::async_runtime::block_on(manager.import(
            "Existing".into(),
            ImportSource::Manual {
                content: FIRST_BODY.into(),
            },
        ))
        .expect("initial import");
        let before = fs::read(&path).expect("saved bytes");
        let pin = serde_json::json!({"proxies":[{"type":"hysteria2","name":"synthetic","server":"fixture.example.invalid","port":443,"password":"synthetic-password","fingerprint":"a".repeat(64),"skip-cert-verify":true}]}).to_string();
        assert_eq!(
            tauri::async_runtime::block_on(manager.update(imported.id, Some(pin.clone()))),
            Err(SubscriptionOperationError::ParseFailed)
        );
        assert_eq!(fs::read(&path).expect("unchanged bytes"), before);
        assert_eq!(
            parse_content(&pin.replace(&"a".repeat(64), "malformed")),
            Err(SubscriptionOperationError::ParseFailed)
        );
        let unicode = pin.replace("synthetic\"", "\\ud83d\\ude00\"");
        assert_eq!(
            parse_content(&unicode),
            Err(SubscriptionOperationError::ParseFailed)
        );
        assert_eq!(
            parse_content(
                r#"{"outbounds":[{"type":"direct","tag":"direct"},{"type":"selector","tag":"group","outbounds":["direct"]}]}"#
            ),
            Err(SubscriptionOperationError::ParseFailed)
        );
        fs::remove_dir_all(path.parent().expect("parent")).expect("cleanup");
    }

    #[test]
    fn compatible_import_persists_warning_and_update_resets_it_atomically() {
        let path = isolated_state_file("compatible-import");
        let manager = manager(path.clone());
        let body = include_str!("../../tests/fixtures/subscriptions/clash-compatible.yaml");
        let imported = tauri::async_runtime::block_on(manager.import(
            "Compatible".into(),
            ImportSource::Manual {
                content: body.into(),
            },
        ))
        .expect("supported nodes imported");
        assert_eq!((imported.node_count, imported.skipped_node_count), (10, 1));
        assert_eq!(
            manager.store.load().expect("reload").subscriptions[0].skipped_unsupported_nodes,
            1
        );
        assert_eq!(manager.list().expect("list")[0].skipped_node_count, 1);
        let before_nodes = manager.store.load().unwrap().nodes;
        let invalid = format!(
            "{body}\n- {{name: broken, type: vless, server: example.invalid, port: 443}}\n"
        );
        let filtered =
            tauri::async_runtime::block_on(manager.update(imported.id.clone(), Some(invalid)))
                .unwrap();
        assert_eq!(
            (
                filtered.subscription.node_count,
                filtered.subscription.skipped_node_count
            ),
            (10, 2)
        );
        assert_eq!(manager.store.load().unwrap().nodes, before_nodes);
        let edited = tauri::async_runtime::block_on(manager.edit(EditSubscription {
            id: imported.id.clone(),
            name: Some("Renamed".into()),
            description: None,
            remote_request: None,
            update_policy: None,
            url_replacement: None,
        }))
        .expect("metadata edit");
        assert_eq!(edited.subscription.skipped_node_count, 2);
        let updated =
            tauri::async_runtime::block_on(manager.update(imported.id, Some(FIRST_BODY.into())))
                .expect("replacement");
        assert_eq!(
            (
                updated.subscription.node_count,
                updated.subscription.skipped_node_count
            ),
            (1, 0)
        );
        assert_eq!(
            manager.store.load().expect("reload").subscriptions[0].skipped_unsupported_nodes,
            0
        );
        fs::remove_dir_all(path.parent().expect("parent")).expect("cleanup");
    }

    #[test]
    fn compatible_parser_rejects_all_filtered_nodes() {
        let body = include_str!("../../tests/fixtures/subscriptions/clash-compatible.yaml");
        let mut document: serde_json::Value = serde_yaml_ng::from_str(body).expect("fixture YAML");
        document["proxies"]
            .as_array_mut()
            .expect("nodes")
            .retain(|node| node.get("smux").is_some());
        assert_eq!(
            document["proxies"]
                .as_array()
                .expect("one unsupported")
                .len(),
            1
        );
        assert_eq!(
            parse_content(&document.to_string()),
            Err(SubscriptionOperationError::ParseFailed)
        );
        document["proxies"]
            .as_array_mut()
            .expect("nodes")
            .push(serde_json::json!({"type":"unknown"}));
        assert_eq!(
            parse_content(&document.to_string()),
            Err(SubscriptionOperationError::ParseFailed)
        );
    }
}
