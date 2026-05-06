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

use std::path::{Path, PathBuf};
use std::process::exit;

use ini::Ini;

use nom::{
    Finish, IResult, Parser,
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while_m_n},
    character::char,
    character::complete::{alphanumeric1, digit1, one_of, space0},
    combinator::{all_consuming, map, map_opt, map_res, rest, value},
    error::Error,
    multi::{count, fold},
    sequence::{delimited, preceded, terminated},
};

use g13m::{
    Rgb,
    handlers::static_handler::{Binds, Colors},
    virtual_keyboard::{Bind, KeyCode, Modifiers, string_to_code},
};

#[derive(Debug, Clone)]
enum EntryKey {
    Bind(usize),
    Script,
    Color,
}

#[derive(Debug, Clone)]
enum Script {
    Lua,
    Path(PathBuf),
}

pub fn load(path: &Path) -> (Binds, Colors, Option<PathBuf>) {
    let config = Ini::load_from_file(path).unwrap_or_else(|err| {
        log::error!("Could not load config {:}: {:}", path.display(), err);
        exit(1);
    });

    let mut binds: Binds = [[None; 29]; 3];
    let mut colors: Colors = [None; 3];
    let mut script: Option<PathBuf> = None;

    for (key, value) in config.general_section() {
        match validate_parse_key(key) {
            EntryKey::Bind(idx) => {
                let bind = validate_parse_bind(value);
                for mode in binds.iter_mut() {
                    mode[idx] = Some(bind);
                }
            }
            EntryKey::Color => colors.fill(Some(validate_parse_color(value))),
            EntryKey::Script => script = Some(validate_parse_script(value, path)),
        }
    }

    for (section, props) in config.iter() {
        if let Some(mode) = section {
            let mode = validate_parse_mode(mode);

            for (key, value) in props {
                match validate_parse_key(key) {
                    EntryKey::Bind(idx) => binds[mode][idx] = Some(validate_parse_bind(value)),
                    EntryKey::Color => colors[mode] = Some(validate_parse_color(value)),
                    EntryKey::Script => {
                        log::error!(
                            "Error while parsing config: `script` may only occur in the top section"
                        );
                        exit(1);
                    }
                }
            }
        }
    }

    (binds, colors, script)
}

// -------------- //
// Error Handling //
// -------------- //

fn validate_parse_color(s: &str) -> Rgb {
    validate(s, "color", parse_color)
}
fn validate_parse_bind(s: &str) -> Bind {
    validate(s, "keybind", parse_bind)
}
fn validate_parse_mode(s: &str) -> usize {
    validate(s, "section header", parse_mode)
}
fn validate_parse_key(s: &str) -> EntryKey {
    validate(s, "key", parse_entry_key)
}

fn validate_parse_script(s: &str, path: &Path) -> PathBuf {
    match validate(s, "script", parse_script) {
        Script::Path(p) => path.parent().unwrap().join(p),
        Script::Lua => {
            let mut path = PathBuf::from(path);
            path.set_extension("lua");
            path
        }
    }
}

fn validate<T>(s: &str, what: &str, parse: fn(&str) -> Result<T, Error<&str>>) -> T {
    parse(s).unwrap_or_else(|err| {
        log::error!(
            "Error while parsing config: '{:}' is not a valid {:} ({:})",
            s,
            what,
            err
        );
        exit(1);
    })
}

// ------- //
// Parsing //
// ------- //

fn parse_mode(s: &str) -> Result<usize, Error<&str>> {
    all_consuming(preceded(one_of("mM"), one_of("123")))
        .parse(s)
        .finish()
        .map(|(_, c)| c.to_digit(4).unwrap() as usize - 1)
}

// Keys
// ----

fn parse_entry_key(s: &str) -> Result<EntryKey, Error<&str>> {
    alt((parse_color_key, parse_bind_key, parse_script_key))
        .parse(s)
        .finish()
        .map(|(_, e)| e)
}

fn parse_bind_key(s: &str) -> IResult<&str, EntryKey> {
    alt((parse_g_bind_key, parse_named_bind_key)).parse(s)
}

