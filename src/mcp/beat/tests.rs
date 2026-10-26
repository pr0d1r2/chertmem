//! The heartbeat files a running server keeps and what `pulse` reads
//! from them (src/mcp T4).

use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::{Beat, Pulse, pulse};
use crate::mcp::tests::{Boxed, Lines, Scratch, Test, env, req, session_with};
use crate::mcp::{Ctx, serve};

/// A scratch data dir, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(name: &str) -> Boxed<Self> {
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!("chert-beat-{pid}-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        Ok(Self(dir))
    }

    fn file(&self, pid: u32, ext: &str) -> PathBuf {
        self.0.join("mcp").join(format!("{P}.{pid}.{ext}"))
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const P: &str = "proj-00000000000000aa";

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn live(us: u64) -> Pulse {
    Pulse::Live {
        median_us: Some(us),
    }
}

#[test]
fn no_mcp_dir_is_dead() -> Test {
    let d = Dir::new("none")?;
    assert_eq!(pulse(&d.0, P), Pulse::Dead);
    assert_eq!(pulse(&d.0.join("absent"), P), Pulse::Dead);
    Ok(())
}

#[test]
fn a_started_server_is_live_before_its_first_reply() -> Test {
    let d = Dir::new("start")?;
    let _beat = Beat::start(&d.0, P, 41)?;
    assert!(d.file(41, "lock").is_file());
    assert_eq!(pulse(&d.0, P), Pulse::Live { median_us: None });
    Ok(())
}

#[test]
fn each_reply_writes_the_median_of_the_last_20() -> Test {
    let d = Dir::new("median")?;
    let mut beat = Beat::start(&d.0, P, 42)?;
    beat.record(ms(1), 1000)?;
    assert_eq!(pulse(&d.0, P), live(1000));
    // 5 slow replies, then 20 at 1..=20 ms: the slow ones fell out.
    let slow = std::iter::repeat_n((900, 2000), 5);
    for (took, at) in slow.chain((1..=20).map(|n| (n, 3000))) {
        beat.record(ms(took), at)?;
    }
    // The upper median of 1..=20 ms: the 11th, 11 ms.
    assert_eq!(pulse(&d.0, P), live(11_000));
    let text = fs::read_to_string(d.file(42, "json"))?;
    let got: Value = serde_json::from_str(&text)?;
    assert_eq!(got.get("at"), Some(&json!(3000)));
    assert_eq!(got.get("replies"), Some(&json!(20)));
    Ok(())
}

#[test]
fn a_clean_exit_removes_both_files_and_reads_dead() -> Test {
    let d = Dir::new("exit")?;
    {
        let mut beat = Beat::start(&d.0, P, 43)?;
        beat.record(ms(2), 1000)?;
    }
    assert!(!d.file(43, "lock").exists());
    assert!(!d.file(43, "json").exists());
    assert_eq!(pulse(&d.0, P), Pulse::Dead);
    Ok(())
}

#[test]
fn files_left_by_a_killed_server_read_dead() -> Test {
    let d = Dir::new("stale")?;
    fs::create_dir_all(d.0.join("mcp"))?;
    fs::write(d.file(44, "lock"), "")?;
    let beat = r#"{"pid":44,"median_us":5,"replies":1,"at":9}"#;
    fs::write(d.file(44, "json"), beat)?;
    assert_eq!(pulse(&d.0, P), Pulse::Dead);
    Ok(())
}

#[test]
fn the_freshest_of_2_live_servers_wins() -> Test {
    let d = Dir::new("fresh")?;
    let mut old = Beat::start(&d.0, P, 45)?;
    let mut new = Beat::start(&d.0, P, 46)?;
    new.record(ms(7), 5000)?;
    old.record(ms(3), 4000)?;
    assert_eq!(pulse(&d.0, P), live(7000));
    Ok(())
}

#[test]
fn another_projects_server_is_not_this_ones() -> Test {
    let d = Dir::new("other")?;
    let mut beat = Beat::start(&d.0, "other-00000000000000bb", 47)?;
    beat.record(ms(1), 1000)?;
    assert_eq!(pulse(&d.0, P), Pulse::Dead);
    Ok(())
}

#[test]
fn a_garbled_beat_of_a_live_server_has_no_median() -> Test {
    let d = Dir::new("garbled")?;
    let _beat = Beat::start(&d.0, P, 48)?;
    fs::write(d.file(48, "json"), "{oops")?;
    assert_eq!(pulse(&d.0, P), Pulse::Live { median_us: None });
    Ok(())
}

#[test]
fn a_second_server_of_the_same_pid_cannot_take_the_lock() -> Test {
    let d = Dir::new("twice")?;
    let _held = Beat::start(&d.0, P, 49)?;
    assert!(Beat::start(&d.0, P, 49).is_err());
    Ok(())
}

#[test]
fn a_data_dir_that_is_a_file_cannot_start() -> Test {
    let d = Dir::new("file")?;
    fs::write(d.0.join("mcp"), "")?;
    assert!(Beat::start(&d.0, P, 50).is_err());
    Ok(())
}

/// The write after each reply stays far under the 2 ms `get_observations`
/// budget, so a client sending its next frame at once barely waits.
/// Ignored like every timed test: a debug build on a shared CI runner
/// missed it (PR #2, ubuntu-latest).
#[test]
#[ignore = "timing: run in release"]
fn the_write_per_reply_costs_well_under_a_millisecond() -> Test {
    let d = Dir::new("cost")?;
    let mut beat = Beat::start(&d.0, P, 51)?;
    let start = Instant::now();
    for n in 0..200 {
        beat.record(ms(1), n)?;
    }
    let each = start.elapsed() / 200;
    assert!(each < Duration::from_millis(1), "{each:?} per write");
    Ok(())
}

/// Stdin of `lines`, 1 per read; before each line past the 1st, the
/// pulse a statusline would read then.
struct Probe<'a> {
    lines: VecDeque<Value>,
    first: bool,
    data: PathBuf,
    project: String,
    seen: &'a RefCell<Vec<Pulse>>,
}

