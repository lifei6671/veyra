//! P2-01 的 OpenBox 视觉迁移；所有业务仍通过同一个 AppServices/request generation。
use super::{
    components::{button, text_input},
    i18n::tr,
    tokens as t,
};
use crate::subscriptions::{Command, Completion, Model, Request, Status, error_key};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, Sizable,
        button::{ButtonCustomVariant, ButtonVariants},
        input::{InputEvent, InputState, Textarea, TextareaState},
        spinner::Spinner,
    },
    prelude::*,
    *,
};
use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use veyra_core::{
    application::subscription_management::{
        EditSubscription, SubscriptionOperationError as Error, SubscriptionSourceKind,
        preview::{PreviewInput, PreviewSource},
    },
    domain::AppState,
};

pub struct SubscriptionsEvent {
    pub request: Request,
    pub command: Command,
}
pub struct SubscriptionsView {
    pub model: Model,
    name: Entity<InputState>,
    description: Entity<InputState>,
    source: Entity<TextareaState>,
    dns_url: Entity<InputState>,
    dns_bootstrap: Entity<InputState>,
    urls: Vec<Entity<InputState>>,
    url_inputs: Vec<Subscription>,
    _inputs: Vec<Subscription>,
    visible: bool,
    // collapse 仅属于视图，不能改变 model generation 或任何持久事实。
    collapsed: HashSet<String>,
    refreshing: Option<String>,
    // DNS折叠只属于UI，不能推进请求generation或持久版本。
    dns_expanded: bool,
    share_collapsed: bool,
    // 分享弹窗是只读UI壳，不能生成token、二维码或向Core发送命令。
    share_editor_open: bool,
    share_inputs: Vec<Entity<InputState>>,
    // 本机OpenBox collapse仅作200ms grid高度过渡，不额外淡入淡出，也不进入Model。
    collapse_motion: HashMap<String, CollapseMotion>,
    editor_page: EditorPage,
    // 按实际内容高度居中弹窗；只影响布局，不进入请求或配置状态。
    editor_content_height: Pixels,
    // 保留弹窗滚动位置，使重新绘制/居中不会丢失滚动状态。
    editor_scroll: ScrollHandle,
    // 页面滚动与弹窗分层，长订阅列表不能裁掉deferred modal。
    page_scroll: ScrollHandle,
    // 分享选择列表独立保存滚动位置，不随输入框/弹窗重绘回到顶部。
    share_scroll: ScrollHandle,
    modal_focus: FocusHandle,
    modal_trigger: Option<FocusHandle>,
    loading: bool,
    load_error: bool,
    page_width: Pixels,
    // 后续规则能力的禁用控件仅保持视觉默认值；不参与input()/Save。
    rule_inputs: Vec<Entity<InputState>>,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum EditorPage {
    Source,
    Rules,
    Nodes,
}
struct CollapseMotion {
    from: (f32, f32),
    to: f32,
    started: Instant,
}
impl CollapseMotion {
    fn value(&self, now: Instant) -> (f32, f32) {
        let elapsed = now.duration_since(self.started).as_secs_f32();
        let height = (elapsed / (t::SUBSCRIPTION_COLLAPSE_MS as f32 / 1000.)).min(1.);
        // 本机 .collapse 的默认ease；内容opacity恒为1，闭合时由高度裁切。
        let ease_height = gpui_kit::component::animation::cubic_bezier(0.25, 0.1, 0.25, 1.)(height);
        (self.from.0 + (self.to - self.from.0) * ease_height, 1.)
    }
}
impl EventEmitter<SubscriptionsEvent> for SubscriptionsView {}
impl SubscriptionsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name = cx.new(|cx| InputState::new(window, cx));
        let description = cx.new(|cx| InputState::new(window, cx));
        let source = cx.new(|cx| TextareaState::new(window, cx));
        let mut inputs = vec![];
        for input in [&name, &description] {
            inputs.push(cx.subscribe(input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.model.invalidate();
                    cx.notify();
                }
            }));
        }
        inputs.push(cx.subscribe(&source, |this, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                this.model.invalidate();
                cx.notify();
            }
        }));
        Self {
            model: Model::default(),
            name,
            description,
            source,
            dns_url: cx.new(|cx| InputState::new(window, cx)),
            dns_bootstrap: cx.new(|cx| InputState::new(window, cx)),
            urls: vec![],
            url_inputs: vec![],
            _inputs: inputs,
            visible: false,
            collapsed: HashSet::new(),
            refreshing: None,
            dns_expanded: false,
            share_collapsed: false,
            share_editor_open: false,
            share_inputs: (0..3)
                .map(|_| cx.new(|cx| InputState::new(window, cx)))
                .collect(),
            collapse_motion: HashMap::new(),
            editor_page: EditorPage::Source,
            editor_content_height: px(480.),
            editor_scroll: ScrollHandle::new(),
            page_scroll: ScrollHandle::new(),
            share_scroll: ScrollHandle::new(),
            modal_focus: cx.focus_handle(),
            modal_trigger: None,
            loading: false,
            load_error: false,
            page_width: px(0.),
            rule_inputs: RULE_VALUES
                .iter()
                .map(|value| cx.new(|cx| InputState::new(window, cx).default_value(*value)))
                .collect(),
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            if !visible {
                self.model.editor_open = false;
                self.share_editor_open = false;
            }
            self.model.invalidate();
            self.refreshing = None;
            self.loading = false;
            // 与 React 页面重新挂载一致：离页后旧写入可在后台完成，回页重新读事实版本。
            if visible {
                self.send(Command::Load, cx);
            }
            cx.notify();
        }
    }
    pub fn project(&mut self, state: &AppState, cx: &mut Context<Self>) {
        self.model.project(state);
        cx.notify();
    }
    pub fn complete(
        &mut self,
        request: &Request,
        result: Result<Completion, Error>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Box<AppState>> {
        let accepted = self.model.accepts(request);
        if accepted {
            self.load_error = self.loading && result.is_err();
        }
        let state = self.model.complete(request, result);
        if accepted {
            self.loading = false;
            self.refreshing = None;
            // React refresh/delete 失败走全局 toast，不另加页面级主操作/信息卡。
            if !self.model.editor_open
                && !self.model.list.is_empty()
                && let Status::Error(error) = self.model.status
            {
                super::components::notify(
                    super::components::Notice::Error,
                    error_key(error),
                    window,
                    cx,
                );
            }
        }
        if state.is_some() && self.model.editor_open {
            let view = cx.entity().downgrade();
            window.on_next_frame(move |window, cx| {
                let _ = view.update(cx, |this, cx| {
                    if this.model.editor_open {
                        this.name.read(cx).focus_handle(cx).focus(window, cx);
                    }
                });
            });
        }
        cx.notify();
        state
    }
    fn send(&mut self, command: Command, cx: &mut Context<Self>) {
        self.loading = matches!(command, Command::Load);
        self.load_error = false;
        self.refreshing = match &command {
            Command::Refresh { id, .. } => Some(id.clone()),
            _ => None,
        };
        let request = self.model.begin();
        cx.emit(SubscriptionsEvent { request, command });
        cx.notify();
    }
    fn add_url(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("https://"));
        self.url_inputs
            .push(cx.subscribe(&input, |this, _, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.model.invalidate();
                    cx.notify();
                }
            }));
        self.urls.push(input);
        self.model.invalidate();
        cx.notify();
    }
    fn input(&self, cx: &App) -> PreviewInput {
        PreviewInput {
            name: self.name.read(cx).value().to_string(),
            description: self.description.read(cx).value().to_string(),
            sources: if self.model.manual {
                vec![PreviewSource::Pasted(
                    self.source.read(cx).value().to_string(),
                )]
            } else {
                self.urls
                    .iter()
                    .map(|i| i.read(cx).value().trim().to_owned())
                    .filter(|v| !v.is_empty())
                    .map(PreviewSource::Url)
                    .collect()
            },
        }
    }
    pub fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open_editor(None, window, cx);
    }
    fn open_editor(&mut self, id: Option<String>, window: &mut Window, cx: &mut Context<Self>) {
        self.model.invalidate();
        self.model.editing = id.clone();
        self.dns_expanded = false;
        self.editor_page = EditorPage::Source;
        self.model.editor_open = true;
        let item = id
            .as_ref()
            .and_then(|id| self.model.list.iter().find(|s| &s.id == id));
        self.model.manual = item.is_some_and(|s| s.source_kind == SubscriptionSourceKind::Manual);
        let name = item.map(|s| s.name.clone()).unwrap_or_default();
        let description = item.map(|s| s.description.clone()).unwrap_or_default();
        self.name.update(cx, |i, cx| i.set_value(name, window, cx));
        self.description
            .update(cx, |i, cx| i.set_value(description, window, cx));
        self.source.update(cx, |i, cx| i.set_value("", window, cx));
        self.urls.clear();
        self.url_inputs.clear();
        self.add_url(window, cx);
        // 弹窗使用无样式Kit Dialog，保留焦点/Escape，禁止styled Dialog默认从顶部滑入。
        self.modal_trigger = window.focused(cx);
        self.name.read(cx).focus_handle(cx).focus(window, cx);
        cx.notify();
    }
    fn render_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let palette = SettingsColors::from_theme(cx);
        let share = self.share_editor_open;
        let busy = !share && self.model.busy();
        let footer_height = if share {
            t::SUBSCRIPTION_SHARE_FOOTER
        } else {
            t::SUBSCRIPTION_FOOTER
        };
        let title = tr(
            cx,
            if share {
                "新增订阅分享"
            } else if self.model.editing.is_some() {
                "修改订阅"
            } else {
                "添加订阅或节点"
            },
        );
        let available_body =
            window.viewport_size().height * 0.9 - px(t::DIALOG_HEADER + footer_height);
        let header = div()
            .h(px(t::DIALOG_HEADER))
            .flex_shrink_0()
            .px(px(t::SECTION_PADDING))
            .py(px(t::GAP))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(palette.text.opacity(0.1))
            .child(
                div()
                    .text_size(px(t::SECTION_TITLE))
                    .line_height(px(t::SECTION_LINE))
                    .font_weight(FontWeight::BOLD)
                    .text_color(palette.text)
                    .child(title),
            )
            .child(
                super::components::icon_button(
                    "subscription-close",
                    tr(cx, "关闭"),
                    "XMark",
                    t::SWITCH_HEIGHT,
                )
                .text_color(palette.text)
                .openbox_ghost(palette.button_hover(), palette.text)
                .mr(-px(t::GAP))
                .disabled(busy)
                .on_click(|_, window, cx| {
                    window.dispatch_action(Box::new(gpui_kit::component::dialog::Cancel), cx)
                }),
            );
        let view = cx.entity().downgrade();
        let body = if share {
            self.share_editor(window, cx)
        } else {
            self.editor(window, cx)
        };
        let measured = div()
            .on_children_prepainted(move |bounds, _, cx| {
                let natural = bounds.iter().map(|b| b.size.height).sum::<Pixels>()
                    + px(t::SECTION_PADDING * 2.);
                let height = natural.min(available_body) + px(t::DIALOG_HEADER);
                let view = view.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |this, cx| {
                        if this.editor_content_height != height {
                            this.editor_content_height = height;
                            cx.notify();
                        }
                    });
                });
            })
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .child(body.flex_shrink_0());
        let surface = div()
            .id(if share {
                "subscription-share-modal"
            } else {
                "subscription-editor-modal"
            })
            .w(px(if share {
                t::SUBSCRIPTION_SHARE_EDITOR_WIDTH
            } else {
                t::SUBSCRIPTION_EDITOR_WIDTH
            }))
            .max_w(window.viewport_size().width - px(t::SECTION_PADDING * 2.))
            .max_h(window.viewport_size().height * 0.9)
            .flex()
            .flex_col()
            .min_h_0()
            .rounded(px(t::SUBSCRIPTION_EDITOR_RADIUS))
            .bg(palette.solid)
            .text_color(palette.text)
            .overflow_hidden()
            .child(header)
            .child(
                div()
                    .id("subscription-editor-body")
                    .track_scroll(&self.editor_scroll)
                    .flex()
                    .flex_col()
                    .h(self.editor_content_height - px(t::DIALOG_HEADER))
                    .max_h(available_body)
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(t::SECTION_PADDING))
                    .child(measured),
            )
            .child(self.footer(cx));
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.modal_focus.clone())
            .close_on_escape(!busy)
            .close_on_backdrop_press(!busy)
            .on_ok(|_, _, _| false)
            .on_cancel(move |_, _, _| !busy)
            .backdrop(div().size_full().bg(cx.theme().overlay))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|this, _, window, cx| {
                if this.share_editor_open {
                    this.share_editor_open = false;
                } else {
                    this.model.editor_open = false;
                    this.model.invalidate();
                    this.refreshing = None;
                }
                if let Some(handle) = this.modal_trigger.take() {
                    handle.focus(window, cx);
                }
                cx.notify();
            }))
            .into_any_element()
    }
    fn source_input(&self, input: &Entity<InputState>, p: SettingsColors) -> impl IntoElement {
        text_input(input)
            .disabled(self.model.busy())
            .border_focus()
            .bg(p.solid)
            .border_color(p.line)
            .text_color(p.text)
            .font_weight(FontWeight::NORMAL)
    }
    fn dns_toggle(
        &self,
        p: SettingsColors,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> super::components::PanelButton {
        let angle = gpui_kit::base::motion::transition(
            "subscription-dns-angle",
            if self.dns_expanded {
                std::f32::consts::PI
            } else {
                0.
            },
            gpui_kit::base::motion::Transition::new(Duration::from_millis(
                t::SUBSCRIPTION_DNS_ROTATION_MS,
            ))
            .ease(gpui_kit::component::animation::cubic_bezier(
                0.4, 0., 0.2, 1.,
            )),
            window,
            cx,
        );
        button("subscription-dns-toggle", "")
            .accessibility_label(tr(cx, "订阅 DNS 解析"))
            .ghost()
            .h(px(t::SWITCH_HEIGHT))
            .px(px(t::GAP))
            .text_color(p.text)
            // Kit label固定14px；自绘文本明确沿用source CSS 11px，不能只改外层。
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::ROW_GAP))
                    .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                    .font_weight(FontWeight::NORMAL)
                    .child(tr(cx, "订阅 DNS 解析"))
                    .child(super::icons::icon("ChevronDown", t::BODY).rotate(Radians(angle))),
            )
            .reference_hover(rgba(0).into(), p.button_hover(), p.text)
            .tooltip(tr(cx, "后续任务能力，当前不可用"))
            .on_click(cx.listener(|this, _, _, cx| {
                this.dns_expanded = !this.dns_expanded;
                cx.notify();
            }))
    }
    fn editor_tabs(&mut self, cx: &mut Context<Self>) -> Div {
        let p = SettingsColors::from_theme(cx);
        let mut tabs = div()
            .flex()
            .items_center()
            .min_h(px(t::SUBSCRIPTION_TAB_HEIGHT))
            .p(px(t::ROW_GAP))
            .rounded(px(t::RADIUS))
            .bg(p.base200());
        for (page, id, key) in [
            (EditorPage::Source, "subscription-editor-source", "订阅"),
            (EditorPage::Rules, "subscription-editor-rules", "规则"),
            (EditorPage::Nodes, "subscription-editor-nodes", "节点"),
        ] {
            // 规则可进入查看禁用表单，不提供编辑或保存的假业务。
            tabs = tabs.child(
                editor_tab(
                    id,
                    if page == EditorPage::Nodes && self.model.editing.is_some() {
                        format!(
                            "{} ({})",
                            tr(cx, key),
                            self.model
                                .nodes
                                .iter()
                                .filter(|n| Some(&n.subscription_id) == self.model.editing.as_ref())
                                .count()
                        )
                    } else {
                        tr(cx, key).to_owned()
                    },
                    self.editor_page == page,
                    self.model.busy(),
                    p,
                    cx,
                )
                .when(page == EditorPage::Rules, |b| {
                    b.tooltip(tr(cx, "规则编辑将在后续任务开放"))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.editor_page = page;
                    this.editor_scroll.set_offset(point(px(0.), px(0.)));
                    cx.notify();
                })),
            );
        }
        tabs
    }
    fn source_types(&mut self, cx: &mut Context<Self>) -> Div {
        let p = SettingsColors::from_theme(cx);
        let mut tabs = div()
            .flex()
            .items_center()
            .min_h(px(t::SUBSCRIPTION_TAB_HEIGHT))
            .p(px(t::ROW_GAP))
            .rounded(px(t::RADIUS))
            .bg(p.base200())
            .when(self.model.editing.is_some(), |d| d.opacity(0.6));
        for (manual, id, key) in [
            (false, "subscription-url", "订阅"),
            (true, "subscription-paste", "节点"),
        ] {
            tabs = tabs.child(
                editor_tab(
                    id,
                    tr(cx, key),
                    manual == self.model.manual,
                    self.model.editing.is_some()
                        || self.model.busy()
                        || self.editor_page != EditorPage::Source,
                    p,
                    cx,
                )
                .when(self.model.editing.is_some(), |b| {
                    b.tooltip(tr(cx, "来源类型不能修改；要换请新建一条订阅。"))
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.model.manual = manual;
                    this.model.invalidate();
                    cx.notify();
                })),
            );
        }
        tabs
    }
    fn editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let busy = self.model.busy();
        let editing = self.model.editing.is_some();
        let p = SettingsColors::from_theme(cx);
        let tabs = self.editor_tabs(cx);
        let source_types = self.source_types(cx);
        if self.editor_page != EditorPage::Source {
            let region = match self.editor_page {
                EditorPage::Rules => self.rules_shell(cx),
                EditorPage::Nodes => {
                    let nodes: Vec<_> = self
                        .model
                        .nodes
                        .iter()
                        .filter(|n| Some(&n.subscription_id) == self.model.editing.as_ref())
                        .cloned()
                        .collect();
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(t::PAD))
                        .when(self.model.editing.is_some(), |d| {
                            d.min_h(px(t::SUBSCRIPTION_NODES_PANEL_HEIGHT))
                        })
                        .child(if self.model.editing.is_none() {
                            // 只迁移preview-state容器；Host已取消实时Preview能力。
                            div()
                                .p(px(t::SUBSCRIPTION_NODES_STATE_PADDING))
                                .border_1()
                                .border_dashed()
                                .border_color(p.text.opacity(0.15))
                                .rounded(px(t::SUBSCRIPTION_NODES_STATE_RADIUS))
                                .flex()
                                .flex_col()
                                .gap(px(t::GAP))
                                .items_center()
                                .justify_center()
                                .text_color(p.text.opacity(0.5))
                                .text_size(px(t::BODY))
                                .line_height(px(t::BODY_LINE))
                                .text_center()
                                .child(super::icons::icon(
                                    "MagnifyingGlass",
                                    t::SUBSCRIPTION_NODES_STATE_ICON,
                                ))
                                .child(tr(cx, "保存后可在这里查看订阅节点"))
                        } else {
                            self.saved_nodes_panel(&nodes, p, cx)
                        })
                }
                EditorPage::Source => unreachable!(),
            };
            return div()
                .flex()
                .flex_col()
                .gap(px(t::SECTION_PADDING))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(source_types)
                        .child(tabs),
                )
                .child(region.flex_shrink_0());
        }
        // 切换语言只更新提示，保留用户草稿与输入状态。
        self.name.update(cx, |i, cx| {
            i.set_placeholder(tr(cx, "比如\"我的订阅\""), window, cx)
        });
        self.source.update(cx, |i, cx| {
            i.set_placeholder(
                tr(
                    cx,
                    "粘贴 ss:// / trojan:// / hysteria2:// 等链接,或 Clash/sing-box 配置…",
                ),
                window,
                cx,
            )
        });
        self.dns_url.update(cx, |i, cx| {
            i.set_placeholder(
                tr(cx, "DoH 地址,如 https://dns.example.net:2096/路径"),
                window,
                cx,
            )
        });
        self.dns_bootstrap.update(cx, |i, cx| {
            i.set_placeholder(tr(cx, "解析 DoH 的 DNS(IP)"), window, cx)
        });
        let mut form = div()
            .flex()
            .flex_col()
            .gap(px(t::SECTION_PADDING))
            // OpenBox 来源表单没有说明字段；隐藏该输入，但编辑时仍保留已保存的说明。
            .child(field(tr(cx, "名称"), self.source_input(&self.name, p)));
        if self.model.manual {
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::GAP))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .min_h(px(t::SWITCH_HEIGHT))
                            .child(label(tr(cx, "节点内容")))
                            .child(self.dns_toggle(p, window, cx)),
                    )
                    .child(
                        Textarea::new(&self.source)
                            .disabled(busy)
                            .appearance(false)
                            .border_1()
                            .border_color(
                                if self.source.read(cx).focus_handle(cx).is_focused(window) {
                                    rgb(t::ACCENT).into()
                                } else {
                                    p.line
                                },
                            )
                            .bg(p.solid)
                            .text_color(p.text)
                            .h(px(t::SUBSCRIPTION_TEXTAREA_HEIGHT))
                            .w_full()
                            .rounded(px(t::RADIUS))
                            .px(px(t::SUBSCRIPTION_TEXTAREA_PX))
                            .py(px(t::SUBSCRIPTION_TEXTAREA_PY))
                            .font_family("SFMono-Regular")
                            .text_size(px(t::SUBSCRIPTION_TEXTAREA_FONT))
                            .line_height(px(t::SUBSCRIPTION_TEXTAREA_LINE)),
                    )
                    .child(hint(
                        tr(
                            cx,
                            if editing {
                                "来源留空保留原内容；替换 URL 仅输入一个地址"
                            } else {
                                "一行一个节点链接（ss:// / trojan:// / vless:// / socks5:// 等）;也支持整段 Clash / sing-box 配置。"
                            },
                        ),
                        p,
                    )),
            );
        } else {
            let mut urls = div().flex().flex_col().gap(px(t::GAP));
            let single = self.urls.len() == 1;
            for (ix, input) in self.urls.iter().enumerate() {
                urls = urls.child(
                    div()
                        .id(("subscription-url-row", input.entity_id()))
                        .flex()
                        .items_center()
                        .gap(px(t::GAP))
                        .child(div().flex_1().min_w_0().child(self.source_input(input, p)))
                        .child(
                            icon_action(("subscription-url-remove", ix), "删除", "Trash", p, cx)
                                .text_color(if single || busy {
                                    p.text.opacity(0.3)
                                } else {
                                    p.text
                                })
                                .disabled(single || busy)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.urls.remove(ix);
                                    drop(this.url_inputs.remove(ix));
                                    // 删除按钮随行卸载后，把焦点交给相邻输入，保留 modal 的键盘作用域。
                                    this.urls[ix.min(this.urls.len() - 1)]
                                        .read(cx)
                                        .focus_handle(cx)
                                        .focus(window, cx);
                                    this.model.invalidate();
                                    cx.notify();
                                })),
                        ),
                );
            }
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::GAP))
                    .child(
                        div()
                            .min_h(px(t::SWITCH_HEIGHT))
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(label(tr(cx, "订阅链接")))
                            .when(!editing, |d| {
                                d.child(
                                    button("subscription-url-add", "")
                                        .accessibility_label(tr(cx, "添加订阅"))
                                        .ghost()
                                        .reference_hover(rgba(0).into(), p.button_hover(), p.text)
                                        .h(px(t::SWITCH_HEIGHT))
                                        .px(px(t::GAP))
                                        .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .disabled(busy || self.urls.len() >= 32)
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
                                                .child(super::icons::icon("Plus", t::BODY))
                                                .child(tr(cx, "添加订阅")),
                                        )
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.add_url(window, cx)
                                        })),
                                )
                            }),
                    )
                    .child(urls)
                    .child(hint(
                        tr(
                            cx,
                            if editing {
                                "来源留空保留原内容；替换 URL 仅输入一个地址"
                            } else {
                                "一行一个 URL；按输入顺序保存有效来源，各来源分别形成订阅。"
                            },
                        ),
                        p,
                    ))
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap(px(t::PAD))
                            .pt(px(t::ROW_GAP))
                            .min_h(px(t::SUBSCRIPTION_OPTIONS_HEIGHT))
                            .child(
                                div()
                                    .id("subscription-periodic-tooltip")
                                    .flex()
                                    .items_center()
                                    .gap(px(t::PAD))
                                    .tooltip(|_, cx| {
                                        cx.new(|cx| {
                                            super::components::tooltip::TooltipContent(
                                                tr(cx, "自动更新将在后续任务开放").into(),
                                            )
                                        })
                                        .into()
                                    })
                                    .child(hint(tr(cx, "定期更新"), p).text_color(p.text))
                                    .child(
                                        gpui_kit::base::Switch::new("subscription-periodic-update")
                                            .accessibility_label(tr(cx, "定期更新"))
                                            .disabled(true)
                                            .w(px(t::SUBSCRIPTION_SWITCH_WIDTH))
                                            .h(px(t::SUBSCRIPTION_SWITCH_HEIGHT))
                                            .p(px(2.))
                                            .border_1()
                                            .border_color(p.muted)
                                            .rounded(px(t::RADIUS))
                                            .bg(p.surface)
                                            .child(
                                                div()
                                                    .size(px(t::SUBSCRIPTION_SWITCH_THUMB))
                                                    .rounded_full()
                                                    .bg(p.muted),
                                            ),
                                    ),
                            )
                            .child(div().flex_1())
                            .child(self.dns_toggle(p, window, cx)),
                    ),
            );
        }
        if self.dns_expanded {
            // DNS尚无本卡Core契约：原版输入区只作明确禁用的视觉壳，不收集或假保存。
            let dns_input = |input: &Entity<InputState>| {
                text_input(input)
                    .disabled(true)
                    .border_focus()
                    .bg(p.solid)
                    .border_color(p.line)
                    .text_color(p.text)
            };
            form = form.child(div().id("subscription-dns-disabled").flex().flex_col().gap(px(t::GAP))
                .child(label(tr(cx, "节点域名解析（可选）")))
                .child(div().flex().gap(px(t::GAP))
                    .child(div().flex_1().min_w_0().child(dns_input(&self.dns_url)))
                    .child(div().w(px(t::SUBSCRIPTION_DNS_BOOTSTRAP_WIDTH)).child(dns_input(&self.dns_bootstrap))))
                .child(hint(tr(cx, "机场要求节点域名用它自己的 DoH 解析时填(相当于 OpenClash 的 proxy-server-nameserver / nameserver-policy)。只用来解析这份订阅的节点服务器域名,普通网站和局域网的 DNS 不受影响;测速也按它解析。DoH 地址是域名时,右边要填解析它用的 DNS(IP)。"), p))
                .tooltip(|_, cx| cx.new(|cx| super::components::tooltip::TooltipContent(tr(cx,"后续任务：节点 DNS 配置暂不可用").into())).into()));
        }
        let mut body = div()
            .flex()
            .flex_col()
            .gap(px(t::SECTION_PADDING))
            .w_full()
            .min_w_0()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(t::SECTION_PADDING))
                    .child(source_types)
                    .child(tabs),
            )
            .child(form);
        if let Status::Error(error) = self.model.status {
            body = body.child(notice(error_key(error), t::SUBSCRIPTION_ERROR, cx));
        }
        if self.model.status == Status::Saved {
            body = body.child(hint(tr(cx, "已保存，尚未应用"), p));
            if let Some(saved) = &self.model.saved {
                // 保留多来源保存事实的必要解释；不再展示预览面板/节点表。
                for source in &saved.sources {
                    let safe = if source.kind == SubscriptionSourceKind::Manual {
                        tr(cx, "粘贴内容")
                    } else {
                        &source.safe_display
                    };
                    body = body.child(hint(
                        format!(
                            "{} · {} · {} {} / {} {}",
                            source.index + 1,
                            safe,
                            source.node_count,
                            tr(cx, "节点"),
                            source.skipped.len(),
                            tr(cx, "跳过")
                        ),
                        p,
                    ));
                    if let Some(error) = source.error {
                        body = body.child(notice(error_key(error), t::SUBSCRIPTION_ERROR, cx));
                    }
                }
                if saved.subscriptions.len() > 1 {
                    body = body.child(hint(
                        tr(cx, "每个有效 URL 分别保存为一个订阅；失败来源未保存"),
                        p,
                    ));
                }
            }
        }
        body
    }
    fn share_panel(&self, p: SettingsColors, cx: &mut Context<Self>) -> Div {
        let target = if self.share_collapsed { 0. } else { 1. };
        let (height, opacity) = self
            .collapse_motion
            .get("share")
            .map_or((target, target), |motion| motion.value(Instant::now()));
        div()
            .w_full()
            .min_w_0()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .min_h(px(t::SUBSCRIPTION_SHARE_HEADER
                + (t::SUBSCRIPTION_SHARE_HEIGHT
                    - t::SUBSCRIPTION_SHARE_HEADER)
                    * height))
            .rounded(cx.theme().radius_lg)
            .bg(p.surface)
            .child(
                div()
                    .h(px(t::SUBSCRIPTION_SHARE_HEADER))
                    .flex_shrink_0()
                    .p(px(t::SECTION_PADDING))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(t::SECTION_PADDING))
                    .child(
                        gpui_kit::base::Button::new("subscription-share-toggle")
                            .p_0()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .items_start()
                            .accessibility_label(tr(
                                cx,
                                if self.share_collapsed {
                                    "展开订阅分享"
                                } else {
                                    "收起订阅分享"
                                },
                            ))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(t::ROW_GAP))
                                    .child(
                                        div()
                                            .text_size(px(t::SECTION_TITLE))
                                            .line_height(px(t::SECTION_LINE))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(tr(cx, "订阅分享")),
                                    )
                                    .child(
                                        super::icons::icon(
                                            if self.share_collapsed {
                                                "ChevronDown"
                                            } else {
                                                "ChevronUp"
                                            },
                                            t::SUBSCRIPTION_SHARE_CHEVRON,
                                        )
                                        .text_color(p.muted),
                                    ),
                            )
                            .child(
                                hint(tr(cx, "生成可供其他设备或代理软件直接使用的订阅链接"), p)
                                    .line_height(px(t::SUBSCRIPTION_LABEL_LINE))
                                    .mt(px(t::ROW_GAP)),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                let now = Instant::now();
                                let target = if this.share_collapsed { 0. } else { 1. };
                                let from = this
                                    .collapse_motion
                                    .get("share")
                                    .map_or((target, target), |motion| motion.value(now));
                                this.share_collapsed = !this.share_collapsed;
                                this.collapse_motion.insert(
                                    "share".into(),
                                    CollapseMotion {
                                        from,
                                        to: 1. - target,
                                        started: now,
                                    },
                                );
                                cx.notify();
                            })),
                    )
                    .child(
                        icon_action("subscription-share-add", "添加订阅分享", "Plus", p, cx)
                            .primary()
                            .disabled(self.model.list.is_empty() || self.model.busy())
                            .with_tooltip(tr(cx, "只读界面预览；订阅分享将在后续任务开放"))
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.share_editor_open = true;
                                this.editor_scroll = ScrollHandle::new();
                                this.share_scroll = ScrollHandle::new();
                                this.modal_trigger = window.focused(cx);
                                this.modal_focus.focus(window, cx);
                                cx.notify();
                            })),
                    ),
            )
            .when(height > 0., |d| {
                d.child(
                    div()
                        .h(px(t::SUBSCRIPTION_NODE_EMPTY * height))
                        .overflow_hidden()
                        .child(
                            div()
                                .h(px(t::SUBSCRIPTION_NODE_EMPTY))
                                .opacity(opacity)
                                .flex_shrink_0()
                                .pt(px(t::PAD))
                                .px(px(t::SECTION_PADDING))
                                .pb(px(t::SECTION_PADDING))
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(p.muted)
                                .text_size(px(t::BODY))
                                .child(tr(cx, "暂无订阅分享，点右上角「添加」创建")),
                        ),
                )
            })
    }
    fn share_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = SettingsColors::from_theme(cx);
        for (input, key) in self.share_inputs.iter().zip([
            "例如:手机代理订阅",
            "当前地址",
            "订阅分享将在后续任务开放",
        ]) {
            input.update(cx, |state, cx| {
                state.set_placeholder(tr(cx, key), window, cx)
            });
        }
        let input = |ix: usize| {
            text_input(&self.share_inputs[ix])
                .disabled(true)
                .border_focus()
                .bg(p.solid)
                .border_color(p.line)
                .text_color(p.text)
        };
        let modal_width = px(t::SUBSCRIPTION_SHARE_EDITOR_WIDTH)
            .min(window.viewport_size().width - px(t::SECTION_PADDING * 2.));
        let left_width = (modal_width
            - px(t::SECTION_PADDING * 2. + t::SUBSCRIPTION_SHARE_COLUMNS_GAP))
            * t::SUBSCRIPTION_SHARE_LEFT_FRACTION;
        let mut subscriptions = div()
            .id("subscription-share-list")
            .flex()
            .flex_col()
            .w_full()
            .min_h_0()
            .h(px((self.model.list.len() as f32
                * t::SUBSCRIPTION_SHARE_ROW_HEIGHT)
                .min(t::SUBSCRIPTION_SHARE_LIST_MAX_HEIGHT)))
            .pr(px(t::ROW_GAP))
            .overflow_y_scroll()
            .track_scroll(&self.share_scroll);
        for (ix, item) in self.model.list.iter().enumerate() {
            subscriptions = subscriptions.child(
                div()
                    .flex()
                    .items_center()
                    .h(px(t::SUBSCRIPTION_SHARE_ROW_HEIGHT))
                    .flex_shrink_0()
                    .gap(px(t::GAP))
                    .border_b_1()
                    .border_color(p.text.opacity(0.1))
                    .when(ix == 0, |d| d.border_t_1())
                    .child(
                        gpui_kit::base::Button::new(("subscription-share-check", ix))
                            .accessibility_label(format!(
                                "{} · {}",
                                item.name,
                                tr(cx, "选择要分享的订阅")
                            ))
                            .disabled(true)
                            .size(px(t::SUBSCRIPTION_SHARE_CHECKBOX))
                            .border_1()
                            .border_color(p.line)
                            .rounded(px(t::SUBSCRIPTION_CHECKBOX_RADIUS)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .text_size(px(t::BODY))
                            .child(item.name.clone()),
                    )
                    .child(hint(format!("{} {}", item.node_count, tr(cx, "个节点")), p)),
            );
        }
        let host = div()
            .flex()
            .w_full()
            .min_w_0()
            .child(
                unavailable_select(
                    "subscription-share-protocol",
                    "http://",
                    t::SUBSCRIPTION_SHARE_PROTOCOL_WIDTH,
                    p,
                    cx,
                )
                .rounded_r(px(0.))
                .flex_shrink_0(),
            )
            .child(div().flex_1().min_w_0().child(input(1).rounded_l(px(0.))));
        let link = div()
            .flex()
            .w_full()
            .min_w_0()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(input(2).rounded_r(px(0.)).font_family("SFMono-Regular")),
            )
            .child(
                icon_action(
                    "subscription-share-copy",
                    "复制分享链接",
                    "ClipboardDocument",
                    p,
                    cx,
                )
                .disabled(true)
                .w(px(t::SUBSCRIPTION_SHARE_COPY_WIDTH))
                .rounded_l(px(0.))
                .bg(p.base200())
                .with_tooltip(tr(cx, "订阅分享将在后续任务开放")),
            );
        // 保留原版二维码区域的尺寸与位置；明确不可用，禁止制造可扫码的假分享地址。
        let qr = div()
            .size(px(t::SUBSCRIPTION_SHARE_QR_SIZE))
            .rounded(cx.theme().radius_lg)
            .bg(rgb(0xffffff))
            .p(px(t::ROW_GAP))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(rgb(t::SUBSCRIPTION_UNTESTED))
                    .text_size(px(t::SUBSCRIPTION_LABEL_FONT))
                    .child(tr(cx, "二维码暂不可用")),
            );
        div()
            .flex()
            .gap(px(t::SUBSCRIPTION_SHARE_COLUMNS_GAP))
            .w_full()
            .min_h(px(t::SUBSCRIPTION_SHARE_BODY_HEIGHT))
            .child(
                div()
                    .w(left_width)
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .gap(px(t::GAP))
                    .child(label(tr(cx, "选择要分享的订阅")))
                    .child(subscriptions),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(t::PAD))
                    .child(field(tr(cx, "标题"), input(0)))
                    .child(field(tr(cx, "域名或 IP"), host))
                    .child(field(tr(cx, "分享链接"), link))
                    .child(div().flex().justify_center().child(qr)),
            )
    }
    fn rules_shell(&self, cx: &App) -> Div {
        let p = SettingsColors::from_theme(cx);
        let input = |ix: usize| {
            text_input(&self.rule_inputs[ix])
                .disabled(true)
                .border_focus()
                .bg(p.solid)
                .border_color(p.line)
                .text_color(p.text)
        };
        let mut regions = div()
            .flex()
            .flex_col()
            .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP));
        for (ix, name) in RULE_REGIONS.iter().enumerate() {
            regions = regions.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
                    .child(
                        div()
                            .w(px(t::SUBSCRIPTION_REGION_DRAG))
                            .child(super::icons::icon("Bars3", t::ICON_SMALL).text_color(p.muted)),
                    )
                    .child(unavailable_select(
                        [
                            "subscription-region-us",
                            "subscription-region-hk",
                            "subscription-region-jp",
                            "subscription-region-sg",
                            "subscription-region-tw",
                            "subscription-region-kr",
                            "subscription-region-uk",
                            "subscription-region-de",
                            "subscription-region-cn",
                        ][ix],
                        name,
                        t::SUBSCRIPTION_REGION_WIDTH,
                        p,
                        cx,
                    ))
                    .child(div().flex_1().min_w_0().child(input(ix + 1)))
                    .child(
                        icon_action(
                            ("subscription-region-remove", ix),
                            "删除地区规则",
                            "XMark",
                            p,
                            cx,
                        )
                        .size(px(t::SUBSCRIPTION_REGION_REMOVE))
                        .disabled(true),
                    ),
            );
        }
        let mut tokens = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
            .flex_1()
            .child(hint(tr(cx, "拖拽排序:"), p));
        for (ix, key) in ["地区", "特征", "序号"].into_iter().enumerate() {
            tokens = tokens.child(
                gpui_kit::base::Button::new(("subscription-rule-token", ix))
                    .accessibility_label(tr(cx, key))
                    .disabled(true)
                    .h(px(t::SUBSCRIPTION_RULE_TOKEN_HEIGHT))
                    .px(px(t::GAP))
                    .border_1()
                    .border_color(p.line)
                    .rounded_full()
                    .flex()
                    .items_center()
                    .gap(px(t::ROW_GAP))
                    .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                    .child(super::icons::icon("Bars3", t::SUBSCRIPTION_LABEL_FONT).opacity(0.55))
                    .child(tr(cx, key)),
            );
        }
        let mut form = div()
            .id("subscription-rules-disabled")
            .flex()
            .flex_col()
            .gap(px(t::SECTION_PADDING))
            .tooltip(|_, cx| {
                cx.new(|cx| {
                    super::components::tooltip::TooltipContent(
                        tr(cx, "规则编辑将在后续任务开放").into(),
                    )
                })
                .into()
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::GAP))
                    .child(
                        gpui_kit::base::Switch::new("subscription-prefix")
                            .accessibility_label(tr(
                                cx,
                                "节点名前加订阅名（如「机场名称 | 香港-01」）",
                            ))
                            .checked(true)
                            .disabled(true)
                            .size(px(t::ICON_SMALL))
                            .rounded(px(t::SUBSCRIPTION_CHECKBOX_RADIUS))
                            .bg(p.text.opacity(0.2))
                            .child(
                                super::icons::icon("Check", t::SUBSCRIPTION_LABEL_FONT)
                                    .text_color(rgb(0xffffff)),
                            ),
                    )
                    .child(hint(
                        tr(cx, "节点名前加订阅名（如「机场名称 | 香港-01」）"),
                        p,
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(t::GAP))
                            .child(
                                div()
                                    .text_size(px(t::BODY))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr(cx, "重命名规则")),
                            )
                            .child(disabled_rule_switch(
                                "subscription-rule-enabled",
                                true,
                                p,
                                cx,
                            )),
                    )
                    .child(
                        button("subscription-rules-reset", "")
                            .accessibility_label(tr(cx, "重置"))
                            .tooltip(tr(cx, "规则编辑将在后续任务开放"))
                            .ghost()
                            .reference_hover(rgba(0).into(), p.button_hover(), p.text)
                            .disabled(true)
                            .h(px(t::SWITCH_HEIGHT))
                            .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
                                    .child(super::icons::icon("ArrowPath", t::BODY))
                                    .child(tr(cx, "重置")),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::GAP))
                    .child(tokens)
                    .child(hint(tr(cx, "序号位数:"), p))
                    .child(
                        input(0)
                            .w(px(t::SUBSCRIPTION_RULE_SEQUENCE_WIDTH))
                            .h(px(t::SUBSCRIPTION_RULE_SEQUENCE_HEIGHT)),
                    ),
            )
            .child(hint(tr(cx, "效果:美国-专线-01"), p).mt(-px(t::GAP)))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::GAP))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(label(tr(cx, "地区关键词")))
                            .child(
                                button("subscription-region-add", "")
                                    .accessibility_label(tr(cx, "新增"))
                                    .tooltip(tr(cx, "规则编辑将在后续任务开放"))
                                    .ghost()
                                    .reference_hover(rgba(0).into(), p.button_hover(), p.text)
                                    .disabled(true)
                                    .h(px(t::SWITCH_HEIGHT))
                                    .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
                                            .child(super::icons::icon("Plus", t::BODY))
                                            .child(tr(cx, "新增")),
                                    ),
                            ),
                    )
                    .child(hint(tr(cx, "按原始节点名匹配（不区分大小写）。"), p).mt(-px(t::GAP)))
                    .child(regions),
            );
        for (ix, key, help) in [
            (10, "无法识别地区时的标签", ""),
            (
                11,
                "特征关键词",
                "命中的关键词转成大写写进节点名,如填 iplc,ipv6,「美国 IPLC IPv6 01」变成 美国-IPLC-IPV6-01。",
            ),
            (
                12,
                "过滤节点",
                "原始节点名命中关键词的整条不导入,用来过滤「官网」「提工单」这类公告条目;清空则不过滤。",
            ),
        ] {
            form = form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::SUBSCRIPTION_EMPTY_GAP))
                    .child(label(tr(cx, key)))
                    .when(!help.is_empty(), |d| {
                        d.child(
                            hint(tr(cx, help), p).line_height(px(t::SUBSCRIPTION_RULE_HINT_LINE)),
                        )
                    })
                    .child(input(ix)),
            );
        }
        div().child(form)
    }
    fn saved_nodes_panel(
        &self,
        nodes: &[crate::subscriptions::SubscriptionNodePresentation],
        p: SettingsColors,
        cx: &App,
    ) -> Div {
        let mut tabs = div()
            .flex()
            .items_center()
            .p(px(t::ROW_GAP))
            .rounded(px(t::RADIUS))
            .bg(p.base200());
        for (id, key, count, active) in [
            (
                "subscription-saved-kept",
                "有效",
                nodes.len().to_string(),
                true,
            ),
            ("subscription-saved-filtered", "过滤", "—".into(), false),
            ("subscription-saved-disabled", "禁用", "—".into(), false),
        ] {
            tabs = tabs.child(
                editor_tab(
                    id,
                    format!("{}（{}）", tr(cx, key), count),
                    active,
                    true,
                    p,
                    cx,
                )
                .tooltip(tr(cx, "后续任务能力，当前不可用")),
            );
        }
        let action = |id, key, icon| {
            button(id, "")
                .accessibility_label(tr(cx, key))
                .disabled(true)
                .h(px(t::CONTROL))
                .px(px(t::PAD))
                .rounded(px(t::RADIUS))
                .bg(p.base200())
                .text_color(p.muted)
                .tooltip(tr(cx, "后续任务能力，当前不可用"))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
                        .text_size(px(t::BODY))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(super::icons::icon(icon, t::ICON_SMALL))
                        .child(tr(cx, key)),
                )
        };
        let mut table = div()
            .w_full()
            .flex()
            .flex_col()
            .border_1()
            .border_color(p.line)
            .rounded(px(t::SUBSCRIPTION_NODES_TABLE_RADIUS))
            .overflow_hidden()
            .child(
                div()
                    .h(px(t::SUBSCRIPTION_NODES_TABLE_HEADER))
                    .flex()
                    .items_center()
                    .bg(p.solid)
                    .child(
                        div()
                            .w(relative(t::SUBSCRIPTION_NODES_ORIGINAL_FRACTION))
                            .px(px(t::PAD))
                            .child(label(tr(cx, "原名"))),
                    )
                    .child(div().flex_1().px(px(t::PAD)).child(label(tr(cx, "新名")))),
            );
        for node in nodes {
            table = table.child(
                div()
                    .h(px(t::SUBSCRIPTION_NODES_TABLE_ROW))
                    .flex()
                    .items_center()
                    .border_t_1()
                    .border_color(p.line)
                    .child(
                        div()
                            .w(relative(t::SUBSCRIPTION_NODES_ORIGINAL_FRACTION))
                            .px(px(t::PAD))
                            .text_color(p.muted)
                            // 已保存DTO没有原始导入名，显示未知，不能拿当前名冒充它。
                            .child("—"),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .px(px(t::PAD))
                            .flex()
                            .items_center()
                            .gap(px(t::ROW_GAP))
                            .child(
                                super::icons::icon("ArrowRight", t::SUBSCRIPTION_LABEL_FONT)
                                    .text_color(p.muted.opacity(0.48)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .h(px(t::CONTROL))
                                    .px(px(t::PAD))
                                    .border_1()
                                    .border_color(p.line)
                                    .rounded(px(t::RADIUS))
                                    .bg(p.solid)
                                    .text_size(px(t::BODY))
                                    .text_color(p.text)
                                    .flex()
                                    .items_center()
                                    .child(div().truncate().child(node.name.clone())),
                            )
                            .child(
                                div()
                                    .w(px(t::SUBSCRIPTION_NODES_LATENCY_WIDTH))
                                    .text_size(px(t::SUBSCRIPTION_LABEL_FONT))
                                    .text_color(p.muted)
                                    .text_right()
                                    .child(tr(cx, "未测速")),
                            )
                            .child(
                                icon_action(
                                    SharedString::from(format!(
                                        "subscription-saved-test-{}",
                                        node.id
                                    )),
                                    "测速",
                                    "Bolt",
                                    p,
                                    cx,
                                )
                                .disabled(true),
                            )
                            .child(
                                icon_action(
                                    SharedString::from(format!(
                                        "subscription-saved-disable-{}",
                                        node.id
                                    )),
                                    "禁用",
                                    "NoSymbol",
                                    p,
                                    cx,
                                )
                                .disabled(true),
                            ),
                    ),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(t::PAD))
            .min_h(px(t::SUBSCRIPTION_NODES_PANEL_HEIGHT))
            .child(
                div()
                    .text_size(px(t::BODY))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(format!(
                        "{} {} {}",
                        tr(cx, "已保存"),
                        nodes.len(),
                        tr(cx, "个节点")
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(t::GAP))
                    .child(tabs)
                    .child(action(
                        "subscription-saved-restore",
                        "恢复默认名称",
                        "ArrowUturnLeft",
                    ))
                    .child(action("subscription-saved-test-all", "一键测速", "Bolt")),
            )
            .child(table)
    }
    fn footer(&mut self, cx: &mut Context<Self>) -> Div {
        let p = SettingsColors::from_theme(cx);
        let share = self.share_editor_open;
        let busy = !share && self.model.busy();
        let input = self.input(cx);
        let disabled = share
            || busy
            || self.editor_page != EditorPage::Source
            || input.name.trim().is_empty()
            || (self.model.editing.is_none()
                && input.sources.iter().all(|source| match source {
                    PreviewSource::Url(value) | PreviewSource::Pasted(value) => {
                        value.trim().is_empty()
                    }
                }))
            || (self.model.editing.is_none() && self.model.status == Status::Saved);
        div()
            .min_h(px(if share {
                t::SUBSCRIPTION_SHARE_FOOTER
            } else {
                t::SUBSCRIPTION_FOOTER
            }))
            .flex_shrink_0()
            .flex()
            .items_center()
            .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
            .font_weight(FontWeight::SEMIBOLD)
            .justify_end()
            .gap(px(t::GAP))
            .px(px(t::SECTION_PADDING))
            .py(px(t::GAP))
            .when(share, |d| d.pt(px(t::PAD)))
            .border_t_1()
            .border_color(p.text.opacity(0.1))
            .child(
                button("subscription-cancel", tr(cx, "取消"))
                    .reference_hover(
                        p.base200(),
                        rgb(if p.dark {
                            t::REFERENCE_GHOST_HOVER_DARK
                        } else {
                            t::REFERENCE_GHOST_HOVER_LIGHT
                        })
                        .into(),
                        p.text,
                    )
                    .disabled(busy)
                    .w(px(t::SUBSCRIPTION_FOOTER_BUTTON))
                    .px(px(t::PAD))
                    .bg(p.base200())
                    .rounded(px(t::SUBSCRIPTION_CONTROL_RADIUS))
                    .text_color(p.text)
                    .font_weight(FontWeight::SEMIBOLD)
                    .on_click(|_, w, cx| {
                        w.dispatch_action(Box::new(gpui_kit::component::dialog::Cancel), cx)
                    }),
            )
            .child(
                button("subscription-save", tr(cx, "保存"))
                    .w(px(t::SUBSCRIPTION_FOOTER_BUTTON))
                    .px(px(t::PAD))
                    .custom(
                        ButtonCustomVariant::new(cx)
                            .color(if disabled {
                                p.text.opacity(0.1)
                            } else {
                                rgb(t::PRIMARY_BUTTON_BG).into()
                            })
                            .foreground(if disabled {
                                p.text.opacity(0.2)
                            } else {
                                rgb(t::PRIMARY_BUTTON_TEXT).into()
                            })
                            .hover(rgb(t::PRIMARY_BUTTON_BG).into())
                            .active(rgb(t::PRIMARY_BUTTON_BG).into())
                            .shadow(false),
                    )
                    .bg(if disabled {
                        p.text.opacity(0.1)
                    } else {
                        rgb(t::PRIMARY_BUTTON_BG).into()
                    })
                    .text_color(if disabled {
                        p.text.opacity(0.2)
                    } else {
                        rgb(t::PRIMARY_BUTTON_TEXT).into()
                    })
                    .when(!disabled, |b| {
                        b.reference_hover(
                            rgb(t::PRIMARY_BUTTON_BG).into(),
                            rgb(t::PRIMARY_BUTTON_HOVER).into(),
                            rgb(t::PRIMARY_BUTTON_TEXT).into(),
                        )
                    })
                    .reference_focus(rgb(t::PRIMARY_BUTTON_BG).into())
                    .font_weight(FontWeight::SEMIBOLD)
                    .disabled(disabled)
                    .when(busy, |b| b.child(Spinner::new().with_size(px(t::BODY))))
                    .when(share, |b| b.tooltip(tr(cx, "订阅分享将在后续任务开放")))
                    .when(!share, |b| {
                        b.on_click(cx.listener(|this, _, window, cx| {
                            this.name.read(cx).focus_handle(cx).focus(window, cx);
                            if let Some(id) = this.model.editing.clone() {
                                let source = if this.model.manual {
                                    this.source.read(cx).value().to_string()
                                } else {
                                    this.urls
                                        .iter()
                                        .map(|i| i.read(cx).value().trim().to_owned())
                                        .filter(|v| !v.is_empty())
                                        .collect::<Vec<_>>()
                                        .join("\n")
                                };
                                this.send(
                                    Command::Edit(
                                        EditSubscription {
                                            id,
                                            name: Some(this.name.read(cx).value().to_string()),
                                            description: Some(
                                                this.description.read(cx).value().to_string(),
                                            ),
                                            url_replacement: (!this.model.manual
                                                && !source.trim().is_empty())
                                            .then(|| source.trim().into()),
                                            remote_request: None,
                                            update_policy: None,
                                        },
                                        (this.model.manual && !source.trim().is_empty())
                                            .then_some(source),
                                    ),
                                    cx,
                                );
                            } else {
                                this.send(Command::SaveDraft(this.input(cx)), cx);
                            }
                        }))
                    }),
            )
    }
}
// React defaultRenameOptions默认值只用于禁用视觉壳，绝不形成Core配置。
const RULE_REGIONS: &[&str] = &[
    "美国",
    "香港",
    "日本",
    "新加坡",
    "台湾",
    "韩国",
    "英国",
    "德国",
    "中国",
];
const RULE_VALUES: &[&str] = &[
    "2",
    "us,united states,america,美国,美國,洛杉矶,洛杉磯,硅谷,圣何塞,西雅图,纽约",
    "hk,hong kong,hongkong,香港,深港",
    "jp,japan,日本,东京,東京,大阪",
    "sg,singapore,新加坡,狮城,獅城",
    "tw,taiwan,台湾,台灣,臺灣,台北",
    "kr,korea,韩国,韓國,首尔,首爾",
    "uk,gb,united kingdom,britain,英国,英國,伦敦,倫敦",
    "de,germany,德国,德國,法兰克福,法蘭克福",
    "cn,china,中国,中國,回国,回國,back to china",
    "其他",
    "iepl,iplc,ipv6,专线,家宽,2x",
    "官网,工单,客服",
];
fn disabled_rule_switch(
    id: &'static str,
    checked: bool,
    p: SettingsColors,
    cx: &App,
) -> impl IntoElement {
    gpui_kit::base::Switch::new(id)
        .checked(checked)
        .disabled(true)
        .accessibility_label(tr(cx, "后续任务能力，当前不可用"))
        .w(px(t::SUBSCRIPTION_RULE_SWITCH_WIDTH))
        .h(px(t::SUBSCRIPTION_RULE_SWITCH_HEIGHT))
        .p(px(t::SUBSCRIPTION_SWITCH_PADDING))
        .border_1()
        .border_color(if checked {
            rgb(t::PRIMARY_BUTTON_BG).into()
        } else {
            p.muted
        })
        .rounded(px(t::SUBSCRIPTION_RULE_SWITCH_RADIUS))
        .bg(if checked {
            rgb(t::PRIMARY_BUTTON_BG).into()
        } else {
            p.surface
        })
        .child(
            div()
                .size(px(t::SUBSCRIPTION_RULE_SWITCH_THUMB))
                .rounded_full()
                .when(checked, |d| d.ml(px(t::SUBSCRIPTION_RULE_SWITCH_THUMB)))
                .bg(if checked {
                    rgb(0xffffff).into()
                } else {
                    p.muted
                }),
        )
}
fn unavailable_select(
    id: &'static str,
    key: &str,
    width: f32,
    p: SettingsColors,
    cx: &App,
) -> super::components::PanelButton {
    let flag = match key {
        "美国" => Some("🇺🇸"),
        "香港" => Some("🇭🇰"),
        "日本" => Some("🇯🇵"),
        "新加坡" => Some("🇸🇬"),
        "台湾" => Some("🇹🇼"),
        "韩国" => Some("🇰🇷"),
        "英国" => Some("🇬🇧"),
        "德国" => Some("🇩🇪"),
        "中国" => Some("🇨🇳"),
        _ => None,
    };
    button(id, "")
        .accessibility_label(tr(cx, key))
        .disabled(true)
        .w(px(width))
        .h(px(t::CONTROL))
        .bg(p.solid)
        .border_1()
        .border_color(p.line)
        .rounded(px(t::SUBSCRIPTION_CONTROL_RADIUS))
        .px(px(t::PAD))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .text_size(px(t::BODY))
                .font_weight(FontWeight::NORMAL)
                .justify_between()
                .text_size(px(t::BODY))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .text_size(px(t::BODY))
                        .font_weight(FontWeight::NORMAL)
                        .gap(px(t::SUBSCRIPTION_RULE_ROW_GAP))
                        .when_some(flag, |d, f| {
                            d.child(
                                div()
                                    .w(px(t::SUBSCRIPTION_SWITCH_HEIGHT))
                                    .text_size(px(t::BODY))
                                    .child(f),
                            )
                        })
                        .child(div().min_w_0().truncate().child(tr(cx, key).to_owned())),
                )
                .child(super::icons::icon("ChevronDown", t::BODY)),
        )
}
fn health_dots(count: usize, _p: SettingsColors, cx: &App) -> Div {
    div()
        .min_h(px(t::SUBSCRIPTION_DOTS_HEIGHT))
        .mt(px(t::SUBSCRIPTION_UPDATED_GAP))
        .pt(px(t::SUBSCRIPTION_DOTS_PT))
        .flex()
        .flex_wrap()
        .items_center()
        .gap(px(t::ROW_GAP))
        .children((0..count).map(|ix| {
            super::components::HoverSurface::new(("subscription-untested", ix))
                .size(px(t::ICON_SMALL))
                .scale(t::ICON_SMALL)
                .rounded_full()
                .bg(rgb(t::SUBSCRIPTION_UNTESTED))
                .with_tooltip(tr(cx, "未测速"))
        }))
}
/// 卡片与编辑器两个真实区域共用ProxyOption只读展示，节点事实仅来自Model::project。
fn node_grid(
    nodes: &[crate::subscriptions::SubscriptionNodePresentation],
    p: SettingsColors,
    padded: bool,
    width: Pixels,
    cx: &App,
) -> Div {
    if nodes.is_empty() {
        return div()
            .min_h(px(t::SUBSCRIPTION_NODE_EMPTY))
            .px(px(t::SECTION_PADDING))
            .pb(px(t::SECTION_PADDING))
            .flex()
            .items_center()
            .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
            .font_weight(FontWeight::SEMIBOLD)
            .justify_center()
            .text_size(px(t::SUBSCRIPTION_LOADING_FONT))
            .text_color(p.muted)
            .child(tr(cx, "暂无可显示节点"));
    }
    let available = f32::from(width) - if padded { t::SECTION_PADDING * 2. } else { 0. };
    let cols = ((available + t::GAP) / (t::SUBSCRIPTION_NODE_MIN_WIDTH + t::GAP))
        .floor()
        .max(1.) as u16;
    div()
        .w_full()
        .min_w_0()
        .grid()
        .grid_cols(cols)
        .gap(px(t::GAP))
        .when(padded, |d| {
            d.px(px(t::SECTION_PADDING)).pb(px(t::SECTION_PADDING))
        })
        .children(nodes.iter().map(|node| {
            super::components::HoverSurface::new(SharedString::from(format!(
                "subscription-node-{}",
                node.id
            )))
            .relative()
            .min_w_0()
            .h(px(t::SUBSCRIPTION_NODE_HEIGHT))
            .border_1()
            .rounded(px(t::SUBSCRIPTION_NODE_RADIUS))
            .colors(
                p.base200(),
                rgb(if p.dark {
                    t::SUBSCRIPTION_NODE_DARK_HOVER
                } else {
                    t::SUBSCRIPTION_NODE_LIGHT_HOVER
                })
                .into(),
                p.text.opacity(t::SUBSCRIPTION_NODE_DARK_BORDER_ALPHA),
                p.text.opacity(t::SUBSCRIPTION_NODE_DARK_HOVER_ALPHA),
            )
            .child(
                div()
                    .h_full()
                    .pl(px(t::GAP))
                    .pr(px(t::GAP))
                    .py(px(t::SUBSCRIPTION_NODE_PY))
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(t::SUBSCRIPTION_NODE_NAME_HEIGHT))
                            .flex()
                            .items_center()
                            .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                            .font_weight(FontWeight::SEMIBOLD)
                            .min_w_0()
                            .text_size(px(t::BODY))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(p.text)
                            .truncate()
                            .child(node.name.clone()),
                    )
                    .child(
                        hint(
                            if node.protocol == "hysteria2" {
                                "hy2".to_owned()
                            } else {
                                node.protocol.clone()
                            },
                            p,
                        )
                        .mt(px(t::GAP))
                        .pr(px(t::SUBSCRIPTION_LATENCY_WIDTH + t::GAP))
                        .text_color(p.text.opacity(t::SUBSCRIPTION_NODE_DARK_MUTED_ALPHA)),
                    ),
            )
            .child(
                super::components::icon_button_content(
                    SharedString::from(format!("subscription-node-test-{}", node.id)),
                    tr(cx, "节点测速将在 P2-08 开放"),
                    t::SUBSCRIPTION_LATENCY_HEIGHT,
                    super::icons::icon("Bolt", t::BODY),
                )
                .with_tooltip(tr(cx, "节点测速将在 P2-08 开放"))
                .absolute()
                .right(px(t::GAP))
                .bottom(px(t::SUBSCRIPTION_NODE_PY - 1.))
                .w(px(t::SUBSCRIPTION_LATENCY_WIDTH))
                .h(px(t::SUBSCRIPTION_LATENCY_HEIGHT))
                .rounded(px(t::SUBSCRIPTION_LATENCY_RADIUS))
                .bg(p.solid)
                .text_color(p.text)
                .disabled(true),
            )
        }))
}
// 颜色只来自全局 Theme；订阅页面不能维护第二份 Light/Dark palette。
#[derive(Clone, Copy)]
struct SettingsColors {
    dark: bool,
    text: Hsla,
    muted: Hsla,
    surface: Hsla,
    solid: Hsla,
    line: Hsla,
    field: Hsla,
}
impl SettingsColors {
    // 本机 OpenBox base-200；Dark复用全局Theme，Light按实际CSS灰阶。
    fn base200(self) -> Hsla {
        if self.dark {
            Hsla {
                a: 1.,
                ..self.field
            }
        } else {
            rgb(t::PANEL_BASE200_LIGHT).into()
        }
    }
    fn button_hover(self) -> Hsla {
        rgb(if self.dark {
            t::REFERENCE_GHOST_HOVER_DARK
        } else {
            t::REFERENCE_GHOST_HOVER_LIGHT
        })
        .into()
    }

