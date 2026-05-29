// g13m
// Copyright (c) 2026, Mathijs Saey

// g13m is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// g13m is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even
// the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with this program.  If
// not, see <http://www.gnu.org/licenses/>.

use std::fs::File;
use std::io::Write;
use std::ops::RangeInclusive;
use std::os::fd::AsFd;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::{fmt, io};

use async_io::Async;
use futures_lite::future;

use evdev::{EventType, InputEvent};

use crate::{DeviceHandler, Handler};

const PRODUCT_ID: u16 = 0xc21c;
const VENDOR_ID: u16 = 0x46d;

const IGNORED_KEYS: [u16; 7] = [
    700,   // Change display function
    696,   // Display fn 1
    697,   // Display fn 2
    698,   // Display fn 3
    699,   // Display fn 4
    0x21e, // LIGHTS_TOGGLE
    688,   // Macro record
];

const M_KEYS: RangeInclusive<u16> = 691..=693;
const G_KEYS: RangeInclusive<u16> = 656..=677;

const TOP_THUMB_KEY: u16 = 294;
const BTM_THUMB_KEY: u16 = 295;
const JOYSTICK_KEY: u16 = 289;

/// An RGB color
#[derive(Debug, Clone, Copy)]
pub struct Rgb(pub u8, pub u8, pub u8);

/// A single G13 device.
///
/// Once created, the main use of this struct is to be turned into an event loop through the use of
/// [`Device::into_event_loop`].
#[derive(Debug)]
pub struct Device {
    syspath: PathBuf,
    thumbstick: evdev::Device,
    keypad: evdev::Device,
    m_leds: [RedLed; 3],
    mr_led: RedLed,
    backlight: BackLight,
}

/// Reference to a G13 device processed by an event loop and a [`crate::DeviceHandler`].
///
/// This reference is automatically created by [`Device::into_event_loop`], which passes it to
/// [`crate::Handler::handler_for_device`]. The handler may store a clone of this reference in the
/// [`crate::DeviceHandler`], which can use it to interact with the device.
///
/// Behind the scenes, this references uses a [`Mutex`] wrapped in a [`Arc`] to handle access to
/// the shared device data. Note that the handled device may still exist when the device backing it
/// no longer does (e.g. when the device is unplugged).
#[derive(Clone, Debug)]
pub struct HandledDeviceRef {
    syspath: PathBuf,
    device_ref: Arc<Mutex<HandledDevice>>,
}

#[derive(Debug)]
struct HandledDevice {
    mode: u8,
    m_leds: [RedLed; 3],
    _mr_led: RedLed,
    backlight: BackLight,
}

#[derive(Debug)]
struct RedLed(File);

#[derive(Debug)]
struct BackLight(File);

/// Errors which may occur when creating a G13 [`Device`].
#[derive(Debug)]
pub enum Error {
    /// The device is not a G13
    NoG13,

    /// Could not find the needed files in sysfs
    ///
    /// This _may_ mean the project is no longer up to date with the entries the kernel driver adds
    /// to sysfs.
    InvalidChildren,

    /// A generic [`std::io::Error`] occurred.
    Io(io::Error),
}

// -------------- //
// Error Handling //
// -------------- //

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => err.fmt(f),
            Error::NoG13 => write!(f, "not a G13"),
            Error::InvalidChildren => {
                write!(f, "could not find child devices")
            }
        }
    }
}

// ----------- //
// Led Devices //
// ----------- //

impl RedLed {
    fn new(path: &Path) -> io::Result<Self> {
        let path = path.join("brightness");
        Ok(RedLed(File::create(path)?))
    }

    fn on(&mut self) -> io::Result<()> {
        self.0.write_all(b"1")?;
        Ok(())
    }

    fn off(&mut self) -> io::Result<()> {
        self.0.write_all(b"0")?;
        Ok(())
    }
}

impl BackLight {
    fn new(path: &Path) -> io::Result<Self> {
        let path = path.join("multi_intensity");
        Ok(Self(File::create(path)?))
    }

    /// Set the RGB value of the backlight.
    fn set(&mut self, Rgb(red, green, blue): Rgb) -> io::Result<()> {
        let data = format!("{} {} {}", red, green, blue);
        self.0.write_all(data.as_bytes())
    }
}

// ---------------- //
// Device Discovery //
// ---------------- //

