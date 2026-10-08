//! 首次合作让权的root半边。Prepared只保留租约/证据，不是启动授权。
use super::{assets::Assets, transport::Peer, writer_lease::WriterLease, *};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

const RECORD: &str = "bootstrap-prepared.json";
#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Record {
    ticket: BootstrapTicket,
    peer: Peer,
    source: super::source::BootstrapSource,
    config_digest: String,
}
pub(super) struct Prepared {
    record: Record,
    lease: WriterLease,
    durable: bool,
    committed: bool,
    commit_started: bool,
    root: std::path::PathBuf,
    peer_exit: super::source::PeerExit,
    cycle: u64,
    rebind: Option<(RebindRecord, bool)>,
    rebind_commit_started: bool,
    rebind_committed: bool,
}
fn read_file<T: serde::de::DeserializeOwned>(root: &Path, name: &str) -> Result<T, Error> {
    let m = fs::symlink_metadata(root).map_err(|_| Error::HandoffRequired)?;
    if !m.is_dir()
        || m.uid() != unsafe { libc::geteuid() }
        || !matches!(m.mode() & 0o7777, 0o700 | 0o711)
    {
        return Err(Error::UnsafeEndpoint);
    }
    #[cfg(not(test))]
    super::transport::protected_directories(root)?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(root.join(name))
        .map_err(|_| Error::HandoffRequired)?;
    let m = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o7777 != 0o600
        || m.nlink() != 1
        || m.len() > 8192
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut bytes = vec![];
    file.take(8193)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    serde_json::from_slice(&bytes).map_err(|_| Error::HandoffRequired)
}
fn generation(assets: &Assets, peer: Peer) -> Result<veyra_core::domain::StateEpoch, Error> {
    if peer.uid != assets.uid || peer.gid != assets.gid || peer.uid == 0 {
        return Err(Error::Unauthorized);
    }
    assets.verify()?;
    let expected = assets.installation.as_ref().ok_or(Error::HandoffRequired)?;
    #[cfg(test)]
    let root = if assets.fixture {
        assets.root.parent().ok_or(Error::UnsafeEndpoint)?
    } else {
        Path::new(transport::ROOT)
    };
    #[cfg(not(test))]
    let root = Path::new(transport::ROOT);
    if super::install::installed_identity(root, peer.uid, peer.gid)? != *expected {
        return Err(Error::HandoffRequired);
    }
    Ok(expected.generation.clone())
}
pub(super) fn handle(
    assets: &Assets,
    slot: &mut Option<Prepared>,
    peer: Peer,
    action: BootstrapAction,
    mut source: impl FnMut(&BootstrapTicket) -> Result<super::source::BootstrapSource, Error>,
) -> Result<BootstrapReply, Error> {
    super::source::live_peer(peer)?;
    let generation = generation(assets, peer)?;
    match action {
        BootstrapAction::RebindPreflight { .. }
        | BootstrapAction::RebindPrepare { .. }
        | BootstrapAction::RebindQuery { .. }
        | BootstrapAction::RebindCommit { .. } => Err(Error::HandoffRequired),
        BootstrapAction::Preflight {
            id,
            expected,
            config,
        } => {
            config.validate()?;
            if config.version() != expected {
                return Err(Error::VersionConflict);
            }
            if slot.is_some() {
                return Err(Error::HandoffRequired);
            }
            assets.pristine_root()?; // 只读；没有创建lease/owner/session/cache
            Ok(BootstrapReply {
                ticket: BootstrapTicket {
                    id,
                    installation: generation,
                    version: expected,
                },
                phase: BootstrapPhase::Eligible,
            })
        }
        BootstrapAction::Query { ticket } => {
            if ticket.installation != generation {
                return Err(Error::VersionConflict);
            }
            let record = read_file::<Record>(&assets.root, RECORD)?;
            if record.peer != peer {
                return Err(Error::Unauthorized);
            }
            if record.ticket != ticket {
                return Err(Error::RequestConflict);
            }
            // 重启丢失内存slot/租约后只能读诊断，不能重获Prepared能力。
            let phase = if let Some(current) = slot {
                if !current.durable
                    || current.record != record
                    || (current.commit_started && !current.committed)
                    || (!current.committed
                        && fs::symlink_metadata(assets.root.join("bootstrap-committed.json"))
                            .is_ok())
                {
                    BootstrapPhase::RecoveryRequired
                } else {
                    current.lease.verify()?;
                    if source(&ticket)? != record.source {
                        return Err(Error::Unauthorized);
                    }
                    if current.committed {
                        if read_file::<Record>(&assets.root, "bootstrap-committed.json")? != record
                        {
                            return Err(Error::HandoffRequired);
                        }
                        BootstrapPhase::Committed
                    } else {
                        BootstrapPhase::Prepared
                    }
                }
            } else {
                BootstrapPhase::RecoveryRequired
            };
            Ok(BootstrapReply { ticket, phase })
        }
        BootstrapAction::Commit { ticket, config } => {
            config.validate()?;
            if ticket.installation != generation || config.version() != ticket.version {
                return Err(Error::VersionConflict);
            }
            let current = slot.as_mut().ok_or(Error::HandoffRequired)?;
            if current.record.peer != peer {
                return Err(Error::Unauthorized);
            }
            if current.record.ticket != ticket
                || current.record.config_digest != config_digest(&config)?
            {
                return Err(Error::RequestConflict);
            }
            current.lease.verify()?;
            if source(&ticket)? != current.record.source
                || read_file::<Record>(&assets.root, RECORD)? != current.record
                || !current.durable
            {
                return Err(Error::HandoffRequired);
            }
            if !current.committed {
                for entry in fs::read_dir(&assets.root).map_err(|_| Error::UnsafeEndpoint)? {
                    let name = entry.map_err(|_| Error::UnsafeEndpoint)?.file_name();
                    #[cfg(test)]
                    if assets.fixture && name == super::writer_lease::NAME {
                        continue;
                    }
                    if name != RECORD {
                        return Err(Error::HandoffRequired);
                    }
                }
            }
            if current.committed {
                current.verify_committed(peer)?;
            } else {
                if current.commit_started {
                    return Err(Error::HandoffRequired);
                }
                current.commit_started = true;
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(assets.root.join("bootstrap-committed.json"))
                    .map_err(|_| Error::HandoffRequired)?;
                file.write_all(
                    &serde_json::to_vec(&current.record).map_err(|_| Error::Unavailable)?,
                )
                .and_then(|_| file.sync_all())
                .map_err(|_| Error::Unavailable)?;
                File::open(&assets.root)
                    .and_then(|f| f.sync_all())
                    .map_err(|_| Error::Unavailable)?;
                current.committed = true;
            }
            Ok(BootstrapReply {
                ticket,
                phase: BootstrapPhase::Committed,
            })
        }
        BootstrapAction::Prepare { ticket, config } => {
            config.validate()?;
            if ticket.installation != generation || config.version() != ticket.version {
                return Err(Error::VersionConflict);
            }
            let source_proof = source(&ticket)?; // incarnation-only半写入在此拒绝
            let record = Record {
                ticket: ticket.clone(),
                peer,
                source: source_proof,
                config_digest: veyra_core::application::runtime_recovery::digest(
                    &serde_json::to_vec(&config).map_err(|_| Error::InvalidRequest)?,
                ),
            };
            if let Some(current) = slot {
                if current.record.peer != peer {
                    return Err(Error::Unauthorized);
                }
                if current.record != record {
                    return Err(Error::RequestConflict);
                }
                if !current.durable
                    || (current.commit_started && !current.committed)
                    || read_file::<Record>(&assets.root, RECORD)? != record
                {
                    return Err(Error::HandoffRequired);
                }
                current.lease.verify()?;
                return Ok(BootstrapReply {
                    ticket,
                    phase: if current.committed {
                        BootstrapPhase::Committed
                    } else {
                        BootstrapPhase::Prepared
                    },
                });
            }
            let peer_exit = super::source::PeerExit::capture(peer)?;
            assets.pristine_root()?;
            assets.prepare_root()?;
            let lease = super::process::ProcessPort::acquire_writer_lease(assets)?;
            lease.verify()?;
            // 取得lease后即保留slot；任何持久失败都不自动释放或擦除证据。
            *slot = Some(Prepared {
                record,
                lease,
                durable: false,
                committed: false,
                commit_started: false,
                root: assets.root.clone(),
                peer_exit,
                cycle: 0,
                rebind: None,
                rebind_commit_started: false,
                rebind_committed: false,
            });
            let current = slot.as_mut().expect("lease held");
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(assets.root.join(RECORD))
                .map_err(|_| Error::HandoffRequired)?;
            let bytes = serde_json::to_vec(&current.record).map_err(|_| Error::Unavailable)?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| Error::Unavailable)?;
            File::open(&assets.root)
                .and_then(|f| f.sync_all())
                .map_err(|_| Error::Unavailable)?;
            current.durable = true;
            Ok(BootstrapReply {
                ticket,
                phase: BootstrapPhase::Prepared,
            })
        }
    }
}

