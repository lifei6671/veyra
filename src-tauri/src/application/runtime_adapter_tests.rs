//! Regression tests composing the shared supervisor with the legacy Windows adapter.

use crate::platform::windows::{
    recovery::{ProxyRecoveryRecord, ProxyRecoveryStore, RecoveryStoreError},
    system_proxy::{ProxySnapshot, SystemProxyPort, SystemProxyPortError, WindowsSystemProxy},
};
use std::{
    collections::VecDeque,
    num::NonZeroU16,
    sync::{Arc, Mutex, mpsc::Sender},
};
use veyra_core::{
    application::{runtime::*, system_proxy::*},
    domain::*,
    singbox::runtime::*,
};

#[derive(Default)]
struct MockSidecar {
    events: Arc<Mutex<Vec<&'static str>>>,
    fail_check: bool,
    fail_stop: bool,
    next_identity: u64,
    second_check: Option<Sender<usize>>,
    observation:
        Option<Result<crate::singbox::clash_api::ClashRuntimeObservation, SidecarPortError>>,
}

impl MockSidecar {
    fn record(&self, event: &'static str) {
        self.events.lock().expect("test events").push(event);
    }
}

impl SidecarPort for MockSidecar {
    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        Ok(())
    }

    fn has_pending_cleanup(&self) -> bool {
        false
    }

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

impl RuntimeObservationSidecarPort for MockSidecar {
    fn read_runtime_observation(
        &mut self,
        _: &ManagedSidecar,
    ) -> Result<crate::singbox::clash_api::ClashRuntimeObservation, SidecarPortError> {
        self.record("observe");
        self.observation.take().unwrap_or(Err(SidecarPortError))
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

fn default_target() -> RouteTarget {
    RouteTarget::Pool(crate::domain::PoolId("main".to_owned()))
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
            options: ProtocolOptions::Vless {
                uuid: "00000000-0000-4000-8000-000000000001".to_owned(),
                flow: None,
            },
            transport: None,
            tls: None,
        }],
        pools: vec![crate::domain::RuntimePool {
            id: crate::domain::PoolId("main".to_owned()),
            members: vec![NodeId("node".to_owned())],
            selection: crate::domain::SelectionPolicy::Manual {
                selected_node_id: None,
            },
        }],
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
        runtime.activate_system_proxy(&intent(), &default_target()),
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
        runtime.activate_system_proxy(&intent(), &default_target()),
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
        runtime.activate_system_proxy(&intent(), &default_target()),
        Err(RuntimeSupervisorError::RecoveryRequired)
    );
    assert_eq!(runtime.capture_mode(), CaptureMode::RecoveryRequired);
    assert_eq!(
        *events.lock().expect("test events"),
        vec!["check", "prepare", "run", "ready"]
    );
}
