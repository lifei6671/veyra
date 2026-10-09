//! React PanelIconPicker 的本地图标目录；不解析 URL、不访问外部资源。
use super::{button, text_input};
use crate::ui::i18n::{Locale, tr};
use crate::ui::icons::icon;
use crate::ui::tokens as t;
use gpui_kit::{
    component::{
        ActiveTheme, Disableable,
        button::{Button, ButtonCustomVariant, ButtonVariants},
        input::{InputEvent, InputState},
        popover::Popover,
    },
    prelude::*,
    *,
};
use std::sync::{Arc, OnceLock};
struct Entry {
    code: String,
    labels: [String; 3],
    category: String,
    search_text: String,
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
                let labels = [
                    veyra_core::domain::DesktopLanguage::SimplifiedChinese,
                    veyra_core::domain::DesktopLanguage::English,
                    veyra_core::domain::DesktopLanguage::TraditionalChinese,
                ]
                .map(|language| {
                    crate::ui::i18n::icon_label(language, v["label"].as_str().unwrap())
                });
                Entry {
                    code: v["code"].as_str().unwrap().into(),

                    category: v["category"].as_str().unwrap().into(),
                    search_text: format!("{} {}", labels.join(" "), v["code"].as_str().unwrap())
                        .to_lowercase(),
                    labels,
                    image: Arc::new(Image::from_bytes(ImageFormat::Svg, bytes)),
                }
            })
            .collect()
    })
}
// 搜索键随本地图标目录缓存，滚动重绘不重复分配/转换全部标签。
fn matching_options(category: &str, query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| {
            (category == "all" || e.category == category) && e.search_text.contains(&query)
        })
        .map(|(index, _)| index)
        .collect()
}
pub struct IconPicked(pub String);
pub struct IconPicker {
    pub value: String,
    pub expanded: bool,
    /// React 分组图标与主备页签图标拥有不同的真实字段宽度。
    pub expanded_width: f32,
    pub disabled: bool,
    search: Entity<InputState>,
    // Kit 的 value 包含 IME 预编辑内容；筛选只消费 Change 事件提交后的值。
    query: String,
    search_language: veyra_core::domain::DesktopLanguage,
    category: usize,
    scroll: UniformListScrollHandle,
    _subscription: Subscription,
}
impl EventEmitter<IconPicked> for IconPicker {}
impl IconPicker {
    // set_value 只更新显示，不发 Change；两个清空入口必须同步提交后的筛选词。
    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.query.clear();
        self.search.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }
    pub fn new(value: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("搜索国家/地区"));
        let subscription = cx.subscribe(&search, |this, input, event, cx| {
            if matches!(event, InputEvent::Change) {
                this.query = input.read(cx).value().to_string();
                this.scroll.scroll_to_item_strict(0, ScrollStrategy::Top);
                cx.notify()
            }
        });
        Self {
            expanded: false,
            expanded_width: t::groups::ICON_FIELD,
            disabled: false,
            value,
            search,
            query: String::new(),
            search_language: Default::default(),
            category: 0,
            scroll: UniformListScrollHandle::new(),
            _subscription: subscription,
        }
    }
}
impl Render for IconPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = cx.global::<Locale>().0;
        if self.search_language != language {
            self.search_language = language;
            self.search.update(cx, |search, cx| {
                search.set_placeholder(tr(cx, "搜索国家/地区"), window, cx)
            });
        }
        let current = entries().iter().find(|e| e.code == self.value);
        let entity = cx.entity();
        Popover::new("site-icon-picker")
            .appearance(false)
            .anchor(if self.expanded { Anchor::TopLeft } else { Anchor::BottomLeft })
            .offset(px(4.))
            .track_focus(&self.search.focus_handle(cx))
            .on_open_change(cx.listener(|this, open, window, cx| {
                if *open {
                    this.category = 0;
                    let index = picker_options("all", "", this.expanded)
                        .iter()
                        .position(|entry| entry.map_or(this.value.is_empty(), |index| entries()[index].code == this.value))
                        .unwrap_or(0);
                    this.scroll
                        .scroll_to_item_strict(index, ScrollStrategy::Center);
                    this.clear_search(window, cx);
                }
            }))
            .trigger(
                Button::new("icon-trigger")
                    .ghost()
                    .disabled(self.disabled)
                    .accessibility_label(tr(cx, if self.expanded { "分组图标" } else { "选择测试站点图标" }))
                    .w(px(if self.expanded { self.expanded_width } else { t::SITE_ICON_WIDTH }))
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
                            .gap(px(t::GAP))
                            .when(!self.expanded,|d|d.justify_around())
                            .children(current.map(|e| img(e.image.clone()).size(px(if self.expanded {t::ICON_SMALL}else{20.}))))
                            .when(self.expanded&&current.is_none(),|d|d.child(div().w(px(t::ICON)).text_color(cx.theme().muted_foreground).child("—")))
                            .when(self.expanded, |d| d.child(div().flex_1().min_w_0().overflow_hidden().text_ellipsis().child(current.map_or_else(||tr(cx,"无").to_owned(), |e|e.labels[match language {veyra_core::domain::DesktopLanguage::SimplifiedChinese=>0,veyra_core::domain::DesktopLanguage::English=>1,veyra_core::domain::DesktopLanguage::TraditionalChinese=>2}].clone()))))
                            .child(icon("ChevronDown", 12.).text_color(cx.theme().foreground)),
                    ),
            )
            .content(move |_, _, cx| {
                let picker = entity.read(cx);
                let query = &picker.query;
                let category = ["all", "region", "brand", "other"][picker.category];
                let popup = cx.entity();
                // 固定 32px 行复用 GPUI 可见范围列表，不再每帧创建 896 个 Button/SVG。
                let options = picker_options(category, query, picker.expanded);
                let list_height = (options.len() as f32 * t::CONTROL).min(t::PICKER_LIST_HEIGHT);
                let value = picker.value.clone();
                let list_entity = entity.clone();
                let list = uniform_list("icon-options", options.len(), move |range, _, cx| {
                    range
                        .map(|position| {
                            let entry = options[position].map(|index| &entries()[index]);
                            let entity = list_entity.clone();
                            let popup = popup.clone();
                            let code = entry.map_or_else(String::new, |e| e.code.clone());
                            let label = entry.map_or_else(|| tr(cx, "无").to_owned(), |e| e.labels[match cx.global::<Locale>().0 { veyra_core::domain::DesktopLanguage::SimplifiedChinese => 0, veyra_core::domain::DesktopLanguage::English => 1, veyra_core::domain::DesktopLanguage::TraditionalChinese => 2 }].clone());
                            Button::new(("icon-option", position))
                                .ghost()
                                .w_full()
                                .h(px(t::CONTROL))
                                .px(px(t::GAP))
                                .justify_start()
                                .rounded(px(6.))
                                .when(code == value, |b| {
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
                                        .children(entry.map(|e| img(e.image.clone()).size(px(16.))))
                                        .child(label),
                                )
                                .on_click(move |_, w, cx| {
                                    entity.update(cx, |s, cx| {
                                        s.value = code.clone();
                                        cx.emit(IconPicked(code.clone()));
                                        cx.notify();
                                    });
                                    popup.update(cx, |s, cx| s.dismiss(w, cx));
                                })
                        })
                        .collect()
                })
                .track_scroll(&picker.scroll)
                .w_full()
                .h(px(list_height))
                .min_h_0();
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
                                    tr(cx, "清空图标搜索"),
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
                                            s.clear_search(w, cx);
                                            s.scroll.scroll_to_item_strict(0, ScrollStrategy::Top);
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
                                        // React 分类 Tab 没有 Kit ghost 的 hover/active 配色；
                                        // 用户指定暗色绿色选中项黑字，未选中项仍用 muted；
                                        // 三个鼠标状态使用相同配色，不让 Kit 再覆盖。
                                        let foreground = if i == picker.category {
                                            if cx.theme().mode.is_dark() {
                                                rgb(t::PICKER_SELECTED_DARK_TEXT).into()
                                            } else {
                                                cx.theme().foreground
                                            }
                                        } else {
                                            cx.theme().muted_foreground
                                        };
                                        let background = if i == picker.category {
                                            rgb(t::ACCENT).into()
                                        } else {
                                            cx.theme().transparent
                                        };
                                        // React tab flex:1 必须作用于直接 flex child；共享按钮外层保留焦点装饰。
                                        div().flex_1().min_w_0().child(
                                            button(label, tr(cx, label))
                                                .custom(
                                                    ButtonCustomVariant::new(cx)
                                                        .foreground(foreground)
                                                        .color(background)
                                                        .hover(background)
                                                        .active(background),
                                                )
                                                // Kit Custom 的 normal 背景会混合透明色，
                                                // React 的 active Tab 则直接使用不透明 accent。
                                                .bg(background)
                                                .rounded(px(6.))
                                                .w_full()
                                                .h(px(24.))
                                                .p_0()
                                                .on_click(move |_, _, cx| {
                                                    entity.update(cx, |s, cx| {
                                                        s.category = i;
                                                        s.scroll.scroll_to_item_strict(
                                                            0,
                                                            ScrollStrategy::Top,
                                                        );
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

// 原版组图标菜单将“无”放在品牌与地区之间，选择后保存空 code。
fn picker_options(category: &str, query: &str, expanded: bool) -> Vec<Option<usize>> {
    let mut options: Vec<_> = matching_options(category, query)
        .into_iter()
        .map(Some)
        .collect();
    if expanded {
        let position = options
            .iter()
            .position(|index| entries()[index.unwrap()].category == "region")
            .unwrap_or(options.len());
        options.insert(position, None);
    }
    options
}

/// 列表与图标选择器复用同源 SVG，保持资源来源与许可。
pub fn group_icon(code: &str, size: f32, cx: &App) -> Div {
    let container = div().size(px(size)).flex_shrink_0();
    if let Some(entry) = entries().iter().find(|e| e.code == code) {
        container.child(img(entry.image.clone()).size(px(size)))
    } else {
        // GroupIcon 对空图标和未知 code 使用不同占位符。
        container
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(t::groups::FALLBACK_ICON))
            .line_height(px(size))
            .text_color(cx.theme().muted_foreground)
            .child(if code.is_empty() { "—" } else { "◎" })
    }
}

/// 自动国家分组沿用原版矩形国旗，区别于节点组列表的方形图标。
pub fn country_icon(code: &str, cx: &App) -> Div {
    div()
        .w(px(t::groups::COUNTRY_ICON_WIDTH))
        .h(px(t::groups::COUNTRY_ICON_HEIGHT))
        .flex_shrink_0()
        .border_1()
        .border_color(cx.theme().foreground.opacity(0.15))
        .children(entries().iter().find(|e| e.code == code).map(|e| {
            img(e.image.clone())
                .w(px(t::groups::COUNTRY_ICON_WIDTH))
                .h(px(t::groups::COUNTRY_ICON_HEIGHT))
                .object_fit(ObjectFit::Cover)
        }))
}

#[cfg(test)]
mod tests {
    // 保护语言切换：目录名称/变体不遗漏中文，搜索同时支持翻译名称和稳定 code。
    #[test]
    fn catalog_labels_and_search_cover_english() {
        for entry in super::entries() {
            assert!(
                !entry.labels[1]
                    .chars()
                    .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)),
                "{}",
                entry.labels[1]
            );
        }
        assert!(!super::matching_options("region", "United States").is_empty());
        assert!(!super::matching_options("brand", "Baidu").is_empty());
    }
    // 保护可见行映射：分类/搜索后的点击仍对应原目录，并保留名称/代码大小写不敏感搜索。
    #[test]
    fn filtered_positions_retain_icon_identity_and_search_contract() {
        let catalog = super::entries();
        assert_eq!(super::matching_options("all", "").len(), catalog.len());
        let by_code = super::matching_options("brand", "BRAND:BAIDU");
        assert!(!by_code.is_empty());
        assert!(
            by_code
                .iter()
                .all(|i| catalog[*i].category == "brand" && catalog[*i].code.contains("baidu"))
        );
        assert!(super::matching_options("all", "百度").contains(&by_code[0]));
        assert!(super::matching_options("region", "BRAND:BAIDU").is_empty());
        assert!(super::matching_options("all", "no-such-veyra-icon").is_empty());
    }
    // 所有随包选项必须能解码，防止打开站点图标选择器时崩溃。
    #[test]
    fn bundled_picker_catalog_decodes() {
        let entries = super::entries();
        assert!(entries.iter().any(|e| e.code == "brand:google"));
        assert!(entries.iter().all(|e| !e.code.is_empty()));
    }
}
