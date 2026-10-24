//! The `ollama` embedding provider (src/search T4, root I): 1 HTTP/1.1
//! POST to `/api/embed` over a std `TcpStream`, no crate, no TLS. Only
//! built when `embedding.provider = "ollama"` and the query is semantic
//! (src/search V3, root V6).

#[cfg(test)]
mod tests;

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde_json::{Value, json};

use super::Error;
use super::blend::Embed;

/// The model asked for; `chert.toml` names none yet (src/search R2).
pub(super) const MODEL: &str = "nomic-embed-text";
/// Ollama's own default when `OLLAMA_HOST` is unset.
const PORT: &str = "11434";
const CONNECT: Duration = Duration::from_secs(2);
const REPLY: Duration = Duration::from_secs(30);

/// An ollama server at `host:port`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Ollama {
    pub(super) addr: String,
}

impl Ollama {
    /// The server `OLLAMA_HOST` names: `host`, `host:port` or
    /// `http://host:port`; empty means `127.0.0.1:11434`.
    pub(super) fn new(host: &str) -> Result<Self, Error> {
        let host = host.trim();
        if host.starts_with("https://") {
            return Err(fail("https is not supported, give http://"));
        }
        let bare = host.strip_prefix("http://").unwrap_or(host);
        let bare = bare.split('/').next().unwrap_or_default();
        // RED: every host form reads as the default.
        let addr = match bare.get(..0).unwrap_or_default() {
            "" => format!("127.0.0.1:{PORT}"),
            b if b.rsplit_once(':').is_some_and(|(_, p)| !p.contains(']')) => {
                b.to_owned()
            }
            b => format!("{b}:{PORT}"),
        };
        Ok(Self { addr })
    }
}

impl Embed for Ollama {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f64>>, Error> {
        let body = json!({ "model": MODEL, "input": texts }).to_string();
        vectors(&post(&self.addr, "/api/embed", &body)?)
    }
}

fn fail(reason: impl Into<String>) -> Error {
    Error::Embed {
        reason: reason.into(),
    }
}

fn io(e: &std::io::Error) -> Error {
    fail(e.to_string())
}

/// POST `body` as JSON to `path`; the reply's body.
fn post(addr: &str, path: &str, body: &str) -> Result<String, Error> {
    let to = addr.to_socket_addrs().map_err(|e| io(&e))?.next();
    let to = to.ok_or_else(|| fail(format!("{addr} resolves to nothing")))?;
    let mut tcp =
        TcpStream::connect_timeout(&to, CONNECT).map_err(|e| io(&e))?;
    tcp.set_read_timeout(Some(REPLY)).map_err(|e| io(&e))?;
    let head = format!(
        "POST {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Type: \
         application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    tcp.write_all(format!("{head}{body}").as_bytes())
        .map_err(|e| io(&e))?;
    let mut raw = Vec::new();
    tcp.read_to_end(&mut raw).map_err(|e| io(&e))?;
    reply(&String::from_utf8_lossy(&raw))
}

/// The body of a raw HTTP/1.1 reply; any status but 200 fails.
pub(super) fn reply(raw: &str) -> Result<String, Error> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| fail("a reply without headers"))?;
    let status = head.lines().next().unwrap_or_default();
    if status.split_whitespace().nth(1) != Some("200") {
        return Err(fail(format!("ollama answered {status}")));
    }
    let lower = head.to_ascii_lowercase();
    if lower.contains("transfer-encoding: chunked") {
        return dechunk(body);
    }
    Ok(body.to_owned())
}

/// A chunked body joined.
fn dechunk(mut rest: &str) -> Result<String, Error> {
    let bad = || fail("a broken chunked reply");
    let mut out = String::new();
    loop {
        let (size, tail) = rest.split_once("\r\n").ok_or_else(bad)?;
        let size = size.split(';').next().unwrap_or_default().trim();
        let n = usize::from_str_radix(size, 16).map_err(|_| bad())?;
        if n == 0 {
            return Ok(out);
        }
        out.push_str(tail.get(..n).ok_or_else(bad)?);
        let after = tail.get(n..).and_then(|t| t.strip_prefix("\r\n"));
        rest = after.ok_or_else(bad)?;
    }
}

/// `{"embeddings": [[f64, ...], ...]}` as vectors.
pub(super) fn vectors(body: &str) -> Result<Vec<Vec<f64>>, Error> {
    let value: Value =
        serde_json::from_str(body).map_err(|e| fail(e.to_string()))?;
    let all = value.get("embeddings").and_then(Value::as_array);
    let all = all.ok_or_else(|| fail("a reply without embeddings"))?;
    all.iter().map(vector).collect()
}

fn vector(v: &Value) -> Result<Vec<f64>, Error> {
    let items = v
        .as_array()
        .ok_or_else(|| fail("an embedding not a list"))?;
    let each = items
        .iter()
        .map(|x| x.as_f64().ok_or_else(|| fail("not a number")));
    each.collect()
}
