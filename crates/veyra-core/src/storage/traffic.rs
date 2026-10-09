//! P3-04：受管实例的统计存储。输入 seam 只接受已有证据的计数，
//! 不从速率推算字节、不猜测连接身份，也不将两个未对齐来源拼平。
use chrono::{TimeZone, Utc};
use chrono_tz::Tz;
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub const BUCKET_MS: i64 = 60_000;
pub const DETAIL_DAYS: i64 = 7;
pub const SUMMARY_DAYS: i64 = 365;
pub const CAPACITY_BYTES: u64 = 256 * 1024 * 1024;
const QUEUE: usize = 1024;
const BATCH: usize = 256;
const FLUSH: Duration = Duration::from_secs(2);
const MAX_TEXT: usize = 512;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("traffic SQLite: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("traffic filesystem: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid traffic sample")]
    Invalid,
    #[error("traffic writer queue full")]
    Full,
    #[error("traffic writer stopped")]
    Stopped,
    #[error("traffic capacity reached")]
    Capacity,
    #[error("traffic writer failed")]
    Failed,
}

/// 实例必须来自正式受管 Runtime；epoch 与实例共同隔离 restart。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub epoch: String,
    pub instance: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Dimensions {
    pub node: Option<String>,
    pub host: Option<String>,
    pub client: Option<String>,
    pub direct: Option<bool>,
}
#[derive(Clone, Debug)]
pub enum Count {
    /// P0-07 /traffic 已经是区间 bytes，绝对不能再差分。
    ObservedInterval { upload: u64, download: u64 },
    /// 同一稳定连接 ID 的累计量；第一次只建立基线，历史 bytes 不猜归属。
    Connection {
        id: String,
        upload: u64,
        download: u64,
        dimensions: Dimensions,
    },
    /// 仅用于来源已直接确认未归属的 bytes，禁止 total-attributed 截断。
    Unattributed { upload: u64, download: u64 },
}
#[derive(Clone, Debug)]
pub struct Sample {
    pub source: Source,
    /// 来源 generation/sequence 的稳定键；同一来源重放必须复用。
    pub key: String,
    pub start_ms: i64,
    pub end_ms: i64,
    pub count: Count,
}
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub committed: u64,
    pub rejected: u64,
    pub failed_batch: usize,
    pub uncommitted_lost: usize,
    pub failed: bool,
    pub detail_cutoff_ms: Option<i64>,
    pub capacity_shortened: bool,
    pub capacity_blocked: bool,
    pub file_bytes: u64,
}
#[derive(Debug)]
pub struct Usage {
    pub page_count: u64,
    pub freelist_count: u64,
    pub page_size: u64,
    pub database_bytes: u64,
    pub wal_bytes: u64,
    pub shm_bytes: u64,
}
impl Usage {
    pub fn total_bytes(&self) -> u64 {
        self.database_bytes + self.wal_bytes + self.shm_bytes
    }
}

