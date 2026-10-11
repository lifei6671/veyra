//! 代理测速的最小 SQLite 历史；沿用本地统计存储的 rusqlite，不保存业务配置。
use crate::{
    application::network::NetworkError,
    domain::{NodeId, OutboundId, StateEpoch},
};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

pub const HISTORY_LIMIT: usize = 10;

/// 零值是已返回的无有效延迟，不能充当成功，也不能与从未测试混淆。
#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum LatencyOutcome {
    Success(u64),
    Zero,
    Unknown,
    Timeout,
    Failed,
}
impl LatencyOutcome {
    pub fn milliseconds(&self) -> Option<u64> {
        match self {
            Self::Success(ms) => Some(*ms),
            _ => None,
        }
    }
    pub(crate) fn from_result(
        result: &Result<crate::application::network::Latency, NetworkError>,
    ) -> Self {
        use crate::{
            singbox::clash_api::ClashApiError,
            subscription::{FetchError, outbound::OutboundError},
        };
        match result {
            Ok(value) => match value.milliseconds {
                Some(0) => Self::Zero,
                Some(ms) => Self::Success(ms),
                None => Self::Unknown,
            },
            Err(
                NetworkError::Controller(ClashApiError::Request(FetchError::Timeout))
                | NetworkError::Request(OutboundError::Fetch(FetchError::Timeout)),
            ) => Self::Timeout,
            Err(
                NetworkError::NodeNotApplied
                | NetworkError::ProxyUnavailable
                | NetworkError::InvalidRequest
                | NetworkError::UnsupportedNodeTestUrl
                | NetworkError::Closed,
            ) => Self::Unknown,
            Err(_) => Self::Failed,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct LatencyRecord {
    pub epoch: StateEpoch,
    pub node: NodeId,
    pub tested_at_ms: i64,
    pub instance_id: String,
    pub generation: u64,
    pub outcome: LatencyOutcome,
}

#[derive(Debug, thiserror::Error)]
pub enum LatencyStoreError {
    #[error("latency SQLite operation failed")]
    Sql(#[from] rusqlite::Error),
    #[error("invalid latency history")]
    Json(#[from] serde_json::Error),
}

/// 由 ProxyService 持有单个连接；短事务原子追加和裁剪，失败不删除旧历史。
pub struct LatencyStore {
    connection: Connection,
}
impl LatencyStore {
    pub fn open(path: &Path) -> Result<Self, LatencyStoreError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(1))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            CREATE TABLE IF NOT EXISTS latency_history (
                seq INTEGER PRIMARY KEY, epoch TEXT NOT NULL, scope TEXT NOT NULL,
                tested_at_ms INTEGER NOT NULL, record TEXT NOT NULL);
            CREATE INDEX IF NOT EXISTS latency_scope ON latency_history(epoch, scope, tested_at_ms DESC, seq DESC);")?;
        Ok(Self { connection })
    }
    pub fn history(
        &self,
        epoch: &StateEpoch,
        scope: &OutboundId,
    ) -> Result<Vec<LatencyRecord>, LatencyStoreError> {
        let mut statement = self.connection.prepare("SELECT record FROM latency_history WHERE epoch=?1 AND scope=?2 ORDER BY tested_at_ms DESC, seq DESC LIMIT 10")?;
        let rows = statement.query_map(
            params![serde_json::to_string(epoch)?, serde_json::to_string(scope)?],
            |row| row.get::<_, String>(0),
        )?;
        rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
    }
    pub fn append(
        &mut self,
        group: Option<&OutboundId>,
        record: &LatencyRecord,
    ) -> Result<(), LatencyStoreError> {
        let epoch = serde_json::to_string(&record.epoch)?;
        let json = serde_json::to_string(record)?;
        let tx = self.connection.transaction()?;
        // 一个实测同时供节点与组的最近十条使用；任意写入失败整笔回滚。
        let node = OutboundId::Node(record.node.clone());
        for scope in std::iter::once(&node).chain(group) {
            let scope = serde_json::to_string(scope)?;
            tx.execute(
                "INSERT INTO latency_history(epoch,scope,tested_at_ms,record) VALUES (?1,?2,?3,?4)",
                params![epoch, scope, record.tested_at_ms, json],
            )?;
            tx.execute("DELETE FROM latency_history WHERE epoch=?1 AND scope=?2 AND seq NOT IN (SELECT seq FROM latency_history WHERE epoch=?1 AND scope=?2 ORDER BY tested_at_ms DESC,seq DESC LIMIT 10)", params![epoch,scope])?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
