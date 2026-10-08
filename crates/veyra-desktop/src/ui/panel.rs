use super::components::select::{SelectEvent, SelectState};
use super::components::*;
use crate::ui::i18n::tr;
use gpui_kit::{
    component::{
        IndexPath, WindowExt,
        input::InputState,
        slider::{Slider, SliderEvent, SliderState},
    },
    prelude::*,
    *,
};
use std::path::PathBuf;
use veyra_core::domain::*;
pub enum PanelEvent {
    Edit(DesktopVisualPreferences),
    Import(PathBuf),
    Retry,
    ChooseImage,
    ExportEvidence,
    ClipboardWrite,
    ClipboardRestore,
    ExternalLink,
}
pub struct PanelView {
    pub draft: DesktopVisualPreferences,
    pub behavior: Option<Entity<super::behavior_panel::BehaviorPanel>>,
    language: Entity<SelectState>,
    theme: Entity<SelectState>,
    opacity: Entity<SliderState>,
    blur: Entity<SliderState>,
    pub input: Entity<InputState>,
    path: Entity<InputState>,
    background_adjust: bool,
    modal_trigger: FocusHandle,
    modal_input: Entity<InputState>,
    modal_select: Entity<SelectState>,
    _subscriptions: Vec<gpui_kit::Subscription>,
}
impl EventEmitter<PanelEvent> for PanelView {}
impl PanelView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let language = cx.new(|cx| {
            SelectState::new(
                vec!["English", "简体中文", "繁體中文"],
                Some(IndexPath::new(1)),
                window,
                cx,
            )
        });
        let theme = cx.new(|cx| {
            SelectState::new(
                vec!["跟随系统", "亮色", "暗色"],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let opacity = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(90.)
        });
        let blur = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(40.)
                .step(1.)
                .default_value(10.)
        });
        let input =
            cx.new(|cx| InputState::new(window, cx).placeholder("中文 · English · 123 · 😀 🇨🇳 🇺🇸"));
        let path =
            cx.new(|cx| InputState::new(window, cx).placeholder("本地图片路径 · PNG/JPEG ≤ 8 MiB"));
        let modal_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Modal 中文输入 · 仅会话"));
        let modal_select = cx.new(|cx| {
            SelectState::new(
                vec!["Alpha", "Beta", "Gamma"],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe_in(
                &modal_select,
                window,
                |_, _, _: &DismissEvent, window, cx| {
                    eprintln!(
                        "overlay dropdown dismissed; modal remains={}",
                        window.has_active_dialog(cx)
                    )
                },
            ),
            cx.subscribe(&language, |this, _, event, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.draft.language = match value.as_ref() {
                        "English" => DesktopLanguage::English,
                        "繁體中文" => DesktopLanguage::TraditionalChinese,
                        _ => DesktopLanguage::SimplifiedChinese,
                    };
                    this.changed(cx);
                }
            }),
            cx.subscribe(&theme, |this, _, event, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.draft.theme_mode = if *value == "暗色" {
                        DesktopThemeMode::Dark
                    } else if *value == "亮色" {
                        DesktopThemeMode::Light
                    } else {
                        DesktopThemeMode::System
                    };
                    this.changed(cx);
                }
            }),
            cx.subscribe(&opacity, |this, _, event, cx| {
                if let SliderEvent::Change(value) = event {
                    this.draft.background_opacity = value.start().round() as u8;
                    this.changed(cx);
                }
            }),
            cx.subscribe(&blur, |this, _, event, cx| {
                if let SliderEvent::Change(value) = event {
                    this.draft.background_blur = value.start().round() as u8;
                    this.changed(cx);
                }
            }),
        ];
        Self {
            behavior: None,
            draft: Default::default(),
            language,
            theme,
            opacity,
            blur,
            input,
            path,
            background_adjust: false,
            modal_trigger: cx.focus_handle(),
            modal_input,
            modal_select,
            _subscriptions: subscriptions,
        }
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        cx.emit(PanelEvent::Edit(self.draft.clone()));
        cx.notify();
    }
    pub fn project(
        &mut self,
        draft: &DesktopVisualPreferences,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.draft.background != draft.background {
            self.background_adjust = matches!(draft.background, DesktopBackground::ManagedAsset(_));
        }
        self.draft = draft.clone();
        self.language.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match draft.language {
                    DesktopLanguage::English => 0,
                    DesktopLanguage::SimplifiedChinese => 1,
                    DesktopLanguage::TraditionalChinese => 2,
                })),
                window,
                cx,
            )
        });
        self.theme.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match draft.theme_mode {
                    DesktopThemeMode::System => 0,
                    DesktopThemeMode::Light => 1,
                    DesktopThemeMode::Dark => 2,
                })),
                window,
                cx,
            )
        });
        self.opacity.update(cx, |s, cx| {
            s.set_value(draft.background_opacity as f32, window, cx)
        });
        self.blur.update(cx, |s, cx| {
            s.set_value(draft.background_blur as f32, window, cx)
        });
        cx.notify();
    }
    fn modal(&mut self, trigger: FocusHandle, window: &mut Window, cx: &mut Context<Self>) {
        let focus = super::components::overlay::ModalFocus::capture(Some(trigger));
        let input = self.modal_input.clone();
        let select = self.modal_select.clone();
        window.open_dialog(cx, move |dialog, _, _| {
            let focus = focus.clone();
            dialog
                .title("视觉组件 / Visual components")
                .w(px(480.))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_4()
                        .child("中文 IME / nested Select / Escape · session only")
                        .child(text_input(&input))
                        .child(super::components::select(&select, "Modal Select")),
                )
                .on_cancel(|_, _, cx| {
                    super::components::overlay::escape_target(
                        gpui_kit::base::GlobalState::is_in_deferred_context(cx),
                        true,
                    ) == super::components::overlay::EscapeTarget::Modal
                })
                .on_close(move |_, window, cx| {
                    if let Some(trigger) = focus.return_to() {
                        trigger.focus(window, cx);
                        eprintln!(
                            "overlay modal closed; trigger focus returned={}",
                            trigger.is_focused(window)
                        );
                    }
                })
        });
        self.modal_input.read(cx).focus_handle(cx).focus(window, cx);
        eprintln!("overlay modal opened; initial focus=input");
    }
}
fn label(
    lang: DesktopLanguage,
    zh: &'static str,
    en: &'static str,
    tw: &'static str,
) -> &'static str {
    match lang {
        DesktopLanguage::English => en,
        DesktopLanguage::TraditionalChinese => tw,
        _ => zh,
    }
}
fn row(label: &'static str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .h(px(40.))
        .gap_3()
        .border_b_1()
        .border_color(rgba(0x80808022))
        .child(div().flex_1().child(label))
        .child(div().flex_shrink_0().child(control))
}
impl PanelView {
    fn render_evidence(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let lang = self.draft.language;
        let l = |zh, en, tw| label(lang, zh, en, tw);
        let path = PathBuf::from(self.path.read(cx).value().as_str());
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap_1()
            .min_h(px(680.))
            .w_full()
            .when(cfg!(debug_assertions), |view| {
                view.child(
                    div()
                        .flex_col()
                        .gap_2()
                        .child("Platform evidence · preview namespace · no Runtime/tray")
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(button("platform-export", "导出平台测试文件").on_click(
                                    cx.listener(|_, _, _, cx| cx.emit(PanelEvent::ExportEvidence)),
                                ))
                                .child(button("clipboard-write", "Clipboard marker").on_click(
                                    cx.listener(|_, _, _, cx| cx.emit(PanelEvent::ClipboardWrite)),
                                ))
                                .child(button("clipboard-restore", "Restore clipboard").on_click(
                                    cx.listener(|_, _, _, cx| {
                                        cx.emit(PanelEvent::ClipboardRestore)
                                    }),
                                ))
                                .child(button("external-link", "OS link handoff").on_click(
                                    cx.listener(|_, _, _, cx| cx.emit(PanelEvent::ExternalLink)),
                                )),
                        ),
                )
            })
            .child(
                div()
                    .text_size(px(16.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(l("通用", "General", "通用")),
            )
            .child(row(
                l("面板语言", "Language", "面板語言"),
                select(&self.language, "Language"),
            ))
            .child(row(
                l("主题", "Theme", "主題"),
                select(&self.theme, "Theme"),
            ))
            .child(row(
                l("面板背景", "Background", "面板背景"),
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            "choose-background",
                            l("选择本地图片", "Choose local image", "選擇本地圖片"),
                        )
                        .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::ChooseImage))),
                    )
                    .child(
                        button("clear-background", l("清除背景", "Clear", "清除背景")).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.draft.background = DesktopBackground::None;
                                this.changed(cx);
                            }),
                        ),
                    )
                    .child(match &self.draft.background {
                        DesktopBackground::None => "None".into(),
                        DesktopBackground::ManagedAsset(id) => format!("{}…", &id.0[..8]),
                    }),
            ))
            .child(row(
                l("透明度", "Opacity", "透明度"),
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .w(px(260.))
                    .child(Slider::new(&self.opacity).w(px(208.)))
                    .child(format!("{}%", self.draft.background_opacity)),
            ))
            .child(row(
                l("毛玻璃强度", "Blur preference", "毛玻璃強度"),
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .w(px(260.))
                    .child(Slider::new(&self.blur).w(px(208.)))
                    .child(format!("{}", self.draft.background_blur)),
            ))
            .child(div().text_xs().child(l(
                "图片 blur：0–40 缓存处理；Window-level blur 非零开启，无 card backdrop blur。",
                "Image blur: cached 0–40. Native window blur: on/off only. No card backdrop blur.",
                "圖片 blur：0–40 快取；Window-level blur 非零開啟，無 card backdrop blur。",
            )))
            .child(row(
                l("全局圆角", "Panel radius", "全局圓角"),
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        button("radius-down", "−").on_click(cx.listener(|this, _, _, cx| {
                            this.draft.global_radius = this.draft.global_radius.saturating_sub(1);
                            this.changed(cx);
                        })),
                    )
                    .child(format!("{} px", self.draft.global_radius))
                    .child(
                        button("radius-up", "+").on_click(cx.listener(|this, _, _, cx| {
                            this.draft.global_radius = (this.draft.global_radius + 1).min(24);
                            this.changed(cx);
                        })),
                    ),
            ))
            .child(row(
                l("折叠侧栏", "Collapse sidebar", "摺疊側欄"),
                toggle(
                    "sidebar-switch",
                    self.draft.sidebar_collapsed,
                    "Collapse sidebar",
                )
                .on_change(cx.listener(|this, value, _, cx| {
                    this.draft.sidebar_collapsed = *value;
                    this.changed(cx);
                })),
            ))
            .child(div().text_xs().child(l(
                "行为偏好可切换查看。本地图片通过原生文件选择器导入。",
                "Switch to Behaviour settings. Native macOS file picker.",
                "行為偏好可切換查看。原生檔案選擇器已接入。",
            )))
            .when(cfg!(debug_assertions), |view| {
                view.child(
                    div()
                        .flex()
                        .gap_2()
                        .child(text_input(&self.path).w(px(420.)))
                        .child(
                            button(
                                "import-background",
                                l("导入本地图片", "Import local image", "匯入本地圖片"),
                            )
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.emit(PanelEvent::Import(PathBuf::from(
                                    this.path.read(cx).value().as_str(),
                                )))
                            })),
                        ),
                )
            })
            .child(
                div()
                    .id("background-drop")
                    .h(px(48.))
                    .w_full()
                    .border_1()
                    .border_color(rgb(0x70c996))
                    .rounded_lg()
                    .flex()
                    .items_center()
                    .justify_center()
                    .drag_over::<FileDrag>(|style, _, _, _| style.bg(rgba(0x70c99644)))
                    .on_drop(cx.listener(|_, file: &FileDrag, _, cx| {
                        eprintln!("drag drop stable-id={}", file.id);
                        cx.emit(PanelEvent::Import(file.path.clone()));
                    }))
                    .on_drop(cx.listener(|_, files: &ExternalPaths, _, cx| {
                        if let Some(path) = files.paths().first() {
                            cx.emit(PanelEvent::Import(path.clone()));
                        }
                    }))
                    .child("Drop PNG / JPEG · 8 MiB · managed copy"),
            )
            .child(
                div()
                    .id("background-drag-source")
                    .cursor_pointer()
                    .text_xs()
                    .child("Drag local image → drop area (stable ID)")
                    .on_drag(
                        FileDrag {
                            id: "background-source",
                            path,
                        },
                        |file, _, _, cx| {
                            eprintln!("drag start stable-id={}", file.id);
                            cx.new(|_| DragPreview(file.id))
                        },
                    ),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(text_input(&self.input).w(px(380.)))
                    .child(
                        div()
                            .id("visual-modal-focus")
                            .track_focus(&self.modal_trigger)
                            .tab_stop(true)
                            .rounded(px(9.))
                            .when(self.modal_trigger.is_focused(window), |d| {
                                d.border_2().border_color(rgb(0x70c996))
                            })
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                    cx.stop_propagation();
                                    this.modal(this.modal_trigger.clone(), window, cx);
                                }
                            }))
                            .child(
                                button("visual-modal", l("打开弹窗", "Open modal", "開啟彈窗"))
                                    .tab_stop(false)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.modal_trigger.focus(window, cx);
                                        this.modal(this.modal_trigger.clone(), window, cx)
                                    })),
                            ),
                    )
                    .child(
                        button("retry-visual", l("重试保存", "Retry save", "重試儲存"))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Retry))),
                    ),
            )
            .child(
                div().text_xs().child(
                    "系统字体 · 中文 English 123 · 😀 🌿 🇨🇳 🇺🇸 · Regular / Semibold · 14 / 20",
                ),
            )
    }
}

