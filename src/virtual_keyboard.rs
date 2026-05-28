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
//! keyboard device. This device can then be used to press keys. This is used by the various
//! [`crate::DeviceHandler`] to "press" keys in response to events occuring on the G13 device.
//!
//! ## Supported Keys
//!
//! The virtual keyboard can only be used to press certain keys.
//! The following table lists the supported keys and the names [`string_to_code`] will accept for
//! them.
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
//! | [`KeyCode::BTN_LEFT`] | mouse1 |
//! | [`KeyCode::BTN_MIDDLE`] | mouse2 |
//! | [`KeyCode::BTN_RIGHT`] | mouse3 |
//! | [`KeyCode::BTN_EXTRA`] | mouse4 |
//! | [`KeyCode::BTN_SIDE`] | mouse5 |

use bitflags::bitflags;
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

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]

    /// Set of modifier keys (shift, alt, ...) to press.
    ///
    /// This struct represents a set of modifier keys to press. The constants exported by this type
    /// can be used to select a particular modifier to press.
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

impl fmt::Display for Modifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut pressed = Vec::with_capacity(8);

        for flag in self.iter() {
            match flag {
                Modifiers::L_SHIFT => pressed.push("LSHIFT"),
                Modifiers::R_SHIFT => pressed.push("RSHIFT"),
                Modifiers::L_CTRL => pressed.push("LCTRL"),
                Modifiers::R_CTRL => pressed.push("RCTRL"),
                Modifiers::L_ALT => pressed.push("LALT"),
                Modifiers::R_ALT => pressed.push("RALT"),
                Modifiers::L_META => pressed.push("LSUPER"),
                Modifiers::R_META => pressed.push("RSUPER"),
                _ => (),
            };
        }

        write!(f, "{}", pressed.join("+"))
    }
}

impl Modifiers {
    /// Convert a [`KeyCode`] to a [`Modifiers`] set.
    ///
    /// The set only contains the modifier represented by the keycode.
    pub fn from_key_code(code: KeyCode) -> Option<Self> {
        match code {
            KeyCode::KEY_LEFTSHIFT => Some(Self::L_SHIFT),
            KeyCode::KEY_RIGHTSHIFT => Some(Self::R_SHIFT),
            KeyCode::KEY_LEFTALT => Some(Self::L_ALT),
            KeyCode::KEY_RIGHTALT => Some(Self::R_ALT),
            KeyCode::KEY_LEFTCTRL => Some(Self::L_CTRL),
            KeyCode::KEY_RIGHTCTRL => Some(Self::R_CTRL),
            KeyCode::KEY_LEFTMETA => Some(Self::L_META),
            KeyCode::KEY_RIGHTMETA => Some(Self::R_META),
            _ => None,
        }
    }

    fn to_key_event_vec(self, value: i32) -> Vec<InputEvent> {
        self.iter_keycodes()
            .map(|k| *KeyEvent::new(k, value))
            .collect::<Vec<_>>()
    }

    fn iter_keycodes(&self) -> impl Iterator<Item = KeyCode> {
        self.iter().map(|f| f.to_keycode())
    }

