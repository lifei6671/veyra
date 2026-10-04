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
    pub route: Route,
    pub dark: bool,
    pub pages: Pages,
    pub bridge: StateBridge,
    pub services: Arc<AppServices>,
    _receiver: Task<()>,
}
impl AppView {
    pub fn new(
        services: Arc<AppServices>,
        mut receiver: UnboundedReceiver<AppEvent>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let pages = Pages::new(window, cx);
        // Weak UI reference and receiver live only in the GPUI foreground future.
        let foreground = cx.spawn(async move |view, cx| {
            while let Some(event) = receiver.recv().await {
                if view
                    .update(cx, |view, cx| {
                        view.bridge.receive(event);
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut view = Self {
            route: Route::Overview,
            dark: false,
            pages,
            bridge: StateBridge::default(),
            services,
            _receiver: foreground,
        };
        view.refresh(cx);
        view
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
        self.route = route;
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
