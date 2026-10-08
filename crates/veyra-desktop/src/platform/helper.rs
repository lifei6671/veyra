//! 正式 helper 客户端；固定端点，无 prototype、环境变量或任意 path fallback。
use veyra_core::application::{
    helper_protocol::{Command, Configuration, Error, Response},
    manual_runtime::ManualRuntimeSnapshot,
    runtime_snapshot::RuntimeStatus,
    state_service::SnapshotService,
};

pub fn request(
    client: &veyra_helper::production::Client,
    snapshots: &SnapshotService,
    manual: &ManualRuntimeSnapshot,
    source_owner: Option<veyra_core::application::runtime_recovery::OwnerTransfer>,
    mut command: Command,
) -> Result<Response, Error> {
    if matches!(command, Command::Handoff { ref action } if !matches!(**action, veyra_core::application::helper_protocol::HandoffAction::Query))
    {
        return Err(Error::HandoffRequired); // 修改交接只能经RuntimeService::transfer_owner。
    }
    if matches!(command, Command::Selection { .. }) {
        return Err(Error::HandoffRequired);
    }
    if let Command::Bootstrap { action } = &command {
        use veyra_core::application::{
            helper_protocol::BootstrapAction, runtime_recovery::OwnerTransfer,
        };
        let BootstrapAction::Query { ticket } = &**action else {
            return Err(Error::HandoffRequired);
        };
        // 变更必须走同worker编排；只读查询也只能使用本地持久冻结的完整票据。
        if !matches!(&source_owner, Some(OwnerTransfer::BootstrapFrozen { id, installation, version })
            if *id == ticket.id && *installation == ticket.installation && *version == ticket.version)
            || manual.runtime.instance_id.is_some()
        {
            return Err(Error::HandoffRequired);
        }
    }
    let start = matches!(command, Command::Start { .. });
    if let Command::Start {
        config, expected, ..
    }
    | Command::Apply {
        config, expected, ..
    } = &mut command
    {
        // 不能让 IPC 参数取代桌面业务事实；完整模式切换及崩溃owner处置尚未闭合。
        if manual.runtime.status != RuntimeStatus::Stopped || manual.runtime.instance_id.is_some() {
            return Err(Error::HandoffRequired);
        }
        let state = snapshots.snapshot().map_err(|_| Error::Unavailable)?;
        // Start 预期当前业务版本；Apply 的 expected 是 helper 当前 applied，
        // candidate 自带的版本必须匹配本地刚读取的完整业务快照。
        if (start && state.version() != *expected)
            || (!start && state.version() != config.version())
        {
            return Err(Error::VersionConflict);
        }
        *config = Configuration {
            state: Box::new(state),
        };
        config.validate()?;
        use veyra_core::application::{
            helper_protocol::HandoffAction, runtime_recovery::OwnerTransfer,
        };
        if let Some(OwnerTransfer::BootstrapFrozen {
            id,
            installation,
            version,
        }) = &source_owner
        {
            if !start {
                return Err(Error::Unsupported);
            }
            if version != expected {
                return Err(Error::VersionConflict);
            }
            let ticket = veyra_core::application::helper_protocol::BootstrapTicket {
                id: id.clone(),
                installation: installation.clone(),
                version: version.clone(),
            };
            match client.request(Command::Bootstrap { action: Box::new(veyra_core::application::helper_protocol::BootstrapAction::Query { ticket: ticket.clone() }) })? {
                Response::Bootstrap(Ok(reply)) if reply.ticket == ticket && reply.phase == veyra_core::application::helper_protocol::BootstrapPhase::Committed => {},
                _ => return Err(Error::HandoffRequired),
            }
            match client.request(Command::Status)? {
                Response::Status(s) if !s.recovery_required => {}
                _ => return Err(Error::HandoffRequired),
            }
            return client.request(command);
        }
        let Some(OwnerTransfer::Released { ticket }) = source_owner else {
            return Err(Error::HandoffRequired);
        };
        match client.request(Command::Handoff {
            action: Box::new(HandoffAction::Query),
        })? {
            Response::Handoff(Ok(reply))
                if reply.owner == Some(OwnerTransfer::Local { ticket }) => {}
            _ => return Err(Error::HandoffRequired),
        }
        let Response::Status(status) = client.request(Command::Status)? else {
            return Err(Error::InvalidRequest);
        };
        if status.recovery_required {
            return Err(Error::HandoffRequired);
        }
        if start {
            if status.applied.as_ref().or(status.last_successful.as_ref()) != Some(expected) {
                return Err(Error::VersionConflict);
            }
        } else if status.applied.as_ref() != Some(expected) {
            return Err(Error::VersionConflict);
        }
    }
    client.request(command)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::manual_sidecar::ManualSidecarPort;
    use std::sync::Arc;
    use veyra_core::{
        application::{manual_runtime::ManualRuntime, state_access::StateAccessGate},
        domain::AppState,
        storage::JsonStateStore,
    };

    #[test]
    fn existing_manual_owner_is_never_stopped_for_helper_request() {
        // 保护模式切换不能为“接通测试”擅自接管旧 writer；仅合成 Runtime snapshot。
        let state = AppState::empty();
        let root = std::env::temp_dir().join(format!("v206-guard-{:?}", state.state_epoch));
        std::fs::create_dir(&root).unwrap();
        let store = JsonStateStore::new(root.join("state.json")).unwrap();
        store.commit(&state).unwrap();
        let snapshots = Arc::new(SnapshotService::new(store, StateAccessGate::default()));
        let runtime = ManualRuntime::new(
            ManualSidecarPort::new(root.clone()),
            snapshots.clone(),
            root.clone(),
            false,
        );
        let mut manual = runtime.snapshot().unwrap();
        manual.runtime.status = RuntimeStatus::Ready;
        let command = Command::Start {
            request_id: 1,
            expected: state.version(),
            config: Configuration {
                state: Box::new(state),
            },
        };
        assert_eq!(
            request(
                &veyra_helper::production::Client::default(),
                &snapshots,
                &manual,
                None,
                command
            ),
            Err(Error::HandoffRequired)
        );
        drop(runtime);
        std::fs::remove_dir_all(root).unwrap();
    }
}
