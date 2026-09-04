//! 受管 sing-box 的固定只读 Clash API client。
//!
//! URL、路径、认证 header 和 secret 均不接受调用方输入。响应只提取可进入安全 DTO 的
//! 数字摘要；连接对象中的目标、进程、规则和链路信息在反序列化时即被丢弃。
// 连接摘要接线将在 SF-002 的观测桥接步骤使用；当前端口先将 Ready 接入启动事务。
#![allow(dead_code)]

use std::time::{Duration, Instant};

use futures_util::StreamExt;
use reqwest::{
    Client,
    header::{AUTHORIZATION, HeaderValue},
};
use serde::{Deserialize, de::IgnoredAny};
use thiserror::Error;
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{
        Message,
        client::IntoClientRequest,
        http::header::{
            AUTHORIZATION as WEBSOCKET_AUTHORIZATION, HeaderValue as WebSocketHeaderValue,
        },
        protocol::WebSocketConfig,
    },
};

use super::managed_sidecar::ApiSecret;

const CLASH_API_ROOT_URL: &str = "http://127.0.0.1:9090/";
const CLASH_API_CONNECTIONS_URL: &str = "http://127.0.0.1:9090/connections/";
const CLASH_API_TRAFFIC_URL: &str = "ws://127.0.0.1:9090/traffic";
const CLASH_API_LOGS_URL: &str = "ws://127.0.0.1:9090/logs";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
const MIN_STREAM_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const MAX_STREAM_FRAME_BYTES: usize = 16 * 1024;
const MAX_STREAM_MESSAGE_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy)]
enum FixedStream {
    Traffic,
    Logs,
}

impl FixedStream {
    const fn url(self) -> &'static str {
        match self {
            Self::Traffic => CLASH_API_TRAFFIC_URL,
            Self::Logs => CLASH_API_LOGS_URL,
        }
    }
}

/// API 响应经最小化提取后的运行数据；不携带连接对象或其它原始响应字段。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClashConnectionSnapshot {
    pub(crate) upload_total_bytes: u64,
    pub(crate) download_total_bytes: u64,
    pub(crate) connection_count: u32,
}

/// 固定核心一秒窗口的字节计数；不含连接或流量内容。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClashTrafficCounters {
    pub(crate) up_bytes: u64,
    pub(crate) down_bytes: u64,
}

/// 安全流量摘要；速率取核心一秒窗口，累计量取此前的 REST 读取时点，并非原子快照。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClashTrafficObservation {
    pub(crate) upload_bytes_per_second: u64,
    pub(crate) download_bytes_per_second: u64,
    pub(crate) upload_total_bytes: u64,
    pub(crate) download_total_bytes: u64,
}

/// 单次 bridge 采样形成的封闭摘要；不携带原始流消息、连接对象或认证材料。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClashRuntimeObservation {
    pub(crate) connections: ClashConnectionSnapshot,
    pub(crate) traffic: Option<ClashTrafficObservation>,
    pub(crate) latest_log: Option<ClashLogSummary>,
}

/// Core 日志经固定 allowlist 映射后的摘要；不保留原始 `type` 或 `payload`。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ClashLogSummary {
    pub(crate) level: ClashLogLevel,
    pub(crate) category: ClashLogCategory,
    pub(crate) message: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClashLogLevel {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ClashLogCategory {
    Runtime,
    Recovery,
}

/// 单个受管 child 专属的串行流摘要 owner。
///
/// 只有拥有该 child 内部 `ApiSecret` 的平台适配器才能调用它。`&mut self` 将同类流的
/// in-flight 读取限制为一条，drop 此对象即可丢弃 child stop/替换后的旧采样状态。
#[derive(Default)]
pub(crate) struct RuntimeObservationBridge {
    last_traffic_sample_at: Option<Instant>,
    last_log_sample_at: Option<Instant>,
}

impl RuntimeObservationBridge {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 只采样固定 REST 摘要及到期的固定流；调用方不得传入网络参数。
    pub(crate) async fn sample(
        &mut self,
        secret: &ApiSecret,
    ) -> Result<ClashRuntimeObservation, ClashApiError> {
        let client = ClashApiClient::new(secret)?;
        let connections = client.read_connections().await?;
        let traffic = if self.traffic_sample_due(Instant::now()) {
            let sampled_at = Instant::now();
            let counters = client.read_traffic_once().await?;
            Some(self.record_traffic_sample(counters, connections, sampled_at))
        } else {
            None
        };
        let latest_log = if self.log_sample_due(Instant::now()) {
            self.last_log_sample_at = Some(Instant::now());
            client.read_log_once().await?
        } else {
            None
        };

        Ok(ClashRuntimeObservation {
            connections,
            traffic,
            latest_log,
        })
    }

    fn traffic_sample_due(&self, now: Instant) -> bool {
        self.last_traffic_sample_at
            .is_none_or(|previous| now.duration_since(previous) >= MIN_STREAM_SAMPLE_INTERVAL)
    }

    fn log_sample_due(&self, now: Instant) -> bool {
        self.last_log_sample_at
            .is_none_or(|previous| now.duration_since(previous) >= MIN_STREAM_SAMPLE_INTERVAL)
    }

    fn record_traffic_sample(
        &mut self,
        counters: ClashTrafficCounters,
        connections: ClashConnectionSnapshot,
        sampled_at: Instant,
    ) -> ClashTrafficObservation {
        self.last_traffic_sample_at = Some(sampled_at);
        ClashTrafficObservation {
            upload_bytes_per_second: counters.up_bytes,
            download_bytes_per_second: counters.down_bytes,
            upload_total_bytes: connections.upload_total_bytes,
            download_total_bytes: connections.download_total_bytes,
        }
    }
}

/// 固定 loopback API 的失败类别；错误文本不包含 endpoint、secret 或原始响应。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum ClashApiError {
    #[error("managed Clash API is unavailable")]
    Unavailable,
    #[error("managed Clash API returned an invalid response")]
    InvalidResponse,
}

