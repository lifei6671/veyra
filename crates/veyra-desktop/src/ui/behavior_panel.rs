//! Panel Settings behavior form. Mature inputs, independent draft and explicit save/rebase.
use super::components::select::{SelectEvent, SelectState};
use super::components::{button, select, text_input, toggle};
use crate::behavior_preferences::BehaviorStatus;
use crate::ui::i18n::tr;
use gpui_kit::{
    component::{
        Disableable, IndexPath, WindowExt,
        button::{Button, ButtonVariants},
        input::{InputEvent, InputState},
    },
    prelude::*,
    *,
};
use veyra_core::domain::*;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputKey {
    Url,
    Timeout,
    Low,
    Medium,
    Width,
    Order,
    SiteName(usize),
    SiteUrl(usize),
    SiteIcon(usize),
}
impl InputKey {
    fn value(self, p: &DesktopBehaviorPreferences) -> String {
        match self {
            Self::Url => p.latency.test_url.0.clone(),
            Self::Timeout => p.latency.timeout_ms.to_string(),
            Self::Low => p.latency.low_ms.to_string(),
            Self::Medium => p.latency.medium_ms.to_string(),
            Self::Width => p.proxy_view.node_card_min_width.to_string(),
            Self::Order => p.proxy_view.strategy_order.join(", "),
            Self::SiteName(i) => p.test_sites[i].name.clone(),
            Self::SiteUrl(i) => p.test_sites[i].url.clone(),
            Self::SiteIcon(i) => p.test_sites[i].icon_key.clone(),
        }
    }
    fn field(self) -> FieldPath {
        match self {
            Self::Url => FieldPath::LatencyUrl,
            Self::Timeout => FieldPath::LatencyTimeout,
            Self::Low => FieldPath::LatencyLow,
            Self::Medium => FieldPath::LatencyMedium,
            Self::Width => FieldPath::ProxyNodeWidth,
            Self::Order => FieldPath::ProxyStrategyOrder,
            Self::SiteName(_) => FieldPath::TestSiteName,
            Self::SiteUrl(_) => FieldPath::TestSiteUrl,
            Self::SiteIcon(_) => FieldPath::TestSiteIcon,
        }
    }
    fn patch(
        self,
        value: String,
        p: &DesktopBehaviorPreferences,
    ) -> Result<DesktopBehaviorPreferencesPatch, AppError> {
        let mut edit = DesktopBehaviorPreferencesPatch::default();
        let invalid = || AppError::validation(self.field());
        match self {
            Self::Url => edit.test_url = Some(UiLatencyUrl(value)),
            Self::Timeout => edit.timeout_ms = Some(value.parse().map_err(|_| invalid())?),
            Self::Low => edit.low_ms = Some(value.parse().map_err(|_| invalid())?),
            Self::Medium => edit.medium_ms = Some(value.parse().map_err(|_| invalid())?),
            Self::Width => edit.node_card_min_width = Some(value.parse().map_err(|_| invalid())?),
            Self::Order => {
                edit.strategy_order = Some(
                    value
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect(),
                )
            }
            Self::SiteName(i) | Self::SiteUrl(i) | Self::SiteIcon(i) => {
                let mut sites = p.test_sites.clone();
                match self {
                    Self::SiteName(_) => sites[i].name = value,
                    Self::SiteUrl(_) => sites[i].url = value,
                    Self::SiteIcon(_) => sites[i].icon_key = value,
                    _ => unreachable!(),
                }
                edit.test_sites = Some(sites);
            }
        }
        Ok(edit)
    }
}
pub enum BehaviorPanelEvent {
    Edit(DesktopBehaviorPreferencesPatch),
    Save,
    Rebase,
    ExternalWrite,
}
pub struct BehaviorPanel {
    icons: Vec<(
        Entity<super::components::icon_picker::IconPicker>,
        gpui_kit::Subscription,
    )>,
    pub draft: DesktopBehaviorPreferences,
    pub status: BehaviorStatus,
    // 由 Shell 投影的视觉状态，仅用于响应式布局，不新增持久化事实。
    pub sidebar_collapsed: bool,
    inputs: Vec<(InputKey, Entity<InputState>)>,
    columns: Entity<SelectState>,
    sort: Entity<SelectState>,
    provider: Entity<SelectState>,
    tab: usize,
    site: usize,
    input_errors: Vec<InputKey>,
    _subscriptions: Vec<gpui_kit::Subscription>,
}
impl EventEmitter<BehaviorPanelEvent> for BehaviorPanel {}
impl BehaviorPanel {
    // 领域允许 1–32 个测试站点；picker 与持久化列表一起增减，不能只为默认四项分配。
    fn site_icon(
        i: usize,
        key: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (
        Entity<super::components::icon_picker::IconPicker>,
        gpui_kit::Subscription,
    ) {
        let picker = cx.new(|cx| super::components::icon_picker::IconPicker::new(key, window, cx));
        let subscription = cx.subscribe(
            &picker,
            move |this, _, event: &super::components::icon_picker::IconPicked, cx| {
                let mut sites = this.draft.test_sites.clone();
                sites[i].icon_key = event.0.clone();
                this.edit(
                    DesktopBehaviorPreferencesPatch {
                        test_sites: Some(sites),
                        ..Default::default()
                    },
                    cx,
                );
            },
        );
        (picker, subscription)
    }
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let draft = DesktopBehaviorPreferences::default();
        let mut keys = vec![
            InputKey::Url,
            InputKey::Timeout,
            InputKey::Low,
            InputKey::Medium,
            InputKey::Width,
            InputKey::Order,
        ];
        for i in 0..draft.test_sites.len() {
            keys.extend([
                InputKey::SiteName(i),
                InputKey::SiteUrl(i),
                InputKey::SiteIcon(i),
            ]);
        }
        let inputs = keys
            .into_iter()
            .map(|key| {
                (
                    key,
                    cx.new(|cx| InputState::new(window, cx).default_value(key.value(&draft))),
                )
            })
            .collect::<Vec<_>>();
        let columns = cx.new(|cx| {
            SelectState::new(
                vec!["单列", "双列", "三列"],
                Some(IndexPath::new(1)),
                window,
                cx,
            )
        });
        let sort = cx.new(|cx| {
            SelectState::new(
                vec![
                    "default",
                    "nameAsc",
                    "nameDesc",
                    "latencyAsc",
                    "latencyDesc",
                ],
                Some(IndexPath::new(3)),
                window,
                cx,
            )
        });
        let provider = cx.new(|cx| {
            SelectState::new(
                vec!["ip.sb", "ipwho.is", "ipapi.is"],
                Some(IndexPath::new(0)),
                window,
                cx,
            )
        });
        let mut subscriptions = vec![
            cx.subscribe(&columns, |this, _, event, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    let columns = match value.as_ref() {
                        "单列" => 1,
                        "双列" => 2,
                        _ => 3,
                    };
                    if columns != this.draft.proxy_view.group_columns {
                        this.edit(
                            DesktopBehaviorPreferencesPatch {
                                group_columns: Some(columns),
                                ..Default::default()
                            },
                            cx,
                        );
                    }
                }
            }),
            cx.subscribe(&sort, |this, _, event, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    let sort = match value.as_ref() {
                        "default" => NodeSort::Default,
                        "nameAsc" => NodeSort::NameAsc,
                        "nameDesc" => NodeSort::NameDesc,
                        "latencyDesc" => NodeSort::LatencyDesc,
                        _ => NodeSort::LatencyAsc,
                    };
                    if sort != this.draft.proxy_view.node_sort {
                        this.edit(
                            DesktopBehaviorPreferencesPatch {
                                node_sort: Some(sort),
                                ..Default::default()
                            },
                            cx,
                        );
                    }
                }
            }),
            cx.subscribe(&provider, |this, _, event, cx| {
                if let SelectEvent::Confirm(Some(value)) = event {
                    let provider = match value.as_ref() {
                        "ipwho.is" => IpInfoProvider::IpWhoIs,
                        "ipapi.is" => IpInfoProvider::IpApiIs,
                        _ => IpInfoProvider::IpSb,
                    };
                    if provider != this.draft.ip_info {
                        this.edit(
                            DesktopBehaviorPreferencesPatch {
                                ip_info: Some(provider),
                                ..Default::default()
                            },
                            cx,
                        );
                    }
                }
            }),
        ];
        for (key, input) in &inputs {
            let key = *key;
            subscriptions.push(
                cx.subscribe(input, move |this, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        this.input_errors.retain(|k| *k != key);
                        if value != key.value(&this.draft) {
                            match key.patch(value, &this.draft) {
                                Ok(patch) => this.edit(patch, cx),
                                Err(_) => this.input_errors.push(key),
                            }
                        }
                        cx.notify();
                    }
                }),
            );
        }
        let icons = draft
            .test_sites
            .iter()
            .enumerate()
            .map(|(i, site)| Self::site_icon(i, site.icon_key.clone(), window, cx))
            .collect();
        Self {
            icons,
            draft,
            status: BehaviorStatus::Idle,
            sidebar_collapsed: false,
            inputs,
            columns,
            sort,
            provider,
            tab: 0,
            site: 0,
            input_errors: vec![],
            _subscriptions: subscriptions,
        }
    }
    fn edit(&mut self, patch: DesktopBehaviorPreferencesPatch, cx: &mut Context<Self>) {
        self.draft = patch.apply(&self.draft);
        self.status = BehaviorStatus::Pending;
        cx.emit(BehaviorPanelEvent::Edit(patch));
        cx.notify();
    }
    pub fn project(
        &mut self,
        draft: &DesktopBehaviorPreferences,
        status: &BehaviorStatus,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.draft = draft.clone();
        self.status = status.clone();
        self.icons.truncate(draft.test_sites.len());
        for i in self.icons.len()..draft.test_sites.len() {
            self.icons.push(Self::site_icon(
                i,
                draft.test_sites[i].icon_key.clone(),
                window,
                cx,
            ));
        }
        for ((picker, _), site) in self.icons.iter().zip(&draft.test_sites) {
            picker.update(cx, |s, cx| {
                s.value = site.icon_key.clone();
                cx.notify();
            });
        }
        // Preserve invalid raw numeric input across refresh/rebase as well as save failure.
        self.inputs.retain(|(key, _)| match key {
            InputKey::SiteName(i) | InputKey::SiteUrl(i) | InputKey::SiteIcon(i) => {
                *i < draft.test_sites.len()
            }
            _ => true,
        });
        for (key, input) in &self.inputs {
            let value = key.value(draft);
            if !self.input_errors.contains(key) && input.read(cx).value().as_str() != value {
                input.update(cx, |input, cx| input.set_value(value, window, cx));
            }
        }
        self.columns.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(usize::from(
                    draft.proxy_view.group_columns - 1,
                ))),
                window,
                cx,
            )
        });
        self.sort.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match draft.proxy_view.node_sort {
                    NodeSort::Default => 0,
                    NodeSort::NameAsc => 1,
                    NodeSort::NameDesc => 2,
                    NodeSort::LatencyAsc => 3,
                    NodeSort::LatencyDesc => 4,
                })),
                window,
                cx,
            )
        });
        self.provider.update(cx, |s, cx| {
            s.set_selected_index(
                Some(IndexPath::new(match draft.ip_info {
                    IpInfoProvider::IpSb => 0,
                    IpInfoProvider::IpWhoIs => 1,
                    IpInfoProvider::IpApiIs => 2,
                })),
                window,
                cx,
            )
        });
        // Core accepts bounded configurable site lists; allocate stable input entities on projection,
        // never in render. The four canonical sites are not an assumed storage array length.
        let mut missing = vec![];
        for i in 0..draft.test_sites.len() {
            for key in [
                InputKey::SiteName(i),
                InputKey::SiteUrl(i),
                InputKey::SiteIcon(i),
            ] {
                if !self.inputs.iter().any(|(k, _)| *k == key) {
                    missing.push(key);
                }
            }
        }
        for key in missing {
            let input = cx.new(|cx| InputState::new(window, cx).default_value(key.value(draft)));
            self._subscriptions.push(cx.subscribe(
                &input,
                move |this, input, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        let value = input.read(cx).value().to_string();
                        if value != key.value(&this.draft) {
                            this.edit(key.patch(value, &this.draft).unwrap(), cx);
                        }
                    }
                },
            ));
            self.inputs.push((key, input));
        }
        self.site = self.site.min(draft.test_sites.len() - 1);
        cx.notify();
    }
    fn input(&self, key: InputKey) -> Entity<InputState> {
        self.inputs
            .iter()
            .find(|(k, _)| *k == key)
            .unwrap()
            .1
            .clone()
    }
    fn input_row(&self, label: &'static str, key: InputKey) -> Div {
        let invalid = self.input_errors.contains(&key)
            || matches!(&self.status,BehaviorStatus::Failed(error) if error.field()==Some(key.field()));
        row(
            label,
            div()
                .w(px(440.))
                .child(text_input(&self.input(key)))
                .when(invalid, |d| {
                    d.child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xd95858))
                            .child("请检查此字段"),
                    )
                }),
        )
    }
}
fn row(label: impl Into<SharedString>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .min_h(px(40.))
        .py(px(4.))
        .gap_3()
        .border_b_1()
        .border_color(rgba(0x80808022))
        .child(div().flex_1().child(label.into()))
        .child(div().flex_shrink_0().child(control))
}
impl BehaviorPanel {
    fn render_evidence(&mut self, _: &mut Window, cx: &mut Context<Self>) -> Div {
        let labels = [
            "延迟 / Latency",
            "代理显示 / Proxy display",
            "诊断 / IP 信息",
            "测速站点 / Test sites",
        ];
        let mut body = div().flex().flex_col().w_full().flex_shrink_0().gap_1();
        match self.tab {
            0 => {
                body=body.child(self.input_row("节点测速地址 · 仅偏好，不请求",InputKey::Url)).child(self.input_row("Timeout · ms",InputKey::Timeout)).child(self.input_row("低延迟阈值 · ms",InputKey::Low)).child(self.input_row("中延迟阈值 · ms",InputKey::Medium)).child(div().text_xs().child("UI latency URL 与 Runtime / Group health URL 独立。阈值统一为 Panel 默认 400 / 800 ms。"));
            }
            1 => {
                body = body
                    .child(row("节点组列数", select(&self.columns, "Group columns")))
                    .child(row(
                        "隐藏不可用节点",
                        toggle(
                            "hide-unavailable",
                            self.draft.proxy_view.hide_unavailable,
                            "Hide unavailable",
                        )
                        .on_change(cx.listener(|this, value, _, cx| {
                            this.edit(
                                DesktopBehaviorPreferencesPatch {
                                    hide_unavailable: Some(*value),
                                    ..Default::default()
                                },
                                cx,
                            )
                        })),
                    ))
                    .child(row("节点排序", select(&self.sort, "Node sort")))
                    .child(row(
                        "按 Provider 分组",
                        toggle(
                            "group-provider",
                            self.draft.proxy_view.group_by_provider,
                            "Group by provider",
                        )
                        .on_change(cx.listener(|this, value, _, cx| {
                            this.edit(
                                DesktopBehaviorPreferencesPatch {
                                    group_by_provider: Some(*value),
                                    ..Default::default()
                                },
                                cx,
                            )
                        })),
                    ))
                    .child(self.input_row("节点卡片最小宽度 · 100–320 px", InputKey::Width))
                    .child(self.input_row("策略显示 ID · 逗号分隔", InputKey::Order));
            }
            2 => {
                body=body.child(row("UI diagnostics IPv6 test",toggle("diagnostic-ipv6",self.draft.diagnostics.ipv6_test,"Diagnostic IPv6 test").on_change(cx.listener(|this,value,_,cx|this.edit(DesktopBehaviorPreferencesPatch { ipv6_test:Some(*value),..Default::default() },cx))))).child(row("IP 信息 provider · 仅偏好",select(&self.provider,"IP info provider"))).child(div().text_xs().child("本选项不修改 Profile.ipv6，不触发 Runtime apply / restart；本卡不调用 IP provider。"));
            }
            _ => {
                body = body
                    .child(div().flex().gap_2().children(
                        self.draft.test_sites.iter().enumerate().map(|(i, site)| {
                            Button::new(SharedString::from(format!("behavior-site-{}", site.id)))
                                .label(site.id.clone())
                                .h(px(32.))
                                .when(self.site == i, |b| b.primary())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.site = i;
                                    cx.notify();
                                }))
                        }),
                    ))
                    .child(self.input_row("站点名称", InputKey::SiteName(self.site)))
                    .child(
                        self.input_row("站点 URL · 仅偏好，不请求", InputKey::SiteUrl(self.site)),
                    )
                    .child(self.input_row("Icon key · 本地文字标识", InputKey::SiteIcon(self.site)))
                    .child(
                        button("restore-sites", "恢复四个默认站点").on_click(cx.listener(
                            |this, _, window, cx| {
                                this.edit(
                                    DesktopBehaviorPreferencesPatch {
                                        test_sites: Some(
                                            DesktopBehaviorPreferences::default().test_sites,
                                        ),
                                        ..Default::default()
                                    },
                                    cx,
                                );
                                let draft = this.draft.clone();
                                let status = this.status.clone();
                                this.project(&draft, &status, window, cx);
                            },
                        )),
                    )
                    .child(
                        div()
                            .text_xs()
                            .child("不下载图标、不实际测速；旧浏览器 localStorage 不自动导入。"),
                    );
            }
        }
        div()
            .flex()
            .flex_col()
            .w_full()
            .flex_shrink_0()
            .gap_3()
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .children(labels.into_iter().enumerate().map(|(i, label)| {
                        button(
                            [
                                "behavior-latency",
                                "behavior-proxy",
                                "behavior-ip",
                                "behavior-sites",
                            ][i],
                            label,
                        )
                        .when(self.tab == i, |b| b.primary())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.tab = i;
                            cx.notify();
                        }))
                    })),
            )
            .child(body)
            .child(
                div()
                    .min_h(px(84.))
                    .p_4()
                    .rounded_lg()
                    .border_1()
                    .border_color(rgb(0x70c996))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(format!(
                        "本地行为偏好 · {}",
                        match &self.status {
                            BehaviorStatus::Idle => "已保存",
                            BehaviorStatus::Pending => "草稿未保存",
                            BehaviorStatus::Saving => "保存中",
                            BehaviorStatus::Failed(_) => "保存失败 · draft 保留",
                        }
                    ))
                    .when(!self.input_errors.is_empty(), |d| {
                        d.child(
                            div()
                                .text_color(rgb(0xd95858))
                                .child("整数输入无效：保留原始输入，请修正对应字段。"),
                        )
                    })
                    .when(matches!(&self.status, BehaviorStatus::Failed(_)), |d| {
                        d.child(div().text_color(rgb(0xd95858)).child(match &self.status {
                            BehaviorStatus::Failed(e) => {
                                crate::ui::i18n::message(cx, &e.to_string())
                            }
                            _ => unreachable!(),
                        }))
                    })
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                button("save-behavior", "保存 / Retry")
                                    .primary()
                                    .disabled(
                                        !self.input_errors.is_empty()
                                            || self.status == BehaviorStatus::Saving,
                                    )
                                    .on_click(
                                        cx.listener(|_, _, _, cx| {
                                            cx.emit(BehaviorPanelEvent::Save)
                                        }),
                                    ),
                            )
                            .child(button("rebase-behavior", "刷新 / Rebase").on_click(
                                cx.listener(|_, _, _, cx| cx.emit(BehaviorPanelEvent::Rebase)),
                            ))
                            .when(cfg!(debug_assertions), |d| {
                                d.child(
                                    button("external-behavior", "Evidence: 外部 CAS 写入")
                                        .on_click(cx.listener(|_, _, _, cx| {
                                            cx.emit(BehaviorPanelEvent::ExternalWrite)
                                        })),
                                )
                            }),
                    ),
            )
            .child(div().text_xs().child(
                "Preferences 消费契约已提供；真实代理、概览、连接与诊断业务将在 P2/P3/P4/P5 接入。",
            ))
    }
}

