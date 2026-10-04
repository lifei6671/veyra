use crate::{app::AppView, navigation::Route, state_bridge::LoadState};
use gpui_kit::{
    component::{button::*, *},
    prelude::*,
    *,
};
impl Render for AppView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (background, panel, foreground, muted) = if self.dark {
            (0x16231f, 0x22322c, 0xe9f3ed, 0xa8bdb1)
        } else {
            (0xe8f1ec, 0xfafdfb, 0x253b30, 0x586d61)
        };
        let status = match &self.bridge.load {
            LoadState::Idle => "等待本地状态".to_string(),
            LoadState::Busy => "Loading / Busy · 正在读取本地状态，仍可切换页面".to_string(),
            LoadState::Ready => "本地状态已加载".to_string(),
            LoadState::Error(error) => format!("读取失败 · {:?} · {}", error.code(), error),
        };
        let mut tools = div()
            .flex()
            .gap_2()
            .child(
                Button::new("refresh")
                    .label(if matches!(self.bridge.load, LoadState::Error(_)) {
                        "重试 / Retry"
                    } else {
                        "刷新本地状态"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
            .child(
                Button::new("theme")
                    .label(if self.dark {
                        "切换 Light"
                    } else {
                        "切换 Dark"
                    })
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.dark = !this.dark;
                        Theme::change(
                            if this.dark {
                                ThemeMode::Dark
                            } else {
                                ThemeMode::Light
                            },
                            Some(window),
                            cx,
                        );
                        cx.notify();
                    })),
            );
        if cfg!(debug_assertions) {
            #[cfg(debug_assertions)]
            {
                tools = tools
                    .child(Button::new("retention").label("Entity 自检").on_click(
                        cx.listener(|this, _, window, cx| this.evidence_retention(window, cx)),
                    ))
                    .child(
                        Button::new("race")
                            .label("A500 / B50")
                            .on_click(cx.listener(|this, _, _, cx| this.evidence_race(cx))),
                    )
                    .child(
                        Button::new("busy")
                            .label("Busy 3s")
                            .on_click(cx.listener(|this, _, _, cx| this.evidence_busy(cx))),
                    )
                    .child(
                        Button::new("instances")
                            .label("实例 evidence")
                            .on_click(cx.listener(|this, _, _, cx| this.evidence_instances(cx))),
                    );
            }
        }
        let mut snapshot = div()
            .flex()
            .flex_col()
            .gap_1()
            .text_sm()
            .text_color(rgb(muted));
        if let Some(state) = &self.bridge.snapshot {
            snapshot = snapshot
                .child(format!("StateEpoch {:?}", state.state_epoch))
                .child(format!(
                    "config revision {}   ·   selection revision {}   ·   accepted request {:?}",
                    state.config_version().0.revision,
                    state.selection_version().0.revision,
                    self.bridge.accepted_request
                ));
            if state.subscriptions.is_empty()
                && state.nodes.is_empty()
                && state.config_revision == 0
            {
                snapshot = snapshot.child("本地配置尚未创建 / 空状态（Core 已初始化空快照）");
            }
        }
        div()
            .flex()
            .size_full()
            .font_family(".SystemUIFont")
            .text_color(rgb(foreground))
            .bg(rgb(background))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w(px(256.))
                    .flex_shrink_0()
                    .h_full()
                    .p_4()
                    .gap_2()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .py_4()
                            .child("◈  VEYRA"),
                    )
                    .children(Route::ALL.map(|route| {
                        div()
                            .id(route.id())
                            .w_full()
                            .px_4()
                            .py_3()
                            .rounded_lg()
                            .cursor_pointer()
                            .when(self.route == route, |d| {
                                d.bg(rgb(0x71d6a2)).text_color(rgb(0x153d29))
                            })
                            .child(format!("{}   {}", route.label(), route.id()))
                            .on_click(cx.listener(move |this, _, _, cx| this.navigate(route, cx)))
                    }))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(muted))
                            .child("P1-03 · 本地桌面壳"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(muted))
                            .child("Runtime 尚未接入"),
                    )
                    .child(
                        Button::new("quit")
                            .label("退出")
                            .on_click(|_, _, cx| cx.quit()),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .p_4()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_lg().child(self.route.label()))
                            .child(tools),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(muted))
                            .child("P1-03 临时本地状态 · 正式目录由 P1-05 接管 · 主题仅此会话"),
                    )
                    .child(
                        div()
                            .rounded_2xl()
                            .bg(rgb(panel))
                            .p_5()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(status)
                            .child(snapshot),
                    )
                    .child(
                        div()
                            .id("page-content")
                            .rounded_2xl()
                            .bg(rgb(panel))
                            .p_6()
                            .flex_1()
                            .overflow_y_scroll()
                            .child(self.pages.get(self.route)),
                    )
                    .child(div().text_xs().text_color(rgb(muted)).child(
                        "Debug / evidence · 延迟仅调试构建；实例事件为 synthetic bridge evidence",
                    ))
                    .child(
                        div().h(px(100.)).text_xs().text_color(rgb(muted)).children(
                            self.bridge
                                .history
                                .iter()
                                .cloned()
                                .map(|line| div().child(line)),
                        ),
                    ),
            )
    }
}
