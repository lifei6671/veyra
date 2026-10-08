//! 固定Desktop来源与本次helper生命周期内的OS退出观察。不是跨重启退出journal。
use super::{Error, transport::Peer};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use veyra_core::{
    application::runtime_recovery::{
        ClosedBundle, HANDOFF_WIRE_BYTES, KERNEL_DIGEST, OwnerTransfer, SourceSession,
        TransferTicket,
    },
    domain::{SnapshotVersion, StateEpoch},
};
fn process(pid: i32) -> Result<libc::proc_bsdinfo, Error> {
    let mut i = unsafe { std::mem::zeroed::<libc::proc_bsdinfo>() };
    let n = std::mem::size_of_val(&i) as i32;
    if unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut i as *mut libc::proc_bsdinfo).cast(),
            n,
        )
    } != n
    {
        return Err(
            if std::io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH) {
                Error::StaleInstance
            } else {
                Error::HandoffRequired
            },
        );
    }
    Ok(i)
}
pub(super) fn live_peer(peer: Peer) -> Result<(), Error> {
    let i = process(peer.pid)?;
    if i.pbi_uid != peer.uid
        || i.pbi_ruid != peer.uid
        || (i.pbi_start_tvsec, i.pbi_start_tvusec) != peer.start
    {
        return Err(Error::Unauthorized);
    }
    Ok(())
}
fn root(peer: Peer) -> Result<PathBuf, Error> {
    let mut pwd = unsafe { std::mem::zeroed::<libc::passwd>() };
    let mut bytes = vec![0u8; 16384];
    let mut found = std::ptr::null_mut();
    if unsafe {
        libc::getpwuid_r(
            peer.uid,
            &mut pwd,
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            &mut found,
        )
    } != 0
        || found.is_null()
        || pwd.pw_uid != peer.uid
        || pwd.pw_dir.is_null()
    {
        return Err(Error::Unauthorized);
    }
    let home = unsafe { std::ffi::CStr::from_ptr(pwd.pw_dir) }
        .to_str()
        .map_err(|_| Error::UnsafeEndpoint)?;
    if !Path::new(home).is_absolute() {
        return Err(Error::UnsafeEndpoint);
    }
    Ok(Path::new(home).join("Library/Application Support/me.disign.veyra.gpui-preview"))
}
fn directories(path: &Path, uid: u32, base: &Path) -> Result<(), Error> {
    for p in path.ancestors() {
        let m = fs::symlink_metadata(p).map_err(|_| Error::UnsafeEndpoint)?;
        if !m.is_dir() || (m.uid() != 0 && m.uid() != uid) || m.mode() & 0o022 != 0 {
            return Err(Error::UnsafeEndpoint);
        }
        if p.starts_with(base) && (m.uid() != uid || m.mode() & 0o7777 != 0o700) {
            return Err(Error::UnsafeEndpoint);
        }
        #[cfg(test)]
        if p == base {
            break;
        } // 仅测试私有root允许位于系统临时目录；生产逐级检查到/。
    }
    Ok(())
}
fn read(base: &Path, relative: &str, uid: u32, max: usize) -> Result<Vec<u8>, Error> {
    use std::os::unix::ffi::OsStrExt;
    let p = base.join(relative);
    let parent = p.parent().ok_or(Error::UnsafeEndpoint)?;
    directories(parent, uid, base)?;
    // 生产逐级openat+O_NOFOLLOW，避免检查父目录后被同UID替换为symlink。
    #[cfg(test)]
    let (mut directory, rest) = (
        File::open(base).map_err(|_| Error::UnsafeEndpoint)?,
        parent
            .strip_prefix(base)
            .map_err(|_| Error::UnsafeEndpoint)?,
    );
    #[cfg(not(test))]
    let (mut directory, rest) = (
        File::open("/").map_err(|_| Error::UnsafeEndpoint)?,
        parent
            .strip_prefix("/")
            .map_err(|_| Error::UnsafeEndpoint)?,
    );
    for part in rest.components() {
        let std::path::Component::Normal(part) = part else {
            return Err(Error::UnsafeEndpoint);
        };
        let name = std::ffi::CString::new(part.as_bytes()).map_err(|_| Error::UnsafeEndpoint)?;
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(Error::UnsafeEndpoint);
        }
        directory = unsafe { File::from_raw_fd(fd) };
        let m = directory.metadata().map_err(|_| Error::UnsafeEndpoint)?;
        if !m.is_dir() || (m.uid() != 0 && m.uid() != uid) || m.mode() & 0o022 != 0 {
            return Err(Error::UnsafeEndpoint);
        }
    }
    let name = std::ffi::CString::new(p.file_name().ok_or(Error::UnsafeEndpoint)?.as_bytes())
        .map_err(|_| Error::UnsafeEndpoint)?;
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return Err(Error::HandoffRequired);
    }
    let f = unsafe { File::from_raw_fd(fd) };
    let m = f.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file()
        || m.uid() != uid
        || m.nlink() != 1
        || m.mode() & 0o7777 != 0o600
        || m.len() > max as u64
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut b = Vec::new();
    f.take(max as u64 + 1)
        .read_to_end(&mut b)
        .map_err(|_| Error::Unavailable)?;
    if b.len() > max {
        return Err(Error::TooLarge);
    }
    Ok(b)
}
fn children(peer: Peer) -> Result<Vec<i32>, Error> {
    let mut ids = [0i32; 65];
    let bytes = std::mem::size_of_val(&ids) as i32;
    let n = unsafe { libc::proc_listchildpids(peer.pid, ids.as_mut_ptr().cast(), bytes) };
    if n < 0 || n as usize >= ids.len() {
        return Err(Error::HandoffRequired);
    }
    Ok(ids[..n as usize]
        .iter()
        .copied()
        .filter(|id| *id > 0)
        .collect())
}
fn kernel(pid: i32, uid: u32, fixture: bool, deadline: Instant) -> Result<bool, Error> {
    let mut path = [0u8; 4096];
    let n = unsafe { libc::proc_pidpath(pid, path.as_mut_ptr().cast(), path.len() as u32) };
    if n <= 0 {
        return Err(Error::HandoffRequired);
    }
    let p = PathBuf::from(
        std::ffi::CStr::from_bytes_until_nul(&path)
            .map_err(|_| Error::UnsafeEndpoint)?
            .to_str()
            .map_err(|_| Error::UnsafeEndpoint)?,
    );
    #[cfg(test)]
    if fixture {
        return Ok(p == std::env::current_exe().map_err(|_| Error::Unavailable)?);
    }
    let _ = fixture;
    // 来源内核也必须已经使用产品可执行文件名，不能用旧basename进程交接冒充新身份。
    if p.file_name().and_then(|n| n.to_str())
        != Some(veyra_core::application::runtime_recovery::KERNEL_EXECUTABLE)
    {
        return Ok(false);
    }
    if Instant::now() >= deadline {
        return Err(Error::Timeout);
    }
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(p)
        .map_err(|_| Error::UnsafeEndpoint)?;
    let m = f.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !m.is_file()
        || (m.uid() != 0 && m.uid() != uid)
        || m.mode() & 0o022 != 0
        || m.len() > 256 * 1024 * 1024
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut hash = Sha256::new();
    let mut f = f;
    let mut b = [0u8; 65536];
    loop {
        if Instant::now() >= deadline {
            return Err(Error::Timeout);
        }
        let n = f.read(&mut b).map_err(|_| Error::Unavailable)?;
        if n == 0 {
            break;
        }
        hash.update(&b[..n]);
    }
    Ok(format!("{:x}", hash.finalize()) == KERNEL_DIGEST)
}
pub(super) struct Witness {
    peer: Peer,
    root: PathBuf,
    id: StateEpoch,
    version: SnapshotVersion,
    session: SourceSession,
    child: i32,
    start: (u64, u64),
    queue: OwnedFd,
    exited: bool,
    fixture: bool,
    #[cfg(test)]
    exclude: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
}
impl Witness {
    pub fn capture(
        peer: Peer,
        id: StateEpoch,
        version: SnapshotVersion,
        #[cfg(test)] injected: Option<PathBuf>,
        #[cfg(test)] exclude: std::sync::Arc<std::sync::Mutex<Vec<u32>>>,
    ) -> Result<Self, Error> {
        live_peer(peer)?;
        #[cfg(test)]
        let fixture = injected.is_some();
        #[cfg(not(test))]
        let fixture = false;
        #[cfg(test)]
        let base = if let Some(p) = injected {
            p
        } else {
            root(peer)?
        };
        #[cfg(not(test))]
        let base = root(peer)?;
        directories(&base, peer.uid, &base)?;
        let session: SourceSession =
            serde_json::from_slice(&read(&base, "runtime/source-session.json", peer.uid, 1024)?)
                .map_err(|_| Error::InvalidRequest)?;
        if session.uid != peer.uid
            || session.pid != peer.pid
            || session.start != peer.start
            || session.incarnation.len() != 64
        {
            return Err(Error::Unauthorized);
        }
        let manifest: veyra_core::application::runtime_recovery::LastAppliedManifest =
            serde_json::from_slice(&read(&base, "runtime/last-applied.json", peer.uid, 65536)?)
                .map_err(|_| Error::InvalidRequest)?;
        if manifest.config != version.config
            || manifest.confirmed_selection.version != version.selection
        {
            return Err(Error::VersionConflict);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut managed = Vec::new();
        for pid in children(peer)? {
            if kernel(pid, peer.uid, fixture, deadline)? {
                managed.push(pid)
            }
        }
        // 只启用预检时可观察到唯一活内核的正常路径；已Stopped/cold/orphan均不猜测。
        if managed.len() != 1 {
            return Err(Error::HandoffRequired);
        }
        let child = managed[0];
        let info = process(child)?;
        if info.pbi_ppid != peer.pid as u32 || info.pbi_uid != peer.uid || info.pbi_ruid != peer.uid
        {
            return Err(Error::Unauthorized);
        }
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(Error::Unavailable);
        }
        let queue = unsafe { OwnedFd::from_raw_fd(fd) };
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(Error::Unavailable);
        }
        let event = libc::kevent {
            ident: child as usize,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        if unsafe { libc::kevent(fd, &event, 1, std::ptr::null_mut(), 0, std::ptr::null()) } != 0 {
            return Err(Error::HandoffRequired);
        }
        // 注册观察与采样之间也可能退出/PID重用；必须仍是采样的同一活身份。
        // 失败留在只读预检，不允许用后来另一个进程的NOTE_EXIT授权旧writer交接。
        let registered = process(child)?;
        if registered.pbi_ppid != info.pbi_ppid
            || registered.pbi_uid != info.pbi_uid
            || registered.pbi_ruid != info.pbi_ruid
            || (registered.pbi_start_tvsec, registered.pbi_start_tvusec)
                != (info.pbi_start_tvsec, info.pbi_start_tvusec)
        {
            return Err(Error::HandoffRequired);
        }
        Ok(Self {
            peer,
            root: base,
            id,
            version,
            session,
            child,
            start: (info.pbi_start_tvsec, info.pbi_start_tvusec),
            queue,
            exited: false,
            fixture,
            #[cfg(test)]
            exclude,
        })
    }
    pub fn matches(&self, peer: Peer, id: &StateEpoch, version: &SnapshotVersion) -> bool {
        self.peer == peer && self.id == *id && self.version == *version
    }
    pub fn verify(
        &mut self,
        peer: Peer,
        ticket: &TransferTicket,
        released: bool,
    ) -> Result<(), Error> {
        if !self.matches(peer, &ticket.id, &ticket.version) {
            return Err(Error::Unauthorized);
        }
        live_peer(peer)?;
        if !self.exited {
            let mut e = unsafe { std::mem::zeroed::<libc::kevent>() };
            let timeout = libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            };
            let n = unsafe {
                libc::kevent(
                    self.queue.as_raw_fd(),
                    std::ptr::null(),
                    0,
                    &mut e,
                    1,
                    &timeout,
                )
            };
            if n != 1 || e.ident != self.child as usize || e.fflags & libc::NOTE_EXIT == 0 {
                return Err(Error::HandoffRequired);
            }
            self.exited = true;
        }
        // NOTE_EXIT之后仍要求旧身份已被回收；PID重用不能被当成旧child还活着或用于kill。
        match process(self.child) {
            Ok(i) if (i.pbi_start_tvsec, i.pbi_start_tvusec) != self.start => {}
            Err(Error::StaleInstance) => {}
            _ => return Err(Error::HandoffRequired),
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        for pid in children(peer)? {
            #[cfg(test)]
            if self.exclude.lock().unwrap().contains(&(pid as u32)) {
                continue;
            }
            if kernel(pid, peer.uid, self.fixture, deadline)? {
                return Err(Error::HandoffRequired);
            }
        }
        let session: SourceSession = serde_json::from_slice(&read(
            &self.root,
            "runtime/source-session.json",
            peer.uid,
            1024,
        )?)
        .map_err(|_| Error::InvalidRequest)?;
        if session != self.session
            || read(&self.root, "runtime/owner-incarnation.json", peer.uid, 256)?
                != self.session.incarnation.as_bytes()
        {
            return Err(Error::Unauthorized);
        }
        let owner: OwnerTransfer = serde_json::from_slice(&read(
            &self.root,
            "runtime/owner-transfer.json",
            peer.uid,
            4096,
        )?)
        .map_err(|_| Error::InvalidRequest)?;
        let expected = if released {
            OwnerTransfer::Released {
                ticket: ticket.clone(),
            }
        } else {
            OwnerTransfer::Closed {
                ticket: ticket.clone(),
            }
        };
        if owner != expected
            && !(!released
                && owner
                    == OwnerTransfer::Released {
                        ticket: ticket.clone(),
                    })
        {
            return Err(Error::HandoffRequired);
        }
        let bundle: ClosedBundle = serde_json::from_slice(&read(
            &self.root,
            "runtime/owner-bundle.json",
            peer.uid,
            HANDOFF_WIRE_BYTES,
        )?)
        .map_err(|_| Error::InvalidRequest)?;
        if bundle
            .ticket(ticket.id.clone())
            .map_err(|_| Error::InvalidConfiguration)?
            != *ticket
        {
            return Err(Error::VersionConflict);
        }
        let manifest = bundle.source_manifest();
        let actual: veyra_core::application::runtime_recovery::LastAppliedManifest =
            serde_json::from_slice(&read(
                &self.root,
                "runtime/last-applied.json",
                peer.uid,
                65536,
            )?)
            .map_err(|_| Error::InvalidRequest)?;
        if actual != *manifest {
            return Err(Error::VersionConflict);
        }
        let cache = manifest.cache.as_ref().ok_or(Error::HandoffRequired)?;
        for (prefix, hash, max) in [
            ("runtime/plan-", &manifest.plan.digest, 1024 * 1024),
            ("kernel-cache/rollback-", &cache.digest, 2 * 1024 * 1024),
        ] {
            if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(Error::InvalidRequest);
            }
            let suffix = if prefix.starts_with("runtime/") {
                ".json"
            } else {
                ".db"
            };
            let bytes = read(
                &self.root,
                &format!("{prefix}{hash}{suffix}"),
                peer.uid,
                max,
            )?;
            if format!("{:x}", Sha256::digest(&bytes)) != *hash {
                return Err(Error::InvalidConfiguration);
            }
        }
        let generation = veyra_core::application::runtime_recovery::cache_generation(
            &ticket.version.config.0.epoch,
        );
        let live = read(
            &self.root,
            &format!("kernel-cache/{generation}.db"),
            peer.uid,
            2 * 1024 * 1024,
        )?;
        if format!("{:x}", Sha256::digest(&live)) != cache.digest {
            return Err(Error::InvalidConfiguration);
        }
        Ok(())
    }
}

