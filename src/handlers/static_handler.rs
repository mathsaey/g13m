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

//! Maps each G13 key, mode pair to a fixed bind and each mode to a color.
//!
//! Create a [`StaticHandler`] with [`StaticHandler::new`].

use std::fmt::Debug;
use std::io;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use bitflags::bitflags;

use crate::virtual_keyboard::{Bind, VirtualKeyboard};
use crate::{DeviceHandler, Error, HandledDeviceRef, Handler, Rgb};

/// Keybinds for each key.
///
/// This is an array of arrays of `Option<Bind>`s. The outer array contains 3 items. The first (at
/// index 0) contains the keybinds for M1. The second (index 1) for M2, the third (index 2) for M3.
/// The inner array contains a [`Bind`] for each key. The meaning of each index is shown in the
/// table below.
///
/// | Index | Key |
/// | ----- | --- |
/// | 0     | G1  |
/// | 1     | G2  |
/// | ...   | ... |
/// | 21    | G22 |
/// | 22    | left thumb key |
/// | 23    | bottom thumb key |
/// | 24    | joystick key |
/// | 25    | joystick up |
/// | 26    | joystick down |
/// | 27    | joystick left |
/// | 28    | joystick right |
pub type Binds = [[Option<Bind>; 29]; 3];

/// Colors for each mode.
///
/// This is an array of `Option<Rgb>`. The array contains 3 items, each of which represents the
/// [`Rgb`] color of a mode.
pub type Colors = [Option<Rgb>; 3];

const JS_MAX: i32 = 127;
const JS_DEADZONE_SIZE: i32 = 75;
const JS_DEADZONE: Range<i32> = (JS_MAX - JS_DEADZONE_SIZE)..(JS_MAX + JS_DEADZONE_SIZE);

#[derive(Debug)]
pub struct StaticHandler {
    keyboard: Arc<Mutex<VirtualKeyboard>>,
    binds: Arc<Binds>,
    colors: Colors,
}

#[derive(Clone, Debug)]
pub struct StaticDeviceHandler {
    keyboard: Arc<Mutex<VirtualKeyboard>>,
    binds: Arc<Binds>,
    colors: Colors,

    held: HeldDirections,
    device_ref: HandledDeviceRef,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct HeldDirections: u8 {
        const UP = 1 << 0;
        const DOWN = 1 << 1;
        const LEFT = 1 << 2;
        const RIGHT = 1 << 3;
    }
}

impl StaticHandler {
    /// Create a static handler.
    ///
    /// A static handler is created with a set of [`Binds`] and [`Colors`] which determine how it
    /// maps the various keys and modes. Please refer to the documentation of each type for more
    /// information.
    pub fn new(keyboard: Arc<Mutex<VirtualKeyboard>>, binds: Binds, colors: Colors) -> Self {
        StaticHandler {
            keyboard,
            binds: Arc::new(binds),
            colors,
        }
    }
}

impl Handler for StaticHandler {
    #[allow(refining_impl_trait)]
    fn handler_for_device(&self, device_ref: HandledDeviceRef) -> StaticDeviceHandler {
        device_ref.set_mode(1);
        if let Some(color) = self.colors[0] {
            device_ref
                .set_backlight(color)
                .unwrap_or_else(backlight_error);
        }

        StaticDeviceHandler {
            keyboard: self.keyboard.clone(),
            binds: self.binds.clone(),
            colors: self.colors,
            held: HeldDirections::empty(),
            device_ref,
        }
    }
}

impl StaticDeviceHandler {
    fn lookup_bind(&self, idx: u16) -> Option<Bind> {
        self.binds[(self.device_ref.mode() - 1) as usize][idx as usize]
    }

    fn exec_bind_action(
        &mut self,
        key: u16,
        action: fn(&mut VirtualKeyboard, Bind) -> io::Result<()>,
    ) {
        if let Some(bind) = self.lookup_bind(key - 1) {
            let mut keyboard = self.keyboard.lock().unwrap();
            action(&mut keyboard, bind).unwrap_or_else(|err| virtual_keyboard_error(err, bind));
        }
    }

    fn check_axis(&mut self, val: i32, top_flag: HeldDirections, bottom_flag: HeldDirections) {
        let previous = self.held.intersection(bottom_flag | top_flag);
        let mut next = HeldDirections::empty();

        if val < JS_DEADZONE.start {
            next.insert(top_flag)
        } else if val > JS_DEADZONE.end {
            next.insert(bottom_flag)
        }

        if next != previous {
            let to_press = next.difference(previous);
            let to_release = previous.difference(next);

            self.held.insert(to_press);
            self.held.remove(to_release);

            self.axis_action(to_press, VirtualKeyboard::key_down);
            self.axis_action(to_release, VirtualKeyboard::key_up);
        }
    }

    fn axis_action(
        &mut self,
        flags: HeldDirections,
        action: fn(&mut VirtualKeyboard, Bind) -> io::Result<()>,
    ) {
        let mut keyboard = self.keyboard.lock().unwrap();

        flags
            .iter()
            .filter_map(|flag| {
                let idx = match flag {
                    HeldDirections::UP => 25,
                    HeldDirections::DOWN => 26,
                    HeldDirections::LEFT => 27,
                    HeldDirections::RIGHT => 28,
                    any => panic!("Invalid flag: {any:?}"),
                };
                self.lookup_bind(idx)
            })
            .for_each(|bind| {
                action(&mut keyboard, bind).unwrap_or_else(|err| virtual_keyboard_error(err, bind))
            })
    }
}

impl DeviceHandler for StaticDeviceHandler {
    fn g_key_down(&mut self, key: u16) {
        self.exec_bind_action(key, VirtualKeyboard::key_down);
    }

    fn g_key_up(&mut self, key: u16) {
        self.exec_bind_action(key, VirtualKeyboard::key_up);
    }

    fn m_key_down(&mut self, key: u16) {
        if let Some(color) = self.colors[(key - 1) as usize] {
            self.device_ref
                .set_backlight(color)
                .unwrap_or_else(backlight_error);
        }

        self.axis_action(self.held, VirtualKeyboard::key_up);
        self.held = HeldDirections::empty();
    }

    fn x_axis(&mut self, value: i32) {
        self.check_axis(value, HeldDirections::LEFT, HeldDirections::RIGHT)
    }

    fn y_axis(&mut self, value: i32) {
        self.check_axis(value, HeldDirections::UP, HeldDirections::DOWN)
    }
}

fn virtual_keyboard_error(error: io::Error, bind: Bind) {
    log::error!(
        "Could not press key ({:} {:}): {:}",
        bind.0,
        bind.1.0,
        error
    );
}

fn backlight_error(err: Error) {
    log::error!("Could not change backlight: {}", err);
}
