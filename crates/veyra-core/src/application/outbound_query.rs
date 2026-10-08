use crate::domain::{
    AppError, AppState, OutboundCatalog, OutboundEntry, OutboundId, OutboundResolveError,
    RouteTarget, SnapshotVersion,
};

use super::state_service::SnapshotService;

/// P2-08 可直接消费的查询快照：同一版本上的 list/get/resolve，不持有可编辑副本。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutboundQuerySnapshot {
    pub version: SnapshotVersion,
    catalog: OutboundCatalog,
}

impl OutboundQuerySnapshot {
    pub fn from_state(state: &AppState) -> Self {
        Self {
            version: state.version(),
            catalog: OutboundCatalog::from_state(state),
        }
    }
    pub fn list(&self) -> &[OutboundEntry] {
        self.catalog.list()
    }
    pub fn get(&self, id: &OutboundId) -> Result<&OutboundEntry, OutboundResolveError> {
        self.catalog.get(id)
    }
    pub fn resolve(&self, target: &RouteTarget) -> Result<OutboundId, OutboundResolveError> {
        self.catalog.resolve(target)
    }
}

/// 复用既有快照事务和 gate，每次查询读取当前事实；不增加持久化或后台缓存。
pub struct OutboundQueryService {
    snapshots: SnapshotService,
}
impl OutboundQueryService {
    pub fn new(snapshots: SnapshotService) -> Self {
        Self { snapshots }
    }
    pub fn snapshot(&self) -> Result<OutboundQuerySnapshot, AppError> {
        Ok(OutboundQuerySnapshot::from_state(
            &self.snapshots.snapshot()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::state_access::StateAccessGate,
        domain::StateEpoch,
        storage::{JsonStateStore, StateStore},
    };

    // 保护查询不写入第二份出口事实、读取当前版本，且默认未配置不能伪装成 Direct。
    #[test]
    fn query_reads_current_snapshot_without_changing_schema_or_state_bytes() {
        let root =
            std::env::temp_dir().join(format!("veyra-p2-02a-{:?}", StateEpoch::fresh().unwrap()));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        let mut state = AppState::empty();
        store.save(&state).unwrap();
        let service = OutboundQueryService::new(SnapshotService::new(
            store.clone(),
            StateAccessGate::default(),
        ));
        let before = std::fs::read(root.join("state.json")).unwrap();
        let json_before = serde_json::to_vec(&state).unwrap();
        let query = service.snapshot().unwrap();
        assert_eq!(query.list().len(), 2);
        assert_eq!(
            query.resolve(&RouteTarget::Unconfigured),
            Err(OutboundResolveError::Unconfigured)
        );
        assert_eq!(query.resolve(&RouteTarget::Direct), Ok(OutboundId::Direct));
        assert_eq!(query.get(&OutboundId::Block).unwrap().id, OutboundId::Block);
        assert_eq!(query.version, state.version());
        assert_eq!(std::fs::read(root.join("state.json")).unwrap(), before);
        assert_eq!(serde_json::to_vec(&state).unwrap(), json_before);
        assert_eq!(state.schema_version, 9);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&json_before)
                .unwrap()
                .as_object()
                .unwrap()
                .len(),
            14
        );
        state.default_target = RouteTarget::Block;
        let saved = store.commit(&state).unwrap();
        let current_bytes = std::fs::read(root.join("state.json")).unwrap();
        let current = service.snapshot().unwrap();
        assert_eq!(current.version, saved.version());
        assert_ne!(current.version, query.version);
        assert_eq!(
            std::fs::read(root.join("state.json")).unwrap(),
            current_bytes
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
