//! TASK-006 的受管 sing-box 资产与启动边界。
//!
//! 这里不提供通用命令、路径或 JSON 入口。调用方只能在后续生命周期接线中使用
//! 已打包的固定资源、由语义编译器产生的配置和本模块生成的临时 secret。

// 本 checkpoint 先交付独立可测的安全边界；真实生命周期接线将在同一 TASK 的后续分区完成。
#![allow(dead_code)]

use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use getrandom::fill;
use serde_json::Value;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::GeneratedConfig;

pub(crate) const SIDECAR_ARCHIVE_FILE: &str = "sing-box-1.14.0-windows-amd64.zip";
pub(crate) const SIDECAR_EXECUTABLE_FILE: &str = "sing-box.exe";
pub(crate) const SIDECAR_LIBRARY_FILE: &str = "libcronet.dll";
pub(crate) const SIDECAR_LICENSE_FILE: &str = "LICENSE";
pub(crate) const SIDECAR_ARCHIVE_SHA256: &str =
    "3ffb56267da14e287be48bd10cf7e6505260125bad940b75101fbb4d5d58e5d6";
pub(crate) const SIDECAR_EXECUTABLE_SHA256: &str =
    "aad0ede010eafa7b277e520464f3a66fde820103d737eff739f40f3cc9451dcc";
pub(crate) const SIDECAR_LIBRARY_SHA256: &str =
    "eee741046f0a3975124bae349aeac237aa306f3cc4de59ff5de070e74dbfdaeb";
pub(crate) const SIDECAR_LICENSE_SHA256: &str =
    "bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c";

const SECRET_BYTES: usize = 32;
const CHECK_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_TIMEOUT: Duration = Duration::from_secs(2);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Error, Eq, PartialEq)]
pub(crate) enum ManagedSidecarError {
    #[error("embedded sidecar asset integrity verification failed")]
    AssetIntegrity,
    #[error("generated sidecar configuration violates the managed runtime contract")]
    ConfigurationRejected,
    #[error("could not generate the sidecar API secret")]
    SecretGeneration,
    #[error("sidecar configuration check failed")]
    ConfigurationCheck,
    #[error("could not launch the managed sidecar")]
    Spawn,
    #[error("could not stop the managed sidecar")]
    Stop,
}

/// 仅受管 sidecar 与固定 API client 可借用的短生命周期 secret。
///
/// 它不实现 `Clone`、`Debug` 或字符串转换，避免被传播到错误、事件或 UI。释放时会覆盖
/// 自己持有的 ASCII hex 缓冲区；生成配置序列化过程中的临时副本同样只在写入私有 config 前后短暂存在。
pub(crate) struct ApiSecret {
    value: String,
}

impl ApiSecret {
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }
}

impl Drop for ApiSecret {
    fn drop(&mut self) {
        // ASCII hex 长度固定，NUL 仍是合法 UTF-8，因此不会破坏 `String` 不变量。
        unsafe {
            self.value.as_mut_vec().fill(0);
        }
    }
}

