//! Structured error kinds and stderr envelopes (D-01..D-04, FOUND-05).

use serde::Serialize;
use serde_json::{Map, Value};

/// Process exit code contract (FOUND-05 / D-03).
///
/// | Outcome | `Kind` | Exit code |
/// |---------|--------|-----------|
/// | Success | — | 0 |
/// | User input / usage | `UserError` | 1 |
/// | Template render / schema | `TemplateError` | 2 |
/// | File exists / overwrite denied | `FileConflict` | 3 |
/// | Network / download | `NetworkError` | 4 |
/// | Config parse / validation | `ConfigError` | 5 |
/// | Panic (normalized) | `InternalPanic` | 99 |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum Kind {
    UserError,
    TemplateError,
    FileConflict,
    NetworkError,
    ConfigError,
    InternalPanic,
}

impl Kind {
    /// Maps each variant 1:1 to its locked exit code (D-03).
    #[must_use]
    pub const fn exit_code(self) -> i32 {
        match self {
            Self::UserError => 1,
            Self::TemplateError => 2,
            Self::FileConflict => 3,
            Self::NetworkError => 4,
            Self::ConfigError => 5,
            Self::InternalPanic => 99,
        }
    }
}

/// Failure payload written to stderr (D-02).
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    pub kind: Kind,
    pub msg: String,
    pub exit_code: i32,
    pub hint: String,
    pub details: Map<String, Value>,
}

impl ErrorEnvelope {
    /// User input / CLI validation failure (D-09).
    #[must_use]
    pub fn user_error(
        msg: impl Into<String>,
        flag: Option<&str>,
        value: Option<&str>,
        hint: impl Into<String>,
    ) -> Self {
        let mut details = Map::new();
        if let Some(f) = flag {
            details.insert("flag".into(), Value::String(f.to_string()));
        }
        if let Some(v) = value {
            details.insert("value".into(), Value::String(v.to_string()));
        }
        Self {
            kind: Kind::UserError,
            msg: msg.into(),
            exit_code: Kind::UserError.exit_code(),
            hint: hint.into(),
            details,
        }
    }

    /// Builds an `InternalPanic` envelope from panic hook data (D-04).
    #[must_use]
    pub fn internal_panic(
        msg: impl Into<String>,
        location: Option<&str>,
        thread: Option<&str>,
    ) -> Self {
        let mut details = Map::new();
        if let Some(loc) = location {
            details.insert("location".into(), Value::String(loc.to_string()));
        }
        if let Some(t) = thread {
            details.insert("thread".into(), Value::String(t.to_string()));
        }
        Self {
            kind: Kind::InternalPanic,
            msg: msg.into(),
            exit_code: Kind::InternalPanic.exit_code(),
            hint: String::new(),
            details,
        }
    }
}
