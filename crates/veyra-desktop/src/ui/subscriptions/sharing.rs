use super::*;
use veyra_core::{
    application::shares::{ShareCommand, ShareError, ShareService},
    domain::SubscriptionShare,
};

impl SubscriptionsView {
    pub fn attach_shares(&mut self, services: std::sync::Arc<crate::services::AppServices>) {
        self.share_services = Some(services);
    }
    pub(super) fn share_send(&mut self, command: Option<ShareCommand>, cx: &mut Context<Self>) {
        if self.share_busy {
            return;
        }
        let Some(services) = &self.share_services else {
            return;
        };
        let refreshing = command.is_none();
        let previous_error = self.share_error.filter(|_| self.share_failed_mutation);
        let saving = matches!(command, Some(ShareCommand::Save(_)));
        let receiver = services.shares_command(self.share_version.clone(), command);
        self.share_busy = true;
        // 刷新只读取已保存事实，不能把先前失败的保存/列表动作变为成功。
        if !refreshing {
            self.share_error = None;
        }
        cx.spawn(async move |view, cx| {
            let result = receiver.await.unwrap_or(Err(ShareError::Closing));
            let _ = view.update(cx, |this, cx| {
                this.share_busy = false;
                if !refreshing {
                    this.share_failed_mutation = result.is_err();
                }
                this.share_error =
                    share_failure_after(refreshing, previous_error, result.as_ref().err().copied());
                if let Err(error) = &result {
                    super::super::components::notice::notify_app(
                        super::super::components::Notice::Error,
                        share_error_key(*error),
                        cx,
                    );
                }
                if let Ok(state) = result {
                    this.project(&state, cx);
                    cx.emit(SharesUpdated(Box::new(state)));
                    if saving {
                        this.share_editor_open = false;
                        this.share_draft = None;
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn open_share(
        &mut self,
        existing: Option<SubscriptionShare>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let draft = match existing.map(Ok).unwrap_or_else(ShareService::draft) {
            Ok(draft) => draft,
            Err(error) => {
                self.share_error = Some(error);
                cx.notify();
                return;
            }
        };
        for (input, value) in self.share_inputs.iter().zip([
            draft.name.clone(),
            draft.host.clone(),
            draft.url(),
            draft.listen.to_string(),
        ]) {
            input.update(cx, |state, cx| state.set_value(value, window, cx));
        }
        self.share_draft = Some(draft);
        self.share_editor_open = true;
        self.share_error = None;
        self.share_failed_mutation = false;
        self.share_inputs[0]
            .read(cx)
            .focus_handle(cx)
            .focus(window, cx);
        cx.notify();
    }
    pub(super) fn save_share(&mut self, cx: &mut Context<Self>) {
        let Some(mut draft) = self.share_draft.clone() else {
            return;
        };
        draft.name = self.share_inputs[0].read(cx).value().to_string();
        draft.host = self.share_inputs[1].read(cx).value().trim().to_owned();
        let Ok(listen) = self.share_inputs[3].read(cx).value().parse() else {
            self.share_error = Some(ShareError::Invalid);
            self.share_failed_mutation = true;
            cx.notify();
            return;
        };
        draft.listen = listen;
        self.share_send(Some(ShareCommand::Save(draft)), cx);
    }
    fn share_error_view(&self, cx: &mut Context<Self>) -> Div {
        let mut view = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(t::GAP))
            .text_size(px(t::SUBSCRIPTION_SHARE_FIELD_FONT));
        if let Some(error) = self.share_error {
            let key = share_error_key(error);
            view = view.child(tr(cx, key)).child(
                button("share-refresh", tr(cx, "刷新"))
                    .disabled(self.share_busy)
                    .on_click(cx.listener(|this, _, _, cx| this.share_send(None, cx))),
            );
        }
        view
    }
    pub(super) fn share_rows(&self, p: SettingsColors, cx: &mut Context<Self>) -> Div {
        let mut body = div()
            .font_weight(super::super::theme::MISANS_REGULAR)
            .flex()
            .flex_col()
            .px(px(t::SECTION_PADDING))
            .pb(px(t::SECTION_PADDING))
            .gap(px(t::GAP));
        if self.share_busy {
            body = body.child(share_spinner("share-list-loading", t::BODY, p.text));
        }
        if self.share_error.is_some() && !self.share_editor_open {
            body = body.child(self.share_error_view(cx));
        }
        if self.shares.is_empty() && !self.share_busy {
            return body.pb_0().child(
                div()
                    .h(px(t::SUBSCRIPTION_NODE_EMPTY))
                    .flex()
                    .items_center()
                    .justify_center()
                    .pt(px(t::PAD))
                    .pb(px(t::SECTION_PADDING))
                    .text_color(p.muted)
                    .child(tr(cx, "暂无订阅分享，点右上角「添加」创建")),
            );
        }
        for share in &self.shares {
            let id = share.id.clone();
            let enabled = share.enabled;
            let edit = share.clone();
            let regenerate = id.clone();
            let delete = id.clone();
            let url = share.url();
            let names = share
                .subscription_ids
                .iter()
                .map(|id| {
                    self.model
                        .list
                        .iter()
                        .find(|s| s.id == id.0)
                        .map(|s| s.name.clone())
                        .unwrap_or_else(|| id.0.clone())
                })
                .collect::<Vec<_>>()
                .join("、");
            let actions = div()
                .flex()
                .gap(px(t::ROW_GAP))
                .child(
                    icon_action(
                        SharedString::from(format!("share-power-{id}")),
                        if enabled {
                            "停用订阅分享"
                        } else {
                            "启用订阅分享"
                        },
                        "Power",
                        p,
                        cx,
                    )
                    .when(enabled, |b| {
                        b.text_color(rgb(t::ACCENT_STRONG)).bg(p.button_hover())
                    })
                    .disabled(self.share_busy)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.share_send(Some(ShareCommand::Enabled(id.clone(), !enabled)), cx)
                    })),
                )
                .child(
                    icon_action(
                        SharedString::from(format!("share-copy-{}", share.id)),
                        "复制分享链接",
                        "ClipboardDocument",
                        p,
                        cx,
                    )
                    .on_click(move |_, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(url.clone()))
                    }),
                )
                .child(
                    icon_action(
                        SharedString::from(format!("share-rotate-{}", share.id)),
                        "重新生成分享链接",
                        "ArrowPathRoundedSquare",
                        p,
                        cx,
                    )
                    .disabled(self.share_busy)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.share_confirm = Some(ShareCommand::Regenerate(regenerate.clone()));
                        cx.notify();
                    })),
                )
                .child(
                    icon_action(
                        SharedString::from(format!("share-edit-{}", share.id)),
                        "修改订阅分享",
                        "PencilSquare",
                        p,
                        cx,
                    )
                    .disabled(self.share_busy)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_share(Some(edit.clone()), window, cx)
                    })),
                )
                .child(
                    icon_action(
                        SharedString::from(format!("share-delete-{}", share.id)),
                        "删除订阅分享",
                        "Trash",
                        p,
                        cx,
                    )
                    .disabled(self.share_busy)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.share_confirm = Some(ShareCommand::Delete(delete.clone()));
                        cx.notify();
                    })),
                );
            body = body.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(t::PAD))
                    .min_h(px(t::SUBSCRIPTION_SHARE_CARD_HEIGHT))
                    .py(px(t::GAP))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(t::SUBSCRIPTION_SHARE_TEXT_GAP))
                            .min_w_0()
                            .flex_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .text_size(px(t::BODY))
                                    .child(
                                        div()
                                            .font_weight(super::super::theme::MISANS_MEDIUM)
                                            .child(share.name.clone()),
                                    )
                                    .child(
                                        hint(format!(" · {names}"), p)
                                            .text_size(px(t::SUBSCRIPTION_SHARE_META_FONT)),
                                    ),
                            )
                            .child(
                                hint(share.url(), p)
                                    .font_family("Menlo")
                                    .text_size(px(t::SUBSCRIPTION_SHARE_META_FONT)),
                            ),
                    )
                    .child(actions),
            );
        }
        body
    }
    pub(super) fn share_confirmation(&self, cx: &mut Context<Self>) -> AnyElement {
        let command = self.share_confirm.clone().unwrap();
        let deleting = matches!(command, ShareCommand::Delete(_));
        let id = match &command {
            ShareCommand::Delete(id) | ShareCommand::Regenerate(id) => id,
            _ => unreachable!("only delete and regenerate require confirmation"),
        };
        let Some(share) = self.shares.iter().find(|share| &share.id == id) else {
            return div().into_any_element();
        };
        // 名称是用户数据；仅翻译模板，保持 React confirm 的具体操作对象。
        let key = if deleting {
            "确定删除订阅分享「{name}」吗？删除后链接将失效。"
        } else {
            "重新生成「{name}」的链接？旧链接会立即失效，已经导入的设备要重新添加。"
        };
        let message = tr(cx, key).replace("{name}", &share.name);
        let p = SettingsColors::sharing(cx);
        let text = if cx.theme().mode.is_dark() {
            p.text
        } else {
            rgb(t::SUBSCRIPTION_SHARE_CONFIRM_TEXT).into()
        };
        let confirm_button = |id, label, danger| {
            button(id, label)
                .h(px(t::CONTROL))
                .px(px(t::PAD))
                .text_size(px(t::BODY))
                .line_height(px(t::BODY_LINE))
                .font_weight(super::super::theme::MISANS_SEMIBOLD)
                .rounded(px(t::SUBSCRIPTION_SHARE_CONFIRM_BUTTON_RADIUS))
                .bg(if danger {
                    rgb(t::SUBSCRIPTION_SHARE_CONFIRM_ERROR)
                } else {
                    rgb(t::SUBSCRIPTION_SHARE_CONFIRM_CANCEL)
                })
                .text_color(if danger { rgb(0x000000).into() } else { text })
        };
        let surface = div()
            .w(px(t::DIALOG_WIDTH))
            .rounded(px(t::SUBSCRIPTION_SHARE_CONFIRM_RADIUS))
            .overflow_hidden()
            .bg(p.solid)
            .text_color(text)
            .font_weight(super::super::theme::MISANS_REGULAR)
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(t::DIALOG_HEADER))
                    .px(px(t::SECTION_PADDING))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(text.opacity(0.1))
                    .child(
                        div()
                            .text_size(px(t::SECTION_TITLE))
                            .line_height(px(t::SECTION_LINE))
                            .font_weight(FontWeight::BOLD)
                            .child(tr(cx, "确定")),
                    )
                    .child(
                        super::super::components::icon_button(
                            "share-confirm-close",
                            tr(cx, "关闭"),
                            "XMark",
                            t::SWITCH_HEIGHT,
                        )
                        .mr(-px(t::GAP))
                        .text_color(text)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.share_confirm = None;
                            cx.notify();
                        })),
                    ),
            )
            .child(
                div()
                    .p(px(t::SECTION_PADDING))
                    .text_size(px(t::BODY))
                    .line_height(px(t::BODY_LINE))
                    .child(message),
            )
            .child(
                div()
                    .px(px(t::SECTION_PADDING))
                    .py(px(t::GAP))
                    .border_t_1()
                    .border_color(text.opacity(0.1))
                    .flex()
                    .justify_end()
                    .gap(px(t::GAP))
                    .child(
                        confirm_button("share-dismiss", tr(cx, "取消"), false).on_click(
                            cx.listener(|this, _, _, cx| {
                                this.share_confirm = None;
                                cx.notify();
                            }),
                        ),
                    )
                    .child(
                        confirm_button("share-confirm", tr(cx, "确定"), true).on_click(
                            cx.listener(move |this, _, _, cx| {
                                this.share_confirm = None;
                                this.share_send(Some(command.clone()), cx);
                            }),
                        ),
                    ),
            );
        gpui_kit::base::Dialog::new(cx)
            .open(true)
            .backdrop(
                div()
                    .size_full()
                    .bg(rgba(t::SUBSCRIPTION_SHARE_CONFIRM_OVERLAY)),
            )
            .on_ok(|_, _, _| false)
            .popup(gpui_kit::base::DialogPopup::new().child(surface))
            .on_close(cx.listener(|this, _, _, cx| {
                this.share_confirm = None;
                cx.notify();
            }))
            .into_any_element()
    }
    pub(super) fn share_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Div {
        let p = SettingsColors::sharing(cx);
        let Some(draft) = self.share_draft.clone() else {
            return div();
        };
        let host = self.share_inputs[1].read(cx).value().trim().to_owned();
        let url = format!("http://{host}/sub/{}", draft.token);
        self.share_inputs[2].update(cx, |state, cx| state.set_value(url.clone(), window, cx));
        self.share_inputs[0].update(cx, |state, cx| {
            state.set_placeholder(tr(cx, "例如:手机代理订阅"), window, cx)
        });
        let input = |ix| {
            text_input(&self.share_inputs[ix])
                .border_focus()
                .text_size(px(t::BODY))
                .font_weight(super::super::theme::MISANS_REGULAR)
                .bg(p.solid)
                .border_color(p.line)
                .rounded(px(t::SUBSCRIPTION_SHARE_CANCEL_RADIUS))
                .text_color(p.text)
        };
        let width = px(t::SUBSCRIPTION_SHARE_EDITOR_WIDTH)
            .min(window.viewport_size().width - px(t::SECTION_PADDING * 2.));
        let left = (width
            - px(t::SECTION_PADDING * 2.
                + t::SUBSCRIPTION_SHARE_COLUMNS_GAP
                + t::SUBSCRIPTION_SHARE_BORDER * 2.))
            * t::SUBSCRIPTION_SHARE_LEFT_FRACTION;
        let mut choices = div()
            .id("subscription-share-list")
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .max_h(px(t::SUBSCRIPTION_SHARE_LIST_MAX_HEIGHT))
            .track_scroll(&self.share_scroll);
        for (ix, item) in self.model.list.iter().enumerate() {
            let id = veyra_core::domain::SubscriptionId(item.id.clone());
            let checked = draft.subscription_ids.contains(&id);
            choices = choices.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(t::GAP))
                    .min_h(px(t::SUBSCRIPTION_SHARE_ROW_HEIGHT))
                    .border_b_1()
                    .border_color(p.text.opacity(0.1))
                    .child(
                        gpui_kit::base::Button::new(("subscription-share-check", ix))
                            .accessibility_label(item.name.clone())
                            .size(px(t::SUBSCRIPTION_SHARE_CHECKBOX))
                            // 在线 checkbox-sm 是20px，不保留旧原生 checkbox 的 margin。
                            .ml(px(t::SUBSCRIPTION_SHARE_CHECK_MARGIN))
                            .mr(-px(t::SUBSCRIPTION_SHARE_CHECK_MARGIN / 2.))
                            .border_1()
                            .border_color(p.line)
                            .rounded(px(t::SUBSCRIPTION_SHARE_CHECK_RADIUS))
                            .bg(rgba(0))
                            .text_color(p.text)
                            .when(checked, |b| {
                                // 在线 DaisyUI checkbox 的 12px polygon 旋转45°，不是 Heroicons 细描边。
                                b.child(gpui_kit::component::Icon::default().data(
                                    br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 12 12"><path fill="currentColor" transform="rotate(45 6 6)" d="M2.4 12V9.6H6V0H8.4V12Z"/></svg>"#.as_slice(),
                                ).with_size(px(t::SUBSCRIPTION_SHARE_CHECK_MARK)))
                            })
                            .on_click(cx.listener(move |this, _, _, cx| {
                                if let Some(d) = &mut this.share_draft {
                                    if checked {
                                        d.subscription_ids.retain(|s| s != &id);
                                    } else {
                                        d.subscription_ids.push(id.clone());
                                    }
                                }
                                cx.notify();
                            })),
                    )
                    .child(div().flex_1().child(item.name.clone()))
                    .child(
                        hint(format!("{} {}", item.node_count, tr(cx, "个节点")), p)
                            .text_size(px(t::SUBSCRIPTION_SHARE_META_FONT)),
                    ),
            );
        }
        let copy = url.clone();
        let link = div()
            .flex()
            .child(
                div().flex_1().min_w_0().child(
                    input(2)
                        .readonly(true)
                        // React 在本机 SFMono/Consolas 不可用时实际解析为 Menlo。
                        .font_family("Menlo")
                        .text_size(px(t::BODY))
                        .rounded_r(px(0.)),
                ),
            )
            .child(
                icon_action(
                    "subscription-share-copy",
                    "复制分享链接",
                    "ClipboardDocument",
                    p,
                    cx,
                )
                .w(px(t::SUBSCRIPTION_SHARE_COPY_WIDTH))
                .h(px(t::SUBSCRIPTION_SHARE_LINK_HEIGHT))
                .border_1()
                .border_color(p.line)
                .rounded_l(px(0.))
                .bg(p.base200())
                .on_click(move |_, _, cx| {
                    cx.write_to_clipboard(ClipboardItem::new_string(copy.clone()))
                }),
            );
        let qr = div()
            .size(px(t::SUBSCRIPTION_SHARE_QR_SIZE))
            .p(px(t::SUBSCRIPTION_SHARE_QR_PADDING))
            .bg(rgb(0xffffff))
            .rounded(px(t::SUBSCRIPTION_SHARE_QR_RADIUS))
            .child(qr_code(&url));
        div()
            .font_weight(super::super::theme::MISANS_REGULAR)
            .text_size(px(t::SUBSCRIPTION_SHARE_BODY_FONT))
            .flex()
            .gap(px(t::SUBSCRIPTION_SHARE_COLUMNS_GAP))
            .w_full()
            .min_h(px(t::SUBSCRIPTION_SHARE_BODY_HEIGHT))
            .child(
                div()
                    .w(left)
                    .flex_shrink_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .pb(px(t::GAP))
                            .border_b_1()
                            .border_color(p.text.opacity(0.1))
                            .text_size(px(t::SUBSCRIPTION_SHARE_FIELD_FONT))
                            .line_height(px(t::SECTION_PADDING))
                            .font_weight(super::super::theme::MISANS_MEDIUM)
                            .child(tr(cx, "选择要分享的订阅")),
                    )
                    .child(choices)
                    .child(share_field(tr(cx, "分享监听地址"), input(3)).mt(px(t::GAP)))
                    .child(
                        hint(
                            tr(
                                cx,
                                "仅提供 HTTP。127.0.0.1 仅本机；局域网可监听 0.0.0.0，右侧填写本机局域网 IP 与相同端口。",
                            ),
                            p,
                        )
                        .w_full()
                        .mt(px(t::GAP))
                        .whitespace_normal(),
                    )
                    .when(self.share_error.is_some(), |left| {
                        left.child(self.share_error_view(cx).mt(px(t::GAP)))
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .gap(px(t::PAD))
                    .child(share_field(tr(cx, "标题"), input(0)))
                    .child(share_field(
                        tr(cx, "域名或 IP"),
                        div()
                            .flex()
                            .child(
                                unavailable_select(
                                    "subscription-share-protocol",
                                    "http://",
                                    t::SUBSCRIPTION_SHARE_PROTOCOL_WIDTH,
                                    p,
                                    cx,
                                )
                                .rounded_r(px(0.))
                                .h(px(t::SUBSCRIPTION_SHARE_LINK_HEIGHT)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .ml(-px(1.))
                                    .child(input(1).font_family("Menlo").rounded_l(px(0.))),
                            ),
                    ))
                    .child(share_field(tr(cx, "分享链接"), link))
                    .child(
                        div()
                            .flex()
                            .justify_center()
                            .pt(px(t::SUBSCRIPTION_SHARE_QR_TOP))
                            .child(qr),
                    ),
            )
    }
}
fn share_error_key(error: ShareError) -> &'static str {
    match error {
        ShareError::Bind => "分享监听失败：地址不可用或端口已占用",
        ShareError::Invalid => "请检查名称、订阅、监听地址与分享端口",
        ShareError::Export => "所选订阅包含无法无损导出的节点",
        _ => "分享操作失败，请重试；原配置已保留",
    }
}
/// 只有实际写操作成功才能清除原操作的失败；读取成功仅更新事实版本。
fn share_failure_after(
    refreshing: bool,
    previous: Option<ShareError>,
    failure: Option<ShareError>,
) -> Option<ShareError> {
    failure.or(if refreshing { previous } else { None })
}
fn share_field(text: &str, input: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .text_size(px(t::SUBSCRIPTION_SHARE_FIELD_FONT))
        .line_height(px(t::SECTION_PADDING))
        .font_weight(super::super::theme::MISANS_MEDIUM)
        .gap(px(t::ROW_GAP))
        .child(text.to_owned())
        .child(input)
}
/// 线上 DaisyUI loading-spinner：24px viewBox、r=9.5、3px圆头描边，2秒旋转与1.5秒变长。
/// GPUI SVG不执行SMIL，直接在同一6秒周期中画等价圆弧；三个真实加载位置共用。
pub(super) fn share_spinner(id: &'static str, extent: f32, color: Hsla) -> impl IntoElement {
    div().size(px(extent)).with_animation(
        id,
        Animation::new(Duration::from_secs(6))
            .repeat()
            .with_easing(|v| v),
        move |element, progress| {
            element.child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        let phase = (progress * 4.).fract();
                        let (dash, offset) = if phase < 0.475 {
                            (42. * phase / 0.475, -16. * phase / 0.475)
                        } else {
                            (42., -16. - 43. * (phase - 0.475) / 0.525)
                        };
                        if dash > 0. {
                            let scale = extent / 24.;
                            let radius = 9.5 * scale;
                            let stroke = 3. * scale;
                            let start = std::f32::consts::TAU * progress * 3. - offset / 9.5;
                            let sweep = dash / 9.5;
                            let center = bounds.origin + point(px(extent / 2.), px(extent / 2.));
                            let at = |angle: f32| {
                                center + point(px(radius * angle.cos()), px(radius * angle.sin()))
                            };
                            let from = at(start);
                            let to = at(start + sweep);
                            let mut path = PathBuilder::stroke(px(stroke));
                            path.move_to(from);
                            path.arc_to(
                                point(px(radius), px(radius)),
                                px(0.),
                                sweep > std::f32::consts::PI,
                                true,
                                to,
                            );
                            window.paint_path(path.build().expect("valid loading SVG arc"), color);
                            for end in [from, to] {
                                window.paint_quad(
                                    fill(
                                        Bounds::new(
                                            end - point(px(stroke / 2.), px(stroke / 2.)),
                                            size(px(stroke), px(stroke)),
                                        ),
                                        color,
                                    )
                                    .corner_radii(px(stroke / 2.)),
                                );
                            }
                        }
                    },
                )
                .size(px(extent)),
            )
        },
    )
}

