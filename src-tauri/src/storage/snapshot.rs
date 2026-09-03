use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::StateStoreError;

pub(crate) fn backup_path(state_file: &Path) -> PathBuf {
    sibling_path(state_file, "bak")
}

pub(crate) fn pre_migration_backup_path(state_file: &Path) -> PathBuf {
    sibling_path(state_file, "pre-migration")
}

pub(crate) fn corrupt_copy_path(state_file: &Path) -> PathBuf {
    sibling_path(state_file, "corrupt")
}

pub(crate) fn read_snapshot(path: &Path) -> Result<Vec<u8>, StateStoreError> {
    fs::read(path).map_err(|_| StateStoreError::ReadFailed)
}

pub(crate) fn copy_for_backup(source: &Path, destination: &Path) -> Result<(), StateStoreError> {
    let bytes = read_snapshot(source)?;
    atomic_replace(destination, &bytes)
}

pub(crate) fn preserve_corrupt_copy(state_file: &Path) -> Result<(), StateStoreError> {
    let corrupt_copy = corrupt_copy_path(state_file);
    copy_for_backup(state_file, &corrupt_copy)
}

pub(crate) fn atomic_replace(destination: &Path, contents: &[u8]) -> Result<(), StateStoreError> {
    let parent = destination
        .parent()
        .ok_or(StateStoreError::InvalidStatePath)?;
    fs::create_dir_all(parent).map_err(|_| StateStoreError::WriteFailed)?;

    let temporary = destination.with_extension("tmp");
    let mut file = File::create(&temporary).map_err(|_| StateStoreError::WriteFailed)?;
    file.write_all(contents)
        .and_then(|_| file.sync_all())
        .map_err(|_| StateStoreError::WriteFailed)?;
    drop(file);

    fs::rename(&temporary, destination).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        StateStoreError::ReplaceFailed
    })
}

fn sibling_path(state_file: &Path, suffix: &str) -> PathBuf {
    let file_name = state_file.file_name().unwrap_or_default().to_string_lossy();
    state_file.with_file_name(format!("{file_name}.{suffix}"))
}
