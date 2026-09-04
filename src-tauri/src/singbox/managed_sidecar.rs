//! TASK-006 的受管 sing-box 资产与启动边界。
//!
//! 这里不提供通用命令、路径或 JSON 入口。调用方只能在后续生命周期接线中使用
//! 已打包的固定资源、由语义编译器产生的配置和本模块生成的临时 secret。

// 本 checkpoint 先交付独立可测的安全边界；真实生命周期接线将在同一 TASK 的后续分区完成。
#![allow(dead_code)]

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use getrandom::fill;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::GeneratedConfig;

pub(crate) const SIDECAR_ARCHIVE_FILE: &str = "sing-box-1.14.0-windows-amd64.zip";
pub(crate) const SIDECAR_EXECUTABLE_FILE: &str = "sing-box.exe";
pub(crate) const SIDECAR_LIBRARY_FILE: &str = "libcronet.dll";
pub(crate) const SIDECAR_LICENSE_FILE: &str = "LICENSE";
pub(crate) const SIDECAR_ARCHIVE_SHA256: &str =
    "3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6";
pub(crate) const SIDECAR_EXECUTABLE_SHA256: &str =
    "aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc";
pub(crate) const SIDECAR_LIBRARY_SHA256: &str =
    "eee741046f0a3975124bae349aeac237aa306f3cc4de59ff5de070e74dbfdaeb";
pub(crate) const SIDECAR_LICENSE_SHA256: &str =
    "bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c";

const SECRET_BYTES: usize = 32;
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_TIMEOUT: Duration = Duration::from_secs(2);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Error, Eq, PartialEq)]
pub(crate) enum ManagedSidecarError {
    #[error("embedded sidecar asset integrity verification failed")]
    AssetIntegrity,
    #[error("generated sidecar configuration violates the managed runtime contract")]
    ConfigurationRejected,
    #[error("could not generate the sidecar API secret")]
    SecretGeneration,
    #[error("sidecar configuration check failed")]
    ConfigurationCheck,
    #[error("could not launch the managed sidecar")]
    Spawn,
    #[error("could not stop the managed sidecar")]
    Stop,
}

/// 仅受管 sidecar 与固定 API client 可借用的短生命周期 secret。
///
/// 它不实现 `Clone`、`Debug` 或字符串转换，避免被传播到错误、事件或 UI。释放时会覆盖
/// 自己持有的 ASCII hex 缓冲区；生成配置序列化过程中的临时副本同样只在写入私有 config 前后短暂存在。
pub(crate) struct ApiSecret {
    value: String,
}

