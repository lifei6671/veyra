use anyhow::anyhow;
use cocoa::appkit::CGFloat;
use collections::{HashMap, HashSet};
use core_foundation::{
    array::{CFArray, CFArrayRef},
    attributed_string::CFMutableAttributedString,
    base::{CFRange, CFType, TCFType},
    number::CFNumber,
    string::CFString,
};
use core_graphics::{
    base::{CGGlyph, kCGImageAlphaPremultipliedLast},
    color_space::CGColorSpace,
    context::{CGContext, CGTextDrawingMode},
    display::CGPoint,
};
use core_text::{
    font::CTFont,
    font_collection::CTFontCollectionRef,
    font_descriptor::{
        CTFontDescriptor, kCTFontSlantTrait, kCTFontSymbolicTrait, kCTFontWeightTrait,
        kCTFontWidthTrait,
    },
    line::CTLine,
    string_attributes::kCTFontAttributeName,
};
use font_kit::{
    font::Font as FontKitFont,
    handle::Handle,
    hinting::HintingOptions,
    metrics::Metrics,
    properties::{Style as FontkitStyle, Weight as FontkitWeight},
    source::SystemSource,
    sources::mem::MemSource,
};
use gpui::{
    Bounds, DevicePixels, Font, FontFallbacks, FontFeatures, FontId, FontMetrics, FontRun,
    FontStyle, FontWeight, GlyphId, Hsla, LineLayout, Pixels, PlatformTextSystem,
    RenderGlyphParams, Result, Rgba, SUBPIXEL_VARIANTS_X, ShapedGlyph, ShapedRun, SharedString,
    Size, TextRenderingMode, point, px, size, swap_rgba_pa_to_bgra,
};
use parking_lot::{RwLock, RwLockUpgradableReadGuard};
use pathfinder_geometry::{
    rect::{RectF, RectI},
    transform2d::Transform2F,
    vector::Vector2F,
};
use smallvec::SmallVec;
use std::{
    borrow::Cow,
    char,
    convert::TryFrom,
    hash::{Hash, Hasher},
    sync::OnceLock,
};

use crate::open_type::apply_features_and_fallbacks;
#[path = "noto_flags.rs"]
mod noto_flags;
use noto_flags::NotoFlags;

// 原 native 字体保持 FontKit；仅原 Noto Flags 使用原 bytes 的位图资源。
enum FontResource {
    Native(FontKitFont),
    Flags(NotoFlags),
}
impl FontResource {
    fn native(&self) -> &FontKitFont {
        match self {
            Self::Native(font) => font,
            Self::Flags(_) => unreachable!("flags handled by bitmap branch"),
        }
    }
}

#[allow(non_upper_case_globals)]
const kCGImageAlphaOnly: u32 = 7;

/// macOS text system using CoreText for font shaping.
pub struct MacTextSystem(RwLock<MacTextSystemState>);

#[derive(Clone, PartialEq, Eq, Hash)]
struct FontKey {
    font_family: SharedString,
    font_features: FontFeatures,
    font_fallbacks: Option<FontFallbacks>,
}

// 同 PostScript 的原字节子集仍是不同字体。CoreText 的对象相等/哈希保留
// 实际 face、variation 和 feature 身份；统一字号避免每次 shape 新建 FontId。
#[derive(Clone, PartialEq, Eq)]
struct NativeFontKey(CTFont);
impl NativeFontKey {
    fn new(font: &CTFont) -> Self {
        Self(font.clone_with_font_size(16.0))
    }
}
impl Hash for NativeFontKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe { core_foundation::base::CFHash(self.0.as_CFTypeRef()) }.hash(state);
    }
}

struct MacTextSystemState {
    memory_source: MemSource,
    memory_fonts: Vec<FontKitFont>,
    system_source: SystemSource,
    fonts: Vec<FontResource>,
    flags_font_id: Option<FontId>,
    // 每个 primary 的显式 fallback 顺序；不让原 Noto 越过在前的完整字体。
    flags_policies: HashMap<FontId, (FontFeatures, Vec<FontKitFont>)>,
    font_selections: HashMap<Font, FontId>,
    font_ids_by_native_font: HashMap<NativeFontKey, FontId>,
    font_ids_by_font_key: HashMap<FontKey, SmallVec<[FontId; 4]>>,
    postscript_names_by_font_id: HashMap<FontId, String>,
}

impl MacTextSystem {
    /// Create a new MacTextSystem.
    pub fn new() -> Self {
        Self(RwLock::new(MacTextSystemState {
            memory_source: MemSource::empty(),
            system_source: SystemSource::new(),
            fonts: Vec::new(),
            flags_font_id: None,
            flags_policies: HashMap::default(),
            font_selections: HashMap::default(),
            font_ids_by_native_font: HashMap::default(),
            memory_fonts: Vec::new(),
            font_ids_by_font_key: HashMap::default(),
            postscript_names_by_font_id: HashMap::default(),
        }))
    }
}

impl Default for MacTextSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformTextSystem for MacTextSystem {
    fn add_fonts(&self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        self.0.write().add_fonts(fonts)
    }

    fn all_font_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let collection = core_text::font_collection::create_for_all_families();
        // NOTE: We intentionally avoid using `collection.get_descriptors()` here because
        // it has a memory leak bug in core-text v21.0.0. The upstream code uses
        // `wrap_under_get_rule` but `CTFontCollectionCreateMatchingFontDescriptors`
        // follows the Create Rule (caller owns the result), so it should use
        // `wrap_under_create_rule`. We call the function directly with correct memory management.
        unsafe extern "C" {
            fn CTFontCollectionCreateMatchingFontDescriptors(
                collection: CTFontCollectionRef,
            ) -> CFArrayRef;
        }
        let descriptors: Option<CFArray<CTFontDescriptor>> = unsafe {
            let array_ref =
                CTFontCollectionCreateMatchingFontDescriptors(collection.as_concrete_TypeRef());
            if array_ref.is_null() {
                None
            } else {
                Some(CFArray::wrap_under_create_rule(array_ref))
            }
        };
        let Some(descriptors) = descriptors else {
            return names;
        };
        for descriptor in descriptors.into_iter() {
            names.extend(lenient_font_attributes::family_name(&descriptor));
        }
        if let Ok(fonts_in_memory) = self.0.read().memory_source.all_families() {
            names.extend(fonts_in_memory);
        }
        if self.0.read().flags_font_id.is_some() {
            names.push(noto_flags::FAMILY.into());
        }
        names
    }

    fn font_id(&self, font: &Font) -> Result<FontId> {
        let lock = self.0.upgradable_read();
        if let Some(font_id) = lock.font_selections.get(font) {
            Ok(*font_id)
        } else {
            let mut lock = RwLockUpgradableReadGuard::upgrade(lock);
            let font_key = FontKey {
                font_family: font.family.clone(),
                font_features: font.features.clone(),
                font_fallbacks: font.fallbacks.clone(),
            };
            let candidates = if let Some(font_ids) = lock.font_ids_by_font_key.get(&font_key) {
                font_ids.as_slice()
            } else {
                let font_ids =
                    lock.load_family(&font.family, &font.features, font.fallbacks.as_ref())?;
                lock.font_ids_by_font_key.insert(font_key.clone(), font_ids);
                lock.font_ids_by_font_key[&font_key].as_ref()
            };

            let candidate_properties = candidates
                .iter()
                .map(|font_id| lock.fonts[font_id.0].native().properties())
                .collect::<SmallVec<[_; 4]>>();

            let ix = font_kit::matching::find_best_match(
                &candidate_properties,
                &font_kit::properties::Properties {
                    style: fontkit_style(font.style),
                    weight: fontkit_weight(font.weight),
                    stretch: Default::default(),
                },
            )?;

            let font_id = candidates[ix];
            lock.font_selections.insert(font.clone(), font_id);
            Ok(font_id)
        }
    }

    fn font_metrics(&self, font_id: FontId) -> FontMetrics {
        match &self.0.read().fonts[font_id.0] {
            FontResource::Native(font) => font_kit_metrics_to_metrics(font.metrics()),
            FontResource::Flags(font) => font.metrics(),
        }
    }

    fn typographic_bounds(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Bounds<f32>> {
        match &self.0.read().fonts[font_id.0] {
            FontResource::Native(font) => {
                Ok(bounds_from_rect(font.typographic_bounds(glyph_id.0)?))
            }
            FontResource::Flags(font) => font.bounds_units(glyph_id),
        }
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        self.0.read().advance(font_id, glyph_id)
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        self.0.read().glyph_for_char(font_id, ch)
    }

    fn glyph_raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        self.0.read().raster_bounds(params)
    }

    fn rasterize_glyph(
        &self,
        glyph_id: &RenderGlyphParams,
        raster_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        self.0.read().rasterize_glyph(glyph_id, raster_bounds)
    }

    fn layout_line(&self, text: &str, font_size: Pixels, font_runs: &[FontRun]) -> LineLayout {
        self.0.write().layout_line(text, font_size, font_runs)
    }

    fn recommended_rendering_mode(
        &self,
        _font_id: FontId,
        _font_size: Pixels,
    ) -> TextRenderingMode {
        TextRenderingMode::Grayscale
    }

    fn glyph_dilation_for_color(&self, color: Hsla) -> u8 {
        // When font smoothing is enabled, CoreGraphics thickens glyph strokes by an amount that
        // depends on the foreground color's luminance. We replicate the logic used by CoreGraphics
        // to select between the different levels of dilation.
        if !font_smoothing_allowed_by_user() {
            return 0;
        }
        let rgba: Rgba = color.into();
        let luminance = 0.2126 * rgba.r + 0.7152 * rgba.g + 0.0722 * rgba.b;
        let level = ((4.0 * luminance) + 0.5).floor() as i32;
        level.clamp(0, 4) as u8
    }
}

