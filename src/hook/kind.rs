//! The 5 hook events and the IDEs whose dialect `chert` reads (src/hook
//! I, C).

#[cfg(test)]
mod tests;

/// A hook event, named as `chert hook run <event>` names it (root I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    SessionStart,
    UserPromptSubmit,
    PostToolUse,
    Stop,
    SessionEnd,
}

impl Kind {
    /// Every event, in a session's order.
    pub const ALL: [Self; 5] = [
        Self::SessionStart,
        Self::UserPromptSubmit,
        Self::PostToolUse,
        Self::Stop,
        Self::SessionEnd,
    ];

    /// The CLI name, also the stored observation kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SessionStart => "session-start",
            Self::UserPromptSubmit => "user-prompt-submit",
            Self::PostToolUse => "post-tool-use",
            Self::Stop => "stop",
            Self::SessionEnd => "session-end",
        }
    }

    /// The event of CLI name `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.as_str() == text)
    }

    /// Claude Code's `hook_event_name`, which its output names again.
    pub(super) fn event_name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PostToolUse => "PostToolUse",
            Self::Stop => "Stop",
            Self::SessionEnd => "SessionEnd",
        }
    }
}

/// An IDE dialect (src/hook C): v1 reads Claude Code only (D24).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ide {
    ClaudeCode,
}

impl Ide {
    /// The `--ide` name, also the stored observation ide.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
        }
    }

    /// The IDE of `--ide` name `text`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        (text == Self::ClaudeCode.as_str()).then_some(Self::ClaudeCode)
    }
}
