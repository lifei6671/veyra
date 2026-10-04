use super::{PlatformError, directories::AppDirectories};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{
            fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
            net::{UnixListener, UnixStream},
        },
    },
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceCommand {
    Activate,
}
// Fixed versioned wire bytes: no paths, arbitrary commands, shell or JSON.
const ACTIVATE: [u8; 2] = [1, 1];
const ACK: [u8; 2] = [1, 2];
fn same_user(stream: &UnixStream) -> bool {
    let mut uid = 0;
    let mut gid = 0;
    // SAFETY: valid socket FD and live scalar output pointers; no ownership transfer.
    unsafe {
        libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) == 0 && uid == libc::geteuid()
    }
}
fn locked_file(d: &AppDirectories) -> Result<Option<File>, PlatformError> {
    let f = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(d.lock())
        .map_err(|_| PlatformError::PermissionDenied)?;
    let m = f.metadata().map_err(|_| PlatformError::PermissionDenied)?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } {
        return Err(PlatformError::PermissionDenied);
    }
    f.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|_| PlatformError::PermissionDenied)?;
    if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(f));
    }
    let e = std::io::Error::last_os_error();
    if e.kind() == std::io::ErrorKind::WouldBlock {
        Ok(None)
    } else {
        Err(PlatformError::PermissionDenied)
    }
}
pub enum Ownership {
    Primary(PrimaryInstance, UnboundedReceiver<InstanceCommand>),
    Secondary,
}
pub struct PrimaryInstance {
    _lock: File,
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<thread::JoinHandle<()>>>,
    socket: std::path::PathBuf,
}
impl PrimaryInstance {
    /// GPUI's native termination does not unwind main. Stop IPC/remove its socket,
    /// but retain the lock FD in the composition root until the process actually exits.
    pub fn prepare_quit(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(t) = self.thread.lock().expect("instance listener owner").take() {
            let _ = t.join();
        }
        let _ = fs::remove_file(&self.socket);
    }
}
impl Drop for PrimaryInstance {
    fn drop(&mut self) {
        self.prepare_quit(); // FD closes AFTER listener/socket cleanup.
    }
}
/// Must be called before constructing ANY AppServices/JsonStateStore writer.
pub fn acquire(d: &AppDirectories) -> Result<Ownership, PlatformError> {
    d.prepare_ownership()?;
    let Some(lock) = locked_file(d)? else {
        let deadline = Instant::now() + Duration::from_millis(800);
        loop {
            if let Ok(mut s) = UnixStream::connect(d.socket()) {
                s.set_read_timeout(Some(Duration::from_millis(200)))
                    .map_err(|_| PlatformError::InstanceBusyUnconfirmed)?;
                s.set_write_timeout(Some(Duration::from_millis(200)))
                    .map_err(|_| PlatformError::InstanceBusyUnconfirmed)?;
                let mut ack = [0; 2];
                if same_user(&s)
                    && s.write_all(&ACTIVATE).is_ok()
                    && s.read_exact(&mut ack).is_ok()
                    && ack == ACK
                {
                    return Ok(Ownership::Secondary);
                }
            }
            if Instant::now() >= deadline {
                return Err(PlatformError::InstanceBusyUnconfirmed);
            }
            thread::sleep(Duration::from_millis(40));
        }
    };
    let socket = d.socket();
    if let Ok(m) = fs::symlink_metadata(&socket) {
        if !m.file_type().is_socket() || m.uid() != unsafe { libc::geteuid() } {
            return Err(PlatformError::InstanceBusyUnconfirmed);
        }
        // 取得 flock 不代表其他 listener 已失效；只有明确拒绝连接才清理。
        match UnixStream::connect(&socket) {
            Err(e) if e.kind() == std::io::ErrorKind::ConnectionRefused => {}
            _ => return Err(PlatformError::InstanceBusyUnconfirmed),
        }
        fs::remove_file(&socket).map_err(|_| PlatformError::InstanceBusyUnconfirmed)?;
    }
    let listener =
        UnixListener::bind(&socket).map_err(|_| PlatformError::InstanceBusyUnconfirmed)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))
        .map_err(|_| PlatformError::PermissionDenied)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| PlatformError::InstanceBusyUnconfirmed)?;
    let (tx, rx) = unbounded_channel();
    let stop = Arc::new(AtomicBool::new(false));
    let done = stop.clone();
    let t = thread::spawn(move || {
        while !done.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut s, _)) => {
                    let _ = s.set_read_timeout(Some(Duration::from_millis(200)));
                    let _ = s.set_write_timeout(Some(Duration::from_millis(200)));
                    let mut command = [0; 2];
                    if same_user(&s)
                        && s.read_exact(&mut command).is_ok()
                        && command == ACTIVATE
                        && tx.send(InstanceCommand::Activate).is_ok()
                    {
                        let _ = s.write_all(&ACK);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(_) => break,
            }
        }
    });
    Ok(Ownership::Primary(
        PrimaryInstance {
            _lock: lock,
            stop,
            thread: Mutex::new(Some(t)),
            socket,
        },
        rx,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn dirs() -> AppDirectories {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        AppDirectories::injected(std::env::temp_dir().join(format!(
            "veyra-p105-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        )))
    }
    #[test]
    fn primary_secondary_fixed_activate_and_one_services_writer() {
        let d = dirs();
        let Ownership::Primary(owner, mut rx) = acquire(&d).unwrap() else {
            panic!()
        };
        let before = crate::services::WRITERS_CREATED.get();
        let (services, _) =
            crate::services::AppServices::new(d.application_support.clone()).unwrap();
        services.snapshots.snapshot().unwrap();
        let mut writer_created_count = 1;
        for _ in 0..5 {
            match acquire(&d).unwrap() {
                Ownership::Secondary => {}
                Ownership::Primary(_, _) => writer_created_count += 1,
            }
            assert_eq!(rx.try_recv().unwrap(), InstanceCommand::Activate);
        }
        assert_eq!(writer_created_count, 1);
        assert_eq!(crate::services::WRITERS_CREATED.get() - before, 1);
        drop(services);
        owner.prepare_quit();
        assert!(!d.socket().exists());
        assert!(matches!(
            acquire(&d),
            Err(PlatformError::InstanceBusyUnconfirmed)
        ));
        assert_eq!(crate::services::WRITERS_CREATED.get() - before, 1);
        drop(owner);
        assert!(!d.socket().exists());
        fs::remove_dir_all(d.application_support).unwrap();
    }
    #[test]
    fn stale_socket_and_busy_without_confirmed_control_never_take_writer() {
        let d = dirs();
        d.prepare_ownership().unwrap();
        let lock = locked_file(&d).unwrap().unwrap();
        assert!(matches!(
            acquire(&d),
            Err(PlatformError::InstanceBusyUnconfirmed)
        ));
        assert!(!d.state().exists());
        drop(lock);
        let stale = UnixListener::bind(d.socket()).unwrap();
        drop(stale);
        let Ownership::Primary(owner, _) = acquire(&d).unwrap() else {
            panic!()
        };
        drop(owner);
        fs::remove_dir_all(d.application_support).unwrap();
    }
    // 保护未参与 Veyra flock 的独立活 listener，避免删除或接管其 socket。
    #[test]
    fn live_socket_without_veyra_lock_is_busy_and_remains_connectable() {
        let d = dirs();
        d.prepare_ownership().unwrap();
        let listener = UnixListener::bind(d.socket()).unwrap();
        let inode = fs::symlink_metadata(d.socket()).unwrap().ino();

        assert!(matches!(
            acquire(&d),
            Err(PlatformError::InstanceBusyUnconfirmed)
        ));
        assert_eq!(fs::symlink_metadata(d.socket()).unwrap().ino(), inode);
        let _connection = UnixStream::connect(d.socket()).unwrap();
        listener.accept().unwrap();
        assert!(!d.state().exists());
        assert!(locked_file(&d).unwrap().is_some());
        drop(listener);
        fs::remove_dir_all(d.application_support).unwrap();
    }
    #[test]
    fn crash_process_releases_kernel_lock_without_pid_file_cleanup() {
        use std::process::{Command, Stdio};
        let d = dirs();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "platform::single_instance::tests::lock_child",
                "--nocapture",
            ])
            .env("VEYRA_LOCK_CHILD", &d.application_support)
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        let ready = d.support_path("ready");
        let until = Instant::now() + Duration::from_secs(5);
        while !ready.exists() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(10));
        }
        assert!(matches!(
            acquire(&d),
            Err(PlatformError::InstanceBusyUnconfirmed)
        ));
        child.kill().unwrap();
        child.wait().unwrap();
        let Ownership::Primary(owner, _) = acquire(&d).unwrap() else {
            panic!()
        };
        drop(owner);
        fs::remove_dir_all(d.application_support).unwrap();
    }
    #[test]
    fn lock_child() {
        if let Some(root) = std::env::var_os("VEYRA_LOCK_CHILD") {
            let d = AppDirectories::injected(root.into());
            d.prepare_ownership().unwrap();
            let _lock = locked_file(&d).unwrap().unwrap();
            fs::write(d.support_path("ready"), b"ready").unwrap();
            loop {
                thread::sleep(Duration::from_secs(1));
            }
        }
    }
}
