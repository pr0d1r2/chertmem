use super::{Grammar, Zstd};
use crate::config::Error as ConfigError;
use crate::config::tests::{
    Scratch, Test, ZSTD_DEFAULT, project_error, project_only, put,
};

#[test]
fn compression_and_search_keys_parse() -> Test {
    let text = "compression.grammar = \"ultra\"\n\
        compression.zstd = { level = 19, dictionary = false }\n\
        search.alpha = 0.25\nsearch.default_limit = 20\n";
    let c = project_only("search", text)?;
    let (z, s) = (c.compression, c.search);
    let zstd = Zstd {
        level: 19,
        dictionary: false,
    };
    assert_eq!((z.grammar, z.zstd), (Grammar::Ultra, zstd));
    assert_eq!((s.alpha, s.default_limit), (0.25, 20));
    Ok(())
}

const fn zstd(level: i32, dictionary: bool) -> Zstd {
    Zstd { level, dictionary }
}

const ZSTD_TABLES: &[(&str, Zstd)] = &[
    ("[compression.zstd]\nlevel = 7\n", zstd(7, true)),
    (
        "compression.zstd = { dictionary = false }\n",
        zstd(3, false),
    ),
    ("compression.zstd = {}\n", ZSTD_DEFAULT),
    ("compression.zstd = { level = 1 }\n", zstd(1, true)),
    ("compression.zstd = { level = 22 }\n", zstd(22, true)),
];

#[test]
fn zstd_table_keys_default_one_by_one() -> Test {
    for (i, &(text, want)) in ZSTD_TABLES.iter().enumerate() {
        let c = project_only(&format!("zstd{i}"), text)?;
        assert_eq!(c.compression.zstd, want, "{text:?}");
    }
    Ok(())
}

#[test]
fn zstd_table_merges_key_by_key() -> Test {
    let s = Scratch::new("zstdmerge")?;
    put(&s.user_file(), "compression.zstd = { level = 9 }\n")?;
    put(
        &s.project_file(),
        "[compression.zstd]\ndictionary = false\n",
    )?;
    assert_eq!(s.load()?.config.compression.zstd, zstd(9, false));
    Ok(())
}

#[test]
fn zstd_not_a_table_is_invalid_naming_the_table_form() -> Test {
    for (i, text) in ["true", "false", "3", "[1]"].iter().enumerate() {
        let line = format!("compression.zstd = {text}\n");
        let e = project_error(&format!("zstdshape{i}"), &line)?;
        let ConfigError::Invalid { key, accepted, .. } = &e else {
            return Err(format!("{line:?}: {e}").into());
        };
        assert_eq!(key, "compression.zstd", "{line:?}");
        assert!(accepted.contains("level"), "{e}");
        assert!(accepted.contains("dictionary"), "{e}");
    }
    Ok(())
}

#[test]
fn zstd_level_out_of_range_names_the_range() -> Test {
    let e = project_error("zstdrange", "compression.zstd = { level = 23 }\n")?;
    let ConfigError::Invalid { key, accepted, .. } = &e else {
        return Err(e.to_string().into());
    };
    assert_eq!(key, "compression.zstd.level");
    assert!(accepted.contains("1 to 22"), "{e}");
    Ok(())
}

#[test]
fn each_grammar_maps_to_its_intensity() {
    use crate::grammar::Intensity;
    let pairs = [
        (Grammar::Off, Intensity::Off),
        (Grammar::Lite, Intensity::Lite),
        (Grammar::Full, Intensity::Full),
        (Grammar::Ultra, Intensity::Ultra),
    ];
    for (g, i) in pairs {
        assert_eq!(g.intensity(), i);
    }
}
