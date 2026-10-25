//! The fit alone, then the overflow policies and the memory check
//! through a handle (V4).

use std::error::Error as StdError;
use std::mem::size_of;

use super::{
    DAY, Fit, INDEX_ROW, Plan, ROW_OVERHEAD, days, estimate, fit, memory,
    overflow, window_ms,
};
use crate::cache::tests::{
    HOUR, NOW, START, Scratch, Test, env, event, from, logged, recorded, same,
    seed, wait_warm,
};
use crate::cache::{Cache, Env, Source};
use crate::config::{self, Config, Overflow, Span};
use crate::store::{Mode, Observation, Partition, State, Stats};

fn span(text: &str) -> Result<Span, String> {
    Span::parse(text).ok_or_else(|| format!("bad span {text}"))
}

fn stats(rows: u64, bytes: u64) -> Stats {
    Stats {
        rows,
        bytes,
        checksum: String::new(),
    }
}

fn part(t0: u64, t1: u64, rows: u64, bytes: u64) -> Partition {
    Partition {
        path: format!("chunks/p/{t0}.db"),
        dims: "project,time".into(),
        project: "p".into(),
        ide: String::new(),
        t0,
        t1,
        shard: None,
        stats: stats(rows, bytes),
        state: State::Hot,
        spec: String::new(),
    }
}

/// 28 rows of 1000 bytes spread over the 28 days before [`NOW`]: 1
/// day costs `1000 + ROW`.
fn month() -> Vec<Partition> {
    vec![part(START, NOW, 28, 28_000)]
}

/// What a row costs beyond its body, its index share included.
const ROW: u64 = ROW_OVERHEAD + INDEX_ROW;

const PER_DAY: u64 = 1000 + ROW;

fn cache(max_bytes: u64, on_overflow: Overflow) -> config::Cache {
    config::Cache {
        window: Span::parse("28d").unwrap_or(Span {
            amount: 28,
            unit: config::Unit::Days,
        }),
        max_bytes,
        on_overflow,
    }
}

#[test]
fn the_window_length_follows_its_unit() -> Result<(), String> {
    assert_eq!(window_ms(span("12h")?), 12 * 3_600_000);
    assert_eq!(window_ms(span("28d")?), 28 * DAY);
    assert_eq!(window_ms(span("2mo")?), 62 * DAY);
    assert_eq!((days(span("12h")?), days(span("28d")?)), (1, 28));
    Ok(())
}

#[test]
fn a_row_costs_its_body_and_the_overhead() {
    let inline = size_of::<Observation>() + size_of::<(u64, String)>();
    // Plus 2 ULIDs on the heap, the least a row holds there.
    assert!(ROW_OVERHEAD >= u64::try_from(inline + 2 * 26).unwrap_or(0));
    let one = [part(0, DAY, 2, 100)];
    assert_eq!(estimate(&one, 0, DAY), 100 + 2 * ROW);
}

#[test]
fn a_partition_counts_its_share_from_since() {
    let ten = [part(0, 10 * DAY, 10, 0)];
    let whole = 10 * ROW;
    let at = |since| estimate(&ten, since, 10 * DAY);
    assert_eq!([at(0), at(5 * DAY), at(10 * DAY)], [whole, whole / 2, 0]);
    // The current bucket: its rows spread over the 4 days past.
    assert_eq!(estimate(&ten, 2 * DAY, 4 * DAY), whole / 2);
    // A bucket not begun yet counts whole.
    assert_eq!(estimate(&ten, 2 * DAY, 0), whole);
}

#[test]
fn a_window_that_fits_is_kept_whole() {
    let got = fit(&month(), &cache(256 << 20, Overflow::Warn), NOW);
    let plan = Plan {
        estimate: 28 * PER_DAY,
        index: 28 * INDEX_ROW,
        limit: 256 << 20,
        days_kept: 28,
    };
    let since = Some(START);
    assert_eq!(got, Fit { plan, since });
    assert!(!got.over());
}

#[test]
fn warn_and_shrink_keep_the_newest_days_that_fit() {
    for policy in [Overflow::Warn, Overflow::Shrink] {
        let got = fit(&month(), &cache(10 * PER_DAY, policy), NOW);
        assert_eq!(got.plan.days_kept, 10, "{policy:?}");
        assert_eq!(got.plan.estimate, 28 * PER_DAY);
        assert_eq!((got.since, got.over()), (Some(NOW - 10 * DAY), true));
    }
}

#[test]
fn fail_or_nothing_fitting_turns_the_cache_off() {
    let fail = fit(&month(), &cache(10 * PER_DAY, Overflow::Fail), NOW);
    let tiny = fit(&month(), &cache(1, Overflow::Warn), NOW);
    for got in [fail, tiny] {
        let off = (got.plan.days_kept, got.since, got.over());
        assert_eq!(off, (0, None, true));
    }
}

