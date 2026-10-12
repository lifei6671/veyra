//! 当前 P1 使用的 React 固定几何；源：openbox.css:86–155、820–836、1631–1730。
//! 局部规格保留独立名称，避免把 Panel 覆盖误用于其他页面。
// 原 MiSans 4.003 子集实际 family；CSS alias 为 MiSans-VF。
pub const OPENBOX_FONT: &str = "MiSans VF";
pub const OPENBOX_EMOJI_FONT: &str = "Noto Color Emoji Flags";
// CSS语义字重：禁止把另一字体的静态face内部权重用在原版字体上。
pub const FONT_REGULAR: gpui_kit::FontWeight = gpui_kit::FontWeight(400.);
pub const FONT_MEDIUM: gpui_kit::FontWeight = gpui_kit::FontWeight(500.);
pub const FONT_SEMIBOLD: gpui_kit::FontWeight = gpui_kit::FontWeight(600.);
pub const FONT_BOLD: gpui_kit::FontWeight = gpui_kit::FontWeight(700.);
pub const FONT_TOAST: gpui_kit::FontWeight = gpui_kit::FontWeight(650.);
// 对应 React MiSans-VF/NotoEmoji 的显式顺序；原 96 个同族子集仍由字体 owner 排在前面。
// App Shell 与所有共享控件共用；只设置应用文本的显式 fallback，不修改系统字体。
pub fn openbox_font() -> gpui_kit::Font {
    gpui_kit::Font {
        family: OPENBOX_FONT.into(),
        fallbacks: Some(gpui_kit::FontFallbacks::from_fonts(vec![
            OPENBOX_EMOJI_FONT.into(),
        ])),
        ..Default::default()
    }
}
pub const SIDEBAR: f32 = 256.;
pub const SIDEBAR_COLLAPSED: f32 = 64.;
// .sidebar { transition: width .24s ease; }，CSS ease 的标准 Bezier 控制点。
pub const SIDEBAR_TRANSITION_MS: u64 = 240;
pub const SIDEBAR_EASE: [f32; 4] = [0.25, 0.1, 0.25, 1.];
pub const BRAND_WIDTH: f32 = 126.33;
// React 品牌 PNG 不随主题反色；文字区域最常见的不透明 RGB 为 (56,68,82)。
pub const BRAND_INK: u32 = 0x384452;
// Panel Dark 的品牌局部 filter: invert(1) hue-rotate(180deg)。
pub const BRAND_PANEL_DARK_INK: u32 = 0xb2becc;
pub const SMALL: f32 = 12.;
pub const BODY: f32 = 14.;
pub const BODY_LINE: f32 = 20.;
pub const NAV_HEIGHT: f32 = 36.;
pub const CONTROL: f32 = 32.;
pub const ICON: f32 = 20.;
pub const ICON_SMALL: f32 = 16.;
// 共享顶部沿用 React .settings-top-nav 的48px最小高度；LAN最终代理控件为32px。
pub const PAGE_TOOLBAR_HEIGHT: f32 = 48.;
// 通用组合的最小纵向留白；页面按实测控件高度居中，32px控件起点为8px。
pub const PAGE_TOOLBAR_PY: f32 = 4.;
pub const SETTINGS_ICON: f32 = 18.;
pub const GAP: f32 = 8.;
pub const PAD: f32 = 12.;
// AppShell 最终级联 .status-grid/.mini-stat/.uptime；同态 DOM 测量见 P2-08 reference-sidebar-computed.json。
pub const SIDEBAR_STAT_COLUMN_GAP: f32 = 16.;
pub const SIDEBAR_STAT_VALUE_LINE: f32 = 18.;
pub const SIDEBAR_STAT_VALUE_TOP: f32 = 1.;
pub const SIDEBAR_UPTIME_LABEL_LINE: f32 = 15.;
pub const SIDEBAR_UPTIME_GAP: f32 = 1.;
// React .uptime strong 最终 CSS：font-weight:500。
pub const SIDEBAR_UPTIME_WEIGHT: gpui_kit::FontWeight = FONT_MEDIUM;
pub const RADIUS: f32 = 9.;
pub const NAV_RADIUS: f32 = 11.;
pub const ACCENT: u32 = 0x70c996;
pub const SETTING_HEIGHT: f32 = 40.;
pub const SECTION_PADDING: f32 = 16.;
pub const COLUMN_GAP: f32 = 48.;
// .panel-settings-grid 的 viewport media queries；折叠侧栏时更早切换双列。
// .panel-settings-grid { width: 100%; max-width: 1280px; }
pub const SETTINGS_GRID_MAX_WIDTH: f32 = 1280.;
pub const PANEL_TWO_COLUMNS: f32 = 1024.;
pub const PANEL_COLLAPSED_TWO_COLUMNS: f32 = 768.;
pub const PANEL_WIDE: f32 = 1280.;
pub const PANEL_EXTRA_WIDE: f32 = 1536.;
pub const PANEL_COLUMN_GAP: f32 = 16.;
pub const PANEL_MEDIUM_COLUMN_GAP: f32 = 32.;
pub const PANEL_EXTRA_COLUMN_GAP: f32 = 64.;
pub const SECTION_TITLE: f32 = 16.;
pub const SECTION_LINE: f32 = 24.;
pub const SELECT_WIDTH: f32 = 192.;
pub const NUMBER_WIDTH: f32 = 80.;
pub const SLIDER_WIDTH: f32 = 256.;
// .panel-settings 局部控件与浮层规格。
pub const ROW_GAP: f32 = 4.;
// React PanelIconPicker 的展开字段：input-sm / gap-1.5 / h-3.5。
pub const ICON_PICKER_CONTENT_GAP: f32 = 6.;
pub const ICON_PICKER_CARET: f32 = 14.;
pub const ICON_PICKER_FIELD_OPACITY: f32 = 0.9;
pub const ICON_PICKER_RADIUS: f32 = 9.3;
pub const SWITCH_WIDTH: f32 = 40.;
pub const SWITCH_HEIGHT: f32 = 24.;
pub const SWITCH_THUMB: f32 = 18.;
pub const SWITCH_TRAVEL: f32 = 16.;
pub const SWITCH_PADDING: f32 = 2.;
pub const SWITCH_RADIUS: f32 = 12.;
// 在线 OpenBox .toggle-sm：故障转移高级设置复用相同的灰色轨道。
pub const SMALL_SWITCH_WIDTH: f32 = 33.;
pub const SMALL_SWITCH_HEIGHT: f32 = 20.;
pub const SMALL_SWITCH_THUMB: f32 = 13.;
pub const SMALL_SWITCH_PADDING: f32 = 2.5;
pub const SMALL_SWITCH_TRAVEL: f32 = 13.;
pub const SMALL_SWITCH_RADIUS: f32 = 9.3;
pub const SMALL_SWITCH_BACKGROUND_ALPHA: f32 = 0.9;
pub const SMALL_SWITCH_OFF_ALPHA: f32 = 0.5;
pub const POPOVER_RADIUS: f32 = 13.;
pub const PROVIDER_WIDTH: f32 = 96.;
pub const BACKGROUND_WIDTH: f32 = 152.;
pub const UPLOAD_WIDTH: f32 = 40.;
pub const SITE_ICON_WIDTH: f32 = 64.;
pub const SITE_NAME_WIDTH: f32 = 112.;
pub const PICKER_WIDTH: f32 = 256.;
pub const PICKER_MAX_HEIGHT: f32 = 320.;
// 搜索/分类区之外的既有图标列表视口，行高使用 CONTROL（32px）。
pub const PICKER_LIST_HEIGHT: f32 = 232.;
// 用户明确调整暗色图标分类选中项：accent 底上的文字为黑色。
pub const PICKER_SELECTED_DARK_TEXT: u32 = 0x000000;
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
// 用户统一两主题：OpenBox .tippy-box 14px/1.4，neutral 墨绿色与浅色文字。
pub const TOOLTIP_BG: u32 = 0x19362d;
pub const TOOLTIP_TEXT: u32 = 0xcdd3d1;
pub const TOOLTIP_RADIUS: f32 = 7.;
pub const TOOLTIP_Y: f32 = 5.;
pub const TOOLTIP_OFFSET: f32 = 6.;
pub const IP_TOOLTIP_WIDTH: f32 = 280.;
pub const SITE_TOOLTIP_WIDTH: f32 = 350.;
// 所有 Tooltip 共用，不允许侧栏/页面覆盖字号。
pub const TOOLTIP_FONT: f32 = 14.;
pub const TOOLTIP_LINE: f32 = 19.6;
// .proxy-tool-tooltip 的几何/出现时序；颜色与字体继续使用全局 Tooltip。
pub const TOOL_TOOLTIP_MAX_WIDTH: f32 = 160.;
pub const TOOL_TOOLTIP_PX: f32 = 9.;
pub const TOOL_TOOLTIP_RADIUS: f32 = 8.;
pub const TOOL_TOOLTIP_OFFSET: f32 = 7.;
pub const TOOL_TOOLTIP_ARROW: f32 = 5.;
pub const TOOL_TOOLTIP_ARROW_RIGHT: f32 = 11.;
pub const TOOL_TOOLTIP_PRIORITY: usize = 120;
pub const TOOL_TOOLTIP_DELAY_MS: u64 = 120;
pub const TOOL_TOOLTIP_TRANSITION_MS: u64 = 140;
pub const TOOL_TOOLTIP_RISE: f32 = 3.;
pub const TOOL_TOOLTIP_SHADOW_Y: f32 = 8.;
pub const TOOL_TOOLTIP_SHADOW_BLUR: f32 = 22.;
pub const TOOL_TOOLTIP_SHADOW: u32 = 0x121a2833;

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