impl BehaviorPanel {
    pub fn provider_control(&self) -> super::components::select::Select {
        select(&self.provider, "IP信息API").w(px(super::tokens::PROVIDER_WIDTH))
    }
    fn number_setting(&self, label: &'static str, key: InputKey) -> impl IntoElement {
        super::components::compact_setting(
            label,
            div()
                .flex()
                .items_center()
                .gap(px(super::tokens::GAP))
                .child(
                    text_input(&self.input(key))
                        .numeric()
                        .w(px(super::tokens::NUMBER_WIDTH)),
                )
                .child("ms"),
        )
    }
}
impl Render for BehaviorPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use super::components::*;
        use super::tokens as t;
        use gpui_kit::component::ActiveTheme;
        if evidence_visible() {
            return self.render_evidence(window, cx);
        }
        let dark = cx.theme().mode.is_dark();
        let layout = SettingsLayout::new(
            f32::from(window.viewport_size().width),
            self.sidebar_collapsed,
        );
        let latency = setting_section(tr(cx, "延迟"), dark, f32::from(cx.theme().radius_lg))
            .child(
                layout
                    .grid()
                    .child(self.number_setting(tr(cx, "测速超时"), InputKey::Timeout))
                    .child(self.number_setting(tr(cx, "黄色的阈值"), InputKey::Low))
                    .child(self.number_setting(tr(cx, "红色的阈值"), InputKey::Medium))
                    .child(compact_setting(
                        tr(cx, "IPv6 测试"),
                        toggle(
                            "diagnostic-ipv6",
                            self.draft.diagnostics.ipv6_test,
                            tr(cx, "IPv6 测试"),
                        )
                        .on_change(cx.listener(|this, value, _, cx| {
                            this.edit(
                                DesktopBehaviorPreferencesPatch {
                                    ipv6_test: Some(*value),
                                    ..Default::default()
                                },
                                cx,
                            )
                        })),
                    ))
                    .child(compact_setting(
                        tr(cx, "隐藏不可用节点"),
                        toggle(
                            "hide-unavailable",
                            self.draft.proxy_view.hide_unavailable,
                            tr(cx, "隐藏不可用节点"),
                        )
                        .on_change(cx.listener(|this, value, _, cx| {
                            this.edit(
                                DesktopBehaviorPreferencesPatch {
                                    hide_unavailable: Some(*value),
                                    ..Default::default()
                                },
                                cx,
                            )
                        })),
                    )),
            )
            .child(
                section_heading(tr(cx, "布局"))
                    .mt(px(crate::ui::tokens::ROW_GAP))
                    .mb(px(crate::ui::tokens::ROW_GAP)),
            )
            .child(layout.grid().child(compact_setting(
                tr(cx, "代理组分列"),
                select(&self.columns, tr(cx, "代理组分列")).w(px(t::SITE_NAME_WIDTH)),
            )));
        let mut sites = setting_section(
            super::components::help_label(
                tr(cx, "测试站点"),
                "sites-help",
                tr(
                    cx,
                    "概览里的延时小卡片和规则页右上角的快捷查询共用这四个站点。图标从图标库选;名称空着就用图标的品牌名(没有品牌名就用网址的主机名);网址填 http(s) 地址,延时按内核经当前分流访问这个地址计",
                ),
                cx,
            ),
            dark,
            f32::from(cx.theme().radius_lg),
        );
        let rows = (0..self.draft.test_sites.len())
            .map(|i| {
                div()
                    .flex()
                    .min_w_0()
                    .h(px(t::SETTING_HEIGHT))
                    .items_center()
                    .gap(px(t::GAP))
                    .border_b_1()
                    .border_color(if dark {
                        rgba(0x110d0dcc)
                    } else {
                        rgba(crate::ui::tokens::ROW_BORDER)
                    })
                    .child(self.icons[i].0.clone())
                    .child(text_input(&self.input(InputKey::SiteName(i))).w(px(t::SITE_NAME_WIDTH)))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .relative()
                            .child(
                                text_input(&self.input(InputKey::SiteUrl(i)))
                                    .pr(px(crate::ui::tokens::CONTROL)),
                            )
                            .child(
                                icon_button(
                                    ("clear-site-url", i),
                                    tr(cx, "清空测试地址"),
                                    "XMark",
                                    16.,
                                )
                                .absolute()
                                .top(px(crate::ui::tokens::GAP))
                                .right(px(crate::ui::tokens::GAP))
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.input(InputKey::SiteUrl(i))
                                            .update(cx, |s, cx| s.replace_all("", window, cx))
                                    },
                                )),
                            ),
                    )
            })
            .collect::<Vec<_>>();
        sites = sites.child(layout.grid().children(rows));
        let entity = cx.entity().downgrade();
        sites=sites.relative().child(icon_button("restore-sites",tr(cx, "恢复默认测试站点"),"ArrowUturnLeft",t::CONTROL)
            .bg(if dark { rgb(0x161212) } else { rgba(0xededeeb3) })
            .on_click(move |_,window,cx|{
                let entity=entity.clone();
                window.open_dialog(cx,move |dialog,window,cx| {
                    let entity=entity.clone();
                    panel_dialog(dialog,window,cx)
                        .child(div().flex().flex_col().gap_0()
                            .child(div().flex().h(px(t::DIALOG_HEADER)).items_center().justify_between().px(px(crate::ui::tokens::SECTION_PADDING)).border_b_1().border_color(rgba(0x4b52631a))
                                .child(tr(cx, "恢复默认")).child(icon_button("close-restore",tr(cx, "关闭"),"XMark",24.).on_click(|_,w,cx|w.close_dialog(cx))))
                            .child(div().p(px(crate::ui::tokens::SECTION_PADDING)).child(tr(cx, "四个站点的图标、名称和网址都会覆盖回随包默认。确定吗?")))
                            .child(div().flex().justify_end().gap(px(10.)).px(px(crate::ui::tokens::SECTION_PADDING)).py(px(crate::ui::tokens::GAP)).border_t_1().border_color(rgba(0x4b52631a))
                                .child(button("cancel-restore",tr(cx, "取消")).on_click(|_,w,cx|w.close_dialog(cx)))
                                .child(button("confirm-restore",tr(cx, "确定")).bg(rgb(0xff5263)).on_click(move |_,window,cx|{
                                    let _=entity.update(cx,|this,cx|{
                                        this.edit(DesktopBehaviorPreferencesPatch{test_sites:Some(DesktopBehaviorPreferences::default().test_sites),..Default::default()},cx);
                                        let draft=this.draft.clone();let status=this.status.clone();this.project(&draft,&status,window,cx);
                                    });
                                    window.close_dialog(cx);
                                }))))
                });
            }).tooltip("restore-sites-tooltip", tr(cx, "恢复默认")).absolute().top(px(t::SECTION_PADDING)).right(px(t::SECTION_PADDING)));
        // P1-04B 显式提交、错误草稿和 CAS rebase 保持有效。内容不可收缩，否则保存按钮被裁切且父级无法滚动。
        div()
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(px(t::GAP))
            .child(latency)
            .child(sites)
            .child(
                div()
                    .flex()
                    .gap(px(t::GAP))
                    .child(
                        button("save-behavior", tr(cx, "保存"))
                            .disabled(
                                !self.input_errors.is_empty()
                                    || self.status == BehaviorStatus::Saving,
                            )
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(BehaviorPanelEvent::Save))),
                    )
                    .when(matches!(self.status, BehaviorStatus::Failed(_)), |d| {
                        d.child(button("rebase-behavior", tr(cx, "重新加载")).on_click(
                            cx.listener(|_, _, _, cx| cx.emit(BehaviorPanelEvent::Rebase)),
                        ))
                    }),
            )
            .when(!self.input_errors.is_empty(), |d| {
                d.child(tr(cx, "整数输入无效，请修正对应字段。"))
            })
            .when(matches!(self.status, BehaviorStatus::Failed(_)), |d| {
                d.child(match &self.status {
                    BehaviorStatus::Failed(e) => crate::ui::i18n::message(cx, &e.to_string()),
                    _ => unreachable!(),
                })
            })
    }
}