fn config_digest(config: &Configuration) -> Result<String, Error> {
    Ok(veyra_core::application::runtime_recovery::digest(
        &serde_json::to_vec(config).map_err(|_| Error::InvalidRequest)?,
    ))
}
impl Prepared {
    pub(super) fn is_committed(&self) -> bool {
        self.committed && (self.rebind.is_none() || self.rebind_committed)
    }
    pub(super) fn ticket(&self) -> &BootstrapTicket {
        &self.active_record().ticket
    }
    fn active_record(&self) -> &Record {
        if self.rebind_committed {
            &self.rebind.as_ref().expect("committed rebind").0.next
        } else {
            &self.record
        }
    }
    pub(super) fn is_rebound(&self) -> bool {
        self.rebind_committed
    }
    pub(super) fn verify_committed(&self, peer: Peer) -> Result<(), Error> {
        if self.active_record().peer != peer {
            return Err(Error::Unauthorized);
        }
        if !self.committed
            || !self.durable
            || read_file::<Record>(&self.root, RECORD)? != self.record
            || read_file::<Record>(&self.root, "bootstrap-committed.json")? != self.record
        {
            return Err(Error::HandoffRequired);
        }
        if self.rebind_committed {
            let expected = &self.rebind.as_ref().ok_or(Error::HandoffRequired)?.0;
            if read_file::<RebindRecord>(&self.root, "bootstrap-rebind-committed.json")?
                != *expected
                || read_file::<RebindRecord>(&self.root, "bootstrap-rebind-prepared.json")?
                    != *expected
                || read_file::<Peer>(&self.root, "bootstrap-current-owner.json")? != peer
            {
                return Err(Error::HandoffRequired);
            }
        }
        #[cfg(test)]
        let install_root = self.root.parent().ok_or(Error::UnsafeEndpoint)?;
        #[cfg(not(test))]
        let install_root = Path::new(transport::ROOT);
        if super::install::installed_identity(install_root, peer.uid, peer.gid)?.generation
            != self.record.ticket.installation
        {
            return Err(Error::HandoffRequired);
        }
        self.lease.verify()
    }
    pub(super) fn verify_source(
        &self,
        proof: &super::source::BootstrapSource,
    ) -> Result<(), Error> {
        if &self.active_record().source == proof {
            Ok(())
        } else {
            Err(Error::HandoffRequired)
        }
    }
    pub(super) fn lease_for_start(
        &self,
        peer: Peer,
        config: &Configuration,
    ) -> Result<WriterLease, Error> {
        self.verify_committed(peer)?;
        if config.version() != self.active_record().ticket.version
            || config_digest(config)? != self.active_record().config_digest
        {
            return Err(Error::VersionConflict);
        }
        self.lease.duplicate()
    }
}

