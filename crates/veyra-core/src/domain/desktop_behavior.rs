//! Local UI behavior preferences. No Runtime or Group configuration lives here.
use super::{AppError, FieldPath};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum IpInfoProvider {
    #[default]
    #[serde(rename = "ip.sb")]
    IpSb,
    #[serde(rename = "ipwho.is")]
    IpWhoIs,
    #[serde(rename = "ipapi.is")]
    IpApiIs,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeSort {
    Default,
    NameAsc,
    NameDesc,
    #[default]
    LatencyAsc,
    LatencyDesc,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct UiLatencyUrl(pub String);
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct RuntimeHealthUrl(String);
// 持久化和 patch 读入同一校验边界，不能通过 serde 绕过 URL 类型。
impl<'de> Deserialize<'de> for RuntimeHealthUrl {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GroupHealthUrl(String);
impl RuntimeHealthUrl {
    pub fn new(value: String) -> Result<Self, AppError> {
        validate_http_url(&value, FieldPath::RuntimeHealthUrl)?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl GroupHealthUrl {
    /// Empty Group input means inheritance, never a UI latency preference fallback.
    pub fn optional(value: String) -> Result<Option<Self>, AppError> {
        if value.trim().is_empty() {
            return Ok(None);
        }
        validate_http_url(&value, FieldPath::GroupHealthUrl)?;
        Ok(Some(Self(value)))
    }
}
/// P2/P4 own actual Profile/Group persistence and Runtime consumption.
pub fn resolve_group_health_url<'a>(
    group: Option<&'a GroupHealthUrl>,
    runtime: &'a RuntimeHealthUrl,
) -> &'a str {
    group.map_or(runtime.as_str(), |value| value.0.as_str())
}
fn validate_http_url(value: &str, field: FieldPath) -> Result<(), AppError> {
    let parsed = reqwest::Url::parse(value).map_err(|_| AppError::validation(field))?;
    if value.len() > 2048
        || value.trim() != value
        || !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || value.split_once("://").is_some_and(|(_, tail)| {
            tail.split(['/', '?', '#'])
                .next()
                .is_some_and(|authority| authority.contains('@'))
        })
    {
        return Err(AppError::validation(field));
    }
    Ok(())
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UiLatencyPreferences {
    pub test_url: UiLatencyUrl,
    pub timeout_ms: u32,
    pub low_ms: u32,
    pub medium_ms: u32,
}
impl Default for UiLatencyPreferences {
    fn default() -> Self {
        Self {
            test_url: UiLatencyUrl("http://www.gstatic.com/generate_204".into()),
            timeout_ms: 5000,
            low_ms: 400,
            medium_ms: 800,
        }
    }
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProxyViewPreferences {
    pub group_columns: u8,
    pub hide_unavailable: bool,
    pub node_sort: NodeSort,
    pub group_by_provider: bool,
    pub node_card_min_width: u16,
    /// Display IDs only; never Group membership or domain ordering.
    pub strategy_order: Vec<String>,
}
impl Default for ProxyViewPreferences {
    fn default() -> Self {
        Self {
            group_columns: 2,
            hide_unavailable: false,
            node_sort: NodeSort::LatencyAsc,
            group_by_provider: true,
            node_card_min_width: 145,
            strategy_order: vec![],
        }
    }
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UiDiagnosticsPreferences {
    pub ipv6_test: bool,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestSite {
    pub id: String,
    pub name: String,
    pub url: String,
    pub icon_key: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DesktopBehaviorPreferences {
    pub latency: UiLatencyPreferences,
    pub proxy_view: ProxyViewPreferences,
    pub diagnostics: UiDiagnosticsPreferences,
    pub ip_info: IpInfoProvider,
    pub test_sites: Vec<TestSite>,
}
impl Default for DesktopBehaviorPreferences {
    fn default() -> Self {
        Self {
            latency: Default::default(),
            proxy_view: Default::default(),
            diagnostics: Default::default(),
            ip_info: Default::default(),
            test_sites: [
                (
                    "baidu",
                    "百度",
                    "https://www.baidu.com/favicon.ico",
                    "brand:baidu",
                ),
                (
                    "google",
                    "Google",
                    "https://www.google.com/generate_204",
                    "brand:google",
                ),
                (
                    "openai",
                    "OpenAI",
                    "https://api.openai.com/v1/models",
                    "brand:openai-light",
                ),
                (
                    "telegram",
                    "Telegram",
                    "https://telegram.org/favicon.ico",
                    "brand:telegram",
                ),
            ]
            .into_iter()
            .map(|(id, name, url, icon_key)| TestSite {
                id: id.into(),
                name: name.into(),
                url: url.into(),
                icon_key: icon_key.into(),
            })
            .collect(),
        }
    }
}
fn bounded_text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
impl DesktopBehaviorPreferences {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_http_url(&self.latency.test_url.0, FieldPath::LatencyUrl)?;
        if self.latency.timeout_ms == 0 {
            return Err(AppError::validation(FieldPath::LatencyTimeout));
        }
        if self.latency.medium_ms < self.latency.low_ms {
            return Err(AppError::validation(FieldPath::LatencyMedium));
        }
        if !(1..=3).contains(&self.proxy_view.group_columns) {
            return Err(AppError::validation(FieldPath::ProxyColumns));
        }
        if !(100..=320).contains(&self.proxy_view.node_card_min_width) {
            return Err(AppError::validation(FieldPath::ProxyNodeWidth));
        }
        if self.proxy_view.strategy_order.len() > 128
            || self
                .proxy_view
                .strategy_order
                .iter()
                .any(|id| !bounded_text(id, 128))
        {
            return Err(AppError::validation(FieldPath::ProxyStrategyOrder));
        }
        if self.test_sites.is_empty() || self.test_sites.len() > 32 {
            return Err(AppError::validation(FieldPath::TestSites));
        }
        let mut ids = HashSet::new();
        for site in &self.test_sites {
            if !bounded_text(&site.id, 64) || !ids.insert(&site.id) {
                return Err(AppError::validation(FieldPath::TestSiteId));
            }
            if !bounded_text(&site.name, 128) {
                return Err(AppError::validation(FieldPath::TestSiteName));
            }
            if !bounded_text(&site.icon_key, 64) {
                return Err(AppError::validation(FieldPath::TestSiteIcon));
            }
            validate_http_url(&site.url, FieldPath::TestSiteUrl)?;
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum BehaviorField {
    LatencyUrl,
    LatencyTimeout,
    LatencyLow,
    LatencyMedium,
    ProxyColumns,
    HideUnavailable,
    NodeSort,
    GroupByProvider,
    NodeWidth,
    StrategyOrder,
    Ipv6Test,
    IpInfo,
    TestSites,
}
/// None = unchanged. Lists, when present, replace the complete preference list.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DesktopBehaviorPreferencesPatch {
    pub test_url: Option<UiLatencyUrl>,
    pub timeout_ms: Option<u32>,
    pub low_ms: Option<u32>,
    pub medium_ms: Option<u32>,
    pub group_columns: Option<u8>,
    pub hide_unavailable: Option<bool>,
    pub node_sort: Option<NodeSort>,
    pub group_by_provider: Option<bool>,
    pub node_card_min_width: Option<u16>,
    pub strategy_order: Option<Vec<String>>,
    pub ipv6_test: Option<bool>,
    pub ip_info: Option<IpInfoProvider>,
    pub test_sites: Option<Vec<TestSite>>,
}
// The same explicit field list drives application, dirty tracking and changed-field notification.
macro_rules! behavior_fields {
    ($op:ident, $s:expr, $p:expr) => {
        $op!($s, $p, test_url, latency.test_url, LatencyUrl);
        $op!($s, $p, timeout_ms, latency.timeout_ms, LatencyTimeout);
        $op!($s, $p, low_ms, latency.low_ms, LatencyLow);
        $op!($s, $p, medium_ms, latency.medium_ms, LatencyMedium);
        $op!(
            $s,
            $p,
            group_columns,
            proxy_view.group_columns,
            ProxyColumns
        );
        $op!(
            $s,
            $p,
            hide_unavailable,
            proxy_view.hide_unavailable,
            HideUnavailable
        );
        $op!($s, $p, node_sort, proxy_view.node_sort, NodeSort);
        $op!(
            $s,
            $p,
            group_by_provider,
            proxy_view.group_by_provider,
            GroupByProvider
        );
        $op!(
            $s,
            $p,
            node_card_min_width,
            proxy_view.node_card_min_width,
            NodeWidth
        );
        $op!(
            $s,
            $p,
            strategy_order,
            proxy_view.strategy_order,
            StrategyOrder
        );
        $op!($s, $p, ipv6_test, diagnostics.ipv6_test, Ipv6Test);
        $op!($s, $p, ip_info, ip_info, IpInfo);
        $op!($s, $p, test_sites, test_sites, TestSites);
    };
}
impl DesktopBehaviorPreferencesPatch {
    pub fn apply(&self, saved: &DesktopBehaviorPreferences) -> DesktopBehaviorPreferences {
        let mut next = saved.clone();
        macro_rules! assign { ($s:expr,$p:expr,$field:ident,$($path:ident).+,$topic:ident) => { if let Some(value) = &$p.$field { $s.$($path).+ = value.clone(); } }; }
        behavior_fields!(assign, next, self);
        let mut ids = HashSet::new();
        next.proxy_view
            .strategy_order
            .retain(|id| ids.insert(id.clone()));
        next
    }
    pub fn between(
        before: &DesktopBehaviorPreferences,
        after: &DesktopBehaviorPreferences,
    ) -> Self {
        let mut patch = Self::default();
        macro_rules! diff { ($s:expr,$p:expr,$field:ident,$($path:ident).+,$topic:ident) => { if before.$($path).+ != after.$($path).+ { $p.$field = Some(after.$($path).+.clone()); } }; }
        behavior_fields!(diff, before, patch);
        patch
    }
    pub fn changed_fields(&self) -> Vec<BehaviorField> {
        let mut fields = Vec::new();
        macro_rules! topic {
            ($s:expr,$p:expr,$field:ident,$($path:ident).+,$topic:ident) => {
                if $p.$field.is_some() {
                    fields.push(BehaviorField::$topic);
                }
            };
        }
        behavior_fields!(topic, self, self);
        fields
    }
    /// Acknowledge only fields still equal to the submitted value; newer edits stay dirty.
    pub fn acknowledge(&mut self, submitted: &Self) {
        macro_rules! ack {
            ($s:expr,$p:expr,$field:ident,$($path:ident).+,$topic:ident) => {
                if $s.$field == $p.$field {
                    $s.$field = None;
                }
            };
        }
        behavior_fields!(ack, self, submitted);
    }
    pub fn merge(&mut self, edit: &Self) {
        macro_rules! merge {
            ($s:expr,$p:expr,$field:ident,$($path:ident).+,$topic:ident) => {
                if $p.$field.is_some() {
                    $s.$field = $p.$field.clone();
                }
            };
        }
        behavior_fields!(merge, self, edit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_defaults_and_round_trip() {
        let p = DesktopBehaviorPreferences::default();
        assert_eq!(
            (p.latency.low_ms, p.latency.medium_ms, p.latency.timeout_ms),
            (400, 800, 5000)
        );
        assert_eq!(p.latency.test_url.0, "http://www.gstatic.com/generate_204");
        assert_eq!(
            p.proxy_view,
            ProxyViewPreferences {
                group_columns: 2,
                hide_unavailable: false,
                node_sort: NodeSort::LatencyAsc,
                group_by_provider: true,
                node_card_min_width: 145,
                strategy_order: vec![]
            }
        );
        assert!(!p.diagnostics.ipv6_test);
        assert_eq!(p.ip_info, IpInfoProvider::IpSb);
        assert_eq!(
            p.test_sites
                .iter()
                .map(|s| s.id.as_str())
                .collect::<Vec<_>>(),
            ["baidu", "google", "openai", "telegram"]
        );
        assert!(p.validate().is_ok());
        assert_eq!(
            serde_json::from_value::<DesktopBehaviorPreferences>(serde_json::to_value(&p).unwrap())
                .unwrap(),
            p
        );
    }
    #[test]
    fn unsafe_urls_rejected_with_exact_field() {
        for url in [
            "",
            "file:///tmp/a",
            "ftp://example.invalid",
            "https://u:p@example.invalid",
            "https://@example.invalid",
            "https://example.invalid/\n",
        ] {
            let p = DesktopBehaviorPreferencesPatch {
                test_url: Some(UiLatencyUrl(url.into())),
                ..Default::default()
            }
            .apply(&Default::default());
            assert_eq!(
                p.validate().unwrap_err().field(),
                Some(FieldPath::LatencyUrl)
            );
        }
    }
    #[test]
    fn numeric_bounds_and_threshold_relation() {
        for (patch, field) in [
            (
                DesktopBehaviorPreferencesPatch {
                    timeout_ms: Some(0),
                    ..Default::default()
                },
                FieldPath::LatencyTimeout,
            ),
            (
                DesktopBehaviorPreferencesPatch {
                    low_ms: Some(801),
                    ..Default::default()
                },
                FieldPath::LatencyMedium,
            ),
            (
                DesktopBehaviorPreferencesPatch {
                    group_columns: Some(4),
                    ..Default::default()
                },
                FieldPath::ProxyColumns,
            ),
            (
                DesktopBehaviorPreferencesPatch {
                    node_card_min_width: Some(99),
                    ..Default::default()
                },
                FieldPath::ProxyNodeWidth,
            ),
            (
                DesktopBehaviorPreferencesPatch {
                    node_card_min_width: Some(321),
                    ..Default::default()
                },
                FieldPath::ProxyNodeWidth,
            ),
        ] {
            assert_eq!(
                patch
                    .apply(&Default::default())
                    .validate()
                    .unwrap_err()
                    .field(),
                Some(field)
            );
        }
        assert!(
            DesktopBehaviorPreferencesPatch {
                low_ms: Some(0),
                medium_ms: Some(0),
                timeout_ms: Some(u32::MAX),
                ..Default::default()
            }
            .apply(&Default::default())
            .validate()
            .is_ok()
        );
    }
    #[test]
    fn closed_enums_reject_unknown() {
        assert!(serde_json::from_str::<NodeSort>("\"fast\"").is_err());
        assert!(serde_json::from_str::<IpInfoProvider>("\"other\"").is_err());
    }
    #[test]
    fn sites_and_strategy_bounds_and_deduplication() {
        let mut p = DesktopBehaviorPreferences::default();
        p.test_sites.push(p.test_sites[0].clone());
        assert_eq!(
            p.validate().unwrap_err().field(),
            Some(FieldPath::TestSiteId)
        );
        p.test_sites.pop();
        p.test_sites[0].url = "javascript:a".into();
        assert_eq!(
            p.validate().unwrap_err().field(),
            Some(FieldPath::TestSiteUrl)
        );
        p.test_sites[0].url = "https://example.invalid".into();
        p.test_sites[0].name = "x".repeat(129);
        assert_eq!(
            p.validate().unwrap_err().field(),
            Some(FieldPath::TestSiteName)
        );
        p = Default::default();
        p.test_sites[0].icon_key = "x".repeat(65);
        assert_eq!(
            p.validate().unwrap_err().field(),
            Some(FieldPath::TestSiteIcon)
        );
        p = Default::default();
        p.test_sites = (0..36).map(|_| p.test_sites[0].clone()).collect();
        assert_eq!(
            p.validate().unwrap_err().field(),
            Some(FieldPath::TestSites)
        );
        let p = DesktopBehaviorPreferencesPatch {
            strategy_order: Some(vec!["a".into(), "a".into(), "b".into()]),
            ..Default::default()
        }
        .apply(&Default::default());
        assert_eq!(p.proxy_view.strategy_order, ["a", "b"]);
        assert!(
            DesktopBehaviorPreferencesPatch {
                strategy_order: Some((0..129).map(|i| i.to_string()).collect()),
                ..Default::default()
            }
            .apply(&Default::default())
            .validate()
            .is_err()
        );
    }
    #[test]
    fn partial_patch_preserves_unedited_fields() {
        let before = DesktopBehaviorPreferences::default();
        let after = DesktopBehaviorPreferencesPatch {
            timeout_ms: Some(7000),
            ..Default::default()
        }
        .apply(&before);
        let mut expected = before;
        expected.latency.timeout_ms = 7000;
        assert_eq!(after, expected);
    }
    #[test]
    fn ui_runtime_and_group_url_roles_are_separate() {
        let ui = UiLatencyUrl("https://ui.example.invalid".into());
        let runtime = RuntimeHealthUrl::new("https://runtime.example.invalid".into()).unwrap();
        let group = GroupHealthUrl::optional("https://group.example.invalid".into()).unwrap();
        assert_eq!(
            resolve_group_health_url(group.as_ref(), &runtime),
            "https://group.example.invalid"
        );
        assert_eq!(
            resolve_group_health_url(
                GroupHealthUrl::optional(" ".into()).unwrap().as_ref(),
                &runtime
            ),
            "https://runtime.example.invalid"
        );
        assert_ne!(resolve_group_health_url(None, &runtime), ui.0);
    }
}
