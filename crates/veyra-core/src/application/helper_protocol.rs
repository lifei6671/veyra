//! P2-06 受限权限宿主协议。只传业务事实，不传执行路径、PID、命令或系统字典。
use crate::{
    domain::{AppState, NodeId, OutboundId, PoolId, SnapshotVersion},
    singbox::{ProductCompileRequest, ProductRuntimeResources, SingBoxCompiler, SingBoxPlan},
};
use serde::{Deserialize, Serialize};

pub const PROTOCOL: u16 = 2;
/// 固定Desktop primary只读探针，不激活窗口。
pub const OWNER_PROBE: [u8; 2] = [1, 3];
pub const OWNER_ACK: [u8; 2] = [1, 4];
pub const CONTROL_BYTES: usize = 16 * 1024;
pub const CONFIG_BYTES: usize = 1024 * 1024;
/// 仅交接使用有界关闭cache/封闭plan；compiler仍无外部资源入口，不是任意文件上传。
pub const RESOURCE_BYTES: usize = super::runtime_recovery::HANDOFF_WIRE_BYTES;
pub const EVENT_BATCH: usize = 16;
pub const EVENT_CAPACITY: usize = 256;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Configuration {
    pub state: Box<AppState>,
}
impl std::fmt::Debug for Configuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Configuration([redacted])")
    }
}
impl Configuration {
    pub fn version(&self) -> SnapshotVersion {
        SnapshotVersion {
            config: self.state.config_version(),
            selection: self.state.selection_version(),
        }
    }
    pub fn validate(&self) -> Result<(), Error> {
        let bytes = serde_json::to_vec(self).map_err(|_| Error::InvalidRequest)?;
        if bytes.len() > CONFIG_BYTES {
            return Err(Error::TooLarge);
        }
        self.state
            .validate()
            .map_err(|_| Error::InvalidConfiguration)?;
        if self.state.nodes.len() > 4096 || self.state.pools.len() > 256 {
            return Err(Error::TooLarge);
        }
        // Start/Apply/交接配置不能携带pending；远程选择只能通过独立fence/读回/CAS合同。
        if self.state.pools.iter().any(|p| {
            matches!(
                &p.selection,
                crate::domain::SelectionPolicy::Manual {
                    pending_node_id: Some(_),
                    ..
                }
            )
        }) {
            return Err(Error::HandoffRequired);
        }
        Ok(())
    }
    /// 唯一产品 compiler；资源只能由当前 Runtime owner 提供。
    pub fn compile(&self, resources: &ProductRuntimeResources) -> Result<SingBoxPlan, Error> {
        self.validate()?;
        let projection = super::selected_subscription::project_selected_runtime(&self.state)
            .map_err(|_| Error::InvalidConfiguration)?;
        let outbound = OutboundId::from_route_target(&projection.projected_default_target)
            .map_err(|_| Error::InvalidConfiguration)?;
        SingBoxCompiler
            .compile_product(ProductCompileRequest {
                state: &self.state,
                runtime_intent: &projection.runtime_intent,
                default_outbound: &outbound,
                resources,
            })
            .map_err(|_| Error::InvalidConfiguration)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: u16,
    pub command: Command,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum Command {
    Hello,
    Bootstrap {
        action: Box<BootstrapAction>,
    },
    Selection {
        action: Box<RemoteSelectionAction>,
    },
    Handoff {
        action: Box<HandoffAction>,
    },
    Status,
    Operation {
        request_id: u64,
    },
    Start {
        request_id: u64,
        expected: SnapshotVersion,
        config: Configuration,
    },
    Apply {
        request_id: u64,
        instance: u64,
        expected: SnapshotVersion,
        config: Configuration,
    },
    Stop {
        request_id: u64,
        instance: u64,
    },
    RuntimeCommand {
        request_id: u64,
        instance: u64,
        expected: SnapshotVersion,
        command: RuntimeCommand,
    },
    RuntimeEvent {
        instance: u64,
        after: u64,
        limit: usize,
    },
    ApplySystemProxy {
        request_id: u64,
        instance: u64,
    },
    RestoreSystemProxy {
        request_id: u64,
        instance: u64,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum RuntimeCommand {
    Select { pool: PoolId, node: NodeId },
    Observe,
}
impl Command {
    pub fn request_id(&self) -> Option<u64> {
        match self {
            Self::Start { request_id, .. }
            | Self::Apply { request_id, .. }
            | Self::Stop { request_id, .. }
            | Self::RuntimeCommand { request_id, .. }
            | Self::ApplySystemProxy { request_id, .. }
            | Self::RestoreSystemProxy { request_id, .. } => Some(*request_id),
            _ => None,
        }
    }
    pub fn configuration(&self) -> Option<&Configuration> {
        match self {
            Self::Start { config, .. } | Self::Apply { config, .. } => Some(config),
            Self::Bootstrap { action } => match &**action {
                BootstrapAction::Preflight { config, .. }
                | BootstrapAction::Prepare { config, .. }
                | BootstrapAction::Commit { config, .. }
                | BootstrapAction::RebindPreflight { config, .. }
                | BootstrapAction::RebindPrepare { config, .. }
                | BootstrapAction::RebindCommit { config, .. } => Some(config),
                BootstrapAction::Query { .. } | BootstrapAction::RebindQuery { .. } => None,
            },
            Self::Selection { action } => match &**action {
                RemoteSelectionAction::Confirm { config, .. } => Some(config),
                _ => None,
            },
            Self::Handoff { action } => match &**action {
                HandoffAction::Preflight { config, .. } => Some(config),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn instance(&self) -> Option<u64> {
        match self {
            Self::Apply { instance, .. }
            | Self::Stop { instance, .. }
            | Self::RuntimeCommand { instance, .. }
            | Self::RuntimeEvent { instance, .. }
            | Self::ApplySystemProxy { instance, .. }
            | Self::RestoreSystemProxy { instance, .. } => Some(*instance),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Error {
    IncompatibleVersion,
    Unauthorized,
    UnsafeEndpoint,
    NotInstalled,
    Unavailable,
    InvalidRequest,
    InvalidConfiguration,
    TooLarge,
    Timeout,
    Busy,
    StaleInstance,
    VersionConflict,
    RequestConflict,
    HandoffRequired,
    Unsupported,
    Failed,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    pub protocol: u16,
    pub host_version: String,
    pub kernel_version: Option<String>,
    pub required_kernel_version: String,
    pub runtime: Result<(), Error>,
    /// 可预检交接不代表该会话已经获准启动。
    pub handoff: Result<(), Error>,
    pub system_proxy: bool,
    pub resource_bytes: usize,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Status {
    /// 只有实际 owner 完成 Ready/选择对账后才发布。IPC 本身不推断 Ready。
    pub instance: Option<u64>,
    pub applied: Option<SnapshotVersion>,
    pub last_successful: Option<SnapshotVersion>,
    pub recovery_required: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Operation {
    Inflight,
    Completed(Result<Status, Error>),
    /// TTL/LRU 淘汰或宿主重启后未知。先 Status，不能把未知当失败重放。
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Event {
    pub sequence: u64,
    pub status: Status,
    pub result: Result<(), Error>,
}
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Response {
    Hello(Capabilities),
    Bootstrap(Result<BootstrapReply, Error>),
    Handoff(Result<HandoffReply, Error>),
    Selection(Result<RemoteSelectionReply, Error>),
    Status(Status),
    Operation(Operation),
    Events { events: Vec<Event>, gap: bool },
    Rejected(Error),
}

/// 双向使用同一交接合同；只有已验证OS peer有权持有当前转移，payload不含身份/路径。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", deny_unknown_fields)]
pub enum HandoffAction {
    Query,
    Preflight {
        id: crate::domain::StateEpoch,
        config: Configuration,
    },
    Export {
        id: crate::domain::StateEpoch,
        expected: SnapshotVersion,
    },
    Prepare {
        config: Configuration,
        ticket: super::runtime_recovery::TransferTicket,
        bundle: Box<super::runtime_recovery::ClosedBundle>,
    },
    Release {
        ticket: super::runtime_recovery::TransferTicket,
    },
    Commit {
        release: super::runtime_recovery::OwnerTransfer,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HandoffReply {
    pub owner: Option<super::runtime_recovery::OwnerTransfer>,
    pub bundle: Option<Box<super::runtime_recovery::ClosedBundle>>,
}

/// 单个未完成远程选择；Execute重连只能复用结果，Query绝不发PUT。
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", deny_unknown_fields)]
pub enum RemoteSelectionAction {
    Execute {
        request: crate::storage::RemoteSelection,
    },
    Query {
        request: crate::storage::RemoteSelection,
    },
    Confirm {
        request: crate::storage::RemoteSelection,
        config: Configuration,
    },
}
impl RemoteSelectionAction {
    pub fn request(&self) -> &crate::storage::RemoteSelection {
        match self {
            Self::Execute { request } | Self::Query { request } | Self::Confirm { request, .. } => {
                request
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RemoteSelectionReply {
    pub request: crate::storage::RemoteSelection,
    /// None/third都不是确认；只接受当前child真实GET的稳定NodeId。
    pub actual: Option<NodeId>,
    pub confirmed: Option<SnapshotVersion>,
}

/// 首次安装合作让权；generation只由系统安装记录提供，不是客户端授权token。
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BootstrapTicket {
    pub id: crate::domain::StateEpoch,
    pub installation: crate::domain::StateEpoch,
    pub version: SnapshotVersion,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", deny_unknown_fields)]
pub enum BootstrapAction {
    RebindPreflight {
        id: crate::domain::StateEpoch,
        previous: BootstrapTicket,
        config: Configuration,
    },
    RebindPrepare {
        previous: BootstrapTicket,
        ticket: BootstrapTicket,
        config: Configuration,
    },
    RebindCommit {
        ticket: BootstrapTicket,
        config: Configuration,
    },
    RebindQuery {
        ticket: BootstrapTicket,
    },
    Preflight {
        id: crate::domain::StateEpoch,
        expected: SnapshotVersion,
        config: Configuration,
    },
    Prepare {
        ticket: BootstrapTicket,
        config: Configuration,
    },
    Query {
        ticket: BootstrapTicket,
    },
    Commit {
        ticket: BootstrapTicket,
        config: Configuration,
    },
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BootstrapPhase {
    Eligible,
    Prepared,
    Committed,
    RecoveryRequired,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BootstrapReply {
    pub ticket: BootstrapTicket,
    pub phase: BootstrapPhase,
}

/// 新Primary的有界申请，保留旧Frozen/incarnation，不等于已转移owner。
#[derive(Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RebindPending {
    pub previous: BootstrapTicket,
    pub ticket: BootstrapTicket,
    pub session: super::runtime_recovery::SourceSession,
}
