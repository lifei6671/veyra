//! BackendSettings：五个 Profile 字段保存与 Runtime 操作分离，其余保留只读视觉槽位。
use super::{
    components::{button, surface, text_input},
    i18n::tr,
    icons::icon,
    theme::Palette,
    tokens::backend as t,
};
use crate::backend_profile::{Change, Field, Request, Saves};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::{
    component::{ActiveTheme, Disableable, Sizable},
    prelude::*,
    *,
};
use veyra_core::application::{
    manual_runtime::{ManualRuntimeSnapshot, RuntimeCommand, RuntimeError, RuntimeResult},
    runtime_snapshot::RuntimeStatus,
};
pub struct BackendView {
    pub snapshot: Option<ManualRuntimeSnapshot>,
    pub operation: Option<RuntimeCommand>,
    /// 权威值来自 AppState，排队期间只叠加当前字段的视觉草稿。
    pub profile: Option<veyra_core::domain::Profile>,
    pub saves: Saves,
    urls: [Entity<InputState>; 2],
    dirty: [bool; 2],
    syncing: bool,
    visible: bool,
    _inputs: Vec<Subscription>,
    brand: Option<std::sync::Arc<Image>>,
}
impl EventEmitter<RuntimeCommand> for BackendView {}
impl EventEmitter<Request> for BackendView {}
impl BackendView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let urls = [
            cx.new(|cx| InputState::new(window, cx)),
            cx.new(|cx| InputState::new(window, cx)),
        ];
        let subscriptions = urls
            .iter()
            .enumerate()
            .map(|(index, input)| {
                cx.subscribe_in(
                    input,
                    window,
                    move |this, input, event: &InputEvent, window, cx| {
                        if this.syncing || !this.visible {
                            return;
                        }
                        match event {
                            InputEvent::Change => {
                                this.dirty[index] = this.profile.as_ref().is_some_and(|p| {
                                    input.read(cx).value().as_str()
                                        != if index == 0 {
                                            p.test_url.as_str()
                                        } else {
                                            p.direct_test_url.as_str()
                                        }
                                });
                            }
                            InputEvent::PressEnter { .. } | InputEvent::Blur => {
                                let value = input.read(cx).value().to_string();
                                let current = this.profile.as_ref().map(|p| {
                                    if index == 0 {
                                        p.test_url.as_str()
                                    } else {
                                        p.direct_test_url.as_str()
                                    }
                                });
                                if current == Some(value.as_str()) {
                                    this.dirty[index] = false;
                                    return;
                                }
                                if !this.dirty[index] {
                                    return;
                                }
                                let change = if index == 0 {
                                    Change::TestUrl(value)
                                } else {
                                    Change::DirectTestUrl(value)
                                };
                                if change.patch().is_err() {
                                    super::components::notify(
                                        super::components::Notice::Error,
                                        "请输入有效的 HTTP / HTTPS 测速地址",
                                        window,
                                        cx,
                                    );
                                    return;
                                }
                                this.submit(change, cx);
                            }
                            _ => {}
                        }
                    },
                )
            })
            .collect();
        Self {
            snapshot: None,
            operation: None,
            profile: None,
            saves: Saves::default(),
            urls,
            dirty: [false; 2],
            syncing: false,
            visible: false,
            _inputs: subscriptions,
            brand: None,
        }
    }
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
        if self.visible != visible {
            self.saves.leave();
            // 离页只撤销请求归属；未提交或失败的 URL 草稿不由迟到完成覆盖。
        }
        self.visible = visible;
        cx.notify();
    }
    pub fn project_profile(
        &mut self,
        state: &veyra_core::domain::AppState,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let replacement = self
            .saves
            .state
            .as_ref()
            .is_some_and(|s| s.state_epoch != state.state_epoch);
        if replacement {
            self.dirty = [false; 2];
        }
        self.saves.project(state);
        self.profile = Some(state.profile.clone());
        self.syncing = true;
        for (index, value) in [
            state.profile.test_url.as_str(),
            state.profile.direct_test_url.as_str(),
        ]
        .into_iter()
        .enumerate()
        {
            // 未提交和失败的 URL 草稿保留；轮询不会覆盖输入中的文字。
            if !self.dirty[index] && self.urls[index].read(cx).value().as_str() != value {
                self.urls[index].update(cx, |input, cx| {
                    input.set_value(value.to_owned(), window, cx)
                });
            }
        }
        self.syncing = false;
        cx.notify();
    }
    fn submit(&mut self, change: Change, cx: &mut Context<Self>) {
        if let Some(request) = self.saves.enqueue(change) {
            cx.emit(request);
        }
        cx.notify();
    }
    pub fn complete(&mut self, request: &Request, success: bool, cx: &mut Context<Self>) -> bool {
        if !self.saves.accepts(request) {
            return false;
        }
        if success {
            match request.change.field() {
                Field::TestUrl => self.dirty[0] = false,
                Field::DirectTestUrl => self.dirty[1] = false,
                _ => {}
            }
        }
        if let Some(next) = self.saves.complete(request, success) {
            cx.emit(next);
        }
        cx.notify();
        true
    }
    fn profile_switch(
        &self,
        id: &'static str,
        value: Option<bool>,
        field: Field,
        p: Palette,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let busy = self.saves.busy(field);
        // 复用已验收的 track/thumb 几何；只有本字段正在保存时遮盖为 spinner。
        readonly_switch(id, value, p, cx)
            .role(Role::Switch)
            .aria_description(tr(cx, "重启内核后生效"))
            .cursor_pointer()
            .relative()
            .when(busy, |d| {
                d.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgb(p.surface))
                        .rounded_full()
                        .child(gpui_kit::component::spinner::Spinner::new().with_size(px(t::ICON))),
                )
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if this.saves.busy(field) {
                    return;
                }
                let value = !value.unwrap_or(false);
                let change = match field {
                    Field::DirectForNodes => Change::DirectForNodes(value),
                    Field::RejectQuic => Change::RejectQuic(value),
                    Field::Ipv6 => Change::Ipv6(value),
                    _ => unreachable!(),
                };
                this.submit(change, cx);
            }))
    }
}
// Backend 与 sidebar 共享相同的操作归属判断，不能把全局 Busy 显示成每个按钮加载。
pub fn action_is_busy(operation: Option<RuntimeCommand>, command: RuntimeCommand) -> bool {
    operation == Some(command)
}
fn status_tone(status: Option<RuntimeStatus>, p: Palette, dark: bool) -> (u32, u32) {
    match status {
        Some(RuntimeStatus::Ready) => ((t::ON << 8) | 0xff, t::ON_BG),
        Some(RuntimeStatus::Failed) => ((t::OFF << 8) | 0xff, t::OFF_BG),
        Some(RuntimeStatus::Starting | RuntimeStatus::Recovering) => {
            let color = if dark {
                t::TRANSITION_DARK
            } else {
                t::TRANSITION_LIGHT
            };
            ((color << 8) | 0xff, (color << 8) | t::TRANSITION_ALPHA)
        }
        _ => (p.muted, (p.field << 8) | 0xff),
    }
}
pub fn status(snapshot: Option<&ManualRuntimeSnapshot>) -> &'static str {
    match snapshot.map(|s| s.runtime.status) {
        Some(RuntimeStatus::Stopped) => "未运行",
        Some(RuntimeStatus::Starting) => "正在启动",
        Some(RuntimeStatus::Ready) => "运行中",
        Some(RuntimeStatus::Recovering) => "恢复中",
        Some(RuntimeStatus::Failed) => "操作失败",
        None => "正在读取服务状态",
    }
}
pub fn feedback(result: Result<RuntimeResult, RuntimeError>) -> &'static str {
    match result {
        Ok(RuntimeResult::Started) => "内核已启动",
        Ok(RuntimeResult::Stopped) => "内核已停止",
        Ok(RuntimeResult::Restarted) => "内核已重启",
        Ok(RuntimeResult::AlreadyRunning) => "内核已在运行",
        Ok(RuntimeResult::Refreshed) => "服务状态已刷新",
        Err(RuntimeError::KernelUnavailable) => "固定内核不可用",
        Err(RuntimeError::StateUnavailable) => "无法读取已保存配置",
        Err(RuntimeError::CompileFailed) => "当前配置无法启动，请检查有效订阅和配置选项",
        Err(RuntimeError::CandidateFailed) => "候选内核检查或启动失败",
        Err(RuntimeError::StopFailed) => "内核清理失败，请重试停止",
        Err(RuntimeError::UnexpectedExit) => "内核意外退出",
    }
}
// 只读外观组件共用 React 的尺寸；没有回调即没有未来业务入口。
fn row() -> Div {
    div().flex().items_center().gap(px(t::ACTION_GAP))
}
fn hint(text: impl Into<SharedString>, p: Palette) -> Div {
    div()
        .text_size(px(t::HINT))
        .line_height(px(t::HINT_LINE))
        .text_color(rgba(p.muted))
        .child(text.into())
}
fn heading(text: impl Into<SharedString>) -> Div {
    div()
        .text_size(px(t::TITLE))
        .line_height(px(t::TITLE_LINE))
        .font_weight(super::theme::MISANS_SEMIBOLD)
        .child(text.into())
}
fn section(id: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .flex()
        .flex_col()
        .flex_shrink_0()
        .p(px(t::PADDING))
        .gap(px(t::GAP))
}
fn divider(line: u32) -> Div {
    div().h(px(1.)).flex_shrink_0().bg(rgba(line))
}
// 静态只读元素没有 action/focus/click；不借用 Kit Button 的 disabled opacity。
fn shell_button(
    id: &'static str,
    text: impl Into<SharedString>,
    p: Palette,
    cx: &App,
) -> Stateful<Div> {
    let text = text.into();
    div()
        .id(id)
        .role(Role::Label)
        .aria_label(text.clone())
        .aria_description(tr(cx, "只读，未接入编辑"))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .h(px(t::BUTTON_HEIGHT))
        .px(px(t::BUTTON_PADDING))
        .rounded(px(t::BUTTON_RADIUS))
        .gap(px(t::BUTTON_GAP))
        .text_size(px(t::BODY))
        .line_height(px(t::LINE))
        .font_weight(super::theme::MISANS_SEMIBOLD)
        .text_color(rgb(p.text))
        .bg(rgb(if cx.theme().mode.is_dark() {
            t::BUTTON_DARK
        } else {
            t::BUTTON_LIGHT
        }))
        .when(!text.is_empty(), |d| d.child(text))
}
fn readonly_field(
    id: &'static str,
    text: impl Into<SharedString>,
    width: f32,
    select: bool,
    p: Palette,
    cx: &App,
) -> Stateful<Div> {
    let text = text.into();
    shell_button(id, "", p, cx)
        .role(Role::Label)
        .aria_label(text.clone())
        .aria_description(tr(cx, "只读，未接入编辑"))
        .w(px(width))
        .font_weight(super::theme::MISANS_REGULAR)
        .border_1()
        .border_color(rgba((p.text << 8) | t::FIELD_LINE_ALPHA))
        .bg(rgba((p.surface << 8) | t::FIELD_ALPHA))
        .child(
            row()
                .gap(px(t::FIELD_GAP))
                .w_full()
                .min_w_0()
                .justify_between()
                .child(div().flex_1().min_w_0().truncate().child(text))
                .when(select, |d| d.child(icon("ChevronDown", t::HINT))),
        )
}
fn readonly_switch(id: &'static str, value: Option<bool>, p: Palette, cx: &App) -> Stateful<Div> {
    // 未建模值仅绘制 neutral/off 轮廓；“未接入”由相邻文本表达。
    let checked = value == Some(true);
    div()
        .id(id)
        .role(Role::Label)
        .aria_description(tr(cx, "只读，未接入编辑"))
        .flex()
        .items_center()
        .flex_shrink_0()
        .w(px(super::tokens::SUBSCRIPTION_SWITCH_WIDTH))
        .h(px(super::tokens::SUBSCRIPTION_SWITCH_HEIGHT))
        .p(px(super::tokens::SUBSCRIPTION_SWITCH_PADDING))
        .rounded_full()
        .border_1()
        .border_color(rgba((p.text << 8) | t::SWITCH_BORDER_ALPHA))
        .child(
            div()
                .size(px(super::tokens::SUBSCRIPTION_SWITCH_THUMB))
                .flex_shrink_0()
                .rounded_full()
                .bg(rgba(
                    (p.text << 8) | if checked { 255 } else { t::SWITCH_BORDER_ALPHA },
                ))
                .ml(px(if checked { t::SWITCH_TRAVEL } else { 0. })),
        )
}
fn badge(label: &str, tone: (u32, u32), dark: bool) -> Div {
    div()
        .flex()
        .items_center()
        .h(px(t::BADGE_HEIGHT))
        .px(px(t::BADGE_PADDING))
        .rounded(px(t::BUTTON_RADIUS))
        .text_size(px(t::HINT))
        .line_height(px(t::HINT_LINE))
        .border_1()
        .border_color(rgba(if dark {
            t::BADGE_LINE_DARK
        } else {
            t::BADGE_LINE_LIGHT
        }))
        .text_color(rgba(tone.0))
        .bg(rgba(tone.1))
        .child(label.to_owned())
}
impl Render for BackendView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use veyra_core::domain::{Ipv6Proxy, TunStack};
        // 保持图像 identity，不随每次状态轮询重新解码品牌资源。
        let brand = self
            .brand
            .get_or_insert_with(|| {
                std::sync::Arc::new(Image::from_bytes(
                    ImageFormat::Png,
                    include_bytes!("../../../../src-tauri/icons/icon.png").to_vec(),
                ))
            })
            .clone();
        let dark = cx.theme().mode.is_dark();
        let p = Palette::new(dark, false);
        let line = if dark { t::LINE_DARK } else { t::LINE_LIGHT };
        let runtime = self.snapshot.as_ref();
        let state = runtime.map(|s| s.runtime.status);
        let ready = state == Some(RuntimeStatus::Ready);
        let mut projected = self.profile.clone();
        if let Some(profile) = &mut projected {
            for change in self.saves.changes() {
                match change {
                    Change::DirectForNodes(v) => profile.direct_for_nodes = *v,
                    Change::RejectQuic(v) => profile.reject_quic = *v,
                    Change::Ipv6(v) => profile.ipv6 = *v,
                    _ => {}
                }
            }
        }
        let profile = projected.as_ref();
        let card = || {
            surface(dark, px(t::RADIUS))
                .bg(rgb(p.surface))
                .border_1()
                .border_color(rgba(line))
                .flex_shrink_0()
                .overflow_hidden()
        };
        let link = |label: &str| {
            row()
                .gap(px(t::BUTTON_GAP))
                .text_size(px(t::BODY))
                .line_height(px(t::LINE))
                .text_color(rgba(p.muted))
                .child(icon("Github", t::ICON))
                .child(label.to_owned())
                .child(icon("ArrowTopRightOnSquare", t::ICON))
        };
        let kernel = runtime
            .filter(|s| s.kernel_available)
            .map(|_| "sing-box version 1.14.0")
            .unwrap_or("—");
        let controller = match state {
            Some(RuntimeStatus::Ready) if runtime.is_some_and(|s| s.endpoints.is_some()) => "就绪",
            Some(RuntimeStatus::Starting | RuntimeStatus::Recovering) => "等待",
            Some(RuntimeStatus::Stopped) => "未运行",
            _ => "不可用",
        };
        let metadata = runtime
            .map(|s| {
                format!(
                    "{} {} · {} {}",
                    tr(cx, "保存版本"),
                    s.runtime.saved_version.0.revision,
                    tr(cx, "运行版本"),
                    s.runtime
                        .applied_version
                        .as_ref()
                        .map(|v| v.0.revision.to_string())
                        .unwrap_or("—".into())
                )
            })
            .unwrap_or_default();
        let stale = runtime.is_some_and(|s| {
            s.runtime
                .applied_version
                .as_ref()
                .is_some_and(|v| v != &s.runtime.saved_version)
        });
        let service = card()
            .id("backend-service-card")
            .p(px(t::PADDING))
            .gap(px(t::GAP))
            .child(
                row()
                    .id("backend-brand")
                    .h(px(t::BUTTON_HEIGHT))
                    .justify_between()
                    .child(
                        row()
                            .text_size(px(t::TITLE))
                            .line_height(px(t::TITLE_LINE))
                            .child(img(brand).size(px(t::TITLE_LINE)))
                            .child(
                                div()
                                    .font_weight(super::theme::MISANS_SEMIBOLD)
                                    .child("VEYRA"),
                            )
                            .child(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .child(link("GitHub")),
                    )
                    .child(
                        shell_button("backend-reset-unavailable", "", p, cx)
                            .w(px(t::BUTTON_HEIGHT))
                            .p_0()
                            .bg(rgb(p.surface))
                            .aria_label(tr(cx, "恢复出厂设置：未接入"))
                            .child(icon("ExclamationTriangle", t::ICON)),
                    ),
            )
            .child(
                row()
                    .id("backend-version-row")
                    .h(px(t::TITLE_LINE))
                    .text_color(rgba(p.muted))
                    .child(icon("CpuChip", t::ICON))
                    .child(format!("{}：", tr(cx, "内核版本")))
                    .child(
                        div()
                            .font_weight(super::theme::MISANS_MEDIUM)
                            .text_color(rgb(p.text))
                            .child(kernel),
                    )
                    .child(link("sing-box GitHub")),
            )
            .child(
                row()
                    .id("backend-geo-row")
                    .h(px(t::TITLE_LINE))
                    .gap(px(t::PADDING))
                    .text_color(rgba(p.muted))
                    .child(row().child(icon("Map", t::ICON)).child("GeoSite —"))
                    .child(row().child(icon("CircleStack", t::ICON)).child("GeoIP —"))
                    .child("—")
                    .child(link(tr(cx, "Geo 数据 GitHub"))),
            )
            .child(divider(line))
            .child(
                row()
                    .id("backend-status-row")
                    .h(px(t::LINE))
                    .gap(px(t::PADDING))
                    .child(row().child(tr(cx, "内核")).child(badge(
                        tr(cx, status(runtime)),
                        status_tone(state, p, dark),
                        dark,
                    )))
                    .child(row().child(tr(cx, "控制器")).child(badge(
                        tr(cx, controller),
                        status_tone(
                            if controller == "就绪" {
                                Some(RuntimeStatus::Ready)
                            } else {
                                state
                            },
                            p,
                            dark,
                        ),
                        dark,
                    )))
                    .child(row().child(tr(cx, "开机自启")).child(badge(
                        tr(cx, "未接入"),
                        status_tone(None, p, dark),
                        dark,
                    ))),
            )
            .child(
                row()
                    .id("backend-hint")
                    .h(px(t::HINT_LINE))
                    .justify_between()
                    .child(hint(tr(cx, "保存配置不会自动应用，重启内核后生效"), p))
                    .child(hint(metadata, p).when(stale, |d| {
                        d.text_color(rgba(status_tone(Some(RuntimeStatus::Starting), p, dark).0))
                    })),
            )
            .child(
                row()
                    .id("backend-service-actions")
                    .h(px(t::BUTTON_HEIGHT))
                    .children(
                        [
                            (
                                "backend-start",
                                "启动",
                                "Play",
                                RuntimeCommand::Start,
                                runtime.is_none() || ready,
                            ),
                            (
                                "backend-stop",
                                "停止",
                                "Stop",
                                RuntimeCommand::Stop,
                                runtime.is_none() || state == Some(RuntimeStatus::Stopped),
                            ),
                            (
                                "backend-restart",
                                "重启",
                                "ArrowPath",
                                RuntimeCommand::Restart,
                                !ready,
                            ),
                        ]
                        .map(|(id, label, glyph, command, disabled)| {
                            button(id, "")
                                .accessibility_label(tr(cx, label))
                                .h(px(t::BUTTON_HEIGHT))
                                .px(px(t::BUTTON_PADDING))
                                .rounded(px(t::BUTTON_RADIUS))
                                .gap(px(t::BUTTON_GAP))
                                .text_size(px(t::BODY))
                                .line_height(px(t::LINE))
                                .font_weight(super::theme::MISANS_SEMIBOLD)
                                .bg(rgb(if dark {
                                    t::BUTTON_DARK
                                } else {
                                    t::BUTTON_LIGHT
                                }))
                                .when(!dark && !disabled && self.operation.is_none(), |b| {
                                    b.reference_hover(
                                        rgb(t::BUTTON_LIGHT).into(),
                                        rgb(t::BUTTON_HOVER_LIGHT).into(),
                                        rgb(p.text).into(),
                                    )
                                })
                                .disabled(self.operation.is_some() || disabled)
                                .when(!action_is_busy(self.operation, command), |b| {
                                    b.child(icon(glyph, t::ICON))
                                })
                                .when(action_is_busy(self.operation, command), |b| {
                                    b.child(
                                        gpui_kit::component::spinner::Spinner::new()
                                            .with_size(px(t::ICON)),
                                    )
                                })
                                .child(tr(cx, label))
                                .on_click(cx.listener(move |_, _, _, cx| cx.emit(command)))
                        }),
                    ),
            )
            .child(divider(line))
            .child(
                row()
                    .id("backend-update-row")
                    .h(px(t::BUTTON_HEIGHT))
                    .child(readonly_field(
                        "backend-update-channel",
                        tr(cx, "沿用安装通道"),
                        t::CHANNEL_WIDTH,
                        true,
                        p,
                        cx,
                    ))
                    .child(shell_button(
                        "backend-update-check",
                        tr(cx, "检查更新"),
                        p,
                        cx,
                    ))
                    .child(hint(format!("{}：—", tr(cx, "安装时通道")), p)),
            )
            .child(divider(line))
            .child(
                row()
                    .id("backend-auto-update")
                    .h(px(t::LINE))
                    .gap(px(t::GAP))
                    .child(tr(cx, "自动更新"))
                    .child(readonly_switch(
                        "backend-auto-update-unavailable",
                        None,
                        p,
                        cx,
                    ))
                    .child(hint(
                        tr(
                            cx,
                            "到点先探最新版,有新版才升级;按设定的间隔探,一天最多一次。",
                        ),
                        p,
                    ))
                    .child(hint(tr(cx, "未接入"), p)),
            );
        let toggle_section = |id, title, description, value: Option<bool>, field: Option<Field>| {
            section(id).child(
                div()
                    .child(
                        row()
                            .child(heading(tr(cx, title)))
                            .child(
                                match field {
                                    Some(field) => self.profile_switch(id, value, field, p, cx),
                                    None => readonly_switch(id, value, p, cx),
                                }
                                .aria_label(tr(cx, title)),
                            )
                            .when(value.is_none(), |d| d.child(hint(tr(cx, "未接入"), p))),
                    )
                    .child(hint(tr(cx, description), p)),
            )
        };
        let grouped = card().id("backend-grouped-routing").gap_0()
            .child(toggle_section("backend-direct-nodes","订阅和节点站点直连","订阅地址和各节点的服务器地址一律直连,排在所有站点集之前。",profile.map(|s|s.direct_for_nodes),Some(Field::DirectForNodes)))
            .child(divider(line).mx(px(t::PADDING)))
            .child(toggle_section("backend-direct-bypass","直连不进内核","直连流量在入口就放走、不经过内核转发,吞吐可能翻倍;代价是这部分流量内核看不到——连接页、流量洞察、各项统计里都不会有它们。关掉后所有流量进内核,统计完整,但速度受内核转发能力限制。",None,None))
            .child(divider(line).mx(px(t::PADDING)))
            .child(toggle_section("backend-reject-quic","屏蔽 QUIC","走代理线路的 QUIC(UDP 443)直接拒绝,浏览器会自动退回 TCP。不少节点转发 UDP 很差,YouTube 等走 QUIC 反而卡;直连的站点不受影响。",profile.map(|s|s.reject_quic),Some(Field::RejectQuic)))
            .child(divider(line).mx(px(t::PADDING)))
            .child(section("backend-bypass-ports")
                .child(div().child(heading(tr(cx,"进内核前放行的端口"))).child(hint(tr(cx,"自建服务(RustDesk、端口映射出去的 NAS 服务等)用到的端口:目标端口或来源端口是这些的 TCP / UDP 流量在进内核之前就放行,和没装 Open-Box 一样,端口映射和打洞不受影响。和「前置自定义分流」里的「端口 → 直连」不同:那种已经进了内核,由内核替它重新连接,NAT 会被打乱。黑名单、白名单各存一份,只有选中的那份生效。"),p)))
                .child(row().mt(px(-t::FIELD_GAP)).child(row().gap_0().p(px(t::FIELD_GAP)).rounded(px(t::BUTTON_RADIUS)).bg(rgb(p.field)).child(shell_button("backend-ports-blacklist",tr(cx,"黑名单"),p, cx).px(px(t::ACTION_GAP)).font_weight(super::theme::MISANS_REGULAR).rounded(px(t::SEGMENT_RADIUS)).bg(rgba(0))).child(shell_button("backend-ports-whitelist",tr(cx,"白名单"),p, cx).px(px(t::ACTION_GAP)).font_weight(super::theme::MISANS_REGULAR).rounded(px(t::SEGMENT_RADIUS)).bg(rgba(0)))).child(hint(tr(cx,"未接入"),p)))
                .child(readonly_field("backend-ports-value",tr(cx,"例如 21114-21119, 2233"),t::FIELD_WIDTH,false,p, cx).w_full().font_family("Menlo").text_color(rgba((p.text << 8) | 0x80)))
                .child(hint(tr(cx,"黑名单:列出的端口不进内核,在进内核前就放行(和没装 Open-Box 一样);其余所有流量照常进内核。单个端口、范围(用 -)都行,多个用逗号隔开;留空就是全部进内核。多 WAN(mwan3)时,这些端口的连接一律走主路由表。"),p).mt(px(-t::ACTION_GAP))))
            .child(divider(line).mx(px(t::PADDING)))
            .child(section("backend-ipv6").child(div().child(row().child(heading("IPv6")).child(self.profile_switch("backend-ipv6-value",profile.map(|s|s.ipv6),Field::Ipv6,p, cx).aria_label("IPv6"))).child(hint(tr(cx,"默认关闭。关着时不解析 IPv6 域名,也不访问 IPv6 地址。"),p)))
                .child(row().pt(px(t::GAP)).border_t_1().border_color(rgba(line)).justify_between().child(div().child(tr(cx,"走代理的 IPv6")).child(hint(tr(cx, match profile.map(|s|s.ipv6_proxy) { Some(Ipv6Proxy::Ipv4) => "走代理的域名不再给 AAAA,设备改用 IPv4 连;裸 IPv6 目标要走代理时在内核里拒绝,不从 WAN 直出。直连的 IPv6 照常。", Some(Ipv6Proxy::Bypass) => "IPv6 流量不进内核,按系统路由直接从 WAN 出去（和 OpenClash 默认一样,test-ipv6 能过）:直连的 IPv6 照常;走代理的域名仍会解析出 AAAA,设备会先试 IPv6 直连、不通再退回 IPv4 走代理。", _ => "IPv6 目标和 IPv4 一样交给选中的节点;节点不支持 IPv6 时会失败。直连的 IPv6 照常。" }),p)))
                    .child(readonly_field("backend-ipv6-policy",profile.map(|s|tr(cx,match s.ipv6_proxy{Ipv6Proxy::Node=>"交给节点",Ipv6Proxy::Ipv4=>"降为 IPv4",Ipv6Proxy::Bypass=>"不进内核,直连放行"})).unwrap_or("—"),t::IPV6_FIELD_WIDTH,true,p, cx))))
            .child(divider(line).mx(px(t::PADDING)))
            .child(section("backend-tun")
                .child(div().child(heading(tr(cx,"TUN 参数"))).child(hint(tr(cx,"内核 tun 接口的几个底层参数,一般不用改。"),p)))
                .child(row().child(div().w(px(t::TUN_LABEL_WIDTH)).child(tr(cx,"协议栈"))).child(readonly_field("backend-tun-stack",profile.map(|s|match s.tun.stack{TunStack::Mixed=>"mixed",TunStack::System=>"system",TunStack::Gvisor=>"gvisor"}).unwrap_or("—"),t::TUN_SELECT_WIDTH,true,p, cx)))
                .child(hint(tr(cx,"mixed(默认):TCP 走系统内核栈、其余走用户态,吞吐最高。gvisor:全部走用户态栈,绕开内核的 DNAT 路径——MT6000 这类开着硬件流量卸载的机器,有线口直连大文件损坏时选它。system:全部走系统栈。"),p).mt(px(-t::FIELD_GAP)))
                .child(row().child(div().w(px(t::TUN_LABEL_WIDTH)).child("MTU")).child(readonly_field("backend-tun-mtu",profile.map(|s|s.tun.mtu.to_string()).unwrap_or("—".into()),t::TUN_LABEL_WIDTH,false,p, cx)))
                .child(hint(tr(cx,"0 = 内核默认(9000);范围 1280–65535。"),p).mt(px(-t::FIELD_GAP)))
                .child(row().child(div().w(px(t::TUN_LABEL_WIDTH)).child(tr(cx,"出站 TCP MSS"))).child(readonly_field("backend-tun-mss",profile.map(|s|s.tun.tcp_mss.to_string()).unwrap_or("—".into()),t::TUN_LABEL_WIDTH,false,p, cx)))
                .child(hint(tr(cx,"内核自己发出的 TCP 连接(走代理和直连的都算)在 SYN 上钳制 MSS;0 = 不钳制。上游链路 MTU 偏小(PPPoE、隧道)、大包卡住时可试 1400 左右;局域网转发的流量由系统防火墙的 MSS 钳制管。"),p).mt(px(-t::FIELD_GAP))));
        let diagnostic = card()
            .id("backend-grouped-diagnostics")
            .gap_0()
            .child(
                section("backend-test-urls")
                    .child(div().child(heading(tr(cx, "测速地址"))).child(hint(
                        tr(cx, "自动择优组和代理页的延迟测试用这两个地址;每个自动择优组也可以单独指定。"),
                        p,
                    )))
                    .child(
                        row().items_start().gap(px(t::GAP)).children(
                            [
                                (
                                    "backend-test-url",
                                    "测速地址",
                                    0usize,
                                ),
                                (
                                    "backend-direct-test-url",
                                    "直连测速地址",
                                    1usize,
                                ),
                            ]
                            .map(|(id, label, index)| {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap(px(t::FIELD_GAP))
                                    .child(hint(tr(cx, label), p).font_weight(super::theme::MISANS_MEDIUM))
                                    .child(
                                        text_input(&self.urls[index])
                                        .disabled(self.saves.busy(if index==0 {Field::TestUrl}else{Field::DirectTestUrl}))
                                        .w_full().h(px(t::BUTTON_HEIGHT)).px(px(t::BUTTON_PADDING)).py_0()
                                        .rounded(px(t::BUTTON_RADIUS)).border_color(rgba((p.text<<8)|t::FIELD_LINE_ALPHA))
                                        .bg(rgba((p.surface<<8)|t::FIELD_ALPHA)).text_color(rgb(p.text))
                                        .text_size(px(t::BODY)).line_height(px(t::LINE))
                                        .font_family("Menlo"),
                                    )
                                    .child(hint(tr(cx, if id == "backend-test-url" { "自动择优、手动测速和故障转移使用；支持 HTTP / HTTPS，默认使用 HTTP。" } else { "只给内置直连用；支持 HTTP / HTTPS，默认使用 HTTP。" }), p))
                            }),
                        ),
                    ),
            )
            .child(divider(line).mx(px(t::PADDING)))
            .child(
                section("backend-retention")

                    .child(
                        div().child(heading(tr(cx, "分析数据保留时长"))).child(
                            hint(tr(cx, "每日流量记录在路由器上存多久,超期每天清理一次;按小时的明细只留最近 7 天。超过 30 天的日子只保留当天流量最大的 300 个访问目标,其余合并成一行「其他」(总量、终端、节点不受影响)。"), p)
                                ,
                        ),
                    )
                    .child(
                        row()
                            .child(readonly_field(
                                "backend-retention-value",
                                "—",
                                t::SMALL_FIELD_WIDTH,
                                false,
                                p, cx
                            ))
                            .child(tr(cx, "个月"))
                            .child(hint(tr(cx, "未接入"), p)),
                    )
                    .child(hint(tr(cx, "已存数据：—"), p)),
            )
            .child(divider(line).mx(px(t::PADDING)))
            .child(
                section("backend-timezone")
                    .child(
                        div()
                            .child(heading(tr(cx, "时区")))
                            .child(hint(tr(cx, "路由器的系统时区,和 LuCI「系统 → 时区」是同一个设置。自动更新、订阅定时更新都按这个时区的钟点执行。"), p)),
                    )
                    .child(readonly_field(
                        "backend-timezone-value",
                        "—",
                        t::TIMEZONE_WIDTH,
                        true,
                        p, cx
                    ))
                    .child(hint(tr(cx, "后端时间：—"), p)),
            );
        let data = card()
            .id("backend-grouped-data")
            .gap_0()
            .child(
                section("backend-data-tools")
                    .child(
                        div()
                            .child(heading(tr(cx, "导出与导入")))
                            .child(hint(tr(cx, "把分流、站点集、节点组、DNS、面板设置等打成一个文件,可选带上订阅、链式代理、终端分流、共享网络;新设备导入即用。"), p)),
                    )
                    .child(
                        row()
                            .gap(px(t::GAP))
                            .child(
                                shell_button("backend-export", "", p, cx)
                                    .aria_label(tr(cx, "导出"))
                                    .child(icon("ArrowDownTray", t::ICON))
                                    .child(tr(cx, "导出")),
                            )
                            .children(
                                [
                                    ("backend-export-subscriptions", "包含订阅和节点"),
                                    ("backend-export-chains", "包含链式代理"),
                                    ("backend-export-clients", "包含终端分流"),
                                    ("backend-export-shared", "包含共享网络"),
                                ]
                                .map(|(id, label)| {
                                    row()
                                        .child(
                                            shell_button(id, "", p, cx)
                                                .size(px(t::BADGE_HEIGHT))
                                                .p_0()
                                                .border_1()
                                                .border_color(rgba(p.muted))
                                                .rounded(px(t::FIELD_GAP)),
                                        )
                                        .child(hint(tr(cx, label), p).text_size(px(t::BODY)))
                                }),
                            ),
                    )
                    .child(
                        row()
                            .gap(px(t::GAP))
                            .child(
                                shell_button("backend-import", "", p, cx)
                                    .aria_label(tr(cx, "导入"))
                                    .child(icon("ArrowUpTray", t::ICON))
                                    .child(tr(cx, "导入")),
                            )
                            .child(hint(tr(cx, "选一个导出的 .json 文件,或直接把文件拖到这张卡片上。"), p))
                            .child(hint(tr(cx, "未接入"), p)),
                    ),
            )
            .child(divider(line).mx(px(t::PADDING)))
            .child(
                section("backend-export-diagnostics")
                    .child(
                        div()
                            .child(heading(tr(cx, "导出诊断包")))
                            .child(hint(tr(cx, "版本、固件、内核状态、脱敏配置和最近日志打成一个文件,反馈问题时贴到 GitHub issue。"), p)),
                    )
                    .child(
                        row().child(
                            shell_button("backend-diagnostics-export", "", p, cx)
                                .aria_label(tr(cx, "导出诊断包"))
                                .child(icon("ArrowDownTray", t::ICON))
                                .child(tr(cx, "导出诊断包")),
                        ).child(hint(tr(cx,"已去掉密码、密钥、节点和订阅地址,可以直接贴到公开的 issue 里。"),p)).child(hint(tr(cx,"未接入"),p)),
                    ),
            );
        div()
            .id("backend-settings")
            .flex_shrink_0()
            .w_full()
            .flex()
            .flex_col()
            .gap(px(super::tokens::GAP))
            .p(px(super::tokens::GAP))
            .text_color(rgb(p.text))
            .text_size(px(t::BODY))
            .line_height(px(t::LINE))
            .children([service, grouped, diagnostic, data])
    }
}
#[cfg(test)]
mod tests {
    use super::{Palette, RuntimeCommand, RuntimeStatus, action_is_busy, status_tone};
    #[test]
    fn backend_only_active_command_spins() {
        let commands = [
            RuntimeCommand::Start,
            RuntimeCommand::Stop,
            RuntimeCommand::Restart,
        ];
        for (index, active) in commands.into_iter().enumerate() {
            let spinning = commands.map(|command| action_is_busy(Some(active), command));
            assert_eq!(spinning.iter().filter(|v| **v).count(), 1);
            assert!(spinning[index]);
        }
        assert!(
            commands
                .into_iter()
                .all(|c| !action_is_busy(Some(RuntimeCommand::Refresh), c))
        );
        assert!(commands.into_iter().all(|c| !action_is_busy(None, c)));
    }
    #[test]
    fn status_tones_distinguish_transition_error_and_stopped() {
        for dark in [false, true] {
            let p = Palette::new(dark, false);
            let tone = |s| status_tone(Some(s), p, dark);
            assert_eq!(tone(RuntimeStatus::Stopped).0, p.muted);
            assert_eq!(
                tone(RuntimeStatus::Starting),
                tone(RuntimeStatus::Recovering)
            );
            assert_ne!(tone(RuntimeStatus::Starting), tone(RuntimeStatus::Failed));
            assert_ne!(tone(RuntimeStatus::Stopped), tone(RuntimeStatus::Failed));
            assert_ne!(tone(RuntimeStatus::Ready), tone(RuntimeStatus::Starting));
        }
    }
    // 保护整页层级，不允许后续接线又退回单一内核卡。
    #[test]
    fn backend_full_hierarchy_keeps_reference_region_order() {
        let source = include_str!("backend.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let mut previous = 0;
        for region in [
            "backend-service-card",
            "backend-brand",
            "backend-version-row",
            "backend-geo-row",
            "backend-status-row",
            "backend-hint",
            "backend-service-actions",
            "backend-update-row",
            "backend-auto-update",
            "backend-grouped-routing",
            "backend-direct-nodes",
            "backend-direct-bypass",
            "backend-reject-quic",
            "backend-bypass-ports",
            "backend-ipv6",
            "backend-tun",
            "backend-grouped-diagnostics",
            "backend-test-urls",
            "backend-retention",
            "backend-timezone",
            "backend-grouped-data",
            "backend-data-tools",
            "backend-export-diagnostics",
        ] {
            let position = source.find(&format!("\"{region}\"")).unwrap();
            assert!(
                position > previous,
                "{region} must follow the previous region"
            );
            previous = position;
        }
    }
    // 禁止 visual shell 产生保存、网络、更新、TUN 等未来命令。
    #[test]
    fn future_controls_are_readonly_without_business_callbacks() {
        let source = include_str!("backend.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let helpers = source
            .split("fn shell_button")
            .nth(1)
            .unwrap()
            .split("impl Render")
            .next()
            .unwrap();
        assert!(!helpers.contains(".disabled("));
        assert!(helpers.contains(".role(Role::Label)"));
        assert!(helpers.contains(".aria_description"));
        assert!(!helpers.contains(".opacity("));
        assert!(!helpers.contains(".on_click"));
        assert!(!source.contains(".on_change"));
        assert_eq!(source.matches("cx.emit(command)").count(), 1);
        assert_eq!(source.matches(".on_click").count(), 2);
        assert!(source.contains(".disabled(self.operation.is_some() || disabled)"));
        assert!(!source.contains("ProfilePatch"));
        assert!(!source.contains("reqwest"));
        let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
        for id in [
            "backend-ipv6-policy",
            "backend-tun-stack",
            "backend-tun-mtu",
            "backend-tun-mss",
            "backend-timezone-value",
            "backend-retention-value",
            "backend-ports-value",
            "backend-update-channel",
        ] {
            assert!(
                compact.contains(&format!("readonly_field(\"{id}\"")),
                "{id} must remain read-only"
            );
        }
    }
    // 保护 snapshot 事实来源：只读值不使用默认 Profile 冒充真实保存值。
    #[test]
    fn readonly_profile_and_unknown_slots_are_honest() {
        let source = include_str!("backend.rs")
            .split("#[cfg(test)]")
            .next()
            .unwrap();
        let app = include_str!("../app.rs");
        assert!(app.contains("view.project_profile(state, window, cx)"));
        for field in [
            "direct_for_nodes",
            "reject_quic",
            "ipv6",
            "ipv6_proxy",
            "tun.stack",
            "tun.mtu",
            "tun.tcp_mss",
            "test_url.as_str()",
            "direct_test_url.as_str()",
        ] {
            assert!(
                source.contains(&format!("s.{field}"))
                    || source.contains(&format!("state.profile.{field}")),
                "{field}"
            );
        }
        assert!(!source.contains("Profile::default"));
        assert!(source.contains("GeoSite —"));
        assert!(source.contains("GeoIP —"));
        assert!(!source.contains("SECTION_HEIGHT"));
        let compact: String = source.chars().filter(|c| !c.is_whitespace()).collect();
        assert!(compact.contains("tr(cx,\"未接入\"),status_tone(None,p,dark)"));
        assert!(compact.contains("readonly_switch(\"backend-auto-update-unavailable\",None,p,cx"));
    }
}
