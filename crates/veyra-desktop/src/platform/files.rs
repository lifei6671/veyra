use super::PlatformError;
use std::{
    fs::{self, DirBuilder, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
};
pub const MAX_IMAGE_BYTES: u64 = 8 * 1024 * 1024;
pub fn private_directory(path: &Path) -> Result<(), PlatformError> {
    DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(|_| PlatformError::PermissionDenied)?;
    let m = fs::symlink_metadata(path).map_err(|_| PlatformError::PermissionDenied)?;
    // Reject an injected symlink or a directory owned by someone else before chmod.
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } {
        return Err(PlatformError::PermissionDenied);
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| PlatformError::PermissionDenied)
}
pub struct FileSelection {
    file: File,
}
impl FileSelection {
    /// Opens once with O_NOFOLLOW; validates the opened object, never closes/reopens the path.
    /// Parent directories are not scanned. Symlink/alias inputs are not imported.
    pub fn open(path: &Path) -> Result<Self, PlatformError> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
            .map_err(|_| PlatformError::InvalidFile)?;
        let m = file.metadata().map_err(|_| PlatformError::FileReadFailed)?;
        if !m.is_file() {
            return Err(PlatformError::InvalidFile);
        }
        if m.len() > MAX_IMAGE_BYTES {
            return Err(PlatformError::FileTooLarge);
        }
        Ok(Self { file })
    }
    pub fn image_bytes(mut self) -> Result<Vec<u8>, PlatformError> {
        let mut bytes = Vec::new();
        Read::by_ref(&mut self.file)
            .take(MAX_IMAGE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| PlatformError::FileReadFailed)?;
        if bytes.len() as u64 > MAX_IMAGE_BYTES {
            return Err(PlatformError::FileTooLarge);
        }
        match image::guess_format(&bytes) {
            Ok(image::ImageFormat::Png | image::ImageFormat::Jpeg) => Ok(bytes),
            _ => Err(PlatformError::InvalidFile),
        }
    }
}
/// User explicitly accepted NSSavePanel (including Replace if destination existed).
/// Atomically publishes bytes; failures before rename preserve an existing target.
pub fn save_selected(destination: &Path, bytes: &[u8]) -> Result<(), PlatformError> {
    let parent = destination.parent().ok_or(PlatformError::FileWriteFailed)?;
    let name = destination
        .file_name()
        .ok_or(PlatformError::FileWriteFailed)?
        .to_string_lossy();
    let temporary = parent.join(format!(
        ".{name}.veyra-{}-{:?}.tmp",
        std::process::id(),
        veyra_core::domain::StateEpoch::fresh().map_err(|_| PlatformError::FileWriteFailed)?
    ));
    let result = (|| {
        let mut f = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|_| PlatformError::FileWriteFailed)?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|_| PlatformError::FileWriteFailed)?;
        drop(f);
        fs::rename(&temporary, destination).map_err(|_| PlatformError::FileWriteFailed)?;
        File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| PlatformError::FileWriteFailed)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
pub const EXPORT_FIXTURE: &[u8] = b"Veyra P1-05 file bridge evidence\n";
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_file_boundary_and_atomic_save_preserve_source_and_existing_target() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p105-files-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        private_directory(&root).unwrap();
        assert_eq!(fs::metadata(&root).unwrap().mode() & 0o777, 0o700);
        let src = root.join("source.png");
        image::RgbaImage::new(2, 2).save(&src).unwrap();
        let bytes = fs::read(&src).unwrap();
        assert_eq!(
            FileSelection::open(&src).unwrap().image_bytes().unwrap(),
            bytes
        );
        let alias = root.join("link");
        std::os::unix::fs::symlink(&src, &alias).unwrap();
        assert!(matches!(
            FileSelection::open(&alias),
            Err(PlatformError::InvalidFile)
        ));
        fs::write(&src, b"invalid").unwrap();
        assert_eq!(
            FileSelection::open(&src)
                .unwrap()
                .image_bytes()
                .unwrap_err(),
            PlatformError::InvalidFile
        );
        File::create(&src)
            .unwrap()
            .set_len(MAX_IMAGE_BYTES + 1)
            .unwrap();
        assert!(matches!(
            FileSelection::open(&src),
            Err(PlatformError::FileTooLarge)
        ));
        let target = root.join("export.txt");
        save_selected(&target, EXPORT_FIXTURE).unwrap();
        assert_eq!(fs::read(&target).unwrap(), EXPORT_FIXTURE);
        assert_eq!(fs::metadata(&target).unwrap().mode() & 0o777, 0o600);
        assert_eq!(
            save_selected(&root.join("invalid-parent/export"), b"x"),
            Err(PlatformError::FileWriteFailed)
        );
        assert_eq!(fs::read(&target).unwrap(), EXPORT_FIXTURE);
        fs::create_dir(root.join("collision")).unwrap();
        assert_eq!(
            save_selected(&root.join("collision"), b"x"),
            Err(PlatformError::FileWriteFailed)
        );
        assert!(
            !fs::read_dir(&root).unwrap().any(|e| e
                .unwrap()
                .path()
                .extension()
                .is_some_and(|s| s == "tmp"))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
