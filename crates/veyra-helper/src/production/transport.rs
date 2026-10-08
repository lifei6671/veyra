//! macOS 系统凭据、受保护端点和有总 deadline 的帧传输；不依赖 prototype。
use super::*;
use std::{
    fs,
    io::{Read, Write},
    mem,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{
            fs::{FileTypeExt, MetadataExt},
            net::UnixStream,
        },
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub const ROOT: &str = "/Library/Application Support/Veyra/Helper";
pub const SOCKET: &str = "/Library/Application Support/Veyra/Helper/control.sock";
pub const KERNEL: &str = "/Library/Application Support/Veyra/Helper/veyra-sing-box";
const IO_BUDGET: Duration = Duration::from_secs(2);

/// 构造器不公开：业务调用方不能伪造 OS peer；PID 重用由启动时间区分。
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Peer {
    pub uid: u32,
    pub gid: u32,
    pub pid: i32,
    pub start: (u64, u64),
}
pub(super) fn peer(stream: &UnixStream) -> Result<Peer, Error> {
    let (mut uid, mut gid, mut pid) = (0, 0, 0i32);
    let mut size = mem::size_of_val(&pid) as libc::socklen_t;
    // getpeereid + LOCAL_PEERPID 均由已连接 socket 的内核身份提供。
    if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0
        || unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                0,
                2,
                (&mut pid as *mut i32).cast(),
                &mut size,
            )
        } != 0
        || size as usize != mem::size_of_val(&pid)
        || pid <= 0
    {
        return Err(Error::Unauthorized);
    }
    let mut info: libc::proc_bsdinfo = unsafe { mem::zeroed() };
    let wanted = mem::size_of_val(&info) as i32;
    if unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            wanted,
        )
    } != wanted
        || info.pbi_pid != pid as u32
        || info.pbi_uid != uid
        || info.pbi_ruid != uid
    {
        return Err(Error::Unauthorized);
    }
    Ok(Peer {
        uid,
        gid,
        pid,
        start: (info.pbi_start_tvsec, info.pbi_start_tvusec),
    })
}

/// 每一级目录都不可被业务用户替换；不跟随符号链接。
pub(super) fn protected_directories(path: &Path) -> Result<(), Error> {
    for component in path.ancestors() {
        let m = fs::symlink_metadata(component).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::NotInstalled
            } else {
                Error::UnsafeEndpoint
            }
        })?;
        if !m.is_dir() || m.file_type().is_symlink() || m.uid() != 0 || m.mode() & 0o022 != 0 {
            return Err(Error::UnsafeEndpoint);
        }
    }
    Ok(())
}
pub(super) fn socket_metadata(
    path: &Path,
    uid: u32,
    gid: u32,
    mode: u32,
) -> Result<(u64, u64), Error> {
    let m = fs::symlink_metadata(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            Error::NotInstalled
        } else {
            Error::UnsafeEndpoint
        }
    })?;
    if !m.file_type().is_socket() || m.uid() != uid || m.gid() != gid || m.mode() & 0o7777 != mode {
        return Err(Error::UnsafeEndpoint);
    }
    Ok((m.dev(), m.ino()))
}