/// 首次Prepare仅核对合作冻结，不宣称已观察旧writer退出或验证客户端代码签名。
pub(super) fn bootstrap(
    peer: Peer,
    ticket: &super::BootstrapTicket,
) -> Result<BootstrapSource, Error> {
    bootstrap_at(peer, ticket, &root(peer)?)
}
#[cfg(test)]
pub(super) fn test_bootstrap(
    peer: Peer,
    ticket: &super::BootstrapTicket,
    base: &Path,
) -> Result<BootstrapSource, Error> {
    bootstrap_at(peer, ticket, base)
}
fn bootstrap_at(
    peer: Peer,
    ticket: &super::BootstrapTicket,
    base: &Path,
) -> Result<BootstrapSource, Error> {
    live_peer(peer)?;
    let (lock, socket) = primary(peer, base)?;
    let owner: OwnerTransfer =
        serde_json::from_slice(&read(base, "runtime/owner-transfer.json", peer.uid, 4096)?)
            .map_err(|_| Error::HandoffRequired)?;
    if owner
        != (OwnerTransfer::BootstrapFrozen {
            id: ticket.id.clone(),
            installation: ticket.installation.clone(),
            version: ticket.version.clone(),
        })
    {
        return Err(Error::HandoffRequired);
    }
    let session: SourceSession =
        serde_json::from_slice(&read(base, "runtime/source-session.json", peer.uid, 4096)?)
            .map_err(|_| Error::HandoffRequired)?;
    let incarnation = read(base, "runtime/owner-incarnation.json", peer.uid, 256)?;
    if session.uid != peer.uid
        || session.pid != peer.pid
        || session.start != peer.start
        || session.incarnation.len() != 64
        || !session.incarnation.bytes().all(|b| b.is_ascii_hexdigit())
        || session.incarnation.as_bytes() != incarnation
    {
        return Err(Error::Unauthorized);
    }
    live_peer(peer)?;
    Ok(BootstrapSource {
        incarnation: session.incarnation,
        lock,
        socket,
    })
}

