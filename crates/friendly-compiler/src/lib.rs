//! Fast, pure friendly input compiler and command shim for Casette.
//!
//! Translates human-friendly math shorthand into canonical SageMath expressions.

pub mod error;
pub mod result;

pub use error::CompileError;
pub use result::CompileResult;
