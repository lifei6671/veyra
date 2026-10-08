//! 单窗口通知展示：复用 Kit base 的 Alert 语义，外观和关闭按钮完全由 React token 定义。
use gpui_kit::{base::Toast, component::ActiveTheme, prelude::*, *};
#[derive(Clone, Copy)]
pub enum Notice {
    Saving,
    Success,
    Error,
}
struct Item {
    message: SharedString,
}
pub struct NoticeCenter {
    item: Option<Item>,
    timer: Option<Task<()>>,
}
#[derive(Clone)]
struct NoticeHost(Entity<NoticeCenter>);
impl Global for NoticeHost {}
impl NoticeCenter {
    pub fn mount(cx: &mut App) -> Entity<Self> {
        let entity = cx.new(|_| Self {
            item: None,
            timer: None,
        });
        cx.set_global(NoticeHost(entity.clone()));
        entity
    }
}
pub fn notify(kind: Notice, message: impl Into<SharedString>, _: &mut Window, cx: &mut App) {
    notify_app(kind, message, cx);
}
/// Worker 完成后同样发布到既有通知中心，无需捕获 Window。
pub fn notify_app(_kind: Notice, message: impl Into<SharedString>, cx: &mut App) {
    let entity = cx.global::<NoticeHost>().0.clone();
    entity.update(cx, |this, cx| {
        this.item = Some(Item {
            message: message.into(),
        });
        // 与旧通知相同的单一身份：新结果取代保存中，旧任务随句柄释放取消。
        this.timer = Some(cx.spawn(async |view, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(4))
                .await;
            let _ = view.update(cx, |view, cx| {
                view.item = None;
                cx.notify();
            });
        }));
        cx.notify();
    });
}
impl Render for NoticeCenter {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(item) = &self.item else {
            return div().into_any_element();
        };
        let dark = cx.theme().mode.is_dark();
        // OpenBoxApp.showToast 调用 toast(message)：Panel 通知没有类型图标。
        let accent = 0x4d9d72;
        let (from, to) = if dark {
            (0x26302d, 0x1f392e)
        } else {
            (0xf8fdfa, 0xe4f4ec)
        };
        let text = if dark { 0xedf2f0 } else { 0x4b5263 };
        Toast::new("app-notice")
            .absolute()
            .top(px(24.))
            .left((window.viewport_size().width - px(380.)) / 2.)
            .w(px(380.))
            .rounded(px(14.))
            .border_1()
            .border_color(rgba(if dark { 0x7ccd9d4d } else { 0x4d9d724d }))
            .shadow(vec![
                BoxShadow::new(px(4.), px(0.), rgb(accent).into()).inset(),
                BoxShadow::new(
                    px(0.),
                    px(if dark { 20. } else { 18. }),
                    rgba(if dark { 0x0000006b } else { 0x2343362e }).into(),
                )
                .blur_radius(px(if dark { 52. } else { 46. })),
                BoxShadow::new(
                    px(0.),
                    px(4.),
                    rgba(if dark { 0x0000003d } else { 0x23433617 }).into(),
                )
                .blur_radius(px(if dark { 16. } else { 14. })),
            ])
            .bg(linear_gradient(
                135.,
                linear_color_stop(rgba((from << 8) | 0xfa), 0.),
                linear_color_stop(rgba((to << 8) | 0xfa), 1.),
            ))
            .py(px(14.))
            .pl(px(16.))
            .pr(px(42.))
            .text_size(px(14.))
            .line_height(px(20.))
            .font_weight(FontWeight(650.))
            .text_color(rgb(text))
            .child(crate::ui::i18n::message(cx, &item.message))
            .child(
                gpui_kit::base::Button::new("dismiss-notice")
                    .accessibility_label(crate::ui::i18n::tr(cx, "关闭通知"))
                    .absolute()
                    .top(px(9.))
                    .right(px(9.))
                    .size(px(22.))
                    .rounded_full()
                    .border_1()
                    .border_color(rgba(0x4d9d724d))
                    .bg(rgba(if dark { 0x2a3030c7 } else { 0xffffffc7 }))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .child(crate::ui::icons::icon("XMark", 12.).text_color(rgb(text)))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.item = None;
                        this.timer = None;
                        cx.notify();
                    })),
            )
            // Sonner 通知位于弹窗/下拉层之上；失败时编辑器仍打开，不能被遮住。
            .map(|toast| deferred(toast).with_priority(gpui_kit::base::POPUP_PRIORITY + 1))
            .into_any_element()
    }
}