/// 只借用当前受管实例的不可复制 secret，不能被构造为任意 HTTP client。
pub(crate) struct ClashApiClient<'secret> {
    client: Client,
    secret: &'secret ApiSecret,
}

impl<'secret> ClashApiClient<'secret> {
    pub(crate) fn new(secret: &'secret ApiSecret) -> Result<Self, ClashApiError> {
        let client = Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| ClashApiError::Unavailable)?;
        Ok(Self { client, secret })
    }

    /// 只接受固定根路径的 JSON hello，既验证认证又作为受管 child 的 loopback Ready 判据。
    pub(crate) async fn read_ready(&self) -> Result<(), ClashApiError> {
        let response: ReadyResponse = self.get_json(CLASH_API_ROOT_URL).await?;
        (response.hello == "clash")
            .then_some(())
            .ok_or(ClashApiError::InvalidResponse)
    }

    /// 只读取累计流量和连接数量；每个连接对象在解析时被 `IgnoredAny` 丢弃。
    pub(crate) async fn read_connections(&self) -> Result<ClashConnectionSnapshot, ClashApiError> {
        let response: ConnectionsResponse = self.get_json(CLASH_API_CONNECTIONS_URL).await?;
        let connection_count = u32::try_from(response.connections.len())
            .map_err(|_| ClashApiError::InvalidResponse)?;
        Ok(ClashConnectionSnapshot {
            upload_total_bytes: response.upload_total,
            download_total_bytes: response.download_total,
            connection_count,
        })
    }

    /// 从固定 `/traffic` 流读取一条核心一秒窗口计数消息，随后只发送协议 Close 帧。
    pub(crate) async fn read_traffic_once(&self) -> Result<ClashTrafficCounters, ClashApiError> {
        let message = self
            .read_single_stream_message(FixedStream::Traffic)
            .await?;
        parse_traffic_message(&message)
    }

    /// 从固定 `/logs` 流读取最多一条消息；在首帧等待周期内没有日志属于正常空摘要。
    pub(crate) async fn read_log_once(&self) -> Result<Option<ClashLogSummary>, ClashApiError> {
        let Some(message) = self.read_single_log_message().await? else {
            return Ok(None);
        };
        parse_log_message(&message).map(Some)
    }

    async fn get_json<T>(&self, url: &'static str) -> Result<T, ClashApiError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let authorization = HeaderValue::from_str(&format!("Bearer {}", self.secret.as_str()))
            .map_err(|_| ClashApiError::Unavailable)?;
        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, authorization)
            .send()
            .await
            .map_err(|_| ClashApiError::Unavailable)?;
        if !response.status().is_success() {
            return Err(ClashApiError::Unavailable);
        }
        response
            .json::<T>()
            .await
            .map_err(|_| ClashApiError::InvalidResponse)
    }

    async fn read_single_stream_message(
        &self,
        stream: FixedStream,
    ) -> Result<String, ClashApiError> {
        #[cfg(test)]
        let mut probe = stream_test_probe::Operation::start(stream_test_probe::Kind::Traffic);
        let mut socket = self
            .open_fixed_stream(stream)
            .await?
            .ok_or(ClashApiError::Unavailable)?;
        let read = socket.next();
        #[cfg(test)]
        let read = probe.watch(read);
        let next = timeout(REQUEST_TIMEOUT, read)
            .await
            .map_err(|_| ClashApiError::Unavailable)?;
        let _ = timeout(REQUEST_TIMEOUT, socket.close(None)).await;
        match next {
            Some(Ok(Message::Text(message))) => Ok(message.to_string()),
            _ => Err(ClashApiError::Unavailable),
        }
    }

    async fn read_single_log_message(&self) -> Result<Option<String>, ClashApiError> {
        #[cfg(test)]
        let mut probe = stream_test_probe::Operation::start(stream_test_probe::Kind::Logs);
        let Some(mut socket) = self.open_fixed_stream(FixedStream::Logs).await? else {
            return Ok(None);
        };
        let read = socket.next();
        #[cfg(test)]
        let read = probe.watch(read);
        let next = timeout(REQUEST_TIMEOUT, read).await;
        let _ = timeout(REQUEST_TIMEOUT, socket.close(None)).await;
        match next {
            Err(_) | Ok(None) => Ok(None),
            Ok(Some(Ok(Message::Text(message)))) => Ok(Some(message.to_string())),
            Ok(Some(Ok(Message::Close(_)))) => Ok(None),
            Ok(Some(Ok(_))) | Ok(Some(Err(_))) => Err(ClashApiError::Unavailable),
        }
    }

    async fn open_fixed_stream(
        &self,
        stream: FixedStream,
    ) -> Result<
        Option<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        ClashApiError,
    > {
        let authorization =
            WebSocketHeaderValue::from_str(&format!("Bearer {}", self.secret.as_str()))
                .map_err(|_| ClashApiError::Unavailable)?;
        let mut request = stream
            .url()
            .into_client_request()
            .map_err(|_| ClashApiError::Unavailable)?;
        request
            .headers_mut()
            .insert(WEBSOCKET_AUTHORIZATION, authorization);
        let configuration = WebSocketConfig::default()
            .max_frame_size(Some(MAX_STREAM_FRAME_BYTES))
            .max_message_size(Some(MAX_STREAM_MESSAGE_BYTES));
        let handshake = timeout(
            REQUEST_TIMEOUT,
            connect_async_with_config(request, Some(configuration), false),
        )
        .await
        .map_err(|_| ClashApiError::Unavailable)?;
        match handshake {
            Ok((socket, _)) => Ok(Some(socket)),
            // 固定核心禁用日志时实际返回 200/零长度；只接受 Logs 的确定空响应。
            Err(tokio_tungstenite::tungstenite::Error::Http(response))
                if matches!(stream, FixedStream::Logs)
                    && response.body().as_ref().is_none_or(|body| body.is_empty())
                    && (response.status().as_u16() == 204
                        || (response.status().as_u16() == 200
                            && response
                                .headers()
                                .get("content-length")
                                .is_some_and(|value| value == "0")
                            && !response.headers().contains_key("transfer-encoding"))) =>
            {
                Ok(None)
            }
            Err(_) => Err(ClashApiError::Unavailable),
        }
    }
}

