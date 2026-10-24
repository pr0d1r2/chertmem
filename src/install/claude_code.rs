//! Claude Code's `settings.json`: chert's hook entries and its MCP
//! server entry, added, removed and read back (src/install T1, R2).
//!
//! An entry is chert's when it is what chert writes: a hook command
//! `<chert> hook run <event> --ide claude-code` whose program is named
//! `chert`, and the server `mcpServers.chert`. Nothing else is touched.

#[cfg(test)]
mod tests;

use std::path::Path;

use serde_json::{Map, Value, json};

use crate::hook::Kind;

/// Each hook chert registers and Claude Code's name for its event. The
/// `PostToolUseFailure` row joins once `src/hook` T9 reads it.
const EVENTS: [(Kind, &str); 5] = [
    (Kind::SessionStart, "SessionStart"),
    (Kind::UserPromptSubmit, "UserPromptSubmit"),
    (Kind::PostToolUse, "PostToolUse"),
    (Kind::Stop, "Stop"),
    (Kind::SessionEnd, "SessionEnd"),
];

/// The `mcpServers` key chert's server sits under.
pub(super) const SERVER: &str = "chert";

/// `text` safe as 1 shell word: as it is when it holds only characters
/// no shell reads specially, else in single quotes.
pub(super) fn quote(text: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "@%+=:,./_-".contains(c);
    if !text.is_empty() && text.chars().all(plain) {
        return text.to_owned();
    }
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// The word [`quote`] made, back as it was.
pub(super) fn unquote(word: &str) -> String {
    let inner = word.strip_prefix('\'').and_then(|w| w.strip_suffix('\''));
    inner.map_or_else(|| word.to_owned(), |w| w.replace(r"'\''", "'"))
}

/// The hook command for `kind`, running `chert`.
pub(super) fn command(chert: &str, kind: Kind) -> String {
    let event = kind.as_str();
    format!("{} hook run {event} --ide claude-code", quote(chert))
}

/// Whether `command` is a hook chert wrote for `kind`, whichever
/// `chert` program it runs.
pub(super) fn owned(command: &str, kind: Kind) -> bool {
    let tail = format!(" hook run {} --ide claude-code", kind.as_str());
    command.strip_suffix(&tail).is_some_and(|program| {
        let program = unquote(program);
        Path::new(&program)
            .file_name()
            .is_some_and(|n| n == "chert")
    })
}

/// The MCP server entry, as cavemem writes its own (src/install R2).
pub(super) fn server(chert: &str) -> Value {
    json!({"command": chert, "args": ["mcp"]})
}

/// The object `value` holds, else the key it sits under as the error.
fn object<'a>(
    value: &'a mut Value,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    value.as_object_mut().ok_or_else(|| key.to_owned())
}

/// The object under `key`, made empty when there is none.
fn object_at<'a>(
    map: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    object(map.entry(key).or_insert_with(|| json!({})), key)
}

/// Every hook command in a list of matcher groups.
fn commands(groups: &[Value]) -> impl Iterator<Item = &str> {
    groups
        .iter()
        .filter_map(|g| g.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|h| h.get("command").and_then(Value::as_str))
}

/// Whether hook object `hook` is chert's for `kind`.
fn owned_hook(hook: &Value, kind: Kind) -> bool {
    let command = hook.get("command").and_then(Value::as_str);
    command.is_some_and(|c| owned(c, kind))
}

/// Add chert's hooks and MCP server to `root`; a hook already there is
/// kept as it is, and 1 running another `chert` is replaced.
///
/// # Errors
/// The key whose value is of the wrong JSON type.
pub(super) fn wire(root: &mut Value, chert: &str) -> Result<(), String> {
    let top = object(root, "settings")?;
    let hooks = object_at(top, "hooks")?;
    for (kind, name) in EVENTS {
        let entry = hooks.entry(name).or_insert_with(|| json!([]));
        let groups = entry.as_array_mut().ok_or_else(|| name.to_owned())?;
        hook_in(groups, &command(chert, kind), kind);
    }
    object_at(top, "mcpServers")?.insert(SERVER.to_owned(), server(chert));
    Ok(())
}

/// Append a group running `cmd` unless 1 runs it already.
fn hook_in(groups: &mut Vec<Value>, cmd: &str, kind: Kind) {
    if commands(groups).any(|c| c == cmd) {
        return;
    }
    drop_owned(groups, kind);
    let hook = json!({"type": "command", "command": cmd});
    groups.push(json!({"hooks": [hook]}));
}

/// Drop chert's hooks for `kind`, and each group they leave empty;
/// whether any went.
fn drop_owned(groups: &mut Vec<Value>, kind: Kind) -> bool {
    let mut changed = false;
    groups.retain_mut(|g| keep_group(g, kind, &mut changed));
    changed
}

/// Drop chert's hooks from `group`; `false` when that emptied it.
fn keep_group(group: &mut Value, kind: Kind, changed: &mut bool) -> bool {
    let inner = group.get_mut("hooks").and_then(Value::as_array_mut);
    let Some(inner) = inner else { return true };
    let before = inner.len();
    inner.retain(|h| !owned_hook(h, kind));
    if inner.len() == before {
        return true;
    }
    *changed = true;
    !inner.is_empty()
}

/// Remove every entry chert wrote from `root`, and each container only
/// that removal left empty; nothing else.
///
/// # Errors
/// The key whose value is of the wrong JSON type.
pub(super) fn unwire(root: &mut Value) -> Result<(), String> {
    let top = object(root, "settings")?;
    let mut changed = false;
    if let Some(hooks) = top.get_mut("hooks") {
        let hooks = object(hooks, "hooks")?;
        for (kind, name) in EVENTS {
            changed |= unhook(hooks, name, kind)?;
        }
    }
    prune(top, "hooks", changed);
    let servers = top.get_mut("mcpServers").and_then(Value::as_object_mut);
    let gone = servers.is_some_and(|s| s.remove(SERVER).is_some());
    prune(top, "mcpServers", gone);
    Ok(())
}

/// Drop chert's hooks of event `name`, and the list when that emptied
/// it; whether any went.
fn unhook(
    hooks: &mut Map<String, Value>,
    name: &str,
    kind: Kind,
) -> Result<bool, String> {
    let Some(entry) = hooks.get_mut(name) else {
        return Ok(false);
    };
    let groups = entry.as_array_mut().ok_or_else(|| name.to_owned())?;
    let changed = drop_owned(groups, kind);
    prune(hooks, name, changed);
    Ok(changed)
}

/// Remove `key` when `changed` left its object or list empty.
fn prune(map: &mut Map<String, Value>, key: &str, changed: bool) {
    let empty = map.get(key).is_some_and(|v| match v {
        Value::Object(o) => o.is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    });
    if changed && empty {
        map.remove(key);
    }
}
