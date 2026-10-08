//! Mature Kit controls with Veyra sizing/theme; overlay ownership remains window-level Kit Root.
use crate::ui::i18n::tr;
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
    border_focus: bool,
    disabled: bool,
}
impl PanelInput {
    /// subscription-source-form 用 1px accent border，而不是浏览器按钮 outline。
    pub fn border_focus(mut self) -> Self {
        self.border_focus = true;
        self
    }
    // 当前图标搜索框有 CSS 局部 focus 颜色/offset，输入语义仍由 Kit 管理。
    pub fn focus_style(mut self, color: Hsla, offset: f32) -> Self {
        self.focus_style = Some((color, offset));
        self
    }
    pub(super) fn step_value(value: &str, increment: bool) -> String {
        let value = value.parse::<u64>().unwrap_or(0);
        if increment {
            value.saturating_add(1)
        } else {
            value.saturating_sub(1)
        }
        .to_string()
    }
    pub fn numeric(mut self) -> Self {
        self.number = true;
        self
    }
}
impl Disableable for PanelInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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
            .when(self.border_focus, |d| d.rounded(px(t::RADIUS)))
            .child(Styled::h(
                Input::new(&self.state)
                    .disabled(self.disabled)
                    .focus_bordered(false)
                    .rounded(px(t::RADIUS))
                    .border_color(cx.theme().border)
                    .bg(cx.theme().input)
                    .px(px(t::PAD))
                    .py(px(t::ROW_GAP))
                    .text_size(px(t::BODY))
                    .line_height(px(t::BODY_LINE)),
                px(t::CONTROL),
            ).refine_style(&self.style)
             .when(focused && self.border_focus, |i| i.border_color(rgb(t::ACCENT))))
            .when(focused && !self.border_focus, |d| {
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
                        // 步进区盖住输入文字 hitbox，焦点后的 I-beam 不得截获按钮操作。
                        .occlude()
                        .cursor_default()
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
                                .accessibility_label(tr(cx, if increment { "增加数值" } else { "减少数值" }))
                                .w(px(t::SPINNER_WIDTH))
                                .h(px(t::SPINNER_HEIGHT / 2.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .p_0()
                                .cursor_default()
                                .on_mouse_down(MouseButton::Left, |_, window, cx| {
                                    window.prevent_default();
                                    cx.stop_propagation();
                                })
                                .text_color(rgb(if cx.theme().mode.is_dark() { t::SPINNER_DARK_TEXT } else { t::SPINNER_TEXT }))
                                .child(gpui_kit::component::Icon::default().data(arrow).with_size(px(t::SPINNER_ARROW)).h(px(t::SPINNER_ARROW / 2.)))
                                .on_click(move |_, window, cx| {
                                    state.update(cx, |input, cx| {
                                        let next = PanelInput::step_value(input.value().as_str(), increment);
                                        // replace_all 保留 undo，并明确发 Change；set_value 只用于静默投影。
                                        input.replace_all(next, window, cx);
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
        border_focus: false,
        disabled: false,
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
    tooltip: Option<SharedString>,
    id: &'static str,
    radius: f32,
    disabled: bool,
    reference_hover: Option<[Hsla; 3]>,
    reference_focus: Option<Hsla>,
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
    /// 原版 btn 的颜色过渡；仅明确指定参考颜色的调用方启用。
    pub fn reference_hover(mut self, normal: Hsla, hover: Hsla, foreground: Hsla) -> Self {
        self.reference_hover = Some([normal, hover, foreground]);
        self.reference_focus = Some(foreground);
        self
    }
    /// OpenBox .btn:focus-visible 的颜色；primary按钮使用primary而非其文字色。
    pub fn reference_focus(mut self, color: Hsla) -> Self {
        self.reference_focus = Some(color);
        self
    }
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
        self.tooltip = Some(text.into());
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
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // 与 Kit Button 在同一父元素 scope 读取它的稳定焦点 id；放入 child 后
        // scope 已进入 Button/content，会读到另一个 handle 且被 overflow 裁切。
        if let Some([normal, hover, foreground]) = self.reference_hover {
            let state = window.use_keyed_state(
                (ElementId::from(self.id), "reference-hover"),
                cx,
                |_, _| false,
            );
            let hovered = !self.disabled && *state.read(cx);
            let bg = gpui_kit::base::motion::transition(
                (self.id, "reference-bg"),
                if hovered { hover } else { normal },
                reference_button_transition(),
                window,
                cx,
            );
            self.button = self
                .button
                .custom(
                    ButtonCustomVariant::new(cx)
                        .color(bg)
                        .foreground(foreground)
                        .hover(bg)
                        .active(bg)
                        .shadow(false),
                )
                .bg(bg)
                .text_color(foreground)
                .when(!self.disabled, |b| {
                    b.on_hover(move |hovered, _, cx| {
                        state.update(cx, |state, cx| {
                            *state = *hovered;
                            cx.notify();
                        })
                    })
                });
        }
        let tooltip = self.tooltip;
        let button = RenderOnce::render(self.button, window, cx).into_any_element();
        let focus = window
            .use_keyed_state(self.id, cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        let focused =
            !self.disabled && focus.is_focused(window) && window.last_input_was_keyboard();
        div()
            .id(self.id)
            .when_some(tooltip, |d, text| {
                d.tooltip(move |_, cx| cx.new(|_| tooltip::TooltipContent(text.clone())).into())
            })
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
                d.child(
                    focus_outline(
                        super::tokens::browser_focus(cx.theme().mode.is_dark()),
                        self.radius,
                        if self.reference_focus.is_some() {
                            super::tokens::REFERENCE_BUTTON_FOCUS_OFFSET
                        } else {
                            0.
                        },
                    )
                    .when_some(self.reference_focus, |outline, color| {
                        outline.border_color(color)
                    }),
                )
            })
    }
}
pub fn button(id: &'static str, label: impl Into<SharedString>) -> PanelButton {
    use gpui_kit::component::FocusableExt as _;
    let label = label.into();
    PanelButton {
        id,
        tooltip: None,
        radius: super::tokens::RADIUS,
        disabled: false,
        reference_hover: None,
        reference_focus: None,
        button: Button::new(id)
            .focus_ring(false)
            // 空 label 不应成为 flex child；否则文字/图标内容会多出 Kit 的 gap。
            .when(!label.is_empty(), |b| b.label(label))
            .border_0()
            .h(px(super::tokens::CONTROL))
            .rounded(px(super::tokens::RADIUS))
            .text_size(px(super::tokens::BODY)),
    }
}
fn reference_button_transition() -> gpui_kit::base::motion::Transition {
    gpui_kit::base::motion::Transition::new(std::time::Duration::from_millis(
        super::tokens::REFERENCE_BUTTON_TRANSITION_MS,
    ))
    .ease(gpui_kit::component::animation::cubic_bezier(
        0., 0., 0.2, 1.,
    ))
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
        .font_weight(super::theme::MISANS_REGULAR)
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
        tooltip: None,
        button,
        disabled: false,
        reference_hover: None,
        reference_focus: None,
        radius: if category { t::RADIUS } else { t::NAV_RADIUS },
    }
}

/// 工程验收入口显式启用；debug 构建默认也使用正式布局。
pub fn evidence_visible() -> bool {
    cfg!(debug_assertions) && std::env::var_os("VEYRA_UI_EVIDENCE").is_some()
}

/// React .surface，Settings 与订阅卡共用相同主题/圆角，局部 padding 由组合层提供。
pub fn surface(dark: bool, radius: Pixels) -> Div {
    let p = super::theme::Palette::new(dark, true);
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .rounded(radius)
        .bg(rgba((p.surface << 8) | 0xbf))
}

/// 对应 React .surface.settings-block，页面只组合真实设置行。
pub fn setting_section(title: impl IntoElement, dark: bool, radius: f32) -> Div {
    use super::tokens as t;
    surface(dark, px(radius))
        .gap(px(super::tokens::ROW_GAP))
        .p(px(t::SECTION_PADDING))
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
        .font_weight(super::theme::MISANS_SEMIBOLD)
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
            .w_full()
            .min_w_0()
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
                    .min_w_0()
                    .font_weight(super::theme::MISANS_MEDIUM)
                    .child(self.label),
            )
            .child(div().min_w_0().flex_shrink_0().child(self.control))
    }
}
pub fn compact_setting(label: impl IntoElement, control: impl IntoElement) -> CompactSetting {
    CompactSetting {
        label: label.into_any_element(),
        control: control.into_any_element(),
    }
}
/// 复用 React .panel-settings-grid：按窗口逻辑宽度与侧栏状态决定列数、列间距。
/// 子项保持 DOM 顺序，单列时不插入为双列占位的空行。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsLayout {
    columns: u16,
    column_gap: f32,
}
impl SettingsLayout {
    pub fn new(width: f32, sidebar_collapsed: bool) -> Self {
        use super::tokens as t;
        Self {
            columns: if width >= t::PANEL_TWO_COLUMNS
                || (sidebar_collapsed && width >= t::PANEL_COLLAPSED_TWO_COLUMNS)
            {
                2
            } else {
                1
            },
            column_gap: if width >= t::PANEL_EXTRA_WIDE {
                t::PANEL_EXTRA_COLUMN_GAP
            } else if width >= t::PANEL_WIDE {
                t::COLUMN_GAP
            } else if width >= t::PANEL_TWO_COLUMNS {
                t::PANEL_MEDIUM_COLUMN_GAP
            } else {
                t::PANEL_COLUMN_GAP
            },
        }
    }
    pub fn grid(self) -> Div {
        div()
            .w_full()
            .max_w(px(super::tokens::SETTINGS_GRID_MAX_WIDTH))
            .min_w_0()
            .grid()
            .grid_cols(self.columns)
            .gap_x(px(self.column_gap))
            .gap_y(px(super::tokens::ROW_GAP))
    }
}

#[cfg(test)]
mod settings_layout_tests {
    use super::SettingsLayout;

    // 超宽窗口也必须沿用 React 1280px 上限，不让两列无限扩张。
    #[test]
    fn grid_has_react_max_width() {
        use gpui_kit::{Styled, px};
        assert_eq!(super::super::tokens::SETTINGS_GRID_MAX_WIDTH, 1280.);
        for width in [1600., 1920., 2560.] {
            let mut grid = SettingsLayout::new(width, false).grid();
            assert_eq!(grid.style().max_size.width, Some(px(1280.).into()));
        }
    }
    // 保护实际窗口缩放：展开/折叠各自在 React 断点切列，不让固定宽度控件挤进窄列。
    #[test]
    fn columns_follow_viewport_and_sidebar_breakpoints() {
        for (width, collapsed, columns) in [
            (767., true, 1),
            (768., true, 2),
            (900., false, 1),
            (1023., false, 1),
            (1024., false, 2),
            (1280., false, 2),
        ] {
            assert_eq!(SettingsLayout::new(width, collapsed).columns, columns);
        }
    }
    #[test]
    fn column_gaps_follow_react_media_queries() {
        for (width, gap) in [(900., 16.), (1024., 32.), (1280., 48.), (1536., 64.)] {
            assert_eq!(SettingsLayout::new(width, false).column_gap, gap);
        }
    }
}

/// React .icon-button：通用按钮的真实复用规格，调用方可指定 24/32/36px。
#[derive(IntoElement)]
pub struct IconButton {
    button: gpui_kit::base::Button,
    id: ElementId,
    tooltip_text: Option<SharedString>,
    disabled: bool,
    primary: bool,
    reference_ghost: Option<(Hsla, Hsla)>,
}
impl IconButton {
    /// 复用 OpenBox btn-primary btn-square；禁用只阻断操作，不另加透明度改变参考颜色。
    pub fn primary(mut self) -> Self {
        self.primary = true;
        self
    }
    /// 本机OpenBox btn-ghost：hover底色与focus边框，不改变disabled业务行为。
    pub fn openbox_ghost(mut self, hover: Hsla, focus: Hsla) -> Self {
        self.reference_ghost = Some((hover, focus));
        self
    }
    pub fn with_tooltip(mut self, text: impl Into<SharedString>) -> Self {
        self.tooltip_text = Some(text.into());
        self
    }
    pub fn tooltip(self, id: &'static str, text: &'static str) -> tooltip::TooltipButton {
        tooltip::with_tooltip(
            self.button.hover(|s| s.bg(rgba(super::tokens::HOVER))),
            id,
            text,
            false,
        )
    }
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button = self.button.on_click(handler);
        self
    }
}
impl Disableable for IconButton {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.button = self.button.disabled(disabled);
        self
    }
}
impl Styled for IconButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.button.style()
    }
}
impl RenderOnce for IconButton {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let hover_state = window.use_keyed_state((self.id.clone(), "icon-hover"), cx, |_, _| false);
        let hovered = !self.disabled && *hover_state.read(cx);
        let normal_bg = if self.primary {
            rgb(super::tokens::PRIMARY_BUTTON_BG).into()
        } else {
            self.button
                .style()
                .background
                .as_ref()
                .and_then(|fill| fill.color())
                .and_then(|bg| bg.as_solid())
                .unwrap_or_else(|| rgba(0).into())
        };
        let normal_fg = if self.primary {
            rgb(super::tokens::PRIMARY_BUTTON_TEXT).into()
        } else {
            self.button
                .style()
                .text
                .color
                .unwrap_or(cx.theme().foreground)
        };
        let hover_bg: Hsla = if self.primary {
            rgb(super::tokens::PRIMARY_BUTTON_HOVER).into()
        } else {
            self.reference_ghost
                .map_or_else(|| rgba(super::tokens::HOVER).into(), |(hover, _)| hover)
        };
        let hover_fg = if self.primary || self.reference_ghost.is_some() {
            normal_fg
        } else {
            cx.theme().foreground
        };
        let bg = gpui_kit::base::motion::transition(
            (self.id.clone(), "icon-bg"),
            if hovered { hover_bg } else { normal_bg },
            reference_button_transition(),
            window,
            cx,
        );
        let fg = gpui_kit::base::motion::transition(
            (self.id.clone(), "icon-fg"),
            if hovered { hover_fg } else { normal_fg },
            reference_button_transition(),
            window,
            cx,
        );
        let focus = window
            .use_keyed_state((self.id, "icon-focus"), cx, ControlFocus::new)
            .read(cx)
            .handle
            .clone();
        let focused =
            !self.disabled && focus.is_focused(window) && window.last_input_was_keyboard();
        let radius = if self.primary {
            super::tokens::PRIMARY_BUTTON_RADIUS
        } else {
            super::tokens::RADIUS
        };
        self.button
            .bg(bg)
            .text_color(fg)
            .when(self.primary, |b| b.rounded(px(radius)))
            .when(!self.disabled, |b| {
                b.on_hover(move |hovered, _, cx| {
                    hover_state.update(cx, |state, cx| {
                        *state = *hovered;
                        cx.notify();
                    })
                })
            })
            .when_some(self.tooltip_text, |b, text| {
                b.tooltip(move |_, cx| cx.new(|_| tooltip::TooltipContent(text.clone())).into())
            })
            .track_focus(&focus)
            .when(!self.disabled, |b| {
                b.on_mouse_down(MouseButton::Left, move |_, w, cx| focus.focus(w, cx))
            })
            .when(focused, |b| {
                b.child(
                    focus_outline(
                        super::tokens::browser_focus(cx.theme().mode.is_dark()),
                        radius,
                        if self.primary || self.reference_ghost.is_some() {
                            super::tokens::REFERENCE_BUTTON_FOCUS_OFFSET
                        } else {
                            0.
                        },
                    )
                    .when(self.primary, |outline| {
                        outline.border_color(rgb(super::tokens::PRIMARY_BUTTON_BG))
                    })
                    .when_some(self.reference_ghost, |outline, (_, color)| {
                        outline.border_color(color)
                    }),
                )
            })
    }
}
pub fn icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    glyph: &'static str,
    size: f32,
) -> IconButton {
    icon_button_content(
        id,
        label,
        size,
        super::icons::icon(glyph, super::tokens::ICON_SMALL),
    )
}
/// 同一 IconButton 外壳复用真实图标/Refresh spinner，不另建页面按钮样式。
pub fn icon_button_content(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    size: f32,
    content: impl IntoElement,
) -> IconButton {
    let id = id.into();
    IconButton {
        id: id.clone(),
        tooltip_text: None,
        disabled: false,
        primary: false,
        reference_ghost: None,
        button: gpui_kit::base::Button::new(id)
            .accessibility_label(label)
            .size(px(size))
            .p_0()
            .rounded(px(super::tokens::RADIUS))
            .child(content),
    }
}

