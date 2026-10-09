//! Every main route owns one persistent PageView entity and its own input entities.
use crate::navigation::{Category, Route};
use gpui_kit::{
    component::{ActiveTheme, Disableable, button::*, input::InputState},
    prelude::*,
    *,
};

pub struct PageView {
    pub route: Route,
    pub input: Entity<InputState>,
    pub category: Category,
    pub alternate: bool,
    pub panel: Option<Entity<super::panel::PanelView>>,
    pub behavior: Option<Entity<super::behavior_panel::BehaviorPanel>>,
    pub subscriptions: Option<Entity<super::subscriptions::SubscriptionsView>>,
    pub shared_network: Option<Entity<super::shared_network::SharedNetworkView>>,
    pub groups: Option<Entity<super::groups::GroupsView>>,
    pub logs: Option<Entity<super::logs::LogsView>>,
    pub backend: Option<Entity<super::backend::BackendView>>,
    behavior_open: bool,
    settings_scroll: ScrollHandle,
    category_scroll: ScrollHandle,
    backend_scroll: ScrollHandle,
}
impl PageView {
    pub fn new(route: Route, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            route,
            category: Category::Panel,
            alternate: false,
            panel: None,
            behavior: None,
            subscriptions: None,
            groups: None,
            shared_network: None,
            backend: None,
            logs: None,
            behavior_open: false,
            settings_scroll: ScrollHandle::new(),
            category_scroll: ScrollHandle::new(),
            backend_scroll: ScrollHandle::new(),
            input: cx.new(|cx| {
                InputState::new(window, cx).placeholder(if route == Route::Settings {
                    "P1-03 evidence draft · 仅此会话，不写入 Profile"
                } else {
                    "会话筛选 · 业务数据尚未接入"
                })
            }),
        }
    }
}
impl Render for PageView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.route == Route::Logs
            && let Some(logs) = &self.logs
        {
            return div().size_full().child(logs.clone());
        }
        if self.route != Route::Settings && !super::components::evidence_visible() {
            return super::components::unavailable(self.route.task(), cx);
        }
        let mut content = div()
            .flex()
            .flex_col()
            .gap_0()
            .w_full()
            .min_w_0()
            .overflow_hidden()
            .when(self.route == Route::Settings, |d| {
                // cached Entity 已由 page-content 分配确定 bounds；错误横条会缩小父高度。
                d.size_full().min_h_0()
            });
        if self.route == Route::Settings {
            let dark = matches!(cx.theme().mode, gpui_kit::component::ThemeMode::Dark);
            content = content.child(
                div()
                    .flex()
                    .h(px(crate::ui::tokens::SETTINGS_NAV_HEIGHT))
                    .bg(rgb(super::theme::Palette::new(dark, true).surface)
                        .opacity(crate::ui::tokens::PANEL_TOOLBAR_ALPHA))
                    .flex_shrink_0()
                    .p(px(super::tokens::GAP))
                    .gap(px(super::tokens::GAP))
                    .child(
                        div()
                            .flex_1()
                            .id("settings-nav-scroll")
                            .track_scroll(&self.category_scroll)
                            .flex()
                            .min_w_0()
                            .gap(px(super::tokens::GAP))
                            .overflow_x_scroll()
                            .children(Category::ALL.into_iter().enumerate().map(
                                |(i, category)| {
                                    super::components::navigation_item(
                                        category.id(),
                                        crate::ui::i18n::tr(cx, category.label()),
                                        [
                                            "Home",
                                            "Rss",
                                            "RectangleStack",
                                            "Map",
                                            "DevicePhoneMobile",
                                            "Link",
                                            "Share",
                                            "ServerStack",
                                            "CpuChip",
                                        ][i],
                                        self.category == category,
                                        super::components::NavigationKind::Category,
                                        dark,
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| {
                                            if let Some(subscriptions) = &this.subscriptions {
                                                subscriptions.update(cx, |view, cx| {
                                                    view.set_visible(
                                                        category == Category::Subscriptions,
                                                        cx,
                                                    )
                                                });
                                            }
                                            if let Some(backend) = &this.backend {
                                                backend.update(cx, |view, cx| {
                                                    view.set_visible(
                                                        category == Category::Backend,
                                                        cx,
                                                    )
                                                });
                                            }
                                            if let Some(groups) = &this.groups {
                                                groups.update(cx, |view, cx| {
                                                    view.set_visible(
                                                        category == Category::Groups,
                                                        cx,
                                                    )
                                                });
                                            }
                                            if category == Category::Share
                                                && let Some(view) = &this.shared_network
                                            {
                                                view.update(cx, |v, cx| v.load(cx));
                                            }
                                            this.category = category;
                                            // 对应 React 分类栏：选中项必须完整滚入视口。
                                            this.category_scroll.scroll_to_item(i);
                                            cx.notify();
                                        },
                                    ))
                                },
                            )),
                    )
                    .when(self.category == Category::Share, |d| {
                        d.child(
                            super::components::icon_button(
                                "server-header-add",
                                crate::ui::i18n::tr(cx, "添加服务器"),
                                "Plus",
                                super::tokens::CONTROL,
                            )
                            .primary()
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    if let Some(view) = &this.shared_network {
                                        view.update(cx, |v, cx| v.open(None, window, cx));
                                    }
                                },
                            )),
                        )
                    })
                    .when(self.category == Category::Groups, |d| {
                        d.children(
                            [
                                ("恢复默认", "ArrowUturnLeft"),
                                ("自动分组", "Sparkles"),
                                ("添加分组", "Plus"),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(i, (label, glyph))| {
                                super::components::icon_button(
                                    label,
                                    crate::ui::i18n::tr(cx, label),
                                    glyph,
                                    super::tokens::CONTROL,
                                )
                                .with_tooltip(crate::ui::i18n::tr(cx, label))
                                .when(i == 2, |b| b.primary())
                                .disabled(self.groups.as_ref().is_some_and(|g| g.read(cx).busy))
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        if let Some(groups) = &this.groups {
                                            groups.update(cx, |view, cx| match i {
                                                0 => view.restore(window, cx),
                                                1 => view.automatic(window, cx),
                                                _ => view.open_add(window, cx),
                                            });
                                        }
                                    },
                                ))
                            }),
                        )
                    })
                    .when(self.category == Category::Subscriptions, |d| {
                        d.child(
                            super::components::icon_button(
                                "subscription-header-add",
                                crate::ui::i18n::tr(cx, "添加订阅或节点"),
                                "Plus",
                                super::tokens::CONTROL,
                            )
                            .primary()
                            .with_tooltip(crate::ui::i18n::tr(cx, "添加订阅或节点"))
                            .on_click(cx.listener(
                                |this, _, window, cx| {
                                    if let Some(view) = &this.subscriptions {
                                        view.update(cx, |view, cx| view.open_add(window, cx));
                                    }
                                },
                            )),
                        )
                    }),
            );
            if self.category == Category::Panel && !super::components::evidence_visible() {
                let mut panel_content = div()
                    .id("settings-content")
                    .track_scroll(&self.settings_scroll)
                    .min_w_0()
                    .w_full()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .gap(px(crate::ui::tokens::GAP))
                    .p(px(crate::ui::tokens::GAP));
                if let Some(panel) = &self.panel {
                    panel_content = panel_content.child(panel.clone());
                }
                if let Some(behavior) = &self.behavior {
                    panel_content = panel_content.child(behavior.clone());
                }
                content = content.child(panel_content);
            } else if self.category == Category::Panel {
                content = content.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(
                            Button::new("visual-settings")
                                .label("视觉偏好")
                                .when(!self.behavior_open, |b| b.primary())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.behavior_open = false;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("behavior-settings")
                                .label("行为偏好")
                                .when(self.behavior_open, |b| b.primary())
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.behavior_open = true;
                                    cx.notify();
                                })),
                        ),
                );
                if self.behavior_open {
                    if let Some(behavior) = &self.behavior {
                        content = content.child(behavior.clone());
                    }
                } else if let Some(panel) = &self.panel {
                    content = content.child(panel.clone());
                }
            } else if self.category == Category::Backend {
                if let Some(backend) = &self.backend {
                    content = content.child(
                        div()
                            .id("backend-scroll")
                            .track_scroll(&self.backend_scroll)
                            .w_full()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(backend.clone()),
                    );
                }
            } else if self.category == Category::Share {
                if let Some(view) = &self.shared_network {
                    content = content.child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .p(px(super::tokens::GAP))
                            .child(view.clone()),
                    );
                }
            } else if self.category == Category::Groups {
                if let Some(groups) = &self.groups {
                    content = content.child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .p(px(super::tokens::GAP))
                            .child(groups.clone()),
                    );
                }
            } else if self.category == Category::Subscriptions {
                if let Some(subscriptions) = &self.subscriptions {
                    content = content.child(
                        div()
                            .id("subscriptions-content")
                            .flex_1()
                            .min_h_0()
                            .p(px(super::tokens::GAP))
                            .child(subscriptions.clone()),
                    );
                }
            } else {
                if !super::components::evidence_visible() {
                    return content.child(super::components::unavailable(self.category.task(), cx));
                }
                content = content
                    .child(format!(
                        "业务能力尚未迁移 · 将在 {} 接入",
                        self.category.task()
                    ))
                    .child("P1-03 evidence draft / session only")
                    .child(super::components::text_input(&self.input));
            }
        } else {
            content = content
                .child(div().text_xl().child(format!(
                    "{} · {}",
                    self.route.label(),
                    self.route.id()
                )))
                .child(format!(
                    "业务能力尚未迁移 · 将在 {} 接入",
                    self.route.task()
                ));
            if self.route != Route::Overview {
                content = content
                    .child(super::components::text_input(&self.input))
                    .child(
                        Button::new("session-choice")
                            .label(match self.route {
                                Route::Logs => {
                                    if self.alternate {
                                        "级别筛选：Error（会话）"
                                    } else {
                                        "级别筛选：All（会话）"
                                    }
                                }
                                Route::Connections => {
                                    if self.alternate {
                                        "选中标记：开启（无连接数据）"
                                    } else {
                                        "选中标记：关闭"
                                    }
                                }
                                _ => {
                                    if self.alternate {
                                        "排序：名称降序（会话）"
                                    } else {
                                        "排序：名称升序（会话）"
                                    }
                                }
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.alternate = !this.alternate;
                                cx.notify();
                            })),
                    );
            } else {
                content = content
                    .child("本地桌面壳已就绪")
                    .child("当前仅加载本地 Core 状态。流量、内核、站点测速及统计尚未接入。")
                    .child("暂无业务配置时也可浏览全部页面。请勿将占位内容视为业务完成。");
            }
        }
        content
    }
}

