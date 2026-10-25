use super::Statusline;
use crate::config::Error as ConfigError;
use crate::config::tests::{Scratch, Test, project_error, project_only};

#[test]
fn statusline_defaults_are_10_50_ms_1_5_gib_and_80_percent() -> Test {
    let got = Scratch::new("statusdefault")?.load()?.config.statusline;
    let want = Statusline {
        ms_green: 10,
        ms_amber: 50,
        size_green: 1 << 30,
        size_amber: 5 << 30,
        fit_amber: 80,
    };
    assert_eq!(got, want);
    Ok(())
}

#[test]
fn statusline_keys_parse() -> Test {
    let text = "[statusline]\nms_green = 5\nms_amber = 20\n\
                size_green = \"512MiB\"\nsize_amber = \"2GiB\"\n\
                fit_amber = 90\n";
    let got = project_only("statuskeys", text)?.statusline;
    let want = Statusline {
        ms_green: 5,
        ms_amber: 20,
        size_green: 512 << 20,
        size_amber: 2 << 30,
        fit_amber: 90,
    };
    assert_eq!(got, want);
    Ok(())
}

#[test]
fn bad_statusline_values_are_errors() -> Test {
    let cases = [
        "statusline.ms_green = 0",
        "statusline.ms_amber = \"50\"",
        "statusline.size_green = 1024",
        "statusline.size_amber = \"5GB\"",
        "statusline.fit_amber = 0",
        "statusline.fit_amber = 101",
    ];
    for (i, line) in cases.iter().enumerate() {
        let e = project_error(&format!("badstatus{i}"), &format!("{line}\n"))?;
        assert!(matches!(e, ConfigError::Invalid { .. }), "{line}: {e}");
    }
    Ok(())
}

#[test]
fn a_fit_of_100_percent_is_accepted() -> Test {
    let got = project_only("statusfit", "statusline.fit_amber = 100\n")?;
    assert_eq!(got.statusline.fit_amber, 100);
    Ok(())
}
