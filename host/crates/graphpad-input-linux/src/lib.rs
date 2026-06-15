use graphpad_core::mapping::{MappedPenEvent, TabletExtents};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::mem;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong};
use std::slice;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const DEVICE_NAME: &str = "GraphPad Virtual Tablet";

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const SYN_REPORT: u16 = 0x00;

const BTN_TOOL_PEN: u16 = 0x140;
const BTN_TOUCH: u16 = 0x14a;
const BTN_STYLUS: u16 = 0x14b;
const BTN_STYLUS2: u16 = 0x14c;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_PRESSURE: u16 = 0x18;
const ABS_TILT_X: u16 = 0x1a;
const ABS_TILT_Y: u16 = 0x1b;

const BUS_USB: u16 = 0x03;

const UI_DEV_CREATE: c_ulong = 0x5501;
const UI_DEV_DESTROY: c_ulong = 0x5502;
const UI_SET_EVBIT: c_ulong = 0x40045564;
const UI_SET_KEYBIT: c_ulong = 0x40045565;
const UI_SET_ABSBIT: c_ulong = 0x40045567;

extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

#[derive(Debug)]
pub struct LinuxTablet {
    file: File,
}

impl LinuxTablet {
    pub fn open(extents: TabletExtents) -> io::Result<Self> {
        let mut file = OpenOptions::new().write(true).open("/dev/uinput")?;
        let fd = file.as_raw_fd();

        set_bit(fd, UI_SET_EVBIT, EV_SYN)?;
        set_bit(fd, UI_SET_EVBIT, EV_KEY)?;
        set_bit(fd, UI_SET_EVBIT, EV_ABS)?;

        set_bit(fd, UI_SET_KEYBIT, BTN_TOOL_PEN)?;
        set_bit(fd, UI_SET_KEYBIT, BTN_TOUCH)?;
        set_bit(fd, UI_SET_KEYBIT, BTN_STYLUS)?;
        set_bit(fd, UI_SET_KEYBIT, BTN_STYLUS2)?;

        set_bit(fd, UI_SET_ABSBIT, ABS_X)?;
        set_bit(fd, UI_SET_ABSBIT, ABS_Y)?;
        set_bit(fd, UI_SET_ABSBIT, ABS_PRESSURE)?;
        set_bit(fd, UI_SET_ABSBIT, ABS_TILT_X)?;
        set_bit(fd, UI_SET_ABSBIT, ABS_TILT_Y)?;

        let user_dev = UInputUserDev::new(extents);
        write_struct(&mut file, &user_dev)?;
        unsafe_ioctl(fd, UI_DEV_CREATE, 0)?;

        // The kernel creates the input node asynchronously; a short pause avoids
        // dropping the first events on slower machines.
        thread::sleep(Duration::from_millis(50));
        Ok(Self { file })
    }

    pub fn emit_pen(&mut self, event: MappedPenEvent) -> io::Result<()> {
        let in_proximity = event.tool_down || event.hover;
        emit_key(&mut self.file, BTN_TOOL_PEN, in_proximity)?;
        emit_key(&mut self.file, BTN_TOUCH, event.tool_down)?;
        emit_key(&mut self.file, BTN_STYLUS, event.buttons & 0b0000_0001 != 0)?;
        emit_key(
            &mut self.file,
            BTN_STYLUS2,
            event.buttons & 0b0000_0010 != 0,
        )?;
        emit_abs(&mut self.file, ABS_X, event.x)?;
        emit_abs(&mut self.file, ABS_Y, event.y)?;
        emit_abs(&mut self.file, ABS_PRESSURE, event.pressure)?;
        emit_abs(&mut self.file, ABS_TILT_X, event.tilt_x)?;
        emit_abs(&mut self.file, ABS_TILT_Y, event.tilt_y)?;
        emit_syn(&mut self.file)
    }
}

impl Drop for LinuxTablet {
    fn drop(&mut self) {
        let _ = unsafe_ioctl(self.file.as_raw_fd(), UI_DEV_DESTROY, 0);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UInputUserDev {
    name: [i8; 80],
    id: InputId,
    ff_effects_max: u32,
    absmax: [i32; 64],
    absmin: [i32; 64],
    absfuzz: [i32; 64],
    absflat: [i32; 64],
}

impl UInputUserDev {
    fn new(extents: TabletExtents) -> Self {
        let mut dev = Self {
            name: [0; 80],
            id: InputId {
                bustype: BUS_USB,
                vendor: 0x1209,
                product: 0x4750,
                version: 1,
            },
            ff_effects_max: 0,
            absmax: [0; 64],
            absmin: [0; 64],
            absfuzz: [0; 64],
            absflat: [0; 64],
        };

        for (slot, byte) in DEVICE_NAME.bytes().take(dev.name.len() - 1).enumerate() {
            dev.name[slot] = byte as i8;
        }

        dev.absmin[ABS_X as usize] = extents.x_min;
        dev.absmax[ABS_X as usize] = extents.x_max;
        dev.absmin[ABS_Y as usize] = extents.y_min;
        dev.absmax[ABS_Y as usize] = extents.y_max;
        dev.absmin[ABS_PRESSURE as usize] = extents.pressure_min;
        dev.absmax[ABS_PRESSURE as usize] = extents.pressure_max;
        dev.absmin[ABS_TILT_X as usize] = extents.tilt_min;
        dev.absmax[ABS_TILT_X as usize] = extents.tilt_max;
        dev.absmin[ABS_TILT_Y as usize] = extents.tilt_min;
        dev.absmax[ABS_TILT_Y as usize] = extents.tilt_max;

        dev
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TimeVal {
    tv_sec: std::os::raw::c_long,
    tv_usec: std::os::raw::c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    time: TimeVal,
    type_: u16,
    code: u16,
    value: i32,
}

fn emit_key(file: &mut File, code: u16, pressed: bool) -> io::Result<()> {
    emit(file, EV_KEY, code, i32::from(pressed))
}

fn emit_abs(file: &mut File, code: u16, value: i32) -> io::Result<()> {
    emit(file, EV_ABS, code, value)
}

fn emit_syn(file: &mut File) -> io::Result<()> {
    emit(file, EV_SYN, SYN_REPORT, 0)
}

fn emit(file: &mut File, type_: u16, code: u16, value: i32) -> io::Result<()> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    let event = InputEvent {
        time: TimeVal {
            tv_sec: now.as_secs() as std::os::raw::c_long,
            tv_usec: now.subsec_micros() as std::os::raw::c_long,
        },
        type_,
        code,
        value,
    };
    write_struct(file, &event)
}

fn write_struct<T>(file: &mut File, value: &T) -> io::Result<()> {
    let bytes =
        unsafe { slice::from_raw_parts(value as *const T as *const u8, mem::size_of::<T>()) };
    file.write_all(bytes)
}

fn set_bit(fd: c_int, request: c_ulong, bit: u16) -> io::Result<()> {
    unsafe_ioctl(fd, request, bit as c_int)
}

fn unsafe_ioctl(fd: c_int, request: c_ulong, value: c_int) -> io::Result<()> {
    let result = unsafe { ioctl(fd, request, value) };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
