//! 目标分流策略的唯一持久事实；Selector 是此策略的派生出口，不是节点组副本。
use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "TargetRoutingPolicyInput")]
pub struct TargetRoutingPolicy {
    pub id: PoolId,
    pub name: String,
    pub icon: String,
    pub icon_scale: i32,
    pub enabled: bool,
    /// 同一策略的匹配条件按 OR 分别编译；策略数组顺序才是命中优先级。
    pub matchers: Vec<TrafficMatcher>,
    /// 配置默认意图，不冒充 Controller 当前选择；用户正式选择仍优先。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_outbound: Option<OutboundId>,
}
// 兼容本轮早期快照的人为 members 字段；只读后丢弃，不形成第二份出口事实。
// 其它未知字段继续拒绝，后续保存不再写出成员缓存。
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TargetRoutingPolicyInput {
    id: PoolId,
    name: String,
    icon: String,
    icon_scale: i32,
    enabled: bool,
    matchers: Vec<TrafficMatcher>,
    #[serde(default)]
    default_outbound: Option<OutboundId>,
    #[serde(default, rename = "members")]
    _members: serde::de::IgnoredAny,
}
impl From<TargetRoutingPolicyInput> for TargetRoutingPolicy {
    fn from(input: TargetRoutingPolicyInput) -> Self {
        Self {
            id: input.id,
            name: input.name,
            icon: input.icon,
            icon_scale: input.icon_scale,
            enabled: input.enabled,
            matchers: input.matchers,
            default_outbound: input.default_outbound,
        }
    }
}
impl TargetRoutingPolicy {
    pub fn fresh() -> Result<Self, AppError> {
        let mut bytes = [0u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| AppError::new(AppErrorCode::StorageFailed))?;
        Ok(Self::new(PoolId(format!(
            "routing-{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ))))
    }
    pub fn new(id: PoolId) -> Self {
        Self {
            id,
            name: String::new(),
            icon: String::new(),
            icon_scale: 0,
            enabled: true,
            matchers: vec![],
            default_outbound: None,
        }
    }
    pub fn runtime_selector(&self, state: &AppState) -> RuntimeGroup {
        let mut members = state.target_routing_members();
        // 原编译/恢复契约以首成员作为未选择时的默认出口，保持同一份有序成员。
        if let Some(default) = &self.default_outbound
            && let Some(index) = members.iter().position(|member| member == default)
        {
            let default = members.remove(index);
            members.insert(0, default);
        }
        RuntimeGroup {
            id: self.id.clone(),
            members,
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
                pending_node_id: None,
            },
        }
    }
}
impl ProfileRouting {
    /// 包含停用策略用于编辑和引用校验；不把停用策略冒充已应用出口。
    pub fn selectors(&self) -> impl Iterator<Item = &TargetRoutingPolicy> {
        self.policies.iter().chain(self.fallback.iter())
    }
    pub fn selector(&self, id: &PoolId) -> Option<&TargetRoutingPolicy> {
        self.selectors().find(|p| &p.id == id)
    }
}
impl AppState {
    /// 策略可选项随正式出站配置变化；保留配置顺序，不暴露内部 lane、原始节点或策略自身。
    /// 旧自定义池仍属于已配置出站；空组与停用出站不进入 Selector/Compiler。
    pub fn target_routing_members(&self) -> Vec<OutboundId> {
        let mut members = Vec::new();
        if !self.groups.iter().any(|g| g.rule == GroupRule::Direct) {
            members.push(OutboundId::Direct);
        }
        members.extend(
            self.pools
                .iter()
                .filter(|p| {
                    p.enabled
                        && p.kind == PoolKind::Custom
                        && !self.resolve_pool_members(p).is_empty()
                })
                .map(|p| OutboundId::Pool(p.id.clone())),
        );
        members.extend(
            self.groups
                .iter()
                .filter(|g| g.enabled && (g.builtin() || !self.group_members(g).is_empty()))
                .map(NodeGroup::outbound_id),
        );
        if !self.groups.iter().any(|g| g.rule == GroupRule::Block) {
            members.push(OutboundId::Block);
        }
        members
    }
    pub(crate) fn validate_target_routing(&self) -> Result<(), StateValidationError> {
        // 无新策略的旧快照沿用旧验证路径与错误语义。
        if self.profile.routing.selectors().next().is_none() {
            return Ok(());
        }
        let mut ids = self
            .pools
            .iter()
            .map(|p| p.id.clone())
            .chain(self.groups.iter().map(|g| g.id.clone()))
            .chain(
                self.groups
                    .iter()
                    .flat_map(|g| g.lanes.iter().map(|l| l.pool_id(&g.id))),
            )
            .collect::<HashSet<_>>();
        let mut names = HashSet::new();
        let catalog = OutboundCatalog::from_state(self);
        for policy in self.profile.routing.selectors() {
            let fallback = self
                .profile
                .routing
                .fallback
                .as_ref()
                .is_some_and(|p| p.id == policy.id);
            if self.target_routing_members().is_empty()
                || policy.id.0.trim().is_empty()
                || !ids.insert(policy.id.clone())
                || policy.name.trim().is_empty()
                || !names.insert(policy.name.trim())
                || (fallback && (!policy.enabled || !policy.matchers.is_empty()))
                || (!fallback
                    && (policy.matchers.is_empty()
                        || policy.matchers.iter().any(|m| !m.is_valid())))
            {
                return Err(StateValidationError::InvalidRoute);
            }
            for member in &self.target_routing_members() {
                // 统一图先拒绝 self/cycle/dangling；停用策略可保留已有的停用引用。
                if catalog.get(member).is_err()
                    || (policy.enabled && catalog.require_available(member).is_err())
                {
                    return Err(StateValidationError::InvalidRoute);
                }
            }
        }
        catalog
            .validate_graph()
            .map_err(|_| StateValidationError::InvalidRoute)
    }
}
