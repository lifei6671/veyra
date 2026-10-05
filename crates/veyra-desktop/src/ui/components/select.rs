//! 有限选项 Select：Kit base 管理焦点、Escape、弹层生命周期；React CSS 定义外观。
use crate::ui::{icons::icon, tokens as t};
use gpui_kit::{
    base::{Popover, Select as BaseSelect},
    component::{ActiveTheme, IndexPath},
    prelude::*,
    *,
};
pub enum SelectEvent {
    Confirm(Option<&'static str>),
}
pub struct SelectState {
    options: Vec<&'static str>,
    selected: Option<usize>,
    cursor: usize,
    open: bool,
    trigger: FocusHandle,
    popup: FocusHandle,
    label: SharedString,
    width: Pixels,
}
impl EventEmitter<SelectEvent> for SelectState {}
impl EventEmitter<DismissEvent> for SelectState {}
impl SelectState {
    pub fn new(
        options: Vec<&'static str>,
        selected: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            options,
            selected: selected.map(|i| i.row),
            cursor: selected.map_or(0, |i| i.row),
            open: false,
            trigger: cx.focus_handle(),
            popup: cx.focus_handle(),
            label: "".into(),
            width: px(t::SELECT_WIDTH),
        }
    }
    pub fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.selected = index.map(|i| i.row);
        cx.notify();
    }
    fn confirm(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.selected = Some(index);
        self.open = false;
        self.trigger.focus(window, cx);
        cx.emit(DismissEvent);
        cx.emit(SelectEvent::Confirm(Some(self.options[index])));
        cx.notify();
    }
}
#[derive(IntoElement)]
pub struct Select {
    state: Entity<SelectState>,
    label: SharedString,
    style: StyleRefinement,
}
impl Select {
    pub fn new(state: &Entity<SelectState>, label: impl Into<SharedString>) -> Self {
        Self {
            state: state.clone(),
            label: label.into(),
            style: StyleRefinement::default(),
        }
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
            .h(px(t::CONTROL))
            .child(self.state)
    }
}
impl Render for SelectState {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entity = cx.entity();
        let popup_entity = entity.clone();
        let open_entity = entity.clone();
        let selected = self.selected.map_or("", |i| self.options[i]);
        BaseSelect::new("select")
            .open(self.open)
            .focus_handle(&self.trigger)
            .content_focus_handle(&self.popup)
            .accessibility_label(self.label.clone())
            .accessibility_value(selected)
            .on_open_change(move |open, _, cx| {
                open_entity.update(cx, |s, cx| {
                    s.open = open;
                    s.cursor = s.selected.unwrap_or(0);
                    cx.notify();
                })
            })
            .on_confirm(move |w, cx| entity.update(cx, |s, cx| s.confirm(s.cursor, w, cx)))
            .child(
                Popover::new("select-popup")
                    .open(self.open)
                    .offset(px(t::MENU_PADDING))
                    .track_focus(&self.popup)
                    .on_open_change(cx.listener(|this, open, _, cx| {
                        this.open = *open;
                        if *open {
                            this.cursor = this.selected.unwrap_or(0);
                        } else {
                            cx.emit(DismissEvent);
                        }
                        cx.notify();
                    }))
                    .trigger(
                        gpui_kit::base::Button::new("select-trigger")
                            .flex()
                            .w(self.width)
                            .h(px(t::CONTROL))
                            .items_center()
                            .justify_between()
                            .px(px(t::PAD))
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(px(t::RADIUS))
                            .bg(cx.theme().input)
                            .text_color(cx.theme().foreground)
                            .cursor_pointer()
                            .focus_visible(|s| s.border_color(rgb(t::ACCENT)))
                            .child(selected)
                            .child(
                                icon("ChevronDown", 14.).text_color(cx.theme().muted_foreground),
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
                        let cursor = s.cursor;
                        let selected = s.selected;
                        let options = s.options.clone();
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
                                        s.cursor = (s.cursor + 1) % s.options.len();
                                        cx.notify();
                                    });
                                    cx.stop_propagation();
                                }
                            })
                            .on_action({
                                let entity = keyboard.clone();
                                move |_: &gpui_kit::base::actions::SelectUp, _, cx| {
                                    entity.update(cx, |s, cx| {
                                        s.cursor =
                                            (s.cursor + s.options.len() - 1) % s.options.len();
                                        cx.notify();
                                    });
                                    cx.stop_propagation();
                                }
                            })
                            .on_action({
                                let entity = keyboard.clone();
                                move |_: &gpui_kit::base::actions::Confirm, w, cx| {
                                    entity.update(cx, |s, cx| s.confirm(s.cursor, w, cx));
                                    cx.stop_propagation();
                                }
                            })
                            .on_action(move |_: &gpui_kit::base::actions::Cancel, w, cx| {
                                keyboard.update(cx, |s, cx| {
                                    s.open = false;
                                    s.trigger.focus(w, cx);
                                    cx.emit(DismissEvent);
                                    cx.notify();
                                });
                                cx.stop_propagation();
                            })
                            .children(options.into_iter().enumerate().map(|(i, label)| {
                                let entity = popup_entity.clone();
                                div()
                                    .id(("option", i))
                                    .role(Role::ListBoxOption)
                                    .aria_selected(selected == Some(i))
                                    .aria_label(label)
                                    .flex()
                                    .h(px(t::CONTROL))
                                    .items_center()
                                    .justify_between()
                                    .px(px(t::OPTION_PADDING))
                                    .rounded(px(t::OPTION_RADIUS))
                                    .cursor_pointer()
                                    .when(cursor == i, |d| {
                                        d.bg(rgba(highlight)).text_color(rgb(highlight_text))
                                    })
                                    .when(selected == Some(i), |d| {
                                        d.font_weight(FontWeight::SEMIBOLD)
                                            .text_color(rgb(highlight_text))
                                    })
                                    .hover(|d| d.bg(rgba(highlight)))
                                    .child(label)
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
