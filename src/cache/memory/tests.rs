use super::{available, meminfo, memsize};

#[test]
fn meminfo_gives_mem_available_in_bytes() {
    let text = "MemTotal:       16384 kB\nMemFree:  1 kB\n\
                MemAvailable:    2048 kB\nBuffers: 3 kB\n";
    assert_eq!(meminfo(text), Some(2048 * 1024));
}

#[test]
fn meminfo_without_a_readable_figure_is_unknown() {
    for text in ["MemTotal: 16384 kB\n", "MemAvailable: lots kB\n", ""] {
        assert_eq!(meminfo(text), None, "{text:?}");
    }
    assert_eq!(meminfo("MemAvailable: 2048\n"), None);
}

#[test]
fn memsize_gives_bytes() {
    assert_eq!(memsize("17179869184\n"), Some(17_179_869_184));
    assert_eq!(memsize("hw.memsize: 1\n"), None);
}

#[test]
fn the_host_figure_is_unknown_or_positive() {
    assert!(available().is_none_or(|bytes| bytes > 0));
}
