//! 站点集 URL 导入的只读预览：不持有 Store，不执行 Runtime 配置或选择写入。
use super::network::RequestedPath;
use crate::subscription::{
    FetchClientOptions, FetchError,
    outbound::{OutboundClient, OutboundError, ProxySource, RoutePolicy},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Stdio, time::Duration};
use tokio::sync::watch;
mod mrs;
#[cfg(test)]
mod tests;

pub const MAX_IMPORT_ENTRIES: usize = 200;
const MAX_DECODED_BYTES: usize = 4 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleImportKind {
    Domain,
    DomainSuffix,
    DomainKeyword,
    IpCidr,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuleImportEntry {
    pub kind: RuleImportKind,
    pub value: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleImportPreview {
    pub entries: Vec<RuleImportEntry>,
    /// 全文件解析得到的明细总数；超过 200 时明确 truncated，UI 不允许确认明细导入。
    pub total: usize,
    pub truncated: bool,
}
pub struct RuleImportRequest {
    pub url: String,
    pub path: RequestedPath,
    pub timeout: Duration,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RuleImportError {
    #[error("rule import cancelled")]
    Cancelled,
    #[error("rule import timed out")]
    Timeout,
    #[error("invalid rule import URL")]
    InvalidUrl,
    #[error("unsupported rule file format")]
    UnsupportedFormat,
    #[error("invalid rule file")]
    InvalidFormat,
    #[error("rule cannot be represented by supported import entries")]
    UnsupportedRule,
    #[error("rule file exceeds resource size limit")]
    BodyTooLarge,
    #[error("running proxy unavailable")]
    ProxyUnavailable,
    #[error("rule import request failed: {0}")]
    Fetch(FetchError),
    #[error("fixed kernel unavailable")]
    KernelUnavailable,
    #[error("rule-set decompile failed")]
    DecompileFailed,
    #[error("owned import workspace IO failed")]
    Io,
}
pub struct RuleImportService {
    source: ProxySource,
    kernel: Option<PathBuf>,
}
impl RuleImportService {
    /// kernel 只由 composition root 注入；SRS 执行前核验既有固定 basename/digest。
    pub fn new(source: ProxySource, fixed_kernel: Option<PathBuf>) -> Self {
        Self {
            source,
            kernel: fixed_kernel,
        }
    }
    pub async fn preview(
        &self,
        request: RuleImportRequest,
        mut cancel: watch::Receiver<bool>,
    ) -> Result<RuleImportPreview, RuleImportError> {
        if *cancel.borrow() {
            return Err(RuleImportError::Cancelled);
        }
        let url = reqwest::Url::parse(&request.url).map_err(|_| RuleImportError::InvalidUrl)?;
        let extension = url
            .path()
            .rsplit('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !matches!(extension.as_str(), "list" | "mrs" | "srs") {
            return Err(RuleImportError::UnsupportedFormat);
        }
        let route = match request.path {
            RequestedPath::Direct => RoutePolicy::Direct,
            RequestedPath::ViaRunningProxy => RoutePolicy::ViaRunningProxy(
                self.source
                    .current()
                    .filter(|p| p.is_valid())
                    .ok_or(RuleImportError::ProxyUnavailable)?,
            ),
        };
        let client = OutboundClient::new(
            route,
            &FetchClientOptions {
                timeout: request.timeout,
                ..Default::default()
            },
        )
        .map_err(map_outbound)?;
        let deadline = tokio::time::Instant::now() + request.timeout;
        let bytes = tokio::select! { biased;
            _ = cancel.wait_for(|v|*v) => return Err(RuleImportError::Cancelled),
            result = tokio::time::timeout_at(deadline,client.resource_bytes(&request.url)) => result.map_err(|_|RuleImportError::Timeout)?.map_err(map_outbound)?,
        };
        let entries = match extension.as_str() {
            "list" => parse_list(&bytes)?,
            "mrs" => mrs::parse(&bytes)?,
            "srs" => self.decompile(&bytes, deadline, &mut cancel).await?,
            _ => unreachable!(),
        };
        if *cancel.borrow() {
            return Err(RuleImportError::Cancelled);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(RuleImportError::Timeout);
        }
        if entries.is_empty() {
            return Err(RuleImportError::InvalidFormat);
        }
        let total = entries.len();
        Ok(RuleImportPreview {
            entries: entries.into_iter().take(MAX_IMPORT_ENTRIES).collect(),
            total,
            truncated: total > MAX_IMPORT_ENTRIES,
        })
    }
    async fn decompile(
        &self,
        bytes: &[u8],
        deadline: tokio::time::Instant,
        cancel: &mut watch::Receiver<bool>,
    ) -> Result<Vec<RuleImportEntry>, RuleImportError> {
        use super::runtime_recovery::{KERNEL_DIGEST, KERNEL_EXECUTABLE};
        let kernel = self
            .kernel
            .as_ref()
            .ok_or(RuleImportError::KernelUnavailable)?
            .canonicalize()
            .map_err(|_| RuleImportError::KernelUnavailable)?;
        if kernel.file_name() != Some(std::ffi::OsStr::new(KERNEL_EXECUTABLE))
            || format!(
                "{:x}",
                Sha256::digest(
                    std::fs::read(&kernel).map_err(|_| RuleImportError::KernelUnavailable)?
                )
            ) != KERNEL_DIGEST
        {
            return Err(RuleImportError::KernelUnavailable);
        }
        let workspace = OwnedWorkspace::new()?;
        let source = workspace.0.join("source.srs");
        let output = workspace.0.join("preview.json");
        std::fs::write(&source, bytes).map_err(|_| RuleImportError::Io)?;
        // 参数固定，URL 不进入 shell/命令行；不启动 sing-box run 或 Controller。
        let mut child = tokio::process::Command::new(kernel)
            .args(["rule-set", "decompile"])
            .arg(&source)
            .arg("--output")
            .arg(&output)
            .current_dir(&workspace.0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| RuleImportError::DecompileFailed)?;
        let result = tokio::select! { biased;
            _ = cancel.wait_for(|v|*v) => Err(RuleImportError::Cancelled),
            status=tokio::time::timeout_at(deadline,child.wait()) => match status { Ok(Ok(s)) if s.success()=>Ok(()),Ok(_)=>Err(RuleImportError::DecompileFailed),Err(_)=>Err(RuleImportError::Timeout)},
        };
        if let Err(error) = result {
            // 仅终止本函数持有的 Child，确认 wait 后才清理本次目录。
            if child.try_wait().map_err(|_| RuleImportError::Io)?.is_none() {
                child.kill().await.map_err(|_| RuleImportError::Io)?;
            }
            return Err(error);
        }
        let mut file = std::fs::File::open(output).map_err(|_| RuleImportError::Io)?;
        use std::io::Read;
        let mut decoded = Vec::new();
        (&mut file)
            .take(MAX_DECODED_BYTES as u64 + 1)
            .read_to_end(&mut decoded)
            .map_err(|_| RuleImportError::Io)?;
        if decoded.len() > MAX_DECODED_BYTES {
            return Err(RuleImportError::BodyTooLarge);
        }
        parse_srs_json(&decoded)
    }
}
fn map_outbound(error: OutboundError) -> RuleImportError {
    match error {
        OutboundError::Fetch(FetchError::Timeout) => RuleImportError::Timeout,
        OutboundError::Fetch(FetchError::BodyTooLarge) => RuleImportError::BodyTooLarge,
        OutboundError::Fetch(FetchError::InvalidUrl) => RuleImportError::InvalidUrl,
        OutboundError::Fetch(e) => RuleImportError::Fetch(e),
        _ => RuleImportError::ProxyUnavailable,
    }
}
struct OwnedWorkspace(PathBuf);
impl OwnedWorkspace {
    fn new() -> Result<Self, RuleImportError> {
        let mut identity = [0u8; 16];
        getrandom::fill(&mut identity).map_err(|_| RuleImportError::Io)?;
        let path = std::env::temp_dir().join(format!(
            "veyra-rule-import-{}",
            identity
                .iter()
                .map(|v| format!("{v:02x}"))
                .collect::<String>()
        ));
        std::fs::create_dir(&path).map_err(|_| RuleImportError::Io)?;
        Ok(Self(path))
    }
}
impl Drop for OwnedWorkspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn entry(kind: RuleImportKind, value: &str) -> Result<RuleImportEntry, RuleImportError> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Err(RuleImportError::InvalidFormat);
    }
    if kind == RuleImportKind::IpCidr {
        let (address, prefix) = value
            .split_once('/')
            .ok_or(RuleImportError::InvalidFormat)?;
        let ip: std::net::IpAddr = address
            .parse()
            .map_err(|_| RuleImportError::InvalidFormat)?;
        let prefix: u8 = prefix.parse().map_err(|_| RuleImportError::InvalidFormat)?;
        if prefix > if ip.is_ipv4() { 32 } else { 128 } {
            return Err(RuleImportError::InvalidFormat);
        }
    }
    Ok(RuleImportEntry {
        kind,
        value: value.into(),
    })
}
fn parse_list(bytes: &[u8]) -> Result<Vec<RuleImportEntry>, RuleImportError> {
    let text = std::str::from_utf8(bytes).map_err(|_| RuleImportError::InvalidFormat)?;
    let mut result = Vec::new();
    for line in text.trim_start_matches('\u{feff}').lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(['#', ';']) || line.starts_with("//") {
            continue;
        }
        let fields = line.split(',').map(str::trim).collect::<Vec<_>>();
        if fields.len() < 2 {
            return Err(RuleImportError::InvalidFormat);
        }
        let kind = match fields[0] {
            "DOMAIN" => RuleImportKind::Domain,
            "DOMAIN-SUFFIX" => RuleImportKind::DomainSuffix,
            "DOMAIN-KEYWORD" => RuleImportKind::DomainKeyword,
            "IP-CIDR" | "IP-CIDR6" => RuleImportKind::IpCidr,
            _ => return Err(RuleImportError::UnsupportedRule),
        };
        // no-resolve 是列表格式的附注；其它尾部（含策略/选项）不能被静默丢弃。
        if fields.len() > 3 || (fields.len() == 3 && fields[2] != "no-resolve") {
            return Err(RuleImportError::UnsupportedRule);
        }
        result.push(entry(kind, fields[1])?);
    }
    Ok(result)
}
fn parse_srs_json(bytes: &[u8]) -> Result<Vec<RuleImportEntry>, RuleImportError> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| RuleImportError::InvalidFormat)?;
    let rules = value["rules"]
        .as_array()
        .ok_or(RuleImportError::InvalidFormat)?;
    let mut entries = Vec::new();
    for rule in rules {
        parse_headless(rule, &mut entries)?;
    }
    Ok(entries)
}
fn parse_headless(
    rule: &serde_json::Value,
    entries: &mut Vec<RuleImportEntry>,
) -> Result<(), RuleImportError> {
    let object = rule.as_object().ok_or(RuleImportError::InvalidFormat)?;
    if rule["invert"].as_bool().unwrap_or(false) {
        return Err(RuleImportError::UnsupportedRule);
    }
    if rule["type"] == "logical" {
        if rule["mode"] != "or"
            || object
                .keys()
                .any(|k| !matches!(k.as_str(), "type" | "mode" | "rules" | "invert"))
        {
            return Err(RuleImportError::UnsupportedRule);
        }
        for inner in rule["rules"]
            .as_array()
            .ok_or(RuleImportError::InvalidFormat)?
        {
            parse_headless(inner, entries)?;
        }
        return Ok(());
    }
    let fields = object
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "type" | "invert"))
        .collect::<Vec<_>>();
    // 多字段 headless rule 是 AND，拆成多个 OR 会改变路由，必须明确拒绝。
    if fields.len() != 1 {
        return Err(RuleImportError::UnsupportedRule);
    }
    let (name, values) = fields[0];
    let kind = match name.as_str() {
        "domain" => RuleImportKind::Domain,
        "domain_suffix" => RuleImportKind::DomainSuffix,
        "domain_keyword" => RuleImportKind::DomainKeyword,
        "ip_cidr" => RuleImportKind::IpCidr,
        _ => return Err(RuleImportError::UnsupportedRule),
    };
    if let Some(value) = values.as_str() {
        entries.push(entry(kind, value)?);
    } else {
        for value in values.as_array().ok_or(RuleImportError::InvalidFormat)? {
            entries.push(entry(
                kind,
                value.as_str().ok_or(RuleImportError::InvalidFormat)?,
            )?);
        }
    }
    Ok(())
}
