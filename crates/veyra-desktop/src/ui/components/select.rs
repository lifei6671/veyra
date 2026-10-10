//! 有限选项 Select：Kit base 管理焦点、Escape、弹层生命周期；React CSS 定义外观。
use crate::ui::i18n::tr;
use crate::ui::{icons::icon, tokens as t};
use gpui_kit::{
    base::{Popover, Select as BaseSelect, StyledExt},
    component::{ActiveTheme, IndexPath},
    prelude::*,
    *,
};
pub enum SelectEvent {
    Confirm(Option<SharedString>),
}
// 选项、已提交值与待确认 cursor 共用一个 presentation state，鼠标不另存 hover 值。
struct Selection {
    options: Vec<SharedString>,
    selected: Option<usize>,
    cursor: usize,
}
impl Selection {
    fn new(options: Vec<impl Into<SharedString>>, selected: Option<usize>) -> Self {
        let selected = selected.filter(|&i| i < options.len());
        Self {
            options: options.into_iter().map(Into::into).collect(),
            selected,
            cursor: selected.unwrap_or(0),
        }
    }
    fn set_selected(&mut self, index: Option<usize>, open: bool) {
        self.selected = index.filter(|&i| i < self.options.len());
        if !open {
            self.cursor = self.selected.unwrap_or(0);
        }
    }
    fn activate(&mut self, index: usize) {
        if index < self.options.len() {
            self.cursor = index;
        }
    }
    fn step(&mut self, down: bool) {
        let len = self.options.len();
        if len == 0 {
            return;
        }
        self.cursor = if down {
            (self.cursor + 1) % len
        } else {
            (self.cursor + len - 1) % len
        };
    }
    fn confirm(&mut self, index: usize) -> Option<SharedString> {
        let value = self.options.get(index).cloned()?;
        self.activate(index);
        self.selected = Some(index);
        Some(value)
    }
    fn selected_value(&self) -> &str {
        self.selected
            .and_then(|i| self.options.get(i).map(|s| s.as_ref()))
            .unwrap_or("")
    }
}
pub struct SelectState {
    selection: Selection,
    open: bool,
    trigger: FocusHandle,
    popup: FocusHandle,
    _focus_subscriptions: Vec<Subscription>,
    label: SharedString,
    width: Pixels,
    height: Pixels,
    compact: bool,
    border_focus: bool,
    trigger_style: StyleRefinement,
    disabled: bool,
    pub translated_options: usize,
}
impl EventEmitter<SelectEvent> for SelectState {}
impl EventEmitter<DismissEvent> for SelectState {}
impl SelectState {
    pub fn new(
        options: Vec<impl Into<SharedString>>,
        selected: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let trigger = cx.focus_handle();
        let enter = cx.on_focus(&trigger, window, |_, _, cx| cx.notify());
        let leave = cx.on_blur(&trigger, window, |_, _, cx| cx.notify());
        Self {
            selection: Selection::new(options, selected.map(|i| i.row)),
            open: false,
            trigger,
            popup: cx.focus_handle(),
            _focus_subscriptions: vec![enter, leave],
            label: "".into(),
            width: px(t::SELECT_WIDTH),
            height: px(t::CONTROL),
            compact: false,
            border_focus: false,
            trigger_style: StyleRefinement::default(),
            disabled: false,
            translated_options: usize::MAX,
        }
    }
    pub fn set_options(
        &mut self,
        options: Vec<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.close(false, window, cx);
        self.selection = Selection::new(options, Some(0));
        cx.notify();
    }
    pub fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selection.set_selected(index.map(|i| i.row), self.open);
        cx.notify();
    }
    fn set_open(&mut self, open: bool, window: &mut Window, cx: &mut Context<Self>) {
        let open = open && !self.disabled && !self.selection.options.is_empty();
        if open {
            if !self.open {
                self.selection.cursor = self.selection.selected.unwrap_or(0);
            }
            self.open = true;
            cx.notify();
        } else {
            self.close(false, window, cx);
        }
    }
    /// Confirm/Escape 明确回到 trigger；outside 等本次点击完成后再判断，保留新控件焦点。
    fn close(&mut self, restore: bool, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            return;
        }
        self.open = false;
        if restore {
            self.trigger.focus(window, cx);
        } else {
            cx.defer_in(window, |this, window, cx| {
                if !this.open
                    && (window.focused(cx).is_none() || this.popup.contains_focused(window, cx))
                {
                    this.trigger.focus(window, cx);
                }
            });
        }
        cx.emit(DismissEvent);
        cx.notify();
    }
    fn move_cursor(&mut self, down: bool, cx: &mut Context<Self>) {
        self.selection.step(down);
        cx.notify();
    }
    fn confirm(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.disabled {
            return;
        }
        let Some(value) = self.selection.confirm(index) else {
            return;
        };
        self.close(true, window, cx);
        // 展示翻译不进入业务：始终返回 catalog 的稳定原值。
        cx.emit(SelectEvent::Confirm(Some(value)));
        cx.notify();
    }
}
#[derive(IntoElement)]
pub struct Select {
    state: Entity<SelectState>,
    label: SharedString,
    style: StyleRefinement,
    disabled: bool,
    compact: bool,
    border_focus: bool,
}
impl Select {
    /// 对应 .group-field > .ob-select-trigger:focus 的局部边框覆盖。
    pub fn border_focus(mut self) -> Self {
        self.border_focus = true;
        self
    }

