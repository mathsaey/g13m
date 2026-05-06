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

use bitflags::bitflags;
use evdev::{AttributeSet, InputEvent, KeyEvent, uinput::VirtualDevice};
use std::{fmt, io};

pub use evdev::KeyCode;
pub type Bind = (Modifiers, KeyCode);

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

#[derive(Debug)]
pub struct VirtualKeyboard(VirtualDevice);

impl VirtualKeyboard {
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

    pub fn press(&mut self, bind: Bind) -> io::Result<()> {
        self.key_down(bind)?;
        self.key_up(bind)?;
        Ok(())
    }

    pub fn key_down(&mut self, (modifiers, code): Bind) -> io::Result<()> {
        log::trace!("Key down {:?} {:?}", modifiers, code);
        self.0.emit(&modifiers.to_key_event_vec(1))?;
        self.0.emit(&[*KeyEvent::new(code, 1)])?;
        Ok(())
    }

    pub fn key_up(&mut self, (modifiers, code): Bind) -> io::Result<()> {
        log::trace!("Key up {:?} {:?}", modifiers, code);
        self.0.emit(&modifiers.to_key_event_vec(0))?;
        self.0.emit(&[*KeyEvent::new(code, 0)])?;
        Ok(())
    }

    pub fn keys_down(&mut self, keys: &[KeyCode]) -> io::Result<()> {
        let events: Vec<InputEvent> = keys.iter().map(|&c| *KeyEvent::new(c, 1)).collect();
        self.0.emit(&events)?;
        Ok(())
    }

    pub fn keys_up(&mut self, keys: &[KeyCode]) -> io::Result<()> {
        let events: Vec<InputEvent> = keys.iter().map(|&c| *KeyEvent::new(c, 0)).collect();
        self.0.emit(&events)?;
        Ok(())
    }

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
        pub fn supported_keys() -> AttributeSet<KeyCode> {
            let mut keys = AttributeSet::<KeyCode>::new();

            $(keys.insert($keycode);)*

            keys
        }

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
