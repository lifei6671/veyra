//! Dock 与托盘共用用户提供的图案；仅调整展示留白，不改写原始资源。
use image::{RgbaImage, imageops};

pub fn image(size: u32, artwork_extent: u32) -> RgbaImage {
    let source = image::load_from_memory(include_bytes!("../../../src-tauri/icons/icon.png"))
        .expect("bundled application icon")
        .to_rgba8();
    // 当前 1024px 原图主体位于 (195,176)..(849,840)。保留四周抗锯齿，
    // 去掉多余透明画布；方形裁区避免拉伸。Dock 保留外边距，托盘用满裁区。
    let artwork = imageops::crop_imm(&source, 184, 164, 680, 680).to_image();
    let extent = artwork_extent;
    let artwork = imageops::resize(&artwork, extent, extent, imageops::FilterType::Lanczos3);
    let mut canvas = RgbaImage::new(size, size);
    let inset = (size - extent) / 2;
    imageops::overlay(&mut canvas, &artwork, inset.into(), inset.into());
    canvas
}
