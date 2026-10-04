use super::{components::button, theme::Palette};
use crate::{app::AppView, navigation::Route, preferences::SaveStatus, state_bridge::LoadState};
use gpui_kit::{
    component::{Icon, IconName, button::*, scroll::ScrollableElement},
    prelude::*,
    *,
};
use veyra_core::domain::DesktopLanguage;
fn route_label(route: Route, lang: DesktopLanguage) -> &'static str {
    match lang {
        DesktopLanguage::English => [
            "Overview",
            "Proxies",
            "Connections",
            "Logs",
            "Rules",
            "Settings",
        ][route as usize],
        DesktopLanguage::TraditionalChinese => {
            ["概覽", "代理", "連線", "日誌", "規則", "設定"][route as usize]
        }
        _ => route.label(),
    }
}
impl Render for AppView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let prefs = &self.visual.draft;
        let collapsed = prefs.sidebar_collapsed;
        let lang = prefs.language;
        let radius = prefs.global_radius as f32;
        let p = Palette::new(self.dark, self.route == Route::Settings);
        let mut root = div()
            .relative()
            .flex()
            .size_full()
            .font_family(".SystemUIFont")
            .text_size(px(14.))
            .line_height(px(20.))
            .text_color(rgb(p.text))
            .bg(rgb(p.window));
        if let veyra_core::domain::DesktopBackground::ManagedAsset(id) = &prefs.background {
            root = root.child(
                img(self
                    .background_preview
                    .clone()
                    .unwrap_or_else(|| self.services.assets.path(id)))
                .absolute()
                .size_full()
                .object_fit(ObjectFit::Cover)
                .opacity(prefs.background_opacity as f32 / 100.),
            );
        }
        let sidebar = div()
            .relative()
            .flex()
            .flex_col()
            .w(px(if collapsed { 64. } else { 256. }))
            .flex_shrink_0()
            .h_full()
            .p_2()
            .gap_2()
            .bg(rgba((p.sidebar << 8) | 0xd9))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(48.))
                    .when(!collapsed, |d| {
                        d.child(
                            div()
                                .pl_3()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("VEYRA"),
                        )
                    })
                    .child(
                        Button::new("collapse")
                            .child(
                                Icon::new(if collapsed {
                                    IconName::ChevronRight
                                } else {
                                    IconName::ChevronLeft
                                })
                                .size(px(20.)),
                            )
                            .accessibility_label("折叠/展开侧栏 · Collapse/Expand sidebar")
                            .w(px(40.))
                            .tooltip(if collapsed {
                                "展开侧栏 / Expand"
                            } else {
                                "折叠侧栏 / Collapse"
                            })
                            .on_click(cx.listener(|this, _, window, cx| {
                                let mut draft = this.visual.draft.clone();
                                draft.sidebar_collapsed = !draft.sidebar_collapsed;
                                this.edit_visual(draft, window, cx);
                            })),
                    ),
            )
            .children(Route::ALL.map(|route| {
                let icon = match route {
                    Route::Overview => IconName::LayoutDashboard,
                    Route::Proxies => IconName::Globe,
                    Route::Connections => IconName::Network,
                    Route::Logs => IconName::FileText,
                    Route::Rules => IconName::Map,
                    Route::Settings => IconName::Settings,
                };
                Button::new(route.id())
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .items_center()
                            .gap_3()
                            .when(collapsed, |d| d.justify_center())
                            .when(!collapsed, |d| d.justify_start())
                            .child(Icon::new(icon).size(px(22.)))
                            .when(!collapsed, |d| d.child(route_label(route, lang))),
                    )
                    .accessibility_label(route_label(route, lang))
                    .tooltip(route_label(route, lang))
                    .justify_start()
                    .px(px(if collapsed { 8. } else { 12. }))
                    .h(px(36.))
                    .w_full()
                    .ghost()
                    .when(self.route == route, |b| {
                        b.bg(rgb(p.active)).text_color(rgb(if self.dark {
                            p.text
                        } else {
                            0x368555
                        }))
                    })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.navigate(route, cx);
                        this.apply_theme(window, cx);
                    }))
            }))
            .child(div().flex_1())
            .when(!collapsed, |d| {
                d.child(
                    div()
                        .px_3()
                        .text_xs()
                        .text_color(rgb(p.muted))
                        .child("Runtime 尚未接入"),
                )
            })
            .child(
                button("quit", if collapsed { "×" } else { "退出 / Quit" })
                    .tooltip("退出 / Quit")
                    .on_click(|_, _, cx| cx.quit()),
            );
        let status = match &self.bridge.load {
            LoadState::Idle => "等待状态".into(),
            LoadState::Busy => "Loading · 本地状态".into(),
            LoadState::Ready => "本地状态".into(),
            LoadState::Error(e) => format!("读取失败 · {e}"),
        };
        let saved = self
            .bridge
            .snapshot
            .as_ref()
            .map(|s| {
                format!(
                    "schema {} · config {} · selection {}",
                    s.schema_version, s.config_revision, s.selection_revision
                )
            })
            .unwrap_or_default();
        let save_status = match &self.visual.status {
            SaveStatus::Idle => "SavedOnly · 已保存".to_string(),
            SaveStatus::Pending => "Draft · 等待合并保存".to_string(),
            SaveStatus::Saving => "Saving · 保存中".to_string(),
            SaveStatus::Failed(e) => format!("保存失败 · draft 保留 · {e}"),
        };
        let mut bridge_tools = div().flex().gap_2();
        #[cfg(debug_assertions)]
        {
            bridge_tools = bridge_tools
                .child(
                    button("evidence-retention", "Entity")
                        .bg(rgb(p.active))
                        .border_1()
                        .border_color(rgb(0x70c996))
                        .text_color(rgb(p.text))
                        .tooltip("Debug evidence · 页面实体与草稿保留")
                        .on_click(cx.listener(|this, _, w, cx| this.evidence_retention(w, cx))),
                )
                .child(
                    button("evidence-race", "A/B")
                        .bg(rgb(p.active))
                        .border_1()
                        .border_color(rgb(0x70c996))
                        .text_color(rgb(p.text))
                        .tooltip("Debug evidence · 迟到请求隔离")
                        .on_click(cx.listener(|this, _, _, cx| this.evidence_race(cx))),
                )
                .child(
                    button("evidence-busy", "Busy")
                        .bg(rgb(p.active))
                        .border_1()
                        .border_color(rgb(0x70c996))
                        .text_color(rgb(p.text))
                        .tooltip("Debug evidence · 非阻塞 Loading")
                        .on_click(cx.listener(|this, _, _, cx| this.evidence_busy(cx))),
                )
                .child(
                    button("evidence-instance", "Synthetic")
                        .bg(rgb(p.active))
                        .border_1()
                        .border_color(rgb(0x70c996))
                        .text_color(rgb(p.text))
                        .tooltip("Synthetic bridge evidence · 非真实 Runtime")
                        .on_click(cx.listener(|this, _, _, cx| this.evidence_instances(cx))),
                );
        }
        root.child(sidebar).child(
            div()
                .relative()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .h_full()
                .p_4()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .h(px(32.))
                        .bg(rgba((p.surface << 8) | 0xf2))
                        .rounded(px(9.))
                        .px_2()
                        .child(
                            div()
                                .text_lg()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(route_label(self.route, lang)),
                        )
                        .child(
                            button("refresh", "刷新 / Refresh")
                                .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_4()
                        .bg(rgba((p.surface << 8) | 0xf2))
                        .rounded(px(6.))
                        .px_2()
                        .text_xs()
                        .text_color(rgb(p.muted))
                        .child(status)
                        .child(saved)
                        .child(save_status),
                )
                .child(
                    div()
                        .id("page-content")
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h_0()
                        .rounded(px(radius))
                        .bg(rgba((p.surface << 8) | 0xe8))
                        .p_5()
                        .overflow_y_scroll()
                        .track_scroll(&self.page_scroll)
                        .vertical_scrollbar(&self.page_scroll)
                        .child(self.pages.get(self.route)),
                )
                .child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .min_h(px(64.))
                        .items_center()
                        .gap_4()
                        .justify_between()
                        .bg(rgb(p.surface))
                        .border_1()
                        .border_color(rgb(p.line))
                        .rounded(px(9.))
                        .px_4()
                        .py_2()
                        .text_xs()
                        .text_color(rgb(p.muted))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_color(rgb(p.text))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child("本地视觉偏好"),
                                )
                                .child("P1-05 接管正式目录 / 文件选择器"),
                        )
                        .child(bridge_tools)
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(format!("UI changes {}", self.visual.changes))
                                .child(format!("Save submissions {}", self.visual.submissions)),
                        ),
                ),
        )
    }
}
