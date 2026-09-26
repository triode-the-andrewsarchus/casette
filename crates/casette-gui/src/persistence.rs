//! Session persistence and export functions for Casette sessions.

use crate::model::{SessionState, TapeRow};
use anyhow::{bail, Context, Result};
use casette_kernel::WorkerResponse;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Supported session persistence schema version.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Top-level session persistence container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEnvelope {
    pub schema_version: u32,
    pub rows: Vec<TapeRow>,
    #[serde(default)]
    pub history: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SchemaPeek {
    schema_version: Option<u32>,
}

/// Resolves the default persistent session file path according to the session storage policy.
#[must_use]
pub fn default_session_path() -> std::path::PathBuf {
    if let Ok(dir) = std::env::var("CASETTE_CONFIG_DIR") {
        return std::path::PathBuf::from(dir).join("sessions").join("last-session.json");
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return std::path::PathBuf::from(xdg).join("casette").join("sessions").join("last-session.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return std::path::PathBuf::from(home)
            .join(".config")
            .join("casette")
            .join("sessions")
            .join("last-session.json");
    }
    std::env::temp_dir().join("casette_last_session.json")
}

/// Saves the session tape entries and settings to a JSON file.
pub fn save_session(state: &SessionState, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create session directory")?;
    }
    let envelope = SessionEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        rows: state.rows.clone(),
        history: state.history.clone(),
    };
    let json =
        serde_json::to_string_pretty(&envelope).context("failed to serialize session envelope")?;
    fs::write(path, json).context("failed to write session file")?;
    Ok(())
}

/// Loads session tape entries from a JSON file into session state.
///
/// Implements Problem 001 and 011 invariants:
/// 1. Peeks schema version first. If the file has a newer schema version,
///    refuses the file so it is left intact for newer versions and not overwritten.
/// 2. Converts any mid-eval running row (`is_evaluating == true`) to `interrupted`
///    so it never spins forever in the UI.
/// 3. Marks every restored row as `is_live = false`.
pub fn load_session(state: &mut SessionState, path: &Path) -> Result<()> {
    let json = fs::read_to_string(path).context("failed to read session file")?;

    // 1. Peek schema version before full decode
    if let Ok(peek) = serde_json::from_str::<SchemaPeek>(&json) {
        if let Some(v) = peek.schema_version {
            if v > CURRENT_SCHEMA_VERSION {
                bail!("refused session schema version {v} (current supported version is {CURRENT_SCHEMA_VERSION})");
            }
        }
    }

    // 2. Decode either structured SessionEnvelope or legacy raw Vec<TapeRow>
    let (rows, history) = if let Ok(envelope) = serde_json::from_str::<SessionEnvelope>(&json) {
        let hist = if envelope.history.is_empty() {
            envelope.rows.iter().map(|r| r.input_raw.clone()).collect()
        } else {
            envelope.history
        };
        (envelope.rows, hist)
    } else {
        let r: Vec<TapeRow> = serde_json::from_str(&json)
            .context("failed to deserialize session tape rows")?;
        let hist = r.iter().map(|row| row.input_raw.clone()).collect();
        (r, hist)
    };

    state.rows = rows;
    state.history = history;
    state.next_row_number = state
        .rows
        .iter()
        .map(|r| r.row_number)
        .max()
        .map(|n| n + 1)
        .unwrap_or(1);

    // 3. Mark restored rows as not live and ensure in-flight rows do not spin
    for row in &mut state.rows {
        row.is_live = false;
        if row.is_evaluating {
            row.is_evaluating = false;
            if row.result.is_none() {
                row.result = Some(WorkerResponse::new_interrupted(
                    "Casette quit before this evaluation finished".into(),
                ));
            }
        }
    }

    Ok(())
}

/// Exports the session tape as a formatted Markdown notebook document.
#[must_use]
pub fn export_to_markdown(state: &SessionState) -> String {
    let mut out = String::from("# Casette Session Notebook\n\n");

    for row in &state.rows {
        out.push_str(&format!("### Row #{}\n\n", row.row_number));
        out.push_str(&format!("**Input:** `{}`\n\n", row.input_raw));

        if let Some(res) = &row.result {
            if let Some(plain) = &res.plain {
                out.push_str("```text\n");
                out.push_str(plain);
                out.push_str("\n```\n\n");
            }
            if let Some(latex) = &res.latex {
                out.push_str(&format!("$$\n{}\n$$\n\n", latex));
            }
        }
    }

    out
}
