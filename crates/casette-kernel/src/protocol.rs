//! JSONL protocol messages exchanged with worker.py.

use serde::{Deserialize, Serialize};

/// Request sent to worker.py.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRequest {
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub op: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// Structured response received from worker.py.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResponse {
    pub id: Option<String>,
    pub ok: bool,
    #[serde(default)]
    pub op: Option<String>,
    #[serde(default)]
    pub pong: Option<bool>,
    #[serde(default)]
    pub pid: Option<u32>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub plain: Option<String>,
    #[serde(default)]
    pub latex: Option<String>,
    #[serde(default)]
    pub stdout: Option<String>,
    #[serde(default)]
    pub stderr: Option<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
}
