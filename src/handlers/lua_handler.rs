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

//! Calls functions in a lua script in response to events.
//!
//! This handler aims to recreate (part of) the Logitech G-series API. It is created with a path to
//! a script, which should contain an `OnEvent(event, arg, device)` function. This function will be
//! called for the following events:
//!
//! | event | arg | description |
//! | ----- | --- | ----------- |
//! | "G_PRESSED" | `i` | Called when a G key is held down, `i` represents the number of the G key which was pressed. |
//! | "G_RELEASED" | `i` | Called when a G key is released, `i` represents the number of the G key which was released. |
//! | "M_PRESSED" | `i` | Called when an M key is held down, `i` represents the number of the M key which was pressed. |
//! | "M_RELEASED" | `i` | Called when an G key is released, `i` represents the number of the M key which was released. |
//!
//! The third argument (`device`) is currently always `"lhc"`.
//!
//! The following events are currently not supported: "PROFILE_ACTIVATED", "PROFILE_DEACTIVATED",
//! "MOUSE_BUTTON_PRESSED", "MOUSE_BUTTON_RELEASED".
//!
//! The following functions can be called.
//!
//! | function | arguments | description |
//! | -------- | --------- | ----------- |
//! | PressKey | variable amount of keycodes or scancodes | Press the provided keys, in order. Keys can be provided as a keycode (i.e. a name, see [`crate::virtual_keyboard`]) or a scancode (an index). |
//! | ReleaseKey | variable amount of keycodes or scancodes | Release the provided keys, in order. Keys can be provided as a keycode (i.e. a name, see [`crate::virtual_keyboard`]) or a scancode (an index). |
//! | PressAndReleaseKey | variable amount of keycodes or scancodes | Press and release the provided keys, in order. Keys can be provided as a keycode (i.e. a name, see [`crate::virtual_keyboard`]) or a scancode (an index). |
//! | PressMouseButton | index between 1 and 5 | Press the provided mouse button. [`crate::virtual_keyboard`] details which mouse index maps to which mouse button.
//! | ReleaseMouseButton | index between 1 and 5 | Release the provided mouse button. [`crate::virtual_keyboard`] details which mouse index maps to which mouse button.
//! | PressandReleaseMouseButton | index between 1 and 5 | Press and release the provided mouse button. [`crate::virtual_keyboard`] details which mouse index maps to which mouse button.
//! | GetMKeyState | | Get the currently active M state. |
//! | SetMKeyState | i | Set the M state to `i`. |
//! | SetBacklightColor | r, g, b | Change the backlight color to the provided rgb value. |
//! | Sleep | time | Sleep `time` ms. Blocks the handler while sleeping. |
//! | GetDate | | Mirror of `os.date`. |
//! | GetRunningTime | | Get the time in ms the handler has been running. |
//! | OutputLogMessage | msg, arg1, arg2, ... | Output a log message. `msg` is formatted with the provided `arg`s using `string.format`. |
//! | OutputDebugMessage | msg, arg1, arg2, ... | Output a debug message. `msg` is formatted with the provided `arg`s using `string.format`. Note that this message will not show up in release builds. |
//!
//! The following functions provided by G-series API are currently not supported. Calling them will
//! log an error but not crash the lua script.
//!
//!  OutputLCDMessage, ClearLCD, PlayMacro, AbortMacro, ClearLog, IsKeyLockOn, IsModifierPressed,
//!  IsMouseButtonPressed, MoveMouseTo, MoveMouseWheel, MoveMouseRelative, MoveMouseToVirtual,
//!  GetMousePosition, SetMouseDPITable, SetMouseDPITableIndex, EnablePrimaryMouseButtonEvents,
//!  SetMouseSpeed, GetMouseSpeed, IncrementMouseSpeed, DecrementMouseSpeed,
//!  SetSteeringWheelProperty

use std::io;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use mlua::prelude::*;

use crate::Rgb;
use crate::virtual_keyboard::{Bind, KeyCode, Modifiers, VirtualKeyboard, string_to_code};
use crate::{DeviceHandler, HandledDeviceRef, Handler};

#[derive(Debug)]
pub struct LuaHandler {
    keyboard: Arc<Mutex<VirtualKeyboard>>,
    callback: LuaFunction,
    lua: Rc<Lua>,
}

#[derive(Clone, Debug)]
pub struct LuaDeviceHandler {
    keyboard: Arc<Mutex<VirtualKeyboard>>,
    callback: LuaFunction,
    lua: Rc<Lua>,
    start: Arc<Instant>,
    device_ref: HandledDeviceRef,
}

impl LuaHandler {
    pub fn new(keyboard: Arc<Mutex<VirtualKeyboard>>, path: &Path) -> LuaResult<Self> {
        let lua = Rc::new(Lua::new());
        lua.load(path).exec()?;

        let callback: LuaFunction = lua.globals().get("OnEvent")?;

        Ok(LuaHandler {
            keyboard,
            callback,
            lua,
        })
    }
}