/// OS合作身份只对本产品守约参与者成立，不是同UID恶意代码认证。
#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BootstrapSource {
    pub incarnation: String,
    lock: (u64, u64),
    socket: (u64, u64),
}
type PrimaryFiles = ((u64, u64), (u64, u64));
fn primary(peer: Peer, base: &Path) -> Result<PrimaryFiles, Error> {
    use std::os::unix::fs::FileTypeExt;
    let lock_path = base.join("locks/desktop.lock");
    let socket_path = base.join("runtime/desktop.sock");
    directories(lock_path.parent().unwrap(), peer.uid, base)?;
    directories(socket_path.parent().unwrap(), peer.uid, base)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(&lock_path)
        .map_err(|_| Error::HandoffRequired)?;
    let metadata = lock.metadata().map_err(|_| Error::UnsafeEndpoint)?;
    if !metadata.is_file()
        || metadata.uid() != peer.uid
        || metadata.mode() & 0o7777 != 0o600
        || metadata.nlink() != 1
    {
        return Err(Error::UnsafeEndpoint);
    }
    let locked = || {
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            unsafe {
                libc::flock(lock.as_raw_fd(), libc::LOCK_UN);
            }
            return Err(Error::HandoffRequired);
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::WouldBlock {
            return Err(Error::UnsafeEndpoint);
        }
        Ok(())
    };
    locked()?;
    let socket = fs::symlink_metadata(&socket_path).map_err(|_| Error::HandoffRequired)?;
    if !socket.file_type().is_socket()
        || socket.uid() != peer.uid
        || socket.mode() & 0o7777 != 0o600
    {
        return Err(Error::UnsafeEndpoint);
    }
    let mut stream = super::transport::connect_bounded(&socket_path)?;
    if super::transport::peer(&stream)? != peer {
        return Err(Error::Unauthorized);
    }
    let deadline = Instant::now() + Duration::from_millis(500);
    let mut probe = super::OWNER_PROBE;
    super::transport::transfer(&mut stream, &mut probe, true, deadline)?;
    let mut ack = [0; 2];
    super::transport::transfer(&mut stream, &mut ack, false, deadline)?;
    if ack != super::OWNER_ACK {
        return Err(Error::Unauthorized);
    }
    locked()?;
    let lm = fs::symlink_metadata(lock_path).map_err(|_| Error::UnsafeEndpoint)?;
    let sm = fs::symlink_metadata(socket_path).map_err(|_| Error::UnsafeEndpoint)?;
    if (lm.dev(), lm.ino()) != (metadata.dev(), metadata.ino())
        || (sm.dev(), sm.ino()) != (socket.dev(), socket.ino())
    {
        return Err(Error::UnsafeEndpoint);
    }
    live_peer(peer)?;
    Ok(((lm.dev(), lm.ino()), (sm.dev(), sm.ino())))
}

