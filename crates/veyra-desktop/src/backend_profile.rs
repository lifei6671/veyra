//! Backend 的五字段保存队列。只有 ProfileService 是 writer；队列不执行 Runtime 操作。
use std::collections::VecDeque;
use veyra_core::domain::{
    AppError, AppState, ConfigVersion, Patch, ProfilePatch, RuntimeHealthUrl,
};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Field {
    DirectForNodes,
    RejectQuic,
    Ipv6,
    TestUrl,
    DirectTestUrl,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Change {
    DirectForNodes(bool),
    RejectQuic(bool),
    Ipv6(bool),
    TestUrl(String),
    DirectTestUrl(String),
}
impl Change {
    pub fn field(&self) -> Field {
        match self {
            Self::DirectForNodes(_) => Field::DirectForNodes,
            Self::RejectQuic(_) => Field::RejectQuic,
            Self::Ipv6(_) => Field::Ipv6,
            Self::TestUrl(_) => Field::TestUrl,
            Self::DirectTestUrl(_) => Field::DirectTestUrl,
        }
    }
    pub fn patch(&self) -> Result<ProfilePatch, AppError> {
        let mut patch = ProfilePatch::default();
        match self {
            Self::DirectForNodes(v) => patch.direct_for_nodes = Patch::Value(*v),
            Self::RejectQuic(v) => patch.reject_quic = Patch::Value(*v),
            Self::Ipv6(v) => patch.ipv6 = Patch::Value(*v),
            Self::TestUrl(v) => patch.test_url = Patch::Value(RuntimeHealthUrl::new(v.clone())?),
            Self::DirectTestUrl(v) => {
                patch.direct_test_url = Patch::Value(RuntimeHealthUrl::new(v.clone())?)
            }
        }
        Ok(patch)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Request {
    pub id: u64,
    pub generation: u64,
    pub expected: ConfigVersion,
    pub change: Change,
}
#[derive(Default)]
pub struct Saves {
    pub active: Option<Request>,
    queued: VecDeque<Change>,
    generation: u64,
    next: u64,
    pub state: Option<Box<AppState>>,
}
impl Saves {
    pub fn project(&mut self, state: &AppState) {
        if self
            .state
            .as_ref()
            .is_some_and(|s| s.state_epoch != state.state_epoch)
        {
            self.leave();
        }
        self.state = Some(Box::new(state.clone()));
    }
    pub fn leave(&mut self) {
        self.generation += 1;
        self.active = None;
        self.queued.clear();
    }
    pub fn busy(&self, field: Field) -> bool {
        self.active
            .as_ref()
            .is_some_and(|r| r.change.field() == field)
            || self.queued.iter().any(|c| c.field() == field)
    }
    pub fn changes(&self) -> impl Iterator<Item = &Change> {
        self.active
            .iter()
            .map(|r| &r.change)
            .chain(self.queued.iter())
    }
    pub fn enqueue(&mut self, change: Change) -> Option<Request> {
        if self.busy(change.field()) || self.state.is_none() {
            return None;
        }
        self.queued.push_back(change);
        self.next()
    }
    fn next(&mut self) -> Option<Request> {
        if self.active.is_some() {
            return None;
        }
        let state = self.state.as_ref()?;
        let change = self.queued.pop_front()?;
        self.next += 1;
        let request = Request {
            id: self.next,
            generation: self.generation,
            expected: state.config_version(),
            change,
        };
        self.active = Some(request.clone());
        Some(request)
    }
    pub fn accepts(&self, request: &Request) -> bool {
        self.active.as_ref() == Some(request)
    }
    pub fn complete(&mut self, request: &Request, success: bool) -> Option<Request> {
        if !self.accepts(request) {
            return None;
        }
        self.active = None;
        // CAS/validation/storage 失败均不自动重试，用户明确重新操作。
        if !success {
            self.queued.clear();
        }
        self.next()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use veyra_core::{
        application::{
            state_access::StateAccessGate,
            state_service::{ApplyEffect, ProfileService, SnapshotService},
        },
        domain::AppErrorCode,
        storage::JsonStateStore,
    };
    // 五类控件都必须通过既有 writer 保存，拒绝无效 URL/CAS，不能产生 Applied。
    #[test]
    fn five_fields_persist_saved_only_and_reject_invalid_and_stale() {
        let root = std::env::temp_dir().join(format!(
            "veyra-backend-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let snapshots = SnapshotService::new(
            JsonStateStore::new(root.join("state.json")).unwrap(),
            StateAccessGate::default(),
        );
        let profiles = ProfileService::new(snapshots.clone());
        let initial = snapshots.snapshot().unwrap();
        let changes = [
            Change::DirectForNodes(false),
            Change::RejectQuic(true),
            Change::Ipv6(true),
            Change::TestUrl("http://127.0.0.1:3001/proxy".into()),
            Change::DirectTestUrl("http://127.0.0.1:3001/direct".into()),
        ];
        for change in changes {
            let before = snapshots.snapshot().unwrap();
            let outcome = profiles
                .patch(before.config_version(), change.patch().unwrap())
                .unwrap();
            assert_eq!(outcome.effect, ApplyEffect::SavedOnly);
            let after = snapshots.snapshot().unwrap();
            assert_eq!(after.profile, outcome.value);
            assert_eq!(after.config_revision, before.config_revision + 1);
        }
        let saved = snapshots.snapshot().unwrap();
        assert!(!saved.profile.direct_for_nodes && saved.profile.reject_quic && saved.profile.ipv6);
        assert_eq!(
            saved.profile.test_url.as_str(),
            "http://127.0.0.1:3001/proxy"
        );
        assert_eq!(
            saved.profile.direct_test_url.as_str(),
            "http://127.0.0.1:3001/direct"
        );
        for invalid in ["", "not a URL", "file:///tmp/example"] {
            assert!(Change::TestUrl(invalid.into()).patch().is_err());
            assert!(Change::DirectTestUrl(invalid.into()).patch().is_err());
        }
        assert_eq!(
            profiles
                .patch(
                    initial.config_version(),
                    Change::RejectQuic(false).patch().unwrap()
                )
                .unwrap_err()
                .code(),
            AppErrorCode::RevisionConflict
        );
        assert_eq!(snapshots.snapshot().unwrap().profile, saved.profile);
        std::fs::remove_dir_all(root).unwrap();
    }
    // 快速跨字段操作排队，第二次基于首个成功的版本；旧回调不能清空当前 busy。
    #[test]
    fn rapid_fields_are_serial_and_old_completion_cannot_clear_new_busy() {
        let mut saves = Saves::default();
        let mut state = AppState::empty();
        saves.project(&state);
        let a = saves.enqueue(Change::DirectForNodes(true)).unwrap();
        assert!(saves.enqueue(Change::RejectQuic(true)).is_none());
        assert!(saves.busy(Field::DirectForNodes) && saves.busy(Field::RejectQuic));
        state.config_revision += 1;
        saves.project(&state);
        let b = saves.complete(&a, true).unwrap();
        assert_eq!(b.expected, state.config_version());
        assert!(saves.complete(&a, true).is_none());
        assert!(saves.busy(Field::RejectQuic));
        saves.complete(&b, true);
        assert!(!saves.busy(Field::RejectQuic));
    }
    // 离页、epoch 替换及失败都不得自动提交先前的草稿。
    #[test]
    fn leaving_epoch_replacement_and_failure_cancel_queued_drafts() {
        let mut saves = Saves::default();
        saves.project(&AppState::empty());
        let a = saves.enqueue(Change::Ipv6(true)).unwrap();
        saves.leave();
        let b = saves.enqueue(Change::Ipv6(false)).unwrap();
        saves.complete(&a, true);
        assert!(saves.accepts(&b));
        saves.enqueue(Change::RejectQuic(true));
        assert!(saves.complete(&b, false).is_none());
        assert!(!saves.busy(Field::RejectQuic));
        let c = saves.enqueue(Change::Ipv6(true)).unwrap();
        saves.project(&AppState::empty());
        assert!(!saves.accepts(&c));
    }
    // 验证 Desktop writer 与既有 Runtime owner 的集成：保存不应用，显式重启才推进 applied。
    #[test]
    fn profile_save_preserves_runtime_applied_until_explicit_restart() {
        use std::sync::Arc;
        use veyra_core::{
            application::manual_runtime::{ManualRuntime, RuntimeCommand},
            singbox::{
                GeneratedConfig,
                runtime::{ManagedRuntimeEndpoints, ManagedSidecar, SidecarPort, SidecarPortError},
            },
            storage::StateStore,
        };
        #[derive(Default)]
        struct MockPort {
            next: u64,
            active: Option<u64>,
            config: Option<serde_json::Value>,
            selectors: std::collections::BTreeMap<String, String>,
        }
        impl SidecarPort for MockPort {
            fn read_selector(
                &mut self,
                _: &ManagedSidecar,
                tag: &str,
            ) -> Result<String, SidecarPortError> {
                self.selectors.get(tag).cloned().ok_or(SidecarPortError)
            }
            fn write_selector(
                &mut self,
                _: &ManagedSidecar,
                tag: &str,
                node: &str,
            ) -> Result<(), SidecarPortError> {
                self.selectors.insert(tag.into(), node.into());
                Ok(())
            }
            fn check(&mut self, config: &GeneratedConfig) -> Result<(), SidecarPortError> {
                self.config = Some(serde_json::from_slice(config.as_bytes()).unwrap());
                Ok(())
            }
            fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
                Ok(())
            }
            fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
                let c = self.config.as_ref().unwrap();
                for pool in c["outbounds"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| v["type"] == "selector")
                {
                    self.selectors.insert(
                        pool["tag"].as_str().unwrap().into(),
                        pool["default"]
                            .as_str()
                            .unwrap_or(pool["outbounds"][0].as_str().unwrap())
                            .into(),
                    );
                }
                let cache =
                    std::path::Path::new(c["experimental"]["cache_file"]["path"].as_str().unwrap());
                std::fs::write(cache, b"mock-opaque-cache").unwrap();
                {
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(cache, std::fs::Permissions::from_mode(0o600))
                        .unwrap();
                }
                self.next += 1;
                self.active = Some(self.next);
                Ok(ManagedSidecar::from_port_identity(self.next))
            }
            fn ready(&mut self, i: &ManagedSidecar) -> Result<(), SidecarPortError> {
                assert_eq!(self.active, Some(i.identity()));
                Ok(())
            }
            fn stop(&mut self, i: &ManagedSidecar) -> Result<(), SidecarPortError> {
                assert_eq!(self.active, Some(i.identity()));
                self.active = None;
                Ok(())
            }
            fn endpoints(&self, i: &ManagedSidecar) -> Option<ManagedRuntimeEndpoints> {
                (self.active == Some(i.identity())).then_some(ManagedRuntimeEndpoints {
                    mixed: "127.0.0.1:23001".parse().unwrap(),
                    controller: "127.0.0.1:23002".parse().unwrap(),
                })
            }
            fn is_alive(&mut self, i: &ManagedSidecar) -> Result<bool, SidecarPortError> {
                Ok(self.active == Some(i.identity()))
            }
            fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
                Ok(())
            }
            fn has_pending_cleanup(&self) -> bool {
                false
            }
        }
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../veyra-core/tests/fixtures/compiler/p2-02b-selected.json"
        ))
        .unwrap();
        for (k, v) in fixture.as_object().unwrap() {
            value[k] = v.clone();
        }
        let state: AppState = serde_json::from_value(value).unwrap();
        let root =
            std::env::temp_dir().join(format!("veyra-backend-runtime-{:?}", state.state_epoch));
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.save(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let profiles = ProfileService::new((*snapshots).clone());
        let mut runtime =
            ManualRuntime::new(MockPort::default(), snapshots.clone(), root.clone(), true);
        runtime.execute(RuntimeCommand::Start, |_| {}).unwrap();
        let before = runtime.snapshot().unwrap();
        let saved = profiles
            .patch(
                state.config_version(),
                Change::RejectQuic(!state.profile.reject_quic)
                    .patch()
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(saved.effect, ApplyEffect::SavedOnly);
        let pending = runtime.snapshot().unwrap();
        assert_eq!(
            pending.runtime.applied_version,
            before.runtime.applied_version
        );
        assert_eq!(pending.runtime.instance_id, before.runtime.instance_id);
        assert_eq!(pending.runtime.saved_version, saved.version);
        runtime.execute(RuntimeCommand::Restart, |_| {}).unwrap();
        let after = runtime.snapshot().unwrap();
        assert_eq!(after.runtime.applied_version, Some(saved.version));
        assert_ne!(after.runtime.instance_id, before.runtime.instance_id);
        runtime.execute(RuntimeCommand::Stop, |_| {}).unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
}
