//! 1 spool line: a JSON object of strings (the timestamp in decimal, so
//! no value passes through a float), `type` naming what it holds
//! (src/store I).

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use tinyjson::JsonValue;

use super::session::{Begun, Ended};
use super::write::{Item, Note, Row};
use super::{Error, Event, Scope, SessionStart, Summary};

type Map = HashMap<String, JsonValue>;

/// The line of `item`, without its newline.
pub(super) fn encode(item: &Item) -> Result<String, Error> {
    let fields = match item {
        Item::Observation(row) => observation(row),
        Item::Summary(note) => summary(note),
        Item::Begun(begun) => started(begun),
        Item::Ended(ended) => finished(ended),
    };
    let object = fields.into_iter().map(|(k, v)| (k.to_owned(), v));
    let value = JsonValue::Object(object.collect::<Map>());
    value.stringify().map_err(|e| Error::Sqlite {
        reason: format!("spool line: {e}"),
    })
}

fn text(s: &str) -> JsonValue {
    JsonValue::String(s.to_owned())
}

fn observation(row: &Row) -> Vec<(&'static str, JsonValue)> {
    let e = &row.event;
    let identity = e.identity.as_deref().map_or(JsonValue::Null, text);
    vec![
        ("type", text("observation")),
        ("id", text(&row.id)),
        ("project", text(&row.project)),
        ("ts", text(&e.ts.to_string())),
        ("session", text(&e.session)),
        ("kind", text(&e.kind)),
        ("ide", text(&e.ide)),
        ("identity", identity),
        ("body", text(&e.body)),
    ]
}

fn summary(note: &Note) -> Vec<(&'static str, JsonValue)> {
    let s = &note.summary;
    vec![
        ("type", text("summary")),
        ("id", text(&note.id)),
        ("project", text(&note.project)),
        ("ts", text(&s.ts.to_string())),
        ("session", text(&s.session)),
        ("scope", text(s.scope.as_str())),
        ("ide", text(&s.ide)),
        ("body", text(&s.body)),
    ]
}

fn started(begun: &Begun) -> Vec<(&'static str, JsonValue)> {
    let s = &begun.start;
    vec![
        ("type", text("session-start")),
        ("id", text(&s.id)),
        ("project", text(&begun.project)),
        ("ts", text(&s.ts.to_string())),
        ("ide", text(&s.ide)),
        ("cwd", text(&s.cwd)),
    ]
}

fn finished(ended: &Ended) -> Vec<(&'static str, JsonValue)> {
    vec![
        ("type", text("session-end")),
        ("id", text(&ended.id)),
        ("project", text(&ended.project)),
        ("ts", text(&ended.ts.to_string())),
    ]
}

/// The item of 1 spool line; `None` for anything else.
pub(super) fn decode(line: &str) -> Option<Item> {
    let value: JsonValue = line.parse().ok()?;
    let map: &Map = value.get()?;
    let (id, project) = (field(map, "id")?, field(map, "project")?);
    match field(map, "type")?.as_str() {
        "observation" => {
            let event = event(map)?;
            Some(Item::Observation(Row { id, project, event }))
        }
        "summary" => Some(Item::Summary(note(map, id, project)?)),
        "session-start" => Some(Item::Begun(begun(map, id, project)?)),
        "session-end" => Some(Item::Ended(ended(map, id, project)?)),
        _ => None,
    }
}

fn ended(map: &Map, id: String, project: String) -> Option<Ended> {
    let ts = field(map, "ts")?.parse().ok()?;
    Some(Ended { project, id, ts })
}

fn begun(map: &Map, id: String, project: String) -> Option<Begun> {
    let start = SessionStart {
        id,
        ide: field(map, "ide")?,
        cwd: field(map, "cwd")?,
        ts: field(map, "ts")?.parse().ok()?,
    };
    Some(Begun { project, start })
}

fn note(map: &Map, id: String, project: String) -> Option<Note> {
    let summary = summary_of(map)?;
    Some(Note {
        id,
        project,
        summary,
    })
}

fn field(map: &Map, key: &str) -> Option<String> {
    map.get(key)?.get::<String>().cloned()
}

fn event(map: &Map) -> Option<Event> {
    let identity = match map.get("identity")? {
        JsonValue::Null => None,
        other => Some(other.get::<String>()?.clone()),
    };
    Some(Event {
        ts: field(map, "ts")?.parse().ok()?,
        session: field(map, "session")?,
        kind: field(map, "kind")?,
        ide: field(map, "ide")?,
        identity,
        body: field(map, "body")?,
    })
}

fn summary_of(map: &Map) -> Option<Summary> {
    Some(Summary {
        ts: field(map, "ts")?.parse().ok()?,
        session: field(map, "session")?,
        scope: Scope::parse(&field(map, "scope")?)?,
        ide: field(map, "ide")?,
        body: field(map, "body")?,
    })
}
