//! PolicyLatency 的延迟出现时间线；记录来自正式 SQLite，最新记录在前。
use super::{history_time, latency_label};
use crate::ui::{
    components::tooltip::bubble,
    tokens::{self as t, proxies as pt},
};
use gpui_kit::{component::ActiveTheme as _, prelude::*, *};
use veyra_core::storage::latency::{LatencyOutcome, LatencyRecord};

struct HoverState {
    visible: bool,
    waiting: Option<Task<()>>,
}
#[derive(IntoElement)]
pub(super) struct HistoryTrigger {
    pub id: SharedString,
    pub trigger: AnyElement,
    pub records: Vec<(String, LatencyRecord)>,
}
impl RenderOnce for HistoryTrigger {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| HoverState {
            visible: false,
            waiting: None,
        });
        let visible = state.read(cx).visible;
        // React hover 退出仍保留浮层直到 160ms 淡出结束；不能立刻移除子树。
        let progress = gpui_kit::base::motion::transition(
            (self.id.clone(), "proxy-history-appear"),
            if visible { 1_f32 } else { 0_f32 },
            gpui_kit::base::motion::Transition::new(std::time::Duration::from_millis(160)).ease(
                gpui_kit::component::animation::cubic_bezier(0.25, 0.1, 0.25, 1.),
            ),
            window,
            cx,
        );
        div().relative().id(self.id).child(self.trigger)
            .on_hover(move |hovered, _, cx| {
                state.update(cx, |s, cx| {
                    s.waiting = None;
                    s.visible = false;
                    if *hovered {
                        s.waiting = Some(cx.spawn(async move |state, cx| {
                            cx.background_executor().timer(std::time::Duration::from_millis(pt::HISTORY_DELAY_MS)).await;
                            let _ = state.update(cx, |s, cx| {s.visible = true; cx.notify();});
                        }));
                    }
                    cx.notify();
                });
            })
            .when((visible || progress > 0.) && !self.records.is_empty(), |d| {
                // 颜色、字体遵守用户全局 Tooltip 覆盖，几何与时间线来自 React。
                // 原版每行独立 grid：两个 auto 轨道平分剩余宽，不能只让 meta flex_1。
                // 使用同源字体的实际 advance，避免 intrinsic 文本盒取整扩大 max-content。
                let text_width = |text: &str, weight: FontWeight| {
                    f32::from(window.text_system().shape_line(
                        text.to_owned().into(), px(t::TOOLTIP_FONT),
                        &[TextRun {len: text.len(), font: Font {weight, ..t::openbox_font()},
                            color: cx.theme().foreground, background_color: None, underline: None, strikethrough: None}], None,
                    ).width)
                };
                let rows = self.records.into_iter().take(10).map(|(name, record)| {
                    let success = matches!(record.outcome, LatencyOutcome::Success(_));
                    let time = history_time(record.tested_at_ms);
                    let text = format!("{}{}", latency_label(Some(&record.outcome), cx), if success {"ms"} else {""});
                    let meta = pt::HISTORY_META_WIDTH
                        .max(text_width(&name, pt::FONT_MEDIUM).min(pt::HISTORY_NAME_MAX))
                        .max(text_width(&time, pt::FONT_SEMIBOLD));
                    let result = pt::HISTORY_RESULT_WIDTH.max(text_width(&text, pt::FONT_SEMIBOLD));
                    (name, time, text, success, meta, result)
                }).collect::<Vec<_>>();
                let fixed = pt::HISTORY_TRACK_WIDTH + 2. * pt::HISTORY_GAP;
                let list_width = rows.iter().map(|(_, _, _, _, meta, result)| meta + fixed + result)
                    .fold(pt::HISTORY_MIN_WIDTH - 2. * pt::HISTORY_PX, f32::max)
                    .min(pt::HISTORY_MAX_WIDTH - 2. * pt::HISTORY_PX);
                let row_height = pt::HISTORY_ITEM_HEIGHT.max(t::TOOLTIP_LINE * 2. + 1.);
                let list_height = rows.len() as f32 * row_height;
                let popup = bubble(cx).relative().w(px(list_width + 2. * pt::HISTORY_PX)).min_w(px(pt::HISTORY_MIN_WIDTH)).max_w(px(pt::HISTORY_MAX_WIDTH))
                    .on_children_prepainted(|bounds, _, _| {
                        #[cfg(debug_assertions)]
                        crate::ui::components::log_layout("proxy-history-popup", &bounds);
                    })
                    .px(px(pt::HISTORY_PX)).py(px(pt::HISTORY_PY)).rounded(px(pt::HISTORY_RADIUS))
                    .shadow(vec![BoxShadow::new(px(0.), px(pt::HISTORY_SHADOW_Y), rgba(pt::HISTORY_SHADOW).into()).blur_radius(px(pt::HISTORY_SHADOW_BLUR))])
                    .child(div().absolute().top(px(-6.)).right(px(14.)).child(gpui_kit::component::Icon::default().data(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 6"><path d="M0 6 6 0 12 6Z" fill="currentColor"/></svg>"#.as_slice()).w(px(12.)).h(px(6.)).text_color(rgb(t::TOOLTIP_BG))))
                    // 列表总高按 CSS 两行行高计算，避免各行独立像素取整累积丢失。
                    .child(div().w(px(list_width)).flex().flex_col().h(px(list_height)).children(rows.into_iter().enumerate().map(|(index,(name,time,text,success,meta,result))| {
                        let spare = (list_width - meta - fixed - result).max(0.) / 2.;
                        // 全局 Tooltip 两行 19.6px 加 1px 时间间距，保留 CSS 自然行高。
                        div().flex().items_center().flex_1().min_h(px(pt::HISTORY_ITEM_HEIGHT)).gap(px(pt::HISTORY_GAP))
                            .on_children_prepainted(move |bounds, _, _| {
                                #[cfg(debug_assertions)]
                                crate::ui::components::log_layout(&format!("proxy-history-columns:{index}"), &bounds);
                            })
                            .child(div().w(px(meta + spare)).flex_shrink_0().flex().flex_col().items_end()
                                .on_children_prepainted(|bounds, _, _| {
                                    #[cfg(debug_assertions)]
                                    crate::ui::components::log_layout("proxy-history-meta-lines", &bounds);
                                })
                                .child(div().max_w(px(pt::HISTORY_NAME_MAX)).font_weight(pt::FONT_MEDIUM).opacity(0.72).truncate().child(name))
                                .child(// 时间为纯 Latin，使用 React 同源子集，避免官方静态字宽扩大浮层。
                                div().w_full().text_right().mt(px(1.)).font_family(t::OPENBOX_FONT).font_weight(pt::FONT_SEMIBOLD).whitespace_nowrap().child(time)))
                            .child(div().w(px(pt::HISTORY_TRACK_WIDTH)).h_full().relative().flex_shrink_0()
                                .child(div().absolute().left(px(7.5)).top_0().bottom_0().w(px(1.)).bg(rgba(pt::HISTORY_TRACK)))
                                .when(index==0,|d|d.child(gpui_kit::component::Icon::default().data(br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 8 6"><path d="M0 6 4 0 8 6Z" fill="currentColor"/></svg>"#.as_slice()).w(px(8.)).h(px(6.)).absolute().left(px(4.)).top(px(-1.)).text_color(rgba(pt::HISTORY_TRACK_ARROW))))
                                .child(div().absolute().left(px(3.)).top(relative(0.5)).mt(px(-5.)).size(px(pt::HISTORY_DOT)).rounded_full().bg(rgb(if success {pt::HISTORY_SUCCESS_DOT} else {pt::HISTORY_UNKNOWN_DOT}))))
                            .child(div().w(px(result + spare)).flex_shrink_0().when(success, |d| d.font_family(t::OPENBOX_FONT)).font_weight(pt::FONT_SEMIBOLD).whitespace_nowrap().child(text))
                    })));
                // 时间线整组淡入/淡出，名称自身 0.72 保留在内部；终态不建合成层。
                // motion::transition 已按 reduce_motion 直接返回目标值。
                let popup = popup.when(progress < 1., |popup| popup.group_opacity(progress))
                    .top(px(-3. * (1. - progress)));
                d.child(div().absolute().top_0().right_0().child(deferred(anchored().position_mode(AnchoredPositionMode::Local)
                    .position(point(px(2.),px(pt::LATENCY_HEIGHT+pt::HISTORY_OFFSET))).anchor(Anchor::TopRight).snap_to_window().child(popup)).with_priority(110)))
            })
    }
}
