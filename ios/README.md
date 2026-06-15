# GraphPad iPad client

This directory contains the native iPad client source for the tablet-only MVP.
Create an iOS app target in Xcode named `GraphPad`, add the Swift files under
`GraphPad/`, and enable local network access in the app's Info.plist.

The client streams Apple Pencil samples to the Rust host daemon using the v1
binary protocol documented in `../docs/protocol.md`.

## Development run

1. On Linux, start the host:

   ```sh
   cargo run -p graphpad-daemon -- --backend log
   ```

2. On iPad, connect to the host IP and port `47391`.
3. Draw in the capture surface with Apple Pencil.

For wired testing, connect the iPad by USB-C and enable Personal Hotspot /
USB tethering. Enter the host IP on that tethered subnet in the iPad app.