impl ApiSecret {
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

impl Drop for ApiSecret {
    fn drop(&mut self) {
        // ASCII hex 长度固定，NUL 仍是合法 UTF-8，因此不会破坏 `String` 不变量。
        unsafe {
            self.value.as_mut_vec().fill(0);
        }
    }
}

/// 从操作系统熵源生成单次运行使用的 API secret，不持久化且不接受外部输入。
pub(crate) fn generate_api_secret() -> Result<ApiSecret, ManagedSidecarError> {
    let mut bytes = [0_u8; SECRET_BYTES];
    fill(&mut bytes).map_err(|_| ManagedSidecarError::SecretGeneration)?;

    let mut secret = String::with_capacity(SECRET_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut secret, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(ApiSecret { value: secret })
}

/// 验证唯一允许的构建缓存归档及其完整三文件内容，避免打包替换的内置资产。
pub(crate) fn verify_embedded_assets(
    archive_path: &Path,
    executable_path: &Path,
    library_path: &Path,
    license_path: &Path,
) -> Result<(), ManagedSidecarError> {
    verify_file_sha256(archive_path, SIDECAR_ARCHIVE_SHA256)?;
    verify_bundled_resources(executable_path, library_path, license_path)
}

/// 运行期只接受 Tauri bundle 中固定的 executable、DLL 与许可证，且逐个校验身份。
pub(crate) fn verify_bundled_resources(
    executable_path: &Path,
    library_path: &Path,
    license_path: &Path,
) -> Result<(), ManagedSidecarError> {
    verify_file_sha256(executable_path, SIDECAR_EXECUTABLE_SHA256)?;
    verify_file_sha256(library_path, SIDECAR_LIBRARY_SHA256)?;
    verify_file_sha256(license_path, SIDECAR_LICENSE_SHA256)
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<(), ManagedSidecarError> {
    let actual = sha256_file(path).map_err(|_| ManagedSidecarError::AssetIntegrity)?;
    if actual == expected {
        Ok(())
    } else {
        Err(ManagedSidecarError::AssetIntegrity)
    }
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 从已通过最终白名单的配置提取认证身份；不再注入或重新序列化配置。
pub(crate) fn api_secret_from_config(
    generated: &GeneratedConfig,
) -> Result<ApiSecret, ManagedSidecarError> {
    generated
        .validate_final()
        .map_err(|_| ManagedSidecarError::ConfigurationRejected)?;
    let mut document: Value = serde_json::from_slice(generated.as_bytes())
        .map_err(|_| ManagedSidecarError::ConfigurationRejected)?;
    let value = document["experimental"]["clash_api"]["secret"].take();
    let Value::String(value) = value else {
        return Err(ManagedSidecarError::ConfigurationRejected);
    };
    Ok(ApiSecret { value })
}

#[cfg(test)]
pub(crate) fn test_api_secret() -> ApiSecret {
    ApiSecret {
        value: "a".repeat(64),
    }
}

/// 已启动 child 的唯一所有权；不暴露 PID，也不会扫描或清理其他进程。
pub(crate) struct ManagedSidecar {
    child: Child,
}

impl ManagedSidecar {
    /// 仅用于验证测试配置的实际 listener 归属，不进入产品 API。
    #[cfg(test)]
    pub(crate) fn test_process_id(&self) -> u32 {
        self.child.id()
    }

    /// 先返回 check 子进程归属，等待或超时失败不能让临时 Child 丢失。
    pub(crate) fn start_check(
        executable_path: &Path,
        config_path: &Path,
    ) -> Result<Self, ManagedSidecarError> {
        let child = managed_command(executable_path)
            .arg("check")
            .arg("-c")
            .arg(config_path)
            .spawn()
            .map_err(|_| ManagedSidecarError::ConfigurationCheck)?;
        Ok(Self { child })
    }

    /// check 最长 10s，超时后只有一次 kill/2s 退出确认；失败时 owner 仍持有本对象。
    pub(crate) fn wait_check(&mut self) -> Result<(), ManagedSidecarError> {
        match wait_for_exit(&mut self.child, CHECK_TIMEOUT) {
            Some(status) if status.success() => Ok(()),
            Some(_) => Err(ManagedSidecarError::ConfigurationCheck),
            None => {
                self.stop()?;
                Err(ManagedSidecarError::ConfigurationCheck)
            }
        }
    }

    /// 仅在 [`Self::wait_check`] 已成功后使用固定 `run -c` 启动。所有标准流均为 null，
    /// Windows 不创建控制台窗口。
    pub(crate) fn start_checked(
        executable_path: &Path,
        config_path: &Path,
    ) -> Result<Self, ManagedSidecarError> {
        let child = managed_command(executable_path)
            .arg("run")
            .arg("-c")
            .arg(config_path)
            .spawn()
            .map_err(|_| ManagedSidecarError::Spawn)?;
        Ok(Self { child })
    }

    /// 只检查本对象拥有的 child 是否仍在运行；不暴露 PID。
    pub(crate) fn is_running(&mut self) -> Result<bool, ManagedSidecarError> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|_| ManagedSidecarError::Stop)
    }

    /// 只终止这个结构体拥有的 child；调用方无需也不能提供 PID。
    pub(crate) fn stop(&mut self) -> Result<(), ManagedSidecarError> {
        match self
            .child
            .try_wait()
            .map_err(|_| ManagedSidecarError::Stop)?
        {
            Some(_) => Ok(()),
            None => {
                self.child.kill().map_err(|_| ManagedSidecarError::Stop)?;
                wait_for_exit(&mut self.child, STOP_TIMEOUT)
                    .map(|_| ())
                    .ok_or(ManagedSidecarError::Stop)
            }
        }
    }
}

impl Drop for ManagedSidecar {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now().checked_add(timeout)?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(PROCESS_POLL_INTERVAL),
            Ok(None) | Err(_) => return None,
        }
    }
}

fn managed_command(executable_path: &Path) -> Command {
    let mut command = Command::new(executable_path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn is_valid_secret(secret: &str) -> bool {
    secret.len() == SECRET_BYTES * 2
        && secret.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secret_is_fixed_length_lowercase_hex() {
        let secret = generate_api_secret().expect("system entropy");

        assert!(is_valid_secret(secret.as_str()));
        assert_eq!(secret.as_str().len(), 64);
    }

    #[test]
    fn rejects_non_compiler_json_without_echoing_credentials() {
        let config = GeneratedConfig::from_bytes(
            br#"{"outbounds":[],"route":{},"inbounds":[{"type":"tun"}],"secret":"fixture-secret"}"#
                .to_vec(),
        );
        let error = api_secret_from_config(&config)
            .err()
            .expect("reject raw config");
        assert_eq!(error, ManagedSidecarError::ConfigurationRejected);
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn file_hash_is_lowercase_sha256() {
        let path =
            std::env::temp_dir().join(format!("veyra-managed-sidecar-hash-{}", std::process::id()));
        std::fs::write(&path, b"veyra").expect("write fixture");

        let hash = sha256_file(&path).expect("hash fixture");
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            hash,
            "954d457a7a9d956481b90e6b30a4bc101ec6120d4e63866b761be906ea13c7d1"
        );
    }
}
