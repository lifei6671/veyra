//! 异步只读探测，结果回到原串行 owner。时钟由 worker 注入，阈值仍由纯策略判断。
use super::*;
use crate::{
    application::failover::*,
    domain::{GroupRule, GroupSelectionMode},
};
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailoverStatus {
    Checking,
    Healthy,
    AllFailed,
    PinnedUnavailable,
    ProbeFailed,
    ConfigChanged,
    Pending,
    SelectionFailed(SelectionError),
}
pub(super) struct Runner {
    identity: FailoverProbeToken,
    incarnation: u64,
    mode: GroupSelectionMode,
    policy: FailoverPolicy,
    last_probe_ms: Option<u64>,
    task: Option<tokio::task::JoinHandle<()>>,
}
impl Drop for Runner {
    fn drop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}
pub(super) struct ProbeResult {
    incarnation: u64,
    token: FailoverProbeToken,
    health: Result<Vec<(String, bool)>, ()>,
}
impl<P: SidecarPort> ManualRuntime<P> {
    pub(super) fn invalidate_failover(&mut self) {
        self.failover.clear();
        self.failover_generation = self
            .failover_generation
            .checked_add(1)
            .expect("probe incarnation exhausted");
    }
    pub(super) fn invalidate_group_probes(&mut self, target: &PoolId) {
        self.failover.remove(target);
        self.failover_generation = self
            .failover_generation
            .checked_add(1)
            .expect("probe incarnation exhausted");
        for runner in self.failover.values_mut() {
            if let Some(task) = runner.task.take() {
                task.abort();
            }
            runner
                .policy
                .rebind_selection(runner.identity.selection.clone());
            runner.incarnation = self.failover_generation;
        }
    }
    pub(super) fn rebind_failover_selection(&mut self, selection: &SelectionVersion) {
        for runner in self.failover.values_mut() {
            if let Some(task) = runner.task.take() {
                task.abort();
            }
            runner.identity.selection = selection.clone();
            runner.policy.rebind_selection(selection.clone());
        }
    }
    pub fn poll_failover(&mut self, now_ms: u64) {
        let RuntimeState::Ready {
            instance_id,
            applied_version,
        } = &self.facts.current
        else {
            self.invalidate_failover();
            self.failover_status.clear();
            return;
        };
        let instance = instance_id.clone();
        let config = applied_version.clone();
        let Ok(state) = self.snapshots.snapshot() else {
            self.invalidate_failover();
            return;
        };
        let groups: Vec<_> = state
            .groups
            .iter()
            .filter(|g| g.rule == GroupRule::Failover && g.enabled)
            .collect();
        if state.config_version() != config || Self::has_pending(&state) {
            if state.config_version() != config {
                self.invalidate_failover();
            }
            for group in groups {
                self.failover_status.insert(
                    group.id.clone(),
                    if Self::has_pending(&state) {
                        FailoverStatus::Pending
                    } else {
                        FailoverStatus::ConfigChanged
                    },
                );
            }
            return;
        }
        let Some(active) = &self.active else {
            return;
        };
        if active.selection.version != state.selection_version() {
            return;
        }
        let index = active.plan.artifact_index().unwrap().clone();
        let confirmed = active.selection.groups.clone();
        self.failover
            .retain(|id, _| groups.iter().any(|g| g.id == *id));
        self.failover_status
            .retain(|id, _| groups.iter().any(|g| g.id == *id));
        for group in &groups {
            let mode = state
                .group_selections
                .get(&group.id)
                .map(|s| s.mode)
                .unwrap_or_default();
            let identity = FailoverProbeToken {
                instance: instance.clone(),
                group: group.id.clone(),
                selection: state.selection_version(),
                config: config.clone(),
                generation: 0,
            };
            if let Some(runner) = self.failover.get_mut(&group.id)
                && runner.identity.instance == identity.instance
                && runner.identity.config == identity.config
                && runner.mode == mode
                && runner.identity.selection != identity.selection
            {
                if let Some(task) = runner.task.take() {
                    task.abort();
                }
                runner.identity.selection = identity.selection.clone();
                runner.policy.rebind_selection(identity.selection.clone());
            }
            if self
                .failover
                .get(&group.id)
                .is_none_or(|r| r.identity != identity || r.mode != mode)
            {
                let Ok(mut policy) = FailoverPolicy::new(
                    instance.clone(),
                    group,
                    state.selection_version(),
                    config.clone(),
                ) else {
                    continue;
                };
                if mode == GroupSelectionMode::ManualPin {
                    let selected = confirmed.get(&group.id);
                    let Some(lane) = group
                        .lanes
                        .iter()
                        .find(|l| Some(&OutboundId::Pool(l.pool_id(&group.id))) == selected)
                    else {
                        self.failover_status.insert(
                            group.id.clone(),
                            FailoverStatus::SelectionFailed(SelectionError::InvalidMember),
                        );
                        continue;
                    };
                    let _ = policy.pin(&lane.id, state.selection_version());
                }
                self.failover.insert(
                    group.id.clone(),
                    Runner {
                        identity,
                        incarnation: self.failover_generation,
                        mode,
                        policy,
                        last_probe_ms: None,
                        task: None,
                    },
                );
            }
        }
        // 不对未知结果重发选择；只有本代完整健康批次才能提出一个新选择。
        while let Ok(result) = self.failover_results.try_recv() {
            let Some(group) = groups.iter().find(|g| g.id == result.token.group) else {
                continue;
            };
            let Some(runner) = self.failover.get_mut(&group.id) else {
                continue;
            };
            if result.incarnation != runner.incarnation
                || result.token.instance != runner.identity.instance
                || result.token.config != runner.identity.config
                || result.token.selection != runner.identity.selection
            {
                continue;
            }
            let current = confirmed
                .get(&group.id)
                .and_then(|m| {
                    group
                        .lanes
                        .iter()
                        .find(|l| *m == OutboundId::Pool(l.pool_id(&group.id)))
                })
                .map(|l| l.id.as_str());
            let health = match result.health {
                Ok(health) => health,
                Err(()) => {
                    runner.task = None;
                    self.failover_status
                        .insert(group.id.clone(), FailoverStatus::ProbeFailed);
                    continue;
                }
            };
            let decision = runner
                .policy
                .observe(&result.token, current, &health, now_ms);
            if decision == FailoverDecision::StaleProbe {
                continue;
            }
            runner.task = None;
            let status = match decision {
                FailoverDecision::AllFailed => FailoverStatus::AllFailed,
                FailoverDecision::PinnedUnavailable => FailoverStatus::PinnedUnavailable,
                FailoverDecision::Select { lane_id, .. } => {
                    let lane = group.lanes.iter().find(|l| l.id == lane_id).unwrap();
                    let request = ManualSelectionRequest {
                        instance: instance.clone(),
                        config: config.clone(),
                        expected: state.selection_version(),
                        pool: group.id.clone(),
                        node: GroupChoice {
                            group: group.id.clone(),
                            member: OutboundId::Pool(lane.pool_id(&group.id)),
                            mode: GroupSelectionMode::Auto,
                        },
                    };
                    match self.select_manual(request) {
                        Ok(_) => FailoverStatus::Healthy,
                        Err(e) => FailoverStatus::SelectionFailed(e),
                    }
                }
                _ => FailoverStatus::Healthy,
            };
            self.failover_status.insert(group.id.clone(), status);
            // 成功选择推进全局 selection_revision；同一批其它组结果下一 tick 重新绑定。
            if self
                .snapshots
                .snapshot()
                .is_ok_and(|s| s.selection_version() != state.selection_version())
            {
                return;
            }
        }
        let endpoint = self
            .sidecar
            .with_active_port(|p, c| Ok(p.observation_endpoint(c)))
            .ok()
            .flatten()
            .flatten();
        let Some(endpoint) = endpoint else {
            for group in groups {
                self.failover_status
                    .insert(group.id.clone(), FailoverStatus::ProbeFailed);
            }
            return;
        };
        let Some(executor) = &self.observation_executor else {
            return;
        };
        for group in groups {
            let Some(runner) = self.failover.get_mut(&group.id) else {
                continue;
            };
            if runner.task.is_some()
                || runner
                    .last_probe_ms
                    .is_some_and(|last| now_ms.saturating_sub(last) < group.interval_secs * 1000)
            {
                continue;
            }
            let tags: Option<Vec<_>> = group
                .lanes
                .iter()
                .map(|l| {
                    Some((
                        l.id.clone(),
                        index.group_health.get(&l.pool_id(&group.id))?.clone(),
                        index.group_tag(&l.pool_id(&group.id)),
                    ))
                })
                .collect();
            let Some(tags) = tags else {
                self.failover_status
                    .insert(group.id.clone(), FailoverStatus::ProbeFailed);
                continue;
            };
            let token = runner.policy.begin_probe();
            let incarnation = runner.incarnation;
            let sender = self.failover_sender.clone();
            let endpoint = endpoint.clone();
            let timeout = group.failover.as_ref().unwrap().timeout_ms;
            let test_url = if group.test_url.is_empty() {
                state.profile.test_url.as_str().into()
            } else {
                group.test_url.clone()
            };
            runner.last_probe_ms = Some(now_ms);
            runner.task = Some(executor.spawn(async move {
                let mut health = vec![];
                let result = async {
                    let client = crate::singbox::clash_api::ClashApiClient::managed(&endpoint)
                        .map_err(|_| ())?;
                    for (lane, tag, selector) in tags {
                        let selected = if let Some(selector) = selector {
                            let mut selected =
                                client.read_selector(&selector).await.map_err(|_| ())?;
                            let mut visited = std::collections::HashSet::new();
                            // 当前 Manual 成员可引用其它组；只读收敛到内核实际叶子，不自选节点。
                            while selected.starts_with("pool-") {
                                if !visited.insert(selected.clone()) {
                                    return Err(());
                                }
                                selected = client.read_selector(&selected).await.map_err(|_| ())?;
                            }
                            Some(selected)
                        } else {
                            None
                        };
                        let delays = client
                            .test_group(&tag, timeout, &test_url)
                            .await
                            .map_err(|_| ())?;
                        health.push((
                            lane,
                            selected
                                .as_ref()
                                .map_or(!delays.is_empty(), |tag| delays.contains_key(tag)),
                        ));
                    }
                    Ok(health)
                }
                .await;
                let _ = sender.send(ProbeResult {
                    incarnation,
                    token,
                    health: result,
                });
            }));
            // 保留上轮失败直到新结果到达，避免每周期把错误闪成成功。
            self.failover_status
                .entry(group.id.clone())
                .or_insert(FailoverStatus::Checking);
        }
    }
}