/// 从操作系统熵源生成单次运行使用的 API secret，不持久化且不接受外部输入。
pub(crate) fn generate_api_secret() -> Result<ApiSecret, ManagedSidecarError> {
    let mut bytes = [0_u8; SECRET_BYTES];
    fill(&mut bytes).map_err(|_| ManagedSidecarError::SecretGeneration)?;

    let mut secret = String::with_capacity(SECRET_BYTES * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut secret, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(ApiSecret { value: secret })
}

/// 验证唯一允许的构建缓存归档及其完整三文件内容，避免打包替换的内置资产。
pub(crate) fn verify_embedded_assets(
    archive_path: &Path,
    executable_path: &Path,
    library_path: &Path,
    license_path: &Path,
) -> Result<(), ManagedSidecarError> {
    verify_file_sha256(archive_path, SIDECAR_ARCHIVE_SHA256)?;
    verify_bundled_resources(executable_path, library_path, license_path)
}

/// 运行期只接受 Tauri bundle 中固定的 executable、DLL 与许可证，且逐个校验身份。
pub(crate) fn verify_bundled_resources(
    executable_path: &Path,
    library_path: &Path,
    license_path: &Path,
) -> Result<(), ManagedSidecarError> {
    verify_file_sha256(executable_path, SIDECAR_EXECUTABLE_SHA256)?;
    verify_file_sha256(library_path, SIDECAR_LIBRARY_SHA256)?;
    verify_file_sha256(license_path, SIDECAR_LICENSE_SHA256)
}

fn verify_file_sha256(path: &Path, expected: &str) -> Result<(), ManagedSidecarError> {
    let actual = sha256_file(path).map_err(|_| ManagedSidecarError::AssetIntegrity)?;
    if actual == expected {
        Ok(())
    } else {
        Err(ManagedSidecarError::AssetIntegrity)
    }
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 16 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// 从已通过最终白名单的配置提取认证身份；不再注入或重新序列化配置。
pub(crate) fn api_secret_from_config(
    generated: &GeneratedConfig,
) -> Result<ApiSecret, ManagedSidecarError> {
    generated
        .validate_final()
        .map_err(|_| ManagedSidecarError::ConfigurationRejected)?;
    let mut document: Value = serde_json::from_slice(generated.as_bytes())
        .map_err(|_| ManagedSidecarError::ConfigurationRejected)?;
    let value = document["experimental"]["clash_api"]["secret"].take();
    let Value::String(value) = value else {
        return Err(ManagedSidecarError::ConfigurationRejected);
    };
    Ok(ApiSecret { value })
}

#[cfg(test)]
pub(crate) fn test_api_secret() -> ApiSecret {
    ApiSecret {
        value: "a".repeat(64),
    }
}

/// 已启动 child 的唯一所有权；不暴露 PID，也不会扫描或清理其他进程。
pub(crate) struct ManagedSidecar {
    child: Child,
    #[cfg(test)]
    dns_probe: Option<dns_probe::Capture>,
}

impl ManagedSidecar {
    /// 仅用于验证测试配置的实际 listener 归属，不进入产品 API。
    #[cfg(test)]
    pub(crate) fn test_process_id(&self) -> u32 {
        self.child.id()
    }

    /// 从同一 child 句柄读取创建时间，避免把采集上下文或可复用 PID 当作完整归属。
    #[cfg(all(test, windows))]
    pub(crate) fn test_creation_time_100ns(&self) -> Result<u64, ManagedSidecarError> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::{
            Foundation::{FILETIME, HANDLE},
            System::Threading::GetProcessTimes,
        };
        let mut created = FILETIME::default();
        let mut exited = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe {
            GetProcessTimes(
                HANDLE(self.child.as_raw_handle()),
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            )
        }
        .map_err(|_| ManagedSidecarError::Stop)?;
        Ok((u64::from(created.dwHighDateTime) << 32) | u64::from(created.dwLowDateTime))
    }

    /// 先返回 check 子进程归属，等待或超时失败不能让临时 Child 丢失。
    pub(crate) fn start_check(
        executable_path: &Path,
        config_path: &Path,
    ) -> Result<Self, ManagedSidecarError> {
        let child = managed_command(executable_path)
            .arg("check")
            .arg("-c")
            .arg(config_path)
            .spawn()
            .map_err(|_| ManagedSidecarError::ConfigurationCheck)?;
        Ok(Self {
            child,
            #[cfg(test)]
            dns_probe: None,
        })
    }

    /// check 最长 10s，超时后只有一次 kill/2s 退出确认；失败时 owner 仍持有本对象。
    pub(crate) fn wait_check(&mut self) -> Result<(), ManagedSidecarError> {
        match wait_for_exit(&mut self.child, CHECK_TIMEOUT) {
            Some(status) if status.success() => Ok(()),
            Some(_) => Err(ManagedSidecarError::ConfigurationCheck),
            None => {
                self.stop()?;
                Err(ManagedSidecarError::ConfigurationCheck)
            }
        }
    }

    /// 仅在 [`Self::wait_check`] 已成功后使用固定 `run -c` 启动。所有标准流均为 null，
    /// Windows 不创建控制台窗口。
    pub(crate) fn start_checked(
        executable_path: &Path,
        config_path: &Path,
    ) -> Result<Self, ManagedSidecarError> {
        let child = managed_command(executable_path)
            .arg("run")
            .arg("-c")
            .arg(config_path)
            .spawn()
            .map_err(|_| ManagedSidecarError::Spawn)?;
        Ok(Self {
            child,
            #[cfg(test)]
            dns_probe: None,
        })
    }

    /// 仅供已核定完整 DNS 测试元组及最终字节身份的 Windows Port 调用。
    #[cfg(all(test, windows))]
    pub(crate) fn start_checked_for_dns_probe(
        executable_path: &Path,
        config_path: &Path,
    ) -> Result<Self, ManagedSidecarError> {
        let spawned_at = Instant::now();
        let mut child = dns_probe_command(executable_path, config_path)
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| ManagedSidecarError::Spawn)?;
        let stderr = child.stderr.take().expect("piped stderr belongs to child");
        let capture = dns_probe::Capture::start(stderr, child.id(), spawned_at);
        // 即使读取线程创建失败也返回 owner，由固定失败状态驱动调用方停止 child。
        Ok(Self {
            child,
            dns_probe: Some(capture),
        })
    }

    #[cfg(test)]
    pub(crate) fn test_dns_probe_snapshot(&self) -> Option<TestDnsProbeSnapshot> {
        self.dns_probe.as_ref().map(dns_probe::Capture::snapshot)
    }

    /// 只检查本对象拥有的 child 是否仍在运行；不暴露 PID。
    pub(crate) fn is_running(&mut self) -> Result<bool, ManagedSidecarError> {
        self.child
            .try_wait()
            .map(|status| status.is_none())
            .map_err(|_| ManagedSidecarError::Stop)
    }

    /// 只终止这个结构体拥有的 child；调用方无需也不能提供 PID。
    pub(crate) fn stop(&mut self) -> Result<(), ManagedSidecarError> {
        let stopped = match self
            .child
            .try_wait()
            .map_err(|_| ManagedSidecarError::Stop)?
        {
            Some(_) => Ok(()),
            None => {
                self.child.kill().map_err(|_| ManagedSidecarError::Stop)?;
                wait_for_exit(&mut self.child, STOP_TIMEOUT)
                    .map(|_| ())
                    .ok_or(ManagedSidecarError::Stop)
            }
        };
        stopped?;
        #[cfg(test)]
        if let Some(capture) = self.dns_probe.as_mut() {
            capture.finish()?;
        }
        Ok(())
    }
}

