use std::net::TcpListener;

use serde_json::Value;

use super::{MODEL, Ollama, reply, vectors};
use crate::search::blend::Embed;
use crate::search::tests::{Test, lengths, serve};

#[test]
fn ollama_host_forms_name_1_address() -> Test {
    let addr = |h: &str| Ollama::new(h).map(|o| o.addr);
    assert_eq!(addr("")?, "127.0.0.1:11434");
    assert_eq!(addr("http://box:8080/")?, "box:8080");
    assert_eq!(addr("box")?, "box:11434");
    assert_eq!(addr("[::1]")?, "[::1]:11434");
    assert_eq!(addr("[::1]:9")?, "[::1]:9");
    assert!(addr("https://box").is_err());
    Ok(())
}

#[test]
fn a_reply_gives_its_body_and_any_other_status_fails() -> Test {
    assert_eq!(reply("HTTP/1.1 200 OK\r\nA: b\r\n\r\n{}")?, "{}");
    let chunked = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
    assert_eq!(
        reply(&format!("{chunked}2\r\n{{}}\r\n1;x\r\n \r\n0\r\n\r\n"))?,
        "{} "
    );
    assert!(reply("HTTP/1.1 404 Not Found\r\n\r\nno").is_err());
    assert!(reply("no headers").is_err());
    Ok(())
}

#[test]
fn a_broken_chunked_reply_fails() {
    let chunked = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n";
    for bad in ["9\r\n{}\r\n", "zz\r\n", "2\r\n{}", "no line end"] {
        assert!(reply(&format!("{chunked}{bad}")).is_err(), "{bad}");
    }
}

#[test]
fn vectors_read_embeddings_and_refuse_anything_else() -> Test {
    assert_eq!(
        vectors(r#"{"embeddings": [[1, 2.5], []]}"#)?,
        [vec![1.0, 2.5], vec![]]
    );
    for bad in [
        "nope",
        "{}",
        r#"{"embeddings": [1]}"#,
        r#"{"embeddings": [["x"]]}"#,
    ] {
        assert!(vectors(bad).is_err(), "{bad}");
    }
    Ok(())
}

#[test]
fn embed_posts_the_model_and_texts_and_reads_vectors() -> Test {
    let (addr, server) = serve(lengths)?;
    let got = Ollama { addr }.embed(&["ab", "c"])?;
    assert_eq!(got, [vec![2.0, 1.0], vec![1.0, 1.0]]);
    let sent = server
        .join()
        .map_err(|_| "server panicked")?
        .ok_or("no request")?;
    assert_eq!(sent.get("model").and_then(Value::as_str), Some(MODEL));
    Ok(())
}

#[test]
fn a_server_not_listening_is_an_embed_error() -> Test {
    let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?;
    let gone = Ollama {
        addr: port.to_string(),
    };
    assert!(gone.embed(&["x"]).is_err());
    assert!(
        Ollama {
            addr: "no-such-host.invalid:1".into()
        }
        .embed(&["x"])
        .is_err()
    );
    Ok(())
}

/// Against a real ollama: skipped unless `CHERT_TEST_OLLAMA=1` opts in,
/// `OLLAMA_HOST` naming it and the model pulled.
#[test]
fn a_real_ollama_embeds_when_opted_in() -> Test {
    if std::env::var("CHERT_TEST_OLLAMA").as_deref() != Ok("1") {
        eprintln!("skipped: set CHERT_TEST_OLLAMA=1 to run against ollama");
        return Ok(());
    }
    let host = std::env::var("OLLAMA_HOST").unwrap_or_default();
    let got = Ollama::new(&host)?.embed(&["cargo build", "git push"])?;
    assert_eq!(got.len(), 2);
    assert!(got.iter().all(|v| !v.is_empty()));
    Ok(())
}
