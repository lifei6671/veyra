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
    base::{Scrollbar, ScrollbarMode},
    component::{
        ActiveTheme, Disableable, IndexPath,
        button::ButtonVariants,
        input::{InputEvent, InputState},
        scroll::ScrollableElement,
    },
    prelude::*,
    *,
};
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, Weak},
};
use veyra_core::application::rule_import::{RuleImportError, RuleImportKind, RuleImportPreview};
use veyra_core::domain::*;

#[derive(Clone)]
struct RuleDrag(Entity<InputState>);
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
struct RuleInput {
    kind: Entity<SelectState>,
    value: Entity<InputState>,
    note: Entity<InputState>,
    _blur: gpui_kit::Subscription,
    _note_length: gpui_kit::Subscription,
}
pub struct RoutingView {
    services: Weak<crate::services::AppServices>,
    state: Option<Box<AppState>>,
    pub busy: bool,
    error: Option<AppError>,
    draft: Option<TargetRoutingPolicy>,
    kind: EditorKind,
    name: Entity<InputState>,
    scale: Entity<InputState>,
    import_url: Entity<InputState>,
    import_open: bool,
    import_busy: bool,
    import_preview: Option<RuleImportPreview>,
    import_cancel: Option<tokio::sync::watch::Sender<bool>>,
    import_generation: u64,
    import_focus: FocusHandle,
    import_scroll: ScrollHandle,
    picker: Entity<IconPicker>,
    rules: Vec<RuleInput>,
    focus: FocusHandle,
    restore_focus: Option<FocusHandle>,
    close_after_save: bool,
    restore_open: bool,
    scroll: ScrollHandle,
    editor_scroll: ScrollHandle,
    _picker: gpui_kit::Subscription,
    _import_enter: gpui_kit::Subscription,
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
        let scale = cx.new(|cx| InputState::new(window, cx));
        let import_url = cx.new(|cx| InputState::new(window, cx));
        // 与原版 URL 输入一致：Enter 只预览当前草稿，不提交外层配置。
        let import_enter = cx.subscribe(&import_url, |view, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::PressEnter { .. })
                && view.import_open
                && !view.import_busy
            {
                view.preview_import(cx);
            }
        });
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
            scale,
            import_url,
            import_open: false,
            import_busy: false,
            import_preview: None,
            import_cancel: None,
            import_generation: 0,
            import_focus: cx.focus_handle(),
            import_scroll: ScrollHandle::new(),
            picker,
            rules: vec![],
            focus: cx.focus_handle(),
            restore_focus: None,
            close_after_save: false,
            restore_open: false,
            scroll: ScrollHandle::new(),
            editor_scroll: ScrollHandle::new(),
            _picker: subscription,
            _import_enter: import_enter,
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
        self.scale.update(cx, |input, cx| {
            input.set_value(policy.icon_scale.to_string(), window, cx)
        });
        self.picker
            .update(cx, |picker, _| picker.value = policy.icon.clone());
        self.rules.clear();
        for matcher in &policy.matchers {
            // React policyRuleRows：每个域名/规则值单独一行，不能合并为逗号字符串。
            for (kind, value) in matcher_rows(matcher) {
                let note = policy
                    .notes
                    .get(&rule_note_key(kind, &value))
                    .cloned()
                    .unwrap_or_default();
                let row = self.rule_input(kind, value, note, window, cx);
                self.rules.push(row);
            }
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
        if self.import_open {
            self.close_import(window, cx);
            return;
        }
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
        let rule = self.rule_input(kind, value, String::new(), window, cx);
        self.rules.push(rule);
        cx.notify();
    }
    fn rule_input(
        &mut self,
        kind: usize,
        value: String,
        initial_note: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> RuleInput {
        let value = cx.new(|cx| InputState::new(window, cx).default_value(value));
        let blur = cx.subscribe_in(&value, window, |view, input, event, window, cx| {
            if matches!(event, InputEvent::Blur) && !view.busy {
                // 和 React RuleValue.onBlur 一样：粘贴多值后在原位置展开，保留其它行。
                let Some(index) = view.rules.iter().position(|row| row.value == *input) else {
                    return;
                };
                let kind = view.rules[index].kind.read(cx).selected_value();
                if !matches!(kind, "域名" | "域名后缀" | "域名关键词" | "IP 段") {
                    return;
                }
                let Some(kind_index) = RULES.iter().position(|label| *label == kind) else {
                    return;
                };
                let values = split_rule_values(kind, &input.read(cx).value());
                if values.len() <= 1 {
                    return;
                }
                input.update(cx, |input, cx| {
                    input.set_value(values[0].clone(), window, cx)
                });
                let rows = values
                    .into_iter()
                    .skip(1)
                    .map(|value| view.rule_input(kind_index, value, String::new(), window, cx))
                    .collect::<Vec<_>>();
                view.rules.splice(index + 1..index + 1, rows);
                cx.notify();
            }
        });
        // 在输入引擎记录历史前限制本次插入；拒绝的超长文本不进入 Undo 栈。
        let previous_note = Rc::new(RefCell::new(initial_note.clone()));
        let note = cx.new(|cx| InputState::new(window, cx).default_value(initial_note));
        let previous = previous_note.clone();
        let weak_note = note.downgrade();
        let handle = window.window_handle();
        note.update(cx, |note, cx| {
            note.set_validator(
                move |text, cx| {
                    if text.encode_utf16().count() <= 60 {
                        return true;
                    }
                    let previous = previous.borrow().clone();
                    // 旧配置可包含超长备注；允许用户缩短它，不在打开时改写既有内容。
                    if text.encode_utf16().count() < previous.encode_utf16().count() {
                        return true;
                    }
                    let accepted = limit_note_edit(&previous, text);
                    if accepted != previous {
                        let weak_note = weak_note.clone();
                        // Validator 正在借用 Input，延后应用一次有效的原子编辑，沿用它的Undo。
                        cx.defer(move |cx| {
                            let Some(note) = weak_note.upgrade() else {
                                return;
                            };
                            let _ = handle.update(cx, |_, window, cx| {
                                if note.read(cx).value().as_str() == previous {
                                    let range = note.read(cx).selected_range();
                                    let end = accepted.len() - (previous.len() - range.end);
                                    let inserted = accepted[range.start..end].to_owned();
                                    note.update(cx, |note, cx| {
                                        // 只替换真实选区，Kit 同次记录正文与光标的 Undo/Redo。
                                        note.replace(inserted, window, cx);
                                    });
                                }
                            });
                        });
                    }
                    false
                },
                cx,
            )
        });
        let note_length = cx.subscribe(&note, move |_, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                *previous_note.borrow_mut() = input.read(cx).value().to_string();
            }
        });
        RuleInput {
            kind: cx
                .new(|cx| SelectState::new(RULES.to_vec(), Some(IndexPath::new(kind)), window, cx)),
            value,
            note,
            _blur: blur,
            _note_length: note_length,
        }
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
        policy.notes.clear();
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
                let kind = RULES.iter().position(|label| *label == kind).unwrap();
                // React notes 按类型和值保存；拆分批量输入时只有第一行保留备注。
                let note = row.note.read(cx).value();
                if !note.trim().is_empty() {
                    let value = matcher_rows(&matcher).into_iter().next().unwrap().1;
                    policy
                        .notes
                        .insert(rule_note_key(kind, &value), note.trim().into());
                }
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
    fn open_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.import_open = true;
        self.import_busy = false;
        self.import_preview = None;
        self.import_url
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.import_focus.focus(window, cx);
        cx.notify();
    }
    fn close_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(cancel) = self.import_cancel.take() {
            let _ = cancel.send(true);
        }
        self.import_generation += 1;
        self.import_open = false;
        self.import_busy = false;
        self.import_preview = None;
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn preview_import(&mut self, cx: &mut Context<Self>) {
        let url = self.import_url.read(cx).value().trim().to_owned();
        if self.import_busy || url.is_empty() {
            return;
        }
        let Some(services) = self.services.upgrade() else {
            return;
        };
        let (cancel, receiver) = tokio::sync::watch::channel(false);
        self.import_cancel = Some(cancel);
        self.import_busy = true;
        self.import_preview = None;
        self.import_generation += 1;
        let generation = self.import_generation;
        let receiver = services.preview_routing_rules(url, receiver);
        cx.spawn(async move |view, cx| {
            let result = receiver.await.unwrap_or(Err(RuleImportError::Io));
            let _ = view.update(cx, |view, cx| {
                // 关闭或重新预览后的旧响应不能覆盖当前弹窗，也不能写外层草稿。
                if !view.import_open || view.import_generation != generation {
                    return;
                }
                view.import_busy = false;
                view.import_cancel = None;
                match result {
                    Ok(preview) => view.import_preview = Some(preview),
                    Err(error) => components::notice::notify_app(
                        components::Notice::Error,
                        import_error_message(error),
                        cx,
                    ),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    fn confirm_import(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(preview) = self.import_preview.take() else {
            return;
        };
        if self.import_busy || preview.truncated || preview.entries.is_empty() {
            self.import_preview = Some(preview);
            return;
        }
        // 与React importRuleRows一致：丢掉空白占位行，只追加受支持的真实明细；外层保存才CAS。
        self.rules
            .retain(|row| !row.value.read(cx).value().trim().is_empty());
        for entry in preview.entries {
            let kind = match entry.kind {
                RuleImportKind::Domain => 0,
                RuleImportKind::DomainSuffix => 1,
                RuleImportKind::DomainKeyword => 6,
                RuleImportKind::IpCidr => 3,
            };
            self.add_rule(kind, entry.value, window, cx);
        }
        self.close_import(window, cx);
    }
    fn import_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.import_url.update(cx, |input, cx| {
            input.set_placeholder(
                tr(cx, "https://例子/rules.list · 支持 .list / .mrs / .srs"),
                window,
                cx,
            )
        });
        let busy = self.import_busy;
        let mut content=div().w_full().flex().flex_col().gap(px(t::PAD))
            .child(div().text_size(px(t::SMALL)).line_height(px(rt::LABEL_LINE)).text_color(cx.theme().foreground.opacity(0.6))
                .child(tr(cx,"输入规则集地址（支持 .list / .mrs / .srs），预览后会把域名和 IP 明细追加到当前站点集，已有规则会保留。只支持导入 200 条以内；超过 200 条不能导入明细。")))
            .child(div().w_full().flex().items_center().gap(px(t::GAP))
                .child(div().flex_1().min_w_0().child(components::text_input(&self.import_url).outlined_focus(cx.theme().foreground).w_full().readonly(busy)
                    .font_family(rt::RULE_FONT).rounded(px(t::ICON_PICKER_RADIUS)).bg(cx.theme().popover.opacity(rt::FIELD_OPACITY))))
                .child(button("routing-import-preview",tr(cx,if busy {"正在预览…"}else{"预览"})).primary()
                    .disabled(busy || self.import_url.read(cx).value().trim().is_empty())
                    .on_click(cx.listener(|view,_,_,cx|view.preview_import(cx)))));
        if let Some(preview) = &self.import_preview {
            let count = tr(
                cx,
                if preview.truncated {
                    "共 {count} 条，超过 200 条不能导入明细。"
                } else {
                    "共 {count} 条可导入规则"
                },
            )
            .replace("{count}", &preview.total.to_string());
            content = content
                .child(
                    div()
                        .text_size(px(t::SMALL))
                        .line_height(px(rt::LABEL_LINE))
                        .when(preview.truncated, |d| d.text_color(cx.theme().danger))
                        .child(count),
                )
                .child(
                    div()
                        .id("routing-import-list")
                        .track_scroll(&self.import_scroll)
                        .max_h(px(rt::IMPORT_LIST_HEIGHT))
                        .overflow_y_scroll()
                        .vertical_scrollbar(&self.import_scroll)
                        .children(preview.entries.iter().enumerate().map(|(index, entry)| {
                            let label = match entry.kind {
                                RuleImportKind::Domain => "域名",
                                RuleImportKind::DomainSuffix => "域名后缀",
                                RuleImportKind::DomainKeyword => "域名关键词",
                                RuleImportKind::IpCidr => "IP 段",
                            };
                            div()
                                .id(("routing-import-entry", index))
                                .w_full()
                                .h(px(t::CONTROL))
                                .flex()
                                .items_center()
                                .gap(px(t::GAP))
                                .text_size(px(t::SMALL))
                                .child(div().w(px(rt::IMPORT_KIND_WIDTH)).child(tr(cx, label)))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .font_family(rt::RULE_FONT)
                                        .child(entry.value.clone()),
                                )
                        })),
                );
        }
        let confirm = self
            .import_preview
            .as_ref()
            .is_some_and(|preview| !preview.truncated && !preview.entries.is_empty());
        let surface = div()
            .w(px(rt::FALLBACK_EDITOR_WIDTH))
            .max_w(window.viewport_size().width - px(t::SECTION_PADDING * 2.))
            .max_h(window.viewport_size().height * rt::EDITOR_HEIGHT_RATIO)
            .rounded(px(rt::EDITOR_RADIUS))
            .overflow_hidden()
            .font(t::openbox_font())
            .font_weight(t::FONT_REGULAR)
            .flex()
            .flex_col()
            .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY))
            // 与原版 --panel-background-blur 同源；只在目标分流自绘弹窗启用。
            .backdrop_blur(px(self
                .state
                .as_ref()
                .map_or(0., |state| state.app_config.visual.background_blur as f32)))
            .child(components::modal_header(
                tr(cx, "导入规则"),
                icon_button("routing-import-close", tr(cx, "关闭"), "XMark", pt::TAB)
                    .on_click(cx.listener(|view, _, window, cx| view.close_import(window, cx))),
                cx,
            ))
            .child(
                div()
                    .w_full()
                    .min_h_0()
                    .p(px(t::SECTION_PADDING))
                    .child(content),
            )
            .child(
                div()
                    .w_full()
                    .h(px(rt::FOOTER_HEIGHT))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(t::GAP))
                    .px(px(t::SECTION_PADDING))
                    .border_t_1()
                    .border_color(cx.theme().foreground.opacity(0.1))
                    .child(
                        button("routing-import-cancel", tr(cx, "取消"))
                            .w(px(rt::FOOTER_BUTTON_WIDTH))
                            .on_click(
                                cx.listener(|view, _, window, cx| view.close_import(window, cx)),
                            ),
                    )
                    .when(self.import_preview.is_some(), |footer| {
                        footer.child(
                            button("routing-import-confirm", tr(cx, "导入明细"))
                                .primary()
                                .disabled(busy || !confirm)
                                .on_click(cx.listener(|view, _, window, cx| {
                                    view.confirm_import(window, cx)
                                })),
                        )
                    }),
            );
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.import_focus.clone())
            .on_ok(|_, _, _| false)
            .close_on_escape(true)
            .close_on_backdrop_press(true)
            .backdrop(div().size_full().bg(rgba(0x00000066)))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|view, _, window, cx| view.close_import(window, cx)))
            .into_any_element()
    }
    fn editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let draft = self.draft.as_ref().unwrap().clone();
        let busy = self.busy;
        self.picker.update(cx, |picker, _| picker.disabled = busy);
        self.scale.update(cx, |input, cx| {
            input.set_value(draft.icon_scale.to_string(), window, cx)
        });
        for row in &self.rules {
            row.note.update(cx, |input, cx| {
                input.set_placeholder(tr(cx, "备注"), window, cx)
            });
        }
        let field = |label: &'static str, control: AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(px(t::ROW_GAP))
                .child(
                    div()
                        .text_size(px(t::SMALL))
                        .line_height(px(rt::LABEL_LINE))
                        .font_weight(t::FONT_MEDIUM)
                        .child(tr(cx, label)),
                )
                .child(control)
        };
        // 与原版 join 相同：30 / 48 / 30 / 52，接缝重叠1px，读数为真实readonly输入。
        let scale_bg = if cx.theme().mode.is_dark() {
            cx.theme().button
        } else {
            rgb(rt::SCALE_BUTTON_LIGHT).into()
        };
        let scale = div()
            .flex()
            .h(px(t::CONTROL))
            .flex_shrink_0()
            .on_children_prepainted(|bounds, _, _| {
                #[cfg(debug_assertions)]
                components::log_layout("routing-scale", &bounds);
            })
            .child(
                components::icon_button_content(
                    "routing-scale-down",
                    tr(cx, "缩小 1px"),
                    rt::SCALE_STEP_WIDTH,
                    super::icons::icon("Minus", t::BODY),
                )
                .h(px(t::CONTROL))
                .flex_shrink_0()
                .bg(scale_bg)
                .rounded_none()
                .rounded_l(px(t::RADIUS))
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
                    .flex_shrink_0()
                    .ml(px(-1.))
                    .child(
                        components::text_input(&self.scale)
                            .outlined_focus(cx.theme().foreground)
                            .readonly(true)
                            .font_family(rt::RULE_FONT)
                            .text_center()
                            .px_0()
                            .py_0()
                            .rounded_none()
                            .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY)),
                    ),
            )
            .child(
                components::icon_button_content(
                    "routing-scale-up",
                    tr(cx, "加大 1px"),
                    rt::SCALE_STEP_WIDTH,
                    super::icons::icon("Plus", t::BODY),
                )
                .ml(px(-1.))
                .h(px(t::CONTROL))
                .flex_shrink_0()
                .bg(scale_bg)
                .rounded_none()
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
                    .w(px(rt::SCALE_RESET_WIDTH))
                    .bg(if busy || draft.icon_scale == 0 {
                        cx.theme().foreground.opacity(0.1)
                    } else {
                        scale_bg
                    })
                    .text_color(if busy || draft.icon_scale == 0 {
                        cx.theme().foreground.opacity(0.2)
                    } else {
                        cx.theme().foreground
                    })
                    .rounded_none()
                    .rounded_r(px(t::RADIUS))
                    .px_0()
                    .disabled(busy || draft.icon_scale == 0)
                    .on_click(cx.listener(|view, _, _, cx| {
                        if let Some(draft) = &mut view.draft {
                            draft.icon_scale = 0;
                        }
                        cx.notify();
                    })),
            );
        let mut form = div()
            .w_full()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(px(t::SECTION_PADDING))
            .on_children_prepainted(|bounds, _, _| {
                #[cfg(debug_assertions)]
                components::log_layout("routing-form", &bounds);
            })
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_end()
                    .gap(px(t::GAP))
                    .on_children_prepainted(|bounds, _, _| {
                        #[cfg(debug_assertions)]
                        components::log_layout("routing-metadata", &bounds);
                    })
                    .child(field("图标", self.picker.clone().into_any_element()))
                    .child(
                        field("缩放", scale.into_any_element())
                            .w(px(rt::SCALE_WIDTH))
                            .flex_shrink_0(),
                    )
                    .child(
                        div().flex_1().min_w_0().child(field(
                            "站点集名称",
                            components::text_input(&self.name)
                                .outlined_focus(cx.theme().foreground)
                                .readonly(busy)
                                .w_full()
                                .border_color(cx.theme().foreground.opacity(0.2))
                                .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY))
                                .into_any_element(),
                        )),
                    ),
            );
        if self.kind == EditorKind::Policy {
            // 仅按原版字段顺序投影编辑行；不改持久Matcher次序或匹配契约。
            let mut displayed = self.rules.iter().enumerate().collect::<Vec<_>>();
            displayed.sort_by_key(|(_, row)| {
                let kind = RULES
                    .iter()
                    .position(|label| *label == row.kind.read(cx).selected_value())
                    .unwrap();
                [7, 1, 0, 6, 3, 2, 4, 5]
                    .iter()
                    .position(|value| *value == kind)
                    .unwrap()
            });
            let rules = div()
                .w_full()
                .flex()
                .flex_col()
                .flex_shrink_0()
                .gap(px(rt::RULE_GAP))
                .on_children_prepainted(|bounds, _, _| {
                    #[cfg(debug_assertions)]
                    components::log_layout("routing-rules", &bounds);
                })
                .child(
                    div()
                        .w_full()
                        .h(px(rt::TOOLBAR_HEIGHT))
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(t::SMALL))
                                .font_weight(t::FONT_MEDIUM)
                                .line_height(px(rt::LABEL_LINE))
                                .child(tr(cx, "规则")),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(t::ROW_GAP))
                                .child(
                                    button("routing-import-rule", "")
                                        .accessibility_label(tr(cx, "导入规则"))
                                        .child(
                                            div()
                                                .text_size(px(rt::TOOLBAR_TEXT))
                                                .line_height(px(rt::TOOLBAR_LINE))
                                                .font_weight(t::FONT_SEMIBOLD)
                                                .text_color(cx.theme().foreground)
                                                .child(tr(cx, "导入规则")),
                                        )
                                        .ghost()
                                        .h(px(rt::TOOLBAR_HEIGHT))
                                        .text_size(px(rt::TOOLBAR_TEXT))
                                        .px(px(t::GAP))
                                        .disabled(busy)
                                        .on_click(cx.listener(|view, _, window, cx| {
                                            view.open_import(window, cx)
                                        })),
                                )
                                .child(
                                    button("routing-add-rule", "")
                                        .accessibility_label(tr(cx, "添加规则"))
                                        .ghost()
                                        .h(px(rt::TOOLBAR_HEIGHT))
                                        .text_size(px(rt::TOOLBAR_TEXT))
                                        .px(px(t::GAP))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap(px(rt::TOOLBAR_ICON_GAP))
                                                .text_size(px(rt::TOOLBAR_TEXT))
                                                .line_height(px(rt::TOOLBAR_LINE))
                                                .font_weight(t::FONT_SEMIBOLD)
                                                .text_color(cx.theme().foreground)
                                                .on_children_prepainted(|bounds, _, _| {
                                                    #[cfg(debug_assertions)]
                                                    components::log_layout(
                                                        "routing-add-rule-content",
                                                        &bounds,
                                                    );
                                                })
                                                .child(super::icons::icon("Plus", t::BODY))
                                                .child(tr(cx, "添加规则")),
                                        )
                                        .disabled(busy)
                                        .on_click(cx.listener(|view, _, window, cx| {
                                            view.add_rule(1, String::new(), window, cx)
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .gap(px(rt::RULE_GAP))
                        .children(displayed.into_iter().map(|(i, row)| {
                            let dragged = row.value.clone();
                            let target = row.value.clone();
                            let deleted = row.value.clone();
                            div()
                                .w_full()
                                .h(px(t::CONTROL))
                                .flex_shrink_0()
                                .on_children_prepainted(move |bounds, _, _| {
                                    #[cfg(debug_assertions)]
                                    components::log_layout(&format!("routing-rule-{i}"), &bounds);
                                })
                                .id(("routing-rule-row", row.value.entity_id()))
                                .when(!busy, |d| {
                                    d.on_drag(RuleDrag(dragged), |drag, _, _, cx| {
                                        cx.new(|_| drag.clone())
                                    })
                                    .on_drop(cx.listener(
                                        move |view, drag: &RuleDrag, _, cx| {
                                            if !view.busy
                                                && let Some(from) = view
                                                    .rules
                                                    .iter()
                                                    .position(|row| row.value == drag.0)
                                                && let Some(to) = view
                                                    .rules
                                                    .iter()
                                                    .position(|row| row.value == target)
                                                && from != to
                                            {
                                                let rule = view.rules.remove(from);
                                                view.rules.insert(to, rule);
                                                cx.notify();
                                            }
                                        },
                                    ))
                                })
                                .flex()
                                .items_center()
                                .gap(px(rt::RULE_GAP))
                                .child(
                                    super::icons::icon("Bars3", t::ICON_SMALL)
                                        .text_color(cx.theme().foreground.opacity(0.4)),
                                )
                                .child(
                                    components::select(&row.kind, tr(cx, "规则类型"))
                                        .outlined_focus()
                                        .w(px(rt::MATCHER_WIDTH))
                                        .rounded(px(t::ICON_PICKER_RADIUS))
                                        .indicator(
                                            if cx.theme().mode.is_dark() {
                                                rt::RULE_CARET_DARK
                                            } else {
                                                rt::RULE_CARET_LIGHT
                                            },
                                            8.,
                                            4.,
                                            16.,
                                        )
                                        .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY))
                                        .border_color(cx.theme().foreground.opacity(0.2))
                                        .disabled(busy),
                                )
                                .child(
                                    div().flex_1().min_w_0().child(
                                        components::text_input(&row.value)
                                            .outlined_focus(cx.theme().foreground)
                                            .readonly(busy)
                                            .w_full()
                                            .font_family(rt::RULE_FONT)
                                            .text_size(px(t::BODY))
                                            .rounded(px(t::ICON_PICKER_RADIUS))
                                            .border_color(cx.theme().foreground.opacity(0.2))
                                            .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY)),
                                    ),
                                )
                                .child(
                                    components::text_input(&row.note)
                                        .outlined_focus(cx.theme().foreground)
                                        .readonly(busy)
                                        .w(px(rt::RULE_NOTE_WIDTH))
                                        .flex_shrink_0()
                                        .border_color(cx.theme().foreground.opacity(0.2))
                                        .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY)),
                                )
                                .child(
                                    icon_button(
                                        SharedString::from(format!("routing-delete-rule-{i}")),
                                        tr(cx, "删除规则"),
                                        "Trash",
                                        t::CONTROL,
                                    )
                                    .disabled(busy)
                                    .on_click(cx.listener(
                                        move |view, _, _, cx| {
                                            view.rules.retain(|row| row.value != deleted);
                                            cx.notify();
                                        },
                                    )),
                                )
                        })),
                );
            form = form.child(rules);
        }
        if let Some(error) = &self.error {
            form = form.child(
                div()
                    .text_color(cx.theme().danger)
                    .child(tr(cx, error.code().message())),
            );
        }
        let body = div()
            .id("routing-editor-scroll")
            .track_scroll(&self.editor_scroll)
            .overflow_y_scroll()
            .w_full()
            .flex_1()
            .min_h_0()
            .p(px(t::SECTION_PADDING))
            .child(form);
        let surface = div()
            .w(px(if self.kind == EditorKind::Policy {
                rt::EDITOR_WIDTH
            } else {
                rt::FALLBACK_EDITOR_WIDTH
            }))
            .max_w(window.viewport_size().width - px(t::SECTION_PADDING * 2.))
            .max_h(window.viewport_size().height * rt::EDITOR_HEIGHT_RATIO)
            .when(self.kind == EditorKind::Policy, |surface| {
                surface.h(window.viewport_size().height * rt::EDITOR_HEIGHT_RATIO)
            })
            .font(t::openbox_font())
            .font_weight(t::FONT_REGULAR)
            .text_size(px(t::BODY))
            .line_height(px(t::BODY_LINE))
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(rt::EDITOR_RADIUS))
            .bg(cx.theme().popover.opacity(rt::FIELD_OPACITY))
            .backdrop_blur(px(self
                .state
                .as_ref()
                .map_or(0., |state| state.app_config.visual.background_blur as f32)))
            .on_children_prepainted(|bounds, _, _| {
                #[cfg(debug_assertions)]
                components::log_layout("routing-modal", &bounds);
            })
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
            // 原版 body 滚动条常显；覆盖 ScrollHandle 的 bounds，不占表单宽度。
            .child(
                Scrollbar::vertical(&self.editor_scroll)
                    .id("routing-editor-scrollbar")
                    .mode(ScrollbarMode::Always)
                    .styles(|s| {
                        let thumb = cx.theme().foreground.opacity(rt::EDITOR_SCROLL_ALPHA);
                        s.track(|v| v.width(px(rt::EDITOR_SCROLL_TRACK)))
                            .thumb(|v| {
                                v.bg(thumb)
                                    .width(px(rt::EDITOR_SCROLL_THUMB))
                                    .inset(px(rt::EDITOR_SCROLL_INSET))
                                    .radius(px(rt::EDITOR_SCROLL_RADIUS))
                            })
                            .thumb_hover(|v| {
                                v.bg(thumb)
                                    .width(px(rt::EDITOR_SCROLL_THUMB))
                                    .inset(px(rt::EDITOR_SCROLL_INSET))
                                    .radius(px(rt::EDITOR_SCROLL_RADIUS))
                            })
                            .thumb_active(|v| {
                                v.bg(thumb)
                                    .width(px(rt::EDITOR_SCROLL_THUMB))
                                    .inset(px(rt::EDITOR_SCROLL_INSET))
                                    .radius(px(rt::EDITOR_SCROLL_RADIUS))
                            })
                    }),
            )
            .child(
                div()
                    .w_full()
                    .h(px(rt::FOOTER_HEIGHT))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(t::GAP))
                    .px(px(t::SECTION_PADDING))
                    .border_t_1()
                    .border_color(cx.theme().foreground.opacity(0.1))
                    .on_children_prepainted(|bounds, _, _| {
                        #[cfg(debug_assertions)]
                        components::log_layout("routing-footer", &bounds);
                    })
                    .child(
                        button("routing-cancel", tr(cx, "取消"))
                            .w(px(rt::FOOTER_BUTTON_WIDTH))
                            .px(px(t::PAD))
                            .disabled(busy)
                            .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
                    )
                    .child(
                        button("routing-save", tr(cx, "保存"))
                            .w(px(rt::FOOTER_BUTTON_WIDTH))
                            .px(px(t::PAD))
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
// 与 React applyPolicyRules 的备注键一致；备注不进入匹配或 Compiler。
fn import_error_message(error: RuleImportError) -> &'static str {
    match error {
        RuleImportError::Cancelled => "规则预览已取消",
        RuleImportError::Timeout => "规则预览超时，请重试",
        RuleImportError::InvalidUrl => "请填写有效的规则集地址",
        RuleImportError::UnsupportedFormat => "规则格式不支持，请使用 .list、.mrs 或 .srs",
        RuleImportError::InvalidFormat => "规则文件无效或已损坏",
        RuleImportError::UnsupportedRule => "规则包含当前明细导入无法等价表达的条件",
        RuleImportError::BodyTooLarge => "规则文件超过读取大小限制",
        RuleImportError::ProxyUnavailable => "当前运行代理不可用，规则预览失败",
        RuleImportError::Fetch(veyra_core::subscription::FetchError::DirectPathUnverified) => {
            "当前直连路径无法确认，规则预览失败"
        }
        RuleImportError::KernelUnavailable => "固定内核资源不可用，无法读取 .srs",
        RuleImportError::DecompileFailed => "规则集解码失败",
        RuleImportError::Fetch(_) | RuleImportError::Io => "规则预览失败，请重试",
    }
}

