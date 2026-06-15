# GraphPad

GraphPad turns an iPad and Apple Pencil into a Wacom-style graphics tablet for a
desktop host. The first milestone is tablet-only input: the iPad acts as the pen
surface and the host exposes a virtual graphics tablet to creative apps.

The Linux target uses `/dev/uinput` for real tablet events and ships optional
libwacom metadata so desktop environments and apps can recognize the virtual
device. The Windows target is structured for Windows Ink support but is not
implemented yet.

## Current status

This repository now contains the Phase 0 / early Phase 1 vertical slice:

- Versioned pen streaming protocol (`docs/protocol.md`, `proto/pen.proto`)
- Rust host daemon that listens for iPad TCP clients
- Linux uinput backend for absolute X/Y, pressure, tilt, hover/proximity, and
  stylus buttons
- Logging backend for network/client validation without uinput permissions
- SwiftUI iPad client source with coalesced Apple Pencil capture
- libwacom descriptor and udev install helper
- Linux and Windows setup notes

## Repository layout

```text
graphing-tablet-app/
├── proto/                         # Protocol schema
├── host/
│   ├── crates/
│   │   ├── graphpad-core/         # Protocol, mapping, pressure curves
│   │   ├── graphpad-input-linux/  # /dev/uinput virtual tablet backend
│   │   ├── graphpad-input-windows/# Phase 3 Windows Ink boundary
│   │   └── graphpad-daemon/       # TCP daemon and backend selection
│   └── packaging/linux/           # udev/libwacom install helper
├── ios/                           # iPad SwiftUI client source
├── libwacom/                      # .tablet metadata and layout SVG
└── docs/                          # Protocol and setup docs
```

## Run the host

Start in logging mode to validate the iPad client and network path:

```sh
cargo run -p graphpad-daemon -- --backend log
```

Run with Linux tablet injection:

```sh
sudo sh host/packaging/linux/install-linux.sh
cargo run -p graphpad-daemon -- --backend uinput
```

By default, the daemon listens on `0.0.0.0:47391`. Use `--listen` to change it:

```sh
cargo run -p graphpad-daemon -- --listen 0.0.0.0:47391 --backend auto
```

Pressure curves:

```sh
cargo run -p graphpad-daemon -- --pressure-curve linear
cargo run -p graphpad-daemon -- --pressure-curve soft
cargo run -p graphpad-daemon -- --pressure-curve firm
```

## Run the iPad client

Create an iOS app target in Xcode named `GraphPad`, add the Swift files from
`ios/GraphPad/`, and use `ios/GraphPad/Info.plist` for local network permission
metadata. Connect to the host IP and port `47391`, then draw in the capture
surface with Apple Pencil.

For the preferred wired path, connect the iPad over USB-C and enable Personal
Hotspot / USB tethering. iPadOS does not provide raw USB HID gadget mode, so the
single-cable path is TCP over the tethered network interface.

## Roadmap

1. **Linux tablet-only MVP**
   - Stabilize uinput device behavior in Krita/GIMP.
   - Add mDNS discovery and manual tether IP helpers.
   - Add multi-monitor mapping and aspect-ratio correction.
2. **Linux desktop polish**
   - Validate libwacom metadata across GNOME/KDE.
   - Improve hover and stylus button mapping.
   - Package daemon and install scripts.
3. **Windows host**
   - Add Windows Ink virtual digitizer backend with WinUHid or VHF.
   - Build installer and validation matrix.
4. **Android client**
   - Reuse the protocol.
   - Add optional Android Open Accessory USB mode.
5. **Screen mirroring**
   - Treat as a separate product phase after input latency and app
     compatibility are solid.

## Success targets

- Wired input latency: `< 8 ms` median, `< 15 ms` p95
- Effective input rate: `120-240 Hz`
- Pressure: at least `1024` usable steps; host default range is `0..4096`
- Linux apps: Krita, GIMP, Blender
- Windows apps after Phase 3: Krita, Clip Studio Paint, Photoshop
