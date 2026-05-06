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

use crate::HandledDeviceRef;
use std::fmt::Debug;

/// Type which creates [`DeviceHandler`]s to respond to events.
///
/// A type which implements this trait may be passed to [`crate::Device::into_event_loop`],
/// after which it will be called to create a [`DeviceHandler`] which will handle events generated
/// by the G13.
///
/// This trait is decoupled from the [`DeviceHandler`] trait to enable the implementation of
/// handlers using the typestate pattern. In this way, a [`Handler`] type should only contain the
/// information that is related to the handler itself, while the [`DeviceHandler`] contains this
/// information along with any relevant device-specific state, such as a [`HandledDeviceRef`].
pub trait Handler {
    /// Create the [`DeviceHandler`] before starting the G13 event loop.
    ///
    /// Called by [`crate::Device::into_event_loop`] before the various event loops are created.
    /// This function is used to return a [`DeviceHandler`] which will be used to process events.
    fn handler_for_device(&self, device_ref: HandledDeviceRef) -> impl DeviceHandler;
}

/// Type which responds to G13 events.
///
/// Instances of types which implement this trait are created by [`Handler::handler_for_device`] to
/// process events occuring on a specific G13 device.
///
/// An instance of this type will be cloned to multiple event loops, which is why it must implement
/// [`Clone`].
pub trait DeviceHandler: Clone + Debug {
    /// Handle a key down event for a G key.
    ///
    /// The `key` index refers to the number of the pressed `G` key.
    /// A press on the thumb or joystick keys will also trigger this function.
    /// - 23 refers to the top thumb key
    /// - 24 refers to the bottom thumb key
    /// - 25 refers to the joystick key
    fn g_key_down(&mut self, _key: u16) {}

    /// Handle a key up event for a G key.
    ///
    /// Like [`DeviceHandler::g_key_down`], but for a key up event.
    fn g_key_up(&mut self, _key: u16) {}

    /// Handle a key up event for an M key.
    ///
    /// This method is guaranteed to be called _before_ the mode index of the G13 is actually
    /// changed in [`HandledDeviceRef`]. Therefore, calling [`HandledDeviceRef::mode`] in this
    /// method will return the _previous_ state of the G13.
    ///
    /// This method should **not** set the mode of the G13, as the device loop will set the new
    /// mode after this function returns. Moreover, there is no need for the handler to track the
    /// current mode the G13 is in, as this can be accessed using [`HandledDeviceRef::mode`].
    fn m_key_down(&mut self, _key: u16) {}

    /// Handle a key down event for an M key.
    ///
    /// See [`DeviceHandler::m_key_down`].
    fn m_key_up(&mut self, _key: u16) {}

    /// Handle a change in the x axis of the joystick.
    ///
    /// Possible joystick values go from 0 to 255, where 0 represents left and 255 represents
    /// right.
    ///
    /// Note that the joystick triggers easily. You probably don't want to perform long-running
    /// computations here. Due to the sensitivity, it is also advisable to implement some sort of
    /// dead zone.
    fn x_axis(&mut self, _val: i32) {}

    /// Handle a change in the y axis of the joystick.
    ///
    /// Possible joystick values go from 0 to 255, where 0 represents top and 255 represents
    /// bottom.
    ///
    /// Note that the joystick triggers easily. You probably don't want to perform long-running
    /// computations here. Due to the sensitivity, it is also advisable to implement some sort of
    /// dead zone.
    fn y_axis(&mut self, _val: i32) {}
}
