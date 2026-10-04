//! Mature Kit controls with Veyra sizing/theme; overlay ownership remains window-level Kit Root.
use gpui_kit::{
    component::{
        Sizable, WindowExt,
        button::Button,
        input::{Input, InputState},
        notification::Notification,
        select::{Select, SelectState},
        switch::Switch,
    },
    *,
};
pub fn text_input(state: &Entity<InputState>) -> Input {
    // Kit Medium defaults to 8 px vertical padding: at 32 px height that
    // leaves only 14 px after borders and clips the 20 px text/IME viewport.
    // Input::h is Kit's multi-line-only API; use Styled height for single-line.
    Styled::h(
        Input::new(state)
            .py(px(4.))
            .text_size(px(14.))
            .line_height(px(20.)),
        px(32.),
    )
}
pub fn select(
    state: &Entity<SelectState<Vec<&'static str>>>,
    label: impl Into<SharedString>,
) -> Select<Vec<&'static str>> {
    Select::new(state)
        .accessibility_label(label)
        .h(px(32.))
        .w(px(192.))
}
pub fn toggle(id: &'static str, checked: bool, label: impl Into<SharedString>) -> Switch {
    Switch::new(id)
        .checked(checked)
        .color(rgb(0x70c996))
        .accessibility_label(label)
        .small()
}
pub fn button(id: &'static str, label: impl Into<SharedString>) -> Button {
    Button::new(id).label(label).h(px(32.)).text_size(px(14.))
}
pub enum Notice {
    Saving,
    Success,
    Error,
}
pub fn notify(kind: Notice, message: impl Into<SharedString>, window: &mut Window, cx: &mut App) {
    let notification = match kind {
        Notice::Saving => Notification::info(message),
        Notice::Success => Notification::success(message),
        Notice::Error => Notification::error(message),
    };
    window.push_notification(notification.id::<Notice>(), cx);
}

/// Stable application identity survives render; GPUI owns gesture/drop/cancellation lifetime.
#[derive(Clone)]
pub struct FileDrag {
    pub id: &'static str,
    pub path: std::path::PathBuf,
}
pub struct DragPreview(pub &'static str);
impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_3()
            .rounded_lg()
            .bg(rgb(0x70c996))
            .text_color(rgb(0x183c29))
            .child(format!("{} → drop background", self.0))
    }
}

pub mod overlay;
