//! 节点组的唯一持久配置。成员以稳定出口 ID 保存，显示名只用于展示。
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupRule {
    Selector,
    UrlTest,
    Direct,
    Block,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupMode {
    Static,
    Dynamic,
}
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct NodeGroup {
    pub id: PoolId,
    pub name: String,
    pub rule: GroupRule,
    pub mode: GroupMode,
    pub enabled: bool,
    pub icon: String,
    pub icon_scale: i32,
    pub keywords: Vec<String>,
    pub members: Vec<OutboundId>,
    pub interval_secs: u64,
    pub tolerance_ms: u32,
    /// 空值继承 Profile.test_url，绝不读取 UI 测速偏好。
    pub test_url: String,
}
impl NodeGroup {
    pub fn fresh() -> Result<Self, AppError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| AppError::new(AppErrorCode::StorageFailed))?;
        Ok(Self::new(PoolId(format!(
            "group-{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))))
    }

    pub fn new(id: PoolId) -> Self {
        Self {
            id,
            name: String::new(),
            rule: GroupRule::UrlTest,
            mode: GroupMode::Static,
            enabled: true,
            icon: String::new(),
            icon_scale: 0,
            keywords: vec![],
            members: vec![],
            interval_secs: 300,
            tolerance_ms: 100,
            test_url: String::new(),
        }
    }
    pub fn builtin(&self) -> bool {
        matches!(self.rule, GroupRule::Direct | GroupRule::Block)
    }
    pub fn outbound_id(&self) -> OutboundId {
        match self.rule {
            GroupRule::Direct => OutboundId::Direct,
            GroupRule::Block => OutboundId::Block,
            _ => OutboundId::Pool(self.id.clone()),
        }
    }
}
/// 配置投影后的实际成员；Compiler 消费此值，不另存第二份成员事实。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeGroup {
    pub id: PoolId,
    pub members: Vec<OutboundId>,
    pub selection: SelectionPolicy,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GroupIssue {
    InvalidName(PoolId),
    DuplicateName(String),
    DuplicateId(PoolId),
    InvalidSettings(PoolId),
    DuplicateMember(PoolId),
    EmptyStatic(PoolId),
    BlockMember(PoolId),
    Graph(OutboundGraphError<OutboundId>),
    Unavailable { group: PoolId, member: OutboundId },
    Referenced(OutboundId),
}

pub fn default_groups() -> Vec<NodeGroup> {
    [
        ("builtin-direct", "直连", GroupRule::Direct, "misc:dart"),
        (
            "all-auto",
            "所有-自动",
            GroupRule::UrlTest,
            "globe:earth-asia",
        ),
        (
            "all-manual",
            "所有-手动",
            GroupRule::Selector,
            "globe:earth-meridians",
        ),
        ("builtin-block", "拒绝", GroupRule::Block, "misc:cross"),
    ]
    .into_iter()
    .map(|(id, name, rule, icon)| NodeGroup {
        name: name.into(),
        rule,
        icon: icon.into(),
        mode: GroupMode::Dynamic,
        ..NodeGroup::new(PoolId(id.into()))
    })
    .collect()
}

/// 对应 OpenBox normalizeNodeName：旗帜转换为独立国家码，避免 US 匹配 Russia。
fn normalized_name(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if ('\u{1f1e6}'..='\u{1f1ff}').contains(&c)
            && chars
                .peek()
                .is_some_and(|c| ('\u{1f1e6}'..='\u{1f1ff}').contains(c))
        {
            out.push(' ');
            out.push(char::from_u32(c as u32 - 0x1f1e6 + 97).unwrap());
            out.push(char::from_u32(chars.next().unwrap() as u32 - 0x1f1e6 + 97).unwrap());
            out.push(' ');
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}
pub fn group_keyword_matches(name: &str, keyword: &str) -> bool {
    let name = normalized_name(name);
    let normalized = normalized_name(keyword);
    let keyword = normalized.trim();
    if keyword.is_empty() {
        return false;
    }
    if (2..=3).contains(&keyword.len()) && keyword.bytes().all(|c| c.is_ascii_alphabetic()) {
        name.match_indices(keyword).any(|(i, _)| {
            let before = name[..i].chars().next_back();
            let after = &name[i + keyword.len()..];
            let cn2 = keyword == "cn"
                && after.starts_with('2')
                && !after.as_bytes().get(1).is_some_and(u8::is_ascii_digit);
            !before.is_some_and(|c| c.is_ascii_alphabetic())
                && !after
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
                && !cn2
        })
    } else {
        name.contains(keyword)
    }
}
impl AppState {
    /// 读取与保存共享同一派生结果，稳定 ID 供 UI 定位，不另存告警状态。
    pub fn dropped_groups(&self) -> Vec<PoolId> {
        self.groups
            .iter()
            .filter(|g| g.enabled && !g.builtin() && self.group_members(g).is_empty())
            .map(|g| g.id.clone())
            .collect()
    }
    /// 订阅现有隐式池停用后，该来源节点不再成为动态组候选。
    pub fn group_available_nodes(&self) -> impl Iterator<Item = &ProxyNode> {
        self.nodes.iter().filter(|node| {
            !self.pools.iter().any(|pool| {
                pool.kind == PoolKind::ImplicitProvider
                    && !pool.enabled
                    && pool
                        .sources
                        .iter()
                        .any(|source| source.provider_id == node.provider_id)
            })
        })
    }
    pub fn group_members(&self, group: &NodeGroup) -> Vec<OutboundId> {
        if group.builtin() {
            return vec![];
        }
        match group.mode {
            GroupMode::Static => group.members.clone(),
            GroupMode::Dynamic => self
                .group_available_nodes()
                .filter(|node| {
                    group.keywords.is_empty()
                        || group
                            .keywords
                            .iter()
                            .any(|k| group_keyword_matches(&node.name, k))
                })
                .map(|node| OutboundId::Node(node.id.clone()))
                .collect(),
        }
    }
    pub fn validate_groups(&self) -> Result<(), GroupIssue> {
        let mut ids = self
            .pools
            .iter()
            .map(|p| p.id.clone())
            .collect::<HashSet<_>>();
        let mut names = HashSet::new();
        let mut terminals = HashSet::new();
        for group in &self.groups {
            if group.id.0.trim().is_empty() || !ids.insert(group.id.clone()) {
                return Err(GroupIssue::DuplicateId(group.id.clone()));
            }
            if group.name.trim().is_empty() {
                return Err(GroupIssue::InvalidName(group.id.clone()));
            }
            if !names.insert(group.name.trim()) {
                return Err(GroupIssue::DuplicateName(group.name.clone()));
            }
            if group.builtin() && !terminals.insert(group.outbound_id()) {
                return Err(GroupIssue::DuplicateId(group.id.clone()));
            }
            if !group.builtin()
                && group.rule == GroupRule::UrlTest
                && (!(5..=86400).contains(&group.interval_secs)
                    || (!group.test_url.is_empty()
                        && GroupHealthUrl::optional(group.test_url.clone()).is_err()))
            {
                return Err(GroupIssue::InvalidSettings(group.id.clone()));
            }
            if !group.builtin() && group.mode == GroupMode::Static && group.members.is_empty() {
                return Err(GroupIssue::EmptyStatic(group.id.clone()));
            }
            if group.mode == GroupMode::Static
                && group.members.iter().collect::<HashSet<_>>().len() != group.members.len()
            {
                return Err(GroupIssue::DuplicateMember(group.id.clone()));
            }
            if group.mode == GroupMode::Static && group.members.contains(&OutboundId::Block) {
                return Err(GroupIssue::BlockMember(group.id.clone()));
            }
        }
        let catalog = OutboundCatalog::from_state(self);
        catalog.validate_graph().map_err(GroupIssue::Graph)?;
        // 空动态组允许保存，但保持 unavailable；有外部引用时拒绝，不猜测直连。
        for group in self.groups.iter().filter(|g| g.enabled && !g.builtin()) {
            for member in self.group_members(group) {
                if catalog.require_available(&member).is_err() {
                    return Err(GroupIssue::Unavailable {
                        group: group.id.clone(),
                        member,
                    });
                }
            }
        }
        for target in std::iter::once(&self.default_target)
            .chain(self.routes.iter().filter(|r| r.enabled).map(|r| &r.target))
            .chain(
                self.profile
                    .routing
                    .custom
                    .iter()
                    .filter(|r| r.enabled)
                    .flat_map(|r| r.rules.iter().map(|r| &r.target)),
            )
        {
            if let RouteTarget::Pool(id) = target
                && (self.groups.iter().any(|group| &group.id == id)
                    || !self.pools.iter().any(|pool| &pool.id == id))
                && catalog
                    .require_available(&OutboundId::Pool(id.clone()))
                    .is_err()
            {
                return Err(GroupIssue::Referenced(OutboundId::Pool(id.clone())));
            }
        }
        Ok(())
    }
    pub fn runtime_groups(&self) -> Vec<RuntimeGroup> {
        self.groups
            .iter()
            .filter(|g| g.enabled && !g.builtin())
            .filter_map(|g| {
                let members = self.group_members(g);
                if members.is_empty() {
                    return None;
                }
                Some(RuntimeGroup {
                    id: g.id.clone(),
                    members,
                    selection: match g.rule {
                        GroupRule::Selector => SelectionPolicy::Manual {
                            selected_node_id: None,
                            pending_node_id: None,
                        },
                        GroupRule::UrlTest => SelectionPolicy::UrlTest {
                            probe_url: if g.test_url.is_empty() {
                                self.profile.test_url.as_str().to_owned()
                            } else {
                                g.test_url.clone()
                            },
                            interval_secs: g.interval_secs,
                            tolerance_ms: g.tolerance_ms,
                        },
                        _ => unreachable!(),
                    },
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GroupCountry {
    pub code: String,
    pub name: String,
    pub keywords: Vec<String>,
}
pub fn group_countries() -> &'static [GroupCountry] {
    static COUNTRIES: std::sync::OnceLock<Vec<GroupCountry>> = std::sync::OnceLock::new();
    COUNTRIES.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../../src/openbox/assets/group-countries.json"
        ))
        .expect("OpenBox country catalog")
    })
}
/// 同国家已有组保持相邻，自动在手动之前；同名不重复生成。
pub fn auto_groups(
    existing: &[NodeGroup],
    codes: &[String],
    rules: &[GroupRule],
) -> Vec<NodeGroup> {
    let mut result = existing.to_vec();
    for code in codes {
        let Some(country) = group_countries().iter().find(|c| &c.code == code) else {
            continue;
        };
        let insertion = result
            .iter()
            .position(|g| g.icon.eq_ignore_ascii_case(code))
            .unwrap_or(result.len());
        let mut block = Vec::new();
        result.retain(|g| {
            if g.icon.eq_ignore_ascii_case(code) {
                block.push(g.clone());
                false
            } else {
                true
            }
        });
        for rule in rules
            .iter()
            .filter(|r| matches!(r, GroupRule::Selector | GroupRule::UrlTest))
        {
            let suffix = if *rule == GroupRule::UrlTest {
                "自动"
            } else {
                "手动"
            };
            let name = format!("{}-{suffix}", country.name);
            if existing.iter().any(|g| g.name == name) {
                continue;
            }
            let id = format!(
                "auto-{}-{}",
                code.to_lowercase(),
                if *rule == GroupRule::UrlTest {
                    "urltest"
                } else {
                    "selector"
                }
            );
            // 用户可能改过旧自动组名称，仍按稳定 ID 避免重复注册。
            if existing.iter().any(|g| g.id.0 == id) {
                continue;
            }
            block.push(NodeGroup {
                name,
                rule: *rule,
                mode: GroupMode::Dynamic,
                icon: code.clone(),
                keywords: country.keywords.clone(),
                ..NodeGroup::new(PoolId(id))
            });
        }
        block.sort_by_key(|g| if g.rule == GroupRule::UrlTest { 0 } else { 1 });
        result.splice(insertion..insertion, block);
    }
    result
}

#[cfg(test)]
mod tests;
