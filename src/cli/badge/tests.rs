//! `chert statusline`: the badge of what is stored and running, exit 0
//! whatever is missing (T3, V2).

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use super::super::Tty;
use super::super::tests::{Feed, NOW, Ran, Res, Sandbox, Test, exec};
use crate::cache::Env;
use crate::config::{Config, load_from};
use crate::mcp;
use crate::store::{Event, Mode, Store};

const DOWN: &str = "\x1b[38;5;196mchert\x1b[0m";
const LIVE: &str = "\x1b[38;5;172mchert\x1b[0m";
const NEVER: [&str; 3] = ["statusline", "--color", "never"];
const ALWAYS: [&str; 3] = ["statusline", "--color", "always"];
const AUTO: [&str; 1] = ["statusline"];

/// Empty stdin, no terminal, in `cwd`, the sandbox home injected.
fn at(s: &Sandbox, cwd: PathBuf) -> Feed {
    Feed {
        stdin: Vec::new(),
        cwd,
        home: Some(s.home()),
        tty: Tty::default(),
    }
}

/// `payload` on stdin, run outside the project.
fn fed(s: &Sandbox, payload: &serde_json::Value) -> Feed {
    Feed {
        stdin: payload.to_string().into_bytes(),
        ..at(s, s.file("elsewhere"))
    }
}

/// A statusline payload naming `cwd`.
fn payload(cwd: &Path) -> serde_json::Value {
    serde_json::json!({"cwd": cwd, "session_id": "s"})
}

/// The badge of the sandbox project, run in it.
fn here(s: &Sandbox, args: &[&str]) -> Ran {
    exec(args, &at(s, s.project()))
}

/// `out` as `[label:ms, n, size]`, exit 0 asserted.
fn cells(ran: &Ran) -> Vec<String> {
    assert_eq!((ran.code, ran.err.as_str()), (0, ""), "{ran:?}");
    let line = ran.out.strip_suffix('\n').unwrap_or(&ran.out);
    line.split(',').map(str::to_owned).collect()
}

/// The count cell of `ran`.
fn count(ran: &Ran) -> Option<String> {
    cells(ran).get(1).cloned()
}

#[test]
fn no_data_dir_is_a_down_label_and_zeros() -> Test {
    let s = Sandbox::new("badge-homeless")?;
    let ran = s.run_homeless(&NEVER);
    assert_eq!((ran.code, ran.out.as_str()), (0, "chert:-ms,0,0B\n"));
    let ran = s.run_homeless(&ALWAYS);
    assert_eq!(ran.out, format!("{DOWN}:-ms,0,0B\n"), "{ran:?}");
    Ok(())
}

#[test]
fn no_catalog_yet_is_the_same_and_makes_no_data_dir() -> Test {
    let s = Sandbox::new("badge-empty")?;
    assert_eq!(here(&s, &NEVER).out, "chert:-ms,0,0B\n");
    assert!(!s.home().join(".local").exists());
    Ok(())
}

#[test]
fn stored_rows_give_the_count_and_size_in_green() -> Test {
    let s = Sandbox::new("badge-rows")?;
    s.hook("post-tool-use")?;
    s.hook("post-tool-use-failure")?;
    let got = cells(&here(&s, &NEVER));
    let size = got.get(2).ok_or("no size")?;
    assert_eq!(got.get(..2), Some(&["chert:-ms".into(), "2".into()][..]));
    assert!(size.ends_with('B') && size != "0B", "{got:?}");
    let colored = cells(&here(&s, &ALWAYS));
    assert_eq!(colored.get(1).map(String::as_str), Some("\x1b[32m2\x1b[0m"));
    let green = format!("\x1b[32m{size}\x1b[0m");
    assert_eq!(colored.get(2), Some(&green), "{colored:?}");
    Ok(())
}

