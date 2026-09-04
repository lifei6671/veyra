//! 受管 sidecar 的 Windows 私有运行目录。
//!
//! 此模块只接收应用内部给定的父目录和语义编译后的配置；不接受前端路径、PID、
//! 用户名或 SID。目录和 config 在写入 API secret 前都必须回读为当前 TokenUser
//! SID 独占的受保护 DACL。
#![allow(dead_code)]

use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::windows::{fs::MetadataExt, prelude::OsStrExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_SUCCESS, GENERIC_ALL, HLOCAL, LocalFree},
        Security::Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT, SetNamedSecurityInfoW},
        Security::{
            ACCESS_ALLOWED_ACE, ACL, ACL_REVISION, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION,
            EqualSid, GetAce, GetAclInformation, GetLengthSid, GetSecurityDescriptorControl,
            GetSecurityDescriptorDacl, GetSecurityDescriptorOwner, GetTokenInformation,
            INHERITED_ACE, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, PSID, SE_DACL_PROTECTED, TOKEN_QUERY, TOKEN_USER, TokenUser,
        },
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
    core::PCWSTR,
};

use crate::singbox::GeneratedConfig;

const CONFIG_FILE: &str = "config.json";
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const FILE_ALL_ACCESS: u32 = 0x001f_01ff;
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(0);

/// 向调用方隐藏 Windows 路径、SID 和底层错误，避免把 private runtime 细节带出平台层。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum PrivateRuntimeError {
    #[error("could not prepare the private sidecar runtime")]
    Preparation,
    #[error("could not write the private sidecar configuration")]
    ConfigurationWrite,
    #[error("could not clean the private sidecar runtime")]
    Cleanup,
}

/// 创建失败只携带固定类别；清理未完成的对象必须转交上层 owner。
#[derive(Error)]
pub(crate) enum PrivateRuntimeCreateError {
    #[error("could not prepare the private sidecar runtime")]
    Preparation,
    #[error("private sidecar runtime cleanup is pending")]
    CleanupPending(PrivateRuntime),
}

impl std::fmt::Debug for PrivateRuntimeCreateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Preparation => "PrivateRuntimeCreateError::Preparation",
            Self::CleanupPending(_) => "PrivateRuntimeCreateError::CleanupPending",
        })
    }
}

impl From<PrivateRuntimeError> for PrivateRuntimeCreateError {
    fn from(_: PrivateRuntimeError) -> Self {
        Self::Preparation
    }
}

/// 经 ACL 设置与 readback 证明后才能使用的固定 config 路径。
pub(crate) struct PrivateRuntime {
    instance_dir: PathBuf,
    config_path: PathBuf,
}

