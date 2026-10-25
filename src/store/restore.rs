//! Restore: an exported observation stored back under its own id, ts,
//! project and issued ref (`src/import` V1, root V15).

#[cfg(test)]
mod tests;

use super::write::{Ctx, Row, immediate, slot};
use super::{
    Appended, Error, Observation, Ref, Store, catalog, partition, repartition,
};

impl Store {
    /// Store `observation`, an export read back, under its own id, ts,
    /// identity and issued ref, in the bucket of its own ts (src/store
    /// V1) and under its own project, which may differ from this
    /// store's: an export can hold several. The ref is stored verbatim,
    /// never minted again. Its id already stored gives back that row's
    /// ref and stores nothing; so does its identity (V9). Allowed in
    /// both modes; a lock outlasting the busy timeout is
    /// [`Error::Busy`], never spooled, since a replay would mint a new
    /// ref.
    ///
    /// # Errors
    /// [`Error::RefTaken`] when another id holds its ref in its project
    /// (root V15); otherwise as [`Store::append`]: a sealed target
    /// routes to its bucket's fresh partition (V13).
    pub fn restore(
        &mut self,
        observation: &Observation,
    ) -> Result<Appended, Error> {
        let (row, issued) = repartition::row(observation.clone());
        let stored = put(&mut self.ctx()?, &row, &issued)?;
        Ok(Appended::Stored(stored))
    }
}

/// Store `row` under `issued` unless its id or identity is stored.
fn put(cx: &mut Ctx<'_>, row: &Row, issued: &str) -> Result<Ref, Error> {
    let tx = immediate(cx.catalog)?;
    if let Some(found) = catalog::existing(&tx, row)? {
        return Ok(Ref::stored(found));
    }
    if catalog::taken(&tx, &row.project, issued)? {
        let (reference, id) = (issued.to_owned(), row.id.clone());
        return Err(Error::RefTaken { reference, id });
    }
    let (place, conn) = slot(cx.plan, cx.chunks, &tx, row.key())?;
    let stored = partition::insert(conn, row, issued)?;
    catalog::record(&tx, &place, row, &stored)?;
    tx.commit()?;
    Ok(Ref::stored(stored))
}
