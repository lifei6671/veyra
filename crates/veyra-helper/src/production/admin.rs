//! 固定管理员卸载 barrier。请求文件不是业务用户凭证，也不携带PID/路径/命令。
//! drain永久封住本安装；失败保留请求和恢复资料，禁止用重启绕过。
use super::Error;
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};
const REQUEST: &[u8] = b"veyra-uninstall-v1\n";
const ACK: &[u8] = b"veyra-stopped-and-sealed-v1\n";

fn read(root: &Path, name: &str) -> Result<Option<Vec<u8>>, Error> {
    let file = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join(name))
    {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::UnsafeEndpoint),
    };
    let m = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.nlink() != 1
        || m.mode() & 0o7777 != 0o600
        || m.len() > 128
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut bytes = Vec::new();
    file.take(129)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    Ok(Some(bytes))
}
fn write(root: &Path, name: &str, bytes: &[u8]) -> Result<(), Error> {
    if let Some(old) = read(root, name)? {
        return if old == bytes {
            Ok(())
        } else {
            Err(Error::UnsafeEndpoint)
        };
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(root.join(name))
        .map_err(|_| Error::Unavailable)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| Error::Unavailable)?;
    File::open(root)
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unavailable)
}
pub(super) fn requested(root: &Path) -> Result<bool, Error> {
    match read(root, "uninstall-request")? {
        None => Ok(false),
        Some(bytes) if bytes == REQUEST => Ok(true),
        Some(_) => Err(Error::UnsafeEndpoint),
    }
}
pub(super) fn request(root: &Path) -> Result<(), Error> {
    write(root, "uninstall-request", REQUEST)
}
pub(super) fn acknowledge(root: &Path) -> Result<(), Error> {
    if !requested(root)? {
        return Err(Error::InvalidRequest);
    }
    write(root, "uninstall-ack", ACK)
}
pub(super) fn acknowledged(root: &Path) -> Result<bool, Error> {
    Ok(requested(root)? && read(root, "uninstall-ack")?.as_deref() == Some(ACK))
}
/// 服务从检查marker之前到所有worker结束持有同一inode。锁释放仅证明服务退出，
/// 不证明孤儿child死亡；安装器还必须持有本次安装的持久drain ACK。
pub(super) fn lock(root: &Path, name: &str) -> Result<File, Error> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join(name))
        .map_err(|_| Error::UnsafeEndpoint)?;
    let m = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.nlink() != 1
        || m.mode() & 0o7777 != 0o600
        || m.len() != 0
    {
        return Err(Error::UnsafeEndpoint);
    }
    file.try_lock().map_err(|_| Error::Busy)?;
    Ok(file)
}
pub(super) fn no_runtime(root: &Path) -> Result<(), Error> {
    match fs::symlink_metadata(root.join("runtime")) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(Error::HandoffRequired),
    }
}

// 仅安装器在固定bootout成功后写入；用于失败重试，不是child死亡证明。
pub(super) fn record_bootout(root: &Path) -> Result<(), Error> {
    if !acknowledged(root)? {
        return Err(Error::HandoffRequired);
    }
    write(root, "bootout-complete", b"veyra-bootout-v1\n")
}
pub(super) fn booted_out(root: &Path) -> Result<bool, Error> {
    Ok(read(root, "bootout-complete")?.as_deref() == Some(b"veyra-bootout-v1\n"))
}
