use super::{DAY, window_ms};
use crate::config::Span;

fn span(text: &str) -> Result<Span, String> {
    Span::parse(text).ok_or_else(|| format!("bad span {text}"))
}

#[test]
fn the_window_length_follows_its_unit() -> Result<(), String> {
    assert_eq!(window_ms(span("12h")?), 12 * 3_600_000);
    assert_eq!(window_ms(span("28d")?), 28 * DAY);
    assert_eq!(window_ms(span("2mo")?), 62 * DAY);
    Ok(())
}