impl Read for Probe<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let Some(line) = self.lines.pop_front() else {
            return Ok(0);
        };
        if !std::mem::take(&mut self.first) {
            self.seen
                .borrow_mut()
                .push(pulse(&self.data, &self.project));
        }
        (&mut *buf).write(format!("{line}\n").as_bytes())
    }
}

/// What a statusline read between the replies of a session of `n`
/// pings, and the data dir and project id it read.
fn probed(s: &Scratch, n: u64) -> Boxed<(Vec<Pulse>, PathBuf, String)> {
    let data = s.cfg.storage.data_dir.clone().ok_or("no data dir")?;
    let project = crate::store::project_id(&s.root());
    let seen = RefCell::new(Vec::new());
    let probe = Probe {
        lines: (1..=n).map(|n| req(n, "ping", &Value::Null)).collect(),
        first: true,
        data: data.clone(),
        project: project.clone(),
        seen: &seen,
    };
    serve_from(s, BufReader::new(probe))?;
    Ok((seen.into_inner(), data, project))
}

/// 1 session of `s` reading `input`.
fn serve_from(s: &Scratch, input: impl BufRead) -> Test {
    let root = s.root();
    let ctx = Ctx {
        config: &s.cfg,
        root: &root,
        env: env(&Lines::default()),
    };
    Ok(serve(&ctx, input, &mut Vec::new())?)
}

#[test]
fn a_session_beats_after_each_reply_and_clears_on_eof() -> Test {
    let s = Scratch::new("beat-session")?;
    let (seen, data, project) = probed(&s, 3)?;
    assert_eq!(seen.len(), 2, "{seen:?}");
    let timed = |p: &Pulse| matches!(p, Pulse::Live { median_us: Some(_) });
    assert!(seen.iter().all(timed), "{seen:?}");
    assert_eq!(pulse(&data, &project), Pulse::Dead);
    Ok(())
}

#[test]
fn a_server_without_a_heartbeat_still_serves_and_logs_why() -> Test {
    let s = Scratch::new("beat-off")?;
    let data = s.cfg.storage.data_dir.clone().ok_or("no data dir")?;
    fs::create_dir_all(&data)?;
    fs::write(data.join("mcp"), "")?;
    let lines = Lines::default();
    let got = session_with(&s, env(&lines), &[req(1, "ping", &Value::Null)])?;
    assert_eq!(got.len(), 1, "{got:?}");
    let all = lines.lock().map_err(|_| "poisoned")?.clone();
    assert!(all.iter().any(|l| l.contains("heartbeat off")), "{all:?}");
    Ok(())
}
