// Only `#[test]` fns here, imports inside them: `cargo bench` builds
// this module with `--cfg test` and strips every test (lib.rs).

#[test]
fn a_vector_counts_each_word_in_its_bucket() {
    use super::{DIM, vector};
    let v = vector("cargo test cargo");
    assert_eq!(v.len(), DIM);
    assert!((v.iter().sum::<f64>() - 3.0).abs() < f64::EPSILON);
    assert!(v.iter().any(|&x| (x - 2.0).abs() < f64::EPSILON));
    assert_eq!(vector("cargo test cargo"), v);
    assert_ne!(vector("git push"), v);
}

#[test]
fn an_embed_body_gets_1_vector_per_input() -> crate::fixture::Res<()> {
    let reply = super::embeddings(r#"{"model":"m","input":["a b","c"]}"#);
    let value: serde_json::Value = serde_json::from_str(&reply)?;
    let all = value.get("embeddings").and_then(|e| e.as_array());
    assert_eq!(all.map(Vec::len), Some(2));
    let none = super::embeddings("not json");
    assert!(none.contains(r#""embeddings":[]"#), "{none}");
    Ok(())
}

#[test]
fn the_server_answers_an_http_embed_request() -> crate::fixture::Res<()> {
    use std::io::{Read, Write};
    let body = r#"{"model":"nomic-embed-text","input":["cache"]}"#;
    let mut tcp = std::net::TcpStream::connect(super::start()?)?;
    let head =
        format!("POST /api/embed HTTP/1.1\r\nContent-Length: {}", body.len());
    write!(tcp, "{head}\r\nHost: x\r\n\r\n{body}")?;
    let mut reply = String::new();
    tcp.read_to_string(&mut reply)?;
    assert!(reply.starts_with("HTTP/1.1 200 OK\r\n"), "{reply}");
    let (_, json) = reply.split_once("\r\n\r\n").ok_or("no body")?;
    let value: serde_json::Value = serde_json::from_str(json)?;
    let first = value.pointer("/embeddings/0").and_then(|v| v.as_array());
    assert_eq!(first.map(Vec::len), Some(super::DIM));
    Ok(())
}

#[test]
fn a_child_runs_with_ollama_host_naming_the_server() -> crate::fixture::Res<()>
{
    let sh = |script: &str| {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", script]);
        cmd
    };
    let named = r#"case "$OLLAMA_HOST" in 127.0.0.1:*) exit 0;; esac; exit 4"#;
    assert_eq!(super::serve_for(sh(named))?, 0);
    assert_eq!(super::serve_for(sh("exit 3"))?, 3);
    Ok(())
}

#[test]
fn the_bench_runs_again_as_itself() -> crate::fixture::Res<()> {
    let again = super::again()?;
    assert_eq!(again.get_program(), std::env::current_exe()?.as_os_str());
    let host = std::env::var_os("OLLAMA_HOST");
    assert_eq!(super::needed(), host.is_none());
    Ok(())
}
