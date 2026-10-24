//! 1 tool call's `arguments`, checked as cavemem's zod schemas check
//! them; a bad one is a message naming the tool and the argument.

#[cfg(test)]
mod tests;

use serde_json::{Map, Value};

use crate::store::Ref;

/// The arguments of a call to `tool`; absent arguments read as `{}`.
pub(super) struct Args<'a> {
    tool: &'a str,
    map: Option<&'a Map<String, Value>>,
}

impl<'a> Args<'a> {
    pub(super) fn new(tool: &'a str, arguments: &'a Value) -> Self {
        Self {
            tool,
            map: arguments.as_object(),
        }
    }

    /// Why `key` is refused.
    fn bad(&self, key: &str, why: &str) -> String {
        format!("Invalid arguments for tool {}: {key} {why}", self.tool)
    }

    fn get(&self, key: &str) -> Option<&'a Value> {
        self.map.and_then(|m| m.get(key)).filter(|v| !v.is_null())
    }

    /// A required, non-empty string.
    pub(super) fn text(&self, key: &str) -> Result<String, String> {
        match self.get(key).and_then(Value::as_str) {
            Some(text) if !text.is_empty() => Ok(text.to_owned()),
            _ => Err(self.bad(key, "must be a non-empty string")),
        }
    }

    /// An optional integer in `1..=max`.
    pub(super) fn count(
        &self,
        key: &str,
        max: u32,
    ) -> Result<Option<u32>, String> {
        let Some(value) = self.get(key) else {
            return Ok(None);
        };
        let n = value.as_u64().and_then(|n| u32::try_from(n).ok());
        match n.filter(|n| (1..=max).contains(n)) {
            Some(n) => Ok(Some(n)),
            None => {
                Err(self.bad(key, &format!("must be an integer 1 to {max}")))
            }
        }
    }

    /// An optional boolean.
    pub(super) fn flag(&self, key: &str) -> Result<Option<bool>, String> {
        self.get(key)
            .map(|v| {
                v.as_bool()
                    .ok_or_else(|| self.bad(key, "must be a boolean"))
            })
            .transpose()
    }

    /// An optional `chert:<ref>`.
    pub(super) fn reference(&self, key: &str) -> Result<Option<Ref>, String> {
        self.get(key).map(|v| self.one(key, v)).transpose()
    }

    /// A required array of 1 to `max` refs.
    pub(super) fn refs(
        &self,
        key: &str,
        max: usize,
    ) -> Result<Vec<Ref>, String> {
        let items = self.get(key).and_then(Value::as_array);
        let Some(items) = items.filter(|a| (1..=max).contains(&a.len())) else {
            return Err(self.bad(key, &format!("must hold 1 to {max} refs")));
        };
        items.iter().map(|v| self.one(key, v)).collect()
    }

    fn one(&self, key: &str, value: &Value) -> Result<Ref, String> {
        let text = value.as_str().unwrap_or_default();
        Ref::parse(text).map_err(|_| self.bad(key, "must be chert:<ref>"))
    }
}