impl PrivateRuntime {
    /// 下次受管运行前，只回收名称、owner 与 DACL 都可证明属于本应用的遗留实例。
    ///
    /// 目录中任一无法证明归属的项目都会使调用方保持 fail-closed，不能继续创建真实运行时。
    pub(crate) fn cleanup_orphaned_instances(
        runtime_root: &Path,
    ) -> Result<(), PrivateRuntimeError> {
        if !runtime_root.exists() {
            return Ok(());
        }

        ensure_directory_not_reparse(runtime_root).map_err(|_| PrivateRuntimeError::Cleanup)?;
        let sid = CurrentUserSid::query().map_err(|_| PrivateRuntimeError::Cleanup)?;
        verify_private_dacl(runtime_root, sid.sid()).map_err(|_| PrivateRuntimeError::Cleanup)?;

        for entry in fs::read_dir(runtime_root).map_err(|_| PrivateRuntimeError::Cleanup)? {
            let entry = entry.map_err(|_| PrivateRuntimeError::Cleanup)?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| PrivateRuntimeError::Cleanup)?;
            if !is_managed_instance_name(&name) {
                return Err(PrivateRuntimeError::Cleanup);
            }

            let instance_dir = entry.path();
            ensure_directory_not_reparse(&instance_dir)
                .map_err(|_| PrivateRuntimeError::Cleanup)?;
            verify_private_dacl(&instance_dir, sid.sid())
                .map_err(|_| PrivateRuntimeError::Cleanup)?;

            let config_path = instance_dir.join(CONFIG_FILE);
            let mut entry_count = 0;
            for instance_entry in
                fs::read_dir(&instance_dir).map_err(|_| PrivateRuntimeError::Cleanup)?
            {
                let instance_entry = instance_entry.map_err(|_| PrivateRuntimeError::Cleanup)?;
                entry_count += 1;
                if entry_count != 1 || instance_entry.file_name() != CONFIG_FILE {
                    return Err(PrivateRuntimeError::Cleanup);
                }
                ensure_file_not_reparse(&config_path).map_err(|_| PrivateRuntimeError::Cleanup)?;
                verify_private_dacl(&config_path, sid.sid())
                    .map_err(|_| PrivateRuntimeError::Cleanup)?;
            }

            PrivateRuntime {
                instance_dir,
                config_path,
            }
            .remove_config_and_directory()?;
        }
        Ok(())
    }

    /// 创建唯一的应用生成实例目录和空 config；两者均不会继承父级 ACL。
    pub(crate) fn create(
        runtime_root: &Path,
        #[cfg(test)] after_instance_created: Option<fn(&Path)>,
    ) -> Result<Self, PrivateRuntimeCreateError> {
        if !runtime_root.exists() {
            fs::create_dir_all(runtime_root).map_err(|_| PrivateRuntimeError::Preparation)?;
        }
        ensure_directory_not_reparse(runtime_root).map_err(|_| PrivateRuntimeError::Preparation)?;
        let sid = CurrentUserSid::query().map_err(|_| PrivateRuntimeError::Preparation)?;
        secure_and_verify(runtime_root, sid.sid()).map_err(|_| PrivateRuntimeError::Preparation)?;

        let instance_dir = create_instance_directory(runtime_root)?;
        if ensure_directory_not_reparse(&instance_dir).is_err()
            || secure_and_verify(&instance_dir, sid.sid()).is_err()
        {
            return cleanup_after_prepare_failure(&instance_dir);
        }
        #[cfg(test)]
        if let Some(after_instance_created) = after_instance_created {
            after_instance_created(&instance_dir);
        }

        let config_path = instance_dir.join(CONFIG_FILE);
        if OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&config_path)
            .is_err()
            || ensure_file_not_reparse(&config_path).is_err()
            || secure_and_verify(&config_path, sid.sid()).is_err()
        {
            return cleanup_after_prepare_failure(&instance_dir);
        }

        Ok(Self {
            instance_dir,
            config_path,
        })
    }

    /// 写入前再次检查目录和 config 的 DACL；调用方只能交付语义编译器产生的配置。
    pub(crate) fn write_checked_config(
        &self,
        config: &GeneratedConfig,
    ) -> Result<(), PrivateRuntimeError> {
        let sid = CurrentUserSid::query().map_err(|_| PrivateRuntimeError::ConfigurationWrite)?;
        if ensure_directory_not_reparse(&self.instance_dir).is_err()
            || ensure_file_not_reparse(&self.config_path).is_err()
            || verify_private_dacl(&self.instance_dir, sid.sid()).is_err()
            || verify_private_dacl(&self.config_path, sid.sid()).is_err()
        {
            return Err(PrivateRuntimeError::ConfigurationWrite);
        }

        let result = (|| {
            let mut file = OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(&self.config_path)?;
            file.write_all(config.as_bytes())?;
            file.sync_all()
        })();
        if result.is_ok() {
            return Ok(());
        }

        self.remove_config_after_failed_write()
    }

    pub(crate) fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// 只删除本对象创建且仍能证明不是 reparse point 的固定 config/实例目录。
    pub(crate) fn cleanup(&self) -> Result<(), PrivateRuntimeError> {
        self.remove_config_and_directory()
    }

    fn remove_config_after_failed_write(&self) -> Result<(), PrivateRuntimeError> {
        match self.remove_config_and_directory() {
            Ok(()) => Err(PrivateRuntimeError::ConfigurationWrite),
            Err(_) => Err(PrivateRuntimeError::Cleanup),
        }
    }

    fn remove_config_and_directory(&self) -> Result<(), PrivateRuntimeError> {
        if self.config_path.exists() {
            ensure_file_not_reparse(&self.config_path).map_err(|_| PrivateRuntimeError::Cleanup)?;
            fs::remove_file(&self.config_path).map_err(|_| PrivateRuntimeError::Cleanup)?;
        }
        ensure_directory_not_reparse(&self.instance_dir)
            .map_err(|_| PrivateRuntimeError::Cleanup)?;
        fs::remove_dir(&self.instance_dir).map_err(|_| PrivateRuntimeError::Cleanup)
    }
}

