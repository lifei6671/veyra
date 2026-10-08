//! P0 capability probe. Synthetic data only; one GPUI/AppKit main loop.
#[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
mod p0_05_gui_owner;

use gpui_kit::{
    assets::Assets,
    component::{
        button::*,
        input::{Input, InputState, Textarea, TextareaState},
        *,
    },
    *,
};
use std::{sync::mpsc, time::Duration};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

struct Prototype {
    #[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
    gui_owner_status: Option<&'static str>,
    input: Entity<InputState>,
    text: Entity<TextareaState>,
    modal_text: Entity<TextareaState>,
    scroll: UniformListScrollHandle,
    selected: Option<usize>,
    dark: bool,
    blur: bool,
    running: bool,
    status_item: MenuItem,
    _tray: TrayIcon,
}
impl Prototype {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        tray: TrayIcon,
        status_item: MenuItem,
    ) -> Self {
        window.on_window_should_close(cx, |_, cx| {
            cx.hide();
            false
        });
        Self {
            #[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
            gui_owner_status: None,
            input: cx.new(|cx| InputState::new(window, cx).placeholder("在此使用中文输入法")),
            text: cx
                .new(|cx| TextareaState::new(window, cx).placeholder("多行中文 · 换行 · 跨行选择")),
            modal_text: cx.new(|cx| TextareaState::new(window, cx).placeholder("弹层内输入中文")),
            scroll: UniformListScrollHandle::new(),
            selected: None,
            dark: false,
            blur: true,
            running: false,
            status_item,
            _tray: tray,
        }
    }
    fn toggle_state(&mut self, cx: &mut Context<Self>) {
        self.running = !self.running;
        self.status_item.set_text(if self.running {
            "状态：运行中（模拟）"
        } else {
            "状态：已停止（模拟）"
        });
        cx.notify();
    }
    fn modal(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        let text = self.modal_text.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title("长文本 / 焦点验证")
                .w(px(650.))
                .child(Textarea::new(&text).h(px(240.)))
        });
    }
}
impl Render for Prototype {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let card = if self.dark {
            rgba(0x202d39dd)
        } else {
            rgba(0xffffffdd)
        };
        let fg = if self.dark {
            rgb(0xe9f0f3)
        } else {
            rgb(0x263c48)
        };
        let sidebar = if self.dark {
            rgb(0x17232d)
        } else {
            rgb(0xf4faf7)
        };
        let selected = self.selected;
        let entity = cx.entity().downgrade();
        div()
            .flex()
            .size_full()
            .text_color(fg)
            .font_family(".SystemUIFont")
            .child(
                div()
                    .v_flex()
                    .w(px(220.))
                    .h_full()
                    .p_6()
                    .gap_4()
                    .bg(sidebar)
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("Veyra / P0"),
                    )
                    .child("GPUI 能力原型")
                    .child("输入 · 焦点 · 列表")
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
                    )
                    .child(
                        Button::new("blur")
                            .label("切换 Window Blur")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.blur = !this.blur;
                                window.set_background_appearance(if this.blur {
                                    WindowBackgroundAppearance::Blurred
                                } else {
                                    WindowBackgroundAppearance::Transparent
                                });
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("state")
                            .label("更新托盘状态")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_state(cx))),
                    )
                    .child(
                        Button::new("hide")
                            .label("隐藏窗口")
                            .on_click(|_, _, cx| cx.hide()),
                    )
                    .child(Button::new("disabled").label("禁用状态").disabled(true))
                    .child(format!("Window blur: {}", self.blur))
                    .child("仅测试数据 / 无网络业务")
                    .children({
                        #[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
                        {
                            self.gui_owner_status
                        }
                        #[cfg(not(all(target_os = "macos", feature = "p0-05-gui-owner")))]
                        {
                            None::<&str>
                        }
                    }),
            )
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .h_full()
                    .p_5()
                    .gap_3()
                    .bg(linear_gradient(
                        120.,
                        linear_color_stop(
                            if self.dark {
                                rgba(0x203749aa)
                            } else {
                                rgba(0xf0c6ada0)
                            },
                            0.,
                        ),
                        linear_color_stop(
                            if self.dark {
                                rgba(0x193c31aa)
                            } else {
                                rgba(0x80cbbfa0)
                            },
                            1.,
                        ),
                    ))
                    .child(
                        div()
                            .v_flex()
                            .p_4()
                            .gap_3()
                            .rounded_2xl()
                            .bg(card)
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("输入与原生文本编辑"),
                            )
                            .child(Input::new(&self.input))
                            .child(Textarea::new(&self.text).h(px(95.)))
                            .child(
                                Button::new("modal")
                                    .primary()
                                    .label("打开长文本弹窗")
                                    .on_click(cx.listener(Self::modal)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_3()
                            .h(px(82.))
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .p_3()
                                    .rounded_2xl()
                                    .bg(card)
                                    .child("中文 English 012345 · 中英混排")
                                    .child(div().text_xl().child("😀 🌿 🚀 🇨🇳 🇺🇸 · Bold 字重")),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .w(px(290.))
                                    .p_3()
                                    .rounded_2xl()
                                    .bg(rgba(0x66cc9980))
                                    .child("半透明 Card / 背景透出")
                                    .child("窗口级 Blur ≠ 卡片 backdrop"),
                            ),
                    )
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_h_0()
                            .p_3()
                            .gap_2()
                            .rounded_2xl()
                            .bg(card)
                            .child(
                                div()
                                    .flex()
                                    .gap_2()
                                    .items_center()
                                    .child(format!(
                                        "10,000 行 / 选中 {:?}",
                                        self.selected.map(|i| i + 1)
                                    ))
                                    .child(Button::new("middle").label("中段").on_click(
                                        cx.listener(|this, _, _, cx| {
                                            this.scroll.scroll_to_item(5000, ScrollStrategy::Top);
                                            cx.notify();
                                        }),
                                    ))
                                    .child(Button::new("end").label("末尾").on_click(cx.listener(
                                        |this, _, _, cx| {
                                            this.scroll.scroll_to_item(9999, ScrollStrategy::Top);
                                            cx.notify();
                                        },
                                    ))),
                            )
                            .child(
                                uniform_list("synthetic-list", 10_000, move |range, _, _| {
                                    range
                                        .map(|ix| {
                                            let entity = entity.clone();
                                            div()
                                                .id(ix)
                                                .h(px(32.))
                                                .px_3()
                                                .flex()
                                                .items_center()
                                                .rounded_md()
                                                .bg(if selected == Some(ix) {
                                                    rgba(0x70c996aa)
                                                } else {
                                                    rgba(0x00000000)
                                                })
                                                .child(format!(
                                                    "fixture-row-{:05}     合成节点 · Example {}",
                                                    ix + 1,
                                                    ix + 1
                                                ))
                                                .on_click(move |_, _, cx| {
                                                    let _ = entity.update(cx, |this, cx| {
                                                        this.selected = Some(ix);
                                                        cx.notify();
                                                    });
                                                })
                                        })
                                        .collect()
                                })
                                .flex_1()
                                .track_scroll(&self.scroll),
                            ),
                    )
                    .child(format!(
                        "GPUI Kit 0.7.0 / gpui-pre 0.3.7 · synthetic state: {}",
                        self.running
                    )),
            )
    }
}
fn main() {
    gpui_kit::application().with_assets(Assets).run(|cx| {
        gpui_kit::init(cx);
        let menu = Menu::new();
        let show = MenuItem::new("显示窗口", true, None);
        let status = MenuItem::new("状态：已停止（模拟）", false, None);
        let update = MenuItem::new("更新状态", true, None);
        let hide = MenuItem::new("隐藏窗口", true, None);
        let quit = MenuItem::new("退出", true, None);
        menu.append_items(&[&show, &status, &update, &hide, &quit])
            .expect("tray menu");
        let icon = tray_icon::Icon::from_rgba([0x34, 0x99, 0x68, 0xff].repeat(18 * 18), 18, 18)
            .expect("icon");
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(icon)
            .with_tooltip("Veyra P0 prototype")
            .build()
            .expect("tray");
        let (sender, receiver) = mpsc::channel();
        MenuEvent::set_event_handler(Some(move |event| {
            let _ = sender.send(event);
        }));
        let (window, view) = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1280.), px(720.)), cx)),
                window_background: WindowBackgroundAppearance::Blurred,
                titlebar: Some(TitlebarOptions {
                    title: Some("Veyra GPUI Prototype".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| cx.new(|cx| Prototype::new(window, cx, tray, status)),
        )
        .expect("prototype window");
        cx.activate(true);
        // 窗口已真实创建后才启动验收；后台线程仍是同一 GUI PID，不创建 CLI child。
        #[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
        let gui_owner_receiver = p0_05_gui_owner::start();
        #[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
        if gui_owner_receiver.is_some() {
            view.update(cx, |view, cx| {
                view.gui_owner_status = Some("P0-05 GUI Owner: checking");
                cx.notify();
            });
        }
        // GPUI foreground task owns the receiver and UI handles. Only MenuEvent crosses the channel.
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(40))
                    .await;
                #[cfg(all(target_os = "macos", feature = "p0-05-gui-owner"))]
                if let Some(receiver) = &gui_owner_receiver
                    && let Ok(status) = receiver.try_recv()
                {
                    cx.update(|cx| {
                        view.update(cx, |view, cx| {
                            view.gui_owner_status = Some(status);
                            cx.notify();
                        });
                    });
                }
                while let Ok(event) = receiver.try_recv() {
                    cx.update(|cx| {
                        if event.id == show.id() {
                            cx.activate(true);
                            let _ = window.update(cx, |_, window, _| window.activate_window());
                        } else if event.id == hide.id() {
                            cx.hide();
                        } else if event.id == update.id() {
                            view.update(cx, |view, cx| view.toggle_state(cx));
                        } else if event.id == quit.id() {
                            cx.quit();
                        }
                    });
                }
            }
        })
        .detach();
    });
}
