//! `chert config show|get|set|path` (root I) over `config::{show, get,
//! set, paths}`, home and project injected through [`Io`].

#[cfg(test)]
mod tests;

use std::path::Path;

use clap::{ArgMatches, value_parser};

use super::args::{flag, one, value, verb};
use super::setup::no_home;
use super::{Done, Io, fail, say};
use crate::config::{self, Entry, Origin};

#[derive(Debug)]
pub(super) enum Config {
    Show,
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
        user: bool,
    },
    Path,
}

/// `config show|get|set|path`.
pub(super) fn commands() -> [clap::Command; 4] {
    let key = || value("key", "KEY", value_parser!(String));
    let value = value("value", "VALUE", value_parser!(String));
    let user = flag("user", "USER", "user");
    let set = verb("set", SET)
        .arg(key())
        .arg(value.help(VALUE))
        .arg(user.help("Write `~/.config/chert/chert.toml` instead"));
    [
        verb("show", SHOW),
        verb("get", "1 key's effective value").arg(key()),
        set,
        verb("path", "The user and project `chert.toml` paths"),
    ]
}

const SHOW: &str = "Every key's effective value and the file it came from";
const SET: &str = "Set 1 key in the project file, or the user file with --user";
const VALUE: &str = "A TOML value; text that is not one is taken as a string";

/// The action `name` names, when it is 1 of `config`'s.
pub(super) fn parse(name: &str, m: &ArgMatches) -> Option<Config> {
    match name {
        "show" => Some(Config::Show),
        "get" => one(m, "key").map(|key| Config::Get { key }),
        "set" => Some(Config::Set {
            key: one(m, "key")?,
            value: one(m, "value")?,
            user: m.get_flag("user"),
        }),
        "path" => Some(Config::Path),
        _ => None,
    }
}

pub(super) fn run(action: Config, io: &mut Io<'_>) -> Done {
    match action {
        Config::Show => show(io),
        Config::Get { key } => get(&key, io),
        Config::Set { key, value, user } => set(&key, &value, user, io),
        Config::Path => path(io),
    }
}

/// `<key> = <value>  # <origin>` per key.
fn show(io: &mut Io<'_>) -> Done {
    for entry in config::show(io.home, io.cwd).map_err(fail)? {
        say(io, &line(&entry))?;
    }
    Ok(0)
}

fn line(entry: &Entry) -> String {
    let value = entry.value.as_deref().unwrap_or("(unset)");
    let origin = match &entry.origin {
        Origin::Default => "default".to_owned(),
        Origin::User(file) => format!("user {}", file.display()),
        Origin::Project(file) => format!("project {}", file.display()),
    };
    format!("{} = {value}  # {origin}", entry.key)
}

/// The effective value alone; an unknown key is exit 2.
fn get(key: &str, io: &mut Io<'_>) -> Done {
    let entry = config::get(io.home, io.cwd, key).map_err(fail)?;
    let entry = entry.ok_or_else(|| {
        format!("chert: unknown key {key} (see chert config show)")
    })?;
    say(io, entry.value.as_deref().unwrap_or("(unset)"))?;
    Ok(0)
}

/// Write 1 key; the `inject.on_prompt` disclaimer printed when given.
fn set(key: &str, value: &str, user: bool, io: &mut Io<'_>) -> Done {
    let paths = config::paths(io.home, io.cwd);
    let file = if user {
        paths.user.ok_or_else(no_home)?
    } else {
        paths.project
    };
    let note = config::set(&file, key, value).map_err(fail)?;
    say(io, &format!("set {key} in {}", file.display()))?;
    if let Some(text) = note {
        say(io, text)?;
    }
    Ok(0)
}

/// `user <path>` (or `user (no home)`) and `project <path>`.
fn path(io: &mut Io<'_>) -> Done {
    let paths = config::paths(io.home, io.cwd);
    let user = paths.user.as_deref().map(Path::display);
    let user = user.map_or("(no home)".to_owned(), |p| p.to_string());
    say(io, &format!("user {user}"))?;
    say(io, &format!("project {}", paths.project.display()))?;
    Ok(0)
}
