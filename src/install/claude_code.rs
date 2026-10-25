//! Claude Code's config: chert's hook entries in `settings.json` and
//! its MCP server entry in `~/.claude.json`, added, removed and read
//! back (src/install T1, T3, R2, V4).
//!
//! An entry is chert's when it is what chert writes: a hook command
//! `<chert> hook run <event> --ide claude-code` whose program is named
//! `chert`, and the server `mcpServers.chert`. Nothing else is touched.
//! A server under `mcpServers.chert` in `settings.json`, where chert
//! once put it (B1), goes on install: Claude Code never read it there.

#[cfg(test)]
mod tests;

use std::path::Path;

use serde_json::{Map, Value, json};

use super::{Wire, WireState};

/// Each hook chert registers: the event as `chert hook run` names it,
/// and Claude Code's name for it.
const EVENTS: [(&str, &str); 5] = [
    ("session-start", "SessionStart"),
    ("user-prompt-submit", "UserPromptSubmit"),
    ("post-tool-use", "PostToolUse"),
    ("stop", "Stop"),
    ("session-end", "SessionEnd"),
];

/// The `mcpServers` key chert's server sits under.
pub(super) const SERVER: &str = "chert";

/// The name a wrong-typed top of `~/.claude.json` is reported under.
const MCP_FILE: &str = "claude.json";

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

/// The hook command for `event`, running `chert`.
pub(super) fn command(chert: &str, event: &str) -> String {
    format!("{} hook run {event} --ide claude-code", quote(chert))
}

/// Whether `command` is a hook chert wrote for `event`, whichever
/// `chert` program it runs.
pub(super) fn owned(command: &str, event: &str) -> bool {
    let tail = format!(" hook run {event} --ide claude-code");
    command.strip_suffix(&tail).is_some_and(|program| {
        let program = unquote(program);
        Path::new(&program)
            .file_name()
            .is_some_and(|n| n == "chert")
    })
}

/// The MCP server entry, as `claude mcp add --scope user chert --
/// <chert> mcp` writes it (src/install T3).
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

/// Whether hook object `hook` is chert's for `event`.
fn owned_hook(hook: &Value, event: &str) -> bool {
    let command = hook.get("command").and_then(Value::as_str);
    command.is_some_and(|c| owned(c, event))
}

/// Add chert's hooks to settings `root`, and drop a server chert put
/// there (B1); a hook already there is kept as it is, and 1 running
/// another `chert` is replaced.
///
/// # Errors
/// The key whose value is of the wrong JSON type.
pub(super) fn wire(root: &mut Value, chert: &str) -> Result<(), String> {
    let top = object(root, "settings")?;
    let hooks = object_at(top, "hooks")?;
    for (event, name) in EVENTS {
        let entry = hooks.entry(name).or_insert_with(|| json!([]));
        let groups = entry.as_array_mut().ok_or_else(|| name.to_owned())?;
        hook_in(groups, &command(chert, event), event);
    }
    drop_server(top);
    Ok(())
}

/// Add chert's server to `~/.claude.json` `root`.
///
/// # Errors
/// The key whose value is of the wrong JSON type.
pub(super) fn serve(root: &mut Value, chert: &str) -> Result<(), String> {
    let top = object(root, MCP_FILE)?;
    object_at(top, "mcpServers")?;
    let _ = chert;
    Ok(())
}

/// Drop chert's server from `~/.claude.json` `root`, and `mcpServers`
/// when that emptied it.
///
/// # Errors
/// The top is not an object.
pub(super) fn unserve(root: &mut Value) -> Result<(), String> {
    drop_server(object(root, MCP_FILE)?);
    Ok(())
}

/// Drop `mcpServers.chert` from `top`, and `mcpServers` when that
/// emptied it; a wrong-typed `mcpServers` is left alone.
fn drop_server(top: &mut Map<String, Value>) {
    let servers = top.get_mut("mcpServers").and_then(Value::as_object_mut);
    let gone = servers.is_some_and(|s| s.remove(SERVER).is_some());
    prune(top, "mcpServers", gone);
}

/// Append a group running `cmd` unless 1 runs it already.
fn hook_in(groups: &mut Vec<Value>, cmd: &str, event: &str) {
    if commands(groups).any(|c| c == cmd) {
        return;
    }
    drop_owned(groups, event);
    let hook = json!({"type": "command", "command": cmd});
    groups.push(json!({"hooks": [hook]}));
}

/// Drop chert's hooks for `event`, and each group they leave empty;
/// whether any went.
fn drop_owned(groups: &mut Vec<Value>, event: &str) -> bool {
    let mut changed = false;
    groups.retain_mut(|g| keep_group(g, event, &mut changed));
    changed
}

/// Drop chert's hooks from `group`; `false` when that emptied it.
fn keep_group(group: &mut Value, event: &str, changed: &mut bool) -> bool {
    let inner = group.get_mut("hooks").and_then(Value::as_array_mut);
    let Some(inner) = inner else { return true };
    let before = inner.len();
    inner.retain(|h| !owned_hook(h, event));
    if inner.len() == before {
        return true;
    }
    *changed = true;
    !inner.is_empty()
}

/// Remove every entry chert wrote from settings `root`, and each
/// container only that removal left empty; nothing else.
///
/// # Errors
/// The key whose value is of the wrong JSON type.
pub(super) fn unwire(root: &mut Value) -> Result<(), String> {
    let top = object(root, "settings")?;
    let mut changed = false;
    if let Some(hooks) = top.get_mut("hooks") {
        let hooks = object(hooks, "hooks")?;
        for (event, name) in EVENTS {
            changed |= unhook(hooks, name, event)?;
        }
    }
    prune(top, "hooks", changed);
    drop_server(top);
    Ok(())
}

/// Drop chert's hooks of event `name`, and the list when that emptied
/// it; whether any went.
fn unhook(
    hooks: &mut Map<String, Value>,
    name: &str,
    event: &str,
) -> Result<bool, String> {
    let Some(entry) = hooks.get_mut(name) else {
        return Ok(false);
    };
    let groups = entry.as_array_mut().ok_or_else(|| name.to_owned())?;
    let changed = drop_owned(groups, event);
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

/// How each of chert's entries stands, hooks in `settings` in event
/// order, then the server in `mcp`, against the ones `chert` would
/// write.
pub(super) fn wiring(settings: &Value, mcp: &Value, chert: &str) -> Vec<Wire> {
    let hooks = EVENTS
        .iter()
        .map(|&(e, name)| hook(settings, chert, e, name));
    hooks.chain([server_wire(mcp, chert)]).collect()
}

/// How the hook of event `name` stands.
fn hook(root: &Value, chert: &str, event: &str, name: &str) -> Wire {
    let at = root.get("hooks").and_then(|h| h.get(name));
    let groups = at.and_then(Value::as_array).map_or(&[][..], Vec::as_slice);
    let want = command(chert, event);
    let other = commands(groups).find(|c| owned(c, event));
    let state = if commands(groups).any(|c| c == want) {
        WireState::Ok
    } else {
        other.map_or(WireState::Missing, |c| WireState::Other(c.to_owned()))
    };
    Wire {
        name: format!("hook {name}"),
        state,
    }
}

/// How the MCP server entry stands.
fn server_wire(root: &Value, chert: &str) -> Wire {
    let at = root.get("mcpServers").and_then(|s| s.get(SERVER));
    let state = match at {
        None => WireState::Missing,
        Some(v) if *v == server(chert) => WireState::Ok,
        Some(v) => WireState::Other(v.to_string()),
    };
    Wire {
        name: format!("mcp server {SERVER}"),
        state,
    }
}
