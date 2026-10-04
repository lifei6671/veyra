//! Owned local assets; root is injected until P1-05 establishes platform directories.
use image::{ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};
use veyra_core::domain::{AppError, AppErrorCode, DesktopBackground, VisualAssetId};
const MAX_BYTES: u64 = 8 * 1024 * 1024;
#[derive(Clone)]
pub struct VisualAssetStore {
    root: PathBuf,
}
fn failure() -> AppError {
    AppError::new(AppErrorCode::StorageFailed)
}
impl VisualAssetStore {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn path(&self, id: &VisualAssetId) -> PathBuf {
        self.root.join(format!("{}.png", id.0))
    }
    /// Only called for one explicitly provided file, never searches input directories.
    pub fn import(&self, source: &Path) -> Result<VisualAssetId, AppError> {
        let mut bytes = Vec::new();
        File::open(source)
            .map_err(|_| failure())?
            .take(MAX_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| failure())?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(AppError::new(AppErrorCode::Validation));
        }
        let format =
            image::guess_format(&bytes).map_err(|_| AppError::new(AppErrorCode::Validation))?;
        if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
            return Err(AppError::new(AppErrorCode::Validation));
        }
        let mut reader = ImageReader::with_format(Cursor::new(&bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        let image = reader
            .decode()
            .map_err(|_| AppError::new(AppErrorCode::Validation))?;
        let mut encoded = Cursor::new(Vec::new());
        image
            .write_to(&mut encoded, ImageFormat::Png)
            .map_err(|_| failure())?;
        let bytes = encoded.into_inner();
        let id = VisualAssetId(format!("{:x}", Sha256::digest(&bytes)));
        fs::create_dir_all(&self.root).map_err(|_| failure())?;
        let path = self.path(&id);
        if path.exists() {
            return Ok(id);
        }
        let temporary = path.with_extension("tmp");
        let result = (|| {
            let mut file = File::create(&temporary).map_err(|_| failure())?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| failure())?;
            drop(file);
            fs::rename(&temporary, &path).map_err(|_| failure())?;
            File::open(&self.root)
                .and_then(|d| d.sync_all())
                .map_err(|_| failure())?;
            Ok(id)
        })();
        if result.is_err() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
    /// Cached three-box approximation to image Gaussian blur; never CSS card backdrop blur.
    pub fn preview(&self, id: &VisualAssetId, blur: u8) -> Result<PathBuf, AppError> {
        if blur == 0 {
            return Ok(self.path(id));
        }
        let directory = self.root.join("cache").join(&id.0);
        let path = directory.join(format!("fast-{blur}.png"));
        if path.exists() {
            return Ok(path);
        }
        let started = std::time::Instant::now();
        let image = image::open(self.path(id))
            .map_err(|_| failure())?
            .fast_blur(blur as f32);
        fs::create_dir_all(&directory).map_err(|_| failure())?;
        let temporary = path.with_extension("tmp");
        let mut file = File::create(&temporary).map_err(|_| failure())?;
        image
            .write_to(&mut file, ImageFormat::Png)
            .map_err(|_| failure())?;
        file.sync_all().map_err(|_| failure())?;
        drop(file);
        fs::rename(&temporary, &path).map_err(|_| failure())?;
        eprintln!(
            "background image blur={blur} cache prepared duration_ms={}",
            started.elapsed().as_millis()
        );
        Ok(path)
    }
    /// Explicit orphan collection inside our managed root, never input directories.
    pub fn collect_orphans(&self, refs: &[DesktopBackground]) -> Result<(), AppError> {
        if !self.root.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&self.root).map_err(|_| failure())? {
            let entry = entry.map_err(|_| failure())?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "png") {
                let id = VisualAssetId(
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                );
                if id.is_valid() {
                    self.release_unreferenced(&id, refs)?;
                }
            }
        }
        Ok(())
    }
    /// Keep assets referenced by the authoritative snapshot AND its recovery backup.
    pub fn release_unreferenced(
        &self,
        id: &VisualAssetId,
        refs: &[DesktopBackground],
    ) -> Result<(), AppError> {
        if refs
            .iter()
            .any(|r| matches!(r,DesktopBackground::ManagedAsset(v) if v==id))
        {
            return Ok(());
        }
        let path = self.path(id);
        if path.exists() {
            fs::remove_file(path).map_err(|_| failure())?;
            let cache = self.root.join("cache").join(&id.0);
            if cache.exists() {
                fs::remove_dir_all(cache).map_err(|_| failure())?;
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "veyra-visual-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ))
    }
    #[test]
    fn valid_import_atomic_identity_type_and_size_boundary() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .save(&source)
            .unwrap();
        let assets = VisualAssetStore::new(root.join("assets"));
        let id = assets.import(&source).unwrap();
        assert!(id.is_valid());
        assert!(assets.path(&id).exists());
        assert!(!assets.path(&id).with_extension("tmp").exists());
        assert_eq!(assets.import(&source).unwrap(), id);
        fs::write(&source, b"not an image").unwrap();
        assert_eq!(
            assets.import(&source).unwrap_err().code(),
            AppErrorCode::Validation
        );
        File::create(&source)
            .unwrap()
            .set_len(MAX_BYTES + 1)
            .unwrap();
        assert_eq!(
            assets.import(&source).unwrap_err().code(),
            AppErrorCode::Validation
        );
        let reference = DesktopBackground::ManagedAsset(id.clone());
        assets.release_unreferenced(&id, &[reference]).unwrap();
        assert!(assets.path(&id).exists());
        assets.release_unreferenced(&id, &[]).unwrap();
        assert!(!assets.path(&id).exists());
        fs::remove_dir_all(root).unwrap();
    }
}
