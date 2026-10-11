//! 折叠保留真实内容，用裁切高度与透明度迁移 CSS grid 的 1fr/0fr 过渡。
use crate::ui::tokens::proxies as pt;
use gpui_kit::{
    base::motion::{Transition, transition},
    prelude::*,
    *,
};

pub(super) fn minimum(
    id: SharedString,
    closed: bool,
    open_height: f32,
    provider: bool,
    window: &mut Window,
    cx: &mut App,
) -> f32 {
    transition(
        (id, "minimum"),
        if closed {
            pt::COLLAPSED_MIN_HEIGHT
        } else {
            open_height
        },
        Transition::new(std::time::Duration::from_millis(if provider {
            280
        } else {
            200
        }))
        .ease(if provider {
            gpui_kit::component::animation::cubic_bezier(0.4, 0., 0.2, 1.)
        } else {
            gpui_kit::component::animation::cubic_bezier(0., 0., 0.58, 1.)
        }),
        window,
        cx,
    )
}
#[derive(IntoElement)]
pub(super) struct CollapseBody {
    pub id: SharedString,
    pub closed: bool,
    pub height: f32,
    pub provider: bool,
    pub dots: bool,
    pub content: AnyElement,
}
impl RenderOnce for CollapseBody {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let visible = if self.dots { self.closed } else { !self.closed };
        let duration = if self.dots {
            240
        } else if self.provider {
            280
        } else {
            200
        };
        let curve = if self.dots || self.provider {
            (0.4, 0., 0.2, 1.)
        } else {
            (0., 0., 0.58, 1.)
        };
        let policy = Transition::new(std::time::Duration::from_millis(duration)).ease(
            gpui_kit::component::animation::cubic_bezier(curve.0, curve.1, curve.2, curve.3),
        );
        let policy = if self.dots && visible {
            policy.delay(std::time::Duration::from_millis(40))
        } else {
            policy
        };
        let height = transition(
            (self.id.clone(), "height"),
            if visible { self.height } else { 0. },
            policy,
            window,
            cx,
        );
        let opacity = transition(
            (self.id.clone(), "opacity"),
            if visible { 1. } else { 0. },
            Transition::new(std::time::Duration::from_millis(
                if visible && self.provider {
                    180
                } else if visible {
                    160
                } else if self.provider {
                    140
                } else {
                    120
                },
            ))
            .delay(std::time::Duration::from_millis(if self.dots && visible {
                100
            } else {
                0
            }))
            .ease(gpui_kit::component::animation::cubic_bezier(
                0.25, 0.1, 0.25, 1.,
            )),
            window,
            cx,
        );
        div()
            .id(self.id)
            .w_full()
            .h(px(height))
            .flex_shrink_0()
            .overflow_hidden()
            .opacity(opacity)
            .when(height > 0., |d| {
                d.child(div().w_full().h(px(self.height)).child(self.content))
            })
    }
}
