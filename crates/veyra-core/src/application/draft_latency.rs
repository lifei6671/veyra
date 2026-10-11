//! 单草稿测试只拥有本次 SidecarRuntime、临时配置和缓存；不使用 ManualRuntime/RecoveryStore。
use super::{
    network::{Latency, NetworkError, NetworkService},
    proxies::TestCancellation,
};
use crate::{
    domain::{
        AppState, NodeId, OutboundId, Provider, ProviderId, RuntimeIntent, StateEpoch,
        Subscription, SubscriptionId, SubscriptionSource, SubscriptionUpdatePolicy,
        UiLatencyPreferences,
    },
    singbox::{
        CompileError, LoopbackListener, ManagedCacheFile, ProductCompileRequest,
        ProductRuntimeResources, SingBoxCompiler,
        runtime::{SidecarError, SidecarPort, SidecarRuntime},
        secret::generate_api_secret,
    },
    subscription::{
        ProxyNodeDraft, normalize_nodes,
        outbound::{ProxySource, RunningProxy},
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug)]
pub enum DraftLatencyError {
    InvalidDraft,
    State(crate::domain::AppError),
    Compile(CompileError),
    Resources(std::io::Error),
    Sidecar(SidecarError),
    Endpoint,
    Secret,
    Network(NetworkError),
    Cancelled,
}
#[derive(Debug)]
pub struct DraftLatencyOutcome {
    pub result: Result<Latency, DraftLatencyError>,
    /// 清理失败与测试成功分开，session保留root/port，可显式cleanup，不伪报无残留。
    pub cleanup: Result<(), DraftLatencyError>,
}

