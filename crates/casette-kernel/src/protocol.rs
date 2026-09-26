//! JSONL protocol messages exchanged with `worker.py`.
//!
//! The worker process communicates using newline-delimited JSON (JSONL) on standard
//! input and standard output. This module defines the request and response structures
//! according to the Casette worker protocol specification.

use serde::{Deserialize, Serialize};

/// Request sent to `worker.py`.
///
/// Each request is serialized to a single line of JSON and written to the worker's
/// standard input.
///
/// # Examples
///
/// ```
/// use casette_kernel::WorkerRequest;
///
/// let req = WorkerRequest::eval("req-1", "factor(x^2 - 1)", false, None);
/// assert_eq!(req.id.as_deref(), Some("req-1"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerRequest {
    /// Unique identifier for matching responses to requests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Operation type: `"ping"`, `"shutdown"`, `"symbols"`, `"config"`, or `None` (eval).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,

    /// SageMath code to execute (used when `op` is `None` or omitted).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,

    /// Precision in decimal digits for numerical approximations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub precision_digits: Option<u32>,

    /// When true, forces numerical approximation as the primary display result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numeric: Option<bool>,
}

impl WorkerRequest {
    /// Creates a new code evaluation request.
    ///
    /// # Arguments
    ///
    /// * `id` - Request identifier.
    /// * `code` - Sage source code to evaluate.
    /// * `numeric` - Whether to force numerical approximation.
    /// * `precision_digits` - Optional precision override for this request.
    #[must_use]
    pub fn eval(
        id: impl Into<String>,
        code: impl Into<String>,
        numeric: bool,
        precision_digits: Option<u32>,
    ) -> Self {
        Self {
            id: Some(id.into()),
            op: None,
            code: Some(code.into()),
            precision_digits,
            numeric: if numeric { Some(true) } else { None },
        }
    }

    /// Creates a ping request to probe worker liveness and latency.
    #[must_use]
    pub fn ping(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            op: Some("ping".to_string()),
            code: None,
            precision_digits: None,
            numeric: None,
        }
    }

    /// Creates a shutdown request telling the worker to exit cleanly.
    #[must_use]
    pub fn shutdown(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            op: Some("shutdown".to_string()),
            code: None,
            precision_digits: None,
            numeric: None,
        }
    }

    /// Creates a request querying active user symbols from the Sage namespace.
    #[must_use]
    pub fn symbols(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            op: Some("symbols".to_string()),
            code: None,
            precision_digits: None,
            numeric: None,
        }
    }

    /// Creates a configuration request updating session-level precision digits.
    #[must_use]
    pub fn config_precision(id: impl Into<String>, digits: u32) -> Self {
        Self {
            id: Some(id.into()),
            op: Some("config".to_string()),
            code: None,
            precision_digits: Some(digits),
            numeric: None,
        }
    }
}

/// Structured response received from `worker.py`.
///
/// Responses include the ready envelope emitted at worker startup, op replies (ping,
/// symbols, config), evaluation result envelopes, and error or interrupted envelopes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkerResponse {
    /// Request identifier echoed back from the matching request.
    #[serde(default)]
    pub id: Option<String>,

    /// Whether the operation or evaluation succeeded.
    pub ok: bool,

    /// Operation echo (`"ready"`, `"ping"`, `"shutdown"`, `"symbols"`, `"config"`, etc.).
    #[serde(default)]
    pub op: Option<String>,

    /// Pong confirmation for ping requests.
    #[serde(default)]
    pub pong: Option<bool>,

    /// Real operating system process ID reported by the worker in the ready envelope.
    #[serde(default)]
    pub pid: Option<u32>,

    /// Classified kind of the result (e.g. `"integer"`, `"matrix"`, `"symbolic"`,
    /// `"plot"`, `"error"`, `"interrupted"`).
    #[serde(default)]
    pub kind: Option<String>,

    /// Primary human-readable plain text display string.
    #[serde(default)]
    pub plain: Option<String>,

    /// LaTeX representation for rendering mathematical typography.
    #[serde(default)]
    pub latex: Option<String>,

    /// Unambiguous fallback representation (`repr(value)` in Python).
    #[serde(default)]
    pub repr: Option<String>,

    /// Numeric approximation string, or `None` if inexact/unavailable.
    #[serde(default)]
    pub approx: Option<String>,

    /// Precision in decimal digits that `approx` carries.
    #[serde(default)]
    pub approx_digits: Option<u32>,

    /// Whether the primary result is exact.
    #[serde(default)]
    pub exact: Option<bool>,

    /// Whether the primary result is approximate (e.g. force-numeric or float).
    #[serde(default)]
    pub primary_is_approx: Option<bool>,

    /// Original exact form when evaluated with `numeric = true`.
    #[serde(default)]
    pub exact_value: Option<String>,

    /// Contextual UI action names available for this result.
    #[serde(default)]
    pub actions: Vec<String>,

    /// Generated image or plot artifacts.
    #[serde(default)]
    pub artifacts: Vec<ArtifactPayload>,

    /// Structured 2D plot data payload for interactive plotting.
    #[serde(default)]
    pub plot_data: Option<PlotDataPayload>,

    /// Whether `plain` or `repr` output was truncated to prevent wire exhaustion.
    #[serde(default)]
    pub truncated: Option<bool>,

    /// Truncation metrics if output was capped.
    #[serde(default)]
    pub truncation: Option<TruncationPayload>,

    /// Inspectable matrix boolean properties if the result is a Sage matrix.
    #[serde(default)]
    pub matrix_properties: Option<Vec<MatrixPropertyPayload>>,

    /// Captured standard output from user `print(...)` statements or Sage routines.
    #[serde(default)]
    pub stdout: Option<String>,

    /// Captured standard error diagnostics.
    #[serde(default)]
    pub stderr: Option<String>,

    /// Detailed error payload when `ok` is false.
    #[serde(default)]
    pub error: Option<ErrorPayload>,

    /// Whether the statement yielded an expression value (vs a bare statement or None).
    #[serde(default)]
    pub value: Option<bool>,

    /// List of user variables, functions, and symbols returned by `op: "symbols"`.
    #[serde(default)]
    pub symbols: Option<Vec<SymbolPayload>>,

    /// Current session precision digits returned by `op: "config"`.
    #[serde(default)]
    pub precision_digits: Option<u32>,
}

