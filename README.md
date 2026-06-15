# GraphPad — iPad as a Graphics Tablet

Turn your iPad into a Wacom-style graphics tablet for Linux (and eventually Windows). A native iPad app streams Apple Pencil input over a low-latency binary protocol; a Rust host daemon exposes a virtual tablet via `uinput` so Krita, GIMP, and Blender recognize pressure and tilt.

**Mode:** Tablet-only (iPad is the pen surface; cursor moves on host monitors). Screen mirroring is planned for a later phase.

## Architecture

```
iPad (Swift/SwiftUI)          Host (Rust)
  Apple Pencil        TCP     graphpad-host
  Coalesced touches   :9470   uinput virtual tablet
  Pressure + tilt             libwacom metadata
```

## Repository layout

```
graphing-tablet-app/
├── proto/              Protocol schema (pen.proto)
├── host/               Rust workspace — host daemon
│   └── crates/
│       ├── graphpad-core/          Protocol, mapping, TCP server
│       ├── graphpad-input-linux/   uinput virtual tablet
│       ├── graphpad-input-windows/ Windows stub (Phase 3)
│       └── graphpad-daemon/        CLI binary (graphpad-host)
├── ios/                iPad client (SwiftUI)
├── libwacom/           .tablet descriptor + SVG layout
├── scripts/            Linux setup (udev, libwacom install)
└── docs/               Protocol and setup guides
```

## Quick start (Linux host)

```bash
# System permissions + libwacom metadata
sudo ./scripts/setup-linux.sh

# Build and run host
cd host && cargo build --release
./target/release/graphpad-host
```

Then open the iPad app, connect to the host IP (USB tether or Wi-Fi), and draw in the **Draw** tab.

See [docs/setup-linux.md](docs/setup-linux.md) for verification steps (libinput, Krita).

## iPad app

Open `ios/GraphPad/GraphPad.xcodeproj` in Xcode 15+, sign with your team, and run on a physical iPad. See [ios/README.md](ios/README.md).

## Protocol

Binary framing on TCP port **9470**. Full spec: [docs/protocol.md](docs/protocol.md).

## Roadmap

| Phase | Status | Scope |
|-------|--------|-------|
| 0–1 | **MVP (this repo)** | Linux uinput, iPad pen streaming, wired + Wi-Fi |
| 2 | Planned | libwacom polish, hover, stylus buttons |
| 3 | Stub | Windows Ink via vmulti/WinUHid |
| 4 | Planned | Android client (+ optional AOA USB) |
| 5 | Planned | Screen mirroring (Astropad-class) |

## License

MIT
