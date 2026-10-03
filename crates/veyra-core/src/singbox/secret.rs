//! Runtime API secret shared by the compiler and platform adapter.

use super::GeneratedConfig;
use getrandom::fill;
use serde_json::Value;
use thiserror::Error;

const SECRET_BYTES: usize = 32;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ManagedSidecarError {
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
pub struct ApiSecret {
    value: String,
}

impl ApiSecret {
    pub fn as_str(&self) -> &str {
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
pub fn generate_api_secret() -> Result<ApiSecret, ManagedSidecarError> {
    let mut bytes = [0_u8; SECRET_BYTES];
    fill(&mut bytes).map_err(|_| ManagedSidecarError::SecretGeneration)?;

    let mut secret = String::with_capacity(SECRET_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut secret, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(ApiSecret { value: secret })
}

/// 从已通过最终白名单的配置提取认证身份；不再注入或重新序列化配置。
pub fn api_secret_from_config(
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

#[cfg(any(test, feature = "legacy-test-support"))]
pub fn test_api_secret() -> ApiSecret {
    ApiSecret {
        value: "a".repeat(64),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secret_is_fixed_length_lowercase_hex() {
        let secret = generate_api_secret().expect("system entropy");

        assert!(
            secret
                .as_str()
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        );
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
}
