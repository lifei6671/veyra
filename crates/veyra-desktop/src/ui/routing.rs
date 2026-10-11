//! 用户批准的目标分流最小切片：正式配置、图标、名称、匹配顺序与派生Selector。
use super::{
    components::{
        self, button, icon_button,
        icon_picker::{IconPicked, IconPicker, scaled_group_icon},
        select::SelectState,
    },
    i18n::tr,
    tokens::{self as t, proxies as pt, routing as rt},
};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, IndexPath, button::ButtonVariants, input::InputState,
        scroll::ScrollableElement,
    },
    prelude::*,
    *,
};
use std::sync::{Arc, Weak};
use veyra_core::domain::*;

#[derive(Clone)]
struct RuleDrag(usize);
impl Render for RuleDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::icons::icon("Bars3", t::ICON_SMALL).text_color(cx.theme().muted_foreground)
    }
}
#[derive(Clone)]
struct RoutingDrag {
    id: PoolId,
    name: String,
}
impl Render for RoutingDrag {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(t::GAP))
            .rounded(px(pt::CONTROL_RADIUS))
            .bg(rgba(0xffffffdd))
            .child(self.name.clone())
    }
}
#[derive(Clone, Copy, PartialEq)]
enum EditorKind {
    Policy,
    Fallback,
}
#[derive(Clone)]
struct RuleInput {
    kind: Entity<SelectState>,
    value: Entity<InputState>,
}
pub struct RoutingView {
    services: Weak<crate::services::AppServices>,
    state: Option<Box<AppState>>,
    pub busy: bool,
    error: Option<AppError>,
    draft: Option<TargetRoutingPolicy>,
    kind: EditorKind,
    name: Entity<InputState>,
    picker: Entity<IconPicker>,
    rules: Vec<RuleInput>,
    focus: FocusHandle,
    restore_focus: Option<FocusHandle>,
    close_after_save: bool,
    restore_open: bool,
    scroll: ScrollHandle,
    editor_scroll: ScrollHandle,
    _picker: gpui_kit::Subscription,
}
impl EventEmitter<Box<AppState>> for RoutingView {}
const RULES: [&str; 8] = [
    "域名",
    "域名后缀",
    "应用",
    "IP 段",
    "端口",
    "协议",
    "域名关键词",
    "规则集",
];
impl RoutingView {
    pub fn new(
        services: &Arc<crate::services::AppServices>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        let picker = cx.new(|cx| IconPicker::new(String::new(), window, cx));
        picker.update(cx, |picker, _| {
            picker.expanded = true;
            picker.expanded_width = rt::ICON_PICKER_WIDTH;
        });
        let subscription = cx.subscribe(&picker, |view, _, event: &IconPicked, cx| {
            if let Some(draft) = &mut view.draft {
                draft.icon = event.0.clone();
                cx.notify();
            }
        });
        Self {
            services: Arc::downgrade(services),
            state: None,
            busy: false,
            error: None,
            draft: None,
            kind: EditorKind::Policy,
            name,
            picker,
            rules: vec![],
            focus: cx.focus_handle(),
            restore_focus: None,
            close_after_save: false,
            restore_open: false,
            scroll: ScrollHandle::new(),
            editor_scroll: ScrollHandle::new(),
            _picker: subscription,
        }
    }
    pub fn project(&mut self, state: &AppState, cx: &mut Context<Self>) {
        self.state = Some(Box::new(state.clone()));
        // 后台投影不清空未保存草稿；保存失败仍可重试同一编辑内容。
        cx.notify();
    }
    pub fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Ok(policy) = TargetRoutingPolicy::fresh() else {
            self.error = Some(AppError::new(AppErrorCode::StorageFailed));
            return;
        };
        // 可选出站由正式目录实时派生，编辑器不冻结创建时的分组名单。
        self.open(policy, EditorKind::Policy, window, cx);
        self.add_rule(1, String::new(), window, cx);
    }
    pub fn open_fallback(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(state) = &self.state else { return };
        let policy = if let Some(policy) = &state.profile.routing.fallback {
            policy.clone()
        } else {
            let Ok(mut policy) = TargetRoutingPolicy::fresh() else {
                return;
            };
            policy.name = tr(cx, "其他").into();
            policy.icon = "globe:generic".into();
            policy
        };
        self.open(policy, EditorKind::Fallback, window, cx);
    }
    fn open(
        &mut self,
        policy: TargetRoutingPolicy,
        kind: EditorKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.name.update(cx, |input, cx| {
            input.set_value(policy.name.clone(), window, cx)
        });
        self.picker
            .update(cx, |picker, _| picker.value = policy.icon.clone());
        self.rules.clear();
        for matcher in &policy.matchers {
            let (kind, value) = matcher_input(matcher);
            self.add_rule(kind, value, window, cx);
        }
        self.draft = Some(policy);
        self.kind = kind;
        self.error = None;
        self.restore_focus = window.focused(cx);
        self.focus.focus(window, cx);
        self.editor_scroll.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.draft = None;
        self.restore_open = false;
        self.rules.clear();
        self.error = None;
        if let Some(focus) = self.restore_focus.take() {
            focus.focus(window, cx);
        }
        cx.notify();
    }
    fn add_rule(
        &mut self,
        kind: usize,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.rules.push(RuleInput {
            kind: cx
                .new(|cx| SelectState::new(RULES.to_vec(), Some(IndexPath::new(kind)), window, cx)),
            value: cx.new(|cx| InputState::new(window, cx).default_value(value)),
        });
        cx.notify();
    }
    fn persist(&mut self, routing: ProfileRouting, close: bool, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(state) = &self.state else { return };
        let Some(services) = self.services.upgrade() else {
            return;
        };
        let receiver = services.save_target_routing(state.config_version(), routing);
        self.begin_save(receiver, close, cx);
    }
    fn begin_save(
        &mut self,
        receiver: tokio::sync::oneshot::Receiver<Result<Box<AppState>, AppError>>,
        close: bool,
        cx: &mut Context<Self>,
    ) {
        self.busy = true;
        self.error = None;
        cx.notify();
        cx.spawn(async move |view, cx| {
            let result = receiver
                .await
                .unwrap_or_else(|_| Err(AppError::new(AppErrorCode::StorageFailed)));
            let _ = view.update(cx, |view, cx| {
                view.busy = false;
                match result {
                    Ok(state) => {
                        view.state = Some(state.clone());
                        // 返回正式提交快照后，下一次 window render 走同一关窗/焦点路径。
                        view.close_after_save = close;
                        cx.emit(state);
                    }
                    Err(error) => view.error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.restore_open = true;
        self.error = None;
        self.restore_focus = window.focused(cx);
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn restore_defaults(&mut self, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let Some(state) = &self.state else {
            return;
        };
        let Some(services) = self.services.upgrade() else {
            return;
        };
        let receiver = services.restore_target_routing(state.config_version());
        self.begin_save(receiver, true, cx);
    }
    fn restore_dialog(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let busy = self.busy;
        let surface = div()
            .flex()
            .flex_col()
            .w(px(rt::RESTORE_WIDTH))
            .rounded(px(pt::SETTINGS_RADIUS))
            .bg(cx.theme().popover.opacity(0.9))
            .child(components::modal_header(
                tr(cx, "恢复默认分流"),
                icon_button("routing-restore-close", tr(cx, "关闭"), "XMark", pt::TAB)
                    .disabled(busy)
                    .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
                cx,
            ))
            .child(div().p(px(t::PAD)).child(tr(
                cx,
                "恢复随包目标分流及显示排序？当前目标分流会被替换，订阅和出站节点保持不变。",
            )))
            .when(self.error.is_some(), |surface| {
                surface.child(
                    div()
                        .p(px(t::PAD))
                        .text_color(cx.theme().danger)
                        .child(tr(cx, self.error.as_ref().unwrap().code().message())),
                )
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(t::GAP))
                    .p(px(t::PAD))
                    .child(
                        button("routing-restore-cancel", tr(cx, "取消"))
                            .disabled(busy)
                            .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
                    )
                    .child(
                        button("routing-restore-confirm", tr(cx, "恢复默认"))
                            .primary()
                            .disabled(busy)
                            .on_click(cx.listener(|view, _, _, cx| view.restore_defaults(cx))),
                    ),
            );
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.focus.clone())
            .close_on_escape(!busy)
            .close_on_backdrop_press(!busy)
            .backdrop(div().size_full().bg(rgba(0x00000066)))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|view, _, window, cx| view.close(window, cx)))
            .into_any_element()
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        let Some(mut policy) = self.draft.clone() else {
            return;
        };
        policy.name = self.name.read(cx).value().trim().to_owned();
        policy.matchers = if self.kind == EditorKind::Fallback {
            vec![]
        } else {
            let mut matchers = vec![];
            for row in &self.rules {
                let kind = row.kind.read(cx).selected_value();
                let value = row.value.read(cx).value();
                let Some(matcher) = parse_matcher(kind, &value) else {
                    self.error = Some(AppError::new(AppErrorCode::Validation));
                    cx.notify();
                    return;
                };
                matchers.push(matcher);
            }
            matchers
        };
        let Some(state) = &self.state else { return };
        let mut routing = state.profile.routing.clone();
        if self.kind == EditorKind::Fallback {
            policy.enabled = true;
            routing.fallback = Some(policy);
        } else if let Some(old) = routing.policies.iter_mut().find(|p| p.id == policy.id) {
            *old = policy;
        } else {
            routing.policies.push(policy);
        }
        self.persist(routing, true, cx);
    }
    fn editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let draft = self.draft.as_ref().unwrap().clone();
        let busy = self.busy;
        self.picker.update(cx, |picker, _| picker.disabled = busy);
        let field = |label: &'static str, control: AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(px(t::ROW_GAP))
                .child(div().text_size(px(t::SMALL)).child(tr(cx, label)))
                .child(control)
        };
        let scale = div()
            .flex()
            .h(px(t::CONTROL))
            .child(
                icon_button(
                    "routing-scale-down",
                    tr(cx, "缩小 1px"),
                    "Minus",
                    t::CONTROL,
                )
                .disabled(busy)
                .on_click(cx.listener(|view, _, _, cx| {
                    if let Some(draft) = &mut view.draft {
                        draft.icon_scale -= 1;
                    }
                    cx.notify();
                })),
            )
            .child(
                div()
                    .w(px(rt::SCALE_VALUE_WIDTH))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(draft.icon_scale.to_string()),
            )
            .child(
                icon_button("routing-scale-up", tr(cx, "加大 1px"), "Plus", t::CONTROL)
                    .disabled(busy)
                    .on_click(cx.listener(|view, _, _, cx| {
                        if let Some(draft) = &mut view.draft {
                            draft.icon_scale += 1;
                        }
                        cx.notify();
                    })),
            )
            .child(
                button("routing-scale-reset", tr(cx, "重置"))
                    .disabled(busy || draft.icon_scale == 0)
                    .on_click(cx.listener(|view, _, _, cx| {
                        if let Some(draft) = &mut view.draft {
                            draft.icon_scale = 0;
                        }
                        cx.notify();
                    })),
            );
        let mut body = div()
            .id("routing-editor-scroll")
            .track_scroll(&self.editor_scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.editor_scroll)
            .w_full()
            .min_h_0()
            .max_h(window.viewport_size().height * 0.9 - px(97.))
            .p(px(t::PAD))
            .flex()
            .flex_col()
            .gap(px(t::PAD))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(t::GAP))
                    .child(field("图标", self.picker.clone().into_any_element()))
                    .child(field("缩放", scale.into_any_element()))
                    .child(
                        div().flex_1().min_w_0().child(field(
                            "站点集名称",
                            components::text_input(&self.name)
                                .readonly(busy)
                                .w_full()
                                .into_any_element(),
                        )),
                    ),
            );
        if self.kind == EditorKind::Policy {
            body = body.child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(tr(cx, "规则"))
                    .child(
                        icon_button("routing-add-rule", tr(cx, "添加规则"), "Plus", t::CONTROL)
                            .disabled(busy)
                            .on_click(cx.listener(|view, _, window, cx| {
                                view.add_rule(1, String::new(), window, cx)
                            })),
                    ),
            );
            body = body.children(self.rules.iter().enumerate().map(|(i, row)| {
                div()
                    .id(("routing-rule-row", i))
                    .when(!busy, |row| {
                        row.on_drag(RuleDrag(i), |drag, _, _, cx| cx.new(|_| drag.clone()))
                            .on_drop(cx.listener(move |view, drag: &RuleDrag, _, cx| {
                                if !view.busy && drag.0 != i {
                                    let rule = view.rules.remove(drag.0);
                                    view.rules.insert(i, rule);
                                    cx.notify();
                                }
                            }))
                    })
                    .w_full()
                    .flex()
                    .items_center()
                    .gap(px(t::GAP))
                    .child(
                        super::icons::icon("Bars3", t::ICON_SMALL)
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        components::select(&row.kind, tr(cx, "规则类型"))
                            .w(px(rt::MATCHER_WIDTH))
                            .disabled(busy),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(components::text_input(&row.value).readonly(busy).w_full()),
                    )
                    .child(
                        icon_button(
                            SharedString::from(format!("routing-delete-rule-{i}")),
                            tr(cx, "删除规则"),
                            "Trash",
                            t::CONTROL,
                        )
                        .disabled(busy)
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.rules.remove(i);
                            cx.notify();
                        })),
                    )
            }));
        }
        if let Some(error) = &self.error {
            body = body.child(
                div()
                    .text_color(cx.theme().danger)
                    .child(tr(cx, error.code().message())),
            );
        }
        let surface = div()
            .w(px(rt::EDITOR_WIDTH))
            .max_w(window.viewport_size().width)
            .max_h(window.viewport_size().height * 0.9)
            .font(t::openbox_font())
            .font_weight(pt::FONT_REGULAR)
            .text_size(px(t::BODY))
            .line_height(px(pt::CONTROL_LINE))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(pt::SETTINGS_RADIUS))
            .bg(cx.theme().popover.opacity(0.9))
            .child(components::modal_header(
                tr(
                    cx,
                    if self.kind == EditorKind::Fallback {
                        "修改兜底站点集"
                    } else if self.state.as_ref().is_some_and(|state| {
                        state
                            .profile
                            .routing
                            .policies
                            .iter()
                            .any(|p| p.id == draft.id)
                    }) {
                        "修改站点集"
                    } else {
                        "添加站点集"
                    },
                ),
                icon_button("routing-editor-close", tr(cx, "关闭"), "XMark", pt::TAB)
                    .disabled(busy)
                    .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
                cx,
            ))
            .child(body)
            .child(
                div()
                    .w_full()
                    .h(px(rt::FOOTER_HEIGHT))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(t::GAP))
                    .px(px(t::PAD))
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        button("routing-cancel", tr(cx, "取消"))
                            .disabled(busy)
                            .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
                    )
                    .child(
                        button("routing-save", tr(cx, "保存"))
                            .primary()
                            .disabled(busy)
                            .on_click(cx.listener(|view, _, _, cx| view.save(cx))),
                    ),
            );
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.focus.clone())
            .on_ok(|_, _, _| false)
            .close_on_escape(!busy)
            .close_on_backdrop_press(!busy)
            .backdrop(div().size_full().bg(rgba(0x00000066)))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|view, _, window, cx| view.close(window, cx)))
            .into_any_element()
    }
}
fn matcher_input(matcher: &TrafficMatcher) -> (usize, String) {
    match matcher {
        TrafficMatcher::Domain(v) => (0, v.join(", ")),
        TrafficMatcher::DomainSuffix(v) => (1, v.join(", ")),
        TrafficMatcher::DomainKeyword(v) => (6, v.join(", ")),
        TrafficMatcher::RuleSet(v) => (7, v.join(", ")),
        TrafficMatcher::Application(v) => (2, v.join(", ")),
        TrafficMatcher::IpCidr(v) => (3, v.join(", ")),
        TrafficMatcher::Port(v) => (
            4,
            v.iter().map(u16::to_string).collect::<Vec<_>>().join(", "),
        ),
        TrafficMatcher::Protocol(v) => (
            5,
            v.iter()
                .map(|p| match p {
                    NetworkProtocol::Tcp => "tcp",
                    NetworkProtocol::Udp => "udp",
                })
                .collect::<Vec<_>>()
                .join(", "),
        ),
    }
}
fn parse_matcher(kind: &str, value: &str) -> Option<TrafficMatcher> {
    let values = value
        .split([',', '\n'])
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if values.is_empty() {
        return None;
    }
    Some(match kind {
        "域名" => TrafficMatcher::Domain(values),
        "域名后缀" => TrafficMatcher::DomainSuffix(values),
        "域名关键词" => TrafficMatcher::DomainKeyword(values),
        "规则集" => TrafficMatcher::RuleSet(values),
        "应用" => TrafficMatcher::Application(values),
        "IP 段" => TrafficMatcher::IpCidr(values),
        "端口" => TrafficMatcher::Port(
            values
                .iter()
                .map(|v| v.parse().ok())
                .collect::<Option<Vec<u16>>>()?,
        ),
        "协议" => TrafficMatcher::Protocol(
            values
                .iter()
                .map(|v| match v.to_lowercase().as_str() {
                    "tcp" => Some(NetworkProtocol::Tcp),
                    "udp" => Some(NetworkProtocol::Udp),
                    _ => None,
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        _ => return None,
    })
}
impl Render for RoutingView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.close_after_save {
            self.close_after_save = false;
            self.close(window, cx);
        }
        let mut body = div()
            .w_full()
            .flex_1()
            .min_h_0()
            .id("routing-policies-scroll")
            .track_scroll(&self.scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.scroll)
            .flex()
            .flex_col()
            .gap(px(t::GAP));
        if let Some(state) = &self.state {
            let routing = state.profile.routing.clone();
            if let Some(custom) = &routing.custom {
                body = body.child(
                    div()
                        .w_full()
                        .min_h(px(rt::CARD_MIN_HEIGHT))
                        .flex()
                        .items_center()
                        .gap(px(t::GAP))
                        .p(px(t::PAD))
                        .rounded(cx.theme().radius_lg)
                        .bg(cx.theme().popover.opacity(0.75))
                        .child(scaled_group_icon(
                            &custom.icon,
                            rt::CARD_ICON,
                            custom.icon_scale,
                            cx,
                        ))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    custom
                                        .name
                                        .clone()
                                        .unwrap_or_else(|| tr(cx, "前置自定义分流").into()),
                                )
                                .child(tr(cx, "前置")),
                        ),
                );
            }
            let policies = routing.selectors().cloned().collect::<Vec<_>>();
            for (index, policy) in policies.into_iter().enumerate() {
                let fallback = routing.fallback.as_ref().is_some_and(|p| p.id == policy.id);
                let title = policy.name.clone();
                let drag_id = policy.id.clone();
                let drop_id = policy.id.clone();
                let summary = if fallback {
                    tr(cx, "上面都没命中的流量走它，在代理页选择出口。").into()
                } else {
                    policy
                        .matchers
                        .iter()
                        .map(|m| {
                            let (kind, value) = matcher_input(m);
                            format!("{}: {value}", tr(cx, RULES[kind]))
                        })
                        .collect::<Vec<_>>()
                        .join(" · ")
                };
                let card = div()
                    .id(("target-routing-policy", index))
                    .w_full()
                    .min_h(px(rt::CARD_MIN_HEIGHT))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .gap(px(t::GAP))
                    .p(px(t::PAD))
                    .rounded(cx.theme().radius_lg)
                    .bg(cx.theme().popover.opacity(0.75))
                    .opacity(if policy.enabled { 1. } else { 0.52 })
                    .when(!fallback, |card| {
                        card.on_drag(
                            RoutingDrag {
                                id: drag_id,
                                name: title.clone(),
                            },
                            |drag, _, _, cx| cx.new(|_| drag.clone()),
                        )
                        .on_drop(cx.listener(
                            move |view, drag: &RoutingDrag, _, cx| {
                                if view.busy {
                                    return;
                                }
                                let Some(state) = &view.state else { return };
                                let mut routing = state.profile.routing.clone();
                                if let Some(from) =
                                    routing.policies.iter().position(|p| p.id == drag.id)
                                    && let Some(to) =
                                        routing.policies.iter().position(|p| p.id == drop_id)
                                    && from != to
                                {
                                    let item = routing.policies.remove(from);
                                    routing.policies.insert(to, item);
                                    view.persist(routing, false, cx);
                                }
                            },
                        ))
                    })
                    .child(
                        super::icons::icon("Bars3", t::ICON_SMALL).opacity(if fallback {
                            0.
                        } else {
                            1.
                        }),
                    )
                    .child(scaled_group_icon(
                        &policy.icon,
                        rt::CARD_ICON,
                        policy.icon_scale,
                        cx,
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(div().font_weight(pt::FONT_MEDIUM).child(title))
                            .child(
                                div()
                                    .text_size(px(t::SMALL))
                                    .text_color(cx.theme().muted_foreground)
                                    .child(summary),
                            ),
                    )
                    .child(
                        icon_button(
                            SharedString::from(format!("routing-toggle-{index}")),
                            tr(
                                cx,
                                if policy.enabled {
                                    "当前已启用,点击停用"
                                } else {
                                    "当前已停用,点击启用"
                                },
                            ),
                            "Power",
                            t::CONTROL,
                        )
                        .disabled(self.busy || fallback)
                        .on_click(cx.listener({
                            let id = policy.id.clone();
                            move |view, _, _, cx| {
                                if let Some(state) = &view.state {
                                    let mut routing = state.profile.routing.clone();
                                    if let Some(policy) =
                                        routing.policies.iter_mut().find(|p| p.id == id)
                                    {
                                        policy.enabled = !policy.enabled;
                                    }
                                    view.persist(routing, false, cx);
                                }
                            }
                        })),
                    )
                    .child(
                        icon_button(
                            SharedString::from(format!("routing-edit-{index}")),
                            tr(cx, "编辑"),
                            "PencilSquare",
                            t::CONTROL,
                        )
                        .disabled(self.busy)
                        .on_click(cx.listener({
                            let policy = policy.clone();
                            move |view, _, window, cx| {
                                view.open(
                                    policy.clone(),
                                    if fallback {
                                        EditorKind::Fallback
                                    } else {
                                        EditorKind::Policy
                                    },
                                    window,
                                    cx,
                                )
                            }
                        })),
                    )
                    .child(
                        icon_button(
                            SharedString::from(format!("routing-delete-{index}")),
                            tr(cx, "删除"),
                            "Trash",
                            t::CONTROL,
                        )
                        .disabled(self.busy || fallback)
                        .on_click(cx.listener({
                            let id = policy.id.clone();
                            move |_view, _, window, cx| {
                                let answer = window.prompt(
                                    PromptLevel::Warning,
                                    tr(cx, "删除此站点集？"),
                                    None,
                                    &[tr(cx, "取消"), tr(cx, "删除")],
                                    cx,
                                );
                                let id = id.clone();
                                cx.spawn(async move |view, cx| {
                                    if answer.await == Ok(1) {
                                        let _ = view.update(cx, |view, cx| {
                                            if let Some(state) = &view.state {
                                                let mut routing = state.profile.routing.clone();
                                                routing.policies.retain(|p| p.id != id);
                                                view.persist(routing, false, cx);
                                            }
                                        });
                                    }
                                })
                                .detach();
                            }
                        })),
                    );
                body = body.child(card);
            }
        } else {
            body = body.child(tr(cx, "加载中"));
        }
        if let Some(error) = &self.error {
            body = body.child(
                div()
                    .text_color(cx.theme().danger)
                    .child(tr(cx, error.code().message())),
            );
        }
        body.when(self.draft.is_some(), |body| {
            body.child(self.editor(window, cx))
        })
        .when(self.restore_open, |body| {
            body.child(self.restore_dialog(cx))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // 保护目标分流输入的类型与值，不让展示翻译或非法值保存成其他规则。
    #[::core::prelude::v1::test]
    fn routing_rule_input_preserves_type_and_rejects_invalid_values() {
        assert_eq!(
            parse_matcher("域名后缀", " example.com, example.org\n"),
            Some(TrafficMatcher::DomainSuffix(vec![
                "example.com".into(),
                "example.org".into()
            ]))
        );
        assert_eq!(
            parse_matcher("端口", "443, 8443"),
            Some(TrafficMatcher::Port(vec![443, 8443]))
        );
        assert_eq!(
            parse_matcher("协议", "TCP, udp"),
            Some(TrafficMatcher::Protocol(vec![
                NetworkProtocol::Tcp,
                NetworkProtocol::Udp
            ]))
        );
        assert!(parse_matcher("端口", "65536").is_none());
        assert!(parse_matcher("协议", "icmp").is_none());
        assert!(parse_matcher("域名", " , ").is_none());
    }
}
