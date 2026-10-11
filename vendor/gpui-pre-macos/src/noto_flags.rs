//! 原 Noto Flags 的窄接线：字节保留、原 GSUB、原 CBLC/CBDT，不注册系统字体。
use anyhow::{anyhow, bail};
use core_graphics::{
    base::{kCGImageAlphaLast, kCGImageAlphaPremultipliedLast},
    color_space::CGColorSpace,
    context::{CGContext, CGInterpolationQuality},
    data_provider::CGDataProvider,
    geometry::{CGPoint, CGRect, CGSize},
    image::CGImage,
};
use gpui::{
    Bounds, DevicePixels, FontFeatures, FontMetrics, GlyphId, RenderGlyphParams, Result,
    SUBPIXEL_VARIANTS_X, point, size, swap_rgba_pa_to_bgra,
};
use rustybuzz::ttf_parser::{Face, RasterGlyphImage, RasterImageFormat};
use std::{borrow::Cow, sync::Arc};

pub(super) const FAMILY: &str = "Noto Color Emoji Flags";
pub(super) struct NotoFlags {
    pub bytes: Cow<'static, [u8]>,
}
impl NotoFlags {
    pub fn from_bytes(bytes: Cow<'static, [u8]>) -> Result<Option<Self>> {
        let Ok(face) = Face::parse(&bytes, 0) else {
            return Ok(None);
        };
        let is_original_family = face.names().into_iter().any(|name| {
            name.name_id == 6 && name.to_string().as_deref() == Some("NotoColorEmojiFlags")
        });
        if !is_original_family {
            return Ok(None);
        }
        if face.tables().cbdt.is_none() || face.tables().gsub.is_none() {
            bail!("Noto Flags lacks original CBDT/GSUB")
        }
        Ok(Some(Self { bytes }))
    }
    fn face(&self) -> Face<'_> {
        Face::parse(&self.bytes, 0).expect("validated original Noto Flags")
    }
    pub fn glyph_for_char(&self, ch: char) -> Option<GlyphId> {
        self.face().glyph_index(ch).map(|g| GlyphId(g.0 as u32))
    }
    pub fn shape_regionals(
        &self,
        text: &str,
        features: &FontFeatures,
    ) -> Option<Vec<(GlyphId, usize)>> {
        let face = rustybuzz::Face::from_slice(&self.bytes, 0)?;
        let mut buffer = rustybuzz::UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let features = features
            .tag_value_list()
            .iter()
            .filter_map(|(tag, value)| format!("{tag}={value}").parse().ok())
            .collect::<Vec<rustybuzz::Feature>>();
        let shaped = rustybuzz::shape(&face, &features, buffer);
        let info = shaped.glyph_infos();
        let mut glyphs = Vec::new();
        for info in info {
            if info.glyph_id == 0 {
                return None;
            }
            let glyph = GlyphId(info.glyph_id);
            self.raster(glyph).ok()?;
            glyphs.push((glyph, info.cluster as usize));
        }
        if glyphs.is_empty() {
            None
        } else {
            Some(glyphs)
        }
    }
    fn raster(&self, glyph: GlyphId) -> Result<RasterGlyphImage<'_>> {
        // CBDT table 的 get 保留原 bytes lifetime；不返回临时 Face 的借用。
        let table = self
            .face()
            .tables()
            .cbdt
            .ok_or_else(|| anyhow!("Noto Flags CBDT missing"))?;
        table
            .get(rustybuzz::ttf_parser::GlyphId(glyph.0 as u16), u16::MAX)
            .ok_or_else(|| anyhow!("Noto Flags glyph has no original strike"))
    }
    // parser 返回 image/bearing，没有 advance；本原件 format17 的 small metrics
    // 紧邻原 PNG length。验证原布局后读取真实 advance，绝不以 image width 猜。
    pub fn strike_advance(&self, glyph: GlyphId) -> Result<f32> {
        let r = self.raster(glyph)?;
        if r.format != RasterImageFormat::PNG {
            bail!("Noto Flags strike is not PNG")
        }
        let offset = r.data.as_ptr() as usize - self.bytes.as_ptr() as usize;
        let m = offset
            .checked_sub(9)
            .and_then(|start| self.bytes.get(start..offset))
            .ok_or_else(|| anyhow!("Noto Flags small metrics missing"))?;
        if m[0] as u16 != r.height
            || m[1] as u16 != r.width
            || m[2] as i8 as i16 != r.x
            || m[3] as i8 as i16 - r.height as i16 != r.y
            || u32::from_be_bytes(m[5..9].try_into().unwrap()) as usize != r.data.len()
        {
            bail!("Unsupported Noto Flags strike metrics")
        }
        Ok(m[4] as f32 / r.pixels_per_em as f32)
    }
    pub fn advance_at(&self, glyph: GlyphId, font_size: f32) -> Result<f32> {
        // 与实际 React 14/16px 测量对应的 Skia 26.6 候选模型；Native实测继续核验。
        Ok((self.strike_advance(glyph)? * font_size * 64.).round() / 64.)
    }
    pub fn metrics(&self) -> FontMetrics {
        let f = self.face();
        let b = f.global_bounding_box();
        FontMetrics {
            units_per_em: f.units_per_em() as u32,
            ascent: f.ascender() as f32,
            descent: f.descender() as f32,
            line_gap: f.line_gap() as f32,
            underline_position: f.underline_metrics().map_or(0., |m| m.position as f32),
            underline_thickness: f.underline_metrics().map_or(0., |m| m.thickness as f32),
            cap_height: f.capital_height().unwrap_or(0) as f32,
            x_height: f.x_height().unwrap_or(0) as f32,
            bounding_box: Bounds {
                origin: point(b.x_min as f32, b.y_min as f32),
                size: size((b.x_max - b.x_min) as f32, (b.y_max - b.y_min) as f32),
            },
        }
    }
    pub fn bounds_units(&self, glyph: GlyphId) -> Result<Bounds<f32>> {
        let r = self.raster(glyph)?;
        // 先用原整数度量在 f64 转换，再舍入至平台 f32；避免 ratio先舍入
        // 给 -27/109 原 bottom bound 引入额外误差。
        let scale = self.face().units_per_em() as f64 / r.pixels_per_em as f64;
        Ok(Bounds {
            // 与 FontKit typographic_bounds 相同的 y-up font units；
            // raster bounds 则另行转换为 GPUI device y-down。
            origin: point((r.x as f64 * scale) as f32, (r.y as f64 * scale) as f32),
            size: size(
                (r.width as f64 * scale) as f32,
                (r.height as f64 * scale) as f32,
            ),
        })
    }
    pub fn raster_bounds(&self, p: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        let r = self.raster(p.glyph_id)?;
        let scale = f32::from(p.font_size) * p.scale_factor / r.pixels_per_em as f32;
        let x = r.x as f32 * scale;
        let y = -(r.y as f32 + r.height as f32) * scale;
        let left = x.floor() as i32 - 1;
        let top = y.floor() as i32 - 1;
        let right = (x + r.width as f32 * scale).ceil() as i32 + 1;
        let bottom = (y + r.height as f32 * scale).ceil() as i32 + 1;
        Ok(Bounds {
            origin: point(DevicePixels(left), DevicePixels(top)),
            size: size(DevicePixels(right - left), DevicePixels(bottom - top)),
        })
    }
    pub fn rasterize(
        &self,
        p: &RenderGlyphParams,
        b: Bounds<DevicePixels>,
    ) -> Result<(gpui::Size<DevicePixels>, Vec<u8>)> {
        let r = self.raster(p.glyph_id)?;
        let image = image::load_from_memory(r.data)?.to_rgba8();
        if image.width() != r.width as u32 || image.height() != r.height as u32 {
            bail!("Noto Flags PNG dimensions disagree")
        }
        let provider = CGDataProvider::from_buffer(Arc::new(image.into_raw()));
        let cg = CGImage::new(
            r.width as usize,
            r.height as usize,
            8,
            32,
            r.width as usize * 4,
            &CGColorSpace::create_device_rgb(),
            kCGImageAlphaLast,
            &provider,
            true,
            0,
        );
        let mut dimensions = b.size;
        if p.subpixel_variant.x > 0 {
            dimensions.width += DevicePixels(1)
        }
        if p.subpixel_variant.y > 0 {
            dimensions.height += DevicePixels(1)
        }
        let mut bytes = vec![0; dimensions.width.0 as usize * dimensions.height.0 as usize * 4];
        let cx = CGContext::create_bitmap_context(
            Some(bytes.as_mut_ptr().cast()),
            dimensions.width.0 as usize,
            dimensions.height.0 as usize,
            8,
            dimensions.width.0 as usize * 4,
            &CGColorSpace::create_device_rgb(),
            kCGImageAlphaPremultipliedLast,
        );
        // 同态原字体细边缘实验：High 保持原轮廓/bounds，减少默认缩小插值残差。
        cx.set_interpolation_quality(CGInterpolationQuality::CGInterpolationQualityHigh);
        cx.translate(
            -b.origin.x.0 as f64,
            (b.origin.y.0 + b.size.height.0) as f64,
        );
        cx.scale(p.scale_factor as f64, p.scale_factor as f64);
        let scale = f32::from(p.font_size) / r.pixels_per_em as f32;
        let shift = p
            .subpixel_variant
            .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32 / p.scale_factor);
        cx.draw_image(
            CGRect::new(
                &CGPoint::new(
                    (r.x as f32 * scale + shift.x) as f64,
                    (r.y as f32 * scale + shift.y) as f64,
                ),
                &CGSize::new(
                    (r.width as f32 * scale) as f64,
                    (r.height as f32 * scale) as f64,
                ),
            ),
            &cg,
        );
        // 复用 GPUI native emoji 的 premultiplied RGBA → straight BGRA 合同。
        for pixel in bytes.chunks_exact_mut(4) {
            swap_rgba_pa_to_bgra(pixel)
        }
        Ok((dimensions, bytes))
    }
}
