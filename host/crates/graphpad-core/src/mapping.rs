//! Coordinate and pressure mapping from normalized iPad input to tablet axes.

use crate::protocol::{MappingMode, PenEvent, PressureCurve, PRESSURE_MAX, TABLET_AXIS_MAX, TILT_MAX};

/// Maps normalized pen coordinates to virtual tablet absolute axes.
#[derive(Debug, Clone)]
pub struct TabletMapper {
    pub mapping_mode: MappingMode,
    pub pressure_curve: PressureCurve,
    pub area_left: f32,
    pub area_top: f32,
    pub area_right: f32,
    pub area_bottom: f32,
    pub tablet_width: i32,
    pub tablet_height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MappedPenState {
    pub x: i32,
    pub y: i32,
    pub pressure: i32,
    pub tilt_x: i32,
    pub tilt_y: i32,
    pub in_contact: bool,
    pub hovering: bool,
    pub stylus_primary: bool,
    pub stylus_secondary: bool,
    pub eraser: bool,
}

impl Default for TabletMapper {
    fn default() -> Self {
        Self {
            mapping_mode: MappingMode::Absolute,
            pressure_curve: PressureCurve::Linear,
            area_left: 0.0,
            area_top: 0.0,
            area_right: 1.0,
            area_bottom: 1.0,
            tablet_width: TABLET_AXIS_MAX,
            tablet_height: TABLET_AXIS_MAX,
        }
    }
}

impl TabletMapper {
    pub fn map_pen_event(&self, event: &PenEvent, in_contact: bool, hovering: bool) -> MappedPenState {
        let x_clamped = clamp_norm(map_area_x(event.x_norm, self.area_left, self.area_right));
        let y_clamped = clamp_norm(map_area_y(event.y_norm, self.area_top, self.area_bottom));

        let x = (x_clamped * self.tablet_width as f32).round() as i32;
        let y = (y_clamped * self.tablet_height as f32).round() as i32;

        let pressure = map_pressure(event.pressure, self.pressure_curve);
        let tilt_x = (event.tilt_x.clamp(-1.0, 1.0) * TILT_MAX as f32).round() as i32;
        let tilt_y = (event.tilt_y.clamp(-1.0, 1.0) * TILT_MAX as f32).round() as i32;

        let buttons = event.buttons;
        MappedPenState {
            x: x.clamp(0, self.tablet_width),
            y: y.clamp(0, self.tablet_height),
            pressure: pressure.clamp(0, PRESSURE_MAX),
            tilt_x: tilt_x.clamp(-TILT_MAX, TILT_MAX),
            tilt_y: tilt_y.clamp(-TILT_MAX, TILT_MAX),
            in_contact,
            hovering,
            stylus_primary: buttons & crate::protocol::buttons::STYLUS_PRIMARY != 0,
            stylus_secondary: buttons & crate::protocol::buttons::STYLUS_SECONDARY != 0,
            eraser: buttons & crate::protocol::buttons::ERASER != 0,
        }
    }
}

fn clamp_norm(value: f32) -> f32 {
    value.clamp(0.0, 1.0)
}

fn map_area_x(x: f32, left: f32, right: f32) -> f32 {
    if (right - left).abs() < f32::EPSILON {
        return 0.0;
    }
    (x - left) / (right - left)
}

fn map_area_y(y: f32, top: f32, bottom: f32) -> f32 {
    if (bottom - top).abs() < f32::EPSILON {
        return 0.0;
    }
    (y - top) / (bottom - top)
}

fn map_pressure(pressure: f32, curve: PressureCurve) -> i32 {
    let p = pressure.clamp(0.0, 1.0);
    let curved = match curve {
        PressureCurve::Linear => p,
        PressureCurve::Soft => p * p,
        PressureCurve::Firm => p.sqrt(),
    };
    (curved * PRESSURE_MAX as f32).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_center_with_full_pressure() {
        let mapper = TabletMapper::default();
        let event = PenEvent {
            timestamp_us: 0,
            x_norm: 0.5,
            y_norm: 0.5,
            pressure: 1.0,
            tilt_x: 0.0,
            tilt_y: 0.0,
            buttons: 0,
            sequence: 1,
        };
        let mapped = mapper.map_pen_event(&event, true, false);
        assert!((mapped.x - TABLET_AXIS_MAX / 2).abs() <= 1);
        assert_eq!(mapped.pressure, PRESSURE_MAX);
    }
}
