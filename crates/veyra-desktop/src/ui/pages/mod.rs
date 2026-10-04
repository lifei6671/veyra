//! Every main route owns one persistent PageView entity and its own input entities.
use crate::navigation::{Category, Route};
use gpui_kit::{
    component::{button::*, input::InputState},
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
    behavior_open: bool,
}
impl PageView {
    pub fn new(route: Route, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            route,
            category: Category::Panel,
            alternate: false,
            panel: None,
            behavior: None,
            behavior_open: false,
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
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut content = div().flex().flex_col().flex_shrink_0().gap_3().w_full();
        if self.route == Route::Settings {
            if self.category == Category::Panel {
                content = content.min_h(px(680.));
            }
            content =
                content
                    .child(div().flex().flex_wrap().gap_2().children(Category::ALL.map(
                        |category| {
                            Button::new(category.id())
                                .label(category.label())
                                .when(self.category == category, |b| b.primary())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.category = category;
                                    eprintln!("category {}", category.id());
                                    cx.notify();
                                }))
                        },
                    )))
                    .child(div().text_xl().child(self.category.label()));
            if self.category == Category::Panel {
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
            } else {
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
