//! Application focus return contract; Kit Root owns the actual overlay stack.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EscapeTarget {
    Dropdown,
    Modal,
    None,
}
pub fn escape_target(dropdown_open: bool, modal_open: bool) -> EscapeTarget {
    if dropdown_open {
        EscapeTarget::Dropdown
    } else if modal_open {
        EscapeTarget::Modal
    } else {
        EscapeTarget::None
    }
}
#[derive(Clone)]
pub struct ModalFocus<T> {
    trigger: Option<T>,
}
impl<T: Clone> ModalFocus<T> {
    pub fn capture(trigger: Option<T>) -> Self {
        Self { trigger }
    }
    pub fn return_to(&self) -> Option<T> {
        self.trigger.clone()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn topmost_escape_and_focus_return_across_reopen() {
        let focus = ModalFocus::capture(Some("modal-trigger"));
        assert_eq!(escape_target(true, true), EscapeTarget::Dropdown);
        assert_eq!(escape_target(false, true), EscapeTarget::Modal);
        assert_eq!(focus.return_to(), Some("modal-trigger"));
        assert_eq!(escape_target(false, false), EscapeTarget::None);
        let reopened = ModalFocus::capture(focus.return_to());
        assert_eq!(reopened.return_to(), Some("modal-trigger"));
    }
}
