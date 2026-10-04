//! UI-thread AppKit adapters. Panels use asynchronous completion/sheets, never runModal.
use super::PlatformError;
use block2::RcBlock;
use objc2::{MainThreadMarker, rc::Retained, runtime::ProtocolObject};
use objc2_app_kit::{
    NSApplication, NSModalResponseOK, NSOpenPanel, NSPasteboard, NSPasteboardItem,
    NSPasteboardWriting, NSSavePanel, NSWorkspace,
};
use objc2_foundation::{NSArray, NSData, NSString, NSURL};
use std::{cell::Cell, path::PathBuf};
use tokio::sync::oneshot;
fn marker() -> MainThreadMarker {
    MainThreadMarker::new().expect("AppKit adapter must run on UI main thread")
}
#[derive(Clone, Copy, Debug)]
pub enum DialogKind {
    Image,
    SaveEvidence,
}
#[derive(Debug, Eq, PartialEq)]
pub enum DialogResult {
    Cancelled,
    Selected(PathBuf),
}
impl DialogResult {
    pub fn save_fixture(self) -> Result<bool, PlatformError> {
        match self {
            Self::Cancelled => Ok(false),
            Self::Selected(path) => {
                super::files::save_selected(&path, super::files::EXPORT_FIXTURE)?;
                Ok(true)
            }
        }
    }
}
// This existing AppKit API supports PNG/JPEG filtering without another dependency.
#[allow(deprecated)]
pub fn begin_dialog(kind: DialogKind) -> oneshot::Receiver<Result<DialogResult, PlatformError>> {
    let mtm = marker();
    let (tx, rx) = oneshot::channel();
    let panel: Retained<NSSavePanel> = match kind {
        DialogKind::Image => {
            let open = NSOpenPanel::openPanel(mtm);
            open.setCanChooseDirectories(false);
            open.setCanChooseFiles(true);
            open.setAllowsMultipleSelection(false);
            open.setResolvesAliases(false);
            open.setAllowedFileTypes(Some(&NSArray::from_retained_slice(&[
                NSString::from_str("png"),
                NSString::from_str("jpg"),
                NSString::from_str("jpeg"),
            ])));
            open.setAllowsOtherFileTypes(false);
            open.into_super()
        }
        DialogKind::SaveEvidence => {
            let save = NSSavePanel::savePanel(mtm);
            save.setNameFieldStringValue(&NSString::from_str("veyra-platform-evidence.txt"));
            save
        }
    };
    #[cfg(debug_assertions)]
    if let Some(root) = std::env::var_os("VEYRA_DESKTOP_ROOT_OVERRIDE")
        .or_else(|| std::env::var_os("VEYRA_DESKTOP_STATE_ROOT"))
    {
        panel.setDirectoryURL(Some(&NSURL::fileURLWithPath(&NSString::from_str(
            &PathBuf::from(root).to_string_lossy(),
        ))));
    }
    let tx = Cell::new(Some(tx));
    let callback = RcBlock::new({
        let panel = panel.clone();
        move |response: isize| {
            let result = if response == NSModalResponseOK {
                panel
                    .URL()
                    .and_then(|u| u.to_file_path())
                    .map(DialogResult::Selected)
                    .ok_or(PlatformError::FileDialogUnavailable)
            } else if response == 0 {
                Ok(DialogResult::Cancelled)
            } else {
                Err(PlatformError::FileDialogUnavailable)
            };
            if let Some(tx) = tx.take() {
                let _ = tx.send(result);
            }
        }
    });
    if let Some(window) = NSApplication::sharedApplication(mtm).keyWindow() {
        panel.beginSheetModalForWindow_completionHandler(&window, &callback);
    } else {
        panel.beginWithCompletionHandler(&callback);
    }
    rx
}
/// 只读 native 事实：隐藏窗口仍保留在 NSApplication 中，不销毁/重建。
#[cfg(debug_assertions)]
pub fn log_window_visibility() {
    let app = NSApplication::sharedApplication(marker());
    eprintln!(
        "native hidden={} window_count={} active_key={}",
        app.isHidden(),
        app.windows().len(),
        primary_focus_confirmed()
    );
}
pub fn primary_focus_confirmed() -> bool {
    let app = NSApplication::sharedApplication(marker());
    app.isActive() && app.keyWindow().is_some_and(|window| window.isKeyWindow())
}
pub struct ExternalUrl(String);
impl ExternalUrl {
    pub fn new(value: &str) -> Result<Self, PlatformError> {
        let url = NSURL::URLWithString(&NSString::from_str(value))
            .ok_or(PlatformError::ExternalUrlRejected)?;
        if !url
            .scheme()
            .is_some_and(|s| matches!(s.to_string().as_str(), "http" | "https"))
            || url.host().is_none()
            || url.user().is_some()
            || url.password().is_some()
        {
            return Err(PlatformError::ExternalUrlRejected);
        }
        Ok(Self(value.into()))
    }
    pub fn open(&self) -> Result<(), PlatformError> {
        let _ = marker();
        let url = NSURL::URLWithString(&NSString::from_str(&self.0))
            .ok_or(PlatformError::ExternalUrlRejected)?;
        if NSWorkspace::sharedWorkspace().openURL(&url) {
            Ok(())
        } else {
            Err(PlatformError::ExternalUrlRejected)
        }
    }
}
fn text_type() -> Retained<NSString> {
    NSString::from_str("public.utf8-plain-text")
}
pub fn read_text() -> Result<Option<String>, PlatformError> {
    let _ = marker();
    Ok(NSPasteboard::generalPasteboard()
        .stringForType(&text_type())
        .map(|s| s.to_string()))
}
pub fn write_text(value: &str) -> Result<(), PlatformError> {
    let _ = marker();
    let p = NSPasteboard::generalPasteboard();
    p.clearContents();
    if p.setString_forType(&NSString::from_str(value), &text_type()) {
        Ok(())
    } else {
        Err(PlatformError::ClipboardUnavailable)
    }
}
/// Materializes EVERY item/type before clearing; original contents never leave memory/logs.
pub struct ClipboardSnapshot {
    items: Vec<Retained<NSPasteboardItem>>,
    restored: bool,
}
impl ClipboardSnapshot {
    pub fn capture() -> Result<Self, PlatformError> {
        let _ = marker();
        let mut result = vec![];
        if let Some(items) = NSPasteboard::generalPasteboard().pasteboardItems() {
            for item in items.iter() {
                let copied = NSPasteboardItem::new();
                for kind in item.types().iter() {
                    let data = item
                        .dataForType(&kind)
                        .ok_or(PlatformError::ClipboardUnavailable)?;
                    let data = NSData::with_bytes(&data.to_vec());
                    if !copied.setData_forType(&data, &kind) {
                        return Err(PlatformError::ClipboardUnavailable);
                    }
                }
                result.push(copied);
            }
        }
        Ok(Self {
            items: result,
            restored: false,
        })
    }
    pub fn restore(&mut self) -> Result<(), PlatformError> {
        let _ = marker();
        let p = NSPasteboard::generalPasteboard();
        let objects: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = self
            .items
            .iter()
            .map(|v| ProtocolObject::from_ref::<NSPasteboardItem>(v))
            .collect();
        p.clearContents();
        if objects.is_empty() || p.writeObjects(&NSArray::from_slice(&objects)) {
            self.restored = true;
            Ok(())
        } else {
            Err(PlatformError::ClipboardUnavailable)
        }
    }
}
impl Drop for ClipboardSnapshot {
    fn drop(&mut self) {
        if !self.restored {
            let _ = self.restore();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn external_url_closed_schemes_without_handoff() {
        for url in ["http://example.invalid", "https://example.invalid/path"] {
            assert!(ExternalUrl::new(url).is_ok());
        }
        for url in [
            "file:///tmp/x",
            "javascript:alert(1)",
            "data:text/plain,x",
            "veyra:arbitrary",
            "https://user:secret@example.invalid",
        ] {
            assert!(ExternalUrl::new(url).is_err());
        }
    }
    #[test]
    fn cancelled_save_and_invalid_parent_never_publish_file() {
        assert_eq!(DialogResult::Cancelled.save_fixture(), Ok(false));
        assert_eq!(
            DialogResult::Selected(std::path::PathBuf::from(
                "/private/tmp/veyra-p105-missing-parent/export"
            ))
            .save_fixture(),
            Err(PlatformError::FileWriteFailed)
        );
    }
}