// The ownership table is generic only to test route/state retention without
// GPUI's uncached test platform dependencies. Production T is always Entity<PageView>.
pub struct Pages<T = Entity<PageView>> {
    entities: [T; 6],
}
impl Pages {
    pub fn new(window: &mut Window, cx: &mut App) -> Self {
        Self {
            entities: Route::ALL.map(|route| cx.new(|cx| PageView::new(route, window, cx))),
        }
    }
}
impl<T: Clone> Pages<T> {
    pub fn get(&self, route: Route) -> T {
        self.entities[route as usize].clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{Category, Pages, Route};
    use std::{cell::RefCell, rc::Rc};

    #[derive(Default)]
    struct Session {
        input: String,
        category: Category,
    }
    fn pages() -> Pages<Rc<RefCell<Session>>> {
        Pages {
            entities: Route::ALL.map(|_| Rc::new(RefCell::new(Session::default()))),
        }
    }
    // Protect the production ownership/route lookup: selecting another route must
    // neither replace the original session nor alias two independent pages.
    #[test]
    fn page_identity_and_filter_survive_round_trip() {
        let pages = pages();
        let proxies = pages.get(Route::Proxies);
        proxies.borrow_mut().input = "filter".into();
        let overview = pages.get(Route::Overview);
        assert!(!Rc::ptr_eq(&proxies, &overview));
        assert!(overview.borrow().input.is_empty());
        let restored = pages.get(Route::Proxies);
        assert!(Rc::ptr_eq(&proxies, &restored));
        assert_eq!(restored.borrow().input, "filter");
        for (i, route) in Route::ALL.into_iter().enumerate() {
            for other in &Route::ALL[i + 1..] {
                assert!(!Rc::ptr_eq(&pages.get(route), &pages.get(*other)));
            }
        }
    }
    #[test]
    fn settings_category_and_draft_survive_all_routes() {
        let pages = pages();
        let settings = pages.get(Route::Settings);
        settings.borrow_mut().input = "session draft".into();
        settings.borrow_mut().category = Category::Chain;
        for route in Route::ALL {
            let _ = pages.get(route);
        }
        let restored = pages.get(Route::Settings);
        assert!(Rc::ptr_eq(&settings, &restored));
        assert_eq!(restored.borrow().category, Category::Chain);
        assert_eq!(restored.borrow().input, "session draft");
    }
}