/// 仅观察经过内核认证的peer；退出事件不是PID kill授权，清理仍只使用已有Child句柄。
pub(super) struct PeerExit {
    queue: OwnedFd,
    peer: Peer,
    exited: bool,
}
impl PeerExit {
    pub(super) fn capture(peer: Peer) -> Result<Self, Error> {
        live_peer(peer)?;
        let fd = unsafe { libc::kqueue() };
        if fd < 0 {
            return Err(Error::Unavailable);
        }
        let queue = unsafe { OwnedFd::from_raw_fd(fd) };
        if unsafe { libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC) } < 0 {
            return Err(Error::Unavailable);
        }
        let event = libc::kevent {
            ident: peer.pid as usize,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_ENABLE,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        if unsafe { libc::kevent(fd, &event, 1, std::ptr::null_mut(), 0, std::ptr::null()) } != 0 {
            return Err(Error::HandoffRequired);
        }
        live_peer(peer)?; // 注册前后必须仍是同一启动身份。
        Ok(Self {
            queue,
            peer,
            exited: false,
        })
    }
    pub(super) fn exited(&mut self) -> Result<bool, Error> {
        if self.exited {
            return Ok(true);
        }
        let mut event = unsafe { std::mem::zeroed::<libc::kevent>() };
        let timeout = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        let n = unsafe {
            libc::kevent(
                self.queue.as_raw_fd(),
                std::ptr::null(),
                0,
                &mut event,
                1,
                &timeout,
            )
        };
        if n < 0 {
            return Err(Error::Unavailable);
        }
        if n == 0 {
            return Ok(false);
        }
        if event.ident != self.peer.pid as usize
            || event.fflags & libc::NOTE_EXIT == 0
            || event.flags & libc::EV_ERROR != 0
        {
            return Err(Error::HandoffRequired);
        }
        self.exited = true;
        Ok(true)
    }
}

