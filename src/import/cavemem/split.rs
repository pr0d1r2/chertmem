//! cavemem rows split by their session's `cwd`: each session's rows go
//! to the chert project of its own cwd, the [`crate::store::project_id`]
//! the hook derives from it; a session without one goes to the current
//! project and is counted (src/import T5).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::path::Path;

use super::{Error, Imported, summary};
use crate::config::Config;
use crate::store::{Event, Mode, Store, Summary};

/// A session's `cwd`; `None` when cavemem has none or a blank one.
pub(super) type Cwd = Option<String>;

/// `cwd` as read, a blank one as none.
pub(super) fn cwd(text: Option<String>) -> Cwd {
    text.filter(|c| !c.trim().is_empty())
}

/// The rows bound for 1 project.
#[derive(Default)]
struct Batch {
    events: Vec<Event>,
    notes: Vec<Summary>,
}

/// Store `events` and `notes`, each in the project of its cwd, a
/// missing one in the project at `root`; 1 reading open per project.
pub(super) fn put(
    config: &Config,
    root: &Path,
    events: Vec<(Cwd, Event)>,
    notes: Vec<(Cwd, Summary)>,
) -> Result<Imported, Error> {
    let mut got = Imported::default();
    for (cwd, batch) in grouped(events, notes) {
        let home = cwd.as_deref().map_or(root, Path::new);
        let mut store = Store::open(config, home, Mode::Read)?;
        for event in &batch.events {
            store.append(event)?;
        }
        summary::put(&mut store, &batch.notes)?;
        count(&mut got, cwd.is_none(), &batch);
    }
    Ok(got)
}

/// The rows by cwd, each keeping cavemem's order.
fn grouped(
    events: Vec<(Cwd, Event)>,
    notes: Vec<(Cwd, Summary)>,
) -> BTreeMap<Cwd, Batch> {
    let mut by = BTreeMap::<Cwd, Batch>::new();
    for (cwd, event) in events {
        by.entry(cwd).or_default().events.push(event);
    }
    for (cwd, note) in notes {
        by.entry(cwd).or_default().notes.push(note);
    }
    by
}

/// Add `batch` to `got`; to `no_cwd` too when it had no cwd.
fn count(got: &mut Imported, no_cwd: bool, batch: &Batch) {
    let (rows, notes) = (batch.events.len(), batch.notes.len());
    got.observations = got.observations.saturating_add(rows);
    got.summaries = got.summaries.saturating_add(notes);
    if no_cwd {
        got.no_cwd = got.no_cwd.saturating_add(rows.saturating_add(notes));
    }
}
