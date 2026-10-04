//! 窗口/托盘/单实例共享意图。只有明确 Quit 才启动应用退出清理。
use gpui_kit::{App, Window};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Intent {
    Close,
    Show,
    Activate,
    Quit,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    Hide,
    RevealExisting,
    Quit,
}
impl Intent {
    pub fn effect(self) -> Effect {
        match self {
            Self::Close => Effect::Hide,
            Self::Show | Self::Activate => Effect::RevealExisting,
            Self::Quit => Effect::Quit,
        }
    }
}
/// GPUI foreground 专用；此入口不创建窗口、业务服务或 writer。
pub fn dispatch(intent: Intent, window: Option<&mut Window>, cx: &mut App) {
    eprintln!("desktop intent={intent:?} effect={:?}", intent.effect());
    match intent.effect() {
        // GPUI macOS hide 隐藏 app 的窗口，TrayIcon 仍驻留菜单栏；拒绝销毁最后窗口。
        Effect::Hide => cx.hide(),
        Effect::RevealExisting => {
            cx.activate(true);
            if let Some(window) = window {
                eprintln!("restore existing window={:?}", window.window_handle());
                window.activate_window();
            }
        }
        // on_app_quit 负责 P1-05 listener/socket；flock FD 保留到进程退出。
        Effect::Quit => cx.quit(),
    }
    #[cfg(debug_assertions)]
    if intent != Intent::Quit {
        // AppKit hide/activate 完成后做一次事实采样，不以同步调用中的瞬态值下结论。
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(100))
                .await;
            cx.update(|_| {
                eprintln!("visibility after {intent:?}");
                crate::platform::macos::log_window_visibility();
            });
        })
        .detach();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn close_hides_and_only_explicit_quit_enters_cleanup() {
        assert_eq!(Intent::Close.effect(), Effect::Hide);
        for intent in [Intent::Close, Intent::Show, Intent::Activate] {
            assert_ne!(intent.effect(), Effect::Quit);
        }
        assert_eq!(Intent::Quit.effect(), Effect::Quit);
    }
    #[test]
    fn reveal_intents_target_existing_window() {
        assert_eq!(Intent::Show.effect(), Effect::RevealExisting);
        assert_eq!(Intent::Activate.effect(), Effect::RevealExisting);
    }
    // 保护隐藏/恢复期间真实 writer、flock/socket 不变，仅退出允许 IPC cleanup。
    #[test]
    fn repeated_visibility_intents_retain_single_writer_and_ownership_until_quit() {
        use crate::platform::{
            directories::AppDirectories,
            single_instance::{self, InstanceCommand, Ownership},
        };
        let root =
            std::env::temp_dir().join(format!("veyra-p106-lifecycle-{}", std::process::id()));
        let directories = AppDirectories::injected(root.clone());
        let Ownership::Primary(owner, mut activation) =
            single_instance::acquire(&directories).unwrap()
        else {
            panic!()
        };
        let before = crate::services::WRITERS_CREATED.get();
        let (services, _) = crate::services::AppServices::new(root.clone()).unwrap();
        let saved = services.snapshots.snapshot().unwrap();
        for _ in 0..5 {
            assert_eq!(Intent::Close.effect(), Effect::Hide);
            assert!(directories.socket().exists());
            assert!(matches!(
                single_instance::acquire(&directories).unwrap(),
                Ownership::Secondary
            ));
            assert_eq!(activation.try_recv().unwrap(), InstanceCommand::Activate);
            assert_eq!(Intent::Activate.effect(), Effect::RevealExisting);
            assert_eq!(Intent::Show.effect(), Effect::RevealExisting);
            assert_eq!(services.snapshots.snapshot().unwrap(), saved);
            assert_eq!(crate::services::WRITERS_CREATED.get() - before, 1);
        }
        assert_eq!(Intent::Quit.effect(), Effect::Quit);
        owner.prepare_quit();
        assert!(!directories.socket().exists());
        assert!(matches!(
            single_instance::acquire(&directories),
            Err(crate::platform::PlatformError::InstanceBusyUnconfirmed)
        ));
        drop(owner);
        let Ownership::Primary(restarted, _) = single_instance::acquire(&directories).unwrap()
        else {
            panic!()
        };
        drop(restarted);
        drop(services);
        std::fs::remove_dir_all(root).unwrap();
    }
}
