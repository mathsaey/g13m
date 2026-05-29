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

//! Virtual keyboard device for emulating keypresses
//!
//! This module defines the [`VirtualKeyboard`] struct, which can be used to create a virtual
//! keyboard device. This device can then be used to press buttons representing keyboard keypresses
//! or mouse button presses. This is used by the various [`crate::DeviceHandler`] to "press"
//! buttons in response to events occuring on the G13 device.
//!
//! ## Supported Keys
//!
//! The virtual keyboard can only be used to press certain buttons.
//! The following table lists the supported buttons, their keycodes, and the names
//! [`string_to_code`] will accept for them.
//!
//! Modifiers:
//!
//! | [`KeyCode`] | [`string_to_code`] names |
//! | ----------- | ------------------------ |
//! | [`KeyCode::KEY_LEFTSHIFT`] | shift, lshift |
//! | [`KeyCode::KEY_RIGHTSHIFT`] | rshift |
//! | [`KeyCode::KEY_LEFTALT`] | alt, lalt |
//! | [`KeyCode::KEY_RIGHTALT`] | ralt, altgr |
//! | [`KeyCode::KEY_LEFTCTRL`] | ctrl, lctrl |
//! | [`KeyCode::KEY_RIGHTCTRL`] | rctrl |
//! | [`KeyCode::KEY_LEFTMETA`] | meta, lmeta, super, lsuper |
//! | [`KeyCode::KEY_RIGHTMETA`] | rmeta, rsuper |
//!
//! Mouse buttons:
//!
//! | [`KeyCode`] | [`string_to_code`] names |
//! | ----------- | ------------------------ |
//! | [`KeyCode::BTN_LEFT`] | mouse1, mouse left |
//! | [`KeyCode::BTN_MIDDLE`] | mouse2, mouse middle |
//! | [`KeyCode::BTN_RIGHT`] | mouse3, mouse right |
//! | [`KeyCode::BTN_EXTRA`] | mouse4, mouse extra |
//! | [`KeyCode::BTN_SIDE`] | mouse5, mouse side |
//!
//! Regular keys:
//!
//! | [`KeyCode`] | [`string_to_code`] names |
//! | ----------- | ------------------------ |
//! | [`KeyCode::KEY_ESC`] | esc |
//! | [`KeyCode::KEY_ENTER`] | enter |
//! | [`KeyCode::KEY_BACKSPACE`] | backspace |
//! | [`KeyCode::KEY_TAB`] | tab |
//! | [`KeyCode::KEY_CAPSLOCK`] | capslock |
//! | [`KeyCode::KEY_SPACE`] | space |
//! | [`KeyCode::KEY_1`] | 1 |
//! | [`KeyCode::KEY_2`] | 2 |
//! | [`KeyCode::KEY_3`] | 3 |
//! | [`KeyCode::KEY_4`] | 4 |
//! | [`KeyCode::KEY_5`] | 5 |
//! | [`KeyCode::KEY_6`] | 6 |
//! | [`KeyCode::KEY_7`] | 7 |
//! | [`KeyCode::KEY_8`] | 8 |
//! | [`KeyCode::KEY_9`] | 9 |
//! | [`KeyCode::KEY_0`] | 0 |
//! | [`KeyCode::KEY_MINUS`] | - |
//! | [`KeyCode::KEY_EQUAL`] | = |
//! | [`KeyCode::KEY_Q`] | q |
//! | [`KeyCode::KEY_W`] | w |
//! | [`KeyCode::KEY_E`] | e |
//! | [`KeyCode::KEY_R`] | r |
//! | [`KeyCode::KEY_T`] | t |
//! | [`KeyCode::KEY_Y`] | y |
//! | [`KeyCode::KEY_U`] | u |
//! | [`KeyCode::KEY_I`] | i |
//! | [`KeyCode::KEY_O`] | o |
//! | [`KeyCode::KEY_P`] | p |
//! | [`KeyCode::KEY_LEFTBRACE`] | [ |
//! | [`KeyCode::KEY_RIGHTBRACE`] | ] |
//! | [`KeyCode::KEY_A`] | a |
//! | [`KeyCode::KEY_S`] | s |
//! | [`KeyCode::KEY_D`] | d |
//! | [`KeyCode::KEY_F`] | f |
//! | [`KeyCode::KEY_G`] | g |
//! | [`KeyCode::KEY_H`] | h |
//! | [`KeyCode::KEY_J`] | j |
//! | [`KeyCode::KEY_K`] | k |
//! | [`KeyCode::KEY_L`] | l |
//! | [`KeyCode::KEY_SEMICOLON`] | ; |
//! | [`KeyCode::KEY_APOSTROPHE`] | ' |
//! | [`KeyCode::KEY_GRAVE`] | ~ |
//! | [`KeyCode::KEY_BACKSLASH`] | \ |
//! | [`KeyCode::KEY_Z`] | z |
//! | [`KeyCode::KEY_X`] | x |
//! | [`KeyCode::KEY_C`] | c |
//! | [`KeyCode::KEY_V`] | v |
//! | [`KeyCode::KEY_B`] | b |
//! | [`KeyCode::KEY_N`] | n |
//! | [`KeyCode::KEY_M`] | m |
//! | [`KeyCode::KEY_COMMA`] | , |
//! | [`KeyCode::KEY_DOT`] | . |
//! | [`KeyCode::KEY_SLASH`] | / |
//! | [`KeyCode::KEY_F1`] | f1 |
//! | [`KeyCode::KEY_F2`] | f2 |
//! | [`KeyCode::KEY_F3`] | f3 |
//! | [`KeyCode::KEY_F4`] | f4 |
//! | [`KeyCode::KEY_F5`] | f5 |
//! | [`KeyCode::KEY_F6`] | f6 |
//! | [`KeyCode::KEY_F7`] | f7 |
//! | [`KeyCode::KEY_F8`] | f8 |
//! | [`KeyCode::KEY_F9`] | f9 |
//! | [`KeyCode::KEY_F10`] | f10 |
//! | [`KeyCode::KEY_F11`] | f11 |
//! | [`KeyCode::KEY_F12`] | f12 |