#[test]
fn the_overflow_warning_names_estimate_and_limit() {
    let cfg = cache(10 * PER_DAY, Overflow::Warn);
    let kept = overflow(&fit(&month(), &cfg, NOW).plan, &cfg);
    let (estimate, limit) = (28 * PER_DAY, 10 * PER_DAY);
    assert!(kept.starts_with("chert: cache estimate"), "{kept}");
    assert!(kept.contains(&format!("{estimate} bytes")), "{kept}");
    assert!(kept.contains(&format!("{limit} bytes")), "{kept}");
    assert!(kept.contains("newest 10 of 28 days"), "{kept}");
    let off =
        overflow(&fit(&month(), &cache(1, Overflow::Fail), NOW).plan, &cfg);
    assert!(off.contains("the cache is off"), "{off}");
}

#[test]
fn memory_warns_over_a_quarter_of_what_is_available() {
    let warned = memory(300, Some(1000)).unwrap_or_default();
    assert!(
        warned.contains("300 bytes is over 25% of the 1000"),
        "{warned}"
    );
    assert_eq!(memory(250, Some(1000)), None);
    assert_eq!(memory(u64::MAX, None), None);
}

#[test]
fn a_plan_prints_its_numbers_and_an_index_line() {
    let plan = Plan {
        estimate: 3,
        index: 1,
        limit: 2,
        days_kept: 1,
    };
    let text = "estimate 3 bytes, limit 2 bytes, 1 days kept\n\
                index 1 bytes of the estimate";
    assert_eq!(plan.to_string(), text);
}

#[test]
fn the_index_counts_toward_the_estimate_before_a_load() {
    // The high end of what search T9 measured with `Vec` slack (R6).
    const { assert!(INDEX_ROW >= 450) };
    let got = fit(&month(), &cache(256 << 20, Overflow::Warn), NOW);
    assert_eq!(got.plan.index, 28 * INDEX_ROW);
    let rows = estimate(&month(), START, NOW) - got.plan.index;
    assert_eq!(rows, 28 * (1000 + ROW_OVERHEAD));
}

#[test]
fn a_live_plan_swaps_in_the_index_bytes_when_over_only() {
    let cfg = cache(28 * PER_DAY, Overflow::Warn);
    let got = fit(&month(), &cfg, NOW);
    let rows = 28 * (1000 + ROW_OVERHEAD);
    assert_eq!(got.live(28 * INDEX_ROW, &cfg), None);
    let live = got.live(28 * INDEX_ROW + 1, &cfg).ok_or("fits");
    let want = (rows + 28 * INDEX_ROW + 1, 28 * INDEX_ROW + 1, 28);
    let live = live.map(|p| (p.estimate, p.index, p.days_kept));
    assert_eq!(live, Ok(want));
    let fail = cache(28 * PER_DAY, Overflow::Fail);
    let off = got.live(u64::MAX, &fail).map(|p| p.days_kept);
    assert_eq!(off, Some(0));
    // Over before the load: reported then, not again.
    let over = fit(&month(), &cache(1, Overflow::Warn), NOW);
    assert_eq!(over.live(u64::MAX, &cfg), None);
}

/// A scratch store with 20 days of 2000-byte rows, and a config whose
/// limit holds only some of them.
fn crowded(name: &str) -> Result<(Scratch, Config), Box<dyn StdError>> {
    let s = Scratch::new(name)?;
    let mut hook = s.open(Mode::Write)?;
    for day in 0..20_u64 {
        let ts = (NOW - HOUR).saturating_sub(day.saturating_mul(DAY));
        hook.append(&event(ts, &"x".repeat(2000)))?;
    }
    let mut cfg = s.config()?;
    cfg.cache.max_bytes = 20_000;
    Ok((s, cfg))
}

#[test]
fn warn_logs_on_every_load_and_answers_stay_equal() -> Test {
    let (s, cfg) = crowded("warn")?;
    let cold = s.cold(&cfg)?;
    cold.handle.warm()?;
    cold.handle.warm()?;
    let logs = logged(&cold.logs)?;
    assert_eq!(logs.len(), 2);
    assert!(logs.iter().all(|l| l.contains("20000 bytes")), "{logs:?}");
    let kept = cold.handle.plan()?.days_kept;
    assert!((1..28).contains(&kept), "{kept}");
    let mut truth = s.open(Mode::Read)?;
    let inside = from(&truth, NOW.saturating_sub(kept.saturating_mul(DAY)));
    assert_eq!(same(&cold.handle, &mut truth, &inside)?, Source::Cache);
    let whole = from(&truth, START);
    assert_eq!(same(&cold.handle, &mut truth, &whole)?, Source::Store);
    Ok(())
}

