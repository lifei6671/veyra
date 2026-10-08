//! P0-05 固定配置传输：磁盘仍为 root-only，child 只接收匿名 pipe FD3。
use super::{PreExecStage, Result, pre_exec_error};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

// POSIX pipe 至少容纳 512 bytes 的原子写；本原型固定配置为 425 bytes。
// 超限明确失败，不能在尚无 reader 的 spawn 前等待 pipe 空间。
const MAX_CONFIG_BYTES: usize = 512;

pub(super) fn read_private_config(path: &Path, expected_uid: u32) -> Result<Vec<u8>> {
    // 不跟随 symlink；对实际打开的 inode 校验，而非把安装资产的 0755 规则用于配置。
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file() || meta.uid() != expected_uid || meta.mode() & 0o7777 != 0o600 {
        return Err(
            "config must be a regular file owned by the expected UID with mode 0600".into(),
        );
    }
    let mut bytes = Vec::new();
    file.take((MAX_CONFIG_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err("fixed config exceeds the 512-byte pipe budget".into());
    }
    Ok(bytes)
}

fn pipe_ends() -> std::io::Result<(OwnedFd, File)> {
    let mut fds = [-1; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    // 获得所有权后才做可失败的 fcntl；任何失败都由 RAII 关闭两端。
    let read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { File::from_raw_fd(fds[1]) };
    for fd in fds {
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    // 即使容量前提不成立，也只返回 WouldBlock，绝不卡住 helper。
    let flags = unsafe { libc::fcntl(write.as_raw_fd(), libc::F_GETFL) };
    if flags < 0
        || unsafe { libc::fcntl(write.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
    {
        return Err(std::io::Error::last_os_error());
    }
    Ok((read, write))
}

pub(super) fn anonymous_pipe(bytes: &[u8]) -> Result<OwnedFd> {
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err("fixed config exceeds the 512-byte pipe budget".into());
    }
    let (read, mut write) = pipe_ends()?;
    // write_all 处理 EINTR/partial write；非阻塞写失败不 spawn、不留下端点。
    write.write_all(bytes)?;
    // spawn 前关闭唯一 writer：child 读取完配置后立刻得到 EOF，无后台 writer 生命周期。
    drop(write);
    Ok(read)
}

pub(super) unsafe fn inherit_fd3(read_fd: RawFd) -> std::io::Result<()> {
    // pre_exec 仅使用 async-signal-safe syscall 和既有固定整数错误编码。
    if read_fd != 3 && unsafe { libc::dup2(read_fd, 3) } < 0 {
        return Err(pre_exec_error(PreExecStage::Dup2, unsafe {
            *libc::__error()
        }));
    }
    // read_fd == 3 时 dup2 不会清 CLOEXEC，也必须显式清除。
    if unsafe { libc::fcntl(3, libc::F_SETFD, 0) } < 0 {
        return Err(pre_exec_error(PreExecStage::Fcntl, unsafe {
            *libc::__error()
        }));
    }
    // 原 read_fd（若非 3）仍为 CLOEXEC，不依赖 Command 自动关闭未知 fd。
    Ok(())
}

pub(super) fn evidence(bytes: &[u8]) -> Value {
    // 只输出长度、摘要和代码契约，不输出 JSON 或 secret。
    json!({"config_transport":"anonymous_pipe_fd3", "config_bytes":bytes.len(),
        "config_sha256":format!("{:x}", Sha256::digest(bytes)), "disk_config_mode":"0600",
        "fd3_contract":{"fd":3,"survives_exec":true,"helper_ends_cloexec":true,
            "writer_closed_before_spawn":true,"non_fd3_read_end_cloexec":true}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;
    use std::process::{Child, Command, Output, Stdio};
    use std::time::{Duration, Instant};

    fn output(mut child: Child) -> Output {
        let deadline = Instant::now() + Duration::from_secs(3);
        while child.try_wait().unwrap().is_none() {
            if Instant::now() >= deadline {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("FD3 child exceeded deadline (possible writer leak)");
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        child.wait_with_output().unwrap()
    }

    fn cat(read: OwnedFd) -> Output {
        let fd = read.as_raw_fd();
        let mut command = Command::new("/bin/cat");
        command
            .args(["/dev/fd/3"])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            command.pre_exec(move || inherit_fd3(fd));
        }
        let child = command.spawn().unwrap();
        drop(read);
        output(child)
    }

    // 保护核心修复：实际 exec/cat 经 /dev/fd/3 读取完整字节并得到 EOF。
    #[test]
    fn pipe_fd3_reopens_reads_full_budget_and_reaches_eof() {
        let bytes = vec![b'x'; MAX_CONFIG_BYTES];
        let result = cat(anonymous_pipe(&bytes).unwrap());
        assert!(result.status.success());
        assert_eq!(result.stdout, bytes);
        assert!(result.stderr.is_empty());
    }

    // 保护 helper fd 不随 exec 泄漏，以及 FD3 已等于原 read fd 时仍清 CLOEXEC。
    #[test]
    fn helper_ends_are_cloexec_and_fd3_same_fd_survives_exec() {
        let (read, mut write) = pipe_ends().unwrap();
        let (r, w) = (read.as_raw_fd(), write.as_raw_fd());
        for fd in [r, w] {
            assert_eq!(
                unsafe { libc::fcntl(fd, libc::F_GETFD) } & libc::FD_CLOEXEC,
                libc::FD_CLOEXEC
            );
        }
        write.write_all(b"pipe-contract").unwrap();
        // 仅测试脚本/端点号在 argv；配置字节始终只通过 pipe。
        let script = "import os,sys,errno\nfor fd in map(int,sys.argv[1:]):\n if fd != 3:\n  try: os.fstat(fd)\n  except OSError as e: assert e.errno == errno.EBADF\n  else: raise AssertionError('helper fd survived exec')\nassert os.read(3,512) == b'pipe-contract'\nassert os.read(3,1) == b''\n";
        let mut command = Command::new("/usr/bin/python3");
        command
            .args(["-c", script, &r.to_string(), &w.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        unsafe {
            command.pre_exec(move || {
                inherit_fd3(r)?;
                // 强制恢复 CLOEXEC，再走 read_fd == 3 分支；不更改父进程 fd3。
                if libc::fcntl(3, libc::F_SETFD, libc::FD_CLOEXEC) < 0 {
                    return Err(pre_exec_error(PreExecStage::Fcntl, *libc::__error()));
                }
                inherit_fd3(3)
            });
        }
        let child = command.spawn().unwrap();
        drop((read, write));
        let result = output(child);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }

    // 保护容量边界：超限在 spawn 前失败；完整写处理短写及 EINTR，不用大 payload 堵 pipe。
    #[test]
    fn bounded_payload_and_partial_interrupted_writes() {
        struct ShortWriter {
            bytes: Vec<u8>,
            interrupted: bool,
        }
        impl Write for ShortWriter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                if !self.interrupted {
                    self.interrupted = true;
                    return Err(std::io::ErrorKind::Interrupted.into());
                }
                let count = bytes.len().min(7);
                self.bytes.extend_from_slice(&bytes[..count]);
                Ok(count)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let bytes = vec![b'p'; 97];
        let mut writer = ShortWriter {
            bytes: Vec::new(),
            interrupted: false,
        };
        writer.write_all(&bytes).unwrap();
        assert_eq!(writer.bytes, bytes);
        assert!(anonymous_pipe(&vec![0; MAX_CONFIG_BYTES + 1]).is_err());
    }

    // 保护磁盘权限/内容/摘要一致性：测试 owner 为当前普通用户，生产调用固定要求 UID 0。
    #[test]
    fn private_disk_bytes_match_pipe_and_safe_evidence() {
        let temporary =
            std::env::temp_dir().join(format!("veyra-p005-fd3-disk-{}", std::process::id()));
        fs::create_dir(&temporary).unwrap();
        let path = temporary.join("config.json");
        let secret = "0123456789abcdef".repeat(4);
        let bytes = super::super::system_proxy_config(&secret)
            .to_string()
            .into_bytes();
        assert_eq!(bytes.len(), 425);
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(&bytes).unwrap();
        drop(file);
        let uid = unsafe { libc::geteuid() };
        let disk = read_private_config(&path, uid).unwrap();
        let result = cat(anonymous_pipe(&disk).unwrap());
        assert!(result.status.success());
        assert_eq!(result.stdout, bytes);
        assert_eq!(evidence(&disk), evidence(&result.stdout));
        let report = evidence(&disk);
        assert_eq!(report["disk_config_mode"], "0600");
        assert_eq!(report["config_bytes"], bytes.len());
        assert_eq!(
            report["config_sha256"],
            format!("{:x}", Sha256::digest(&bytes))
        );
        assert!(!report.to_string().contains(&secret));
        assert!(read_private_config(&path, uid + 1).is_err());
        for mode in [0o000, 0o400, 0o640, 0o644, 0o1600] {
            fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
            assert!(read_private_config(&path, uid).is_err());
        }
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
        let link = temporary.join("link");
        std::os::unix::fs::symlink(&path, &link).unwrap();
        assert!(read_private_config(&link, uid).is_err());
        assert!(read_private_config(&temporary, uid).is_err());
        fs::write(&path, vec![0; MAX_CONFIG_BYTES + 1]).unwrap();
        assert!(read_private_config(&path, uid).is_err());
        fs::remove_dir_all(temporary).unwrap();
    }

    // 保护已确认的 macOS vnode reopen 根因；只 chmod 自有 tmp，不模拟 root/降权。
    #[cfg(target_os = "macos")]
    #[test]
    fn inherited_regular_fd_reopen_denied_after_chmod_zero() {
        assert_ne!(
            unsafe { libc::geteuid() },
            0,
            "requires an unprivileged test runner"
        );
        let path =
            std::env::temp_dir().join(format!("veyra-p005-fd3-vnode-{}", std::process::id()));
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .unwrap();
        file.write_all(b"vnode permission repro").unwrap();
        // 单独重新 open 将 offset 归零；File 默认 CLOEXEC，child 仅继承 FD3。
        let mut inherited = File::open(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();
        let mut direct = Vec::new();
        inherited.read_to_end(&mut direct).unwrap();
        assert_eq!(direct, b"vnode permission repro");
        let result = cat(inherited.into());
        fs::remove_file(path).unwrap();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("Permission denied"));
    }

    // 保护真实 fork/exec 失败通道：不绕过错误，父侧仍能回收 read end/读到 EOF。
    #[test]
    fn pre_exec_failure_preserves_mapping_and_parent_pipe_ownership() {
        let read = anonymous_pipe(b"spawn-failure").unwrap();
        let mut command = Command::new("/bin/cat");
        unsafe {
            command.pre_exec(|| inherit_fd3(-1));
        }
        let error = command.spawn().unwrap_err();
        let (stage, errno) =
            super::super::decode_pre_exec_error(error.raw_os_error().unwrap()).unwrap();
        assert_eq!(stage.name(), "dup2");
        assert_eq!(errno, libc::EBADF);
        let mut bytes = Vec::new();
        File::from(read).read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"spawn-failure");
    }

    // 保护新增 C 诊断：实际无特权 child 退出后，双流截断加 config 元数据仍满足 IPC 预算。
    #[test]
    fn transport_evidence_with_readiness_failure_fits_frame_budget() {
        let secret = "0123456789abcdef".repeat(4);
        let bytes = super::super::system_proxy_config(&secret)
            .to_string()
            .into_bytes();
        let read = anonymous_pipe(&bytes).unwrap();
        let fd = read.as_raw_fd();
        let mut command = Command::new("/usr/bin/python3");
        // 不输出配置，不解析端口，不触发 controller 网络鉴权。
        command.args(["-c", "import os\nassert open('/dev/fd/3','rb').read()\nos.write(1,b'x'*2048)\nos.write(2,b'y'*2048)\nos._exit(1)\n"])
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
        unsafe {
            command.pre_exec(move || inherit_fd3(fd));
        }
        let mut child = command.spawn().unwrap();
        drop(read);
        // 固定最大长度 fixture 身份只用于序列化预算，不冒充实际 proc_pidinfo。
        let identity = json!({"pid":2147483647,"uid":4294967295u32,"ruid":4294967295u32,
            "gid":4294967295u32,"rgid":4294967295u32,"pgid":2147483647,
            "start_sec":18446744073709551615u64,"start_usec":999999});
        let mut error = super::super::readiness::wait(&mut child, &identity, &secret).unwrap_err();
        child.wait().unwrap();
        let readiness = error
            .downcast_mut::<super::super::readiness::ReadinessError>()
            .unwrap();
        readiness.diagnostics["config_input"] = evidence(&bytes);
        assert_eq!(readiness.diagnostics["outcome"], "child_exited");
        assert_eq!(readiness.diagnostics["exit_status"]["code"], 1);
        for stream in ["stdout", "stderr"] {
            assert_eq!(readiness.diagnostics[stream]["eof"], true);
            assert_eq!(readiness.diagnostics[stream]["truncated"], true);
        }
        assert!(readiness.diagnostics.to_string().len() + 1024 < super::super::MAX);
        let response = json!({"ok":false,"error":error.to_string(),
            "readiness":error.downcast_ref::<super::super::readiness::ReadinessError>().unwrap().diagnostics});
        assert!(response.to_string().len() < super::super::MAX);
        assert!(!response.to_string().contains(&secret));
    }
}