pub(super) fn transfer(
    stream: &mut UnixStream,
    bytes: &mut [u8],
    writing: bool,
    deadline: Instant,
) -> Result<(), Error> {
    let mut position = 0;
    while position < bytes.len() {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(Error::Timeout)?;
        let mut poll = libc::pollfd {
            fd: stream.as_raw_fd(),
            events: if writing { libc::POLLOUT } else { libc::POLLIN },
            revents: 0,
        };
        let result = unsafe {
            libc::poll(
                &mut poll,
                1,
                remaining.as_millis().clamp(1, i32::MAX as u128) as i32,
            )
        };
        if result == 0 {
            return Err(Error::Timeout);
        }
        if result < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(Error::Unavailable);
        }
        // HUP 可能与已缓冲数据同时到达，先 drain；不重复设置 SO_RCVTIMEO。
        let result = if writing {
            stream.write(&bytes[position..])
        } else {
            stream.read(&mut bytes[position..])
        };
        match result {
            Ok(0) => return Err(Error::InvalidRequest),
            Ok(n) => position += n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                ) => {}
            Err(_) => return Err(Error::Unavailable),
        }
    }
    Ok(())
}
fn read_bytes(stream: &mut UnixStream) -> Result<(u8, Vec<u8>), Error> {
    stream
        .set_nonblocking(true)
        .map_err(|_| Error::Unavailable)?;
    let deadline = Instant::now() + IO_BUDGET;
    let mut header = [0; 5];
    transfer(stream, &mut header, false, deadline)?;
    let limit = match header[0] {
        1 => CONTROL_BYTES,
        2 => CONFIG_BYTES + CONTROL_BYTES,
        3 => RESOURCE_BYTES + CONFIG_BYTES + CONTROL_BYTES,
        _ => return Err(Error::InvalidRequest),
    };
    let size = u32::from_be_bytes(header[1..].try_into().expect("four length bytes")) as usize;
    if size == 0 || size > limit {
        return Err(Error::TooLarge);
    }
    let mut body = vec![0; size];
    transfer(stream, &mut body, false, deadline)?;
    Ok((header[0], body))
}
fn write_bytes(stream: &mut UnixStream, kind: u8, mut bytes: Vec<u8>) -> Result<(), Error> {
    stream
        .set_nonblocking(true)
        .map_err(|_| Error::Unavailable)?;
    let deadline = Instant::now() + IO_BUDGET;
    let mut header = [kind, 0, 0, 0, 0];
    header[1..].copy_from_slice(&(bytes.len() as u32).to_be_bytes());
    transfer(stream, &mut header, true, deadline)?;
    transfer(stream, &mut bytes, true, deadline)
}
pub(super) fn read_request(stream: &mut UnixStream) -> Result<Request, Error> {
    let (kind, bytes) = read_bytes(stream)?;
    let value: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRequest)?;
    let request: Request =
        serde_json::from_value(value.clone()).map_err(|_| Error::InvalidRequest)?;
    // 既有领域 DTO 有兼容字段默认值；IPC 只接受完整规范形态，递归拒绝被 serde 忽略的字段。
    if serde_json::to_value(&request).map_err(|_| Error::InvalidRequest)? != value
        || kind
            != if matches!(&request.command, Command::Handoff { action } if matches!(**action,HandoffAction::Prepare{..}))
            {
                3
            } else if request.command.configuration().is_some() {
                2
            } else {
                1
            }
    {
        return Err(Error::InvalidRequest);
    }
    Ok(request)
}
pub(super) fn write_response(stream: &mut UnixStream, response: &Response) -> Result<(), Error> {
    let bytes = serde_json::to_vec(response).map_err(|_| Error::InvalidRequest)?;
    let kind = if matches!(response, Response::Handoff(Ok(reply)) if reply.bundle.is_some()) {
        3
    } else {
        1
    };
    let limit = if kind == 3 {
        RESOURCE_BYTES + CONTROL_BYTES
    } else {
        CONTROL_BYTES
    };
    if bytes.len() > limit {
        return Err(Error::TooLarge);
    }
    write_bytes(stream, kind, bytes)
}

#[derive(Clone)]
pub struct Client {
    path: PathBuf,
    #[cfg(test)]
    isolated: bool,
}
impl Default for Client {
    fn default() -> Self {
        Self {
            path: SOCKET.into(),
            #[cfg(test)]
            isolated: false,
        }
    }
}
impl Client {
    #[cfg(test)]
    pub(super) fn isolated(path: PathBuf) -> Self {
        Self {
            path,
            isolated: true,
        }
    }
    /// 退出只停止同票据Local owner的实例；断线后查询同request，绝不重发或清持久fence。
    pub fn stop_bootstrap(&self, ticket: &BootstrapTicket) -> Result<(), Error> {
        match self.request(Command::Bootstrap {
            action: Box::new(BootstrapAction::Query {
                ticket: ticket.clone(),
            }),
        })? {
            Response::Bootstrap(Ok(reply))
                if reply.ticket == *ticket && reply.phase == BootstrapPhase::Committed => {}
            Response::Bootstrap(Err(e)) | Response::Rejected(e) => return Err(e),
            _ => return Err(Error::HandoffRequired),
        }
        stop_current(|command| self.request(command))
    }
    pub fn stop_released(
        &self,
        ticket: &veyra_core::application::runtime_recovery::TransferTicket,
    ) -> Result<(), Error> {
        stop_released(ticket, |command| self.request(command))
    }