// P2-01：openbox.css .subscription-empty/manage-card/editor-modal/source-form。
pub const SUBSCRIPTION_EMPTY_HEIGHT: f32 = 132.;
pub const SUBSCRIPTION_CARD_HEIGHT: f32 = 84.;
pub const SUBSCRIPTION_EDITOR_WIDTH: f32 = 672.;
pub const SUBSCRIPTION_TEXTAREA_HEIGHT: f32 = 162.;
pub const SUBSCRIPTION_TEXTAREA_FONT: f32 = 12.;
pub const SUBSCRIPTION_TEXTAREA_LINE: f32 = 18.;
pub const SUBSCRIPTION_TEXTAREA_PX: f32 = 12.;
pub const SUBSCRIPTION_TEXTAREA_PY: f32 = 8.;

pub const SUBSCRIPTION_UPDATED_GAP: f32 = 6.;

// .subscription-empty svg / :root --accent-strong；所有主题共用此基线。
pub const SUBSCRIPTION_EMPTY_ICON: f32 = 26.;
pub const ACCENT_STRONG: u32 = 0x5aba83;

// .subscription-source-field / preview-error / preview-skipped。
pub const SUBSCRIPTION_LABEL_FONT: f32 = 12.;
pub const SUBSCRIPTION_LABEL_LINE: f32 = 16.;
pub const SUBSCRIPTION_ERROR: u32 = 0xf06b6b;

// OpenBox .btn-primary：本机真实页面 OKLCH 转 sRGB；hover 为 primary 混入 7% 黑色。
pub const PRIMARY_BUTTON_TEXT: u32 = 0x223d30;
pub const PRIMARY_BUTTON_BG: u32 = 0x66cc8a;
pub const PRIMARY_BUTTON_HOVER: u32 = 0x5cb97d;
pub const PRIMARY_BUTTON_RADIUS: f32 = 9.3;
pub const REFERENCE_BUTTON_TRANSITION_MS: u64 = 200;
// GroupSettings/ProxiesPage FLIP共用固定时长，快速反向不缩短。
pub const REORDER_MOVE_MS: u64 = 180;
pub const SUBSCRIPTION_DNS_ROTATION_MS: u64 = 150;
pub const SUBSCRIPTION_HOVER_TRANSITION_MS: u64 = 150;
// .subscription-tab-group active 与 primary 使用同一颜色。
pub const SUBSCRIPTION_TAB_TEXT: u32 = PRIMARY_BUTTON_TEXT;
pub const SUBSCRIPTION_TAB_BG: u32 = PRIMARY_BUTTON_BG;
pub const SUBSCRIPTION_TAB_RADIUS: f32 = 6.;

pub const SUBSCRIPTION_FOOTER: f32 = 49.;
pub const SUBSCRIPTION_FOOTER_BUTTON: f32 = 52.;
pub const SUBSCRIPTION_TAB_HEIGHT: f32 = 40.;
pub const SUBSCRIPTION_TAB_MIN_WIDTH: f32 = 44.;
pub const SUBSCRIPTION_TEXT_BUTTON: f32 = 11.;
pub const SUBSCRIPTION_SKIPPED_GAP: f32 = 3.;
pub const SUBSCRIPTION_EMPTY_GAP: f32 = 5.;
pub const SUBSCRIPTION_NODE_EMPTY: f32 = 60.;
// 在线订阅读取区 .py-14 / .loading-md 实测。
pub const SUBSCRIPTION_LOADING_PADDING: f32 = 56.;
pub const SUBSCRIPTION_LOADING_FONT: f32 = 13.;
pub const SUBSCRIPTION_LOADING_SIZE: f32 = 24.;

// .subscription-source-options / .switch-input page-local CSS overrides.
pub const SUBSCRIPTION_OPTIONS_HEIGHT: f32 = 28.;
pub const SUBSCRIPTION_SWITCH_WIDTH: f32 = 33.;
pub const SUBSCRIPTION_SWITCH_HEIGHT: f32 = 20.;

// Host原版节点DNS输入区：右侧Bootstrap固定192px。
pub const SUBSCRIPTION_DNS_BOOTSTRAP_WIDTH: f32 = 192.;

