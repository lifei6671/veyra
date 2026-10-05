//! Mature Kit controls with Veyra sizing/theme; overlay ownership remains window-level Kit Root.
use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::*;
use gpui_kit::{
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonCustomVariant, ButtonVariant, ButtonVariants},
        input::{Input, InputState},
    },
    *,
};
#[derive(IntoElement)]
pub struct PanelInput {
    state: Entity<InputState>,
    style: StyleRefinement,
    number: bool,
    focus_style: Option<(Hsla, f32)>,
}
impl PanelInput {
    // 当前图标搜索框有 CSS 局部 focus 颜色/offset，输入语义仍由 Kit 管理。
    pub fn focus_style(mut self, color: Hsla, offset: f32) -> Self {
        self.focus_style = Some((color, offset));
        self
    }
    pub fn numeric(mut self) -> Self {
        self.number = true;
        self
    }
}
impl Styled for PanelInput {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}
impl RenderOnce for PanelInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use super::tokens as t;
        use gpui_kit::component::ActiveTheme;
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        let mut layout = self.style.clone();
        layout.padding = Default::default();
        // 输入/IME 仍由 Kit 管理；禁用 Kit 的厚 shadow，复用 reference 的细 outline。
        div()
            .relative()
            .h(px(t::CONTROL))
            .refine_style(&layout)
            .child(Styled::h(
                Input::new(&self.state)
                    .focus_bordered(false)
                    .rounded(px(t::RADIUS))
                    .border_color(cx.theme().border)
                    .bg(cx.theme().input)
                    .px(px(t::PAD))
                    .py(px(t::ROW_GAP))
                    .text_size(px(t::BODY))
                    .line_height(px(t::BODY_LINE))
                    .refine_style(&self.style),
                px(t::CONTROL),
            ))
            .when(focused, |d| {
                let (color, offset) = self.focus_style.unwrap_or((rgb(t::browser_focus(cx.theme().mode.is_dark())).into(), t::INPUT_FOCUS_OFFSET));
                d.child(focus_outline(t::browser_focus(cx.theme().mode.is_dark()), t::RADIUS, offset).border_color(color))
            })
            .when(self.number, |d| {
                // Chromium number-field 的 UA spinner 仅 hover/focus 可见。
                // 仍通过同一个 InputState Change 更新草稿，不能绕过验证/CAS保存。
                d.group("number-field").child(
                    div()
                        .absolute()
                        .right(px(t::PAD))
                        .top(px((t::CONTROL - t::SPINNER_HEIGHT) / 2.))
                        .w(px(t::SPINNER_WIDTH))
                        .h(px(t::SPINNER_HEIGHT))
                        .flex()
                        .flex_col()
                        .bg(rgb(if cx.theme().mode.is_dark() { t::SPINNER_DARK_BG } else { t::SPINNER_BG }))
                        .when(!focused, |d| d.invisible().group_hover("number-field", |s| s.visible()))
                        .children([true, false].map(|increment| {
                            use gpui_kit::component::Sizable;
                            let state = self.state.clone();
                            let arrow: &[u8] = if increment {
                                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 4"><path d="M0 4 4 0 8 4Z" fill="currentColor"/></svg>"#
                            } else {
                                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 4"><path d="M0 0 4 4 8 0Z" fill="currentColor"/></svg>"#
                            };
                            gpui_kit::base::Button::new((if increment { "number-up" } else { "number-down" }, state.entity_id()))
                                .tab_stop(false)
                                .accessibility_label(if increment { "增加数值" } else { "减少数值" })
                                .w(px(t::SPINNER_WIDTH))
                                .h(px(t::SPINNER_HEIGHT / 2.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .p_0()
                                .text_color(rgb(if cx.theme().mode.is_dark() { t::SPINNER_DARK_TEXT } else { t::SPINNER_TEXT }))
                                .child(gpui_kit::component::Icon::default().data(arrow).with_size(px(t::SPINNER_ARROW)).h(px(t::SPINNER_ARROW / 2.)))
                                .on_click(move |_, window, cx| {
                                    state.update(cx, |input, cx| {
                                        let value = input.value().parse::<u64>().unwrap_or(0);
                                        let next = if increment { value.saturating_add(1) } else { value.saturating_sub(1) };
                                        input.set_value(next.to_string(), window, cx);
                                        input.focus_handle(cx).focus(window, cx);
                                    });
                                })
                        })),
                )
            })
    }
}
// 当前 Input / Switch / IconButton 共用描边；空心边框不会染色半透明 field。
fn focus_outline(color: u32, radius: f32, offset: f32) -> Div {
    let extent = 2. + offset;
    div()
        .absolute()
        .top(-px(extent))
        .left(-px(extent))
        .right(-px(extent))
        .bottom(-px(extent))
        .border_2()
        .border_color(rgb(color))
        .rounded(px(radius + extent))
}
struct ControlFocus {
    handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}
