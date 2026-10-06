//! 主线程长期持有 TrayIcon；菜单 callback 仅将固定意图送往 GPUI 前台。
use crate::desktop_lifecycle::Intent;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};
use veyra_core::{
    application::runtime_snapshot::{RuntimeSnapshot, RuntimeStatus},
    domain::{AppState, ConfigVersion},
};

/// 纯投影：不存在 desktop Runtime bool。None 表示 P2 owner 尚未接通。
#[derive(Debug, Eq, PartialEq)]
struct Projection {
    text: &'static str,
    saved_version: Option<ConfigVersion>,
}
impl Projection {
    fn from_core(state: Option<&AppState>, runtime: Option<&RuntimeSnapshot>) -> Self {
        Self {
            text: match runtime.map(|r| r.status) {
                None => "内核：未接入（当前不可用）",
                Some(RuntimeStatus::Stopped) => "内核：未运行",
                Some(RuntimeStatus::Starting) => "内核：正在启动",
                Some(RuntimeStatus::Ready) => "内核：运行中",
                Some(RuntimeStatus::Recovering) => "内核：恢复中",
                Some(RuntimeStatus::Failed) => "内核：失败",
            },
            saved_version: state.map(AppState::config_version),
        }
    }
}

/// 托盘展示缓存：project 是投影 key 的唯一写入口，换语言只重译这个 key。
struct TrayPresentation {
    key: std::cell::Cell<&'static str>,
    language: std::cell::Cell<veyra_core::domain::DesktopLanguage>,
}
impl Default for TrayPresentation {
    fn default() -> Self {
        Self {
            key: std::cell::Cell::new(Projection::from_core(None, None).text),
            language: std::cell::Cell::new(Default::default()),
        }
    }
}
impl TrayPresentation {
    fn text(&self) -> &'static str {
        crate::ui::i18n::translate(self.language.get(), self.key.get())
    }
    fn set_language(&self, language: veyra_core::domain::DesktopLanguage) -> &'static str {
        self.language.set(language);
        self.text()
    }
    fn project(&self, state: Option<&AppState>, runtime: Option<&RuntimeSnapshot>) -> &'static str {
        self.key.set(Projection::from_core(state, runtime).text);
        self.text()
    }
}