fn qr_code(url: &str) -> AnyElement {
    let Ok(code) = qrcode::QrCode::with_error_correction_level(url.as_bytes(), qrcode::EcLevel::M)
    else {
        return div().into_any_element();
    };
    // 编码完整 URL；线上 PNG 在4px容器 padding内另有一模块 quiet zone，实拍黑区约160px。
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            window.paint_quad(fill(bounds, rgb(0xffffff)));
            let n = code.width();
            let quiet = t::SUBSCRIPTION_SHARE_QR_QUIET_MODULES;
            let cell = bounds.size.width / (n as f32 + quiet * 2.);
            for y in 0..n {
                for x in 0..n {
                    if code[(x, y)] == qrcode::Color::Dark {
                        window.paint_quad(fill(
                            Bounds::new(
                                bounds.origin
                                    + point(cell * (x as f32 + quiet), cell * (y as f32 + quiet)),
                                size(cell, cell),
                            ),
                            rgb(0x000000),
                        ));
                    }
                }
            }
        },
    )
    .size(px(
        t::SUBSCRIPTION_SHARE_QR_SIZE - t::SUBSCRIPTION_SHARE_QR_PADDING * 2.
    ))
    .into_any_element()
}

#[cfg(test)]
mod tests {
    /// 保护失败保存/删除不会被读取成功误报为完成；再次实际保存成功才清错。
    #[test]
    fn sharing_refresh_does_not_report_failed_action_as_success() {
        use super::share_failure_after;
        use veyra_core::application::shares::ShareError;
        for failed_action in [ShareError::Bind, ShareError::Storage, ShareError::NotFound] {
            assert_eq!(
                share_failure_after(true, Some(failed_action), None),
                Some(failed_action)
            );
            assert_eq!(share_failure_after(false, Some(failed_action), None), None);
        }
        assert_eq!(
            share_failure_after(true, Some(ShareError::Bind), Some(ShareError::Storage)),
            Some(ShareError::Storage)
        );
        assert_eq!(
            share_failure_after(false, None, Some(ShareError::Bind)),
            Some(ShareError::Bind)
        );
    }
    /// 保护三个语言的分享动作、错误与地址语义都有明确翻译。
    #[test]
    fn sharing_labels_have_three_languages() {
        use veyra_core::domain::DesktopLanguage::*;
        for key in [
            "分享监听地址",
            "编辑订阅分享",
            "停用订阅分享",
            "启用订阅分享",
            "复制分享链接",
            "重新生成分享链接",
            "修改订阅分享",
            "删除订阅分享",
            "分享监听失败：地址不可用或端口已占用",
            "请检查名称、订阅、监听地址与分享端口",
            "所选订阅包含无法无损导出的节点",
            "分享操作失败，请重试；原配置已保留",
            "确定删除订阅分享「{name}」吗？删除后链接将失效。",
            "重新生成「{name}」的链接？旧链接会立即失效，已经导入的设备要重新添加。",
            "仅提供 HTTP。127.0.0.1 仅本机；局域网可监听 0.0.0.0，右侧填写本机局域网 IP 与相同端口。",
        ] {
            assert_ne!(crate::ui::i18n::translate(English, key), key, "{key}");
            assert_ne!(
                crate::ui::i18n::translate(TraditionalChinese, key),
                key,
                "{key}"
            );
        }
    }
    /// 列表新增图标必须在真实资源表注册，防止保存后渲染崩溃回归。
    #[test]
    fn sharing_action_icons_are_registered() {
        for name in [
            "Power",
            "ClipboardDocument",
            "ArrowPathRoundedSquare",
            "PencilSquare",
            "Trash",
            "Check",
        ] {
            let _ = super::super::super::icons::icon(name, 16.);
        }
    }
}