    /// OpenBox 成员筛选的 24px 紧凑样式；两侧共用，保留普通 Select 外观。
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }
    pub fn new(state: &Entity<SelectState>, label: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            label: label.into(),
            style: StyleRefinement::default(),
            disabled: false,
            compact: false,
            border_focus: false,
        }
    }
}
impl gpui_kit::component::Disableable for Select {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}
impl Styled for Select {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for Select {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        self.state.update(cx, |s, _| {
            s.label = self.label;
            s.disabled = self.disabled;
            s.compact = self.compact;
            s.border_focus = self.border_focus;
            s.trigger_style = self.style.clone();
            s.height = self
                .style
                .size
                .height
                .and_then(|h| match h {
                    Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(p))) => {
                        Some(p)
                    }
                    _ => None,
                })
                .unwrap_or(px(t::CONTROL));
            s.width = self
                .style
                .size
                .width
                .and_then(|w| match w {
                    Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(p))) => {
                        Some(p)
                    }
                    _ => None,
                })
                .unwrap_or(px(t::SELECT_WIDTH));
        });
        div()
            .w(self.state.read(cx).width)
            .h(self.state.read(cx).height)
            .child(self.state)
    }
}
impl Render for SelectState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let popup_entity = entity.clone();
        let open_entity = entity.clone();
        let selected = if self
            .selection
            .selected
            .is_some_and(|i| i < self.translated_options)
        {
            tr(cx, self.selection.selected_value())
        } else {
            self.selection.selected_value()
        }
        .to_owned();
        BaseSelect::new("select")
            .open(self.open)
            .disabled(self.disabled || self.selection.options.is_empty())
            .focus_handle(&self.trigger)
            .content_focus_handle(&self.popup)
            .accessibility_label(tr(cx, &self.label).to_owned())
            .accessibility_value(selected.clone())
            .on_open_change(move |open, w, cx| {
                open_entity.update(cx, |s, cx| s.set_open(open, w, cx))
            })
            .on_confirm(move |w, cx| {
                entity.update(cx, |s, cx| s.confirm(s.selection.cursor, w, cx))
            })
            .child(
                Popover::new("select-popup")
                    .open(self.open)
                    .offset(px(t::MENU_PADDING))
                    .track_focus(&self.popup)
                    .on_open_change(cx.listener(|this, open, w, cx| {
                        this.set_open(*open, w, cx);
                    }))
                    .trigger(
                        gpui_kit::base::Button::new("select-trigger")
                            .disabled(self.disabled || self.selection.options.is_empty())
                            .tab_stop(false)
                            .opacity(if self.disabled { 0.5 } else { 1. })
                            .font_weight(crate::ui::theme::MISANS_REGULAR)
                            .flex()
                            .w(self.width)
                            .h(self.height)
                            .text_size(px(if self.compact && self.height < px(t::CONTROL) {
                                t::groups::FILTER_TEXT
                            } else {
                                t::BODY
                            }))
                            .items_center()
                            .justify_between()
                            .px(px(if self.compact { t::GAP } else { t::PAD }))
                            .gap(px(t::ROW_GAP))
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(px(if self.compact && self.height < px(t::CONTROL) {
                                t::groups::COMPACT_RADIUS
                            } else {
                                t::RADIUS
                            }))
                            .bg(if self.compact {
                                cx.theme().popover
                            } else {
                                cx.theme().input
                            })
                            .text_color(cx.theme().foreground)
                            .cursor_pointer()
                            .focus_visible(|s| s.border_color(rgb(t::ACCENT)))
                            .refine_style(&self.trigger_style)
                            // 焦点只由 BaseSelect 登记；Escape 回焦后按 React 绘制触发器边框。
                            .when(
                                self.border_focus
                                    && !self.disabled
                                    && self.trigger.is_focused(window),
                                |b| b.border_color(rgb(t::ACCENT)),
                            )
                            .child(selected)
                            .child(
                                icon(
                                    "ChevronDown",
                                    if self.compact {
                                        t::groups::MEMBER_BADGE_TEXT
                                    } else {
                                        t::BODY
                                    },
                                )
                                .text_color(cx.theme().muted_foreground),
                            ),
                    )
                    .content(move |_, _, cx| {
                        let dark = cx.theme().mode.is_dark();
                        let highlight = if dark {
                            0x57c98b26
                        } else {
                            t::SELECT_HIGHLIGHT
                        };
                        let highlight_text = if dark { 0x86dcae } else { t::SELECT_TEXT };
                        let s = popup_entity.read(cx);
                        let width = s.width;
                        let cursor = s.selection.cursor;
                        let selected = s.selection.selected;
                        let options = s.selection.options.clone();
                        let translated_options = s.translated_options;
                        let focus = s.popup.clone();
                        let keyboard = popup_entity.clone();
                        div()
                            .id("select-options")
                            .role(Role::ListBox)
                            .track_focus(&focus)
                            .w(width)
                            .p(px(t::MENU_PADDING))
                            .rounded(px(t::MENU_RADIUS))
                            .shadow(vec![
                                BoxShadow::new(
                                    px(0.),
                                    px(if dark { 14. } else { 10. }),
                                    rgba(if dark { 0x00000059 } else { 0x1f272d29 }).into(),
                                )
                                .blur_radius(px(if dark {
                                    38.
                                } else {
                                    34.
                                })),
                            ])
                            .border_1()
                            .border_color(cx.theme().border)
                            .bg(if cx.theme().mode.is_dark() {
                                rgba(0x2a3030fc)
                            } else {
                                rgba(0xfffffffa)
                            })
                            .text_color(cx.theme().foreground)
                            .key_context("Select")
                            .on_action({
                                let entity = keyboard.clone();
                                move |_: &gpui_kit::base::actions::SelectDown, _, cx| {
                                    entity.update(cx, |s, cx| {
                                        s.move_cursor(true, cx);
                                    });
                                    cx.stop_propagation();
                                }
                            })
                            .on_action({
                                let entity = keyboard.clone();
                                move |_: &gpui_kit::base::actions::SelectUp, _, cx| {
                                    entity.update(cx, |s, cx| {
                                        s.move_cursor(false, cx);
                                    });
                                    cx.stop_propagation();
                                }
                            })
                            .on_action({
                                let entity = keyboard.clone();
                                move |_: &gpui_kit::base::actions::Confirm, w, cx| {
                                    entity.update(cx, |s, cx| s.confirm(s.selection.cursor, w, cx));
                                    cx.stop_propagation();
                                }
                            })
                            .on_action(move |_: &gpui_kit::base::actions::Cancel, w, cx| {
                                keyboard.update(cx, |s, cx| {
                                    s.close(true, w, cx);
                                });
                                cx.stop_propagation();
                            })
                            .children(options.into_iter().enumerate().map(|(i, label)| {
                                let entity = popup_entity.clone();
                                let hover_entity = entity.clone();
                                div()
                                    .id(("option", i))
                                    .role(Role::ListBoxOption)
                                    .aria_selected(selected == Some(i))
                                    .aria_label(if i < translated_options {
                                        tr(cx, &label).to_owned()
                                    } else {
                                        label.to_string()
                                    })
                                    .flex()
                                    .h(px(t::CONTROL))
                                    .items_center()
                                    .justify_between()
                                    .px(px(t::OPTION_PADDING))
                                    .rounded(px(t::OPTION_RADIUS))
                                    .cursor_pointer()
                                    .when(cursor == i, |d| {
                                        d.aria_active_descendant()
                                            .bg(rgba(highlight))
                                            .text_color(rgb(highlight_text))
                                    })
                                    .when(selected == Some(i), |d| {
                                        d.font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(highlight_text))
                                    })
                                    // hover 与键盘共用 cursor，不另画第二个 active 背景。
                                    .on_hover(move |hovered, _, cx| {
                                        if *hovered {
                                            hover_entity.update(cx, |s, cx| {
                                                s.selection.activate(i);
                                                cx.notify();
                                            });
                                        }
                                    })
                                    .child(if i < translated_options {
                                        tr(cx, &label).to_owned()
                                    } else {
                                        label.to_string()
                                    })
                                    .when(selected == Some(i), |d| {
                                        d.child(icon("Check", 14.).text_color(rgb(highlight_text)))
                                    })
                                    .on_click(move |_, w, cx| {
                                        entity.update(cx, |s, cx| s.confirm(i, w, cx))
                                    })
                            }))
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::Selection;
    // 保护键盘/鼠标混用：一个 cursor 决定唯一高亮、AX active 与 Enter 的业务原值。
    #[test]
    fn hover_replaces_keyboard_cursor_before_confirm() {
        let mut s = Selection::new(vec!["a", "b", "c"], Some(0));
        s.step(true);
        assert_eq!((s.selected, s.cursor), (Some(0), 1));
        s.activate(2);
        assert_eq!((s.selected, s.cursor), (Some(0), 2));
        assert_eq!((0..3).filter(|&i| s.cursor == i).collect::<Vec<_>>(), [2]);
        assert_eq!(s.confirm(s.cursor), Some("c".into()));
        assert_eq!(s.selected, Some(2));
    }
    #[test]
    fn empty_options_and_invalid_selection_are_normalized() {
        let mut empty = Selection::new(Vec::<gpui_kit::SharedString>::new(), Some(9));
        empty.step(true);
        empty.step(false);
        empty.activate(9);
        assert_eq!(
            (empty.selected, empty.cursor, empty.selected_value()),
            (None, 0, "")
        );
        assert_eq!(empty.confirm(0), None);
        let mut s = Selection::new(vec!["a", "b"], Some(9));
        assert_eq!((s.selected, s.cursor), (None, 0));
        assert_eq!(s.confirm(9), None);
        s.set_selected(Some(1), false);
        assert_eq!((s.selected, s.cursor), (Some(1), 1));
        s.set_selected(Some(9), false);
        assert_eq!((s.selected, s.cursor), (None, 0));
    }
    #[test]
    fn arrows_wrap_and_projection_does_not_commit_cursor() {
        let mut s = Selection::new(vec!["简体中文", "繁體中文", "English"], Some(0));
        s.step(false);
        assert_eq!((s.selected, s.cursor), (Some(0), 2));
        s.step(true);
        assert_eq!(s.cursor, 0);
        s.step(true);
        s.set_selected(Some(2), true);
        assert_eq!((s.selected, s.cursor), (Some(2), 1));
        assert_eq!(s.confirm(s.cursor), Some("繁體中文".into()));
    }
}