fn font_smoothing_allowed_by_user() -> bool {
    static ALLOWED: OnceLock<bool> = OnceLock::new();
    *ALLOWED.get_or_init(|| {
        use core_foundation_sys::preferences::{
            CFPreferencesCopyAppValue, kCFPreferencesCurrentApplication,
        };

        let key = CFString::new("AppleFontSmoothing");
        let value_ref = unsafe {
            CFPreferencesCopyAppValue(key.as_concrete_TypeRef(), kCFPreferencesCurrentApplication)
        };
        if value_ref.is_null() {
            return true;
        }
        let value = unsafe { CFType::wrap_under_create_rule(value_ref) };
        let Some(number) = value.downcast_into::<CFNumber>() else {
            return true;
        };
        // Only an explicit value of `0` means that font smoothing is disabled.
        number.to_i64() != Some(0)
    })
}

impl MacTextSystemState {
    fn add_fonts(&mut self, fonts: Vec<Cow<'static, [u8]>>) -> Result<()> {
        let mut handles = Vec::new();
        let mut memory_fonts = Vec::new();
        let mut flags = Vec::new();
        for bytes in fonts {
            if let Some(font) = NotoFlags::from_bytes(bytes.clone())? {
                flags.push(font);
                continue;
            }
            // 从未改原件创建 descriptor，保留无 URL 的内存子集身份，不注册系统字体。
            let descriptor = core_text::font_manager::create_font_descriptor(&bytes)
                .map_err(|()| anyhow!("Could not load an embedded font descriptor."))?;
            let native = core_text::font::new_from_descriptor(&descriptor, 16.0);
            let font = unsafe { FontKitFont::from_native_font(&native) };
            handles.push(Handle::from_native(&font));
            memory_fonts.push(font);
        }
        self.memory_source.add_fonts(handles.into_iter())?;
        self.memory_fonts.extend(memory_fonts);
        for font in flags {
            if let Some(id) = self.flags_font_id {
                let FontResource::Flags(existing) = &self.fonts[id.0] else {
                    unreachable!()
                };
                if existing.bytes != font.bytes {
                    anyhow::bail!("Different Noto Flags resource already registered");
                }
            } else {
                let id = FontId(self.fonts.len());
                self.fonts.push(FontResource::Flags(font));
                self.flags_font_id = Some(id);
                self.postscript_names_by_font_id
                    .insert(id, "NotoColorEmojiFlags".into());
            }
        }
        // add_fonts 后已有 family 的 cascade 也必须包含新添加的原始子集。
        self.font_selections.clear();
        self.font_ids_by_font_key.clear();
        Ok(())
    }

    fn load_family(
        &mut self,
        name: &str,
        features: &FontFeatures,
        fallbacks: Option<&FontFallbacks>,
    ) -> Result<SmallVec<[FontId; 4]>> {
        let name = gpui::font_name_with_fallbacks(name, ".AppleSystemUIFont");

        let mut font_ids = SmallVec::new();
        let mut native_fonts_seen = HashSet::default();
        let family = self
            .memory_source
            .select_family_by_name(name)
            .or_else(|_| self.system_source.select_family_by_name(name))?;
        for font in family.fonts() {
            let mut font = font.load()?;

            // 缺 m 的同 family 子集仅加入 fallback，绝不成为编辑器 primary。
            let primary_fallbacks = self
                .memory_fonts
                .iter()
                .filter(|candidate| {
                    candidate.family_name() == name && candidate.glyph_for_char('m').is_none()
                })
                .map(|candidate| candidate.native_font().copy_descriptor())
                .collect::<Vec<_>>();
            let mut memory_fallbacks = Vec::new();
            // 保持调用者 fallback family 的顺序。普通完整字体仍走原有名称/trait
            // 路径；只有含缺 m 子集的内存 family 才需要 direct descriptor。
            for family in fallbacks
                .into_iter()
                .flat_map(|fallbacks| fallbacks.fallback_list())
            {
                let candidates = self
                    .memory_fonts
                    .iter()
                    .filter(|candidate| candidate.family_name() == *family)
                    .collect::<Vec<_>>();
                if !candidates
                    .iter()
                    .any(|candidate| candidate.glyph_for_char('m').is_none())
                {
                    continue;
                }
                let primaries = candidates
                    .iter()
                    .filter(|candidate| candidate.glyph_for_char('m').is_some())
                    .collect::<Vec<_>>();
                let properties = primaries
                    .iter()
                    .map(|candidate| candidate.properties())
                    .collect::<Vec<_>>();
                let mut descriptors = Vec::new();
                if let Ok(index) =
                    font_kit::matching::find_best_match(&properties, &font.properties())
                {
                    descriptors.push(primaries[index].native_font().copy_descriptor());
                }
                descriptors.extend(
                    candidates
                        .iter()
                        .filter(|candidate| candidate.glyph_for_char('m').is_none())
                        .map(|candidate| candidate.native_font().copy_descriptor()),
                );
                memory_fallbacks.push((family.clone(), descriptors));
            }
            apply_features_and_fallbacks(
                &mut font,
                features,
                fallbacks,
                &primary_fallbacks,
                &memory_fallbacks,
            )?;
            // This block contains a precautionary fix to guard against loading fonts
            // that might cause panics due to `.unwrap()`s up the chain.
            {
                // We use the 'm' character for text measurements in various spots
                // (e.g., the editor). However, at time of writing some of those usages
                // will panic if the font has no 'm' glyph.
                //
                // Therefore, we check up front that the font has the necessary glyph.
                let has_m_glyph = font.glyph_for_char('m').is_some();

                // HACK: The 'Segoe Fluent Icons' font does not have an 'm' glyph,
                // but we need to be able to load it for rendering Windows icons in
                // the Storybook (on macOS).
                let is_segoe_fluent_icons = font.full_name() == "Segoe Fluent Icons";

                if !has_m_glyph && !is_segoe_fluent_icons {
                    // I spent far too long trying to track down why a font missing the 'm'
                    // character wasn't loading. This log statement will hopefully save
                    // someone else from suffering the same fate.
                    log::warn!(
                        "font '{}' has no 'm' character and was not loaded",
                        font.full_name()
                    );
                    continue;
                }
            }

            // We've seen a number of panics in production caused by calling font.properties()
            // which unwraps a downcast to CFNumber. This is an attempt to avoid the panic,
            // and to try and identify the incalcitrant font.
            let traits = font.native_font().all_traits();
            if unsafe {
                !(traits
                    .get(kCTFontSymbolicTrait)
                    .downcast::<CFNumber>()
                    .is_some()
                    && traits
                        .get(kCTFontWidthTrait)
                        .downcast::<CFNumber>()
                        .is_some()
                    && traits
                        .get(kCTFontWeightTrait)
                        .downcast::<CFNumber>()
                        .is_some()
                    && traits
                        .get(kCTFontSlantTrait)
                        .downcast::<CFNumber>()
                        .is_some())
            } {
                log::error!(
                    "Failed to read traits for font {:?} (PostScript name {:?})",
                    font.full_name(),
                    font.postscript_name(),
                );
                continue;
            }

            let Some(postscript_name) = font.postscript_name() else {
                log::warn!(
                    "font {:?} in family {:?} has no PostScript name; skipping",
                    font.full_name(),
                    name,
                );
                continue;
            };
            // 本轮按真实 face 去重，不能按同名 PostScript 合并不同原件子集。
            let native_key = NativeFontKey::new(&font.native_font());
            if !native_fonts_seen.insert(native_key.clone()) {
                continue;
            }
            let font_id = FontId(self.fonts.len());
            font_ids.push(font_id);
            self.font_ids_by_native_font.insert(native_key, font_id);
            self.postscript_names_by_font_id
                .insert(font_id, postscript_name);
            if self.flags_font_id.is_some() {
                if let Some(list) = fallbacks.map(|f| f.fallback_list()) {
                    if let Some(position) =
                        list.iter().position(|family| family == noto_flags::FAMILY)
                    {
                        let mut preceding = vec![font.clone()];
                        preceding.extend(
                            self.memory_fonts
                                .iter()
                                .filter(|f| f.family_name() == name)
                                .cloned(),
                        );
                        for family in &list[..position] {
                            preceding.extend(
                                self.memory_fonts
                                    .iter()
                                    .filter(|f| f.family_name() == *family)
                                    .cloned(),
                            );
                            if let Ok(family) = self.system_source.select_family_by_name(family) {
                                preceding.extend(
                                    family
                                        .fonts()
                                        .iter()
                                        .filter_map(|handle| handle.load().ok()),
                                );
                            }
                        }
                        self.flags_policies
                            .insert(font_id, (features.clone(), preceding));
                    }
                }
            }
            self.fonts.push(FontResource::Native(font));
        }
        Ok(font_ids)
    }

