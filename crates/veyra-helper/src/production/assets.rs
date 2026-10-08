//! 固定资产身份。不使用环境变量、IPC path 或 prototype binary。
use super::{Error, transport};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use veyra_core::application::runtime_recovery::KERNEL_DIGEST;

pub(super) struct Assets {
    pub kernel: PathBuf,
    pub root: PathBuf,
    pub uid: u32,
    pub gid: u32,
    pub installation: Option<super::install::Installation>,
    #[cfg(test)]
    pub fixture: bool,
    #[cfg(test)]
    pub spawned: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
}
impl Assets {
    pub fn installed(uid: u32, gid: u32) -> Result<Self, Error> {
        if uid == 0 || gid == 0 {
            return Err(Error::Unauthorized);
        }
        verify_kernel(Path::new(transport::KERNEL))?;
        let installation =
            super::install::installed_identity(Path::new(transport::ROOT), uid, gid)?;
        Ok(Self {
            installation: Some(installation),
            kernel: transport::KERNEL.into(),
            root: Path::new(transport::ROOT)
                .join("runtime")
                .join(uid.to_string()),
            uid,
            gid,
            #[cfg(test)]
            fixture: false,
            #[cfg(test)]
            spawned: Default::default(),
        })
    }
    pub fn verify(&self) -> Result<(), Error> {
        #[cfg(test)]
        if self.fixture {
            let test_exe = std::env::current_exe().map_err(|_| Error::Unavailable)?;
            // 仅测试允许当前测试exe的逐字节副本，用于核验真正exec的新basename。
            let test_copy = self.kernel.file_name().and_then(|n| n.to_str())
                == Some(veyra_core::application::runtime_recovery::KERNEL_EXECUTABLE)
                && fs::read(&self.kernel).map_err(|_| Error::Unavailable)?
                    == fs::read(test_exe.clone()).map_err(|_| Error::Unavailable)?;
            return ((self.kernel == test_exe || test_copy)
                && self.uid == unsafe { libc::geteuid() })
            .then_some(())
            .ok_or(Error::Unauthorized);
        }
        let expected = self.installation.as_ref().ok_or(Error::HandoffRequired)?;
        if super::install::installed_identity(Path::new(transport::ROOT), self.uid, self.gid)?
            != *expected
        {
            return Err(Error::HandoffRequired);
        }
        verify_kernel(&self.kernel)
    }
    /// 预检绝不创建/chmod；安装目录不安全时在desktop停旧writer之前拒绝。
    pub fn preflight_root(&self) -> Result<(), Error> {
        #[cfg(test)]
        let fixture = self.fixture;
        #[cfg(not(test))]
        let fixture = false;
        let paths = if fixture {
            vec![self.root.as_path()]
        } else {
            vec![
                self.root.as_path(),
                self.root.parent().ok_or(Error::UnsafeEndpoint)?,
            ]
        };
        for path in paths {
            match fs::symlink_metadata(path) {
                Ok(m)
                    if !m.is_dir()
                        || m.uid() != unsafe { libc::geteuid() }
                        || m.mode() & 0o022 != 0 =>
                {
                    return Err(Error::UnsafeEndpoint);
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(Error::UnsafeEndpoint),
            }
        }
        let lease_root = if fixture {
            self.root.as_path()
        } else {
            Path::new(transport::ROOT)
        };
        // 不把旧owner拒绝降格成可盲重试的Busy；需要交接/恢复的语义保持不变。
        super::writer_lease::WriterLease::preflight(lease_root).map_err(|error| {
            if error == Error::Busy {
                Error::HandoffRequired
            } else {
                error
            }
        })
    }
    /// 只确认当前目录没有材料；不证明没有旧writer，不能单独授权首次启动。
    /// 不能只检查manifest而漏掉孤儿cache/owner记录。
    pub fn pristine_root(&self) -> Result<(), Error> {
        self.preflight_root()?;
        match fs::read_dir(&self.root) {
            Ok(mut entries) => match entries.next() {
                None => Ok(()),
                Some(Ok(_)) => Err(Error::HandoffRequired),
                Some(Err(_)) => Err(Error::UnsafeEndpoint),
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(_) => Err(Error::UnsafeEndpoint),
        }
    }
    pub fn prepare_root(&self) -> Result<(), Error> {
        let parent = self.root.parent().ok_or(Error::UnsafeEndpoint)?;
        #[cfg(test)]
        let fixture = self.fixture;
        #[cfg(not(test))]
        let fixture = false;
        if !fixture {
            private_directory(parent, 0o711)?;
        }
        private_directory(&self.root, 0o700)
    }
}
pub(super) fn private_directory(path: &Path, mode: u32) -> Result<(), Error> {
    use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
    match fs::symlink_metadata(path) {
        Ok(m) if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 => {
            return Err(Error::UnsafeEndpoint);
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            fs::DirBuilder::new()
                .mode(mode)
                .create(path)
                .map_err(|_| Error::Unavailable)?;
        }
        Err(_) => return Err(Error::UnsafeEndpoint),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|_| Error::Unavailable)
}
pub(super) fn protected_file(path: &Path, mode: u32, max: u64) -> Result<File, Error> {
    transport::protected_directories(path.parent().ok_or(Error::UnsafeEndpoint)?)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotInstalled
            } else {
                Error::UnsafeEndpoint
            }
        })?;
    let m = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file() || m.uid() != 0 || m.mode() & 0o7777 != mode || m.nlink() != 1 || m.len() > max
    {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(file)
}
fn verify_kernel(path: &Path) -> Result<(), Error> {
    if path.file_name().and_then(|n| n.to_str())
        != Some(veyra_core::application::runtime_recovery::KERNEL_EXECUTABLE)
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut file = protected_file(path, 0o755, 256 * 1024 * 1024)?;
    let mut digest = Sha256::new();
    let mut chunk = [0; 65536];
    loop {
        let n = file.read(&mut chunk).map_err(|_| Error::Unavailable)?;
        if n == 0 {
            break;
        }
        digest.update(&chunk[..n]);
    }
    if format!("{:x}", digest.finalize()) != KERNEL_DIGEST {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(())
}