/// 仅测试线程安装；观察真实socket future的Pending，不替换响应或修改产品超时。
#[cfg(test)]
pub(crate) mod stream_test_probe {
    use std::{
        cell::RefCell,
        future::{Future, poll_fn},
        pin::Pin,
        sync::Arc,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum Kind {
        Traffic,
        Logs,
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) enum Stage {
        Started,
        Pending,
        Finished,
    }
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(crate) struct Event {
        pub(crate) kind: Kind,
        pub(crate) stage: Stage,
    }
    type Observer = Arc<dyn Fn(Event) + Send + Sync>;
    thread_local! { static OBSERVER: RefCell<Option<Observer>> = const { RefCell::new(None) }; }

    pub(crate) struct Installation;
    pub(crate) fn install(observer: Observer) -> Installation {
        OBSERVER.with(|slot| {
            assert!(slot.borrow().is_none(), "one observer per test worker");
            *slot.borrow_mut() = Some(observer);
        });
        Installation
    }
    impl Drop for Installation {
        fn drop(&mut self) {
            OBSERVER.with(|slot| *slot.borrow_mut() = None);
        }
    }
    pub(crate) struct Operation {
        observer: Option<Observer>,
        kind: Kind,
        pending_reported: bool,
    }
    impl Operation {
        pub(crate) fn start(kind: Kind) -> Self {
            let observer = OBSERVER.with(|slot| slot.borrow().clone());
            if let Some(observer) = &observer {
                observer(Event {
                    kind,
                    stage: Stage::Started,
                });
            }
            Self {
                observer,
                kind,
                pending_reported: false,
            }
        }
        pub(crate) fn watch<'a, F: Future + Unpin + 'a>(
            &'a mut self,
            mut future: F,
        ) -> impl Future<Output = F::Output> + 'a {
            poll_fn(move |context| {
                let polled = Pin::new(&mut future).poll(context);
                if polled.is_pending() && !self.pending_reported {
                    self.pending_reported = true;
                    if let Some(observer) = &self.observer {
                        observer(Event {
                            kind: self.kind,
                            stage: Stage::Pending,
                        });
                    }
                }
                polled
            })
        }
    }
    impl Drop for Operation {
        fn drop(&mut self) {
            if let Some(observer) = &self.observer {
                observer(Event {
                    kind: self.kind,
                    stage: Stage::Finished,
                });
            }
        }
    }

    #[test]
    fn pending_probe_reports_once_and_releases_on_cancel_without_leaking_installation() {
        use std::{
            sync::Mutex,
            task::{Context, Waker},
        };
        let events = Arc::new(Mutex::new(Vec::new()));
        {
            let observed = events.clone();
            let _installation =
                install(Arc::new(move |event| observed.lock().unwrap().push(event)));
            let mut operation = Operation::start(Kind::Traffic);
            {
                let mut waiting = Box::pin(operation.watch(std::future::pending::<()>()));
                let mut context = Context::from_waker(Waker::noop());
                assert!(waiting.as_mut().poll(&mut context).is_pending());
                assert!(waiting.as_mut().poll(&mut context).is_pending());
            }
            drop(operation);
        }
        // 安装仅属当前测试线程，离开scope后不能把后续操作记入上一实例的记录。
        drop(Operation::start(Kind::Logs));
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                Event {
                    kind: Kind::Traffic,
                    stage: Stage::Started
                },
                Event {
                    kind: Kind::Traffic,
                    stage: Stage::Pending
                },
                Event {
                    kind: Kind::Traffic,
                    stage: Stage::Finished
                },
            ]
        );
    }
}

