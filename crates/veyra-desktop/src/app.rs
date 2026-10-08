#[cfg(debug_assertions)]
use crate::state_bridge::InstanceToken;
use crate::{
    navigation::Route,
    services::AppServices,
    state_bridge::{AppEvent, StateBridge},
    ui::pages::Pages,
};
use gpui_kit::*;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
#[cfg(debug_assertions)]
use veyra_core::application::runtime_snapshot::InstanceId;

pub struct AppView {
    // 同一 AppView 持有主线程 TrayIcon；隐藏窗口不释放托盘或页面 Entity。
    tray: crate::tray::DesktopTray,
    dialog_open: bool,
    clipboard_snapshot: Option<crate::platform::macos::ClipboardSnapshot>,

    pub route: Route,
    pub dark: bool,
    pub pages: Pages,
    pub subscriptions: Entity<crate::ui::subscriptions::SubscriptionsView>,
    pub groups: Entity<crate::ui::groups::GroupsView>,
    _groups_subscription: Subscription,
    _subscriptions: Subscription,
    _shares_updates: Subscription,
    pub backend: Entity<crate::ui::backend::BackendView>,
    _runtime_subscription: Subscription,
    _profile_subscription: Subscription,
    pub bridge: StateBridge,
    pub services: Arc<AppServices>,
    pub behavior: crate::behavior_preferences::BehaviorCoordinator,
    pub behavior_panel: Entity<crate::ui::behavior_panel::BehaviorPanel>,
    _behavior_subscription: Subscription,
    pub visual: crate::preferences::SaveCoordinator,
    // 仅呈现状态；最终折叠偏好仍由原有 visual/CAS 保存路径负责。
    pub sidebar_transition: Option<crate::ui::shell::SidebarTransition>,
    pub panel: Entity<crate::ui::panel::PanelView>,
    pub page_scroll: ScrollHandle,
    pub notice_center: Entity<crate::ui::components::notice::NoticeCenter>,
    pub default_background: crate::ui::background::DefaultBackground,
    _bounds: Subscription,
    pub background_preview: Option<std::path::PathBuf>,
    preview_key: Option<(veyra_core::domain::VisualAssetId, u8)>,
    preview_generation: u64,
    save_timer: Option<Task<()>>,
    started: std::time::Instant,
    asset_generation: u64,
    _panel_subscription: Subscription,
    _appearance: Subscription,
    _receiver: Task<()>,
    _tray_quit: Subscription,
}
impl AppView {
    pub fn new(
        services: Arc<AppServices>,
        mut receiver: UnboundedReceiver<AppEvent>,
        tray: crate::tray::DesktopTray,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let notice_center = crate::ui::components::notice::NoticeCenter::mount(cx);
        let pages = Pages::new(window, cx);
        let backend = cx.new(|cx| crate::ui::backend::BackendView::new(window, cx));
        pages
            .get(Route::Settings)
            .update(cx, |p, _| p.backend = Some(backend.clone()));
        let runtime_subscription = cx.subscribe(
            &backend,
            |this, _, command: &veyra_core::application::manual_runtime::RuntimeCommand, cx| {
                this.runtime_command(*command, cx)
            },
        );
        let profile_subscription = cx.subscribe(
            &backend,
            |this, _, request: &crate::backend_profile::Request, _cx| {
                this.services.save_backend_profile(request.clone())
            },
        );
        let subscriptions =
            cx.new(|cx| crate::ui::subscriptions::SubscriptionsView::new(window, cx));
        subscriptions.update(cx, |view, _| view.attach_shares(services.clone()));
        pages.get(Route::Settings).update(cx, |page, _| {
            page.subscriptions = Some(subscriptions.clone())
        });
        let subscriptions_listener = cx.subscribe(
            &subscriptions,
            |this, _, event: &crate::ui::subscriptions::SubscriptionsEvent, _| {
                this.services
                    .subscription_command(event.request.clone(), event.command.clone());
            },
        );
        let shares_updates = cx.subscribe(
            &subscriptions,
            |this, _, event: &crate::ui::subscriptions::SharesUpdated, cx| {
                // 分享保存也发布权威快照，避免 Runtime 只读投影把旧配置版本送回页面。
                this.bridge.leave_page();
                this.behavior.rebase(&event.0);
                this.bridge.snapshot = Some(event.0.clone());
                cx.notify();
            },
        );
        let groups = cx.new(|cx| crate::ui::groups::GroupsView::new(window, cx));
        pages
            .get(Route::Settings)
            .update(cx, |page, _| page.groups = Some(groups.clone()));
        let groups_subscription = cx.subscribe(
            &groups,
            |this, _, event: &crate::ui::groups::GroupsEvent, _| {
                this.services.groups_command(event.clone())
            },
        );
        let panel = cx.new(|cx| crate::ui::panel::PanelView::new(window, cx));
        pages
            .get(Route::Settings)
            .update(cx, |p, _| p.panel = Some(panel.clone()));
        let behavior_panel = cx.new(|cx| crate::ui::behavior_panel::BehaviorPanel::new(window, cx));
        pages
            .get(Route::Settings)
            .update(cx, |p, _| p.behavior = Some(behavior_panel.clone()));
        panel.update(cx, |p, _| p.behavior = Some(behavior_panel.clone()));
        let behavior_subscription =
            cx.subscribe_in(&behavior_panel, window, |this, _, event, window, cx| {
                use crate::ui::behavior_panel::BehaviorPanelEvent;
                match event {
                    BehaviorPanelEvent::Edit(patch) => this.behavior.edit(patch.clone()),
                    BehaviorPanelEvent::Save => this.submit_behavior(window, cx),
                    BehaviorPanelEvent::Rebase => this.refresh(cx),
                    BehaviorPanelEvent::ExternalWrite => {
                        #[cfg(debug_assertions)]
                        this.services.evidence_external_preferences();
                    }
                }
                cx.notify();
            });
        let panel_subscription = cx.subscribe_in(&panel, window, |this, _, event, window, cx| {
            use crate::ui::panel::PanelEvent;
            match event {
                PanelEvent::Edit(value) => this.edit_visual(value.clone(), window, cx),
                PanelEvent::Import(path) => {
                    this.asset_generation += 1;
                    this.services
                        .import_background(this.asset_generation, path.clone());
                }
                PanelEvent::ChooseImage => {
                    this.open_platform_dialog(crate::platform::macos::DialogKind::Image, window, cx)
                }
                PanelEvent::ExportEvidence => this.open_platform_dialog(
                    crate::platform::macos::DialogKind::SaveEvidence,
                    window,
                    cx,
                ),
                PanelEvent::ClipboardWrite => {
                    let result = (|| {
                        if this.clipboard_snapshot.is_none() {
                            this.clipboard_snapshot =
                                Some(crate::platform::macos::ClipboardSnapshot::capture()?);
                        }
                        crate::platform::macos::write_text("veyra-p1-05-clipboard")?;
                        if crate::platform::macos::read_text()?.as_deref()
                            != Some("veyra-p1-05-clipboard")
                        {
                            return Err(crate::platform::PlatformError::ClipboardUnavailable);
                        }
                        Ok(())
                    })();
                    this.platform_notice(
                        result,
                        "Clipboard marker ready; use Cmd+V, then Restore",
                        window,
                        cx,
                    );
                }
                PanelEvent::ClipboardRestore => {
                    let result = this
                        .clipboard_snapshot
                        .as_mut()
                        .map_or(Ok(()), |snapshot| snapshot.restore());
                    if result.is_ok() {
                        this.clipboard_snapshot.take();
                    }
                    eprintln!("clipboard restoration result={result:?}; contents not logged");
                    this.platform_notice(result, "Clipboard restored", window, cx);
                }
                PanelEvent::ExternalLink => {
                    let result =
                        crate::platform::macos::ExternalUrl::new("http://127.0.0.1:9/veyra-p1-05")
                            .and_then(|u| u.open());
                    eprintln!(
                        "external URL OS handoff result={result:?}; target=loopback evidence"
                    );
                    this.platform_notice(result, "OS external link handoff", window, cx);
                }
                PanelEvent::Retry => {
                    this.retry_visual(window, cx);
                }
            }
        });
        let bounds = cx.observe_window_bounds(window, |this, window, _| {
            this.request_default_background(window);
        });
        let appearance = cx.observe_window_appearance(window, |this, window, cx| {
            this.apply_theme(window, cx);
            cx.notify();
        });
        // Weak UI reference and receiver live only in the GPUI foreground future.
        let foreground = cx.spawn_in(window, async move |view, cx| {
            while let Some(event) = receiver.recv().await {
                if view
                    .update_in(cx, |view, window, cx| {
                        view.receive_event(event, window, cx);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        #[cfg(debug_assertions)]
        cx.spawn_in(window, async |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1500))
                .await;
            let _ = view.update_in(cx, |view, _, _| view.tray.diagnose());
        })
        .detach();
        let tray_quit = cx.on_app_quit(|view, cx| {
            view.tray.prepare_quit();
            view.services.shares.shutdown();
            let done = view.services.manual_runtime.shutdown();
            cx.background_executor().spawn(async move {
                // 退出等待有界Runtime清理；失败是Unknown，不能记录成已停止。
                if !matches!(done.recv(), Ok(Ok(()))) {
                    eprintln!("runtime quit cleanup unresolved; recovery records retained");
                }
            })
        });
        eprintln!(
            "AppView created entity={:?}; pages_created_count=6",
            cx.entity_id()
        );
        let mut view = Self {
            backend,
            _runtime_subscription: runtime_subscription,
            _profile_subscription: profile_subscription,
            tray,
            dialog_open: false,
            clipboard_snapshot: None,
            route: Route::Overview,
            dark: false,
            pages,
            subscriptions,
            _subscriptions: subscriptions_listener,
            _shares_updates: shares_updates,
            groups,
            _groups_subscription: groups_subscription,
            bridge: StateBridge::default(),
            services,
            behavior: Default::default(),
            behavior_panel,
            _behavior_subscription: behavior_subscription,
            visual: Default::default(),
            sidebar_transition: None,
            panel,
            page_scroll: ScrollHandle::new(),
            notice_center,
            default_background: Default::default(),
            _bounds: bounds,
            background_preview: None,
            preview_key: None,
            preview_generation: 0,
            save_timer: None,
            started: std::time::Instant::now(),
            asset_generation: 0,
            _panel_subscription: panel_subscription,
            _appearance: appearance,
            _receiver: foreground,
            _tray_quit: tray_quit,
        };
        view.refresh(cx);
        view.runtime_command(
            veyra_core::application::manual_runtime::RuntimeCommand::Refresh,
            cx,
        );
        view
    }
    fn platform_notice(
        &self,
        result: Result<(), crate::platform::PlatformError>,
        success: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use crate::ui::components::{Notice, notify};
        match result {
            Ok(()) => notify(Notice::Success, success.to_owned(), window, cx),
            Err(error) => notify(Notice::Error, error.to_string(), window, cx),
        }
    }
    fn open_platform_dialog(
        &mut self,
        kind: crate::platform::macos::DialogKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.dialog_open {
            return;
        }
        self.dialog_open = true;
        let focus = window.focused(cx);
        let result = crate::platform::macos::begin_dialog(kind);
        eprintln!("native dialog opened kind={kind:?}");
        cx.spawn_in(window, async move |view, cx| {
            let result = result
                .await
                .unwrap_or(Err(crate::platform::PlatformError::FileDialogUnavailable));
            let _ = view.update_in(cx, |this, window, cx| {
                this.dialog_open = false;
                window.activate_window();
                if let Some(focus) = focus {
                    focus.focus(window, cx);
                }
                match result {
                    Ok(crate::platform::macos::DialogResult::Cancelled) => {
                        eprintln!("native dialog cancelled kind={kind:?}; no side effect")
                    }
                    Ok(crate::platform::macos::DialogResult::Selected(path)) => {
                        eprintln!("native dialog accepted kind={kind:?}; selected path redacted");
                        match kind {
                            crate::platform::macos::DialogKind::Image => {
                                this.asset_generation += 1;
                                this.services.import_background(this.asset_generation, path);
                            }
                            crate::platform::macos::DialogKind::SaveEvidence => {
                                this.services.save_platform_evidence(path)
                            }
                        }
                    }
                    Err(error) => this.platform_notice(Err(error), "", window, cx),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn now_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }
    pub fn apply_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        crate::ui::i18n::set(self.visual.draft.language, cx);
        self.tray.set_language(self.visual.draft.language);
        self.dark = crate::ui::theme::apply(
            &self.visual.draft,
            self.route == Route::Settings,
            window,
            cx,
        );
        self.behavior_panel.update(cx, |panel, cx| {
            if panel.sidebar_collapsed != self.visual.draft.sidebar_collapsed {
                panel.sidebar_collapsed = self.visual.draft.sidebar_collapsed;
                cx.notify();
            }
        });
        #[cfg(debug_assertions)]
        {
            use gpui_kit::component::ActiveTheme;
            eprintln!(
                "theme effective preference={:?} AppView.dark={} Kit={:?} route={:?} content={:?} snapshot_loaded={}",
                self.visual.draft.theme_mode,
                self.dark,
                cx.theme().mode,
                self.route,
                window.viewport_size(),
                self.bridge.snapshot.is_some()
            );
        }
    }
    fn request_default_background(&mut self, window: &Window) {
        // 等 Core 的真实偏好完成加载；受管背景仍由既有 preview 服务负责。
        if self.bridge.snapshot.is_none()
            || !matches!(
                self.visual.draft.background,
                veyra_core::domain::DesktopBackground::None
            )
        {
            return;
        }
        let key = crate::ui::background::Key::new(
            window.viewport_size(),
            self.visual.draft.background_blur,
        );
        if let Some(request) = self.default_background.request(key) {
            self.services.default_background(request);
        }
    }
    pub fn edit_visual(
        &mut self,
        value: veyra_core::domain::DesktopVisualPreferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let generation = self.visual.edit(value, self.now_ms());
        eprintln!(
            "visual change generation={generation} changes={}",
            self.visual.changes
        );
        self.panel
            .update(cx, |p, cx| p.project(&self.visual.draft, window, cx));
        self.apply_theme(window, cx);
        self.request_default_background(window);
        self.save_timer = Some(cx.spawn_in(window, async move |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(
                    crate::preferences::DEBOUNCE_MS,
                ))
                .await;
            let _ = view.update_in(cx, |view, window, cx| view.submit_visual(window, cx));
        }));
        cx.notify();
    }
    fn request_preview(&mut self) {
        use veyra_core::domain::DesktopBackground;
        if let DesktopBackground::ManagedAsset(id) = &self.visual.draft.background {
            let key = (id.clone(), self.visual.draft.background_blur);
            if self.preview_key.as_ref() != Some(&key) {
                self.preview_generation += 1;
                self.background_preview = None;
                self.preview_key = Some(key.clone());
                self.services
                    .background_preview(self.preview_generation, key.0, key.1);
            }
        } else {
            self.preview_generation += 1;
            self.preview_key = None;
            self.background_preview = None;
        }
    }
    fn submit_behavior(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(request) = self.behavior.submit() {
            eprintln!(
                "behavior submit generation={} expected={:?} fields={:?}",
                request.generation,
                request.expected,
                request.patch.changed_fields()
            );
            crate::ui::components::notify(
                crate::ui::components::Notice::Saving,
                "行为偏好正在保存",
                window,
                cx,
            );
            self.services.save_behavior(request);
        }
        self.project_behavior(window, cx);
    }
    fn project_behavior(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.behavior_panel.update(cx, |panel, cx| {
            panel.project(&self.behavior.draft, &self.behavior.status, window, cx)
        });
    }
    pub fn retry_visual(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.visual.retry(self.now_ms());
        self.submit_visual(window, cx);
    }
    fn submit_visual(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.request_preview();
        let Some(state) = &self.bridge.snapshot else {
            return;
        };
        if let Some(request) = self.visual.take_due(self.now_ms(), state.config_version()) {
            eprintln!(
                "visual submit generation={} commits={} opacity={} blur={}",
                request.generation,
                self.visual.submissions,
                request.draft.background_opacity,
                request.draft.background_blur
            );
            // 自动保存不弹进度通知，失败仍由完成事件反馈。
            self.services.save_visual(request);
        }
        cx.notify();
    }
    fn receive_event(&mut self, event: AppEvent, window: &mut Window, cx: &mut Context<Self>) {
        use crate::ui::components::{Notice, notify};
        match event {
            AppEvent::BackendProfile {
                request,
                result,
                snapshot,
            } => {
                // 保存完成先接纳同 epoch 且不倒退的快照；旧请求不能解除新字段的 busy。
                // 若原子保存后已有其它 writer 推进版本，取消排队草稿并要求重试；
                // 不能把外部写入自动当成下一条用户操作的 CAS 基线。
                let mut success = result.as_ref().is_ok_and(|saved| {
                    snapshot
                        .as_ref()
                        .is_ok_and(|state| state.config_version() == saved.version)
                });
                match snapshot {
                    Ok(state)
                        if self.bridge.snapshot.as_ref().is_some_and(|current| {
                            current.state_epoch == request.expected.0.epoch
                                && (state.state_epoch != current.state_epoch
                                    || state.config_revision >= current.config_revision)
                        }) =>
                    {
                        self.bridge.leave_page();
                        self.bridge.snapshot = Some(state);
                    }
                    _ => success = false,
                }
                if let Some(state) = &self.bridge.snapshot {
                    self.backend
                        .update(cx, |view, cx| view.project_profile(state, window, cx));
                }
                let accepted = self
                    .backend
                    .update(cx, |view, cx| view.complete(&request, success, cx));
                if accepted {
                    let message = if success {
                        "配置已保存，重启内核后生效"
                    } else if result.is_ok()
                        || result.as_ref().is_err_and(|e| {
                            e.code() == veyra_core::domain::AppErrorCode::RevisionConflict
                        })
                    {
                        "配置已被更新，请重新操作"
                    } else {
                        "配置保存失败，URL 草稿已保留，请重试"
                    };
                    notify(
                        if success {
                            Notice::Success
                        } else {
                            Notice::Error
                        },
                        message,
                        window,
                        cx,
                    );
                }
                self.project_runtime(cx);
            }
            AppEvent::Runtime(event) => {
                let result = event.result;
                let accepted = self.bridge.receive(AppEvent::Runtime(event))
                    == crate::state_bridge::Disposition::Accepted;
                if accepted && let Some(result) = result {
                    // 仅最终且当前请求的结果提示；轮询不产生通知。
                    if result
                        != Ok(veyra_core::application::manual_runtime::RuntimeResult::Refreshed)
                    {
                        notify(
                            if result.is_ok() {
                                Notice::Success
                            } else {
                                Notice::Error
                            },
                            crate::ui::backend::feedback(result),
                            window,
                            cx,
                        );
                    }
                }
                self.project_runtime(cx);
            }
            AppEvent::Groups {
                request,
                result,
                saved,
                snapshot,
            } => {
                let (result, snapshot) = crate::ui::groups::reconcile_completion(
                    self.bridge.snapshot.as_deref(),
                    result,
                    snapshot,
                );
                if let Some(state) = self.groups.update(cx, |view, cx| {
                    view.complete(request, result, saved, snapshot, window, cx)
                }) {
                    self.bridge.leave_page();
                    self.behavior.rebase(&state);
                    self.bridge.snapshot = Some(state);
                    self.project_behavior(window, cx);
                }
            }
            AppEvent::Subscriptions { request, result } => {
                let state = self
                    .subscriptions
                    .update(cx, |view, cx| view.complete(&request, result, window, cx));
                if let Some(state) = state {
                    self.bridge.leave_page();
                    self.behavior.rebase(&state);
                    self.bridge.snapshot = Some(state);
                    self.project_behavior(window, cx);
                }
            }
            AppEvent::DefaultBackground { request, bytes } => {
                let accepted = self.default_background.complete(request, bytes);
                eprintln!(
                    "default background generation={} accepted={accepted}",
                    request.generation
                );
                self.request_default_background(window);
            }
            AppEvent::PlatformSaved { result } => {
                eprintln!("platform export complete result={result:?}");
                self.platform_notice(result, "Platform evidence file saved", window, cx);
            }
            AppEvent::ExternalPreferenceWrite { result } => {
                eprintln!(
                    "external preferences writer {result:?}; UI version intentionally unchanged"
                );
                match result {
                    Ok(version) => notify(
                        Notice::Success,
                        format!(
                            "Evidence: 外部已写入 {:?}；UI 保持旧 version",
                            version.0.revision
                        ),
                        window,
                        cx,
                    ),
                    Err(error) => {
                        notify(Notice::Error, format!("外部写入失败 · {error}"), window, cx)
                    }
                }
            }
            AppEvent::BehaviorSaved {
                generation,
                mut result,
            } => {
                if !self.behavior.accepts_completion(generation) {
                    eprintln!("behavior completion {generation} discarded");
                    return;
                }
                if result.as_ref().is_ok_and(|saved| {
                    self.bridge.snapshot.as_ref().is_none_or(|current| {
                        saved.state_epoch != current.state_epoch
                            || saved.config_revision < current.config_revision
                    })
                }) {
                    result = Err(veyra_core::domain::AppError::new(
                        veyra_core::domain::AppErrorCode::RevisionConflict,
                    ));
                }
                let accepted = self.behavior.complete(
                    generation,
                    result.as_ref().map(|s| s.as_ref()).map_err(Clone::clone),
                );
                eprintln!(
                    "behavior completion generation={generation} accepted={accepted} status={:?} pending={:?}",
                    self.behavior.status,
                    self.behavior.pending.changed_fields()
                );
                match result {
                    Ok(saved) => {
                        self.bridge.leave_page();
                        self.bridge.snapshot = Some(saved);
                        if accepted {
                            notify(Notice::Success, "行为偏好已保存 · SavedOnly", window, cx);
                        }
                    }
                    Err(error) => notify(
                        Notice::Error,
                        format!("行为保存失败 · draft 保留 · {error}"),
                        window,
                        cx,
                    ),
                }
                self.project_behavior(window, cx);
            }
            AppEvent::BackgroundPreview { generation, result } => {
                if generation == self.preview_generation {
                    match result {
                        Ok(path) => self.background_preview = Some(path),
                        Err(error) => {
                            notify(Notice::Error, format!("背景预览失败 · {error}"), window, cx)
                        }
                    }
                }
            }
            AppEvent::VisualSaved { generation, result } => {
                if !self.visual.accepts_completion(generation) {
                    eprintln!(
                        "visual completion generation={generation} discarded: no matching in-flight request"
                    );
                    return;
                }
                // Version/epoch authority stays in Core. Do not adopt an old epoch completion.
                let same_epoch = result.as_ref().ok().is_none_or(|state| {
                    self.bridge.snapshot.as_ref().is_some_and(|current| {
                        current.state_epoch == state.state_epoch
                            && current.config_revision <= state.config_revision
                    })
                });
                if !same_epoch {
                    eprintln!("visual generation={generation} discard old epoch");
                    self.visual.complete(
                        generation,
                        Err(veyra_core::domain::AppError::new(
                            veyra_core::domain::AppErrorCode::RevisionConflict,
                        )),
                    );
                    return;
                }
                let completion = result.as_ref().map(|_| ()).map_err(Clone::clone);
                if let Ok(saved) = result {
                    self.bridge.leave_page();
                    self.behavior.rebase(&saved);
                    self.bridge.snapshot = Some(saved);
                    self.project_behavior(window, cx);
                }
                let accepted = self.visual.complete(generation, completion.clone());
                eprintln!(
                    "visual completion generation={generation} accepted={accepted} result={completion:?}"
                );
                // 视觉偏好自动保存成功保持静默；失败仍提示并保留草稿。
                if accepted && let Err(error) = completion {
                    notify(
                        Notice::Error,
                        format!("保存失败 · draft 保留 · {error}"),
                        window,
                        cx,
                    );
                }
                self.services
                    .collect_asset_orphans(self.visual.draft.background.clone());
                self.submit_visual(window, cx);
            }
            AppEvent::AssetPrepared { generation, result } => {
                if generation != self.asset_generation {
                    eprintln!(
                        "asset generation={generation} discarded; unreferenced asset retained in managed orphan set until cleanup"
                    );
                    return;
                }
                match result {
                    Ok(id) => {
                        eprintln!("asset prepared id={}", id.0);
                        let mut draft = self.visual.draft.clone();
                        draft.background = veyra_core::domain::DesktopBackground::ManagedAsset(id);
                        self.edit_visual(draft, window, cx);
                    }
                    Err(error) => {
                        notify(Notice::Error, format!("图片导入失败 · {error}"), window, cx)
                    }
                }
            }
            event => {
                self.bridge.receive(event);
                if let Some(state) = &self.bridge.snapshot {
                    self.visual.restore(&state.app_config.visual);
                    self.behavior.rebase(state);
                }
                self.project_behavior(window, cx);
                if self.visual.generation == 0 {
                    self.panel
                        .update(cx, |p, cx| p.project(&self.visual.draft, window, cx));
                }
                self.apply_theme(window, cx);
                self.request_preview();
                self.request_default_background(window);
            }
        }
        if let Some(state) = &self.bridge.snapshot {
            self.groups.update(cx, |view, cx| view.project(state, cx));
            self.subscriptions
                .update(cx, |view, cx| view.project(state, cx));
        }
        if let Some(state) = &self.bridge.snapshot {
            self.backend
                .update(cx, |view, cx| view.project_profile(state, window, cx));
        }
        // 当前 composition root 未绑定 P2 Runtime owner，不从偏好或配置推断运行状态。
        self.tray.project(self.bridge.snapshot.as_deref(), None);
    }
    fn project_runtime(&mut self, cx: &mut Context<Self>) {
        self.backend.update(cx, |view, cx| {
            view.snapshot = self.bridge.runtime.clone();
            view.operation = self.bridge.runtime_operation;
            if let (Some(runtime), Some(state)) = (&mut view.snapshot, &self.bridge.snapshot) {
                runtime.runtime.saved_version = state.config_version();
            }
            cx.notify();
        });
        cx.notify();
    }
    pub fn runtime_command(
        &mut self,
        command: veyra_core::application::manual_runtime::RuntimeCommand,
        cx: &mut Context<Self>,
    ) {
        let request = self.bridge.begin_runtime(command);
        self.services.manual_runtime.submit(request, command);
        self.project_runtime(cx);
    }
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let request = self.bridge.begin();
        self.services.refresh(request);
        cx.notify();
    }
    pub fn navigate(&mut self, route: Route, cx: &mut Context<Self>) {
        if route == self.route {
            return;
        }
        self.bridge.leave_page();
        let visible = route == Route::Settings
            && self.pages.get(Route::Settings).read(cx).category
                == crate::navigation::Category::Subscriptions;
        self.subscriptions
            .update(cx, |view, cx| view.set_visible(visible, cx));
        let backend_visible = route == Route::Settings
            && self.pages.get(Route::Settings).read(cx).category
                == crate::navigation::Category::Backend;
        self.backend
            .update(cx, |view, cx| view.set_visible(backend_visible, cx));
        let groups_visible = route == Route::Settings
            && self.pages.get(Route::Settings).read(cx).category
                == crate::navigation::Category::Groups;
        self.groups
            .update(cx, |view, cx| view.set_visible(groups_visible, cx));
        self.route = route;
        // The shell viewport is shared; a long Settings page must not leave
        // the next page's heading above the viewport.
        self.page_scroll.set_offset(point(px(0.), px(0.)));
        eprintln!(
            "navigate {} entity={:?}",
            route.id(),
            self.pages.get(route).entity_id()
        );
        // Initial load cancelled by navigation must not leave an uninitialized shell.
        if self.bridge.snapshot.is_none() {
            self.refresh(cx);
        }
        cx.notify();
    }
    #[cfg(debug_assertions)]
    pub fn evidence_race(&mut self, cx: &mut Context<Self>) {
        let a = self.bridge.begin();
        self.services.evidence_refresh(a, 500);
        let b = self.bridge.begin();
        self.services.evidence_refresh(b, 50);
        cx.notify();
    }
    #[cfg(debug_assertions)]
    pub fn evidence_busy(&mut self, cx: &mut Context<Self>) {
        let request = self.bridge.begin();
        self.services.evidence_refresh(request, 3000);
        cx.notify();
    }
    #[cfg(debug_assertions)]
    pub fn evidence_instances(&mut self, cx: &mut Context<Self>) {
        if let Some(state) = &self.bridge.snapshot {
            let epoch = state.state_epoch.clone();
            let current = InstanceToken {
                id: InstanceId("B".into()),
                generation: 2,
            };
            self.bridge.synthetic_instance = Some(current.clone());
            self.services.synthetic_event(AppEvent::SyntheticInstance {
                instance: InstanceToken {
                    id: InstanceId("A".into()),
                    generation: 1,
                },
                epoch: epoch.clone(),
            });
            self.services.synthetic_event(AppEvent::SyntheticInstance {
                instance: current,
                epoch,
            });
            cx.notify();
        }
    }
    /// Real GPUI entities, executed only on explicit debug evidence action.
    #[cfg(debug_assertions)]
    pub fn evidence_retention(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use crate::navigation::Category;
        eprintln!(
            "shell scroll evidence max={:?} bounds={:?}",
            self.page_scroll.max_offset(),
            self.page_scroll.bounds()
        );
        let route = self.route;
        let identities = Route::ALL.map(|r| self.pages.get(r).entity_id());
        let proxies = self.pages.get(Route::Proxies);
        let settings = self.pages.get(Route::Settings);
        let proxy_input = proxies.read(cx).input.clone();
        let settings_input = settings.read(cx).input.clone();
        let old_filter = proxy_input.read(cx).value();
        let old_draft = settings_input.read(cx).value();
        let old_category = settings.read(cx).category;
        proxy_input.update(cx, |input, cx| {
            input.set_value("automated-filter", window, cx)
        });
        settings_input.update(cx, |input, cx| {
            input.set_value("P1-03 automated draft", window, cx)
        });
        settings.update(cx, |p, _| p.category = Category::Chain);
        for next in Route::ALL {
            self.navigate(next, cx);
        }
        assert_eq!(
            identities,
            Route::ALL.map(|r| self.pages.get(r).entity_id())
        );
        assert_eq!(
            self.pages
                .get(Route::Proxies)
                .read(cx)
                .input
                .read(cx)
                .value(),
            "automated-filter"
        );
        assert_eq!(
            self.pages.get(Route::Settings).read(cx).category,
            Category::Chain
        );
        assert_eq!(
            self.pages
                .get(Route::Settings)
                .read(cx)
                .input
                .read(cx)
                .value(),
            "P1-03 automated draft"
        );
        eprintln!(
            "entity-evidence: 3 PASS: six identities, filter retained, category/draft retained"
        );
        proxy_input.update(cx, |input, cx| input.set_value(old_filter, window, cx));
        settings_input.update(cx, |input, cx| input.set_value(old_draft, window, cx));
        settings.update(cx, |p, _| p.category = old_category);
        self.navigate(route, cx);
    }
}
