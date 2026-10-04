//! Owned local assets under the injected preview Application Support/assets directory.
use image::{ImageFormat, ImageReader};
use sha2::{Digest, Sha256};
#[cfg(test)]
use std::fs::File;
use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
};
use veyra_core::domain::{AppError, AppErrorCode, DesktopBackground, VisualAssetId};
#[cfg(test)]
const MAX_BYTES: u64 = crate::platform::files::MAX_IMAGE_BYTES;
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
        let bytes = crate::platform::files::FileSelection::open(source)
            .and_then(|file| file.image_bytes())
            .map_err(|error| {
                eprintln!("file selection rejected error={error:?}; selected path redacted");
                match error {
                    crate::platform::PlatformError::InvalidFile
                    | crate::platform::PlatformError::FileTooLarge => {
                        AppError::new(AppErrorCode::Validation)
                    }
                    _ => failure(),
                }
            })?;
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
        crate::platform::files::private_directory(&self.root).map_err(|_| failure())?;
        let path = self.path(&id);
        if path.exists() {
            return Ok(id);
        }
        crate::platform::files::save_selected(&path, &bytes).map_err(|_| failure())?;
        if fs::read(&path).map_err(|_| failure())? != bytes {
            return Err(failure());
        }
        Ok(id)
    }
    /// Cached three-box approximation to image Gaussian blur; never CSS card backdrop blur.
    pub fn preview(&self, id: &VisualAssetId, blur: u8) -> Result<PathBuf, AppError> {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        if blur == 0 {
            return Ok(self.path(id));
        }
        let directory = self.root.join("cache").join(&id.0);
        let path = directory.join(format!("fast-{blur}.png"));
        if path.exists() {
            // 旧缓存复用前也必须收敛到私有权限。
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).map_err(|_| failure())?;
            return Ok(path);
        }
        let started = std::time::Instant::now();
        let image = image::open(self.path(id))
            .map_err(|_| failure())?
            .fast_blur(blur as f32);
        crate::platform::files::private_directory(&directory).map_err(|_| failure())?;
        let temporary = path.with_extension("tmp");
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| failure())?;
        // mode 仅影响新建文件；已存在的 stale temp 必须通过已打开 FD 修正。
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| failure())?;
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
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    fn root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "veyra-visual-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ))
    }
    // 保护旧写入遗留 temp 时，最终背景缓存仍为私有文件的契约。
    #[test]
    fn preview_repairs_world_readable_stale_temporary_before_publish() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .save(&source)
            .unwrap();
        let assets = VisualAssetStore::new(root.join("assets"));
        let id = assets.import(&source).unwrap();
        let directory = assets.root.join("cache").join(&id.0);
        crate::platform::files::private_directory(&directory).unwrap();
        let temporary = directory.join("fast-8.tmp");
        fs::write(&temporary, b"stale").unwrap();
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(fs::metadata(&temporary).unwrap().mode() & 0o777, 0o644);

        let preview = assets.preview(&id, 8).unwrap();
        assert_eq!(fs::metadata(&preview).unwrap().mode() & 0o777, 0o600);
        assert!(image::open(&preview).is_ok());
        assert!(!temporary.exists());
        for path in [&assets.root, &directory] {
            assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o700);
        }
        fs::remove_dir_all(root).unwrap();
    }
    // 保护缓存复用时的私有权限，同时验证不重建或替换已有内容。
    #[test]
    fn preview_repairs_world_readable_final_cache_on_reuse() {
        let root = root();
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .save(&source)
            .unwrap();
        let assets = VisualAssetStore::new(root.join("assets"));
        let id = assets.import(&source).unwrap();
        let preview = assets.preview(&id, 8).unwrap();
        let bytes = fs::read(&preview).unwrap();
        let inode = fs::metadata(&preview).unwrap().ino();
        fs::set_permissions(&preview, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(fs::metadata(&preview).unwrap().mode() & 0o777, 0o644);
        fs::remove_file(assets.path(&id)).unwrap(); // 缓存复用不应依赖源资产仍存在。

        assert_eq!(assets.preview(&id, 8).unwrap(), preview);
        assert_eq!(fs::metadata(&preview).unwrap().mode() & 0o777, 0o600);
        assert_eq!(fs::metadata(&preview).unwrap().ino(), inode);
        assert_eq!(fs::read(&preview).unwrap(), bytes);
        assert_eq!(fs::metadata(&assets.root).unwrap().mode() & 0o777, 0o700);
        fs::remove_dir_all(root).unwrap();
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
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            fs::metadata(assets.path(&id)).unwrap().mode() & 0o777,
            0o600
        );
        assert_eq!(fs::metadata(&assets.root).unwrap().mode() & 0o777, 0o700);
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