fn parse_g_bind_key(s: &str) -> IResult<&str, EntryKey> {
    all_consuming(preceded(
        one_of("gG"),
        map_opt(digit1, |s: &str| match s.parse::<usize>().ok() {
            Some(i) if (1..=22).contains(&i) => Some(EntryKey::Bind(i - 1)),
            _ => None,
        }),
    ))
    .parse(s)
}

fn parse_named_bind_key(s: &str) -> IResult<&str, EntryKey> {
    all_consuming(alt((
        value(22, tag_no_case("thumb left")),
        value(23, tag_no_case("thumb right")),
        value(24, tag_no_case("thumb stick")),
        value(25, tag_no_case("up")),
        value(26, tag_no_case("down")),
        value(27, tag_no_case("left")),
        value(28, tag_no_case("right")),
    )))
    .map(EntryKey::Bind)
    .parse(s)
}

fn parse_color_key(s: &str) -> IResult<&str, EntryKey> {
    all_consuming(value(EntryKey::Color, tag_no_case("color"))).parse(s)
}

fn parse_script_key(s: &str) -> IResult<&str, EntryKey> {
    all_consuming(value(EntryKey::Script, tag_no_case("script"))).parse(s)
}

// Values
// ------

fn parse_color(s: &str) -> Result<Rgb, Error<&str>> {
    let hex_color = map_res(take_while_m_n(2, 2, |c: char| c.is_ascii_hexdigit()), |s| {
        u8::from_str_radix(s, 16)
    });

    all_consuming(preceded(tag("#"), count(hex_color, 3)))
        .parse(s)
        .finish()
        .map(|(_, v)| Rgb(v[0], v[1], v[2]))
}

fn parse_script(s: &str) -> Result<Script, Error<&str>> {
    alt((parse_script_name, parse_script_path))
        .parse(s)
        .finish()
        .map(|(_, v)| v)
}

fn parse_script_name(s: &str) -> IResult<&str, Script> {
    all_consuming(value(Script::Lua, tag_no_case("lua"))).parse(s)
}

fn parse_script_path(s: &str) -> IResult<&str, Script> {
    all_consuming(map(rest, |path: &str| Script::Path(PathBuf::from(path)))).parse(s)
}

fn parse_bind(s: &str) -> Result<Bind, Error<&str>> {
    (parse_modifiers, parse_key)
        .parse(s)
        .finish()
        .map(|(_, v)| v)
}

fn parse_key(s: &str) -> IResult<&str, KeyCode> {
    all_consuming(map_opt(alphanumeric1, string_to_code)).parse(s)
}

fn parse_modifiers(s: &str) -> IResult<&str, Modifiers> {
    let modifier = alt((
        // TODO: use string_to_code here so names are only defined in one place.
        // To make this work we need a keycode -> modifier mapping.
        value(Modifiers::L_SHIFT, tag_no_case("shift")),
        value(Modifiers::L_SHIFT, tag_no_case("lshift")),
        value(Modifiers::R_SHIFT, tag_no_case("rshift")),
        value(Modifiers::R_ALT, tag_no_case("altgr")),
        value(Modifiers::L_ALT, tag_no_case("alt")),
        value(Modifiers::L_ALT, tag_no_case("lalt")),
        value(Modifiers::R_ALT, tag_no_case("ralt")),
        value(Modifiers::L_CTRL, tag_no_case("ctrl")),
        value(Modifiers::L_CTRL, tag_no_case("lctrl")),
        value(Modifiers::R_CTRL, tag_no_case("rctrl")),
        value(Modifiers::L_META, tag_no_case("meta")),
        value(Modifiers::L_META, tag_no_case("lmeta")),
        value(Modifiers::R_META, tag_no_case("rmeta")),
        value(Modifiers::L_META, tag_no_case("super")),
        value(Modifiers::L_META, tag_no_case("lsuper")),
        value(Modifiers::R_META, tag_no_case("rsuper")),
    ));

    fold(
        0..,
        terminated(modifier, delimited(space0, char('+'), space0)),
        Modifiers::empty,
        Modifiers::union,
    )
    .parse(s)
}
