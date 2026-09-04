//! Windows 上固定打包资源与受管 child 的内部适配器。
//!
//! 不接受前端路径、命令、PID、endpoint 或 secret。它只从 Tauri 已解析的资源根目录和
//! app-local 数据根目录取得固定位置，并将 opaque Runtime identity 映射回自身持有的 child。
#![allow(dead_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    platform::windows::private_runtime::{PrivateRuntime, PrivateRuntimeCreateError},
    singbox::{
        GeneratedConfig,
        clash_api::{ClashApiClient, ClashRuntimeObservation, RuntimeObservationBridge},
        managed_sidecar::{
            ApiSecret, ManagedSidecar as ChildProcess, api_secret_from_config,
            verify_bundled_resources,
        },
        runtime::{ManagedSidecar, RuntimeObservationSidecarPort, SidecarPort, SidecarPortError},
    },
};

const RESOURCE_DIRECTORY: &str = "sing-box/1.14.0";
const RUNTIME_DIRECTORY: &str = "sidecar-runtime";
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
const EXPECTED_RESOURCE_FILES: [&str; 3] = ["LICENSE", "libcronet.dll", "sing-box.exe"];

/// 固定 Windows 1.14.0 bundle 资源的已校验位置。
pub(crate) struct WindowsManagedSidecarPort {
    executable_path: PathBuf,
    library_path: PathBuf,
    license_path: PathBuf,
    runtime_root: PathBuf,
    pending: Option<PendingRuntime>,
    running: BTreeMap<u64, RunningChild>,
    next_identity: u64,
    #[cfg(test)]
    after_instance_created: Option<fn(&Path)>,
}

struct RunningChild {
    child: ChildProcess,
    runtime: PrivateRuntime,
    secret: ApiSecret,
    observation_bridge: RuntimeObservationBridge,
    ready: bool,
}

struct PendingRuntime {
    runtime: PrivateRuntime,
    secret: ApiSecret,
    digest: [u8; 32],
    check_child: Option<ChildProcess>,
    checked: bool,
}

impl WindowsManagedSidecarPort {
    /// 只由应用内部将 Tauri 已解析的 resource/app-local 根目录传入；不接收 UI 数据。
    pub(crate) fn new(
        resource_root: PathBuf,
        app_local_data_root: PathBuf,
    ) -> Result<Self, SidecarPortError> {
        let runtime_root = app_local_data_root.join(RUNTIME_DIRECTORY);
        PrivateRuntime::cleanup_orphaned_instances(&runtime_root).map_err(|_| SidecarPortError)?;

        let resource_root = resource_root.join(RESOURCE_DIRECTORY);
        let executable_path = resource_root.join("sing-box.exe");
        let library_path = resource_root.join("libcronet.dll");
        let license_path = resource_root.join("LICENSE");
        verify_fixed_resources(
            &resource_root,
            &executable_path,
            &library_path,
            &license_path,
        )?;

        Ok(Self {
            executable_path,
            library_path,
            license_path,
            runtime_root,
            pending: None,
            running: BTreeMap::new(),
            next_identity: 0,
            #[cfg(test)]
            after_instance_created: None,
        })
    }

    fn verify_resources(&self) -> Result<(), SidecarPortError> {
        let resource_root = self.executable_path.parent().ok_or(SidecarPortError)?;
        verify_fixed_resources(
            resource_root,
            &self.executable_path,
            &self.library_path,
            &self.license_path,
        )
    }

    fn discard_pending(&mut self) -> Result<(), SidecarPortError> {
        if let Some(pending) = self.pending.as_mut() {
            pending.checked = false;
            if let Some(check) = pending.check_child.as_mut() {
                check.stop().map_err(|_| SidecarPortError)?;
                pending.check_child = None;
            }
            // write_checked_config 的失败路径可能已完成目录清理。
            if pending
                .runtime
                .config_path()
                .parent()
                .ok_or(SidecarPortError)?
                .try_exists()
                .map_err(|_| SidecarPortError)?
            {
                pending.runtime.cleanup().map_err(|_| SidecarPortError)?;
            }
        }
        self.pending = None;
        Ok(())
    }

    fn prepare_checked_config(
        &mut self,
        candidate: &GeneratedConfig,
    ) -> Result<(), SidecarPortError> {
        // 已有候选或未清资源只能显式取消，不能覆盖其所有权。
        if self.pending.is_some() {
            return Err(SidecarPortError);
        }
        self.verify_resources()?;
        let secret = api_secret_from_config(candidate).map_err(|_| SidecarPortError)?;
        let (runtime, preparation_failed) = match PrivateRuntime::create(
            &self.runtime_root,
            #[cfg(test)]
            self.after_instance_created,
        ) {
            Ok(runtime) => (runtime, false),
            Err(PrivateRuntimeCreateError::Preparation) => return Err(SidecarPortError),
            Err(PrivateRuntimeCreateError::CleanupPending(runtime)) => (runtime, true),
        };
        self.pending = Some(PendingRuntime {
            runtime,
            secret,
            digest: Sha256::digest(candidate.as_bytes()).into(),
            check_child: None,
            checked: false,
        });
        if preparation_failed {
            // 创建阶段已经尝试过清理；保留失败 owner，等待用户手动 Stop。
            return Err(SidecarPortError);
        }
        let result = self.check_pending(candidate);
        if result.is_err() {
            // check 超时且退出未确认时不叠加自动停止；保留原句柄等待手动 Stop。
            if self
                .pending
                .as_ref()
                .is_some_and(|pending| pending.check_child.is_some())
            {
                return result;
            }
            self.discard_pending()?;
        }
        result
    }

    fn check_pending(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError> {
        let pending = self.pending.as_mut().ok_or(SidecarPortError)?;
        pending
            .runtime
            .write_checked_config(candidate)
            .map_err(|_| SidecarPortError)?;
        if !matches_final_bytes(pending.runtime.config_path(), candidate) {
            return Err(SidecarPortError);
        }
        pending.check_child = Some(
            ChildProcess::start_check(&self.executable_path, pending.runtime.config_path())
                .map_err(|_| SidecarPortError)?,
        );
        let check = pending
            .check_child
            .as_mut()
            .expect("check child was assigned");
        let result = check.wait_check();
        if check.is_running().map_err(|_| SidecarPortError)? {
            return Err(SidecarPortError);
        }
        pending.check_child = None;
        result.map_err(|_| SidecarPortError)?;
        if !matches_final_bytes(pending.runtime.config_path(), candidate) {
            return Err(SidecarPortError);
        }
        pending.checked = true;
        Ok(())
    }
}

impl SidecarPort for WindowsManagedSidecarPort {
    /// 在停止旧 child 前完成 private config + 固定 `sing-box check`，失败不触及旧实例。
    fn check(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError> {
        self.prepare_checked_config(candidate)
    }

    /// `check` 已验证并保存最终配置及其绑定的 secret；这里拒绝任何缺失或越序的 pending 状态。
    fn prepare(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError> {
        let pending = self.pending.as_ref().ok_or(SidecarPortError)?;
        if !pending.checked
            || pending.digest != <[u8; 32]>::from(Sha256::digest(candidate.as_bytes()))
            || !matches_final_bytes(pending.runtime.config_path(), candidate)
        {
            return Err(SidecarPortError);
        }
        Ok(())
    }

    /// 已检查 pending 只消费一次；run 前再次核对最终字节，失败资源留在原 owner。
    fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        let result = self.run_pending();
        if result.is_err() {
            self.discard_pending()?;
        }
        result
    }

    fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
        self.discard_pending()
    }

    fn has_pending_cleanup(&self) -> bool {
        self.pending
            .as_ref()
            .is_some_and(|pending| !pending.checked)
    }

    /// Ready 必须同时证明当前 child 存活且已通过固定认证的 loopback API；不接受任意地址或 header。
    fn ready(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
        let running = self
            .running
            .get_mut(&instance.identity())
            .ok_or(SidecarPortError)?;
        running
            .child
            .is_running()
            .map_err(|_| SidecarPortError)?
            .then_some(())
            .ok_or(SidecarPortError)?;
        let client = ClashApiClient::new(&running.secret).map_err(|_| SidecarPortError)?;
        tauri::async_runtime::block_on(client.read_ready())
            .map(|_| running.ready = true)
            .map_err(|_| SidecarPortError)
    }

    /// 只能停止 Runtime 当前持有的 opaque identity；未知 identity 一律失败。
    fn stop(&mut self, instance: &ManagedSidecar) -> Result<(), SidecarPortError> {
        {
            let running = self
                .running
                .get_mut(&instance.identity())
                .ok_or(SidecarPortError)?;
            running.ready = false;
            running.child.stop().map_err(|_| SidecarPortError)?;
            running.runtime.cleanup().map_err(|_| SidecarPortError)?;
        }
        // `remove` 让 RunningChild 中不可复制的 secret 在 config 已删除后立刻被 Drop 擦除。
        self.running.remove(&instance.identity());
        Ok(())
    }
}

impl WindowsManagedSidecarPort {
    fn run_pending(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
        if !self.running.is_empty() {
            return Err(SidecarPortError);
        }
        let pending = self.pending.as_ref().ok_or(SidecarPortError)?;
        if !pending.checked || !matches_final_digest(pending.runtime.config_path(), pending.digest)
        {
            return Err(SidecarPortError);
        }
        let identity = self.next_identity.checked_add(1).ok_or(SidecarPortError)?;
        let child =
            ChildProcess::start_checked(&self.executable_path, pending.runtime.config_path())
                .map_err(|_| SidecarPortError)?;
        let pending = self
            .pending
            .take()
            .expect("checked pending exists until spawn succeeds");
        self.next_identity = identity;
        self.running.insert(
            identity,
            RunningChild {
                child,
                runtime: pending.runtime,
                secret: pending.secret,
                observation_bridge: RuntimeObservationBridge::new(),
                ready: false,
            },
        );
        Ok(ManagedSidecar::from_port_identity(identity))
    }
}

impl WindowsManagedSidecarPort {
    /// 仅供拥有当前 opaque child identity 的后端运行时调用。
    ///
    /// snapshot、事件和窗口生命周期都不得调用此方法；child 停止或身份替换后，对应 bridge 会随
    /// `RunningChild` 一并移除，从而丢弃旧的采样状态且拒绝后续网络读取。
    pub(crate) fn sample_runtime_observation(
        &mut self,
        instance: &ManagedSidecar,
    ) -> Result<ClashRuntimeObservation, SidecarPortError> {
        let running = self
            .running
            .get_mut(&instance.identity())
            .ok_or(SidecarPortError)?;
        running.ready.then_some(()).ok_or(SidecarPortError)?;
        running
            .child
            .is_running()
            .map_err(|_| SidecarPortError)?
            .then_some(())
            .ok_or(SidecarPortError)?;
        tauri::async_runtime::block_on(running.observation_bridge.sample(&running.secret))
            .map_err(|_| SidecarPortError)
    }
}

impl RuntimeObservationSidecarPort for WindowsManagedSidecarPort {
    fn read_runtime_observation(
        &mut self,
        instance: &ManagedSidecar,
    ) -> Result<ClashRuntimeObservation, SidecarPortError> {
        self.sample_runtime_observation(instance)
    }
}

// 私有文件在 check 前后必须与编译器最终字节相同；读回副本用后覆盖。
fn matches_final_bytes(path: &Path, candidate: &GeneratedConfig) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let Ok(metadata) = fs::symlink_metadata(parent) else {
        return false;
    };
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return false;
    }

    let Ok(mut bytes) = fs::read(path) else {
        return false;
    };
    let matches = bytes == candidate.as_bytes();
    bytes.fill(0);
    matches
}

fn matches_final_digest(path: &Path, digest: [u8; 32]) -> bool {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let Ok(metadata) = fs::symlink_metadata(parent) else {
        return false;
    };
    if !metadata.is_dir() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return false;
    }

    let Ok(mut bytes) = fs::read(path) else {
        return false;
    };
    let matches = <[u8; 32]>::from(Sha256::digest(&bytes)) == digest;
    bytes.fill(0);
    matches
}

