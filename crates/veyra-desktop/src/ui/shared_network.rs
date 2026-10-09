//! 「共享网络」实际 Core/Store 编辑页；不拥有 Runtime 或 P5-06 HTTP 分享服务。
use super::{
    components::{
        self, button,
        select::{Select, SelectEvent, SelectState},
        text_input,
    },
    i18n::tr,
    icons::icon,
    tokens::{self as t, shared_network as st},
};
use crate::shared_network::{Command, Draft, METHODS, PROTOCOLS};
use gpui_kit::{
    component::{
        ActiveTheme, Disableable, IndexPath,
        button::ButtonVariants,
        input::{InputEvent, InputState},
    },
    prelude::*,
    *,
};
use std::sync::Arc;
use veyra_core::{
    application::shared_inbounds::{
        SharedServerCommand,
        sharing::{random_secret, random_uuid, shared_address_is_local, shared_server_uri},
    },
    domain::*,
};
pub struct Updated(pub Box<AppState>);
impl EventEmitter<Updated> for SharedNetworkView {}
#[derive(Clone)]
struct ServerDrag {
    index: usize,
    name: String,
    version: ConfigVersion,
}
impl Render for ServerDrag {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p(px(t::PAD))
            .rounded(px(t::RADIUS))
            .bg(cx.theme().popover)
            .child(self.name.clone())
    }
}
// 名称、连接地址、端口、UUID、用户名、密码、混淆密码、只读分享链接。
const NAME: usize = 0;
const ADDRESS: usize = 1;
const PORT: usize = 2;
const UUID: usize = 3;
const USER: usize = 4;
const PASSWORD: usize = 5;
const OBFS: usize = 6;
const LINK: usize = 7;
pub struct SharedNetworkView {
    services: Arc<crate::services::AppServices>,
    state: Option<AppState>,
    draft: Option<Draft>,
    editing: bool,
    pub busy: bool,
    error: Option<&'static str>,
    certificate_busy: bool,
    generation: u64,
    inputs: Vec<Entity<InputState>>,
    protocol: Entity<SelectState>,
    method: Entity<SelectState>,
    _subscriptions: Vec<gpui_kit::Subscription>,
    deleting: Option<SharedServer>,
    sharing: Option<SharedServer>,
    scroll: ScrollHandle,
    modal_scroll: ScrollHandle,
}
impl SharedNetworkView {
    pub fn new(
        services: Arc<crate::services::AppServices>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let inputs = (0..8)
            .map(|_| cx.new(|cx| InputState::new(window, cx)))
            .collect::<Vec<_>>();
        let protocol =
            cx.new(|cx| SelectState::new(PROTOCOLS.to_vec(), Some(IndexPath::new(0)), window, cx));
        let method =
            cx.new(|cx| SelectState::new(METHODS.to_vec(), Some(IndexPath::new(0)), window, cx));
        let mut subscriptions = Vec::new();
        for input in &inputs {
            subscriptions.push(cx.subscribe(input, |_, _, _: &InputEvent, cx| cx.notify()));
        }
        subscriptions.push(cx.subscribe_in(
            &protocol,
            window,
            |this, _, event: &SelectEvent, window, cx| {
                if !this.busy
                    && let SelectEvent::Confirm(Some(value)) = event
                    && let Some(index) = PROTOCOLS.iter().position(|p| *p == value.as_str())
                {
                    if this.draft.as_ref().is_some_and(|d| d.protocol == index) {
                        return;
                    }
                    this.sync_draft(cx);
                    if let Some(d) = &mut this.draft
                        && let Err(e) = d.switch(index)
                    {
                        this.error = Some(e);
                    }
                    this.generation += 1;
                    this.set_inputs(window, cx);
                    this.ensure_certificate(cx);
                    cx.notify();
                }
            },
        ));
        subscriptions.push(cx.subscribe_in(
            &method,
            window,
            |this, _, event: &SelectEvent, window, cx| {
                if !this.busy
                    && let SelectEvent::Confirm(Some(value)) = event
                    && let Some(index) = METHODS.iter().position(|p| *p == value.as_str())
                {
                    if this.draft.as_ref().is_some_and(|d| d.method == index) {
                        return;
                    }
                    this.sync_draft(cx);
                    if let Some(d) = &mut this.draft {
                        d.method = index;
                        match random_secret(index) {
                            Ok(s) => d.password = s,
                            Err(e) => this.error = Some(e),
                        }
                    }
                    this.set_inputs(window, cx);
                    cx.notify();
                }
            },
        ));
        Self {
            services,
            state: None,
            draft: None,
            editing: false,
            busy: false,
            error: None,
            certificate_busy: false,
            generation: 0,
            inputs,
            protocol,
            method,
            _subscriptions: subscriptions,
            deleting: None,
            sharing: None,
            scroll: ScrollHandle::new(),
            modal_scroll: ScrollHandle::new(),
        }
    }
    pub fn project(&mut self, state: &AppState, cx: &mut Context<Self>) {
        if !self.busy {
            self.state = Some(state.clone());
            cx.notify();
        }
    }
    pub fn load(&mut self, cx: &mut Context<Self>) {
        self.send(Command::Read, cx);
    }
    fn sync_draft(&mut self, cx: &App) {
        if let Some(d) = &mut self.draft {
            let v = |i: usize| self.inputs[i].read(cx).value().to_string();
            d.name = v(NAME);
            d.address = v(ADDRESS);
            d.port = v(PORT);
            d.uuid = v(UUID);
            d.username = v(USER);
            d.password = v(PASSWORD);
            d.obfs = v(OBFS);
        }
    }
    fn set_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(d) = &self.draft {
            for (input, value) in self.inputs.iter().zip([
                d.name.clone(),
                d.address.clone(),
                d.port.clone(),
                d.uuid.clone(),
                d.username.clone(),
                d.password.clone(),
                d.obfs.clone(),
                String::new(),
            ]) {
                input.update(cx, |input, cx| input.set_value(value, window, cx));
            }
            self.protocol.update(cx, |s, cx| {
                s.set_selected_index(Some(IndexPath::new(d.protocol)), window, cx)
            });
            self.method.update(cx, |s, cx| {
                s.set_selected_index(Some(IndexPath::new(d.method)), window, cx)
            });
        }
    }
    pub fn open(
        &mut self,
        server: Option<SharedServer>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        self.editing = server.is_some();
        self.draft = match server {
            Some(s) => Some(Draft::from_server(&s)),
            None => match Draft::new() {
                Ok(d) => Some(d),
                Err(e) => {
                    self.error = Some(e);
                    None
                }
            },
        };
        self.error = None;
        self.generation += 1;
        self.set_inputs(window, cx);
        self.ensure_certificate(cx);
        cx.notify();
    }
    fn ensure_certificate(&mut self, cx: &mut Context<Self>) {
        self.certificate_busy = false;
        if !self
            .draft
            .as_ref()
            .is_some_and(|d| d.needs_tls() && d.tls.is_none())
        {
            return;
        }
        self.certificate_busy = true;
        let generation = self.generation;
        let receiver = self.services.shared_certificate();
        cx.spawn(async move |view, cx| {
            let result = receiver.await.unwrap_or(Err("证书生成失败，请重试"));
            let _ = view.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.certificate_busy = false;
                match result {
                    Ok(tls) => {
                        if let Some(d) = &mut this.draft {
                            d.tls = Some(tls);
                        }
                    }
                    Err(e) => this.error = Some(e),
                }
                cx.notify();
            });
        })
        .detach();
    }
    fn send(&mut self, command: Command, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        let read = matches!(command, Command::Read);
        let saving = matches!(command, Command::Save(..));
        let expected = self.state.as_ref().map(AppState::config_version);
        let receiver = self.services.shared_network_command(expected, command);
        self.busy = true;
        if !read {
            self.error = None;
        }
        cx.spawn(async move |view, cx| {
            if let Ok(completion) = receiver.await {
                let _ = view.update(cx, |this, cx| {
                    this.busy = false;
                    if let Some(s) = completion.snapshot {
                        this.state = Some(s.clone());
                        cx.emit(Updated(Box::new(s)));
                    }
                    match completion.result {
                        Ok(state) => {
                            this.state = Some(state.clone());
                            cx.emit(Updated(Box::new(state)));
                            if !read {
                                this.error = None;
                                this.deleting = None;
                                if saving {
                                    this.draft = None;
                                    this.generation += 1;
                                }
                                components::notice::notify_app(
                                    components::notice::Notice::Success,
                                    "已保存，待应用",
                                    cx,
                                );
                            }
                        }
                        Err(e) => {
                            this.error = Some(e);
                            components::notice::notify_app(
                                components::notice::Notice::Error,
                                e,
                                cx,
                            );
                        }
                    }
                    cx.notify();
                });
            } else {
                let _ = view.update(cx, |this, cx| {
                    this.busy = false;
                    this.error = Some("数据未能保存或读取");
                    cx.notify();
                });
            }
        })
        .detach();
        cx.notify();
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        self.sync_draft(cx);
        if let Some(d) = self.draft.clone() {
            self.send(Command::Save(Box::new(d), self.editing), cx);
        }
    }
    fn copy(value: String, cx: &mut App) {
        if !value.is_empty() {
            cx.write_to_clipboard(ClipboardItem::new_string(value));
            components::notice::notify_app(components::notice::Notice::Success, "复制成功", cx);
        }
    }
    fn close(&mut self, cx: &mut Context<Self>) {
        self.draft = None;
        self.deleting = None;
        self.sharing = None;
        self.generation += 1;
        cx.notify();
    }
    fn field(&self, label: &'static str, index: usize, cx: &App) -> Div {
        let mono = [UUID, USER, PASSWORD, OBFS, LINK].contains(&index);
        let mut input = text_input(&self.inputs[index])
            .disabled(self.busy)
            .bg(cx.theme().input)
            .border_color(cx.theme().foreground.opacity(0.2))
            .text_color(cx.theme().foreground)
            .rounded(px(t::RADIUS));
        if mono {
            input = input.font_family("Menlo");
        }
        if index == PORT {
            input = input.numeric();
        }
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(t::ROW_GAP))
            .child(
                div()
                    .text_size(px(st::LABEL))
                    .line_height(px(st::LABEL_LINE))
                    .font_weight(super::theme::MISANS_MEDIUM)
                    .child(tr(cx, label)),
            )
            .child(input)
    }
    fn credential(&self, label: &'static str, index: usize, cx: &mut Context<Self>) -> Div {
        let input = text_input(&self.inputs[index])
            .disabled(self.busy)
            .font_family("Menlo")
            .bg(cx.theme().input)
            .border_color(cx.theme().foreground.opacity(0.2))
            .rounded_r(px(0.));
        // 与 React server-join 共用一条32px输入行；按钮仅替换当前协议适用凭据。
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(px(t::ROW_GAP))
            .child(
                div()
                    .text_size(px(st::LABEL))
                    .line_height(px(st::LABEL_LINE))
                    .font_weight(super::theme::MISANS_MEDIUM)
                    .child(tr(cx, label)),
            )
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .child(div().flex_1().min_w_0().child(input))
                    .child(
                        components::icon_button(
                            ("server-generate", index),
                            tr(cx, "随机生成"),
                            "ArrowPath",
                            st::JOIN_BUTTON,
                        )
                        .h(px(t::CONTROL))
                        .bg(cx.theme().button)
                        .rounded_l(px(0.))
                        .disabled(self.busy)
                        .on_click(cx.listener(
                            move |this, _, window, cx| {
                                let result = if index == UUID {
                                    random_uuid()
                                } else {
                                    random_secret(if index == PASSWORD {
                                        this.draft.as_ref().map_or(0, |d| {
                                            if d.protocol == 0 { d.method } else { 0 }
                                        })
                                    } else {
                                        0
                                    })
                                };
                                match result {
                                    Ok(value) => this.inputs[index]
                                        .update(cx, |s, cx| s.set_value(value, window, cx)),
                                    Err(e) => this.error = Some(e),
                                }
                                cx.notify();
                            },
                        )),
                    ),
            )
    }
    fn error_view(&self, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .items_center()
            .gap(px(t::GAP))
            .when_some(self.error, |d, e| {
                d.child(tr(cx, e)).child(
                    button("server-refresh", tr(cx, "刷新"))
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.load(cx))),
                )
            })
    }
    fn link(&self, value: String, cx: &mut Context<Self>) -> Div {
        div()
            .flex()
            .min_w_0()
            .child(
                div().flex_1().min_w_0().child(
                    text_input(&self.inputs[LINK])
                        .readonly(true)
                        .font_family("Menlo")
                        .text_size(px(t::BODY))
                        .rounded_r(px(0.)),
                ),
            )
            .child(
                components::icon_button(
                    "server-copy-link",
                    tr(cx, "复制链接"),
                    "ClipboardDocument",
                    st::JOIN_BUTTON,
                )
                .h(px(t::CONTROL))
                .bg(cx.theme().button)
                .rounded_l(px(0.))
                .disabled(value.is_empty())
                .on_click(move |_, _, cx| Self::copy(value.clone(), cx)),
            )
    }
    fn qr(value: &str, extent: f32) -> Div {
        div()
            .size(px(extent))
            .p(px(st::QR_PAD))
            .bg(rgb(0xffffff))
            .rounded(px(st::QR_RADIUS))
            .child(components::qr::qr_code(value, extent - st::QR_PAD * 2.))
    }
    fn editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        self.sync_draft(cx);
        let draft = self.draft.as_ref().expect("open draft").clone();
        for (index, key) in [
            (NAME, "例如:家里"),
            (ADDRESS, "域名或 IP"),
            (USER, "可选，留空则不需要认证"),
            (
                PASSWORD,
                if draft.protocol == 4 {
                    "可选，留空则不需要认证"
                } else {
                    ""
                },
            ),
            (OBFS, "可选（salamander）"),
        ] {
            self.inputs[index].update(cx, |s, cx| s.set_placeholder(tr(cx, key), window, cx));
        }

        let mut preview = draft.clone();
        if preview.name.trim().is_empty() {
            preview.name = preview.id.clone();
        }
        let link = preview
            .server()
            .map(|s| shared_server_uri(&s))
            .unwrap_or_default();
        self.inputs[LINK].update(cx, |s, cx| {
            s.set_value(
                if link.is_empty() {
                    tr(cx, "填了连接地址才会生成分享链接").to_owned()
                } else {
                    link.clone()
                },
                window,
                cx,
            )
        });
        let field_width = (px(st::EDIT_WIDTH)
            .min(window.viewport_size().width - px(t::SECTION_PADDING * 2.))
            - px(t::SECTION_PADDING * 3.))
            / 2.;
        let select_field = |label: &'static str, entity: &Entity<SelectState>, cx: &App| {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(t::ROW_GAP))
                .child(
                    div()
                        .text_size(px(st::LABEL))
                        .line_height(px(st::LABEL_LINE))
                        .child(tr(cx, label)),
                )
                .child(
                    Select::new(entity, label)
                        .w(field_width)
                        .disabled(self.busy),
                )
        };
        let row = || div().flex().gap(px(t::SECTION_PADDING)).w_full();
        let mut form = div()
            .flex()
            .flex_col()
            .gap(px(t::SECTION_PADDING))
            .child(
                row()
                    .child(self.field("备注", NAME, cx))
                    .child(select_field("协议", &self.protocol, cx)),
            )
            .child(
                row()
                    .child(self.field("域名 / IP", ADDRESS, cx))
                    .child(self.field("端口", PORT, cx)),
            );
        let mut credentials = row();
        match draft.protocol {
            0 => {
                credentials = credentials
                    .child(select_field("加密", &self.method, cx))
                    .child(self.credential("密码", PASSWORD, cx))
            }
            1 => {
                let label = if draft
                    .tls
                    .as_ref()
                    .is_some_and(|t| t.source == SharedCertificateSource::LocalPem)
                {
                    "TLS（已导入证书）"
                } else {
                    "TLS（自签证书）"
                };
                credentials = credentials.child(self.credential("UUID", UUID, cx)).child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(t::PAD))
                        .child(
                            components::toggle("server-tls", draft.use_tls, tr(cx, label))
                                .disabled(self.busy)
                                .on_change(cx.listener(|this, checked, _, cx| {
                                    if let Some(d) = &mut this.draft {
                                        d.use_tls = *checked;
                                    }
                                    this.generation += 1;
                                    this.ensure_certificate(cx);
                                    cx.notify();
                                })),
                        )
                        .child(tr(cx, label)),
                );
            }
            2 => {
                credentials = credentials
                    .child(self.credential("UUID", UUID, cx))
                    .child(self.credential("密码", PASSWORD, cx))
            }
            3 => {
                credentials = credentials
                    .child(self.credential("密码", PASSWORD, cx))
                    .child(self.credential("混淆密码", OBFS, cx))
            }
            _ => {
                credentials = credentials
                    .child(self.field("用户名", USER, cx))
                    .child(self.credential("密码", PASSWORD, cx))
            }
        };
        form = form.child(credentials);
        let hint = if draft.needs_tls() {
            if draft
                .tls
                .as_ref()
                .is_some_and(|t| t.source == SharedCertificateSource::LocalPem)
            {
                "使用已导入证书，客户端须信任该证书。"
            } else {
                "使用自签证书，分享链接已允许不安全证书。"
            }
        } else if draft.protocol == 4 {
            "SOCKS5 和 HTTP 代理共用这个端口，设备填写本机地址和端口即可使用。用户名和密码同时留空时不需要认证。"
        } else {
            "不套 TLS：客户端与服务器之间只有协议本身的加密。"
        };
        form = form
            .child(
                div()
                    .text_size(px(st::LABEL))
                    .line_height(px(st::LABEL_LINE))
                    .text_color(cx.theme().foreground.opacity(0.6))
                    .child(tr(cx, hint)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(t::ROW_GAP))
                    .child(
                        div()
                            .text_size(px(st::LABEL))
                            .line_height(px(st::LABEL_LINE))
                            .child(tr(cx, "分享链接")),
                    )
                    .child(self.link(link.clone(), cx)),
            )
            .when(!link.is_empty(), |d| {
                d.child(
                    div()
                        .flex()
                        .justify_center()
                        .child(Self::qr(&link, st::EDIT_QR)),
                )
            })
            .when(self.error.is_some(), |d| d.child(self.error_view(cx)));
        let footer = div()
            .p(px(t::GAP))
            .px(px(t::SECTION_PADDING))
            .border_t_1()
            .border_color(cx.theme().foreground.opacity(0.1))
            .flex()
            .justify_end()
            .gap(px(t::GAP))
            .child(
                button("server-cancel", tr(cx, "取消"))
                    .disabled(self.busy)
                    .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
            )
            .child(
                button(
                    "server-save",
                    tr(
                        cx,
                        if self.busy {
                            "保存中"
                        } else if self.certificate_busy {
                            "生成中"
                        } else {
                            "保存"
                        },
                    ),
                )
                .primary()
                .disabled(self.busy || self.certificate_busy)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.ensure_certificate(cx);
                    if !this.certificate_busy {
                        this.save(cx);
                    }
                })),
            );
        self.dialog(
            if self.editing {
                "编辑服务器"
            } else {
                "添加服务器"
            },
            st::EDIT_WIDTH,
            form,
            Some(footer),
            window,
            cx,
        )
    }
    fn dialog(
        &self,
        title: &'static str,
        width: f32,
        body: Div,
        footer: Option<Div>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let surface = div()
            .w(px(width))
            .max_w(window.viewport_size().width - px(t::SECTION_PADDING * 2.))
            .max_h(window.viewport_size().height - px(t::SECTION_PADDING * 2.))
            .flex()
            .flex_col()
            .rounded(px(st::MODAL_RADIUS))
            .bg(if cx.theme().mode.is_dark() {
                cx.theme().popover
            } else {
                rgba(t::SUBSCRIPTION_SHARE_SURFACE).into()
            })
            .text_color(cx.theme().foreground)
            .overflow_hidden()
            .child(
                div()
                    .h(px(st::HEADER))
                    .flex_shrink_0()
                    .px(px(t::SECTION_PADDING))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().foreground.opacity(0.1))
                    .child(
                        div()
                            .text_size(px(t::SECTION_TITLE))
                            .font_weight(FontWeight::BOLD)
                            .child(if title == "扫码连接" {
                                format!(
                                    "{}「{}」",
                                    tr(cx, title),
                                    self.sharing.as_ref().map_or("", |s| s.name.as_str())
                                )
                            } else {
                                tr(cx, title).to_owned()
                            }),
                    )
                    .child(
                        components::icon_button(
                            "server-close",
                            tr(cx, "关闭"),
                            "XMark",
                            t::CONTROL,
                        )
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                    ),
            )
            .child(
                div()
                    .id("server-modal-body")
                    .track_scroll(&self.modal_scroll)
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(t::SECTION_PADDING))
                    .child(body),
            )
            .children(footer);
        gpui_kit::base::Dialog::new(cx)
            .open(true)
            .close_on_escape(!self.busy)
            .close_on_backdrop_press(!self.busy)
            .on_ok(|_, _, _| false)
            .backdrop(div().size_full().bg(cx.theme().overlay))
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|this, _, _, cx| this.close(cx)))
            .into_any_element()
    }
}
impl Render for SharedNetworkView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div()
            .id("server-list")
            .track_scroll(&self.scroll)
            .size_full()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(t::GAP))
            .font_weight(super::theme::MISANS_REGULAR)
            .text_size(px(t::BODY))
            .when(self.error.is_some(), |d| d.child(self.error_view(cx)));
        if self.busy || self.state.is_none() {
            body = body.child(div().p(px(t::SECTION_PADDING)).child(tr(cx, "加载中")));
        }
        if let Some(state) = &self.state {
            if state.app_config.shared_servers.is_empty() && !self.busy {
                body = body.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .gap(px(t::PAD))
                        .py(px(st::EMPTY_PAD))
                        .text_color(cx.theme().muted_foreground)
                        .child(icon("Share", st::EMPTY_ICON))
                        .child(tr(cx, "还没有服务器。"))
                        .child(
                            button("server-empty-add", "")
                                .accessibility_label(tr(cx, "添加服务器"))
                                .child(icon("Plus", t::ICON_SMALL))
                                .child(tr(cx, "添加服务器"))
                                .primary()
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.open(None, window, cx)),
                                ),
                        ),
                );
            }
            for (index, server) in state.app_config.shared_servers.iter().enumerate() {
                let editing = server.clone();
                let id = server.id.clone();
                let enabled = server.enabled;
                let copy = shared_server_uri(server);
                let sharing = server.clone();
                let deleting = server.clone();
                let version = state.config_version();
                let protocol = Draft::from_server(server).protocol;
                body = body.child(
                    div()
                        .id(("server-card", index))
                        .flex()
                        .items_center()
                        .gap(px(t::GAP))
                        .p(px(st::CARD_PAD))
                        .rounded(px(st::CARD_RADIUS))
                        .bg(cx.theme().popover)
                        .border_1()
                        .border_color(cx.theme().foreground.opacity(0.1))
                        .opacity(if enabled { 1. } else { 0.5 })
                        .on_drag(
                            ServerDrag {
                                index,
                                name: server.name.clone(),
                                version: version.clone(),
                            },
                            |drag, _, _, cx| cx.new(|_| drag.clone()),
                        )
                        .on_drop(cx.listener(move |this, drag: &ServerDrag, _, cx| {
                            if let Some(state) = &this.state
                                && !this.busy
                                && state.config_version() == drag.version
                                && drag.index < state.app_config.shared_servers.len()
                            {
                                let mut ids = state
                                    .app_config
                                    .shared_servers
                                    .iter()
                                    .map(|s| s.id.clone())
                                    .collect::<Vec<_>>();
                                let id = ids.remove(drag.index);
                                ids.insert(index, id);
                                this.send(
                                    Command::Edit(Box::new(SharedServerCommand::Reorder(ids))),
                                    cx,
                                );
                            }
                        }))
                        .child(icon("Bars3", t::ICON_SMALL).text_color(cx.theme().muted_foreground))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(4.))
                                .child(
                                    div()
                                        .text_size(px(t::SECTION_TITLE))
                                        .line_height(px(t::SECTION_LINE))
                                        .child(server.name.clone()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap_x(px(12.))
                                        .text_size(px(st::LABEL))
                                        .line_height(px(st::LABEL_LINE))
                                        .text_color(cx.theme().foreground.opacity(0.6))
                                        .child(PROTOCOLS[protocol])
                                        .child(format!("{} {}", tr(cx, "端口"), server.port))
                                        .child(server.address.clone()),
                                ),
                        )
                        .child(
                            components::icon_button(
                                ("server-scan", index),
                                tr(cx, "扫码"),
                                "QrCode",
                                t::CONTROL,
                            )
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.sharing = Some(sharing.clone());
                                    cx.notify();
                                },
                            )),
                        )
                        .child(
                            components::icon_button(
                                ("server-copy", index),
                                tr(cx, "复制链接"),
                                "ClipboardDocument",
                                t::CONTROL,
                            )
                            .on_click(move |_, _, cx| Self::copy(copy.clone(), cx)),
                        )
                        .child(
                            components::icon_button(
                                ("server-power", index),
                                tr(cx, if enabled { "停用" } else { "启用" }),
                                "Power",
                                t::CONTROL,
                            )
                            .when(enabled, |b| b.text_color(rgb(t::ACCENT_STRONG)))
                            .disabled(self.busy)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.send(
                                        Command::Edit(Box::new(SharedServerCommand::SetEnabled {
                                            id: id.clone(),
                                            enabled: !enabled,
                                        })),
                                        cx,
                                    )
                                },
                            )),
                        )
                        .child(
                            components::icon_button(
                                ("server-edit", index),
                                tr(cx, "编辑服务器"),
                                "PencilSquare",
                                t::CONTROL,
                            )
                            .disabled(self.busy)
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    this.open(Some(editing.clone()), window, cx)
                                },
                            )),
                        )
                        .child(
                            components::icon_button(
                                ("server-delete", index),
                                tr(cx, "删除服务器"),
                                "Trash",
                                t::CONTROL,
                            )
                            .disabled(self.busy)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    this.deleting = Some(deleting.clone());
                                    cx.notify();
                                },
                            )),
                        ),
                );
            }
        }
        let mut layout = div().size_full().min_h_0().child(body);
        if self.draft.is_some() {
            layout = layout.child(self.editor(window, cx));
        } else if let Some(server) = self.deleting.clone() {
            let id = server.id.clone();
            let footer = div()
                .p(px(t::GAP))
                .flex()
                .justify_end()
                .gap(px(t::GAP))
                .child(
                    button("server-delete-cancel", tr(cx, "取消"))
                        .disabled(self.busy)
                        .on_click(cx.listener(|this, _, _, cx| this.close(cx))),
                )
                .child(
                    button("server-delete-confirm", tr(cx, "确定"))
                        .disabled(self.busy)
                        .bg(rgb(t::SUBSCRIPTION_SHARE_CONFIRM_ERROR))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.send(
                                Command::Edit(Box::new(SharedServerCommand::Delete {
                                    id: id.clone(),
                                })),
                                cx,
                            )
                        })),
                );
            let text = tr(
                cx,
                "确定删除共享服务器「{name}」吗？使用它的客户端将无法连接。",
            )
            .replace("{name}", &server.name);
            layout = layout.child(self.dialog(
                "确定",
                t::DIALOG_WIDTH,
                div().child(text).child(self.error_view(cx)),
                Some(footer),
                window,
                cx,
            ));
        } else if let Some(server) = self.sharing.clone() {
            let link = shared_server_uri(&server);
            self.inputs[LINK].update(cx, |s, cx| s.set_value(link.clone(), window, cx));
            let content = div()
                .flex()
                .flex_col()
                .gap(px(t::PAD))
                .child(
                    div()
                        .flex()
                        .justify_center()
                        .child(Self::qr(&link, st::CODE_QR)),
                )
                .child(
                    div()
                        .text_size(px(st::LABEL))
                        .text_center()
                        .line_height(px(st::LABEL_LINE))
                        .child(tr(cx, "普通节点链接，给其他客户端用。")),
                )
                .child(self.link(link, cx))
                .when(!server.enabled, |d| {
                    d.child(tr(cx, "这台服务器已停用，配置尚未应用。"))
                })
                .when(shared_address_is_local(&server.address), |d| {
                    d.child(
                        div()
                            .text_size(px(st::LABEL))
                            .text_color(rgb(t::backend::TRANSITION_LIGHT))
                            .child(tr(
                                cx,
                                "此地址只适用于本机或局域网，外网连接请填写公网 IP 或域名。",
                            )),
                    )
                });
            layout =
                layout.child(self.dialog("扫码连接", st::CODE_WIDTH, content, None, window, cx));
        }
        layout
    }
}
