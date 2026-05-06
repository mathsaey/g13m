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

//! Handler which combines static binds with a lua handler.
//!
//! This handler combines a [`StaticHandler`] with an (optional) [`LuaHandler`]. It will always
//! execute the lua handler first.

use crate::handlers::lua_handler::{LuaDeviceHandler, LuaHandler};
use crate::handlers::static_handler::{StaticDeviceHandler, StaticHandler};
use crate::{DeviceHandler, HandledDeviceRef, Handler};

#[derive(Debug)]
pub struct CombinedHandler {
    stc: StaticHandler,
    lua: Option<LuaHandler>,
}

#[derive(Clone, Debug)]
struct CombinedDeviceHandler {
    stc: StaticDeviceHandler,
    lua: Option<LuaDeviceHandler>,
}

impl CombinedHandler {
    /// Create a CombinedHandler.
    pub fn new(stc: StaticHandler, lua: Option<LuaHandler>) -> Self {
        CombinedHandler { stc, lua }
    }
}

impl Handler for CombinedHandler {
    fn handler_for_device(&self, device_ref: HandledDeviceRef) -> impl DeviceHandler {
        CombinedDeviceHandler {
            stc: self.stc.handler_for_device(device_ref.clone()),

            lua: self
                .lua
                .as_ref()
                .map(|lua| lua.handler_for_device(device_ref.clone())),
        }
    }
}

impl DeviceHandler for CombinedDeviceHandler {
    fn g_key_down(&mut self, key: u16) {
        if let Some(lua) = self.lua.as_mut() {
            lua.g_key_down(key)
        };
        self.stc.g_key_down(key);
    }

    fn g_key_up(&mut self, key: u16) {
        if let Some(lua) = self.lua.as_mut() {
            lua.g_key_up(key)
        };
        self.stc.g_key_up(key);
    }

    fn m_key_down(&mut self, key: u16) {
        if let Some(lua) = self.lua.as_mut() {
            lua.m_key_down(key)
        };
        self.stc.m_key_down(key);
    }

    fn m_key_up(&mut self, key: u16) {
        if let Some(lua) = self.lua.as_mut() {
            lua.m_key_up(key)
        };
        self.stc.m_key_up(key);
    }

    fn x_axis(&mut self, val: i32) {
        if let Some(lua) = self.lua.as_mut() {
            lua.x_axis(val)
        };
        self.stc.x_axis(val);
    }

    fn y_axis(&mut self, val: i32) {
        if let Some(lua) = self.lua.as_mut() {
            lua.y_axis(val)
        };
        self.stc.y_axis(val);
    }
}