#[test]
fn the_count_turns_amber_near_the_cache_limit_and_red_over_it() -> Test {
    let s = Sandbox::new("badge-fit")?;
    s.hook("post-tool-use")?;
    s.project_toml("[cache]\nmax_bytes = \"1KiB\"\n")?;
    let red = count(&here(&s, &ALWAYS));
    assert_eq!(red.as_deref(), Some("\x1b[31m1\x1b[0m"));
    let amber =
        "[cache]\nmax_bytes = \"100KiB\"\n[statusline]\nfit_amber = 1\n";
    s.project_toml(amber)?;
    let amber = count(&here(&s, &ALWAYS));
    assert_eq!(amber.as_deref(), Some("\x1b[33m1\x1b[0m"));
    Ok(())
}

#[test]
fn the_size_wears_its_band() -> Test {
    let s = Sandbox::new("badge-size")?;
    s.hook("post-tool-use")?;
    let size = cells(&here(&s, &NEVER)).get(2).cloned().ok_or("no size")?;
    let toml = "[statusline]\nsize_green = \"1KiB\"\nsize_amber = \"1KiB\"\n";
    s.project_toml(toml)?;
    let got = cells(&here(&s, &ALWAYS));
    let small = format!("\x1b[32m{size}\x1b[0m");
    assert_eq!(got.get(2), Some(&small), "{got:?}");
    Ok(())
}

#[test]
fn the_payload_names_the_project_over_the_working_dir() -> Test {
    let s = Sandbox::new("badge-payload")?;
    s.hook("post-tool-use")?;
    let ran = exec(&NEVER, &fed(&s, &payload(&s.project())));
    assert_eq!(count(&ran).as_deref(), Some("1"));
    let json = serde_json::json!({"cwd": s.file("elsewhere"),
        "workspace": {"project_dir": s.project()}});
    let ran = exec(&NEVER, &fed(&s, &json));
    assert_eq!(count(&ran).as_deref(), Some("1"));
    Ok(())
}

#[test]
fn auto_colors_a_payload_or_a_terminal_and_nothing_else() -> Test {
    let s = Sandbox::new("badge-auto")?;
    assert_eq!(here(&s, &AUTO).out, "chert:-ms,0,0B\n");
    let ran = exec(&AUTO, &fed(&s, &payload(&s.project())));
    assert!(ran.out.starts_with(DOWN), "{ran:?}");
    let tty = Tty {
        stdin: false,
        stdout: true,
    };
    let feed = Feed {
        tty,
        ..at(&s, s.project())
    };
    assert!(exec(&AUTO, &feed).out.starts_with(DOWN));
    Ok(())
}

#[test]
fn a_terminal_stdin_is_never_read() -> Test {
    let s = Sandbox::new("badge-tty")?;
    s.hook("post-tool-use")?;
    let tty = Tty {
        stdin: true,
        stdout: false,
    };
    let feed = Feed {
        tty,
        ..fed(&s, &payload(&s.project()))
    };
    assert_eq!(exec(&AUTO, &feed).out, "chert:-ms,0,0B\n");
    Ok(())
}

#[test]
fn bad_stdin_falls_back_to_the_working_dir() -> Test {
    let s = Sandbox::new("badge-badstdin")?;
    s.hook("post-tool-use")?;
    for stdin in [&b"{oops"[..], b"[1]", b"{\"cwd\": 7}", b"{\"cwd\": \"\"}"] {
        let feed = Feed {
            stdin: stdin.to_vec(),
            ..at(&s, s.project())
        };
        assert_eq!(count(&exec(&NEVER, &feed)).as_deref(), Some("1"));
    }
    Ok(())
}

#[test]
fn a_bad_config_still_prints_a_badge_with_exit_0() -> Test {
    let s = Sandbox::new("badge-badconfig")?;
    s.project_toml("[cache]\nwindow = 7\n")?;
    let ran = here(&s, &ALWAYS);
    assert_eq!((ran.code, ran.out), (0, format!("{DOWN}:-ms,0,0B\n")));
    Ok(())
}