/// ProxyOption 与 NodeHealthDots 复用参考 hover 时序；状态仅属于绘制，不进入业务模型。
#[derive(IntoElement)]
pub struct HoverSurface {
    element: Stateful<Div>,
    id: ElementId,
    colors: Option<[Hsla; 4]>,
    scale: Option<f32>,
}
impl HoverSurface {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            element: div().id(id.clone()),
            id,
            colors: None,
            scale: None,
        }
    }
    pub fn colors(mut self, normal: Hsla, hover: Hsla, border: Hsla, hover_border: Hsla) -> Self {
        self.colors = Some([normal, hover, border, hover_border]);
        self
    }
    pub fn scale(mut self, size: f32) -> Self {
        self.scale = Some(size);
        self
    }
    pub fn with_tooltip(mut self, text: impl Into<SharedString>) -> Self {
        let text = text.into();
        self.element = self
            .element
            .tooltip(move |_, cx| cx.new(|_| tooltip::TooltipContent(text.clone())).into());
        self
    }
}
impl Styled for HoverSurface {
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}
impl ParentElement for HoverSurface {
    fn extend(&mut self, children: impl IntoIterator<Item = AnyElement>) {
        self.element.extend(children);
    }
}
impl RenderOnce for HoverSurface {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state((self.id.clone(), "surface-hover"), cx, |_, _| false);
        let hovered = *state.read(cx);
        let policy = || {
            gpui_kit::base::motion::Transition::new(std::time::Duration::from_millis(
                super::tokens::SUBSCRIPTION_HOVER_TRANSITION_MS,
            ))
            .ease(gpui_kit::component::animation::cubic_bezier(
                0.4, 0., 0.2, 1.,
            ))
        };
        if let Some([normal, hover, border, hover_border]) = self.colors {
            let bg = gpui_kit::base::motion::transition(
                (self.id.clone(), "surface-bg"),
                if hovered { hover } else { normal },
                policy(),
                window,
                cx,
            );
            let line = gpui_kit::base::motion::transition(
                (self.id.clone(), "surface-border"),
                if hovered { hover_border } else { border },
                policy(),
                window,
                cx,
            );
            self.element = self.element.bg(bg).border_color(line);
        }
        if let Some(size) = self.scale {
            let scale = gpui_kit::base::motion::transition(
                (self.id, "surface-scale"),
                if hovered { 1.1 } else { 1. },
                policy(),
                window,
                cx,
            );
            // 保持 dot 的占位尺寸；放大只改变自身，不挤动相邻点。
            self.element = self
                .element
                .size(px(size * scale))
                .m(px(size * (1. - scale) / 2.));
        }
        self.element.on_hover(move |hovered, _, cx| {
            state.update(cx, |state, cx| {
                *state = *hovered;
                cx.notify();
            })
        })
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
pub fn unavailable(task: &'static str, cx: &App) -> Div {
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
                .child(tr(cx, "当前功能尚未接入")),
        )
        .child(
            div()
                .text_size(px(10.))
                .child(format!("{} {task}", tr(cx, "将在"))),
        )
}

/// React 两处实际复用的说明图标，语义与文案由页面提供。
pub fn help_label(label: &'static str, id: &'static str, help: &'static str, cx: &App) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(super::tokens::ROW_GAP))
        .child(tr(cx, label))
        .child(
            gpui_kit::base::Button::new(id)
                .flex()
                .items_center()
                .justify_center()
                .rounded_full()
                .size(px(super::tokens::ICON_SMALL))
                .p_0()
                .accessibility_label(tr(cx, help))
                .child(
                    super::icons::icon("QuestionMarkCircle", super::tokens::ICON_SMALL)
                        .text_color(cx.theme().muted_foreground),
                )
                .map(|b| with_tooltip(b, id, help, false)),
        )
}

pub mod tooltip;
pub use tooltip::with_tooltip;
