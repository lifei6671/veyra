use std::collections::HashSet;

use crate::domain::{
    AppState, NodeId, PoolId, PoolKind, RouteTarget, RuntimeIntent, RuntimePool, SelectionPolicy,
    StateValidationError, SubscriptionId,
};

const RUNTIME_POOL_PREFIX: &str = "runtime-active-";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedRuntimeProjection {
    pub(crate) runtime_intent: RuntimeIntent,
    pub(crate) projected_default_target: RouteTarget,
    pub(crate) selected_subscription_id: SubscriptionId,
    pub(crate) selected_generation: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionProjectionError {
    SelectionRequired,
    NotFound,
    SelectionConflict,
    ConfigurationFailed,
}

pub(crate) fn project_selected_runtime(
    state: &AppState,
) -> Result<SelectedRuntimeProjection, SelectionProjectionError> {
    let selected_subscription_id = state
        .active_subscription_id
        .clone()
        .ok_or(SelectionProjectionError::SelectionRequired)?;
    if !state
        .subscriptions
        .iter()
        .any(|subscription| subscription.id == selected_subscription_id)
    {
        return Err(SelectionProjectionError::NotFound);
    }
    state
        .validate()
        .map_err(|_| SelectionProjectionError::ConfigurationFailed)?;
    let selected_provider_ids = state
        .providers
        .iter()
        .filter(|provider| provider.subscription_id == selected_subscription_id)
        .map(|provider| provider.id.clone())
        .collect::<HashSet<_>>();
    if selected_provider_ids.is_empty() {
        return Err(SelectionProjectionError::ConfigurationFailed);
    }

    let mut nodes = state
        .nodes
        .iter()
        .filter(|node| selected_provider_ids.contains(&node.provider_id))
        .cloned()
        .collect::<Vec<_>>();
    if nodes.is_empty() {
        return Err(SelectionProjectionError::ConfigurationFailed);
    }
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    let mut selected_node_ids = nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();

    let implicit_pool_id = PoolId(format!(
        "{RUNTIME_POOL_PREFIX}{}",
        selected_subscription_id.0
    ));
    if state.pools.iter().any(|pool| pool.id == implicit_pool_id) {
        return Err(SelectionProjectionError::SelectionConflict);
    }
    let mut pools = vec![RuntimePool {
        id: implicit_pool_id.clone(),
        members: sorted_node_ids(&selected_node_ids),
        selection: SelectionPolicy::Manual {
            selected_node_id: None,
        },
    }];

    for pool in state.pools.iter().filter(|pool| pool.enabled) {
        let mut filtered = pool.clone();
        if pool.kind == PoolKind::ImplicitProvider {
            filtered
                .sources
                .retain(|source| selected_provider_ids.contains(&source.provider_id));
        }
        if filtered.sources.is_empty() {
            continue;
        }
        let members = state
            .resolve_pool_members(&filtered)
            .into_iter()
            .filter(|node_id| pool.kind == PoolKind::Custom || selected_node_ids.contains(node_id))
            .collect::<Vec<_>>();
        if members.is_empty()
            || matches!(
                &filtered.selection,
                SelectionPolicy::Manual {
                    selected_node_id: Some(node_id)
                } if !members.contains(node_id)
            )
        {
            return Err(SelectionProjectionError::SelectionConflict);
        }
        if pool.kind == PoolKind::Custom {
            selected_node_ids.extend(members.iter().cloned());
        }
        pools.push(RuntimePool {
            id: filtered.id,
            members,
            selection: filtered.selection,
        });
    }
    nodes = state
        .nodes
        .iter()
        .filter(|node| selected_node_ids.contains(&node.id))
        .cloned()
        .collect();
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    pools.sort_by(|left, right| left.id.cmp(&right.id));
    let pool_ids = pools
        .iter()
        .map(|pool| pool.id.clone())
        .collect::<HashSet<_>>();

    let mut routes = state
        .routes
        .iter()
        .filter(|route| route.enabled)
        .cloned()
        .collect::<Vec<_>>();
    if routes.iter().any(
        |route| matches!(&route.target, RouteTarget::Pool(pool_id) if !pool_ids.contains(pool_id)),
    ) {
        return Err(SelectionProjectionError::SelectionConflict);
    }
    routes.sort_by_key(|route| (route.priority, route.id.clone()));

    let projected_default_target = match &state.default_target {
        RouteTarget::Unconfigured => RouteTarget::Pool(implicit_pool_id),
        RouteTarget::Pool(pool_id) if pool_ids.contains(pool_id) => {
            RouteTarget::Pool(pool_id.clone())
        }
        RouteTarget::Direct => RouteTarget::Direct,
        RouteTarget::Block => RouteTarget::Block,
        RouteTarget::Pool(_) => return Err(SelectionProjectionError::SelectionConflict),
    };
    if let RouteTarget::Pool(default_pool_id) = &projected_default_target
        && pools
            .iter()
            .find(|pool| &pool.id == default_pool_id)
            .is_none_or(|pool| pool.members.is_empty())
    {
        return Err(SelectionProjectionError::ConfigurationFailed);
    }

    Ok(SelectedRuntimeProjection {
        runtime_intent: RuntimeIntent {
            nodes,
            pools,
            routes,
        },
        projected_default_target,
        selected_subscription_id,
        selected_generation: state.active_configuration_generation,
    })
}

fn sorted_node_ids(node_ids: &HashSet<NodeId>) -> Vec<NodeId> {
    let mut values = node_ids.iter().cloned().collect::<Vec<_>>();
    values.sort();
    values
}

impl From<StateValidationError> for SelectionProjectionError {
    fn from(_: StateValidationError) -> Self {
        Self::ConfigurationFailed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        NodeFilter, NodePool, PoolKind, PoolSource, ProtocolOptions, Provider, ProviderId,
        ProxyNode, ProxyProtocol, RoutePolicy, RoutePolicyId, Subscription, SubscriptionSource,
        SubscriptionUpdatePolicy, TrafficMatcher, Transport,
    };

    fn subscription(id: &str) -> Subscription {
        Subscription {
            document: None,
            skipped_unsupported_nodes: 0,
            id: SubscriptionId(id.to_owned()),
            name: id.to_owned(),
            description: String::new(),
            source: SubscriptionSource::Manual,
            last_success_at_ms: None,
            last_attempt_at_ms: None,
            http_metadata: None,
            remote_request: None,
            update_policy: SubscriptionUpdatePolicy::manual(),
        }
    }

    fn state() -> AppState {
        let mut state = AppState::empty();
        state.active_subscription_id = Some(SubscriptionId("subscription-b".to_owned()));
        state.active_configuration_generation = 3;
        for suffix in ["a", "b"] {
            state
                .subscriptions
                .push(subscription(&format!("subscription-{suffix}")));
            state.providers.push(Provider {
                id: ProviderId(format!("provider-{suffix}")),
                subscription_id: SubscriptionId(format!("subscription-{suffix}")),
                name: suffix.to_owned(),
            });
            state.nodes.push(ProxyNode {
                id: NodeId(format!("node-{suffix}")),
                provider_id: ProviderId(format!("provider-{suffix}")),
                name: suffix.to_owned(),
                protocol: ProxyProtocol::Shadowsocks,
                server: "example.invalid".to_owned(),
                port: 443,
                options: ProtocolOptions::Shadowsocks {
                    method: "aes-128-gcm".to_owned(),
                    password: "secret".to_owned(),
                },
                transport: Some(Transport::Tcp),
                tls: None,
            });
        }
        state
    }

    #[test]
    fn unconfigured_default_projects_only_the_selected_subscription() {
        let projection = project_selected_runtime(&state()).expect("selected projection");

        assert_eq!(projection.selected_subscription_id.0, "subscription-b");
        assert_eq!(projection.selected_generation, 3);
        assert_eq!(projection.runtime_intent.nodes.len(), 1);
        assert_eq!(projection.runtime_intent.nodes[0].id.0, "node-b");
        let RouteTarget::Pool(default_pool) = projection.projected_default_target else {
            panic!("default must be a pool")
        };
        assert!(
            projection
                .runtime_intent
                .pools
                .iter()
                .any(|pool| pool.id == default_pool && !pool.members.is_empty())
        );
    }

    #[test]
    fn enabled_custom_pool_adds_only_its_exact_cross_subscription_members() {
        let mut state = state();
        state.pools.push(NodePool {
            id: PoolId("mixed".to_owned()),
            name: "mixed".to_owned(),
            kind: PoolKind::Custom,
            sources: ["a", "b"]
                .map(|suffix| PoolSource {
                    provider_id: ProviderId(format!("provider-{suffix}")),
                    filter: NodeFilter::default(),
                })
                .to_vec(),
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(NodeId("node-b".to_owned())),
            },
            enabled: true,
        });
        state.default_target = RouteTarget::Pool(PoolId("mixed".to_owned()));

        let projection = project_selected_runtime(&state).expect("filtered projection");

        assert_eq!(
            projection.projected_default_target,
            RouteTarget::Pool(PoolId("mixed".to_owned()))
        );
        assert_eq!(
            projection
                .runtime_intent
                .pools
                .iter()
                .find(|pool| pool.id.0 == "mixed")
                .expect("mixed pool")
                .members,
            vec![NodeId("node-a".to_owned()), NodeId("node-b".to_owned())]
        );
        assert_eq!(
            projection
                .runtime_intent
                .nodes
                .iter()
                .map(|node| node.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["node-a", "node-b"]
        );
    }

    #[test]
    fn direct_and_block_remain_valid_default_targets() {
        for target in [RouteTarget::Direct, RouteTarget::Block] {
            let mut state = state();
            state.default_target = target.clone();
            assert_eq!(
                project_selected_runtime(&state)
                    .expect("direct and block are structural targets")
                    .projected_default_target,
                target
            );
        }
    }

    #[test]
    fn route_can_target_an_enabled_cross_subscription_custom_pool() {
        let mut state = state();
        state.pools.push(NodePool {
            id: PoolId("a-only".to_owned()),
            name: "a-only".to_owned(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider-a".to_owned()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: None,
            },
            enabled: true,
        });
        state.routes.push(RoutePolicy {
            id: RoutePolicyId("route".to_owned()),
            name: "route".to_owned(),
            enabled: true,
            priority: 0,
            matcher: TrafficMatcher::Domain(vec!["example.com".to_owned()]),
            target: RouteTarget::Pool(PoolId("a-only".to_owned())),
        });
        let projection = project_selected_runtime(&state).expect("explicit custom pool closure");
        assert_eq!(projection.runtime_intent.routes.len(), 1);
        assert_eq!(projection.runtime_intent.nodes.len(), 2);
    }

    #[test]
    fn disabled_custom_pool_does_not_expand_the_active_projection() {
        let mut state = state();
        state.pools.push(NodePool {
            id: PoolId("disabled".to_owned()),
            name: "disabled".to_owned(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider-a".to_owned()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(NodeId("node-a".to_owned())),
            },
            enabled: false,
        });

        let projection = project_selected_runtime(&state).expect("active-only projection");
        assert_eq!(projection.runtime_intent.nodes.len(), 1);
        assert_eq!(projection.runtime_intent.nodes[0].id.0, "node-b");
        assert!(
            projection
                .runtime_intent
                .pools
                .iter()
                .all(|pool| pool.id.0 != "disabled")
        );
    }

    #[test]
    fn custom_filter_does_not_import_unmatched_siblings() {
        let mut state = state();
        let mut sibling = state.nodes[0].clone();
        sibling.id = NodeId("node-a-sibling".to_owned());
        sibling.name = "sibling".to_owned();
        state.nodes.push(sibling);
        state.pools.push(NodePool {
            id: PoolId("filtered".to_owned()),
            name: "filtered".to_owned(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider-a".to_owned()),
                filter: NodeFilter {
                    include_node_ids: vec![NodeId("node-a".to_owned())],
                    ..NodeFilter::default()
                },
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(NodeId("node-a".to_owned())),
            },
            enabled: true,
        });

        let projection = project_selected_runtime(&state).expect("filtered custom closure");
        assert!(
            projection
                .runtime_intent
                .nodes
                .iter()
                .any(|node| node.id.0 == "node-a")
        );
        assert!(
            projection
                .runtime_intent
                .nodes
                .iter()
                .all(|node| node.id.0 != "node-a-sibling")
        );
    }
}
