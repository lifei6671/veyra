//! 策略设置只修改正式proxy_view偏好；显示排序与目标分流命中顺序分开。
use super::*;
use crate::behavior_preferences::{BehaviorCoordinator, BehaviorStatus};
use crate::ui::components::{
    self,
    order_motion::OrderMotion,
    select::{SelectEvent, SelectState},
};
use gpui_kit::component::IndexPath;
use std::{cell::RefCell, collections::HashMap, rc::Rc, time::Duration};
use veyra_core::domain::{AppState, DesktopBehaviorPreferencesPatch, ProxyViewPreferences};

pub(super) enum SettingsEvent {
    Changed(ProxyViewPreferences),
    Saved(Box<AppState>),
}
// 用户指定此处悬停/拖动均为四向箭头；其它平台保留原有行为，macOS独立适配。
fn policy_cursor(_dragging: bool) -> CursorStyle {
    #[cfg(target_os = "macos")]
    {
        CursorStyle::Move
    }
    #[cfg(not(target_os = "macos"))]
    {
        if _dragging {
            CursorStyle::ClosedHand
        } else {
            CursorStyle::OpenHand
        }
    }
}
#[derive(Clone)]
struct PolicyDrag {
    item: PolicyItem,
    size: Size<Pixels>,
    offset: Point<Pixels>,
}
impl Render for PolicyDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 浮层与排序行共享完整图标/标题；保留按下时抓取点，越过阈值不跳位。
        div().w(self.size.width).h(self.size.height).child(
            policy_order_row(&self.item, cx)
                .w(self.size.width)
                .absolute()
                .left(self.offset.x)
                .top(self.offset.y)
                .cursor(policy_cursor(true))
                .shadow(vec![
                    BoxShadow::new(
                        px(0.),
                        px(pt::ORDER_DRAG_SHADOW_Y),
                        cx.theme().foreground.opacity(pt::ORDER_DRAG_SHADOW_ALPHA),
                    )
                    .blur_radius(px(pt::ORDER_DRAG_SHADOW_BLUR)),
                ]),
        )
    }
}
#[derive(Clone, PartialEq, Eq)]
pub(super) struct PolicyItem {
    id: String,
    name: String,
    icon: String,
    scale: i32,
    enabled: bool,
}
pub(super) struct ProxySettings {
    services: Weak<crate::services::AppServices>,
    coordinator: BehaviorCoordinator,
    pub open: bool,
    focus: FocusHandle,
    restore_focus: Option<FocusHandle>,
    scroll: ScrollHandle,
    sort: Entity<SelectState>,
    width: Entity<InputState>,
    items: Vec<PolicyItem>,
    rules: Vec<PolicyItem>,
    custom: Option<(Option<String>, bool)>,
    // 仅拖动预览；松手确认前不修改正式偏好或规则命中顺序。
    dragging: Option<String>,
    drag_order: Option<Vec<String>>,
    drag_grab: Point<Pixels>,
    order_bounds: Rc<RefCell<Vec<Bounds<Pixels>>>>,
    order_list_bounds: Rc<RefCell<Bounds<Pixels>>>,
    order_motion: HashMap<String, OrderMotion>,
    _events: Vec<Subscription>,
}
impl EventEmitter<SettingsEvent> for ProxySettings {}
impl ProxySettings {
    pub fn new(
        services: &Arc<crate::services::AppServices>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let sort = cx.new(|cx| {
            SelectState::new(
                vec![
                    "按配置排序",
                    "按名称升序",
                    "按名称降序",
                    "按延迟升序",
                    "按延迟降序",
                ],
                Some(IndexPath::new(3)),
                window,
                cx,
            )
        });
        let width = cx.new(|cx| InputState::new(window, cx).default_value("145"));
        let events = vec![
            cx.subscribe(&sort, |view, _, event, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    let node_sort = match value.as_ref() {
                        "按配置排序" => NodeSort::Default,
                        "按名称升序" => NodeSort::NameAsc,
                        "按名称降序" => NodeSort::NameDesc,
                        "按延迟降序" => NodeSort::LatencyDesc,
                        _ => NodeSort::LatencyAsc,
                    };
                    view.edit(
                        DesktopBehaviorPreferencesPatch {
                            node_sort: Some(node_sort),
                            ..Default::default()
                        },
                        cx,
                    );
                }
            }),
            cx.subscribe(&width, |view, input, event, cx| {
                if matches!(event, InputEvent::Change) {
                    let Ok(value) = input.read(cx).value().parse::<u32>() else {
                        return;
                    };
                    // 用户核对的当前原版没有100–320限制；手输与步进走同一正式保存路径。
                    if value != view.coordinator.draft.proxy_view.node_card_min_width {
                        view.edit(
                            DesktopBehaviorPreferencesPatch {
                                node_card_min_width: Some(value),
                                ..Default::default()
                            },
                            cx,
                        );
                    }
                }
            }),
        ];
        Self {
            services: Arc::downgrade(services),
            coordinator: BehaviorCoordinator::default(),
            open: false,
            focus: cx.focus_handle(),
            restore_focus: None,
            scroll: ScrollHandle::new(),
            sort,
            width,
            items: vec![],
            rules: vec![],
            custom: None,
            dragging: None,
            drag_order: None,
            drag_grab: Point::default(),
            order_bounds: Rc::new(RefCell::new(vec![])),
            order_list_bounds: Rc::new(RefCell::new(Bounds::default())),
            order_motion: HashMap::new(),
            _events: events,
        }
    }
    pub fn project(&mut self, state: &AppState, cx: &mut Context<Self>) {
        self.coordinator.rebase(state);
        let previous_items = self.items.clone();
        self.rules = state
            .profile
            .routing
            .selectors()
            .map(|p| PolicyItem {
                id: p.id.0.clone(),
                name: p.name.clone(),
                icon: p.icon.clone(),
                scale: p.icon_scale,
                enabled: p.enabled,
            })
            .collect();
        self.items = self.rules.iter().filter(|p| p.enabled).cloned().collect();
        // 上一drop保存回包不能清掉下一次拖动；只有正式目标分流内容变化才撤销预览。
        if self.items != previous_items {
            self.dragging = None;
            self.drag_order = None;
        }
        self.custom = state
            .profile
            .routing
            .custom
            .as_ref()
            .map(|r| (r.name.clone(), r.enabled && !r.rules.is_empty()));
        cx.emit(SettingsEvent::Changed(
            self.coordinator.draft.proxy_view.clone(),
        ));
        cx.notify();
    }
    pub fn show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.restore_focus = window.focused(cx);
        self.open = true;
        self.order_motion.clear();
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.focus.focus(window, cx);
        cx.notify();
    }
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = false;
        if self.dragging.take().is_some() {
            cx.stop_active_drag(window);
        }
        self.drag_order = None;
        if let Some(focus) = self.restore_focus.take() {
            focus.focus(window, cx);
        }
        cx.notify();
    }
    fn edit(&mut self, patch: DesktopBehaviorPreferencesPatch, cx: &mut Context<Self>) {
        self.coordinator.edit(patch);
        cx.emit(SettingsEvent::Changed(
            self.coordinator.draft.proxy_view.clone(),
        ));
        self.save(cx);
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        let Some(services) = self.services.upgrade() else {
            return;
        };
        let Some(request) = self.coordinator.submit() else {
            return;
        };
        let receiver = services.proxy_save_preferences(request.clone());
        cx.spawn(async move |view, cx| {
            let result = receiver.await.unwrap_or_else(|_| {
                Err(veyra_core::domain::AppError::new(
                    veyra_core::domain::AppErrorCode::StorageFailed,
                ))
            });
            let _ = view.update(cx, |view, cx| {
                view.coordinator
                    .complete(request.generation, result.as_deref().map_err(Clone::clone));
                if let Ok(state) = result {
                    cx.emit(SettingsEvent::Saved(state));
                }
                if view.coordinator.status == BehaviorStatus::Pending {
                    view.save(cx);
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn order(&self) -> Vec<String> {
        // 保留偏好顺序，仅补齐当前正式策略；不存在的ID不能生成卡片。
        let mut order = self
            .coordinator
            .draft
            .proxy_view
            .strategy_order
            .iter()
            .filter(|id| self.items.iter().any(|item| item.id == **id))
            .cloned()
            .collect::<Vec<_>>();
        for item in &self.items {
            if !order.contains(&item.id) {
                order.push(item.id.clone());
            }
        }
        order
    }
}
// 最近目标行中心吸附；显示列表行高和gap保持React原值。
fn order_slot(top: f32, count: usize) -> usize {
    (top / (pt::SETTINGS_ROW + pt::SETTINGS_LIST_GAP))
        .round()
        .clamp(0., (count - 1) as f32) as usize
}
fn policy_order_row(item: &PolicyItem, cx: &App) -> Div {
    // 列表与跟手浮层共用一份实际卡片，不把拖动项简化成文字chip。
    div()
        .h(px(pt::SETTINGS_ROW))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(t::GAP))
        .px(px(t::GAP))
        .py(px(pt::ORDER_ROW_PY))
        .rounded(cx.theme().radius_lg)
        .bg(rgba(pt::SETTINGS_ITEM_BG))
        .font(t::openbox_font())
        .font_weight(pt::FONT_REGULAR)
        .text_size(px(t::BODY))
        .line_height(px(pt::CONTROL_LINE))
        .text_color(cx.theme().foreground)
        .child(
            super::super::icons::icon("Bars3", t::ICON_SMALL)
                .text_color(cx.theme().muted_foreground),
        )
        .child(crate::ui::components::icon_picker::scaled_group_icon(
            &item.icon,
            t::ICON_SMALL,
            item.scale,
            cx,
        ))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .text_ellipsis()
                .child(item.name.clone()),
        )
}
fn row(label: &'static str, control: impl IntoElement, cx: &App) -> Div {
    div()
        .w_full()
        // React .proxy-setting-row 的 min-height 包括只有 24px 的开关行。
        .min_h(px(pt::SETTINGS_ROW))
        .flex()
        .items_center()
        .gap(px(t::GAP))
        .child(tr(cx, label))
        .child(control)
}
impl Render for ProxySettings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preferences = self.coordinator.draft.proxy_view.clone();
        let index = match preferences.node_sort {
            NodeSort::Default => 0,
            NodeSort::NameAsc => 1,
            NodeSort::NameDesc => 2,
            NodeSort::LatencyAsc => 3,
            NodeSort::LatencyDesc => 4,
        };
        self.sort.update(cx, |sort, cx| {
            sort.set_selected_index(Some(IndexPath::new(index)), window, cx)
        });
        if !self.width.read(cx).focus_handle(cx).is_focused(window) {
            let value = preferences.node_card_min_width.to_string();
            if self.width.read(cx).value().as_ref() != value {
                self.width
                    .update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
        let width_line = rgba(if cx.theme().mode.is_dark() {
            pt::WIDTH_LINE_DARK
        } else {
            pt::WIDTH_LINE_LIGHT
        });
        let width_control = div()
            .flex()
            .flex_shrink_0()
            .h(px(pt::SETTINGS_ROW))
            .border_1()
            .border_color(width_line)
            .rounded(px(pt::WIDTH_CONTROL_RADIUS))
            .bg(rgb(if cx.theme().mode.is_dark() {
                pt::WIDTH_SURFACE_DARK
            } else {
                pt::WIDTH_SURFACE_LIGHT
            }))
            .overflow_hidden()
            .on_children_prepainted(|bounds, _, _| {
                #[cfg(debug_assertions)]
                components::log_layout("proxy-width-control", &bounds);
            })
            .child(
                components::text_input(&self.width)
                    .numeric()
                    .w(px(pt::SETTINGS_INPUT_WIDTH))
                    .border_0()
                    .border_r_1()
                    .border_color(width_line)
                    .rounded_none()
                    .bg(rgba(0))
                    .px(px(pt::WIDTH_INPUT_PX))
                    .py_0()
                    .focus_style(rgba(0).into(), 0.)
                    .h(px(pt::SETTINGS_ROW - 2.)),
            )
            .child(
                button("proxy-width-reset", tr(cx, "重置"))
                    .h(px(pt::SETTINGS_ROW - 2.))
                    .w(px(pt::SETTINGS_RESET_WIDTH))
                    .border_0()
                    .rounded_none()
                    .px(px(pt::WIDTH_RESET_PX))
                    // Kit 的默认 variant 会覆盖 Styled 背景；按原 CSS --field 绘制。
                    .reference_hover(
                        rgba(if cx.theme().mode.is_dark() {
                            pt::SETTINGS_RESET_DARK_BG
                        } else {
                            pt::SETTINGS_RESET_BG
                        })
                        .into(),
                        rgba(if cx.theme().mode.is_dark() {
                            pt::SETTINGS_RESET_DARK_BG
                        } else {
                            pt::SETTINGS_RESET_BG
                        })
                        .into(),
                        cx.theme().foreground,
                    )
                    .font_weight(t::FONT_SEMIBOLD)
                    .text_size(px(t::BODY))
                    .line_height(px(pt::CONTROL_LINE))
                    .on_click(cx.listener(|view, _, window, cx| {
                        view.width
                            .update(cx, |input, cx| input.set_value("145", window, cx));
                        view.edit(
                            DesktopBehaviorPreferencesPatch {
                                node_card_min_width: Some(145),
                                ..Default::default()
                            },
                            cx,
                        );
                    })),
            );
        // GPUI在列表外松手/取消时清理原生drag；同步撤销纯视觉预览，不写偏好。
        if self.dragging.is_some() && !cx.has_active_drag() {
            self.dragging = None;
            self.drag_order = None;
        }
        let order = self.drag_order.clone().unwrap_or_else(|| self.order());
        let bounds = self.order_bounds.clone();
        let list_bounds = self.order_list_bounds.clone();
        let row_count = order.len();
        let mut list = div()
            .w_full()
            .relative()
            .flex()
            .flex_col()
            .gap(px(pt::SETTINGS_LIST_GAP))
            .on_children_prepainted(move |mut values, _, _| {
                // 末尾绝对定位canvas只测真实列表bounds，不参与排序行索引。
                values.truncate(row_count);
                #[cfg(debug_assertions)]
                components::log_layout("proxy-display-order-bounds", &values);
                *bounds.borrow_mut() = values;
            })
            .on_drag_move(
                cx.listener(|view, event: &DragMoveEvent<PolicyDrag>, _, cx| {
                    let id = &event.drag(cx).item.id;
                    if view.dragging.as_ref() != Some(id) {
                        return;
                    }
                    let mut order = view.drag_order.clone().unwrap();
                    let from = order.iter().position(|item| item == id).unwrap();
                    // 用稳定列表坐标和抓取点选最近槽位；不按正在动画的行命中反复换位。
                    let top = f32::from(
                        event.event.position.y - event.bounds.origin.y - view.drag_grab.y,
                    );
                    let slot = order_slot(top, order.len());
                    if from != slot {
                        let id = order.remove(from);
                        order.insert(slot, id);
                        #[cfg(debug_assertions)]
                        eprintln!("proxy-order preview slot={slot} order={order:?}");
                        view.drag_order = Some(order);
                        cx.notify();
                    }
                }),
            )
            .on_drop(cx.listener(|view, drag: &PolicyDrag, _, cx| {
                if view.dragging.take().as_ref() == Some(&drag.item.id)
                    && let Some(order) = view.drag_order.take()
                    && order != view.order()
                {
                    // 一次drop只提交显示排序，继续使用既有CAS/保存失败草稿保留路径。
                    view.edit(
                        DesktopBehaviorPreferencesPatch {
                            strategy_order: Some(order),
                            ..Default::default()
                        },
                        cx,
                    );
                }
                cx.notify();
            }));
        for (index, id) in order.into_iter().enumerate() {
            let item = self
                .items
                .iter()
                .find(|item| item.id == id)
                .unwrap()
                .clone();
            let view = cx.entity().downgrade();
            let slot_y = index as f32 * (pt::SETTINGS_ROW + pt::SETTINGS_LIST_GAP);
            let now = cx.background_executor().now();
            let motion = self.order_motion.entry(id.clone()).or_insert(OrderMotion {
                from: slot_y,
                target: slot_y,
                started_at: now,
            });
            motion.retarget(slot_y, now);
            let position = if cx.reduce_motion() {
                motion.from = slot_y;
                slot_y
            } else {
                let position = motion.sample(now);
                if now.saturating_duration_since(motion.started_at)
                    < Duration::from_millis(pt::ORDER_MOVE_MS)
                    && motion.from != motion.target
                {
                    window.request_animation_frame();
                }
                position
            };
            let row = policy_order_row(&item, cx)
                .id(SharedString::from(format!("proxy-display-order-{}", item.id)))
                .w_full()
                .relative()
                .top(px(position - slot_y))
                .cursor(policy_cursor(false))
                .opacity(if self.dragging.as_ref() == Some(&id) { pt::ORDER_PLACEHOLDER_OPACITY } else { 1. })
                .on_mouse_down(MouseButton::Left, cx.listener(move |view, event: &MouseDownEvent, _, _| {
                    view.drag_grab = event.position - view.order_bounds.borrow()[index].origin;
                }))
                .on_drag(PolicyDrag { item, size: Size::default(), offset: Point::default() },
                    move |drag, cursor_offset, window, cx| {
                        let mut preview = drag.clone();
                        let _ = view.update(cx, |view, cx| {
                            preview.size = view.order_bounds.borrow()[index].size;
                            preview.offset = cursor_offset - view.drag_grab;
                            let mut order = view.order();
                            // 快速拖放首次move就吸附，不能等下一次鼠标事件才给目标反馈。
                            let first_y = view.order_list_bounds.borrow().origin.y;
                            let slot = order_slot(f32::from(window.mouse_position().y - first_y - view.drag_grab.y), order.len());
                            let id = order.remove(index);
                            order.insert(slot, id);
                            view.dragging = Some(drag.item.id.clone());
                            view.drag_order = Some(order);
                            #[cfg(debug_assertions)]
                            eprintln!("proxy-order begin id={} slot={slot} size={:?} grab={:?} offset={:?}", drag.item.id, preview.size, view.drag_grab, preview.offset);
                            cx.notify();
                        });
                        cx.new(|_| preview)
                    });
            list = list.child(row);
        }
        // 起点使用列表自身的真实bounds；前一动画未结束时，第一子行不能作起点。
        list = list.child(
            canvas(
                move |bounds, _, _| {
                    *list_bounds.borrow_mut() = bounds;
                    #[cfg(debug_assertions)]
                    components::log_layout("proxy-display-order-list", &[bounds]);
                },
                |_, _, _, _| {},
            )
            .absolute()
            .inset_0(),
        );
        let column = |label: &'static str, list: Div, note: &'static str| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(t::GAP))
                .child(
                    div()
                        .text_size(px(t::SMALL))
                        .line_height(px(t::ICON_SMALL))
                        .font_weight(pt::FONT_MEDIUM)
                        .child(tr(cx, label)),
                )
                .child(list)
                .child(
                    div()
                        .text_size(px(t::SMALL))
                        .line_height(px(pt::SETTINGS_NOTE_LINE))
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(cx, note)),
                )
        };
        let mut rules = div()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(pt::SETTINGS_LIST_GAP));
        // 原参考无条件展示 pinned 行；没有配置时只读呈现 None，不写入假规则。
        let custom_name = self
            .custom
            .as_ref()
            .and_then(|(name, _)| name.clone())
            .unwrap_or_else(|| tr(cx, "前置自定义分流").into());
        let custom_active = self.custom.as_ref().is_some_and(|(_, active)| *active);
        rules = rules.child(
            div()
                .h(px(pt::SETTINGS_ROW))
                .flex()
                .items_center()
                .gap(px(t::GAP))
                .px(px(t::GAP))
                .rounded(cx.theme().radius_lg)
                .bg(rgba(pt::SETTINGS_ITEM_BG))
                .child(
                    super::super::icons::icon("MapPin", pt::SETTINGS_PIN_SIZE)
                        .rotate(Radians(pt::SETTINGS_PIN_ROTATION.to_radians()))
                        .text_color(rgb(pt::SETTINGS_PIN_COLOR)),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .text_ellipsis()
                        .child(custom_name),
                )
                .child(
                    div()
                        .text_size(px(pt::SETTINGS_BADGE_FONT))
                        .px(px(pt::SETTINGS_BADGE_PX))
                        .py(px(pt::SETTINGS_BADGE_PY))
                        .rounded(px(pt::SETTINGS_BADGE_RADIUS))
                        .bg(cx.theme().background.opacity(0.65))
                        .text_color(cx.theme().muted_foreground)
                        .child(tr(cx, "前置")),
                )
                .when(!custom_active, |row| {
                    row.child(
                        div()
                            .text_size(px(pt::SETTINGS_BADGE_FONT))
                            .px(px(pt::SETTINGS_BADGE_PX))
                            .py(px(pt::SETTINGS_BADGE_PY))
                            .text_color(cx.theme().muted_foreground)
                            .child(tr(cx, "未生效")),
                    )
                }),
        );
        rules = rules.children(self.rules.iter().enumerate().map(|(i, item)| {
            div()
                .h(px(pt::SETTINGS_ROW))
                .flex()
                .items_center()
                .gap(px(t::GAP))
                .px(px(t::GAP))
                .rounded(cx.theme().radius_lg)
                .bg(rgba(pt::SETTINGS_ITEM_BG))
                .opacity(if item.enabled { 1. } else { 0.45 })
                .child(
                    div()
                        .w(px(pt::SETTINGS_NUMBER_WIDTH))
                        .flex_shrink_0()
                        .text_center()
                        .text_size(px(t::SMALL))
                        .line_height(px(pt::SETTINGS_NOTE_LINE))
                        .text_color(cx.theme().foreground.opacity(0.4))
                        .child((i + 1).to_string()),
                )
                .child(crate::ui::components::icon_picker::scaled_group_icon(
                    &item.icon,
                    t::ICON_SMALL,
                    item.scale,
                    cx,
                ))
                .child(item.name.clone())
        }));
        let body = div()
            .on_children_prepainted(|_bounds, _, _| {
                #[cfg(debug_assertions)]
                components::log_layout("proxy-settings-body", &_bounds);
            })
            .id("proxy-settings-body")
            .w_full()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .track_scroll(&self.scroll)
            .overflow_y_scroll()
            .vertical_scrollbar(&self.scroll)
            .p(px(pt::SETTINGS_BODY_PAD))
            .gap(px(pt::SETTINGS_GAP))
            .child(row(
                "节点排序方式",
                components::select(&self.sort, tr(cx, "节点排序方式"))
                    .w(px(pt::SETTINGS_SORT_WIDTH))
                    .h(px(pt::SETTINGS_ROW))
                    .pl(px(pt::SETTINGS_SORT_LEFT))
                    .pr(px(pt::SETTINGS_SORT_RIGHT))
                    .line_height(px(pt::CONTROL_LINE))
                    .rounded(px(pt::CONTROL_RADIUS))
                    .border_color(cx.theme().foreground.opacity(0.2))
                    .bg(cx.theme().popover.opacity(0.9))
                    .indicator(pt::SETTINGS_SORT_ARROW, pt::SETTINGS_SORT_ARROW_WIDTH, pt::SETTINGS_SORT_ARROW_HEIGHT, pt::SETTINGS_SORT_ARROW_RIGHT),
                cx,
            ))
            .child(row(
                "节点根据提供商分组",
                components::toggle(
                    "proxy-provider-group",
                    preferences.group_by_provider,
                    tr(cx, "节点根据提供商分组"),
                )
                .on_change(cx.listener(|view, _, _, cx| {
                    view.edit(
                        DesktopBehaviorPreferencesPatch {
                            group_by_provider: Some(
                                !view.coordinator.draft.proxy_view.group_by_provider,
                            ),
                            ..Default::default()
                        },
                        cx,
                    )
                })),
                cx,
            ))
            .child(row("节点卡片最小宽度", width_control, cx))
            .child(
                div()
                    .w_full()
                    .h(px(pt::SETTINGS_GAP))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .child(div().w_full().h(px(1.)).bg(cx.theme().border)),
            )
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_start()
                    .gap(px(pt::SETTINGS_COLUMNS_GAP))
                    .child(column(
                        "显示排序",
                        list,
                        "拖动调整;代理页的「策略」按这个顺序显示,和命中顺序无关。",
                    ))
                    .child(column(
                        "命中规则排序",
                        rules,
                        "流量按这个顺序逐条匹配,首条命中生效;有域名的访问只看域名条件,都没命中就走兜底,IP 条件只管直接按 IP 发起的连接。要改去「设置 → 目标分流」。",
                    )),
            )
            .when(
                matches!(self.coordinator.status, BehaviorStatus::Failed(_)),
                |body| {
                    body.child(
                        div()
                            .text_color(rgb(pt::DANGER))
                            .child(tr(cx, "保存失败，草稿已保留"))
                            .child(
                                button("proxy-settings-retry", tr(cx, "重试"))
                                    .on_click(cx.listener(|view, _, _, cx| view.save(cx))),
                            ),
                    )
                },
            );
        let height = (window.viewport_size().height * 0.9).min(px(648.));
        let surface = div()
            .w(px(pt::SETTINGS_WIDTH))
            .max_w(window.viewport_size().width)
            .max_h(height)
            .flex()
            .flex_col()
            .overflow_hidden()
            .rounded(px(pt::SETTINGS_RADIUS))
            .font(t::openbox_font())
            .text_size(px(t::BODY))
            .line_height(px(pt::CONTROL_LINE))
            .font_weight(pt::FONT_REGULAR)
            .bg(cx.theme().popover.opacity(0.9))
            .child(components::modal_header(
                tr(cx, "策略设置"),
                icon_button("proxy-settings-close", tr(cx, "关闭"), "XMark", pt::TAB)
                    .on_click(cx.listener(|view, _, window, cx| view.close(window, cx))),
                cx,
            ))
            .child(body);
        gpui_kit::base::Dialog::new(cx)
            .focus_handle(self.focus.clone())
            // 仅Escape/关闭/遮罩关闭；数值输入Enter不得触发Kit默认Confirm关闭。
            .on_ok(|_, _, _| false)
            .close_on_escape(true)
            .close_on_backdrop_press(true)
            .backdrop(div().size_full().bg(rgba(0x00000066)))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|view, _, window, cx| view.close(window, cx)))
    }
}

