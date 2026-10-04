use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::StateStoreError;

pub fn backup_path(state_file: &Path) -> PathBuf {
    sibling_path(state_file, "bak")
}

pub fn pre_migration_backup_path(state_file: &Path) -> PathBuf {
    sibling_path(state_file, "pre-migration")
}

pub fn corrupt_copy_path(state_file: &Path) -> PathBuf {
    sibling_path(state_file, "corrupt")
}

pub fn read_snapshot(path: &Path) -> Result<Vec<u8>, StateStoreError> {
    fs::read(path).map_err(|_| StateStoreError::ReadFailed)
}

pub fn copy_for_backup(source: &Path, destination: &Path) -> Result<(), StateStoreError> {
    let bytes = read_snapshot(source)?;
    atomic_replace(destination, &bytes)
}

pub fn preserve_corrupt_copy(state_file: &Path) -> Result<(), StateStoreError> {
    let corrupt_copy = corrupt_copy_path(state_file);
    copy_for_backup(state_file, &corrupt_copy)
}

pub fn atomic_replace(destination: &Path, contents: &[u8]) -> Result<(), StateStoreError> {
    let parent = destination
        .parent()
        .ok_or(StateStoreError::InvalidStatePath)?;
    fs::create_dir_all(parent).map_err(|_| StateStoreError::WriteFailed)?;

    let temporary = destination.with_extension("tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&temporary)
        .map_err(|_| StateStoreError::WriteFailed)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| StateStoreError::WriteFailed)?;
    }
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|_| StateStoreError::WriteFailed)?;
    drop(file);

    // rename 成功即发布成功；不能在发布后再把保存报告为失败。
    fs::rename(&temporary, destination).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        StateStoreError::ReplaceFailed
    })
}

fn sibling_path(state_file: &Path, suffix: &str) -> PathBuf {
    let file_name = state_file.file_name().unwrap_or_default().to_string_lossy();
    state_file.with_file_name(format!("{file_name}.{suffix}"))
}

#[cfg(all(test, unix))]
mod permission_tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;
    #[test]
    fn state_and_backup_are_private_at_creation() {
        let root = std::env::temp_dir().join(format!(
            "veyra-p105-core-{:?}",
            crate::domain::StateEpoch::fresh().unwrap()
        ));
        let state = root.join("state.json");
        atomic_replace(&state, b"first").unwrap();
        copy_for_backup(&state, &backup_path(&state)).unwrap();
        // A stale temporary from an older writer must not publish world-readable state.
        let stale = state.with_extension("tmp");
        fs::write(&stale, b"stale").unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&stale, fs::Permissions::from_mode(0o644)).unwrap();
        atomic_replace(&state, b"second").unwrap();
        for path in [&state, &backup_path(&state)] {
            assert_eq!(fs::metadata(path).unwrap().mode() & 0o777, 0o600);
        }
        assert_eq!(fs::read(backup_path(&state)).unwrap(), b"first");
        assert_eq!(fs::read(&state).unwrap(), b"second");
        fs::remove_dir_all(root).unwrap();
    }
}