fn verify_fixed_resources(
    resource_root: &Path,
    executable_path: &Path,
    library_path: &Path,
    license_path: &Path,
) -> Result<(), SidecarPortError> {
    let directory = fs::symlink_metadata(resource_root).map_err(|_| SidecarPortError)?;
    if !directory.is_dir() || directory.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
        return Err(SidecarPortError);
    }

    let actual_files = fs::read_dir(resource_root)
        .map_err(|_| SidecarPortError)?
        .map(|entry| {
            let entry = entry.map_err(|_| SidecarPortError)?;
            let metadata = entry
                .path()
                .symlink_metadata()
                .map_err(|_| SidecarPortError)?;
            if !metadata.is_file() || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
            {
                return Err(SidecarPortError);
            }
            entry
                .file_name()
                .into_string()
                .map_err(|_| SidecarPortError)
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    let expected_files = EXPECTED_RESOURCE_FILES
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if actual_files != expected_files {
        return Err(SidecarPortError);
    }

    verify_bundled_resources(executable_path, library_path, license_path)
        .map_err(|_| SidecarPortError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::singbox::test_support::FIXED_CLASH_API_TEST_LOCK;

    // 仅测试构建使用私有父子管道驱动 DCR-009 peer，不加入产品 IPC 或运行配置入口。
    mod wg_peer_test {
        use super::*;
        use crate::{
            domain::*,
            singbox::{ConfigCompiler, RuntimeProfile, SingBoxCompiler, runtime::SidecarRuntime},
            subscription::{normalize_nodes, parse_subscription},
        };
        use serde::Deserialize;
        use std::{
            io::{Read, Write},
            os::windows::process::CommandExt,
            process::{Child, Command, Stdio},
            sync::{
                Arc,
                atomic::{AtomicBool, AtomicUsize, Ordering},
                mpsc,
            },
            thread::{self, JoinHandle},
            time::{Duration, Instant},
        };

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Selftest {
            tcp: bool,
            udp: bool,
            icmp: bool,
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct LocalIcmp {
            sent: u64,
            received: u64,
            id: u16,
            sequences: [u16; 3],
            payloads_valid: bool,
            addresses_valid: bool,
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        #[serde(rename_all = "snake_case")]
        enum LocalProbeError {
            None,
            Refused,
            Reset,
            Eof,
            Timeout,
        }

        #[derive(Debug, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct LocalCase {
            case_id: u8,
            sent: bool,
            equal_echo: bool,
            error: LocalProbeError,
        }

        #[derive(Deserialize)]
        #[serde(tag = "event", deny_unknown_fields)]
        enum Frame {
            #[serde(rename = "ready")]
            Ready {
                v: u8,
                run_id: String,
                udp_port: u16,
                peer_public_key: String,
                dut_public_key: String,
                selftest: Selftest,
            },
            #[serde(rename = "tcp")]
            Tcp {
                v: u8,
                run_id: String,
                requests: u64,
                response_status: u16,
                rx_tcp_packets: u64,
                tx_tcp_packets: u64,
                authenticated: bool,
                response_acked: bool,
            },
            #[serde(rename = "icmp")]
            Icmp {
                v: u8,
                run_id: String,
                sent: u64,
                received: u64,
                id: u16,
                sequences: [u16; 3],
                payloads_valid: bool,
                addresses_valid: bool,
            },
            #[serde(rename = "udp")]
            Udp {
                v: u8,
                run_id: String,
                received: u64,
                replied: u64,
                sequences: [u32; 3],
                rx_udp_packets: u64,
                tx_udp_packets: u64,
                payloads_valid: bool,
                addresses_valid: bool,
                authenticated: bool,
            },
            #[serde(rename = "phase_ready")]
            PhaseReady { v: u8, run_id: String, phase: u8 },
            #[serde(rename = "bootstrap")]
            Bootstrap {
                v: u8,
                run_id: String,
                phase: u8,
                requests: u64,
                response_status: u16,
                rx_tcp_packets: u64,
                tx_tcp_packets: u64,
                authenticated: bool,
                response_acked: bool,
            },
            #[serde(rename = "local_probe")]
            LocalProbe {
                v: u8,
                run_id: String,
                phase: u8,
                cases: [LocalCase; 4],
                icmp: LocalIcmp,
            },
            #[serde(rename = "phase_stopped")]
            PhaseStopped { v: u8, run_id: String, phase: u8 },
            #[serde(rename = "failed")]
            Failed {
                v: u8,
                run_id: String,
                stage: FailureStage,
                code: FailureCode,
            },
            #[serde(rename = "stopped")]
            Stopped {
                v: u8,
                run_id: String,
                resources_closed: bool,
            },
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum FailureStage {
            Init,
            Selftest,
            Bind,
            Tcp,
            Icmp,
            Protocol,
            Deadline,
            Cleanup,
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "snake_case")]
        enum FailureCode {
            InvalidInput,
            IoError,
            Timeout,
            UnexpectedPacket,
            LimitExceeded,
            ResourceError,
        }

        impl Frame {
            fn belongs_to(&self, expected: &str) -> bool {
                let (v, run_id) = match self {
                    Self::Ready { v, run_id, .. }
                    | Self::Tcp { v, run_id, .. }
                    | Self::Icmp { v, run_id, .. }
                    | Self::Udp { v, run_id, .. }
                    | Self::PhaseReady { v, run_id, .. }
                    | Self::Bootstrap { v, run_id, .. }
                    | Self::LocalProbe { v, run_id, .. }
                    | Self::PhaseStopped { v, run_id, .. }
                    | Self::Failed { v, run_id, .. }
                    | Self::Stopped { v, run_id, .. } => (v, run_id),
                };
                *v == 1 && run_id == expected
            }
        }

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        enum PeerStage {
            InitTcp,
            InitUdp,
            InitReject,
            ReadyReject,
            BeginPhase(u8),
            PhaseReady(u8),
            Bootstrap(u8),
            ProbeLocal(u8),
            LocalProbe(u8),
            FinishPhase(u8),
            PhaseStopped(u8),
            ReadyTcp,
            ReadyUdp,
            Tcp,
            ProbeIcmp,
            Icmp,
            Udp,
            Complete,
        }

        impl PeerStage {
            fn after_command(self, op: &str) -> Result<Self, &'static str> {
                match (self, op) {
                    (Self::InitTcp, "init") => Ok(Self::ReadyTcp),
                    (Self::InitUdp, "init_udp") => Ok(Self::ReadyUdp),
                    (Self::InitReject, "init_reject") => Ok(Self::ReadyReject),
                    (Self::BeginPhase(phase), "begin_phase") => Ok(Self::PhaseReady(phase)),
                    (Self::ProbeLocal(phase), "probe_local") => Ok(Self::LocalProbe(phase)),
                    (Self::FinishPhase(phase), "finish_phase") => Ok(Self::PhaseStopped(phase)),
                    (Self::ProbeIcmp, "probe_icmp") => Ok(Self::Icmp),
                    (_, "shutdown") => Ok(self), // 清理也允许取消尚未完成的场景。
                    _ => Err("peer command scenario/order"),
                }
            }

            fn after_frame(self, frame: &Frame) -> Result<Self, &'static str> {
                match (self, frame) {
                    (Self::ReadyTcp, Frame::Ready { .. }) => Ok(Self::Tcp),
                    (Self::ReadyUdp, Frame::Ready { .. }) => Ok(Self::Udp),
                    (Self::ReadyReject, Frame::Ready { .. }) => Ok(Self::BeginPhase(1)),
                    (Self::PhaseReady(expected), Frame::PhaseReady { phase, .. })
                        if expected == *phase =>
                    {
                        Ok(Self::Bootstrap(expected))
                    }
                    (Self::Bootstrap(expected), Frame::Bootstrap { phase, .. })
                        if expected == *phase =>
                    {
                        Ok(Self::ProbeLocal(expected))
                    }
                    (Self::LocalProbe(expected), Frame::LocalProbe { phase, .. })
                        if expected == *phase =>
                    {
                        Ok(Self::FinishPhase(expected))
                    }
                    (Self::PhaseStopped(expected), Frame::PhaseStopped { phase, .. })
                        if expected == *phase =>
                    {
                        Ok(if expected == 3 {
                            Self::Complete
                        } else {
                            Self::BeginPhase(expected + 1)
                        })
                    }
                    (Self::Tcp, Frame::Tcp { .. }) => Ok(Self::ProbeIcmp),
                    (Self::Icmp, Frame::Icmp { .. }) | (Self::Udp, Frame::Udp { .. }) => {
                        Ok(Self::Complete)
                    }
                    (_, Frame::Failed { .. }) => Err("peer reported fixed failure"),
                    _ => Err("peer event scenario/order"),
                }
            }
        }

        enum PipeEvent {
            Line(Vec<u8>),
            Eof,
        }
        type WriteRequest = (Vec<u8>, mpsc::SyncSender<Result<(), &'static str>>);

        struct Peer {
            child: Child,
            input: Option<mpsc::SyncSender<WriteRequest>>,
            output: mpsc::Receiver<PipeEvent>,
            threads: Vec<JoinHandle<()>>,
            pipe_failed: Arc<AtomicBool>,
            stderr_bytes: Arc<AtomicUsize>,
            stdin_closed: Arc<AtomicBool>,
            started: Instant,
            run_id: String,
            stage: PeerStage,
            hard_lifetime: Duration,
            final_lifetime: Duration,
        }

        impl Peer {
            fn spawn(path: &Path, run_id: String, stage: PeerStage) -> Self {
                let started = Instant::now();
                let (hard_lifetime, final_lifetime) = if stage == PeerStage::InitReject {
                    (Duration::from_secs(150), Duration::from_secs(159))
                } else {
                    (Duration::from_secs(55), Duration::from_secs(59))
                };
                let mut child = Command::new(path)
                    .creation_flags(0x0800_0000)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .expect("start fixed test peer");
                let mut stdout = child.stdout.take().expect("peer stdout");
                let mut stderr = child.stderr.take().expect("peer stderr");
                let mut stdin = child.stdin.take().expect("peer stdin");
                let (input, writes) = mpsc::sync_channel::<WriteRequest>(1);
                let (lines, output) = mpsc::sync_channel(8);
                let pipe_failed = Arc::new(AtomicBool::new(false));
                let stdout_failed = Arc::clone(&pipe_failed);
                let stdout_thread = thread::spawn(move || {
                    let mut chunk = [0; 512];
                    let mut line = Vec::with_capacity(4096);
                    let mut total = 0usize;
                    let mut invalid = false;
                    loop {
                        match stdout.read(&mut chunk) {
                            Ok(0) => {
                                if !line.is_empty() {
                                    stdout_failed.store(true, Ordering::SeqCst);
                                }
                                let _ = lines.try_send(PipeEvent::Eof);
                                break;
                            }
                            Ok(count) => {
                                total = total.saturating_add(count);
                                if total > 16_384 {
                                    invalid = true;
                                }
                                for byte in &chunk[..count] {
                                    if invalid {
                                        continue;
                                    }
                                    line.push(*byte);
                                    if line.len() > 4096 {
                                        invalid = true;
                                        continue;
                                    }
                                    if *byte == b'\n' {
                                        if lines
                                            .try_send(PipeEvent::Line(std::mem::take(&mut line)))
                                            .is_err()
                                        {
                                            invalid = true;
                                        }
                                        line = Vec::with_capacity(4096);
                                    }
                                }
                                if invalid {
                                    stdout_failed.store(true, Ordering::SeqCst);
                                    line.clear();
                                }
                            }
                            Err(_) => {
                                stdout_failed.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                });
                let stderr_bytes = Arc::new(AtomicUsize::new(0));
                let stderr_count = Arc::clone(&stderr_bytes);
                let stderr_failed = Arc::clone(&pipe_failed);
                let stderr_thread = thread::spawn(move || {
                    let mut chunk = [0; 512];
                    loop {
                        match stderr.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(count) => {
                                if stderr_count.fetch_add(count, Ordering::SeqCst) + count > 4096 {
                                    stderr_failed.store(true, Ordering::SeqCst);
                                }
                                chunk.fill(0); // 只保留字节数，绝不转储 peer 原始诊断。
                            }
                            Err(_) => {
                                stderr_failed.store(true, Ordering::SeqCst);
                                break;
                            }
                        }
                    }
                });
                let stdin_closed = Arc::new(AtomicBool::new(false));
                let writer_closed = Arc::clone(&stdin_closed);
                let writer_thread = thread::spawn(move || {
                    // 父计时早于 Go 启动；不能在父侧第55秒先于 peer 硬限关闭 stdin。
                    let hard_deadline = started + final_lifetime;
                    let mut total = 0usize;
                    while let Some(remaining) = hard_deadline.checked_duration_since(Instant::now())
                    {
                        let Ok((mut bytes, ack)) = writes.recv_timeout(remaining) else {
                            break;
                        };
                        total += bytes.len();
                        let result = if bytes.len() > 4096 || total > 16_384 {
                            Err("peer input limit")
                        } else {
                            stdin
                                .write_all(&bytes)
                                .and_then(|_| stdin.flush())
                                .map_err(|_| "peer input I/O")
                        };
                        bytes.fill(0);
                        let failed = result.is_err();
                        let _ = ack.try_send(result);
                        if failed {
                            break;
                        }
                    }
                    drop(stdin);
                    writer_closed.store(true, Ordering::SeqCst);
                });
                Self {
                    child,
                    input: Some(input),
                    output,
                    threads: vec![stdout_thread, stderr_thread, writer_thread],
                    pipe_failed,
                    stderr_bytes,
                    stdin_closed,
                    started,
                    run_id,
                    stage,
                    hard_lifetime,
                    final_lifetime,
                }
            }

            fn send(
                &mut self,
                value: serde_json::Value,
                deadline: Instant,
            ) -> Result<(), &'static str> {
                if let PeerStage::BeginPhase(expected) = self.stage
                    && value["op"] == "begin_phase"
                    && value["phase"].as_u64() != Some(u64::from(expected))
                {
                    return Err("peer command phase mismatch");
                }
                let stage = self
                    .stage
                    .after_command(value["op"].as_str().ok_or("peer command missing")?)?;
                let mut bytes = serde_json::to_vec(&value).map_err(|_| "peer input encoding")?;
                bytes.push(b'\n');
                let (ack, received) = mpsc::sync_channel(1);
                self.input
                    .as_ref()
                    .ok_or("peer input closed")?
                    .try_send((bytes, ack))
                    .map_err(|_| "peer input queue")?;
                received
                    .recv_timeout(remaining(deadline, Duration::from_secs(2))?)
                    .map_err(|_| "peer input deadline")??;
                self.stage = stage;
                Ok(())
            }

            fn frame(&mut self, deadline: Instant) -> Result<Frame, &'static str> {
                loop {
                    if self.pipe_failed.load(Ordering::SeqCst) {
                        return Err("peer pipe invalid");
                    }
                    if self
                        .child
                        .try_wait()
                        .map_err(|_| "peer process status")?
                        .is_some()
                    {
                        return Err("peer exited before DUT cleanup");
                    }
                    match self
                        .output
                        .recv_timeout(remaining(deadline, Duration::from_millis(25))?)
                    {
                        Ok(PipeEvent::Line(bytes)) => {
                            let frame: Frame =
                                serde_json::from_slice(&bytes).map_err(|_| "peer frame invalid")?;
                            if !frame.belongs_to(&self.run_id) {
                                return Err("peer frame identity");
                            }
                            self.stage = self.stage.after_frame(&frame)?;
                            return Ok(frame);
                        }
                        Ok(PipeEvent::Eof) => return Err("peer output ended"),
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(_) => return Err("peer output closed"),
                    }
                }
            }

            // 未确认 DUT/pending 停止时保留 peer 至自身硬限；强制清理仅用于已确认分支。
            fn finish(&mut self, dut_stopped: bool) -> Result<(), &'static str> {
                if !dut_stopped {
                    let deadline = self.started + self.final_lifetime;
                    let mut exit = None;
                    let mut invalid = false;
                    while Instant::now() < deadline {
                        // 继续排空固定失败摘要，既不发送 shutdown，也不关闭 stdin 或 kill。
                        match self.output.recv_timeout(Duration::from_millis(10)) {
                            Ok(PipeEvent::Line(bytes)) => {
                                match serde_json::from_slice::<Frame>(&bytes) {
                                    Ok(frame @ Frame::Failed { .. })
                                        if frame.belongs_to(&self.run_id) => {}
                                    _ => invalid = true,
                                }
                            }
                            Ok(PipeEvent::Eof) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                            Err(mpsc::RecvTimeoutError::Disconnected) => {
                                thread::sleep(Duration::from_millis(10));
                            }
                        }
                        match self.child.try_wait() {
                            Ok(Some(status)) => {
                                exit = Some(status);
                                break;
                            }
                            Ok(None) => {}
                            Err(_) => invalid = true,
                        }
                    }
                    let Some(exit) = exit else {
                        // 保留句柄和输入端；截止未确认退出只能失败，不能假称已清理。
                        return Err("DUT stop unconfirmed; peer hard exit unavailable");
                    };
                    let elapsed = self.started.elapsed();
                    self.input.take(); // 仅在已观察到 peer 自行退出之后结束写侧。
                    while self.threads.iter().any(|thread| !thread.is_finished())
                        && Instant::now() < deadline
                    {
                        thread::sleep(Duration::from_millis(10));
                    }
                    if self.threads.iter().any(|thread| !thread.is_finished()) {
                        return Err("DUT stop unconfirmed; peer pipe join deadline");
                    }
                    for thread in self.threads.drain(..) {
                        if thread.join().is_err() {
                            invalid = true;
                        }
                    }
                    eprintln!(
                        "task009 peer_hold elapsed_ms={} exit_code={:?} dut_stop_confirmed=false",
                        elapsed.as_millis(),
                        exit.code()
                    );
                    if invalid
                        || self.pipe_failed.load(Ordering::SeqCst)
                        || exit.success()
                        || elapsed < self.hard_lifetime
                    {
                        return Err("DUT stop unconfirmed; peer ended unexpectedly");
                    }
                    // peer 按硬限退出不代表 DUT 停止或整条清理链成功。
                    return Err("DUT stop unconfirmed; peer hard exit observed");
                }
                let deadline = (self.started + self.final_lifetime)
                    .min(Instant::now() + Duration::from_secs(5));
                let mut normal = self
                    .send(
                        serde_json::json!({
                            "v":1,"op":"shutdown","run_id":self.run_id,"dut_stopped":true
                        }),
                        deadline,
                    )
                    .is_ok();
                let mut stopped = false;
                let mut eof = false;
                let mut exit = None;
                let graceful_deadline = deadline.min(Instant::now() + Duration::from_secs(2));
                while Instant::now() < graceful_deadline {
                    match self.output.recv_timeout(Duration::from_millis(10)) {
                        Ok(PipeEvent::Line(bytes)) => {
                            match serde_json::from_slice::<Frame>(&bytes) {
                                Ok(
                                    frame @ Frame::Stopped {
                                        resources_closed: true,
                                        ..
                                    },
                                ) if frame.belongs_to(&self.run_id) && !stopped => stopped = true,
                                _ => normal = false,
                            }
                        }
                        Ok(PipeEvent::Eof) => eof = true,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                        Err(_) => eof = true,
                    }
                    exit = self.child.try_wait().map_err(|_| "peer cleanup status")?;
                    if exit.is_some() && eof {
                        break;
                    }
                }
                if exit.is_none() {
                    normal = false;
                    self.child.kill().map_err(|_| "peer owned kill")?;
                    while Instant::now() < deadline {
                        exit = self
                            .child
                            .try_wait()
                            .map_err(|_| "peer forced exit status")?;
                        if exit.is_some() {
                            break;
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                }
                self.input.take();
                while self.threads.iter().any(|thread| !thread.is_finished())
                    && Instant::now() < deadline
                {
                    thread::sleep(Duration::from_millis(10));
                }
                if self.threads.iter().any(|thread| !thread.is_finished()) {
                    return Err("peer pipe join deadline");
                }
                for thread in self.threads.drain(..) {
                    thread.join().map_err(|_| "peer pipe thread")?;
                }
                if !normal
                    || !stopped
                    || !exit.is_some_and(|status| status.success())
                    || self.pipe_failed.load(Ordering::SeqCst)
                {
                    return Err("peer cleanup incomplete");
                }
                Ok(())
            }
        }

        fn remaining(deadline: Instant, limit: Duration) -> Result<Duration, &'static str> {
            deadline
                .checked_duration_since(Instant::now())
                .filter(|time| !time.is_zero())
                .map(|time| time.min(limit))
                .ok_or("peer absolute deadline")
        }

        fn key_base64(bytes: &[u8; 32]) -> String {
            const ALPHABET: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
            let mut encoded = String::with_capacity(44);
            for chunk in bytes.chunks(3) {
                let value = chunk
                    .iter()
                    .fold(0u32, |value, byte| (value << 8) | u32::from(*byte))
                    << (8 * (3 - chunk.len()));
                encoded.push(ALPHABET[((value >> 18) & 63) as usize] as char);
                encoded.push(ALPHABET[((value >> 12) & 63) as usize] as char);
                encoded.push(ALPHABET[((value >> 6) & 63) as usize] as char);
                encoded.push(if chunk.len() == 3 {
                    ALPHABET[(value & 63) as usize] as char
                } else {
                    '='
                });
            }
            encoded
        }

        fn valid_public_key(key: &str) -> bool {
            let bytes = key.as_bytes();
            if bytes.len() != 44 || bytes[43] != b'=' {
                return false;
            }
            let value = |byte: u8| match byte {
                b'A'..=b'Z' => Some(byte - b'A'),
                b'a'..=b'z' => Some(byte - b'a' + 26),
                b'0'..=b'9' => Some(byte - b'0' + 52),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            };
            bytes[..43].iter().all(|byte| value(*byte).is_some())
                && value(bytes[42]).is_some_and(|last| last & 3 == 0)
        }

        #[derive(Debug, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        struct SocketInventory {
            tcp: Vec<String>,
            udp: Vec<String>,
        }

        fn owned_sockets(pid: u32, root: &Path, deadline: Instant) -> SocketInventory {
            let query_started = Instant::now();
            assert!(
                query_started < deadline,
                "socket query classification=case_deadline"
            );
            // PowerShell/CIM 冷启动不是业务网络 I/O；仍受调用方绝对截止约束，不重试。
            let query_deadline = query_started + Duration::from_secs(5);
            let until = deadline.min(query_deadline);
            let powershell = PathBuf::from(std::env::var_os("SystemRoot").expect("Windows root"))
                .join("System32/WindowsPowerShell/v1.0/powershell.exe");
            let output_path = root.join(format!("owned-sockets-{pid}.json"));
            // CIM 在源端只查询持有句柄对应 PID；不保存其它进程的端点。
            let script = format!(
                "$ErrorActionPreference='Stop'; $t=@(Get-CimInstance -Namespace root/StandardCimv2 -ClassName MSFT_NetTCPConnection -Filter 'OwningProcess = {pid} AND State = 2' | ForEach-Object {{'['+$_.LocalAddress+']:'+$_.LocalPort}}); $u=@(Get-CimInstance -Namespace root/StandardCimv2 -ClassName MSFT_NetUDPEndpoint -Filter 'OwningProcess = {pid}' | ForEach-Object {{'['+$_.LocalAddress+']:'+$_.LocalPort}}); @{{tcp=$t;udp=$u}} | ConvertTo-Json -Compress"
            );
            let mut child = Command::new(powershell)
                .args(["-NoProfile", "-NonInteractive", "-Command", &script])
                .creation_flags(0x0800_0000)
                .stdin(Stdio::null())
                .stdout(Stdio::from(
                    fs::File::create(&output_path).expect("owned socket evidence"),
                ))
                .stderr(Stdio::null())
                .spawn()
                .expect("owned socket query");
            let mut status = None;
            while Instant::now() < until {
                status = child.try_wait().expect("socket query status");
                if status.is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            if status.is_none() {
                let classification = if deadline <= query_deadline {
                    "case_deadline"
                } else {
                    "query_deadline"
                };
                let mut exited = child.try_wait().expect("query timeout exit status");
                if exited.is_none() {
                    child.kill().expect("stop owned socket query");
                }
                let exit_deadline = Instant::now() + Duration::from_secs(2);
                while exited.is_none() && Instant::now() < exit_deadline {
                    exited = child.try_wait().expect("query exit status");
                    if exited.is_some() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                eprintln!(
                    "task009 socket_query classification={classification} elapsed_ms={} exit_confirmed={} exit_code={:?}",
                    query_started.elapsed().as_millis(),
                    exited.is_some(),
                    exited.and_then(|status| status.code())
                );
                assert!(
                    exited.is_some(),
                    "socket query classification=cleanup_deadline"
                );
                panic!("owned socket query classification={classification}");
            }
            let status = status.expect("socket query completed");
            assert!(
                status.success(),
                "owned socket query classification=exit_failure exit_code={:?}",
                status.code()
            );
            assert!(
                fs::metadata(&output_path)
                    .expect("socket evidence size")
                    .len()
                    <= 16_384
            );
            serde_json::from_slice(&fs::read(output_path).expect("owned socket evidence"))
                .expect("socket evidence format")
        }

        fn verify_sockets(peer: &SocketInventory, dut: &SocketInventory, peer_port: u16) {
            assert!(peer.tcp.is_empty());
            assert_eq!(peer.udp, vec![format!("[127.0.0.1]:{peer_port}")]);
            assert_eq!(dut.tcp, vec!["[127.0.0.1]:9090"]);
            assert!(!dut.udp.is_empty() && dut.udp.len() <= 2);
            let mut ports = BTreeSet::new();
            for endpoint in &dut.udp {
                let (address, port) = endpoint.rsplit_once(':').expect("UDP endpoint");
                assert!(matches!(
                    address,
                    "[0.0.0.0]" | "[127.0.0.1]" | "[::]" | "[::1]"
                ));
                let port: u16 = port.parse().expect("UDP port");
                assert!(port != 0 && port != 9090 && port != peer_port);
                ports.insert(port);
            }
            assert_eq!(
                ports.len(),
                1,
                "one WG UDP binding, possibly shared dual stack"
            );
        }

        fn verify_udp_sockets(
            peer: &SocketInventory,
            dut: &SocketInventory,
            peer_port: u16,
            inlet: u16,
        ) {
            let business = format!("[127.0.0.1]:{inlet}");
            assert_eq!(
                dut.udp
                    .iter()
                    .filter(|endpoint| **endpoint == business)
                    .count(),
                1,
                "exactly one owned loopback UDP business inlet"
            );
            let protocol = SocketInventory {
                tcp: dut.tcp.clone(),
                udp: dut
                    .udp
                    .iter()
                    .filter(|endpoint| **endpoint != business)
                    .cloned()
                    .collect(),
            };
            // 只减去精确入口；任何额外端点仍须通过原 WG 单端口组规则。
            verify_sockets(peer, &protocol, peer_port);
        }

        #[derive(Clone, Default, Debug)]
        struct LocalTargetCounts {
            tcp_accepts: [u32; 3],
            udp_receives: [u32; 3],
            payloads: [[u32; 4]; 3],
            active: u32,
            failed: bool,
        }

        // phase2即使迟到至phase3也永远失败；同一格只允许一个精确载荷。
        fn accept_local_payload(
            counts: &mut LocalTargetCounts,
            bytes: &[u8],
            token: &[u8; 16],
            phase: usize,
            tcp: bool,
        ) -> bool {
            let valid = bytes.len() == 20
                && bytes[..16] == *token
                && matches!(phase, 1 | 3)
                && usize::from(bytes[16]) == phase
                && bytes[18..] == [0, 0]
                && if tcp {
                    matches!(bytes[17], 1 | 2)
                } else {
                    matches!(bytes[17], 3 | 4)
                };
            if !valid {
                counts.failed = true;
                return false;
            }
            let count = &mut counts.payloads[phase - 1][usize::from(bytes[17]) - 1];
            if *count != 0 {
                counts.failed = true;
                return false;
            }
            *count = 1;
            true
        }

        struct LocalTargets {
            tcp: std::net::TcpListener,
            udp: std::net::UdpSocket,
            counts: Arc<std::sync::Mutex<LocalTargetCounts>>,
            phase: Arc<AtomicUsize>,
            cancel: Arc<AtomicBool>,
            threads: Vec<JoinHandle<()>>,
        }

        impl LocalTargets {
            fn bind() -> Self {
                let tcp =
                    std::net::TcpListener::bind(("127.0.0.1", 0)).expect("owned local TCP target");
                let udp =
                    std::net::UdpSocket::bind(("127.0.0.1", 0)).expect("owned local UDP target");
                let tcp_port = tcp.local_addr().expect("TCP target address").port();
                let udp_port = udp.local_addr().expect("UDP target address").port();
                assert!(tcp_port != 9090 && udp_port != 9090 && tcp_port != udp_port);
                tcp.set_nonblocking(true).expect("bounded TCP accept");
                udp.set_read_timeout(Some(Duration::from_millis(25)))
                    .expect("bounded UDP target receive");
                Self {
                    tcp,
                    udp,
                    counts: Arc::new(std::sync::Mutex::new(LocalTargetCounts::default())),
                    phase: Arc::new(AtomicUsize::new(0)),
                    cancel: Arc::new(AtomicBool::new(false)),
                    threads: Vec::new(),
                }
            }

            fn start(&mut self, token: [u8; 16], deadline: Instant) {
                let listener = self.tcp.try_clone().expect("retain TCP target owner");
                listener
                    .set_nonblocking(true)
                    .expect("cloned TCP target nonblocking accept");
                let counts = Arc::clone(&self.counts);
                let phase = Arc::clone(&self.phase);
                let cancel = Arc::clone(&self.cancel);
                self.threads.push(thread::spawn(move || {
                    while Instant::now() < deadline {
                        let (mut stream, source) = match listener.accept() {
                            Ok(connection) => connection,
                            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                                if cancel.load(Ordering::SeqCst) {
                                    break;
                                }
                                thread::sleep(Duration::from_millis(10));
                                continue;
                            }
                            Err(_) => {
                                counts.lock().expect("target counts").failed = true;
                                break;
                            }
                        };
                        let current = phase.load(Ordering::SeqCst);
                        {
                            let mut counts = counts.lock().expect("target counts");
                            counts.active += 1;
                            if (1..=3).contains(&current) {
                                counts.tcp_accepts[current - 1] += 1;
                            } else {
                                counts.failed = true;
                            }
                            if current == 2 || counts.tcp_accepts.iter().sum::<u32>() > 4 {
                                counts.failed = true;
                            }
                        }
                        let served = (|| -> Result<(), &'static str> {
                            if !source.ip().is_loopback() {
                                return Err("unexpected TCP target source");
                            }
                            stream
                                .set_nonblocking(false)
                                .map_err(|_| "target TCP mode")?;
                            let mut payload = [0; 20];
                            metering_block_io(&mut stream, &mut payload, false, deadline)?;
                            let accepted = accept_local_payload(
                                &mut counts.lock().expect("target counts"),
                                &payload,
                                &token,
                                current,
                                true,
                            );
                            if !accepted {
                                return Err("unexpected TCP target payload");
                            }
                            metering_block_io(&mut stream, &mut payload, true, deadline)?;
                            payload.fill(0);
                            stream
                                .set_read_timeout(Some(remaining(
                                    deadline,
                                    Duration::from_secs(2),
                                )?))
                                .map_err(|_| "target TCP final timeout")?;
                            let mut extra = [0; 1];
                            if stream
                                .read(&mut extra)
                                .map_err(|_| "target TCP final read")?
                                != 0
                            {
                                return Err("extra TCP target bytes");
                            }
                            Ok(())
                        })();
                        let mut counts = counts.lock().expect("target counts");
                        counts.active -= 1;
                        counts.failed |= served.is_err();
                    }
                }));
                let socket = self.udp.try_clone().expect("retain UDP target owner");
                socket
                    .set_read_timeout(Some(Duration::from_millis(25)))
                    .expect("cloned UDP target receive deadline");
                let counts = Arc::clone(&self.counts);
                let phase = Arc::clone(&self.phase);
                let cancel = Arc::clone(&self.cancel);
                self.threads.push(thread::spawn(move || {
                    let mut bytes = [0; 21];
                    while Instant::now() < deadline {
                        let (count, source) = match socket.recv_from(&mut bytes) {
                            Ok(datagram) => datagram,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                                ) =>
                            {
                                if cancel.load(Ordering::SeqCst) {
                                    break;
                                }
                                continue;
                            }
                            Err(_) => {
                                counts.lock().expect("target counts").failed = true;
                                break;
                            }
                        };
                        let current = phase.load(Ordering::SeqCst);
                        let accepted = {
                            let mut counts = counts.lock().expect("target counts");
                            counts.active += 1;
                            if (1..=3).contains(&current) {
                                counts.udp_receives[current - 1] += 1;
                            } else {
                                counts.failed = true;
                            }
                            let accepted = accept_local_payload(
                                &mut counts,
                                &bytes[..count],
                                &token,
                                current,
                                false,
                            );
                            if !source.ip().is_loopback() {
                                counts.failed = true;
                            }
                            accepted && source.ip().is_loopback()
                        };
                        let served = accepted
                            && remaining(deadline, Duration::from_secs(2))
                                .and_then(|timeout| {
                                    socket
                                        .set_write_timeout(Some(timeout))
                                        .map_err(|_| "target UDP timeout")
                                })
                                .and_then(|()| {
                                    socket
                                        .send_to(&bytes[..count], source)
                                        .map_err(|_| "target UDP reply")
                                })
                                == Ok(20);
                        bytes.fill(0);
                        let mut counts = counts.lock().expect("target counts");
                        counts.active -= 1;
                        counts.failed |= !served;
                    }
                }));
            }

            fn snapshot(&self) -> LocalTargetCounts {
                self.counts.lock().expect("target counts").clone()
            }

            fn settled(&self, phase: usize, deadline: Instant) -> LocalTargetCounts {
                loop {
                    let snapshot = self.snapshot();
                    assert!(
                        !snapshot.failed,
                        "local target rejected unexpected or late traffic"
                    );
                    if snapshot.active == 0 {
                        let rounds = if phase == 3 { [2, 0, 2] } else { [2, 0, 0] };
                        assert_eq!(snapshot.tcp_accepts, rounds);
                        assert_eq!(snapshot.udp_receives, rounds);
                        assert_eq!(snapshot.payloads[0], [1; 4]);
                        assert_eq!(snapshot.payloads[1], [0; 4]);
                        assert_eq!(
                            snapshot.payloads[2],
                            if phase == 3 { [1; 4] } else { [0; 4] }
                        );
                        return snapshot;
                    }
                    assert!(
                        Instant::now() < deadline,
                        "target handlers settle within phase deadline"
                    );
                    thread::sleep(Duration::from_millis(10));
                }
            }

            fn stop_handlers(&mut self, deadline: Instant) {
                self.cancel.store(true, Ordering::SeqCst);
                while self.threads.iter().any(|worker| !worker.is_finished())
                    && Instant::now() < deadline
                {
                    thread::sleep(Duration::from_millis(10));
                }
                assert!(
                    self.threads.iter().all(|worker| worker.is_finished()),
                    "bounded target handler exit tcp_finished={} udp_finished={}",
                    self.threads
                        .first()
                        .is_none_or(|worker| worker.is_finished()),
                    self.threads
                        .get(1)
                        .is_none_or(|worker| worker.is_finished())
                );
                for worker in self.threads.drain(..) {
                    worker.join().expect("target handler join");
                }
                // 原始listener仍由self持有；只有所有DUT已停止（或硬限失败）后才drop self。
            }
        }

        #[test]
        fn local_target_rejects_late_protected_payloads_and_duplicate_positive_cells() {
            let token = [7; 16];
            let mut payload = [0; 20];
            payload[..16].copy_from_slice(&token);
            payload[16] = 2;
            payload[17] = 1;
            let mut counts = LocalTargetCounts::default();
            assert!(!accept_local_payload(
                &mut counts,
                &payload,
                &token,
                3,
                true
            ));
            assert!(counts.failed && counts.payloads == [[0; 4]; 3]);
            let mut counts = LocalTargetCounts::default();
            payload[16] = 1;
            assert!(accept_local_payload(&mut counts, &payload, &token, 1, true));
            assert!(!accept_local_payload(
                &mut counts,
                &payload,
                &token,
                1,
                true
            ));
            assert!(counts.failed);
            let mut counts = LocalTargetCounts::default();
            payload[17] = 3;
            assert!(!accept_local_payload(
                &mut counts,
                &payload,
                &token,
                1,
                true
            ));
            assert!(counts.failed);
        }

        #[test]
        fn local_targets_count_empty_tcp_and_late_udp_without_echo() {
            let token = [11; 16];
            let mut targets = LocalTargets::bind();
            targets.phase.store(3, Ordering::SeqCst);
            let deadline = Instant::now() + Duration::from_secs(3);
            targets.start(token, deadline);
            let tcp = std::net::TcpStream::connect_timeout(
                &targets.tcp.local_addr().unwrap(),
                Duration::from_secs(1),
            )
            .expect("owned empty TCP connection");
            drop(tcp);
            let udp = std::net::UdpSocket::bind(("127.0.0.1", 0)).expect("owned UDP client");
            udp.connect(targets.udp.local_addr().unwrap())
                .expect("owned target");
            let mut payload = [0; 20];
            payload[..16].copy_from_slice(&token);
            payload[16] = 2;
            payload[17] = 3;
            udp.set_write_timeout(Some(Duration::from_secs(1))).unwrap();
            assert_eq!(udp.send(&payload).expect("late protected payload"), 20);
            while Instant::now() < deadline {
                let counts = targets.snapshot();
                if counts.tcp_accepts[2] == 1 && counts.udp_receives[2] == 1 && counts.active == 0 {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            targets.stop_handlers(deadline + Duration::from_secs(1));
            let counts = targets.snapshot();
            assert_eq!(counts.tcp_accepts, [0, 0, 1]);
            assert_eq!(counts.udp_receives, [0, 0, 1]);
            assert_eq!(counts.payloads, [[0; 4]; 3]);
            assert!(counts.failed);
            udp.set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            assert!(
                matches!(udp.recv(&mut payload),Err(error) if matches!(error.kind(),std::io::ErrorKind::TimedOut|std::io::ErrorKind::WouldBlock)),
                "invalid payload must not be echoed"
            );
        }

        #[test]
        fn local_reject_protocol_rejects_cross_phase_and_missing_bootstrap() {
            let phase_ready = Frame::PhaseReady {
                v: 1,
                run_id: "fixture".into(),
                phase: 2,
            };
            assert!(PeerStage::PhaseReady(1).after_frame(&phase_ready).is_err());
            assert!(
                PeerStage::Bootstrap(1)
                    .after_command("probe_local")
                    .is_err()
            );
            assert!(PeerStage::ReadyTcp.after_command("begin_phase").is_err());
            assert!(
                PeerStage::BeginPhase(1)
                    .after_command("probe_icmp")
                    .is_err()
            );
            let bootstrap = Frame::Bootstrap {
                v: 1,
                run_id: "fixture".into(),
                phase: 1,
                requests: 1,
                response_status: 204,
                rx_tcp_packets: 4,
                tx_tcp_packets: 4,
                authenticated: true,
                response_acked: true,
            };
            assert_eq!(
                PeerStage::Bootstrap(1).after_frame(&bootstrap),
                Ok(PeerStage::ProbeLocal(1))
            );
            assert!(PeerStage::Bootstrap(2).after_frame(&bootstrap).is_err());
            assert!(PeerStage::ProbeLocal(1).after_frame(&bootstrap).is_err());
            let stopped = Frame::PhaseStopped {
                v: 1,
                run_id: "fixture".into(),
                phase: 3,
            };
            assert_eq!(
                PeerStage::PhaseStopped(3).after_frame(&stopped),
                Ok(PeerStage::Complete)
            );
            assert!(PeerStage::PhaseStopped(2).after_frame(&stopped).is_err());
            assert!(
                PeerStage::LocalProbe(3)
                    .after_command("finish_phase")
                    .is_err()
            );
            let mut summary = serde_json::json!({"v":1,"event":"local_probe","run_id":"fixture","phase":2,
                "cases":[{"case_id":1,"sent":true,"equal_echo":false,"error":"timeout"}],
                "icmp":{"sent":3,"received":3,"id":9,"sequences":[1,2,3],"payloads_valid":true,"addresses_valid":true}});
            assert!(
                serde_json::from_value::<Frame>(summary.clone()).is_err(),
                "all four cases are required"
            );
            summary["cases"]=serde_json::json!((1..=4).map(|case_id|serde_json::json!({"case_id":case_id,"sent":true,"equal_echo":false,"error":"decode_error"})).collect::<Vec<_>>());
            assert!(
                serde_json::from_value::<Frame>(summary).is_err(),
                "decode failure is never negative-path evidence"
            );
        }

        #[test]
        fn wg_udp_frames_reject_cross_scenario_events_and_incomplete_evidence() {
            let udp_value = serde_json::json!({"v":1,"event":"udp","run_id":"fixture",
                "received":3,"replied":3,"sequences":[1,2,3],"rx_udp_packets":3,"tx_udp_packets":3,
                "payloads_valid":true,"addresses_valid":true,"authenticated":true});
            let udp: Frame = serde_json::from_value(udp_value.clone()).expect("UDP frame");
            let tcp: Frame =
                serde_json::from_value(serde_json::json!({"v":1,"event":"tcp","run_id":"fixture",
                "requests":1,"response_status":204,"rx_tcp_packets":4,"tx_tcp_packets":4,
                "authenticated":true,"response_acked":true}))
                .expect("TCP frame");
            let icmp: Frame = serde_json::from_value(serde_json::json!({"v":1,"event":"icmp","run_id":"fixture",
                "sent":3,"received":3,"id":9,"sequences":[1,2,3],"payloads_valid":true,"addresses_valid":true})).expect("ICMP frame");
            assert!(PeerStage::Tcp.after_frame(&udp).is_err());
            assert!(PeerStage::Icmp.after_frame(&udp).is_err());
            assert!(PeerStage::Udp.after_frame(&tcp).is_err());
            assert!(PeerStage::Udp.after_frame(&icmp).is_err());
            assert_eq!(PeerStage::Udp.after_frame(&udp), Ok(PeerStage::Complete));
            assert!(PeerStage::Complete.after_frame(&udp).is_err());
            assert!(PeerStage::ReadyUdp.after_frame(&udp).is_err());
            assert!(PeerStage::InitTcp.after_command("init_udp").is_err());
            assert!(PeerStage::InitUdp.after_command("init").is_err());
            let initialized = PeerStage::InitUdp
                .after_command("init_udp")
                .expect("UDP init");
            assert!(initialized.after_command("init_udp").is_err());
            assert!(PeerStage::Udp.after_command("probe_icmp").is_err());
            assert!(PeerStage::Udp.after_command("arbitrary").is_err());
            for field in [
                "authenticated",
                "tx_udp_packets",
                "addresses_valid",
                "sequences",
            ] {
                let mut missing = udp_value.clone();
                missing.as_object_mut().unwrap().remove(field);
                assert!(serde_json::from_value::<Frame>(missing).is_err());
            }
            for (field, value) in [
                ("extra", serde_json::json!(1)),
                ("received", serde_json::json!(-1)),
                ("tx_udp_packets", serde_json::json!(3.5)),
                ("authenticated", serde_json::json!("true")),
                ("sequences", serde_json::json!([1, 2, 3, 4])),
            ] {
                let mut invalid = udp_value.clone();
                invalid[field] = value;
                assert!(serde_json::from_value::<Frame>(invalid).is_err());
            }
        }

        #[test]
        fn wg_peer_frames_reject_unknown_fields_versions_and_noncanonical_keys() {
            let stopped =
                br#"{"v":1,"event":"stopped","run_id":"fixture","resources_closed":true}"#;
            assert!(
                serde_json::from_slice::<Frame>(stopped)
                    .unwrap()
                    .belongs_to("fixture")
            );
            for invalid in [
                r#"{"v":1,"event":"stopped","run_id":"fixture","resources_closed":true,"extra":1}"#,
                r#"{"v":1,"event":"stopped","run_id":"fixture","resources_closed":true} {}"#,
                r#"{"v":1,"event":"stopped","run_id":"fixture","resources_closed":true,"v":1}"#,
                r#"{"v":1,"event":"tcp","run_id":"fixture","requests":1}"#,
            ] {
                assert!(serde_json::from_str::<Frame>(invalid).is_err());
            }
            let other = serde_json::from_str::<Frame>(
                r#"{"v":2,"event":"stopped","run_id":"fixture","resources_closed":true}"#,
            )
            .unwrap();
            assert!(!other.belongs_to("fixture"));
            assert!(
                !serde_json::from_slice::<Frame>(stopped)
                    .unwrap()
                    .belongs_to("other")
            );
            assert_eq!(
                key_base64(&[1; 32]),
                "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE="
            );
            assert!(valid_public_key(&key_base64(&[255; 32])));
            assert!(!valid_public_key(
                "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQF="
            ));
        }

        #[test]
        fn real_peer_preserves_binding_until_hard_exit_when_dut_stop_is_unconfirmed() {
            verify_real_unconfirmed_peer_hold(false);
        }

        #[test]
        fn real_reject_peer_and_targets_hold_until_150_second_hard_exit() {
            verify_real_unconfirmed_peer_hold(true);
        }

        fn verify_real_unconfirmed_peer_hold(local_reject: bool) {
            use std::net::UdpSocket;

            let _lock = FIXED_CLASH_API_TEST_LOCK.lock().expect("fixed API lock");
            let helper_path =
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/task009-wg-peer.exe");
            assert!(
                helper_path.is_file(),
                "build the approved fixed WG test peer before this test"
            );
            let helper_hash = format!(
                "{:x}",
                Sha256::digest(fs::read(&helper_path).expect("fixed helper bytes"))
            );
            let mut run_bytes = [0; 16];
            let mut token_bytes = [0; 16];
            let mut dut_private = [0; 32];
            let mut peer_private = [0; 32];
            getrandom::fill(&mut run_bytes).expect("run entropy");
            getrandom::fill(&mut token_bytes).expect("token entropy");
            getrandom::fill(&mut dut_private).expect("DUT key entropy");
            getrandom::fill(&mut peer_private).expect("peer key entropy");
            for key in [&mut dut_private, &mut peer_private] {
                key[0] &= 248;
                key[31] &= 127;
                key[31] |= 64;
            }
            let hex = |bytes: &[u8]| {
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            let run_id = hex(&run_bytes);
            let mut targets = local_reject.then(LocalTargets::bind);
            let target_ports = targets.as_ref().map(|targets| {
                (
                    targets.tcp.local_addr().expect("held TCP target").port(),
                    targets.udp.local_addr().expect("held UDP target").port(),
                )
            });
            let mut peer = Peer::spawn(
                &helper_path,
                run_id.clone(),
                if local_reject {
                    PeerStage::InitReject
                } else {
                    PeerStage::InitTcp
                },
            );
            let peer_pid = peer.child.id();
            let initialized = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut initial = serde_json::json!({"v":1,"op":"init","run_id":run_id,
                    "dut_private_key":dut_private,"peer_private_key":peer_private,"token":hex(&token_bytes)});
                if let Some(targets) = &mut targets {
                    targets.start(token_bytes, peer.started + Duration::from_secs(135));
                    let (tcp_port, udp_port) = target_ports.expect("held ports");
                    initial["op"] = serde_json::json!("init_reject");
                    initial["tcp_port"] = serde_json::json!(tcp_port);
                    initial["udp_port"] = serde_json::json!(udp_port);
                }
                peer.send(initial, peer.started + Duration::from_secs(10))
                    .expect("private init write");
                let Frame::Ready {
                    udp_port, selftest, ..
                } = peer
                    .frame(peer.started + Duration::from_secs(10))
                    .expect("real peer Ready")
                else {
                    panic!("expected initialized peer")
                };
                assert!(selftest.tcp && selftest.udp && selftest.icmp);
                assert!(udp_port != 0 && udp_port != 9090);
                if let Some((tcp, udp)) = target_ports {
                    assert!(udp_port != tcp && udp_port != udp);
                }
                udp_port
            }));
            dut_private.fill(0);
            peer_private.fill(0);
            token_bytes.fill(0);
            let udp_port = match initialized {
                Ok(port) => port,
                Err(panic) => {
                    // 此测试未创建 DUT；初始化失败可按“DUT从未启动”正常终止自己的 helper。
                    let cleanup = peer.finish(true);
                    if let Some(targets) = &mut targets {
                        targets.stop_handlers(peer.started + peer.final_lifetime);
                    }
                    eprintln!(
                        "task009 hold_fixture initialization_failed=true helper_cleanup_ok={}",
                        cleanup.is_ok()
                    );
                    std::panic::resume_unwind(panic);
                }
            };
            let started = peer.started;
            let hard_lifetime = peer.hard_lifetime;
            let final_lifetime = peer.final_lifetime;
            assert_eq!(
                hard_lifetime,
                Duration::from_secs(if local_reject { 150 } else { 55 })
            );
            assert_eq!(
                final_lifetime,
                Duration::from_secs(if local_reject { 159 } else { 59 })
            );
            let hold_started = Instant::now();
            let stdin_closed = Arc::clone(&peer.stdin_closed);
            // 只模拟停止确认缺失；真实运行该场景原定55/150秒硬限，不启动 DUT。
            let worker = thread::spawn(move || {
                let result = peer.finish(false);
                (peer, result)
            });
            let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for check_at in [
                    hold_started + Duration::from_secs(3),
                    started + hard_lifetime - Duration::from_secs(2),
                ] {
                    thread::sleep(check_at.saturating_duration_since(Instant::now()));
                    assert!(
                        !worker.is_finished(),
                        "unconfirmed branch must still hold the real peer"
                    );
                    assert!(
                        !stdin_closed.load(Ordering::SeqCst),
                        "stdin remains open before the peer hard limit"
                    );
                    match UdpSocket::bind(("127.0.0.1", udp_port)) {
                        Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::AddrInUse),
                        Ok(_) => panic!("peer UDP binding was released before its hard limit"),
                    }
                    if let Some((tcp, udp)) = target_ports {
                        assert!(
                            matches!(std::net::TcpListener::bind(("127.0.0.1",tcp)),Err(error) if error.kind()==std::io::ErrorKind::AddrInUse),
                            "target TCP owner retained until peer hard limit"
                        );
                        assert!(
                            matches!(UdpSocket::bind(("127.0.0.1",udp)),Err(error) if error.kind()==std::io::ErrorKind::AddrInUse),
                            "target UDP owner retained until peer hard limit"
                        );
                    }
                    println!(
                        "task009 peer_hold peer_pid={peer_pid} elapsed_ms={} udp_binding_retained=true stdin_open=true local_target_ports={target_ports:?} simulated_dut_stop_failure=true",
                        started.elapsed().as_millis()
                    );
                }
            }));
            // 早期断言失败也等待同一真实硬限；测试不通过 kill 缩短被验证的生命周期。
            let deadline = started + final_lifetime;
            while !worker.is_finished() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(10));
            }
            assert!(
                worker.is_finished(),
                "unconfirmed peer finish remains bounded by the parent deadline"
            );
            let (mut peer, result) = worker.join().expect("bounded hold worker");
            if let Some(targets) = &mut targets {
                // 即使异常peer提前退出，模拟未确认DUT时也不提前释放目标原句柄。
                while Instant::now() < started + hard_lifetime {
                    thread::sleep(
                        (started + hard_lifetime)
                            .saturating_duration_since(Instant::now())
                            .min(Duration::from_millis(100)),
                    );
                }
                targets.stop_handlers(deadline);
                assert!(!targets.snapshot().failed && targets.snapshot().active == 0);
            }
            assert_eq!(result, Err("DUT stop unconfirmed; peer hard exit observed"));
            assert!(started.elapsed() >= hard_lifetime);
            let exit = peer
                .child
                .try_wait()
                .expect("peer hard exit status")
                .expect("peer exited");
            assert_eq!(exit.code(), Some(1), "hard exit remains a failure");
            assert!(peer.input.is_none() && peer.threads.is_empty());
            assert!(stdin_closed.load(Ordering::SeqCst));
            let reclaimed = UdpSocket::bind(("127.0.0.1", udp_port))
                .expect("owned peer UDP port released after hard exit");
            assert_eq!(
                reclaimed.local_addr().expect("reclaimed address").port(),
                udp_port
            );
            drop(reclaimed);
            drop(targets);
            if let Some((tcp, udp)) = target_ports {
                drop(
                    std::net::TcpListener::bind(("127.0.0.1", tcp))
                        .expect("TCP target released after hard exit"),
                );
                drop(
                    UdpSocket::bind(("127.0.0.1", udp))
                        .expect("UDP target released after hard exit"),
                );
            }
            println!(
                "task009 peer_hold helper_sha256={helper_hash} elapsed_ms={} hard_exit_code=1 udp_binding_released=true cleanup_result=FAIL simulated_dut_stop_failure=true",
                started.elapsed().as_millis()
            );
            if let Err(panic) = assertions {
                std::panic::resume_unwind(panic);
            }
        }

        #[test]
        fn real_wireguard_local_reject_has_positive_before_and_after_four_blocked_paths() {
            use std::num::NonZeroU16;
            let _lock = FIXED_CLASH_API_TEST_LOCK.lock().expect("fixed API lock");
            let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
            let helper_path = target.join("task009-wg-peer.exe");
            assert!(
                helper_path.is_file(),
                "build the approved fixed WG peer before this test"
            );
            let helper_hash = format!(
                "{:x}",
                Sha256::digest(fs::read(&helper_path).expect("helper bytes"))
            );
            let name = format!(
                "task009-wg-local-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            );
            let root = target.join(&name);
            let resources = root.join("resources");
            let bundled = resources.join(RESOURCE_DIRECTORY);
            fs::create_dir_all(&bundled).expect("owned local comparison resources");
            let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("binaries/sing-box-1.14.0-windows-amd64");
            for file in EXPECTED_RESOURCE_FILES {
                fs::hard_link(cache.join(file), bundled.join(file))
                    .expect("fixed resource hardlink");
            }
            let data = std::env::temp_dir().join(&name);
            fs::create_dir(&data).expect("owned comparison app data");
            let mut runtime = SidecarRuntime::new_observation_only(
                WindowsManagedSidecarPort::new(resources, data.clone())
                    .expect("fixed assets and ACL"),
            );
            let mut targets = LocalTargets::bind();
            let tcp_port = targets.tcp.local_addr().expect("TCP target").port();
            let udp_port = targets.udp.local_addr().expect("UDP target").port();
            let mut run_bytes = [0; 16];
            let mut token_bytes = [0; 16];
            let mut dut_private = [0; 32];
            let mut peer_private = [0; 32];
            getrandom::fill(&mut run_bytes).expect("run entropy");
            getrandom::fill(&mut token_bytes).expect("token entropy");
            getrandom::fill(&mut dut_private).expect("DUT key entropy");
            getrandom::fill(&mut peer_private).expect("peer key entropy");
            for key in [&mut dut_private, &mut peer_private] {
                key[0] &= 248;
                key[31] &= 127;
                key[31] |= 64;
            }
            let hex = |bytes: &[u8]| {
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            let run_id = hex(&run_bytes);
            let token = hex(&token_bytes);
            let mut peer = Peer::spawn(&helper_path, run_id.clone(), PeerStage::InitReject);
            let peer_pid = peer.child.id();
            let work_deadline = peer.started + Duration::from_secs(135);
            let mut stop_unconfirmed = false;
            let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                targets.start(token_bytes, work_deadline);
                peer.send(serde_json::json!({"v":1,"op":"init_reject","run_id":run_id,"token":token,
                    "dut_private_key":dut_private,"peer_private_key":peer_private,"tcp_port":tcp_port,"udp_port":udp_port}),
                    peer.started + Duration::from_secs(10)).expect("private reject init");
                let Frame::Ready {
                    udp_port: peer_port,
                    peer_public_key,
                    dut_public_key,
                    selftest,
                    ..
                } = peer
                    .frame(peer.started + Duration::from_secs(10))
                    .expect("local comparison peer Ready")
                else {
                    panic!("expected comparison Ready")
                };
                assert!(selftest.tcp && selftest.udp && selftest.icmp);
                assert!(valid_public_key(&peer_public_key) && valid_public_key(&dut_public_key));
                assert!(peer_public_key != dut_public_key);
                assert!(
                    peer_port != 0
                        && peer_port != 9090
                        && peer_port != tcp_port
                        && peer_port != udp_port
                );
                let peer_initial = owned_sockets(peer_pid, &root, work_deadline);
                assert!(peer_initial.tcp.is_empty());
                assert_eq!(peer_initial.udp, vec![format!("[127.0.0.1]:{peer_port}")]);
                let mut hashes = BTreeSet::new();
                let mut identities = BTreeSet::new();
                let mut previous_secret: Option<String> = None;
                for phase in 1u8..=3 {
                    let phase_started = Instant::now();
                    let phase_deadline =
                        work_deadline.min(Instant::now() + Duration::from_secs(40));
                    let phase_work = phase_deadline - Duration::from_secs(6);
                    targets.phase.store(usize::from(phase), Ordering::SeqCst);
                    peer.send(
                        serde_json::json!({"v":1,"op":"begin_phase","run_id":run_id,"phase":phase}),
                        phase_work,
                    )
                    .expect("begin next phase");
                    let Frame::PhaseReady {
                        phase: confirmed, ..
                    } = peer.frame(phase_work).expect("phase Ready")
                    else {
                        panic!("expected phase Ready")
                    };
                    assert_eq!(confirmed, phase);
                    let parsed = parse_subscription(&serde_json::json!({"outbounds":[{
                        "type":"wireguard","tag":"local-comparison","server":"127.0.0.1","server_port":peer_port,
                        "private_key":key_base64(&dut_private),"peer_public_key":peer_public_key,
                        "local_address":["198.18.0.1/32"],"mtu":1280
                    }]}).to_string()).expect("normal WG parsing");
                    assert!(parsed.skipped.is_empty());
                    assert_eq!(parsed.nodes.len(), 1);
                    let mut state = AppState::empty();
                    state.subscriptions.push(Subscription {
                        id: SubscriptionId("local-reject".into()),
                        name: "local-reject".into(),
                    });
                    state.providers.push(Provider {
                        id: ProviderId("local-reject".into()),
                        subscription_id: SubscriptionId("local-reject".into()),
                        name: "local-reject".into(),
                    });
                    state.nodes = normalize_nodes(ProviderId("local-reject".into()), parsed.nodes)
                        .expect("normal WG normalization");
                    state.pools.push(NodePool {
                        id: PoolId("local-reject".into()),
                        name: "local-reject".into(),
                        kind: PoolKind::Custom,
                        sources: vec![PoolSource {
                            provider_id: ProviderId("local-reject".into()),
                            filter: NodeFilter::default(),
                        }],
                        selection: SelectionPolicy::UrlTest {
                            probe_url: format!(
                                "http://198.18.0.2:18080/task009-wg?token={token}&phase={phase}"
                            ),
                            interval_secs: 300,
                            tolerance_ms: 50,
                        },
                        enabled: true,
                    });
                    state.default_target = RouteTarget::Pool(PoolId("local-reject".into()));
                    state.routes.push(RoutePolicy {
                        id: RoutePolicyId("local-ports".into()),
                        name: "local-ports".into(),
                        enabled: true,
                        priority: 0,
                        matcher: TrafficMatcher::Port(vec![tcp_port, udp_port]),
                        target: RouteTarget::Direct,
                    });
                    let intent = RuntimeIntent::from_state(&state).expect("whole phase state");
                    let plan = if phase == 2 {
                        SingBoxCompiler.compile(
                            &intent,
                            &state.default_target,
                            DnsPolicy::System,
                            RuntimeProfile::ObservationOnly,
                        )
                    } else {
                        SingBoxCompiler.compile_wireguard_local_positive(
                            &intent,
                            &state.default_target,
                            DnsPolicy::System,
                            NonZeroU16::new(tcp_port).unwrap(),
                            NonZeroU16::new(udp_port).unwrap(),
                        )
                    }
                    .expect("phase typed configuration");
                    let secret = crate::singbox::managed_sidecar::generate_api_secret()
                        .expect("new phase API entropy");
                    if let Some(previous) = &previous_secret {
                        assert!(previous != secret.as_str());
                    }
                    previous_secret = Some(secret.as_str().to_owned());
                    let config = plan.finalize(&secret).expect("phase finalized bytes");
                    let config_hash = format!("{:x}", Sha256::digest(config.as_bytes()));
                    assert!(hashes.insert(config_hash.clone()));
                    let document: serde_json::Value =
                        serde_json::from_slice(config.as_bytes()).expect("phase final readback");
                    assert_eq!(document["inbounds"], serde_json::json!([]));
                    assert_eq!(document["dns"]["rules"][0]["action"], "reject");
                    assert_eq!(
                        document["route"]["rules"][if phase == 2 { 0 } else { 2 }]["action"],
                        "reject"
                    );
                    assert!(
                        phase_work.saturating_duration_since(Instant::now())
                            >= Duration::from_secs(13),
                        "reserve core startup limits"
                    );
                    runtime
                        .start_or_replace(config)
                        .expect("phase fixed core check/run/Ready");
                    let (identity, pid, private_instance) = runtime
                        .with_active_port(|port, child| {
                            let running = port
                                .running
                                .get(&child.identity())
                                .expect("owned phase child");
                            assert_eq!(
                                format!(
                                    "{:x}",
                                    Sha256::digest(
                                        fs::read(running.runtime.config_path())
                                            .expect("phase private readback")
                                    )
                                ),
                                config_hash
                            );
                            Ok((
                                child.identity(),
                                running.child.test_process_id(),
                                running
                                    .runtime
                                    .config_path()
                                    .parent()
                                    .expect("private instance")
                                    .to_path_buf(),
                            ))
                        })
                        .expect("phase owner")
                        .expect("active phase");
                    assert!(identities.insert(identity));
                    let dut_ready = owned_sockets(pid, &root, phase_work);
                    verify_sockets(&peer_initial, &dut_ready, peer_port);
                    let Frame::Bootstrap {
                        phase: confirmed,
                        requests: 1,
                        response_status: 204,
                        rx_tcp_packets,
                        tx_tcp_packets,
                        authenticated: true,
                        response_acked: true,
                        ..
                    } = peer.frame(phase_work).expect("fresh phase bootstrap")
                    else {
                        panic!("expected fresh authenticated phase bootstrap")
                    };
                    assert_eq!(confirmed, phase);
                    assert!(rx_tcp_packets > 0 && tx_tcp_packets > 0);
                    peer.send(
                        serde_json::json!({"v":1,"op":"probe_local","run_id":run_id}),
                        phase_work,
                    )
                    .expect("fixed four local probes");
                    let Frame::LocalProbe {
                        phase: confirmed,
                        cases,
                        icmp,
                        ..
                    } = peer.frame(phase_work).expect("phase local probe summary")
                    else {
                        panic!("expected local probe summary")
                    };
                    println!(
                        "task009 local_reject phase={phase} reported_phase={confirmed} elapsed_ms={} helper_sha256={helper_hash} config_sha256={config_hash} identity={identity} dut_pid={pid} peer_pid={peer_pid} tcp_target={tcp_port} udp_target={udp_port} peer_udp_port={peer_port} bootstrap_acked=true cases={cases:?} target_counts={:?} dut_ready_sockets={dut_ready:?}",
                        phase_started.elapsed().as_millis(),
                        targets.snapshot()
                    );
                    assert_eq!(confirmed, phase);
                    for (index, case) in cases.iter().enumerate() {
                        assert_eq!(usize::from(case.case_id), index + 1);
                        assert!(case.sent, "each probe must be submitted through WG");
                        if phase == 2 {
                            assert!(!case.equal_echo);
                            assert_ne!(case.error, LocalProbeError::None);
                        } else {
                            assert!(case.equal_echo);
                            assert_eq!(case.error, LocalProbeError::None);
                        }
                    }
                    assert_eq!(
                        (icmp.sent, icmp.received, icmp.id, icmp.sequences),
                        (3, 3, 9, [1, 2, 3])
                    );
                    assert!(icmp.payloads_valid && icmp.addresses_valid);
                    runtime
                        .with_active_port(|port, child| {
                            let running = port
                                .running
                                .get_mut(&child.identity())
                                .expect("same phase owner");
                            assert_eq!(running.child.test_process_id(), pid);
                            assert!(running.child.is_running().map_err(|_| SidecarPortError)?);
                            let client = ClashApiClient::new(&running.secret)
                                .map_err(|_| SidecarPortError)?;
                            tauri::async_runtime::block_on(client.read_connections())
                                .map_err(|_| SidecarPortError)?;
                            Ok(())
                        })
                        .expect("same DUT authenticated API alive")
                        .expect("active health instance");
                    let counts = targets.settled(usize::from(phase), phase_work);
                    let peer_current = owned_sockets(peer_pid, &root, phase_work);
                    let dut_current = owned_sockets(pid, &root, phase_work);
                    verify_sockets(&peer_current, &dut_current, peer_port);
                    assert_eq!(dut_current, dut_ready);
                    assert!(
                        peer.child
                            .try_wait()
                            .expect("peer alive across phases")
                            .is_none()
                    );
                    println!(
                        "task009 local_reject phase={phase} icmp=3/3 target_counts={counts:?} dut_sockets={dut_current:?}"
                    );
                    assert!(
                        Instant::now() < phase_work,
                        "phase leaves room for Stop and handshake cleanup"
                    );
                    if runtime.stop().is_err() {
                        stop_unconfirmed = true;
                        panic!("phase DUT Stop unconfirmed");
                    }
                    assert!(!private_instance.exists());
                    peer.send(serde_json::json!({"v":1,"op":"finish_phase","run_id":run_id,"dut_stopped":true}),phase_deadline).expect("confirmed phase finish");
                    let Frame::PhaseStopped {
                        phase: confirmed, ..
                    } = peer.frame(phase_deadline).expect("peer phase stopped")
                    else {
                        panic!("expected peer phase stopped")
                    };
                    assert_eq!(confirmed, phase);
                    assert!(Instant::now() < phase_deadline);
                    println!(
                        "task009 local_reject phase={phase} elapsed_ms={} dut_stop_confirmed=true peer_phase_stopped=true private_instance_removed=true",
                        phase_started.elapsed().as_millis()
                    );
                    targets.settled(usize::from(phase), phase_deadline);
                }
            }));
            // 不能因断言或服务失败提前释放目标；首先只停止当前自有DUT。
            let dut_stopped = !stop_unconfirmed && runtime.stop().is_ok();
            let peer_cleanup = peer.finish(dut_stopped);
            if !dut_stopped {
                let hard_limit = peer.started + peer.hard_lifetime;
                while Instant::now() < hard_limit {
                    thread::sleep(
                        hard_limit
                            .saturating_duration_since(Instant::now())
                            .min(Duration::from_millis(100)),
                    );
                }
            }
            dut_private.fill(0);
            peer_private.fill(0);
            token_bytes.fill(0);
            let cleanup_assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                targets.stop_handlers(peer.started + peer.final_lifetime);
                let peer_status = peer.child.try_wait();
                let peer_exited = matches!(&peer_status, Ok(Some(_)));
                let peer_exit_code = peer_status
                    .as_ref()
                    .ok()
                    .and_then(|status| status.as_ref().and_then(std::process::ExitStatus::code));
                let port = runtime.into_port();
                let dut_owned_empty = port.running.is_empty() && !port.has_pending_cleanup();
                let private_count =
                    fs::read_dir(data.join(RUNTIME_DIRECTORY)).map(|entries| entries.count());
                println!(
                    "task009 local_reject cleanup dut_stop_confirmed={dut_stopped} dut_owned_empty={dut_owned_empty} peer_pid={peer_pid} peer_exited={peer_exited} peer_exit_code={peer_exit_code:?} peer_cleanup={peer_cleanup:?} private_count={:?} target_counts={:?} original_assertion_failed={}",
                    private_count.as_ref().ok(),
                    targets.snapshot(),
                    assertions.is_err()
                );
                assert!(
                    dut_stopped,
                    "retain failed DUT resources; target and peer Hold ended at hard limit"
                );
                assert!(
                    peer_exited && dut_owned_empty,
                    "owned processes cleanup confirmed"
                );
                assert_eq!(private_count.expect("private runtime root"), 0);
                peer_cleanup.expect("comparison peer clean exit");
            }));
            // 清理失败仍使测试失败，但不能覆盖先前四格断言的原始失败。
            if let Err(panic) = assertions {
                std::panic::resume_unwind(panic);
            }
            if let Err(panic) = cleanup_assertions {
                std::panic::resume_unwind(panic);
            }
            let final_counts = targets.snapshot();
            assert!(!final_counts.failed && final_counts.active == 0);
            assert_eq!(final_counts.tcp_accepts, [2, 0, 2]);
            assert_eq!(final_counts.udp_receives, [2, 0, 2]);
            assert_eq!(final_counts.payloads, [[1; 4], [0; 4], [1; 4]]);
            println!(
                "task009 local_reject final_tcp=4 final_udp=4 protected_lifetime_arrivals=0 cleanup=dut_then_peer_then_targets confirmed=true"
            );
            drop(targets);
            for (owned, parent) in [(data, std::env::temp_dir()), (root, target)] {
                let resolved = owned
                    .canonicalize()
                    .expect("resolved owned comparison fixture");
                let parent = parent.canonicalize().expect("resolved fixture parent");
                assert_eq!(resolved.parent(), Some(parent.as_path()));
                assert_eq!(
                    resolved.file_name().expect("owned name"),
                    std::ffi::OsStr::new(&name)
                );
                fs::remove_dir_all(resolved).expect("remove successful owned comparison fixture");
            }
        }

        #[test]
        fn real_wireguard_udp_returns_three_exact_client_datagrams() {
            use std::{net::UdpSocket, num::NonZeroU16};

            let _lock = FIXED_CLASH_API_TEST_LOCK.lock().expect("fixed API lock");
            let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
            let helper_path = target.join("task009-wg-peer.exe");
            assert!(
                helper_path.is_file(),
                "build the approved fixed WG test peer before this test"
            );
            let helper_hash = format!(
                "{:x}",
                Sha256::digest(fs::read(&helper_path).expect("fixed helper bytes"))
            );
            let name = format!(
                "task009-wg-udp-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            );
            let root = target.join(&name);
            let resources = root.join("resources");
            let bundled = resources.join(RESOURCE_DIRECTORY);
            fs::create_dir_all(&bundled).expect("owned UDP test resources");
            let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("binaries/sing-box-1.14.0-windows-amd64");
            for file in EXPECTED_RESOURCE_FILES {
                fs::hard_link(cache.join(file), bundled.join(file))
                    .expect("fixed resource hardlink");
            }
            let data = std::env::temp_dir().join(&name);
            fs::create_dir(&data).expect("owned UDP test app data");
            let port = WindowsManagedSidecarPort::new(resources, data.clone())
                .expect("fixed assets and ACL");
            let mut runtime = SidecarRuntime::new_observation_only(port);
            let mut run_bytes = [0; 16];
            let mut token_bytes = [0; 16];
            let mut dut_private = [0; 32];
            let mut peer_private = [0; 32];
            getrandom::fill(&mut run_bytes).expect("run entropy");
            getrandom::fill(&mut token_bytes).expect("token entropy");
            getrandom::fill(&mut dut_private).expect("DUT key entropy");
            getrandom::fill(&mut peer_private).expect("peer key entropy");
            for key in [&mut dut_private, &mut peer_private] {
                key[0] &= 248;
                key[31] &= 127;
                key[31] |= 64;
            }
            let hex = |bytes: &[u8]| {
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            let run_id = hex(&run_bytes);
            let mut peer = Peer::spawn(&helper_path, run_id.clone(), PeerStage::InitUdp);
            let peer_pid = peer.child.id();
            let mut dut_pid = None;
            let mut client = None;
            let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                peer.send(serde_json::json!({"v":1,"op":"init_udp","run_id":run_id,
                    "dut_private_key":dut_private,"peer_private_key":peer_private,"token":hex(&token_bytes)}),
                    peer.started + Duration::from_secs(10)).expect("private UDP init write");
                let Frame::Ready {
                    udp_port,
                    peer_public_key,
                    dut_public_key,
                    selftest,
                    ..
                } = peer
                    .frame(peer.started + Duration::from_secs(10))
                    .expect("UDP peer Ready/selftest")
                else {
                    panic!("expected UDP peer Ready")
                };
                let work_deadline = (peer.started + Duration::from_secs(45))
                    .min(Instant::now() + Duration::from_secs(30));
                assert!(udp_port != 0 && udp_port != 9090);
                assert!(selftest.tcp && selftest.udp && selftest.icmp);
                assert!(valid_public_key(&peer_public_key) && valid_public_key(&dut_public_key));
                assert!(peer_public_key != dut_public_key);
                let peer_before = owned_sockets(peer_pid, &root, work_deadline);
                assert!(peer_before.tcp.is_empty());
                assert_eq!(peer_before.udp, vec![format!("[127.0.0.1]:{udp_port}")]);
                let reserved = UdpSocket::bind(("127.0.0.1", 0)).expect("reserve owned UDP inlet");
                let inlet = reserved.local_addr().expect("reserved inlet address");
                assert!(inlet.port() != 9090 && inlet.port() != udp_port);
                let parsed = parse_subscription(&serde_json::json!({"outbounds":[{
                    "type":"wireguard","tag":"controlled-wg-udp","server":"127.0.0.1","server_port":udp_port,
                    "private_key":key_base64(&dut_private),"peer_public_key":peer_public_key,
                    "local_address":["198.18.0.1/32"],"mtu":1280
                }]}).to_string()).expect("ordinary WG parser");
                assert!(parsed.skipped.is_empty());
                assert_eq!(parsed.nodes.len(), 1);
                let mut state = AppState::empty();
                state.subscriptions.push(Subscription {
                    id: SubscriptionId("wg-udp-test".into()),
                    name: "wg-udp-test".into(),
                });
                state.providers.push(Provider {
                    id: ProviderId("wg-udp-test".into()),
                    subscription_id: SubscriptionId("wg-udp-test".into()),
                    name: "wg-udp-test".into(),
                });
                state.nodes = normalize_nodes(ProviderId("wg-udp-test".into()), parsed.nodes)
                    .expect("ordinary WG normalization");
                state.pools.push(NodePool {
                    id: PoolId("wg-udp-test".into()),
                    name: "wg-udp-test".into(),
                    kind: PoolKind::Custom,
                    sources: vec![PoolSource {
                        provider_id: ProviderId("wg-udp-test".into()),
                        filter: NodeFilter::default(),
                    }],
                    selection: SelectionPolicy::Manual {
                        selected_node_id: Some(state.nodes[0].id.clone()),
                    },
                    enabled: true,
                });
                state.default_target = RouteTarget::Pool(PoolId("wg-udp-test".into()));
                assert!(state.routes.is_empty());
                let intent = RuntimeIntent::from_state(&state).expect("whole UDP state validation");
                let config = SingBoxCompiler
                    .compile_wireguard_udp(
                        &intent,
                        &state.default_target,
                        DnsPolicy::System,
                        NonZeroU16::new(inlet.port()).expect("nonzero inlet"),
                    )
                    .expect("typed fixed WG UDP plan")
                    .finalize(
                        &crate::singbox::managed_sidecar::generate_api_secret()
                            .expect("API entropy"),
                    )
                    .expect("final UDP configuration");
                let config_hash = format!("{:x}", Sha256::digest(config.as_bytes()));
                let document: serde_json::Value =
                    serde_json::from_slice(config.as_bytes()).expect("final UDP readback");
                assert_eq!(
                    document["inbounds"],
                    serde_json::json!([{
                        "type":"direct","tag":"test-wg-udp","listen":"127.0.0.1","listen_port":inlet.port(),
                        "network":"udp","override_address":"198.18.0.2","override_port":18081
                    }])
                );
                assert_eq!(document["route"]["rules"][0]["action"], "reject");
                assert_eq!(document["dns"]["rules"][0]["action"], "reject");
                assert!(
                    work_deadline.saturating_duration_since(Instant::now())
                        >= Duration::from_secs(13),
                    "reserve check and Ready deadlines before final cleanup"
                );
                // 只在已生成固定配置、即将交给 DUT 时释放预留；端口争用不重试。
                drop(reserved);
                runtime
                    .start_or_replace(config)
                    .expect("fixed UDP check/run/Ready");
                let (identity, pid) = runtime
                    .with_active_port(|port, child| {
                        let running = port
                            .running
                            .get(&child.identity())
                            .expect("owned WG UDP child");
                        assert_eq!(
                            format!(
                                "{:x}",
                                Sha256::digest(
                                    fs::read(running.runtime.config_path())
                                        .expect("private UDP readback")
                                )
                            ),
                            config_hash
                        );
                        Ok((child.identity(), running.child.test_process_id()))
                    })
                    .expect("active UDP owner")
                    .expect("active UDP child");
                dut_pid = Some(pid);
                assert!(peer.child.try_wait().expect("peer remains owned").is_none());
                let dut_before = owned_sockets(pid, &root, work_deadline);
                let peer_ready = owned_sockets(peer_pid, &root, work_deadline);
                verify_udp_sockets(&peer_ready, &dut_before, udp_port, inlet.port());
                client =
                    Some(UdpSocket::bind(("127.0.0.1", 0)).expect("one owned loopback UDP client"));
                let socket = client.as_ref().expect("owned client");
                socket.connect(inlet).expect("connect to fixed test inlet");
                let client_address = socket.local_addr().expect("client identity");
                for sequence in 1u32..=3 {
                    let mut request = [0; 20];
                    request[..16].copy_from_slice(&token_bytes);
                    request[16..].copy_from_slice(&sequence.to_be_bytes());
                    socket
                        .set_write_timeout(Some(
                            remaining(work_deadline, Duration::from_secs(2))
                                .expect("UDP send deadline"),
                        ))
                        .expect("UDP write timeout");
                    assert_eq!(
                        socket.send(&request).expect("one UDP request, no retry"),
                        20
                    );
                    socket
                        .set_read_timeout(Some(
                            remaining(work_deadline, Duration::from_secs(2))
                                .expect("UDP receive deadline"),
                        ))
                        .expect("UDP read timeout");
                    let mut response = [0; 21];
                    let received = socket
                        .recv(&mut response)
                        .expect("UDP reply from same connected inlet");
                    assert_eq!(received, 20, "one complete 20-byte response");
                    assert!(
                        response[..20] == request,
                        "exact token and sequence without exposing payload"
                    );
                    assert_eq!(
                        socket.local_addr().expect("same client identity"),
                        client_address
                    );
                    request.fill(0);
                    response.fill(0);
                }
                let Frame::Udp {
                    received: 3,
                    replied: 3,
                    sequences: [1, 2, 3],
                    rx_udp_packets: 3,
                    tx_udp_packets: 3,
                    payloads_valid: true,
                    addresses_valid: true,
                    authenticated: true,
                    ..
                } = peer
                    .frame(work_deadline)
                    .expect("authenticated UDP boundary summary")
                else {
                    panic!("expected exact three-packet UDP boundary evidence")
                };
                let dut_after = owned_sockets(pid, &root, work_deadline);
                let peer_after = owned_sockets(peer_pid, &root, work_deadline);
                verify_udp_sockets(&peer_after, &dut_after, udp_port, inlet.port());
                assert_eq!(
                    dut_before.udp.iter().collect::<BTreeSet<_>>(),
                    dut_after.udp.iter().collect::<BTreeSet<_>>(),
                    "no extra UDP listener or WG protocol port after traffic"
                );
                assert!(
                    peer.child
                        .try_wait()
                        .expect("peer alive before DUT Stop")
                        .is_none()
                );
                println!(
                    "task009 wg_udp helper_sha256={helper_hash} config_sha256={config_hash} identity={identity} dut_pid={pid} peer_pid={peer_pid} client_received=3 request_bytes=60 response_bytes=60 rx_udp_packets=3 tx_udp_packets=3 authenticated=true dut_sockets={dut_after:?} peer_sockets={peer_after:?}"
                );
            }));
            // 包括 panic：先关闭本机客户端，再停止 DUT，最后交由既有 peer 清理契约。
            client.take();
            let dut_stopped = runtime.stop().is_ok();
            let peer_cleanup = peer.finish(dut_stopped);
            dut_private.fill(0);
            peer_private.fill(0);
            token_bytes.fill(0);
            assert!(
                dut_stopped,
                "DUT cleanup failed; retain private resources and fixture"
            );
            peer_cleanup.expect("UDP peer stopped/exit/pipes all confirmed");
            let cleanup_deadline = peer.started + Duration::from_secs(59);
            let peer_after = owned_sockets(peer_pid, &root, cleanup_deadline);
            assert!(peer_after.tcp.is_empty() && peer_after.udp.is_empty());
            if let Some(pid) = dut_pid {
                let dut_after = owned_sockets(pid, &root, cleanup_deadline);
                assert!(dut_after.tcp.is_empty() && dut_after.udp.is_empty());
            }
            let port = runtime.into_port();
            assert!(port.running.is_empty() && !port.has_pending_cleanup());
            assert_eq!(
                fs::read_dir(data.join(RUNTIME_DIRECTORY))
                    .expect("private UDP runtime root")
                    .count(),
                0
            );
            println!(
                "task009 wg_udp cleanup=client_then_dut_then_peer confirmed=true stderr_bytes={}",
                peer.stderr_bytes.load(Ordering::SeqCst)
            );
            if let Err(panic) = assertions {
                std::panic::resume_unwind(panic);
            }
            for (owned_path, parent) in [(data, std::env::temp_dir()), (root, target)] {
                let resolved = owned_path
                    .canonicalize()
                    .expect("resolved owned UDP fixture");
                let parent = parent.canonicalize().expect("resolved parent");
                assert_eq!(resolved.parent(), Some(parent.as_path()));
                assert_eq!(
                    resolved.file_name().expect("owned UDP name"),
                    std::ffi::OsStr::new(&name)
                );
                fs::remove_dir_all(resolved).expect("remove only successful owned UDP fixture");
            }
        }

        #[test]
        fn real_wireguard_peer_delivers_tcp_response_and_virtual_icmp() {
            let _lock = FIXED_CLASH_API_TEST_LOCK.lock().expect("fixed API lock");
            let target = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
            let helper_path = target.join("task009-wg-peer.exe");
            assert!(
                helper_path.is_file(),
                "build the approved fixed WG test peer before this test"
            );
            let helper_hash = format!(
                "{:x}",
                Sha256::digest(fs::read(&helper_path).expect("fixed helper bytes"))
            );
            let name = format!(
                "task009-wg-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            );
            let root = target.join(&name);
            let resources = root.join("resources");
            let bundled = resources.join(RESOURCE_DIRECTORY);
            fs::create_dir_all(&bundled).expect("owned resources");
            let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("binaries/sing-box-1.14.0-windows-amd64");
            for file in EXPECTED_RESOURCE_FILES {
                fs::hard_link(cache.join(file), bundled.join(file))
                    .expect("fixed resource hardlink");
            }
            let data = std::env::temp_dir().join(&name);
            fs::create_dir(&data).expect("owned app data");
            let port = WindowsManagedSidecarPort::new(resources, data.clone())
                .expect("fixed assets and ACL");
            let mut runtime = SidecarRuntime::new_observation_only(port);
            let mut run_bytes = [0; 16];
            let mut token_bytes = [0; 16];
            let mut dut_private = [0; 32];
            let mut peer_private = [0; 32];
            getrandom::fill(&mut run_bytes).expect("run entropy");
            getrandom::fill(&mut token_bytes).expect("token entropy");
            getrandom::fill(&mut dut_private).expect("DUT key entropy");
            getrandom::fill(&mut peer_private).expect("peer key entropy");
            for key in [&mut dut_private, &mut peer_private] {
                key[0] &= 248;
                key[31] &= 127;
                key[31] |= 64;
            }
            let hex = |bytes: &[u8]| {
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            let run_id = hex(&run_bytes);
            let token = hex(&token_bytes);
            let mut peer = Peer::spawn(&helper_path, run_id.clone(), PeerStage::InitTcp);
            let peer_pid = peer.child.id();
            let work_deadline = peer.started + Duration::from_secs(45);
            let mut dut_pid = None;
            let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                peer.send(
                    serde_json::json!({"v":1,"op":"init","run_id":run_id,
                    "dut_private_key":dut_private,"peer_private_key":peer_private,"token":token}),
                    work_deadline,
                )
                .expect("private init write");
                let Frame::Ready {
                    udp_port,
                    peer_public_key,
                    dut_public_key,
                    selftest,
                    ..
                } = peer
                    .frame(peer.started + Duration::from_secs(10))
                    .expect("peer Ready/selftest")
                else {
                    panic!("expected peer Ready")
                };
                assert!(udp_port != 0 && udp_port != 9090);
                assert!(selftest.tcp && selftest.udp && selftest.icmp);
                assert!(valid_public_key(&peer_public_key) && valid_public_key(&dut_public_key));
                assert!(peer_public_key != dut_public_key);
                let peer_before = owned_sockets(peer_pid, &root, work_deadline);
                assert!(peer_before.tcp.is_empty());
                assert_eq!(peer_before.udp, vec![format!("[127.0.0.1]:{udp_port}")]);
                let parsed = parse_subscription(&serde_json::json!({"outbounds":[{
                    "type":"wireguard","tag":"controlled-wg","server":"127.0.0.1","server_port":udp_port,
                    "private_key":key_base64(&dut_private),"peer_public_key":peer_public_key,
                    "local_address":["198.18.0.1/32"],"mtu":1280
                }]}).to_string()).expect("ordinary WG parser");
                assert!(parsed.skipped.is_empty());
                assert_eq!(parsed.nodes.len(), 1);
                let mut state = AppState::empty();
                state.subscriptions.push(Subscription {
                    id: SubscriptionId("wg-test".into()),
                    name: "wg-test".into(),
                });
                state.providers.push(Provider {
                    id: ProviderId("wg-test".into()),
                    subscription_id: SubscriptionId("wg-test".into()),
                    name: "wg-test".into(),
                });
                state.nodes = normalize_nodes(ProviderId("wg-test".into()), parsed.nodes)
                    .expect("ordinary WG normalization");
                state.pools.push(NodePool {
                    id: PoolId("wg-test".into()),
                    name: "wg-test".into(),
                    kind: PoolKind::Custom,
                    sources: vec![PoolSource {
                        provider_id: ProviderId("wg-test".into()),
                        filter: NodeFilter::default(),
                    }],
                    selection: SelectionPolicy::UrlTest {
                        probe_url: format!("http://198.18.0.2:18080/task009-wg?token={token}"),
                        interval_secs: 300,
                        tolerance_ms: 50,
                    },
                    enabled: true,
                });
                state.default_target = RouteTarget::Pool(PoolId("wg-test".into()));
                let intent = RuntimeIntent::from_state(&state).expect("whole state validation");
                let config = SingBoxCompiler
                    .compile(
                        &intent,
                        &state.default_target,
                        DnsPolicy::System,
                        RuntimeProfile::ObservationOnly,
                    )
                    .expect("ordinary typed WG plan")
                    .finalize(
                        &crate::singbox::managed_sidecar::generate_api_secret()
                            .expect("API entropy"),
                    )
                    .expect("final WG configuration");
                let config_hash = format!("{:x}", Sha256::digest(config.as_bytes()));
                let document: serde_json::Value =
                    serde_json::from_slice(config.as_bytes()).expect("final readback");
                assert_eq!(document["inbounds"], serde_json::json!([]));
                assert_eq!(
                    document["endpoints"].as_array().expect("WG endpoint").len(),
                    1
                );
                assert_eq!(document["route"]["rules"][0]["action"], "reject");
                assert_eq!(document["dns"]["rules"][0]["action"], "reject");
                assert!(
                    work_deadline.saturating_duration_since(Instant::now())
                        >= Duration::from_secs(13),
                    "reserve check and Ready deadlines before final cleanup"
                );
                runtime
                    .start_or_replace(config)
                    .expect("fixed WG check/run/Ready");
                let (identity, pid) = runtime
                    .with_active_port(|port, child| {
                        let running = port.running.get(&child.identity()).expect("owned WG child");
                        assert_eq!(
                            format!(
                                "{:x}",
                                Sha256::digest(
                                    fs::read(running.runtime.config_path())
                                        .expect("private readback")
                                )
                            ),
                            config_hash
                        );
                        Ok((child.identity(), running.child.test_process_id()))
                    })
                    .expect("active owner")
                    .expect("active child");
                dut_pid = Some(pid);
                assert!(peer.child.try_wait().expect("peer remains owned").is_none());
                let dut_sockets = owned_sockets(pid, &root, work_deadline);
                let peer_sockets = owned_sockets(peer_pid, &root, work_deadline);
                verify_sockets(&peer_sockets, &dut_sockets, udp_port);
                let Frame::Tcp {
                    requests: 1,
                    response_status: 204,
                    rx_tcp_packets,
                    tx_tcp_packets,
                    authenticated: true,
                    response_acked: true,
                    ..
                } = peer
                    .frame(work_deadline)
                    .expect("WG TCP response confirmation")
                else {
                    panic!("expected complete authenticated TCP response ACK")
                };
                assert!(rx_tcp_packets > 0 && tx_tcp_packets > 0);
                peer.send(
                    serde_json::json!({"v":1,"op":"probe_icmp","run_id":run_id}),
                    work_deadline,
                )
                .expect("ICMP request");
                let Frame::Icmp {
                    sent: 3,
                    received: 3,
                    id: 9,
                    sequences: [1, 2, 3],
                    payloads_valid: true,
                    addresses_valid: true,
                    ..
                } = peer.frame(work_deadline).expect("three WG Echo replies")
                else {
                    panic!("expected exact authenticated virtual ICMP replies")
                };
                let final_dut = owned_sockets(pid, &root, work_deadline);
                let final_peer = owned_sockets(peer_pid, &root, work_deadline);
                verify_sockets(&final_peer, &final_dut, udp_port);
                assert_eq!(
                    final_dut, dut_sockets,
                    "WG socket ownership remains unchanged"
                );
                assert!(
                    peer.child
                        .try_wait()
                        .expect("peer alive before DUT Stop")
                        .is_none()
                );
                println!(
                    "task009 wg helper_sha256={helper_hash} config_sha256={config_hash} identity={identity} dut_pid={pid} peer_pid={peer_pid} tcp_response_acked=true rx_tcp_packets={rx_tcp_packets} tx_tcp_packets={tx_tcp_packets} icmp_sent=3 icmp_received=3 dut_sockets={final_dut:?} peer_sockets={final_peer:?}"
                );
            }));
            // 所有失败（包括 panic）均先交由真实 owner 清理 DUT，再结束 peer。
            let dut_stopped = runtime.stop().is_ok();
            let peer_cleanup = peer.finish(dut_stopped);
            dut_private.fill(0);
            peer_private.fill(0);
            token_bytes.fill(0);
            assert!(
                dut_stopped,
                "DUT cleanup failed; retain private resources and fixture"
            );
            peer_cleanup.expect("peer stopped/exit/pipes all confirmed");
            let cleanup_deadline = peer.started + Duration::from_secs(59);
            let peer_after = owned_sockets(peer_pid, &root, cleanup_deadline);
            assert!(peer_after.tcp.is_empty() && peer_after.udp.is_empty());
            if let Some(pid) = dut_pid {
                let dut_after = owned_sockets(pid, &root, cleanup_deadline);
                assert!(dut_after.tcp.is_empty() && dut_after.udp.is_empty());
            }
            let port = runtime.into_port();
            assert!(port.running.is_empty() && !port.has_pending_cleanup());
            assert_eq!(
                fs::read_dir(data.join(RUNTIME_DIRECTORY))
                    .expect("private runtime root")
                    .count(),
                0
            );
            println!(
                "task009 wg cleanup=dut_then_peer confirmed=true stderr_bytes={}",
                peer.stderr_bytes.load(Ordering::SeqCst)
            );
            if let Err(panic) = assertions {
                std::panic::resume_unwind(panic);
            }
            for (owned_path, parent) in [(data, std::env::temp_dir()), (root, target)] {
                let resolved = owned_path.canonicalize().expect("resolved owned fixture");
                let parent = parent.canonicalize().expect("resolved parent");
                assert_eq!(resolved.parent(), Some(parent.as_path()));
                assert_eq!(
                    resolved.file_name().expect("owned name"),
                    std::ffi::OsStr::new(&name)
                );
                fs::remove_dir_all(resolved).expect("remove only successful owned fixture");
            }
        }
    }

    // netstat只读枚举；输出留在本次测试目录，日志仅打印已拥有child的两个端点。
    fn metering_listeners(pid: u32, output_path: &Path) -> BTreeSet<std::net::SocketAddr> {
        use std::{
            os::windows::process::CommandExt,
            process::Stdio,
            time::{Duration, Instant},
        };
        let executable = PathBuf::from(std::env::var_os("SystemRoot").expect("Windows root"))
            .join("System32/netstat.exe");
        let output = fs::File::create(output_path).expect("owned netstat output");
        let mut command = std::process::Command::new(executable);
        command
            .args(["-ano"])
            .creation_flags(0x0800_0000)
            .stdin(Stdio::null())
            .stdout(Stdio::from(output))
            .stderr(Stdio::null());
        let mut process = command.spawn().expect("read-only listener enumeration");
        let deadline = Instant::now() + Duration::from_secs(2);
        let status = loop {
            if let Some(status) = process.try_wait().expect("enumeration status") {
                break Some(status);
            }
            if Instant::now() >= deadline {
                break None;
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if status.is_none() {
            process.kill().expect("stop owned enumeration");
            let deadline = Instant::now() + Duration::from_secs(2);
            while process.try_wait().expect("enumeration exit").is_none()
                && Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        assert!(
            status.is_some_and(|status| status.success()),
            "listener enumeration deadline/status"
        );
        assert!(fs::metadata(output_path).expect("output metadata").len() <= 1024 * 1024);
        let bytes = fs::read(output_path).expect("listener output");
        let output = String::from_utf8_lossy(&bytes);
        output
            .lines()
            .filter_map(|line| {
                let fields = line.split_whitespace().collect::<Vec<_>>();
                if fields.len() == 5
                    && fields[0] == "TCP"
                    && fields[4].parse::<u32>().ok() == Some(pid)
                    && matches!(fields[2], "0.0.0.0:0" | "[::]:0")
                {
                    Some(fields[1].parse().expect("owned listener address"))
                } else {
                    None
                }
            })
            .collect()
    }

    // 固定整个块的期限，防止分段I/O不断重置socket超时而延长测试。
    fn metering_block_io(
        stream: &mut std::net::TcpStream,
        bytes: &mut [u8],
        write: bool,
        service_deadline: std::time::Instant,
    ) -> Result<(), &'static str> {
        use std::{
            io::{Read, Write},
            time::{Duration, Instant},
        };
        let deadline = service_deadline.min(Instant::now() + Duration::from_secs(2));
        let mut offset = 0;
        while offset < bytes.len() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or("block I/O deadline")?;
            let transferred = if write {
                stream
                    .set_write_timeout(Some(remaining))
                    .map_err(|_| "write timeout")?;
                stream.write(&bytes[offset..]).map_err(|_| "block write")?
            } else {
                stream
                    .set_read_timeout(Some(remaining))
                    .map_err(|_| "read timeout")?;
                stream
                    .read(&mut bytes[offset..])
                    .map_err(|_| "block read")?
            };
            if transferred == 0 {
                return Err("block closed");
            }
            offset += transferred;
        }
        Ok(())
    }

    #[test]
    fn metering_io_obeys_short_service_deadline_and_expired_write_sends_nothing() {
        use std::{
            io::Read,
            net::{TcpListener, TcpStream},
            time::{Duration, Instant},
        };
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("deadline fixture listener");
        let mut client =
            TcpStream::connect_timeout(&listener.local_addr().unwrap(), Duration::from_secs(2))
                .expect("deadline client");
        listener.set_nonblocking(true).expect("bounded accept");
        let (mut server, _) = listener.accept().expect("already connected peer");
        server
            .set_nonblocking(false)
            .expect("deadline blocking socket");
        let mut bytes = [17];
        assert_eq!(
            metering_block_io(&mut server, &mut bytes, true, Instant::now()),
            Err("block I/O deadline")
        );
        client.set_nonblocking(true).expect("inspect no write");
        assert_eq!(
            client.read(&mut bytes).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        let started = Instant::now();
        assert!(
            metering_block_io(
                &mut server,
                &mut bytes,
                false,
                started + Duration::from_millis(50)
            )
            .is_err()
        );
        assert!(started.elapsed() >= Duration::from_millis(40));
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "service cutoff must supersede the normal two-second I/O limit"
        );
    }

    #[test]
    fn real_loopback_metering_tracks_bytes_silence_and_new_instance() {
        use crate::{
            application::observability::{InMemoryRuntimeObservations, RuntimeObservationPort},
            domain::*,
            singbox::{
                SingBoxCompiler, managed_sidecar::generate_api_secret, runtime::SidecarRuntime,
            },
            subscription::{normalize_nodes, parse_subscription},
        };
        use std::{
            net::{TcpListener, TcpStream},
            num::NonZeroU16,
            sync::{
                Arc,
                atomic::{AtomicBool, Ordering},
            },
            thread,
            time::{Duration, Instant},
        };
        let _lock = FIXED_CLASH_API_TEST_LOCK.lock().expect("fixed API lock");
        let name = format!(
            "task009-metering-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        );
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(&name);
        let resources = root.join("resources");
        let bundled = resources.join(RESOURCE_DIRECTORY);
        fs::create_dir_all(&bundled).expect("owned resource directory");
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/sing-box-1.14.0-windows-amd64");
        for file in EXPECTED_RESOURCE_FILES {
            fs::hard_link(cache.join(file), bundled.join(file)).expect("fixed resource hardlink");
        }
        let data = std::env::temp_dir().join(&name);
        fs::create_dir(&data).expect("owned app data");
        let echo_listener = TcpListener::bind(("127.0.0.1", 0)).expect("owned echo listener");
        let echo_port = echo_listener.local_addr().expect("echo address").port();
        let reserved = TcpListener::bind(("127.0.0.1", 0)).expect("reserve test inlet");
        let inlet = reserved.local_addr().expect("inlet address");
        let parsed = parse_subscription(&serde_json::json!({"outbounds":[{
            "type":"socks","tag":"unused-local-node","server":"127.0.0.1","server_port":echo_port
        }]}).to_string()).expect("fixture subscription");
        assert!(parsed.skipped.is_empty());
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            id: SubscriptionId("metering".into()),
            name: "metering".into(),
        });
        state.providers.push(Provider {
            id: ProviderId("metering".into()),
            subscription_id: SubscriptionId("metering".into()),
            name: "metering".into(),
        });
        state.nodes = normalize_nodes(ProviderId("metering".into()), parsed.nodes)
            .expect("fixture normalization");
        state.pools.push(NodePool {
            id: PoolId("metering".into()),
            name: "metering".into(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("metering".into()),
                filter: NodeFilter::default(),
            }],
            selection: SelectionPolicy::Manual {
                selected_node_id: Some(state.nodes[0].id.clone()),
            },
            enabled: true,
        });
        state.default_target = RouteTarget::Pool(PoolId("metering".into()));
        state.routes.push(RoutePolicy {
            id: RoutePolicyId("metering-direct".into()),
            name: "metering".into(),
            enabled: true,
            priority: 0,
            matcher: TrafficMatcher::IpCidr(vec!["127.0.0.1/32".into()]),
            target: RouteTarget::Direct,
        });
        let intent = RuntimeIntent::from_state(&state).expect("whole test AppState");
        let plan = SingBoxCompiler
            .compile_loopback_metering(
                &intent,
                &state.default_target,
                DnsPolicy::System,
                NonZeroU16::new(inlet.port()).unwrap(),
                NonZeroU16::new(echo_port).unwrap(),
            )
            .expect("typed test-only metering plan");
        let first_secret = generate_api_secret().expect("first instance entropy");
        let first_config = plan.finalize(&first_secret).expect("first final bytes");
        let first_hash = format!("{:x}", Sha256::digest(first_config.as_bytes()));
        let port = WindowsManagedSidecarPort::new(resources.clone(), data.clone())
            .expect("fixed assets and ACL");
        let mut runtime = SidecarRuntime::new_observation_only(port);
        let observations = InMemoryRuntimeObservations::new_mock();
        let cancel = Arc::new(AtomicBool::new(false));
        let mut echo_thread = None;
        let mut client_thread = None;
        // 失败也先关闭有界网络任务并停止owner，再传播断言，避免泄漏私有实例。
        let assertions = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            drop(reserved);
            runtime
                .start_or_replace(first_config)
                .expect("checked metering core Ready");
            observations.record_managed_ready();
            let (first_identity, first_pid) = runtime
                .with_active_port(|port, child| {
                    let running = port.running.get(&child.identity()).expect("owned child");
                    assert_eq!(
                        format!(
                            "{:x}",
                            Sha256::digest(
                                fs::read(running.runtime.config_path()).expect("private bytes")
                            )
                        ),
                        first_hash
                    );
                    Ok((child.identity(), running.child.test_process_id()))
                })
                .expect("active identity")
                .expect("active child present");
            let expected = BTreeSet::from([inlet, "127.0.0.1:9090".parse().unwrap()]);
            assert_eq!(
                metering_listeners(first_pid, &root.join("listeners.txt")),
                expected
            );
            let initial = runtime
                .with_active_port(|port, child| port.sample_runtime_observation(child))
                .expect("initial observation")
                .expect("active child present");
            assert_eq!(
                (
                    initial.connections.upload_total_bytes,
                    initial.connections.download_total_bytes
                ),
                (0, 0)
            );
            assert!(initial.latest_log.is_none());
            // 80个已知块，收发各1.25MiB；发送覆盖8秒，观察正速率后再验证静默。
            const CHUNKS: usize = 80;
            const SIZE: usize = 16 * 1024;
            const BYTES: usize = CHUNKS * SIZE;
            assert!(BYTES <= 2 * 1024 * 1024);
            let echo_cancel = cancel.clone();
            // 主测试保留原listener到所有child停止；线程结束不能把固定目标端口让给其他进程。
            let echo_service_listener = echo_listener
                .try_clone()
                .expect("retain echo port ownership");
            let service_deadline = Instant::now() + Duration::from_secs(30);
            echo_thread = Some(thread::spawn(move || -> Result<usize, &'static str> {
                let deadline = service_deadline;
                echo_service_listener
                    .set_nonblocking(true)
                    .map_err(|_| "echo accept mode")?;
                let mut stream = loop {
                    if echo_cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                        return Err("echo cancelled/deadline");
                    }
                    match echo_service_listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(
                                deadline
                                    .saturating_duration_since(Instant::now())
                                    .min(Duration::from_millis(10)),
                            )
                        }
                        Err(_) => return Err("echo accept"),
                    }
                };
                // Windows accept可能继承listener非阻塞模式；块I/O采用固定期限的阻塞读写。
                stream
                    .set_nonblocking(false)
                    .map_err(|_| "echo stream mode")?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|_| "echo read timeout")?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .map_err(|_| "echo write timeout")?;
                for index in 0..CHUNKS {
                    if echo_cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                        return Err("echo cancelled/deadline");
                    }
                    let mut block = [0; SIZE];
                    metering_block_io(&mut stream, &mut block, false, deadline)?;
                    if !block.iter().all(|byte| *byte == index as u8) {
                        return Err("unexpected echo pattern");
                    }
                    metering_block_io(&mut stream, &mut block, true, deadline)?;
                }
                Ok(BYTES)
            }));
            let client_cancel = cancel.clone();
            client_thread = Some(thread::spawn(move || -> Result<usize, &'static str> {
                let deadline = service_deadline;
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or("client deadline")?;
                let mut stream =
                    TcpStream::connect_timeout(&inlet, remaining.min(Duration::from_secs(2)))
                        .map_err(|_| "client connect")?;
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .map_err(|_| "client read timeout")?;
                stream
                    .set_write_timeout(Some(Duration::from_secs(2)))
                    .map_err(|_| "client write timeout")?;
                for index in 0..CHUNKS {
                    if client_cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                        return Err("client cancelled/deadline");
                    }
                    let mut block = [index as u8; SIZE];
                    metering_block_io(&mut stream, &mut block, true, deadline)?;
                    let mut reply = [0; SIZE];
                    metering_block_io(&mut stream, &mut reply, false, deadline)?;
                    if reply != block {
                        return Err("unexpected client echo");
                    }
                    let remaining = deadline
                        .checked_duration_since(Instant::now())
                        .ok_or("client deadline")?;
                    thread::sleep(remaining.min(Duration::from_millis(100)));
                }
                if Instant::now() >= deadline {
                    return Err("client deadline");
                }
                Ok(BYTES)
            }));
            let mut positive = false;
            for _ in 0..4 {
                thread::sleep(Duration::from_millis(1100));
                let sample = runtime
                    .with_active_port(|port, child| port.sample_runtime_observation(child))
                    .expect("same child positive sample")
                    .expect("active child present");
                let traffic = sample.traffic.expect("due traffic window");
                positive |=
                    traffic.upload_bytes_per_second > 0 && traffic.download_bytes_per_second > 0;
                observations.record_managed_observation(sample);
                println!(
                    "metering phase=transfer identity={first_identity} pid={first_pid} config_sha256={first_hash} up_rate={} down_rate={} up_total={} down_total={}",
                    traffic.upload_bytes_per_second,
                    traffic.download_bytes_per_second,
                    traffic.upload_total_bytes,
                    traffic.download_total_bytes
                );
            }
            assert!(
                positive,
                "real routed flow yields positive bidirectional rates"
            );
            assert!(
                observations
                    .snapshot()
                    .traffic_history
                    .iter()
                    .any(|point| point.upload_rate_bps > 0 && point.download_rate_bps > 0)
            );
            assert_eq!(
                client_thread.take().unwrap().join().expect("client join"),
                Ok(BYTES)
            );
            assert_eq!(
                echo_thread.take().unwrap().join().expect("echo join"),
                Ok(BYTES)
            );
            assert_eq!(
                echo_listener
                    .local_addr()
                    .expect("retained echo address")
                    .port(),
                echo_port
            );
            assert!(
                TcpListener::bind(("127.0.0.1", echo_port)).is_err(),
                "echo target remains owned after service thread exit"
            );
            for _ in 0..2 {
                thread::sleep(Duration::from_millis(2100));
                let sample = runtime
                    .with_active_port(|port, child| port.sample_runtime_observation(child))
                    .expect("quiet sample")
                    .expect("active child present");
                let traffic = sample.traffic.expect("quiet window");
                assert_eq!(
                    (
                        traffic.upload_bytes_per_second,
                        traffic.download_bytes_per_second
                    ),
                    (0, 0)
                );
                assert_eq!(
                    (
                        sample.connections.upload_total_bytes,
                        sample.connections.download_total_bytes
                    ),
                    (BYTES as u64, BYTES as u64)
                );
                assert_eq!(sample.connections.connection_count, 0);
                observations.record_managed_observation(sample);
            }
            println!(
                "metering phase=silent identity={first_identity} exact_bytes_each={BYTES} rates=0 totals_stable=true"
            );
            runtime.stop().expect("confirmed first Stop");
            assert!(
                runtime
                    .with_active_port(|port, child| port.sample_runtime_observation(child))
                    .expect("stopped runtime read")
                    .is_none()
            );
            observations.record_managed_stopped();
            assert!(observations.snapshot().traffic_history.is_empty());
            let second_secret = generate_api_secret().expect("second instance entropy");
            assert!(first_secret.as_str() != second_secret.as_str());
            let second_config = plan.finalize(&second_secret).expect("second final bytes");
            let second_hash = format!("{:x}", Sha256::digest(second_config.as_bytes()));
            assert_ne!(first_hash, second_hash);
            runtime
                .start_or_replace(second_config)
                .expect("second checked instance Ready");
            observations.record_managed_ready();
            assert!(observations.snapshot().traffic_history.is_empty());
            let (second_identity, second_pid) = runtime
                .with_active_port(|port, child| {
                    let running = port
                        .running
                        .get(&child.identity())
                        .expect("second owned child");
                    assert_eq!(
                        format!(
                            "{:x}",
                            Sha256::digest(
                                fs::read(running.runtime.config_path())
                                    .expect("second private bytes")
                            )
                        ),
                        second_hash
                    );
                    Ok((child.identity(), running.child.test_process_id()))
                })
                .expect("second identity")
                .expect("active child present");
            assert!(second_identity > first_identity);
            assert_eq!(
                metering_listeners(second_pid, &root.join("listeners.txt")),
                expected
            );
            assert!(
                TcpListener::bind(("127.0.0.1", echo_port)).is_err(),
                "echo target remains owned throughout new instance"
            );
            let old_client = ClashApiClient::new(&first_secret).expect("old auth fixture");
            assert!(
                tauri::async_runtime::block_on(old_client.read_ready()).is_err(),
                "old secret rejected by new child"
            );
            let sample = runtime
                .with_active_port(|port, child| port.sample_runtime_observation(child))
                .expect("new authenticated sample")
                .expect("active child present");
            assert_eq!(
                (
                    sample.connections.upload_total_bytes,
                    sample.connections.download_total_bytes
                ),
                (0, 0)
            );
            let traffic = sample.traffic.expect("new window");
            assert_eq!(
                (
                    traffic.upload_bytes_per_second,
                    traffic.download_bytes_per_second
                ),
                (0, 0)
            );
            observations.record_managed_observation(sample);
            assert!(
                observations
                    .snapshot()
                    .traffic_history
                    .iter()
                    .all(|point| point.upload_rate_bps == 0 && point.download_rate_bps == 0)
            );
            println!(
                "metering phase=new_instance identity={second_identity} pid={second_pid} config_sha256={second_hash} old_auth_rejected=true totals=0 history_isolated=true listeners=api+loopback_inlet"
            );
        }));
        cancel.store(true, Ordering::Relaxed);
        let client_done = client_thread.map(|handle| handle.join());
        let echo_done = echo_thread.map(|handle| handle.join());
        let stopped = runtime.stop();
        let port = runtime.into_port();
        assert!(
            stopped.is_ok() && port.running.is_empty() && port.pending.is_none(),
            "owned core cleanup confirmed"
        );
        drop(port);
        drop(echo_listener);
        assert!(client_done.is_none_or(|result| result.is_ok()));
        assert!(echo_done.is_none_or(|result| result.is_ok()));
        // 解析路径后仅删除本次自有根；历史失败夹具不在本次清理集合中。
        for (path, parent) in [
            (data, std::env::temp_dir()),
            (
                root,
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target"),
            ),
        ] {
            let resolved = path.canonicalize().expect("owned root resolution");
            let parent = parent.canonicalize().expect("owned parent resolution");
            assert_eq!(resolved.parent(), Some(parent.as_path()));
            assert_eq!(resolved.file_name().unwrap(), std::ffi::OsStr::new(&name));
            fs::remove_dir_all(resolved).expect("remove only confirmed stopped fixture");
        }
        if let Err(panic) = assertions {
            std::panic::resume_unwind(panic);
        }
    }

    #[test]
    fn fixed_bundle_resources_run_and_stop_only_the_owned_loopback_child() {
        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        let cache_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join("sing-box-1.14.0-windows-amd64");
        if !cache_root.is_dir() {
            eprintln!("managed sidecar preflight unavailable: fixed build cache is absent");
            return;
        }
        let test_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "veyra-managed-port-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock after epoch")
                    .as_nanos()
            ));
        let resource_root = test_root.join("resources");
        let resource_directory = resource_root.join(RESOURCE_DIRECTORY);
        fs::create_dir_all(&resource_directory).expect("create resource fixture");
        for file in EXPECTED_RESOURCE_FILES {
            fs::hard_link(cache_root.join(file), resource_directory.join(file))
                .expect("hard link verified resource fixture");
        }
        let runtime_root = std::env::temp_dir().join(format!(
            "veyra-managed-runtime-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        ));
        fs::create_dir(&runtime_root).expect("create runtime root");
        let orphaned_runtime = PrivateRuntime::create(&runtime_root.join(RUNTIME_DIRECTORY), None)
            .expect("prepare private runtime");
        drop(orphaned_runtime);
        let mut port = WindowsManagedSidecarPort::new(resource_root.clone(), runtime_root.clone())
            .expect("recover proven orphan and verify fixed bundle resources");
        assert!(
            fs::read_dir(runtime_root.join(RUNTIME_DIRECTORY))
                .expect("read recovered managed runtime directory")
                .next()
                .is_none()
        );
        let candidate = compile_subscription_node(
            serde_json::json!({
                "type":"socks", "tag":"fixture", "server":"127.0.0.1", "server_port":1080
            }),
            false,
        );

        port.check(&candidate).expect("fixed sing-box check");
        port.prepare(&candidate).expect("pending checked config");
        let child = port.run().expect("start fixed loopback child");
        assert!(port.sample_runtime_observation(&child).is_err());
        port.ready(&child).expect("owned child stays running");
        {
            let running = port.running.get(&child.identity()).expect("owned instance");
            let client = ClashApiClient::new(&running.secret).expect("fixed authenticated client");
            tauri::async_runtime::block_on(client.read_connections())
                .expect("real connections endpoint");
            tauri::async_runtime::block_on(client.read_traffic_once())
                .expect("real traffic WebSocket endpoint");
            assert!(
                tauri::async_runtime::block_on(client.read_log_once())
                    .expect("real disabled log endpoint must yield an empty summary")
                    .is_none()
            );
        }
        let observation = port
            .sample_runtime_observation(&child)
            .expect("read the real fixed loopback summaries");
        assert!(observation.traffic.is_some());
        port.stop(&child).expect("stop owned child");
        assert!(port.sample_runtime_observation(&child).is_err());
        assert!(
            fs::read_dir(runtime_root.join(RUNTIME_DIRECTORY))
                .expect("read cleaned managed runtime directory")
                .next()
                .is_none()
        );
        fs::remove_dir(runtime_root.join(RUNTIME_DIRECTORY))
            .expect("remove managed runtime directory");
        fs::remove_dir(&runtime_root).expect("remove runtime root");
        for file in EXPECTED_RESOURCE_FILES {
            fs::remove_file(resource_directory.join(file)).expect("remove resource fixture");
        }
        fs::remove_dir(&resource_directory).expect("remove versioned resource directory");
        fs::remove_dir(resource_directory.parent().expect("resource parent"))
            .expect("remove resource family directory");
        fs::remove_dir(&resource_root).expect("remove resource root");
        fs::remove_dir(&test_root).expect("remove test root");
    }
    #[test]
    fn creation_cleanup_failure_stays_owned_by_port_until_manual_stop() {
        use crate::singbox::runtime::{SidecarError, SidecarLifecycle, SidecarRuntime};
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("task009-create-owner-{suffix}"));
        let bundled = resources.join(RESOURCE_DIRECTORY);
        fs::create_dir_all(&bundled).unwrap();
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/sing-box-1.14.0-windows-amd64");
        for file in EXPECTED_RESOURCE_FILES {
            fs::hard_link(cache.join(file), bundled.join(file)).unwrap();
        }
        let data = std::env::temp_dir().join(format!("veyra-create-owner-{suffix}"));
        fs::create_dir(&data).unwrap();
        let mut port = WindowsManagedSidecarPort::new(resources.clone(), data.clone()).unwrap();
        port.after_instance_created = Some(|instance| {
            fs::create_dir(instance.join("config.json")).expect("create owned I/O blocker");
        });
        let candidate = compile_subscription_node(
            serde_json::json!({
                "type":"socks", "tag":"local", "server":"127.0.0.1", "server_port":1080
            }),
            false,
        );
        // 经真实 check -> create 的生产错误返回路径，尚未启动任何 check/运行进程。
        assert!(port.check(&candidate).is_err());
        let pending = port
            .pending
            .as_ref()
            .expect("Port must retain failed creation owner");
        assert!(!pending.checked);
        assert!(pending.check_child.is_none());
        let blocker = pending.runtime.config_path().to_owned();
        let instance = blocker.parent().unwrap().to_owned();
        assert!(blocker.is_dir());
        assert_eq!(port.next_identity, 0);
        assert!(port.running.is_empty());
        assert!(port.has_pending_cleanup());

        let mut runtime = SidecarRuntime::new_observation_only(port);
        assert_eq!(
            runtime.start_or_replace(candidate),
            Err(SidecarError::RecoveryRequired)
        );
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert_eq!(runtime.stop(), Err(SidecarError::CandidateStop));
        assert_eq!(
            runtime.snapshot().lifecycle,
            SidecarLifecycle::RecoveryRequired
        );
        assert!(instance.exists());
        fs::remove_dir(&blocker).unwrap();
        runtime
            .stop()
            .expect("manual Stop cleans the retained creation owner");
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert!(!instance.exists());
        let port = runtime.into_port();
        assert!(port.pending.is_none());
        assert!(!port.has_pending_cleanup());
        assert_eq!(port.next_identity, 0);
        assert!(port.running.is_empty());
        assert_eq!(
            fs::read_dir(data.join(RUNTIME_DIRECTORY)).unwrap().count(),
            0
        );
        fs::remove_dir(data.join(RUNTIME_DIRECTORY)).unwrap();
        fs::remove_dir(data).unwrap();
        for file in EXPECTED_RESOURCE_FILES {
            fs::remove_file(bundled.join(file)).unwrap();
        }
        fs::remove_dir(&bundled).unwrap();
        fs::remove_dir(bundled.parent().unwrap()).unwrap();
        fs::remove_dir(resources).unwrap();
    }

    #[test]
    fn fixed_core_runtime_replacement_check_failure_and_ready_fail_stop() {
        use crate::singbox::runtime::{SidecarError, SidecarLifecycle, SidecarRuntime};
        use std::{cell::Cell, rc::Rc};

        struct ObservedPort {
            inner: WindowsManagedSidecarPort,
            fail_ready: Rc<Cell<bool>>,
            runs: Rc<Cell<u64>>,
        }
        impl SidecarPort for ObservedPort {
            fn check(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError> {
                self.inner.check(candidate)
            }
            fn prepare(&mut self, candidate: &GeneratedConfig) -> Result<(), SidecarPortError> {
                self.inner.prepare(candidate)
            }
            fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
                let child = self.inner.run()?;
                self.runs.set(self.runs.get() + 1);
                Ok(child)
            }
            fn ready(&mut self, child: &ManagedSidecar) -> Result<(), SidecarPortError> {
                if self.fail_ready.get() {
                    // 只改变测试客户端凭据，已检查的文件保持不变，真实 API 应拒绝认证。
                    self.inner
                        .running
                        .get_mut(&child.identity())
                        .unwrap()
                        .secret = crate::singbox::managed_sidecar::generate_api_secret().unwrap();
                }
                self.inner.ready(child)
            }
            fn stop(&mut self, child: &ManagedSidecar) -> Result<(), SidecarPortError> {
                self.inner.stop(child)
            }
            fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
                self.inner.cancel_pending()
            }
            fn has_pending_cleanup(&self) -> bool {
                self.inner.has_pending_cleanup()
            }
        }

        let _lock = FIXED_CLASH_API_TEST_LOCK
            .lock()
            .expect("fixed API test lock");
        // 仅用固定资产、手动 SOCKS loopback 节点和独立私有目录；不发起代理目标请求。
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/sing-box-1.14.0-windows-amd64");
        assert!(cache.is_dir(), "fixed asset prerequisite must be available");
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let resources = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!("task009-runtime-{suffix}"));
        let bundled = resources.join(RESOURCE_DIRECTORY);
        fs::create_dir_all(&bundled).unwrap();
        for file in EXPECTED_RESOURCE_FILES {
            fs::hard_link(cache.join(file), bundled.join(file)).unwrap();
        }
        let data = std::env::temp_dir().join(format!("veyra-task009-{suffix}"));
        fs::create_dir(&data).unwrap();
        let inner = WindowsManagedSidecarPort::new(resources.clone(), data.clone()).unwrap();
        let fail_ready = Rc::new(Cell::new(false));
        let runs = Rc::new(Cell::new(0));
        let mut runtime = SidecarRuntime::new_observation_only(ObservedPort {
            inner,
            fail_ready: fail_ready.clone(),
            runs: runs.clone(),
        });
        let fixture = || {
            compile_subscription_node(
                serde_json::json!({
                    "type":"socks", "tag":"local", "server":"127.0.0.1", "server_port":1080
                }),
                false,
            )
        };

        let first = fixture();
        let first_secret = api_secret_from_config(&first).unwrap();
        let first_digest: [u8; 32] = Sha256::digest(first.as_bytes()).into();
        runtime
            .start_or_replace(first)
            .expect("real cold start and Ready");
        let (first_identity, first_path) = runtime
            .with_active_port(|port, child| {
                let running = port.inner.running.get(&child.identity()).unwrap();
                assert!(matches_final_digest(
                    running.runtime.config_path(),
                    first_digest
                ));
                Ok((child.identity(), running.runtime.config_path().to_owned()))
            })
            .unwrap()
            .unwrap();
        eprintln!(
            "TASK009 first identity={first_identity} final_sha256={:x}",
            Sha256::digest(fs::read(&first_path).unwrap())
        );

        // 最终白名单/check失败不停止旧 child，也不消费下一 identity。
        assert_eq!(
            runtime.start_or_replace(GeneratedConfig::from_bytes(b"{}".to_vec())),
            Err(SidecarError::CandidateCheck)
        );
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Ready);
        runtime
            .with_active_port(|port, child| {
                assert_eq!(child.identity(), first_identity);
                assert!(
                    port.inner
                        .running
                        .get_mut(&child.identity())
                        .unwrap()
                        .child
                        .is_running()
                        .unwrap()
                );
                assert!(port.inner.pending.is_none());
                Ok(())
            })
            .unwrap();
        assert_eq!(runs.get(), 1);

        let second = fixture();
        let second_secret = api_secret_from_config(&second).unwrap();
        assert!(
            first_secret.as_str() != second_secret.as_str(),
            "replacement must use a new API secret"
        );
        let second_digest: [u8; 32] = Sha256::digest(second.as_bytes()).into();
        runtime
            .start_or_replace(second)
            .expect("real internal replacement");
        assert!(!first_path.exists());
        let second_path = runtime
            .with_active_port(|port, child| {
                assert_ne!(child.identity(), first_identity);
                assert_eq!(port.inner.running.len(), 1);
                let running = port.inner.running.get(&child.identity()).unwrap();
                assert!(matches_final_digest(
                    running.runtime.config_path(),
                    second_digest
                ));
                assert!(
                    tauri::async_runtime::block_on(
                        ClashApiClient::new(&first_secret).unwrap().read_ready()
                    )
                    .is_err()
                );
                tauri::async_runtime::block_on(
                    ClashApiClient::new(&second_secret).unwrap().read_ready(),
                )
                .unwrap();
                eprintln!(
                    "TASK009 replacement identity={} final_sha256={:x}",
                    child.identity(),
                    Sha256::digest(fs::read(running.runtime.config_path()).unwrap())
                );
                Ok(running.runtime.config_path().to_owned())
            })
            .unwrap()
            .unwrap();

        fail_ready.set(true);
        assert_eq!(
            runtime.start_or_replace(fixture()),
            Err(SidecarError::CandidateReady)
        );
        assert_eq!(runs.get(), 3, "failed candidate never restarts previous");
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert!(!runtime.snapshot().has_active_config);
        assert!(!second_path.exists());
        let mut observed = runtime.into_port();
        assert!(observed.inner.running.is_empty());
        assert!(observed.inner.pending.is_none());
        assert_eq!(
            fs::read_dir(data.join(RUNTIME_DIRECTORY)).unwrap().count(),
            0
        );

        // prepare 必须匹配原候选，run 必须拒绝 check 后变更的字节。
        let candidate = fixture();
        observed.inner.check(&candidate).unwrap();
        assert!(observed.inner.prepare(&fixture()).is_err());
        observed.inner.prepare(&candidate).unwrap();
        let pending_path = observed
            .inner
            .pending
            .as_ref()
            .unwrap()
            .runtime
            .config_path()
            .to_owned();
        fs::write(&pending_path, b"{}").unwrap();
        assert!(observed.inner.run().is_err());
        assert!(!pending_path.exists());
        assert_eq!(observed.inner.next_identity, 3);

        // 私有目录删除失败不能丢弃候选；移除测试占位后可由手动 Stop 完成清理。
        observed.inner.check(&fixture()).unwrap();
        let pending_path = observed
            .inner
            .pending
            .as_ref()
            .unwrap()
            .runtime
            .config_path()
            .to_owned();
        let owned_marker = pending_path.parent().unwrap().join("owned-test-marker");
        fs::write(&owned_marker, b"test").unwrap();
        assert!(observed.inner.cancel_pending().is_err());
        assert!(observed.inner.has_pending_cleanup());
        assert!(observed.inner.check(&fixture()).is_err());
        fs::remove_file(owned_marker).unwrap();
        observed.inner.cancel_pending().unwrap();
        assert!(!observed.inner.has_pending_cleanup());
        assert!(observed.inner.pending.is_none());

        // 无关 listener 由测试明确拥有；端口冲突只停止新候选，不能影响占用者。
        fail_ready.set(false);
        let unrelated = std::net::TcpListener::bind("127.0.0.1:9090").unwrap();
        let mut runtime = SidecarRuntime::new_observation_only(observed);
        assert_eq!(
            runtime.start_or_replace(fixture()),
            Err(SidecarError::CandidateReady)
        );
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert_eq!(unrelated.local_addr().unwrap().port(), 9090);
        assert!(
            std::net::TcpStream::connect_timeout(
                &unrelated.local_addr().unwrap(),
                std::time::Duration::from_millis(200)
            )
            .is_ok()
        );
        drop(unrelated);

        // 对明确归属的 child 注入退出，随后观测失败清理私有文件；没有自动重启。
        runtime
            .start_or_replace(fixture())
            .expect("manual start after failed candidate cleanup");
        let crashed_path = runtime
            .with_active_port(|port, child| {
                let running = port.inner.running.get_mut(&child.identity()).unwrap();
                let path = running.runtime.config_path().to_owned();
                running.child.stop().unwrap();
                assert!(port.inner.sample_runtime_observation(child).is_err());
                Ok(path)
            })
            .unwrap()
            .unwrap();
        let runs_before_cleanup = runs.get();
        runtime.recover_active_failure().unwrap();
        assert_eq!(runtime.snapshot().lifecycle, SidecarLifecycle::Stopped);
        assert_eq!(runs.get(), runs_before_cleanup);
        assert!(!crashed_path.exists());
        let observed = runtime.into_port();
        assert!(observed.inner.running.is_empty());
        assert!(observed.inner.pending.is_none());
        let released = std::net::TcpListener::bind("127.0.0.1:9090").unwrap();
        drop(released);

        fs::remove_dir(data.join(RUNTIME_DIRECTORY)).unwrap();
        fs::remove_dir(&data).unwrap();
        for file in EXPECTED_RESOURCE_FILES {
            fs::remove_file(bundled.join(file)).unwrap();
        }
        fs::remove_dir(&bundled).unwrap();
        fs::remove_dir(bundled.parent().unwrap()).unwrap();
        fs::remove_dir(&resources).unwrap();
    }

    fn compile_subscription_node(node: serde_json::Value, urltest: bool) -> GeneratedConfig {
        use crate::domain::*;
        use crate::singbox::{ConfigCompiler, RuntimeProfile, SingBoxCompiler};
        use crate::subscription::{normalize_nodes, parse_subscription};
        let protocol = node["type"].as_str().expect("fixture protocol").to_owned();
        let input = serde_json::json!({"outbounds": [node]});
        let parsed = parse_subscription(&input.to_string()).expect("parse subscription fixture");
        assert!(
            parsed.skipped.is_empty(),
            "fixture must preserve all {protocol} protocol fields"
        );
        assert_eq!(parsed.nodes.len(), 1);
        let mut state = AppState::empty();
        state.subscriptions.push(Subscription {
            id: SubscriptionId("subscription".into()),
            name: "fixture".into(),
        });
        state.providers.push(Provider {
            id: ProviderId("provider".into()),
            subscription_id: SubscriptionId("subscription".into()),
            name: "fixture".into(),
        });
        state.nodes = normalize_nodes(ProviderId("provider".into()), parsed.nodes)
            .expect("normalize fixture");
        state.pools.push(NodePool {
            id: PoolId("chosen".into()),
            name: "fixture".into(),
            kind: PoolKind::Custom,
            sources: vec![PoolSource {
                provider_id: ProviderId("provider".into()),
                filter: NodeFilter::default(),
            }],
            selection: if urltest {
                SelectionPolicy::UrlTest {
                    probe_url: "https://probe.example.invalid/check".into(),
                    interval_secs: 300,
                    tolerance_ms: 50,
                }
            } else {
                SelectionPolicy::Manual {
                    selected_node_id: Some(state.nodes[0].id.clone()),
                }
            },
            enabled: true,
        });
        state.default_target = RouteTarget::Pool(PoolId("chosen".into()));
        state.routes = vec![
            RoutePolicy {
                id: RoutePolicyId("pool-route".into()),
                name: "fixture".into(),
                enabled: true,
                priority: 0,
                matcher: TrafficMatcher::DomainSuffix(vec!["example.invalid".into()]),
                target: state.default_target.clone(),
            },
            RoutePolicy {
                id: RoutePolicyId("direct-route".into()),
                name: "fixture".into(),
                enabled: true,
                priority: 1,
                matcher: TrafficMatcher::Port(vec![80]),
                target: RouteTarget::Direct,
            },
            RoutePolicy {
                id: RoutePolicyId("block-route".into()),
                name: "fixture".into(),
                enabled: true,
                priority: 2,
                matcher: TrafficMatcher::Domain(vec!["blocked.invalid".into()]),
                target: RouteTarget::Block,
            },
        ];
        let intent = RuntimeIntent::from_state(&state).expect("whole state validation");
        SingBoxCompiler
            .compile(
                &intent,
                &state.default_target,
                DnsPolicy::System,
                RuntimeProfile::ObservationOnly,
            )
            .expect("compile typed plan")
            .finalize(
                &crate::singbox::managed_sidecar::generate_api_secret().expect("fixture entropy"),
            )
            .expect("bind final config")
    }

    #[test]
    fn fixed_core_checks_final_subscription_matrix_without_starting_children() {
        use sha2::{Digest, Sha256};
        let input: serde_json::Value = serde_json::from_str(
            r#"{
  "outbounds": [
    {
      "type": "vless",
      "tag": "vless-reality-tcp",
      "server": "192.0.2.1",
      "server_port": 443,
      "uuid": "00000000-0000-4000-8000-000000000001",
      "tls": {
        "enabled": true,
        "server_name": "example.invalid",
        "reality": {
          "enabled": true,
          "public_key": "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE",
          "short_id": "0123456789abcdef"
        }
      }
    },
    {
      "type": "vless",
      "tag": "vless-reality-websocket",
      "server": "192.0.2.1",
      "server_port": 443,
      "uuid": "00000000-0000-4000-8000-000000000001",
      "tls": {
        "enabled": true,
        "server_name": "example.invalid",
        "reality": {
          "enabled": true,
          "public_key": "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE",
          "short_id": "0123456789abcdef"
        }
      },
      "transport": {"type": "ws", "path": "/ws", "headers": {"Host": "edge.example.invalid"}}
    },
    {
      "type": "socks",
      "tag": "socks",
      "server": "192.0.2.1",
      "server_port": 443,
      "password": "synthetic-password",
      "version": 5,
      "username": "fixture-user"
    },
    {
      "type": "http",
      "tag": "http",
      "server": "192.0.2.1",
      "server_port": 443,
      "username": "fixture-user",
      "password": "synthetic-password"
    },
    {
      "type": "shadowsocks",
      "tag": "shadowsocks",
      "server": "192.0.2.1",
      "server_port": 443,
      "method": "aes-128-gcm",
      "password": "synthetic-password"
    },
    {
      "type": "vmess",
      "tag": "vmess",
      "server": "192.0.2.1",
      "server_port": 443,
      "security": "auto",
      "alter_id": 0,
      "uuid": "00000000-0000-4000-8000-000000000001"
    },
    {
      "type": "vless",
      "tag": "vless",
      "server": "192.0.2.1",
      "server_port": 443,
      "uuid": "00000000-0000-4000-8000-000000000001"
    },
    {
      "type": "trojan",
      "tag": "trojan",
      "server": "192.0.2.1",
      "server_port": 443,
      "password": "synthetic-password",
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      }
    },
    {
      "type": "hysteria",
      "tag": "hysteria",
      "server": "192.0.2.1",
      "server_port": 443,
      "up_mbps": 10,
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      },
      "down_mbps": 20,
      "auth_str": "synthetic-password"
    },
    {
      "type": "hysteria2",
      "tag": "hysteria2",
      "server": "192.0.2.1",
      "server_port": 443,
      "password": "synthetic-password",
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      }
    },
    {
      "type": "tuic",
      "tag": "tuic",
      "server": "192.0.2.1",
      "server_port": 443,
      "udp_relay_mode": "native",
      "zero_rtt": false,
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      },
      "uuid": "00000000-0000-4000-8000-000000000001",
      "password": "synthetic-password",
      "congestion_control": "bbr"
    },
    {
      "type": "shadowtls",
      "tag": "shadowtls",
      "server": "192.0.2.1",
      "server_port": 443,
      "password": "synthetic-password",
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      },
      "version": 3
    },
    {
      "type": "ssh",
      "tag": "ssh",
      "server": "192.0.2.1",
      "server_port": 443,
      "user": "fixture-user",
      "password": "synthetic-password"
    },
    {
      "type": "naive",
      "tag": "naive",
      "server": "192.0.2.1",
      "server_port": 443,
      "password": "synthetic-password",
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      },
      "username": "fixture-user"
    },
    {
      "type": "anytls",
      "tag": "anytls",
      "server": "192.0.2.1",
      "server_port": 443,
      "password": "synthetic-password",
      "tls": {
        "server_name": "example.invalid",
        "enabled": true
      }
    },
    {
      "type": "snell",
      "tag": "snell",
      "server": "192.0.2.1",
      "server_port": 443,
      "version": 4,
      "psk": "synthetic-password"
    },
    {
      "type": "wireguard",
      "tag": "wireguard",
      "server": "192.0.2.1",
      "server_port": 51820,
      "private_key": "AQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQE=",
      "peer_public_key": "AgICAgICAgICAgICAgICAgICAgICAgICAgICAgICAgI=",
      "local_address": [
        "10.0.0.2/32"
      ],
      "mtu": 1400
    }
  ]
}"#,
        )
        .expect("matrix fixture");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join(format!(
                "task008-final-check-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("clock")
                    .as_nanos()
            ));
        let resources = root.join("resources").join(RESOURCE_DIRECTORY);
        fs::create_dir_all(&resources).expect("resource fixture directory");
        let cache = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries/sing-box-1.14.0-windows-amd64");
        for file in EXPECTED_RESOURCE_FILES {
            fs::hard_link(cache.join(file), resources.join(file))
                .expect("fixed asset must be available");
        }
        let app_data = std::env::temp_dir().join(root.file_name().expect("isolated test name"));
        fs::create_dir(&app_data).expect("temporary app-local data fixture");
        let mut port = WindowsManagedSidecarPort::new(root.join("resources"), app_data.clone())
            .expect("verified fixed core");
        for node in input["outbounds"].as_array().expect("matrix nodes") {
            for urltest in [false, true] {
                let candidate = compile_subscription_node(node.clone(), urltest);
                let identity = Sha256::digest(candidate.as_bytes());
                let secret = api_secret_from_config(&candidate).expect("extract existing binding");
                let final_document: serde_json::Value =
                    serde_json::from_slice(candidate.as_bytes()).expect("final compiler document");
                assert!(
                    final_document["experimental"]["clash_api"]["secret"].as_str()
                        == Some(secret.as_str()),
                    "extracted secret must match this final candidate"
                );
                assert!(
                    port.check(&candidate).is_ok(),
                    "fixed core rejected protocol {} urltest={urltest}",
                    node["type"]
                );
                let pending = port.pending.as_ref().expect("checked file");
                let readback =
                    fs::read(pending.runtime.config_path()).expect("private final config");
                assert!(
                    readback == candidate.as_bytes(),
                    "private file must match the final candidate bytes"
                );
                assert_eq!(Sha256::digest(&readback), identity);
                assert!(
                    pending.secret.as_str() == secret.as_str(),
                    "checked file binding must match candidate"
                );
                assert!(port.running.is_empty());
                assert_eq!(port.next_identity, 0, "check must not call run");
                println!(
                    "final-check protocol={} urltest={} sha256={:x} case={}",
                    node["type"].as_str().expect("protocol"),
                    urltest,
                    identity,
                    node["tag"].as_str().expect("case label")
                );
                port.discard_pending().expect("remove checked fixture");
                assert!(port.pending.is_none());
            }
        }
        let rejected = GeneratedConfig::from_bytes(br#"{"inbounds":[{"type":"tun"}]}"#.to_vec());
        assert!(port.check(&rejected).is_err());
        assert!(port.pending.is_none());
        assert!(port.running.is_empty());
        assert_eq!(port.next_identity, 0);
        drop(port);
        fs::remove_dir_all(root).expect("remove isolated fixture root");
        fs::remove_dir_all(app_data).expect("remove isolated app-data root");
    }
}
