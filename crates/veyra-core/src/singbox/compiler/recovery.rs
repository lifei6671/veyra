//! 封闭的 pre-finalize 恢复格式；tag 只在 Compiler/Runtime adapter 中流转。
use super::*;
use crate::domain::{AppState, ConfigVersion, PoolId, SelectionVersion};
use std::collections::BTreeMap;

pub const RECOVERY_FORMAT: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AppliedPool {
    pub runtime_tag: String,
    pub members: BTreeMap<NodeId, String>,
    /// Compiler 排序后的首成员，None selection 使用这一明确默认值。
    pub default_node: NodeId,
    pub selected_node: Option<NodeId>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AppliedArtifactIndex {
    pub config: ConfigVersion,
    pub selection: SelectionVersion,
    pub pools: BTreeMap<PoolId, AppliedPool>,
    pub nodes: BTreeMap<NodeId, String>,
    /// 组 selector 可含节点、组或 Direct，保持用户顺序；旧节点池选择契约不变。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub groups: BTreeMap<PoolId, Vec<OutboundId>>,
}
impl AppliedArtifactIndex {
    pub(super) fn compile(state: &AppState, intent: &RuntimeIntent) -> Self {
        let pools = intent
            .pools
            .iter()
            .filter_map(|pool| {
                let SelectionPolicy::Manual {
                    selected_node_id, ..
                } = &pool.selection
                else {
                    return None;
                };
                let members: BTreeMap<_, _> = pool
                    .members
                    .iter()
                    .map(|id| (id.clone(), node_tag(&id.0)))
                    .collect();
                // 与 selector 的 runtime tag 排序一致，禁止由显示名猜测。
                let default_node = members
                    .iter()
                    .min_by_key(|(_, tag)| *tag)
                    .expect("validated nonempty pool")
                    .0
                    .clone();
                Some((
                    pool.id.clone(),
                    AppliedPool {
                        runtime_tag: pool_tag(&pool.id.0),
                        members,
                        default_node,
                        selected_node: selected_node_id.clone(),
                    },
                ))
            })
            .collect();
        Self {
            config: state.config_version(),
            selection: state.selection_version(),
            pools,
            groups: intent
                .groups
                .iter()
                .filter(|g| matches!(g.selection, SelectionPolicy::Manual { .. }))
                .map(|g| (g.id.clone(), g.members.clone()))
                .collect(),
            nodes: intent
                .nodes
                .iter()
                .map(|n| (n.id.clone(), node_tag(&n.id.0)))
                .collect(),
        }
    }
    fn validate(&self, document: &Document) -> Result<(), CompileError> {
        let invalid = CompileError::InvalidFinalConfiguration;
        if self.config.0.epoch != self.selection.0.epoch {
            return Err(invalid);
        }
        let actual_nodes: BTreeSet<_> = document
            .outbounds
            .iter()
            .filter_map(|o| o.domain().ok().flatten().map(|_| o.tag().to_owned()))
            .chain(document.endpoints.iter().map(|e| e.tag.clone()))
            .collect();
        if self.nodes.iter().any(|(id, tag)| *tag != node_tag(&id.0))
            || self.nodes.values().cloned().collect::<BTreeSet<_>>() != actual_nodes
        {
            return Err(invalid);
        }
        let selectors: Vec<_> = document
            .outbounds
            .iter()
            .filter_map(|v| {
                if let CoreOutbound::Selector(s) = v {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();
        if selectors.len() != self.pools.len() + self.groups.len() {
            return Err(invalid);
        }
        for (id, members) in &self.groups {
            let selector = selectors
                .iter()
                .find(|s| s.tag == pool_tag(&id.0))
                .ok_or(invalid)?;
            if self.pools.contains_key(id)
                || members.is_empty()
                || selector.default.is_some()
                || selector.outbounds != members.iter().map(outbound_tag).collect::<Vec<_>>()
            {
                return Err(invalid);
            }
        }
        for (id, pool) in &self.pools {
            let selector = selectors
                .iter()
                .find(|s| s.tag == pool.runtime_tag)
                .ok_or(invalid)?;
            let tags: BTreeSet<_> = pool.members.values().cloned().collect();
            if pool.runtime_tag != pool_tag(&id.0)
                || pool.members.is_empty()
                || !pool.members.contains_key(&pool.default_node)
                || pool.members.iter().any(|(id, tag)| *tag != node_tag(&id.0))
                || tags != selector.outbounds.iter().cloned().collect()
                || pool
                    .members
                    .iter()
                    .any(|(id, tag)| self.nodes.get(id) != Some(tag))
                || pool
                    .selected_node
                    .as_ref()
                    .is_some_and(|id| !pool.members.contains_key(id))
                || selector.default != pool.selected_node.as_ref().map(|id| node_tag(&id.0))
                || pool.members.get(&pool.default_node) != selector.outbounds.first()
            {
                return Err(invalid);
            }
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryPlan {
    format: u32,
    document: Document,
    health: RuntimeHealthPlan,
    index: AppliedArtifactIndex,
}
impl SingBoxPlan {
    pub fn artifact_index(&self) -> Option<&AppliedArtifactIndex> {
        self.index.as_ref()
    }
    /// 凭据只落私有 recovery artifact；没有 secret、PID 或已分配端口。
    pub fn recovery_bytes(&self) -> Result<Vec<u8>, CompileError> {
        self.document.validate(false)?;
        let index = self
            .index
            .clone()
            .ok_or(CompileError::InvalidFinalConfiguration)?;
        index.validate(&self.document)?;
        let mut document = self.document.clone();
        document.experimental.clash_api.external_controller = "127.0.0.1:0".into();
        for inbound in &mut document.inbounds {
            match inbound {
                CoreInbound::Mixed(v) => v.listen_port = 0,
                #[cfg(any(test, feature = "legacy-test-support"))]
                CoreInbound::Test(_) => return Err(CompileError::InvalidFinalConfiguration),
            }
        }
        serde_json::to_vec(&RecoveryPlan {
            format: RECOVERY_FORMAT,
            document,
            health: self
                .health
                .clone()
                .ok_or(CompileError::InvalidFinalConfiguration)?,
            index,
        })
        .map_err(|_| CompileError::SerializationFailed)
    }
    /// 读回同一 deny_unknown 封闭模型并验证，然后仅绑定 owner 提供的受管资源。
    pub fn recover(
        bytes: &[u8],
        resources: &ProductRuntimeResources,
    ) -> Result<Self, CompileError> {
        let mut value: RecoveryPlan =
            serde_json::from_slice(bytes).map_err(|_| CompileError::InvalidFinalConfiguration)?;
        let raw: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| CompileError::InvalidFinalConfiguration)?;
        if raw != serde_json::to_value(&value).map_err(|_| CompileError::SerializationFailed)? {
            return Err(CompileError::InvalidFinalConfiguration);
        }
        if value.format != RECOVERY_FORMAT
            || value.document.experimental.cache_file.is_none()
            || value.document.experimental.clash_api.external_controller != "127.0.0.1:0"
            || value
                .document
                .inbounds
                .iter()
                .any(|v| !matches!(v, CoreInbound::Mixed(m) if m.listen_port == 0))
        {
            return Err(CompileError::InvalidFinalConfiguration);
        }
        value.document.validate(false)?;
        value.index.validate(&value.document)?;
        let cache = value.document.experimental.cache_file.as_mut().unwrap();
        if cache.cache_id != resources.cache.cache_id() {
            return Err(CompileError::InvalidRuntimeResources);
        }
        cache.path = resources.cache.path().to_owned();
        value.document.experimental.clash_api.external_controller =
            resources.controller.address().to_string();
        for inbound in &mut value.document.inbounds {
            match inbound {
                CoreInbound::Mixed(v) => v.listen_port = resources.mixed.address().port(),
                #[cfg(any(test, feature = "legacy-test-support"))]
                CoreInbound::Test(_) => return Err(CompileError::InvalidFinalConfiguration),
            }
        }
        value.document.validate(false)?;
        Ok(Self {
            document: value.document,
            health: Some(value.health),
            index: Some(value.index),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (SingBoxPlan, ProductRuntimeResources) {
        let mut value = serde_json::to_value(AppState::empty()).unwrap();
        let parts: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/compiler/p2-02b-selected.json"
        ))
        .unwrap();
        for (k, v) in parts.as_object().unwrap() {
            value[k] = v.clone();
        }
        let state: AppState = serde_json::from_value(value).unwrap();
        let projection =
            crate::application::selected_subscription::project_selected_runtime(&state).unwrap();
        let resources = ProductRuntimeResources {
            mixed: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
            controller: LoopbackListener::new("127.0.0.1:0".parse().unwrap()).unwrap(),
            cache: ManagedCacheFile::new(
                std::path::Path::new("/tmp/veyra-p204-plan"),
                "/tmp/veyra-p204-plan/cache.db".into(),
                "fixed-generation".into(),
                false,
                false,
            )
            .unwrap(),
        };
        let target = OutboundId::from_route_target(&projection.projected_default_target).unwrap();
        (
            SingBoxCompiler
                .compile_product(ProductCompileRequest {
                    state: &state,
                    runtime_intent: &projection.runtime_intent,
                    default_outbound: &target,
                    resources: &resources,
                })
                .unwrap(),
            resources,
        )
    }
    // 保护首次导入 synthetic pool 的index，以及恢复计划确实不含已绑定secret/port。
    #[test]
    fn closed_plan_roundtrip_and_projected_index() {
        let (plan, resources) = fixture();
        let bytes = plan.recovery_bytes().unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(v["document"]["experimental"]["clash_api"]["secret"], "");
        assert_eq!(v["document"]["inbounds"][0]["listen_port"], 0);
        let index = plan.artifact_index().unwrap();
        assert!(
            index
                .pools
                .keys()
                .any(|p| p.0.starts_with("runtime-active-"))
        );
        assert_eq!(index.nodes.len(), 3);
        let restored = SingBoxPlan::recover(&bytes, &resources).unwrap();
        assert_eq!(restored.artifact_index(), Some(index));
        let first = restored
            .finalize(&crate::singbox::secret::generate_api_secret().unwrap())
            .unwrap();
        let next = restored
            .finalize(&crate::singbox::secret::generate_api_secret().unwrap())
            .unwrap();
        assert_ne!(first.as_bytes(), next.as_bytes());
        assert_eq!(restored.runtime_health(), plan.runtime_health());
    }
    // 保护磁盘schema/closed model/index校验，不能通过合法JSON注入任意配置或runtime tag。
    #[test]
    fn recovery_rejects_unknown_fields_bound_ports_and_inconsistent_index() {
        let (plan, resources) = fixture();
        let bytes = plan.recovery_bytes().unwrap();
        let base: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        for kind in [
            "root", "nested", "version", "tag", "member", "format", "port", "secret",
        ] {
            let mut v = base.clone();
            match kind {
                "root" => v["extra"] = true.into(),
                "nested" => v["document"]["experimental"]["cache_file"]["extra"] = true.into(),
                "version" => v["index"]["config"]["extra"] = true.into(),
                "tag" => {
                    let pool = v["index"]["pools"]
                        .as_object_mut()
                        .unwrap()
                        .values_mut()
                        .next()
                        .unwrap();
                    pool["runtime_tag"] = "arbitrary".into();
                }
                "member" => v["index"]["nodes"]["a"] = "wrong".into(),
                "format" => v["format"] = 2.into(),
                "port" => v["document"]["inbounds"][0]["listen_port"] = 12345.into(),
                "secret" => {
                    v["document"]["experimental"]["clash_api"]["secret"] = "old-secret".into()
                }
                _ => unreachable!(),
            }
            assert!(
                SingBoxPlan::recover(&serde_json::to_vec(&v).unwrap(), &resources).is_err(),
                "{kind}"
            );
        }
        let mut incompatible = resources.clone();
        incompatible.cache = ManagedCacheFile::new(
            std::path::Path::new("/tmp/veyra-p204-plan"),
            "/tmp/veyra-p204-plan/other.db".into(),
            "incompatible".into(),
            false,
            false,
        )
        .unwrap();
        assert!(SingBoxPlan::recover(&bytes, &incompatible).is_err());
    }
}
