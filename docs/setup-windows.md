# Windows Setup (Phase 3 — stub)

Windows host support is planned for Phase 3 using a virtual digitizer HID driver (`vmulti` or WinUHid). The protocol and iPad client are already cross-platform.

## Current status

- `graphpad-host` builds on Windows
- Use `--dry-run` to test the TCP protocol without a digitizer driver

```powershell
cd host
cargo build --release
.\target\release\graphpad-host.exe --dry-run
```

## Planned integration

1. **vmulti-win11** — digitizer mode for Windows Ink
2. **WinUHid** — user-mode virtual HID (lighter weight)
3. Signed driver bundle for release installs

## Validation targets (Phase 3)

- Device appears in Device Manager as a digitizer
- Windows Ink works in Krita, Clip Studio Paint, Photoshop
- Same iPad app and wire protocol as Linux

WinTab support for legacy apps is deferred until Windows Ink path is stable.
