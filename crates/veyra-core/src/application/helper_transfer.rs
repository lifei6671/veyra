//! Desktop串行Runtime worker的双向握手。不持有StateAccessGate等待IPC，也不自动启动目标。
use super::{helper_protocol::*, manual_runtime::ManualRuntime, runtime_recovery::*};
use crate::{
    domain::{SnapshotVersion, StateEpoch},
    singbox::runtime::SidecarPort,
};
#[derive(Clone, Copy, Debug)]
pub enum TransferDirection {
    ToHelper,
    ToDesktop,
}
pub fn transfer<P: SidecarPort>(
    local: &mut ManualRuntime<P>,
    id: StateEpoch,
    expected: SnapshotVersion,
    direction: TransferDirection,
    mut exchange: impl FnMut(HandoffAction) -> Result<HandoffReply, Error>,
) -> Result<OwnerTransfer, Error> {
    // 双向交接都不能越过尚未完成的远程选择，包含桌面已CAS但helper未回告的窗口。
    let state = local
        .preflight_transfer(&expected)
        .map_err(|_| Error::HandoffRequired)?;
    if state.version() != expected {
        return Err(Error::VersionConflict);
    }
    match direction {
        TransferDirection::ToHelper => {
            local
                .preflight_handoff(&expected)
                .map_err(|_| Error::HandoffRequired)?;
            let config = Configuration {
                state: Box::new(state),
            };
            config.validate()?;
            if config.version() != expected {
                return Err(Error::VersionConflict);
            }
            #[cfg(target_os = "macos")]
            local
                .publish_source_session()
                .map_err(|_| Error::HandoffRequired)?;
            // 客户端在此完成固定endpoint/OS peer/Hello核验；目标只读检查可接入。
            // 失败不会写本地fence或停止手动child。预检不承诺后续执行不会失败。
            exchange(HandoffAction::Preflight {
                id: id.clone(),
                config: config.clone(),
            })?;
            let (ticket, bundle) = local
                .prepare_handoff(id, expected.clone())
                .map_err(|_| Error::HandoffRequired)?;
            let prepared = exchange(HandoffAction::Prepare {
                config,
                ticket: ticket.clone(),
                bundle: Box::new(bundle),
            })?;
            if !matches!(prepared.owner,Some(OwnerTransfer::Prepared{ticket:ref t}|OwnerTransfer::Local{ticket:ref t}) if *t==ticket)
            {
                return Err(Error::HandoffRequired);
            }
            let release = local
                .release_handoff(&ticket)
                .map_err(|_| Error::HandoffRequired)?;
            let committed = exchange(HandoffAction::Commit {
                release: release.clone(),
            })?;
            if committed.owner != Some(OwnerTransfer::Local { ticket }) {
                return Err(Error::HandoffRequired);
            }
            Ok(release)
        }
        TransferDirection::ToDesktop => {
            let exported = exchange(HandoffAction::Export {
                id,
                expected: expected.clone(),
            })?;
            let ticket = match exported.owner {
                Some(OwnerTransfer::Closed { ticket } | OwnerTransfer::Released { ticket })
                    if ticket.version == expected =>
                {
                    ticket
                }
                _ => return Err(Error::HandoffRequired),
            };
            local
                .receive_handoff(&ticket, &*exported.bundle.ok_or(Error::InvalidRequest)?)
                .map_err(|_| Error::HandoffRequired)?;
            let released = exchange(HandoffAction::Release {
                ticket: ticket.clone(),
            })?;
            let release = released.owner.ok_or(Error::InvalidRequest)?;
            if release
                != (OwnerTransfer::Released {
                    ticket: ticket.clone(),
                })
            {
                return Err(Error::HandoffRequired);
            }
            local
                .commit_handoff(&release)
                .map_err(|_| Error::HandoffRequired)?;
            Ok(OwnerTransfer::Local { ticket })
        }
    }
}