    fn advance(&self, font_id: FontId, glyph_id: GlyphId) -> Result<Size<f32>> {
        match &self.fonts[font_id.0] {
            FontResource::Native(font) => Ok(size_from_vector2f(font.advance(glyph_id.0)?)),
            FontResource::Flags(font) => Ok(size(
                font.strike_advance(glyph_id)? * font.metrics().units_per_em as f32,
                0.,
            )),
        }
    }

    fn glyph_for_char(&self, font_id: FontId, ch: char) -> Option<GlyphId> {
        match &self.fonts[font_id.0] {
            FontResource::Native(font) => font.glyph_for_char(ch).map(GlyphId),
            FontResource::Flags(font) => font.glyph_for_char(ch),
        }
    }

    fn id_for_native_font(&mut self, requested_font: CTFont) -> FontId {
        let key = NativeFontKey::new(&requested_font);
        if let Some(font_id) = self.font_ids_by_native_font.get(&key) {
            *font_id
        } else {
            let font_id = FontId(self.fonts.len());
            // FontKit 单位度量同样使用统一字号的实际 face，避免本次 shape 的
            // next_up 字号把舍入误差带入后续其它字号的共用度量缓存。
            let font = unsafe { FontKitFont::from_native_font(&key.0) };
            self.font_ids_by_native_font.insert(key, font_id);
            self.postscript_names_by_font_id
                .insert(font_id, requested_font.postscript_name());
            self.fonts.push(FontResource::Native(font));
            font_id
        }
    }

    fn is_emoji(&self, font_id: FontId) -> bool {
        self.postscript_names_by_font_id
            .get(&font_id)
            .is_some_and(|postscript_name| {
                postscript_name == "AppleColorEmoji"
                    || postscript_name == ".AppleColorEmojiUI"
                    || postscript_name == "NotoColorEmojiFlags"
            })
    }

    fn raster_bounds(&self, params: &RenderGlyphParams) -> Result<Bounds<DevicePixels>> {
        let font = match &self.fonts[params.font_id.0] {
            FontResource::Native(font) => font,
            FontResource::Flags(font) => return font.raster_bounds(params),
        };
        let scale = Transform2F::from_scale(params.scale_factor);
        let bounds: Bounds<DevicePixels> = bounds_from_rect_i(font.raster_bounds(
            params.glyph_id.0,
            params.font_size.into(),
            scale,
            HintingOptions::None,
            font_kit::canvas::RasterizationOptions::GrayscaleAa,
        )?);

        // Expand the bounds by 1 pixel on each side to give CG room for anti-aliasing.
        Ok(bounds.dilate(DevicePixels(1)))
    }

    fn rasterize_glyph(
        &self,
        params: &RenderGlyphParams,
        glyph_bounds: Bounds<DevicePixels>,
    ) -> Result<(Size<DevicePixels>, Vec<u8>)> {
        if let FontResource::Flags(font) = &self.fonts[params.font_id.0] {
            return font.rasterize(params, glyph_bounds);
        }
        if glyph_bounds.size.width.0 == 0 || glyph_bounds.size.height.0 == 0 {
            anyhow::bail!("glyph bounds are empty");
        } else {
            // Add an extra pixel when the subpixel variant isn't zero to make room for anti-aliasing.
            let mut bitmap_size = glyph_bounds.size;
            if params.subpixel_variant.x > 0 {
                bitmap_size.width += DevicePixels(1);
            }
            if params.subpixel_variant.y > 0 {
                bitmap_size.height += DevicePixels(1);
            }
            let bitmap_size = bitmap_size;

            let mut bytes;
            let cx;
            if params.is_emoji {
                bytes = vec![0; bitmap_size.width.0 as usize * 4 * bitmap_size.height.0 as usize];
                cx = CGContext::create_bitmap_context(
                    Some(bytes.as_mut_ptr() as *mut _),
                    bitmap_size.width.0 as usize,
                    bitmap_size.height.0 as usize,
                    8,
                    bitmap_size.width.0 as usize * 4,
                    &CGColorSpace::create_device_rgb(),
                    kCGImageAlphaPremultipliedLast,
                );
            } else {
                bytes = vec![0; bitmap_size.width.0 as usize * bitmap_size.height.0 as usize];
                cx = CGContext::create_bitmap_context(
                    Some(bytes.as_mut_ptr() as *mut _),
                    bitmap_size.width.0 as usize,
                    bitmap_size.height.0 as usize,
                    8,
                    bitmap_size.width.0 as usize,
                    &CGColorSpace::create_device_gray(),
                    kCGImageAlphaOnly,
                );
            }

            // Move the origin to bottom left and account for scaling, this
            // makes drawing text consistent with the font-kit's raster_bounds.
            cx.translate(
                -glyph_bounds.origin.x.0 as CGFloat,
                (glyph_bounds.origin.y.0 + glyph_bounds.size.height.0) as CGFloat,
            );
            cx.scale(
                params.scale_factor as CGFloat,
                params.scale_factor as CGFloat,
            );

            let subpixel_shift = params
                .subpixel_variant
                .map(|v| v as f32 / SUBPIXEL_VARIANTS_X as f32);
            cx.set_text_drawing_mode(CGTextDrawingMode::CGTextFill);
            cx.set_allows_antialiasing(true);
            cx.set_should_antialias(true);
            cx.set_allows_font_subpixel_positioning(true);
            cx.set_should_subpixel_position_fonts(true);
            cx.set_allows_font_subpixel_quantization(false);
            cx.set_should_subpixel_quantize_fonts(false);

            if params.dilation > 0 {
                let luminance = params.dilation as f64 * 0.25;
                cx.set_should_smooth_fonts(true);
                cx.set_gray_fill_color(luminance, 1.0);
            } else {
                cx.set_gray_fill_color(0.0, 1.0);
            }
            self.fonts[params.font_id.0]
                .native()
                .native_font()
                .clone_with_font_size(f32::from(params.font_size) as CGFloat)
                .draw_glyphs(
                    &[params.glyph_id.0 as CGGlyph],
                    &[CGPoint::new(
                        (subpixel_shift.x / params.scale_factor) as CGFloat,
                        (subpixel_shift.y / params.scale_factor) as CGFloat,
                    )],
                    cx,
                );

            if params.is_emoji {
                // Convert from RGBA with premultiplied alpha to BGRA with straight alpha.
                for pixel in bytes.chunks_exact_mut(4) {
                    swap_rgba_pa_to_bgra(pixel);
                }
            }

            Ok((bitmap_size, bytes))
        }
    }