    fn from_theme(cx: &App) -> Self {
        let theme = cx.theme();
        Self {
            dark: theme.mode == gpui_kit::component::ThemeMode::Dark,
            text: theme.foreground,
            muted: theme.foreground.opacity(0.5),
            surface: theme.popover,
            solid: theme.popover,
            line: theme.foreground.opacity(0.2),
            field: theme.button,
        }
    }
}
fn editor_tab(
    id: &'static str,
    label: impl Into<SharedString>,
    active: bool,
    disabled: bool,
    p: SettingsColors,
    cx: &App,
) -> super::components::PanelButton {
    let bg: Hsla = if active {
        rgb(t::SUBSCRIPTION_TAB_BG).into()
    } else {
        rgba(0).into()
    };
    let fg = if active {
        rgb(t::SUBSCRIPTION_TAB_TEXT).into()
    } else {
        p.text.opacity(0.5)
    };
    button(id, label)
        .custom(
            ButtonCustomVariant::new(cx)
                .color(bg)
                .foreground(fg)
                .hover(bg)
                .active(bg)
                .shadow(false),
        )
        .bg(bg)
        .text_color(fg)
        .min_w(px(t::SUBSCRIPTION_TAB_MIN_WIDTH))
        .h(px(t::CONTROL))
        .px(px(t::GAP))
        .rounded(px(t::SUBSCRIPTION_TAB_RADIUS))
        .font_weight(FontWeight::SEMIBOLD)
        .disabled(disabled)
}
fn label(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(t::SUBSCRIPTION_LABEL_FONT))
        .line_height(px(t::SUBSCRIPTION_LABEL_LINE))
        .font_weight(FontWeight::MEDIUM)
        .child(text.into())
}
fn field(text: impl Into<SharedString>, input: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(px(t::ROW_GAP))
        .child(label(text))
        .child(input)
}
fn hint(text: impl Into<SharedString>, p: SettingsColors) -> Div {
    div()
        .text_size(px(t::SUBSCRIPTION_LABEL_FONT))
        .line_height(px(t::SUBSCRIPTION_LABEL_LINE))
        .text_color(p.muted)
        .child(text.into())
}
fn notice_box(color: u32) -> Div {
    div()
        .px(px(t::SUBSCRIPTION_NOTICE_PX))
        .py(px(t::SUBSCRIPTION_NOTICE_PY))
        .rounded(px(t::RADIUS))
        .border_1()
        .border_color(rgb(color).opacity(0.35))
        .bg(rgb(color).opacity(0.08))
        .text_color(rgb(color))
        .text_size(px(t::SUBSCRIPTION_LABEL_FONT))
        .line_height(px(t::BODY_LINE))
        .flex()
        .flex_col()
        .gap(px(t::SUBSCRIPTION_SKIPPED_GAP))
}
fn notice(key: &'static str, color: u32, cx: &App) -> Div {
    notice_box(color).child(tr(cx, key))
}
fn icon_action(
    id: impl Into<ElementId>,
    key: &'static str,
    glyph: &'static str,
    p: SettingsColors,
    cx: &App,
) -> super::components::IconButton {
    icon_action_busy(id, key, glyph, p, cx, false)
}
fn icon_action_busy(
    id: impl Into<ElementId>,
    key: &'static str,
    glyph: &'static str,
    p: SettingsColors,
    cx: &App,
    spinning: bool,
) -> super::components::IconButton {
    let content = if spinning {
        Spinner::new()
            .icon(super::icons::icon(glyph, t::ICON_SMALL))
            .with_size(px(t::ICON_SMALL))
            .into_any_element()
    } else {
        super::icons::icon(glyph, t::ICON_SMALL).into_any_element()
    };
    super::components::icon_button_content(id, tr(cx, key), t::CONTROL, content)
        .with_tooltip(tr(cx, key))
        .bg(rgba(0))
        .text_color(p.text)
        .openbox_ghost(p.button_hover(), p.text)
}
impl Render for SubscriptionsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = Instant::now();
        self.collapse_motion.retain(|_, motion| {
            !cx.reduce_motion()
                && now.duration_since(motion.started)
                    < Duration::from_millis(t::SUBSCRIPTION_COLLAPSE_MS)
        });
        if !self.collapse_motion.is_empty() {
            window.request_animation_frame();
        }
        let p = SettingsColors::from_theme(cx);
        let mut page = div()
            .flex()
            .flex_col()
            .w_full()
            .min_w_0()
            .gap(px(t::GAP))
            .text_color(p.text)
            .text_size(px(t::BODY))
            .line_height(px(t::BODY_LINE))
            .on_children_prepainted({
                let view = cx.entity().downgrade();
                move |bounds, _, cx| {
                    if let Some(width) = bounds
                        .iter()
                        .map(|b| b.size.width)
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                    {
                        // 使用实际内容宽度计算auto-fill，Sidebar/窗口变化不修改业务代次。
                        let view = view.clone();
                        cx.defer(move |cx| {
                            let _ = view.update(cx, |this, cx| {
                                if this.page_width != width {
                                    this.page_width = width;
                                    cx.notify();
                                }
                            });
                        });
                    }
                }
            });
        if self.loading && self.model.busy() && !self.model.editor_open {
            return page.child(
                div()
                    .min_h(px(t::SUBSCRIPTION_EMPTY_HEIGHT))
                    .rounded(cx.theme().radius_lg)
                    .bg(p.surface)
                    .flex()
                    .items_center()
                    .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                    .font_weight(FontWeight::SEMIBOLD)
                    .justify_center()
                    .gap(px(t::GAP))
                    .text_color(p.muted)
                    .child(Spinner::new().with_size(px(t::BODY)))
                    .child(
                        div()
                            .text_size(px(t::SUBSCRIPTION_LOADING_FONT))
                            .font_weight(FontWeight::MEDIUM)
                            .child(tr(cx, "正在加载订阅数据")),
                    ),
            );
        }
        if self.load_error
            && !self.model.editor_open
            && let Status::Error(error) = self.model.status
        {
            page = page.child(
                div()
                    .w_full()
                    .max_w(px(t::SUBSCRIPTION_ERROR_WIDTH))
                    .mx_auto()
                    .mb(px(t::PAD))
                    .p(px(t::SECTION_PADDING))
                    .rounded(cx.theme().radius_lg)
                    .bg(p.surface)
                    .flex()
                    .items_start()
                    .gap(px(t::SUBSCRIPTION_ERROR_GAP))
                    .child(
                        super::icons::icon("ExclamationTriangle", t::SUBSCRIPTION_EMPTY_ICON)
                            .text_color(rgb(t::SUBSCRIPTION_ERROR)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(t::BODY))
                                    .font_weight(FontWeight::BOLD)
                                    .child(tr(cx, "订阅数据加载失败")),
                            )
                            .child(
                                div()
                                    .mt(px(t::SUBSCRIPTION_SKIPPED_GAP))
                                    .text_size(px(t::SUBSCRIPTION_TEXT_BUTTON))
                                    .text_color(p.muted)
                                    .child(tr(cx, error_key(error))),
                            ),
                    )
                    .child(
                        button("subscription-retry", tr(cx, "重新加载"))
                            .primary()
                            .on_click(cx.listener(|this, _, _, cx| this.send(Command::Load, cx))),
                    ),
            );
        }
        page = page.child(self.share_panel(p, cx));
        if self.model.list.is_empty() && !matches!(self.model.status, Status::Error(_)) {
            page = page.child(
                div()
                    .min_h(px(t::SUBSCRIPTION_EMPTY_HEIGHT))
                    .w_full()
                    .rounded(cx.theme().radius_lg)
                    .bg(p.surface)
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(t::SUBSCRIPTION_EMPTY_GAP))
                    .child(
                        super::icons::icon("Link", t::SUBSCRIPTION_EMPTY_ICON)
                            .text_color(rgb(t::ACCENT_STRONG)),
                    )
                    .child(
                        div()
                            .text_size(px(t::BODY))
                            .font_weight(FontWeight::BOLD)
                            .child(tr(cx, "暂无订阅")),
                    )
                    .child(hint(tr(cx, "点右上角「添加」创建"), p)),
            );
        }
        for item in self.model.list.clone() {
            let id = item.id.clone();
            let is_collapsed = self.collapsed.contains(&id);
            let refresh_id = id.clone();
            let edit_id = id.clone();
            let delete_id = id.clone();
            let manual = item.source_kind == SubscriptionSourceKind::Manual;
            let refreshing = self.model.busy() && self.refreshing.as_ref() == Some(&id);
            let updated = if let Some(saved_at) = item.last_success_at_ms {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("system epoch")
                    .as_millis() as u64;
                let elapsed = now.saturating_sub(saved_at);
                let relative = if elapsed < 60_000 {
                    tr(cx, "几秒前").to_owned()
                } else if elapsed < 3_600_000 {
                    format!("{} {}", elapsed / 60_000, tr(cx, "分钟前"))
                } else if elapsed < 86_400_000 {
                    format!("{} {}", elapsed / 3_600_000, tr(cx, "小时前"))
                } else {
                    format!("{} {}", elapsed / 86_400_000, tr(cx, "天前"))
                };
                format!("{} {}", tr(cx, "更新于"), relative)
            } else {
                tr(cx, "尚未更新").to_owned()
            };
            let nodes: Vec<_> = self
                .model
                .nodes
                .iter()
                .filter(|n| n.subscription_id == item.id)
                .cloned()
                .collect();
            let toggle = gpui_kit::base::Button::new(SharedString::from(format!(
                "subscription-toggle-{id}"
            )))
            .accessibility_label(format!(
                "{} {}",
                tr(cx, if is_collapsed { "展开" } else { "收起" }),
                item.name
            ))
            .flex_1()
            .min_w_0()
            .p_0()
            .flex()
            .flex_col()
            .items_start()
            .text_color(p.text)
            .on_click(cx.listener(move |this, _, _, cx| {
                let now = Instant::now();
                let target = if this.collapsed.contains(&id) { 0. } else { 1. };
                let from = this
                    .collapse_motion
                    .get(&id)
                    .map_or((target, target), |motion| motion.value(now));
                this.collapse_motion.insert(
                    id.clone(),
                    CollapseMotion {
                        from,
                        to: 1. - target,
                        started: now,
                    },
                );
                if !this.collapsed.remove(&id) {
                    this.collapsed.insert(id.clone());
                }
                cx.notify();
            }))
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .w_full()
                    .h(px(t::SECTION_LINE))
                    .line_height(px(t::SECTION_LINE))
                    .items_center()
                    .gap(px(t::SUBSCRIPTION_UPDATED_GAP))
                    .child(super::icons::icon("Bars3", t::ICON_SMALL).text_color(p.muted))
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(px(t::SECTION_TITLE))
                            .font_weight(FontWeight::MEDIUM)
                            .child(item.name),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(t::SUBSCRIPTION_LABEL_FONT))
                            .text_color(p.muted)
                            .child(format!("(—/{})", item.node_count)),
                    ),
            )
            .child(
                hint(updated.to_owned(), p)
                    .line_height(px(t::SUBSCRIPTION_LABEL_LINE))
                    .mt(px(t::ROW_GAP)),
            )
            .when(is_collapsed, |d| d.child(health_dots(nodes.len(), p, cx)))
            .when(!item.description.is_empty(), |d| {
                let description = item.description.clone();
                d.tooltip(move |_, cx| {
                    cx.new(|_| {
                        super::components::tooltip::TooltipContent(description.clone().into())
                    })
                    .into()
                })
            });
            let target = if is_collapsed { 0. } else { 1. };
            let (height, opacity) = self
                .collapse_motion
                .get(&item.id)
                .map_or((target, target), |motion| motion.value(now));
            let cols = ((f32::from(self.page_width) - t::SECTION_PADDING * 2. + t::GAP)
                / (t::SUBSCRIPTION_NODE_MIN_WIDTH + t::GAP))
                .floor()
                .max(1.) as usize;
            let rows = nodes.len().div_ceil(cols);
            let body_height = if rows == 0 {
                t::SUBSCRIPTION_NODE_EMPTY
            } else {
                rows as f32 * t::SUBSCRIPTION_NODE_HEIGHT
                    + rows.saturating_sub(1) as f32 * t::GAP
                    + t::SECTION_PADDING
            };
            let actions = div()
                .flex()
                .flex_shrink_0()
                .gap(px(t::GAP))
                .child(
                    icon_action(
                        "subscription-power",
                        "启用/停用将在后续 Runtime 任务开放",
                        "Power",
                        p,
                        cx,
                    )
                    .disabled(true)
                    .opacity(0.5),
                )
                .child(
                    icon_action(
                        "subscription-test",
                        "节点测速将在 P2-08 开放",
                        "Bolt",
                        p,
                        cx,
                    )
                    .disabled(true)
                    .opacity(0.5),
                )
                .child(
                    icon_action_busy(
                        "subscription-refresh",
                        "刷新",
                        "ArrowPath",
                        p,
                        cx,
                        refreshing,
                    )
                    .disabled(self.model.busy())
                    .when(self.model.busy(), |b| b.opacity(0.5))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if manual {
                            this.open_editor(Some(refresh_id.clone()), window, cx);
                        } else {
                            this.send(
                                Command::Refresh {
                                    id: refresh_id.clone(),
                                    content: None,
                                },
                                cx,
                            );
                        }
                    })),
                )
                .child(
                    icon_action("subscription-edit", "修改订阅", "PencilSquare", p, cx)
                        .disabled(self.model.busy())
                        .when(self.model.busy(), |b| b.opacity(0.5))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_editor(Some(edit_id.clone()), window, cx)
                        })),
                )
                .child(
                    icon_action("subscription-delete", "删除", "Trash", p, cx)
                        .disabled(self.model.busy())
                        .when(self.model.busy(), |b| b.opacity(0.5))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.send(Command::Delete(delete_id.clone()), cx)
                        })),
                );
            page = page.child(
                div()
                    .id(SharedString::from(item.id))
                    .w_full()
                    .min_w_0()
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .min_h(px(t::SUBSCRIPTION_CARD_HEIGHT))
                    .rounded(cx.theme().radius_lg)
                    .bg(p.surface)
                    .child(
                        div()
                            .min_h(px(t::SUBSCRIPTION_CARD_HEIGHT))
                            .p(px(t::SECTION_PADDING))
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap(px(t::SECTION_PADDING))
                            .child(toggle)
                            .child(actions),
                    )
                    .when(height > 0., |d| {
                        d.child(div().h(px(body_height * height)).overflow_hidden().child(
                            node_grid(&nodes, p, true, self.page_width, cx).opacity(opacity),
                        ))
                    }),
            );
        }
        // 弹窗挂在固定视口层，不能作为可滚动长列表的最后一个child被裁切。
        let mut root = div()
            .relative()
            .size_full()
            .min_h_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .id("subscription-page-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.page_scroll)
                    .child(page),
            );
        if self.model.editor_open || self.share_editor_open {
            root = root.child(self.render_modal(window, cx));
        }
        root
    }
}
