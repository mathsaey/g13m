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

//! # g13m
//!
//! This crate provides support for handling Logitech G13 macropads.
//! It leverages the support Linux 6.19 added for these devices to forego writing low-level driver
//! code. As such, it will only work on systems running Linux 6.19 (and later) which have the
//! appropriate driver (`hid_lg_15`).
//!
//! Note that by default, the kernel will only allow `root` to access the leds and backlight of the
//! G13. The udev rules found in the `support` dir work around this by allowing members of the
//! `input` group to access these devices. This assumes the user running the software which uses
//! this crate is a member of this group.
//!
//! Besides this crate, this project can also build a `g13m` binary. The documentation here only
//! documents the library this project; please visit the project's repository for information about
//! the binary.
//!
//! ## Overview
//!
//! This library broadly does two things:
//!
//! 1. It makes it possible to find G13 devices connected to the system.
//! 2. It enables callbacks to be trigerred when specific events (keypresses, joystick actions,
//!    ...) occur on a device.
//!
//! This library heavily relies on creating [`Future`]s, which must then be ran by an executor. It
//! does not impose any particular backend to execute these futures.
//!
//! ### Finding Devices
//!
//! There are currently two ways to find connected G13 devices:
//! - The currently connected G13 devices can be retrieved through [`list`].
//! - A [`discovery_loop`] can be created to scan for newly connected G13 devices. The returned
//!   loop is wrapped inside a [`Future`] which you must execute yourself. The loop will call the
//!   provided closure when a G13 is found.
//!
//! Both of these functions provide you with a [`std::path::PathBuf`] which points to a sysfs entry
//! which can be used by [`Device::from_syspath`] to open the G13 device.
//!
//! ### Responding to Events
//!
//! Once a device is created from a syspath, [`Device::into_event_loop`] can be called to turn the
//! device into an event loop. This event loop is returned as a [`Future`] which you must execute
//! yourself.
//!
//! [`Device::into_event_loop`] accepts a [`Handler`] argument, on which it will call
//! [`Handler::handler_for_device`]. This method must return a [`DeviceHandler`] instance, on which
//! a method will be called any time an event occurs on the  G13 for which the event loop was
//! spawned.
//!
//! ### Summary
//!
//! Thus, to use this library, you must:
//!
//! 1. Obtain a G13 device (typically through [`list`] or [`discovery_loop`])
//! 2. Create a custom [`Handler`] and [`DeviceHandler`] and pass them to
//!    [`Device::into_event_loop`].
//!
//! `StaticHandler` and `LuaHandler` provide an example of the structure of such a handler.
//!
//! ## Features
//!
//! This crate provides the following features. None are enabled by default.
//!
//! - `virtual_keyboard`: Adds a module by the same name which can be used to push keys on a
//!   virtual keyboard. Useful to press "real" keys in response to G13 events.
//! - `bin`: Builds the binary, pulling in a lot of additional dependencies for argument and
//!   configuration parsing. Requires `handler_static`.
//! - `handler_static`: A handler which binds a particular keypress to each key (in each mode) and
//!   which binds a particular colour to each mode. Requires `virtual_keyboard`.
//! - `handler_lua*`: A handler which emulates the G-series lua api. You should not activate this
//!   feature directly, instead, use `handler_luajit`, `handler_lua54`, `handler_lua53`,
//!   `handler_lua52`, or `handler_lua51` to enable the `mlua` crate to enable support for the
//!   appropriate lua version. Requires `virtual_keyboard`.

mod g13;
pub use g13::{Device, Error, HandledDeviceRef, Rgb, discovery_loop, list};

mod handler;
pub use handler::{DeviceHandler, Handler};

#[cfg(feature = "virtual_keyboard")]
pub mod virtual_keyboard;

#[cfg(feature = "handler_static")]
pub mod handlers;
