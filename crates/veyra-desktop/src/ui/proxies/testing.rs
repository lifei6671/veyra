//! 测速 Bolt 的 CSS filter/transform：独立绘制层避免 Kit Button 内容裁切光晕。
use crate::ui::tokens::proxies as pt;
use gpui_kit::{prelude::*, *};
use std::sync::Arc;

pub(super) fn control(
    id: SharedString,
    control: AnyElement,
    active: bool,
    bounds: (f32, f32, f32),
    color: Hsla,
    cx: &App,
) -> AnyElement {
    if !active {
        return control;
    }
    let (width, height, size) = bounds;
    let graphic = div().size(px(size)).relative();
    let graphic = if cx.reduce_motion() {
        graphic
            .child(super::super::icons::icon("Bolt", size))
            .into_any_element()
    } else {
        graphic
            .with_animation(
                "testing-bolt-filter",
                Animation::new(std::time::Duration::from_millis(pt::TESTING_PERIOD_MS)).repeat(),
                move |graphic, progress| {
                    let phase = 1. - (progress * 2. - 1.).abs();
                    let pulse =
                        gpui_kit::component::animation::cubic_bezier(0.42, 0., 0.58, 1.)(phase);
                    graphic.child(BoltFrame { size, pulse })
                },
            )
            .into_any_element()
    };
    // 真实 Button 仍拥有点击与 disabled；绘制叠层没有事件/hitbox。
    div()
        .id(id)
        .w(px(width))
        .h(px(height))
        .relative()
        .text_color(color)
        .opacity(pt::CONTROL_DISABLED_OPACITY)
        .child(control)
        .child(
            div()
                .absolute()
                .left(px((width - size) / 2.))
                .top(px((height - size) / 2.))
                .child(graphic),
        )
        .into_any_element()
}

#[derive(IntoElement)]
struct BoltFrame {
    size: f32,
    pulse: f32,
}
impl RenderOnce for BoltFrame {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = window.text_style().color.to_rgb();
        let size = self.size;
        let scale = 0.88 + 0.28 * self.pulse;
        let translate = size * (1. - scale) / 2.;
        let y = 1. - 2. * self.pulse;
        // filter 在 transform 之前作用；外扩3σ画布保留同色光晕。
        let padding = 12.;
        let extent = size + padding * 2.;
        let original = include_str!("../../../assets/heroicons/Bolt.svg")
            .replace("<svg ", "<svg width=\"24\" height=\"24\" ");
        // usvg 的滤镜 flood-color 不继承 currentColor；实测会变成黑色阴影。
        // 显式展开相同文本色，保留 React drop-shadow(currentColor) 的真实颜色。
        let color_css = format!(
            "rgb({},{},{})",
            (color.r * 255.).round(),
            (color.g * 255.).round(),
            (color.b * 255.).round()
        );
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{extent}" height="{extent}" viewBox="-{padding} -{padding} {extent} {extent}" color="{color_css}"><defs><filter id="glow" filterUnits="userSpaceOnUse" x="-{padding}" y="-{padding}" width="{extent}" height="{extent}"><feDropShadow dx="0" dy="0" stdDeviation="{}" flood-color="{color_css}" flood-opacity="{}"/></filter></defs><g transform="translate({translate} {}) scale({scale})"><g filter="url(#glow)"><g transform="scale({})">{original}</g></g></g></svg>"#,
            3. * self.pulse,
            self.pulse,
            translate + y,
            size / 24.
        );
        let frame = cx
            .svg_renderer()
            .render_single_frame(svg.as_bytes(), window.scale_factor())
            .expect("bundled Bolt with CSS filter");
        let previous = window.use_keyed_state("testing-bolt-last-frame", cx, |_, cx| {
            // 元素退场时也退掉最后一帧；非活动/换页/减少动态不会残留 sprite。
            cx.on_release(|frame: &mut Option<Arc<RenderImage>>, cx| {
                if let Some(frame) = frame.take() {
                    cx.drop_image(frame, None);
                }
            })
            .detach();
            None::<Arc<RenderImage>>
        });
        let old = previous.update(cx, |previous, _| previous.replace(frame.clone()));
        // 每个控件只保留当前帧，移除旧 sprite，避免连续浮点滤镜生成无界 atlas 缓存。
        if let Some(old) = old {
            window.drop_image(old).expect("retire testing Bolt frame");
        }
        img(frame)
            .absolute()
            .left(px(-padding))
            .top(px(-padding))
            .size(px(extent))
            .opacity((0.58 + 0.42 * self.pulse) * color.a)
    }
}
