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

#[cfg(feature = "handler_static")]
pub mod static_handler;

#[cfg(feature = "handler_lua")]
pub mod lua_handler;

#[cfg(feature = "handler_static")]
mod combined_handler;
#[cfg(feature = "handler_static")]
pub use combined_handler::CombinedHandler;

// Lua handler placeholder
// -----------------------

#[cfg(not(feature = "handler_lua"))]
pub mod lua_handler {
    use crate::HandledDeviceRef;
    use crate::handler::{DeviceHandler, Handler};
    use crate::virtual_keyboard::VirtualKeyboard;
    use std::{
        path::Path,
        sync::{Arc, Mutex},
    };

    #[derive(Debug)]
    pub struct LuaHandler {}

    #[derive(Clone, Debug)]
    pub struct LuaDeviceHandler {}

    impl LuaHandler {
        pub fn new(_: Arc<Mutex<VirtualKeyboard>>, _: &Path) -> Result<Self, std::io::Error> {
            Ok(LuaHandler {})
        }
    }

    impl Handler for LuaHandler {
        #[allow(refining_impl_trait)]
        fn handler_for_device(&self, _: HandledDeviceRef) -> LuaDeviceHandler {
            LuaDeviceHandler {}
        }
    }

    impl DeviceHandler for LuaDeviceHandler {}
}
