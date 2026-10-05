//! React PanelIconPicker 的本地图标目录；不解析 URL、不访问外部资源。
use super::{button, text_input};
use crate::ui::icons::icon;
use crate::ui::tokens as t;
use gpui_kit::{
    component::{
        ActiveTheme,
        button::{Button, ButtonVariants},
        input::{InputEvent, InputState},
        popover::Popover,
    },
    prelude::*,
    *,
};
use std::sync::{Arc, OnceLock};
struct Entry {
    code: String,
    label: String,
    category: String,
    image: Arc<Image>,
}
fn entries() -> &'static [Entry] {
    static ENTRIES: OnceLock<Vec<Entry>> = OnceLock::new();
    ENTRIES.get_or_init(|| {
        let data: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../src/openbox/assets/panel-icons.json"
        ))
        .expect("bundled React icon catalog");
        data.into_iter()
            .map(|v| {
                let source = v["asset"]
                    .as_str()
                    .unwrap()
                    .split_once(',')
                    .expect("bundled SVG data URI")
                    .1;
                let mut bytes = Vec::new();
                let mut chars = source.as_bytes().iter().copied();
                while let Some(c) = chars.next() {
                    if c == b'%' {
                        let a = (chars.next().unwrap() as char).to_digit(16).unwrap();
                        let b = (chars.next().unwrap() as char).to_digit(16).unwrap();
                        bytes.push((a * 16 + b) as u8);
                    } else {
                        bytes.push(c);
                    }
                }
                Entry {
                    code: v["code"].as_str().unwrap().into(),
                    label: v["label"].as_str().unwrap().into(),
                    category: v["category"].as_str().unwrap().into(),
                    image: Arc::new(Image::from_bytes(ImageFormat::Svg, bytes)),
                }
            })
            .collect()
    })
}
pub struct IconPicked(pub String);
pub struct IconPicker {
    pub value: String,
    search: Entity<InputState>,
    category: usize,
    scroll: ScrollHandle,
    _subscription: Subscription,
}
impl EventEmitter<IconPicked> for IconPicker {}
impl IconPicker {
    pub fn new(value: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索国家/地区"));
        let subscription = cx.subscribe(&search, |_, _, event, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify()
            }
        });
        Self {
            value,
            search,
            category: 0,
            scroll: ScrollHandle::new(),
            _subscription: subscription,
        }
    }
}
impl Render for IconPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current = entries().iter().find(|e| e.code == self.value);
        let entity = cx.entity();
        Popover::new("site-icon-picker")
            .appearance(false)
            .anchor(Anchor::BottomLeft)
            .offset(px(4.))
            .track_focus(&self.search.focus_handle(cx))
            .on_open_change(cx.listener(|this, open, window, cx| {
                if *open {
                    this.category = 0;
                    let index = entries()
                        .iter()
                        .position(|e| e.code == this.value)
                        .unwrap_or(0);
                    this.scroll
                        .set_offset(point(px(0.), -px((index as f32 * 32. - 100.).max(0.))));
                    this.search.update(cx, |s, cx| s.set_value("", window, cx));
                    cx.notify();
                }
            }))
            .trigger(
                Button::new("icon-trigger")
                    .ghost()
                    .accessibility_label("选择测试站点图标")
                    .w(px(t::SITE_ICON_WIDTH))
                    .h(px(t::CONTROL))
                    .px(px(t::GAP))
                    .rounded(px(t::RADIUS))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().input)
                    .child(
                        div()
                            .flex()
                            .w_full()
                            .items_center()
                            .justify_around()
                            .children(current.map(|e| img(e.image.clone()).size(px(20.))))
                            .child(icon("ChevronDown", 12.).text_color(cx.theme().foreground)),
                    ),
            )
            .content(move |_, _, cx| {
                let picker = entity.read(cx);
                let query = picker.search.read(cx).value().to_lowercase();
                let category = ["all", "region", "brand", "other"][picker.category];
                let popup = cx.entity();
                let mut list = div()
                    .id("icon-options")
                    .track_scroll(&picker.scroll)
                    .overflow_y_scroll()
                    .min_h_0()
                    .max_h(px(232.));
                for (index, e) in entries().iter().enumerate().filter(|(_, e)| {
                    (category == "all" || e.category == category)
                        && format!("{} {}", e.label, e.code)
                            .to_lowercase()
                            .contains(&query)
                }) {
                    let entity = entity.clone();
                    let popup = popup.clone();
                    let code = e.code.clone();
                    list = list.child(
                        Button::new(("icon-option", index))
                            .ghost()
                            .w_full()
                            .h(px(t::CONTROL))
                            .px(px(t::GAP))
                            .justify_start()
                            .rounded(px(6.))
                            .when(e.code == picker.value, |b| {
                                b.bg(if cx.theme().mode.is_dark() {
                                    rgb(0x151111)
                                } else {
                                    rgba(0xe8e8e8bf)
                                })
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .w_full()
                                    .gap(px(t::GAP))
                                    .child(img(e.image.clone()).size(px(16.)))
                                    .child(e.label.clone()),
                            )
                            .on_click(move |_, w, cx| {
                                entity.update(cx, |s, cx| {
                                    s.value = code.clone();
                                    cx.emit(IconPicked(code.clone()));
                                    cx.notify();
                                });
                                popup.update(cx, |s, cx| s.dismiss(w, cx));
                            }),
                    );
                }
                div()
                    .flex()
                    .flex_col()
                    .w(px(t::PICKER_WIDTH))
                    .max_h(px(t::PICKER_MAX_HEIGHT))
                    .p(px(t::GAP))
                    .gap(px(t::ROW_GAP))
                    .rounded(px(t::POPOVER_RADIUS))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().popover)
                    .text_color(cx.theme().foreground)
                    .shadow(vec![
                        BoxShadow::new(px(0.), px(4.), rgba(0x00000026).into())
                            .blur_radius(px(12.)),
                    ])
                    .child(
                        div()
                            .relative()
                            .child(
                                text_input(&picker.search).focus_style(cx.theme().foreground, 2.),
                            )
                            .child(
                                super::icon_button(
                                    "clear-icon-search",
                                    "清空图标搜索",
                                    "XMark",
                                    16.,
                                )
                                .absolute()
                                .top(px(8.))
                                .right(px(8.))
                                .on_click({
                                    let entity = entity.clone();
                                    move |_, w, cx| {
                                        entity.update(cx, |s, cx| {
                                            s.search
                                                .update(cx, |input, cx| input.set_value("", w, cx));
                                        });
                                    }
                                }),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .h(px(t::CONTROL))
                            .p(px(t::ROW_GAP))
                            .gap(px(t::ROW_GAP))
                            .children(
                                ["全部", "地区", "公司", "其他"]
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, label)| {
                                        let entity = entity.clone();
                                        // React tab flex:1 必须作用于直接 flex child；共享按钮外层保留焦点装饰。
                                        div().flex_1().min_w_0().child(
                                            button(label, label)
                                                .ghost()
                                                .rounded(px(6.))
                                                .text_color(cx.theme().muted_foreground)
                                                .w_full()
                                                .h(px(24.))
                                                .p_0()
                                                .when(i == picker.category, |b| {
                                                    b.bg(rgb(t::ACCENT))
                                                        .text_color(cx.theme().foreground)
                                                })
                                                .on_click(move |_, _, cx| {
                                                    entity.update(cx, |s, cx| {
                                                        s.category = i;
                                                        cx.notify();
                                                    })
                                                }),
                                        )
                                    }),
                            ),
                    )
                    .child(list)
            })
    }
}

#[cfg(test)]
mod tests {
    // 所有随包选项必须能解码，防止打开站点图标选择器时崩溃。
    #[test]
    fn bundled_picker_catalog_decodes() {
        let entries = super::entries();
        assert!(entries.iter().any(|e| e.code == "brand:google"));
        assert!(entries.iter().all(|e| !e.code.is_empty()));
    }
}