// SubscriptionSettings完整视觉骨架：openbox.css 1100–1142、ProxyOption 426/440/444。
pub const SUBSCRIPTION_SHARE_HEIGHT: f32 = 136.;
pub const SUBSCRIPTION_SHARE_HEADER: f32 = 76.;
pub const SUBSCRIPTION_SHARE_CHEVRON: f32 = 15.;
pub const SUBSCRIPTION_NODE_HEIGHT: f32 = 62.;
pub const SUBSCRIPTION_NODE_MIN_WIDTH: f32 = 145.;
pub const SUBSCRIPTION_NODE_RADIUS: f32 = 13.6;
pub const SUBSCRIPTION_NODE_PY: f32 = 8.;
pub const SUBSCRIPTION_NODE_NAME_HEIGHT: f32 = 20.;
pub const SUBSCRIPTION_LATENCY_WIDTH: f32 = 40.;
pub const SUBSCRIPTION_LATENCY_HEIGHT: f32 = 20.;
pub const SUBSCRIPTION_DOTS_HEIGHT: f32 = 30.;
pub const SUBSCRIPTION_DOTS_PT: f32 = 14.;
pub const SUBSCRIPTION_UNTESTED: u32 = 0x9ca3af; // oklch(70.7% .022 261.325)→sRGB
pub const SUBSCRIPTION_RULE_SEQUENCE_WIDTH: f32 = 56.;
pub const SUBSCRIPTION_RULE_SEQUENCE_HEIGHT: f32 = 24.;
pub const SUBSCRIPTION_REGION_WIDTH: f32 = 128.;
pub const SUBSCRIPTION_REGION_DRAG: f32 = 18.;
pub const SUBSCRIPTION_REGION_REMOVE: f32 = 24.;
pub const SUBSCRIPTION_RULE_ROW_GAP: f32 = 6.;
pub const SUBSCRIPTION_RULE_HINT_LINE: f32 = 16.;
pub const SUBSCRIPTION_ERROR_WIDTH: f32 = 720.;
pub const SUBSCRIPTION_ERROR_GAP: f32 = 13.;

pub const SUBSCRIPTION_SWITCH_THUMB: f32 = 14.;
pub const SUBSCRIPTION_SWITCH_PADDING: f32 = 2.;

// .policy-option / .policy-option-latency 的浅色字面RGBA。
// 2026-10-07本机OpenBox实测：base-200 Light oklch(93% 0 0)。
pub const PANEL_BASE200_LIGHT: u32 = 0xe8e8e8;
pub const PANEL_BACKGROUND_TINT: f32 = 0.5;
// openbox.css .settings-page::before 使用最终 --surface：浅色白色 75%。
pub const PANEL_TOOLBAR_ALPHA: f32 = 0.75;
pub const SUBSCRIPTION_NODE_LIGHT_HOVER: u32 = 0xf1ead6;
pub const SUBSCRIPTION_LATENCY_RADIUS: f32 = 16.;
// Host本地OpenBox 2026-10-06实测暗色proxy-node-card：base-200由全局Theme提供，
// border-base-content/[.08]、协议text-base-content/60与sm:hover色单独保留。
pub const SUBSCRIPTION_NODE_DARK_BORDER_ALPHA: f32 = 0.08;
pub const SUBSCRIPTION_NODE_DARK_MUTED_ALPHA: f32 = 0.6;
pub const SUBSCRIPTION_NODE_DARK_HOVER_ALPHA: f32 = 0.16;
pub const SUBSCRIPTION_NODE_DARK_HOVER: u32 = 0x4b4428;
pub const SUBSCRIPTION_CHECKBOX_RADIUS: f32 = 9.3;
// 本机OpenBox toggle-xs：规则开关26×16，区别于自动更新33×20。
pub const SUBSCRIPTION_RULE_SWITCH_WIDTH: f32 = 26.;
pub const SUBSCRIPTION_RULE_SWITCH_HEIGHT: f32 = 16.;
pub const SUBSCRIPTION_RULE_SWITCH_THUMB: f32 = 10.;
pub const SUBSCRIPTION_CONTROL_RADIUS: f32 = 9.3;
pub const SUBSCRIPTION_EDITOR_RADIUS: f32 = 13.6;
pub const SUBSCRIPTION_RULE_TOKEN_HEIGHT: f32 = 20.;

pub const SUBSCRIPTION_RULE_SWITCH_RADIUS: f32 = 10.;

// .subscription-{share,node}-collapse: height .2s ease-out / opacity .16s(.12s collapsed) ease。
pub const SUBSCRIPTION_COLLAPSE_MS: u64 = 200;

// .subscription-preview-panel/.subscription-preview-state；业务仍为已保存节点只读展示。
pub const SUBSCRIPTION_NODES_PANEL_HEIGHT: f32 = 350.;
pub const SUBSCRIPTION_NODES_STATE_PADDING: f32 = 24.;
pub const SUBSCRIPTION_NODES_STATE_RADIUS: f32 = 15.8;
pub const SUBSCRIPTION_NODES_STATE_ICON: f32 = 24.;

// .subscription-preview-table：只读已保存节点复用其容器，未来改名/测速保持禁用。
pub const SUBSCRIPTION_NODES_TABLE_RADIUS: f32 = 16.;
pub const SUBSCRIPTION_NODES_TABLE_HEADER: f32 = 38.;
pub const SUBSCRIPTION_NODES_TABLE_ROW: f32 = 48.;
pub const SUBSCRIPTION_NODES_ORIGINAL_FRACTION: f32 = 5. / 12.;
pub const SUBSCRIPTION_NODES_LATENCY_WIDTH: f32 = 80.;

// 本机OpenBox .btn:hover：base-200在OKLab中混合7%黑色，区分全局主题。
pub const REFERENCE_GHOST_HOVER_LIGHT: u32 = 0xd2d2d2;
pub const REFERENCE_GHOST_HOVER_DARK: u32 = 0x130f0f;

pub const REFERENCE_BUTTON_FOCUS_OFFSET: f32 = 2.;