/// P 复用现有 SidecarPort；构造闭包只把本次自有root交给既有平台实现。
/// 在后台blocking任务内调用，取消通过独立句柄通知，不阻塞主Runtime队列。
pub struct DraftLatencySession<P: SidecarPort> {
    root: PathBuf,
    runtime: SidecarRuntime<P>,
    source: ProxySource,
}
impl<P: SidecarPort> DraftLatencySession<P> {
    pub fn new(parent: &Path, bind: impl FnOnce(&Path) -> P) -> Result<Self, DraftLatencyError> {
        let id = StateEpoch::fresh().map_err(DraftLatencyError::State)?;
        let root = parent.join(format!("draft-{id:?}"));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&root)
            .map_err(DraftLatencyError::Resources)?;
        if let Err(error) = fs::create_dir_all(root.join("runtime/configs")) {
            let _ = fs::remove_dir_all(&root);
            return Err(DraftLatencyError::Resources(error));
        }
        let port = bind(&root);
        Ok(Self {
            root,
            runtime: SidecarRuntime::new_dynamic(port),
            source: Default::default(),
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn test(
        &mut self,
        draft: ProxyNodeDraft,
        preferences: &UiLatencyPreferences,
        cancellation: &TestCancellation,
    ) -> DraftLatencyOutcome {
        let result = self.measure(draft, preferences, cancellation);
        let cleanup = self.cleanup();
        DraftLatencyOutcome { result, cleanup }
    }
    fn measure(
        &mut self,
        draft: ProxyNodeDraft,
        preferences: &UiLatencyPreferences,
        cancellation: &TestCancellation,
    ) -> Result<Latency, DraftLatencyError> {
        if *cancellation.receiver().borrow() {
            return Err(DraftLatencyError::Cancelled);
        }
        let mut state = AppState::try_empty().map_err(DraftLatencyError::State)?;
        let provider = ProviderId("draft-provider".into());
        state.nodes = normalize_nodes(provider.clone(), vec![draft])
            .map_err(|_| DraftLatencyError::InvalidDraft)?;
        let node: NodeId = state.nodes[0].id.clone();
        let subscription = SubscriptionId("draft-subscription".into());
        state.providers.push(Provider {
            id: provider,
            subscription_id: subscription.clone(),
            name: "draft".into(),
        });
        state.subscriptions.push(Subscription {
            id: subscription,
            name: "draft".into(),
            description: String::new(),
            source: SubscriptionSource::Manual,
            last_success_at_ms: None,
            last_attempt_at_ms: None,
            http_metadata: None,
            remote_request: None,
            update_policy: SubscriptionUpdatePolicy::manual(),
            skipped_unsupported_nodes: 0,
            document: None,
        });
        state.groups.clear();
        state.app_config.behavior.latency = preferences.clone();
        state
            .app_config
            .behavior
            .validate()
            .map_err(DraftLatencyError::State)?;
        let intent =
            RuntimeIntent::from_state(&state).map_err(|_| DraftLatencyError::InvalidDraft)?;
        let loopback = LoopbackListener::new("127.0.0.1:0".parse().expect("loopback"))
            .map_err(DraftLatencyError::Compile)?;
        let resources = ProductRuntimeResources {
            mixed: loopback,
            controller: loopback,
            cache: ManagedCacheFile::new(
                &self.root,
                self.root.join("cache.db"),
                "draft-node".into(),
                false,
                false,
            )
            .map_err(DraftLatencyError::Compile)?,
        };
        let plan = SingBoxCompiler
            .compile_product(ProductCompileRequest {
                state: &state,
                runtime_intent: &intent,
                default_outbound: &OutboundId::Node(node.clone()),
                resources: &resources,
            })
            .map_err(DraftLatencyError::Compile)?;
        let index = plan
            .artifact_index()
            .ok_or(DraftLatencyError::Endpoint)?
            .clone();
        let config = plan
            .finalize(&generate_api_secret().map_err(|_| DraftLatencyError::Secret)?)
            .map_err(DraftLatencyError::Compile)?;
        self.runtime
            .start_or_replace(config)
            .map_err(DraftLatencyError::Sidecar)?;
        // check/run/ready沿用现有Port有界预算；期间收到取消也必须先清理本实例。
        if *cancellation.receiver().borrow() {
            return Err(DraftLatencyError::Cancelled);
        }
        let endpoints = self
            .runtime
            .endpoints()
            .ok_or(DraftLatencyError::Endpoint)?;
        let endpoint = self
            .runtime
            .with_active_port(|port, child| Ok(port.observation_endpoint(child)))
            .map_err(|_| DraftLatencyError::Endpoint)?
            .flatten()
            .ok_or(DraftLatencyError::Endpoint)?;
        let mut proxy =
            RunningProxy::ready(format!("draft-{:?}", state.state_epoch), endpoints.mixed)
                .map_err(|_| DraftLatencyError::Endpoint)?;
        proxy.controller = Some(endpoint);
        proxy.nodes = index.nodes;
        self.source.publish(proxy);
        let service = NetworkService::new(self.source.clone());
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(DraftLatencyError::Resources)?;
        executor.block_on(async {
            let mut cancelled = cancellation.receiver();
            if *cancelled.borrow() { return Err(DraftLatencyError::Cancelled); }
            tokio::select! { biased;
                _ = cancelled.wait_for(|v| *v) => Err(DraftLatencyError::Cancelled),
                result = service.node_latency(&node, &preferences.test_url.0, Duration::from_millis(u64::from(preferences.timeout_ms))) => result.map_err(DraftLatencyError::Network),
            }
        })
    }
    pub fn cleanup(&mut self) -> Result<(), DraftLatencyError> {
        self.source.revoke();
        self.runtime.stop().map_err(DraftLatencyError::Sidecar)?;
        match fs::remove_dir_all(&self.root) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(DraftLatencyError::Resources(error)),
        }
    }
}
impl<P: SidecarPort> Drop for DraftLatencySession<P> {
    fn drop(&mut self) {
        // 未确认退出保留root。P仍持有owned child，不删除其配置/缓存材料。
        let _ = self.cleanup();
    }
}

#[cfg(test)]
mod tests;