impl ControlFocus {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let handle = cx.focus_handle();
        let enter = cx.on_focus(&handle, window, |_, _, cx| cx.notify());
        let leave = cx.on_blur(&handle, window, |_, _, cx| cx.notify());
        Self {
            handle,
            _subscriptions: vec![enter, leave],
        }
    }
}

pub fn text_input(state: &Entity<InputState>) -> PanelInput {
    PanelInput {
        state: state.clone(),
        style: StyleRefinement::default(),
        number: false,
        focus_style: None,
    }
}
pub mod select;
pub fn select(
    state: &Entity<select::SelectState>,
    label: impl Into<SharedString>,
) -> select::Select {
    select::Select::new(state, label)
}
/// Kit base 只提供键盘/焦点/切换语义，轨道与 thumb 按 Panel CSS 自绘。
#[derive(IntoElement)]
pub struct PanelSwitch {
    control: gpui_kit::base::Switch,
    checked: bool,
    id: ElementId,
}
impl PanelSwitch {
    pub fn on_change(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.control = self
            .control
            .on_change(move |value, _, window, cx| handler(&value, window, cx));
        self
    }
}
impl RenderOnce for PanelSwitch {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        use gpui_kit::component::ActiveTheme;
        let focus = window
            .use_keyed_state((self.id, "switch-focus"), cx, ControlFocus::new)
            .read(cx)
            .handle
            .clone();
        let focused = focus.is_focused(window) && window.last_input_was_keyboard();
        let dark = cx.theme().mode.is_dark();
        let p = super::theme::Palette::new(dark, true);
        let thumb = if self.checked {
            rgb(p.text)
        } else if dark {
            rgba(p.muted)
        } else {
            rgba(0x4b526380)
        };
        self.control
            .relative()
            .track_focus(&focus)
            .w(px(super::tokens::SWITCH_WIDTH))
            .h(px(super::tokens::SWITCH_HEIGHT))
            .p(px(super::tokens::SWITCH_PADDING))
            .border_1()
            .border_color(if dark {
                rgba(p.muted)
            } else {
                rgba(0x4b52638c)
            })
            .rounded(px(super::tokens::SWITCH_RADIUS))
            .bg(if dark {
                cx.theme().input
            } else {
                rgba(0xffffffbf).into()
            })
            .when(focused, |s| {
                s.child(focus_outline(
                    super::tokens::SWITCH_FOCUS,
                    super::tokens::SWITCH_RADIUS,
                    2.,
                ))
            })
            .child(
                div()
                    .size(px(super::tokens::SWITCH_THUMB))
                    .rounded_full()
                    .bg(thumb)
                    .ml(px(if self.checked {
                        super::tokens::SWITCH_TRAVEL
                    } else {
                        0.
                    })),
            )
    }
}
pub fn toggle(id: &'static str, checked: bool, label: impl Into<SharedString>) -> PanelSwitch {
    PanelSwitch {
        id: id.into(),
        control: gpui_kit::base::Switch::new(id)
            .checked(checked)
            .accessibility_label(label),
        checked,
    }
}
/// Kit 保留点击与键盘行为；焦点装饰在内容裁切层外绘制。
#[derive(IntoElement)]
pub struct PanelButton {
    button: Button,
    id: &'static str,
    radius: f32,
    disabled: bool,
}
impl Styled for PanelButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}
impl ParentElement for PanelButton {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.button.extend(elements);
    }
}
impl PanelButton {
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button = self.button.on_click(handler);
        self
    }
    pub fn tab_stop(mut self, stop: bool) -> Self {
        self.button = self.button.tab_stop(stop);
        self
    }
    pub fn tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.button = self.button.tooltip(text);
        self
    }
    pub fn accessibility_label(mut self, text: impl Into<SharedString>) -> Self {
        self.button = self.button.accessibility_label(text);
        self
    }
}
impl Disableable for PanelButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.button = self.button.disabled(disabled);
        self
    }
}
impl ButtonVariants for PanelButton {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.button = self.button.with_variant(variant);
        self
    }
}
impl RenderOnce for PanelButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // 与 Kit Button 在同一父元素 scope 读取它的稳定焦点 id；放入 child 后
        // scope 已进入 Button/content，会读到另一个 handle 且被 overflow 裁切。
        let button = RenderOnce::render(self.button, window, cx).into_any_element();
        let focus = window
            .use_keyed_state(self.id, cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focused =
            !self.disabled && focus.is_focused(window) && window.last_input_was_keyboard();
        div()
            .relative()
            .flex_shrink_0()
            // Kit 主动抑制 mouse focus；React 按钮点击会让旧 Input blur。
            .when(!self.disabled, |d| {
                d.on_mouse_down(MouseButton::Left, move |_, window, cx| {
                    focus.focus(window, cx)
                })
            })
            .child(button)
            .when(focused, |d| {
                d.child(focus_outline(
                    super::tokens::browser_focus(cx.theme().mode.is_dark()),
                    self.radius,
                    0.,
                ))
            })
    }
}
pub fn button(id: &'static str, label: impl Into<SharedString>) -> PanelButton {
    use gpui_kit::component::FocusableExt as _;
    PanelButton {
        id,
        radius: super::tokens::RADIUS,
        disabled: false,
        button: Button::new(id)
            .focus_ring(false)
            .label(label)
            .border_0()
            .h(px(super::tokens::CONTROL))
            .rounded(px(super::tokens::RADIUS))
            .text_size(px(super::tokens::BODY)),
    }
}
pub mod notice;
pub use notice::{Notice, notify};

