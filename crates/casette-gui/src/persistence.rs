//! Session persistence and export functions for Casette sessions.

use crate::model::SessionState;
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Saves the session tape entries and settings to a JSON file.
pub fn save_session(state: &SessionState, path: &Path) -> Result<()> {
    let json =
        serde_json::to_string_pretty(&state.rows).context("failed to serialize session tape")?;
    fs::write(path, json).context("failed to write session file")?;
    Ok(())
}

/// Loads session tape entries from a JSON file into session state.
pub fn load_session(state: &mut SessionState, path: &Path) -> Result<()> {
    let json = fs::read_to_string(path).context("failed to read session file")?;
    let rows = serde_json::from_str(&json).context("failed to deserialize session tape")?;

    state.rows = rows;
    state.next_row_number = state
        .rows
        .iter()
        .map(|r| r.row_number)
        .max()
        .map(|n| n + 1)
        .unwrap_or(1);

    // Mark restored rows as not live until re-evaluated
    for row in &mut state.rows {
        row.is_live = false;
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
