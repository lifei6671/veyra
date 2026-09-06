use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use futures_util::future::{Either, select};

use super::subscription_management::{SubscriptionManager, SubscriptionOperationError};

const SCHEDULER_INTERVAL: Duration = Duration::from_secs(60);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ScheduledUpdateOutcome {
    pub id: String,
    pub result: Result<bool, SubscriptionOperationError>,
}

/// Owns the application scheduler task and closes every subscription write path before exit.
pub(crate) struct SubscriptionScheduler {
    manager: Arc<SubscriptionManager>,
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    completion: Arc<(Mutex<bool>, Condvar)>,
    shutdown_timeout: Duration,
}

impl SubscriptionScheduler {
    pub(crate) fn start(manager: Arc<SubscriptionManager>) -> Self {
        Self::start_with_timing(manager, SCHEDULER_INTERVAL, SHUTDOWN_TIMEOUT)
    }

    fn start_with_timing(
        manager: Arc<SubscriptionManager>,
        interval: Duration,
        shutdown_timeout: Duration,
    ) -> Self {
        let completion = Arc::new((Mutex::new(false), Condvar::new()));
        let task_manager = Arc::clone(&manager);
        let task_completion = Arc::clone(&completion);
        let task = tauri::async_runtime::spawn(async move {
            scheduler_loop(task_manager, interval).await;
            let (done, wake) = &*task_completion;
            if let Ok(mut done) = done.lock() {
                *done = true;
                wake.notify_all();
            }
        });
        Self {
            manager,
            task: Mutex::new(Some(task)),
            completion,
            shutdown_timeout,
        }
    }

    pub(crate) fn request_stop(&self) {
        self.manager.request_closing();
    }

    /// Returns true only after the coordinator exited and no subscription write or state commit
    /// can still be in flight. A timeout leaves the task owned for a later shutdown attempt.
    pub(crate) fn shutdown(&self) -> bool {
        self.request_stop();
        let deadline = Instant::now() + self.shutdown_timeout;
        if !wait_for_completion(&self.completion, deadline) {
            return false;
        }
        if !self.manager.shutdown_quiescent(deadline) {
            return false;
        }
        self.task
            .lock()
            .expect("subscription scheduler task mutex")
            .take();
        true
    }
}

impl Drop for SubscriptionScheduler {
    fn drop(&mut self) {
        self.request_stop();
    }
}

async fn scheduler_loop(manager: Arc<SubscriptionManager>, interval: Duration) {
    let mut closing = manager.closing_receiver();
    loop {
        match select(
            Box::pin(closing.wait_for(|closing| *closing)),
            Box::pin(tokio::time::sleep(interval)),
        )
        .await
        {
            Either::Left((result, _)) => {
                let _ = result;
                break;
            }
            Either::Right(_) => {}
        }
        if manager.is_closing() {
            break;
        }
        let now_ms = match current_time_ms() {
            Ok(value) => value,
            Err(()) => continue,
        };
        match select(
            Box::pin(closing.wait_for(|closing| *closing)),
            Box::pin(run_scheduler_tick(&manager, now_ms)),
        )
        .await
        {
            Either::Left((result, _)) => {
                let _ = result;
                break;
            }
            Either::Right(_) => {}
        }
    }
}

fn wait_for_completion(completion: &(Mutex<bool>, Condvar), deadline: Instant) -> bool {
    let (done, wake) = completion;
    let Ok(mut done) = done.lock() else {
        return false;
    };
    while !*done {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        let Ok((next, timeout)) = wake.wait_timeout(done, remaining) else {
            return false;
        };
        done = next;
        if timeout.timed_out() && !*done {
            return false;
        }
    }
    true
}

fn current_time_ms() -> Result<u64, ()> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ())?
        .as_millis();
    u64::try_from(millis).map_err(|_| ())
}

