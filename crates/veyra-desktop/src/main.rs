mod app;
mod behavior_preferences;
mod navigation;
mod preferences;
mod services;
mod state_bridge;
mod ui;
mod visual_assets;
use gpui_kit::{
    assets::Assets,
    component::{Theme, ThemeMode},
    *,
};
use std::{path::PathBuf, sync::Arc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // P1 temporary/evidence isolation. P1-05 owns the platform directory decision.
    let explicit_root = std::env::var_os("VEYRA_DESKTOP_STATE_ROOT")
        .or_else(|| std::env::var_os("VEYRA_P1_03_STATE_ROOT"))
        .map(PathBuf::from);
    let root = explicit_root.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("veyra-p1-04b-{}", std::process::id()))
    });
    eprintln!("P1-04B injected temporary state root: {}", root.display());
    let (services, receiver) = services::AppServices::new(root.clone())?;
    let services = Arc::new(services);
    gpui_kit::application().with_assets(Assets).run(move |cx| {
        gpui_kit::init(cx);
        Theme::change(ThemeMode::Light, None, cx);
        gpui_kit::open_window(
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
                    cx.quit();
                    true
                });
                eprintln!(
                    "logical content 1280x720; scale factor {}",
                    window.scale_factor()
                );
                cx.new(|cx| app::AppView::new(services, receiver, window, cx))
            },
        )
        .expect("desktop window");
        cx.activate(true);
    });
    if explicit_root.is_none() && root.exists() {
        std::fs::remove_dir_all(root)?;
    }
    Ok(())
}
