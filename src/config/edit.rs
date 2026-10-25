//! `chert config set`: 1 key written into 1 `chert.toml`, every other
//! byte of the file kept (src/config T6, V3, V4).

#[cfg(test)]
mod tests;

use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, Value};

use super::source::{KEYS, Source, read_text};
use super::{Config, Error};

/// Set `key` to `value_text` in `file` (the user or the project file of
/// [`super::paths`]), creating it when absent. `value_text` is a TOML
/// value (`14`, `"14d"`, `["time"]`); text that is not one is taken as
/// a string, so `14d` means `"14d"`. The file must then load as
/// [`super::load_from`] demands (V3), else nothing is written. Turning
/// `inject.on_prompt` on hands back the speed disclaimer (V4).
///
/// # Errors
/// An unknown key, a bad value, an unreadable, invalid or unwritable
/// file.
pub fn set(
    file: &Path,
    key: &str,
    value_text: &str,
) -> Result<Option<&'static str>, Error> {
    let unknown = || Error::UnknownKey {
        file: file.to_path_buf(),
        key: key.to_owned(),
    };
    if !KEYS.contains(&key) {
        return Err(unknown());
    }
    let old = read_text(file)?.unwrap_or_default();
    let new = edited(file, &old, key, value(value_text))?;
    let config = check(file, &new)?;
    write(file, &new)?;
    let mut inject = config.inject;
    inject.on_prompt = check(file, &old).is_ok_and(|c| c.inject.on_prompt);
    Ok(inject.set_on_prompt(config.inject.on_prompt))
}

/// `old` with `key` set; a key whose parent is not a table means `old`
/// itself does not load, and that error comes back.
fn edited(
    file: &Path,
    old: &str,
    key: &str,
    value: Value,
) -> Result<String, Error> {
    let mut doc = old.parse::<DocumentMut>().map_err(|e| Error::Parse {
        file: file.to_path_buf(),
        reason: e.to_string().trim().to_owned(),
    })?;
    let Some(item) = slot(&mut doc, key) else {
        return Err(check(file, old).err().unwrap_or(Error::UnknownKey {
            file: file.to_path_buf(),
            key: key.to_owned(),
        }));
    };
    replace(item, value);
    Ok(doc.to_string())
}

/// The item at the dotted `key`, missing sections made as `[section]`
/// tables (or inside an inline table, inline); `None` past a value.
fn slot<'a>(doc: &'a mut DocumentMut, key: &str) -> Option<&'a mut Item> {
    let mut item = doc.as_item_mut();
    let mut parts = key.split('.').peekable();
    while let Some(part) = parts.next() {
        let in_table = item.is_table();
        item = item.get_mut(part)?;
        if in_table && item.is_none() && parts.peek().is_some() {
            let mut table = Table::new();
            table.set_implicit(true);
            *item = Item::Table(table);
        }
    }
    Some(item)
}

/// Put `value` in `item`, keeping the old value's comments and spacing.
fn replace(item: &mut Item, mut value: Value) {
    if let Some(old) = item.as_value() {
        *value.decor_mut() = old.decor().clone();
    }
    *item = Item::Value(value);
}

fn value(text: &str) -> Value {
    text.parse::<Value>().unwrap_or_else(|_| Value::from(text))
}

/// `text` as `file` would load on its own (V3).
fn check(file: &Path, text: &str) -> Result<Config, Error> {
    Config::build(&Source::of_text(file, text)?, None)
}

fn write(file: &Path, text: &str) -> Result<(), Error> {
    let dir = file.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)
        .and_then(|()| std::fs::write(file, text))
        .map_err(|e| Error::Write {
            file: file.to_path_buf(),
            reason: e.to_string(),
        })
}
