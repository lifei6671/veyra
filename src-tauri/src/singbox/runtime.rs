//! 受管 sing-box sidecar 的纯内存事务边界。
//!
//! 实际进程、路径和配置文件由后续 Platform Port 实现；本模块只允许固定的
//! `check`/`run` 语义，且从不接收命令、路径、PID 或配置原文。

// TASK-004 故意不把 Port 接入真实 sidecar；生产 wiring 在获得独立资产与运行授权后
// 才能实施，本模块在此之前由自身和 Application 的 Mock 用例覆盖。
#![allow(dead_code)]

use std::num::NonZeroU16;
use thiserror::Error;

use super::GeneratedConfig;

/// Platform 层只能报告无敏感内容的 sidecar 操作失败。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("sidecar port operation failed")]
pub(crate) struct SidecarPortError;

/// 仅由 [`SidecarPort::run`] 产生的受管实例身份。
///
/// 其值不包含 PID，调用方不能构造任意待停止的进程身份。
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedSidecar {
    identity: u64,
}

impl ManagedSidecar {
    /// 仅 Platform Port 可用的内部实例标识，不是操作系统 PID。
    pub(crate) fn from_port_identity(identity: u64) -> Self {
        Self { identity }
    }

    pub(crate) fn identity(&self) -> u64 {
        self.identity
    }
}

/// 固定 sidecar 操作的封闭 Port。
///
/// 实现负责把这组语义映射到应用拥有的目录、已验证的 sidecar 身份以及固定参数；
/// Runtime 不提供执行任意命令、访问任意路径或停止任意 PID 的入口。
pub(crate) trait SidecarPort {
    fn check(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError>;
    fn prepare(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError>;
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError>;
    fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError>;
    fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError>;
}

/// 不暴露配置原文的运行时快照。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SidecarSnapshot {
    pub lifecycle: SidecarLifecycle,
    pub has_active_config: bool,
    pub has_previous_config: bool,
    pub has_candidate_config: bool,
}

/// sidecar 的已验证生命周期状态。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SidecarLifecycle {
    Stopped,
    Ready,
    RecoveryRequired,
}

/// 不携带配置、凭据、路径或平台错误文本的运行时失败。
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum SidecarError {
    #[error("sidecar candidate check failed")]
    CandidateCheck,
    #[error("sidecar candidate preparation failed")]
    CandidatePrepare,
    #[error("sidecar active instance could not be stopped")]
    ActiveStop,
    #[error("sidecar candidate could not be started")]
    CandidateStart,
    #[error("sidecar candidate did not become ready")]
    CandidateReady,
    #[error("sidecar candidate could not be stopped")]
    CandidateStop,
    #[error("previous sidecar configuration could not be restored")]
    Rollback,
}

#[derive(Clone, Default)]
struct ConfigSlots {
    candidate: Option<GeneratedConfig>,
    active: Option<GeneratedConfig>,
    previous: Option<GeneratedConfig>,
}

/// 单一受管 sidecar 的配置替换事务。
pub(crate) struct SidecarRuntime<P> {
    port: P,
    mixed_port: NonZeroU16,
    configs: ConfigSlots,
    child: Option<ManagedSidecar>,
    recovery_required: bool,
}

