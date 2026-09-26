//! SageMath background worker process management and JSONL protocol client for Casette.
//!
//! This crate manages the lifecycle of the underlying Python/SageMath worker process (`worker.py`),
//! provides environment discovery on Linux systems, manages process groups and `SIGINT`
//! signal delivery for evaluation cancellation, and exposes an asynchronous interface for
//! evaluating expressions, retrieving live session symbols, configuring precision, and
//! handling plot artifacts.

pub mod client;
pub mod discovery;
pub mod error;
pub mod protocol;
pub mod woxi_kernel;

pub use client::{boot_prelude, KernelConfig, SageKernel, BOOT_VARIABLE_NAMES};
pub use discovery::{SageDiscovery, SageEnvironment};
pub use error::KernelError;
pub use protocol::{
    ArtifactPayload, ErrorPayload, ExtremumKind, LocalExtremum, MatrixPropertyPayload,
    PlotDataPayload, PlotSeriesPayload, SymbolPayload, TruncationPayload, WorkerRequest,
    WorkerResponse,
};
pub use woxi_kernel::{normalize_for_woxi, WoxiKernel};
