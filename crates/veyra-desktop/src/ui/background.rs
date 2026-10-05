//! 默认背景只保存当前窗口的派生缓存；偏好事实仍由原 visual coordinator 持有。
use gpui_kit::{Image, ImageFormat, Pixels, Size};
use std::sync::{Arc, OnceLock};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Key {
    width: u32,
    height: u32,
    blur: u8,
}
impl Key {
    pub fn new(size: Size<Pixels>, blur: u8) -> Self {
        Self {
            width: (f32::from(size.width) + 20.).ceil() as u32,
            height: (f32::from(size.height) + 20.).ceil() as u32,
            blur,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Request {
    pub generation: u64,
    key: Key,
}
#[derive(Default)]
pub struct DefaultBackground {
    desired: Option<Key>,
    generation: u64,
    in_flight: Option<Request>,
    ready: Option<(Key, Arc<Image>)>,
}
impl DefaultBackground {
    /// 快速变化合并为最新 key，同一窗口最多一个 CPU job；此方法不读图、不派生。
    pub fn request(&mut self, key: Key) -> Option<Request> {
        if self.desired != Some(key) {
            self.desired = Some(key);
            self.generation += 1;
        }
        if self.in_flight.is_some() || self.ready.as_ref().is_some_and(|(k, _)| *k == key) {
            return None;
        }
        let request = Request {
            generation: self.generation,
            key,
        };
        self.in_flight = Some(request);
        Some(request)
    }
    pub fn complete(&mut self, request: Request, bytes: Vec<u8>) -> bool {
        if self.in_flight != Some(request) {
            return false;
        }
        self.in_flight = None;
        if self.desired != Some(request.key) || self.generation != request.generation {
            return false;
        }
        self.ready = Some((
            request.key,
            Arc::new(Image::from_bytes(ImageFormat::Png, bytes)),
        ));
        true
    }
    /// Render 仅取已就绪的图。新图派生期间保留上一帧，避免清空背景闪烁。
    pub fn ready(&self) -> Option<Arc<Image>> {
        self.ready.as_ref().map(|(_, image)| image.clone())
    }
}
/// 仅由 AppServices.spawn_blocking 调用；decode/cover/blur/encode 均不进入 GPUI Render。
pub fn derive(request: Request) -> Vec<u8> {
    static SOURCE: OnceLock<image::DynamicImage> = OnceLock::new();
    let source = SOURCE.get_or_init(|| {
        image::load_from_memory(include_bytes!(
            "../../../../src/openbox/assets/panel-background.jpg"
        ))
        .expect("bundled React background")
    });
    let image = source
        .resize_to_fill(
            request.key.width,
            request.key.height,
            image::imageops::FilterType::Triangle,
        )
        .fast_blur(f32::from(request.key.blur));
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .expect("encode bundled background");
    bytes.into_inner()
}
#[cfg(test)]
mod tests {
    use super::*;
    fn key(width: u32, blur: u8) -> Key {
        Key {
            width,
            height: 740,
            blur,
        }
    }
    // 保护连续 resize/blur 时旧派生结果不会覆盖最新窗口背景。
    #[test]
    fn stale_size_and_blur_are_discarded_and_latest_request_is_coalesced() {
        let mut cache = DefaultBackground::default();
        let old = cache.request(key(1300, 10)).unwrap();
        assert!(cache.request(key(1400, 20)).is_none());
        assert!(cache.request(key(1500, 30)).is_none());
        assert!(!cache.complete(old, vec![]));
        assert!(cache.ready().is_none());
        let latest = cache.request(key(1500, 30)).unwrap();
        assert_eq!(latest.generation, 3);
        assert!(!cache.complete(old, vec![]));
        assert!(cache.complete(latest, vec![]));
        assert!(cache.ready().is_some());
    }
    // 保护重复 frame/相同偏好不重复提交昂贵派生。
    #[test]
    fn identical_in_flight_or_ready_key_does_not_derive_again() {
        let mut cache = DefaultBackground::default();
        let k = key(1300, 10);
        let request = cache.request(k).unwrap();
        assert!(cache.request(k).is_none());
        assert!(cache.complete(request, vec![]));
        assert!(cache.request(k).is_none());
        assert!(cache.request(key(1300, 11)).is_some());
    }
    // 职责门禁保护 UI 响应：Render 只能读取 ready，CPU 唯一入口由阻塞任务调用。
    #[test]
    fn render_reads_ready_cache_and_derivation_stays_in_blocking_service() {
        let render = include_str!("shell.rs");
        assert!(render.contains("self.default_background.ready()"));
        for forbidden in [
            "default_background(",
            "background::derive",
            "resize_to_fill",
            "fast_blur",
            "write_to",
        ] {
            assert!(
                !render.contains(forbidden),
                "Render CPU operation: {forbidden}"
            );
        }
        let service = include_str!("../services.rs");
        let job = service
            .split("pub fn default_background")
            .nth(1)
            .unwrap()
            .split("pub fn ")
            .next()
            .unwrap();
        assert!(job.contains("spawn_blocking"));
        assert!(job.contains("background::derive(request)"));
    }
}
