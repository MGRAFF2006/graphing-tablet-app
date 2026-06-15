//! uinput virtual tablet implementation for Linux.

use anyhow::Context;
use evdev::uinput::VirtualDevice;
use evdev::{
    AbsoluteAxisCode, AbsoluteAxisEvent, AbsInfo, AttributeSet, BusType, InputId, KeyCode,
    KeyEvent, PropType, UinputAbsSetup,
};
use graphpad_core::mapping::MappedPenState;
use graphpad_core::session::InputBackend;
use graphpad_core::{PRESSURE_MAX, TABLET_AXIS_MAX, TILT_MAX};
use log::info;

pub struct LinuxTablet {
    device: VirtualDevice,
    last_x: i32,
    last_y: i32,
    in_contact: bool,
}

impl LinuxTablet {
    pub fn new() -> anyhow::Result<Self> {
        let mut keys = AttributeSet::<KeyCode>::new();
        keys.insert(KeyCode::BTN_TOUCH);
        keys.insert(KeyCode::BTN_STYLUS);
        keys.insert(KeyCode::BTN_STYLUS2);
        keys.insert(KeyCode::BTN_TOOL_PEN);
        keys.insert(KeyCode::BTN_TOOL_RUBBER);

        let abs_axes = [
            (AbsoluteAxisCode::ABS_X, 0, TABLET_AXIS_MAX),
            (AbsoluteAxisCode::ABS_Y, 0, TABLET_AXIS_MAX),
            (AbsoluteAxisCode::ABS_PRESSURE, 0, PRESSURE_MAX),
            (AbsoluteAxisCode::ABS_TILT_X, -TILT_MAX, TILT_MAX),
            (AbsoluteAxisCode::ABS_TILT_Y, -TILT_MAX, TILT_MAX),
            (AbsoluteAxisCode::ABS_DISTANCE, 0, 255),
        ];

        let mut builder = VirtualDevice::builder()
            .context("failed to open /dev/uinput — ensure your user is in the 'input' group")?
            .name("GraphPad Virtual Tablet")
            .input_id(InputId::new(BusType::BUS_USB, 0x28bd, 0x0901, 1))
            .with_keys(&keys)?;

        for (code, min, max) in abs_axes {
            let setup = UinputAbsSetup::new(
                code,
                AbsInfo::new(0, min, max, 0, 0, 0),
            );
            builder = builder.with_absolute_axis(&setup)?;
        }

        let mut properties = AttributeSet::<PropType>::new();
        properties.insert(PropType(0x01)); // INPUT_PROP_DIRECT
        builder = builder.with_properties(&properties)?;

        let mut device = builder.build()?;

        if let Ok(nodes) = device.enumerate_dev_nodes_blocking() {
            for node in nodes {
                if let Ok(path) = node {
                    info!("Created virtual tablet at {}", path.display());
                }
            }
        }

        Ok(Self {
            device,
            last_x: 0,
            last_y: 0,
            in_contact: false,
        })
    }

    fn emit_events(&mut self, events: &[evdev::InputEvent]) -> anyhow::Result<()> {
        self.device
            .emit(events)
            .context("failed to emit uinput events")
    }
}

impl InputBackend for LinuxTablet {
    fn apply_state(&mut self, state: MappedPenState) -> anyhow::Result<()> {
        let tool_pen = state.in_contact || state.hovering;

        let events = [
            *KeyEvent::new(
                KeyCode::BTN_TOOL_PEN,
                if tool_pen && !state.eraser { 1 } else { 0 },
            ),
            *KeyEvent::new(
                KeyCode::BTN_TOOL_RUBBER,
                if tool_pen && state.eraser { 1 } else { 0 },
            ),
            *KeyEvent::new(KeyCode::BTN_TOUCH, if state.in_contact { 1 } else { 0 }),
            *KeyEvent::new(
                KeyCode::BTN_STYLUS,
                if state.stylus_primary { 1 } else { 0 },
            ),
            *KeyEvent::new(
                KeyCode::BTN_STYLUS2,
                if state.stylus_secondary { 1 } else { 0 },
            ),
            *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_X, state.x),
            *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_Y, state.y),
            *AbsoluteAxisEvent::new(
                AbsoluteAxisCode::ABS_PRESSURE,
                if state.in_contact { state.pressure } else { 0 },
            ),
            *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_TILT_X, state.tilt_x),
            *AbsoluteAxisEvent::new(AbsoluteAxisCode::ABS_TILT_Y, state.tilt_y),
            *AbsoluteAxisEvent::new(
                AbsoluteAxisCode::ABS_DISTANCE,
                if state.hovering {
                    10
                } else if state.in_contact {
                    0
                } else {
                    255
                },
            ),
        ];

        self.last_x = state.x;
        self.last_y = state.y;
        self.in_contact = state.in_contact;

        self.emit_events(&events)
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        if self.in_contact {
            let state = MappedPenState {
                x: self.last_x,
                y: self.last_y,
                pressure: 0,
                tilt_x: 0,
                tilt_y: 0,
                in_contact: false,
                hovering: false,
                stylus_primary: false,
                stylus_secondary: false,
                eraser: false,
            };
            self.apply_state(state)?;
        }
        Ok(())
    }
}