enum Command {
    Sample(Sample),
    Flush(SyncSender<Result<(), Error>>),
    Stop,
}
/// 单线程拥有唯一 SQLite 写连接；队列、单样本长度、批大小均有界。
pub struct TrafficWriter {
    tx: SyncSender<Command>,
    worker: Option<JoinHandle<()>>,
    status: Arc<Mutex<Status>>,
}
impl TrafficWriter {
    pub fn open(directory: &Path, timezone: Tz) -> Result<Self, Error> {
        Self::open_with_capacity(directory, timezone, CAPACITY_BYTES)
    }
    pub fn open_with_capacity(
        directory: &Path,
        timezone: Tz,
        capacity: u64,
    ) -> Result<Self, Error> {
        if capacity == 0 {
            return Err(Error::Invalid);
        }
        std::fs::create_dir_all(directory)?;
        let lease = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("traffic-writer.lock"))?;
        lease.try_lock().map_err(|_| Error::Failed)?;
        let path = directory.join("traffic.sqlite3");
        let mut db = open_db(&path, timezone)?;
        let (tx, rx) = mpsc::sync_channel(QUEUE);
        let restored: Option<(i64, bool)> = db
            .query_row(
                "SELECT cutoff_ms,shortened FROM retention WHERE id=1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        let status = Arc::new(Mutex::new(Status {
            detail_cutoff_ms: restored.map(|r| r.0),
            capacity_shortened: restored.is_some_and(|r| r.1),
            file_bytes: measure(&db, &path)?.total_bytes(),
            ..Status::default()
        }));
        let shared = status.clone();
        let worker = thread::Builder::new()
            .name("traffic-sqlite".into())
            .spawn(move || {
                let _lease = lease;
                let mut batch = Vec::with_capacity(BATCH);
                let mut deadline = Instant::now() + FLUSH;
                loop {
                    let command =
                        rx.recv_timeout(deadline.saturating_duration_since(Instant::now()));
                    match command {
                        Ok(Command::Sample(sample)) => batch.push(sample),
                        Ok(Command::Flush(reply)) => {
                            let result = commit_batch(
                                &mut db, &path, timezone, capacity, &mut batch, &shared,
                            );
                            let failed = result.is_err();
                            let _ = reply.send(result);
                            if failed {
                                break;
                            }
                            deadline = Instant::now() + FLUSH;
                            continue;
                        }
                        Ok(Command::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            let _ = commit_batch(
                                &mut db, &path, timezone, capacity, &mut batch, &shared,
                            );
                            break;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    if batch.len() >= BATCH || Instant::now() >= deadline {
                        if commit_batch(&mut db, &path, timezone, capacity, &mut batch, &shared)
                            .is_err()
                        {
                            break;
                        }
                        deadline = Instant::now() + FLUSH;
                    }
                }
                let queued = rx
                    .try_iter()
                    .filter(|c| matches!(c, Command::Sample(_)))
                    .count();
                shared.lock().unwrap().uncommitted_lost += queued;
            })?;
        Ok(Self {
            tx,
            worker: Some(worker),
            status,
        })
    }
    pub fn submit(&self, sample: Sample) -> Result<(), Error> {
        validate(&sample)?;
        let mut state = self.status.lock().unwrap();
        if state.failed {
            state.rejected += 1;
            return Err(Error::Failed);
        }
        match self.tx.try_send(Command::Sample(sample)) {
            Ok(()) => Ok(()),
            Err(error) => {
                state.rejected += 1;
                Err(match error {
                    TrySendError::Full(_) => Error::Full,
                    TrySendError::Disconnected(_) => Error::Stopped,
                })
            }
        }
    }
    /// 确认前面已接收样本落盘；不把 enqueue 成功当作写入成功。
    pub fn flush(&self) -> Result<(), Error> {
        let (tx, rx) = mpsc::sync_channel(1);
        self.tx
            .send(Command::Flush(tx))
            .map_err(|_| Error::Stopped)?;
        rx.recv().map_err(|_| Error::Stopped)?
    }
    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }
    pub fn close(mut self) -> Result<Status, Error> {
        self.stop();
        let status = self.status();
        if status.failed {
            Err(Error::Failed)
        } else {
            Ok(status)
        }
    }
    fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.tx.send(Command::Stop);
            if worker.join().is_err() {
                self.status.lock().unwrap().failed = true;
            }
        }
    }
}
impl Drop for TrafficWriter {
    fn drop(&mut self) {
        self.stop();
    }
}

