//! P2-01 正式预览与 Direct 保存。预览不持有节点/凭据，保存重新下载、解析并共用原事务。
use super::*;
use crate::domain::ConfigVersion;
use crate::subscription::SkippedNode;

// 输入包含私密 URL/节点内容，刻意不实现 Debug/Serialize。
#[derive(Clone, Eq, PartialEq)]
pub struct PreviewInput {
    pub name: String,
    pub description: String,
    pub sources: Vec<PreviewSource>,
}
#[derive(Clone, Eq, PartialEq)]
pub enum PreviewSource {
    Url(String),
    Pasted(String),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePreview {
    pub index: usize,
    pub kind: SubscriptionSourceKind,
    pub safe_display: String,
    pub node_count: usize,
    pub skipped: Vec<SkippedNode>,
    pub error: Option<SubscriptionOperationError>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubscriptionPreview {
    pub sources: Vec<SourcePreview>,
    pub savable_sources: usize,
    pub node_count: usize,
    pub expected: Option<ConfigVersion>,
    // 只绑定原始输入；不能把外部传回的预览对象当成候选事实。
    fingerprint: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreviewSaveResult {
    pub sources: Vec<SourcePreview>,
    pub subscriptions: Vec<SubscriptionSummary>,
}

impl PreviewInput {
    fn validate(&self) -> Result<(), SubscriptionOperationError> {
        validate_name(self.name.clone())?;
        validate_description(self.description.clone())?;
        if self.sources.is_empty() || self.sources.len() > 32 {
            return Err(SubscriptionOperationError::InvalidInput);
        }
        Ok(())
    }
    fn fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        for value in [&self.name, &self.description] {
            hash.update(value.len().to_le_bytes());
            hash.update(value.as_bytes());
        }
        for source in &self.sources {
            let (tag, value) = match source {
                PreviewSource::Url(v) => (0_u8, v),
                PreviewSource::Pasted(v) => (1_u8, v),
            };
            hash.update([tag]);
            hash.update(value.len().to_le_bytes());
            hash.update(value.as_bytes());
        }
        hash.finalize().into()
    }
}

impl SubscriptionManager {
    fn preview_version(&self) -> Result<Option<ConfigVersion>, SubscriptionOperationError> {
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        self.store
            .read_current_version()
            .map_err(|_| SubscriptionOperationError::StateUnavailable)
    }

    pub async fn preview(
        &self,
        input: &PreviewInput,
    ) -> Result<SubscriptionPreview, SubscriptionOperationError> {
        input.validate()?;
        let expected = self.preview_version()?;
        let mut sources = Vec::new();
        for (index, source) in input.sources.iter().enumerate() {
            let (view, _) = self.prepare_preview_source(index, source, input).await;
            sources.push(view);
        }
        Ok(SubscriptionPreview {
            savable_sources: sources.iter().filter(|s| s.error.is_none()).count(),
            node_count: sources.iter().map(|s| s.node_count).sum(),
            sources,
            expected,
            fingerprint: input.fingerprint(),
        })
    }

    /// 一个多来源操作构造一个候选并 commit 一次；失败来源保留顺序且不创建实体。
    pub async fn save_preview(
        &self,
        input: PreviewInput,
        preview: &SubscriptionPreview,
    ) -> Result<PreviewSaveResult, SubscriptionOperationError> {
        let _write = self.begin_write_owned()?;
        input.validate()?;
        if preview.fingerprint != input.fingerprint()
            || preview.expected != self.preview_version()?
        {
            return Err(SubscriptionOperationError::Busy);
        }
        let mut sources = Vec::new();
        let mut prepared = Vec::new();
        for (index, source) in input.sources.iter().enumerate() {
            let (view, import) = self.prepare_preview_source(index, source, &input).await;
            sources.push(view);
            if let Some(import) = import {
                prepared.push(import);
            }
        }
        if prepared.is_empty() {
            return Ok(PreviewSaveResult {
                sources,
                subscriptions: Vec::new(),
            });
        }
        let _access = self.gate.try_lock().map_err(map_access_error)?;
        let current = self.load_existing_or_empty()?;
        if current.as_ref().map(AppState::config_version) != preview.expected {
            return Err(SubscriptionOperationError::Busy);
        }
        let mut candidate = current.unwrap_or_else(AppState::empty);
        let first = candidate.subscriptions.len();
        for import in prepared {
            self.append_import(&mut candidate, import)?;
        }
        self.ensure_commit_open()?;
        let saved = self
            .store
            .commit(&candidate)
            .map_err(|_| SubscriptionOperationError::SaveFailed)?;
        let subscriptions = (first..saved.subscriptions.len())
            .map(|index| summary_for(&saved, index))
            .collect::<Vec<_>>();
        drop(_access);
        for subscription in &subscriptions {
            self.notify_change(&subscription.id, SubscriptionChange::Updated);
        }
        Ok(PreviewSaveResult {
            sources,
            subscriptions,
        })
    }

    async fn prepare_preview_source(
        &self,
        index: usize,
        source: &PreviewSource,
        input: &PreviewInput,
    ) -> (SourcePreview, Option<ImportCommit>) {
        let (kind, safe_display) = match source {
            PreviewSource::Url(url) => (
                SubscriptionSourceKind::Remote,
                reqwest::Url::parse(url)
                    .ok()
                    .filter(|u| u.host_str().is_some())
                    .map(|u| u.origin().ascii_serialization())
                    .unwrap_or_else(|| "URL".into()),
            ),
            PreviewSource::Pasted(_) => (SubscriptionSourceKind::Manual, "pasted/manual".into()),
        };
        let mut view = SourcePreview {
            index,
            kind,
            safe_display,
            node_count: 0,
            skipped: Vec::new(),
            error: None,
        };
        let result = self.prepare_direct_import(source, input).await;
        match result {
            Ok(import) => {
                view.node_count = import.parsed.nodes.len();
                view.skipped = import.parsed.skipped.clone();
                (view, Some(import))
            }
            Err((error, skipped)) => {
                view.error = Some(error);
                view.skipped = skipped;
                (view, None)
            }
        }
    }

    async fn prepare_direct_import(
        &self,
        source: &PreviewSource,
        input: &PreviewInput,
    ) -> Result<ImportCommit, (SubscriptionOperationError, Vec<SkippedNode>)> {
        let prepare = async {
            let (body, source, remote_request, http_metadata, update_policy) = match source {
                PreviewSource::Pasted(content) => (
                    content.clone(),
                    SubscriptionSource::Manual,
                    None,
                    None,
                    SubscriptionUpdatePolicy::manual(),
                ),
                PreviewSource::Url(url) => {
                    validate_source_url(url).map_err(map_url_error)?;
                    let remote = RemoteRequestOptions::default_remote();
                    let fetched = self
                        .fetch_remote(url, ConditionalHeaders::default(), &remote, None)
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
                    let mut policy = SubscriptionUpdatePolicy::manual();
                    adopt_profile_interval(&mut policy, profile_update_interval_minutes);
                    (
                        body,
                        SubscriptionSource::Remote { url: url.clone() },
                        Some(remote),
                        Some(metadata),
                        policy,
                    )
                }
            };
            Ok((body, source, remote_request, http_metadata, update_policy))
        }
        .await
        .map_err(|e| (e, Vec::new()))?;
        let (body, source, remote_request, http_metadata, update_policy) = prepare;
        if body.is_empty() || body.len() > MAX_CONTENT_BYTES {
            return Err((SubscriptionOperationError::InvalidInput, Vec::new()));
        }
        // 空候选也保留逐项原因，禁止把全 unsupported 存成有效订阅。
        let skipped = parse_subscription(&body)
            .map(|p| p.skipped)
            .unwrap_or_default();
        let parsed = parse_content(&body).map_err(|e| (e, skipped))?;
        let document = document_for_body(body, parsed.format);
        Ok(ImportCommit {
            name: input.name.trim().into(),
            description: input.description.clone(),
            source,
            remote_request,
            update_policy,
            parsed,
            document,
            http_metadata,
            expected_initially_empty: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::{Read, Write},
        net::TcpListener,
        thread,
    };
    const FIRST_BODY: &str = r#"{"outbounds":[{"type":"socks","tag":"first","server":"127.0.0.1","server_port":1080,"version":"5"}]}"#;
    const SECOND_BODY: &str = r#"{"outbounds":[{"type":"socks","tag":"second","server":"127.0.0.1","server_port":1081,"version":"5"}]}"#;
    fn isolated_state_file(label: &str) -> std::path::PathBuf {
        std::env::temp_dir()
            .join(format!(
                "veyra-{label}-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ))
            .join("state.json")
    }
    fn manager() -> (SubscriptionManager, std::path::PathBuf) {
        let path = isolated_state_file("p201");
        (
            SubscriptionManager::new(path.clone(), StateAccessGate::default()).unwrap(),
            path,
        )
    }
    fn input(source: PreviewSource) -> PreviewInput {
        PreviewInput {
            name: "Fixture".into(),
            description: "manual metadata".into(),
            sources: vec![source],
        }
    }
    fn run<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }
    fn server(bodies: Vec<String>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/subscription", listener.local_addr().unwrap());
        let handle = thread::spawn(move || {
            for body in bodies {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut bytes = [0; 8192];
                let count = stream.read(&mut bytes).unwrap();
                assert!(count > 0, "fixture receives an HTTP request");
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .unwrap();
            }
        });
        (url, handle)
    }
    // 用户预览不得修改任何磁盘事实；保存重新校验原输入并能重启读取完整稳定 ID 图。
    #[test]
    fn pasted_preview_no_persistence_save_reload_and_input_cas() {
        let (m, path) = manager();
        let request = input(PreviewSource::Pasted(FIRST_BODY.into()));
        let preview = run(m.preview(&request)).unwrap();
        assert!(!path.exists());
        assert_eq!(preview.expected, None);
        assert_eq!(preview.node_count, 1);
        let saved = run(m.save_preview(request.clone(), &preview)).unwrap();
        assert_eq!(saved.subscriptions.len(), 1);
        let state = JsonStateStore::new(path.clone()).unwrap().load().unwrap();
        state.validate().unwrap();
        assert_eq!(state.subscriptions.len(), 1);
        assert_eq!(state.providers.len(), 1);
        assert_eq!(state.nodes.len(), 1);
        assert!(state.subscriptions[0].id.0.starts_with("sub-"));
        assert!(state.providers[0].id.0.starts_with("provider-"));
        assert!(state.nodes[0].id.0.starts_with("node-"));
        let before = fs::read(&path).unwrap();
        let preview = run(m.preview(&request)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), before);
        let after = m.store.load().unwrap();
        assert_eq!(after.config_version(), state.config_version());
        assert_eq!(after, state);
        let mut changed = request.clone();
        changed.name = "New input".into();
        assert_eq!(
            run(m.save_preview(changed, &preview)),
            Err(SubscriptionOperationError::Busy)
        );
        let replaced = AppState::empty();
        m.store.save(&replaced).unwrap();
        assert_eq!(
            run(m.save_preview(request, &preview)),
            Err(SubscriptionOperationError::Busy)
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 多 URL 按用户输入顺序保留部分成功、unsupported 与失败；DTO 不带 query/path/节点密钥。
    #[test]
    fn single_and_multi_url_partial_preview_and_one_commit() {
        let (m, path) = manager();
        let mixed = "proxies:\n  - name: same\n    type: socks5\n    server: fixture.invalid\n    port: 1080\n  - name: skipped-secret\n    type: unsupported\n    password: secret-node-token\n";
        let (url, handle) = server(vec![
            FIRST_BODY.into(),
            FIRST_BODY.into(),
            FIRST_BODY.into(),
        ]);
        let single = input(PreviewSource::Url(url.clone()));
        assert_eq!(run(m.preview(&single)).unwrap().node_count, 1);
        let (url2, handle2) = server(vec![mixed.into(), mixed.into()]);
        let mut request = single;
        request.sources.extend([
            PreviewSource::Url(url2),
            PreviewSource::Url("http://user:pass@localhost/?token=x".into()),
        ]);
        let preview = run(m.preview(&request)).unwrap();
        assert_eq!(
            preview.sources.iter().map(|s| s.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(preview.savable_sources, 2);
        assert_eq!(preview.node_count, 2);
        assert_eq!(
            preview.sources[1].skipped,
            vec![SkippedNode::UnsupportedProtocol]
        );
        assert_eq!(
            preview.sources[2].error,
            Some(SubscriptionOperationError::InvalidInput)
        );
        let display = format!("{preview:?}");
        for secret in [
            "private-query",
            "token=",
            "pass@",
            "secret-node-token",
            "skipped-secret",
            "/subscription",
        ] {
            assert!(!display.contains(secret));
        }
        assert!(!path.exists());
        let saved = run(m.save_preview(request, &preview)).unwrap();
        assert_eq!(
            saved.sources[2].error,
            Some(SubscriptionOperationError::InvalidInput)
        );
        assert_eq!(saved.subscriptions.len(), 2);
        let state = m.store.load().unwrap();
        assert_eq!(state.config_revision, 1);
        assert_eq!(state.providers.len(), 2);
        assert_ne!(state.providers[0].id, state.providers[1].id);
        handle.join().unwrap();
        handle2.join().unwrap();
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 刷新保持协议身份、手工选择/元数据；同名不同端点必须拥有不同 ID。
    #[test]
    fn refresh_identity_selection_same_name_and_failure_preserve_bytes() {
        let (m, path) = manager();
        let request = input(PreviewSource::Pasted(FIRST_BODY.into()));
        let preview = run(m.preview(&request)).unwrap();
        let saved = run(m.save_preview(request, &preview)).unwrap();
        let id = saved.subscriptions[0].id.clone();
        let mut state = m.store.load().unwrap();
        let node = state.nodes[0].id.clone();
        state.pools[0].selection = SelectionPolicy::Manual {
            selected_node_id: Some(node.clone()),
            pending_node_id: None,
        };
        state.pools[0].name = "hand-labelled pool".into();
        m.store.commit(&state).unwrap();
        let state = m.store.load().unwrap();
        let body = FIRST_BODY.replace("first", "renamed");
        run(m.refresh_direct(id.clone(), Some(body), state.config_version())).unwrap();
        let state = m.store.load().unwrap();
        assert_eq!(state.nodes[0].id, node);
        assert_eq!(
            state.pools[0].selection,
            SelectionPolicy::Manual {
                selected_node_id: Some(node),
                pending_node_id: None,
            }
        );
        assert_eq!(state.pools[0].name, "hand-labelled pool");
        assert_eq!(state.subscriptions[0].description, "manual metadata");
        let body = r#"{"outbounds":[{"type":"socks","tag":"same","server":"127.0.0.1","server_port":1080},{"type":"socks","tag":"same","server":"127.0.0.1","server_port":1081}]}"#;
        run(m.refresh_direct(id.clone(), Some(body.into()), state.config_version())).unwrap();
        let state = m.store.load().unwrap();
        assert_eq!(state.nodes.len(), 2);
        assert_ne!(state.nodes[0].id, state.nodes[1].id);
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            run(m.refresh_direct(id, Some("invalid".into()), state.config_version())),
            Err(SubscriptionOperationError::ParseFailed)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(m.store.load().unwrap(), state);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 下载/解析失败与旧 epoch 请求都保留完整旧保存 bytes；没有失败时间的持久化写回。
    #[test]
    fn direct_refresh_download_parse_failure_and_old_epoch() {
        let (m, path) = manager();
        let (url, handle) = server(vec![FIRST_BODY.into(), FIRST_BODY.into(), "invalid".into()]);
        let request = input(PreviewSource::Url(url));
        let preview = run(m.preview(&request)).unwrap();
        let saved = run(m.save_preview(request, &preview)).unwrap();
        let id = saved.subscriptions[0].id.clone();
        let state = m.store.load().unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            run(m.refresh_direct(id.clone(), None, state.config_version())),
            Err(SubscriptionOperationError::ParseFailed)
        );
        handle.join().unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(
            run(m.refresh_direct(id.clone(), None, state.config_version())),
            Err(SubscriptionOperationError::Network(
                crate::subscription::FetchError::ConnectionRefused
            ))
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(m.store.load().unwrap(), state);
        let mut next = state.clone();
        next.state_epoch = AppState::empty().state_epoch;
        m.store.save(&next).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            run(m.refresh_direct(id, None, state.config_version())),
            Err(SubscriptionOperationError::Busy)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 被路由引用的订阅删除必须给 typed conflict，不能部分删除 provider/nodes。
    #[test]
    fn delete_reference_conflict_and_version_check() {
        let (m, path) = manager();
        let request = input(PreviewSource::Pasted(SECOND_BODY.into()));
        let preview = run(m.preview(&request)).unwrap();
        let saved = run(m.save_preview(request, &preview)).unwrap();
        let mut state = m.store.load().unwrap();
        state.default_target = RouteTarget::Pool(state.pools[0].id.clone());
        m.store.commit(&state).unwrap();
        let state = m.store.load().unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            m.delete_at_version(
                saved.subscriptions[0].id.clone(),
                false,
                state.config_version()
            ),
            Err(SubscriptionOperationError::ReferenceConflict)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 延迟 download 已开始后整体替换 epoch：旧刷新/保存均不能写回。
    #[test]
    fn delayed_refresh_and_save_cannot_overwrite_replaced_epoch() {
        for saving in [false, true] {
            let (m, path) = manager();
            let m = Arc::new(m);
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let url = format!("http://{}/subscription", listener.local_addr().unwrap());
            let (started_tx, started_rx) = std::sync::mpsc::channel();
            let (release_tx, release_rx) = std::sync::mpsc::channel();
            let server = thread::spawn(move || {
                for index in 0..3 {
                    let (mut stream, _) = listener.accept().unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let mut bytes = [0; 8192];
                    let count = stream.read(&mut bytes).unwrap();
                    assert!(count > 0, "fixture receives an HTTP request");
                    if index == 2 {
                        started_tx.send(()).unwrap();
                        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                    }
                    write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        FIRST_BODY.len(),
                        FIRST_BODY
                    )
                    .unwrap();
                }
            });
            let request = input(PreviewSource::Url(url));
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .unwrap();
            let preview = runtime.block_on(m.preview(&request)).unwrap();
            let saved = runtime
                .block_on(m.save_preview(request.clone(), &preview))
                .unwrap();
            let state = m.store.load().unwrap();
            let preview = SubscriptionPreview {
                expected: Some(state.config_version()),
                ..preview
            };
            let worker = m.clone();
            let id = saved.subscriptions[0].id.clone();
            let task = runtime.spawn(async move {
                if saving {
                    worker.save_preview(request, &preview).await.map(|_| ())
                } else {
                    worker
                        .refresh_direct(id, None, state.config_version())
                        .await
                        .map(|_| ())
                }
            });
            started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            {
                let _access = m.gate.try_lock().unwrap();
                m.store.save(&AppState::empty()).unwrap();
            }
            let bytes = fs::read(&path).unwrap();
            release_tx.send(()).unwrap();
            assert_eq!(
                runtime.block_on(task).unwrap(),
                Err(SubscriptionOperationError::Busy)
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert!(m.store.load().unwrap().subscriptions.is_empty());
            server.join().unwrap();
            fs::remove_dir_all(path.parent().unwrap()).unwrap();
        }
    }
    // 预览遇到损坏或旧 schema 时不得隐式恢复/迁移、推进 epoch 或生成备份。
    #[test]
    fn preview_does_not_recover_corrupt_state() {
        let (m, path) = manager();
        m.store.save(&AppState::empty()).unwrap();
        m.store.save(&AppState::empty()).unwrap();
        let backup = path.with_extension("json.bak");
        let backup_bytes = fs::read(&backup).unwrap();
        fs::write(&path, b"{corrupt").unwrap();
        let inventory = fs::read_dir(path.parent().unwrap()).unwrap().count();
        assert_eq!(
            run(m.preview(&input(PreviewSource::Pasted(FIRST_BODY.into())))),
            Err(SubscriptionOperationError::StateUnavailable)
        );
        assert_eq!(fs::read(&path).unwrap(), b"{corrupt");
        assert_eq!(fs::read(&backup).unwrap(), backup_bytes);
        assert_eq!(
            fs::read_dir(path.parent().unwrap()).unwrap().count(),
            inventory
        );
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 正式 Direct 边界无代理 fallback；磁盘 commit 失败保留旧内容与元数据。
    #[test]
    fn refresh_commit_failure_and_non_direct_rejected() {
        let (m, path) = manager();
        let request = input(PreviewSource::Pasted(FIRST_BODY.into()));
        let preview = run(m.preview(&request)).unwrap();
        let saved = run(m.save_preview(request, &preview)).unwrap();
        let state = m.store.load().unwrap();
        let bytes = fs::read(&path).unwrap();
        fs::create_dir(path.with_extension("tmp")).unwrap();
        assert_eq!(
            run(m.refresh_direct(
                saved.subscriptions[0].id.clone(),
                Some(SECOND_BODY.into()),
                state.config_version()
            )),
            Err(SubscriptionOperationError::SaveFailed)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(m.store.load().unwrap(), state);
        fs::remove_dir(path.with_extension("tmp")).unwrap();
        let mut state = state;
        state.subscriptions[0].source = SubscriptionSource::Remote {
            url: "https://example.invalid/private?token=secret".into(),
        };
        state.subscriptions[0].remote_request = Some(RemoteRequestOptions {
            proxy_mode: SubscriptionProxyMode::ManagedCore,
            ..RemoteRequestOptions::default_remote()
        });
        m.store.commit(&state).unwrap();
        let state = m.store.load().unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            run(m.refresh_direct(
                saved.subscriptions[0].id.clone(),
                None,
                state.config_version()
            )),
            Err(SubscriptionOperationError::ProxyUnavailable)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // 保存手工编辑同时提交 metadata/source；失败不产生“名称已存但内容没存”的半更新。
    #[test]
    fn manual_edit_metadata_and_source_are_one_transaction() {
        let (m, path) = manager();
        let input = input(PreviewSource::Pasted(FIRST_BODY.into()));
        let preview = run(m.preview(&input)).unwrap();
        let saved = run(m.save_preview(input, &preview)).unwrap();
        let state = m.store.load().unwrap();
        let node_id = state.nodes[0].id.clone();
        let edit = EditSubscription {
            id: saved.subscriptions[0].id.clone(),
            name: Some("Edited".into()),
            description: Some("Confirmed".into()),
            url_replacement: None,
            remote_request: None,
            update_policy: None,
        };
        run(m.edit_direct_with_content(
            edit.clone(),
            Some(FIRST_BODY.replace("first", "renamed")),
            state.config_version(),
        ))
        .unwrap();
        let next = m.store.load().unwrap();
        assert_eq!(next.config_revision, state.config_revision + 1);
        assert_eq!(next.nodes[0].id, node_id);
        assert_eq!(next.nodes[0].name, "renamed");
        assert_eq!(next.subscriptions[0].name, "Edited");
        assert_eq!(next.subscriptions[0].description, "Confirmed");
        assert_eq!(next.subscriptions[0].source, SubscriptionSource::Manual);
        assert!(next.subscriptions[0].http_metadata.is_none());
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            run(m.edit_direct_with_content(
                EditSubscription {
                    name: Some("Must not save".into()),
                    ..edit
                },
                Some("bad content".into()),
                next.config_version()
            )),
            Err(SubscriptionOperationError::ParseFailed)
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
    // Preview 成功后的远端失效，在 Save 重校验时解释失败；零有效来源不生成任何保存事实。
    #[test]
    fn save_revalidates_download_and_all_failed_sources_do_not_commit() {
        let (m, path) = manager();
        let (url, server) = server(vec![FIRST_BODY.into(), "invalid".into()]);
        let input = input(PreviewSource::Url(url));
        let preview = run(m.preview(&input)).unwrap();
        assert_eq!(preview.savable_sources, 1);
        let saved = run(m.save_preview(input, &preview)).unwrap();
        assert!(saved.subscriptions.is_empty());
        assert_eq!(
            saved.sources[0].error,
            Some(SubscriptionOperationError::ParseFailed)
        );
        assert!(!path.exists());
        server.join().unwrap();
    }
}
