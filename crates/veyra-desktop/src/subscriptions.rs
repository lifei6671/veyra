//! 订阅页面的请求代次仅属 Desktop；不进入 domain，也不成为第二份持久状态。
use veyra_core::{
    application::subscription_management::{
        EditSubscription, SubscriptionOperationError as Error, SubscriptionSummary,
        preview::{PreviewInput, PreviewSaveResult, SubscriptionPreview},
    },
    domain::{AppState, ConfigVersion},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: u64,
    pub generation: u64,
    pub expected: Option<ConfigVersion>,
}
#[derive(Clone)]
pub enum Command {
    Load,
    // 独立Preview桥只供既有合同回归；当前GUI已按Host要求删除预览入口。
    #[cfg(test)]
    Preview(PreviewInput),
    #[cfg(test)]
    Save(PreviewInput, SubscriptionPreview),
    // Host移除GUI预览区：保存点击在后台复用typed preview/save，Core契约不变。
    SaveDraft(PreviewInput),
    Edit(EditSubscription, Option<String>),
    Refresh {
        id: String,
        content: Option<String>,
    },
    Delete(String),
}
pub enum Completion {
    Loaded(Box<AppState>),
    SaveRejected(PreviewSaveResult),
    #[cfg(test)]
    Preview(SubscriptionPreview),
    Mutation {
        state: Box<AppState>,
        saved: Option<PreviewSaveResult>,
    },
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Status {
    #[default]
    Empty,
    Ready,
    Busy,
    #[cfg(test)]
    Preview,
    Saved,
    Error(Error),
}
/// 仅从已验证AppState派生的只读展示；没有凭据、endpoint或独立持久化事实。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionNodePresentation {
    pub id: String,
    pub provider_id: String,
    pub subscription_id: String,
    pub name: String,
    pub protocol: String,
}
#[derive(Default)]
pub struct Model {
    pub list: Vec<SubscriptionSummary>,
    pub nodes: Vec<SubscriptionNodePresentation>,
    pub preview: Option<SubscriptionPreview>,
    pub saved: Option<PreviewSaveResult>,
    pub version: Option<ConfigVersion>,
    pub status: Status,
    pub editor_open: bool,
    pub editing: Option<String>,
    pub manual: bool,
    generation: u64,
    next_id: u64,
    pending: Option<Request>,
}
impl Model {
    pub fn project(&mut self, state: &AppState) {
        let version = Some(state.config_version());
        if self.version != version {
            self.invalidate();
        }
        self.version = version;
        self.list = veyra_core::application::subscription_management::summaries(state)
            .expect("validated core snapshot");
        self.nodes = state
            .nodes
            .iter()
            .map(|node| {
                let provider = state
                    .providers
                    .iter()
                    .find(|p| p.id == node.provider_id)
                    .expect("validated node provider ownership");
                let protocol = serde_json::to_value(node.protocol)
                    .expect("protocol enum")
                    .as_str()
                    .expect("protocol string")
                    .to_owned();
                SubscriptionNodePresentation {
                    id: node.id.0.clone(),
                    provider_id: node.provider_id.0.clone(),
                    subscription_id: provider.subscription_id.0.clone(),
                    name: node.name.clone(),
                    protocol,
                }
            })
            .collect();
        if matches!(self.status, Status::Empty | Status::Ready) {
            self.status = if self.list.is_empty() {
                Status::Empty
            } else {
                Status::Ready
            };
        }
    }
    pub fn invalidate(&mut self) {
        self.generation += 1;
        self.pending = None;
        self.preview = None;
        self.saved = None;
        self.status = if self.list.is_empty() {
            Status::Empty
        } else {
            Status::Ready
        };
    }
    pub fn begin(&mut self) -> Request {
        self.next_id += 1;
        let request = Request {
            id: self.next_id,
            generation: self.generation,
            expected: self.version.clone(),
        };
        self.pending = Some(request.clone());
        self.status = Status::Busy;
        request
    }
    pub fn busy(&self) -> bool {
        self.status == Status::Busy
    }
    pub fn accepts(&self, request: &Request) -> bool {
        self.pending.as_ref() == Some(request) && request.expected == self.version
    }
    pub fn complete(
        &mut self,
        request: &Request,
        result: Result<Completion, Error>,
    ) -> Option<Box<AppState>> {
        // 旧完成结果不得清掉新请求 busy；input/page/epoch 任一改变都丢弃。
        if !self.accepts(request) {
            return None;
        }
        self.pending = None;
        match result {
            Ok(Completion::Loaded(state)) => {
                self.invalidate();
                self.project(&state);
                Some(state)
            }
            Ok(Completion::SaveRejected(report)) => {
                self.status = Status::Error(
                    report
                        .sources
                        .iter()
                        .find_map(|source| source.error)
                        .unwrap_or(Error::ParseFailed),
                );
                self.saved = Some(report);
                None
            }
            #[cfg(test)]
            Ok(Completion::Preview(preview)) => {
                if preview.expected != self.version {
                    self.status = Status::Error(Error::Busy);
                    return None;
                }
                // Retry 成功后以新预览为唯一报告，避免旧 SaveRejected 遮住结果。
                self.saved = None;
                self.preview = Some(preview);
                self.status = Status::Preview;
                None
            }
            Ok(Completion::Mutation { state, saved }) => {
                if request.expected.as_ref().is_some_and(|v| {
                    v.0.epoch != state.state_epoch || v.0.revision > state.config_revision
                }) {
                    self.status = Status::Error(Error::Busy);
                    return None;
                }
                self.project(&state);
                self.status = if saved.as_ref().is_some_and(|s| s.subscriptions.is_empty()) {
                    Status::Error(Error::ParseFailed)
                } else {
                    Status::Saved
                };
                self.saved = saved;
                Some(state)
            }
            Err(error) => {
                self.status = Status::Error(error);
                None
            }
        }
    }
}
// UI 错误只展示稳定类别；ReferenceConflict 包含用户可执行的下一步。
pub fn error_key(error: Error) -> &'static str {
    match error {
        Error::ReferenceConflict => {
            "订阅仍被引用：请先移除分流/出站引用；活动订阅需由运行服务确认停止后删除"
        }
        Error::Busy => "输入或保存版本已改变，请重新打开订阅并重试",
        Error::InvalidInput | Error::InvalidOptions => "订阅输入无效，请检查名称、来源和 URL",
        Error::FetchFailed => "Direct 下载失败，旧内容已保留，请检查网络后重试",
        Error::ParseFailed | Error::UnsupportedClashProviders | Error::NormalizationFailed => {
            "内容无法解析为支持的节点，旧内容已保留"
        }
        Error::ProxyUnavailable | Error::SystemProxyUnavailable => {
            "当前仅支持 Direct，其他下载路径尚未接入"
        }
        Error::SaveFailed => "保存失败，草稿与旧内容已保留，请重试",
        Error::NotFound => "订阅不存在，请重新加载",
        _ => "订阅操作失败，请重新加载后重试",
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    // 保护只读节点按真实provider归属展示；同名不同endpoint不能在presentation合并，展示不改磁盘。
    #[test]
    fn readonly_nodes_preserve_identity_ownership_and_saved_bytes() {
        use veyra_core::{
            application::state_access::StateAccessGate,
            application::subscription_management::{SubscriptionManager, preview::PreviewSource},
        };
        let directory = std::env::temp_dir().join(format!(
            "veyra-readonly-presentation-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let path = directory.join("state.json");
        let manager = SubscriptionManager::new(path.clone(), StateAccessGate::default()).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        for name in ["first", "second"] {
            let input=PreviewInput {name:name.into(),description:String::new(),sources:vec![PreviewSource::Pasted(
                "proxies:\n  - {name: same, type: socks5, server: fixture-a.invalid, port: 1080}\n  - {name: same, type: socks5, server: fixture-b.invalid, port: 1081}\n".into())]};
            let preview = runtime.block_on(manager.preview(&input)).unwrap();
            runtime
                .block_on(manager.save_preview(input, &preview))
                .unwrap();
        }
        let bytes = std::fs::read(&path).unwrap();
        let mut state: AppState = serde_json::from_slice(&bytes).unwrap();
        let mut model = Model::default();
        model.project(&state);
        assert_eq!(model.nodes.len(), 4);
        for subscription in &state.subscriptions {
            let nodes: Vec<_> = model
                .nodes
                .iter()
                .filter(|n| n.subscription_id == subscription.id.0)
                .collect();
            assert_eq!(nodes.len(), 2);
            assert_ne!(nodes[0].id, nodes[1].id);
            assert_eq!(nodes[0].name, "same");
            assert_eq!(nodes[0].protocol, "socks");
        }
        let id = model.nodes[0].id.clone();
        state.nodes[0].name = "renamed".into();
        model.project(&state);
        assert_eq!(model.nodes[0].id, id);
        assert_eq!(model.nodes[0].name, "renamed");
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        std::fs::remove_dir_all(directory).unwrap();
    }
    // 保护实际 Save 失败后不改输入再次 Preview 的用户重试路径。
    #[test]
    fn successful_preview_replaces_rejected_save_report() {
        use veyra_core::{
            application::state_access::StateAccessGate,
            application::subscription_management::SubscriptionManager,
        };
        let path = std::env::temp_dir().join(format!("veyra-preview-retry-{}", std::process::id()));
        let manager = SubscriptionManager::new(path.clone(), StateAccessGate::default()).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let preview = runtime.block_on(manager.preview(&PreviewInput {
            name: "retry".into(), description: String::new(),
            sources: vec![veyra_core::application::subscription_management::preview::PreviewSource::Pasted(
                "proxies:\n  - {name: alpha, type: socks5, server: fixture.invalid, port: 1080}\n".into())],
        })).unwrap();
        let mut model = Model::default();
        let failed = model.begin();
        model.complete(
            &failed,
            Ok(Completion::SaveRejected(PreviewSaveResult {
                sources: vec![],
                subscriptions: vec![],
            })),
        );
        assert!(model.saved.is_some());
        let retry = model.begin();
        model.complete(&retry, Ok(Completion::Preview(preview)));
        assert_eq!(model.status, Status::Preview);
        assert_eq!(model.preview.as_ref().unwrap().node_count, 1);
        assert!(model.saved.is_none());
        assert!(!path.exists(), "preview must not create storage");
    }
    // 保护空/忙碌/错误/重试；旧结果不能覆盖新 generation 或清除新 busy。
    #[test]
    fn empty_busy_error_retry_stale_preview_and_page_leave() {
        let mut model = Model::default();
        assert_eq!(model.status, Status::Empty);
        let old = model.begin();
        model.invalidate();
        let new = model.begin();
        model.complete(&old, Err(Error::FetchFailed));
        assert!(model.busy());
        model.complete(&new, Err(Error::FetchFailed));
        assert_eq!(model.status, Status::Error(Error::FetchFailed));
        let retry = model.begin();
        model.complete(&retry, Err(Error::ParseFailed));
        assert_eq!(model.status, Status::Error(Error::ParseFailed));
        let pending = model.begin();
        model.invalidate();
        model.complete(&pending, Err(Error::Busy));
        assert_eq!(model.status, Status::Empty);
    }
    // 新 epoch 投影后不允许旧 refresh 结果重建页面或 snapshot。
    #[test]
    fn refresh_result_old_epoch_and_saved_states() {
        let mut model = Model::default();
        let state = AppState::empty();
        model.project(&state);
        let old = model.begin();
        let replacement = AppState::empty();
        model.project(&replacement);
        let new = model.begin();
        assert!(
            model
                .complete(
                    &old,
                    Ok(Completion::Mutation {
                        state: Box::new(state),
                        saved: None
                    })
                )
                .is_none()
        );
        assert!(model.busy());
        let state = model.complete(
            &new,
            Ok(Completion::Mutation {
                state: Box::new(replacement),
                saved: None,
            }),
        );
        assert!(state.is_some());
        assert_eq!(model.status, Status::Saved);
    }
}