/// rebind只核验新Primary与保留的旧来源，不把用户目录当root CleanStop证明。
pub(super) fn rebind(
    peer: Peer,
    old_peer: Peer,
    old_incarnation: &str,
    previous: &super::BootstrapTicket,
    ticket: Option<&super::BootstrapTicket>,
    #[cfg(test)] injected: Option<&Path>,
) -> Result<Option<BootstrapSource>, Error> {
    #[cfg(test)]
    let base = injected
        .map(Path::to_path_buf)
        .map(Ok)
        .unwrap_or_else(|| root(peer))?;
    #[cfg(not(test))]
    let base = root(peer)?;
    live_peer(peer)?;
    let (lock, socket) = primary(peer, &base)?;
    let owner: OwnerTransfer =
        serde_json::from_slice(&read(&base, "runtime/owner-transfer.json", peer.uid, 4096)?)
            .map_err(|_| Error::HandoffRequired)?;
    if owner
        != (OwnerTransfer::BootstrapFrozen {
            id: previous.id.clone(),
            installation: previous.installation.clone(),
            version: previous.version.clone(),
        })
    {
        return Err(Error::HandoffRequired);
    }
    let old: SourceSession =
        serde_json::from_slice(&read(&base, "runtime/source-session.json", peer.uid, 4096)?)
            .map_err(|_| Error::HandoffRequired)?;
    if old.uid != old_peer.uid
        || old.pid != old_peer.pid
        || old.start != old_peer.start
        || old.incarnation != old_incarnation
        || read(&base, "runtime/owner-incarnation.json", peer.uid, 256)?
            != old_incarnation.as_bytes()
    {
        return Err(Error::Unauthorized);
    }
    let Some(ticket) = ticket else {
        return Ok(None);
    };
    let proposal: super::RebindPending = serde_json::from_slice(&read(
        &base,
        "runtime/bootstrap-rebind.json",
        peer.uid,
        8192,
    )?)
    .map_err(|_| Error::HandoffRequired)?;
    if proposal.previous != *previous
        || proposal.ticket != *ticket
        || proposal.session.uid != peer.uid
        || proposal.session.pid != peer.pid
        || proposal.session.start != peer.start
        || proposal.session.incarnation == old_incarnation
        || proposal.session.incarnation.len() != 64
        || !proposal
            .session
            .incarnation
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(Error::Unauthorized);
    }
    live_peer(peer)?;
    Ok(Some(BootstrapSource {
        incarnation: proposal.session.incarnation,
        lock,
        socket,
    }))
}