/// RPC失败不释放持久fence。重连先Query，只有明确同一请求的实际读回才能CAS。
pub fn select_remote(
    snapshots: &super::state_service::SnapshotService,
    request: crate::storage::RemoteSelection,
    mut exchange: impl FnMut(RemoteSelectionAction) -> Result<RemoteSelectionReply, Error>,
) -> Result<SnapshotVersion, Error> {
    let retry = snapshots
        .remote_selection_fence()
        .map_err(|_| Error::Unavailable)?
        .is_some();
    let fence = snapshots
        .begin_remote_selection(&request)
        .map_err(|_| Error::VersionConflict)?;
    let result = exchange(if retry {
        RemoteSelectionAction::Query {
            request: request.clone(),
        }
    } else {
        RemoteSelectionAction::Execute {
            request: request.clone(),
        }
    })?;
    if result.request != request {
        return Err(Error::RequestConflict);
    }
    let actual = result.actual.ok_or(Error::HandoffRequired)?;
    let state = snapshots
        .confirm_remote_selection(&fence, &actual)
        .map_err(|_| Error::VersionConflict)?;
    let version = state.version();
    let ack = exchange(RemoteSelectionAction::Confirm {
        request: request.clone(),
        config: Configuration {
            state: Box::new(state),
        },
    })?;
    if ack.request != request
        || ack.actual.as_ref() != Some(&actual)
        || ack.confirmed.as_ref() != Some(&version)
    {
        return Err(Error::VersionConflict);
    }
    snapshots
        .finish_remote_selection(&fence, &version)
        .map_err(|_| Error::Unavailable)?;
    if actual != request.node {
        return Err(Error::Failed);
    } // 已读回旧值并clear，不把选择失败报告成成功。
    Ok(version)
}

/// 首次bootstrap的有限编排：先只读目标预检，再本地freeze，再root Prepared。
/// Prepared不代表可Start；失败保留本地冻结，显式重试仍使用同一id/安装世代。
pub fn prepare_bootstrap<P: SidecarPort>(
    local: &mut ManualRuntime<P>,
    id: StateEpoch,
    expected: SnapshotVersion,
    mut exchange: impl FnMut(BootstrapAction) -> Result<BootstrapReply, Error>,
) -> Result<BootstrapReply, Error> {
    let state = local
        .preflight_transfer(&expected)
        .map_err(|_| Error::HandoffRequired)?;
    let config = Configuration {
        state: Box::new(state),
    };
    config.validate()?;
    let ticket = match local.owner_transfer().map_err(|_| Error::HandoffRequired)? {
        None => {
            let reply = exchange(BootstrapAction::Preflight {
                id: id.clone(),
                expected: expected.clone(),
                config: config.clone(),
            })?;
            if reply.phase != BootstrapPhase::Eligible
                || reply.ticket.id != id
                || reply.ticket.version != expected
            {
                return Err(Error::VersionConflict);
            }
            reply.ticket
        }
        Some(OwnerTransfer::BootstrapFrozen {
            id: old,
            installation,
            version,
        }) if old == id && version == expected => BootstrapTicket {
            id: old,
            installation,
            version,
        },
        _ => return Err(Error::HandoffRequired),
    };
    local
        .freeze_first_bootstrap(
            ticket.id.clone(),
            ticket.installation.clone(),
            ticket.version.clone(),
        )
        .map_err(|_| Error::HandoffRequired)?;
    local
        .preflight_transfer(&expected)
        .map_err(|_| Error::VersionConflict)?;
    let reply = exchange(BootstrapAction::Prepare {
        ticket: ticket.clone(),
        config,
    })?;
    if reply.ticket != ticket || reply.phase != BootstrapPhase::Prepared {
        return Err(Error::HandoffRequired);
    }
    Ok(reply)
}

/// 同worker的首次提交；不启动内核。业务变化/未确认选择保留Frozen并拒绝Commit。
pub fn commit_bootstrap<P: SidecarPort>(
    local: &mut ManualRuntime<P>,
    expected: SnapshotVersion,
    mut exchange: impl FnMut(BootstrapAction) -> Result<BootstrapReply, Error>,
) -> Result<BootstrapReply, Error> {
    let state = local
        .preflight_transfer(&expected)
        .map_err(|_| Error::HandoffRequired)?;
    let Some(OwnerTransfer::BootstrapFrozen {
        id,
        installation,
        version,
    }) = local.owner_transfer().map_err(|_| Error::HandoffRequired)?
    else {
        return Err(Error::HandoffRequired);
    };
    if version != expected {
        return Err(Error::VersionConflict);
    }
    local
        .freeze_first_bootstrap(id.clone(), installation.clone(), version.clone())
        .map_err(|_| Error::HandoffRequired)?;
    let ticket = BootstrapTicket {
        id,
        installation,
        version,
    };
    let config = Configuration {
        state: Box::new(state),
    };
    config.validate()?;
    let reply = exchange(BootstrapAction::Commit {
        ticket: ticket.clone(),
        config,
    })?;
    if reply.ticket != ticket || reply.phase != BootstrapPhase::Committed {
        return Err(Error::HandoffRequired);
    }
    let queried = exchange(BootstrapAction::Query { ticket })?;
    if queried != reply {
        return Err(Error::HandoffRequired);
    }
    Ok(reply)
}

