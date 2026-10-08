//! 本地管理与只读 HTTP 分离。每次请求读取生产快照，撤销和订阅更新不依赖内容缓存。
use super::state_service::SnapshotService;
use crate::domain::{AppState, ConfigVersion, SubscriptionShare};
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use std::{
    collections::BTreeMap,
    net::{SocketAddr, TcpListener},
    sync::{Arc, Mutex},
};
use tokio::{runtime::Handle, task::JoinHandle};

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ShareError {
    #[error("invalid share configuration")]
    Invalid,
    #[error("share not found")]
    NotFound,
    #[error("state unavailable or changed; retry")]
    Storage,
    #[error("listener bind failed")]
    Bind,
    #[error("secure random source unavailable")]
    Random,
    #[error("selected subscription cannot be exported losslessly")]
    Export,
    #[error("sharing service is closing")]
    Closing,
}
#[derive(Clone, Debug)]
pub enum ShareCommand {
    Save(SubscriptionShare),
    Enabled(String, bool),
    Regenerate(String),
    Delete(String),
}
struct Listener {
    task: JoinHandle<()>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
}
impl Drop for Listener {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
struct Live {
    listeners: BTreeMap<SocketAddr, Listener>,
    closing: bool,
}
pub struct ShareService {
    snapshots: Arc<SnapshotService>,
    handle: Handle,
    live: Mutex<Live>,
}
impl ShareService {
    pub fn new(snapshots: Arc<SnapshotService>, handle: Handle) -> Self {
        Self {
            snapshots,
            handle,
            live: Mutex::new(Live {
                listeners: BTreeMap::new(),
                closing: false,
            }),
        }
    }
    pub fn draft() -> Result<SubscriptionShare, ShareError> {
        Ok(SubscriptionShare {
            id: random()?,
            name: String::new(),
            host: "127.0.0.1:8787".into(),
            protocol: "http".into(),
            listen: "127.0.0.1:8787".parse().unwrap(),
            subscription_ids: vec![],
            token: random()?,
            enabled: true,
        })
    }
    /// 启动/重试恢复保存的 enabled；绑定错误返回 UI，不删除持久配置。
    pub fn restore(&self) -> Result<AppState, ShareError> {
        let mut live = self.live.lock().unwrap();
        if live.closing {
            return Err(ShareError::Closing);
        }
        live.listeners
            .retain(|_, listener| !listener.task.is_finished());
        let state = self.snapshots.snapshot().map_err(|_| ShareError::Storage)?;
        let prepared = self.prepare(&state, &live)?;
        self.publish(&state, &mut live, prepared);
        Ok(state)
    }
    pub fn execute(
        &self,
        expected: ConfigVersion,
        command: ShareCommand,
    ) -> Result<AppState, ShareError> {
        let mut live = self.live.lock().unwrap();
        if live.closing {
            return Err(ShareError::Closing);
        }
        let mut state = self.snapshots.snapshot().map_err(|_| ShareError::Storage)?;
        state
            .config_version()
            .0
            .require(&expected.0)
            .map_err(|_| ShareError::Storage)?;
        let shares = &mut state.app_config.subscription_shares;
        let changed = match &command {
            ShareCommand::Save(s) => Some(s.id.clone()),
            ShareCommand::Enabled(id, true) => Some(id.clone()),
            _ => None,
        };
        match command {
            ShareCommand::Save(mut share) => {
                share.name = share.name.trim().into();
                share.host = share.host.trim().into();
                share.validate().map_err(|_| ShareError::Invalid)?;
                if let Some(old) = shares.iter_mut().find(|s| s.id == share.id) {
                    // 编辑不能绕过单独的 token 更新动作。
                    share.token = old.token.clone();
                    share.enabled = old.enabled;
                    *old = share;
                } else {
                    shares.push(share);
                }
            }
            ShareCommand::Enabled(id, enabled) => find(shares, &id)?.enabled = enabled,
            ShareCommand::Regenerate(id) => find(shares, &id)?.token = random()?,
            ShareCommand::Delete(id) => {
                find(shares, &id)?;
                shares.retain(|s| s.id != id);
            }
        }
        for share in state
            .app_config
            .subscription_shares
            .iter()
            .filter(|s| changed.as_ref() == Some(&s.id))
        {
            if share
                .subscription_ids
                .iter()
                .any(|id| !state.subscriptions.iter().any(|s| &s.id == id))
            {
                return Err(ShareError::Invalid);
            }
            export(&state, share)?;
        }
        // 新端口先 bind、后提交；未发布的 listener 在错误路径直接 drop。
        let prepared = self.prepare(&state, &live)?;
        let state = self
            .snapshots
            .save_shares(expected, state.app_config.subscription_shares)
            .map_err(|_| ShareError::Storage)?;
        self.publish(&state, &mut live, prepared);
        Ok(state)
    }
    fn prepare(
        &self,
        state: &AppState,
        live: &Live,
    ) -> Result<BTreeMap<SocketAddr, TcpListener>, ShareError> {
        let mut prepared = BTreeMap::new();
        for share in state
            .app_config
            .subscription_shares
            .iter()
            .filter(|s| s.enabled)
        {
            if !live.listeners.contains_key(&share.listen) && !prepared.contains_key(&share.listen)
            {
                // 真实 HTTP 连接关闭后的 TIME_WAIT 不应阻止立即重启；不允许多个 listener 共享端口。
                let _entered = self.handle.enter();
                let socket = if share.listen.is_ipv4() {
                    tokio::net::TcpSocket::new_v4()
                } else {
                    tokio::net::TcpSocket::new_v6()
                }
                .map_err(|_| ShareError::Bind)?;
                socket.set_reuseaddr(true).map_err(|_| ShareError::Bind)?;
                socket.bind(share.listen).map_err(|_| ShareError::Bind)?;
                let listener = socket.listen(128).map_err(|_| ShareError::Bind)?;
                prepared.insert(
                    share.listen,
                    listener.into_std().map_err(|_| ShareError::Bind)?,
                );
            }
        }
        Ok(prepared)
    }
    fn publish(
        &self,
        state: &AppState,
        live: &mut Live,
        prepared: BTreeMap<SocketAddr, TcpListener>,
    ) {
        live.listeners.retain(|address, _| {
            state
                .app_config
                .subscription_shares
                .iter()
                .any(|s| s.enabled && &s.listen == address)
        });
        for (address, socket) in prepared {
            let snapshots = self.snapshots.clone();
            let (stop, stopped) = tokio::sync::oneshot::channel();
            let task = self.handle.spawn(async move {
                let listener =
                    tokio::net::TcpListener::from_std(socket).expect("bound nonblocking listener");
                let router = Router::new()
                    .route("/sub/{token}", get(read_share))
                    .with_state((snapshots, address));
                // 路径/token/凭据不记录日志。异常由 restore 的 finished 检查反馈并重试。
                let _ = axum::serve(listener, router)
                    .with_graceful_shutdown(async move {
                        let _ = stopped.await;
                    })
                    .await;
            });
            live.listeners.insert(
                address,
                Listener {
                    task,
                    stop: Some(stop),
                },
            );
        }
    }
    /// AppServices 的退出钩子调用；窗口隐藏不关闭分享。
    pub fn shutdown(&self) {
        let mut live = self.live.lock().unwrap();
        live.closing = true;
        live.listeners.clear();
    }
}
impl Drop for ShareService {
    fn drop(&mut self) {
        self.shutdown();
    }
}
fn find<'a>(
    shares: &'a mut [SubscriptionShare],
    id: &str,
) -> Result<&'a mut SubscriptionShare, ShareError> {
    shares
        .iter_mut()
        .find(|s| s.id == id)
        .ok_or(ShareError::NotFound)
}
fn random() -> Result<String, ShareError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|_| ShareError::Random)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
async fn read_share(
    State((snapshots, address)): State<(Arc<SnapshotService>, SocketAddr)>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(move || {
        let state = snapshots
            .snapshot()
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let share = state
            .app_config
            .subscription_shares
            .iter()
            .find(|s| s.enabled && s.listen == address && s.token == token)
            .ok_or(StatusCode::NOT_FOUND)?;
        export(&state, share).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    })
    .await;
    match result {
        Ok(Ok(body)) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json; charset=utf-8"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            body,
        ),
        Ok(Err(code)) => (
            code,
            [
                (header::CONTENT_TYPE, "text/plain"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            Vec::new(),
        ),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            [
                (header::CONTENT_TYPE, "text/plain"),
                (header::CACHE_CONTROL, "no-store"),
            ],
            Vec::new(),
        ),
    }
}
fn export(state: &AppState, share: &SubscriptionShare) -> Result<Vec<u8>, ShareError> {
    if share
        .subscription_ids
        .iter()
        .any(|id| !state.subscriptions.iter().any(|s| &s.id == id))
    {
        return Err(ShareError::Export);
    }
    let providers: Vec<_> = state
        .providers
        .iter()
        .filter(|p| share.subscription_ids.contains(&p.subscription_id))
        .map(|p| &p.id)
        .collect();
    let nodes: Vec<_> = state
        .nodes
        .iter()
        .filter(|n| providers.contains(&&n.provider_id))
        .collect();
    crate::singbox::subscription_document(&nodes).map_err(|_| ShareError::Export)
}

#[cfg(test)]
mod tests;