#[derive(Deserialize)]
struct ReadyResponse {
    hello: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConnectionsResponse {
    upload_total: u64,
    download_total: u64,
    connections: Vec<IgnoredAny>,
}

#[derive(Deserialize)]
struct TrafficResponse {
    up: u64,
    down: u64,
}

#[derive(Deserialize)]
struct LogResponse {
    #[serde(rename = "type")]
    kind: String,
    payload: IgnoredAny,
}

fn parse_traffic_message(message: &str) -> Result<ClashTrafficCounters, ClashApiError> {
    let response: TrafficResponse =
        serde_json::from_str(message).map_err(|_| ClashApiError::InvalidResponse)?;
    Ok(ClashTrafficCounters {
        up_bytes: response.up,
        down_bytes: response.down,
    })
}

fn parse_log_message(message: &str) -> Result<ClashLogSummary, ClashApiError> {
    let response: LogResponse =
        serde_json::from_str(message).map_err(|_| ClashApiError::InvalidResponse)?;
    let _ = response.payload;
    Ok(match response.kind.as_str() {
        "debug" | "info" => ClashLogSummary {
            level: ClashLogLevel::Info,
            category: ClashLogCategory::Runtime,
            message: "sidecar log observed",
        },
        "warn" | "warning" => ClashLogSummary {
            level: ClashLogLevel::Warning,
            category: ClashLogCategory::Runtime,
            message: "sidecar log observed",
        },
        "error" | "fatal" | "panic" => ClashLogSummary {
            level: ClashLogLevel::Error,
            category: ClashLogCategory::Runtime,
            message: "sidecar log observed",
        },
        _ => ClashLogSummary {
            level: ClashLogLevel::Error,
            category: ClashLogCategory::Recovery,
            message: "sidecar log type rejected",
        },
    })
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    use futures_util::{SinkExt, StreamExt};
    use tokio::time::timeout;
    use tokio_tungstenite::{
        accept_hdr_async,
        tungstenite::{
            Message,
            handshake::server::{Request, Response},
        },
    };

    use super::*;
    use crate::singbox::{
        managed_sidecar::generate_api_secret, test_support::FIXED_CLASH_API_TEST_LOCK,
    };

    #[test]
    fn fixed_client_reads_only_ready_and_connection_summaries() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let listener = TcpListener::bind("127.0.0.1:9090").expect("bind fixed loopback fixture");
        let server = thread::spawn(move || {
            serve_json(&listener, "/", r#"{"hello":"clash"}"#);
            serve_json(
                &listener,
                "/connections/",
                r#"{"uploadTotal":12,"downloadTotal":34,"connections":[{"metadata":{"host":"secret.invalid"}},{"processPath":"C:\\secret.exe"}]}"#,
            );
        });
        let secret = generate_api_secret().expect("system entropy");
        let client = ClashApiClient::new(&secret).expect("construct fixed client");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");

        runtime.block_on(async {
            client.read_ready().await.expect("authenticated ready");
            assert_eq!(
                client
                    .read_connections()
                    .await
                    .expect("safe connection summary"),
                ClashConnectionSnapshot {
                    upload_total_bytes: 12,
                    download_total_bytes: 34,
                    connection_count: 2,
                }
            );
        });
        server.join().expect("fixed API fixture completes");
    }

    #[test]
    fn invalid_api_payload_returns_a_closed_error_without_secret() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let listener = TcpListener::bind("127.0.0.1:9090").expect("bind fixed loopback fixture");
        let server = thread::spawn(move || serve_json(&listener, "/", r#"{"hello":"unexpected"}"#));
        let secret = generate_api_secret().expect("system entropy");
        let client = ClashApiClient::new(&secret).expect("construct fixed client");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");

        let error = runtime
            .block_on(client.read_ready())
            .expect_err("reject unexpected hello");
        server.join().expect("fixed API fixture completes");

        assert_eq!(error, ClashApiError::InvalidResponse);
        assert!(!error.to_string().contains(secret.as_str()));
    }

    #[test]
    fn fixed_traffic_stream_reads_one_authenticated_summary_then_closes() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let secret = generate_api_secret().expect("system entropy");
        let client = ClashApiClient::new(&secret).expect("construct fixed client");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let server = runtime.block_on(spawn_websocket_fixture(
            "/traffic",
            format!("Bearer {}", secret.as_str()),
            Some(r#"{"up":12,"down":34}"#.to_owned()),
        ));

        let counters = runtime
            .block_on(client.read_traffic_once())
            .expect("safe traffic summary");
        runtime
            .block_on(server)
            .expect("fixed WebSocket fixture completes");

        assert_eq!(
            counters,
            ClashTrafficCounters {
                up_bytes: 12,
                down_bytes: 34,
            }
        );
    }

    #[test]
    fn fixed_log_stream_maps_known_and_unknown_types_without_payload() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let secret = generate_api_secret().expect("system entropy");
        let client = ClashApiClient::new(&secret).expect("construct fixed client");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");

        let known_server = runtime.block_on(spawn_websocket_fixture(
            "/logs",
            format!("Bearer {}", secret.as_str()),
            Some(r#"{"type":"warning","payload":"subscription-secret"}"#.to_owned()),
        ));
        assert_eq!(
            runtime
                .block_on(client.read_log_once())
                .expect("known log summary"),
            Some(ClashLogSummary {
                level: ClashLogLevel::Warning,
                category: ClashLogCategory::Runtime,
                message: "sidecar log observed",
            })
        );
        runtime
            .block_on(known_server)
            .expect("known log fixture completes");

        let unknown_server = runtime.block_on(spawn_websocket_fixture(
            "/logs",
            format!("Bearer {}", secret.as_str()),
            Some(r#"{"type":"trace","payload":{"token":"subscription-secret"}}"#.to_owned()),
        ));
        assert_eq!(
            runtime
                .block_on(client.read_log_once())
                .expect("unknown log summary"),
            Some(ClashLogSummary {
                level: ClashLogLevel::Error,
                category: ClashLogCategory::Recovery,
                message: "sidecar log type rejected",
            })
        );
        runtime
            .block_on(unknown_server)
            .expect("unknown log fixture completes");
    }

    #[test]
    fn fixed_logs_accept_only_authenticated_no_content_and_reject_other_handshake_failures() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        const EMPTY: &str = "Content-Length: 0\r\n\r\n";
        const NONEMPTY: &str = "Content-Length: 1\r\n\r\nx";
        const CHUNKED: &str = "Transfer-Encoding: chunked\r\n\r\n0\r\n\r\n";
        for (stream, status, valid_secret, delay_response, content, empty_logs) in [
            (FixedStream::Logs, 204, true, false, EMPTY, true),
            (FixedStream::Logs, 200, true, false, EMPTY, true),
            (FixedStream::Logs, 200, true, false, NONEMPTY, false),
            (FixedStream::Logs, 200, true, false, "\r\n", false),
            (FixedStream::Logs, 200, true, false, CHUNKED, false),
            (
                FixedStream::Logs,
                200,
                true,
                false,
                "Content-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n",
                false,
            ),
            (FixedStream::Traffic, 204, true, false, EMPTY, false),
            (FixedStream::Traffic, 200, true, false, EMPTY, false),
            (FixedStream::Logs, 401, true, false, EMPTY, false),
            (FixedStream::Logs, 500, true, false, EMPTY, false),
            (FixedStream::Logs, 204, false, false, EMPTY, false),
            (FixedStream::Logs, 200, false, false, EMPTY, false),
            (FixedStream::Logs, 200, true, true, EMPTY, false),
        ] {
            let secret = generate_api_secret().expect("client secret");
            let other_secret = generate_api_secret().expect("different fixture secret");
            let expected_authorization = format!(
                "Bearer {}",
                if valid_secret {
                    secret.as_str()
                } else {
                    other_secret.as_str()
                }
            );
            let client = ClashApiClient::new(&secret).expect("fixed client");
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
                    .await
                    .expect("fixed fixture");
                let server = tokio::spawn(async move {
                    let (mut socket, _) = timeout(REQUEST_TIMEOUT, listener.accept())
                        .await
                        .expect("bounded accept")
                        .expect("accept");
                    let mut request = Vec::new();
                    timeout(REQUEST_TIMEOUT, async {
                        while !request.ends_with(b"\r\n\r\n") {
                            assert!(request.len() < 4096, "bounded fixture header");
                            request.push(socket.read_u8().await.expect("request byte"));
                        }
                    })
                    .await
                    .expect("bounded header read");
                    let request = std::str::from_utf8(&request).expect("ASCII headers");
                    let path = match stream {
                        FixedStream::Logs => "/logs",
                        FixedStream::Traffic => "/traffic",
                    };
                    assert!(
                        request.starts_with(&format!("GET {path} HTTP/1.1\r\n")),
                        "fixed request path"
                    );
                    let authenticated = request.lines().any(|line| {
                        line.split_once(':').is_some_and(|(name, value)| {
                            name.eq_ignore_ascii_case("authorization")
                                && value.trim() == expected_authorization
                        })
                    });
                    assert_eq!(
                        authenticated, valid_secret,
                        "fixture authentication outcome"
                    );
                    if delay_response {
                        tokio::time::sleep(REQUEST_TIMEOUT + Duration::from_millis(100)).await;
                        return;
                    }
                    let actual_status = if authenticated { status } else { 401 };
                    let response = format!(
                        "HTTP/1.1 {actual_status} Fixture\r\nConnection: close\r\n{content}"
                    );
                    timeout(REQUEST_TIMEOUT, socket.write_all(response.as_bytes()))
                        .await
                        .expect("bounded response")
                        .expect("write response");
                });
                match stream {
                    FixedStream::Logs if empty_logs => {
                        assert_eq!(client.read_log_once().await, Ok(None))
                    }
                    FixedStream::Logs => assert_eq!(
                        client.read_log_once().await,
                        Err(ClashApiError::Unavailable)
                    ),
                    FixedStream::Traffic => assert_eq!(
                        client.read_traffic_once().await,
                        Err(ClashApiError::Unavailable)
                    ),
                }
                server.await.expect("fixture completes");
            });
        }
    }