#[cfg(test)]
mod tests {
    use super::{Duration, OrderMotion, order_slot};
    use std::time::Instant;
    #[test]
    fn policy81_reverse_keeps_current_position_and_full_duration() {
        // 保护快速反向的跟手反馈：不跳回原槽，也不按前一进度缩短新动画。
        let start = Instant::now();
        let mut motion = OrderMotion {
            from: 0.,
            target: 72.,
            started_at: start,
        };
        let reverse_at = start + Duration::from_millis(30);
        let before = motion.sample(reverse_at);
        motion.retarget(0., reverse_at);
        assert_eq!(motion.sample(reverse_at), before);
        assert!(motion.sample(reverse_at + Duration::from_millis(120)) > 0.);
        assert_eq!(motion.sample(reverse_at + Duration::from_millis(180)), 0.);
        motion.retarget(72., reverse_at + Duration::from_millis(20));
        assert_eq!(motion.started_at, reverse_at + Duration::from_millis(20));
    }
    #[test]
    fn policy80_drag_snaps_at_row_midpoint_without_edge_jitter() {
        // 保护用户预览目标：跨过相邻槽中点才让位，列表两端吸附合法位置。
        assert_eq!(order_slot(-200., 3), 0);
        assert_eq!(order_slot(17.9, 3), 0);
        assert_eq!(order_slot(18.1, 3), 1);
        assert_eq!(order_slot(53.9, 3), 1);
        assert_eq!(order_slot(54.1, 3), 2);
        assert_eq!(order_slot(999., 3), 2);
    }
}
