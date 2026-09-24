//! Fast, pure friendly input compiler and command shim for Casette.
//!
//! Translates human-friendly math shorthand into canonical SageMath expressions.

pub mod compiler;
pub mod error;
pub mod ir;
pub mod result;
pub mod scanner;
pub mod variables;

pub use compiler::{compile, free_variables, CommandMatch, Keyword, PartialRange, Range};
pub use error::CompileError;
pub use ir::FormulaIR;
pub use result::CompileResult;