use evdev::{AttributeSet, KeyCode, KeyEvent, uinput::VirtualDevice};
use std::{fmt, io};

/// A single keybind consisting of a [`Button`] and [`Modifiers`]
pub type Bind = (Modifiers, Button);

/// A single button on the keyboard or mouse to press.
#[derive(Debug, Copy, Clone)]
pub struct Button(KeyCode);

/// Set of modifier keys (shift, alt, ...) to press.
///
/// This struct contains a compact representation of a set of modifiers keys.
/// [`Modifiers::none`] can be used to represent the notion of no pressed modifiers, while
/// [`Modifiers::union`] can be used to combine two sets of modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers(modifiers_wrapper::Modifiers);

/// Virtual keyboard device.
///
/// This struct represents a virtual keyboard device. Once created, it can be used to press and
/// release buttons.
#[derive(Debug)]
pub struct VirtualKeyboard(VirtualDevice);

// --------- //
// Modifiers //
// --------- //

// Don't expose all the bitflags wrappers, selecively export them instead.
mod modifiers_wrapper {
    use bitflags::bitflags;

    bitflags! {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct Modifiers: u8 {
            const L_SHIFT = 1 << 0;
            const R_SHIFT = 1 << 1;
            const L_CTRL  = 1 << 2;
            const R_CTRL  = 1 << 3;
            const L_ALT   = 1 << 4;
            const R_ALT   = 1 << 5;
            const L_META  = 1 << 6;
            const R_META  = 1 << 7;
        }
    }
}