// 用户指定在线 OpenBox 2026-10-09 实测分享弹窗；旧 React/CSS 候选基线保留在 evidence。
// 768px、1:0.9 双栏、20px gap、32px按钮和176px二维码来自实际 computed style。
pub const SUBSCRIPTION_SHARE_FOOTER_GAP: f32 = 8.;
pub const SUBSCRIPTION_SHARE_BUTTON_GAP: f32 = 6.;
pub const SUBSCRIPTION_SHARE_CANCEL_RADIUS: f32 = 9.3;
pub const SUBSCRIPTION_SHARE_SAVE_HEIGHT: f32 = 32.;
pub const SUBSCRIPTION_SHARE_SAVE_PADDING: f32 = 12.;
pub const SUBSCRIPTION_SHARE_SAVE_RADIUS: f32 = 9.3;
pub const SUBSCRIPTION_SHARE_SAVE_WEIGHT: gpui_kit::FontWeight = FONT_SEMIBOLD;
pub const SUBSCRIPTION_SHARE_SURFACE: u32 = 0xffffffe6;
// 在线 modal 无边框，checkbox 无浏览器默认 margin，QR 使用4px内边距。
pub const SUBSCRIPTION_SHARE_BORDER: f32 = 0.;
pub const SUBSCRIPTION_SHARE_CHECK_MARGIN: f32 = 0.;
pub const SUBSCRIPTION_SHARE_QR_TOP: f32 = 0.;
pub const SUBSCRIPTION_SHARE_EDITOR_WIDTH: f32 = 768.;
pub const SUBSCRIPTION_SHARE_COLUMNS_GAP: f32 = 20.;
pub const SUBSCRIPTION_SHARE_LEFT_FRACTION: f32 = 1. / 1.9;
pub const SUBSCRIPTION_SHARE_BODY_HEIGHT: f32 = 368.;
pub const SUBSCRIPTION_SHARE_FOOTER: f32 = 53.;
pub const SUBSCRIPTION_SHARE_ROW_HEIGHT: f32 = 38.;
pub const SUBSCRIPTION_SHARE_LIST_MAX_HEIGHT: f32 = 320.;
pub const SUBSCRIPTION_SHARE_CHECKBOX: f32 = 20.;
pub const SUBSCRIPTION_SHARE_PROTOCOL_WIDTH: f32 = 83.125;
pub const SUBSCRIPTION_SHARE_COPY_WIDTH: f32 = 40.;
pub const SUBSCRIPTION_SHARE_QR_SIZE: f32 = 176.;
// React 分享卡片、表单与 QRCodeSVG 容器；监听说明沿用相同排版。
pub const SUBSCRIPTION_SHARE_CARD_HEIGHT: f32 = 60.;
pub const SUBSCRIPTION_SHARE_TEXT_GAP: f32 = 2.;
pub const SUBSCRIPTION_SHARE_META_FONT: f32 = 12.;
pub const SUBSCRIPTION_SHARE_FIELD_FONT: f32 = 12.;
pub const SUBSCRIPTION_SHARE_BODY_FONT: f32 = 14.;
pub const SUBSCRIPTION_SHARE_CHECK_RADIUS: f32 = 9.3;
pub const SUBSCRIPTION_SHARE_LINK_HEIGHT: f32 = 32.;
pub const SUBSCRIPTION_SHARE_QR_PADDING: f32 = 4.;
pub const SUBSCRIPTION_SHARE_QR_RADIUS: f32 = 16.;
pub const SUBSCRIPTION_SHARE_QR_QUIET_MODULES: f32 = 1.;
pub const SUBSCRIPTION_SHARE_CHECK_MARK: f32 = 12.;
// 2026-10-09 指定在线参考站点的 Dialog computed style；不同于仓库旧 window.confirm。
// oklch 色值转为 sRGB：text(.35519 .032 262.988)、error(.7176 .221 22.18)、cancel(.93 0 0)。
pub const SUBSCRIPTION_SHARE_CONFIRM_RADIUS: f32 = 13.6;
pub const SUBSCRIPTION_SHARE_CONFIRM_BUTTON_RADIUS: f32 = 9.3;
pub const SUBSCRIPTION_SHARE_CONFIRM_TEXT: u32 = 0x333c4d;
pub const SUBSCRIPTION_SHARE_CONFIRM_ERROR: u32 = 0xff5861;
pub const SUBSCRIPTION_SHARE_CONFIRM_CANCEL: u32 = 0xe8e8e8;
pub const SUBSCRIPTION_SHARE_CONFIRM_OVERLAY: u32 = 0x00000066;

/// OpenBox .backend-card/.backend-badge/.backend-service-actions (openbox.css:2313–2352).
pub mod backend {
    // OpenBox只读控件保留原生对比度，不叠加Kit disabled opacity。
    pub const FIELD_LINE_ALPHA: u32 = 0x21;
    pub const FIELD_ALPHA: u32 = 0xbf;
    pub const SWITCH_BORDER_ALPHA: u32 = 0x80;
    // 本机 OpenBox 1280px viewport 的实际分组几何，未知字段保留布局槽位。
    pub const CHANNEL_WIDTH: f32 = 126.;
    pub const FIELD_WIDTH: f32 = 200.;
    pub const SMALL_FIELD_WIDTH: f32 = 96.;
    pub const TIMEZONE_WIDTH: f32 = 384.;
    pub const FIELD_GAP: f32 = 4.;
    pub const SEGMENT_RADIUS: f32 = 6.4;
    pub const TUN_LABEL_WIDTH: f32 = 112.;
    pub const TUN_SELECT_WIDTH: f32 = 144.;
    pub const IPV6_FIELD_WIDTH: f32 = 176.;
    pub const SWITCH_TRAVEL: f32 = 12.;
    // P2-03-VISUAL-BUSY-001：过渡态使用琥珀色，不复用失败红色。
    pub const TRANSITION_LIGHT: u32 = 0x9a6700;
    pub const TRANSITION_DARK: u32 = 0xe7bb62;
    pub const TRANSITION_ALPHA: u32 = 0x2e;
    pub const LINE_LIGHT: u32 = 0xd1d1d199;
    pub const LINE_DARK: u32 = 0xffffff0c;
    pub const BADGE_LINE_LIGHT: u32 = 0x4b526311;
    pub const BADGE_LINE_DARK: u32 = 0xffffff0a;
    pub const TITLE_LINE: f32 = 24.;
    pub const PADDING: f32 = 16.;
    pub const GAP: f32 = 12.;
    pub const RADIUS: f32 = 16.;
    pub const BODY: f32 = 14.;
    pub const LINE: f32 = 20.;
    pub const TITLE: f32 = 16.;
    pub const HINT: f32 = 12.;
    pub const HINT_LINE: f32 = 16.;
    pub const ICON: f32 = 16.;
    pub const ACTION_GAP: f32 = 8.;
    pub const BUTTON_HEIGHT: f32 = 32.;
    pub const BUTTON_PADDING: f32 = 12.;
    pub const BUTTON_GAP: f32 = 6.;
    pub const BUTTON_RADIUS: f32 = 8.7;
    pub const BADGE_HEIGHT: f32 = 20.;
    pub const BADGE_PADDING: f32 = 9.;
    pub const BUTTON_LIGHT: u32 = 0xe8e8e8;
    // 本机 OpenBox .btn:hover computed color (oklab L=.8649)。
    pub const BUTTON_HOVER_LIGHT: u32 = 0xd2d2d2;
    pub const BUTTON_DARK: u32 = 0x191e24;
    pub const ON: u32 = 0x00a96e;
    pub const ON_BG: u32 = 0x00a96e2e;
    pub const OFF: u32 = 0xff5861;
    pub const OFF_BG: u32 = 0xff586126;
}

