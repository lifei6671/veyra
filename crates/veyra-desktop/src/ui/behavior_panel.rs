//! Panel Settings behavior form. Mature inputs, independent draft and explicit save/rebase.
use super::components::{button, select, text_input, toggle};
use crate::behavior_preferences::BehaviorStatus;
use gpui_kit::{
    component::{
        Disableable, IndexPath,
        button::{Button, ButtonVariants},
        input::{InputEvent, InputState},
        select::{SelectEvent, SelectState},
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
    pub draft: DesktopBehaviorPreferences,
    pub status: BehaviorStatus,
    inputs: Vec<(InputKey, Entity<InputState>)>,
    columns: Entity<SelectState<Vec<&'static str>>>,
    sort: Entity<SelectState<Vec<&'static str>>>,
    provider: Entity<SelectState<Vec<&'static str>>>,
    tab: usize,
    site: usize,
    input_errors: Vec<InputKey>,
    _subscriptions: Vec<gpui_kit::Subscription>,
}
impl EventEmitter<BehaviorPanelEvent> for BehaviorPanel {}
impl BehaviorPanel {
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
        let columns =
            cx.new(|cx| SelectState::new(vec!["1", "2", "3"], Some(IndexPath::new(1)), window, cx));
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
                    let columns = value.parse().unwrap();
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
                    let sort = match *value {
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
                    let provider = match *value {
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
        Self {
            draft,
            status: BehaviorStatus::Idle,
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
impl Render for BehaviorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                            BehaviorStatus::Failed(e) => e.to_string(),
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