    fn to_keycode(self) -> KeyCode {
        match self {
            Modifiers::L_SHIFT => KeyCode::KEY_LEFTSHIFT,
            Modifiers::R_SHIFT => KeyCode::KEY_RIGHTSHIFT,
            Modifiers::L_CTRL => KeyCode::KEY_LEFTCTRL,
            Modifiers::R_CTRL => KeyCode::KEY_RIGHTCTRL,
            Modifiers::L_ALT => KeyCode::KEY_LEFTALT,
            Modifiers::R_ALT => KeyCode::KEY_RIGHTALT,
            Modifiers::L_META => KeyCode::KEY_LEFTMETA,
            Modifiers::R_META => KeyCode::KEY_RIGHTMETA,
            _ => panic!("Invalid modifier variant"),
        }
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

macro_rules! supported_keys {
    ($(($keystring:expr, $keycode:expr)),* $(,)?) => {
        fn supported_keys() -> AttributeSet<KeyCode> {
            let mut keys = AttributeSet::<KeyCode>::new();

            $(keys.insert($keycode);)*

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
                $($keystring => Some($keycode),)*
                _ => None
            }
        }
    };
}

supported_keys!(
    // Modifiers must be included here, or they can't be pressed by the virtual keyboard.
    // Make sure the names here match those in config::parse_modifiers for consistency.
    ("shift", KeyCode::KEY_LEFTSHIFT),
    ("lshift", KeyCode::KEY_LEFTSHIFT),
    ("rshift", KeyCode::KEY_RIGHTSHIFT),
    ("alt", KeyCode::KEY_LEFTALT),
    ("lalt", KeyCode::KEY_LEFTALT),
    ("ralt", KeyCode::KEY_RIGHTALT),
    ("altgr", KeyCode::KEY_RIGHTALT),
    ("ctrl", KeyCode::KEY_LEFTCTRL),
    ("lctrl", KeyCode::KEY_LEFTCTRL),
    ("rctrl", KeyCode::KEY_RIGHTCTRL),
    ("meta", KeyCode::KEY_LEFTMETA),
    ("lmeta", KeyCode::KEY_LEFTMETA),
    ("rmeta", KeyCode::KEY_RIGHTMETA),
    ("super", KeyCode::KEY_LEFTMETA),
    ("lsuper", KeyCode::KEY_LEFTMETA),
    ("rsuper", KeyCode::KEY_RIGHTMETA),
    //
    ("esc", KeyCode::KEY_ESC),
    ("enter", KeyCode::KEY_ENTER),
    ("backspace", KeyCode::KEY_BACKSPACE),
    ("tab", KeyCode::KEY_TAB),
    ("capslock", KeyCode::KEY_CAPSLOCK),
    ("space", KeyCode::KEY_SPACE),
    //
    ("1", KeyCode::KEY_1),
    ("2", KeyCode::KEY_2),
    ("3", KeyCode::KEY_3),
    ("4", KeyCode::KEY_4),
    ("5", KeyCode::KEY_5),
    ("6", KeyCode::KEY_6),
    ("7", KeyCode::KEY_7),
    ("8", KeyCode::KEY_8),
    ("9", KeyCode::KEY_9),
    ("0", KeyCode::KEY_0),
    ("-", KeyCode::KEY_MINUS),
    ("=", KeyCode::KEY_EQUAL),
    ("q", KeyCode::KEY_Q),
    ("w", KeyCode::KEY_W),
    ("e", KeyCode::KEY_E),
    ("r", KeyCode::KEY_R),
    ("t", KeyCode::KEY_T),
    ("y", KeyCode::KEY_Y),
    ("u", KeyCode::KEY_U),
    ("i", KeyCode::KEY_I),
    ("o", KeyCode::KEY_O),
    ("p", KeyCode::KEY_P),
    ("[", KeyCode::KEY_LEFTBRACE),
    ("]", KeyCode::KEY_RIGHTBRACE),
    ("a", KeyCode::KEY_A),
    ("s", KeyCode::KEY_S),
    ("d", KeyCode::KEY_D),
    ("f", KeyCode::KEY_F),
    ("g", KeyCode::KEY_G),
    ("h", KeyCode::KEY_H),
    ("j", KeyCode::KEY_J),
    ("k", KeyCode::KEY_K),
    ("l", KeyCode::KEY_L),
    (";", KeyCode::KEY_SEMICOLON),
    ("'", KeyCode::KEY_APOSTROPHE),
    ("~", KeyCode::KEY_GRAVE),
    (r"\", KeyCode::KEY_BACKSLASH),
    ("z", KeyCode::KEY_Z),
    ("x", KeyCode::KEY_X),
    ("c", KeyCode::KEY_C),
    ("v", KeyCode::KEY_V),
    ("b", KeyCode::KEY_B),
    ("n", KeyCode::KEY_N),
    ("m", KeyCode::KEY_M),
    (",", KeyCode::KEY_COMMA),
    (".", KeyCode::KEY_DOT),
    ("/", KeyCode::KEY_SLASH),
    ("f1", KeyCode::KEY_F1),
    ("f2", KeyCode::KEY_F2),
    ("f3", KeyCode::KEY_F3),
    ("f4", KeyCode::KEY_F4),
    ("f5", KeyCode::KEY_F5),
    ("f6", KeyCode::KEY_F6),
    ("f7", KeyCode::KEY_F7),
    ("f8", KeyCode::KEY_F8),
    ("f9", KeyCode::KEY_F9),
    ("f10", KeyCode::KEY_F10),
    ("f11", KeyCode::KEY_F11),
    ("f12", KeyCode::KEY_F12),
    //
    ("mouse1", KeyCode::BTN_LEFT),
    ("mouse2", KeyCode::BTN_MIDDLE),
    ("mouse3", KeyCode::BTN_RIGHT),
    ("mouse4", KeyCode::BTN_EXTRA),
    ("mouse5", KeyCode::BTN_SIDE),
);