impl Modifiers {
    pub const L_SHIFT: Self = Self(modifiers_wrapper::Modifiers::L_SHIFT);
    pub const R_SHIFT: Self = Self(modifiers_wrapper::Modifiers::R_SHIFT);
    pub const L_CTRL: Self = Self(modifiers_wrapper::Modifiers::L_CTRL);
    pub const R_CTRL: Self = Self(modifiers_wrapper::Modifiers::R_CTRL);
    pub const L_ALT: Self = Self(modifiers_wrapper::Modifiers::L_ALT);
    pub const R_ALT: Self = Self(modifiers_wrapper::Modifiers::R_ALT);
    pub const L_META: Self = Self(modifiers_wrapper::Modifiers::L_META);
    pub const R_META: Self = Self(modifiers_wrapper::Modifiers::R_META);

    /// A set with no pressed modifiers.
    pub fn none() -> Self {
        Self(modifiers_wrapper::Modifiers::empty())
    }

    /// Combine two sets of modifiers.
    pub fn union(self, other: Self) -> Self {
        Self(self.0.union(other.0))
    }

    fn emit(self, device: &mut VirtualDevice, event: i32) -> io::Result<()> {
        for flag in self.0.iter() {
            Self(flag).to_button().emit(device, event)?
        }
        Ok(())
    }
}

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pressed = Vec::with_capacity(8);

        for flag in self.0.iter() {
            match flag {
                modifiers_wrapper::Modifiers::L_SHIFT => pressed.push("LSHIFT"),
                modifiers_wrapper::Modifiers::R_SHIFT => pressed.push("RSHIFT"),
                modifiers_wrapper::Modifiers::L_CTRL => pressed.push("LCTRL"),
                modifiers_wrapper::Modifiers::R_CTRL => pressed.push("RCTRL"),
                modifiers_wrapper::Modifiers::L_ALT => pressed.push("LALT"),
                modifiers_wrapper::Modifiers::R_ALT => pressed.push("RALT"),
                modifiers_wrapper::Modifiers::L_META => pressed.push("LSUPER"),
                modifiers_wrapper::Modifiers::R_META => pressed.push("RSUPER"),
                _ => (),
            };
        }

        write!(f, "{}", pressed.join("+"))
    }
}

// ------ //
// Button //
// ------ //

impl Button {
    /// Create a button from a raw key code.
    ///
    /// This function does not guarantee that the virtual keyboard supports pressing the provided
    /// button. Prefer using [`Button::from_name`] instead.
    pub fn from_key_code(code: u16) -> Self {
        Self(evdev::KeyCode(code))
    }

    fn emit(self, device: &mut VirtualDevice, event: i32) -> io::Result<()> {
        log::trace!("Button {:?}: {:?}", self, event);
        device.emit(&[*KeyEvent::new(self.0, event)])?;
        Ok(())
    }
}

// -------- //
// Keyboard //
// -------- //

impl VirtualKeyboard {
    /// Create a new virtual keyboard.
    pub fn new() -> io::Result<Self> {
        let builder = VirtualDevice::builder()?
            .name(concat!(env!("CARGO_PKG_NAME"), " Virtual Keyboard"))
            .with_keys(&supported_keys())?;

        let mut keyboard = VirtualKeyboard(builder.build()?);
        log::debug!(
            "Virtual keyboard created at {:}",
            keyboard.0.get_syspath().unwrap_or_default().display()
        );

        Ok(keyboard)
    }

    /// Push and release a [`Bind`].
    pub fn press_bind(&mut self, bind: Bind) -> io::Result<()> {
        self.bind_down(bind)?;
        self.bind_up(bind)?;
        Ok(())
    }

    /// Hold down a [`Bind`].
    pub fn bind_down(&mut self, (modifiers, button): Bind) -> io::Result<()> {
        modifiers.emit(&mut self.0, 1)?;
        button.emit(&mut self.0, 1)?;
        Ok(())
    }

    /// Release a held [`Bind`].
    pub fn bind_up(&mut self, (modifiers, button): Bind) -> io::Result<()> {
        button.emit(&mut self.0, 0)?;
        modifiers.emit(&mut self.0, 0)?;
        Ok(())
    }

