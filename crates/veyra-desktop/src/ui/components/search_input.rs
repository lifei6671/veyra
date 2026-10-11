//! SearchInputGroup 的局部级联；Kit 保留输入、焦点和辅助功能语义。
use crate::ui::tokens::{self as t, proxies as pt};
use gpui_kit::{
    component::{
        ActiveTheme,
        input::{Input, InputState},
    },
    prelude::*,
    *,
};

#[derive(IntoElement)]
pub struct SearchInput {
    pub input: Input,
    pub state: Entity<InputState>,
}

impl RenderOnce for SearchInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let focused = self.state.read(cx).focus_handle(cx).is_focused(window);
        // CSS color-mix(in srgb, var(--accent) 72%, var(--line)) 含透明度，
        // 必须先预乘 alpha，不能直接对 RGB 做普通插值。
        let accent = rgb(t::ACCENT);
        let line = Rgba::from(cx.theme().border);
        let a = pt::SEARCH_FOCUS_ACCENT_RATIO;
        let alpha = a + line.a * (1. - a);
        let border = Rgba {
            r: (accent.r * a + line.r * line.a * (1. - a)) / alpha,
            g: (accent.g * a + line.g * line.a * (1. - a)) / alpha,
            b: (accent.b * a + line.b * line.a * (1. - a)) / alpha,
            a: alpha,
        };
        let input = self
            .input
            .font(Font {
                weight: pt::FONT_REGULAR,
                ..t::openbox_font()
            })
            .focus_bordered(false)
            .when(focused, |input| {
                input.border_color(border).shadow(vec![
                    BoxShadow::new(px(0.), px(0.), rgba(pt::SEARCH_FOCUS_SHADOW).into())
                        .spread_radius(px(pt::SEARCH_FOCUS_SPREAD)),
                ])
            });
        let frame = RenderOnce::render(input, window, cx).into_any_element();
        // Kit render 会投影 editor style；在同次渲染后覆盖 UA placeholder 色，
        // 仅作用于这个搜索输入，不改全局 muted 文本或其它页面。
        self.state.update(cx, |state, _| {
            state.set_editor_style(gpui_kit::base::input::InputEditorStyle {
                muted_foreground: rgb(pt::SEARCH_PLACEHOLDER).into(),
                ..Default::default()
            })
        });
        frame
    }
}
