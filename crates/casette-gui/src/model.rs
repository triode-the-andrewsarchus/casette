//! Core data models for the Casette GUI session, tape, and message channels.

use casette_kernel::{SymbolPayload, WorkerResponse};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// High-level operational status of the background SageMath worker process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerStatus {
    /// Worker is booting or searching for Sage environment.
    Starting,
    /// Worker is idle, responsive, and ready to accept evaluations.
    Ready,
    /// Worker is currently executing a computation.
    Busy,
    /// Worker process is restarting.
    Restarting,
    /// Worker disconnected or exited with error.
    Error,
}

/// A single entry in the persistent session tape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TapeRow {
    /// Visible row number badge, 1-indexed (`#1`, `#2`, etc.).
    pub row_number: usize,

    /// Original friendly or raw code input entered by the user.
    pub input_raw: String,

    /// Compiled SageMath code statement sent to the worker.
    pub compiled_sage: String,

    /// Structured worker response envelope, or `None` if evaluation is in flight.
    pub result: Option<WorkerResponse>,

    /// Whether this row is currently evaluating in the background.
    pub is_evaluating: bool,

    /// Wall-clock evaluation execution duration.
    pub duration: Option<Duration>,

    /// Whether this row's value is live in the current Sage session namespace.
    pub is_live: bool,

    /// Epoch timestamp when the row was submitted.
    pub timestamp_secs: u64,
}

impl TapeRow {
    /// Creates a new in-flight tape row entry.
    #[must_use]
    pub fn new_evaluating(row_number: usize, input_raw: String, compiled_sage: String) -> Self {
        let timestamp_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            row_number,
            input_raw,
            compiled_sage,
            result: None,
            is_evaluating: true,
            duration: None,
            is_live: true,
            timestamp_secs,
        }
    }

    /// Completes the evaluation for this row with the received response.
    pub fn complete(&mut self, response: WorkerResponse, duration: Duration) {
        self.is_evaluating = false;
        self.duration = Some(duration);
        self.result = Some(response);
    }

    /// Returns the Sage expression that can be safely reused for follow-up commands
    /// (e.g. tape references `#ROW`). Returns `None` if the row failed, was interrupted,
    /// or cannot be expressed as a single reusable value.
    #[must_use]
    pub fn reusable_expression(&self) -> Option<String> {
        let res = self.result.as_ref()?;
        if !res.ok
            || res.kind.as_deref() == Some("error")
            || res.kind.as_deref() == Some("interrupted")
            || res.kind.as_deref() == Some("plot")
        {
            return None;
        }

        let sage = self.compiled_sage.trim();
        if sage.is_empty()
            || sage.starts_with("plot(")
            || sage.starts_with("implicit_plot(")
            || sage.starts_with("parametric_plot(")
        {
            return None;
        }

        // Echoed assignment check: "A = ...\nA"
        let lines: Vec<&str> = sage
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .collect();
        if lines.len() == 2 {
            if let Some((lhs, _)) = lines[0].split_once('=') {
                let target = lhs.trim();
                if lines[1] == target && friendly_compiler::variables::is_plausible_variable(target)
                {
                    return Some(target.to_string());
                }
            }
        }

        // Multiline non-assignment cannot be safely reused as a single expression
        if lines.len() > 1 {
            return None;
        }

        Some(sage.to_string())
    }
}

/// Actions dispatched from the UI thread to the asynchronous kernel task.
#[derive(Debug)]
pub enum GuiAction {
    /// Submit expression to evaluate.
    Evaluate {
        row_number: usize,
        input: String,
        compiled_code: String,
        numeric: bool,
    },
    /// Synchronize valid reusable tape references to Sage worker dictionary.
    SyncTapeRefs(Vec<(usize, String)>),
    /// Clear all tape references and reset row numbering.
    ClearTape,
    /// Probe worker responsiveness.
    Ping,
    /// Introspect session symbol table.
    FetchSymbols,
    /// Update session approximation precision.
    SetPrecision(u32),
    /// Cancel in-flight computation immediately.
    Interrupt,
    /// Restart background worker process.
    RestartWorker,
}