impl Handler for LuaHandler {
    #[allow(refining_impl_trait)]
    fn handler_for_device(&self, device_ref: HandledDeviceRef) -> LuaDeviceHandler {
        LuaDeviceHandler::new(self, device_ref)
    }
}

impl LuaDeviceHandler {
    fn new(handler: &LuaHandler, device_ref: HandledDeviceRef) -> Self {
        let res = Self {
            keyboard: handler.keyboard.clone(),
            callback: handler.callback.clone(),
            lua: handler.lua.clone(),
            start: Arc::new(Instant::now()),
            device_ref,
        };

        res.bind_functions().unwrap_or_else(|err| {
            log::error!("Could not bind lua functions: {}", err);
        });

        res.on_event("PROFILE_ACTIVATED", LuaNil);

        res
    }

    fn bind_function<A, R, F>(&self, name: &str, fun: F) -> LuaResult<()>
    where
        A: FromLuaMulti,
        R: IntoLuaMulti,
        F: LuaNativeFn<A, Output = LuaResult<R>> + 'static,
    {
        self.lua.globals().set(name, LuaFunction::wrap(fun))
    }

    fn bind_not_implemented(&self, name: &'static str) -> LuaResult<()> {
        self.bind_function(name, create_lua_not_implemented(name))
    }

    fn mirror_native(&self, name: &str, tab_name: &str, fn_name: &str) -> LuaResult<()> {
        let tab: LuaTable = self.lua.globals().get(tab_name)?;
        let fun: LuaFunction = tab.get(fn_name)?;
        self.lua.globals().set(name, fun)
    }

    fn bind_functions(&self) -> LuaResult<()> {
        self.bind_function(
            "PressKey",
            create_key_fun(self.keyboard.clone(), VirtualKeyboard::keys_down),
        )?;
        self.bind_function(
            "ReleaseKey",
            create_key_fun(self.keyboard.clone(), VirtualKeyboard::keys_up),
        )?;
        self.bind_function(
            "PressAndReleaseKey",
            create_key_fun(self.keyboard.clone(), VirtualKeyboard::press_keys),
        )?;

        self.bind_function(
            "PressMouseButton",
            create_mouse_button_fun(self.keyboard.clone(), VirtualKeyboard::key_down),
        )?;
        self.bind_function(
            "ReleaseMouseButton",
            create_mouse_button_fun(self.keyboard.clone(), VirtualKeyboard::key_up),
        )?;
        self.bind_function(
            "PressAndReleaseMouseButton",
            create_mouse_button_fun(self.keyboard.clone(), VirtualKeyboard::press),
        )?;

        self.bind_function(
            "GetMKeyState",
            create_get_m_key_state(self.device_ref.clone()),
        )?;
        self.bind_function(
            "SetMKeyState",
            create_set_m_key_state(self.device_ref.clone()),
        )?;
        self.bind_function(
            "SetBacklightColor",
            create_set_backlight_color(self.device_ref.clone()),
        )?;

        self.bind_function("Sleep", lua_sleep)?;
        self.mirror_native("GetDate", "os", "date")?;
        self.bind_function(
            "GetRunningTime",
            create_get_running_time(self.start.clone()),
        )?;

        let string_tab: LuaTable = self.lua.globals().get("string")?;
        let format_fn: LuaFunction = string_tab.get("format")?;

        self.bind_function(
            "OutputLogMessage",
            create_output_log_message(format_fn.clone()),
        )?;
        self.bind_function(
            "OutputDebugMessage",
            create_output_debug_message(format_fn.clone()),
        )?;

        // Requires LCD support
        self.bind_not_implemented("OutputLCDMessage")?;
        self.bind_not_implemented("ClearLCD")?;

        // Requires macro support
        self.bind_not_implemented("PlayMacro")?;
        self.bind_not_implemented("AbortMacro")?;

        self.bind_not_implemented("ClearLog")?;
        self.bind_not_implemented("IsKeyLockOn")?;
        self.bind_not_implemented("IsModifierPressed")?;
        self.bind_not_implemented("IsMouseButtonPressed")?;
        self.bind_not_implemented("MoveMouseTo")?;
        self.bind_not_implemented("MoveMouseWheel")?;
        self.bind_not_implemented("MoveMouseRelative")?;
        self.bind_not_implemented("MoveMouseToVirtual")?;
        self.bind_not_implemented("GetMousePosition")?;
        self.bind_not_implemented("SetMouseDPITable")?;
        self.bind_not_implemented("SetMouseDPITableIndex")?;
        self.bind_not_implemented("EnablePrimaryMouseButtonEvents")?;
        self.bind_not_implemented("SetMouseSpeed")?;
        self.bind_not_implemented("GetMouseSpeed")?;
        self.bind_not_implemented("IncrementMouseSpeed")?;
        self.bind_not_implemented("DecrementMouseSpeed")?;
        self.bind_not_implemented("SetSteeringWheelProperty")?;

        Ok(())
    }

    fn on_event(&self, event: &str, args: impl IntoLua) {
        self.callback
            .call::<()>((event, args, "lhc"))
            .unwrap_or_else(lua_error)
    }
}

