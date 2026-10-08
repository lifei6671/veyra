//! 卸载后的固定Recovery Archive：仅封存已经ACK且bootout成功的安装，不恢复旧状态。
use super::{Error, admin};
use std::{
    ffi::CString,
    fs::{self, File, OpenOptions},
    io::Read,
    os::{
        fd::AsRawFd,
        unix::{
            ffi::OsStrExt,
            fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
        },
    },
    path::{Path, PathBuf},
};
const DIRECTORY: &str = "RecoveryArchive";
fn directory(path: &Path, private: bool) -> Result<std::fs::Metadata, Error> {
    let m = fs::symlink_metadata(path).map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_dir()
        || m.uid() != unsafe { libc::geteuid() }
        || (private && m.mode() & 0o7777 != 0o700)
        || m.mode() & 0o022 != 0
    {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(m)
}
fn sync(path: &Path) -> Result<(), Error> {
    File::open(path)
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unavailable)
}
fn name(m: &std::fs::Metadata) -> String {
    format!("installation-{:x}-{:x}", m.dev(), m.ino())
}
fn archive(parent: &Path) -> Result<PathBuf, Error> {
    directory(parent, false)?;
    let path = parent.join(DIRECTORY);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&path)
                .map_err(|_| Error::Unavailable)?;
            sync(parent)?;
        }
        Err(_) => return Err(Error::UnsafeEndpoint),
        Ok(_) => {}
    }
    directory(&path, true)?;
    Ok(path)
}
/// 新安装只验证封存证据，不读业务config/cache、更不把Archive当候选状态。
pub(super) fn validate_archives(parent: &Path) -> Result<(), Error> {
    directory(parent, false)?;
    let path = parent.join(DIRECTORY);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(Error::UnsafeEndpoint),
        Ok(_) => {}
    }
    directory(&path, true)?;
    for entry in fs::read_dir(path).map_err(|_| Error::Unavailable)? {
        let entry = entry.map_err(|_| Error::Unavailable)?;
        let m = directory(&entry.path(), true)?;
        if entry.file_name() != std::ffi::OsStr::new(&name(&m))
            || !admin::acknowledged(&entry.path())?
            || !admin::booted_out(&entry.path())?
        {
            return Err(Error::HandoffRequired);
        }
    }
    Ok(())
}
/// rename已经成功、响应或父目录fsync失败后的重试；只补目录持久化，不恢复/删除资料。
pub(super) fn confirm_committed(parent: &Path) -> Result<(), Error> {
    validate_archives(parent)?;
    match fs::symlink_metadata(parent.join(DIRECTORY)) {
        Ok(_) => sync(&parent.join(DIRECTORY))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Error::UnsafeEndpoint),
    }
    sync(parent)
}
fn exclusive_rename(from: &Path, to: &Path) -> Result<(), Error> {
    let from = CString::new(from.as_os_str().as_bytes()).map_err(|_| Error::UnsafeEndpoint)?;
    let to = CString::new(to.as_os_str().as_bytes()).map_err(|_| Error::UnsafeEndpoint)?;
    if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) } != 0 {
        return Err(match std::io::Error::last_os_error().raw_os_error() {
            Some(libc::EEXIST) => Error::Busy,
            _ => Error::Unavailable,
        });
    }
    Ok(())
}
fn fixed_plist(path: &Path, expected: &[u8]) -> Result<(), Error> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::UnsafeEndpoint)?;
    let m = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.nlink() != 1
        || m.mode() & 0o7777 != 0o644
        || m.len() > 16384
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut bytes = Vec::new();
    file.take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes != expected {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(())
}
pub(super) fn validate_plist(root: &Path, plist: &Path, expected: &[u8]) -> Result<(), Error> {
    match fs::symlink_metadata(plist) {
        Ok(_) => fixed_plist(plist, expected),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && admin::booted_out(root)? => {
            fixed_plist(&root.join("launchd.plist"), expected)
        }
        _ => Err(Error::UnsafeEndpoint),
    }
}
/// 调用方持有root外deployment锁；此处检查ACK后自己取得生命周期锁，
/// 原子rename期间不释放它。不存在“检查锁后放开再迁移”的窗口。
pub(super) fn finish(root: &Path, plist: &Path, expected: &[u8]) -> Result<PathBuf, Error> {
    let parent = root.parent().ok_or(Error::UnsafeEndpoint)?;
    let m = directory(root, false)?;
    if !admin::acknowledged(root)? || !admin::booted_out(root)? {
        return Err(Error::HandoffRequired);
    }
    let _offline = admin::lock(root, "lifecycle-lock")?;
    // 服务退出/旧ACK都不能覆盖仍由孤儿child持有的writer租约；失败保留原root。
    let _writer = super::writer_lease::WriterLease::acquire(root)?;
    let destination = archive(parent)?;
    if directory(parent, false)?.dev() != m.dev() || directory(&destination, true)?.dev() != m.dev()
    {
        return Err(Error::UnsafeEndpoint);
    }
    let target = destination.join(name(&m));
    // 冲突在移动plist前拒绝；RENAME_EXCL仍负责原子不覆盖。
    match fs::symlink_metadata(&target) {
        Ok(_) => return Err(Error::Busy),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Error::UnsafeEndpoint),
    }
    validate_plist(root, plist, expected)?;
    if fs::symlink_metadata(plist).is_ok() {
        exclusive_rename(plist, &root.join("launchd.plist"))?;
        sync(plist.parent().ok_or(Error::UnsafeEndpoint)?)?;
        sync(root)?;
    }
    // 目录FD绑定原inode，不通过可替换路径chmod。已无服务/writer，资料原封不动。
    let fd = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(root)
        .map_err(|_| Error::UnsafeEndpoint)?;
    if fd.metadata().map_err(|_| Error::UnsafeEndpoint)?.ino() != m.ino() {
        return Err(Error::UnsafeEndpoint);
    }
    if unsafe { libc::fchmod(fd.as_raw_fd(), 0o700) } != 0 {
        return Err(Error::Unavailable);
    }
    fd.sync_all().map_err(|_| Error::Unavailable)?;
    exclusive_rename(root, &target)?;
    sync(&destination)?;
    sync(parent)?;
    Ok(target)
}

