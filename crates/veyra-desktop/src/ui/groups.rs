//! OpenBox GroupSettings 的业务编辑器；仅使用 Core 快照、CAS 保存和同源组件。
use super::{
    components::{
        self,
        icon_picker::{IconPicked, IconPicker, group_icon},
        select::{Select, SelectEvent, SelectState},
    },
    i18n::tr,
    icons::icon,
    tokens as t,
};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, IndexPath,
        button::ButtonVariants,
        input::{InputEvent, InputState},
        popover::Popover,
    },
    prelude::*,
    *,
};
use std::collections::HashSet;
use veyra_core::{application::state_service::GroupSaveError, domain::*};

#[derive(Clone)]
pub struct GroupsEvent {
    pub request: u64,
    pub save: Option<(ConfigVersion, Vec<NodeGroup>)>,
}
#[derive(Clone, Copy)]
enum Field {
    Name,
    Interval,
    Tolerance,
    Url,
    Keywords,
    Available,
    Selected,
    Country,
    Scale,
    LaneName,
    Timeout,
    Failure,
    Recovery,
}
#[derive(Clone)]
struct Member {
    id: OutboundId,
    name: String,
    subscription: String,
}
#[derive(Clone)]
struct GroupDrag {
    index: usize,
    name: String,
}
impl Render for GroupDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px(px(t::PAD))
            .py(px(t::GAP))
            .rounded(px(t::RADIUS))
            .bg(cx.theme().popover)
            .child(self.name.clone())
    }
}
#[derive(Clone)]
struct MemberDrag {
    id: OutboundId,
    name: String,
}
impl Render for MemberDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(t::GAP))
            .bg(cx.theme().popover)
            .child(self.name.clone())
    }
}
#[derive(Clone)]
struct CountryDrag {
    index: usize,
    name: String,
}
impl Render for CountryDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(t::GAP))
            .bg(cx.theme().popover)
            .child(self.name.clone())
    }
}
#[derive(Clone)]
enum Confirmation {
    Delete(PoolId),
    Restore,
}