    pub fn request(&self, command: Command) -> Result<Response, Error> {
        let parent = self.path.parent().ok_or(Error::UnsafeEndpoint)?;
        #[cfg(test)]
        let isolated = self.isolated;
        #[cfg(not(test))]
        let isolated = false;
        if !isolated {
            protected_directories(parent)?;
        }
        let uid = if isolated {
            unsafe { libc::geteuid() }
        } else {
            0
        };
        // 固定安装时 socket root:业务 GID 0660，服务端仍要求精确 UID。
        let gid = unsafe { libc::getegid() };
        let inode = socket_metadata(&self.path, uid, gid, 0o660)?;
        let mut stream = connect_bounded(&self.path)?;
        if peer(&stream)?.uid != uid || socket_metadata(&self.path, uid, gid, 0o660)? != inode {
            return Err(Error::Unauthorized);
        }
        let hello = Request {
            protocol: PROTOCOL,
            command: Command::Hello,
        };
        write_bytes(
            &mut stream,
            1,
            serde_json::to_vec(&hello).map_err(|_| Error::InvalidRequest)?,
        )?;
        let (kind, bytes) = read_bytes(&mut stream)?;
        if kind != 1 || bytes.len() > CONTROL_BYTES {
            return Err(Error::InvalidRequest);
        }
        let response: Response =
            serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRequest)?;
        match &response {
            Response::Hello(c) if c.protocol == PROTOCOL => {}
            Response::Rejected(e) => return Err(*e),
            _ => return Err(Error::IncompatibleVersion),
        }
        if matches!(&command, Command::Handoff { action } if matches!(**action, HandoffAction::Preflight { .. }))
            || matches!(&command, Command::Bootstrap { action } if matches!(**action, BootstrapAction::Preflight { .. } | BootstrapAction::RebindPreflight { .. }))
        {
            let Response::Hello(c) = &response else {
                return Err(Error::IncompatibleVersion);
            };
            c.handoff?;
            if c.host_version != env!("CARGO_PKG_VERSION")
                || c.required_kernel_version
                    != veyra_core::application::runtime_recovery::KERNEL_VERSION
                || c.kernel_version.as_deref()
                    != Some(veyra_core::application::runtime_recovery::KERNEL_VERSION)
                || c.resource_bytes != RESOURCE_BYTES
            {
                return Err(Error::IncompatibleVersion);
            }
        }
        if matches!(command, Command::Hello) {
            return Ok(response);
        }
        let handoff = matches!(command, Command::Handoff { .. });
        let kind = if matches!(&command,Command::Handoff{action} if matches!(**action,HandoffAction::Prepare{..}))
        {
            3
        } else if command.configuration().is_some() {
            2
        } else {
            1
        };
        let limit = if kind == 3 {
            RESOURCE_BYTES + CONFIG_BYTES + CONTROL_BYTES
        } else if kind == 2 {
            CONFIG_BYTES + CONTROL_BYTES
        } else {
            CONTROL_BYTES
        };
        let request = Request {
            protocol: PROTOCOL,
            command,
        };
        let bytes = serde_json::to_vec(&request).map_err(|_| Error::InvalidRequest)?;
        if bytes.len() > limit {
            return Err(Error::TooLarge);
        }
        write_bytes(&mut stream, kind, bytes)?;
        let (kind, bytes) = read_bytes(&mut stream)?;
        if !((kind == 1 && bytes.len() <= CONTROL_BYTES)
            || (handoff && kind == 3 && bytes.len() <= RESOURCE_BYTES + CONTROL_BYTES))
        {
            return Err(Error::InvalidRequest);
        }
        serde_json::from_slice(&bytes).map_err(|_| Error::InvalidRequest)
    }
}

/// std::UnixStream::connect 没有 timeout；队列拥塞时也必须服从固定 Host 预算。
pub(super) fn connect_bounded(path: &Path) -> Result<UnixStream, Error> {
    use std::os::unix::ffi::OsStrExt;
    let bytes = path.as_os_str().as_bytes();
    let mut address: libc::sockaddr_un = unsafe { mem::zeroed() };
    if bytes.len() >= address.sun_path.len() || bytes.contains(&0) {
        return Err(Error::UnsafeEndpoint);
    }
    address.sun_family = libc::AF_UNIX as libc::sa_family_t;
    address.sun_len = mem::size_of_val(&address) as u8;
    for (target, source) in address.sun_path.iter_mut().zip(bytes) {
        *target = *source as libc::c_char;
    }
    let fd = unsafe { libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return Err(Error::Unavailable);
    }
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
        return Err(Error::Unavailable);
    }
    stream
        .set_nonblocking(true)
        .map_err(|_| Error::Unavailable)?;
    let result = unsafe {
        libc::connect(
            fd,
            (&address as *const libc::sockaddr_un).cast(),
            mem::size_of_val(&address) as libc::socklen_t,
        )
    };
    if result == 0 {
        return Ok(stream);
    }
    if std::io::Error::last_os_error().raw_os_error() != Some(libc::EINPROGRESS) {
        return Err(Error::Unavailable);
    }
    let deadline = Instant::now() + IO_BUDGET;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or(Error::Timeout)?;
        let mut poll = libc::pollfd {
            fd,
            events: libc::POLLOUT,
            revents: 0,
        };
        let ready = unsafe { libc::poll(&mut poll, 1, remaining.as_millis().max(1) as i32) };
        if ready == 0 {
            return Err(Error::Timeout);
        }
        if ready < 0 {
            if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            return Err(Error::Unavailable);
        }
        let mut error = 0i32;
        let mut size = mem::size_of_val(&error) as libc::socklen_t;
        if unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_ERROR,
                (&mut error as *mut i32).cast(),
                &mut size,
            )
        } != 0
            || error != 0
        {
            return Err(Error::Unavailable);
        }
        return Ok(stream);
    }
}

