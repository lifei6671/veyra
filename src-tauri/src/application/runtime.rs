//! 仅包含 Off 与 System Proxy 的串行运行时协调器。
//!
//! 该层接收已验证的领域运行意图，绝不接收 UI 命令、路径、PID 或原生参数。具体
//! sidecar 与 Windows 代理操作均通过封闭 Port 注入，因此默认构造和单元测试不会
//! 启动外部进程或改写系统代理。

// TASK-004 只验证闭合 Port 的运行时语义，明确不接入 Tauri/UI 或真实生产 Port；
// 在后续授权接线前，这些内部用例会仅由本模块单元测试构造。
#![allow(dead_code)]

use std::sync::Mutex;

use thiserror::Error;

use crate::{
    domain::RuntimeIntent,
    platform::windows::{SystemProxyController, SystemProxyEnableError, SystemProxyRestoreOutcome},
    singbox::{
        ConfigCompiler, SingBoxCompiler,
        runtime::{SidecarLifecycle, SidecarPort, SidecarRuntime},
    },
};

/// 应用可证明的捕获状态。`RecoveryRequired` 不能被静默降级为 Off。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CaptureMode {
    Off,
    SystemProxy,
    RecoveryRequired,
}

/// 进入 System Proxy 后的可观察结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActivateOutcome {
    Activated,
    Reconfigured,
}

/// 退出 System Proxy 后的可观察结果。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DeactivateOutcome {
    Deactivated,
    UserProxyPreserved,
    AlreadyOff,
}

/// 脱敏且面向状态恢复的应用层失败。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum RuntimeSupervisorError {
    #[error("runtime intent could not be compiled")]
    BuildFailed,
    #[error("managed sidecar could not reach a stable ready state")]
    SidecarFailed,
    #[error("Windows system proxy could not be applied")]
    ProxyApplyFailed,
    #[error("Windows system proxy could not be restored")]
    ProxyRestoreFailed,
    #[error("runtime requires recovery before another mode transition")]
    RecoveryRequired,
}

/// 串行协调 sidecar 与 System Proxy 的内部运行态；不持久化到 `AppState`。
pub(crate) struct RuntimeSupervisor<S, P> {
    compiler: SingBoxCompiler,
    sidecar: Mutex<SidecarRuntime<S>>,
    proxy: P,
    state: Mutex<CaptureMode>,
    transition: Mutex<()>,
}