pub struct GroupsView {
    pub state: Option<Box<AppState>>,
    pub busy: bool,
    visible: bool,
    request: u64,
    load_error: bool,
    loading: bool,
    error: Option<GroupSaveError>,
    draft: Option<NodeGroup>,
    inputs: [Entity<InputState>; 13],
    _input_events: Vec<gpui_kit::Subscription>,
    language: DesktopLanguage,
    picker: Entity<IconPicker>,
    rule: Entity<SelectState>,
    lane_mode: Entity<SelectState>,
    lane_picker: Entity<IconPicker>,
    active_lane: usize,
    advanced: bool,
    deleting_lane: bool,
    discard_confirm: bool,
    discard_accepted: bool,
    original_failover: bool,
    previous_plain: Option<NodeGroup>,
    previous_failover: Option<NodeGroup>,
    filters: [Entity<SelectState>; 2],
    filter_values: [String; 2],
    checked: [HashSet<OutboundId>; 2],
    confirmation: Option<Confirmation>,
    auto_open: bool,
    auto_codes: Vec<String>,
    auto_rules: [bool; 2],
    focus: FocusHandle,
    trigger: Option<FocusHandle>,
    scroll: ScrollHandle,
}
impl EventEmitter<GroupsEvent> for GroupsView {}
impl GroupsView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let inputs = std::array::from_fn(|i| {
            cx.new(|cx| InputState::new(window, cx).placeholder(tr(cx, PLACEHOLDERS[i])))
        });
        let picker = cx.new(|cx| {
            let mut p = IconPicker::new(String::new(), window, cx);
            p.expanded = true;
            p
        });
        let rule = cx.new(|cx| {
            SelectState::new(
                vec![
                    "自动择优（url-test）",
                    "手动选择（select）",
                    "故障转移（failover）",
                ],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let lane_mode = cx.new(|cx| {
            SelectState::new(
                vec!["自动优选", "手动选择"],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let lane_picker = cx.new(|cx| {
            let mut p = IconPicker::new(String::new(), window, cx);
            p.expanded = true;
            p.expanded_width = t::groups::LANE_ICON_WIDTH;
            p
        });
        let filters = std::array::from_fn(|_| {
            cx.new(|cx| {
                SelectState::new(
                    vec!["全部", "全部节点组", "全部节点"],
                    Some(IndexPath::new(0)),
                    window,
                    cx,
                )
            })
        });
        let mut input_events = vec![
            cx.subscribe(&picker, |this, _, event: &IconPicked, cx| {
                if this.busy {
                    return;
                }
                if let Some(d) = &mut this.draft {
                    d.icon = event.0.clone();
                }
                cx.notify();
            }),
            cx.subscribe_in(&rule, window, |this, _, event: &SelectEvent, window, cx| {
                if this.busy {
                    return;
                }
                if let SelectEvent::Confirm(Some(v)) = event {
                    let rule = match v.as_ref() {
                        "手动选择（select）" => GroupRule::Selector,
                        "故障转移（failover）" => GroupRule::Failover,
                        _ => GroupRule::UrlTest,
                    };
                    this.change_rule(rule, window, cx);
                }
                cx.notify();
            }),
        ];
        input_events.push(
            cx.subscribe(&lane_mode, |this, _, event: &SelectEvent, cx| {
                if !this.busy
                    && let SelectEvent::Confirm(Some(v)) = event
                    && let Some(lane) = this.active_lane_mut()
                {
                    lane.manual = v.as_ref() == "手动选择";
                    cx.notify();
                }
            }),
        );
        input_events.push(
            cx.subscribe(&lane_picker, |this, _, event: &IconPicked, cx| {
                if !this.busy
                    && let Some(lane) = this.active_lane_mut()
                {
                    lane.icon = event.0.clone();
                    cx.notify();
                }
            }),
        );
        for (i, input) in inputs.iter().enumerate() {
            input_events.push(
                cx.subscribe(input, move |this, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.error = None;
                        if i == Field::LaneName as usize
                            && let Some(lane) = this.active_lane_mut()
                        {
                            lane.name = input.read(cx).value().to_string();
                        }
                        if i == Field::Keywords as usize
                            && let Some(d) = &mut this.draft
                        {
                            d.keywords = comma_values(input.read(cx).value().as_str());
                        }
                        cx.notify();
                    }
                }),
            );
        }
        for (side, filter) in filters.iter().enumerate() {
            input_events.push(
                cx.subscribe(filter, move |this, _, event: &SelectEvent, cx| {
                    if let SelectEvent::Confirm(Some(v)) = event {
                        this.filter_values[side] = v.to_string();
                        cx.notify();
                    }
                }),
            );
        }
        Self {
            state: None,
            busy: false,
            visible: false,
            request: 0,
            load_error: false,
            loading: false,
            error: None,
            draft: None,
            inputs,
            _input_events: input_events,
            language: cx.global::<super::i18n::Locale>().0,
            picker,
            rule,
            lane_mode,
            lane_picker,
            active_lane: 0,
            advanced: false,
            deleting_lane: false,
            discard_confirm: false,
            discard_accepted: false,
            original_failover: false,
            previous_plain: None,
            previous_failover: None,
            filters,
            filter_values: ["全部".into(), "全部".into()],
            checked: Default::default(),
            confirmation: None,
            auto_open: false,
            auto_codes: vec![],
            auto_rules: [true, false],
            focus: cx.focus_handle(),
            trigger: None,
            scroll: ScrollHandle::new(),
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.visible = visible;
            if visible && !self.busy {
                self.send(None, cx);
            }
        }
    }
    pub fn project(&mut self, state: &AppState, cx: &mut Context<Self>) {
        if self.draft.is_none()
            && !self.busy
            && self.state.as_ref().is_none_or(|s| {
                s.state_epoch != state.state_epoch || s.config_revision <= state.config_revision
            })
        {
            self.state = Some(Box::new(state.clone()));
            cx.notify();
        }
    }
    fn send(&mut self, save: Option<Vec<NodeGroup>>, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let save = if let Some(groups) = save {
            let Some(state) = &self.state else { return };
            Some((state.config_version(), groups))
        } else {
            None
        };
        self.request += 1;
        self.busy = true;
        self.loading = save.is_none();
        self.error = None;
        self.load_error = false;

        cx.emit(GroupsEvent {
            request: self.request,
            save,
        });
        cx.notify();
    }
    pub fn complete(
        &mut self,
        request: u64,
        result: Result<Box<AppState>, GroupSaveError>,
        saved: bool,
        snapshot: Option<Box<AppState>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<Box<AppState>> {
        if request != self.request {
            return None;
        }
        self.busy = false;
        self.loading = false;
        match result {
            Ok(state) => {
                self.state = Some(state.clone());

                self.error = None;
                if saved {
                    let dropped = state.dropped_groups();
                    let names = state
                        .groups
                        .iter()
                        .filter(|g| dropped.contains(&g.id))
                        .map(|g| g.name.as_str())
                        .collect::<Vec<_>>()
                        .join("、");
                    let message = if names.is_empty() {
                        tr(cx, "已保存，待应用").to_owned()
                    } else {
                        format!(
                            "{}；{}：{}",
                            tr(cx, "已保存，待应用"),
                            tr(cx, "空组未编译"),
                            names
                        )
                    };
                    components::notify(components::Notice::Success, message, window, cx);
                    self.close(window, cx);
                }
                cx.notify();
                Some(state)
            }
            Err(error) => {
                if let Some(state) = &snapshot {
                    self.state = Some(state.clone());
                }
                self.load_error = !saved;
                if saved {
                    // React persist 的失败走同一全局通知；保留编辑草稿供原地重试。
                    let message = error_text(
                        &error,
                        self.state.as_deref(),
                        self.draft.as_ref(),
                        cx.global::<super::i18n::Locale>().0,
                    );
                    components::notify(components::Notice::Error, message, window, cx);
                    self.error = None;
                } else {
                    self.error = Some(error);
                }
                cx.notify();
                snapshot
            }
        }
    }
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.draft = None;
        self.deleting_lane = false;
        self.discard_confirm = false;
        self.confirmation = None;
        self.auto_open = false;
        if let Some(f) = self.trigger.take() {
            f.focus(window, cx);
        }
        cx.notify();
    }
    pub fn open_add(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        // OS 随机身份来自已有 StateEpoch 工具，不使用可冲突的显示名。
        let group = match NodeGroup::fresh() {
            Ok(v) => v,
            Err(error) => {
                self.error = Some(GroupSaveError::Storage(error));
                cx.notify();
                return;
            }
        };
        self.edit(group, window, cx);
    }
    fn edit(&mut self, group: NodeGroup, window: &mut Window, cx: &mut Context<Self>) {
        self.trigger = window.focused(cx);
        self.error = None;
        self.active_lane = 0;
        self.advanced = false;
        self.deleting_lane = false;
        self.discard_confirm = false;
        self.discard_accepted = false;
        self.original_failover = group.rule == GroupRule::Failover;
        self.previous_plain = None;
        self.previous_failover = None;
        let settings = group.failover.clone().unwrap_or_default();
        let values = [
            group.name.clone(),
            group.interval_secs.to_string(),
            group.tolerance_ms.to_string(),
            group.test_url.clone(),
            group.keywords.join(","),
            String::new(),
            String::new(),
            String::new(),
            group.icon_scale.to_string(),
            group
                .lanes
                .first()
                .map(|l| l.name.clone())
                .unwrap_or_default(),
            (settings.timeout_ms / 1000).to_string(),
            settings.failure_threshold.to_string(),
            (settings.recovery_hold_ms / 1000).to_string(),
        ];
        for (input, value) in self.inputs.iter().zip(values) {
            input.update(cx, |i, cx| i.set_value(value, window, cx));
        }
        self.picker.update(cx, |p, cx| {
            p.value = group.icon.clone();
            cx.notify();
        });
        self.rule.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match group.rule {
                    GroupRule::Selector => 1,
                    GroupRule::Failover => 2,
                    _ => 0,
                })),
                window,
                cx,
            )
        });
        self.checked = Default::default();
        self.filter_values = ["全部".into(), "全部".into()];
        self.draft = Some(group);
        self.sync_lane(window, cx);
        self.refresh_filters(window, cx);
        self.inputs[Field::Name as usize]
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
        cx.notify();
    }
    fn refresh_filters(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let failover = self
            .draft
            .as_ref()
            .is_some_and(|g| g.rule == GroupRule::Failover);
        let mut options = if failover {
            vec!["全部".to_owned()]
        } else {
            vec!["全部".to_owned(), "全部节点组".into(), "全部节点".into()]
        };
        for member in self.members() {
            if !member.subscription.is_empty() && !options.contains(&member.subscription) {
                options.push(member.subscription);
            }
        }
        // 同一组件的动态选项：订阅名不要求静态字符串，也不泄漏临时内存。
        for (side, filter) in self.filters.iter().enumerate() {
            filter.update(cx, |s, cx| {
                s.translated_options = if failover { 1 } else { 3 };
                s.set_options(options.clone(), window, cx)
            });
            self.filter_values[side] = "全部".into();
        }
    }
    pub fn restore(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.busy {
            self.trigger = window.focused(cx);
            self.confirmation = Some(Confirmation::Restore);
            self.focus.focus(window, cx);
            cx.notify();
        }
    }
    pub fn automatic(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.trigger = window.focused(cx);
        self.auto_open = true;
        self.auto_codes = ["HK", "TW", "SG", "JP", "KR", "US"]
            .map(str::to_owned)
            .to_vec();
        self.auto_rules = [true, false];
        self.inputs[Field::Country as usize].update(cx, |i, cx| i.set_value("", window, cx));
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn save_draft(&mut self, cx: &mut Context<Self>) {
        if self.original_failover
            && !self.discard_accepted
            && self
                .draft
                .as_ref()
                .is_some_and(|d| d.rule != GroupRule::Failover)
        {
            self.discard_confirm = true;
            cx.notify();
            return;
        }
        let (Some(mut draft), Some(state)) = (self.draft.clone(), self.state.as_ref()) else {
            return;
        };
        draft.name = self.value(Field::Name, cx).trim().to_owned();
        draft.test_url = self.value(Field::Url, cx).trim().to_owned();
        let Ok(scale) = self.value(Field::Scale, cx).parse() else {
            self.error = Some(GroupSaveError::Invalid(GroupIssue::InvalidSettings(
                draft.id,
            )));
            cx.notify();
            return;
        };
        draft.icon_scale = scale;
        if matches!(draft.rule, GroupRule::UrlTest | GroupRule::Failover) {
            let (Ok(interval), Ok(tolerance)) = (
                self.value(Field::Interval, cx).parse(),
                self.value(Field::Tolerance, cx).parse(),
            ) else {
                self.error = Some(GroupSaveError::Invalid(GroupIssue::InvalidSettings(
                    draft.id,
                )));
                cx.notify();
                return;
            };
            draft.interval_secs = interval;
            draft.tolerance_ms = tolerance;
        }
        if draft.rule == GroupRule::Failover {
            let (Ok(timeout), Ok(failure), Ok(recovery)) = (
                self.value(Field::Timeout, cx).parse::<u64>(),
                self.value(Field::Failure, cx).parse::<u32>(),
                self.value(Field::Recovery, cx).parse::<u64>(),
            ) else {
                self.error = Some(GroupSaveError::Invalid(GroupIssue::InvalidSettings(
                    draft.id,
                )));
                cx.notify();
                return;
            };
            let (Some(timeout_ms), Some(recovery_hold_ms)) =
                (timeout.checked_mul(1000), recovery.checked_mul(1000))
            else {
                self.error = Some(GroupSaveError::Invalid(GroupIssue::InvalidSettings(
                    draft.id,
                )));
                cx.notify();
                return;
            };
            let settings = draft.failover.as_mut().expect("failover draft");
            settings.timeout_ms = timeout_ms;
            settings.failure_threshold = failure;
            settings.recovery_hold_ms = recovery_hold_ms;
        }
        let mut groups = state.groups.clone();
        if let Some(g) = groups.iter_mut().find(|g| g.id == draft.id) {
            *g = draft;
        } else {
            groups.push(draft);
        }
        self.send(Some(groups), cx);
    }
    fn value(&self, field: Field, cx: &App) -> String {
        self.inputs[field as usize].read(cx).value().to_string()
    }
    fn members(&self) -> Vec<Member> {
        let Some(state) = &self.state else {
            return vec![];
        };
        let current = self.draft.as_ref().map(NodeGroup::outbound_id);
        let mut members = state
            .groups
            .iter()
            .filter(|g| g.enabled && Some(g.outbound_id()) != current)
            .map(|g| Member {
                id: g.outbound_id(),
                name: g.name.clone(),
                subscription: String::new(),
            })
            .collect::<Vec<_>>();
        if !members.iter().any(|m| m.id == OutboundId::Direct)
            && !state.groups.iter().any(|g| g.rule == GroupRule::Direct)
        {
            members.push(Member {
                id: OutboundId::Direct,
                name: "直连".into(),
                subscription: String::new(),
            });
        }
        if !members.iter().any(|m| m.id == OutboundId::Block) {
            members.push(Member {
                id: OutboundId::Block,
                name: "拒绝".into(),
                subscription: String::new(),
            });
        }
        members.extend(state.group_available_nodes().map(|node| {
            Member {
                id: OutboundId::Node(node.id.clone()),
                name: node.name.clone(),
                subscription: state
                    .providers
                    .iter()
                    .find(|p| p.id == node.provider_id)
                    .and_then(|p| {
                        state
                            .subscriptions
                            .iter()
                            .find(|s| s.id == p.subscription_id)
                    })
                    .map(|s| s.name.clone())
                    .unwrap_or_default(),
            }
        }));
        if self
            .draft
            .as_ref()
            .is_some_and(|d| d.rule == GroupRule::Failover)
        {
            members.retain(|m| matches!(m.id, OutboundId::Node(_)));
        }
        members
    }
    fn member_pool(&self, side: usize) -> Vec<Member> {
        if self.draft.is_none() {
            return vec![];
        }
        let all = self.members();
        let selected = self.draft_members();
        if side == 0 {
            all.into_iter()
                .filter(|m| !selected.contains(&m.id))
                .collect::<Vec<_>>()
        } else {
            selected
                .iter()
                .map(|id| {
                    all.iter()
                        .find(|m| &m.id == id)
                        .cloned()
                        .unwrap_or_else(|| Member {
                            id: id.clone(),
                            name: format!("{id:?}"),
                            subscription: String::new(),
                        })
                })
                .collect()
        }
    }
    fn pane_members(&self, side: usize, cx: &App) -> Vec<Member> {
        let pool = self.member_pool(side);
        let search = self
            .value(
                if side == 0 {
                    Field::Available
                } else {
                    Field::Selected
                },
                cx,
            )
            .trim()
            .to_lowercase();
        let filter = &self.filter_values[side];
        pool.into_iter()
            .filter(|m| {
                (search.is_empty() || m.name.to_lowercase().contains(&search))
                    && (filter == "全部"
                        || (filter == "全部节点" && matches!(m.id, OutboundId::Node(_)))
                        || (filter == "全部节点组" && !matches!(m.id, OutboundId::Node(_)))
                        || filter == &m.subscription)
            })
            .collect()
    }
    fn transfer(&mut self, side: usize, ids: Vec<OutboundId>, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let active_lane = self.active_lane;
        if let Some(draft) = &mut self.draft {
            let members = if draft.rule == GroupRule::Failover {
                let Some(lane) = draft.lanes.get_mut(active_lane) else {
                    return;
                };
                &mut lane.members
            } else {
                &mut draft.members
            };
            transfer_members(members, &mut self.checked[side], side, ids);
        }
        cx.notify();
    }
    fn input(&self, field: Field, cx: &App) -> impl IntoElement {
        components::text_input(&self.inputs[field as usize])
            .disabled(self.busy)
            .border_focus()
            .bg(cx.theme().popover)
            .border_color(cx.theme().foreground.opacity(0.2))
            .h(px(t::CONTROL))
            .text_size(px(t::BODY))
    }
    fn editor(&mut self, cx: &mut Context<Self>) -> Div {
        let draft = self.draft.as_ref().unwrap().clone();
        self.picker.update(cx, |p, _| p.disabled = self.busy);
        let scale = div()
            .flex()
            .h(px(t::CONTROL))
            .child(
                components::icon_button_content(
                    "scale-down",
                    tr(cx, "缩小"),
                    t::groups::CARD_ACTION,
                    icon("Minus", t::BODY),
                )
                .disabled(self.busy)
                .h(px(t::CONTROL))
                .bg(cx.theme().foreground.opacity(0.07))
                .rounded_none()
                .rounded_l(px(t::RADIUS))
                .on_click(cx.listener(|this, _, window, cx| this.scale(-1, window, cx))),
            )
            .child(
                div()
                    .w(px(t::groups::SCALE_VALUE))
                    .flex_shrink_0()
                    .ml(px(-1.))
                    .child(
                        components::text_input(&self.inputs[Field::Scale as usize])
                            .disabled(self.busy)
                            .border_focus()
                            .px_0()
                            .py_0()
                            .rounded_none()
                            .font_family("SFMono-Regular")
                            .text_size(px(t::SMALL))
                            .text_center()
                            .bg(cx.theme().popover),
                    ),
            )
            .child(
                components::icon_button_content(
                    "scale-up",
                    tr(cx, "放大"),
                    t::groups::CARD_ACTION,
                    icon("Plus", t::BODY),
                )
                .ml(px(-1.))
                .disabled(self.busy)
                .h(px(t::CONTROL))
                .bg(cx.theme().foreground.opacity(0.07))
                .rounded_none()
                .on_click(cx.listener(|this, _, window, cx| this.scale(1, window, cx))),
            )
            .child(
                group_button("scale-reset", tr(cx, "重置"), t::BODY)
                    .w(px(t::groups::SCALE_RESET))
                    .bg(cx.theme().foreground.opacity(0.07))
                    .px_0()
                    .rounded_none()
                    .rounded_r(px(t::RADIUS))
                    .disabled(self.busy || self.value(Field::Scale, cx) == "0")
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.inputs[Field::Scale as usize]
                            .update(cx, |i, cx| i.set_value("0", window, cx));
                        cx.notify();
                    })),
            );
        let primary = div()
            .flex()
            .items_end()
            .gap(px(t::GAP))
            .child(field("图标", self.picker.clone(), cx).w(px(t::groups::ICON_FIELD)))
            .child(
                field("缩放", scale, cx)
                    .w(px(t::groups::SCALE_WIDTH))
                    .flex_shrink_0(),
            )
            .child(field("分组名称", self.input(Field::Name, cx), cx).flex_1());
        let mut form = div()
            .flex()
            .flex_col()
            .gap(px(t::SECTION_PADDING))
            .child(primary);
        if draft.builtin() {
            return form.child(hint("内置出口仅可改名和图标", cx));
        }
        let mut rules = div().flex().items_end().gap(px(t::PAD)).child(field(
            "分组规则",
            Select::new(&self.rule, tr(cx, "分组规则"))
                .compact()
                .disabled(self.busy)
                .w(px(t::groups::RULE_FIELD)),
            cx,
        ));
        if matches!(draft.rule, GroupRule::UrlTest | GroupRule::Failover) {
            rules = rules
                .child(field(
                    "检测间隔",
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::ROW_GAP))
                        .child(
                            div()
                                .w(px(t::NUMBER_WIDTH))
                                .child(self.input(Field::Interval, cx)),
                        )
                        .child(hint("秒", cx)),
                    cx,
                ))
                .child(field(
                    if draft.rule == GroupRule::Failover {
                        "组内延迟容差"
                    } else {
                        "容差"
                    },
                    div()
                        .flex()
                        .items_center()
                        .gap(px(t::ROW_GAP))
                        .child(
                            div().w(px(t::NUMBER_WIDTH)).child(
                                components::text_input(&self.inputs[Field::Tolerance as usize])
                                    .disabled(
                                        self.busy
                                            || (draft.rule == GroupRule::Failover
                                                && draft
                                                    .lanes
                                                    .iter()
                                                    .all(|l| l.members.len() <= 1)),
                                    )
                                    .border_focus(),
                            ),
                        )
                        .child(hint("毫秒", cx)),
                    cx,
                ));
        }
        form = form.child(rules);
        if matches!(draft.rule, GroupRule::UrlTest | GroupRule::Failover) {
            form = form.child(field("测速地址", self.input(Field::Url, cx), cx));
        }
        if draft.rule == GroupRule::Failover {
            return form.child(self.failover_editor(cx));
        }
        let tabs = div()
            .flex()
            .w(px(t::groups::TABS_WIDTH))
            .p(px(t::ROW_GAP))
            .rounded(px(t::RADIUS))
            .bg(cx.theme().popover)
            .children(
                [
                    (GroupMode::Dynamic, "动态组"),
                    (GroupMode::Static, "静态组"),
                ]
                .map(|(mode, label)| {
                    group_button(label, tr(cx, label), t::BODY)
                        .ghost()
                        .rounded(px(t::groups::COMPACT_RADIUS))
                        .text_color(cx.theme().muted_foreground)
                        .px(px(t::GAP))
                        .disabled(self.busy)
                        .when(mode == draft.mode, |b| {
                            b.primary().text_color(rgb(t::groups::CHECK_COLOR))
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(d) = &mut this.draft {
                                d.mode = mode;
                            }
                            cx.notify();
                        }))
                }),
            );
        form = form.child(tabs);
        if draft.mode == GroupMode::Dynamic {
            let names = self
                .state
                .as_ref()
                .map(|state| {
                    state
                        .group_available_nodes()
                        .filter(|n| {
                            draft.keywords.is_empty()
                                || draft
                                    .keywords
                                    .iter()
                                    .any(|k| group_keyword_matches(&n.name, k))
                        })
                        .map(|n| n.name.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let results = div()
                .border_1()
                .border_color(cx.theme().foreground.opacity(0.1))
                .rounded(px(t::POPOVER_RADIUS))
                .overflow_hidden()
                .child(
                    div()
                        .px(px(t::PAD))
                        .py(px(6.))
                        .border_b_1()
                        .border_color(cx.theme().foreground.opacity(0.1))
                        .text_size(px(t::SMALL))
                        .line_height(px(t::groups::HINT_LINE))
                        .font_weight(super::theme::MISANS_MEDIUM)
                        .child(
                            tr(cx, "当前命中 {count} 个节点")
                                .replace("{count}", &names.len().to_string()),
                        ),
                )
                .child(
                    div()
                        .id("dynamic-members")
                        .max_h(px(t::groups::DYNAMIC_HEIGHT))
                        .overflow_y_scroll()
                        .children(names.iter().map(|name| {
                            div()
                                .px(px(t::PAD))
                                .py(px(6.))
                                .text_size(px(t::BODY))
                                .line_height(px(t::groups::MEMBER_LINE))
                                .font_weight(super::theme::MISANS_REGULAR)
                                .child(name.clone())
                        }))
                        .when(names.is_empty(), |d| {
                            d.child(hint("当前没有节点命中这些关键词", cx).p(px(t::PAD)))
                        }),
                );
            form.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::ROW_GAP))
                    .child(field("关键词", self.input(Field::Keywords, cx), cx))
                    .child(hint(
                        "节点名命中任一关键词就进组,留空 = 所有节点;新订阅也按此自动加入。",
                        cx,
                    ))
                    .child(results.mt(px(t::ROW_GAP))),
            )
        } else {
            let transfer = div()
                .w(px(t::CONTROL))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(t::GAP))
                .children([0, 1].map(|side| {
                    components::icon_button(
                        ("transfer", side),
                        tr(
                            cx,
                            if side == 0 {
                                "加入所选"
                            } else {
                                "移出所选"
                            },
                        ),
                        if side == 0 {
                            "ChevronRight"
                        } else {
                            "ChevronLeft"
                        },
                        t::CONTROL,
                    )
                    .bg(cx.theme().button)
                    .disabled(self.busy || self.checked[side].is_empty())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        // 筛选只改变显示；批量操作按完整原始顺序覆盖隐藏的勾选项。
                        let ids = checked_members(&this.member_pool(side), &this.checked[side]);
                        this.transfer(side, ids, cx);
                    }))
                }));
            form.child(
                div()
                    .flex()
                    .gap(px(t::PAD))
                    .child(self.member_pane(0, cx))
                    .child(transfer)
                    .child(self.member_pane(1, cx)),
            )
        }
    }
    fn member_pane(&self, side: usize, cx: &mut Context<Self>) -> Stateful<Div> {
        let members = self.pane_members(side, cx);
        let failover = self
            .draft
            .as_ref()
            .is_some_and(|d| d.rule == GroupRule::Failover);
        let title = format!(
            "{} ({})",
            tr(cx, if side == 0 { "可选" } else { "已选" }),
            self.member_pool(side).len()
        );
        let head = div()
            .flex()
            .flex_col()
            .gap(px(t::ROW_GAP))
            .px(px(t::GAP))
            .py(px(6.))
            .border_b_1()
            .border_color(cx.theme().foreground.opacity(0.1))
            .child(
                div()
                    .flex()
                    .gap(px(t::GAP))
                    .items_center()
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(t::groups::FILTER_TEXT))
                            .font_weight(super::theme::MISANS_MEDIUM)
                            .child(title),
                    )
                    .when(
                        side == 1
                            && self
                                .draft
                                .as_ref()
                                .is_some_and(|d| d.rule == GroupRule::Failover),
                        |d| {
                            d.child(
                                Select::new(&self.lane_mode, tr(cx, "页签模式"))
                                    .compact()
                                    .disabled(self.busy)
                                    .w(px(t::groups::LANE_MODE_WIDTH))
                                    .h(px(t::groups::LANE_MODE_HEIGHT)),
                            )
                        },
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap(px(t::groups::LANE_SEARCH_GAP))
                            .when(failover && side == 1, |d| {
                                d.child(
                                    icon("MagnifyingGlass", t::groups::LANE_SEARCH_ICON)
                                        .text_color(cx.theme().muted_foreground),
                                )
                            })
                            .child(
                                components::text_input(
                                    &self.inputs[if side == 0 {
                                        Field::Available
                                    } else {
                                        Field::Selected
                                    } as usize],
                                )
                                .flex_1()
                                .min_w_0()
                                .disabled(self.busy)
                                .border_focus()
                                .bg(cx.theme().popover)
                                .px(px(t::groups::SEARCH_PAD))
                                .h(px(t::groups::FILTER_HEIGHT))
                                .text_size(px(t::groups::FILTER_TEXT))
                                .when(failover && side == 1, |i| {
                                    i.border_0().bg(gpui::transparent_black()).px(px(0.))
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::ROW_GAP))
                    .children(["全选", "反选", "清空选择"].into_iter().enumerate().map(
                        |(action, label)| {
                            div().flex_shrink_0().child(
                                group_button(label, tr(cx, label), t::SMALL)
                                    .ghost()
                                    .disabled(self.busy)
                                    .h(px(t::groups::FILTER_HEIGHT))
                                    .px(px(t::GAP))
                                    .text_size(px(12.))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        let members = this.pane_members(side, cx);
                                        for m in
                                            members.iter().filter(|m| m.id != OutboundId::Block)
                                        {
                                            match action {
                                                0 => {
                                                    this.checked[side].insert(m.id.clone());
                                                }
                                                1 => {
                                                    if !this.checked[side].remove(&m.id) {
                                                        this.checked[side].insert(m.id.clone());
                                                    }
                                                }
                                                _ => {
                                                    this.checked[side].remove(&m.id);
                                                }
                                            }
                                        }
                                        cx.notify();
                                    })),
                            )
                        },
                    ))
                    .child(div().flex_1())
                    .child(
                        Select::new(&self.filters[side], tr(cx, "成员筛选"))
                            .compact()
                            .disabled(self.busy)
                            .w(px((tr(cx, &self.filter_values[side]).chars().count()
                                as f32
                                * t::groups::FILTER_TEXT
                                + t::groups::FILTER_EXTRA)
                                .min(t::groups::FILTER_WIDTH)))
                            .h(px(t::groups::FILTER_HEIGHT)),
                    ),
            );
        let rows = div()
            .id(("members", side))
            .min_h_0()
            .max_h(px(t::groups::MEMBER_HEIGHT))
            .overflow_y_scroll()
            .children(members.iter().enumerate().map(|(i, member)| {
                let id = member.id.clone();
                let move_id = id.clone();
                let drag_id = id.clone();
                let drop_id = id.clone();
                let move_button = || {
                    components::icon_button(
                        ("move-member", i),
                        tr(cx, if side == 0 { "加入" } else { "移出" }),
                        if side == 0 {
                            "ChevronRight"
                        } else if failover {
                            "ArrowLeft"
                        } else {
                            "ChevronLeft"
                        },
                        t::groups::FILTER_HEIGHT,
                    )
                    .disabled(self.busy || move_id == OutboundId::Block)
                    .on_click(cx.listener({
                        let move_id = move_id.clone();
                        move |this, _, _, cx| this.transfer(side, vec![move_id.clone()], cx)
                    }))
                };
                let checked = self.checked[side].contains(&id);
                let blocked = id == OutboundId::Block;
                div()
                    .id((
                        if side == 0 {
                            "available-member"
                        } else {
                            "selected-member"
                        },
                        i,
                    ))
                    .flex()
                    .min_h(px(t::groups::MEMBER_ROW))
                    .text_size(px(t::BODY))
                    .line_height(px(t::groups::MEMBER_LINE))
                    .font_weight(super::theme::MISANS_REGULAR)
                    .items_center()
                    .gap(px(t::GAP))
                    .px(px(t::PAD))
                    .py(px(6.))
                    .hover(|d| d.bg(cx.theme().button))
                    .opacity(if blocked { 0.45 } else { 1. })
                    .when(side == 1 && !self.busy, |d| {
                        d.on_drag(
                            MemberDrag {
                                id: drag_id,
                                name: member.name.clone(),
                            },
                            |drag, _, _, cx| cx.new(|_| drag.clone()),
                        )
                        .on_drop(cx.listener(
                            move |this, drag: &MemberDrag, _, cx| {
                                if !this.busy
                                    && let Some(draft) = &mut this.draft
                                {
                                    let members = if draft.rule == GroupRule::Failover {
                                        &mut draft.lanes[this.active_lane].members
                                    } else {
                                        &mut draft.members
                                    };
                                    reorder_member(members, &drag.id, &drop_id);
                                    cx.notify();
                                }
                            },
                        ))
                    })
                    .child(
                        gpui_kit::base::Button::new(("member-check", i))
                            .size(px(t::ICON_SMALL))
                            .flex_shrink_0()
                            .disabled(self.busy || blocked)
                            .accessibility_label(member.name.clone())
                            .child(
                                div()
                                    .size(px(t::ICON_SMALL))
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(px(t::groups::MEMBER_CHECK_RADIUS))
                                    .when(checked, |d| {
                                        d.bg(rgb(t::ACCENT)).border_color(rgb(t::ACCENT)).child(
                                            icon("Check", t::groups::FILTER_TEXT)
                                                .text_color(rgb(t::groups::CHECK_COLOR)),
                                        )
                                    }),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if !this.checked[side].remove(&id) {
                                    this.checked[side].insert(id.clone());
                                }
                                cx.notify();
                            })),
                    )
                    .when(side == 1 && !failover, |d| {
                        d.child(
                            div()
                                .flex_shrink_0()
                                .w(px(t::groups::FILTER_HEIGHT))
                                .child(move_button()),
                        )
                    })
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .text_ellipsis()
                            .child(member.name.clone()),
                    )
                    .when(
                        side == 0 && !matches!(member.id, OutboundId::Node(_)),
                        |d| {
                            d.child(
                                div()
                                    .flex_shrink_0()
                                    .h(px(t::groups::MEMBER_BADGE_HEIGHT))
                                    .px(px(t::groups::MEMBER_BADGE_PAD))
                                    .rounded(px(t::ROW_GAP))
                                    .bg(cx.theme().button)
                                    .text_size(px(t::groups::MEMBER_BADGE_TEXT))
                                    .line_height(px(t::groups::MEMBER_BADGE_HEIGHT))
                                    .child(tr(cx, "出站节点")),
                            )
                        },
                    )
                    .when(side == 0 || failover, |d| {
                        d.child(div().flex_1()).child(move_button())
                    })
            }))
            .when(members.is_empty(), |d| {
                d.child(
                    hint(
                        if side == 0 {
                            "没有匹配的条目。"
                        } else {
                            "勾选左边的条目,按中间的箭头加进来。"
                        },
                        cx,
                    )
                    .p(px(t::PAD))
                    .text_center(),
                )
            });
        div()
            .id(("member-pane", side))
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .rounded(px(t::POPOVER_RADIUS))
            .border_1()
            .border_color(cx.theme().foreground.opacity(0.1))
            .when(
                side == 1
                    && self
                        .draft
                        .as_ref()
                        .is_some_and(|d| d.rule == GroupRule::Failover),
                |d| d.child(self.lane_header(cx)),
            )
            .child(head)
            .child(rows)
    }
    fn modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let builtin = self.draft.as_ref().is_some_and(NodeGroup::builtin);
        let editing = self.draft.is_some();
        let busy = self.busy;
        let cannot_save =
            busy || self.draft.as_ref().is_some_and(|d| {
                self.value(Field::Name, cx).trim().is_empty()
                    || (!d.builtin()
                        && d.rule != GroupRule::Failover
                        && d.mode == GroupMode::Static
                        && d.members.is_empty())
                    || (d.rule == GroupRule::Failover
                        && (d.lanes.len() < if self.original_failover { 1 } else { 2 }
                            || d.lanes.iter().any(|l| l.members.is_empty())))
            }) || (self.auto_open
                && (self.auto_codes.is_empty() || !self.auto_rules.iter().any(|v| *v)));
        let title = if editing {
            if self.state.as_ref().is_some_and(|s| {
                s.groups
                    .iter()
                    .any(|g| Some(&g.id) == self.draft.as_ref().map(|d| &d.id))
            }) {
                "修改分组"
            } else {
                "添加分组"
            }
        } else if self.auto_open {
            "按国家自动分组"
        } else if matches!(self.confirmation, Some(Confirmation::Delete(_))) {
            "删除分组"
        } else {
            "恢复默认"
        };
        let body = if editing {
            self.editor(cx)
        } else if self.auto_open {
            self.auto_editor(cx)
        } else {
            hint(
                if matches!(self.confirmation, Some(Confirmation::Delete(_))) {
                    "删除此组？引用需先解除"
                } else {
                    "恢复默认组？自建组将移除"
                },
                cx,
            )
        };
        let mut surface = div()
            .font_weight(super::theme::MISANS_REGULAR)
            .flex()
            .flex_col()
            .w(px(if editing {
                t::groups::MODAL_WIDTH
            } else {
                t::groups::AUTO_WIDTH
            })
            .min(window.viewport_size().width - px(t::CONTROL)))
            .max_h(window.viewport_size().height * 0.9)
            .rounded(px(t::POPOVER_RADIUS))
            .bg(cx.theme().popover)
            .overflow_hidden();
        if editing && !builtin {
            surface = surface.h(window.viewport_size().height * 0.9);
        }
        surface = surface
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(t::groups::HEADER))
                    .flex_shrink_0()
                    .px(px(t::SECTION_PADDING))
                    .border_b_1()
                    .border_color(cx.theme().foreground.opacity(0.1))
                    .child(
                        div()
                            .text_size(px(t::SECTION_TITLE))
                            .font_weight(FontWeight(600.))
                            .child(tr(cx, title)),
                    )
                    .child(
                        components::icon_button(
                            "group-close",
                            tr(cx, "关闭"),
                            "XMark",
                            t::SWITCH_HEIGHT,
                        )
                        .disabled(busy)
                        .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    ),
            )
            .child(
                div()
                    .id("group-editor-body")
                    .min_h_0()
                    .when(editing && !builtin, |d| d.flex_1())
                    .overflow_y_scroll()
                    .p(px(t::SECTION_PADDING))
                    .child(body),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(t::groups::FOOTER_GAP))
                    .min_h(px(t::groups::FOOTER))
                    .flex_shrink_0()
                    .px(px(t::SECTION_PADDING))
                    .py(px(t::GAP))
                    .border_t_1()
                    .border_color(cx.theme().foreground.opacity(0.1))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(t::SMALL))
                            .text_color(cx.theme().danger)
                            .when_some(self.error.as_ref(), |d, e| {
                                d.child(error_text(
                                    e,
                                    self.state.as_deref(),
                                    self.draft.as_ref(),
                                    cx.global::<super::i18n::Locale>().0,
                                ))
                            }),
                    )
                    .child(
                        group_button("group-cancel", tr(cx, "取消"), t::BODY)
                            .px(px(t::groups::FOOTER_BUTTON_PAD))
                            .font_weight(super::theme::MISANS_SEMIBOLD)
                            .disabled(busy)
                            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx))),
                    )
                    .child(
                        group_button(
                            "group-save",
                            if busy {
                                tr(cx, "保存中").to_owned()
                            } else if self.auto_open {
                                tr(cx, "生成 {count} 个分组").replace(
                                    "{count}",
                                    &(self.auto_codes.len()
                                        * self.auto_rules.iter().filter(|v| **v).count())
                                    .to_string(),
                                )
                            } else {
                                tr(cx, if editing { "保存" } else { "确定" }).to_owned()
                            },
                            t::groups::PRIMARY_TEXT,
                        )
                        .primary()
                        .px(px(t::groups::FOOTER_BUTTON_PAD))
                        .bg(rgb(t::ACCENT_STRONG))
                        .text_color(rgb(0xffffff))
                        .when(busy, |b| b.child(components::loading_spinner(t::BODY, cx)))
                        .font_weight(super::theme::MISANS_SEMIBOLD)
                        .disabled(cannot_save)
                        .on_click(cx.listener(|this, _, _, cx| {
                            if this.draft.is_some() {
                                this.save_draft(cx);
                                return;
                            }
                            let Some(state) = &this.state else { return };
                            let groups = if this.auto_open {
                                let rules = [GroupRule::UrlTest, GroupRule::Selector]
                                    .into_iter()
                                    .zip(this.auto_rules)
                                    .filter(|(_, enabled)| *enabled)
                                    .map(|(r, _)| r)
                                    .collect::<Vec<_>>();
                                auto_groups(&state.groups, &this.auto_codes, &rules)
                            } else {
                                match &this.confirmation {
                                    Some(Confirmation::Restore) => default_groups(),
                                    Some(Confirmation::Delete(id)) => state
                                        .groups
                                        .iter()
                                        .filter(|g| &g.id != id)
                                        .cloned()
                                        .collect(),
                                    None => return,
                                }
                            };
                            this.send(Some(groups), cx);
                        })),
                    ),
            );
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.focus.clone())
            .close_on_escape(!busy)
            .close_on_backdrop_press(!busy)
            .on_ok(|_, _, _| false)
            .on_cancel(move |_, _, _| !busy)
            .backdrop(div().size_full().bg(rgba(0x00000066)))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|this, _, window, cx| this.close(window, cx)))
            .into_any_element()
    }
    fn scale(&mut self, delta: i32, window: &mut Window, cx: &mut Context<Self>) {
        if let Ok(value) = self.value(Field::Scale, cx).parse::<i32>() {
            self.inputs[Field::Scale as usize].update(cx, |i, cx| {
                i.set_value(value.saturating_add(delta).to_string(), window, cx)
            });
            cx.notify();
        }
    }
    fn auto_editor(&self, cx: &mut Context<Self>) -> Div {
        let entity = cx.entity();
        let picker = Popover::new("country-add")
            .appearance(false)
            .anchor(Anchor::TopLeft)
            .track_focus(&self.inputs[Field::Country as usize].focus_handle(cx))
            .trigger(
                gpui_kit::base::Button::new("add-country")
                    .flex()
                    .items_center()
                    .gap(px(t::GAP))
                    .h(px(t::CONTROL))
                    .px(px(t::PAD))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(t::RADIUS))
                    .bg(cx.theme().input)
                    .w(px(t::groups::COUNTRY_PICKER))
                    .disabled(self.busy)
                    .child(icon("GlobeAlt", t::ICON_SMALL))
                    .child(div().flex_1().child(tr(cx, "添加国家/地区")))
                    .child(icon("ChevronDown", t::BODY)),
            )
            .content(move |_, _, cx| {
                let popup = cx.entity();
                let dismiss = move |window: &mut Window, cx: &mut App| {
                    popup.update(cx, |s, cx| s.dismiss(window, cx))
                };
                entity.update(cx, |this, cx| this.country_picker(dismiss, cx))
            });
        div()
            .flex()
            .flex_col()
            .gap(px(t::SECTION_PADDING))
            .child(field(
                "要建哪种组（可以都建）",
                div().flex().gap(px(t::SECTION_PADDING)).children(
                    ["自动择优（url-test）", "手动选择（select）"]
                        .into_iter()
                        .enumerate()
                        .map(|(i, label)| {
                            gpui_kit::base::Button::new(label)
                                .disabled(self.busy)
                                .flex()
                                .items_center()
                                .gap(px(t::GAP))
                                .accessibility_label(tr(cx, label))
                                .child(
                                    div()
                                        .size(px(t::ICON))
                                        .rounded(px(t::groups::RULE_CHECK_RADIUS))
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .when(self.auto_rules[i], |d| {
                                            d.bg(rgb(t::ACCENT_STRONG))
                                                .border_color(rgb(t::ACCENT_STRONG))
                                                .child(
                                                    icon("Check", t::ICON)
                                                        .text_color(rgb(0xffffff)),
                                                )
                                        }),
                                )
                                .child(tr(cx, label))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.auto_rules[i] = !this.auto_rules[i];
                                    cx.notify();
                                }))
                        }),
                ),
                cx,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::ROW_GAP))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(t::GAP))
                            .min_h(px(t::CONTROL))
                            .child(div().text_size(px(t::BODY)).child(tr(cx, "选择国家/地区")))
                            .child(div().flex_1())
                            .child(picker)
                            .child(
                                group_button(
                                    "clear-countries",
                                    tr(cx, "清空选择"),
                                    t::SMALL,
                                )
                                .ghost()
                                .disabled(self.busy || self.auto_codes.is_empty())
                                .text_size(px(t::SMALL))
                                .on_click(cx.listener(
                                    |this, _, _, cx| {
                                        this.auto_codes.clear();
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .id("auto-countries")
                            .when(self.auto_codes.is_empty(), |d| {
                                d.child(hint("还没有选国家/地区,用右上角的下拉框添加。", cx).p(px(t::SECTION_PADDING)))
                            })
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(px(t::POPOVER_RADIUS))
                            .max_h(px(t::groups::MEMBER_HEIGHT))
                            .overflow_y_scroll()
                            .children(self.auto_codes.iter().enumerate().map(|(index, code)| {
                                let country =
                                    group_countries().iter().find(|c| &c.code == code).unwrap();
                                let count = self.country_count(country);
                                div()
                                    .id(("country", index))
                                    .flex()
                                    .items_center()
                                    .gap(px(t::GAP))
                                    .px(px(t::PAD))
                                    .min_h(px(t::groups::MEMBER_ROW))
                                    .on_drag(
                                        CountryDrag {
                                            index,
                                            name: tr(cx, &country.name).to_owned(),
                                        },
                                        |d, _, _, cx| cx.new(|_| d.clone()),
                                    )
                                    .on_drop(cx.listener(move |this, drag: &CountryDrag, _, cx| {
                                        if !this.busy {
                                            let code = this.auto_codes.remove(drag.index);
                                            this.auto_codes.insert(index, code);
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        icon("Bars3", t::ICON_SMALL)
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(super::components::icon_picker::country_icon(code, cx))
                                    .child(div().flex_1().child(tr(cx, &country.name).to_owned()))
                                    .child(hint(&format!("{count} {}", tr(cx, "个节点")), cx))
                                    .child(
                                        components::icon_button_content(
                                            ("remove-country", index),
                                            tr(cx, "移除"),
                                            t::SWITCH_HEIGHT, icon("Trash", t::BODY),
                                        )
                                        .disabled(self.busy)
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                this.auto_codes.remove(index);
                                                cx.notify();
                                            }),
                                        ),
                                    )
                            })),
                    ),
            )
            .child(hint("按「国家-自动」「国家-手动」命名并配国旗;动态组,以后新订阅里这个国家的节点自动进组。", cx))
    }
    fn country_count(&self, country: &GroupCountry) -> usize {
        self.state.as_ref().map_or(0, |s| {
            s.group_available_nodes()
                .filter(|n| {
                    country
                        .keywords
                        .iter()
                        .any(|k| group_keyword_matches(&n.name, k))
                })
                .count()
        })
    }
    fn country_picker(
        &self,
        dismiss: impl Fn(&mut Window, &mut App) + Clone + 'static,
        cx: &mut Context<Self>,
    ) -> Div {
        let search = self.value(Field::Country, cx).to_lowercase();
        let mut countries = group_countries()
            .iter()
            .filter(|c| {
                !self.auto_codes.contains(&c.code)
                    && self.country_count(c) > 0
                    && (search.is_empty()
                        || tr(cx, &c.name).to_lowercase().contains(&search)
                        || c.code.to_lowercase().contains(&search))
            })
            .collect::<Vec<_>>();
        countries.sort_by_key(|c| std::cmp::Reverse(self.country_count(c)));
        div()
            .w(px(t::groups::COUNTRY_MENU))
            .p(px(t::GAP))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(t::RADIUS))
            .bg(cx.theme().popover)
            .child(self.input(Field::Country, cx))
            .child(
                div()
                    .id("country-options")
                    .max_h(px(t::groups::MEMBER_HEIGHT))
                    .overflow_y_scroll()
                    .children(countries.iter().map(|country| {
                        let code = country.code.clone();
                        let dismiss = dismiss.clone();
                        div().id(SharedString::from(code.clone())).child(
                            components::button("country-option", "")
                                .accessibility_label(tr(cx, &country.name).to_owned())
                                .child(
                                    div()
                                        .w_full()
                                        .flex()
                                        .items_center()
                                        .gap(px(t::GAP))
                                        .text_size(px(t::BODY))
                                        .font_weight(super::theme::MISANS_REGULAR)
                                        .child(super::components::icon_picker::country_icon(
                                            &country.code,
                                            cx,
                                        ))
                                        .child(
                                            div().flex_1().child(tr(cx, &country.name).to_owned()),
                                        )
                                        .child(hint(&self.country_count(country).to_string(), cx)),
                                )
                                .ghost()
                                .w_full()
                                .justify_start()
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.auto_codes.push(code.clone());
                                    dismiss(window, cx);
                                    cx.notify();
                                })),
                        )
                    }))
                    .when(countries.is_empty(), |d| {
                        d.child(hint("暂无匹配节点", cx).p(px(t::GAP)))
                    }),
            )
    }
}
impl Render for GroupsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 回调订阅随页面实体持有，语言切换不重建输入或清空草稿。
        let language = cx.global::<super::i18n::Locale>().0;
        if self.language != language {
            self.language = language;
            for (input, label) in self.inputs.iter().zip(PLACEHOLDERS) {
                input.update(cx, |i, cx| i.set_placeholder(tr(cx, label), window, cx));
            }
        }

        let mut root = div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(t::GAP))
            .font_weight(super::theme::MISANS_REGULAR);
        if let Some(error) = &self.error
            && self.draft.is_none()
            && self.confirmation.is_none()
            && !self.auto_open
        {
            root = root.child(div().text_color(cx.theme().danger).child(error_text(
                error,
                self.state.as_deref(),
                self.draft.as_ref(),
                cx.global::<super::i18n::Locale>().0,
            )));
        }
        if self.load_error {
            root = root.child(
                group_button("groups-retry", tr(cx, "重试"), t::BODY)
                    .on_click(cx.listener(|this, _, _, cx| this.send(None, cx))),
            );
        }
        if self.loading {
            root = root.child(
                div()
                    .min_h(px(t::groups::LOADING_HEIGHT))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(components::loading_spinner(t::groups::LOADING_ICON, cx)),
            );
        }
        if let Some(state) = &self.state
            && !self.loading
        {
            let state = state.clone();
            let radius = state.app_config.visual.global_radius as f32;
            root = root.child(
                div()
                    .id("group-list")
                    .track_scroll(&self.scroll)
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .gap(px(t::GAP))
                    .when(state.groups.is_empty(), |d| {
                        d.child(hint("暂无节点组，点击添加", cx))
                    })
                    .children(state.groups.iter().enumerate().map(|(index, group)| {
                        let edit = group.clone();
                        let enabled = group.enabled;
                        let id = group.id.clone();
                        let delete = id.clone();
                        let count = state.group_members(group).len();
                        let description =
                            if !group.builtin() && group.mode == GroupMode::Dynamic && count == 0 {
                                tr(cx, "空动态组不参与编译").to_owned()
                            } else if group.rule == GroupRule::Direct {
                                tr(cx, "流量不经代理,直接从路由器出去。内核离不开它,停用只是站点集里选不到。").to_owned()
                            } else if group.rule == GroupRule::Block {
                                tr(cx, "命中的流量直接丢弃。").to_owned()
                            } else if group.rule == GroupRule::Failover {
                                tr(cx,"故障转移 · {lanes} 个页签 · {count} 个节点")
                                    .replace("{lanes}",&group.lanes.len().to_string())
                                    .replace("{count}",&group.lanes.iter().flat_map(|l| &l.members).collect::<HashSet<_>>().len().to_string())
                            } else {
                                format!(
                                    "{} · {}",
                                    tr(
                                        cx,
                                        if group.mode == GroupMode::Static {
                                            "静态组"
                                        } else {
                                            "动态组"
                                        }
                                    ),
                                    tr(cx, "当前 {count} 个节点").replace("{count}", &count.to_string())
                                )
                            };
                        let description = if group.rule == GroupRule::UrlTest {
                            format!("{description} · {} {}s · {} {}ms", tr(cx, "检测间隔"), group.interval_secs, tr(cx, "容差"), group.tolerance_ms)
                        } else { description };
                        div()
                            .id(("group-card", index))
                            .flex()
                            .items_center()
                            .gap(px(t::GAP))
                            .min_h(px(t::groups::CARD_HEIGHT))
                            .px(px(t::PAD))
                            .py(px(t::groups::CARD_PAD_Y))
                            .border_1()
                            .border_color(cx.theme().foreground.opacity(0.1))
                            .rounded(px(radius))
                            .bg(cx.theme().popover)
                            .opacity(if enabled { 1. } else { 0.52 })
                            .on_drag(
                                GroupDrag {
                                    index,
                                    name: group.name.clone(),
                                },
                                |g, _, _, cx| cx.new(|_| g.clone()),
                            )
                            .on_drop(cx.listener(move |this, drag: &GroupDrag, _, cx| {
                                if !this.busy
                                    && let Some(state) = &this.state
                                {
                                    let mut groups = state.groups.clone();
                                    let item = groups.remove(drag.index);
                                    groups.insert(index, item);
                                    this.send(Some(groups), cx);
                                }
                            }))
                            .child(
                                icon("Bars3", t::groups::DRAG_ICON)
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(group_icon(
                                &group.icon,
                                (t::groups::CARD_ICON + group.icon_scale as f32).max(12.),
                                cx,
                            ))
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(t::GAP))
                                            .child(
                                                div()
                                                    .text_size(px(t::BODY))
                                                    .font_weight(super::theme::MISANS_MEDIUM)
                                                    .child(group.name.clone()),
                                            )
                                            .child(
                                                div()
                                                    .px(px(t::groups::BADGE_PAD_X))
                                                    .py(px(t::groups::BADGE_PAD_Y))
                                                    .line_height(px(t::groups::BADGE_LINE))
                                                    .text_color(cx.theme().muted_foreground)
                                                    .rounded(px(t::groups::BADGE_RADIUS))
                                                    .bg(cx.theme().foreground.opacity(0.05))
                                                    .text_size(px(10.))
                                                    .child(tr(
                                                        cx,
                                                        match group.rule {
                                                            GroupRule::Direct
                                                            | GroupRule::Block => "内置",
                                                            GroupRule::Selector => {
                                                                "手动选择（select）"
                                                            }
                                                            GroupRule::UrlTest => {
                                                                "自动择优（url-test）"
                                                            }
                                                            GroupRule::Failover => "故障转移（failover）",
                                                        },
                                                    )),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .mt(px(t::groups::DESCRIPTION_TOP))
                                            .text_size(px(11.))
                                            .line_height(px(16.))
                                            .text_color(cx.theme().muted_foreground)
                                            .child(description),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap(px(t::ROW_GAP))
                                    .child(
                                        components::icon_button(
                                            ("toggle-group", index),
                                            tr(cx, if enabled { "停用" } else { "启用" }),
                                            "Power",
                                            t::groups::CARD_ACTION,
                                        )
                                        .disabled(self.busy)
                                        .when(enabled, |b| b.text_color(rgb(t::ACCENT_STRONG)))
                                        .on_click(
                                            cx.listener(move |this, _, _, cx| {
                                                if let Some(state) = &this.state {
                                                    let mut groups = state.groups.clone();
                                                    if let Some(g) =
                                                        groups.iter_mut().find(|g| g.id == id)
                                                    {
                                                        g.enabled = !g.enabled;
                                                    }
                                                    this.send(Some(groups), cx);
                                                }
                                            }),
                                        ),
                                    )
                                    .child(
                                        components::icon_button(
                                            ("edit-group", index),
                                            tr(cx, "修改分组"),
                                            "PencilSquare",
                                            t::groups::CARD_ACTION,
                                        )
                                        .disabled(self.busy)
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                this.edit(edit.clone(), window, cx)
                                            }),
                                        ),
                                    )
                                    .child(
                                        components::icon_button(
                                            ("delete-group", index),
                                            tr(cx, "删除"),
                                            "Trash",
                                            t::groups::CARD_ACTION,
                                        )
                                        .disabled(self.busy || group.builtin())
                                        .on_click(
                                            cx.listener(move |this, _, window, cx| {
                                                this.trigger = window.focused(cx);
                                                this.confirmation =
                                                    Some(Confirmation::Delete(delete.clone()));
                                                this.focus.focus(window, cx);
                                                cx.notify();
                                            }),
                                        ),
                                    ),
                            )
                    })),
            );
        }
        if self.draft.is_some() || self.confirmation.is_some() || self.auto_open {
            root = root.child(self.modal(window, cx));
        }
        if self.deleting_lane || self.discard_confirm {
            root = root.child(self.lane_confirmation(cx));
        }
        root
    }
}
fn field(label: &str, control: impl IntoElement, cx: &App) -> Div {
    div()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(t::ROW_GAP))
        .child(
            div()
                .text_size(px(12.))
                .line_height(px(16.))
                .font_weight(super::theme::MISANS_MEDIUM)
                .child(tr(cx, label).to_owned()),
        )
        .child(control)
}
fn hint(label: &str, cx: &App) -> Div {
    div()
        .text_size(px(12.))
        .line_height(px(16.))
        .text_color(cx.theme().muted_foreground)
        .child(tr(cx, label).to_owned())
}
fn comma_values(value: &str) -> Vec<String> {
    value
        .split([',', '，', '、'])
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_owned)
        .collect()
}
fn error_text(
    error: &GroupSaveError,
    state: Option<&AppState>,
    draft: Option<&NodeGroup>,
    language: DesktopLanguage,
) -> String {
    let key = match error {
        GroupSaveError::Storage(e) => {
            if e.code() == AppErrorCode::RevisionConflict {
                "配置已更新，请检查后重试"
            } else {
                "保存失败，草稿已保留"
            }
        }
        GroupSaveError::Invalid(issue) => match issue {
            GroupIssue::DuplicateMember(_) => "成员重复",
            GroupIssue::DuplicateName(_) => "分组名称重复",
            GroupIssue::InvalidName(_) => "请输入分组名称",
            GroupIssue::EmptyStatic(_) => "静态组至少添加一个成员",
            GroupIssue::InvalidLane { .. } => "每条线路至少添加一个成员，页签标识不能重复",
            GroupIssue::InvalidSettings(_) => "请检查检测参数",
            GroupIssue::BlockMember(_) => "拒绝暂不能作为组成员",
            GroupIssue::Graph(_) => "分组引用无效，请检查成员",
            GroupIssue::Unavailable { .. } | GroupIssue::Referenced(_) => "引用出口不可用",
            GroupIssue::DuplicateId(_) => "分组标识重复",
        },
    };
    let label = super::i18n::translate(language, key);
    let name = |id: &OutboundId| -> String {
        if let Some(group) = draft
            .filter(|g| g.outbound_id() == *id)
            .or_else(|| state.and_then(|s| s.groups.iter().find(|g| g.outbound_id() == *id)))
            && !group.name.is_empty()
        {
            return group.name.clone();
        }
        match id {
            OutboundId::Pool(id) => state
                .and_then(|s| s.pools.iter().find(|p| p.id == *id))
                .map_or_else(|| id.0.clone(), |p| p.name.clone()),
            OutboundId::Node(id) => state
                .and_then(|s| s.nodes.iter().find(|n| n.id == *id))
                .map_or_else(|| id.0.clone(), |n| n.name.clone()),
            OutboundId::Direct => super::i18n::translate(language, "直连").into(),
            OutboundId::Block => super::i18n::translate(language, "拒绝").into(),
        }
    };
    let subject = match error {
        GroupSaveError::Invalid(GroupIssue::Graph(graph)) => match graph {
            OutboundGraphError::Dangling { source, target } => {
                format!("{} → {}", name(source), name(target))
            }
            OutboundGraphError::Cycle(ids) => ids.iter().map(name).collect::<Vec<_>>().join(" → "),
            OutboundGraphError::SelfReference(id) | OutboundGraphError::Duplicate(id) => name(id),
        },
        GroupSaveError::Invalid(GroupIssue::Unavailable { group, member }) => format!(
            "{} → {}",
            name(&OutboundId::Pool(group.clone())),
            name(member)
        ),
        GroupSaveError::Invalid(GroupIssue::Referenced(id)) => name(id),
        GroupSaveError::Invalid(GroupIssue::DuplicateName(name)) => name.clone(),
        _ => String::new(),
    };
    if subject.is_empty() {
        label.into()
    } else {
        format!("{label}：{subject}")
    }
}
// 搜索只改变可见集合；拖拽仍按稳定 ID 更新完整成员顺序。
// 批量范围与筛选显示分离，HashSet 的迭代顺序不参与持久成员顺序。
fn checked_members(pool: &[Member], checked: &HashSet<OutboundId>) -> Vec<OutboundId> {
    pool.iter()
        .filter(|m| checked.contains(&m.id))
        .map(|m| m.id.clone())
        .collect()
}
fn transfer_members(
    members: &mut Vec<OutboundId>,
    checked: &mut HashSet<OutboundId>,
    side: usize,
    ids: Vec<OutboundId>,
) {
    for id in ids {
        if id == OutboundId::Block {
            continue;
        }
        if side == 0 {
            if !members.contains(&id) {
                members.push(id.clone());
            }
        } else {
            members.retain(|m| m != &id);
        }
        checked.remove(&id);
    }
}

