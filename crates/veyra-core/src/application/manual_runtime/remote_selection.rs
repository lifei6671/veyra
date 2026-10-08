//! helper选择执行器：仅操作当前controller和manifest，绝不提交桌面业务state。
use super::*;
use crate::storage::RemoteSelection;
impl<P: SidecarPort> ManualRuntime<P> {
    pub fn remote_selection_preflight(
        &mut self,
        request: &RemoteSelection,
    ) -> Result<(), SelectionError> {
        if !matches!(self.facts.current, RuntimeState::Ready { .. })
            || !self
                .sidecar
                .refresh_alive()
                .map_err(|_| SelectionError::StaleInstance)?
        {
            return Err(SelectionError::StaleInstance);
        }
        let active = self.active.as_ref().ok_or(SelectionError::StaleInstance)?;
        let index = active
            .plan
            .artifact_index()
            .ok_or(SelectionError::ConfigChanged)?;
        if index.config != request.expected.config
            || active.selection.version != request.expected.selection
        {
            return Err(SelectionError::StaleVersion);
        }
        let state = self
            .snapshots
            .snapshot()
            .map_err(|_| SelectionError::StateUnavailable)?;
        if state.version() != request.expected
            || !Self::pending_in(&state).is_empty()
            || !state.pools.iter().any(|p| {
                p.id == request.pool && matches!(p.selection, SelectionPolicy::Manual { .. })
            })
        {
            return Err(SelectionError::ConfigChanged);
        }
        let pool = index
            .pools
            .get(&request.pool)
            .ok_or(SelectionError::InvalidMember)?;
        if !pool.members.contains_key(&request.node) {
            return Err(SelectionError::InvalidMember);
        }
        Ok(())
    }
    /// 调用方已经保留单个事务slot；PUT报错仍须GET，错误本身不能证明未切换。
    pub fn remote_selection_controller(
        &mut self,
        request: &RemoteSelection,
        write: bool,
    ) -> Result<NodeId, SelectionError> {
        if write {
            self.remote_selection_preflight(request)?;
        } else if !self
            .sidecar
            .refresh_alive()
            .map_err(|_| SelectionError::StaleInstance)?
            || self.active.as_ref().is_none_or(|a| {
                a.plan.artifact_index().is_none_or(|i| {
                    i.config != request.expected.config || !i.pools.contains_key(&request.pool)
                })
            })
        {
            return Err(SelectionError::StaleInstance);
        }
        let pool = self
            .active
            .as_ref()
            .unwrap()
            .plan
            .artifact_index()
            .unwrap()
            .pools[&request.pool]
            .clone();
        if write {
            let _ = self.sidecar.with_active_port(|p, c| {
                p.write_selector(c, &pool.runtime_tag, &pool.members[&request.node])
            });
        }
        let tag = self
            .sidecar
            .with_active_port(|p, c| p.read_selector(c, &pool.runtime_tag))
            .map_err(|_| SelectionError::ControllerReadBack)?
            .ok_or(SelectionError::StaleInstance)?;
        pool.members
            .iter()
            .find(|(_, t)| **t == tag)
            .map(|(id, _)| id.clone())
            .ok_or(SelectionError::ControllerReadBack)
    }
    /// 桌面精确CAS确认后仅更新运行事实和last-applied；失败不能假装manifest成功。
    pub fn acknowledge_remote_selection(
        &mut self,
        request: &RemoteSelection,
        actual: NodeId,
        version: SelectionVersion,
    ) -> Result<(), SelectionError> {
        let active = self.active.as_mut().ok_or(SelectionError::StaleInstance)?;
        if active.plan.artifact_index().unwrap().config != request.expected.config
            || (active.selection.version != request.expected.selection
                && active.selection.version != version)
        {
            return Err(SelectionError::StaleVersion);
        }
        active.selection.nodes.insert(request.pool.clone(), actual);
        active.selection.version = version;
        self.commit_confirmed_selection()
    }
}
