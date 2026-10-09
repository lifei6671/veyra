//! 两种分享界面共用编码与绘制；value 即复制的完整 URI，不截断凭据或查询参数。
use gpui_kit::{prelude::*, *};
pub fn qr_code(value: &str, extent: f32) -> AnyElement {
    let Ok(code) =
        qrcode::QrCode::with_error_correction_level(value.as_bytes(), qrcode::EcLevel::M)
    else {
        return div().into_any_element();
    };
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            window.paint_quad(fill(bounds, rgb(0xffffff)));
            let n = code.width();
            let quiet = crate::ui::tokens::SUBSCRIPTION_SHARE_QR_QUIET_MODULES;
            let cell = bounds.size.width / (n as f32 + quiet * 2.);
            for y in 0..n {
                for x in 0..n {
                    if code[(x, y)] == qrcode::Color::Dark {
                        window.paint_quad(fill(
                            Bounds::new(
                                bounds.origin
                                    + point(cell * (x as f32 + quiet), cell * (y as f32 + quiet)),
                                size(cell, cell),
                            ),
                            rgb(0x000000),
                        ));
                    }
                }
            }
        },
    )
    .size(px(extent))
    .into_any_element()
}