fn cleanup_after_prepare_failure(
    instance_dir: &Path,
) -> Result<PrivateRuntime, PrivateRuntimeCreateError> {
    let runtime = PrivateRuntime {
        instance_dir: instance_dir.to_owned(),
        config_path: instance_dir.join(CONFIG_FILE),
    };
    match runtime.remove_config_and_directory() {
        Ok(()) => Err(PrivateRuntimeCreateError::Preparation),
        Err(_) => Err(PrivateRuntimeCreateError::CleanupPending(runtime)),
    }
}

fn create_instance_directory(runtime_root: &Path) -> Result<PathBuf, PrivateRuntimeError> {
    for _ in 0..32 {
        let instance_dir = runtime_root.join(next_instance_id());
        match fs::create_dir(&instance_dir) {
            // 创建后立即把路径交回 create；后续验证失败也必须保留清理归属。
            Ok(()) => return Ok(instance_dir),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(PrivateRuntimeError::Preparation),
        }
    }
    Err(PrivateRuntimeError::Preparation)
}

fn next_instance_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
    format!("sidecar-{timestamp:032x}-{sequence:016x}")
}

fn is_managed_instance_name(name: &str) -> bool {
    let Some(suffix) = name.strip_prefix("sidecar-") else {
        return false;
    };
    let Some((timestamp, sequence)) = suffix.split_once('-') else {
        return false;
    };
    timestamp.len() == 32
        && sequence.len() == 16
        && timestamp.bytes().chain(sequence.bytes()).all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

fn ensure_directory_not_reparse(path: &Path) -> Result<(), ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    (metadata.is_dir() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0)
        .then_some(())
        .ok_or(())
}

fn ensure_file_not_reparse(path: &Path) -> Result<(), ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    (metadata.is_file() && metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT == 0)
        .then_some(())
        .ok_or(())
}

fn secure_and_verify(path: &Path, sid: PSID) -> Result<(), ()> {
    let mut acl_storage = vec![0_usize; acl_buffer_len(sid)?];
    let acl = acl_storage.as_mut_ptr().cast::<ACL>();
    unsafe {
        windows::Win32::Security::InitializeAcl(
            acl,
            (acl_storage.len() * std::mem::size_of::<usize>()) as u32,
            ACL_REVISION,
        )
        .map_err(|_| ())?;
        windows::Win32::Security::AddAccessAllowedAce(acl, ACL_REVISION, GENERIC_ALL.0, sid)
            .map_err(|_| ())?;
    }

    let security_info = OWNER_SECURITY_INFORMATION
        | DACL_SECURITY_INFORMATION
        | PROTECTED_DACL_SECURITY_INFORMATION;
    let path_wide = wide_path(path)?;
    let result = unsafe {
        SetNamedSecurityInfoW(
            PCWSTR(path_wide.as_ptr()),
            SE_FILE_OBJECT,
            security_info,
            Some(sid),
            None,
            Some(acl.cast_const()),
            None,
        )
    };
    if result != ERROR_SUCCESS {
        return Err(());
    }
    verify_private_dacl(path, sid)
}

