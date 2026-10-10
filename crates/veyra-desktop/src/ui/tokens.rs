//! 当前 P1 使用的 React 固定几何；源：openbox.css:86–155、820–836、1631–1730。
//! 局部规格保留独立名称，避免把 Panel 覆盖误用于其他页面。
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
// OpenBox .ob-switch 默认尺寸、关闭色；面板局部覆盖沿用 SWITCH_*。
pub const OB_SWITCH_WIDTH: f32 = 32.;
pub const OB_SWITCH_HEIGHT: f32 = 18.;
pub const OB_SWITCH_THUMB: f32 = 14.;
pub const OB_SWITCH_PADDING: f32 = 2.;
pub const OB_SWITCH_TRAVEL: f32 = 14.;
pub const OB_SWITCH_OFF: u32 = 0xd9dedc;
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
pub const SUBSCRIPTION_NOTICE_PX: f32 = 10.;
pub const SUBSCRIPTION_NOTICE_PY: f32 = 8.;
pub const SUBSCRIPTION_ERROR: u32 = 0xf06b6b;

// OpenBox .btn-primary：本机真实页面 OKLCH 转 sRGB；hover 为 primary 混入 7% 黑色。
pub const PRIMARY_BUTTON_TEXT: u32 = 0x223d30;
pub const PRIMARY_BUTTON_BG: u32 = 0x66cc8a;
pub const PRIMARY_BUTTON_HOVER: u32 = 0x5cb97d;
pub const PRIMARY_BUTTON_RADIUS: f32 = 9.3;
pub const REFERENCE_BUTTON_TRANSITION_MS: u64 = 200;
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
pub const PANEL_TOOLBAR_ALPHA: f32 = 0.2;
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
pub const SUBSCRIPTION_SHARE_SAVE_WEIGHT: f32 = 520.;
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
    pub const LANE_SEARCH_GAP: f32 = 5.;
    pub const LANE_SEARCH_ICON: f32 = 13.;
    pub const LANE_MODE_WIDTH: f32 = 92.;
    pub const LANE_MODE_HEIGHT: f32 = 28.;
    pub const LANE_MODE_TEXT: f32 = 10.;
    pub const LANE_HEAD_HEIGHT: f32 = 38.;
    pub const LANE_SETTINGS_HEIGHT: f32 = 36.;
    pub const LANE_TAB_HEIGHT: f32 = 31.;
    pub const LANE_TAB_PAD: f32 = 13.;
    pub const LANE_TAB_BAR: f32 = 39.;
    pub const LANE_SETTINGS_PAD: f32 = 7.;
    pub const LANE_ICON_WIDTH: f32 = 145.;
    pub const LANE_DELETE_WIDTH: f32 = 34.;
    pub const FAILOVER_MIN_HEIGHT: f32 = 322.;
    pub const ADVANCED_TIMEOUT: f32 = 120.;
    pub const ADVANCED_FAILURE: f32 = 140.;
    pub const ADVANCED_RECOVERY: f32 = 140.;
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
    pub const TABS_WIDTH: f32 = 124.;
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
    pub const TOOLBAR: f32 = 48.;
    pub const SURFACE_ALPHA: f32 = 0.75;
    pub const SEARCH_ICON: f32 = 15.;
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
