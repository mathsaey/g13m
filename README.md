# g13m

Key mapper for Logitech G13 devices on Linux.

This tool provides support for handling Logitech G13 macropads.
It leverages the support Linux 6.19 added for these devices to forego writing
low-level driver code. As such, it will only work on systems running Linux 6.19
(and later) which have the appropriate driver (`hid_lg_15`).

This project can either be used as a command-line application or as a rust
library. This README mainly documents the command-line application. For
information about the crate, please refer to the
[docs](https://docs.rs/g13m/latest).

## Features

- Static configuration using an ini-based configuration file:
  - Binding G keys to keypresses.
  - Giving each m key its own associated backlight color.
- Responding to keypresses from a Lua script:
  - The `OnEvent` function is called for each G or M key press or release.
  - Programatically press and release keys.
  - Programatically set the current M profile.
  - Programatically set the backlight color.
  - Support for other functions from the original G-series Lua API. Note that
    not all functions are supported.

Additionally, this project can be used as a Rust crate, which enables it to
serve as a backend for other software which aims to leverage Linux's support
for the G13.

###  Not Supported

- LCD
- Recording macros

## Building & Installation

The following software needs to be available on your system:
- cargo & rust
- pkg-config
- libudev
- Lua (optional)

To build without Lua support: `cargo build --release --bins --features bin`
To build with Lua `X.Y` support: `cargo build --release --bins --features bin,handler_luaXY`

### udev rules

By default, the kernel will only allow `root` to access the leds and backlight
of a G13. The udev rules found in the `support` directory work around this by
allowing members of the `input` group to access these devices. Thus, after
installation, you should:

- Ensure these rules are picked up by udev.
- Ensure the user running the application is a member of the `input` group.

## Usage

Once installed, you can use the application as follows:
```
g13m -p profile_name
```

This will monitor your system for any connected G13s and map their keys
according to your configuration in `$XDG_CONFIG_HOME/g13m/profile_name.ini`.
If no profile name is provided, `$XDG_CONFIG_HOME/g13m/default.ini` will be
used.

An example configuration file can be found in the `support` directory.

Please refer to the (upcoming) man page for more information.
