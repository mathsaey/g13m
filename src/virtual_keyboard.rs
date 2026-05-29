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

use evdev::{AttributeSet, InputEvent, KeyEvent, uinput::VirtualDevice};
use std::{fmt, io};

/// Wrapper around an evdev [`KeyCode`]
///
/// This type represents a single key on the virtual keyboard or on the mouse.
/// Note that only certain keys can be pressed on the virtual keyboard. See the module
/// documentation for more information.
///
/// ## Original evdev documentation
///
pub use evdev::KeyCode;

/// A single keybind consisting of a [`KeyCode`] and [`Modifiers`]
pub type Bind = (Modifiers, KeyCode);

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

/// Set of modifier keys (shift, alt, ...) to press.
///
/// This struct contains a compact representation of a set of modifiers keys to press.
/// [`Modifiers::none`] can be used to represent the notion of no pressed modifiers, while
/// [`Modifiers::union`] can be used to combine two sets of modifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifiers(modifiers_wrapper::Modifiers);

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

    fn to_key_event_vec(self, value: i32) -> Vec<InputEvent> {
        self.iter_keycodes()
            .map(|k| *KeyEvent::new(k, value))
            .collect::<Vec<_>>()
    }

    fn iter_keycodes(&self) -> impl Iterator<Item = KeyCode> {
        self.0.iter().map(|f| Self(f).to_keycode())
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

/// Virtual keyboard device.
///
/// This struct represents a virtual keyboard device. Once created, it can be used to press and
/// release keys.
#[derive(Debug)]
pub struct VirtualKeyboard(VirtualDevice);

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

    /// Push and release a key [`Bind`].
    pub fn press(&mut self, bind: Bind) -> io::Result<()> {
        self.key_down(bind)?;
        self.key_up(bind)?;
        Ok(())
    }

    /// Hold down a key [`Bind`].
    pub fn key_down(&mut self, (modifiers, code): Bind) -> io::Result<()> {
        log::trace!("Key down {:?} {:?}", modifiers, code);
        self.0.emit(&modifiers.to_key_event_vec(1))?;
        self.0.emit(&[*KeyEvent::new(code, 1)])?;
        Ok(())
    }

    /// Release a held key [`Bind`].
    pub fn key_up(&mut self, (modifiers, code): Bind) -> io::Result<()> {
        log::trace!("Key up {:?} {:?}", modifiers, code);
        self.0.emit(&[*KeyEvent::new(code, 0)])?;
        self.0.emit(&modifiers.to_key_event_vec(0))?;
        Ok(())
    }

    /// Hold down a bunch of keys.
    ///
    /// Use this instead of [`VirtualKeyboard::key_down`] if you need to push several keys at once
    /// instead of activating a single [`Bind`].
    pub fn keys_down(&mut self, keys: &[KeyCode]) -> io::Result<()> {
        let events: Vec<InputEvent> = keys.iter().map(|&c| *KeyEvent::new(c, 1)).collect();
        self.0.emit(&events)?;
        Ok(())
    }

    /// Release a bunch of keys.
    ///
    /// Use this instead of [`VirtualKeyboard::key_up`] if you need to release several keys at once
    /// instead of activating a single [`Bind`].
    pub fn keys_up(&mut self, keys: &[KeyCode]) -> io::Result<()> {
        let events: Vec<InputEvent> = keys.iter().map(|&c| *KeyEvent::new(c, 0)).collect();
        self.0.emit(&events)?;
        Ok(())
    }

    /// Push and release a bunch of keys.
    ///
    /// Use this instead of [`VirtualKeyboard::press`] if you need to press several keys at once
    /// instead of activating a single [`Bind`].
    pub fn press_keys(&mut self, keys: &[KeyCode]) -> io::Result<()> {
        let mut events: Vec<InputEvent> = keys.iter().map(|&c| *KeyEvent::new(c, 1)).collect();
        self.0.emit(&events)?;
        events
            .iter_mut()
            .for_each(|e| *e = *KeyEvent::new(KeyCode(e.code()), 0));
        self.0.emit(&events)?;
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

        /// Map a key name to its [`KeyCode`].
        ///
        /// Return `None` if the name is not a valid key name.
        /// The provided string is downcased before it is matched.
        ///
        /// See the module documentation for the list of supported keys and their names.
        pub fn string_to_code(s: &str) -> Option<KeyCode> {
            match s.to_lowercase().as_str() {
                $(concat!("mouse", $mouse_idx) => Some($mouse_key),)*
                $($mouse_name => Some($mouse_key),)*
                $($($mod_name)|* => Some($mod_key),)*
                $($key_name => Some($key_key),)*
                _ => None
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

            fn to_keycode(self) -> KeyCode {
                match self {
                    $($mod_mod => $mod_key,)*
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
