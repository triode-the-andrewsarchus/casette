//! Error types for kernel operations and worker process lifecycle.

use std::time::Duration;
use thiserror::Error;

/// Errors that can occur during Sage kernel operations or communication.
#[derive(Debug, Error)]
pub enum KernelError {
    /// Failed to discover a valid SageMath installation or Python environment.
    #[error("failed to discover Sage environment: {0}")]
    DiscoveryFailed(String),

    /// Failed to spawn the background worker process.
    #[error("failed to spawn worker process: {0}")]
    SpawnFailed(String),

    /// Standard I/O error encountered while communicating with the worker.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Failed to serialize or deserialize JSONL protocol envelopes.
    #[error("protocol JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The worker process exited unexpectedly.
    #[error("worker process exited unexpectedly (code: {code:?}): {stderr}")]
    WorkerExited {
        /// Exit code if available.
        code: Option<i32>,
        /// Stderr output emitted before exit.
        stderr: String,
    },

    /// Request exceeded the allowed timeout duration.
    #[error("request '{id}' timed out after {duration:?}")]
    RequestTimeout {
        /// Request identifier.
        id: String,
        /// Maximum duration elapsed before timeout.
        duration: Duration,
    },

    /// Request was cancelled or interrupted.
    #[error("request '{id}' was cancelled")]
    Cancelled {
        /// Request identifier.
        id: String,
    },

    /// The worker emitted an invalid or unexpected protocol message.
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),

    /// Process signal delivery failed.
    #[error("failed to send signal to worker (pid: {pid}): {reason}")]
    SignalFailed {
        /// Target process ID.
        pid: u32,
        /// Error explanation.
        reason: String,
    },

    /// The kernel has been shut down and can no longer accept requests.
    #[error("kernel has been shut down")]
    ShutDown,
}