/// Runs one application-owned scheduling pass. Each due subscription is attempted once; persisted
/// and process-local deadlines decide whether a later pass may retry it.
pub(crate) async fn run_scheduler_tick(
    manager: &SubscriptionManager,
    now_ms: u64,
) -> Result<Vec<ScheduledUpdateOutcome>, SubscriptionOperationError> {
    let ids = manager.due_subscription_ids(now_ms)?;
    let mut outcomes = Vec::with_capacity(ids.len());
    for id in ids {
        if manager.is_closing() {
            break;
        }
        let result = manager
            .update_with_route(id.clone(), None, None)
            .await
            .map(|updated| updated.content_changed);
        outcomes.push(ScheduledUpdateOutcome { id, result });
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        path::PathBuf,
        sync::mpsc,
        thread,
    };

    use crate::{
        application::{
            state_access::StateAccessGate,
            subscription_management::{
                ImportRequestSource, RemoteImportOptions, SubscriptionImport,
            },
        },
        storage::{JsonStateStore, StateStore},
    };

    use super::*;

    const BODY: &str = r#"{"outbounds":[{"type":"socks","tag":"node","server":"127.0.0.1","server_port":1080,"version":"5"}]}"#;

    fn isolated_state_file(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "veyra-scheduler-{label}-{}-{nanos}",
                std::process::id()
            ))
            .join("state.json")
    }

    fn remote_import(url: String) -> SubscriptionImport {
        SubscriptionImport {
            name: "Remote".to_owned(),
            description: String::new(),
            source: ImportRequestSource::Remote {
                url,
                options: RemoteImportOptions {
                    interval_minutes: Some(1_440),
                    ..RemoteImportOptions::default()
                },
            },
        }
    }

    #[test]
    fn an_empty_due_set_finishes_without_network_or_retry() {
        let path = isolated_state_file("empty");
        let manager =
            SubscriptionManager::new(path.clone(), StateAccessGate::default()).expect("manager");
        let outcomes = tauri::async_runtime::block_on(run_scheduler_tick(&manager, 1_000))
            .expect("empty tick");
        assert!(outcomes.is_empty());
        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn shutdown_cancels_a_manual_fetch_and_waits_for_the_write_guard_and_gate() {
        let state_file = isolated_state_file("manual-cancel");
        let gate = StateAccessGate::default();
        let manager =
            Arc::new(SubscriptionManager::new(state_file.clone(), gate.clone()).expect("manager"));
        let (url, accepted, server) = stalled_server();
        let operation_manager = Arc::clone(&manager);
        let operation = thread::spawn(move || {
            tauri::async_runtime::block_on(
                operation_manager.import_with_options(remote_import(url)),
            )
        });
        accepted.recv().expect("fetch accepted");

        let scheduler = SubscriptionScheduler::start_with_timing(
            Arc::clone(&manager),
            Duration::from_secs(60),
            Duration::from_secs(2),
        );
        assert!(scheduler.shutdown());
        assert_eq!(
            operation.join().expect("manual operation exits"),
            Err(SubscriptionOperationError::Busy)
        );
        assert_eq!(
            manager.begin_write_owned().err(),
            Some(SubscriptionOperationError::Busy)
        );
        assert!(!state_file.exists());
        server.join().expect("stalled fixture exits");
        if let Some(parent) = state_file.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn shutdown_cancels_a_scheduled_fetch_without_a_late_attempt_commit() {
        let state_file = isolated_state_file("scheduled-cancel");
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        let address = listener.local_addr().expect("fixture address");
        let (accepted_tx, accepted_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut first, _) = listener.accept().expect("accept import");
            let mut bytes = [0_u8; 8_192];
            let _ = first.read(&mut bytes).expect("read import");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{BODY}",
                BODY.len()
            );
            first
                .write_all(response.as_bytes())
                .expect("write import response");
            let (mut second, _) = listener.accept().expect("accept scheduled update");
            let _ = second.read(&mut bytes).expect("read scheduled update");
            accepted_tx.send(()).expect("report accepted update");
            thread::sleep(Duration::from_millis(250));
        });
        let import_manager = Arc::new(
            SubscriptionManager::new(state_file.clone(), StateAccessGate::default())
                .expect("manager"),
        );
        tauri::async_runtime::block_on(
            import_manager
                .import_with_options(remote_import(format!("http://{address}/subscription"))),
        )
        .expect("remote import");
        drop(import_manager);
        let store = JsonStateStore::new(state_file.clone()).expect("store");
        let mut due = store.load().expect("load imported state");
        due.subscriptions[0].last_attempt_at_ms = Some(0);
        due.subscriptions[0].last_success_at_ms = Some(0);
        store.save(&due).expect("make subscription due");
        let before = std::fs::read(&state_file).expect("read due state");
        let manager = Arc::new(
            SubscriptionManager::new(state_file.clone(), StateAccessGate::default())
                .expect("scheduler manager"),
        );

        let scheduler = SubscriptionScheduler::start_with_timing(
            Arc::clone(&manager),
            Duration::from_millis(5),
            Duration::from_secs(2),
        );
        accepted_rx.recv().expect("scheduled fetch accepted");
        assert!(scheduler.shutdown());
        assert_eq!(
            std::fs::read(&state_file).expect("read state after shutdown"),
            before
        );
        server.join().expect("fixture exits");
        std::fs::remove_dir_all(state_file.parent().expect("state parent")).expect("cleanup");
    }

    #[test]
    fn shutdown_does_not_report_completion_while_a_write_or_state_gate_is_held() {
        let state_file = isolated_state_file("barrier");
        let gate = StateAccessGate::default();
        let manager =
            Arc::new(SubscriptionManager::new(state_file.clone(), gate.clone()).expect("manager"));
        let write = manager.begin_write_owned().expect("hold write guard");
        let access = gate.try_lock().expect("hold state gate");
        let scheduler = SubscriptionScheduler::start_with_timing(
            Arc::clone(&manager),
            Duration::from_secs(60),
            Duration::from_millis(40),
        );
        assert!(!scheduler.shutdown());
        drop(access);
        drop(write);
        assert!(scheduler.shutdown());
        if let Some(parent) = state_file.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    fn stalled_server() -> (String, mpsc::Receiver<()>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind stalled fixture");
        let address = listener.local_addr().expect("fixture address");
        let (accepted_tx, accepted_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut bytes = [0_u8; 8_192];
            let _ = stream.read(&mut bytes).expect("read request");
            accepted_tx.send(()).expect("report accepted request");
            thread::sleep(Duration::from_millis(250));
        });
        (
            format!("http://{address}/subscription"),
            accepted_rx,
            server,
        )
    }
}