/// 新Primary仅准备受控rebind；旧Frozen保持不变，Prepared不能发Start。
#[cfg(target_os = "macos")]
pub fn prepare_rebind<P: SidecarPort>(
    local: &mut ManualRuntime<P>,
    id: StateEpoch,
    expected: SnapshotVersion,
    mut exchange: impl FnMut(BootstrapAction) -> Result<BootstrapReply, Error>,
) -> Result<BootstrapReply, Error> {
    let state = local
        .preflight_transfer(&expected)
        .map_err(|_| Error::HandoffRequired)?;
    let Some(OwnerTransfer::BootstrapFrozen {
        id: old_id,
        installation,
        version,
    }) = local.owner_transfer().map_err(|_| Error::HandoffRequired)?
    else {
        return Err(Error::HandoffRequired);
    };
    if version != expected {
        return Err(Error::VersionConflict);
    }
    let previous = BootstrapTicket {
        id: old_id,
        installation,
        version,
    };
    let config = Configuration {
        state: Box::new(state),
    };
    config.validate()?;
    let reply = exchange(BootstrapAction::RebindPreflight {
        id: id.clone(),
        previous: previous.clone(),
        config: config.clone(),
    })?;
    if reply.phase != BootstrapPhase::Eligible
        || reply.ticket.id != id
        || reply.ticket.version != expected
        || reply.ticket.installation != previous.installation
    {
        return Err(Error::HandoffRequired);
    }
    local
        .prepare_rebind(&previous, &reply.ticket)
        .map_err(|_| Error::HandoffRequired)?;
    let prepared = exchange(BootstrapAction::RebindPrepare {
        previous,
        ticket: reply.ticket.clone(),
        config,
    })?;
    if prepared.ticket != reply.ticket || prepared.phase != BootstrapPhase::Prepared {
        return Err(Error::HandoffRequired);
    }
    let queried = exchange(BootstrapAction::RebindQuery {
        ticket: prepared.ticket.clone(),
    })?;
    if queried != prepared {
        return Err(Error::HandoffRequired);
    }
    Ok(prepared)
}

/// 同worker重验持久申请后提交；超时不解除Frozen，下次先Query同一票据。
#[cfg(target_os = "macos")]
pub fn commit_rebind<P: SidecarPort>(
    local: &mut ManualRuntime<P>,
    expected: SnapshotVersion,
    mut exchange: impl FnMut(BootstrapAction) -> Result<BootstrapReply, Error>,
) -> Result<BootstrapReply, Error> {
    let state = local
        .preflight_transfer(&expected)
        .map_err(|_| Error::HandoffRequired)?;
    let ticket = local
        .rebind_ticket()
        .map_err(|_| Error::HandoffRequired)?
        .ok_or(Error::HandoffRequired)?;
    if ticket.version != expected {
        return Err(Error::VersionConflict);
    }
    let Some(OwnerTransfer::BootstrapFrozen {
        id,
        installation,
        version,
    }) = local.owner_transfer().map_err(|_| Error::HandoffRequired)?
    else {
        return Err(Error::HandoffRequired);
    };
    local
        .prepare_rebind(
            &BootstrapTicket {
                id,
                installation,
                version,
            },
            &ticket,
        )
        .map_err(|_| Error::HandoffRequired)?;
    let config = Configuration {
        state: Box::new(state),
    };
    config.validate()?;
    let query = exchange(BootstrapAction::RebindQuery {
        ticket: ticket.clone(),
    })?;
    if query.ticket != ticket {
        return Err(Error::RequestConflict);
    }
    match query.phase {
        BootstrapPhase::Committed => return Ok(query),
        BootstrapPhase::Prepared => {}
        _ => return Err(Error::HandoffRequired),
    }
    let result = exchange(BootstrapAction::RebindCommit {
        ticket: ticket.clone(),
        config,
    })?;
    if result.ticket != ticket || result.phase != BootstrapPhase::Committed {
        return Err(Error::HandoffRequired);
    }
    let confirmed = exchange(BootstrapAction::RebindQuery { ticket })?;
    if confirmed != result {
        return Err(Error::HandoffRequired);
    }
    Ok(confirmed)
}