/// Stable application identity survives render; GPUI owns gesture/drop/cancellation lifetime.
#[derive(Clone)]
pub struct FileDrag {
    pub id: &'static str,
    pub path: std::path::PathBuf,
}
pub struct DragPreview(pub &'static str);
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .rounded_lg()
            .bg(rgb(super::tokens::ACCENT))
            .text_color(rgb(0x183c29))
            .child(format!("{} → drop background", self.0))
    }
}

pub mod overlay;

/// 同一导航样式供侧栏和设置分类使用，保留两种真实规格。
pub enum NavigationKind {
    Sidebar { collapsed: bool, panel: bool },
    Category,
}
pub fn navigation_item(
    id: &'static str,
    label: &'static str,
    glyph: &'static str,
    active: bool,
    kind: NavigationKind,
    dark: bool,
    cx: &App,
) -> PanelButton {
    use super::{icons::icon, tokens as t};
    let category = matches!(kind, NavigationKind::Category);
    let collapsed = matches!(
        kind,
        NavigationKind::Sidebar {
            collapsed: true,
            ..
        }
    );
    let panel = category || matches!(kind, NavigationKind::Sidebar { panel: true, .. });
    let p = super::theme::Palette::new(dark, panel);
    let active_bg = if dark {
        if panel { 0x1f352d } else { 0x76d8a5 }
    } else {
        0x4b5263
    };
    let active_text = if dark {
        if panel { p.text } else { 0x173228 }
    } else {
        0xfafafa
    };
    let background: Hsla = if active {
        rgb(active_bg).into()
    } else {
        rgba(0).into()
    };
    let foreground = rgb(if active { active_text } else { p.text });
    use gpui_kit::component::FocusableExt as _;
    let button = Button::new(id)
        .focus_ring(false)
        .border_0()
        .custom(
            ButtonCustomVariant::new(cx)
                .color(background)
                .foreground(foreground.into())
                .hover(if active {
                    background
                } else {
                    rgba(crate::ui::tokens::HOVER).into()
                })
                .active(background)
                .shadow(false),
        )
        .h(px(if category { t::CONTROL } else { t::NAV_HEIGHT }))
        .px(px(if collapsed { 0. } else { t::PAD }))
        .gap(px(t::GAP))
        .rounded(px(if category { t::RADIUS } else { t::NAV_RADIUS }))
        .text_size(px(t::BODY))
        .text_color(rgb(p.text))
        .font_weight(FontWeight::NORMAL)
        .child(
            div()
                .flex()
                .w_full()
                .items_center()
                .when(collapsed, |d| d.justify_center())
                .gap(px(if category { t::GAP } else { t::PAD }))
                .child(
                    icon(glyph, if category { t::SETTINGS_ICON } else { t::ICON })
                        .text_color(foreground),
                )
                .when(!collapsed, |d| d.child(label)),
        )
        .accessibility_label(label)
        .when(!category, |b| b.w_full().justify_start())
        .when(collapsed, |b| b.justify_center())
        .when(active, |b| b.bg(rgb(active_bg)).text_color(foreground));
    PanelButton {
        id,
        button,
        disabled: false,
        radius: if category { t::RADIUS } else { t::NAV_RADIUS },
    }
}

