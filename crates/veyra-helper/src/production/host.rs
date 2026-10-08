use super::{
    transport::{self, Peer},
    *,
};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::{
        fs::{MetadataExt, OpenOptionsExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread,
    time::{Duration, Instant},
};

const RETAIN: Duration = Duration::from_secs(120);
const RESULTS: usize = 128;
const OPERATION_BUDGET: Duration = Duration::from_secs(30);
const CONNECTIONS: usize = 4;

/// Port 仅在本 crate 内可实现；生产实现始终以已授权普通用户运行child。
pub(super) trait Backend: Send + 'static {
    fn capabilities(&self) -> Capabilities;
    fn bootstrap(
        &mut self,
        _peer: Peer,
        _action: BootstrapAction,
    ) -> (Status, Result<BootstrapReply, Error>) {
        (Status::default(), Err(Error::Unsupported))
    }
    fn handoff(
        &mut self,
        _peer: Peer,
        _action: HandoffAction,
        _deadline: Instant,
    ) -> (Status, Result<HandoffReply, Error>) {
        (Status::default(), Err(Error::Unsupported))
    }
    fn selection(
        &mut self,
        _peer: Peer,
        _action: RemoteSelectionAction,
        _deadline: Instant,
    ) -> (Status, Result<RemoteSelectionReply, Error>) {
        (Status::default(), Err(Error::Unsupported))
    }
    fn poll(&mut self) -> Option<Status> {
        None
    }
    /// deadline 为 Host 固定预算；返回实际状态，即使失败也不能虚构清理/Ready。
    fn execute(
        &mut self,
        peer: Peer,
        command: Command,
        deadline: Instant,
    ) -> (Status, Result<Status, Error>);
}
struct Record {
    peer: Peer,
    id: u64,
    fingerprint: [u8; 32],
    result: Operation,
    completed: Option<Instant>,
    selection: Option<Result<RemoteSelectionReply, Error>>,
}
struct State {
    capabilities: Capabilities,
    status: Status,
    owner: Option<Peer>,
    config_digest: Option<[u8; 32]>,
    inflight: bool,
    records: VecDeque<Record>,
    events: VecDeque<Event>,
    sequence: u64,
}
struct Job {
    peer: Peer,
    id: u64,
    command: Command,
    verify_instance: Option<u64>,
    retained_completion: Option<Instant>,
    handoff_reply: Option<mpsc::SyncSender<Response>>,
}
pub(super) struct Host {
    uid: u32,
    state: Arc<Mutex<State>>,
    jobs: mpsc::SyncSender<Option<Job>>,
    worker: Option<thread::JoinHandle<()>>,
    lifecycle: Option<Arc<std::fs::File>>,
    pub(super) retain: Duration,
    pub(super) capacity: usize,
    #[cfg(test)]
    pub(super) idle_poll: Arc<std::sync::atomic::AtomicBool>,
}
impl Host {
    pub(super) fn new(uid: u32, mut backend: impl Backend) -> Self {
        let capabilities = backend.capabilities();
        let initial_status = backend.poll().unwrap_or_default();
        let state = Arc::new(Mutex::new(State {
            capabilities,
            status: initial_status,
            owner: None,
            config_digest: None,
            inflight: false,
            records: VecDeque::new(),
            events: VecDeque::new(),
            sequence: 0,
        }));
        let (jobs, receiver) = mpsc::sync_channel::<Option<Job>>(1);
        let shared = state.clone();
        #[cfg(test)]
        let idle_poll = Arc::new(std::sync::atomic::AtomicBool::new(true));
        #[cfg(test)]
        let polling = idle_poll.clone();
        let worker = thread::spawn(move || {
            loop {
                let job = match receiver.recv_timeout(Duration::from_millis(250)) {
                    Ok(Some(job)) => job,
                    Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        #[cfg(test)]
                        if !polling.load(std::sync::atomic::Ordering::Acquire) {
                            continue;
                        }
                        if let Some(status) = backend.poll() {
                            let mut state = shared.lock().expect("helper state poisoned");
                            if state.status != status {
                                state.status = status.clone();
                                if status.instance.is_none() {
                                    state.config_digest = None;
                                }
                                state.sequence += 1;
                                let sequence = state.sequence;
                                if state.events.len() == EVENT_CAPACITY {
                                    state.events.pop_front();
                                }
                                state.events.push_back(Event {
                                    sequence,
                                    status,
                                    result: Err(Error::Failed),
                                });
                            }
                        }
                        continue;
                    }
                };
                if let Command::Selection { action } = job.command {
                    let selection_id = action.request().request_id;
                    let (status, result) =
                        backend.selection(job.peer, *action, Instant::now() + OPERATION_BUDGET);
                    let mut state = shared.lock().expect("helper state poisoned");
                    state.status = status.clone();
                    state.inflight = false;
                    if result.is_ok() {
                        state.owner = status.instance.map(|_| job.peer);
                    }
                    let outcome = match &result {
                        Ok(reply)
                            if reply.confirmed.is_some()
                                && reply.actual.as_ref() == Some(&reply.request.node) =>
                        {
                            Ok(())
                        }
                        Ok(reply) if reply.confirmed.is_some() => Err(Error::Failed),
                        Ok(_) => Err(Error::HandoffRequired),
                        Err(e) => Err(*e),
                    };
                    if let Some(record) = state
                        .records
                        .iter_mut()
                        .find(|r| r.peer == job.peer && r.id == selection_id)
                    {
                        record.result = Operation::Completed(outcome.map(|_| status.clone()));
                        record.selection = Some(result.clone());
                        record.completed.get_or_insert(Instant::now());
                    }

                    if state.owner == Some(job.peer) {
                        state.sequence += 1;
                        let sequence = state.sequence;
                        if state.events.len() == EVENT_CAPACITY {
                            state.events.pop_front();
                        }
                        state.events.push_back(Event {
                            sequence,
                            status,
                            result: outcome,
                        });
                    }
                    if let Some(reply) = job.handoff_reply {
                        let _ = reply.send(Response::Selection(result));
                    }
                    continue;
                }
                if let Command::Bootstrap { action } = job.command {
                    let (status, result) = backend.bootstrap(job.peer, *action);
                    let mut state = shared.lock().expect("helper state poisoned");
                    state.capabilities = backend.capabilities();
                    state.status = status;
                    state.inflight = false;
                    if let Some(reply) = job.handoff_reply {
                        let _ = reply.send(Response::Bootstrap(result));
                    }
                    continue;
                }
                if let Command::Handoff { action } = job.command {
                    let (status, result) =
                        backend.handoff(job.peer, *action, Instant::now() + OPERATION_BUDGET);
                    let mut state = shared.lock().expect("helper state poisoned");
                    state.capabilities = backend.capabilities();
                    state.status = status;
                    state.inflight = false;
                    if state.status.instance.is_none() {
                        state.config_digest = None;
                    }
                    if let Some(reply) = job.handoff_reply {
                        let _ = reply.send(Response::Handoff(result));
                    }
                    continue;
                }
                let config_digest = job.command.configuration().map(|config| {
                    <[u8; 32]>::from(Sha256::digest(
                        serde_json::to_vec(config).expect("typed configuration"),
                    ))
                });
                let (status, result) = if let Some(instance) = job.verify_instance {
                    // 新ID的幂等Start也必须在owner线程同步核验child，不能用250ms缓存冒充Ready。
                    let status = backend.poll().unwrap_or_else(|| Status {
                        recovery_required: true,
                        ..Status::default()
                    });
                    let result = if status.instance == Some(instance)
                        && status.applied == job.command.configuration().map(Configuration::version)
                        && status.applied.is_some()
                        && !status.recovery_required
                    {
                        Ok(status.clone())
                    } else {
                        Err(Error::StaleInstance)
                    };
                    (status, result)
                } else {
                    backend.execute(job.peer, job.command, Instant::now() + OPERATION_BUDGET)
                };
                let mut state = shared.lock().expect("helper state poisoned");
                if state.owner != Some(job.peer) {
                    state.events.clear();
                }
                if status.instance.is_none() {
                    state.config_digest = None;
                } else if result.is_ok() && config_digest.is_some() {
                    state.config_digest = config_digest;
                }
                state.status = status.clone();
                state.owner = status.instance.map(|_| job.peer);
                state.inflight = false;
                let record = state
                    .records
                    .iter_mut()
                    .find(|r| r.peer == job.peer && r.id == job.id)
                    .expect("inflight records are never evicted");
                record.result = Operation::Completed(result.clone());
                record.completed = Some(job.retained_completion.unwrap_or_else(Instant::now));
                state.sequence += 1;
                let event = Event {
                    sequence: state.sequence,
                    status,
                    result: result.map(|_| ()),
                };
                if state.events.len() == EVENT_CAPACITY {
                    state.events.pop_front();
                }
                state.events.push_back(event);
            }
        });
        Self {
            uid,
            state,
            jobs,
            worker: Some(worker),
            lifecycle: None,
            retain: RETAIN,
            capacity: RESULTS,
            #[cfg(test)]
            idle_poll,
        }
    }
    pub(super) fn dispatch(&self, peer: Peer, request: Request) -> Response {
        if peer.uid == 0 || peer.uid != self.uid {
            return Response::Rejected(Error::Unauthorized);
        }
        if request.protocol != PROTOCOL {
            return Response::Rejected(Error::IncompatibleVersion);
        }
        let command = request.command;
        if let Some(config) = command.configuration()
            && let Err(error) = config.validate()
        {
            return Response::Rejected(error);
        }
        let mut state = self.state.lock().expect("helper state poisoned");
        state
            .records
            .retain(|r| r.completed.is_none_or(|at| at.elapsed() < self.retain));
        match &command {
            Command::Hello => return Response::Hello(state.capabilities.clone()),
            Command::Status => return Response::Status(state.status.clone()),
            Command::Operation { request_id } => {
                let result = if let Some(index) = state
                    .records
                    .iter()
                    .position(|r| r.peer == peer && r.id == *request_id)
                {
                    let record = state.records.remove(index).expect("record index");
                    let result = record.result.clone();
                    state.records.push_back(record); // LRU 访问不延长绝对 TTL。
                    result
                } else {
                    Operation::Unknown
                };
                return Response::Operation(result);
            }
            _ => {}
        }
        if let Command::Selection { action } = &command {
            let request = action.request();
            if request.request_id == 0 {
                return Response::Rejected(Error::InvalidRequest);
            }
            let fingerprint: [u8; 32] =
                Sha256::digest(serde_json::to_vec(request).expect("typed selection")).into();
            if let Some(record) = state
                .records
                .iter()
                .find(|r| r.peer == peer && r.id == request.request_id)
            {
                if record.fingerprint != fingerprint {
                    return Response::Rejected(Error::RequestConflict);
                }
                if matches!(action.as_ref(), RemoteSelectionAction::Execute { .. }) {
                    return record
                        .selection
                        .clone()
                        .map(Response::Selection)
                        .unwrap_or(Response::Operation(Operation::Inflight));
                }
            } else if matches!(action.as_ref(), RemoteSelectionAction::Execute { .. }) {
                if state.inflight {
                    return Response::Rejected(Error::Busy);
                }
                while state.records.len() >= self.capacity {
                    state.records.pop_front();
                }
                state.records.push_back(Record {
                    peer,
                    id: request.request_id,
                    fingerprint,
                    result: Operation::Inflight,
                    completed: None,
                    selection: None,
                });
            }
        }
        if matches!(
            command,
            Command::Handoff { .. } | Command::Selection { .. } | Command::Bootstrap { .. }
        ) {
            if state.inflight {
                return Response::Rejected(Error::Busy);
            }
            state.inflight = true;
            drop(state);
            let (tx, rx) = mpsc::sync_channel(1);
            self.jobs
                .send(Some(Job {
                    peer,
                    id: 0,
                    command,
                    verify_instance: None,
                    retained_completion: None,
                    handoff_reply: Some(tx),
                }))
                .expect("helper worker alive");
            // 断线/超时不取消握手；持久owner记录可以Query，不能重新释放本地writer。
            return rx
                .recv_timeout(OPERATION_BUDGET)
                .unwrap_or(Response::Rejected(Error::Timeout));
        }
        let fingerprint: [u8; 32] =
            Sha256::digest(serde_json::to_vec(&command).expect("serializable command")).into();
        if let Some(id) = command.request_id() {
            if id == 0 {
                return Response::Rejected(Error::InvalidRequest);
            }
            if let Some(index) = state
                .records
                .iter()
                .position(|r| r.peer == peer && r.id == id)
            {
                let mut record = state.records.remove(index).expect("record index");
                // 同ID成功Start也是当前实例的使用请求，不能直接重放已退出child的Ready。
                // 只重新poll，不重新execute；Operation仍用于查询历史结果，TTL不被重试延长。
                if record.fingerprint == fingerprint
                    && matches!(command, Command::Start { .. })
                    && let Operation::Completed(Ok(ref previous)) = record.result
                    && let Some(instance) = previous.instance
                {
                    if state.inflight {
                        state.records.push_back(record);
                        return Response::Rejected(Error::Busy);
                    }
                    if state.owner != Some(peer) || state.status.instance != Some(instance) {
                        record.result = Operation::Completed(Err(Error::StaleInstance));
                        let response = Response::Operation(record.result.clone());
                        state.records.push_back(record);
                        return response;
                    }
                    let retained_completion = record.completed.take();
                    record.result = Operation::Inflight;
                    state.records.push_back(record);
                    state.inflight = true;
                    drop(state);
                    self.jobs
                        .send(Some(Job {
                            peer,
                            id,
                            command,
                            verify_instance: Some(instance),
                            retained_completion,
                            handoff_reply: None,
                        }))
                        .expect("helper worker alive");
                    return Response::Operation(Operation::Inflight);
                }
                let result = if record.fingerprint == fingerprint {
                    Response::Operation(record.result.clone())
                } else {
                    Response::Rejected(Error::RequestConflict)
                };
                state.records.push_back(record);
                return result;
            }
        }
        if let Some(instance) = command.instance()
            && (state.owner != Some(peer) || state.status.instance != Some(instance))
        {
            return Response::Rejected(Error::StaleInstance);
        }
        match &command {
            Command::Start {
                expected, config, ..
            } => {
                if expected != &config.version() {
                    return Response::Rejected(Error::VersionConflict);
                }
                if state.status.instance.is_some() {
                    if state.owner != Some(peer) {
                        return Response::Rejected(Error::Busy);
                    }
                    // 新 request ID 的 Start 也不能产生第二个 child；改变配置应显式 Apply。
                    let digest: [u8; 32] =
                        Sha256::digest(serde_json::to_vec(config).expect("typed config")).into();
                    if state.status.applied.as_ref() != Some(expected)
                        || state.config_digest != Some(digest)
                    {
                        return Response::Rejected(Error::VersionConflict);
                    }
                    if state.inflight {
                        return Response::Rejected(Error::Busy);
                    }
                }
            }
            Command::Apply {
                expected, config, ..
            } => {
                let version = config.version();
                if state.status.applied.as_ref() != Some(expected)
                    || expected.config.0.epoch != version.config.0.epoch
                    || version.config.0.revision <= expected.config.0.revision
                    || version.selection.0.revision < expected.selection.0.revision
                {
                    return Response::Rejected(Error::VersionConflict);
                }
            }
            Command::RuntimeCommand { expected, .. }
                if state.status.applied.as_ref() != Some(expected) =>
            {
                return Response::Rejected(Error::VersionConflict);
            }
            Command::RuntimeEvent { after, limit, .. } => {
                if *limit == 0 || *limit > EVENT_BATCH {
                    return Response::Rejected(Error::TooLarge);
                }
                let gap = state
                    .events
                    .front()
                    .is_some_and(|e| after.saturating_add(1) < e.sequence);
                let events = state
                    .events
                    .iter()
                    .filter(|e| e.sequence > *after)
                    .take(*limit)
                    .cloned()
                    .collect();
                return Response::Events { events, gap };
            }
            Command::ApplySystemProxy { .. } | Command::RestoreSystemProxy { .. } => {
                return Response::Rejected(Error::Unsupported);
            }
            _ => {}
        }
        if state.inflight {
            return Response::Rejected(Error::Busy);
        }
        let id = command.request_id().expect("read commands returned above");
        while state.records.len() >= self.capacity {
            state.records.pop_front();
        }
        state.records.push_back(Record {
            peer,
            id,
            fingerprint,
            result: Operation::Inflight,
            completed: None,
            selection: None,
        });
        let verify_instance = if matches!(command, Command::Start { .. }) {
            state.status.instance
        } else {
            None
        };
        state.inflight = true;
        // 有界队列至多一个业务操作。连接断开不会删除 job 或释放业务 owner。
        self.jobs
            .send(Some(Job {
                peer,
                id,
                command,
                verify_instance,
                retained_completion: None,
                handoff_reply: None,
            }))
            .expect("helper worker alive");
        Response::Operation(Operation::Inflight)
    }
    pub(super) fn connection(&self, mut stream: UnixStream) -> Result<(), Error> {
        let peer = transport::peer(&stream)?;
        if peer.uid == 0 || peer.uid != self.uid {
            return Err(Error::Unauthorized);
        }
        let first = transport::read_request(&mut stream)?;
        if !matches!(first.command, Command::Hello) {
            return Err(Error::InvalidRequest);
        }
        let response = self.dispatch(peer, first);
        transport::write_response(&mut stream, &response)?;
        if !matches!(response, Response::Hello(_)) {
            return Ok(());
        }
        let request = transport::read_request(&mut stream)?;
        let response = self.dispatch(peer, request);
        // 接收完整合法请求后先交 worker，再尝试写响应；写失败不取消操作。
        transport::write_response(&mut stream, &response)
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = self.jobs.send(None);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

/// 安装登记文件只由管理员写入，不能由 IPC 注册用户。
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Authorization {
    uid: u32,
    gid: u32,
}
fn authorization() -> Result<Authorization, Error> {
    transport::protected_directories(Path::new(transport::ROOT))?;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(Path::new(transport::ROOT).join("authorization.json"))
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                Error::NotInstalled
            } else {
                Error::UnsafeEndpoint
            }
        })?;
    let meta = file.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !meta.is_file() || meta.uid() != 0 || meta.mode() & 0o7777 != 0o600 || meta.len() > 1024 {
        return Err(Error::UnsafeEndpoint);
    }
    let mut bytes = Vec::new();
    file.take(1025)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Unavailable)?;
    let auth: Authorization = serde_json::from_slice(&bytes).map_err(|_| Error::Unauthorized)?;
    if auth.uid == 0 || auth.gid == 0 {
        return Err(Error::Unauthorized);
    }
    Ok(auth)
}
use super::backend::InstalledBackend;
/// 唯一生产 server 入口。无 path/UID/命令参数，必须读取管理员固定登记。
pub fn serve_installed() -> Result<(), Error> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(Error::Unauthorized);
    }
    let auth = authorization()?;
    let lifecycle = Arc::new(super::admin::lock(
        Path::new(transport::ROOT),
        "lifecycle-lock",
    )?);
    if super::admin::requested(Path::new(transport::ROOT))? {
        return Err(Error::HandoffRequired);
    }
    // 不删除旧 socket：不能把另一宿主或需要恢复的现场当作 stale 自动接管。
    if fs::symlink_metadata(transport::SOCKET).is_ok() {
        return Err(Error::Busy);
    }
    let listener = UnixListener::bind(transport::SOCKET).map_err(|_| Error::Unavailable)?;
    let metadata = fs::symlink_metadata(transport::SOCKET).map_err(|_| Error::UnsafeEndpoint)?;
    let _socket = OwnedSocket {
        identity: (metadata.dev(), metadata.ino()),
    };
    let path = std::ffi::CString::new(transport::SOCKET).expect("fixed path");
    if unsafe { libc::chown(path.as_ptr(), 0, auth.gid) } != 0 {
        return Err(Error::UnsafeEndpoint);
    }
    fs::set_permissions(transport::SOCKET, fs::Permissions::from_mode(0o660))
        .map_err(|_| Error::UnsafeEndpoint)?;
    transport::socket_metadata(Path::new(transport::SOCKET), 0, auth.gid, 0o660)?;
    let mut host = Host::new(auth.uid, InstalledBackend::new(auth.uid, auth.gid));
    host.lifecycle = Some(lifecycle);
    let host = Arc::new(host);
    let (sender, receiver) = mpsc::sync_channel::<UnixStream>(CONNECTIONS);
    let receiver = Arc::new(Mutex::new(receiver));
    // 固定四条连接 worker；慢客户端最多消耗固定资源和两秒帧预算。
    for _ in 0..CONNECTIONS {
        let host = host.clone();
        let receiver = receiver.clone();
        thread::spawn(move || {
            loop {
                let stream = receiver.lock().expect("connection queue poisoned").recv();
                match stream {
                    Ok(stream) => {
                        let _ = host.connection(stream);
                    }
                    Err(_) => break,
                }
            }
        });
    }
    for stream in listener.incoming() {
        let stream = stream.map_err(|_| Error::Unavailable)?;
        // 满载时关闭新连接；不增加无界线程/排队。
        let _ = sender.try_send(stream);
    }
    Ok(())
}