/// List all connected G13 devices.
///
/// Creates a vector of paths which point to G13 usb interfaces in sysfs. These paths can be used
/// by [`Device::from_syspath`] to create a [`Device`] struct.
pub fn list() -> io::Result<Vec<PathBuf>> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_is_initialized()?;
    enumerator.match_subsystem("usb")?;

    Ok(enumerator
        .scan_devices()?
        .filter(|dev| {
            dev.devtype()
                .is_some_and(|devtype| devtype == "usb_interface")
        })
        .filter(is_g13)
        .map(|dev| {
            log::info!("Found G13: {:}", dev.syspath().display());
            dev.syspath().to_path_buf()
        })
        .collect())
}

/// Create a future that will scan for G13 devices.
///
/// When executed, the future will wake up any time a new usb interface appears in sysfs. If the
/// usb interface is a G13, the provided closure will be executed with the path to the usb
/// interface. This path can be used by [`Device::from_syspath`] to create a new [`Device`]. This
/// function ensures that the device has been initialized by the kernel _before_ the closure is
/// called.
pub async fn discovery_loop<T>(mut action: T) -> io::Result<()>
where
    T: FnMut(&Path),
{
    let socket = Async::new(
        udev::MonitorBuilder::new()?
            .match_subsystem_devtype("usb", "usb_interface")?
            .listen()?,
    )?;

    log::info!("Starting device discovery loop");
    loop {
        socket.readable().await?;

        socket
            .get_ref()
            .iter()
            .filter(|event| event.event_type() == udev::EventType::Bind)
            .map(|event| event.device())
            .filter(is_g13)
            .for_each(|dev| {
                log::info!("Found G13: {:}", dev.syspath().display());
                action(dev.syspath())
            })
    }
}

fn attribute_as_u16(dev: &udev::Device, id: &str) -> Option<u16> {
    dev.attribute_value(id)
        .and_then(|s| s.to_str())
        .and_then(|s| u16::from_str_radix(s, 16).ok())
}

fn is_g13(parent: &udev::Device) -> bool {
    match attribute_as_u16(parent, "idVendor").zip(attribute_as_u16(parent, "idProduct")) {
        Some((vid, pid)) if vid == VENDOR_ID && pid == PRODUCT_ID => true,
        Some((_, _)) => false,
        None => parent
            .parent()
            .is_some_and(|grandparent| is_g13(&grandparent)),
    }
}

fn find_input_devices(parent: &udev::Device) -> Result<(evdev::Device, evdev::Device), Error> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_parent(parent)?;
    enumerator.match_is_initialized()?;
    enumerator.match_subsystem("input")?;

    let (thumbstick, keypad) = enumerator
        .scan_devices()?
        .filter(|d| d.sysname().to_str().is_some_and(|s| s.starts_with("event")))
        .collect::<Vec<_>>()
        .get(0..2)
        .and_then(|slice| {
            if is_joystick(&slice[0]) {
                Some((&slice[0], &slice[1]))
            } else if is_joystick(&slice[1]) {
                Some((&slice[1], &slice[0]))
            } else {
                None
            }
        })
        .and_then(|(thumbstick, keypad)| {
            Some((
                thumbstick.devnode()?.to_owned(),
                keypad.devnode()?.to_owned(),
            ))
        })
        .ok_or(Error::InvalidChildren)?;

    log::debug!("Found thumbstick: {:}", thumbstick.display());
    log::debug!("Found keypad: {:}", keypad.display());

    Ok((
        evdev::Device::open(thumbstick)?,
        evdev::Device::open(keypad)?,
    ))
}

fn is_joystick(dev: &udev::Device) -> bool {
    dev.property_value("ID_INPUT_JOYSTICK")
        .is_some_and(|s| s == "1")
}