#[cfg(test)]
mod edit_tests {
    use super::InputKey;
    use crate::{
        behavior_preferences::{BehaviorCoordinator, BehaviorStatus},
        ui::components::PanelInput,
    };
    use veyra_core::domain::*;
    // 保护 spinner 生成值进入既有 typed patch/CAS 草稿；原生 Change 另由 targeted GUI 验证。
    #[test]
    fn spinner_values_match_typed_draft_and_save_patch() {
        let mut state = AppState::empty();
        state.app_config.behavior.latency.timeout_ms = 800;
        let mut c = BehaviorCoordinator::default();
        c.rebase(&state);
        let mut display = "800".to_owned();
        for (increment, expected) in [(true, 801), (false, 800)] {
            display = PanelInput::step_value(&display, increment);
            c.edit(InputKey::Timeout.patch(display.clone(), &c.draft).unwrap());
            assert_eq!(c.draft.latency.timeout_ms, expected);
            assert_eq!(display, InputKey::Timeout.value(&c.draft));
            assert_eq!(c.status, BehaviorStatus::Pending);
        }
        let request = c.submit().unwrap();
        assert_eq!(request.expected, state.config_version());
        assert_eq!(request.patch.timeout_ms, Some(800));
    }
    // 空 URL 是真实业务编辑；保留无效草稿，保存应按原有领域验证拒绝。
    #[test]
    fn clear_url_keeps_empty_draft_and_original_validation() {
        let state = AppState::empty();
        let mut c = BehaviorCoordinator::default();
        c.rebase(&state);
        c.edit(InputKey::SiteUrl(0).patch(String::new(), &c.draft).unwrap());
        assert_eq!(InputKey::SiteUrl(0).value(&c.draft), "");
        assert_eq!(c.status, BehaviorStatus::Pending);
        let request = c.submit().unwrap();
        assert_eq!(request.patch.test_sites.as_ref().unwrap()[0].url, "");
        let error = request
            .patch
            .apply(&state.app_config.behavior)
            .validate()
            .unwrap_err();
        c.complete(request.generation, Err(error));
        assert!(matches!(c.status, BehaviorStatus::Failed(_)));
        assert_eq!(c.draft.test_sites[0].url, "");
        assert_eq!(c.pending.test_sites.as_ref().unwrap()[0].url, "");
    }
    #[test]
    fn rebase_projects_values_without_new_edit_or_pending_patch() {
        let mut state = AppState::empty();
        let mut c = BehaviorCoordinator::default();
        state.app_config.behavior.latency.timeout_ms = 801;
        c.rebase(&state);
        assert_eq!(InputKey::Timeout.value(&c.draft), "801");
        assert_eq!(c.generation, 0);
        assert!(c.pending.changed_fields().is_empty());
        assert_eq!(c.status, BehaviorStatus::Idle);
    }
}