/// 只生成计划，不创建文件/child。cache 路径由宿主 OS 授权 UID 与受管代际派生。
#[cfg(test)]
pub(super) fn compile_configuration(
    config: &Configuration,
    uid: u32,
) -> Result<veyra_core::singbox::SingBoxPlan, Error> {
    use veyra_core::{
        application::runtime_recovery::cache_generation,
        singbox::{LoopbackListener, ManagedCacheFile, ProductRuntimeResources},
    };
    let generation = cache_generation(&config.state.state_epoch);
    let root = Path::new(transport::ROOT)
        .join("runtime")
        .join(uid.to_string())
        .join(&generation);
    let resources = ProductRuntimeResources {
        mixed: LoopbackListener::new(([127, 0, 0, 1], 0).into())
            .map_err(|_| Error::InvalidConfiguration)?,
        controller: LoopbackListener::new(([127, 0, 0, 1], 0).into())
            .map_err(|_| Error::InvalidConfiguration)?,
        cache: ManagedCacheFile::new(&root, root.join("cache.db"), generation, false, false)
            .map_err(|_| Error::InvalidConfiguration)?,
    };
    config.compile(&resources)
}

struct OwnedSocket {
    identity: (u64, u64),
}
impl Drop for OwnedSocket {
    fn drop(&mut self) {
        if fs::symlink_metadata(transport::SOCKET)
            .is_ok_and(|m| (m.dev(), m.ino()) == self.identity)
        {
            let _ = fs::remove_file(transport::SOCKET);
        }
    }
}
