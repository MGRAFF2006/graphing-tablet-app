//! GraphPad binary wire protocol v1.
//!
//! Frame layout (big-endian):
//! ```text
//! magic[4]   = b"GPAD"
//! version    = u8  (currently 1)
//! frame_type = u8
//! length     = u16 (payload bytes)
//! payload    = frame-specific
//! ```

use std::io;

use bytes::{Buf, BufMut, BytesMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MAGIC: &[u8; 4] = b"GPAD";
pub const PROTOCOL_VERSION: u8 = 1;
pub const DEFAULT_PORT: u16 = 9470;
pub const SERVICE_TYPE: &str = "_graphpad._tcp";

/// Maximum tablet axis value (matches common Wacom tablets).
pub const TABLET_AXIS_MAX: i32 = 32767;
/// Pressure range exposed to the OS (4096 levels).
pub const PRESSURE_MAX: i32 = 4096;
/// Tilt axis range (-90° to +90° mapped to this range).
pub const TILT_MAX: i32 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameType {
    Hello = 1,
    PenDown = 2,
    PenMove = 3,
    PenUp = 4,
    PenHover = 5,
    Button = 6,
    Heartbeat = 7,
    Config = 8,
    HostHello = 9,
}

impl FrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Hello),
            2 => Some(Self::PenDown),
            3 => Some(Self::PenMove),
            4 => Some(Self::PenUp),
            5 => Some(Self::PenHover),
            6 => Some(Self::Button),
            7 => Some(Self::Heartbeat),
            8 => Some(Self::Config),
            9 => Some(Self::HostHello),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MappingMode {
    Absolute = 0,
    Relative = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum PressureCurve {
    Linear = 0,
    Soft = 1,
    Firm = 2,
}

/// Stylus button flags.
pub mod buttons {
    pub const STYLUS_PRIMARY: u8 = 1 << 0;
    pub const STYLUS_SECONDARY: u8 = 1 << 1;
    pub const ERASER: u8 = 1 << 2;
    pub const BARREL: u8 = 1 << 3;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hello {
    pub protocol_version: u16,
    pub device_name: String,
    pub screen_width_mm: f32,
    pub screen_height_mm: f32,
    pub max_pressure: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostHello {
    pub protocol_version: u16,
    pub host_name: String,
    pub tablet_width: u32,
    pub tablet_height: u32,
    pub pressure_max: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PenEvent {
    pub timestamp_us: u64,
    pub x_norm: f32,
    pub y_norm: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub buttons: u8,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ButtonEvent {
    pub timestamp_us: u64,
    pub buttons: u8,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Heartbeat {
    pub timestamp_us: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    pub mapping_mode: MappingMode,
    pub pressure_curve: PressureCurve,
    pub target_monitor: u32,
    pub area_left: f32,
    pub area_top: f32,
    pub area_right: f32,
    pub area_bottom: f32,
}

#[derive(Debug, Clone)]
pub enum Frame {
    Hello(Hello),
    HostHello(HostHello),
    PenDown(PenEvent),
    PenMove(PenEvent),
    PenUp(PenEvent),
    PenHover(PenEvent),
    Button(ButtonEvent),
    Heartbeat(Heartbeat),
    Config(Config),
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("invalid magic bytes")]
    InvalidMagic,
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u8),
    #[error("unknown frame type: {0}")]
    UnknownFrameType(u8),
    #[error("payload too short: expected {expected}, got {actual}")]
    PayloadTooShort { expected: usize, actual: usize },
    #[error("invalid UTF-8 in string field")]
    InvalidUtf8,
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

pub fn encode_frame(frame: &Frame) -> BytesMut {
    let payload = encode_payload(frame);
    let frame_type = frame_type_of(frame);

    let mut buf = BytesMut::with_capacity(8 + payload.len());
    buf.put_slice(MAGIC);
    buf.put_u8(PROTOCOL_VERSION);
    buf.put_u8(frame_type as u8);
    buf.put_u16(payload.len() as u16);
    buf.put_slice(&payload);
    buf
}

pub fn decode_frame(buf: &[u8]) -> Result<Frame, ProtocolError> {
    if buf.len() < 8 {
        return Err(ProtocolError::PayloadTooShort {
            expected: 8,
            actual: buf.len(),
        });
    }

    let mut cursor = buf;
    let mut magic = [0u8; 4];
    cursor.copy_to_slice(&mut magic);
    if &magic != MAGIC {
        return Err(ProtocolError::InvalidMagic);
    }

    let version = cursor.get_u8();
    if version != PROTOCOL_VERSION {
        return Err(ProtocolError::UnsupportedVersion(version));
    }

    let frame_type = FrameType::from_u8(cursor.get_u8())
        .ok_or(ProtocolError::UnknownFrameType(buf[5]))?;
    let length = cursor.get_u16() as usize;

    if cursor.remaining() < length {
        return Err(ProtocolError::PayloadTooShort {
            expected: length,
            actual: cursor.remaining(),
        });
    }

    let payload = &cursor[..length];
    decode_payload(frame_type, payload)
}

fn frame_type_of(frame: &Frame) -> FrameType {
    match frame {
        Frame::Hello(_) => FrameType::Hello,
        Frame::HostHello(_) => FrameType::HostHello,
        Frame::PenDown(_) => FrameType::PenDown,
        Frame::PenMove(_) => FrameType::PenMove,
        Frame::PenUp(_) => FrameType::PenUp,
        Frame::PenHover(_) => FrameType::PenHover,
        Frame::Button(_) => FrameType::Button,
        Frame::Heartbeat(_) => FrameType::Heartbeat,
        Frame::Config(_) => FrameType::Config,
    }
}

fn encode_payload(frame: &Frame) -> Vec<u8> {
    let mut buf = Vec::new();
    match frame {
        Frame::Hello(h) => {
            buf.put_u16(h.protocol_version);
            put_string(&mut buf, &h.device_name);
            buf.put_f32(h.screen_width_mm);
            buf.put_f32(h.screen_height_mm);
            buf.put_f32(h.max_pressure);
        }
        Frame::HostHello(h) => {
            buf.put_u16(h.protocol_version);
            put_string(&mut buf, &h.host_name);
            buf.put_u32(h.tablet_width);
            buf.put_u32(h.tablet_height);
            buf.put_u32(h.pressure_max);
        }
        Frame::PenDown(e)
        | Frame::PenMove(e)
        | Frame::PenUp(e)
        | Frame::PenHover(e) => {
            encode_pen_event(&mut buf, *e);
        }
        Frame::Button(b) => {
            buf.put_u64(b.timestamp_us);
            buf.put_u8(b.buttons);
        }
        Frame::Heartbeat(h) => {
            buf.put_u64(h.timestamp_us);
        }
        Frame::Config(c) => {
            buf.put_u8(c.mapping_mode as u8);
            buf.put_u8(c.pressure_curve as u8);
            buf.put_u32(c.target_monitor);
            buf.put_f32(c.area_left);
            buf.put_f32(c.area_top);
            buf.put_f32(c.area_right);
            buf.put_f32(c.area_bottom);
        }
    }
    buf
}

fn encode_pen_event(buf: &mut Vec<u8>, event: PenEvent) {
    buf.put_u64(event.timestamp_us);
    buf.put_f32(event.x_norm);
    buf.put_f32(event.y_norm);
    buf.put_f32(event.pressure);
    buf.put_f32(event.tilt_x);
    buf.put_f32(event.tilt_y);
    buf.put_u8(event.buttons);
    buf.put_u32(event.sequence);
}

fn decode_payload(frame_type: FrameType, payload: &[u8]) -> Result<Frame, ProtocolError> {
    let mut cursor = payload;
    match frame_type {
        FrameType::Hello => {
            need_bytes(&mut cursor, 2)?;
            let protocol_version = cursor.get_u16();
            let device_name = get_string(&mut cursor)?;
            need_bytes(&mut cursor, 12)?;
            Ok(Frame::Hello(Hello {
                protocol_version,
                device_name,
                screen_width_mm: cursor.get_f32(),
                screen_height_mm: cursor.get_f32(),
                max_pressure: cursor.get_f32(),
            }))
        }
        FrameType::HostHello => {
            need_bytes(&mut cursor, 2)?;
            let protocol_version = cursor.get_u16();
            let host_name = get_string(&mut cursor)?;
            need_bytes(&mut cursor, 12)?;
            Ok(Frame::HostHello(HostHello {
                protocol_version,
                host_name,
                tablet_width: cursor.get_u32(),
                tablet_height: cursor.get_u32(),
                pressure_max: cursor.get_u32(),
            }))
        }
        FrameType::PenDown | FrameType::PenMove | FrameType::PenUp | FrameType::PenHover => {
            let event = decode_pen_event(&mut cursor)?;
            Ok(match frame_type {
                FrameType::PenDown => Frame::PenDown(event),
                FrameType::PenMove => Frame::PenMove(event),
                FrameType::PenUp => Frame::PenUp(event),
                FrameType::PenHover => Frame::PenHover(event),
                _ => unreachable!(),
            })
        }
        FrameType::Button => {
            need_bytes(&mut cursor, 9)?;
            Ok(Frame::Button(ButtonEvent {
                timestamp_us: cursor.get_u64(),
                buttons: cursor.get_u8(),
            }))
        }
        FrameType::Heartbeat => {
            need_bytes(&mut cursor, 8)?;
            Ok(Frame::Heartbeat(Heartbeat {
                timestamp_us: cursor.get_u64(),
            }))
        }
        FrameType::Config => {
            need_bytes(&mut cursor, 22)?;
            let mapping_mode = match cursor.get_u8() {
                0 => MappingMode::Absolute,
                _ => MappingMode::Relative,
            };
            let pressure_curve = match cursor.get_u8() {
                0 => PressureCurve::Linear,
                1 => PressureCurve::Soft,
                _ => PressureCurve::Firm,
            };
            Ok(Frame::Config(Config {
                mapping_mode,
                pressure_curve,
                target_monitor: cursor.get_u32(),
                area_left: cursor.get_f32(),
                area_top: cursor.get_f32(),
                area_right: cursor.get_f32(),
                area_bottom: cursor.get_f32(),
            }))
        }
    }
}

fn decode_pen_event(cursor: &mut &[u8]) -> Result<PenEvent, ProtocolError> {
    need_bytes(cursor, 33)?;
    Ok(PenEvent {
        timestamp_us: cursor.get_u64(),
        x_norm: cursor.get_f32(),
        y_norm: cursor.get_f32(),
        pressure: cursor.get_f32(),
        tilt_x: cursor.get_f32(),
        tilt_y: cursor.get_f32(),
        buttons: cursor.get_u8(),
        sequence: cursor.get_u32(),
    })
}

fn put_string(buf: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    buf.put_u16(bytes.len() as u16);
    buf.extend_from_slice(bytes);
}

fn get_string(cursor: &mut &[u8]) -> Result<String, ProtocolError> {
    need_bytes(cursor, 2)?;
    let len = cursor.get_u16() as usize;
    need_bytes(cursor, len)?;
    let value = std::str::from_utf8(&cursor[..len])
        .map_err(|_| ProtocolError::InvalidUtf8)?
        .to_owned();
    cursor.advance(len);
    Ok(value)
}

fn need_bytes(cursor: &mut &[u8], count: usize) -> Result<(), ProtocolError> {
    if cursor.len() < count {
        return Err(ProtocolError::PayloadTooShort {
            expected: count,
            actual: cursor.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_pen_move() {
        let event = PenEvent {
            timestamp_us: 1_234_567,
            x_norm: 0.5,
            y_norm: 0.25,
            pressure: 0.75,
            tilt_x: 0.1,
            tilt_y: -0.2,
            buttons: buttons::STYLUS_PRIMARY,
            sequence: 42,
        };
        let frame = Frame::PenMove(event);
        let encoded = encode_frame(&frame);
        let decoded = decode_frame(&encoded).unwrap();
        match decoded {
            Frame::PenMove(e) => assert_eq!(e.sequence, 42),
            _ => panic!("wrong frame type"),
        }
    }

    #[test]
    fn round_trip_hello() {
        let hello = Hello {
            protocol_version: 1,
            device_name: "iPad Pro".into(),
            screen_width_mm: 214.0,
            screen_height_mm: 273.0,
            max_pressure: 1.0,
        };
        let encoded = encode_frame(&Frame::Hello(hello));
        let decoded = decode_frame(&encoded).unwrap();
        match decoded {
            Frame::Hello(h) => assert_eq!(h.device_name, "iPad Pro"),
            _ => panic!("wrong frame type"),
        }
    }
}
