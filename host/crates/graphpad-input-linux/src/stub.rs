//! Stub for non-Linux builds (CI / cross-compile checks).

use anyhow::bail;
use graphpad_core::mapping::MappedPenState;
use graphpad_core::session::InputBackend;

pub struct LinuxTablet;

impl LinuxTablet {
    pub fn new() -> anyhow::Result<Self> {
        bail!("Linux tablet backend is only available on Linux")
    }
}

impl InputBackend for LinuxTablet {
    fn apply_state(&mut self, _state: MappedPenState) -> anyhow::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