/// 单份当前生命周期记录，不是历史请求journal；CleanStop只作后续rebind的输入，当前不授予新peer权限。
#[derive(serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Lifecycle {
    owner: Record,
    cycle: u64,
    closed: Option<String>,
}
const LIFECYCLE: &str = "bootstrap-lifecycle.json";
impl Prepared {
    pub(super) fn peer_exited(&mut self) -> Result<bool, Error> {
        self.peer_exit.exited()
    }
    fn write_lifecycle(&self, closed: Option<String>) -> Result<(), Error> {
        self.verify_committed(self.active_record().peer)?;
        self.persist_lifecycle(self.active_record().clone(), closed)
    }
    fn persist_lifecycle(&self, owner: Record, closed: Option<String>) -> Result<(), Error> {
        let record = Lifecycle {
            owner,
            cycle: self.cycle,
            closed,
        };
        // 固定临时文件残留表示未知半写入；不删除、不覆盖后重试。
        let temporary = self.root.join("bootstrap-lifecycle.pending");
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&temporary)
            .map_err(|_| Error::HandoffRequired)?;
        f.write_all(&serde_json::to_vec(&record).map_err(|_| Error::Unavailable)?)
            .and_then(|_| f.sync_all())
            .map_err(|_| Error::Unavailable)?;
        fs::rename(temporary, self.root.join(LIFECYCLE)).map_err(|_| Error::Unavailable)?;
        File::open(&self.root)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Unavailable)
    }
    pub(super) fn begin_run(&mut self) -> Result<(), Error> {
        if self.cycle != 0 {
            if self.clean_digest()?.is_none() {
                return Err(Error::HandoffRequired);
            }
        } else if fs::symlink_metadata(self.root.join(LIFECYCLE)).is_ok() {
            return Err(Error::HandoffRequired);
        }
        self.cycle = self.cycle.checked_add(1).ok_or(Error::HandoffRequired)?;
        self.write_lifecycle(None) // 必须先持久失效旧CleanStop，才能启动下一个child。
    }
    pub(super) fn clean_stop(
        &self,
        bundle: &veyra_core::application::runtime_recovery::ClosedBundle,
    ) -> Result<(), Error> {
        let owner_file = if self.rebind_committed {
            "bootstrap-current-owner.json"
        } else {
            "owner-session.json"
        };
        if read_file::<Peer>(&self.root, owner_file)? != self.active_record().peer {
            return Err(Error::HandoffRequired);
        }
        let old: Lifecycle = read_file(&self.root, LIFECYCLE)?;
        if old.owner != *self.active_record()
            || old.cycle != self.cycle
            || old.closed.is_some()
            || bundle.version() != &self.active_record().ticket.version
        {
            return Err(Error::HandoffRequired);
        }
        let digest = bundle
            .ticket(self.active_record().ticket.id.clone())
            .map_err(|_| Error::HandoffRequired)?
            .digest;
        self.write_lifecycle(Some(digest))
    }
    pub(super) fn clean_digest(&self) -> Result<Option<String>, Error> {
        self.verify_committed(self.active_record().peer)?;
        let record: Lifecycle = read_file(&self.root, LIFECYCLE)?;
        if record.owner != *self.active_record()
            || record.cycle != self.cycle
            || fs::symlink_metadata(self.root.join("bootstrap-lifecycle.pending")).is_ok()
        {
            return Err(Error::HandoffRequired);
        }
        Ok(record.closed)
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct RebindRecord {
    previous: Record,
    next: Record,
    cycle: u64,
    closed: String,
}
impl Prepared {
    pub(super) fn rebind(
        &mut self,
        peer: Peer,
        action: BootstrapAction,
        bundle: Option<&veyra_core::application::runtime_recovery::ClosedBundle>,
        #[cfg(test)] source_root: Option<&Path>,
    ) -> Result<BootstrapReply, Error> {
        // 已提交重放不要求再次Stopped；否则丢回复后Start会让Query永远不可用。
        if self.rebind_committed {
            self.verify_committed(peer)?;
            let ticket = match &action {
                BootstrapAction::RebindQuery { ticket } => ticket,
                BootstrapAction::RebindCommit { ticket, config } => {
                    config.validate()?;
                    self.lease_for_start(peer, config)?;
                    self.rebind_source(
                        peer,
                        #[cfg(test)]
                        source_root,
                    )?;
                    ticket
                }
                _ => return Err(Error::RequestConflict),
            };
            if ticket != self.ticket() {
                return Err(Error::RequestConflict);
            }
            return Ok(BootstrapReply {
                ticket: ticket.clone(),
                phase: BootstrapPhase::Committed,
            });
        }
        self.verify_committed(self.record.peer)?;
        if peer.uid != self.record.peer.uid
            || peer.gid != self.record.peer.gid
            || peer == self.record.peer
            || !self.peer_exit.exited()?
        {
            return Err(Error::Unauthorized);
        }
        if read_file::<Peer>(&self.root, "owner-session.json")? != self.record.peer {
            return Err(Error::HandoffRequired);
        }
        let closed = self.clean_digest()?.ok_or(Error::HandoffRequired)?;
        if bundle
            .ok_or(Error::HandoffRequired)?
            .ticket(self.active_record().ticket.id.clone())
            .map_err(|_| Error::HandoffRequired)?
            .digest
            != closed
        {
            return Err(Error::HandoffRequired);
        }
        let preparing = matches!(action, BootstrapAction::RebindPrepare { .. });
        let committing = matches!(action, BootstrapAction::RebindCommit { .. });
        let (previous, ticket, config, query) = match action {
            BootstrapAction::RebindPreflight {
                id,
                previous,
                config,
            } => {
                let ticket = BootstrapTicket {
                    id,
                    installation: previous.installation.clone(),
                    version: previous.version.clone(),
                };
                (previous, ticket, Some(config), false)
            }
            BootstrapAction::RebindPrepare {
                previous,
                ticket,
                config,
            } => (previous, ticket, Some(config), false),
            BootstrapAction::RebindQuery { ticket } => {
                (self.record.ticket.clone(), ticket, None, true)
            }
            BootstrapAction::RebindCommit { ticket, config } => {
                (self.record.ticket.clone(), ticket, Some(config), false)
            }
            _ => return Err(Error::InvalidRequest),
        };
        if previous != self.record.ticket
            || ticket.installation != previous.installation
            || ticket.version != previous.version
            || ticket.id == previous.id
        {
            return Err(Error::VersionConflict);
        }
        if let Some(config) = &config {
            config.validate()?;
            if config.version() != previous.version
                || config_digest(config)? != self.record.config_digest
            {
                return Err(Error::VersionConflict);
            }
        }
        // Preflight与Prepare在调用处以是否已有本地申请区分，不能由token跳过OS身份。
        let proof = super::source::rebind(
            peer,
            self.record.peer,
            &self.record.source.incarnation,
            &previous,
            (preparing || query || committing).then_some(&ticket),
            #[cfg(test)]
            source_root,
        )?;
        if !preparing && !query && !committing {
            if self
                .rebind
                .as_ref()
                .is_some_and(|(r, _)| r.next.peer != peer || r.next.ticket != ticket)
            {
                return Err(Error::RequestConflict);
            }
            return Ok(BootstrapReply {
                ticket,
                phase: BootstrapPhase::Eligible,
            });
        }
        let next = Record {
            ticket: ticket.clone(),
            peer,
            source: proof.ok_or(Error::HandoffRequired)?,
            config_digest: self.record.config_digest.clone(),
        };
        let record = RebindRecord {
            previous: self.record.clone(),
            next,
            cycle: self.cycle,
            closed,
        };
        if let Some((old, durable)) = &self.rebind {
            if old != &record {
                return Err(Error::RequestConflict);
            }
            let phase = if *durable
                && !self.rebind_commit_started
                && read_file::<RebindRecord>(&self.root, "bootstrap-rebind-prepared.json")?
                    == record
            {
                BootstrapPhase::Prepared
            } else {
                BootstrapPhase::RecoveryRequired
            };
            if committing && phase == BootstrapPhase::Prepared {
                // 在第一笔写入前绑定新NOTE_EXIT；全程沿用同一lease/ProcessPort。
                let exit = super::source::PeerExit::capture(peer)?;
                self.rebind_commit_started = true;
                for (name, bytes) in [
                    (
                        "bootstrap-rebind-committed.json",
                        serde_json::to_vec(&record),
                    ),
                    ("bootstrap-current-owner.json", serde_json::to_vec(&peer)),
                ] {
                    let mut f = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .mode(0o600)
                        .custom_flags(libc::O_NOFOLLOW)
                        .open(self.root.join(name))
                        .map_err(|_| Error::HandoffRequired)?;
                    f.write_all(&bytes.map_err(|_| Error::Unavailable)?)
                        .and_then(|_| f.sync_all())
                        .map_err(|_| Error::Unavailable)?;
                    File::open(&self.root)
                        .and_then(|f| f.sync_all())
                        .map_err(|_| Error::Unavailable)?;
                }
                // 旧CleanStop的owner/cycle/closed已在不可覆盖的rebind记录内保留。
                // 新生命周期沿用cycle；下一次begin_run先失效clean并单调+1。
                self.persist_lifecycle(record.next.clone(), Some(record.closed.clone()))?;
                self.peer_exit = exit;
                self.rebind_committed = true;
                return Ok(BootstrapReply {
                    ticket,
                    phase: BootstrapPhase::Committed,
                });
            }
            return Ok(BootstrapReply { ticket, phase });
        }
        if query || committing {
            return Err(Error::HandoffRequired);
        }
        self.rebind = Some((record.clone(), false)); // 写失败也保持冻结，不能退回旧owner启动。
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.root.join("bootstrap-rebind-prepared.json"))
            .map_err(|_| Error::HandoffRequired)?;
        file.write_all(&serde_json::to_vec(&record).map_err(|_| Error::Unavailable)?)
            .and_then(|_| file.sync_all())
            .map_err(|_| Error::Unavailable)?;
        File::open(&self.root)
            .and_then(|f| f.sync_all())
            .map_err(|_| Error::Unavailable)?;
        self.rebind.as_mut().unwrap().1 = true;
        Ok(BootstrapReply {
            ticket,
            phase: BootstrapPhase::Prepared,
        })
    }
}

impl Prepared {
    pub(super) fn rebind_source(
        &self,
        peer: Peer,
        #[cfg(test)] root: Option<&Path>,
    ) -> Result<super::source::BootstrapSource, Error> {
        if !self.rebind_committed {
            return Err(Error::HandoffRequired);
        }
        let proof = super::source::rebind(
            peer,
            self.record.peer,
            &self.record.source.incarnation,
            &self.record.ticket,
            Some(self.ticket()),
            #[cfg(test)]
            root,
        )?
        .ok_or(Error::HandoffRequired)?;
        self.verify_source(&proof)?;
        Ok(proof)
    }
}
