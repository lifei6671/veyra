//! 单一串行 worker 同时拥有应用服务和真实 Port；GPUI 只投递封闭命令。
use crate::{platform::manual_sidecar::ManualSidecarPort, state_bridge::AppEvent};
use std::{
    sync::{Arc, mpsc},
    thread,
    time::Duration,
};
use veyra_core::application::{
    manual_runtime::{
        ManualRuntime, ManualRuntimeSnapshot, RuntimeCommand, RuntimeError, RuntimeResult,
    },
    state_service::SnapshotService,
};
#[derive(Clone, Debug)]
pub struct RuntimeEvent {
    pub request: u64,
    pub sequence: u64,
    pub snapshot: Result<ManualRuntimeSnapshot, RuntimeError>,
    pub result: Option<Result<RuntimeResult, RuntimeError>>,
}
enum Message {
    Operation(u64, RuntimeCommand),
    Quit(mpsc::Sender<()>),
}
pub struct RuntimeService {
    sender: mpsc::Sender<Message>,
    worker: Option<thread::JoinHandle<()>>,
}
impl RuntimeService {
    pub fn new(
        root: std::path::PathBuf,
        snapshots: Arc<SnapshotService>,
        events: tokio::sync::mpsc::UnboundedSender<AppEvent>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = thread::spawn(move || {
            let port = ManualSidecarPort::new(root.clone());
            let available = port.kernel_available();
            let mut owner = ManualRuntime::new(port, snapshots, root, available);
            let (mut request, mut sequence) = (0, 0);
            let mut previous = None;
            loop {
                let message = receiver.recv_timeout(Duration::from_millis(250));
                let mut emit = |snapshot, result| {
                    sequence += 1;
                    let _ = events.send(AppEvent::Runtime(RuntimeEvent {
                        request,
                        sequence,
                        snapshot,
                        result,
                    }));
                };
                match message {
                    Ok(Message::Quit(done)) => {
                        let _ = owner.execute(RuntimeCommand::Stop, |_| {});
                        let _ = done.send(());
                        break;
                    }
                    Err(mpsc::RecvTimeoutError::Disconnected) => {
                        let _ = owner.execute(RuntimeCommand::Stop, |_| {});
                        break;
                    }
                    Ok(Message::Operation(id, command)) => {
                        request = id;
                        let result = owner.execute(command, |snapshot| {
                            sequence += 1;
                            let _ = events.send(AppEvent::Runtime(RuntimeEvent {
                                request,
                                sequence,
                                snapshot: Ok(snapshot),
                                result: None,
                            }));
                        });
                        let snapshot = owner.snapshot();
                        previous = Some(snapshot.clone());
                        sequence += 1;
                        eprintln!("manual-runtime operation request={request} result={result:?}");
                        let _ = events.send(AppEvent::Runtime(RuntimeEvent {
                            request,
                            sequence,
                            snapshot,
                            result: Some(result),
                        }));
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) if request != 0 => {
                        let result = owner.execute(RuntimeCommand::Refresh, |_| {});
                        let snapshot = owner.snapshot();
                        if previous.as_ref() != Some(&snapshot) {
                            previous = Some(snapshot.clone());
                            emit(snapshot, result.err().map(Err));
                        }
                    }
                    _ => {}
                }
            }
        });
        Self {
            sender,
            worker: Some(worker),
        }
    }
    pub fn submit(&self, request: u64, command: RuntimeCommand) {
        let _ = self.sender.send(Message::Operation(request, command));
    }
    /// on_app_quit 在后台 await 清理完成；窗口隐藏不调用此方法。
    pub fn shutdown(&self) -> mpsc::Receiver<()> {
        let (tx, rx) = mpsc::channel();
        let _ = self.sender.send(Message::Quit(tx));
        rx
    }
}
impl Drop for RuntimeService {
    fn drop(&mut self) {
        let _ = self.shutdown();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