fn acl_buffer_len(sid: PSID) -> Result<usize, ()> {
    let sid_len = unsafe { GetLengthSid(sid) } as usize;
    if sid_len == 0 {
        return Err(());
    }
    let acl_bytes = std::mem::size_of::<ACL>() + std::mem::size_of::<ACCESS_ALLOWED_ACE>()
        - std::mem::size_of::<u32>()
        + sid_len;
    Ok(acl_bytes.div_ceil(std::mem::size_of::<usize>()))
}

fn verify_private_dacl(path: &Path, expected_sid: PSID) -> Result<(), ()> {
    let path_wide = wide_path(path)?;
    let mut owner = PSID::default();
    let mut dacl = std::ptr::null_mut::<ACL>();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    let security_info = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
    let result = unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(path_wide.as_ptr()),
            SE_FILE_OBJECT,
            security_info,
            Some(&mut owner),
            None,
            Some(&mut dacl),
            None,
            &mut descriptor,
        )
    };
    if result != ERROR_SUCCESS || descriptor.is_invalid() {
        return Err(());
    }
    let descriptor_guard = LocalAllocation(descriptor.0);
    let verified = unsafe {
        EqualSid(owner, expected_sid).map_err(|_| ())?;

        let mut descriptor_owner = PSID::default();
        let mut owner_defaulted = false.into();
        GetSecurityDescriptorOwner(descriptor, &mut descriptor_owner, &mut owner_defaulted)
            .map_err(|_| ())?;
        EqualSid(descriptor_owner, expected_sid).map_err(|_| ())?;

        let mut control = 0_u16;
        let mut revision = 0_u32;
        GetSecurityDescriptorControl(descriptor, &mut control, &mut revision).map_err(|_| ())?;
        if control & SE_DACL_PROTECTED.0 == 0 {
            return Err(());
        }

        let mut dacl_present = false.into();
        let mut descriptor_dacl = std::ptr::null_mut::<ACL>();
        let mut dacl_defaulted = false.into();
        GetSecurityDescriptorDacl(
            descriptor,
            &mut dacl_present,
            &mut descriptor_dacl,
            &mut dacl_defaulted,
        )
        .map_err(|_| ())?;
        if !dacl_present.as_bool() || descriptor_dacl.is_null() || descriptor_dacl != dacl {
            return Err(());
        }

        let mut acl_info = ACL_SIZE_INFORMATION::default();
        GetAclInformation(
            dacl,
            (&mut acl_info as *mut ACL_SIZE_INFORMATION).cast(),
            std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32,
            windows::Win32::Security::AclSizeInformation,
        )
        .map_err(|_| ())?;
        if acl_info.AceCount != 1 {
            return Err(());
        }

        let mut ace = std::ptr::null_mut();
        GetAce(dacl, 0, &mut ace).map_err(|_| ())?;
        let ace = ace.cast::<ACCESS_ALLOWED_ACE>();
        if ace.is_null()
            || (*ace).Header.AceType != ACCESS_ALLOWED_ACE_TYPE
            || (*ace).Header.AceFlags & INHERITED_ACE.0 as u8 != 0
            || (*ace).Mask != FILE_ALL_ACCESS
        {
            return Err(());
        }
        let ace_sid = PSID(std::ptr::addr_of_mut!((*ace).SidStart).cast());
        EqualSid(ace_sid, expected_sid).map_err(|_| ())?;
        Ok(())
    };
    drop(descriptor_guard);
    verified
}

fn wide_path(path: &Path) -> Result<Vec<u16>, ()> {
    let mut path: Vec<u16> = path.as_os_str().encode_wide().collect();
    if path.is_empty() || path.contains(&0) {
        return Err(());
    }
    path.push(0);
    Ok(path)
}

struct CurrentUserSid {
    token: windows::Win32::Foundation::HANDLE,
    _storage: Vec<usize>,
    sid: PSID,
}