// OpenBox openbox.css:1311–1465 节点组；原版固定逻辑像素，不取 Kit 默认值。
pub mod groups {
    // OpenBox .group-failover-*：只覆盖主备编辑器，不改变普通组。
    pub const FAILOVER_HINT_TEXT: f32 = 11.;
    pub const FAILOVER_HINT_LINE: f32 = 20.;
    // 在线原版 2026-10-11 select-xs 的最终 computed：86×24，11px。
    pub const LANE_MODE_WIDTH: f32 = 86.;
    pub const LANE_MODE_HEIGHT: f32 = 24.;
    pub const LANE_MODE_TEXT: f32 = 11.;
    // 同一两行成员 header，共用外层内边距；select 文本外占用含边框共42px。
    pub const MEMBER_HEAD_PAD_Y: f32 = 6.;
    pub const MEMBER_TITLE_LINE: f32 = 16.;
    pub const MEMBER_TOOL_TEXT: f32 = 11.;
    pub const MEMBER_TOOL_LINE: f32 = 16.5;
    pub const FAILOVER_SEARCH_PAD: f32 = 12.;
    pub const FAILOVER_SELECT_CHROME: f32 = 42.;
    pub const LANE_SETTINGS_HEIGHT: f32 = 36.;
    pub const LANE_TAB_HEIGHT: f32 = 31.;
    pub const LANE_TAB_PAD: f32 = 13.;
    pub const LANE_TAB_BAR: f32 = 39.;
    pub const LANE_SETTINGS_PAD: f32 = 7.;
    pub const LANE_ICON_WIDTH: f32 = 145.;
    pub const LANE_DELETE_WIDTH: f32 = 34.;
    pub const FAILOVER_MIN_HEIGHT: f32 = 322.;
    // LAN 原版最终 scrollbar-color 为 foreground/.35、透明轨道；轨道 CSS12px，
    // 同态截图的实绘 thumb 约7px、距右边2.5px、圆头。溢出时按用户要求显示。
    pub const EDITOR_SCROLL_TRACK: f32 = 12.;
    pub const EDITOR_SCROLL_THUMB: f32 = 7.;
    pub const EDITOR_SCROLL_INSET: f32 = 2.5;
    pub const EDITOR_SCROLL_RADIUS: f32 = 3.5;
    pub const EDITOR_SCROLL_ALPHA: f32 = 0.35;
    // 用户当前原版高级区域：flex-wrap、gap12、padding8/12、r16，无边框。
    pub const ADVANCED_RADIUS: f32 = 16.;
    pub const ADVANCED_SURFACE_ALPHA: f32 = 0.6;
    pub const ADVANCED_SWITCH_BOTTOM: f32 = 6.;
    pub const ADVANCED_TOGGLE_RADIUS: f32 = 9.3;
    pub const ADVANCED_TOGGLE_ICON: f32 = 14.;
    pub const COMPACT_RADIUS: f32 = 7.;
    pub const FOOTER_GAP: f32 = 10.;
    // OpenBox .compact-button / .primary-button gap。
    pub const FOOTER_CONTENT_GAP: f32 = 7.;
    pub const PRIMARY_TEXT: f32 = 11.;
    pub const HINT_LINE: f32 = 16.;
    pub const FOOTER_BUTTON_PAD: f32 = 14.;
    pub const LOADING_HEIGHT: f32 = 180.;
    pub const LOADING_ICON: f32 = 28.;
    // OpenBox .group-member-* / .group-scale-control 的局部尺寸。
    pub const FILTER_TEXT: f32 = 12.;
    pub const FILTER_EXTRA: f32 = 32.;
    pub const SEARCH_PAD: f32 = 10.;
    pub const MEMBER_LINE: f32 = 20.;
    pub const MEMBER_BADGE_HEIGHT: f32 = 16.;
    pub const MEMBER_BADGE_PAD: f32 = 6.;
    pub const MEMBER_BADGE_TEXT: f32 = 10.;
    pub const MEMBER_CHECK_RADIUS: f32 = 6.;
    pub const CHECK_COLOR: u32 = 0x17382b;
    pub const FALLBACK_ICON: f32 = 15.;
    pub const RULE_CHECK_RADIUS: f32 = 2.;
    pub const BADGE_PAD_X: f32 = 7.;
    pub const BADGE_PAD_Y: f32 = 1.;
    pub const BADGE_LINE: f32 = 17.;
    pub const BADGE_RADIUS: f32 = 7.;
    pub const DESCRIPTION_TOP: f32 = 2.;
    pub const COUNTRY_ICON_WIDTH: f32 = 16.;
    pub const COUNTRY_ICON_HEIGHT: f32 = 12.;
    pub const COUNTRY_PICKER: f32 = 144.;
    pub const COUNTRY_MENU: f32 = 256.;
    pub const SCALE_RESET: f32 = 52.;
    // .group-mode-tabs：按文本 max-content，外层 4px / 9px，按钮圆角 7px。
    pub const MODE_TABS_PADDING: f32 = 4.;
    pub const MODE_TABS_RADIUS: f32 = 9.;
    // 用户 2026-10-11：模式 Tab 悬浮仅轻微改变文字颜色，无背景变化。
    pub const MODE_TAB_HOVER_OPACITY: f32 = 0.85;
    // 已有不透明编辑 surface 的浅色底：本轮用户原版截图空白区实测 #f5f4f4。
    // 白色 Tab/输入与该底色分层；不改其它弹窗，也不宣称等价 backdrop blur。
    pub const EDIT_SURFACE_LIGHT: u32 = 0xf5f4f4;
    pub const MODAL_WIDTH: f32 = 896.;
    // P4-02 已批准不透明 surface 替代 GPUI 无法等价实现的 backdrop blur。
    // 只降低 alpha 会让列表文字清晰穿透，与原版模糊背景不同。
    pub const MODAL_ALPHA: f32 = 1.;
    pub const AUTO_MODAL_ALPHA: f32 = 0.94;
    pub const HEADER: f32 = 41.;
    pub const FOOTER: f32 = 49.;
    pub const TITLE_LINE: f32 = 20.;
    pub const RUNTIME_WIDTH: f32 = 380.;
    pub const CARD_HEIGHT: f32 = 60.;
    // 在线OpenBox btn-sm/btn-square的neutral表面；只用于出站页header。
    pub const HEADER_BG_DARK: u32 = 0x191e24;
    pub const HEADER_HOVER_LIGHT: u32 = 0xd2d2d2;
    pub const HEADER_HOVER_DARK: u32 = 0x15191e;
    pub const HEADER_RADIUS: f32 = 9.3;
    // Legacy group-card.is-drag-placeholder，不改正式enabled。
    pub const DRAG_PLACEHOLDER_ALPHA: f32 = 0.3;
    pub const DRAG_BORDER_ALPHA: f32 = 0.58;
    pub const DRAG_INSET_ALPHA: f32 = 0.25;
    // .group-card background: var(--surface)，浅色为白色 75%。
    pub const CARD_ALPHA: f32 = 0.75;
    pub const CARD_PAD_Y: f32 = 11.;
    pub const CARD_ICON: f32 = 18.;
    pub const CARD_ACTION: f32 = 30.;
    pub const DRAG_ICON: f32 = 15.;
    pub const ICON_FIELD: f32 = 224.;
    pub const SCALE_VALUE: f32 = 48.;
    pub const SCALE_WIDTH: f32 = 158.;
    pub const RULE_FIELD: f32 = 174.;
    pub const MEMBER_HEIGHT: f32 = 256.;
    pub const MEMBER_ROW: f32 = 36.;
    pub const FILTER_HEIGHT: f32 = 24.;
    pub const FILTER_WIDTH: f32 = 128.;
    pub const AUTO_WIDTH: f32 = 576.;
    pub const DYNAMIC_HEIGHT: f32 = 224.;
    pub const DYNAMIC_HEADER_PAD_Y: f32 = 6.;
    pub const DYNAMIC_RESULTS_HEIGHT: f32 =
        DYNAMIC_HEIGHT + HINT_LINE + 2. * DYNAMIC_HEADER_PAD_Y + 3.;
}

/// 共享网络：在线 OpenBox 2026-10-09 实测；字段32px、双栏间16px、编辑576px。
pub mod shared_network {
    pub const EDIT_WIDTH: f32 = 576.;
    pub const CODE_WIDTH: f32 = 448.;
    pub const EDIT_QR: f32 = 176.;
    pub const CODE_QR: f32 = 224.;
    // 用户本轮要求：拖动卡片预览、原位占位和按行吸附；静止布局不变。
    pub const CARD_HEIGHT: f32 = 70.;
    pub const CARD_PAD: f32 = 12.;
    pub const CARD_RADIUS: f32 = 16.;
    pub const MODAL_RADIUS: f32 = 13.6;
    pub const HEADER: f32 = 41.;
    pub const LABEL: f32 = 12.;
    pub const LABEL_LINE: f32 = 16.;
    pub const JOIN_BUTTON: f32 = 40.;
    pub const QR_PAD: f32 = 4.;
    pub const QR_RADIUS: f32 = 9.;
    pub const EMPTY_PAD: f32 = 56.;
    pub const EMPTY_ICON: f32 = 40.;
}

