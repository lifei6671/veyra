mod app;
mod behavior_preferences;
mod desktop_lifecycle;
mod navigation;
mod platform;
mod preferences;
mod services;
mod state_bridge;
mod tray;
mod ui;
mod visual_assets;
use gpui_kit::{
    assets::Assets,
    component::{Theme, ThemeMode},
    *,
};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directories = platform::directories::AppDirectories::resolve()?;
    eprintln!(
        "product={} namespace={}",
        platform::directories::PRODUCT_IDENTIFIER,
        platform::directories::PREVIEW_NAMESPACE
    );
    let (ownership, mut activation) = match platform::single_instance::acquire(&directories)? {
        platform::single_instance::Ownership::Secondary => {
            eprintln!("role=secondary Activate acknowledged; writer_created_count=0");
            return Ok(());
        }
        platform::single_instance::Ownership::Primary(owner, receiver) => (owner, receiver),
    };
    let ownership = Arc::new(ownership);
    directories.prepare_primary()?;
    eprintln!("role=primary lock acquired; control socket listening");
    let (services, receiver) = services::AppServices::new(directories.application_support.clone())?;
    eprintln!("writer_created_count=1");
    let services = Arc::new(services);
    let quit_owner = ownership.clone();
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        let (tray, mut tray_events) = tray::DesktopTray::new().expect("desktop tray");
        cx.on_app_quit(move |_| {
            quit_owner.prepare_quit();
            eprintln!("quit cleanup: listener stopped; socket removed; flock retained until exit");
            async {}
        })
        .detach();
        Theme::change(ThemeMode::Light, None, cx);
        let window_handle = gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(size(px(1280.), px(720.)), cx)),
                titlebar: Some(TitlebarOptions {
                    title: Some("Veyra · Desktop".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            cx,
            |window, cx| {
                window.on_window_should_close(cx, |_, cx| {
                    desktop_lifecycle::dispatch(desktop_lifecycle::Intent::Close, None, cx);
                    false
                });
                eprintln!(
                    "logical content 1280x720; scale factor {}",
                    window.scale_factor()
                );
                cx.new(|cx| app::AppView::new(services, receiver, tray, window, cx))
            },
        )
        .expect("desktop window");
        cx.spawn(async move |cx| {
            let mut count = 0u64;
            while activation.recv().await.is_some() {
                count += 1;
                let _ = window_handle.0.update(cx, |_, window, cx| {
                    desktop_lifecycle::dispatch(
                        desktop_lifecycle::Intent::Activate,
                        Some(window),
                        cx,
                    );
                    eprintln!("Activate accepted activation_count={count} window focus requested");
                });
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(100))
                    .await;
                let _ = window_handle.0.update(cx, |_, _, _| {
                    eprintln!(
                        "activation_count={count} primary_front_key={}",
                        platform::macos::primary_focus_confirmed()
                    );
                });
            }
        })
        .detach();
        // 托盘 callback 仅发送类型化意图；recv 直接唤醒 GPUI 前台任务，无定时轮询。
        cx.spawn(async move |cx| {
            while let Some(intent) = tray_events.recv().await {
                let _ = window_handle.0.update(cx, |_, window, cx| {
                    desktop_lifecycle::dispatch(intent, Some(window), cx);
                });
            }
        })
        .detach();
        cx.activate(true);
    });
    drop(ownership);
    Ok(())
}