impl WorkerResponse {
    /// Constructs a failed response representing a client-side or kernel-side error.
    #[must_use]
    pub fn new_error(error_type: String, message: String, traceback: Option<String>) -> Self {
        Self {
            ok: false,
            kind: Some("error".into()),
            plain: Some(message.clone()),
            repr: Some(message.clone()),
            actions: vec!["copy_traceback".into()],
            error: Some(ErrorPayload {
                r#type: error_type,
                message,
                traceback,
            }),
            ..Default::default()
        }
    }

    /// Constructs an interrupted response envelope.
    #[must_use]
    pub fn new_interrupted(message: String) -> Self {
        Self {
            ok: false,
            kind: Some("interrupted".into()),
            plain: Some(message.clone()),
            repr: Some(message.clone()),
            error: Some(ErrorPayload {
                r#type: "Interrupted".into(),
                message,
                traceback: None,
            }),
            ..Default::default()
        }
    }

    /// Returns `true` if this response represents the initial worker ready banner.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.ok && self.op.as_deref() == Some("ready") && self.pid.is_some()
    }

    /// Returns `true` if this response represents an interrupted evaluation.
    #[must_use]
    pub fn is_interrupted(&self) -> bool {
        self.kind.as_deref() == Some("interrupted")
            || self
                .error
                .as_ref()
                .is_some_and(|e| e.r#type == "KeyboardInterrupt")
    }
}

/// Image or plot artifact generated by Sage during evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactPayload {
    /// Artifact category, typically `"image"`.
    pub r#type: String,

    /// Image format, e.g. `"svg"` or `"png"`.
    #[serde(default)]
    pub format: String,

    /// Absolute filesystem path to the rendered artifact file.
    #[serde(default)]
    pub path: Option<String>,

    /// File size in bytes recorded at save time.
    #[serde(default)]
    pub bytes: Option<u64>,

    /// Error message if this specific format failed to save.
    #[serde(default)]
    pub error: Option<String>,
}

/// Category of a local extremum on a curve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExtremumKind {
    Minimum,
    Maximum,
}

/// A detected local extremum (peak or valley) on a plot series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalExtremum {
    pub x: f64,
    pub y: f64,
    pub kind: ExtremumKind,
}

/// Data for an individual plot curve or series.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotSeriesPayload {
    #[serde(default)]
    pub label: Option<String>,
    pub points: Vec<[f64; 2]>,
    pub color: [u8; 3],
    #[serde(default)]
    pub is_scatter: bool,
    #[serde(default)]
    pub extrema: Vec<LocalExtremum>,
}

/// Structured 2D plot data payload for interactive plotting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotDataPayload {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub x_label: Option<String>,
    #[serde(default)]
    pub y_label: Option<String>,
    pub series: Vec<PlotSeriesPayload>,
    pub x_range: [f64; 2],
    pub y_range: [f64; 2],
}

