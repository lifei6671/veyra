//! OpenBox shell tokens and Panel Settings' separate dark override.
use gpui_kit::{
    component::{Theme, ThemeMode},
    *,
};
use veyra_core::domain::{DesktopThemeMode, DesktopVisualPreferences};
#[derive(Clone, Copy)]
pub struct Palette {
    pub window: u32,
    pub surface: u32,
    pub sidebar: u32,
    pub text: u32,
    pub muted: u32,
    pub field: u32,
    pub line: u32,
    pub active: u32,
}
impl Palette {
    pub fn new(dark: bool, panel: bool) -> Self {
        if dark && panel {
            Self {
                window: 0x1b1818,
                surface: 0x242020,
                sidebar: 0x171313,
                text: 0xcac9c9,
                muted: 0x928f8f,
                field: 0x201c1c,
                line: 0x393333,
                active: 0x1f352d,
            }
        } else if dark {
            Self {
                window: 0x1e2324,
                surface: 0x2a3030,
                sidebar: 0x222828,
                text: 0xedf2f0,
                muted: 0x98a3a3,
                field: 0x303636,
                line: 0x444b4b,
                active: 0x1f352d,
            }
        } else {
            Self {
                window: 0xffffff,
                surface: 0xffffff,
                sidebar: 0xf7f8f8,
                text: 0x4b5263,
                muted: 0x858995,
                field: 0xf7f7f8,
                line: 0xe0e1e4,
                active: 0xe9f6ee,
            }
        }
    }
}
pub fn apply(
    prefs: &DesktopVisualPreferences,
    panel: bool,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let dark = match prefs.theme_mode {
        DesktopThemeMode::System => matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        ),
        DesktopThemeMode::Dark => true,
        DesktopThemeMode::Light => false,
    };
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        Some(window),
        cx,
    );
    let p = Palette::new(dark, panel);
    Theme::update(cx, |t| {
        t.font_size = px(14.);
        t.radius = px(9.);
        t.radius_lg = px(prefs.global_radius as f32);
        t.shadow = false;
        t.colors.background = rgb(p.window).into();
        t.colors.foreground = rgb(p.text).into();
        t.colors.input = rgb(p.field).into();
        t.colors.border = rgb(p.line).into();
        t.colors.muted_foreground = rgb(p.muted).into();
        t.colors.popover = rgb(p.surface).into();
        t.colors.popover_foreground = rgb(p.text).into();
        t.colors.primary = rgb(0x70c996).into();
        t.colors.primary_foreground = rgb(0x183c29).into();
        t.colors.ring = rgb(0x70c996).into();
        t.colors.accent = rgb(p.active).into();
        t.colors.accent_foreground = rgb(p.text).into();
        t.colors.button = rgb(p.field).into();
        t.colors.button_foreground = rgb(p.text).into();
        t.colors.button_primary = rgb(0x70c996).into();
        t.colors.button_primary_foreground = rgb(0x183c29).into();
        t.colors.button_primary_hover = rgb(0x5ab981).into();
        t.colors.button_primary_active = rgb(0x4daa74).into();
        t.colors.slider_bar = rgb(0x70c996).into();
        t.colors.slider_thumb = rgb(0x70c996).into();
    });
    // Native backend offers material blur, not a calibrated CSS radius. Intensity is persisted only.
    window.set_background_appearance(if prefs.background_blur == 0 {
        WindowBackgroundAppearance::Opaque
    } else {
        WindowBackgroundAppearance::Blurred
    });
    dark
}