fn reorder_member(members: &mut Vec<OutboundId>, source: &OutboundId, target: &OutboundId) {
    if let (Some(from), Some(to)) = (
        members.iter().position(|m| m == source),
        members.iter().position(|m| m == target),
    ) {
        let member = members.remove(from);
        members.insert(to, member);
    }
}
const PLACEHOLDERS: [&str; 13] = [
    "分组名称",
    "秒",
    "毫秒",
    "留空使用全局地址",
    "关键词,用逗号分隔,如:香港,hk",
    "按名称过滤…",
    "按名称过滤…",
    "搜索地区",
    "缩放",
    "页签名（可选）",
    "秒",
    "连续失败次数",
    "秒",
];

/// 与全局快照对齐，拒绝晚到的旧版本；错误后仍保留草稿供用户检查。
pub fn reconcile_completion(
    current: Option<&AppState>,
    result: Result<Box<AppState>, GroupSaveError>,
    snapshot: Option<Box<AppState>>,
) -> (Result<Box<AppState>, GroupSaveError>, Option<Box<AppState>>) {
    let stale = |state: &AppState| {
        current.is_some_and(|c| {
            state.state_epoch != c.state_epoch
                || state.config_revision < c.config_revision
                || state.selection_revision < c.selection_revision
        })
    };
    if result.as_ref().is_ok_and(|state| stale(state)) {
        return (
            Err(GroupSaveError::Storage(AppError::new(
                AppErrorCode::RevisionConflict,
            ))),
            current.cloned().map(Box::new),
        );
    }
    let snapshot = if snapshot.as_ref().is_some_and(|s| stale(s)) {
        current.cloned().map(Box::new)
    } else {
        snapshot
    };
    (result, snapshot)
}

