# GraphPad iPad Client

Native SwiftUI app that streams Apple Pencil input to the GraphPad host daemon over TCP.

## Requirements

- Xcode 15+
- iPad with Apple Pencil support
- iOS 17+

## Open the project

1. Open `ios/GraphPad/GraphPad.xcodeproj` in Xcode.
2. Select your development team under Signing & Capabilities.
3. Build and run on a physical iPad (Pencil input requires hardware).

If the Xcode project is not present, create one:

1. File → New → Project → iOS App (SwiftUI)
2. Product name: `GraphPad`
3. Add all files from `ios/GraphPad/GraphPad/` to the target
4. Set deployment target to iOS 17, devices to iPad

## Usage

1. Start `graphpad-host` on your Linux PC (see `docs/setup-linux.md`).
2. Connect iPad via USB-C and enable tethering, or use the same Wi-Fi network.
3. In GraphPad → **Connect**, enter the host IP address.
4. Switch to **Draw** and use Apple Pencil on the canvas area.

## Features (MVP)

- Coalesced touch handling for high-rate pen samples
- Pencil-only mode (palm rejection)
- Pressure and tilt capture
- Pressure curve presets (linear / soft / firm)
- USB tether and Wi-Fi connection

## Protocol

See `docs/protocol.md`. Swift implementation: `GraphPad/Protocol/GraphPadProtocol.swift`.
