use crate::state_bridge::{AppEvent, Request};
use std::{path::PathBuf, sync::Arc};
use tokio::{
    runtime::{Builder, Runtime},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};
use veyra_core::{
    application::{
        state_access::StateAccessGate,
        state_service::{DesktopPreferencesService, ProfileService, SnapshotService},
    },
    storage::JsonStateStore,
};

#[cfg(test)]
thread_local! {
    pub(crate) static WRITERS_CREATED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub struct AppServices {
    pub manual_runtime: crate::runtime_service::RuntimeService,
    pub snapshots: Arc<SnapshotService>,
    pub shares: Arc<veyra_core::application::shares::ShareService>,
    pub network: Arc<veyra_core::application::network::NetworkService>,
    pub subscriptions: Arc<veyra_core::application::subscription_management::SubscriptionManager>,
    // Profile 保存复用 Core CAS writer；选择服务继续由各功能按契约接入。
    pub profiles: Arc<ProfileService>,
    pub preferences: Arc<DesktopPreferencesService>,
    pub assets: Arc<crate::visual_assets::VisualAssetStore>,
    root: PathBuf,
    gate: StateAccessGate,
    runtime: Runtime,
    sender: UnboundedSender<AppEvent>,
}
impl Drop for AppServices {
    fn drop(&mut self) {
        // Runtime 销毁会等待 blocking worker；先消费 Core 已有关闭信号取消下载，
        // 避免退出线程等待完整网络 timeout。普通窗口隐藏不销毁 AppServices。
        self.request_closing();
    }
}
impl AppServices {
    /// 真正 Quit 与服务释放共用；隐藏窗口不关闭网络 Service。
    pub fn request_closing(&self) {
        self.subscriptions.request_closing();
        self.network.shutdown();
        self.shares.shutdown();
    }

    /// 已由NSSavePanel确认的脱敏页面快照，在既有有界blocking worker写文件。
    pub fn save_log_export(
        &self,
        selection: Result<crate::platform::macos::DialogResult, crate::platform::PlatformError>,
        bytes: Vec<u8>,
    ) -> tokio::sync::oneshot::Receiver<Result<bool, crate::platform::PlatformError>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        match selection {
            Ok(crate::platform::macos::DialogResult::Cancelled) => {
                let _ = sender.send(Ok(false));
            }
            Err(error) => {
                let _ = sender.send(Err(error));
            }
            Ok(crate::platform::macos::DialogResult::Selected(path)) => {
                self.runtime.spawn_blocking(move || {
                    let _ = sender
                        .send(crate::platform::files::save_selected(&path, &bytes).map(|_| true));
                });
            }
        }
        receiver
    }
    pub fn new(
        root: PathBuf,
    ) -> Result<(Self, UnboundedReceiver<AppEvent>), Box<dyn std::error::Error>> {
        let gate = StateAccessGate::default();
        let snapshots = SnapshotService::new(
            JsonStateStore::new(
                crate::platform::directories::AppDirectories::injected(root.clone()).state(),
            )?,
            gate.clone(),
        );
        #[cfg(test)]
        WRITERS_CREATED.set(WRITERS_CREATED.get() + 1);
        let (sender, receiver) = unbounded_channel();
        let mut subscriptions =
            veyra_core::application::subscription_management::SubscriptionManager::new(
                root.join("state.json"),
                gate.clone(),
            )?;
        let snapshots = Arc::new(snapshots);
        let manual_runtime = crate::runtime_service::RuntimeService::new(
            root.clone(),
            snapshots.clone(),
            sender.clone(),
        );
        subscriptions.bind_managed_proxy_source(manual_runtime.outbound_proxy_source());
        let subscriptions = Arc::new(subscriptions);
        let network = Arc::new(veyra_core::application::network::NetworkService::new(
            manual_runtime.outbound_proxy_source(),
        ));
        let runtime = Builder::new_multi_thread()
            .worker_threads(2)
            .max_blocking_threads(1)
            .enable_all()
            .build()?;
        let shares = Arc::new(veyra_core::application::shares::ShareService::new(
            snapshots.clone(),
            runtime.handle().clone(),
        ));
        let restore = shares.clone();
        // 启动恢复失败留给分享页面重试并显示真实错误，不阻止其它页面使用。
        if root.join("state.json").exists() {
            runtime.spawn_blocking(move || {
                let _ = restore.restore();
            });
        }
        let services = Self {
            shares,

            manual_runtime,
            network,
            subscriptions,
            profiles: Arc::new(ProfileService::new((*snapshots).clone())),
            preferences: Arc::new(DesktopPreferencesService::new((*snapshots).clone())),
            assets: Arc::new(crate::visual_assets::VisualAssetStore::new(
                root.join("assets"),
            )),
            root,
            gate,
            snapshots,
            runtime,
            sender,
        };
        Ok((services, receiver))
    }
    pub fn shared_network_command(
        &self,
        expected: Option<veyra_core::domain::ConfigVersion>,
        command: crate::shared_network::Command,
    ) -> tokio::sync::oneshot::Receiver<crate::shared_network::Completion> {
        let snapshots = self.snapshots.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.runtime.spawn_blocking(move || {
            let _ = sender.send(crate::shared_network::execute(
                &snapshots, expected, command,
            ));
        });
        receiver
    }
    pub fn shared_certificate(
        &self,
    ) -> tokio::sync::oneshot::Receiver<Result<veyra_core::domain::SharedTls, &'static str>> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.runtime.spawn_blocking(move || {
            let _ = sender.send(
                veyra_core::application::shared_inbounds::sharing::generate_shared_certificate()
                    .map_err(|_| "证书生成失败，请重试"),
            );
        });
        receiver
    }
    pub fn shares_command(
        &self,
        expected: Option<veyra_core::domain::ConfigVersion>,
        command: Option<veyra_core::application::shares::ShareCommand>,
    ) -> tokio::sync::oneshot::Receiver<
        Result<veyra_core::domain::AppState, veyra_core::application::shares::ShareError>,
    > {
        let shares = self.shares.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.runtime.spawn_blocking(move || {
            let result = match command {
                Some(command) => expected
                    .ok_or(veyra_core::application::shares::ShareError::Storage)
                    .and_then(|version| shares.execute(version, command)),
                None => shares.restore(),
            };
            let _ = sender.send(result);
        });
        receiver
    }
    /// GPUI 不获得 Controller；所有 Group 操作投递同一个 Runtime worker。
    pub fn groups_runtime_command(&self, event: crate::ui::groups::GroupsRuntimeEvent) {
        let request = event.request;
        let reply = if let Some(selection) = event.selection {
            self.manual_runtime.select_manual(selection)
        } else {
            self.manual_runtime
                .reconcile_selection(event.instance, event.expected, event.group)
        };
        let snapshots = self.snapshots.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = reply.recv().unwrap_or(Err(
                veyra_core::application::manual_runtime::SelectionError::StaleInstance,
            ));
            let snapshot = snapshots.snapshot().ok().map(Box::new);
            let _ = sender.send(AppEvent::GroupSelection {
                request,
                result,
                snapshot,
            });
        });
    }
    pub fn groups_command(&self, event: crate::ui::groups::GroupsEvent) {
        let snapshots = self.snapshots.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let saved = event.save.is_some();
            let result = match event.save {
                Some((expected, groups)) => snapshots
                    .save_groups(expected, groups)
                    .map(|out| Box::new(out.value)),
                None => snapshots
                    .snapshot()
                    .map(Box::new)
                    .map_err(veyra_core::application::state_service::GroupSaveError::Storage),
            };
            // 失败保留草稿并重读权威版本，用户可以修正后再保存。
            let snapshot = if result.is_err() {
                snapshots.snapshot().ok().map(Box::new)
            } else {
                None
            };
            let _ = sender.send(AppEvent::Groups {
                request: event.request,
                result,
                saved,
                snapshot,
            });
        });
    }
    pub fn save_backend_profile(&self, request: crate::backend_profile::Request) {
        let profiles = self.profiles.clone();
        let snapshots = self.snapshots.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = request
                .change
                .patch()
                .and_then(|patch| profiles.patch(request.expected.clone(), patch));
            // 失败也重读：CAS 冲突只能由权威快照恢复，不能以 UI 草稿覆盖外部写入。
            let snapshot = snapshots.snapshot().map(Box::new);
            let _ = sender.send(AppEvent::BackendProfile {
                request,
                result,
                snapshot,
            });
        });
    }
    pub fn subscription_command(
        &self,
        request: crate::subscriptions::Request,
        command: crate::subscriptions::Command,
    ) {
        let manager = self.subscriptions.clone();
        let snapshots = self.snapshots.clone();
        let sender = self.sender.clone();
        let handle = self.runtime.handle().clone();
        // 包含网络 await、parse 与磁盘 commit 的整段操作在服务 blocking worker 上执行，
        // 既不阻塞 GPUI，也不在 Tokio reactor 执行同步解析/存储。
        self.runtime.spawn_blocking(move || {
            use crate::subscriptions::{Command, Completion};
            use veyra_core::application::subscription_management::SubscriptionOperationError as Error;
            let result = handle.block_on(async {
                match command {
                    Command::Load => snapshots.snapshot()
                        .map(|state| Completion::Loaded(Box::new(state)))
                        .map_err(|_| Error::StateUnavailable),
                    #[cfg(test)]
                    Command::Preview(input) => manager.preview(&input).await.map(Completion::Preview),
                    Command::SaveDraft(input) => {
                        let preview = manager.preview(&input).await?;
                        if preview.expected != request.expected { return Err(Error::Busy); }
                        let saved = manager.save_preview(input, &preview).await?;
                        if saved.subscriptions.is_empty() { return Ok(Completion::SaveRejected(saved)); }
                        let state = snapshots.snapshot().map_err(|_| Error::StateUnavailable)?;
                        Ok(Completion::Mutation { state: Box::new(state), saved: Some(saved) })
                    }
                    #[cfg(test)]
                    Command::Save(input, preview) => {
                        if preview.expected != request.expected { return Err(Error::Busy); }
                        let saved = manager.save_preview(input, &preview).await?;
                        if saved.subscriptions.is_empty() { return Ok(Completion::SaveRejected(saved)); }
                        let state = snapshots.snapshot().map_err(|_| Error::StateUnavailable)?;
                        Ok(Completion::Mutation { state: Box::new(state), saved: Some(saved) })
                    }
                    command => {
                        let expected = request.expected.clone().ok_or(Error::StateUnavailable)?;
                        let saved = match command {

                            Command::Edit(input, content) => {
                                manager.edit_direct_with_content(input, content, expected).await?;
                                None
                            }
                            Command::Refresh { id, content } => {
                                manager.refresh_direct(id, content, expected).await?;
                                None
                            }
                            // 尚无 Runtime owner；绝不把未知活动状态冒充“已停止”。
                            Command::Delete(id) => {
                                manager.delete_at_version(id, false, expected)?;
                                None
                            }
                            Command::Load | Command::SaveDraft(_) => unreachable!(),
                            #[cfg(test)]
                            Command::Preview(_) | Command::Save(_,_) => unreachable!(),
                        };
                        let state = snapshots.snapshot().map_err(|_| Error::StateUnavailable)?;
                        Ok(Completion::Mutation { state: Box::new(state), saved })
                    }
                }
            });
            let _ = sender.send(AppEvent::Subscriptions { request, result });
        });
    }

    pub fn save_behavior(&self, request: crate::behavior_preferences::BehaviorSave) {
        let preferences = self.preferences.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = preferences
                .patch_behavior(request.expected, request.patch)
                .map(|outcome| Box::new(outcome.value));
            let _ = sender.send(AppEvent::BehaviorSaved {
                generation: request.generation,
                result,
            });
        });
    }
    #[cfg(debug_assertions)]
    pub fn evidence_external_preferences(&self) {
        let preferences = self.preferences.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = preferences.behavior_snapshot().and_then(|current| {
                preferences
                    .patch_behavior(
                        current.version,
                        veyra_core::domain::DesktopBehaviorPreferencesPatch {
                            group_columns: Some(if current.value.proxy_view.group_columns == 3 {
                                1
                            } else {
                                3
                            }),
                            ..Default::default()
                        },
                    )
                    .map(|saved| saved.version)
            });
            // Deliberately do not replace the UI snapshot: this is a real competing writer.
            let _ = sender.send(AppEvent::ExternalPreferenceWrite { result });
        });
    }
    pub fn save_platform_evidence(&self, path: PathBuf) {
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = crate::platform::macos::DialogResult::Selected(path)
                .save_fixture()
                .map(|_| ());
            let _ = sender.send(AppEvent::PlatformSaved { result });
        });
    }
    pub fn save_visual(&self, request: crate::preferences::VisualSave) {
        let preferences = self.preferences.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = preferences
                .save(request.expected, request.draft)
                .map(|outcome| Box::new(outcome.value));
            let _ = sender.send(AppEvent::VisualSaved {
                generation: request.generation,
                result,
            });
        });
    }
    pub fn import_background(&self, generation: u64, path: PathBuf) {
        let assets = self.assets.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = assets.import(&path);
            let _ = sender.send(AppEvent::AssetPrepared { generation, result });
        });
    }
    pub fn default_background(&self, request: crate::ui::background::Request) {
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let started = std::time::Instant::now();
            let bytes = crate::ui::background::derive(request);
            eprintln!(
                "default background CPU job generation={} thread={:?} duration_ms={}",
                request.generation,
                std::thread::current().id(),
                started.elapsed().as_millis()
            );
            let _ = sender.send(AppEvent::DefaultBackground { request, bytes });
        });
    }
    pub fn background_preview(
        &self,
        generation: u64,
        id: veyra_core::domain::VisualAssetId,
        blur: u8,
    ) {
        let assets = self.assets.clone();
        let sender = self.sender.clone();
        self.runtime.spawn_blocking(move || {
            let result = assets.preview(&id, blur);
            let _ = sender.send(AppEvent::BackgroundPreview { generation, result });
        });
    }
    pub fn collect_asset_orphans(&self, draft: veyra_core::domain::DesktopBackground) {
        let assets = self.assets.clone();
        let root = self.root.clone();
        let gate = self.gate.clone();
        self.runtime.spawn_blocking(move || {
            let Ok(_access) = gate.try_lock() else {
                eprintln!("orphan cleanup deferred: state access busy");
                return;
            };
            let read = |name| {
                std::fs::read(root.join(name)).ok().and_then(|bytes| {
                    serde_json::from_slice::<veyra_core::domain::AppState>(&bytes).ok()
                })
            };
            // Current state, recovery backup and the pending draft are live references.
            // Keep assets if a reference cannot be decoded; never race a core writer.
            if let (Some(current), Some(backup)) = (read("state.json"), read("state.json.bak"))
                && let Err(error) = assets.collect_orphans(&[
                    current.app_config.visual.background,
                    backup.app_config.visual.background,
                    draft,
                ])
            {
                eprintln!("orphan cleanup deferred: {error}");
            }
        });
    }
    pub fn refresh(&self, request: Request) {
        self.read(request, 0);
    }
    #[cfg(debug_assertions)]
    pub fn evidence_refresh(&self, request: Request, delay_ms: u64) {
        self.read(request, delay_ms.min(3000));
    }
    fn read(&self, request: Request, delay_ms: u64) {
        let snapshots = self.snapshots.clone();
        let sender = self.sender.clone();
        let handle = self.runtime.handle().clone();
        // This future captures core services + typed values only. No GPUI imports here.
        self.runtime.spawn(async move {
            let result = handle
                .spawn_blocking(move || snapshots.snapshot().map(Box::new))
                .await
                .expect("snapshot worker panicked");
            // Delay delivery AFTER the real read so A is genuinely an older result.
            #[cfg(debug_assertions)]
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            #[cfg(not(debug_assertions))]
            let _ = delay_ms;
            let _ = sender.send(AppEvent::Snapshot { request, result });
        });
    }
    #[cfg(debug_assertions)]
    pub fn synthetic_event(&self, event: AppEvent) {
        let _ = self.sender.send(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_bridge::{Disposition, LoadState, StateBridge};
    /// 保护正式 Desktop Worker 连续保存/轮换及 AppServices 退出释放监听。
    #[test]
    fn sharing_worker_roundtrip_and_application_drop_release_port() {
        use std::io::{Read, Write};
        use veyra_core::application::{
            shares::{ShareCommand, ShareService},
            subscription_management::preview::{PreviewInput, PreviewSource},
        };
        let root = std::env::temp_dir().join(format!(
            "veyra-p506-desktop-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let (services, _rx) = AppServices::new(root.clone()).unwrap();
        let input = PreviewInput {
            name: "owned".into(),
            description: String::new(),
            sources: vec![PreviewSource::Pasted(
                "socks5://127.0.0.1:1080#owned".into(),
            )],
        };
        services.runtime.block_on(async {
            let preview = services.subscriptions.preview(&input).await.unwrap();
            services
                .subscriptions
                .save_preview(input, &preview)
                .await
                .unwrap();
        });
        let state = services.snapshots.snapshot().unwrap();
        let socket = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = socket.local_addr().unwrap();
        drop(socket);
        let mut share = ShareService::draft().unwrap();
        share.name = "owned".into();
        share.listen = address;
        share.host = address.to_string();
        share.subscription_ids = vec![state.subscriptions[0].id.clone()];
        let state = services
            .shares_command(
                Some(state.config_version()),
                Some(ShareCommand::Save(share.clone())),
            )
            .blocking_recv()
            .unwrap()
            .unwrap();
        let state = services
            .shares_command(
                Some(state.config_version()),
                Some(ShareCommand::Regenerate(share.id)),
            )
            .blocking_recv()
            .unwrap()
            .unwrap();
        let token = &state.app_config.subscription_shares[0].token;
        let mut stream = std::net::TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "GET /sub/{token} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        drop(stream);
        drop(services);
        let rebound = std::net::TcpListener::bind(address).unwrap();
        drop(rebound);
        std::fs::remove_dir_all(root).unwrap();
    }
    /// 保护删除分享后重建正式 Service 的首次全局读取；不靠导航或延时恢复。
    #[test]
    fn sharing_deleted_restart_initial_snapshot_reaches_ready() {
        use veyra_core::application::shares::{ShareCommand, ShareService};
        let root = std::env::temp_dir().join(format!(
            "veyra-p506-startup-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let (services, _) = AppServices::new(root.clone()).unwrap();
        use veyra_core::application::subscription_management::preview::{
            PreviewInput, PreviewSource,
        };
        let input = PreviewInput {
            name: "owned".into(),
            description: String::new(),
            sources: vec![PreviewSource::Pasted(
                "socks5://127.0.0.1:1080#owned".into(),
            )],
        };
        services.runtime.block_on(async {
            let preview = services.subscriptions.preview(&input).await.unwrap();
            services
                .subscriptions
                .save_preview(input, &preview)
                .await
                .unwrap();
        });
        let state = services.snapshots.snapshot().unwrap();
        let mut share = ShareService::draft().unwrap();
        share.subscription_ids = vec![state.subscriptions[0].id.clone()];
        share.name = "startup-owned".into();
        share.enabled = false;
        let state = services
            .shares_command(
                Some(state.config_version()),
                Some(ShareCommand::Save(share.clone())),
            )
            .blocking_recv()
            .unwrap()
            .unwrap();
        services
            .shares_command(
                Some(state.config_version()),
                Some(ShareCommand::Delete(share.id)),
            )
            .blocking_recv()
            .unwrap()
            .unwrap();
        drop(services);
        let before = std::fs::read(root.join("state.json")).unwrap();
        let mut failures = 0;
        for iteration in 0..80 {
            let (services, mut rx) = AppServices::new(root.clone()).unwrap();
            let mut bridge = StateBridge::default();
            services.refresh(bridge.begin());
            services.runtime.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    while matches!(bridge.load, LoadState::Busy) {
                        let event = rx.recv().await.unwrap();
                        if let AppEvent::Snapshot {
                            result: Err(ref error),
                            ..
                        } = event
                        {
                            eprintln!("restart={iteration} initial_snapshot_error={error:?}");
                        }
                        bridge.receive(event);
                    }
                })
                .await
                .unwrap();
            });
            if !matches!(bridge.load, LoadState::Ready) {
                failures += 1;
            }
            assert!(
                bridge
                    .snapshot
                    .as_ref()
                    .unwrap()
                    .app_config
                    .subscription_shares
                    .is_empty()
            );
            // 与 composition root 相同：首次读取完成才投递 Runtime Refresh。
            let request = bridge
                .begin_initial_runtime_refresh(Disposition::Accepted)
                .unwrap();
            services.manual_runtime.submit(
                request,
                veyra_core::application::manual_runtime::RuntimeCommand::Refresh,
            );
            services.runtime.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    while bridge.runtime_operation.is_some() {
                        bridge.receive(rx.recv().await.unwrap());
                    }
                })
                .await
                .unwrap();
            });
            assert!(bridge.runtime_result.as_ref().unwrap().is_ok());
            assert!(matches!(bridge.load, LoadState::Ready));
            drop(services);
            assert_eq!(std::fs::read(root.join("state.json")).unwrap(), before);
        }
        std::fs::remove_dir_all(root).unwrap();
        assert_eq!(
            failures, 0,
            "initial global reads failed without navigation"
        );
    }
    // Protect real StateStore errors/retry through the production channel and runtime.
    #[test]
    fn real_store_empty_error_retry_and_late_delivery() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-03-test-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let mut bridge = StateBridge::default();
        let request = bridge.begin();
        services.refresh(request);
        assert_eq!(
            bridge.receive(rx.blocking_recv().unwrap()),
            Disposition::Accepted
        );
        assert!(bridge.snapshot.as_ref().unwrap().nodes.is_empty());
        std::fs::write(root.join("state.json"), b"{malformed").unwrap();
        let request = bridge.begin();
        services.refresh(request);
        bridge.receive(rx.blocking_recv().unwrap());
        assert!(matches!(bridge.load, LoadState::Error(_)));
        std::fs::remove_file(root.join("state.json")).unwrap();
        let request = bridge.begin();
        services.refresh(request);
        bridge.receive(rx.blocking_recv().unwrap());
        assert!(matches!(bridge.load, LoadState::Ready));
        let a = bridge.begin();
        services.evidence_refresh(a, 500);
        let b = bridge.begin();
        let id = b.id;
        services.evidence_refresh(b, 50);
        assert_eq!(
            bridge.receive(rx.blocking_recv().unwrap()),
            Disposition::Accepted
        );
        assert_eq!(
            bridge.receive(rx.blocking_recv().unwrap()),
            Disposition::StaleRequest
        );
        assert_eq!(bridge.accepted_request, Some(id));
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod visual_tests {
    use super::*;
    use crate::preferences::{SaveCoordinator, VisualSave};
    use veyra_core::domain::*;
    #[test]
    fn prepared_background_failed_commit_retry_and_backup_reference_are_atomic() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-04a-atomic-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let a = root.join("a.png");
        let b = root.join("b.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([0, 180, 100, 255]))
            .save(&a)
            .unwrap();
        image::RgbaImage::from_pixel(2, 2, image::Rgba([180, 100, 0, 255]))
            .save(&b)
            .unwrap();
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let before = services.snapshots.snapshot().unwrap();
        let id_a = services.assets.import(&a).unwrap();
        let id_b = services.assets.import(&b).unwrap();
        let mut draft = before.app_config.visual.clone();
        draft.background = DesktopBackground::ManagedAsset(id_a.clone());
        let saved_a = services
            .preferences
            .save(before.config_version(), draft.clone())
            .unwrap()
            .value;
        std::fs::create_dir(root.join("state.tmp")).unwrap();
        draft.background = DesktopBackground::ManagedAsset(id_b.clone());
        services.save_visual(VisualSave {
            generation: 1,
            expected: saved_a.config_version(),
            draft: draft.clone(),
        });
        let AppEvent::VisualSaved { result, .. } = rx.blocking_recv().unwrap() else {
            panic!("typed result")
        };
        assert_eq!(result.unwrap_err().code(), AppErrorCode::StorageFailed);
        assert_eq!(services.snapshots.snapshot().unwrap(), saved_a);
        assert!(services.assets.path(&id_a).exists());
        assert!(services.assets.path(&id_b).exists());
        std::fs::remove_dir(root.join("state.tmp")).unwrap();
        services.save_visual(VisualSave {
            generation: 2,
            expected: saved_a.config_version(),
            draft: draft.clone(),
        });
        let AppEvent::VisualSaved { result, .. } = rx.blocking_recv().unwrap() else {
            panic!("typed result")
        };
        let saved_b = result.unwrap();
        assert_eq!(saved_b.app_config.visual, draft);
        assert_eq!(saved_b.state_epoch, before.state_epoch);
        assert_eq!(saved_b.selection_revision, before.selection_revision);
        assert!(
            services.assets.path(&id_a).exists(),
            "backup still references A"
        );
        let backup: AppState =
            serde_json::from_slice(&std::fs::read(root.join("state.json.bak")).unwrap()).unwrap();
        assert_eq!(
            backup.app_config.visual.background,
            DesktopBackground::ManagedAsset(id_a.clone())
        );
        draft.global_radius = 8;
        let current = services
            .preferences
            .save(saved_b.config_version(), draft.clone())
            .unwrap()
            .value;
        let backup: AppState =
            serde_json::from_slice(&std::fs::read(root.join("state.json.bak")).unwrap()).unwrap();
        services
            .assets
            .collect_orphans(&[
                current.app_config.visual.background.clone(),
                backup.app_config.visual.background,
            ])
            .unwrap();
        assert!(!services.assets.path(&id_a).exists());
        assert!(services.assets.path(&id_b).exists());
        let preview = services.assets.preview(&id_b, 10).unwrap();
        assert!(preview.exists());
        assert_eq!(services.assets.preview(&id_b, 10).unwrap(), preview);
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rapid_slider_coalesces_real_commit_and_restart_projection() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-04a-coalesce-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let before = services.snapshots.snapshot().unwrap();
        let mut coordinator = SaveCoordinator::default();
        coordinator.restore(&before.app_config.visual);
        for (i, value) in [10, 20, 30, 40].into_iter().enumerate() {
            let mut draft = coordinator.draft.clone();
            draft.background_opacity = value;
            coordinator.edit(draft, i as u64 * 10);
        }
        services.save_visual(coordinator.take_due(430, before.config_version()).unwrap());
        let AppEvent::VisualSaved { generation, result } = rx.blocking_recv().unwrap() else {
            panic!("typed result")
        };
        let saved = result.unwrap();
        coordinator.complete(generation, Ok(()));
        assert_eq!(saved.config_revision, before.config_revision + 1);
        assert_eq!(saved.app_config.visual.background_opacity, 40);
        drop(services);
        let (restarted, _) = AppServices::new(root.clone()).unwrap();
        let restored = restarted.snapshots.snapshot().unwrap();
        let mut projection = SaveCoordinator::default();
        projection.restore(&restored.app_config.visual);
        assert_eq!(projection.draft, saved.app_config.visual);
        assert_eq!(restored.state_epoch, before.state_epoch);
        assert_eq!(restored.selection_revision, before.selection_revision);
        drop(restarted);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod behavior_integration_tests {
    use super::*;
    use crate::behavior_preferences::{BehaviorCoordinator, BehaviorStatus};
    use veyra_core::domain::*;
    #[test]
    fn real_store_failure_conflict_rebase_retry_and_restart() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p1-04b-test-{:?}",
            StateEpoch::fresh().unwrap()
        ));
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let mut c = BehaviorCoordinator::default();
        let initial = services.snapshots.snapshot().unwrap();
        c.rebase(&initial);
        c.edit(DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(7500),
            ..Default::default()
        });
        std::fs::create_dir(root.join("state.tmp")).unwrap();
        let save = c.submit().unwrap();
        services.save_behavior(save);
        let AppEvent::BehaviorSaved { generation, result } = rx.blocking_recv().unwrap() else {
            panic!("typed behavior event")
        };
        assert_eq!(
            result.as_ref().unwrap_err().code(),
            AppErrorCode::StorageFailed
        );
        c.complete(generation, result.as_deref().map_err(Clone::clone));
        assert_eq!(c.draft.latency.timeout_ms, 7500);
        assert_eq!(
            services
                .snapshots
                .snapshot()
                .unwrap()
                .app_config
                .behavior
                .latency
                .timeout_ms,
            5000
        );
        std::fs::remove_dir(root.join("state.tmp")).unwrap();
        let external = services
            .preferences
            .patch_behavior(
                initial.config_version(),
                DesktopBehaviorPreferencesPatch {
                    group_columns: Some(3),
                    ..Default::default()
                },
            )
            .unwrap();
        services.save_behavior(c.submit().unwrap());
        let AppEvent::BehaviorSaved { generation, result } = rx.blocking_recv().unwrap() else {
            panic!("typed behavior event")
        };
        assert_eq!(
            result.as_ref().unwrap_err().code(),
            AppErrorCode::RevisionConflict
        );
        c.complete(generation, result.as_deref().map_err(Clone::clone));
        c.rebase(&external.value);
        services.save_behavior(c.submit().unwrap());
        let AppEvent::BehaviorSaved { generation, result } = rx.blocking_recv().unwrap() else {
            panic!("typed behavior event")
        };
        c.complete(generation, result.as_deref().map_err(Clone::clone));
        assert_eq!(c.status, BehaviorStatus::Idle);
        assert_eq!(
            (c.draft.latency.timeout_ms, c.draft.proxy_view.group_columns),
            (7500, 3)
        );
        let saved = services.snapshots.snapshot().unwrap();
        assert_eq!(saved.state_epoch, initial.state_epoch);
        assert_eq!(saved.selection_revision, initial.selection_revision);
        assert_eq!(saved.config_revision, initial.config_revision + 2);
        drop(services);
        let (restarted, _) = AppServices::new(root.clone()).unwrap();
        assert_eq!(restarted.snapshots.snapshot().unwrap(), saved);
        drop(restarted);
        std::fs::remove_dir_all(root).unwrap();
    }
    // 真实 AppServices/channel：只用隔离 JSON 和 pasted fixture，保护 UI busy/error/retry/保存投影。
    #[test]
    fn subscriptions_bridge_preview_save_edit_refresh_delete_and_stale() {
        use crate::subscriptions::{Command, Completion, Model, Status};
        use veyra_core::application::subscription_management::{
            EditSubscription, SubscriptionOperationError as Error,
            preview::{PreviewInput, PreviewSource},
        };
        let root = std::env::temp_dir().join(format!(
            "veyra-p201-bridge-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let receive = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>| {
            services.runtime.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
                    .await
                    .unwrap()
                    .unwrap()
            })
        };
        let mut model = Model::default();
        assert_eq!(model.status, Status::Empty);
        let input = PreviewInput {
            name: "Fixture".into(),
            description: String::new(),
            sources: vec![PreviewSource::Pasted("socks5://127.0.0.1:1080#one".into())],
        };
        let old = model.begin();
        services.subscription_command(old.clone(), Command::Preview(input.clone()));
        model.invalidate();
        let request = model.begin();
        services.subscription_command(request, Command::Preview(input.clone()));
        assert!(model.busy());
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        assert_eq!(request, old);
        model.complete(&request, result);
        assert!(model.busy());
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        model.complete(&request, result);
        assert_eq!(model.status, Status::Preview);
        assert!(
            !root.join("state.json").exists(),
            "preview must not initialize storage"
        );
        let receipt = model.preview.clone().unwrap();
        let request = model.begin();
        services.subscription_command(request, Command::Save(input, receipt));
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        let state = model.complete(&request, result).unwrap();
        assert_eq!(model.status, Status::Saved);
        assert_eq!(model.list.len(), 1);
        assert_eq!(services.snapshots.snapshot().unwrap(), *state);
        let id = model.list[0].id.clone();
        let request = model.begin();
        services.subscription_command(
            request,
            Command::Edit(
                EditSubscription {
                    id: id.clone(),
                    name: Some("Edited".into()),
                    description: Some("Manual metadata".into()),
                    url_replacement: None,
                    remote_request: None,
                    update_policy: None,
                },
                None,
            ),
        );
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        model.complete(&request, result);
        assert_eq!(model.list[0].name, "Edited");
        let request = model.begin();
        services.subscription_command(
            request,
            Command::Refresh {
                id: id.clone(),
                content: Some("invalid".into()),
            },
        );
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        model.complete(&request, result);
        assert_eq!(model.status, Status::Error(Error::ParseFailed));
        let request = model.begin();
        services.subscription_command(
            request,
            Command::Refresh {
                id: id.clone(),
                content: Some("socks5://127.0.0.1:1080#renamed".into()),
            },
        );
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        model.complete(&request, result);
        assert_eq!(model.status, Status::Saved);
        let request = model.begin();
        services.subscription_command(request, Command::Delete(id));
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        model.complete(&request, result);
        assert_eq!(model.status, Status::Saved);
        assert!(model.list.is_empty());
        let request = model.begin();
        services.subscription_command(request, Command::Load);
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        assert!(matches!(result, Ok(Completion::Loaded(_))));
        model.complete(&request, result);
        assert_eq!(model.status, Status::Empty);
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
    // Host删除GUI预览区后，保护一次Save完成校验/原子保存及旧版本拒绝，不假保存失败来源。
    #[test]
    fn subscription_save_draft_is_one_operation_and_preserves_version_gate() {
        use crate::subscriptions::{Command, Completion, Model, Status};
        use veyra_core::application::subscription_management::{
            SubscriptionOperationError as Error,
            preview::{PreviewInput, PreviewSource},
        };
        let root = std::env::temp_dir().join(format!(
            "veyra-save-draft-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (services, mut rx) = AppServices::new(root.clone()).unwrap();
        let receive = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<AppEvent>| {
            services.runtime.block_on(async {
                tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
                    .await
                    .unwrap()
                    .unwrap()
            })
        };
        let input = |content: &str| PreviewInput {
            name: "One click".into(),
            description: "Description".into(),
            sources: vec![PreviewSource::Pasted(content.into())],
        };
        let mut model = Model::default();
        services.subscription_command(model.begin(), Command::SaveDraft(input("invalid")));
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        assert!(matches!(&result, Ok(Completion::SaveRejected(_))));
        model.complete(&request, result);
        assert!(!root.join("state.json").exists());
        let stale = model.begin();
        let current = model.begin();
        services.subscription_command(
            current,
            Command::SaveDraft(input("socks5://127.0.0.1:1080#one")),
        );
        let AppEvent::Subscriptions { request, result } = receive(&mut rx) else {
            panic!("subscription event")
        };
        let state = model.complete(&request, result).unwrap();
        assert_eq!(model.status, Status::Saved);
        assert_eq!(model.list.len(), 1);
        assert_eq!(state.config_revision, 1);
        let bytes = std::fs::read(root.join("state.json")).unwrap();
        services.subscription_command(
            stale,
            Command::SaveDraft(input("socks5://127.0.0.1:1081#two")),
        );
        let AppEvent::Subscriptions { result, .. } = receive(&mut rx) else {
            panic!("subscription event")
        };
        assert!(matches!(result, Err(Error::Busy)));
        assert_eq!(std::fs::read(root.join("state.json")).unwrap(), bytes);
        drop(services);
        let (restarted, _) = AppServices::new(root.clone()).unwrap();
        assert_eq!(restarted.snapshots.snapshot().unwrap(), *state);
        drop(restarted);
        std::fs::remove_dir_all(root).unwrap();
    }
    // 保护退出时取消正在读取的下载：复用既有 closing watch，而非 UI 等网络预算结束。
    #[test]
    fn subscriptions_service_drop_cancels_stalled_download_without_initializing_state() {
        use crate::subscriptions::{Command, Model};
        use std::{
            io::{Read, Write},
            net::TcpListener,
            time::{Duration, Instant},
        };
        use veyra_core::application::subscription_management::preview::{
            PreviewInput, PreviewSource,
        };
        let root = std::env::temp_dir().join(format!(
            "veyra-p201-cancel-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/source", listener.local_addr().unwrap());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).unwrap();
            assert!(count > 0);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n")
                .unwrap();
            started_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(10));
        });
        let (services, _rx) = AppServices::new(root.clone()).unwrap();
        let request = Model::default().begin();
        services.subscription_command(
            request,
            Command::Preview(PreviewInput {
                name: "Fixture".into(),
                description: String::new(),
                sources: vec![PreviewSource::Url(url)],
            }),
        );
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let start = Instant::now();
        drop(services);
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "shutdown must cancel instead of waiting for 30s download budget"
        );
        release_tx.send(()).unwrap();
        server.join().unwrap();
        assert!(!root.join("state.json").exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
mod log_export_tests {
    use super::*;
    use crate::platform::{PlatformError, macos::DialogResult};
    // 保护原生面板结果进入真实Service/磁盘：取消无错误/无写入，失败保旧，可重试成功。
    #[test]
    fn logs_panel_cancel_success_failure_and_retry_write_actual_filtered_bytes() {
        let root = std::env::temp_dir().join(format!(
            "veyra-logs-export-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        let (services, _) = AppServices::new(root.clone()).unwrap();
        let safe = b"4\t12:00:00\tinfo\tdns query example.org\n[REDACTED]".to_vec();
        let target = root.join("filtered.log");
        assert_eq!(
            services
                .save_log_export(Ok(DialogResult::Cancelled), safe.clone())
                .blocking_recv()
                .unwrap(),
            Ok(false)
        );
        assert!(!target.exists());
        assert_eq!(
            services
                .save_log_export(Ok(DialogResult::Selected(target.clone())), safe.clone())
                .blocking_recv()
                .unwrap(),
            Ok(true)
        );
        assert_eq!(std::fs::read(&target).unwrap(), safe);
        assert_eq!(
            services
                .save_log_export(
                    Ok(DialogResult::Selected(
                        root.join("missing/sub/filtered.log")
                    )),
                    safe.clone()
                )
                .blocking_recv()
                .unwrap(),
            Err(PlatformError::FileWriteFailed)
        );
        assert_eq!(std::fs::read(&target).unwrap(), safe);
        assert_eq!(
            services
                .save_log_export(Err(PlatformError::FileDialogUnavailable), safe.clone())
                .blocking_recv()
                .unwrap(),
            Err(PlatformError::FileDialogUnavailable)
        );
        assert_eq!(
            services
                .save_log_export(Ok(DialogResult::Selected(target.clone())), safe.clone())
                .blocking_recv()
                .unwrap(),
            Ok(true)
        );
        assert_eq!(std::fs::read(target).unwrap(), safe);
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
}
