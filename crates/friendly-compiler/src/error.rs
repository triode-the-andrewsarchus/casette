//! Error types for input compilation.

use std::fmt;

/// A structured, position-bearing parse error with a human suggestion.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CompileError {
    /// Human-readable description of what went wrong.
    pub message: String,
    /// 0-based UTF-8 byte offset into the original input where the problem is, if known.
    pub position: Option<usize>,
    /// A concrete "did you mean" / "try this" hint.
    pub suggestion: Option<String>,
}

impl CompileError {
    /// Creates a new compile error.
    pub fn new(message: impl Into<String>, position: Option<usize>, suggestion: Option<String>) -> Self {
        Self {
            message: message.into(),
            position,
            suggestion,
        }
    }
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CompileError {}
