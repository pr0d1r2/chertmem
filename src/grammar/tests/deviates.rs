//! The golden cases chert deliberately differs from (src/grammar V5).

/// The corpus cases where chert's output differs from the golden file
/// (src/grammar V5), as (corpus file, intensity, golden kind, chert's
/// exact bytes). Deviation (a), the pass skips code, and (b), inline
/// code after a fence, change no corpus case; (c), the blank before a
/// path kept, changes `urls_paths` at every intensity, in `compress` and
/// so in `expand` of it: `in ./config` and `is ../shared` keep a blank.
pub(super) const DEVIATES: [(&str, &str, &str, &str); 6] = [
    ("urls_paths", "lite", "compressed", URLS_LITE),
    ("urls_paths", "lite", "expanded", URLS_LITE_EXPANDED),
    ("urls_paths", "full", "compressed", URLS_FULL),
    ("urls_paths", "full", "expanded", URLS_FULL_EXPANDED),
    ("urls_paths", "ultra", "compressed", URLS_ULTRA),
    ("urls_paths", "ultra", "expanded", URLS_ULTRA_EXPANDED),
];

const URLS_LITE: &str = "\
See https://example.com/docs/the-guide?section=a for the docs, or
the mirror at http://mirror.example.org/repository/(main) instead.
The config lives in ./config/chert.toml and the binary in
/usr/local/bin/chert; the parent is ../shared/lib and the home \
copy is
~/.config/chert/settings.toml. On Windows it is C:\\Program\\chert.exe.
Work with the database and without the cache, w/o surprises.
";

const URLS_LITE_EXPANDED: &str = "\
See https://example.com/docs/the-guide?section=a for the documentation, \
or
the mirror at http://mirror.example.org/repository/(main) instead.
The configuration lives in ./config/chert.toml and the binary in
/usr/local/bin/chert; the parent is ../shared/lib and the home \
copy is
~/.config/chert/settings.toml. On Windows it is C:\\Program\\chert.exe.
Work with the database and without the cache, w/o surprises.
";

const URLS_FULL: &str = "\
See https://example.com/docs/the-guide?section=a for docs, or
mirror at http://mirror.example.org/repository/(main) instead.
config lives in ./config/chert.toml and binary in
/usr/local/bin/chert; parent is ../shared/lib and home copy is
~/.config/chert/settings.toml. On Windows it is C:\\Program\\chert.exe.
Work with db and without cache, w/o surprises.
";

const URLS_FULL_EXPANDED: &str = "\
See https://example.com/docs/the-guide?section=a for documentation, or
mirror at http://mirror.example.org/repository/(main) instead.
configuration lives in ./config/chert.toml and binary in
/usr/local/bin/chert; parent is ../shared/lib and home copy is
~/.config/chert/settings.toml. On Windows it is C:\\Program\\chert.exe.
Work with database and without cache, w/o surprises.
";

const URLS_ULTRA: &str = "\
See https://example.com/docs/the-guide?section=a for docs, or
mirror at http://mirror.example.org/repository/(main) instead.
cfg lives in ./config/chert.toml & binary in
/usr/local/bin/chert; parent is ../shared/lib & home copy is
~/.config/chert/settings.toml. On Windows it is C:\\Program\\chert.exe.
Work w/ db & w/o cache, w/o surprises.
";

const URLS_ULTRA_EXPANDED: &str = "\
See https://example.com/docs/the-guide?section=a for documentation, or
mirror at http://mirror.example.org/repository/(main) instead.
configuration lives in ./config/chert.toml & binary in
/usr/local/bin/chert; parent is ../shared/lib & home copy is
~/.config/chert/settings.toml. On Windows it is C:\\Program\\chert.exe.
Work w/ database & w/o cache, w/o surprises.
";
