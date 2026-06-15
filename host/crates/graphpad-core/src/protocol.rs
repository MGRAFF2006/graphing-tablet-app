use std::fmt;
use std::io::{self, Read, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAGIC: &[u8; 4] = b"GPAD";
pub const HEADER_LEN: usize = 8;
pub const MAX_FRAME_LEN: u32 = 64 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Hello {
    pub protocol_version: u16,
    pub device_name: String,
    pub width_px: u32,
    pub height_px: u32,
    pub max_pressure: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PenPhase {
    Down,
    Move,
    Up,
    Hover,
}

impl PenPhase {
    pub fn tool_down(self) -> bool {
        matches!(self, Self::Down | Self::Move)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenEvent {
    pub phase: PenPhase,
    pub timestamp_us: u64,
    pub x_norm: f32,
    pub y_norm: f32,
    pub pressure: f32,
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub buttons: u8,
    pub sequence: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonEvent {
    pub timestamp_us: u64,
    pub buttons: u8,
    pub sequence: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Frame {
    Hello(Hello),
    Pen(PenEvent),
    Button(ButtonEvent),
    Heartbeat { timestamp_us: u64 },
    Config(Config),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingMode {
    Absolute,
    Relative,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Config {
    pub display_width: u32,
    pub display_height: u32,
    pub mapping_mode: MappingMode,
    pub preserve_aspect_ratio: bool,
}

#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    BadMagic([u8; 4]),
    UnsupportedVersion(u16),
    UnknownFrameType(u8),
    InvalidLength(u32),
    InvalidPayload(&'static str),
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::BadMagic(magic) => write!(f, "bad magic bytes: {magic:?}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported protocol version: {version}")
            }
            Self::UnknownFrameType(kind) => write!(f, "unknown frame type: {kind}"),
            Self::InvalidLength(length) => write!(f, "invalid frame length: {length}"),
            Self::InvalidPayload(reason) => write!(f, "invalid frame payload: {reason}"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<io::Error> for ProtocolError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Hello = 1,
    PenDown = 2,
    PenMove = 3,
    PenUp = 4,
    PenHover = 5,
    Button = 6,
    Heartbeat = 7,
    Config = 8,
}

impl FrameKind {
    fn from_byte(value: u8) -> Result<Self, ProtocolError> {
        match value {
            1 => Ok(Self::Hello),
            2 => Ok(Self::PenDown),
            3 => Ok(Self::PenMove),
            4 => Ok(Self::PenUp),
            5 => Ok(Self::PenHover),
            6 => Ok(Self::Button),
            7 => Ok(Self::Heartbeat),
            8 => Ok(Self::Config),
            other => Err(ProtocolError::UnknownFrameType(other)),
        }
    }
}

impl From<PenPhase> for FrameKind {
    fn from(value: PenPhase) -> Self {
        match value {
            PenPhase::Down => Self::PenDown,
            PenPhase::Move => Self::PenMove,
            PenPhase::Up => Self::PenUp,
            PenPhase::Hover => Self::PenHover,
        }
    }
}

impl Frame {
    pub fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        let kind = match self {
            Self::Hello(hello) => {
                write_u16(&mut payload, hello.protocol_version);
                write_u32(&mut payload, hello.width_px);
                write_u32(&mut payload, hello.height_px);
                write_u16(&mut payload, hello.max_pressure);
                write_string(&mut payload, &hello.device_name);
                FrameKind::Hello
            }
            Self::Pen(event) => {
                write_u64(&mut payload, event.timestamp_us);
                write_f32(&mut payload, event.x_norm);
                write_f32(&mut payload, event.y_norm);
                write_f32(&mut payload, event.pressure);
                write_f32(&mut payload, event.tilt_x);
                write_f32(&mut payload, event.tilt_y);
                payload.push(event.buttons);
                write_u32(&mut payload, event.sequence);
                event.phase.into()
            }
            Self::Button(event) => {
                write_u64(&mut payload, event.timestamp_us);
                payload.push(event.buttons);
                write_u32(&mut payload, event.sequence);
                FrameKind::Button
            }
            Self::Heartbeat { timestamp_us } => {
                write_u64(&mut payload, *timestamp_us);
                FrameKind::Heartbeat
            }
            Self::Config(config) => {
                write_u32(&mut payload, config.display_width);
                write_u32(&mut payload, config.display_height);
                payload.push(match config.mapping_mode {
                    MappingMode::Absolute => 0,
                    MappingMode::Relative => 1,
                });
                payload.push(u8::from(config.preserve_aspect_ratio));
                FrameKind::Config
            }
        };

        let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
        frame.extend_from_slice(MAGIC);
        frame.push(PROTOCOL_VERSION as u8);
        frame.push(kind as u8);
        write_u16(&mut frame, payload.len() as u16);
        frame.extend_from_slice(&payload);
        frame
    }

    pub fn decode(kind: u8, payload: &[u8]) -> Result<Self, ProtocolError> {
        let kind = FrameKind::from_byte(kind)?;
        let mut cursor = Cursor::new(payload);

        match kind {
            FrameKind::Hello => {
                let protocol_version = cursor.read_u16()?;
                if protocol_version != PROTOCOL_VERSION {
                    return Err(ProtocolError::UnsupportedVersion(protocol_version));
                }

                let hello = Hello {
                    protocol_version,
                    width_px: cursor.read_u32()?,
                    height_px: cursor.read_u32()?,
                    max_pressure: cursor.read_u16()?,
                    device_name: cursor.read_string()?,
                };
                cursor.finish()?;
                Ok(Self::Hello(hello))
            }
            FrameKind::PenDown | FrameKind::PenMove | FrameKind::PenUp | FrameKind::PenHover => {
                let phase = match kind {
                    FrameKind::PenDown => PenPhase::Down,
                    FrameKind::PenMove => PenPhase::Move,
                    FrameKind::PenUp => PenPhase::Up,
                    FrameKind::PenHover => PenPhase::Hover,
                    _ => unreachable!(),
                };
                let event = PenEvent {
                    phase,
                    timestamp_us: cursor.read_u64()?,
                    x_norm: cursor.read_f32()?.clamp(0.0, 1.0),
                    y_norm: cursor.read_f32()?.clamp(0.0, 1.0),
                    pressure: cursor.read_f32()?.clamp(0.0, 1.0),
                    tilt_x: cursor.read_f32()?.clamp(-1.0, 1.0),
                    tilt_y: cursor.read_f32()?.clamp(-1.0, 1.0),
                    buttons: cursor.read_u8()?,
                    sequence: cursor.read_u32()?,
                };
                cursor.finish()?;
                Ok(Self::Pen(event))
            }
            FrameKind::Button => {
                let event = ButtonEvent {
                    timestamp_us: cursor.read_u64()?,
                    buttons: cursor.read_u8()?,
                    sequence: cursor.read_u32()?,
                };
                cursor.finish()?;
                Ok(Self::Button(event))
            }
            FrameKind::Heartbeat => {
                let timestamp_us = cursor.read_u64()?;
                cursor.finish()?;
                Ok(Self::Heartbeat { timestamp_us })
            }
            FrameKind::Config => {
                let config = Config {
                    display_width: cursor.read_u32()?,
                    display_height: cursor.read_u32()?,
                    mapping_mode: match cursor.read_u8()? {
                        0 => MappingMode::Absolute,
                        1 => MappingMode::Relative,
                        _ => return Err(ProtocolError::InvalidPayload("unknown mapping mode")),
                    },
                    preserve_aspect_ratio: cursor.read_u8()? != 0,
                };
                cursor.finish()?;
                Ok(Self::Config(config))
            }
        }
    }
}

pub struct FrameReader<R> {
    reader: R,
}

impl<R: Read> FrameReader<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn read_frame(&mut self) -> Result<Frame, ProtocolError> {
        let mut header = [0u8; HEADER_LEN];
        self.reader.read_exact(&mut header)?;

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&header[..4]);
        if &magic != MAGIC {
            return Err(ProtocolError::BadMagic(magic));
        }

        let version = header[4] as u16;
        if version != PROTOCOL_VERSION {
            return Err(ProtocolError::UnsupportedVersion(version));
        }

        let length = u16::from_be_bytes([header[6], header[7]]) as u32;
        if length > MAX_FRAME_LEN {
            return Err(ProtocolError::InvalidLength(length));
        }

        let mut payload = vec![0u8; length as usize];
        self.reader.read_exact(&mut payload)?;
        Frame::decode(header[5], &payload)
    }
}

pub fn write_frame<W: Write>(writer: &mut W, frame: &Frame) -> io::Result<()> {
    writer.write_all(&frame.encode())
}

pub fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_micros()
        .min(u128::from(u64::MAX)) as u64
}

fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn write_string(out: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    write_u16(out, bytes.len().min(u16::MAX as usize) as u16);
    out.extend_from_slice(&bytes[..bytes.len().min(u16::MAX as usize)]);
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn finish(self) -> Result<(), ProtocolError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(ProtocolError::InvalidPayload("trailing payload bytes"))
        }
    }

    fn read_u8(&mut self) -> Result<u8, ProtocolError> {
        let bytes = self.read_exact(1)?;
        Ok(bytes[0])
    }

    fn read_u16(&mut self) -> Result<u16, ProtocolError> {
        let bytes = self.read_exact(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, ProtocolError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, ProtocolError> {
        let bytes = self.read_exact(8)?;
        Ok(u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn read_f32(&mut self) -> Result<f32, ProtocolError> {
        Ok(f32::from_bits(self.read_u32()?))
    }

    fn read_string(&mut self) -> Result<String, ProtocolError> {
        let len = self.read_u16()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| ProtocolError::InvalidPayload("utf8"))
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], ProtocolError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or(ProtocolError::InvalidPayload("offset overflow"))?;
        if end > self.bytes.len() {
            return Err(ProtocolError::InvalidPayload("payload too short"));
        }
        let bytes = &self.bytes[self.offset..end];
        self.offset = end;
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pen_frame_round_trips() {
        let frame = Frame::Pen(PenEvent {
            phase: PenPhase::Move,
            timestamp_us: 42,
            x_norm: 0.25,
            y_norm: 0.75,
            pressure: 0.5,
            tilt_x: -0.1,
            tilt_y: 0.2,
            buttons: 1,
            sequence: 99,
        });

        let encoded = frame.encode();
        let decoded = Frame::decode(encoded[5], &encoded[HEADER_LEN..]).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn hello_frame_round_trips_through_reader() {
        let frame = Frame::Hello(Hello {
            protocol_version: PROTOCOL_VERSION,
            device_name: "iPad Pro".into(),
            width_px: 2732,
            height_px: 2048,
            max_pressure: 4096,
        });
        let bytes = frame.encode();
        let mut reader = FrameReader::new(bytes.as_slice());
        assert_eq!(reader.read_frame().unwrap(), frame);
    }
}