    #[test]
    fn fixed_log_stream_treats_an_empty_stream_as_a_normal_empty_summary() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let secret = generate_api_secret().expect("system entropy");
        let client = ClashApiClient::new(&secret).expect("construct fixed client");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let server = runtime.block_on(spawn_websocket_fixture(
            "/logs",
            format!("Bearer {}", secret.as_str()),
            None,
        ));

        assert_eq!(
            runtime
                .block_on(client.read_log_once())
                .expect("empty log stream is not an API failure"),
            None
        );
        runtime
            .block_on(server)
            .expect("empty log fixture completes");
    }

    #[test]
    fn fixed_traffic_stream_rejects_messages_above_the_approved_limit() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let secret = generate_api_secret().expect("system entropy");
        let client = ClashApiClient::new(&secret).expect("construct fixed client");
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        let server = runtime.block_on(spawn_websocket_fixture(
            "/traffic",
            format!("Bearer {}", secret.as_str()),
            Some("x".repeat(MAX_STREAM_MESSAGE_BYTES + 1)),
        ));

        assert_eq!(
            runtime
                .block_on(client.read_traffic_once())
                .expect_err("reject oversized fixed stream message"),
            ClashApiError::Unavailable
        );
        runtime
            .block_on(server)
            .expect("oversized traffic fixture completes");
    }

    #[test]
    fn bridge_maps_each_core_window_and_preserves_rest_totals() {
        let mut bridge = RuntimeObservationBridge::new();
        let first = Instant::now();
        // 首帧、等量、增量、减量、静默及最大值均是独立窗口，不做相邻差分。
        let windows = [
            (100, 200),
            (100, 200),
            (150, 260),
            (10, 20),
            (0, 0),
            (u64::MAX, u64::MAX),
        ];
        for (index, (up, down)) in windows.into_iter().enumerate() {
            let connections = ClashConnectionSnapshot {
                upload_total_bytes: 10_000 + index as u64,
                download_total_bytes: 20_000 + index as u64,
                connection_count: 2,
            };
            assert_eq!(
                bridge.record_traffic_sample(
                    ClashTrafficCounters {
                        up_bytes: up,
                        down_bytes: down
                    },
                    connections,
                    first + Duration::from_secs(index as u64 * 3),
                ),
                ClashTrafficObservation {
                    upload_bytes_per_second: up,
                    download_bytes_per_second: down,
                    upload_total_bytes: connections.upload_total_bytes,
                    download_total_bytes: connections.download_total_bytes,
                }
            );
        }
    }

    #[test]
    fn bridge_uses_monotonic_time_only_for_stream_throttling_and_resets_for_new_child() {
        let mut bridge = RuntimeObservationBridge::new();
        let first = Instant::now();
        assert!(bridge.traffic_sample_due(first));
        assert!(bridge.log_sample_due(first));
        bridge.record_traffic_sample(
            ClashTrafficCounters {
                up_bytes: 100,
                down_bytes: 200,
            },
            ClashConnectionSnapshot {
                upload_total_bytes: 1_000,
                download_total_bytes: 2_000,
                connection_count: 1,
            },
            first,
        );
        bridge.last_log_sample_at = Some(first);
        let too_soon = first + Duration::from_millis(999);
        assert!(!bridge.traffic_sample_due(too_soon));
        assert!(!bridge.log_sample_due(too_soon));
        let due = first + MIN_STREAM_SAMPLE_INTERVAL;
        assert!(bridge.traffic_sample_due(due));
        assert!(bridge.log_sample_due(due));

        let mut replacement = RuntimeObservationBridge::new();
        assert!(replacement.traffic_sample_due(too_soon));
        assert!(replacement.log_sample_due(too_soon));
        assert_eq!(
            replacement.record_traffic_sample(
                ClashTrafficCounters {
                    up_bytes: 7,
                    down_bytes: 9
                },
                ClashConnectionSnapshot {
                    upload_total_bytes: 11,
                    download_total_bytes: 13,
                    connection_count: 0,
                },
                too_soon,
            ),
            ClashTrafficObservation {
                upload_bytes_per_second: 7,
                download_bytes_per_second: 9,
                upload_total_bytes: 11,
                download_total_bytes: 13,
            }
        );
        assert!(!replacement.traffic_sample_due(due));
        assert!(bridge.traffic_sample_due(due));
    }

    #[test]
    fn traffic_payload_rejects_negative_overflow_and_invalid_counters() {
        for message in [
            r#"{"up":-1,"down":0}"#,
            r#"{"up":0,"down":-1}"#,
            r#"{"up":18446744073709551616,"down":0}"#,
            r#"{"up":0,"down":18446744073709551616}"#,
            r#"{"up":1.5,"down":0}"#,
            r#"{"up":"1","down":0}"#,
            r#"{"up":null,"down":0}"#,
            r#"{"unknown":1}"#,
        ] {
            assert_eq!(
                parse_traffic_message(message),
                Err(ClashApiError::InvalidResponse)
            );
        }
        assert_eq!(
            parse_traffic_message(r#"{"up":18446744073709551615,"down":18446744073709551615}"#),
            Ok(ClashTrafficCounters {
                up_bytes: u64::MAX,
                down_bytes: u64::MAX
            })
        );
        for body in [
            r#"{"uploadTotal":-1,"downloadTotal":0,"connections":[]}"#,
            r#"{"uploadTotal":0,"downloadTotal":-1,"connections":[]}"#,
            r#"{"uploadTotal":18446744073709551616,"downloadTotal":0,"connections":[]}"#,
            r#"{"uploadTotal":0,"downloadTotal":18446744073709551616,"connections":[]}"#,
        ] {
            assert!(serde_json::from_str::<ConnectionsResponse>(body).is_err());
        }
    }

    #[test]
    fn bridge_sample_keeps_authenticated_rest_totals_distinct_from_websocket_window() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let secret = generate_api_secret().expect("system entropy");
        let authorization = format!("Bearer {}", secret.as_str());
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
                .await.expect("bind fixed bridge fixture");
            let server = tokio::spawn(async move {
                for path in ["/connections/", "/traffic", "/logs"] {
                    let (mut stream, _) = timeout(REQUEST_TIMEOUT, listener.accept())
                        .await.expect("bounded fixture accept").expect("accept bridge request");
                    if path == "/traffic" {
                        let mut socket = timeout(REQUEST_TIMEOUT, accept_hdr_async(stream, |request: &Request, response: Response| {
                            assert_eq!(request.uri().path(), path);
                            assert!(request.headers().get("authorization").and_then(|v| v.to_str().ok()) == Some(authorization.as_str()), "authenticated bridge traffic");
                            Ok(response)
                        })).await.expect("bounded handshake").expect("traffic handshake");
                        timeout(REQUEST_TIMEOUT, socket.send(Message::Text(r#"{"up":17,"down":23}"#.into())))
                            .await.expect("bounded window send").expect("send core window");
                        assert!(matches!(timeout(REQUEST_TIMEOUT, socket.next()).await, Ok(Some(Ok(Message::Close(_))))));
                    } else {
                        let mut request = Vec::new();
                        timeout(REQUEST_TIMEOUT, async {
                            while !request.ends_with(b"\r\n\r\n") {
                                assert!(request.len() < 4096, "bounded request headers");
                                request.push(stream.read_u8().await.expect("read header byte"));
                            }
                        }).await.expect("bounded request read");
                        let request = std::str::from_utf8(&request).expect("ASCII request");
                        assert!(request.starts_with(&format!("GET {path} HTTP/1.1\r\n")));
                        assert!(request.lines().any(|line| line.split_once(':').is_some_and(|(name, value)| name.eq_ignore_ascii_case("authorization") && value.trim() == authorization)), "authenticated bridge request");
                        let body = r#"{"uploadTotal":1000,"downloadTotal":2000,"connections":[{},{}]}"#;
                        let response = if path == "/connections/" {
                            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                        } else {
                            "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n".to_owned()
                        };
                        timeout(REQUEST_TIMEOUT, stream.write_all(response.as_bytes()))
                            .await.expect("bounded response write").expect("write fixed response");
                    }
                }
            });
            let observation = RuntimeObservationBridge::new().sample(&secret).await.expect("sample same bridge");
            timeout(Duration::from_secs(6), server).await.expect("bounded fixture completion").expect("bridge fixture completes");
            assert_eq!(observation.connections, ClashConnectionSnapshot {
                upload_total_bytes: 1000,
                download_total_bytes: 2000,
                connection_count: 2,
            });
            assert_eq!(observation.traffic, Some(ClashTrafficObservation {
                upload_bytes_per_second: 17,
                download_bytes_per_second: 23,
                upload_total_bytes: 1000,
                download_total_bytes: 2000,
            }));
            assert_eq!(observation.latest_log, None);
        });
    }

    #[test]
    fn traffic_first_frame_timeout_releases_socket_before_fresh_authenticated_sample() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let old_secret = generate_api_secret().expect("old instance entropy");
        let new_secret = generate_api_secret().expect("new instance entropy");
        assert!(
            old_secret.as_str() != new_secret.as_str(),
            "distinct instance secrets"
        );
        let old_authorization = format!("Bearer {}", old_secret.as_str());
        let new_authorization = format!("Bearer {}", new_secret.as_str());
        let runtime = tokio::runtime::Runtime::new().expect("test runtime");
        runtime.block_on(async {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
                .await
                .expect("bind fixed loopback fixture");
            let (expired, mut expiry) = tokio::sync::oneshot::channel();
            let server = tokio::spawn(async move {
                for (index, authorization) in [old_authorization, new_authorization]
                    .into_iter()
                    .enumerate()
                {
                    let (stream, _) = timeout(REQUEST_TIMEOUT, listener.accept())
                        .await
                        .expect("bounded accept")
                        .expect("accept fixed traffic request");
                    let mut socket = timeout(
                        REQUEST_TIMEOUT,
                        accept_hdr_async(stream, |request: &Request, response: Response| {
                            assert_eq!(request.uri().path(), "/traffic");
                            assert!(
                                request
                                    .headers()
                                    .get("authorization")
                                    .and_then(|value| value.to_str().ok())
                                    == Some(authorization.as_str()),
                                "current instance authentication"
                            );
                            Ok(response)
                        }),
                    )
                    .await
                    .expect("bounded handshake")
                    .expect("authenticated handshake");
                    if index == 0 {
                        // 首帧保持未发送，真正等待客户端网络读取期限耗尽，再确认旧连接释放。
                        timeout(Duration::from_secs(4), &mut expiry)
                            .await
                            .expect("bounded timeout notification")
                            .expect("client timeout observed");
                        assert!(
                            matches!(
                                timeout(REQUEST_TIMEOUT, socket.next()).await,
                                Ok(None) | Ok(Some(Err(_))) | Ok(Some(Ok(Message::Close(_))))
                            ),
                            "expired read releases old socket"
                        );
                    } else {
                        timeout(
                            REQUEST_TIMEOUT,
                            socket.send(Message::Text(r#"{"up":17,"down":23}"#.into())),
                        )
                        .await
                        .expect("bounded new frame")
                        .expect("send new instance counters");
                        assert!(
                            matches!(
                                timeout(REQUEST_TIMEOUT, socket.next()).await,
                                Ok(Some(Ok(Message::Close(_))))
                            ),
                            "successful sample closes stream"
                        );
                    }
                }
            });
            let old_client = ClashApiClient::new(&old_secret).expect("old client");
            let started = Instant::now();
            let error = timeout(Duration::from_secs(4), old_client.read_traffic_once())
                .await
                .expect("bounded client failure")
                .expect_err("missing traffic is an error, not zero");
            assert_eq!(error, ClashApiError::Unavailable);
            assert!(started.elapsed() >= REQUEST_TIMEOUT);
            assert!(!error.to_string().contains(old_secret.as_str()));
            expired.send(()).expect("notify completed timeout");
            let new_client = ClashApiClient::new(&new_secret).expect("fresh client");
            let counters = timeout(Duration::from_secs(4), new_client.read_traffic_once())
                .await
                .expect("bounded fresh sample")
                .expect("new instance can sample");
            assert_eq!(
                counters,
                ClashTrafficCounters {
                    up_bytes: 17,
                    down_bytes: 23
                }
            );
            timeout(Duration::from_secs(4), server)
                .await
                .expect("bounded fixture join")
                .expect("fixture completes");
        });
    }

    async fn spawn_websocket_fixture(
        expected_path: &'static str,
        expected_authorization: String,
        message: Option<String>,
    ) -> tokio::task::JoinHandle<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
            .await
            .expect("bind fixed loopback WebSocket fixture");
        tokio::spawn(async move {
            let (stream, _) = listener
                .accept()
                .await
                .expect("accept fixed loopback WebSocket request");
            let mut socket = accept_hdr_async(stream, |request: &Request, response: Response| {
                assert_eq!(request.uri().path(), expected_path);
                assert!(
                    request
                        .headers()
                        .get("authorization")
                        .and_then(|value| value.to_str().ok())
                        == Some(expected_authorization.as_str()),
                    "fixed fixture authorization"
                );
                Ok(response)
            })
            .await
            .expect("complete fixed WebSocket handshake");
            let accepts_connection_reset = message
                .as_ref()
                .is_some_and(|message| message.len() > MAX_STREAM_MESSAGE_BYTES);
            let sent_message = message.is_some();
            if let Some(message) = message {
                socket
                    .send(Message::Text(message.into()))
                    .await
                    .expect("send exactly one fixture message");
            } else {
                socket
                    .close(None)
                    .await
                    .expect("close empty fixture stream");
            }
            if sent_message {
                match timeout(REQUEST_TIMEOUT, socket.next()).await {
                    Ok(Some(Ok(Message::Close(_)))) | Ok(None) => {}
                    Ok(Some(Err(tokio_tungstenite::tungstenite::Error::Io(error))))
                        if accepts_connection_reset
                            && error.kind() == std::io::ErrorKind::ConnectionReset => {}
                    result => panic!("expected client close after one message: {result:?}"),
                }
            }
        })
    }

    fn serve_json(listener: &TcpListener, expected_path: &str, body: &str) {
        let (mut stream, _) = listener.accept().expect("accept fixed loopback request");
        let mut request = [0_u8; 4096];
        let read = stream.read(&mut request).expect("read fixed request");
        let request = std::str::from_utf8(&request[..read]).expect("HTTP is ASCII");
        assert!(request.starts_with(&format!("GET {expected_path} HTTP/1.1\r\n")));
        assert!(
            request
                .to_ascii_lowercase()
                .contains("authorization: bearer ")
        );
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write fixture response");
    }
}
