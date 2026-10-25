//! `chert hook run` passes `hook::run`'s exit through and stores the
//! event; `chert mcp` is exit 0 on a clean EOF (V2).

use clap::ValueEnum;

use super::super::tests::{NOW, SESSION, Sandbox, Test, fixture};
use super::Event;
use crate::hook::Kind;

#[test]
fn event_names_are_the_hook_kinds() {
    let names: Vec<Kind> =
        Event::value_variants().iter().map(|e| e.0).collect();
    assert_eq!(names, Kind::ALL);
}

#[test]
fn a_hook_stores_its_event_with_exit_0() -> Test {
    let s = Sandbox::new("hook-stores")?;
    s.hook("post-tool-use")?;
    let ran = s.run(&["timeline", SESSION]);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains(&format!("post-tool-use {NOW}")), "{ran:?}");
    Ok(())
}

#[test]
fn every_event_runs_with_exit_0() -> Test {
    let s = Sandbox::new("hook-every")?;
    for kind in Kind::ALL {
        s.hook(kind.as_str())?;
    }
    Ok(())
}

/// A past session `past`, its turn and end stored: a summary to inject.
fn past(s: &Sandbox) {
    let stop = br#"{"session_id":"past","last_assistant_message":"Fixed it"}"#;
    let end = br#"{"session_id":"past","reason":"other"}"#;
    for (event, stdin) in
        [("stop", stop.as_slice()), ("session-end", end.as_slice())]
    {
        let ran = s.run_with(&["hook", "run", event], stdin);
        assert_eq!((ran.code, ran.err.as_str()), (0, ""), "{ran:?}");
    }
}

#[test]
fn session_start_prints_its_context_on_stdout() -> Test {
    let s = Sandbox::new("hook-inject")?;
    past(&s);
    let ran = s.run_with(
        &["hook", "run", "session-start", "--ide", "claude-code"],
        &fixture("session-start")?,
    );
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.out.contains("SessionStart"), "{ran:?}");
    Ok(())
}

#[test]
fn an_unreadable_payload_is_noted_with_exit_0() -> Test {
    let s = Sandbox::new("hook-unreadable")?;
    let ran = s.run_with(&["hook", "run", "stop"], b"not json");
    assert_eq!((ran.code, ran.out.as_str()), (0, ""), "{ran:?}");
    assert!(ran.err.contains("unreadable hook payload"), "{ran:?}");
    Ok(())
}

#[test]
fn a_bad_config_is_noted_with_exit_0_not_blocking_the_agent() -> Test {
    let s = Sandbox::new("hook-bad-config")?;
    s.project_toml("[nope]\nkey = 1\n")?;
    let stdin = fixture("post-tool-use")?;
    let ran = s.run_with(&["hook", "run", "post-tool-use"], &stdin);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert!(ran.err.contains("event not stored"), "{ran:?}");
    Ok(())
}

#[test]
fn an_unknown_event_or_ide_is_exit_2() -> Test {
    let s = Sandbox::new("hook-unknown")?;
    assert_eq!(s.run(&["hook", "run", "pre-tool-use"]).code, 2);
    let ran = s.run(&["hook", "run", "stop", "--ide", "vim"]);
    assert_eq!(ran.code, 2, "{ran:?}");
    Ok(())
}

#[test]
fn mcp_on_a_clean_eof_is_exit_0() -> Test {
    let s = Sandbox::new("mcp-eof")?;
    let ran = s.run(&["mcp"]);
    assert_eq!((ran.code, ran.out.as_str()), (0, ""), "{ran:?}");
    Ok(())
}

#[test]
fn mcp_answers_1_frame_per_line() -> Test {
    let s = Sandbox::new("mcp-frame")?;
    let ping = br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let ran = s.run_with(&["mcp"], ping);
    assert_eq!(ran.code, 0, "{ran:?}");
    assert_eq!(ran.out.lines().count(), 1, "{ran:?}");
    assert!(ran.out.contains(r#""id":1"#), "{ran:?}");
    Ok(())
}

#[test]
fn mcp_with_a_bad_config_is_exit_2() -> Test {
    let s = Sandbox::new("mcp-bad-config")?;
    s.project_toml("[cache]\nwindow = 7\n")?;
    assert_eq!(s.run(&["mcp"]).code, 2);
    Ok(())
}
