use std::sync::Mutex;

use crate::domain::{AppState, ProviderId, StateValidationError};
use crate::storage::{StateStore, StateStoreError};
use crate::subscription::{NormalizationError, ParseResult, normalize_nodes};

/// 在持久化成功前不交换内存状态的 Provider 节点替换事务。
#[allow(
    dead_code,
    reason = "TASK-007 establishes the internal transaction before a later IPC delivery owns its entrypoint"
)]
pub struct ProviderReplacementService<S> {
    store: S,
    state: Mutex<AppState>,
}

#[allow(
    dead_code,
    reason = "TASK-007 establishes the internal transaction before a later IPC delivery owns its entrypoint"
)]
impl<S: StateStore> ProviderReplacementService<S> {
    pub fn new(store: S, state: AppState) -> Result<Self, ProviderReplacementError> {
        state.validate()?;
        Ok(Self {
            store,
            state: Mutex::new(state),
        })
    }

    pub fn snapshot(&self) -> AppState {
        self.state
            .lock()
            .expect("provider state mutex is not poisoned")
            .clone()
    }

    pub fn replace(
        &self,
        provider_id: ProviderId,
        parsed: ParseResult,
    ) -> Result<(), ProviderReplacementError> {
        if parsed.nodes.is_empty() || !parsed.skipped.is_empty() {
            return Err(ProviderReplacementError::RejectedBatch);
        }
        let replacement = normalize_nodes(provider_id.clone(), parsed.nodes)?;
        let mut state = self
            .state
            .lock()
            .expect("provider state mutex is not poisoned");
        if !state
            .providers
            .iter()
            .any(|provider| provider.id == provider_id)
        {
            return Err(ProviderReplacementError::MissingProvider);
        }
        let mut candidate = state.clone();
        candidate
            .nodes
            .retain(|node| node.provider_id != provider_id);
        candidate.nodes.extend(replacement);
        candidate.validate()?;
        self.store.save(&candidate)?;
        *state = candidate;
        Ok(())
    }
}

#[derive(Debug)]
#[allow(
    dead_code,
    reason = "the internal transaction error becomes reachable with its later IPC entrypoint"
)]
pub enum ProviderReplacementError {
    RejectedBatch,
    MissingProvider,
    Normalize(NormalizationError),
    State(StateValidationError),
    Store(StateStoreError),
}

impl std::fmt::Display for ProviderReplacementError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::RejectedBatch => "provider replacement batch was rejected",
            Self::MissingProvider => "provider replacement references a missing provider",
            Self::Normalize(error) => {
                let _ = std::mem::discriminant(error);
                "provider replacement could not normalize nodes"
            }
            Self::State(error) => {
                let _ = std::mem::discriminant(error);
                "provider replacement candidate is invalid"
            }
            Self::Store(_) => "provider replacement could not be saved",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ProviderReplacementError {}

impl From<NormalizationError> for ProviderReplacementError {
    fn from(value: NormalizationError) -> Self {
        Self::Normalize(value)
    }
}

impl From<StateValidationError> for ProviderReplacementError {
    fn from(value: StateValidationError) -> Self {
        Self::State(value)
    }
}

impl From<StateStoreError> for ProviderReplacementError {
    fn from(value: StateStoreError) -> Self {
        Self::Store(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;

    use crate::domain::{
        NodeFilter, NodeId, NodePool, PoolId, PoolKind, PoolSource, ProtocolOptions, Provider,
        ProxyNode, ProxyProtocol, SelectionPolicy, Subscription, SubscriptionId, Transport,
    };
    use crate::subscription::{ProxyNodeDraft, SkippedNode, SubscriptionFormat};

    struct TestStore {
        fail_save: bool,
    }

    struct RecordingStore {
        saved: Mutex<Vec<AppState>>,
    }

    impl StateStore for TestStore {
        fn load(&self) -> Result<AppState, StateStoreError> {
            Err(StateStoreError::ReadFailed)
        }

        fn save(&self, _: &AppState) -> Result<(), StateStoreError> {
            if self.fail_save {
                Err(StateStoreError::WriteFailed)
            } else {
                Ok(())
            }
        }
    }

    impl StateStore for RecordingStore {
        fn load(&self) -> Result<AppState, StateStoreError> {
            Err(StateStoreError::ReadFailed)
        }

        fn save(&self, state: &AppState) -> Result<(), StateStoreError> {
            self.saved
                .lock()
                .expect("recording store mutex is not poisoned")
                .push(state.clone());
            Ok(())
        }
    }

    fn state() -> AppState {
        AppState {
            schema_version: crate::domain::CURRENT_SCHEMA_VERSION,
            default_target: crate::domain::RouteTarget::Unconfigured,
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
                id: NodeId("old-node".to_owned()),
                provider_id: ProviderId("provider".to_owned()),
                name: "Old".to_owned(),
                protocol: ProxyProtocol::Vless,
                server: "old.example.invalid".to_owned(),
                port: 443,
                options: ProtocolOptions::Vless {
                    uuid: "old-secret".to_owned(),
                    flow: None,
                },
                transport: Some(Transport::Tcp),
                tls: None,
            }],
            pools: Vec::new(),
            routes: Vec::new(),
        }
    }

