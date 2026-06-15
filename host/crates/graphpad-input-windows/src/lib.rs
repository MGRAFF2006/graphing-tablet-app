use graphpad_core::mapping::MappedPenEvent;
use std::io;

#[derive(Debug, Default)]
pub struct WindowsInkTablet;

impl WindowsInkTablet {
    pub fn open() -> io::Result<Self> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows Ink backend is planned for Phase 3 via WinUHid or a VHF digitizer driver",
        ))
    }

    pub fn emit_pen(&mut self, _event: MappedPenEvent) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Windows Ink backend is not implemented yet",
        ))
    }
}