/// LogsPage.tsx / openbox.css:657–683：日志页固定工具栏与可见行几何。
pub mod logs {
    pub const SEARCH_ICON: f32 = 15.;
    pub const SURFACE_ALPHA: f32 = 0.75;
    pub const EMPTY_HEIGHT: f32 = 160.;
    pub const BADGE_X: f32 = 7.;
    pub const BADGE_Y: f32 = 3.;
    // 级别来自 React 12px；时间按用户2026-10-09明确要求同步至同字号。
    pub const TAG_FONT: f32 = 12.;
    pub const DEBUG_BACKGROUND: u32 = 0xf7f7f8c2; // React .level-debug rgba(247,247,248,.76)
    pub const TIME_BACKGROUND: u32 = 0x70c99621;
    pub const LEVEL_WIDTH: f32 = 96.;
    pub const FILTER_WIDTH: f32 = 256.;
    pub const SEARCH_WIDTH: f32 = 256.;
    pub const ROW_HEIGHT: f32 = 56.;
    pub const ROW_PITCH: f32 = 64.;
    pub const ROW_RADIUS: f32 = 15.8;
    pub const NUMBER_WIDTH: f32 = 42.;
    pub const TIME_WIDTH: f32 = 68.;
    pub const LEVEL_TAG_WIDTH: f32 = 58.;
    pub const BADGE_RADIUS: f32 = 8.;
    pub const TOOL_BACKGROUND: u32 = 0xe7e7e7;
    pub const INFO: u32 = 0x4384c0;
    pub const WARNING: u32 = 0xa26d1b;
    pub const ERROR: u32 = 0xc44d4d;
    pub const DEBUG: u32 = 0x6e7b80;
}

/// OpenBox shared EmptyState / ErrorState，CSS 915–920、1506–1510。
pub mod status {
    pub const EMPTY_ACCENT_DARK: u32 = 0x57c98b;
    pub const EMPTY_ALPHA: f32 = 0.13;
    pub const EMPTY_ALPHA_DARK: f32 = 0.14;
    pub const EMPTY_HEIGHT: f32 = 180.;
    pub const EMPTY_ICON_BOX: f32 = 52.;
    pub const EMPTY_ICON_RADIUS: f32 = 17.;
    pub const EMPTY_ICON_TEXT: f32 = 20.;
    pub const EMPTY_TITLE_MARGIN: f32 = 14.;
    pub const EMPTY_TEXT_MARGIN: f32 = 5.;
    pub const EMPTY_TEXT_WIDTH: f32 = 380.;
    pub const EMPTY_TEXT_SIZE: f32 = 10.;
    pub const EMPTY_TEXT_LINE: f32 = 17.;
    pub const ERROR_WIDTH: f32 = 720.;
    pub const ERROR_GAP: f32 = 13.;
    pub const RETRY_HEIGHT: f32 = 36.;
    pub const RETRY_PADDING: f32 = 15.;
    pub const RETRY_RADIUS: f32 = 11.;
}

