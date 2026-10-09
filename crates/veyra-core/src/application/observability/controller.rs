//! 单个受管实例的四流采集。只消费 owner 交付的 endpoint，不发现进程或扫描端口。
use crate::application::runtime_snapshot::InstanceId;
use crate::singbox::clash_api::{
    ClashApiClient, ClashLogSummary, FixedStream, ManagedControllerEndpoint, parse_log_message,
};
use crate::storage::traffic::{self, Count, Dimensions, Sample, Source, TrafficWriter};
use futures_util::StreamExt;
use serde::Deserialize;
use std::{
    collections::VecDeque,
    net::IpAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_tungstenite::tungstenite::Message;

const EVENT_CAPACITY: usize = 64;
const LOG_CAPACITY: usize = 1000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Stream {
    Traffic,
    Memory,
    Connections,
    Logs,
}
impl Stream {
    fn index(self) -> usize {
        self as usize
    }
    fn fixed(self) -> FixedStream {
        match self {
            Self::Traffic => FixedStream::Traffic,
            Self::Memory => FixedStream::Memory,
            Self::Connections => FixedStream::Connections,
            Self::Logs => FixedStream::Logs,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Identity {
    pub instance_id: InstanceId,
    pub generation: u64,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Traffic {
    pub interval_ms: u64,
    /// monotonic 到达间隔近似值；并非服务器精确计数窗口。
    pub upload_bytes_per_second: u64,
    pub download_bytes_per_second: u64,
    pub session_upload_bytes: u64,
    pub session_download_bytes: u64,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct ObservedClient {
    #[serde(rename = "sourceIP")]
    pub source_ip: Option<IpAddr>,
    #[serde(rename = "inboundName")]
    pub inbound_name: Option<String>,
}
/// 同一个 /connections 帧的最小只读投影。未知字段不补默认身份或路由。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionRecord {
    pub id: Option<String>,
    pub upload: Option<u64>,
    pub download: Option<u64>,
    /// node 是 API 实际 chains[0] 的 outbound tag，不是推断的业务 NodeId。
    pub dimensions: Dimensions,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionRecords {
    pub identity: Identity,
    pub stream_generation: u64,
    pub sequence: u64,
    pub sampled_at_ms: i64,
    pub records: Vec<ConnectionRecord>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrafficStorageFailure {
    Open,
    Submit,
    Commit,
}
/// 入队与已提交分开报告；缺 ID/计数的记录不生成伪统计。
#[derive(Clone, Debug, Default)]
pub struct TrafficStorageStatus {
    pub active: bool,
    pub writer: traffic::Status,
    pub incomplete_records: u64,
    pub failure: Option<TrafficStorageFailure>,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    pub identity: Option<Identity>,
    pub revision: u64,
    pub available: [bool; 4],
    pub stream_generation: [u64; 4],
    pub sequence: [u64; 4],
    pub gaps: [u64; 4],
    pub traffic: Option<Traffic>,
    pub memory_bytes: Option<u64>,
    pub lifetime_totals: Option<(u64, u64)>,
    pub counter_resets: u64,
    pub connection_count: Option<usize>,
    /// 活动连接的只读投影，无证据时 None；不猜测 MAC、设备名或所有者。
    pub clients: Option<Vec<ObservedClient>>,
    pub connection_records: Option<ConnectionRecords>,
    /// 固定 allowlist 摘要，条数和每条字节均有界；不存原始 payload。
    pub logs: VecDeque<ClashLogSummary>,
}
impl Snapshot {
    fn empty(identity: Option<Identity>, revision: u64) -> Self {
        Self {
            identity,
            revision,
            available: [false; 4],
            stream_generation: [0; 4],
            sequence: [0; 4],
            gaps: [0; 4],
            traffic: None,
            memory_bytes: None,
            lifetime_totals: None,
            counter_resets: 0,
            connection_count: None,
            clients: None,
            connection_records: None,
            logs: VecDeque::new(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Event {
    pub identity: Option<Identity>,
    pub revision: u64,
    pub stream: Option<Stream>,
}
/// 慢订阅读到 Gap 后重新读取 snapshot；队列仅放小型身份/版本通知，不复制大型连接快照。
#[derive(Debug)]
pub enum Delivery {
    Event(Event),
    Gap { dropped: u64 },
    Closed,
}
pub struct Subscription {
    receiver: broadcast::Receiver<Event>,
}
impl Subscription {
    pub async fn recv(&mut self) -> Delivery {
        match self.receiver.recv().await {
            Ok(e) => Delivery::Event(e),
            Err(broadcast::error::RecvError::Lagged(dropped)) => Delivery::Gap { dropped },
            Err(_) => Delivery::Closed,
        }
    }
}
struct State {
    snapshot: Snapshot,
    events: broadcast::Sender<Event>,
    writer: Option<TrafficWriter>,
    traffic_status: TrafficStorageStatus,
}
impl State {
    fn notify(&mut self, stream: Option<Stream>) {
        self.snapshot.revision += 1;
        let _ = self.events.send(Event {
            identity: self.snapshot.identity.clone(),
            revision: self.snapshot.revision,
            stream,
        });
    }
    fn disconnected(&mut self, id: Identity, stream: Stream) {
        if self.snapshot.identity != Some(id.clone()) {
            return;
        }
        let i = stream.index();
        self.snapshot.available[i] = false;
        self.snapshot.gaps[i] += 1;
        match stream {
            Stream::Traffic => {
                if let Some(t) = &mut self.snapshot.traffic {
                    t.interval_ms = 0;
                }
            }
            Stream::Memory => self.snapshot.memory_bytes = None,
            Stream::Connections => {
                self.snapshot.connection_count = None;
                self.snapshot.clients = None;
                self.snapshot.connection_records = None;
            }
            Stream::Logs => {}
        }
        self.notify(Some(stream));
    }
}
/// 由 Runtime owner 持有一个 Service；页面只能 snapshot/subscribe/observed，不建立连接。
/// bind/stop 只切换观测来源，不发 Runtime 命令、不改 Runtime 状态。
pub struct ObservationService {
    state: Arc<Mutex<State>>,
    tasks: Vec<JoinHandle<()>>,
    generation: u64,
    traffic_storage: Option<(PathBuf, chrono_tz::Tz)>,
}
impl Default for ObservationService {
    fn default() -> Self {
        Self::new()
    }
}
impl ObservationService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(EVENT_CAPACITY);
        Self {
            state: Arc::new(Mutex::new(State {
                snapshot: Snapshot::empty(None, 0),
                events,
                writer: None,
                traffic_status: TrafficStorageStatus::default(),
            })),
            tasks: Vec::new(),
            generation: 0,
            traffic_storage: None,
        }
    }
    pub fn snapshot(&self) -> Snapshot {
        self.state
            .lock()
            .expect("observation state")
            .snapshot
            .clone()
    }
    pub fn subscribe(&self) -> Subscription {
        Subscription {
            receiver: self
                .state
                .lock()
                .expect("observation state")
                .events
                .subscribe(),
        }
    }
    pub fn observed(&self) -> Option<Vec<ObservedClient>> {
        self.snapshot().clients
    }
    /// Composition 在首次 Ready 之前指定自己的目录；helper 不默认打开用户统计库。
    pub fn enable_traffic_storage(&mut self, directory: PathBuf, timezone: chrono_tz::Tz) {
        assert!(self.tasks.is_empty(), "configure statistics before Ready");
        self.traffic_storage = Some((directory, timezone));
    }
    pub fn traffic_status(&self) -> TrafficStorageStatus {
        let state = self.state.lock().expect("observation state");
        let mut status = state.traffic_status.clone();
        if let Some(writer) = &state.writer {
            status.writer = writer.status();
            if status.writer.failed {
                status.active = false;
                status.failure = Some(TrafficStorageFailure::Commit);
            }
        }
        status
    }
    pub fn bind(
        &mut self,
        instance_id: InstanceId,
        endpoint: Arc<ManagedControllerEndpoint>,
    ) -> Identity {
        self.stop();
        self.generation += 1;
        let id = Identity {
            instance_id,
            generation: self.generation,
        };
        {
            let mut state = self.state.lock().expect("observation state");
            let rev = state.snapshot.revision;
            state.snapshot = Snapshot::empty(Some(id.clone()), rev);
            if let Some((directory, timezone)) = &self.traffic_storage {
                state.traffic_status = TrafficStorageStatus::default();
                match TrafficWriter::open(directory, *timezone) {
                    Ok(writer) => {
                        state.writer = Some(writer);
                        state.traffic_status.active = true;
                    }
                    Err(_) => {
                        state.traffic_status.failure = Some(TrafficStorageFailure::Open);
                        tracing::warn!("traffic storage unavailable");
                    }
                }
            }
            state.notify(None);
        }
        for stream in [
            Stream::Traffic,
            Stream::Memory,
            Stream::Connections,
            Stream::Logs,
        ] {
            let state = self.state.clone();
            let endpoint = endpoint.clone();
            self.tasks
                .push(tokio::spawn(collect(state, endpoint, id.clone(), stream)));
        }
        id
    }
    pub fn stop(&mut self) {
        for task in self.tasks.drain(..) {
            task.abort();
        }
        let writer = {
            let mut state = self.state.lock().expect("observation state");
            let revision = state.snapshot.revision;
            // 先撤销来源、取走 writer：任何旧任务随后取得锁都不能再提交。
            state.snapshot = Snapshot::empty(None, revision);
            state.traffic_status.active = false;
            state.notify(None);
            state.writer.take()
        };
        if let Some(mut writer) = writer {
            // close 内部 drain/commit/join；离开这里时 DB 与文件 lease 已释放。
            writer.stop();
            let status = writer.status();
            let mut state = self.state.lock().expect("observation state");
            state.traffic_status.writer = status.clone();
            if status.failed {
                state.traffic_status.failure = Some(TrafficStorageFailure::Commit);
                tracing::warn!("traffic storage commit failed during stop");
            }
        }
    }
}
impl Drop for ObservationService {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Deserialize)]
struct TrafficFrame {
    up: u64,
    down: u64,
}
#[derive(Deserialize)]
struct MemoryFrame {
    inuse: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionsFrame {
    upload_total: u64,
    download_total: u64,
    connections: Option<Vec<Connection>>,
}
#[derive(Deserialize)]
struct Connection {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    upload: Option<u64>,
    #[serde(default)]
    download: Option<u64>,
    #[serde(default)]
    chains: Vec<String>,
    metadata: ConnectionMetadata,
}
#[derive(Deserialize)]
struct ConnectionMetadata {
    #[serde(flatten)]
    client: ObservedClient,
    host: Option<String>,
}
fn bounded_field(value: Option<String>) -> Option<String> {
    value.filter(|s| !s.is_empty() && s.len() <= 512)
}
fn submit(state: &mut State, sample: Sample) {
    if let Some(writer) = &state.writer
        && writer.submit(sample).is_err()
    {
        if state.traffic_status.failure.is_none() {
            tracing::warn!("traffic storage rejected a sample");
        }
        state.traffic_status.failure = Some(TrafficStorageFailure::Submit);
    }
}
fn apply(
    state: &mut State,
    id: Identity,
    stream: Stream,
    generation: u64,
    sequence: u64,
    text: &str,
    interval_ms: u64,
) -> Result<(), ()> {
    let s = &mut state.snapshot;
    let sampled_at_ms = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ())?
            .as_millis(),
    )
    .map_err(|_| ())?;
    let mut samples = Vec::new();
    let i = stream.index();
    if s.identity != Some(id.clone())
        || s.stream_generation[i] != generation
        || sequence <= s.sequence[i]
    {
        return Err(());
    }
    match stream {
        Stream::Traffic => {
            let frame: TrafficFrame = serde_json::from_str(text).map_err(|_| ())?;
            let previous = s.traffic.as_ref();
            let upload = previous
                .map_or(0, |t| t.session_upload_bytes)
                .checked_add(frame.up)
                .ok_or(())?;
            let download = previous
                .map_or(0, |t| t.session_download_bytes)
                .checked_add(frame.down)
                .ok_or(())?;
            let rate = |bytes: u64| -> Result<u64, ()> {
                u64::try_from(u128::from(bytes) * 1000 / u128::from(interval_ms.max(1)))
                    .map_err(|_| ())
            };
            s.traffic = Some(Traffic {
                interval_ms,
                upload_bytes_per_second: rate(frame.up)?,
                download_bytes_per_second: rate(frame.down)?,
                session_upload_bytes: upload,
                session_download_bytes: download,
            });
            samples.push((
                "traffic".to_owned(),
                Count::ObservedInterval {
                    upload: frame.up,
                    download: frame.down,
                },
            ));
        }
        Stream::Memory => {
            let bytes = serde_json::from_str::<MemoryFrame>(text)
                .map_err(|_| ())?
                .inuse;
            // 固定 1.14.0 每次订阅的首帧 0 是尚未采样的哨兵，不能冒充真实内存。
            s.memory_bytes = if sequence == 1 && bytes == 0 {
                None
            } else {
                Some(bytes)
            };
        }
        Stream::Connections => {
            let frame: ConnectionsFrame = serde_json::from_str(text).map_err(|_| ())?;
            if s.lifetime_totals
                .is_some_and(|(up, down)| frame.upload_total < up || frame.download_total < down)
            {
                s.counter_resets += 1;
            }
            s.lifetime_totals = Some((frame.upload_total, frame.download_total));
            let connections = frame.connections.unwrap_or_default();
            s.connection_count = Some(connections.len());
            let mut clients = Vec::new();
            let mut records = Vec::with_capacity(connections.len());
            for (index, c) in connections.into_iter().enumerate() {
                let record = ConnectionRecord {
                    id: bounded_field(c.id),
                    upload: c.upload,
                    download: c.download,
                    dimensions: Dimensions {
                        node: bounded_field(c.chains.into_iter().next()),
                        host: bounded_field(c.metadata.host),
                        client: c.metadata.client.source_ip.map(|ip| ip.to_string()),
                        // 1.14.0 API 未发布 outbound type；不把 tag 字符串当 Direct 证据。
                        direct: None,
                    },
                };
                if let (Some(connection_id), Some(upload), Some(download)) =
                    (&record.id, record.upload, record.download)
                {
                    samples.push((
                        format!("connection:{index}"),
                        Count::Connection {
                            id: connection_id.clone(),
                            upload,
                            download,
                            dimensions: record.dimensions.clone(),
                        },
                    ));
                } else {
                    state.traffic_status.incomplete_records += 1;
                }
                records.push(record);
                if c.metadata.client.source_ip.is_some() && !clients.contains(&c.metadata.client) {
                    clients.push(c.metadata.client);
                }
            }
            s.clients = Some(clients);
            s.connection_records = Some(ConnectionRecords {
                identity: id.clone(),
                stream_generation: generation,
                sequence,
                sampled_at_ms,
                records,
            });
        }
        Stream::Logs => {
            let log = parse_log_message(text).map_err(|_| ())?;
            if s.logs.len() == LOG_CAPACITY {
                s.logs.pop_front();
            }
            s.logs.push_back(log);
        }
    }
    s.available[i] = true;
    s.sequence[i] = sequence;
    for (key, count) in samples {
        submit(
            state,
            Sample {
                source: Source {
                    epoch: format!("observation-{}", id.generation),
                    instance: id.instance_id.0.clone(),
                },
                key: format!("{generation}:{sequence}:{key}"),
                start_ms: sampled_at_ms.saturating_sub(i64::try_from(interval_ms).map_err(|_| ())?),
                end_ms: sampled_at_ms,
                count,
            },
        );
    }
    state.notify(Some(stream));
    Ok(())
}
async fn collect(
    state: Arc<Mutex<State>>,
    endpoint: Arc<ManagedControllerEndpoint>,
    id: Identity,
    stream: Stream,
) {
    let Ok(client) = ClashApiClient::managed(&endpoint) else {
        return;
    };
    let mut backoff = Duration::from_millis(100);
    loop {
        if !endpoint.is_valid() {
            state
                .lock()
                .expect("observation state")
                .disconnected(id.clone(), stream);
            return;
        }
        let generation = {
            let mut state = state.lock().expect("observation state");
            if state.snapshot.identity != Some(id.clone()) {
                return;
            }
            state.snapshot.stream_generation[stream.index()] += 1;
            state.snapshot.sequence[stream.index()] = 0;
            state.snapshot.stream_generation[stream.index()]
        };
        if let Ok(Some(mut socket)) = client.open_fixed_stream(stream.fixed()).await {
            let mut previous = Instant::now();
            let mut last_received = previous;
            let mut sequence = 0;
            loop {
                // 日志可以正常沉默；指标断流超时标未知。100ms 检查 endpoint 撤销。
                let next = tokio::time::timeout(Duration::from_millis(100), socket.next()).await;
                if !endpoint.is_valid() {
                    break;
                }
                match next {
                    Err(_)
                        if stream == Stream::Logs
                            || last_received.elapsed() < Duration::from_secs(3) =>
                    {
                        continue;
                    }
                    Ok(Some(Ok(Message::Text(text)))) => {
                        let now = Instant::now();
                        let interval_ms = now.duration_since(previous).as_millis().max(1) as u64;
                        previous = now;
                        last_received = now;
                        sequence += 1;
                        if apply(
                            &mut state.lock().expect("observation state"),
                            id.clone(),
                            stream,
                            generation,
                            sequence,
                            &text,
                            interval_ms,
                        )
                        .is_err()
                        {
                            break;
                        }
                        backoff = Duration::from_millis(100);
                    }
                    Ok(Some(Ok(Message::Ping(_) | Message::Pong(_)))) => continue,
                    _ => break,
                }
            }
            // Drop 释放 socket；不让对端不响应 close 阻塞替换/停止。
        }
        state
            .lock()
            .expect("observation state")
            .disconnected(id.clone(), stream);
        // 采集归属于有效受管实例；snapshot/observed 读者不需要事件订阅也能恢复观测。
        tokio::time::sleep(backoff).await;
        if !endpoint.is_valid() {
            return;
        }
        {
            let state = state.lock().expect("observation state");
            if state.snapshot.identity != Some(id.clone()) {
                return;
            }
        }
        backoff = (backoff * 2).min(Duration::from_secs(2));
    }
}

#[cfg(test)]
mod tests;
