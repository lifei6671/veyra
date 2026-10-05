//! 当前 P1 使用的 React 固定几何；源：openbox.css:86–155、820–836、1631–1730。
//! 局部规格保留独立名称，避免把 Panel 覆盖误用于其他页面。
pub const SIDEBAR: f32 = 256.;
pub const SIDEBAR_COLLAPSED: f32 = 64.;
pub const BRAND_WIDTH: f32 = 126.33;
// React 品牌 PNG 不随主题反色；文字区域最常见的不透明 RGB 为 (56,68,82)。
pub const BRAND_INK: u32 = 0x384452;
// Panel Dark 的品牌局部 filter: invert(1) hue-rotate(180deg)。
pub const BRAND_PANEL_DARK_INK: u32 = 0xb2becc;
pub const BODY: f32 = 14.;
pub const BODY_LINE: f32 = 20.;
pub const NAV_HEIGHT: f32 = 36.;
pub const CONTROL: f32 = 32.;
pub const ICON: f32 = 20.;
pub const ICON_SMALL: f32 = 16.;
pub const SETTINGS_NAV_HEIGHT: f32 = 48.;
pub const SETTINGS_ICON: f32 = 18.;
pub const GAP: f32 = 8.;
pub const PAD: f32 = 12.;
pub const RADIUS: f32 = 9.;
pub const NAV_RADIUS: f32 = 11.;
pub const ACCENT: u32 = 0x70c996;
pub const SETTING_HEIGHT: f32 = 40.;
pub const SECTION_PADDING: f32 = 16.;
pub const COLUMN_GAP: f32 = 48.;
pub const SECTION_TITLE: f32 = 16.;
pub const SECTION_LINE: f32 = 24.;
pub const SELECT_WIDTH: f32 = 192.;
pub const NUMBER_WIDTH: f32 = 80.;
pub const SLIDER_WIDTH: f32 = 256.;
// .panel-settings 局部控件与浮层规格。
pub const ROW_GAP: f32 = 4.;
pub const SWITCH_WIDTH: f32 = 40.;
pub const SWITCH_HEIGHT: f32 = 24.;
pub const SWITCH_THUMB: f32 = 18.;
pub const SWITCH_TRAVEL: f32 = 16.;
pub const SWITCH_PADDING: f32 = 2.;
pub const SWITCH_RADIUS: f32 = 12.;
pub const POPOVER_RADIUS: f32 = 13.;
pub const PROVIDER_WIDTH: f32 = 96.;
pub const BACKGROUND_WIDTH: f32 = 152.;
pub const UPLOAD_WIDTH: f32 = 40.;
pub const SITE_ICON_WIDTH: f32 = 64.;
pub const SITE_NAME_WIDTH: f32 = 112.;
pub const PICKER_WIDTH: f32 = 256.;
pub const PICKER_MAX_HEIGHT: f32 = 320.;
pub const MENU_PADDING: f32 = 4.;
pub const MENU_RADIUS: f32 = 10.;
pub const OPTION_RADIUS: f32 = 7.;
pub const OPTION_PADDING: f32 = 10.;
pub const DIALOG_WIDTH: f32 = 512.;
pub const DIALOG_HEIGHT: f32 = 142.;
pub const DIALOG_HEADER: f32 = 41.;
pub const ROW_BORDER: u32 = 0x4b526329;
pub const HOVER: u32 = 0x4b526312;
pub const SELECT_HIGHLIGHT: u32 = 0x65cb8f2e;
pub const SELECT_TEXT: u32 = 0x37664c;
// .panel-ip-tooltip / .test-sites-tooltip / .tooltip-trigger::after。
pub const TOOLTIP_BG: u32 = 0x30394c;
pub const TOOLTIP_TEXT: u32 = 0xfafafa;
pub const TOOLTIP_RADIUS: f32 = 7.;
pub const TOOLTIP_Y: f32 = 5.;
pub const TOOLTIP_OFFSET: f32 = 6.;
pub const IP_TOOLTIP_WIDTH: f32 = 280.;
pub const SITE_TOOLTIP_WIDTH: f32 = 350.;
pub const SITE_TOOLTIP_LINE: f32 = 19.6;

// 本地 Chromium reference 默认按钮 focus outline。
pub const BROWSER_FOCUS: u32 = 0x005fcc;
pub const BROWSER_FOCUS_DARK: u32 = 0x99c8ff;
pub fn browser_focus(dark: bool) -> u32 {
    if dark {
        BROWSER_FOCUS_DARK
    } else {
        BROWSER_FOCUS
    }
}
// Chromium Panel input 的蓝色焦点线位于原 80×32 field 边界内，不向外扩展。
pub const INPUT_FOCUS_OFFSET: f32 = -2.;

// color-mix(in srgb, #70c996 65%, white)，2px outline + 2px offset。
pub const SWITCH_FOCUS: u32 = 0xa2dcb9;

// React .sidebar-collapse-tooltip 的独立横向内边距。
pub const COLLAPSE_TOOLTIP_X: f32 = 9.;
// .sidebar-collapse-tooltip::before：8px 正方形旋转45°。
pub const TOOLTIP_ARROW_SIZE: f32 = 8.;
// 本地 Chromium number-field UA spinner（hover/focus reference测量）。
pub const SPINNER_WIDTH: f32 = 14.;
pub const SPINNER_HEIGHT: f32 = 18.;
pub const SPINNER_ARROW: f32 = 8.;
pub const SPINNER_BG: u32 = 0xf1f1f1;
pub const SPINNER_TEXT: u32 = 0x505050;
pub const SPINNER_DARK_BG: u32 = 0x424242;
pub const SPINNER_DARK_TEXT: u32 = 0xffffff;
// sidebar-toggle.svg 的固定描边；Dark CSS filter + opacity .92。
pub const COLLAPSE_INK: u32 = 0x30394c;
pub const COLLAPSE_DARK_ALPHA: f32 = 0.92;