/// ProxiesPage/ProxyCards：openbox.css 346、408–449、970（最终 active 覆盖）。
pub mod proxies {
    // 原件 CSS 字重；font-synthesis:none，静态子集不伪造粗体。
    pub const FONT_REGULAR: gpui_kit::FontWeight = super::FONT_REGULAR;
    pub const FONT_MEDIUM: gpui_kit::FontWeight = super::FONT_MEDIUM;
    pub const FONT_SEMIBOLD: gpui_kit::FontWeight = super::FONT_SEMIBOLD;
    // ProxyCards .policy-footer / .policy-penetration / .penetration-summary。
    pub const PENETRATION_FOOTER_TOP: f32 = 15.;
    pub const PENETRATION_FOOTER_HEIGHT: f32 = 30.;
    pub const PENETRATION_FOOTER_PX: f32 = 11.;
    pub const PENETRATION_FOOTER_RADIUS: f32 = 8.;
    pub const PENETRATION_FOOTER_BG: u32 = 0x303a4cff;
    pub const PENETRATION_TOP: f32 = 8.;
    // LAN 展开穿透：层 pt10 + 内部pt4、头部24+6+20、节点顶部6。
    pub const PENETRATION_PADDING_TOP: f32 = 10.;
    pub const PENETRATION_INNER_TOP: f32 = 4.;
    pub const PENETRATION_HEAD: f32 = 50.;
    pub const PENETRATION_GRID_TOP: f32 = 6.;
    pub const PENETRATION_TITLE_GAP: f32 = 6.;
    pub const LATENCY_ICON: f32 = 14.;
    pub const HISTORY_DELAY_MS: u64 = 1000;
    pub const HISTORY_MIN_WIDTH: f32 = 214.;
    pub const HISTORY_MAX_WIDTH: f32 = 350.;
    pub const HISTORY_PX: f32 = 9.;
    pub const HISTORY_PY: f32 = 5.;
    pub const HISTORY_RADIUS: f32 = 9.;
    pub const HISTORY_SHADOW_Y: f32 = 14.;
    pub const HISTORY_SHADOW_BLUR: f32 = 32.;
    pub const HISTORY_SHADOW: u32 = 0x121a283d;
    pub const HISTORY_ITEM_HEIGHT: f32 = 33.;
    pub const HISTORY_GAP: f32 = 7.;
    pub const HISTORY_META_WIDTH: f32 = 118.;
    pub const HISTORY_NAME_MAX: f32 = 150.;
    pub const HISTORY_TRACK_WIDTH: f32 = 16.;
    pub const HISTORY_RESULT_WIDTH: f32 = 45.;
    pub const HISTORY_DOT: f32 = 10.;
    pub const HISTORY_TRACK: u32 = 0xcfd8e680;
    pub const HISTORY_TRACK_ARROW: u32 = 0xcfd8e6b8;
    pub const HISTORY_SUCCESS_DOT: u32 = 0x61c991;
    pub const HISTORY_UNKNOWN_DOT: u32 = 0x9aa5b5;
    pub const HISTORY_OFFSET: f32 = 7.;
    // .policy-option-latency.success / .policy-latency.success 的 accent-strong。
    pub const LATENCY_SUCCESS: u32 = 0x5aba83;
    // --surface 的实际级联；不沿用其它页面批准的不透明替代。
    pub const SURFACE_LIGHT: u32 = 0xffffffbf;
    pub const SURFACE_DARK: u32 = 0x2a3030f0;
    // 用户2026-10-11指出偏高；LAN最终级联为32px，Tab按钮为24px。
    pub const TOOLBAR: f32 = 32.;
    // openbox.css .proxy-search / .proxy-tabs / .node-provider-head
    pub const SEARCH_WIDTH: f32 = 320.;
    // Chromium 最终 ::placeholder UA 色；openbox.css:357 focus-within。
    pub const SEARCH_PLACEHOLDER: u32 = 0x757575;
    pub const SEARCH_FOCUS_ACCENT_RATIO: f32 = 0.72;
    pub const SEARCH_FOCUS_SHADOW: u32 = 0x70c99621;
    pub const SEARCH_FOCUS_SPREAD: f32 = 2.;
    // Chromium input 的最终 UA padding: 1px 2px；Kit 单行编辑器无内边距。
    // LAN最终 solid accent oklch(.76662 .13515 153.45) 转 sRGB #66cc8a。
    pub const TAB_SELECTED: u32 = 0x66cc8aff;
    pub const TAB_SELECTED_TEXT: u32 = 0x37664c;
    pub const PROVIDER_HEAD: f32 = 40.;
    pub const TAB: f32 = 24.;
    pub const TAB_PX: f32 = 8.;
    pub const TAB_RADIUS: f32 = 6.4;
    pub const CONTROL_RADIUS: f32 = 9.3;
    pub const CONTROL_LINE: f32 = 20.;
    // LAN策略设置弹窗；视口上限90%，内部独立滚动，标题41px。
    pub const SETTINGS_WIDTH: f32 = 672.;
    pub const SETTINGS_RADIUS: f32 = 13.6;
    pub const SETTINGS_ROW: f32 = 32.;
    pub const SETTINGS_BODY_PAD: f32 = 24.;
    pub const SETTINGS_GAP: f32 = 16.;
    pub const SETTINGS_COLUMNS_GAP: f32 = 12.;
    pub const SETTINGS_LIST_GAP: f32 = 4.;
    pub const SETTINGS_ITEM_BG: u32 = 0xededed99;
    // ProxiesPage.tsx FLIP及openbox.css proxy-order-item.dragging；用户要求跟手/目标吸附。
    pub const ORDER_MOVE_MS: u64 = super::REORDER_MOVE_MS;
    pub const ORDER_ROW_PY: f32 = 6.;
    pub const ORDER_PLACEHOLDER_OPACITY: f32 = 0.55;
    pub const ORDER_DRAG_SHADOW_Y: f32 = 7.;
    pub const ORDER_DRAG_SHADOW_BLUR: f32 = 18.;
    pub const ORDER_DRAG_SHADOW_ALPHA: f32 = 0.14;
    // 原版命中规则固定前置行；None 只显示未生效，不创建自定义规则。
    pub const SETTINGS_PIN_SIZE: f32 = 17.;
    pub const SETTINGS_NUMBER_WIDTH: f32 = 16.;
    pub const SETTINGS_INPUT_WIDTH: f32 = 80.;
    pub const SETTINGS_RESET_WIDTH: f32 = 52.;
    // openbox.css:382–384：统一32px外框；内侧input仅右分隔线，按钮无独立边框。
    pub const WIDTH_CONTROL_RADIUS: f32 = 9.;
    pub const WIDTH_INPUT_PX: f32 = 11.;
    pub const WIDTH_RESET_PX: f32 = 12.;
    pub const WIDTH_LINE_LIGHT: u32 = 0x4b526321;
    pub const WIDTH_LINE_DARK: u32 = 0xffffff14;
    pub const WIDTH_SURFACE_LIGHT: u32 = 0xffffff;
    pub const WIDTH_SURFACE_DARK: u32 = 0x2a3030;
    // LAN SELECT.select-sm：双 4px CSS gradient 的同几何 SVG，右偏移 12.1px。
    pub const SETTINGS_SORT_WIDTH: f32 = 112.;
    pub const SETTINGS_SORT_LEFT: f32 = 12.;
    pub const SETTINGS_SORT_RIGHT: f32 = 28.;
    pub const SETTINGS_SORT_ARROW_WIDTH: f32 = 7.9;
    pub const SETTINGS_SORT_ARROW_HEIGHT: f32 = 4.;
    pub const SETTINGS_SORT_ARROW_RIGHT: f32 = 16.1;
    pub const SETTINGS_SORT_ARROW: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 7.9 4"><path d="M0 0H4V4Z M3.9 0H7.9L3.9 4Z" fill="currentColor"/></svg>"#;
    pub const ICON_SCALE_BASE: f32 = 46.;
    // openbox.css .proxy-width-control button → --field。
    pub const SETTINGS_RESET_BG: u32 = 0xf7f7f8c2;
    pub const SETTINGS_RESET_DARK_BG: u32 = 0x333a3aff;
    pub const SETTINGS_NOTE_LINE: f32 = 16.;
    pub const SETTINGS_PIN_ROTATION: f32 = -22.;
    pub const SETTINGS_PIN_COLOR: u32 = 0xe78eaa;
    pub const SETTINGS_BADGE_FONT: f32 = 10.;
    pub const SETTINGS_BADGE_RADIUS: f32 = 5.;
    pub const SETTINGS_BADGE_PX: f32 = 5.;
    pub const SETTINGS_BADGE_PY: f32 = 1.;
    // openbox.css .subscription-provider-card / title / count / updated。
    pub const SUBSCRIPTION_MIN_HEIGHT: f32 = 160.;
    // React inline h2 的字体度量形成 21.5px 首行，加5px和20px更新时间行。
    pub const SUBSCRIPTION_HEADER: f32 = 46.5;
    pub const GROUP_TITLE: f32 = 16.;
    // LAN 首行标题16/24、类型12/16；内层穿透保持 Legacy20px行高。
    pub const GROUP_TITLE_LINE: f32 = 24.;
    pub const GROUP_METADATA_LINE: f32 = 16.;
    pub const SUBSCRIPTION_COUNT_GAP: f32 = 6.;
    // .proxy-tool-with-tooltip 的后置 margin-left:6px；provider-actions gap:4px。
    pub const TOOL_TOOLTIP_MARGIN: f32 = 6.;
    pub const SUBSCRIPTION_UPDATED_GAP: f32 = 5.;
    // LAN collapse 展开按内容自然高度，固定205px会在单行组选项下留下额外空白。
    pub const CARD_MIN_HEIGHT: f32 = 0.;
    pub const COLLAPSED_MIN_HEIGHT: f32 = 116.;
    pub const HEAD: f32 = 54.;
    // 用户2026-10-11纠正：运行原版13个策略标题图标实测均24×24。
    // 本地Legacy CSS仍为46px，保留原参考并在审计中单列这项基线冲突。
    pub const GROUP_ICON: f32 = 24.;
    // options 保留24px；LAN current16px，Legacy current18px差异单列。
    pub const OPTION_ICON: f32 = 24.;
    pub const CURRENT_ICON: f32 = 16.;
    pub const CURRENT_GAP: f32 = 4.;
    pub const CURRENT_TOP: f32 = 6.;
    // ProxyCards PenetrationGroup 内层保持原 p 字号与间距，不能吃外层 LAN 覆盖。
    pub const PENETRATION_CURRENT_TOP: f32 = 6.;
    pub const PENETRATION_CURRENT_GAP: f32 = 4.;
    pub const PENETRATION_CURRENT_FONT: f32 = 14.;
    // LAN 出口链最终 text-sm = 14px/20px，Legacy p 的13px不同。
    pub const CURRENT_FONT: f32 = 14.;
    // 用户2026-10-11按运行原版纠正：徽章bg-base-200/.9，14/500/20；类型与计数12/400/16。
    // LAN oklab(.93 0 0)转sRGB≈231.67；GPUI颜色通道按8bit量化。
    pub const POLICY_BADGE_PX: f32 = 8.;
    pub const POLICY_BADGE_PY: f32 = 2.;
    pub const POLICY_BADGE_RADIUS: f32 = 9.3;
    pub const POLICY_BADGE_FONT: f32 = 14.;
    pub const POLICY_METADATA_FONT: f32 = 12.;
    pub const POLICY_TITLE_GAP: f32 = 4.;
    pub const POLICY_BADGE_BG: u32 = 0xffffff80;
    pub const POLICY_BADGE_BG_LIGHT: u32 = 0xe8e8e8e6;
    pub const POLICY_BADGE_TEXT: u32 = 0x333c4dcc;
    pub const POLICY_METADATA_TEXT: u32 = 0x333c4d99;
    // LAN最终 node-group-header-toggle gap；订阅标题仍沿用 Legacy14px。
    pub const POLICY_HEAD_GAP: f32 = 6.;
    pub const HEAD_GAP: f32 = 14.;
    pub const NODE_BORDER: f32 = 1.;
    // .node-provider-options small 的后置覆盖，标题不预留延迟区。
    pub const NODE_SUBTITLE_RIGHT: f32 = 44.;
    pub const NODE_HEIGHT: f32 = 61.;
    pub const NODE_RADIUS: f32 = 11.;
    pub const NODE_PY: f32 = 7.;
    pub const NODE_RIGHT: f32 = 52.;
    pub const NODE_TITLE: f32 = 26.;
    pub const LATENCY_HEIGHT: f32 = 20.;
    pub const GROUP_LATENCY_LINE: f32 = 16.;
    pub const LATENCY_MIN_WIDTH: f32 = 36.;
    pub const LATENCY_PX: f32 = 7.;
    pub const DOT: f32 = 16.;
    // .node-health-dots min-height:20px，圆点垂直居中。
    pub const DOT_LINE: f32 = 20.;
    // CSS332 / 452–460：测速动画只在真实在途期间存在。
    pub const TESTING_PERIOD_MS: u64 = 720;
    pub const TESTING_DOTS_HEIGHT: f32 = 8.;
    pub const TESTING_SCALE: f32 = 0.96;
    pub const TESTING_BACKGROUND: u32 = 0xffffffe6;
    pub const TESTING_DOT: f32 = 3.;
    pub const TESTING_DOT_GAP: f32 = 2.;
    // 原版 .policy-option.split-action.disabled 的整卡不透明度。
    pub const NODE_DISABLED_OPACITY: f32 = 0.55;
    // .policy-latency:disabled / .penetration-test:disabled；含 active Bolt 整层。
    pub const CONTROL_DISABLED_OPACITY: f32 = 0.5;
    // 当前原版实际 .proxy-node-card：bg-base-200/.9 与 bg-primary/.9，不能沿用Legacy后置active/.32。
    // 原版primary oklab(.76662,-.120764,.0603421)→sRGB(102.35,203.93,138.14)。
    pub const NODE_BG: u32 = 0xededeead;
    pub const NODE_SELECTED: u32 = 0x66ce9252;
    pub const NODE_BG_LIGHT: u32 = 0xe8e8e8e6;
    pub const NODE_SELECTED_LIGHT: u32 = 0x66cc8ae6;
    pub const NODE_BORDER_COLOR: u32 = 0x333c4d14;
    pub const NODE_SELECTED_BORDER: u32 = 0x52be7f80;
    pub const NODE_SELECTED_BORDER_LIGHT: u32 = 0x333c4d29;
    pub const NODE_TEXT: u32 = 0x333c4dff;
    pub const NODE_MUTED_TEXT: u32 = 0x333c4d99;
    pub const NODE_SELECTED_TEXT: u32 = 0x223d30ff;
    pub const NODE_HOVER: u32 = 0x57c98b14;
    pub const NODE_HOVER_LIGHT: u32 = 0xf1ead6ff;
    pub const NODE_HOVER_BORDER: u32 = 0x333c4d29;
    pub const NODE_SELECTED_HOVER_BORDER: u32 = 0x333c4d3d;
    pub const SUCCESS: u32 = 0x35bf83;
    pub const WARNING: u32 = 0xa26d1b;
    pub const DANGER: u32 = 0xc44d4d;
    pub const UNKNOWN: u32 = 0x9098a5;
}