#[test]
fn shrink_logs_once() -> Test {
    let (s, mut cfg) = crowded("shrink")?;
    cfg.cache.on_overflow = Overflow::Shrink;
    let cold = s.cold(&cfg)?;
    cold.handle.warm()?;
    cold.handle.warm()?;
    assert_eq!(logged(&cold.logs)?.len(), 1);
    assert!(cold.handle.is_warm()?);
    Ok(())
}

#[test]
fn fail_keeps_every_query_on_the_disk() -> Test {
    let (s, mut cfg) = crowded("fail")?;
    cfg.cache.on_overflow = Overflow::Fail;
    let cold = s.cold(&cfg)?;
    cold.handle.warm()?;
    assert!(!cold.handle.is_warm()?);
    let mut truth = s.open(Mode::Read)?;
    let recent = from(&truth, NOW - HOUR);
    assert_eq!(same(&cold.handle, &mut truth, &recent)?, Source::Store);
    assert_eq!(cold.handle.plan()?.days_kept, 0);
    let logs = logged(&cold.logs)?;
    assert!(logs.len() == 1 && logs.iter().all(|l| l.contains("off")));
    Ok(())
}

/// 1 MiB: the default 256 MiB limit is over a quarter of it.
fn small() -> Option<u64> {
    1_u64.checked_shl(20)
}

#[test]
fn start_warns_when_max_bytes_is_over_a_quarter_of_memory() -> Test {
    let s = Scratch::new("memory")?;
    let (base, logs) = env();
    let env = Env {
        memory: small,
        ..base
    };
    let handle = Cache::start_with(s.open(Mode::Read)?, &s.config()?, env);
    wait_warm(&handle)?;
    let all = logged(&logs)?;
    assert_eq!(all.len(), 1);
    assert!(all.iter().all(|l| l.contains("over 25%")), "{all:?}");
    Ok(())
}

#[test]
fn plan_alone_equals_the_handle_plan_under_every_policy() -> Test {
    let (s, mut cfg) = crowded("plan-alone")?;
    for policy in [Overflow::Warn, Overflow::Shrink, Overflow::Fail] {
        cfg.cache.on_overflow = policy;
        let cold = s.cold(&cfg)?;
        let alone = Cache::plan_at(&s.open(Mode::Read)?, &cfg, NOW)?;
        assert!(alone.estimate > alone.limit, "{alone}");
        assert_eq!(alone, cold.handle.plan()?, "{policy:?}");
        assert!(logged(&cold.logs)?.is_empty(), "{policy:?}");
    }
    Ok(())
}

#[test]
fn plan_alone_runs_on_the_system_clock() -> Test {
    let (s, cfg) = crowded("plan-system")?;
    let plan = Cache::plan(&s.open(Mode::Read)?, &cfg)?;
    let got = (plan.estimate, plan.limit, plan.days_kept);
    assert_eq!(got, (0, 20_000, 28), "the rows are long past: {plan}");
    Ok(())
}

/// A live index of 1 MB a row: the 3 seeded rows of the window fit
/// `max_bytes` before the load, not once they are indexed.
const HEAVY: u64 = 1_000_000;

/// 2 loads of the seeded `s` under `policy` with a [`HEAVY`] index:
/// how many warnings, and whether the cache stays warm.
fn heavy(
    s: &Scratch,
    policy: Overflow,
) -> Result<(usize, bool), Box<dyn StdError>> {
    let mut cfg = s.config()?;
    (cfg.cache.max_bytes, cfg.cache.on_overflow) = (1_000_000, policy);
    let cold = recorded(s, &cfg, HEAVY)?;
    cold.handle.warm()?;
    cold.handle.warm()?;
    let (logs, warm) = (logged(&cold.logs)?, cold.handle.is_warm()?);
    let index = format!("(index {} bytes)", 3 * HEAVY);
    assert!(logs.iter().all(|l| l.contains(&index)), "{logs:?}");
    let off = logs.iter().all(|l| l.contains("the cache is off"));
    assert_eq!(off, !warm, "{logs:?}");
    // The plan before a load, as `cache plan` prints it: all days.
    assert_eq!(cold.handle.plan()?.days_kept, 28, "{policy:?}");
    let q = from(&s.open(Mode::Read)?, START);
    assert_eq!(cold.handle.with_index(&q, |_, _| ())?.is_some(), warm);
    Ok((logs.len(), warm))
}

#[test]
fn a_live_index_over_the_limit_warns_and_fail_turns_the_cache_off() -> Test {
    let s = Scratch::new("live")?;
    seed(&s)?;
    assert_eq!(heavy(&s, Overflow::Warn)?, (2, true));
    assert_eq!(heavy(&s, Overflow::Shrink)?, (1, true));
    assert_eq!(heavy(&s, Overflow::Fail)?, (2, false));
    Ok(())
}
