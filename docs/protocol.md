# GraphPad protocol v1

GraphPad v1 uses a compact binary protocol over TCP. TCP is the default because
it works over Wi-Fi and iPad USB tethering without custom USB gadget support.

The schema in `proto/pen.proto` is the long-term compatibility contract. The
Rust and Swift MVP use the byte layout below directly to avoid a code generation
toolchain requirement during the first spike.

## Frame header

All integers are big-endian.

| Bytes | Field | Description |
| --- | --- | --- |
| 0..4 | magic | ASCII `GPAD` |
| 4 | version | `1` |
| 5 | frame type | See frame types below |
| 6..8 | payload length | `u16` byte count |
| 8.. | payload | Type-specific payload |

The daemon rejects frames with the wrong magic, unsupported version, unknown
frame type, or malformed payload.

## Frame types

| Type | Name | Direction | Payload |
| --- | --- | --- | --- |
| 1 | HELLO | client -> host | `u16 protocol_version`, `u32 width_px`, `u32 height_px`, `u16 max_pressure`, `string device_name` |
| 2 | PEN_DOWN | client -> host | Pen event payload |
| 3 | PEN_MOVE | client -> host | Pen event payload |
| 4 | PEN_UP | client -> host | Pen event payload |
| 5 | PEN_HOVER | client -> host | Pen event payload |
| 6 | BUTTON | client -> host | `u64 timestamp_us`, `u8 buttons`, `u32 sequence` |
| 7 | HEARTBEAT | both | `u64 timestamp_us` |
| 8 | CONFIG | host -> client | `u32 display_width`, `u32 display_height`, `u8 mapping_mode`, `u8 preserve_aspect_ratio` |

Strings are encoded as `u16 byte_length` followed by UTF-8 bytes.

## Pen event payload

| Field | Type | Notes |
| --- | --- | --- |
| `timestamp_us` | `u64` | Client monotonic or wall-clock timestamp in microseconds |
| `x_norm` | `f32` | `0.0..1.0` across active iPad capture width |
| `y_norm` | `f32` | `0.0..1.0` across active iPad capture height |
| `pressure` | `f32` | `0.0..1.0`; host maps to 0..4096 by default |
| `tilt_x` | `f32` | `-1.0..1.0`; host maps to tilt degrees |
| `tilt_y` | `f32` | `-1.0..1.0`; host maps to tilt degrees |
| `buttons` | `u8` | Bit 0 -> BTN_STYLUS, bit 1 -> BTN_STYLUS2 |
| `sequence` | `u32` | Monotonic client sequence for diagnostics |

## Host mapping defaults

The Linux backend presents:

- X/Y axes: `0..65535`
- Pressure: `0..4096`
- Tilt X/Y: `-60..60`
- Device name: `GraphPad Virtual Tablet`
- Virtual USB ID: `1209:4750`

Those IDs match `libwacom/graphpad-virtual.tablet`.