impl<S, P> RuntimeSupervisor<S, P>
where
    S: SidecarPort,
    P: SystemProxyController,
{
    pub(crate) fn new(sidecar: SidecarRuntime<S>, proxy: P) -> Self {
        Self {
            compiler: SingBoxCompiler,
            sidecar: Mutex::new(sidecar),
            proxy,
            state: Mutex::new(CaptureMode::Off),
            transition: Mutex::new(()),
        }
    }

    pub(crate) fn capture_mode(&self) -> CaptureMode {
        self.state
            .lock()
            .map(|state| *state)
            .unwrap_or(CaptureMode::RecoveryRequired)
    }

    /// 在 Off 时先使 sidecar Ready，再改写受控 loopback 代理；在已启用时仅替换
    /// 已验证 sidecar 配置，避免重新覆盖用户系统设置。
    pub(crate) fn activate_system_proxy(
        &self,
        intent: &RuntimeIntent,
    ) -> Result<ActivateOutcome, RuntimeSupervisorError> {
        let _transition = self
            .transition
            .lock()
            .map_err(|_| RuntimeSupervisorError::RecoveryRequired)?;
        match self.capture_mode() {
            CaptureMode::Off => self.activate_from_off(intent),
            CaptureMode::SystemProxy => self.reconfigure(intent),
            CaptureMode::RecoveryRequired => Err(RuntimeSupervisorError::RecoveryRequired),
        }
    }

    /// 先按 Platform Adapter 的语义条件恢复用户代理，再停止当前受管 child。
    pub(crate) fn deactivate_system_proxy(
        &self,
    ) -> Result<DeactivateOutcome, RuntimeSupervisorError> {
        let _transition = self
            .transition
            .lock()
            .map_err(|_| RuntimeSupervisorError::RecoveryRequired)?;
        match self.capture_mode() {
            CaptureMode::Off => Ok(DeactivateOutcome::AlreadyOff),
            CaptureMode::RecoveryRequired => Err(RuntimeSupervisorError::RecoveryRequired),
            CaptureMode::SystemProxy => {
                let restore = self
                    .proxy
                    .restore_proxy()
                    .map_err(|_| RuntimeSupervisorError::ProxyRestoreFailed)?;
                self.stop_sidecar_or_require_recovery()?;
                self.set_state(CaptureMode::Off);
                Ok(match restore {
                    SystemProxyRestoreOutcome::UserModified => {
                        DeactivateOutcome::UserProxyPreserved
                    }
                    SystemProxyRestoreOutcome::Restored | SystemProxyRestoreOutcome::NotManaged => {
                        DeactivateOutcome::Deactivated
                    }
                })
            }
        }
    }

    fn activate_from_off(
        &self,
        intent: &RuntimeIntent,
    ) -> Result<ActivateOutcome, RuntimeSupervisorError> {
        let mixed_port = self.start_sidecar(intent)?;
        match self.proxy.enable_loopback_proxy(mixed_port) {
            Ok(_) => {
                self.set_state(CaptureMode::SystemProxy);
                Ok(ActivateOutcome::Activated)
            }
            Err(SystemProxyEnableError::SafelyUnapplied(_)) => {
                match self.stop_sidecar_or_require_recovery() {
                    Ok(()) => Err(RuntimeSupervisorError::ProxyApplyFailed),
                    Err(_) => Err(RuntimeSupervisorError::RecoveryRequired),
                }
            }
            Err(SystemProxyEnableError::StateUncertain(_)) => {
                self.set_state(CaptureMode::RecoveryRequired);
                Err(RuntimeSupervisorError::RecoveryRequired)
            }
        }
    }

    fn reconfigure(
        &self,
        intent: &RuntimeIntent,
    ) -> Result<ActivateOutcome, RuntimeSupervisorError> {
        self.start_sidecar(intent)?;
        Ok(ActivateOutcome::Reconfigured)
    }

    fn start_sidecar(
        &self,
        intent: &RuntimeIntent,
    ) -> Result<std::num::NonZeroU16, RuntimeSupervisorError> {
        let candidate = self
            .compiler
            .compile(intent)
            .map_err(|_| RuntimeSupervisorError::BuildFailed)?;
        let mut sidecar = self
            .sidecar
            .lock()
            .map_err(|_| RuntimeSupervisorError::RecoveryRequired)?;
        if sidecar.start_or_replace(candidate).is_err() {
            if sidecar.snapshot().lifecycle == SidecarLifecycle::RecoveryRequired {
                self.set_state(CaptureMode::RecoveryRequired);
            }
            return Err(RuntimeSupervisorError::SidecarFailed);
        }
        sidecar
            .mixed_port()
            .ok_or(RuntimeSupervisorError::SidecarFailed)
    }

    fn stop_sidecar_or_require_recovery(&self) -> Result<(), RuntimeSupervisorError> {
        let mut sidecar = self
            .sidecar
            .lock()
            .map_err(|_| RuntimeSupervisorError::RecoveryRequired)?;
        if sidecar.stop().is_err() {
            self.set_state(CaptureMode::RecoveryRequired);
            return Err(RuntimeSupervisorError::RecoveryRequired);
        }
        Ok(())
    }

    fn set_state(&self, state: CaptureMode) {
        if let Ok(mut current) = self.state.lock() {
            *current = state;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        num::NonZeroU16,
        sync::{
            Arc, Mutex,
            mpsc::{self, Receiver, Sender},
        },
        thread,
        time::Duration,
    };

    use super::*;
    use crate::{
        domain::{NodeCredentials, NodeId, ProviderId, ProxyNode, ProxyProtocol},
        platform::windows::{
            recovery::{ProxyRecoveryRecord, ProxyRecoveryStore, RecoveryStoreError},
            system_proxy::{
                ProxySnapshot, ProxyState, SystemProxyEnableError, SystemProxyEnableOutcome,
                SystemProxyError, SystemProxyPort, SystemProxyPortError, WindowsSystemProxy,
            },
        },
        singbox::runtime::{ManagedSidecar, SidecarPortError},
    };

    #[derive(Default)]
    struct MockSidecar {
        events: Arc<Mutex<Vec<&'static str>>>,
        fail_check: bool,
        fail_stop: bool,
        next_identity: u64,
        second_check: Option<Sender<usize>>,
    }

    impl MockSidecar {
        fn record(&self, event: &'static str) {
            self.events.lock().expect("test events").push(event);
        }
    }

    impl SidecarPort for MockSidecar {
        fn check(&mut self, _: &crate::singbox::GeneratedConfig) -> Result<(), SidecarPortError> {
            self.record("check");
            self.next_identity += 1;
            if self.next_identity > 1
                && let Some(second_check) = &self.second_check
            {
                let _ = second_check.send(self.next_identity as usize);
            }
            (!self.fail_check).then_some(()).ok_or(SidecarPortError)
        }

        fn prepare(&mut self, _: &crate::singbox::GeneratedConfig) -> Result<(), SidecarPortError> {
            self.record("prepare");
            Ok(())
        }

        fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
            self.record("run");
            Ok(ManagedSidecar::from_port_identity(self.next_identity))
        }

        fn ready(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
            self.record("ready");
            Ok(())
        }

        fn stop(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
            self.record("stop");
            (!self.fail_stop).then_some(()).ok_or(SidecarPortError)
        }
    }

    struct MockProxy {
        events: Arc<Mutex<Vec<&'static str>>>,
        fail_enable: bool,
        fail_restore: bool,
        restore: SystemProxyRestoreOutcome,
        enable_started: Mutex<Option<Sender<()>>>,
        enable_release: Mutex<Option<Receiver<()>>>,
    }

    impl MockProxy {
        fn new(events: Arc<Mutex<Vec<&'static str>>>) -> Self {
            Self {
                events,
                fail_enable: false,
                fail_restore: false,
                restore: SystemProxyRestoreOutcome::Restored,
                enable_started: Mutex::new(None),
                enable_release: Mutex::new(None),
            }
        }

        fn record(&self, event: &'static str) {
            self.events.lock().expect("test events").push(event);
        }
    }

    impl SystemProxyController for MockProxy {
        fn enable_loopback_proxy(
            &self,
            _: NonZeroU16,
        ) -> Result<SystemProxyEnableOutcome, SystemProxyEnableError> {
            self.record("proxy-enable");
            if let Some(started) = self.enable_started.lock().expect("test gate").take() {
                started.send(()).expect("test receiver");
                self.enable_release
                    .lock()
                    .expect("test gate")
                    .take()
                    .expect("test release gate")
                    .recv()
                    .expect("test release");
            }
            (!self.fail_enable)
                .then_some(SystemProxyEnableOutcome::Enabled)
                .ok_or(SystemProxyEnableError::SafelyUnapplied(
                    SystemProxyError::Write,
                ))
        }

        fn restore_proxy(&self) -> Result<SystemProxyRestoreOutcome, SystemProxyError> {
            self.record("proxy-restore");
            (!self.fail_restore)
                .then_some(self.restore)
                .ok_or(SystemProxyError::Read)
        }

        fn state(&self) -> Result<ProxyState, SystemProxyError> {
            Ok(ProxyState::NotManaged)
        }
    }

    struct ConcreteProxyPort {
        current: ProxySnapshot,
        reads: VecDeque<Result<ProxySnapshot, SystemProxyPortError>>,
        fail_notify: bool,
    }

    impl SystemProxyPort for ConcreteProxyPort {
        fn read_default_connection(&mut self) -> Result<ProxySnapshot, SystemProxyPortError> {
            if let Some(next) = self.reads.pop_front() {
                self.current = next?;
            }
            Ok(self.current.clone())
        }

        fn write_default_connection(
            &mut self,
            state: &ProxySnapshot,
        ) -> Result<(), SystemProxyPortError> {
            self.current = state.clone();
            Ok(())
        }

        fn notify_settings_changed(&mut self) -> Result<(), SystemProxyPortError> {
            (!self.fail_notify)
                .then_some(())
                .ok_or(SystemProxyPortError::NotificationRejected)
        }

        fn refresh_settings(&mut self) -> Result<(), SystemProxyPortError> {
            Ok(())
        }

        fn notify_proxy_settings_changed(&mut self) -> Result<(), SystemProxyPortError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct VolatileRecoveryStore;

    impl ProxyRecoveryStore for VolatileRecoveryStore {
        fn load(&mut self) -> Result<Option<ProxyRecoveryRecord>, RecoveryStoreError> {
            Ok(None)
        }

        fn save(&mut self, _: &ProxyRecoveryRecord) -> Result<(), RecoveryStoreError> {
            Ok(())
        }

        fn clear(&mut self) -> Result<(), RecoveryStoreError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FailStableRecoveryStore {
        saves: usize,
    }

    impl ProxyRecoveryStore for FailStableRecoveryStore {
        fn load(&mut self) -> Result<Option<ProxyRecoveryRecord>, RecoveryStoreError> {
            Ok(None)
        }

        fn save(&mut self, _: &ProxyRecoveryRecord) -> Result<(), RecoveryStoreError> {
            self.saves += 1;
            (self.saves != 2)
                .then_some(())
                .ok_or(RecoveryStoreError::Unavailable)
        }

        fn clear(&mut self) -> Result<(), RecoveryStoreError> {
            Ok(())
        }
    }

    fn original_proxy_snapshot() -> ProxySnapshot {
        ProxySnapshot {
            direct: true,
            proxy_enabled: false,
            proxy_server: None,
            proxy_bypass: None,
            auto_config_url: None,
            auto_config_enabled: false,
            auto_detect: false,
        }
    }

    fn intent() -> RuntimeIntent {
        RuntimeIntent {
            nodes: vec![ProxyNode {
                id: NodeId("node".to_owned()),
                provider_id: ProviderId("provider".to_owned()),
                name: "fixture node".to_owned(),
                protocol: ProxyProtocol::Vless,
                server: "example.invalid".to_owned(),
                port: 443,
                credentials: NodeCredentials::Uuid {
                    uuid: "fixture-secret".to_owned(),
                    flow: None,
                },
                transport: None,
                tls: None,
            }],
            pools: Vec::new(),
            routes: Vec::new(),
        }
    }

    fn supervisor<P>(sidecar: MockSidecar, proxy: P) -> RuntimeSupervisor<MockSidecar, P>
    where
        P: SystemProxyController,
    {
        RuntimeSupervisor::new(
            SidecarRuntime::new(
                sidecar,
                NonZeroU16::new(20_890).expect("non-zero fixture port"),
            ),
            proxy,
        )
    }

    #[test]
    fn activation_waits_for_ready_before_applying_loopback_proxy() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let result = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            MockProxy::new(Arc::clone(&events)),
        )
        .activate_system_proxy(&intent());

        assert_eq!(result, Ok(ActivateOutcome::Activated));
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["check", "prepare", "run", "ready", "proxy-enable"]
        );
    }

    #[test]
    fn sidecar_failure_leaves_system_proxy_untouched() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                fail_check: true,
                ..MockSidecar::default()
            },
            MockProxy::new(Arc::clone(&events)),
        );

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Err(RuntimeSupervisorError::SidecarFailed)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::Off);
        assert_eq!(*events.lock().expect("test events"), vec!["check"]);
    }

    #[test]
    fn proxy_apply_failure_stops_the_ready_sidecar() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            MockProxy {
                events: Arc::clone(&events),
                fail_enable: true,
                fail_restore: false,
                restore: SystemProxyRestoreOutcome::Restored,
                enable_started: Mutex::new(None),
                enable_release: Mutex::new(None),
            },
        );

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Err(RuntimeSupervisorError::ProxyApplyFailed)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::Off);
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["check", "prepare", "run", "ready", "proxy-enable", "stop"]
        );
    }

    #[test]
    fn proxy_apply_compensation_stop_failure_requires_recovery() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                fail_stop: true,
                ..MockSidecar::default()
            },
            MockProxy {
                events: Arc::clone(&events),
                fail_enable: true,
                fail_restore: false,
                restore: SystemProxyRestoreOutcome::Restored,
                enable_started: Mutex::new(None),
                enable_release: Mutex::new(None),
            },
        );

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Err(RuntimeSupervisorError::RecoveryRequired)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::RecoveryRequired);
    }

    #[test]
    fn restore_failure_keeps_sidecar_running_and_mode_stable() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            MockProxy {
                events: Arc::clone(&events),
                fail_enable: false,
                fail_restore: true,
                restore: SystemProxyRestoreOutcome::Restored,
                enable_started: Mutex::new(None),
                enable_release: Mutex::new(None),
            },
        );
        runtime
            .activate_system_proxy(&intent())
            .expect("activate runtime");
        events.lock().expect("test events").clear();

        assert_eq!(
            runtime.deactivate_system_proxy(),
            Err(RuntimeSupervisorError::ProxyRestoreFailed)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::SystemProxy);
        assert_eq!(*events.lock().expect("test events"), vec!["proxy-restore"]);
    }

    #[test]
    fn user_proxy_change_is_preserved_before_stopping_sidecar() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            MockProxy {
                events: Arc::clone(&events),
                fail_enable: false,
                fail_restore: false,
                restore: SystemProxyRestoreOutcome::UserModified,
                enable_started: Mutex::new(None),
                enable_release: Mutex::new(None),
            },
        );
        runtime
            .activate_system_proxy(&intent())
            .expect("activate runtime");
        events.lock().expect("test events").clear();

        assert_eq!(
            runtime.deactivate_system_proxy(),
            Ok(DeactivateOutcome::UserProxyPreserved)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::Off);
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["proxy-restore", "stop"]
        );
    }

    #[test]
    fn reconfiguration_replaces_sidecar_without_reapplying_system_proxy() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            MockProxy::new(Arc::clone(&events)),
        );
        runtime
            .activate_system_proxy(&intent())
            .expect("activate runtime");
        events.lock().expect("test events").clear();

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Ok(ActivateOutcome::Reconfigured)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::SystemProxy);
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["check", "prepare", "stop", "run", "ready"]
        );
    }

    #[test]
    fn concurrent_mode_changes_cannot_interleave_proxy_and_sidecar_operations() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let (proxy_started_tx, proxy_started_rx) = mpsc::channel();
        let (proxy_release_tx, proxy_release_rx) = mpsc::channel();
        let (second_check_tx, second_check_rx) = mpsc::channel();
        let proxy = MockProxy::new(Arc::clone(&events));
        *proxy.enable_started.lock().expect("test gate") = Some(proxy_started_tx);
        *proxy.enable_release.lock().expect("test gate") = Some(proxy_release_rx);
        let runtime = Arc::new(supervisor(
            MockSidecar {
                events,
                second_check: Some(second_check_tx),
                ..MockSidecar::default()
            },
            proxy,
        ));

        let first_runtime = Arc::clone(&runtime);
        let first = thread::spawn(move || first_runtime.activate_system_proxy(&intent()));
        proxy_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("first transition reached the blocked proxy operation");

        let second_runtime = Arc::clone(&runtime);
        let second = thread::spawn(move || second_runtime.activate_system_proxy(&intent()));
        assert!(
            second_check_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err(),
            "a second sidecar check must wait until the first proxy operation finishes"
        );

        proxy_release_tx.send(()).expect("release first transition");
        assert_eq!(
            first.join().expect("first transition thread"),
            Ok(ActivateOutcome::Activated)
        );
        assert_eq!(
            second.join().expect("second transition thread"),
            Ok(ActivateOutcome::Reconfigured)
        );
        assert_eq!(
            second_check_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("second transition eventually starts"),
            2
        );
    }

    #[test]
    fn concrete_proxy_readback_failure_preserves_ready_sidecar_and_requires_recovery() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let original = original_proxy_snapshot();
        let proxy = WindowsSystemProxy::new(
            ConcreteProxyPort {
                current: original.clone(),
                reads: VecDeque::from([Ok(original), Err(SystemProxyPortError::Unavailable)]),
                fail_notify: false,
            },
            VolatileRecoveryStore,
        );
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            proxy,
        );

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Err(RuntimeSupervisorError::RecoveryRequired)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::RecoveryRequired);
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["check", "prepare", "run", "ready"]
        );
    }

    #[test]
    fn concrete_proxy_notification_rollback_failure_preserves_ready_sidecar() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let proxy = WindowsSystemProxy::new(
            ConcreteProxyPort {
                current: original_proxy_snapshot(),
                reads: VecDeque::new(),
                fail_notify: true,
            },
            VolatileRecoveryStore,
        );
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            proxy,
        );

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Err(RuntimeSupervisorError::RecoveryRequired)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::RecoveryRequired);
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["check", "prepare", "run", "ready"]
        );
    }

    #[test]
    fn concrete_proxy_stable_record_failure_preserves_ready_sidecar() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let proxy = WindowsSystemProxy::new(
            ConcreteProxyPort {
                current: original_proxy_snapshot(),
                reads: VecDeque::new(),
                fail_notify: false,
            },
            FailStableRecoveryStore::default(),
        );
        let runtime = supervisor(
            MockSidecar {
                events: Arc::clone(&events),
                ..MockSidecar::default()
            },
            proxy,
        );

        assert_eq!(
            runtime.activate_system_proxy(&intent()),
            Err(RuntimeSupervisorError::RecoveryRequired)
        );
        assert_eq!(runtime.capture_mode(), CaptureMode::RecoveryRequired);
        assert_eq!(
            *events.lock().expect("test events"),
            vec!["check", "prepare", "run", "ready"]
        );
    }
}