pub struct DesktopTray {
    tray: Option<TrayIcon>,
    status: MenuItem,
    localized_items: [(MenuItem, &'static str); 4],
    presentation: TrayPresentation,
}
impl DesktopTray {
    pub fn new() -> Result<(Self, UnboundedReceiver<Intent>), Box<dyn std::error::Error>> {
        // 与 GPUI run 同一 macOS 主线程，不另开 UI event loop。
        assert!(objc2::MainThreadMarker::new().is_some());
        let menu = Menu::new();
        let show = MenuItem::new("显示窗口", true, None);
        let status = MenuItem::new(Projection::from_core(None, None).text, false, None);
        // P1 没有可执行的 Runtime contract，禁止制造模拟启停状态。
        let start = MenuItem::new("启动（Runtime 未接入）", false, None);
        let stop = MenuItem::new("停止（Runtime 未接入）", false, None);
        let quit = MenuItem::new("退出", true, None);
        menu.append_items(&[&show, &status, &start, &stop, &quit])?;
        // 同源彩色图案；36px 提供 Retina 像素，tray-icon 以 18pt 显示。
        let rgba = crate::application_icon::image(36, 36).into_raw();
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(tray_icon::Icon::from_rgba(rgba, 36, 36)?)
            .with_icon_as_template(false)
            .with_tooltip("Veyra · 显示窗口")
            .build()?;
        let show_id = show.id().clone();
        let quit_id = quit.id().clone();
        let (sender, receiver) = unbounded_channel();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            let intent = if event.id == show_id {
                Intent::Show
            } else if event.id == quit_id {
                Intent::Quit
            } else {
                return;
            };
            let _ = sender.send(intent);
        }));
        eprintln!("TrayIcon created main_thread=true; Runtime unavailable; start/stop disabled");
        Ok((
            Self {
                tray: Some(tray),
                status,
                localized_items: [
                    (show, "显示窗口"),
                    (start, "启动（Runtime 未接入）"),
                    (stop, "停止（Runtime 未接入）"),
                    (quit, "退出"),
                ],
                presentation: TrayPresentation::default(),
            },
            receiver,
        ))
    }
    pub fn set_language(&self, language: veyra_core::domain::DesktopLanguage) {
        let status = self.presentation.set_language(language);
        for (item, key) in &self.localized_items {
            item.set_text(crate::ui::i18n::translate(language, key));
        }
        self.status.set_text(status);
        if let Some(tray) = &self.tray {
            let _ = tray.set_tooltip(Some(crate::ui::i18n::translate(
                language,
                "Veyra · 显示窗口",
            )));
        }
    }
    pub fn project(&self, state: Option<&AppState>, runtime: Option<&RuntimeSnapshot>) {
        self.status
            .set_text(self.presentation.project(state, runtime));
        // 仅诊断 OS 托盘可见性；创建成功不能替代用户实际看到菜单栏图标。
        #[cfg(debug_assertions)]
        if let Some(tray) = &self.tray {
            eprintln!(
                "tray native rect={:?} visible={:?}",
                tray.rect(),
                tray.ns_status_item().map(|item| item.isVisible())
            );
        }
    }
    /// 创建后延迟采样原生几何；只观察自有 status item，不扫描其他应用或改系统设置。
    #[cfg(debug_assertions)]
    pub fn diagnose(&self) {
        use objc2_app_kit::{NSApplication, NSStatusBar};
        let mtm = objc2::MainThreadMarker::new().expect("tray probe main thread");
        let app = NSApplication::sharedApplication(mtm);
        let options = app.currentSystemPresentationOptions();
        eprintln!(
            "tray probe delayed main_thread=true activation_policy={:?} presentation={:?} thickness={}",
            app.activationPolicy(),
            options,
            NSStatusBar::systemStatusBar().thickness()
        );
        let Some(tray) = &self.tray else {
            eprintln!("tray probe absent");
            return;
        };
        let Some(item) = tray.ns_status_item() else {
            eprintln!("tray probe status item absent");
            return;
        };
        let button = item.button(mtm);
        eprintln!(
            "tray probe visible={} button_exists={} rect={:?}",
            item.isVisible(),
            button.is_some(),
            tray.rect()
        );
        if let Some(button) = button {
            let window = button.window();
            eprintln!(
                "tray probe button.frame={:?} window.frame={:?}",
                button.frame(),
                window.as_ref().map(|w| w.frame())
            );
            if let Some(screen) = window.and_then(|w| w.screen()) {
                eprintln!(
                    "tray probe screen.frame={:?} visibleFrame={:?}",
                    screen.frame(),
                    screen.visibleFrame()
                );
            }
        }
        // 公共 AppKit presentation flags 只反映当前 app。全局自动隐藏/菜单栏展开由真人确认。
    }
    /// NSApplication terminate 不展开 main stack，退出 hook 必须显式释放原生托盘。
    pub fn prepare_quit(&mut self) {
        self.tray.take();
        eprintln!("TrayIcon released on main thread");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veyra_core::application::runtime_snapshot::{
        InstanceId, LastSuccessfulVersion, RuntimeFacts, RuntimeState,
    };
    // 展示缓存只接受 project 的 key；换语言仅翻译缓存，不重新推断运行状态。
    #[test]
    fn cached_runtime_projection_keeps_status_across_languages() {
        use veyra_core::domain::DesktopLanguage;
        let state = AppState::empty();
        let id = InstanceId("tray-language-test".into());
        for (current, zh, en) in [
            (
                RuntimeState::Ready {
                    instance_id: id.clone(),
                    applied_version: state.config_version(),
                },
                "内核：运行中",
                "Core: running",
            ),
            (
                RuntimeState::Failed { instance_id: None },
                "内核：失败",
                "Core: failed",
            ),
            (RuntimeState::Stopped, "内核：未运行", "Core: stopped"),
            (
                RuntimeState::Starting { instance_id: id },
                "内核：正在启动",
                "Core: starting",
            ),
            (
                RuntimeState::Recovering { instance: None },
                "内核：恢复中",
                "Core: recovering",
            ),
        ] {
            let runtime = RuntimeSnapshot::from_owner(
                state.config_version(),
                &RuntimeFacts {
                    current,
                    last_successful: None,
                },
            );
            let cache = TrayPresentation::default();
            assert_eq!(cache.project(Some(&state), Some(&runtime)), zh);
            for (lang, expected) in [
                (DesktopLanguage::SimplifiedChinese, zh),
                (DesktopLanguage::English, en),
            ] {
                assert_eq!(cache.set_language(lang), expected);
                assert_eq!(cache.key.get(), zh);
            }
        }
    }
    #[test]
    fn no_runtime_owner_never_projects_running_even_with_saved_configuration() {
        let mut state = AppState::try_empty().unwrap();
        let absent = Projection::from_core(Some(&state), None);
        assert_eq!(absent.text, "内核：未接入（当前不可用）");
        state.config_revision += 1;
        let changed = Projection::from_core(Some(&state), None);
        assert_eq!(changed.text, absent.text);
        assert_ne!(changed.saved_version, absent.saved_version);
    }
    #[test]
    fn menu_projects_core_snapshot_and_does_not_infer_ready_from_history() {
        let state = AppState::try_empty().unwrap();
        let instance_id = InstanceId("tray-unit-instance".into());
        for (current, text) in [
            (RuntimeState::Stopped, "内核：未运行"),
            (
                RuntimeState::Starting {
                    instance_id: instance_id.clone(),
                },
                "内核：正在启动",
            ),
            (
                RuntimeState::Ready {
                    instance_id: instance_id.clone(),
                    applied_version: state.config_version(),
                },
                "内核：运行中",
            ),
            (RuntimeState::Recovering { instance: None }, "内核：恢复中"),
            (RuntimeState::Failed { instance_id: None }, "内核：失败"),
        ] {
            // 历史成功配置不能把当前 Stopped 或缺失 owner 投影成运行中。
            let snapshot = RuntimeSnapshot::from_owner(
                state.config_version(),
                &RuntimeFacts {
                    current,
                    last_successful: Some(LastSuccessfulVersion {
                        config: state.config_version(),
                        selection: state.selection_version(),
                    }),
                },
            );
            assert_eq!(
                Projection::from_core(Some(&state), Some(&snapshot)).text,
                text
            );
        }
    }
}
