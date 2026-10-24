//! 1 session: the frames it answers and the tools it runs (src/mcp
//! I). A failed tool call answers `isError`, as the SDK does; the
//! session goes on.

#[cfg(test)]
mod tests;

use serde_json::{Value, json};

use super::rpc::{self, Frame};
use super::tools::{self, Call};
use super::{Ctx, Error, shape};
use crate::search::{self, Engine, Query};
use crate::store::{Mode, Ref, Store};

/// What a session reads: the engine for `search` and `timeline`, a
/// store for `get_observations`.
pub(super) struct Server {
    engine: Engine,
    store: Store,
}

impl Server {
    /// Open the engine and store of `ctx`'s project.
    pub(super) fn open(ctx: &Ctx<'_>) -> Result<Self, Error> {
        Ok(Self {
            engine: Engine::open(ctx.config, ctx.root)?,
            store: Store::open(ctx.config, ctx.root, Mode::Read)?,
        })
    }

    /// The frame answering `line`, if any.
    pub(super) fn reply(&mut self, line: &str) -> Option<Value> {
        match rpc::parse(line) {
            Frame::Request { id, method, params } => {
                Some(self.answer(id, &method, &params))
            }
            Frame::Bad(frame) => Some(frame),
            Frame::Silent => None,
        }
    }

    fn answer(&mut self, id: Value, method: &str, params: &Value) -> Value {
        match method {
            "initialize" => rpc::ok(id, rpc::initialize(params)),
            "ping" => rpc::ok(id, json!({})),
            "tools/list" => rpc::ok(id, tools::list()),
            "tools/call" => rpc::ok(id, self.call(params)),
            _ => rpc::fail(id, rpc::NO_METHOD, "Method not found"),
        }
    }

    /// A tool's result, or its error as a result.
    fn call(&mut self, params: &Value) -> Value {
        let done = tools::parse(params)
            .and_then(|c| self.run(c).map_err(|e| e.to_string()));
        done.map_or_else(|m| shape::error(&m), |v| shape::content(&v))
    }

    fn run(&mut self, call: Call) -> Result<Value, search::Error> {
        match call {
            Call::Search { query, limit } => self.search(query, limit),
            Call::Timeline {
                session_id,
                around_id,
                limit,
            } => self.timeline(&session_id, around_id.as_ref(), limit),
            Call::Get { ids, expand } => {
                Ok(shape::observations(&self.store.get(&ids)?, expand))
            }
        }
    }

    /// The best hits in this project (src/mcp R2).
    fn search(
        &mut self,
        text: String,
        limit: Option<u32>,
    ) -> Result<Value, search::Error> {
        let q = Query {
            text,
            limit,
            project: Some(self.engine.project().to_owned()),
            semantic: true,
            ..Query::default()
        };
        Ok(shape::hits(&self.engine.run(&q, None)?))
    }

    fn timeline(
        &mut self,
        session: &str,
        around: Option<&Ref>,
        limit: Option<u32>,
    ) -> Result<Value, search::Error> {
        let rows = self.engine.timeline(session, around, limit)?;
        Ok(shape::entries(&rows))
    }
}
