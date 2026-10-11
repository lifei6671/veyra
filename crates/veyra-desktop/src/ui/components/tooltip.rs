//! React 说明浮层：hover 和键盘 focus 共用外观，Escape 隐藏至下一次进入。
use crate::ui::tokens as t;
use gpui_kit::base::StyledExt as _;
use gpui_kit::component::{ActiveTheme, Sizable};
use gpui_kit::{base::Button, prelude::*, *};

// 所有自绘提示共用同一字体和颜色，Kit 仅负责触发/定位。
pub fn bubble(_cx: &App) -> Div {
    div()
        .bg(rgb(t::TOOLTIP_BG))
        .text_color(rgb(t::TOOLTIP_TEXT))
        .font(t::openbox_font())
        .text_size(px(t::TOOLTIP_FONT))
        .line_height(px(t::TOOLTIP_LINE))
        .font_weight(crate::ui::tokens::FONT_REGULAR)
}
pub struct TooltipContent(pub SharedString);
impl Render for TooltipContent {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        bubble(cx)
            .px(px(t::GAP))
            .py(px(t::TOOLTIP_Y))
            .rounded(px(t::TOOLTIP_RADIUS))
            .max_w(px(t::SITE_TOOLTIP_WIDTH))
            .child(crate::ui::i18n::message(cx, &self.0))
    }
}