// 等价 maxlength：只截本次新增部分，原有前缀/后缀和选中替换的语义保持。
fn limit_note_edit(previous: &str, edited: &str) -> String {
    if edited.encode_utf16().count() <= 60 {
        return edited.into();
    }
    let prefix = previous
        .chars()
        .zip(edited.chars())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let suffix = previous[prefix..]
        .chars()
        .rev()
        .zip(edited[prefix..].chars().rev())
        .take_while(|(a, b)| a == b)
        .map(|(c, _)| c.len_utf8())
        .sum::<usize>();
    let retained = edited[..prefix].encode_utf16().count()
        + edited[edited.len() - suffix..].encode_utf16().count();
    let budget = 60usize.saturating_sub(retained);
    let mut units = 0;
    let inserted = edited[prefix..edited.len() - suffix]
        .chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= budget
        })
        .collect::<String>();
    format!(
        "{}{}{}",
        &edited[..prefix],
        inserted,
        &edited[edited.len() - suffix..]
    )
}

fn rule_note_key(kind: usize, value: &str) -> String {
    format!(
        "{}:{value}",
        [
            "domain",
            "domainSuffix",
            "application",
            "ipCidr",
            "port",
            "protocol",
            "domainKeyword",
            "ruleset"
        ][kind]
    )
}
// React policySummary：卡片按类型汇总；编辑行和持久 Matcher 的分组方式不影响摘要。
fn policy_summary(matchers: &[TrafficMatcher]) -> Vec<(usize, String)> {
    let mut groups: [Vec<String>; RULES.len()] = std::array::from_fn(|_| Vec::new());
    for matcher in matchers {
        let (kind, values) = matcher_input(matcher);
        groups[kind].push(values);
    }
    // 原版 summaryFields：规则集、域名后缀、域名、关键词、IP；本地已有类型放后面。
    // ruleUrls 尚无正式模型，不制造假字段；保持所有已支持类型和类型内原值顺序。
    [7, 1, 0, 6, 3, 2, 4, 5]
        .into_iter()
        .filter(|kind| !groups[*kind].is_empty())
        .map(|kind| (kind, groups[kind].join(", ")))
        .collect()
}
fn matcher_rows(matcher: &TrafficMatcher) -> Vec<(usize, String)> {
    let (kind, joined) = matcher_input(matcher);
    match matcher {
        TrafficMatcher::Domain(values)
        | TrafficMatcher::DomainSuffix(values)
        | TrafficMatcher::DomainKeyword(values)
        | TrafficMatcher::IpCidr(values)
        | TrafficMatcher::RuleSet(values) => {
            values.iter().map(|value| (kind, value.clone())).collect()
        }
        // 应用/端口/协议仍保留既有类型语义，不把空格视作应用名称分隔符。
        _ => vec![(kind, joined)],
    }
}
fn split_rule_values(kind: &str, value: &str) -> Vec<String> {
    let domain_rule = matches!(kind, "域名" | "域名后缀" | "域名关键词" | "IP 段");
    // React splitRuleRows 的相同分隔符；应用名称仍可包含空格。
    value
        .split(|c: char| {
            matches!(c, ',' | '\n')
                || (domain_rule && (c.is_whitespace() || matches!(c, '，' | ';' | '；' | '、')))
        })
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}
fn parse_matcher(kind: &str, value: &str) -> Option<TrafficMatcher> {
    let values = split_rule_values(kind, value);
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
            // Settings 的外层是定高块容器；百分比高度约束列表，避免自然高度挤掉滚动范围。
            .h_full()
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
                body =
                    body.child(
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
                                        div()
                                            .truncate()
                                            .text_size(px(rt::CARD_TITLE))
                                            .line_height(px(rt::CARD_TITLE_LINE))
                                            .child(custom.name.clone().unwrap_or_else(|| {
                                                tr(cx, "前置自定义分流").into()
                                            })),
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
                    policy_summary(&policy.matchers)
                        .into_iter()
                        .map(|(kind, value)| format!("{}: {value}", tr(cx, RULES[kind])))
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
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(rt::CARD_TITLE))
                                    .line_height(px(rt::CARD_TITLE_LINE))
                                    .font_weight(pt::FONT_MEDIUM)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(t::SMALL))
                                    .line_height(px(rt::CARD_SUMMARY_LINE))
                                    .mt(px(rt::CARD_COPY_GAP))
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
        .when(self.import_open, |body| {
            body.child(self.import_editor(window, cx))
        })
        .when(self.restore_open, |body| {
            body.child(self.restore_dialog(cx))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // 保护备注满长时的前/中部插入与选中替换，不丢失未编辑的尾部。
    #[::core::prelude::v1::test]
    fn note_length_limits_only_inserted_text_and_preserves_suffix() {
        let full = format!("{}Z", "A".repeat(59));
        assert_eq!(limit_note_edit(&full, &format!("ABC{full}")), full);
        assert_eq!(
            limit_note_edit(&full, &format!("{}XYZ{}", &full[..30], &full[30..])),
            full
        );
        assert_eq!(
            limit_note_edit(&full, &format!("BCDEFG{}", &full[3..])),
            format!("BCD{}", &full[3..])
        );
        assert_eq!(limit_note_edit("", &"A".repeat(61)), "A".repeat(60));
        assert_eq!(limit_note_edit("", &"😀".repeat(31)), "😀".repeat(30));
    }

    // 保护保存后多个独立编辑行仍显示一个类型标签，且摘要顺序和原版一致。
    #[::core::prelude::v1::test]
    fn routing_summary_is_identical_for_grouped_and_saved_individual_rules() {
        let grouped = vec![
            TrafficMatcher::Domain(vec!["one.example".into(), "two.example".into()]),
            TrafficMatcher::IpCidr(vec!["192.0.2.0/24".into(), "2001:db8::/32".into()]),
            TrafficMatcher::DomainSuffix(vec!["example.com".into(), "example.org".into()]),
            TrafficMatcher::RuleSet(vec!["geosite-cn".into(), "geoip-cn".into()]),
            TrafficMatcher::DomainKeyword(vec!["video".into(), "search".into()]),
        ];
        let saved = grouped
            .iter()
            .flat_map(matcher_rows)
            .map(|(kind, value)| parse_matcher(RULES[kind], &value).unwrap())
            .collect::<Vec<_>>();
        let expected = vec![
            (7, "geosite-cn, geoip-cn".into()),
            (1, "example.com, example.org".into()),
            (0, "one.example, two.example".into()),
            (6, "video, search".into()),
            (3, "192.0.2.0/24, 2001:db8::/32".into()),
        ];
        assert_eq!(policy_summary(&grouped), expected);
        assert_eq!(policy_summary(&saved), expected);
        assert!(policy_summary(&[]).is_empty());
    }
    // 汇总保留交错类型里的值、重复值及本地已有应用/端口/协议，不修改匹配配置。
    #[::core::prelude::v1::test]
    fn routing_summary_preserves_interleaved_values_and_local_rule_types() {
        let matchers = vec![
            TrafficMatcher::DomainSuffix(vec!["one.example".into()]),
            TrafficMatcher::Application(vec!["My Browser".into()]),
            TrafficMatcher::Port(vec![443]),
            TrafficMatcher::Protocol(vec![NetworkProtocol::Tcp]),
            TrafficMatcher::DomainSuffix(vec!["two.example".into(), "one.example".into()]),
            TrafficMatcher::Application(vec!["Other Browser".into()]),
            TrafficMatcher::Port(vec![8443]),
            TrafficMatcher::Protocol(vec![NetworkProtocol::Udp]),
        ];
        assert_eq!(
            policy_summary(&matchers),
            vec![
                (1, "one.example, two.example, one.example".into()),
                (2, "My Browser, Other Browser".into()),
                (4, "443, 8443".into()),
                (5, "tcp, udp".into()),
            ]
        );
    }
    // 保护已有多域名策略打开、逐条编辑、保存后不丢值、不改变类型与顺序。
    #[::core::prelude::v1::test]
    fn routing_editor_roundtrip_preserves_each_domain_and_rule_value() {
        for matcher in [
            TrafficMatcher::Domain(vec!["one.example".into(), "two.example".into()]),
            TrafficMatcher::DomainSuffix(vec!["example.com".into(), "example.org".into()]),
            TrafficMatcher::DomainKeyword(vec!["video".into(), "search".into()]),
            TrafficMatcher::IpCidr(vec!["192.0.2.0/24".into(), "2001:db8::/32".into()]),
            TrafficMatcher::RuleSet(vec!["geosite-cn".into(), "geoip-cn".into()]),
        ] {
            let rows = matcher_rows(&matcher);
            assert_eq!(rows.len(), 2);
            let restored = rows
                .iter()
                .map(|(kind, value)| parse_matcher(RULES[*kind], value).unwrap())
                .collect::<Vec<_>>();
            assert_eq!(
                restored
                    .iter()
                    .map(matcher_input)
                    .map(|(_, value)| value)
                    .collect::<Vec<_>>()
                    .join(", "),
                matcher_input(&matcher).1
            );
            assert!(
                restored
                    .iter()
                    .all(|value| matcher_input(value).0 == matcher_input(&matcher).0)
            );
        }
        assert_eq!(
            matcher_rows(&TrafficMatcher::Application(vec![
                "My Browser".into(),
                "Other Browser".into()
            ])),
            vec![(2, "My Browser, Other Browser".into())]
        );
    }
    // 保护粘贴批量域名的失焦拆分与直接保存一致；保留应用名称的合法空格。
    #[::core::prelude::v1::test]
    fn routing_domain_paste_accepts_reference_separators_without_losing_values() {
        let raw = "one.example, two.example，three.example;four.example；five.example、six.example\nseven.example eight.example";
        let values = split_rule_values("域名", raw);
        assert_eq!(
            values,
            [
                "one", "two", "three", "four", "five", "six", "seven", "eight"
            ]
            .into_iter()
            .map(|name| format!("{name}.example"))
            .collect::<Vec<_>>()
        );
        assert_eq!(
            parse_matcher("域名", raw),
            Some(TrafficMatcher::Domain(values))
        );
        assert_eq!(
            parse_matcher("应用", "My Browser, Other Browser"),
            Some(TrafficMatcher::Application(vec![
                "My Browser".into(),
                "Other Browser".into()
            ]))
        );
        assert!(parse_matcher("域名", "，；、 \n").is_none());
    }
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