/// Detects local extrema (peaks and valleys) along a sequence of 2D sampled points.
#[must_use]
pub fn compute_local_extrema(points: &[[f64; 2]]) -> Vec<LocalExtremum> {
    if points.len() < 3 {
        return Vec::new();
    }
    let mut extrema = Vec::new();
    for i in 1..points.len() - 1 {
        let prev_y = points[i - 1][1];
        let curr_y = points[i][1];
        let next_y = points[i + 1][1];
        if !prev_y.is_finite() || !curr_y.is_finite() || !next_y.is_finite() {
            continue;
        }
        if curr_y > prev_y && curr_y > next_y {
            extrema.push(LocalExtremum {
                x: points[i][0],
                y: curr_y,
                kind: ExtremumKind::Maximum,
            });
        } else if curr_y < prev_y && curr_y < next_y {
            extrema.push(LocalExtremum {
                x: points[i][0],
                y: curr_y,
                kind: ExtremumKind::Minimum,
            });
        }
    }
    extrema
}

/// Truncation metrics when output string caps are applied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TruncationPayload {
    /// Original uncapped character count of `plain`.
    #[serde(alias = "plainLength")]
    pub plain_len: Option<usize>,

    /// Original uncapped character count of `repr`.
    #[serde(alias = "reprLength")]
    pub repr_len: Option<usize>,

    /// Maximum character cap applied to `plain`.
    #[serde(alias = "plainCap")]
    pub plain_cap: Option<usize>,

    /// Maximum character cap applied to `repr`.
    #[serde(alias = "reprCap")]
    pub repr_cap: Option<usize>,
}

/// One boolean property for matrix inspection (e.g. `is_invertible`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatrixPropertyPayload {
    /// Sage method name, e.g. `"is_invertible"`.
    pub name: String,

    /// User-friendly display label, e.g. `"Invertible"`.
    pub label: String,

    /// Evaluated boolean value, or `None` if indeterminate or not implemented.
    #[serde(default)]
    pub value: Option<bool>,

    /// Error description if inspecting this property raised an exception.
    #[serde(default)]
    pub error: Option<String>,
}

/// Structured error payload when evaluation or operation fails.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorPayload {
    /// Python exception class name, e.g. `"ZeroDivisionError"`, `"TypeError"`.
    pub r#type: String,

    /// Human-readable error message.
    pub message: String,

    /// Python traceback snippet.
    #[serde(default)]
    pub traceback: Option<String>,
}

/// Live symbol metadata extracted from the SageMath session namespace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolPayload {
    /// Symbol identifier name in Python/Sage.
    pub name: String,

    /// High-level symbol kind (e.g. `"variable"`, `"function"`, `"matrix"`).
    pub kind: String,

    /// Concise one-line summary description of the symbol value.
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_request_serialization() {
        let req = WorkerRequest::eval("req-1", "factor(x^2 - 1)", false, None);
        let json = serde_json::to_string(&req).expect("serialize");
        assert!(json.contains("\"id\":\"req-1\""));
        assert!(json.contains("\"code\":\"factor(x^2 - 1)\""));
        assert!(!json.contains("\"op\""));

        let ping = WorkerRequest::ping("p-1");
        let ping_json = serde_json::to_string(&ping).expect("serialize");
        assert!(ping_json.contains("\"op\":\"ping\""));
    }

    #[test]
    fn test_worker_response_deserialization() {
        let raw = r#"{
            "id": "req-1",
            "ok": true,
            "kind": "matrix",
            "plain": "[1 2]\n[3 4]",
            "latex": "\\left(\\begin{array}{rr} 1 & 2 \\\\ 3 & 4 \\end{array}\\right)",
            "repr": "[1 2]\n[3 4]",
            "approx": null,
            "actions": ["det", "rank"],
            "artifacts": [
                {"type": "image", "format": "png", "path": "/tmp/plot.png", "bytes": 1024}
            ],
            "truncated": false,
            "stdout": "",
            "stderr": "",
            "value": true
        }"#;

        let resp: WorkerResponse = serde_json::from_str(raw).expect("deserialize");
        assert_eq!(resp.id.as_deref(), Some("req-1"));
        assert!(resp.ok);
        assert_eq!(resp.kind.as_deref(), Some("matrix"));
        assert_eq!(resp.actions.len(), 2);
        assert_eq!(resp.artifacts.len(), 1);
        assert_eq!(resp.artifacts[0].format, "png");
        assert_eq!(resp.artifacts[0].bytes, Some(1024));
    }

    #[test]
    fn test_ready_envelope_recognition() {
        let raw = r#"{"op": "ready", "ok": true, "pid": 4210}"#;
        let resp: WorkerResponse = serde_json::from_str(raw).expect("deserialize");
        assert!(resp.is_ready());
        assert_eq!(resp.pid, Some(4210));
    }

    #[test]
    fn test_interrupted_envelope_recognition() {
        let raw = r#"{
            "id": "req-99",
            "ok": false,
            "kind": "interrupted",
            "plain": "",
            "error": {
                "type": "KeyboardInterrupt",
                "message": "eval interrupted"
            }
        }"#;
        let resp: WorkerResponse = serde_json::from_str(raw).expect("deserialize");
        assert!(resp.is_interrupted());
    }
}
