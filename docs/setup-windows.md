# Windows setup

Windows host support is a Phase 3 target. The repository already has a
`graphpad-input-windows` crate boundary so the daemon can keep the same protocol
and session logic when the virtual digitizer backend is added.

Planned backend options:

1. WinUHid user-mode virtual HID digitizer for a lower-friction prototype.
2. VHF-based virtual digitizer driver for a production Windows Ink path.
3. Optional WinTab shim only after Windows Ink is validated.

Validation targets:

- Device appears as a pen/digitizer in Device Manager.
- Windows Ink works in Krita, Clip Studio Paint, and Photoshop.
- Same iPad client and GraphPad TCP protocol work without app changes.

The current Windows crate returns `Unsupported` instead of silently degrading to
mouse events. That keeps the compatibility boundary explicit while Linux reaches
the first working tablet milestone.