fn find_leds(parent: &udev::Device) -> Result<([RedLed; 3], RedLed, BackLight), Error> {
    let mut enumerator = udev::Enumerator::new()?;
    enumerator.match_parent(parent)?;
    enumerator.match_is_initialized()?;
    enumerator.match_subsystem("leds")?;

    let mut ms: [Option<RedLed>; 3] = [None, None, None];
    let mut mr: Option<RedLed> = None;
    let mut bg: Option<BackLight> = None;

    for dev in enumerator.scan_devices()? {
        match dev.sysnum() {
            Some(i) => {
                log::debug!("Found macro led {}: {:}", i, dev.syspath().display());
                ms[i - 1] = Some(RedLed::new(dev.syspath())?)
            }
            None if dev.sysname() == "g13:red:macro_record" => {
                log::debug!("Found macro record led: {:}", dev.syspath().display());
                mr = Some(RedLed::new(dev.syspath())?)
            }
            None if dev.sysname() == "g13:rgb:kbd_backlight" => {
                log::debug!("Found keyboard led: {:}", dev.syspath().display());
                bg = Some(BackLight::new(dev.syspath())?)
            }
            None => Err(Error::InvalidChildren)?,
        }
    }

    let ms = ms
        .into_iter()
        .map(|m| m.ok_or(Error::InvalidChildren))
        .collect::<Result<Vec<RedLed>, Error>>()?
        .try_into()
        .map_err(|_| Error::InvalidChildren);

    Ok((
        ms?,
        mr.ok_or(Error::InvalidChildren)?,
        bg.ok_or(Error::InvalidChildren)?,
    ))
}

// ---------------- //
// Event Processing //
// ---------------- //

async fn device_loop(
    mut raw_device: evdev::Device,
    device_ref: HandledDeviceRef,
    mut handler: impl DeviceHandler,
) -> io::Result<()> {
    // Avoid other applications responding to events
    raw_device.grab()?;

    let poller = Async::new(raw_device.as_fd().try_clone_to_owned()?)?;
    loop {
        poller.readable().await?;

        for event in raw_device.fetch_events()? {
            log::trace!(
                "Processing event: {:?} with device: {:?}",
                event,
                device_ref.syspath.display()
            );
            process_event(event, device_ref.clone(), &mut handler);
        }
    }
}

fn process_event(
    event: InputEvent,
    device_ref: HandledDeviceRef,
    handler: &mut impl DeviceHandler,
) {
    match event.event_type() {
        EventType::KEY if IGNORED_KEYS.contains(&event.code()) => {
            log::debug!("Ignoring key: {:}", event.code())
        }
        EventType::KEY if M_KEYS.contains(&event.code()) => {
            handle_m_key(event.value(), event.code(), device_ref, handler)
        }
        EventType::KEY => handle_g_key(event.value(), event.code(), handler),
        EventType::ABSOLUTE => handle_joystick(event.value(), event.code(), handler),
        EventType::SYNCHRONIZATION => (),
        _ => log::warn!("Received unexpected event type {:?}", event),
    }
}

fn handle_m_key(
    value: i32,
    key: u16,
    device_ref: HandledDeviceRef,
    handler: &mut impl DeviceHandler,
) {
    let m_idx = key - M_KEYS.start() + 1;

    match value {
        0 => handler.m_key_up(m_idx),
        1 => {
            handler.m_key_down(m_idx);
            device_ref.set_mode(m_idx);
        }
        _ => panic!("Invalid key event value"),
    }
}

fn handle_g_key(value: i32, key: u16, handler: &mut impl DeviceHandler) {
    let g_idx = match key {
        TOP_THUMB_KEY => 23,
        BTM_THUMB_KEY => 24,
        JOYSTICK_KEY => 25,
        key if G_KEYS.contains(&key) => key - G_KEYS.start() + 1,
        _ => panic!("Unsupported key code"),
    };

    match value {
        0 => handler.g_key_up(g_idx),
        1 => handler.g_key_down(g_idx),
        _ => panic!("Invalid key event value"),
    }
}

fn handle_joystick(value: i32, code: u16, handler: &mut impl DeviceHandler) {
    match code {
        0 => handler.x_axis(value),
        1 => handler.y_axis(value),
        _ => panic!("Invalid joystick axis"),
    }
}

// -------------- //
// Device & State //
// -------------- //

impl Device {
    /// Create a [`Device`] from a path which points to a G13 device.
    ///
    /// This function creates a [`Device`] struct representing a physical G13 device based on a
    /// path which points to a device in sysfs. The provided path may point to a usb device, a
    /// usb interface or a hid device. Such a path can be obtained by calling [`list`]; it is also
    /// passed to the [`discovery_loop`] closure.
    ///
    /// Based on this path, this function will search for the various child devices (leds, lcd,
    /// input, ...) which constitute the G13 and wrap them in a [`Device`] struct.
    pub fn from_syspath(path: &Path) -> Result<Self, Error> {
        let parent = udev::Device::from_syspath(path)?;
        Self::from_parent(&parent)
    }

