//! Legacy entrypoint composition; macOS has no bundled observation sidecar yet.
#[cfg(windows)]
pub(crate) use super::windows::managed_sidecar_port::WindowsManagedSidecarPort as ObservationSidecarPort;

#[cfg(not(windows))]
mod unsupported {
    use crate::singbox::{
        GeneratedConfig,
        clash_api::ClashRuntimeObservation,
        runtime::{ManagedSidecar, RuntimeObservationSidecarPort, SidecarPort, SidecarPortError},
    };
    use std::path::PathBuf;

    pub(crate) struct ObservationSidecarPort;
    impl ObservationSidecarPort {
        pub(crate) fn new(_: PathBuf, _: PathBuf) -> Result<Self, SidecarPortError> {
            Err(SidecarPortError)
        }
    }
    impl SidecarPort for ObservationSidecarPort {
        fn check(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            Err(SidecarPortError)
        }
        fn prepare(&mut self, _: &GeneratedConfig) -> Result<(), SidecarPortError> {
            Err(SidecarPortError)
        }
        fn run(&mut self) -> Result<ManagedSidecar, SidecarPortError> {
            Err(SidecarPortError)
        }
        fn ready(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
            Err(SidecarPortError)
        }
        fn stop(&mut self, _: &ManagedSidecar) -> Result<(), SidecarPortError> {
            Err(SidecarPortError)
        }
        fn cancel_pending(&mut self) -> Result<(), SidecarPortError> {
            Ok(())
        }
        fn has_pending_cleanup(&self) -> bool {
            false
        }
    }
    impl RuntimeObservationSidecarPort for ObservationSidecarPort {
        fn read_runtime_observation(
            &mut self,
            _: &ManagedSidecar,
        ) -> Result<ClashRuntimeObservation, SidecarPortError> {
            Err(SidecarPortError)
        }
    }
}
#[cfg(not(windows))]
pub(crate) use unsupported::ObservationSidecarPort;
