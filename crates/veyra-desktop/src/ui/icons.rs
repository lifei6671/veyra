//! 与 React 相同的 Heroicons 2.2.0 outline；来源和 MIT 许可见 assets/heroicons。
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;
pub fn icon(name: &str, size: f32) -> Icon {
    let bytes: &[u8] = match name {
        "MagnifyingGlass" => include_bytes!("../../assets/heroicons/MagnifyingGlass.svg"),
        "Power" => include_bytes!("../../assets/heroicons/Power.svg"),
        "Bolt" => include_bytes!("../../assets/heroicons/Bolt.svg"),
        "ExclamationTriangle" => include_bytes!("../../assets/heroicons/ExclamationTriangle.svg"),
        "Bars3" => include_bytes!("../../assets/heroicons/Bars3.svg"),
        "ClipboardDocument" => include_bytes!("../../assets/heroicons/ClipboardDocument.svg"),
        "NoSymbol" => include_bytes!("../../assets/heroicons/NoSymbol.svg"),
        "ArrowRight" => include_bytes!("../../assets/heroicons/ArrowRight.svg"),
        "Plus" => include_bytes!("../../assets/heroicons/Plus.svg"),
        "CheckCircle" => include_bytes!("../../assets/heroicons/CheckCircle.svg"),
        "InformationCircle" => include_bytes!("../../assets/heroicons/InformationCircle.svg"),
        "XCircle" => include_bytes!("../../assets/heroicons/XCircle.svg"),
        "Check" => include_bytes!("../../assets/heroicons/Check.svg"),
        "Home" => include_bytes!("../../assets/heroicons/Home.svg"),
        "GlobeAlt" => include_bytes!("../../assets/heroicons/GlobeAlt.svg"),
        "ArrowsRightLeft" => include_bytes!("../../assets/heroicons/ArrowsRightLeft.svg"),
        "DocumentText" => include_bytes!("../../assets/heroicons/DocumentText.svg"),
        "Funnel" => include_bytes!("../../assets/heroicons/Funnel.svg"),
        "Cog6Tooth" => include_bytes!("../../assets/heroicons/Cog6Tooth.svg"),
        "Play" => include_bytes!("../../assets/heroicons/Play.svg"),
        "Stop" => include_bytes!("../../assets/heroicons/Stop.svg"),
        "ArrowPathRoundedSquare" => {
            include_bytes!("../../assets/heroicons/ArrowPathRoundedSquare.svg")
        }
        "ArrowPath" => include_bytes!("../../assets/heroicons/ArrowPath.svg"),
        "PencilSquare" => include_bytes!("../../assets/heroicons/PencilSquare.svg"),
        "Trash" => include_bytes!("../../assets/heroicons/Trash.svg"),
        "Rss" => include_bytes!("../../assets/heroicons/Rss.svg"),
        "RectangleStack" => include_bytes!("../../assets/heroicons/RectangleStack.svg"),
        "Map" => include_bytes!("../../assets/heroicons/Map.svg"),
        "DevicePhoneMobile" => include_bytes!("../../assets/heroicons/DevicePhoneMobile.svg"),
        "Link" => include_bytes!("../../assets/heroicons/Link.svg"),
        "QrCode" => include_bytes!("../../assets/heroicons/QrCode.svg"),
        "Share" => include_bytes!("../../assets/heroicons/Share.svg"),
        "ServerStack" => include_bytes!("../../assets/heroicons/ServerStack.svg"),
        "CircleStack" => include_bytes!("../../assets/heroicons/CircleStack.svg"),
        "ArrowTopRightOnSquare" => {
            include_bytes!("../../assets/heroicons/ArrowTopRightOnSquare.svg")
        }
        "ArrowDownTray" => include_bytes!("../../assets/heroicons/ArrowDownTray.svg"),
        "Github" => include_bytes!("../../assets/heroicons/Github.svg"),
        "CpuChip" => include_bytes!("../../assets/heroicons/CpuChip.svg"),
        "ArrowUpTray" => include_bytes!("../../assets/heroicons/ArrowUpTray.svg"),
        "AdjustmentsHorizontal" => {
            include_bytes!("../../assets/heroicons/AdjustmentsHorizontal.svg")
        }
        "ArrowUturnLeft" => include_bytes!("../../assets/heroicons/ArrowUturnLeft.svg"),
        "QuestionMarkCircle" => include_bytes!("../../assets/heroicons/QuestionMarkCircle.svg"),
        "ChevronUp" => include_bytes!("../../assets/heroicons/ChevronUp.svg"),
        "ChevronDown" => include_bytes!("../../assets/heroicons/ChevronDown.svg"),
        "XMark" => include_bytes!("../../assets/heroicons/XMark.svg"),
        "SidebarToggle" => include_bytes!("../../assets/heroicons/SidebarToggle.svg"),
        "Sparkles" => include_bytes!("../../assets/heroicons/Sparkles.svg"),
        "Minus" => include_bytes!("../../assets/heroicons/Minus.svg"),
        "ChevronLeft" => include_bytes!("../../assets/heroicons/ChevronLeft.svg"),
        "ChevronRight" => include_bytes!("../../assets/heroicons/ChevronRight.svg"),
        _ => unreachable!("未登记的 React 图标"),
    };
    Icon::default()
        .data(bytes)
        .with_size(px(size))
        .flex_shrink_0()
}
