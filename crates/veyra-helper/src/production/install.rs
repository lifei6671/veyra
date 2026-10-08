//! 固定管理员入口。这里只提供代码；开发测试绝不调用 root/launchctl。
use super::{Error, PROTOCOL, assets::protected_file, transport};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const BUNDLE: &str = "/Applications/Veyra.app/Contents/Resources/helper";
const PLIST: &str = "/Library/LaunchDaemons/me.disign.veyra.helper.plist";
const LABEL: &str = "system/me.disign.veyra.helper";
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    protocol: u16,
    helper_sha256: String,
    kernel_sha256: String,
}
/// 管理员创建的安装世代绑定固定root inode与OS授权用户；不是桌面退出证明。
/// 旧安装缺此记录时拒绝自动升级，Archive原样保留它，不消费为新安装授权。
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Installation {
    protocol: u16,
    pub(super) generation: veyra_core::domain::StateEpoch,
    uid: u32,
    gid: u32,
    device: u64,
    inode: u64,
}
fn installation_root(root: &Path) -> Result<std::fs::Metadata, Error> {
    let m = fs::symlink_metadata(root).map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(m)
}
pub(super) fn record_installation(root: &Path, uid: u32, gid: u32) -> Result<(), Error> {
    if uid == 0 || gid == 0 {
        return Err(Error::Unauthorized);
    }
    let m = installation_root(root)?;
    let record = Installation {
        protocol: PROTOCOL,
        generation: veyra_core::domain::StateEpoch::fresh().map_err(|_| Error::Unavailable)?,
        uid,
        gid,
        device: m.dev(),
        inode: m.ino(),
    };
    create(
        &root.join("installation.json"),
        &serde_json::to_vec(&record).map_err(|_| Error::Unavailable)?,
        0o600,
    )
}
pub(super) fn installed_identity(root: &Path, uid: u32, gid: u32) -> Result<Installation, Error> {
    let m = installation_root(root)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join("installation.json"))
        .map_err(|_| Error::HandoffRequired)?;
    let fm = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !fm.is_file()
        || fm.uid() != unsafe { libc::geteuid() }
        || fm.nlink() != 1
        || fm.mode() & 0o7777 != 0o600
        || fm.len() > 4096
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut bytes = vec![];
    file.take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    let record: Installation = serde_json::from_slice(&bytes).map_err(|_| Error::UnsafeEndpoint)?;
    if record.protocol != PROTOCOL
        || record.uid != uid
        || record.gid != gid
        || uid == 0
        || gid == 0
        || record.device != m.dev()
        || record.inode != m.ino()
    {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(record)
}
/// 打包阶段生成manifest；只计算字节摘要，不安装/执行输入，不授予资源任何信任。
/// 安装时仍重新验证固定bundle的root owner、模式与锁定kernel SHA。
pub fn installation_manifest(helper: &[u8], kernel: &[u8]) -> Result<Vec<u8>, Error> {
    if helper.is_empty() || helper.len() > 256 * 1024 * 1024 || kernel.len() > 256 * 1024 * 1024 {
        return Err(Error::InvalidRequest);
    }
    let manifest = Manifest {
        protocol: PROTOCOL,
        helper_sha256: format!("{:x}", Sha256::digest(helper)),
        kernel_sha256: format!("{:x}", Sha256::digest(kernel)),
    };
    validate(&manifest, helper, kernel)?;
    serde_json::to_vec(&manifest).map_err(|_| Error::Unavailable)
}
fn admin() -> Result<(), Error> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
fn console_user() -> Result<(u32, u32), Error> {
    let uid = fs::symlink_metadata("/dev/console")
        .map_err(|_| Error::Unauthorized)?
        .uid();
    if uid == 0 {
        return Err(Error::Unauthorized);
    }
    let mut value: libc::passwd = unsafe { std::mem::zeroed() };
    let mut buffer = vec![0u8; 16384];
    let mut found = std::ptr::null_mut();
    if unsafe {
        libc::getpwuid_r(
            uid,
            &mut value,
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &mut found,
        )
    } != 0
        || found.is_null()
        || value.pw_uid != uid
        || value.pw_gid == 0
    {
        return Err(Error::Unauthorized);
    }
    Ok((uid, value.pw_gid))
}
fn source(name: &str, max: u64) -> Result<Vec<u8>, Error> {
    let path = Path::new(BUNDLE).join(name);
    // 安装输入也必须来自root保护的bundle，SHA manifest不作为自签授权。
    let mode = if name == "install-manifest.json" {
        0o644
    } else {
        0o755
    };
    let file = protected_file(&path, mode, max)?;
    let mut bytes = vec![];
    file.take(max + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    if bytes.len() as u64 > max {
        return Err(Error::TooLarge);
    }
    Ok(bytes)
}
fn validate(manifest: &Manifest, helper: &[u8], kernel: &[u8]) -> Result<(), Error> {
    if manifest.protocol != PROTOCOL
        || manifest.kernel_sha256 != veyra_core::application::runtime_recovery::KERNEL_DIGEST
        || format!("{:x}", Sha256::digest(helper)) != manifest.helper_sha256
        || format!("{:x}", Sha256::digest(kernel)) != manifest.kernel_sha256
    {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(())
}
fn create(path: &Path, bytes: &[u8], mode: u32) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(|_| Error::Busy)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| Error::Unavailable)?;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(|_| Error::Unavailable)?;
    std::fs::File::open(path.parent().ok_or(Error::UnsafeEndpoint)?)
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Unavailable)
}
fn launchctl(action: &str) -> Result<(), Error> {
    let mut command = Command::new("/bin/launchctl");
    command
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match action {
        "bootstrap" => {
            command.args(["bootstrap", "system", PLIST]);
        }
        "bootout" => {
            command.args(["bootout", LABEL]);
        }
        _ => return Err(Error::InvalidRequest),
    };
    let mut child = command.spawn().map_err(|_| Error::Unavailable)?;
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(s) = child.try_wait().map_err(|_| Error::Unavailable)? {
            return if s.success() {
                Ok(())
            } else {
                Err(Error::Failed)
            };
        }
        if Instant::now() >= until {
            veyra_core::application::owned_child::terminate(&mut child)
                .map_err(|_| Error::Unavailable)?;
            return Err(Error::Timeout);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
pub fn install() -> Result<(), Error> {
    admin()?;
    // 管理员只运行固定安装包内入口；不从下载管道、客户端或参数接受资源位置。
    if std::env::current_exe().map_err(|_| Error::Unavailable)?
        != Path::new(BUNDLE).join("veyra-helper")
    {
        return Err(Error::UnsafeEndpoint);
    }
    let manifest: Manifest = serde_json::from_slice(&source("install-manifest.json", 4096)?)
        .map_err(|_| Error::InvalidRequest)?;
    let helper = source("veyra-helper", 256 * 1024 * 1024)?;
    let kernel = source("veyra-sing-box", 256 * 1024 * 1024)?;
    validate(&manifest, &helper, &kernel)?;
    let (uid, gid) = console_user()?;
    let parent = Path::new(transport::ROOT).parent().unwrap();
    transport::protected_directories(parent.parent().unwrap())?;
    if !parent.exists() {
        fs::create_dir(parent).map_err(|_| Error::Unavailable)?;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o755))
            .map_err(|_| Error::Unavailable)?;
    }
    transport::protected_directories(parent)?;
    let _deployment = super::admin::lock(parent, "deployment-lock")?;
    super::archive::validate_archives(parent)?;
    if fs::symlink_metadata(transport::ROOT).is_ok() || fs::symlink_metadata(PLIST).is_ok() {
        return Err(Error::Busy);
    }
    transport::protected_directories(Path::new("/Library/LaunchDaemons"))?;
    fs::create_dir(transport::ROOT).map_err(|_| Error::Unavailable)?;
    fs::set_permissions(transport::ROOT, fs::Permissions::from_mode(0o755))
        .map_err(|_| Error::Unavailable)?;
    // 任一步失败保留已写固定材料，不启动半安装，也不删除诊断现场。
    create(
        &Path::new(transport::ROOT).join("veyra-helper"),
        &helper,
        0o755,
    )?;
    create(Path::new(transport::KERNEL), &kernel, 0o755)?;
    create(
        &Path::new(transport::ROOT).join("authorization.json"),
        &serde_json::to_vec(&serde_json::json!({"uid":uid,"gid":gid}))
            .map_err(|_| Error::InvalidRequest)?,
        0o600,
    )?;
    // 在同一deployment锁下、发布plist之前签发一次；create_new禁止半安装重试重置世代。
    record_installation(Path::new(transport::ROOT), uid, gid)?;
    create(Path::new(PLIST), installation_plist().as_bytes(), 0o644)?;
    launchctl("bootstrap")
}
/// 打包工具与管理员安装共用同一固定plist；不能指定label、程序或参数。
pub fn installation_plist() -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><plist version=\"1.0\"><dict><key>Label</key><string>me.disign.veyra.helper</string><key>ProgramArguments</key><array><string>{}/veyra-helper</string><string>serve</string></array><key>RunAtLoad</key><true/><key>WorkingDirectory</key><string>/</string></dict></plist>",
        transport::ROOT
    )
}
pub fn uninstall() -> Result<(), Error> {
    admin()?;
    let root = Path::new(transport::ROOT);
    let parent = root.parent().unwrap();
    transport::protected_directories(parent)?;
    let _deployment = super::admin::lock(parent, "deployment-lock")?;
    // rename已提交但响应丢失：不触碰archive，固定root/plist均不存在时重试成功。
    if !root.try_exists().map_err(|_| Error::UnsafeEndpoint)? && fs::symlink_metadata(root).is_err()
    {
        super::archive::confirm_committed(parent)?;
        return match fs::symlink_metadata(PLIST) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            _ => Err(Error::HandoffRequired),
        };
    }
    transport::protected_directories(root)?;
    // 管理员操作串行；从此请求永久封住该安装，取消/超时不隐式解除安全barrier。

    for (name, mode) in [
        ("veyra-helper", 0o755),
        ("veyra-sing-box", 0o755),
        ("authorization.json", 0o600),
    ] {
        protected_file(
            &Path::new(transport::ROOT).join(name),
            mode,
            256 * 1024 * 1024,
        )?;
    }
    transport::protected_directories(Path::new("/Library/LaunchDaemons"))?;
    super::archive::validate_plist(root, Path::new(PLIST), installation_plist().as_bytes())?;
    super::admin::request(root)?;
    let until = Instant::now() + Duration::from_secs(35);
    loop {
        if super::admin::acknowledged(root)? {
            break;
        }
        match super::admin::lock(root, "lifecycle-lock") {
            Ok(_offline) => {
                // 服务已退出不能证明旧child已死。只有完全没有runtime历史才可离线卸载。
                super::admin::no_runtime(root)?;
                if fs::symlink_metadata(transport::SOCKET).is_ok() {
                    return Err(Error::HandoffRequired);
                }
                super::admin::acknowledge(root)?;
                break;
            }
            Err(Error::Busy) => {}
            Err(error) => return Err(error),
        }
        if Instant::now() >= until {
            return Err(Error::Timeout);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // ACK由原worker在Stop/reap/封存成功后生成；不是root伪装业务session的Stop。
    super::archive::complete_bootout(root, || launchctl("bootout"))?;
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        match super::archive::finish(root, Path::new(PLIST), installation_plist().as_bytes()) {
            Ok(_) => return Ok(()),
            Err(Error::Busy) if Instant::now() < until => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(error) => return Err(error),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installation_generation_is_exclusive_and_survives_archive() {
        // 保护首次安装的世代不会因重试/重装被覆盖；真文件锁/rename，非管理员安装。
        let parent = std::env::temp_dir().join(format!(
            "v206-generation-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        fs::create_dir(&parent).unwrap();
        fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).unwrap();
        let root = parent.join("Helper");
        fs::create_dir(&root).unwrap();
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        let lock = super::super::admin::lock(&parent, "deployment-lock").unwrap();
        assert!(matches!(
            super::super::admin::lock(&parent, "deployment-lock"),
            Err(Error::Busy)
        ));
        assert_eq!(
            installed_identity(&root, uid, gid),
            Err(Error::HandoffRequired)
        );
        record_installation(&root, uid, gid).unwrap();
        let first = installed_identity(&root, uid, gid).unwrap();
        let bytes = fs::read(root.join("installation.json")).unwrap();
        assert_eq!(record_installation(&root, uid, gid), Err(Error::Busy));
        assert_eq!(fs::read(root.join("installation.json")).unwrap(), bytes);
        // 复用真实Archive事务；bootout仅受控mock，不执行launchctl。
        let plist = parent.join("fixture.plist");
        fs::write(&plist, installation_plist()).unwrap();
        fs::set_permissions(&plist, fs::Permissions::from_mode(0o644)).unwrap();
        super::super::admin::request(&root).unwrap();
        super::super::admin::acknowledge(&root).unwrap();
        super::super::archive::complete_bootout(&root, || Ok(())).unwrap();
        let archived =
            super::super::archive::finish(&root, &plist, installation_plist().as_bytes()).unwrap();
        assert_eq!(installed_identity(&archived, uid, gid).unwrap(), first);
        fs::create_dir(&root).unwrap();
        // 复制旧安装凭据至新root不能授权该inode；测试后只删除本测试自己写入的复制品。
        fs::write(root.join("installation.json"), &bytes).unwrap();
        fs::set_permissions(
            root.join("installation.json"),
            fs::Permissions::from_mode(0o600),
        )
        .unwrap();
        assert_eq!(
            installed_identity(&root, uid, gid),
            Err(Error::UnsafeEndpoint)
        );
        fs::remove_file(root.join("installation.json")).unwrap();
        record_installation(&root, uid, gid).unwrap();
        let second = installed_identity(&root, uid, gid).unwrap();
        assert_ne!(first.generation, second.generation);
        assert_eq!(fs::read(archived.join("installation.json")).unwrap(), bytes);
        drop(lock);
        fs::remove_dir_all(parent).unwrap();
    }
    #[test]
    fn installation_identity_rejects_permissions_peer_and_partial_record() {
        // 保护缺失/半写/可篡改安装证据不能被当作干净首次启动，且诊断不修改原件。
        let root = std::env::temp_dir().join(format!(
            "v206-identity-{:?}",
            veyra_core::domain::StateEpoch::fresh().unwrap()
        ));
        fs::create_dir(&root).unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        let uid = unsafe { libc::geteuid() };
        let gid = unsafe { libc::getegid() };
        record_installation(&root, uid, gid).unwrap();
        let path = root.join("installation.json");
        let bytes = fs::read(&path).unwrap();
        assert_eq!(
            installed_identity(&root, uid + 1, gid),
            Err(Error::UnsafeEndpoint)
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o666)).unwrap();
        assert_eq!(
            installed_identity(&root, uid, gid),
            Err(Error::UnsafeEndpoint)
        );
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        fs::hard_link(&path, root.join("alias")).unwrap();
        assert_eq!(
            installed_identity(&root, uid, gid),
            Err(Error::UnsafeEndpoint)
        );
        fs::remove_file(root.join("alias")).unwrap();
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink("target", &path).unwrap();
        fs::write(root.join("target"), &bytes).unwrap();
        assert!(installed_identity(&root, uid, gid).is_err());
        assert_eq!(fs::read(root.join("target")).unwrap(), bytes);
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"{").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(
            installed_identity(&root, uid, gid),
            Err(Error::UnsafeEndpoint)
        );
        assert_eq!(record_installation(&root, uid, gid), Err(Error::Busy));
        assert_eq!(fs::read(&path).unwrap(), b"{");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_unprivileged_install_and_uninstall_before_filesystem() {
        assert_ne!(unsafe { libc::geteuid() }, 0);
        assert_eq!(install(), Err(Error::Unauthorized));
        assert_eq!(uninstall(), Err(Error::Unauthorized));
    }
    #[test]
    fn fixed_plist_and_manifest_refuse_untrusted_assets() {
        assert_eq!(
            installation_manifest(b"fixture", b"foreign-kernel"),
            Err(Error::UnsafeEndpoint)
        );
        assert_eq!(installation_manifest(b"", b""), Err(Error::InvalidRequest));
        let p = installation_plist();
        // 安装器、launchctl目标和生成plist必须使用同一新身份；不执行管理员命令。
        assert_eq!(LABEL, "system/me.disign.veyra.helper");
        assert_eq!(PLIST, "/Library/LaunchDaemons/me.disign.veyra.helper.plist");
        assert!(p.contains("<string>me.disign.veyra.helper</string>"));
        assert!(p.contains("<string>serve</string>"));
        assert!(!p.contains("sudo"));
        let m = Manifest {
            protocol: PROTOCOL,
            helper_sha256: format!("{:x}", Sha256::digest(b"fixture")),
            kernel_sha256: veyra_core::application::runtime_recovery::KERNEL_DIGEST.into(),
        };
        assert_eq!(
            validate(&m, b"fixture", b"foreign-kernel"),
            Err(Error::UnsafeEndpoint)
        );
        assert!(
            serde_json::from_str::<Manifest>(
                r#"{"protocol":1,"helper_sha256":"x","kernel_sha256":"y","path":"/tmp/kernel"}"#
            )
            .is_err()
        );
    }
}
