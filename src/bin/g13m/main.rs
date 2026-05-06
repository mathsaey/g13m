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

use std::env;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, Mutex};

use async_executor::LocalExecutor;
use clap::Parser;
use futures_lite::future;

use g13m::handlers::{CombinedHandler, lua_handler::LuaHandler, static_handler::StaticHandler};
use g13m::{virtual_keyboard::VirtualKeyboard, *};

mod config;

/// Key mapper for Logitech G13 devices.
///
/// Once started, this application will look for any connected G13 and map its keys according to
/// the settings defined in the profile configuration.
///
/// If no G13s are present, or when the connected G13 is disconnected, the application will keep
/// running and listen for any G13s which are connected in the future. This behaviour can be
/// modified with the --oneshot or --device options.
///
/// By default, this application will read configuration from $XDG_CONFIG_HOME/default.ini; the
/// --profile or --profile-path options can be used to override this location.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Set the log level.
    ///
    /// Accepted values: info, warn, error, off.
    #[arg(short, long, value_enum, default_value = "info")]
    log_level: log::LevelFilter,

    /// Name of the profile to be used.
    ///
    /// The profile configuration will be read from $XDG_CONFIG_HOME/PROFILE.ini, where PROFILE
    /// is replaced with the value provided to this option. Mutually exclusive with --profile-path.
    #[arg(
        short = 'p',
        long = "profile",
        default_value = "default",
        conflicts_with = "path",
        hide_default_value = true
    )]
    profile: String,

    /// Path to the profile file to use.
    ///
    /// The profile configuration will be read from the provided path.
    /// Mutually exclusive with the --profile option.
    #[arg(short = 'P', long = "profile-path", conflicts_with = "profile")]
    path: Option<PathBuf>,

    /// Only map the G13 device at PATH.
    ///
    /// PATH should be a sysfs path pointing to a usb device, usb interface, or hid device.
    /// The application will exit when the device at PATH is no longer available.
    /// This can be useful to make systemd start the mapper when a G13 is plugged in.
    ///
    /// Mutually exclusive with --oneshot
    #[arg(short, long, conflicts_with = "oneshot")]
    device: Option<PathBuf>,

    /// Exit when no g13 is present or when it is unplugged.
    ///
    /// Attempt to find a G13 and map it. Once the g13 is unplugged, the application stops.
    /// If multiple G13s are plugged in, only one will be mapped.
    ///
    /// Mutually exclusive with --device.
    #[arg(short = '1', long, conflicts_with = "device")]
    oneshot: bool,
}

fn init_logger(cli: &Cli) {
    env_logger::builder()
        .filter_level(cli.log_level)
        .format_target(cli.log_level >= log::LevelFilter::Trace)
        .init();
}

fn get_config_path(cli: &Cli) -> PathBuf {
    match &cli.path {
        Some(path) => path.clone(),
        None => {
            let mut home = env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(env::home_dir)
                .unwrap_or_else(|| {
                    log::error!("Could not find home directory");
                    exit(1);
                });

            home.push(env!("CARGO_BIN_NAME"));
            home.push(&cli.profile);
            home.add_extension("ini");
            home
        }
    }
}

fn handle_device_async<'a>(path: &Path, handler: &'a impl Handler, executor: &LocalExecutor<'a>) {
    match Device::from_syspath(path) {
        Err(err) => log::error!("Could not open G13 device at {:}: {}", path.display(), err),
        Ok(dev) => {
            let path = path.to_path_buf();

            executor
                .spawn(async move {
                    dev.into_event_loop(handler).await.unwrap_or_else(|err| {
                        log::error!(
                            "Device loop for {} stopped with error: {}",
                            path.display(),
                            err
                        );
                    })
                })
                .detach();
        }
    };
}

fn monitor_mode(handler: impl Handler) {
    let executor = LocalExecutor::new();
    let monitor = async {
        g13m::discovery_loop(|path| handle_device_async(path, &handler, &executor))
            .await
            .unwrap_or_else(|err| {
                log::error!("Discovery loop stopped with error: {}", err);
                exit(1);
            })
    };

    let list = async {
        let paths = list().unwrap_or_else(|err| {
            log::error!("Could no fetch existing devices: {:}", err);
            exit(1);
        });
        for path in paths.iter() {
            handle_device_async(path, &handler, &executor);
        }
    };

    future::block_on(executor.run(future::zip(monitor, list)));
}

fn device_mode(path: PathBuf, handler: impl Handler) {
    let dev = Device::from_syspath(&path).unwrap_or_else(|err| {
        log::error!("Could not open G13 device at {:}: {}", path.display(), err);
        exit(1);
    });

    let executor = LocalExecutor::new();
    future::block_on(executor.run(dev.into_event_loop(&handler))).unwrap_or_else(|err| {
        log::error!("Device loop stopped with error: {}", err);
        exit(1);
    });
}

fn oneshot_mode(handler: impl Handler) {
    let paths = list().unwrap_or_else(|err| {
        log::error!("Could not fetch existing devices: {:}", err);
        exit(1);
    });

    match paths.len() {
        0 => {
            log::error!("No G13 was found");
            exit(1);
        }
        1 => (),
        x => {
            log::warn!("{} G13s were found, only one will be mapped", x);
        }
    }

    device_mode(paths[0].clone(), handler)
}

fn main() {
    let cli = Cli::parse();
    init_logger(&cli);
    log::info!(
        "{:} v{:}",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let config_path = get_config_path(&cli);
    log::info!("Reading profile: {}", config_path.display());
    let (binds, colors, lua_path) = config::load(&config_path);

    let keyboard = Arc::new(Mutex::new(VirtualKeyboard::new().unwrap_or_else(|err| {
        log::error!("Could not create virtual keyboard: {}", err);
        exit(1);
    })));

    let lua_handler = lua_path.map(|path| {
        log::info!("Reading lua script: {}", path.display());
        LuaHandler::new(keyboard.clone(), &path).unwrap_or_else(|err| {
            log::error!("Error loading lua file: {}, {}", path.display(), err);
            exit(1);
        })
    });

    let static_handler = StaticHandler::new(keyboard.clone(), binds, colors);
    let handler = CombinedHandler::new(static_handler, lua_handler);

    if let Some(path) = cli.device {
        device_mode(path, handler)
    } else if cli.oneshot {
        oneshot_mode(handler)
    } else {
        monitor_mode(handler)
    }
}
