//! 唯一采集器的页面消费者；页面/暂停/清空不拥有内核或 Runtime 诊断写权。
use super::{
    components::{
        icon_button, select,
        select::{SelectEvent, SelectState},
    },
    i18n::tr,
    tokens::{self as t, logs as lt},
};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, IndexPath,
        input::{Input, InputEvent, InputState},
    },
    prelude::*,
    *,
};
use std::{
    sync::{Arc, Weak},
    time::Duration,
};
use veyra_core::application::observability::{
    controller::{LogDelivery, LogStatus, LogSubscription, ObservationReader},
    logs::{Buffer, Category, Level, Record, normalize_query, time_label},
};

/// 随可见行的 keyed state 保留；拖选变化即时刷新，离开虚拟列表后连同订阅释放。
struct LogTextSelection {
    handle: gpui_kit::base::TextSelectionHandle,
    _refresh: Subscription,
}

pub struct LogsView {
    services: Weak<crate::services::AppServices>,
    reader: Option<ObservationReader>,
    receiver: Option<LogSubscription>,
    pub buffer: Buffer,
    status: Option<LogStatus>,
    input: Entity<InputState>,
    text_focus: FocusHandle,
    level: Entity<SelectState>,
    category: Entity<SelectState>,
    visible: bool,
    rows: Vec<Arc<Record>>,
    exporting: bool,
    _subscriptions: Vec<Subscription>,
    _poll: Task<()>,
}
impl LogsView {
    pub fn new(
        services: &Arc<crate::services::AppServices>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(tr(cx, "搜索 | Regex")));
        let level = cx.new(|cx| {
            SelectState::new(
                Level::ALL.iter().map(|l| l.label()).collect::<Vec<_>>(),
                Some(IndexPath::new(2)),
                window,
                cx,
            )
        });
        let category = cx.new(|cx| {
            SelectState::new(
                std::iter::once("全部")
                    .chain(Category::ALL.iter().map(|c| c.label()))
                    .collect::<Vec<_>>(),
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let subscriptions = vec![
            cx.subscribe(&input, |this, input, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    this.buffer.query(input.read(cx).value().as_ref());
                    this.project(cx);
                }
            }),
            cx.subscribe(&level, |this, _, event: &SelectEvent, cx| {
                if let SelectEvent::Confirm(Some(value)) = event
                    && let Some(l) = Level::ALL.iter().find(|l| l.label() == value.as_ref())
                {
                    this.buffer.minimum = *l;
                    this.project(cx);
                }
            }),
            cx.subscribe(&category, |this, _, event: &SelectEvent, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    this.buffer.category = Category::ALL
                        .iter()
                        .copied()
                        .find(|c| c.label() == value.as_ref());
                    this.project(cx);
                }
            }),
        ];
        let poll = cx.spawn(async move |entity, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if entity.update(cx, |this, cx| this.poll(cx)).is_err() {
                    break;
                }
            }
        });
        Self {
            services: Arc::downgrade(services),
            reader: None,
            receiver: None,
            buffer: Buffer::default(),
            status: None,
            input,
            text_focus: cx.focus_handle(),
            level,
            category,
            visible: false,
            rows: Vec::new(),
            exporting: false,
            _subscriptions: subscriptions,
            _poll: poll,
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        self.visible = visible;
        if let Some(receiver) = &mut self.receiver {
            receiver.discard_pending()
        }
        self.poll(cx);
    }
    fn project(&mut self, cx: &mut Context<Self>) {
        self.rows = self.buffer.visible();
        cx.notify();
    }
    fn poll(&mut self, cx: &mut Context<Self>) {
        if self.reader.is_none() {
            self.reader = self
                .services
                .upgrade()
                .and_then(|s| s.manual_runtime.observation_reader());
        }
        let next = self.reader.as_ref().and_then(ObservationReader::log_status);
        let mut changed = self.status != next;
        if let Some(status) = &next {
            changed |= self.buffer.set_identity(status.identity.clone());
        } else {
            changed |= self.buffer.set_identity(None);
        }
        self.status = next;
        if self.receiver.is_none() {
            self.receiver = self
                .reader
                .as_ref()
                .and_then(ObservationReader::subscribe_logs);
        }
        if let Some(receiver) = &mut self.receiver {
            // 隐藏与暂停仍排空页面队列；恢复按钮另resubscribe切断已有积压。
            for _ in 0..128 {
                match receiver.try_recv() {
                    Some(LogDelivery::Record(record)) => {
                        if self.visible
                            && !self.buffer.paused
                            && self.status.as_ref().is_some_and(|s| s.available)
                        {
                            changed |= self.buffer.push(record);
                        }
                    }
                    Some(LogDelivery::Gap { dropped }) => {
                        if self.visible && !self.buffer.paused {
                            self.buffer.dropped += dropped;
                            changed = true;
                        }
                    }
                    Some(LogDelivery::Status(_)) => {}
                    Some(LogDelivery::Closed) => {
                        self.reader = None;
                        changed |= self.buffer.set_identity(None);
                        break;
                    }
                    None => break,
                }
            }
        }
        if changed {
            self.project(cx)
        }
    }
    fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        self.buffer.paused = !self.buffer.paused;
        if let Some(receiver) = &mut self.receiver {
            receiver.discard_pending();
        }
        cx.notify();
    }
    fn export(&mut self, cx: &mut Context<Self>) {
        if self.exporting || self.rows.is_empty() {
            return;
        }
        // 点击时的筛选结果快照；后续日志/筛选变化不改变本次导出内容。
        let bytes = self.buffer.export().into_bytes();
        self.exporting = true;
        cx.notify();
        let selected = crate::platform::macos::begin_log_export();
        let services = self.services.clone();
        cx.spawn(async move |entity, cx| {
            let selection = selected
                .await
                .unwrap_or(Err(crate::platform::PlatformError::FileDialogUnavailable));
            let result = if let Some(s) = services.upgrade() {
                s.save_log_export(selection, bytes)
                    .await
                    .unwrap_or(Err(crate::platform::PlatformError::FileWriteFailed))
            } else {
                Err(crate::platform::PlatformError::FileWriteFailed)
            };
            let _ = entity.update(cx, |this, cx| {
                this.exporting = false;
                // 导出失败复用全局 toast；成功和取消保持安静，不挤占日志列表。
                if result.is_err() {
                    super::components::notice::notify_app(
                        super::components::Notice::Error,
                        "日志导出失败，请重试",
                        cx,
                    );
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn message(&self) -> Option<&'static str> {
        if self.buffer.invalid_regex {
            None
        } else if self.buffer.paused {
            Some("已暂停")
        } else if self.status.as_ref().is_none_or(|s| s.identity.is_none()) {
            Some("内核未运行")
        } else if self
            .status
            .as_ref()
            .is_some_and(|s| !s.available && s.gaps > 0)
        {
            Some("日志连接失败")
        } else if self.status.as_ref().is_some_and(|s| !s.available) {
            Some("正在连接日志")
        } else {
            None
        }
    }
}
impl Render for LogsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dark = cx.theme().mode.is_dark();
        let palette = super::theme::Palette::new(dark, false);
        let placeholder = tr(cx, "搜索 | Regex");
        if self.input.read(cx).presentation().placeholder().as_ref() != placeholder {
            self.input.update(cx, |input, cx| {
                input.set_placeholder(placeholder, window, cx)
            });
        }
        let clear_search = div()
            .id("logs-clear-search")
            .child("×")
            .text_color(rgb(palette.text).opacity(0.68))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, window, cx| {
                this.input
                    .update(cx, |input, cx| input.set_value("", window, cx));
                this.buffer.query("");
                this.project(cx);
            }));
        let search = Input::new(&self.input)
            .prefix(super::icons::icon("MagnifyingGlass", lt::SEARCH_ICON))
            .suffix(clear_search)
            .h(px(t::CONTROL))
            .px(px(t::GAP))
            .py(px(0.))
            .text_size(px(t::BODY))
            .line_height(px(t::BODY_LINE))
            .rounded(px(t::RADIUS))
            .bg(rgb(palette.surface).opacity(lt::SURFACE_ALPHA))
            .w(px(lt::SEARCH_WIDTH))
            .when(self.buffer.invalid_regex, |d| {
                d.border_1().border_color(rgb(lt::ERROR))
            });
        let toolbar = div()
            .flex()
            .h(px(lt::TOOLBAR))
            .flex_shrink_0()
            .p(px(t::GAP))
            .gap(px(t::GAP))
            .items_center()
            .bg(rgb(palette.surface).opacity(lt::SURFACE_ALPHA))
            .child(select(&self.level, tr(cx, "级别")).w(px(lt::LEVEL_WIDTH)))
            .child(select(&self.category, tr(cx, "类型")).w(px(lt::FILTER_WIDTH)))
            .child(search)
            .child(
                icon_button("logs-format", tr(cx, "格式化查询"), "Sparkles", t::CONTROL)
                    .bg(rgb(lt::TOOL_BACKGROUND))
                    .on_click(cx.listener(|this, _, window, cx| {
                        let query = normalize_query(this.input.read(cx).value().as_ref());
                        this.input
                            .update(cx, |input, cx| input.set_value(query.clone(), window, cx));
                        this.buffer.query(&query);
                        this.project(cx);
                    })),
            )
            .child(div().flex_1())
            .child(
                icon_button("logs-export", tr(cx, "导出"), "ArrowDownTray", t::CONTROL)
                    .with_tooltip(tr(cx, "导出"))
                    .bg(rgb(lt::TOOL_BACKGROUND))
                    .disabled(self.rows.is_empty() || self.exporting)
                    .on_click(cx.listener(|this, _, _, cx| this.export(cx))),
            )
            .child(
                icon_button(
                    "logs-pause",
                    tr(
                        cx,
                        if self.buffer.paused {
                            "继续"
                        } else {
                            "暂停"
                        },
                    ),
                    if self.buffer.paused { "Play" } else { "Pause" },
                    t::CONTROL,
                )
                .with_tooltip(tr(
                    cx,
                    if self.buffer.paused {
                        "继续"
                    } else {
                        "暂停"
                    },
                ))
                .bg(rgb(lt::TOOL_BACKGROUND))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_pause(cx))),
            )
            .child(
                icon_button("logs-clear", tr(cx, "清空"), "XMark", t::CONTROL)
                    .with_tooltip(tr(cx, "清空"))
                    .bg(rgb(lt::TOOL_BACKGROUND))
                    .disabled(self.buffer.is_empty())
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.buffer.clear();
                        this.project(cx);
                    })),
            );
        let rows = self.rows.clone();
        let text_focus = self.text_focus.clone();
        let list = uniform_list("logs-list", rows.len(), move |range, window, cx| {
            range
                .map(|i| {
                    let r = &rows[i];
                    let (color, bg) = match r.level {
                        Level::Info => (lt::INFO, 0x5ca7fa1f),
                        Level::Warning => (lt::WARNING, 0xeab66026),
                        Level::Error | Level::Fatal | Level::Panic => (lt::ERROR, 0xf06b6b21),
                        _ => (lt::DEBUG, lt::DEBUG_BACKGROUND),
                    };
                    // Kit窗口选择层保留全文；原生StyledText遵守单行ellipsis，不解析正文HTML。
                    let text_id = format!(
                        "log-body-{}-{}-{}",
                        r.identity.generation, r.stream_generation, r.sequence
                    );
                    let selection = window.use_keyed_state(text_id.clone(), cx, |window, cx| {
                        let handle =
                            gpui_kit::base::TextSelectionHandle::new(r.body().to_owned(), cx);
                        let refresh = handle.refresh_window_on_change(window, cx);
                        LogTextSelection {
                            handle,
                            _refresh: refresh,
                        }
                    });
                    let text = gpui_kit::base::SelectableText::with_handle(
                        text_id,
                        selection.read(cx).handle.clone(),
                        r.body().to_owned(),
                    )
                    .document_order(i as u64);
                    div()
                        .id(format!("log-row-{}-{}", r.stream_generation, r.sequence))
                        .h(px(lt::ROW_PITCH))
                        .pb(px(t::GAP))
                        .child(
                            div()
                                .flex()
                                .h(px(lt::ROW_HEIGHT))
                                .items_center()
                                .gap(px(t::GAP))
                                .px(px(t::SECTION_PADDING))
                                .rounded(px(lt::ROW_RADIUS))
                                .bg(rgb(palette.surface).opacity(lt::SURFACE_ALPHA))
                                .text_size(px(t::BODY))
                                .line_height(px(t::BODY_LINE))
                                .text_color(rgb(palette.text))
                                .child(
                                    div()
                                        .w(px(lt::NUMBER_WIDTH))
                                        .flex_shrink_0()
                                        .child(format!("{:02}.", r.sequence)),
                                )
                                .child(
                                    div()
                                        .w(px(lt::TIME_WIDTH))
                                        .flex_shrink_0()
                                        .text_size(px(lt::TAG_FONT))
                                        .font_weight(super::theme::MISANS_MEDIUM)
                                        .whitespace_nowrap()
                                        .font_features(FontFeatures(Arc::new(vec![(
                                            "tnum".into(),
                                            1,
                                        )])))
                                        .px(px(lt::BADGE_X))
                                        .py(px(lt::BADGE_Y))
                                        .rounded(px(lt::BADGE_RADIUS))
                                        .text_color(rgb(t::ACCENT_STRONG))
                                        .bg(rgba(lt::TIME_BACKGROUND))
                                        .child(time_label(r.received_at_ms)),
                                )
                                .child(
                                    div()
                                        .w(px(lt::LEVEL_TAG_WIDTH))
                                        .flex_shrink_0()
                                        .text_center()
                                        .px(px(lt::BADGE_X))
                                        .py(px(lt::BADGE_Y))
                                        .rounded(px(lt::BADGE_RADIUS))
                                        .text_size(px(lt::TAG_FONT))
                                        .font_weight(super::theme::MISANS_MEDIUM)
                                        .text_color(rgb(color))
                                        .bg(rgba(bg))
                                        .child(r.level.label()),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .h(px(t::BODY_LINE))
                                        // 只限制文字为单行省略，不把选择层的滚动视口裁成20px正文。
                                        // 跨行拖选应使用外层日志列表视口，否则会误触发自动滚动和跳行。
                                        .whitespace_nowrap()
                                        .text_ellipsis()
                                        // 每行独立选择句柄，按显示顺序跨行复制；正文接管焦点，避免搜索框拦截 Cmd+C。
                                        .track_focus(&text_focus)
                                        .cursor_text()
                                        .on_mouse_down(MouseButton::Left, {
                                            let focus = text_focus.clone();
                                            move |_, window, cx| focus.focus(window, cx)
                                        })
                                        .child(text),
                                ),
                        )
                        .into_any_element()
                })
                .collect::<Vec<_>>()
        })
        .size_full();
        div()
            .flex()
            .flex_col()
            .size_full()
            .min_h_0()
            .child(toolbar)
            .when(self.message().is_some(), |d| {
                d.child(div().px(px(t::GAP)).child(tr(cx, self.message().unwrap())))
            })
            .when(self.buffer.dropped > 0, |d| {
                d.child(div().px(px(t::GAP)).child(format!(
                    "{}: {}",
                    tr(cx, "已丢弃"),
                    self.buffer.dropped
                )))
            })
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p(px(t::GAP))
                    .when(self.rows.is_empty(), |d| {
                        d.child(
                            div()
                                .h(px(lt::EMPTY_HEIGHT))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(lt::ROW_RADIUS))
                                .bg(rgb(palette.surface).opacity(lt::SURFACE_ALPHA))
                                .child(tr(
                                    cx,
                                    if self.buffer.invalid_regex {
                                        "正则表达式格式不正确"
                                    } else if self.buffer.is_empty() {
                                        "暂无日志"
                                    } else {
                                        "没有匹配的日志"
                                    },
                                )),
                        )
                    })
                    .when(!self.rows.is_empty(), |d| d.child(list)),
            )
    }
}