/// LAN Modal 与目标分流编辑器的最终几何；不覆盖其它页面局部规格。
pub mod modal {
    pub const HEADER_HEIGHT: f32 = 41.;
    pub const TITLE_LINE: f32 = 24.;
}
pub mod routing {
    // Legacy .routing-confirm-modal: min(512px, 100vw - 32px)。
    pub const RESTORE_WIDTH: f32 = 512.;
    pub const EDITOR_WIDTH: f32 = 896.;
    pub const ICON_PICKER_WIDTH: f32 = 224.;
    pub const SCALE_VALUE_WIDTH: f32 = 48.;
    pub const MATCHER_WIDTH: f32 = 144.;
    pub const CARD_MIN_HEIGHT: f32 = 68.;
    // React .routing-policy-title / .routing-policy-copy p，在线最终级联一致。
    pub const CARD_TITLE: f32 = 16.;
    pub const CARD_TITLE_LINE: f32 = 24.;
    pub const CARD_SUMMARY_LINE: f32 = 16.;
    pub const CARD_COPY_GAP: f32 = 2.;
    // LAN 修改站点集实测：41px header + flex body + 49px footer，固定90dvh。
    pub const EDITOR_HEIGHT_RATIO: f32 = 0.9;
    pub const LABEL_LINE: f32 = 16.;
    pub const RULE_GAP: f32 = 8.;
    pub const RULE_NOTE_WIDTH: f32 = 112.;
    pub const EDITOR_RADIUS: f32 = 13.6;
    pub const FOOTER_BUTTON_WIDTH: f32 = 52.;
    pub const SCALE_BUTTON_LIGHT: u32 = 0xededed;
    pub const SCALE_WIDTH: f32 = 158.;
    pub const FALLBACK_EDITOR_WIDTH: f32 = 672.;
    pub const IMPORT_LIST_HEIGHT: f32 = 240.;
    pub const IMPORT_KIND_WIDTH: f32 = 80.;
    // LAN scrollbar 最终级联 foreground/35，DPR2 实测 thumb 12px、右侧6px。
    pub const EDITOR_SCROLL_TRACK: f32 = 12.;
    pub const EDITOR_SCROLL_THUMB: f32 = 6.;
    pub const EDITOR_SCROLL_INSET: f32 = 3.;
    pub const EDITOR_SCROLL_RADIUS: f32 = 3.;
    pub const EDITOR_SCROLL_ALPHA: f32 = 0.35;
    // 原版select的两个4px线性渐变构成7.9×4的下三角，中心向下1px。
    pub const RULE_CARET_LIGHT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4" viewBox="0 0 8 4"><path fill="#333c4d" d="M0 0h8L4 4z"/></svg>"##;
    pub const RULE_CARET_DARK: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="8" height="4" viewBox="0 0 8 4"><path fill="#cac9c9" d="M0 0h8L4 4z"/></svg>"##;

    pub const SCALE_STEP_WIDTH: f32 = 30.;
    pub const SCALE_RESET_WIDTH: f32 = 52.;
    pub const TOOLBAR_HEIGHT: f32 = 24.;
    pub const TOOLBAR_TEXT: f32 = 11.;
    pub const TOOLBAR_LINE: f32 = 16.5;
    // LAN .btn-xs 内部 icon/text gap 的最终computedStyle，非外部按钮间距4px。
    pub const TOOLBAR_ICON_GAP: f32 = 6.;
    pub const FIELD_OPACITY: f32 = 0.9;
    // 原版 font-mono 的实际face经 CSS.getPlatformFontsForNode 核对为Menlo-Regular。
    pub const RULE_FONT: &str = "Menlo";
    pub const FOOTER_HEIGHT: f32 = 49.;
    pub const CARD_ICON: f32 = 18.;
}