    fn layout_line(&mut self, text: &str, font_size: Pixels, font_runs: &[FontRun]) -> LineLayout {
        let Some(flags_id) = self.flags_font_id else {
            return self.layout_native_line(text, font_size, font_runs);
        };
        let FontResource::Flags(flags) = &self.fonts[flags_id.0] else {
            unreachable!()
        };
        let mut pairs = Vec::new();
        let mut start = 0;
        for run in font_runs {
            let slice = &text[start..start + run.len];
            if let Some((features, preceding)) = self.flags_policies.get(&run.font_id) {
                let mut chars = slice.char_indices().peekable();
                while let Some((ix, first)) = chars.next() {
                    let is_regional = |ch: char| ('\u{1f1e6}'..='\u{1f1ff}').contains(&ch);
                    if !is_regional(first) {
                        continue;
                    }
                    let second = chars.peek().copied().filter(|(_, ch)| is_regional(*ch));
                    let end = second.map_or(ix + first.len_utf8(), |(next_ix, ch)| {
                        next_ix + ch.len_utf8()
                    });
                    // 同 primary 的真实子集及显式前序 family 均优先；系统
                    // 默认 fallback 在显式 Noto 后，不能抢先替换原件。
                    if !preceding.iter().any(|font| {
                        font.glyph_for_char(first).is_some()
                            && second.is_none_or(|(_, ch)| font.glyph_for_char(ch).is_some())
                    }) {
                        if let Some(glyphs) = flags.shape_regionals(&slice[ix..end], features) {
                            let glyphs = glyphs
                                .into_iter()
                                .map(|(glyph, cluster)| {
                                    (
                                        glyph,
                                        cluster,
                                        flags
                                            .advance_at(glyph, font_size.into())
                                            .expect("validated original strike"),
                                    )
                                })
                                .collect::<Vec<_>>();
                            pairs.push((start + ix, start + end, glyphs));
                        }
                    }
                    if second.is_some() {
                        chars.next();
                    }
                }
            }
            start += run.len;
        }
        if pairs.is_empty() {
            return self.layout_native_line(text, font_size, font_runs);
        }
        let flags_metrics = flags.metrics();
        let mut layout = LineLayout {
            runs: Vec::new(),
            font_size,
            width: px(0.),
            ascent: flags_metrics.ascent(font_size),
            descent: -flags_metrics.descent(font_size),
            len: text.len(),
        };
        // 上游整行合同包含 primary metrics，即使字符全由 fallback 绘制。
        // 保留该约束，再与实际原 Noto metrics 取 max；不以字体诊断图猜高度。
        for run in font_runs {
            let metrics = self.fonts[run.font_id.0].native().metrics();
            let scale = f32::from(font_size) / metrics.units_per_em as f32;
            layout.ascent = layout.ascent.max(px(metrics.ascent * scale));
            layout.descent = layout.descent.max(px(-metrics.descent * scale));
        }
        let mut cursor = 0;
        for (begin, end, glyphs) in pairs {
            self.append_native_span(text, cursor, begin, font_size, font_runs, &mut layout);
            let mut run = ShapedRun {
                font_id: flags_id,
                glyphs: Vec::new(),
            };
            for (glyph, cluster, advance) in glyphs {
                run.glyphs.push(ShapedGlyph {
                    id: glyph,
                    position: point(layout.width, px(0.)),
                    index: begin + cluster,
                    is_emoji: true,
                });
                layout.width += px(advance);
            }
            layout.runs.push(run);
            cursor = end;
        }
        self.append_native_span(text, cursor, text.len(), font_size, font_runs, &mut layout);
        layout
    }

    // 原文本片段仍由 CTLine 提供 native face、glyph、kerning 和 UTF-8 索引；
    // 只在不同实际字体的原 flag 边界连接，不逐字 shape 普通文本。
    fn append_native_span(
        &mut self,
        text: &str,
        begin: usize,
        end: usize,
        font_size: Pixels,
        font_runs: &[FontRun],
        layout: &mut LineLayout,
    ) {
        if begin == end {
            return;
        }
        let mut start = 0;
        let mut span_runs = Vec::new();
        for run in font_runs {
            let finish = start + run.len;
            let len = finish.min(end).saturating_sub(start.max(begin));
            if len > 0 {
                span_runs.push(FontRun {
                    font_id: run.font_id,
                    len,
                });
            }
            start = finish;
        }
        let mut native = self.layout_native_line(&text[begin..end], font_size, &span_runs);
        for run in &mut native.runs {
            for glyph in &mut run.glyphs {
                glyph.index += begin;
                glyph.position.x += layout.width;
            }
        }
        layout.runs.extend(native.runs);
        layout.width += native.width;
        layout.ascent = layout.ascent.max(native.ascent);
        layout.descent = layout.descent.max(native.descent);
    }