/// 工程验收入口显式启用；debug 构建默认也使用正式布局。
pub fn evidence_visible() -> bool {
    cfg!(debug_assertions) && std::env::var_os("VEYRA_UI_EVIDENCE").is_some()
}

/// 对应 React .surface.settings-block，页面只组合真实设置行。
pub fn setting_section(title: impl IntoElement, dark: bool, radius: f32) -> Div {
    use super::tokens as t;
    let p = super::theme::Palette::new(dark, true);
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .gap(px(super::tokens::ROW_GAP))
        .p(px(t::SECTION_PADDING))
        .rounded(px(radius))
        .bg(rgba((p.surface << 8) | 0xbf))
        .child(section_heading(title).mb(px(t::ROW_GAP)))
}
pub fn section_heading(title: impl IntoElement) -> Div {
    use super::tokens as t;
    div()
        .h(px(t::CONTROL))
        .flex_shrink_0()
        .pb(px(t::GAP))
        .text_size(px(t::SECTION_TITLE))
        .line_height(px(t::SECTION_LINE))
        .font_weight(FontWeight::SEMIBOLD)
        .child(title)
}
#[derive(IntoElement)]
pub struct CompactSetting {
    label: AnyElement,
    control: AnyElement,
}
impl RenderOnce for CompactSetting {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        use super::tokens as t;
        use gpui_kit::component::ActiveTheme;
        div()
            .flex()
            .items_center()
            .h(px(t::SETTING_HEIGHT))
            .flex_shrink_0()
            .gap(px(t::GAP))
            .border_b_1()
            .border_color(if cx.theme().mode.is_dark() {
                rgba(0x110d0dcc)
            } else {
                rgba(t::ROW_BORDER)
            })
            .child(
                div()
                    .flex_1()
                    .font_weight(FontWeight::MEDIUM)
                    .child(self.label),
            )
            .child(div().flex_shrink_0().child(self.control))
    }
}
pub fn compact_setting(label: impl IntoElement, control: impl IntoElement) -> CompactSetting {
    CompactSetting {
        label: label.into_any_element(),
        control: control.into_any_element(),
    }
}
/// 1280 logical viewport 的 CSS 双列，按 DOM 顺序逐行排列。
pub fn settings_pair(left: impl IntoElement, right: impl IntoElement) -> Div {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .gap(px(super::tokens::COLUMN_GAP))
        .child(div().flex_1().min_w_0().child(left))
        .child(div().flex_1().min_w_0().child(right))
}

/// React .icon-button：通用按钮的真实复用规格，调用方可指定 24/32/36px。
#[derive(IntoElement)]
pub struct IconButton {
    button: gpui_kit::base::Button,
    id: ElementId,
}
impl IconButton {
    pub fn tooltip(self, id: &'static str, text: &'static str) -> tooltip::TooltipButton {
        tooltip::with_tooltip(self.button, id, text, false)
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button = self.button.on_click(handler);
        self
    }
}
impl Styled for IconButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}
impl RenderOnce for IconButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focus = window
            .use_keyed_state((self.id, "icon-focus"), cx, ControlFocus::new)
            .read(cx)
            .handle
            .clone();
        let focused = focus.is_focused(window) && window.last_input_was_keyboard();
        self.button
            .track_focus(&focus)
            .on_mouse_down(MouseButton::Left, move |_, w, cx| focus.focus(w, cx))
            .when(focused, |b| {
                b.child(focus_outline(
                    super::tokens::browser_focus(cx.theme().mode.is_dark()),
                    super::tokens::RADIUS,
                    0.,
                ))
            })
    }
}
pub fn icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: &'static str,
    size: f32,
) -> IconButton {
    let id = id.into();
    IconButton {
        id: id.clone(),
        button: gpui_kit::base::Button::new(id)
            .hover(|s| s.bg(rgba(super::tokens::HOVER)))
            .accessibility_label(label)
            .size(px(size))
            .p_0()
            .rounded(px(super::tokens::RADIUS))
            .child(super::icons::icon(glyph, super::tokens::ICON_SMALL)),
    }
}

