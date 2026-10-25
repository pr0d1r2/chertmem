//! Why `chert.toml` could not be loaded (src/config V3).

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

/// Why `chert.toml` could not be loaded or set; each names the file
/// (src/config V3, root C `chert: <what> (<fix>)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file exists but could not be read.
    Read { file: PathBuf, reason: String },
    /// The file could not be written (`chert config set`, T6).
    Write { file: PathBuf, reason: String },
    /// The file is not valid TOML.
    Parse { file: PathBuf, reason: String },
    /// A key outside src/config I.
    UnknownKey { file: PathBuf, key: String },
    /// A known key with a value outside its accepted forms.
    Invalid {
        file: PathBuf,
        key: String,
        value: String,
        accepted: String,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "chert: {} ({})", self.what(), self.fix())
    }
}

impl Error {
    /// The file the error is about.
    #[must_use]
    pub fn file(&self) -> &Path {
        match self {
            Self::Read { file, .. }
            | Self::Write { file, .. }
            | Self::Parse { file, .. }
            | Self::UnknownKey { file, .. }
            | Self::Invalid { file, .. } => file,
        }
    }

    fn what(&self) -> String {
        let file = self.file().display();
        match self {
            Self::Read { reason, .. } | Self::Write { reason, .. } => {
                self.io(reason)
            }
            Self::Parse { reason, .. } => {
                format!("{file} is not valid TOML: {reason}")
            }
            Self::UnknownKey { key, .. } => {
                format!("unknown key `{key}` in {file}")
            }
            Self::Invalid { key, value, .. } => {
                format!("`{key} = {value}` in {file} is invalid")
            }
        }
    }

    fn io(&self, reason: &str) -> String {
        let verb = match self {
            Self::Read { .. } => "read",
            _ => "write",
        };
        format!("cannot {verb} {}: {reason}", self.file().display())
    }

    fn fix(&self) -> String {
        match self {
            Self::Read { .. } => "check the file".to_owned(),
            Self::Write { .. } => "check the file and its directory".to_owned(),
            Self::Parse { .. } => "fix its syntax".to_owned(),
            Self::UnknownKey { .. } => "remove or respell it".to_owned(),
            Self::Invalid { accepted, .. } => format!("accepted: {accepted}"),
        }
    }
}

impl std::error::Error for Error {}
