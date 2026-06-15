//! Windows virtual digitizer backend (Phase 3 stub).

use anyhow::bail;
use graphpad_core::mapping::MappedPenState;
use graphpad_core::session::InputBackend;
use log::warn;

pub struct WindowsTablet;

impl WindowsTablet {
    pub fn new() -> anyhow::Result<Self> {
        warn!("Windows digitizer backend is not yet implemented (Phase 3)");
        bail!(
            "Windows support requires vmulti/WinUHid integration — see docs/setup-windows.md"
        )
    }
}

impl InputBackend for WindowsTablet {
    fn apply_state(&mut self, _state: MappedPenState) -> anyhow::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

/// Log-only backend for Windows development without a driver installed.
pub struct LogOnlyBackend;

impl InputBackend for LogOnlyBackend {
    fn apply_state(&mut self, state: MappedPenState) -> anyhow::Result<()> {
        log::debug!(
            "pen x={} y={} pressure={} contact={}",
            state.x,
            state.y,
            state.pressure,
            state.in_contact
        );
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}
