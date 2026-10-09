use super::{
    components::{button, evidence_visible, navigation_item},
    icons::icon,
    theme::Palette,
    tokens as t,
};
use crate::ui::i18n::tr;
use crate::{app::AppView, navigation::Route, preferences::SaveStatus, state_bridge::LoadState};
use gpui_kit::{
    component::{Disableable, Sizable, button::*, scroll::ScrollableElement},
    prelude::*,
    *,
};
use std::time::{Duration, Instant};
use veyra_core::application::manual_runtime::RuntimeCommand;
use veyra_core::domain::DesktopLanguage;

fn sidebar_width(collapsed: bool) -> f32 {
    if collapsed {
        t::SIDEBAR_COLLAPSED
    } else {
        t::SIDEBAR
    }
}

pub struct SidebarTransition {
    from: f32,
    to: f32,
    started: Instant,
}

impl SidebarTransition {
    fn new(from: f32, collapsed: bool, started: Instant) -> Self {
        Self {
            from,
            to: sidebar_width(collapsed),
            started,
        }
    }

    fn finished(&self, now: Instant) -> bool {
        now.duration_since(self.started) >= Duration::from_millis(t::SIDEBAR_TRANSITION_MS)
    }

    fn width(&self, now: Instant) -> f32 {
        let progress = (now.duration_since(self.started).as_secs_f32()
            / Duration::from_millis(t::SIDEBAR_TRANSITION_MS).as_secs_f32())
        .min(1.);
        let [x1, y1, x2, y2] = t::SIDEBAR_EASE;
        let eased = gpui_kit::component::animation::cubic_bezier(x1, y1, x2, y2)(progress);
        self.from + (self.to - self.from) * eased
    }
}

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
        let now = Instant::now();
        if self
            .sidebar_transition
            .as_ref()
            .is_some_and(|a| a.finished(now))
            || cx.reduce_motion()
        {
            self.sidebar_transition = None;
        }
        let sidebar_visual_width = self.sidebar_transition.as_ref().map_or_else(
            || sidebar_width(self.visual.draft.sidebar_collapsed),
            |animation| {
                // GPUI 帧回调驱动；没有后台任务，完成后停止请求帧。
                window.request_animation_frame();
                animation.width(now)
            },
        );
        let prefs = &self.visual.draft;
        let collapsed = prefs.sidebar_collapsed;
        let lang = prefs.language;
        let radius = prefs.global_radius as f32;
        let p = Palette::new(self.dark, self.route == Route::Settings);
        // 本机OpenBox无自定义背景时：base100 + base200/50，而非内置图片。
        // Settings共用此背景，订阅页不能拥有另一套浅深色。
        let panel_plain = self.route == Route::Settings
            && matches!(
                prefs.background,
                veyra_core::domain::DesktopBackground::None
            );
        let surface_alpha = if panel_plain {
            0xff
        } else if self.dark && self.route != Route::Settings {
            0xf0
        } else {
            0xbf
        };
        let mut root = div()
            .id("app-shell")
            // 仅 debug 的证据快捷键，正式布局不增加控件，也不代替 Tray 验收。
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if cfg!(debug_assertions)
                    && event.keystroke.modifiers.control
                    && event.keystroke.modifiers.alt
                {
                    match event.keystroke.key.as_str() {
                        "0" => {
                            window.resize(size(px(1280.), px(720.)));
                            cx.stop_propagation();
                        }
                        "s" => this.runtime_command(
                            veyra_core::application::manual_runtime::RuntimeCommand::Start,
                            cx,
                        ),
                        "t" => this.runtime_command(
                            veyra_core::application::manual_runtime::RuntimeCommand::Stop,
                            cx,
                        ),
                        "r" => this.runtime_command(
                            veyra_core::application::manual_runtime::RuntimeCommand::Restart,
                            cx,
                        ),
                        "q" => crate::desktop_lifecycle::dispatch(
                            crate::desktop_lifecycle::Intent::Quit,
                            None,
                            cx,
                        ),
                        _ => {}
                    }
                }
            }))
            .relative()
            .flex()
            .size_full()
            .font_family("MiSans")
            .font_weight(super::theme::MISANS_REGULAR)
            .text_size(px(t::BODY))
            .line_height(px(t::BODY_LINE))
            .text_color(rgb(p.text))
            .bg(rgb(p.window));
        // React 默认背景是随包资源；无用户背景时仍显示相同来源，避免纯白替代。
        if panel_plain {
            root = root.child(
                div()
                    .absolute()
                    .size_full()
                    .bg(rgb(p.sidebar).opacity(t::PANEL_BACKGROUND_TINT)),
            );
        }
        if !panel_plain
            && matches!(
                prefs.background,
                veyra_core::domain::DesktopBackground::None
            )
        {
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
        if self.dark && self.route == Route::Settings && !panel_plain {
            root = root.child(div().absolute().size_full().bg(rgba(0x000000a6)));
        }
        let sidebar = div()
            .relative()
            .flex()
            .flex_col()
            .w(px(sidebar_visual_width))
            .flex_shrink_0()
            .h_full()
            .p(px(t::GAP))
            .bg(rgba(
                (p.sidebar << 8) | if panel_plain { 0xff } else { 0xbf },
            ))
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
                                .font_weight(super::theme::MISANS_SEMIBOLD)
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
                            // 用户要求收起/展开按钮在 hover 时显示共享 IconButton 背景。
                            .hover(|b| b.bg(rgba(t::HOVER)))
                            .child(icon("SidebarToggle", t::ICON).text_color(if self.dark {
                                hsla(0., 0., 1., t::COLLAPSE_DARK_ALPHA)
                            } else {
                                rgb(t::COLLAPSE_INK).into()
                            }))
                            .h(px(t::NAV_HEIGHT))
                            .accessibility_label(tr(cx, "折叠/展开侧栏 · Collapse/Expand sidebar"))
                            .w(px(t::NAV_HEIGHT))
                            .on_click(cx.listener(|this, _, window, cx| {
                                let mut draft = this.visual.draft.clone();
                                let now = Instant::now();
                                // 反向切换从本帧宽度继续，不跳回上一次的固定端点。
                                let from =
                                    this.sidebar_transition.as_ref().map_or(
                                        sidebar_width(draft.sidebar_collapsed),
                                        |animation| animation.width(now),
                                    );
                                draft.sidebar_collapsed = !draft.sidebar_collapsed;
                                this.sidebar_transition = Some(SidebarTransition::new(
                                    from,
                                    draft.sidebar_collapsed,
                                    now,
                                ));
                                this.edit_visual(draft, window, cx);
                            }))
                            .map(|b| {
                                super::components::with_tooltip(
                                    b,
                                    "collapse-tooltip",
                                    if collapsed {
                                        tr(cx, "展开侧边栏")
                                    } else {
                                        tr(cx, "收起侧边栏")
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
                                .bg(rgba((p.surface << 8) | if panel_plain {0xbf} else {surface_alpha}))
                                .children(
                                    [
                                        [tr(cx, "连接数"), tr(cx, "内存使用")],
                                        [tr(cx, "进站流量"), tr(cx, "进站速率")],
                                        [tr(cx, "出站流量"), tr(cx, "出站速率")],
                                    ]
                                    .map(|labels| {
                                        div().flex().gap(px(t::GAP)).children(labels.map(|label| {
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
                                                        .text_size(px(14.))
                                                        .line_height(px(20.))
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
                                                .line_height(px(16.))
                                                .text_color(rgba(p.muted))
                                                .child(tr(cx, "运行时长")),
                                        )
                                        .child(
                                            div()
                                                .line_height(px(t::BODY_LINE))
                                                .text_size(px(14.)).font_weight(super::theme::MISANS_MEDIUM)
                                                .child(self.bridge.runtime.as_ref().and_then(|s| s.uptime_seconds).map(|seconds| format!("{:02}:{:02}:{:02}", seconds/3600, seconds/60%60, seconds%60)).unwrap_or_else(|| tr(cx, super::backend::status(self.bridge.runtime.as_ref())).to_owned())),
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
                                            ("start", tr(cx, "启动内核"), "Play", RuntimeCommand::Start),
                                            ("stop", tr(cx, "停止内核"), "Stop", RuntimeCommand::Stop),
                                            (
                                                "refresh-unavailable",
                                                tr(cx, "刷新数据"),
                                                "ArrowPath", RuntimeCommand::Refresh,
                                            ),
                                        ]
                                        .map(
                                            |(id, label, glyph, command)| {
                                                Button::new(id)
                                                    .ghost()
                                                    .disabled(self.bridge.runtime_operation.is_some() || self.bridge.runtime.is_none() || (id == "start" && self.bridge.runtime.as_ref().is_some_and(|s| s.runtime.status == veyra_core::application::runtime_snapshot::RuntimeStatus::Ready)) || (id == "stop" && self.bridge.runtime.as_ref().is_some_and(|s| s.runtime.status == veyra_core::application::runtime_snapshot::RuntimeStatus::Stopped)))
                                                    .on_click(cx.listener(move |this, _, _, cx| { this.runtime_command(command, cx); }))
                                                    .accessibility_label(label)
                                                    .size(px(t::NAV_HEIGHT))
                                                    .rounded(px(t::NAV_RADIUS))
                                                    .bg(rgba((p.surface << 8) | if panel_plain {0xbf} else {surface_alpha}))
                                                    .when(!super::backend::action_is_busy(self.bridge.runtime_operation, command), |b| b.child(icon(glyph, t::ICON_SMALL)))
                                                    .when(super::backend::action_is_busy(self.bridge.runtime_operation, command), |b| b.child(gpui_kit::component::spinner::Spinner::new().with_size(px(t::ICON_SMALL))))
                                            },
                                        ),
                                    ),
                            ),
                    ),
            );
        let status = match &self.bridge.load {
            LoadState::Idle => tr(cx, "等待状态").into(),
            LoadState::Busy => tr(cx, "Loading · 本地状态").into(),
            LoadState::Ready => tr(cx, "本地状态").into(),
            LoadState::Error(e) => format!(
                "{} · {}",
                tr(cx, "读取失败"),
                crate::ui::i18n::message(cx, &e.to_string())
            ),
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
                                        .font_weight(super::theme::MISANS_SEMIBOLD)
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
                                    .child(tr(cx, "视觉偏好保存失败，草稿已保留"))
                                    .child(
                                        button("retry-visual-save", tr(cx, "重试保存")).on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.retry_visual(window, cx)
                                            }),
                                        ),
                                    ),
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
                                        .child(
                                            button("retry-state-load", tr(cx, "重新加载"))
                                                .on_click(
                                                    cx.listener(|this, _, _, cx| this.refresh(cx)),
                                                ),
                                        ),
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
                            .when(
                                self.route != Route::Settings && self.route != Route::Logs,
                                |d| d.p(px(t::GAP)).rounded(px(radius)),
                            )
                            .when(
                                self.route != Route::Settings && self.route != Route::Logs,
                                |d| {
                                    d.overflow_y_scroll()
                                        .track_scroll(&self.page_scroll)
                                        .vertical_scrollbar(&self.page_scroll)
                                },
                            )
                            // 页面已拥有固定视口：复用 GPUI 缓存边界，避免动画每帧
                            // 在父布局测量时展开整棵控件树；尺寸/实体变化仍会重绘。
                            .child(if window.is_a11y_active() {
                                // pinned GPUI 的 cached reuse 不重建 AX 子树；无障碍激活时
                                // 用同一页面 Entity 正常绘制，普通路径仍保留缓存优化。
                                self.pages.get(self.route).into_any_element()
                            } else {
                                self.pages
                                    .get(self.route)
                                    .cached(StyleRefinement::default().size_full())
                                    .into_any_element()
                            }),
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
                                                .font_weight(super::theme::MISANS_SEMIBOLD)
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

#[cfg(test)]
mod sidebar_transition_tests {
    use super::{SidebarTransition, t};
    use std::time::{Duration, Instant};

    #[test]
    fn sidebar_moves_between_exact_endpoints_and_finishes() {
        let start = Instant::now();
        let duration = Duration::from_millis(t::SIDEBAR_TRANSITION_MS);
        for (from, collapsed, to) in [
            (t::SIDEBAR, true, t::SIDEBAR_COLLAPSED),
            (t::SIDEBAR_COLLAPSED, false, t::SIDEBAR),
        ] {
            let animation = SidebarTransition::new(from, collapsed, start);
            assert_eq!(animation.width(start), from);
            let midway = animation.width(start + duration / 2);
            assert!(midway > t::SIDEBAR_COLLAPSED && midway < t::SIDEBAR);
            assert!(!animation.finished(start + duration / 2));
            assert_eq!(animation.width(start + duration), to);
            assert!(animation.finished(start + duration));
        }
    }

    #[test]
    fn reversing_sidebar_keeps_current_width_without_jumping() {
        let start = Instant::now();
        let collapsing = SidebarTransition::new(t::SIDEBAR, true, start);
        let reverse_at = start + Duration::from_millis(80);
        let current = collapsing.width(reverse_at);
        let expanding = SidebarTransition::new(current, false, reverse_at);
        assert_eq!(expanding.width(reverse_at), current);
        assert!(expanding.width(reverse_at + Duration::from_millis(40)) > current);
        assert_eq!(
            expanding.width(reverse_at + Duration::from_millis(t::SIDEBAR_TRANSITION_MS)),
            t::SIDEBAR
        );
    }
}

#[cfg(test)]
mod runtime_action_tests {
    use super::RuntimeCommand;
    #[test]
    fn sidebar_spinner_belongs_only_to_submitted_command() {
        let commands = [
            RuntimeCommand::Start,
            RuntimeCommand::Stop,
            RuntimeCommand::Refresh,
        ];
        for (index, active) in commands.into_iter().enumerate() {
            let spinning = commands
                .map(|command| super::super::backend::action_is_busy(Some(active), command));
            assert_eq!(spinning.iter().filter(|v| **v).count(), 1);
            assert!(spinning[index]);
        }
        // sidebar 没有 Restart 按钮，不将其伪装成 Refresh 正在执行。
        assert!(
            commands
                .into_iter()
                .all(|c| !super::super::backend::action_is_busy(Some(RuntimeCommand::Restart), c))
        );
        assert!(
            commands
                .into_iter()
                .all(|c| !super::super::backend::action_is_busy(None, c))
        );
    }
}