impl Render for PanelView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if evidence_visible() {
            return self.render_evidence(window, cx);
        }
        use super::{icons::icon, tokens as t};
        use gpui_kit::component::{ActiveTheme, Disableable};
        let dark = cx.theme().mode.is_dark();
        // 本地背景只通过系统选择器/拖放导入；展示文件名而非一个没有提交语义的路径输入。
        let background = div()
            .id("background-control")
            .flex()
            .h(px(t::CONTROL))
            .gap(px(t::GAP))
            .on_drop(cx.listener(|_, files: &ExternalPaths, _, cx| {
                if let Some(path) = files.paths().first() {
                    cx.emit(PanelEvent::Import(path.clone()));
                }
            }))
            .child(
                div()
                    .flex()
                    .h_full()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .w(px(t::BACKGROUND_WIDTH))
                            .h_full()
                            .px(px(t::PAD))
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded_l(px(t::RADIUS))
                            .bg(cx.theme().input)
                            .overflow_hidden()
                            .child(match &self.draft.background {
                                DesktopBackground::ManagedAsset(_) => tr(cx, "本地图片"),
                                DesktopBackground::None => "",
                            })
                            .when(
                                matches!(self.draft.background, DesktopBackground::ManagedAsset(_)),
                                |d| {
                                    d.child(
                                        icon_button(
                                            "clear-background",
                                            tr(cx, "清空面板背景"),
                                            "XMark",
                                            16.,
                                        )
                                        .on_click(
                                            cx.listener(|this, _, _, cx| {
                                                this.draft.background = DesktopBackground::None;
                                                this.changed(cx);
                                            }),
                                        ),
                                    )
                                },
                            ),
                    )
                    .child(
                        button("choose-background", "")
                            .w(px(t::UPLOAD_WIDTH))
                            .rounded_l_none()
                            .child(icon("ArrowUpTray", t::ICON_SMALL))
                            .accessibility_label(tr(cx, "上传面板背景"))
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::ChooseImage))),
                    ),
            )
            .when(
                matches!(self.draft.background, DesktopBackground::ManagedAsset(_)),
                |d| {
                    d.child(
                        icon_button(
                            "adjust-background",
                            tr(cx, "调整面板背景"),
                            "AdjustmentsHorizontal",
                            t::CONTROL,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.background_adjust = !this.background_adjust;
                            cx.notify();
                        })),
                    )
                },
            );
        let radius = div()
            .relative()
            .flex()
            .items_center()
            .w(px(190.))
            .h(px(t::CONTROL))
            .child(
                button("radius-down", "-")
                    .rounded_r(px(0.))
                    .w(px(crate::ui::tokens::COLUMN_GAP))
                    .disabled(self.draft.global_radius == 0)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.draft.global_radius = this.draft.global_radius.saturating_sub(1);
                        this.changed(cx);
                    })),
            )
            .child(
                div()
                    .w(px(96.))
                    .flex_shrink_0()
                    .ml(px(-1.))
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().input)
                    .child(format!("{}px", self.draft.global_radius)),
            )
            .child(
                button("radius-up", "+")
                    .rounded_l(px(0.))
                    .ml(px(-1.))
                    .w(px(crate::ui::tokens::COLUMN_GAP))
                    .disabled(self.draft.global_radius == 24)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.draft.global_radius = (this.draft.global_radius + 1).min(24);
                        this.changed(cx);
                    })),
            )
            // React 右按钮 -1px 拼接覆盖了中段边线；最后补绘同一条 1px 分隔线。
            .child(
                div()
                    .absolute()
                    .right(px(t::COLUMN_GAP - 1.))
                    .top_0()
                    .bottom_0()
                    .w(px(1.))
                    .bg(cx.theme().border),
            );
        let provider = self
            .behavior
            .as_ref()
            .map(|b| b.read(cx).provider_control());
        let layout = SettingsLayout::new(
            f32::from(window.viewport_size().width),
            self.draft.sidebar_collapsed,
        );
        setting_section(tr(cx, "通用"), dark, self.draft.global_radius as f32)
            .child(layout.grid()
                .child(compact_setting(tr(cx, "面板语言"), select(&self.language, tr(cx, "面板语言"))))
                .child(compact_setting(tr(cx, "面板背景"), background))
                .when(
                    self.background_adjust
                        && matches!(self.draft.background, DesktopBackground::ManagedAsset(_)),
                    |grid| grid
                        .child(compact_setting(tr(cx, "透明度"), panel_slider(&self.opacity, cx)))
                        .child(compact_setting(tr(cx, "毛玻璃强度"), panel_slider(&self.blur, cx))),
                )
                .child(compact_setting(tr(cx, "全局圆角"), radius))
                .child(compact_setting(tr(cx, "主题"), select(&self.theme, tr(cx, "主题"))))
                .child(compact_setting(
                    tr(cx, "修改密码"),
                    button("password-unavailable", tr(cx, "修改密码"))
                        .disabled(true)
                        .tooltip(tr(cx, "本地桌面未提供访问密码服务")),
                ))
                .child(compact_setting(super::components::help_label(tr(cx, "IP信息API"), "ip-api-help", tr(cx, "此API会用于IP检查中全球节点IP信息查询、连接详情中的IP地理信息查询、面板DNS查询中的IP地理信息查询。"), cx), div().children(provider))))
    }
}
