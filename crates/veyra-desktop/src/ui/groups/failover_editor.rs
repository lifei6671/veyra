//! 主备编辑复用 GroupsView 的真实草稿、成员双栏与保存链路。
//! 视觉数字均追溯至 OpenBox .group-failover-*，不接管 Runtime 的选择写权。
use super::*;
impl GroupsView {
    pub(super) fn active_lane_mut(&mut self) -> Option<&mut FailoverLane> {
        self.draft
            .as_mut()
            .filter(|d| d.rule == GroupRule::Failover)?
            .lanes
            .get_mut(self.active_lane)
    }
    pub(super) fn draft_members(&self) -> &[OutboundId] {
        match &self.draft {
            Some(d) if d.rule == GroupRule::Failover => d
                .lanes
                .get(self.active_lane)
                .map(|l| l.members.as_slice())
                .unwrap_or_default(),
            Some(d) => &d.members,
            None => &[],
        }
    }
    pub(super) fn change_rule(
        &mut self,
        rule: GroupRule,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.draft.as_ref() else {
            return;
        };
        if current.rule == rule {
            return;
        }
        let mut next = current.clone();
        if rule == GroupRule::Failover {
            self.previous_plain = Some(current.clone());
            if let Some(previous) = &self.previous_failover {
                next.lanes = previous.lanes.clone();
                next.failover = previous.failover.clone();
            } else {
                let lanes = (0..2)
                    .map(|_| FailoverLane::fresh())
                    .collect::<Result<Vec<_>, _>>();
                match lanes {
                    Ok(lanes) => next.lanes = lanes,
                    Err(e) => {
                        self.error = Some(GroupSaveError::Storage(e));
                        cx.notify();
                        return;
                    }
                }
                next.failover = Some(FailoverSettings::default());
            }
            next.mode = GroupMode::Static;
            next.members.clear();
            next.keywords.clear();
        } else if current.rule == GroupRule::Failover {
            self.previous_failover = Some(current.clone());
            if let Some(previous) = &self.previous_plain {
                next.mode = previous.mode;
                next.members = previous.members.clone();
                next.keywords = previous.keywords.clone();
            }
            next.lanes.clear();
            next.failover = None;
        }
        next.rule = rule;
        self.draft = Some(next);
        self.active_lane = 0;
        self.checked = Default::default();
        self.sync_lane(window, cx);
        self.refresh_filters(window, cx);
        cx.notify();
    }
    pub(super) fn sync_lane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(d) = &self.draft else {
            return;
        };
        let Some(lane) = d.lanes.get(self.active_lane) else {
            return;
        };
        let name = lane.name.clone();
        let manual = lane.manual;
        let icon = if lane.icon.is_empty() {
            d.icon.clone()
        } else {
            lane.icon.clone()
        };
        self.inputs[Field::LaneName as usize]
            .update(cx, |input, cx| input.set_value(name, window, cx));
        self.lane_picker.update(cx, |picker, cx| {
            picker.value = icon;
            cx.notify();
        });
        self.lane_mode.update(cx, |select, cx| {
            select.set_selected_index(Some(IndexPath::new(usize::from(manual))), window, cx)
        });
    }
    fn select_lane(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.active_lane = index;
        self.checked = Default::default();
        self.sync_lane(window, cx);
        cx.notify();
    }
    fn remove_lane(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        if let Some(d) = &mut self.draft {
            d.lanes.remove(self.active_lane);
            self.active_lane = self.active_lane.min(d.lanes.len().saturating_sub(1));
        }
        self.deleting_lane = false;
        self.checked = Default::default();
        self.sync_lane(window, cx);
        cx.notify();
    }
    pub(super) fn failover_editor(&mut self, cx: &mut Context<Self>) -> Div {
        let restore = self
            .draft
            .as_ref()
            .and_then(|d| d.failover.as_ref())
            .is_some_and(|s| s.restore_primary);
        let advanced = div()
            .flex()
            .items_end()
            .gap(px(t::PAD))
            .p(px(t::PAD))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(px(t::POPOVER_RADIUS))
            .bg(cx.theme().button.opacity(0.5))
            .child(
                field("测速超时", self.unit_input(Field::Timeout, "秒", cx), cx)
                    .w(px(t::groups::ADVANCED_TIMEOUT)),
            )
            .child(
                field("连续失败次数", self.input(Field::Failure, cx), cx)
                    .w(px(t::groups::ADVANCED_FAILURE)),
            )
            .child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(t::CONTROL))
                    .gap(px(t::GAP))
                    .text_size(px(t::SMALL))
                    .child(tr(cx, "主用恢复后自动切回"))
                    .child(
                        components::toggle(
                            "failover-restore",
                            restore,
                            tr(cx, "主用恢复后自动切回"),
                        )
                        .compact()
                        .disabled(self.busy)
                        .on_change(cx.listener(
                            |this, value: &bool, _, cx| {
                                if let Some(settings) =
                                    this.draft.as_mut().and_then(|d| d.failover.as_mut())
                                {
                                    settings.restore_primary = *value;
                                }
                                cx.notify();
                            },
                        )),
                    ),
            )
            .child(
                field(
                    "恢复主用等待",
                    self.unit_input(Field::Recovery, "秒", cx),
                    cx,
                )
                .w(px(t::groups::ADVANCED_RECOVERY)),
            );
        let transfer = div()
            .w(px(t::CONTROL))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .justify_center()
            .gap(px(t::GAP))
            .children([0, 1].map(|side| {
                components::icon_button(
                    ("lane-transfer", side),
                    tr(
                        cx,
                        if side == 0 {
                            "加入所选"
                        } else {
                            "移出所选"
                        },
                    ),
                    if side == 0 { "ArrowRight" } else { "ArrowLeft" },
                    t::CONTROL,
                )
                .bg(cx.theme().button)
                .disabled(self.busy || self.checked[side].is_empty())
                .when(self.busy && !self.checked[side].is_empty(), |b| {
                    b.retain_disabled_appearance()
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    let ids = checked_members(&this.member_pool(side), &this.checked[side]);
                    this.transfer(side, ids, cx);
                }))
            }));
        div().flex().flex_col().gap(px(t::PAD))
            .child(div().flex().child(group_button("failover-advanced",tr(cx,"高级设置"),t::SMALL).ghost().reference_hover(gpui::transparent_black(), cx.theme().input, cx.theme().foreground).h(px(t::groups::CARD_ACTION))
                .disabled(self.busy).child(icon(if self.advanced { "ChevronUp" } else { "ChevronDown" },t::groups::LANE_SEARCH_ICON))
                .on_click(cx.listener(|this,_,_,cx| { this.advanced = !this.advanced; cx.notify(); }))))
            .when(self.advanced,|d| d.child(advanced))
            .child(hint("从左侧选节点加入当前页签；最前面的页签是主用，后面依次备用。单节点页签直接使用，多节点页签可选自动优选或手动选择。",cx).text_size(px(t::groups::FAILOVER_HINT_TEXT)).line_height(px(t::groups::FAILOVER_HINT_LINE)))
            .child(div().flex().gap(px(t::PAD)).min_h(px(t::groups::FAILOVER_MIN_HEIGHT))
                .child(self.member_pane(0,cx)).child(transfer).child(self.member_pane(1,cx)))
    }
    fn unit_input(&self, field: Field, unit: &str, cx: &App) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(t::ROW_GAP))
            .child(
                div()
                    .w(px(t::NUMBER_WIDTH))
                    .flex_shrink_0()
                    .child(self.input(field, cx)),
            )
            .child(hint(unit, cx))
    }
    pub(super) fn lane_header(&self, cx: &mut Context<Self>) -> Div {
        let Some(draft) = &self.draft else {
            return div();
        };
        let tabs = div()
            .flex()
            .min_h(px(t::groups::LANE_TAB_BAR))
            .items_center()
            .gap(px(t::ROW_GAP))
            .p(px(t::ROW_GAP))
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().button.opacity(0.58))
            .children(draft.lanes.iter().enumerate().map(|(index, lane)| {
                let label = if index == 0 {
                    tr(cx, "主用").to_string()
                } else {
                    tr(cx, "备用 {index}").replace("{index}", &index.to_string())
                };
                div()
                    .id(SharedString::from(format!("lane-tab-scope-{}", lane.id)))
                    .child(
                        group_button(
                            "lane-tab",
                            format!("{label} · {}", lane.members.len()),
                            t::SMALL,
                        )
                        .ghost()
                        .h(px(t::groups::LANE_TAB_HEIGHT))
                        .px(px(t::groups::LANE_TAB_PAD))
                        .rounded(px(t::groups::COMPACT_RADIUS))
                        .disabled(self.busy)
                        .text_color(cx.theme().foreground)
                        .when(index == self.active_lane, |b| {
                            b.primary()
                                .bg(rgb(t::ACCENT))
                                .text_color(rgb(t::groups::CHECK_COLOR))
                        })
                        .on_click(cx.listener(
                            move |this, _, window, cx| this.select_lane(index, window, cx),
                        )),
                    )
            }))
            .when(draft.lanes.len() < 3, |d| {
                d.child(div().flex_1()).child(
                    components::icon_button(
                        "add-backup",
                        tr(cx, "添加备用页签"),
                        "Plus",
                        t::groups::LANE_TAB_HEIGHT,
                    )
                    .disabled(self.busy)
                    .retain_disabled_appearance()
                    .on_click(
                        cx.listener(|this, _, window, cx| match FailoverLane::fresh() {
                            Ok(lane) => {
                                if let Some(draft) = &mut this.draft {
                                    draft.lanes.push(lane);
                                    let index = draft.lanes.len() - 1;
                                    this.select_lane(index, window, cx);
                                }
                            }
                            Err(error) => {
                                this.error = Some(GroupSaveError::Storage(error));
                                cx.notify();
                            }
                        }),
                    ),
                )
            });
        div().child(tabs).child(
            div()
                .flex()
                .items_center()
                .gap(px(t::groups::LANE_SETTINGS_PAD))
                .p(px(t::groups::LANE_SETTINGS_PAD))
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    div()
                        .w(px(t::groups::LANE_ICON_WIDTH))
                        .child(self.lane_picker.clone()),
                )
                .child(
                    div().flex_1().child(
                        self.input(Field::LaneName, cx)
                            .h(px(t::groups::LANE_SETTINGS_HEIGHT)),
                    ),
                )
                .child(
                    components::icon_button(
                        "delete-lane",
                        tr(cx, "删除当前页签"),
                        "Trash",
                        t::groups::LANE_DELETE_WIDTH,
                    )
                    .h(px(t::groups::LANE_SETTINGS_HEIGHT))
                    .disabled(self.busy || draft.lanes.is_empty())
                    .when(self.busy && !draft.lanes.is_empty(), |b| {
                        b.retain_disabled_appearance()
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        if this.draft_members().is_empty() {
                            this.remove_lane(window, cx);
                        } else {
                            this.deleting_lane = true;
                            cx.notify();
                        }
                    })),
                ),
        )
    }
    pub(super) fn lane_confirmation(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let discard = self.discard_confirm;
        let title = if discard {
            "修改分组规则"
        } else {
            "删除页签"
        };
        let message = if discard {
            "改为其他分组规则后，现有的主用和备用页签配置会被删除。确定继续？"
        } else {
            "当前页签中已有节点，确定删除？"
        };
        let actions = div()
            .flex()
            .justify_end()
            .gap(px(t::GAP))
            .child(
                group_button("lane-confirm-cancel", tr(cx, "取消"), t::BODY).on_click(cx.listener(
                    |this, _, _, cx| {
                        this.deleting_lane = false;
                        this.discard_confirm = false;
                        cx.notify();
                    },
                )),
            )
            .child(
                group_button("lane-confirm-ok", tr(cx, "确定"), t::BODY)
                    .primary()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if discard {
                            this.discard_accepted = true;
                            this.discard_confirm = false;
                            this.save_draft(cx);
                        } else {
                            this.remove_lane(window, cx);
                        }
                    })),
            );
        let surface = div()
            .w(px(t::groups::AUTO_WIDTH))
            .p(px(t::SECTION_PADDING))
            .bg(cx.theme().popover)
            .rounded(px(t::POPOVER_RADIUS))
            .child(div().text_size(px(t::SECTION_TITLE)).child(tr(cx, title)))
            .child(hint(message, cx).py(px(t::PAD)))
            .child(actions);
        gpui_kit::base::Dialog::new(cx)
            .close_on_escape(true)
            .close_on_backdrop_press(true)
            .on_cancel(|_, _, _| true)
            .on_ok(|_, _, _| false)
            .backdrop(div().size_full().bg(rgba(0x00000066)))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|this, _, _, cx| {
                this.deleting_lane = false;
                this.discard_confirm = false;
                cx.notify();
            }))
            .into_any_element()
    }
}
