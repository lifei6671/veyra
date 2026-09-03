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
};

use getrandom::fill;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::GeneratedConfig;

pub(crate) const SIDECAR_ARCHIVE_FILE: &str = "sing-box-1.14.0-windows-amd64.zip";
pub(crate) const SIDECAR_EXECUTABLE_FILE: &str = "sing-box-x86_64-pc-windows-msvc.exe";
pub(crate) const SIDECAR_LIBRARY_FILE: &str = "libcronet.dll";
pub(crate) const SIDECAR_ARCHIVE_SHA256: &str =
    "3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6";
pub(crate) const SIDECAR_EXECUTABLE_SHA256: &str =
    "aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc";

const CLASH_API_ADDRESS: &str = "127.0.0.1:9090";
const SECRET_BYTES: usize = 32;

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

/// 从操作系统熵源生成单次运行使用的 API secret，不持久化且不接受外部输入。
pub(crate) fn generate_api_secret() -> Result<String, ManagedSidecarError> {
    let mut bytes = [0_u8; SECRET_BYTES];
    fill(&mut bytes).map_err(|_| ManagedSidecarError::SecretGeneration)?;

    let mut secret = String::with_capacity(SECRET_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut secret, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(secret)
}

/// 验证唯一允许的归档和已解出的 executable，避免运行被替换的内置资产。
pub(crate) fn verify_embedded_assets(
    archive_path: &Path,
    executable_path: &Path,
) -> Result<(), ManagedSidecarError> {
    verify_file_sha256(archive_path, SIDECAR_ARCHIVE_SHA256)?;
    verify_file_sha256(executable_path, SIDECAR_EXECUTABLE_SHA256)
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

/// 只在语义编译器的顶层 JSON 上注入固定 loopback Clash API；拒绝任何额外运行面。
pub(crate) fn with_fixed_clash_api(
    generated: &GeneratedConfig,
    secret: &str,
) -> Result<GeneratedConfig, ManagedSidecarError> {
    if !is_valid_secret(secret) {
        return Err(ManagedSidecarError::ConfigurationRejected);
    }

    let mut document: Value = serde_json::from_slice(generated.as_bytes())
        .map_err(|_| ManagedSidecarError::ConfigurationRejected)?;
    let object = document
        .as_object_mut()
        .ok_or(ManagedSidecarError::ConfigurationRejected)?;

    if object
        .keys()
        .any(|key| key != "outbounds" && key != "route")
        || !object.contains_key("outbounds")
        || !object.contains_key("route")
    {
        return Err(ManagedSidecarError::ConfigurationRejected);
    }

    object.insert(
        "experimental".to_owned(),
        json!({
            "clash_api": {
                "external_controller": CLASH_API_ADDRESS,
                "secret": secret,
            }
        }),
    );

    let serialized =
        serde_json::to_vec(&document).map_err(|_| ManagedSidecarError::ConfigurationRejected)?;
    Ok(GeneratedConfig::from_bytes(serialized))
}

/// 已启动 child 的唯一所有权；不暴露 PID，也不会扫描或清理其他进程。
pub(crate) struct ManagedSidecar {
    child: Child,
}

impl ManagedSidecar {
    /// 使用固定 `check` 再 `run -c` 序列启动。所有标准流均为 null，Windows 不创建控制台窗口。
    pub(crate) fn start(
        executable_path: &Path,
        config_path: &Path,
    ) -> Result<Self, ManagedSidecarError> {
        let checked = managed_command(executable_path)
            .arg("check")
            .arg("-c")
            .arg(config_path)
            .status()
            .map_err(|_| ManagedSidecarError::ConfigurationCheck)?;
        if !checked.success() {
            return Err(ManagedSidecarError::ConfigurationCheck);
        }

        let child = managed_command(executable_path)
            .arg("run")
            .arg("-c")
            .arg(config_path)
            .spawn()
            .map_err(|_| ManagedSidecarError::Spawn)?;
        Ok(Self { child })
    }

    /// 只终止这个结构体拥有的 child；调用方无需也不能提供 PID。
    pub(crate) fn stop(mut self) -> Result<(), ManagedSidecarError> {
        match self
            .child
            .try_wait()
            .map_err(|_| ManagedSidecarError::Stop)?
        {
            Some(_) => Ok(()),
            None => {
                self.child.kill().map_err(|_| ManagedSidecarError::Stop)?;
                self.child.wait().map_err(|_| ManagedSidecarError::Stop)?;
                Ok(())
            }
        }
    }
}

impl Drop for ManagedSidecar {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
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

        assert!(is_valid_secret(&secret));
        assert_eq!(secret.len(), 64);
    }

    #[test]
    fn injects_only_the_fixed_loopback_clash_api_contract() {
        let config = GeneratedConfig::from_bytes(
            br#"{"outbounds":[{"type":"direct","tag":"direct"}],"route":{"rules":[]}}"#.to_vec(),
        );
        let secret = "a".repeat(64);

        let configured = with_fixed_clash_api(&config, &secret).expect("fixed API injection");
        let value: Value = serde_json::from_slice(configured.as_bytes()).expect("config JSON");

        assert_eq!(
            value["experimental"]["clash_api"]["external_controller"],
            CLASH_API_ADDRESS
        );
        assert_eq!(value["experimental"]["clash_api"]["secret"], secret);
        assert!(value.get("inbounds").is_none());
        assert!(value.get("services").is_none());
        assert!(value.get("api").is_none());
    }

    #[test]
    fn rejects_non_compiler_json_and_invalid_secret_without_echoing_it() {
        let config = GeneratedConfig::from_bytes(
            br#"{"outbounds":[],"route":{},"inbounds":[{"type":"tun"}]}"#.to_vec(),
        );
        let bad_secret = "fixture-secret";

        let error = with_fixed_clash_api(&config, bad_secret)
            .expect_err("managed config must reject inbounds");

        assert_eq!(error, ManagedSidecarError::ConfigurationRejected);
        assert!(!error.to_string().contains(bad_secret));
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