/// ProxyTool 在 toolbar 和订阅 header 的真实复用；用触发器 bounds 定位，避免跟随鼠标。
#[derive(IntoElement)]
pub struct ToolTooltip {
    pub id: ElementId,
    pub trigger: AnyElement,
    pub text: SharedString,
    pub right: bool,
    pub x_offset: f32,
    pub focus: FocusHandle,
}
impl RenderOnce for ToolTooltip {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = self.focus.clone();
        let state = window.use_keyed_state(
            (self.id.clone(), "tool-tooltip-hover"),
            cx,
            move |window, cx| TooltipState::with_focus(focus, window, cx),
        );
        let s = state.read(cx);
        let visible = tooltip_visible(
            s.hovered,
            self.focus.is_focused(window),
            window.last_input_was_keyboard(),
            s.dismissed,
        );
        let alpha = gpui_kit::base::motion::transition(
            (self.id.clone(), "tool-tooltip-alpha"),
            if visible { 1_f32 } else { 0_f32 },
            gpui_kit::base::motion::Transition::new(std::time::Duration::from_millis(
                t::TOOL_TOOLTIP_TRANSITION_MS,
            ))
            .ease(gpui_kit::component::animation::cubic_bezier(
                0.25, 0.1, 0.25, 1.,
            ))
            .delay(std::time::Duration::from_millis(if visible {
                t::TOOL_TOOLTIP_DELAY_MS
            } else {
                0
            })),
            window,
            cx,
        );
        let hover = state.clone();
        div().id((self.id, "tool-tooltip")).relative().child(self.trigger)
            .on_hover(move |hovered, _, cx| hover.update(cx, |s, cx| {
                s.hovered = *hovered;
                if *hovered { s.dismissed = false; }
                cx.notify();
            }))
            .on_key_down(move |event, _, cx| {
                if event.keystroke.key == "escape" {
                    state.update(cx, |s, cx| { s.dismissed = true; cx.notify(); });
                    cx.stop_propagation();
                }
            })
            .when(visible || alpha > 0., |d| {
                // CSS width:max-content + border-box max-width：短文案不能拉满160px。
                let text = crate::ui::i18n::message(cx, &self.text);
                let width = f32::from(window.text_system().shape_line(
                    text.clone().into(), px(t::TOOLTIP_FONT),
                    &[TextRun { len: text.len(), font: Font { weight: crate::ui::tokens::FONT_REGULAR, ..t::openbox_font() }, color: rgb(t::TOOLTIP_TEXT).into(), background_color: None, underline: None, strikethrough: None }], None,
                ).width);
                let content_limit = t::TOOL_TOOLTIP_MAX_WIDTH - 2. * t::TOOL_TOOLTIP_PX;
                let popup = bubble(cx).relative()
                    .w(px(width.min(content_limit) + 2. * t::TOOL_TOOLTIP_PX))
                    // max-content 的短文案不参与断行；仅超过 CSS 上限时自动换行。
                    // 保留精确盒宽，避免 GPUI 浮点度量在整字边界多折一行。
                    .when(width <= content_limit, |d| d.whitespace_nowrap())
                    .px(px(t::TOOL_TOOLTIP_PX)).py(px(t::TOOLTIP_Y))
                    .max_w(px(t::TOOL_TOOLTIP_MAX_WIDTH))
                    .rounded(px(t::TOOL_TOOLTIP_RADIUS))
                    // 代理浮层先合成再淡入，避免文字、箭头与背景逐项叠加透明度。
                    // 终态保留原绘制路径，不让其它页面的普通 Tooltip 消费该机制。
                    .when(alpha < 1., |popup| popup.group_opacity(alpha))
                    .shadow(vec![BoxShadow::new(px(0.), px(t::TOOL_TOOLTIP_SHADOW_Y), rgba(t::TOOL_TOOLTIP_SHADOW).into()).blur_radius(px(t::TOOL_TOOLTIP_SHADOW_BLUR))])
                    .child(text)
                    .on_children_prepainted(|bounds, _, _| {
                        #[cfg(debug_assertions)]
                        super::log_layout("proxy-tool-tooltip-content", &bounds);
                    })
                    .child(div().absolute().top(px(-t::TOOL_TOOLTIP_ARROW))
                        .when(self.right, |d| d.right(px(t::TOOL_TOOLTIP_ARROW_RIGHT)))
                        .when(!self.right, |d| d.left(relative(0.5)).ml(px(-t::TOOL_TOOLTIP_ARROW)))
                        .child(gpui_kit::component::Icon::default().data(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 5"><path d="M0 5 5 0 10 5Z" fill="currentColor"/></svg>"#.as_slice()).w(px(2. * t::TOOL_TOOLTIP_ARROW)).h(px(t::TOOL_TOOLTIP_ARROW)).text_color(rgb(t::TOOLTIP_BG))));
                d.child(div().absolute().top_full()
                    .when(self.right, |d| d.right_0())
                    .when(!self.right, |d| d.left(relative(0.5)))
                    .child(deferred(anchored().position_mode(AnchoredPositionMode::Local)
                        .position(point(px(self.x_offset), px(t::TOOL_TOOLTIP_OFFSET - t::TOOL_TOOLTIP_RISE * (1. - alpha))))
                        .anchor(if self.right { Anchor::TopRight } else { Anchor::TopCenter })
                        .snap_to_window().child(popup)).with_priority(t::TOOL_TOOLTIP_PRIORITY)))
            })
    }
}

// 鼠标点击会保留按钮焦点，但不能因此让 hover 提示在移开后继续显示。
// 键盘聚焦仍可查看提示，Escape 的主动关闭优先于两种显示来源。
fn tooltip_visible(hovered: bool, focused: bool, keyboard: bool, dismissed: bool) -> bool {
    (hovered || (focused && keyboard)) && !dismissed
}

struct TooltipState {
    focus: FocusHandle,
    hovered: bool,
    dismissed: bool,
    _subscriptions: Vec<Subscription>,
}
impl TooltipState {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus = cx.focus_handle();
        Self::with_focus(focus, window, cx)
    }
    fn with_focus(focus: FocusHandle, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let enter = cx.on_focus(&focus, window, |s, _, cx| {
            s.dismissed = false;
            cx.notify();
        });
        let leave = cx.on_blur(&focus, window, |s, _, cx| {
            s.dismissed = false;
            cx.notify();
        });
        Self {
            focus,
            hovered: false,
            dismissed: false,
            _subscriptions: vec![enter, leave],
        }
    }
}
#[derive(IntoElement)]
pub struct TooltipButton {
    button: Button,
    id: &'static str,
    text: &'static str,
    sidebar: bool,
    sidebar_collapsed: bool,
    style: StyleRefinement,
}
impl Styled for TooltipButton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
pub fn with_tooltip(
    button: Button,
    id: &'static str,
    text: &'static str,
    sidebar: bool,
) -> TooltipButton {
    TooltipButton {
        button,
        id,
        text,
        sidebar,
        sidebar_collapsed: false,
        style: StyleRefinement::default(),
    }
}
impl TooltipButton {
    pub fn sidebar_collapsed(mut self, collapsed: bool) -> Self {
        self.sidebar_collapsed = collapsed;
        self
    }
}
impl RenderOnce for TooltipButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id, cx, TooltipState::new);
        let s = state.read(cx);
        let visible = tooltip_visible(
            s.hovered,
            s.focus.is_focused(window),
            window.last_input_was_keyboard(),
            s.dismissed,
        );
        let focus = s.focus.clone();
        let hover = state.clone();
        let sites = self.id == "sites-help";
        let restore = self.id == "restore-sites-tooltip";
        // anchored + deferred 避免设置滚动区域裁切；不移动触发器焦点。
        div()
            .id(self.id)
            .relative()
            .refine_style(&self.style)
            .when(!self.sidebar && !restore, |d| d.size(px(t::ICON_SMALL)))
            .on_hover(move |hovered, _, cx| {
                hover.update(cx, |s, cx| {
                    s.hovered = *hovered;
                    if *hovered {
                        s.dismissed = false;
                    }
                    cx.notify();
                })
            })
            .on_key_down(move |event, _, cx| {
                if event.keystroke.key == "escape" {
                    state.update(cx, |s, cx| {
                        s.dismissed = true;
                        cx.notify();
                    });
                    cx.stop_propagation();
                }
            })
            .child(
                self.button
                    .relative()
                    .when(
                        focus.is_focused(window) && window.last_input_was_keyboard(),
                        |b| {
                            b.child(super::focus_outline(
                                t::browser_focus(cx.theme().mode.is_dark()),
                                if restore || self.sidebar {
                                    t::RADIUS
                                } else {
                                    0.
                                },
                                0.,
                            ))
                        },
                    )
                    .track_focus(&focus)
                    .on_mouse_down(MouseButton::Left, move |_, w, cx| focus.focus(w, cx)),
            )
            .when(visible, |d| {
                let background = t::TOOLTIP_BG;
                let popup = bubble(cx)
                    .relative()
                    .px(px(if self.sidebar {
                        t::COLLAPSE_TOOLTIP_X
                    } else {
                        t::GAP
                    }))
                    .py(px(t::TOOLTIP_Y))
                    .rounded(px(if self.sidebar {
                        t::RADIUS
                    } else {
                        t::TOOLTIP_RADIUS
                    }))
                    .shadow(vec![
                        BoxShadow::new(px(0.), px(4.), rgba(0x00000024).into())
                            .blur_radius(px(12.)),
                    ])
                    .when(!self.sidebar && !restore, |d| {
                        d.w(px(if sites {
                            t::SITE_TOOLTIP_WIDTH
                        } else {
                            t::IP_TOOLTIP_WIDTH
                        }))
                    })
                    .when(!sites && !self.sidebar, |d| d.text_center())
                    .child(crate::ui::i18n::tr(cx, self.text))
                    .when(self.sidebar, |d| {
                        // React ::before 是 8px 正方形旋转45°；外接尺寸8√2，
                        // 中心在顶部/左侧边缘，颜色继承 tooltip 背景。
                        let diameter = t::TOOLTIP_ARROW_SIZE * std::f32::consts::SQRT_2;
                        let arrow = gpui_kit::component::Icon::default()
                            .data(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 8"><path d="M4 0 8 4 4 8 0 4Z" fill="currentColor"/></svg>"#.as_slice())
                            .with_size(px(diameter))
                            .text_color(rgb(background))
                            .absolute();
                        d.child(if self.sidebar_collapsed {
                            arrow.left(px(-diameter / 2.)).top(relative(0.5)).mt(px(-diameter / 2.))
                        } else {
                            arrow.top(px(-diameter / 2.)).left(relative(0.5)).ml(px(-diameter / 2.))
                        })
                    });
                // Local 使用 anchored 自身的布局原点；用绝对定位容器固定在
                // trigger 左上角，不能把 trigger 高度再次加到 CSS offset 上。
                d.child(
                    div().absolute().top_0().left_0().child(
                        deferred(
                            anchored()
                                .position_mode(AnchoredPositionMode::Local)
                                .position(point(
                                    px(if self.sidebar && self.sidebar_collapsed {
                                        t::NAV_HEIGHT + t::TOOLTIP_OFFSET
                                    } else if self.sidebar {
                                        t::NAV_HEIGHT / 2.
                                    } else if restore {
                                        t::CONTROL
                                    } else if sites {
                                        0.
                                    } else {
                                        (t::ICON_SMALL - t::IP_TOOLTIP_WIDTH) / 2.
                                    }),
                                    px(if sites {
                                        -t::TOOLTIP_OFFSET
                                    } else if self.sidebar && self.sidebar_collapsed {
                                        t::NAV_HEIGHT / 2.
                                    } else if self.sidebar {
                                        t::NAV_HEIGHT + t::TOOLTIP_OFFSET
                                    } else if restore {
                                        t::CONTROL + t::TOOLTIP_OFFSET
                                    } else {
                                        t::ICON_SMALL + t::TOOLTIP_OFFSET
                                    }),
                                ))
                                .anchor(if self.sidebar && self.sidebar_collapsed {
                                    Anchor::LeftCenter
                                } else if self.sidebar {
                                    Anchor::TopCenter
                                } else if restore {
                                    Anchor::TopRight
                                } else if sites {
                                    Anchor::BottomLeft
                                } else {
                                    Anchor::TopLeft
                                })
                                .snap_to_window()
                                .child(popup),
                        )
                        .with_priority(1),
                    ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::tooltip_visible;

    #[test]
    fn pointer_focus_does_not_keep_tooltip_after_mouse_leaves() {
        assert!(tooltip_visible(true, true, false, false));
        assert!(!tooltip_visible(false, true, false, false));
    }

    #[test]
    fn keyboard_focus_shows_tooltip_until_blur_or_escape() {
        assert!(tooltip_visible(false, true, true, false));
        assert!(!tooltip_visible(false, false, true, false));
        assert!(!tooltip_visible(true, true, true, true));
    }
}
