//! Group 与线路仍由唯一 Runtime owner 驱动；稳定 OutboundId 不进入旧 NodeId map。
use super::*;
use crate::domain::{GroupSelectionIntent, GroupSelectionMode};

impl<P: SidecarPort> ManualRuntime<P> {
    pub(super) fn group_selection_for(
        plan: &SingBoxPlan,
        state: &AppState,
        prior: Option<&ConfirmedSelection>,
        fallbacks: &mut Vec<PoolId>,
    ) -> BTreeMap<PoolId, OutboundId> {
        let index = plan.artifact_index().expect("index");
        index
            .groups
            .iter()
            .map(|(id, members)| {
                let selected = if state.state_epoch == index.config.0.epoch
                    && prior.is_none_or(|p| state.selection_revision >= p.version.0.revision)
                {
                    state
                        .group_selections
                        .get(id)
                        .and_then(|s| s.selected.clone())
                } else {
                    prior.and_then(|p| p.groups.get(id).cloned())
                };
                let member = match selected {
                    Some(m) if members.contains(&m) => m,
                    Some(_) => {
                        fallbacks.push(id.clone());
                        members[0].clone()
                    }
                    None => members[0].clone(),
                };
                (id.clone(), member)
            })
            .collect()
    }
    pub(super) fn select_group(
        &mut self,
        _instance: InstanceId,
        config: ConfigVersion,
        expected: SelectionVersion,
        selector: PoolId,
        choice: GroupChoice,
    ) -> Result<SelectionVersion, SelectionError> {
        let state = self
            .snapshots
            .snapshot()
            .map_err(|_| SelectionError::StateUnavailable)?;
        let group = state
            .groups
            .iter()
            .find(|g| g.id == choice.group)
            .ok_or(SelectionError::InvalidMember)?;
        if choice.mode == GroupSelectionMode::ManualPin
            && group.rule != crate::domain::GroupRule::Failover
        {
            return Err(SelectionError::InvalidMember);
        }
        if selector != group.id
            && !group
                .lanes
                .iter()
                .any(|l| l.pool_id(&group.id) == selector && (l.manual || l.members.len() == 1))
        {
            return Err(SelectionError::InvalidMember);
        }
        if selector != group.id && choice.mode != GroupSelectionMode::Auto {
            return Err(SelectionError::InvalidMember);
        }
        let active = self.active.as_ref().expect("ready plan");
        let index = active.plan.artifact_index().expect("index");
        let tag = index
            .group_tag(&selector)
            .ok_or(SelectionError::InvalidMember)?;
        let member_tag = index
            .group_member_tag(&selector, &choice.member)
            .ok_or(SelectionError::InvalidMember)?;
        let old = index
            .group_member_tag(&selector, &active.selection.groups[&selector])
            .ok_or(SelectionError::InvalidMember)?;
        // 人工操作在任何 pending/Controller 写之前取消旧 probe；失败仍由持久旧 mode 重建。
        self.invalidate_group_probes(&choice.group);
        let intent = GroupSelectionIntent {
            member: choice.member.clone(),
            mode: choice.mode,
        };
        let service = SelectionService::new((*self.snapshots).clone());
        let pending = service
            .stage_group(
                config.clone(),
                expected,
                selector.clone(),
                intent.clone(),
                None,
            )
            .map_err(|e| {
                if e.code() == crate::domain::AppErrorCode::RevisionConflict {
                    SelectionError::StaleVersion
                } else {
                    SelectionError::PendingSaveFailed
                }
            })?;
        if let Err(error) = self.controller_confirm(&config, &tag, &member_tag, &pending) {
            if error == SelectionError::ControllerWrite
                && matches!(self.sidecar.with_active_port(|p,c| p.read_selector(c,&tag)), Ok(Some(ref value)) if *value == old)
            {
                let cleared = service
                    .stage_group(config, pending, selector, intent, Some(false))
                    .map_err(|_| SelectionError::PendingClearFailed)?;
                self.active.as_mut().unwrap().selection.version = cleared;
                self.commit_confirmed_selection()?;
            }
            return Err(error);
        }
        let saved = service
            .stage_group(config, pending, selector.clone(), intent, Some(true))
            .map_err(|_| SelectionError::ConfirmationSaveFailed)?;
        let active = self.active.as_mut().unwrap();
        active.selection.groups.insert(selector, choice.member);
        active.selection.version = saved.clone();
        self.rebind_failover_selection(&saved);
        self.commit_confirmed_selection()?;
        Ok(saved)
    }
    /// 重连读取真实选择解决 pending。未知第三值保留 pending，绝不重发旧意图。
    pub(super) fn reconcile_groups(
        &mut self,
        plan: &SingBoxPlan,
        current: &mut AppState,
        selection: &mut ConfirmedSelection,
        trusted: bool,
    ) -> Result<(), RuntimeError> {
        let index = plan.artifact_index().unwrap();
        let service = SelectionService::new((*self.snapshots).clone());
        for (id, member) in &mut selection.groups {
            let tag = index.group_tag(id).unwrap();
            let pending = current
                .group_selections
                .get(id)
                .and_then(|s| s.pending.clone());
            if pending.is_some() && !trusted {
                return Err(RuntimeError::SelectionPending);
            }
            let actual = self
                .sidecar
                .with_active_port(|p, c| p.read_selector(c, &tag))
                .map_err(|_| RuntimeError::SelectionPending)?
                .ok_or(RuntimeError::SelectionReconcileFailed)?;
            if let Some(intent) = pending {
                let requested = index
                    .group_member_tag(id, &intent.member)
                    .ok_or(RuntimeError::SelectionPending)?;
                let old = current
                    .group_selections
                    .get(id)
                    .and_then(|s| s.selected.clone())
                    .unwrap_or_else(|| index.groups[id][0].clone());
                let confirm = if actual == requested {
                    true
                } else if index
                    .group_member_tag(id, &old)
                    .is_some_and(|tag| tag == actual)
                {
                    false
                } else {
                    return Err(RuntimeError::SelectionPending);
                };
                let saved = service
                    .stage_group(
                        current.config_version(),
                        current.selection_version(),
                        id.clone(),
                        intent.clone(),
                        Some(confirm),
                    )
                    .map_err(|_| RuntimeError::SelectionPending)?;
                *member = if confirm { intent.member } else { old };
                let config = current.config_version();
                *current = self
                    .snapshots
                    .snapshot()
                    .map_err(|_| RuntimeError::StateUnavailable)?;
                if current.selection_version() != saved || current.config_version() != config {
                    return Err(RuntimeError::SelectionPending);
                }
                selection.version = saved;
            } else if actual != index.group_member_tag(id, member).unwrap() {
                self.controller_confirm(
                    &current.config_version(),
                    &tag,
                    &index.group_member_tag(id, member).unwrap(),
                    &current.selection_version(),
                )
                .map_err(|_| RuntimeError::SelectionReconcileFailed)?;
            }
        }
        Ok(())
    }
}