    fn from_parent(parent: &udev::Device) -> Result<Self, Error> {
        if !is_g13(parent) {
            return Err(Error::NoG13);
        };

        let syspath = parent.syspath().to_path_buf();
        let (thumbstick, keypad) = find_input_devices(parent)?;
        let (m_leds, mr_led, backlight) = find_leds(parent)?;

        Ok(Self {
            syspath,
            thumbstick,
            keypad,
            m_leds,
            mr_led,
            backlight,
        })
    }

    /// Turn the [`Device`] into an asynchronous event loop.
    ///
    /// This method consumes the device and returns a future. Executing the future runs several
    /// event loops, which will trigger when the G13 is used (e.g., when a key is pressed). When
    /// such an event occurs, the event loop will invoke the [`crate::DeviceHandler`] created by
    /// the [`crate::Handler`] passed as an argument to this method.
    ///
    /// [`crate::Handler::handler_for_device`] will be called before the event loops are started.
    /// This function will be called with a [`HandledDeviceRef`], which can be used to interact
    /// with the g13 while the event loop is running.
    pub async fn into_event_loop(self, handler: &impl Handler) -> io::Result<()> {
        let device_ref = HandledDeviceRef {
            syspath: self.syspath,
            device_ref: Arc::new(Mutex::new(HandledDevice {
                mode: 0,
                m_leds: self.m_leds,
                _mr_led: self.mr_led,
                backlight: self.backlight,
            })),
        };
        device_ref.reset();

        let device_handler = handler.handler_for_device(device_ref.clone());
        let result = future::try_zip(
            device_loop(self.thumbstick, device_ref.clone(), device_handler.clone()),
            device_loop(self.keypad, device_ref.clone(), device_handler.clone()),
        )
        .await;

        match result {
            Err(e) if e.raw_os_error().is_some_and(|e| e == 19) => {
                log::info!("G13 disconnected: {:}", device_ref.syspath.display());
                Ok(())
            }
            Ok(((), ())) => Ok(()),
            Err(e) => Err(e),
        }
    }
}

impl HandledDeviceRef {
    fn dev(&self) -> MutexGuard<'_, HandledDevice> {
        self.device_ref.lock().unwrap()
    }

    fn reset(&self) {
        for mode in 1..=3 {
            self.set_mode_led(mode, RedLed::off);
        }
        self.set_mode(1);
    }

    /// Read the path of the handled device.
    ///
    /// This is mainly useful to have a unique way to refer to the device (e.g. for logging or
    /// error messages).
    ///
    /// Calling this function does not lock the device.
    pub fn syspath(&self) -> &Path {
        &self.syspath
    }

    /// Get access to the current mode (m-key) the G13 is in.
    ///
    /// This always returns a number between 1 and 3.
    ///
    /// Calling this function locks the device.
    pub fn mode(&self) -> u8 {
        self.dev().mode + 1
    }

    /// Change the backlight of the G13.
    ///
    /// Calling this function locks the device.
    pub fn set_backlight(&self, color: Rgb) -> Result<(), Error> {
        Ok(self.dev().backlight.set(color)?)
    }

    /// Change the mode of the G13.
    ///
    /// Changes the (M-key) mode the G13 is in. Note that this function should _not_ be invoked
    /// when an M key is pressed, as the device loop ensures this happens.
    ///
    /// Instead, this function can be used to change the G13's mode from a script or in response to
    /// the press of a G key.
    ///
    /// Calling this function locks the device.
    pub fn set_mode(&self, m_key: u16) {
        log::trace!("Changing mode: {} -> {}", self.mode(), m_key);
        let idx = (m_key - 1) as usize;

        self.set_mode_led(self.mode(), RedLed::off);
        self.dev().mode = idx as u8;
        self.set_mode_led(self.mode(), RedLed::on);
    }

    fn set_mode_led(&self, mode: u8, action: fn(&mut RedLed) -> io::Result<()>) {
        action(&mut self.dev().m_leds[(mode - 1) as usize]).unwrap_or_else(|err| {
            log::error!("Could not set LED: {}", err);
        })
    }
}
