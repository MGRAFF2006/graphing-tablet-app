use crate::protocol::PenEvent;

pub const DEFAULT_TABLET_MAX_X: i32 = 65_535;
pub const DEFAULT_TABLET_MAX_Y: i32 = 65_535;
pub const DEFAULT_PRESSURE_MAX: i32 = 4_096;
pub const DEFAULT_TILT_MIN: i32 = -60;
pub const DEFAULT_TILT_MAX: i32 = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TabletExtents {
    pub x_min: i32,
    pub x_max: i32,
    pub y_min: i32,
    pub y_max: i32,
    pub pressure_min: i32,
    pub pressure_max: i32,
    pub tilt_min: i32,
    pub tilt_max: i32,
}

impl Default for TabletExtents {
    fn default() -> Self {
        Self {
            x_min: 0,
            x_max: DEFAULT_TABLET_MAX_X,
            y_min: 0,
            y_max: DEFAULT_TABLET_MAX_Y,
            pressure_min: 0,
            pressure_max: DEFAULT_PRESSURE_MAX,
            tilt_min: DEFAULT_TILT_MIN,
            tilt_max: DEFAULT_TILT_MAX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressureCurve {
    Linear,
    Soft,
    Firm,
}

impl Default for PressureCurve {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapper {
    extents: TabletExtents,
    pressure_curve: PressureCurve,
}

impl Mapper {
    pub fn new(extents: TabletExtents, pressure_curve: PressureCurve) -> Self {
        Self {
            extents,
            pressure_curve,
        }
    }

    pub fn extents(&self) -> TabletExtents {
        self.extents
    }

    pub fn map_pen_event(&self, event: PenEvent) -> MappedPenEvent {
        MappedPenEvent {
            x: scale_unit(event.x_norm, self.extents.x_min, self.extents.x_max),
            y: scale_unit(event.y_norm, self.extents.y_min, self.extents.y_max),
            pressure: self.map_pressure(event.pressure),
            tilt_x: scale_signed_unit(event.tilt_x, self.extents.tilt_min, self.extents.tilt_max),
            tilt_y: scale_signed_unit(event.tilt_y, self.extents.tilt_min, self.extents.tilt_max),
            buttons: event.buttons,
            tool_down: event.phase.tool_down(),
            hover: matches!(event.phase, crate::protocol::PenPhase::Hover),
            sequence: event.sequence,
            timestamp_us: event.timestamp_us,
        }
    }

    fn map_pressure(&self, pressure: f32) -> i32 {
        let curved = match self.pressure_curve {
            PressureCurve::Linear => pressure,
            PressureCurve::Soft => pressure.sqrt(),
            PressureCurve::Firm => pressure * pressure,
        };
        scale_unit(curved, self.extents.pressure_min, self.extents.pressure_max)
    }
}

impl Default for Mapper {
    fn default() -> Self {
        Self::new(TabletExtents::default(), PressureCurve::Linear)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappedPenEvent {
    pub x: i32,
    pub y: i32,
    pub pressure: i32,
    pub tilt_x: i32,
    pub tilt_y: i32,
    pub buttons: u8,
    pub tool_down: bool,
    pub hover: bool,
    pub sequence: u32,
    pub timestamp_us: u64,
}

fn scale_unit(value: f32, min: i32, max: i32) -> i32 {
    let value = value.clamp(0.0, 1.0);
    let span = (max - min) as f32;
    min + (value * span).round() as i32
}

fn scale_signed_unit(value: f32, min: i32, max: i32) -> i32 {
    let value = (value.clamp(-1.0, 1.0) + 1.0) * 0.5;
    scale_unit(value, min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{PenEvent, PenPhase};

    #[test]
    fn maps_normalized_values_to_default_extents() {
        let mapper = Mapper::default();
        let mapped = mapper.map_pen_event(PenEvent {
            phase: PenPhase::Move,
            timestamp_us: 1,
            x_norm: 0.5,
            y_norm: 1.0,
            pressure: 0.25,
            tilt_x: -1.0,
            tilt_y: 1.0,
            buttons: 0,
            sequence: 2,
        });

        assert_eq!(mapped.x, 32_768);
        assert_eq!(mapped.y, DEFAULT_TABLET_MAX_Y);
        assert_eq!(mapped.pressure, 1_024);
        assert_eq!(mapped.tilt_x, DEFAULT_TILT_MIN);
        assert_eq!(mapped.tilt_y, DEFAULT_TILT_MAX);
        assert!(mapped.tool_down);
    }
}