fn validate(sample: &Sample) -> Result<(), Error> {
    let valid_text = |s: &str| !s.is_empty() && s.len() <= MAX_TEXT;
    if sample.start_ms < 0
        || sample.end_ms < sample.start_ms
        || !valid_text(&sample.key)
        || !valid_text(&sample.source.epoch)
        || !valid_text(&sample.source.instance)
    {
        return Err(Error::Invalid);
    }
    let (up, down) = match &sample.count {
        Count::Connection {
            id,
            upload,
            download,
            dimensions,
        } => {
            if !valid_text(id)
                || [&dimensions.node, &dimensions.host, &dimensions.client]
                    .iter()
                    .any(|s| s.as_ref().is_some_and(|v| !valid_text(v)))
            {
                return Err(Error::Invalid);
            }
            (*upload, *download)
        }
        Count::ObservedInterval { upload, download } | Count::Unattributed { upload, download } => {
            (*upload, *download)
        }
    };
    if up > i64::MAX as u64
        || down > i64::MAX as u64
        || Utc.timestamp_millis_opt(sample.end_ms).single().is_none()
    {
        return Err(Error::Invalid);
    }
    Ok(())
}

fn open_db(path: &Path, timezone: Tz) -> Result<Connection, Error> {
    let db = Connection::open(path)?;
    db.busy_timeout(Duration::from_secs(2))?;
    let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
    if version != 0 && version != 1 {
        return Err(Error::Invalid);
    }
    // DELETE journal 使容量测量/收尾不遗漏 WAL；FULL 保证已提交事务。
    db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL; PRAGMA auto_vacuum=INCREMENTAL;
        BEGIN IMMEDIATE;
        CREATE TABLE IF NOT EXISTS metadata(timezone TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS seen(epoch TEXT,instance TEXT,key TEXT,end_ms INTEGER,PRIMARY KEY(epoch,instance,key)) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS seen_time ON seen(end_ms);
        CREATE TABLE IF NOT EXISTS counters(epoch TEXT,instance TEXT,id TEXT,end_ms INTEGER,up INTEGER,down INTEGER,node TEXT,host TEXT,client TEXT,direct INTEGER,PRIMARY KEY(epoch,instance,id)) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS counters_time ON counters(end_ms);
        CREATE TABLE IF NOT EXISTS detail(epoch TEXT,instance TEXT,key TEXT,start_ms INTEGER,end_ms INTEGER,bucket_ms INTEGER,day TEXT,kind TEXT,node TEXT,host TEXT,client TEXT,direct INTEGER,up INTEGER CHECK(typeof(up)='integer'),down INTEGER CHECK(typeof(down)='integer'),connections INTEGER);
        CREATE INDEX IF NOT EXISTS detail_window ON detail(epoch,instance,bucket_ms,node,client,host);
        CREATE INDEX IF NOT EXISTS detail_bucket ON detail(bucket_ms);
        CREATE INDEX IF NOT EXISTS detail_time ON detail(end_ms);
        CREATE TABLE IF NOT EXISTS visits(epoch TEXT,instance TEXT,id TEXT,bucket_ms INTEGER,day TEXT,PRIMARY KEY(epoch,instance,id,bucket_ms)) WITHOUT ROWID;
        CREATE INDEX IF NOT EXISTS visits_day ON visits(epoch,instance,id,day);
        CREATE TABLE IF NOT EXISTS day_visits(epoch TEXT,instance TEXT,id TEXT,day TEXT,PRIMARY KEY(epoch,instance,id,day)) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS daily(epoch TEXT,instance TEXT,day TEXT,kind TEXT,direct INTEGER,up INTEGER CHECK(typeof(up)='integer'),down INTEGER CHECK(typeof(down)='integer'),connections INTEGER,PRIMARY KEY(epoch,instance,day,kind,direct)) WITHOUT ROWID;
        CREATE TABLE IF NOT EXISTS retention(id INTEGER PRIMARY KEY CHECK(id=1),cutoff_ms INTEGER,shortened INTEGER);
        PRAGMA user_version=1; COMMIT;")?;
    let old: Option<String> = db
        .query_row("SELECT timezone FROM metadata LIMIT 1", [], |r| r.get(0))
        .optional()?;
    match old {
        Some(old) if old != timezone.name() => return Err(Error::Invalid),
        None => {
            db.execute("INSERT INTO metadata VALUES (?)", [timezone.name()])?;
        }
        _ => {}
    }
    Ok(db)
}

/// 日边界通过 IANA 时区转换，绝不以 UTC 秒数除以 86400 推算本地日期。
pub fn statistical_day(utc_ms: i64, timezone: Tz) -> Result<String, Error> {
    Ok(Utc
        .timestamp_millis_opt(utc_ms)
        .single()
        .ok_or(Error::Invalid)?
        .with_timezone(&timezone)
        .date_naive()
        .to_string())
}
pub fn usage(path: &Path) -> Result<Usage, Error> {
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    measure(&db, path)
}
fn measure(db: &Connection, path: &Path) -> Result<Usage, Error> {
    let bytes = |p: PathBuf| -> Result<u64, Error> {
        match std::fs::metadata(p) {
            Ok(m) => Ok(m.len()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(e.into()),
        }
    };
    Ok(Usage {
        page_count: db
            .pragma_query_value(None, "page_count", |r| r.get::<_, i64>(0).map(|v| v as u64))?,
        freelist_count: db.pragma_query_value(None, "freelist_count", |r| {
            r.get::<_, i64>(0).map(|v| v as u64)
        })?,
        page_size: db
            .pragma_query_value(None, "page_size", |r| r.get::<_, i64>(0).map(|v| v as u64))?,
        database_bytes: bytes(path.to_owned())?,
        wal_bytes: bytes(PathBuf::from(format!("{}-wal", path.display())))?,
        shm_bytes: bytes(PathBuf::from(format!("{}-shm", path.display())))?,
    })
}

fn commit_batch(
    db: &mut Connection,
    path: &Path,
    timezone: Tz,
    capacity: u64,
    batch: &mut Vec<Sample>,
    status: &Mutex<Status>,
) -> Result<(), Error> {
    if batch.is_empty() {
        return Ok(());
    }
    let mut committed = false;
    let result = (|| {
        let latest = batch.iter().map(|s| s.end_ms).max().unwrap();
        let cutoff = latest.saturating_sub(DETAIL_DAYS * 86_400_000);
        let long_day = Utc
            .timestamp_millis_opt(latest)
            .single()
            .ok_or(Error::Invalid)?
            .with_timezone(&timezone)
            .date_naive()
            .checked_sub_days(chrono::Days::new(SUMMARY_DAYS as u64))
            .ok_or(Error::Invalid)?
            .to_string();
        let tx = db.transaction()?;
        tx.execute("DELETE FROM detail WHERE end_ms < ?", [cutoff])?;
        // 去重凭据不能随细节删除，否则旧帧可再次计数。超过保留范围的输入拒绝。
        tx.execute("DELETE FROM seen WHERE end_ms < ?", [cutoff])?;
        // 旧连接checkpoint只在整个可接收窗口已过期后删除，重现时重新建立基线，不重加累计。
        tx.execute("DELETE FROM counters WHERE end_ms < ?", [cutoff])?;
        tx.execute("DELETE FROM daily WHERE day < ?", [&long_day])?;
        tx.execute("DELETE FROM day_visits WHERE day < ?", [&long_day])?;
        tx.execute("DELETE FROM visits WHERE bucket_ms < ?", [cutoff])?;
        for sample in batch.iter() {
            write_sample(&tx, timezone, sample, cutoff)?;
        }
        tx.execute("INSERT INTO retention VALUES (1,?,0) ON CONFLICT(id) DO UPDATE SET cutoff_ms=MAX(cutoff_ms,excluded.cutoff_ms)", [cutoff])?;
        // 所有表和索引一并计量；不能通过只统计明细隐瞒 checkpoint 大小。
        let pages: u64 =
            tx.pragma_query_value(None, "page_count", |r| r.get::<_, i64>(0).map(|v| v as u64))?;
        let free: u64 = tx.pragma_query_value(None, "freelist_count", |r| {
            r.get::<_, i64>(0).map(|v| v as u64)
        })?;
        let size: u64 =
            tx.pragma_query_value(None, "page_size", |r| r.get::<_, i64>(0).map(|v| v as u64))?;
        let mut shortened = false;
        let mut used = (pages - free) * size;
        while used > capacity {
            // 按时间整桶缩短范围，保留所有日总量，绝不 TopN。
            let earlier: Option<i64> =
                tx.query_row("SELECT MIN(bucket_ms) FROM detail", [], |r| r.get(0))?;
            if let Some(earlier) = earlier {
                if earlier >= latest / BUCKET_MS * BUCKET_MS {
                    return Err(Error::Capacity);
                }
                tx.execute("DELETE FROM detail WHERE bucket_ms=?", [earlier])?;
                tx.execute(
                    "UPDATE retention SET cutoff_ms=MAX(cutoff_ms,?),shortened=1 WHERE id=1",
                    [earlier + BUCKET_MS],
                )?;
                tx.execute("DELETE FROM seen WHERE end_ms < ?", [earlier + BUCKET_MS])?;
                tx.execute(
                    "DELETE FROM counters WHERE end_ms < ?",
                    [earlier + BUCKET_MS],
                )?;
                tx.execute(
                    "DELETE FROM visits WHERE bucket_ms < ?",
                    [earlier + BUCKET_MS],
                )?;
                shortened = true;
            }
            let pages: u64 = tx
                .pragma_query_value(None, "page_count", |r| r.get::<_, i64>(0).map(|v| v as u64))?;
            let free: u64 = tx.pragma_query_value(None, "freelist_count", |r| {
                r.get::<_, i64>(0).map(|v| v as u64)
            })?;
            let next_used = (pages - free) * size;
            if earlier.is_none() {
                return Err(Error::Capacity);
            }
            used = next_used;
        }
        tx.commit()?;
        committed = true;
        status.lock().unwrap().committed += batch.len() as u64;
        // incremental_vacuum 每回收一页返回一行，必须排空，execute_batch只step不能完成回收。
        let mut vacuum = db.prepare("PRAGMA incremental_vacuum")?;
        let mut reclaimed = vacuum.query([])?;
        while reclaimed.next()?.is_some() {}
        drop(reclaimed);
        drop(vacuum);
        let actual = measure(db, path)?.total_bytes();
        status.lock().unwrap().file_bytes = actual;
        if actual > capacity {
            return Err(Error::Capacity);
        }
        let (cutoff, persisted_shortened): (i64, bool) = db.query_row(
            "SELECT cutoff_ms,shortened FROM retention WHERE id=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        let mut state = status.lock().unwrap();

        state.detail_cutoff_ms = Some(cutoff);
        state.capacity_shortened |= shortened || persisted_shortened;
        Ok(())
    })();
    if let Err(error) = &result {
        let mut state = status.lock().unwrap();
        state.failed = true;
        state.failed_batch = if committed { 0 } else { batch.len() };
        if !committed {
            state.uncommitted_lost += batch.len();
        }
        state.capacity_blocked = matches!(error, Error::Capacity);
    }
    batch.clear();
    result
}

fn write_sample(tx: &Transaction<'_>, timezone: Tz, s: &Sample, cutoff: i64) -> Result<(), Error> {
    let old_cutoff: Option<i64> = tx
        .query_row("SELECT cutoff_ms FROM retention WHERE id=1", [], |r| {
            r.get(0)
        })
        .optional()?;
    if s.end_ms < cutoff || old_cutoff.is_some_and(|c| s.end_ms < c) {
        return Err(Error::Invalid);
    }
    if tx.execute(
        "INSERT OR IGNORE INTO seen VALUES (?,?,?,?)",
        params![s.source.epoch, s.source.instance, s.key, s.end_ms],
    )? == 0
    {
        return Ok(());
    }
    let mut dimensions = Dimensions::default();
    let mut count = 0;
    let mut daily_count = 0;
    let (kind, up, down, start) = match &s.count {
        Count::ObservedInterval { upload, download } => {
            ("observed", *upload, *download, s.start_ms)
        }
        Count::Unattributed { upload, download } => {
            ("unattributed", *upload, *download, s.start_ms)
        }
        Count::Connection {
            id,
            upload,
            download,
            dimensions: current,
        } => {
            type Baseline = (
                i64,
                u64,
                u64,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<bool>,
            );
            let old: Option<Baseline> = tx.query_row("SELECT end_ms,up,down,node,host,client,direct FROM counters WHERE epoch=? AND instance=? AND id=?", params![s.source.epoch,s.source.instance,id], |r|Ok((r.get(0)?,r.get::<_,i64>(1)? as u64,r.get::<_,i64>(2)? as u64,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?))).optional()?;
            if old.as_ref().is_some_and(|b| b.0 >= s.end_ms) {
                return Ok(());
            }
            tx.execute("INSERT INTO counters VALUES (?,?,?,?,?,?,?,?,?,?) ON CONFLICT(epoch,instance,id) DO UPDATE SET end_ms=excluded.end_ms,up=excluded.up,down=excluded.down,node=excluded.node,host=excluded.host,client=excluded.client,direct=excluded.direct", params![s.source.epoch,s.source.instance,id,s.end_ms,*upload as i64,*download as i64,current.node,current.host,current.client,current.direct])?;
            match old {
                None => {
                    dimensions = current.clone();
                    ("baseline", 0, 0, s.start_ms)
                }
                Some((at, u, d, node, host, client, direct)) if *upload >= u && *download >= d => {
                    if *current
                        == (Dimensions {
                            node,
                            host,
                            client,
                            direct,
                        })
                    {
                        dimensions = current.clone();
                        (
                            if current.node.is_some()
                                && current.host.is_some()
                                && current.client.is_some()
                                && current.direct.is_some()
                            {
                                "attributed"
                            } else {
                                "partial"
                            },
                            upload - u,
                            download - d,
                            at,
                        )
                    } else {
                        ("dimension_gap", upload - u, download - d, at)
                    }
                }
                Some((at, u, d, ..)) => (
                    "counter_reset",
                    upload.checked_sub(u).unwrap_or(0),
                    download.checked_sub(d).unwrap_or(0),
                    at,
                ),
            }
        }
    };
    let day = statistical_day(s.end_ms, timezone)?;
    if let Count::Connection { id, .. } = &s.count {
        count = tx.execute(
            "INSERT OR IGNORE INTO visits VALUES (?,?,?,?,?)",
            params![
                s.source.epoch,
                s.source.instance,
                id,
                s.end_ms / BUCKET_MS * BUCKET_MS,
                day
            ],
        )?;
        daily_count = tx.execute(
            "INSERT OR IGNORE INTO day_visits VALUES (?,?,?,?)",
            params![s.source.epoch, s.source.instance, id, day],
        )?;
    }
    // 跨桶/跨日的累计差值属于测量终点桶；保留真实区间，不能伪造均摊。
    tx.execute(
        "INSERT INTO detail VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)",
        params![
            s.source.epoch,
            s.source.instance,
            s.key,
            start,
            s.end_ms,
            s.end_ms / BUCKET_MS * BUCKET_MS,
            day,
            kind,
            dimensions.node,
            dimensions.host,
            dimensions.client,
            dimensions.direct,
            up as i64,
            down as i64,
            count as i64
        ],
    )?;
    tx.execute("INSERT INTO daily VALUES (?,?,?,?,?,?,?,?) ON CONFLICT(epoch,instance,day,kind,direct) DO UPDATE SET up=up+excluded.up,down=down+excluded.down,connections=connections+excluded.connections", params![s.source.epoch,s.source.instance,day,kind,dimensions.direct.map(i64::from).unwrap_or(-1),up as i64,down as i64,daily_count as i64])?;
    Ok(())
}

#[cfg(test)]
mod tests;
