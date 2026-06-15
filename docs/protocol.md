# GraphPad Wire Protocol v1

Binary framing shared by the iPad client, future Android client, and Rust host daemon.

## Frame layout

All multi-byte integers are **big-endian**.

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | Magic `GPAD` |
| 4 | 1 | Protocol version (`1`) |
| 5 | 1 | Frame type |
| 6 | 2 | Payload length (bytes) |
| 8 | N | Payload |

Default TCP port: **9470**

mDNS service type (planned): `_graphpad._tcp`

## Frame types

| Value | Name | Direction |
|-------|------|-----------|
| 1 | HELLO | Client → Host |
| 2 | PEN_DOWN | Client → Host |
| 3 | PEN_MOVE | Client → Host |
| 4 | PEN_UP | Client → Host |
| 5 | PEN_HOVER | Client → Host |
| 6 | BUTTON | Client → Host |
| 7 | HEARTBEAT | Both |
| 8 | CONFIG | Client → Host |
| 9 | HOST_HELLO | Host → Client |

## Payloads

### HELLO (client → host)

| Field | Type | Description |
|-------|------|-------------|
| protocol_version | u16 | Currently `1` |
| device_name | string | Length-prefixed UTF-8 (u16 len) |
| screen_width_mm | f32 | Active area width |
| screen_height_mm | f32 | Active area height |
| max_pressure | f32 | Client max pressure (1.0) |

### HOST_HELLO (host → client)

| Field | Type |
|-------|------|
| protocol_version | u16 |
| host_name | string |
| tablet_width | u32 |
| tablet_height | u32 |
| pressure_max | u32 |

### Pen events (PEN_DOWN / PEN_MOVE / PEN_UP / PEN_HOVER)

| Field | Type | Description |
|-------|------|-------------|
| timestamp_us | u64 | Microseconds |
| x_norm | f32 | 0.0–1.0 |
| y_norm | f32 | 0.0–1.0 |
| pressure | f32 | 0.0–1.0 |
| tilt_x | f32 | -1.0–1.0 |
| tilt_y | f32 | -1.0–1.0 |
| buttons | u8 | Bit flags |
| sequence | u32 | Monotonic frame counter |

Button flags:

- `0x01` — stylus primary
- `0x02` — stylus secondary
- `0x04` — eraser
- `0x08` — barrel / squeeze

### CONFIG

| Field | Type |
|-------|------|
| mapping_mode | u8 (`0` absolute, `1` relative) |
| pressure_curve | u8 (`0` linear, `1` soft, `2` firm) |
| target_monitor | u32 |
| area_left | f32 |
| area_top | f32 |
| area_right | f32 |
| area_bottom | f32 |

### HEARTBEAT

| Field | Type |
|-------|------|
| timestamp_us | u64 |

## Host mapping

Normalized coordinates map to virtual tablet axes:

- `ABS_X` / `ABS_Y`: 0–32767
- `ABS_PRESSURE`: 0–4096
- `ABS_TILT_X` / `ABS_TILT_Y`: -90–90

The semantic schema is also defined in `proto/pen.proto`.