/// Stdin of 1 ping, then held open until `done` says stop.
struct Held {
    ping: Option<String>,
    done: mpsc::Receiver<()>,
}

impl Read for Held {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(line) = self.ping.take() else {
            let _ = self.done.recv();
            return Ok(0);
        };
        (&mut *buf).write(line.as_bytes())
    }
}

/// A `chert mcp` of the project at `root` until `done`.
fn serve(cfg: &Config, root: &Path, done: mpsc::Receiver<()>) -> Test {
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let input = io::BufReader::new(Held {
        ping: Some(format!("{ping}\n")),
        done,
    });
    let env = Env {
        log: Arc::new(|_| {}),
        ..Env::system()
    };
    let ctx = mcp::Ctx {
        config: cfg,
        root,
        env,
    };
    Ok(mcp::serve(&ctx, input, io::sink())?)
}

/// The colored badge once a live server has beaten, waiting up to 10 s.
fn beaten(s: &Sandbox) -> Ran {
    let start = Instant::now();
    loop {
        let ran = here(s, &ALWAYS);
        let late = start.elapsed() > Duration::from_secs(10);
        if late || !ran.out.contains(":-ms") {
            return ran;
        }
        thread::sleep(Duration::from_millis(10));
    }
}

/// The badge while a server of the sandbox project runs on a thread.
fn while_live(s: &Sandbox) -> Res<Ran> {
    let cfg = load_from(Some(&s.home()), &s.project())?.config;
    let ((stop, done), root) = (mpsc::channel(), s.project());
    let server = thread::spawn(move || {
        serve(&cfg, &root, done).map_err(|e| e.to_string())
    });
    let ran = beaten(s);
    stop.send(())?;
    server.join().map_err(|_| "server panicked")??;
    Ok(ran)
}

#[test]
fn a_live_server_shows_its_median_under_an_orange_label() -> Test {
    let s = Sandbox::new("badge-live")?;
    let ran = while_live(&s)?;
    let ms = ran.out.strip_prefix(&format!("{LIVE}:\x1b[32m"));
    let ms = ms.and_then(|r| r.split_once("ms\x1b[0m,")).map(|(m, _)| m);
    assert!(ms.is_some_and(|m| m.parse::<u64>().is_ok()), "{ran:?}");
    assert!(here(&s, &ALWAYS).out.starts_with(&format!("{DOWN}:-ms,")));
    Ok(())
}

/// 52 weekly partitions, 1 row each, as the 100k-p52 bench fixture's
/// catalog has: the badge reads only the catalog, so rows cost nothing.
fn weekly(s: &Sandbox) -> Test {
    let cfg = load_from(Some(&s.home()), &s.project())?.config;
    let mut store = Store::open(&cfg, &s.project(), Mode::Read)?;
    for week in 0..52_u64 {
        store.append(&Event {
            ts: NOW.saturating_sub(week.saturating_mul(604_800_000)),
            session: "s".into(),
            kind: "post-tool-use".into(),
            ide: "claude-code".into(),
            identity: None,
            body: "fix the auth bug".into(),
        })?;
    }
    Ok(())
}

/// Root C's 5 ms, in process: `cargo nextest run --release
/// --run-ignored only -E 'test(statusline_draws)'`.
#[test]
#[ignore = "timing: run in release"]
fn statusline_draws_in_under_5_ms_on_52_partitions() -> Test {
    let s = Sandbox::new("badge-timed")?;
    weekly(&s)?;
    let time = |_| {
        let start = Instant::now();
        let _ = here(&s, &NEVER);
        start.elapsed()
    };
    let mut took: Vec<Duration> = (0..50).map(time).collect();
    took.sort_unstable();
    let median = took.get(25).copied().unwrap_or_default();
    eprintln!("statusline median {median:?} over 50 runs");
    assert!(median < Duration::from_millis(5), "{median:?}");
    Ok(())
}