impl<P> SidecarRuntime<P>
where
    P: SidecarPort,
{
    /// `mixed_port` 只能由后端构造期固定，不从 UI 或运行时字符串取得。
    pub(crate) fn new(port: P, mixed_port: NonZeroU16) -> Self {
        Self {
            port,
            mixed_port,
            configs: ConfigSlots::default(),
            child: None,
            recovery_required: false,
        }
    }

    pub(crate) fn snapshot(&self) -> SidecarSnapshot {
        let lifecycle = if self.recovery_required {
            SidecarLifecycle::RecoveryRequired
        } else if self.child.is_some() {
            SidecarLifecycle::Ready
        } else {
            SidecarLifecycle::Stopped
        };
        SidecarSnapshot {
            lifecycle,
            has_active_config: self.configs.active.is_some(),
            has_previous_config: self.configs.previous.is_some(),
            has_candidate_config: self.configs.candidate.is_some(),
        }
    }

    /// 仅在 sidecar 已通过 Ready 判据后暴露固定 loopback mixed port。
    pub(crate) fn mixed_port(&self) -> Option<NonZeroU16> {
        (self.snapshot().lifecycle == SidecarLifecycle::Ready).then_some(self.mixed_port)
    }

    /// 检查、准备、替换并确认候选；任一步失败均恢复最后一个可证明状态。
    pub(crate) fn start_or_replace(
        &mut self,
        candidate: GeneratedConfig,
    ) -> Result<(), SidecarError> {
        let previous_configs = self.configs.clone();
        self.configs.candidate = Some(candidate);
        let candidate = self
            .configs
            .candidate
            .as_ref()
            .expect("candidate was assigned before check");
        if self.port.check(candidate).is_err() {
            self.configs = previous_configs;
            return Err(SidecarError::CandidateCheck);
        }
        if self.port.prepare(candidate).is_err() {
            self.configs = previous_configs;
            return Err(SidecarError::CandidatePrepare);
        }

        self.promote_candidate();
        let replaced_child = match self.child.take() {
            Some(child) => {
                if self.port.stop(&child).is_err() {
                    self.child = Some(child);
                    self.configs = previous_configs;
                    return Err(SidecarError::ActiveStop);
                }
                Some(child)
            }
            None => None,
        };

        let candidate_child = match self.port.run() {
            Ok(child) => child,
            Err(_) => {
                return self.rollback_after_start_failure(previous_configs, replaced_child);
            }
        };
        if self.port.ready(&candidate_child).is_err() {
            return self.rollback_after_ready_failure(
                previous_configs,
                replaced_child,
                candidate_child,
            );
        }

        self.child = Some(candidate_child);
        self.recovery_required = false;
        Ok(())
    }

    /// 只停止 Runtime 当前持有的受管实例，绝不接受外部 PID 或命令。
    pub(crate) fn stop(&mut self) -> Result<(), SidecarError> {
        let Some(child) = self.child.take() else {
            return Ok(());
        };
        if self.port.stop(&child).is_err() {
            self.child = Some(child);
            return Err(SidecarError::ActiveStop);
        }
        self.recovery_required = false;
        Ok(())
    }

    pub(crate) fn into_port(self) -> P {
        self.port
    }

    fn promote_candidate(&mut self) {
        let candidate = self
            .configs
            .candidate
            .take()
            .expect("candidate was checked and prepared before promotion");
        self.configs.previous = self.configs.active.replace(candidate);
    }

    fn rollback_after_start_failure(
        &mut self,
        previous_configs: ConfigSlots,
        replaced_child: Option<ManagedSidecar>,
    ) -> Result<(), SidecarError> {
        self.configs = previous_configs;
        if self.restore_previous(replaced_child).is_err() {
            return Err(SidecarError::Rollback);
        }
        Err(SidecarError::CandidateStart)
    }

    fn rollback_after_ready_failure(
        &mut self,
        previous_configs: ConfigSlots,
        replaced_child: Option<ManagedSidecar>,
        candidate_child: ManagedSidecar,
    ) -> Result<(), SidecarError> {
        if self.port.stop(&candidate_child).is_err() {
            self.configs = previous_configs;
            self.child = Some(candidate_child);
            self.recovery_required = true;
            return Err(SidecarError::CandidateStop);
        }
        self.configs = previous_configs;
        if self.restore_previous(replaced_child).is_err() {
            return Err(SidecarError::Rollback);
        }
        Err(SidecarError::CandidateReady)
    }

    fn restore_previous(&mut self, replaced_child: Option<ManagedSidecar>) -> Result<(), ()> {
        if replaced_child.is_none() {
            return Ok(());
        }
        let active = self
            .configs
            .active
            .as_ref()
            .expect("a replaced child always has an active configuration");
        if self.port.prepare(active).is_err() {
            return Err(());
        }
        let child = self.port.run().map_err(|_| ())?;
        if self.port.ready(&child).is_err() {
            let _ = self.port.stop(&child);
            return Err(());
        }
        self.child = Some(child);
        self.recovery_required = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::num::NonZeroU16;

    use super::*;
    use crate::{
        domain::{NodeCredentials, NodeId, ProviderId, ProxyNode, ProxyProtocol, RuntimeIntent},
        singbox::{ConfigCompiler, SingBoxCompiler},
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum Event {
        Check,
        Prepare,
        Run,
        Ready(u64),
        Stop(u64),
    }

    #[derive(Default)]
    struct MockPort {
        events: Vec<Event>,
        check: VecDeque<Result<(), SidecarPortError>>,
        prepare: VecDeque<Result<(), SidecarPortError>>,
        run: VecDeque<Result<ManagedSidecar, SidecarPortError>>,
        ready: VecDeque<Result<(), SidecarPortError>>,
        stop: VecDeque<Result<(), SidecarPortError>>,
        next_identity: u64,
    }

    impl MockPort {
        fn result(
            queue: &mut VecDeque<Result<(), SidecarPortError>>,
        ) -> Result<(), SidecarPortError> {
            queue.pop_front().unwrap_or(Ok(()))
        }
    }

    impl SidecarPort for MockPort {
        fn check(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            self.events.push(Event::Check);
            Self::result(&mut self.check)
        }

        fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            self.events.push(Event::Prepare);
            Self::result(&mut self.prepare)
        }

        fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
            self.events.push(Event::Run);
            self.run.pop_front().unwrap_or_else(|| {
                self.next_identity += 1;
                Ok(ManagedSidecar::from_port_identity(self.next_identity))
            })
        }

        fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
            self.events.push(Event::Ready(instance.identity()));
            Self::result(&mut self.ready)
        }

        fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
            self.events.push(Event::Stop(instance.identity()));
            Self::result(&mut self.stop)
        }
    }

    fn config() -> GeneratedConfig {
        SingBoxCompiler
            .compile(&RuntimeIntent {
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
            })
            .expect("fixture configuration")
    }

    fn mixed_port() -> NonZeroU16 {
        NonZeroU16::new(20_890).expect("non-zero fixture port")
    }

    #[test]
    fn starts_in_fixed_check_prepare_run_ready_order() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());

        runtime.start_or_replace(config()).expect("start");

        let port = runtime.into_port();
        assert_eq!(
            port.events,
            vec![Event::Check, Event::Prepare, Event::Run, Event::Ready(1)]
        );
    }

    #[test]
    fn fixed_mixed_port_is_hidden_until_ready() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());

        assert_eq!(runtime.mixed_port(), None);
        runtime.start_or_replace(config()).expect("start");

        assert_eq!(runtime.mixed_port(), Some(mixed_port()));
    }

    #[test]
    fn check_failure_keeps_active_config_and_child() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");
        runtime.port.check.push_back(Err(SidecarPortError));

        let error = runtime
            .start_or_replace(config())
            .expect_err("check failure");

        assert_eq!(error, SidecarError::CandidateCheck);
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        assert!(runtime.snapshot().has_active_config);
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn ready_candidate_becomes_active_and_keeps_the_replaced_config_as_previous() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");

        runtime.start_or_replace(config()).expect("replace stable");

        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        assert!(runtime.snapshot().has_active_config);
        assert!(runtime.snapshot().has_previous_config);
        assert!(!runtime.snapshot().has_candidate_config);
        assert_eq!(
            runtime.port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Check,
                Event::Prepare,
                Event::Stop(1),
                Event::Run,
                Event::Ready(2),
            ]
        );
    }

    #[test]
    fn ready_failure_stops_candidate_and_restores_previous_instance() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");
        runtime.port.ready.push_back(Err(SidecarPortError));

        let error = runtime
            .start_or_replace(config())
            .expect_err("candidate ready failure");

        assert_eq!(error, SidecarError::CandidateReady);
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        assert!(runtime.snapshot().has_active_config);
        assert!(!runtime.snapshot().has_candidate_config);
        assert_eq!(
            runtime.port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Check,
                Event::Prepare,
                Event::Stop(1),
                Event::Run,
                Event::Ready(2),
                Event::Stop(2),
                Event::Prepare,
                Event::Run,
                Event::Ready(3),
            ]
        );
    }

    #[test]
    fn stop_uses_only_the_current_managed_instance() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.start_or_replace(config()).expect("start stable");

        runtime.stop().expect("stop managed instance");
        runtime.stop().expect("stopped runtime is a no-op");

        let port = runtime.into_port();
        assert_eq!(
            port.events,
            vec![
                Event::Check,
                Event::Prepare,
                Event::Run,
                Event::Ready(1),
                Event::Stop(1),
            ]
        );
    }

    #[test]
    fn failed_candidate_stop_requires_recovery_without_leaking_config() {
        let mut runtime = SidecarRuntime::new(MockPort::default(), mixed_port());
        runtime.port.ready.push_back(Err(SidecarPortError));
        runtime.port.stop.push_back(Err(SidecarPortError));

        let error = runtime
            .start_or_replace(config())
            .expect_err("candidate stop failure");

        assert_eq!(error, SidecarError::CandidateStop);
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert!(!error.to_string().contains("fixture-secret"));
    }
}