impl DeviceHandler for LuaDeviceHandler {
    fn g_key_down(&mut self, key: u16) {
        self.on_event("G_PRESSED", key);
    }

    fn g_key_up(&mut self, key: u16) {
        self.on_event("G_RELEASED", key);
    }

    fn m_key_down(&mut self, key: u16) {
        self.on_event("M_PRESSED", key);
    }

    fn m_key_up(&mut self, key: u16) {
        self.on_event("M_RELEASED", key);
    }
}

fn lua_error(err: LuaError) {
    log::warn!("Calling lua caused an error: {}", err);
}

fn create_lua_not_implemented(name: &'static str) -> impl LuaNativeFn<(), Output = LuaResult<()>> {
    move || {
        log::warn!("Ignoring call to unimplemented lua function {}", name);
        Ok(())
    }
}

fn create_key_fun(
    keyboard: Arc<Mutex<VirtualKeyboard>>,
    fun: fn(&mut VirtualKeyboard, &[KeyCode]) -> io::Result<()>,
) -> impl LuaNativeFn<(LuaVariadic<LuaValue>,), Output = LuaResult<()>> {
    move |args: LuaVariadic<LuaValue>| {
        let mut kb = keyboard.lock().unwrap();
        fun(&mut kb, &map_to_keycode(args))?;
        Ok(())
    }
}

fn map_to_keycode(args: LuaVariadic<LuaValue>) -> Vec<KeyCode> {
    log::debug!("{:?}", args);
    let res = args
        .iter()
        .filter_map(|val| match val {
            LuaValue::Integer(scancode) => Some(KeyCode(*scancode as u16)),
            LuaValue::String(keyname) => keyname.to_str().as_deref().ok().and_then(string_to_code),
            _ => None,
        })
        .collect();
    log::debug!("{:?}", res);

    res
}

fn create_mouse_button_fun(
    keyboard: Arc<Mutex<VirtualKeyboard>>,
    fun: fn(&mut VirtualKeyboard, Bind) -> io::Result<()>,
) -> impl LuaNativeFn<(u16,), Output = LuaResult<()>> {
    move |m: u16| {
        if let Some(bind) = map_mouse_button(m) {
            let mut kb = keyboard.lock().unwrap();
            fun(&mut kb, bind)?;
        } else {
            log::warn!("Ignoring invalid mouse button: {}", m);
        }
        Ok(())
    }
}

fn map_mouse_button(button: u16) -> Option<Bind> {
    // Use string_to_ode so that virtual_keyboard is the only source of truth for mapping names to
    // keycodes. Assume rust will be smart enough to optimise this to a constant lookup.
    match button {
        1 => Some("mouse1"),
        2 => Some("mouse2"),
        3 => Some("mouse3"),
        4 => Some("mouse4"),
        5 => Some("mouse5"),
        _ => None,
    }
    .and_then(string_to_code)
    .map(|button| (Modifiers::empty(), button))
}

fn create_get_m_key_state(
    device_ref: HandledDeviceRef,
) -> impl LuaNativeFn<(), Output = LuaResult<u8>> {
    move || Ok(device_ref.mode())
}

fn create_set_m_key_state(
    device_ref: HandledDeviceRef,
) -> impl LuaNativeFn<(u16,), Output = LuaResult<()>> {
    move |m: u16| {
        device_ref.set_mode(m);
        Ok(())
    }
}

fn create_set_backlight_color(
    device_ref: HandledDeviceRef,
) -> impl LuaNativeFn<(u8, u8, u8), Output = LuaResult<()>> {
    move |r, g, b| {
        device_ref
            .set_backlight(Rgb(r, g, b))
            .unwrap_or_else(|err| {
                log::error!("Could not change backlight: {}", err);
            });
        Ok(())
    }
}

fn lua_sleep(time: u64) -> LuaResult<()> {
    sleep(Duration::from_millis(time));
    Ok(())
}

fn create_get_running_time(start: Arc<Instant>) -> impl LuaNativeFn<(), Output = LuaResult<u128>> {
    move || Ok(start.elapsed().as_millis())
}

fn call_lua_format_fn(format_fn: &LuaFunction, args: LuaMultiValue) -> Option<String> {
    let res: LuaResult<LuaString> = format_fn.call(args);
    match res {
        Ok(s) => Some(s.to_string_lossy()),
        Err(e) => {
            log::warn!("Could not format lua string: {}", e);
            None
        }
    }
}

fn create_output_log_message(
    format_fn: LuaFunction,
) -> impl LuaNativeFn<(LuaMultiValue,), Output = LuaResult<()>> {
    move |args| {
        if let Some(s) = call_lua_format_fn(&format_fn, args) {
            log::info!("[Lua Log] {}", s);
        }
        Ok(())
    }
}

fn create_output_debug_message(
    format_fn: LuaFunction,
) -> impl LuaNativeFn<(LuaMultiValue,), Output = LuaResult<()>> {
    move |args| {
        if let Some(s) = call_lua_format_fn(&format_fn, args) {
            log::debug!("[Lua Log] {}", s);
        }
        Ok(())
    }
}