/// bootout错误不记录成功；重试只认可同一root里受保护的成功记录。
pub(super) fn complete_bootout(
    root: &Path,
    action: impl FnOnce() -> Result<(), Error>,
) -> Result<(), Error> {
    if !admin::acknowledged(root)? {
        return Err(Error::HandoffRequired);
    }
    if !admin::booted_out(root)? {
        action()?;
        admin::record_bootout(root)?;
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        os::unix::fs::PermissionsExt,
        time::{Duration, Instant},
    };
    const PLIST: &[u8] = b"fixed-fixture-plist";
    struct Fixture {
        parent: PathBuf,
        root: PathBuf,
        plist: PathBuf,
    }
    impl Fixture {
        fn new() -> Self {
            let parent = std::env::temp_dir().join(format!(
                "v206-archive-{:?}",
                veyra_core::domain::StateEpoch::fresh().unwrap()
            ));
            fs::DirBuilder::new().mode(0o700).create(&parent).unwrap();
            let f = Self {
                root: parent.join("Helper"),
                plist: parent.join("fixed.plist"),
                parent,
            };
            f.install();
            f
        }
        fn install(&self) {
            let _deployment = admin::lock(&self.parent, "deployment-lock").unwrap();
            validate_archives(&self.parent).unwrap();
            assert!(!self.root.exists());
            assert!(!self.plist.exists());
            fs::DirBuilder::new()
                .mode(0o755)
                .create(&self.root)
                .unwrap();
            fs::DirBuilder::new()
                .mode(0o700)
                .create(self.root.join("runtime"))
                .unwrap();
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(self.root.join("runtime/evidence"))
                .unwrap();
            std::io::Write::write_all(&mut file, b"closed-cache-manifest-owner-fixture").unwrap();
            fs::write(&self.plist, PLIST).unwrap();
            fs::set_permissions(&self.plist, fs::Permissions::from_mode(0o644)).unwrap();
        }
        fn ack(&self) {
            admin::request(&self.root).unwrap();
            admin::acknowledge(&self.root).unwrap();
        }
        fn bootout(&self) {
            self.ack();
            complete_bootout(&self.root, || Ok(())).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.parent).unwrap();
        }
    }
    #[test]
    fn two_install_uninstall_cycles_archive_without_consuming_old_material() {
        // 保护成功卸载之后可以新安装；模拟bootout，文件rename/fsync/locks是真OS。
        let f = Fixture::new();
        let mut archived = Vec::new();
        for _ in 0..2 {
            f.bootout();
            let _deployment = admin::lock(&f.parent, "deployment-lock").unwrap();
            let target = finish(&f.root, &f.plist, PLIST).unwrap();
            assert!(!f.root.exists());
            assert!(!f.plist.exists());
            assert_eq!(
                fs::read(target.join("runtime/evidence")).unwrap(),
                b"closed-cache-manifest-owner-fixture"
            );
            assert_eq!(fs::metadata(&target).unwrap().mode() & 0o7777, 0o700);
            confirm_committed(&f.parent).unwrap();
            archived.push(target);
            drop(_deployment);
            f.install();
            assert!(!f.root.join("uninstall-request").exists());
        }
        assert_ne!(archived[0], archived[1]);
        for path in archived {
            assert!(path.join("runtime/evidence").exists());
        }
    }
    #[test]
    fn missing_ack_bootout_failure_and_live_lock_preserve_original_root() {
        // 保护拒绝/失败和仍活服务：没有成功证据绝不移动现场或释放安装ROOT。
        let f = Fixture::new();
        assert_eq!(
            complete_bootout(&f.root, || panic!("must not bootout")),
            Err(Error::HandoffRequired)
        );
        assert_eq!(
            finish(&f.root, &f.plist, PLIST),
            Err(Error::HandoffRequired)
        );
        f.ack();
        assert_eq!(
            complete_bootout(&f.root, || Err(Error::Failed)),
            Err(Error::Failed)
        );
        assert!(!admin::booted_out(&f.root).unwrap());
        assert!(f.plist.exists());
        f.bootout();
        let live = admin::lock(&f.root, "lifecycle-lock").unwrap();
        assert_eq!(finish(&f.root, &f.plist, PLIST), Err(Error::Busy));
        drop(live);
        fs::remove_file(f.root.join("uninstall-ack")).unwrap();
        assert_eq!(
            finish(&f.root, &f.plist, PLIST),
            Err(Error::HandoffRequired)
        );
        assert!(f.root.join("runtime/evidence").exists());
        assert!(f.plist.exists());
    }
    #[test]
    fn unsafe_archive_and_duplicate_target_never_overwrite_or_disclose() {
        // 保护archive父目录权限/符号链接，及已存在同名记录不被覆盖。
        let f = Fixture::new();
        f.bootout();
        let directory = f.parent.join(DIRECTORY);
        std::os::unix::fs::symlink(&f.root, &directory).unwrap();
        assert_eq!(finish(&f.root, &f.plist, PLIST), Err(Error::UnsafeEndpoint));
        fs::remove_file(&directory).unwrap();
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o777)).unwrap();
        assert_eq!(finish(&f.root, &f.plist, PLIST), Err(Error::UnsafeEndpoint));
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let collision = directory.join(name(&fs::metadata(&f.root).unwrap()));
        fs::create_dir(&collision).unwrap();
        fs::write(collision.join("keep"), b"existing").unwrap();
        assert_eq!(finish(&f.root, &f.plist, PLIST), Err(Error::Busy));
        assert_eq!(fs::read(collision.join("keep")).unwrap(), b"existing");
        assert!(f.plist.exists());
        assert!(validate_archives(&f.parent).is_err()); // 不明归属不能被新安装绕过。
    }
    #[test]
    fn plist_rename_failure_and_partial_uninstall_retry_preserve_evidence() {
        // 真正RENAME_EXCL失败不能先删资产；plist已移动的半提交可重试，不重复bootout。
        let f = Fixture::new();
        f.bootout();
        let parked = f.root.join("launchd.plist");
        fs::write(&parked, b"collision").unwrap();
        assert_eq!(finish(&f.root, &f.plist, PLIST), Err(Error::Busy));
        assert_eq!(fs::read(&parked).unwrap(), b"collision");
        assert!(f.plist.exists());
        fs::remove_file(&parked).unwrap();
        exclusive_rename(&f.plist, &parked).unwrap();
        complete_bootout(&f.root, || panic!("bootout already committed")).unwrap();
        let target = finish(&f.root, &f.plist, PLIST).unwrap();
        assert_eq!(fs::read(target.join("launchd.plist")).unwrap(), PLIST);
        assert!(target.join("runtime/evidence").exists());
    }
    #[test]
    fn killed_service_releases_lock_but_never_substitutes_for_ack() {
        // 真实SIGKILL仅作用于本测试Child；没有ACK时即使锁释放也保持原root。
        struct Child(std::process::Child);
        impl Drop for Child {
            fn drop(&mut self) {
                let _ = self.0.kill();
                self.0.wait().unwrap();
            }
        }
        let f = Fixture::new();
        admin::request(&f.root).unwrap();
        let mut child = Child(
            std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "production::process_tests::admin_crash_fixture",
                    "--ignored",
                ])
                .env("VEYRA_ADMIN_FIXTURE_ROOT", &f.root)
                .env("VEYRA_ADMIN_FIXTURE_ROLE", "service")
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(5);
        while !f.root.join("service-ready").exists() {
            assert!(Instant::now() < until);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(
            admin::lock(&f.root, "lifecycle-lock"),
            Err(Error::Busy)
        ));
        child.0.kill().unwrap();
        child.0.wait().unwrap();
        let _offline = admin::lock(&f.root, "lifecycle-lock").unwrap();
        assert_eq!(
            finish(&f.root, &f.plist, PLIST),
            Err(Error::HandoffRequired)
        );
        assert!(f.root.join("runtime/evidence").exists());
        assert!(f.plist.exists());
    }
}