/// 弹层焦点与 Escape 继续由 Kit Root 管理；内容几何来自 panel-settings-dialog。
pub fn panel_dialog(
    dialog: gpui_kit::component::dialog::Dialog,
    window: &Window,
    cx: &App,
) -> gpui_kit::component::dialog::Dialog {
    use gpui_kit::component::ActiveTheme;
    dialog
        .bg(cx.theme().popover.opacity(0.75))
        .w(px(super::tokens::DIALOG_WIDTH))
        .h(px(super::tokens::DIALOG_HEIGHT))
        .margin_top((window.viewport_size().height - px(super::tokens::DIALOG_HEIGHT)) / 2.)
        .rounded(px(super::tokens::POPOVER_RADIUS))
        .border_0()
        .p_0()
        .gap_0()
        .close_button(false)
}

pub mod icon_picker;

/// Panel range 的 256×24 轨道；Kit base 保留拖动、键盘、AX value 行为。
pub fn panel_slider(
    state: &Entity<gpui_kit::component::slider::SliderState>,
    cx: &App,
) -> impl IntoElement {
    use gpui_kit::component::ActiveTheme;
    let percentage = state.read(cx).percentage().end;
    let text = cx.theme().foreground;
    use gpui_kit::base::{Slider, SliderIndicator, SliderThumb, SliderTrack};
    // Kit 的 root 仅提供 AX/release，指针输入必须保留 Track/Thumb 行为。
    Slider::new(state)
        .w(px(super::tokens::SLIDER_WIDTH))
        .h(px(24.))
        .child(
            SliderTrack::new(state)
                .relative()
                .w_full()
                .h(px(24.))
                .child(
                    SliderIndicator::new(state)
                        .absolute()
                        .left_0()
                        .top_0()
                        .w_full()
                        .h_full(),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(6.))
                        .w_full()
                        .h(px(12.))
                        .rounded(px(8.))
                        .bg(text.opacity(0.1)),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .w(relative(percentage))
                        .h(px(24.))
                        .rounded(px(super::tokens::SWITCH_RADIUS))
                        .bg(text),
                )
                .child(
                    SliderThumb::new(state)
                        .absolute()
                        .top_0()
                        .left(px(percentage * (super::tokens::SLIDER_WIDTH - 24.)))
                        .size(px(24.))
                        .border_4()
                        .border_color(text)
                        .rounded(px(12.))
                        .bg(cx.theme().popover),
                ),
        )
}

/// 未迁移路由共用空状态，不制造 Runtime 或观测数据。
pub fn unavailable(task: &'static str) -> Div {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .min_h(px(300.))
        .gap(px(super::tokens::GAP))
        .child(
            div()
                .text_size(px(super::tokens::BODY))
                .child("当前功能尚未接入"),
        )
        .child(div().text_size(px(10.)).child(format!("将在 {task} 实现")))
}

/// React 两处实际复用的说明图标，语义与文案由页面提供。
pub fn help_label(label: &'static str, id: &'static str, help: &'static str, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(super::tokens::ROW_GAP))
        .child(label)
        .child(
            gpui_kit::base::Button::new(id)
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .size(px(super::tokens::ICON_SMALL))
                .p_0()
                .accessibility_label(help)
                .child(
                    super::icons::icon("QuestionMarkCircle", super::tokens::ICON_SMALL)
                        .text_color(cx.theme().muted_foreground),
                )
                .map(|b| with_tooltip(b, id, help, false)),
        )
}

pub mod tooltip;
pub use tooltip::with_tooltip;
