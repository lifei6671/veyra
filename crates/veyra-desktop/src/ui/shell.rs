use super::{
    components::{button, evidence_visible, navigation_item},
    icons::icon,
    theme::Palette,
    tokens as t,
};
use crate::{app::AppView, navigation::Route, preferences::SaveStatus, state_bridge::LoadState};
use gpui_kit::{
    component::{Disableable, button::*, scroll::ScrollableElement},
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let prefs = &self.visual.draft;
        let collapsed = prefs.sidebar_collapsed;
        let lang = prefs.language;
        let radius = prefs.global_radius as f32;
        let p = Palette::new(self.dark, self.route == Route::Settings);
        let surface_alpha = if self.dark && self.route != Route::Settings {
            0xf0
        } else {
            0xbf
        };
        let mut root = div()
            .id("app-shell")
            // 仅 debug 的证据快捷键，正式布局不增加控件，也不代替 Tray 验收。
            .on_key_down(|event, window, cx| {
                if cfg!(debug_assertions)
                    && event.keystroke.modifiers.control
                    && event.keystroke.modifiers.alt
                {
                    match event.keystroke.key.as_str() {
                        "0" => {
                            window.resize(size(px(1280.), px(720.)));
                            cx.stop_propagation();
                        }
                        "q" => crate::desktop_lifecycle::dispatch(
                            crate::desktop_lifecycle::Intent::Quit,
                            None,
                            cx,
                        ),
                        _ => {}
                    }
                }
            })
            .relative()
            .flex()
            .size_full()
            .font_family(".SystemUIFont")
            .text_size(px(t::BODY))
            .line_height(px(t::BODY_LINE))
            .text_color(rgb(p.text))
            .bg(rgb(p.window));
        // React 默认背景是随包资源；无用户背景时仍显示相同来源，避免纯白替代。
        if matches!(
            prefs.background,
            veyra_core::domain::DesktopBackground::None
        ) {
            let size = window.viewport_size();
            if let Some(image) = self.default_background.ready() {
                root = root.child(
                    img(image)
                        .absolute()
                        .left(px(-10.))
                        .top(px(-10.))
                        .w(size.width + px(20.))
                        .h(size.height + px(20.))
                        .object_fit(ObjectFit::Cover)
                        .opacity(if self.route == Route::Settings {
                            1.
                        } else {
                            prefs.background_opacity as f32 / 100.
                        }),
                );
            }
        }
        if let veyra_core::domain::DesktopBackground::ManagedAsset(id) = &prefs.background {
            root = root.child(
                img(self
                    .background_preview
                    .clone()
                    .unwrap_or_else(|| self.services.assets.path(id)))
                .absolute()
                .size_full()
                .object_fit(ObjectFit::Cover)
                .opacity(if self.route == Route::Settings {
                    1.
                } else {
                    prefs.background_opacity as f32 / 100.
                }),
            );
        }
        if self.dark && self.route == Route::Settings {
            root = root.child(div().absolute().size_full().bg(rgba(0x000000a6)));
        }
        let sidebar = div()
            .relative()
            .flex()
            .flex_col()
            .w(px(if collapsed {
                t::SIDEBAR_COLLAPSED
            } else {
                t::SIDEBAR
            }))
            .flex_shrink_0()
            .h_full()
            .p(px(t::GAP))
            .bg(rgba((p.sidebar << 8) | 0xbf))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(t::NAV_HEIGHT))
                    .flex_shrink_0()
                    .mb(px(t::GAP))
                    .when(collapsed, |d| d.justify_center())
                    .when(!collapsed, |d| {
                        d.child(
                            div()
                                .w(px(t::BRAND_WIDTH))
                                .h(px(24.))
                                .text_size(px(18.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(if self.dark && self.route == Route::Settings {
                                    t::BRAND_PANEL_DARK_INK
                                } else {
                                    t::BRAND_INK
                                }))
                                .child("Veyra"),
                        )
                    })
                    .child(
                        gpui_kit::base::Button::new("collapse")
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(t::RADIUS))
                            .child(icon("SidebarToggle", t::ICON).text_color(if self.dark {
                                hsla(0., 0., 1., t::COLLAPSE_DARK_ALPHA)
                            } else {
                                rgb(t::COLLAPSE_INK).into()
                            }))
                            .h(px(t::NAV_HEIGHT))
                            .accessibility_label("折叠/展开侧栏 · Collapse/Expand sidebar")
                            .w(px(t::NAV_HEIGHT))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let mut draft = this.visual.draft.clone();
                                draft.sidebar_collapsed = !draft.sidebar_collapsed;
                                this.edit_visual(draft, window, cx);
                            }))
                            .map(|b| {
                                super::components::with_tooltip(
                                    b,
                                    "collapse-tooltip",
                                    if collapsed {
                                        "展开侧边栏"
                                    } else {
                                        "收起侧边栏"
                                    },
                                    true,
                                )
                                .sidebar_collapsed(collapsed)
                            }),
                    ),
            )
            .children(Route::ALL.map(|route| {
                let glyph = [
                    "Home",
                    "GlobeAlt",
                    "ArrowsRightLeft",
                    "DocumentText",
                    "Funnel",
                    "Cog6Tooth",
                ][route as usize];
                navigation_item(
                    route.id(),
                    route_label(route, lang),
                    glyph,
                    self.route == route,
                    super::components::NavigationKind::Sidebar {
                        collapsed,
                        panel: self.route == Route::Settings,
                    },
                    self.dark,
                    cx,
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.navigate(route, cx);
                    this.apply_theme(window, cx);
                }))
            }))
            .child(div().flex_1())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .when(!collapsed, |d| {
                        d.child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(t::GAP))
                                .p(px(t::PAD))
                                .rounded(px(16.))
                                .bg(rgba((p.surface << 8) | surface_alpha))
                                .children(
                                    [
                                        ["连接数", "内存使用"],
                                        ["进站流量", "进站速率"],
                                        ["出站流量", "出站速率"],
                                    ]
                                    .map(|labels| {
                                        div().flex().gap(px(16.)).children(labels.map(|label| {
                                            div()
                                                .flex_1()
                                                .flex()
                                                .flex_col()
                                                .child(
                                                    div()
                                                        .text_size(px(12.))
                                                        .line_height(px(16.))
                                                        .text_color(rgba(p.muted))
                                                        .child(label),
                                                )
                                                .child(
                                                    div()
                                                        .mt(px(1.))
                                                        .line_height(px(18.))
                                                        .font_weight(FontWeight::SEMIBOLD)
                                                        .child("—"),
                                                )
                                        }))
                                    }),
                                ),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .justify_between()
                            .gap(px(t::GAP))
                            .when(!collapsed, |d| {
                                d.mt(px(t::GAP)).child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .line_height(px(15.))
                                                .text_color(rgba(p.muted))
                                                .child("运行时长"),
                                        )
                                        .child(
                                            div().line_height(px(t::BODY_LINE)).child("当前不可用"),
                                        ),
                                )
                            })
                            .when(collapsed, |d| d.justify_center())
                            .child(
                                div()
                                    .flex()
                                    .gap(px(t::GAP))
                                    .when(collapsed, |d| d.flex_col())
                                    .children(
                                        [
                                            ("start", "启动内核", "Play"),
                                            ("stop", "停止内核", "Stop"),
                                            ("refresh-unavailable", "刷新数据", "ArrowPath"),
                                        ]
                                        .map(
                                            |(id, label, glyph)| {
                                                Button::new(id)
                                                    .ghost()
                                                    .disabled(true)
                                                    .accessibility_label(label)
                                                    .size(px(t::NAV_HEIGHT))
                                                    .rounded(px(t::NAV_RADIUS))
                                                    .bg(rgba((p.surface << 8) | surface_alpha))
                                                    .child(icon(glyph, t::ICON_SMALL))
                                            },
                                        ),
                                    ),
                            ),
                    ),
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
        root.child(sidebar)
            .child(
                div()
                    .relative()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .p_0()
                    .gap_0()
                    .when(evidence_visible(), |d| {
                        d.child(
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
                    })
                    .when(evidence_visible(), |d| {
                        d.child(
                            div()
                                .flex()
                                .gap_4()
                                .bg(rgba((p.surface << 8) | 0xf2))
                                .rounded(px(6.))
                                .px_2()
                                .text_xs()
                                .text_color(rgba(p.muted))
                                .child(status.clone())
                                .child(saved)
                                .child(save_status),
                        )
                    })
                    .when(!evidence_visible(), |d| {
                        d.when(matches!(self.visual.status, SaveStatus::Failed(_)), |d| {
                            d.child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(t::GAP))
                                    .p(px(t::GAP))
                                    .child("视觉偏好保存失败，草稿已保留")
                                    .child(button("retry-visual-save", "重试保存").on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.retry_visual(window, cx)
                                        }),
                                    )),
                            )
                        })
                        .when(
                            matches!(self.bridge.load, LoadState::Error(_)),
                            |d| {
                                d.child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(t::GAP))
                                        .p(px(t::GAP))
                                        .child(status.clone())
                                        .child(button("retry-state-load", "重新加载").on_click(
                                            cx.listener(|this, _, _, cx| this.refresh(cx)),
                                        )),
                                )
                            },
                        )
                    })
                    .child(
                        div()
                            .id("page-content")
                            .w_full()
                            .min_w_0()
                            .overflow_hidden()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_h_0()
                            .when(self.route != Route::Settings, |d| {
                                d.p(px(t::GAP)).rounded(px(radius))
                            })
                            .when(self.route != Route::Settings, |d| {
                                d.overflow_y_scroll()
                                    .track_scroll(&self.page_scroll)
                                    .vertical_scrollbar(&self.page_scroll)
                            })
                            .child(self.pages.get(self.route)),
                    )
                    .when(evidence_visible(), |d| {
                        d.child(
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
                                .text_color(rgba(p.muted))
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
                                        .child("GPUI preview · 平台目录 / 单实例"),
                                )
                                .child(bridge_tools)
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(format!("UI changes {}", self.visual.changes))
                                        .child(format!(
                                            "Save submissions {}",
                                            self.visual.submissions
                                        )),
                                ),
                        )
                    }),
            )
            .child(self.notice_center.clone())
    }
}