/// Messages sent from the asynchronous runtime to update the GUI thread.
#[derive(Debug)]
pub enum AsyncMessage {
    /// Worker successfully booted and reported its PID and Sage version.
    WorkerReady { pid: u32, version: String },
    /// An evaluation has finished.
    EvalFinished {
        row_number: usize,
        response: Box<WorkerResponse>,
        duration: Duration,
    },
    /// Symbols query completed.
    SymbolsUpdated(Vec<SymbolPayload>),
    /// Precision updated successfully.
    PrecisionUpdated(u32),
    /// Worker status changed.
    StatusChanged(WorkerStatus),
    /// Worker encountered an error.
    ErrorOccurred(String),
}

/// Active session state owned by the desktop app.
pub struct SessionState {
    /// All entries currently recorded on the tape.
    pub rows: Vec<TapeRow>,
    /// Next row number counter.
    pub next_row_number: usize,
    /// Currently selected row for sidebar inspection.
    pub selected_row_index: Option<usize>,
    /// Default decimal precision digits (e.g. 5, 10, 20).
    pub precision_digits: u32,
    /// Active worker process status.
    pub worker_status: WorkerStatus,
    /// Reported real worker process ID.
    pub worker_pid: Option<u32>,
    /// Discovered SageMath version banner.
    pub sage_version: String,
    /// Active symbols in session.
    pub symbols: Vec<SymbolPayload>,
    /// Chronological input history.
    pub history: Vec<String>,
    /// Current history browsing pointer.
    pub history_pointer: Option<usize>,
    /// Active draft input text in editor.
    pub draft_input: String,
    /// Force numeric evaluation mode toggle.
    pub force_numeric: bool,
    /// Flag requesting the input editor to take focus.
    pub request_input_focus: bool,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            rows: Vec::new(),
            next_row_number: 1,
            selected_row_index: None,
            precision_digits: 5, // App default is 5 digits per Casette specification
            worker_status: WorkerStatus::Starting,
            worker_pid: None,
            sage_version: String::new(),
            symbols: Vec::new(),
            history: Vec::new(),
            history_pointer: None,
            draft_input: String::new(),
            force_numeric: false,
            request_input_focus: true,
        }
    }
}

impl SessionState {
    /// Adds a newly submitted expression to the tape and returns the assigned row number.
    pub fn push_row(&mut self, input_raw: String, compiled_sage: String) -> usize {
        let row_num = self.next_row_number;
        self.next_row_number += 1;

        let row = TapeRow::new_evaluating(row_num, input_raw.clone(), compiled_sage);
        self.rows.push(row);
        self.history.push(input_raw);
        self.history_pointer = None;
        self.selected_row_index = Some(self.rows.len() - 1);
        row_num
    }

    /// Clears all entries from the tape and resets row numbering.
    pub fn clear_tape(&mut self) {
        self.rows.clear();
        self.next_row_number = 1;
        self.selected_row_index = None;
        self.request_input_focus = true;
    }

    /// Returns a [`friendly_compiler::TapeReferenceTable`] containing valid reusable expressions from the tape.
    #[must_use]
    pub fn tape_references(&self) -> friendly_compiler::TapeReferenceTable {
        let mut table = friendly_compiler::TapeReferenceTable::empty();
        let valid_rows: Vec<(usize, String)> = self
            .rows
            .iter()
            .filter_map(|r| r.reusable_expression().map(|expr| (r.row_number, expr)))
            .collect();
        let start = valid_rows
            .len()
            .saturating_sub(friendly_compiler::TapeReferenceTable::CAPACITY);
        for (num, expr) in &valid_rows[start..] {
            table.insert(*num, expr.clone());
        }
        table
    }
}