    fn parsed(skipped: Vec<SkippedNode>) -> ParseResult {
        ParseResult {
            format: SubscriptionFormat::UriList,
            nodes: vec![ProxyNodeDraft {
                name: "New".to_owned(),
                protocol: ProxyProtocol::Vless,
                server: "new.example.invalid".to_owned(),
                port: 443,
                options: ProtocolOptions::Vless {
                    uuid: "new-secret".to_owned(),
                    flow: None,
                },
                transport: Some(Transport::Tcp),
                tls: None,
            }],
            skipped,
        }
    }

    fn parsed_named(name: &str) -> ParseResult {
        let mut parsed = parsed(Vec::new());
        parsed.nodes[0].name = name.to_owned();
        parsed
    }

    #[test]
    fn only_exchanges_memory_after_a_successful_save() {
        let service = ProviderReplacementService::new(TestStore { fail_save: false }, state())
            .expect("valid state");
        service
            .replace(ProviderId("provider".to_owned()), parsed(Vec::new()))
            .expect("replace succeeds");
        assert_eq!(service.snapshot().nodes[0].name, "New");
    }

    #[test]
    fn preserves_old_nodes_for_skipped_or_unsaved_batches() {
        let rejected = ProviderReplacementService::new(TestStore { fail_save: false }, state())
            .expect("valid state");
        assert!(matches!(
            rejected.replace(
                ProviderId("provider".to_owned()),
                parsed(vec![SkippedNode::InvalidNode])
            ),
            Err(ProviderReplacementError::RejectedBatch)
        ));
        assert_eq!(rejected.snapshot().nodes[0].name, "Old");

        let unsaved = ProviderReplacementService::new(TestStore { fail_save: true }, state())
            .expect("valid state");
        assert!(matches!(
            unsaved.replace(ProviderId("provider".to_owned()), parsed(Vec::new())),
            Err(ProviderReplacementError::Store(
                StateStoreError::WriteFailed
            ))
        ));
        assert_eq!(unsaved.snapshot().nodes[0].name, "Old");
    }

    #[test]
    fn preserves_old_nodes_for_normalization_or_candidate_validation_failures() {
        let normalization =
            ProviderReplacementService::new(TestStore { fail_save: false }, state())
                .expect("valid state");
        let mut duplicate = parsed(Vec::new());
        duplicate.nodes.push(duplicate.nodes[0].clone());
        assert!(matches!(
            normalization.replace(ProviderId("provider".to_owned()), duplicate),
            Err(ProviderReplacementError::Normalize(
                NormalizationError::DuplicateNodeIdentity
            ))
        ));
        assert_eq!(normalization.snapshot().nodes[0].name, "Old");

        let mut referenced_state = state();
        referenced_state.pools.push(NodePool {
            id: PoolId("pool".to_owned()),
            name: "Pool".to_owned(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider".to_owned()),
                filter: NodeFilter {
                    include_node_ids: vec![NodeId("old-node".to_owned())],
                    ..NodeFilter::default()
                },
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(NodeId("old-node".to_owned())),
            },
            enabled: true,
        });
        let invalid_candidate =
            ProviderReplacementService::new(TestStore { fail_save: false }, referenced_state)
                .expect("valid state before replacement");
        assert!(matches!(
            invalid_candidate.replace(ProviderId("provider".to_owned()), parsed(Vec::new())),
            Err(ProviderReplacementError::State(
                StateValidationError::InvalidFilter
            ))
        ));
        assert_eq!(invalid_candidate.snapshot().nodes[0].name, "Old");
    }

    #[test]
    fn serializes_concurrent_provider_replacements() {
        let store = RecordingStore {
            saved: Mutex::new(Vec::new()),
        };
        let service =
            Arc::new(ProviderReplacementService::new(store, state()).expect("valid state"));
        let barrier = Arc::new(Barrier::new(2));
        let workers = ["First", "Second"].map(|name| {
            let service = Arc::clone(&service);
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                service
                    .replace(ProviderId("provider".to_owned()), parsed_named(name))
                    .expect("replacement succeeds");
            })
        });
        for worker in workers {
            worker.join().expect("worker does not panic");
        }

        let saved = service
            .store
            .saved
            .lock()
            .expect("recording store mutex is not poisoned");
        assert_eq!(saved.len(), 2);
        assert!(matches!(
            service.snapshot().nodes[0].name.as_str(),
            "First" | "Second"
        ));
    }
}
