//! 用户指定的 OpenBox 当前配置与完整规则集快照；新安装和恢复默认共用，不联网。
use super::*;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

pub fn default_target_routing() -> ProfileRouting {
    serde_json::from_str(include_str!("../../assets/openbox-defaults/routing.json"))
        .expect("随包目标分流通过构建测试校验")
}
pub fn default_strategy_order() -> Vec<String> {
    serde_json::from_str(include_str!(
        "../../assets/openbox-defaults/display-order.json"
    ))
    .expect("随包显示排序")
}
pub fn default_outbound_groups() -> Vec<NodeGroup> {
    serde_json::from_str(include_str!("../../assets/openbox-defaults/groups.json"))
        .expect("随包出站配置通过构建测试校验")
}
/// 两条默认线路引用正式动态组，实际叶子来自本项目订阅；不内置用户凭据或原版节点名。
/// 无来源时保存停用结构，用户接入订阅后可启用；不虚构可用出口。
pub fn default_groups_for(state: &AppState) -> Vec<NodeGroup> {
    let mut groups = default_outbound_groups();
    if let Some(group) = groups.iter_mut().find(|g| g.rule == GroupRule::Failover) {
        group.enabled = state.group_available_nodes().next().is_some();
    }
    groups
}
pub fn new_builtin_state() -> Result<AppState, AppError> {
    let mut state = AppState::try_empty()?;
    state.groups = default_outbound_groups();
    state.profile.routing = default_target_routing();
    state.app_config.behavior.proxy_view.strategy_order = default_strategy_order();
    Ok(state)
}

/// 封闭的无头条件，仅承载原版规则集实际使用的五类数据，不接受任意配置透传。
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BuiltinHeadlessRule {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_suffix: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_keyword: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_regex: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_cidr: Option<Vec<String>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleSetSource {
    version: u32,
    rules: Vec<BuiltinHeadlessRule>,
}
/// 内容一次解析并保持原顺序；UI/验证只引用标签，保存时不复制十余万条规则。
pub fn rule_sets() -> &'static BTreeMap<&'static str, Vec<BuiltinHeadlessRule>> {
    static DATA: OnceLock<BTreeMap<&'static str, Vec<BuiltinHeadlessRule>>> = OnceLock::new();
    DATA.get_or_init(|| {
        [
            (
                "geoip-cloudflare",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-cloudflare.json"),
            ),
            (
                "geoip-cloudfront",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-cloudfront.json"),
            ),
            (
                "geoip-cn",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-cn.json"),
            ),
            (
                "geoip-facebook",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-facebook.json"),
            ),
            (
                "geoip-fastly",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-fastly.json"),
            ),
            (
                "geoip-google",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-google.json"),
            ),
            (
                "geoip-netflix",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-netflix.json"),
            ),
            (
                "geoip-private",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-private.json"),
            ),
            (
                "geoip-telegram",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-telegram.json"),
            ),
            (
                "geoip-tor",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-tor.json"),
            ),
            (
                "geoip-twitter",
                include_str!("../../assets/openbox-defaults/rulesets/geoip-twitter.json"),
            ),
            (
                "geosite-apple",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-apple.json"),
            ),
            (
                "geosite-category-ai-!cn",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-category-ai-!cn.json"),
            ),
            (
                "geosite-cn",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-cn.json"),
            ),
            (
                "geosite-gfw",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-gfw.json"),
            ),
            (
                "geosite-github",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-github.json"),
            ),
            (
                "geosite-google",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-google.json"),
            ),
            (
                "geosite-microsoft",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-microsoft.json"),
            ),
            (
                "geosite-netflix",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-netflix.json"),
            ),
            (
                "geosite-sony",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-sony.json"),
            ),
            (
                "geosite-steam",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-steam.json"),
            ),
            (
                "geosite-tiktok",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-tiktok.json"),
            ),
            (
                "geosite-tiktok@!cn",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-tiktok@!cn.json"),
            ),
            (
                "geosite-youtube",
                include_str!("../../assets/openbox-defaults/rulesets/geosite-youtube.json"),
            ),
        ]
        .into_iter()
        .map(|(tag, text)| {
            let source: RuleSetSource = serde_json::from_str(text).expect("已校验的随包规则集");
            assert_eq!(source.version, 3);
            (tag, source.rules)
        })
        .collect()
    })
}
pub fn rule_set(tag: &str) -> Option<&'static Vec<BuiltinHeadlessRule>> {
    rule_sets().get(tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    // 保护完整随包数据而非首100条截断，并验证未选默认意图不冒充Controller事实。
    #[test]
    fn policy87_full_rules_and_default_intent() {
        assert_eq!(rule_sets().len(), 24);
        let count = |rules: &Vec<BuiltinHeadlessRule>| {
            rules
                .iter()
                .map(|r| {
                    [
                        &r.domain,
                        &r.domain_suffix,
                        &r.domain_keyword,
                        &r.domain_regex,
                        &r.ip_cidr,
                    ]
                    .into_iter()
                    .flatten()
                    .map(Vec::len)
                    .sum::<usize>()
                })
                .sum::<usize>()
        };
        assert_eq!(rule_sets().values().map(count).sum::<usize>(), 139986);
        assert_eq!(count(rule_set("geosite-cn").unwrap()), 111224);
        assert!(
            rule_set("geosite-netflix")
                .unwrap()
                .iter()
                .any(|r| r.domain_regex.is_some())
        );
        assert!(!TrafficMatcher::RuleSet(vec!["not-bundled".into()]).is_valid());
        let state = new_builtin_state().unwrap();
        state.validate().unwrap();
        assert_eq!(state.profile.routing.selectors().count(), 13);
        assert_eq!(state.groups.len(), 5);
        assert!(state.group_selections.is_empty());
        let policy = state.profile.routing.fallback.as_ref().unwrap();
        assert_eq!(
            policy.runtime_selector(&state).members[0],
            OutboundId::Direct
        );
    }
}
