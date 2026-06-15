# Linux Setup

GraphPad exposes a virtual graphics tablet on Linux via `uinput`. Desktop compositors and apps (Krita, GIMP, Blender) see it through `libinput`; the optional libwacom `.tablet` file helps GNOME/KDE configure the active area.

## Requirements

- Linux with `uinput` module (`/dev/uinput`)
- Rust toolchain (1.75+)
- User in the `input` group

## Quick start

```bash
# 1. System setup (udev + libwacom metadata)
sudo ./scripts/setup-linux.sh

# 2. Build host daemon
cd host
cargo build --release

# 3. Run (creates "GraphPad Virtual Tablet")
./target/release/graphpad-host
```

Dry-run mode (log events without creating a virtual device):

```bash
./target/release/graphpad-host --dry-run
```

## Verify the virtual tablet

```bash
# Device should appear after host starts
libinput list-devices | grep -A5 GraphPad

# Watch raw events while drawing from iPad
libinput debug-events --device /dev/input/eventN

# libwacom metadata (after setup-linux.sh)
libwacom-list-local-devices | grep -A10 GraphPad
```

## iPad connection

### USB tether (recommended)

1. Connect iPad to PC with USB-C.
2. On iPad: enable Personal Hotspot or trust the computer when prompted.
3. Note the host IP on the tether interface (often `192.168.x.x`).
4. In GraphPad iPad app → Connect → enter host IP.
5. Open Draw tab and use Apple Pencil.

### Wi-Fi

Same subnet required. Host binds `0.0.0.0:9470` by default.

## Troubleshooting

| Issue | Fix |
|-------|-----|
| `failed to open /dev/uinput` | Run `setup-linux.sh`, add user to `input` group, re-login |
| Device not in Krita | Confirm `libinput` sees tablet; restart Krita |
| No pressure | Check `libinput debug-events` for `ABS_PRESSURE` |
| libwacom not listing device | Re-run setup script; verify `/etc/libwacom/graphpad.tablet` |

## udev rule

Installed by `scripts/setup-linux.sh`:

```
KERNEL=="uinput", MODE="0660", GROUP="input", OPTIONS+="static_node=uinput"
```
