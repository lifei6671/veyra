use serde::Serialize;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum AppErrorCode {
    Validation,
    RevisionConflict,
    UnavailableOnPlatform,
    PermissionDenied,
    PortInUse,
    KernelUnavailable,
    Timeout,
    Cancelled,
    DownloadFailed,
    StorageFailed,
    ApplyFailed,
}

impl AppErrorCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::Validation => "输入无效，请检查标记的字段",
            Self::RevisionConflict => "数据已变更，请重新加载后保存",
            Self::UnavailableOnPlatform => "当前平台不支持此操作",
            Self::PermissionDenied => "没有执行此操作的权限",
            Self::PortInUse => "端口已被占用",
            Self::KernelUnavailable => "内核不可用",
            Self::Timeout => "操作超时",
            Self::Cancelled => "操作已取消",
            Self::DownloadFailed => "下载失败",
            Self::StorageFailed => "数据未能保存或读取",
            Self::ApplyFailed => "配置未能应用",
        }
    }
}

/// Closed paths prevent user-controlled keys or identifiers from becoming error output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum FieldPath {
    #[serde(rename = "app_config.behavior.latency.test_url")]
    LatencyUrl,
    #[serde(rename = "app_config.behavior.latency.timeout_ms")]
    LatencyTimeout,
    #[serde(rename = "app_config.behavior.latency.low_ms")]
    LatencyLow,
    #[serde(rename = "app_config.behavior.latency.medium_ms")]
    LatencyMedium,
    #[serde(rename = "app_config.behavior.proxy_view.group_columns")]
    ProxyColumns,
    #[serde(rename = "app_config.behavior.proxy_view.node_card_min_width")]
    ProxyNodeWidth,
    #[serde(rename = "app_config.behavior.proxy_view.strategy_order")]
    ProxyStrategyOrder,
    #[serde(rename = "app_config.behavior.test_sites")]
    TestSites,
    #[serde(rename = "app_config.behavior.test_sites.id")]
    TestSiteId,
    #[serde(rename = "app_config.behavior.test_sites.name")]
    TestSiteName,
    #[serde(rename = "app_config.behavior.test_sites.url")]
    TestSiteUrl,
    #[serde(rename = "app_config.behavior.test_sites.icon_key")]
    TestSiteIcon,
    #[serde(rename = "runtime.health_url")]
    RuntimeHealthUrl,
    #[serde(rename = "group.health_url")]
    GroupHealthUrl,
    #[serde(rename = "app_config.visual")]
    DesktopVisual,
    #[serde(rename = "profile.ipv6")]
    Ipv6,
    #[serde(rename = "profile.dns")]
    Dns,
    #[serde(rename = "profile.dns.split")]
    DnsSplit,
    #[serde(rename = "profile.dns.direct")]
    DnsDirect,
    #[serde(rename = "profile.dns.directPort")]
    DnsDirectPort,
    #[serde(rename = "profile.dns.directExtras")]
    DnsDirectExtras,
    #[serde(rename = "profile.dns.proxy")]
    DnsProxy,
    #[serde(rename = "profile.dns.proxyPort")]
    DnsProxyPort,
    #[serde(rename = "profile.dns.proxyExtras")]
    DnsProxyExtras,
    #[serde(rename = "profile.routing")]
    Routing,
    #[serde(rename = "profile.routing.custom")]
    RoutingCustom,
    #[serde(rename = "profile.routing.custom.name")]
    RoutingCustomName,
    #[serde(rename = "profile.routing.custom.enabled")]
    RoutingCustomEnabled,
    #[serde(rename = "profile.routing.custom.rules")]
    RoutingCustomRules,
    #[serde(rename = "profile.tun")]
    Tun,
    #[serde(rename = "profile.tun.autoRedirect")]
    TunAutoRedirect,
    #[serde(rename = "profile.tun.stack")]
    TunStack,
    #[serde(rename = "profile.tun.mtu")]
    TunMtu,
    #[serde(rename = "profile.tun.tcpMss")]
    TunTcpMss,
    #[serde(rename = "selection")]
    Selection,
    #[serde(rename = "snapshot")]
    Snapshot,
}

/// Only allowlisted diagnostic categories cross the UI/log boundary. Never attach sources,
/// filesystem paths, config bodies, credentials or raw HTTP/serde errors.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum ErrorDetail {
    Busy,
    Read,
    Write,
    Replace,
    InvalidSnapshot,
    RevisionExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AppError {
    code: AppErrorCode,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    field: Option<FieldPath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<ErrorDetail>,
}

impl AppError {
    pub fn new(code: AppErrorCode) -> Self {
        Self {
            code,
            message: code.message(),
            field: None,
            detail: None,
        }
    }
    pub fn validation(field: FieldPath) -> Self {
        Self {
            field: Some(field),
            ..Self::new(AppErrorCode::Validation)
        }
    }
    pub fn with_detail(mut self, detail: ErrorDetail) -> Self {
        self.detail = Some(detail);
        self
    }
    pub fn code(&self) -> AppErrorCode {
        self.code
    }
    pub fn field(&self) -> Option<FieldPath> {
        self.field
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}
impl std::error::Error for AppError {}
