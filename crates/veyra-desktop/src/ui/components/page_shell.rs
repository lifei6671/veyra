//! 所有主页面共用固定顶部与内容视口；滚动只能放在内容子区域。
use crate::ui::{theme::Palette, tokens as t};
use gpui_kit::{component::ActiveTheme, prelude::*, *};

pub fn viewport() -> Div {
    div()
        .size_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .overflow_hidden()
}

/// 所有路由统一为设置页48px顶部；页面控件在固定表面内居中。
pub fn toolbar(cx: &App) -> Div {
    div()
        .w_full()
        .h(px(t::PAGE_TOOLBAR_HEIGHT))
        .flex_shrink_0()
        .flex()
        .items_center()
        .gap(px(t::GAP))
        .px(px(t::GAP))
        .py(px(t::PAGE_TOOLBAR_PY))
        .bg(rgb(Palette::new(cx.theme().mode.is_dark(), true).surface)
            .opacity(t::PANEL_TOOLBAR_ALPHA))
}
