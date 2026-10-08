//! Desktop 与 helper 共用的 Child 终止/回收语义；无任意 PID 入口。
use crate::singbox::runtime::SidecarPortError;
use std::{
    process::Child,
    thread,
    time::{Duration, Instant},
};
/// 只使用持有的 Child；try_wait 后不再对已回收 PID 发信号。
pub fn terminate(child: &mut Child) -> Result<(), SidecarPortError> {
    if child.try_wait().map_err(|_| SidecarPortError)?.is_some() {
        return Ok(());
    }
    unsafe {
        libc::kill(child.id() as i32, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if child.try_wait().map_err(|_| SidecarPortError)?.is_some() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    child.kill().map_err(|_| SidecarPortError)?;
    child.wait().map_err(|_| SidecarPortError)?;
    Ok(())
}
