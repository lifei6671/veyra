use super::{
    components::{button, evidence_visible, navigation_item},
    icons::icon,
    theme::Palette,
    tokens as t,
};
use crate::ui::i18n::tr;
use crate::{app::AppView, navigation::Route, preferences::SaveStatus, state_bridge::LoadState};
use gpui_kit::{
    component::{Disableable, Sizable, button::*},
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

// AppShell formatDuration：不足分钟显示秒，其余省略秒；全局语言仍由 i18n 决定。
fn sidebar_uptime(seconds: u64, lang: DesktopLanguage) -> String {
    use super::i18n::translate;
    if seconds < 60 {
        return format!("{seconds}{}", translate(lang, "秒"));
    }
    let hours = seconds / 3600;
    let minutes = seconds / 60 % 60;
    if hours == 0 {
        return format!("{minutes}{}", translate(lang, "分钟"));
    }
    let mut value = format!("{hours}{}", translate(lang, "小时"));
    if minutes != 0 {
        value.push_str(&format!("{minutes}{}", translate(lang, "分钟")));
    }
    value
}

// AppShell 使用十进制流量单位、二进制内存单位；未知保持未知，不能伪造 0。
fn sidebar_bytes(value: u64, decimals: usize) -> String {
    let units = ["B", "kB", "MB", "GB", "TB"];
    let mut amount = value as f64;
    let mut index = 0;
    while amount >= 1000. && index < units.len() - 1 {
        amount /= 1000.;
        index += 1;
    }
    let amount = format!("{:.*}", if index == 0 { 0 } else { decimals }, amount);
    let amount = amount.strip_suffix(".0").unwrap_or(&amount);
    format!("{amount} {}", units[index])
}
fn sidebar_stats(
    observation: Option<&veyra_core::application::observability::controller::Snapshot>,
    instance: Option<&veyra_core::application::runtime_snapshot::InstanceId>,
) -> [String; 6] {
    let snapshot = observation.filter(|s| {
        s.identity
            .as_ref()
            .is_some_and(|identity| instance == Some(&identity.instance_id))
    });
    let unknown = || "—".to_owned();
    let count = snapshot
        .filter(|s| s.available[2])
        .and_then(|s| s.connection_count)
        .map(|n| n.to_string())
        .unwrap_or_else(unknown);
    let memory = snapshot
        .filter(|s| s.available[1])
        .and_then(|s| s.memory_bytes)
        .map(|n| {
            if n == 0 {
                return "0 B".to_owned();
            }
            let value = format!("{:.1}", n as f64 / 1024. / 1024.);
            format!("{} MiB", value.strip_suffix(".0").unwrap_or(&value))
        })
        .unwrap_or_else(unknown);
    // 正式 Observation 区分本次累计与区间速率；不使用 legacy hook 的帧差推测。
    let traffic = snapshot.and_then(|s| s.traffic.as_ref());
    let rate = traffic.filter(|t| t.interval_ms > 0 && snapshot.is_some_and(|s| s.available[0]));
    [
        count,
        memory,
        traffic
            .map(|t| sidebar_bytes(t.session_download_bytes, 2))
            .unwrap_or_else(unknown),
        rate.map(|t| format!("{}/s", sidebar_bytes(t.download_bytes_per_second, 1)))
            .unwrap_or_else(unknown),
        traffic
            .map(|t| sidebar_bytes(t.session_upload_bytes, 2))
            .unwrap_or_else(unknown),
        rate.map(|t| format!("{}/s", sidebar_bytes(t.upload_bytes_per_second, 1)))
            .unwrap_or_else(unknown),
    ]
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
        let p = Palette::new(self.dark, true);
        // 背景事实只来自面板视觉偏好；切换主导航不再改变背景或透明度。
        let panel_plain = matches!(
            prefs.background,
            veyra_core::domain::DesktopBackground::None
        );
        let surface_alpha = if panel_plain {
            0xff
        } else if self.dark {
            0xf0
        } else {
            0xbf
        };
        let stats = sidebar_stats(
            self.sidebar_observation.as_ref(),
            self.bridge
                .runtime
                .as_ref()
                .and_then(|r| r.runtime.instance_id.as_ref()),
        );
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
            .font(t::openbox_font())
            .font_weight(crate::ui::tokens::FONT_REGULAR)
            .text_size(px(t::BODY))
            .line_height(px(t::BODY_LINE))
            .text_color(rgb(p.text))
            .bg(rgb(p.window));
        // None 与设置页一致：base100 + base200/50；不会偷偷显示另一张默认图片。
        if panel_plain {
            root = root.child(
                div()
                    .absolute()
                    .size_full()
                    .bg(rgb(p.sidebar).opacity(t::PANEL_BACKGROUND_TINT)),
            );
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
                .opacity(prefs.background_opacity as f32 / 100.),
            );
        }
        if self.dark && !panel_plain {
            root = root.child(div().absolute().size_full().bg(rgba(0x000000a6)));
        }
        let sidebar = div()
            .font(t::openbox_font())
            .font_weight(t::proxies::FONT_REGULAR)
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
                                .font_weight(t::proxies::FONT_SEMIBOLD)
                                .text_color(rgb(if self.dark {
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
                            .accessibility_label(tr(cx, if self.visual.draft.sidebar_collapsed { "展开侧边栏" } else { "收起侧边栏" }))
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
                                        [(tr(cx, "连接数"), stats[0].clone()), (tr(cx, "内存使用"), stats[1].clone())],
                                        [(tr(cx, "进站流量"), stats[2].clone()), (tr(cx, "进站速率"), stats[3].clone())],
                                        [(tr(cx, "出站流量"), stats[4].clone()), (tr(cx, "出站速率"), stats[5].clone())],
                                    ]
                                    .map(|labels| {
                                        div().flex().gap(px(t::SIDEBAR_STAT_COLUMN_GAP)).children(labels.map(|(label, value)| {
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
                                                        .line_height(px(t::SIDEBAR_STAT_VALUE_LINE))
                                                        .mt(px(t::SIDEBAR_STAT_VALUE_TOP))
                                                        .font_weight(t::proxies::FONT_SEMIBOLD)
                                                        .child(value),
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
                                        .gap(px(t::SIDEBAR_UPTIME_GAP))
                                        .child(
                                            div()
                                                .text_size(px(12.))
                                                .line_height(px(t::SIDEBAR_UPTIME_LABEL_LINE))
                                                .text_color(rgba(p.muted))
                                                .child(tr(cx, "运行时长")),
                                        )
                                        .child(
                                            div()
                                                .line_height(px(t::BODY_LINE))
                                                .text_size(px(14.)).font_weight(t::SIDEBAR_UPTIME_WEIGHT)
                                                .child(self.bridge.runtime.as_ref().and_then(|s| s.uptime_seconds).map(|seconds| sidebar_uptime(seconds,lang)).unwrap_or_else(|| tr(cx, super::backend::status(self.bridge.runtime.as_ref())).to_owned())),
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
                                                    .map(|mut button| {
                                                        button.interactivity().tooltip(move |_, cx| cx.new(|_| super::components::tooltip::TooltipContent(label.into())).into());
                                                        button
                                                    })
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
        // 设置页和代理页挂载同一个订阅弹窗；当前页面的路由和滚动容器不动。
        let subscription_modal = self.subscriptions.update(cx, |view, cx| {
            view.model
                .editor_open
                .then(|| view.render_modal(window, cx))
        });
        root.child(sidebar)
            .children(subscription_modal)
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
                                        .font_weight(t::proxies::FONT_SEMIBOLD)
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
                                                .font_weight(t::proxies::FONT_SEMIBOLD)
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

    // 保护用户看到的时长边界及切换语言，不回归为另一套 HH:MM:SS 格式。
    #[test]
    fn sidebar_uptime_matches_reference_units_and_language() {
        use veyra_core::domain::DesktopLanguage as L;
        for (seconds, expected) in [
            (0, "0秒"),
            (59, "59秒"),
            (60, "1分钟"),
            (3599, "59分钟"),
            (3600, "1小时"),
            (3660, "1小时1分钟"),
        ] {
            assert_eq!(
                super::sidebar_uptime(seconds, L::SimplifiedChinese),
                expected
            );
        }
        assert_eq!(
            super::sidebar_uptime(3660, L::TraditionalChinese),
            "1小時1分鐘"
        );
        assert_eq!(super::sidebar_uptime(3660, L::English), "1h1m");
    }

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

#[cfg(test)]
mod sidebar_stats_tests {
    use super::sidebar_stats;
    use veyra_core::application::{
        observability::controller::{Identity, ObservationService, Traffic},
        runtime_snapshot::InstanceId,
    };
    // 保护侧栏六项映射、未知不能假冒零、旧实例清除以及断流不能继续显示旧速率。
    #[test]
    fn stats_use_owned_snapshot_and_hide_unknown_or_old_instance() {
        let id = InstanceId("owned-sidebar".into());
        let mut snapshot = ObservationService::new().snapshot();
        assert_eq!(sidebar_stats(Some(&snapshot), Some(&id)), ["—"; 6]);
        snapshot.identity = Some(Identity {
            instance_id: id.clone(),
            generation: 1,
        });
        snapshot.available = [true; 4];
        snapshot.memory_bytes = Some(16 * 1024 * 1024);
        snapshot.connection_count = Some(3);
        snapshot.traffic = Some(Traffic {
            interval_ms: 1000,
            upload_bytes_per_second: 1200,
            download_bytes_per_second: 0,
            session_upload_bytes: 123456,
            session_download_bytes: 0,
        });
        assert_eq!(
            sidebar_stats(Some(&snapshot), Some(&id)),
            ["3", "16 MiB", "0 B", "0 B/s", "123.46 kB", "1.2 kB/s"]
        );
        assert_eq!(
            sidebar_stats(Some(&snapshot), Some(&InstanceId("replaced".into()))),
            ["—"; 6]
        );
        snapshot.available[0] = false;
        let values = sidebar_stats(Some(&snapshot), Some(&id));
        assert_eq!(values[3], "—");
        assert_eq!(values[5], "—");
        assert_eq!(values[4], "123.46 kB");
        snapshot.available[0] = true;
        snapshot.traffic.as_mut().unwrap().interval_ms = 0;
        assert_eq!(sidebar_stats(Some(&snapshot), Some(&id))[3], "—");
        assert_eq!(sidebar_stats(Some(&snapshot), None), ["—"; 6]);
    }
}
