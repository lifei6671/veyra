//! 同一安装的内核writer租约；父进程死亡不释放仍由child继承的同一open-file-description。
//! 锁只能证明本合同参与者互斥，不能替代持久owner/clean-stop或未知旧writer恢复证据。
use super::{Error, admin};
use std::{
    fs::{self, File, OpenOptions},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
};
pub(super) const NAME: &str = "runtime-writer-lock";
pub(super) struct WriterLease {
    file: File,
    path: PathBuf,
}
impl WriterLease {
    /// 交接预检只读既有路径；已占有时在Desktop冻结/Stop之前拒绝，不创建锁文件。
    pub fn preflight(root: &Path) -> Result<(), Error> {
        let path = root.join(NAME);
        let file = match OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(&path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => return Err(Error::UnsafeEndpoint),
        };
        let lease = Self { file, path };
        lease.verify()?;
        lease.file.try_lock().map_err(|_| Error::Busy)
    }
    pub fn acquire(root: &Path) -> Result<Self, Error> {
        let lease = Self {
            file: admin::lock(root, NAME)?,
            path: root.join(NAME),
        };
        lease.verify()?;
        Ok(lease)
    }
    pub fn verify(&self) -> Result<(), Error> {
        let fd = self.file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
        let path = fs::symlink_metadata(&self.path).map_err(|_| Error::UnsafeEndpoint)?;
        if !path.is_file()
            || path.uid() != unsafe { libc::geteuid() }
            || path.mode() & 0o7777 != 0o600
            || path.nlink() != 1
            || path.len() != 0
            || (fd.dev(), fd.ino()) != (path.dev(), path.ino())
        {
            return Err(Error::UnsafeEndpoint);
        }
        Ok(())
    }
    /// 同一open-file-description的FD副本，不是重新flock；原Prepared在Port接入前后都保留排他。
    pub(super) fn duplicate(&self) -> Result<Self, Error> {
        self.verify()?;
        Ok(Self {
            file: self.file.try_clone().map_err(|_| Error::Unavailable)?,
            path: self.path.clone(),
        })
    }
    pub fn child_fd(&self) -> Result<OwnedFd, Error> {
        self.verify()?;
        // parent提前复制到>=5，避免child映射配置FD3时覆盖租约原FD；不在fork后分配。
        let fd = unsafe { libc::fcntl(self.file.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 5) };
        if fd < 0 {
            return Err(Error::Unavailable);
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{BufRead, BufReader, Read, Write},
        os::unix::{fs::PermissionsExt, process::CommandExt},
        process::{Command, Stdio},
    };
    #[test]
    #[ignore = "only spawned by lease tests with inherited FD4"]
    fn inherited_writer_fixture() {
        // 不获取新的锁：必须验证继承的同一FD仍存在。无真实内核/网络。
        assert_eq!(unsafe { libc::fcntl(4, libc::F_GETFD) }, 0);
        let file = unsafe { File::from_raw_fd(4) };
        assert!(file.metadata().unwrap().is_file());
        println!("lease-ready");
        std::io::stdout().flush().unwrap();
        let mut byte = [0];
        std::io::stdin().read_exact(&mut byte).unwrap();
        assert_eq!(byte[0], 1);
        drop(file);
    }
    struct Child(std::process::Child);
    impl Drop for Child {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
    fn root() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "v206-lease-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        fs::create_dir(&p).unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        let root = p.join("Helper");
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        root
    }
    #[test]
    fn inherited_child_keeps_exclusion_after_parent_descriptor_loss() {
        // 保护helper父端丢失租约句柄后，仍活的自有child不能与新owner同时写。
        let root = root();
        assert_eq!(WriterLease::preflight(&root), Ok(()));
        assert!(
            !root.join(NAME).exists(),
            "preflight must not create a record"
        );
        let lease = WriterLease::acquire(&root).unwrap();
        assert_eq!(WriterLease::preflight(&root), Err(Error::Busy));
        let inherited = lease.child_fd().unwrap();
        let fd = inherited.as_raw_fd();
        let mut cmd = Command::new(std::env::current_exe().unwrap());
        cmd.args([
            "--exact",
            "production::writer_lease::tests::inherited_writer_fixture",
            "--ignored",
            "--nocapture",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
        unsafe {
            cmd.pre_exec(move || {
                if libc::dup2(fd, 4) < 0 || libc::fcntl(4, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut service = Child(cmd.spawn().unwrap());
        let mut child = Child(cmd.spawn().unwrap());
        drop(inherited);
        let mut reader = BufReader::new(child.0.stdout.take().unwrap());
        let mut line = String::new();
        loop {
            line.clear();
            assert_ne!(reader.read_line(&mut line).unwrap(), 0);
            if line.trim() == "lease-ready" {
                break;
            }
        }
        let mut service_reader = BufReader::new(service.0.stdout.take().unwrap());
        loop {
            line.clear();
            assert_ne!(service_reader.read_line(&mut line).unwrap(), 0);
            if line.trim() == "lease-ready" {
                break;
            }
        }
        drop(lease);
        // 两个测试Child共享同一open-file-description；服务持有者被SIGKILL后writer仍活。
        // 均由本测试持有Child句柄，不从payload取PID，也不影响既有业务。
        service.0.kill().unwrap();
        service.0.wait().unwrap();
        assert!(child.0.try_wait().unwrap().is_none());
        assert!(matches!(WriterLease::acquire(&root), Err(Error::Busy)));
        // 即使已有ACK与服务锁可得，Archive也不能移动活writer的root。
        super::super::admin::request(&root).unwrap();
        super::super::admin::acknowledge(&root).unwrap();
        super::super::archive::complete_bootout(&root, || Ok(())).unwrap();
        let plist = root.join("fixture.plist");
        fs::write(&plist, b"fixture").unwrap();
        fs::set_permissions(&plist, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            super::super::archive::finish(&root, &plist, b"fixture"),
            Err(Error::Busy)
        ));
        assert!(root.is_dir());
        assert_eq!(fs::read(&plist).unwrap(), b"fixture");
        child.0.stdin.take().unwrap().write_all(&[1]).unwrap();
        assert!(child.0.wait().unwrap().success());
        let next = WriterLease::acquire(&root).unwrap();
        next.verify().unwrap();
        drop(next);
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
    #[test]
    fn replaced_or_untrusted_lease_is_never_a_clean_owner_proof() {
        // 保护被替换/权限放宽的锁路径不能继续spawn；丢文件也不能解释为干净owner。
        let root = root();
        let lease = WriterLease::acquire(&root).unwrap();
        let path = root.join(NAME);
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        assert_eq!(lease.verify(), Err(Error::UnsafeEndpoint));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::rename(&path, root.join("old-lock")).unwrap();
        assert_eq!(lease.verify(), Err(Error::UnsafeEndpoint));
        let replacement = WriterLease::acquire(&root).unwrap();
        assert_eq!(lease.verify(), Err(Error::UnsafeEndpoint));
        drop(replacement);
        drop(lease);
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("old-lock", &path).unwrap();
        assert!(matches!(
            WriterLease::acquire(&root),
            Err(Error::UnsafeEndpoint)
        ));
        fs::remove_dir_all(root.parent().unwrap()).unwrap();
    }
}
