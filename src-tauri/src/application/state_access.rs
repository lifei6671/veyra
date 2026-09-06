use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

/// 串行保护 state.json、备份和迁移恢复的短临界区；不得跨网络或 sidecar 生命周期持有。
#[derive(Clone, Default)]
pub(crate) struct StateAccessGate {
    inner: Arc<Mutex<()>>,
}

impl StateAccessGate {
    pub(crate) fn try_lock(&self) -> Result<MutexGuard<'_, ()>, StateAccessError> {
        match self.inner.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::WouldBlock) => Err(StateAccessError::Busy),
            Err(TryLockError::Poisoned(_)) => Err(StateAccessError::Unavailable),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StateAccessError {
    Busy,
    Unavailable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_state_access_fails_fast_instead_of_waiting() {
        let gate = StateAccessGate::default();
        let _first = gate.try_lock().expect("first access");
        assert_eq!(gate.try_lock().err(), Some(StateAccessError::Busy));
    }
}
