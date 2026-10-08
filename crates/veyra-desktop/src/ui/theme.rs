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
                window: 0x1b1717,
                surface: 0x1b1717,
                sidebar: 0x161212,
                text: 0xcac9c9,
                muted: 0xcac9c9ad,
                field: 0x1b1717,
                line: 0x393333,
                active: 0x1f352d,
            }
        } else if dark {
            Self {
                window: 0x1e2324,
                surface: 0x2a3030,
                sidebar: 0xededed,
                text: 0xedf2f0,
                muted: 0x98a3a3ff,
                field: 0x303636,
                line: 0x444b4b,
                active: 0x1f352d,
            }
        } else {
            Self {
                window: 0xffffff,
                surface: 0xffffff,
                sidebar: if panel {
                    super::tokens::PANEL_BASE200_LIGHT
                } else {
                    0xededed
                },
                text: if panel { 0x333c4d } else { 0x4b5263 },
                muted: 0x4b5263ad,
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
        t.font_size = px(super::tokens::BODY);
        t.radius = px(super::tokens::RADIUS);
        t.radius_lg = px(prefs.global_radius as f32);
        t.shadow = false;
        t.notification.placement = Anchor::TopCenter;
        t.notification.width = px(380.);
        t.notification.margins.top = px(20.);
        t.notification.margins.bottom = px(20.);
        t.colors.overlay = rgba(0x00000066).into();
        t.colors.background = rgb(p.window).into();
        t.colors.foreground = rgb(p.text).into();
        t.colors.input = if panel {
            rgba((if dark { p.window } else { 0xffffff } << 8) | 0xbf).into()
        } else {
            rgb(p.field).into()
        };
        t.colors.border = if panel {
            rgba((if dark { p.text } else { 0x4b5263 } << 8) | 0x33).into()
        } else {
            rgb(p.line).into()
        };
        t.colors.muted_foreground = rgba(p.muted).into();
        t.colors.popover = rgb(p.surface).into();
        t.colors.popover_foreground = rgb(p.text).into();
        t.colors.primary = rgb(super::tokens::ACCENT).into();
        t.colors.primary_foreground = rgb(0x183c29).into();
        t.colors.ring = rgb(if panel { p.text } else { super::tokens::ACCENT }).into();
        t.colors.accent = rgb(p.active).into();
        t.colors.accent_foreground = rgb(p.text).into();
        t.colors.button = if panel {
            rgba((if dark { 0x161212 } else { 0xededee } << 8) | 0xb3).into()
        } else {
            rgb(p.field).into()
        };
        t.colors.button_foreground = rgb(p.text).into();
        t.colors.button_primary = rgb(super::tokens::ACCENT).into();
        t.colors.button_primary_foreground = rgb(0x183c29).into();
        t.colors.button_primary_hover = rgb(0x5ab981).into();
        t.colors.button_primary_active = rgb(0x4daa74).into();
        t.colors.slider_bar = rgb(super::tokens::ACCENT).into();
        t.colors.slider_thumb = rgb(super::tokens::ACCENT).into();
    });
    // Native backend offers material blur, not a calibrated CSS radius. Intensity is persisted only.
    window.set_background_appearance(if prefs.background_blur == 0 {
        WindowBackgroundAppearance::Opaque
    } else {
        WindowBackgroundAppearance::Blurred
    });
    dark
}