/// 固定退出预算；每次RPC仍受transport各阶段2秒限制，超时/UNKNOWN都保留恢复材料。
pub(super) fn stop_released(
    ticket: &veyra_core::application::runtime_recovery::TransferTicket,
    mut request: impl FnMut(Command) -> Result<Response, Error>,
) -> Result<(), Error> {
    use veyra_core::application::runtime_recovery::OwnerTransfer;
    match request(Command::Handoff {
        action: Box::new(HandoffAction::Query),
    })? {
        Response::Handoff(Ok(reply))
            if reply.owner
                == Some(OwnerTransfer::Local {
                    ticket: ticket.clone(),
                }) => {}
        Response::Rejected(e) | Response::Handoff(Err(e)) => return Err(e),
        _ => return Err(Error::HandoffRequired),
    }
    stop_current(request)
}
/// 已核验owner后的共同Stop/Operation/Status闭环，不因UNKNOWN重新发送Stop。
fn stop_current(mut request: impl FnMut(Command) -> Result<Response, Error>) -> Result<(), Error> {
    use sha2::{Digest, Sha256};
    let Response::Status(before) = request(Command::Status)? else {
        return Err(Error::InvalidRequest);
    };
    let Some(instance) = before.instance else {
        return if before.applied.is_none() && !before.recovery_required {
            Ok(())
        } else {
            Err(Error::HandoffRequired)
        };
    };
    // 只生成退出操作ID；不传递任意PID/路径/超时，也不更改任何业务版本。
    let entropy = veyra_core::domain::StateEpoch::fresh().map_err(|_| Error::Unavailable)?;
    let hash = Sha256::digest(format!("quit:{entropy:?}").as_bytes());
    let id = u64::from_be_bytes(hash[..8].try_into().expect("eight bytes"));
    let deadline = Instant::now() + Duration::from_secs(40);
    let mut reply = request(Command::Stop {
        request_id: id,
        instance,
    });
    loop {
        match reply {
            Ok(Response::Operation(Operation::Completed(result))) => {
                // 结果不是当前事实；必须再查真实Status，停止失败或未解决pending仍不可报Stopped。
                let observed = request(Command::Status)?;
                result?;
                return match observed {
                    Response::Status(s)
                        if s.instance.is_none() && s.applied.is_none() && !s.recovery_required =>
                    {
                        Ok(())
                    }
                    _ => Err(Error::HandoffRequired),
                };
            }
            Ok(Response::Operation(Operation::Unknown)) => {
                let _ = request(Command::Status)?;
                return Err(Error::HandoffRequired);
            }
            Ok(Response::Rejected(e)) => return Err(e),
            Ok(Response::Operation(Operation::Inflight))
            | Err(Error::Timeout | Error::Unavailable) => {}
            Err(e) => return Err(e),
            _ => return Err(Error::InvalidRequest),
        }
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        std::thread::sleep(Duration::from_millis(50));
        reply = request(Command::Operation { request_id: id });
    }
}

#[cfg(test)]
mod quit_tests {
    use super::*;
    #[test]
    fn unknown_or_failed_quit_never_claims_cleanup_or_replays_stop() {
        // Mock故障保护：UNKNOWN即使Status为空也不能证明本请求清理成功；失败保留恢复材料。
        use veyra_core::{application::runtime_recovery::*, domain::AppState};
        let ticket = TransferTicket {
            id: veyra_core::domain::StateEpoch::fresh().unwrap(),
            version: AppState::empty().version(),
            digest: "0".repeat(64),
        };
        for outcome in [
            Operation::Unknown,
            Operation::Completed(Err(Error::Failed)),
            Operation::Completed(Ok(Status::default())),
        ] {
            let mut stop_count = 0;
            let mut status_count = 0;
            let reply = stop_released(&ticket, |command| {
                Ok(match command {
                    Command::Handoff { .. } => Response::Handoff(Ok(HandoffReply {
                        owner: Some(OwnerTransfer::Local {
                            ticket: ticket.clone(),
                        }),
                        bundle: None,
                    })),
                    Command::Status => {
                        status_count += 1;
                        Response::Status(if status_count == 1 {
                            Status {
                                instance: Some(1),
                                ..Status::default()
                            }
                        } else {
                            Status {
                                recovery_required: true,
                                ..Status::default()
                            }
                        })
                    }
                    Command::Stop { .. } => {
                        stop_count += 1;
                        Response::Operation(outcome.clone())
                    }
                    _ => panic!("must not replay or submit any other mutation"),
                })
            });
            assert!(reply.is_err());
            assert_eq!(stop_count, 1);
            assert_eq!(status_count, 2);
        }
    }
}
