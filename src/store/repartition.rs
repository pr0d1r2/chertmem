//! Repartition: rewrite every partition written under another
//! `partition.*` spec to the current one, each row under its own id and
//! ref, so every ref keeps resolving to its own row (src/store T6, V5).
//! 1 source partition per catalog transaction; the targets are written,
//! the source's moved rows or files go, then the catalog commits (V6).

#[cfg(test)]
mod tests;

use std::path::Path;

use rusqlite::Connection;

use super::layout::{Place, Plan};
use super::partition::{self, Chunks};
use super::write::{self, Note, Row, immediate};
use super::{
    Error, Event, Filter, Observation, State, Store, catalog, cold, generation,
    summary, tail,
};

/// A partition to rewrite: its path and tier.
struct Source {
    path: String,
    state: State,
}

/// A source's rows with their refs, and its summaries, each with the
/// place the current spec gives it.
#[derive(Default)]
struct Load {
    rows: Vec<(Row, String, Place)>,
    notes: Vec<(Note, Place)>,
}

/// What 1 source's rewrite reaches, under the catalog's write lock.
struct Move<'a> {
    tx: &'a Connection,
    chunks: &'a mut Chunks,
    plan: &'a Plan,
    from: &'a Source,
}

/// A stored observation as the row it was appended as, and its ref.
fn row(o: Observation) -> (Row, String) {
    let event = Event {
        ts: o.ts,
        session: o.session,
        kind: o.kind,
        ide: o.ide,
        identity: o.identity,
        body: o.body,
    };
    let (id, project) = (o.id, o.project);
    let reference = o.reference.as_str().to_owned();
    (Row { id, project, event }, reference)
}

impl Move<'_> {
    /// Every row and summary of the source, by seq, placed.
    fn load(&mut self) -> Result<Load, Error> {
        let Some(conn) = self.chunks.existing(&self.from.path)? else {
            return Ok(Load::default());
        };
        let found = tail::rows(conn, 0, &Filter::default())?;
        let notes = summary::all(conn)?;
        let mut load = Load::default();
        for (r, reference) in found.into_iter().map(|t| row(t.observation)) {
            let place = self.plan.place(r.key())?;
            load.rows.push((r, reference, place));
        }
        for note in notes {
            let place = self.plan.place(note.key())?;
            load.notes.push((note, place));
        }
        Ok(load)
    }

    /// Refuse before anything moves when a place is sealed: a target
    /// warm, cold or dropped, or a sealed source keeping rows (V7).
    fn check(&self, load: &Load) -> Result<(), Error> {
        let rows = load.rows.iter().map(|(_, _, p)| p);
        for place in rows.chain(load.notes.iter().map(|(_, p)| p)) {
            let ok = if place.path == self.from.path {
                self.from.state == State::Hot
            } else {
                let found = catalog::find(self.tx, &place.path)?;
                found.is_none_or(|(_, s)| s == Some(State::Hot))
            };
            if !ok {
                let path = place.path.clone();
                return Err(Error::ReadOnly { path });
            }
        }
        Ok(())
    }

    /// Write every row that leaves into its target, its ref pointed
    /// there; whether any row or summary stays in the source.
    fn carry(&mut self, load: &Load) -> Result<bool, Error> {
        let mut stays = false;
        for (r, reference, place) in &load.rows {
            if place.path == self.from.path {
                stays = true;
                continue;
            }
            let (slot, conn) =
                write::slot(self.plan, self.chunks, self.tx, r.key())?;
            partition::insert(conn, r, reference)?;
            catalog::moved(self.tx, &slot, r)?;
        }
        Ok(self.carry_notes(load)? || stays)
    }

    fn carry_notes(&mut self, load: &Load) -> Result<bool, Error> {
        let mut stays = false;
        for (note, place) in &load.notes {
            if place.path == self.from.path {
                stays = true;
                continue;
            }
            let (_, conn) =
                write::slot(self.plan, self.chunks, self.tx, note.key())?;
            summary::insert(conn, note)?;
        }
        Ok(stays)
    }

    /// Keep the source under the current spec without what left it.
    fn retain(&mut self, load: &Load) -> Result<(), Error> {
        let path = &self.from.path;
        let conn = self.chunks.get(path)?;
        for (r, _, place) in &load.rows {
            if place.path != *path {
                partition::remove(conn, &r.id)?;
            }
        }
        for (note, place) in &load.notes {
            if place.path != *path {
                summary::remove(conn, &note.id)?;
            }
        }
        partition::restamp(conn, self.plan)?;
        let stats = partition::stats(conn)?;
        catalog::restate(self.tx, path, self.plan, &stats)
    }

    /// Remove the emptied source's files, then its catalog row.
    fn clear(&mut self, root: &Path) -> Result<(), Error> {
        self.chunks.close(&self.from.path);
        cold::remove(root, &self.from.path)?;
        catalog::forget(self.tx, &self.from.path)
    }
}

impl Store {
    /// Rewrite every partition written under another `partition.*`
    /// spec to the current one, oldest first; every ref keeps resolving
    /// to its own row (src/store V5) and each rewrite bumps
    /// `generation`. Rewritten rows land hot; a sealed target refuses.
    ///
    /// # Errors
    /// [`Error::WriteOnly`] on a write-only store; [`Error::ReadOnly`]
    /// for a sealed target, before that source moves.
    pub fn repartition(&mut self) -> Result<(), Error> {
        self.reading()?;
        let catalog = self.catalog.as_ref().ok_or(Error::WriteOnly)?;
        let stale = catalog::stale(catalog, &self.plan.spec())?;
        for (path, state) in stale {
            self.rewrite(&Source { path, state })?;
        }
        Ok(())
    }

    /// Rewrite 1 source in 1 catalog transaction.
    fn rewrite(&mut self, from: &Source) -> Result<(), Error> {
        let catalog = self.catalog.as_ref().ok_or(Error::WriteOnly)?;
        let tx = immediate(catalog)?;
        let (chunks, plan) = (&mut self.chunks, &self.plan);
        Move {
            tx: &tx,
            chunks,
            plan,
            from,
        }
        .run(&self.root)?;
        generation::bump(&tx, &self.root)?;
        Ok(tx.commit()?)
    }
}

impl Move<'_> {
    /// Load, check, carry, then keep or clear the source.
    fn run(mut self, root: &Path) -> Result<(), Error> {
        let load = self.load()?;
        self.check(&load)?;
        if self.carry(&load)? {
            self.retain(&load)
        } else {
            self.clear(root)
        }
    }
}
