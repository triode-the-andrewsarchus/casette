//! Fast, pure friendly input compiler and command shim for Casette.
//!
//! Translates human-friendly math shorthand into canonical SageMath expressions.

pub mod compiler;
pub mod error;
pub mod ir;
pub mod result;
pub mod scanner;
pub mod tape_ref;
pub mod variables;

pub use compiler::{compile, free_variables, CommandMatch, Keyword, PartialRange, Range};
pub use error::CompileError;
pub use ir::FormulaIR;
pub use result::CompileResult;
pub use tape_ref::TapeReferenceTable;

/// Compiles input after expanding tape references (`#ROW`) via `refs`.
///
/// If reference expansion fails with a missing reference error, returns
/// [`CompileResult::Error`]. Otherwise compiles the expanded input through [`compile`].
#[must_use]
pub fn compile_with_refs(input: &str, refs: &TapeReferenceTable) -> CompileResult {
    match refs.expand_references(input) {
        Ok(expanded) => compile(&expanded),
        Err(err) => CompileResult::Error(err),
    }
}
