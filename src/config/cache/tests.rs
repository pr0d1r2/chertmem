use super::{Overflow, Span, window_warning};
use crate::config::tests::{
    Scratch, Test, days, hours, months, project_only, put,
};

#[test]
fn cache_keys_parse() -> Test {
    let text = "cache.window = \"2mo\"\ncache.max_bytes = \"2GiB\"\n\
        cache.on_overflow = \"shrink\"\n";
    let c = project_only("cache", text)?;
    let k = c.cache;
    let cache = (months(2), 2 << 30, Overflow::Shrink);
    assert_eq!((k.window, k.max_bytes, k.on_overflow), cache);
    Ok(())
}

#[test]
fn window_off_the_bucket_warns_and_still_loads() -> Test {
    let text = "cache.window = \"30d\"\npartition.time_bucket = \"7d\"\n";
    let s = Scratch::new("offbucket")?;
    put(&s.project_file(), text)?;
    let loaded = s.load()?;
    assert_eq!(loaded.config.cache.window, days(30));
    let [warning] = loaded.warnings.as_slice() else {
        return Err(format!("{:?}", loaded.warnings).into());
    };
    assert!(warning.contains("cache.window = \"30d\""), "{warning}");
    assert!(warning.contains("time_bucket = \"7d\""), "{warning}");
    Ok(())
}

const WHOLE: &[(Span, Span)] = &[
    (days(28), days(7)),
    (days(7), days(7)),
    (months(2), months(1)),
    (hours(48), days(1)),
    (days(1), hours(12)),
];

#[test]
fn whole_multiples_do_not_warn() {
    for &(window, bucket) in WHOLE {
        let warning = window_warning(window, bucket);
        assert_eq!(warning, None, "{window} / {bucket}");
    }
}

const OFF: &[(Span, Span)] = &[
    (days(30), days(7)),
    (days(3), days(7)),
    (months(1), days(7)),
    (days(28), months(1)),
    (hours(36), days(1)),
    (months(3), months(2)),
];

#[test]
fn other_windows_warn_naming_both_values() {
    for &(window, bucket) in OFF {
        let warning = window_warning(window, bucket).unwrap_or_default();
        assert!(warning.contains(&format!("\"{window}\"")), "{warning}");
        assert!(warning.contains(&format!("\"{bucket}\"")), "{warning}");
    }
}