    /// Push and release a [`Button`].
    pub fn press_button(&mut self, button: Button) -> io::Result<()> {
        self.button_down(button)?;
        self.button_up(button)?;
        Ok(())
    }

    /// Hold down a [`Button`].
    pub fn button_down(&mut self, button: Button) -> io::Result<()> {
        button.emit(&mut self.0, 1)?;
        Ok(())
    }

    /// Release a held key [`Button`].
    pub fn button_up(&mut self, button: Button) -> io::Result<()> {
        button.emit(&mut self.0, 0)?;
        Ok(())
    }

    pub fn press_buttons(&mut self, buttons: &[Button]) -> io::Result<()> {
        self.buttons_down(buttons)?;
        self.buttons_up(buttons)?;
        Ok(())
    }

    pub fn buttons_down(&mut self, buttons: &[Button]) -> io::Result<()> {
        for button in buttons {
            button.emit(&mut self.0, 1)?;
        }
        Ok(())
    }

    pub fn buttons_up(&mut self, buttons: &[Button]) -> io::Result<()> {
        for button in buttons {
            button.emit(&mut self.0, 0)?;
        }
        Ok(())
    }

}

macro_rules! buttons {
    (
        mouse: [ $( ($mouse_key:expr, $mouse_idx: expr, $mouse_name: expr) ),* $(,)? ],
        modifiers: [ $( ($mod_key:expr, $mod_mod:path, [$($mod_name: expr),+]) ),* $(,)? ],
        keys: [ $( ($key_key:expr, $key_name: expr) ),* $(,)? ]
    ) => {
        fn supported_keys() -> AttributeSet<KeyCode> {
            let mut keys = AttributeSet::<KeyCode>::new();
            $(keys.insert($mouse_key);)*
            $(keys.insert($mod_key);)*
            $(keys.insert($key_key);)*
            keys
        }

        impl Button {
            /// Create a button from its name.
            ///
            /// Return `None` if the name is not a valid key or mouse button name.
            /// The provided string is downcased before it is matched.
            ///
            /// See the module documentation for the list of supported keys and their names.
            pub fn from_name(s: &str) -> Option<Self> {
                match s.to_lowercase().as_str() {
                    $(concat!("mouse", $mouse_idx) => Some(Button($mouse_key)),)*
                    $($mouse_name => Some(Button($mouse_key)),)*
                    $($($mod_name)|* => Some(Button($mod_key)),)*
                    $($key_name => Some(Button($key_key)),)*
                    _ => None
                }
            }

            /// Create a (mouse) button from its index.
            ///
            /// Indices 1 to 5 are supported. None is returned if another index is provided.
            pub fn from_mouse_idx(button: u16) -> Option<Self> {
                match button {
                    $($mouse_idx => Some(Button($mouse_key)),)*
                    _ => None
                }
            }
        }

        impl Modifiers {
            /// Create a modifier from its name.
            ///
            /// Return `None` if the name is not a valid modifier name.
            /// The provided string is downcased before it is matched.
            ///
            /// See the module documentation for the list of supported keys and their names.
            pub fn from_name(s: &str) -> Option<Self> {
                match s.to_lowercase().as_str() {
                    $($( $mod_name)|* => Some($mod_mod),)*
                    _ => None
                }
            }

            fn to_button(self) -> Button {
                match self {
                    $($mod_mod => Button($mod_key),)*
                    _ => panic!("Invalid modifier variant"),
                }
            }
        }
    };
}

