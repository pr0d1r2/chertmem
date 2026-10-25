//! A stand-in ollama for the semantic re-rank suite: `/api/embed` on a
//! loopback port, answering bag-of-words vectors at once, so the suite
//! measures chert's side of the re-rank, never a model (root V6: no
//! egress). `Engine` reads `OLLAMA_HOST` only from the environment, so
//! the bench runs again as a child with it set to this server.

#[cfg(test)]
mod tests;

use std::env;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::Command;
use std::thread;

use serde_json::{Value, json};

use crate::fixture::Res;

/// The width of `nomic-embed-text`'s vectors.
pub(crate) const DIM: usize = 768;

/// Whether this process has no server to embed with yet.
pub(crate) fn needed() -> bool {
    env::var_os("OLLAMA_HOST").is_none()
}

/// This bench again, with the same arguments.
pub(crate) fn again() -> Res<Command> {
    let mut cmd = Command::new(env::current_exe()?);
    cmd.args(env::args_os().skip(1));
    Ok(cmd)
}

/// Run `cmd` with `OLLAMA_HOST` naming a fresh server; its exit code.
pub(crate) fn serve_for(mut cmd: Command) -> Res<i32> {
    let addr = start()?;
    let status = cmd.env("OLLAMA_HOST", addr.to_string()).status()?;
    Ok(status.code().unwrap_or(1))
}

/// A server on a free loopback port, answering until the process ends.
pub(crate) fn start() -> Res<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            drop(answer(stream));
        }
    });
    Ok(addr)
}

fn answer(mut stream: TcpStream) -> io::Result<()> {
    let body = request(&mut stream)?;
    let reply = embeddings(&body);
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        reply.len()
    );
    stream.write_all(format!("{head}{reply}").as_bytes())
}

/// The body of 1 HTTP/1.1 request, sized by its `Content-Length`.
fn request(stream: &mut TcpStream) -> io::Result<String> {
    let mut reader = BufReader::new(stream);
    let mut length = 0;
    let mut line = String::new();
    while reader.read_line(&mut line)? > 2 {
        let lower = line.to_ascii_lowercase();
        if let Some(n) = lower.strip_prefix("content-length:") {
            length = n.trim().parse().unwrap_or(0);
        }
        line.clear();
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body)?;
    Ok(String::from_utf8_lossy(&body).into_owned())
}

/// The reply to an `/api/embed` body: 1 vector per `input` text.
pub(crate) fn embeddings(body: &str) -> String {
    let asked: Value = serde_json::from_str(body).unwrap_or_default();
    let texts = asked.get("input").and_then(Value::as_array);
    let all = texts.map_or_else(Vec::new, |t| {
        t.iter()
            .map(|v| vector(v.as_str().unwrap_or_default()))
            .collect()
    });
    json!({"model": "nomic-embed-text", "embeddings": all}).to_string()
}

/// `text`'s words counted into [`DIM`] buckets by FNV-1a: texts that
/// share words point alike, as a model's vectors would.
pub(crate) fn vector(text: &str) -> Vec<f64> {
    let mut v = vec![0.0; DIM];
    for word in text.split_whitespace() {
        let hash = word.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
        });
        let slot = usize::try_from(hash).unwrap_or_default();
        if let Some(x) = slot.checked_rem(DIM).and_then(|i| v.get_mut(i)) {
            *x += 1.0;
        }
    }
    v
}
