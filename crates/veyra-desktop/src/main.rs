mod app;
mod application_icon;
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
    // debug 本地证据将 stderr 写入指定文件，实际 bundle 启动也能留存主线程诊断。
    #[cfg(all(debug_assertions, target_os = "macos"))]
    if let Some(path) = std::env::var_os("VEYRA_DESKTOP_EVIDENCE_LOG") {
        use std::os::fd::AsRawFd;
        let log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        // dup2 复制 FD 所有权；文件在此作用域关闭后 stderr 仍有效。
        if unsafe { libc::dup2(log.as_raw_fd(), libc::STDERR_FILENO) } < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
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
        cx.set_global(ui::i18n::Locale::default());
        platform::macos::set_application_icon();
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
                // 保留桌面布局：内容区宽度不得进入原版 <=768px 的手机断点。
                // 保留桌面侧栏和底部状态区所需空间；设置内容继续纵向滚动。
                window_min_size: Some(size(px(769.), px(600.))),
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
                // macOS 初始 centered bounds 会包含标题栏取整；明确设置应用内容区。
                window.resize(size(px(1280.), px(720.)));
                window.on_next_frame(|window, _| {
                    window.on_next_frame(|window, _| {
                        eprintln!(
                            "logical content {:?}; scale factor {}",
                            window.viewport_size(),
                            window.scale_factor()
                        );
                    });
                });
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
