# Linux setup

The current Linux host target is the first functional MVP backend. It creates a
virtual graphics tablet through `/dev/uinput` and streams Apple Pencil samples
from the iPad client into standard kernel input events.

## Build

```sh
cargo build -p graphpad-daemon
```

## Run in logging mode

Use this first to validate network connectivity without needing uinput access:

```sh
cargo run -p graphpad-daemon -- --backend log
```

Connect the iPad client to the host IP and port `47391`. The daemon should print
`HELLO` and `PEN` lines while drawing.

## Enable uinput

Install the udev rule and libwacom metadata:

```sh
sudo sh host/packaging/linux/install-linux.sh
```

Then ensure your user belongs to the `input` group and start a new login
session:

```sh
sudo usermod -aG input "$USER"
```

Run the daemon with the Linux backend:

```sh
cargo run -p graphpad-daemon -- --backend uinput
```

The daemon's `auto` backend will try uinput first and fall back to logging if it
cannot open `/dev/uinput`.

## Validate desktop integration

Useful commands:

```sh
libinput debug-events
libwacom-list-local-devices
```

Open Krita or GIMP and select the `GraphPad Virtual Tablet` input device. Verify
absolute movement and pressure before tuning pressure curves or desktop mapping.

## Wired iPad path

iPad cannot act as a raw USB HID gadget. For a single-cable workflow, connect
the iPad over USB-C and enable Personal Hotspot / USB tethering. Enter the host
IP on the tethered interface in the iPad client.