buttons!(
    mouse: [
        (KeyCode::BTN_LEFT, 1, "mouse left"),
        (KeyCode::BTN_MIDDLE, 2, "mouse middle"),
        (KeyCode::BTN_RIGHT, 3, "mouse right"),
        (KeyCode::BTN_EXTRA, 4, "mouse extra"),
        (KeyCode::BTN_SIDE, 5, "mouse side"),
    ],
    modifiers: [
        (KeyCode::KEY_LEFTSHIFT, Modifiers::L_SHIFT, ["shift", "lshift"]),
        (KeyCode::KEY_RIGHTSHIFT, Modifiers::R_SHIFT, ["rshift"]),
        (KeyCode::KEY_LEFTALT, Modifiers::L_ALT, ["alt", "lalt"]),
        (KeyCode::KEY_RIGHTALT, Modifiers::R_ALT, ["ralt", "altgr"]),
        (KeyCode::KEY_LEFTCTRL, Modifiers::L_CTRL, ["ctrl", "lctrl"]),
        (KeyCode::KEY_RIGHTCTRL, Modifiers::R_CTRL, ["rctrl"]),
        (KeyCode::KEY_LEFTMETA, Modifiers::L_META, ["meta", "super", "lmeta", "lsuper"]),
        (KeyCode::KEY_RIGHTMETA, Modifiers::R_META, ["rmeta", "rsuper"]),
    ],
    keys: [
        (KeyCode::KEY_1, "1"),
        (KeyCode::KEY_2, "2"),
        (KeyCode::KEY_3, "3"),
        (KeyCode::KEY_4, "4"),
        (KeyCode::KEY_5, "5"),
        (KeyCode::KEY_6, "6"),
        (KeyCode::KEY_7, "7"),
        (KeyCode::KEY_8, "8"),
        (KeyCode::KEY_9, "9"),
        (KeyCode::KEY_0, "0"),
        (KeyCode::KEY_MINUS, "-"),
        (KeyCode::KEY_EQUAL, "="),
        (KeyCode::KEY_Q, "q"),
        (KeyCode::KEY_W, "w"),
        (KeyCode::KEY_E, "e"),
        (KeyCode::KEY_R, "r"),
        (KeyCode::KEY_T, "t"),
        (KeyCode::KEY_Y, "y"),
        (KeyCode::KEY_U, "u"),
        (KeyCode::KEY_I, "i"),
        (KeyCode::KEY_O, "o"),
        (KeyCode::KEY_P, "p"),
        (KeyCode::KEY_LEFTBRACE, "["),
        (KeyCode::KEY_RIGHTBRACE, "]"),
        (KeyCode::KEY_A, "a"),
        (KeyCode::KEY_S, "s"),
        (KeyCode::KEY_D, "d"),
        (KeyCode::KEY_F, "f"),
        (KeyCode::KEY_G, "g"),
        (KeyCode::KEY_H, "h"),
        (KeyCode::KEY_J, "j"),
        (KeyCode::KEY_K, "k"),
        (KeyCode::KEY_L, "l"),
        (KeyCode::KEY_SEMICOLON, ";"),
        (KeyCode::KEY_APOSTROPHE, "'"),
        (KeyCode::KEY_GRAVE, "~"),
        (KeyCode::KEY_BACKSLASH, r"\"),
        (KeyCode::KEY_Z, "z"),
        (KeyCode::KEY_X, "x"),
        (KeyCode::KEY_C, "c"),
        (KeyCode::KEY_V, "v"),
        (KeyCode::KEY_B, "b"),
        (KeyCode::KEY_N, "n"),
        (KeyCode::KEY_M, "m"),
        (KeyCode::KEY_COMMA, ","),
        (KeyCode::KEY_DOT, "."),
        (KeyCode::KEY_SLASH, "/"),
        (KeyCode::KEY_F1, "f1"),
        (KeyCode::KEY_F2, "f2"),
        (KeyCode::KEY_F3, "f3"),
        (KeyCode::KEY_F4, "f4"),
        (KeyCode::KEY_F5, "f5"),
        (KeyCode::KEY_F6, "f6"),
        (KeyCode::KEY_F7, "f7"),
        (KeyCode::KEY_F8, "f8"),
        (KeyCode::KEY_F9, "f9"),
        (KeyCode::KEY_F10, "f10"),
        (KeyCode::KEY_F11, "f11"),
        (KeyCode::KEY_F12, "f12"),
    ]
);