    fn layout_native_line(
        &mut self,
        text: &str,
        font_size: Pixels,
        font_runs: &[FontRun],
    ) -> LineLayout {
        // Construct the attributed string, converting UTF8 ranges to UTF16 ranges.
        let mut string = CFMutableAttributedString::new();
        let mut max_ascent = 0.0f32;
        let mut max_descent = 0.0f32;

        {
            let mut text = text;
            let mut break_ligature = true;
            for run in font_runs {
                let text_run;
                (text_run, text) = text.split_at(run.len);

                let utf16_start = string.char_len(); // insert at end of string
                // note: replace_str may silently ignore codepoints it dislikes (e.g., BOM at start of string)
                string.replace_str(&CFString::new(text_run), CFRange::init(utf16_start, 0));
                let utf16_end = string.char_len();

                let length = utf16_end - utf16_start;
                let cf_range = CFRange::init(utf16_start, length);
                let font = self.fonts[run.font_id.0].native();

                let font_metrics = font.metrics();
                let font_scale = f32::from(font_size) / font_metrics.units_per_em as f32;
                max_ascent = max_ascent.max(font_metrics.ascent * font_scale);
                max_descent = max_descent.max(-font_metrics.descent * font_scale);

                let font_size = if break_ligature {
                    px(f32::from(font_size).next_up())
                } else {
                    font_size
                };
                unsafe {
                    string.set_attribute(
                        cf_range,
                        kCTFontAttributeName,
                        &font.native_font().clone_with_font_size(font_size.into()),
                    );
                }
                break_ligature = !break_ligature;
            }
        }
        // Retrieve the glyphs from the shaped line, converting UTF16 offsets to UTF8 offsets.
        let line = CTLine::new_with_attributed_string(string.as_concrete_TypeRef());
        let glyph_runs = line.glyph_runs();
        let mut runs = <Vec<ShapedRun>>::with_capacity(glyph_runs.len() as usize);
        let mut ix_converter = StringIndexConverter::new(text);
        for run in glyph_runs.into_iter() {
            let attributes = run.attributes().unwrap();
            let font = unsafe {
                attributes
                    .get(kCTFontAttributeName)
                    .downcast::<CTFont>()
                    .unwrap()
            };
            let font_id = self.id_for_native_font(font);

            let glyphs = match runs.last_mut() {
                Some(run) if run.font_id == font_id => &mut run.glyphs,
                _ => {
                    runs.push(ShapedRun {
                        font_id,
                        glyphs: Vec::with_capacity(run.glyph_count().try_into().unwrap_or(0)),
                    });
                    &mut runs.last_mut().unwrap().glyphs
                }
            };
            for ((&glyph_id, position), &glyph_utf16_ix) in run
                .glyphs()
                .iter()
                .zip(run.positions().iter())
                .zip(run.string_indices().iter())
            {
                let glyph_utf16_ix = usize::try_from(glyph_utf16_ix).unwrap();
                if ix_converter.utf16_ix > glyph_utf16_ix {
                    // We cannot reuse current index converter, as it can only seek forward. Restart the search.
                    ix_converter = StringIndexConverter::new(text);
                }
                ix_converter.advance_to_utf16_ix(glyph_utf16_ix);
                glyphs.push(ShapedGlyph {
                    id: GlyphId(glyph_id as u32),
                    position: point(position.x as f32, position.y as f32).map(px),
                    index: ix_converter.utf8_ix,
                    is_emoji: self.is_emoji(font_id),
                });
            }
        }
        let typographic_bounds = line.get_typographic_bounds();
        LineLayout {
            runs,
            font_size,
            width: typographic_bounds.width.into(),
            ascent: max_ascent.into(),
            descent: max_descent.into(),
            len: text.len(),
        }
    }
}

#[derive(Debug, Clone)]
struct StringIndexConverter<'a> {
    text: &'a str,
    /// Index in UTF-8 bytes
    utf8_ix: usize,
    /// Index in UTF-16 code units
    utf16_ix: usize,
}

impl<'a> StringIndexConverter<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            utf8_ix: 0,
            utf16_ix: 0,
        }
    }

    fn advance_to_utf16_ix(&mut self, utf16_target: usize) {
        for (ix, c) in self.text[self.utf8_ix..].char_indices() {
            if self.utf16_ix >= utf16_target {
                self.utf8_ix += ix;
                return;
            }
            self.utf16_ix += c.len_utf16();
        }
        self.utf8_ix = self.text.len();
    }
}

fn font_kit_metrics_to_metrics(metrics: Metrics) -> FontMetrics {
    FontMetrics {
        units_per_em: metrics.units_per_em,
        ascent: metrics.ascent,
        descent: metrics.descent,
        line_gap: metrics.line_gap,
        underline_position: metrics.underline_position,
        underline_thickness: metrics.underline_thickness,
        cap_height: metrics.cap_height,
        x_height: metrics.x_height,
        bounding_box: bounds_from_rect(metrics.bounding_box),
    }
}

fn bounds_from_rect(rect: RectF) -> Bounds<f32> {
    Bounds {
        origin: point(rect.origin_x(), rect.origin_y()),
        size: size(rect.width(), rect.height()),
    }
}

fn bounds_from_rect_i(rect: RectI) -> Bounds<DevicePixels> {
    Bounds {
        origin: point(DevicePixels(rect.origin_x()), DevicePixels(rect.origin_y())),
        size: size(DevicePixels(rect.width()), DevicePixels(rect.height())),
    }
}

// impl From<Vector2I> for Size<DevicePixels> {
//     fn from(value: Vector2I) -> Self {
//         size(value.x().into(), value.y().into())
//     }
// }

// impl From<RectI> for Bounds<i32> {
//     fn from(rect: RectI) -> Self {
//         Bounds {
//             origin: point(rect.origin_x(), rect.origin_y()),
//             size: size(rect.width(), rect.height()),
//         }
//     }
// }

// impl From<Point<u32>> for Vector2I {
//     fn from(size: Point<u32>) -> Self {
//         Vector2I::new(size.x as i32, size.y as i32)
//     }
// }

fn size_from_vector2f(vec: Vector2F) -> Size<f32> {
    size(vec.x(), vec.y())
}

fn fontkit_weight(value: FontWeight) -> FontkitWeight {
    FontkitWeight(value.0)
}

fn fontkit_style(style: FontStyle) -> FontkitStyle {
    match style {
        FontStyle::Normal => FontkitStyle::Normal,
        FontStyle::Italic => FontkitStyle::Italic,
        FontStyle::Oblique => FontkitStyle::Oblique,
    }
}

// Some fonts may have no attributes despite `core_text` requiring them (and panicking).
// This is the same version as `core_text` has without `expect` calls.
mod lenient_font_attributes {
    use core_foundation::{
        base::{CFRetain, CFType, TCFType},
        string::{CFString, CFStringRef},
    };
    use core_text::font_descriptor::{
        CTFontDescriptor, CTFontDescriptorCopyAttribute, kCTFontFamilyNameAttribute,
    };

    pub fn family_name(descriptor: &CTFontDescriptor) -> Option<String> {
        unsafe { get_string_attribute(descriptor, kCTFontFamilyNameAttribute) }
    }

    fn get_string_attribute(
        descriptor: &CTFontDescriptor,
        attribute: CFStringRef,
    ) -> Option<String> {
        unsafe {
            let value = CTFontDescriptorCopyAttribute(descriptor.as_concrete_TypeRef(), attribute);
            if value.is_null() {
                return None;
            }

            let value = CFType::wrap_under_create_rule(value);
            assert!(value.instance_of::<CFString>());
            let s = wrap_under_get_rule(value.as_CFTypeRef() as CFStringRef);
            Some(s.to_string())
        }
    }