impl CurrentUserSid {
    fn query() -> Result<Self, ()> {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
            .map_err(|_| ())?;

        let mut required = 0_u32;
        let initial = unsafe { GetTokenInformation(token, TokenUser, None, 0, &mut required) };
        if initial.is_ok() || required == 0 {
            unsafe { CloseHandle(token) }.map_err(|_| ())?;
            return Err(());
        }

        let mut storage = vec![0_usize; (required as usize).div_ceil(std::mem::size_of::<usize>())];
        if unsafe {
            GetTokenInformation(
                token,
                TokenUser,
                Some(storage.as_mut_ptr().cast()),
                required,
                &mut required,
            )
        }
        .is_err()
        {
            unsafe { CloseHandle(token) }.map_err(|_| ())?;
            return Err(());
        }
        let sid = unsafe { storage.as_ptr().cast::<TOKEN_USER>().read().User.Sid };
        if sid.is_invalid() {
            unsafe { CloseHandle(token) }.map_err(|_| ())?;
            return Err(());
        }
        Ok(Self {
            token,
            _storage: storage,
            sid,
        })
    }

    fn sid(&self) -> PSID {
        self.sid
    }
}

impl Drop for CurrentUserSid {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.token);
        }
    }
}

struct LocalAllocation(*mut core::ffi::c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        unsafe {
            let _ = LocalFree(Some(HLOCAL(self.0)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::singbox::GeneratedConfig;

    fn temporary_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "veyra-private-runtime-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).expect("create test root");
        root
    }

    fn create_directory_reparse_fixture(target: &Path, link: &Path) -> std::io::Result<()> {
        match std::os::windows::fs::symlink_dir(target, link) {
            Ok(()) => Ok(()),
            Err(error)
                if error.kind() == std::io::ErrorKind::PermissionDenied
                    || error.raw_os_error() == Some(1314) =>
            {
                let link = link.display().to_string().replace('\'', "''");
                let target = target.display().to_string().replace('\'', "''");
                let command = format!(
                    "New-Item -ItemType Junction -Path '{link}' -Target '{target}' | Out-Null"
                );
                let output = std::process::Command::new("powershell.exe")
                    .args(["-NoProfile", "-NonInteractive", "-Command", &command])
                    .output()?;
                if output.status.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other(format!(
                        "create directory reparse-point fixture with PowerShell: {}",
                        String::from_utf8_lossy(&output.stderr).trim()
                    )))
                }
            }
            Err(error) => Err(error),
        }
    }

    #[test]
    fn create_returns_owned_directory_after_preparation_and_cleanup_both_fail() {
        let root = temporary_root("create-cleanup-owner");
        let error = PrivateRuntime::create(
            &root,
            Some(|instance| {
                // 真实文件打开失败，随后 remove_file 也因同名目录失败。
                fs::create_dir(instance.join(CONFIG_FILE)).expect("create owned blocker");
            }),
        )
        .err()
        .expect("preparation must fail");
        assert_eq!(
            format!("{error:?}"),
            "PrivateRuntimeCreateError::CleanupPending"
        );
        let PrivateRuntimeCreateError::CleanupPending(runtime) = error else {
            panic!("failed cleanup must return its owner");
        };
        let instance = runtime.config_path().parent().unwrap().to_owned();
        assert!(instance.is_dir());
        assert!(runtime.cleanup().is_err());
        fs::remove_dir(runtime.config_path()).expect("remove owned blocker");
        runtime
            .cleanup()
            .expect("retry cleanup through returned owner");
        assert!(!instance.exists());
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn create_preparation_failure_with_successful_cleanup_returns_no_owner() {
        let root = temporary_root("create-cleaned-failure");
        let error = PrivateRuntime::create(
            &root,
            Some(|instance| {
                // create_new 被同名普通文件拒绝；清理仍可删除自有文件和目录。
                fs::write(instance.join(CONFIG_FILE), b"owned fixture").unwrap();
            }),
        )
        .err()
        .expect("exclusive config creation must fail");
        assert!(matches!(error, PrivateRuntimeCreateError::Preparation));
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn create_failure_before_instance_creation_has_no_cleanup_owner() {
        let root = temporary_root("create-early-failure");
        let invalid_root = root.join("file");
        fs::write(&invalid_root, b"owned fixture").unwrap();
        assert!(matches!(
            PrivateRuntime::create(&invalid_root, None),
            Err(PrivateRuntimeCreateError::Preparation)
        ));
        fs::remove_file(invalid_root).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn creates_and_rechecks_current_user_exclusive_config_before_write() {
        let root = temporary_root("acl");
        let runtime = PrivateRuntime::create(&root, None).expect("create private runtime");
        let config = GeneratedConfig::from_bytes(b"{\"secret\":\"test-only\"}".to_vec());

        runtime
            .write_checked_config(&config)
            .expect("write checked config");
        assert_eq!(
            fs::read(runtime.config_path()).expect("read config"),
            config.as_bytes()
        );
        runtime.cleanup().expect("clean private runtime");
        fs::remove_dir(&root).expect("remove test root");
    }

    #[test]
    fn rejects_a_default_inherited_acl_before_it_can_be_used_for_config() {
        let root = temporary_root("inherited");
        let existing = root.join("existing-config.json");
        fs::write(&existing, b"not a private config").expect("write inherited fixture");
        let sid = CurrentUserSid::query().expect("current token user");

        assert!(verify_private_dacl(&existing, sid.sid()).is_err());
        fs::remove_file(&existing).expect("remove inherited fixture");
        fs::remove_dir(&root).expect("remove test root");
    }

    #[test]
    fn rejects_directory_reparse_points() {
        let root = temporary_root("reparse");
        let target = root.join("target");
        let link = root.join("link");
        fs::create_dir(&target).expect("create target");

        create_directory_reparse_fixture(&target, &link)
            .expect("create directory reparse-point fixture");
        assert!(ensure_directory_not_reparse(&link).is_err());

        let _ = fs::remove_dir(&link);
        fs::remove_dir(&target).expect("remove target");
        fs::remove_dir(&root).expect("remove test root");
    }

    #[test]
    fn cleans_only_a_proven_orphaned_instance() {
        let root = temporary_root("orphan");
        let runtime_root = root.join("sidecar-runtime");
        let runtime = PrivateRuntime::create(&runtime_root, None).expect("create private runtime");
        runtime
            .write_checked_config(&GeneratedConfig::from_bytes(b"{}".to_vec()))
            .expect("write orphaned config");
        drop(runtime);

        PrivateRuntime::cleanup_orphaned_instances(&runtime_root)
            .expect("clean proven orphaned instance");
        assert!(
            fs::read_dir(&runtime_root)
                .expect("read cleaned runtime root")
                .next()
                .is_none()
        );

        fs::remove_dir(&runtime_root).expect("remove runtime root");
        fs::remove_dir(&root).expect("remove test root");
    }

    #[test]
    fn refuses_to_clean_an_unproven_runtime_entry() {
        let root = temporary_root("unproven");
        let runtime_root = root.join("sidecar-runtime");
        let runtime = PrivateRuntime::create(&runtime_root, None).expect("create private runtime");
        runtime.cleanup().expect("clean initial instance");

        let unproven = runtime_root.join("not-a-managed-instance");
        fs::create_dir(&unproven).expect("create unproven entry");
        let sid = CurrentUserSid::query().expect("current token user");
        secure_and_verify(&unproven, sid.sid()).expect("secure unproven fixture");

        assert_eq!(
            PrivateRuntime::cleanup_orphaned_instances(&runtime_root),
            Err(PrivateRuntimeError::Cleanup)
        );

        fs::remove_dir(&unproven).expect("remove unproven entry");
        fs::remove_dir(&runtime_root).expect("remove runtime root");
        fs::remove_dir(&root).expect("remove test root");
    }
}
