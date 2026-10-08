use super::{
    AppState, NodeId, NodePool, OutboundGraphError, PoolId, PoolKind, ProviderId, ProxyNode,
    RouteTarget, RuntimeIntent, SelectionPolicy, SubscriptionId, validate_outbound_graph,
};

/// 显示名、协议和内核 tag 均不参与出口身份；当前仅定义 Base 所需种类。
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum OutboundId {
    Node(NodeId),
    Pool(PoolId),
    Direct,
    Block,
}

impl OutboundId {
    pub fn from_route_target(target: &RouteTarget) -> Result<Self, OutboundResolveError> {
        match target {
            RouteTarget::Pool(id) => Ok(Self::Pool(id.clone())),
            RouteTarget::Direct => Ok(Self::Direct),
            RouteTarget::Block => Ok(Self::Block),
            RouteTarget::Unconfigured => Err(OutboundResolveError::Unconfigured),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutboundKind {
    Node,
    Selector,
    UrlTest,
    Direct,
    Block,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutboundUnavailableReason {
    Disabled,
    EmptyMembers,
    MissingProvider(ProviderId),
    MissingSubscription(SubscriptionId),
    MissingNode(NodeId),
    SelectedNodeNotMember(NodeId),
    UnavailableReference(OutboundId),
    InvalidGraph(OutboundGraphError<OutboundId>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutboundAvailability {
    Available,
    Unavailable(OutboundUnavailableReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutboundResolveError {
    Unconfigured,
    Missing(OutboundId),
    Unavailable {
        id: OutboundId,
        reason: OutboundUnavailableReason,
    },
}

/// 仅含查询所需的非敏感事实，不复制节点凭据。内建出口无用户显示名，由 UI 本地化。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboundEntry {
    pub id: OutboundId,
    pub kind: OutboundKind,
    pub display_name: Option<String>,
    pub pool_kind: Option<PoolKind>,
    pub provider_ids: Vec<ProviderId>,
    pub subscription_ids: Vec<SubscriptionId>,
    pub references: Vec<OutboundId>,
    pub selection: Option<SelectionPolicy>,
    pub availability: OutboundAvailability,
}

impl OutboundEntry {
    pub fn node_members(&self) -> impl Iterator<Item = &NodeId> {
        self.references.iter().filter_map(|id| match id {
            OutboundId::Node(id) => Some(id),
            _ => None,
        })
    }
}

/// 每次从当前配置事实派生；不实现 serde、不存入 AppState 或 state.json。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboundCatalog {
    entries: Vec<OutboundEntry>,
}

impl OutboundCatalog {
    pub fn from_state(state: &AppState) -> Self {
        let mut entries = state.nodes.iter().map(node_entry).collect::<Vec<_>>();
        for entry in &mut entries {
            let provider_id = &entry.provider_ids[0];
            match state
                .providers
                .iter()
                .find(|provider| &provider.id == provider_id)
            {
                None => {
                    entry.availability = OutboundAvailability::Unavailable(
                        OutboundUnavailableReason::MissingProvider(provider_id.clone()),
                    )
                }
                Some(provider) => {
                    entry
                        .subscription_ids
                        .push(provider.subscription_id.clone());
                    if !state
                        .subscriptions
                        .iter()
                        .any(|s| s.id == provider.subscription_id)
                    {
                        entry.availability = OutboundAvailability::Unavailable(
                            OutboundUnavailableReason::MissingSubscription(
                                provider.subscription_id.clone(),
                            ),
                        );
                    }
                }
            }
        }
        for pool in &state.pools {
            entries.push(Self::pool_from_state(state, pool));
        }
        Self::with_terminals(entries)
    }

    /// 既有 RuntimeIntent 已是选定配置投影；沿用同一身份、成员、选择和图校验。
    /// 没有 provider/subscription 元数据时不猜测它们的生命周期或显示名。
    pub fn from_runtime_intent(intent: &RuntimeIntent) -> Self {
        let mut entries = intent.nodes.iter().map(node_entry).collect::<Vec<_>>();
        entries.extend(
            intent.pools.iter().map(|pool| {
                pool_entry(&pool.id, None, None, pool.members.clone(), &pool.selection)
            }),
        );
        Self::with_terminals(entries)
    }

    fn with_terminals(mut entries: Vec<OutboundEntry>) -> Self {
        for (id, kind) in [
            (OutboundId::Direct, OutboundKind::Direct),
            (OutboundId::Block, OutboundKind::Block),
        ] {
            entries.push(OutboundEntry {
                id,
                kind,
                display_name: None,
                pool_kind: None,
                provider_ids: Vec::new(),
                subscription_ids: Vec::new(),
                references: Vec::new(),
                selection: None,
                availability: OutboundAvailability::Available,
            });
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        // 将依赖不可用传播到 summary，list 与 resolve 使用相同结果。
        let unavailable = entries
            .iter()
            .filter(|e| e.availability != OutboundAvailability::Available)
            .map(|e| e.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let ids = entries
            .iter()
            .map(|e| e.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for entry in &mut entries {
            if entry.availability == OutboundAvailability::Available {
                if let Some(missing) = entry.references.iter().find(|id| !ids.contains(*id)) {
                    entry.availability = OutboundAvailability::Unavailable(match missing {
                        OutboundId::Node(id) => OutboundUnavailableReason::MissingNode(id.clone()),
                        _ => {
                            OutboundUnavailableReason::InvalidGraph(OutboundGraphError::Dangling {
                                source: entry.id.clone(),
                                target: missing.clone(),
                            })
                        }
                    });
                } else if let Some(id) =
                    entry.references.iter().find(|id| unavailable.contains(*id))
                {
                    entry.availability = OutboundAvailability::Unavailable(
                        OutboundUnavailableReason::UnavailableReference(id.clone()),
                    );
                }
            }
        }
        // 图结构非法时整个派生快照不能作为可用配置；list/get/resolve 保持一致。
        if let Err(error) =
            validate_outbound_graph(entries.iter().map(|e| (e.id.clone(), e.references.clone())))
        {
            for entry in &mut entries {
                if entry.availability == OutboundAvailability::Available {
                    entry.availability = OutboundAvailability::Unavailable(
                        OutboundUnavailableReason::InvalidGraph(error.clone()),
                    );
                }
            }
        }
        Self { entries }
    }

    /// 与 StateValidation 共用成员计算；未验证快照也能明确报告缺失引用。
    pub fn pool_from_state(state: &AppState, pool: &NodePool) -> OutboundEntry {
        let mut entry = pool_entry(
            &pool.id,
            Some(pool.name.clone()),
            Some(pool.kind),
            state.resolve_pool_members(pool),
            &pool.selection,
        );
        let mut reason = None;
        for source in &pool.sources {
            entry.provider_ids.push(source.provider_id.clone());
            if let Some(provider) = state.providers.iter().find(|p| p.id == source.provider_id) {
                entry
                    .subscription_ids
                    .push(provider.subscription_id.clone());
                if !state
                    .subscriptions
                    .iter()
                    .any(|s| s.id == provider.subscription_id)
                {
                    reason.get_or_insert(OutboundUnavailableReason::MissingSubscription(
                        provider.subscription_id.clone(),
                    ));
                }
            } else {
                reason.get_or_insert(OutboundUnavailableReason::MissingProvider(
                    source.provider_id.clone(),
                ));
            }
            // include/exclude 是配置引用，即使节点已删除，也不能静默解释为空的过滤结果。
            for id in source
                .filter
                .include_node_ids
                .iter()
                .chain(&source.filter.exclude_node_ids)
            {
                if !state
                    .nodes
                    .iter()
                    .any(|n| n.id == *id && n.provider_id == source.provider_id)
                {
                    reason.get_or_insert(OutboundUnavailableReason::MissingNode(id.clone()));
                }
            }
        }
        entry.provider_ids.sort();
        entry.provider_ids.dedup();
        entry.subscription_ids.sort();
        entry.subscription_ids.dedup();
        if !pool.enabled {
            reason = Some(OutboundUnavailableReason::Disabled);
        }
        if let Some(reason) = reason {
            entry.availability = OutboundAvailability::Unavailable(reason);
        }
        entry
    }

    pub fn list(&self) -> &[OutboundEntry] {
        &self.entries
    }

    /// lookup 保留 unavailable entry；缺失身份与不可用身份严格区分。
    pub fn get(&self, id: &OutboundId) -> Result<&OutboundEntry, OutboundResolveError> {
        self.entries
            .iter()
            .find(|e| &e.id == id)
            .ok_or_else(|| OutboundResolveError::Missing(id.clone()))
    }

    pub fn resolve(&self, target: &RouteTarget) -> Result<OutboundId, OutboundResolveError> {
        let id = OutboundId::from_route_target(target)?;
        self.require_available(&id)?;
        Ok(id)
    }

    pub fn require_available(
        &self,
        id: &OutboundId,
    ) -> Result<&OutboundEntry, OutboundResolveError> {
        let entry = self.get(id)?;
        if let OutboundAvailability::Unavailable(reason) = &entry.availability {
            return Err(OutboundResolveError::Unavailable {
                id: id.clone(),
                reason: reason.clone(),
            });
        }
        Ok(entry)
    }

    pub fn validate_graph(&self) -> Result<(), OutboundGraphError<OutboundId>> {
        validate_outbound_graph(
            self.entries
                .iter()
                .map(|e| (e.id.clone(), e.references.clone())),
        )
    }
}

fn node_entry(node: &ProxyNode) -> OutboundEntry {
    OutboundEntry {
        id: OutboundId::Node(node.id.clone()),
        kind: OutboundKind::Node,
        display_name: Some(node.name.clone()),
        pool_kind: None,
        provider_ids: vec![node.provider_id.clone()],
        subscription_ids: Vec::new(),
        references: Vec::new(),
        selection: None,
        availability: OutboundAvailability::Available,
    }
}

fn pool_entry(
    id: &PoolId,
    display_name: Option<String>,
    pool_kind: Option<PoolKind>,
    mut members: Vec<NodeId>,
    selection: &SelectionPolicy,
) -> OutboundEntry {
    members.sort();
    members.dedup();
    let reason = match selection {
        _ if members.is_empty() => Some(OutboundUnavailableReason::EmptyMembers),
        SelectionPolicy::Manual {
            selected_node_id: Some(id),
        } if !members.contains(id) => {
            Some(OutboundUnavailableReason::SelectedNodeNotMember(id.clone()))
        }
        _ => None,
    };
    OutboundEntry {
        id: OutboundId::Pool(id.clone()),
        kind: match selection {
            SelectionPolicy::Manual { .. } => OutboundKind::Selector,
            SelectionPolicy::UrlTest { .. } => OutboundKind::UrlTest,
        },
        display_name,
        pool_kind,
        provider_ids: Vec::new(),
        subscription_ids: Vec::new(),
        references: members.into_iter().map(OutboundId::Node).collect(),
        selection: Some(selection.clone()),
        availability: reason.map_or(
            OutboundAvailability::Available,
            OutboundAvailability::Unavailable,
        ),
    }
}

#[cfg(test)]
mod tests;