// Kit label 有固定字号；显式内容仍复用全局按钮的行为、焦点和主题。
fn group_button(
    id: &'static str,
    label: impl Into<SharedString>,
    size: f32,
) -> components::PanelButton {
    let label = label.into();
    components::button(id, "")
        .accessibility_label(label.clone())
        .font_weight(super::theme::MISANS_REGULAR)
        .child(div().text_size(px(size)).child(label))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[::core::prelude::v1::test]
    fn p402_bulk_transfer_keeps_hidden_checks_and_source_order() {
        // 勾选顺序/搜索都不能改变批量范围；加入和移出后仅清理已处理勾选。
        let pool: Vec<Member> = ["a", "b", "c"]
            .into_iter()
            .map(|name| Member {
                id: OutboundId::Node(NodeId(name.into())),
                name: name.into(),
                subscription: "fixture".into(),
            })
            .collect();
        let mut checked = HashSet::from([pool[2].id.clone(), pool[0].id.clone()]);
        let visible = pool.iter().filter(|m| m.name == "b").collect::<Vec<_>>();
        assert_eq!((pool.len(), visible.len()), (3, 1));
        let ids = checked_members(&pool, &checked);
        let mut selected = vec![OutboundId::Direct];
        transfer_members(&mut selected, &mut checked, 0, ids);
        assert_eq!(
            selected,
            vec![OutboundId::Direct, pool[0].id.clone(), pool[2].id.clone()]
        );
        assert!(checked.is_empty());
        checked.extend([pool[0].id.clone(), pool[2].id.clone()]);
        let ids = checked_members(&pool, &checked);
        transfer_members(&mut selected, &mut checked, 1, ids);
        assert_eq!(selected, vec![OutboundId::Direct]);
        assert!(checked.is_empty());
    }
    #[::core::prelude::v1::test]
    fn p402_member_reorder_uses_ids_when_filtered() {
        // 筛选出 A/C 时把 C 拖到 A，隐藏的 B 仍保留且保存顺序可预期。
        let ids = ["a", "b", "c"].map(|s| OutboundId::Node(NodeId(s.into())));
        let mut members = ids.to_vec();
        reorder_member(&mut members, &ids[2], &ids[0]);
        assert_eq!(
            members,
            vec![ids[2].clone(), ids[0].clone(), ids[1].clone()]
        );
    }
    #[::core::prelude::v1::test]
    fn p402_editor_placeholders_follow_language() {
        // 保护实际英文编辑器：全部输入占位与单位都通过全局语言表翻译。
        for key in PLACEHOLDERS {
            assert_ne!(
                super::super::i18n::translate(DesktopLanguage::English, key),
                key
            );
        }
        assert_eq!(
            super::super::i18n::translate(DesktopLanguage::TraditionalChinese, "搜索节点"),
            "搜尋節點"
        );
    }
    #[::core::prelude::v1::test]
    fn p402_late_completion_preserves_new_config_selection_and_epoch() {
        // 保护用户已显示的新配置/选择，晚到的组保存结果不能回退全局快照。
        let old = AppState::empty();
        for current in [
            AppState {
                config_revision: old.config_revision + 1,
                ..old.clone()
            },
            AppState {
                selection_revision: old.selection_revision + 1,
                ..old.clone()
            },
            AppState::empty(),
        ] {
            let (result, snapshot) =
                reconcile_completion(Some(&current), Ok(Box::new(old.clone())), None);
            assert!(result.is_err());
            assert_eq!(snapshot.as_deref(), Some(&current));
        }
        assert!(
            reconcile_completion(Some(&old), Ok(Box::new(old.clone())), None)
                .0
                .is_ok()
        );
    }
    #[::core::prelude::v1::test]
    fn p402_editor_labels_translate_in_existing_languages() {
        // 保护真实三语编辑器：共用控件与加载/错误标签不回退为中文。
        for (key, english, traditional) in [
            ("图标", "Icon", "圖示"),
            ("无", "None", "無"),
            ("移除", "Remove", "移除"),
            ("重试", "Retry", "重試"),
            ("加载中", "Loading", "載入中"),
        ] {
            assert_eq!(
                super::super::i18n::translate(DesktopLanguage::English, key),
                english
            );
            assert_eq!(
                super::super::i18n::translate(DesktopLanguage::TraditionalChinese, key),
                traditional
            );
        }
    }
    #[::core::prelude::v1::test]
    fn p402_reference_error_identifies_source_and_missing_member() {
        // 删除被引用的成员时，用户能从反馈找到需修正的组；稳定 ID 是缺失名称的事实。
        let mut state = AppState::empty();
        let mut group = NodeGroup::new(PoolId("a".into()));
        group.name = "工作".into();
        state.groups.push(group);
        let error = GroupSaveError::Invalid(GroupIssue::Graph(OutboundGraphError::Dangling {
            source: OutboundId::Pool(PoolId("a".into())),
            target: OutboundId::Node(NodeId("missing".into())),
        }));
        let text = error_text(&error, Some(&state), None, DesktopLanguage::English);
        assert!(text.contains("工作 → missing"));
        assert!(!text.contains("分组引用无效"));
    }
}

mod failover_editor;