impl Drop for ManagedSidecar {
    fn drop(&mut self) {
        #[cfg(test)]
        if self.dns_probe.is_some() {
            let _ = self.stop();
            if self.child.try_wait().ok().flatten().is_none() {
                let _ = self.child.kill();
            }
            // Drop 不能返回恢复 owner；取消只用于异常释放读侧，绝不伪造 EOF。
            if let Some(capture) = self.dns_probe.as_mut() {
                capture.cancel_and_join();
            }
            return;
        }
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
    }
}

fn wait_for_exit(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now().checked_add(timeout)?;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(PROCESS_POLL_INTERVAL),
            Ok(None) | Err(_) => return None,
        }
    }
}

/// 禁色是固定核心的CLI参数；仅已核定DNS测试run分支可用。
#[cfg(all(test, windows))]
fn dns_probe_command(executable_path: &Path, config_path: &Path) -> Command {
    let mut command = managed_command(executable_path);
    command
        .arg("run")
        .arg("--disable-color")
        .arg("-c")
        .arg(config_path);
    command
}

fn managed_command(executable_path: &Path) -> Command {
    let mut command = Command::new(executable_path);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

fn is_valid_secret(secret: &str) -> bool {
    secret.len() == SECRET_BYTES * 2
        && secret.bytes().all(|byte| {
            byte.is_ascii_digit() || (byte.is_ascii_lowercase() && byte.is_ascii_hexdigit())
        })
}

#[cfg(test)]
pub(crate) use dns_probe::{
    Event as TestDnsProbeEvent, Record as TestDnsProbeRecord, RecordType as TestDnsProbeRecordType,
    TestDnsProbeSnapshot, TestDnsProbeStatus,
};

/// DCR-013 的私有诊断；固定 revision 的日志语法不进入产品日志接口。
#[cfg(test)]
mod dns_probe {
    use super::*;
    use std::{
        net::IpAddr,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
    };

    const DOMAIN: &str = "veyra.disign.me";
    const OWNER: &str = "veyra.disign.me.";
    const BLOCK_BYTES: usize = 512;
    const LINE_BYTES: usize = 4096;
    const TOTAL_BYTES: usize = 65536;
    const MAX_RECORDS: usize = 128;
    const MAX_ADDRESSES: usize = 8;
    const POLL: Duration = Duration::from_millis(5);

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum TestDnsProbeFailure {
        Encoding,
        Truncated,
        LineLimit,
        TotalLimit,
        RecordLimit,
        AddressLimit,
        DnsError,
        UnknownFormat,
        Contradiction,
        ReadError,
        ReaderStart,
        ReaderPanicked,
        ReaderTimeout,
        Cancelled,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum TestDnsProbeStatus {
        Pending,
        Success,
        Failure(TestDnsProbeFailure),
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Event {
        Lookup,
        Exchanged,
        Cached,
        Optimistic,
        Refreshed,
        LookupSucceeded,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum RecordType {
        A,
        Aaaa,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct Record {
        pub event: Event,
        pub domain: &'static str,
        pub rr_type: Option<RecordType>,
        pub address: Option<IpAddr>,
        pub ttl: Option<u32>,
        pub rcode: Option<&'static str>,
        pub elapsed_ms: u64,
    }

    #[derive(Clone, Debug)]
    pub(crate) struct TestDnsProbeSnapshot {
        pub status: TestDnsProbeStatus,
        pub addresses: Vec<IpAddr>,
        pub records: Vec<Record>,
        pub pid: u32,
        pub spawned_at: Instant,
        pub received_bytes: usize,
        pub eof: bool,
        pub reader_joined: bool,
    }

    struct Parser {
        snapshot: TestDnsProbeSnapshot,
        line: Vec<u8>,
        query_context: Option<Option<u32>>,
        exchanged: bool,
        exchanged_addresses: Vec<IpAddr>,
        all_addresses: Vec<IpAddr>,
    }

    impl Parser {
        fn new(pid: u32, spawned_at: Instant) -> Self {
            Self {
                snapshot: TestDnsProbeSnapshot {
                    status: TestDnsProbeStatus::Pending,
                    addresses: Vec::new(),
                    records: Vec::new(),
                    pid,
                    spawned_at,
                    received_bytes: 0,
                    eof: false,
                    reader_joined: false,
                },
                line: Vec::with_capacity(LINE_BYTES),
                query_context: None,
                exchanged: false,
                exchanged_addresses: Vec::new(),
                all_addresses: Vec::new(),
            }
        }

        fn fail(&mut self, reason: TestDnsProbeFailure) {
            if !matches!(self.snapshot.status, TestDnsProbeStatus::Failure(_)) {
                self.snapshot.status = TestDnsProbeStatus::Failure(reason);
                self.snapshot.addresses.clear();
            }
            self.line.fill(0);
            self.line.clear();
        }

        fn feed(&mut self, bytes: &[u8]) {
            self.snapshot.received_bytes = self.snapshot.received_bytes.saturating_add(bytes.len());
            if self.snapshot.received_bytes > TOTAL_BYTES {
                self.fail(TestDnsProbeFailure::TotalLimit);
            }
            for &byte in bytes {
                if matches!(self.snapshot.status, TestDnsProbeStatus::Failure(_)) {
                    break;
                }
                if byte == b'\n' {
                    // 原始行只活在当前解析栈，不进入快照或错误。
                    let mut line =
                        std::mem::replace(&mut self.line, Vec::with_capacity(LINE_BYTES));
                    match std::str::from_utf8(&line) {
                        Ok(line) => {
                            if let Err(reason) = self.parse_line(line) {
                                self.fail(reason);
                            }
                        }
                        Err(_) => self.fail(TestDnsProbeFailure::Encoding),
                    }
                    line.fill(0);
                } else if self.line.len() == LINE_BYTES {
                    self.fail(TestDnsProbeFailure::LineLimit);
                } else {
                    self.line.push(byte);
                }
            }
        }

        fn eof(&mut self) {
            self.snapshot.eof = true;
            if !self.line.is_empty() {
                self.fail(TestDnsProbeFailure::Truncated);
            }
        }

        fn record(
            &mut self,
            event: Event,
            address: Option<IpAddr>,
            ttl: Option<u32>,
            rcode: Option<&'static str>,
        ) -> Result<(), TestDnsProbeFailure> {
            if self.snapshot.records.len() == MAX_RECORDS {
                return Err(TestDnsProbeFailure::RecordLimit);
            }
            if let Some(address) = address
                && !self.all_addresses.contains(&address)
            {
                if self.all_addresses.len() == MAX_ADDRESSES {
                    return Err(TestDnsProbeFailure::AddressLimit);
                }
                self.all_addresses.push(address);
            }
            self.snapshot.records.push(Record {
                event,
                domain: DOMAIN,
                rr_type: address.map(|ip| {
                    if ip.is_ipv4() {
                        RecordType::A
                    } else {
                        RecordType::Aaaa
                    }
                }),
                address,
                ttl,
                rcode,
                elapsed_ms: self
                    .snapshot
                    .spawned_at
                    .elapsed()
                    .as_millis()
                    .min(u64::MAX as u128) as u64,
            });
            Ok(())
        }

        // Formatter: LEVEL[0000] [可选 uint32 上下文ID/持续时间] dns: 消息。
        // DNS client_log/router 的固定 revision：0b8995879f29a9b98ee027bc17b75e101445b238。
        fn parse_line(&mut self, line: &str) -> Result<(), TestDnsProbeFailure> {
            let Some((level, context, message)) = formatted_dns_line(line) else {
                if line.contains("dns:") && exact_domain_in(line) {
                    return Err(TestDnsProbeFailure::UnknownFormat);
                }
                return Ok(());
            };
            if !exact_domain_in(message) {
                return Ok(());
            }
            if matches!(level, "ERROR" | "WARN" | "FATAL" | "PANIC") {
                return Err(TestDnsProbeFailure::DnsError);
            }
            let words: Vec<_> = message.split(' ').collect();
            if words == ["lookup", "domain", DOMAIN] && level == "DEBUG" {
                if self.query_context.is_some() && self.query_context != Some(context) {
                    return Err(TestDnsProbeFailure::Contradiction);
                }
                self.query_context = Some(context);
                return self.record(Event::Lookup, None, None, None);
            }
            if self.query_context.is_some() && self.query_context != Some(context) {
                return Err(TestDnsProbeFailure::Contradiction);
            }
            if let Some(addresses) = message.strip_prefix("lookup succeed for veyra.disign.me: ") {
                if level != "INFO" {
                    return Err(TestDnsProbeFailure::UnknownFormat);
                }
                let mut result = Vec::new();
                for address in addresses.split(' ') {
                    let address: IpAddr = address
                        .parse()
                        .map_err(|_| TestDnsProbeFailure::UnknownFormat)?;
                    if result.contains(&address) || result.len() == MAX_ADDRESSES {
                        return Err(TestDnsProbeFailure::Contradiction);
                    }
                    result.push(address);
                }
                if result.is_empty() {
                    return Err(TestDnsProbeFailure::Contradiction);
                }
                if self.query_context.is_none()
                    || !self.exchanged
                    || self.exchanged_addresses.is_empty()
                {
                    // 缺交换证据属于不可用，不能把缓存答案升级成成功或负向通过。
                    for &ip in &result {
                        self.record(Event::LookupSucceeded, Some(ip), None, None)?;
                    }
                    return Ok(());
                }
                if result
                    .iter()
                    .any(|ip| !self.exchanged_addresses.contains(ip))
                {
                    return Err(TestDnsProbeFailure::Contradiction);
                }
                result.sort();
                if !self.snapshot.addresses.is_empty() && self.snapshot.addresses != result {
                    return Err(TestDnsProbeFailure::Contradiction);
                }
                for &ip in &result {
                    self.record(Event::LookupSucceeded, Some(ip), None, None)?;
                }
                self.snapshot.addresses = result;
                self.snapshot.status = TestDnsProbeStatus::Success;
                return Ok(());
            }
            let event = match words.first().copied() {
                Some("exchanged") => Event::Exchanged,
                Some("cached") => Event::Cached,
                Some("optimistic") => Event::Optimistic,
                Some("refreshed") => Event::Refreshed,
                _ => return Err(TestDnsProbeFailure::UnknownFormat),
            };
            if words.get(1) == Some(&DOMAIN) && level == "DEBUG" {
                let expected = if event == Event::Optimistic { 3 } else { 4 };
                if words.len() != expected {
                    return Err(TestDnsProbeFailure::UnknownFormat);
                }
                if words[2] != "NOERROR" {
                    return Err(TestDnsProbeFailure::DnsError);
                }
                let ttl = words
                    .get(3)
                    .map(|ttl| ttl.parse::<u32>())
                    .transpose()
                    .map_err(|_| TestDnsProbeFailure::UnknownFormat)?;
                if event == Event::Exchanged && self.query_context.is_some() {
                    self.exchanged = true;
                }
                return self.record(event, None, ttl, Some("NOERROR"));
            }
            if level != "INFO"
                || words.len() != 7
                || words[2] != OWNER
                || words[4] != "IN"
                || words[1] != words[5]
            {
                return Err(TestDnsProbeFailure::UnknownFormat);
            }
            let ttl = words[3]
                .parse::<u32>()
                .map_err(|_| TestDnsProbeFailure::UnknownFormat)?;
            let address: IpAddr = words[6]
                .parse()
                .map_err(|_| TestDnsProbeFailure::UnknownFormat)?;
            if !matches!(
                (words[1], address),
                ("A", IpAddr::V4(_)) | ("AAAA", IpAddr::V6(_))
            ) {
                return Err(TestDnsProbeFailure::UnknownFormat);
            }
            if event == Event::Exchanged {
                if self.query_context.is_none() {
                    return self.record(event, Some(address), Some(ttl), None);
                }
                if !self.exchanged {
                    return Err(TestDnsProbeFailure::Contradiction);
                }
                self.record(event, Some(address), Some(ttl), None)?;
                if !self.exchanged_addresses.contains(&address) {
                    self.exchanged_addresses.push(address);
                }
                return Ok(());
            }
            self.record(event, Some(address), Some(ttl), None)
        }
    }

    fn exact_domain_in(text: &str) -> bool {
        text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_'))
            .any(|part| part == DOMAIN || part == OWNER)
    }

    fn formatted_dns_line(line: &str) -> Option<(&str, Option<u32>, &str)> {
        let (level, tail) = line.split_once('[')?;
        if !matches!(
            level,
            "DEBUG" | "INFO" | "WARN" | "ERROR" | "FATAL" | "PANIC"
        ) {
            return None;
        }
        let (seconds, mut tail) = tail.split_once("] ")?;
        if seconds.len() < 4
            || !seconds.bytes().all(|b| b.is_ascii_digit())
            || seconds.parse::<u32>().is_err()
        {
            return None;
        }
        let mut context = None;
        if let Some(rest) = tail.strip_prefix('[') {
            let (prefix, rest) = rest.split_once("] ")?;
            let (id, duration) = prefix.split_once(' ')?;
            if !id.bytes().all(|b| b.is_ascii_digit()) || !valid_duration(duration) {
                return None;
            }
            context = Some(id.parse().ok()?);
            tail = rest;
        }
        Some((level, context, tail.strip_prefix("dns: ")?))
    }

    fn valid_duration(duration: &str) -> bool {
        let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
        if let Some(ms) = duration.strip_suffix("ms") {
            return digits(ms);
        }
        if let Some(s) = duration.strip_suffix('s') {
            if let Some((minutes, seconds)) = s.split_once('m') {
                return digits(minutes) && digits(seconds);
            }
            if let Some((seconds, fraction)) = s.split_once('.') {
                return digits(seconds) && digits(fraction) && fraction.len() <= 2;
            }
        }
        false
    }

    pub(super) struct Capture {
        parser: Arc<Mutex<Parser>>,
        cancel: Arc<AtomicBool>,
        reader: Option<thread::JoinHandle<()>>,
    }

    impl Capture {
        #[cfg(windows)]
        pub(super) fn start<R: Read + std::os::windows::io::AsRawHandle + Send + 'static>(
            pipe: R,
            pid: u32,
            spawned_at: Instant,
        ) -> Self {
            let parser = Arc::new(Mutex::new(Parser::new(pid, spawned_at)));
            let cancel = Arc::new(AtomicBool::new(false));
            let reader_parser = Arc::clone(&parser);
            let reader_cancel = Arc::clone(&cancel);
            let reader = thread::Builder::new()
                .name("dns-probe-reader".into())
                .spawn(move || {
                    read_pipe(pipe, &reader_parser, &reader_cancel);
                });
            let reader = match reader {
                Ok(reader) => Some(reader),
                Err(_) => {
                    parser
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .fail(TestDnsProbeFailure::ReaderStart);
                    None
                }
            };
            Self {
                parser,
                cancel,
                reader,
            }
        }

        pub(super) fn snapshot(&self) -> TestDnsProbeSnapshot {
            let mut parser = self.parser.lock().unwrap_or_else(|p| p.into_inner());
            if self.reader.as_ref().is_some_and(|r| r.is_finished()) && !parser.snapshot.eof {
                parser.fail(TestDnsProbeFailure::ReaderPanicked);
            }
            parser.snapshot.clone()
        }

        pub(super) fn finish(&mut self) -> Result<(), ManagedSidecarError> {
            let deadline = Instant::now() + STOP_TIMEOUT;
            while self.reader.as_ref().is_some_and(|r| !r.is_finished()) {
                if Instant::now() >= deadline {
                    self.parser
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .fail(TestDnsProbeFailure::ReaderTimeout);
                    return Err(ManagedSidecarError::Stop);
                }
                thread::sleep(POLL);
            }
            self.join();
            let snapshot = self.snapshot();
            if snapshot.eof && snapshot.reader_joined {
                Ok(())
            } else {
                Err(ManagedSidecarError::Stop)
            }
        }

        fn join(&mut self) {
            if let Some(reader) = self.reader.take() {
                let result = reader.join();
                let mut parser = self.parser.lock().unwrap_or_else(|p| p.into_inner());
                parser.snapshot.reader_joined = true;
                if result.is_err() {
                    parser.fail(TestDnsProbeFailure::ReaderPanicked);
                }
            }
        }

        pub(super) fn cancel_and_join(&mut self) {
            if self.reader.as_ref().is_some_and(|r| !r.is_finished()) {
                self.cancel.store(true, Ordering::Release);
            }
            self.join();
        }
    }

    impl Drop for Capture {
        fn drop(&mut self) {
            self.cancel_and_join();
        }
    }

    #[cfg(windows)]
    fn read_pipe<R: Read + std::os::windows::io::AsRawHandle>(
        mut pipe: R,
        parser: &Mutex<Parser>,
        cancel: &AtomicBool,
    ) {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn PeekNamedPipe(
                pipe: *mut std::ffi::c_void,
                buffer: *mut std::ffi::c_void,
                buffer_size: u32,
                bytes_read: *mut u32,
                total_available: *mut u32,
                bytes_left: *mut u32,
            ) -> i32;
        }
        let mut buffer = [0_u8; BLOCK_BYTES];
        loop {
            if cancel.load(Ordering::Acquire) {
                parser
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .fail(TestDnsProbeFailure::Cancelled);
                return;
            }
            let mut available = 0;
            // reader 独占此读句柄；只查询可用量，随后最多读取已存在的512字节，避免无界阻塞。
            let peeked = unsafe {
                PeekNamedPipe(
                    pipe.as_raw_handle(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                    &mut available,
                    std::ptr::null_mut(),
                )
            };
            if peeked == 0 {
                let error = io::Error::last_os_error();
                let mut parser = parser.lock().unwrap_or_else(|p| p.into_inner());
                if error.raw_os_error() == Some(109) {
                    // ERROR_BROKEN_PIPE
                    parser.eof();
                } else {
                    parser.fail(TestDnsProbeFailure::ReadError);
                }
                return;
            }
            if available == 0 {
                thread::sleep(POLL);
                continue;
            }
            let count = (available as usize).min(buffer.len());
            match pipe.read(&mut buffer[..count]) {
                Ok(0) => {
                    parser.lock().unwrap_or_else(|p| p.into_inner()).eof();
                    return;
                }
                Ok(read) => {
                    parser
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .feed(&buffer[..read]);
                    buffer[..read].fill(0);
                }
                Err(_) => {
                    parser
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .fail(TestDnsProbeFailure::ReadError);
                    return;
                }
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        const QUERY: &str = "DEBUG[0000] [123 0ms] dns: lookup domain veyra.disign.me\n";
        const EXCHANGE: &str = "DEBUG[0000] [123 1ms] dns: exchanged veyra.disign.me NOERROR 1\n";
        const RR: &str =
            "INFO[0000] [123 1ms] dns: exchanged A veyra.disign.me. 1 IN A 198.20.0.255\n";
        const LOOKUP: &str =
            "INFO[0000] [123 1ms] dns: lookup succeed for veyra.disign.me: 198.20.0.255\n";

        fn parsed(parts: &[&str]) -> Parser {
            let mut parser = Parser::new(7, Instant::now());
            for part in parts {
                parser.feed(part.as_bytes());
            }
            parser
        }

        #[test]
        fn dns_probe_requires_same_query_exchange_rr_and_lookup() {
            let rr6 =
                "INFO[0000] [123 2ms] dns: exchanged AAAA veyra.disign.me. 1 IN AAAA fc00::fe\n";
            let lookup = "INFO[0000] [123 2ms] dns: lookup succeed for veyra.disign.me: 198.20.0.255 fc00::fe\n";
            let mut parser = parsed(&[QUERY, EXCHANGE, RR, EXCHANGE, rr6, lookup]);
            parser.eof();
            assert_eq!(parser.snapshot.status, TestDnsProbeStatus::Success);
            assert_eq!(
                parser.snapshot.addresses,
                vec![
                    "198.20.0.255".parse::<IpAddr>().unwrap(),
                    "fc00::fe".parse().unwrap()
                ]
            );
            assert!(parser.snapshot.eof);
            assert_eq!(parser.snapshot.pid, 7);
            assert_eq!(parser.snapshot.records[1].rcode, Some("NOERROR"));
            assert_eq!(parser.snapshot.records[2].ttl, Some(1));
            assert_eq!(parser.snapshot.records[2].rr_type, Some(RecordType::A));
            assert!(parser.snapshot.records.iter().all(|r| r.domain == DOMAIN));
        }

        #[test]
        fn dns_probe_missing_query_exchange_rr_or_lookup_stays_unavailable() {
            for parts in [
                vec![EXCHANGE, RR, LOOKUP],
                vec![QUERY, LOOKUP],
                vec![QUERY, EXCHANGE, LOOKUP],
                vec![QUERY, EXCHANGE, RR],
            ] {
                let parser = parsed(&parts);
                assert_eq!(parser.snapshot.status, TestDnsProbeStatus::Pending);
                assert!(parser.snapshot.addresses.is_empty());
            }
            for event in ["cached", "optimistic", "refreshed"] {
                let summary = if event == "optimistic" {
                    "DEBUG[0000] [123 1ms] dns: optimistic veyra.disign.me NOERROR\n".to_owned()
                } else {
                    EXCHANGE.replace("exchanged", event)
                };
                let rr = RR.replace("exchanged", event);
                let parser = parsed(&[QUERY, &summary, &rr, LOOKUP]);
                assert_eq!(parser.snapshot.status, TestDnsProbeStatus::Pending);
                assert!(parser.snapshot.addresses.is_empty());
            }
        }

        #[test]
        fn dns_probe_rejects_wrong_logger_domain_context_and_rr() {
            for modified in [
                [QUERY, EXCHANGE, RR, LOOKUP]
                    .concat()
                    .replace("dns:", "outbound/wireguard[test]:"),
                [QUERY, EXCHANGE, RR, LOOKUP]
                    .concat()
                    .replace(DOMAIN, "veyra.disign.me.evil"),
            ] {
                let parser = parsed(&[&modified]);
                assert_eq!(parser.snapshot.status, TestDnsProbeStatus::Pending);
            }
            for invalid_rr in [
                RR.replace(" IN A ", " CH A "),
                RR.replace(" IN A ", " IN AAAA "),
                RR.replace("198.20.0.255", "fc00::fe"),
                RR.replace("198.20.0.255", "999.20.0.255"),
                RR.replace(". 1 IN", ". -1 IN"),
                RR.replace("exchanged A ", "exchanged CNAME "),
                RR.replace("[123 ", "[124 "),
                RR.replace("[0000]", "[0]"),
                RR.replace("[123 1ms]", "[123 invalid]"),
            ] {
                let parser = parsed(&[QUERY, EXCHANGE, &invalid_rr, LOOKUP]);
                assert!(matches!(
                    parser.snapshot.status,
                    TestDnsProbeStatus::Failure(_)
                ));
                assert!(parser.snapshot.addresses.is_empty());
            }
        }

        #[test]
        fn dns_probe_rejects_extra_addresses_errors_and_conflicting_results() {
            let other = LOOKUP.replace("198.20.0.255", "198.20.0.254");
            let empty = LOOKUP.replace("198.20.0.255", "");
            for lookup in [&other, &empty] {
                let parser = parsed(&[QUERY, EXCHANGE, RR, lookup]);
                assert!(matches!(
                    parser.snapshot.status,
                    TestDnsProbeStatus::Failure(_)
                ));
            }
            for error in [
                "ERROR[0000] [123 1ms] dns: lookup failed for veyra.disign.me: private-token\n",
                "DEBUG[0000] [123 1ms] dns: exchanged veyra.disign.me NXDOMAIN 1\n",
                "DEBUG[0000] [123 1ms] dns: future operation veyra.disign.me\n",
            ] {
                let parser = parsed(&[QUERY, EXCHANGE, RR, LOOKUP, error]);
                assert!(matches!(
                    parser.snapshot.status,
                    TestDnsProbeStatus::Failure(_)
                ));
                assert!(!format!("{:?}", parser.snapshot).contains("private-token"));
            }
            let rr_other = RR.replace("198.20.0.255", "198.20.0.254");
            let parser = parsed(&[QUERY, EXCHANGE, RR, LOOKUP, &rr_other, &other]);
            assert_eq!(
                parser.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::Contradiction)
            );
        }

        #[test]
        fn dns_probe_ignores_unrelated_http_failure_and_accepts_uncolored_contextless_format() {
            let text = [QUERY, EXCHANGE, RR, LOOKUP]
                .concat()
                .replace("[123 0ms] ", "")
                .replace("[123 1ms] ", "");
            let parser = parsed(&[
                &text,
                "ERROR[0001] outbound/urltest: request failed for http://veyra.disign.me:18080/task009-dns-preflight\n",
            ]);
            assert_eq!(parser.snapshot.status, TestDnsProbeStatus::Success);
            for duration in ["0ms", "999ms", "1.1s", "2.01s", "1m1s"] {
                assert!(valid_duration(duration));
            }
        }

        #[test]
        fn dns_probe_bounds_encoding_lines_total_records_and_addresses() {
            let mut invalid = parsed(&[]);
            invalid.feed(&[0xff, b'\n']);
            assert_eq!(
                invalid.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::Encoding)
            );
            let mut truncated = parsed(&["INFO[0000] unfinished"]);
            truncated.eof();
            assert_eq!(
                truncated.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::Truncated)
            );
            let mut line = parsed(&[]);
            line.feed(&vec![b'x'; LINE_BYTES]);
            assert_eq!(line.snapshot.status, TestDnsProbeStatus::Pending);
            line.feed(b"x");
            assert_eq!(
                line.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::LineLimit)
            );
            let mut total = parsed(&[]);
            for _ in 0..TOTAL_BYTES / BLOCK_BYTES {
                total.feed(&[b'\n'; BLOCK_BYTES]);
            }
            assert_eq!(total.snapshot.status, TestDnsProbeStatus::Pending);
            total.feed(b"\n");
            assert_eq!(
                total.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::TotalLimit)
            );
            let mut records = parsed(&[]);
            for _ in 0..MAX_RECORDS {
                records.feed(QUERY.as_bytes());
            }
            assert_eq!(records.snapshot.records.len(), MAX_RECORDS);
            records.feed(QUERY.as_bytes());
            assert_eq!(
                records.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::RecordLimit)
            );
            let mut addresses = parsed(&[QUERY, EXCHANGE]);
            for index in 1..=MAX_ADDRESSES + 1 {
                addresses.feed(
                    RR.replace("198.20.0.255", &format!("198.20.0.{index}"))
                        .as_bytes(),
                );
            }
            assert_eq!(
                addresses.snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::AddressLimit)
            );
            assert_eq!(addresses.all_addresses.len(), MAX_ADDRESSES);
        }

        #[cfg(windows)]
        #[test]
        fn dns_probe_pipe_eof_joins_and_overflow_keeps_draining() {
            use std::io::Write;
            let (reader, mut writer) = std::io::pipe().unwrap();
            let mut capture = Capture::start(reader, 11, Instant::now());
            let writing = thread::spawn(move || {
                // 超过总量且不会触发单行上限；writer 必须全部完成，证明失败后仍排空。
                let bytes = vec![b'\n'; TOTAL_BYTES + BLOCK_BYTES * 16];
                writer.write_all(&bytes)
            });
            let result = capture.finish();
            let write_result = writing.join().unwrap();
            assert!(write_result.is_ok());
            assert!(result.is_ok());
            let snapshot = capture.snapshot();
            assert_eq!(
                snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::TotalLimit)
            );
            assert_eq!(snapshot.received_bytes, TOTAL_BYTES + BLOCK_BYTES * 16);
            assert!(snapshot.eof && snapshot.reader_joined);
        }

        #[cfg(windows)]
        #[test]
        fn dns_probe_pipe_zero_data_cancel_is_not_eof() {
            let (reader, writer) = std::io::pipe().unwrap();
            let mut capture = Capture::start(reader, 12, Instant::now());
            capture.cancel_and_join();
            let snapshot = capture.snapshot();
            assert_eq!(
                snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::Cancelled)
            );
            assert!(!snapshot.eof);
            assert!(snapshot.reader_joined);
            assert!(capture.finish().is_err());
            drop(writer);
        }

        #[cfg(windows)]
        #[test]
        fn dns_probe_pipe_timeout_retains_owner_until_eof() {
            let (reader, writer) = std::io::pipe().unwrap();
            let mut capture = Capture::start(reader, 15, Instant::now());
            assert!(capture.finish().is_err());
            assert!(capture.reader.is_some());
            assert_eq!(
                capture.snapshot().status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::ReaderTimeout)
            );
            assert!(!capture.snapshot().eof);
            drop(writer);
            capture.finish().unwrap();
            assert!(capture.snapshot().eof && capture.snapshot().reader_joined);
            assert_eq!(
                capture.snapshot().status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::ReaderTimeout)
            );
        }

        #[cfg(windows)]
        #[test]
        fn dns_probe_pipe_complete_summary_and_read_error_propagate() {
            use std::{
                io::Write,
                os::windows::io::{AsRawHandle, RawHandle},
            };
            let (reader, mut writer) = std::io::pipe().unwrap();
            let mut capture = Capture::start(reader, 13, Instant::now());
            writer
                .write_all([QUERY, EXCHANGE, RR, LOOKUP].concat().as_bytes())
                .unwrap();
            drop(writer);
            capture.finish().unwrap();
            assert_eq!(capture.snapshot().status, TestDnsProbeStatus::Success);
            assert!(capture.snapshot().eof && capture.snapshot().reader_joined);

            struct FailedRead(std::io::PipeReader);
            impl AsRawHandle for FailedRead {
                fn as_raw_handle(&self) -> RawHandle {
                    self.0.as_raw_handle()
                }
            }
            impl Read for FailedRead {
                fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                    Err(io::Error::other("private-token"))
                }
            }
            let (reader, mut writer) = std::io::pipe().unwrap();
            writer.write_all(b"x").unwrap();
            let mut capture = Capture::start(FailedRead(reader), 14, Instant::now());
            assert!(capture.finish().is_err());
            let snapshot = capture.snapshot();
            assert_eq!(
                snapshot.status,
                TestDnsProbeStatus::Failure(TestDnsProbeFailure::ReadError)
            );
            assert!(snapshot.reader_joined && !snapshot.eof);
            assert!(!format!("{snapshot:?}").contains("private-token"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn dns_probe_argv_disables_color_only_for_fixed_test_run() {
        let executable = Path::new("fixed-sing-box.exe");
        let config = Path::new("owned-private-config.json");
        let command = dns_probe_command(executable, config);
        assert_eq!(command.get_program(), executable.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["run", "--disable-color", "-c", "owned-private-config.json"].map(std::ffi::OsStr::new)
        );
        assert!(command.get_envs().next().is_none());
        assert!(command.get_current_dir().is_none());
        // 普通启动器不附加测试参数；产品run/check仍由各自原调用点组装。
        assert!(managed_command(executable).get_args().next().is_none());
    }

    #[test]
    fn generated_secret_is_fixed_length_lowercase_hex() {
        let secret = generate_api_secret().expect("system entropy");

        assert!(is_valid_secret(secret.as_str()));
        assert_eq!(secret.as_str().len(), 64);
    }

    #[test]
    fn rejects_non_compiler_json_without_echoing_credentials() {
        let config = GeneratedConfig::from_bytes(
            br#"{"outbounds":[],"route":{},"inbounds":[{"type":"tun"}],"secret":"fixture-secret"}"#
                .to_vec(),
        );
        let error = api_secret_from_config(&config)
            .err()
            .expect("reject raw config");
        assert_eq!(error, ManagedSidecarError::ConfigurationRejected);
        assert!(!error.to_string().contains("fixture-secret"));
    }

    #[test]
    fn file_hash_is_lowercase_sha256() {
        let path =
            std::env::temp_dir().join(format!("veyra-managed-sidecar-hash-{}", std::process::id()));
        std::fs::write(&path, b"veyra").expect("write fixture");

        let hash = sha256_file(&path).expect("hash fixture");
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            hash,
            "954d457a7a9d956481b90e6b30a4bc101ec6120d4e63866b761be906ea13c7d1"
        );
    }
}