    unsafe fn wrap_under_get_rule(reference: CFStringRef) -> CFString {
        unsafe {
            assert!(!reference.is_null(), "Attempted to create a NULL object.");
            let reference = CFRetain(reference as *const ::std::os::raw::c_void) as CFStringRef;
            TCFType::wrap_under_create_rule(reference)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::MacTextSystem;
    use gpui::{FontRun, GlyphId, PlatformTextSystem, font, px};

    fn original_flags_fonts() -> MacTextSystem {
        use std::borrow::Cow;
        let fonts = MacTextSystem::new();
        fonts
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!(
                    "../../../src/openbox/assets/fonts/MiSans-VF.94-gh6bDI8D.woff2"
                )),
                Cow::Borrowed(include_bytes!(
                    "../../../src/openbox/assets/fonts/MiSans-VF.95-B0-MMqvr.woff2"
                )),
                Cow::Borrowed(include_bytes!(
                    "../../../src/openbox/assets/fonts/NotoColorEmoji-flagsonly-CWWDk9km.ttf"
                )),
            ])
            .unwrap();
        fonts
    }

    // 保护原 fallback 可见结果：实际 pair GSUB、浏览器字宽、UTF-8 index、
    // CJK 原94的 face 均正确；truncate/wrap 所用 LineLayout.width 不取 hmtx。
    #[test]
    fn test_original_flags_public_shape_and_mixed_clusters() {
        use gpui::{FontFallbacks, FontFeatures};
        let fonts = original_flags_fonts();
        for fallback in [
            vec!["Noto Color Emoji Flags".into()],
            vec!["MiSans VF".into(), "Noto Color Emoji Flags".into()],
        ] {
            let mut face = font("MiSans VF");
            face.fallbacks = Some(FontFallbacks::from_fonts(fallback));
            for features in [FontFeatures::default(), FontFeatures::disable_ligatures()] {
                face.features = features;
                let primary = fonts.font_id(&face).unwrap();
                assert_eq!(fonts.glyph_for_char(primary, 'm'), Some(GlyphId(59)));
                for (text, expected) in [
                    ("🇨🇳", 75),
                    ("🇺🇸", 261),
                    ("🇯🇵", 143),
                    ("🇭🇰", 124),
                    ("🇪🇺", 98),
                    ("🇺🇳", 260),
                    ("🇹🇼", 256),
                ] {
                    for (font_size, expected_width) in [(14., 17.46875), (16., 19.96875)] {
                        let line = fonts.layout_line(
                            text,
                            px(font_size),
                            &[FontRun {
                                font_id: primary,
                                len: text.len(),
                            }],
                        );
                        assert_eq!(line.width, px(expected_width));
                        // 独立原 native path 的 baseline，保留其 f32 运算顺序；
                        // 不用另一个公式的微小舍入差伪报 primary baseline回归。
                        let native = fonts.layout_line(
                            "m",
                            px(font_size),
                            &[FontRun {
                                font_id: primary,
                                len: 1,
                            }],
                        );
                        assert_eq!(
                            line.ascent,
                            px(1900. / 2048. * font_size).max(native.ascent)
                        );
                        assert_eq!(
                            line.descent,
                            px(500. / 2048. * font_size).max(native.descent)
                        );
                        assert_eq!(line.runs.len(), 1);
                        let run = &line.runs[0];
                        assert_eq!(run.font_id, fonts.0.read().flags_font_id.unwrap());
                        assert_eq!(run.glyphs.len(), 1);
                        assert_eq!(run.glyphs[0].id, GlyphId(expected));
                        assert_eq!(run.glyphs[0].index, 0);
                        assert!(run.glyphs[0].is_emoji);
                    }
                }
                let text = "A🇯🇵多个🇺🇸";
                let line = fonts.layout_line(
                    text,
                    px(14.),
                    &[FontRun {
                        font_id: primary,
                        len: text.len(),
                    }],
                );
                let glyphs = line
                    .runs
                    .iter()
                    .flat_map(|run| run.glyphs.iter())
                    .collect::<Vec<_>>();
                assert_eq!(
                    glyphs.iter().map(|g| g.index).collect::<Vec<_>>(),
                    vec![0, 1, 9, 12, 15]
                );
                assert_eq!(glyphs[1].id, GlyphId(143));
                assert_eq!(glyphs[2].id, GlyphId(61));
                assert_eq!(glyphs[3].id, GlyphId(9));
                assert_eq!(glyphs[4].id, GlyphId(261));
                assert!(glyphs.windows(2).all(|g| g[0].position.x < g[1].position.x));
                let pair = fonts.layout_line(
                    "🇯🇵🇺🇸",
                    px(14.),
                    &[FontRun {
                        font_id: primary,
                        len: 16,
                    }],
                );
                assert_eq!(pair.width, px(34.9375));
                assert_eq!(pair.runs[1].glyphs[0].position.x, px(17.46875));
                assert_eq!(
                    fonts
                        .0
                        .read()
                        .fonts
                        .iter()
                        .filter(|f| matches!(f, super::FontResource::Flags(_)))
                        .count(),
                    1
                );
            }
        }
    }

    // 保护用户 literal 单 RI 和显式 ccmp=0 的原字体/cluster，不能因非
    // 默认 feature 退回系统emoji；仍只处理原字体含有的regional-indicator。
    #[test]
    fn test_original_flags_single_regional_and_explicit_ccmp() {
        use gpui::{FontFallbacks, FontFeatures};
        use std::sync::Arc;
        let fonts = original_flags_fonts();
        let mut face = font("MiSans VF");
        face.fallbacks = Some(FontFallbacks::from_fonts(vec![
            "Noto Color Emoji Flags".into(),
        ]));
        let primary = fonts.font_id(&face).unwrap();
        let single = fonts.layout_line(
            "🇯",
            px(14.),
            &[FontRun {
                font_id: primary,
                len: 4,
            }],
        );
        assert_eq!(
            single.runs[0].font_id,
            fonts.0.read().flags_font_id.unwrap()
        );
        assert_eq!(single.runs[0].glyphs[0].id, GlyphId(10));
        face.features = FontFeatures(Arc::new(vec![("ccmp".into(), 0)]));
        let primary = fonts.font_id(&face).unwrap();
        let text = "A🇯🇵多";
        let line = fonts.layout_line(
            text,
            px(14.),
            &[FontRun {
                font_id: primary,
                len: text.len(),
            }],
        );
        let original = &line.runs[1];
        assert_eq!(original.font_id, fonts.0.read().flags_font_id.unwrap());
        assert_eq!(
            original.glyphs.iter().map(|g| g.id.0).collect::<Vec<_>>(),
            vec![10, 16]
        );
        assert_eq!(
            original.glyphs.iter().map(|g| g.index).collect::<Vec<_>>(),
            // 原 shaper 同 grapheme cluster：即使 ccmp=0 也不能拆断 pair。
            vec![1, 1]
        );
    }

    // 保护显式 family 顺序：在前的真实 AppleColorEmoji 不可被原 Noto 越过；
    // 无原 Noto fallback 的普通 native 文本行为仍走上游 CTLine。
    #[test]
    fn test_original_flags_explicit_fallback_order() {
        use gpui::FontFallbacks;
        let fonts = original_flags_fonts();
        for (families, want_original) in [
            (
                vec!["Apple Color Emoji".into(), "Noto Color Emoji Flags".into()],
                false,
            ),
            (
                vec!["Noto Color Emoji Flags".into(), "Apple Color Emoji".into()],
                true,
            ),
            (Vec::new(), false),
        ] {
            let mut face = font("MiSans VF");
            face.fallbacks = Some(FontFallbacks::from_fonts(families));
            let primary = fonts.font_id(&face).unwrap();
            let line = fonts.layout_line(
                "🇯🇵",
                px(14.),
                &[FontRun {
                    font_id: primary,
                    len: 8,
                }],
            );
            assert_eq!(
                line.runs[0].font_id == fonts.0.read().flags_font_id.unwrap(),
                want_original
            );
        }
        // Noto 是缺 m 的 fallback-only，不能破坏 GPUI primary 测量前提。
        assert!(fonts.font_id(&font("Noto Color Emoji Flags")).is_err());
    }

    // 保护原 CBDT 基线、bounds、BGRA stride/alpha 与 DPR atlas 合同。109px
    // 的内部 opaque 像素独立对照原PNG，避免只比较同一缓存/renderer。
    #[test]
    fn test_original_flags_raster_baseline_bgra_and_dpr() {
        use gpui::{DevicePixels, FontFallbacks, RenderGlyphParams, point};
        let fonts = original_flags_fonts();
        let mut face = font("MiSans VF");
        face.fallbacks = Some(FontFallbacks::from_fonts(vec![
            "Noto Color Emoji Flags".into(),
        ]));
        let primary = fonts.font_id(&face).unwrap();
        let line = fonts.layout_line(
            "🇨🇳",
            px(14.),
            &[FontRun {
                font_id: primary,
                len: 8,
            }],
        );
        let flags_id = line.runs[0].font_id;
        let raw = rustybuzz::ttf_parser::Face::parse(
            include_bytes!(
                "../../../src/openbox/assets/fonts/NotoColorEmoji-flagsonly-CWWDk9km.ttf"
            ),
            0,
        )
        .unwrap();
        let bounds_units = fonts.typographic_bounds(flags_id, GlyphId(75)).unwrap();
        assert_eq!(bounds_units.origin, point(0., -27. * 2048. / 109.));
        assert_eq!(
            bounds_units.size,
            gpui::size(136. * 2048. / 109., 128. * 2048. / 109.)
        );
        let source = raw
            .glyph_raster_image(rustybuzz::ttf_parser::GlyphId(75), 109)
            .unwrap();
        let png = image::load_from_memory(source.data).unwrap().to_rgba8();
        for (font_size, dpr, expected) in [
            (109., 1., (-1, -102, 138, 130)),
            (14., 1., (-1, -14, 20, 19)),
            (14., 2., (-1, -27, 37, 35)),
        ] {
            for subpixel in [point(0, 0), point(1, 2)] {
                let params = RenderGlyphParams {
                    font_id: flags_id,
                    glyph_id: GlyphId(75),
                    font_size: px(font_size),
                    subpixel_variant: subpixel,
                    scale_factor: dpr,
                    is_emoji: true,
                    subpixel_rendering: false,
                    dilation: 0,
                };
                let bounds = fonts.glyph_raster_bounds(&params).unwrap();
                assert_eq!(
                    (
                        bounds.origin.x.0,
                        bounds.origin.y.0,
                        bounds.size.width.0,
                        bounds.size.height.0
                    ),
                    expected
                );
                let (dimensions, bytes) = fonts.rasterize_glyph(&params, bounds).unwrap();
                assert_eq!(
                    dimensions.width,
                    bounds.size.width + DevicePixels(if subpixel.x > 0 { 1 } else { 0 })
                );
                assert_eq!(
                    dimensions.height,
                    bounds.size.height + DevicePixels(if subpixel.y > 0 { 1 } else { 0 })
                );
                assert_eq!(
                    bytes.len(),
                    dimensions.width.0 as usize * dimensions.height.0 as usize * 4
                );
                assert!(bytes.chunks_exact(4).any(|p| p[3] == 255 && p[2] > p[0]));
                assert_eq!(&bytes[0..4], &[0, 0, 0, 0]);
                if font_size == 109. && subpixel == point(0, 0) {
                    for (x, y, pixel) in png.enumerate_pixels().filter(|(_, _, p)| p[3] == 255) {
                        let offset = (((y + 1) * dimensions.width.0 as u32 + x + 1) * 4) as usize;
                        assert_eq!(
                            &bytes[offset..offset + 4],
                            &[pixel[2], pixel[1], pixel[0], 255]
                        );
                    }
                }
                // Alpha 是 straight BGRA；边缘红色不应仍被 alpha 乘暗。
                assert!(
                    bytes
                        .chunks_exact(4)
                        .any(|p| p[3] > 0 && p[3] < 255 && p[2] > p[3])
                );
                if subpixel == point(0, 0) {
                    if let Ok(path) = std::env::var("VEYRA_FONT_OWNER_EVIDENCE") {
                        let rgba = bytes
                            .chunks_exact(4)
                            .flat_map(|p| [p[2], p[1], p[0], p[3]])
                            .collect::<Vec<_>>();
                        image::RgbaImage::from_raw(
                            dimensions.width.0 as u32,
                            dimensions.height.0 as u32,
                            rgba,
                        )
                        .unwrap()
                        .save(std::path::Path::new(&path).join(format!(
                            "font-owner-noto-public-raster-{font_size}px-dpr{dpr}.png"
                        )))
                        .unwrap();
                    }
                }
            }
        }
    }

    // 保护 P2-08 原件 fallback 的可观察结果：glyph、度量与 raster 都来自
    // 实际原94；同 PostScript 的原95不可复用其 FontId，primary 的 m仍有效。
    #[test]
    fn test_original_subsets_shape_metrics_and_raster() {
        use gpui::{FontFallbacks, FontFeatures, RenderGlyphParams, point};
        use std::borrow::Cow;
        let original94 =
            include_bytes!("../../../src/openbox/assets/fonts/MiSans-VF.94-gh6bDI8D.woff2");
        let original95 =
            include_bytes!("../../../src/openbox/assets/fonts/MiSans-VF.95-B0-MMqvr.woff2");
        let fonts = MacTextSystem::new();
        fonts
            .add_fonts(vec![Cow::Borrowed(original94), Cow::Borrowed(original95)])
            .unwrap();
        let raw94 = unsafe {
            font_kit::font::Font::from_native_font(
                &core_text::font::new_from_buffer(original94).unwrap(),
            )
        };
        let raw95 = unsafe {
            font_kit::font::Font::from_native_font(
                &core_text::font::new_from_buffer(original95).unwrap(),
            )
        };
        assert_eq!(raw94.postscript_name(), raw95.postscript_name());
        assert!(
            super::NativeFontKey::new(&raw94.native_font())
                != super::NativeFontKey::new(&raw95.native_font())
        );
        // 独立直接原件 renderer：只插入 raw94，不经过 add_fonts、shape 或
        // id_for_native_font/native identity cache；避免对照命中同一个 FontId。
        let raw_renderer = MacTextSystem::new();
        raw_renderer
            .0
            .write()
            .fonts
            .push(super::FontResource::Native(raw94.clone()));
        let mut fallback_id = None;
        for fallback in [None, Some("MiSans VF"), Some("MiSansVF")] {
            for features in [FontFeatures::default(), FontFeatures::disable_ligatures()] {
                let mut face = font("MiSans VF");
                face.fallbacks =
                    fallback.map(|family| FontFallbacks::from_fonts(vec![family.into()]));
                face.features = features;
                let primary = fonts.font_id(&face).unwrap();
                assert_eq!(fonts.glyph_for_char(primary, 'm'), Some(GlyphId(59)));
                let text = "m|多个关用分";
                let layout = fonts.layout_line(
                    text,
                    px(14.),
                    &[FontRun {
                        font_id: primary,
                        len: text.len(),
                    }],
                );
                for run in layout.runs {
                    for glyph in run.glyphs {
                        let ch = text[glyph.index..].chars().next().unwrap();
                        if ch == 'm' {
                            continue;
                        }
                        let expected = raw94.glyph_for_char(ch).unwrap();
                        assert_eq!(glyph.id, GlyphId(expected), "original94 glyph for {ch}");
                        assert_ne!(run.font_id, primary);
                        assert_eq!(
                            fonts.advance(run.font_id, glyph.id).unwrap(),
                            super::size_from_vector2f(raw94.advance(expected).unwrap())
                        );
                        assert_eq!(
                            fonts.typographic_bounds(run.font_id, glyph.id).unwrap(),
                            super::bounds_from_rect(raw94.typographic_bounds(expected).unwrap())
                        );
                        let params = RenderGlyphParams {
                            font_id: run.font_id,
                            glyph_id: glyph.id,
                            font_size: px(14.),
                            subpixel_variant: point(0, 0),
                            scale_factor: 2.,
                            is_emoji: false,
                            subpixel_rendering: false,
                            dilation: 0,
                        };
                        let mut direct_params = params.clone();
                        direct_params.font_id = gpui::FontId(0);
                        direct_params.glyph_id = GlyphId(expected);
                        let bounds = fonts.glyph_raster_bounds(&params).unwrap();
                        assert_eq!(
                            bounds,
                            raw_renderer.glyph_raster_bounds(&direct_params).unwrap()
                        );
                        let rendered = fonts.rasterize_glyph(&params, bounds).unwrap();
                        assert!(rendered.1.iter().any(|byte| *byte != 0));
                        assert_eq!(
                            rendered,
                            raw_renderer
                                .rasterize_glyph(&direct_params, bounds)
                                .unwrap()
                        );
                        fallback_id = Some(run.font_id);
                    }
                }
            }
        }
        assert!(fallback_id.is_some());
        assert!(raw_renderer.0.read().font_ids_by_native_font.is_empty());
        // 相同字体/字号的重复 shape 必须稳定缓存；不能每行追加新的 FontId。
        let face = font("MiSans VF");
        let primary = fonts.font_id(&face).unwrap();
        let runs = [FontRun {
            font_id: primary,
            len: "多个".len(),
        }];
        let first = fonts.layout_line("多个", px(14.), &runs).runs[0].font_id;
        let second = fonts.layout_line("多个", px(20.), &runs).runs[0].font_id;
        assert_eq!(first, second);
    }

    // 保护公共 fallback list 语义：完整名称 A 必须在内存子集 B 之前；
    // 逆序必须实际选择 B，而不是统一将所有内存资源放到名称列表最前。
    #[test]
    fn test_mixed_named_and_memory_fallback_order() {
        use gpui::FontFallbacks;
        use std::borrow::Cow;
        let fonts = MacTextSystem::new();
        fonts
            .add_fonts(vec![
                Cow::Borrowed(include_bytes!(
                    "../../../src/openbox/assets/fonts/MiSans-VF.94-gh6bDI8D.woff2"
                )),
                Cow::Borrowed(include_bytes!(
                    "../../../src/openbox/assets/fonts/MiSans-VF.95-B0-MMqvr.woff2"
                )),
            ])
            .unwrap();
        let named_id = fonts.font_id(&font("PingFang SC")).unwrap();
        let named_glyph = fonts.glyph_for_char(named_id, '多').unwrap();
        assert_ne!(named_glyph, GlyphId(61));
        for (families, expected) in [
            (vec!["PingFang SC".into(), "MiSans VF".into()], named_glyph),
            (vec!["MiSans VF".into(), "PingFang SC".into()], GlyphId(61)),
        ] {
            let mut face = font("Helvetica");
            face.fallbacks = Some(FontFallbacks::from_fonts(families));
            let primary = fonts.font_id(&face).unwrap();
            assert!(fonts.glyph_for_char(primary, 'm').is_some());
            let layout = fonts.layout_line(
                "多",
                px(14.),
                &[FontRun {
                    font_id: primary,
                    len: "多".len(),
                }],
            );
            assert_eq!(layout.runs[0].glyphs[0].id, expected);
        }
    }

    #[test]
    fn test_fallback_only_subset_never_becomes_primary() {
        use std::borrow::Cow;
        let fonts = MacTextSystem::new();
        fonts
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../../../src/openbox/assets/fonts/MiSans-VF.94-gh6bDI8D.woff2"
            ))])
            .unwrap();
        assert!(fonts.font_id(&font("MiSans VF")).is_err());
        let primary = fonts.font_id(&font("Helvetica")).unwrap();
        assert!(fonts.glyph_for_char(primary, 'm').is_some());
        // primary 已选择之后再加入同 family子集，也需刷新 cascade。
        let fonts = MacTextSystem::new();
        fonts
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../../../src/openbox/assets/fonts/MiSans-VF.95-B0-MMqvr.woff2"
            ))])
            .unwrap();
        fonts.font_id(&font("MiSans VF")).unwrap();
        fonts
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../../../src/openbox/assets/fonts/MiSans-VF.94-gh6bDI8D.woff2"
            ))])
            .unwrap();
        let primary = fonts.font_id(&font("MiSans VF")).unwrap();
        let layout = fonts.layout_line(
            "多",
            px(14.),
            &[FontRun {
                font_id: primary,
                len: "多".len(),
            }],
        );
        assert_eq!(layout.runs[0].glyphs[0].id, GlyphId(61));
    }

    #[test]
    fn test_layout_line_bom_char() {
        let fonts = MacTextSystem::new();
        let font_id = fonts.font_id(&font("Helvetica")).unwrap();
        let line = "\u{feff}";
        let mut style = FontRun {
            font_id,
            len: line.len(),
        };

        let layout = fonts.layout_line(line, px(16.), &[style]);
        assert_eq!(layout.len, line.len());
        assert!(layout.runs.is_empty());

        let line = "a\u{feff}b";
        style.len = line.len();
        let layout = fonts.layout_line(line, px(16.), &[style]);
        assert_eq!(layout.len, line.len());
        assert_eq!(layout.runs.len(), 1);
        assert_eq!(layout.runs[0].glyphs.len(), 2);
        assert_eq!(layout.runs[0].glyphs[0].id, GlyphId(68u32)); // a
        // There's no glyph for \u{feff}
        assert_eq!(layout.runs[0].glyphs[1].id, GlyphId(69u32)); // b

        let line = "\u{feff}ab";
        let font_runs = &[
            FontRun {
                len: "\u{feff}".len(),
                font_id,
            },
            FontRun {
                len: "ab".len(),
                font_id,
            },
        ];
        let layout = fonts.layout_line(line, px(16.), font_runs);
        assert_eq!(layout.len, line.len());
        assert_eq!(layout.runs.len(), 1);
        assert_eq!(layout.runs[0].glyphs.len(), 2);
        // There's no glyph for \u{feff}
        assert_eq!(layout.runs[0].glyphs[0].id, GlyphId(68u32)); // a
        assert_eq!(layout.runs[0].glyphs[1].id, GlyphId(69u32)); // b
    }

    #[test]
    fn test_layout_line_zwnj_insertion() {
        let fonts = MacTextSystem::new();
        let font_id = fonts.font_id(&font("Helvetica")).unwrap();

        let text = "hello world";
        let font_runs = &[
            FontRun { font_id, len: 5 }, // "hello"
            FontRun { font_id, len: 6 }, // " world"
        ];

        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, text.len());

        for run in &layout.runs {
            for glyph in &run.glyphs {
                assert!(
                    glyph.index < text.len(),
                    "Glyph index {} is out of bounds for text length {}",
                    glyph.index,
                    text.len()
                );
            }
        }

        // Test with different font runs - should not insert ZWNJ
        let font_id2 = fonts.font_id(&font("Times")).unwrap_or(font_id);
        let font_runs_different = &[
            FontRun { font_id, len: 5 }, // "hello"
            // " world"
            FontRun {
                font_id: font_id2,
                len: 6,
            },
        ];

        let layout2 = fonts.layout_line(text, px(16.), font_runs_different);
        assert_eq!(layout2.len, text.len());

        for run in &layout2.runs {
            for glyph in &run.glyphs {
                assert!(
                    glyph.index < text.len(),
                    "Glyph index {} is out of bounds for text length {}",
                    glyph.index,
                    text.len()
                );
            }
        }
    }

    #[test]
    fn test_layout_line_zwnj_edge_cases() {
        let fonts = MacTextSystem::new();
        let font_id = fonts.font_id(&font("Helvetica")).unwrap();

        let text = "hello";
        let font_runs = &[FontRun { font_id, len: 5 }];
        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, text.len());

        let text = "abc";
        let font_runs = &[
            FontRun { font_id, len: 1 }, // "a"
            FontRun { font_id, len: 1 }, // "b"
            FontRun { font_id, len: 1 }, // "c"
        ];
        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, text.len());

        for run in &layout.runs {
            for glyph in &run.glyphs {
                assert!(
                    glyph.index < text.len(),
                    "Glyph index {} is out of bounds for text length {}",
                    glyph.index,
                    text.len()
                );
            }
        }

        // Test with empty text
        let text = "";
        let font_runs = &[];
        let layout = fonts.layout_line(text, px(16.), font_runs);
        assert_eq!(layout.len, 0);
        assert!(layout.runs.is_empty());
    }
}
