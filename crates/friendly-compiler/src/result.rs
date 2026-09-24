//! Structured compilation results.

use crate::error::CompileError;

/// The outcome of compiling one line of input.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CompileResult {
    /// Input matched a friendly command form and was rewritten to Sage source.
    Success {
        /// Sage source to display and eval.
        generated_sage: String,
        /// Free variables required in the session.
        required_variables: Vec<String>,
    },
    /// Input did not match any friendly form — passed through untouched as raw Sage.
    Bypass {
        /// Raw Sage string passed through.
        raw_sage: String,
    },
    /// Input matched a friendly form but was malformed.
    Error(CompileError),
    /// Input is ambiguous between multiple candidate interpretations.
    Ambiguous {
        /// Candidate interpretations.
        candidates: Vec<String>,
    },
}

impl CompileResult {
    /// Convenience for tests/tooling: returns the generated Sage if Success or Bypass, else None.
    pub fn sage(&self) -> Option<&str> {
        match self {
            Self::Success { generated_sage, .. } => Some(generated_sage),
            Self::Bypass { raw_sage } => Some(raw_sage),
            Self::Error(_) | Self::Ambiguous { .. } => None,
        }
    }
}
